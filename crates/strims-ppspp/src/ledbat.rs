// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The LEDBAT congestion controller (RFC 6817) that sizes a channel's
//! request window, with an RFC 6298 style congestion timeout from ping
//! round trips and their minimum over the last [`MIN_RTT_WINDOW`].
//!
//! Delays are signed [`Duration`]s: delay samples are one-way delays
//! computed from the sender's data timestamps, so clock skew makes them
//! negative, and LEDBAT only uses differences between them. Peers control
//! those timestamps, so the window arithmetic saturates.
//!
//! The base delay is the minimum over four one-minute buckets, so a rise in
//! the path's base delay reads as queueing until the old minimum ages out.
//! The owner can set a floor on the window ([`Ledbat::set_min_cwnd`]) that
//! bounds that delay-driven decrease but not the response to losses.

use strims_base::stats::Ema;
use strims_base::time::{Duration, Time};

use crate::windowed::WindowedMin;

/// RFC 6817 `TARGET`: the queuing delay LEDBAT aims for, in nanoseconds.
const TARGET: i64 = 100_000_000;
/// RFC 6817 `GAIN`.
const GAIN: i64 = 1;
/// Minutes of base delay history.
const BASE_HISTORY: usize = 4;
/// Samples in the current delay filter.
const CURRENT_DELAYS: usize = 10;
/// The initial window in segments: a new channel requests at a useful
/// rate before its first delay sample.
const INIT_CWND: i64 = 32;
const MIN_CWND: i64 = 2;
/// The segment size the window is counted in.
pub const MSS: i64 = 1024;
/// How long after a loss the owner's window floor
/// ([`Ledbat::set_min_cwnd`]) doesn't hold.
pub const FLOOR_LOSS_HOLD: Duration = Duration::from_secs(2);

/// How long a ping round trip counts toward [`Ledbat::rtt_min`]. Short:
/// a floor on the minimum follows a real rise in the path's delay only
/// once the older pings have left the window, and the stream can't wait
/// long (strims-sim's `latency_step` wants it flowing 10 s after a step).
/// Long enough to hold a few pings (they come about once a second), one
/// of which likely found the peer's pipe drained.
pub const MIN_RTT_WINDOW: Duration = Duration::from_secs(4);

/// RFC 6298 `G`: the clock granularity floor of the variance term, 100 ms.
const COEF_G: f64 = 100_000_000.0;
/// RFC 6298 `K`.
const COEF_K: f64 = 4.0;
/// Jacobson's `alpha` and `beta` for the RTT mean and variance.
const COEF_ALPHA: f64 = 0.125;
const COEF_BETA: f64 = 0.25;

const MAX_DELAY_SAMPLE: Duration = Duration::MAX;

/// The minimum of `durations`.
fn filter(durations: &[Duration]) -> Duration {
    durations
        .iter()
        .copied()
        .fold(MAX_DELAY_SAMPLE, Duration::min)
}

/// A LEDBAT congestion controller.
#[derive(Clone, Debug)]
pub struct Ledbat {
    base_delays: [Duration; BASE_HISTORY],
    current_delays: [Duration; CURRENT_DELAYS],
    base_delay_index: usize,
    current_delay_index: usize,
    last_rollover: Time,

    flight_size: i64,
    cwnd: i64,

    /// The congestion timeout.
    cto: Duration,

    last_data_loss: Time,
    last_ack_time: Time,
    rtt_mean: Ema,
    rtt_var: Ema,
    /// Ping round trips over [`MIN_RTT_WINDOW`], nanoseconds, and the
    /// latest one's time (`Time::NIL` before the first).
    rtt_min: WindowedMin,
    last_rtt_sample: Time,

    ack_size: i64,

    /// A floor on the window, at least `MIN_CWND` segments, set by the
    /// owner (see [`Ledbat::set_min_cwnd`]).
    min_cwnd: i64,
    /// The floor doesn't hold until then: a loss was seen.
    floor_held_until: Time,
    /// The owner sizes the floor on [`Ledbat::rtt_min`], not
    /// [`Ledbat::rtt_mean`] ([`Ledbat::floor_rtt`]).
    floor_on_min: bool,
}

