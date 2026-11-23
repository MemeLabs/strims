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
//! With [`Scenario::pacer_lateness`] a browser's host pacer runs late
//! too: each data channel's writer the host's pacer holds sleeps on a
//! timer of its own, `HOST_POLL` and its lateness from its hold or its
//! last poll, as strims-wasm's `DcSocket::send` does, the timers armed for
//! one deadline drawing one lateness (`crate::net`). With
//! [`Scenario::pacer_hold_start`] the holds a browser seed's frame
//! writer's pass meets start late as well: the page's work from the
//! writer's timer to the write its host refuses takes about a ms, so such
//! a hold arms its first poll whole ms past the write's due time, and no
//! earlier than the pass (`crate::net`). With
//! [`Scenario::pacer_pass_start`] too, the pass's writes start late, not
//! only its holds: the page's work from the frame's write to its pass's
//! hand-offs takes about a ms, so the writes the pass hands its data
//! channels start whole ms past the write's due time, and no earlier than
//! the write, as the pass start draws them, and the holds they meet with
//! them, but on a Chromium host without the timer probe, whose held writes
//! draw hold starts of their own ([`held_model`]) and start at the later of
//! the two: on the testbed a pass's hold comes after its writes to the
//! children before it, 0.3 to 0.6 ms past its first.
//!
//! Each host draws from the model of its kind ([`HostKind`]: native,
//! Chromium or Firefox, by the browser its data channels model, as the
//! `hosts` table names it) for each source ([`Source`]): its node task's
//! wake-ups, its harness's timers (the frame writer's waits and the timer
//! probe's), and its host pacer's polls ([`model`]). Each model is the
//! timer bench's: the shares of 0, 1 and 2 ms late, and the measured
//! spread of those 3 ms or more late, that the `timers` table of
//! thin-star testbed runs reads (the farm's `testbed-timers` matrix: a
//! seed and three viewers of one kind on a host of their own, a 10 Mbit/s
//! seed, 90 s, idle and beside a competing upload, every node of the kind,
//! each run weighed alike, its first two repeats). The bench (scoring.sql's
//! `timer_fit_of`) holds a twin of those runs to the testbed's shares,
//! mean and p99, kind by kind and source by source. A Firefox node's model
//! is a page's served cross-origin isolated, as the testbed's timer log
//! serves it, whose timers run less late than a plain page's (PLAN).
//!
//! The bench's runs keep the timer probe on, which runs in the node's own
//! worker, and a browser's writer runs otherwise beside it. A Chromium
//! host without the probe, as a deployed node runs, and as every run runs
//! but the timer bench's twins ([`Scenario::timer_probe`]), draws its frame
//! writer, and its pass starts, from the write path bench's runs instead,
//! which run none: its writes run less late than beside the probe, and its
//! passes start later. With [`Scenario::writer_by_cpu`] such a host's
//! writer on a testbed host (a twin's, [`Scenario::testbed_cpu`]) draws its
//! host's CPU model's, and its writes due on the stream's whole second,
//! where the testbed node's line timer wakes its worker, a model of their
//! own ([`WRITER_BY_CPU`]). A host that
//! runs the probe draws the timer bench's,
//! its pass starts the hold start's. Without the probe a Firefox seed's
//! writes run later (PLAN), which no model has yet: a Firefox host draws the bench's
//! either way. A browser's timer for a deadline already due resolves at
//! once, as strims-wasm's `Sleep` and `setTimeout`-free waits do, drawing
//! nothing; a native node's fires on its next tick, as any. A testbed
//! node's line timer has no draw: the controller samples on time. Without
//! [`Scenario::pacer_lateness`] a browser's host pacer polls on time and
//! draws nothing.
//!
//! The model draws no stalls. In the bench's runs, each on a host of its
//! own, no timer of the stream fires 10 ms late, on any kind, and no
//! native node's 3 ms late, so its models have no tail; a browser's 3+ ms
//! timers are its clamps' few ms. A host with other load stalls timers
//! for tens to hundreds of ms, one node's or every node's at once, which
//! the model leaves out (PLAN).
//!
//! Each host's lateness comes from its own stream (see the crate's
//! determinism notes), which its incarnations' node clocks, its frame
//! writer and its host pacer's polls draw from in turn. A deadline's
//! lateness is drawn once, so a node that resets its timer to the deadline
//! it had keeps it, and passes that change no deadline (a sample's query)
//! draw nothing.

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
    /// models (the window model's, its profile's [`SctpProfile::host`]),
    /// else native.
    ///
    /// [`SctpProfile::host`]: crate::wan::SctpProfile::host
    pub fn of_host(s: &Scenario, h: HostIx) -> HostKind {
        let window = s.send_buffer.is_some_and(|b| b.window);
        s.hosts
            .get(h)
            .and_then(|x| x.rtc_stack.filter(|_| window && x.webrtc))
            .map_or(HostKind::Native, |p| p.host)
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
    /// A browser's host pacer's poll: the sleep of a data channel's writer
    /// that the host's pacer holds, until it reads the host's load again
    /// (strims-wasm's `DcSocket::send`, `crate::net`).
    Pacer,
    /// A browser seed's hold start: the whole ms past its frame writer's
    /// write's due time at which a hold that write's pass meets arms its
    /// first poll, the pass's own if that is later (`crate::net`).
    Hold,
    /// A browser seed's pass start, with
    /// [`Scenario::pacer_pass_start`](crate::Scenario::pacer_pass_start):
    /// the whole ms past its frame writer's write's due time at which the
    /// writes that write's pass hands its data channels begin, and the
    /// holds they meet arm their first polls, at the write if that is
    /// later (`crate::net`).
    Pass,
}

