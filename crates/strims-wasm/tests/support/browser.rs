// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The browser tests' native side: the `wasm-e2e` bundle, a server for it
//! and the pages' WebSockets, and headless Firefox.
//!
//! - **Bundle.** `npx webpack --mode development --env TARGET=wasm-e2e`,
//!   run from the repo root, builds `dist/wasm-e2e`, with strims-wasm
//!   built for wasm32 with the `e2e` feature in the shared target dir. It
//!   runs once per test binary. Every webpack build ([`build`]) runs under
//!   one lock file in the target dir, so two test binaries don't build at
//!   once, and its files are then read into memory, so another binary's
//!   build can't change what a running test serves.
//! - **Server** ([`Site`], strims-testbed's). One listener on 127.0.0.1
//!   serves the bundle at its root. Every WebSocket upgrade goes to the
//!   test, by its path and query: e2e.html's `/control?i=<n>`, or whatever
//!   a page was told to connect to, such as test.html's `?manage=`.
//! - **Firefox.** `firefox --headless --no-remote` on a new profile under
//!   `~/snap/firefox/common`, since a snap Firefox can't see `/tmp`. Its
//!   prefs (`strims_testbed::browser::FIREFOX_PREFS`) leave ICE host
//!   candidates as addresses, so two pages in one browser link without
//!   mDNS, and put the pages' console on stdout. The output is captured
//!   and printed when the test fails. A test binary
//!   runs one [`Browser`] at a time; its Firefox can be closed and opened
//!   again on the same profile, which keeps its storage and loads only
//!   the page opened, and is killed when it drops.
//! - **Skips.** Without node and npx, node_modules, firefox, or a
//!   wasm-bindgen CLI of strims-wasm's pinned version, [`Browser::serve`]
//!   prints why and returns `None`.
//!
//! [`Browser::start`] opens e2e.html with n nodes, and returns a
//! [`Control`] for each, which speaks the protocol of strims-wasm's `e2e`
//! module.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::net::{IpAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use strims_crypto::key_from_seed;
pub use strims_testbed::browser::{Bundle, Site, Socket};
use strims_testbed::browser::{args, firefox_profile};
use strims_testbed::topology::Kind;
use tokio_tungstenite::tungstenite::Message;

/// How long a page has to open its sockets and start its nodes: the
/// first load compiles the wasm module.
pub const PAGE_TIMEOUT: Duration = Duration::from_secs(60);

/// How long [`Control`]'s helpers wait for an answer.
pub const STEP: Duration = Duration::from_secs(30);

/// The pages' zap log level.
const LOG_LEVEL: i32 = 0;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `cmd args`' stdout, when it runs and succeeds.
fn output_of(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The version in strims-wasm's `wasm-bindgen = "=X"` pin.
fn bindgen_pin() -> String {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("strims-wasm's manifest");
    manifest
        .lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("wasm-bindgen = \"=")?
                .strip_suffix('"')
                .map(str::to_owned)
        })
        .expect("strims-wasm pins wasm-bindgen")
}

/// Why the bundle can't be built here, if it can't.
fn missing_tool() -> Option<String> {
    for tool in ["node", "npx"] {
        if output_of(tool, &["--version"]).is_none() {
            return Some(format!("`{tool}` is not on PATH"));
        }
    }
    if !repo_root().join("node_modules").is_dir() {
        return Some("node_modules is missing (run `npm ci` at the repo root)".to_owned());
    }
    if output_of("firefox", &["--version"]).is_none() {
        return Some("`firefox` is not on PATH".to_owned());
    }
    let pin = bindgen_pin();
    match output_of("wasm-bindgen", &["--version"]) {
        Some(v) if v.split_whitespace().nth(1) == Some(pin.as_str()) => None,
        found => Some(format!(
            "strims-wasm needs wasm-bindgen-cli {pin} \
             (cargo install wasm-bindgen-cli --version {pin} --locked); found {}",
            found.as_deref().map_or("none", str::trim)
        )),
    }
}

pub enum Unavailable {
    Skip(String),
    Failed(String),
}

/// The wasm-e2e bundle, built on the first call. `None` after printing
/// why it can't be built here; a failed build fails the test.
fn bundle() -> Option<Arc<Bundle>> {
    static BUNDLE: OnceLock<Result<Arc<Bundle>, Unavailable>> = OnceLock::new();
    let built = BUNDLE.get_or_init(|| {
        let args = ["--mode", "development", "--env", "TARGET=wasm-e2e"];
        build("wasm-e2e", &args, "dist/wasm-e2e").map(Arc::new)
    });
    match built {
        Ok(b) => Some(b.clone()),
        Err(Unavailable::Skip(why)) => {
            eprintln!("skipping: {why}");
            None
        }
        Err(Unavailable::Failed(why)) => panic!("{why}"),
    }
}