impl Default for Ledbat {
    fn default() -> Ledbat {
        Ledbat::new()
    }
}

impl Ledbat {
    /// A controller at the initial window with a 1 s congestion timeout.
    pub fn new() -> Ledbat {
        Ledbat {
            base_delays: [MAX_DELAY_SAMPLE; BASE_HISTORY],
            current_delays: [MAX_DELAY_SAMPLE; CURRENT_DELAYS],
            base_delay_index: 0,
            current_delay_index: 0,
            last_rollover: Time::EPOCH,
            flight_size: 0,
            cwnd: INIT_CWND * MSS,
            cto: Duration::SECOND,
            last_data_loss: Time::NIL,
            last_ack_time: Time::MAX,
            rtt_mean: Ema::new(COEF_ALPHA),
            rtt_var: Ema::new(COEF_BETA),
            rtt_min: WindowedMin::new(MIN_RTT_WINDOW),
            last_rtt_sample: Time::NIL,
            ack_size: 0,
            min_cwnd: MIN_CWND * MSS,
            floor_held_until: Time::EPOCH,
            floor_on_min: false,
        }
    }

    /// Sets a floor of `bytes` on the window (at least `MIN_CWND`
    /// segments) and raises the window to it.
    ///
    /// RFC 6817's minimum of 2 segments lets a request window carry 2 KiB
    /// per round trip, and LEDBAT holds the window there for as
    /// long as it reads a rise in the path's base delay as queueing: until
    /// the old base ages out of its history, minutes. The scheduler sets a
    /// floor that still carries the stream. The floor only overrides the
    /// delay signal: for [`FLOOR_LOSS_HOLD`] after a loss (a timed out
    /// request, or a congestion timeout without acks) the window follows
    /// LEDBAT alone, down to the RFC minimum. Data timestamps are stamped
    /// after the sender's queue, so a congested sender shows up only as
    /// timeouts, never as delay.
    pub fn set_min_cwnd(&mut self, bytes: i64, now: Time) {
        self.min_cwnd = bytes.max(MIN_CWND * MSS);
        self.cwnd = self.cwnd.max(self.floor(now));
    }

    /// Sizes the owner's floor on the minimum round trip
    /// ([`Ledbat::floor_rtt`]).
    pub fn set_floor_on_min(&mut self, on: bool) {
        self.floor_on_min = on;
    }

    /// Whether the owner's floor is sized on the minimum round trip.
    pub fn floor_on_min(&self) -> bool {
        self.floor_on_min
    }

    /// The round trip the owner sizes its floor on:
    /// [`Ledbat::rtt_min`] after [`Ledbat::set_floor_on_min`], otherwise
    /// [`Ledbat::rtt_mean`].
    pub fn floor_rtt(&self) -> Duration {
        if self.floor_on_min {
            self.rtt_min()
        } else {
            self.rtt_mean()
        }
    }

    /// The floor set by [`Ledbat::set_min_cwnd`], in bytes.
    pub fn min_cwnd(&self) -> i64 {
        self.min_cwnd
    }

    /// The smallest window LEDBAT may set now.
    fn floor(&self, now: Time) -> i64 {
        if now < self.floor_held_until {
            MIN_CWND * MSS
        } else {
            self.min_cwnd
        }
    }

    /// Shrinks the window by `size` bytes (not below the minimum), for a
    /// competing flow the owner accounts for.
    pub fn hack_test(&mut self, size: i64) {
        self.cwnd = self.cwnd.saturating_sub(size).max(MIN_CWND * MSS);
    }

    /// The congestion window in bytes.
    pub fn cwnd(&self) -> i64 {
        self.cwnd
    }

    /// The congestion timeout.
    pub fn cto(&self) -> Duration {
        self.cto
    }

    /// Bytes requested and not yet acknowledged or lost.
    pub fn flight_size(&self) -> i64 {
        self.flight_size
    }

    /// The smoothed round trip from ping samples.
    pub fn rtt_mean(&self) -> Duration {
        Duration::from_nanos(self.rtt_mean.value() as i64)
    }

