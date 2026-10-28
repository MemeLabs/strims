// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The push/pull hybrid ([`PushMode::Hybrid`]): per-stream push trees
//! over the pull scheduler, after Plumtree and Thicket.
//!
//! Pull stays the backbone and the repair path. A parent can cost a child
//! time but never data: a dead, slow, overloaded or malicious parent delays
//! the chunks it was expected to push by at most [`DEADLINE_MAX`] before
//! pull takes them back, and its misses remove it. Chunk `i` belongs to
//! stream `i % stream_count`, so each stream is a 1/`stream_count` slice of
//! the rate with a tree of its own.
//!
//! # Child side
//!
//! A subscription to a *parent* (STREAM_REQUEST, answered by STREAM_OPEN)
//! is a standing request for every later chunk of the stream, held under a
//! lease of per-chunk deadlines.
//!
//! - **Expected chunks.** When anyone announces past the newest announced
//!   chunk, the chunks of streams with an open parent that we neither have
//!   nor requested, from the parent's start on, are marked requested (so
//!   pull leaves them alone) with a deadline. A chunk whose deadline passes
//!   goes back to pull once pull is expected to win (below), counts as a
//!   miss against the parent, and is cancelled at the parent (CANCEL),
//!   which then drops its push if it hasn't sent it. Pull leaves it to the
//!   other channels for `REQUEST_BACKOFF`: the parent's answer would queue
//!   behind its pushes.
//! - **Late pushes.** The parent may still push a chunk until
//!   [`DEADLINE_MAX`] past its first announcement, and a repair that comes
//!   after the push only adds load. So while no other holder is expected
//!   to answer before then (by its mean answer time,
//!   [`crate::route::AnswerTime`]; a holder not yet timed counts as quick),
//!   a chunk past its deadline stays expected, looked at again every
//!   [`RELEASE_RECHECK`].
//! - **Deadlines** run from when anyone first announced the chunk, and come
//!   from the parent's push delay (see [`DEADLINE_K`] to
//!   [`DEADLINE_GRANULARITY`]). Pushes leave in munro bursts, so the delay
//!   is a sawtooth whose top a mean and deviation underestimate: the
//!   estimate tracks a decaying peak. A new parent's estimate starts from
//!   its announcement lag, so a parent deep in the tree doesn't miss its
//!   whole first munro.
//! - **Graft.** Every [`PARENT_CHECK_INTERVAL`], once the read position is
//!   set, a stream without a parent asks the candidate with the lowest
//!   announcement lag (how long after the first announcement anywhere its
//!   HAVEs arrive), from the first chunk after everything we have or have
//!   seen announced. A candidate has a handshake, doesn't choke us, isn't
//!   backed off for the stream, isn't our own subscriber for it (no
//!   2-cycles), and announced it within [`CANDIDATE_FRESHNESS`]. The seed
//!   never subscribes: it has every chunk first.
//! - **Switch.** A candidate that beats the parent at [`SWITCH_CHECKS`]
//!   checks in a row, outside [`SWITCH_HOLD`] after the parent opened, is
//!   grafted; the old parent is cancelled only once the new one opens.
//!   Beating means announcing the stream sooner than the parent announces
//!   it ([`SWITCH_MARGIN`]), or sooner than the parent's pushes arrive
//!   ([`SLOW_PARENT_MARGIN`]). Announcements are compared with
//!   announcements: a parent's pushes queue behind its other children's,
//!   and a bystander's HAVEs don't.
//! - **Prune.** A parent that went silent ([`PRUNE_SILENCE`]) or whose miss
//!   rate is too high ([`PRUNE_MISS_RATE`]) is cancelled (STREAM_CANCEL).
//!   Misses in a row alone prune nothing: one lost frame carries a dozen
//!   pushes.
//! - **Release.** A prune, a shed (STREAM_CLOSE from the parent), an
//!   unsubscribe or a closed channel releases the stream's expected chunks
//!   to pull at once, cancels those due from the parent at it if it is
//!   still a neighbour (its queued pushes outlive the subscription) and
//!   leaves them to the other channels as a miss is, and gives every
//!   channel a write cycle. A switch keeps them expected: the old parent
//!   queued those pushes before our cancel, and the deadlines catch what
//!   doesn't come.
//! - **Bad answers.** A request answered by neither OPEN nor CLOSE within
//!   [`OPEN_TIMEOUT`] is cancelled. An OPEN more than a live window past the
//!   request would never be expected to deliver anything, so nothing could
//!   prune it: it counts as a decline. Pushes of a stream from a peer that
//!   isn't its parent trigger another STREAM_CANCEL, at most once per
//!   [`UNSOLICITED_CANCEL_INTERVAL`]. Invalid data from the parent is a
//!   miss, released at once. A peer that declined, shed or timed out is
//!   backed off for [`DECLINE_BACKOFF`]; one pruned or with an unacceptable
//!   OPEN for [`PRUNE_BACKOFF`].
//! - **Reports.** Per parent and stream we tally its hits, its late pushes
//!   and its misses, as the prune rules count them, and the hits' push
//!   delay. Every [`REPORT_INTERVAL`] the tallies go out as ANNOUNCE
//!   entries to each neighbour that asked for them (an empty ANNOUNCE)
//!   within [`REPORT_LEASE`], leaving out the entries about that
//!   neighbour, and start over. A parent is named by the id its driver
//!   gave it. Only an adaptive seed asks ([`seed`]).
//!
//! # Parent side
//!
//! - **Thicket.** A non-seed node is *interior* (forwards to children) for
//!   at most one stream: the first it accepts a subscriber for, until it has
//!   had none for [`INTERIOR_IDLE`]. The seed is interior for every stream.
//! - **Admission.** A STREAM_REQUEST is accepted while the projected upload
//!   (the larger of the measured rate and the committed push, plus one more
//!   subscriber) stays under [`INTERIOR_UTILIZATION`] of the measured
//!   capacity for the interior stream, or [`EXTRA_UTILIZATION`] for another.
//!   Before the upload is measured, [`UNMEASURED_FANOUT`] subscribers per
//!   interior stream and no others. A subscriber costs our store rate per
//!   stream times [`PUSH_OVERHEAD`]. A repeat from a subscriber is always
//!   accepted; a declined request is answered with STREAM_CLOSE.
//! - **Shedding.** While the uplink is busy [`SHED_UTILIZATION`] of the time,
//!   at most once per [`SHED_INTERVAL`], enough subscribers are closed to
//!   bring the committed push under the interior target: extra streams
//!   first, then the newest.
//! - **Pushes** carry the time they were queued. At write time a push that
//!   waited longer than [`PUSH_STALE`] is dropped (the child has released
//!   it), and so is one of a chunk the child has announced or cancelled
//!   since. A CANCEL covers a chunk whatever the child asked for it by: a
//!   child that cancelled a request doesn't want the chunk's push either. An
//!   adaptive seed still counts a cancelled push's wait in its queue.
//! - **Upload measurement** is the runner's, shared by its swarms: bytes
//!   written over the time any peer had data queued. A rarely busy link
//!   counts as busy at least 5 % of the time.
//!
//! # Seed allocation
//!
//! What a seed pushes to whom is its admission's choice: see [`seed`].
//!
//! # Interaction with pull and reciprocity
//!
//! - Pushes add nothing to the request window's flight and give it no
//!   delay or round-trip samples (a push has no request time); the
//!   controller sees them only as delivered bytes. Push load is bounded by
//!   admission, shedding, the child's deadlines and prunes, and stale drops;
//!   pull answers on the same link see push queueing in their delays.
//! - Pushes go before request answers: within a peer's queue, and across
//!   peers through the answer gate ([`crate::answer_gate`]), which writes
//!   no answer while a push waits on a link that drains. Otherwise the
//!   transport's weights decide between peers.
//! - Pushed data counts toward upload weights like pulled data (verified
//!   and new only), so a child weighs its parent highly. Children never send
//!   a parent the stream they get from it.
//!
//! # Bounds
//!
//! - Expected-chunk marking is clipped like the HAVE walk (the new chunks
//!   within the tracked history) and stops at a window of deadlines
//!   ([`max_pending_requests`]); chunks past it are left to pull.
//! - The expected and released chunk maps, and a channel's cancelled
//!   chunks, are bounded by the window and pruned at GC. A release walks
//!   only the expected map, once per event.
//! - Parent checks cost O(streams × channels) per [`PARENT_CHECK_INTERVAL`];
//!   shedding sorts the subscribers at most once per [`SHED_INTERVAL`].
//! - Per-stream state is fixed: one subscription per stream; one lag,
//!   backoff, cancel time and tally per channel and stream; one subscriber
//!   entry per channel and stream. A report due to a neighbour holds at
//!   most twice the stream count of entries.
//!
//! # Wire
//!
//! STREAM_REQUEST/OPEN/CANCEL/CLOSE are used with their v2 meanings
//! (CLOSE from a provider: not, or no longer, pushing this stream), so the
//! hybrid mixes with nodes in any mode, and ANNOUNCE carries reports to
//! whoever asked for them. CANCEL of a chunk covers its push as well as its
//! request. The child side assumes nothing more of a parent: one that never
//! declines or sheds, or keeps pushing after STREAM_CANCEL or CANCEL, is
//! contained by deadlines, silence pruning and repeated cancels.
//!
//! This is a child module of `scheduler_peer` so it can reach the
//! scheduler's private state; the hooks there call into it only in
//! `Hybrid`.

use strims_base::stats::Sma;
use strims_base::time::{Duration, Time};
use strims_binmap::{Bin, Cursor, Map};

use super::{
    PeerChannelScheduler, PeerSwarmScheduler, REQUEST_BACKOFF, StreamMessage, SwarmState,
    max_pending_requests,
};
use crate::bin_queue::BinQueue;
use crate::channel::MeteredWriter;
use crate::codec::{
    self, Announce, AnnounceEntry, MAX_ANNOUNCE_ENTRIES, MESSAGE_TYPE_LEN, Message, MessageWriter,
    Stream,
};
use crate::options::PushMode;
use crate::route::ROUTE_MARGIN;
use crate::runner::PeerKey;
use crate::scheduler::PeerTaskQueue;
use crate::tap::{
    Admission, AdmitRule, LagEstimate, ParentAction, ParentChoice, Prune, Shed, TapKind, Upload,
};

#[path = "scheduler_seed.rs"]
pub mod seed;