/// Runs `npx webpack <args>` at the repo root under the target dir's lock
/// file, then reads `out`, a directory below the repo root, into memory.
pub fn build(name: &str, args: &[&str], out: &str) -> Result<Bundle, Unavailable> {
    if let Some(why) = missing_tool() {
        return Err(Unavailable::Skip(why));
    }
    let failed = |what: &str, e: std::io::Error| Unavailable::Failed(format!("{what}: {e}"));
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(tmp).map_err(|e| failed("the target tmp dir", e))?;
    let lock = File::create(tmp.join("webpack.lock")).map_err(|e| failed("the lock file", e))?;
    lock.lock().map_err(|e| failed("the lock file", e))?;

    let started = Instant::now();
    let root = repo_root();
    let res = Command::new("npx")
        // Never install: webpack comes from node_modules.
        .args(["--no", "--", "webpack"])
        .args(args)
        .current_dir(&root)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| failed("npx", e))?;
    if !res.status.success() {
        return Err(Unavailable::Failed(format!(
            "the {name} build failed ({}):\n{}\n{}",
            res.status,
            String::from_utf8_lossy(&res.stdout),
            String::from_utf8_lossy(&res.stderr)
        )));
    }
    let bundle = Bundle::read(&root.join(out)).map_err(|e| failed(out, e))?;
    drop(lock);
    eprintln!(
        "built the {name} bundle in {:.1?}: {} files, {} bytes",
        started.elapsed(),
        bundle.0.len(),
        bundle.0.values().map(Vec::len).sum::<usize>()
    );
    Ok(bundle)
}

/// A new Firefox profile with the harness's prefs. A snap Firefox has a
/// private /tmp; its own common dir is visible.
fn new_profile() -> PathBuf {
    static PROFILES: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("snap/firefox/common"))
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_TARGET_TMPDIR")));
    let profile = base.join(format!(
        "strims-wasm-e2e-{}-{}",
        std::process::id(),
        PROFILES.fetch_add(1, Ordering::Relaxed)
    ));
    firefox_profile(&profile).unwrap();
    profile
}

/// A headless Firefox on a page. Its stdout and stderr lines go to
/// `output`; dropping it kills it.
struct Firefox(Child);

impl Firefox {
    fn launch(profile: &Path, url: &str, output: &Arc<Mutex<Vec<String>>>) -> Firefox {
        let mut child = Command::new("firefox")
            .args(args(Kind::Firefox, profile, url))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let pipes: [Box<dyn Read + Send>; 2] = [
            Box::new(child.stdout.take().unwrap()),
            Box::new(child.stderr.take().unwrap()),
        ];
        for pipe in pipes {
            let output = output.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                    output.lock().unwrap().push(line);
                }
            });
        }
        Firefox(child)
    }
}

