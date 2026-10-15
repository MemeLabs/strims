// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Live interop with pion: the Go helper (`golden/main.go`) and a Rust
//! [`WebRtcEndpoint`] on real loopback UDP, signalling over the helper's
//! stdin/stdout, with Rust as offerer and as answerer.
//!
//! - `raw`: pion configured as `vnic.NewWebRTCInterface` (UDP mux, detached
//!   pre-negotiated channel) against the endpoint alone; 4 MiB each way,
//!   byte-exact, then a close from each side.
//! - `vnic`: Go's real `NewWebRTCInterface` and `vnic.Host` against a Rust
//!   `strims_vnic::Host` composed with the endpoint as the crate docs'
//!   driver contract describes: `PeerInit` both ways, then echoed frames up
//!   to the MTU and a 2 MiB stream from Go's `FrameWriter`.
//!
//! Skipped when `go` isn't on PATH.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    // A real subprocess, sockets and the wall clock.
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::task::Poll;
use std::time::{Duration as StdDuration, Instant as StdInstant, SystemTime};

use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::key_from_seed;
use strims_vnic::{Fill, Host, HostConfig, LinkConfig, PeerKey, WriterId};
use strims_webrtc::{
    CloseReason, ConnId, DATA_CHANNEL_MTU, EndpointConfig, Event, RemoteDescriptionAction, SdpType,
    SessionDescription, WebRtcEndpoint,
};

/// Stream seeds, shared with the helper.
const GO_STREAM_SEED: u64 = 0x60;
const RUST_STREAM_SEED: u64 = 0x52;

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

/// The helper's splitmix64 stream.
fn stream(seed: u64, n: usize) -> Vec<u8> {
    let mut state = seed;
    let mut out = Vec::with_capacity(n + 8);
    while out.len() < n {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        out.extend_from_slice(&(z ^ (z >> 31)).to_le_bytes());
    }
    out.truncate(n);
    out
}

// ---- the helper process ------------------------------------------------------

struct GoPeer {
    child: Child,
    stdin: ChildStdin,
    out: Receiver<String>,
    err: Receiver<String>,
    log: Vec<String>,
}

fn lines(r: impl std::io::Read + Send + 'static) -> Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(r).lines() {
            let Ok(line) = line else { return };
            if tx.send(line).is_err() {
                return;
            }
        }
    });
    rx
}

impl GoPeer {
    fn spawn(args: &[&str]) -> GoPeer {
        let mut child = Command::new("go")
            .args(["run", "-tags", "golden", "./crates/strims-webrtc/golden"])
            .args(args)
            .current_dir(repo_root())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn go");
        let stdin = child.stdin.take().unwrap();
        let out = lines(child.stdout.take().unwrap());
        let err = lines(child.stderr.take().unwrap());
        let mut go = GoPeer {
            child,
            stdin,
            out,
            err,
            log: vec![],
        };
        // `go run` compiles pion first.
        let ready = go.out.recv_timeout(StdDuration::from_secs(600));
        assert_eq!(ready.as_deref(), Ok("ready"), "{:#?}", go.drain_log());
        go
    }

    fn send(&mut self, line: &str) {
        writeln!(self.stdin, "{line}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn drain_log(&mut self) -> Vec<String> {
        while let Ok(l) = self.err.try_recv() {
            self.log.push(l);
        }
        self.log.clone()
    }

    fn has(&mut self, line: &str) -> bool {
        self.drain_log().iter().any(|l| l == line)
    }

    fn wait(mut self) -> Vec<String> {
        let deadline = StdInstant::now() + StdDuration::from_secs(30);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                // Let the stderr thread finish.
                while let Ok(l) = self.err.recv_timeout(StdDuration::from_secs(2)) {
                    self.log.push(l);
                }
                assert!(status.success(), "go helper failed: {:#?}", self.log);
                return self.log;
            }
            assert!(
                StdInstant::now() < deadline,
                "go helper hangs: {:#?}",
                self.drain_log()
            );
            std::thread::sleep(StdDuration::from_millis(20));
        }
    }
}

// ---- a tiny real-socket driver -------------------------------------------------

struct Live {
    sock: UdpSocket,
    local: SocketAddr,
    ep: WebRtcEndpoint,
    buf: Vec<u8>,
}

impl Live {
    fn new() -> Live {
        let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
        let local = sock.local_addr().unwrap();
        let config = EndpointConfig {
            ice_servers: Some(vec![]),
            ..EndpointConfig::default()
        };
        let mut ep = WebRtcEndpoint::new(now(), config, det_rng(99));
        ep.add_local_addr(local);
        Live {
            sock,
            local,
            ep,
            buf: vec![0; 65536],
        }
    }

