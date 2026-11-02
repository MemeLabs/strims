// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! One node of the TCP bench ([`strims_sim::bench`]) on real sockets and
//! the wall clock, for the calibration testbed (`hack/testbed/bench.sh`),
//! or the same bench on the sim's sockets.
//!
//! ```text
//! testbed_bench --index <i> --topology <file> --schedule <file>
//!               [--secs <s>] [--start <unix ms>] [--webrtc]
//!               [--web <dir>] [--browser-bin <path>]
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
//! A browser node (its kind in the topology, `firefox` or `chromium`;
//! `run.sh`'s `KINDS`) runs the bench in a headless browser, over its
//! data channels, so it needs `--webrtc`. The harness makes the node's TCP
//! links as a native node does, serves `--web`'s [`PAGE`] (the bench page,
//! `hack/testbed/bench/page`) on 127.0.0.1, opens it in the browser
//! (`firefox` or `chromium` on `PATH`, unless `--browser-bin` says) and
//! carries each link's offer and answer between the page and the link,
//! in the same order and format as a native node's (the dialer offers).
//! The page writes and reads the records, its log lines come to stdout
//! as a native node's, and its control protocol is in its own comment.
//!
//! `--sim` runs the bench a testbed run made (its `settings`, `topology`
//! and `schedule` files) under the WAN model, with the send buffers'
//! window model if `--window`, and writes each node's log to
//! `<out>/node-<i>.jsonl`.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::cell::RefCell;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use strims_node::{TokioNet, WebRtcConfig};
use strims_sim::bench::{
    Bench, BenchClock, Entry, LINGER, NodePlan, RtcPlan, Schedule, links, node, read_signal,
    simulate, write_signal,
};
use strims_testbed::browser::{Bundle, Site, Socket, args as browser_args, firefox_profile};
use strims_testbed::native::now_ms;
use strims_testbed::topology::{Kind, Topology};
use tokio_tungstenite::tungstenite::Message;

/// The bench page, below `--web`.
const PAGE: &str = "bench.html";
/// How long the browser has to open the page's control socket.
const PAGE_TIMEOUT: Duration = Duration::from_secs(60);

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
    web: Option<PathBuf>,
    browser_bin: Option<PathBuf>,
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
            "--web" => a.web = Some(value()?.into()),
            "--browser-bin" => a.browser_bin = Some(value()?.into()),
            "--join-at" | "--rate" | "--hosts" | "--capture" | "--units" | "--player" => {
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
    let res = if this.kind.is_browser() {
        if !a.webrtc {
            return Err("a browser node's links need --webrtc".to_owned());
        }
        browser(a, this.kind, plan, clock, log).await
    } else {
        node(TokioNet, clock, plan, log)
            .await
            .map_err(|e| e.to_string())
    };
    let _ = out.borrow_mut().flush();
    res
}

/// The browser, killed when this drops.
struct Browser(Child);

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Copies `pipe`'s lines to stderr, on a thread.
fn relay_output(pipe: impl Read + Send + 'static) {
    // A blocking pipe, read until the browser closes it.
    #[allow(clippy::disallowed_methods)]
    std::thread::spawn(move || {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            eprintln!("browser: {line}");
        }
    });
}

/// The bench page's control socket; its log lines go to `log`.
struct Page<L> {
    ws: Socket,
    log: L,
}

impl<L: Fn(String)> Page<L> {
    async fn send(&mut self, v: Value) -> Result<(), String> {
        self.ws
            .send(Message::text(v.to_string()))
            .await
            .map_err(|e| format!("page: {e}"))
    }

    /// The page's next event, after the log lines and errors before it.
    async fn next(&mut self) -> Result<Value, String> {
        loop {
            let msg = self
                .ws
                .next()
                .await
                .ok_or("the page closed its control socket")?
                .map_err(|e| format!("page: {e}"))?;
            let Message::Text(text) = msg else {
                continue;
            };
            let v: Value = serde_json::from_str(&text).map_err(|e| format!("page: {e}"))?;
            match v.get("type").and_then(Value::as_str) {
                Some("log") => {
                    let lines = v.get("lines").and_then(Value::as_array);
                    for line in lines.into_iter().flatten().filter_map(Value::as_str) {
                        (self.log)(line.to_owned());
                    }
                }
                Some("error") => eprintln!("page: {}", v.get("message").unwrap_or(&Value::Null)),
                _ => return Ok(v),
            }
        }
    }

    /// Waits for the page's `kind` event about `peer`.
    async fn wait(&mut self, kind: &str, peer: usize) -> Result<Value, String> {
        loop {
            let v = self.next().await?;
            if v.get("type").and_then(Value::as_str) == Some(kind)
                && v.get("peer").and_then(Value::as_u64) == Some(peer as u64)
            {
                return Ok(v);
            }
            eprintln!("page: {v} while waiting for {kind} from {peer}");
        }
    }

