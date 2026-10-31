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
//!   address and publishes a synthetic stream to `live/` and the
//!   channel's stream key, paced in simulated time.
//! - m finds the channel's media listing in its view, of the unit format's
//!   MIME type and the ingest swarm's URI, with the channel's live snippet,
//!   and opens a stream of the swarm.
//! - o's own stream of the channel reads the ingest swarm where it is:
//!   the node has one video swarm, the ingest's.
//! - p unpublishes: the session ends, its swarm goes, and the listing
//!   goes from m's view.
//!
//! A run's [`Leg`] sets the stream's GOPs and where m joins: p goes on
//! sending while m opens its stream half way through a GOP. m's
//! low-latency swarm reader starts at the newest chunk it has and egress
//! rewinds a spare reader to the GOP's sync record, through the swarm's
//! backlog from o, so m's records start at that CONFIGS record and the
//! GOP's keyframe, and its units are a contiguous run of the script's. On
//! an uplink too slow for the GOP's bytes to come before the next GOP's
//! sync record and the rewind's grace, the rewind is dropped and m's
//! records start at that sync record. m logs how long its join took and
//! how far behind the head it started.
//!
//! A leg whose m reads on to the stream's end ([`member_to_end`]) checks
//! that the session's END record reaches m, after the script's last unit,
//! before o's swarm goes.
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
use std::future::Future;
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
use strims_video::synth::script::{MetaValue, Script, ScriptConfig, Tag, TagKind};
use strims_video::units::ReaderEvent;
use strims_video::units::record::{Record, UnitHeader, parse_record};
use strims_video::{
    AccessUnit, DecoderConfig, EgressPiece, EgressStream, FlvSource, ReaderMetrics, SourceEvent,
    SwarmKind, TrackId, UnitReader, VideoControl, VideoEvent, units, units_mime,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task::spawn_local;

const TCP_PORT: u16 = 7000;
/// The port o's ingress listens on.
const RTMP_PORT: u16 = 1935;
/// The video track.
const VIDEO: TrackId = TrackId(1);
/// The ingest swarm's integrity unit.
const UNIT_BYTES: u64 = 8 * 4096;

/// What a run streams, and how m joins it.
#[derive(Clone, Copy, Debug)]
struct Leg {
    /// The test that runs it, for the replay command.
    test: &'static str,
    /// Pictures per GOP, at 30 fps.
    gop_frames: u32,
    duration_secs: u32,
    /// The media time p sends up to before m opens its stream, half way
    /// through a GOP.
    join_ms: u32,
    /// o's upload cap, in bytes per second.
    uplink: Option<u64>,
    /// Where the join starts.
    start: JoinStart,
}

/// Where a join starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JoinStart {
    /// At the sync record of the GOP live at the open, rewound to.
    Rewound,
    /// At the next GOP's sync record, the rewind overtaken.
    NextGop,
    /// Either: over an uplink this slow, the GOP's backlog may or may not
    /// arrive in time.
    Either,
}

impl Leg {
    /// The index of the tag p waits at for m.
    fn pause(&self, tags: &[Tag]) -> usize {
        tags.iter()
            .position(|t| t.timestamp >= self.join_ms)
            .expect("a tag after the join")
    }
}

/// A unit as the source made it, without its publish time: track, DTS,
/// PTS, keyframe and payload.
type Unit = (TrackId, i64, i64, bool, Vec<u8>);

fn owned(u: &AccessUnit<'_>) -> Unit {
    (u.track, u.dts, u.pts, u.keyframe, u.payload.to_vec())
}

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
            "magnet:?xt=urn:ppspp:{}&x.im=2&x.hf=7&x.sa=2&x.cs=4096&x.ps=8&x.sc=16",
            SwarmId::new(&self.key)
        )
    }
}

