// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The scenario suite. Every test sweeps its scenario over the seeds of
//! `strims_sim::seeds()`, checks each report, and replays the first seed to
//! check the trace hash. Run with `--release` and `--nocapture` to see the
//! per-seed summaries.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::time::Duration;

use strims_base::rng::{DetRng, det_rng};
use strims_ppspp::{DeliveryMode, PushMode, SeedPolicy};
use strims_sim::topology::random_mesh;
use strims_sim::{
    Binding, Competing, Fault, HostIx, Invariants, NetworkSpec, Report, Scenario, SendBuffer,
    SwarmSpec, Transport, VpnProbes, knob, stream_minutes, sweep, sweep_at, sweep_n,
};
use strims_vpn::Routing;
use strims_wan::sample::{pick, unit};
use strims_wan::{Access, City, GLOBAL_AUDIENCE, RESIDENTIAL, Region, Site};

#[global_allocator]
static ALLOC: strims_sim::memory::Allocator<std::alloc::System> =
    strims_sim::memory::Allocator::system();

const KIB: u64 = 1024;
/// The stream rate of every streaming scenario (bytes per second).
const RATE: u64 = 32 * KIB;
const MIB: i64 = 1 << 20;

fn secs(s: u64) -> Duration {
    Duration::from_secs(s)
}

fn ms(d: Duration) -> u64 {
    d.as_millis() as u64
}

/// A seed and `n` viewers in a seeded random mesh (each host dials up to 3
/// earlier ones, 40 % over WebSocket), streaming `RATE` from `start` to
/// `stop`. Draws that vary the scenario further continue from `rng`.
struct Streaming {
    s: Scenario,
    seed: HostIx,
    viewers: Vec<HostIx>,
    rng: DetRng,
}

fn streaming(
    name: &str,
    seed: u64,
    viewers: usize,
    uplinks: (u64, u64),
    push: PushMode,
    (start, stop): (u64, u64),
) -> Streaming {
    let mut s = Scenario::new(name);
    let mut rng = det_rng(seed);
    let seed_host = s.host("seed", Some(uplinks.0));
    let vs = s.hosts("v", viewers, Some(uplinks.1));
    let all: Vec<HostIx> = std::iter::once(seed_host)
        .chain(vs.iter().copied())
        .collect();
    random_mesh(&mut s, &mut rng, &all, 3, 0.4);
    s.swarm(
        SwarmSpec::live(seed_host, vs.clone(), RATE, secs(start), secs(stop)).with_push_mode(push),
    );
    s.duration = secs(stop + 8);
    Streaming {
        s,
        seed: seed_host,
        viewers: vs,
        rng,
    }
}

/// The stream count of the striped WAN cases.
const STRIPES: usize = 16;

/// An adaptive seed with uplink shares by allocation.
const ADAPTIVE: SeedPolicy = SeedPolicy::ADAPTIVE;

/// `s` with every swarm in `streams` streams and its seed on `policy`.
fn striped(mut s: Scenario, streams: usize, policy: SeedPolicy) -> Scenario {
    for sw in &mut s.swarms {
        sw.options.stream_count = streams;
        sw.options.seed = policy;
    }
    let adaptive = if policy == SeedPolicy::ADMISSION {
        ""
    } else {
        " adaptive"
    };
    s.name = format!("{} x{streams}{adaptive}", s.name);
    s
}

// --- checks ---------------------------------------------------------------

/// Every frame of swarm 0 written in `[from, to)` reached each of `who`.
fn complete(r: &Report, who: &[HostIx], from: Duration, to: Duration) -> Result<(), String> {
    complete_swarm(r, 0, who, from, to)
}

/// Every frame of swarm `sw` written in `[from, to)` reached each of `who`.
fn complete_swarm(
    r: &Report,
    sw: usize,
    who: &[HostIx],
    from: Duration,
    to: Duration,
) -> Result<(), String> {
    let spec = &r.scenario.swarms[sw];
    let range = spec.frames_between(from, to);
    for &h in who {
        let v = r
            .viewer(sw, h)
            .ok_or_else(|| format!("{} never read swarm {sw}", r.scenario.name_of(h)))?;
        let missing = r.missing(v, range.clone());
        if missing > 0 {
            let first: Vec<String> = r.written[sw]
                .iter_in(range.clone())
                .filter(|&s| !v.stats.received.contains(s))
                .take(8)
                .map(|s| {
                    format!(
                        "{s}@{}ms",
                        ms(spec.start) + s / spec.frames_per_write as u64 * ms(spec.write_interval)
                    )
                })
                .collect();
            return Err(format!(
                "{} missed {missing} of swarm {sw}'s frames written {}–{} s, first {}",
                v.name,
                from.as_secs(),
                to.as_secs(),
                first.join(" ")
            ));
        }
    }
    Ok(())
}

/// No viewer saw a damaged or out-of-order frame, and no invariant broke.
fn sound(r: &Report) -> Result<(), String> {
    for v in &r.viewers {
        if v.stats.bad_frames > 0 || v.stats.regressions > 0 {
            return Err(format!(
                "{}: {} bad frames, {} regressions",
                v.name, v.stats.bad_frames, v.stats.regressions
            ));
        }
    }
    if let Some((t, what)) = r.violations.first() {
        return Err(format!("invariant at {t} ms: {what}"));
    }
    Ok(())
}

fn all_viewers(r: &Report) -> Vec<HostIx> {
    r.scenario.swarms[0].viewers.clone()
}

fn whole_stream(r: &Report) -> Result<(), String> {
    let spec = &r.scenario.swarms[0];
    complete(r, &all_viewers(r), spec.start, spec.stop)
}

// --- 1. live stream over a mesh -------------------------------------------

fn live_mesh(seed: u64, push: PushMode) -> Scenario {
    let mut m = streaming(
        &format!("live_mesh {push:?}"),
        seed,
        16,
        (2048 * KIB, 1024 * KIB),
        push,
        (3, 33),
    );
    m.s.invariants.max_heap = Some(96 * MIB);
    m.s.invariants.progress = vec![(secs(8), secs(33))];
    m.s
}

fn check_live_mesh(r: &Report) -> Result<(), String> {
    sound(r)?;
    whole_stream(r)
}

#[test]
fn live_mesh_pull() {
    sweep(
        "live_mesh_pull",
        |s| live_mesh(s, PushMode::Off),
        check_live_mesh,
    );
}

#[test]
fn live_mesh_eager_have() {
    sweep(
        "live_mesh_eager_have",
        |s| live_mesh(s, PushMode::EagerHave),
        check_live_mesh,
    );
}

#[test]
fn live_mesh_hybrid() {
    sweep(
        "live_mesh_hybrid",
        |s| live_mesh(s, PushMode::Hybrid),
        check_live_mesh,
    );
}

// --- 2. a seed capped at 3x the stream rate --------------------------------

const SEED_CAP: u64 = 3 * RATE;

fn capped_seed(seed: u64, push: PushMode) -> Scenario {
    let mut m = streaming(
        &format!("capped_seed {push:?}"),
        seed,
        16,
        (SEED_CAP, 8 * RATE),
        push,
        (3, 43),
    );
    m.s.invariants.max_heap = Some(96 * MIB);
    m.s
}

fn check_capped_seed(r: &Report) -> Result<(), String> {
    sound(r)?;
    whole_stream(r)?;
    let u = r.uploads(0);
    // The cap holds (plus QoS's one-second burst and vpn/handshake bytes).
    let limit = SEED_CAP as f64 * (r.sim_secs + 1.0) * 1.05;
    if u.seed as f64 > limit {
        return Err(format!("seed uploaded {} > {limit:.0}", u.seed));
    }
    // 16 viewers need 16x the stream; the seed can send 3x, so relays
    // carry most of the data. Pull spreads it over many of them; a push
    // tree over its interior nodes, a few viewers each feeding several.
    let share = r.data_share(0);
    let min_uploaders = match r.scenario.swarms[0].options.push_mode.resolve() {
        PushMode::Hybrid => 3,
        _ => 8,
    };
    let uploaders = r.data_uploaders(0, 64 * KIB);
    if share > 0.25 || uploaders < min_uploaders {
        return Err(format!(
            "relays don't carry the load: seed data share {share:.2}, {uploaders} uploaders"
        ));
    }
    Ok(())
}

#[test]
fn capped_seed_pull() {
    sweep(
        "capped_seed_pull",
        |s| capped_seed(s, PushMode::Off),
        check_capped_seed,
    );
}

#[test]
fn capped_seed_eager_have() {
    sweep(
        "capped_seed_eager_have",
        |s| capped_seed(s, PushMode::EagerHave),
        check_capped_seed,
    );
}

#[test]
fn capped_seed_hybrid() {
    sweep(
        "capped_seed_hybrid",
        |s| capped_seed(s, PushMode::Hybrid),
        check_capped_seed,
    );
}

/// Honest peers never send ppspp input the runner rejects, so
/// `ChannelMetrics::rejected` flags hostile peers. Seed 4 of
/// `capped_seed_hybrid` exercises the honest cases closest to it: a
/// parent with nothing stored yet opens at `NONE`, and a STREAM_OPEN can
/// cross the child's STREAM_CANCEL after the 1 s open timeout (a stale
/// open).
#[test]
fn honest_hybrid_peers_send_nothing_rejected() {
    let seed = 4;
    let r = strims_sim::run(capped_seed(seed, PushMode::Hybrid), seed).unwrap();
    let rejected: Vec<String> = r
        .hosts
        .iter()
        .filter(|h| h.runner.swarm(0).rejected > 0)
        .map(|h| format!("{} {}", h.name, h.runner.swarm(0).rejected))
        .collect();
    assert!(
        rejected.is_empty(),
        "honest peers sent rejected messages to: {}",
        rejected.join(", ")
    );
}

// --- 3. viewer churn --------------------------------------------------------

/// (viewer index, crash at, restart at)
const CHURN: [(usize, u64, u64); 3] = [(3, 12, 17), (6, 16, 22), (8, 24, 26)];
/// After a restart, frames written this long after it must all arrive.
const CATCH_UP: u64 = 8;

fn viewer_churn(seed: u64, push: PushMode) -> Scenario {
    let mut m = streaming(
        &format!("viewer_churn {push:?}"),
        seed,
        10,
        (2048 * KIB, 1024 * KIB),
        push,
        (3, 45),
    );
    for (v, down, up) in CHURN {
        m.s.at(secs(down), Fault::Crash(m.viewers[v]));
        m.s.at(secs(up), Fault::Restart(m.viewers[v]));
    }
    m.s.invariants.max_heap = Some(96 * MIB);
    m.s.invariants.progress = vec![(secs(8), secs(45))];
    m.s
}

fn check_viewer_churn(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    let viewers = all_viewers(r);
    let churned: Vec<HostIx> = CHURN.iter().map(|&(v, _, _)| viewers[v]).collect();
    let steady: Vec<HostIx> = viewers
        .iter()
        .copied()
        .filter(|h| !churned.contains(h))
        .collect();
    complete(r, &steady, spec.start, spec.stop)?;
    for (v, _, up) in CHURN {
        let h = viewers[v];
        complete(r, &[h], secs(up + CATCH_UP), spec.stop)?;
        let joins = &r.viewer(0, h).unwrap().stats.joins;
        if joins.len() != 2 || joins[1].caught_up_ms.is_none() {
            return Err(format!("v{v} rejoined without catching up: {joins:?}"));
        }
    }
    Ok(())
}

