// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's egress streams on fake nodes (see `strims_testworld`), through
//! its RPC connection: the response framing, whole records across resets
//! and mid-stream joins, the swarms streams share and release, and the
//! checks and bounds on what a stream opens.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::time::Duration;

use futures_util::StreamExt;
use strims_chunkstream::packed::{DEFAULT_SIZE, PackedEncoder};
use strims_crypto::key_from_seed;
use strims_kv::MemoryKv;
use strims_ppspp::{SwarmId, SwarmOptions, Uri, WriterOptions};
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::service::CallError;
use strims_proto::video::v1::egress_open_stream_response::Body;
use strims_proto::video::v1::{EgressClient, EgressOpenStreamRequest, EgressOpenStreamResponse};
use strims_rpc::{Client, ClientStream, Router};
use strims_testworld::*;
use strims_transfer::{SwarmHost, SwarmWrite, TransferId};
use strims_video::hls::HlsRoutes;
use strims_video::{
    Config, EgressSettings, NoVideoHost, SwarmKind, VideoControl, bind_frontend, capture_options,
};

type Control = VideoControl<MemoryKv, FakeHost, NoVideoHost, TokioTimer>;
type Responses = ClientStream<EgressOpenStreamResponse>;

/// The most bytes of a Data message, by default.
const MAX_DATA: usize = 32 * 1024;

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

/// A UI's egress client on a connection to `video`'s frontend.
fn egress(video: &Control) -> EgressClient<Client> {
    let router = Router::new();
    bind_frontend(&router, video);
    EgressClient::new(frontend_client(router))
}

/// A swarm written on a node outside the controls, in the packed layout.
struct Publisher {
    id: SwarmId,
    uri: String,
    encoder: PackedEncoder,
    writer: FakeWriter,
    out: Vec<u8>,
}

impl Publisher {
    async fn new(n: &Node, seed: u8, options: SwarmOptions) -> Publisher {
        let (_, id, writer) = n
            .host
            .create_swarm(WriterOptions {
                swarm_options: options.clone(),
                key: key_from_seed(&[seed; 32]),
            })
            .await
            .unwrap();
        let uri = Uri::new(id.clone(), options.uri_options()).to_string();
        Publisher {
            id,
            uri,
            encoder: PackedEncoder::new(DEFAULT_SIZE).unwrap(),
            writer,
            out: Vec::new(),
        }
    }

    fn transfer_id(&self) -> TransferId {
        TransferId::new(&self.id, b"")
    }

    /// Writes `data` as a record, in pieces with a pause after each.
    async fn record(&mut self, data: &[u8]) {
        for piece in data.chunks(16 * 1024) {
            self.encoder.write(piece, &mut self.out);
            self.flush().await;
        }
        self.encoder.end_record(&mut self.out).unwrap();
        self.flush().await;
    }