    fn flush(&mut self) {
        while let Some(t) = self.ep.poll_udp_transmit() {
            assert_eq!(t.source, self.local);
            self.sock.send_to(&t.contents, t.destination).unwrap();
        }
    }

    /// One round: send, wait for datagrams until the next deadline (at most
    /// 5 ms), run due timers, send again; returns the events.
    fn pump(&mut self) -> Vec<Event> {
        self.flush();
        let n = now();
        let wait = self
            .ep
            .poll_timeout()
            .map_or(Duration::from_millis(5), |t| t - n)
            .clamp(Duration::from_micros(100), Duration::from_millis(5));
        self.sock.set_read_timeout(wait.to_std()).unwrap();
        for _ in 0..256 {
            match self.sock.recv_from(&mut self.buf) {
                Ok((len, src)) => {
                    self.ep
                        .handle_udp_input(now(), src, self.local, &self.buf[..len]);
                    self.sock
                        .set_read_timeout(Some(StdDuration::from_micros(1)))
                        .unwrap();
                }
                Err(_) => break,
            }
        }
        let n = now();
        if self.ep.poll_timeout().is_some_and(|t| t <= n) {
            self.ep.handle_timeout(n);
        }
        self.flush();
        std::iter::from_fn(|| self.ep.poll_event()).collect()
    }

    fn local_description(&mut self, conn: ConnId) -> SessionDescription {
        loop {
            if let Poll::Ready(r) = self.ep.poll_local_description(now(), conn) {
                return r.unwrap();
            }
            self.pump();
        }
    }
}

/// Signals with the helper; returns our connection.
fn signal(live: &mut Live, go: &mut GoPeer, rust_offers: bool) -> ConnId {
    let remote = |go: &mut GoPeer| {
        let line = go
            .out
            .recv_timeout(StdDuration::from_secs(60))
            .unwrap_or_else(|_| panic!("no description from go: {:#?}", go.drain_log()));
        let json = line.strip_prefix("desc ").expect("desc line");
        SessionDescription::from_json(json).unwrap()
    };
    let conn;
    if rust_offers {
        conn = live.ep.create_connection(now()).unwrap();
        let offer = live.local_description(conn);
        assert_eq!(offer.sdp_type, SdpType::Offer);
        go.send(&format!("desc {}", offer.to_json()));
        let answer = remote(go);
        assert_eq!(answer.sdp_type, SdpType::Answer);
        assert_eq!(
            live.ep
                .set_remote_description(now(), conn, &answer)
                .unwrap(),
            RemoteDescriptionAction::AwaitLink
        );
    } else {
        let offer = remote(go);
        assert_eq!(offer.sdp_type, SdpType::Offer);
        conn = live.ep.create_connection(now()).unwrap();
        assert_eq!(
            live.ep.set_remote_description(now(), conn, &offer).unwrap(),
            RemoteDescriptionAction::AwaitInBackground
        );
        let answer = live.local_description(conn);
        assert_eq!(answer.sdp_type, SdpType::Answer);
        go.send(&format!("desc {}", answer.to_json()));
    }
    conn
}

fn deadline(secs: u64) -> StdInstant {
    StdInstant::now() + StdDuration::from_secs(secs)
}

// ---- raw ---------------------------------------------------------------------

const RAW_SIZE: usize = 4 << 20;