/// A host's lateness stream, shared by its incarnations' clocks: its
/// rng, its kind, whether it runs the timer probe
/// ([`Scenario::timer_probe`](crate::Scenario::timer_probe)), which picks
/// its frame writer's and its pass starts' models ([`model`]), and its
/// frame writer's testbed host's models where it has one ([`writer_model`]).
#[derive(Clone, Debug)]
pub struct Lateness(Arc<Mutex<DetRng>>, HostKind, bool, Option<WriterModel>);

impl Lateness {
    pub(crate) fn new(rng: DetRng, kind: HostKind, probe: bool) -> Lateness {
        Lateness(Arc::new(Mutex::new(rng)), kind, probe, None)
    }

    /// The stream with its frame writer on a testbed host of CPU model
    /// `cpu` ([`Scenario::testbed_cpu`], [`writer_model`]); `None` keeps
    /// [`model`]'s.
    pub(crate) fn on_cpu(mut self, cpu: Option<&str>) -> Lateness {
        self.3 = cpu.and_then(|cpu| writer_model(self.1, self.2, cpu));
        self
    }

    /// How late the host's frame writer's next write runs, armed for a
    /// write `due` already, due on the stream's whole second if
    /// `on_second`: as [`Lateness::draw`] draws [`Source::Writer`], from
    /// its testbed host's models where it has one ([`WriterModel`]).
    pub(crate) fn draw_write(&self, due: bool, on_second: bool) -> Duration {
        let Some(w) = self.3 else {
            return self.draw(Source::Writer, due);
        };
        if due && self.1.resolves_due_at_once() {
            return Duration::ZERO;
        }
        let m = if on_second { w.on_second } else { w.off_second };
        pick(
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .next_u32(),
            m,
        )
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
            self.2,
        )
    }

    /// With the pass start, the host's next held write's own hold start, as
    /// [`held_lateness`] draws it: none on a host whose holds start at their
    /// pass's start.
    pub(crate) fn draw_held(&self) -> Option<Duration> {
        held_lateness(
            &mut self.0.lock().unwrap_or_else(PoisonError::into_inner),
            self.1,
            self.2,
        )
    }

    /// Whether the host's held writes draw hold starts of their own with the
    /// pass start ([`held_model`]).
    pub(crate) fn draws_held(&self) -> bool {
        held_model(self.1, self.2).is_some()
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
/// native node, whose timers there never fired 3 ms late. A host pacer's
/// polls are its seed's, the one node whose writes the bench's hosts held:
/// every poll fires a row, so its shares are the testbed's as they are. In
/// the fit's repeats a Chromium seed's polls never fired 2 ms late (the
/// third repeat of the loaded cell has one of its 2793); 7 of a Firefox
/// seed's 4555 fired 3 ms late, none 4. The polls armed for one deadline
/// share a draw, which leaves each poll's shares the model's. A native
/// node has no host pacer, and its model, on time, is never drawn
/// (`crate::net`).
///
/// A hold start's is the seed's in the bench's idle cells, Chromium's and
/// Firefox's runs together, each run weighed alike (the bench reads no
/// kind apart): for each push period off the whole second (the testbed's
/// line timer shares the second's) whose frame writer ran within its due
/// ms, the whole ms from that ms to the arming of the period's first poll
/// that no other poll's read preceded, the hold the writer's pass met. A
/// writer that runs a ms late ([`Source::Writer`]) meets its hold in its
/// own ms in 91-93 % of such periods (the rest the next), which taking the
/// later of the two gives; one 2 ms late or more in its own ms in 35-38 %
/// (Chromium's 3 of 8, Firefox's 6 of 17), the rest a ms later, which the
/// model leaves out. One period's first hold 18 ms past its writer's, a
/// hold the pass didn't meet, is left out. A native node has no host
/// pacer; its model, at once, is never drawn. A pass start's is the hold
/// start's (the bench's runs log no writes, and a hold arms as the pass's
/// writes begin).
///
/// Those are the models of a host that runs the timer probe (`probe`), as
/// the bench's runs do. A Chromium host without it draws its frame writer
/// and its pass starts from the write path bench's runs (C4), which run
/// none: the testbed's halve-7 cells with SPEC=1 and PCAP=1 (the farm's
/// `testbed-h7spec` matrices, a Chromium seed at 10 and at 20 Mbit/s on
/// c7a, 8 repeats each) and tier r6's Chromium-seed cells halve-7-20 (on
/// c8a), star-15-20 and star-3-10 (PCAP=1), each push period of the
/// stream from 4 to 30 s off the whole second. The writer's is the whole
/// ms from a period's due ms to the queueing of its first push
/// (`seed_sends`), the h7spec cells' 1248 periods: the tail is their 10
/// periods 3 ms late or more, one ms each. The pass start's comes from the
/// ms in which a period's first data packet to a viewer was offered
/// (`packets`, at least 1000 bytes): the cells' mean share in the due ms
/// (each cell weighed alike, 0.302: 0.23 to 0.30 on c7a, 0.40 on c8a) over
/// the writer's on-time share (0.807), so that a pass that starts no
/// earlier than its write offers in its due ms as often, the rest a ms
/// later. The h7spec cells' first hand-off comes 0.67 to 0.88 ms past the
/// due ms in a period that hands off in it, and 1.08 to 1.49 in one that
/// hands off in the next, its first write's begin 0.1 ms after, its first
/// packet's offer 0.1 ms after that, which the model's whole ms leave out.
/// A host without the probe that draws hold starts alone (without
/// [`Scenario::pacer_pass_start`](crate::Scenario::pacer_pass_start))
/// draws the bench's.
///
/// That writer's model is c7a's off the stream's whole second
/// ([`WRITER_BY_CPU`]), which a host draws for every write where it has no
/// testbed host's ([`writer_model`]): a sim of no testbed host, as a
/// deployed node runs, whose writer no line timer wakes.
pub(crate) fn model(kind: HostKind, source: Source, probe: bool) -> Model {
    match (kind, source) {
        (HostKind::Chromium, Source::Writer) if !probe => C7A_WRITER,
        (HostKind::Chromium, Source::Pass) if !probe => Model {
            shares: [374, 626, 0],
            tail: &[],
        },
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
        (HostKind::Native, Source::Pacer) => Model {
            shares: [1000, 0, 0],
            tail: &[],
        },
        (HostKind::Chromium, Source::Pacer) => Model {
            shares: [905, 95, 0],
            tail: &[],
        },
        (HostKind::Firefox, Source::Pacer) => Model {
            shares: [859, 139, 1],
            tail: &[3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3],
        },
        (HostKind::Native, Source::Hold | Source::Pass) => Model {
            shares: [1000, 0, 0],
            tail: &[],
        },
        (HostKind::Chromium | HostKind::Firefox, Source::Hold | Source::Pass) => Model {
            shares: [203, 795, 2],
            tail: &[],
        },
    }
}

/// c7a's probe-off Chromium writer off the stream's whole second
/// ([`WRITER_BY_CPU`]): the write path bench's fit ([`model`]).
const C7A_WRITER: Model = Model {
    shares: [807, 184, 1],
    tail: &[4, 5, 6, 7, 8, 8, 8, 8, 8, 8],
};

/// A probe-off Chromium frame writer's lateness on a testbed host of one
/// CPU model ([`WRITER_BY_CPU`]): its writes due off the stream's whole
/// second, and those due on it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WriterModel {
    pub(crate) off_second: Model,
    pub(crate) on_second: Model,
}