/// How often streams look for a parent, re-evaluate the one they have, and
/// time out unanswered requests.
pub const PARENT_CHECK_INTERVAL: Duration = Duration::from_millis(500);
/// A STREAM_REQUEST answered by neither STREAM_OPEN nor STREAM_CLOSE is
/// cancelled after this (a lost message, or a peer that ignores it).
pub const OPEN_TIMEOUT: Duration = Duration::SECOND;
/// A peer that declined, shed or never answered a stream isn't asked for
/// it again for this long.
pub const DECLINE_BACKOFF: Duration = Duration::from_secs(3);
/// A parent pruned for misses isn't asked for the stream again for this
/// long.
pub const PRUNE_BACKOFF: Duration = Duration::from_secs(10);
/// Deviations of the parent's push delay a deadline allows ...
pub const DEADLINE_K: f64 = 4.0;
/// ... or this times its recent peak, whichever is longer. Pushes leave in
/// bursts (a munro's chunks to every child), so the delay is a sawtooth
/// whose top a mean and deviation underestimate.
pub const DEADLINE_PEAK_FACTOR: f64 = 1.25;
/// The peak decays toward the mean over this long.
pub const PEAK_DECAY: Duration = Duration::from_secs(4);
/// Added to every deadline (and its minimum).
pub const DEADLINE_MARGIN: Duration = Duration::from_millis(50);
/// The longest deadline: past this, pull is the better bet. A deadline
/// runs from when the chunk was first announced by anyone, so it bounds how
/// far the parent may trail the fastest neighbour.
pub const DEADLINE_MAX: Duration = Duration::SECOND;
/// The deadline before a parent's first push, if nothing is known about the
/// peer; normally a new parent's delay estimate starts from how late it
/// announced the stream before.
pub const DEADLINE_INITIAL: Duration = Duration::from_millis(300);
/// Deadlines are rounded up to this, so a burst of chunks shares a timer.
pub const DEADLINE_GRANULARITY: Duration = Duration::from_millis(10);
/// A chunk past its deadline that no other holder is expected to deliver
/// before the parent's last chance ([`DEADLINE_MAX`]) looks for one again
/// this often.
pub const RELEASE_RECHECK: Duration = Duration::from_millis(100);
/// A peer's announcement lag stays plausible this many mean deviations
/// past its mean ([`crate::route`]).
pub const HORIZON_DEVS: f64 = 2.0;
/// A parent that delivered nothing for this long while at least
/// [`PRUNE_SILENT_MISSES`] chunks missed is pruned (dead, withholding, or
/// cut off).
pub const PRUNE_SILENCE: Duration = Duration::SECOND;
pub const PRUNE_SILENT_MISSES: u32 = 4;
/// The miss rate (an EWMA over expected chunks) that prunes a parent ...
pub const PRUNE_MISS_RATE: f64 = 0.25;
/// ... once it was expected to deliver this many chunks ...
pub const PRUNE_MIN_EXPECTED: u64 = 64;
/// ... and has been the parent this long (a munro is a burst of up to 64
/// misses for one stream).
pub const PRUNE_MIN_AGE: Duration = Duration::from_secs(2);
/// The miss rate EWMA's weight per expected chunk: slow enough that a lost
/// frame (a burst of a dozen misses) stays under the limit.
const MISS_ALPHA: f64 = 1.0 / 64.0;
/// A non-parent must announce a stream this much (or a quarter of the
/// parent's lag, if more) sooner than the parent announces it (it is closer
/// to the source) ...
pub const SWITCH_MARGIN: Duration = Duration::from_millis(50);
/// ... or this much sooner than the parent's pushes arrive (the parent is
/// slow) ...
pub const SLOW_PARENT_MARGIN: Duration = Duration::from_millis(250);
/// ... at this many parent checks in a row before we graft onto it ...
pub const SWITCH_CHECKS: u32 = 3;
/// ... and not within this long of the parent's STREAM_OPEN.
pub const SWITCH_HOLD: Duration = Duration::from_secs(5);
/// A candidate's announcement lag counts only if it announced the stream
/// this recently.
pub const CANDIDATE_FRESHNESS: Duration = Duration::from_secs(2);
/// A provider drops pushes that waited in its queue longer than this: the
/// child has released them to pull long since.
pub const PUSH_STALE: Duration = Duration::SECOND;
/// Admission: the projected upload utilization a node accepts a subscriber
/// of its interior stream at (the seed: any stream) ...
pub const INTERIOR_UTILIZATION: f64 = 0.7;
/// ... and a subscriber of another stream at (measured spare upload).
pub const EXTRA_UTILIZATION: f64 = 0.5;
/// Subscribers per interior stream a node accepts before it has measured
/// its upload.
pub const UNMEASURED_FANOUT: usize = 4;
/// What a subscriber costs relative to the stream's data rate (integrity
/// messages ride along with pushed chunks).
pub const PUSH_OVERHEAD: f64 = 1.25;
/// The busy fraction below which the upload measurement can't tell the
/// capacity apart from more: the capacity estimate uses at least this.
const MIN_BUSY: f64 = 0.05;
/// The busy fraction from which the measurement shows a saturated uplink:
/// below it a known capacity stands, since a link that isn't saturated
/// shows only a lower bound.
pub(crate) const SATURATED_BUSY: f64 = 0.5;
/// The busy fraction (time with data queued) at which a provider sheds
/// subscribers ...
pub const SHED_UTILIZATION: f64 = 0.9;
/// ... at most this often.
pub const SHED_INTERVAL: Duration = Duration::SECOND;
/// A non-seed node whose interior stream has had no subscribers this long
/// lets another stream claim it.
pub const INTERIOR_IDLE: Duration = Duration::from_secs(10);
/// Unsolicited pushes from a peer trigger a STREAM_CANCEL for their stream
/// at most this often.
pub const UNSOLICITED_CANCEL_INTERVAL: Duration = Duration::SECOND;
/// How often a node reports how its parents pushed to it.
pub const REPORT_INTERVAL: Duration = Duration::SECOND;
/// A node reports to a neighbour for this long after the neighbour asked.
pub const REPORT_LEASE: Duration = Duration::from_secs(10);

/// The node's upload as the runner measures it: what admission control and
/// shedding decide on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct UploadInfo {
    /// Data bytes written per second over the meter's window.
    pub(crate) rate: u64,
    /// The fraction of the window during which data was queued to send.
    pub(crate) busy: f64,
    /// The capacity the driver knows, in bytes per second.
    pub(crate) hint: Option<u64>,
    /// The push rate the node's other swarms commit (their subscriptions
    /// at their stream rates), in bytes per second: this swarm's
    /// admission, shedding and seed budget leave it room.
    pub(crate) others: f64,
    /// The queueing delay past the runner, from the peers' ping round
    /// trips; `None` with too few peers.
    pub(crate) queue_delay: Option<Duration>,
}

impl UploadInfo {
    /// The upload capacity in bytes per second. With the driver's hint, the
    /// hint, or less while the link is saturated (at least
    /// [`SATURATED_BUSY`]) at a lower rate. Without, the rate while busy: a
    /// link that is rarely busy has at least `rate / MIN_BUSY`. `None`
    /// before anything was sent and without a hint.
    fn capacity(&self) -> Option<f64> {
        let measured = (self.rate > 0).then(|| self.rate as f64 / self.busy.max(MIN_BUSY));
        match (measured, self.hint.map(|h| h as f64)) {
            (Some(m), Some(h)) if self.busy >= SATURATED_BUSY => Some(m.min(h)),
            (_, Some(h)) => Some(h),
            (m, None) => m,
        }
    }
}

/// An exponentially weighted delay estimate (TCP's RTO estimator: mean
/// with weight 1/8, mean deviation with weight 1/4), plus a peak that
/// decays linearly toward the mean over [`PEAK_DECAY`]. Only basic float
/// operations, so it is deterministic on every target.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct LagEwma {
    /// Nanoseconds.
    mean: f64,
    dev: f64,
    peak: f64,
    n: u32,
    last: Time,
}

impl LagEwma {
    fn update(&mut self, sample: Duration, now: Time) {
        let x = sample.as_nanos().max(0) as f64;
        if self.n == 0 {
            self.mean = x;
            self.dev = x / 2.0;
            self.peak = x;
        } else {
            let err = x - self.mean;
            self.mean += err / 8.0;
            self.dev += (err.abs() - self.dev) / 4.0;
            let dt = (now - self.last).as_nanos().max(0) as f64;
            let k = (dt / PEAK_DECAY.as_nanos() as f64).min(1.0);
            self.peak -= (self.peak - self.mean).max(0.0) * k;
            self.peak = self.peak.max(x);
        }
        self.n = self.n.saturating_add(1);
        self.last = now;
    }

    fn fresh(&self, now: Time) -> bool {
        self.n > 0 && now - self.last <= CANDIDATE_FRESHNESS
    }

    fn mean(&self) -> Duration {
        Duration::from_nanos(self.mean as i64)
    }

    /// How late a sample still fits the estimate: the decaying peak, or
    /// the mean plus [`HORIZON_DEVS`] deviations, whichever is later.
    fn horizon(&self) -> Duration {
        Duration::from_nanos(self.peak.max(self.mean + HORIZON_DEVS * self.dev) as i64)
    }

    /// How long a push from this parent may take.
    fn deadline(&self) -> Duration {
        if self.n == 0 {
            return DEADLINE_INITIAL;
        }
        let d = (self.mean + DEADLINE_K * self.dev).max(DEADLINE_PEAK_FACTOR * self.peak);
        (Duration::from_nanos(d as i64) + DEADLINE_MARGIN).clamp(DEADLINE_MARGIN, DEADLINE_MAX)
    }

    /// The estimate as the tap reports it.
    fn estimate(&self) -> LagEstimate {
        let d = |x: f64| Duration::from_nanos(x as i64);
        LagEstimate {
            mean: d(self.mean),
            dev: d(self.dev),
            peak: d(self.peak),
            deadline: self.deadline(),
        }
    }
}

/// A peer we asked for a stream, or receive it from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Parent {
    pub(crate) peer: PeerKey,
    start: Bin,
    /// `NIL` until the next tick when set without a clock
    /// (`Runner::request_stream`).
    since: Time,
    /// The peer's announcement lag when we asked: the first estimate of
    /// its push delay.
    hint: LagEwma,
}

/// The child side of one stream.
#[derive(Clone, Debug, Default)]
pub(crate) struct Subscription {
    /// The parent pushing the stream to us.
    pub(crate) open: Option<Parent>,
    /// A STREAM_REQUEST in flight: the first parent, or a faster one to
    /// switch to (graft before prune).
    pending: Option<Parent>,
    /// The open parent's push delay: arrival after the chunk was first
    /// seen anywhere. Starts from its announcement lag.
    lag: LagEwma,
    /// Pushes measured (`lag` starts with announcement samples).
    pushes: u32,
    miss_rate: f64,
    expected: u64,
    /// Misses since the last hit, and when that was (the open, at first).
    silent_misses: u32,
    last_hit: Time,
    switch_votes: u32,
}

