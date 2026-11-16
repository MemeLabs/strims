// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A testbed run's tables: `hack/testbed/run.sh`'s out dir read as a run's
//! report ([`strims_sim::testbed`]) and written as a sim run writes its
//! tables, with a `job.json` for `hack/sim-db`'s fold.
//!
//! ```text
//! testbed_tables <out dir> <tables dir> [--knob KEY=VALUE ...]
//! ```
//!
//! `job.json` holds the run's key (`k`), its sim (`testbed`), its test
//! (`calibration_star`), its twin's knobs (`env`: [`knobs_of`] and the
//! `--knob`s, which add to them or replace them) and its row
//! ([`strims_sim::testbed::row`]).
//!
//! [`knobs_of`]: strims_sim::testbed::knobs_of

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use strims_sim::testbed::{self, Run};

fn main() {
    if let Err(e) = run() {
        eprintln!("testbed_tables: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    // The command line names the run.
    #[allow(clippy::disallowed_methods)]
    let mut args = std::env::args().skip(1);
    let usage = "usage: testbed_tables <out dir> <tables dir> [--knob KEY=VALUE ...]";
    let dir = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().ok_or(usage)?);
    let mut extra = BTreeMap::new();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--knob" => {
                let kv = args.next().ok_or("--knob KEY=VALUE")?;
                let (k, v) = kv.split_once('=').ok_or("--knob KEY=VALUE")?;
                extra.insert(k.to_owned(), v.to_owned());
            }
            _ => return Err(usage.to_owned()),
        }
    }
    let run = Run::read(&dir, &extra)?;
    let report = run.report();
    let bin = Duration::from_millis(strims_testbed::record::BIN_MS.unsigned_abs());
    report
        .write_tables(&out, bin, true)
        .map_err(|e| format!("{}: {e}", out.display()))?;
    let job = testbed::job(&run, &report);
    std::fs::write(out.join("job.json"), format!("{job}\n"))
        .map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{}", out.display());
    Ok(())
}