impl Drop for Firefox {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The address of the interface with the default route, unless that is
/// loopback or there is none. No datagram is sent.
pub fn outbound_ip() -> Option<IpAddr> {
    let probe = UdpSocket::bind("0.0.0.0:0").ok()?;
    probe.connect("192.0.2.1:9").ok()?;
    let ip = probe.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

/// The server, a Firefox profile, and the Firefox on it while a page is
/// open ([`Browser::open`]).
pub struct Browser {
    site: Site,
    /// Sockets that arrived while a test waited for another.
    pending: Vec<(String, Socket)>,
    profile: PathBuf,
    firefox: Option<Firefox>,
    /// The output of every Firefox on this profile, in order.
    output: Arc<Mutex<Vec<String>>>,
    /// Held while this browser lives: Firefox instances of one snap
    /// mustn't start concurrently.
    _turn: tokio::sync::MutexGuard<'static, ()>,
}

impl Drop for Browser {
    fn drop(&mut self) {
        self.close();
        if std::thread::panicking() {
            eprintln!(
                "--- firefox output ---\n{}\n--- end of firefox output ---",
                self.output().join("\n")
            );
        }
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

impl Browser {
    /// Serves the bundle, building it first if this binary hasn't, with no
    /// browser yet. `None` after printing why the bundle can't be built
    /// here.
    pub async fn serve() -> Option<Browser> {
        static TURN: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
        let bundle = bundle()?;
        let turn = TURN.lock().await;
        Some(Browser {
            site: Site::serve(bundle).await.unwrap(),
            pending: Vec::new(),
            profile: new_profile(),
            firefox: None,
            output: Arc::default(),
            _turn: turn,
        })
    }

    /// e2e.html with `count` nodes, and a control for each. `None` as for
    /// [`Browser::serve`].
    pub async fn start(count: usize) -> Option<(Browser, Vec<Control>)> {
        let mut browser = Browser::serve().await?;
        browser.open(&format!("/e2e.html?count={count}&logLevel={LOG_LEVEL}"));
        let mut controls = Vec::with_capacity(count);
        for index in 0..count {
            let ws = browser
                .socket(&format!("/control?i={index}"), PAGE_TIMEOUT)
                .await;
            controls.push(Control {
                index,
                ws,
                seen: VecDeque::new(),
            });
        }
        Some((browser, controls))
    }

    /// `http://127.0.0.1:<port>`.
    pub fn origin(&self) -> String {
        format!("http://{}", self.site.addr)
    }

    /// `ws://127.0.0.1:<port>`, for pages told where to connect.
    pub fn ws_origin(&self) -> String {
        format!("ws://{}", self.site.addr)
    }

    /// Opens `page`, a path and query on the server such as
    /// `/test.html?…`, in a headless Firefox on this browser's profile.
    pub fn open(&mut self, page: &str) {
        assert!(self.firefox.is_none(), "a browser is already open");
        let url = format!("{}{page}", self.origin());
        self.firefox = Some(Firefox::launch(&self.profile, &url, &self.output));
    }

    /// Kills the Firefox [`Browser::open`] started, if it runs. The
    /// profile stays, with its storage, for the next `open`.
    pub fn close(&mut self) {
        drop(self.firefox.take());
    }

    /// The next WebSocket a page opens to `path` (its path and query),
    /// waiting up to `timeout`.
    pub async fn socket(&mut self, path: &str, timeout: Duration) -> Socket {
        if let Some(i) = self.pending.iter().position(|(p, _)| p == path) {
            return self.pending.remove(i).1;
        }
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            match tokio::time::timeout_at(deadline, self.site.sockets.recv()).await {
                Ok(Some((p, ws))) if p == path => return ws,
                Ok(Some(other)) => self.pending.push(other),
                Ok(None) => unreachable!("the server runs while the browser does"),
                Err(_) => {
                    let opened: Vec<&str> = self.pending.iter().map(|(p, _)| p.as_str()).collect();
                    panic!("timed out waiting for the page to open {path}; it opened {opened:?}");
                }
            }
        }
    }

    /// The paths of the sockets pages opened that no [`Browser::socket`]
    /// call took.
    pub fn unclaimed(&mut self) -> Vec<String> {
        while let Ok(s) = self.site.sockets.try_recv() {
            self.pending.push(s);
        }
        self.pending.iter().map(|(p, _)| p.clone()).collect()
    }

    /// What Firefox printed so far, the pages' console included.
    pub fn output(&self) -> Vec<String> {
        self.output.lock().map(|o| o.clone()).unwrap_or_default()
    }
}

/// How [`Control::start`] starts a page's node.
#[derive(Clone, Debug)]
pub struct StartOptions {
    /// The ed25519 seed of the node's profile key.
    pub profile_seed: [u8; 32],
    pub rng_seed: u64,
    pub network_key: Vec<u8>,
    pub bind_port: u16,
    pub echo_port: u16,
    pub reply_port: u16,
    pub max_buffered_amount: Option<usize>,
}

/// A page's peer, from `peerAdded`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PagePeer {
    pub kind: String,
    pub dialed: bool,
}

/// One e2e node of the page: commands out, events in.
pub struct Control {
    pub index: usize,
    ws: Socket,
    /// Events that arrived while a wait looked for another.
    seen: VecDeque<Value>,
}

/// `event` with its byte strings replaced by their lengths.
fn summary(event: &Value) -> Value {
    let mut e = event.clone();
    if let Some(b) = e.get_mut("bytes")
        && let Some(s) = b.as_str()
    {
        *b = format!("<{} bytes>", s.len() / 2).into();
    }
    e
}

impl Control {
    /// Sends one command.
    pub async fn command(&mut self, cmd: Value) {
        let msg = Message::binary(cmd.to_string().into_bytes());
        if let Err(e) = self.ws.send(msg).await {
            panic!("node {}: the control socket failed: {e}", self.index);
        }
    }

    /// The next event from the page within `timeout`.
    async fn recv(&mut self, timeout: Duration) -> Option<Value> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let msg = match tokio::time::timeout_at(deadline, self.ws.next()).await {
                Err(_) => return None,
                Ok(None) => panic!("node {}: the page closed its control socket", self.index),
                Ok(Some(Err(e))) => panic!("node {}: the control socket failed: {e}", self.index),
                Ok(Some(Ok(m))) => m,
            };
            match msg {
                Message::Binary(b) => {
                    return Some(serde_json::from_slice(&b).unwrap_or_else(|e| {
                        panic!("node {}: an event that isn't JSON: {e}", self.index)
                    }));
                }
                Message::Close(_) => {
                    panic!("node {}: the page closed its control socket", self.index)
                }
                _ => {}
            }
        }
    }

