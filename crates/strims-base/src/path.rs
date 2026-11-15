// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A link's path as its sender measures it: round trips, the bytes in
//! flight, the delivery rate and the bytes queued below the sender.
//!
//! Now and then the sender writes a probe carrying an unpredictable nonce
//! into the link's byte stream ([`PathMeter`]). It notes when, and how many
//! bytes it had written up to and including the probe. The receiver answers
//! the newest probe it has read with an echo of the nonce ([`Echo`]). A
//! link is an ordered byte stream, so an echo proves that every byte up to
//! its probe arrived.
//!
//! Each echo gives:
//!
//! - the round trip: the echo's arrival minus the probe's send time. It
//!   includes however long the receiver held the echo. That hold is the
//!   receiver's claim, so nothing subtracts it;
//! - the bytes in flight: those written during that round trip;
//! - the delivery rate: the bytes delivered between the probes
//!   acknowledged in the last [`RATE_WINDOW`], over the longer of the time
//!   between their sends and the time between their echoes. Bunched echoes
//!   can't raise it past the rate the bytes were written, and bunched
//!   probes can't raise it past the rate they were read;
//! - the bytes queued: those in flight beyond what the path holds at its
//!   base round trip, `in flight − rate × base`. While the sender writes
//!   steadily that is `rate × (round trip − base)`.
//!
//! A [`PathReading`] holds the values as of the latest echo, plus the
//! bytes written since. It doesn't estimate a drain: between echoes, a
//! link's queue is at most [`PathReading::queued_bound`].
//!
//! # Trust
//!
//! An echo is a claim by the peer. An echo whose nonce matches no
//! outstanding probe is forged or replayed. It is ignored and counted, and
//! it voids every outstanding probe. A forger therefore gets one guess per
//! window of [`MAX_PROBES`] probes, and stalls only its own link's
//! samples. A peer can't answer a probe it hasn't read, so it can only
//! delay its echoes. That makes its own link read farther (a longer round
//! trip), fuller (more in flight) and slower, never nearer or emptier. A
//! steady delay cancels out of the queued bytes, because the bytes in
//! flight and the base grow together.
//!
//! [`aggregate`] takes a low quantile across links, so a minority of slow
//! or delaying peers can't move a node-wide reading.
//!
//! # Base and recent round trips
//!
//! [`PathDelay`] keeps a path's base: the least round trip over
//! [`BASE_BUCKETS`] buckets of [`BASE_BUCKET`] (ten minutes). Queued
//! samples can't raise a minimum, and the first probe goes out before any
//! data, so a queue that builds later reads as excess. A queue that stands
//! for the whole window becomes the base. A path's current delay is the
//! least round trip of the last [`RECENT`], so one sample that met a burst
//! doesn't read as a standing queue.

use crate::time::{Duration, Time};

/// The most probes a link has unanswered. At this bound the sender
/// waits for an echo before it probes again, so it never forgets a probe
/// and an unknown nonce is always forged.
pub const MAX_PROBES: usize = 16;
/// How often a link probes while it writes at least [`BUSY_BYTES`] of data
/// between probes ...
pub const PROBE_INTERVAL: Duration = Duration::from_millis(200);
/// ... a quarter of a media swarm's chunk: a low-bitrate stream sends less
/// than a chunk an interval (its partial chunks without their trailing
/// zeros), and its links queue at each unit as much as any ...
pub const BUSY_BYTES: u64 = 1024;
/// ... and how often while it writes less. Only links that carry data
/// build queues worth sampling closely.
pub const QUIET_PROBE_INTERVAL: Duration = Duration::SECOND;
/// The span of echoes the delivery rate is measured over.
pub const RATE_WINDOW: Duration = Duration::from_secs(1);
/// Acknowledged probes kept for the delivery rate.
const ACKED: usize = 16;
/// The span of one base bucket ...
pub const BASE_BUCKET: Duration = Duration::from_secs(30);
/// ... and how many make up the base window.
pub const BASE_BUCKETS: usize = 20;
/// Round trips that arrived this recently stand for the path's current
/// delay.
pub const RECENT: Duration = Duration::from_secs(3);
/// Round trips a path keeps between folds into its base.
const SAMPLES: usize = 32;
/// The fewest paths with recent samples that give an [`aggregate`].
pub const MIN_PATHS: usize = 3;
/// How old a link's latest echo may be to count in [`latest_excess`]:
/// several probe intervals of a link that writes, so a link that stopped
/// writing drops out.
pub const LATEST_MAX_AGE: Duration = Duration::SECOND;
/// The resolution a driver's clock and event loop time a read to.
pub const TRAIN_TICK: Duration = Duration::from_millis(1);
/// How long an uplink train's leg takes to drain at the uplink it measures
/// ([`UplinkTrains`]): fifteen [`TRAIN_TICK`]s, so a leg reads its rate
/// within a few percent ...
pub const TRAIN_DRAIN: Duration = Duration::from_millis(15);
/// ... in legs of at most this many bytes on a stream link, which its
/// transport's window takes at once even after idling (TCP's restart
/// window of ten packets) ...
pub const TRAIN_LEG: u64 = 12 * 1024;
/// ... and on a data channel (SCTP's of four).
pub const TRAIN_LEG_CHANNEL: u64 = 4 * 1024;
/// The latest legs, on as many links, that [`UplinkTrains::rate`] reads:
/// the fewest whose ⌊k/3⌋+1-th fastest no single peer raises past an
/// honest leg's ([`MIN_PATHS`]), so a fall of the uplink shows in as few
/// legs as that allows.
pub const TRAIN_LEGS: usize = MIN_PATHS;
/// How long a leg counts toward [`UplinkTrains::rate`], and how long one
/// may wait for its echo.
pub const TRAIN_SPAN: Duration = Duration::from_secs(10);
/// Legs an [`UplinkTrains`] keeps.
const TRAINS: usize = 32;

/// One round trip, from a probe to its echo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathSample {
    /// When the echo arrived.
    pub at: Time,
    pub rtt: Duration,
}

/// A link's path as of its latest echo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathReading {
    /// Echoes accepted so far.
    pub samples: u64,
    /// When the latest echo arrived.
    pub at: Time,
    /// The latest round trip.
    pub rtt: Duration,
    /// The least round trip of the base window. `None` once the window
    /// holds no sample.
    pub base: Option<Duration>,
    /// The least round trip of the last [`RECENT`]. `None` without a
    /// recent sample.
    pub recent_min: Option<Duration>,
    /// Bytes written during the latest echo's round trip.
    pub in_flight: u64,
    /// Bytes per second delivered. `None` until two echoes.
    pub rate: Option<u64>,
    /// Bytes in flight beyond `rate × base`, at the latest echo. `None`
    /// without a rate.
    pub queued: Option<u64>,
    /// Bytes written since the latest echo.
    pub written_since: u64,
}

