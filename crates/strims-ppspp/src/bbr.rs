// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A BBR-style request controller: a model of one peer's delivery rate and
//! minimum round trip, measured on the local clock only. See [`BbrLike`].

use strims_base::time::{Duration, Time};

use crate::controller::RequestController;
use crate::windowed::{WindowedMax, WindowedMin};

/// The in-flight cap never goes below this many chunks. Kept small: at a
/// peer that many channels ask and that can't serve them all, each
/// outstanding request may duplicate another channel's, and the peer's
/// upload goes to copies of the same chunks.
pub const FLOOR_CHUNKS: i64 = 2;
/// The cap of a new channel before it has measured anything.
pub const INITIAL_CHUNKS: i64 = 32;
/// How long the minimum round trip holds before it expires (BBR's
/// `min_rtt_win_sec`).
pub const MIN_RTT_WINDOW: Duration = Duration::from_secs(10);
/// How long a delivery-rate sample counts toward the maximum.
pub const RATE_WINDOW: Duration = Duration::from_secs(2);
/// Request rounds remembered for delivery-rate samples: an answer to a
/// request older than the oldest gives no sample.
const SEND_MARKS: usize = 64;
/// The shortest interval a delivery-rate sample covers; otherwise a
/// round trip.
pub const SAMPLE_MIN: Duration = Duration::from_millis(50);
/// Ping round trips bound the minimum round trip from below over this
/// window.
pub const PING_WINDOW: Duration = Duration::from_secs(3);
/// Pings agree when the largest over [`PING_WINDOW`] is at most this share
/// above the smallest, or [`PING_STEADY_MIN`].
const PING_STEADY: Gain = (1, 4);
/// See `PING_STEADY`.
pub const PING_STEADY_MIN: Duration = Duration::from_millis(10);
/// How long ProbeRTT holds the cap at half the bandwidth-delay product
/// (at least a round trip).
pub const PROBE_RTT_DURATION: Duration = Duration::from_millis(200);
/// The request timeout's margin over the smallest round trip.
pub const TIMEOUT_MIN_SPREAD: Duration = Duration::from_millis(100);
/// The request timeout before any round trip is known.
pub const TIMEOUT_INITIAL: Duration = Duration::SECOND;
/// The longest request timeout.
pub const TIMEOUT_MAX: Duration = Duration::from_secs(8);
/// Each round with a timeout doubles the request timeout, up to this many
/// times, until an answer arrives.
pub const MAX_BACKOFF: u32 = 3;
/// Round trips count as at least this: the clock's precision (nodes keep
/// time in milliseconds), so a sample is never zero.
pub const RTT_GRANULARITY: Duration = Duration::MILLISECOND;

/// Gains as (numerator, denominator), so the arithmetic is exact.
type Gain = (i64, i64);
/// Startup's gain, BBR's 2/ln 2.
const STARTUP_GAIN: Gain = (2885, 1000);
/// ProbeBW's cap is this many bandwidth-delay products (BBR's
/// `cwnd_gain`), times the phase's gain.
const CWND_GAIN: Gain = (2, 1);
/// ProbeBW's phases: probe up, drain, cruise.
const PROBE_CYCLE: [Gain; 8] = [
    (5, 4),
    (3, 4),
    (1, 1),
    (1, 1),
    (1, 1),
    (1, 1),
    (1, 1),
    (1, 1),
];
/// A round whose timed-out bytes exceed this share of its settled bytes
/// (answered or timed out) bounds the cap.
const LOSS_THRESH: Gain = (2, 100);
/// The bound after such a round: this share of the round's peak flight.
const LOSS_BETA: Gain = (7, 10);
/// Each round without such losses raises the bound by this share.
const LOSS_RECOVERY: Gain = (1, 4);
/// Startup ends after this many rounds without the rate growing by
/// `FULL_BW_GROWTH`.
const FULL_BW_ROUNDS: u32 = 3;
const FULL_BW_GROWTH: Gain = (5, 4);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Grows the cap by `STARTUP_GAIN` per round until the delivery rate
    /// stops growing.
    Startup,
    /// One bandwidth-delay product until the flight drains to it.
    Drain,
    /// Twice the bandwidth-delay product, cycling through a probe up and a
    /// drain phase.
    ProbeBw,
    /// Half the bandwidth-delay product for [`PROBE_RTT_DURATION`], to
    /// measure the round trip without our own queue.
    ProbeRtt,
}

