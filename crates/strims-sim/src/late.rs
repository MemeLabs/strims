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
//! Each host draws from the model of its kind ([`HostKind`]: native,
//! Chromium or Firefox, by the browser its data channels model, as the
//! `hosts` table names it) for each source ([`Source`]): its node task's
//! wake-ups, and its harness's timers, the frame writer's waits and the
//! timer probe's ([`model`]). Each model is the timer bench's: the shares
//! of 0, 1 and 2 ms late, and the measured spread of those 3 ms or more
//! late, that the `timers` table of thin-star testbed runs reads (the
//! farm's `testbed-timers` matrix: a seed and three viewers of one kind on
//! a host of their own, a 10 Mbit/s seed, 90 s, idle and beside a
//! competing upload, every node of the kind, each run weighed alike, its
//! first two repeats). The bench (scoring.sql's `timer_fit_of`) holds a
//! twin of those runs to the testbed's shares, mean and p99, kind by kind
//! and source by source. A Firefox node's model is a page's served
//! cross-origin isolated, as the testbed's timer log serves it, whose
//! timers run less late than a plain page's (PLAN). The bench's runs keep
//! the timer probe on, which runs in the node's own worker, and a
//! browser's writer runs otherwise beside it: without the probe a Firefox
//! seed's writes run later, a Chromium seed's less late (PLAN). A
//! browser's timer for a deadline already due resolves at once, as
//! strims-wasm's `Sleep` and `setTimeout`-free waits do, drawing nothing;
//! a native node's fires on its next tick, as any. A testbed node's line
//! timer and a browser's host pacer have no draw: the controller samples
//! on time, and the model's host pacer polls on time.
//!
//! The model draws no stalls. In the bench's runs, each on a host of its
//! own, no timer of the stream fires 10 ms late, on any kind, and no
//! native node's 3 ms late, so its models have no tail; a browser's 3+ ms
//! timers are its clamps' few ms. A host with other load stalls timers
//! for tens to hundreds of ms, one node's or every node's at once, which
//! the model leaves out (PLAN).
//!
//! Each host's lateness comes from its own stream (see the crate's
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

use crate::spec::{HostIx, Scenario};
use crate::wan::SctpProfile;

/// What a host is, as its timers' lateness goes ([`model`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostKind {
    Native,
    Chromium,
    Firefox,
}

impl HostKind {
    /// The kind of host `h` of `s`, the `hosts` table's (its row's `kind`,
    /// `Report::scenario_host_row`): the browser whose data channels it
    /// models (the window model's under Chromium's or Firefox's profile),
    /// else native.
    pub fn of_host(s: &Scenario, h: HostIx) -> HostKind {
        let window = s.send_buffer.is_some_and(|b| b.window);
        match s
            .hosts
            .get(h)
            .and_then(|x| x.rtc_stack.filter(|_| window && x.webrtc))
        {
            Some(p) if p == SctpProfile::CHROMIUM => HostKind::Chromium,
            Some(p) if p == SctpProfile::FIREFOX => HostKind::Firefox,
            _ => HostKind::Native,
        }
    }

    /// Its name in the tables: `native`, `chromium`, `firefox`.
    pub fn name(self) -> &'static str {
        match self {
            HostKind::Native => "native",
            HostKind::Chromium => "chromium",
            HostKind::Firefox => "firefox",
        }
    }

    /// Whether its timers resolve a deadline already due at once: a
    /// browser's do (strims-wasm's `Sleep` checks its deadline before it
    /// arms a timer), where a native node's tokio timer fires on a tick.
    pub(crate) fn resolves_due_at_once(self) -> bool {
        self != HostKind::Native
    }
}

/// Whose timer a draw is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The node task's wake-up ([`LateClock`]).
    NodeTask,
    /// A harness's timer: the frame writer's wait for a write's time, and
    /// the timer probe's.
    Writer,
}

/// A host's lateness stream, shared by its incarnations' clocks.
#[derive(Clone, Debug)]
pub struct Lateness(Arc<Mutex<DetRng>>, HostKind);

impl Lateness {
    pub(crate) fn new(rng: DetRng, kind: HostKind) -> Lateness {
        Lateness(Arc::new(Mutex::new(rng)), kind)
    }