impl PathReading {
    /// How far the recent round trip stands above the base.
    pub fn excess(&self) -> Option<Duration> {
        let recent = self.recent_min?;
        Some(
            self.base
                .map_or(Duration::ZERO, |b| (recent - b).max(Duration::ZERO)),
        )
    }

    /// How far the latest round trip stands above the base; zero without
    /// a base.
    pub fn latest_excess(&self) -> Duration {
        self.base
            .map_or(Duration::ZERO, |b| (self.rtt - b).max(Duration::ZERO))
    }

    /// How long the queued bytes take to drain at the delivery rate.
    pub fn queued_delay(&self) -> Option<Duration> {
        let rate = self.rate.filter(|&r| r > 0)?;
        let nanos = u128::from(self.queued?) * 1_000_000_000 / u128::from(rate);
        Some(Duration::from_nanos(
            i64::try_from(nanos).unwrap_or(i64::MAX),
        ))
    }

    /// The most the link can have queued now: the bytes queued at the latest
    /// echo (all in flight without a rate), plus every byte written since.
    pub fn queued_bound(&self) -> u64 {
        self.queued
            .unwrap_or(self.in_flight)
            .saturating_add(self.written_since)
    }

    /// How old the reading is.
    pub fn age(&self, now: Time) -> Duration {
        now - self.at
    }
}

/// An echo whose nonce matches no outstanding probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownEcho;

impl core::fmt::Display for UnknownEcho {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("echo of an unknown probe")
    }
}

impl std::error::Error for UnknownEcho {}

#[derive(Clone, Copy, Debug, Default)]
struct Probe {
    nonce: u32,
    sent: Time,
    /// Bytes written up to and including the probe.
    written: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct Acked {
    sent: Time,
    acked: Time,
    delivered: u64,
}

/// A train in progress ([`PathMeter::start_train`]).
#[derive(Clone, Copy, Debug)]
struct OpenTrain {
    /// The first probe's nonce, when it went, and the bytes written
    /// through it.
    first: u32,
    sent: Time,
    written: u64,
    /// The bytes to write between the two probes.
    len: u64,
    /// The closing probe's nonce and the bytes written through it, once
    /// written.
    last: Option<(u32, u64)>,
}

/// One leg of a train: what [`PathMeter::take_train`] gives once its
/// closing probe's echo came.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LegSample {
    /// When the closing probe's echo came.
    pub at: Time,
    /// The bytes after the first probe, through the closing one.
    pub bytes: u64,
    /// How long after the first probe the receiver read the closing one,
    /// as its echo says ([`Echo::read_timed`]): the time those bytes took
    /// to arrive.
    pub gap: Duration,
}

/// `bytes` per second over `d`; `None` for a span that isn't positive.
fn rate_over(bytes: u64, d: Duration) -> Option<u64> {
    let nanos = u128::try_from(d.as_nanos()).ok().filter(|&n| n > 0)?;
    u64::try_from(u128::from(bytes) * 1_000_000_000 / nanos).ok()
}

#[derive(Clone, Copy, Debug)]
struct Latest {
    at: Time,
    rtt: Duration,
    in_flight: u64,
    rate: Option<u64>,
    queued: Option<u64>,
    /// Bytes written when the echo arrived.
    written: u64,
}

/// The sending side of a link's path measure.
///
/// The owner reports every byte it puts into the link's stream
/// ([`PathMeter::wrote`]), writes a probe when [`PathMeter::probe_due`]
/// says so ([`PathMeter::probe`]), and hands it the nonces of echoes
/// ([`PathMeter::echo`]).
#[derive(Clone, Debug)]
pub struct PathMeter {
    written: u64,
    /// Unanswered probes, oldest first. Bound: [`MAX_PROBES`].
    probes: [Probe; MAX_PROBES],
    len: usize,
    last_probe: Time,
    /// Data bytes (not probes and echoes) written since the last probe.
    sent: u64,
    /// Acknowledged probes, oldest first. Bound: [`ACKED`].
    acked: [Acked; ACKED],
    acked_len: usize,
    delay: PathDelay,
    latest: Option<Latest>,
    samples: u64,
    unknown: u64,
    negative: u64,
    train: Option<OpenTrain>,
    /// The latest train's measure, until taken.
    leg: Option<LegSample>,
}

impl Default for PathMeter {
    fn default() -> PathMeter {
        PathMeter::new()
    }
}

impl PathMeter {
    pub fn new() -> PathMeter {
        PathMeter {
            written: 0,
            probes: [Probe::default(); MAX_PROBES],
            len: 0,
            last_probe: Time::NIL,
            sent: 0,
            acked: [Acked::default(); ACKED],
            acked_len: 0,
            delay: PathDelay::default(),
            latest: None,
            samples: 0,
            unknown: 0,
            negative: 0,
            train: None,
            leg: None,
        }
    }

    /// Counts `n` bytes put into the stream. `data` is false for the
    /// measure's own probes and echoes, which don't make a link a
    /// sending one.
    pub fn wrote(&mut self, n: u64, data: bool) {
        self.written = self.written.saturating_add(n);
        if data {
            self.sent = self.sent.saturating_add(n);
        }
    }

    /// A probe is due: none has gone out yet, or since the last one the
    /// link wrote [`BUSY_BYTES`] of data and `interval` passed, or wrote
    /// any and [`QUIET_PROBE_INTERVAL`] passed (or `interval`, if longer).
    /// Never while [`MAX_PROBES`] are unanswered.
    pub fn probe_due(&self, now: Time, interval: Duration) -> bool {
        if !self.can_probe() || self.train_open(now) {
            return false;
        }
        if self.last_probe.is_nil() {
            return true;
        }
        let since = now - self.last_probe;
        (self.sent >= BUSY_BYTES && since >= interval)
            || (self.sent > 0 && since >= interval.max(QUIET_PROBE_INTERVAL))
    }

    /// Fewer than [`MAX_PROBES`] are unanswered.
    pub fn can_probe(&self) -> bool {
        self.len < MAX_PROBES
    }

    /// `nonce` belongs to an unanswered probe. A new probe's nonce must not.
    pub fn outstanding(&self, nonce: u32) -> bool {
        self.unanswered().iter().any(|p| p.nonce == nonce)
    }

    /// Records a probe just written, after its bytes were counted with
    /// [`PathMeter::wrote`]. Ignored at [`MAX_PROBES`].
    pub fn probe(&mut self, nonce: u32, now: Time) {
        let Some(slot) = self.probes.get_mut(self.len) else {
            return;
        };
        *slot = Probe {
            nonce,
            sent: now,
            written: self.written,
        };
        self.len += 1;
        self.last_probe = now;
        self.sent = 0;
    }

