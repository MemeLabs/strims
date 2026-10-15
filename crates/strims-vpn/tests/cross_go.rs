// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Live Go↔Rust run: two Go hosts (`golden/main.go live`), A and B, in one
//! vpn network, with A linked to a Rust vnic and vpn host over the helper's
//! stdin/stdout and to B over an in-memory pipe. B is two hops away.
//!
//! The Rust side completes the vnic handshake, joins the network, and then:
//! - exchanges encrypted unicasts with A (one hop) and B (two hops);
//! - broadcasts, and makes A and B broadcast;
//! - gets a hash table record B published, and has B get one it published;
//! - finds B through the peer index, and has B find it;
//! - runs peer exchange both ways: Rust's offer to B is refused (B's vnic
//!   allows no new peers), and B's offer is answered with a
//!   Rust TCP listener that B dials, which makes B a direct link.
//!
//! Commands go to the Go hosts' port 30000 as vpn messages. Skipped when
//! `go` isn't on PATH.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    // A real subprocess and sockets: threads and the wall clock.
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration as StdDuration, Instant, SystemTime};

use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_crypto::key_from_seed;
use strims_kademlia::ID;
use strims_vnic::{HostConfig, LinkConfig, LinkKind, PeerKey, TcpMuxAcceptor, TcpUri};
use strims_vpn::{
    Config, Event, ExchangeOutcome, HashTableGetOptions, Host, LinkDescription, NetworkId,
    VerifyMode, VnicHost,
};

const GO_SEED: u8 = 0x42;
const RUST_SEED: u8 = 0x21;
const MTU: usize = 16 * 1024;
const VPN_PORT: u16 = 10000;
const APP_PORT: u16 = 20000;
const CONTROL_PORT: u16 = 30000;
const REPLY_PORT: u16 = 30001;

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

/// A `go -overlay` flag for building the helper when `pkg/vpn` holds the
/// uncommitted debug topology recorder (`graph*.go`, wired into `host.go`
/// and `network.go`), whose `DebugPrint` writes
/// `/workspaces/go-ppspp/meme.dot` on every received frame and panics when
/// that path doesn't exist. The overlay uses the committed `host.go` and
/// `network.go` and drops `graph*.go`, leaving the source tree alone.
/// `None` when the recorder isn't there.
fn overlay_without_graph_wip() -> Option<String> {
    let vpn = repo_root().join("pkg/vpn");
    if !vpn.join("graph_native.go").exists() {
        return None;
    }
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("vpn-go-overlay");
    std::fs::create_dir_all(&dir).unwrap();
    let mut replace = Vec::new();
    for f in ["host.go", "network.go"] {
        let out = Command::new("git")
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

fn now() -> Time {
    Time::from_system(SystemTime::now())
}

enum Input {
    Stdio(Vec<u8>),
    Line(String),
    Accepted(TcpStream),
    Tcp(Vec<u8>),
}

fn spawn_reader(tx: Sender<Input>, mut r: impl Read + Send + 'static, wrap: fn(Vec<u8>) -> Input) {
    std::thread::spawn(move || {
        let mut buf = vec![0u8; 1777];
        loop {
            match r.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    if tx.send(wrap(buf[..n].to_vec())).is_err() {
                        return;
                    }
                }
            }
        }
    });
}

struct Live {
    child: Child,
    vnic: VnicHost<NetworkId>,
    vpn: Host<u16>,
    net: NetworkId,
    stdio: PeerKey,
    stdin: ChildStdin,
    tcp: Option<(TcpStream, Option<PeerKey>, TcpMuxAcceptor)>,
    rx: Receiver<Input>,
    tx: Sender<Input>,
    lines: Vec<String>,
    events: Vec<Event<u16>>,
    errors: Vec<String>,
    go_a: Option<ID>,
    go_b: Option<ID>,
}