#[test]
fn viewer_churn_pull() {
    sweep(
        "viewer_churn_pull",
        |s| viewer_churn(s, PushMode::Off),
        check_viewer_churn,
    );
}

#[test]
fn viewer_churn_hybrid() {
    sweep(
        "viewer_churn_hybrid",
        |s| viewer_churn(s, PushMode::Hybrid),
        check_viewer_churn,
    );
}

// --- 4. partition and heal -------------------------------------------------

const SPLIT: u64 = 12;
const HEAL: u64 = 27;
/// After the heal, links that lost segments wait out the 25 s read timeout
/// before they are redialed; frames written this long after the heal must
/// arrive everywhere.
const HEAL_GRACE: u64 = 30;

fn partition_heal_scenario(seed: u64) -> Scenario {
    let mut m = streaming(
        "partition_heal",
        seed,
        11,
        (2048 * KIB, 1024 * KIB),
        PushMode::Off,
        (3, 75),
    );
    let (a, b) = m.viewers.split_at(5);
    let side_a: Vec<HostIx> = std::iter::once(m.seed).chain(a.iter().copied()).collect();
    let side_b = b.to_vec();
    m.s.at(
        secs(SPLIT),
        Fault::Partition(side_a.clone(), side_b.clone()),
    );
    m.s.at(secs(HEAL), Fault::Repair(side_a, side_b));
    m.s.sample_every = secs(2);
    m.s.invariants.max_heap = Some(96 * MIB);
    m.s
}

fn check_partition_heal(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    let viewers = all_viewers(r);
    let (a, b) = viewers.split_at(5);
    complete(r, a, spec.start, spec.stop)?;
    complete(r, b, spec.start, secs(SPLIT - 1))?;
    complete(r, b, secs(HEAL + HEAL_GRACE), spec.stop)?;
    // Memory stays flat while the halves are apart.
    let m = r.memory.as_ref().ok_or("no memory report")?;
    let before = m.max_between(0, SPLIT * 1000);
    let during = m.max_between(SPLIT * 1000, (HEAL + HEAL_GRACE) * 1000);
    if during > before + before / 4 + 4 * MIB {
        return Err(format!(
            "heap grew from {} KiB to {} KiB during the partition",
            before / 1024,
            during / 1024
        ));
    }
    Ok(())
}

#[test]
fn partition_heal() {
    sweep(
        "partition_heal",
        partition_heal_scenario,
        check_partition_heal,
    );
}

// --- 5. the seed crashes and restarts, then resets its writer --------------

const SEED_DOWN: u64 = 15;
const SEED_UP: u64 = 18;
const WRITER_RESET: u64 = 32;

fn seed_restart(seed: u64, push: PushMode) -> Scenario {
    let mut m = streaming(
        &format!("seed_restart {push:?}"),
        seed,
        8,
        (2048 * KIB, 1024 * KIB),
        push,
        (3, 50),
    );
    m.s.at(secs(SEED_DOWN), Fault::Crash(m.seed));
    m.s.at(secs(SEED_UP), Fault::Restart(m.seed));
    m.s.at(secs(WRITER_RESET), Fault::ResetWriter(0));
    m.s.invariants.max_heap = Some(96 * MIB);
    m.s
}

fn check_seed_restart(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    let viewers = all_viewers(r);
    complete(r, &viewers, spec.start, secs(SEED_DOWN - 1))?;
    // A new epoch drops what the old one hadn't delivered yet, so the last
    // second before each fault may be lost.
    complete(
        r,
        &viewers,
        secs(SEED_UP + CATCH_UP),
        secs(WRITER_RESET - 1),
    )?;
    complete(r, &viewers, secs(WRITER_RESET + CATCH_UP), spec.stop)?;
    for &h in &viewers {
        let v = r.viewer(0, h).unwrap();
        if v.stats.resets < 2 {
            return Err(format!("{} saw {} resets, not 2", v.name, v.stats.resets));
        }
    }
    Ok(())
}

#[test]
fn seed_restart_pull() {
    sweep(
        "seed_restart_pull",
        |s| seed_restart(s, PushMode::Off),
        check_seed_restart,
    );
}

#[test]
fn seed_restart_hybrid() {
    sweep(
        "seed_restart_hybrid",
        |s| seed_restart(s, PushMode::Hybrid),
        check_seed_restart,
    );
}

/// Stores that keep each chunk's frames as
/// [`FrameCodec`](strims_sim::stream::FrameCodec) state make the trace that
/// stores keeping the bytes do, through a seed restart and a writer reset
/// (sequence numbers go on while chunk offsets start over), in chunks of a
/// frame and of four.
#[test]
fn frame_codec_changes_no_trace() {
    let seed = 1;
    for chunks in [None, Some((4 * KIB as usize, 8 * KIB as usize))] {
        let scenario = |stored_chunks| {
            let mut s = seed_restart(seed, PushMode::Hybrid);
            if let Some((chunk, munro)) = chunks {
                s.swarms = std::mem::take(&mut s.swarms)
                    .into_iter()
                    .map(|sw| sw.with_chunks(chunk, munro).unwrap())
                    .collect();
            }
            s.stored_chunks = stored_chunks;
            s
        };
        let stored = strims_sim::run(scenario(true), seed).unwrap();
        let coded = strims_sim::run(scenario(false), seed).unwrap();
        assert_eq!(coded.trace_hash, stored.trace_hash, "{chunks:?}");
        check_seed_restart(&coded).unwrap();
        // every node's window, 1 MiB, less 16 bytes a chunk
        let peak = |r: &Report| r.memory.as_ref().map_or(0, |m| m.peak);
        assert!(
            peak(&coded) + 8 * MIB < peak(&stored),
            "{chunks:?}: {} against {}",
            peak(&coded),
            peak(&stored)
        );
    }
}

// --- 6. vpn services over multi-hop paths ----------------------------------

/// How the vpn scenarios lay out their hosts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VpnTopology {
    /// 16 hosts, each dialing 2 earlier ones (about 4 links per host);
    /// peer exchanges add links as the run goes.
    Mesh,
    /// 32 hosts, each dialing 2 earlier ones, no peer exchanges: about 4
    /// links per host, a sparse graph for comparing vpn routings.
    Sparse,
    /// 24 hosts in a random tree (each dials one earlier host), no peer
    /// exchanges: one path between any pair, often longer than vpn's
    /// 5-hop limit.
    Tree,
}

/// One network with explicit bindings over every host. Probes: a unicast
/// every 100 ms, a hash table get and a peer index search every 500 ms,
/// and (mesh only) a peer exchange every 3 s. With `churn`, three hosts
/// crash and restart.
fn vpn(
    seed: u64,
    topology: VpnTopology,
    routing: Routing,
    greedy_relays: bool,
    churn: bool,
) -> Scenario {
    let name = format!(
        "vpn {topology:?} {routing:?}{}{}",
        if greedy_relays { " +greedy relays" } else { "" },
        if churn { " churn" } else { "" }
    );
    let mut s = Scenario::new(&name);
    s.routing = routing;
    let (n, degree) = match topology {
        VpnTopology::Mesh => (16, 2),
        VpnTopology::Sparse => (32, 2),
        VpnTopology::Tree => (24, 1),
    };
    let hosts = s.hosts("h", n, Some(1024 * KIB));
    if greedy_relays {
        for &h in hosts.iter().step_by(2) {
            s.hosts[h].routing = Some(Routing::Greedy);
        }
    }
    random_mesh(&mut s, &mut det_rng(seed), &hosts, degree, 0.4);
    let net = s.network(NetworkSpec {
        members: hosts.clone(),
        binding: Binding::Explicit,
        port: 9000,
    });
    s.probes = Some(VpnProbes {
        network: net,
        start: secs(5),
        stop: secs(55),
        unicast_every: Some(Duration::from_millis(100)),
        lookup_every: Some(Duration::from_millis(500)),
        exchange_every: match topology {
            VpnTopology::Mesh => Some(secs(3)),
            VpnTopology::Sparse | VpnTopology::Tree => None,
        },
    });
    if churn {
        for (h, down, up) in [(5, 15, 25), (11, 20, 30), (2, 35, 38)] {
            s.at(secs(down), Fault::Crash(hosts[h]));
            s.at(secs(up), Fault::Restart(hosts[h]));
        }
    }
    s.duration = secs(60);
    s.invariants.max_heap = Some(128 * MIB);
    s
}

/// Delivery rates are printed in the summaries; the floors only catch a
/// collapse: `min_unicast` of all unicasts, `min_in_reach` of those between
/// hosts at most 5 links apart (vpn's hop limit), and `min_lookups` of the
/// hash table gets and peer index searches, overall and among those started
/// in the first 10 s (5–10 s). Hosts publish at startup, before they have
/// links, and publishes repeat only every 10 minutes, so the lookups also
/// check that vpn gives records to links that come up later.
fn check_vpn(
    min_unicast: f64,
    min_in_reach: f64,
    min_lookups: f64,
) -> impl Fn(&Report) -> Result<(), String> {
    move |r| {
        sound(r)?;
        let v = r.vpn.as_ref().ok_or("no vpn report")?;
        if v.unicast_rate() < min_unicast || v.in_reach_rate() < min_in_reach {
            return Err(format!(
                "unicast delivery {:.3} (in reach {:.3}) below {min_unicast} ({min_in_reach})",
                v.unicast_rate(),
                v.in_reach_rate()
            ));
        }
        if v.hops.keys().any(|&h| h > 5) {
            return Err(format!("a unicast took more than 5 hops: {:?}", v.hops));
        }
        let first = v.lookups_by_phase.first().cloned().unwrap_or_default();
        let rate = |ok: u64, n: u64| if n == 0 { 0.0 } else { ok as f64 / n as f64 };
        for (what, got) in [
            ("gets", v.get_rate()),
            ("searches", v.search_rate()),
            ("gets in the first 10 s", rate(first.gets_ok, first.gets)),
            (
                "searches in the first 10 s",
                rate(first.searches_ok, first.searches),
            ),
        ] {
            if got < min_lookups {
                return Err(format!("{what}: {got:.3} found, below {min_lookups}"));
            }
        }
        if r.hosts.iter().all(|h| h.vpn_records_pushed == 0) {
            return Err("vpn pushed no records to new links".to_owned());
        }
        Ok(())
    }
}

#[test]
fn vpn_sparse_learned() {
    sweep(
        "vpn_sparse_learned",
        |s| vpn(s, VpnTopology::Sparse, Routing::Learned, false, false),
        check_vpn(0.95, 0.95, 0.8),
    );
}

#[test]
fn vpn_sparse_greedy() {
    sweep(
        "vpn_sparse_greedy",
        |s| vpn(s, VpnTopology::Sparse, Routing::Greedy, false, false),
        check_vpn(0.2, 0.2, 0.8),
    );
}

#[test]
fn vpn_sparse_mixed() {
    sweep(
        "vpn_sparse_mixed",
        |s| vpn(s, VpnTopology::Sparse, Routing::Learned, true, false),
        check_vpn(0.2, 0.2, 0.8),
    );
}

#[test]
fn vpn_learned_churn() {
    sweep(
        "vpn_learned_churn",
        |s| vpn(s, VpnTopology::Mesh, Routing::Learned, false, true),
        check_vpn(0.9, 0.9, 0.8),
    );
}