    /// Makes the probe just written ([`PathMeter::probe`]) a train's first:
    /// the owner writes `len` bytes behind it back to back (filler where it
    /// has no data: [`PathMeter::train_left`]), then a closing probe
    /// ([`PathMeter::close_train`]). The closing probe's echo shows how
    /// long they took to drain ([`PathMeter::take_train`]). No other
    /// probe goes out while a train is open, for [`TRAIN_SPAN`] at most.
    /// Ignored without such a probe, or while a train is open.
    pub fn start_train(&mut self, len: u64, now: Time) {
        if self.train_open(now) || self.last_probe != now {
            return;
        }
        let Some(p) = self.unanswered().last().copied() else {
            return;
        };
        self.train = Some(OpenTrain {
            first: p.nonce,
            sent: now,
            written: p.written,
            len,
            last: None,
        });
    }

    /// A train is open: started within [`TRAIN_SPAN`], its closing probe's
    /// echo yet to come.
    pub fn train_open(&self, now: Time) -> bool {
        self.train.is_some_and(|t| now - t.sent < TRAIN_SPAN)
    }

    /// The bytes the open train still needs before its closing probe;
    /// `None` without one, or once it is closed.
    pub fn train_left(&self) -> Option<u64> {
        let t = self.train.filter(|t| t.last.is_none())?;
        Some(t.len.saturating_sub(self.written.saturating_sub(t.written)))
    }

    /// The open train's first probe is the last thing written: no byte
    /// follows it yet.
    pub fn train_unstarted(&self) -> bool {
        self.train
            .is_some_and(|t| t.last.is_none() && self.written == t.written)
    }

    /// Makes the probe just written the open train's closing one.
    pub fn close_train(&mut self) {
        let last = self.unanswered().last().copied();
        if let (Some(t), Some(p)) = (self.train.as_mut(), last)
            && t.last.is_none()
            && p.nonce != t.first
        {
            t.last = Some((p.nonce, p.written));
        }
    }

    /// Takes the echo of `nonce`, arriving at `now`. It answers its probe and
    /// every older one. The sample is `None` when the clock stepped back
    /// past the probe, and for a train's closing probe, whose echo times
    /// its train ([`PathMeter::echo_gap`]); the bytes still count as
    /// delivered.
    pub fn echo(&mut self, nonce: u32, now: Time) -> Result<Option<PathSample>, UnknownEcho> {
        self.echo_gap(nonce, None, now)
    }

    /// [`PathMeter::echo`] for an echo that carries the receiver's `gap`
    /// ([`Echo::gap`]): for a train's closing probe, how long after the
    /// first it was read, which [`PathMeter::take_train`] then gives.
    pub fn echo_gap(
        &mut self,
        nonce: u32,
        gap: Option<Duration>,
        now: Time,
    ) -> Result<Option<PathSample>, UnknownEcho> {
        let Some(i) = self.unanswered().iter().position(|p| p.nonce == nonce) else {
            self.unknown += 1;
            self.len = 0;
            self.train = None;
            return Err(UnknownEcho);
        };
        let Some(p) = self.probes.get(i).copied() else {
            return Err(UnknownEcho);
        };
        // An echo of a later probe answers the train's closing one with no
        // round trip of its own: the train measures nothing.
        let closes = self.train.and_then(|t| t.last).map(|(n, _)| n);
        let skipped = closes.is_some_and(|n| {
            n != nonce
                && self
                    .probes
                    .get(..i)
                    .unwrap_or_default()
                    .iter()
                    .any(|q| q.nonce == n)
        });
        self.probes.copy_within(i + 1..self.len, 0);
        self.len -= i + 1;
        let rate = self.acknowledge(Acked {
            sent: p.sent,
            acked: now,
            delivered: p.written,
        });
        if skipped {
            self.train = None;
        }
        let rtt = now - p.sent;
        if rtt.is_negative() {
            self.negative += 1;
            if closes == Some(nonce) {
                self.train = None;
            }
            return Ok(None);
        }
        if let Some(t) = self.train
            && let Some((n, written)) = t.last
            && n == nonce
        {
            self.leg = gap.map(|gap| LegSample {
                at: now,
                bytes: written.saturating_sub(t.written),
                gap,
            });
            self.train = None;
            return Ok(None);
        }
        self.samples += 1;
        self.delay.add(now, rtt);
        self.delay.fold();
        let in_flight = self.written.saturating_sub(p.written);
        let queued = match (rate, self.delay.base(now)) {
            (Some(r), Some(b)) => Some(in_flight.saturating_sub(bytes_in(r, b))),
            _ => None,
        };
        self.latest = Some(Latest {
            at: now,
            rtt,
            in_flight,
            rate,
            queued,
            written: self.written,
        });
        Ok(Some(PathSample { at: now, rtt }))
    }

    /// The path as of the latest echo. `None` before the first.
    pub fn reading(&self, now: Time) -> Option<PathReading> {
        let l = self.latest?;
        Some(PathReading {
            samples: self.samples,
            at: l.at,
            rtt: l.rtt,
            base: self.delay.base(now),
            recent_min: self.delay.recent_min(now),
            in_flight: l.in_flight,
            rate: l.rate,
            queued: l.queued,
            written_since: self.written.saturating_sub(l.written),
        })
    }

    /// The latest train's measure, once its closing probe's echo came; it
    /// is taken.
    pub fn take_train(&mut self) -> Option<LegSample> {
        self.leg.take()
    }

    /// Bytes put into the stream so far.
    pub fn bytes_written(&self) -> u64 {
        self.written
    }

    /// Probes not yet answered.
    pub fn unanswered_probes(&self) -> usize {
        self.len
    }

    /// Echoes of unknown probes so far.
    pub fn unknown_echoes(&self) -> u64 {
        self.unknown
    }

    /// Echoes whose round trip came out negative (the clock stepped back).
    pub fn negative_rtts(&self) -> u64 {
        self.negative
    }

    fn unanswered(&self) -> &[Probe] {
        self.probes.get(..self.len).unwrap_or_default()
    }

    /// Records an acknowledged probe and returns the delivery rate: from
    /// the oldest probe acknowledged within [`RATE_WINDOW`] (or the one
    /// before this, if none) to this one.
    fn acknowledge(&mut self, k: Acked) -> Option<u64> {
        if self.acked_len == ACKED {
            self.acked.rotate_left(1);
            self.acked_len -= 1;
        }
        let earlier = self.acked.get(..self.acked_len).unwrap_or_default();
        let j = earlier
            .iter()
            .find(|e| k.acked - e.acked <= RATE_WINDOW)
            .or(earlier.last())
            .copied();
        if let Some(slot) = self.acked.get_mut(self.acked_len) {
            *slot = k;
            self.acked_len += 1;
        }
        let j = j?;
        let span = (k.sent - j.sent).max(k.acked - j.acked);
        if span <= Duration::ZERO {
            return None;
        }
        let delivered = u128::from(k.delivered.saturating_sub(j.delivered));
        let nanos = u128::try_from(span.as_nanos()).unwrap_or(0).max(1);
        Some(u64::try_from(delivered * 1_000_000_000 / nanos).unwrap_or(u64::MAX))
    }
}

