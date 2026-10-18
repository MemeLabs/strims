// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's captures on fake nodes (see `strims_testworld`), through its
//! RPC connection: records another node's stream reads, the listings and
//! snippet, a record dropped for a second end in its frame, the checks and
//! bounds on a capture, and captures belonging to their connection.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fmt::Debug;
use std::time::Duration;

use futures_util::StreamExt;
use strims_crypto::SignableMessage;
use strims_dao::Table;
use strims_dao::schema::network::Networks;
use strims_kv::MemoryKv;
use strims_ppspp::Uri;
use strims_proto::network::v1::Network;
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::service::CallError;
use strims_proto::video::v1::egress_open_stream_response::Body;
use strims_proto::video::v1::{
    CaptureAppendRequest, CaptureClient, CaptureCloseRequest, CaptureOpenRequest,
    CaptureUpdateRequest, EgressClient, EgressOpenStreamRequest, EgressOpenStreamResponse,
};
use strims_rpc::{Client, ClientStream, Router, Timer, connect};
use strims_testworld::*;
use strims_transfer::TransferId;
use strims_video::hls::HlsRoutes;
use strims_video::{CaptureSettings, Config, NoVideoHost, SwarmKind, VideoControl, bind_frontend};
use tokio::task::{JoinHandle, spawn_local};

type Control = VideoControl<MemoryKv, FakeHost, NoVideoHost, TokioTimer>;

const MIME: &str = "video/webm;codecs=vp9,opus";

fn start(n: &Node, config: Config) -> Control {
    let video = VideoControl::new(
        n.host.clone(),
        NoVideoHost,
        n.store.clone(),
        n.profile.clone(),
        n.network.clone(),
        n.transfers.clone(),
        n.directory.clone(),
        HlsRoutes::new(),
        n.timer.clone(),
        n.rng(),
        config,
    );
    let v = video.clone();
    n.spawn(async move { v.run().await });
    video
}

/// A UI's connection to `video`'s frontend: its clients, and the server's
/// task, whose abort closes the connection.
struct Ui {
    capture: CaptureClient<Client>,
    egress: EgressClient<Client>,
    server: JoinHandle<()>,
}

fn connect_ui(video: &Control) -> Ui {
    let router = Router::new();
    bind_frontend(&router, video);
    let (up, down) = (Pipe::default(), Pipe::default());
    let config = strims_rpc::Config::frontend();
    let (_, server) = connect(
        config.clone(),
        Reader(up.clone()),
        Writer(down.clone()),
        Some(router),
    );
    let (client, conn) = connect(config, Reader(down), Writer(up), None);
    let server = spawn_local(async move {
        let _ = server.await;
    });
    spawn_local(async move {
        let _ = conn.await;
    });
    Ui {
        capture: CaptureClient::new(client.clone()),
        egress: EgressClient::new(client),
        server,
    }
}

fn snippet(title: &str) -> ListingSnippet {
    ListingSnippet {
        title: title.into(),
        description: "desc".into(),
        channel_name: "chan".into(),
        live: true,
        ..Default::default()
    }
}

/// The message of a call's remote error.
fn refusal<T: Debug>(r: Result<T, CallError>) -> String {
    match r {
        Err(CallError::Remote(e)) => e.message,
        r => panic!("not refused: {r:?}"),
    }
}

impl Ui {
    async fn open(&self, keys: &[Vec<u8>], title: &str) -> Result<Vec<u8>, CallError> {
        self.open_with(MIME, keys, snippet(title)).await
    }

    async fn open_with(
        &self,
        mime: &str,
        keys: &[Vec<u8>],
        s: ListingSnippet,
    ) -> Result<Vec<u8>, CallError> {
        let req = CaptureOpenRequest {
            directory_snippet: Some(s),
            mime_type: mime.into(),
            network_keys: keys.to_vec(),
        };
        Ok(self.capture.open(&req).await?.id)
    }

    async fn append(&self, id: &[u8], data: &[u8], segment_end: bool) -> Result<(), CallError> {
        let req = CaptureAppendRequest {
            id: id.to_vec(),
            data: data.to_vec(),
            segment_end,
        };
        self.capture.append(&req).await.map(|_| ())
    }

    /// Appends `data` as a record, in three appends.
    async fn record(&self, id: &[u8], data: &[u8]) {
        let third = data.len() / 3;
        self.append(id, &data[..third], false).await.unwrap();
        self.append(id, &data[third..2 * third], false)
            .await
            .unwrap();
        self.append(id, &data[2 * third..], true).await.unwrap();
    }

    async fn update(&self, id: &[u8], title: &str) -> Result<(), CallError> {
        let req = CaptureUpdateRequest {
            id: id.to_vec(),
            directory_snippet: Some(snippet(title)),
        };
        self.capture.update(&req).await.map(|_| ())
    }