/// A BBR-style model of one peer for a channel's request window, after BBR
/// v1 and v2 (Cardwell et al.) applied to requests and the data answering
/// them.
///
/// - **Delivery rate:** per answered request, the verified bytes the peer
///   delivered (pushed or pulled) between the request and its answer, over
///   that time; the maximum over [`RATE_WINDOW`]. Measured over each
///   request's lifetime, a burst (a whole frame of chunks at once) reads as
///   the rate it was asked for at.
/// - **Minimum round trip:** request to data, on our clock; the smallest
///   over [`MIN_RTT_WINDOW`] (pings stand in until the first answer). When
///   it expires the next sample replaces it, and ProbeRTT lasts until one
///   has. While recent pings agree on a higher round trip, the path got
///   slower and that is the minimum (see `PING_STEADY`).
/// - **In-flight cap:** a gain times rate times minimum round trip, at
///   least [`FLOOR_CHUNKS`]. Startup's gain is 2.89 until the rate stops
///   growing by 25 % for 3 rounds, then Drain (1x) until the flight fits,
///   then ProbeBW (2x, times 1.25, 0.75 and six 1s, a phase per round),
///   with ProbeRTT (0.5x) when the minimum round trip expires. A round is
///   a minimum round trip, at least [`SAMPLE_MIN`].
/// - **Loss bound:** a round in which more than 2 % of the settled bytes
///   timed out bounds the cap by 0.7 times that round's peak flight; each
///   clean round relaxes the bound by a quarter. Timeouts mean the peer
///   can't serve our requests in time.
/// - **Request timeout:** the smoothed request-to-data round trip plus
///   four deviations (RFC 6298), at least 1.25 times the largest round trip
///   of the last [`RATE_WINDOW`] (a round's requests are answered one after
///   another, and the smoothed estimate trails the last ones), the minimum
///   round trip plus [`TIMEOUT_MIN_SPREAD`], and the ping round trip plus
///   its spread; doubled for each round that timed out (at most
///   [`MAX_BACKOFF`] times, and not past [`TIMEOUT_MAX`]) until an answer
///   arrives.
/// - **Answer delay:** the smoothed request-to-data round trip.
///
/// Invariants: the peer's data timestamps are never read, so clock skew
/// can't move the model; the state is fixed-size (scalars, four
/// `windowed::BUCKETS`-bucket windows and a `SEND_MARKS`-entry ring); the
/// arithmetic is integer and saturating.
#[derive(Clone, Debug)]
pub struct BbrLike {
    /// Bytes per chunk (the floor and initial cap are in chunks).
    chunk: i64,
    mode: Mode,
    /// Bytes requested and not answered, lost or settled.
    flight: i64,
    /// Bytes delivered so far.
    delivered: i64,
    /// The current round's start.
    interval_start: Time,
    /// For the last `SEND_MARKS` request rounds: when they were sent, and
    /// the bytes delivered by then (a ring; `Time::NIL` is empty).
    send_marks: [(Time, i64); SEND_MARKS],
    send_mark_next: usize,
    /// Delivery-rate samples, bytes per second.
    rate: WindowedMax,
    /// Request-to-data round trips, nanoseconds.
    max_rtt: WindowedMax,
    /// The latest event's time.
    last_now: Time,
    /// The minimum round trip, zero while unknown, and when it was set.
    min_rtt: Duration,
    min_rtt_stamp: Time,
    /// `min_rtt` comes from a ping, not from an answer.
    min_rtt_from_ping: bool,
    /// Request-to-data round trips, smoothed (RFC 6298).
    srtt: Duration,
    rttvar: Duration,
    have_rtt: bool,
    /// The smallest and largest ping round trips over [`PING_WINDOW`].
    ping_min: WindowedMin,
    ping_max: WindowedMax,
    /// Ping round trips, smoothed.
    ping_srtt: Duration,
    ping_var: Duration,
    have_ping: bool,
    /// Timeout doublings since the last answer.
    backoff: u32,
    /// Startup: the rate to beat and the rounds it wasn't.
    full_bw: u64,
    full_bw_rounds: u32,
    full_pipe: bool,
    /// ProbeBW's phase in `PROBE_CYCLE`.
    phase: usize,
    /// ProbeRTT ends then.
    probe_rtt_until: Time,
    /// The cap, as of the last event.
    cap: i64,
    rounds: u64,
    /// An upper bound on the cap from rounds with losses; `i64::MAX` when
    /// none holds.
    inflight_hi: i64,
    /// This round: bytes answered, bytes timed out, and the peak flight.
    round_answered: i64,
    round_lost: i64,
    round_flight_peak: i64,
}

