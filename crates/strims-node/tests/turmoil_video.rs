// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Live video on whole nodes, under turmoil. Every node runs a profile
//! store and the network, transfer, directory and video controls, the
//! video control's RTMP ingress listening through a `NetVideoHost`, and
//! serves its peers' RPC links with the network and transfer controls.
//!
//! - o creates a network, serves its directory and invites m, who joins
//!   and links to o. o's profile owns a video channel on the network, and
//!   o enables its ingress, listening on port 1935 and advertising
//!   `o:1935`.
//! - p, a plain host without a node, connects to the channel's RTMP
//!   address and publishes a synthetic stream of 1-second GOPs to `live/`
//!   and the channel's stream key, paced in simulated time. A third of
//!   the way through, it waits until m reads the stream, so m joins with
//!   most of the stream to come.
//! - m finds the channel's media listing in its view, of the records'
//!   MIME type and the ingest swarm's URI, with the channel's live
//!   snippet, and opens a stream of the swarm. Its records, joined mid
//!   stream, are a contiguous run of those the script produces.
//! - o's own stream of the channel reads the ingest swarm where it is:
//!   the node has one video swarm, the ingest's.
//! - p unpublishes: the session ends, its swarm goes, and the listing
//!   goes from m's view.
//!
//! Every host appends what it sees, stamped with simulated time, to one
//! trace. The same seed must give the same trace, and another seed a
//! different one. `STRIMS_TURMOIL_SEED=<s>` runs seed `s` in place of 7.

