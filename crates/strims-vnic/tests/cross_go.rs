// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Live runs of a [`Host`] against a Go `vnic.Host` (`golden/main.go`): over
//! the helper's stdin/stdout (plain, and with the TCP mux init and AES
//! handshake in both roles) and over TCP sockets through the Go TCP
//! listener and dialer. Both sides complete the `PeerInit` handshake, then
//! exchange frames on several ports and through a channel pair (see
//! `setupPeer` in the helper). Over TCP, an idle period longer than the Go
//! peer's read deadline checks that the link survives with keepalives and
//! closes without them. Skipped when `go` isn't on PATH.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    // These tests drive a real subprocess and sockets, so they need threads
    // and the wall clock.
    clippy::disallowed_methods
)]

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, SystemTime};

use strims_base::rng::det_rng;
use strims_base::time::{Duration as VDuration, Time};
use strims_crypto::key_from_seed;
use strims_vnic::{
    Event, Fill, Host, HostConfig, LinkConfig, LinkKind, PeerKey, TcpMuxAcceptor, TcpUri, WriterId,
};

const GO_SEED: u8 = 0x42;
const RUST_SEED: u8 = 0x21;
/// The Go helper's stdio link MTU.
const GO_STDIO_MTU: usize = 1200;
/// The Rust side's MTU on stdio links, so frames differ in size each way.
const RUST_STDIO_MTU: usize = 999;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn go_available() -> bool {
    match Command::new("go").arg("version").output() {
        Ok(o) if o.status.success() => true,
        _ => {
            eprintln!("skipping: `go` is not on PATH");
            false
        }
    }
}

fn now() -> Time {
    Time::from_system(SystemTime::now())
}

fn pattern(n: usize, salt: u8) -> Vec<u8> {
    (0..n).map(|i| ((i * 7) as u8) ^ salt).collect()
}

fn hex(b: &[u8]) -> String {
    hex::encode(b)
}

/// The helper process. Its stderr lines arrive on `err`.
struct Go {
    child: Child,
    err: Receiver<String>,
    /// Lines read before `run` (the listener's uri).
    early: Vec<String>,
}

/// The helper's stdin and a channel of its stdout reads.
type StdioLink = (ChildStdin, Receiver<Vec<u8>>);

fn spawn_go(args: &[String], stdio_link: bool) -> (Go, Option<StdioLink>) {
    let mut child = Command::new("go")
        .args(["run", "-tags", "golden", "./crates/strims-vnic/golden"])
        .args(args)
        .current_dir(repo_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn go");
    let stderr = child.stderr.take().unwrap();
    let (etx, erx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let Ok(line) = line else { return };
            if etx.send(line).is_err() {
                return;
            }
        }
    });
    let stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let link = stdio_link.then(|| (stdin, spawn_reader(move |b| stdout.read(b))));
    (
        Go {
            child,
            err: erx,
            early: vec![],
        },
        link,
    )
}

fn spawn_reader(
    mut read: impl FnMut(&mut [u8]) -> std::io::Result<usize> + Send + 'static,
) -> Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        // Odd buffer size so frames straddle reads.
        let mut buf = vec![0u8; 1777];
        loop {
            match read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        return;
                    }
                }
            }
        }
    });
    rx
}

/// One side of a byte stream to the Go peer.
struct Wire {
    tx: Box<dyn Write>,
    rx: Receiver<Vec<u8>>,
}

impl Wire {
    fn recv(&self) -> Option<Vec<u8>> {
        // The first bytes wait for `go run` to compile the helper.
        match self.rx.recv_timeout(Duration::from_secs(600)) {
            Ok(b) => Some(b),
            Err(RecvTimeoutError::Disconnected) => None,
            Err(RecvTimeoutError::Timeout) => panic!("timed out waiting for the Go peer"),
        }
    }
}

const ECHO: u32 = 1001;
const GO3000: u32 = 3000;
const CH0: u32 = 5001;
const CH1: u32 = 5000;

