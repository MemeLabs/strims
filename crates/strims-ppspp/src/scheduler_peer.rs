// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The peer scheduler: what a swarm requests from, announces to and pushes
//! to each peer.
//!
//! `PeerSwarmScheduler` holds a swarm's shared state (`SwarmState`) and a
//! `PeerChannelScheduler` per peer. Channel methods take the shared state
//! and the peers' queues (a `PeerTaskQueue`) as arguments, so every method
//! is a plain `&mut` borrow.
//!
//! # Pull
//!
//! Each channel requests chunks its peer HAVEs and nobody was asked for
//! (`request_bins`), oldest request group first, up to its request window:
//! a [`RequestController`] chosen by `SwarmOptions::request_controller`. A
//! request not answered within the window's request timeout is cancelled
//! and becomes requestable again.
//!
//! Constraints on what a channel asks for:
//!
//! - **Inside the peer's window.** Requests start at the peer's live window
//!   as its newest HAVE implies, moved on by the chunks the stream adds
//!   before an answer can arrive. A request for a chunk the peer has evicted
//!   is never answered and times out as a loss, so a channel that asked
//!   behind that point would never catch up.
//! - **Spread over a shared peer.** Within a group of [`REQUEST_GROUP`]
//!   chunks each node asks in its own random order, so nodes that hear of
//!   chunks from the same peer at the same time ask it for different ones
//!   and can trade the rest.
//! - **Away from peers that fail to deliver.** A bin whose request timed
//!   out, whose data was invalid, or whose push we gave up on is left to
//!   the other channels for `REQUEST_BACKOFF`, and every channel gets a
//!   write cycle, so a peer that announces chunks and withholds them can't
//!   keep them.
//! - **Enough to carry the stream.** LEDBAT's window has a floor of
//!   [`STREAM_WINDOW_FLOOR`] times the stream's bandwidth-delay product,
//!   which yields to losses ([`Ledbat::set_min_cwnd`]).
//! - **Flight that settles.** A request whose chunk arrived from another
//!   peer while this one never answered leaves the window's flight at its
//!   timeout without a loss signal; an answer unsets its request time, so a
//!   second copy doesn't count again.
//!
//! In [`PushMode::EagerHave`] and [`PushMode::Hybrid`], HAVEs go out as soon
//! as chunks are stored and requests as soon as a HAVE makes chunks
//! requestable; in [`PushMode::Off`] both wait for the next tick. Every
//! chunk of a frame lands in the HAVE map before the driver polls, so an
//! eager channel writes once per frame handled, not once per chunk.
//!
//! # Streams and push
//!
//! STREAM_REQUEST subscribes the requester to a stream: what we have of it
//! from the requested bin (at most a window back) is queued at high
//! priority, and each new chunk of it is pushed as it is stored. A repeat
//! from the same or a later bin only moves the subscription. STREAM_CANCEL
//! unsubscribes. STREAM_OPEN is accepted only for a stream we requested
//! from that peer, and makes the peer the stream's source. Outside
//! [`PushMode::Hybrid`], a source's announced chunks that nobody else was
//! asked for are marked requested with no deadline and stay with it until
//! it sends STREAM_CLOSE or the channel closes; such a source exists only
//! after [`Runner::request_stream`]. In `Hybrid` the [`push`] module
//! manages subscriptions, and every chunk expected by push has a deadline
//! after which pull takes it back.
//!
//! # Bounds on peer input
//!
//! Every per-message path costs at most O(live window) and usually O(new
//! bins), and every collection a peer can grow is bounded (each field
//! states its bound):
//!
//! - bins at or past [`MAX_BIN`] are ignored, and a REQUEST must also pass
//!   [`peer_bin_ok`] against the window;
//! - per channel, a peer's HAVEs are tracked only for the last
//!   [`HISTORY_WINDOWS`] windows before the highest bin it announced, so a
//!   HAVE of a huge bin costs at most that much;
//! - bin times are kept for the same span before the newest bin seen; a
//!   seed's are its publish times, which no HAVE sets or prunes;
//! - the swarm's have and request maps are pruned behind the store's
//!   window, so the HAVEs we send never claim more than a window behind it;
//! - a channel has at most [`max_pending_requests`] request deadlines, and a
//!   request round collects at most a frame of requests plus one request
//!   group;
//! - extra messages are deduplicated: a handshake, a restart, one pong,
//!   one ask for reports and one message per stream and direction; a
//!   report holds at most twice the stream count of entries;
//! - STREAM_REQUEST walks only the stream's own bins, and none on a repeat;
//!   STREAM_CLOSE walks only the bins left to sources, from the store's
//!   window;
//! - invalid data releases its bin only inside our window.
//!
//! [`Ledbat::set_min_cwnd`]: crate::ledbat::Ledbat::set_min_cwnd
//! [`Runner::request_stream`]: crate::Runner::request_stream
//! [`peer_bin_ok`]: crate::peer_bin_ok

use indexmap::IndexMap;
use slotmap::Key;
use strims_base::rng::DetRng;
use strims_base::stats::Sma;
use strims_base::time::{Duration, Time};
use strims_binmap::{Bin, Cursor, IntersectionIter, Map};

use crate::MAX_BIN;
use crate::bin_queue::BinQueue;
use crate::channel::MeteredWriter;
use crate::codec::{
    self, Announce, Cancel, Have, MAX_PEER_ID, Message, MessageWriter, Ping, Pong, Request, Stream,
    StreamCancel, StreamClose, StreamOpen, StreamRequest,
};
use crate::controller::{ChannelController, RequestController, RequestControllerKind};
use crate::integrity::ProtectionMethod;
use crate::options::{DeliveryMode, PushMode, SchedulingMethod, SeedPolicy};
use crate::runner::{PeerKey, SwarmKey, TICK_INTERVAL};
use crate::scheduler::{PeerPriority, PeerTaskQueue, stream_bin_offset};
use crate::store;
use crate::swarm::Swarm;
use crate::tap::{Tap, TapKind};
use crate::time_set::TimeSet;

#[path = "scheduler_push.rs"]
pub mod push;

use push::{PushChannel, PushState};

/// How often the scheduler prunes state behind the window.
pub(crate) const SCHEDULER_GC_INTERVAL: Duration = Duration::from_secs(5);
/// How long a channel waits for the peer's handshake before it sends
/// RESTART (again).
pub(crate) const SCHEDULER_RESTART_COOLDOWN: Duration = Duration::from_secs(1);
/// How often a channel that writes also pings.
pub(crate) const PING_INTERVAL: Duration = Duration::from_secs(1);
/// Pings whose round trips a channel still accepts for the upload meter:
/// a queue up to this many [`PING_INTERVAL`]s long still shows.
const SENT_PINGS: usize = 4;
/// How long a channel leaves bins alone after its peer failed to deliver
/// them: two ticks, so every other channel gets a write cycle to request
/// them first.
pub(crate) const REQUEST_BACKOFF: Duration = Duration::from_millis(200);

/// How many live windows of a peer's HAVEs, and of bin times, are tracked
/// behind the highest bin seen.
pub const HISTORY_WINDOWS: u64 = 2;

/// Chunks per request group: a channel requests groups in stream order
/// and the chunks within a group in its node's own random order, so nodes
/// asking the same peer ask for different chunks. Larger groups decorrelate
/// more; smaller ones delay the reader's next chunk less when a window
/// can't take a whole group.
pub const REQUEST_GROUP: u64 = 64;

pub use crate::controller::{STREAM_WINDOW_FLOOR, STREAM_WINDOW_FLOOR_RTT};

/// The group of chunk bin `b`.
fn request_group(b: Bin) -> u64 {
    b.base_offset() / REQUEST_GROUP
}

/// Where chunk bin `b` comes in a request round: its group, then its
/// place in `order` within the group.
fn request_key(order: &[u8], b: Bin) -> (u64, u8) {
    let o = b.base_offset();
    let i = usize::try_from(o % REQUEST_GROUP).unwrap_or(0);
    (o / REQUEST_GROUP, order.get(i).copied().unwrap_or(0))
}

/// The smallest REQUEST: type byte, one-byte bin and timestamp varints.
const MIN_REQUEST_LEN: usize = 3;
/// Candidate bins kept allocated between request rounds.
const CANDIDATE_CAPACITY: usize = 256;

/// The most request deadlines a channel tracks: one live window of
/// chunks. Requests beyond it wait for deadlines to pass, as if the
/// congestion window were full.
pub fn max_pending_requests(live_window: u64) -> usize {
    usize::try_from(live_window).unwrap_or(usize::MAX)
}

/// Errors a channel scheduler returns for a message; the frame fails with
/// them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SchedulerError {
    #[error("stream {0} out of range")]
    InvalidStream(Stream),
}

/// What [`PeerSwarmScheduler::handle_stream_open`] made of a STREAM_OPEN.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StreamOpenOutcome {
    /// The peer is the stream's source now.
    Accepted,
    /// The channel has no handshake from the peer, yet or since the stream
    /// restarted, or there is no channel: no request of ours can be waiting
    /// for the open. Ignored without counting: after a restart, answers to
    /// requests from the old stream land here.
    BeforeHandshake,
    /// It answers a request we cancelled before the answer arrived: a race
    /// between honest peers, ignored (`ChannelMetrics::stale_stream_open`).
    Stale,
    /// Unsolicited or out of bounds, ignored (`ChannelMetrics::rejected`).
    Rejected,
}

