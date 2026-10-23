// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What a hybrid seed pushes to whom ([`SeedPolicy`]).
//!
//! A seed receives no data, so reciprocity weighs its peers alike, and
//! every neighbour's graft prefers it: it announces first. Its admission
//! alone therefore places its pushes, by accepting, declining and closing
//! STREAM_REQUESTs; an adaptive seed's only message of its own asks its
//! neighbours for reports.
//!
//! # Adaptive allocation
//!
//! The seed's measured upload pays for some whole copies of the stream. It
//! pushes them to that many *holders*, the neighbours with the shortest
//! round trip to it, and declines everyone else: a holder's requests are
//! accepted for every stream, so its copy is whole. Near holders have each
//! chunk soonest and relay it from about where the seed would, multiplying
//! its uplink; a whole copy at few peers keeps the trees shallow. The
//! round trip includes the last mile, so mobile and satellite links rank
//! behind wired ones nearby.
//!
//! - **Budget.** K stream slots (one subscriber of one stream each) are
//!   what [`ALLOC_UTILIZATION`] of the measured upload capacity carries at
//!   a subscriber's cost, once the store rate has been measured for
//!   [`ALLOC_MEASURE`]; they pay for K / streams copies, or K of the
//!   streams at one holder when K is under one copy (the rest go to pull).
//!   It falls only once it is [`OVER_BUDGET`] too high. Pull answers take
//!   what pushes leave of the uplink, so neither they nor the time the
//!   uplink is busy with them bound it: [`SeedQos::ByAllocation`] puts
//!   them behind the pushes.
//! - **Window.** The copies are at most a window, and at most the
//!   budget's copies once it is measured. The window starts at one copy.
//!   Until the budget is measured and the window first cut, each request
//!   raises it to what [`ALLOC_UTILIZATION`] of the measured capacity pays
//!   for at the store rate since the first chunk, which reads high while
//!   the first munros arrive: the first wave, every neighbour asking for
//!   every stream, is what places whole copies.
//!
//!   From then on the window follows the pushes. Each check reads, per
//!   holder, the shortest wait of the pushes written to it or dropped
//!   because it cancelled them (they waited all the same) since the last
//!   check, which stays long only while its queue never empties between
//!   munro bursts, and its stale drops; pushes queued before the window
//!   last changed don't count. A holder is behind past
//!   [`PUSH_WAIT_TARGET`], and overloaded past [`PUSH_WAIT_OVERLOAD`] or
//!   after a stale drop.
//!   - While the uplink is busy [`SHED_UTILIZATION`] of the time and
//!     more than half the holders pushed to are overloaded, the window
//!     halves (at least one copy), and doubling stops there (at two copies
//!     at least).
//!   - With no majority behind and pushes written, it grows, provided it
//!     is under twice the holders with a subscription or the uplink has
//!     idle time: so a wave of neighbours asking at once can at most
//!     double a load that kept up. It doubles until the first cut, then
//!     takes one copy per [`WINDOW_PROBE`].
//!
//!   A holder that is slow by its own path, or stops reading, leaves the
//!   window alone while the uplink has room or most holders keep up. A
//!   child's STREAM_CANCEL is its own decision and counts for nothing.
//!   Without [`SeedQos::ByAllocation`] a holder's share of the uplink is
//!   the same whatever the copies, so its waits tell nothing about them:
//!   the window is the first wave's size, then the budget's copies.
//! - **Holders.** A neighbour that asks while there are fewer holders than
//!   copies becomes one, so the first to ask hold the stream. One without
//!   a subscription is pending: it keeps its copy for [`HOLDER_GRACE`]
//!   after it joined or last asked, then gives it back (a holder that gave
//!   up every subscription frees only its own copy). At each check the
//!   farthest subscribed holders past the copies go, and free copies go to
//!   the nearest neighbours that asked within the grace. With none free,
//!   the nearest of those, if its round trip is [`NEAR_MARGIN`] (or a
//!   tenth) shorter than the farthest holder's, is given one, which that
//!   holder gives up once the new one subscribed: one move per
//!   [`ALLOC_CHECK_INTERVAL`], so jitter doesn't move the allocation and
//!   no copy goes empty. A neighbour that stops being a holder loses all
//!   its subscriptions (STREAM_CLOSE).
//! - **Silent holders.** A holder announces what it is pushed. Each check
//!   samples the first push written to each holder since the last; at the
//!   check after next, at least [`ANNOUNCE_TIMEOUT`] later, the holder's
//!   HAVEs must cover it. One that didn't at [`SILENT_CHECKS`] checks in a
//!   row is barred: its copy goes, and it holds none for [`HOLDER_BAR`],
//!   doubling with each bar. Both the pushes and the HAVEs are the seed's
//!   own observations.
//! - **Served holders.** A holder must also serve, which only other
//!   nodes' observations can show. The seed asks every neighbour for
//!   reports every [`ASK_INTERVAL`]; a viewer's report says, per parent and
//!   stream, how many pushes came on time, came late or missed. An entry
//!   from reporter R about P on stream s counts only if the seed's own
//!   observations allow it:
//!   1. R takes s from someone other than the seed, and P takes s from the
//!      seed: what is judged is P's relay of the seed's copy. A report
//!      about R itself is invalid.
//!   2. R announces s ([`CANDIDATE_FRESHNESS`](super::CANDIDATE_FRESHNESS)),
//!      so a free rider's reports count for nothing.
//!   3. R's HAVE lag on s is at least P's less P's round trip and
//!      [`REPORT_JITTER`]: R can't announce what P relays before P could
//!      have, whatever the paths.
//!   4. An entry with more misses than hits shows them in R's own HAVE
//!      timing: R's lag on s is at least P's plus
//!      [`DEADLINE_MARGIN`](super::DEADLINE_MARGIN), as a child's is that
//!      pulled what its parent missed.
//!
//!   Each reporter's entries about a subject are summed over the current
//!   [`REPORT_BUCKET`] and the one before, and count as one vote once it
//!   expected [`MIN_REPORTED`] pushes. Past [`REPORT_PROBATION`] after it
//!   joined, a holder that [`REPORT_QUORUM`] reporters voted on is barred
//!   as a silent one is when most of them got less than
//!   [`UNSERVED_SHARE`] of what they expected; most getting
//!   [`SERVED_SHARE`] as hits proves it. The seed blames a holder only
//!   after [`KEPT_CHECKS`] checks in a row at which its own pushes to it
//!   kept up (none stale, one written or cancelled within
//!   [`PUSH_WAIT_OVERLOAD`]), and drops the reports about it at a check
//!   that didn't: children miss what the seed was slow to send. A verdict
//!   is a majority over other nodes' observations, each checked against
//!   the seed's own. A holder no reporter votes on keeps its copy.
//! - **Round trips** are each neighbour's smallest ping round trip over
//!   the last one or two [`RTT_WINDOW`]s, which leaves out queueing.
//! - With room for every neighbour's every stream, everything is accepted.
//!
//! # Pins and uplink shares
//!
//! - **A pin** (`Runner::pin_seed_allocation`) fixes the peers each stream
//!   is pushed to: their requests are accepted, everyone else's declined
//!   and every other subscriber closed, and nothing is shed.
//! - **Uplink shares.** With [`SeedQos::ByAllocation`] the runner weights
//!   each peer at least in proportion to the streams pushed to it, and the
//!   peers it pushes nothing to at the minimum: pushes leave at the rate
//!   each holder needs, and pull answers take what they leave. The weights
//!   follow each change of the seed's subscribers in the runner call that
//!   made it.
//!
//! # The seed's view
//!
//! [`SeedAllocationStats`] gives, per neighbour, its HAVE lag per stream
//! after publishing, whether it is barred, and its reporters and their
//! verdict at the latest check. A seed samples each stream once per
//! signature unit, at the unit's last chunk of it: a unit is published at
//! once, so that chunk tells when the neighbour had the unit's share of
//! the stream whole.
//!
//! # Bounds
//!
//! Per channel a fixed record, plus two tallies per stream of the
//! subjects its reports named. A report costs O(entries × channels), and a
//! reporter's entries past [`report_entry_limit`] per check are invalid. A
//! check costs O(channels × streams × log channels) plus the subscribers;
//! an admission O(channels).

