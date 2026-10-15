// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Seeded link layouts.

use rand_core::Rng;
use strims_base::rng::DetRng;

use crate::spec::{HostIx, Scenario, Transport};

/// Links each of `hosts[1..]` to up to `degree` distinct hosts that come
/// before it (the first always to `hosts[0]`'s side of the graph), so the
/// graph is connected and every host has at least one path to `hosts[0]`.
/// Each link is WebSocket with probability `ws_share`, else TCP.
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
}