/// Per stream, whether a STREAM_OPEN may still answer a request of ours:
/// an open that crossed our STREAM_CANCEL is a race between honest peers,
/// an unsolicited one is invalid input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OpenRace {
    /// No request is waiting for an answer.
    #[default]
    None,
    /// We sent (or queued) a STREAM_REQUEST and got no OPEN or CLOSE yet.
    Awaiting,
    /// We cancelled such a request: one OPEN may still cross the cancel.
    Cancelled,
}

/// Why [`PeerSwarmScheduler::write_data`] wrote nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WriteDataError {
    /// The frame is full; the bin (or its parts) went back to the front of
    /// the queue.
    NotEnoughSpace,
    /// The bin can't be sent (not stored, integrity missing, or a chunk
    /// too large for an empty frame); it was dropped.
    Dropped,
}

/// A channel subscribed to a stream, from `start_bin`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StreamSubscription {
    pub(crate) start_bin: Bin,
    pub(crate) channel: PeerKey,
    /// Hybrid: accepted beyond the interior stream (shed first).
    pub(crate) extra: bool,
    /// Hybrid: when it was accepted (the newest is shed first).
    pub(crate) since: Time,
}

/// A stream's push state: who pushes it to us and whom we push it to.
#[derive(Clone, Debug, Default)]
pub(crate) struct PeerSchedulerStream {
    /// The channel pushing this stream to us.
    pub(crate) source: Option<StreamSubscription>,
    /// The channels we push this stream to. Bound: one per channel.
    pub(crate) subscribers: Vec<StreamSubscription>,
}

impl PeerSchedulerStream {
    /// Subscribes `channel` from `b`, replacing its earlier subscription.
    fn add_subscriber(&mut self, channel: PeerKey, b: Bin, extra: bool, since: Time) {
        self.remove_subscriber(channel);
        self.subscribers.push(StreamSubscription {
            start_bin: b,
            channel,
            extra,
            since,
        });
    }

    /// Unsubscribes `channel`; true if it was subscribed.
    fn remove_subscriber(&mut self, channel: PeerKey) -> bool {
        let i = self.subscribers.iter().position(|s| s.channel == channel);
        if let Some(i) = i {
            self.subscribers.swap_remove(i);
        }
        i.is_some()
    }
}

/// The swarm-wide part of the scheduler, shared by its channels.
#[derive(Debug)]
pub(crate) struct SwarmState {
    pub(crate) key: SwarmKey,

    stream_count: u16,
    stream_bits: u64,
    signature_layer: u64,
    /// When each bin was first seen. Bound: [`HISTORY_WINDOWS`] windows
    /// before `bin_times_max`.
    bin_times: TimeSet,
    bin_times_max: Bin,

    peer_have_chunk_rate: Sma,
    peer_max_have_bin: Bin,

    /// The chunks we have. Bound: pruned one window behind the store's.
    pub(crate) have_bins: Map,
    pub(crate) have_bin_max: Bin,
    pub(crate) wasted_chunks: u64,
    /// Chunks written to peers, pushes included, and the pushes alone.
    chunks_sent: u64,
    pushed_chunks: u64,
    /// The chunks written to anyone. Bound: pruned behind the store's
    /// window at GC.
    sent_bins: Map,
    distinct_chunks_sent: u64,

    /// Bins requested from someone or had. Bound: filled before the store's
    /// window; the rest is the window plus outstanding requests.
    pub(crate) request_bins: Map,
    /// Bins left to a stream's source outside the hybrid: requested on its
    /// behalf, with no deadline. Bound: a subset of `request_bins` (pruned
    /// behind the store's window at GC, cleared on arrival).
    source_bins: Map,
    pub(crate) streams: Vec<PeerSchedulerStream>,
    /// Streams with a source: the source marking walk in `handle_have`
    /// runs only while there are any.
    stream_sources: usize,

    chunk_size: usize,
    /// The live window in bins (twice the chunks).
    live_window: Bin,
    /// The live window in chunks.
    window: u64,
    delivery_mode: DeliveryMode,

    pub(crate) first_chunk_set: bool,

    next_gc_time: Time,

    /// A random permutation of the chunk offsets in a request group: the
    /// order in which this node requests a group's chunks.
    /// Bound: [`REQUEST_GROUP`] entries.
    request_order: Vec<u8>,

    /// How push mixes into pull.
    mode: PushMode,
    /// The channels' request window model (resolved).
    controller: RequestControllerKind,
    /// The node publishes the swarm (`SchedulingMethod::SEED`).
    is_seed: bool,
    /// How a hybrid seed spends its upload.
    seed: SeedPolicy,
    /// The hybrid's state, with [`PushMode::Hybrid`] only.
    push: Option<PushState>,
    /// Decisions for the driver's capture.
    pub(crate) tap: Tap,
    /// Our subscribers changed since the runner last asked.
    subscribers_changed: bool,
}

impl SwarmState {
    /// The stream of chunk bin `b`: chunk `i` is stream
    /// `i % stream_count`.
    pub(crate) fn bin_stream(&self, b: Bin) -> Stream {
        ((b.0 >> 1) & self.stream_bits) as Stream
    }

    /// The first base bin of the [`HISTORY_WINDOWS`] windows ending at
    /// `max`.
    fn history_start(&self, max: Bin) -> Bin {
        let span = HISTORY_WINDOWS
            .saturating_mul(self.window)
            .saturating_sub(1);
        Bin(max.base_offset().saturating_sub(span).saturating_mul(2))
    }

    /// Records when `b` (widened to its signature unit) was first seen,
    /// keeping only recent bins: older ones are ignored and a new maximum
    /// prunes behind it.
    fn set_bin_time(&mut self, b: Bin, t: Time) {
        let mut b = b;
        if b.layer() < self.signature_layer {
            b = b.layer_shift(self.signature_layer);
        }
        let br = b.base_right();
        if br > self.bin_times_max || self.bin_times_max == Bin::NONE {
            self.bin_times_max = br;
            let lo = self.history_start(br);
            self.bin_times.prune(lo);
        } else if br < self.history_start(self.bin_times_max) {
            return;
        }
        self.bin_times.set(b, t);
    }

    fn bin_time(&self, b: Bin) -> Time {
        self.bin_times.get(b).map_or(Time::EPOCH, |(_, t)| t)
    }

    /// Counts the chunks of `b`, written to a peer at `pri`.
    fn count_sent(&mut self, b: Bin, pri: PeerPriority) {
        let n = b.base_length();
        self.chunks_sent = self.chunks_sent.saturating_add(n);
        if pri == PeerPriority::High {
            self.pushed_chunks = self.pushed_chunks.saturating_add(n);
        }
        let (mut x, r) = b.base();
        while x <= r {
            if self.sent_bins.empty_at(x) {
                self.distinct_chunks_sent = self.distinct_chunks_sent.saturating_add(1);
            }
            x += 2;
        }
        self.sent_bins.set(b);
    }

    fn set_stream_source(&mut self, stream: Stream, channel: PeerKey, b: Bin) {
        if let Some(s) = self.streams.get_mut(usize::from(stream)) {
            if s.source.is_none() {
                self.stream_sources += 1;
            }
            s.source = Some(StreamSubscription {
                start_bin: b,
                channel,
                extra: false,
                since: Time::EPOCH,
            });
        }
    }

    /// Leaves chunk `b` to its stream's source, unless we have it or it was
    /// requested from another channel.
    fn leave_to_source(&mut self, b: Bin) {
        if self.request_bins.empty_at(b) {
            self.request_bins.set(b);
            self.source_bins.set(b);
        }
    }

    fn reset_stream_source(&mut self, stream: Stream) {
        if let Some(s) = self.streams.get_mut(usize::from(stream))
            && s.source.take().is_some()
        {
            self.stream_sources -= 1;
        }
    }

    fn check_stream(&self, s: Stream) -> Result<(), SchedulerError> {
        if s < self.stream_count {
            Ok(())
        } else {
            Err(SchedulerError::InvalidStream(s))
        }
    }
}

/// The messages a channel sends besides HAVE/CANCEL/REQUEST/DATA,
/// deduplicated so they are bounded: the handshake, restart and ask for
/// reports are flags, only the latest pong is kept, and a stream keeps only
/// its latest message per direction (a later message supersedes an earlier
/// one; their order across streams is first-queued).
#[derive(Debug, Default)]
struct ExtraMessages {
    handshake: bool,
    restart: bool,
    /// An empty ANNOUNCE: an adaptive seed asks for the peer's reports.
    ask: bool,
    pong: Option<u64>,
    /// Bound: two entries per stream.
    order: std::collections::VecDeque<(bool, Stream)>,
    pending: std::collections::BTreeMap<(bool, Stream), StreamMessage>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StreamMessage {
    Request(Bin),
    Cancel,
    Open(Bin),
    Close,
}

impl StreamMessage {
    /// True for the messages we send as a stream's requester, false for
    /// those we send as its provider.
    fn requester(self) -> bool {
        matches!(self, StreamMessage::Request(_) | StreamMessage::Cancel)
    }
}

impl ExtraMessages {
    fn push_stream(&mut self, stream: Stream, m: StreamMessage) {
        let k = (m.requester(), stream);
        if self.pending.insert(k, m).is_none() {
            self.order.push_back(k);
        }
    }