use std::collections::{BTreeMap, BTreeSet};

use strims_base::time::{Duration, Time};
use strims_binmap::Bin;

use super::{DEADLINE_MARGIN, PRUNE_MISS_RATE, PRUNE_SILENT_MISSES, PushChannel, SHED_UTILIZATION};
use crate::codec::{Announce, AnnounceEntry, Stream};
use crate::controller::RequestController;
use crate::options::{SeedAllocation, SeedQos};
use crate::runner::{MAX_PEER_QOS_WEIGHT, MIN_PEER_QOS_WEIGHT, PeerKey};
use crate::scheduler::PeerTaskQueue;
use crate::scheduler_peer::PeerSwarmScheduler;
use crate::tap::TapKind;

#[cfg(doc)]
use crate::SeedPolicy;

/// How often an adaptive seed re-evaluates its allocation.
pub const ALLOC_CHECK_INTERVAL: Duration = Duration::SECOND;
/// The share of the measured upload capacity the budget spends on pushes.
pub const ALLOC_UTILIZATION: f64 = 0.7;
/// How long the store rate is measured before the budget follows it.
pub const ALLOC_MEASURE: Duration = Duration::from_secs(2);
/// A holder whose pushes all waited longer than this between two checks
/// is behind: its queue never emptied.
pub const PUSH_WAIT_TARGET: Duration = Duration::from_millis(100);
/// ... and past this, or after a stale drop, overloaded: its pushes near
/// [`PUSH_STALE`](super::PUSH_STALE).
pub const PUSH_WAIT_OVERLOAD: Duration = Duration::from_millis(500);
/// After its first cut, the window grows by one copy at most this often:
/// each holder that leaves costs its children a backoff and a graft.
pub const WINDOW_PROBE: Duration = Duration::from_secs(10);
/// A holder with no subscription keeps its copy this long after it joined
/// or last asked, and a neighbour that asked this recently may be given a
/// free copy: a declined neighbour asks again after its
/// [`DECLINE_BACKOFF`](super::DECLINE_BACKOFF).
pub const HOLDER_GRACE: Duration = Duration::from_secs(4);
/// How far the budget must fall before it is lowered.
pub const OVER_BUDGET: f64 = 0.1;
/// How much nearer (or a tenth, if more) a neighbour must be to displace
/// a holder.
pub const NEAR_MARGIN: Duration = Duration::from_millis(5);
/// A round trip is the smallest over the current window of this length
/// and the one before.
pub const RTT_WINDOW: Duration = Duration::from_secs(10);
/// A push written to a holder is announced within this long: about ten
/// times a WAN holder's HAVE lag.
pub const ANNOUNCE_TIMEOUT: Duration = Duration::SECOND;
/// A holder that hasn't announced its sampled push at this many checks in
/// a row is silent.
pub const SILENT_CHECKS: u8 = 3;
/// A neighbour whose copy was taken back for not serving holds none for
/// this long, doubling with each bar up to [`MAX_BAR_DOUBLINGS`] times.
pub const HOLDER_BAR: Duration = Duration::from_secs(30);
/// See [`HOLDER_BAR`].
pub const MAX_BAR_DOUBLINGS: u32 = 3;
/// An adaptive seed asks each neighbour for reports this often: a lost
/// ask lapses within one [`REPORT_LEASE`](super::REPORT_LEASE).
pub const ASK_INTERVAL: Duration = Duration::from_secs(5);
/// A reporter's entries about a subject are summed over the current
/// bucket of this length and the one before.
pub const REPORT_BUCKET: Duration = Duration::from_secs(5);
/// A holder is judged by reports once it held its copy this long:
/// children graft onto a new holder over a few parent checks.
pub const REPORT_PROBATION: Duration = Duration::from_secs(3);
/// The reporters a verdict takes.
pub const REPORT_QUORUM: usize = 2;
/// A reporter votes on a subject once it expected this many pushes from
/// it: every child that pruned it for silence did.
pub const MIN_REPORTED: u64 = PRUNE_SILENT_MISSES as u64;
/// A reporter saw its parent serve when this share of the pushes it
/// expected were hits, the share a child keeps a parent at ...
pub const SERVED_SHARE: f64 = 1.0 - PRUNE_MISS_RATE;
/// ... and saw it serve nothing when less than this share arrived at all.
pub const UNSERVED_SHARE: f64 = 0.25;
/// The slack on a reporter's HAVE lag against its parent's: about twice a
/// WAN end's jitter.
pub const REPORT_JITTER: Duration = Duration::from_millis(40);
/// The checks in a row at which the seed's pushes to a holder kept up
/// before its reporters' misses count: a miss is tallied up to a deadline
/// after the push was due and reported up to a report interval later.
pub const KEPT_CHECKS: u32 = 3;

/// The report entries a neighbour may send per check interval: twice
/// the most an honest report holds, so a late report and the next one
/// can arrive together.
pub fn report_entry_limit(streams: u16) -> u32 {
    4 * u32::from(streams)
}

/// What became of a push at write time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PushFate {
    Written,
    /// Dropped for waiting longer than [`PUSH_STALE`](super::PUSH_STALE).
    Stale,
    /// Dropped because the child cancelled the chunk.
    Cancelled,
}

/// A seed's record of one neighbour.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Holder {
    /// It holds a copy.
    holds: bool,
    /// When it last asked for a stream, or became a holder.
    active: Time,
    /// The smallest ping round trip in the current [`RTT_WINDOW`] (its
    /// index) and in the one before.
    window: i64,
    rtt: Option<Duration>,
    rtt_before: Option<Duration>,
    /// Since the last check, of the pushes queued since the window last
    /// changed: the shortest wait of one written or cancelled, and whether
    /// one went stale.
    wait: Option<Duration>,
    stale: bool,
    /// The first push written in the current check interval, and the one
    /// of the interval before: its bin, and when.
    sample: Option<(Bin, Time)>,
    due: Option<(Bin, Time)>,
    /// Checks in a row at which the due push wasn't announced.
    silent: u8,
    /// No copy before then; and how often it was barred.
    barred_until: Time,
    bars: u32,
    /// When it last became a holder.
    joined: Time,
    /// Since the last check, whatever the window: the shortest wait of a
    /// push written to it or cancelled by it, and whether one went stale.
    fed: Option<Duration>,
    starved: bool,
    /// Whether the pushes to it kept up at each recent check, the latest
    /// in the lowest bit.
    kept: u8,
    /// At the latest check: the reporters that voted on it, whether most
    /// saw it serve (`None`: too few to tell), and their median push
    /// delay.
    reporters: u8,
    served: Option<bool>,
    delay: Option<Duration>,
    /// Its report entries since the last check.
    entries: u32,
    /// When it is asked for reports next.
    ask_at: Time,
    /// What became of its report entries.
    reports: ReportCounters,
}

