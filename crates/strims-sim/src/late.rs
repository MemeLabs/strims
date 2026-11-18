// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A node's timers fire late, as a real node's do
//! ([`Scenario::timer_lateness`](crate::Scenario::timer_lateness)).
//!
//! Under turmoil a timer fires at its deadline to the millisecond, so the
//! node's work that runs on its timer keeps one phase against everything
//! else that runs on a fixed cadence: the stream's writes, the runner's
//! ticks, the probes. A real node's timer fires a millisecond or two late:
//! tokio's wheel rounds a deadline up to its millisecond tick, the thread
//! wakes from epoll, and the clock is read in whole milliseconds. Rules
//! that schedule from the time a pass ran (the adaptive seed's check,
//! `now + ALLOC_CHECK_INTERVAL`) then slip a whole tick whenever the next
//! pass runs less late than the last, and so walk through the phases of
//! the writes, and what they read over a window (the budget's stream rate,
//! the writes stored over two seconds) moves with the phase.
//!
//! What runs late is the node task's timer wake-up: the sleep its driver
//! takes until the core's next deadline (`NodeIo::reset_sleep` in
//! strims-node's native driver), so a pass that a deadline wakes (the
//! runner's, the schedulers' and vnic's timers) runs late, and one that
//! input wakes (a frame, a command) runs when the input comes. The frame
//! writer's writes run late too, each frame stamped with its write's due
//! time (`Report::written_at`), as the testbed node stamps it, so a
//! viewer's lag counts the lateness. Everything else keeps its deadline:
//! the media writer, the transfers' timer (strims-node's `NodeTimer` over
//! the host's own `SimClock`, skewed as the node's clock is, without the
//! lateness), the driver's own sleeps (its accept and receive backoffs)
//! and the WAN model's transport timers.
//!
//! Every host draws the same shares (`lateness`): 0 ms one time in ten,
//! 1 ms six, 2 ms three, rounded from what a native testbed seed's 1-s
//! record timer read in three runs pooled (strims-testbed's
//! `tick_late_ms`: 0 ms in 9 % of its seconds, 1 ms in 60 %, 2 ms in 28 %,
//! 3 ms or more in 3 %). Larger pools read otherwise: 168 native seeds of
//! the farm's testbed runs read 11.0, 68.2, 18.5 and 2.3 %, and the
//! browser runs' Chromium seeds read 0 ms in 92.7 % of their seconds, yet
//! a twin's browser hosts draw the native shares too. The frame writer
//! draws them as well, which the testbed's writes don't follow: a native
//! seed's `write_late_ms`, the latest of a second's writes past its due
//! time, reads 1 ms in about half its seconds and 2 ms in the other half,
//! where the latest of a second's four draws reads 1 ms in a quarter and
//! 2 ms in three quarters.
//!
//! Each host's lateness comes from its own rng (see the crate's
//! determinism notes), which its incarnations' node clocks and its frame
//! writer draw from in turn. A deadline's lateness is drawn once, so a node
//! that resets its timer to the deadline it had keeps it, and passes that
//! change no deadline (a sample's query) draw nothing.

// The node task's clock must be `Send + Sync`; everything runs on turmoil's
// thread, so the lock is never contended.
#![allow(clippy::disallowed_types)]

use std::sync::{Arc, Mutex, PoisonError};

use rand_core::Rng;
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_node::{Clock, SimClock};
use tokio::time::Instant;

/// Mixed into the run's seed for the rng the hosts' lateness streams are
/// forked from, apart from the run's own.
pub(crate) const RNG_SALT: u64 = 0x1a7e_c10c;

/// A host's lateness stream, shared by its incarnations' clocks.
#[derive(Clone, Debug)]
pub struct Lateness(Arc<Mutex<DetRng>>);

impl Lateness {
    pub(crate) fn new(rng: DetRng) -> Lateness {
        Lateness(Arc::new(Mutex::new(rng)))
    }

