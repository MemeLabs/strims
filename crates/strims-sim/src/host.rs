// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Each host's software: the node, its event policy, link supervision,
//! the swarm writer and readers, and the transfers that bind the swarms'
//! channels.
//!
//! turmoil re-runs [`host_main`] when a host restarts, so everything here
//! starts from the spec and the shared registry: a restarted host joins the
//! same networks and swarms with a new host key, its seed continues the
//! stream from the current time, and its viewers rejoin at the live edge
//! ([`joins_live`]) and keep accumulating into the same statistics. A host
//! with [`HostSpec::arrive_at`] starts at its arrival, or with the run
//! and joins its swarms at its arrival where the testbed harness's join
//! ([`HostSpec::links_at`]) boots it early, and one that departs
//! gracefully leaves through [`depart`].
//!
//! [`HostSpec::arrive_at`]: crate::HostSpec::arrive_at
//! [`HostSpec::links_at`]: crate::HostSpec::links_at
//!
//! Every node's stores keep each chunk of a stream as its frames' sequence
//! number and write time ([`FrameCodec`]), or a media chunk's
//! ([`MediaCodec`]), not its bytes.

use std::cell::Cell;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::Stream;
use strims_base::time::Time;
use strims_node::app::{NodeTimer, PeerHandler, PeerServices};
use strims_node::event::{TimerRecord, TimerSource};
use strims_node::{
    HostId, NetworkId, NetworkOptions, Node, NodeError, NodeEvent, NodeEvents,
    PeerIndexPublisherId, PeerIndexSearch, PeerInfo, ReadError, SimClock, SwarmId, SwarmKey,
    SwarmReader, SwarmWriter, WriterOptions,
};
use strims_ppspp::store::ChunkBytes;
use strims_ppspp::{DeliveryMode, SwarmOptions};
use strims_rpc::{Client, Router};
use strims_transfer::{TransferControl, TransferId};
use tokio::task::spawn_local;

use crate::APP_PORT;
use crate::net::SimNet;
use crate::pool::{Pool, PoolConfig};
use crate::spec::{
    ARRIVAL_REWIND, Binding, Departure, Fault, HostIx, LinkSpec, MediaSpec, SwarmSpec, Transport,
};
use crate::stream::{
    FRAME_LEN, FrameCodec, FrameParser, JoinRewind, MEDIA_HEADER, MediaCodec, MediaParser,
    encode_frame, fill_media, media_header,
};
use crate::world::{HASH_TABLE_SALT, PEER_INDEX_KEY, World, sim_now, sim_now_ms};
use strims_wan::live::Signer;

/// How long a dial may take before the supervisor gives up on it.
const DIAL_TIMEOUT: Duration = Duration::from_secs(30);

/// Swarm `s`'s options at host `h`: its stores keep [`FrameCodec`]
/// state, or a media swarm's [`MediaCodec`] state, unless the scenario
/// keeps the bytes
/// ([`Scenario::stored_chunks`](crate::Scenario::stored_chunks)) or the
/// chunks aren't whole frames. A viewer that joins `live` reads from the
/// live edge ([`joins_live`]).
fn swarm_options(w: &World, s: usize, spec: &SwarmSpec, live: bool) -> SwarmOptions {
    let mut o = spec.options.clone();
    if live {
        o.delivery_mode = DeliveryMode::LOW_LATENCY;
    }
    let swarm = u32::try_from(s).unwrap_or(u32::MAX);
    if w.spec.stored_chunks {
        return o;
    }
    if spec.media.is_some() {
        o.chunk_bytes = ChunkBytes::Codec(Arc::new(MediaCodec::new(w.seed, swarm)));
    } else if FrameCodec::fits(o.chunk_size) {
        o.chunk_bytes = ChunkBytes::Codec(Arc::new(FrameCodec::new(w.seed, swarm)));
    }
    o
}

/// Whether incarnation `inc` of host `h` (1 for the first) is a viewer of
/// `spec` that joins at the live edge, as a player that joins or rejoins a
/// live stream does: one that arrives
/// ([`HostSpec::arrive_at`](crate::HostSpec::arrive_at)), or any
/// incarnation after its first (a churn's restart or a fault's,
/// [`Fault::Restart`]). A viewer there from the run reads as the swarm's
/// options say until it restarts. A host crashed before it ever ran has
/// its first incarnation at its restart, and reads as the swarm's options
/// say: the viewers `calibration_star` boots at or before the stream's
/// start, as the testbed's viewers at the start read.
fn joins_live(w: &World, h: HostIx, inc: u32, spec: &SwarmSpec) -> bool {
    spec.viewers.contains(&h)
        && (inc > 1 || w.spec.hosts.get(h).is_some_and(|x| x.arrive_at.is_some()))
}