impl Holder {
    /// A ping round trip.
    fn ping(&mut self, rtt: Duration, now: Time) {
        let w = now.unix_nanos() / RTT_WINDOW.as_nanos();
        if w != self.window {
            self.rtt_before = if w == self.window.saturating_add(1) {
                self.rtt
            } else {
                None
            };
            self.rtt = None;
            self.window = w;
        }
        self.rtt = Some(self.rtt.map_or(rtt, |r| r.min(rtt)));
    }

    fn barred(&self, now: Time) -> bool {
        now < self.barred_until
    }

    /// Bars it from holding a copy, for longer each time.
    fn bar(&mut self, now: Time) {
        self.barred_until = now + HOLDER_BAR * (1 << self.bars.min(MAX_BAR_DOUBLINGS));
        self.bars = self.bars.saturating_add(1);
        self.silent = 0;
    }

    /// It becomes a holder.
    fn join(&mut self, now: Time) {
        self.holds = true;
        self.active = now;
        self.joined = now;
    }

    /// Its pushes kept up at the last [`KEPT_CHECKS`] checks.
    fn kept_up(&self) -> bool {
        let mask = (1u8 << KEPT_CHECKS) - 1;
        self.kept & mask == mask
    }

    /// The smallest round trip over the current window and the one before.
    fn min_rtt(&self, now: Time) -> Option<Duration> {
        let w = now.unix_nanos() / RTT_WINDOW.as_nanos();
        if w == self.window {
            [self.rtt, self.rtt_before].into_iter().flatten().min()
        } else if w == self.window.saturating_add(1) {
            self.rtt
        } else {
            None
        }
    }
}

/// A reporter's vote on a subject: pushes it expected (hits and misses),
/// got on time, and got at all (hits and late), and the hits' delays in
/// milliseconds, summed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Votes {
    expected: u64,
    hits: u64,
    pushed: u64,
    delay_ms: u64,
}

impl Votes {
    fn add(&mut self, o: Votes) {
        self.expected = self.expected.saturating_add(o.expected);
        self.hits = self.hits.saturating_add(o.hits);
        self.pushed = self.pushed.saturating_add(o.pushed);
        self.delay_ms = self.delay_ms.saturating_add(o.delay_ms);
    }

    fn from_entry(e: &AnnounceEntry<'_>) -> Votes {
        let ms = u64::try_from(e.delay.as_millis()).unwrap_or(0);
        Votes {
            expected: e.hits.saturating_add(e.misses),
            hits: e.hits,
            pushed: e.hits.saturating_add(e.late),
            delay_ms: e.hits.saturating_mul(ms),
        }
    }
}

/// What a reporter said about one subject on one stream, in the current
/// [`REPORT_BUCKET`] (its index) and the one before.
#[derive(Clone, Copy, Debug)]
struct HeardSlot {
    subject: PeerKey,
    bucket: i64,
    current: Votes,
    before: Votes,
    at: Time,
}

impl HeardSlot {
    fn add(&mut self, v: Votes, now: Time) {
        let b = now.unix_nanos() / REPORT_BUCKET.as_nanos();
        if b != self.bucket {
            self.before = if b == self.bucket.saturating_add(1) {
                self.current
            } else {
                Votes::default()
            };
            self.current = Votes::default();
            self.bucket = b;
        }
        self.current.add(v);
        self.at = now;
    }

    fn window(&self, now: Time) -> Votes {
        let b = now.unix_nanos() / REPORT_BUCKET.as_nanos();
        let mut v = Votes::default();
        if b == self.bucket {
            v.add(self.current);
            v.add(self.before);
        } else if b == self.bucket.saturating_add(1) {
            v.add(self.current);
        }
        v
    }
}

/// An adaptive seed's tallies of one neighbour's reports. Bound: two
/// subjects per stream, allocated at its first counted entry; a third
/// replaces the one reported least recently.
#[derive(Clone, Debug, Default)]
pub(crate) struct Heard {
    slots: Vec<[Option<HeardSlot>; 2]>,
}

impl Heard {
    fn add(&mut self, streams: usize, stream: Stream, subject: PeerKey, v: Votes, now: Time) {
        if self.slots.is_empty() {
            self.slots = vec![[None; 2]; streams];
        }
        let Some(slots) = self.slots.get_mut(usize::from(stream)) else {
            return;
        };
        let i = slots
            .iter()
            .position(|x| x.is_some_and(|x| x.subject == subject))
            .or_else(|| slots.iter().position(Option::is_none))
            .unwrap_or_else(|| {
                let at = |i: usize| {
                    slots
                        .get(i)
                        .copied()
                        .flatten()
                        .map_or(Time::EPOCH, |x| x.at)
                };
                usize::from(at(1) < at(0))
            });
        let Some(slot) = slots.get_mut(i) else {
            return;
        };
        if slot.is_none_or(|x| x.subject != subject) {
            *slot = Some(HeardSlot {
                subject,
                bucket: 0,
                current: Votes::default(),
                before: Votes::default(),
                at: now,
            });
        }
        if let Some(x) = slot.as_mut() {
            x.add(v, now);
        }
    }

    /// Forgets what it said about `subject`.
    fn forget(&mut self, subject: PeerKey) {
        for x in self.slots.iter_mut().flatten() {
            if x.is_some_and(|x| x.subject == subject) {
                *x = None;
            }
        }
    }
}

/// One check interval's evidence for the [`Window`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Signals {
    /// Holders with a push written or dropped as stale, and those of them
    /// behind and overloaded.
    pub(crate) pushed: usize,
    pub(crate) behind: usize,
    pub(crate) overloaded: usize,
    /// Holders with a subscription.
    pub(crate) in_use: usize,
    /// The uplink was busy at least [`SHED_UTILIZATION`] of the time.
    pub(crate) saturated: bool,
}

/// The slow start's window; see the module docs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Window {
    /// Whole copies, at least one.
    copies: usize,
    /// Doubling stops here: `usize::MAX` until the first cut.
    ssthresh: usize,
    /// When the window last changed: earlier pushes don't count.
    since: Time,
    /// Past the first cut, when it may grow by one copy next.
    probe_at: Time,
}

impl Window {
    fn new(now: Time) -> Window {
        Window {
            copies: 1,
            ssthresh: usize::MAX,
            since: now,
            probe_at: Time::NIL,
        }
    }

    /// A check's update with the interval's signals, to at most `cap`
    /// copies. True if the window changed.
    fn update(&mut self, s: Signals, cap: usize, now: Time) -> bool {
        let before = self.copies;
        let majority = s.behind.saturating_mul(2) > s.pushed;
        if s.saturated && s.overloaded.saturating_mul(2) > s.pushed {
            // At one copy the cut changes nothing, so doubling may still
            // take the next step.
            self.ssthresh = (self.copies / 2).max(2);
            self.copies = (self.copies / 2).max(1);
            self.probe_at = now + WINDOW_PROBE;
        } else if !majority
            && s.pushed > 0
            && (self.copies < s.in_use.saturating_mul(2) || !s.saturated)
        {
            if self.copies < self.ssthresh {
                self.copies = self.copies.saturating_mul(2).min(self.ssthresh);
            } else if now >= self.probe_at {
                self.copies = self.copies.saturating_add(1);
                self.probe_at = now + WINDOW_PROBE;
            }
        }
        self.set(self.copies.min(cap), before, now)
    }

