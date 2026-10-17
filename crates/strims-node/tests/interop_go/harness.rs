// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The harness: Go nodes (the `golden/interop` helper, one subprocess per
//! node, driven over stdin/stdout) and Rust nodes (`Node` on `TokioNet`),
//! behind one [`TestNode`] interface so topologies can mix them freely.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration as StdDuration;

use prost::Message;
use sha2::{Digest, Sha256};
use strims_crypto::key_from_seed;
use strims_node::{
    DeliveryMode, HashTableGetOptions, HostId, LinkKind, NetworkId, NetworkOptions, Node,
    NodeConfig, NodeEvent, NodeEvents, SwarmId, SwarmKey, SwarmOptions, SwarmWriter, SystemClock,
    TokioNet, WebRtcConfig, WriterOptions,
};
use strims_proto::chat::v1 as chat;
use strims_proto::network::v1::directory::{AssetBundle, EventBroadcast};
use strims_vpn::{Routing, VerifyMode};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::Instant;

/// The vnic port every peer is bound to the network on, on both sides.
pub const VPN_PORT: u16 = 10_000;
/// The vpn port test messages go to.
pub const APP_PORT: u16 = 20_000;
/// How long one step (a reply, an event) may take.
pub const STEP: StdDuration = StdDuration::from_secs(20);
/// Stream bytes are written in units of one signature (64 chunks of 1 KiB).
pub const UNIT: usize = 64 * 1024;

/// The seed of the network's key, which issues membership certificates.
pub const NETWORK_SEED: [u8; 32] = [0xee; 32];

pub fn network_key() -> Vec<u8> {
    key_from_seed(&NETWORK_SEED).public
}

/// The stream byte at absolute offset `o` (the Go helper's `streamByte`).
pub fn stream_byte(o: u64) -> u8 {
    let x = o.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ (o >> 7);
    (x >> 56) as u8
}

/// SHA-256 of the stream bytes `start..start + len`.
pub fn expected_sha(start: u64, len: u64) -> String {
    let mut h = Sha256::new();
    let mut b = Vec::with_capacity(UNIT);
    let mut o = start;
    while o < start + len {
        b.clear();
        let end = (o + UNIT as u64).min(start + len);
        b.extend((o..end).map(stream_byte));
        h.update(&b);
        o = end;
    }
    hex::encode(h.finalize())
}