impl BbrLike {
    /// A controller for chunks of `chunk_size` bytes. `phase_seed` picks
    /// the ProbeBW phase it starts in (spreading channels' probes).
    pub fn new(chunk_size: usize, phase_seed: u64, now: Time) -> BbrLike {
        let chunk = i64::try_from(chunk_size).unwrap_or(i64::MAX).max(1);
        let cruise = PROBE_CYCLE.len() - 2;
        BbrLike {
            chunk,
            mode: Mode::Startup,
            flight: 0,
            delivered: 0,
            interval_start: now,
            send_marks: [(Time::NIL, 0); SEND_MARKS],
            send_mark_next: 0,
            rate: WindowedMax::new(RATE_WINDOW),
            max_rtt: WindowedMax::new(RATE_WINDOW),
            last_now: now,
            min_rtt: Duration::ZERO,
            min_rtt_stamp: now,
            min_rtt_from_ping: false,
            srtt: Duration::ZERO,
            rttvar: Duration::ZERO,
            have_rtt: false,
            ping_min: WindowedMin::new(PING_WINDOW),
            ping_max: WindowedMax::new(PING_WINDOW),
            ping_srtt: Duration::ZERO,
            ping_var: Duration::ZERO,
            have_ping: false,
            backoff: 0,
            full_bw: 0,
            full_bw_rounds: 0,
            full_pipe: false,
            phase: 2 + usize::try_from(phase_seed % cruise as u64).unwrap_or(0),
            probe_rtt_until: now,
            cap: INITIAL_CHUNKS.saturating_mul(chunk),
            rounds: 0,
            inflight_hi: i64::MAX,
            round_answered: 0,
            round_lost: 0,
            round_flight_peak: 0,
        }
    }

    /// The current mode.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Rounds (delivery-rate intervals) so far.
    pub fn rounds(&self) -> u64 {
        self.rounds
    }

    /// The largest delivery-rate sample in the window ending at `now`.
    fn max_rate(&self, now: Time) -> u64 {
        u64::try_from(self.rate.get(now)).unwrap_or(0)
    }

    fn add_rate_sample(&mut self, now: Time, rate: u64) {
        self.rate.add(now, i64::try_from(rate).unwrap_or(i64::MAX));
    }

    /// The minimum round trip the model uses: the request-to-data minimum,
    /// but at least the path's round trip while recent pings agree on it.
    ///
    /// Pings bypass the peer's data queue, so a request-to-data minimum
    /// below steady pings predates a rise in the path's delay, and until it
    /// expires it would hold the cap to the old path's bandwidth-delay
    /// product. Pings that disagree (the largest more than
    /// `PING_STEADY` above the smallest) wait on a busy sender's frames,
    /// which is queueing, not path delay: they don't count.
    fn base_rtt(&self) -> Duration {
        if self.min_rtt <= Duration::ZERO {
            return Duration::ZERO;
        }
        let (Some(lo), hi) = (
            self.ping_min.get(self.last_now),
            self.ping_max.get(self.last_now),
        ) else {
            return self.min_rtt;
        };
        let (n, d) = PING_STEADY;
        let spread = i128::from(lo) * i128::from(n) / i128::from(d);
        if i128::from(hi) > i128::from(lo) + spread.max(i128::from(PING_STEADY_MIN.as_nanos())) {
            return self.min_rtt;
        }
        self.min_rtt.max(Duration::from_nanos(lo))
    }

    /// Rate times minimum round trip, in bytes; `None` while either is
    /// unknown.
    fn bdp(&self, now: Time) -> Option<i64> {
        let rate = self.max_rate(now);
        let base = self.base_rtt();
        if rate == 0 || base <= Duration::ZERO {
            return None;
        }
        let b = i128::from(rate) * i128::from(base.as_nanos())
            / i128::from(Duration::SECOND.as_nanos());
        Some(i64::try_from(b).unwrap_or(i64::MAX))
    }

    fn floor(&self) -> i64 {
        FLOOR_CHUNKS.saturating_mul(self.chunk)
    }

    fn gain(bytes: i64, (n, d): Gain) -> i64 {
        i64::try_from(i128::from(bytes) * i128::from(n) / i128::from(d.max(1))).unwrap_or(i64::MAX)
    }

