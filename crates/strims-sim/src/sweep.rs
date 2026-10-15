// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Seed sweeps, replay commands and the determinism check.
//!
//! - `STRIMS_SIM_SEEDS=<n>` runs seeds `1..=n` (default [`DEFAULT_SEEDS`],
//!   or [`DEFAULT_SEEDS_DEBUG`] in a debug build, so the plain
//!   `cargo test` stays short).
//! - `STRIMS_SIM_SEED=<s>` runs only seed `s` (the replay of a failure).
//! - `STRIMS_SIM_TRACE=1` prints every run's trace.
//!
//! After the sweep the first seed runs again and must give the same trace
//! hash.

use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

use crate::report::Report;
use crate::run::run;
use crate::spec::Scenario;

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
    if let Some(s) = env("STRIMS_SIM_SEED").and_then(|s| s.trim().parse().ok()) {
        return vec![s];
    }
    let n = env("STRIMS_SIM_SEEDS")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(if cfg!(debug_assertions) {
            DEFAULT_SEEDS_DEBUG
        } else {
            DEFAULT_SEEDS
        })
        .max(1);
    (1..=n).collect()
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

fn run_one(test: &str, build: &impl Fn(u64) -> Scenario, seed: u64) -> Report {
    let result = catch_unwind(AssertUnwindSafe(|| run(build(seed), seed)));
    match result {
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

/// Runs scenario `build(seed)` for every seed of [`seeds`], checks each
/// report with `check`, then replays the first seed and compares trace
/// hashes. A failure panics with the seed and a one-line replay command.
/// `test` is the test function's name (for the replay command).
pub fn sweep(
    test: &str,
    build: impl Fn(u64) -> Scenario,
    check: impl Fn(&Report) -> Result<(), String>,
) -> Vec<Report> {
    let mut reports = Vec::new();
    for seed in seeds() {
        let r = run_one(test, &build, seed);
        eprintln!("{}", r.summary());
        if let Err(e) = check(&r) {
            fail(test, seed, &e, &r.trace);
        }
        reports.push(r);
    }
    if let Some(first) = reports.first() {
        let again = run_one(test, &build, first.seed);
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