    /// Without uplink shares by allocation: the window is `cap` copies. A
    /// holder's share is then the same whatever the copies, so its waits
    /// tell nothing about them.
    fn track(&mut self, cap: usize, now: Time) -> bool {
        let before = self.copies;
        self.set(cap, before, now)
    }

    fn set(&mut self, copies: usize, before: usize, now: Time) -> bool {
        self.copies = copies.max(1);
        let changed = self.copies != before;
        if changed {
            self.since = now;
        }
        changed
    }
}

/// An adaptive seed's state; see the module docs.
#[derive(Debug)]
pub(crate) struct Allocator {
    next_check: Time,
    /// When the store rate was first seen (`NIL` before).
    measuring_since: Time,
    /// Once measured, the budget in stream slots.
    measured: bool,
    budget: usize,
    /// The copies in effect, and the streams a holder may take (fewer
    /// than all only under one copy).
    copies: usize,
    streams: usize,
    window: Window,
    /// Room for every neighbour's every stream.
    unconstrained: bool,
    stats: AllocCounters,
    /// The first wave: the most copies the window opened before the
    /// budget was measured, and the capacity reading (bytes per second)
    /// that sized them.
    first_wave: usize,
    first_wave_capacity: u64,
}

impl Allocator {
    fn new(now: Time) -> Allocator {
        let window = Window::new(now);
        Allocator {
            next_check: now,
            measuring_since: Time::NIL,
            measured: false,
            budget: 0,
            copies: window.copies,
            streams: usize::MAX,
            window,
            unconstrained: false,
            stats: AllocCounters::default(),
            first_wave: 0,
            first_wave_capacity: 0,
        }
    }

    /// Fills `out`'s seed fields.
    pub(super) fn push_stats(&self, out: &mut super::PushStats) {
        out.seed_budget = self.budget;
        out.seed_window = self.window.copies;
        out.seed_copies = self.copies;
        out.seed_first_wave = self.first_wave;
        out.seed_first_wave_capacity = self.first_wave_capacity;
    }
}

/// An adaptive seed's cumulative counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocCounters {
    /// Requests accepted, and declined.
    pub accepted: u64,
    pub declined: u64,
    /// Neighbours that became holders, and stopped being holders.
    pub joined: u64,
    pub left: u64,
    /// Holders barred for not announcing their pushes, and because most
    /// of their reporters saw them push nothing.
    pub silenced: u64,
    pub unserved: u64,
    /// What became of the report entries.
    pub reports: ReportCounters,
}

/// What became of report entries: tallied, or counting for nothing and
/// why (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReportCounters {
    pub tallied: u64,
    /// The parent isn't a neighbour, or two neighbours share its id.
    pub unknown: u64,
    /// Invalid: the parent is the reporter, the stream is past the count,
    /// or the entry is past [`report_entry_limit`].
    pub invalid: u64,
    /// The reporter takes the stream from the seed.
    pub reporter_fed: u64,
    /// The parent doesn't take the stream from the seed.
    pub subject_unfed: u64,
    /// The reporter doesn't announce the stream.
    pub reporter_silent: u64,
    /// The reporter announces the stream sooner than its parent could
    /// relay it.
    pub too_early: u64,
    /// More misses than hits, and the reporter's HAVEs don't show them.
    pub misses_unseen: u64,
}

impl ReportCounters {
    fn add(&mut self, o: &ReportCounters) {
        let ReportCounters {
            tallied,
            unknown,
            invalid,
            reporter_fed,
            subject_unfed,
            reporter_silent,
            too_early,
            misses_unseen,
        } = *o;
        for (a, b) in [
            (&mut self.tallied, tallied),
            (&mut self.unknown, unknown),
            (&mut self.invalid, invalid),
            (&mut self.reporter_fed, reporter_fed),
            (&mut self.subject_unfed, subject_unfed),
            (&mut self.reporter_silent, reporter_silent),
            (&mut self.too_early, too_early),
            (&mut self.misses_unseen, misses_unseen),
        ] {
            *a = a.saturating_add(b);
        }
    }
}

/// Why a report entry counts for nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unheard {
    ReporterFed,
    SubjectUnfed,
    ReporterSilent,
    TooEarly,
    MissesUnseen,
}

impl Unheard {
    fn count(self, c: &mut ReportCounters) {
        let n = match self {
            Unheard::ReporterFed => &mut c.reporter_fed,
            Unheard::SubjectUnfed => &mut c.subject_unfed,
            Unheard::ReporterSilent => &mut c.reporter_silent,
            Unheard::TooEarly => &mut c.too_early,
            Unheard::MissesUnseen => &mut c.misses_unseen,
        };
        *n = n.saturating_add(1);
    }
}

/// A seed's view of its neighbours, for diagnostics and tests.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SeedAllocationStats {
    pub neighbours: Vec<SeedNeighbour>,
    /// An adaptive seed's latest check (zero otherwise): the budget in
    /// stream slots (zero until measured), the window and the copies in
    /// effect, and the counters.
    pub budget: usize,
    pub window: usize,
    pub copies: usize,
    pub counters: AllocCounters,
}

/// One neighbour of a seed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedNeighbour {
    pub peer: PeerKey,
    /// The streams pushed to it.
    pub streams: Vec<Stream>,
    /// Per stream, its mean HAVE lag after publishing, if it announced the
    /// stream within [`CANDIDATE_FRESHNESS`](super::CANDIDATE_FRESHNESS).
    pub lags: Vec<Option<Duration>>,
    /// The channel's round trip, and its smallest ping round trip lately.
    pub rtt: Duration,
    pub min_rtt: Option<Duration>,
    /// An adaptive seed's holder.
    pub holder: bool,
    /// Barred from holding a copy.
    pub barred: bool,
    /// At an adaptive seed's latest check: the reporters that voted on it,
    /// whether most saw it serve (`None`: too few to tell), and their
    /// median push delay; whether the seed's pushes to it kept up.
    pub reporters: usize,
    pub served: Option<bool>,
    pub report_delay: Option<Duration>,
    pub kept_up: bool,
    /// What became of its own report entries.
    pub reports: ReportCounters,
}

/// A ping round trip to the channel's peer, at a seed.
pub(crate) fn ping(pc: &mut Option<PushChannel>, rtt: Duration, now: Time) {
    if let Some(pc) = pc.as_mut() {
        pc.holder.ping(rtt, now);
    }
}

impl PeerSwarmScheduler {
    /// A seed allocating by its own measurements: no pin.
    pub(crate) fn adaptive_seed(&self) -> bool {
        self.s.is_seed
            && self.s.seed.allocation != SeedAllocation::Admission
            && self.s.push.as_ref().is_some_and(|p| p.pin.is_none())
    }