/// A Chromium host's frame writer without the timer probe on a testbed host,
/// with [`Scenario::writer_by_cpu`], by a part of the host's
/// `/proc/cpuinfo` model name (run.sh's `cpu`, a twin's
/// [`Scenario::testbed_cpu`]), as
/// [`CPU_COSTS`](crate::testbed::thread_cost::CPU_COSTS) keys a packet's
/// cost. Each is the whole ms from a push period's due ms (its run's modal
/// ms of the 250 ms period) to the queueing of its first push
/// (`seed_sends`, node 0), periods from 4 to 30 s: c8a's (EPYC 9R45) the 16
/// runs of the farm's `testbed-pph` and `testbed-h7c8` matrices
/// (halve-7-20), 1248 periods off the second and 416 on it; c7a's (EPYC
/// 9R14) the `testbed-h7spec` matrices' 16 (halve-7 at 10 and 20 Mbit/s),
/// as many, off the second [`model`]'s fit. Off the second c8a's writer
/// runs within its due ms more often than c7a's (84.4 % against 80.7 %), but
/// 3 to 8 ms late more often too (2.2 % against 0.8 %). A run's share a ms
/// late spreads about its CPU's as a binomial's of its 78 periods (0.024 to
/// 0.053 across runs), so the host's CPU, not the run, sets it.
///
/// A write due on the whole second shares its ms with the testbed node's
/// line timer (strims-testbed's `node::run`, every second of stream), a
/// `setTimeout` on the same worker armed about a second before, which wakes
/// the worker in that ms; the writer's timer, due by then, runs in that
/// wake. c8a's writer runs within its due ms in 99.0 % of those periods (1.0
/// % a ms late), c7a's in 94.5 % (4.0 % a ms late, 1.2 % 5 to 8 ms), where
/// the twin's writer drew the off-second model's 80 %. The twin's
/// controller samples a host without a timer of its own (the [module
/// docs](self)), so the model takes those periods' measured shares rather
/// than a line timer's lateness and the wake it shares.
///
/// A host of no testbed CPU, or without [`Scenario::writer_by_cpu`], draws
/// [`model`]'s, c7a's off the second, for every write: no line timer runs
/// on a deployed node, and c7a's is the model
/// every sim drew before the CPU keyed it, so a sim of no testbed host keeps
/// its runs; no measure of users' machines says which CPU's is nearer
/// theirs. A testbed host of a CPU model the table lacks (the dev VM's)
/// draws c7a's both.
pub(crate) const WRITER_BY_CPU: &[(&str, WriterModel)] = &[
    (
        "EPYC 9R45",
        WriterModel {
            off_second: Model {
                shares: [844, 130, 4],
                tail: &[
                    3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 6, 6, 7, 7, 8, 8,
                ],
            },
            on_second: Model {
                shares: [990, 10, 0],
                tail: &[],
            },
        },
    ),
    ("EPYC 9R14", C7A),
];