#[derive(Clone)]
struct World {
    seed: u64,
    leg: Leg,
    trace: Trace,
    ids: Rc<RefCell<BTreeMap<String, HostId>>>,
    invitation: Rc<RefCell<Option<Invitation>>>,
    channel: Rc<RefCell<Option<Channel>>>,
    /// The start time of o's live session, in unix seconds.
    start_time: Rc<RefCell<Option<i64>>>,
    script: Rc<Script>,
    /// The units the source makes of `script`, in order.
    units: Rc<Vec<Unit>>,
    /// Steps done.
    done: Rc<RefCell<BTreeSet<String>>>,
}

impl World {
    fn new(seed: u64, leg: Leg) -> World {
        let script = Script::new(&ScriptConfig {
            gop_frames: leg.gop_frames,
            duration_secs: leg.duration_secs,
            ..ScriptConfig::default()
        });
        World {
            seed,
            leg,
            trace: Rc::default(),
            ids: Rc::default(),
            invitation: Rc::default(),
            channel: Rc::default(),
            start_time: Rc::default(),
            units: Rc::new(script_units(&script)),
            script: Rc::new(script),
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
}

/// The units the source makes of `script`, in order.
fn script_units(script: &Script) -> Vec<Unit> {
    let mut source = FlvSource::new();
    let mut units = Vec::new();
    for tag in &script.tags {
        let e = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, epoch()),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, epoch()),
            TagKind::Metadata => continue,
        };
        if let Ok(Some(SourceEvent::Unit(u))) = e {
            units.push(owned(&u));
        }
    }
    units
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

/// What a stream of the unit format yielded: its records, when its first
/// piece came, and its reader's metrics.
struct UnitRun {
    records: Vec<Vec<u8>>,
    first_piece: Time,
    metrics: ReaderMetrics,
}

