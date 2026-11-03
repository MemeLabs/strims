// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! [`ChannelPacer`]: how much a data channel's writer lets its buffered
//! amount hold; [`HostPacer`]: how much a host's channels hold together.
//!
//! A channel's buffered amount counts the bytes it hasn't sent and those
//! sent and not yet acknowledged, and a channel sends in FIFO order. A
//! writer that keeps it at a fixed cap (`WebRtcConfig::max_buffered_amount`)
//! leaves a queue of up to the cap less the flight below vnic, where vnic's
//! QoS can't serve fresh data first and the runner can't see it waiting. A
//! TCP link keeps that queue small with `TCP_NOTSENT_LOWAT`; a channel has
//! no such mark, and a browser reads nothing of its flight. The pacer
//! estimates the flight as BBR does, from the path's bandwidth and round
//! trip, so that the queue past it stays about [`SLACK`]:
//!
//! - the bandwidth: the most the channel delivered (the bytes written less
//!   the buffered amount) over a round trip (20 ms at least), in the last
//!   [`BW_SPAN`], so a stall (a loss's recovery) doesn't shrink it;
//! - the round trip: the least of the last [`RTT_SPAN`], from each write to
//!   the reading that shows it delivered, so it holds the path's and as
//!   little of the channel's own queue as the span saw;
//! - the cap: the bandwidth over twice the round trip and 50 ms (room for
//!   the channel's window to grow and for the queues of the path), plus
//!   `SLACK`; [`FLOOR`] at least, and before the first round trip.
//!
//! The writer reports each write ([`ChannelPacer::wrote`]) and reads the
//! buffered amount as it writes and as acknowledgements free it
//! ([`ChannelPacer::observe`]); while it waits, it waits for the amount to
//! fall to [`ChannelPacer::low_mark`] (a browser's
//! `bufferedAmountLowThreshold`). Times are the node's clock, so a
//! simulation replays.
//!
//! # The host's channels together
//!
//! A host's channels share its uplink, but each pacer reads its channel's
//! delivery rate as if it had the uplink alone. Channels that write at
//! once (a wave of answers to neighbours that missed the same chunks) then
//! hold more than the uplink's queue takes, and a browser sends its
//! datagrams unpaced: the queue drops packets of every channel, whose SCTP
//! halves its window and regains a packet a round trip, as Reno does. The
//! host pacer reads the host's summed buffered amounts as a channel pacer
//! reads one channel's, for the bandwidth the host's channels deliver
//! together, and bounds their *load* ([`HostSum`]):
//!
//! - the load is each channel's buffered amount, as the uplink must carry
//!   it; but a channel that waits on its own cap counts at most its own
//!   estimated flight (its bandwidth over its horizon): its own path holds
//!   it back (a narrow downlink, a slow reader), and the rest waits for
//!   its window, not in the uplink's queue. Counted whole, a backed-up
//!   neighbour would take the host's cap from the rest;
//! - the cap is the host's bandwidth over the channels' horizons, each
//!   weighted by its channel's bandwidth, plus [`SLACK`] ([`FLOOR`] at
//!   least): the flight the channels hold while they share the uplink as
//!   their rates do. The round trips of the host's summed amounts would be
//!   its nearest neighbour's, which leave a far one no room for its flight.
//!
//! A channel writes while both its own pacer and the host's admit it; one
//! the host holds waits for the load to fall to the host's mark.

use std::collections::VecDeque;

use strims_base::time::{Duration, Time};

/// The bytes past the estimated flight a channel holds.
pub const SLACK: usize = 16 * 1024;

/// The least cap: before a round trip is known, and on an idle channel.
pub const FLOOR: usize = 32 * 1024;

/// How long a delivery rate counts toward the bandwidth.
pub const BW_SPAN: Duration = Duration::from_secs(2);

/// How long a round trip counts toward the least.
pub const RTT_SPAN: Duration = Duration::from_secs(10);

/// The writes the pacer times at once; more merge into the latest.
const WRITES: usize = 64;