pub fn swarm_options(live_window: usize) -> SwarmOptions {
    SwarmOptions {
        live_window,
        delivery_mode: DeliveryMode::MANDATORY,
        ..Default::default()
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn id_hex(id: &HostId) -> String {
    hex::encode(id.to_bytes())
}

fn parse_id(s: &str) -> HostId {
    HostId::unmarshal(&hex::decode(s).expect("host id hex"))
        .expect("host id")
        .0
}

fn unhex(s: &str) -> Vec<u8> {
    if s == "-" {
        Vec::new()
    } else {
        hex::decode(s).expect("hex")
    }
}

fn hexs(b: &[u8]) -> String {
    if b.is_empty() {
        "-".to_owned()
    } else {
        hex::encode(b)
    }
}

// --- the Go helper -----------------------------------------------------------

/// The helper must build `pkg/vpn` as committed: the `graph*.go` topology
/// recorder needs a fixed path on every received frame. This `go -overlay`
/// takes `host.go` and `network.go` from `HEAD` and drops `graph*.go`;
/// `None` when the working copy has no recorder.
fn overlay_without_graph_wip() -> Option<String> {
    let vpn = repo_root().join("pkg/vpn");
    if !vpn.join("graph_native.go").exists() {
        return None;
    }
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("node-interop-overlay");
    std::fs::create_dir_all(&dir).unwrap();
    let mut replace = Vec::new();
    for f in ["host.go", "network.go"] {
        let out = std::process::Command::new("git")
            .args(["show", &format!("HEAD:pkg/vpn/{f}")])
            .current_dir(repo_root())
            .output()
            .expect("git show");
        assert!(out.status.success(), "git show HEAD:pkg/vpn/{f}");
        let path = dir.join(f);
        std::fs::write(&path, out.stdout).unwrap();
        replace.push(format!(
            "{:?}: {:?}",
            vpn.join(f).canonicalize().unwrap().display().to_string(),
            path.display().to_string()
        ));
    }
    for f in [
        "graph.go",
        "graph_js.go",
        "graph_native.go",
        "graph_test.go",
    ] {
        if vpn.join(f).exists() {
            replace.push(format!(
                "{:?}: \"\"",
                vpn.join(f).canonicalize().unwrap().display().to_string()
            ));
        }
    }
    let json = dir.join("overlay.json");
    std::fs::write(
        &json,
        format!("{{\"Replace\": {{{}}}}}", replace.join(", ")),
    )
    .unwrap();
    Some(format!("-overlay={}", json.display()))
}

/// Builds the helper once per test binary; `None` (skip) without `go`.
pub fn helper() -> Option<PathBuf> {
    static HELPER: OnceLock<Option<PathBuf>> = OnceLock::new();
    HELPER.get_or_init(build_helper).clone()
}

fn build_helper() -> Option<PathBuf> {
    match std::process::Command::new("go").arg("version").output() {
        Ok(o) if o.status.success() => {}
        _ => {
            eprintln!("skipping: `go` is not on PATH");
            return None;
        }
    }
    let bin = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("node-interop-go");
    let out = std::process::Command::new("go")
        .args(["build", "-tags", "golden"])
        .args(overlay_without_graph_wip())
        .arg("-o")
        .arg(&bin)
        .arg("./crates/strims-node/golden/interop")
        .current_dir(repo_root())
        .output()
        .expect("go build");
    assert!(
        out.status.success(),
        "go build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(bin)
}

/// What a Go node reported.
#[derive(Clone, Debug, PartialEq)]
pub enum GoEvent {
    Peer {
        host_id: HostId,
        platform: String,
        version: String,
        /// The Go type under the peer's link: `*vnic.aesLink` (TCP),
        /// `*httputil.WSReadWriter` (WebSocket), `*vnic.dcLink` (WebRTC).
        link: String,
    },
    PeerRemoved(HostId),
    /// internal/network bound the peer to the network with this key.
    NetOpen(HostId, Vec<u8>),
    /// internal/network unbound the peer from the network.
    NetClose(HostId, Vec<u8>),
    /// internal/directory read an event broadcast on the network with
    /// this key.
    DirEvent(Vec<u8>, EventBroadcast),
    /// internal/directory read an asset bundle on the network with this
    /// key.
    DirAssets(Vec<u8>, AssetBundle),
    /// internal/chat read an event of the chat server with this key.
    ChatEvent(Vec<u8>, chat::ServerEvent),
    /// internal/chat read an asset bundle of the chat server with this
    /// key.
    ChatAssets(Vec<u8>, chat::AssetBundle),
    /// The read of the chat server with this key ended.
    ChatReadEnd(Vec<u8>),
    /// internal/chat wrote a whisper record.
    WhisperRecord(chat::WhisperRecord),
    /// internal/chat wrote a whisper thread.
    WhisperThread(chat::WhisperThread),
    TransferError(HostId, String),
    Written(String),
    ReadDone(String, ReadResult),
    Other(String),
}

/// A received vpn message, from either implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Msg {
    pub src: HostId,
    pub dst_port: u16,
    pub hops: usize,
    pub flags: u16,
    pub body: Vec<u8>,
    /// Rust only (`VerifyMode::Audit`): whether every trailer signature
    /// verified.
    pub verified: Option<bool>,
}

/// A finished stream read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadResult {
    pub start: u64,
    pub len: u64,
    pub sha: String,
    pub mismatches: u64,
    pub error: Option<String>,
}

impl ReadResult {
    /// Byte-exact from offset 0 to `total`.
    pub fn assert_exact(&self, who: &str, total: u64) {
        assert_eq!(self.error, None, "{who}: read failed: {self:?}");
        assert_eq!(self.mismatches, 0, "{who}: mismatched bytes: {self:?}");
        assert_eq!(self.start, 0, "{who}: started late: {self:?}");
        assert_eq!(self.len, total, "{who}: short read: {self:?}");
        assert_eq!(
            self.sha,
            expected_sha(self.start, self.len),
            "{who}: sha differs"
        );
    }
}

pub struct GoNode {
    pub name: String,
    pub host_id: HostId,
    pub profile_key: Vec<u8>,
    pub tcp_uri: String,
    pub ws_uri: String,
    /// The WebRTC UDP mux address, with `-webrtc`.
    pub webrtc_addr: Option<String>,
    _child: Child,
    stdin: ChildStdin,
    lines: mpsc::UnboundedReceiver<String>,
    next_id: u64,
    replies: BTreeMap<u64, Result<String, String>>,
    events: Vec<GoEvent>,
    msgs: Vec<Msg>,
}

#[derive(Clone, Debug)]
pub struct GoOptions {
    /// Profile key seed byte; the host key seed is derived from it.
    pub seed: u8,
    pub max_peers: u32,
    pub rate_limit: Option<u64>,
    /// Serve TCP and WebSocket (false: dial only, advertise nothing).
    pub listen: bool,
    /// Add the WebRTC interface (`NewWebRTCInterface` on a loopback UDP
    /// mux, no ICE servers).
    pub webrtc: bool,
    /// Run RPC on every peer's channel pair, serving the test Debug
    /// service (the `rpc-*` verbs call the peer's).
    pub peer_rpc: bool,
    /// Issue a host certificate on the network for RPC over vpn (the
    /// `vpn-*` verbs).
    pub vpn_rpc: bool,
    /// Run swarms through internal/transfer (the `transfer-*` verbs);
    /// needs `peer_rpc`.
    pub transfer: bool,
    /// Bind peers to networks through internal/network (the `net-*`
    /// verbs) in place of the fixed port pair, and run their swarms
    /// through its internal/transfer; needs `peer_rpc`.
    pub network_control: bool,
    /// Run internal/directory on the networks (the `dir-*` verbs and
    /// events); needs `network_control`.
    pub directory: bool,
    /// Run internal/chat on the profile (the `chat-*` verbs, the chat and
    /// whisper events); needs `directory`.
    pub chat: bool,
}

impl GoOptions {
    pub fn new(seed: u8) -> GoOptions {
        GoOptions {
            seed,
            max_peers: 0,
            rate_limit: None,
            listen: true,
            webrtc: false,
            peer_rpc: false,
            vpn_rpc: false,
            transfer: false,
            network_control: false,
            directory: false,
            chat: false,
        }
    }
}

impl GoNode {
    pub async fn spawn(bin: &PathBuf, name: &str, o: GoOptions) -> GoNode {
        let mut cmd = Command::new(bin);
        cmd.arg(format!("-profile-seed={}", hex::encode([o.seed; 32])))
            .arg(format!(
                "-host-seed={}",
                hex::encode([o.seed.wrapping_add(0x80); 32])
            ))
            .arg(format!("-network={}", hex::encode(network_key())))
            .arg(format!("-vpn-port={VPN_PORT}"))
            .arg(format!("-label=go-{name}"))
            .arg(format!("-max-peers={}", o.max_peers))
            .arg(format!("-rate-limit={}", o.rate_limit.unwrap_or(0)))
            .arg(format!("-listen={}", o.listen))
            .arg(format!("-webrtc={}", o.webrtc))
            .arg(format!("-peer-rpc={}", o.peer_rpc))
            .arg(format!("-transfer={}", o.transfer))
            .arg(format!("-network-control={}", o.network_control))
            .arg(format!("-directory={}", o.directory))
            .arg(format!("-chat={}", o.chat))
            .arg(format!(
                "-network-seed={}",
                if o.vpn_rpc {
                    hex::encode(NETWORK_SEED)
                } else {
                    String::new()
                }
            ))
            .arg("-timeout=15m")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = cmd.spawn().expect("spawn go helper");
        let (tx, mut lines) = mpsc::unbounded_channel();
        let stdout = child.stdout.take().unwrap();
        tokio::spawn(async move {
            let mut r = BufReader::new(stdout).lines();
            while let Ok(Some(l)) = r.next_line().await {
                if tx.send(l).is_err() {
                    return;
                }
            }
        });
        let stderr = child.stderr.take().unwrap();
        let tag = name.to_owned();
        tokio::spawn(async move {
            let mut r = BufReader::new(stderr).lines();
            while let Ok(Some(l)) = r.next_line().await {
                eprintln!("go {tag}: {l}");
            }
        });
        let stdin = child.stdin.take().unwrap();
        let ready = tokio::time::timeout(STEP, lines.recv())
            .await
            .unwrap_or_else(|_| panic!("go {name}: no ready line"))
            .unwrap_or_else(|| panic!("go {name}: exited before ready"));
        let f: Vec<&str> = ready.split(' ').collect();
        assert_eq!(f[0], "ready", "go {name}: {ready}");
        let mut g = GoNode {
            name: name.to_owned(),
            host_id: parse_id(f[1]),
            profile_key: key_from_seed(&[o.seed; 32]).public,
            tcp_uri: f[2].to_owned(),
            ws_uri: f[3].to_owned(),
            webrtc_addr: f.get(4).filter(|a| **a != "-").map(|a| (*a).to_owned()),
            _child: child,
            stdin,
            lines,
            next_id: 1,
            replies: BTreeMap::new(),
            events: Vec::new(),
            msgs: Vec::new(),
        };
        g.cmd(&format!("handler {APP_PORT}")).await.unwrap();
        g
    }

    fn handle_line(&mut self, l: String) {
        let f: Vec<&str> = l.split(' ').collect();
        match f[0] {
            "ok" | "err" => {
                let id: u64 = f[1].parse().unwrap();
                let v = match f.get(2..) {
                    Some(v) if !v.is_empty() => v.join(" "),
                    _ => "-".to_owned(),
                };
                let r = if f[0] == "ok" {
                    Ok(v)
                } else {
                    Err(String::from_utf8_lossy(&unhex(&v)).into_owned())
                };
                self.replies.insert(id, r);
            }
            "msg" => self.msgs.push(Msg {
                src: parse_id(f[1]),
                dst_port: f[3].parse().unwrap(),
                hops: f[4].parse().unwrap(),
                flags: f[5].parse().unwrap(),
                body: unhex(f[6]),
                verified: None,
            }),
            "peer" => self.events.push(GoEvent::Peer {
                host_id: parse_id(f[1]),
                platform: String::from_utf8(unhex(f[2])).unwrap(),
                version: String::from_utf8(unhex(f[3])).unwrap(),
                link: f.get(4).copied().unwrap_or_default().to_owned(),
            }),
            "peer-removed" => self.events.push(GoEvent::PeerRemoved(parse_id(f[1]))),
            "net-open" => self
                .events
                .push(GoEvent::NetOpen(parse_id(f[1]), unhex(f[2]))),
            "net-close" => self
                .events
                .push(GoEvent::NetClose(parse_id(f[1]), unhex(f[2]))),
            "dir-event" => self.events.push(GoEvent::DirEvent(
                unhex(f[1]),
                EventBroadcast::decode(unhex(f[2]).as_slice()).expect("go's event broadcast"),
            )),
            "dir-assets" => self.events.push(GoEvent::DirAssets(
                unhex(f[1]),
                AssetBundle::decode(unhex(f[2]).as_slice()).expect("go's asset bundle"),
            )),
            "chat-event" => self.events.push(GoEvent::ChatEvent(
                unhex(f[1]),
                chat::ServerEvent::decode(unhex(f[2]).as_slice()).expect("go's chat event"),
            )),
            "chat-assets" => self.events.push(GoEvent::ChatAssets(
                unhex(f[1]),
                chat::AssetBundle::decode(unhex(f[2]).as_slice()).expect("go's chat assets"),
            )),
            "chat-read-end" => self.events.push(GoEvent::ChatReadEnd(unhex(f[1]))),
            "whisper-record" => self.events.push(GoEvent::WhisperRecord(
                chat::WhisperRecord::decode(unhex(f[1]).as_slice()).expect("go's whisper record"),
            )),
            "whisper-thread" => self.events.push(GoEvent::WhisperThread(
                chat::WhisperThread::decode(unhex(f[1]).as_slice()).expect("go's whisper thread"),
            )),
            "transfer-error" => {
                eprintln!("go {}: {l}", self.name);
                self.events.push(GoEvent::TransferError(
                    parse_id(f[1]),
                    String::from_utf8_lossy(&unhex(f[2])).into_owned(),
                ));
            }
            "written" => self.events.push(GoEvent::Written(f[1].to_owned())),
            "read-done" => self.events.push(GoEvent::ReadDone(
                f[1].to_owned(),
                ReadResult {
                    start: f[2].parse().unwrap(),
                    len: f[3].parse().unwrap(),
                    sha: f[4].to_owned(),
                    mismatches: f[5].parse().unwrap(),
                    error: None,
                },
            )),
            "read-error" => self.events.push(GoEvent::ReadDone(
                f[1].to_owned(),
                ReadResult {
                    start: 0,
                    len: f[2].parse().unwrap(),
                    sha: String::new(),
                    mismatches: 0,
                    error: Some(String::from_utf8_lossy(&unhex(f[3])).into_owned()),
                },
            )),
            _ => self.events.push(GoEvent::Other(l)),
        }
    }

    /// Handles whatever the helper has printed so far.
    pub fn poll(&mut self) {
        while let Ok(l) = self.lines.try_recv() {
            self.handle_line(l);
        }
    }

    /// Waits for one more line.
    async fn next_line(&mut self, deadline: Instant) -> bool {
        match tokio::time::timeout_at(deadline, self.lines.recv()).await {
            Ok(Some(l)) => {
                self.handle_line(l);
                true
            }
            Ok(None) => panic!("go {}: helper exited", self.name),
            Err(_) => false,
        }
    }

    /// Runs a command; its reply values, or the error.
    pub async fn cmd_timeout(
        &mut self,
        line: &str,
        timeout: StdDuration,
    ) -> Result<String, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.stdin
            .write_all(format!("{id} {line}\n").as_bytes())
            .await
            .expect("go stdin");
        self.stdin.flush().await.expect("go stdin");
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(r) = self.replies.remove(&id) {
                return r;
            }
            if !self.next_line(deadline).await {
                panic!("go {}: no reply to {line:?}", self.name);
            }
        }
    }

    pub async fn cmd(&mut self, line: &str) -> Result<String, String> {
        self.cmd_timeout(line, STEP).await
    }

    /// Waits for an event matching `f`, keeping the others.
    pub async fn wait(&mut self, what: &str, mut f: impl FnMut(&GoEvent) -> bool) -> GoEvent {
        let deadline = Instant::now() + STEP;
        self.wait_until(what, deadline, &mut f).await
    }

    pub async fn wait_until(
        &mut self,
        what: &str,
        deadline: Instant,
        mut f: impl FnMut(&GoEvent) -> bool,
    ) -> GoEvent {
        loop {
            if let Some(i) = self.events.iter().position(&mut f) {
                return self.events.remove(i);
            }
            if !self.next_line(deadline).await {
                panic!("go {}: timed out waiting for {what}", self.name);
            }
        }
    }

    pub fn events(&mut self) -> &mut Vec<GoEvent> {
        self.poll();
        &mut self.events
    }

    /// Messages received so far, drained.
    pub fn take_msgs(&mut self) -> Vec<Msg> {
        self.poll();
        std::mem::take(&mut self.msgs)
    }
}

