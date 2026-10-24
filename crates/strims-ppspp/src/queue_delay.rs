// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Queueing delay on the node's uplink, from ping round trips.
//!
//! A ping's nonce is our clock when its frame was built, so its round trip
//! includes whatever waits past the runner: the driver's link buffers, the
//! kernel's unsent bytes and congestion window, the access network's queue.
//! A queue on our uplink delays every peer's round trip by about the same
//! amount; one on a peer's path (its downlink, its own upload of the pong)
//! delays only that peer's. So the node's queueing delay is a low quantile,
//! across peers, of how far each peer's recent round trips stand above its
//! base: a slow neighbour, or a minority that delays its pongs, can't move
//! it.
//!
//! Each peer's base is its least round trip over [`BASE_BUCKETS`] buckets of
//! [`BASE_BUCKET`] (ten minutes). Queued samples can't raise a minimum,
//! and a channel's first ping goes out with its handshake, before any
//! data, so a queue that builds later reads as excess; one that stands
//! for the whole window becomes the base.

use strims_base::time::{Duration, Time};

/// The span of one base bucket ...
pub(crate) const BASE_BUCKET: Duration = Duration::from_secs(30);
/// ... and how many make up the base window.
pub(crate) const BASE_BUCKETS: usize = 20;
/// Round trips that arrived this recently stand for the path's current
/// delay: the least of them, so one sample that met a burst doesn't read
/// as a standing queue.
pub(crate) const RECENT: Duration = Duration::from_secs(3);
/// Round trips a peer keeps between folds into its base.
const SAMPLES: usize = 8;
/// The fewest peers with recent samples that give a reading.
pub(crate) const MIN_PEERS: usize = 3;

/// One peer's round trips.
#[derive(Clone, Debug, Default)]
pub(crate) struct PathDelay {
    /// The latest round trips with their arrival times, oldest first.
    /// Bound: [`SAMPLES`].
    samples: [(Time, Duration); SAMPLES],
    len: usize,
    /// Per bucket, its index (arrival time over [`BASE_BUCKET`]) and the
    /// least clean round trip in it.
    base: [Option<(i64, Duration)>; BASE_BUCKETS],
    /// The arrival time of the newest sample folded into `base`.
    folded: Time,
}

impl PathDelay {
    /// Records a round trip that arrived at `now`.
    pub(crate) fn add(&mut self, now: Time, rtt: Duration) {
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
    pub(crate) fn fold(&mut self) {
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

    /// How far the least round trip of the last [`RECENT`] stands above
    /// the base. `None` without a recent sample.
    pub(crate) fn excess(&self, now: Time) -> Option<Duration> {
        let current = self
            .recent_samples()
            .iter()
            .filter(|s| now - s.0 <= RECENT)
            .map(|s| s.1)
            .min()?;
        let oldest = now.unix_nanos() / BASE_BUCKET.as_nanos() - BASE_BUCKETS as i64;
        let base = self
            .base
            .iter()
            .flatten()
            .filter(|(i, _)| *i > oldest)
            .map(|(_, d)| *d)
            .min();
        Some(base.map_or(Duration::ZERO, |b| (current - b).max(Duration::ZERO)))
    }
}

/// The node's queueing delay from its peers' excesses (`scratch` is
/// reordered): the ⌊n/3⌋+1-th smallest, so it reads our queue only when it
/// delays most peers. `None` with fewer than [`MIN_PEERS`].
pub(crate) fn aggregate(scratch: &mut [Duration]) -> Option<Duration> {
    if scratch.len() < MIN_PEERS {
        return None;
    }
    let k = scratch.len() / 3;
    scratch.select_nth_unstable(k);
    scratch.get(k).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: i64) -> Time {
        Time::from_unix_millis(1_000_000 + ms)
    }

    fn ms(n: i64) -> Duration {
        Duration::from_millis(n)
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

    #[test]
    fn one_slow_peer_doesnt_move_the_aggregate() {
        let mut d = vec![ms(2), ms(500), ms(3), ms(1)];
        assert_eq!(aggregate(&mut d), Some(ms(2)));
        let mut queued = vec![ms(90), ms(500), ms(95), ms(80)];
        assert_eq!(aggregate(&mut queued), Some(ms(90)));
        assert_eq!(aggregate(&mut [ms(90), ms(95)]), None);
    }
}