/// Bytes moved at `rate` bytes per second in `d`.
fn bytes_in(rate: u64, d: Duration) -> u64 {
    let nanos = u128::try_from(d.as_nanos()).unwrap_or(0);
    u64::try_from(u128::from(rate) * nanos / 1_000_000_000).unwrap_or(u64::MAX)
}

/// The receiving side of a link's path measure: the newest probe read,
/// waiting for its echo.
#[derive(Clone, Copy, Debug, Default)]
pub struct Echo {
    /// The probe waiting, when it was read, and for a timed probe how long
    /// after the probe before it.
    pending: Option<(u32, Time, Option<Duration>)>,
    /// When the latest probe was read.
    last_read: Option<Time>,
    last_hold: Duration,
}

impl Echo {
    /// Notes a probe read at `now`. It replaces any probe still waiting:
    /// echoing the newest answers the older ones too.
    pub fn read(&mut self, nonce: u32, now: Time) {
        self.pending = Some((nonce, now, None));
        self.last_read = Some(now);
    }

    /// Notes a timed probe read at `now`: its echo carries how long after
    /// the probe before it this one was read ([`Echo::gap`]), the time the
    /// bytes between them took to arrive (see [`UplinkTrains`]).
    pub fn read_timed(&mut self, nonce: u32, now: Time) {
        let gap = self.last_read.map(|t| (now - t).max(Duration::ZERO));
        self.pending = Some((nonce, now, gap));
        self.last_read = Some(now);
    }

    /// A probe waits for its echo.
    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// The gap the echo waiting carries, for a timed probe read after
    /// another.
    pub fn gap(&self) -> Option<Duration> {
        self.pending.and_then(|p| p.2)
    }

    /// Takes the nonce to echo now, and how long the probe waited for it.
    pub fn take(&mut self, now: Time) -> Option<(u32, Duration)> {
        let (nonce, read, _) = self.pending.take()?;
        self.last_hold = (now - read).max(Duration::ZERO);
        Some((nonce, self.last_hold))
    }

    /// How long the latest echo waited after its probe was read.
    pub fn last_hold(&self) -> Duration {
        self.last_hold
    }
}

/// One path's round trips: its base and its recent minimum (see the
/// module docs).
#[derive(Clone, Debug, Default)]
pub struct PathDelay {
    /// The latest round trips with their arrival times, oldest first.
    /// Bound: [`SAMPLES`].
    samples: [(Time, Duration); SAMPLES],
    len: usize,
    /// Per bucket, its index (arrival time over [`BASE_BUCKET`]) and the
    /// least round trip in it.
    base: [Option<(i64, Duration)>; BASE_BUCKETS],
    /// The arrival time of the newest sample folded into `base`.
    folded: Time,
}

impl PathDelay {
    /// Records a round trip that arrived at `now`.
    pub fn add(&mut self, now: Time, rtt: Duration) {
        let rtt = rtt.max(Duration::ZERO);
        if self.len == SAMPLES {
            self.samples.rotate_left(1);
            self.len -= 1;
        }
        if let Some(s) = self.samples.get_mut(self.len) {
            *s = (now, rtt);
            self.len += 1;
        }
    }

    /// Folds the samples that arrived since the last fold into the base.
    pub fn fold(&mut self) {
        let Some(newest) = self.recent_samples().last().map(|s| s.0) else {
            return;
        };
        for i in 0..self.len {
            let Some(&(t, rtt)) = self.samples.get(i) else {
                continue;
            };
            if t <= self.folded {
                continue;
            }
            let index = t.unix_nanos() / BASE_BUCKET.as_nanos();
            let slot = self
                .base
                .get_mut(index.rem_euclid(BASE_BUCKETS as i64) as usize);
            if let Some(slot) = slot {
                match slot {
                    Some((i, min)) if *i == index => *min = (*min).min(rtt),
                    _ => *slot = Some((index, rtt)),
                }
            }
        }
        self.folded = self.folded.max(newest);
    }

    fn recent_samples(&self) -> &[(Time, Duration)] {
        self.samples.get(..self.len).unwrap_or_default()
    }

    /// The least round trip of the last [`RECENT`]. `None` without a recent
    /// sample.
    pub fn recent_min(&self, now: Time) -> Option<Duration> {
        self.recent_samples()
            .iter()
            .filter(|s| now - s.0 <= RECENT)
            .map(|s| s.1)
            .min()
    }

    /// The least folded round trip of the base window.
    pub fn base(&self, now: Time) -> Option<Duration> {
        let oldest = now.unix_nanos() / BASE_BUCKET.as_nanos() - BASE_BUCKETS as i64;
        self.base
            .iter()
            .flatten()
            .filter(|(i, _)| *i > oldest)
            .map(|(_, d)| *d)
            .min()
    }

    /// How far the least round trip of the last [`RECENT`] stands above
    /// the base. `None` without a recent sample.
    pub fn excess(&self, now: Time) -> Option<Duration> {
        let current = self.recent_min(now)?;
        Some(
            self.base(now)
                .map_or(Duration::ZERO, |b| (current - b).max(Duration::ZERO)),
        )
    }
}

/// A node-wide delay from its paths' delays (`scratch` is reordered): the
/// ⌊n/3⌋+1-th smallest, so it reads a queue only when the queue delays most
/// paths. `None` with fewer than [`MIN_PATHS`].
pub fn aggregate(scratch: &mut [Duration]) -> Option<Duration> {
    if scratch.len() < MIN_PATHS {
        return None;
    }
    let k = scratch.len() / 3;
    scratch.select_nth_unstable(k);
    scratch.get(k).copied()
}

