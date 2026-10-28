// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The sans-IO multi-peer driver surface. The [`Runner`] owns the swarms,
//! the peers, a scheduler per swarm while it has channels, and a channel
//! (integrity verifier, message counters, transfer metrics) per peer and
//! swarm, all addressed by keys. It does no I/O and reads no clock: every
//! call that depends on time takes `now`.
//!
//! [`Runner`]'s docs give the contract a driver must honour.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;
use slotmap::SlotMap;
use strims_base::path::{PathReading, PathSample};
use strims_base::rng::{DetRng, fork};
use strims_base::stats::Sma;
use strims_base::time::{Duration, Time};
use strims_binmap::Bin;

use crate::answer_gate::{self, AnswerRate};
use crate::channel::{ChannelMetrics, HandshakeError, MeteredWriter, validate_handshake};
use crate::codec::{self, FrameReader, Message, Reader, ReaderConfig, Stream};
use crate::epoch::EpochError;
use crate::integrity::ChannelVerifier;
use crate::options::PushMode;
use crate::peer::{
    MetricsSnapshot, PeerMetricsSnapshot, PeerStats, Queues, RunnerPeer, TransferMetrics, Wake,
};
use crate::queue_delay;
use crate::scheduler::PeerPriority;
use crate::scheduler_peer::push::seed::SeedAllocationStats;
use crate::scheduler_peer::push::{PushStats, UploadInfo};
use crate::scheduler_peer::{
    ChannelSchedulerStats, PeerSwarmScheduler, SchedulerError, StreamOpenOutcome,
    SwarmSchedulerStats, WriteDataError,
};
use crate::swarm::{Swarm, SwarmError, SwarmEvent};
use crate::tap::{Tap, TapEvent, TapKind, TapLevel};
use crate::writer::Writer;

slotmap::new_key_type! {
    /// A swarm in a [`Runner`].
    pub struct SwarmKey;
    /// A peer in a [`Runner`].
    pub struct PeerKey;
}

/// The runner's clock: scheduler runs and deferred writes happen on this
/// tick.
pub const TICK_INTERVAL: Duration = Duration::from_millis(100);
/// How often the peers' upload weights are recomputed.
pub const PEER_QOS_UPDATE_INTERVAL: Duration = Duration::SECOND;
/// The upload weight of a peer that sends us nothing useful.
pub const MIN_PEER_QOS_WEIGHT: u64 = 50;
/// The upload weight of the peer that sends us the most useful data.
pub const MAX_PEER_QOS_WEIGHT: u64 = 1000;

/// Runner errors. An error from [`Runner::handle_frame`] means the peer
/// broke the protocol; see the [driver contract](Runner#driver-contract).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RunnerError {
    #[error("unknown peer")]
    UnknownPeer,
    #[error("unknown swarm")]
    UnknownSwarm,
    /// The peer already runs a channel for this swarm.
    #[error("channel for swarm/peer pair already running")]
    ChannelRunning,
    /// The peer already uses this channel id for another swarm.
    #[error("channel id in use")]
    ChannelInUse,
    /// The peer's handshake hasn't arrived on the channel, so it can't
    /// request streams yet (`ChannelSchedulerStats::handshake_received`).
    #[error("peer handshake pending")]
    HandshakePending,
    /// A decode error in a frame.
    #[error(transparent)]
    Codec(#[from] codec::Error),
    #[error(transparent)]
    Handshake(#[from] HandshakeError),
    /// An epoch whose signature doesn't verify.
    #[error(transparent)]
    Epoch(#[from] EpochError),
    /// A stream index out of range.
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),
    #[error(transparent)]
    Swarm(#[from] SwarmError),
}

/// Notifications for the driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// The swarm's local readers may have data, an error or a new offset.
    Readable(SwarmKey),
    /// The peer started a write cycle: poll its frames (set the vnic
    /// writer's backlog).
    Transmit(PeerKey),
    /// The peer's upload weight changed: set its writer's weight.
    QosWeight { peer: PeerKey, weight: u64 },
}

/// The channel layer for one peer and swarm.
#[derive(Debug)]
struct Channel {
    /// Our channel id: the peer's frames for this swarm carry it.
    id: u64,
    /// The peer's channel id: our frames carry it.
    peer_channel: u64,
    reader_config: ReaderConfig,
    verifier: ChannelVerifier,
    metrics_in: ChannelMetrics,
    metrics_out: ChannelMetrics,
    read: TransferMetrics,
    write: TransferMetrics,
}

/// A swarm and the state that exists while it has channels.
#[derive(Debug)]
struct RunnerSwarm {
    swarm: Swarm,
    /// Exists while the swarm has channels.
    scheduler: Option<PeerSwarmScheduler>,
    channels: IndexMap<PeerKey, Channel>,
    /// A hybrid seed's pinned allocation, given to each new scheduler.
    seed_pin: Option<Vec<Vec<PeerKey>>>,
    /// The push rate its subscriptions committed at the last tick.
    committed: f64,
}

#[derive(Debug, Default)]
struct Events {
    /// Bound: one per swarm.
    readable: BTreeSet<SwarmKey>,
    /// Bound: one per peer.
    transmit: BTreeSet<PeerKey>,
    /// Bound: one per peer.
    weights: BTreeMap<PeerKey, u64>,
}

/// The node's upload, for push admission control (runs while any swarm is
/// in [`PushMode::Hybrid`]): bytes written, and the time during which data
/// was queued to send. Bytes over busy time is the capacity, provided the
/// driver drains queued data as fast as the link allows. A driver that
/// queues data further on (a rate limit's burst, kernel socket buffers)
/// hides that time, and the reading runs high; one that holds a peer's data
/// back for that peer alone makes it read low. The driver's hint, when it
/// has one, bounds it. The queueing delay below the runner, from the
/// driver's measure of the peers' links ([`queue_delay`](crate::queue_delay)),
/// shows the time those further queues hold data.
#[derive(Debug)]
struct UploadMeter {
    busy_since: Option<Time>,
    /// Busy nanoseconds; 2 s in 100 ms intervals.
    busy: Sma,
    /// Bytes written; the same window.
    bytes: Sma,
    /// The capacity the driver knows ([`Runner::set_upload_capacity`]).
    hint: Option<u64>,
    /// The queueing delay at the last tick.
    queue_delay: Option<Duration>,
    /// Scratch for the peers' excesses. Bound: one per peer.
    excesses: Vec<Duration>,
}

impl UploadMeter {
    fn new() -> UploadMeter {
        UploadMeter {
            busy_since: None,
            busy: Sma::new(20, Duration::from_millis(100), Time::EPOCH),
            bytes: Sma::new(20, Duration::from_millis(100), Time::EPOCH),
            hint: None,
            queue_delay: None,
            excesses: Vec::new(),
        }
    }

    /// Takes the node's queueing delay from the peers' links.
    fn update_queue_delay(&mut self, peers: &SlotMap<PeerKey, RunnerPeer>) {
        self.excesses.clear();
        self.excesses
            .extend(peers.values().filter_map(|p| p.link_path?.excess()));
        self.queue_delay = queue_delay::aggregate(&mut self.excesses);
    }

    /// Accounts the time since the last call as busy if data was queued
    /// then, and records whether data is queued now.
    fn observe(&mut self, now: Time, backlog: bool) {
        if let Some(t) = self.busy_since {
            let d = (now - t).as_nanos();
            if d > 0 {
                self.busy.add(d as u64, now);
            }
        }
        self.busy_since = backlog.then_some(now);
    }

    fn info(&mut self, now: Time) -> UploadInfo {
        let busy = self.busy.rate(Duration::SECOND, now) as f64 / 1e9;
        UploadInfo {
            rate: self.bytes.rate(Duration::SECOND, now),
            busy: busy.clamp(0.0, 1.0),
            hint: self.hint,
            others: 0.0,
            queue_delay: self.queue_delay,
        }
    }
}

/// Channel state, for diagnostics and tests.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelStats {
    pub scheduler: ChannelSchedulerStats,
    pub inbound: ChannelMetrics,
    pub outbound: ChannelMetrics,
}

