// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The answer gate: a [`PushMode::Hybrid`](crate::PushMode::Hybrid) node
//! answers pull requests only from the uplink its pushes leave.
//!
//! An uplink byte goes first to what only this node sends. Behind a seed
//! with little margin, answers compete with pushes for its uplink, and a
//! lower priority within a peer's frame doesn't stop them: across peers
//! the driver's QoS decides, and below the runner the access link splits
//! the uplink by transport share, so answer bytes already handed down sit
//! ahead of the next push. Viewers whose pushes come late prune their
//! parents and pull more, and the pull load locks in.
//!
//! So a hybrid swarm's answers queue apart, below pushes and below the
//! answers of swarms that don't push, and none is written while a push
//! waits for any peer whose push queue stood empty within
//! [`BLOCKED_SPAN`]. A peer whose pushes have waited longer is blocked on
//! its own link (a slow viewer whose transport doesn't drain), so it holds
//! no answer back. When every peer's pushes back up (an overcommitted
//! uplink), answers go as the driver polls. Peers whose answers the gate
//! held are woken as soon as it opens.
//!
//! Holding answers behind the runner's pushes doesn't keep them from
//! queueing below it, where they delay pushes all the same and read as load
//! that keeps the node from admitting the subscribers that would relieve it
//! (the pull lock-in, whether or not the node has subscribers yet). So
//! answers mustn't hold a standing queue below the runner either
//! ([`AnswerRate`]). Jitter and a burst of repairs don't stand, and answers
//! that only fill idle uplink cost nothing, so answers go unpaced until the
//! node's queueing delay below the runner, read from its links' probes,
//! has stood past [`TARGET_DELAY`] for [`STANDING_SPAN`] (past [`DEEP`]
//! times it, for a quarter of that) while answers made a fair share of
//! what went out: a queue of the node's own pushes isn't theirs to drain,
//! and pacing them would only delay the repairs it will need. Then they go
//! at the rate they had, cut by their share of the queue, and the rate
//! moves on the least reading of the last quarter span: up while answers
//! wait and the queue stays under the target (doubling once it has
//! drained under half of it, or while answers make little of what goes
//! out, as the queue is then the node's pushes), down above it. Pacing
//! lets go once answers went out over a span without waiting and the queue
//! stands under half the target.
//!
//! The gate reads only the node's own queues and its links' round trips: it
//! needs no capacity estimate, and trusts no requester. A peer that stops
//! reading holds back no answer for longer than [`BLOCKED_SPAN`]. A peer can
//! only delay its echoes, which makes its own link read farther, never
//! nearer; the reading is a low quantile over the links (the least of one or
//! two), so no single peer starts pacing, or stops it for the others.

use std::collections::VecDeque;

use slotmap::SlotMap;
use strims_base::path::{self, PROBE_INTERVAL, PathReading};
use strims_base::time::{Duration, Time};

use crate::peer::RunnerPeer;
use crate::runner::PeerKey;
use crate::scheduler::PeerPriority;

/// How long a peer's oldest queued push may have waited for its pushes to
/// hold answers back.
pub const BLOCKED_SPAN: Duration = Duration::from_millis(500);