/// The Rust half of the script in the helper's `setupPeer`.
struct Script {
    host: Host<u32>,
    link: PeerKey,
    peer: Option<PeerKey>,
    queues: Vec<(WriterId, VecDeque<Vec<u8>>)>,
    done: Option<WriterId>,
    echo_sent: Vec<Vec<u8>>,
    ch_sent: Vec<Vec<u8>>,
    to_2000: Vec<Vec<u8>>,
    echo_got: Vec<Vec<u8>>,
    ch_got: Vec<Vec<u8>>,
    go3000_got: Vec<Vec<u8>>,
}

impl Script {
    fn new(host: Host<u32>, link: PeerKey) -> Script {
        let sizes = [1usize, 7, 100, 995, 333, 64, 995];
        Script {
            host,
            link,
            peer: None,
            queues: vec![],
            done: None,
            echo_sent: sizes
                .iter()
                .enumerate()
                .map(|(i, &n)| pattern(n, i as u8))
                .collect(),
            ch_sent: [5usize, 200, 995]
                .iter()
                .enumerate()
                .map(|(i, &n)| pattern(n, 0x40 + i as u8))
                .collect(),
            to_2000: [3usize, 900, 10]
                .iter()
                .enumerate()
                .map(|(i, &n)| pattern(n, 0x80 + i as u8))
                .collect(),
            echo_got: vec![],
            ch_got: vec![],
            go3000_got: vec![],
        }
    }

    fn on_peer(&mut self, peer: PeerKey) {
        let h = &mut self.host;
        let root = h.qos_root();
        h.set_handler(peer, 1001, ECHO).unwrap();
        h.set_handler(peer, 3000, GO3000).unwrap();
        // Writes 5000 and reads 5001, the helper's reply port.
        let (ch0, _ch1) = h.channel_pair(peer, 5000, 5001, root, CH0, CH1).unwrap();
        let w1000 = h.open_writer(peer, 1000, root, 1000).unwrap();
        let w2000 = h.open_writer(peer, 2000, root, 2000).unwrap();
        self.done = Some(h.open_writer(peer, 9999, root, 9999).unwrap());
        self.queues = vec![
            (w1000, self.echo_sent.iter().cloned().collect()),
            (w2000, self.to_2000.iter().cloned().collect()),
            (ch0, self.ch_sent.iter().cloned().collect()),
            (self.done.unwrap(), VecDeque::new()),
        ];
        for (w, q) in &self.queues {
            if !q.is_empty() {
                h.set_backlog(*w).unwrap();
            }
        }
        self.peer = Some(peer);
    }

    fn input(&mut self, data: &mut [u8]) {
        let mut off = 0;
        while off < data.len() {
            let Script {
                host,
                echo_got,
                ch_got,
                go3000_got,
                ..
            } = self;
            off += host
                .handle_link_input(
                    self.link,
                    now(),
                    &mut data[off..],
                    &mut |f| match f.handler {
                        ECHO => echo_got.push(f.body.to_vec()),
                        CH0 => ch_got.push(f.body.to_vec()),
                        GO3000 => go3000_got.push(f.body.to_vec()),
                        h => panic!("unexpected frame for handler {h}"),
                    },
                )
                .expect("link input");
            while let Some(e) = self.host.poll_event() {
                match e {
                    Event::PeerAdded { peer, .. } => self.on_peer(peer),
                    e => panic!("unexpected event {e:?}"),
                }
            }
        }
    }

    fn finished_receiving(&self, go_mtu: usize) -> bool {
        self.echo_got.len() == self.echo_sent.len()
            && self.ch_got.len() == self.ch_sent.len()
            && self.go3000_got.len() == 5 + 3000usize.div_ceil(go_mtu - 4)
    }