    /// How late the host's next timer of `source` fires, armed for a
    /// deadline that was `due` already (at or before the arming code's
    /// now): at once on a host whose timers resolve it so, drawing
    /// nothing ([`HostKind`]), else as [`lateness`] draws.
    pub(crate) fn draw(&self, source: Source, due: bool) -> Duration {
        if due && self.1.resolves_due_at_once() {
            return Duration::ZERO;
        }
        lateness(
            &mut self.0.lock().unwrap_or_else(PoisonError::into_inner),
            self.1,
            source,
        )
    }

    /// The host's kind.
    pub(crate) fn kind(&self) -> HostKind {
        self.1
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
                let d = self.stream.draw(Source::NodeTask, t <= self.inner.now());
                *last = Some((t, d));
                d
            }
        }
    }
}

/// How late a timer fires ([`model`]): `shares`, in parts of [`PARTS`],
/// of 0, 1 and 2 ms late, and the rest from `tail`, 3 ms or more: whole
/// ms, ascending, each as likely as the others; empty where the shares
/// hold every part.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Model {
    pub(crate) shares: [u32; 3],
    pub(crate) tail: &'static [u32],
}

/// The most a timer fires late, in ms: the largest of the tails
/// ([`model`]).
pub const MAX_LATE_MS: i64 = 8;

/// The parts [`Model::shares`] are in.
const PARTS: u32 = 1000;

/// How late `kind`'s timers of `source` fire. Each is the timer bench's
/// testbed runs' (see the [module docs](self)): the shares of 0, 1 and 2
/// ms late from the `timers` table, every node of the kind, each run
/// weighed alike, and for the node task a step that undoes what the
/// model's own passes leave out (a timer drawn late enough for input to
/// pass first fires no row, so the sim's rows read less late than its
/// draws); and the tail, the 3+ ms timers' 20 quantiles, none for a
/// native node, whose timers there never fired 3 ms late.
pub(crate) fn model(kind: HostKind, source: Source) -> Model {
    match (kind, source) {
        (HostKind::Native, Source::NodeTask) => Model {
            shares: [107, 733, 160],
            tail: &[],
        },
        (HostKind::Native, Source::Writer) => Model {
            shares: [127, 686, 187],
            tail: &[],
        },
        (HostKind::Chromium, Source::NodeTask) => Model {
            shares: [658, 233, 64],
            tail: &[3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 7, 8],
        },
        (HostKind::Chromium, Source::Writer) => Model {
            shares: [755, 105, 35],
            tail: &[3, 3, 3, 3, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 6, 7, 7, 7, 8],
        },
        (HostKind::Firefox, Source::NodeTask) => Model {
            shares: [593, 266, 85],
            tail: &[3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 6],
        },
        (HostKind::Firefox, Source::Writer) => Model {
            shares: [874, 103, 6],
            tail: &[3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 6],
        },
    }
}