    async fn flush(&mut self) {
        if !self.out.is_empty() {
            self.writer.write(&self.out).await.unwrap();
            self.out.clear();
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    /// Starts the stream over, as a restarted writer does.
    async fn reset(&mut self) {
        self.writer.reset().await.unwrap();
        self.encoder = PackedEncoder::new(DEFAULT_SIZE).unwrap();
    }
}

/// `n` bytes of record `i`.
fn record(i: u8, n: usize) -> Vec<u8> {
    (0..n)
        .map(|j| i.wrapping_mul(29).wrapping_add(j as u8))
        .collect()
}

/// Records over a frame each, so no two end in one frame.
fn records(from: u8, n: u8) -> Vec<Vec<u8>> {
    (from..from + n)
        .map(|i| record(i, 66_000 + usize::from(i % 5) * 9_000))
        .collect()
}

/// Opens a stream: its responses after `Open`, and the transfer id
/// `Open` carried. Fails with the refusal's message.
async fn open(
    e: &EgressClient<Client>,
    uri: &str,
    keys: &[Vec<u8>],
) -> Result<(Responses, Vec<u8>), String> {
    let mut s = e
        .open_stream(&EgressOpenStreamRequest {
            swarm_uri: uri.into(),
            network_keys: keys.to_vec(),
        })
        .await
        .unwrap();
    match next(&mut s).await {
        Ok(Body::Open(o)) => Ok((s, o.transfer_id)),
        Ok(b) => panic!("not open first: {b:?}"),
        Err(CallError::Remote(e)) => Err(e.message),
        Err(e) => panic!("{e}"),
    }
}

/// The stream's next response within 10 seconds.
async fn next(s: &mut Responses) -> Result<Body, CallError> {
    match tokio::time::timeout(Duration::from_secs(10), s.next()).await {
        Ok(Some(r)) => r.map(|r| r.body.unwrap()),
        r => panic!("no response: {r:?}"),
    }
}

/// The next `n` records of a stream, as the UI takes them: each with
/// whether a discontinuity came before it. Every message is a bounded
/// Data message.
async fn read(s: &mut Responses, n: usize) -> Vec<(Vec<u8>, bool)> {
    let mut out = Vec::new();
    let mut rec = Vec::new();
    let mut flagged = false;
    while out.len() < n {
        let Body::Data(d) = next(s).await.unwrap() else {
            panic!("not data");
        };
        assert!(d.data.len() <= MAX_DATA, "{} bytes", d.data.len());
        if d.discontinuity {
            rec.clear();
            flagged = true;
        }
        rec.extend_from_slice(&d.data);
        if d.segment_end {
            out.push((std::mem::take(&mut rec), std::mem::take(&mut flagged)));
        } else {
            assert_eq!(d.data.len(), MAX_DATA, "a message waits until full");
        }
    }
    out
}

fn unflagged(rs: &[Vec<u8>]) -> Vec<(Vec<u8>, bool)> {
    rs.iter().map(|r| (r.clone(), false)).collect()
}

/// A stream answers `Open` with its transfer first, then carries each
/// record whole in Data messages of at most 32 KiB, the last of each with
/// `segment_end`; records written before it opened come too.
#[test]
fn a_stream_opens_then_carries_whole_records_in_bounded_messages() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        let mut p = Publisher::new(&a, 7, capture_options()).await;
        let mut written = vec![record(0, DEFAULT_SIZE - 4), record(1, 3 * MAX_DATA)];
        for r in &written {
            p.record(r).await;
        }

        let video = start(&b, Config::default());
        let e = egress(&video);
        let (mut s, tid) = open(&e, &p.uri, std::slice::from_ref(&net.public))
            .await
            .unwrap();
        assert_eq!(tid, p.transfer_id().as_bytes().to_vec());
        let more = vec![record(2, 100_000), record(3, 66_000), record(4, 250_000)];
        for r in &more {
            p.record(r).await;
        }
        written.extend(more);
        assert_eq!(read(&mut s, 5).await, unflagged(&written));
        assert!(
            b.transfers
                .is_published(p.transfer_id(), b.net(&net.public))
        );
    });
}

/// A writer that starts its stream over shows as one `discontinuity`,
/// whose message begins the next whole record.
#[test]
fn a_writer_reset_is_one_discontinuity_then_the_next_whole_record() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        let mut p = Publisher::new(&a, 7, capture_options()).await;
        let video = start(&b, Config::default());
        let e = egress(&video);
        let (mut s, _) = open(&e, &p.uri, std::slice::from_ref(&net.public))
            .await
            .unwrap();

        let before = records(0, 2);
        for r in &before {
            p.record(r).await;
        }
        assert_eq!(read(&mut s, 2).await, unflagged(&before));
        // A record cut short by the reset is never a record.
        p.encoder.write(&record(9, 70_000), &mut p.out);
        p.flush().await;
        p.reset().await;
        let after = records(5, 3);
        for r in &after {
            p.record(r).await;
        }
        let mut expected = unflagged(&after);
        expected[0].1 = true;
        assert_eq!(read(&mut s, 3).await, expected);
        assert_eq!(video.metrics().reopens, 0);
    });
}

/// A stream that joins inside a record drops it and starts at the next
/// whole one, without a discontinuity.
#[test]
fn joining_mid_stream_drops_the_partial_first_record() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        // A 256 KiB window, which the first records leave.
        let options = SwarmOptions {
            live_window: 256,
            ..capture_options()
        };
        let mut p = Publisher::new(&a, 7, options).await;
        let mut written = records(0, 8);
        for r in &written {
            p.record(r).await;
        }
        let video = start(&b, Config::default());
        let e = egress(&video);
        let (mut s, _) = open(&e, &p.uri, std::slice::from_ref(&net.public))
            .await
            .unwrap();
        let more = records(8, 3);
        for r in &more {
            p.record(r).await;
        }
        written.extend(more);

        let mut got = Vec::new();
        while got.last() != written.last() {
            assert!(got.len() < written.len());
            let (r, flagged) = read(&mut s, 1).await.remove(0);
            assert!(!flagged);
            got.push(r);
        }
        let start = written.len() - got.len();
        assert!(start > 0 && start < 8, "starts at record {start}");
        assert_eq!(got, written[start..]);
    });
}

