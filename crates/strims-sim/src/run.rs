// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Runs one scenario with one seed: builds the turmoil simulation, steps
//! it while applying the timed faults, and assembles the [`Report`].

use std::rc::Rc;
use std::time::{Duration, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use turmoil::Sim;

use crate::memory::MemoryGroup;
use crate::report::{MemoryReport, Report, ViewerReport, WanReport};
use crate::spec::{Departure, Fault, HostIx, Scenario};
use crate::wan::{Scheduler, sim_time};
use crate::world::World;
use crate::{EPOCH_MS, control, host};

/// The turmoil host name of the controller.
const CONTROLLER: &str = "ctl";

/// Wall-clock seconds since `start`, for the report's speed figure only;
/// nothing in a run depends on it.
#[allow(clippy::disallowed_methods, clippy::disallowed_types)]
fn wall_clock() -> impl FnOnce() -> f64 {
    let start = std::time::Instant::now();
    move || start.elapsed().as_secs_f64()
}

/// Runs `spec` with `seed`. `Err` carries turmoil's error (a host task
/// failed, or the run didn't finish).
pub fn run(spec: Scenario, seed: u64) -> Result<Report, String> {
    let mut group = MemoryGroup::new();
    let counters = group.as_ref().map(MemoryGroup::counters);
    let records = MemoryGroup::new();
    let records_counters = records.as_ref().map(MemoryGroup::counters);
    let w = Rc::new(World::new(spec, seed, counters, records));
    let guard = group.as_mut().map(MemoryGroup::enter);
    let elapsed = wall_clock();
    let sim_secs = simulate(&w);
    let wall_secs = elapsed();
    drop(guard);
    let (sim_secs, wan) = sim_secs?;
    let memory = counters.filter(|c| c.peak() > 0).map(|c| MemoryReport {
        peak: c.peak(),
        records_peak: records_counters.map_or(0, |r| r.peak()),
        samples: w.st.borrow().memory_samples.clone(),
    });
    Ok(report(&w, sim_secs, wall_secs, memory, wan))
}

fn simulate(w: &Rc<World>) -> Result<(f64, Option<WanReport>), String> {
    let spec = &w.spec;
    // str0m draws ICE credentials and SCTP details from a thread-local rng.
    strims_node::seed_str0m_thread_rng(w.seed);
    // No `fail_rate`: loss comes from `Fault`s alone, since a TCP stream
    // that loses a segment stalls until its read timeout.
    let mut builder = turmoil::Builder::new();
    builder
        .rng_seed(w.seed)
        .epoch(UNIX_EPOCH + Duration::from_millis(EPOCH_MS.unsigned_abs()))
        .simulation_duration(spec.duration + Duration::from_secs(30))
        .tick_duration(spec.tick)
        .min_message_latency(spec.latency.0)
        .max_message_latency(spec.latency.1);
    // turmoil holds 64 unread segments a connection: keep that many bytes
    // with segments split from 16 KiB writes.
    if let Some(n) = spec.segment() {
        builder.tcp_capacity(64 * (16 * 1024usize).div_ceil(n.max(1)));
    }
    let mut sim = builder.build();
    for (h, hs) in spec.hosts.iter().enumerate() {
        let wc = w.clone();
        sim.host(hs.name.as_str(), move || host::host_main(wc.clone(), h));
    }
    sim.client(CONTROLLER, control::controller(w.clone()));
    let wan = match spec.wan {
        Some(options) => Some(Scheduler::new(
            &sim,
            spec,
            options,
            w.seed,
            w.send_buffers.clone(),
        )?),
        None => None,
    };
    let has_wan = wan.is_some();
    *w.wan.lock() = wan;
    if has_wan {
        for hs in &spec.hosts {
            if let Some(site) = hs.site {
                w.log_at(0, "wan", format!("{} {site}", hs.name));
            }
        }
        for hs in spec.hosts.iter().filter(|hs| !hs.up_steps.is_empty()) {
            let steps: Vec<String> = hs
                .up_steps
                .iter()
                .map(|(at, rate)| format!("{rate} B/s from {} ms", at.as_millis()))
                .collect();
            w.log_at(
                0,
                "capacity",
                format!("{} up {}", hs.name, steps.join(", ")),
            );
        }
    }
    if !spec.clock_skew.is_off() {
        for (h, hs) in spec.hosts.iter().enumerate() {
            let ms = w.clock_offset(h).as_millis();
            w.log_at(0, "clock", format!("{} {ms} ms", hs.name));
        }
    }

    let mut faults = spec.faults.clone();
    faults.sort_by_key(|f| f.at);
    let mut next = 0;
    // The bins' width and the next bin edge ([`Scenario::wire_bins`]).
    let mut bins = spec
        .wire_bins
        .filter(|_| has_wan)
        .zip(spec.swarms.first())
        .map(|(width, s)| (width.max(Duration::from_millis(1)), s.start));
    loop {
        while let Some(f) = faults.get(next)
            && sim.elapsed() >= f.at
        {
            apply(&mut sim, w, w.wan.lock().as_mut(), &f.fault);
            next += 1;
        }
        // The counters as they stood at each edge passed: before this
        // walk's sends, and without adding to the schedule.
        if let Some((width, next_edge)) = bins.as_mut()
            && sim.elapsed() >= *next_edge
            && let Some(s) = w.wan.lock().as_ref()
        {
            let c: Vec<_> = (0..spec.hosts.len()).map(|h| s.up_counters(h)).collect();
            while sim.elapsed() >= *next_edge {
                w.record_wire_bin(c.clone());
                *next_edge += *width;
            }
        }
        if let Some(s) = w.wan.lock().as_mut() {
            s.walk(&sim)?;
        }
        match sim.step() {
            Ok(true) => break,
            Ok(false) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    let secs = sim.elapsed().as_secs_f64();
    let end = sim.elapsed();
    let ms = u64::try_from(end.as_millis()).unwrap_or(u64::MAX);
    // Dropping the simulation drops every runtime, and with them the nodes.
    drop(sim);
    w.observe_stopped(end, true);
    let wan = w.wan.lock().take().map(|s| s.report(spec));
    for h in wan.iter().flat_map(|r| &r.hosts) {
        w.log_at(
            ms,
            "wan",
            format!(
                "{} up {} down {} uplink queue peak {} ms downlink queue peak {} ms retransmissions {} datagrams lost {}",
                h.name,
                h.stats.up_bytes,
                h.stats.down_bytes,
                h.stats.up_queue_max.as_millis(),
                h.stats.down_queue_max.as_millis(),
                h.stats.retransmits,
                h.stats.drops,
            ),
        );
    }
    Ok((secs, wan))
}

fn names(w: &World, hs: &[HostIx]) -> String {
    let v: Vec<&str> = hs.iter().map(|&h| w.name(h)).collect();
    v.join(",")
}

fn for_pairs(w: &World, a: &[HostIx], b: &[HostIx], mut f: impl FnMut(&str, &str)) {
    for &x in a {
        for &y in b {
            f(w.name(x), w.name(y));
        }
    }
}

fn for_ix_pairs(a: &[HostIx], b: &[HostIx], mut f: impl FnMut(HostIx, HostIx)) {
    for &x in a {
        for &y in b {
            f(x, y);
        }
    }
}

/// Every other host of the scenario.
fn others(w: &World, h: HostIx) -> Vec<HostIx> {
    (0..w.spec.hosts.len()).filter(|&x| x != h).collect()
}

/// Crashes host `h` (`Sim::crash`), cut off from every other host first:
/// the sockets the crash drops then close in tokio's task order, which
/// depends on process-wide task ids and isn't reproducible. Their FINs are
/// dropped, as a power loss would; peers notice by timeouts, or by an RST
/// once the address answers again. The node's counters and logs since its
/// last sample are kept ([`World::observe_stopped`]).
fn cut_off(sim: &mut Sim<'_>, w: &World, wan: Option<&mut Scheduler>, h: HostIx) {
    for x in others(w, h) {
        sim.partition(w.name(h), w.name(x));
    }
    if let Some(s) = wan {
        for x in others(w, h) {
            s.partition(h, x);
        }
        s.crashed(h, sim_time(sim.elapsed()));
    }
    sim.crash(w.name(h));
    w.observe_stopped(sim.elapsed(), false);
}

/// Applies a fault. In WAN mode the scheduler holds every link between
/// hosts, so turmoil's own hold, release and latency never apply to them:
/// the scheduler freezes pairs, takes new latencies, and is told of every
/// partition and repair (a repaired link is held anew).
fn apply(sim: &mut Sim<'_>, w: &World, wan: Option<&mut Scheduler>, f: &Fault) {
    let ms = u64::try_from(sim.elapsed().as_millis()).unwrap_or(u64::MAX);
    let line = match f {
        Fault::Partition(a, b) => {
            for_pairs(w, a, b, |x, y| sim.partition(x, y));
            if let Some(s) = wan {
                for_ix_pairs(a, b, |x, y| s.partition(x, y));
            }
            format!("partition {} | {}", names(w, a), names(w, b))
        }
        Fault::Repair(a, b) => {
            for_pairs(w, a, b, |x, y| sim.repair(x, y));
            if let Some(s) = wan {
                for_pairs(w, a, b, |x, y| sim.hold(x, y));
                for_ix_pairs(a, b, |x, y| s.repair(x, y));
            }
            format!("repair {} | {}", names(w, a), names(w, b))
        }
        Fault::Hold(a, b) => {
            match wan {
                Some(s) => for_ix_pairs(a, b, |x, y| s.freeze(x, y)),
                None => for_pairs(w, a, b, |x, y| sim.hold(x, y)),
            }
            format!("hold {} | {}", names(w, a), names(w, b))
        }
        Fault::Release(a, b) => {
            match wan {
                Some(s) => for_ix_pairs(a, b, |x, y| s.thaw(x, y)),
                None => for_pairs(w, a, b, |x, y| sim.release(x, y)),
            }
            format!("release {} | {}", names(w, a), names(w, b))
        }
        Fault::Latency(a, b, d) => {
            match wan {
                Some(s) => for_ix_pairs(a, b, |x, y| s.set_latency(x, y, (*d).into())),
                None => for_pairs(w, a, b, |x, y| sim.set_link_latency(x, y, *d)),
            }
            format!(
                "latency {} | {} {} ms",
                names(w, a),
                names(w, b),
                d.as_millis()
            )
        }
        Fault::Crash(h) if w.skips_crash(*h, sim.elapsed()) => {
            format!("crash {} skipped: not there", w.name(*h))
        }
        Fault::Crash(h) => {
            cut_off(sim, w, wan, *h);
            w.crashed(*h, ms);
            format!("crash {}", w.name(*h))
        }
        Fault::Restart(h) if w.skips_restart(*h) => {
            format!("restart {} skipped: not there", w.name(*h))
        }
        Fault::Restart(h) => {
            for x in others(w, *h) {
                sim.repair(w.name(*h), w.name(x));
            }
            if let Some(s) = wan {
                for x in others(w, *h) {
                    sim.hold(w.name(*h), w.name(x));
                    s.repair(*h, x);
                }
            }
            sim.bounce(w.name(*h));
            w.restarted(*h);
            format!("restart {}", w.name(*h))
        }
        // The host's own task leaves its swarms and closes its links
        // (`host::depart`); from here on no host dials it.
        Fault::Depart(h, Departure::Graceful) => {
            w.depart(*h, ms, false);
            format!("depart {} graceful", w.name(*h))
        }
        Fault::Depart(h, Departure::Abrupt) => {
            let up = !w.departed(*h) && !w.is_down(*h);
            if up {
                cut_off(sim, w, wan, *h);
            }
            w.depart(*h, ms, up);
            format!("depart {} abrupt", w.name(*h))
        }
        Fault::PauseReading(h) => {
            w.set_reading_paused(*h, true, ms);
            format!("pause reading {}", w.name(*h))
        }
        Fault::ResumeReading(h) => {
            w.set_reading_paused(*h, false, ms);
            format!("resume reading {}", w.name(*h))
        }
        Fault::Uplink(h, rate) => {
            w.set_uplink(*h, *rate);
            format!("uplink {} {rate} B/s", w.name(*h))
        }
        Fault::Connect { from, to, network } => {
            w.request_connect(*from, *to, *network);
            format!("connect {} -> {}", w.name(*from), w.name(*to))
        }
        Fault::ResetWriter(s) => {
            w.request_writer_reset(*s);
            format!("reset writer of swarm {s}")
        }
    };
    w.log_at(ms, "fault", line);
}

fn report(
    w: &World,
    sim_secs: f64,
    wall_secs: f64,
    memory: Option<MemoryReport>,
    wan: Option<WanReport>,
) -> Report {
    let mut st = w.st.borrow_mut();
    let mut vpn = st.vpn.clone();
    if w.spec.probes.is_some() {
        vpn.unicast_sent = st.probes.len() as u64;
        let distances: Vec<Vec<Option<usize>>> = (0..w.spec.hosts.len())
            .map(|h| w.spec.hop_distances(h))
            .collect();
        for p in st.probes.values() {
            let in_reach = distances
                .get(p.src)
                .and_then(|d| d.get(p.dst).copied().flatten())
                .is_some_and(|d| d <= strims_vpn::message::MAX_MESSAGE_HOPS);
            vpn.in_reach_sent += u64::from(in_reach);
            if let Some(h) = p.delivered {
                vpn.unicast_delivered += 1;
                vpn.in_reach_delivered += u64::from(in_reach);
                *vpn.hops.entry(h).or_default() += 1;
            }
        }
    }
    let hosts = st
        .hosts
        .iter()
        .enumerate()
        .map(|(h, s)| {
            let mut r = s.report();
            r.name = w.name(h).to_owned();
            r.clock_offset = w.clock_offset(h);
            r
        })
        .collect();
    let viewers = std::mem::take(&mut st.viewers)
        .into_iter()
        .map(|((swarm, host), stats)| ViewerReport {
            host,
            name: w.name(host).to_owned(),
            swarm,
            stats,
        })
        .collect();
    let trace = std::mem::take(&mut st.trace);
    let written = std::mem::take(&mut st.written);
    let media = std::mem::take(&mut st.media);
    let violations = std::mem::take(&mut st.violations);
    let wire_bins = std::mem::take(&mut st.wire_bins);
    let parent_losses = std::mem::take(&mut st.parent_losses);
    drop(st);
    let parent_changes = w.parent_changes();
    if w.spec.probes.is_some() {
        vpn.unicast_by_phase = control::unicast_phases(w);
        vpn.lookups_by_phase = control::lookup_phases(w);
    }
    let mut hash = Sha256::new();
    for l in &trace {
        hash.update(l.as_bytes());
        hash.update(b"\n");
    }
    let mut r = Report {
        scenario: w.spec.clone(),
        seed: w.seed,
        trace_hash: hex::encode(hash.finalize()),
        trace,
        sim_secs,
        wall_secs,
        viewers,
        hosts,
        written,
        media,
        vpn: w.spec.probes.is_some().then_some(vpn),
        memory,
        violations,
        wan,
        wire_bins,
        parent_losses,
        parent_changes,
    };
    r.scenario.faults.sort_by_key(|f| f.at);
    r
}