impl Subscription {
    fn on_open(&mut self, p: Parent) {
        *self = Subscription {
            open: Some(p),
            last_hit: p.since,
            lag: p.hint,
            ..Default::default()
        };
    }

    fn on_hit(&mut self, now: Time) {
        self.silent_misses = 0;
        self.last_hit = now;
        self.miss_rate *= 1.0 - MISS_ALPHA;
    }

    fn on_miss(&mut self) {
        self.silent_misses = self.silent_misses.saturating_add(1);
        self.miss_rate = self.miss_rate * (1.0 - MISS_ALPHA) + MISS_ALPHA;
    }

    /// Whether the parent is due a prune: the rules that hold, silent and
    /// lossy, if either does.
    fn prune_rule(&self, now: Time) -> Option<(bool, bool)> {
        let open = self.open?;
        let silent = self.silent_misses >= PRUNE_SILENT_MISSES
            && !self.last_hit.is_nil()
            && now - self.last_hit >= PRUNE_SILENCE;
        let lossy = self.expected >= PRUNE_MIN_EXPECTED
            && self.miss_rate > PRUNE_MISS_RATE
            && !open.since.is_nil()
            && now - open.since >= PRUNE_MIN_AGE;
        (silent || lossy).then_some((silent, lossy))
    }

    /// A prune by `rule` of `parent` (already taken from `open`), as the
    /// tap reports it.
    fn prune_why(&self, (silent, lossy): (bool, bool), parent: &Parent, now: Time) -> Prune {
        let since = |t: Time| (!t.is_nil()).then(|| now - t);
        Prune {
            silent,
            lossy,
            silent_misses: self.silent_misses.into(),
            since_hit: since(self.last_hit),
            miss_rate_ppm: (self.miss_rate * 1e6) as u64,
            expected: self.expected,
            pushes: self.pushes.into(),
            age: since(parent.since),
            lag: self.lag.estimate(),
        }
    }
}

/// How a parent pushed one stream to us since the last report tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tally {
    hits: u64,
    late: u64,
    misses: u64,
    /// The hits' push delays, summed, in milliseconds.
    delay_ms: u64,
}

impl Tally {
    fn is_empty(&self) -> bool {
        self.hits == 0 && self.late == 0 && self.misses == 0
    }

    fn hit(&mut self, delay: Duration) {
        self.hits = self.hits.saturating_add(1);
        let ms = u64::try_from(delay.as_millis()).unwrap_or(0);
        self.delay_ms = self.delay_ms.saturating_add(ms);
    }

    fn entry<'a>(&self, stream: Stream, parent: &'a [u8]) -> AnnounceEntry<'a> {
        let ms = self.delay_ms.checked_div(self.hits).unwrap_or(0);
        AnnounceEntry {
            stream,
            hits: self.hits,
            late: self.late,
            misses: self.misses,
            delay: Duration::from_millis(i64::try_from(ms).unwrap_or(i64::MAX)),
            parent,
        }
    }
}

/// A report to one neighbour that asked: encoded ANNOUNCE entries, the end
/// of each, and how many are written.
#[derive(Clone, Debug, Default)]
pub(crate) struct Report {
    /// Bound: `2 × stream_count` entries.
    bytes: Vec<u8>,
    ends: Vec<usize>,
    sent: usize,
}

/// Per channel, per stream: what we know about the peer as a parent.
#[derive(Clone, Debug)]
pub(crate) struct PushChannel {
    /// How long after a chunk was first seen the peer announces it.
    /// Bound: one per stream.
    have_lag: Vec<LagEwma>,
    /// Not a candidate for the stream until then. Bound: one per stream.
    backoff: Vec<Time>,
    /// When we last cancelled the peer's unsolicited pushes of the stream.
    /// Bound: one per stream.
    cancel_at: Vec<Time>,
    /// Chunks the peer cancelled: their pushes are dropped at write time.
    /// Bound: the window ([`PeerSwarmScheduler::push_cancelled`] admits
    /// only bins within a live window of our newest, pruned at GC).
    cancelled: Map,
    /// How the peer pushed each stream to us since the last report tick.
    /// Bound: one per stream.
    tally: Vec<Tally>,
    /// When the peer last asked for our reports (`NIL`: never), and the
    /// report it is due.
    asked: Time,
    report: Report,
    /// An adaptive seed's record of the peer.
    holder: seed::Holder,
    /// An adaptive seed's tallies of the peer's reports.
    heard: seed::Heard,
}

impl PushChannel {
    pub(crate) fn new(streams: usize) -> PushChannel {
        PushChannel {
            have_lag: vec![LagEwma::default(); streams],
            backoff: vec![Time::EPOCH; streams],
            cancel_at: vec![Time::EPOCH; streams],
            cancelled: Map::new(),
            tally: vec![Tally::default(); streams],
            asked: Time::NIL,
            report: Report::default(),
            holder: seed::Holder::default(),
            heard: seed::Heard::default(),
        }
    }

    fn tally(&mut self, stream: Stream) -> Option<&mut Tally> {
        self.tally.get_mut(usize::from(stream))
    }

    /// Appends to `lags` the peer's announcement lag per stream, where
    /// fresh: its mean, and how late it still fits.
    pub(crate) fn fresh_lags(&self, now: Time, lags: &mut Vec<Option<(Duration, Duration)>>) {
        lags.extend(
            self.have_lag
                .iter()
                .map(|l| l.fresh(now).then(|| (l.mean(), l.horizon()))),
        );
    }

    fn back_off(&mut self, stream: Stream, until: Time) {
        if let Some(b) = self.backoff.get_mut(usize::from(stream)) {
            *b = (*b).max(until);
        }
    }
}

/// Counters; see [`PushStats`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PushCounters {
    expected: u64,
    hits: u64,
    misses: u64,
    late: u64,
    unsolicited: u64,
    requests_sent: u64,
    opens: u64,
    declined: u64,
    open_timeouts: u64,
    shed: u64,
    pruned: u64,
    switched: u64,
    accepted: u64,
    rejected: u64,
    sheds_sent: u64,
    stale_drops: u64,
    suppressed: u64,
}

/// The swarm side of the hybrid: subscriptions, push deadlines, and the
/// provider's interior stream and load.
#[derive(Debug)]
pub(crate) struct PushState {
    /// Bound: one per stream.
    pub(crate) subs: Vec<Subscription>,
    /// Streams with an open parent (skips the marking walk without any).
    open: usize,
    /// Deadlines of push-expected chunks. Bound: [`max_pending_requests`]
    /// (one live window); chunks past it are left to pull.
    deadlines: BinQueue,
    /// Chunks expected from a parent. Bound: the window (pruned at GC,
    /// cleared on arrival).
    marked: Map,
    /// Chunks released to pull after a miss, to tell late pushes from
    /// hits. Bound: pruned behind the window at GC.
    released: Map,
    /// The one stream a non-seed node forwards.
    interior: Option<Stream>,
    interior_idle_since: Time,
    next_check: Time,
    next_shed: Time,
    next_report: Time,
    /// Chunks stored per second, for a stream's rate.
    chunk_rate: Sma,
    /// When the first chunk was stored, and the chunks stored since.
    first_stored: Time,
    stored: u64,
    pub(crate) upload: UploadInfo,
    stats: PushCounters,
    /// A seed's fixed peers per stream (`Runner::pin_seed_allocation`).
    /// Bound: the caller's lists, one per stream.
    pin: Option<Vec<Vec<PeerKey>>>,
    /// An adaptive seed's allocation, from its first request or check.
    alloc: Option<seed::Allocator>,
    /// The upload measurement the tap last reported; set only while the
    /// tap is on.
    tapped_upload: Option<Upload>,
}

impl PushState {
    /// `report_at` is the first report tick.
    pub(crate) fn new(streams: usize, now: Time, report_at: Time) -> PushState {
        PushState {
            subs: vec![Subscription::default(); streams],
            open: 0,
            deadlines: BinQueue::default(),
            marked: Map::new(),
            released: Map::new(),
            interior: None,
            interior_idle_since: Time::NIL,
            next_check: now,
            next_shed: now,
            next_report: report_at,
            chunk_rate: Sma::new(10, Duration::from_millis(200), now),
            first_stored: Time::NIL,
            stored: 0,
            upload: UploadInfo::default(),
            stats: PushCounters::default(),
            pin: None,
            alloc: None,
            tapped_upload: None,
        }
    }

    fn sub(&mut self, stream: Stream) -> Option<&mut Subscription> {
        self.subs.get_mut(usize::from(stream))
    }
}

/// Push/pull state of a swarm, for diagnostics and tests. Counters are
/// cumulative.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PushStats {
    pub mode: PushMode,
    /// The open parent per stream.
    pub parents: Vec<Option<PeerKey>>,
    /// Per stream, the open parent's mean push delay (after the chunk was
    /// first seen) and the deadline it gets.
    pub parent_lag: Vec<Option<(Duration, Duration)>>,
    /// The upload measurement admission control last saw: bytes per
    /// second, and the busy fraction in thousandths.
    pub upload_rate: u64,
    pub upload_busy_permille: u32,
    /// The upload capacity admission control estimated from it, in bytes
    /// per second (0 before anything was sent).
    pub upload_capacity: u64,
    /// The node's queueing delay past the runner, from its peers' ping
    /// round trips; `None` with too few peers.
    pub upload_queue_delay: Option<Duration>,
    /// The stream a non-seed node forwards.
    pub interior: Option<Stream>,
    /// Subscriptions we serve, all streams.
    pub children: usize,
    /// Chunks marked as expected from a parent.
    pub expected: u64,
    /// Pushes from a parent that arrived before their deadline (or before
    /// anyone announced them).
    pub hits: u64,
    /// Deadlines that passed without the chunk: released to pull.
    pub misses: u64,
    /// Pushes that arrived after their chunk was released.
    pub late: u64,
    /// Data nobody asked for from a peer that isn't our parent for it.
    pub unsolicited: u64,
    /// STREAM_REQUESTs sent.
    pub requests_sent: u64,
    /// Subscriptions opened (STREAM_OPEN accepted).
    pub opens: u64,
    /// STREAM_REQUESTs declined (STREAM_CLOSE while pending).
    pub declined: u64,
    /// STREAM_REQUESTs answered by nothing in [`OPEN_TIMEOUT`].
    pub open_timeouts: u64,
    /// Open subscriptions the parent closed.
    pub shed: u64,
    /// Parents cancelled for misses.
    pub pruned: u64,
    /// Parents replaced by a faster peer.
    pub switched: u64,
    /// STREAM_REQUESTs we accepted as a provider.
    pub accepted: u64,
    /// STREAM_REQUESTs we declined (STREAM_CLOSE).
    pub rejected: u64,
    /// Subscribers we closed because our uplink saturated.
    pub sheds_sent: u64,
    /// Pushes dropped after waiting longer than [`PUSH_STALE`].
    pub stale_drops: u64,
    /// Pushes not sent because the child had announced the chunk (it got
    /// it elsewhere, e.g. pulled it after a miss) or cancelled it.
    pub suppressed: u64,
    /// An adaptive seed's latest check (zero otherwise): the budget in
    /// stream slots (zero until measured), the window and the copies in
    /// effect; and its first wave, the most copies the window opened
    /// before the budget was measured, with the capacity reading (bytes per
    /// second) that sized them.
    pub seed_budget: usize,
    pub seed_window: usize,
    pub seed_copies: usize,
    pub seed_first_wave: usize,
    pub seed_first_wave_capacity: u64,
}