#[test]
fn vpn_greedy_churn() {
    sweep(
        "vpn_greedy_churn",
        |s| vpn(s, VpnTopology::Mesh, Routing::Greedy, false, true),
        check_vpn(0.2, 0.2, 0.8),
    );
}

#[test]
fn vpn_tree_learned() {
    sweep(
        "vpn_tree_learned",
        |s| vpn(s, VpnTopology::Tree, Routing::Learned, false, false),
        check_vpn(0.5, 0.95, 0.3),
    );
}

#[test]
fn vpn_tree_learned_churn() {
    sweep(
        "vpn_tree_learned_churn",
        |s| vpn(s, VpnTopology::Tree, Routing::Learned, false, true),
        check_vpn(0.3, 0.5, 0.3),
    );
}

// --- 7. a viewer that stops reading -----------------------------------------

const PAUSE: u64 = 10;
/// The 40 s pause is longer than the 1 MiB (32 s) window, so the reader
/// falls out of it.
const RESUME: u64 = 50;

fn slow_consumer_scenario(seed: u64) -> Scenario {
    let mut m = streaming(
        "slow_consumer",
        seed,
        6,
        (2048 * KIB, 1024 * KIB),
        PushMode::Off,
        (3, 70),
    );
    m.s.at(secs(PAUSE), Fault::PauseReading(m.viewers[2]));
    m.s.at(secs(RESUME), Fault::ResumeReading(m.viewers[2]));
    m.s.sample_every = secs(2);
    m.s.invariants.max_heap = Some(64 * MIB);
    m.s.invariants.progress = vec![(secs(8), secs(70))];
    m.s
}

fn check_slow_consumer(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    let slow = r.scenario.swarms[0].viewers[2];
    let others: Vec<HostIx> = all_viewers(r).into_iter().filter(|&h| h != slow).collect();
    complete(r, &others, spec.start, spec.stop)?;
    complete(r, &[slow], spec.start, secs(PAUSE - 1))?;
    complete(r, &[slow], secs(RESUME + 2), spec.stop)?;
    let v = r.viewer(0, slow).unwrap();
    if v.stats.underruns == 0 {
        return Err("the paused reader never underran".to_owned());
    }
    // The paused node's memory doesn't grow: the store is a ring.
    let m = r.memory.as_ref().ok_or("no memory report")?;
    let before = m.max_between(5_000, PAUSE * 1000);
    let during = m.max_between(PAUSE * 1000, RESUME * 1000);
    if during > before + before / 10 + 2 * MIB {
        return Err(format!(
            "heap grew from {} KiB to {} KiB while a reader was paused",
            before / 1024,
            during / 1024
        ));
    }
    Ok(())
}

#[test]
fn slow_consumer() {
    sweep("slow_consumer", slow_consumer_scenario, check_slow_consumer);
}

// --- 8. held links and latency changes -------------------------------------

const HOLD: u64 = 10;
const RELEASE: u64 = 13;
const SLOW_FROM: u64 = 20;
const SLOW_TO: u64 = 30;

fn hold_and_latency_scenario(seed: u64) -> Scenario {
    let mut m = streaming(
        "hold_and_latency",
        seed,
        8,
        (2048 * KIB, 1024 * KIB),
        PushMode::Off,
        (3, 40),
    );
    let seed_side = vec![m.seed];
    let neighbours = m.s.neighbours(m.seed);
    m.s.at(
        secs(HOLD),
        Fault::Hold(seed_side.clone(), m.viewers.clone()),
    );
    m.s.at(
        secs(RELEASE),
        Fault::Release(seed_side.clone(), m.viewers.clone()),
    );
    m.s.at(
        secs(SLOW_FROM),
        Fault::Latency(
            seed_side.clone(),
            neighbours.clone(),
            Duration::from_millis(150),
        ),
    );
    m.s.at(
        secs(SLOW_TO),
        Fault::Latency(seed_side, neighbours, Duration::from_millis(10)),
    );
    m.s.invariants.max_heap = Some(64 * MIB);
    m.s
}

fn check_hold_and_latency(r: &Report) -> Result<(), String> {
    sound(r)?;
    whole_stream(r)?;
    // Held messages arrive late but intact: no link drops.
    let dropped = r.trace.iter().filter(|l| l.contains(" peer- ")).count();
    if dropped > 0 {
        return Err(format!("{dropped} peers dropped"));
    }
    // The hold shows up as lag: frames written during it wait for the
    // release.
    let spec = &r.scenario.swarms[0];
    let held = spec.frames_between(secs(HOLD), secs(RELEASE));
    let lag = max_lag(r, held);
    if lag < 1000 {
        return Err(format!(
            "frames written during the hold arrived after {lag} ms"
        ));
    }
    Ok(())
}

/// The largest lag of any frame in `range` at any viewer.
fn max_lag(r: &Report, range: std::ops::Range<u64>) -> u32 {
    r.viewers
        .iter()
        .map(|v| v.stats.lag_in(range.clone()).max)
        .max()
        .unwrap_or(0)
}

#[test]
fn hold_and_latency() {
    sweep(
        "hold_and_latency",
        hold_and_latency_scenario,
        check_hold_and_latency,
    );
}

// --- 8b. a latency step on the seed's links ---------------------------------

const STEP_AT: u64 = 20;
const HIGH_LATENCY: Duration = Duration::from_millis(150);

/// The seed's links run at turmoil's 2–20 ms, then at 150 ms one way from
/// `STEP_AT` on (or from the start, the control).
fn latency_step(seed: u64, push: PushMode, from_start: bool) -> Scenario {
    let mut m = streaming(
        &format!(
            "latency_{} {push:?}",
            if from_start { "high" } else { "step" }
        ),
        seed,
        8,
        (2048 * KIB, 1024 * KIB),
        push,
        (3, 60),
    );
    let at = if from_start { 0 } else { STEP_AT };
    let neighbours = m.s.neighbours(m.seed);
    m.s.at(
        secs(at),
        Fault::Latency(vec![m.seed], neighbours, HIGH_LATENCY),
    );
    m.s.invariants.max_heap = Some(64 * MIB);
    m.s
}

/// Ten seconds after the step, the stream flows again at under 2 s lag.
fn check_latency_step(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    let from = secs(STEP_AT + 10);
    complete(r, &all_viewers(r), from, spec.stop)?;
    let lag = max_lag(r, spec.frames_between(from, spec.stop));
    if lag > 2000 {
        return Err(format!("lag still {lag} ms after the step"));
    }
    Ok(())
}

#[test]
fn latency_high_pull() {
    sweep(
        "latency_high_pull",
        |s| latency_step(s, PushMode::Off, true),
        check_latency_step,
    );
}

#[test]
fn latency_step_pull() {
    sweep(
        "latency_step_pull",
        |s| latency_step(s, PushMode::Off, false),
        check_latency_step,
    );
}

#[test]
fn latency_step_hybrid() {
    sweep(
        "latency_step_hybrid",
        |s| latency_step(s, PushMode::Hybrid, false),
        check_latency_step,
    );
}

// --- 8c. the seed is cut off for longer than the window ------------------

const CUT: u64 = 10;
const RECONNECT: u64 = 50;

/// The seed is partitioned from every viewer for 40 s, longer than the
/// 32 s window, so when the links come back every viewer is more than a
/// window behind the seed.
fn seed_outage(seed: u64, push: PushMode, mode: DeliveryMode) -> Scenario {
    let mut m = streaming(
        &format!("seed_outage {push:?} {mode:?}"),
        seed,
        8,
        (2048 * KIB, 1024 * KIB),
        push,
        (3, 80),
    );
    m.s.swarms[0].options.delivery_mode = mode;
    m.s.at(secs(CUT), Fault::Partition(vec![m.seed], m.viewers.clone()));
    m.s.at(
        secs(RECONNECT),
        Fault::Repair(vec![m.seed], m.viewers.clone()),
    );
    m.s.invariants.max_heap = Some(64 * MIB);
    m.s
}

/// Frames written from 15 s after the links are back reach everyone.
fn check_seed_outage(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    complete(r, &all_viewers(r), secs(RECONNECT + 15), spec.stop)
}

#[test]
fn seed_outage_pull() {
    sweep(
        "seed_outage_pull",
        |s| seed_outage(s, PushMode::Off, DeliveryMode::MANDATORY),
        check_seed_outage,
    );
}

#[test]
fn seed_outage_pull_low_latency() {
    sweep(
        "seed_outage_pull_low_latency",
        |s| seed_outage(s, PushMode::Off, DeliveryMode::LOW_LATENCY),
        check_seed_outage,
    );
}

#[test]
fn seed_outage_hybrid() {
    sweep(
        "seed_outage_hybrid",
        |s| seed_outage(s, PushMode::Hybrid, DeliveryMode::MANDATORY),
        check_seed_outage,
    );
}

// --- 8d. the seed's uplink drops below the stream rate ----------------------

const STARVE: u64 = 10;
const FEED: u64 = 90;

/// The seed's uplink falls to half the stream rate for 80 s, so viewers
/// fall more than a window (32 s) behind on links that stay up, then it
/// comes back.
fn seed_starved(seed: u64, push: PushMode) -> Scenario {
    let mut m = streaming(
        &format!("seed_starved {push:?}"),
        seed,
        8,
        (1024 * KIB, 1024 * KIB),
        push,
        (3, 130),
    );
    m.s.at(secs(STARVE), Fault::Uplink(m.seed, RATE / 2));
    m.s.at(secs(FEED), Fault::Uplink(m.seed, 1024 * KIB));
    m.s.invariants.max_heap = Some(64 * MIB);
    m.s
}

/// Frames written from 15 s after the uplink is back reach everyone.
fn check_seed_starved(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    complete(r, &all_viewers(r), secs(FEED + 15), spec.stop)
}

#[test]
fn seed_starved_pull() {
    sweep(
        "seed_starved_pull",
        |s| seed_starved(s, PushMode::Off),
        check_seed_starved,
    );
}

#[test]
fn seed_starved_hybrid() {
    sweep(
        "seed_starved_hybrid",
        |s| seed_starved(s, PushMode::Hybrid),
        check_seed_starved,
    );
}

// --- 8e. WebRTC upgrades -----------------------------------------------------

const UPGRADE: u64 = 2;
const RELAY_DOWN: u64 = 20;

/// A seed and 4 viewers with no listeners, only WebRTC, each linked to a
/// relay over TCP or WebSocket. The relay isn't in the swarm, so nothing
/// streams until the seed links to every viewer through peer exchange
/// (the descriptions go through the relay) and a direct WebRTC link comes
/// up. Then the relay crashes for good. turmoil carries the UDP.
fn webrtc_upgrade(seed: u64, push: PushMode) -> Scenario {
    let mut s = Scenario::new(&format!("webrtc_upgrade {push:?}"));
    let mut rng = det_rng(seed);
    let relay = s.host("relay", Some(2048 * KIB));
    let seed_host = s.host("seed", Some(2048 * KIB));
    let viewers = s.hosts("v", 4, Some(1024 * KIB));
    for &h in std::iter::once(&seed_host).chain(&viewers) {
        s.hosts[h].listen = false;
        s.hosts[h].webrtc = true;
        s.link(h, relay, strims_sim::topology::transport(&mut rng, 0.5));
    }
    let mut all = vec![relay, seed_host];
    all.extend(&viewers);
    let net = s.network(NetworkSpec {
        members: all,
        binding: Binding::Auto,
        port: 9000,
    });
    for &v in &viewers {
        s.at(
            secs(UPGRADE),
            Fault::Connect {
                from: seed_host,
                to: v,
                network: net,
            },
        );
    }
    s.at(secs(RELAY_DOWN), Fault::Crash(relay));
    s.swarm(SwarmSpec::live(seed_host, viewers, RATE, secs(5), secs(40)).with_push_mode(push));
    s.duration = secs(48);
    s.invariants.max_heap = Some(64 * MIB);
    s.invariants.progress = vec![(secs(10), secs(40))];
    s
}