    /// An adaptive seed's admission of `peer`'s new request for a stream.
    pub(crate) fn grant(&mut self, peer: PeerKey, now: Time) -> bool {
        let holders = self.holders();
        let held = self
            .s
            .streams
            .iter()
            .filter(|st| st.subscribers.iter().any(|x| x.channel == peer))
            .count();
        let Some((holds, barred)) = self
            .channels
            .get_mut(&peer)
            .and_then(|c| c.push.as_mut())
            .map(|pc| {
                pc.holder.active = now;
                (pc.holder.holds, pc.holder.barred(now))
            })
        else {
            return false;
        };
        let early = self.s.early_stream_rate(now);
        let streams = usize::from(self.s.stream_count.max(1));
        let neighbours = self
            .channels
            .values()
            .filter(|c| c.handshake_received)
            .count();
        let Some(p) = self.s.push.as_mut() else {
            return false;
        };
        let cap = p.upload.capacity();
        let a = p.alloc.get_or_insert_with(|| Allocator::new(now));
        // The first wave's size, from the early measurements.
        if !a.measured
            && a.window.ssthresh == usize::MAX
            && let (Some(cap), Some(rate)) = (cap, early)
        {
            let paid =
                ((cap * ALLOC_UTILIZATION - p.upload.others).max(0.0) / rate) as usize / streams;
            let w = paid.min(neighbours).max(a.window.copies);
            if w > a.window.copies {
                a.window.copies = w;
                a.window.since = now;
                a.copies = w;
                a.first_wave = w;
                a.first_wave_capacity = cap as u64;
            }
        }
        let holder = !barred && (holds || a.unconstrained || holders < a.copies);
        if holder
            && !holds
            && let Some(h) = self
                .channels
                .get_mut(&peer)
                .and_then(|c| c.push.as_mut())
                .map(|pc| &mut pc.holder)
        {
            h.join(now);
            let key = self.s.key;
            self.s.tap.emit(key, Some(peer), || TapKind::HolderJoined);
            if let Some(a) = self.s.push.as_mut().and_then(|p| p.alloc.as_mut()) {
                a.stats.joined = a.stats.joined.saturating_add(1);
            }
        }
        let Some(a) = self.s.push.as_mut().and_then(|p| p.alloc.as_mut()) else {
            return false;
        };
        let ok = holder && held < a.streams;
        let n = if ok {
            &mut a.stats.accepted
        } else {
            &mut a.stats.declined
        };
        *n = n.saturating_add(1);
        ok
    }

    /// An adaptive seed's push of `bin` to `peer`, queued at `queued`, met
    /// `fate` at `now`. A cancelled push's wait counts as a written one's:
    /// it waited in our queue all the same, though the peer never got it.
    pub(crate) fn note_push(
        &mut self,
        peer: PeerKey,
        bin: Bin,
        queued: Time,
        fate: PushFate,
        now: Time,
    ) {
        let Some(since) = self
            .s
            .push
            .as_ref()
            .and_then(|p| p.alloc.as_ref())
            .map(|a| a.window.since)
        else {
            return;
        };
        let Some(h) = self
            .channels
            .get_mut(&peer)
            .and_then(|c| c.push.as_mut())
            .map(|pc| &mut pc.holder)
        else {
            return;
        };
        if fate == PushFate::Written && h.sample.is_none() {
            h.sample = Some((bin, now));
        }
        let stale = fate == PushFate::Stale;
        if stale {
            h.starved = true;
        } else {
            let w = now - queued;
            h.fed = Some(h.fed.map_or(w, |x| x.min(w)));
        }
        if queued < since {
            return;
        }
        if stale {
            h.stale = true;
        } else {
            let w = now - queued;
            h.wait = Some(h.wait.map_or(w, |x| x.min(w)));
        }
    }

    /// The window's signals since the last check; they start over.
    fn window_signals(&mut self, saturated: bool) -> Signals {
        let in_use: BTreeSet<PeerKey> = self
            .s
            .streams
            .iter()
            .flat_map(|st| st.subscribers.iter().map(|x| x.channel))
            .collect();
        let mut s = Signals {
            saturated,
            ..Signals::default()
        };
        for (peer, c) in &mut self.channels {
            let Some(h) = c.push.as_mut().map(|pc| &mut pc.holder) else {
                continue;
            };
            let (wait, stale) = (h.wait.take(), std::mem::take(&mut h.stale));
            let (fed, starved) = (h.fed.take(), std::mem::take(&mut h.starved));
            h.entries = 0;
            if !h.holds {
                h.kept = 0;
                continue;
            }
            let kept = !starved && fed.is_some_and(|w| w <= PUSH_WAIT_OVERLOAD);
            h.kept = (h.kept << 1) | u8::from(kept);
            if in_use.contains(peer) {
                s.in_use += 1;
            }
            if wait.is_some() || stale {
                s.pushed += 1;
                if stale || wait.is_some_and(|w| w > PUSH_WAIT_TARGET) {
                    s.behind += 1;
                }
                if stale || wait.is_some_and(|w| w > PUSH_WAIT_OVERLOAD) {
                    s.overloaded += 1;
                }
            }
        }
        s
    }

    /// The neighbours holding a copy.
    fn holders(&self) -> usize {
        self.channels
            .values()
            .filter(|c| c.push.as_ref().is_some_and(|pc| pc.holder.holds))
            .count()
    }