    async fn close(&self, id: &[u8]) -> Result<(), CallError> {
        let req = CaptureCloseRequest { id: id.to_vec() };
        self.capture.close(&req).await.map(|_| ())
    }

    /// Opens a stream of `uri`: its responses after `Open`, and the
    /// transfer id `Open` carried.
    async fn stream(&self, uri: &str, keys: &[Vec<u8>]) -> (Responses, Vec<u8>) {
        let mut s = self
            .egress
            .open_stream(&EgressOpenStreamRequest {
                swarm_uri: uri.into(),
                network_keys: keys.to_vec(),
            })
            .await
            .unwrap();
        let Body::Open(o) = next(&mut s).await else {
            panic!("not open first");
        };
        (s, o.transfer_id)
    }
}

type Responses = ClientStream<EgressOpenStreamResponse>;

async fn next(s: &mut Responses) -> Body {
    match tokio::time::timeout(Duration::from_secs(10), s.next()).await {
        Ok(Some(Ok(r))) => r.body.unwrap(),
        r => panic!("no response: {r:?}"),
    }
}

/// The next `n` records of a stream, none after a discontinuity.
async fn read(s: &mut Responses, n: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut rec = Vec::new();
    while out.len() < n {
        let Body::Data(d) = next(s).await else {
            panic!("not data");
        };
        assert!(!d.discontinuity);
        rec.extend_from_slice(&d.data);
        if d.segment_end {
            out.push(std::mem::take(&mut rec));
        }
    }
    out
}

/// `n` bytes of record `i`.
fn record(i: u8, n: usize) -> Vec<u8> {
    (0..n)
        .map(|j| i.wrapping_mul(13).wrapping_add(j as u8))
        .collect()
}

/// The media listings of network `network_id`'s view on `n`: id, content
/// and snippet.
fn media(n: &Node, network_id: u64) -> Vec<(u64, Media, Option<ListingSnippet>)> {
    n.directory
        .listings(network_id)
        .into_iter()
        .filter_map(|l| match l.listing.content {
            Some(Content::Media(m)) => Some((l.id, m, l.snippet)),
            _ => None,
        })
        .collect()
}

/// A capture's records reach a stream on another node whole, by the URI
/// its listing carries; closing it takes down its listing, transfer and
/// swarm.
#[test]
fn a_capture_is_read_by_another_nodes_stream() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        let keys = [net.public.clone()];
        let va = start(&a, Config::default());
        let vb = start(&b, Config::default());
        let (ua, ub) = (connect_ui(&va), connect_ui(&vb));

        let id = ua.open(&keys, "live").await.unwrap();
        let tid = TransferId::from_bytes(&id).unwrap();
        let c = va.captures();
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].id, c[0].mime_type.as_str()), (tid, MIME));
        let swarm = Uri::parse(&c[0].swarm_uri).unwrap().id;
        assert_eq!(TransferId::new(&swarm, b""), tid);
        assert!(
            c[0].swarm_uri
                .ends_with("&x.im=2&x.hf=4&x.sa=2&x.cs=1024&x.ps=32&x.sc=1")
        );
        assert!(a.transfers.is_published(tid, a.net(&net.public)));
        assert_eq!(a.host.swarms_of(&swarm), (1, 0));
        let s = &va.video_swarms()[0];
        assert_eq!((s.kind, s.transfer_id), (SwarmKind::Capture, tid));

        wait_for(10, "the listing", || !media(&b, 10).is_empty()).await;
        let (_, m, _) = media(&b, 10).remove(0);
        assert_eq!(
            (m.mime_type.as_str(), &m.swarm_uri),
            (MIME, &c[0].swarm_uri)
        );
        let (mut stream, stream_id) = ub.stream(&m.swarm_uri, &keys).await;
        assert_eq!(stream_id, id);
        let records: Vec<Vec<u8>> = (0..4)
            .map(|i| record(i, 70_000 + usize::from(i) * 30_000))
            .collect();
        for r in &records {
            ua.record(&id, r).await;
        }
        assert_eq!(read(&mut stream, 4).await, records);

        ua.close(&id).await.unwrap();
        assert!(va.captures().is_empty());
        assert!(va.video_swarms().is_empty());
        assert!(!a.transfers.list().contains(&tid));
        settle().await;
        assert_eq!(a.host.swarms_of(&swarm), (0, 0));
        wait_for(10, "the listing to go", || media(&b, 10).is_empty()).await;
        assert_eq!(
            refusal(ua.append(&id, b"x", true).await),
            "capture not found"
        );
    });
}