    fn len(&self) -> usize {
        usize::from(self.handshake)
            + usize::from(self.restart)
            + usize::from(self.ask)
            + usize::from(self.pong.is_some())
            + self.pending.len()
    }
}

/// One channel's scheduler: what we request from, announce to and push to
/// one peer.
#[derive(Debug)]
pub(crate) struct PeerChannelScheduler {
    pub(crate) peer: PeerKey,
    /// The peer's id, by which reports name it: `None` if the driver gave
    /// none, or one longer than [`MAX_PEER_ID`].
    id: Option<Vec<u8>>,

    choked: bool,
    /// When we requested each bin. Bound: requested bins, pruned behind the
    /// store's read position.
    request_times: TimeSet,

    /// Bins to send HAVEs for. Bound: our window.
    have_bins: Map,
    /// Bins to send CANCELs for. Bound: timed out requests and chunks
    /// released from the peer as a parent, within the window.
    cancel_bins: Map,
    /// Request deadlines. Bound: [`max_pending_requests`].
    request_bins: BinQueue,
    /// Bins this peer failed to deliver (a request timed out, its data was
    /// invalid, or we gave up on its push), left to the other channels
    /// until `backoff_until`. Bound: those bins in the window.
    backoff_bins: Map,
    backoff_until: Time,
    /// Bound: one per stream.
    request_streams: Vec<Bin>,
    /// Requests whose answer may still arrive. Bound: one per stream.
    open_race: Vec<OpenRace>,
    extra: ExtraMessages,

    enqueue_now: bool,
    peer_live_window: Bin,
    pub(crate) peer_max_have_bin: Bin,
    /// The bins the peer claims to have. Bound: [`HISTORY_WINDOWS`]
    /// windows before `peer_max_have_bin`.
    pub(crate) peer_have_bins: Map,

    /// The request window.
    pub(crate) controller: ChannelController,

    next_restart_time: Time,
    pub(crate) handshake_received: bool,

    next_ping_time: Time,
    /// The nonce of the outstanding ping, 0 if none.
    ping_nonce: u64,
    /// The nonces of the latest pings, unanswered ones nonzero: their
    /// round trips go to the upload meter, however long they take.
    sent_pings: [u64; SENT_PINGS],

    /// Scratch for `write1`. Bound: one frame of requests plus a request
    /// group, and [`CANDIDATE_CAPACITY`] between rounds.
    candidate_bins: Vec<Bin>,

    /// The hybrid's view of the peer as a parent, with
    /// [`PushMode::Hybrid`] only.
    push: Option<PushChannel>,
}

impl PeerChannelScheduler {
    /// A new channel; its first write is our handshake.
    fn new(peer: PeerKey, id: &[u8], s: &SwarmState, now: Time) -> PeerChannelScheduler {
        let streams = usize::from(s.stream_count);
        PeerChannelScheduler {
            peer,
            id: (1..=MAX_PEER_ID).contains(&id.len()).then(|| id.to_vec()),
            choked: false,
            request_times: TimeSet::default(),
            have_bins: s.have_bins.clone(),
            cancel_bins: Map::new(),
            request_bins: BinQueue::default(),
            backoff_bins: Map::new(),
            backoff_until: Time::EPOCH,
            request_streams: vec![Bin::NONE; streams],
            open_race: vec![OpenRace::None; streams],
            extra: ExtraMessages {
                handshake: true,
                ..Default::default()
            },
            enqueue_now: false,
            peer_live_window: Bin(0),
            peer_max_have_bin: Bin(0),
            peer_have_bins: Map::new(),
            controller: Self::new_controller(peer, s, now),
            next_restart_time: now + SCHEDULER_RESTART_COOLDOWN,
            handshake_received: false,
            next_ping_time: Time::EPOCH,
            ping_nonce: 0,
            sent_pings: [0; SENT_PINGS],
            candidate_bins: Vec::new(),
            push: s.push.as_ref().map(|_| PushChannel::new(streams)),
        }
    }

    /// A fresh request window. Its probing phase derives from the peer's
    /// key, so channels probe at different times, deterministically.
    fn new_controller(peer: PeerKey, s: &SwarmState, now: Time) -> ChannelController {
        ChannelController::new(s.controller, s.chunk_size, peer.data().as_ffi(), now)
    }

    /// Forgets the old stream after a swarm reset (a new epoch).
    fn reset(&mut self, s: &SwarmState, now: Time) {
        let streams = usize::from(s.stream_count);
        self.request_times = TimeSet::default();
        self.have_bins = Map::new();
        self.cancel_bins = Map::new();
        self.request_bins = BinQueue::default();
        self.backoff_bins = Map::new();
        // NONE: no request is open, so a later STREAM_CLOSE releases
        // nothing.
        self.request_streams = vec![Bin::NONE; streams];
        self.open_race = vec![OpenRace::None; streams];
        self.extra = ExtraMessages {
            handshake: true,
            ..Default::default()
        };
        self.peer_have_bins = Map::new();
        // The new stream starts at bin 0: requests must not wait for it to
        // pass the old stream's position.
        self.peer_max_have_bin = Bin(0);
        self.controller = Self::new_controller(self.peer, s, now);
        self.next_restart_time = now + SCHEDULER_RESTART_COOLDOWN;
        self.handshake_received = false;
        if self.push.is_some() {
            self.push = Some(PushChannel::new(streams));
        }
    }

    /// Queues a RESTART if the peer's handshake is overdue.
    fn try_restart(&mut self, s: &SwarmState, now: Time, q: &mut impl PeerTaskQueue) {
        if !self.handshake_received && now > self.next_restart_time {
            self.next_restart_time = now + SCHEDULER_RESTART_COOLDOWN;
            self.extra.restart = true;
            q.enqueue(self.peer, s.key);
        }
    }

    /// Times out the requests whose deadline passed. Returns true if any
    /// chunk we still lack timed out.
    ///
    /// Timed out bins skip this channel's request rounds for
    /// [`REQUEST_BACKOFF`] and the caller queues every channel, so a peer
    /// that announces bins and never sends them can't keep them from
    /// everyone.
    ///
    /// A request whose chunk arrived from elsewhere (a push, or another
    /// channel after invalid data) while this peer never answered leaves the
    /// flight here without a loss signal (answers unset their request time).
    /// Otherwise it would stay in flight for good, and at a window of 2
    /// chunks two of them stop the channel from requesting.
    fn time_out_requests(
        &mut self,
        s: &mut SwarmState,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) -> bool {
        let mut n: u64 = 0;
        let mut settled: u64 = 0;
        while let Some(b) = self.request_bins.pop_le(now) {
            let mut it = s.have_bins.iterate_empty_at(b);
            while let Some(x) = it.next_base() {
                s.request_bins.reset(x);
                self.request_times.unset(x);
                self.cancel_bins.set(x);
                self.backoff_bins.set(x);
                n += 1;
                s.tap
                    .emit(s.key, Some(self.peer), || TapKind::RequestTimeout {
                        bin: x,
                    });
            }
            // Requests are single chunks; anything left with a request
            // time arrived from elsewhere unanswered.
            let (mut x, r) = b.base();
            while x <= r {
                if self.request_times.get(x).is_some_and(|(tb, _)| tb == x) {
                    self.request_times.unset(x);
                    settled += 1;
                }
                x += 2;
            }
        }
        if settled > 0 {
            let size = (settled as i64).saturating_mul(s.chunk_size as i64);
            self.controller.on_settled(now, size);
        }
        if n > 0 {
            self.backoff_until = now + REQUEST_BACKOFF;
            let size = (n as i64).saturating_mul(s.chunk_size as i64);
            self.controller.on_timed_out(now, size);
            q.enqueue(self.peer, s.key);
        }
        n > 0
    }

    /// Releases the channel's outstanding requests to the other channels.
    fn clear_requests(&mut self, s: &mut SwarmState) {
        while let Some(b) = self.request_bins.pop_le(Time::MAX) {
            let mut it = s.have_bins.iterate_empty_at(b);
            while let Some(x) = it.next_base() {
                s.request_bins.reset(x);
            }
        }
    }

    /// Ends our subscription to `stream` from this peer. If the peer is the
    /// stream's source, the bins left to it that we lack go back to pull;
    /// chunks requested from other channels stay theirs. The walk covers
    /// only the bins left to sources from the store's window, so it costs
    /// at most a window.
    fn close_stream(&mut self, s: &mut SwarmState, stream: Stream, window_start: Bin) {
        let Some(r) = self.request_streams.get_mut(usize::from(stream)) else {
            return;
        };
        if r.is_none() {
            return;
        }
        *r = Bin::NONE;
        let is_source = s
            .streams
            .get(usize::from(stream))
            .and_then(|st| st.source)
            .is_some_and(|src| src.channel == self.peer);
        if !is_source {
            return;
        }
        let mut cur = Cursor::filled_at(s.source_bins.root_bin());
        let mut next = cur.next_base_after(&s.source_bins, window_start);
        while let Some(x) = next {
            if s.bin_stream(x) == stream {
                s.source_bins.reset(x);
                if s.have_bins.empty_at(x) {
                    s.request_bins.reset(x);
                }
            }
            next = cur.next_base(&s.source_bins);
        }
        s.reset_stream_source(stream);
    }

    /// Queues a STREAM_REQUEST for `stream` from `b`.
    fn add_stream_request(&mut self, stream: Stream, b: Bin) {
        if let Some(r) = self.request_streams.get_mut(usize::from(stream)) {
            *r = b;
        }
        if let Some(o) = self.open_race.get_mut(usize::from(stream)) {
            *o = OpenRace::Awaiting;
        }
        self.extra.push_stream(stream, StreamMessage::Request(b));
    }