    /// An adaptive seed's check: the budget and the window, then the
    /// holders.
    pub(crate) fn check_allocation(&mut self, now: Time, q: &mut impl PeerTaskQueue) {
        if !self.adaptive_seed() {
            return;
        }
        let rate = self.s.stream_rate(now);
        let streams = usize::from(self.s.stream_count);
        let neighbours = self
            .channels
            .values()
            .filter(|c| c.handshake_received)
            .count();
        let Some(p) = self.s.push.as_mut() else {
            return;
        };
        let upload = p.upload;
        let a = p.alloc.get_or_insert_with(|| Allocator::new(now));
        if now < a.next_check {
            return;
        }
        a.next_check = now + ALLOC_CHECK_INTERVAL;
        let signals = self.window_signals(upload.busy >= SHED_UTILIZATION);
        let early = self.s.early_stream_rate(now);
        let follows = self.s.seed.qos == SeedQos::ByAllocation;
        let Some(a) = self.s.push.as_mut().and_then(|p| p.alloc.as_mut()) else {
            return;
        };

        if rate > 0.0 && a.measuring_since.is_nil() {
            a.measuring_since = now;
        }
        let measuring = !a.measuring_since.is_nil() && now - a.measuring_since >= ALLOC_MEASURE;
        if let Some(cap) = upload.capacity().filter(|_| measuring && rate > 0.0) {
            let target = ((cap * ALLOC_UTILIZATION - upload.others).max(0.0) / rate) as usize;
            if !a.measured
                || target > a.budget
                || (target as f64) < a.budget as f64 * (1.0 - OVER_BUDGET)
            {
                a.budget = target;
            }
            a.measured = true;
        }
        let before = (a.budget, a.copies, a.streams);
        let paid = if a.measured {
            (a.budget / streams.max(1)).max(1)
        } else if let (Some(cap), Some(rate)) = (upload.capacity(), early) {
            (((cap * ALLOC_UTILIZATION - upload.others).max(0.0) / rate) as usize / streams.max(1))
                .max(a.window.copies)
        } else {
            usize::MAX
        };
        let cap = paid.min(neighbours);
        let changed = if follows {
            a.window.update(signals, cap, now)
        } else {
            a.window.track(cap, now)
        };
        a.copies = a.window.copies;
        if a.measured {
            a.streams = a.budget.clamp(1, streams);
        }
        a.unconstrained =
            a.measured && a.budget >= neighbours.saturating_mul(streams) && a.copies >= neighbours;
        let (copies, room) = (a.copies, a.streams);
        let key = self.s.key;
        if changed || signals.pushed > 0 {
            self.s.tap.emit(key, None, || TapKind::SeedWindow {
                copies: copies as u64,
                pushed: signals.pushed as u64,
                behind: signals.behind as u64,
            });
        }
        if before != (a.budget, a.copies, a.streams) {
            let budget = a.budget as u64;
            self.s.tap.emit(key, None, || TapKind::SeedBudget {
                budget,
                copies: copies as u64,
                streams: room as u64,
            });
        }
        if a.unconstrained {
            return;
        }

        // Under one copy, a holder's streams past the budget go.
        let mut excess: Vec<(PeerKey, Stream)> = Vec::new();
        if room < streams {
            let mut seen: BTreeMap<PeerKey, usize> = BTreeMap::new();
            for (i, st) in self.s.streams.iter().enumerate() {
                for x in &st.subscribers {
                    let n = seen.entry(x.channel).or_default();
                    *n += 1;
                    if *n > room {
                        excess.push((x.channel, i as Stream));
                    }
                }
            }
        }
        for (peer, stream) in excess {
            self.close_subscriber(peer, stream, q);
            let key = self.s.key;
            self.s
                .tap
                .emit(key, Some(peer), || TapKind::Shed { stream });
        }

        // A holder announces what it is pushed: the first push written to
        // it in an interval is due announced at the check after next.
        let mut silent: Vec<PeerKey> = Vec::new();
        for (&peer, c) in &mut self.channels {
            let have = &c.peer_have_bins;
            let Some(h) = c.push.as_mut().map(|pc| &mut pc.holder) else {
                continue;
            };
            let due = std::mem::replace(&mut h.due, h.sample.take());
            if !h.holds {
                h.silent = 0;
                continue;
            }
            if let Some((bin, at)) = due
                && now - at >= ANNOUNCE_TIMEOUT
            {
                h.silent = if have.filled_at(bin.base_left()) {
                    0
                } else {
                    h.silent.saturating_add(1)
                };
            }
            if h.silent >= SILENT_CHECKS {
                h.bar(now);
                silent.push(peer);
            }
        }
        for &peer in &silent {
            let key = self.s.key;
            self.s.tap.emit(key, Some(peer), || TapKind::HolderSilent);
            self.leave(peer, q);
        }
        let unserved = self.judge(now, q);

        // Holders by round trip, unmeasured ones farthest. One without a
        // subscription is pending: it keeps its copy for HOLDER_GRACE after
        // it joined or last asked. Past the copies the farthest subscribed
        // holders go; free copies go to the nearest neighbours that asked
        // within the grace; and with none free, the nearest of those, if
        // nearer than the farthest holder by the margin, is given one that
        // holder gives up once it subscribed.
        let subscribed: BTreeSet<PeerKey> = self
            .s
            .streams
            .iter()
            .flat_map(|st| st.subscribers.iter().map(|x| x.channel))
            .collect();
        let mut held: Vec<(bool, Option<Duration>, PeerKey)> = Vec::new();
        let mut askers: Vec<(Duration, PeerKey)> = Vec::new();
        let mut leave: Vec<PeerKey> = Vec::new();
        let mut pending = 0;
        for (&peer, c) in &self.channels {
            let Some(h) = c.push.as_ref().map(|pc| pc.holder) else {
                continue;
            };
            let recent = now - h.active <= HOLDER_GRACE;
            let rtt = h.min_rtt(now);
            if !h.holds {
                if recent
                    && c.handshake_received
                    && !h.barred(now)
                    && let Some(r) = rtt
                {
                    askers.push((r, peer));
                }
            } else if subscribed.contains(&peer) {
                held.push((rtt.is_none(), rtt, peer));
            } else if recent {
                pending += 1;
            } else {
                leave.push(peer);
            }
        }
        held.sort();
        askers.sort();
        leave.extend(held.iter().skip(copies).map(|h| h.2));
        let kept = held.len().min(copies);
        let free = copies.saturating_sub(kept + pending);
        let mut join: Vec<PeerKey> = askers.iter().take(free).map(|a| a.1).collect();
        let farthest = kept.checked_sub(1).and_then(|i| held.get(i));
        if free == 0
            && pending == 0
            && let (Some(&(r, peer)), Some(&(_, far, _))) = (askers.first(), farthest)
            && far.is_none_or(|f| r + NEAR_MARGIN.max(f / 10) < f)
        {
            join.push(peer);
        }
        for &peer in &leave {
            self.leave(peer, q);
        }
        for &peer in &join {
            if let Some(h) = self
                .channels
                .get_mut(&peer)
                .and_then(|c| c.push.as_mut())
                .map(|pc| &mut pc.holder)
            {
                h.join(now);
            }
            let key = self.s.key;
            self.s.tap.emit(key, Some(peer), || TapKind::HolderJoined);
        }
        if let Some(a) = self.s.push.as_mut().and_then(|p| p.alloc.as_mut()) {
            let left = leave.len() + silent.len() + unserved;
            a.stats.left = a.stats.left.saturating_add(left as u64);
            a.stats.joined = a.stats.joined.saturating_add(join.len() as u64);
            a.stats.silenced = a.stats.silenced.saturating_add(silent.len() as u64);
            a.stats.unserved = a.stats.unserved.saturating_add(unserved as u64);
        }
    }

    /// The reports' verdicts on the holders; see the module docs. Bars
    /// those most reporters saw push nothing, and returns how many.
    fn judge(&mut self, now: Time, q: &mut impl PeerTaskQueue) -> usize {
        // One vote per reporter and subject, over both buckets.
        let mut votes: BTreeMap<(PeerKey, PeerKey), Votes> = BTreeMap::new();
        for (&reporter, c) in &self.channels {
            let Some(pc) = c.push.as_ref() else {
                continue;
            };
            for x in pc.heard.slots.iter().flatten().flatten() {
                let v = x.window(now);
                if v != Votes::default() {
                    votes.entry((x.subject, reporter)).or_default().add(v);
                }
            }
        }
        let mut by_subject: BTreeMap<PeerKey, Vec<Votes>> = BTreeMap::new();
        for ((subject, _), v) in votes {
            if v.expected >= MIN_REPORTED {
                by_subject.entry(subject).or_default().push(v);
            }
        }

        let mut forget: Vec<PeerKey> = Vec::new();
        let mut unserved: Vec<(PeerKey, usize)> = Vec::new();
        for (&peer, c) in &mut self.channels {
            let Some(h) = c.push.as_mut().map(|pc| &mut pc.holder) else {
                continue;
            };
            let mine = by_subject.get(&peer).map_or(&[][..], Vec::as_slice);
            let n = mine.len();
            h.reporters = u8::try_from(n).unwrap_or(u8::MAX);
            let served = mine
                .iter()
                .filter(|v| v.hits as f64 >= SERVED_SHARE * v.expected as f64)
                .count();
            let none = mine
                .iter()
                .filter(|v| (v.pushed as f64) < UNSERVED_SHARE * v.expected as f64)
                .count();
            h.served = (n >= REPORT_QUORUM).then_some(served * 2 > n);
            let mut delays: Vec<u64> = mine
                .iter()
                .filter_map(|v| v.delay_ms.checked_div(v.hits))
                .collect();
            delays.sort_unstable();
            h.delay = delays
                .get(delays.len() / 2)
                .map(|&ms| Duration::from_millis(i64::try_from(ms).unwrap_or(i64::MAX)));
            if !h.holds {
                continue;
            }
            if !h.kept_up() {
                forget.push(peer);
                continue;
            }
            if now - h.joined < REPORT_PROBATION {
                continue;
            }
            if n >= REPORT_QUORUM && none * 2 > n {
                h.bar(now);
                unserved.push((peer, n));
                forget.push(peer);
            }
        }
        for &subject in &forget {
            for c in self.channels.values_mut() {
                if let Some(pc) = c.push.as_mut() {
                    pc.heard.forget(subject);
                }
            }
        }
        let key = self.s.key;
        for &(peer, n) in &unserved {
            self.s
                .tap
                .emit(key, Some(peer), || TapKind::HolderUnserved {
                    reporters: n as u64,
                });
            self.leave(peer, q);
        }
        unserved.len()
    }