    /// How late the next timer fires ([`lateness`]).
    pub(crate) fn draw(&self) -> Duration {
        lateness(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// A [`SimClock`] whose deadlines fire late.
#[derive(Clone, Debug)]
pub(crate) struct LateClock {
    inner: SimClock,
    stream: Lateness,
    /// The last deadline and its lateness.
    last: Arc<Mutex<Option<(Time, Duration)>>>,
}

impl LateClock {
    pub(crate) fn new(inner: SimClock, stream: Lateness) -> LateClock {
        LateClock {
            inner,
            stream,
            last: Arc::new(Mutex::new(None)),
        }
    }

    /// The lateness of deadline `t`: the last one's if `t` is the last
    /// deadline, else a new draw.
    fn late(&self, t: Time) -> Duration {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        match *last {
            Some((at, d)) if at == t => d,
            _ => {
                let d = self.stream.draw();
                *last = Some((t, d));
                d
            }
        }
    }
}

/// The most a timer fires late, in ms ([`lateness`]).
pub const MAX_LATE_MS: i64 = 2;

/// How late a timer fires: 0 ms one time in ten, 1 ms six in ten, 2 ms
/// three in ten.
pub(crate) fn lateness(rng: &mut DetRng) -> Duration {
    Duration::from_millis(match rng.next_u32() % 10 {
        0 => 0,
        1..=6 => 1,
        _ => 2,
    })
}

impl Clock for LateClock {
    fn now(&self) -> Time {
        self.inner.now()
    }

    fn precise_now(&self) -> Time {
        self.inner.precise_now()
    }

    fn deadline(&self, t: Time) -> Instant {
        self.inner.deadline(t + self.late(t))
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use strims_base::rng::det_rng;

    /// The draws' shares: 0 ms one in ten, 1 ms six, 2 ms three.
    #[test]
    fn a_timer_fires_up_to_two_milliseconds_late() {
        let mut rng = det_rng(7);
        let mut n = [0u32; 3];
        for _ in 0..10_000 {
            let d = lateness(&mut rng).as_millis();
            assert!((0..=2).contains(&d), "{d} ms");
            n[usize::try_from(d).unwrap_or(0)] += 1;
        }
        for (i, want) in [1_000, 6_000, 3_000].into_iter().enumerate() {
            assert!(n[i].abs_diff(want) < 300, "{i} ms: {} of 10000", n[i]);
        }
    }

    /// A deadline fires no earlier than it would on time and at most 2 ms
    /// later, and keeps its lateness while the timer is reset to it; a
    /// restart's clock goes on drawing where the host's stream was.
    #[test]
    fn a_deadline_keeps_its_lateness() {
        let epoch = Time::from_unix_millis(1_700_000_000_000);
        let inner = SimClock::with_base(epoch, Instant::now());
        let stream = Lateness::new(det_rng(3));
        let late = LateClock::new(inner, stream.clone());
        for k in 0..200 {
            let t = epoch + Duration::from_millis(100 * k);
            let at = late.deadline(t);
            assert!(at >= inner.deadline(t));
            assert!(at <= inner.deadline(t + Duration::from_millis(2)));
            assert_eq!(late.deadline(t), at, "reset to the same deadline");
        }
        // The same stream from the start, and a clock of a second
        // incarnation sharing it after 200 draws, read alike.
        let mut fresh = det_rng(3);
        for _ in 0..200 {
            let _ = lateness(&mut fresh);
        }
        let again = LateClock::new(inner, stream);
        let t = epoch + Duration::from_secs(100);
        assert_eq!(again.deadline(t), inner.deadline(t + lateness(&mut fresh)));
    }

    /// Turning lateness on draws from no rng the run's other choices draw
    /// from: a trickle's arrivals, the dishonest mix and the clock offsets
    /// come out as they do without it, the run's rng is where it would be
    /// for the controller's choices after them, and only the lateness
    /// streams are added.
    #[test]
    fn lateness_moves_no_other_draw() {
        use crate::Scenario;
        use crate::arrivals::Arrivals;
        use crate::spec::{SwarmSpec, Transport};
        use crate::world::World;
        let build = |late: bool| {
            let mut s = Scenario::new("t");
            let seed = s.host("seed", None);
            let vs = s.hosts("v", 12, None);
            for &v in &vs {
                s.link(v, seed, Transport::Tcp);
            }
            s.swarm(SwarmSpec::live(
                seed,
                vs,
                1024,
                std::time::Duration::from_secs(5),
                std::time::Duration::from_secs(65),
            ));
            s.arrivals = Arrivals::Trickle { rate: None };
            s.mix = true;
            s.timer_lateness = late;
            World::new(s, 11, None, None)
        };
        let (off, on) = (build(false), build(true));
        // Each of those choices drew something.
        assert!(!off.clock_offsets.is_empty());
        assert!(off.spec.hosts.iter().any(|h| h.arrive_at.is_some()));
        assert!(
            off.spec
                .hosts
                .iter()
                .any(|h| h.class != crate::spec::HostClass::Honest)
        );
        let drawn = |w: &World| {
            w.spec
                .hosts
                .iter()
                .map(|h| (h.arrive_at, format!("{:?}", h.class)))
                .collect::<Vec<_>>()
        };
        assert_eq!(drawn(&off), drawn(&on));
        assert_eq!(off.clock_offsets, on.clock_offsets);
        let next = |w: &World| w.with_rng(|r| r.next_u64());
        assert_eq!(next(&off), next(&on), "the controller's choices");
        assert!(off.lateness.is_empty());
        assert_eq!(on.lateness.len(), on.spec.hosts.len());
    }
}