fn check_webrtc_upgrade(r: &Report) -> Result<(), String> {
    sound(r)?;
    whole_stream(r)?;
    let webrtc = r
        .trace
        .iter()
        .filter(|l| l.contains(" seed peer+ to ") && l.ends_with("WebRtc"))
        .count();
    if webrtc != 4 {
        return Err(format!("{webrtc} WebRTC links from the seed, not 4"));
    }
    Ok(())
}

#[test]
fn webrtc_upgrade_pull() {
    sweep(
        "webrtc_upgrade_pull",
        |s| webrtc_upgrade(s, PushMode::Off),
        check_webrtc_upgrade,
    );
}

#[test]
fn webrtc_upgrade_hybrid() {
    sweep(
        "webrtc_upgrade_hybrid",
        |s| webrtc_upgrade(s, PushMode::Hybrid),
        check_webrtc_upgrade,
    );
}

// --- 8f. cross traffic: two streams on the same capped uplinks ---------------

/// Every host's uplink in `cross_traffic`.
const CROSS_UPLINK: u64 = 7 * RATE / 2;

/// Two streams at `RATE`, each seeded by its own host and viewed by the
/// other seed and 8 viewers, over one random mesh whose uplinks are all
/// capped at `CROSS_UPLINK`: every relay carries both streams, about 2x
/// the stream rate on average plus overhead.
fn cross_traffic(seed: u64, push: PushMode) -> Scenario {
    let mut s = Scenario::new(&format!("cross_traffic {push:?}"));
    let mut rng = det_rng(seed);
    let seed_a = s.host("seed_a", Some(CROSS_UPLINK));
    let seed_b = s.host("seed_b", Some(CROSS_UPLINK));
    let vs = s.hosts("v", 8, Some(CROSS_UPLINK));
    let all: Vec<HostIx> = [seed_a, seed_b].into_iter().chain(vs.clone()).collect();
    random_mesh(&mut s, &mut rng, &all, 3, 0.4);
    let mut viewers_a = vs.clone();
    viewers_a.push(seed_b);
    let mut viewers_b = vs;
    viewers_b.push(seed_a);
    for (seed_host, viewers) in [(seed_a, viewers_a), (seed_b, viewers_b)] {
        s.swarm(SwarmSpec::live(seed_host, viewers, RATE, secs(3), secs(43)).with_push_mode(push));
    }
    // No progress invariant: near capacity, pull's lag reaches several
    // seconds and a viewer can go 5 s without a new frame. Its tail
    // reaches 13 s, so the run lasts that long past the last write.
    s.duration = secs(56);
    s.invariants.max_heap = Some(96 * MIB);
    s
}

/// Both streams arrive whole at every viewer.
fn check_cross_traffic(r: &Report) -> Result<(), String> {
    sound(r)?;
    for sw in 0..2 {
        let spec = &r.scenario.swarms[sw];
        complete_swarm(r, sw, &spec.viewers, spec.start, spec.stop)?;
    }
    Ok(())
}

#[test]
fn cross_traffic_pull() {
    sweep(
        "cross_traffic_pull",
        |s| cross_traffic(s, PushMode::Off),
        check_cross_traffic,
    );
}

#[test]
fn cross_traffic_hybrid() {
    sweep(
        "cross_traffic_hybrid",
        |s| cross_traffic(s, PushMode::Hybrid),
        check_cross_traffic,
    );
}

// --- 9. ten minutes ---------------------------------------------------------

/// A seed and 8 viewers for 10 minutes at half the rate, a viewer crashing
/// and restarting every minute, invariants checked every 10 s.
fn long_run_scenario(seed: u64) -> Scenario {
    let mut m = streaming(
        "long_run",
        seed,
        8,
        (1024 * KIB, 512 * KIB),
        PushMode::Off,
        (3, 595),
    );
    m.s.swarms[0] = SwarmSpec::live(m.seed, m.viewers.clone(), RATE / 2, secs(3), secs(595));
    for minute in 1..10u64 {
        let v = m.viewers[(seed as usize + minute as usize) % m.viewers.len()];
        m.s.at(secs(minute * 60), Fault::Crash(v));
        m.s.at(secs(minute * 60 + 10), Fault::Restart(v));
    }
    m.s.sample_every = secs(10);
    m.s.duration = secs(600);
    m.s.invariants.max_heap = Some(64 * MIB);
    m.s.invariants.progress = vec![(secs(10), secs(595))];
    m.s
}

fn check_long_run(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    // Every viewer has every frame written while it was up and settled.
    for v in &r.viewers {
        for (i, j) in v.stats.joins.iter().enumerate() {
            let from = Duration::from_millis(j.start_ms) + secs(CATCH_UP);
            let to = r
                .scenario
                .faults
                .iter()
                .filter(|f| f.fault == Fault::Crash(v.host) && ms(f.at) > j.start_ms)
                .map(|f| f.at - secs(1))
                .min()
                .unwrap_or(spec.stop);
            if from < to {
                complete(r, &[v.host], from, to)
                    .map_err(|e| format!("incarnation {}: {e}", i + 1))?;
            }
        }
    }
    // The heap stays flat: the last five minutes peak no higher than the
    // first five (after the first minute's warm-up), give or take 15 %.
    let m = r.memory.as_ref().ok_or("no memory report")?;
    let per_minute: Vec<String> = (0..10)
        .map(|i| format!("{}", m.max_between(i * 60_000, (i + 1) * 60_000) / 1024))
        .collect();
    eprintln!("long_run heap per minute (KiB): {}", per_minute.join(" "));
    let early = m.max_between(60_000, 300_000);
    let late = m.max_between(300_000, 600_000);
    if late > early + early * 3 / 20 {
        return Err(format!(
            "heap grew from {} KiB (minutes 1–5) to {} KiB (minutes 5–10)",
            early / 1024,
            late / 1024
        ));
    }
    Ok(())
}

#[test]
#[ignore = "10 simulated minutes, run twice for the replay; run in release with --include-ignored"]
fn long_run() {
    sweep("long_run", long_run_scenario, check_long_run);
}

// --- 10. WAN paths ------------------------------------------------------------

/// The stream rate of the WAN scenarios: about 2 Mbit/s.
const WAN_RATE: u64 = 256 * KIB;
/// Their live window: 16 s of stream.
const WAN_WINDOW: usize = 4096;
/// Salt of the rng that draws viewer sites.
const SITES: u64 = 0x5349_5445;

/// A seed at `seed_site` and a viewer at each of `sites`, in a seeded
/// random mesh (each host dials up to 3 earlier ones, 40 % over
/// WebSocket), streaming `WAN_RATE` from `start` to `stop`.
fn wan(
    name: &str,
    seed: u64,
    seed_site: Site,
    sites: &[Site],
    push: PushMode,
    (start, stop): (u64, u64),
) -> Streaming {
    let mut s = Scenario::new(name);
    let mut rng = det_rng(seed);
    let seed_host = s.host_at("seed", seed_site);
    let vs = s.hosts_at("v", sites);
    let all: Vec<HostIx> = std::iter::once(seed_host)
        .chain(vs.iter().copied())
        .collect();
    random_mesh(&mut s, &mut rng, &all, 3, 0.4);
    s.swarm(
        SwarmSpec::live(seed_host, vs.clone(), WAN_RATE, secs(start), secs(stop))
            .with_push_mode(push)
            .with_live_window(WAN_WINDOW),
    );
    s.invariants = Invariants {
        max_heap: Some(256 * MIB),
        ..Invariants::for_window(WAN_WINDOW)
    };
    s.duration = secs(stop + 10);
    Streaming {
        s,
        seed: seed_host,
        viewers: vs,
        rng,
    }
}

/// `n` viewer sites drawn by weight from `cities` and `access`.
fn viewer_sites(
    seed: u64,
    n: usize,
    cities: &[(City, u32)],
    access: &[(Access, u32)],
) -> Vec<Site> {
    let mut rng = det_rng(seed ^ SITES);
    (0..n)
        .map(|_| {
            let city = pick(&mut rng, cities).unwrap();
            Site::new(city, pick(&mut rng, access).unwrap())
        })
        .collect()
}

/// `GLOBAL_AUDIENCE`'s cities in North America and Europe.
fn atlantic() -> Vec<(City, u32)> {
    GLOBAL_AUDIENCE
        .iter()
        .copied()
        .filter(|(c, _)| matches!(c.region(), Region::NorthAmerica | Region::Europe))
        .collect()
}

/// The viewers of swarm 0 whose site is in `region`.
fn in_region(r: &Report, region: Region) -> Vec<HostIx> {
    all_viewers(r)
        .into_iter()
        .filter(|&h| {
            r.scenario.hosts[h]
                .site
                .is_some_and(|s| s.city.region() == region)
        })
        .collect()
}

/// No access link queued a message for longer than `max`.
fn queues_within(r: &Report, max: Duration) -> Result<(), String> {
    let wan = r.wan.as_ref().ok_or("not a WAN run")?;
    let ((up, up_host), (down, down_host)) = wan.queue_peaks();
    let max = ms(max);
    if u64::from(up) > max || u64::from(down) > max {
        return Err(format!(
            "access queues reached {up} ms up ({up_host}) and {down} ms down ({down_host}), over {max} ms"
        ));
    }
    Ok(())
}

/// The p99 lag of swarm 0 is at most `max` ms.
fn p99_within(r: &Report, max: u32) -> Result<(), String> {
    let lag = r.lag(0);
    if lag.p99 > max {
        return Err(format!("lag {lag}, p99 over {max} ms"));
    }
    Ok(())
}

// 10a. Two continents: a London seed, three viewers in Europe and three
// in North America.

fn wan_two_continents_scenario(seed: u64) -> Scenario {
    let sites = [
        Site::new(City::Amsterdam, Access::Cable),
        Site::new(City::Frankfurt, Access::Fiber),
        Site::new(City::Warsaw, Access::Dsl),
        Site::new(City::NewYork, Access::Cable),
        Site::new(City::Chicago, Access::Fiber),
        Site::new(City::Toronto, Access::Lte),
    ];
    let seed_site = Site::new(City::London, Access::Fiber);
    let mut m = wan(
        "wan_two_continents",
        seed,
        seed_site,
        &sites,
        PushMode::Off,
        (5, 30),
    );
    m.s.invariants.max_heap = Some(96 * MIB);
    m.s
}

