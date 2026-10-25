// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A run with ppspp capture on: the trace a run without it gives, and a
//! dump per host that reads back.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::Path;
use std::time::Duration;

use std::collections::{BTreeMap, BTreeSet};

use strims_base::rng::det_rng;
use strims_ppspp::PushMode;
use strims_ppspp_capture::{Dir, Reader, Record, sample_kind};
use strims_sim::topology::random_mesh;
use strims_sim::{Scenario, SendBuffer, SwarmSpec, Transport, run};
use strims_wan::{Access, City, Site};

#[global_allocator]
static ALLOC: strims_sim::memory::Allocator<std::alloc::System> =
    strims_sim::memory::Allocator::system();

/// A seed and four viewers in a mesh, streaming 32 KiB/s in Hybrid.
fn mesh(seed: u64) -> Scenario {
    let mut s = Scenario::new("capture");
    let mut rng = det_rng(seed);
    let src = s.host("seed", Some(1 << 20));
    let viewers = s.hosts("v", 4, Some(1 << 20));
    let all: Vec<_> = std::iter::once(src)
        .chain(viewers.iter().copied())
        .collect();
    random_mesh(&mut s, &mut rng, &all, 2, 0.0);
    s.swarm(
        SwarmSpec::live(
            src,
            viewers,
            32 * 1024,
            Duration::from_secs(2),
            Duration::from_secs(8),
        )
        .with_push_mode(PushMode::Hybrid),
    );
    s.duration = Duration::from_secs(10);
    s
}

#[test]
fn a_captured_run_keeps_its_trace() {
    let plain = run(mesh(1), 1).unwrap();
    // Per process: test binaries running at once share the directory.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "a_captured_run_keeps_its_trace-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let mut spec = mesh(1);
    spec.capture = Some(dir.clone());
    let captured = run(spec, 1).unwrap();
    assert_eq!(plain.trace_hash, captured.trace_hash);

    for name in ["seed", "v0", "v1", "v2", "v3"] {
        let f = std::fs::read(dir.join(format!("{name}-1.ppcap"))).unwrap();
        let mut r = Reader::new(&f[..]).unwrap();
        assert_eq!(r.header().label, name);
        let (mut frames_in, mut frames_out, mut taps) = (0, 0, 0);
        while let Some(e) = r.next_record() {
            match e.unwrap().record {
                Record::Frame { dir: Dir::In, .. } => frames_in += 1,
                Record::Frame { dir: Dir::Out, .. } => frames_out += 1,
                Record::Tap(_) => taps += 1,
                _ => {}
            }
        }
        assert!(frames_in > 0 && frames_out > 0 && taps > 0, "{name}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A seed and three viewers on cable in Chicago, each linked to the
/// others over TCP, with the WAN model's windows and access queues.
fn wan_mesh() -> Scenario {
    let mut s = Scenario::new("capture_wan");
    let site = Site::new(City::Chicago, Access::Cable);
    let src = s.host_at("seed", site);
    let viewers: Vec<_> = (0..3).map(|i| s.host_at(&format!("v{i}"), site)).collect();
    let all: Vec<_> = std::iter::once(src).chain(viewers.iter().copied()).collect();
    for (i, &a) in all.iter().enumerate() {
        for &b in all.iter().skip(i + 1) {
            s.link(b, a, Transport::Tcp);
        }
    }
    s.wan = Some(strims_wan::Options {
        jitter: false,
        loss: false,
    });
    s.send_buffer = Some(SendBuffer {
        uplink_queue: Duration::from_millis(50),
        window: true,
    });
    s.swarm(
        SwarmSpec::live(
            src,
            viewers,
            64 * 1024,
            Duration::from_secs(2),
            Duration::from_secs(8),
        )
        .with_push_mode(PushMode::Hybrid),
    );
    s.duration = Duration::from_secs(10);
    s
}

#[test]
fn a_captured_wan_run_samples_every_link() {
    let plain = run(wan_mesh(), 1).unwrap();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "a_captured_wan_run_samples_every_link-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let mut spec = wan_mesh();
    spec.capture = Some(dir.clone());
    let captured = run(spec, 1).unwrap();
    assert_eq!(plain.trace_hash, captured.trace_hash);

    for name in ["seed", "v0", "v1", "v2"] {
        let f = std::fs::read(dir.join(format!("{name}-1.ppcap"))).unwrap();
        let mut r = Reader::new(&f[..]).unwrap();
        // Peers each source sampled, and whether a window ever showed.
        let mut sampled: BTreeMap<&str, BTreeSet<u32>> = BTreeMap::new();
        let mut windowed = false;
        while let Some(e) = r.next_record() {
            if let Record::Sample(s) = e.unwrap().record {
                let k = sample_kind(s.source).unwrap().name;
                sampled.entry(k).or_default().insert(s.peer);
                if k == "SimConn" && s.field(2) > 0 {
                    windowed = true;
                }
            }
        }
        assert_eq!(sampled["Queues"].len(), 3, "{name}: {sampled:?}");
        assert_eq!(sampled["SimConn"], sampled["Queues"], "{name}");
        assert!(windowed, "{name}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