/// The queueing delay below the runner that answers may raise: LEDBAT's
/// target is at most 100 ms.
pub const TARGET_DELAY: Duration = Duration::from_millis(50);
/// A queue stands when its least reading over this span passes
/// [`TARGET_DELAY`]: jitter and a burst of answers don't last it. Pacing lets
/// go once the rate hasn't bound answers for this long and the queue stands
/// under half the target.
pub const STANDING_SPAN: Duration = Duration::SECOND;
/// A queue standing past this many times the target stands within a quarter
/// of [`STANDING_SPAN`]: the deeper it is, the sooner it matters.
pub const DEEP: i64 = 4;
/// Pacing starts only while answers make at least this share of what went
/// out, and below it a paced rate grows back as from a drained queue.
const ENGAGE_SHARE: f64 = 0.25;
/// The paced answers' rate never falls below this, in bytes per second, so a
/// node behind a standing queue still repairs what pushes miss ...
pub const ANSWER_RATE_FLOOR: u64 = 32 * 1024;
/// ... and never grows past this.
pub const ANSWER_RATE_MAX: u64 = 1 << 30;
/// The rate before pacing first starts, which sets nothing: pacing starts
/// at the answers' measured rate.
const ANSWER_RATE_START: u64 = 128 * 1024;
/// The rate's growth per update once the queue has drained under half the
/// target, or while answers make little of what goes out, as answers wait:
/// the probe interval is the rate's round trip, and TCP's slow start
/// doubles per round trip ...
const DRAINED_GROWTH: f64 = 2.0;
/// ... and nearer the target, at no delay, as a share of the rate ...
const GROWTH: f64 = 0.1;
/// ... or this many bytes per second, whichever is more; it shrinks as the
/// delay nears the target.
const GROWTH_MIN: f64 = 8.0 * 1024.0;
/// The largest cut per update, at twice the target or more, before it is
/// scaled by answers' share of what went out.
const CUT: f64 = 0.5;
/// The bucket holds this much of the rate ...
const BUCKET_SPAN: Duration = Duration::from_millis(20);
/// ... and at least this many bytes: two 4 KiB chunks.
pub const ANSWER_BUCKET_MIN: u64 = 8 * 1024;

/// Whether a hybrid swarm's answer may be written now: no peer's oldest
/// queued push is younger than [`BLOCKED_SPAN`], and `rate` has tokens while
/// it paces. Drops the dead entries ahead of each peer's oldest live push.
pub(crate) fn admits(
    peers: &mut SlotMap<PeerKey, RunnerPeer>,
    rate: &mut AnswerRate,
    now: Time,
) -> bool {
    let waiting = peers.values_mut().any(|p| {
        p.queue_mut(PeerPriority::High)
            .front_time()
            .is_some_and(|t| now - t < BLOCKED_SPAN)
    });
    if waiting {
        return false;
    }
    if !rate.open(now) {
        rate.note_limited();
        return false;
    }
    true
}

/// The pace of a node's hybrid answers (see the [module docs](self)): a
/// token bucket and the rate that fills it, which binds only while a queue
/// stands below the runner.
#[derive(Debug)]
pub(crate) struct AnswerRate {
    /// Bytes per second.
    rate: f64,
    /// Bytes; one write may take it below zero.
    tokens: f64,
    /// When `tokens` was last brought up to date; nil before the first.
    refilled: Time,
    /// Answers waited for tokens since the rate last moved.
    limited: bool,
    /// When the rate last moved: at most once per probe interval.
    moved: Time,
    /// The links' latest queueing excess, as of the last update.
    reading: Option<Duration>,
    /// Scratch for the links' excesses. Bound: one per peer.
    scratch: Vec<Duration>,
    /// Whether the rate binds now ...
    engaged: bool,
    /// ... and the least reading over the last quarter of
    /// [`STANDING_SPAN`], which moves the rate in place of the latest:
    /// jitter doesn't cut it, a queue still shows.
    settled: Option<Duration>,
    /// Answers waited for tokens since the last update ...
    waited: bool,
    /// ... and when they last did, as of an update.
    last_limited: Time,
    /// Answer bytes written since the last update ...
    sent: u64,
    /// ... and bytes of every kind.
    written: u64,
    /// Each update over the last [`STANDING_SPAN`] and the one before it:
    /// when, the reading, and the answer bytes and all bytes written since
    /// the update before. Bound: the updates of a span.
    recent: VecDeque<(Time, Option<Duration>, u64, u64)>,
    /// Answers' share of the bytes written over the last quarter of
    /// [`STANDING_SPAN`]: cutting answers drains only their part of a
    /// queue, so a cut takes this share of its full depth.
    share: f64,
}