/// The ppspp runner: one per node, driven by frames in and out, a clock
/// and events.
///
/// # Driver contract
///
/// The driver owns the transport (one writer per peer, with a backlog flag
/// and a weight for its share of the uplink) and must:
///
/// - **Peers.** Call [`Runner::add_peer`] when a transport peer appears
///   and [`Runner::remove_peer`] when it goes. `add_peer` queues the peer's
///   initial [`Event::QosWeight`]; drain events before the peer's writer
///   carries data, or it runs at the transport's default weight until the
///   next update.
/// - **Channels.** Call [`Runner::run_swarm`] with the channel ids agreed
///   with the peer out of band (the peer's frames carry `channel`, ours
///   carry `peer_channel`) and [`Runner::stop_swarm`] when it closes. A
///   peer runs at most one channel per swarm, and a channel id names one
///   swarm per peer.
/// - **Ingress.** Pass every frame body from the peer to
///   [`Runner::handle_frame`]. Payloads for unknown channels are skipped.
///   An error means the peer broke the protocol: the messages before it
///   were handled and the rest of the frame was dropped. Stop that peer's
///   channels (each of [`Runner::peer_swarms`]) or remove it; the runner
///   keeps them running otherwise.
/// - **Egress.** On [`Event::Transmit`], mark the peer's writer as having
///   a backlog. For each frame the transport can send, call
///   [`Runner::poll_transmit`] with the frame's body buffer and keep the
///   backlog while [`Runner::wants_transmit`]. A return of 0 means nothing
///   to send; the next `Transmit` sets the backlog again. The body must fit
///   one chunk plus its integrity messages (about 1.5 KiB for 1 KiB
///   chunks): a chunk that can't fit an empty frame is dropped and must be
///   fetched elsewhere.
/// - **Timers.** Call [`Runner::handle_timeout`] at
///   [`Runner::poll_timeout`]. The runner ticks every [`TICK_INTERVAL`]
///   while any channel runs (request timeouts, restarts, garbage
///   collection, the first read offset, deferred write cycles) and
///   updates upload weights every
///   [`PEER_QOS_UPDATE_INTERVAL`]. In [`PushMode::Hybrid`] it can also
///   return an earlier push deadline. `None` means no channel runs.
///   A hybrid seed under [`SeedQos::ByAllocation`](crate::SeedQos::ByAllocation)
///   also updates them in the call that changed its subscribers (or the
///   next one, after a pin).
/// - **Events.** Drain [`Runner::poll_event`] after every call that takes
///   input or time (`add_peer`, `run_swarm`, `handle_frame`,
///   `handle_timeout`, `write` and the [`SwarmMut`] guard,
///   `request_stream`, `cancel_stream`): [`Event::Transmit`] sets the
///   peer's backlog, [`Event::QosWeight`] sets its writer's weight
///   ([`MIN_PEER_QOS_WEIGHT`]..=[`MAX_PEER_QOS_WEIGHT`]), and
///   [`Event::Readable`] wakes the swarm's local readers.
/// - **Publishing.** Write through [`Runner::write`] (or
///   [`Runner::swarm_mut`]); new chunks reach the scheduler before the call
///   returns, and their HAVEs go out at the next tick (at once in the eager
///   push modes).
/// - **Reading.** On [`Event::Readable`], read through [`Runner::swarm`]
///   until [`store::Error::WouldBlock`](crate::store::Error::WouldBlock);
///   recover an underrun reader through [`Runner::swarm_mut`]. A reader's
///   offset is unknown until the scheduler sets the first read position.
/// - **Paths.** Measure each peer's link as [`strims_base::path`] does
///   (vnic's probes), pass every round trip to [`Runner::on_path_sample`]
///   as it comes, and the reading to [`Runner::set_peer_path`] every pass.
/// - **Clock.** Pass wall-clock time with millisecond precision or better.
///   Peers' clocks may differ: the congestion controllers use only
///   differences of one-way delays.
///
/// Upload weights reward reciprocity: each peer's weight is its rate of
/// verified, new data sent to us, normalized to the top peer's rate and
/// clamped to the weight range. With no useful data in the window every
/// peer gets the maximum.
#[derive(Debug)]
pub struct Runner {
    swarms: SlotMap<SwarmKey, RunnerSwarm>,
    peers: SlotMap<PeerKey, RunnerPeer>,
    rng: DetRng,
    next_tick: Time,
    next_weight_update: Time,
    events: Events,
    /// Scratch for `handle_frame`. Bound: one per channel of the peer.
    touched: Vec<SwarmKey>,
    /// The latest time a call passed (for calls that take none).
    last_now: Time,
    /// Swarms in [`PushMode::Hybrid`]; the upload meter runs while any.
    hybrid_swarms: usize,
    upload: UploadMeter,
    /// The push rate every swarm's subscriptions committed at the last tick.
    committed: f64,
    /// Lifecycle events; the schedulers keep their own.
    tap: Tap,
    tap_level: TapLevel,
    /// Peers whose hybrid answers the answer gate held back at their last
    /// poll ([`crate::answer_gate`]). Bound: one per peer.
    held: BTreeSet<PeerKey>,
    /// The pace of hybrid answers while a queue stands below the runner.
    answer_rate: AnswerRate,
    /// The rate and reading last tapped, so a tap marks a change.
    tapped_rate: Option<(u64, Option<Duration>)>,
}

/// Hands a swarm's queued events to its scheduler and the driver. Called
/// after every call that can publish or reset.
fn drain_swarm(
    swarm: &mut Swarm,
    mut scheduler: Option<&mut PeerSwarmScheduler>,
    key: SwarmKey,
    now: Time,
    q: &mut Queues<'_>,
    readable: &mut BTreeSet<SwarmKey>,
) {
    while let Some(e) = swarm.poll_event() {
        match e {
            SwarmEvent::Reset => {
                if let Some(s) = scheduler.as_deref_mut() {
                    s.reset(now, q);
                }
            }
            SwarmEvent::Published(b) => {
                if let Some(s) = scheduler.as_deref_mut() {
                    s.consume(b, now, swarm, q);
                }
            }
            SwarmEvent::Readable => {
                readable.insert(key);
            }
        }
    }
}