impl UnitRun {
    fn record(&self, i: usize) -> Record<'_> {
        parse_record(&self.records[i]).unwrap()
    }

    /// The index of the first CONFIGS record.
    fn first_configs(&self) -> usize {
        (0..self.records.len())
            .find(|&i| matches!(self.record(i), Record::Configs { .. }))
            .expect("a CONFIGS record")
    }

    /// The header of record `i`, a video keyframe.
    fn keyframe(&self, i: usize) -> UnitHeader {
        match self.record(i) {
            Record::Unit { header, .. } if header.track == VIDEO && header.keyframe => header,
            r => panic!("record {i} is no video keyframe: {r:?}"),
        }
    }

    /// The header of the first video keyframe after record `i`.
    fn next_keyframe(&self, i: usize) -> UnitHeader {
        (i + 1..self.records.len())
            .find_map(|j| match self.record(j) {
                Record::Unit { header, .. } if header.track == VIDEO && header.keyframe => {
                    Some(header)
                }
                _ => None,
            })
            .expect("a later keyframe")
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

    /// Reads `stream` as a player does, records through a unit reader, up
    /// to the keyframe `gops` GOPs after the first unit. The units are a
    /// contiguous run of the script's from a video keyframe, with its
    /// configs and no gaps; it logs their place among the script's.
    async fn read_units(&self, stream: &mut Stream, gops: usize) -> UnitRun {
        let mut reader = UnitReader::new();
        let (mut records, mut units, mut record) = (Vec::new(), Vec::new(), Vec::new());
        let mut first_piece = None;
        let mut keyframes = 0;
        while keyframes <= gops {
            let piece = tokio::time::timeout(StdDuration::from_secs(60), stream.next(usize::MAX))
                .await
                .expect("a piece in time")
                .expect("a piece");
            first_piece.get_or_insert(self.clock.now());
            match piece {
                EgressPiece::Data(d) => record.extend_from_slice(d),
                EgressPiece::RecordEnd => {
                    let r = std::mem::take(&mut record);
                    if let Some(u) = reader.read(&r) {
                        keyframes += usize::from(u.track == VIDEO && u.keyframe);
                        units.push(owned(&u));
                    }
                    while let Some(e) = reader.poll_event() {
                        assert!(!matches!(e, ReaderEvent::Gap { .. }), "{e:?}");
                    }
                    records.push(r);
                }
                EgressPiece::Discontinuity => {
                    panic!("a discontinuity after {} records", records.len())
                }
            }
        }
        let script = &self.world.units;
        let start = script
            .iter()
            .position(|u| *u == units[0])
            .expect("the first unit is one of the script's");
        let end = start + units.len();
        assert!(script[start].0 == VIDEO && script[start].3, "unit {start}");
        assert!(
            end <= script.len() && units == script[start..end],
            "units {start}..{end} of {} differ",
            script.len()
        );
        assert_eq!(reader.configs(), script_configs(&self.world.script));
        self.log(format!(
            "read units {start}..{end} of {} in {} records",
            script.len(),
            records.len()
        ));
        UnitRun {
            records,
            first_piece: first_piece.unwrap(),
            metrics: stream.metrics(),
        }
    }

    /// Checks where m's stream of the unit format, opened at `opened`,
    /// started, and logs how long its join took and how far behind the
    /// head it started. A rewind brings it to the sync record of the GOP
    /// live at the open; an overtaken or failed one, to the next GOP's.
    fn check_join(&self, run: &UnitRun, opened: Time) {
        let start = run.first_configs();
        let key = run.keyframe(start + 1);
        let next = run.next_keyframe(start + 1);
        let m = run.metrics;
        let rewinds = (
            m.rewinds,
            m.refused_rewinds,
            m.overtaken_rewinds,
            m.discontinuities,
        );
        // The records from the sync record to the head, as of the first
        // piece.
        let behind: usize = run.records[start..]
            .iter()
            .take_while(|r| match parse_record(r).unwrap() {
                Record::Unit { header, .. } => header.published <= run.first_piece,
                _ => true,
            })
            .map(Vec::len)
            .sum();
        let ms = |t: Time| (t - opened).as_millis();
        self.log(format!(
            "joined: first piece at +{} ms, rewinds {rewinds:?}; its keyframe published at \
             {:+} ms, {} ms and {behind} record bytes behind the head at the first piece; the \
             next keyframe published at {:+} ms",
            ms(run.first_piece),
            ms(key.published),
            (run.first_piece - key.published).as_millis(),
            ms(next.published)
        ));

        // Every join starts at a sync record, dropping the records before.
        assert_eq!(start, 0);
        assert!(
            matches!(run.record(0), Record::Configs { gop_back: 0, .. }),
            "record 0: {:?}",
            run.record(0)
        );
        let rewound = match (self.world.leg.start, rewinds) {
            (JoinStart::Rewound | JoinStart::Either, (1, 0, 0, 0)) => true,
            (JoinStart::NextGop | JoinStart::Either, (0, 0, 1, 0)) => false,
            (JoinStart::Either, (0, 1, 0, 0)) => false,
            (s, other) => panic!("{s:?}: rewinds {other:?}"),
        };
        if rewound {
            // It starts behind the head, at a GOP live at the open or
            // after: its reader starts once the swarm has chunks.
            assert!(
                key.published <= run.first_piece,
                "a keyframe after the first piece"
            );
            assert!(
                opened < next.published,
                "a GOP before the one live at the open"
            );
        } else {
            assert!(key.published > opened, "a keyframe from before the open");
        }
        if self.world.leg.start == JoinStart::NextGop {
            // It waited the rewind's grace past the keyframe, not the
            // rewind.
            let grace = strims_video::EgressSettings::default().rewind_grace;
            let waited = run.first_piece - key.published;
            assert!(
                waited < grace + Duration::from_millis(500),
                "first piece {waited:?} after the keyframe"
            );
        }
    }
}

