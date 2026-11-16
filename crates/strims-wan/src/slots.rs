// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! An access link's medium access in slots: what its shaper passed leaves
//! when the next slot opens.
//!
//! A cable modem sends upstream only in the minislots the CMTS grants it,
//! on request: an idle modem's first packet waits for its request to be
//! granted (2-8 ms: CableLabs, Low Latency DOCSIS white paper, 2019, s3),
//! and a loaded one's grants follow its piggybacked requests, a cycle of
//! about 5 ms (Cisco, "Upstream Scheduler Mode Configuration for the Cisco
//! uBR CMTS"). A Wi-Fi station sends what it holds in one aggregate once
//! it wins the medium. Both act where a packet leaves the modem, on what
//! its shaper passed. [`Slots`] is the model the Docker testbed shapes them
//! with, a netem of `slot <min> <max>` alone after the shaper: a packet that
//! is ready while the current slot is still to come leaves when it opens;
//! one ready after it draws the next, uniformly `min` to `max` after the
//! time it is looked at ([`SlotState::depart`], which with no delay of its
//! own netem's dequeue is).

use core::fmt;

use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};

use crate::sample::unit;

/// Slots that open `min` to `max` (uniformly) after the packet that draws
/// them is looked at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slots {
    pub min: Duration,
    pub max: Duration,
}

impl Slots {
    /// netem's form, `slot <min>us <max>us`, for run.sh.
    pub fn netem(self) -> String {
        format!(
            "slot {}us {}us",
            self.min.as_nanos() / 1000,
            self.max.as_nanos() / 1000
        )
    }
}

impl fmt::Display for Slots {
    /// `<min>-<max>us`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-{}us",
            self.min.as_nanos() / 1000,
            self.max.as_nanos() / 1000
        )
    }
}

/// One link's slots: the next to open and the rng that draws them.
#[derive(Clone, Debug)]
pub struct SlotState {
    slots: Slots,
    next: Time,
    rng: DetRng,
}

impl SlotState {
    pub fn new(slots: Slots, rng: DetRng) -> SlotState {
        SlotState {
            slots,
            next: Time::NIL,
            rng,
        }
    }

    fn draw(&mut self, now: Time) -> Time {
        let span = (self.slots.max - self.slots.min).max(Duration::ZERO);
        let u = unit(&mut self.rng);
        now + self.slots.min + Duration::from_nanos((u * span.as_nanos() as f64) as i64)
    }

    /// When a packet that is ready at `ready` leaves, the link having
    /// looked at it first at `head` (when the packet before it left, or
    /// when it came to an empty link): a slot that opened before the packet
    /// was ready is replaced by one drawn from the time of the look, the
    /// link looks again when the packet is ready or the slot opens,
    /// whichever is later, and the packet leaves once both have come. A
    /// netem with no delay of its own dequeues so; one that also delays
    /// redraws a passed slot on every dequeue while a packet waits out its
    /// delay, which this doesn't follow.
    pub fn depart(&mut self, ready: Time, head: Time) -> Time {
        let mut now = head;
        loop {
            if self.next < ready {
                self.next = self.draw(now);
            }
            if ready <= now && self.next <= now {
                return now;
            }
            now = ready.max(self.next);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strims_base::rng::det_rng;

    fn t(us: i64) -> Time {
        Time::from_unix_millis(1_700_000_000_000) + Duration::from_micros(us)
    }

    fn fixed(ms: i64) -> SlotState {
        SlotState::new(
            Slots {
                min: Duration::from_millis(ms),
                max: Duration::from_millis(ms),
            },
            det_rng(1),
        )
    }

    #[test]
    fn an_idle_link_waits_a_slot() {
        let mut s = fixed(2);
        assert_eq!(s.depart(t(0), t(0)), t(2000));
        // Ready before the slot opened: it rides it.
        assert_eq!(s.depart(t(1500), t(2000)), t(2000));
        // Ready after: the next slot, drawn from when it was looked at.
        assert_eq!(s.depart(t(2500), t(2500)), t(4500));
    }

    #[test]
    fn a_packet_ready_later_than_its_look_draws_from_its_readiness() {
        // netem's delay: looked at on arrival, ready 20 ms on; the slot
        // drawn at the look opened before it was ready, so another is
        // drawn when it is.
        let mut s = fixed(2);
        assert_eq!(s.depart(t(20_000), t(0)), t(22_000));
        // A slot longer than the delay is taken as it comes.
        let mut s = fixed(30);
        assert_eq!(s.depart(t(20_000), t(0)), t(30_000));
    }

    #[test]
    fn a_loaded_link_sends_in_slot_batches() {
        let mut s = SlotState::new(
            Slots {
                min: Duration::from_millis(2),
                max: Duration::from_millis(8),
            },
            det_rng(7),
        );
        let (mut head, mut batches, mut last) = (t(0), 0, Time::NIL);
        let mut gaps = Vec::new();
        for k in 0..2000 {
            // A packet ready every 1.2 ms (10 Mbit/s of full frames).
            let ready = t(k * 1200);
            let d = s.depart(ready, head.max(ready));
            assert!(d >= ready);
            if d != last {
                if last != Time::NIL {
                    gaps.push((d - last).as_nanos());
                }
                batches += 1;
                last = d;
            }
            head = d;
        }
        let mean = gaps.iter().sum::<i64>() as f64 / gaps.len() as f64 / 1e6;
        // Each slot opens 2-8 ms after the first packet ready past the
        // last: 5 ms on average, plus that packet's wait for readiness.
        assert!((5.0..6.5).contains(&mean), "{mean} ms between slots");
        assert!(batches < 2000 / 3, "{batches}");
    }

    #[test]
    fn netem_form() {
        let s = Slots {
            min: Duration::from_millis(2),
            max: Duration::from_millis(8),
        };
        assert_eq!(s.netem(), "slot 2000us 8000us");
    }
}
