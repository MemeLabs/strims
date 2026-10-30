// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! One browser node of the calibration testbed (`hack/testbed/`): a
//! headless Firefox or Chromium running strims-wasm's node, driven like
//! strims-sim's native `testbed_node`. It runs in the node's container.
//!
//! ```text
//! testbed_browser --index <i> --topology <file> --web <dir>
//!                 [--browser-bin <path>] [--secs <s>] [--start <unix ms>]
//!                 [--rate <bytes/s>] [--units <chunks per signature>]
//!                 [--join-at <s>] [--hosts <dir>] [--capture <dir>]
//!                 [--player <file>]
//! ```
//!
//! The node's kind (`firefox` or `chromium`) comes from the topology
//! ([`strims_testbed::topology`]). The harness serves the bundle in `--web`
//! (`npx webpack --mode production --env TARGET=wasm-testbed`) on
//! 127.0.0.1, and opens its `e2e.html` in the browser (`firefox`, or
//! `chromium`, on `PATH` unless `--browser-bin` says), so the page's
//! control and bundle traffic stays on loopback and the container's only
//! other interface, the shaped one, carries every link. Over the page's
//! control socket (the protocol of strims-wasm's `e2e` module) it:
//!
//! 1. starts the node with node `<i>`'s profile key, in the testbed
//!    network, and has it run the testbed's swarm (`testbed`);
//! 2. dials the relay over WebSocket, and once the network knows the link
//!    (1 s), writes the node's host id into `--hosts`;
//! 3. at `--join-at` seconds after the start, makes the node's links
//!    ([`Topology::plan`]): WebSocket dials, and peer exchanges once the
//!    other node's host id is in `--hosts`, each retried until it links.
//!
//! The node's line per second goes to stdout as the page sends it, with
//! `cpu_ms` the CPU time of the browser and every process below it,
//! `post_ms` the page's clock as it posted the line, and `host_t_ms` the
//! harness's clock when the line arrived: the page's clock and the
//! container's agree when `host_t_ms` stays a few ms past `post_ms`. Everything else (the browser's output, the node's errors and
//! peers) goes to stderr. The harness ends 5 s after the stream does,
//! like `testbed_node`, and kills the browser.
//!
//! With `--units`, the stream is media in the unit format
//! ([`strims_testbed::media`]): the page plays what the node reads through
//! the baseline player, and its `player` events, a JSON object a second of
//! the player's stats and the video's events, go to `--player`, one line
//! each with `rel_ms` (the page's `postMs` less the start) and `host_t_ms`
//! added.
//!
//! With `--capture`, the node captures its ppspp traffic as a native
//! node's `--capture` does, and the harness writes what the page sends of
//! it to `<dir>/<host id hex>-<n>.ppcap`. It reads on for
//! [`CAPTURE_GRACE`] after the last line, so the capture's last flush
//! arrives before the browser is killed.

#![allow(clippy::print_stdout, clippy::print_stderr)]
// Indexing a `serde_json::Value` by key gives `Null` for a missing key.
#![allow(clippy::indexing_slicing)]

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use strims_testbed::browser::{Bundle, Site, args as browser_args, firefox_profile};
use strims_testbed::native::{now_ms, read_host, tree_cpu_ms, write_host};
use strims_testbed::shape::RATE;
use strims_testbed::topology::{BIND_PORT, Kind, Link, Topology, key_of, network_key};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::Message;

/// How long the page has to load and compile the wasm module.
const PAGE_TIMEOUT: Duration = Duration::from_secs(120);
/// How long a command's answer may take.
const STEP: Duration = Duration::from_secs(30);
/// The e2e node's vpn ports, which the testbed doesn't use.
const ECHO_PORT: u16 = 20_000;
const REPLY_PORT: u16 = 20_001;
/// How long the harness reads the capture after the last line: two of the
/// node's flush intervals.
const CAPTURE_GRACE: Duration = Duration::from_secs(2);
/// A capture file's size before the next, as `testbed_node`'s.
const CAPTURE_FILE_BYTES: u64 = 512 << 20;
/// The first byte of a message of capture bytes (strims-wasm's `e2e`).
const CAPTURE_BYTES: u8 = 0x01;