/// The stream reaches everyone from 8 s on, North America reads it later
/// than Europe, and no access link queues for a second.
fn check_wan_two_continents(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    complete(r, &all_viewers(r), secs(8), spec.stop)?;
    queues_within(r, secs(1))?;
    let eu = r.lag_of(0, &in_region(r, Region::Europe));
    let na = r.lag_of(0, &in_region(r, Region::NorthAmerica));
    if na.p50 <= eu.p50 {
        return Err(format!("north america lag {na}, europe {eu}"));
    }
    let sites = r.trace.iter().filter(|l| l.starts_with("0 wan ")).count();
    if sites != r.scenario.hosts.len() {
        return Err(format!("{sites} site lines in the trace"));
    }
    Ok(())
}

#[test]
fn wan_two_continents() {
    sweep(
        "wan_two_continents",
        wan_two_continents_scenario,
        check_wan_two_continents,
    );
}

// 10b. A worldwide audience of a datacenter seed.

fn wan_global_audience(seed: u64, push: PushMode) -> Scenario {
    let sites = viewer_sites(seed, 16, GLOBAL_AUDIENCE, RESIDENTIAL);
    wan(
        &format!("wan_global_audience {push:?}"),
        seed,
        Site::new(City::Frankfurt, Access::Datacenter),
        &sites,
        push,
        (5, 65),
    )
    .s
}

/// Everyone gets the stream from 10 s on, within 3.5 s at p99, through
/// access queues under 2 s.
fn check_wan_global_audience(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    complete(r, &all_viewers(r), secs(10), spec.stop)?;
    p99_within(r, 3500)?;
    queues_within(r, secs(2))
}

#[test]
#[ignore = "WAN, 17 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_global_audience_pull() {
    sweep(
        "wan_global_audience_pull",
        |s| wan_global_audience(s, PushMode::Off),
        check_wan_global_audience,
    );
}

#[test]
#[ignore = "WAN, 17 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_global_audience_hybrid() {
    sweep(
        "wan_global_audience_hybrid",
        |s| wan_global_audience(s, PushMode::Hybrid),
        check_wan_global_audience,
    );
}

// 10c. A seed on a cable uplink that can serve a few viewers, and 16
// viewers around the Atlantic: the viewers relay.

fn wan_residential_seed(seed: u64, push: PushMode) -> Scenario {
    wan_residential_seed_for(seed, push, (5, 65))
}

/// [`wan_residential_seed`] streaming over `window` (seconds).
fn wan_residential_seed_for(seed: u64, push: PushMode, window: (u64, u64)) -> Scenario {
    residential_seed_mesh(seed, push, window).s
}

/// [`wan_residential_seed_for`] with its rng.
fn residential_seed_mesh(seed: u64, push: PushMode, window: (u64, u64)) -> Streaming {
    let sites = viewer_sites(seed, 16, &atlantic(), RESIDENTIAL);
    wan(
        &format!("wan_residential_seed {push:?}"),
        seed,
        Site::new(City::Chicago, Access::Cable),
        &sites,
        push,
        window,
    )
}

/// [`wan_residential_seed`]'s viewers plus a 17th in a New York
/// datacenter: a super-peer whose uplink could carry the whole swarm.
fn wan_super_peer(seed: u64, push: PushMode) -> Scenario {
    super_peer_mesh(seed, push).s
}

/// [`wan_super_peer`] with its rng.
fn super_peer_mesh(seed: u64, push: PushMode) -> Streaming {
    let mut sites = viewer_sites(seed, 16, &atlantic(), RESIDENTIAL);
    sites.push(Site::new(City::NewYork, Access::Datacenter));
    wan(
        &format!("wan_super_peer {push:?}"),
        seed,
        Site::new(City::Chicago, Access::Cable),
        &sites,
        push,
        (5, 65),
    )
}

/// Everyone gets the stream from 10 s on while the seed sends at most 60 %
/// of the data and at least 2 viewers relay.
fn check_wan_residential_seed(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    complete(r, &all_viewers(r), secs(10), spec.stop)?;
    let share = r.data_share(0);
    let relays = r.data_uploaders(0, 4 * MIB.unsigned_abs());
    if share > 0.6 || relays < 2 {
        return Err(format!(
            "seed data share {:.0}%, {relays} viewers relayed 4 MiB",
            share * 100.0
        ));
    }
    queues_within(r, secs(2))
}

#[test]
#[ignore = "WAN, 17 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_residential_seed_pull() {
    sweep(
        "wan_residential_seed_pull",
        |s| wan_residential_seed(s, PushMode::Off),
        check_wan_residential_seed,
    );
}

#[test]
#[ignore = "WAN, 17 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_residential_seed_hybrid() {
    sweep(
        "wan_residential_seed_hybrid",
        |s| wan_residential_seed(s, PushMode::Hybrid),
        check_wan_residential_seed,
    );
}

#[test]
#[ignore = "WAN, 17 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_residential_seed_adaptive() {
    sweep(
        "wan_residential_seed_adaptive",
        |s| striped(wan_residential_seed(s, PushMode::Hybrid), STRIPES, ADAPTIVE),
        check_wan_residential_seed,
    );
}

#[test]
#[ignore = "WAN, 18 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_super_peer_adaptive() {
    sweep(
        "wan_super_peer_adaptive",
        |s| striped(wan_super_peer(s, PushMode::Hybrid), STRIPES, ADAPTIVE),
        check_wan_residential_seed,
    );
}

/// The uplink queue of the uncapped WAN cases.
const UPLINK_QUEUE: Duration = Duration::from_millis(100);

/// `s` with no upload limits: each node's uplink paces it through its
/// sockets' send buffers, as on a node that sets no limit.
fn uncapped(mut s: Scenario) -> Scenario {
    for h in &mut s.hosts {
        h.uplink = None;
    }
    s.send_buffer = Some(SendBuffer {
        uplink_queue: UPLINK_QUEUE,
        window: false,
    });
    s.name = format!("{} uncapped", s.name);
    s
}

#[test]
#[ignore = "WAN, 17 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_residential_seed_adaptive_uncapped() {
    sweep(
        "wan_residential_seed_adaptive_uncapped",
        |s| {
            uncapped(striped(
                wan_residential_seed(s, PushMode::Hybrid),
                STRIPES,
                ADAPTIVE,
            ))
        },
        check_wan_residential_seed,
    );
}

/// Minutes the long WAN cases stream unless `STRIMS_SIM_MINUTES` says.
const LONG_MINUTES: u64 = 10;
/// Seeds the long WAN cases run unless `STRIMS_SIM_SEEDS` says.
const LONG_SEEDS: u64 = 2;

/// The long WAN cases measure how lag and the upload meter evolve over a
/// stream's length (the summary's per-bucket line, `STRIMS_SIM_ROWS`).
/// Everyone gets the stream from 10 s on, up to the last 20 s, whose
/// frames may still be in flight when the run ends: a channel that stops
/// requesting for good shows up here.
fn check_wan_long(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    complete(
        r,
        &all_viewers(r),
        secs(10),
        spec.stop.saturating_sub(secs(20)),
    )
}

#[test]
#[ignore = "WAN, 17 hosts at 2 Mbit/s for minutes; run in release with --include-ignored"]
fn wan_residential_seed_adaptive_uncapped_long() {
    let stop = 5 + 60 * stream_minutes(LONG_MINUTES);
    sweep_n(
        "wan_residential_seed_adaptive_uncapped_long",
        LONG_SEEDS,
        |s| {
            uncapped(striped(
                wan_residential_seed_for(s, PushMode::Hybrid, (5, stop)),
                STRIPES,
                ADAPTIVE,
            ))
        },
        check_wan_long,
    );
}

#[test]
#[ignore = "WAN, 18 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_super_peer_adaptive_uncapped() {
    sweep(
        "wan_super_peer_adaptive_uncapped",
        |s| {
            uncapped(striped(
                wan_super_peer(s, PushMode::Hybrid),
                STRIPES,
                ADAPTIVE,
            ))
        },
        check_wan_residential_seed,
    );
}

// 11. Calibration: the shaped links of hack/testbed/run.sh in the WAN
// model, so the upload meter's readings on a real link and on the
// simulated one can be compared.

/// The testbed's settings, under the names `run.sh` takes with a
/// `STRIMS_SIM_CAL_` prefix (and the uplink queue from
/// `STRIMS_SIM_SEND_BUFFER`). `STRIMS_SIM_CAL_JITTER=1` adds the cable
/// profile's jitter, so seeds differ the way a real link's timing does.
///
/// Three more serve sweeps on the sim farm, whose jobs take no files:
///
/// - `STRIMS_SIM_CAL_MESH=full` links every pair of nodes, and
///   `STRIMS_SIM_CAL_MESH=mesh6` takes the farm's testbed meshes
///   (`hack/sim-farm/matrices/topologies/mesh6-v<viewers>`: the seed linked
///   to up to 32 viewers, each viewer to 6 others), built in, for 3, 7, 15,
///   31 or 63 viewers. Either replaces `STRIMS_SIM_CAL_TOPOLOGY`.
/// - `STRIMS_SIM_CAL_SEED_UP_SCALE=<s>:<factor> ...` steps the seed's
///   uplink to `factor` times `STRIMS_SIM_CAL_SEED_UP_MBIT`, so one value
///   fits every seed uplink (`60:0.5 180:1` halves it from 60 s to 180 s).
///   It replaces `STRIMS_SIM_CAL_SEED_UP_STEPS`.
/// - `STRIMS_SIM_CAL_RATE_LIMIT_PCT=<pct>` gives every node a rate limit
///   (`NodeConfig::rate_limit`) of `pct` percent of its starting uplink,
///   so the node's QoS, not the access link, is where its peers' data
///   waits and the seed's holders' weights apply. A rate limit is also the
///   capacity the upload meter knows, and it doesn't follow the uplink's
///   steps, as a user's setting wouldn't.
struct Calibration {
    viewers: usize,
    seed_up_mbit: u64,
    viewer_up_mbit: u64,
    seed_delay_ms: u64,
    viewer_delay_ms: u64,
    secs: u64,
    join_at: u64,
    slow_down_kbit: Option<u64>,
    /// `link <a> <b>` lines, node 0 the seed and node `i` viewer `i - 1`;
    /// a star without.
    topology: Option<String>,
    /// The seed's uplink from given seconds of stream on, in Mbit/s
    /// (`<s>:<mbit> ...`).
    seed_up_steps: Vec<(f64, f64)>,
    /// Every node's rate limit, in percent of its starting uplink.
    rate_limit_pct: Option<u64>,
    /// The send buffers' congestion windows ([`SendBuffer::window`]), which
    /// `STRIMS_SIM_TCP=1` also turns on.
    window: bool,
    /// The swarm's chunk and munro sizes ([`SwarmSpec::with_chunks`]);
    /// `None` keeps the testbed's.
    chunks: Option<(usize, usize)>,
    /// Bulk uploads from the seed's host, from given seconds of stream on
    /// for given seconds (`<s>:<d> ...`), to a sink 30 ms away (the seed's
    /// delay and the testbed sink's). They need the window model
    /// (`STRIMS_SIM_TCP=1`).
    compete: Vec<(f64, f64)>,
}