    /// The smallest ping round trip over the [`MIN_RTT_WINDOW`] up to the
    /// latest one, or zero before the first. Measured up to the latest
    /// sample, not to now, so the minimum holds while pings go
    /// unanswered.
    pub fn rtt_min(&self) -> Duration {
        if self.last_rtt_sample.is_nil() {
            return Duration::ZERO;
        }
        self.rtt_min
            .get(self.last_rtt_sample)
            .map_or(Duration::ZERO, Duration::from_nanos)
    }

    /// The round trip's mean deviation.
    pub fn rtt_var(&self) -> Duration {
        Duration::from_nanos(self.rtt_var.value() as i64)
    }

    /// Puts `size` requested bytes in flight.
    pub fn add_sent(&mut self, size: i64) {
        self.flight_size = self.flight_size.saturating_add(size);
    }

    /// Takes `size` bytes out of flight without a loss signal: requests
    /// the peer never answered whose data arrived from elsewhere. Left in
    /// flight they would never leave it, and at the 2-segment minimum two
    /// of them stop the channel from requesting.
    pub fn forget_sent(&mut self, size: i64) {
        self.flight_size = self.flight_size.saturating_sub(size).max(0);
    }

    /// Records the one-way delay of `size` acknowledged bytes.
    pub fn add_delay_sample(&mut self, d: Duration, size: i64, now: Time) {
        self.update_current_delay(d);
        self.update_base_delay(d, now);
        self.ack_size = self.ack_size.saturating_add(size);
        self.last_ack_time = now;
    }

    fn update_current_delay(&mut self, d: Duration) {
        self.current_delay_index = (self.current_delay_index + 1) % CURRENT_DELAYS;
        if let Some(slot) = self.current_delays.get_mut(self.current_delay_index) {
            *slot = d;
        }
    }

    fn update_base_delay(&mut self, d: Duration, now: Time) {
        let now = now.truncate(Duration::MINUTE);
        if now != self.last_rollover {
            self.last_rollover = now;
            self.base_delay_index = (self.base_delay_index + 1) % BASE_HISTORY;
            if let Some(slot) = self.base_delays.get_mut(self.base_delay_index) {
                *slot = d;
            }
            return;
        }
        if let Some(slot) = self.base_delays.get_mut(self.base_delay_index)
            && d < *slot
        {
            *slot = d;
        }
    }

    /// Applies the samples added since the last call: the window grows
    /// while the queuing delay is under the 100 ms target and shrinks above
    /// it.
    /// With bytes in flight and no ack for a congestion timeout, the window
    /// collapses and the timeout doubles (up to 1 s).
    pub fn digest_delay_samples(&mut self, now: Time) {
        if self.flight_size > 0 && now - self.last_ack_time > self.cto {
            self.floor_held_until = now + FLOOR_LOSS_HOLD;
            self.cwnd = MIN_CWND * MSS;
            self.cto = (self.cto * 2).min(Duration::SECOND);
        }

        if self.ack_size == 0 {
            return;
        }

        let queuing_delay =
            i128::from((filter(&self.current_delays) - filter(&self.base_delays)).as_nanos());
        // Peers control the timestamps behind these delays: compute wide
        // and saturate.
        let delta = (i128::from(TARGET) - queuing_delay)
            * i128::from(GAIN)
            * i128::from(self.ack_size)
            * i128::from(MSS)
            / i128::from(self.cwnd.max(1))
            / i128::from(TARGET);
        let delta = i64::try_from(delta).unwrap_or(if delta < 0 { i64::MIN } else { i64::MAX });
        self.cwnd = self.cwnd.saturating_add(delta).max(self.floor(now));

        self.flight_size = self.flight_size.saturating_sub(self.ack_size).max(0);
        self.ack_size = 0;
    }

