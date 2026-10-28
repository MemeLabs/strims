// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Seed sweeps, replay commands and the determinism check.
//!
//! - `STRIMS_SIM_SEEDS=<n>` runs seeds `1..=n` (default [`DEFAULT_SEEDS`],
//!   or [`DEFAULT_SEEDS_DEBUG`] in a debug build, so the plain
//!   `cargo test` stays short), and with `STRIMS_SIM_SEED_FROM=<k>` the
//!   `n` seeds from `k`.
//! - `STRIMS_SIM_SEED=<s>` runs only seed `s` (the replay of a failure).
//! - `STRIMS_SIM_TRACE=1` prints every run's trace.
//! - `STRIMS_SIM_CAPTURE=<dir>` captures every node's ppspp traffic
//!   ([`Scenario::capture`]) into `<dir>/<test>/seed-<s>/`, and the replay's
//!   into `seed-<s>-replay/`.
//! - `STRIMS_SIM_SEND_BUFFER=<ms>` gives every WAN scenario send buffers
//!   ([`Scenario::send_buffer`]) with an uplink queue of `ms` milliseconds,
//!   replacing the scenario's own, for comparisons.
//! - `STRIMS_SIM_TCP=1` gives the send buffers congestion windows and
//!   bounded access queues ([`SendBuffer::window`]).
//! - `STRIMS_SIM_TCP_SEGMENT=<bytes>` splits TCP writes into segments of at
//!   most that many bytes ([`Scenario::tcp_segment`]), so the window model
//!   fills windows as real TCP does (1448, a segment's payload).
//! - `STRIMS_SIM_KNOWN_UPLINK=1` tells every WAN host its site's uplink
//!   ([`HostSpec::upload_capacity`](crate::HostSpec::upload_capacity)): the
//!   oracle the upload meter is measured against. `=seed` tells only the
//!   hosts that seed a swarm, `=viewers` only the others.
//! - `STRIMS_SIM_MINUTES=<m>` sets the stream length of the scenarios that
//!   take one ([`stream_minutes`]).
//! - `STRIMS_SIM_CHUNK=<bytes>` and `STRIMS_SIM_MUNRO=<bytes>` give every
//!   swarm chunks and signatures of those sizes
//!   ([`with_chunks`](crate::SwarmSpec::with_chunks)): a media swarm's 4096
//!   and 32768 against the scenarios' 1 KiB frames. Either alone keeps the
//!   other's bytes.
//! - `STRIMS_SIM_STREAM_SECS=<s>` streams every swarm for `s` seconds from
//!   its start, the run ending 10 s after the last stops: the WAN cases'
//!   minute, longer. A scenario's faults, steps and competing uploads keep
//!   their times.
//! - `STRIMS_SIM_SERIES=1` samples every node's transfers
//!   ([`Scenario::transfer_samples`]), so the row carries the run second by
//!   second ([`Report::series`]).
//! - `STRIMS_SIM_ROWS=1` prints each run's measures as one line,
//!   `ROW {json}`, on stdout ([`Report::row`]).
//! - `STRIMS_SIM_JOB=1` runs the sweep as one sim farm job
//!   (`hack/sim-farm`): the one seed `STRIMS_SIM_SEED` names, once, and its
//!   row. The farm checks that runs reproduce, so the seed isn't replayed
//!   here. A failed check goes into the row (`passed`, `failure`) and to
//!   stderr, and doesn't fail the test: a trial counts it.
//!
//! Outside a farm job, the first seed runs a second time and must give the
//! same trace hash.
//!
//! A sweep starts every run at once, the second run of the first seed
//! included, each on a thread of its own. Runs on separate threads are
//! independent: turmoil keeps a simulation's world in thread-local state
//! and runs all of its hosts on the thread that steps it, and [`run`] seeds
//! str0m's thread-local rng and counts the heap of that thread only. Once
//! every run has ended, the reports are printed and checked in seed order,
//! so a failing sweep reports its lowest failing seed.

use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::thread;
use std::time::Duration;

use crate::report::Report;
use crate::run::run;
use crate::spec::{HostIx, Scenario, SendBuffer};

/// Seeds per sweep when `STRIMS_SIM_SEEDS` isn't set.
pub const DEFAULT_SEEDS: u64 = 8;
/// The same in a debug build.
pub const DEFAULT_SEEDS_DEBUG: u64 = 2;

/// Reads a run setting from the environment. Only the test run's
/// configuration comes from here, never the simulated nodes'.
#[allow(clippy::disallowed_methods)]
fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// The seeds a sweep runs (see the module docs).
pub fn seeds() -> Vec<u64> {
    seeds_or(if cfg!(debug_assertions) {
        DEFAULT_SEEDS_DEBUG
    } else {
        DEFAULT_SEEDS
    })
}

/// [`seeds`], with `n` seeds when `STRIMS_SIM_SEEDS` isn't set.
fn seeds_or(n: u64) -> Vec<u64> {
    if let Some(s) = env("STRIMS_SIM_SEED").and_then(|s| s.trim().parse().ok()) {
        return vec![s];
    }
    let n = env("STRIMS_SIM_SEEDS")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(n)
        .max(1);
    let from: u64 = env("STRIMS_SIM_SEED_FROM")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(1);
    (from..from.saturating_add(n)).collect()
}

