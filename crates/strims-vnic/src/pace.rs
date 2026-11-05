// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! [`EgressPacer`]: a host's egress paced at a rate its links' paths
//! bound, for a host whose transports hide what they hold in flight.
//!
//! A browser's data channel counts as buffered only the bytes its SCTP
//! hasn't sent: they leave the amount as they first go out, so a writer
//! that bounds the amount bounds none of the flight, and each association
//! sends what it is handed while its window has room. Handed frames for a
//! dozen neighbours at once, the associations together burst past the
//! uplink's queue (on the testbed a seed offered 159 KB in 10 ms to a
//! 125 KB queue), the queue drops packets of every channel, and each loss
//! holds its ordered channel until SCTP resends it. Such a host
//! ([`HostConfig::pace`](crate::host::HostConfig::pace)) paces its egress in
//! time instead: QoS's rate limit at a rate its links' paths bound
//! ([`strims_base::path`]), with a bucket of [`PACE_BURST`] at that rate.
//!
//! Every [`PACE_UPDATE`], over the links whose latest echo is at most
//! [`LATEST_MAX_AGE`] old, the pacer takes what they deliver together:
//! their delivery rates summed ([`PathReading::rate`]: the bytes echoes
//! acknowledged over the longer of their sends' and their echoes' spans,
//! which bursts and bunched echoes can't raise). The most of those over the
//! last [`BW_SPAN`] is what the uplink carries. (A link's bytes in flight
//! over its round trip read what the host handed it: two to four times the
//! uplink in a browser's bursts.)
//!
//! While a queue stands, the rate is [`HEADROOM`] times what the uplink
//! carries, with a bucket of [`PACE_BURST`] at the rate: it bounds how fast
//! a burst goes out, not what the host sends. Past the bucket the uplink's
//! queue grows at the headroom's margin over the uplink while the burst
//! lasts, not at a dozen associations' windows at once. A rate near what
//! the uplink carries would hold back each unit's chunks behind the ones
//! before them.
//!
//! What the links delivered reads no more than the host had to send, so
//! early on, or on a host whose demand grows, it is far under the uplink.
//! While no queue stands the rate grows by [`GROWTH`] at each update from
//! where it was, so a host whose uplink has room stops being held back
//! within a second of its queue clearing, and one whose uplink is full
//! keeps the rate its queue gives.
//!
//! It paces once the links' [`latest_excess`] has stood past
//! [`QUEUE_TARGET`] at every update for [`QUEUE_HOLD`], until no queue has
//! for [`UNPACE_AFTER`]; otherwise the host goes unpaced ([`UNPACED`]): one
//! whose uplink doesn't fill gains nothing from a rate taken from what it
//! sent, and one whose uplink does shows a queue at its next bursts. A
//! queue that drains within the hold doesn't count: a relay that forwards
//! each unit to several children over an uplink with room queues at every
//! burst, and paced at the headroom over what it delivered on average it
//! would send each burst slower than its uplink, the frames waiting in QoS
//! with its requests and answers. The excess includes the echoes' way back,
//! so a peer's queue can read as the host's: the headroom and the growth
//! make that cheap.
//!
//! The rate holds QoS's limit while it is under the host's upload limit,
//! with its bucket; otherwise the upload limit applies with QoS's own.
//!
//! The rate is [`MIN_RATE`] at least; without a fresh reading it holds.
//! Probes and echoes bypass QoS, so the pacer's own wait never reads as a
//! queue.

use std::collections::VecDeque;

use strims_base::path::{LATEST_MAX_AGE, PathReading, latest_excess};
use strims_base::time::{Duration, Time};