/// A node's uplink as its trains show it.
///
/// A train's leg goes on one link: a probe written alone (so that its
/// receiver reads it as it arrives, not with the packet or segment it
/// would share), filler behind it and a closing probe
/// ([`PathMeter::start_train`]), no more than its transport's window takes
/// after idling ([`TRAIN_LEG`], [`TRAIN_LEG_CHANNEL`]), and the bytes the
/// uplink measured drains in [`TRAIN_DRAIN`]. The closing probe is timed
/// ([`Echo::read_timed`]): its echo says how long after the first the
/// receiver read it, the time the leg's bytes took to arrive, which neither
/// the queue ahead of the leg, the path's base nor the way back (a
/// receiver's echoes wait behind its own uplink's queue, which drains
/// between them) enters; traffic that queues during the leg goes behind it.
/// A leg can't read more than the narrowest of the uplink, its path and its
/// receiver's downlink; a window that holds it back, a lost packet or
/// another link's bytes between its probes only lower it. Legs go one at a
/// time, apart: the data that queues behind one waits a leg's drain, not a
/// train's.
///
/// A leg's gap is its receiver's claim: a peer can shorten it. Of legs on
/// `k` links (each link's latest), the ⌊k/3⌋+1-th fastest is one no
/// minority of their peers raises past an honest leg's, and that slow
/// downlinks, lost packets and other traffic on a minority don't lower.
/// With fewer than [`MIN_PATHS`] links it is their fastest: one narrow
/// first receiver (a mobile downlink, a leg in its transport's first round
/// trips) doesn't hold the reading down until a third link has legs, and
/// the copies a raising liar among so few makes the budget pay for can't
/// pass the neighbours the node has. A leg read in one piece (both probes
/// in one read: a receiver that takes its downlink's bursts whole) drained
/// within a [`TRAIN_TICK`], at least its bytes over one. The node's uplink
/// is the lower of that over every link with a leg in the last
/// [`TRAIN_SPAN`], which a third of its neighbours can't raise however its
/// legs fell among them, and over the latest [`TRAIN_LEGS`] links, which
/// follows the uplink.
///
/// A leg slower than the uplink by more than two [`TRAIN_TICK`]s of its gap
/// (its own and its reading's) asks for the next as soon as it is measured
/// ([`UplinkTrains::falls`]), so a fall of the uplink shows within as many
/// legs' round trips as outnumber the faster ones, one leg in flight at a
/// time.
///
/// Legs go a staleness over [`TRAIN_LEGS`] apart
/// ([`UplinkTrains::spacing`]), the staleness the owner allows
/// ([`UplinkTrains::set_staleness`]), so each of the latest legs is at most
/// about that old.
#[derive(Clone, Debug, Default)]
pub struct UplinkTrains {
    /// Measured legs, oldest first. Bound: [`TRAINS`].
    legs: std::collections::VecDeque<Leg>,
    staleness: Duration,
    /// When the latest leg started ([`UplinkTrains::started`]).
    started: Time,
}

/// A measured leg ([`UplinkTrains::measured`]).
#[derive(Clone, Copy, Debug)]
struct Leg {
    at: Time,
    link: u64,
    rate: u64,
    bytes: u64,
    gap: Duration,
}

/// Whether a leg of `bytes` read with `gap` agrees with `rate`: the gap
/// that rate drains them in is within two [`TRAIN_TICK`]s of it, the leg's
/// and the one of the legs the rate came from.
fn agrees(rate: u64, bytes: u64, gap: Duration) -> bool {
    let slow = rate_over(bytes, gap + TRAIN_TICK * 2).unwrap_or(0);
    let fast = rate_over(bytes, gap - TRAIN_TICK * 2).unwrap_or(u64::MAX);
    (slow..=fast).contains(&rate)
}

impl UplinkTrains {
    /// The latest leg, measured since the latest started, is slower than
    /// the uplink by more than two [`TRAIN_TICK`]s of its gap: the next
    /// should start now, to tell a fall of the uplink from a leg that met
    /// other traffic.
    pub fn falls(&self, now: Time) -> bool {
        match self.legs.back() {
            Some(l) if now - l.at <= TRAIN_SPAN && l.at > self.started => {
                self.rate(now).is_some_and(|r| {
                    !agrees(r, l.bytes, l.gap) && rate_over(l.bytes, l.gap).is_some_and(|x| x < r)
                })
            }
            _ => false,
        }
    }

    /// A leg started at `now`.
    pub fn started(&mut self, now: Time) {
        self.started = now;
    }

    /// How stale the owner lets the reading get: legs go a staleness over
    /// [`TRAIN_LEGS`] apart.
    pub fn set_staleness(&mut self, staleness: Duration) {
        self.staleness = staleness;
    }

    /// The spacing between legs now (see the type's docs).
    pub fn spacing(&self) -> Duration {
        Duration::from_nanos(self.staleness.as_nanos() / TRAIN_LEGS as i64).max(TRAIN_DRAIN)
    }

    /// The latest leg's rate on each link measured in the last
    /// [`TRAIN_SPAN`], newest first, at most `most`.
    fn latest(&self, now: Time, most: usize) -> Vec<u64> {
        let mut links: Vec<u64> = Vec::new();
        let mut rates: Vec<u64> = Vec::new();
        for l in self.legs.iter().rev() {
            if now - l.at > TRAIN_SPAN || rates.len() == most {
                break;
            }
            if !links.contains(&l.link) {
                links.push(l.link);
                rates.push(l.rate);
            }
        }
        rates
    }

    /// A leg on `link` (any number the owner tells its links apart by), as
    /// its [`PathMeter::take_train`] timed it.
    pub fn measured(&mut self, link: u64, leg: &LegSample, now: Time) {
        let gap = leg.gap.max(TRAIN_TICK);
        let Some(rate) = rate_over(leg.bytes, gap) else {
            return;
        };
        if self.legs.len() == TRAINS {
            self.legs.pop_front();
        }
        self.legs.push_back(Leg {
            at: now,
            link,
            rate,
            bytes: leg.bytes,
            gap,
        });
    }

    /// The node's uplink in bytes per second (see the type's docs); `None`
    /// without a leg measured in the last [`TRAIN_SPAN`].
    pub fn rate(&self, now: Time) -> Option<u64> {
        let all = train_value(self.latest(now, usize::MAX));
        let recent = train_value(self.latest(now, TRAIN_LEGS));
        match (all, recent) {
            (Some(a), Some(r)) => Some(a.min(r)),
            (a, r) => a.or(r),
        }
    }
}

/// The uplink from the latest legs' rates (see [`UplinkTrains`]).
fn train_value(mut legs: Vec<u64>) -> Option<u64> {
    if legs.len() < MIN_PATHS {
        return legs.into_iter().max();
    }
    legs.sort_unstable_by(|a, b| b.cmp(a));
    legs.get(legs.len() / 3).copied()
}

/// A node's queueing delay as its links' latest echoes show it at `now`:
/// how far each link's latest round trip stands above its base, over the
/// links whose latest echo is at most [`LATEST_MAX_AGE`] old; their
/// [`aggregate`] with at least [`MIN_PATHS`], the least with fewer, `None`
/// with none (`scratch` is cleared and refilled).
///
/// It follows a queue within a probe interval and a round trip, where
/// [`PathReading::excess`] reads the least of the last [`RECENT`]: a
/// controller can act on it. One sample that met a burst moves only its
/// own link. A peer can only delay its echoes, which raises its own link's
/// reading, so with the low quantile (or the least of two) no single peer
/// lowers the node's reading, or raises it for the others' sake.
pub fn latest_excess(
    readings: impl IntoIterator<Item = PathReading>,
    now: Time,
    scratch: &mut Vec<Duration>,
) -> Option<Duration> {
    scratch.clear();
    scratch.extend(
        readings
            .into_iter()
            .filter(|r| r.age(now) <= LATEST_MAX_AGE)
            .map(|r| r.latest_excess()),
    );
    if scratch.len() >= MIN_PATHS {
        aggregate(scratch)
    } else {
        scratch.iter().copied().min()
    }
}