struct Args {
    index: usize,
    topology: String,
    web: PathBuf,
    browser_bin: Option<PathBuf>,
    secs: u64,
    start_ms: Option<u64>,
    rate: u64,
    join_at: u64,
    hosts: Option<PathBuf>,
    capture: Option<PathBuf>,
    units: Option<u64>,
    player: Option<PathBuf>,
}

fn args() -> Result<Args, String> {
    // The testbed's settings come from its command line.
    #[allow(clippy::disallowed_methods)]
    let mut it = std::env::args().skip(1);
    let mut a = Args {
        index: usize::MAX,
        topology: String::new(),
        web: PathBuf::new(),
        browser_bin: None,
        secs: 180,
        start_ms: None,
        rate: RATE,
        join_at: 0,
        hosts: None,
        capture: None,
        units: None,
        player: None,
    };
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        let num = |v: String| v.parse::<u64>().map_err(|e| format!("{flag}: {e}"));
        match flag.as_str() {
            "--index" => a.index = num(value()?)? as usize,
            "--topology" => a.topology = value()?,
            "--web" => a.web = value()?.into(),
            "--browser-bin" => a.browser_bin = Some(value()?.into()),
            "--secs" => a.secs = num(value()?)?,
            "--start" => a.start_ms = Some(num(value()?)?),
            "--rate" => a.rate = num(value()?)?,
            "--join-at" => a.join_at = num(value()?)?,
            "--hosts" => a.hosts = Some(value()?.into()),
            "--capture" => a.capture = Some(value()?.into()),
            "--units" => a.units = Some(num(value()?)?),
            "--player" => a.player = Some(value()?.into()),
            _ => return Err(format!("unknown flag {flag}")),
        }
    }
    if a.index == usize::MAX || a.topology.is_empty() || a.web.as_os_str().is_empty() {
        return Err("--index, --topology and --web are required".to_owned());
    }
    Ok(a)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("testbed_browser: {e}");
        std::process::exit(1);
    }
}

/// The browser, killed when this drops.
struct Browser(Child);

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The page's control socket: commands out, events to every subscriber.
#[derive(Clone)]
struct Control {
    commands: mpsc::UnboundedSender<Value>,
    events: broadcast::Sender<Value>,
}

impl Control {
    /// Sends `cmd` and waits for the first event `f` accepts; `None` on a
    /// timeout or an `error` about the command.
    async fn ask(&self, cmd: Value, f: impl Fn(&Value) -> bool) -> Option<Value> {
        let mut events = self.events.subscribe();
        let kind = cmd["type"].clone();
        let _ = self.commands.send(cmd);
        let deadline = tokio::time::Instant::now() + STEP;
        loop {
            match tokio::time::timeout_at(deadline, events.recv()).await {
                Ok(Ok(e)) if f(&e) => return Some(e),
                Ok(Ok(e)) if e["type"] == "error" && e["command"] == kind => return None,
                Ok(Ok(_) | Err(broadcast::error::RecvError::Lagged(_))) => {}
                Ok(Err(broadcast::error::RecvError::Closed)) | Err(_) => return None,
            }
        }
    }