    fn update_cap(&mut self, now: Time) {
        let initial = INITIAL_CHUNKS.saturating_mul(self.chunk);
        let cap = match (self.mode, self.bdp(now)) {
            (Mode::Startup, None) => initial,
            (Mode::Startup, Some(b)) => Self::gain(b, STARTUP_GAIN).max(initial),
            (_, None) => self.floor(),
            (Mode::Drain, Some(b)) => b,
            (Mode::ProbeBw, Some(b)) => {
                let phase = PROBE_CYCLE.get(self.phase).copied().unwrap_or((1, 1));
                Self::gain(Self::gain(b, CWND_GAIN), phase)
            }
            (Mode::ProbeRtt, Some(b)) => b / 2,
        };
        self.cap = cap.min(self.inflight_hi).max(self.floor());
    }

    fn sample_interval(&self) -> Duration {
        self.base_rtt().max(SAMPLE_MIN)
    }

    fn min_rtt_expired(&self, now: Time) -> bool {
        self.min_rtt > Duration::ZERO && now - self.min_rtt_stamp > MIN_RTT_WINDOW
    }

    fn enter_probe_rtt(&mut self, now: Time) {
        if self.mode != Mode::ProbeRtt {
            self.mode = Mode::ProbeRtt;
            self.probe_rtt_until = now + PROBE_RTT_DURATION.max(self.min_rtt);
        }
    }

    /// Ends the round if it lasted a round trip, and runs the per-round
    /// state machine.
    fn advance(&mut self, now: Time) {
        self.last_now = now;
        if now - self.interval_start >= self.sample_interval() {
            self.interval_start = now;
            self.end_round(now);
        }
        self.update_cap(now);
    }

    /// Remembers what was delivered when a request round went out.
    fn mark_send(&mut self, now: Time) {
        let last = (self.send_mark_next + SEND_MARKS - 1) % SEND_MARKS;
        if self.send_marks.get(last).is_some_and(|&(t, _)| t == now) {
            return;
        }
        if let Some(m) = self.send_marks.get_mut(self.send_mark_next) {
            *m = (now, self.delivered);
        }
        self.send_mark_next = (self.send_mark_next + 1) % SEND_MARKS;
    }

    /// A delivery-rate sample for an answer to a request sent at `sent`:
    /// everything the peer delivered from then to now, over that time. A
    /// burst (a whole frame of chunks at once) counts over the time it
    /// took to arrive since it was asked for, so batching doesn't read as
    /// a higher rate.
    fn sample_rate(&mut self, now: Time, sent: Time) {
        let Some(&(_, before)) = self.send_marks.iter().find(|&&(t, _)| t == sent) else {
            return;
        };
        let elapsed = (now - sent).max(RTT_GRANULARITY);
        let bytes = self.delivered.saturating_sub(before).max(0);
        let rate = i128::from(bytes) * i128::from(Duration::SECOND.as_nanos())
            / i128::from(elapsed.as_nanos());
        self.add_rate_sample(now, u64::try_from(rate).unwrap_or(u64::MAX));
    }

    fn end_round(&mut self, now: Time) {
        self.rounds = self.rounds.saturating_add(1);
        self.update_loss_bound();
        let rate = self.max_rate(now);
        if !self.full_pipe {
            let (n, d) = FULL_BW_GROWTH;
            if i128::from(rate) * i128::from(d) >= i128::from(self.full_bw) * i128::from(n) {
                self.full_bw = rate;
                self.full_bw_rounds = 0;
            } else {
                self.full_bw_rounds += 1;
                if self.full_bw_rounds >= FULL_BW_ROUNDS {
                    self.full_pipe = true;
                    if self.mode == Mode::Startup {
                        self.mode = Mode::Drain;
                    }
                }
            }
        }
        match self.mode {
            Mode::Startup => {}
            Mode::Drain => {
                if self.bdp(now).is_none_or(|b| self.flight <= b) {
                    self.mode = Mode::ProbeBw;
                }
            }
            Mode::ProbeBw => self.phase = (self.phase + 1) % PROBE_CYCLE.len(),
            Mode::ProbeRtt => {
                // Over once it lasted and an answer renewed the minimum
                // (the first answer after expiry replaces it).
                if now >= self.probe_rtt_until && !self.min_rtt_expired(now) {
                    self.mode = if self.full_pipe {
                        Mode::ProbeBw
                    } else {
                        Mode::Startup
                    };
                }
            }
        }
        if self.min_rtt_expired(now) {
            self.enter_probe_rtt(now);
        }
    }

