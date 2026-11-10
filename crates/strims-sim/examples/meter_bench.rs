// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The upload meter bench ([`strims_sim::bench::meter`]) in the sim.
//!
//! ```text
//! meter_bench sim --out <dir> [--classes a,b|all] [--seeds 1-8]
//!                 [--transports tcp,chromium,firefox,str0m] [--have off,on]
//!                 [--drivers main,oracle] [--meters main,oracle,...]
//!                 [--threads n] [--record] [--force]
//! meter_bench plan --class <c> --seed <s> [--transport t] [--have]
//! ```
//!
//! `sim` runs every class, seed, transport, traffic mix and driving meter
//! into `<out>/<class>-<transport>-<have>/s<seed>/<driver>/`: each node's
//! log as `node-<i>.jsonl` and the condition as `condition.json`. Every
//! run's meters are its driver and then `--meters` (shadows, read on the
//! same observations). A run whose directory holds its logs is kept unless
//! `--force`. `plan` prints a condition.

#![allow(clippy::print_stdout, clippy::print_stderr)]
// One run a thread: each sim is its own, on its thread.
#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use strims_sim::bench::meter::plan::{Class, Condition, Transport};
use strims_sim::bench::meter::simulate;

struct Job {
    class: Class,
    seed: u64,
    transport: Transport,
    have: bool,
    driver: String,
    dir: PathBuf,
}

fn list(s: &str) -> Vec<String> {
    s.split(',')
        .filter(|x| !x.is_empty())
        .map(str::to_owned)
        .collect()
}

fn seeds(s: &str) -> Result<Vec<u64>, String> {
    let mut out = Vec::new();
    for part in s.split(',') {
        match part.split_once('-') {
            Some((a, b)) => {
                let (a, b): (u64, u64) = (
                    a.parse().map_err(|e| format!("{part}: {e}"))?,
                    b.parse().map_err(|e| format!("{part}: {e}"))?,
                );
                out.extend(a..=b);
            }
            None => out.push(part.parse().map_err(|e| format!("{part}: {e}"))?),
        }
    }
    Ok(out)
}

fn run(job: &Job, meters: &[String], record: bool) -> Result<(), String> {
    let cond = Condition::draw(job.class, job.seed, job.transport, job.have);
    let mut all = vec![job.driver.clone()];
    all.extend(meters.iter().filter(|m| **m != job.driver).cloned());
    let logs = simulate(&cond, &all, record)?;
    std::fs::create_dir_all(&job.dir).map_err(|e| e.to_string())?;
    std::fs::write(job.dir.join("condition.json"), cond.json()).map_err(|e| e.to_string())?;
    for (i, l) in logs.iter().enumerate() {
        let mut text = l.join("\n");
        text.push('\n');
        std::fs::write(job.dir.join(format!("node-{i}.jsonl")), text).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn done(dir: &Path) -> bool {
    dir.join("condition.json").exists() && dir.join("node-0.jsonl").exists()
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let has = |name: &str| args.iter().any(|a| a == name);
    match args.first().map(String::as_str) {
        Some("plan") => {
            let class = Class::parse(&flag("--class").ok_or("--class")?)?;
            let seed: u64 = flag("--seed")
                .ok_or("--seed")?
                .parse()
                .map_err(|e| format!("{e}"))?;
            let t = Transport::parse(&flag("--transport").unwrap_or_else(|| "tcp".to_owned()))?;
            println!("{}", Condition::draw(class, seed, t, has("--have")).json());
            Ok(())
        }
        Some("sim") => {
            let out = PathBuf::from(flag("--out").ok_or("--out")?);
            let classes: Vec<Class> = match flag("--classes").as_deref() {
                None | Some("all") => Class::ALL.to_vec(),
                Some(s) => list(s)
                    .iter()
                    .map(|c| Class::parse(c))
                    .collect::<Result<_, _>>()?,
            };
            let seed_list = seeds(&flag("--seeds").unwrap_or_else(|| "1-8".to_owned()))?;
            let transports: Vec<Transport> =
                list(&flag("--transports").unwrap_or_else(|| "tcp".to_owned()))
                    .iter()
                    .map(|t| Transport::parse(t))
                    .collect::<Result<_, _>>()?;
            let haves: Vec<bool> = list(&flag("--have").unwrap_or_else(|| "off".to_owned()))
                .iter()
                .map(|h| match h.as_str() {
                    "on" => Ok(true),
                    "off" => Ok(false),
                    _ => Err(format!("--have {h}")),
                })
                .collect::<Result<_, _>>()?;
            let drivers = list(&flag("--drivers").unwrap_or_else(|| "main".to_owned()));
            let meters = list(&flag("--meters").unwrap_or_else(|| "main,oracle".to_owned()));
            let threads: usize = flag("--threads")
                .map(|t| t.parse().unwrap_or(1))
                .unwrap_or_else(|| {
                    std::thread::available_parallelism()
                        .map_or(4, |n| n.get().saturating_sub(2).max(1))
                });
            let record = has("--record");
            let force = has("--force");
            let mut jobs = Vec::new();
            for &class in &classes {
                for &transport in &transports {
                    for &have in &haves {
                        for &seed in &seed_list {
                            for driver in &drivers {
                                let dir = out
                                    .join(format!(
                                        "{}-{}-{}",
                                        class,
                                        transport.name(),
                                        if have { "have" } else { "state" }
                                    ))
                                    .join(format!("s{seed}"))
                                    .join(driver);
                                if !force && done(&dir) {
                                    continue;
                                }
                                jobs.push(Job {
                                    class,
                                    seed,
                                    transport,
                                    have,
                                    driver: driver.clone(),
                                    dir,
                                });
                            }
                        }
                    }
                }
            }
            let total = jobs.len();
            eprintln!("{total} runs on {threads} threads");
            let queue = Mutex::new(jobs);
            let failed = Mutex::new(Vec::new());
            std::thread::scope(|s| {
                for _ in 0..threads {
                    s.spawn(|| {
                        loop {
                            let job = queue.lock().ok().and_then(|mut q| q.pop());
                            let Some(job) = job else {
                                break;
                            };
                            if let Err(e) = run(&job, &meters, record)
                                && let Ok(mut f) = failed.lock()
                            {
                                f.push(format!("{}: {e}", job.dir.display()));
                            }
                        }
                    });
                }
            });
            let failed = failed.into_inner().unwrap_or_default();
            for f in &failed {
                eprintln!("failed {f}");
            }
            eprintln!("{} of {total} runs done", total - failed.len());
            if failed.is_empty() {
                Ok(())
            } else {
                Err(format!("{} runs failed", failed.len()))
            }
        }
        _ => Err("usage: meter_bench sim|plan ...".to_owned()),
    }
}