fn calibration_settings() -> Calibration {
    let viewers = knob("STRIMS_SIM_CAL_VIEWERS").unwrap_or(3);
    let seed_up_mbit = knob("STRIMS_SIM_CAL_SEED_UP_MBIT").unwrap_or(10);
    let file = knob::<String>("STRIMS_SIM_CAL_TOPOLOGY");
    let mesh = knob::<String>("STRIMS_SIM_CAL_MESH");
    assert!(
        file.is_none() || mesh.is_none(),
        "STRIMS_SIM_CAL_TOPOLOGY or STRIMS_SIM_CAL_MESH, not both"
    );
    let topology = match mesh.as_deref() {
        None => file.map(|path| std::fs::read_to_string(&path).unwrap()),
        Some("full") => Some(full_mesh(viewers)),
        Some("mesh6") => Some(mesh6(viewers).to_owned()),
        Some(m) => panic!("STRIMS_SIM_CAL_MESH={m}: full or mesh6"),
    };
    let steps = pairs_knob("STRIMS_SIM_CAL_SEED_UP_STEPS");
    let scale = pairs_knob("STRIMS_SIM_CAL_SEED_UP_SCALE");
    assert!(
        steps.is_empty() || scale.is_empty(),
        "STRIMS_SIM_CAL_SEED_UP_STEPS or STRIMS_SIM_CAL_SEED_UP_SCALE, not both"
    );
    let seed_up_steps = if scale.is_empty() {
        steps
    } else {
        scale
            .into_iter()
            .map(|(at, f)| (at, f * seed_up_mbit as f64))
            .collect()
    };
    Calibration {
        viewers,
        seed_up_mbit,
        viewer_up_mbit: knob("STRIMS_SIM_CAL_VIEWER_UP_MBIT").unwrap_or(20),
        seed_delay_ms: knob("STRIMS_SIM_CAL_SEED_DELAY_MS").unwrap_or(20),
        viewer_delay_ms: knob("STRIMS_SIM_CAL_VIEWER_DELAY_MS").unwrap_or(10),
        secs: knob("STRIMS_SIM_CAL_SECS").unwrap_or(180),
        join_at: knob("STRIMS_SIM_CAL_JOIN_AT").unwrap_or(0),
        slow_down_kbit: knob("STRIMS_SIM_CAL_SLOW_DOWN_KBIT"),
        topology,
        seed_up_steps,
        rate_limit_pct: knob("STRIMS_SIM_CAL_RATE_LIMIT_PCT"),
        window: false,
        chunks: None,
        compete: pairs_knob("STRIMS_SIM_CAL_COMPETE"),
    }
}

/// `link` lines between every pair of a seed and `viewers` viewers.
fn full_mesh(viewers: usize) -> String {
    let mut t = String::new();
    for a in 0..=viewers {
        for b in a + 1..=viewers {
            t.push_str(&format!("link {a} {b}\n"));
        }
    }
    t
}

/// The farm's testbed mesh for `viewers` viewers.
fn mesh6(viewers: usize) -> &'static str {
    match viewers {
        3 => include_str!("../../../hack/sim-farm/matrices/topologies/mesh6-v3"),
        7 => include_str!("../../../hack/sim-farm/matrices/topologies/mesh6-v7"),
        15 => include_str!("../../../hack/sim-farm/matrices/topologies/mesh6-v15"),
        31 => include_str!("../../../hack/sim-farm/matrices/topologies/mesh6-v31"),
        63 => include_str!("../../../hack/sim-farm/matrices/topologies/mesh6-v63"),
        n => panic!("no mesh6 topology for {n} viewers: 3, 7, 15, 31 or 63"),
    }
}

/// `<a>:<b> ...` from knob `name`.
fn pairs_knob(name: &str) -> Vec<(f64, f64)> {
    knob::<String>(name)
        .unwrap_or_default()
        .split_whitespace()
        .map(|pair| {
            let (a, b) = pair.split_once(':').unwrap();
            (a.parse().unwrap(), b.parse().unwrap())
        })
        .collect()
}

/// A seed and `viewers` viewers, each linked to the seed alone (a star:
/// whatever the seed doesn't push its viewers pull from it, so its load is
/// the viewers times the stream rate), or as the topology says. No jitter
/// or loss; fixed delays; no upload limits unless
/// `STRIMS_SIM_CAL_RATE_LIMIT_PCT` sets them; send buffers.
fn calibration_star_scenario(_seed: u64, c: &Calibration) -> Scenario {
    let mut s = Scenario::new("calibration_star");
    let site = |mbit: u64| Site::new(City::Chicago, Access::Cable).with_up(mbit * 125_000);
    let seed_host = s.host_at("seed", site(c.seed_up_mbit));
    let vs: Vec<HostIx> = (0..c.viewers)
        .map(|i| {
            let mut st = site(c.viewer_up_mbit);
            if i == 0
                && let Some(k) = c.slow_down_kbit
            {
                st = st.with_down(k * 125);
            }
            s.host_at(&format!("v{i}"), st)
        })
        .collect();
    for h in &mut s.hosts {
        h.uplink = c
            .rate_limit_pct
            .and_then(|pct| Some(h.site?.profile.up * pct / 100));
    }
    s.hosts[seed_host].up_steps = c
        .seed_up_steps
        .iter()
        .map(|&(at, mbit)| (Duration::from_secs_f64(5.0 + at), (mbit * 125_000.0) as u64))
        .collect();
    s.hosts[seed_host].competing = c
        .compete
        .iter()
        .map(|&(at, secs)| Competing {
            start: Duration::from_secs_f64(5.0 + at),
            stop: Duration::from_secs_f64(5.0 + at + secs),
            rtt: Duration::from_millis(c.seed_delay_ms + 10),
        })
        .collect();
    s.wan = Some(strims_wan::Options {
        jitter: knob("STRIMS_SIM_CAL_JITTER").unwrap_or(0) == 1,
        loss: false,
    });
    s.send_buffer = Some(SendBuffer {
        uplink_queue: UPLINK_QUEUE,
        window: c.window,
    });
    let all: Vec<HostIx> = std::iter::once(seed_host)
        .chain(vs.iter().copied())
        .collect();
    match &c.topology {
        Some(t) => {
            for line in t.lines() {
                if let ["link", a, b] = line.split_whitespace().collect::<Vec<_>>()[..] {
                    let (a, b): (usize, usize) = (a.parse().unwrap(), b.parse().unwrap());
                    s.link(all[a.max(b)], all[a.min(b)], Transport::Tcp);
                }
            }
        }
        None => {
            for &v in &vs {
                s.link(v, seed_host, Transport::Tcp);
            }
        }
    }
    if vs.len() > 1 {
        s.at(
            Duration::ZERO,
            Fault::Latency(
                vs.clone(),
                vs.clone(),
                Duration::from_millis(c.viewer_delay_ms),
            ),
        );
    }
    // The model's latencies are the same both ways: the seed's paths take
    // the mean of the testbed's two one-way delays.
    s.at(
        Duration::ZERO,
        Fault::Latency(
            vec![seed_host],
            vs.clone(),
            Duration::from_millis((c.seed_delay_ms + c.viewer_delay_ms) / 2),
        ),
    );
    if c.join_at > 0 {
        for &v in &vs {
            s.at(Duration::ZERO, Fault::Crash(v));
            s.at(secs(5 + c.join_at), Fault::Restart(v));
        }
    }
    let mut swarm = SwarmSpec::live(seed_host, vs, WAN_RATE, secs(5), secs(5 + c.secs))
        .with_push_mode(PushMode::Hybrid)
        .with_live_window(WAN_WINDOW);
    if let Some((chunk, munro)) = c.chunks {
        swarm = swarm.with_chunks(chunk, munro).unwrap();
    }
    s.swarm(swarm);
    // The heap grows with the swarm: a 32-node full mesh holds about
    // 450 MiB. Past the bound, the trace names the heap's bytes, which
    // vary between processes, and the farm's reruns would disagree.
    s.invariants = Invariants {
        max_heap: Some((256 + 24 * c.viewers as i64) * MIB),
        ..Invariants::for_window(WAN_WINDOW)
    };
    s.duration = secs(5 + c.secs + 10);
    s.transfer_samples = true;
    striped(s, STRIPES, ADAPTIVE)
}