impl AnswerRate {
    pub(crate) fn new() -> AnswerRate {
        AnswerRate {
            rate: ANSWER_RATE_START as f64,
            tokens: 0.0,
            refilled: Time::NIL,
            limited: false,
            moved: Time::NIL,
            reading: None,
            scratch: Vec::new(),
            engaged: false,
            settled: None,
            waited: false,
            last_limited: Time::NIL,
            sent: 0,
            written: 0,
            recent: VecDeque::new(),
            share: 1.0,
        }
    }

    fn depth(&self) -> f64 {
        (self.rate * BUCKET_SPAN.as_secs_f64()).max(ANSWER_BUCKET_MIN as f64)
    }

    fn refill(&mut self, now: Time) {
        if self.refilled.is_nil() {
            self.refilled = now;
            self.tokens = self.depth();
            return;
        }
        if now <= self.refilled {
            return;
        }
        let dt = (now - self.refilled).as_secs_f64();
        self.tokens = (self.tokens + self.rate * dt).min(self.depth());
        self.refilled = now;
    }

    /// An answer may be written now: the rate doesn't bind, or tokens are
    /// left.
    pub(crate) fn open(&mut self, now: Time) -> bool {
        self.refill(now);
        !self.engaged || self.tokens > 0.0
    }

    /// An answer of `bytes` was written.
    pub(crate) fn spend(&mut self, bytes: usize) {
        self.sent = self.sent.saturating_add(bytes as u64);
        if self.engaged {
            self.tokens -= bytes as f64;
        }
    }

    /// `bytes` of any kind were written.
    pub(crate) fn wrote(&mut self, bytes: usize) {
        self.written = self.written.saturating_add(bytes as u64);
    }

    /// An answer waited for tokens.
    pub(crate) fn note_limited(&mut self) {
        self.limited = true;
        self.waited = true;
    }

    /// The rate binds and the bucket holds no tokens.
    pub(crate) fn exhausted(&self) -> bool {
        self.engaged && self.tokens <= 0.0
    }

    /// When the bucket next holds tokens.
    pub(crate) fn open_at(&self) -> Time {
        if self.tokens > 0.0 || self.refilled.is_nil() {
            return self.refilled;
        }
        let secs = -self.tokens / self.rate.max(1.0);
        self.refilled + Duration::from_nanos((secs * 1e9).ceil() as i64 + 1)
    }

    /// The tick: reads the node's queueing delay below the runner from its
    /// links' `readings`, starts or stops pacing, and while pacing, once per
    /// probe interval, moves the rate: a move shows only once a later
    /// probe's echo returns.
    pub(crate) fn update(&mut self, now: Time, readings: impl IntoIterator<Item = PathReading>) {
        self.refill(now);
        self.reading = path::latest_excess(readings, now, &mut self.scratch);
        if core::mem::take(&mut self.waited) {
            self.last_limited = now;
        }
        if !self.engage(now) {
            self.limited = false;
            return;
        }
        let Some(d) = self.settled else {
            self.limited = false;
            return;
        };
        if !self.moved.is_nil() && now - self.moved < PROBE_INTERVAL {
            return;
        }
        self.moved = now;
        let limited = core::mem::take(&mut self.limited);
        let (d, target) = (d.as_secs_f64(), TARGET_DELAY.as_secs_f64());
        if self.share < ENGAGE_SHARE {
            // Answers make little of what goes out: the queue isn't theirs,
            // and holding them only starves the repairs it causes.
            if limited {
                self.rate *= DRAINED_GROWTH;
            }
        } else if d < target {
            // A queue drained under half the target shows the path has room
            // again: the cut that drained it shouldn't take a slow climb
            // back.
            if limited && d < target / 2.0 {
                self.rate *= DRAINED_GROWTH;
            } else if limited {
                self.rate += (self.rate * GROWTH).max(GROWTH_MIN) * (1.0 - d / target);
            }
        } else {
            let full = (1.0 - CUT * (d - target) / target).max(1.0 - CUT);
            self.rate *= 1.0 - self.share * (1.0 - full);
        }
        self.rate = self
            .rate
            .clamp(ANSWER_RATE_FLOOR as f64, ANSWER_RATE_MAX as f64);
        self.tokens = self.tokens.min(self.depth());
    }

