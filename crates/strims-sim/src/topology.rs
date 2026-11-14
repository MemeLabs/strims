// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Seeded link layouts.

use rand_core::Rng;
use strims_base::rng::DetRng;

use crate::spec::{HostIx, Scenario, Transport};

/// Links each of `hosts[1..]` to up to `degree` distinct hosts that come
/// before it (the first always to `hosts[0]`'s side of the graph), so the
/// graph is connected and every host has at least one path to `hosts[0]`.
/// Each link is WebSocket with probability `ws_share`, else TCP. With
/// arrivals ([`Scenario::arrivals`]) the run lays the links out in arrival
/// order ([`crate::arrivals`]), so "before" is "arrived earlier": each
/// viewer dials hosts already there.
pub fn random_mesh(
    s: &mut Scenario,
    rng: &mut DetRng,
    hosts: &[HostIx],
    degree: usize,
    ws_share: f64,
) {
    for (i, &h) in hosts.iter().enumerate().skip(1) {
        let mut targets: Vec<HostIx> = Vec::new();
        let want = degree.min(i);
        while targets.len() < want {
            let j = (rng.next_u64() % i as u64) as usize;
            if let Some(&t) = hosts.get(j)
                && !targets.contains(&t)
            {
                targets.push(t);
            }
        }
        for t in targets {
            s.link(h, t, transport(rng, ws_share));
        }
    }
}

/// Links each of `hosts` to `degree` distinct others drawn from `rng`
/// (every other host when there are no more), each pair once: a host has
/// the `degree` links it drew and about as many drawn by others, and the
/// draws cost `hosts.len() * degree`, whatever the size. The later host of
/// a pair in `hosts` dials the earlier, as in [`random_mesh`], so with
/// arrivals each viewer dials hosts already there. Each link is WebSocket
/// with probability `ws_share`, else TCP.
pub fn random_graph(
    s: &mut Scenario,
    rng: &mut DetRng,
    hosts: &[HostIx],
    degree: usize,
    ws_share: f64,
) {
    let mut linked = std::collections::BTreeSet::new();
    for i in 0..hosts.len() {
        for j in distinct(rng, hosts.len(), degree, Some(i)) {
            let pair = (i.max(j), i.min(j));
            if linked.insert(pair)
                && let (Some(&from), Some(&to)) = (hosts.get(pair.0), hosts.get(pair.1))
            {
                s.link(from, to, transport(rng, ws_share));
            }
        }
    }
}

/// Links `hub` to `n` distinct hosts of `hosts` drawn from `rng` (all of
/// them when there are no more), each dialing the hub. Each link is
/// WebSocket with probability `ws_share`, else TCP.
pub fn random_hub(
    s: &mut Scenario,
    rng: &mut DetRng,
    hub: HostIx,
    hosts: &[HostIx],
    n: usize,
    ws_share: f64,
) {
    for j in distinct(rng, hosts.len(), n, None) {
        if let Some(&h) = hosts.get(j) {
            s.link(h, hub, transport(rng, ws_share));
        }
    }
}

/// `want` distinct indices below `n`, never `skip`, in the order drawn:
/// every index there is, in order, when that's no more than `want`.
fn distinct(rng: &mut DetRng, n: usize, want: usize, skip: Option<usize>) -> Vec<usize> {
    let others = n - usize::from(skip.is_some_and(|k| k < n));
    if others <= want {
        return (0..n).filter(|&j| Some(j) != skip).collect();
    }
    let mut out: Vec<usize> = Vec::with_capacity(want);
    while out.len() < want {
        let j = (rng.next_u64() % n as u64) as usize;
        if Some(j) != skip && !out.contains(&j) {
            out.push(j);
        }
    }
    out
}

/// A transport drawn from `rng`: WebSocket with probability `ws_share`.
pub fn transport(rng: &mut DetRng, ws_share: f64) -> Transport {
    let x = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
    if x < ws_share {
        Transport::Ws
    } else {
        Transport::Tcp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strims_base::rng::det_rng;

    #[test]
    fn meshes_are_connected_and_seeded() {
        let build = |seed| {
            let mut s = Scenario::new("t");
            let hosts = s.hosts("h", 10, None);
            random_mesh(&mut s, &mut det_rng(seed), &hosts, 3, 0.5);
            s.links
        };
        let a = build(1);
        assert_eq!(a, build(1));
        assert_ne!(a, build(2));
        // Every host but the first dials min(i, 3) earlier hosts.
        assert_eq!(a.len(), 1 + 2 + 3 * 7);
        assert!(a.iter().all(|l| l.to < l.from));
    }

    #[test]
    fn random_graphs_draw_each_hosts_degree_once_a_pair() {
        let build = |seed, n| {
            let mut s = Scenario::new("t");
            let hub = s.host("hub", None);
            let hosts = s.hosts("h", n, None);
            let mut rng = det_rng(seed);
            random_hub(&mut s, &mut rng, hub, &hosts, 32, 0.0);
            random_graph(&mut s, &mut rng, &hosts, 6, 0.0);
            s.links
        };
        let a = build(1, 500);
        assert_eq!(a, build(1, 500));
        assert_ne!(a, build(2, 500));
        assert_eq!(a.iter().filter(|l| l.to == 0).count(), 32);
        let mesh: Vec<_> = a.iter().filter(|l| l.to != 0).collect();
        let mut pairs: Vec<_> = mesh.iter().map(|l| (l.from, l.to)).collect();
        pairs.sort_unstable();
        pairs.dedup();
        assert_eq!(pairs.len(), mesh.len());
        assert!(mesh.iter().all(|l| l.to < l.from));
        // Each host's 6 draws, less the pairs drawn from both ends.
        assert!((500 * 6 * 9 / 10..=500 * 6).contains(&mesh.len()));
        for h in 1..=500 {
            let links = mesh.iter().filter(|l| l.from == h || l.to == h).count();
            assert!(links >= 6, "host {h} has {links} links");
        }
        // Too few hosts for the draws: every pair, and every host at the hub.
        let small = build(1, 5);
        assert_eq!(small.iter().filter(|l| l.to == 0).count(), 5);
        assert_eq!(small.iter().filter(|l| l.to != 0).count(), 10);
    }
}