    /// An adaptive seed asks each neighbour for reports every
    /// [`ASK_INTERVAL`], unless it has room for everyone.
    pub(crate) fn ask_reports(&mut self, now: Time, q: &mut impl PeerTaskQueue) {
        let unconstrained = self
            .s
            .push
            .as_ref()
            .and_then(|p| p.alloc.as_ref())
            .is_some_and(|a| a.unconstrained);
        if unconstrained {
            return;
        }
        let key = self.s.key;
        for (&peer, c) in &mut self.channels {
            if !c.handshake_received {
                continue;
            }
            let Some(h) = c.push.as_mut().map(|pc| &mut pc.holder) else {
                continue;
            };
            if now < h.ask_at {
                continue;
            }
            h.ask_at = now + ASK_INTERVAL;
            c.extra.ask = true;
            q.enqueue(peer, key);
        }
    }

    /// An ANNOUNCE from `peer`. Without entries it asks for our reports;
    /// an adaptive seed tallies the entries that pass its checks. False if
    /// an entry was invalid: about its sender, past the stream count, or
    /// past [`report_entry_limit`].
    pub(crate) fn handle_announce(&mut self, peer: PeerKey, a: Announce<'_>, now: Time) -> bool {
        let Some(c) = self.channels.get_mut(&peer) else {
            return true;
        };
        if !c.handshake_received {
            return true;
        }
        if a.is_empty() {
            if !self.s.is_seed
                && let Some(pc) = c.push.as_mut()
            {
                pc.asked = now;
            }
            return true;
        }
        if !self.adaptive_seed() {
            return true;
        }
        let streams = self.s.stream_count;
        let limit = report_entry_limit(streams);
        let mut valid = true;
        let mut counts = ReportCounters::default();
        for e in a.iter() {
            let Some(h) = self
                .channels
                .get_mut(&peer)
                .and_then(|c| c.push.as_mut())
                .map(|pc| &mut pc.holder)
            else {
                break;
            };
            h.entries = h.entries.saturating_add(1);
            if h.entries > limit || e.stream >= streams {
                valid = false;
                counts.invalid += 1;
                continue;
            }
            let mut found = self
                .channels
                .iter()
                .filter(|(_, c)| c.id.as_deref() == Some(e.parent))
                .map(|(&k, _)| k);
            let (Some(subject), None) = (found.next(), found.next()) else {
                counts.unknown += 1;
                continue;
            };
            if subject == peer {
                valid = false;
                counts.invalid += 1;
                continue;
            }
            match self.check_report(peer, subject, &e, now) {
                Err(why) => why.count(&mut counts),
                Ok(()) => {
                    counts.tallied += 1;
                    if let Some(pc) = self.channels.get_mut(&peer).and_then(|c| c.push.as_mut()) {
                        pc.heard.add(
                            usize::from(streams),
                            e.stream,
                            subject,
                            Votes::from_entry(&e),
                            now,
                        );
                    }
                }
            }
        }
        if let Some(pc) = self.channels.get_mut(&peer).and_then(|c| c.push.as_mut()) {
            pc.holder.reports.add(&counts);
        }
        if let Some(a) = self.s.push.as_mut().and_then(|p| p.alloc.as_mut()) {
            a.stats.reports.add(&counts);
        }
        valid
    }

    /// The seed's own observations against `reporter`'s entry about
    /// `subject`; see the module docs.
    fn check_report(
        &self,
        reporter: PeerKey,
        subject: PeerKey,
        e: &AnnounceEntry<'_>,
        now: Time,
    ) -> Result<(), Unheard> {
        let fed = |peer: PeerKey| {
            self.s
                .streams
                .get(usize::from(e.stream))
                .is_some_and(|st| st.subscribers.iter().any(|x| x.channel == peer))
        };
        if fed(reporter) {
            return Err(Unheard::ReporterFed);
        }
        if !fed(subject) {
            return Err(Unheard::SubjectUnfed);
        }
        let lag = |peer: PeerKey| {
            self.channels
                .get(&peer)
                .and_then(|c| c.push.as_ref())
                .and_then(|pc| pc.have_lag.get(usize::from(e.stream)))
                .filter(|l| l.fresh(now))
                .map(|l| l.mean())
        };
        let Some(r) = lag(reporter) else {
            return Err(Unheard::ReporterSilent);
        };
        let Some(p) = lag(subject) else {
            return Ok(());
        };
        let rtt = self
            .channels
            .get(&subject)
            .and_then(|c| c.push.as_ref())
            .and_then(|pc| pc.holder.min_rtt(now))
            .unwrap_or_default();
        if r < p - rtt - REPORT_JITTER {
            return Err(Unheard::TooEarly);
        }
        if e.misses > e.hits && r < p + DEADLINE_MARGIN {
            return Err(Unheard::MissesUnseen);
        }
        Ok(())
    }

    /// `peer` stops being a holder: every subscription it has goes.
    fn leave(&mut self, peer: PeerKey, q: &mut impl PeerTaskQueue) {
        if let Some(h) = self
            .channels
            .get_mut(&peer)
            .and_then(|c| c.push.as_mut())
            .map(|pc| &mut pc.holder)
        {
            h.holds = false;
        }
        let key = self.s.key;
        self.s.tap.emit(key, Some(peer), || TapKind::HolderLeft);
        let streams: Vec<Stream> = self
            .s
            .streams
            .iter()
            .enumerate()
            .filter(|(_, st)| st.subscribers.iter().any(|x| x.channel == peer))
            .map(|(i, _)| i as Stream)
            .collect();
        for s in streams {
            self.close_subscriber(peer, s, q);
        }
    }

    /// Fixes the peers a seed pushes each stream to, one list per stream
    /// (`None` lifts the pin): subscribers outside it are closed. False if
    /// this isn't a hybrid seed.
    pub(crate) fn pin_allocation(
        &mut self,
        pin: Option<Vec<Vec<PeerKey>>>,
        q: &mut impl PeerTaskQueue,
    ) -> bool {
        if !self.s.is_seed {
            return false;
        }
        let Some(p) = self.s.push.as_mut() else {
            return false;
        };
        p.pin = pin;
        let Some(pin) = p.pin.as_ref() else {
            return true;
        };
        let mut outside = Vec::new();
        for (i, st) in self.s.streams.iter().enumerate() {
            for sub in &st.subscribers {
                if !pin.get(i).is_some_and(|v| v.contains(&sub.channel)) {
                    outside.push((sub.channel, i as Stream));
                }
            }
        }
        for (peer, stream) in outside {
            self.close_subscriber(peer, stream, q);
        }
        true
    }