fn raw(rust_offers: bool, closer: &str) {
    if !go_available() {
        return;
    }
    let size = RAW_SIZE.to_string();
    let role = if rust_offers { "answer" } else { "offer" };
    let mut go = GoPeer::spawn(&["raw", "-role", role, "-size", &size, "-closer", closer]);
    let mut live = Live::new();
    let conn = signal(&mut live, &mut go, rust_offers);

    let out = stream(RUST_STREAM_SEED, RAW_SIZE);
    let want = stream(GO_STREAM_SEED, RAW_SIZE);
    let mut sent = 0;
    let mut got = Vec::with_capacity(RAW_SIZE);
    let mut open = false;
    let mut blocked = false;
    let mut largest = 0;
    let end = deadline(120);
    while got.len() < RAW_SIZE || !go.has(&format!("recv-ok {RAW_SIZE} {DATA_CHANNEL_MTU}")) {
        assert!(
            StdInstant::now() < end,
            "transfer timed out: {:#?}",
            go.drain_log()
        );
        for e in live.pump() {
            match e {
                Event::Open { conn: c, mtu } => {
                    assert_eq!((c, mtu), (conn, DATA_CHANNEL_MTU));
                    open = true;
                }
                Event::Data { data, .. } => {
                    largest = largest.max(data.len());
                    assert!(
                        got.len() + data.len() <= RAW_SIZE
                            && data[..] == want[got.len()..got.len() + data.len()],
                        "go→rust bytes differ at {}",
                        got.len()
                    );
                    got.extend_from_slice(&data);
                }
                Event::Writable(_) => blocked = false,
                Event::LocalDescription(_) => {}
                Event::Closed { reason, .. } => {
                    panic!("closed early: {reason:?} {:#?}", go.drain_log())
                }
            }
        }
        while open && !blocked && sent < RAW_SIZE {
            let n = live.ep.link_write(conn, &out[sent..]).unwrap();
            sent += n;
            if sent < RAW_SIZE {
                blocked = true;
            }
        }
    }
    // Full 64 KiB messages must cross both ways.
    assert_eq!(largest, DATA_CHANNEL_MTU);

    let end = deadline(30);
    let reason = if closer == "rust" {
        live.ep.close(now(), conn);
        loop {
            assert!(StdInstant::now() < end, "close timed out");
            if let Some(r) = live.pump().into_iter().find_map(|e| match e {
                Event::Closed { reason, .. } => Some(reason),
                _ => None,
            }) {
                break r;
            }
        }
    } else {
        go.send("got-all");
        let r = loop {
            assert!(
                StdInstant::now() < end,
                "remote close never seen: {:#?}",
                go.drain_log()
            );
            if let Some(r) = live.pump().into_iter().find_map(|e| match e {
                Event::Closed { reason, .. } => Some(reason),
                _ => None,
            }) {
                break r;
            }
        };
        go.send("bye");
        r
    };
    // Keep answering until the helper exits (its close may need replies).
    let mut log = None;
    let end = deadline(30);
    while log.is_none() {
        assert!(StdInstant::now() < end);
        live.pump();
        if go.child.try_wait().unwrap().is_some() {
            log = Some(go.wait());
            break;
        }
    }
    let log = log.unwrap();
    assert!(log.iter().any(|l| l == "open"), "{log:#?}");
    assert!(log.iter().any(|l| l == "done"), "{log:#?}");
    if closer == "rust" {
        assert_eq!(reason, CloseReason::Local);
        // A local close must end the peer's detached channel.
        assert!(log.iter().any(|l| l.starts_with("eof ")), "{log:#?}");
    } else {
        // A stream reset from the peer must close the whole connection.
        assert_eq!(reason, CloseReason::Remote);
    }
    assert_eq!(live.ep.connections().count(), 0);
}

#[test]
fn pion_raw_rust_offers_rust_closes() {
    raw(true, "rust");
}

#[test]
fn pion_raw_rust_answers_go_closes() {
    raw(false, "go");
}

#[test]
fn pion_raw_rust_offers_go_closes() {
    raw(true, "go");
}

#[test]
fn pion_raw_rust_answers_rust_closes() {
    raw(false, "rust");
}

// ---- vnic --------------------------------------------------------------------

const VNIC_GO_SIZE: usize = 2 << 20;
const ECHO: u32 = 1001;
const GO3000: u32 = 3000;

/// The vnic host side of a link: what a node driver does per connection.
struct VnicSide {
    host: Host<u32>,
    link: Option<PeerKey>,
    peer: Option<PeerKey>,
    echo: Option<WriterId>,
    done: Option<WriterId>,
    echo_out: VecDeque<Vec<u8>>,
    echo_sent: Vec<Vec<u8>>,
    echo_got: Vec<Vec<u8>>,
    go_stream: Vec<u8>,
    done_queued: bool,
}

impl VnicSide {
    fn new() -> VnicSide {
        let config = HostConfig {
            label: "rust-webrtc".to_owned(),
            ..HostConfig::default()
        };
        let host = Host::new(key_from_seed(&[0x21; 32]), config, det_rng(77)).unwrap();
        // Frames from 1 byte to the largest body a 64 KiB link carries.
        let max = DATA_CHANNEL_MTU - 4;
        let sizes = [1usize, 7, 1000, max, 4096, 30_000, max, 65_000, 12, max];
        let all = stream(RUST_STREAM_SEED, sizes.iter().sum());
        let mut off = 0;
        let echo_sent: Vec<Vec<u8>> = sizes
            .iter()
            .map(|&n| {
                off += n;
                all[off - n..off].to_vec()
            })
            .collect();
        VnicSide {
            host,
            link: None,
            peer: None,
            echo: None,
            done: None,
            echo_out: echo_sent.iter().cloned().collect(),
            echo_sent,
            echo_got: vec![],
            go_stream: vec![],
            done_queued: false,
        }
    }

    /// Data-channel bytes into the vnic link.
    fn input(&mut self, mut data: Vec<u8>) {
        let link = self.link.unwrap();
        let mut off = 0;
        while off < data.len() {
            let VnicSide {
                host,
                echo_got,
                go_stream,
                ..
            } = self;
            off += host
                .handle_link_input(link, now(), &mut data[off..], &mut |f| match f.handler {
                    ECHO => echo_got.push(f.body.to_vec()),
                    GO3000 => go_stream.extend_from_slice(f.body),
                    h => panic!("frame for handler {h}"),
                })
                .expect("link input");
            while let Some(e) = self.host.poll_event() {
                match e {
                    strims_vnic::Event::PeerAdded { peer, .. } => self.on_peer(peer),
                    e => panic!("unexpected vnic event {e:?}"),
                }
            }
        }
    }