/// Streams of one swarm share its swarm and transfer; the last to close
/// removes both.
#[test]
fn two_streams_share_one_transfer_until_the_last_closes() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        let mut p = Publisher::new(&a, 7, capture_options()).await;
        let video = start(&b, Config::default());
        let (e1, e2) = (egress(&video), egress(&video));
        let keys = [net.public.clone()];
        let (mut s1, _) = open(&e1, &p.uri, &keys).await.unwrap();
        let (mut s2, _) = open(&e2, &p.uri, &keys).await.unwrap();
        assert_eq!(b.host.swarms_of(&p.id), (0, 1));
        let before = b.transfers.list().len();
        assert!(b.transfers.list().contains(&p.transfer_id()));
        let swarms = video.video_swarms();
        assert_eq!(swarms.len(), 1);
        assert_eq!(
            (swarms[0].kind, swarms[0].streams, swarms[0].added),
            (SwarmKind::Egress, 2, true)
        );
        assert_eq!(swarms[0].network_keys, keys);

        let r = records(0, 2);
        for x in &r {
            p.record(x).await;
        }
        assert_eq!(read(&mut s1, 2).await, unflagged(&r));
        assert_eq!(read(&mut s2, 2).await, unflagged(&r));

        drop(s1);
        settle().await;
        assert_eq!(b.host.swarms_of(&p.id), (0, 1));
        assert_eq!(video.video_swarms()[0].streams, 1);
        p.record(&r[0]).await;
        assert_eq!(read(&mut s2, 1).await, unflagged(&r[..1]));

        drop(s2);
        settle().await;
        assert_eq!(b.host.swarms_of(&p.id), (0, 0));
        assert!(!b.transfers.list().contains(&p.transfer_id()));
        assert_eq!(b.transfers.list().len(), before - 1);
        assert!(video.video_swarms().is_empty());

        // A stream opened after reads the swarm added again, from the
        // window's start.
        let (mut s3, _) = open(&e1, &p.uri, &keys).await.unwrap();
        assert_eq!(b.host.swarms_of(&p.id), (0, 1));
        assert_eq!(read(&mut s3, 1).await, unflagged(&r[..1]));
    });
}

/// A stream of a capture on the same node reads the capture's swarm, adds
/// none, and leaves the capture running when it closes; the capture's
/// close ends it with an error.
#[test]
fn a_local_captures_swarm_is_read_where_it_is() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (net, _) = owned_network(&a, &[], 10).await;
        let config = Config {
            egress: EgressSettings {
                max_swarms: 1,
                ..EgressSettings::default()
            },
            ..Config::default()
        };
        let video = start(&a, config);
        let id = video
            .open_capture(
                "video/webm",
                vec![net.public.clone()],
                ListingSnippet::default(),
            )
            .await
            .unwrap();
        let uri = video.captures()[0].swarm_uri.clone();
        let swarm = Uri::parse(&uri).unwrap().id;
        let e = egress(&video);

        // A stream of another swarm takes the one egress swarm.
        let other = Publisher::new(&a, 9, capture_options()).await;
        let (_other_stream, _) = open(&e, &other.uri, &[]).await.unwrap();

        let (mut s, tid) = open(&e, &uri, &[]).await.unwrap();
        assert_eq!(tid, id.as_bytes().to_vec());
        assert_eq!(a.host.swarms_of(&swarm), (1, 0));
        let r = records(0, 2);
        for x in &r {
            video.append_capture(&id, x.clone(), true).await.unwrap();
        }
        assert_eq!(read(&mut s, 2).await, unflagged(&r));
        drop(s);
        settle().await;
        assert_eq!(a.host.swarms_of(&swarm), (1, 0));
        assert!(a.transfers.list().contains(&id));
        video.append_capture(&id, r[0].clone(), true).await.unwrap();
        assert_eq!(video.captures().len(), 1);

        let (mut s, _) = open(&e, &uri, &[]).await.unwrap();
        assert_eq!(
            read(&mut s, 3).await,
            unflagged(&[&r[..], &r[..1]].concat())
        );
        video.close_capture(&id).await.unwrap();
        match next(&mut s).await.unwrap() {
            Body::Error(e) => assert_eq!(e.message, "stream closed"),
            b => panic!("not an error: {b:?}"),
        }
        let end = tokio::time::timeout(Duration::from_secs(5), s.next()).await;
        assert!(end.unwrap().is_none());
        assert_eq!(a.host.swarms_of(&swarm), (0, 0));
        assert_eq!(video.video_swarms().len(), 1);
    });
}

