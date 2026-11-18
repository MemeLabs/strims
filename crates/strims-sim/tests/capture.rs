// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A run with ppspp capture on: the trace a run without it gives, and a
//! dump per host that reads back. And a run's tables
//! ([`strims_sim::tables`]): the same trace at any bin width, over TCP and
//! as browsers, seconds that are the series', bins that add up to the
//! row's totals, events that are the scenario's and the report's, stalls
//! that are the reads' playout, parents that are the trees' changes and
//! meter samples that are the meter readings the row scores;
//! and the same trace with viewers departing, gracefully and abruptly, whose
//! nodes hand over what they logged as they go.

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
use strims_sim::tables::UP_COUNTERS;
use strims_sim::topology::random_mesh;
use strims_sim::{
    Departure, Fault, HostSet, Scenario, SctpProfile, SendBuffer, SwarmSpec, Transport,
    WanHostReport, run,
};
use strims_wan::playout::DELAYS_MS;
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
    wan_mesh_of(3, None)
}

/// [`wan_mesh`] with `n` viewers, and the seed's uplink `seed_up` bytes a
/// second if set.
fn wan_mesh_of(n: usize, seed_up: Option<u64>) -> Scenario {
    wan_mesh_at(n, seed_up, 64 * 1024)
}

/// [`wan_mesh_of`] streaming `rate` bytes a second.
fn wan_mesh_at(n: usize, seed_up: Option<u64>, rate: u64) -> Scenario {
    let mut s = Scenario::new("capture_wan");
    let site = Site::new(City::Chicago, Access::Cable);
    let src = s.host_at("seed", seed_up.map_or(site, |up| site.with_up(up)));
    let viewers: Vec<_> = (0..n).map(|i| s.host_at(&format!("v{i}"), site)).collect();
    let all: Vec<_> = std::iter::once(src)
        .chain(viewers.iter().copied())
        .collect();
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
            rate,
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
        // Peers each source sampled, whether a window ever showed, and
        // the peers whose path readings had echoes.
        let mut sampled: BTreeMap<&str, BTreeSet<u32>> = BTreeMap::new();
        let mut windowed = false;
        let mut echoed = BTreeSet::new();
        while let Some(e) = r.next_record() {
            if let Record::Sample(s) = e.unwrap().record {
                let k = sample_kind(s.source).unwrap().name;
                sampled.entry(k).or_default().insert(s.peer);
                if k == "SimConn" && s.field(2) > 0 {
                    windowed = true;
                }
                if k == "Path" && s.field(7) > 0 {
                    // The round trip is at least the WAN path's, and the
                    // queued bytes are within those in flight.
                    assert!(s.field(0) > 0, "{name}");
                    assert!(s.field(5) <= s.field(3), "{name}");
                    echoed.insert(s.peer);
                }
            }
        }
        assert_eq!(sampled["Queues"].len(), 3, "{name}: {sampled:?}");
        assert_eq!(sampled["SimConn"], sampled["Queues"], "{name}");
        assert_eq!(sampled["Path"], sampled["Queues"], "{name}");
        assert_eq!(echoed, sampled["Queues"], "{name}");
        assert!(windowed, "{name}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// [`wan_mesh`] as browsers link: over WebRTC, each link a data channel of
/// the window model on Chromium's or Firefox's SCTP, and to a signalling
/// relay over TCP; at 256 KiB/s, so that a write's burst to a viewer
/// (64 KiB) passes a channel's cap.
fn wan_browsers() -> Scenario {
    let mut s = wan_mesh_at(3, None, 256 * 1024);
    s.over_webrtc(&[Some(SctpProfile::CHROMIUM), Some(SctpProfile::FIREFOX)]);
    s
}

#[test]
fn a_run_with_tables_keeps_its_trace() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "a_run_with_tables_keeps_its_trace-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let tcp = wan_mesh as fn() -> Scenario;
    for (name, build) in [("tcp", tcp), ("browsers", wan_browsers)] {
        let mut spec = build();
        spec.transfer_samples = true;
        let plain = run(spec, 1).unwrap();
        for ms in [1000, 1] {
            let bin = Duration::from_millis(ms);
            let mut spec = build();
            spec.transfer_samples = true;
            spec.wire_bins = Some(bin);
            spec.forecast_log = true;
            spec.request_logs = true;
            spec.packet_log = (0..spec.hosts.len()).collect();
            spec.node_bins = true;
            spec.parent_checks = true;
            spec.timer_log = Some(HostSet::All);
            spec.link_log = Some(HostSet::All);
            let r = run(spec, 1).unwrap();
            assert_eq!(
                (&plain.trace_hash, plain.reads_hash()),
                (&r.trace_hash, r.reads_hash()),
                "{name}: {ms} ms bins"
            );
            let d = dir.join(format!("{name}-{ms}"));
            r.write_tables(&d, bin, true).unwrap();
            let lines = |t: &str| {
                strims_sim::tables::read_table(&d, t)
                    .unwrap()
                    .lines()
                    .count()
            };
            // A column of a table, summed.
            let sum = |t: &str, col: &str| -> i64 {
                let csv = strims_sim::tables::read_table(&d, t).unwrap();
                let mut rows = csv.lines();
                let head: Vec<&str> = rows.next().unwrap().split(',').collect();
                let i = head.iter().position(|&c| c == col).unwrap();
                // An empty field is a reading the run doesn't have.
                rows.filter_map(|l| l.split(',').nth(i).unwrap().parse::<i64>().ok())
                    .sum()
            };
            let both = |col: &str| {
                sum("bins", &format!("viewer_{col}")) + sum("bins", &format!("seed_{col}"))
            };
            // The stream starts at 2 s and the run ends at 10 s.
            assert_eq!(lines("seconds"), 8 + 1);
            assert_eq!(lines("viewer_seconds"), 3 * 8 + 1);
            let reads: usize = r.viewers.iter().map(|v| v.stats.lags.len()).sum();
            assert_eq!(lines("reads"), reads + 1);
            assert!(lines("bins") as u64 + 1 >= 7_000 / ms, "{name}: {ms} ms");
            let seed_bytes = sum("bins", "seed_up_bytes");
            assert!(seed_bytes > 64 * 1024 * 6, "{name}: {seed_bytes}");
            // The nodes' own bins: their pushes and their messages' bytes.
            assert!(sum("bins", "hits") > 0, "{name}: {ms} ms");
            assert!(sum("bins", "viewer_have_bytes") > 0, "{name}: {ms} ms");
            assert!(sum("bins", "sent") > 0, "{name}: {ms} ms");
            // The bins' parent changes, sheds and dropped answers add up to
            // the runners' totals (every host's, none of them hostile here).
            let c = r.swarm_counters(0);
            assert_eq!(
                [
                    both("switched"),
                    both("shed"),
                    both("sheds_sent"),
                    both("answers_refused"),
                    both("answers_dropped"),
                ],
                [
                    c.switched,
                    c.shed,
                    c.sheds_sent,
                    c.answers_refused,
                    c.answers_dropped,
                ]
                .map(|x| x as i64),
                "{name}: {ms} ms"
            );
            // The bins' acknowledgements alone (TCP's pure ACKs, the data
            // channels' SACKs) are the seed's and the viewers' between the
            // first edge and the last; with theirs before and after it, and
            // the relay's, they are the row's: every host's, the run through.
            let row: serde_json::Value = serde_json::from_str(&r.row(name, None)).unwrap();
            let wan = r.wan.as_ref().unwrap();
            assert_eq!(wan.hosts.len(), r.scenario.hosts.len());
            let (first, last) = (r.wire_bins.first().unwrap(), r.wire_bins.last().unwrap());
            let swarm = &r.scenario.swarms[0];
            let binned: BTreeSet<usize> = std::iter::once(swarm.seed)
                .chain(swarm.viewers.iter().copied())
                .collect();
            let ix = |c: &str| UP_COUNTERS.iter().position(|&n| n == c).unwrap();
            for (cols, key) in [
                (["up_ack_packets", "up_sack_packets"], "packets_ack"),
                (["up_ack_bytes", "up_sack_bytes"], "up_acks"),
            ] {
                let total = |w: &WanHostReport| match key {
                    "packets_ack" => w.acks,
                    _ => w.ack_bytes,
                };
                let at = |edge: &[u64]| -> u64 { cols.iter().map(|c| edge[ix(c)]).sum() };
                let in_bins: i64 = cols.iter().map(|c| both(c)).sum();
                let between: u64 = binned.iter().map(|&h| at(&last[h]) - at(&first[h])).sum();
                assert_eq!(in_bins as u64, between, "{name}: {ms} ms: {key}");
                let outside: u64 = wan
                    .hosts
                    .iter()
                    .enumerate()
                    .map(|(h, w)| match binned.contains(&h) {
                        true => at(&first[h]) + total(w).checked_sub(at(&last[h])).unwrap(),
                        false => total(w),
                    })
                    .sum();
                assert_eq!(
                    in_bins as u64 + outside,
                    row[key].as_u64().unwrap(),
                    "{name}: {ms} ms: {key}"
                );
            }
            // TCP links carry pure ACKs alone, data channels SACKs too.
            assert!(both("up_ack_packets") > 0, "{name}: {ms} ms");
            assert!(sum("node_bins", "up_ack_bytes") > 0, "{name}: {ms} ms");
            if name == "tcp" {
                assert_eq!(both("up_sack_packets") + both("up_sack_bytes"), 0);
            } else {
                assert!(both("up_sack_packets") > 0, "{name}: {ms} ms");
                assert!(sum("bins", "seed_up_sack_bytes") > 0, "{name}: {ms} ms");
                assert!(sum("node_bins", "up_sack_bytes") > 0, "{name}: {ms} ms");
            }
            assert!(lines("node_bins") > lines("bins"), "{name}: {ms} ms");
            assert!(lines("events") > 3, "{name}: {ms} ms: a join per viewer");
            assert!(lines("forecast") > 1, "{name}: {ms} ms");
            budget_reading_is_the_budgets(&r, &d);
            meter_samples_say_what_they_are(&r, &d);
            peer_samples_say_what_they_are(&r, &d);
            queues_say_what_they_are(&r, &d);
            assert!(
                !d.join("sockets.csv.zst").exists() && !d.join("sockets.csv").exists(),
                "{name}: a sim run writes no sockets"
            );
            held_truth_meets_the_reads(&r, &d);
            request_logs_say_what_they_are(&r, &d);
            timers_say_what_they_are(&r, &d);
            link_logs_say_what_they_are(&r, &d, name);
            stalls_say_how_playout_went(&d);
            parents_say_how_the_trees_changed(&r, &d);
            packets_say_what_they_are(&r, &d, name);
            parent_checks_say_what_they_are(&r, &d);
        }
    }
    // The departure path: a node that goes hands over its last snapshot
    // (`NodeConfig::last_metrics`) and its parent log's last changes.
    for how in ["graceful", "abrupt"] {
        let mut spec = departing_wan(how);
        spec.transfer_samples = true;
        let plain = run(spec, 1).unwrap();
        let mut spec = departing_wan(how);
        spec.transfer_samples = true;
        spec.wire_bins = Some(Duration::from_secs(1));
        spec.forecast_log = true;
        spec.request_logs = true;
        spec.parent_checks = true;
        spec.timer_log = Some(HostSet::All);
        spec.link_log = Some(HostSet::All);
        let r = run(spec, 1).unwrap();
        assert_eq!(
            (&plain.trace_hash, plain.reads_hash()),
            (&r.trace_hash, r.reads_hash()),
            "{how}"
        );
        assert!(!r.parent_losses.is_empty(), "{how}: no parent departed");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The timer log reads the lateness the run draws: the same trace with the
/// log as without it, lateness on and off, and its rows as
/// [`timers_say_what_they_are`] says, the timer probe's too.
#[test]
fn a_timer_log_reads_the_lateness_drawn() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "a_timer_log_reads_the_lateness_drawn-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    for late in [false, true] {
        let mut spec = wan_mesh();
        spec.timer_lateness = late;
        let plain = run(spec, 2).unwrap();
        let mut spec = wan_mesh();
        spec.timer_lateness = late;
        spec.wire_bins = Some(Duration::from_secs(1));
        spec.timer_log = Some(HostSet::All);
        spec.forecast_log = true;
        let r = run(spec, 2).unwrap();
        assert_eq!(
            (&plain.trace_hash, plain.reads_hash()),
            (&r.trace_hash, r.reads_hash()),
            "lateness {late}"
        );
        let d = dir.join(format!("late-{late}"));
        r.write_tables(&d, Duration::from_secs(1), false).unwrap();
        timers_say_what_they_are(&r, &d);
        // The seed stores each chunk at its write, late or on time.
        held_truth_meets_the_reads(&r, &d);
        // And the probe's, on the same model.
        let mut spec = wan_mesh();
        spec.timer_lateness = late;
        spec.wire_bins = Some(Duration::from_secs(1));
        spec.timer_log = Some(HostSet::All);
        spec.timer_probe = true;
        let p = run(spec, 2).unwrap();
        let pd = dir.join(format!("probe-{late}"));
        p.write_tables(&pd, Duration::from_secs(1), false).unwrap();
        timers_say_what_they_are(&p, &pd);
        if late {
            let rows = table_rows(&d, "timers");
            assert!(
                rows.iter().any(|x| int(x, "late_us") != Some(0)),
                "no timer drawn late"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Six viewers and a seed on cable in Chicago, a full mesh over TCP in
/// Hybrid, the seed capped at three copies of the stream (so viewers
/// relay), half the viewers leaving (`how`) within a second from 8 s into
/// the stream; the seed's site uplink halves 2 s in, and a viewer's cap
/// halves 4 s in.
fn departing_wan(how: &str) -> Scenario {
    let mut s = Scenario::new("events_wan");
    let site = Site::new(City::Chicago, Access::Cable);
    let seed = s.host_at("seed", site);
    let viewers: Vec<_> = (1..7).map(|i| s.host_at(&format!("v{i}"), site)).collect();
    let all: Vec<_> = std::iter::once(seed).chain(viewers).collect();
    for (i, &a) in all.iter().enumerate() {
        for &b in all.iter().skip(i + 1) {
            s.link(b, a, Transport::Tcp);
        }
    }
    s.wan = Some(strims_wan::Options {
        jitter: false,
        loss: false,
    });
    s.swarm(
        SwarmSpec::live(
            all[0],
            all[1..].to_vec(),
            64 * 1024,
            Duration::from_secs(2),
            Duration::from_secs(14),
        )
        .with_push_mode(PushMode::Hybrid),
    );
    s.departures = Some(format!("50:8:1:{how}").parse().unwrap());
    s.hosts[all[0]].uplink = Some(3 * 64 * 1024);
    s.hosts[all[0]].up_steps = vec![(Duration::from_secs(4), site.profile.up / 2)];
    let cap = s.hosts[all[1]].uplink.unwrap();
    s.at(Duration::from_secs(6), Fault::Uplink(all[1], cap / 2));
    s.duration = Duration::from_secs(16);
    s
}

/// The `events` table's levels, departures and parent losses, against
/// the report and `series_meta.json`: the seed's `uplink_rate` levels are
/// its `seed_up` steps (the site's rate at the run's start, then each
/// step), a capped host's `rate_limit` starts at its cap; a `depart`'s
/// value is its detail's (1 abrupt); each parent loss
/// ([`strims_sim::Report::parent_losses`]) is a `parent_departed` at its
/// parent's `depart` (the streams it parented, how it left) and, once the
/// viewer noticed, a `parent_noticed` (the ms since); `series_meta.json`
/// names the seed and the stream count. At seed 3, whose runs lose
/// parents both noticed and not: with the timer lateness on (the
/// default), seeds 1 and 2 notice every loss, and seeds 5 to 7 lose none.
#[test]
fn a_runs_events_are_its_scenarios_and_losses() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "a_runs_events_are_its_scenarios_and_losses-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    // Losses noticed, and not: an abrupt departure's child that left too.
    let mut noticed = [0, 0];
    for how in ["graceful", "abrupt"] {
        let mut spec = departing_wan(how);
        spec.transfer_samples = true;
        spec.wire_bins = Some(Duration::from_millis(1000));
        let r = run(spec, 3).unwrap();
        let d = dir.join(how);
        r.write_tables(&d, Duration::from_millis(1000), true)
            .unwrap();
        let csv = strims_sim::tables::read_table(&d, "events").unwrap();
        let events: Vec<(i64, String, i64, i64, i64, String)> = csv
            .lines()
            .skip(1)
            .map(|l| {
                let f: Vec<&str> = l.split(',').collect();
                let int = |i: usize| f[i].parse::<i64>().unwrap();
                (int(0), f[1].into(), int(2), int(3), int(4), f[5].into())
            })
            .collect();
        let of = |kind: &str| -> Vec<&(i64, String, i64, i64, i64, String)> {
            events.iter().filter(|e| e.1 == kind).collect()
        };
        let start = r.scenario.swarms[0].start;
        let us = |at: Duration| at.as_micros() as i64 - start.as_micros() as i64;
        let seed = r.scenario.swarms[0].seed;
        let meta: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(d.join("series_meta.json")).unwrap())
                .unwrap();
        assert_eq!(meta["seed"], seed, "{how}");
        let streams = r.scenario.swarms[0].options.stream_count.max(1);
        assert_eq!(meta["streams"], streams, "{how}");
        let seed_up: Vec<(f64, i64)> = meta["seed_up"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p[0].as_f64().unwrap(), p[1].as_i64().unwrap()))
            .collect();
        let up = of("uplink_rate");
        assert!(
            up.iter().all(|e| e.2 == seed as i64 && e.3 == -1),
            "{how}: {up:?}"
        );
        assert_eq!(up.len(), seed_up.len(), "{how}: {up:?} {seed_up:?}");
        assert!(up[0].0 <= 0, "{how}: {up:?}");
        for (e, &(s, v)) in up.iter().zip(&seed_up) {
            assert_eq!(e.4, v, "{how}");
            assert!(
                (e.0.max(0) as f64 / 1e6 - s).abs() < 0.05,
                "{how}: {e:?} {s}"
            );
        }
        let capped = r.scenario.swarms[0].viewers[0];
        let cap = r.scenario.hosts[capped].uplink.unwrap() as i64;
        let caps: Vec<(i64, i64, i64)> = of("rate_limit").iter().map(|e| (e.0, e.2, e.4)).collect();
        assert_eq!(
            caps,
            [
                (us(Duration::ZERO), capped as i64, cap),
                (us(Duration::from_secs(6)), capped as i64, cap / 2)
            ],
            "{how}"
        );
        let departs = of("depart");
        assert!(!departs.is_empty(), "{how}");
        for e in &departs {
            assert_eq!(
                (e.4, e.5.as_str()),
                (i64::from(how == "abrupt"), how),
                "{e:?}"
            );
        }
        let at = |h: usize| departs.iter().find(|e| e.2 == h as i64).unwrap().0;
        let mut want: Vec<(i64, i64, i64, i64, String)> = r
            .parent_losses
            .iter()
            .map(|l| {
                (
                    at(l.parent),
                    l.viewer as i64,
                    l.parent as i64,
                    l.streams as i64,
                    how.into(),
                )
            })
            .collect();
        let mut got: Vec<_> = of("parent_departed")
            .iter()
            .map(|e| (e.0, e.2, e.3, e.4, e.5.clone()))
            .collect();
        want.sort();
        got.sort();
        assert_eq!(got, want, "{how}");
        let mut want: Vec<(i64, i64, i64, i64)> = r
            .parent_losses
            .iter()
            .filter_map(|l| {
                let n = l.noticed_ms?;
                let t = us(Duration::from_millis(n));
                Some((
                    t,
                    l.viewer as i64,
                    l.parent as i64,
                    (n - l.departed_ms) as i64,
                ))
            })
            .collect();
        let mut got: Vec<_> = of("parent_noticed")
            .iter()
            .map(|e| (e.0, e.2, e.3, e.4))
            .collect();
        want.sort();
        got.sort();
        assert_eq!(got, want, "{how}");
        assert!(
            of("parent_lost").is_empty()
                && of("up_step").is_empty()
                && of("first_frame").is_empty()
        );
        assert!(!r.parent_losses.is_empty(), "{how}: no parent departed");
        for l in &r.parent_losses {
            noticed[usize::from(l.noticed_ms.is_none())] += 1;
        }
    }
    assert!(noticed[0] > 0 && noticed[1] > 0, "{noticed:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The faults of [`skipping`], with whether the run applies each (with
/// the WAN model if `wan`), in the order the scenario gives them: out of
/// time order, and two pairs at one time with different fates. v2 leaves
/// at 3 s, so its crash and restart then find no host, nor does its second
/// departure, given before its first; v3 arrives at 4 s, after its crash
/// and restart; v1's second
/// crash finds it down and leaves its restart to come, and its second
/// restart finds it up, as does v0's restart without a crash; the one-way
/// latency needs the WAN model; and v0's pause, uplink cap and departure
/// come past the run's end.
fn skipping_faults(wan: bool) -> Vec<(Duration, Fault, bool)> {
    let ms = Duration::from_millis;
    let latency = |a: usize, b: usize, d: u64| Fault::Latency(vec![a], vec![b], ms(d));
    let one_way = Fault::LatencyFrom(vec![0], vec![1], ms(20));
    vec![
        (ms(6000), Fault::Restart(2), true),
        (ms(5000), Fault::Crash(3), false),
        (ms(7000), Fault::Depart(3, Departure::Abrupt), false),
        (ms(3000), Fault::Depart(3, Departure::Graceful), true),
        (ms(5000), Fault::Crash(2), true),
        (ms(5500), Fault::Crash(2), false),
        (ms(6000), Fault::Restart(3), false),
        (ms(1000), one_way, wan),
        (ms(1000), latency(1, 2, 10), true),
        (ms(3000), Fault::Crash(4), false),
        (ms(3500), Fault::Restart(4), false),
        (ms(6500), Fault::Restart(2), false),
        (ms(7000), Fault::Restart(1), false),
        (ms(20_000), Fault::PauseReading(1), false),
        (ms(20_000), Fault::Uplink(1, 64 * 1024), false),
        (ms(25_000), Fault::Depart(1, Departure::Graceful), false),
    ]
}

/// A seed and four viewers, in a full mesh, under the WAN model if `wan`,
/// v3 arriving at 4 s, with [`skipping_faults`].
fn skipping(wan: bool) -> Scenario {
    let mut s = Scenario::new("skipping");
    let site = Site::new(City::Chicago, Access::Cable);
    let seed = s.host_at("seed", site);
    let v: Vec<_> = (0..4).map(|i| s.host_at(&format!("v{i}"), site)).collect();
    let all: Vec<_> = std::iter::once(seed).chain(v.iter().copied()).collect();
    for (i, &a) in all.iter().enumerate() {
        for &b in all.iter().skip(i + 1) {
            s.link(b, a, Transport::Tcp);
        }
    }
    // A host at a site puts the run in WAN mode; without it, turmoil's.
    s.wan = wan.then_some(strims_wan::Options {
        jitter: false,
        loss: false,
    });
    s.hosts[v[3]].arrive_at = Some(Duration::from_secs(4));
    s.swarm(
        SwarmSpec::live(
            seed,
            v.clone(),
            32 * 1024,
            Duration::from_secs(2),
            Duration::from_secs(8),
        )
        .with_push_mode(PushMode::Hybrid),
    );
    for (at, f, _) in skipping_faults(wan) {
        s.at(at, f);
    }
    s.duration = Duration::from_secs(10);
    s
}

/// A fault at the scenario's duration is one the run never reaches, as
/// one past it is, though the hosts' teardown runs the sim on past it, and
/// one just before it applies: the boundary strims-sim's testbed reports
/// take too (`Scenario::reaches`), so a twin and its testbed run skip the
/// same faults.
#[test]
fn a_fault_at_the_runs_end_is_unreached() {
    for wan in [true, false] {
        let mut spec = skipping(wan);
        spec.faults.clear();
        let end = spec.duration;
        spec.at(end - Duration::from_millis(1), Fault::PauseReading(1));
        spec.at(end, Fault::PauseReading(2));
        spec.at(end + Duration::from_millis(1), Fault::PauseReading(1));
        assert!(spec.reaches(end - Duration::from_millis(1)) && !spec.reaches(end));
        let r = run(spec, 1).unwrap();
        // The hosts' teardown runs the sim past its duration.
        assert!(r.sim_secs > end.as_secs_f64(), "wan {wan}: {}", r.sim_secs);
        let skipped: Vec<Duration> = r
            .skipped_faults
            .iter()
            .map(|&i| r.scenario.faults[i].at)
            .collect();
        assert_eq!(skipped, [end, end + Duration::from_millis(1)], "wan {wan}");
    }
}

/// The tables read the faults the run applied and leave out those it
/// skipped ([`strims_sim::Report::skipped_faults`], by their index in the
/// time-ordered faults, each named in the trace but the one past the
/// run's end): the `events` table has a row for each applied fault alone,
/// a one-way latency's detail `one-way` and a two-way one's `both`, and the
/// `arrivals` table counts a host's incarnations by its applied restarts,
/// so v3 stays in its first and v1 is in its second after 6 s.
#[test]
fn a_skipped_fault_has_no_event() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "a_skipped_fault_has_no_event-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    for wan in [true, false] {
        let mut spec = skipping(wan);
        spec.wire_bins = Some(Duration::from_millis(1000));
        spec.request_logs = true;
        let r = run(spec, 1).unwrap();
        let d = dir.join(if wan { "wan" } else { "turmoil" });
        r.write_tables(&d, Duration::from_millis(1000), true)
            .unwrap();
        // The faults in time order, a time's in the scenario's order.
        let mut faults = skipping_faults(wan);
        faults.sort_by_key(|f| f.0);
        let skipped: Vec<(Duration, &Fault)> = r
            .skipped_faults
            .iter()
            .map(|&i| (r.scenario.faults[i].at, &r.scenario.faults[i].fault))
            .collect();
        let want: Vec<(Duration, &Fault)> = faults
            .iter()
            .filter(|f| !f.2)
            .map(|f| (f.0, &f.1))
            .collect();
        assert_eq!(skipped, want, "wan {wan}");
        let skips = r.trace.iter().filter(|l| l.contains(" skipped: ")).count();
        assert_eq!(
            skips,
            want.len() - 3,
            "wan {wan}: all but the three past the end"
        );
        // The events of the fault kinds: (time, kind, node, peer, value,
        // detail), sorted.
        let csv = strims_sim::tables::read_table(&d, "events").unwrap();
        let start = r.scenario.swarms[0].start;
        let us = |at: Duration| at.as_micros() as i64 - start.as_micros() as i64;
        let mut got: Vec<(i64, String, i64, i64, i64, String)> = csv
            .lines()
            .skip(1)
            .map(|l| l.split(',').collect::<Vec<_>>())
            .filter(|f| {
                ["crash", "restart", "latency", "depart", "pause_reading"].contains(&f[1])
                    || (f[1] == "rate_limit" && f[2] == "1")
            })
            .map(|f| {
                let int = |i: usize| f[i].parse::<i64>().unwrap();
                (
                    int(0),
                    f[1].to_owned(),
                    int(2),
                    int(3),
                    int(4),
                    f[5].to_owned(),
                )
            })
            .collect();
        let row = |at: u64, kind: &str, node: i64, peer: i64, value: i64, detail: &str| {
            let at = us(Duration::from_millis(at));
            (at, kind.to_owned(), node, peer, value, detail.to_owned())
        };
        let mut want = vec![
            row(1000, "latency", 1, 2, 10, "both"),
            row(3000, "depart", 3, -1, 0, "graceful"),
            row(5000, "crash", 2, -1, 0, ""),
            row(6000, "restart", 2, -1, 0, ""),
        ];
        if wan {
            want.push(row(1000, "latency", 0, 1, 20, "one-way"));
        }
        got.sort();
        want.sort();
        assert_eq!(got, want, "wan {wan}");
        // v1's incarnation from its restart on, v3's never past its first.
        let csv = strims_sim::tables::read_table(&d, "arrivals").unwrap();
        let mut rows = csv.lines();
        let head: Vec<&str> = rows.next().unwrap().split(',').collect();
        let col = |n: &str| head.iter().position(|&c| c == n).unwrap();
        let (t, host, inc) = (col("t_ms"), col("node"), col("incarnation"));
        let mut seen = [0, 0];
        for l in rows {
            let f: Vec<&str> = l.split(',').collect();
            let (h, at) = (
                f[host].parse::<usize>().unwrap(),
                f[t].parse::<i64>().unwrap(),
            );
            let at = at + start.as_millis() as i64;
            let want = match h {
                2 if at >= 6000 => "1",
                4 => "0",
                _ => continue,
            };
            assert_eq!(f[inc], want, "wan {wan}: {l}");
            seen[usize::from(h == 4)] += 1;
        }
        assert!(seen[0] > 0 && seen[1] > 0, "wan {wan}: {seen:?}");
        // v2's node left at its earliest departure, the graceful one at 3 s
        // given after the abrupt one at 7 s: it read nothing past it.
        let v2 = r.viewers.iter().find(|v| v.host == 3).unwrap();
        let last = v2
            .stats
            .lags
            .iter()
            .map(|&(seq, lag)| r.written_at(v2.swarm, seq) + Duration::from_millis(lag.into()))
            .max()
            .unwrap();
        assert!(
            last <= Duration::from_secs(3),
            "wan {wan}: v2 read at {last:?}"
        );
        // v2 left at 3 s; v0's departure past the end leaves it in.
        let csv = strims_sim::tables::read_table(&d, "viewers").unwrap();
        let mut rows = csv.lines();
        let head: Vec<&str> = rows.next().unwrap().split(',').collect();
        let col = |n: &str| head.iter().position(|&c| c == n).unwrap();
        let (host, depart) = (col("host"), col("depart_ms"));
        let departs: Vec<(String, String)> = rows
            .map(|l| l.split(',').collect::<Vec<_>>())
            .map(|f| (f[host].to_owned(), f[depart].to_owned()))
            .collect();
        let left = us(Duration::from_secs(3)) / 1000;
        assert_eq!(
            departs,
            [
                ("1".to_owned(), String::new()),
                ("2".to_owned(), String::new()),
                ("3".to_owned(), left.to_string()),
                ("4".to_owned(), String::new()),
            ],
            "wan {wan}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A node's logs and counters since its last stats sample are kept when
/// it leaves, gracefully or by a crash: with half the viewers gone
/// mid-stream, every read and every honest answer and HAVE still has its
/// hold in `arrivals` ([`every_read_and_answer_has_its_hold`]), the
/// leavers' last ones too, every record of a leaver names it
/// ([`the_logs_name_their_peers`]), the expired requests are the
/// runners' timed out ones, the leavers' among them, and the stalls and
/// parents tables say how playout went and how the trees changed
/// ([`stalls_say_how_playout_went`], [`parents_say_how_the_trees_changed`]).
#[test]
fn a_departures_logs_keep_every_hold() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "a_departures_logs_keep_every_hold-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    for how in ["graceful", "abrupt"] {
        let mut spec = departing_wan(how);
        spec.wire_bins = Some(Duration::from_millis(1000));
        spec.forecast_log = true;
        spec.request_logs = true;
        let r = run(spec, 1).unwrap();
        let d = dir.join(how);
        r.write_tables(&d, Duration::from_millis(1000), true)
            .unwrap();
        every_read_and_answer_has_its_hold(&r, &d);
        the_logs_name_their_peers(&d, how);
        let expired = table_rows(&d, "asks")
            .iter()
            .filter(|a| a["picked"] == "true" && a["outcome"] == "expired")
            .count() as u64;
        let (a, b, c) = r.timeouts(0);
        assert_eq!(expired, a + b + c, "{how}");
        stalls_say_how_playout_went(&d);
        parents_say_how_the_trees_changed(&r, &d);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every record of a neighbour names its host, a neighbour gone before
/// the logs' take among them (node-core keeps a removed peer's host until
/// the next take): the seed's sends, its decisions but a window's and an
/// answer rate's (about the allocation), the asks and the HAVEs.
fn the_logs_name_their_peers(d: &Path, how: &str) {
    for t in ["seed_sends", "seed_decisions", "asks", "haves"] {
        let rows = table_rows(d, t);
        assert!(!rows.is_empty(), "{how}: {t}");
        let unnamed = rows
            .iter()
            .filter(|x| x["peer"] == "-1" || x["peer"].is_empty())
            .filter(|x| {
                !(t == "seed_decisions" && ["window", "answer_rate"].contains(&x["kind"].as_str()))
            })
            .count();
        assert_eq!(
            unnamed,
            0,
            "{how}: {unnamed} of {t}'s {} rows name no peer",
            rows.len()
        );
    }
}

/// [`wan_mesh`] whose seed's site uplink falls to half the stream's rate
/// from 4 s to 7 s into the run (2 s to 5 s into the stream), then comes
/// back: the viewers fall behind and stall.
fn starved_wan() -> Scenario {
    let mut s = wan_mesh();
    let seed = s.swarms[0].seed;
    let up = s.hosts[seed].site.unwrap().profile.up;
    s.hosts[seed].up_steps = vec![
        (Duration::from_secs(4), 32 * 1024),
        (Duration::from_secs(7), up),
    ];
    s
}

/// The `stalls` and `parents` tables of a run whose viewers stall, its
/// seed starved for 3 s ([`starved_wan`]), say how their playout went and
/// how the trees changed ([`stalls_say_how_playout_went`],
/// [`parents_say_how_the_trees_changed`]): stalls at the shorter delay.
/// And its `meter_samples` say what the seed could upload at each sample
/// ([`meter_samples_say_what_they_are`]): over the second before, its
/// rate limit, or while starved half the stream's rate in its meter's units
/// (its share of its link, `own_share`).
#[test]
fn a_starved_runs_stalls_are_its_reads_playout() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "a_starved_runs_stalls_are_its_reads_playout-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let mut spec = starved_wan();
    spec.wire_bins = Some(Duration::from_secs(1));
    let r = run(spec, 1).unwrap();
    r.write_tables(&dir, Duration::from_secs(1), false).unwrap();
    let stalls = stalls_say_how_playout_went(&dir);
    let short = table_rows(&dir, "stalls")
        .iter()
        .filter(|s| int(s, "delay_ms") == Some(DELAYS_MS[0]))
        .count();
    assert!(
        stalls > 0 && short > 0,
        "{stalls} stalls, {short} at the shorter delay"
    );
    parents_say_how_the_trees_changed(&r, &dir);
    // What the seed could upload over the second before a sample is the
    // smaller of its rate limit and its data share of its site's rate, and
    // over a second its site was starved, its share of 32 KiB/s.
    let seed = r.scenario.swarms[0].seed;
    let up = r.upload_cap(seed).unwrap() as i64;
    assert!(up > 32 * 1024);
    let limit = r.scenario.hosts[seed].uplink.unwrap() as i64;
    let site_up = r.scenario.hosts[seed].site.unwrap().profile.up;
    let meter = &r.hosts[seed].runner.meter;
    // The table's truth at a sample is the oracle's as the sample was
    // asked for: from the samples asked before it (`Report::upload_cap_at`),
    // the rate limit in what vnic hands the transports (its channel share).
    let last = |t: Duration| {
        let asked = meter.iter().find(|m| m.t == t).map_or(t, |m| m.asked);
        let k = meter.partition_point(|m| m.asked < asked);
        k.checked_sub(1).and_then(|i| meter.get(i))
    };
    let share = |t: Duration| strims_sim::report::own_share(&r.scenario.hosts[seed], last(t));
    let limit_at = |t: Duration| (limit as f64 * strims_sim::report::channel_share(last(t))) as i64;
    let secs = Duration::from_secs;
    let mut starved_samples = 0;
    for (t, row) in meter_samples_say_what_they_are(&r, &dir) {
        let want = if t >= secs(5) && t <= secs(7) {
            starved_samples += 1;
            (32.0 * 1024.0 * share(t)) as i64
        } else if t < secs(4) || t >= secs(8) {
            limit_at(t).min((site_up as f64 * share(t)) as i64)
        } else {
            // A second that saw the step.
            continue;
        };
        assert_eq!(int(&row, "avail_bps"), Some(want), "at {t:?}");
    }
    assert!(starved_samples > 0, "a sample while starved");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The seed's `budget_capacity` is the reading its budget was measured
/// from: at every meter sample once measured, the budget is what 70 % of
/// it pays for at a subscriber's cost, the cost within a factor of two of
/// the stream's (the budget takes the store's rate over its last second,
/// which a bursty writer swings); and the series' and the bins' columns
/// carry it, empty until measured. ppspp's unit tests check the reading
/// exactly.
fn budget_reading_is_the_budgets(r: &strims_sim::Report, d: &Path) {
    use strims_ppspp::scheduler_peer::push::PUSH_OVERHEAD;
    use strims_ppspp::scheduler_peer::push::seed::{ALLOC_UTILIZATION, OVER_BUDGET};
    let swarm = &r.scenario.swarms[0];
    let seed = &r.hosts[swarm.seed].runner;
    let streams = swarm.options.stream_count.max(1);
    let cost = swarm.rate() / streams as f64 * PUSH_OVERHEAD;
    let mut checked = 0;
    for m in &seed.meter {
        let Some(&cap) = m.budget_capacity.get(&0).filter(|&&c| c > 0) else {
            continue;
        };
        let budget = seed
            .seed_changes
            .iter()
            .rev()
            .find(|(t, s, _)| *s == 0 && *t <= m.t)
            .map(|x| x.2.budget)
            .unwrap();
        // The store's rate falls off once the stream stops.
        if budget > streams && m.t < swarm.stop {
            // The cost the budget implies, before its hysteresis.
            let (lo, hi) = (
                cap as f64 * ALLOC_UTILIZATION / (budget + 1) as f64,
                cap as f64 * ALLOC_UTILIZATION / (budget as f64 * (1.0 - OVER_BUDGET)),
            );
            assert!(
                hi >= cost / 2.0 && lo <= cost * 2.0,
                "at {:?}: budget {budget} from {cap} B/s at {cost} B/s a subscriber",
                m.t
            );
        }
        checked += 1;
    }
    assert!(checked > 0, "the budget was measured");
    let column = |t: &str| -> Vec<i64> {
        let csv = strims_sim::tables::read_table(d, t).unwrap();
        let mut rows = csv.lines();
        let head: Vec<&str> = rows.next().unwrap().split(',').collect();
        let i = head.iter().position(|&c| c == "budget_capacity").unwrap();
        rows.filter_map(|l| l.split(',').nth(i).unwrap().parse().ok())
            .collect()
    };
    let readings: BTreeSet<i64> = seed
        .meter
        .iter()
        .filter_map(|m| m.budget_capacity.get(&0).map(|&c| c as i64))
        .collect();
    let seconds = column("seconds");
    assert!(seconds.iter().any(|&c| c > 0));
    assert!(
        seconds
            .iter()
            .all(|c| *c == -1 || (*c > 0 && readings.contains(c)))
    );
    let bins = column("bins");
    assert!(!bins.is_empty() && bins.iter().all(|&c| c > 0));
}

/// With the scheduler's logs on, the seed's tables say what it decided and
/// what became of each chunk it sent ([`seed_tables_say_what_the_seed_did`]),
/// on the asks test's mesh: 8 viewers behind a seed with the uplink for
/// about 3 copies of the stream, so it holds copies at some and declines
/// the rest.
#[test]
fn the_seed_tables_say_what_the_seed_did() {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "the_seed_tables_say_what_the_seed_did-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    let bin = Duration::from_millis(1000);
    let mut spec = wan_mesh_of(8, Some(200_000));
    spec.wire_bins = Some(bin);
    spec.request_logs = true;
    let r = run(spec, 1).unwrap();
    r.write_tables(&d, bin, true).unwrap();
    seed_tables_say_what_the_seed_did(&r, &d);
    let meta = std::fs::read_to_string(d.join("series_meta.json")).unwrap();
    assert!(meta.contains("\"seed_log_lost\":0"), "{meta}");
    let _ = std::fs::remove_dir_all(&d);
}

/// The seed's `seed_sends` and `seed_decisions` against what the rest of
/// the run shows: every push written to a holder has the holder's hold in
/// `arrivals` (a push written in the run's last second may arrive past its
/// end); its sends' fates count what its runner's counters count (pushes
/// and answers written, pushes dropped as stale, and as cancelled or
/// announced by the peer's HAVEs); every
/// open of a push parent at the seed (`parents`) follows a request the seed
/// granted that neighbour for that stream; and every row is the seed's.
fn seed_tables_say_what_the_seed_did(r: &strims_sim::Report, d: &Path) {
    let seed = r.scenario.swarms[0].seed as i64;
    let sends = table_rows(d, "seed_sends");
    let decisions = table_rows(d, "seed_decisions");
    assert!(!sends.is_empty() && !decisions.is_empty());
    assert!(
        sends
            .iter()
            .chain(&decisions)
            .all(|x| int(x, "node") == Some(seed))
    );
    // Each table numbers the seed's records from 0, in its log's order.
    for t in [&sends, &decisions] {
        let n: Vec<i64> = t.iter().map(|x| int(x, "n").unwrap()).collect();
        assert_eq!(n, (0..t.len() as i64).collect::<Vec<_>>());
    }
    let count = |how: &str, fate: &str| {
        sends
            .iter()
            .filter(|x| x["how"] == how && x["fate"] == fate)
            .count() as u64
    };
    let c = r.hosts[r.scenario.swarms[0].seed].runner.swarm(0);
    assert_eq!(count("push", "written"), c.pushed_chunks, "{c:?}");
    assert_eq!(
        count("answer", "written"),
        c.chunks_sent - c.pushed_chunks,
        "{c:?}"
    );
    assert_eq!(count("push", "stale"), c.stale_drops, "{c:?}");
    assert_eq!(
        count("push", "cancelled") + count("push", "suppressed"),
        c.suppressed,
        "{c:?}"
    );
    assert!(count("push", "written") > 0);

    // Each host's holds, by chunk and incarnation.
    let mut held: BTreeSet<(i64, i64, i64)> = BTreeSet::new();
    for a in table_rows(d, "arrivals") {
        held.insert((
            int(&a, "node").unwrap(),
            int(&a, "seq").unwrap(),
            int(&a, "incarnation").unwrap(),
        ));
    }
    let end =
        r.scenario.duration.as_millis() as i64 - r.scenario.swarms[0].start.as_millis() as i64;
    let mut checked = 0;
    for x in sends
        .iter()
        .filter(|x| x["how"] == "push" && x["fate"] == "written")
    {
        let t = int(x, "t_ms").unwrap();
        let key = (
            int(x, "peer").unwrap(),
            int(x, "seq").unwrap(),
            int(x, "peer_incarnation").unwrap(),
        );
        assert!(
            held.contains(&key) || t > end - 1000,
            "a written push never held: {x:?}"
        );
        // Written, a push left the seed's queue no sooner than it entered.
        assert!(int(x, "queued_ms").unwrap() <= t, "{x:?}");
        assert!(int(x, "bytes").unwrap() > 0, "{x:?}");
        checked += 1;
    }
    assert!(checked > 0);

    // Each open of the seed as a push parent follows the seed's grant of
    // that neighbour's request for that stream.
    let grants: Vec<(i64, i64, i64)> = decisions
        .iter()
        .filter(|x| x["kind"] == "request" && x["granted"] == "true")
        .map(|x| {
            (
                int(x, "peer").unwrap(),
                int(x, "stream").unwrap(),
                int(x, "t_ms").unwrap(),
            )
        })
        .collect();
    let mut opens = 0;
    for p in table_rows(d, "parents") {
        if int(&p, "parent") != Some(seed) || !["open", "switch"].contains(&p["reason"].as_str()) {
            continue;
        }
        let (node, stream, t) = (
            int(&p, "node").unwrap(),
            int(&p, "stream").unwrap(),
            int(&p, "t_ms").unwrap(),
        );
        assert!(
            grants
                .iter()
                .any(|&(n, s, at)| n == node && s == stream && at <= t),
            "an open with no grant before it: {p:?}"
        );
        opens += 1;
    }
    assert!(opens > 0);
    // Declines too: the seed holds a few copies and turns the rest away.
    assert!(
        decisions
            .iter()
            .any(|x| x["kind"] == "request" && x["granted"] == "false")
    );
    // An allocation check a second, each with the window and the holders'
    // readings.
    for kind in ["window", "check", "holder_joined"] {
        assert!(decisions.iter().any(|x| x["kind"] == kind), "{kind}");
    }
}

/// The `meter_samples` table against the run (one swarm's): a row for
/// each of the seed's and the viewers' meter samples from the stream's
/// start, in order, with the host's role and the meter's capacity, empty
/// before it read any; and while the stream is live each host's capacity
/// ÷ its uplink (`avail_bps`) are the readings the row's `meter_seed_p*`
/// and `meter_{pop}_p*` take over their window
/// ([`strims_sim::Report::meter_series`]). Each row holds its sample's
/// other readings (its channel bytes cumulative over the host's
/// incarnations), and the seed's its state as of its last change at or
/// before the sample (`RunnerReport::seed_changes`), which no viewer's
/// row has. Returns the seed's rows, each with its sample's time.
fn meter_samples_say_what_they_are(
    r: &strims_sim::Report,
    d: &Path,
) -> Vec<(Duration, BTreeMap<String, String>)> {
    let swarm = &r.scenario.swarms[0];
    let mut rows = table_rows(d, "meter_samples").into_iter();
    let hosts = std::iter::once(swarm.seed)
        .chain(r.viewers.iter().filter(|v| v.swarm == 0).map(|v| v.host));
    let mut seed = Vec::new();
    for h in hosts {
        let mut readings = Vec::new();
        let mut channel = 0;
        let mut measured = false;
        for m in r.hosts[h]
            .runner
            .meter
            .iter()
            .filter(|m| m.t >= swarm.start)
        {
            let row = rows.next().expect("a row for every sample");
            let at = format!("host {h} at {:?}", m.t);
            assert_eq!(int(&row, "node"), Some(h as i64), "{at}");
            let ms = (m.t - swarm.start).as_millis() as i64;
            assert_eq!(int(&row, "t_ms"), Some(ms), "{at}");
            let role = if h == swarm.seed { "seed" } else { "honest" };
            assert_eq!(row["role"], role, "{at}");
            let capacity = (m.capacity > 0).then_some(m.capacity as i64);
            assert_eq!(int(&row, "capacity_bps"), capacity, "{at}");
            let opt = |v: Option<u32>| v.map(i64::from);
            assert_eq!(
                [
                    int(&row, "rate_bps"),
                    int(&row, "busy_permille"),
                    int(&row, "queue_delay_ms"),
                    int(&row, "path_delay_ms"),
                    int(&row, "path_queued_ms"),
                    int(&row, "path_latest_ms"),
                    int(&row, "children"),
                    int(&row, "requests_in"),
                    int(&row, "rejected"),
                    int(&row, "bytes_written"),
                    int(&row, "channel_bytes"),
                ],
                [
                    Some(m.rate as i64),
                    Some(i64::from(m.busy_permille)),
                    opt(m.queue_delay_ms),
                    opt(m.path_delay_ms),
                    opt(m.path_queued_ms),
                    opt(m.path_latest_ms),
                    m.children.get(&0).map(|&c| c as i64),
                    Some(i64::from(m.requests_in)),
                    Some(i64::from(m.rejected)),
                    Some(m.bytes_written as i64),
                    m.channel_bytes.map(|b| b as i64),
                ],
                "{at}"
            );
            assert!(int(&row, "busy_permille").unwrap() <= 1000, "{at}");
            assert!(int(&row, "rejected") <= int(&row, "requests_in"), "{at}");
            let budget_capacity = m.budget_capacity.get(&0).copied().filter(|&c| c > 0);
            assert_eq!(
                int(&row, "budget_capacity_bps"),
                budget_capacity.map(|c| c as i64),
                "{at}"
            );
            // A floor is read only where a budget was measured.
            if int(&row, "latest_floor_ms").is_some() {
                assert!(budget_capacity.is_some(), "{at}");
            }
            let state = r.hosts[h]
                .runner
                .seed_changes
                .iter()
                .rev()
                .find(|(t, s, _)| *s == 0 && *t <= m.t)
                .map(|(_, _, x)| *x);
            assert_eq!(
                [
                    int(&row, "budget"),
                    int(&row, "window"),
                    int(&row, "copies"),
                    int(&row, "first_wave"),
                    int(&row, "first_wave_capacity_bps"),
                ],
                state.map_or([None; 5], |x| [
                    (x.budget > 0).then_some(x.budget as i64),
                    Some(x.window as i64),
                    Some(x.copies as i64),
                    Some(x.first_wave as i64),
                    Some(x.first_wave_capacity as i64),
                ]),
                "{at}"
            );
            if h != swarm.seed {
                assert_eq!(
                    int(&row, "copies"),
                    None,
                    "{at}: a viewer has no seed state"
                );
            }
            // A budget is empty until measured; once measured it holds a
            // slot a stream at least, and stays measured while its seed
            // runs, which here runs one incarnation.
            if let Some(b) = int(&row, "budget") {
                assert!(b >= 1, "{at}: budget {b}");
                measured = true;
            } else {
                assert!(!measured, "{at}: a budget unmeasured after it was measured");
            }
            // The channel bytes are cumulative over the host's incarnations,
            // and a sim records them at every sample.
            let c = int(&row, "channel_bytes").expect("a sim's channel bytes");
            assert!(c >= channel, "{at}: {c} < {channel}");
            channel = c;
            if m.t <= swarm.stop
                && let (Some(c), Some(a)) = (int(&row, "capacity_bps"), int(&row, "avail_bps"))
                && a > 0
            {
                readings.push((m.t, c as f64 / a as f64));
            }
            if h == swarm.seed {
                seed.push((m.t, row));
            }
        }
        let want: Vec<_> = r
            .meter_series(h)
            .into_iter()
            .map(|(t, c, _)| (t, c))
            .collect();
        assert_eq!(readings, want, "host {h}");
        if h == swarm.seed {
            assert!(!want.is_empty(), "the seed's meter read");
        }
    }
    assert!(rows.next().is_none(), "a row for no sample");
    seed
}

/// The `peer_samples` table against the run: a row for each peer of each
/// of the seed's and the viewers' transfer samples from the stream's start
/// (`HostReport::transfers`), in order, holding that channel's counters;
/// a peer is never its own node, and what it reads back never exceeds what
/// it asked for in flight.
fn peer_samples_say_what_they_are(r: &strims_sim::Report, d: &Path) {
    let swarm = &r.scenario.swarms[0];
    let mut rows = table_rows(d, "peer_samples").into_iter();
    let hosts = std::iter::once(swarm.seed)
        .chain(r.viewers.iter().filter(|v| v.swarm == 0).map(|v| v.host));
    let mut seen = 0;
    for h in hosts {
        for (t, _, x) in r.hosts[h]
            .transfers
            .iter()
            .filter(|(t, s, _)| *s == 0 && *t >= swarm.start)
        {
            for p in &x.peers {
                let row = rows.next().expect("a row for every peer of every sample");
                let at = format!("host {h} at {t:?}");
                assert_eq!(int(&row, "node"), Some(h as i64), "{at}");
                let ms = (*t - swarm.start).as_millis() as i64;
                assert_eq!(int(&row, "t_ms"), Some(ms), "{at}");
                assert_eq!(int(&row, "peer"), p.peer.map(|p| p as i64), "{at}");
                assert_ne!(int(&row, "peer"), Some(h as i64), "{at}: its own peer");
                assert_eq!(
                    [
                        int(&row, "sent_bytes"),
                        int(&row, "received_bytes"),
                        int(&row, "requests_in"),
                        int(&row, "requests_out"),
                        int(&row, "queued_push"),
                        int(&row, "queued_answer"),
                        int(&row, "cwnd_bytes"),
                        int(&row, "flight_bytes"),
                        int(&row, "rtt_ms"),
                        int(&row, "cto_ms"),
                        int(&row, "pending"),
                    ],
                    [
                        p.sent as i64,
                        p.received as i64,
                        p.requests_in as i64,
                        p.requests_out as i64,
                        p.queued[0] as i64,
                        p.queued[1] as i64,
                        p.cwnd,
                        p.flight,
                        p.rtt_ms as i64,
                        p.cto_ms as i64,
                        p.pending as i64,
                    ]
                    .map(Some),
                    "{at}"
                );
                seen += 1;
            }
        }
    }
    assert!(rows.next().is_none(), "a row for no peer");
    assert!(seen > 0, "the nodes' channels were sampled");
}

/// The uplink queues in `bins` and `node_bins`: the seed's in each bin is
/// its node bin's, a reading has its wait and its bytes, a queue of no
/// bytes has no wait (the sim's bytes are its wait at the uplink's rate),
/// and in WAN mode the seed's queue is read.
fn queues_say_what_they_are(r: &strims_sim::Report, d: &Path) {
    let seed = r.scenario.swarms[0].seed as i64;
    let mut by_bin = BTreeMap::new();
    for row in table_rows(d, "node_bins") {
        let (ms, bytes) = (int(&row, "up_queue_ms"), int(&row, "up_queue_bytes"));
        assert_eq!(ms.is_some(), bytes.is_some(), "{row:?}");
        if let (Some(ms), Some(bytes)) = (ms, bytes) {
            assert!(ms >= 0 && bytes >= 0, "{row:?}");
            if bytes == 0 {
                assert_eq!(ms, 0, "{row:?}: a wait with no bytes");
            }
        }
        if int(&row, "node") == Some(seed) {
            by_bin.insert(int(&row, "bin"), (ms, bytes));
        }
    }
    let mut read = 0;
    for row in table_rows(d, "bins") {
        let got = (
            int(&row, "seed_up_queue_ms"),
            int(&row, "seed_up_queue_bytes"),
        );
        if let Some(&want) = by_bin.get(&int(&row, "bin")) {
            assert_eq!(got, want, "bin {:?}", int(&row, "bin"));
        }
        read += usize::from(got.0.is_some());
    }
    assert!(r.wan.is_some() && read > 0, "the seed's queue was read");
}

/// The `hosts` and `packets` tables against the run (`Scenario::packet_log`
/// every host's): every host its row, its role its swarm's; every packet
/// carried leaving no earlier than it entered and reaching its receiver
/// no earlier than it left, every one the queue dropped doing neither, and
/// those no more than the hosts' queues dropped (`queue_drop_packets`,
/// their downlinks' too, and their competing uploads'); acknowledgements
/// alone, TCP's pure ones or a data channel's SACKs, as the links carry
/// them; and the seed's offers (`node_bins`' `offered_bytes`, bin by bin)
/// the IP bytes of its data packets to the swarm's hosts that entered in
/// the bins' span.
fn packets_say_what_they_are(r: &strims_sim::Report, d: &Path, name: &str) {
    let swarm = &r.scenario.swarms[0];
    let hosts = table_rows(d, "hosts");
    assert_eq!(hosts.len(), r.scenario.hosts.len(), "{name}");
    for row in &hosts {
        let h = int(row, "node").unwrap() as usize;
        assert_eq!(row["name"], r.scenario.hosts[h].name, "{name}");
        let role = if h == swarm.seed {
            "seed"
        } else if swarm.viewers.contains(&h) {
            "viewer"
        } else {
            "relay"
        };
        assert_eq!(row["role"], role, "{name}: host {h}");
    }
    let rows = table_rows(d, "packets");
    assert!(!rows.is_empty(), "{name}");
    let wan = r.wan.as_ref().unwrap();
    let mut next: BTreeMap<i64, i64> = BTreeMap::new();
    let mut dropped: BTreeMap<i64, u64> = BTreeMap::new();
    for row in &rows {
        let node = int(row, "node").unwrap();
        let n = next.entry(node).or_default();
        assert_eq!(int(row, "n"), Some(*n), "{name}: {row:?}");
        *n += 1;
        let (t, sent, arrived) = (
            int(row, "t_us").unwrap(),
            int(row, "sent_us"),
            int(row, "arrived_us"),
        );
        match row["lost"].as_str() {
            "queue" => {
                assert!(sent.is_none() && arrived.is_none(), "{name}: {row:?}");
                *dropped.entry(node).or_default() += 1;
            }
            _ => {
                assert!(sent.unwrap() >= t, "{name}: {row:?}");
                assert!(
                    arrived.is_none_or(|a| a >= sent.unwrap()),
                    "{name}: {row:?}"
                );
            }
        }
        assert!(int(row, "charged_bytes").unwrap() >= int(row, "ip_bytes").unwrap());
    }
    for (&node, &n) in &dropped {
        let w = &wan.hosts[node as usize];
        assert!(
            n <= w.queue_drop_packets + w.competing_drop_packets,
            "{name}: host {node}"
        );
    }
    let kinds: BTreeSet<&str> = rows.iter().map(|r| r["kind"].as_str()).collect();
    assert!(
        kinds.contains("data") && kinds.contains("ack"),
        "{name}: {kinds:?}"
    );
    if name != "tcp" {
        assert!(kinds.contains("sack"), "{name}: {kinds:?}");
    }
    let seed = swarm.seed as i64;
    let bins: Vec<BTreeMap<String, String>> = table_rows(d, "node_bins")
        .into_iter()
        .filter(|b| int(b, "node") == Some(seed))
        .collect();
    let (lo, hi) = (
        bins.iter().filter_map(|b| int(b, "bin")).min().unwrap(),
        bins.iter().filter_map(|b| int(b, "bin")).max().unwrap(),
    );
    let width = (int(&bins[1], "t_ms").unwrap() - int(&bins[0], "t_ms").unwrap()) * 1000;
    assert_eq!(
        bins.len() as i64,
        hi - lo + 1,
        "{name}: the seed's bins are whole"
    );
    let offered: i64 = bins.iter().filter_map(|b| int(b, "offered_bytes")).sum();
    let data: i64 = rows
        .iter()
        .filter(|p| {
            int(p, "node") == Some(seed) && matches!(p["kind"].as_str(), "data" | "data_sack")
        })
        .filter(|p| {
            int(p, "peer").is_some_and(|q| {
                let q = q as usize;
                q == swarm.seed || swarm.viewers.contains(&q)
            })
        })
        .filter(|p| (lo * width..(hi + 1) * width).contains(&int(p, "t_us").unwrap()))
        .map(|p| int(p, "ip_bytes").unwrap())
        .sum();
    assert!(offered > 0, "{name}");
    assert_eq!(offered, data, "{name}: the seed's offers");
}

/// A table's rows, each by column name.
fn table_rows(d: &Path, t: &str) -> Vec<BTreeMap<String, String>> {
    let csv = strims_sim::tables::read_table(d, t).unwrap();
    let mut lines = csv.lines();
    let head: Vec<&str> = lines.next().unwrap().split(',').collect();
    lines
        .map(|l| {
            head.iter()
                .zip(l.split(','))
                .map(|(&c, v)| (c.to_owned(), v.to_owned()))
                .collect()
        })
        .collect()
}

/// A row's integer, `None` for an empty field.
fn int(r: &BTreeMap<String, String>, c: &str) -> Option<i64> {
    r[c].parse().ok()
}

/// The timer log's table (`Scenario::timer_log`, every host's) against the
/// run: each host's node task fired timers and the seed's writer waited;
/// a timer fired at its deadline plus the lateness the run draws (none
/// without `Scenario::timer_lateness`, else whole milliseconds within
/// `strims_sim::late`'s draws), after it was armed; its delay is its
/// deadline less the whole ms it was armed at; and a node's timers of a
/// source fire in order, its node task's a millisecond apart at least.
fn timers_say_what_they_are(r: &strims_sim::Report, d: &Path) {
    let rows = table_rows(d, "timers");
    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(d.join("series_meta.json")).unwrap())
            .unwrap();
    assert_eq!(meta["timers_lost"], 0);
    let seed = r.scenario.swarms[0].seed;
    let mut last: BTreeMap<(i64, String), i64> = BTreeMap::new();
    let mut sources: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new();
    let mut ns: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    // A node task's deadline can fall within a millisecond, and its timer
    // then fires at the simulation's next tick past it.
    let most = if r.scenario.timer_lateness {
        strims_sim::late::MAX_LATE_MS * 1000
    } else {
        0
    };
    for row in &rows {
        let x = |c: &str| int(row, c).unwrap();
        let (t, node) = (x("t_us"), x("node"));
        ns.entry(node).or_default().push(x("n"));
        assert_eq!(row["context"], "task", "{row:?}");
        assert_eq!(x("late_us"), t - x("deadline_us"), "{row:?}");
        let late = x("late_us");
        if row["source"] == "node_task" {
            assert!((0..most + 1000).contains(&late), "{row:?}");
        } else {
            assert_eq!(x("deadline_us") % 1000, 0, "{row:?}");
            assert!(late % 1000 == 0 && (0..=most).contains(&late), "{row:?}");
        }
        assert!(x("armed_us") <= t, "{row:?}");
        let floor = x("armed_us").div_euclid(1000) * 1000;
        assert_eq!(x("requested_us"), x("deadline_us") - floor, "{row:?}");
        // A node task's next deadline is a millisecond past its last pass
        // at least; a writer behind its schedule writes at once, as a probe
        // behind its periods arms at once.
        let key = (node, row["source"].clone());
        if let Some(&before) = last.get(&key) {
            let gap = if row["source"] == "node_task" {
                1000
            } else {
                0
            };
            assert!(t >= before + gap, "{row:?} after {before}");
        }
        last.insert(key, t);
        sources
            .entry(node)
            .or_default()
            .insert(row["source"].clone());
    }
    // Each node's rows are numbered from 0 in the order its log kept them,
    // so (node, n) is a row's own.
    for (node, n) in &ns {
        let want: Vec<i64> = (0..n.len() as i64).collect();
        assert_eq!(n, &want, "node {node}");
    }
    for h in 0..r.scenario.hosts.len() as i64 {
        let have = sources.get(&h).cloned().unwrap_or_default();
        assert!(have.contains("node_task"), "host {h}: {have:?}");
        assert_eq!(
            have.contains("writer"),
            h == seed as i64,
            "host {h}: {have:?}"
        );
        assert_eq!(
            have.contains("probe"),
            r.scenario.timer_probe,
            "host {h}: {have:?}"
        );
    }
    if !r.scenario.timer_probe {
        return;
    }
    // The probe arms its delays in turn, one a period, through the
    // stream: a host's rows cycle through them from wherever it began.
    let delays = strims_testbed::node::PROBE_DELAYS_MS.map(|d| d as i64 * 1000);
    let swarm = &r.scenario.swarms[0];
    let periods = (swarm.stop - swarm.start).as_millis() as usize
        / strims_testbed::node::PROBE_PERIOD_MS as usize;
    for h in 0..r.scenario.hosts.len() as i64 {
        let asked: Vec<i64> = rows
            .iter()
            .filter(|x| x["source"] == "probe" && int(x, "node") == Some(h))
            .map(|x| int(x, "requested_us").unwrap())
            .collect();
        assert!(
            asked.len() + 2 >= periods,
            "host {h}: {} of {periods}",
            asked.len()
        );
        let first = delays.iter().position(|&d| d == asked[0]).unwrap();
        for (i, d) in asked.iter().enumerate() {
            assert_eq!(
                *d,
                delays[(first + i) % delays.len()],
                "host {h}: probe {i}"
            );
        }
    }
}

/// The link log's tables (`Scenario::link_log`, every host's) against the
/// run: no record lost; a link's writes and reads run on from its first
/// byte in order, each write handed to the writer before the writer began
/// it and taken whole after; a read is taken after it was read; a byte is
/// read after the write that held it was done, its peer's link of the same
/// ordinal holding the same bytes; holds are a TCP socket's in a `tcp` run
/// and a channel's or its host's on its data channels in a `browsers` run,
/// whose writes' checks read the amount they went at; every host reads its
/// links' transports and queues every capture interval.
fn link_logs_say_what_they_are(r: &strims_sim::Report, d: &Path, run: &str) {
    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(d.join("series_meta.json")).unwrap())
            .unwrap();
    assert_eq!(meta["link_log_lost"], 0, "{run}");
    type Key = (i64, i64, i64);
    let writes = table_rows(d, "link_writes");
    let reads = table_rows(d, "link_reads");
    let samples = table_rows(d, "link_samples");
    assert!(!writes.is_empty() && !reads.is_empty(), "{run}");
    let key = |x: &BTreeMap<String, String>| -> Key {
        let i = |c: &str| int(x, c).unwrap();
        (i("node"), i("peer"), i("link"))
    };
    // Each link's writes: their byte ranges and when each was done.
    let mut written: BTreeMap<Key, Vec<(i64, i64, i64)>> = BTreeMap::new();
    let mut waits: BTreeSet<(String, String)> = BTreeSet::new();
    for w in &writes {
        let x = |c: &str| int(w, c).unwrap();
        let k = key(w);
        let rows = written.entry(k).or_default();
        let (off, n) = (x("off"), x("n"));
        assert_eq!(n, rows.len() as i64, "{run}: {w:?}");
        assert_eq!(
            off,
            rows.last().map_or(0, |&(o, b, _)| o + b),
            "{run}: {w:?}"
        );
        assert!(x("bytes") > 0, "{run}: {w:?}");
        assert!(x("queued_us") <= x("t_us"), "{run}: {w:?}");
        assert!(x("t_us") <= x("done_us"), "{run}: {w:?}");
        rows.push((off, x("bytes"), x("done_us")));
        waits.insert((w["kind"].clone(), w["wait"].clone()));
    }
    let mut read: BTreeMap<Key, i64> = BTreeMap::new();
    for x in &reads {
        let i = |c: &str| int(x, c).unwrap();
        let k = key(x);
        let next = read.entry(k).or_default();
        assert_eq!(i("off"), *next, "{run}: {x:?}");
        *next += i("bytes");
        if let Some(taken) = int(x, "taken_us") {
            assert!(taken >= i("t_us"), "{run}: {x:?}");
        }
        // The peer's write that held the read's last byte was done before.
        let (node, peer, link) = k;
        let last = i("off") + i("bytes") - 1;
        let w = written
            .get(&(peer, node, link))
            .and_then(|ws| ws.iter().find(|&&(o, b, _)| o <= last && last < o + b))
            .unwrap_or_else(|| panic!("{run}: no write holds {x:?}"));
        assert!(w.2 <= i("t_us"), "{run}: {x:?} before its write {w:?}");
    }
    // Every host wrote, the seed to each viewer.
    let seed = r.scenario.swarms[0].seed as i64;
    for &v in &r.scenario.swarms[0].viewers {
        assert!(
            written.keys().any(|&(n, p, _)| (n, p) == (seed, v as i64)),
            "{run}: seed to {v}"
        );
    }
    for (kind, wait) in &waits {
        let ok: &[&str] = match kind.as_str() {
            "tcp" => &["none", "notsent"],
            "webrtc" => &["none", "channel", "host"],
            _ => &["none"],
        };
        assert!(ok.contains(&wait.as_str()), "{run}: {kind} {wait}");
    }
    let kinds: BTreeSet<&str> = writes.iter().map(|w| w["kind"].as_str()).collect();
    if run == "browsers" {
        assert!(kinds.contains("webrtc"), "{run}: {kinds:?}");
        assert!(
            writes
                .iter()
                .any(|w| w["kind"] == "webrtc" && !w["unsent_before"].is_empty()),
            "{run}: a channel's writes read their amount"
        );
        // A Chromium page's writer, reading its own count of the amount,
        // waits for its own cap.
        assert!(
            writes.iter().any(|w| {
                w["wait"] == "channel"
                    && r.scenario.hosts[int(w, "node").unwrap() as usize]
                        .rtc_stack
                        .is_some_and(|p| p.amount_at_sends)
            }),
            "{run}: a Chromium writer held for its own cap"
        );
    } else {
        assert_eq!(kinds, BTreeSet::from(["tcp"]), "{run}");
    }
    let mut sampled: BTreeMap<i64, BTreeSet<(String, String)>> = BTreeMap::new();
    for x in &samples {
        sampled
            .entry(int(x, "node").unwrap())
            .or_default()
            .insert((x["source"].clone(), x["cause"].clone()));
    }
    for h in std::iter::once(seed).chain(r.scenario.swarms[0].viewers.iter().map(|&v| v as i64)) {
        let have = sampled.get(&h).cloned().unwrap_or_default();
        for want in ["queues", "model"] {
            assert!(
                have.contains(&(want.to_owned(), "sample".to_owned())),
                "{run}: host {h}: {have:?}"
            );
        }
    }
    // A write sample is a check that held its write: it falls within a
    // write of its link that something held.
    let mut held: BTreeMap<Key, Vec<(i64, i64)>> = BTreeMap::new();
    for w in writes.iter().filter(|w| w["wait"] != "none") {
        let x = |c: &str| int(w, c).unwrap();
        held.entry(key(w))
            .or_default()
            .push((x("t_us"), x("done_us")));
    }
    for x in samples.iter().filter(|x| x["cause"] == "write") {
        let t = int(x, "t_us").unwrap();
        assert!(
            held.get(&key(x))
                .is_some_and(|ws| ws.iter().any(|&(b, d)| b <= t && t <= d)),
            "{run}: {x:?} outside a held write"
        );
        assert!(!x["buffered_bytes"].is_empty(), "{run}: {x:?}");
    }
    // A page that counts its channel's amount itself
    // (`SctpProfile::amount_at_sends`) reads it fall, and fires its
    // `bufferedamountlow`, only on its stack's answer to its last write,
    // within a ms of it, or as its channel empties.
    for x in samples.iter().filter(|x| x["cause"] == "low") {
        let node = int(x, "node").unwrap();
        let shown = r.scenario.hosts[node as usize]
            .rtc_stack
            .is_some_and(|p| p.amount_at_sends);
        if !shown {
            continue;
        }
        let t = int(x, "t_us").unwrap();
        let last = written.get(&key(x)).and_then(|ws| {
            ws.iter()
                .map(|&(_, _, done)| done)
                .filter(|&d| d <= t)
                .max()
        });
        assert!(
            int(x, "buffered_bytes") == Some(0) || last.is_some_and(|d| t - d <= 1000),
            "{run}: {x:?} woke at {t} past its last write {last:?}"
        );
    }
}

/// The request log's tables (`Scenario::request_logs`) against the run:
/// the holds behind HAVEs, answers and reads
/// ([`every_read_and_answer_has_its_hold`]), the requests' knowledge, and
/// expiries against the runners' timeouts.
fn request_logs_say_what_they_are(r: &strims_sim::Report, d: &Path) {
    every_read_and_answer_has_its_hold(r, d);
    // Every request rested on its peer's HAVE, before it went; and the
    // expiries are the runners' timed out requests.
    let asks = table_rows(d, "asks");
    let asked: Vec<_> = asks.iter().filter(|a| a["picked"] == "true").collect();
    assert!(!asked.is_empty());
    for a in &asked {
        assert_eq!(
            (a["round"].as_str(), a["declares"].as_str()),
            ("flight", "true"),
            "{a:?}"
        );
        assert!(
            int(a, "have_ms").unwrap() <= int(a, "t_ms").unwrap(),
            "{a:?}"
        );
        assert!(int(a, "timeout_ms").unwrap() > 0, "{a:?}");
        assert!(a["predicted_ms"].is_empty(), "{a:?}");
    }
    let expired = asked.iter().filter(|a| a["outcome"] == "expired").count() as u64;
    assert_eq!(expired, {
        let (a, b, c) = r.timeouts(0);
        a + b + c
    });
    for r in asks.iter().filter(|a| a["picked"] == "false") {
        assert!(
            r["outcome"].is_empty() && r["timeout_ms"].is_empty(),
            "{r:?}"
        );
        assert!(
            int(r, "predicted_ms") <= int(r, "predicted_horizon_ms")
                && r["predicted_p90_ms"].is_empty(),
            "{r:?}"
        );
    }
    let meta = std::fs::read_to_string(d.join("series_meta.json")).unwrap();
    for lost in ["held_lost", "haves_lost", "asks_lost"] {
        assert!(meta.contains(&format!("\"{lost}\":0")), "{meta}");
    }
}

/// The `stalls` table says how each viewer's playout went at each delay of
/// [`DELAYS_MS`], from its `reads` (each unit at its first read, due at the
/// read less its lag; the units due at once as late as the latest):
/// playback starts at the first due units' time plus the delay, or plus
/// their lag if more; each stall waits on the units due at one time, from
/// their play at the offset so far until they came, a rise of the offset
/// to their lag; the stalls come in time order, and add up to the viewer's
/// most lag past the offset it started at, so a longer delay stalls no
/// longer. The stalls the table holds.
fn stalls_say_how_playout_went(d: &Path) -> usize {
    // Each viewer's units' first reads: seq -> (t_ms, lag_ms).
    let mut units: BTreeMap<i64, BTreeMap<i64, (i64, i64)>> = BTreeMap::new();
    for r in table_rows(d, "reads") {
        let (v, t, s, l) = (
            int(&r, "viewer").unwrap(),
            int(&r, "t_ms").unwrap(),
            int(&r, "seq").unwrap(),
            int(&r, "lag_ms").unwrap(),
        );
        let e = units.entry(v).or_default().entry(s).or_insert((t, l));
        if t < e.0 {
            *e = (t, l);
        }
    }
    let mut stalls: BTreeMap<(i64, i64), Vec<(i64, i64)>> = BTreeMap::new();
    let rows = table_rows(d, "stalls");
    for r in &rows {
        let (v, delay) = (int(r, "viewer").unwrap(), int(r, "delay_ms").unwrap());
        assert!(
            DELAYS_MS.contains(&delay) && units.contains_key(&v),
            "{r:?}"
        );
        let s = (int(r, "t_ms").unwrap(), int(r, "stall_ms").unwrap());
        stalls.entry((v, delay)).or_default().push(s);
    }
    for (&v, us) in &units {
        // The units by due time, each time's the latest lag.
        let mut due: BTreeMap<i64, i64> = BTreeMap::new();
        for &(t, l) in us.values() {
            let e = due.entry(t - l).or_insert(l);
            *e = (*e).max(l);
        }
        let first = *due.values().next().unwrap();
        let most = *due.values().max().unwrap();
        let mut stalled = Vec::new();
        for delay in DELAYS_MS {
            let start = delay.max(first);
            let (mut offset, mut last) = (start, i64::MIN);
            for &(t, ms) in stalls.get(&(v, delay)).into_iter().flatten() {
                assert!(ms > 0 && t > last, "viewer {v} at {delay} ms: {t} {ms}");
                assert_eq!(
                    due.get(&(t - offset)),
                    Some(&(offset + ms)),
                    "viewer {v} at {delay} ms: the stall at {t} waits on units due then"
                );
                (offset, last) = (offset + ms, t);
            }
            assert_eq!(offset, start.max(most), "viewer {v} at {delay} ms");
            stalled.push(offset - start);
        }
        assert!(
            stalled.windows(2).all(|w| w[0] >= w[1]),
            "viewer {v}: {stalled:?}"
        );
    }
    rows.len()
}

/// The `parents` table says how each node's push parents changed: a node's
/// changes in order (`n` from 0, `t_ms` never falling), each stream's an
/// `open` from no parent, `switch`es to another, and an end (`prune`,
/// `shed`, `lost`) back to none; the seed takes none and no node is its own
/// parent; the switches are the runners' (`switched`), and the logs' caps
/// left none out (`parents_lost`).
fn parents_say_how_the_trees_changed(r: &strims_sim::Report, d: &Path) {
    let rows = table_rows(d, "parents");
    assert!(!rows.is_empty());
    let seed = r.scenario.swarms[0].seed as i64;
    let mut order: BTreeMap<i64, (i64, i64)> = BTreeMap::new();
    let mut open: BTreeMap<(i64, i64), i64> = BTreeMap::new();
    let mut switches = 0;
    for p in &rows {
        let (t, node, n, stream) = (
            int(p, "t_ms").unwrap(),
            int(p, "node").unwrap(),
            int(p, "n").unwrap(),
            int(p, "stream").unwrap(),
        );
        let parent = int(p, "parent").filter(|&x| x >= 0);
        let (next, last) = order.entry(node).or_insert((0, i64::MIN));
        assert_eq!((n, t >= *last), (*next, true), "{p:?}");
        (*next, *last) = (n + 1, t);
        assert!(node != seed && parent != Some(node), "{p:?}");
        let had = open.get(&(node, stream)).copied();
        let ok = match p["reason"].as_str() {
            "open" => had.is_none() && parent.is_some(),
            "switch" => had.is_some() && parent.is_some() && parent != had,
            "prune" | "shed" | "lost" => had.is_some() && parent.is_none(),
            _ => false,
        };
        assert!(ok, "{p:?} after {had:?}");
        match parent {
            Some(x) => open.insert((node, stream), x),
            None => open.remove(&(node, stream)),
        };
        switches += u64::from(p["reason"] == "switch");
    }
    assert_eq!(switches, r.swarm_counters(0).switched);
    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(d.join("series_meta.json")).unwrap())
            .unwrap();
    assert_eq!(meta["parents_lost"], 0, "{meta}");
}

/// The parent checks (`Scenario::parent_checks`) against what they say
/// and the parent log: a viewer's checks of its streams, each with the
/// columns its action has (a parent and votes with a parent, a chosen
/// peer where it asked its best, a best where it found one: always but at
/// none and a reset, whose parent may stand alone), two candidates apart,
/// the best's lag the lowest, and its parent the stream's open parent at
/// some point of its pass, as the parents table has the changes (a pass's
/// share its ms, in order). Returns the checks' count by action.
fn parent_checks_say_what_they_are(r: &strims_sim::Report, d: &Path) -> BTreeMap<String, u64> {
    let rows = table_rows(d, "parent_checks");
    assert!(!rows.is_empty());
    let swarm = &r.scenario.swarms[0];
    let streams = swarm.options.stream_count.max(1) as i64;
    let viewers: BTreeSet<i64> = swarm.viewers.iter().map(|&v| v as i64).collect();
    // Each (node, stream)'s parent from each change on: (t_ms, parent).
    type Changes = Vec<(i64, Option<i64>)>;
    let mut history: BTreeMap<(i64, i64), Changes> = BTreeMap::new();
    for p in table_rows(d, "parents") {
        let key = (int(&p, "node").unwrap(), int(&p, "stream").unwrap());
        let parent = int(&p, "parent").filter(|&x| x >= 0);
        history
            .entry(key)
            .or_default()
            .push((int(&p, "t_ms").unwrap(), parent));
    }
    let mut last = i64::MIN;
    let mut actions: BTreeMap<String, u64> = BTreeMap::new();
    for c in &rows {
        let (t, node, stream) = (
            int(c, "t_us").unwrap(),
            int(c, "node").unwrap(),
            int(c, "stream").unwrap(),
        );
        assert!(t >= last, "{c:?}: out of order");
        last = t;
        assert!(
            viewers.contains(&node) && (0..streams).contains(&stream),
            "{c:?}"
        );
        let (best, runner_up, parent, chosen) = (
            int(c, "best_peer"),
            int(c, "runner_up_peer"),
            int(c, "parent"),
            int(c, "chosen"),
        );
        let (best_lag, runner_up_lag) = (int(c, "best_lag_us"), int(c, "runner_up_lag_us"));
        let votes = int(c, "votes").unwrap();
        let action = c["action"].as_str();
        let asks = matches!(action, "graft" | "switch");
        let parented = matches!(action, "vote" | "switch" | "reset");
        assert!(
            ["graft", "none", "vote", "switch", "reset"].contains(&action),
            "{c:?}"
        );
        assert_eq!(chosen, best.filter(|_| asks), "{c:?}");
        assert_eq!(parent.is_some(), parented, "{c:?}");
        assert_eq!(votes > 0, parented, "{c:?}");
        if !parented {
            assert!(
                c["parent_have_us"].is_empty() && c["parent_push_us"].is_empty(),
                "{c:?}"
            );
        }
        match action {
            "none" => assert!(best_lag.is_none(), "{c:?}"),
            "reset" => {}
            _ => assert!(best_lag.is_some(), "{c:?}"),
        }
        assert!(runner_up_lag.is_none() || best_lag.is_some(), "{c:?}");
        assert!(runner_up.is_none() || runner_up != best, "{c:?}");
        assert!(
            best_lag.zip(runner_up_lag).is_none_or(|(b, u)| b <= u),
            "{c:?}"
        );
        assert!(
            best != Some(node) && runner_up != Some(node) && parent != Some(node),
            "{c:?}"
        );
        assert!(best.is_none() || best != parent, "{c:?}");
        // The stream's parent before the check's ms, and after each change
        // in it.
        let ms = t.div_euclid(1000);
        let changes = history.get(&(node, stream)).map_or(&[][..], Vec::as_slice);
        let before = changes
            .iter()
            .rev()
            .find(|&&(at, _)| at < ms)
            .and_then(|&(_, p)| p);
        let mut could = vec![before];
        could.extend(changes.iter().filter(|&&(at, _)| at == ms).map(|&(_, p)| p));
        assert!(
            could.contains(&parent),
            "{c:?}: the stream's parents then {could:?}"
        );
        *actions.entry(action.to_owned()).or_default() += 1;
    }
    assert!(actions.get("graft").is_some_and(|&n| n > 0), "{actions:?}");
    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(d.join("series_meta.json")).unwrap())
            .unwrap();
    assert_eq!(meta["parent_checks_lost"], 0, "{meta}");
    actions
}

/// The parent checks of a twin whose parents switch: seven viewers in a
/// full mesh behind a 5 Mbit/s seed, the window model on, their checks
/// voting, switching and resetting, as the tables say
/// ([`parent_checks_say_what_they_are`]); the log changes no trace.
#[test]
fn parent_checks_say_what_they_are_where_parents_switch() {
    let build = || {
        let mut c = strims_sim::calibration::Calibration::from_knobs(|k| {
            match k {
                "STRIMS_SIM_CAL_VIEWERS" => Some("7"),
                "STRIMS_SIM_CAL_SEED_UP_MBIT" => Some("5"),
                "STRIMS_SIM_CAL_MESH" => Some("full"),
                "STRIMS_SIM_CAL_SECS" => Some("15"),
                _ => None,
            }
            .map(str::to_owned)
        });
        c.window = true;
        c.chunks = Some((4096, 32 * 1024));
        let mut s = strims_sim::calibration::calibration_star(&c);
        s.transfer_samples = true;
        s
    };
    let plain = run(build(), 1).unwrap();
    let bin = Duration::from_millis(100);
    let mut spec = build();
    spec.wire_bins = Some(bin);
    spec.parent_checks = true;
    let r = run(spec, 1).unwrap();
    assert_eq!(
        (&plain.trace_hash, plain.reads_hash()),
        (&r.trace_hash, r.reads_hash())
    );
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "parent_checks_say_what_they_are_where_parents_switch-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    r.write_tables(&d, bin, true).unwrap();
    let actions = parent_checks_say_what_they_are(&r, &d);
    for a in ["vote", "switch", "reset"] {
        assert!(
            actions.get(a).is_some_and(|&n| n > 0),
            "no {a}: {actions:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Every honest peer's HAVE and answer came once it held the chunk, and
/// every read's chunk (its frame's) its viewer's node held by then, as
/// `arrivals` records the holds: the held log of every node, a crashed or
/// departed one's to its end.
fn every_read_and_answer_has_its_hold(r: &strims_sim::Report, d: &Path) {
    let swarm = &r.scenario.swarms[0];
    let per = (swarm.options.chunk_size / 1024) as i64;
    // Each node's first hold of each chunk, in each incarnation.
    let mut held: BTreeMap<(i64, i64, i64), i64> = BTreeMap::new();
    let mut first: BTreeMap<(i64, i64), i64> = BTreeMap::new();
    for a in table_rows(d, "arrivals") {
        let (n, s, i, t) = (
            int(&a, "node"),
            int(&a, "seq"),
            int(&a, "incarnation"),
            int(&a, "t_ms"),
        );
        let (n, s, i, t) = (n.unwrap(), s.unwrap(), i.unwrap(), t.unwrap());
        let h = held.entry((n, s, i)).or_insert(t);
        *h = (*h).min(t);
        let f = first.entry((n, s)).or_insert(t);
        *f = (*f).min(t);
    }
    let hold = |r: &BTreeMap<String, String>| {
        held.get(&(int(r, "peer")?, int(r, "seq")?, int(r, "peer_incarnation")?))
            .copied()
    };
    // Every honest peer's HAVE came once it held the chunk, in the
    // incarnation the row names.
    let haves = table_rows(d, "haves");
    assert!(!haves.is_empty());
    for h in haves.iter().filter(|h| h["peer_honest"] == "true") {
        let at = hold(h);
        assert!(
            at.is_some_and(|at| at <= int(h, "t_ms").unwrap()),
            "{h:?} held {at:?}"
        );
    }
    // Every honest peer's answer came once it held the chunk.
    let asks = table_rows(d, "asks");
    let answered: Vec<_> = asks
        .iter()
        .filter(|a| a["picked"] == "true" && a["outcome"] == "answered")
        .filter(|a| a["peer_honest"] == "true")
        .collect();
    assert!(!answered.is_empty());
    for a in answered {
        let at = hold(a);
        assert!(
            at.is_some_and(|at| at <= int(a, "t_outcome_ms").unwrap()),
            "{a:?} held {at:?}"
        );
    }
    // Every read's chunk its viewer's node held by then.
    let node: BTreeMap<i64, i64> = table_rows(d, "viewers")
        .iter()
        .map(|v| (int(v, "viewer").unwrap(), int(v, "host").unwrap()))
        .collect();
    let reads = table_rows(d, "reads");
    assert!(!reads.is_empty());
    for r in reads {
        let (v, s, t) = (
            int(&r, "viewer").unwrap(),
            int(&r, "seq").unwrap(),
            int(&r, "t_ms").unwrap(),
        );
        let at = first.get(&(node[&v], s / per));
        assert!(at.is_some_and(|&at| at <= t), "{r:?} held {at:?}");
    }
}

/// Every viewer stored each chunk it read before it read a frame of it,
/// the seed each chunk at its frames' write (when it was due, or at most
/// `late::MAX_LATE_MS` after with `Scenario::timer_lateness`), and the forecast's
/// `held_ms` is the sampled peer's own first store, empty (never -1)
/// without one.
fn held_truth_meets_the_reads(r: &strims_sim::Report, d: &Path) {
    let swarm = &r.scenario.swarms[0];
    let per = (swarm.options.chunk_size / 1024) as u64;
    let first = |h: usize| -> BTreeMap<u64, Duration> {
        let mut m = BTreeMap::new();
        for &(seq, at) in &r.hosts[h].held {
            m.entry(seq).or_insert(at);
        }
        m
    };
    for v in &r.viewers {
        let held = first(v.host);
        for &(seq, lag) in &v.stats.lags {
            let read = r.written_at(v.swarm, seq) + Duration::from_millis(u64::from(lag) + 1);
            let at = held.get(&(seq / per));
            assert!(
                at.is_some_and(|&t| t <= read),
                "{}: frame {seq} read at {read:?}, held {at:?}",
                v.host
            );
        }
    }
    let seed = first(swarm.seed);
    assert!(!seed.is_empty());
    for (&c, &at) in &seed {
        let written = r.written_at(0, c * per + per - 1);
        if r.scenario.timer_lateness {
            let most = Duration::from_millis(strims_sim::late::MAX_LATE_MS.unsigned_abs());
            assert!(
                (written..=written + most).contains(&at),
                "the seed's chunk {c}: due {written:?}, stored {at:?}"
            );
        } else {
            assert_eq!(at, written, "the seed's chunk {c}");
        }
    }
    let csv = strims_sim::tables::read_table(d, "forecast").unwrap();
    let mut rows = csv.lines();
    let head: Vec<&str> = rows.next().unwrap().split(',').collect();
    let col = |n: &str| head.iter().position(|&c| c == n).unwrap();
    let (peer, seq, held) = (col("peer"), col("seq"), col("held_ms"));
    let start = swarm.start;
    let mut truths = 0;
    for l in rows {
        let f: Vec<&str> = l.split(',').collect();
        assert_ne!(f[held], "-1");
        let (Ok(p), Ok(c)) = (f[peer].parse::<usize>(), f[seq].parse::<u64>()) else {
            continue;
        };
        if let Ok(h) = f[held].parse::<i64>() {
            let at = first(p)[&c];
            assert_eq!(h, at.as_millis() as i64 - start.as_millis() as i64, "{l}");
            truths += 1;
        }
    }
    assert!(truths > 0);
}
