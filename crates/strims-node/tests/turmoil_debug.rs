// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Mock streams on whole nodes, under turmoil. Every node runs a profile
//! store and the network, transfer, directory and debug controls, and
//! serves its peers' RPC links with the network and transfer controls.
//!
//! - o creates a network, serves its directory and invites m, who joins
//!   and links to o.
//! - o starts a mock stream on the network: 1000 kbps, a segment every
//!   500 ms, no timeout. The stream lists itself on the directory.
//! - Once m's view holds the listing, m's debug config enables mock
//!   streams on the network, and m reads the stream: segments carrying
//!   o's stream id, each read within a bound of when o wrote it, from the
//!   stream's swarm on m's node.
//! - o stops the stream, and its swarm goes. The listing goes from m's
//!   view, and m's reader and swarm go with it.
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
use strims_debug::{DebugControl, MOCK_STREAM_SERVICE, ReaderInfo};
use strims_directory::{DirectoryControl, DirectoryEvent, ListingEvent};
use strims_kv::MemoryKv;
use strims_network::{NetworkControl, NetworkFrontend};
use strims_node::app::{NodeHost, NodeTimer, PeerServices};
use strims_node::{Clock, HostId, Node, NodeConfig, NodeEvent, SimClock, SwarmId};
use strims_ppspp::{TICK_INTERVAL, Uri};
use strims_proto::debug::v1::Config as DebugConfig;
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::directory::listing::Content;
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateNetworkFromInvitationRequest, CreateServerRequest, Invitation,
    NetworkFrontendService,
};
use strims_proto::profile::v1::Profile;
use strims_proto::service::Context;
use strims_transfer::TransferControl;
use tokio::task::spawn_local;

const TCP_PORT: u16 = 7000;
/// o's mock stream.
const BITRATE_KBPS: u32 = 1000;
const SEGMENT_INTERVAL_MS: u32 = 500;
/// The segments m reads before o stops the stream.
const SEGMENTS: u64 = 6;
/// The simulated network's delay of each message.
const MIN_MESSAGE_LATENCY: StdDuration = StdDuration::from_millis(1);
const MAX_MESSAGE_LATENCY: StdDuration = StdDuration::from_millis(15);

fn epoch() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

/// The longest a segment may take from o's write to m's read. Its HAVEs
/// go out at a tick of o's runner and m requests it at a tick of its own,
/// so it waits at most two ticks; the HAVE, the request and the data each
/// take at most the network's delay, and the bound allows twice that.
fn max_segment_latency() -> Duration {
    TICK_INTERVAL * 2 + Duration::from(MAX_MESSAGE_LATENCY * 2 * 3)
}

type Trace = Rc<RefCell<Vec<String>>>;
type Timer = NodeTimer<SimClock>;
type Network = NetworkControl<MemoryKv, NodeHost, Timer>;
type Directory = DirectoryControl<MemoryKv, NodeHost, Timer>;
type Debugger = DebugControl<MemoryKv, NodeHost, Timer>;

/// What o tells m about its mock stream, once listed.
#[derive(Clone)]
struct Listed {
    id: u64,
    listing_id: u64,
    swarm: SwarmId,
}

#[derive(Clone)]
struct World {
    seed: u64,
    trace: Trace,
    ids: Rc<RefCell<BTreeMap<String, HostId>>>,
    invitation: Rc<RefCell<Option<Invitation>>>,
    stream: Rc<RefCell<Option<Listed>>>,
    /// Steps done.
    done: Rc<RefCell<BTreeSet<String>>>,
}