/// The first bin of `stream` after chunk `b`'s.
fn next_stream_bin(b: Bin, stream: Stream, stream_count: u16) -> Bin {
    let n = u64::from(stream_count.max(1));
    let i0 = b.0 / 2 + 1;
    let i = i0 + (u64::from(stream) + n - i0 % n) % n;
    Bin(i.saturating_mul(2))
}

impl SwarmState {
    fn hybrid(&self) -> bool {
        self.push.is_some()
    }

    /// Marks the chunks of subscribed streams in `from..=to` (base bins)
    /// that we neither have nor requested as expected by push, with the
    /// parent's deadline. Called when someone announces past the newest
    /// announced chunk.
    pub(crate) fn mark_push_expected(&mut self, from: Bin, to: Bin, now: Time) {
        let cap = max_pending_requests(self.window);
        let stream_bits = self.stream_bits;
        let Some(p) = self.push.as_mut() else {
            return;
        };
        if p.open == 0 {
            return;
        }
        let mut x = from.0 & !1;
        while x <= to.0 {
            let b = Bin(x);
            x += 2;
            if p.deadlines.len() >= cap {
                return;
            }
            let stream = ((b.0 >> 1) & stream_bits) as usize;
            let Some(sub) = p.subs.get_mut(stream) else {
                continue;
            };
            let Some(open) = sub.open else {
                continue;
            };
            if b < open.start || self.have_bins.filled_at(b) || self.request_bins.filled_at(b) {
                continue;
            }
            self.request_bins.set(b);
            p.marked.set(b);
            let at = (now + sub.lag.deadline() + DEADLINE_GRANULARITY - Duration::NANOSECOND)
                .truncate(DEADLINE_GRANULARITY);
            p.deadlines.push(b, at);
            sub.expected = sub.expected.saturating_add(1);
            p.stats.expected = p.stats.expected.saturating_add(1);
        }
    }

    /// A chunk was stored.
    pub(crate) fn push_consumed(&mut self, b: Bin, new_chunks: u64, now: Time) {
        if let Some(p) = self.push.as_mut() {
            p.marked.reset(b);
            if new_chunks > 0 {
                p.chunk_rate.add(new_chunks, now);
                if p.first_stored.is_nil() {
                    p.first_stored = now;
                }
                p.stored = p.stored.saturating_add(new_chunks);
            }
        }
    }

    /// Releases the chunks of `stream` still expected from its parent to
    /// pull; the parent in `cancel` gives up those from its start on at
    /// that time. O(expected chunks), at most a window.
    fn release_marked(
        &mut self,
        stream: Stream,
        mut cancel: Option<(Bin, &mut PeerChannelScheduler, Time)>,
    ) -> u64 {
        let Some(p) = self.push.as_mut() else {
            return 0;
        };
        let mut n = 0;
        let mut cur = Cursor::filled_at(p.marked.root_bin());
        let mut next = cur.next_base(&p.marked);
        while let Some(x) = next {
            if ((x.0 >> 1) & self.stream_bits) as Stream == stream {
                p.marked.reset(x);
                if self.have_bins.empty_at(x) {
                    self.request_bins.reset(x);
                    n += 1;
                    if let Some((start, c, now)) = cancel.as_mut()
                        && x >= *start
                    {
                        c.give_up_push(x, *now);
                    }
                }
            }
            next = cur.next_base(&p.marked);
        }
        n
    }

    /// What one subscriber of a stream costs in bytes per second, from our
    /// store rate.
    fn stream_rate(&mut self, now: Time) -> f64 {
        let streams = f64::from(self.stream_count.max(1));
        let Some(p) = self.push.as_mut() else {
            return 0.0;
        };
        p.chunk_rate.rate(Duration::SECOND, now) as f64 * self.chunk_size as f64 * PUSH_OVERHEAD
            / streams
    }

    /// [`SwarmState::stream_rate`] from every chunk stored since the
    /// first, over the time since it: high while the first munros arrive.
    pub(crate) fn early_stream_rate(&self, now: Time) -> Option<f64> {
        let p = self.push.as_ref()?;
        let secs = (now - p.first_stored).as_secs_f64();
        (!p.first_stored.is_nil() && secs > 0.0).then(|| {
            p.stored as f64 * self.chunk_size as f64 * PUSH_OVERHEAD
                / f64::from(self.stream_count.max(1))
                / secs
        })
    }

    fn children(&self) -> usize {
        self.streams.iter().map(|st| st.subscribers.len()).sum()
    }
}

impl PeerChannelScheduler {
    /// Writes the report due to the peer, as many ANNOUNCEs as it takes,
    /// each with the entries that fit. An entry is at most 74 bytes, so one
    /// always fits an empty frame.
    pub(super) fn write_report(&mut self, cw: &mut MeteredWriter<'_>) -> Result<(), codec::Error> {
        let Some(r) = self.push.as_mut().map(|pc| &mut pc.report) else {
            return Ok(());
        };
        while r.sent < r.ends.len() {
            let at = |i: usize| i.checked_sub(1).and_then(|i| r.ends.get(i)).copied();
            let start = at(r.sent).unwrap_or(0);
            let room = cw.available().saturating_sub(MESSAGE_TYPE_LEN + 1);
            let mut end = r.sent;
            while end < r.ends.len()
                && end - r.sent < MAX_ANNOUNCE_ENTRIES
                && at(end + 1).is_some_and(|e| e - start <= room)
            {
                end += 1;
            }
            if end == r.sent {
                return Err(codec::Error::NotEnoughSpace);
            }
            let bytes = r.bytes.get(start..at(end).unwrap_or(start));
            let Some(a) = bytes.and_then(|b| Announce::from_entry_bytes(end - r.sent, b)) else {
                break;
            };
            cw.write(&Message::Announce(a))?;
            r.sent = end;
        }
        *r = Report::default();
        Ok(())
    }

    /// Queues a STREAM_CANCEL for `stream` and forgets the request. Releases
    /// nothing: chunks requested from other channels stay theirs, and what
    /// this peer was expected to push is released by the caller or its
    /// deadlines.
    fn push_cancel(&mut self, stream: Stream) {
        self.cancel_open_race(stream);
        if let Some(r) = self.request_streams.get_mut(usize::from(stream)) {
            *r = Bin::NONE;
        }
        self.extra.push_stream(stream, StreamMessage::Cancel);
    }

    fn push_backoff(&mut self, stream: Stream, until: Time) {
        if let Some(pc) = self.push.as_mut() {
            pc.back_off(stream, until);
        }
    }

    /// Gives up on the peer's push of `x`: CANCEL, and pull leaves the
    /// chunk to the other channels for [`REQUEST_BACKOFF`], as it does a
    /// timed-out request's.
    fn give_up_push(&mut self, x: Bin, now: Time) {
        self.cancel_bins.set(x);
        self.backoff_bins.set(x);
        self.backoff_until = now + REQUEST_BACKOFF;
    }
}

impl PeerSwarmScheduler {
    /// The next push deadline, for the runner's timer.
    pub(crate) fn next_push_deadline(&self) -> Option<Time> {
        self.s.push.as_ref()?.deadlines.peek_time()
    }

    /// Records the runner's upload measurement.
    pub(crate) fn set_upload(&mut self, info: UploadInfo) {
        if let Some(p) = self.s.push.as_mut() {
            p.upload = info;
        }
    }

    /// Reports the upload measurement to the tap, with this swarm's
    /// committed push rate, if it changed since the last report.
    pub(crate) fn tap_upload(&mut self, committed: f64) {
        let s = &mut self.s;
        if !s.tap.is_on() {
            return;
        }
        let Some(p) = s.push.as_mut() else {
            return;
        };
        let u = p.upload;
        let sample = Upload {
            rate: u.rate,
            busy: (u.busy * 1000.0) as u64,
            capacity: u.capacity().map(|c| c as u64),
            hint: u.hint,
            others: u.others as u64,
            committed: committed as u64,
            queue_delay: u.queue_delay,
        };
        if p.tapped_upload == Some(sample) {
            return;
        }
        p.tapped_upload = Some(sample);
        s.tap.emit(s.key, None, || TapKind::Upload(sample));
    }

    /// The push rate this swarm's subscriptions commit, in bytes per second
    /// (0 outside hybrid mode).
    pub(crate) fn committed_push(&mut self, now: Time) -> f64 {
        if !self.s.hybrid() {
            return 0.0;
        }
        let rate = self.s.stream_rate(now);
        self.s.children() as f64 * rate
    }