/// Handles one channel payload from `peer`, message by message, stopping
/// at the first error.
fn handle_payload(
    rs: &mut RunnerSwarm,
    key: SwarmKey,
    peer: PeerKey,
    now: Time,
    payload: &[u8],
    q: &mut Queues<'_>,
    readable: &mut BTreeSet<SwarmKey>,
) -> Result<(), RunnerError> {
    let RunnerSwarm {
        swarm,
        scheduler,
        channels,
        ..
    } = rs;
    let (Some(ch), Some(sched)) = (channels.get_mut(&peer), scheduler.as_mut()) else {
        return Ok(());
    };

    for m in Reader::new(ch.reader_config, payload) {
        let m = m?;
        ch.metrics_in.count(&m);
        match m {
            Message::Handshake(h) => {
                let o = validate_handshake(swarm, &h.options)?;
                let Some(e) = o.epoch else {
                    continue;
                };
                match swarm.sync_epoch(e.timestamp, e.signature) {
                    Ok(true) => {
                        // A newer epoch: the stream restarted. The store,
                        // verifier and scheduler start over.
                        swarm.reset();
                        drain_swarm(swarm, Some(sched), key, now, q, readable);
                    }
                    Ok(false) => {}
                    Err(EpochError::OutOfDate) => continue,
                    Err(e) => return Err(e.into()),
                }
                sched.handle_handshake(peer, o.live_window.unwrap_or(0));
            }
            Message::Restart => sched.handle_restart(peer),
            Message::Data(d) => {
                let n = d.data.len() as u64;
                ch.read.add(n, now);
                if let Some(p) = q.peers.get_mut(peer) {
                    p.read.add(n, now);
                }
                if !sched.expect_data(peer, d.bin) {
                    continue;
                }
                let window_start = swarm.store().window_start();
                // Behind the window the chunk can't be stored, and behind the
                // verifier's ring its munro's signature won't come again.
                if d.bin.base_right() < window_start.max(swarm.verifier().verifiable_from()) {
                    let m = &mut ch.metrics_in;
                    m.stale_data = m.stale_data.saturating_add(1);
                    sched.handle_stale_data(peer, d.bin, d.timestamp, now);
                    continue;
                }
                match swarm.handle_data(&mut ch.verifier, d.bin, d.data) {
                    Ok(published) => {
                        if published.new
                            && let Some(p) = q.peers.get_mut(peer)
                        {
                            p.useful.add(n, now);
                        }
                        drain_swarm(swarm, Some(sched), key, now, q, readable);
                        sched.handle_data(
                            peer,
                            d.bin,
                            d.timestamp,
                            true,
                            published.new,
                            now,
                            window_start,
                            q,
                        );
                    }
                    Err(_) => {
                        let m = &mut ch.metrics_in;
                        m.invalid_data = m.invalid_data.saturating_add(1);
                        m.invalid_chunk = m.invalid_chunk.saturating_add(d.bin.base_length());
                        m.invalid_bytes = m.invalid_bytes.saturating_add(n);
                        sched.handle_data(
                            peer,
                            d.bin,
                            d.timestamp,
                            false,
                            false,
                            now,
                            window_start,
                            q,
                        );
                    }
                }
            }
            Message::Integrity(i) => swarm.handle_integrity(&mut ch.verifier, i.bin, i.hash),
            Message::SignedIntegrity(i) => {
                swarm.handle_signed_integrity(&mut ch.verifier, i.bin, i.timestamp, i.signature)
            }
            // Ignored: a request counts as answered when its data arrives.
            Message::Ack(_) => {}
            Message::Have(h) => {
                if !sched.handle_have(peer, h.bin, now) {
                    ch.metrics_in.rejected = ch.metrics_in.rejected.saturating_add(1);
                }
            }
            Message::Request(r) => {
                if !sched.handle_request(peer, r.bin, now, q) {
                    ch.metrics_in.rejected = ch.metrics_in.rejected.saturating_add(1);
                }
            }
            Message::Cancel(c) => sched.handle_cancel(peer, c.bin, swarm.store().window_start(), q),
            Message::Choke => sched.handle_choke(peer, true),
            Message::Unchoke => sched.handle_choke(peer, false),
            Message::Ping(p) => sched.handle_ping(peer, p.nonce),
            Message::Pong(p) => sched.handle_pong(peer, p.nonce, now),
            Message::StreamRequest(s) => {
                let tail = swarm.store().tail();
                if !sched.handle_stream_request(peer, s.stream, s.bin, tail, now, q)? {
                    ch.metrics_in.rejected = ch.metrics_in.rejected.saturating_add(1);
                }
            }
            Message::StreamCancel(s) => sched.handle_stream_cancel(peer, s.stream)?,
            Message::StreamOpen(s) => {
                match sched.handle_stream_open(peer, s.stream, s.bin, now, q)? {
                    StreamOpenOutcome::Accepted | StreamOpenOutcome::BeforeHandshake => {}
                    StreamOpenOutcome::Stale => {
                        let m = &mut ch.metrics_in;
                        m.stale_stream_open = m.stale_stream_open.saturating_add(1);
                    }
                    StreamOpenOutcome::Rejected => {
                        ch.metrics_in.rejected = ch.metrics_in.rejected.saturating_add(1);
                    }
                }
            }
            Message::StreamClose(s) => {
                let window_start = swarm.store().window_start();
                sched.handle_stream_close(peer, s.stream, window_start, now, q)?
            }
            Message::Announce(a) => {
                if !sched.handle_announce(peer, a, now) {
                    ch.metrics_in.rejected = ch.metrics_in.rejected.saturating_add(1);
                }
            }
            // The reader stops at End.
            Message::End => {}
        }
    }
    Ok(())
}

impl Runner {
    /// A runner without swarms or peers. `rng` seeds the schedulers'
    /// request orders and timer phases.
    pub fn new(rng: DetRng) -> Runner {
        Runner {
            swarms: SlotMap::with_key(),
            peers: SlotMap::with_key(),
            rng,
            next_tick: Time::NIL,
            next_weight_update: Time::NIL,
            events: Events::default(),
            touched: Vec::new(),
            last_now: Time::NIL,
            hybrid_swarms: 0,
            upload: UploadMeter::new(),
            committed: 0.0,
            tap: Tap::default(),
            tap_level: TapLevel::Off,
            held: BTreeSet::new(),
            answer_rate: AnswerRate::new(),
            tapped_rate: None,
        }
    }

    /// The node's upload capacity in bytes per second, when its driver
    /// knows it (an upload limit it was configured with); `None` leaves
    /// the measurement alone. Push admission, shedding and seed allocation
    /// take it as the capacity, or the measurement where that shows a
    /// saturated link below it. It comes from the node's own
    /// configuration, never from a peer.
    pub fn set_upload_capacity(&mut self, bytes_per_sec: Option<u64>) {
        self.upload.hint = bytes_per_sec.filter(|&b| b > 0);
    }

    // --- the link's path --------------------------------------------------

    /// The driver's measure of the link to `peer` as of its latest echo
    /// ([`strims_base::path`]: vnic's probes, or a simulation's model of
    /// them), or `None` before one. The driver sets it every pass, so it is
    /// never older than the pass. The upload meter's queueing delay reads
    /// it.
    pub fn set_peer_path(&mut self, peer: PeerKey, reading: Option<PathReading>) {
        if let Some(p) = self.peers.get_mut(peer) {
            p.link_path = reading;
        }
    }

    /// A round trip of the link to `peer`, from the driver's path measure:
    /// one call per echo. Every channel to `peer` takes it, for an adaptive
    /// seed's holder ranking.
    pub fn on_path_sample(&mut self, peer: PeerKey, sample: PathSample) {
        let Some(p) = self.peers.get(peer) else {
            return;
        };
        for &key in p.channels.values() {
            if let Some(sched) = self
                .swarms
                .get_mut(key)
                .and_then(|rs| rs.scheduler.as_mut())
            {
                sched.handle_path_sample(peer, sample.rtt, sample.at);
            }
        }
    }

