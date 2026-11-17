// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A testbed run's tables: `hack/testbed/run.sh`'s out dir read as a run's
//! report ([`strims_sim::testbed`]) and written as a sim run writes its
//! tables, with a `job.json` for `hack/sim-db`'s fold.
//!
//! ```text
//! testbed_tables <out dir> <tables dir> [--knob KEY=VALUE ...]
//! testbed_tables --knobs <out dir>
//! testbed_tables --uplink <out dir> <node>
//! ```
//!
//! `job.json` holds the run's key (`k`), its sim (`testbed`), its test
//! (`calibration_star`), its twin's knobs (`env`: [`knobs_of`] and the
//! `--knob`s, which add to them or replace them) and its row
//! ([`strims_sim::testbed::row`]). `--knobs` prints the knobs alone, from
//! the out dir's settings, topology and links (no node lines), as JSON.
//! `--uplink` prints, as JSON, the qdisc tree that shaped the run
//! (`qdisc`, [`qdisc_tree`]; `null` where its files don't say) and node
//! `<node>`'s access link queue as the tables read it ([`uplink`]): its
//! `spec`, its queue's `kind` and `dev`, and its `samples`, `[unix ms,
//! drops, held, rate or null]` each; or where the reader refuses its log
//! (a log without its class's queue), the refusal as `error` in their
//! place. It reads any out dir run.sh wrote, a bench's (bench.sh's) too,
//! which has no tables, and refuses a dir without run.sh's `settings`.
//!
//! [`knobs_of`]: strims_sim::testbed::knobs_of
//! [`qdisc_tree`]: strims_sim::testbed::qdisc_tree
//! [`uplink`]: strims_sim::testbed::uplink

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use strims_sim::testbed::{self, Run, knobs_of, settings_of};

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
    let usage = "usage: testbed_tables <out dir> <tables dir> [--knob KEY=VALUE ...] | --knobs <out dir> | --uplink <out dir> <node>";
    let first = args.next().ok_or(usage)?;
    if first == "--uplink" {
        let dir = PathBuf::from(args.next().ok_or(usage)?);
        // A run without its files would read as one without samples.
        if !dir.join("settings").exists() {
            return Err(format!(
                "{}: no run.sh out dir (no settings)",
                dir.display()
            ));
        }
        let node: usize = args
            .next()
            .ok_or(usage)?
            .parse()
            .map_err(|e| format!("<node>: {e}"))?;
        let mut out = serde_json::Map::new();
        out.insert("qdisc".to_owned(), testbed::qdisc_tree(&dir).ok().into());
        match testbed::uplink(&dir, node) {
            Ok(u) => {
                let samples: Vec<serde_json::Value> = u
                    .samples
                    .iter()
                    .map(|s| serde_json::json!([s.ms, s.drops, s.held, s.rate]))
                    .collect();
                out.insert("spec".to_owned(), u.spec.into());
                out.insert("kind".to_owned(), u.kind.into());
                out.insert("dev".to_owned(), u.dev.into());
                out.insert("samples".to_owned(), samples.into());
            }
            Err(e) => {
                out.insert("error".to_owned(), e.into());
            }
        }
        println!("{}", serde_json::Value::Object(out));
        return Ok(());
    }
    if first == "--knobs" {
        let dir = PathBuf::from(args.next().ok_or(usage)?);
        let read = |name: &str| {
            std::fs::read_to_string(dir.join(name)).map_err(|e| format!("{name}: {e}"))
        };
        let links = std::fs::read_to_string(dir.join("links")).ok();
        let knobs = knobs_of(
            &settings_of(&read("settings")?),
            &read("topology")?,
            links.as_deref(),
            &dir,
        )?;
        println!(
            "{}",
            serde_json::to_string(&knobs).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let dir = PathBuf::from(first);
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