/// A node's queueing delay from its links' readings, two ways: the
/// [`aggregate`] of their [`PathReading::excess`]es, and of their
/// [`PathReading::queued_delay`]s. Readings without a recent round trip
/// don't count.
pub fn node_delays(
    readings: impl IntoIterator<Item = PathReading>,
) -> (Option<Duration>, Option<Duration>) {
    let (mut excess, mut queued) = (Vec::new(), Vec::new());
    for r in readings.into_iter().filter(|r| r.recent_min.is_some()) {
        excess.extend(r.excess());
        queued.extend(r.queued_delay());
    }
    (aggregate(&mut excess), aggregate(&mut queued))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn at(ms: i64) -> Time {
        Time::from_unix_millis(1_000_000 + ms)
    }

    fn ms(n: i64) -> Duration {
        Duration::from_millis(n)
    }

    /// A sender writing `rate` bytes per second in 10 ms writes over a path
    /// that delivers each probe's echo `rtt` after it was sent, probing
    /// every [`PROBE_INTERVAL`]. Returns the meter at `until`.
    fn steady(rate: u64, rtt: i64, until: i64) -> PathMeter {
        let mut m = PathMeter::new();
        let mut echoes: Vec<(i64, u32)> = Vec::new();
        let mut nonce = 0;
        for t in (0..=until).step_by(10) {
            while let Some(&(due, n)) = echoes.first()
                && due <= t
            {
                echoes.remove(0);
                m.echo(n, at(due)).unwrap();
            }
            m.wrote(rate / 100, true);
            if m.probe_due(at(t), PROBE_INTERVAL) {
                nonce += 1;
                m.wrote(9, false);
                m.probe(nonce, at(t));
                echoes.push((t + rtt, nonce));
            }
        }
        m
    }

    #[test]
    fn an_echo_gives_the_round_trip_and_the_bytes_written_during_it() {
        let mut m = PathMeter::new();
        m.wrote(9, false);
        m.probe(7, at(0));
        m.wrote(1000, true);
        let s = m.echo(7, at(40)).unwrap().unwrap();
        assert_eq!(
            s,
            PathSample {
                at: at(40),
                rtt: ms(40)
            }
        );
        let r = m.reading(at(40)).unwrap();
        assert_eq!(r.rtt, ms(40));
        assert_eq!(r.in_flight, 1000);
        assert_eq!(r.base, Some(ms(40)));
        assert_eq!(r.rate, None);
        assert_eq!(r.queued, None);
        assert_eq!(r.queued_bound(), 1000);
    }

    #[test]
    fn a_steady_sender_reads_no_queue_on_an_idle_path() {
        let m = steady(100_000, 50, 5_000);
        let r = m.reading(at(5_000)).unwrap();
        assert_eq!(r.base, Some(ms(50)));
        let rate = r.rate.unwrap();
        assert!((95_000..=105_000).contains(&rate), "{rate}");
        assert!(r.queued.unwrap() <= 1_000, "{r:?}");
        assert_eq!(r.excess(), Some(Duration::ZERO));
    }

    #[test]
    fn a_queue_reads_as_rate_times_the_excess_without_a_sawtooth() {
        let mut m = PathMeter::new();
        let mut nonce = 0;
        // 50 ms base round trip, then a standing 150 ms queue.
        let mut probes: Vec<(i64, u32)> = Vec::new();
        let mut queued = Vec::new();
        for t in (0..=20_000).step_by(10) {
            while let Some(&(sent, n)) = probes.first() {
                let rtt = if sent < 2_000 { 50 } else { 200 };
                if sent + rtt > t {
                    break;
                }
                probes.remove(0);
                m.echo(n, at(sent + rtt)).unwrap();
            }
            m.wrote(1_000, true);
            if m.probe_due(at(t), PROBE_INTERVAL) {
                nonce += 1;
                m.probe(nonce, at(t));
                probes.push((t, nonce));
            }
            if t > 10_000 {
                queued.push(m.reading(at(t)).unwrap().queued.unwrap());
            }
        }
        // About 100 kB/s × 150 ms, whenever it's read.
        for q in queued {
            assert!((13_000..=16_000).contains(&q), "{q}");
        }
        let r = m.reading(at(20_000)).unwrap();
        assert_eq!(r.excess(), Some(ms(150)));
        let d = r.queued_delay().unwrap();
        assert!((ms(130)..=ms(160)).contains(&d), "{d}");
    }

    #[test]
    fn a_stalled_peer_only_grows_the_bound() {
        let mut m = steady(100_000, 50, 2_000);
        let before = m.reading(at(2_000)).unwrap();
        let mut last = before.queued_bound();
        for t in 1..100 {
            m.wrote(1_000, true);
            let r = m.reading(at(2_000 + t * 10)).unwrap();
            assert_eq!(r.queued, before.queued);
            assert!(r.queued_bound() > last);
            last = r.queued_bound();
        }
    }

    #[test]
    fn an_echo_answers_every_older_probe() {
        let mut m = PathMeter::new();
        for n in 1..=5 {
            m.wrote(9, false);
            m.probe(n, at(i64::from(n) * 10));
        }
        assert_eq!(m.unanswered_probes(), 5);
        m.echo(3, at(100)).unwrap();
        assert_eq!(m.unanswered_probes(), 2);
        assert!(!m.outstanding(1) && !m.outstanding(3) && m.outstanding(4));
        assert_eq!(m.echo(1, at(110)), Err(UnknownEcho));
    }

    #[test]
    fn an_unknown_echo_is_ignored_and_voids_the_probes() {
        let mut m = PathMeter::new();
        m.wrote(9, false);
        m.probe(1, at(0));
        m.wrote(9, false);
        m.probe(2, at(10));
        assert_eq!(m.echo(99, at(20)), Err(UnknownEcho));
        assert_eq!(m.unknown_echoes(), 1);
        assert_eq!(m.unanswered_probes(), 0);
        assert!(m.reading(at(20)).is_none());
        // Even the right nonce no longer counts.
        assert_eq!(m.echo(2, at(30)), Err(UnknownEcho));
        assert!(m.reading(at(30)).is_none());
    }

    #[test]
    fn a_replayed_echo_is_unknown() {
        let mut m = PathMeter::new();
        m.wrote(9, false);
        m.probe(1, at(0));
        m.echo(1, at(30)).unwrap();
        m.wrote(9, false);
        m.probe(2, at(200));
        assert_eq!(m.echo(1, at(210)), Err(UnknownEcho));
        assert_eq!(m.reading(at(210)).unwrap().samples, 1);
    }

    #[test]
    fn probes_wait_at_the_bound_and_none_is_forgotten() {
        let mut m = PathMeter::new();
        for n in 0..MAX_PROBES as u32 {
            m.wrote(BUSY_BYTES, true);
            assert!(m.probe_due(at(i64::from(n) * 200), PROBE_INTERVAL));
            m.probe(n + 1, at(i64::from(n) * 200));
        }
        m.wrote(1000, true);
        assert!(!m.probe_due(at(10_000), PROBE_INTERVAL));
        // The oldest is still answerable after any time.
        let s = m.echo(1, at(10_000)).unwrap().unwrap();
        assert_eq!(s.rtt, ms(10_000));
        assert!(m.probe_due(at(10_000), PROBE_INTERVAL));
    }

    /// A 128 kbit/s stream sends about 3.2 KB a 200 ms unit: its links
    /// are busy, probed at the interval.
    #[test]
    fn a_partial_chunk_an_interval_is_busy() {
        let mut m = PathMeter::new();
        m.probe(1, at(0));
        for n in 1..10 {
            m.wrote(3200, true);
            assert!(m.probe_due(at(n * 200), PROBE_INTERVAL), "{n}");
            m.probe(n as u32 + 1, at(n * 200));
        }
    }

    #[test]
    fn data_makes_a_probe_due_sooner_the_more_of_it() {
        let mut m = PathMeter::new();
        assert!(m.probe_due(at(0), PROBE_INTERVAL));
        m.wrote(9, false);
        m.probe(1, at(0));
        // Probes and echoes alone never do.
        m.wrote(9, false);
        assert!(!m.probe_due(at(5_000), PROBE_INTERVAL));
        // A little data, after the quiet interval.
        m.wrote(100, true);
        assert!(!m.probe_due(at(200), PROBE_INTERVAL));
        assert!(!m.probe_due(at(999), PROBE_INTERVAL));
        assert!(m.probe_due(at(1_000), PROBE_INTERVAL));
        // A low-bitrate stream's partial chunk, after the probe interval.
        m.wrote(BUSY_BYTES, true);
        assert!(!m.probe_due(at(199), PROBE_INTERVAL));
        assert!(m.probe_due(at(200), PROBE_INTERVAL));
    }

    #[test]
    fn delayed_echoes_read_farther_and_fuller_never_nearer_or_emptier() {
        let honest = steady(100_000, 50, 10_000);
        let h = honest.reading(at(10_000)).unwrap();
        // The same path, with every echo held another 100 ms.
        let held = steady(100_000, 150, 10_000);
        let d = held.reading(at(10_000)).unwrap();
        assert!(d.rtt > h.rtt);
        assert!(d.in_flight > h.in_flight);
        assert!(d.rate.unwrap() <= h.rate.unwrap() + 1_000);
        // A steady hold cancels out of the queued bytes.
        assert!(d.queued.unwrap() <= 1_000, "{d:?}");
    }

    #[test]
    fn bunched_echoes_cant_raise_the_rate_past_the_write_rate() {
        let mut m = PathMeter::new();
        for n in 1..=10 {
            m.wrote(10_000, true);
            m.wrote(9, false);
            m.probe(n, at(i64::from(n) * 100));
        }
        // 100 kB/s written; the echoes arrive within 2 ms.
        for n in 1..=10 {
            m.echo(n, at(2_000 + i64::from(n) / 5)).unwrap();
        }
        let rate = m.reading(at(2_002)).unwrap().rate.unwrap();
        assert!(rate <= 101_000, "{rate}");
    }

    #[test]
    fn a_negative_round_trip_gives_no_sample() {
        let mut m = PathMeter::new();
        m.wrote(9, false);
        m.probe(1, at(100));
        assert_eq!(m.echo(1, at(50)), Ok(None));
        assert_eq!(m.negative_rtts(), 1);
        assert!(m.reading(at(50)).is_none());
        assert_eq!(m.unanswered_probes(), 0);
    }

    #[test]
    fn the_newest_probe_read_is_echoed_with_its_hold() {
        let mut e = Echo::default();
        assert_eq!(e.take(at(0)), None);
        e.read(1, at(0));
        e.read(2, at(10));
        assert!(e.is_pending());
        assert_eq!(e.take(at(15)), Some((2, ms(5))));
        assert_eq!(e.last_hold(), ms(5));
        assert!(!e.is_pending());
    }

    #[test]
    fn a_timed_probe_is_echoed_with_its_gap() {
        let mut e = Echo::default();
        e.read(1, at(0));
        assert_eq!(e.gap(), None);
        e.take(at(1));
        e.read_timed(2, at(12));
        assert_eq!(e.gap(), Some(ms(12)));
        assert_eq!(e.take(at(30)), Some((2, ms(18))));
        assert_eq!(e.gap(), None);
    }

    /// A leg of `bytes` on a fresh meter, its closing probe read `gap`
    /// after its first, echoed at `echo`.
    fn leg(bytes: u64, gap: Duration, echo: i64) -> LegSample {
        let mut m = PathMeter::new();
        m.wrote(9, false);
        m.probe(1, at(0));
        m.start_train(bytes, at(0));
        assert!(m.train_unstarted());
        m.wrote(bytes, false);
        assert_eq!(m.train_left(), Some(0));
        m.wrote(9, false);
        m.probe(2, at(1));
        m.close_train();
        m.echo(1, at(echo - 5)).unwrap();
        assert!(m.train_open(at(echo - 5)));
        assert_eq!(m.echo_gap(2, Some(gap), at(echo)), Ok(None));
        assert!(!m.train_open(at(echo)));
        m.take_train().unwrap()
    }

    #[test]
    fn a_leg_reads_its_bytes_over_the_receivers_gap() {
        let l = leg(12_000, ms(10), 300);
        assert_eq!((l.bytes, l.gap, l.at), (12_009, ms(10), at(300)));
        let mut t = UplinkTrains::default();
        t.measured(1, &l, at(300));
        assert_eq!(t.rate(at(300)), Some(1_200_900));
        assert_eq!(t.rate(at(300) + TRAIN_SPAN + ms(1)), None);
    }

    #[test]
    fn one_peer_cant_raise_the_uplink() {
        let mut t = UplinkTrains::default();
        let honest = [10, 11, 12];
        for (i, g) in honest.into_iter().enumerate() {
            t.measured(i as u64, &leg(10_000, ms(g), 100), at(100 + i as i64));
        }
        let clean = t.rate(at(200));
        // A liar's legs, however many, count once, and only as its latest.
        for i in 0..5 {
            t.measured(9, &leg(10_000, ms(1), 100), at(110 + i));
        }
        let lied = t.rate(at(200));
        assert!(lied <= Some(1_000_900), "{lied:?}");
        assert!(clean <= lied);
        assert_eq!(t.latest(at(200), TRAIN_LEGS).len(), TRAIN_LEGS);
    }

    #[test]
    fn a_minority_of_liars_cant_raise_the_uplink_from_the_latest_legs() {
        let mut t = UplinkTrains::default();
        for i in 0..6 {
            t.measured(i, &leg(10_000, ms(10), 100), at(100 + i as i64));
        }
        // Two of eight lie, and theirs are two of the latest four legs.
        t.measured(6, &leg(10_000, ms(1), 100), at(120));
        t.measured(7, &leg(10_000, ms(1), 100), at(121));
        assert_eq!(t.rate(at(200)), Some(1_000_900));
    }

    #[test]
    fn a_leg_slower_by_more_than_two_ticks_falls() {
        let mut t = UplinkTrains::default();
        t.set_staleness(ms(1500));
        assert_eq!(t.spacing(), ms(500));
        for i in 0..3 {
            t.measured(i, &leg(10_000, ms(10), 100), at(100 + i as i64));
        }
        t.measured(3, &leg(10_000, ms(12), 100), at(110));
        assert!(!t.falls(at(110)));
        t.measured(4, &leg(10_000, ms(20), 100), at(120));
        assert!(t.falls(at(120)));
        t.measured(5, &leg(10_000, ms(5), 100), at(130));
        assert!(!t.falls(at(130)));
    }

    #[test]
    fn a_fall_chains_one_leg_at_a_time() {
        let mut t = UplinkTrains::default();
        for i in 0..3 {
            t.measured(i, &leg(10_000, ms(10), 100), at(100 + i as i64));
        }
        t.measured(3, &leg(10_000, ms(20), 100), at(110));
        assert!(t.falls(at(110)));
        // The chained leg is in flight: no other starts before it is measured.
        t.started(at(110));
        assert!(!t.falls(at(110)));
        assert!(!t.falls(at(150)));
        t.measured(4, &leg(10_000, ms(30), 100), at(160));
        assert!(t.falls(at(160)));
    }

    /// A leg of 10 kB read in `gap` ms.
    fn gap_leg(gap: i64) -> LegSample {
        LegSample {
            at: at(100),
            bytes: 10_000,
            gap: ms(gap),
        }
    }

    #[test]
    fn below_three_links_the_fastest_leg_reads_and_a_whole_leg_drained_in_a_tick() {
        let mut t = UplinkTrains::default();
        t.measured(1, &gap_leg(100), at(100));
        assert_eq!(t.rate(at(120)), Some(100_000));
        t.measured(2, &gap_leg(0), at(101));
        assert_eq!(t.rate(at(120)), Some(10_000_000));
        // From three, a third of the links can't raise it.
        t.measured(3, &gap_leg(10), at(102));
        assert_eq!(t.rate(at(120)), Some(1_000_000));
    }

    #[test]
    fn excess_is_the_recent_minimum_over_the_base() {
        let mut p = PathDelay::default();
        p.add(at(0), ms(40));
        p.fold();
        p.add(at(5000), ms(140));
        p.add(at(6000), ms(120));
        p.fold();
        assert_eq!(p.excess(at(6000)), Some(ms(80)));
    }

    #[test]
    fn a_standing_queue_doesnt_raise_the_base() {
        let mut p = PathDelay::default();
        p.add(at(0), ms(40));
        p.fold();
        for s in 1..200 {
            p.add(at(s * 1000), ms(240));
            p.fold();
        }
        assert_eq!(p.excess(at(199_000)), Some(ms(200)));
    }

    #[test]
    fn a_base_ages_out_of_its_window() {
        let mut p = PathDelay::default();
        p.add(at(0), ms(40));
        p.fold();
        let later = BASE_BUCKET.as_millis() * (BASE_BUCKETS as i64 + 1);
        p.add(at(later), ms(240));
        p.fold();
        assert_eq!(p.excess(at(later)), Some(Duration::ZERO));
    }

    #[test]
    fn stale_samples_give_no_reading() {
        let mut p = PathDelay::default();
        p.add(at(0), ms(40));
        p.fold();
        assert_eq!(p.excess(at(RECENT.as_millis() + 1)), None);
    }

    fn reading(at_ms: i64, rtt: i64, base: i64) -> PathReading {
        PathReading {
            samples: 1,
            at: at(at_ms),
            rtt: ms(rtt),
            base: Some(ms(base)),
            recent_min: Some(ms(rtt)),
            in_flight: 0,
            rate: None,
            queued: None,
            written_since: 0,
        }
    }

    #[test]
    fn the_latest_excess_reads_fresh_links_and_resists_one_delayer() {
        let mut v = Vec::new();
        let now = 10_000;
        // Three fresh links read 20, 30 and 40 ms over their bases; a
        // fourth delays its echoes by 2 s; a fifth is stale.
        let links = [
            reading(now - 100, 60, 40),
            reading(now - 50, 80, 50),
            reading(now - 300, 70, 30),
            reading(now - 10, 2_040, 40),
            reading(now - 5_000, 40, 40),
        ];
        assert_eq!(latest_excess(links, at(now), &mut v), Some(ms(30)));
        // The delayer can't close it for the others: two links, the least.
        let two = [reading(now, 60, 40), reading(now, 2_040, 40)];
        assert_eq!(latest_excess(two, at(now), &mut v), Some(ms(20)));
        assert_eq!(latest_excess([reading(0, 60, 40)], at(now), &mut v), None);
    }

    #[test]
    fn a_readings_latest_excess_is_its_latest_round_trip_over_its_base() {
        // However old, and however far the recent least stands.
        let mut r = reading(0, 90, 40);
        r.recent_min = Some(ms(50));
        assert_eq!(r.latest_excess(), ms(50));
        assert_eq!(reading(0, 30, 40).latest_excess(), Duration::ZERO);
        r.base = None;
        assert_eq!(r.latest_excess(), Duration::ZERO);
    }

    #[test]
    fn one_slow_path_doesnt_move_the_aggregate() {
        let mut d = vec![ms(2), ms(500), ms(3), ms(1)];
        assert_eq!(aggregate(&mut d), Some(ms(2)));
        let mut queued = vec![ms(90), ms(500), ms(95), ms(80)];
        assert_eq!(aggregate(&mut queued), Some(ms(90)));
        assert_eq!(aggregate(&mut [ms(90), ms(95)]), None);
    }
}