/// The room a waiting writer needs before it writes again.
const MIN_WRITE: usize = 8 * 1024;

/// See the [module docs](self).
#[derive(Clone, Debug, Default)]
pub struct ChannelPacer {
    written: u64,
    /// Writes not yet delivered: when each went, and the bytes written
    /// through it. Bound: [`WRITES`].
    writes: VecDeque<(Time, u64)>,
    /// Readings of the bytes delivered, over the last round trip and one
    /// before it. Bound: a round trip's readings.
    acked: VecDeque<(Time, u64)>,
    /// Round trips of the last [`RTT_SPAN`], each under every later one (a
    /// sliding least). Bound: a span's writes.
    rtts: VecDeque<(Time, Duration)>,
    /// Delivery rates of the last [`BW_SPAN`], each over every later one (a
    /// sliding most), in bytes a second. Bound: a span's readings.
    rates: VecDeque<(Time, f64)>,
}

impl ChannelPacer {
    /// The writer handed the channel `n` bytes at `now`.
    pub fn wrote(&mut self, now: Time, n: usize) {
        self.written = self.written.saturating_add(n as u64);
        let full = self.writes.len() >= WRITES;
        match self.writes.back_mut() {
            Some(last) if last.0 == now || full => last.1 = self.written,
            _ => self.writes.push_back((now, self.written)),
        }
    }

    /// The channel's buffered amount read at `now`.
    pub fn observe(&mut self, now: Time, buffered: usize) {
        let acked = self.written.saturating_sub(buffered as u64);
        while let Some(&(at, through)) = self.writes.front() {
            if through > acked {
                break;
            }
            self.writes.pop_front();
            let rtt = now - at;
            while self.rtts.back().is_some_and(|&(_, r)| r >= rtt) {
                self.rtts.pop_back();
            }
            self.rtts.push_back((now, rtt));
        }
        while self.rtts.len() > 1 && self.rtts.front().is_some_and(|&(t, _)| t + RTT_SPAN < now) {
            self.rtts.pop_front();
        }
        if self.acked.back().is_none_or(|&(_, a)| a < acked) {
            self.acked.push_back((now, acked));
        }
        // Keep one reading at or before the round trip's start.
        let win = self.window();
        while self.acked.get(1).is_some_and(|&(t, _)| t + win <= now) {
            self.acked.pop_front();
        }
        if let Some(&(t0, a0)) = self.acked.front()
            && t0 + win <= now
        {
            let rate = (acked - a0) as f64 / (now - t0).as_secs_f64();
            while self.rates.back().is_some_and(|&(_, r)| r <= rate) {
                self.rates.pop_back();
            }
            self.rates.push_back((now, rate));
        }
        while self.rates.front().is_some_and(|&(t, _)| t + BW_SPAN < now) {
            self.rates.pop_front();
        }
    }

    /// Whether the writer may write at `now` with `buffered` bytes
    /// buffered, `max` at most.
    pub fn may_write(&self, now: Time, buffered: usize, max: usize) -> bool {
        buffered < self.cap(now, max)
    }

    /// The buffered amount a waiting writer waits for: a browser's
    /// `bufferedAmountLowThreshold` while it waits.
    pub fn low_mark(&self, now: Time, max: usize) -> usize {
        self.cap(now, max).saturating_sub(MIN_WRITE)
    }

    /// The most the channel's buffered amount may hold at `now`, `max` at
    /// most.
    pub fn cap(&self, _now: Time, max: usize) -> usize {
        let Some(rtt) = self.rtt() else {
            return FLOOR.min(max);
        };
        let horizon = rtt * 2 + Duration::from_millis(50);
        let flight = (self.bandwidth() * horizon.as_secs_f64()) as usize;
        flight.saturating_add(SLACK).max(FLOOR).min(max)
    }

    /// The most the channel delivered over a round trip in the span, in
    /// bytes a second.
    fn bandwidth(&self) -> f64 {
        self.rates.front().map_or(0.0, |&(_, r)| r)
    }