    /// We are cancelling our request for `stream`: if the peer hasn't
    /// answered it, its STREAM_OPEN may still cross our STREAM_CANCEL.
    pub(crate) fn cancel_open_race(&mut self, stream: Stream) {
        if let Some(o) = self.open_race.get_mut(usize::from(stream))
            && *o == OpenRace::Awaiting
        {
            *o = OpenRace::Cancelled;
        }
    }

    /// The peer answered our request for `stream` (STREAM_OPEN or
    /// STREAM_CLOSE). Returns whether a cancelled request was waiting for
    /// that answer.
    fn answer_open_race(&mut self, stream: Stream) -> bool {
        self.open_race
            .get_mut(usize::from(stream))
            .is_some_and(|o| core::mem::take(o) == OpenRace::Cancelled)
    }

    /// Queues a STREAM_CANCEL for `stream` and releases what it was
    /// expected to push.
    fn add_stream_cancel(&mut self, s: &mut SwarmState, stream: Stream, window_start: Bin) {
        self.cancel_open_race(stream);
        self.close_stream(s, stream, window_start);
        self.extra.push_stream(stream, StreamMessage::Cancel);
    }

    /// Subscribes the peer to `stream` from `b` (or our tail) and queues
    /// pushes of what we have of it. In hybrid mode the
    /// pushes carry the time they were queued (to drop stale ones), and
    /// `extra` marks a subscription beyond our interior stream.
    #[allow(clippy::too_many_arguments)]
    fn add_stream_open(
        &mut self,
        s: &mut SwarmState,
        stream: Stream,
        b: Bin,
        tail: Bin,
        extra: bool,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) {
        // Without a read offset the tail is `NONE`. The stream then starts
        // where it was asked for, but at most a window behind the newest
        // chunk we have: that bounds the walk below to a window.
        let lo = if tail.is_none() {
            Bin(s.have_bin_max.0.saturating_sub(s.live_window.0))
        } else {
            tail
        };
        let b = if b < lo { lo } else { b };
        let queued = if s.push.is_some() { now } else { Time::EPOCH };
        let Some(st) = s.streams.get_mut(usize::from(stream)) else {
            return;
        };
        // A peer already subscribed from `b` or earlier had those bins
        // queued when it subscribed or as they arrived, so a repeat only
        // moves its start: it walks nothing and queues nothing.
        let resubscribed = st
            .subscribers
            .iter()
            .any(|sub| sub.channel == self.peer && sub.start_bin <= b);
        st.add_subscriber(self.peer, b, extra, now);
        s.subscribers_changed = true;
        if !resubscribed && !b.is_none() {
            // The stream's chunks are every `stream_count`th: the walk
            // visits only those.
            let stride = u64::from(s.stream_count) * 2;
            let first = stream_bin_offset(stream).0;
            let mut x = b.base_left().0;
            if x % stride != first {
                x = x - x % stride + first;
                if x < b.base_left().0 {
                    x += stride;
                }
            }
            while x <= s.have_bin_max.0 {
                if s.have_bins.filled_at(Bin(x))
                    && !q.push_data(self.peer, s.key, Bin(x), queued, PeerPriority::High)
                {
                    s.tap.emit(s.key, Some(self.peer), || TapKind::DataRefused {
                        bin: Bin(x),
                        push: true,
                    });
                }
                x += stride;
            }
        }
        self.extra.push_stream(stream, StreamMessage::Open(b));
    }

    /// Unsubscribes the peer from `stream` and tells it so (STREAM_CLOSE).
    /// Unused: the hybrid's admission and shedding queue their STREAM_CLOSE
    /// directly.
    #[allow(dead_code)]
    fn add_stream_close(&mut self, s: &mut SwarmState, stream: Stream) {
        if let Some(st) = s.streams.get_mut(usize::from(stream)) {
            s.subscribers_changed |= st.remove_subscriber(self.peer);
        }
        self.extra.push_stream(stream, StreamMessage::Close);
    }

    /// Writes the extra messages, HAVEs and CANCELs.
    fn write0(&mut self, swarm: &Swarm, cw: &mut MeteredWriter<'_>) -> Result<(), codec::Error> {
        if self.extra.handshake {
            let h = swarm.handshake_options();
            cw.write(&Message::Handshake(h.handshake()))?;
            self.extra.handshake = false;
        }
        if self.extra.restart {
            cw.write(&Message::Restart)?;
            self.extra.restart = false;
        }
        if let Some(nonce) = self.extra.pong {
            cw.write(&Message::Pong(Pong { nonce, delay: 0 }))?;
            self.extra.pong = None;
        }
        if self.extra.ask {
            cw.write(&Message::Announce(Announce::ASK))?;
            self.extra.ask = false;
        }
        while let Some(&k) = self.extra.order.front() {
            if let Some(&m) = self.extra.pending.get(&k) {
                let (_, stream) = k;
                let msg = match m {
                    StreamMessage::Request(bin) => {
                        Message::StreamRequest(StreamRequest { stream, bin })
                    }
                    StreamMessage::Cancel => Message::StreamCancel(StreamCancel { stream }),
                    StreamMessage::Open(bin) => Message::StreamOpen(StreamOpen { stream, bin }),
                    StreamMessage::Close => Message::StreamClose(StreamClose { stream }),
                };
                cw.write(&msg)?;
                self.extra.pending.remove(&k);
            }
            self.extra.order.pop_front();
        }

        write_map_bins(&mut self.have_bins, cw, |bin| Message::Have(Have { bin }))?;
        write_map_bins(&mut self.cancel_bins, cw, |bin| {
            Message::Cancel(Cancel { bin })
        })?;
        Ok(())
    }

    /// The first bin worth requesting from the peer: the start of its live
    /// window, as its newest HAVE implies, moved on by the chunks the
    /// stream adds before a request sent now is answered.
    ///
    /// The floor is independent of our own position. A request for a chunk
    /// the peer has evicted by the time it arrives is never answered: it
    /// times out as a loss (the window drops to its minimum) and the chunk
    /// is asked for first again, so a channel that asked behind the peer's
    /// window would never catch up.
    ///
    /// The lead is the stream rate (`rate`, chunks per second) times the
    /// request timeout plus two ticks (the HAVE and the request may each
    /// wait for one), plus one signature unit (chunks are announced a
    /// signature unit at a time), and at most half the peer's window.
    fn request_floor(&self, s: &SwarmState, rate: u64) -> Bin {
        let window = self.peer_live_window.0 / 2;
        let head = self.peer_max_have_bin.base_offset().saturating_add(1);
        if self.peer_max_have_bin.is_none() || head <= window {
            return Bin(0);
        }
        let wait = (self.controller.answer_delay() + TICK_INTERVAL * 2)
            .as_nanos()
            .max(0) as u128;
        let lead = u128::from(rate) * wait / Duration::SECOND.as_nanos() as u128;
        let lead = u64::try_from(lead)
            .unwrap_or(u64::MAX)
            .saturating_add(1 << s.signature_layer)
            .min(window / 2);
        Bin(head
            .saturating_add(lead)
            .saturating_sub(window)
            .saturating_mul(2))
    }

    /// Writes requests: as many as the request window, the frame and
    /// [`max_pending_requests`] allow, from [`Self::request_floor`] on, by
    /// request group.
    fn write1(
        &mut self,
        s: &mut SwarmState,
        now: Time,
        cw: &mut MeteredWriter<'_>,
    ) -> Result<(), codec::Error> {
        if self.choked {
            return Ok(());
        }

        let rate = s.peer_have_chunk_rate.rate(Duration::SECOND, now);
        let mut min = self.request_floor(s, rate);
        // Until the first chunk sets the read position, request nothing
        // before the peer's newest chunk: a low latency reader starts there.
        if !s.first_chunk_set && self.peer_max_have_bin > min {
            min = self.peer_max_have_bin;
        }

        self.candidate_bins.clear();

        let chunk = s.chunk_size.max(1) as i64;
        self.controller
            .on_round(now, rate.saturating_mul(s.chunk_size as u64));
        let timeout = now + self.controller.request_timeout();
        let budget = (self.controller.in_flight_cap() - self.controller.in_flight()).max(0) / chunk;
        let room = max_pending_requests(s.window).saturating_sub(self.request_bins.len());
        // A request is at least `MIN_REQUEST_LEN` bytes: no more than this
        // many fit the frame.
        let fit = cw.available() / MIN_REQUEST_LEN;
        let n = usize::try_from(budget)
            .unwrap_or(usize::MAX)
            .min(room)
            .min(fit);
        if n == 0 {
            return Ok(());
        }
        if now >= self.backoff_until && !self.backoff_bins.is_empty() {
            let root = self.backoff_bins.root_bin();
            self.backoff_bins.reset(root);
        }

        // Collect candidates up to the end of the request group of the
        // `n`th: this node's order within a group needs the whole group.
        let mut it = IntersectionIter::new(
            s.request_bins
                .iterate_empty_at(self.peer_have_bins.root_bin()),
            self.peer_have_bins.iterate_filled(),
        );
        let mut next = it.next_after(min);
        'walk: while let Some(x) = next {
            let (mut b, br) = x.base();
            while b <= br {
                if !self.backoff_bins.filled_at(b) {
                    let v = &self.candidate_bins;
                    let open = v.len() < n
                        || v.get(n - 1)
                            .is_some_and(|&nth| request_group(nth) == request_group(b));
                    if !open {
                        break 'walk;
                    }
                    self.candidate_bins.push(b);
                }
                b += 2;
            }
            next = it.next();
        }