impl Live {
    fn start() -> Live {
        let mut child = Command::new("go")
            .args(["run", "-tags", "golden"])
            .args(overlay_without_graph_wip())
            .args(["./crates/strims-vpn/golden", "live"])
            .arg(format!("-seed={GO_SEED}"))
            .arg(format!("-mtu={MTU}"))
            .current_dir(repo_root())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn go");
        let (tx, rx) = mpsc::channel();
        let stderr = child.stderr.take().unwrap();
        let ltx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let Ok(line) = line else { return };
                if ltx.send(Input::Line(line)).is_err() {
                    return;
                }
            }
        });
        spawn_reader(tx.clone(), child.stdout.take().unwrap(), Input::Stdio);
        let stdin = child.stdin.take().unwrap();

        let hc = HostConfig {
            label: "rust-peer".into(),
            ..HostConfig::default()
        };
        let mut vnic = VnicHost::new(key_from_seed(&[RUST_SEED; 32]), hc, det_rng(5)).unwrap();
        vnic.set_max_peers(16);
        let config = Config {
            verify: VerifyMode::Audit,
            ..Config::default()
        };
        let mut vpn = Host::new(&mut vnic, config, det_rng(6)).unwrap();
        let net = vpn
            .add_network(&mut vnic, &key_from_seed(&[0xee; 32]).public)
            .unwrap();
        vpn.set_handler(net, APP_PORT, APP_PORT).unwrap();
        vpn.set_handler(net, REPLY_PORT, REPLY_PORT).unwrap();
        let stdio = vnic
            .add_link(now(), LinkConfig::new(LinkKind::Other, MTU))
            .unwrap();
        let mut live = Live {
            child,
            vnic,
            vpn,
            net,
            stdio,
            stdin,
            tcp: None,
            rx,
            tx,
            lines: vec![],
            events: vec![],
            errors: vec![],
            go_a: None,
            go_b: None,
        };
        live.service();
        live
    }

    /// Feeds link input, handing vpn frames to the vpn host.
    fn input(&mut self, link: PeerKey, mut data: Vec<u8>) {
        let t = now();
        let mut off = 0;
        while off < data.len() {
            let Live {
                vnic, vpn, errors, ..
            } = self;
            let res = vnic.handle_link_input(link, t, &mut data[off..], &mut |f| {
                if let Err(e) = vpn.handle_frame(t, f.handler, f.peer, f.body) {
                    errors.push(e.to_string());
                }
            });
            match res {
                Ok(n) => off += n,
                Err(e) => panic!("link input: {e}: {:#?}", self.lines),
            }
            self.drain_vnic();
        }
    }

    fn drain_vnic(&mut self) {
        while let Some(e) = self.vnic.poll_event() {
            match e {
                strims_vnic::Event::PeerAdded { peer, host_id } => {
                    eprintln!("rust: peer {host_id}");
                    self.vpn
                        .add_peer(&mut self.vnic, self.net, peer, VPN_PORT, VPN_PORT, self.net)
                        .unwrap();
                }
                strims_vnic::Event::PeerRemoved { peer, .. } => self.vpn.handle_peer_removed(peer),
                strims_vnic::Event::LinkClosed { link, reason } => {
                    panic!("link {link:?} closed: {reason:?}")
                }
            }
        }
    }

    /// The driver pass: flush, egress, transmit, events, timers.
    fn service(&mut self) {
        let t = now();
        if self.vpn.poll_timeout().is_some_and(|d| !t.before(d)) {
            self.vpn.handle_timeout(t);
        }
        self.drain_vnic();
        self.vpn.flush(&mut self.vnic, t);
        let Live { vnic, vpn, .. } = self;
        vnic.poll_egress(t, &mut |f| vpn.fill_egress(f.tag, f.peer, f.body));
        let links: Vec<PeerKey> = self.vnic.links_with_transmit().collect();
        for link in links {
            loop {
                let data = self.vnic.poll_link_transmit(link).unwrap();
                if data.is_empty() {
                    break;
                }
                let n = data.len();
                if link == self.stdio {
                    self.stdin.write_all(data).unwrap();
                    self.stdin.flush().unwrap();
                } else if let Some((s, _, _)) = &mut self.tcp {
                    s.write_all(data).unwrap();
                }
                self.vnic.advance_link_transmit(link, n).unwrap();
            }
        }
        while let Some(e) = self.vpn.poll_event() {
            eprintln!("rust: {e:?}");
            self.events.push(e);
        }
    }

    fn handle(&mut self, input: Input) {
        match input {
            Input::Stdio(data) => {
                let link = self.stdio;
                self.input(link, data);
            }
            Input::Line(l) => {
                eprintln!("go: {l}");
                if let Some(h) = l.strip_prefix("node a ") {
                    self.go_a = Some(ID::unmarshal(&hex::decode(h).unwrap()).unwrap().0);
                }
                if let Some(h) = l.strip_prefix("node b ") {
                    self.go_b = Some(ID::unmarshal(&hex::decode(h).unwrap()).unwrap().0);
                }
                self.lines.push(l);
            }
            Input::Accepted(s) => {
                s.set_nodelay(true).unwrap();
                spawn_reader(self.tx.clone(), s.try_clone().unwrap(), Input::Tcp);
                self.tcp = Some((s, None, TcpMuxAcceptor::new()));
            }
            Input::Tcp(data) => {
                let Some((_, link, acc)) = &mut self.tcp else {
                    panic!("tcp bytes before accept");
                };
                match *link {
                    Some(l) => self.input(l, data),
                    None => {
                        let (n, key) = acc.handle_input(&data).unwrap();
                        if let Some(key) = key {
                            assert_eq!(&key[..], &self.vnic.profile_key().public[..]);
                            let l = self.vnic.add_link(now(), LinkConfig::tcp_accept()).unwrap();
                            if let Some((_, link, _)) = &mut self.tcp {
                                *link = Some(l);
                            }
                            self.input(l, data[n..].to_vec());
                        }
                    }
                }
            }
        }
        self.service();
    }

    /// Runs until `done` holds.
    fn until(&mut self, what: &str, mut done: impl FnMut(&Live) -> bool) {
        // The first wait includes `go run` compiling the helper.
        let deadline = Instant::now() + StdDuration::from_secs(600);
        while !done(self) {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                panic!("timed out waiting for {what}: {:#?}", self.lines);
            }
            let wait = left.min(StdDuration::from_millis(50));
            match self.rx.recv_timeout(wait) {
                Ok(i) => self.handle(i),
                Err(RecvTimeoutError::Timeout) => self.service(),
                Err(RecvTimeoutError::Disconnected) => panic!("go helper gone: {:#?}", self.lines),
            }
            if let Ok(Some(status)) = self.child.try_wait() {
                // Drain what's left so failures show the helper's output.
                while let Ok(i) = self.rx.try_recv() {
                    if let Input::Line(l) = i {
                        self.lines.push(l);
                    }
                }
                if !done(self) {
                    panic!(
                        "go helper exited ({status}) waiting for {what}: {:#?}",
                        self.lines
                    );
                }
            }
        }
    }

    fn send(&mut self, to: ID, port: u16, src_port: u16, body: &str) {
        self.vpn
            .send(now(), self.net, to, port, src_port, body.as_bytes())
            .unwrap();
        self.service();
    }

    fn command(&mut self, to: ID, cmd: &str) {
        self.send(to, CONTROL_PORT, REPLY_PORT, cmd);
    }

    /// The body of an app message from `src` with `hops` hops.
    fn got(&self, src: ID, port: u16, body: &str) -> Option<usize> {
        self.events.iter().find_map(|e| match e {
            Event::Message(d)
                if d.src == src && d.dst_port == port && d.body == body.as_bytes() =>
            {
                Some(d.hops)
            }
            _ => None,
        })
    }

    fn reply(&self, src: ID, prefix: &str) -> Option<String> {
        self.events.iter().find_map(|e| match e {
            Event::Message(d) if d.src == src && d.dst_port == REPLY_PORT => {
                let s = String::from_utf8_lossy(&d.body).into_owned();
                s.starts_with(prefix).then_some(s)
            }
            _ => None,
        })
    }

    fn line(&self, l: &str) -> bool {
        self.lines.iter().any(|x| x == l)
    }
}