    /// The cap's horizon: twice the round trip and 50 ms.
    fn horizon(&self) -> Option<Duration> {
        self.rtt().map(|rtt| rtt * 2 + Duration::from_millis(50))
    }

    /// The flight the cap allows for: the bandwidth over the horizon.
    fn flight(&self) -> Option<f64> {
        self.horizon()
            .map(|h| self.bandwidth() * h.as_secs_f64())
            .filter(|&f| f > 0.0)
    }

    /// The least round trip of the span.
    fn rtt(&self) -> Option<Duration> {
        self.rtts.front().map(|&(_, r)| r)
    }

    /// The span a delivery rate is read over: the round trip, 20 ms at
    /// least.
    fn window(&self) -> Duration {
        self.rtt()
            .unwrap_or(Duration::from_millis(100))
            .max(Duration::from_millis(20))
    }
}

/// See the [module docs](self#the-hosts-channels-together).
#[derive(Clone, Debug, Default)]
pub struct HostPacer {
    pacer: ChannelPacer,
}

/// A host's channels, summed at one reading ([`HostSum::add`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct HostSum {
    /// Their buffered amounts ...
    pub buffered: usize,
    /// ... as the load counts them.
    pub load: usize,
    /// Their bandwidths, and those times their horizons in seconds.
    bandwidth: f64,
    flight: f64,
}

impl HostSum {
    /// Adds a channel with `buffered` bytes buffered that `waits` (or not)
    /// on its own cap.
    pub fn add(&mut self, channel: &ChannelPacer, buffered: usize, waits: bool) {
        self.buffered += buffered;
        let flight = channel.flight();
        self.load += match flight {
            Some(f) if waits => buffered.min(f as usize),
            _ => buffered,
        };
        if let Some(f) = flight {
            self.bandwidth += channel.bandwidth();
            self.flight += f;
        }
    }

    /// A channel was handed `n` more bytes.
    pub fn wrote(&mut self, n: usize) {
        self.buffered += n;
        self.load += n;
    }
}

impl HostPacer {
    /// A channel of the host was handed `n` bytes at `now`.
    pub fn wrote(&mut self, now: Time, n: usize) {
        self.pacer.wrote(now, n);
    }

    /// The host's channels read at `now`.
    pub fn observe(&mut self, now: Time, sum: &HostSum) {
        self.pacer.observe(now, sum.buffered);
    }

    /// Whether a channel may write at `now` with the channels at `sum`.
    pub fn may_write(&self, now: Time, sum: &HostSum) -> bool {
        sum.load < self.cap(now, sum)
    }

    /// The load a channel the host holds waits for.
    pub fn low_mark(&self, now: Time, sum: &HostSum) -> usize {
        self.cap(now, sum).saturating_sub(MIN_WRITE)
    }