    /// Bounds the cap after a lossy round, and relaxes the bound after a
    /// clean one. Timeouts at a peer mean its queue can't serve our requests
    /// in time; requests beyond what it serves only wait there, and at a
    /// peer many channels ask, they overlap with theirs and spend its upload
    /// on copies of the same chunks.
    fn update_loss_bound(&mut self) {
        let settled = self.round_answered.saturating_add(self.round_lost);
        let (n, d) = LOSS_THRESH;
        let lossy = self.round_lost > 0
            && i128::from(self.round_lost) * i128::from(d) > i128::from(settled) * i128::from(n);
        if lossy {
            let peak = self.round_flight_peak.max(self.flight);
            self.inflight_hi = Self::gain(peak, LOSS_BETA)
                .min(self.inflight_hi)
                .max(self.floor());
        } else if self.inflight_hi != i64::MAX {
            let step = Self::gain(self.inflight_hi, LOSS_RECOVERY).max(self.chunk);
            self.inflight_hi = self.inflight_hi.saturating_add(step);
            if self.inflight_hi >= self.cap.saturating_mul(4) {
                self.inflight_hi = i64::MAX;
            }
        }
        self.round_answered = 0;
        self.round_lost = 0;
        self.round_flight_peak = self.flight;
    }

    fn base_timeout(&self) -> Duration {
        // The largest recent round trip bounds it from below: a round's
        // requests are answered one after another, and the smoothed
        // estimate lags behind the last ones.
        let recent = Duration::from_nanos(self.max_rtt.get(self.last_now));
        let data = self.have_rtt.then(|| {
            (self.srtt + self.rttvar * 4)
                .max(self.base_rtt() + TIMEOUT_MIN_SPREAD)
                .max(recent + recent / 4)
        });
        let ping = self
            .have_ping
            .then(|| self.ping_srtt + (self.ping_var * 4).max(TIMEOUT_MIN_SPREAD));
        match (data, ping) {
            (Some(a), Some(b)) => a.max(b),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => TIMEOUT_INITIAL,
        }
    }
}

/// One RFC 6298 step: `mean` moves 1/8 and `var` 1/4 toward the sample.
fn smooth(mean: &mut Duration, var: &mut Duration, have: &mut bool, sample: Duration) {
    if *have {
        let err = (*mean - sample).as_nanos().saturating_abs();
        *var = *var + (Duration::from_nanos(err) - *var) / 4;
        *mean = *mean + (sample - *mean) / 8;
    } else {
        *mean = sample;
        *var = sample / 2;
        *have = true;
    }
}

impl RequestController for BbrLike {
    fn on_round(&mut self, now: Time, _stream_rate: u64) {
        self.last_now = now;
        self.advance(now);
    }

    fn on_sent(&mut self, now: Time, bytes: i64) {
        self.mark_send(now);
        self.flight = self.flight.saturating_add(bytes.max(0));
        self.round_flight_peak = self.round_flight_peak.max(self.flight);
        self.advance(now);
    }

    fn on_delivered(&mut self, now: Time, bytes: i64) {
        self.advance(now);
        self.delivered = self.delivered.saturating_add(bytes.max(0));
    }

    fn on_answered(&mut self, now: Time, bytes: i64, sent: Time, _peer_time: Time) {
        self.flight = self.flight.saturating_sub(bytes.max(0)).max(0);
        self.round_answered = self.round_answered.saturating_add(bytes.max(0));
        self.backoff = 0;
        let rtt = now - sent;
        if rtt < Duration::ZERO {
            return;
        }
        let rtt = rtt.max(RTT_GRANULARITY);
        self.last_now = now;
        self.max_rtt.add(now, rtt.as_nanos());
        self.sample_rate(now, sent);
        smooth(&mut self.srtt, &mut self.rttvar, &mut self.have_rtt, rtt);
        let expired = self.min_rtt_expired(now);
        if self.min_rtt <= Duration::ZERO || self.min_rtt_from_ping || rtt < self.min_rtt || expired
        {
            self.min_rtt = rtt;
            self.min_rtt_stamp = now;
            self.min_rtt_from_ping = false;
            if expired {
                self.enter_probe_rtt(now);
            }
        }
        self.update_cap(now);
    }

    fn on_timed_out(&mut self, now: Time, bytes: i64) {
        self.flight = self.flight.saturating_sub(bytes.max(0)).max(0);
        self.round_lost = self.round_lost.saturating_add(bytes.max(0));
        self.backoff = (self.backoff + 1).min(MAX_BACKOFF);
        self.advance(now);
    }

    fn on_settled(&mut self, now: Time, bytes: i64) {
        self.flight = self.flight.saturating_sub(bytes.max(0)).max(0);
        self.advance(now);
    }