    /// Releases the push-expected chunks whose deadline passed to pull and
    /// counts the misses; prunes parents over the miss limit.
    pub(crate) fn expire_push(&mut self, now: Time, q: &mut impl PeerTaskQueue) {
        let s = &mut self.s;
        let Some(p) = s.push.as_mut() else {
            return;
        };
        let mut released = 0u64;
        while let Some((x, deadline)) = p.deadlines.pop_le_at(now) {
            if !p.marked.filled_at(x) {
                continue;
            }
            if s.have_bins.filled_at(x) {
                p.marked.reset(x);
                continue;
            }
            // The parent may push until DEADLINE_MAX past the chunk's first
            // announcement: pull only if a holder is expected to beat that.
            let stream = usize::from(((x.0 >> 1) & s.stream_bits) as Stream);
            let cap = s.bin_times.get(x).map(|(_, t)| t + DEADLINE_MAX);
            let parent = p
                .subs
                .get(stream)
                .and_then(|sub| sub.open.filter(|o| x >= o.start))
                .map(|o| o.peer);
            if let Some(parent) = parent
                && let Some(cap) = cap
                && now < cap
            {
                // A holder we haven't timed yet may well win.
                let pull = self
                    .channels
                    .values()
                    .filter(|c| c.peer != parent && !c.choked && c.peer_have_bins.filled_at(x))
                    .map(|c| c.answer_time.mean().unwrap_or(Duration::ZERO))
                    .min()
                    .map(|d| now + d);
                if pull.is_none_or(|t| cap + ROUTE_MARGIN < t) {
                    p.deadlines.push(x, cap.min(now + RELEASE_RECHECK));
                    continue;
                }
            }
            p.marked.reset(x);
            s.request_bins.reset(x);
            p.released.set(x);
            released += 1;
            p.stats.misses = p.stats.misses.saturating_add(1);
            // Chunks before the parent's start were expected from the one
            // it replaced.
            let stream = ((x.0 >> 1) & s.stream_bits) as Stream;
            let mut parent = None;
            let mut counted = false;
            if let Some(sub) = p.sub(stream) {
                parent = sub.open.map(|o| o.peer);
                if let Some(o) = sub.open.filter(|o| x >= o.start) {
                    counted = true;
                    sub.on_miss();
                    if let Some(c) = self.channels.get_mut(&o.peer) {
                        c.give_up_push(x, now);
                        if let Some(t) = c.push.as_mut().and_then(|pc| pc.tally(stream)) {
                            t.misses = t.misses.saturating_add(1);
                        }
                    }
                }
            }
            s.tap.emit(s.key, parent, || TapKind::PushMiss {
                bin: x,
                deadline,
                first_seen: s.bin_times.get(x).map_or(Time::EPOCH, |(_, t)| t),
                counted,
                lag: p
                    .subs
                    .get(usize::from(stream))
                    .map(|sub| sub.lag.estimate())
                    .unwrap_or_default(),
            });
        }
        if released == 0 {
            return;
        }
        // Repair at once: every channel gets a write cycle to request them.
        let key = s.key;
        for &peer in self.channels.keys() {
            q.enqueue_now(peer, key);
        }
        for stream in 0..self.s.stream_count {
            let rule = self
                .s
                .push
                .as_ref()
                .and_then(|p| p.subs.get(usize::from(stream)))
                .and_then(|sub| sub.prune_rule(now));
            if let Some(rule) = rule {
                self.prune(stream, rule, now, q);
            }
        }
    }

    /// The report tick: each neighbour that asked within [`REPORT_LEASE`]
    /// is due the tallies of our other parents, at most twice the stream
    /// count of entries, the most expected first; then every tally starts
    /// over.
    fn report(&mut self, now: Time, q: &mut impl PeerTaskQueue) {
        let Some(p) = self.s.push.as_mut() else {
            return;
        };
        if now < p.next_report {
            return;
        }
        p.next_report = now + REPORT_INTERVAL;
        let mut tallies: Vec<(u64, PeerKey, Stream, Tally)> = Vec::new();
        for (&peer, c) in &mut self.channels {
            let named = c.id.is_some();
            let Some(pc) = c.push.as_mut() else {
                continue;
            };
            for (i, t) in pc.tally.iter_mut().enumerate() {
                let t = std::mem::take(t);
                if named && !t.is_empty() {
                    let expected = t.hits.saturating_add(t.misses);
                    tallies.push((expected, peer, i as Stream, t));
                }
            }
        }
        if tallies.is_empty() {
            return;
        }
        tallies.sort_by(|a, b| b.0.cmp(&a.0).then((a.1, a.2).cmp(&(b.1, b.2))));
        let askers: Vec<PeerKey> = self
            .channels
            .iter()
            .filter(|(_, c)| {
                c.handshake_received
                    && c.push
                        .as_ref()
                        .is_some_and(|pc| !pc.asked.is_nil() && now - pc.asked <= REPORT_LEASE)
            })
            .map(|(&peer, _)| peer)
            .collect();
        let cap = 2 * usize::from(self.s.stream_count);
        for asker in askers {
            let mut r = Report::default();
            for &(_, parent, stream, t) in tallies.iter().filter(|x| x.1 != asker).take(cap) {
                let Some(id) = self.channels.get(&parent).and_then(|c| c.id.as_deref()) else {
                    continue;
                };
                let e = t.entry(stream, id);
                let start = r.bytes.len();
                r.bytes.resize(start + e.byte_len(), 0);
                match r.bytes.get_mut(start..).map(|b| e.encode(b)) {
                    Some(Ok(_)) => r.ends.push(r.bytes.len()),
                    _ => r.bytes.truncate(start),
                }
            }
            if let Some(pc) = self
                .channels
                .get_mut(&asker)
                .and_then(|c| c.push.as_mut())
                .filter(|_| !r.ends.is_empty())
            {
                pc.report = r;
                q.enqueue(asker, self.s.key);
            }
        }
    }

    /// Cancels the stream's parent for misses, by `rule` (silent, lossy).
    fn prune(&mut self, stream: Stream, rule: (bool, bool), now: Time, q: &mut impl PeerTaskQueue) {
        let s = &mut self.s;
        let Some(p) = s.push.as_mut() else {
            return;
        };
        let Some(sub) = p.sub(stream) else {
            return;
        };
        let Some(parent) = sub.open.take() else {
            return;
        };
        s.tap.emit(s.key, Some(parent.peer), || TapKind::Pruned {
            stream,
            why: sub.prune_why(rule, &parent, now),
        });
        p.open = p.open.saturating_sub(1);
        p.stats.pruned = p.stats.pruned.saturating_add(1);
        p.next_check = now;
        if let Some(c) = self.channels.get_mut(&parent.peer) {
            c.push_cancel(stream);
            c.push_backoff(stream, now + PRUNE_BACKOFF);
            q.enqueue_now(parent.peer, s.key);
        }
        self.drop_parent(stream, Some((parent, now)), q);
    }

    /// After the open parent of `stream` went away: its expected chunks go
    /// to pull, and it is no longer the stream's source. If it is still a
    /// neighbour (`parent`, and when it went), we give up its pushes of
    /// those due from it.
    fn drop_parent(
        &mut self,
        stream: Stream,
        parent: Option<(Parent, Time)>,
        q: &mut impl PeerTaskQueue,
    ) {
        let s = &mut self.s;
        if s.streams
            .get(usize::from(stream))
            .is_some_and(|st| st.source.is_some())
        {
            s.reset_stream_source(stream);
        }
        let cancel =
            parent.and_then(|(o, now)| self.channels.get_mut(&o.peer).map(|c| (o.start, c, now)));
        if s.release_marked(stream, cancel) > 0 {
            for &peer in self.channels.keys() {
                q.enqueue_now(peer, s.key);
            }
        }
    }

    /// Hybrid `handle_have` bookkeeping: the peer's announcement lag for
    /// the chunk's stream (`pc` is the channel's `push`).
    pub(crate) fn push_have_lag(
        pc: &mut Option<PushChannel>,
        stream: usize,
        lag: Duration,
        now: Time,
    ) {
        if let Some(l) = pc.as_mut().and_then(|pc| pc.have_lag.get_mut(stream)) {
            l.update(lag, now);
        }
    }

    /// Hybrid `handle_data` bookkeeping for verified data: hits, late
    /// pushes, the parent's push delay, and unsolicited pushes.
    pub(crate) fn push_data(
        &mut self,
        peer: PeerKey,
        b: Bin,
        new: bool,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) {
        let Some(c) = self.channels.get_mut(&peer) else {
            return;
        };
        let s = &mut self.s;
        let stream_bits = s.stream_bits;
        let mut first = s.bin_time(b);
        if first == Time::EPOCH {
            first = now;
        }
        let Some(p) = s.push.as_mut() else {
            return;
        };
        let (mut x, r) = b.base();
        while x <= r {
            let bin = x;
            x += 2;
            if c.request_times.get(bin).is_some() {
                // A pull answer.
                continue;
            }
            let stream = ((bin.0 >> 1) & stream_bits) as Stream;
            let Some(sub) = p.subs.get_mut(usize::from(stream)) else {
                continue;
            };
            if sub.open.is_some_and(|o| o.peer == peer && bin >= o.start) {
                let lag = now - first;
                sub.lag.update(lag, now);
                sub.pushes = sub.pushes.saturating_add(1);
                let tally = c.push.as_mut().and_then(|pc| pc.tally(stream));
                if p.released.filled_at(bin) {
                    p.stats.late = p.stats.late.saturating_add(1);
                    if let Some(t) = tally {
                        t.late = t.late.saturating_add(1);
                    }
                    s.tap
                        .emit(s.key, Some(peer), || TapKind::PushLate { bin, lag });
                } else if new {
                    p.stats.hits = p.stats.hits.saturating_add(1);
                    sub.on_hit(now);
                    if let Some(t) = tally {
                        t.hit(lag);
                    }
                    s.tap
                        .emit(s.key, Some(peer), || TapKind::PushHit { bin, lag });
                }
            } else if sub.pending.is_some_and(|o| o.peer == peer)
                || sub.open.is_some_and(|o| o.peer == peer)
            {
                // Pushes racing the STREAM_OPEN, or before the start.
            } else {
                p.stats.unsolicited = p.stats.unsolicited.saturating_add(1);
                s.tap
                    .emit(s.key, Some(peer), || TapKind::PushUnsolicited { bin });
                // A peer pushing a stream we don't take from it: a lost
                // STREAM_CANCEL, or an old parent. Cancel again, rate
                // limited.
                if let Some(pc) = c.push.as_mut()
                    && let Some(at) = pc.cancel_at.get_mut(usize::from(stream))
                    && now - *at >= UNSOLICITED_CANCEL_INTERVAL
                {
                    *at = now;
                    c.extra.push_stream(stream, StreamMessage::Cancel);
                    q.enqueue_now(peer, s.key);
                }
            }
        }
    }

    /// Hybrid `handle_data` for data that failed verification: an expected
    /// chunk is released to pull at once, and counts as a miss if its
    /// parent sent it.
    pub(crate) fn push_invalid(&mut self, peer: PeerKey, b: Bin) {
        let s = &mut self.s;
        let tally = self.channels.get_mut(&peer).and_then(|c| c.push.as_mut());
        let stream_bits = s.stream_bits;
        let Some(p) = s.push.as_mut() else {
            return;
        };
        if !p.marked.filled_at(b) {
            return;
        }
        p.marked.reset(b);
        p.released.set(b);
        let stream = ((b.0 >> 1) & stream_bits) as Stream;
        if let Some(sub) = p.sub(stream)
            && sub.open.is_some_and(|o| o.peer == peer)
        {
            sub.on_miss();
            p.stats.misses = p.stats.misses.saturating_add(1);
            if let Some(t) = tally.and_then(|pc| pc.tally(stream)) {
                t.misses = t.misses.saturating_add(1);
            }
        }
    }