/// c7a's ([`WRITER_BY_CPU`]).
const C7A: WriterModel = WriterModel {
    off_second: C7A_WRITER,
    on_second: Model {
        shares: [945, 40, 3],
        tail: &[5, 6, 7, 7, 8],
    },
};

/// The frame writer's models of a host of `kind`, that runs the timer probe
/// if `probe`, on a testbed host of CPU model `cpu` ([`WRITER_BY_CPU`],
/// c7a's where it lacks the model): a Chromium host's without the probe
/// alone. Every other host's writer draws [`model`]'s.
pub(crate) fn writer_model(kind: HostKind, probe: bool, cpu: &str) -> Option<WriterModel> {
    (kind == HostKind::Chromium && !probe).then(|| {
        WRITER_BY_CPU
            .iter()
            .find(|&&(part, _)| cpu.contains(part))
            .map_or(C7A, |&(_, m)| m)
    })
}

/// With [`Scenario::pacer_pass_start`](crate::Scenario::pacer_pass_start),
/// the model of a held write's own hold start, past its frame writer's
/// write's due time, on a host of `kind` (that runs the timer probe if
/// `probe`) whose holds have one apart from its pass starts: a Chromium
/// host without the probe. Its held write starts at the later of its
/// pass's start and this (`crate::net`). Any other host's pass and hold
/// starts are one model, fit to the hold's own arming, and its holds start
/// at their pass's start, drawing nothing: the later of two draws of that
/// model would start the hold in its due ms in about 4 % of passes, where
/// the model's own share is 20.3 %.
///
/// The model is the testbed's pph runs' (the farm's `testbed-pph` matrix:
/// halve-7-20 on c8a, a Chromium seed, SPEC=1 and PCAP=1, 8 repeats). In
/// their push periods from 4 to 30 s off the whole second whose writer ran
/// within its due ms, the seed's pushes held 2 to 8 ms (`seed_sends`,
/// queued to written) are written 5 ms after their queueing in 3.9 %, 6 ms
/// in 87.8 % and 7 ms in 7.3 %. A push written at 5 ms is one whose pass
/// started in its due ms (the pass start's 0.374), whose hold started there
/// too and whose poll fired on time (the host pacer's 0.905): the share in
/// the due ms is 3.9 % over those two, 11.5 %, the rest the next ms. In the
/// loaded phase (31 to 70 s, the seed's uplink halved) the testbed writes
/// 24 % of such pushes at 5 ms, which the model leaves out.
pub(crate) fn held_model(kind: HostKind, probe: bool) -> Option<Model> {
    (kind == HostKind::Chromium && !probe).then_some(Model {
        shares: [115, 885, 0],
        tail: &[],
    })
}