    /// Updates the RTT estimates and the congestion timeout with a ping
    /// round trip measured at `now`.
    pub fn add_rtt_sample(&mut self, rtt: Duration, now: Time) {
        self.rtt_min.add(now, rtt.as_nanos().max(0));
        self.last_rtt_sample = now;
        let rtt = rtt.as_nanos() as f64;
        if self.rtt_mean.value() == 0.0 {
            self.rtt_mean.set(rtt);
            self.rtt_var.set(rtt / 2.0);
        } else {
            self.rtt_var.update((self.rtt_mean.value() - rtt).abs());
            self.rtt_mean.update(rtt);
        }
        let cto = self.rtt_mean.value() + COEF_G.max(COEF_K * self.rtt_var.value());
        self.cto = Duration::from_nanos(cto as i64);
    }

    /// Records `size` lost bytes; halves the window at most once per mean
    /// RTT.
    pub fn add_data_loss(&mut self, size: i64, retransmitting: bool, now: Time) {
        if !retransmitting {
            self.flight_size = self.flight_size.saturating_sub(size).max(0);
        }

        self.floor_held_until = now + FLOOR_LOSS_HOLD;
        let timeout = Duration::from_nanos(self.rtt_mean.value() as i64);
        if !self.last_data_loss.is_nil() && now - self.last_data_loss < timeout {
            return;
        }
        self.last_data_loss = now;

        let cwnd = (self.cwnd / 2).max(MIN_CWND * MSS);
        if cwnd < self.cwnd {
            self.cwnd = cwnd;
        }
    }
}

#[cfg(test)]
mod tests {
    //! The behaviour the scheduler relies on.

    use super::*;

    fn t(ms: i64) -> Time {
        Time::from_unix_millis(1_700_000_000_000 + ms)
    }

    #[test]
    fn initial_state() {
        let l = Ledbat::new();
        assert_eq!(l.cwnd(), 32 * 1024);
        assert_eq!(l.cto(), Duration::SECOND);
        assert_eq!(l.flight_size(), 0);
    }

    #[test]
    fn grows_below_target_and_shrinks_above() {
        let mut l = Ledbat::new();
        // Constant 20 ms delay: no queuing, the window grows.
        for i in 0..100 {
            l.add_sent(1024);
            l.add_delay_sample(Duration::from_millis(20), 1024, t(i));
            l.digest_delay_samples(t(i));
        }
        let grown = l.cwnd();
        assert!(grown > 32 * 1024, "{grown}");
        assert_eq!(l.flight_size(), 0);

        // Delays rise 300 ms over the base: the window shrinks.
        for i in 100..200 {
            l.add_sent(1024);
            l.add_delay_sample(Duration::from_millis(320), 1024, t(i));
            l.digest_delay_samples(t(i));
        }
        assert!(l.cwnd() < grown, "{} {grown}", l.cwnd());
        assert!(l.cwnd() >= MIN_CWND * MSS);
    }

    #[test]
    fn negative_delays_from_clock_skew() {
        // A peer clock 10 s ahead makes every one-way delay negative; only
        // the differences matter.
        let mut l = Ledbat::new();
        for i in 0..50 {
            l.add_sent(1024);
            l.add_delay_sample(Duration::from_millis(-10_000), 1024, t(i));
            l.digest_delay_samples(t(i));
        }
        assert!(l.cwnd() > 32 * 1024);
    }

    #[test]
    fn extreme_delays_saturate() {
        let mut l = Ledbat::new();
        l.add_sent(1 << 20);
        l.add_delay_sample(Duration::MIN, 1 << 20, t(0));
        l.add_delay_sample(Duration::MAX, 1 << 20, t(0));
        l.digest_delay_samples(t(0));
        assert!(l.cwnd() >= MIN_CWND * MSS);
        l.add_delay_sample(Duration::MAX, i64::MAX, t(1));
        l.digest_delay_samples(t(1));
        assert!(l.cwnd() >= MIN_CWND * MSS);
    }