    fn on_peer(&mut self, peer: PeerKey) {
        let h = &mut self.host;
        let root = h.qos_root();
        h.set_handler(peer, 1001, ECHO).unwrap();
        h.set_handler(peer, 3000, GO3000).unwrap();
        let echo = h.open_writer(peer, 1000, root, 1000).unwrap();
        h.set_backlog(echo).unwrap();
        self.echo = Some(echo);
        self.done = Some(h.open_writer(peer, 9999, root, 9999).unwrap());
        self.peer = Some(peer);
    }

    /// vnic egress into the data channel.
    fn output(&mut self, ep: &mut WebRtcEndpoint, conn: ConnId) {
        let Some(link) = self.link else { return };
        let (echo, done) = (self.echo, self.done);
        let queue = &mut self.echo_out;
        let done_queued = self.done_queued;
        self.host.poll_egress(now(), &mut |f| {
            if Some(f.writer) == echo {
                let Some(m) = queue.pop_front() else {
                    return Fill::NONE;
                };
                f.body[..m.len()].copy_from_slice(&m);
                Fill {
                    len: m.len(),
                    more: !queue.is_empty(),
                }
            } else if Some(f.writer) == done && done_queued {
                f.body[..4].copy_from_slice(b"done");
                Fill::last(4)
            } else {
                Fill::NONE
            }
        });
        loop {
            let data = self.host.poll_link_transmit(link).unwrap();
            if data.is_empty() {
                break;
            }
            let n = ep.link_write(conn, data).unwrap();
            if n == 0 {
                break;
            }
            self.host.advance_link_transmit(link, n).unwrap();
        }
    }
}

fn vnic(rust_offers: bool) {
    if !go_available() {
        return;
    }
    let size = VNIC_GO_SIZE.to_string();
    let role = if rust_offers { "answer" } else { "offer" };
    let mut go = GoPeer::spawn(&["vnic", "-role", role, "-size", &size]);
    let mut live = Live::new();
    let conn = signal(&mut live, &mut go, rust_offers);
    let mut side = VnicSide::new();

    let end = deadline(120);
    let want = stream(GO_STREAM_SEED, VNIC_GO_SIZE);
    while !go.has("done") {
        assert!(StdInstant::now() < end, "timed out: {:#?}", go.drain_log());
        for e in live.pump() {
            match e {
                Event::Open { mtu, .. } => {
                    // The driver adds the vnic link when the channel opens.
                    let link = side
                        .host
                        .add_link(now(), LinkConfig::webrtc().with_mtu(mtu))
                        .unwrap();
                    side.link = Some(link);
                }
                Event::Data { data, .. } => side.input(data),
                Event::Writable(_) | Event::LocalDescription(_) => {}
                Event::Closed { reason, .. } => {
                    panic!("closed: {reason:?} {:#?}", go.drain_log())
                }
            }
        }
        if !side.done_queued
            && side.echo_got.len() == side.echo_sent.len()
            && side.go_stream.len() >= VNIC_GO_SIZE
        {
            side.done_queued = true;
            side.host.set_backlog(side.done.unwrap()).unwrap();
        }
        side.output(&mut live.ep, conn);
    }
    live.ep.close(now(), conn);
    for _ in 0..20 {
        live.pump();
    }
    let log = go.wait();

    // PeerInit both ways: each side knows the other's host.
    let peer = side.host.peer(side.peer.unwrap()).unwrap();
    assert_eq!(peer.certificate().key, key_from_seed(&[0x42; 32]).public);
    let go_id = log
        .iter()
        .find_map(|l| l.strip_prefix("self "))
        .expect("go host id");
    assert_eq!(go_id, hex::encode(peer.host_id().to_bytes()));
    assert!(
        log.contains(&format!("peer {}", hex::encode(side.host.id().to_bytes()))),
        "{log:#?}"
    );
    // The helper reports what its SetRemoteDescription returned, `false` in
    // both roles, and so waits for the peer handler to see the link up.
    let set_remote = log
        .iter()
        .find_map(|l| l.strip_prefix("set-remote "))
        .unwrap();
    assert_eq!(set_remote, "false");
    // Byte-exact both ways.
    assert_eq!(side.echo_got, side.echo_sent);
    assert!(side.go_stream == want, "go stream differs");
}

#[test]
fn pion_vnic_rust_offers() {
    vnic(true);
}

#[test]
fn pion_vnic_rust_answers() {
    vnic(false);
}
