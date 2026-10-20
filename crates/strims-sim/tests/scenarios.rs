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

use strims_base::rng::det_rng;
use strims_ppspp::{
    DeliveryMode, PushMode, RequestControllerKind, SeedAllocation, SeedPolicy, SeedQos,
};
use strims_sim::topology::random_mesh;
use strims_sim::{
    Binding, Fault, HostIx, Invariants, NetworkSpec, Report, Scenario, SwarmSpec, Transport,
    VpnProbes, sweep,
};
use strims_vpn::Routing;
use strims_wan::sample::pick;
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
/// `stop`.
struct Streaming {
    s: Scenario,
    seed: HostIx,
    viewers: Vec<HostIx>,
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
    }
}

const LEDBAT: RequestControllerKind = RequestControllerKind::Ledbat;
const BBR: RequestControllerKind = RequestControllerKind::BbrLike;

/// The stream count of the striped WAN cases.
const STRIPES: usize = 16;

/// An adaptive seed with uplink shares by allocation.
const ADAPTIVE: SeedPolicy = SeedPolicy {
    allocation: SeedAllocation::Adaptive,
    qos: SeedQos::ByAllocation,
};

/// `s` with every swarm in `streams` streams and its seed on `policy`.
fn striped(mut s: Scenario, streams: usize, policy: SeedPolicy) -> Scenario {
    for sw in &mut s.swarms {
        sw.options.stream_count = streams;
        sw.options.seed = policy;
    }
    let adaptive = if policy == SeedPolicy::default() {
        ""
    } else {
        " adaptive"
    };
    s.name = format!("{} x{streams}{adaptive}", s.name);
    s
}