/// How late a timer of `source` fires on a host of `kind`, that runs the
/// timer probe if `probe`: one draw of `rng` ([`pick`]).
pub(crate) fn lateness(rng: &mut DetRng, kind: HostKind, source: Source, probe: bool) -> Duration {
    pick(rng.next_u32(), model(kind, source, probe))
}

/// With the pass start, how late a held write's own hold start is on a host
/// of `kind`, that runs the timer probe if `probe`: one draw of `rng` at
/// [`held_model`]; none on a host it has none for, drawing nothing.
pub(crate) fn held_lateness(rng: &mut DetRng, kind: HostKind, probe: bool) -> Option<Duration> {
    let m = held_model(kind, probe)?;
    Some(pick(rng.next_u32(), m))
}

/// The lateness that a draw `x` picks of `m`: its remainder by [`PARTS`]
/// picks the bucket at its [`Model::shares`], and its quotient the tail's
/// value.
fn pick(x: u32, m: Model) -> Duration {
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
    use crate::wan::SctpProfile;
    use strims_base::rng::det_rng;

    const KINDS: [HostKind; 3] = [HostKind::Native, HostKind::Chromium, HostKind::Firefox];
    const SOURCES: [Source; 5] = [
        Source::NodeTask,
        Source::Writer,
        Source::Pacer,
        Source::Hold,
        Source::Pass,
    ];

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

    /// Each kind's and source's draws, with the timer probe and without,
    /// come at its model's shares, and its 3+ ms draws are its tail's
    /// values, every one drawn; a model has a tail exactly when its shares
    /// leave parts to it; the most any tail holds is MAX_LATE_MS.
    #[test]
    fn a_timer_fires_at_its_kinds_and_sources_model() {
        let mut most = 0;
        for kind in KINDS {
            for source in SOURCES {
                for probe in [false, true] {
                    let m = model(kind, source, probe);
                    let held = m.shares.iter().sum::<u32>();
                    let at = format!("{kind:?} {source:?} {probe}");
                    assert!(held <= PARTS, "{at}");
                    assert_eq!(m.tail.is_empty(), held == PARTS, "{at}");
                    assert!(m.tail.is_sorted() && m.tail.iter().all(|&ms| ms >= 3));
                    most = most.max(m.tail.last().map_or(0, |&ms| i64::from(ms)));
                    let mut rng = det_rng(7);
                    let mut seen = std::collections::BTreeSet::new();
                    let b = buckets(100_000, || {
                        let d = lateness(&mut rng, kind, source, probe);
                        if d.as_millis() >= 3 {
                            assert!(m.tail.contains(&(d.as_millis() as u32)));
                            seen.insert(d.as_millis());
                        }
                        d
                    });
                    assert!(at_shares(b, m), "{at}: {b:?}");
                    let distinct: std::collections::BTreeSet<u32> =
                        m.tail.iter().copied().collect();
                    assert_eq!(seen.len(), distinct.len(), "{at}: every tail value drawn");
                }
            }
        }
        assert_eq!(most, MAX_LATE_MS);
    }

    /// No two kinds and sources share a model: each is its own pool, but
    /// the hold start's, one for both browsers (and never drawn on a
    /// native node), which is the pass start's too but for a Chromium host
    /// without the probe. The probe picks a model of its own for a Chromium
    /// host's frame writer and pass starts alone.
    #[test]
    fn every_kind_and_source_has_a_model_of_its_own() {
        let same = |a: Model, b: Model| a.shares == b.shares && a.tail == b.tail;
        let mut all: Vec<(HostKind, Source, bool)> = KINDS
            .iter()
            .flat_map(|&k| SOURCES.iter().map(move |&s| (k, s, true)))
            .filter(|&(k, s, _)| s != Source::Hold || k == HostKind::Chromium)
            .filter(|&(_, s, _)| s != Source::Pass)
            .collect();
        for k in KINDS {
            for s in SOURCES {
                if k == HostKind::Chromium && matches!(s, Source::Writer | Source::Pass) {
                    all.push((k, s, false));
                } else {
                    assert!(same(model(k, s, false), model(k, s, true)), "{k:?} {s:?}");
                }
            }
            assert!(same(
                model(k, Source::Pass, true),
                model(k, Source::Hold, true)
            ));
        }
        assert!(same(
            model(HostKind::Chromium, Source::Hold, true),
            model(HostKind::Firefox, Source::Hold, true)
        ));
        for (i, &(ka, sa, pa)) in all.iter().enumerate() {
            for &(kb, sb, pb) in &all[i + 1..] {
                assert!(
                    !same(model(ka, sa, pa), model(kb, sb, pb)),
                    "{ka:?} {sa:?} {pa} and {kb:?} {sb:?} {pb}"
                );
            }
        }
    }

    /// With the pass start, a Chromium host without the probe alone draws
    /// its held writes' own hold starts, at their model's shares, a model
    /// none of its sources share; every other host draws none, and no draw
    /// takes its rng.
    #[test]
    fn a_chromium_host_without_the_probe_alone_draws_held_starts() {
        for kind in KINDS {
            for probe in [false, true] {
                let late = Lateness::new(det_rng(7), kind, probe);
                let Some(m) = held_model(kind, probe) else {
                    assert!(!late.draws_held() && late.draw_held().is_none());
                    assert!(kind != HostKind::Chromium || probe, "{kind:?}");
                    assert_eq!(late.draw(Source::Pass, false), {
                        let mut rng = det_rng(7);
                        lateness(&mut rng, kind, Source::Pass, probe)
                    });
                    continue;
                };
                assert!(kind == HostKind::Chromium && !probe && late.draws_held());
                for s in SOURCES {
                    assert_ne!(m.shares, model(kind, s, probe).shares, "{s:?}");
                }
                let b = buckets(100_000, || late.draw_held().unwrap_or(Duration::MAX));
                assert!(at_shares(b, m), "{b:?}");
            }
        }
    }

    /// A probe-off Chromium host on a testbed CPU draws its writes off the
    /// whole second and on it at that CPU's models, a CPU the table lacks
    /// at c7a's, whose writer off the second is the write path bench's;
    /// a write already due draws nothing. Without a CPU, with the probe or
    /// on another kind, a write draws as `Source::Writer` does, from the
    /// same rng.
    #[test]
    fn a_testbed_hosts_writer_draws_its_cpus_models() {
        for &(part, w) in WRITER_BY_CPU {
            for m in [w.off_second, w.on_second] {
                let held = m.shares.iter().sum::<u32>();
                assert!(held <= PARTS, "{part}");
                assert_eq!(m.tail.is_empty(), held == PARTS, "{part}");
                assert!(m.tail.is_sorted() && m.tail.iter().all(|&ms| ms >= 3));
                assert!(m.tail.iter().all(|&ms| i64::from(ms) <= MAX_LATE_MS));
            }
            let cpu = format!("AMD {part} 96-Core Processor");
            for (on, m) in [(false, w.off_second), (true, w.on_second)] {
                let late = Lateness::new(det_rng(7), HostKind::Chromium, false).on_cpu(Some(&cpu));
                let b = buckets(100_000, || late.draw_write(false, on));
                assert!(at_shares(b, m), "{part} {on}: {b:?}");
                assert_eq!(late.draw_write(true, on), Duration::ZERO);
            }
        }
        let shares = |w: WriterModel| (w.off_second.shares, w.on_second.shares);
        let c7a = writer_model(HostKind::Chromium, false, "AMD EPYC 9R14").map(shares);
        let dev = "AMD Ryzen Threadripper PRO 5955WX 16-Cores";
        assert_eq!(
            writer_model(HostKind::Chromium, false, dev).map(shares),
            c7a
        );
        assert_eq!(
            c7a.map(|(off, _)| off),
            Some(model(HostKind::Chromium, Source::Writer, false).shares)
        );
        let c8a = Some("AMD EPYC 9R45");
        for (kind, probe, cpu) in [
            (HostKind::Chromium, false, None),
            (HostKind::Chromium, true, c8a),
            (HostKind::Firefox, false, c8a),
            (HostKind::Native, false, c8a),
        ] {
            let late = Lateness::new(det_rng(3), kind, probe).on_cpu(cpu);
            let mut fresh = det_rng(3);
            for on in [false, true, false] {
                assert_eq!(
                    late.draw_write(false, on),
                    lateness(&mut fresh, kind, Source::Writer, probe),
                    "{kind:?} {probe} {cpu:?}"
                );
            }
        }
    }

    /// A browser's timer for a deadline already due fires at once and
    /// draws nothing; a native node's draws as any.
    #[test]
    fn a_browsers_due_timer_fires_at_once() {
        for kind in [HostKind::Chromium, HostKind::Firefox] {
            let (stream, mut fresh) = (Lateness::new(det_rng(5), kind, false), det_rng(5));
            for source in SOURCES {
                assert_eq!(stream.draw(source, true), Duration::ZERO);
            }
            assert_eq!(
                stream.draw(Source::Writer, false),
                lateness(&mut fresh, kind, Source::Writer, false),
                "{kind:?}: the due ones drew nothing"
            );
        }
        let (stream, mut fresh) = (
            Lateness::new(det_rng(5), HostKind::Native, false),
            det_rng(5),
        );
        assert_eq!(
            stream.draw(Source::NodeTask, true),
            lateness(&mut fresh, HostKind::Native, Source::NodeTask, false)
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
        let stream = Lateness::new(det_rng(3), HostKind::Native, false);
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
        let draw = |rng: &mut DetRng| lateness(rng, HostKind::Native, Source::NodeTask, false);
        for _ in 0..200 {
            let _ = draw(&mut fresh);
        }
        let again = LateClock::new(inner, stream);
        let t = epoch + Duration::from_secs(5000);
        assert_eq!(again.deadline(t), inner.deadline(t + draw(&mut fresh)));
        // Over many deadlines the clock's draws are the node task's.
        let clock = LateClock::new(inner, Lateness::new(det_rng(9), HostKind::Native, false));
        let mut k = 0;
        let b = buckets(100_000, || {
            k += 1;
            let t = epoch + Duration::from_secs(10_000) + Duration::from_millis(10 * k);
            let base = inner.deadline(t);
            Duration::from(clock.deadline(t) - base)
        });
        assert!(
            at_shares(b, model(HostKind::Native, Source::NodeTask, false)),
            "{b:?}"
        );
        assert!(
            !at_shares(b, model(HostKind::Native, Source::Writer, false)),
            "{b:?}"
        );
    }

    /// Lateness draws from the run's rng last of the scenario's draws but
    /// the epoch phase: a trickle's arrivals, the dishonest mix and the
    /// clock offsets come out as they do without it, and then its streams
    /// (a host's each, and with the probe a probe's each) fork the rng the
    /// controller's choices take, which off it doesn't touch; every host's
    /// stream is its kind's.
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