    /// Hybrid STREAM_REQUEST: admission control and the Thicket constraint.
    /// Returns whether the request was accepted, and whether it is for a
    /// stream other than our interior one; the caller then subscribes the
    /// peer.
    pub(crate) fn push_admit(
        &mut self,
        peer: PeerKey,
        stream: Stream,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) -> (bool, bool) {
        if !self.s.hybrid() {
            return (true, false);
        }
        let granted = (self.adaptive_seed()
            && !self
                .s
                .streams
                .get(usize::from(stream))
                .is_some_and(|st| st.subscribers.iter().any(|x| x.channel == peer)))
        .then(|| self.grant(peer, now));
        let rate = self.s.stream_rate(now);
        let children = self.s.children();
        let s = &mut self.s;
        let is_seed = s.is_seed;
        let Some(st) = s.streams.get(usize::from(stream)) else {
            return (false, false);
        };
        let already = st.subscribers.iter().any(|sub| sub.channel == peer);
        let stream_children = st.subscribers.len();
        let Some(p) = s.push.as_mut() else {
            return (true, false);
        };
        let interior = is_seed || p.interior.is_none_or(|i| i == stream);
        let extra = !interior;
        let pinned = p.pin.as_ref().map(|pin| {
            pin.get(usize::from(stream))
                .is_some_and(|v| v.contains(&peer))
        });
        let target = if extra {
            EXTRA_UTILIZATION
        } else {
            INTERIOR_UTILIZATION
        };
        let (ok, rule) = if already {
            (true, AdmitRule::Subscribed)
        } else {
            match (pinned.or(granted.map(|g| g.ok)), p.upload.capacity()) {
                (Some(ok), _) => (
                    ok,
                    match (pinned, granted) {
                        (None, Some(g)) => g.rule,
                        _ => AdmitRule::Pin,
                    },
                ),
                (None, None) => (
                    !extra && stream_children < UNMEASURED_FANOUT,
                    AdmitRule::Unmeasured,
                ),
                (None, Some(cap)) => {
                    let committed = children as f64 * rate + p.upload.others;
                    (
                        (p.upload.rate as f64).max(committed) + rate <= cap * target,
                        AdmitRule::Measured,
                    )
                }
            }
        };
        let why = || Admission {
            rule,
            extra,
            upload_rate: p.upload.rate,
            committed: (children as f64 * rate + p.upload.others) as u64,
            others: p.upload.others as u64,
            stream_rate: rate as u64,
            capacity: p.upload.capacity().map(|c| c as u64),
            target_permille: (target * 1000.0) as u64,
            stream_children: stream_children as u64,
            holders: granted.map_or(0, |g| g.holders as u64),
            copies: granted.map_or(0, |g| g.copies as u64),
            held: granted.map_or(0, |g| g.held as u64),
            per_holder: granted.map_or(0, |g| g.per_holder as u64),
        };
        if ok {
            s.tap.emit(s.key, Some(peer), || TapKind::Admitted {
                stream,
                why: why(),
            });
            if !is_seed && p.interior.is_none() {
                p.interior = Some(stream);
            }
            p.stats.accepted = p.stats.accepted.saturating_add(1);
            return (true, extra);
        }
        s.tap.emit(s.key, Some(peer), || TapKind::Rejected {
            stream,
            why: why(),
        });
        p.stats.rejected = p.stats.rejected.saturating_add(1);
        self.close_subscriber(peer, stream, q);
        (false, extra)
    }

    /// Ends `peer`'s subscription to `stream`, if any, with a STREAM_CLOSE.
    fn close_subscriber(&mut self, peer: PeerKey, stream: Stream, q: &mut impl PeerTaskQueue) {
        if let Some(st) = self.s.streams.get_mut(usize::from(stream)) {
            self.s.subscribers_changed |= st.remove_subscriber(peer);
        }
        if let Some(c) = self.channels.get_mut(&peer) {
            c.extra.push_stream(stream, StreamMessage::Close);
            q.enqueue_now(peer, self.s.key);
        }
    }

    /// Hybrid check of a STREAM_OPEN from a peer we asked: a start more
    /// than a live window past the request means nothing would ever be
    /// expected from the parent (and nothing could prune it). That counts
    /// as a decline: STREAM_CANCEL, and the peer is backed off.
    pub(crate) fn push_open_acceptable(
        &mut self,
        peer: PeerKey,
        stream: Stream,
        b: Bin,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) -> bool {
        let key = self.s.key;
        let window = self.s.live_window;
        let Some(p) = self.s.push.as_mut() else {
            return true;
        };
        let Some(sub) = p.subs.get_mut(usize::from(stream)) else {
            return true;
        };
        let asked = [sub.pending, sub.open]
            .into_iter()
            .flatten()
            .find(|x| x.peer == peer)
            .map(|x| x.start);
        if asked.is_none_or(|a| b.0 <= a.0.saturating_add(window.0)) {
            return true;
        }
        if sub.pending.is_some_and(|x| x.peer == peer) {
            sub.pending = None;
        }
        let was_open = sub.open.filter(|x| x.peer == peer);
        if was_open.is_some() {
            sub.open = None;
            p.open = p.open.saturating_sub(1);
        }
        p.stats.declined = p.stats.declined.saturating_add(1);
        p.next_check = now;
        self.s
            .tap
            .emit(key, Some(peer), || TapKind::Declined { stream });
        if let Some(c) = self.channels.get_mut(&peer) {
            c.push_cancel(stream);
            c.push_backoff(stream, now + PRUNE_BACKOFF);
            q.enqueue_now(peer, key);
        }
        if was_open.is_some() {
            self.drop_parent(stream, was_open.map(|o| (o, now)), q);
        }
        false
    }

    /// Hybrid STREAM_OPEN from `peer` (already checked: we asked it for the
    /// stream). A pending request becomes the parent; a switch cancels the
    /// old parent.
    pub(crate) fn push_open(
        &mut self,
        peer: PeerKey,
        stream: Stream,
        b: Bin,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) {
        let key = self.s.key;
        let Some(p) = self.s.push.as_mut() else {
            return;
        };
        let Some(sub) = p.sub(stream) else {
            return;
        };
        if let Some(pending) = sub.pending
            && pending.peer == peer
        {
            let old = sub.open;
            sub.on_open(Parent {
                peer,
                start: b,
                since: now,
                hint: pending.hint,
            });
            p.stats.opens = p.stats.opens.saturating_add(1);
            match old {
                Some(o) if o.peer != peer => {
                    // Graft done, now prune. What the old parent was
                    // expected to push stays expected: it queued those
                    // pushes before our STREAM_CANCEL, and the deadlines
                    // still release what doesn't come.
                    p.stats.switched = p.stats.switched.saturating_add(1);
                    self.s.tap.emit(key, Some(peer), || TapKind::Switched {
                        stream,
                        from: o.peer,
                    });
                    if let Some(c) = self.channels.get_mut(&o.peer) {
                        c.push_cancel(stream);
                        q.enqueue_now(o.peer, key);
                    }
                }
                Some(_) => {}
                None => p.open += 1,
            }
            // `handle_stream_open` made the peer the stream's source.
        } else if let Some(o) = sub.open.as_mut()
            && o.peer == peer
        {
            o.start = b;
        }
    }

    /// Hybrid STREAM_CLOSE from `peer`: a declined request or a shed
    /// subscription. The peer is backed off for the stream.
    pub(crate) fn push_close(
        &mut self,
        peer: PeerKey,
        stream: Stream,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) {
        let Some(p) = self.s.push.as_mut() else {
            return;
        };
        let Some(sub) = p.subs.get_mut(usize::from(stream)) else {
            return;
        };
        let mut lost_parent = None;
        let mut known = false;
        let mut declined = false;
        if sub.pending.is_some_and(|x| x.peer == peer) {
            sub.pending = None;
            p.stats.declined = p.stats.declined.saturating_add(1);
            declined = true;
            known = true;
        }
        if let Some(o) = sub.open.filter(|x| x.peer == peer) {
            sub.open = None;
            p.open = p.open.saturating_sub(1);
            p.stats.shed = p.stats.shed.saturating_add(1);
            lost_parent = Some(o);
            known = true;
        }
        if !known {
            return;
        }
        let key = self.s.key;
        if declined {
            self.s
                .tap
                .emit(key, Some(peer), || TapKind::Declined { stream });
        }
        if lost_parent.is_some() {
            self.s
                .tap
                .emit(key, Some(peer), || TapKind::ShedBy { stream });
        }
        p.next_check = now;
        if let Some(c) = self.channels.get_mut(&peer) {
            if let Some(r) = c.request_streams.get_mut(usize::from(stream)) {
                *r = Bin::NONE;
            }
            c.push_backoff(stream, now + DECLINE_BACKOFF);
        }
        if lost_parent.is_some() {
            self.drop_parent(stream, lost_parent.map(|o| (o, now)), q);
        }
    }

    /// Hybrid `close_channel`: the peer's subscriptions on our side end.
    pub(crate) fn push_close_channel(&mut self, peer: PeerKey, q: &mut impl PeerTaskQueue) {
        let Some(p) = self.s.push.as_mut() else {
            return;
        };
        let mut lost = Vec::new();
        for (i, sub) in p.subs.iter_mut().enumerate() {
            if sub.pending.is_some_and(|x| x.peer == peer) {
                sub.pending = None;
            }
            if sub.open.is_some_and(|x| x.peer == peer) {
                sub.open = None;
                p.open = p.open.saturating_sub(1);
                p.stats.shed = p.stats.shed.saturating_add(1);
                lost.push(i as Stream);
            }
        }
        p.next_check = Time::EPOCH;
        for stream in lost {
            self.drop_parent(stream, None, q);
        }
    }

    /// `Runner::request_stream` in hybrid mode: ask `peer` for the stream
    /// (it replaces a pending request; the open parent stays until the new
    /// one opens).
    pub(crate) fn push_subscribe(&mut self, peer: PeerKey, stream: Stream, start: Bin) -> bool {
        let Some(p) = self.s.push.as_mut() else {
            return false;
        };
        let Some(sub) = p.subs.get_mut(usize::from(stream)) else {
            return false;
        };
        let old = sub.pending.replace(Parent {
            peer,
            start,
            since: Time::NIL,
            hint: LagEwma::default(),
        });
        p.stats.requests_sent = p.stats.requests_sent.saturating_add(1);
        if let Some(o) = old
            && o.peer != peer
            && let Some(c) = self.channels.get_mut(&o.peer)
        {
            c.push_cancel(stream);
        }
        if let Some(c) = self.channels.get_mut(&peer) {
            c.add_stream_request(stream, start);
        }
        true
    }

    /// `Runner::cancel_stream` in hybrid mode.
    pub(crate) fn push_unsubscribe(
        &mut self,
        peer: PeerKey,
        stream: Stream,
        now: Time,
        q: &mut impl PeerTaskQueue,
    ) -> bool {
        let Some(p) = self.s.push.as_mut() else {
            return false;
        };
        let Some(sub) = p.subs.get_mut(usize::from(stream)) else {
            return false;
        };
        if sub.pending.is_some_and(|x| x.peer == peer) {
            sub.pending = None;
        }
        let was_open = sub.open.filter(|x| x.peer == peer);
        if was_open.is_some() {
            sub.open = None;
            p.open = p.open.saturating_sub(1);
        }
        if let Some(c) = self.channels.get_mut(&peer) {
            c.push_cancel(stream);
            c.push_backoff(stream, now + DECLINE_BACKOFF);
        }
        if was_open.is_some() {
            self.drop_parent(stream, was_open.map(|o| (o, now)), q);
        }
        true
    }

