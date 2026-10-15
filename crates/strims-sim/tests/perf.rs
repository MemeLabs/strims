// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Where idle simulation time goes: turmoil's per-host ticks, node loops
//! woken by the ppspp runner's 100 ms tick, and swarm channels with no
//! data. Prints wall time per simulated second and node wakeups; asserts
//! only determinism.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::time::Duration;

use strims_base::rng::det_rng;
use strims_sim::topology::random_mesh;
use strims_sim::{Scenario, SwarmSpec, run};

#[derive(Clone, Copy, Debug)]
enum Idle {
    /// 13 nodes, no links.
    Alone,
    /// 13 nodes in a mesh, nothing running.
    Linked,
    /// 13 nodes in a mesh running a swarm nobody writes to.
    Swarm,
}

fn idle(kind: Idle, tick_ms: u64) -> Scenario {
    let mut s = Scenario::new(&format!("idle {kind:?} tick {tick_ms} ms"));
    let hosts = s.hosts("n", 13, Some(1 << 20));
    if !matches!(kind, Idle::Alone) {
        random_mesh(&mut s, &mut det_rng(1), &hosts, 3, 0.4);
    }
    if matches!(kind, Idle::Swarm) {
        let far = Duration::from_secs(3600);
        s.swarm(SwarmSpec::live(
            hosts[0],
            hosts[1..].to_vec(),
            32 * 1024,
            far,
            far,
        ));
    }
    s.duration = Duration::from_secs(30);
    s.tick = Duration::from_millis(tick_ms);
    s
}

#[test]
fn idle_cost() {
    for (kind, tick) in [
        (Idle::Alone, 1),
        (Idle::Alone, 10),
        (Idle::Linked, 1),
        (Idle::Swarm, 1),
    ] {
        let a = run(idle(kind, tick), 1).unwrap();
        let wakeups: u64 = a.hosts.iter().map(|h| h.loop_iterations).sum();
        eprintln!(
            "{}: {:.1} ms wall per sim second ({:.1}x), {:.1} node wakeups per host per sim second",
            a.scenario.name,
            a.wall_secs * 1000.0 / a.sim_secs,
            a.speed(),
            wakeups as f64 / 13.0 / a.sim_secs,
        );
        let b = run(idle(kind, tick), 1).unwrap();
        assert_eq!(a.trace_hash, b.trace_hash, "{kind:?}");
    }
}