pub(crate) async fn host_main(w: Rc<World>, h: HostIx) -> turmoil::Result {
    let arrive = w.spec.hosts.get(h).and_then(|s| s.arrive_at);
    // A host with the harness's join boots with the run, or at its restart
    // (HostSpec::links_at).
    let boots_early = w.spec.hosts.get(h).is_some_and(|s| s.links_at.is_some());
    if !boots_early
        && let Some(at) = arrive
        && at > sim_now()
    {
        tokio::time::sleep(at - sim_now()).await;
    }
    let inc = w.begin_incarnation(h);
    let config = w.node_config(h, inc).ok_or("unknown host")?;
    let rtc_stack = w.spec.hosts.get(h).and_then(|s| s.rtc_stack);
    // A browser's held channel writers poll its pacer as the browser's
    // (`Scenario::pacer_lateness`, `Scenario::pacer_turns`), each poll in
    // its timer log.
    let browser = crate::late::HostKind::of_host(&w.spec, h) != crate::late::HostKind::Native;
    let pacer = crate::net::PacerTimers::new(
        browser,
        w.lateness.get(h).filter(|_| w.spec.pacer_lateness).cloned(),
        w.spec.pacer_turns,
        w.lateness
            .get(h)
            .filter(|_| w.spec.pacer_hold_start)
            .cloned()
            .zip(w.hold_starts.get(h).cloned()),
        w.timer_log(h).filter(|_| browser).map(|l| l.sink()),
        w.clock_lead(h),
    );
    let net = SimNet::new(w.send_buffers.clone())
        .with_wan(w.wan.clone())
        .with_segment(w.spec.segment())
        .with_rtc_stack(rtc_stack.filter(|_| w.spec.window_model()))
        .with_setup_draws(w.setup_draws.get(h).cloned().flatten())
        .with_pacer(pacer);
    // The node's clock, off true time by the run's epoch phase and the
    // host's offset (`Scenario::epoch_phase`, `Scenario::clock_skew`), as
    // is the transfers' timer below.
    let clock = SimClock::turmoil(w.clock_epoch(h));
    // Its timers fire late (`Scenario::timer_lateness`).
    let late = w
        .lateness
        .get(h)
        .map(|l| crate::late::LateClock::new(clock, l.clone()));
    let (node, events) = if let Some(late) = late {
        strims_node::spawn_with_tls(config, net, late, strims_node::LinkTls::default()).await?
    } else {
        Node::spawn(config, net, clock).await?
    };
    // Peers may link before the setup below finishes; they must know us.
    w.register_id(h, node.host_id());

    let mut nets: Vec<Option<NetworkId>> = vec![None; w.spec.networks.len()];
    for (n, spec) in w.spec.networks.iter().enumerate() {
        if !spec.members.contains(&h) {
            continue;
        }
        let key = w.network_keys.get(n).ok_or("network key")?;
        let auto_bind_port = (spec.binding == Binding::Auto).then_some(spec.port);
        let net = node
            .add_network(&key.public, NetworkOptions { auto_bind_port })
            .await?;
        node.set_vpn_handler(net, APP_PORT).await?;
        if let Some(slot) = nets.get_mut(n) {
            *slot = Some(net);
        }
    }
    if let Some(p) = &w.spec.probes
        && let Some(Some(net)) = nets.get(p.network)
    {
        spawn_local(publish(w.clone(), h, node.clone(), *net));
    }

    let index = w
        .spec
        .discovery
        .and_then(|n| nets.get(n).copied().flatten());
    let cap = w
        .spec
        .is_browser(h)
        .then(|| w.spec.exp_browser_cap())
        .flatten();
    let rate = w
        .spec
        .swarms
        .iter()
        .find(|s| s.seed == h)
        .map_or(0.0, SwarmSpec::rate);
    let hog = w
        .spec
        .hosts
        .get(h)
        .is_some_and(|s| s.class == crate::HostClass::Hostile("hog"));
    let cfg = if hog {
        Some(PoolConfig::greedy(&w.spec.exp))
    } else {
        PoolConfig::from_flags(&w.spec.exp, cap)
    };
    let pool = index.zip(cfg).map(|(net, cfg)| {
        let cfg = PoolConfig {
            stream_rate: rate,
            ..cfg
        };
        Rc::new(Pool::new(node.clone(), net, cfg, w.clone(), h))
    });
    if let Some(p) = &pool {
        let p = p.clone();
        spawn_local(async move { p.run().await });
    }
    if let Some(p) = &pool {
        let p = p.clone();
        spawn_local(async move { p.resolve_sources().await });
    }
    let transfers = TransferControl::new(
        SimTransferHost {
            node: node.clone(),
            index,
            pool: pool.clone(),
        },
        NodeTimer(SimClock::turmoil(w.clock_epoch(h))),
        strims_transfer::Config {
            discovery: index.is_some(),
            ..strims_transfer::Config::default()
        },
    );
    spawn_local({
        let transfers = transfers.clone();
        async move { transfers.run().await }
    });
    // Booted ahead of its arrival: it links to its network now (the
    // supervisor holds its swarms' links for its join), and joins its
    // swarms at its arrival.
    let ahead = boots_early && arrive.is_some_and(|at| at > sim_now());
    if ahead {
        w.register(h, &node, nets.clone(), None);
        let services =
            PeerServices::new(node.clone()).with(Rc::new(SimTransfers(transfers.clone())));
        spawn_local(event_task(
            w.clone(),
            h,
            node.clone(),
            events,
            nets.clone(),
            services,
            pool,
        ));
        spawn_tasks(&w, h, &node);
        if let Some(at) = arrive {
            tokio::time::sleep(at.saturating_sub(sim_now())).await;
        }
        let (held, swarms) = join_swarms(&w, h, inc, &node, index, &transfers).await?;
        w.register_swarms(h, &swarms);
        spawn_departure(&w, h, &node, &transfers, held);
        std::future::pending::<()>().await;
        return Ok(());
    }
    let (held, swarms) = join_swarms(&w, h, inc, &node, index, &transfers).await?;
    w.register(h, &node, nets.clone(), Some(&swarms));
    spawn_departure(&w, h, &node, &transfers, held);
    let services = PeerServices::new(node.clone()).with(Rc::new(SimTransfers(transfers)));
    spawn_local(event_task(
        w.clone(),
        h,
        node.clone(),
        events,
        nets,
        services,
        pool,
    ));
    spawn_tasks(&w, h, &node);
    std::future::pending::<()>().await;
    Ok(())
}