        // Every node that hears of a chunk from the same peer at the same
        // time would ask that peer for the same chunks in the same order;
        // a peer that can't send to all of them then spends its upload on
        // copies of a few chunks, and nobody has anything to relay. In a
        // node's own order within each group, they ask for different ones.
        if self.candidate_bins.len() > n {
            let order = &s.request_order;
            self.candidate_bins
                .sort_unstable_by_key(|&b| request_key(order, b));
        }

        let size = (s.chunk_size as i64).max(1);
        for &b in self.candidate_bins.iter().take(n) {
            cw.write(&Message::Request(Request {
                bin: b,
                timestamp: now,
            }))?;
            s.request_bins.set(b);
            self.request_times.set(b, now);
            self.request_bins.push(b, timeout);
            self.controller
                .on_sent(now, (b.base_length() as i64).saturating_mul(size));
        }
        self.candidate_bins.clear();
        self.candidate_bins.shrink_to(CANDIDATE_CAPACITY);
        Ok(())
    }
}

/// Writes one message per maximal filled bin of `m`; on a full frame
/// the bins written are cleared and the rest stay.
fn write_map_bins(
    m: &mut Map,
    cw: &mut MeteredWriter<'_>,
    f: impl Fn(Bin) -> Message<'static>,
) -> Result<(), codec::Error> {
    if m.is_empty() {
        return Ok(());
    }
    let mut failed = None;
    for b in m.iterate_filled() {
        if let Err(e) = cw.write(&f(b)) {
            failed = Some((b, e));
            break;
        }
    }
    if let Some((b, e)) = failed {
        m.reset_before(b);
        return Err(e);
    }
    let root = m.root_bin();
    m.reset(root);
    Ok(())
}

/// A swarm's scheduler: the shared state and a channel scheduler per peer.
#[derive(Debug)]
pub(crate) struct PeerSwarmScheduler {
    pub(crate) s: SwarmState,
    /// Keyed by peer: one channel per peer and swarm.
    pub(crate) channels: IndexMap<PeerKey, PeerChannelScheduler>,
}