/// The seed's meter, the upload it had, its push subscribers, its
/// adaptive budget and copies and its true uplink queue per sample, the
/// viewers' p99 lag per second of stream, and every node's transfers per
/// sample (`[s, transfers]` lists by host, the testbed's `transfers`), as
/// one JSON object (`SERIES {json}`).
fn calibration_series(r: &Report) -> String {
    let spec = &r.scenario.swarms[0];
    let seed = spec.seed;
    let up = r.upload_cap(seed).unwrap_or(0);
    let queue: Vec<(Duration, u32)> = r
        .wan
        .as_ref()
        .and_then(|w| w.hosts.iter().find(|h| h.name == r.scenario.name_of(seed)))
        .map(|h| {
            h.up_queue
                .iter()
                .filter_map(|&(t, w)| {
                    Some((
                        t.to_std()?,
                        u32::try_from(w.as_millis().max(0)).unwrap_or(u32::MAX),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let seed_state = |t: Duration| {
        r.hosts[seed]
            .runner
            .seed_changes
            .iter()
            .rev()
            .find(|(at, sw, _)| *sw == 0 && *at <= t)
            .map_or((0, 0), |(_, _, x)| (x.budget, x.copies))
    };
    let mut rows = Vec::new();
    for m in &r.hosts[seed].runner.meter {
        if m.t < spec.start || m.t > spec.stop {
            continue;
        }
        let (budget, copies) = seed_state(m.t);
        let q = queue
            .iter()
            .filter(|(t, _)| t.abs_diff(m.t) <= Duration::from_millis(500))
            .map(|&(_, w)| f64::from(w))
            .collect::<Vec<_>>();
        let q = if q.is_empty() {
            "null".to_owned()
        } else {
            format!("{:.1}", q.iter().sum::<f64>() / q.len() as f64)
        };
        rows.push(format!(
            "{{\"s\":{:.1},\"rate\":{},\"busy\":{},\"capacity\":{},\"avail\":{},\"children\":{},\"budget\":{budget},\"copies\":{copies},\"qd\":{},\"q\":{q}}}",
            (m.t - spec.start).as_secs_f64(),
            m.rate,
            m.busy_permille,
            m.capacity,
            r.upload_cap_at(seed, m.t).unwrap_or(0),
            m.children.get(&0).copied().unwrap_or(0),
            m.queue_delay_ms
                .map_or("null".to_owned(), |d| d.to_string()),
        ));
    }
    let secs_n = spec.stop.saturating_sub(spec.start).as_secs();
    let lags: Vec<String> = (0..secs_n)
        .map(|k| {
            let a = spec.start + Duration::from_secs(k);
            let p = r.lag_between(0, a, a + Duration::from_secs(1));
            format!("[{k},{}]", p.p99)
        })
        .collect();
    let stream_secs = spec.stop.saturating_sub(spec.start).as_secs_f64();
    let waits: Vec<String> = queue
        .iter()
        .filter(|(t, _)| *t >= spec.start && *t <= spec.stop)
        .map(|&(t, w)| format!("[{:.1},{w}]", (t - spec.start).as_secs_f64()))
        .collect();
    let transfers: Vec<String> = r
        .hosts
        .iter()
        .map(|h| {
            let xs: Vec<String> = h
                .transfers
                .iter()
                .filter(|(t, s, _)| *s == 0 && *t >= spec.start && *t <= spec.stop)
                .map(|(t, _, x)| format!("[{:.1},{}]", (*t - spec.start).as_secs_f64(), x.json()))
                .collect();
            format!("[{}]", xs.join(","))
        })
        .collect();
    format!(
        "{{\"up\":{up},\"goodput\":{:.0},\"seed\":[{}],\"lag_p99\":[{}],\"queue\":[{}],\"transfers\":[{}]}}",
        r.hosts[seed].uploaded as f64 / stream_secs,
        rows.join(","),
        lags.join(","),
        waits.join(","),
        transfers.join(",")
    )
}

#[test]
#[ignore = "calibration against hack/testbed; run in release with --include-ignored"]
fn calibration_star() {
    let c = calibration_settings();
    // A measurement: it checks nothing beyond the replay.
    let reports = sweep_n(
        "calibration_star",
        1,
        |s| calibration_star_scenario(s, &c),
        |_| Ok(()),
    );
    // A farm job's row carries the run's seconds (`Report::series`).
    if !knob::<String>("STRIMS_SIM_JOB").is_some_and(|j| j == "1") {
        for r in &reports {
            println!("SERIES {}", calibration_series(r));
        }
    }
}

// 11b. Collapse modes: fixed cases of `calibration_star` in which an
// honest Hybrid swarm stops delivering or lets its lag grow, each with the
// check that its mode is there. A change that removes a mode fails its
// case: the case then becomes that change's target and checks the healthy
// outcome instead. All run 4 KiB chunks and 32 KiB munros (the swarm
// defaults), a full mesh and the window model, with uplinks the nodes
// don't know.

/// The testbed's settings for `viewers` viewers in a full mesh behind a
/// `seed_up_mbit` seed, streaming `secs`, at the swarm defaults' chunks in
/// the window model.
fn collapse_case(viewers: usize, seed_up_mbit: u64, secs: u64) -> Calibration {
    Calibration {
        viewers,
        seed_up_mbit,
        viewer_up_mbit: 20,
        seed_delay_ms: 20,
        viewer_delay_ms: 10,
        secs,
        join_at: 0,
        slow_down_kbit: None,
        topology: Some(full_mesh(viewers)),
        seed_up_steps: Vec::new(),
        rate_limit_pct: None,
        window: true,
        chunks: Some((4096, 32 * 1024)),
        compete: Vec::new(),
    }
}

/// Runs collapse case `c` as test `name` at `seed` and its replay.
fn collapse_sweep(
    name: &str,
    seed: u64,
    c: &Calibration,
    check: impl Fn(&Report) -> Result<(), String>,
) {
    sweep_at(name, seed, |s| calibration_star_scenario(s, c), check);
}

/// The seed's host and its stream's start.
fn collapse_seed(r: &Report) -> (HostIx, Duration) {
    let spec = &r.scenario.swarms[0];
    (spec.seed, spec.start)
}

/// **First-wave overshoot.** The upload meter takes capacity as the bytes
/// written over the share of time some peer had data queued, that share
/// counted as at least `MIN_BUSY` (5 %). The node hands a fresh stream's
/// data straight to its links' buffers and sockets, so at the first wave
/// it reads busy near zero and its capacity at up to 20 times its sending
/// rate. The adaptive seed sizes its first wave from that reading over the
/// store rate since the first chunk, so behind a 10 Mbit uplink that pays
/// for 2 copies it opens a copy to each of 15 neighbours. The pushes queue
/// behind each holder's connection, go stale, and its window falls back
/// over the next seconds while the viewers pull. Without a known capacity
/// the first wave now ramps, as a slow start, from two copies until a
/// queue forms below the runner, and ends on the queue its last doubling
/// built. Checks that the first wave opens at most twice the copies the
/// seed's uplink pays for.
#[test]
#[ignore = "a collapse mode's case; run in release with --include-ignored"]
fn collapse_first_wave_overshoot() {
    collapse_sweep(
        "collapse_first_wave_overshoot",
        1,
        &collapse_case(15, 10, 30),
        |r| {
            let (seed, _) = collapse_seed(r);
            let wave = r.hosts[seed]
                .runner
                .seeds
                .get(&0)
                .map_or(0, |s| s.first_wave);
            let paid = r.paid_copies(0).unwrap_or(0).max(1);
            if wave <= 2 * paid {
                Ok(())
            } else {
                Err(format!(
                    "first wave {wave} copies, the uplink pays for {paid}"
                ))
            }
        },
    );
}

/// **The meter's oscillation.** Past the first wave the same floor keeps
/// misreading: whenever the seed's uplink isn't saturated it reads busy
/// near zero and its capacity at many times the uplink, so its window
/// grows copies past what the uplink pays for, the uplink saturates, the
/// holders' pushes fall behind and the window halves, until the uplink
/// idles again. Behind a 5 Mbit uplink (one copy) three viewers ride this
/// cycle two or three times in 90 s, with lag spikes of a second or two. A
/// cycle is a reading over four times the uplink while busy under 5 %,
/// then one at over 90 % busy. A run's count varies with its seed (0 to
/// 4). Once a queue has stood below the runner, what the links delivered
/// under it bounds the reading, and the window grows only on copies held
/// for a check, so the cycle mostly comes once, as the stream starts. The
/// case checks 8 seeds for under 1.5 cycles a run.
#[test]
#[ignore = "a collapse mode's case; run in release with --include-ignored"]
fn collapse_meter_oscillation() {
    let cycles = |r: &Report| {
        let (seed, start) = collapse_seed(r);
        let (mut cycles, mut overread) = (0u64, false);
        for m in r.hosts[seed]
            .runner
            .meter
            .iter()
            .filter(|m| m.t > start + secs(5))
        {
            let cap = r.upload_cap_at(seed, m.t).unwrap_or(u64::MAX);
            if m.busy_permille < 50 && m.capacity > 4 * cap {
                overread = true;
            } else if overread && m.busy_permille > 900 {
                cycles += 1;
                overread = false;
            }
        }
        cycles
    };
    let reports = sweep_n(
        "collapse_meter_oscillation",
        8,
        |s| calibration_star_scenario(s, &collapse_case(3, 5, 90)),
        |_| Ok(()),
    );
    let runs: Vec<(u64, u64)> = reports.iter().map(|r| (r.seed, cycles(r))).collect();
    let total: u64 = runs.iter().map(|&(_, c)| c).sum();
    assert!(
        2 * total < 3 * runs.len() as u64,
        "{total} cycles of overreading then saturating in {} runs (seed, cycles): {runs:?}",
        runs.len()
    );
}

/// **Pull lock-in.** Viewers without timely pushes pull the newest chunks,
/// and the seed, which announces every chunk first, answers each fresh
/// chunk several times over. Its uplink fills with answers, and on a node
/// with no rate limit its QoS never arbitrates between peers (each peer
/// blocks on its own link), so its holders get only their TCP share of the
/// access link: their pushes wait, go stale or arrive after the children
/// pulled the chunks, the children prune them, and more viewers pull from
/// the seed. Behind a 5 Mbit seed 15 viewers never get out: the seed sends
/// most of its chunks as answers and the lag grows for the whole stream.
/// Bounding the seed's answers per chunk, or a rate limit under the uplink
/// (so the holders' QoS weights apply), lets the trees close, and so does
/// pacing the answers while a queue stands below the runner or pushes wait
/// in it. Checks that the seed sends at least half its chunks as pushes
/// and that the frames written in the stream's last 20 s lag under 2 s at
/// p99.
#[test]
#[ignore = "a collapse mode's case; run in release with --include-ignored"]
fn collapse_pull_lock_in() {
    collapse_sweep("collapse_pull_lock_in", 1, &collapse_case(15, 5, 60), |r| {
        let (seed, start) = collapse_seed(r);
        let (sent, pushed) = r.hosts[seed]
            .transfers
            .iter()
            .rev()
            .find(|(_, s, _)| *s == 0)
            .map_or((0, 0), |(_, _, x)| (x.chunks_sent, x.pushed_chunks));
        let late = r.lag_between(0, start + secs(40), start + secs(60)).p99;
        if pushed * 2 >= sent && late < 2000 {
            Ok(())
        } else {
            Err(format!(
                "pushed {pushed} of {sent} chunks; late frames' p99 {late} ms"
            ))
        }
    });
}

/// **Lock-in after a halving.** A swarm whose trees stand falls into the
/// pull lock-in when its seed's uplink halves: the seed's copies no longer
/// fit, its holders' pushes fall behind, viewers prune it and pull from it,
/// and its queue never drains while the uplink stays halved, though the
/// halved uplink still carries two copies. Behind a 10 Mbit seed halved at
/// 30 s, 7 viewers stream well before and lag seconds after. Checks that
/// frames written in the 10 s before the halving lag under 2 s at p99, and
/// those written from 10 s after it over 2 s: healthy before, not after.
/// Holding answers below pushes, routing by expected delivery, holding
/// late pushes for their parent, pacing answers and frames of about a
/// chunk (little waits below the runner when the uplink halves) ease the
/// mode but don't remove it: once its window is cut, the seed holds two or
/// three copies while the viewers it dropped find relays, and on about a
/// third of seeds that takes long enough to show. At this seed frames lag
/// about 0.2 s at p99 before the halving and 2.8 s after.
#[test]
#[ignore = "a collapse mode's case; run in release with --include-ignored"]
fn collapse_halving_lock_in() {
    let mut c = collapse_case(7, 10, 70);
    c.seed_up_steps = vec![(30.0, 5.0)];
    collapse_sweep("collapse_halving_lock_in", 2, &c, |r| {
        let (_, start) = collapse_seed(r);
        let before = r.lag_between(0, start + secs(20), start + secs(30)).p99;
        let after = r.lag_between(0, start + secs(40), start + secs(70)).p99;
        if before < 2000 && after > 2000 {
            Ok(())
        } else {
            Err(format!(
                "p99 lag {before} ms before the halving, {after} ms after"
            ))
        }
    });
}

// 10d. Mobile viewers: six on LTE and six on cable in North America.

fn wan_mobile_viewers(seed: u64, push: PushMode) -> Scenario {
    let na: Vec<(City, u32)> = GLOBAL_AUDIENCE
        .iter()
        .copied()
        .filter(|(c, _)| c.region() == Region::NorthAmerica)
        .collect();
    let mut sites = viewer_sites(seed, 6, &na, &[(Access::Lte, 1)]);
    sites.extend(viewer_sites(seed + 1, 6, &na, &[(Access::Cable, 1)]));
    let mut m = wan(
        &format!("wan_mobile_viewers {push:?}"),
        seed,
        Site::new(City::NewYork, Access::Datacenter),
        &sites,
        push,
        (5, 65),
    );
    m.s.invariants.max_heap = Some(192 * MIB);
    m.s
}

/// Everyone gets the stream from 10 s on, LTE viewers within 3.5 s at p99,
/// and packets were lost on the way.
fn check_wan_mobile_viewers(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    complete(r, &all_viewers(r), secs(10), spec.stop)?;
    let lte: Vec<HostIx> = all_viewers(r)
        .into_iter()
        .filter(|&h| {
            r.scenario.hosts[h]
                .site
                .is_some_and(|s| s.access == Access::Lte)
        })
        .collect();
    let lag = r.lag_of(0, &lte);
    if lag.p99 > 3500 {
        return Err(format!("LTE viewers' lag {lag}"));
    }
    let (retransmits, _) = r.wan.as_ref().map_or((0, 0), |w| w.losses());
    if retransmits == 0 {
        return Err("no stream message lost a packet".to_owned());
    }
    queues_within(r, secs(2))
}

#[test]
#[ignore = "WAN, 13 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_mobile_viewers_pull() {
    sweep(
        "wan_mobile_viewers_pull",
        |s| wan_mobile_viewers(s, PushMode::Off),
        check_wan_mobile_viewers,
    );
}

#[test]
#[ignore = "WAN, 13 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_mobile_viewers_hybrid() {
    sweep(
        "wan_mobile_viewers_hybrid",
        |s| wan_mobile_viewers(s, PushMode::Hybrid),
        check_wan_mobile_viewers,
    );
}

// 10e. The upload meter's cases without known uplinks: the WAN cases
// above with no upload limits, and send buffers.

#[test]
#[ignore = "WAN, 17 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_global_audience_adaptive_uncapped() {
    sweep(
        "wan_global_audience_adaptive_uncapped",
        |s| {
            uncapped(striped(
                wan_global_audience(s, PushMode::Hybrid),
                STRIPES,
                ADAPTIVE,
            ))
        },
        check_wan_global_audience,
    );
}

#[test]
#[ignore = "WAN, 13 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_mobile_viewers_adaptive_uncapped() {
    sweep(
        "wan_mobile_viewers_adaptive_uncapped",
        |s| {
            uncapped(striped(
                wan_mobile_viewers(s, PushMode::Hybrid),
                STRIPES,
                ADAPTIVE,
            ))
        },
        check_wan_mobile_viewers,
    );
}

/// [`wan_residential_seed`] with a second stream from the same seed to the
/// same viewers: two swarms share every uplink.
fn wan_residential_two_streams(seed: u64) -> Scenario {
    let mut s = wan_residential_seed(seed, PushMode::Hybrid);
    let first = s.swarms[0].clone();
    s.swarm(first);
    s.name = "wan_residential_two_streams".to_owned();
    s
}

/// Everyone gets both streams from 10 s on.
fn check_wan_two_streams(r: &Report) -> Result<(), String> {
    sound(r)?;
    for sw in 0..r.scenario.swarms.len() {
        let spec = &r.scenario.swarms[sw];
        complete_swarm(r, sw, &spec.viewers, secs(10), spec.stop)?;
    }
    Ok(())
}

#[test]
#[ignore = "WAN, 17 hosts at 2 x 2 Mbit/s; run in release with --include-ignored"]
fn wan_residential_two_streams_adaptive_uncapped() {
    sweep(
        "wan_residential_two_streams_adaptive_uncapped",
        |s| uncapped(striped(wan_residential_two_streams(s), STRIPES, ADAPTIVE)),
        check_wan_two_streams,
    );
}

/// [`wan_residential_seed`] with its first viewer behind a 1 Mbit/s
/// downlink, half the stream: it can't keep up, and whoever sends to it
/// finds that one connection backed up.
fn wan_slow_viewer(seed: u64) -> Scenario {
    let mut s = wan_residential_seed(seed, PushMode::Hybrid);
    let v = s.swarms[0].viewers[0];
    if let Some(site) = s.hosts[v].site.as_mut() {
        *site = site.with_down(125_000);
    }
    // Pushes queue toward the slow viewer while its connection is backed
    // up, past one window, up to ppspp's cap; and what it sends about the
    // chunks it still lacks can be behind its neighbours' windows, which
    // they count as invalid.
    s.invariants.max_data_queue = Some(strims_ppspp::DATA_QUEUE_CAP);
    s.invariants.clean_input = false;
    s.name = "wan_slow_viewer".to_owned();
    s
}

/// The viewers other than the slow one get the stream from 10 s on.
fn check_wan_slow_viewer(r: &Report) -> Result<(), String> {
    sound(r)?;
    let spec = &r.scenario.swarms[0];
    complete(r, &spec.viewers[1..], secs(10), spec.stop)
}

#[test]
#[ignore = "WAN, 17 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_slow_viewer_adaptive_uncapped() {
    sweep(
        "wan_slow_viewer_adaptive_uncapped",
        |s| uncapped(striped(wan_slow_viewer(s), STRIPES, ADAPTIVE)),
        check_wan_slow_viewer,
    );
}

// 10f. Upload that comes and goes: other apps' bulk uploads share some
// hosts' uplinks for a while, so a node's upload estimate must follow.

/// A uniform draw from `[lo, hi)` seconds, to the millisecond.
fn uniform_secs(rng: &mut DetRng, lo: u64, hi: u64) -> Duration {
    Duration::from_millis(lo * 1000 + (unit(rng) * ((hi - lo) * 1000) as f64) as u64)
}

/// `m` with the seed and a random third of the viewers sharing their
/// uplinks with another app's bulk upload that comes and goes: from
/// U[10, 50] s into the run, each runs one for U[30, 120] s, to a sink
/// U[20, 80] ms away, then none for U[20, 120] s, and again. The uploads
/// take what TCP's sharing gives them, so they need the window model
/// ([`windowed`]).
fn varying(mut m: Streaming) -> Scenario {
    let mut hosts = vec![m.seed];
    hosts.extend(
        m.viewers
            .iter()
            .copied()
            .filter(|_| unit(&mut m.rng) < 1.0 / 3.0),
    );
    for h in hosts {
        let mut t = uniform_secs(&mut m.rng, 10, 50);
        while t < m.s.duration {
            let stop = t + uniform_secs(&mut m.rng, 30, 120);
            let rtt = Duration::from_millis(20 + (unit(&mut m.rng) * 60.0) as u64);
            m.s.hosts[h].competing.push(Competing {
                start: t,
                stop,
                rtt,
            });
            t = stop + uniform_secs(&mut m.rng, 20, 120);
        }
    }
    m.s.name = format!("{} varying", m.s.name);
    m.s
}

/// `s` with its send buffers' window model.
fn windowed(mut s: Scenario) -> Scenario {
    if let Some(b) = &mut s.send_buffer {
        b.window = true;
    }
    s
}

#[test]
#[ignore = "WAN, 17 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_residential_seed_adaptive_uncapped_varying() {
    sweep(
        "wan_residential_seed_adaptive_uncapped_varying",
        |s| {
            windowed(uncapped(striped(
                varying(residential_seed_mesh(s, PushMode::Hybrid, (5, 65))),
                STRIPES,
                ADAPTIVE,
            )))
        },
        check_wan_residential_seed,
    );
}