/// Host `h`'s swarms, joined: each created (a writer task at its seed) or
/// added (a reader task at a viewer), and its transfer published. Returns
/// the transfers it holds and each scenario swarm's key at the host.
async fn join_swarms(
    w: &Rc<World>,
    h: HostIx,
    inc: u32,
    node: &Node,
    index: Option<NetworkId>,
    transfers: &TransferControl<SimTransferHost, NodeTimer<SimClock>>,
) -> Result<(Vec<(TransferId, SwarmKey)>, Vec<Option<SwarmKey>>), Box<dyn std::error::Error>> {
    let mut swarms: Vec<Option<SwarmKey>> = vec![None; w.spec.swarms.len()];
    let mut held: Vec<(TransferId, SwarmKey)> = Vec::new();
    for (s, spec) in w.spec.swarms.iter().enumerate() {
        let key = w.swarm_keys.get(s).ok_or("swarm key")?;
        let live = joins_live(w, h, inc, spec);
        let swarm = if spec.seed == h {
            let (swarm, _, writer) = node
                .create_swarm(WriterOptions {
                    swarm_options: swarm_options(w, s, spec, live),
                    key: key.clone(),
                })
                .await?;
            match spec.media {
                Some(m) => spawn_local(media_writer_task(w.clone(), h, s, writer, m)),
                None => spawn_local(writer_task(
                    w.clone(),
                    h,
                    s,
                    writer,
                    w.lateness.get(h).cloned(),
                )),
            };
            // `pool-find`: the writer names itself the swarm's source.
            if w.spec.exp.contains("pool-find")
                && let Some(net) = index
                && let Err(e) = node
                    .publish_vpn_address(net, key.clone(), crate::pool::SOURCE_SALT.to_vec(), 0)
                    .await
            {
                w.log(w.name(h), format!("source publish failed: {e}"));
            }
            swarm
        } else if spec.member(h) {
            let swarm = node
                .add_swarm(SwarmId::new(&key.public), swarm_options(w, s, spec, live))
                .await?;
            if spec.viewers.contains(&h) {
                spawn_local(reader_task(w.clone(), h, s, node.clone(), swarm, live));
            }
            swarm
        } else {
            continue;
        };
        let t = transfers.add(swarm, &SwarmId::new(&key.public), Vec::new());
        transfers.publish(t, ());
        held.push((t, swarm));
        if let Some(slot) = swarms.get_mut(s) {
            *slot = Some(swarm);
        }
    }
    Ok((held, swarms))
}

/// Host `h`'s graceful departure, if it has one ([`depart`]).
fn spawn_departure(
    w: &Rc<World>,
    h: HostIx,
    node: &Node,
    transfers: &TransferControl<SimTransferHost, NodeTimer<SimClock>>,
    held: Vec<(TransferId, SwarmKey)>,
) {
    if let Some((at, Departure::Graceful)) = w.spec.departure(h) {
        spawn_local(depart(
            w.clone(),
            h,
            node.clone(),
            transfers.clone(),
            held,
            at,
        ));
    }
}

/// Host `h`'s fault control, if a fault needs it ([`control`]), and its
/// link supervisor.
fn spawn_tasks(w: &Rc<World>, h: HostIx, node: &Node) {
    if w.spec.faults.iter().any(|f| match f.fault {
        Fault::Uplink(x, _) | Fault::Connect { from: x, .. } => x == h,
        _ => false,
    }) {
        spawn_local(control(w.clone(), h, node.clone()));
    }
    if w.spec.timer_probe
        && let (Some(log), Some(s)) = (w.timer_log(h), w.spec.swarms.first())
    {
        let ms = |t: Duration| u64::try_from(w.clock_at(h, t).unix_millis()).unwrap_or(0);
        let (from, until) = (ms(s.start), ms(s.stop));
        let probe = ProbeClock {
            w: w.clone(),
            h,
            late: w.probe_lateness.get(h).cloned(),
        };
        let log = log.clone();
        spawn_local(async move { strims_testbed::node::probe(&probe, &log, from, until).await });
    }
    spawn_local(supervisor(w.clone(), h, node.clone()));
}

/// A host's side of the timer probe ([`Scenario::timer_probe`]): its
/// node's clock, and timers as late as its probe's lateness stream draws
/// them, on time without one.
///
/// [`Scenario::timer_probe`]: crate::Scenario::timer_probe
struct ProbeClock {
    w: Rc<World>,
    h: HostIx,
    late: Option<crate::late::Lateness>,
}

impl strims_testbed::node::Runtime for ProbeClock {
    fn now_ms(&self) -> u64 {
        u64::try_from(self.precise_now().unix_millis()).unwrap_or(0)
    }

    fn precise_now(&self) -> Time {
        self.w.clock_at(self.h, sim_now())
    }

    async fn sleep_until_ms(&self, at_ms: u64) {
        let at = Time::from_unix_millis(i64::try_from(at_ms).unwrap_or(i64::MAX));
        let at = (at - self.w.clock_epoch(self.h))
            .to_std()
            .unwrap_or_default();
        let due = at_ms <= self.now_ms();
        let late = match &self.late {
            // A browser's sleep for a deadline already due resolves at once.
            Some(l) if due && l.kind().resolves_due_at_once() => return,
            Some(l) => probe_lateness(l, due).to_std().unwrap_or_default(),
            None => Duration::ZERO,
        };
        tokio::time::sleep((at + late).saturating_sub(sim_now())).await;
    }

    fn cpu_ms(&self) -> Option<u64> {
        None
    }

    async fn line(&self, _: strims_testbed::node::LineParts) {}

    fn error(&self, message: String) {
        self.w.log(self.w.name(self.h), message);
    }
}

/// How late a probe's timer fires, armed for a deadline `due` already or
/// not: as a harness's timers do on the host ([`crate::late::Source::Writer`]).
fn probe_lateness(l: &crate::late::Lateness, due: bool) -> strims_base::time::Duration {
    l.draw(crate::late::Source::Writer, due)
}