/// Fisher-Yates with a seeded rng.
fn shuffle<T>(v: &mut [T], rng: &mut DetRng) {
    use rand_core::Rng;
    for i in (1..v.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
}

fn rand_millis(rng: &mut DetRng, n: u64) -> Duration {
    use rand_core::Rng;
    Duration::from_millis((rng.next_u64() % n) as i64)
}

impl PeerSwarmScheduler {
    /// A scheduler for `swarm`; `rng` draws the request order and the timer
    /// phases.
    pub(crate) fn new(
        swarm: &Swarm,
        key: SwarmKey,
        rng: &mut DetRng,
        now: Time,
    ) -> PeerSwarmScheduler {
        let o = swarm.options();
        let signature_layer = if o.integrity.protection_method == ProtectionMethod::MERKLE_TREE {
            u64::from(o.chunks_per_signature.trailing_zeros())
        } else {
            0
        };
        let stream_count = u16::try_from(o.stream_count).unwrap_or(u16::MAX).max(1);
        let have_bins = swarm.store().bins();
        let mut request_order: Vec<u8> = (0..REQUEST_GROUP as u8).collect();
        shuffle(&mut request_order, rng);
        let window = o.live_window as u64;

        PeerSwarmScheduler {
            s: SwarmState {
                key,
                stream_count,
                stream_bits: u64::from(stream_count) - 1,
                signature_layer,
                bin_times: TimeSet::default(),
                bin_times_max: Bin::NONE,
                peer_have_chunk_rate: Sma::new(15, Duration::SECOND, now),
                peer_max_have_bin: Bin(0),
                request_bins: have_bins.clone(),
                source_bins: Map::new(),
                have_bins,
                have_bin_max: Bin(0),
                wasted_chunks: 0,
                chunks_sent: 0,
                pushed_chunks: 0,
                sent_bins: Map::new(),
                distinct_chunks_sent: 0,
                streams: vec![PeerSchedulerStream::default(); usize::from(stream_count)],
                stream_sources: 0,
                chunk_size: o.chunk_size,
                live_window: Bin(window * 2),
                window,
                delivery_mode: o.delivery_mode,
                first_chunk_set: o.scheduling_method == SchedulingMethod::SEED
                    || !swarm.store().is_empty(),
                next_gc_time: now + rand_millis(rng, 5000),
                request_order,
                mode: o.push_mode,
                controller: o.request_controller.resolve(),
                is_seed: o.scheduling_method == SchedulingMethod::SEED,
                seed: o.seed,
                push: (o.push_mode == PushMode::Hybrid).then(|| {
                    let report_at =
                        now + rand_millis(rng, push::REPORT_INTERVAL.as_millis() as u64);
                    PushState::new(usize::from(stream_count), now, report_at)
                }),
                tap: Tap::default(),
                subscribers_changed: false,
            },
            channels: IndexMap::new(),
        }
    }

    /// The tick: GC, request timeouts, restarts, the hybrid's parent
    /// checks, and the first read position.
    pub(crate) fn run(&mut self, now: Time, swarm: &mut Swarm, q: &mut impl PeerTaskQueue) {
        if now > self.s.next_gc_time {
            self.s.next_gc_time = now + SCHEDULER_GC_INTERVAL;
            self.gc(swarm, now);
        }

        let mut timed_out = false;
        for c in self.channels.values_mut() {
            timed_out |= c.time_out_requests(&mut self.s, now, q);
            c.try_restart(&self.s, now, q);
        }
        self.run_push(now, q);
        if timed_out {
            for &peer in self.channels.keys() {
                q.enqueue(peer, self.s.key);
            }
        }

        if !self.s.first_chunk_set && !self.s.have_bins.is_empty() {
            self.s.first_chunk_set = true;

            let mut b = Bin(0);
            match self.s.delivery_mode {
                DeliveryMode::BEST_EFFORT => {
                    let half_window = Bin(self.s.live_window.0 / 4 * 2);
                    if half_window < self.s.have_bin_max {
                        b = self.s.have_bin_max - half_window.0;
                    }
                }
                DeliveryMode::LOW_LATENCY => b = self.s.have_bin_max,
                _ => {}
            }
            self.s.request_bins.fill_before(b);
            swarm.set_offset(b);

            for &peer in self.channels.keys() {
                q.enqueue(peer, self.s.key);
            }
        }
    }

    /// Prunes bin times, the request map, the hybrid's maps and request
    /// times behind the window.
    fn gc(&mut self, swarm: &Swarm, now: Time) {
        if self.s.live_window < self.s.have_bin_max {
            self.s
                .bin_times
                .prune(self.s.have_bin_max - self.s.live_window.0);
        }

        let store = swarm.store();
        self.s.request_bins.fill_before(store.window_start());
        self.s.source_bins.reset_before(store.window_start());
        self.s.sent_bins.reset_before(store.window_start());
        self.gc_push(store.window_start());

        // Request times go at the read position or the window start,
        // whichever is later: none before the read position is set (answers
        // still need them to count), and a relay nobody reads from still
        // prunes at its window. A request time left before `next` is a
        // request the peer never answered for a chunk the store got
        // elsewhere: it leaves the flight here, since its deadline finds the
        // chunk and no request time, and settles nothing. Missing chunks
        // behind the window time out at their deadlines.
        let next = store.next();
        if next.is_none() {
            return;
        }
        let threshold = next.max(store.window_start());
        let size = self.s.chunk_size as i64;
        for c in self.channels.values_mut() {
            let settled = c.request_times.take_before(next);
            if settled > 0 {
                c.controller
                    .on_settled(now, (settled as i64).saturating_mul(size));
            }
            c.request_times.prune(threshold);
        }
    }

    /// The stream restarted (a new epoch): forget it; every channel starts
    /// over with a handshake.
    pub(crate) fn reset(&mut self, now: Time, q: &mut impl PeerTaskQueue) {
        let s = &mut self.s;
        s.bin_times = TimeSet::default();
        s.bin_times_max = Bin::NONE;
        s.peer_have_chunk_rate = Sma::new(15, Duration::SECOND, now);
        s.have_bins = Map::new();
        s.have_bin_max = Bin(0);
        s.request_bins = Map::new();
        s.source_bins = Map::new();
        s.sent_bins = Map::new();
        s.streams = vec![PeerSchedulerStream::default(); usize::from(s.stream_count)];
        s.subscribers_changed = true;
        s.stream_sources = 0;
        s.first_chunk_set = false;
        // The new stream starts at bin 0; see `PeerChannelScheduler::reset`.
        s.peer_max_have_bin = Bin(0);

        for c in self.channels.values_mut() {
            c.reset(s, now);
            q.enqueue_now(c.peer, s.key);
        }
        self.reset_push(now);
    }

    /// Chunks were stored: push them to subscribers, queue their HAVEs and
    /// mark them requested.
    pub(crate) fn consume(
        &mut self,
        bin: Bin,
        now: Time,
        swarm: &Swarm,
        q: &mut impl PeerTaskQueue,
    ) {
        let s = &mut self.s;
        s.set_bin_time(bin, now);

        // Hybrid pushes carry the time they were queued, so stale ones can
        // be dropped.
        let queued = if s.push.is_some() { now } else { Time::EPOCH };
        let mut wasted = bin.base_length();
        {
            let mut it = s.have_bins.iterate_empty_at(bin);
            while let Some(x) = it.next_base() {
                wasted = wasted.saturating_sub(1);
                if let Some(stream) = s.streams.get(usize::from(s.bin_stream(x))) {
                    for sub in &stream.subscribers {
                        if sub.start_bin <= x
                            && !q.push_data(sub.channel, s.key, x, queued, PeerPriority::High)
                        {
                            s.tap
                                .emit(s.key, Some(sub.channel), || TapKind::DataRefused {
                                    bin: x,
                                    push: true,
                                });
                        }
                    }
                }
            }
        }
        s.wasted_chunks = s.wasted_chunks.saturating_add(wasted);
        s.push_consumed(bin, bin.base_length().saturating_sub(wasted), now);
        s.source_bins.reset(bin);

        s.have_bins.set(bin);
        // Keep one window of history behind the store's window: the HAVEs
        // we send never claim more.
        let prune = Bin(swarm
            .store()
            .window_start()
            .0
            .saturating_sub(s.live_window.0));
        s.have_bins.reset_before(prune);
        let mut hb = s.have_bins.cover(bin);
        if hb.is_all() {
            hb = s.have_bins.root_bin();
        }
        for c in self.channels.values_mut() {
            c.have_bins.set(hb);
            // Eager modes write the HAVEs (and the pushes) at once, `Off` at
            // the next tick. Every chunk of the frame being handled lands in
            // `have_bins` before the driver polls, so this is one write cycle
            // per frame, not per chunk.
            if s.mode == PushMode::Off {
                q.enqueue(c.peer, s.key);
            } else {
                q.enqueue_now(c.peer, s.key);
            }
        }

        let br = bin.base_right();
        if br > s.have_bin_max {
            s.have_bin_max = br;
        }

        s.request_bins.set(bin);
    }

    /// Adds the peer's channel; `id` names the peer in reports.
    pub(crate) fn add_channel(
        &mut self,
        peer: PeerKey,
        id: &[u8],
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) {
        let c = PeerChannelScheduler::new(peer, id, &self.s, now);
        self.channels.insert(peer, c);
        q.enqueue(peer, self.s.key);
    }

    /// Removes the peer's channel: subscriptions both ways end and its
    /// outstanding requests go back to the other channels.
    pub(crate) fn close_channel(
        &mut self,
        peer: PeerKey,
        swarm: &Swarm,
        q: &mut impl PeerTaskQueue,
    ) {
        let Some(c) = self.channels.get_mut(&peer) else {
            return;
        };
        let s = &mut self.s;
        let window_start = swarm.store().window_start();
        let hybrid = s.push.is_some();
        for i in 0..s.stream_count {
            let Some(st) = s.streams.get_mut(usize::from(i)) else {
                continue;
            };
            let removed = st.remove_subscriber(peer);
            if !hybrid && st.source.is_some_and(|src| src.channel == peer) {
                c.close_stream(s, i, window_start);
            }
            s.subscribers_changed |= removed;
        }
        c.clear_requests(s);
        // Hybrid: subscriptions to the peer end; what it was expected to
        // push goes to pull.
        self.push_close_channel(peer, q);
        self.channels.shift_remove(&peer);
        q.remove_runner(peer, self.s.key);
    }

    /// Subscribes to `stream` from `peer`, starting at `start_bin`
    /// (`Runner::request_stream`). Outside the hybrid the peer becomes the
    /// stream's source at once, and the stream's bins up to the highest
    /// HAVE, within the tracked HAVE history, are left to it; chunks
    /// requested from other channels stay theirs.
    ///
    /// Refused until the peer's handshake arrived: a STREAM_OPEN before it
    /// is [`StreamOpenOutcome::BeforeHandshake`], so the request would go
    /// unanswered.
    pub(crate) fn do_stream_sub(&mut self, peer: PeerKey, stream: Stream, start_bin: Bin) -> bool {
        if !self.handshake_received(peer)
            || self.s.check_stream(stream).is_err()
            || start_bin >= MAX_BIN
        {
            return false;
        }
        if self.s.push.is_some() {
            // Hybrid: a request like any graft; chunks are expected (with
            // deadlines) only once the peer opens the stream.
            return self.push_subscribe(peer, stream, start_bin);
        }
        let Some(c) = self.channels.get_mut(&peer) else {
            return false;
        };
        let s = &mut self.s;
        s.set_stream_source(stream, peer, start_bin);
        c.add_stream_request(stream, start_bin);

        let so = stream_bin_offset(s.stream_count).0;
        let mut min = start_bin.0 / so * so + stream_bin_offset(stream).0;
        if min < start_bin.0 {
            min += so;
        }
        let lo = s.history_start(s.peer_max_have_bin).0;
        if min < lo {
            min += (lo - min).div_ceil(so) * so;
        }
        let mut b = min;
        while b <= s.peer_max_have_bin.0 {
            s.leave_to_source(Bin(b));
            b += so;
        }
        true
    }

    /// Unsubscribes from `stream` (`Runner::cancel_stream`).
    pub(crate) fn cancel_stream_sub(
        &mut self,
        peer: PeerKey,
        stream: Stream,
        swarm: &Swarm,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) -> bool {
        if !self.channels.contains_key(&peer) || self.s.check_stream(stream).is_err() {
            return false;
        }
        if self.s.push.is_some() {
            return self.push_unsubscribe(peer, stream, now, q);
        }
        let Some(c) = self.channels.get_mut(&peer) else {
            return false;
        };
        c.add_stream_cancel(&mut self.s, stream, swarm.store().window_start());
        true
    }

    fn channel(&mut self, peer: PeerKey) -> Option<(&mut SwarmState, &mut PeerChannelScheduler)> {
        let c = self.channels.get_mut(&peer)?;
        Some((&mut self.s, c))
    }

    /// The swarm's tap buffer.
    pub(crate) fn tap_mut(&mut self) -> &mut Tap {
        &mut self.s.tap
    }

    /// Whether `peer`'s handshake arrived on its channel.
    pub(crate) fn handshake_received(&self, peer: PeerKey) -> bool {
        self.channels
            .get(&peer)
            .is_some_and(|c| c.handshake_received)
    }

    /// Whether to take data from `peer`: any data, once its handshake
    /// arrived. Unrequested data is verified and stored like any other.
    pub(crate) fn expect_data(&self, peer: PeerKey, _b: Bin) -> bool {
        self.handshake_received(peer)
    }

    /// The peer's handshake arrived: requests and data may flow.
    pub(crate) fn handle_handshake(&mut self, peer: PeerKey, live_window: u32) {
        if let Some(c) = self.channels.get_mut(&peer) {
            c.peer_live_window = Bin(u64::from(live_window) * 2);
            c.handshake_received = true;
        }
    }

    /// The peer asks for our handshake again: resend it with HAVEs for
    /// everything we have.
    pub(crate) fn handle_restart(&mut self, peer: PeerKey) {
        if let Some((s, c)) = self.channel(peer) {
            c.extra.handshake = true;
            c.have_bins.clone_from(&s.have_bins);
        }
    }

    /// Data arrived from `peer`: `valid` says whether it verified, `new`
    /// whether it added chunks we didn't have. Invalid data releases its bin
    /// (inside our window only) to the other channels, which are queued, and
    /// this channel leaves it alone for [`REQUEST_BACKOFF`], so the peer
    /// that sent it can't keep it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_data(
        &mut self,
        peer: PeerKey,
        b: Bin,
        t: Time,
        valid: bool,
        new: bool,
        now: Time,
        window_start: Bin,
        q: &mut impl PeerTaskQueue,
    ) {
        if !self.channels.contains_key(&peer) {
            return;
        }
        if self.s.push.is_some() {
            if valid {
                self.push_data(peer, b, new, now, q);
            } else {
                self.push_invalid(peer, b);
            }
        }
        let Some((s, c)) = self.channel(peer) else {
            return;
        };
        if !valid {
            if crate::peer_bin_ok(b, s.window)
                && b.base_left() >= window_start
                && !s.have_bins.filled_at(b)
            {
                s.request_bins.reset(b);
                s.source_bins.reset(b);
                c.backoff_bins.set(b);
                c.backoff_until = now + REQUEST_BACKOFF;
                for &p in self.channels.keys() {
                    q.enqueue(p, self.s.key);
                }
            }
            return;
        }

        c.enqueue_now = true;
        if !c.handshake_received {
            return;
        }

        let size = (b.base_length() as i64).saturating_mul(s.chunk_size as i64);
        c.controller.on_delivered(now, size);
        if let Some((_, ts)) = c.request_times.get(b) {
            c.controller.on_answered(now, size, ts, t);
            // Answered: a second copy doesn't ack again, and the timeout
            // doesn't settle it.
            c.request_times.unset(b);
        }
    }

    /// Data for a chunk already behind our window: useless, but the peer
    /// delivered it, and it answers our request if we made one.
    pub(crate) fn handle_stale_data(&mut self, peer: PeerKey, b: Bin, t: Time, now: Time) {
        let Some((s, c)) = self.channel(peer) else {
            return;
        };
        s.tap
            .emit(s.key, Some(peer), || TapKind::DataStale { bin: b });
        if !c.handshake_received {
            return;
        }
        let size = (b.base_length() as i64).saturating_mul(s.chunk_size as i64);
        c.controller.on_delivered(now, size);
        if let Some((_, ts)) = c.request_times.get(b) {
            c.controller.on_answered(now, size, ts, t);
            c.request_times.unset(b);
        }
    }

    /// The peer announced `b`. Clipped to the tracked history: a HAVE costs
    /// at most [`HISTORY_WINDOWS`] windows of new bins. False if the bin was
    /// refused.
    pub(crate) fn handle_have(&mut self, peer: PeerKey, b: Bin, now: Time) -> bool {
        let Some((s, c)) = self.channel(peer) else {
            return true;
        };
        if !c.handshake_received {
            return true;
        }
        if b >= MAX_BIN {
            return false;
        }

        // A seed's bin times are its publish times: a HAVE can neither set
        // nor prune them.
        if !s.is_seed {
            s.set_bin_time(b, now);
        }

        let br = b.base_right();
        let new_max = c.peer_max_have_bin.max(br);
        let lo = s.history_start(new_max);
        let found;
        {
            let mut it = c.peer_have_bins.iterate_empty_at(b);
            let mut next = it.next_base_after(b.base_left().max(lo));
            found = next.is_some();
            // Only the hybrid's per-stream HAVE lag needs every new bin. A
            // seed samples a stream's last chunk of each signature unit
            // alone: a unit is published at once, so that chunk tells when
            // the peer had the unit's share of the stream whole.
            let unit = 1u64 << s.signature_layer;
            let last = unit.saturating_sub(u64::from(s.stream_count));
            if s.push.is_some() {
                while let Some(x) = next {
                    // A bin with no time (pruned, or never published by a
                    // seed) gives no sample.
                    let t = s.bin_time(x);
                    let unit_last = x.base_offset() % unit >= last;
                    if t != Time::EPOCH && (!s.is_seed || unit_last) {
                        let stream = usize::from(s.bin_stream(x));
                        Self::push_have_lag(&mut c.push, stream, now - t, now);
                    }
                    next = it.next_base();
                }
            }
        }
        // Eager modes request what the HAVE offers at once, not at the
        // next tick.
        if found && s.mode != PushMode::Off {
            c.enqueue_now = true;
        }

        c.peer_have_bins.set(b);
        if new_max > c.peer_max_have_bin || b.base_left() < lo {
            c.peer_max_have_bin = new_max;
            c.peer_have_bins.reset_before(lo);
        }

        if br > s.peer_max_have_bin {
            if s.peer_max_have_bin != Bin(0) {
                s.peer_have_chunk_rate.add(
                    br.base_offset()
                        .saturating_sub(s.peer_max_have_bin.base_offset()),
                    now,
                );
            }

            if s.push.is_some() {
                // Hybrid: expected by push, with a deadline that releases
                // them to pull.
                let from = s.peer_max_have_bin.max(s.history_start(br));
                s.mark_push_expected(from, br, now);
            } else if s.stream_sources > 0 {
                let mut x = s.peer_max_have_bin.max(s.history_start(br));
                while x <= br {
                    if let Some(src) = s
                        .streams
                        .get(usize::from(s.bin_stream(x)))
                        .and_then(|st| st.source)
                        && x >= src.start_bin
                    {
                        // Left to the source with no deadline, until it
                        // closes the stream or its channel.
                        s.leave_to_source(x);
                    }
                    x += 2;
                }
            }

            s.peer_max_have_bin = br;
        }
        true
    }

    /// The peer requested `b`: queue the answer at low priority. False if
    /// the bin was refused.
    pub(crate) fn handle_request(
        &mut self,
        peer: PeerKey,
        b: Bin,
        t: Time,
        q: &mut impl PeerTaskQueue,
    ) -> bool {
        let Some((s, c)) = self.channel(peer) else {
            return true;
        };
        if !c.handshake_received {
            return true;
        }
        if !crate::peer_bin_ok(b, s.window) {
            return false;
        }
        if !q.push_data(peer, s.key, b, t, PeerPriority::Low) {
            s.tap.emit(s.key, Some(peer), || TapKind::DataRefused {
                bin: b,
                push: false,
            });
        }
        c.enqueue_now = true;
        true
    }

    /// The peer cancelled `b`: drop the queued answer, and in hybrid mode
    /// any push of it (at write time, so one queued later is dropped too).
    pub(crate) fn handle_cancel(
        &mut self,
        peer: PeerKey,
        b: Bin,
        window_start: Bin,
        q: &mut impl PeerTaskQueue,
    ) {
        if self.channels.contains_key(&peer) {
            q.remove_data(peer, self.s.key, b, PeerPriority::Low);
            self.push_cancelled(peer, b, window_start);
        }
    }

    /// The peer (un)choked us; a choked channel requests nothing.
    pub(crate) fn handle_choke(&mut self, peer: PeerKey, choked: bool) {
        if let Some(c) = self.channels.get_mut(&peer) {
            c.choked = choked;
        }
    }

    /// Queues a pong; only the latest ping's is kept.
    pub(crate) fn handle_ping(&mut self, peer: PeerKey, nonce: u64) {
        if let Some(c) = self.channels.get_mut(&peer) {
            c.extra.pong = Some(nonce);
            c.enqueue_now = true;
        }
    }

    /// A pong counts once, and only for the outstanding ping: its round
    /// trip sets the request timeout, so a forged or repeated pong must not.
    /// Returns the round trip of any of the latest [`SENT_PINGS`] pings,
    /// once each, for the upload meter.
    pub(crate) fn handle_pong(&mut self, peer: PeerKey, nonce: u64, now: Time) -> Option<Duration> {
        let c = self.channels.get_mut(&peer)?;
        let rtt = now - Time::from_unix_nanos(nonce as i64);
        let sample = c
            .sent_pings
            .iter_mut()
            .find(|n| nonce != 0 && **n == nonce)
            .map(|n| {
                *n = 0;
                rtt
            });
        if c.ping_nonce != 0 && nonce == c.ping_nonce {
            c.ping_nonce = 0;
            c.controller.on_ping_rtt(now, rtt);
            if self.s.is_seed {
                push::seed::ping(&mut c.push, rtt, now);
            }
        }
        sample
    }

    /// The peer asks us to push `stream` from `b`. In hybrid mode the
    /// request first passes admission control and the Thicket constraint; a
    /// declined request is answered with STREAM_CLOSE.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_stream_request(
        &mut self,
        peer: PeerKey,
        stream: Stream,
        b: Bin,
        tail: Bin,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) -> Result<bool, SchedulerError> {
        let Some((s, c)) = self.channel(peer) else {
            return Ok(true);
        };
        if !c.handshake_received {
            return Ok(true);
        }
        s.check_stream(stream)?;
        if b >= MAX_BIN {
            return Ok(false);
        }
        let (ok, extra) = self.push_admit(peer, stream, now, q);
        if !ok {
            return Ok(true);
        }
        if let Some((s, c)) = self.channel(peer) {
            c.add_stream_open(s, stream, b, tail, extra, now, q);
            if s.push.is_some() {
                q.enqueue_now(peer, s.key);
            }
        }
        Ok(true)
    }

    /// The peer unsubscribes from `stream`.
    pub(crate) fn handle_stream_cancel(
        &mut self,
        peer: PeerKey,
        stream: Stream,
    ) -> Result<(), SchedulerError> {
        let Some((s, c)) = self.channel(peer) else {
            return Ok(());
        };
        if !c.handshake_received {
            return Ok(());
        }
        s.check_stream(stream)?;
        if let Some(st) = s.streams.get_mut(usize::from(stream)) {
            s.subscribers_changed |= st.remove_subscriber(peer);
        }
        Ok(())
    }

    /// The peer opened `stream` from `b`. Accepted only for a stream we
    /// requested from this peer: a source's chunks are left to it, so an
    /// unsolicited open could stall the stream. One open answering a request
    /// we cancelled before the answer arrived is
    /// [`StreamOpenOutcome::Stale`]: honest peers race there.
    pub(crate) fn handle_stream_open(
        &mut self,
        peer: PeerKey,
        stream: Stream,
        b: Bin,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) -> Result<StreamOpenOutcome, SchedulerError> {
        let Some((s, c)) = self.channel(peer) else {
            return Ok(StreamOpenOutcome::BeforeHandshake);
        };
        if !c.handshake_received {
            return Ok(StreamOpenOutcome::BeforeHandshake);
        }
        s.check_stream(stream)?;
        let asked = c
            .request_streams
            .get(usize::from(stream))
            .copied()
            .unwrap_or(Bin::NONE);
        // A parent with nothing stored yet opens at its tail, `NONE`: from
        // wherever its stream will start, which is where we asked.
        let b = if b.is_none() && !asked.is_none() {
            asked
        } else {
            b
        };
        if b >= MAX_BIN {
            return Ok(StreamOpenOutcome::Rejected);
        }
        let requested = !asked.is_none();
        let cancelled = c.answer_open_race(stream);
        if !requested {
            return Ok(if cancelled {
                StreamOpenOutcome::Stale
            } else {
                StreamOpenOutcome::Rejected
            });
        }
        // Hybrid: an open far past what we asked for would never be
        // expected to deliver anything, so it could hold the stream
        // forever; it counts as a decline.
        if !self.push_open_acceptable(peer, stream, b, now, q) {
            return Ok(StreamOpenOutcome::Rejected);
        }
        if let Some(r) = self
            .channels
            .get_mut(&peer)
            .and_then(|c| c.request_streams.get_mut(usize::from(stream)))
        {
            *r = b;
        }
        self.s.set_stream_source(stream, peer, b);
        self.push_open(peer, stream, b, now, q);
        Ok(StreamOpenOutcome::Accepted)
    }

    /// The peer stopped (or declined) pushing `stream`.
    pub(crate) fn handle_stream_close(
        &mut self,
        peer: PeerKey,
        stream: Stream,
        window_start: Bin,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) -> Result<(), SchedulerError> {
        let Some((s, c)) = self.channel(peer) else {
            return Ok(());
        };
        if !c.handshake_received {
            return Ok(());
        }
        s.check_stream(stream)?;
        c.answer_open_race(stream);
        if s.push.is_some() {
            // A declined request or a shed subscription. Only what this
            // parent was expected to push goes to pull: chunks requested
            // from other channels stay theirs.
            self.push_close(peer, stream, now, q);
        } else {
            // Only the source's chunks go to pull: chunks requested from
            // other channels stay theirs.
            c.close_stream(s, stream, window_start);
        }
        Ok(())
    }

    /// The peer's frame was handled: queue the channel's write cycle, at
    /// once if anything in the frame asked for it.
    pub(crate) fn handle_message_end(
        &mut self,
        peer: PeerKey,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) {
        let key = self.s.key;
        if let Some(c) = self.channels.get_mut(&peer) {
            if core::mem::take(&mut c.enqueue_now) {
                q.enqueue_now(peer, key);
            } else {
                q.enqueue(peer, key);
            }
            c.controller.on_frame_end(now);
        }
    }

    /// Writes a ping when due, then
    /// [`write0`](PeerChannelScheduler::write0), requests and the report
    /// due to the peer. A full frame returns
    /// [`codec::Error::NotEnoughSpace`] with what fit left in `cw`.
    pub(crate) fn write(
        &mut self,
        peer: PeerKey,
        now: Time,
        swarm: &Swarm,
        cw: &mut MeteredWriter<'_>,
    ) -> Result<(), codec::Error> {
        let Some((s, c)) = self.channel(peer) else {
            return Ok(());
        };
        if now > c.next_ping_time {
            let nonce = now.unix_nanos() as u64;
            cw.write(&Message::Ping(Ping { nonce }))?;
            c.next_ping_time = now + PING_INTERVAL;
            c.ping_nonce = nonce;
            c.sent_pings.rotate_left(1);
            if let Some(last) = c.sent_pings.last_mut() {
                *last = nonce;
            }
        }
        c.write0(swarm, cw)?;
        c.write1(s, now, cw)?;
        c.write_report(cw)
    }

    /// Writes integrity and data for `b`, or for the leftmost part of `b`
    /// that fits, putting the rest back at the front of the queue.
    /// `frame_empty` says nothing else is in the frame: a chunk that doesn't
    /// fit then never will, and is dropped.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn write_data(
        &mut self,
        peer: PeerKey,
        b: Bin,
        t: Time,
        pri: PeerPriority,
        now: Time,
        swarm: &Swarm,
        cw: &mut MeteredWriter<'_>,
        frame_empty: bool,
        q: &mut impl PeerTaskQueue,
    ) -> Result<(), WriteDataError> {
        let push = pri == PeerPriority::High;
        if push && self.push_unwanted(peer, b, t, now) {
            return Err(WriteDataError::Dropped);
        }
        let Some((s, c)) = self.channel(peer) else {
            return Err(WriteDataError::Dropped);
        };
        let mut b = b;
        loop {
            // The chunks are a lower bound on what `b` takes. The integrity
            // depends on what the peer has, so writing it is what tells
            // whether `b` fits; a write that runs out of space is discarded.
            let chunks = (b.base_length() as usize).saturating_mul(s.chunk_size);
            if chunks <= cw.available() {
                match write_bin(swarm, b, &c.peer_have_bins, now, cw) {
                    Ok(()) => {
                        s.count_sent(b, pri);
                        s.tap.emit(s.key, Some(peer), || TapKind::DataSent {
                            bin: b,
                            push,
                            queued: t,
                        });
                        break;
                    }
                    Err(WriteDataError::NotEnoughSpace) => cw.reset(),
                    Err(e) => {
                        s.tap
                            .emit(s.key, Some(peer), || TapKind::DataDropped { bin: b, push });
                        return Err(e);
                    }
                }
            }
            if b.is_base() {
                if frame_empty {
                    s.tap
                        .emit(s.key, Some(peer), || TapKind::DataDropped { bin: b, push });
                    return Err(WriteDataError::Dropped);
                }
                q.push_front_data(peer, s.key, b, t, pri);
                return Err(WriteDataError::NotEnoughSpace);
            }
            q.push_front_data(peer, s.key, b.right(), t, pri);
            b = b.left();
        }
        if push {
            self.note_push(peer, b, t, push::seed::PushFate::Written, now);
        }
        Ok(())
    }

    /// Diagnostics for one channel.
    pub(crate) fn channel_stats(&self, peer: PeerKey) -> Option<ChannelSchedulerStats> {
        let c = self.channels.get(&peer)?;
        Some(ChannelSchedulerStats {
            handshake_received: c.handshake_received,
            choked: c.choked,
            controller: c.controller.kind(),
            cwnd: c.controller.in_flight_cap(),
            flight_size: c.controller.in_flight(),
            cto: c.controller.request_timeout(),
            rtt_mean: c.controller.rtt(),
            min_rtt: c.controller.min_rtt(),
            delivery_rate: c.controller.delivery_rate(),
            pending_requests: c.request_bins.len(),
            request_time_nodes: c.request_times.size(),
            extra_messages: c.extra.len(),
            peer_max_have_bin: c.peer_max_have_bin,
            peer_have_map_size: map_size(&c.peer_have_bins),
            have_map_size: map_size(&c.have_bins),
            cancel_map_size: map_size(&c.cancel_bins),
        })
    }

    /// Diagnostics for the swarm.
    pub(crate) fn stats(&self) -> SwarmSchedulerStats {
        let s = &self.s;
        SwarmSchedulerStats {
            first_chunk_set: s.first_chunk_set,
            have_bin_max: s.have_bin_max,
            peer_max_have_bin: s.peer_max_have_bin,
            wasted_chunks: s.wasted_chunks,
            bin_time_nodes: s.bin_times.size(),
            have_map_size: map_size(&s.have_bins),
            request_map_size: map_size(&s.request_bins),
            stream_sources: s.stream_sources,
            subscribers: s.streams.iter().map(|st| st.subscribers.len()).sum(),
            chunks_sent: s.chunks_sent,
            pushed_chunks: s.pushed_chunks,
            distinct_chunks_sent: s.distinct_chunks_sent,
        }
    }
}