    // --- tap ------------------------------------------------------------

    /// Sets what the tap reports (it starts off); see [`TapEvent`]. A
    /// decision the tap doesn't report costs a flag check, and a tap that
    /// reports nothing holds no memory.
    pub fn set_tap(&mut self, level: TapLevel) {
        self.tap_level = level;
        self.tap.set(level != TapLevel::Off);
        for rs in self.swarms.values_mut() {
            if let Some(s) = rs.scheduler.as_mut() {
                s.tap_mut().set(level == TapLevel::All);
            }
        }
    }

    /// Hands the tap's events to `f`: the runner's, then each swarm's, then
    /// a [`TapKind::Lost`] if any were dropped. Drain after every call
    /// whose events must stay apart from the next call's: each buffer holds
    /// [`TAP_CAP`](crate::TAP_CAP) events.
    pub fn drain_tap(&mut self, mut f: impl FnMut(TapEvent)) {
        let mut lost = self.tap.drain(&mut f);
        for rs in self.swarms.values_mut() {
            if let Some(s) = rs.scheduler.as_mut() {
                lost = lost.saturating_add(s.tap_mut().drain(&mut f));
            }
        }
        if lost > 0 {
            f(TapEvent {
                swarm: None,
                peer: None,
                kind: TapKind::Lost { count: lost },
            });
        }
    }