    /// The first event `f` accepts, waiting up to `timeout`; the others
    /// are kept for later waits. An `error` event `f` doesn't accept fails
    /// the test.
    pub async fn try_wait(
        &mut self,
        what: &str,
        timeout: Duration,
        mut f: impl FnMut(&Value) -> bool,
    ) -> Option<Value> {
        if let Some(i) = self.seen.iter().position(&mut f) {
            return self.seen.remove(i);
        }
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let e = self.recv(left).await?;
            if f(&e) {
                return Some(e);
            }
            if e["type"] == "error" {
                panic!("node {}: {e} while waiting for {what}", self.index);
            }
            self.seen.push_back(e);
        }
    }

    /// [`Control::try_wait`], failing the test on a timeout.
    pub async fn wait(
        &mut self,
        what: &str,
        timeout: Duration,
        f: impl FnMut(&Value) -> bool,
    ) -> Value {
        match self.try_wait(what, timeout, f).await {
            Some(e) => e,
            None => {
                let seen: Vec<Value> = self.seen.iter().map(summary).collect();
                panic!(
                    "node {}: timed out waiting for {what}; seen {seen:?}",
                    self.index
                )
            }
        }
    }

    /// Starts the page's node and returns its host id.
    pub async fn start(&mut self, o: &StartOptions) -> String {
        let key = key_from_seed(&o.profile_seed);
        let mut cmd = json!({
            "type": "start",
            "profileKey": {
                "type": key.r#type,
                "private": hex::encode(&key.private),
                "public": hex::encode(&key.public),
            },
            "rngSeed": o.rng_seed,
            "networkKey": hex::encode(&o.network_key),
            "bindPort": o.bind_port,
            "echoPort": o.echo_port,
            "replyPort": o.reply_port,
        });
        if let Some(m) = o.max_buffered_amount {
            cmd["maxBufferedAmount"] = m.into();
        }
        self.command(cmd).await;
        let e = self
            .wait("started", PAGE_TIMEOUT, |e| e["type"] == "started")
            .await;
        e["hostId"].as_str().unwrap().to_owned()
    }

    /// Dials `uri` and returns the peer's host id.
    pub async fn dial(&mut self, uri: &str) -> String {
        self.command(json!({"type": "dial", "uri": uri})).await;
        let e = self
            .wait(&format!("dialed {uri}"), STEP, |e| {
                e["type"] == "dialed" && e["uri"] == uri
            })
            .await;
        e["hostId"].as_str().unwrap().to_owned()
    }

    /// Starts a peer exchange with `host`; false when none started.
    pub async fn connect_peer(&mut self, host: &str) -> bool {
        self.command(json!({"type": "connectPeer", "hostId": host}))
            .await;
        let e = self
            .wait(&format!("connecting {host}"), STEP, |e| {
                e["type"] == "connecting" && e["hostId"] == host
            })
            .await;
        e["started"].as_bool().unwrap()
    }

    /// Sends `bytes` to `host`'s echo port.
    pub async fn send_message(&mut self, host: &str, bytes: &[u8]) {
        self.command(json!({"type": "send", "hostId": host, "bytes": hex::encode(bytes)}))
            .await;
    }

    /// The next message from `host` on the reply port, waiting up to
    /// `timeout`.
    pub async fn try_message(&mut self, from: &str, timeout: Duration) -> Option<Vec<u8>> {
        let e = self
            .try_wait(&format!("a message from {from}"), timeout, |e| {
                e["type"] == "message" && e["from"] == from
            })
            .await?;
        Some(hex::decode(e["bytes"].as_str().unwrap()).unwrap())
    }

    pub async fn peer_added(&mut self, host: &str) -> PagePeer {
        let e = self
            .wait(&format!("peerAdded {host}"), STEP, |e| {
                e["type"] == "peerAdded" && e["hostId"] == host
            })
            .await;
        PagePeer {
            kind: e["kind"].as_str().unwrap().to_owned(),
            dialed: e["dialed"].as_bool().unwrap(),
        }
    }

    pub async fn peer_removed(&mut self, host: &str) {
        self.wait(&format!("peerRemoved {host}"), STEP, |e| {
            e["type"] == "peerRemoved" && e["hostId"] == host
        })
        .await;
    }

    /// The outcome of the exchange with `host`, in its debug form.
    pub async fn exchange_closed(&mut self, host: &str) -> String {
        let e = self
            .wait(&format!("exchangeClosed {host}"), STEP, |e| {
                e["type"] == "exchangeClosed" && e["hostId"] == host
            })
            .await;
        e["outcome"].as_str().unwrap().to_owned()
    }

    /// The node's `stats`.
    pub async fn stats(&mut self) -> Value {
        self.command(json!({"type": "stats"})).await;
        self.wait("stats", STEP, |e| e["type"] == "stats").await
    }
}