/// URIs that don't parse, are too long or carry options outside the
/// egress bounds, and network keys past their bounds, are refused, and
/// nothing is added.
#[test]
fn bad_uris_and_options_are_refused() {
    run(async {
        let world = World::default();
        let b = node_in(&world, 2).await;
        let video = start(&b, Config::default());
        let e = egress(&video);
        let id = SwarmId::new(&[7; 32]);
        let uri = |q: &str| format!("magnet:?xt=urn:ppspp:{id}{q}");
        let refused = [
            ("not a uri".to_owned(), "invalid swarm uri: invalid uri"),
            (
                "magnet:?xt=urn:ppspp:%%".to_owned(),
                "invalid swarm uri: invalid uri",
            ),
            (uri("&x.cs=x"), "invalid swarm uri: invalid option value"),
            (uri(&"&x.a=1".repeat(400)), "swarm uri too long"),
            (uri("&x.cs=128"), "unsupported swarm option: chunk size"),
            (uri("&x.cs=131072"), "unsupported swarm option: chunk size"),
            (
                uri("&x.ps=48"),
                "unsupported swarm option: chunks per signature",
            ),
            (uri("&x.sc=100"), "unsupported swarm option: stream count"),
            (
                uri("&x.im=7"),
                "unsupported swarm option: integrity protection method",
            ),
            (
                uri("&x.hf=6"),
                "unsupported swarm option: merkle hash tree function",
            ),
            (
                uri("&x.sa=9"),
                "unsupported swarm option: live signature algorithm",
            ),
            (
                format!("magnet:?xt=urn:ppspp:{}", SwarmId::new(&[7; 8])),
                "unsupported swarm option: swarm id",
            ),
        ];
        for (u, message) in refused {
            assert_eq!(
                open(&e, &u, &[]).await.err().as_deref(),
                Some(message),
                "{u}"
            );
        }
        let keys = vec![vec![1; 32]; 17];
        assert_eq!(
            open(&e, &uri(""), &keys).await.err().as_deref(),
            Some("wrong number of network keys")
        );
        assert_eq!(
            open(&e, &uri(""), &[vec![1; 65]]).await.err().as_deref(),
            Some("network key too long")
        );
        assert!(video.video_swarms().is_empty());
        assert_eq!(b.host.swarms_of(&id), (0, 0));
        assert!(open(&e, &uri(""), &[]).await.is_ok());
    });
}

/// The node adds at most `max_swarms` swarms for streams and opens at most
/// `max_streams` streams; a closed stream frees its place.
#[test]
fn streams_and_their_swarms_are_bounded() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let config = Config {
            egress: EgressSettings {
                max_swarms: 1,
                max_streams: 2,
                ..EgressSettings::default()
            },
            ..Config::default()
        };
        let x = Publisher::new(&a, 7, capture_options()).await;
        let y = Publisher::new(&a, 8, capture_options()).await;
        let video = start(&b, config);
        let e = egress(&video);
        let (s1, _) = open(&e, &x.uri, &[]).await.unwrap();
        assert_eq!(
            open(&e, &y.uri, &[]).await.err().as_deref(),
            Some("too many swarms")
        );
        let (_s2, _) = open(&e, &x.uri, &[]).await.unwrap();
        assert_eq!(
            open(&e, &x.uri, &[]).await.err().as_deref(),
            Some("too many streams")
        );
        drop(s1);
        settle().await;
        let (_s3, _) = open(&e, &x.uri, &[]).await.unwrap();
        assert_eq!(b.host.swarms_of(&x.id), (0, 1));
        assert_eq!(b.host.swarms_of(&y.id), (0, 0));
    });
}