/// Publishes the host's hash table record and peer index entry at
/// startup, before it has links. Publishes repeat only every 10 minutes,
/// so lookups find these records because vpn gives them to links that
/// come up later (see [`strims_vpn::RECORD_PUSH_DELAY`]).
async fn publish(w: Rc<World>, h: HostIx, node: Node, net: NetworkId) {
    let me = w.name(h).to_owned();
    let key = w.hash_table_key(h);
    if let Err(e) = node
        .hash_table_set(net, key, HASH_TABLE_SALT.to_vec(), w.hash_table_value(h))
        .await
    {
        w.log(&me, format!("hash table set failed: {e}"));
    }
    if let Err(e) = node
        .peer_index_publish(net, PEER_INDEX_KEY.to_vec(), w.peer_index_salt(h), APP_PORT)
        .await
    {
        w.log(&me, format!("peer index publish failed: {e}"));
    }
    w.log(&me, "published records");
}

/// The node as the transfers' host. Transfers are published on one
/// network, `()`: every linked peer, so every peer that adds a swarm runs
/// it with every other linked peer that has it. With
/// [`Scenario::discovery`](crate::Scenario), they are advertised and
/// searched for in that vpn network's peer index (`index`), and hosts
/// found there are linked by peer exchange, or offered to the host's
/// [`Pool`]; otherwise the scenario's topology is fixed and nothing is
/// discovered.
struct SimTransferHost {
    node: Node,
    index: Option<NetworkId>,
    pool: Option<Rc<Pool>>,
}

/// The hosts a peer index search finds; none without one.
struct Found(Option<PeerIndexSearch>);

impl Stream for Found {
    type Item = HostId;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<HostId>> {
        match &mut self.0 {
            Some(s) => Pin::new(s).poll_next(cx).map(|h| h.map(|h| h.host_id)),
            None => Poll::Ready(None),
        }
    }
}

impl strims_transfer::Host for SimTransferHost {
    type Network = ();
    type Swarm = SwarmKey;
    type Publication = Option<PeerIndexPublisherId>;
    type Search = Found;
    type Error = NodeError;

    fn host_id(&self) -> HostId {
        self.node.host_id()
    }

    async fn run_swarm(
        &self,
        host: HostId,
        swarm: SwarmKey,
        channel: u64,
        peer_channel: u64,
    ) -> Result<(), NodeError> {
        self.node
            .run_swarm(host, swarm, channel, peer_channel)
            .await
    }

    async fn stop_swarm(&self, host: HostId, swarm: SwarmKey) -> Result<(), NodeError> {
        match self.node.stop_swarm(host, swarm).await {
            Err(NodeError::UnknownPeer) => Ok(()),
            r => r,
        }
    }

    async fn index_publish(
        &self,
        _: (),
        key: Vec<u8>,
        salt: Vec<u8>,
    ) -> Result<Option<PeerIndexPublisherId>, NodeError> {
        match self.index {
            Some(net) => self
                .node
                .peer_index_publish(net, key, salt, 0)
                .await
                .map(Some),
            None => Ok(None),
        }
    }

    async fn index_unpublish(&self, p: Option<PeerIndexPublisherId>) -> Result<(), NodeError> {
        match p {
            Some(p) => self.node.peer_index_unpublish(p).await,
            None => Ok(()),
        }
    }

    async fn index_search(&self, _: (), key: Vec<u8>, salt: Vec<u8>) -> Result<Found, NodeError> {
        match self.index {
            Some(net) => Ok(Found(Some(
                self.node.peer_index_search(net, key, salt).await?,
            ))),
            None => Ok(Found(None)),
        }
    }

    async fn connect_peer(&self, _: (), host: HostId) -> Result<bool, NodeError> {
        match (self.index, &self.pool) {
            (Some(_), Some(pool)) => pool.offer(host).await,
            (Some(net), None) => self.node.connect_peer(net, host).await,
            (None, _) => Ok(false),
        }
    }
}

/// The transfers as a peer handler: every linked peer is on `()`.
struct SimTransfers(TransferControl<SimTransferHost, NodeTimer<SimClock>>);

impl PeerHandler for SimTransfers {
    fn add_peer(&self, peer: &PeerInfo, router: &Router, client: Client) {
        self.0.add_peer(peer.host_id, router, client);
        self.0.add_network_peer((), peer.host_id);
    }

    fn remove_peer(&self, host: HostId) {
        self.0.remove_peer(host);
    }

    fn handle_event(&self, e: &NodeEvent) {
        if let NodeEvent::TransferError { host_id, .. } = e {
            self.0.transfer_failed(*host_id);
        }
    }
}