#![cfg(feature = "turmoil")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use futures_util::StreamExt;
use prost::Message as _;
use sha2::{Digest, Sha256};
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::{key_from_seed, membership_trusted};
use strims_dao::schema::profile::Profile as ProfileRecord;
use strims_dao::seal::StorageKey;
use strims_dao::{ProfileStore, Recv, ReplicatedStore, Singleton};
use strims_directory::{DirectoryControl, DirectoryEvent, ListingEvent};
use strims_kv::MemoryKv;
use strims_network::{NetworkControl, NetworkFrontend};
use strims_node::app::{NodeHost, NodeTimer, PeerServices};
use strims_node::video_host::NetVideoHost;
use strims_node::{Clock, HostId, Node, NodeConfig, NodeEvent, SimClock, SwarmId, TurmoilNet};
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateNetworkFromInvitationRequest, CreateServerRequest, Invitation,
    NetworkFrontendService,
};
use strims_proto::profile::v1::Profile;
use strims_proto::service::Context;
use strims_proto::video::v1::VideoIngressConfig;
use strims_transfer::{TransferControl, TransferId};
use strims_video::amf0::Value;
use strims_video::hls::HlsRoutes;
use strims_video::synth::publisher::{Publisher, PublisherConfig};
use strims_video::synth::script::{
    MetaValue, Script, ScriptConfig, Tag, TagKind, expected_records,
};
use strims_video::{
    CutRule, DecoderConfig, EgressPiece, EgressStream, FlvSource, SourceEvent, SwarmKind,
    VideoControl, VideoEvent, fmp4_mime,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task::spawn_local;

const TCP_PORT: u16 = 7000;
/// The port o's ingress listens on.
const RTMP_PORT: u16 = 1935;
/// The fraction of the script p sends before it waits for m.
const BEFORE_PAUSE: usize = 3;
/// The whole records m reads.
const RECORDS: usize = 3;

fn epoch() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

type Trace = Rc<RefCell<Vec<String>>>;
type Timer = NodeTimer<SimClock>;
type Network = NetworkControl<MemoryKv, NodeHost, Timer>;
type Directory = DirectoryControl<MemoryKv, NodeHost, Timer>;
type Video = VideoControl<MemoryKv, NodeHost, NetVideoHost<TurmoilNet>, Timer>;
type Stream = EgressStream<MemoryKv, NodeHost, NetVideoHost<TurmoilNet>, Timer>;

/// What o tells the others about its channel.
#[derive(Clone)]
struct Channel {
    /// The channel's public key: its swarm's id.
    key: Vec<u8>,
    /// The RTMP address and stream key of its URL.
    server_addr: String,
    stream_key: String,
}

impl Channel {
    /// The URI of the channel's ingest swarm.
    fn uri(&self) -> String {
        format!(
            "magnet:?xt=urn:ppspp:{}&x.im=2&x.hf=4&x.sa=2&x.cs=1024&x.ps=32&x.sc=16",
            SwarmId::new(&self.key)
        )
    }
}

#[derive(Clone)]
struct World {
    seed: u64,
    trace: Trace,
    ids: Rc<RefCell<BTreeMap<String, HostId>>>,
    invitation: Rc<RefCell<Option<Invitation>>>,
    channel: Rc<RefCell<Option<Channel>>>,
    /// The start time of o's live session, in unix seconds.
    start_time: Rc<RefCell<Option<i64>>>,
    script: Rc<Script>,
    /// The records `script` produces.
    expected: Rc<Vec<Vec<u8>>>,
    /// Steps done.
    done: Rc<RefCell<BTreeSet<String>>>,
}

impl World {
    fn new(seed: u64) -> World {
        let script = Script::new(&ScriptConfig {
            gop_frames: 30,
            duration_secs: 10,
            ..ScriptConfig::default()
        });
        let expected = expected_records(&script, CutRule::default());
        World {
            seed,
            trace: Rc::default(),
            ids: Rc::default(),
            invitation: Rc::default(),
            channel: Rc::default(),
            start_time: Rc::default(),
            script: Rc::new(script),
            expected: Rc::new(expected),
            done: Rc::default(),
        }
    }

    fn log(&self, clock: &SimClock, name: &str, line: impl Into<String>) {
        let t = (clock.now() - epoch()).as_millis();
        self.trace
            .borrow_mut()
            .push(format!("{t} {name} {}", line.into()));
    }

    fn profile(&self, name: &str) -> Profile {
        Profile {
            id: u64::from(name.as_bytes()[0]),
            name: name.to_owned(),
            key: Some(self.key(name)),
            device_id: 1,
            ..Default::default()
        }
    }

    fn key(&self, name: &str) -> strims_proto::types::Key {
        let mut s = [name.as_bytes()[0]; 32];
        s[..8].copy_from_slice(&self.seed.to_le_bytes());
        key_from_seed(&s)
    }

    fn uri(&self, name: &str) -> String {
        let ip = turmoil::lookup(name);
        format!(
            "tcp://{ip}:{TCP_PORT}/{}",
            hex::encode(&self.key(name).public)
        )
    }

    fn name(&self, id: &HostId) -> String {
        self.ids
            .borrow()
            .iter()
            .find(|(_, i)| *i == id)
            .map(|(n, _)| n.clone())
            .unwrap_or_default()
    }

    fn set(&self, step: &str) {
        self.done.borrow_mut().insert(step.to_owned());
    }

    /// Waits up to `secs` seconds for `step` to be done.
    async fn step(&self, step: &str, secs: u64) {
        self.wait(step, secs, || {
            self.done.borrow().contains(step).then_some(())
        })
        .await;
    }

    /// Polls `f` every 100 ms of simulated time until it has a value.
    async fn wait<T>(&self, what: &str, secs: u64, mut f: impl FnMut() -> Option<T>) -> T {
        let wait = async {
            loop {
                if let Some(v) = f() {
                    return v;
                }
                tokio::time::sleep(StdDuration::from_millis(100)).await;
            }
        };
        tokio::time::timeout(StdDuration::from_secs(secs), wait)
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
    }

    /// Where `got` starts in the expected records, which it is a
    /// contiguous run of.
    fn run_of(&self, got: &[Vec<u8>]) -> usize {
        let expected = &self.expected;
        let start = expected
            .iter()
            .position(|r| *r == got[0])
            .expect("the first record is one of the expected");
        let end = start + got.len();
        assert!(
            end <= expected.len() && got == &expected[start..end],
            "records {start}..{end} of {} differ",
            expected.len()
        );
        start
    }
}

/// The decoder configs of `script`, in init order.
fn script_configs(script: &Script) -> Vec<DecoderConfig> {
    let mut source = FlvSource::new();
    let mut configs = Vec::new();
    for tag in &script.tags {
        let e = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, epoch()),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, epoch()),
            TagKind::Metadata => continue,
        };
        if let Ok(Some(SourceEvent::Config(c))) = e {
            configs.push(c);
        }
    }
    configs
}