fn replay(test: &str, seed: u64) -> String {
    format!(
        "replay: STRIMS_SIM_SEED={seed} cargo test -p strims-sim --release --test scenarios {test} -- --exact --nocapture --include-ignored"
    )
}

#[allow(clippy::panic)] // a failed sweep fails its test
fn fail(test: &str, seed: u64, msg: &str, trace: &[String]) -> ! {
    let tail = trace.len().saturating_sub(60);
    for l in trace.get(tail..).unwrap_or_default() {
        eprintln!("  | {l}");
    }
    panic!(
        "{test} failed with seed {seed}: {msg}\n{}",
        replay(test, seed)
    );
}

/// How a run ended: its report, turmoil's error, or its panic.
type Outcome = thread::Result<Result<Report, String>>;

/// The report of a run that ended well. A run that failed fails the sweep
/// with its seed; a panic resumes on the calling thread.
fn report_of(test: &str, seed: u64, outcome: Outcome) -> Report {
    match outcome {
        Ok(Ok(r)) => {
            if env("STRIMS_SIM_TRACE").is_some_and(|v| v != "0") {
                for l in &r.trace {
                    eprintln!("  | {l}");
                }
            }
            r
        }
        Ok(Err(e)) => fail(test, seed, &format!("simulation failed: {e}"), &[]),
        Err(p) => {
            eprintln!("{test} panicked with seed {seed}\n{}", replay(test, seed));
            resume_unwind(p)
        }
    }
}

/// A run on a thread of its own, or one whose thread couldn't start, which
/// then runs on the calling thread when joined.
enum Run<'scope> {
    Started(thread::ScopedJoinHandle<'scope, Result<Report, String>>),
    Unstarted(u64),
}

impl Run<'_> {
    /// The run's outcome once it has ended. Every run is joined, so a run's
    /// panic reaches its outcome and never fails the scope.
    fn join(self, build: &impl Fn(u64) -> Scenario) -> Outcome {
        match self {
            Run::Started(handle) => handle.join(),
            Run::Unstarted(seed) => catch_unwind(AssertUnwindSafe(|| run(build(seed), seed))),
        }
    }
}

/// A scenario setting from the run's environment, if set and not blank.
/// A value that doesn't parse fails the test, so a mistyped setting can't
/// run the default instead.
#[allow(clippy::panic)] // a mistyped setting fails its test
pub fn knob<T: std::str::FromStr>(name: &str) -> Option<T> {
    let v = env(name)?;
    let v = v.trim();
    if v.is_empty() {
        return None;
    }
    match v.parse() {
        Ok(x) => Some(x),
        Err(_) => panic!("{name}={v} doesn't parse"),
    }
}

/// The stream length of a scenario that takes it from the run's settings:
/// `STRIMS_SIM_MINUTES=<m>`, else `default` minutes.
pub fn stream_minutes(default: u64) -> u64 {
    env("STRIMS_SIM_MINUTES")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(default)
        .max(1)
}

/// Runs scenario `build(seed)` for every seed of [`seeds`] and the first
/// seed a second time, all at once, then checks each report with `check`
/// and compares the first seed's trace hashes. A failure panics with the
/// seed and a one-line replay command. `test` is the test function's name
/// (for the replay command and the runs' thread names).
pub fn sweep(
    test: &str,
    build: impl Fn(u64) -> Scenario + Sync,
    check: impl Fn(&Report) -> Result<(), String>,
) -> Vec<Report> {
    sweep_seeds(test, seeds(), build, check)
}

/// [`sweep`] over `n` seeds unless `STRIMS_SIM_SEEDS` says otherwise, for
/// scenarios too long for the default count.
pub fn sweep_n(
    test: &str,
    n: u64,
    build: impl Fn(u64) -> Scenario + Sync,
    check: impl Fn(&Report) -> Result<(), String>,
) -> Vec<Report> {
    sweep_seeds(test, seeds_or(n), build, check)
}