    /// Fills frames (and runs vnic's timers) and sends everything pending.
    fn flush(&mut self, tx: &mut dyn Write) {
        let queues = &mut self.queues;
        self.host.poll_egress(now(), &mut |f| {
            let q = &mut queues.iter_mut().find(|(w, _)| *w == f.writer).unwrap().1;
            let Some(m) = q.pop_front() else {
                return Fill::NONE;
            };
            f.body[..m.len()].copy_from_slice(&m);
            Fill {
                len: m.len(),
                more: !q.is_empty(),
            }
        });
        loop {
            let data = self.host.poll_link_transmit(self.link).unwrap();
            if data.is_empty() {
                break;
            }
            let n = data.len();
            tx.write_all(data).unwrap();
            self.host.advance_link_transmit(self.link, n).unwrap();
        }
        tx.flush().unwrap();
    }

    fn send_done(&mut self) {
        let w = self.done.unwrap();
        self.queues
            .iter_mut()
            .find(|(x, _)| *x == w)
            .unwrap()
            .1
            .push_back(b"done".to_vec());
        self.host.set_backlog(w).unwrap();
    }
}

fn rust_host() -> Host<u32> {
    let config = HostConfig {
        label: "rust-peer".to_owned(),
        ..HostConfig::default()
    };
    Host::new(key_from_seed(&[RUST_SEED; 32]), config, det_rng(77)).unwrap()
}

/// The helper's stderr until it closes.
fn stderr_lines(go: &Go) -> Vec<String> {
    let mut lines = go.early.clone();
    while let Ok(l) = go.err.recv_timeout(Duration::from_secs(10)) {
        lines.push(l);
    }
    lines
}

/// Runs the script until the Go side has everything and we have all its
/// frames, then sends "done" and checks what Go reported.
fn run(mut script: Script, mut wire: Wire, mut go: Go, go_mtu: usize) {
    exchange(&mut script, &mut wire, &mut go, go_mtu);
    finish(script, wire, go, go_mtu);
}

/// Runs the script until we have all of Go's frames and Go has ours.
fn exchange(script: &mut Script, wire: &mut Wire, go: &mut Go, go_mtu: usize) {
    loop {
        script.flush(&mut wire.tx);
        if script.finished_receiving(go_mtu) {
            return;
        }
        let Some(mut chunk) = wire.recv() else {
            let _ = go.child.wait();
            panic!("go peer closed early: {:#?}", stderr_lines(go));
        };
        script.input(&mut chunk);
    }
}

/// Leaves the link idle for `d` (no frames from either script), running
/// vnic's timers and feeding whatever arrives. Returns how long after the
/// start the Go peer closed the link, if it did.
fn idle(script: &mut Script, wire: &mut Wire, d: Duration) -> Option<Duration> {
    let start = now();
    let elapsed = || Duration::from_millis((now() - start).as_millis().max(0) as u64);
    loop {
        script.host.handle_timeout(now());
        script.flush(&mut wire.tx);
        let left = d.saturating_sub(elapsed());
        if left.is_zero() {
            return None;
        }
        let wake = script
            .host
            .poll_timeout()
            .map(|t| Duration::from_millis((t - now()).as_millis().max(1) as u64))
            .unwrap_or(left)
            .min(left);
        match wire.rx.recv_timeout(wake) {
            Ok(mut chunk) => script.input(&mut chunk),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Some(elapsed()),
        }
    }
}