fn snippet() -> ListingSnippet {
    ListingSnippet {
        title: "synthetic bars".into(),
        description: "a test pattern".into(),
        channel_name: "o's channel".into(),
        ..Default::default()
    }
}

/// A directory event, for the trace.
fn describe_directory(e: &DirectoryEvent) -> String {
    match e {
        DirectoryEvent::Started { .. } => "dir started".into(),
        DirectoryEvent::Stopped { .. } => "dir stopped".into(),
        DirectoryEvent::Listing { event, .. } => match event {
            ListingEvent::Changed(l) => format!(
                "dir listing {} changed snippet={}",
                l.id,
                l.snippet.as_ref().map_or("-", |s| s.title.as_str())
            ),
            ListingEvent::Unpublished(l) => format!("dir listing {} unpublished", l.id),
            ListingEvent::UserCountChanged { id, user_count, .. } => {
                format!("dir listing {id} users={user_count}")
            }
        },
        DirectoryEvent::User {
            listing_id,
            kind,
            user,
            ..
        } => format!("dir user {kind:?} {listing_id} {}", user.alias),
        DirectoryEvent::AssetBundle { .. } => "dir assets".into(),
        DirectoryEvent::Lagged { .. } => "dir lagged".into(),
    }
}

/// A video event, for the trace.
fn describe_video(e: &VideoEvent) -> String {
    match e {
        VideoEvent::ChannelChanged(c) => format!("video channel {} changed", c.id),
        VideoEvent::ChannelDeleted(c) => format!("video channel {} deleted", c.id),
        VideoEvent::IngressConfigChanged(c) => format!(
            "video ingress config enabled={} addr={} public={}",
            c.enabled, c.server_addr, c.public_server_addr
        ),
        VideoEvent::IngressListenFailed { addr, error } => {
            format!("video ingress listen on {addr} failed: {error}")
        }
        VideoEvent::IngressStarted { channel_id } => format!("video ingest {channel_id} started"),
        VideoEvent::IngressEnded { channel_id, reason } => {
            format!("video ingest {channel_id} ended: {reason}")
        }
        e => format!("video {e:?}"),
    }
}

struct Started {
    world: World,
    name: String,
    node: Node,
    clock: SimClock,
    network: Network,
    frontend: NetworkFrontend<MemoryKv, NodeHost, Timer>,
    directory: Directory,
    video: Video,
    /// Peers bound to the network, by name.
    bound: Rc<RefCell<BTreeMap<String, bool>>>,
    /// The video control's events so far.
    video_events: Rc<RefCell<Vec<VideoEvent>>>,
}