async fn owner(w: World) -> turmoil::Result {
    let h = start(&w, "o", 1).await;
    if let Some(cap) = w.leg.uplink {
        h.node.set_rate_limit(cap).await.unwrap();
        h.log(format!("uploads at most {cap} B/s"));
    }
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
    // A unit-format session creates its swarm once the stream's metadata
    // settles its signing, after the publish's start.
    let swarms = w
        .wait("the ingest swarm", 60, || {
            let swarms = h.video.video_swarms();
            swarms.iter().any(|s| s.added).then_some(swarms)
        })
        .await;
    assert_eq!(swarms.len(), 1);
    assert_eq!(
        (swarms[0].kind, swarms[0].transfer_id, &swarms[0].swarm_uri),
        (SwarmKind::Ingest, t, &channel.uri())
    );
    assert_eq!(swarms[0].network_keys, std::slice::from_ref(&net));

    // o's own stream reads the ingest swarm
    let mut stream = h
        .video
        .open_stream(&channel.uri(), std::slice::from_ref(&net), units::MIME_TYPE)
        .await
        .unwrap();
    assert_eq!(stream.transfer_id(), t);
    let swarms = h.video.video_swarms();
    assert_eq!(swarms.len(), 1);
    assert_eq!((swarms[0].kind, swarms[0].streams), (SwarmKind::Ingest, 1));
    h.log("o's stream reads the ingest swarm");
    let run = h.read_units(&mut stream, 1).await;
    assert!(matches!(run.record(0), Record::Configs { gop_back: 0, .. }));
    assert_eq!(run.metrics.refused_rewinds, 0);
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
    // A record per unit, a sync record per GOP and the session's END; the
    // session's end signed the last partial integrity unit.
    let keyframes = w.units.iter().filter(|u| u.0 == VIDEO && u.3).count();
    assert_eq!(m.segments, (w.units.len() + keyframes + 1) as u64);
    assert_eq!(m.bytes % UNIT_BYTES, 0);
    assert!(m.early_signs > 0);
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
    assert_eq!(media.mime_type, units_mime(&script_configs(&w.script)));
    assert_eq!(
        media.mime_type,
        "video/x.strims-units;codecs=\"avc1.64001f,mp4a.40.2\""
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

    // a stream of the ingest swarm, joined mid stream: half way through a
    // GOP
    w.set("m ready");
    w.step("join", 60).await;
    let opened = h.clock.now();
    let mut stream = h
        .video
        .open_stream(
            &media.swarm_uri,
            std::slice::from_ref(&net),
            &media.mime_type,
        )
        .await
        .unwrap();
    let swarms = h.video.video_swarms();
    assert_eq!(swarms.len(), 1);
    assert_eq!((swarms[0].kind, swarms[0].streams), (SwarmKind::Egress, 1));
    h.log("opened the stream");
    let run = h.read_units(&mut stream, 1).await;
    h.check_join(&run, opened);
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

/// m as [`member`] has it, but for its stream of the unit format, which it
/// opens as `member` does and reads on to the stream's END: a contiguous
/// run of the script's units up to its last, then the END.
async fn member_to_end(w: World) -> turmoil::Result {
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
    let (_, media, _) = w
        .wait("the channel's listing", 120, || {
            h.media()
                .into_iter()
                .find(|(_, _, s)| s.as_ref().is_some_and(|s| s.live))
        })
        .await;
    assert_eq!(media.mime_type, units_mime(&script_configs(&w.script)));
    w.set("m ready");
    w.step("join", 60).await;
    let mut stream = h
        .video
        .open_stream(
            &media.swarm_uri,
            std::slice::from_ref(&net),
            &media.mime_type,
        )
        .await
        .unwrap();
    h.log("opened the stream");

    let mut records = Vec::new();
    let mut record = Vec::new();
    loop {
        let piece = tokio::time::timeout(StdDuration::from_secs(60), stream.next(usize::MAX))
            .await
            .expect("a piece in time")
            .expect("a piece");
        match piece {
            EgressPiece::Data(d) => record.extend_from_slice(d),
            EgressPiece::RecordEnd => {
                let r = std::mem::take(&mut record);
                let end = matches!(parse_record(&r).unwrap(), Record::End { .. });
                records.push(r);
                if end {
                    break;
                }
            }
            EgressPiece::Discontinuity => {
                panic!("a discontinuity after {} records", records.len())
            }
        }
    }
    let ended = h.clock.now();
    let units: Vec<Unit> = records
        .iter()
        .filter_map(|r| match parse_record(r).unwrap() {
            Record::Unit {
                header, payload, ..
            } => Some((
                header.track,
                header.dts,
                header.pts,
                header.keyframe,
                payload.to_vec(),
            )),
            _ => None,
        })
        .collect();
    let script = &w.units;
    let start = script
        .iter()
        .position(|u| *u == units[0])
        .expect("the first unit is one of the script's");
    assert!(
        units == script[start..],
        "units from {start} differ from the script's"
    );
    let m = stream.metrics();
    h.log(format!(
        "read units {start}..{} of {} and the END in {} records, rewinds {:?}",
        script.len(),
        script.len(),
        records.len(),
        (m.rewinds, m.refused_rewinds, m.discontinuities)
    ));

    h.log(format!(
        "read the END at {} ms",
        (ended - epoch()).as_millis()
    ));
    w.step("p done", 120).await;
    w.wait("the listing to go", 60, || {
        h.media().is_empty().then_some(())
    })
    .await;
    h.log("the listing went");
    drop(stream);
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
    let pause = w.leg.pause(&tags);
    let start = tokio::time::Instant::now();
    p.send_paced(&tags[..pause], start).await;
    p.log(format!("sent {pause} tags; waiting for m"));
    let paused = tokio::time::Instant::now();
    w.step("m ready", 25).await;
    w.set("join");
    let resumed = start + (tokio::time::Instant::now() - paused);
    p.send_paced(&tags[pause..], resumed).await;
    p.log(format!("sent {} tags", tags.len()));
    p.publisher.unpublish().unwrap();
    p.until("the server to close", 30, |p| p.closed).await;
    p.log("unpublished; the server closed the connection");
    w.set("p done");
    Ok(())
}

fn run(seed: u64, leg: Leg) -> Vec<String> {
    let w = World::new(seed, leg);
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
        panic!("seed {seed}: {e}\n{}", replay(seed, leg));
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

fn replay(seed: u64, leg: Leg) -> String {
    format!(
        "replay: STRIMS_TURMOIL_SEED={seed} cargo test -p strims-node --features turmoil \
         --test turmoil_video {} -- --exact --nocapture",
        leg.test
    )
}

/// Runs `leg` under the seed to replay or 7, prints its trace, and checks
/// that the seed gives the same trace again and the next seed another.
fn check(leg: Leg) {
    // a seed to replay, from the replay command
    #[allow(clippy::disallowed_methods)]
    let seed = std::env::var("STRIMS_TURMOIL_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);
    let a = run(seed, leg);
    for l in &a {
        eprintln!("{l}");
    }
    eprintln!("seed {seed}: {} lines, {}", a.len(), digest(&a));
    let b = run(seed, leg);
    assert_eq!(
        digest(&a),
        digest(&b),
        "same seed, different traces\n{}",
        replay(seed, leg)
    );
    assert_ne!(
        digest(&a),
        digest(&run(seed + 1, leg)),
        "different seeds, same trace\n{}",
        replay(seed, leg)
    );
}

/// [`run`] with `m` in place of [`member`] as m's host.
fn run_with<F: Future<Output = turmoil::Result> + 'static>(
    seed: u64,
    leg: Leg,
    m: fn(World) -> F,
) -> Vec<String> {
    let w = World::new(seed, leg);
    let mut sim = turmoil::Builder::new()
        .rng_seed(seed)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_secs(1_700_000_000))
        .simulation_duration(StdDuration::from_secs(900))
        .min_message_latency(StdDuration::from_millis(1))
        .max_message_latency(StdDuration::from_millis(15))
        .build();
    sim.client("o", owner(w.clone()));
    sim.client("m", m(w.clone()));
    sim.client("p", publisher(w.clone()));
    let res = sim.run();
    let trace = w.trace.borrow().clone();
    if let Err(e) = res {
        for l in &trace {
            eprintln!("{l}");
        }
        panic!("seed {seed}: {e}\n{}", replay(seed, leg));
    }
    trace
}

/// [`check`] with `m` in place of [`member`] as m's host.
fn check_with<F: Future<Output = turmoil::Result> + 'static>(leg: Leg, m: fn(World) -> F) {
    // a seed to replay, from the replay command
    #[allow(clippy::disallowed_methods)]
    let seed = std::env::var("STRIMS_TURMOIL_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);
    let a = run_with(seed, leg, m);
    for l in &a {
        eprintln!("{l}");
    }
    eprintln!("seed {seed}: {} lines, {}", a.len(), digest(&a));
    let b = run_with(seed, leg, m);
    assert_eq!(
        digest(&a),
        digest(&b),
        "same seed, different traces\n{}",
        replay(seed, leg)
    );
    assert_ne!(
        digest(&a),
        digest(&run_with(seed + 1, leg, m)),
        "different seeds, same trace\n{}",
        replay(seed, leg)
    );
}

/// A leg of `gop_secs`-second GOPs, joined at `join_ms`.
const fn units(test: &'static str, gop_secs: u32, duration_secs: u32, join_ms: u32) -> Leg {
    Leg {
        test,
        gop_frames: gop_secs * 30,
        duration_secs,
        join_ms,
        uplink: None,
        start: JoinStart::Rewound,
    }
}

#[test]
fn a_late_joiner_starts_at_its_gops_keyframe_with_1s_gops() {
    check(units(
        "a_late_joiner_starts_at_its_gops_keyframe_with_1s_gops",
        1,
        10,
        3500,
    ));
}

#[test]
fn a_late_joiner_starts_at_its_gops_keyframe_with_4s_gops() {
    check(units(
        "a_late_joiner_starts_at_its_gops_keyframe_with_4s_gops",
        4,
        16,
        6000,
    ));
}

/// At 2 Mbit/s, under twice the stream's rate, the 2.3 s of a 4-second
/// GOP m joins in may or may not come from o before the next GOP's sync
/// record and the rewind's grace.
#[test]
fn a_late_joiner_on_a_slow_uplink_starts_at_a_keyframe_and_reads_on() {
    check(Leg {
        uplink: Some(250_000),
        start: JoinStart::Either,
        ..units(
            "a_late_joiner_on_a_slow_uplink_starts_at_a_keyframe_and_reads_on",
            4,
            16,
            6300,
        )
    });
}

/// At 2 Mbit/s, o can't send the 5 s of an 8-second GOP m joins in before
/// the next GOP's sync record comes, 3 s later, and the rewind's grace
/// passes: m starts at that record, without waiting for the rewind.
#[test]
fn a_joiner_whose_rewind_the_next_gop_overtakes_starts_there() {
    check(Leg {
        uplink: Some(250_000),
        start: JoinStart::NextGop,
        ..units(
            "a_joiner_whose_rewind_the_next_gop_overtakes_starts_there",
            8,
            30,
            13000,
        )
    });
}

/// The session's END reaches m, a viewer on another node, after the last
/// unit and before o's swarm goes.
#[test]
fn a_remote_viewer_reads_the_end_of_a_session() {
    check_with(
        units("a_remote_viewer_reads_the_end_of_a_session", 1, 10, 3500),
        member_to_end,
    );
}

/// On o's 2 Mbit/s uplink too, the END reaches m before o's swarm goes.
#[test]
fn a_remote_viewer_on_a_slow_uplink_reads_the_end_of_a_session() {
    check_with(
        Leg {
            uplink: Some(250_000),
            ..units(
                "a_remote_viewer_on_a_slow_uplink_reads_the_end_of_a_session",
                4,
                16,
                6300,
            )
        },
        member_to_end,
    );
}