/// Sends "done" and checks what both sides saw.
fn finish(mut script: Script, mut wire: Wire, mut go: Go, go_mtu: usize) {
    script.send_done();
    script.flush(&mut wire.tx);
    let status = go.child.wait().unwrap();
    let lines = stderr_lines(&go);
    assert!(status.success(), "go peer failed: {lines:#?}");

    let peer = script.peer.unwrap();
    let p = script.host.peer(peer).unwrap();
    let go_profile = key_from_seed(&[GO_SEED; 32]).public;
    assert_eq!(p.certificate().key, go_profile);
    assert_eq!(p.node_version(), "dev");
    // Go's view of both hosts.
    assert!(
        lines.contains(&format!(
            "self {} {}",
            hex(&p.host_id().to_bytes()),
            hex(&go_profile)
        )),
        "{lines:#?}"
    );
    assert!(lines.contains(&format!(
        "peer {} {}",
        hex(&script.host.id().to_bytes()),
        hex(&script.host.profile_key().public)
    )));
    let recv: Vec<String> = lines
        .iter()
        .filter_map(|l| l.strip_prefix("recv 2000 ").map(str::to_owned))
        .collect();
    assert_eq!(
        recv,
        script.to_2000.iter().map(|m| hex(m)).collect::<Vec<_>>()
    );
    assert!(lines.iter().any(|l| l == "done"));
    assert!(!lines.iter().any(|l| l == "closed"), "{lines:#?}");

    // What Go sent back.
    assert_eq!(script.echo_got, script.echo_sent);
    let reversed: Vec<Vec<u8>> = script
        .ch_sent
        .iter()
        .map(|m| m.iter().rev().copied().collect())
        .collect();
    assert_eq!(script.ch_got, reversed);
    let names: Vec<Vec<u8>> = (0..5)
        .map(|i| format!("go-3000-{i}").into_bytes())
        .collect();
    assert_eq!(script.go3000_got[..5], names[..]);
    // One 3000-byte Write, cut into frames of at most the Go MTU.
    let big = &script.go3000_got[5..];
    assert!(big.iter().all(|f| f.len() <= go_mtu - 4));
    assert_eq!(big.concat(), pattern(3000, 9));
}

fn stdio_args(extra: &[String]) -> Vec<String> {
    let mut a = vec![
        "live".to_owned(),
        format!("-seed={GO_SEED}"),
        format!("-mtu={GO_STDIO_MTU}"),
    ];
    a.extend_from_slice(extra);
    a
}

#[test]
fn go_peer_over_stdio() {
    if !go_available() {
        return;
    }
    let (go, link) = spawn_go(&stdio_args(&[]), true);
    let (stdin, rx) = link.unwrap();
    let mut host = rust_host();
    let link = host
        .add_link(now(), LinkConfig::new(LinkKind::Other, RUST_STDIO_MTU))
        .unwrap();
    let wire = Wire {
        tx: Box::new(stdin),
        rx,
    };
    run(Script::new(host, link), wire, go, GO_STDIO_MTU);
}

/// Rust dials (mux init + AES), Go accepts.
#[test]
fn go_accepts_aes_link_over_stdio() {
    if !go_available() {
        return;
    }
    let (go, link) = spawn_go(&stdio_args(&["-aes=accept".to_owned()]), true);
    let (stdin, rx) = link.unwrap();
    let mut host = rust_host();
    let go_profile: [u8; 32] = key_from_seed(&[GO_SEED; 32]).public.try_into().unwrap();
    let link = host
        .add_link(
            now(),
            LinkConfig::tcp_dial(&go_profile).with_mtu(RUST_STDIO_MTU),
        )
        .unwrap();
    let wire = Wire {
        tx: Box::new(stdin),
        rx,
    };
    run(Script::new(host, link), wire, go, GO_STDIO_MTU);
}

/// Reads the dialer's `TCPMuxInit`, then adds an accepting AES link and
/// feeds it what followed.
fn accept_mux(host: &mut Host<u32>, wire: &Wire, config: LinkConfig) -> (PeerKey, Vec<u8>) {
    let mut acc = TcpMuxAcceptor::new();
    loop {
        let chunk = wire.recv().expect("go closed before mux init");
        let (n, key) = acc.handle_input(&chunk).unwrap();
        let mut rest = chunk[n..].to_vec();
        if let Some(key) = key {
            assert_eq!(key.as_slice(), host.profile_key().public.as_slice());
            let link = host.add_link(now(), config).unwrap();
            // Bytes after the init (the AES handshake) belong to the link.
            return (link, std::mem::take(&mut rest));
        }
        assert!(rest.is_empty());
    }
}