    /// A seed under [`SeedQos::ByAllocation`] raises each channel's weight
    /// in `weights` to its share of the streams pushed to it (at least the
    /// minimum). False, touching nothing, for any other swarm.
    pub(crate) fn seed_weights(&self, weights: &mut BTreeMap<PeerKey, u64>) -> bool {
        let s = &self.s;
        if !s.is_seed || s.push.is_none() || s.seed.qos != SeedQos::ByAllocation {
            return false;
        }
        let mut held: BTreeMap<PeerKey, u64> = BTreeMap::new();
        for st in &s.streams {
            for sub in &st.subscribers {
                *held.entry(sub.channel).or_default() += 1;
            }
        }
        let n = u64::from(s.stream_count.max(1));
        for &peer in self.channels.keys() {
            let h = held.get(&peer).copied().unwrap_or(0);
            let w = (MAX_PEER_QOS_WEIGHT * h / n).max(MIN_PEER_QOS_WEIGHT);
            let e = weights.entry(peer).or_insert(0);
            *e = (*e).max(w);
        }
        true
    }

    /// Whether our subscribers changed since the last call, for a hybrid
    /// seed under [`SeedQos::ByAllocation`]: its weights follow them.
    pub(crate) fn take_seed_weights_changed(&mut self) -> bool {
        let s = &mut self.s;
        let changed = std::mem::take(&mut s.subscribers_changed);
        changed && s.is_seed && s.push.is_some() && s.seed.qos == SeedQos::ByAllocation
    }

    /// A seed's view of its neighbours; `None` for any other swarm.
    pub(crate) fn seed_allocation(&self, now: Time) -> Option<SeedAllocationStats> {
        let s = &self.s;
        if !s.is_seed {
            return None;
        }
        let p = s.push.as_ref()?;
        let neighbours = self
            .channels
            .iter()
            .map(|(&peer, c)| SeedNeighbour {
                peer,
                streams: s
                    .streams
                    .iter()
                    .enumerate()
                    .filter(|(_, st)| st.subscribers.iter().any(|x| x.channel == peer))
                    .map(|(i, _)| i as Stream)
                    .collect(),
                lags: c.push.as_ref().map_or_else(Vec::new, |pc| {
                    pc.have_lag
                        .iter()
                        .map(|l| l.fresh(now).then(|| l.mean()))
                        .collect()
                }),
                rtt: c.controller.rtt(),
                min_rtt: c.push.as_ref().and_then(|pc| pc.holder.min_rtt(now)),
                holder: c.push.as_ref().is_some_and(|pc| pc.holder.holds),
                barred: c.push.as_ref().is_some_and(|pc| pc.holder.barred(now)),
                reporters: c
                    .push
                    .as_ref()
                    .map_or(0, |pc| usize::from(pc.holder.reporters)),
                served: c.push.as_ref().and_then(|pc| pc.holder.served),
                report_delay: c.push.as_ref().and_then(|pc| pc.holder.delay),
                kept_up: c.push.as_ref().is_some_and(|pc| pc.holder.kept_up()),
                reports: c
                    .push
                    .as_ref()
                    .map_or_else(ReportCounters::default, |pc| pc.holder.reports),
            })
            .collect();
        let mut out = SeedAllocationStats {
            neighbours,
            ..Default::default()
        };
        if let Some(a) = p.alloc.as_ref() {
            out.budget = a.budget;
            out.window = a.window.copies;
            out.copies = a.copies;
            out.counters = a.stats;
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_round_trip_is_the_windowed_minimum() {
        let t = |s: i64| Time::from_unix_millis(1_000_000_000) + Duration::from_secs(s);
        let ms = Duration::from_millis;
        let mut h = Holder::default();
        assert_eq!(h.min_rtt(t(0)), None);
        h.ping(ms(50), t(0));
        h.ping(ms(30), t(1));
        h.ping(ms(80), t(2));
        assert_eq!(h.min_rtt(t(2)), Some(ms(30)));
        // The window before still counts ...
        h.ping(ms(60), t(10));
        assert_eq!(h.min_rtt(t(11)), Some(ms(30)));
        // ... and the one before that no longer does.
        h.ping(ms(70), t(20));
        assert_eq!(h.min_rtt(t(21)), Some(ms(60)));
        assert_eq!(h.min_rtt(t(31)), Some(ms(70)));
        assert_eq!(h.min_rtt(t(45)), None);
    }

    fn signals(pushed: usize, behind: usize, over: usize, in_use: usize, sat: bool) -> Signals {
        Signals {
            pushed,
            behind,
            overloaded: over,
            in_use,
            saturated: sat,
        }
    }

    #[test]
    fn the_window_doubles_while_pushes_keep_up() {
        let t = |s: i64| Time::from_unix_millis(1_000_000_000) + Duration::from_secs(s);
        let mut w = Window::new(t(0));
        // No pushes, no evidence.
        assert!(!w.update(signals(0, 0, 0, 0, false), 64, t(1)));
        assert_eq!(w.copies, 1);
        // On a saturated uplink it leads the copies in use at most twice.
        assert!(w.update(signals(1, 0, 0, 1, true), 64, t(2)));
        assert_eq!((w.copies, w.since), (2, t(2)));
        assert!(!w.update(signals(1, 0, 0, 1, true), 64, t(3)));
        assert!(w.update(signals(2, 0, 0, 2, true), 64, t(4)));
        assert_eq!(w.copies, 4);
        // An uplink with idle time lets it run ahead, up to the cap.
        assert!(w.update(signals(1, 0, 0, 1, false), 64, t(5)));
        assert_eq!(w.copies, 8);
        assert!(w.update(signals(1, 0, 0, 1, false), 5, t(6)));
        assert_eq!(w.copies, 5);
        // A minority behind doesn't hold it back; a majority does.
        assert!(w.update(signals(3, 1, 0, 3, true), 64, t(7)));
        assert_eq!(w.copies, 10);
        assert!(!w.update(signals(10, 6, 0, 10, true), 64, t(8)));
    }

    #[test]
    fn the_window_halves_when_most_holders_are_overloaded() {
        let t = |s: i64| Time::from_unix_millis(1_000_000_000) + Duration::from_secs(s);
        let mut w = Window::new(t(0));
        for i in 1..=3 {
            w.update(signals(1, 0, 0, 1, false), 64, t(i));
        }
        assert_eq!(w.copies, 8);
        // Overloaded on an uplink with idle time: their own paths.
        assert!(!w.update(signals(8, 8, 8, 8, false), 64, t(4)));
        // Behind on a saturated one holds it.
        assert!(!w.update(signals(8, 8, 4, 8, true), 64, t(5)));
        // Overloaded on a saturated one: halved, and doubling stops there.
        assert!(w.update(signals(8, 8, 5, 8, true), 64, t(6)));
        assert_eq!(w.copies, 4);
        // It then grows by one copy per probe interval.
        let probe = t(6) + WINDOW_PROBE;
        assert!(!w.update(signals(4, 0, 0, 4, true), 64, probe - Duration::SECOND));
        assert!(w.update(signals(4, 0, 0, 4, true), 64, probe));
        assert_eq!(w.copies, 5);
        assert!(!w.update(signals(5, 0, 0, 5, true), 64, probe + Duration::SECOND));
        // At one copy there is nothing to cut, and the next step doubles.
        let mut w = Window::new(t(0));
        assert!(!w.update(signals(1, 1, 1, 1, true), 64, t(1)));
        assert!(w.update(signals(1, 0, 0, 1, true), 64, t(2)));
        assert_eq!(w.copies, 2);
        assert!(!w.update(signals(1, 0, 0, 1, true), 64, t(3)));
    }
}