/// `s` with every swarm running request controller `kind`.
fn with_controller(mut s: Scenario, kind: RequestControllerKind) -> Scenario {
    for sw in &mut s.swarms {
        sw.options.request_controller = kind;
    }
    s.name = format!("{} {kind:?}", s.name);
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

#[test]
fn live_mesh_pull_bbr() {
    sweep(
        "live_mesh_pull_bbr",
        |s| with_controller(live_mesh(s, PushMode::Off), BBR),
        check_live_mesh,
    );
}

#[test]
fn live_mesh_hybrid_bbr() {
    sweep(
        "live_mesh_hybrid_bbr",
        |s| with_controller(live_mesh(s, PushMode::Hybrid), BBR),
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
    let min_uploaders = match r.scenario.swarms[0].options.push_mode {
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

#[test]
fn capped_seed_pull_bbr() {
    sweep(
        "capped_seed_pull_bbr",
        |s| with_controller(capped_seed(s, PushMode::Off), BBR),
        check_capped_seed,
    );
}

#[test]
fn capped_seed_hybrid_bbr() {
    sweep(
        "capped_seed_hybrid_bbr",
        |s| with_controller(capped_seed(s, PushMode::Hybrid), BBR),
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

#[test]
fn latency_step_pull_bbr() {
    sweep(
        "latency_step_pull_bbr",
        |s| with_controller(latency_step(s, PushMode::Off, false), BBR),
        check_latency_step,
    );
}

#[test]
fn latency_step_hybrid_bbr() {
    sweep(
        "latency_step_hybrid_bbr",
        |s| with_controller(latency_step(s, PushMode::Hybrid, false), BBR),
        check_latency_step,
    );
}

/// After `at`, how long frames still arrived with more than `threshold_ms`
/// of lag: the write time of the last such frame, minus `at` (zero if none
/// did).
fn settled_after(r: &Report, sw: usize, at: Duration, threshold_ms: u32) -> Duration {
    let spec = &r.scenario.swarms[sw];
    let first = spec.frames_between(Duration::ZERO, at).end;
    let fpw = spec.frames_per_write as u64;
    r.viewers
        .iter()
        .filter(|v| v.swarm == sw)
        .flat_map(|v| v.stats.lags.iter())
        .filter(|&&(seq, lag)| seq >= first && lag > threshold_ms)
        .map(|&(seq, _)| (spec.start + spec.write_interval * (seq / fpw) as u32).saturating_sub(at))
        .max()
        .unwrap_or_default()
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

#[test]
fn seed_starved_pull_bbr() {
    sweep(
        "seed_starved_pull_bbr",
        |s| with_controller(seed_starved(s, PushMode::Off), BBR),
        check_seed_starved,
    );
}

#[test]
fn seed_starved_hybrid_bbr() {
    sweep(
        "seed_starved_hybrid_bbr",
        |s| with_controller(seed_starved(s, PushMode::Hybrid), BBR),
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
/// the stream rate on average plus overhead. Swarm 0 runs request
/// controller `a`, swarm 1 `b`.
fn cross_traffic(
    seed: u64,
    push: PushMode,
    a: RequestControllerKind,
    b: RequestControllerKind,
) -> Scenario {
    let mut s = Scenario::new(&format!("cross_traffic {push:?} {a:?}/{b:?}"));
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
    for (seed_host, viewers, kind) in [(seed_a, viewers_a, a), (seed_b, viewers_b, b)] {
        s.swarm(
            SwarmSpec::live(seed_host, viewers, RATE, secs(3), secs(43))
                .with_push_mode(push)
                .with_controller(kind),
        );
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
        |s| cross_traffic(s, PushMode::Off, LEDBAT, LEDBAT),
        check_cross_traffic,
    );
}

#[test]
fn cross_traffic_hybrid() {
    sweep(
        "cross_traffic_hybrid",
        |s| cross_traffic(s, PushMode::Hybrid, LEDBAT, LEDBAT),
        check_cross_traffic,
    );
}

#[test]
fn cross_traffic_pull_bbr() {
    sweep(
        "cross_traffic_pull_bbr",
        |s| cross_traffic(s, PushMode::Off, BBR, BBR),
        check_cross_traffic,
    );
}

#[test]
fn cross_traffic_hybrid_bbr() {
    sweep(
        "cross_traffic_hybrid_bbr",
        |s| cross_traffic(s, PushMode::Hybrid, BBR, BBR),
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
    // first five (after the first minute's warm-up), give or take 10 %.
    let m = r.memory.as_ref().ok_or("no memory report")?;
    let per_minute: Vec<String> = (0..10)
        .map(|i| format!("{}", m.max_between(i * 60_000, (i + 1) * 60_000) / 1024))
        .collect();
    eprintln!("long_run heap per minute (KiB): {}", per_minute.join(" "));
    let early = m.max_between(60_000, 300_000);
    let late = m.max_between(300_000, 600_000);
    if late > early + early / 10 {
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
    let sites = viewer_sites(seed, 16, &atlantic(), RESIDENTIAL);
    wan(
        &format!("wan_residential_seed {push:?}"),
        seed,
        Site::new(City::Chicago, Access::Cable),
        &sites,
        push,
        (5, 65),
    )
    .s
}

/// [`wan_residential_seed`]'s viewers plus a 17th in a New York
/// datacenter: a super-peer whose uplink could carry the whole swarm.
fn wan_super_peer(seed: u64, push: PushMode) -> Scenario {
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
    .s
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

// --- request controller comparison --------------------------------------------

/// What `controller_ab` measures of one run.
#[derive(Clone, Debug)]
struct AbRun {
    seed: u64,
    passed: bool,
    delivery: [f64; 2],
    p50: [u32; 2],
    p99: [u32; 2],
    duplicates: f64,
    seed_share: f64,
    /// The seed's distinct chunks over the chunks it sent.
    seed_distinct: f64,
    settle: Duration,
    swarms: usize,
    /// The highest p99 lag of a region's viewers, and the region (WAN
    /// runs).
    worst_region: Option<(u32, Region)>,
}

fn ab_measure(r: &Report, check: Check, step: bool) -> AbRun {
    let swarms = r.scenario.swarms.len().min(2);
    let mut run = AbRun {
        seed: r.seed,
        passed: check(r).is_ok(),
        delivery: [0.0; 2],
        p50: [0; 2],
        p99: [0; 2],
        duplicates: r.duplicates(0),
        seed_share: r.data_share(0),
        seed_distinct: r.seed_distinct_ratio(0),
        settle: if step {
            settled_after(r, 0, secs(STEP_AT), 1000)
        } else {
            Duration::ZERO
        },
        swarms,
        worst_region: r.wan.as_ref().and_then(|_| {
            r.lag_by_region(0)
                .into_iter()
                .map(|(g, p)| (p.p99, g))
                .max()
        }),
    };
    for sw in 0..swarms {
        let lag = r.lag(sw);
        run.delivery[sw] = r.delivery(sw);
        run.p50[sw] = lag.p50;
        run.p99[sw] = lag.p99;
    }
    run
}

fn median_of<T: Copy + Ord + Default>(mut v: Vec<T>) -> T {
    v.sort_unstable();
    v.get(v.len() / 2).copied().unwrap_or_default()
}

fn ab_row(name: &str, runs: &[AbRun]) -> String {
    let n = runs.len().max(1) as f64;
    let sw = |i: usize| {
        let d: Vec<f64> = runs.iter().map(|r| r.delivery[i]).collect();
        let p50 = median_of(runs.iter().map(|r| r.p50[i]).collect());
        let p99: Vec<u32> = runs.iter().map(|r| r.p99[i]).collect();
        format!(
            "{:.4} ({:.4}) | {p50} | {} ({})",
            d.iter().sum::<f64>() / n,
            d.iter().copied().fold(1.0, f64::min),
            median_of(p99.clone()),
            p99.iter().max().unwrap_or(&0)
        )
    };
    let settle: Vec<u128> = runs.iter().map(|r| r.settle.as_millis()).collect();
    let failed: Vec<u64> = runs.iter().filter(|r| !r.passed).map(|r| r.seed).collect();
    let mut row = format!(
        "| {name} | {} | {:.2}% | {:.0}% | {:.2} | {} ({}) | {failed:?} |",
        sw(0),
        100.0 * runs.iter().map(|r| r.duplicates).sum::<f64>() / n,
        100.0 * runs.iter().map(|r| r.seed_share).sum::<f64>() / n,
        runs.iter().map(|r| r.seed_distinct).sum::<f64>() / n,
        median_of(settle.clone()),
        settle.iter().max().unwrap_or(&0),
    );
    let worst: Vec<(u32, Region)> = runs.iter().filter_map(|r| r.worst_region).collect();
    if let Some(&(max, region)) = worst.iter().max() {
        row += &format!(
            " worst region p99 {} ({max} {region}) |",
            median_of(worst.iter().map(|w| w.0).collect())
        );
    }
    if runs.iter().any(|r| r.swarms > 1) {
        // Jain's index over the two swarms' delivery, and over their
        // inverse p99 lag: 1 when equal, 0.5 when one swarm gets it all.
        let jain = |x: f64, y: f64| {
            let d = 2.0 * (x * x + y * y);
            if d == 0.0 { 1.0 } else { (x + y) * (x + y) / d }
        };
        let fair_delivery = runs
            .iter()
            .map(|r| jain(r.delivery[0], r.delivery[1]))
            .sum::<f64>()
            / n;
        let fair_lag = runs
            .iter()
            .map(|r| {
                jain(
                    1.0 / f64::from(r.p99[0].max(1)),
                    1.0 / f64::from(r.p99[1].max(1)),
                )
            })
            .sum::<f64>()
            / n;
        row += &format!(
            " swarm 1: {} | fairness {fair_delivery:.4} / {fair_lag:.3} |",
            sw(1)
        );
    }
    row
}

type Build = fn(u64) -> Scenario;
type Check = fn(&Report) -> Result<(), String>;

/// The request controllers compared over the same scenarios, seeds and
/// traces, in pull and Hybrid. Prints one row per scenario, mode and
/// controller: delivery mean (min), lag p50 (median) and p99 (median,
/// max), duplicates, the seed's data share, the latency step's settling
/// time (median, max), the seeds that failed the scenario's check, for
/// cross traffic the second swarm and the fairness between the two, and
/// for WAN runs the p99 lag of the worst region (median, max).
#[test]
#[ignore = "the request controller comparison; run in release with --ignored --nocapture"]
fn controller_ab() {
    let cases: Vec<(&str, Build, Check, bool)> = vec![
        (
            "live_mesh pull ledbat",
            |s| with_controller(live_mesh(s, PushMode::Off), LEDBAT),
            check_live_mesh,
            false,
        ),
        (
            "live_mesh pull bbr",
            |s| with_controller(live_mesh(s, PushMode::Off), BBR),
            check_live_mesh,
            false,
        ),
        (
            "live_mesh hybrid ledbat",
            |s| with_controller(live_mesh(s, PushMode::Hybrid), LEDBAT),
            check_live_mesh,
            false,
        ),
        (
            "live_mesh hybrid bbr",
            |s| with_controller(live_mesh(s, PushMode::Hybrid), BBR),
            check_live_mesh,
            false,
        ),
        (
            "capped_seed pull ledbat",
            |s| with_controller(capped_seed(s, PushMode::Off), LEDBAT),
            check_capped_seed,
            false,
        ),
        (
            "capped_seed pull bbr",
            |s| with_controller(capped_seed(s, PushMode::Off), BBR),
            check_capped_seed,
            false,
        ),
        (
            "capped_seed hybrid ledbat",
            |s| with_controller(capped_seed(s, PushMode::Hybrid), LEDBAT),
            check_capped_seed,
            false,
        ),
        (
            "capped_seed hybrid bbr",
            |s| with_controller(capped_seed(s, PushMode::Hybrid), BBR),
            check_capped_seed,
            false,
        ),
        (
            "latency_step pull ledbat",
            |s| with_controller(latency_step(s, PushMode::Off, false), LEDBAT),
            check_latency_step,
            true,
        ),
        (
            "latency_step pull bbr",
            |s| with_controller(latency_step(s, PushMode::Off, false), BBR),
            check_latency_step,
            true,
        ),
        (
            "latency_step hybrid ledbat",
            |s| with_controller(latency_step(s, PushMode::Hybrid, false), LEDBAT),
            check_latency_step,
            true,
        ),
        (
            "latency_step hybrid bbr",
            |s| with_controller(latency_step(s, PushMode::Hybrid, false), BBR),
            check_latency_step,
            true,
        ),
        (
            "seed_starved pull ledbat",
            |s| with_controller(seed_starved(s, PushMode::Off), LEDBAT),
            check_seed_starved,
            false,
        ),
        (
            "seed_starved pull bbr",
            |s| with_controller(seed_starved(s, PushMode::Off), BBR),
            check_seed_starved,
            false,
        ),
        (
            "seed_starved hybrid ledbat",
            |s| with_controller(seed_starved(s, PushMode::Hybrid), LEDBAT),
            check_seed_starved,
            false,
        ),
        (
            "seed_starved hybrid bbr",
            |s| with_controller(seed_starved(s, PushMode::Hybrid), BBR),
            check_seed_starved,
            false,
        ),
        (
            "cross_traffic pull ledbat/ledbat",
            |s| cross_traffic(s, PushMode::Off, LEDBAT, LEDBAT),
            check_cross_traffic,
            false,
        ),
        (
            "cross_traffic pull bbr/bbr",
            |s| cross_traffic(s, PushMode::Off, BBR, BBR),
            check_cross_traffic,
            false,
        ),
        (
            "cross_traffic pull ledbat/bbr",
            |s| cross_traffic(s, PushMode::Off, LEDBAT, BBR),
            check_cross_traffic,
            false,
        ),
        (
            "cross_traffic hybrid ledbat/ledbat",
            |s| cross_traffic(s, PushMode::Hybrid, LEDBAT, LEDBAT),
            check_cross_traffic,
            false,
        ),
        (
            "cross_traffic hybrid bbr/bbr",
            |s| cross_traffic(s, PushMode::Hybrid, BBR, BBR),
            check_cross_traffic,
            false,
        ),
        (
            "cross_traffic hybrid ledbat/bbr",
            |s| cross_traffic(s, PushMode::Hybrid, LEDBAT, BBR),
            check_cross_traffic,
            false,
        ),
        (
            "wan_global_audience pull ledbat",
            |s| with_controller(wan_global_audience(s, PushMode::Off), LEDBAT),
            check_wan_global_audience,
            false,
        ),
        (
            "wan_global_audience pull bbr",
            |s| with_controller(wan_global_audience(s, PushMode::Off), BBR),
            check_wan_global_audience,
            false,
        ),
        (
            "wan_global_audience hybrid ledbat",
            |s| with_controller(wan_global_audience(s, PushMode::Hybrid), LEDBAT),
            check_wan_global_audience,
            false,
        ),
        (
            "wan_global_audience hybrid bbr",
            |s| with_controller(wan_global_audience(s, PushMode::Hybrid), BBR),
            check_wan_global_audience,
            false,
        ),
        (
            "wan_residential_seed pull ledbat",
            |s| with_controller(wan_residential_seed(s, PushMode::Off), LEDBAT),
            check_wan_residential_seed,
            false,
        ),
        (
            "wan_residential_seed pull bbr",
            |s| with_controller(wan_residential_seed(s, PushMode::Off), BBR),
            check_wan_residential_seed,
            false,
        ),
        (
            "wan_residential_seed hybrid ledbat",
            |s| with_controller(wan_residential_seed(s, PushMode::Hybrid), LEDBAT),
            check_wan_residential_seed,
            false,
        ),
        (
            "wan_residential_seed hybrid bbr",
            |s| with_controller(wan_residential_seed(s, PushMode::Hybrid), BBR),
            check_wan_residential_seed,
            false,
        ),
        (
            "wan_residential_seed hybrid x16 ledbat",
            |s| {
                let s = wan_residential_seed(s, PushMode::Hybrid);
                with_controller(striped(s, STRIPES, SeedPolicy::default()), LEDBAT)
            },
            check_wan_residential_seed,
            false,
        ),
        (
            "wan_residential_seed hybrid x16 bbr",
            |s| {
                let s = wan_residential_seed(s, PushMode::Hybrid);
                with_controller(striped(s, STRIPES, SeedPolicy::default()), BBR)
            },
            check_wan_residential_seed,
            false,
        ),
        (
            "wan_residential_seed hybrid x16 adaptive ledbat",
            |s| {
                let s = wan_residential_seed(s, PushMode::Hybrid);
                with_controller(striped(s, STRIPES, ADAPTIVE), LEDBAT)
            },
            check_wan_residential_seed,
            false,
        ),
        (
            "wan_residential_seed hybrid x16 adaptive bbr",
            |s| {
                let s = wan_residential_seed(s, PushMode::Hybrid);
                with_controller(striped(s, STRIPES, ADAPTIVE), BBR)
            },
            check_wan_residential_seed,
            false,
        ),
        (
            "wan_super_peer hybrid x16 ledbat",
            |s| {
                let s = wan_super_peer(s, PushMode::Hybrid);
                with_controller(striped(s, STRIPES, SeedPolicy::default()), LEDBAT)
            },
            check_wan_residential_seed,
            false,
        ),
        (
            "wan_super_peer hybrid x16 bbr",
            |s| {
                let s = wan_super_peer(s, PushMode::Hybrid);
                with_controller(striped(s, STRIPES, SeedPolicy::default()), BBR)
            },
            check_wan_residential_seed,
            false,
        ),
        (
            "wan_super_peer hybrid x16 adaptive ledbat",
            |s| {
                let s = wan_super_peer(s, PushMode::Hybrid);
                with_controller(striped(s, STRIPES, ADAPTIVE), LEDBAT)
            },
            check_wan_residential_seed,
            false,
        ),
        (
            "wan_super_peer hybrid x16 adaptive bbr",
            |s| {
                let s = wan_super_peer(s, PushMode::Hybrid);
                with_controller(striped(s, STRIPES, ADAPTIVE), BBR)
            },
            check_wan_residential_seed,
            false,
        ),
        (
            "wan_mobile_viewers pull ledbat",
            |s| with_controller(wan_mobile_viewers(s, PushMode::Off), LEDBAT),
            check_wan_mobile_viewers,
            false,
        ),
        (
            "wan_mobile_viewers pull bbr",
            |s| with_controller(wan_mobile_viewers(s, PushMode::Off), BBR),
            check_wan_mobile_viewers,
            false,
        ),
    ];
    let only = ab_filter();
    let cases: Vec<_> = cases
        .into_iter()
        .filter(|c| only.as_ref().is_none_or(|o| c.0.contains(o.as_str())))
        .collect();
    let seeds = strims_sim::seeds();
    let jobs: Vec<(usize, u64)> = (0..cases.len())
        .flat_map(|c| seeds.iter().map(move |&s| (c, s)))
        .collect();
    // Independent runs, spread over threads; each run stays on its thread
    // and is deterministic.
    let (tx, rx) = std::sync::mpsc::channel();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let tx = tx.clone();
            let (jobs, cases, next) = (&jobs, &cases, &next);
            scope.spawn(move || {
                loop {
                    let j = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(&(c, seed)) = jobs.get(j) else { break };
                    let (name, build, check, step) = cases[c];
                    let r = strims_sim::run(build(seed), seed).unwrap();
                    let m = ab_measure(&r, check, step);
                    eprintln!("{name} seed {seed}: {m:?}");
                    tx.send((c, m)).unwrap();
                }
            });
        }
    });
    drop(tx);
    let mut results = vec![Vec::new(); cases.len()];
    for (c, m) in rx {
        results[c].push(m);
    }
    println!(
        "| scenario, mode, controller | delivery mean (min) | lag p50 ms | p99 ms median (max) | duplicates | seed data share | seed distinct | settle ms median (max) | failed seeds |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    for (c, case) in cases.iter().enumerate() {
        results[c].sort_by_key(|r| r.seed);
        println!("{}", ab_row(case.0, &results[c]));
    }
}

/// `STRIMS_SIM_AB=<substring>`: only the `controller_ab` cases whose name
/// contains it.
#[allow(clippy::disallowed_methods)]
fn ab_filter() -> Option<String> {
    std::env::var("STRIMS_SIM_AB").ok()
}