async fn start(w: &World, name: &str, index: u64) -> Started {
    let ip: IpAddr = turmoil::lookup(name);
    let profile = w.profile(name);
    let mut config = NodeConfig::new(profile.key.clone().unwrap(), w.seed.wrapping_add(index));
    config.label = name.to_owned();
    config.tcp_listen = Some(format!("0.0.0.0:{TCP_PORT}"));
    config.public_ip = Some(ip);
    config.max_peers = 16;
    let clock = SimClock::turmoil(epoch());
    let (node, mut events) = Node::spawn(config, TurmoilNet, clock).await.unwrap();
    w.ids.borrow_mut().insert(name.to_owned(), node.host_id());

    let store = ProfileStore::create(
        MemoryKv::new(),
        profile.id,
        StorageKey::from_bytes([index as u8; 32]),
        det_rng(w.seed ^ index),
        clock.now(),
    )
    .await
    .unwrap();
    let mut tx = store.write(clock.now()).await.unwrap();
    ProfileRecord::set(&mut tx, profile.clone()).await.unwrap();
    tx.commit().await.unwrap();
    let store = ReplicatedStore::new(store).await.unwrap();

    let host = NodeHost::new(node.clone());
    let network = NetworkControl::new(
        host.clone(),
        store.clone(),
        profile.clone(),
        NodeTimer(clock),
        det_rng(w.seed.wrapping_mul(31) ^ index),
        strims_network::Config::default(),
    );
    let transfers = TransferControl::new(
        host.clone(),
        NodeTimer(clock),
        strims_transfer::Config {
            search_interval: Duration::from_secs(5),
            ..strims_transfer::Config::default()
        },
    );
    let directory = DirectoryControl::new(
        host.clone(),
        store.clone(),
        network.clone(),
        transfers.clone(),
        NodeTimer(clock),
        det_rng(w.seed.wrapping_mul(37) ^ index),
        strims_directory::Config::default(),
    );
    let video = VideoControl::new(
        host,
        NetVideoHost(TurmoilNet),
        store,
        profile,
        network.clone(),
        transfers.clone(),
        directory.clone(),
        HlsRoutes::new(),
        NodeTimer(clock),
        det_rng(w.seed.wrapping_mul(41) ^ index),
        strims_video::Config::default(),
    );
    spawn_local({
        let network = network.clone();
        async move { network.run().await }
    });
    spawn_local({
        let transfers = transfers.clone();
        async move { transfers.run().await }
    });
    let mut dir_events = directory.subscribe();
    spawn_local({
        let directory = directory.clone();
        async move { directory.run().await }
    });
    let mut events_of_video = video.subscribe();
    spawn_local({
        let video = video.clone();
        async move { video.run().await }
    });
    let (w2, me) = (w.clone(), name.to_owned());
    spawn_local(async move {
        while let Some(e) = dir_events.next().await {
            let line = match e {
                Recv::Event(e) => describe_directory(&e),
                Recv::Lagged => "dir events lagged".into(),
            };
            w2.log(&clock, &me, line);
        }
    });
    let video_events: Rc<RefCell<Vec<VideoEvent>>> = Rc::default();
    let (w2, me, seen) = (w.clone(), name.to_owned(), video_events.clone());
    spawn_local(async move {
        while let Some(e) = events_of_video.next().await {
            match e {
                Recv::Event(e) => {
                    w2.log(&clock, &me, describe_video(&e));
                    seen.borrow_mut().push(VideoEvent::clone(&e));
                }
                Recv::Lagged => w2.log(&clock, &me, "video events lagged"),
            }
        }
    });

    let services = PeerServices::new(node.clone())
        .with(Rc::new(network.clone()))
        .with(Rc::new(transfers));
    let bound: Rc<RefCell<BTreeMap<String, bool>>> = Rc::default();
    let (w2, me, b2) = (w.clone(), name.to_owned(), bound.clone());
    spawn_local(async move {
        while let Some(e) = events.recv().await {
            match &e {
                NodeEvent::PeerAdded(p) => {
                    w2.log(&clock, &me, format!("peer+ {}", w2.name(&p.host_id)))
                }
                NodeEvent::NetworkPeerAdded { host_id, .. } => {
                    let peer = w2.name(host_id);
                    w2.log(&clock, &me, format!("net+ {peer}"));
                    b2.borrow_mut().insert(peer, true);
                }
                NodeEvent::NetworkPeerRemoved { host_id, .. } => {
                    let peer = w2.name(host_id);
                    w2.log(&clock, &me, format!("net- {peer}"));
                    b2.borrow_mut().insert(peer, false);
                }
                NodeEvent::TransferError { host_id, error } => {
                    w2.log(
                        &clock,
                        &me,
                        format!("transfer error {} {error}", w2.name(host_id)),
                    );
                }
                _ => {}
            }
            services.handle_event(&e).await;
        }
    });
    let frontend = NetworkFrontend::new(network.clone());
    Started {
        world: w.clone(),
        name: name.to_owned(),
        node,
        clock,
        network,
        frontend,
        directory,
        video,
        bound,
        video_events,
    }
}

impl Started {
    fn log(&self, line: impl Into<String>) {
        self.world.log(&self.clock, &self.name, line);
    }