// --- Rust nodes --------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct RustOptions {
    /// Profile key seed byte; the host key and rng seeds derive from it.
    pub seed: u8,
    pub max_peers: u32,
    pub rate_limit: Option<u64>,
    pub routing: Routing,
    /// Serve TCP and WebSocket (false: dial only, advertise nothing).
    pub listen: bool,
    /// The WebRTC interface on a loopback UDP socket, no ICE servers.
    pub webrtc: bool,
}

impl RustOptions {
    pub fn new(seed: u8) -> RustOptions {
        RustOptions {
            seed,
            max_peers: 16,
            rate_limit: None,
            routing: Routing::default(),
            listen: true,
            webrtc: false,
        }
    }
}

struct RustSwarm {
    key: SwarmKey,
    writer: Option<SwarmWriter>,
    write: Option<JoinHandle<()>>,
    read: Option<JoinHandle<ReadResult>>,
}

pub struct RustNode {
    pub name: String,
    pub node: Node,
    pub net: NetworkId,
    events: NodeEvents,
    seen: Vec<NodeEvent>,
    msgs: Vec<Msg>,
    swarms: BTreeMap<String, RustSwarm>,
}

impl RustNode {
    pub async fn spawn(name: &str, o: RustOptions) -> RustNode {
        let key = key_from_seed(&[o.seed; 32]);
        let mut config = NodeConfig::new(key, u64::from(o.seed));
        config.host_key_seed = Some([o.seed.wrapping_add(0x80); 32]);
        config.label = format!("rust-{name}");
        if o.listen {
            config.tcp_listen = Some("127.0.0.1:0".to_owned());
            config.ws_listen = Some("127.0.0.1:0".to_owned());
        }
        if o.webrtc {
            config.webrtc = WebRtcConfig::local(["127.0.0.1:0".to_owned()]);
        }
        config.allow_insecure_ws = true;
        config.max_peers = o.max_peers;
        config.rate_limit = o.rate_limit;
        config.vpn.verify = VerifyMode::Audit;
        config.vpn.routing = o.routing;
        let (node, events) = Node::spawn(config, TokioNet, SystemClock).await.unwrap();
        let net = node
            .add_network(
                &network_key(),
                NetworkOptions {
                    auto_bind_port: Some(VPN_PORT),
                },
            )
            .await
            .unwrap();
        node.set_vpn_handler(net, APP_PORT).await.unwrap();
        RustNode {
            name: name.to_owned(),
            node,
            net,
            events,
            seen: Vec::new(),
            msgs: Vec::new(),
            swarms: BTreeMap::new(),
        }
    }