    /// The best peer to take `stream` from: the lowest announcement lag
    /// among peers that announced it recently, aren't backed off, and
    /// aren't our own subscribers for it (no 2-cycles).
    fn best_candidate(
        &self,
        stream: Stream,
        exclude: Option<PeerKey>,
        now: Time,
    ) -> Option<(PeerKey, Duration)> {
        let st = self.s.streams.get(usize::from(stream))?;
        let i = usize::from(stream);
        let mut best: Option<(PeerKey, Duration)> = None;
        for (&peer, c) in &self.channels {
            if Some(peer) == exclude || !c.handshake_received || c.choked {
                continue;
            }
            if st.subscribers.iter().any(|sub| sub.channel == peer) {
                continue;
            }
            let Some(pc) = c.push.as_ref() else {
                continue;
            };
            if pc.backoff.get(i).is_some_and(|&t| t > now) {
                continue;
            }
            let Some(lag) = pc.have_lag.get(i).filter(|l| l.fresh(now)) else {
                continue;
            };
            let m = lag.mean();
            if best.is_none_or(|(_, b)| m < b) {
                best = Some((peer, m));
            }
        }
        best
    }

    /// Reports a parent check of `stream` that did `action`: the
    /// candidates [`PeerSwarmScheduler::best_candidate`] weighs, the parent
    /// excluded, and why it passed over the others.
    #[allow(clippy::too_many_arguments)]
    fn tap_choice(
        &mut self,
        stream: Stream,
        parent: Option<PeerKey>,
        action: ParentAction,
        votes: u32,
        parent_have: Option<Duration>,
        parent_push: Option<Duration>,
        now: Time,
    ) {
        let PeerSwarmScheduler { s, channels } = self;
        if !s.tap.is_on() {
            return;
        }
        let i = usize::from(stream);
        let Some(st) = s.streams.get(i) else {
            return;
        };
        let mut c = ParentChoice {
            action,
            best: None,
            runner_up: None,
            unready: 0,
            subscriber: 0,
            backed_off: 0,
            stale: 0,
            votes: votes.into(),
            parent_have,
            parent_push,
        };
        for (&peer, ch) in channels.iter() {
            if Some(peer) == parent {
                continue;
            }
            if !ch.handshake_received || ch.choked {
                c.unready += 1;
                continue;
            }
            if st.subscribers.iter().any(|sub| sub.channel == peer) {
                c.subscriber += 1;
                continue;
            }
            let Some(pc) = ch.push.as_ref() else {
                c.unready += 1;
                continue;
            };
            if pc.backoff.get(i).is_some_and(|&t| t > now) {
                c.backed_off += 1;
                continue;
            }
            let Some(lag) = pc.have_lag.get(i).filter(|l| l.fresh(now)) else {
                c.stale += 1;
                continue;
            };
            let m = lag.mean();
            if c.best.is_none_or(|(_, b)| m < b) {
                c.runner_up = c.best;
                c.best = Some((peer, m));
            } else if c.runner_up.is_none_or(|(_, r)| m < r) {
                c.runner_up = Some((peer, m));
            }
        }
        let chosen = c.best.map(|b| b.0);
        s.tap.emit(s.key, chosen, || TapKind::ParentChoice {
            stream,
            choice: c,
        });
    }

    /// Sends a STREAM_REQUEST for `stream` to `peer`.
    fn graft(&mut self, peer: PeerKey, stream: Stream, now: Time, q: &mut impl PeerTaskQueue) {
        let hint = self
            .channels
            .get(&peer)
            .and_then(|c| c.push.as_ref())
            .and_then(|pc| pc.have_lag.get(usize::from(stream)))
            .copied()
            .unwrap_or_default();
        let s = &mut self.s;
        let from = s.have_bin_max.max(s.peer_max_have_bin);
        let start = next_stream_bin(from, stream, s.stream_count);
        let Some(p) = s.push.as_mut() else {
            return;
        };
        let Some(sub) = p.subs.get_mut(usize::from(stream)) else {
            return;
        };
        sub.pending = Some(Parent {
            peer,
            start,
            since: now,
            hint,
        });
        sub.switch_votes = 0;
        p.stats.requests_sent = p.stats.requests_sent.saturating_add(1);
        if let Some(c) = self.channels.get_mut(&peer) {
            c.add_stream_request(stream, start);
            q.enqueue_now(peer, s.key);
        }
    }

    /// The hybrid part of `Run`: parent selection and switching, request
    /// timeouts, shedding, and the interior stream's idle release.
    pub(crate) fn run_push(&mut self, now: Time, q: &mut impl PeerTaskQueue) {
        if !self.s.hybrid() {
            return;
        }
        if self.adaptive_seed() {
            self.ask_reports(now, q);
            self.check_allocation(now, q);
        } else {
            self.shed(now, q);
        }
        self.release_idle_interior(now);
        // The seed has every chunk first: it is a parent only.
        if self.s.is_seed {
            return;
        }
        self.report(now, q);

        let due = self.s.push.as_ref().is_some_and(|p| now >= p.next_check);
        if !due || !self.s.first_chunk_set {
            return;
        }
        if let Some(p) = self.s.push.as_mut() {
            p.next_check = now + PARENT_CHECK_INTERVAL;
        }
        for stream in 0..self.s.stream_count {
            self.check_parent(stream, now, q);
        }
    }

    fn check_parent(&mut self, stream: Stream, now: Time, q: &mut impl PeerTaskQueue) {
        let key = self.s.key;
        let Some(sub) = self
            .s
            .push
            .as_mut()
            .and_then(|p| p.subs.get_mut(usize::from(stream)))
        else {
            return;
        };
        if let Some(pending) = sub.pending.as_mut() {
            if pending.since.is_nil() {
                pending.since = now;
            }
            if now - pending.since <= OPEN_TIMEOUT {
                return;
            }
            let pending = *pending;
            sub.pending = None;
            if let Some(p) = self.s.push.as_mut() {
                p.stats.open_timeouts = p.stats.open_timeouts.saturating_add(1);
            }
            self.s
                .tap
                .emit(key, Some(pending.peer), || TapKind::OpenTimeout {
                    stream,
                    waited: now - pending.since,
                });
            if let Some(c) = self.channels.get_mut(&pending.peer) {
                c.push_cancel(stream);
                c.push_backoff(stream, now + DECLINE_BACKOFF);
                q.enqueue_now(pending.peer, key);
            }
        }
        let Some(sub) = self
            .s
            .push
            .as_ref()
            .and_then(|p| p.subs.get(usize::from(stream)))
        else {
            return;
        };
        match sub.open {
            None => {
                let best = self.best_candidate(stream, None, now);
                let action = if best.is_some() {
                    ParentAction::Graft
                } else {
                    ParentAction::None
                };
                self.tap_choice(stream, None, action, 0, None, None, now);
                if let Some((peer, _)) = best {
                    self.graft(peer, stream, now, q);
                }
            }
            Some(parent) => {
                // Graft onto a peer that is closer to the source than the
                // parent (it announces sooner), or much faster than the
                // parent's pushes arrive. Comparing a bystander's
                // announcements with the parent's pushes alone would keep
                // finding a "faster" peer: a parent's pushes queue behind
                // its other children's, a bystander's HAVEs don't.
                let held = !parent.since.is_nil() && now - parent.since < SWITCH_HOLD;
                let push = (sub.pushes >= 4).then(|| sub.lag.mean());
                let have = self
                    .channels
                    .get(&parent.peer)
                    .and_then(|c| c.push.as_ref())
                    .and_then(|pc| pc.have_lag.get(usize::from(stream)))
                    .filter(|l| l.fresh(now))
                    .map(|l| l.mean());
                let better = (!held)
                    .then(|| self.best_candidate(stream, Some(parent.peer), now))
                    .flatten()
                    .filter(|&(_, l)| {
                        have.is_some_and(|h| l + SWITCH_MARGIN.max(h / 4) < h)
                            || push.is_some_and(|p| l + SLOW_PARENT_MARGIN.max(p / 4) < p)
                    });
                let Some(sub) = self
                    .s
                    .push
                    .as_mut()
                    .and_then(|p| p.subs.get_mut(usize::from(stream)))
                else {
                    return;
                };
                let (action, votes) = match better {
                    Some(_) => {
                        sub.switch_votes += 1;
                        let v = sub.switch_votes;
                        let a = if v >= SWITCH_CHECKS {
                            ParentAction::Switch
                        } else {
                            ParentAction::Vote
                        };
                        (Some(a), v)
                    }
                    None => {
                        let v = std::mem::take(&mut sub.switch_votes);
                        ((v > 0).then_some(ParentAction::Reset), v)
                    }
                };
                if let Some(a) = action {
                    self.tap_choice(stream, Some(parent.peer), a, votes, have, push, now);
                }
                if action == Some(ParentAction::Switch)
                    && let Some((peer, _)) = better
                {
                    self.graft(peer, stream, now, q);
                }
            }
        }
    }

    /// Closes subscribers while the uplink is saturated: enough to bring
    /// the committed push rate under the interior target, extra streams
    /// first, newest first.
    fn shed(&mut self, now: Time, q: &mut impl PeerTaskQueue) {
        let rate = self.s.stream_rate(now);
        let children = self.s.children();
        let s = &mut self.s;
        let Some(p) = s.push.as_mut() else {
            return;
        };
        if children == 0 || now < p.next_shed || p.upload.busy < SHED_UTILIZATION || p.pin.is_some()
        {
            return;
        }
        let Some(cap) = p.upload.capacity() else {
            return;
        };
        // Pull answers don't count: closing a subscriber turns its pushes
        // into pulls, so it can't relieve them. Each swarm sheds its share
        // of the node's excess.
        let committed = children as f64 * rate;
        let total = committed + p.upload.others;
        let excess = total - cap * INTERIOR_UTILIZATION;
        if excess <= 0.0 || total <= 0.0 {
            return;
        }
        let excess = excess * committed / total;
        let n = if rate > 0.0 {
            ((excess / rate).ceil() as usize).clamp(1, children)
        } else {
            1
        };
        let (busy, others) = (p.upload.busy, p.upload.others);
        p.next_shed = now + SHED_INTERVAL;
        p.stats.sheds_sent = p.stats.sheds_sent.saturating_add(n as u64);

        let mut victims: Vec<(bool, Time, Stream, PeerKey)> = s
            .streams
            .iter()
            .enumerate()
            .flat_map(|(i, st)| {
                st.subscribers
                    .iter()
                    .map(move |sub| (sub.extra, sub.since, i as Stream, sub.channel))
            })
            .collect();
        // Extra streams first, then the newest.
        victims.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        for &(_, _, stream, peer) in victims.iter().take(n) {
            self.close_subscriber(peer, stream, q);
            let key = self.s.key;
            self.s.tap.emit(key, Some(peer), || TapKind::Shed {
                stream,
                why: Shed::Saturated {
                    busy: (busy * 1000.0) as u64,
                    committed: committed as u64,
                    others: others as u64,
                    capacity: cap as u64,
                    excess: excess as u64,
                    count: n as u64,
                },
            });
        }
    }