/// A capture on two networks keeps a listing on each one's directory.
#[test]
fn a_capture_on_two_networks_has_a_listing_on_each() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (n10, _) = owned_network(&a, &[], 10).await;
        let (n11, _) = owned_network(&a, &[], 11).await;
        let va = start(&a, Config::default());
        let ua = connect_ui(&va);
        let keys = [n10.public.clone(), n11.public.clone()];
        let id = ua.open(&keys, "twice").await.unwrap();
        let tid = TransferId::from_bytes(&id).unwrap();
        for k in &keys {
            assert!(a.transfers.is_published(tid, a.net(k)));
        }

        wait_for(10, "both listings", || {
            va.captures()[0].listings.iter().all(|(_, l)| l.is_some())
                && !media(&a, 10).is_empty()
                && !media(&a, 11).is_empty()
        })
        .await;
        let c = va.captures().remove(0);
        for (network_id, key) in [(10, &n10.public), (11, &n11.public)] {
            let (_, listing) = c.listings.iter().find(|(k, _)| k == key).unwrap();
            let listed = media(&a, network_id);
            assert_eq!(listed.len(), 1, "network {network_id}");
            assert_eq!(Some(listed[0].0), *listing);
            assert_eq!(listed[0].1.swarm_uri, c.swarm_uri);
        }
    });
}

/// A capture's transfer goes on its network again as the node joins it
/// again.
#[test]
fn a_capture_goes_on_its_network_as_the_node_joins_it() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let key = key.public;
        let va = start(&alice, Config::default());
        let ua = connect_ui(&va);
        let id = ua.open(std::slice::from_ref(&key), "again").await.unwrap();
        let tid = TransferId::from_bytes(&id).unwrap();
        let first = alice.net(&key);
        assert!(alice.transfers.is_published(tid, first));

        let record = alice.read_network(10).await;
        let mut tx = alice.store.write(alice.timer.now()).await.unwrap();
        Networks::delete(&mut tx, 10).await.unwrap();
        tx.commit().await.unwrap();
        wait_for(10, "the network to stop", || alice.host.net(&key).is_none()).await;
        alice.write_network(Network { id: 110, ..record }).await;
        wait_for(120, "the transfer published again", || {
            alice
                .host
                .net(&key)
                .is_some_and(|net| net != first && alice.transfers.is_published(tid, net))
        })
        .await;
        assert_eq!(va.captures().len(), 1);
    });
}

/// The snippet is the capture's, signed with its key, and its views show
/// it and its updates.
#[test]
fn the_snippet_is_signed_with_the_capture_key() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        let va = start(&a, Config::default());
        let ua = connect_ui(&va);
        let id = ua
            .open(std::slice::from_ref(&net.public), "first")
            .await
            .unwrap();
        let swarm = Uri::parse(&va.captures()[0].swarm_uri).unwrap().id;

        let shown = |title: &'static str| {
            let b = &b;
            move || {
                media(b, 10)
                    .first()
                    .and_then(|(_, _, s)| s.as_ref())
                    .is_some_and(|s| s.title == title)
            }
        };
        wait_for(10, "the snippet", shown("first")).await;
        let s = media(&b, 10).remove(0).2.unwrap();
        assert_eq!(s.key, swarm.as_bytes());
        s.verify_message().unwrap();
        assert_eq!((s.description.as_str(), s.live), ("desc", true));

        ua.update(&id, "second").await.unwrap();
        wait_for(10, "the update", shown("second")).await;
        let s = media(&b, 10).remove(0).2.unwrap();
        s.verify_message().unwrap();
        assert_eq!(s.key, swarm.as_bytes());

        let long = "x".repeat(9 * 1024);
        assert_eq!(refusal(ua.update(&id, &long).await), "snippet too large");
    });
}

/// A frontend connection's captures close with it.
#[test]
fn closing_the_connection_closes_its_captures() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        let va = start(&a, Config::default());
        let ua = connect_ui(&va);
        let kept = connect_ui(&va);
        let keys = [net.public.clone()];
        let id = ua.open(&keys, "going").await.unwrap();
        let other = kept.open(&keys, "staying").await.unwrap();
        let tid = TransferId::from_bytes(&id).unwrap();
        let going = va.captures().into_iter().find(|c| c.id == tid).unwrap();
        let swarm = Uri::parse(&going.swarm_uri).unwrap().id;
        wait_for(10, "the listings", || media(&b, 10).len() == 2).await;

        let server = ua.server;
        server.abort();
        let _ = server.await;
        wait_for(10, "the capture to close", || va.captures().len() == 1).await;
        assert_eq!(va.captures()[0].id.as_bytes().to_vec(), other);
        assert!(!a.transfers.list().contains(&tid));
        settle().await;
        assert_eq!(a.host.swarms_of(&swarm), (0, 0));
        let listed = media(&b, 10);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].1.swarm_uri, va.captures()[0].swarm_uri);
        kept.append(&other, &record(1, 100), true).await.unwrap();
    });
}