    /// The swarms.
    pub fn swarms(&self) -> impl Iterator<Item = (SwarmKey, &Swarm)> + '_ {
        self.swarms.iter().map(|(k, rs)| (k, &rs.swarm))
    }

    /// The peers and their ids.
    pub fn peers(&self) -> impl Iterator<Item = (PeerKey, &[u8])> + '_ {
        self.peers.iter().map(|(k, p)| (k, p.id.as_slice()))
    }

    /// The running channels: peer, swarm, our channel id and the peer's.
    pub fn channels(&self) -> impl Iterator<Item = (PeerKey, SwarmKey, u64, u64)> + '_ {
        self.swarms.iter().flat_map(|(key, rs)| {
            rs.channels
                .iter()
                .map(move |(&peer, ch)| (peer, key, ch.id, ch.peer_channel))
        })
    }

    /// Starts the clock on the first call that knows the time.
    fn touch_clock(&mut self, now: Time) {
        self.last_now = now;
        if self.next_tick.is_nil() {
            self.next_tick = now + TICK_INTERVAL;
            self.next_weight_update = now + PEER_QOS_UPDATE_INTERVAL;
        }
    }

    /// Feeds the upload meter whether any data is queued that the node
    /// would send now (hybrid only). Answers the push rule holds count:
    /// it holds them only while pushes are queued. Answers the pace holds
    /// count only while it has tokens, so answers held back don't read as
    /// a busy uplink.
    fn observe_upload(&mut self, now: Time) {
        if self.hybrid_swarms == 0 {
            return;
        }
        let paced = self.answer_rate.open(now);
        let backlog = self.peers.values().any(|p| {
            p.dq.iter()
                .zip(PeerPriority::ALL)
                .any(|(q, pri)| !q.is_empty() && (paced || pri != PeerPriority::Scavenger))
        });
        self.upload.observe(now, backlog);
    }

    // --- swarms ---------------------------------------------------------

    /// Adds a swarm, e.g. one returned by [`Writer::new`]. Its scheduler
    /// starts with its first channel.
    pub fn add_swarm(&mut self, swarm: Swarm) -> SwarmKey {
        if swarm.options().push_mode == PushMode::Hybrid {
            self.hybrid_swarms += 1;
        }
        let key = self.swarms.insert(RunnerSwarm {
            swarm,
            scheduler: None,
            channels: IndexMap::new(),
            seed_pin: None,
            committed: 0.0,
        });
        self.tap.push(TapEvent {
            swarm: Some(key),
            peer: None,
            kind: TapKind::SwarmAdded,
        });
        key
    }

    /// Stops the swarm's channels and returns it.
    pub fn remove_swarm(&mut self, key: SwarmKey) -> Option<Swarm> {
        let peers: Vec<PeerKey> = self.swarms.get(key)?.channels.keys().copied().collect();
        for peer in peers {
            self.stop_channel(key, peer);
        }
        self.events.readable.remove(&key);
        let swarm = self.swarms.remove(key).map(|rs| rs.swarm)?;
        self.tap.push(TapEvent {
            swarm: Some(key),
            peer: None,
            kind: TapKind::SwarmRemoved,
        });
        if swarm.options().push_mode == PushMode::Hybrid {
            self.hybrid_swarms = self.hybrid_swarms.saturating_sub(1);
        }
        Some(swarm)
    }

    /// The swarm, for reading.
    pub fn swarm(&self, key: SwarmKey) -> Option<&Swarm> {
        self.swarms.get(key).map(|rs| &rs.swarm)
    }

    /// The swarm, for publishing, resetting or recovering readers. Chunks
    /// published through the guard reach the scheduler when it drops.
    pub fn swarm_mut(&mut self, key: SwarmKey, now: Time) -> Option<SwarmMut<'_>> {
        if !self.swarms.contains_key(key) {
            return None;
        }
        Some(SwarmMut {
            runner: self,
            key,
            now,
        })
    }

    /// Hands the swarm's queued events to its scheduler.
    fn drain(&mut self, key: SwarmKey, now: Time) {
        let Runner {
            swarms,
            peers,
            events,
            ..
        } = self;
        let Some(rs) = swarms.get_mut(key) else {
            return;
        };
        let mut q = Queues {
            peers,
            transmit: &mut events.transmit,
        };
        drain_swarm(
            &mut rs.swarm,
            rs.scheduler.as_mut(),
            key,
            now,
            &mut q,
            &mut events.readable,
        );
        self.observe_upload(now);
        self.refresh_seed_weights(now);
    }

    /// Publishes `data` through `writer` into the swarm; new chunks reach
    /// the scheduler before it returns.
    pub fn write(
        &mut self,
        key: SwarmKey,
        writer: &mut Writer,
        now: Time,
        data: &[u8],
    ) -> Result<usize, RunnerError> {
        let mut s = self.swarm_mut(key, now).ok_or(RunnerError::UnknownSwarm)?;
        Ok(writer.write(&mut s, now, data)?)
    }

    /// [`Writer::flush`] through the runner.
    pub fn flush_writer(
        &mut self,
        key: SwarmKey,
        writer: &mut Writer,
        now: Time,
    ) -> Result<(), RunnerError> {
        let mut s = self.swarm_mut(key, now).ok_or(RunnerError::UnknownSwarm)?;
        Ok(writer.flush(&mut s, now)?)
    }

    /// [`Writer::reset`] through the runner.
    pub fn reset_writer(
        &mut self,
        key: SwarmKey,
        writer: &mut Writer,
        now: Time,
    ) -> Result<(), RunnerError> {
        let mut s = self.swarm_mut(key, now).ok_or(RunnerError::UnknownSwarm)?;
        Ok(writer.reset(&mut s, now)?)
    }

    // --- peers ----------------------------------------------------------

    /// Adds a peer; `id` names it in the reports a swarm's seed asks for,
    /// so it must name the same peer at every node, in at most
    /// [`MAX_PEER_ID`](crate::codec::MAX_PEER_ID) bytes. The peer gets an
    /// upload weight at once (an [`Event::QosWeight`]): the median of the
    /// current peers' weights, or the maximum if it is the only peer, so
    /// it never waits for the first update at the transport's default.
    pub fn add_peer(&mut self, id: &[u8], now: Time) -> PeerKey {
        self.touch_clock(now);
        let weight = self.initial_weight();
        let mut p = RunnerPeer::new(id, now);
        p.qos_weight = Some(weight);
        let key = self.peers.insert(p);
        self.events.weights.insert(key, weight);
        self.tap.push(TapEvent {
            swarm: None,
            peer: Some(key),
            kind: TapKind::PeerAdded,
        });
        key
    }

    /// A new peer's weight: the median of the current weights, so a
    /// newcomer, which can't have reciprocated yet, gets a middle share
    /// until the next update; the maximum without other peers.
    fn initial_weight(&self) -> u64 {
        let mut w: Vec<u64> = self.peers.values().filter_map(|p| p.qos_weight).collect();
        if w.is_empty() {
            return MAX_PEER_QOS_WEIGHT;
        }
        w.sort_unstable();
        w.get((w.len() - 1) / 2)
            .copied()
            .unwrap_or(MAX_PEER_QOS_WEIGHT)
    }

    /// Stops the peer's channels and forgets it.
    pub fn remove_peer(&mut self, peer: PeerKey) {
        let Some(p) = self.peers.get(peer) else {
            return;
        };
        let swarms: Vec<SwarmKey> = p.channels.values().copied().collect();
        for key in swarms {
            self.stop_channel(key, peer);
        }
        self.peers.remove(peer);
        self.held.remove(&peer);
        self.events.transmit.remove(&peer);
        self.events.weights.remove(&peer);
        self.tap.push(TapEvent {
            swarm: None,
            peer: Some(peer),
            kind: TapKind::PeerRemoved,
        });
    }

    /// The id given to [`Runner::add_peer`].
    pub fn peer_id(&self, peer: PeerKey) -> Option<&[u8]> {
        self.peers.get(peer).map(|p| p.id.as_slice())
    }

    /// The swarms running a channel with `peer`, in the order of our
    /// channel ids (empty for an unknown peer).
    pub fn peer_swarms(&self, peer: PeerKey) -> impl Iterator<Item = SwarmKey> + '_ {
        self.peers
            .get(peer)
            .into_iter()
            .flat_map(|p| p.channels.values().copied())
    }

    /// Runs `swarm` with `peer`: frames from the peer carry `channel`, ours
    /// carry `peer_channel`.
    pub fn run_swarm(
        &mut self,
        peer: PeerKey,
        swarm: SwarmKey,
        channel: u64,
        peer_channel: u64,
        now: Time,
    ) -> Result<(), RunnerError> {
        self.touch_clock(now);
        let Runner {
            swarms,
            peers,
            rng,
            events,
            next_tick,
            tap,
            tap_level,
            ..
        } = self;
        let p = peers.get_mut(peer).ok_or(RunnerError::UnknownPeer)?;
        let rs = swarms.get_mut(swarm).ok_or(RunnerError::UnknownSwarm)?;
        if rs.channels.contains_key(&peer) {
            return Err(RunnerError::ChannelRunning);
        }
        if p.channels.contains_key(&channel) {
            return Err(RunnerError::ChannelInUse);
        }
        p.channels.insert(channel, swarm);
        let id = p.id.clone();
        // The clock doesn't tick while no channel runs; the first tick after
        // an idle spell is a full interval away, not in the past.
        if *next_tick < now {
            *next_tick = now + TICK_INTERVAL;
        }

        let sched = rs
            .scheduler
            .get_or_insert_with(|| PeerSwarmScheduler::new(&rs.swarm, swarm, &mut fork(rng), now));
        sched.tap_mut().set(*tap_level == TapLevel::All);
        tap.push(TapEvent {
            swarm: Some(swarm),
            peer: Some(peer),
            kind: TapKind::ChannelOpened {
                channel,
                peer_channel,
            },
        });
        rs.channels.insert(
            peer,
            Channel {
                id: channel,
                peer_channel,
                reader_config: rs.swarm.reader_config(),
                verifier: rs.swarm.channel_verifier(),
                metrics_in: ChannelMetrics::default(),
                metrics_out: ChannelMetrics::default(),
                read: TransferMetrics::new(now),
                write: TransferMetrics::new(now),
            },
        );
        let mut q = Queues {
            peers,
            transmit: &mut events.transmit,
        };
        if rs.channels.len() == 1 && rs.seed_pin.is_some() {
            sched.pin_allocation(rs.seed_pin.clone(), &mut q);
        }
        sched.add_channel(peer, &id, now, &mut q);
        // The link's latest round trip, for an adaptive seed's holder
        // ranking: the probe at the link's start was echoed before any
        // swarm ran.
        if let Some(r) = q.peers.get(peer).and_then(|p| p.link_path) {
            sched.handle_path_sample(peer, r.rtt, now);
        }
        Ok(())
    }

    /// Stops `swarm` with `peer`.
    pub fn stop_swarm(&mut self, peer: PeerKey, swarm: SwarmKey) {
        self.stop_channel(swarm, peer);
    }

    /// Stops a channel; the scheduler goes with the swarm's last channel.
    fn stop_channel(&mut self, key: SwarmKey, peer: PeerKey) {
        let Runner {
            swarms,
            peers,
            events,
            tap,
            ..
        } = self;
        let Some(rs) = swarms.get_mut(key) else {
            return;
        };
        let Some(ch) = rs.channels.shift_remove(&peer) else {
            return;
        };
        tap.push(TapEvent {
            swarm: Some(key),
            peer: Some(peer),
            kind: TapKind::ChannelClosed,
        });
        if let Some(p) = peers.get_mut(peer) {
            p.channels.remove(&ch.id);
        }
        let mut q = Queues {
            peers,
            transmit: &mut events.transmit,
        };
        if let Some(sched) = rs.scheduler.as_mut() {
            sched.close_channel(peer, &rs.swarm, &mut q);
        }
        if rs.channels.is_empty()
            && let Some(mut sched) = rs.scheduler.take()
        {
            tap.absorb(sched.tap_mut());
        }
    }

    /// Subscribes to `stream` from `peer` from `start` (a STREAM_REQUEST).
    /// Outside [`PushMode::Hybrid`] the peer becomes the stream's source at
    /// once, and the bins of the stream it announced are left to it. In
    /// `Hybrid` this is a manual graft: the request goes out at once, and
    /// the peer becomes the stream's parent when it opens the stream.
    /// [`RunnerError::HandshakePending`] until the peer's handshake arrived
    /// on the channel.
    pub fn request_stream(
        &mut self,
        peer: PeerKey,
        swarm: SwarmKey,
        stream: Stream,
        start: Bin,
    ) -> Result<(), RunnerError> {
        let Runner {
            swarms,
            peers,
            events,
            ..
        } = self;
        let rs = swarms.get_mut(swarm).ok_or(RunnerError::UnknownSwarm)?;
        let sched = rs.scheduler.as_mut().ok_or(RunnerError::UnknownSwarm)?;
        if rs.channels.contains_key(&peer) && !sched.handshake_received(peer) {
            return Err(RunnerError::HandshakePending);
        }
        if !sched.do_stream_sub(peer, stream, start) {
            return Err(SchedulerError::InvalidStream(stream).into());
        }
        // Hybrid: the request is written at once.
        let mut q = Queues {
            peers,
            transmit: &mut events.transmit,
        };
        if sched.push_stats().mode == PushMode::Hybrid {
            crate::scheduler::PeerTaskQueue::enqueue_now(&mut q, peer, swarm);
        } else {
            crate::scheduler::PeerTaskQueue::enqueue(&mut q, peer, swarm);
        }
        Ok(())
    }

    /// Cancels a stream subscription (a STREAM_CANCEL); in
    /// [`PushMode::Hybrid`] a manual prune.
    pub fn cancel_stream(
        &mut self,
        peer: PeerKey,
        swarm: SwarmKey,
        stream: Stream,
    ) -> Result<(), RunnerError> {
        let Runner {
            swarms,
            peers,
            events,
            last_now,
            ..
        } = self;
        let rs = swarms.get_mut(swarm).ok_or(RunnerError::UnknownSwarm)?;
        let sched = rs.scheduler.as_mut().ok_or(RunnerError::UnknownSwarm)?;
        let mut q = Queues {
            peers,
            transmit: &mut events.transmit,
        };
        if !sched.cancel_stream_sub(peer, stream, &rs.swarm, *last_now, &mut q) {
            return Err(SchedulerError::InvalidStream(stream).into());
        }
        crate::scheduler::PeerTaskQueue::enqueue(&mut q, peer, swarm);
        Ok(())
    }

    // --- input ----------------------------------------------------------

    /// Handles a frame body from `peer`: any number of channel payloads,
    /// each dispatched to its swarm's channel (unknown channels are
    /// skipped). On an error, the messages before it have been handled and
    /// the rest of the frame is dropped.
    pub fn handle_frame(
        &mut self,
        peer: PeerKey,
        now: Time,
        body: &[u8],
    ) -> Result<(), RunnerError> {
        self.touch_clock(now);
        let upload = (self.hybrid_swarms > 0).then(|| self.upload.info(now));
        let r = self.handle_frame_inner(peer, now, body, upload);
        self.observe_upload(now);
        self.refresh_seed_weights(now);
        r
    }

    fn handle_frame_inner(
        &mut self,
        peer: PeerKey,
        now: Time,
        body: &[u8],
        upload: Option<UploadInfo>,
    ) -> Result<(), RunnerError> {
        let Runner {
            swarms,
            peers,
            events,
            touched,
            committed,
            ..
        } = self;
        if !peers.contains_key(peer) {
            return Err(RunnerError::UnknownPeer);
        }
        touched.clear();
        for item in FrameReader::new(body) {
            let (h, payload) = item?;
            let Some(&key) = peers.get(peer).and_then(|p| p.channels.get(&h.channel)) else {
                continue;
            };
            if !touched.contains(&key) {
                touched.push(key);
            }
            let Some(rs) = swarms.get_mut(key) else {
                continue;
            };
            if let (Some(info), Some(sched)) = (upload, rs.scheduler.as_mut()) {
                sched.set_upload(UploadInfo {
                    others: *committed - rs.committed,
                    ..info
                });
            }
            let mut q = Queues {
                peers,
                transmit: &mut events.transmit,
            };
            handle_payload(rs, key, peer, now, payload, &mut q, &mut events.readable)?;
        }

        let mut q = Queues {
            peers,
            transmit: &mut events.transmit,
        };
        for &key in touched.iter() {
            if let Some(sched) = swarms.get_mut(key).and_then(|rs| rs.scheduler.as_mut()) {
                sched.handle_message_end(peer, now, &mut q);
            }
        }
        Ok(())
    }

    // --- output ---------------------------------------------------------

    /// True while `peer` has a write cycle in progress: call
    /// [`Runner::poll_transmit`] until it's false.
    pub fn wants_transmit(&self, peer: PeerKey) -> bool {
        self.peers.get(peer).is_some_and(|p| p.wake == Wake::Now)
    }

    /// Fills `buf` with one frame body for `peer` and returns its length:
    /// the channels queued to write (handshakes, HAVEs, CANCELs, requests)
    /// first, then queued data by priority, each channel payload as large as
    /// fits. Hybrid swarms' answers go only while the answer gate admits
    /// them ([`crate::answer_gate`]). Returns 0 when there is nothing to
    /// send.
    pub fn poll_transmit(&mut self, peer: PeerKey, now: Time, buf: &mut [u8]) -> usize {
        let Runner {
            swarms,
            peers,
            events,
            held,
            answer_rate,
            ..
        } = self;
        let mut detached = match peers.get_mut(peer) {
            Some(p) if p.wake == Wake::Now => p.rq.detach(),
            _ => return 0,
        };

        let mut off = 0;
        while let Some(key) = detached.pop() {
            let rest = buf.get_mut(off..).unwrap_or_default();
            let (n, full) = write_channel(swarms, peer, key, now, rest);
            off += n;
            if full {
                if let Some(p) = peers.get_mut(peer) {
                    p.rq.push(key);
                    p.rq.reattach(detached);
                }
                break;
            }
        }

        'priorities: for pri in PeerPriority::ALL {
            let gated = pri == PeerPriority::Scavenger;
            loop {
                if gated && !answer_gate::admits(peers, answer_rate, now) {
                    if peers.get(peer).is_some_and(|p| !p.queue(pri).is_empty()) {
                        held.insert(peer);
                    }
                    continue 'priorities;
                }
                let Some((key, b, t)) = peers
                    .get_mut(peer)
                    .and_then(|p| p.dq.get_mut(pri.index()))
                    .and_then(|q| q.pop())
                else {
                    continue 'priorities;
                };
                let rest = buf.get_mut(off..).unwrap_or_default();
                let frame_empty = off == 0;
                match write_channel_data(
                    swarms,
                    peers,
                    events,
                    peer,
                    key,
                    (b, t, pri),
                    now,
                    rest,
                    frame_empty,
                ) {
                    Ok(n) => {
                        off += n;
                        if gated {
                            answer_rate.spend(n);
                        }
                    }
                    Err(WriteDataError::NotEnoughSpace) => continue 'priorities,
                    Err(WriteDataError::Dropped) => {}
                }
            }
        }

        if let Some(p) = peers.get_mut(peer) {
            if p.is_idle() {
                p.wake = Wake::Idle;
            } else if off == 0 {
                // Nothing fit, or the answer gate holds what's left: wait
                // for the next tick (or the gate) so the driver doesn't
                // spin on empty frames.
                p.wake = Wake::AtTick;
            }
        }
        self.reopen(now);
        if self.hybrid_swarms > 0 {
            self.upload.bytes.add(off as u64, now);
            self.answer_rate.wrote(off);
            self.observe_upload(now);
        }
        off
    }

    /// Wakes the peers whose answers the answer gate held back, once it
    /// admits answers again.
    fn reopen(&mut self, now: Time) {
        if self.held.is_empty() || !answer_gate::admits(&mut self.peers, &mut self.answer_rate, now)
        {
            return;
        }
        for peer in core::mem::take(&mut self.held) {
            if let Some(p) = self.peers.get_mut(peer)
                && !p.queue(PeerPriority::Scavenger).is_empty()
                && p.wake != Wake::Now
            {
                p.wake = Wake::Now;
                self.events.transmit.insert(peer);
            }
        }
    }

    // --- timers ---------------------------------------------------------

    /// The next tick while any channel runs, or an earlier push deadline
    /// ([`PushMode::Hybrid`] only). `None` while no channel runs: peers
    /// without channels have nothing to tick, which keeps idle runners
    /// free.
    pub fn poll_timeout(&self) -> Option<Time> {
        let idle = self.swarms.values().all(|rs| rs.scheduler.is_none());
        if self.next_tick.is_nil() || idle {
            return None;
        }
        let mut t = self.next_tick;
        if self.hybrid_swarms > 0 {
            for rs in self.swarms.values() {
                if let Some(d) = rs.scheduler.as_ref().and_then(|s| s.next_push_deadline()) {
                    t = t.min(d);
                }
            }
        }
        // Answers held for want of tokens go when the bucket refills.
        if !self.held.is_empty() && self.answer_rate.exhausted() {
            t = t.min(self.answer_rate.open_at());
        }
        Some(t)
    }

    /// Runs the tick if it is due: every scheduler's run (request
    /// timeouts, restarts, garbage collection, the first read offset),
    /// deferred write cycles, and every
    /// [`PEER_QOS_UPDATE_INTERVAL`] the peers' upload weights. In
    /// [`PushMode::Hybrid`] every call also releases push-expected chunks
    /// whose deadline passed to pull. Every call wakes the peers whose
    /// answers the answer gate held, once it admits them.
    pub fn handle_timeout(&mut self, now: Time) {
        self.touch_clock(now);
        if self.hybrid_swarms > 0 {
            self.expire_push(now);
        }
        self.reopen(now);
        if now < self.next_tick {
            self.observe_upload(now);
            return;
        }
        let behind = (now - self.next_tick).as_nanos() / TICK_INTERVAL.as_nanos() + 1;
        self.next_tick += TICK_INTERVAL * behind;

        if self.hybrid_swarms > 0 {
            self.upload.update_queue_delay(&self.peers);
        }
        self.pace_answers(now);
        let upload = (self.hybrid_swarms > 0).then(|| self.upload.info(now));
        let Runner {
            swarms,
            peers,
            events,
            committed,
            ..
        } = self;
        if upload.is_some() {
            *committed = 0.0;
            for rs in swarms.values_mut() {
                rs.committed = rs.scheduler.as_mut().map_or(0.0, |s| s.committed_push(now));
                *committed += rs.committed;
            }
        }
        for (key, rs) in swarms.iter_mut() {
            let RunnerSwarm {
                swarm,
                scheduler,
                committed: own,
                ..
            } = rs;
            let Some(sched) = scheduler.as_mut() else {
                continue;
            };
            if let Some(info) = upload {
                sched.set_upload(UploadInfo {
                    others: *committed - *own,
                    ..info
                });
                sched.tap_upload(*own);
            }
            let mut q = Queues {
                peers,
                transmit: &mut events.transmit,
            };
            sched.run(now, swarm, &mut q);
            drain_swarm(swarm, Some(sched), key, now, &mut q, &mut events.readable);
        }
        for (key, p) in peers.iter_mut() {
            if p.wake == Wake::AtTick {
                p.wake = Wake::Now;
                events.transmit.insert(key);
            }
        }

        if now >= self.next_weight_update {
            let behind = (now - self.next_weight_update).as_nanos()
                / PEER_QOS_UPDATE_INTERVAL.as_nanos()
                + 1;
            self.next_weight_update += PEER_QOS_UPDATE_INTERVAL * behind;
            self.update_peer_weights(now);
        }
        self.refresh_seed_weights(now);
        self.observe_upload(now);
    }

    /// The tick of the answers' pace ([`crate::answer_gate`]): reads the
    /// links' queueing excess, and taps the rate and reading when they
    /// change.
    fn pace_answers(&mut self, now: Time) {
        self.answer_rate
            .update(now, self.peers.values().filter_map(|p| p.link_path));
        let tapped = (self.answer_rate.rate(), self.answer_rate.reading());
        if self.tap_level == TapLevel::All && self.tapped_rate != Some(tapped) {
            self.tapped_rate = Some(tapped);
            self.tap.push(TapEvent {
                swarm: None,
                peer: None,
                kind: TapKind::AnswerRate {
                    rate: tapped.0,
                    reading: tapped.1,
                },
            });
        }
    }

    /// Releases push-expected chunks whose deadline passed to pull.
    fn expire_push(&mut self, now: Time) {
        let Runner {
            swarms,
            peers,
            events,
            ..
        } = self;
        for rs in swarms.values_mut() {
            let Some(sched) = rs.scheduler.as_mut() else {
                continue;
            };
            if sched.next_push_deadline().is_some_and(|d| d <= now) {
                let mut q = Queues {
                    peers,
                    transmit: &mut events.transmit,
                };
                sched.expire_push(now, &mut q);
            }
        }
    }

    /// Recomputes the weights at once when a hybrid seed under
    /// [`SeedQos::ByAllocation`](crate::SeedQos::ByAllocation) changed its
    /// subscribers, so its uplink shares follow its pushes.
    fn refresh_seed_weights(&mut self, now: Time) {
        let mut changed = false;
        for rs in self.swarms.values_mut() {
            if let Some(sched) = rs.scheduler.as_mut() {
                changed |= sched.take_seed_weights_changed();
            }
        }
        if changed {
            self.update_peer_weights(now);
        }
    }

    /// Upload weight in proportion to the useful data the peer sends us,
    /// between [`MIN_PEER_QOS_WEIGHT`] and [`MAX_PEER_QOS_WEIGHT`]. Only
    /// data that verified and was new counts, so junk earns nothing.
    ///
    /// Rates are normalized to the top peer's (it gets the maximum), so
    /// weights keep their spread however many peers contribute. When nobody
    /// sent anything useful in the window (a seed, a pause) every peer gets
    /// the maximum, an equal share. A hybrid seed under
    /// [`SeedQos::ByAllocation`](crate::SeedQos::ByAllocation) gives each
    /// peer at least its share of the streams pushed to it, and the minimum
    /// instead of the maximum when nobody sent anything.
    fn update_peer_weights(&mut self, now: Time) {
        let mut seed_weights = BTreeMap::new();
        let mut seeding = false;
        for rs in self.swarms.values() {
            if let Some(sched) = rs.scheduler.as_ref() {
                seeding |= sched.seed_weights(&mut seed_weights);
            }
        }
        let top = self
            .peers
            .values_mut()
            .map(|p| p.useful.rate(now))
            .max()
            .unwrap_or(0);
        for (key, p) in self.peers.iter_mut() {
            let weight = if top == 0 {
                if seeding {
                    MIN_PEER_QOS_WEIGHT
                } else {
                    MAX_PEER_QOS_WEIGHT
                }
            } else {
                let rate = u128::from(p.useful.rate(now));
                (rate * u128::from(MAX_PEER_QOS_WEIGHT) / u128::from(top)) as u64
            };
            let weight = weight
                .max(seed_weights.get(&key).copied().unwrap_or(0))
                .clamp(MIN_PEER_QOS_WEIGHT, MAX_PEER_QOS_WEIGHT);
            if p.qos_weight != Some(weight) {
                p.qos_weight = Some(weight);
                self.events.weights.insert(key, weight);
            }
        }
    }

    /// The peer's upload weight (set when it was added, then every
    /// [`PEER_QOS_UPDATE_INTERVAL`], and when a seed's push subscribers
    /// change).
    pub fn peer_qos_weight(&self, peer: PeerKey) -> Option<u64> {
        self.peers.get(peer).and_then(|p| p.qos_weight)
    }

    /// The next notification: readable swarms, peers to poll, then weight
    /// changes.
    pub fn poll_event(&mut self) -> Option<Event> {
        if let Some(key) = self.events.readable.pop_first() {
            return Some(Event::Readable(key));
        }
        while let Some(peer) = self.events.transmit.pop_first() {
            if self.wants_transmit(peer) {
                return Some(Event::Transmit(peer));
            }
        }
        if let Some((peer, weight)) = self.events.weights.pop_first() {
            return Some(Event::QosWeight { peer, weight });
        }
        None
    }

    // --- metrics --------------------------------------------------------

    /// The peer's data transfer totals and rates, overall and per swarm.
    pub fn peer_metrics(&mut self, peer: PeerKey, now: Time) -> Option<PeerMetricsSnapshot> {
        let p = self.peers.get_mut(peer)?;
        let metrics = MetricsSnapshot {
            read: p.read.snapshot(now),
            write: p.write.snapshot(now),
        };
        let swarms = p
            .channels
            .values()
            .copied()
            .filter_map(|key| {
                let ch = self.swarms.get_mut(key)?.channels.get_mut(&peer)?;
                Some((
                    key,
                    MetricsSnapshot {
                        read: ch.read.snapshot(now),
                        write: ch.write.snapshot(now),
                    },
                ))
            })
            .collect();
        Some(PeerMetricsSnapshot { metrics, swarms })
    }

    /// The channel's message counters and scheduler state.
    pub fn channel_stats(&self, peer: PeerKey, swarm: SwarmKey) -> Option<ChannelStats> {
        let rs = self.swarms.get(swarm)?;
        let ch = rs.channels.get(&peer)?;
        Some(ChannelStats {
            scheduler: rs.scheduler.as_ref()?.channel_stats(peer)?,
            inbound: ch.metrics_in,
            outbound: ch.metrics_out,
        })
    }

    /// The swarm scheduler's state, while it has channels.
    pub fn swarm_stats(&self, swarm: SwarmKey) -> Option<SwarmSchedulerStats> {
        Some(self.swarms.get(swarm)?.scheduler.as_ref()?.stats())
    }

    /// The swarm's push/pull state and counters, while it has channels.
    /// See [`scheduler_peer::push`](crate::scheduler_peer::push).
    pub fn push_stats(&self, swarm: SwarmKey) -> Option<PushStats> {
        Some(self.swarms.get(swarm)?.scheduler.as_ref()?.push_stats())
    }

    /// A hybrid seed's view of its neighbours, while it has channels.
    pub fn seed_allocation(&self, swarm: SwarmKey, now: Time) -> Option<SeedAllocationStats> {
        self.swarms
            .get(swarm)?
            .scheduler
            .as_ref()?
            .seed_allocation(now)
    }

    /// Fixes the peers a hybrid seed pushes each stream to, one list per
    /// stream; `None` lifts the pin. While pinned, the seed accepts a
    /// stream's STREAM_REQUESTs from its listed peers only, closes its
    /// other subscribers and sheds none: an experiment's fixed allocation.
    /// The pin outlives the swarm's channels.
    pub fn pin_seed_allocation(
        &mut self,
        swarm: SwarmKey,
        pin: Option<Vec<Vec<PeerKey>>>,
    ) -> Result<(), RunnerError> {
        let Runner {
            swarms,
            peers,
            events,
            ..
        } = self;
        let rs = swarms.get_mut(swarm).ok_or(RunnerError::UnknownSwarm)?;
        rs.seed_pin = pin.clone();
        if let Some(sched) = rs.scheduler.as_mut() {
            let mut q = Queues {
                peers,
                transmit: &mut events.transmit,
            };
            sched.pin_allocation(pin, &mut q);
        }
        Ok(())
    }

    /// The peer's queue state.
    pub fn peer_stats(&self, peer: PeerKey) -> Option<PeerStats> {
        self.peers.get(peer).map(|p| p.stats())
    }
}