    /// The description in the page's `local` event about `peer`.
    async fn local(&mut self, peer: usize) -> Result<String, String> {
        let v = self.wait("local", peer).await?;
        v.get("description")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("no description for {peer}"))
    }
}

fn entry_json(e: &Entry) -> Value {
    match *e {
        Entry::Burst {
            to,
            start_ms,
            period_ms,
            bytes,
            write,
            end_ms,
            ..
        } => json!({
            "kind": "burst", "to": to, "startMs": start_ms, "periodMs": period_ms,
            "bytes": bytes, "write": write, "endMs": end_ms,
        }),
        Entry::Probe {
            to,
            start_ms,
            period_ms,
            end_ms,
            ..
        } => json!({
            "kind": "probe", "to": to, "startMs": start_ms, "periodMs": period_ms,
            "endMs": end_ms,
        }),
    }
}

/// Runs browser node `plan.me` (see the module docs).
async fn browser(
    a: &Args,
    kind: Kind,
    plan: NodePlan,
    clock: Wall,
    log: impl Fn(String),
) -> Result<(), String> {
    let web = a.web.clone().ok_or("a browser node needs --web")?;
    let bundle = Bundle::read(&web).map_err(|e| format!("{}: {e}", web.display()))?;
    if !bundle.0.contains_key(PAGE) {
        return Err(format!("no {PAGE} in {}", web.display()));
    }
    let mut site = Site::serve(Arc::new(bundle))
        .await
        .map_err(|e| format!("serve: {e}"))?;
    let profile = std::env::temp_dir().join(format!("testbed-bench-{}", plan.me));
    let _ = std::fs::remove_dir_all(&profile);
    if kind == Kind::Firefox {
        firefox_profile(&profile).map_err(|e| format!("profile: {e}"))?;
    }
    let bin = a.browser_bin.clone().unwrap_or_else(|| {
        match kind {
            Kind::Chromium => "chromium",
            _ => "firefox",
        }
        .into()
    });
    let url = format!("http://{}/{PAGE}", site.addr);
    let mut child = Command::new(&bin)
        .args(browser_args(kind, &profile, &url))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{}: {e}", bin.display()))?;
    if let Some(out) = child.stdout.take() {
        relay_output(out);
    }
    if let Some(err) = child.stderr.take() {
        relay_output(err);
    }
    let _browser = Browser(child);
    let ws = loop {
        match tokio::time::timeout(PAGE_TIMEOUT, site.sockets.recv()).await {
            Ok(Some((path, ws))) if path == "/control?i=0" => break ws,
            Ok(Some((path, _))) => eprintln!("an unexpected socket at {path}"),
            Ok(None) | Err(_) => return Err("the page didn't open its control socket".to_owned()),
        }
    };
    let mut page = Page { ws, log };
    let stop = plan.end_us + i64::try_from(LINGER.as_micros()).unwrap_or(i64::MAX);
    let entries: Vec<Value> = plan
        .schedule
        .entries
        .iter()
        .filter(|e| matches!(e, Entry::Burst { from, .. } | Entry::Probe { from, .. } if *from == plan.me))
        .map(entry_json)
        .collect();
    page.send(json!({
        "type": "setup",
        "startMs": clock.start_ms,
        "endUs": plan.end_us,
        "stopUs": stop,
        "cap": WebRtcConfig::default().max_buffered_amount,
        "entries": entries,
    }))
    .await?;
    let streams = links(&TokioNet, &clock, &plan)
        .await
        .map_err(|e| e.to_string())?;
    let io = |e: std::io::Error| e.to_string();
    for (peer, mut s) in streams {
        if peer < plan.me {
            page.send(json!({"type": "link", "peer": peer, "offer": true}))
                .await?;
            let offer = page.local(peer).await?;
            write_signal(&mut s, &offer).await.map_err(io)?;
            let answer = read_signal(&mut s).await.map_err(io)?;
            page.send(json!({"type": "remote", "peer": peer, "description": answer}))
                .await?;
        } else {
            let offer = read_signal(&mut s).await.map_err(io)?;
            page.send(json!({"type": "link", "peer": peer, "offer": false, "description": offer}))
                .await?;
            let answer = page.local(peer).await?;
            write_signal(&mut s, &answer).await.map_err(io)?;
        }
        page.wait("open", peer).await?;
    }
    page.send(json!({"type": "go"})).await?;
    let left = Duration::from_micros((stop - clock.now_us()).max(0).unsigned_abs());
    let done = async {
        loop {
            let v = page.next().await?;
            if v.get("type").and_then(Value::as_str) == Some("done") {
                return Ok::<_, String>(());
            }
        }
    };
    let res = tokio::time::timeout(left + Duration::from_secs(10), done)
        .await
        .map_err(|_| "the page didn't end".to_owned())
        .and_then(|r| r);
    let _ = std::fs::remove_dir_all(&profile);
    res
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