    fn handle(&mut self, e: NodeEvent) {
        match e {
            NodeEvent::Message(d) => self.msgs.push(Msg {
                src: d.src,
                dst_port: d.dst_port,
                hops: d.hops,
                flags: d.flags,
                body: d.body,
                verified: d.verified,
            }),
            NodeEvent::TransferError { .. } => {
                eprintln!("rust {}: {e:?}", self.name);
                self.seen.push(e);
            }
            e => self.seen.push(e),
        }
    }

    pub fn poll(&mut self) {
        while let Some(e) = self.events.try_recv() {
            self.handle(e);
        }
    }

    /// Waits for an event matching `f`, keeping the others.
    pub async fn wait(&mut self, what: &str, mut f: impl FnMut(&NodeEvent) -> bool) -> NodeEvent {
        let deadline = Instant::now() + STEP;
        loop {
            if let Some(i) = self.seen.iter().position(&mut f) {
                return self.seen.remove(i);
            }
            match tokio::time::timeout_at(deadline, self.events.recv()).await {
                Ok(Some(e)) => self.handle(e),
                Ok(None) => panic!("rust {}: node stopped", self.name),
                Err(_) => panic!(
                    "rust {}: timed out waiting for {what}; seen {:?}",
                    self.name, self.seen
                ),
            }
        }
    }