/// Writes the channel's queued control messages (handshake, HAVEs,
/// requests, ...) as one payload into the frame: returns the bytes used and
/// whether the frame filled up.
fn write_channel(
    swarms: &mut SlotMap<SwarmKey, RunnerSwarm>,
    peer: PeerKey,
    key: SwarmKey,
    now: Time,
    buf: &mut [u8],
) -> (usize, bool) {
    let Some(rs) = swarms.get_mut(key) else {
        return (0, false);
    };
    let RunnerSwarm {
        swarm,
        scheduler,
        channels,
        ..
    } = rs;
    let (Some(sched), Some(ch)) = (scheduler.as_mut(), channels.get_mut(&peer)) else {
        return (0, false);
    };
    let mut cw = MeteredWriter::new(buf, ch.peer_channel);
    let r = sched.write(peer, now, swarm, &mut cw);
    let (n, _) = cw.finish(&mut ch.metrics_out);
    (n, r == Err(codec::Error::NotEnoughSpace))
}

/// Writes one queued data entry (integrity and data) as one payload into
/// the frame.
#[allow(clippy::too_many_arguments)]
fn write_channel_data(
    swarms: &mut SlotMap<SwarmKey, RunnerSwarm>,
    peers: &mut SlotMap<PeerKey, RunnerPeer>,
    events: &mut Events,
    peer: PeerKey,
    key: SwarmKey,
    (b, t, pri): (Bin, Time, PeerPriority),
    now: Time,
    buf: &mut [u8],
    frame_empty: bool,
) -> Result<usize, WriteDataError> {
    let Some(rs) = swarms.get_mut(key) else {
        return Err(WriteDataError::Dropped);
    };
    let RunnerSwarm {
        swarm,
        scheduler,
        channels,
        ..
    } = rs;
    let (Some(sched), Some(ch)) = (scheduler.as_mut(), channels.get_mut(&peer)) else {
        return Err(WriteDataError::Dropped);
    };
    let mut cw = MeteredWriter::new(buf, ch.peer_channel);
    let mut q = Queues {
        peers,
        transmit: &mut events.transmit,
    };
    sched.write_data(peer, b, t, pri, now, swarm, &mut cw, frame_empty, &mut q)?;
    let (n, data) = cw.finish(&mut ch.metrics_out);
    ch.write.add(data, now);
    if let Some(p) = peers.get_mut(peer) {
        p.write.add(data, now);
    }
    Ok(n)
}

/// A swarm borrowed from the [`Runner`] for publishing. When it drops, the
/// swarm's events reach the scheduler, so the scheduler has seen every
/// published chunk before the runner handles anything else.
pub struct SwarmMut<'a> {
    runner: &'a mut Runner,
    key: SwarmKey,
    now: Time,
}

impl Deref for SwarmMut<'_> {
    type Target = Swarm;

    fn deref(&self) -> &Swarm {
        // invariant: the key was checked when the guard was made, and the
        // guard holds the runner exclusively, so the swarm can't go away.
        #[allow(clippy::indexing_slicing)]
        &self.runner.swarms[self.key].swarm
    }
}

impl DerefMut for SwarmMut<'_> {
    fn deref_mut(&mut self) -> &mut Swarm {
        // invariant: as in `deref`.
        #[allow(clippy::indexing_slicing)]
        &mut self.runner.swarms[self.key].swarm
    }
}

impl Drop for SwarmMut<'_> {
    fn drop(&mut self) {
        self.runner.drain(self.key, self.now);
    }
}

#[cfg(test)]
#[path = "runner_test.rs"]
mod tests;