/// The host's event policy: serve transfers on every peer's RPC link,
/// bind explicit networks, record vpn probes and exchanges, and log it
/// all.
async fn event_task(
    w: Rc<World>,
    h: HostIx,
    node: Node,
    mut events: NodeEvents,
    nets: Vec<Option<NetworkId>>,
    services: PeerServices,
    pool: Option<Rc<Pool>>,
) {
    let me = w.name(h).to_owned();
    while let Some(e) = events.recv().await {
        services.handle_event(&e).await;
        if let Some(p) = &pool {
            p.handle_event(&e);
        }
        match e {
            NodeEvent::PeerAdded(p) => {
                w.peer_changed(h, true);
                let peer = w.ix_of(&p.host_id);
                let dir = if p.dialed { "to" } else { "from" };
                w.log(&me, format!("peer+ {dir} {} {:?}", w.label(peer), p.kind));
                // A host that left takes no link: one a dial started before
                // it left, or a peer's that came in, closes at once.
                if w.departed(h) {
                    let _ = node.close_peer(p.host_id).await;
                    continue;
                }
                // A pin names peers: the seed pins its pinned viewers that
                // are peers now, the new one among them.
                if let Some((swarm, pin)) = w.seed_pin(h)
                    && let Err(e) = node.pin_seed_allocation(swarm, Some(pin)).await
                {
                    w.log(&me, format!("pin failed: {e}"));
                }
                let Some(px) = peer else {
                    continue;
                };
                for (n, net) in nets.iter().enumerate() {
                    let (Some(net), Some(spec)) = (net, w.spec.networks.get(n)) else {
                        continue;
                    };
                    if spec.binding == Binding::Explicit
                        && spec.members.contains(&px)
                        && let Err(e) = node
                            .bind_peer_network(p.host_id, *net, spec.port, spec.port)
                            .await
                    {
                        w.log(&me, format!("bind {} failed: {e}", w.name(px)));
                    }
                }
            }
            NodeEvent::PeerRemoved { host_id } => {
                w.peer_changed(h, false);
                let peer = w.ix_of(&host_id);
                w.log(&me, format!("peer- {}", w.label(peer)));
                w.peer_removed(h, peer);
            }
            NodeEvent::Message(d) => {
                let body = String::from_utf8_lossy(&d.body);
                if let Some(id) = body.strip_prefix("probe ").and_then(|s| s.parse().ok())
                    && w.probe_delivered(id, d.hops)
                {
                    w.log(
                        &me,
                        format!(
                            "probe {id} from {} hops {}",
                            w.label(w.ix_of(&d.src)),
                            d.hops
                        ),
                    );
                }
            }
            NodeEvent::ExchangeClosed {
                host_id, outcome, ..
            } => {
                w.exchange_closed(h, w.ix_of(&host_id), &outcome);
                w.log(
                    &me,
                    format!("exchange {} {outcome:?}", w.label(w.ix_of(&host_id))),
                );
            }
            NodeEvent::TransferError { host_id, error } => {
                w.log(
                    &me,
                    format!("transfer error {}: {error}", w.label(w.ix_of(&host_id))),
                );
            }
            NodeEvent::NetworkPeerAdded { network, .. }
            | NodeEvent::NetworkPeerRemoved { network, .. } => {
                let added = matches!(e, NodeEvent::NetworkPeerAdded { .. });
                if let Some(n) = nets.iter().position(|x| *x == Some(network)) {
                    w.network_peer(h, n, added);
                }
            }
        }
    }
}

/// Dials this host's links that are down, now and every
/// `Scenario::redial_every`, its links to its swarms' hosts from its join
/// on where it has one ([`HostSpec::links_at`](crate::HostSpec), [`join_links`]).
/// It is a run's only reconnect: links heal after partitions and restarts
/// through it. A host that departed dials nothing, and no host dials it.
async fn supervisor(w: Rc<World>, h: HostIx, node: Node) {
    let links: Vec<(usize, LinkSpec)> = w
        .spec
        .links
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, l)| l.from == h)
        .collect();
    if links.is_empty() {
        return;
    }
    let at = w
        .spec
        .hosts
        .get(h)
        .and_then(|s| Some(s.links_at?.max(s.arrive_at.unwrap_or_default())));
    let joined = Rc::new(Cell::new(at.is_none()));
    if let Some(at) = at {
        spawn_local(join_links(
            w.clone(),
            h,
            node.clone(),
            at,
            links.clone(),
            joined.clone(),
        ));
    }
    loop {
        if w.departed(h) {
            return;
        }
        let Ok(peers) = node.peers().await else {
            return;
        };
        for &(l, spec) in &links {
            if !joined.get() && w.spec.in_swarms(spec.to) {
                continue;
            }
            if w.departed(spec.to) {
                continue;
            }
            let Some(key) = w.profile_key(spec.to) else {
                continue;
            };
            let up = peers.iter().any(|p| p.profile_key == key.public);
            if !up && w.start_dial(h, l) {
                spawn_local(dial(w.clone(), h, l, spec, node.clone()));
            }
        }
        tokio::time::sleep(w.spec.redial_every).await;
    }
}

/// Leaves for good at `at` ([`Departure::Graceful`]), as a closed tab
/// does: the swarms' transfers stop (and with them their searches and
/// publications), the swarms go (their readers close), and every link
/// closes, so its peers see the links close. The event task closes any
/// link that comes up later.
async fn depart(
    w: Rc<World>,
    h: HostIx,
    node: Node,
    transfers: TransferControl<SimTransferHost, NodeTimer<SimClock>>,
    held: Vec<(TransferId, SwarmKey)>,
    at: Duration,
) {
    let now = sim_now();
    if at > now {
        tokio::time::sleep(at - now).await;
    }
    for (t, swarm) in held {
        transfers.remove(t);
        let _ = node.remove_swarm(swarm).await;
    }
    let Ok(peers) = node.peers().await else {
        return;
    };
    for p in &peers {
        let _ = node.close_peer(p.host_id).await;
    }
    w.log(w.name(h), format!("left, closing {} links", peers.len()));
}