    /// Dials `uri` until it links.
    async fn dial(&self, uri: &str) {
        loop {
            let cmd = json!({"type": "dial", "uri": uri});
            if self
                .ask(cmd, |e| e["type"] == "dialed" && e["uri"] == uri)
                .await
                .is_some()
            {
                return;
            }
            eprintln!("dial {uri} failed; again");
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Exchanges with `host` through the relay until it links.
    async fn exchange(&self, host: &str) {
        loop {
            let mut events = self.events.subscribe();
            let cmd = json!({"type": "connectPeer", "hostId": host});
            let started = self
                .ask(cmd, |e| e["type"] == "connecting" && e["hostId"] == host)
                .await;
            // Not started: the host is already a peer, or the exchange
            // that will link it runs.
            if started.is_some_and(|e| e["started"] == false) {
                return;
            }
            let deadline = tokio::time::Instant::now() + STEP;
            loop {
                match tokio::time::timeout_at(deadline, events.recv()).await {
                    Ok(Ok(e)) if e["type"] == "peerAdded" && e["hostId"] == host => return,
                    Ok(Ok(e)) if e["type"] == "exchangeClosed" && e["hostId"] == host => {
                        if e["outcome"] == "Connected" {
                            return;
                        }
                        eprintln!("exchange with {host}: {}", e["outcome"]);
                        break;
                    }
                    Ok(Ok(_) | Err(broadcast::error::RecvError::Lagged(_))) => {}
                    Ok(Err(broadcast::error::RecvError::Closed)) | Err(_) => break,
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

/// A capture's files as the page sends them.
struct CaptureFiles {
    dir: PathBuf,
    /// By file index. Bound: the capture's files.
    files: BTreeMap<u64, BufWriter<File>>,
}

impl CaptureFiles {
    fn new(dir: PathBuf) -> Result<CaptureFiles, String> {
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(CaptureFiles {
            dir,
            files: BTreeMap::new(),
        })
    }

    /// `captureOpen`: file `file` of the node named `node` starts.
    fn open(&mut self, node: &str, file: u64) {
        let path = self.dir.join(format!("{node}-{file}.ppcap"));
        match File::create(&path) {
            Ok(f) => {
                self.files.insert(file, BufWriter::new(f));
            }
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
    }

    /// A message of capture bytes: the file's index, then its bytes.
    fn write(&mut self, msg: &[u8]) {
        let mut file = 0u64;
        let mut at = 1;
        for (i, &b) in msg.iter().enumerate().skip(1).take(10) {
            file |= u64::from(b & 0x7f) << (7 * (i - 1));
            at = i + 1;
            if b < 0x80 {
                break;
            }
        }
        let Some(f) = self.files.get_mut(&file) else {
            eprintln!("capture bytes for file {file}, which didn't open");
            return;
        };
        if let Err(e) = f.write_all(msg.get(at..).unwrap_or_default()) {
            eprintln!("capture file {file}: {e}");
        }
    }

    fn flush(&mut self) {
        for (i, f) in &mut self.files {
            if let Err(e) = f.flush() {
                eprintln!("capture file {i}: {e}");
            }
        }
    }
}

/// Copies `pipe`'s lines to stderr, on a thread.
fn relay_output(pipe: impl Read + Send + 'static, tag: &'static str) {
    // A blocking pipe, read until the browser closes it.
    #[allow(clippy::disallowed_methods)]
    std::thread::spawn(move || {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            eprintln!("{tag}: {line}");
        }
    });
}

async fn run() -> Result<(), String> {
    let a = args()?;
    let text = std::fs::read_to_string(&a.topology).map_err(|e| format!("{}: {e}", a.topology))?;
    let topo = Topology::parse(&text)?;
    let me = topo
        .node(a.index)
        .ok_or("this node isn't in the topology")?
        .clone();
    if !me.kind.is_browser() {
        return Err(format!("node {} isn't a browser", a.index));
    }
    let start_ms = a.start_ms.unwrap_or_else(|| now_ms() + 2000);
    let end_ms = start_ms + a.secs * 1000;

    let bundle = Bundle::read(&a.web).map_err(|e| format!("{}: {e}", a.web.display()))?;
    let mut site = Site::serve(Arc::new(bundle))
        .await
        .map_err(|e| format!("serve: {e}"))?;
    let profile = std::env::temp_dir().join(format!("testbed-browser-{}", a.index));
    let _ = std::fs::remove_dir_all(&profile);
    if me.kind == Kind::Firefox {
        firefox_profile(&profile).map_err(|e| format!("profile: {e}"))?;
    }
    let bin = a.browser_bin.clone().unwrap_or_else(|| {
        match me.kind {
            Kind::Chromium => "chromium",
            _ => "firefox",
        }
        .into()
    });
    let url = format!("http://{}/e2e.html?count=1&logLevel=0", site.addr);
    let mut child = Command::new(&bin)
        .args(browser_args(me.kind, &profile, &url))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{}: {e}", bin.display()))?;
    let pid = child.id();
    if let Some(out) = child.stdout.take() {
        relay_output(out, "browser");
    }
    if let Some(err) = child.stderr.take() {
        relay_output(err, "browser");
    }
    let _browser = Browser(child);
    eprintln!("{} opened {url} at {}", bin.display(), now_ms());

    let ws = loop {
        match tokio::time::timeout(PAGE_TIMEOUT, site.sockets.recv()).await {
            Ok(Some((path, ws))) if path == "/control?i=0" => break ws,
            Ok(Some((path, _))) => eprintln!("an unexpected socket at {path}"),
            Ok(None) | Err(_) => return Err("the page didn't open its control socket".to_owned()),
        }
    };
    let mut capture = a.capture.clone().map(CaptureFiles::new).transpose()?;
    let mut player = match &a.player {
        Some(p) => Some(BufWriter::new(
            File::create(p).map_err(|e| format!("{}: {e}", p.display()))?,
        )),
        None => None,
    };
    let (mut sink, mut stream) = ws.split();
    let (commands, mut queued) = mpsc::unbounded_channel::<Value>();
    let (events, _) = broadcast::channel(1024);
    let control = Control { commands, events };
    let send = async move {
        while let Some(cmd) = queued.recv().await {
            if sink
                .send(Message::binary(cmd.to_string().into_bytes()))
                .await
                .is_err()
            {
                return;
            }
        }
    };
    // Lines, events and capture bytes, until the line 5 s after the
    // stream's end, and with a capture, a grace after it.
    let last_rel = (a.secs * 1000 + 5000) as i64;
    let receive = {
        let events = control.events.clone();
        let capture = &mut capture;
        let player = &mut player;
        async move {
            let mut until = None;
            loop {
                let next = match until {
                    Some(t) => match tokio::time::timeout_at(t, stream.next()).await {
                        Ok(m) => m,
                        Err(_) => return,
                    },
                    None => stream.next().await,
                };
                let Some(Ok(msg)) = next else {
                    break;
                };
                let Message::Binary(b) = msg else {
                    continue;
                };
                if b.first() == Some(&CAPTURE_BYTES) {
                    if let Some(c) = capture.as_mut() {
                        c.write(&b);
                    }
                    continue;
                }
                let Ok(e) = serde_json::from_slice::<Value>(&b) else {
                    eprintln!("an event that isn't JSON");
                    continue;
                };
                if e["type"] == "sample" {
                    let Some(line) = e["line"].as_str() else {
                        continue;
                    };
                    let t = now_ms();
                    let cpu = format!("\"cpu_ms\":{}", tree_cpu_ms(pid));
                    let line = line.replacen("\"cpu_ms\":null", &cpu, 1);
                    let post = e["postMs"]
                        .as_u64()
                        .map_or("null".to_owned(), |p| p.to_string());
                    if let Some(body) = line.strip_suffix('}') {
                        println!("{body},\"post_ms\":{post},\"host_t_ms\":{t}}}");
                    }
                    let rel = serde_json::from_str::<Value>(&line)
                        .ok()
                        .and_then(|v| v["rel_ms"].as_i64());
                    if rel.is_some_and(|r| r >= last_rel) && until.is_none() {
                        if capture.is_none() {
                            return;
                        }
                        until = Some(tokio::time::Instant::now() + CAPTURE_GRACE);
                    }
                    continue;
                }
                if e["type"] == "player" {
                    if let (Some(f), Value::Object(mut o)) = (player.as_mut(), e) {
                        let t = now_ms();
                        let rel = o
                            .get("postMs")
                            .and_then(Value::as_u64)
                            .map(|p| p as i64 - start_ms as i64);
                        o.insert("index".to_owned(), json!(a.index));
                        o.insert("rel_ms".to_owned(), json!(rel));
                        o.insert("host_t_ms".to_owned(), json!(t));
                        if let Err(e) = writeln!(f, "{}", Value::Object(o)) {
                            eprintln!("player file: {e}");
                        }
                    }
                    continue;
                }
                if e["type"] == "captureOpen" {
                    if let (Some(c), Some(node), Some(file)) =
                        (capture.as_mut(), e["node"].as_str(), e["file"].as_u64())
                    {
                        c.open(node, file);
                    }
                    continue;
                }
                if e["type"] != "message" {
                    eprintln!("{} {e}", now_ms());
                }
                let _ = events.send(e);
            }
            eprintln!("the page closed its control socket");
        }
    };
    let drive = async {
        let key = key_of(a.index);
        let mut start = json!({
            "type": "start",
            "profileKey": {
                "type": key.r#type,
                "private": hex::encode(&key.private),
                "public": hex::encode(&key.public),
            },
            "rngSeed": a.index + 1,
            "networkKey": hex::encode(network_key()),
            "bindPort": BIND_PORT,
            "echoPort": ECHO_PORT,
            "replyPort": REPLY_PORT,
        });
        if a.capture.is_some() {
            start["capture"] = json!({
                "maxFileBytes": CAPTURE_FILE_BYTES,
                "tap": true,
                "label": format!("testbed-{}", a.index),
            });
        }
        let Some(started) = control.ask(start, |e| e["type"] == "started").await else {
            eprintln!("the node didn't start");
            return;
        };
        let mut testbed = json!({
            "type": "testbed",
            "index": a.index,
            "nodes": topo.nodes.len(),
            "startMs": start_ms,
            "secs": a.secs,
            "rate": a.rate,
        });
        if let Some(cps) = a.units {
            testbed["units"] = json!(cps);
        }
        let _ = control.commands.send(testbed);
        if let Some(uri) = topo.relay_uri(a.index) {
            control.dial(&uri).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
            let host = started["hostId"]
                .as_str()
                .and_then(|h| hex::decode(h).ok())
                .and_then(|b| <[u8; 32]>::try_from(b).ok());
            if let (Some(dir), Some(b)) = (&a.hosts, host) {
                let id = strims_node_core::HostId::from_bytes(&b);
                if let Err(e) = write_host(dir, a.index, id) {
                    eprintln!("host file: {e}");
                }
            }
        }
        let join = start_ms + a.join_at * 1000;
        tokio::time::sleep(Duration::from_millis(join.saturating_sub(now_ms()))).await;
        for link in topo.plan(a.index) {
            match link {
                Link::Dial(uri) => control.dial(&uri).await,
                Link::Exchange(j) => {
                    let Some(dir) = &a.hosts else {
                        eprintln!("an exchange with node {j} needs --hosts");
                        continue;
                    };
                    let host = loop {
                        if let Some(h) = read_host(dir, j) {
                            break h;
                        }
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    };
                    control.exchange(&hex::encode(host.to_bytes())).await;
                }
            }
        }
        std::future::pending::<()>().await;
    };
    // The page's lines end the run; the deadline covers a page that died.
    let deadline = Duration::from_millis((end_ms + 15_000).saturating_sub(now_ms()));
    tokio::select! {
        () = receive => {}
        () = async { tokio::join!(send, drive); } => {}
        () = tokio::time::sleep(deadline) => eprintln!("the page sent no last line"),
    }
    if let Some(c) = capture.as_mut() {
        c.flush();
    }
    if let Some(f) = player.as_mut()
        && let Err(e) = f.flush()
    {
        eprintln!("player file: {e}");
    }
    let _ = std::fs::remove_dir_all(&profile);
    Ok(())
}