    pub fn events(&mut self) -> &mut Vec<NodeEvent> {
        self.poll();
        &mut self.seen
    }
}

// --- either ------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Impl {
    Go,
    Rust,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transport {
    Tcp,
    Ws,
}

impl Transport {
    pub fn kind(self) -> LinkKind {
        match self {
            Transport::Tcp => LinkKind::Tcp,
            Transport::Ws => LinkKind::Ws,
        }
    }
}

pub enum TestNode {
    Go(GoNode),
    Rust(RustNode),
}

impl TestNode {
    pub fn imp(&self) -> Impl {
        match self {
            TestNode::Go(_) => Impl::Go,
            TestNode::Rust(_) => Impl::Rust,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            TestNode::Go(g) => &g.name,
            TestNode::Rust(r) => &r.name,
        }
    }

    pub fn host_id(&self) -> HostId {
        match self {
            TestNode::Go(g) => g.host_id,
            TestNode::Rust(r) => r.node.host_id(),
        }
    }

    pub fn go(&mut self) -> &mut GoNode {
        match self {
            TestNode::Go(g) => g,
            TestNode::Rust(r) => panic!("{} is a Rust node", r.name),
        }
    }

    pub fn rust(&mut self) -> &mut RustNode {
        match self {
            TestNode::Rust(r) => r,
            TestNode::Go(g) => panic!("{} is a Go node", g.name),
        }
    }

    pub fn uri(&self, t: Transport) -> String {
        match (self, t) {
            (TestNode::Go(g), Transport::Tcp) => g.tcp_uri.clone(),
            (TestNode::Go(g), Transport::Ws) => g.ws_uri.clone(),
            (TestNode::Rust(r), Transport::Tcp) => r.node.tcp_uri().unwrap().to_owned(),
            (TestNode::Rust(r), Transport::Ws) => r.node.ws_uri().unwrap().to_owned(),
        }
    }

    pub fn poll(&mut self) {
        match self {
            TestNode::Go(g) => g.poll(),
            TestNode::Rust(r) => r.poll(),
        }
    }