fn sweep_seeds(
    test: &str,
    seeds: Vec<u64>,
    build: impl Fn(u64) -> Scenario + Sync,
    check: impl Fn(&Report) -> Result<(), String>,
) -> Vec<Report> {
    let capture = env("STRIMS_SIM_CAPTURE");
    let send_buffer = env("STRIMS_SIM_SEND_BUFFER").and_then(|s| s.trim().parse::<u64>().ok());
    let known_uplink = env("STRIMS_SIM_KNOWN_UPLINK").map(|s| s.trim().to_owned());
    let rows = env("STRIMS_SIM_ROWS").is_some_and(|s| s.trim() == "1");
    let job = env("STRIMS_SIM_JOB").is_some_and(|s| s.trim() == "1");
    assert!(
        !job || seeds.len() == 1,
        "STRIMS_SIM_JOB runs one seed: set STRIMS_SIM_SEED"
    );
    let window = env("STRIMS_SIM_TCP").is_some_and(|s| s.trim() == "1");
    let series = env("STRIMS_SIM_SERIES").is_some_and(|s| s.trim() == "1");
    let chunk: Option<usize> = knob("STRIMS_SIM_CHUNK");
    let munro: Option<usize> = knob("STRIMS_SIM_MUNRO");
    let stream_secs: Option<u64> = knob("STRIMS_SIM_STREAM_SECS");
    let segment: Option<usize> = knob("STRIMS_SIM_TCP_SEGMENT");
    let with_capture = |seed: u64, replay: &str| {
        let mut spec = build(seed);
        if let Some(ms) = send_buffer
            && spec.wan.is_some()
        {
            spec.send_buffer = Some(SendBuffer {
                uplink_queue: Duration::from_millis(ms),
                window: false,
            });
        }
        if window && let Some(b) = &mut spec.send_buffer {
            b.window = true;
        }
        if series {
            spec.transfer_samples = true;
        }
        if segment.is_some() {
            spec.tcp_segment = segment;
        }
        if let Some(secs) = stream_secs {
            for s in &mut spec.swarms {
                s.stop = s.start + Duration::from_secs(secs);
            }
            if let Some(stop) = spec.swarms.iter().map(|s| s.stop).max() {
                spec.duration = stop + Duration::from_secs(10);
            }
        }
        if chunk.is_some() || munro.is_some() {
            spec.swarms = std::mem::take(&mut spec.swarms)
                .into_iter()
                .map(|s| {
                    let c = chunk.unwrap_or(s.options.chunk_size);
                    let m = munro.unwrap_or(s.options.chunk_size * s.options.chunks_per_signature);
                    s.with_chunks(c, m)
                        .unwrap_or_else(|e| fail(test, seed, &e, &[]))
                })
                .collect();
        }
        if let Some(which) = known_uplink.as_deref() {
            let seeds: Vec<HostIx> = spec.swarms.iter().map(|s| s.seed).collect();
            for (i, h) in spec.hosts.iter_mut().enumerate() {
                let seed = seeds.contains(&i);
                let told = match which {
                    "1" => true,
                    "seed" => seed,
                    "viewers" => !seed,
                    _ => false,
                };
                if told && let Some(site) = h.site {
                    h.upload_capacity = Some(site.profile.up);
                }
            }
        }
        if let Some(dir) = &capture {
            let run = format!("seed-{seed}{replay}");
            spec.capture = Some(std::path::Path::new(dir).join(test).join(run));
        }
        spec
    };
    let first = |seed: u64| with_capture(seed, "");
    let replay = |seed: u64| with_capture(seed, "-replay");
    let (first, replay) = (&first, &replay);
    let (outcomes, again) = thread::scope(|s| {
        let start = |seed: u64, name: String, again: bool| match thread::Builder::new()
            .name(name)
            .spawn_scoped(s, move || {
                let spec = if again { replay(seed) } else { first(seed) };
                run(spec, seed)
            }) {
            Ok(handle) => Run::Started(handle),
            Err(_) => Run::Unstarted(seed),
        };
        let runs: Vec<Run<'_>> = seeds
            .iter()
            .map(|&seed| start(seed, format!("{test} seed {seed}"), false))
            .collect();
        let again = seeds
            .first()
            .filter(|_| !job)
            .map(|&seed| start(seed, format!("{test} replay"), true));
        let outcomes: Vec<Outcome> = runs.into_iter().map(|r| r.join(first)).collect();
        (outcomes, again.map(|r| r.join(replay)))
    });

    let mut reports = Vec::new();
    let mut verdicts = Vec::new();
    for (&seed, outcome) in seeds.iter().zip(outcomes) {
        let r = report_of(test, seed, outcome);
        eprintln!("{}", r.summary());
        let verdict = check(&r);
        if rows || job {
            println!(
                "ROW {}",
                r.row(test, verdict.as_ref().err().map(String::as_str))
            );
        }
        reports.push(r);
        verdicts.push(verdict);
    }
    for (r, verdict) in reports.iter().zip(verdicts) {
        if let Err(e) = verdict {
            if job {
                eprintln!(
                    "{test} failed with seed {}: {e}\n{}",
                    r.seed,
                    self::replay(test, r.seed)
                );
            } else {
                fail(test, r.seed, &e, &r.trace);
            }
        }
    }
    if let (Some(first), Some(again)) = (reports.first(), again) {
        let again = report_of(test, first.seed, again);
        if again.trace_hash != first.trace_hash {
            let at = first
                .trace
                .iter()
                .zip(&again.trace)
                .position(|(a, b)| a != b)
                .unwrap_or(first.trace.len().min(again.trace.len()));
            let msg = format!(
                "not deterministic: trace hashes {} and {} differ at line {at}: {:?} vs {:?}",
                first.trace_hash,
                again.trace_hash,
                first.trace.get(at),
                again.trace.get(at)
            );
            fail(test, first.seed, &msg, &[]);
        }
        eprintln!(
            "{test}: seed {} replayed: same trace hash {} ({} lines)",
            first.seed,
            first.trace_hash,
            first.trace.len()
        );
    }
    reports
}