/// Go dials (mux init + AES), Rust accepts.
#[test]
fn go_dials_aes_link_over_stdio() {
    if !go_available() {
        return;
    }
    let mut host = rust_host();
    let peer = format!("-peer={}", hex(&host.profile_key().public));
    let (go, link) = spawn_go(&stdio_args(&["-aes=dial".to_owned(), peer]), true);
    let (stdin, rx) = link.unwrap();
    let wire = Wire {
        tx: Box::new(stdin),
        rx,
    };
    let (link, mut rest) = accept_mux(
        &mut host,
        &wire,
        LinkConfig::tcp_accept().with_mtu(RUST_STDIO_MTU),
    );
    let mut script = Script::new(host, link);
    script.input(&mut rest);
    run(script, wire, go, GO_STDIO_MTU);
}

fn tcp_wire(stream: TcpStream) -> Wire {
    stream.set_nodelay(true).unwrap();
    let mut read = stream.try_clone().unwrap();
    Wire {
        tx: Box::new(stream),
        rx: spawn_reader(move |b| read.read(b)),
    }
}

/// Go's `TCPMux` and `tcpInterface` listen; Rust dials the advertised URI.
#[test]
fn rust_dials_go_tcp_listener() {
    if !go_available() {
        return;
    }
    let (mut go, _) = spawn_go(
        &["tcp-listen".to_owned(), format!("-seed={GO_SEED}")],
        false,
    );
    let uri = loop {
        let line = go
            .err
            .recv_timeout(Duration::from_secs(600))
            .expect("go listener uri");
        let uri = line.strip_prefix("uri ").map(str::to_owned);
        go.early.push(line);
        if let Some(u) = uri {
            break u;
        }
    };
    let target = TcpUri::parse(&uri).unwrap();
    assert_eq!(
        target.peer_key.as_slice(),
        key_from_seed(&[GO_SEED; 32]).public.as_slice()
    );
    let wire = tcp_wire(TcpStream::connect(&target.host).unwrap());
    let mut host = rust_host();
    let link = host
        .add_link(now(), LinkConfig::tcp_dial(&target.peer_key))
        .unwrap();
    run(Script::new(host, link), wire, go, strims_vnic::tcp::TCP_MTU);
}

/// Rust listens; Go's `Host.Dial` connects through `DialTCPMux`.
#[test]
fn go_dials_rust_tcp_listener() {
    if !go_available() {
        return;
    }
    let mut host = rust_host();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = TcpUri {
        host: listener.local_addr().unwrap().to_string(),
        peer_key: host.profile_key().public.as_slice().try_into().unwrap(),
    }
    .to_uri();
    let (go, _) = spawn_go(
        &[
            "tcp-dial".to_owned(),
            format!("-seed={GO_SEED}"),
            format!("-uri={uri}"),
        ],
        false,
    );
    let (stream, _) = listener.accept().unwrap();
    let wire = tcp_wire(stream);
    let (link, mut rest) = accept_mux(&mut host, &wire, LinkConfig::tcp_accept());
    let mut script = Script::new(host, link);
    script.input(&mut rest);
    run(script, wire, go, strims_vnic::tcp::TCP_MTU);
}

/// Go's per-read deadline in the idle tests, and the idle period: well past
/// the deadline.
const GO_READ_TIMEOUT_MS: u64 = 1500;
const IDLE: Duration = Duration::from_millis(3 * GO_READ_TIMEOUT_MS);

fn go_tcp_listener(read_timeout_ms: u64) -> (Go, String) {
    let (mut go, _) = spawn_go(
        &[
            "tcp-listen".to_owned(),
            format!("-seed={GO_SEED}"),
            format!("-read-timeout={read_timeout_ms}ms"),
        ],
        false,
    );
    let uri = loop {
        let line = go
            .err
            .recv_timeout(Duration::from_secs(600))
            .expect("go listener uri");
        let uri = line.strip_prefix("uri ").map(str::to_owned);
        go.early.push(line);
        if let Some(u) = uri {
            break u;
        }
    };
    (go, uri)
}