    /// Dials `uri`; the peer's host id.
    pub async fn dial(&mut self, uri: &str) -> HostId {
        match self {
            TestNode::Go(g) => parse_id(&g.cmd(&format!("dial {uri}")).await.unwrap()),
            TestNode::Rust(r) => r.node.dial(uri).await.unwrap(),
        }
    }

    /// Waits until `peer` is a connected peer.
    pub async fn wait_peer(&mut self, peer: HostId) {
        match self {
            TestNode::Go(g) => {
                g.wait(
                    "peer",
                    |e| matches!(e, GoEvent::Peer { host_id, .. } if *host_id == peer),
                )
                .await;
            }
            TestNode::Rust(r) => {
                r.wait(
                    "peer",
                    |e| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == peer),
                )
                .await;
            }
        }
    }

    /// Closes the link to `peer`.
    pub async fn close_peer(&mut self, peer: HostId) {
        match self {
            TestNode::Go(g) => {
                g.cmd(&format!("close-peer {}", id_hex(&peer)))
                    .await
                    .unwrap();
            }
            TestNode::Rust(r) => assert!(r.node.close_peer(peer).await.unwrap()),
        }
    }

    /// Waits until `peer` has gone.
    pub async fn wait_peer_removed(&mut self, peer: HostId) {
        match self {
            TestNode::Go(g) => {
                g.wait(
                    "peer removed",
                    |e| matches!(e, GoEvent::PeerRemoved(h) if *h == peer),
                )
                .await;
            }
            TestNode::Rust(r) => {
                r.wait(
                    "peer removed",
                    |e| matches!(e, NodeEvent::PeerRemoved { host_id } if *host_id == peer),
                )
                .await;
            }
        }
    }

    pub async fn send(&mut self, dst: HostId, body: &[u8]) {
        match self {
            TestNode::Go(g) => {
                g.cmd(&format!(
                    "send {} {APP_PORT} {APP_PORT} {}",
                    id_hex(&dst),
                    hexs(body)
                ))
                .await
                .unwrap();
            }
            TestNode::Rust(r) => r
                .node
                .send(r.net, dst, APP_PORT, APP_PORT, body.to_vec())
                .await
                .unwrap(),
        }
    }

    pub async fn broadcast(&mut self, body: &[u8]) {
        match self {
            TestNode::Go(g) => {
                g.cmd(&format!("bcast {APP_PORT} {APP_PORT} {}", hexs(body)))
                    .await
                    .unwrap();
            }
            TestNode::Rust(r) => r
                .node
                .broadcast(r.net, APP_PORT, APP_PORT, body.to_vec())
                .await
                .unwrap(),
        }
    }

    /// Messages received so far, drained.
    pub fn take_msgs(&mut self) -> Vec<Msg> {
        self.poll();
        match self {
            TestNode::Go(g) => std::mem::take(&mut g.msgs),
            TestNode::Rust(r) => std::mem::take(&mut r.msgs),
        }
    }

    /// Waits for a message with this body.
    pub async fn wait_msg(&mut self, body: &[u8]) -> Msg {
        let deadline = Instant::now() + STEP;
        loop {
            self.poll();
            let msgs = match self {
                TestNode::Go(g) => &mut g.msgs,
                TestNode::Rust(r) => &mut r.msgs,
            };
            if let Some(i) = msgs.iter().position(|m| m.body == body) {
                return msgs.remove(i);
            }
            // Wait for the next line or event.
            let more = match self {
                TestNode::Go(g) => g.next_line(deadline).await,
                TestNode::Rust(r) => match tokio::time::timeout_at(deadline, r.events.recv()).await
                {
                    Ok(Some(e)) => {
                        r.handle(e);
                        true
                    }
                    Ok(None) => panic!("rust {}: node stopped", r.name),
                    Err(_) => false,
                },
            };
            if !more {
                panic!(
                    "{}: timed out waiting for message {:?}",
                    self.name(),
                    String::from_utf8_lossy(body)
                );
            }
        }
    }

    pub async fn ht_set(&mut self, key_seed: u8, salt: &[u8], value: &[u8]) {
        match self {
            TestNode::Go(g) => {
                g.cmd(&format!(
                    "ht-set {} {} {}",
                    hex::encode([key_seed; 32]),
                    hexs(salt),
                    hexs(value)
                ))
                .await
                .unwrap();
            }
            TestNode::Rust(r) => {
                r.node
                    .hash_table_set(
                        r.net,
                        key_from_seed(&[key_seed; 32]),
                        salt.to_vec(),
                        value.to_vec(),
                    )
                    .await
                    .unwrap();
            }
        }
    }

    /// One get from the network (the cache disabled).
    pub async fn ht_get(
        &mut self,
        key: &[u8],
        salt: &[u8],
        timeout: StdDuration,
    ) -> Option<Vec<u8>> {
        match self {
            TestNode::Go(g) => {
                let v = g
                    .cmd_timeout(
                        &format!(
                            "ht-get {} {} {}",
                            hexs(key),
                            hexs(salt),
                            timeout.as_millis()
                        ),
                        timeout + STEP,
                    )
                    .await
                    .unwrap();
                (v != "-").then(|| unhex(&v))
            }
            TestNode::Rust(r) => {
                let mut get = r
                    .node
                    .hash_table_get(
                        r.net,
                        key.to_vec(),
                        salt.to_vec(),
                        HashTableGetOptions {
                            disable_cache: true,
                            timeout: strims_base::time::Duration::from_millis(
                                timeout.as_millis() as i64
                            ),
                        },
                    )
                    .await
                    .unwrap();
                tokio::time::timeout(timeout + STEP, get.next())
                    .await
                    .expect("hash table get never closed")
            }
        }
    }

    pub async fn pi_publish(&mut self, key: &[u8], salt: &[u8], port: u16) {
        match self {
            TestNode::Go(g) => {
                g.cmd(&format!("pi-publish {} {} {port}", hexs(key), hexs(salt)))
                    .await
                    .unwrap();
            }
            TestNode::Rust(r) => {
                r.node
                    .peer_index_publish(r.net, key.to_vec(), salt.to_vec(), port)
                    .await
                    .unwrap();
            }
        }
    }

    /// Searches until `want` is found or `timeout` passes: every host found.
    pub async fn pi_search(
        &mut self,
        key: &[u8],
        salt: &[u8],
        want: HostId,
        timeout: StdDuration,
    ) -> Vec<(HostId, u16)> {
        match self {
            TestNode::Go(g) => {
                let v = g
                    .cmd_timeout(
                        &format!(
                            "pi-search {} {} {} {}",
                            hexs(key),
                            hexs(salt),
                            timeout.as_millis(),
                            id_hex(&want)
                        ),
                        timeout + STEP,
                    )
                    .await
                    .unwrap();
                v.split(',')
                    .filter(|s| *s != "-")
                    .map(|s| {
                        let (h, p) = s.split_once(':').unwrap();
                        (parse_id(h), p.parse().unwrap())
                    })
                    .collect()
            }
            TestNode::Rust(r) => {
                let mut search = r
                    .node
                    .peer_index_search(r.net, key.to_vec(), salt.to_vec())
                    .await
                    .unwrap();
                let mut found = Vec::new();
                let _ = tokio::time::timeout(timeout, async {
                    while let Some(h) = search.next().await {
                        found.push((h.host_id, h.port));
                        if h.host_id == want {
                            return;
                        }
                    }
                })
                .await;
                found
            }
        }
    }

    // --- swarms ---------------------------------------------------------------

    /// Publishes a swarm under the key from `key_seed`; its id.
    pub async fn create_swarm(&mut self, name: &str, key_seed: u8, live_window: usize) -> String {
        match self {
            TestNode::Go(g) => g
                .cmd(&format!(
                    "swarm-create {name} {} {live_window} mandatory",
                    hex::encode([key_seed; 32])
                ))
                .await
                .unwrap(),
            TestNode::Rust(r) => {
                let (key, id, writer) = r
                    .node
                    .create_swarm(WriterOptions {
                        swarm_options: swarm_options(live_window),
                        key: key_from_seed(&[key_seed; 32]),
                    })
                    .await
                    .unwrap();
                r.swarms.insert(
                    name.to_owned(),
                    RustSwarm {
                        key,
                        writer: Some(writer),
                        write: None,
                        read: None,
                    },
                );
                id.to_string()
            }
        }
    }

    /// Joins the swarm published under the key from `key_seed`; its id.
    pub async fn add_swarm(&mut self, name: &str, key_seed: u8, live_window: usize) -> String {
        let public = key_from_seed(&[key_seed; 32]).public;
        match self {
            TestNode::Go(g) => g
                .cmd(&format!(
                    "swarm-add {name} {} {live_window} mandatory",
                    hex::encode(&public)
                ))
                .await
                .unwrap(),
            TestNode::Rust(r) => {
                let id = SwarmId::new(&public);
                let key = r
                    .node
                    .add_swarm(id.clone(), swarm_options(live_window))
                    .await
                    .unwrap();
                r.swarms.insert(
                    name.to_owned(),
                    RustSwarm {
                        key,
                        writer: None,
                        write: None,
                        read: None,
                    },
                );
                id.to_string()
            }
        }
    }

    pub async fn run_swarm(&mut self, name: &str, peer: HostId, channel: u64, peer_channel: u64) {
        match self {
            TestNode::Go(g) => {
                g.cmd(&format!(
                    "swarm-run {name} {} {channel} {peer_channel}",
                    id_hex(&peer)
                ))
                .await
                .unwrap();
            }
            TestNode::Rust(r) => {
                let key = r.swarms[name].key;
                r.node
                    .run_swarm(peer, key, channel, peer_channel)
                    .await
                    .unwrap();
            }
        }
    }

    /// Starts writing `total` stream bytes at `rate` bytes per second.
    pub async fn start_write(&mut self, name: &str, total: u64, rate: u64) {
        match self {
            TestNode::Go(g) => {
                g.cmd(&format!("swarm-write {name} {total} {rate}"))
                    .await
                    .unwrap();
            }
            TestNode::Rust(r) => {
                let s = r.swarms.get_mut(name).unwrap();
                let mut writer = s.writer.take().expect("not a writer");
                s.write = Some(tokio::spawn(async move {
                    let start = Instant::now();
                    let mut buf = vec![0u8; UNIT];
                    let mut off = 0u64;
                    while off < total {
                        let due = start + StdDuration::from_secs_f64(off as f64 / rate as f64);
                        tokio::time::sleep_until(due).await;
                        let n = (total - off).min(UNIT as u64) as usize;
                        for (i, b) in buf[..n].iter_mut().enumerate() {
                            *b = stream_byte(off + i as u64);
                        }
                        writer.write(&buf[..n]).await.unwrap();
                        off += n as u64;
                    }
                    writer.flush().await.unwrap();
                }));
            }
        }
    }

    /// Starts reading the stream from the reader's start to `total`.
    pub async fn start_read(&mut self, name: &str, total: u64) {
        match self {
            TestNode::Go(g) => {
                g.cmd(&format!("swarm-read {name} {total}")).await.unwrap();
            }
            TestNode::Rust(r) => {
                let s = r.swarms.get_mut(name).unwrap();
                let mut reader = r.node.open_reader(s.key).await.unwrap();
                s.read = Some(tokio::spawn(async move {
                    let mut buf = vec![0u8; UNIT];
                    let mut h = Sha256::new();
                    let mut off = 0u64;
                    let mut mismatches = 0u64;
                    let mut error = None;
                    while off < total {
                        match reader.read(&mut buf).await {
                            Ok(n) => {
                                let n = n.min((total - off) as usize);
                                for (i, &b) in buf[..n].iter().enumerate() {
                                    if b != stream_byte(off + i as u64) {
                                        mismatches += 1;
                                    }
                                }
                                h.update(&buf[..n]);
                                off += n as u64;
                            }
                            Err(e) => {
                                error = Some(format!("{e:?} at {off}"));
                                break;
                            }
                        }
                    }
                    ReadResult {
                        start: 0,
                        len: off,
                        sha: hex::encode(h.finalize()),
                        mismatches,
                        error,
                    }
                }));
            }
        }
    }

    /// Waits until the stream read finishes.
    pub async fn read_result(&mut self, name: &str, deadline: Instant) -> ReadResult {
        match self {
            TestNode::Go(g) => {
                let n = name.to_owned();
                match g
                    .wait_until(
                        "read",
                        deadline,
                        |e| matches!(e, GoEvent::ReadDone(s, _) if *s == n),
                    )
                    .await
                {
                    GoEvent::ReadDone(_, r) => r,
                    _ => unreachable!(),
                }
            }
            TestNode::Rust(r) => {
                let who = r.name.clone();
                let task = r.swarms.get_mut(name).unwrap().read.take().unwrap();
                // Keep draining node events while the read runs.
                let mut task = task;
                loop {
                    tokio::select! {
                        res = &mut task => return res.unwrap(),
                        _ = tokio::time::sleep(StdDuration::from_millis(100)) => r.poll(),
                    }
                    assert!(
                        Instant::now() < deadline,
                        "rust {who}: timed out reading {name}"
                    );
                }
            }
        }
    }

    /// Waits until the writer has written everything.
    pub async fn written(&mut self, name: &str, deadline: Instant) {
        match self {
            TestNode::Go(g) => {
                let n = name.to_owned();
                g.wait_until(
                    "written",
                    deadline,
                    |e| matches!(e, GoEvent::Written(s) if *s == n),
                )
                .await;
            }
            TestNode::Rust(r) => {
                let task = r.swarms.get_mut(name).unwrap().write.take().unwrap();
                tokio::time::timeout_at(deadline, task)
                    .await
                    .expect("write timed out")
                    .unwrap();
            }
        }
    }

    /// Transfer errors reported so far.
    pub fn transfer_errors(&mut self) -> usize {
        match self {
            TestNode::Go(g) => g
                .events()
                .iter()
                .filter(|e| matches!(e, GoEvent::TransferError(..)))
                .count(),
            TestNode::Rust(r) => r
                .events()
                .iter()
                .filter(|e| matches!(e, NodeEvent::TransferError { .. }))
                .count(),
        }
    }
}

/// Links `a` to `b` (a dials b) and waits for both sides to see the peer.
pub async fn link(nodes: &mut [TestNode], a: usize, b: usize, t: Transport) {
    let uri = nodes[b].uri(t);
    let b_id = nodes[b].host_id();
    let a_id = nodes[a].host_id();
    assert_eq!(nodes[a].dial(&uri).await, b_id);
    nodes[a].wait_peer(b_id).await;
    nodes[b].wait_peer(a_id).await;
}

/// Polls every node until `done` holds or `timeout` passes; whether it held.
pub async fn settle(
    nodes: &mut [TestNode],
    timeout: StdDuration,
    mut done: impl FnMut(&mut [TestNode]) -> bool,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        for n in nodes.iter_mut() {
            n.poll();
        }
        if done(nodes) {
            return true;
        }
        if Instant::now() > deadline {
            return false;
        }
        tokio::time::sleep(StdDuration::from_millis(50)).await;
    }
}