    fn on_ping_rtt(&mut self, now: Time, rtt: Duration) {
        if rtt < Duration::ZERO {
            return;
        }
        let rtt = rtt.max(RTT_GRANULARITY);
        self.last_now = now;
        self.ping_min.add(now, rtt.as_nanos());
        self.ping_max.add(now, rtt.as_nanos());
        smooth(
            &mut self.ping_srtt,
            &mut self.ping_var,
            &mut self.have_ping,
            rtt,
        );
        if self.min_rtt <= Duration::ZERO || (self.min_rtt_from_ping && rtt < self.min_rtt) {
            self.min_rtt = rtt;
            self.min_rtt_stamp = now;
            self.min_rtt_from_ping = true;
        }
        self.update_cap(now);
    }

    fn on_frame_end(&mut self, now: Time) {
        self.advance(now);
    }

    fn in_flight_cap(&self) -> i64 {
        self.cap
    }

    fn in_flight(&self) -> i64 {
        self.flight
    }

    fn request_timeout(&self) -> Duration {
        let base = self.base_timeout();
        let backed_off = base * (1i64 << self.backoff);
        backed_off.min(TIMEOUT_MAX.max(base))
    }

    fn answer_delay(&self) -> Duration {
        if self.have_rtt {
            self.srtt.min(self.request_timeout())
        } else {
            self.request_timeout()
        }
    }

    fn rtt(&self) -> Duration {
        if self.have_rtt {
            self.srtt
        } else {
            self.ping_srtt
        }
    }

    fn min_rtt(&self) -> Duration {
        self.base_rtt()
    }