/// Writes the integrity for a peer that has `peer_have`, then the data, for
/// `b`. `NotEnoughSpace` leaves part of it in `cw`.
fn write_bin(
    swarm: &Swarm,
    b: Bin,
    peer_have: &Map,
    now: Time,
    cw: &mut MeteredWriter<'_>,
) -> Result<(), WriteDataError> {
    match swarm.write_integrity(b, peer_have, cw) {
        Ok(_) => {}
        Err(e) if e.is_not_enough_space() => return Err(WriteDataError::NotEnoughSpace),
        Err(_) => return Err(WriteDataError::Dropped),
    }
    match swarm.write_data(b, now, cw) {
        Ok(_) => Ok(()),
        Err(store::Error::Codec(codec::Error::NotEnoughSpace)) => {
            Err(WriteDataError::NotEnoughSpace)
        }
        Err(_) => Err(WriteDataError::Dropped),
    }
}

/// A proxy for a binmap's memory: the length of its debug dump, which has a
/// line per cell.
fn map_size(m: &Map) -> usize {
    m.debug_string().lines().count()
}

/// Scheduler state of one channel, for diagnostics and tests.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelSchedulerStats {
    pub handshake_received: bool,
    pub choked: bool,
    /// The request window's model.
    pub controller: RequestControllerKind,
    /// The request window (in-flight cap) in bytes: LEDBAT's congestion
    /// window, or `BbrLike`'s cap.
    pub cwnd: i64,
    /// Bytes requested and not yet answered.
    pub flight_size: i64,
    /// The request timeout (LEDBAT's congestion timeout).
    pub cto: Duration,
    /// The smoothed round trip: LEDBAT's ping round trip, `BbrLike`'s
    /// request-to-data round trip.
    pub rtt_mean: Duration,
    /// `BbrLike`'s minimum round trip (zero for LEDBAT).
    pub min_rtt: Duration,
    /// `BbrLike`'s delivery-rate estimate, bytes per second (0 for LEDBAT).
    pub delivery_rate: u64,
    /// Outstanding request deadlines.
    pub pending_requests: usize,
    pub request_time_nodes: usize,
    pub extra_messages: usize,
    pub peer_max_have_bin: Bin,
    /// Lines in the binmaps' debug dumps, roughly their cells.
    pub peer_have_map_size: usize,
    pub have_map_size: usize,
    pub cancel_map_size: usize,
}

/// Scheduler state of a swarm, for diagnostics and tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwarmSchedulerStats {
    pub first_chunk_set: bool,
    pub have_bin_max: Bin,
    pub peer_max_have_bin: Bin,
    /// Chunks received that we already had.
    pub wasted_chunks: u64,
    pub bin_time_nodes: usize,
    pub have_map_size: usize,
    pub request_map_size: usize,
    pub stream_sources: usize,
    pub subscribers: usize,
    /// Chunks written to peers: pushes and request answers.
    pub chunks_sent: u64,
    /// Of `chunks_sent`, pushes to subscribers.
    pub pushed_chunks: u64,
    /// Chunks written to anyone at least once (a seed's count of distinct
    /// chunks uploaded).
    pub distinct_chunks_sent: u64,
}

#[cfg(test)]
#[path = "scheduler_peer_test.rs"]
pub(crate) mod tests;