/// How late a timer of `source` fires on a host of `kind`: one draw of
/// `rng`, whose remainder by [`PARTS`] picks the bucket at its
/// [`Model::shares`], and whose quotient the tail's value.
pub(crate) fn lateness(rng: &mut DetRng, kind: HostKind, source: Source) -> Duration {
    let x = rng.next_u32();
    let m = model(kind, source);
    let mut b = x % PARTS;
    for (ms, share) in (0..).zip(m.shares) {
        if b < share {
            return Duration::from_millis(ms);
        }
        b -= share;
    }
    let i = usize::try_from(x / PARTS).unwrap_or(0) % m.tail.len().max(1);
    Duration::from_millis(i64::from(m.tail.get(i).copied().unwrap_or(3)))
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

    const KINDS: [HostKind; 3] = [HostKind::Native, HostKind::Chromium, HostKind::Firefox];
    const SOURCES: [Source; 2] = [Source::NodeTask, Source::Writer];

    /// The 0, 1, 2 and 3+ ms counts of `n` timers `draw` fires.
    fn buckets(n: u32, mut draw: impl FnMut() -> Duration) -> [u32; 4] {
        let mut b = [0u32; 4];
        for _ in 0..n {
            let ms = draw().as_millis();
            assert!((0..=MAX_LATE_MS).contains(&ms), "{ms} ms");
            b[usize::try_from(ms.min(3)).unwrap_or(3)] += 1;
        }
        b
    }

    /// Whether counts out of 100000 come at `m`'s shares, within 0.5 %.
    fn at_shares(b: [u32; 4], m: Model) -> bool {
        let tail = PARTS - m.shares.iter().sum::<u32>();
        let want = [m.shares[0], m.shares[1], m.shares[2], tail];
        b.iter()
            .zip(want)
            .all(|(&got, w)| got.abs_diff(w * 100) < 500)
    }

    /// Each kind's and source's draws come at its model's shares, and its
    /// 3+ ms draws are its tail's values, every one drawn; a model has a
    /// tail exactly when its shares leave parts to it; the most any tail
    /// holds is MAX_LATE_MS.
    #[test]
    fn a_timer_fires_at_its_kinds_and_sources_model() {
        let mut most = 0;
        for kind in KINDS {
            for source in SOURCES {
                let m = model(kind, source);
                let held = m.shares.iter().sum::<u32>();
                assert!(held <= PARTS, "{kind:?} {source:?}");
                assert_eq!(m.tail.is_empty(), held == PARTS, "{kind:?} {source:?}");
                assert!(m.tail.is_sorted() && m.tail.iter().all(|&ms| ms >= 3));
                most = most.max(m.tail.last().map_or(0, |&ms| i64::from(ms)));
                let mut rng = det_rng(7);
                let mut seen = std::collections::BTreeSet::new();
                let b = buckets(100_000, || {
                    let d = lateness(&mut rng, kind, source);
                    if d.as_millis() >= 3 {
                        assert!(m.tail.contains(&(d.as_millis() as u32)));
                        seen.insert(d.as_millis());
                    }
                    d
                });
                assert!(at_shares(b, m), "{kind:?} {source:?}: {b:?}");
                let distinct: std::collections::BTreeSet<u32> = m.tail.iter().copied().collect();
                assert_eq!(
                    seen.len(),
                    distinct.len(),
                    "{kind:?} {source:?}: every tail value drawn"
                );
            }
        }
        assert_eq!(most, MAX_LATE_MS);
    }

    /// No two kinds and sources share a model: each is its own pool.
    #[test]
    fn every_kind_and_source_has_a_model_of_its_own() {
        let all: Vec<(HostKind, Source)> = KINDS
            .iter()
            .flat_map(|&k| SOURCES.iter().map(move |&s| (k, s)))
            .collect();
        for (i, &(ka, sa)) in all.iter().enumerate() {
            for &(kb, sb) in &all[i + 1..] {
                let (a, b) = (model(ka, sa), model(kb, sb));
                assert!(
                    a.shares != b.shares || a.tail != b.tail,
                    "{ka:?} {sa:?} and {kb:?} {sb:?}"
                );
            }
        }
    }

    /// A browser's timer for a deadline already due fires at once and
    /// draws nothing; a native node's draws as any.
    #[test]
    fn a_browsers_due_timer_fires_at_once() {
        for kind in [HostKind::Chromium, HostKind::Firefox] {
            let (stream, mut fresh) = (Lateness::new(det_rng(5), kind), det_rng(5));
            for source in SOURCES {
                assert_eq!(stream.draw(source, true), Duration::ZERO);
            }
            assert_eq!(
                stream.draw(Source::Writer, false),
                lateness(&mut fresh, kind, Source::Writer),
                "{kind:?}: the due ones drew nothing"
            );
        }
        let (stream, mut fresh) = (Lateness::new(det_rng(5), HostKind::Native), det_rng(5));
        assert_eq!(
            stream.draw(Source::NodeTask, true),
            lateness(&mut fresh, HostKind::Native, Source::NodeTask)
        );
    }

    /// A deadline fires no earlier than it would on time and at most
    /// [`MAX_LATE_MS`] later, and keeps its lateness while the timer is
    /// reset to it; a restart's clock goes on drawing where the host's
    /// stream was; and the clock draws the node task's model, not the
    /// writer's.
    #[test]
    fn a_deadline_keeps_its_lateness() {
        let epoch = Time::from_unix_millis(1_700_000_000_000);
        let inner = SimClock::with_base(epoch, Instant::now());
        let stream = Lateness::new(det_rng(3), HostKind::Native);
        let late = LateClock::new(inner, stream.clone());
        for k in 0..200 {
            let t = epoch + Duration::from_secs(1000) + Duration::from_millis(100 * k);
            let at = late.deadline(t);
            assert!(at >= inner.deadline(t));
            assert!(at <= inner.deadline(t + Duration::from_millis(MAX_LATE_MS)));
            assert_eq!(late.deadline(t), at, "reset to the same deadline");
        }
        // The same stream from the start, and a clock of a second
        // incarnation sharing it after 200 draws, read alike.
        let mut fresh = det_rng(3);
        let draw = |rng: &mut DetRng| lateness(rng, HostKind::Native, Source::NodeTask);
        for _ in 0..200 {
            let _ = draw(&mut fresh);
        }
        let again = LateClock::new(inner, stream);
        let t = epoch + Duration::from_secs(5000);
        assert_eq!(again.deadline(t), inner.deadline(t + draw(&mut fresh)));
        // Over many deadlines the clock's draws are the node task's.
        let clock = LateClock::new(inner, Lateness::new(det_rng(9), HostKind::Native));
        let mut k = 0;
        let b = buckets(100_000, || {
            k += 1;
            let t = epoch + Duration::from_secs(10_000) + Duration::from_millis(10 * k);
            let base = inner.deadline(t);
            Duration::from(clock.deadline(t) - base)
        });
        assert!(
            at_shares(b, model(HostKind::Native, Source::NodeTask)),
            "{b:?}"
        );
        assert!(
            !at_shares(b, model(HostKind::Native, Source::Writer)),
            "{b:?}"
        );
    }

    /// Lateness draws from the run's rng last of the scenario's draws: a
    /// trickle's arrivals, the dishonest mix and the clock offsets come out
    /// as they do without it, and then its streams (a host's each, and with
    /// the probe a probe's each) fork the rng the controller's choices
    /// take, which off it doesn't touch; every host's stream is its kind's.
    #[test]
    fn lateness_forks_last_of_the_scenarios_draws() {
        use crate::Scenario;
        use crate::arrivals::Arrivals;
        use crate::spec::{SendBuffer, SwarmSpec, Transport};
        use crate::world::World;
        let build = |late: bool, probe: bool| {
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
            s.send_buffer = Some(SendBuffer {
                uplink_queue: std::time::Duration::from_millis(50),
                window: true,
            });
            s.over_webrtc(&[Some(SctpProfile::CHROMIUM), Some(SctpProfile::FIREFOX)]);
            s.timer_lateness = late;
            s.timer_probe = probe;
            World::new(s, 11, None, None)
        };
        let (off, on, probed) = (build(false, false), build(true, false), build(true, true));
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
        assert_eq!(drawn(&on), drawn(&probed));
        let next = |w: &World| w.with_rng(|r| r.next_u64());
        assert_ne!(
            next(&off),
            next(&on),
            "the streams fork the controller's rng"
        );
        assert_ne!(next(&on), next(&probed), "the probe's fork it again");
        assert!(off.lateness.is_empty() && off.probe_lateness.is_empty());
        assert_eq!(on.lateness.len(), on.spec.hosts.len());
        assert!(on.probe_lateness.is_empty());
        assert_eq!(probed.probe_lateness.len(), probed.spec.hosts.len());
        for (h, l) in on.lateness.iter().enumerate() {
            assert_eq!(l.kind(), HostKind::of_host(&on.spec, h), "host {h}");
        }
        let kinds: std::collections::BTreeSet<&str> =
            on.lateness.iter().map(|l| l.kind().name()).collect();
        assert!(
            kinds.contains("chromium") && kinds.contains("firefox"),
            "{kinds:?}"
        );
    }

    /// A host's kind is the browser its data channels model under the
    /// window model, else native, as the `hosts` table has it.
    #[test]
    fn a_hosts_kind_is_its_modelled_browsers() {
        use crate::Scenario;
        use crate::spec::SendBuffer;
        let mut s = Scenario::new("t");
        s.hosts("v", 4, None);
        s.over_webrtc(&[
            Some(SctpProfile::CHROMIUM),
            Some(SctpProfile::FIREFOX),
            Some(SctpProfile::STR0M),
            None,
        ]);
        let kinds = |s: &Scenario| -> Vec<&str> {
            (0..4).map(|h| HostKind::of_host(s, h).name()).collect()
        };
        // Outside the window model, no host models a browser's channels.
        assert_eq!(kinds(&s), ["native"; 4]);
        s.send_buffer = Some(SendBuffer {
            uplink_queue: std::time::Duration::from_millis(50),
            window: true,
        });
        assert_eq!(kinds(&s), ["chromium", "firefox", "native", "native"]);
    }
}
