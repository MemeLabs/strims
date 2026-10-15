// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Live Go<->Rust swarms: a real Go `ppspp.Runner` (the helper in
//! `crates/strims-ppspp/golden/interop`) and a Rust [`Runner`] exchange
//! frames over the helper's stdin/stdout, in real time, with Go's default
//! swarm options (Merkle/BLAKE2b/ed25519, low latency delivery). In one
//! test Go seeds with a `ppspp.Writer` and Rust downloads; in the other Rust
//! seeds with a [`Writer`] and Go downloads. The downloader checks every
//! byte from where its reader starts to the end of the stream.
//!
//! Two more cover the push/pull hybrid: a Rust [`PushMode::Hybrid`] leech
//! subscribes to the streams of a 4-stream Go seed, which answers with
//! STREAM_OPEN and pushes, and a Rust hybrid seed serves a Go leech, which
//! only pulls, so that run covers eager HAVEs.
//!
//! Every test runs once per request controller (LEDBAT and the BBR-style
//! `BbrLike`, `SwarmOptions::request_controller`).
//!
//! Skipped when `go` isn't on PATH.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    // These tests drive a real subprocess in real time, so they need
    // threads and the wall clock.
    clippy::disallowed_methods
)]

use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration as StdDuration, SystemTime};

use sha2::{Digest, Sha256};
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_ppspp::store;
use strims_ppspp::{
    Event, PushMode, RequestControllerKind, Runner, Swarm, SwarmId, SwarmOptions, Writer,
    WriterOptions,
};

/// Both request controllers.
const CONTROLLERS: [RequestControllerKind; 2] = [
    RequestControllerKind::Ledbat,
    RequestControllerKind::BbrLike,
];

const MTU: usize = 16 * 1024 - 4;
const UNIT: usize = 64 * 1024;
/// Stream bytes: 8 MiB.
const TOTAL: u64 = 8 << 20;
/// Seed bytes per second.
const RATE: u64 = 2 << 20;

fn stream_byte(o: u64) -> u8 {
    let x = o.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ (o >> 7);
    (x >> 56) as u8
}