    /// The most the channels' load may be at `now`.
    fn cap(&self, now: Time, sum: &HostSum) -> usize {
        if sum.bandwidth <= 0.0 || self.pacer.rtt().is_none() {
            return self.pacer.cap(now, usize::MAX);
        }
        let horizon = sum.flight / sum.bandwidth;
        ((self.pacer.bandwidth() * horizon) as usize)
            .saturating_add(SLACK)
            .max(FLOOR)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn t(ms: i64) -> Time {
        Time::from_unix_millis(1_700_000_000_000 + ms)
    }

    /// 10 KB written every 10 ms, each delivered 40 ms later, from `from`
    /// for `n` writes.
    fn steady(p: &mut ChannelPacer, from: i64, n: i64) {
        for k in 0..n {
            let at = from + 10 * k;
            p.wrote(t(at), 10_000);
            let delivered = p.written.saturating_sub(40_000);
            p.observe(t(at), usize::try_from(p.written - delivered).unwrap());
        }
    }

    /// Before a round trip is known the cap is the floor; a channel that
    /// delivers 1 MB/s over a 40 ms round trip holds that rate over 130 ms
    /// and the slack.
    #[test]
    fn the_cap_follows_the_bandwidth_over_the_round_trip() {
        let mut p = ChannelPacer::default();
        assert_eq!(p.cap(t(0), 1 << 20), FLOOR);
        steady(&mut p, 0, 100);
        assert_eq!(p.rtt(), Some(Duration::from_millis(40)));
        let cap = p.cap(t(990), 1 << 20);
        let want = 130_000 + SLACK;
        assert!(cap.abs_diff(want) < 12_000, "{cap} vs {want}");
        assert_eq!(p.cap(t(990), 64 * 1024), 64 * 1024, "never past max");
        assert!(p.may_write(t(990), cap - 1, 1 << 20));
        assert!(!p.may_write(t(990), cap, 1 << 20));
        assert_eq!(p.low_mark(t(990), 1 << 20), cap - MIN_WRITE);
    }

    /// A channel with 100 KB/s delivered over a 400 ms round trip.
    fn slow() -> ChannelPacer {
        let mut p = ChannelPacer::default();
        for k in 0..100 {
            p.wrote(t(10 * k), 1_000);
            let delivered = p.written.saturating_sub(40_000);
            p.observe(t(10 * k), usize::try_from(p.written - delivered).unwrap());
        }
        p
    }

    /// A channel waiting on its own cap counts at most its own flight; the
    /// host's cap is its bandwidth over the channels' horizons, weighted by
    /// their bandwidths.
    #[test]
    fn the_host_weighs_its_channels_by_their_flights() {
        let (mut fast, slow) = (ChannelPacer::default(), slow());
        steady(&mut fast, 0, 100);
        // 100 KB/s over 850 ms.
        let mut sum = HostSum::default();
        sum.add(&slow, 40_000, false);
        assert_eq!(sum.load, 40_000);
        let mut held = HostSum::default();
        held.add(&slow, 120_000, true);
        assert!(held.load.abs_diff(85_000) < 5_000, "{held:?}");
        let before = held.load;
        held.add(&ChannelPacer::default(), 30_000, true);
        assert_eq!(held.buffered, 150_000);
        assert_eq!(held.load, before + 30_000, "no flight yet: whole");

        // The host delivers 1 MB/s over a 40 ms round trip; its channels'
        // horizons: 130 ms at 1 MB/s and 850 ms at 100 KB/s.
        let mut host = HostPacer::default();
        steady(&mut host.pacer, 0, 100);
        let mut both = HostSum::default();
        both.add(&fast, 0, false);
        both.add(&slow, 0, false);
        let horizon = (1e6 * 0.13 + 1e5 * 0.85) / 1.1e6;
        let want = (1e6 * horizon) as usize + SLACK;
        let cap = host.cap(t(990), &both);
        assert!(cap.abs_diff(want) < 20_000, "{cap} vs {want}");
        assert!(host.may_write(
            t(990),
            &HostSum {
                load: cap - 1,
                ..both
            }
        ));
        assert!(!host.may_write(t(990), &HostSum { load: cap, ..both }));
        assert_eq!(host.low_mark(t(990), &both), cap - MIN_WRITE);
        assert_eq!(
            host.cap(t(990), &HostSum::default()),
            host.pacer.cap(t(990), usize::MAX)
        );
    }

    /// A stall (nothing delivered for a second) keeps the cap the
    /// bandwidth gave it; after the span it falls to the floor.
    #[test]
    fn a_stall_keeps_the_cap_for_the_span() {
        let mut p = ChannelPacer::default();
        steady(&mut p, 0, 100);
        let before = p.cap(t(990), 1 << 20);
        let buffered = usize::try_from(40_000u64).unwrap();
        p.observe(t(1_990), buffered);
        assert_eq!(p.cap(t(1_990), 1 << 20), before);
        p.observe(t(4_000), buffered);
        assert_eq!(p.cap(t(4_000), 1 << 20), FLOOR);
    }
}