/// Rust dials Go's TCP listener, runs the script, then leaves the link idle
/// for longer than Go's read deadline. Returns when Go closed the link, if
/// it did, and the parts to finish with.
fn idle_against_go_listener(keepalive: Option<VDuration>) -> (Option<Duration>, Script, Wire, Go) {
    let (mut go, uri) = go_tcp_listener(GO_READ_TIMEOUT_MS);
    let target = TcpUri::parse(&uri).unwrap();
    let mut wire = tcp_wire(TcpStream::connect(&target.host).unwrap());
    let mut host = rust_host();
    host.set_keepalive_interval(keepalive);
    let link = host
        .add_link(now(), LinkConfig::tcp_dial(&target.peer_key))
        .unwrap();
    let mut script = Script::new(host, link);
    exchange(&mut script, &mut wire, &mut go, strims_vnic::tcp::TCP_MTU);
    let closed = idle(&mut script, &mut wire, IDLE);
    (closed, script, wire, go)
}

/// With keepalives every fifth of the Go peer's read deadline, the link
/// survives an idle period of three deadlines and still carries frames.
#[test]
fn keepalives_keep_a_go_tcp_listener_linked() {
    if !go_available() {
        return;
    }
    let interval = VDuration::from_millis(GO_READ_TIMEOUT_MS as i64 / 5);
    let (closed, script, wire, go) = idle_against_go_listener(Some(interval));
    assert_eq!(closed, None, "go closed the idle link");
    let sent = script.host.metrics().keepalive_write_count;
    assert!(sent >= 10, "{sent} keepalives");
    // `finish` checks that the Go peer got "done" and never closed.
    assert_eq!(script.host.metrics().keepalive_read_count, 0);
    assert!(
        !script
            .host
            .peer(script.peer.unwrap())
            .unwrap()
            .sends_keepalives()
    );
    finish(script, wire, go, strims_vnic::tcp::TCP_MTU);
}

/// Control: without keepalives the idle link closes at the Go peer's read
/// deadline.
#[test]
fn go_tcp_listener_closes_an_idle_link_without_keepalives() {
    if !go_available() {
        return;
    }
    let (closed, script, _wire, mut go) = idle_against_go_listener(None);
    let after = closed.expect("go kept the idle link");
    // About one deadline after our last write, just before the idle began.
    assert!(
        after >= Duration::from_millis(GO_READ_TIMEOUT_MS / 2) && after < IDLE,
        "{after:?}"
    );
    assert_eq!(script.host.metrics().keepalive_write_count, 0);
    let status = go.child.wait().unwrap();
    let lines = stderr_lines(&go);
    assert!(!status.success());
    assert!(lines.iter().any(|l| l == "closed"), "{lines:#?}");
}

/// The same from the accepting side: the Go peer dials, and keepalives go
/// out on the accepted (AES) link.
#[test]
fn keepalives_keep_a_go_dialer_linked() {
    if !go_available() {
        return;
    }
    let mut host = rust_host();
    host.set_keepalive_interval(Some(VDuration::from_millis(GO_READ_TIMEOUT_MS as i64 / 5)));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = TcpUri {
        host: listener.local_addr().unwrap().to_string(),
        peer_key: host.profile_key().public.as_slice().try_into().unwrap(),
    }
    .to_uri();
    let (mut go, _) = spawn_go(
        &[
            "tcp-dial".to_owned(),
            format!("-seed={GO_SEED}"),
            format!("-uri={uri}"),
            format!("-read-timeout={GO_READ_TIMEOUT_MS}ms"),
        ],
        false,
    );
    let (stream, _) = listener.accept().unwrap();
    let mut wire = tcp_wire(stream);
    let (link, mut rest) = accept_mux(&mut host, &wire, LinkConfig::tcp_accept());
    let mut script = Script::new(host, link);
    script.input(&mut rest);
    exchange(&mut script, &mut wire, &mut go, strims_vnic::tcp::TCP_MTU);
    assert_eq!(
        idle(&mut script, &mut wire, IDLE),
        None,
        "go closed the idle link"
    );
    assert!(script.host.metrics().keepalive_write_count >= 10);
    finish(script, wire, go, strims_vnic::tcp::TCP_MTU);
}