fn now() -> Time {
    Time::from_system(SystemTime::now())
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Builds the helper once per test binary; `None` (skip) without `go`.
fn helper() -> Option<PathBuf> {
    static HELPER: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    HELPER.get_or_init(build_helper).clone()
}

fn build_helper() -> Option<PathBuf> {
    match Command::new("go").arg("version").output() {
        Ok(o) if o.status.success() => {}
        _ => {
            eprintln!("skipping: `go` is not on PATH");
            return None;
        }
    }
    let bin = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ppspp-interop-go");
    let out = Command::new("go")
        .args(["build", "-tags", "golden", "-o"])
        .arg(&bin)
        .arg("./crates/strims-ppspp/golden/interop")
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

struct Go {
    child: Child,
    stdin: Option<ChildStdin>,
    frames: mpsc::Receiver<Vec<u8>>,
    lines: mpsc::Receiver<String>,
    log: Vec<String>,
}

impl Go {
    fn spawn(bin: &PathBuf, mode: &str) -> Go {
        Go::spawn_with(bin, mode, &[])
    }

    fn spawn_with(bin: &PathBuf, mode: &str, args: &[&str]) -> Go {
        let mut child = Command::new(bin)
            .arg(mode)
            .arg(format!("-bytes={TOTAL}"))
            .arg(format!("-rate={RATE}"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn go helper");
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let (ftx, frames) = mpsc::channel();
        std::thread::spawn(move || {
            let mut r = BufReader::with_capacity(4 * MTU, stdout);
            let mut hdr = [0u8; 4];
            while r.read_exact(&mut hdr).is_ok() {
                let mut b = vec![0u8; u32::from_be_bytes(hdr) as usize];
                if r.read_exact(&mut b).is_err() || ftx.send(b).is_err() {
                    return;
                }
            }
        });
        let (ltx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let Ok(line) = line else { return };
                if ltx.send(line).is_err() {
                    return;
                }
            }
        });
        Go {
            stdin: child.stdin.take(),
            child,
            frames,
            lines,
            log: vec![],
        }
    }

    fn send(&mut self, frame: &[u8]) {
        if let Some(w) = &mut self.stdin {
            let mut b = Vec::with_capacity(frame.len() + 4);
            b.extend_from_slice(&(frame.len() as u32).to_be_bytes());
            b.extend_from_slice(frame);
            if w.write_all(&b).and_then(|_| w.flush()).is_err() {
                self.stdin = None;
            }
        }
    }

    fn poll_lines(&mut self) {
        while let Ok(l) = self.lines.try_recv() {
            eprintln!("go: {l}");
            self.log.push(l);
        }
    }

    fn line(&self, prefix: &str) -> Option<&str> {
        self.log.iter().find_map(|l| l.strip_prefix(prefix))
    }
}

impl Drop for Go {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The driver loop: frames in, timers, frames out, and `step` for the
/// test's own work, until `step` returns true or the helper exits.
fn drive(
    go: &mut Go,
    runner: &mut Runner,
    peer: strims_ppspp::PeerKey,
    mut step: impl FnMut(&mut Runner, &mut Go, Time, Option<Event>) -> bool,
) {
    let deadline = now() + Duration::from_secs(120);
    let mut buf = vec![0u8; MTU];
    loop {
        assert!(now() < deadline, "timed out; go said {:?}", go.log);
        go.poll_lines();
        let t = now();
        if runner.poll_timeout().is_some_and(|d| d <= t) {
            runner.handle_timeout(t);
        }
        while let Some(e) = runner.poll_event() {
            if step(runner, go, t, Some(e)) {
                return;
            }
        }
        if step(runner, go, t, None) {
            return;
        }
        while runner.wants_transmit(peer) {
            let n = runner.poll_transmit(peer, now(), &mut buf);
            if n == 0 {
                break;
            }
            go.send(&buf[..n]);
        }

        let wait = runner
            .poll_timeout()
            .map(|d| d - now())
            .unwrap_or(Duration::from_millis(10))
            .clamp(Duration::ZERO, Duration::from_millis(5));
        match go.frames.recv_timeout(wait.to_std().unwrap()) {
            Ok(f) => {
                runner.handle_frame(peer, now(), &f).unwrap();
                while let Ok(f) = go.frames.try_recv() {
                    runner.handle_frame(peer, now(), &f).unwrap();
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                go.poll_lines();
                // Let the stderr thread finish.
                std::thread::sleep(StdDuration::from_millis(100));
                go.poll_lines();
                if step(runner, go, now(), None) {
                    return;
                }
                panic!("go exited early: {:?}", go.log);
            }
        }
    }
}

fn key() -> strims_proto::types::Key {
    strims_crypto::key_from_seed(&[9; 32])
}

fn expected_sha(start: u64, len: u64) -> String {
    let mut h = Sha256::new();
    let mut b = Vec::with_capacity(64 * 1024);
    let mut o = start;
    while o < start + len {
        b.clear();
        let end = (o + 64 * 1024).min(start + len);
        b.extend((o..end).map(stream_byte));
        h.update(&b);
        o = end;
    }
    hex::encode(h.finalize())
}

#[test]
fn go_seeds_rust_downloads() {
    let Some(bin) = helper() else { return };
    for kind in CONTROLLERS {
        let go = Go::spawn(&bin, "seed");
        let options = SwarmOptions {
            request_controller: kind,
            ..Default::default()
        };
        let (runner, peer, sk) = download_from_go(go, options, |_, _, _| {});
        let stats = runner.channel_stats(peer, sk).unwrap();
        eprintln!("{kind:?} channel: {stats:?}");
        assert_eq!(stats.scheduler.controller, kind);
        assert_eq!(stats.inbound.invalid_data, 0);
        assert_eq!(stats.inbound.rejected, 0);
    }
}

#[test]
fn rust_hybrid_subscribes_to_go_seed() {
    // The Go seed answers each STREAM_REQUEST with STREAM_OPEN and pushes
    // the stream.
    let Some(bin) = helper() else { return };
    for kind in CONTROLLERS {
        rust_hybrid_subscribes_to_go_seed_with(&bin, kind);
    }
}

fn rust_hybrid_subscribes_to_go_seed_with(bin: &PathBuf, kind: RequestControllerKind) {
    let go = Go::spawn_with(bin, "seed", &["-streams=4"]);
    let options = SwarmOptions {
        stream_count: 4,
        push_mode: PushMode::Hybrid,
        request_controller: kind,
        ..Default::default()
    };
    // Halfway, cancel stream 0: the Go seed must stop pushing it; stream 0
    // is pulled until the hybrid's backoff lets it subscribe again (the
    // stream ends before that here).
    let mut cancelled = None;
    let (runner, peer, sk) = download_from_go(go, options, |runner, pos, peer| {
        if cancelled.is_none() && pos >= TOTAL / 2 {
            let sk = runner.peer_swarms(peer).next().unwrap();
            runner.cancel_stream(peer, sk, 0).unwrap();
            cancelled = Some(runner.push_stats(sk).unwrap());
        }
    });
    let stats = runner.channel_stats(peer, sk).unwrap();
    let push = runner.push_stats(sk).unwrap();
    let before = cancelled.unwrap();
    eprintln!("{kind:?} channel: {stats:?}");
    eprintln!("push at cancel: {before:?}");
    eprintln!("push: {push:?}");
    assert_eq!(stats.scheduler.controller, kind);
    assert_eq!(stats.inbound.invalid_data, 0);
    assert_eq!(stats.inbound.rejected, 0);
    // Go accepted every subscription.
    assert!(before.opens >= 4, "{before:?}");
    assert_eq!(stats.outbound.stream_request, push.requests_sent);
    assert_eq!(stats.inbound.stream_open, push.opens, "{stats:?}");
    // The cancel, plus at most a few repeats for pushes Go had queued
    // (unsolicited pushes are cancelled again at most once a second).
    assert!((1..=4).contains(&stats.outbound.stream_cancel), "{stats:?}");
    assert_eq!(push.parents[1..], [Some(peer); 3]);
    // Pushes did the work: chunks pushed in time, few pulled (stream 0,
    // a quarter of the chunks, is pulled after the cancel).
    assert!(push.hits * 10 > push.expected * 9, "{push:?}");
    assert!(stats.outbound.request * 4 < stats.inbound.data, "{stats:?}");
    // Go stopped pushing stream 0: only what it had queued before the
    // cancel arrived came unasked.
    assert!(push.unsolicited < 64, "{push:?}");
}

/// Runs a Rust leech against the Go seed until it has read to the end of
/// the stream, checking every byte.
fn download_from_go(
    mut go: Go,
    options: SwarmOptions,
    mut at: impl FnMut(&mut Runner, u64, strims_ppspp::PeerKey),
) -> (Runner, strims_ppspp::PeerKey, strims_ppspp::SwarmKey) {
    let key = key();
    let swarm = Swarm::new(SwarmId::new(&key.public), options).unwrap();
    let id = swarm.id().to_string();
    let mut reader = swarm.reader();
    let mut runner = Runner::new(det_rng(1));
    let sk = runner.add_swarm(swarm);
    let peer = runner.add_peer(b"go", now());
    runner.run_swarm(peer, sk, 1, 1, now()).unwrap();

    let mut start = None;
    let mut pos = 0u64;
    let mut mismatches = 0u64;
    let mut buf = vec![0u8; 64 * 1024];
    let mut h = Sha256::new();
    drive(&mut go, &mut runner, peer, |runner, _go, _t, e| {
        at(runner, pos, peer);
        if !matches!(e, Some(Event::Readable(_))) {
            return pos >= TOTAL;
        }
        let swarm = runner.swarm(sk).unwrap();
        if start.is_none() {
            let Ok(o) = reader.offset(swarm.store()) else {
                return false;
            };
            start = Some(o);
            pos = o;
        }
        loop {
            match swarm.read(&mut reader, &mut buf) {
                Ok(n) => {
                    let n = n.min((TOTAL - pos) as usize);
                    for (i, &b) in buf[..n].iter().enumerate() {
                        if b != stream_byte(pos + i as u64) {
                            mismatches += 1;
                        }
                    }
                    h.update(&buf[..n]);
                    pos += n as u64;
                    if pos >= TOTAL {
                        return true;
                    }
                }
                Err(store::Error::WouldBlock) => return false,
                Err(e) => panic!("read at {pos}: {e}"),
            }
        }
    });

    assert_eq!(go.line("swarm "), Some(id.as_str()));
    let start = start.unwrap();
    let len = pos - start;
    eprintln!("rust downloaded {len} bytes from offset {start}");
    assert_eq!(mismatches, 0);
    assert_eq!(pos, TOTAL);
    assert!(start <= 1 << 20, "started late at {start}");
    assert!(len >= 6 << 20, "only {len} bytes");
    assert_eq!(hex::encode(h.finalize()), expected_sha(start, len));
    (runner, peer, sk)
}

#[test]
fn rust_seeds_go_downloads() {
    let Some(bin) = helper() else { return };
    for kind in CONTROLLERS {
        let go = Go::spawn(&bin, "leech");
        seed_to_go(
            go,
            SwarmOptions {
                request_controller: kind,
                ..Default::default()
            },
        );
    }
}

#[test]
fn rust_hybrid_seeds_go_downloads() {
    // Eager HAVEs and the hybrid's admission control facing a Go leech,
    // which never subscribes: plain pull, but HAVEs arrive at once.
    let Some(bin) = helper() else { return };
    for kind in CONTROLLERS {
        let go = Go::spawn_with(&bin, "leech", &["-streams=4"]);
        seed_to_go(
            go,
            SwarmOptions {
                stream_count: 4,
                push_mode: PushMode::Hybrid,
                request_controller: kind,
                ..Default::default()
            },
        );
    }
}

fn seed_to_go(mut go: Go, options: SwarmOptions) {
    let (mut writer, swarm) = Writer::new(
        WriterOptions {
            swarm_options: options,
            key: key(),
        },
        now(),
    )
    .unwrap();
    let id = swarm.id().to_string();
    let mut runner = Runner::new(det_rng(2));
    let sk = runner.add_swarm(swarm);
    let peer = runner.add_peer(b"go", now());
    runner.run_swarm(peer, sk, 1, 1, now()).unwrap();

    let mut started: Option<Time> = None;
    let mut written = 0u64;
    let unit_time = Duration::from_nanos(UNIT as i64 * 1_000_000_000 / RATE as i64);
    let mut data = vec![0u8; UNIT];
    drive(&mut go, &mut runner, peer, |runner, go, t, e| {
        if e.is_some() {
            return false;
        }
        if go.line("result ").is_some() {
            return true;
        }
        // Start writing once Go has spoken.
        if started.is_none() {
            if runner.channel_stats(peer, sk).unwrap().inbound.handshake == 0 {
                return false;
            }
            started = Some(t);
        }
        let due = started.unwrap() + unit_time * (written / UNIT as u64) as i64;
        if written < TOTAL && t >= due {
            for (i, b) in data.iter_mut().enumerate() {
                *b = stream_byte(written + i as u64);
            }
            runner.write(sk, &mut writer, t, &data).unwrap();
            written += UNIT as u64;
        }
        false
    });

    assert_eq!(go.line("swarm "), Some(id.as_str()));
    let result = go.line("result ").expect("go result").to_string();
    let f: Vec<&str> = result.split(' ').collect();
    let (start, len): (u64, u64) = (f[0].parse().unwrap(), f[1].parse().unwrap());
    eprintln!("go downloaded {len} bytes from offset {start}");
    assert_eq!(f[3], "0", "go saw mismatched bytes");
    assert_eq!(start + len, TOTAL);
    assert!(start <= 1 << 20, "started late at {start}");
    assert!(len >= 6 << 20, "only {len} bytes");
    assert_eq!(f[2], expected_sha(start, len));
    let stats = runner.channel_stats(peer, sk).unwrap();
    eprintln!("channel: {stats:?}");
}
