// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! One node of the TCP bench ([`strims_sim::bench`]) on real sockets and
//! the wall clock, for the calibration testbed (`hack/testbed/bench.sh`),
//! or the same bench on the sim's sockets.
//!
//! ```text
//! testbed_bench --index <i> --topology <file> --schedule <file>
//!               [--secs <s>] [--start <unix ms>] [--webrtc]
//! testbed_bench --sim <testbed run dir> --out <dir> [--window]
//!               [--segment <bytes>]
//! ```
//!
//! On the testbed a node listens on its topology address, dials the nodes
//! it links to with a lower index, runs the schedule from `--start` (all
//! nodes share the clock) for `--secs` and prints its log on stdout,
//! after a `{"ev":"start","ms":..}` line with the start in Unix ms. With
//! `--webrtc` its links carry the records over data channels from a UDP
//! port, its TCP port's next. It takes `run.sh`'s other flags and ignores
//! them.
//!
//! `--sim` runs the bench a testbed run made (its `settings`, `topology`
//! and `schedule` files) under the WAN model, with the send buffers'
//! window model if `--window`, and writes each node's log to
//! `<out>/node-<i>.jsonl`.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::cell::RefCell;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::rc::Rc;

use strims_node::TokioNet;
use strims_sim::bench::{Bench, BenchClock, NodePlan, RtcPlan, Schedule, node, simulate};
use strims_testbed::native::now_ms;
use strims_testbed::topology::Topology;

#[derive(Default)]
struct Args {
    index: Option<usize>,
    topology: String,
    schedule: String,
    secs: u64,
    start_ms: Option<u64>,
    sim: Option<PathBuf>,
    out: Option<PathBuf>,
    window: bool,
    segment: Option<usize>,
    webrtc: bool,
}

fn args() -> Result<Args, String> {
    // The bench's settings come from its command line.
    #[allow(clippy::disallowed_methods)]
    let mut it = std::env::args().skip(1);
    let mut a = Args {
        secs: 180,
        ..Args::default()
    };
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        let num = |v: String| v.parse::<u64>().map_err(|e| format!("{flag}: {e}"));
        match flag.as_str() {
            "--index" => a.index = Some(num(value()?)? as usize),
            "--topology" => a.topology = value()?,
            "--schedule" => a.schedule = value()?,
            "--secs" => a.secs = num(value()?)?,
            "--start" => a.start_ms = Some(num(value()?)?),
            "--sim" => a.sim = Some(value()?.into()),
            "--out" => a.out = Some(value()?.into()),
            "--window" => a.window = true,
            "--segment" => a.segment = Some(num(value()?)? as usize),
            "--webrtc" => a.webrtc = true,
            "--join-at" | "--rate" | "--hosts" | "--capture" => {
                value()?;
            }
            _ => return Err(format!("unknown flag {flag}")),
        }
    }
    Ok(a)
}

/// The wall clock, from the testbed's start.
#[derive(Clone, Copy)]
struct Wall {
    start_ms: u64,
}

impl BenchClock for Wall {
    fn now_us(&self) -> i64 {
        // The testbed's clock: its containers share the host's.
        #[allow(clippy::disallowed_methods)]
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        i64::try_from(now.as_micros()).unwrap_or(i64::MAX)
            - i64::try_from(self.start_ms).unwrap_or(0) * 1000
    }
}

fn read(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn sim(dir: &std::path::Path, a: &Args) -> Result<(), String> {
    let out = a.out.clone().ok_or("--sim needs --out")?;
    let mut bench = Bench::testbed(
        &read(&dir.join("settings"))?,
        &read(&dir.join("topology"))?,
        &read(&dir.join("schedule"))?,
        a.window,
    )?;
    bench.tcp_segment = a.segment;
    let logs = simulate(&bench)?;
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    for (i, lines) in logs.iter().enumerate() {
        let mut text = lines.join("\n");
        text.push('\n');
        let path = out.join(format!("node-{i}.jsonl"));
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

async fn testbed(a: &Args) -> Result<(), String> {
    let me = a.index.ok_or("--index is required")?;
    let topo = Topology::parse(&read(a.topology.as_ref())?)?;
    let schedule = Schedule::parse(&read(a.schedule.as_ref())?)?;
    let this = topo.node(me).ok_or(format!("no node {me}"))?;
    let addr = |i: usize| topo.node(i).map(|n| n.addr.to_string()).unwrap_or_default();
    let mut plan = NodePlan::new(
        me,
        &topo.links,
        this.addr.to_string(),
        addr,
        schedule,
        i64::try_from(a.secs).unwrap_or(0) * 1_000_000,
    );
    if a.webrtc {
        // testbed_node's UDP port: its TCP port's next.
        plan.webrtc = Some(RtcPlan {
            udp: format!("0.0.0.0:{}", this.addr.port() + 1),
            ip: this.addr.ip(),
        });
    }
    let clock = Wall {
        start_ms: a.start_ms.unwrap_or_else(|| now_ms() + 2000),
    };
    // Buffered, so the log's writes stay off the links' timing.
    let out = Rc::new(RefCell::new(BufWriter::with_capacity(
        1 << 20,
        std::io::stdout(),
    )));
    let o = out.clone();
    let log = move |line: String| {
        let _ = writeln!(o.borrow_mut(), "{line}");
    };
    log(format!(r#"{{"ev":"start","ms":{}}}"#, clock.start_ms));
    let res = node(TokioNet, clock, plan, log).await;
    let _ = out.borrow_mut().flush();
    res.map_err(|e| e.to_string())
}

fn main() {
    // The sim runs its own runtimes.
    let res = args().and_then(|a| match &a.sim {
        Some(dir) => sim(dir, &a),
        None => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())
            .and_then(|rt| rt.block_on(testbed(&a))),
    });
    if let Err(e) = res {
        eprintln!("testbed_bench: {e}");
        std::process::exit(1);
    }
}