/// How often the rate is taken again.
pub const PACE_UPDATE: Duration = Duration::from_millis(100);
/// How long a reading counts toward what the links delivered.
pub const BW_SPAN: Duration = Duration::from_secs(10);
/// The links' latest excess past which the host paces ...
pub const QUEUE_TARGET: Duration = Duration::from_millis(25);
/// ... once it has stood there this long.
pub const QUEUE_HOLD: Duration = Duration::from_millis(400);
/// The rate over the most the links delivered, while a queue stands.
pub const HEADROOM: f64 = 2.5;
/// How much the rate grows at each update while none does.
pub const GROWTH: f64 = 1.25;
/// The rate while unpaced: QoS's default limit, none ...
pub const UNPACED: u64 = strims_qos::DEFAULT_RATE_LIMIT;
/// ... until a queue stands, and again once none has for this long.
pub const UNPACE_AFTER: Duration = Duration::from_secs(10);
/// The least rate, in bytes a second.
pub const MIN_RATE: u64 = 32 * 1024;
/// The burst the rate limit lets through, in time at the rate: at
/// [`HEADROOM`], 50 ms of what the uplink carries, under the 100 ms queue
/// of a residential uplink.
pub const PACE_BURST: Duration = Duration::from_millis(20);

/// See the [module docs](self).
#[derive(Debug, Clone)]
pub struct EgressPacer {
    rate: u64,
    /// When a queue last stood, while the host paces.
    paced: Option<Time>,
    /// Since when the excess has stood past [`QUEUE_TARGET`] at every
    /// update.
    standing: Option<Time>,
    next: Time,
    /// What the links delivered together at each update of the last
    /// [`BW_SPAN`], each over every later one (a sliding most). Bound: a
    /// span's updates.
    delivered: VecDeque<(Time, u64)>,
    /// Scratch for [`latest_excess`]. Bound: the host's links.
    scratch: Vec<Duration>,
}

impl Default for EgressPacer {
    fn default() -> EgressPacer {
        EgressPacer {
            rate: UNPACED,
            paced: None,
            standing: None,
            next: Time::EPOCH,
            delivered: VecDeque::new(),
            scratch: Vec::new(),
        }
    }
}

impl EgressPacer {
    /// The rate, in bytes a second.
    pub fn rate(&self) -> u64 {
        self.rate
    }

    /// Whether an update is due at `now`.
    pub fn due(&self, now: Time) -> bool {
        now >= self.next
    }