    fn is_bound(&self, peer: &str) -> bool {
        self.bound.borrow().get(peer).copied().unwrap_or(false)
    }

    /// This side's id and key of its one network.
    fn network(&self) -> (u64, Vec<u8>) {
        let n = self.network.networks()[0].network.clone();
        let key = strims_dao::schema::network::network_key(&n);
        (n.id, key)
    }

    fn trusted(&self) -> bool {
        let now = self.clock.now();
        self.network.networks().first().is_some_and(|n| {
            n.network
                .certificate
                .as_ref()
                .is_some_and(|c| membership_trusted(c, now))
        })
    }

    /// The media listings of this side's view: id, content and snippet.
    fn media(&self) -> Vec<(u64, Media, Option<ListingSnippet>)> {
        let (network_id, _) = self.network();
        self.directory
            .listings(network_id)
            .into_iter()
            .filter_map(|l| match l.listing.content {
                Some(Content::Media(m)) => Some((l.id, m, l.snippet)),
                _ => None,
            })
            .collect()
    }

    /// Whether the video control reported an event `f` picks.
    fn saw(&self, f: impl Fn(&VideoEvent) -> bool) -> bool {
        self.video_events.borrow().iter().any(f)
    }

    /// The next `n` whole records of `stream`, logged with their place
    /// among the expected records.
    async fn read_records(&self, stream: &mut Stream, n: usize) -> Vec<Vec<u8>> {
        let mut got = Vec::new();
        let mut record = Vec::new();
        while got.len() < n {
            let piece = tokio::time::timeout(StdDuration::from_secs(60), stream.next(usize::MAX))
                .await
                .expect("a piece in time")
                .expect("a piece");
            match piece {
                EgressPiece::Data(d) => record.extend_from_slice(d),
                EgressPiece::RecordEnd => got.push(std::mem::take(&mut record)),
                EgressPiece::Discontinuity => panic!("a discontinuity after {} records", got.len()),
            }
        }
        let start = self.world.run_of(&got);
        let sizes: Vec<usize> = got.iter().map(Vec::len).collect();
        self.log(format!(
            "read records {start}..{} of {}: {sizes:?} bytes",
            start + got.len(),
            self.world.expected.len()
        ));
        got
    }
}

