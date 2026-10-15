// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Cross-implementation runs: the Go PSZ sender against the Rust receiver
//! and the reverse, with the broker's composition. The Go side is
//! `crates/strims-mpc/golden/main.go`, speaking the protocol over
//! stdin/stdout. Skipped when `go` isn't on PATH.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    // These tests drive a real subprocess, so they need threads.
    clippy::disallowed_methods
)]

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use strims_mpc::{Endpoint, PszReceiver, PszSender};

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

struct GoRun {
    success: bool,
    stderr: String,
}

/// Runs `go run -tags golden ./crates/strims-mpc/golden <role> <inputs>` and
/// pumps bytes between it and `local` until `local` finishes and the child
/// exits.
fn run_against_go(role: &str, seed: &str, inputs: &[Vec<u8>], local: &mut dyn Endpoint) -> GoRun {
    let mut child = Command::new("go")
        .args(["run", "-tags", "golden", "./crates/strims-mpc/golden"])
        .arg(format!("-seed={seed}"))
        .arg(role)
        .args(inputs.iter().map(hex::encode))
        .current_dir(repo_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn go");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let reader = std::thread::spawn(move || {
        let mut buf = vec![0u8; 4096];
        loop {
            match stdout.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        return;
                    }
                }
            }
        }
    });
    let errs = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });

    // Odd buffer size so poll_transmit splits messages.
    let mut buf = vec![0u8; 1000];
    loop {
        while let Some(n) = local.poll_transmit(&mut buf) {
            if stdin.write_all(&buf[..n]).is_err() {
                break;
            }
        }
        let _ = stdin.flush();
        if local.is_done() {
            break;
        }
        // The first message waits for `go run` to compile the helper.
        match rx.recv_timeout(Duration::from_secs(300)) {
            Ok(chunk) => {
                if local.handle_input(&chunk).is_err() {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = child.kill();
                panic!("timed out waiting for the Go peer");
            }
        }
    }
    drop(stdin);
    let status = child.wait().expect("wait for go");
    let _ = reader.join();
    GoRun {
        success: status.success(),
        stderr: errs.join().unwrap(),
    }
}

fn results(stderr: &str) -> BTreeSet<Vec<u8>> {
    stderr
        .lines()
        .filter_map(|l| l.strip_prefix("result "))
        .map(|h| hex::decode(h).unwrap())
        .collect()
}

/// Network-key-like sets: 32 byte keys, `shared` of them common to both.
fn key_sets(a: usize, b: usize, shared: usize, tag: u8) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let key = |side: u8, i: usize| {
        let mut k = vec![tag; 32];
        k[0] = side;
        k[1..9].copy_from_slice(&(i as u64).to_be_bytes());
        k
    };
    let common: Vec<Vec<u8>> = (0..shared).map(|i| key(0xcc, i)).collect();
    let mut aset: Vec<Vec<u8>> = (0..a - shared).map(|i| key(0xaa, i)).collect();
    let mut bset: Vec<Vec<u8>> = (0..b - shared).map(|i| key(0xbb, i)).collect();
    aset.splice(aset.len() / 2..aset.len() / 2, common.iter().cloned());
    bset.extend(common);
    (aset, bset)
}

fn expected(aset: &[Vec<u8>], bset: &[Vec<u8>]) -> BTreeSet<Vec<u8>> {
    let a: BTreeSet<_> = aset.iter().cloned().collect();
    bset.iter().filter(|b| a.contains(*b)).cloned().collect()
}

// These seeds avoid the cuckoo table failure (`CuckooHashMapFull`) that
// hits a few percent of runs.
const CASES: &[(usize, usize, usize, &str, [u8; 16])] = &[
    (5, 4, 2, "000102030405060708090a0b0c0d0e0f", [7; 16]),
    (40, 25, 10, "101112131415161718191a1b1c1d1e1f", [8; 16]),
    (300, 200, 77, "202122232425262728292a2b2c2d2e2f", [9; 16]),
    (1, 1, 1, "303132333435363738393a3b3c3d3e3f", [10; 16]),
];

#[test]
fn go_sender_rust_receiver() {
    if !go_available() {
        return;
    }
    for &(a, b, shared, go_seed, rust_seed) in CASES {
        let (aset, bset) = key_sets(a, b, shared, 1);
        let mut r = PszReceiver::new(bset.clone(), rust_seed);
        let go = run_against_go("psz-send", go_seed, &aset, &mut r);
        assert!(go.success, "go sender failed: {}", go.stderr);
        let got: BTreeSet<_> = r
            .take_result()
            .expect("rust receiver finished")
            .expect("rust receiver succeeded")
            .into_iter()
            .collect();
        assert_eq!(got, expected(&aset, &bset), "({a}, {b}, {shared})");
        assert_eq!(got.len(), shared);
    }
}

#[test]
fn rust_sender_go_receiver() {
    if !go_available() {
        return;
    }
    for &(a, b, shared, go_seed, rust_seed) in CASES {
        let (aset, bset) = key_sets(a, b, shared, 2);
        let mut s = PszSender::new(aset.clone(), rust_seed);
        let go = run_against_go("psz-recv", go_seed, &bset, &mut s);
        assert!(go.success, "go receiver failed: {}", go.stderr);
        assert_eq!(s.result(), Some(Ok(())));
        assert!(go.stderr.lines().any(|l| l == "done"), "{}", go.stderr);
        let got = results(&go.stderr);
        assert_eq!(got, expected(&aset, &bset), "({a}, {b}, {shared})");
        assert_eq!(got.len(), shared);
    }
}

/// Mixed element lengths, including ones short enough to skip SHA-256.
#[test]
fn mixed_lengths_both_directions() {
    if !go_available() {
        return;
    }
    let elem = |i: u32| {
        let mut v = i.to_be_bytes().to_vec();
        v.resize(1 + (i as usize * 7) % 40, i as u8);
        v
    };
    let aset: Vec<Vec<u8>> = (0..30).map(elem).collect();
    let bset: Vec<Vec<u8>> = (15..40).map(elem).collect();
    let want = expected(&aset, &bset);

    let mut r = PszReceiver::new(bset.clone(), [11; 16]);
    let go = run_against_go(
        "psz-send",
        "404142434445464748494a4b4c4d4e4f",
        &aset,
        &mut r,
    );
    assert!(go.success, "{}", go.stderr);
    let got: BTreeSet<_> = r.take_result().unwrap().unwrap().into_iter().collect();
    assert_eq!(got, want);

    let mut s = PszSender::new(aset.clone(), [12; 16]);
    let go = run_against_go(
        "psz-recv",
        "505152535455565758595a5b5c5d5e5f",
        &bset,
        &mut s,
    );
    assert!(go.success, "{}", go.stderr);
    assert_eq!(results(&go.stderr), want);
}