    /// Whether the rate binds after this update. A queue standing past the
    /// target while answers make a fair share of what went out starts
    /// pacing, at the answers' rate over the span cut by their share of a
    /// reading of the standing delay; it lets go once answers went out over
    /// a span without waiting and the queue fell under half the target.
    fn engage(&mut self, now: Time) -> bool {
        let (sent, written) = (
            core::mem::take(&mut self.sent),
            core::mem::take(&mut self.written),
        );
        self.recent.push_back((now, self.reading, sent, written));
        while self
            .recent
            .get(1)
            .is_some_and(|&(t, ..)| now - t >= STANDING_SPAN)
        {
            self.recent.pop_front();
        }
        // The least reading over the last `span`, if the updates cover it;
        // a missing reading shows no standing queue.
        let least = |span: Duration| {
            let from = self.recent.iter().rposition(|&(t, ..)| now - t >= span)?;
            self.recent
                .range(from..)
                .map(|&(_, d, ..)| d)
                .min()
                .flatten()
        };
        let quarter = self
            .recent
            .iter()
            .rposition(|&(t, ..)| now - t >= STANDING_SPAN / 4)
            .map_or(0, |i| i + 1);
        let (sent, written) = self
            .recent
            .range(quarter..)
            .fold((0, 0), |(s, w), &(.., b, all)| (s + b, w + all));
        self.share = if written == 0 {
            1.0
        } else {
            (sent as f64 / written as f64).min(1.0)
        };
        let standing = least(STANDING_SPAN);
        self.settled = least(STANDING_SPAN / 4);
        if !self.engaged {
            let deep = self.settled.filter(|&d| d > TARGET_DELAY * DEEP);
            let Some(d) = deep.or(standing.filter(|&d| d > TARGET_DELAY)) else {
                return false;
            };
            if self.share < ENGAGE_SHARE {
                return false;
            }
            let sent: u64 = self.recent.iter().skip(1).map(|&(.., b, _)| b).sum();
            let (d, target) = (d.as_secs_f64(), TARGET_DELAY.as_secs_f64());
            let full = (1.0 - CUT * (d - target) / target).max(1.0 - CUT);
            let cut = 1.0 - self.share * (1.0 - full);
            self.rate = (sent as f64 / STANDING_SPAN.as_secs_f64() * cut)
                .clamp(ANSWER_RATE_FLOOR as f64, ANSWER_RATE_MAX as f64);
            self.tokens = self.depth();
            self.engaged = true;
            self.moved = now;
            return false;
        }
        // Let go once answers went out over a span and none waited.
        let bound_lately = !self.last_limited.is_nil() && now - self.last_limited < STANDING_SPAN;
        let covered = self
            .recent
            .front()
            .is_some_and(|&(t, ..)| now - t >= STANDING_SPAN);
        let flowed = covered && self.recent.iter().skip(1).any(|&(.., b, _)| b > 0);
        if flowed && !bound_lately && standing.is_none_or(|d| d < TARGET_DELAY / 2) {
            self.engaged = false;
            return false;
        }
        true
    }

    /// Bytes per second.
    pub(crate) fn rate(&self) -> u64 {
        self.rate as u64
    }

    /// The links' latest queueing excess, as of the last update.
    pub(crate) fn reading(&self) -> Option<Duration> {
        self.reading
    }
}

#[cfg(test)]
#[path = "answer_gate_test.rs"]
mod tests;