async fn owner(w: World) -> turmoil::Result {
    let h = start(&w, "o", 1).await;
    h.frontend
        .create_server(
            &Context::default(),
            CreateServerRequest {
                name: "net".into(),
                alias: "olly".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .network
        .unwrap();
    w.wait("o to serve the directory", 60, || {
        h.directory
            .networks()
            .first()
            .is_some_and(|n| n.serving)
            .then_some(())
    })
    .await;
    h.log("serves the directory");
    let (network_id, net) = h.network();
    let invitation = h
        .frontend
        .create_invitation(
            &Context::default(),
            CreateInvitationRequest {
                network_id,
                bootstrap_client_id: 0,
            },
        )
        .await
        .unwrap()
        .invitation
        .unwrap();
    *w.invitation.borrow_mut() = Some(invitation);

    // a channel, and the ingress listening for it
    let channel = h.video.create_channel(&net, snippet()).await.unwrap();
    h.video
        .set_ingress_config(VideoIngressConfig {
            enabled: true,
            server_addr: format!("0.0.0.0:{RTMP_PORT}"),
            public_server_addr: format!("o:{RTMP_PORT}"),
            ..Default::default()
        })
        .await
        .unwrap();
    w.wait("the ingress to listen", 60, || {
        (h.video.metrics().ingress.listens > 0).then_some(())
    })
    .await;
    let url = h.video.channel_url(channel.id).await.unwrap();
    assert_eq!(
        url.url,
        format!("rtmp://o:{RTMP_PORT}/live/{}", url.stream_key)
    );
    h.log(format!("ingress listening for {}", url.server_addr));
    let channel = Channel {
        key: channel.key.as_ref().unwrap().public.clone(),
        server_addr: url.server_addr,
        stream_key: url.stream_key,
    };
    *w.channel.borrow_mut() = Some(channel.clone());

    // p's publish starts the channel's session
    w.wait("the ingest to start", 120, || {
        h.saw(|e| matches!(e, VideoEvent::IngressStarted { .. }))
            .then_some(())
    })
    .await;
    let live = h.video.ingress_streams();
    assert_eq!(live.len(), 1, "{live:?}");
    *w.start_time.borrow_mut() = Some(live[0].created_at);
    let t = TransferId::new(&SwarmId::new(&channel.key), b"");
    let swarms = h.video.video_swarms();
    assert_eq!(swarms.len(), 1);
    assert_eq!(
        (swarms[0].kind, swarms[0].transfer_id, &swarms[0].swarm_uri),
        (SwarmKind::Ingest, t, &channel.uri())
    );
    assert_eq!(swarms[0].network_keys, std::slice::from_ref(&net));

    // o's own stream reads the ingest swarm
    let mut stream = h
        .video
        .open_stream(&channel.uri(), std::slice::from_ref(&net))
        .await
        .unwrap();
    assert_eq!(stream.transfer_id(), t);
    let swarms = h.video.video_swarms();
    assert_eq!(swarms.len(), 1);
    assert_eq!((swarms[0].kind, swarms[0].streams), (SwarmKind::Ingest, 1));
    h.log("o's stream reads the ingest swarm");
    h.read_records(&mut stream, 2).await;
    drop(stream);
    assert_eq!(h.video.video_swarms()[0].streams, 0);

    // p's unpublish ends the session and takes its swarm down
    w.wait("the ingest to end", 120, || {
        h.saw(|e| matches!(e, VideoEvent::IngressEnded { .. }))
            .then_some(())
    })
    .await;
    assert!(h.saw(|e| *e
        == VideoEvent::IngressEnded {
            channel_id: live[0].channel_id,
            reason: "the publisher ended the stream".into(),
        }));
    assert!(h.video.ingress_streams().is_empty());
    assert!(h.video.video_swarms().is_empty());
    let m = h.video.metrics().ingress;
    h.log(format!(
        "ingress: connections={} sessions={} segments={} bytes={} refused={}",
        m.connections, m.sessions, m.segments, m.bytes, m.refused
    ));
    assert_eq!((m.connections, m.sessions, m.refused), (1, 1, 0));
    assert_eq!(m.segments, w.expected.len() as u64);
    assert_eq!(
        m.bytes,
        w.expected.iter().map(|r| r.len() as u64).sum::<u64>()
    );
    assert_eq!((m.dropped_units, m.media_errors, m.write_errors), (0, 0, 0));
    w.step("m done", 120).await;
    w.set("o done");
    Ok(())
}

async fn member(w: World) -> turmoil::Result {
    let h = start(&w, "m", 2).await;
    let invitation = w
        .wait("an invitation", 60, || w.invitation.borrow().clone())
        .await;
    h.frontend
        .create_network_from_invitation(
            &Context::default(),
            CreateNetworkFromInvitationRequest {
                alias: "milly".into(),
                invitation: Some(InvitationField::InvitationBytes(invitation.encode_to_vec())),
            },
        )
        .await
        .unwrap();
    h.node.dial(&w.uri("o")).await.unwrap();
    w.wait("m to bind with o", 120, || {
        (h.is_bound("o") && h.trusted()).then_some(())
    })
    .await;
    h.log("renewed and bound with o");
    w.set("m joined");
    let (_, net) = h.network();
    let channel = w
        .wait("o's channel", 60, || w.channel.borrow().clone())
        .await;

    // the channel's listing, with its live snippet
    let (listing_id, media, shown) = w
        .wait("the channel's listing and snippet", 120, || {
            h.media()
                .into_iter()
                .find(|(_, _, s)| s.as_ref().is_some_and(|s| s.live))
        })
        .await;
    h.log(format!("found the channel as {listing_id}"));
    // the server shows only snippets the listing's swarm key signed: the
    // channel's
    assert_eq!(media.swarm_uri, channel.uri());
    assert_eq!(media.mime_type, fmp4_mime(&script_configs(&w.script)));
    assert_eq!(
        media.mime_type,
        "video/mp4;codecs=\"avc1.64001f,mp4a.40.2\""
    );
    let shown = shown.unwrap();
    assert_eq!(
        (
            shown.title.as_str(),
            shown.channel_name.as_str(),
            shown.live
        ),
        ("synthetic bars", "o's channel", true)
    );
    let start_time = w
        .wait("o's session start", 60, || *w.start_time.borrow())
        .await;
    assert_eq!(shown.start_time, start_time);

    // a stream of the ingest swarm, joined mid stream
    let mut stream = h
        .video
        .open_stream(&media.swarm_uri, std::slice::from_ref(&net))
        .await
        .unwrap();
    let swarms = h.video.video_swarms();
    assert_eq!(swarms.len(), 1);
    assert_eq!((swarms[0].kind, swarms[0].streams), (SwarmKind::Egress, 1));
    h.log("opened the stream");
    w.set("m reads");
    h.read_records(&mut stream, RECORDS).await;
    drop(stream);

    // the session's end takes the listing down
    w.step("p done", 120).await;
    w.wait("the listing to go", 60, || {
        h.media().is_empty().then_some(())
    })
    .await;
    h.log("the listing went");
    w.wait("the stream's swarm to go", 60, || {
        h.video.video_swarms().is_empty().then_some(())
    })
    .await;
    w.set("m done");
    w.step("o done", 120).await;
    Ok(())
}

/// p's RTMP connection and the publisher on it.
struct Publish {
    w: World,
    clock: SimClock,
    stream: turmoil::net::TcpStream,
    publisher: Publisher,
    input: Vec<u8>,
    output: Vec<u8>,
    /// The server closed the connection.
    closed: bool,
}

impl Publish {
    fn log(&self, line: impl Into<String>) {
        self.w.log(&self.clock, "p", line);
    }

    /// Writes what the publisher queued.
    async fn flush(&mut self) {
        loop {
            let n = self.publisher.poll_transmit(&mut self.output);
            if n == 0 {
                return;
            }
            self.stream.write_all(&self.output[..n]).await.unwrap();
        }
    }

    /// Takes and answers the server's bytes until `deadline`, or until
    /// the server closes the connection.
    async fn exchange(&mut self, deadline: tokio::time::Instant) {
        while !self.closed {
            self.flush().await;
            let read = tokio::select! {
                biased;
                r = self.stream.read(&mut self.input) => Some(r),
                () = tokio::time::sleep_until(deadline) => None,
            };
            match read {
                None => return,
                Some(Ok(0)) | Some(Err(_)) => self.closed = true,
                Some(Ok(n)) => self.publisher.handle_input(&self.input[..n]).unwrap(),
            }
        }
    }

    /// Exchanges with the server until `done` holds, within `secs`
    /// seconds.
    async fn until(&mut self, what: &str, secs: u64, done: impl Fn(&Publish) -> bool) {
        let end = tokio::time::Instant::now() + StdDuration::from_secs(secs);
        while !done(self) {
            let now = tokio::time::Instant::now();
            assert!(now < end && !self.closed, "{what} didn't happen");
            self.exchange(now + StdDuration::from_millis(10)).await;
        }
    }

    fn send(&mut self, tag: &Tag) {
        match tag.kind {
            TagKind::Video => self.publisher.send_video(tag.timestamp, &tag.body).unwrap(),
            TagKind::Audio => self.publisher.send_audio(tag.timestamp, &tag.body).unwrap(),
            TagKind::Metadata => {
                let fields: Vec<(&str, Value<'_>)> = self
                    .w
                    .script
                    .metadata
                    .iter()
                    .map(|(k, v)| {
                        let v = match v {
                            MetaValue::Number(n) => Value::Number(*n),
                            MetaValue::Bool(b) => Value::Boolean(*b),
                            MetaValue::String(s) => Value::String(s),
                        };
                        (k.as_str(), v)
                    })
                    .collect();
                self.publisher.send_metadata(&fields).unwrap();
            }
        }
    }

    /// Sends `tags`, each at its time after `start`.
    async fn send_paced(&mut self, tags: &[Tag], start: tokio::time::Instant) {
        for tag in tags {
            let due = start + StdDuration::from_millis(u64::from(tag.timestamp));
            self.exchange(due).await;
            assert!(!self.closed, "the server closed the connection");
            self.send(tag);
        }
        self.flush().await;
    }
}

async fn publisher(w: World) -> turmoil::Result {
    let clock = SimClock::turmoil(epoch());
    w.step("m joined", 300).await;
    let channel = w
        .wait("o's channel", 60, || w.channel.borrow().clone())
        .await;
    let stream = turmoil::net::TcpStream::connect(channel.server_addr.as_str())
        .await
        .unwrap();
    let config = PublisherConfig::new("live", &channel.stream_key);
    let mut p = Publish {
        w: w.clone(),
        clock,
        stream,
        publisher: Publisher::new(config, &mut det_rng(w.seed ^ 0x70)),
        input: vec![0; 64 * 1024],
        output: vec![0; 64 * 1024],
        closed: false,
    };
    p.log(format!("connected to {}", channel.server_addr));
    p.until("the publish's answer", 30, |p| {
        p.publisher.publish_accepted() || p.publisher.refused().is_some()
    })
    .await;
    assert!(
        p.publisher.publish_accepted(),
        "{:?}",
        p.publisher.refused()
    );
    p.log(format!(
        "publishing; server digest {:?}",
        p.publisher.server_digest()
    ));

    let tags = w.script.tags.clone();
    let pause = tags.len() / BEFORE_PAUSE;
    let start = tokio::time::Instant::now();
    p.send_paced(&tags[..pause], start).await;
    p.log(format!("sent {pause} tags; waiting for m"));
    let paused = tokio::time::Instant::now();
    w.step("m reads", 25).await;
    let resumed = start + (tokio::time::Instant::now() - paused);
    p.send_paced(&tags[pause..], resumed).await;
    p.log(format!("sent {} tags", tags.len()));
    p.publisher.unpublish().unwrap();
    p.until("the server to close", 30, |p| p.closed).await;
    p.log("unpublished; the server closed the connection");
    w.set("p done");
    Ok(())
}

fn run(seed: u64) -> Vec<String> {
    let w = World::new(seed);
    let mut sim = turmoil::Builder::new()
        .rng_seed(seed)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_secs(1_700_000_000))
        .simulation_duration(StdDuration::from_secs(900))
        .min_message_latency(StdDuration::from_millis(1))
        .max_message_latency(StdDuration::from_millis(15))
        .build();
    sim.client("o", owner(w.clone()));
    sim.client("m", member(w.clone()));
    sim.client("p", publisher(w.clone()));
    let res = sim.run();
    let trace = w.trace.borrow().clone();
    if let Err(e) = res {
        for l in &trace {
            eprintln!("{l}");
        }
        panic!("seed {seed}: {e}\n{}", replay(seed));
    }
    trace
}

fn digest(trace: &[String]) -> String {
    let mut h = Sha256::new();
    for l in trace {
        h.update(l.as_bytes());
        h.update(b"\n");
    }
    hex::encode(h.finalize())
}

fn replay(seed: u64) -> String {
    format!(
        "replay: STRIMS_TURMOIL_SEED={seed} cargo test -p strims-node --features turmoil \
         --test turmoil_video an_rtmp_stream_reaches_a_member_on_whole_nodes -- --exact --nocapture"
    )
}

#[test]
fn an_rtmp_stream_reaches_a_member_on_whole_nodes() {
    // a seed to replay, from the replay command
    #[allow(clippy::disallowed_methods)]
    let seed = std::env::var("STRIMS_TURMOIL_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);
    let a = run(seed);
    for l in &a {
        eprintln!("{l}");
    }
    eprintln!("seed {seed}: {} lines, {}", a.len(), digest(&a));
    let b = run(seed);
    assert_eq!(
        digest(&a),
        digest(&b),
        "same seed, different traces\n{}",
        replay(seed)
    );
    assert_ne!(
        digest(&a),
        digest(&run(seed + 1)),
        "different seeds, same trace\n{}",
        replay(seed)
    );
}