#[test]
fn go_vpn_network() {
    if !go_available() {
        return;
    }
    let mut live = Live::start();
    let me = live.vnic.id();
    let me_hex = hex::encode(me.to_bytes());

    // vnic handshake with A, and the network link.
    live.until("go hosts and the link to A", |l| {
        l.line("ready") && l.go_a.is_some_and(|a| l.vpn.has_peer(l.net, &a))
    });
    let (a, b) = (live.go_a.unwrap(), live.go_b.unwrap());
    assert!(!live.vpn.has_peer(live.net, &b));

    // Unicast both ways: one hop to A, two to B.
    live.send(a, APP_PORT, APP_PORT, "ping 1");
    live.send(b, APP_PORT, APP_PORT, "ping 2");
    live.until("pongs", |l| {
        l.got(a, APP_PORT, "pong 1").is_some() && l.got(b, APP_PORT, "pong 2").is_some()
    });
    assert_eq!(live.got(a, APP_PORT, "pong 1"), Some(1));
    assert_eq!(live.got(b, APP_PORT, "pong 2"), Some(2));
    // Go saw our messages with the right sender, hops and flags (encrypted).
    assert!(live.line(&format!("recv a {me_hex} 1 2 {}", hex::encode("ping 1"))));
    assert!(live.line(&format!("recv b {me_hex} 2 2 {}", hex::encode("ping 2"))));

    // Broadcasts both ways.
    live.vpn
        .broadcast(now(), live.net, APP_PORT, APP_PORT, b"rust-bcast")
        .unwrap();
    live.service();
    live.command(a, "bcast x");
    live.command(b, "bcast y");
    live.until("broadcasts", |l| {
        l.got(a, APP_PORT, "ack rust-bcast").is_some()
            && l.got(b, APP_PORT, "ack rust-bcast").is_some()
            && l.got(a, APP_PORT, "go-bcast a x").is_some()
            && l.got(b, APP_PORT, "go-bcast b y").is_some()
    });
    assert_eq!(live.got(me, APP_PORT, "rust-bcast"), Some(0));
    assert_eq!(live.got(a, APP_PORT, "go-bcast a x"), Some(1));
    assert_eq!(live.got(b, APP_PORT, "go-bcast b y"), Some(2));
    assert!(live.line(&format!(
        "recv b {me_hex} 2 4 {}",
        hex::encode("rust-bcast")
    )));

    // Hash table: B publishes, Rust gets from the network.
    live.command(
        b,
        &format!("ht-set {} {}", hex::encode("salt"), hex::encode("go value")),
    );
    live.until("ht-set", |l| l.reply(b, "ht-set-ok").is_some());
    let go_key = key_from_seed(&[0x71; 32]).public;
    let opts = HashTableGetOptions {
        disable_cache: true,
        timeout: strims_base::time::Duration::from_secs(20),
    };
    let net = live.net;
    let get = live
        .vpn
        .hash_table_get(now(), net, &go_key, b"salt", opts)
        .unwrap();
    live.service();
    live.until("hash table value", |l| {
        l.events.iter().any(|e| matches!(e, Event::HashTableValue { get: g, value, .. } if *g == get && value == b"go value"))
    });

    // Rust publishes, B gets.
    let rust_key = key_from_seed(&[0x72; 32]);
    live.vpn
        .hash_table_set(now(), net, &rust_key, b"rs", b"rust value")
        .unwrap();
    live.service();
    live.command(
        b,
        &format!(
            "ht-get {} {}",
            hex::encode(&rust_key.public),
            hex::encode("rs")
        ),
    );
    live.until("ht-get", |l| {
        l.reply(b, "ht-value").or(l.reply(b, "ht-none")).is_some()
    });
    assert_eq!(
        live.reply(b, "ht-value")
            .or(live.reply(b, "ht-none"))
            .unwrap(),
        format!("ht-value {}", hex::encode("rust value"))
    );

    // Peer index: B publishes to A, Rust searches and A answers.
    live.command(
        b,
        &format!(
            "pi-publish {} {} 4321",
            hex::encode("swarm"),
            hex::encode("s1")
        ),
    );
    live.until("pi-publish", |l| l.reply(b, "pi-publish-ok").is_some());
    let search = live
        .vpn
        .peer_index_search(now(), net, b"swarm", b"s1")
        .unwrap();
    live.service();
    live.until("peer index host", |l| {
        l.events.iter().any(|e| matches!(e, Event::PeerIndexHost { search: s, host, .. } if *s == search && host.host_id == b && host.port == 4321))
    });

    // Rust publishes to A, B searches and A answers.
    live.vpn
        .peer_index_publish(now(), net, b"swarm", b"s2", 5432)
        .unwrap();
    live.service();
    live.command(
        b,
        &format!("pi-search {} {}", hex::encode("swarm"), hex::encode("s2")),
    );
    live.until("pi-search", |l| {
        l.reply(b, "pi-host").or(l.reply(b, "pi-none")).is_some()
    });
    assert_eq!(
        live.reply(b, "pi-host")
            .or(live.reply(b, "pi-none"))
            .unwrap(),
        format!("pi-host {me_hex} 5432")
    );

    // Peer exchange, Rust → B: B's vnic allows no new peers, so it
    // refuses.
    let x = live
        .vpn
        .connect(&live.vnic, now(), net, b)
        .unwrap()
        .unwrap();
    live.service();
    live.until("gather", |l| {
        l.events
            .iter()
            .any(|e| matches!(e, Event::GatherLocalDescriptions { exchange, .. } if *exchange == x))
    });
    live.vpn
        .set_local_descriptions(now(), x, Ok(vec![]))
        .unwrap();
    live.service();
    live.until("refusal", |l| {
        l.events
            .iter()
            .any(|e| matches!(e, Event::ExchangeClosed { exchange, .. } if *exchange == x))
    });
    assert!(live.events.iter().any(|e| matches!(
        e,
        Event::ExchangeClosed { exchange, outcome: ExchangeOutcome::Rejected(m), .. }
            if *exchange == x && m == "unable to add new peers"
    )));

    // Peer exchange, B → Rust: B offers (no descriptions: its TCP interface
    // has no listener), Rust answers with a TCP listener, and B dials it.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = TcpUri {
        host: listener.local_addr().unwrap().to_string(),
        peer_key: live
            .vnic
            .profile_key()
            .public
            .as_slice()
            .try_into()
            .unwrap(),
    }
    .to_uri();
    let atx = live.tx.clone();
    std::thread::spawn(move || {
        if let Ok((s, _)) = listener.accept() {
            let _ = atx.send(Input::Accepted(s));
        }
    });
    live.command(b, &format!("px-connect {me_hex}"));
    live.until("offer from B", |l| {
        l.events.iter().any(|e| matches!(e, Event::GatherLocalDescriptions { host_id, exchange, .. } if *host_id == b && *exchange != x))
    });
    let x2 = live
        .events
        .iter()
        .find_map(|e| match e {
            Event::GatherLocalDescriptions {
                host_id, exchange, ..
            } if *host_id == b && *exchange != x => Some(*exchange),
            _ => None,
        })
        .unwrap();
    let desc = LinkDescription {
        interface: "tcp".into(),
        description: uri,
    };
    live.vpn
        .set_local_descriptions(now(), x2, Ok(vec![desc]))
        .unwrap();
    live.service();
    live.until("B dials us", |l| {
        l.vpn.has_peer(l.net, &b) && l.line(&format!("peer b {me_hex}"))
    });
    live.send(b, APP_PORT, APP_PORT, "ping 3");
    live.until("direct pong", |l| l.got(b, APP_PORT, "pong 3").is_some());
    assert_eq!(live.got(b, APP_PORT, "pong 3"), Some(1));

    // Every Go message verified, and nothing was rejected.
    let m = live.vpn.metrics();
    assert_eq!(m.verify_failures, 0, "{m:?}");
    assert_eq!(m.decrypt_failures, 0, "{m:?}");
    assert!(live.errors.is_empty(), "{:?}", live.errors);

    live.command(a, "done");
    live.command(b, "done");
    live.until("go done", |l| l.line("done"));
    let status = live.child.wait().unwrap();
    assert!(status.success(), "{:#?}", live.lines);
}