/// A record that would end in the frame of the record before it is
/// dropped: its append fails, the capture goes on, and readers see only
/// whole records.
#[test]
fn a_second_end_in_one_frame_drops_that_record() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        let keys = [net.public.clone()];
        let va = start(&a, Config::default());
        let vb = start(&b, Config::default());
        let (ua, ub) = (connect_ui(&va), connect_ui(&vb));
        let id = ua.open(&keys, "small").await.unwrap();
        let uri = va.captures()[0].swarm_uri.clone();
        let (mut stream, _) = ub.stream(&uri, &keys).await;

        let r1 = record(1, 1000);
        ua.append(&id, &r1, true).await.unwrap();
        let r2 = record(2, 500);
        ua.append(&id, &r2[..200], false).await.unwrap();
        assert_eq!(
            refusal(ua.append(&id, &r2[200..], true).await),
            "record dropped: it ended too soon after the one before it"
        );
        let r3 = record(3, 100_000);
        ua.append(&id, &r3, true).await.unwrap();
        // Record 3 ends in the next frame, after which a small record ends
        // in that frame too.
        assert_eq!(
            refusal(ua.append(&id, &record(4, 10), true).await),
            "record dropped: it ended too soon after the one before it"
        );
        let r5 = record(5, 70_000);
        ua.append(&id, &r5, true).await.unwrap();
        assert_eq!(read(&mut stream, 3).await, [r1, r3, r5]);
        assert_eq!(va.metrics().dropped_records, 2);
    });
}

/// A capture's MIME type, network keys and snippet are checked, and the
/// node runs at most `max_captures`.
#[test]
fn captures_are_checked_and_bounded() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (net, _) = owned_network(&a, &[], 10).await;
        let config = Config {
            capture: CaptureSettings {
                max_captures: 2,
                max_network_keys: 2,
                max_mime_bytes: 32,
            },
            ..Config::default()
        };
        let va = start(&a, config);
        let ua = connect_ui(&va);
        let key = net.public.clone();
        for mime in ["", "video/webm\n", "vidéo/webm", &"x".repeat(33)] {
            let r = ua
                .open_with(mime, std::slice::from_ref(&key), snippet("x"))
                .await;
            assert_eq!(refusal(r), "invalid mime type", "{mime:?}");
        }
        assert_eq!(
            refusal(ua.open(&[], "x").await),
            "wrong number of network keys"
        );
        let three = [key.clone(), vec![2; 32], vec![3; 32]];
        assert_eq!(
            refusal(ua.open(&three, "x").await),
            "wrong number of network keys"
        );
        assert_eq!(
            refusal(ua.open(&[vec![2; 32]], "x").await),
            "network not found"
        );
        let large = snippet(&"t".repeat(9 * 1024));
        assert_eq!(
            refusal(ua.open_with(MIME, std::slice::from_ref(&key), large).await),
            "snippet too large"
        );
        assert!(va.captures().is_empty());

        // A key named twice counts once.
        let first = ua.open(&[key.clone(), key.clone()], "1").await.unwrap();
        assert_eq!(va.captures()[0].listings.len(), 1);
        ua.open(std::slice::from_ref(&key), "2").await.unwrap();
        assert_eq!(
            refusal(ua.open(std::slice::from_ref(&key), "3").await),
            "too many captures"
        );
        ua.close(&first).await.unwrap();
        ua.open(std::slice::from_ref(&key), "3").await.unwrap();
        assert_eq!(va.captures().len(), 2);
        assert_eq!(
            a.host
                .swarms_of(&Uri::parse(&va.captures()[0].swarm_uri).unwrap().id),
            (1, 0)
        );
    });
}

/// Another connection can't append to, update or close a capture it
/// didn't open.
#[test]
fn a_capture_answers_only_its_own_connection() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (net, _) = owned_network(&a, &[], 10).await;
        let va = start(&a, Config::default());
        let (owner, other) = (connect_ui(&va), connect_ui(&va));
        let id = owner
            .open(std::slice::from_ref(&net.public), "mine")
            .await
            .unwrap();

        let not_found = "capture not found";
        assert_eq!(refusal(other.append(&id, b"x", true).await), not_found);
        assert_eq!(refusal(other.update(&id, "theirs").await), not_found);
        assert_eq!(refusal(other.close(&id).await), not_found);
        assert_eq!(refusal(owner.append(&[1; 5], b"x", true).await), not_found);
        assert_eq!(refusal(owner.append(&[1; 32], b"x", true).await), not_found);
        assert_eq!(va.captures().len(), 1);

        owner.append(&id, &record(1, 100), true).await.unwrap();
        owner.update(&id, "still mine").await.unwrap();
        owner.close(&id).await.unwrap();
        assert_eq!(refusal(owner.close(&id).await), not_found);
        assert!(va.captures().is_empty());
    });
}