    #[test]
    fn min_cwnd_floor_holds_against_delay_not_loss() {
        // The owner's floor bounds the delay-based decrease, but
        // a loss or a congestion timeout takes the window below it for
        // FLOOR_LOSS_HOLD.
        let mut l = Ledbat::new();
        l.set_min_cwnd(10 * MSS, t(0));
        assert_eq!(l.min_cwnd(), 10 * MSS);
        assert_eq!(l.cwnd(), 32 * MSS);
        // A base delay of 10 ms, then 150 ms: read as 140 ms of queueing.
        l.add_delay_sample(Duration::from_millis(10), 1024, t(0));
        l.digest_delay_samples(t(0));
        for i in 1..500 {
            l.add_sent(8192);
            l.add_delay_sample(Duration::from_millis(150), 8192, t(i));
            l.digest_delay_samples(t(i));
        }
        assert_eq!(l.cwnd(), 10 * MSS);

        // A loss halves it below the floor, and the floor waits.
        l.add_data_loss(1024, false, t(1_000));
        assert_eq!(l.cwnd(), 5 * MSS);
        l.set_min_cwnd(10 * MSS, t(2_000));
        assert_eq!(l.cwnd(), 5 * MSS);
        l.set_min_cwnd(10 * MSS, t(1_000) + FLOOR_LOSS_HOLD);
        assert_eq!(l.cwnd(), 10 * MSS);

        // So does a congestion timeout.
        l.add_sent(4096);
        l.digest_delay_samples(t(10_000));
        assert_eq!(l.cwnd(), MIN_CWND * MSS);
        l.set_min_cwnd(10 * MSS, t(11_000));
        assert_eq!(l.cwnd(), MIN_CWND * MSS);
        l.set_min_cwnd(40 * MSS, t(10_000) + FLOOR_LOSS_HOLD);
        assert_eq!(l.cwnd(), 40 * MSS);

        // No floor below MIN_CWND segments.
        l.set_min_cwnd(0, t(20_000));
        assert_eq!(l.min_cwnd(), MIN_CWND * MSS);
    }

    #[test]
    fn rtt_sets_cto() {
        let mut l = Ledbat::new();
        l.add_rtt_sample(Duration::from_millis(200), t(0));
        // mean 200 ms + max(100 ms, 4 * 100 ms)
        assert_eq!(l.cto(), Duration::from_millis(600));
        assert_eq!(l.rtt_mean(), Duration::from_millis(200));
    }

    #[test]
    fn rtt_min_over_the_window() {
        let mut l = Ledbat::new();
        assert_eq!(l.rtt_min(), Duration::ZERO);
        // One quick ping among slow ones: the minimum holds it while it is
        // in the window, and the mean doesn't.
        let step = MIN_RTT_WINDOW / 10;
        l.add_rtt_sample(Duration::from_millis(50), t(0));
        for i in 1..10 {
            l.add_rtt_sample(Duration::from_millis(600), t(0) + step * i);
        }
        assert_eq!(l.rtt_min(), Duration::from_millis(50));
        assert!(
            l.rtt_mean() > Duration::from_millis(400),
            "{:?}",
            l.rtt_mean()
        );
        // A rise in the path's delay: the minimum follows once the quick
        // ping leaves the window.
        l.add_rtt_sample(Duration::from_millis(600), t(0) + MIN_RTT_WINDOW);
        assert_eq!(l.rtt_min(), Duration::from_millis(600));
    }

    #[test]
    fn loss_halves_once_per_rtt() {
        let mut l = Ledbat::new();
        l.add_rtt_sample(Duration::from_millis(100), t(0));
        l.add_sent(10 * 1024);
        l.add_data_loss(1024, false, t(0));
        assert_eq!(l.cwnd(), 16 * 1024);
        assert_eq!(l.flight_size(), 9 * 1024);
        l.add_data_loss(1024, false, t(50));
        assert_eq!(l.cwnd(), 16 * 1024);
        l.add_data_loss(1024, false, t(150));
        assert_eq!(l.cwnd(), 8 * 1024);
    }

    #[test]
    fn congestion_timeout_collapses_window() {
        let mut l = Ledbat::new();
        l.add_sent(4096);
        l.add_delay_sample(Duration::from_millis(10), 1024, t(0));
        l.digest_delay_samples(t(0));
        l.add_sent(4096);
        // No ack for more than the 1 s timeout.
        l.digest_delay_samples(t(2_000));
        assert_eq!(l.cwnd(), MIN_CWND * MSS);
        assert_eq!(l.cto(), Duration::SECOND);
    }
}