/// Applies `Fault::Uplink` and `Fault::Connect` to the node.
async fn control(w: Rc<World>, h: HostIx, node: Node) {
    let me = w.name(h).to_owned();
    loop {
        if let Some(rate) = w.take_uplink(h)
            && node.set_rate_limit(rate).await.is_err()
        {
            return;
        }
        for (to, n) in w.take_connects(h) {
            let (Some(net), Some(id)) = (w.net(h, n), w.host_id(to)) else {
                w.log(&me, format!("connect {} skipped", w.name(to)));
                continue;
            };
            match node.connect_peer(net, id).await {
                Ok(started) => w.log(&me, format!("connect {} started {started}", w.name(to))),
                Err(e) => w.log(&me, format!("connect {} failed: {e}", w.name(to))),
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// The join of [`HostSpec::links_at`](crate::HostSpec): at `at`, a dial
/// of each of `links` to a swarm's host that is down, all at once in link
/// order, none awaited, as the testbed's harness makes a node's links at
/// its join; then the supervisor redials those still down.
async fn join_links(
    w: Rc<World>,
    h: HostIx,
    node: Node,
    at: Duration,
    links: Vec<(usize, LinkSpec)>,
    done: Rc<Cell<bool>>,
) {
    let now = sim_now();
    if at > now {
        tokio::time::sleep(at - now).await;
    }
    if w.departed(h) {
        return;
    }
    let Ok(peers) = node.peers().await else {
        return;
    };
    for (l, spec) in links {
        if !w.spec.in_swarms(spec.to) || w.departed(spec.to) {
            continue;
        }
        let Some(key) = w.profile_key(spec.to) else {
            continue;
        };
        if !peers.iter().any(|p| p.profile_key == key.public) && w.start_dial(h, l) {
            spawn_local(dial(w.clone(), h, l, spec, node.clone()));
        }
    }
    done.set(true);
}

async fn dial(w: Rc<World>, h: HostIx, l: usize, spec: LinkSpec, node: Node) {
    let linked = async {
        match spec.transport {
            Transport::WebRtc(n) => exchange(&w, h, spec.to, n, &node).await,
            t => {
                let uri = w.uri(spec.to, t).ok_or("no uri")?;
                node.dial(&uri).await.map(drop).map_err(|e| e.to_string())
            }
        }
    };
    let result = match tokio::time::timeout(DIAL_TIMEOUT, linked).await {
        Ok(r) => r,
        Err(_) => Err("timed out".to_owned()),
    };
    w.end_dial(h, l, result);
}

/// Links `h` to `to` by a peer exchange on network `n`, once both have a
/// route on it (an exchange sent before is lost, and waits out its
/// timeout): done once `to` is a peer, failed once the exchange ended
/// without it (an answerer at its limit of exchanges refuses).
async fn exchange(w: &World, h: HostIx, to: HostIx, n: usize, node: &Node) -> Result<(), String> {
    let (Some(net), Some(id), Some(key)) = (w.net(h, n), w.host_id(to), w.profile_key(to)) else {
        return Err("not on the network yet".to_owned());
    };
    if !(w.on_network(h, n) && w.on_network(to, n)) {
        return Err("not on the network yet".to_owned());
    }
    let ended = w.exchanges_with(h, to);
    node.connect_peer(net, id)
        .await
        .map_err(|e| e.to_string())?;
    loop {
        let peers = node.peers().await.map_err(|e| e.to_string())?;
        if peers.iter().any(|p| p.profile_key == key.public) {
            return Ok(());
        }
        if w.exchanges_with(h, to) != ended {
            return Err("the exchange ended unlinked".to_owned());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Writes `frames_per_write` frames every `write_interval` from `start` to
/// `stop`, each write as late as the host's timers fire (`late`,
/// [`crate::late`]), and with the host's timer log each write's wait for
/// its time, by the node's clock. A frame carries its write's due time,
/// the report's (`Report::written_at`), as the testbed node's frames do,
/// so a viewer's lag counts the writer's lateness. The write index comes
/// from the simulated time, so a restarted seed continues the numbering.
async fn writer_task(
    w: Rc<World>,
    h: HostIx,
    s: usize,
    mut writer: SwarmWriter,
    late: Option<crate::late::Lateness>,
) {
    let Some(spec) = w.spec.swarms.get(s).cloned() else {
        return;
    };
    let me = w.name(h).to_owned();
    let fpw = spec.frames_per_write as u64;
    let step = spec.write_interval.as_millis().max(1);
    let now = sim_now();
    let mut k = if now <= spec.start {
        0
    } else {
        u64::try_from((now - spec.start).as_millis().div_ceil(step)).unwrap_or(u64::MAX)
    };
    let mut buf = vec![0u8; spec.frames_per_write * FRAME_LEN];
    loop {
        let at = spec.start + spec.write_interval * u32::try_from(k).unwrap_or(u32::MAX);
        if at >= spec.stop {
            break;
        }
        let armed = sim_now();
        let late_by = late.as_ref().map_or(Duration::ZERO, |l| {
            l.draw(crate::late::Source::Writer, at <= armed)
                .to_std()
                .unwrap_or_default()
        });
        tokio::time::sleep((at + late_by).saturating_sub(armed)).await;
        if let Some(log) = w.timer_log(h) {
            let clock = |t: Duration| w.clock_at(h, t);
            log.push(TimerRecord {
                source: TimerSource::Writer,
                armed: clock(armed),
                deadline: clock(at),
                fired: clock(sim_now()),
                requested: clock(at) - clock(armed).truncate(Time::PRECISION),
            });
        }
        // The pass the write sets off, whose holds start late
        // (`Scenario::pacer_hold_start`).
        if let Some(starts) = w.hold_starts.get(h) {
            starts.frame_written(crate::wan::sim_time(at));
        }
        if w.take_writer_reset(s) {
            match writer.reset().await {
                Ok(()) => w.log(&me, format!("swarm {s} writer reset")),
                Err(e) => w.log(&me, format!("swarm {s} writer reset failed: {e}")),
            }
        }
        let t = u64::try_from(at.as_millis()).unwrap_or(u64::MAX);
        for (j, f) in buf.chunks_mut(FRAME_LEN).enumerate() {
            encode_frame(w.seed, s as u32, k * fpw + j as u64, t, f);
        }
        if let Err(e) = writer.write(&buf).await {
            w.log(&me, format!("swarm {s} write failed: {e}"));
            return;
        }
        w.mark_written(s, k * fpw..(k + 1) * fpw);
        if k % 40 == 0 {
            w.log(&me, format!("swarm {s} wrote {}", (k + 1) * fpw));
        }
        k += 1;
    }
    w.log(&me, format!("swarm {s} writer done"));
    // Keep the publisher: dropping the writer closes it.
    std::future::pending::<()>().await;
}

/// Writes media swarm `s`'s access units ([`World::media_traces`]) from
/// `start` to `stop`, each record as its access unit arrives, a
/// [`MEDIA_HEADER`] first in each chunk it opens, and signs a unit in
/// progress at its deadline ([`Signer`], `SwarmWriter::flush`). A
/// restarted seed, or a writer reset, starts the units over at the stream's
/// start with the next access unit; the chunks' sequence numbers go on.
async fn media_writer_task(
    w: Rc<World>,
    h: HostIx,
    s: usize,
    mut writer: SwarmWriter,
    m: MediaSpec,
) {
    let (Some(spec), Some(Some(trace))) = (w.spec.swarms.get(s).cloned(), w.media_traces.get(s))
    else {
        return;
    };
    let me = w.name(h).to_owned();
    let chunk = spec.options.chunk_size;
    let cps = spec.options.chunks_per_signature;
    let new_signer = || Signer::new(m.sign.deadline, cps, chunk, MEDIA_HEADER);
    w.media_log(s, |l| {
        l.unit_chunks = cps as u64;
        l.chunk = chunk as u64;
    });
    let mut st = MediaWriter {
        signer: new_signer(),
        chunk: chunk as u64,
        open: None,
        pending: Vec::new(),
    };
    let at = |d: strims_base::time::Duration| {
        spec.start + Duration::from_millis(u64::try_from(d.as_millis()).unwrap_or(0))
    };
    let now = sim_now();
    let mut buf = Vec::new();
    for au in trace.iter().filter(|a| at(a.at) >= now) {
        let when = at(au.at);
        if when >= spec.stop {
            break;
        }
        while let Some(due) = st.signer.due().map(at).filter(|&d| d <= when) {
            tokio::time::sleep(due.saturating_sub(sim_now())).await;
            if !st.flush(&w, s, &mut writer, &me).await {
                return;
            }
        }
        tokio::time::sleep(when.saturating_sub(sim_now())).await;
        if w.take_writer_reset(s) {
            match writer.reset().await {
                Ok(()) => w.log(&me, format!("swarm {s} writer reset")),
                Err(e) => w.log(&me, format!("swarm {s} writer reset failed: {e}")),
            }
            st = MediaWriter {
                signer: new_signer(),
                chunk: chunk as u64,
                open: None,
                pending: Vec::new(),
            };
        }
        let placed = st.signer.record(au);
        if au.key && au.video {
            w.media_log(s, |l| l.gops.push(placed.offset));
        }
        st.lay_out(&w, s, chunk, placed.offset, placed.len, &mut buf);
        if let Err(e) = writer.write(&buf).await {
            w.log(&me, format!("swarm {s} write failed: {e}"));
            return;
        }
        let unit = st.signer.unit_bytes();
        st.signed(
            &w,
            s,
            (placed.offset + placed.len) / unit * unit / chunk as u64,
        );
        w.media_log(s, |l| {
            l.media_bytes += u64::from(au.len);
            l.sent_bytes += placed.len;
            l.units += placed.signed;
        });
    }
    if let Some(due) = st.signer.due() {
        tokio::time::sleep(at(due).saturating_sub(sim_now())).await;
        st.flush(&w, s, &mut writer, &me).await;
    }
    w.log(&me, format!("swarm {s} writer done"));
    // Keep the publisher: dropping the writer closes it.
    std::future::pending::<()>().await;
}

/// A media writer's state: the units, the chunk in progress (its sequence
/// number and the stream offset where it starts), and the chunks not yet
/// signed (index, sequence number and first byte's time, ms).
struct MediaWriter {
    signer: Signer,
    chunk: u64,
    open: Option<(u64, u64)>,
    pending: Vec<(u64, u64, u64)>,
}

impl MediaWriter {
    /// The bytes of the record at `offset`, `len` long, into `buf`: a
    /// header at each chunk it opens, its chunks' payload elsewhere.
    fn lay_out(
        &mut self,
        w: &World,
        s: usize,
        chunk: usize,
        offset: u64,
        len: u64,
        buf: &mut Vec<u8>,
    ) {
        let size = chunk as u64;
        let now = sim_now_ms();
        buf.clear();
        let mut p = offset;
        let end = offset + len;
        while p < end {
            let at = usize::try_from(p % size).unwrap_or(0);
            if at == 0 {
                let seq = w.media_chunk(s, now);
                self.open = Some((seq, p));
                self.pending.push((p / size, seq, now));
                let h = media_header(seq, now);
                let n = h.len().min(usize::try_from(end - p).unwrap_or(0));
                buf.extend_from_slice(h.get(..n).unwrap_or_default());
                p += n as u64;
                continue;
            }
            let take = (size - p % size).min(end - p);
            let take = usize::try_from(take).unwrap_or(0);
            let from = buf.len();
            buf.resize(from + take, 0);
            let seq = self.open.map_or(0, |(seq, _)| seq);
            if let Some(out) = buf.get_mut(from..) {
                fill_media(w.seed, s as u32, seq, at, out);
            }
            p += take as u64;
        }
    }

    /// The chunks before index `through` were signed now: their waits, and
    /// their signing times by sequence number ([`MediaLog::signed`]).
    ///
    /// [`MediaLog::signed`]: crate::report::MediaLog::signed
    fn signed(&mut self, w: &World, s: usize, through: u64) {
        let now = sim_now_ms();
        let done: Vec<(u64, u32)> = self
            .pending
            .iter()
            .filter(|&&(c, _, _)| c < through)
            .map(|&(_, seq, t)| {
                let wait = u32::try_from(now.saturating_sub(t)).unwrap_or(u32::MAX);
                (seq, wait)
            })
            .collect();
        if !done.is_empty() {
            self.pending.retain(|&(c, _, _)| c >= through);
            w.media_log(s, |l| {
                for (seq, wait) in done {
                    l.sign_waits.push(wait);
                    let at = usize::try_from(seq).ok().and_then(|i| l.signed.get_mut(i));
                    if let Some(at) = at {
                        *at = Some(now);
                    }
                }
            });
        }
    }

    /// Signs the unit in progress through the writer; false once the
    /// writer fails or pads where the signer doesn't.
    async fn flush(&mut self, w: &World, s: usize, writer: &mut SwarmWriter, me: &str) -> bool {
        let pad = self.signer.flush();
        match writer.flush().await {
            Ok(next) if next == pad.end => {}
            Ok(next) => {
                w.log(me, format!("swarm {s} flush to {next}, not {}", pad.end));
                return false;
            }
            Err(e) => {
                w.log(me, format!("swarm {s} flush failed: {e}"));
                return false;
            }
        }
        let size = self.chunk.max(1);
        self.signed(w, s, pad.end / size);
        // the padding goes nowhere: DATA leaves out trailing zeros, and
        // the unit's chunks after the chunk in progress are data-less
        w.media_log(s, |l| l.units += 1);
        true
    }
}

/// Reads swarm `s` at viewer `h`. Where its first read lands is the
/// delivery mode's ([`swarm_options`]); `live` ([`joins_live`]) gives a
/// media swarm's join its rewind to its GOP's start ([`media_reader`]).
async fn reader_task(w: Rc<World>, h: HostIx, s: usize, node: Node, swarm: SwarmKey, live: bool) {
    let me = w.name(h).to_owned();
    let mut reader = match node.open_reader(swarm).await {
        Ok(r) => r,
        Err(e) => {
            w.log(&me, format!("swarm {s} open reader failed: {e}"));
            return;
        }
    };
    w.viewer_start(s, h);
    let mut buf = vec![0u8; 64 * 1024];
    if w.spec.swarms.get(s).is_some_and(|x| x.media.is_some()) {
        media_reader(&w, h, s, &mut reader, &mut buf, live).await;
        return;
    }
    let mut parser = FrameParser::default();
    loop {
        while w.reading_paused(h) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        match reader.read_at(&mut buf).await {
            Ok((offset, n)) => {
                w.viewer_read(s, h, offset, n);
                w.viewer_data(s, h, &mut parser, buf.get(..n).unwrap_or_default());
            }
            Err(ReadError::Underrun { skipped }) => {
                parser.resync();
                w.viewer_underrun(s, h, skipped);
            }
            Err(ReadError::Reset) => {
                parser.resync();
                w.viewer_reset(s, h);
            }
            Err(e) => {
                w.log(&me, format!("swarm {s} reader ended: {e}"));
                return;
            }
        }
    }
}

/// [`reader_task`]'s loop for a media swarm: reads at offsets, checked
/// chunk by chunk ([`MediaParser`]), and rewinds to its GOP's start after
/// its first read ([`SwarmSpec::join_rewind`], or [`ARRIVAL_REWIND`] for
/// a join at the live edge).
async fn media_reader(
    w: &World,
    h: HostIx,
    s: usize,
    reader: &mut SwarmReader,
    buf: &mut [u8],
    live: bool,
) {
    let me = w.name(h).to_owned();
    let mut parser = MediaParser::default();
    let mut rewind = w
        .spec
        .swarms
        .get(s)
        .and_then(|x| x.join_rewind.or(live.then_some(ARRIVAL_REWIND)));
    loop {
        while w.reading_paused(h) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        match reader.read_at(buf).await {
            Ok((offset, n)) => {
                w.viewer_read(s, h, offset, n);
                w.viewer_media(s, h, &mut parser, offset, buf.get(..n).unwrap_or_default());
                if let Some(deadline) = rewind.take() {
                    join_rewind(w, h, s, reader, offset, deadline).await;
                }
            }
            Err(ReadError::Underrun { skipped }) => {
                parser.resync();
                w.viewer_underrun(s, h, skipped);
            }
            Err(ReadError::Reset) => {
                parser.resync();
                w.viewer_reset(s, h);
            }
            Err(e) => {
                w.log(&me, format!("swarm {s} reader ended: {e}"));
                return;
            }
        }
    }
}

/// Moves `reader` back to the start of the GOP that stream offset `at`
/// falls in, within `deadline`, and notes how it went.
async fn join_rewind(
    w: &World,
    h: HostIx,
    s: usize,
    reader: &mut SwarmReader,
    at: u64,
    deadline: Duration,
) {
    let Some(to) = w.gop_start(s, at).filter(|&g| g < at) else {
        return;
    };
    let start = sim_now_ms();
    let landed = matches!(
        tokio::time::timeout(deadline, reader.rewind(to)).await,
        Ok(Ok(_))
    );
    let took = sim_now_ms().saturating_sub(start);
    let back = at - to;
    let how = if landed { "landed" } else { "dropped" };
    w.log(
        w.name(h),
        format!("swarm {s} join rewind of {back} bytes {how} after {took} ms"),
    );
    w.viewer_rewound(
        s,
        h,
        JoinRewind {
            start_ms: start,
            took_ms: took,
            landed,
            back,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::late::{HostKind, Lateness, Source, lateness};
    use strims_base::rng::det_rng;

    /// The probe's timers draw a harness's model, the writer's with the
    /// probe on, not the node task's.
    #[test]
    fn a_probes_timer_is_a_harness_timer() {
        for kind in [HostKind::Native, HostKind::Chromium, HostKind::Firefox] {
            let (l, mut fresh) = (Lateness::new(det_rng(4), kind, true), det_rng(4));
            for i in 0..1000 {
                assert_eq!(
                    probe_lateness(&l, false),
                    lateness(&mut fresh, kind, Source::Writer, true),
                    "{kind:?} draw {i}"
                );
            }
        }
    }
}