#[test]
#[ignore = "WAN, 18 hosts at 2 Mbit/s; run in release with --include-ignored"]
fn wan_super_peer_adaptive_uncapped_varying() {
    sweep(
        "wan_super_peer_adaptive_uncapped_varying",
        |s| {
            windowed(uncapped(striped(
                varying(super_peer_mesh(s, PushMode::Hybrid)),
                STRIPES,
                ADAPTIVE,
            )))
        },
        check_wan_residential_seed,
    );
}

#[test]
#[ignore = "WAN, 17 hosts at 2 Mbit/s for minutes; run in release with --include-ignored"]
fn wan_residential_seed_adaptive_uncapped_long_varying() {
    let stop = 5 + 60 * stream_minutes(LONG_MINUTES);
    sweep_n(
        "wan_residential_seed_adaptive_uncapped_long_varying",
        LONG_SEEDS,
        |s| {
            windowed(uncapped(striped(
                varying(residential_seed_mesh(s, PushMode::Hybrid, (5, stop))),
                STRIPES,
                ADAPTIVE,
            )))
        },
        check_wan_long,
    );
}

/// [`wan_residential_seed`] streaming for 4 minutes, the seed's uplink
/// halved from 60 s to 180 s.
fn wan_residential_seed_step(seed: u64) -> Scenario {
    let mut m = residential_seed_mesh(seed, PushMode::Hybrid, (5, 245));
    let seed_host = &mut m.s.hosts[m.seed];
    let up = seed_host.site.map_or(0, |s| s.profile.up);
    seed_host.up_steps = vec![(secs(60), up / 2), (secs(180), up)];
    m.s.name = format!("{} step", m.s.name);
    m.s
}

#[test]
#[ignore = "WAN, 17 hosts at 2 Mbit/s for minutes; run in release with --include-ignored"]
fn wan_residential_seed_adaptive_uncapped_step() {
    sweep(
        "wan_residential_seed_adaptive_uncapped_step",
        |s| uncapped(striped(wan_residential_seed_step(s), STRIPES, ADAPTIVE)),
        check_wan_long,
    );
}

// --- sampling -----------------------------------------------------------------

/// A seed and 3 viewers in a chain, Hybrid, uplinks well above the stream.
fn uncontended(stats_every: Option<Duration>) -> Scenario {
    let mut s = Scenario::new("uncontended");
    let seed = s.host("seed", Some(1024 * KIB));
    let vs = s.hosts("v", 3, Some(1024 * KIB));
    for (i, &v) in vs.iter().enumerate() {
        s.link(v, if i == 0 { seed } else { vs[i - 1] }, Transport::Tcp);
    }
    s.swarm(SwarmSpec::live(seed, vs, RATE, secs(3), secs(20)).with_push_mode(PushMode::Hybrid));
    s.duration = secs(24);
    s.stats_every = stats_every;
    s
}

/// Stats samples log nothing and are answered after passes the nodes make
/// anyway, so a busy run's trace (which logs only the `sample_every`
/// samples) is the same with and without them. A sample that wakes an idle
/// node can shift later sample stamps, which is why `stats_every` is part
/// of the scenario.
#[test]
fn stats_sampling_is_passive_without_contention() {
    let a = strims_sim::run(uncontended(None), 1).unwrap();
    let b = strims_sim::run(uncontended(Some(secs(1))), 1).unwrap();
    assert_eq!(a.trace_hash, b.trace_hash);
    assert_eq!(a.delivery(0), 1.0);
    // The stats samples saw the push tree.
    assert!(b.swarm_counters(0).hits > 0);
    assert!(b.peaks().cwnd_max > 0);
}

/// Transfer samples change no trace, and say who serves whom: in the
/// chain, the seed serves the first viewer alone, which has it as a parent.
#[test]
fn transfer_samples_name_who_serves_whom() {
    let a = strims_sim::run(uncontended(Some(secs(1))), 1).unwrap();
    let mut s = uncontended(Some(secs(1)));
    s.transfer_samples = true;
    let b = strims_sim::run(s, 1).unwrap();
    assert_eq!(a.trace_hash, b.trace_hash);
    let at = |h: usize| {
        let (_, _, x) = b.hosts[h]
            .transfers
            .iter()
            .find(|(t, ..)| *t >= secs(15))
            .unwrap();
        x
    };
    let seed = at(0);
    assert_eq!(
        seed.peers.iter().map(|p| p.peer).collect::<Vec<_>>(),
        [Some(1)]
    );
    assert!(seed.peers[0].sent > 0 && seed.pushed_chunks > 0);
    let v0 = at(1);
    assert!(v0.parents > 0);
    let to_seed = v0.peers.iter().find(|p| p.peer == Some(0)).unwrap();
    assert!(to_seed.received > 0 && to_seed.cwnd > 0 && to_seed.rtt_ms > 0);
}

// --- transports ---------------------------------------------------------------

/// The mesh builder mixes transports; make sure both kinds carry streams.
#[test]
fn meshes_mix_transports() {
    let s = live_mesh(1, PushMode::Off);
    let ws = s
        .links
        .iter()
        .filter(|l| l.transport == Transport::Ws)
        .count();
    assert!(ws > 0 && ws < s.links.len(), "{ws} of {}", s.links.len());
}