    fn delivery_rate(&self) -> u64 {
        self.max_rate(self.last_now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHUNK: i64 = 1024;

    fn t(ms: i64) -> Time {
        Time::from_unix_millis(1_700_000_000_000 + ms)
    }

    /// A peer that answers every request `rtt_ms` after it was sent and
    /// delivers at most `rate` bytes per second, driven in 10 ms steps
    /// from `from` to `to` ms. Requests up to the cap every step. Returns
    /// the bytes delivered.
    fn drive(b: &mut BbrLike, from: i64, to: i64, rtt_ms: i64, rate: i64) -> i64 {
        let mut pending: std::collections::VecDeque<(i64, i64)> = Default::default();
        let mut delivered = 0;
        let mut budget = 0i64;
        let mut ms = from;
        while ms < to {
            let now = t(ms);
            budget += rate / 100;
            while let Some(&(sent, bytes)) = pending.front() {
                if sent + rtt_ms > ms || budget < bytes {
                    break;
                }
                pending.pop_front();
                budget -= bytes;
                b.on_delivered(now, bytes);
                b.on_answered(now, bytes, t(sent), t(sent + 1_000_000));
                delivered += bytes;
            }
            budget = budget.min((rate / 10).max(CHUNK));
            b.on_frame_end(now);
            b.on_round(now, 0);
            while b.in_flight() + CHUNK <= b.in_flight_cap() {
                b.on_sent(now, CHUNK);
                pending.push_back((ms, CHUNK));
            }
            ms += 10;
        }
        // What is still pending leaves the flight (as if answered
        // elsewhere), so the next drive starts clean.
        let left: i64 = pending.iter().map(|&(_, b)| b).sum();
        b.on_settled(t(to), left);
        delivered
    }

    #[test]
    fn starts_at_the_initial_cap() {
        let b = BbrLike::new(1024, 0, t(0));
        assert_eq!(b.mode(), Mode::Startup);
        assert_eq!(b.in_flight_cap(), INITIAL_CHUNKS * CHUNK);
        assert_eq!(b.request_timeout(), TIMEOUT_INITIAL);
        assert_eq!(b.in_flight(), 0);
    }

    #[test]
    fn finds_the_rate_and_leaves_startup() {
        // A 40 ms path that delivers 256 KiB/s: startup ends, and the cap
        // settles near twice the bandwidth-delay product (20 KiB).
        let mut b = BbrLike::new(1024, 0, t(0));
        let delivered = drive(&mut b, 0, 10_000, 40, 256 * 1024);
        assert_eq!(b.mode(), Mode::ProbeBw);
        assert!(b.full_pipe);
        let bdp = 256 * 1024 * 40 / 1000;
        let cap = b.in_flight_cap();
        assert!(cap >= bdp && cap <= 4 * bdp, "cap {cap} bdp {bdp}");
        assert!(b.min_rtt() >= Duration::from_millis(40));
        assert!(b.min_rtt() <= Duration::from_millis(60), "{}", b.min_rtt());
        // It used the path: at least 80 % of its rate.
        assert!(delivered > 256 * 1024 * 10 * 8 / 10, "{delivered}");
    }

    #[test]
    fn cap_has_a_floor() {
        // A slow peer: 8 KiB/s at 10 ms is a bandwidth-delay product of
        // 80 bytes; the cap stays at FLOOR_CHUNKS.
        let mut b = BbrLike::new(1024, 0, t(0));
        drive(&mut b, 0, 10_000, 10, 8 * 1024);
        assert_ne!(b.mode(), Mode::Startup);
        assert_eq!(b.in_flight_cap(), FLOOR_CHUNKS * CHUNK);
    }

    #[test]
    fn pushes_count_toward_the_rate() {
        // One request every 100 ms, answered after 50 ms, while the peer
        // pushes 4 chunks every 10 ms: each answer's sample counts the
        // pushes delivered while it was outstanding: 25 chunks in 50 ms.
        let mut b = BbrLike::new(1024, 0, t(0));
        for round in 0..30 {
            let sent = round * 100;
            b.on_sent(t(sent), CHUNK);
            for ms in (sent..sent + 100).step_by(10) {
                b.on_delivered(t(ms), 4 * CHUNK);
                if ms == sent + 50 {
                    b.on_delivered(t(ms), CHUNK);
                    b.on_answered(t(ms), CHUNK, t(sent), t(0));
                }
                b.on_frame_end(t(ms));
            }
        }
        b.on_round(t(3_000), 0);
        let rate = b.max_rate(t(3_000));
        assert_eq!(rate, 500 * 1024);
        // Out of startup: twice 500 KiB/s times 50 ms, times the phase's
        // gain.
        assert_ne!(b.mode(), Mode::Startup);
        let cap = b.in_flight_cap();
        assert!((37 * 1024..=63 * 1024).contains(&cap), "{cap}");
    }

    #[test]
    fn bursts_read_at_the_rate_they_were_asked_for() {
        // A peer that answers 64 requests, sent at once, with one 64 KiB
        // burst a second later delivered 64 KiB/s, not a burst rate.
        let mut b = BbrLike::new(1024, 0, t(0));
        for i in 0..5 {
            let sent = i * 1_000;
            b.on_sent(t(sent), 64 * CHUNK);
            let at = t(sent + 1_000);
            b.on_delivered(at, 64 * CHUNK);
            for _ in 0..64 {
                b.on_answered(at, CHUNK, t(sent), t(0));
            }
            b.on_frame_end(at);
        }
        let rate = b.max_rate(t(5_000));
        assert_eq!(rate, 64 * 1024);
    }

    #[test]
    fn min_rtt_rises_only_after_its_window() {
        let mut b = BbrLike::new(1024, 0, t(0));
        drive(&mut b, 0, 5_000, 20, 64 * 1024);
        assert!(b.min_rtt() < Duration::from_millis(40));
        // The path's delay steps to 300 ms: the minimum (set by the first
        // answer, at about 20 ms; equal samples don't renew it) holds for
        // its window, then the first sample after it replaces it.
        drive(&mut b, 5_000, 9_900, 300, 64 * 1024);
        assert!(b.min_rtt() < Duration::from_millis(40), "{}", b.min_rtt());
        drive(&mut b, 9_900, 16_000, 300, 64 * 1024);
        assert!(b.min_rtt() >= Duration::from_millis(300), "{}", b.min_rtt());
        assert!(b.min_rtt() < Duration::from_millis(400), "{}", b.min_rtt());
    }

    #[test]
    fn steady_pings_raise_the_minimum_round_trip() {
        // A path whose delay steps from 20 ms to 300 ms: once pings agree
        // on 300 ms, the model uses it at once, well before the 20 ms
        // request-to-data minimum expires.
        let mut b = BbrLike::new(1024, 0, t(0));
        drive(&mut b, 0, 3_000, 20, 64 * 1024);
        assert!(b.min_rtt() < Duration::from_millis(40));
        for i in 0..4 {
            b.on_ping_rtt(t(3_000 + i * 1_000), Duration::from_millis(300 + i));
        }
        b.on_round(t(6_000), 0);
        let m = b.min_rtt();
        assert!((300..=303).contains(&m.as_millis()), "{m}");
        assert!(b.request_timeout() >= Duration::from_millis(400));
    }

    #[test]
    fn scattered_pings_leave_the_minimum_round_trip() {
        // Pongs that wait for a busy sender's frames vary widely: queueing,
        // not path delay.
        let mut b = BbrLike::new(1024, 0, t(0));
        drive(&mut b, 0, 3_000, 20, 64 * 1024);
        let min = b.min_rtt();
        for (i, ms) in [300, 900, 150, 600].into_iter().enumerate() {
            b.on_ping_rtt(t(3_000 + i as i64 * 700), Duration::from_millis(ms));
        }
        b.on_round(t(6_000), 0);
        assert_eq!(b.min_rtt(), min);
    }

    #[test]
    fn timeout_follows_the_round_trip_and_backs_off() {
        let mut b = BbrLike::new(1024, 0, t(0));
        for i in 0..50 {
            b.on_sent(t(i * 10), CHUNK);
            b.on_answered(t(i * 10 + 200), CHUNK, t(i * 10), t(0));
        }
        // srtt 200 ms, no spread: the minimum's margin.
        let base = b.request_timeout();
        assert!(base >= Duration::from_millis(300), "{base}");
        assert!(base <= Duration::from_millis(400), "{base}");
        b.on_timed_out(t(1_000), CHUNK);
        assert_eq!(b.request_timeout(), base * 2);
        for i in 0..10 {
            b.on_timed_out(t(1_100 + i * 100), CHUNK);
        }
        assert_eq!(b.request_timeout(), base * 8);
        // An answer resets the backoff.
        b.on_sent(t(3_000), CHUNK);
        b.on_answered(t(3_200), CHUNK, t(3_000), t(0));
        assert!(b.request_timeout() < base * 2);
        // A ping round trip above the data's lifts the timeout.
        for _ in 0..20 {
            b.on_ping_rtt(t(4_000), Duration::from_millis(900));
        }
        assert!(b.request_timeout() >= Duration::from_millis(1_000));
        // Never past TIMEOUT_MAX by backing off.
        for i in 0..10 {
            b.on_timed_out(t(5_000 + i * 100), CHUNK);
        }
        assert!(b.request_timeout() <= TIMEOUT_MAX);
    }

    #[test]
    fn clock_skew_changes_nothing() {
        // The peer's timestamps are never read: two controllers fed the
        // same local events with wildly different peer times agree.
        let mut a = BbrLike::new(1024, 3, t(0));
        let mut b = BbrLike::new(1024, 3, t(0));
        for i in 0..500 {
            let now = t(i * 10);
            for (c, skew) in [(&mut a, 0), (&mut b, i64::MAX / 2)] {
                c.on_sent(now, CHUNK);
                c.on_delivered(now + Duration::from_millis(30), CHUNK);
                c.on_answered(
                    now + Duration::from_millis(30),
                    CHUNK,
                    now,
                    Time::from_unix_nanos(skew.wrapping_mul(i)),
                );
                c.on_frame_end(now + Duration::from_millis(30));
            }
            assert_eq!(a.in_flight_cap(), b.in_flight_cap());
            assert_eq!(a.request_timeout(), b.request_timeout());
        }
    }

    #[test]
    fn flight_accounting() {
        let mut b = BbrLike::new(1024, 0, t(0));
        b.on_sent(t(0), 4 * CHUNK);
        assert_eq!(b.in_flight(), 4 * CHUNK);
        b.on_answered(t(10), CHUNK, t(0), t(0));
        b.on_settled(t(20), CHUNK);
        b.on_timed_out(t(30), CHUNK);
        assert_eq!(b.in_flight(), CHUNK);
        b.on_timed_out(t(40), 10 * CHUNK);
        assert_eq!(b.in_flight(), 0);
        // Negative and absurd inputs saturate.
        b.on_sent(t(50), i64::MAX);
        b.on_sent(t(50), i64::MAX);
        assert_eq!(b.in_flight(), i64::MAX);
        b.on_answered(t(60), -5, t(70), t(0));
        assert_eq!(b.in_flight(), i64::MAX);
    }

    #[test]
    fn a_silent_peer_falls_to_the_floor() {
        // A peer that delivered and then went silent: the rate ages out of
        // the window and the cap falls to the floor.
        let mut b = BbrLike::new(1024, 0, t(0));
        drive(&mut b, 0, 5_000, 20, 512 * 1024);
        assert!(b.in_flight_cap() > FLOOR_CHUNKS * CHUNK);
        for ms in (5_000..9_000).step_by(100) {
            b.on_round(t(ms), 0);
        }
        assert_eq!(b.in_flight_cap(), FLOOR_CHUNKS * CHUNK);
    }
}