    fn release_idle_interior(&mut self, now: Time) {
        let s = &mut self.s;
        if s.is_seed {
            return;
        }
        let Some(p) = s.push.as_mut() else {
            return;
        };
        let Some(i) = p.interior else {
            return;
        };
        let idle = s
            .streams
            .get(usize::from(i))
            .is_none_or(|st| st.subscribers.is_empty());
        if !idle {
            p.interior_idle_since = Time::NIL;
        } else if p.interior_idle_since.is_nil() {
            p.interior_idle_since = now;
        } else if now - p.interior_idle_since > INTERIOR_IDLE {
            p.interior = None;
            p.interior_idle_since = Time::NIL;
        }
    }

    /// Hybrid GC: forget expectations, releases and cancels behind the
    /// window.
    pub(crate) fn gc_push(&mut self, window_start: Bin) {
        let Some(p) = self.s.push.as_mut() else {
            return;
        };
        p.marked.reset_before(window_start);
        p.released.reset_before(window_start);
        for pc in self.channels.values_mut().filter_map(|c| c.push.as_mut()) {
            pc.cancelled.reset_before(window_start);
        }
    }

    /// Hybrid CANCEL: the peer doesn't want `b` from us, so a push of it is
    /// dropped. Bins behind our window or more than a live window past our
    /// newest chunk are ignored.
    pub(crate) fn push_cancelled(&mut self, peer: PeerKey, b: Bin, window_start: Bin) {
        let s = &self.s;
        let ok = crate::peer_bin_ok(b, s.window)
            && b.base_left() >= window_start
            && b.base_right().0 <= s.have_bin_max.0.saturating_add(s.live_window.0);
        if !ok {
            return;
        }
        if let Some(pc) = self.channels.get_mut(&peer).and_then(|c| c.push.as_mut()) {
            pc.cancelled.set(b);
        }
    }

    /// Hybrid `Reset`: a new stream starts; every subscription ends. A pin
    /// stays.
    pub(crate) fn reset_push(&mut self, now: Time) {
        let streams = usize::from(self.s.stream_count);
        if let Some(p) = self.s.push.as_mut() {
            let stats = p.stats;
            let pin = p.pin.take();
            *p = PushState::new(streams, now, p.next_report);
            p.stats = stats;
            p.pin = pin;
        }
        for c in self.channels.values_mut() {
            if c.push.is_some() {
                c.push = Some(PushChannel::new(streams));
            }
        }
    }

    /// Hybrid write-time check: a push that waited too long, or of a
    /// chunk the child announced or cancelled since, is dropped.
    pub(crate) fn push_unwanted(&mut self, peer: PeerKey, b: Bin, t: Time, now: Time) -> bool {
        let Some(p) = self.s.push.as_mut() else {
            return false;
        };
        let key = self.s.key;
        if t != Time::EPOCH && now - t > PUSH_STALE {
            p.stats.stale_drops = p.stats.stale_drops.saturating_add(1);
            self.s
                .tap
                .emit(key, Some(peer), || TapKind::PushStale { bin: b, queued: t });
            self.note_push(peer, b, t, seed::PushFate::Stale, now);
            return true;
        }
        let Some(c) = self.channels.get(&peer) else {
            return false;
        };
        if c.peer_have_bins.filled_at(b) {
            p.stats.suppressed = p.stats.suppressed.saturating_add(1);
            self.s
                .tap
                .emit(key, Some(peer), || TapKind::PushSuppressed { bin: b });
            return true;
        }
        if c.push.as_ref().is_some_and(|pc| pc.cancelled.filled_at(b)) {
            p.stats.suppressed = p.stats.suppressed.saturating_add(1);
            self.s
                .tap
                .emit(key, Some(peer), || TapKind::PushCancelled { bin: b });
            self.note_push(peer, b, t, seed::PushFate::Cancelled, now);
            return true;
        }
        false
    }

    /// Push/pull state for diagnostics and tests.
    pub(crate) fn push_stats(&self) -> PushStats {
        let s = &self.s;
        let mut out = PushStats {
            mode: s.mode,
            children: s.children(),
            ..Default::default()
        };
        if let Some(p) = s.push.as_ref() {
            let c = p.stats;
            out.parents = p.subs.iter().map(|x| x.open.map(|o| o.peer)).collect();
            out.parent_lag = p
                .subs
                .iter()
                .map(|x| x.open.map(|_| (x.lag.mean(), x.lag.deadline())))
                .collect();
            out.upload_rate = p.upload.rate;
            out.upload_busy_permille = (p.upload.busy * 1000.0) as u32;
            out.upload_capacity = p.upload.capacity().map_or(0, |c| c as u64);
            out.upload_queue_delay = p.upload.queue_delay;
            out.interior = if s.is_seed { None } else { p.interior };
            out.expected = c.expected;
            out.hits = c.hits;
            out.misses = c.misses;
            out.late = c.late;
            out.unsolicited = c.unsolicited;
            out.requests_sent = c.requests_sent;
            out.opens = c.opens;
            out.declined = c.declined;
            out.open_timeouts = c.open_timeouts;
            out.shed = c.shed;
            out.pruned = c.pruned;
            out.switched = c.switched;
            out.accepted = c.accepted;
            out.rejected = c.rejected;
            out.sheds_sent = c.sheds_sent;
            out.stale_drops = c.stale_drops;
            out.suppressed = c.suppressed;
            if let Some(a) = p.alloc.as_ref() {
                a.push_stats(&mut out);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_stream_bin_steps_to_the_stream() {
        // 4 streams: chunk i is stream i % 4, bin 2i.
        assert_eq!(next_stream_bin(Bin(0), 0, 4), Bin(8));
        assert_eq!(next_stream_bin(Bin(0), 1, 4), Bin(2));
        assert_eq!(next_stream_bin(Bin(6), 3, 4), Bin(14));
        assert_eq!(next_stream_bin(Bin(6), 0, 4), Bin(8));
        assert_eq!(next_stream_bin(Bin(10), 0, 1), Bin(12));
    }

    #[test]
    fn deadlines_follow_the_parent() {
        let t = Time::from_unix_millis(1_000);
        let mut l = LagEwma::default();
        assert_eq!(l.deadline(), DEADLINE_INITIAL);
        for _ in 0..50 {
            l.update(Duration::from_millis(40), t);
        }
        // A steady 40 ms: the peak (40) times 1.25, plus the margin.
        assert_eq!(l.deadline(), Duration::from_millis(100));

        // A sawtooth (a munro's pushes leaving in a burst, 0..320 ms every
        // 500 ms): the deadline covers the top of the tooth.
        let mut l = LagEwma::default();
        let mut now = t;
        for _ in 0..20 {
            for i in 0..64i64 {
                l.update(
                    Duration::from_millis(i * 5),
                    now + Duration::from_millis(i * 5),
                );
            }
            now += Duration::from_millis(500);
        }
        assert!(
            l.deadline() >= Duration::from_millis(400),
            "{}",
            l.deadline()
        );

        for _ in 0..50 {
            l.update(Duration::from_secs(5), now);
        }
        assert_eq!(l.deadline(), DEADLINE_MAX);
    }

    #[test]
    fn capacity_is_the_rate_while_busy() {
        let u = UploadInfo {
            rate: 1000,
            busy: 0.5,
            ..Default::default()
        };
        assert_eq!(u.capacity(), Some(2000.0));
        let idle = UploadInfo {
            rate: 1000,
            busy: 0.0,
            ..Default::default()
        };
        assert_eq!(idle.capacity(), Some(1000.0 / MIN_BUSY));
        assert_eq!(UploadInfo::default().capacity(), None);
    }

    #[test]
    fn a_saturated_link_takes_the_smaller_of_hint_and_measurement() {
        let busy = UploadInfo {
            rate: 1000,
            busy: 0.5,
            ..Default::default()
        };
        let hinted = |hint| UploadInfo {
            hint: Some(hint),
            ..busy
        };
        assert_eq!(hinted(1500).capacity(), Some(1500.0));
        assert_eq!(hinted(5000).capacity(), Some(2000.0));
        // A link that isn't saturated shows only a lower bound.
        let idle = UploadInfo {
            busy: 0.1,
            hint: Some(5000),
            ..busy
        };
        assert_eq!(idle.capacity(), Some(5000.0));
        // Before anything was sent, the hint alone.
        let unsent = UploadInfo {
            hint: Some(1500),
            ..UploadInfo::default()
        };
        assert_eq!(unsent.capacity(), Some(1500.0));
    }

    #[test]
    fn misses_prune() {
        let t = |ms: i64| Time::from_unix_millis(1_000_000 + ms);
        let open = |s: &mut Subscription| {
            s.on_open(Parent {
                peer: PeerKey::default(),
                start: Bin(0),
                since: t(0),
                hint: LagEwma::default(),
            })
        };
        // A silent parent: pruned once nothing came for PRUNE_SILENCE.
        let mut s = Subscription::default();
        open(&mut s);
        for _ in 0..20 {
            s.expected += 1;
            s.on_miss();
        }
        assert!(s.prune_rule(t(900)).is_none());
        assert!(s.prune_rule(t(1000)).is_some());

        // A lost frame (a burst of 14 misses) between hits doesn't prune,
        // and neither do scattered misses (1 in 25).
        let mut s = Subscription::default();
        open(&mut s);
        for i in 0..2000i64 {
            s.expected += 1;
            if i % 25 == 0 || (500..514).contains(&i) {
                s.on_miss();
            } else {
                s.on_hit(t(i));
            }
            assert!(s.prune_rule(t(i)).is_none(), "{i}");
        }

        // Missing every other chunk does, once the parent had its chance.
        let mut s = Subscription::default();
        open(&mut s);
        let mut pruned = None;
        for i in 0..400i64 {
            s.expected += 1;
            if i % 2 == 0 {
                s.on_miss();
            } else {
                s.on_hit(t(i * 10));
            }
            if pruned.is_none() && s.prune_rule(t(i * 10)).is_some() {
                pruned = Some(i * 10);
            }
        }
        assert_eq!(pruned, Some(2000));
    }
}