impl World {
    fn new(seed: u64) -> World {
        World {
            seed,
            trace: Rc::default(),
            ids: Rc::default(),
            invitation: Rc::default(),
            stream: Rc::default(),
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

/// A directory event, for the trace.
fn describe_directory(e: &DirectoryEvent) -> String {
    match e {
        DirectoryEvent::Started { .. } => "dir started".into(),
        DirectoryEvent::Stopped { .. } => "dir stopped".into(),
        DirectoryEvent::Listing { event, .. } => match event {
            ListingEvent::Changed(l) => format!("dir listing {} changed", l.id),
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

struct Started {
    world: World,
    name: String,
    node: Node,
    clock: SimClock,
    network: Network,
    frontend: NetworkFrontend<MemoryKv, NodeHost, Timer>,
    directory: Directory,
    debug: Debugger,
    /// Peers bound to the network, by name.
    bound: Rc<RefCell<BTreeMap<String, bool>>>,
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
    let (node, mut events) = Node::spawn(config, strims_node::TurmoilNet, clock)
        .await
        .unwrap();
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
        profile,
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
    let debug = DebugControl::new(
        host,
        store,
        network.clone(),
        transfers.clone(),
        directory.clone(),
        NodeTimer(clock),
        det_rng(w.seed.wrapping_mul(43) ^ index),
        strims_debug::Config::default(),
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
    spawn_local({
        let debug = debug.clone();
        async move { debug.run().await }
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
        debug,
        bound,
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

    /// The ids of the mock stream listings of this side's view.
    fn mock_listings(&self) -> Vec<u64> {
        let (network_id, _) = self.network();
        self.directory
            .listings(network_id)
            .into_iter()
            .filter(|l| {
                matches!(&l.listing.content,
                    Some(Content::Service(s)) if s.r#type == MOCK_STREAM_SERVICE)
            })
            .map(|l| l.id)
            .collect()
    }

    /// The reader of the listing `listing_id`.
    fn reader_of(&self, listing_id: u64) -> Option<ReaderInfo> {
        let readers = self.debug.readers();
        readers.into_iter().find(|r| r.listing_id == listing_id)
    }

    /// The readers and writers the node holds open on `swarm`, if it runs
    /// the swarm.
    async fn swarm_handles(&self, swarm: &SwarmId) -> Option<(usize, usize)> {
        let m = self.node.metrics().await.unwrap();
        m.runner
            .swarms
            .iter()
            .find(|s| s.id == *swarm)
            .map(|s| (s.readers, s.publishers))
    }

    /// Waits up to 60 seconds for the node to drop `swarm`.
    async fn swarm_gone(&self, swarm: &SwarmId) {
        let wait = async {
            while self.swarm_handles(swarm).await.is_some() {
                tokio::time::sleep(StdDuration::from_millis(100)).await;
            }
        };
        tokio::time::timeout(StdDuration::from_secs(60), wait)
            .await
            .unwrap_or_else(|_| panic!("{}'s node runs swarm {swarm}", self.name));
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

    // the mock stream, and its listing on the directory
    let id = h
        .debug
        .start_mock_stream(BITRATE_KBPS, SEGMENT_INTERVAL_MS, 0, net.clone())
        .await
        .unwrap();
    let listing_id = w
        .wait("the stream's listing", 60, || {
            h.debug.mock_streams().first().and_then(|s| s.listing_id)
        })
        .await;
    let streams = h.debug.mock_streams();
    assert_eq!(streams.len(), 1);
    let s = &streams[0];
    assert_eq!(
        (
            s.id,
            &s.network_key,
            s.bitrate_kbps,
            s.segment_interval_ms,
            s.timeout_ms
        ),
        (id, &net, BITRATE_KBPS, SEGMENT_INTERVAL_MS, 0)
    );
    let swarm = Uri::parse(&s.swarm_uri).unwrap().id;
    assert_eq!(h.swarm_handles(&swarm).await, Some((0, 1)));
    h.log(format!("lists mock stream {id} as {listing_id}"));
    *w.stream.borrow_mut() = Some(Listed {
        id,
        listing_id,
        swarm: swarm.clone(),
    });

    // the stop takes the stream and its swarm down
    w.step("m read", 300).await;
    h.debug.stop_mock_stream(id);
    w.wait("the stream to end", 60, || {
        h.debug.mock_streams().is_empty().then_some(())
    })
    .await;
    h.swarm_gone(&swarm).await;
    let m = h.debug.metrics();
    h.log(format!(
        "stopped the stream: segments={} write errors={} host errors={}",
        m.segments_written, m.write_errors, m.host_errors
    ));
    assert_eq!((m.mock_streams_started, m.mock_streams_ended), (1, 1));
    assert_eq!((m.write_errors, m.host_errors), (0, 0));
    w.set("o stopped");
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
    let (_, net) = h.network();
    let listed = w
        .wait("o's mock stream", 120, || w.stream.borrow().clone())
        .await;

    // the listing in m's view, and the config that reads it
    w.wait("the listing in m's view", 120, || {
        h.mock_listings().contains(&listed.listing_id).then_some(())
    })
    .await;
    h.log(format!("found listing {}", listed.listing_id));
    assert!(h.debug.readers().is_empty());
    h.debug
        .set_config(DebugConfig {
            enable_mock_streams: true,
            mock_stream_network_key: net.clone(),
        })
        .await
        .unwrap();

    // each segment's latency, as the reader reports it: at least one
    // message's delay, as the segment crosses the network
    let (min, max) = (Duration::from(MIN_MESSAGE_LATENCY), max_segment_latency());
    let mut seen = 0;
    let reader = w
        .wait("the segments", 120, || {
            let r = h.reader_of(listed.listing_id)?;
            if r.segments > seen {
                seen = r.segments;
                let latency = r.latency.expect("a read segment sets the latency");
                h.log(format!("read segment {}: latency {latency}", r.segments));
                assert!(
                    latency >= min && latency <= max,
                    "segment {} latency {latency}",
                    r.segments
                );
            }
            (r.segments >= SEGMENTS).then_some(r)
        })
        .await;
    assert_eq!(reader.network_key, net);
    assert_eq!(reader.swarm_id, listed.swarm);
    assert_eq!(reader.id, Some(listed.id));
    assert_eq!(reader.malformed, 0);
    assert!(reader.last_segment_at.is_some());
    assert_eq!(h.swarm_handles(&listed.swarm).await, Some((1, 0)));
    h.log(format!(
        "reads mock stream {}: discontinuities={}",
        listed.id, reader.discontinuities
    ));
    w.set("m read");

    // the stop takes the listing, the reader and its swarm down
    w.step("o stopped", 120).await;
    w.wait("the listing to go", 60, || {
        h.mock_listings().is_empty().then_some(())
    })
    .await;
    h.log("the listing went");
    w.wait("the reader to end", 60, || {
        h.debug.readers().is_empty().then_some(())
    })
    .await;
    h.swarm_gone(&listed.swarm).await;
    let m = h.debug.metrics();
    h.log(format!(
        "the reader ended: segments={} read errors={} host errors={}",
        m.segments_read, m.read_errors, m.host_errors
    ));
    assert_eq!((m.readers_started, m.readers_ended), (1, 1));
    assert_eq!((m.read_errors, m.host_errors), (0, 0));
    w.set("m done");
    w.step("o done", 120).await;
    Ok(())
}

fn run(seed: u64) -> Vec<String> {
    let w = World::new(seed);
    let mut sim = turmoil::Builder::new()
        .rng_seed(seed)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_secs(1_700_000_000))
        .simulation_duration(StdDuration::from_secs(900))
        .min_message_latency(MIN_MESSAGE_LATENCY)
        .max_message_latency(MAX_MESSAGE_LATENCY)
        .build();
    sim.client("o", owner(w.clone()));
    sim.client("m", member(w.clone()));
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
         --test turmoil_debug a_mock_stream_is_read_on_whole_nodes_until_it_stops -- --exact --nocapture"
    )
}

#[test]
fn a_mock_stream_is_read_on_whole_nodes_until_it_stops() {
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