    /// Takes the rate again from the links' `readings` at `now` when an
    /// update is due; returns the new rate then.
    pub fn update(
        &mut self,
        now: Time,
        readings: impl IntoIterator<Item = PathReading>,
    ) -> Option<u64> {
        if now < self.next {
            return None;
        }
        self.next = now + PACE_UPDATE;
        let fresh: Vec<PathReading> = readings
            .into_iter()
            .filter(|r| r.age(now) <= LATEST_MAX_AGE)
            .collect();
        if fresh.is_empty() {
            return None;
        }
        let sum = fresh.iter().filter_map(|r| r.rate).sum::<u64>();
        while self.delivered.back().is_some_and(|&(_, d)| d <= sum) {
            self.delivered.pop_back();
        }
        self.delivered.push_back((now, sum));
        while self
            .delivered
            .front()
            .is_some_and(|&(t, _)| t + BW_SPAN < now)
        {
            self.delivered.pop_front();
        }
        let most = self.delivered.front().map_or(0, |&(_, d)| d) as f64;
        if latest_excess(fresh, now, &mut self.scratch).is_some_and(|q| q >= QUEUE_TARGET) {
            self.standing.get_or_insert(now);
        } else {
            self.standing = None;
        }
        let queued = self.standing.is_some_and(|t| now - t >= QUEUE_HOLD);
        if queued {
            self.paced = Some(now);
        } else if self.paced.is_some_and(|t| t + UNPACE_AFTER < now) {
            self.paced = None;
        }
        let rate = match self.paced {
            None => UNPACED as f64,
            Some(_) if queued => most * HEADROOM,
            Some(_) => (self.rate as f64 * GROWTH).clamp(most * HEADROOM, UNPACED as f64),
        };
        self.rate = (rate as u64).max(MIN_RATE);
        Some(self.rate)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn t(ms: i64) -> Time {
        Time::from_unix_millis(1_700_000_000_000 + ms)
    }

    /// A reading taken at `at` of a link carrying `rate` bytes a second
    /// with `excess` past its 20 ms base.
    fn link(at: Time, rate: u64, excess: Duration) -> PathReading {
        let base = Duration::from_millis(20);
        let rtt = base + excess;
        PathReading {
            samples: 10,
            at,
            rtt,
            base: Some(base),
            recent_min: Some(rtt),
            in_flight: rate * u64::try_from(rtt.as_nanos()).unwrap() / 1_000_000_000,
            rate: Some(rate),
            queued: None,
            written_since: 0,
        }
    }

    /// The rates the pacer takes at 100 ms steps from `from`, four links
    /// carrying `rate` each with `excess` ms of it.
    fn run(p: &mut EgressPacer, from: i64, steps: &[(u64, i64)]) -> Vec<u64> {
        steps
            .iter()
            .enumerate()
            .filter_map(|(k, &(rate, excess))| {
                let now = t(from + 100 * i64::try_from(k).unwrap());
                let links = vec![link(now, rate, Duration::from_millis(excess)); 4];
                p.update(now, links)
            })
            .collect()
    }

    /// Unpaced until a queue has stood for the hold; then 2.5 times the
    /// most the links delivered over the span, which a queue's reading
    /// never lowers.
    #[test]
    fn a_queue_paces_the_host_past_the_most_its_links_delivered() {
        let mut p = EgressPacer::default();
        assert_eq!(
            run(&mut p, 0, &[(250_000, 0), (300_000, 0)]),
            vec![UNPACED, UNPACED]
        );
        assert_eq!(
            run(&mut p, 200, &[(100_000, 40); 5]),
            vec![UNPACED, UNPACED, UNPACED, UNPACED, 3_000_000]
        );
        assert_eq!(run(&mut p, 700, &[(50_000, 40)]), vec![3_000_000]);
        // Once the 1.2 MB/s reading leaves the span: what is left in it.
        assert_eq!(run(&mut p, 10_150, &[(50_000, 40)]), vec![1_000_000]);
    }

    /// A queue that drains within the hold, as one each burst raises,
    /// leaves the host unpaced.
    #[test]
    fn a_passing_queue_leaves_the_host_unpaced() {
        let mut p = EgressPacer::default();
        let bursts = [(250_000, 40), (250_000, 40), (250_000, 0)].repeat(4);
        assert!(run(&mut p, 0, &bursts).iter().all(|&r| r == UNPACED));
    }

    /// While no queue stands the rate grows from where it was, and a
    /// queue's reading takes it back to 2.5 times the most delivered.
    #[test]
    fn without_a_queue_the_rate_grows() {
        let mut p = EgressPacer::default();
        assert_eq!(run(&mut p, 0, &[(250_000, 40); 5]).last(), Some(&2_500_000));
        let grown = (2_500_000.0 * GROWTH) as u64;
        let again = (grown as f64 * GROWTH) as u64;
        assert_eq!(
            run(&mut p, 500, &[(250_000, 0), (250_000, 0)]),
            vec![grown, again]
        );
        assert_eq!(
            run(&mut p, 700, &[(250_000, 40); 5]).last(),
            Some(&2_500_000)
        );
    }

    /// Once no queue has stood for ten seconds the host goes unpaced.
    #[test]
    fn a_quiet_host_goes_unpaced() {
        let mut p = EgressPacer::default();
        run(&mut p, 0, &[(250_000, 40); 5]);
        assert_ne!(run(&mut p, 10_400, &[(250_000, 0)]), vec![UNPACED]);
        assert_eq!(run(&mut p, 10_500, &[(250_000, 0)]), vec![UNPACED]);
    }

    /// Without fresh readings the rate holds; it never falls under the
    /// least.
    #[test]
    fn stale_readings_hold_the_rate_and_it_keeps_the_least() {
        let mut p = EgressPacer::default();
        let stale = vec![link(t(-2_000), 1, Duration::ZERO)];
        assert_eq!(p.update(t(0), stale), None);
        assert_eq!(p.rate(), UNPACED);
        for k in 1..=5 {
            let low = vec![link(t(100 * k), 1_000, Duration::from_millis(50))];
            p.update(t(100 * k), low);
        }
        assert_eq!(p.rate(), MIN_RATE);
        assert!(!p.due(t(550)));
    }
}
