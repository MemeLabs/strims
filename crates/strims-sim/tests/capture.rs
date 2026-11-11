// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A run with ppspp capture on: the trace a run without it gives, and a
//! dump per host that reads back. And a run's tables
//! ([`strims_sim::tables`]): the same trace at any bin width, over TCP and
//! as browsers, seconds that are the series', bins that add up to the
//! row's totals, and events that are the scenario's and the report's.

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
    Fault, Scenario, SctpProfile, SendBuffer, SwarmSpec, Transport, WanHostReport, run,
};
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
/// relay over TCP.
fn wan_browsers() -> Scenario {
    let mut s = wan_mesh();
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
            held_truth_meets_the_reads(&r, &d);
            request_logs_say_what_they_are(&r, &d);
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
/// names the seed.
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
        let r = run(spec, 1).unwrap();
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

/// The seed's `budget_capacity` is the reading its budget was measured
/// from: at every meter sample once measured, the budget is what 70 % of
/// it pays for at a subscriber's cost, the cost within a factor of two of
/// the stream's (the budget takes the store's rate over its last second,
/// which a bursty writer swings); and the series' and the bins' columns
/// carry it. ppspp's unit tests check the reading exactly.
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
    assert!(seconds.iter().all(|c| *c == -1 || readings.contains(c)));
    assert!(column("bins").iter().any(|&c| c > 0));
}

/// The request log's tables (`Scenario::request_logs`) against the run: HAVEs, requests and reads against
/// the holds `arrivals` records (a read's frame in its chunk), expiries against the runners' timeouts.
fn request_logs_say_what_they_are(r: &strims_sim::Report, d: &Path) {
    let swarm = &r.scenario.swarms[0];
    let per = (swarm.options.chunk_size / 1024) as i64;
    let rows = |t: &str| -> Vec<BTreeMap<String, String>> {
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
    };
    let int = |r: &BTreeMap<String, String>, c: &str| -> Option<i64> { r[c].parse().ok() };
    // Each node's first hold of each chunk, in each incarnation.
    let mut held: BTreeMap<(i64, i64, i64), i64> = BTreeMap::new();
    let mut first: BTreeMap<(i64, i64), i64> = BTreeMap::new();
    for a in rows("arrivals") {
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
    let haves = rows("haves");
    assert!(!haves.is_empty());
    for h in haves.iter().filter(|h| h["peer_honest"] == "true") {
        let at = hold(h);
        assert!(
            at.is_some_and(|at| at <= int(h, "t_ms").unwrap()),
            "{h:?} held {at:?}"
        );
    }
    // Every request rested on its peer's HAVE, before it went; the
    // honest peers' answers came once they held the chunk; and the
    // expiries are the runners' timed out requests.
    let asks = rows("asks");
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
        if a["outcome"] == "answered" && a["peer_honest"] == "true" {
            let at = hold(a);
            assert!(
                at.is_some_and(|at| at <= int(a, "t_outcome_ms").unwrap()),
                "{a:?} held {at:?}"
            );
        }
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
            int(r, "predicted_ms") <= int(r, "predicted_p90_ms"),
            "{r:?}"
        );
    }
    // Every read's chunk its viewer's node held by then.
    let node: BTreeMap<i64, i64> = rows("viewers")
        .iter()
        .map(|v| (int(v, "viewer").unwrap(), int(v, "host").unwrap()))
        .collect();
    for r in rows("reads") {
        let (v, s, t) = (
            int(&r, "viewer").unwrap(),
            int(&r, "seq").unwrap(),
            int(&r, "t_ms").unwrap(),
        );
        let at = first.get(&(node[&v], s / per));
        assert!(at.is_some_and(|&at| at <= t), "{r:?} held {at:?}");
    }
    let meta = std::fs::read_to_string(d.join("series_meta.json")).unwrap();
    for lost in ["held_lost", "haves_lost", "asks_lost"] {
        assert!(meta.contains(&format!("\"{lost}\":0")), "{meta}");
    }
}

/// Every viewer stored each chunk it read before it read a frame of it,
/// the seed each chunk at its frames' write, and the forecast's `held_ms`
/// is the sampled peer's own first store, empty (never -1) without one.
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
        assert_eq!(at, written, "the seed's chunk {c}");
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
