// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The controller: a turmoil client that samples every node's metrics (and
//! the run's heap) every `Scenario::sample_every`, folds their runner,
//! link and network stats every `Scenario::stats_every`, generates vpn
//! probes, and ends the run after `Scenario::duration` with a final
//! snapshot.

use std::rc::Rc;
use std::time::Duration;

use rand_core::Rng;
use strims_node::{HashTableGetOptions, Node, NodeMetrics, SwarmKey};
use tokio::task::spawn_local;

use crate::APP_PORT;
use crate::report::LookupPhase;
use crate::spec::{HostIx, VpnProbes};
use crate::world::{HASH_TABLE_SALT, PEER_INDEX_KEY, World, sim_now, sim_now_ms};

const CALL_TIMEOUT: Duration = Duration::from_secs(2);
/// vpn lookups wait this long for an answer.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(4);

pub(crate) async fn controller(w: Rc<World>) -> turmoil::Result {
    if let Some(p) = w.spec.probes.clone() {
        spawn_local(probes(w.clone(), p));
    }
    let end = w.spec.duration;
    let every = w.spec.sample_every.max(Duration::from_millis(100));
    let stats_every = w
        .spec
        .stats_every
        .map_or(every, |d| d.clamp(Duration::from_millis(100), every));
    let (mut next_full, mut next_stats) = (every, stats_every);
    loop {
        let next = next_full.min(next_stats);
        if next >= end {
            break;
        }
        tokio::time::sleep(next.saturating_sub(sim_now())).await;
        let full = next >= next_full;
        sample(&w, full, false).await;
        if full {
            next_full += every;
        }
        if next >= next_stats {
            next_stats += stats_every;
        }
    }
    tokio::time::sleep(end.saturating_sub(sim_now())).await;
    sample(&w, true, true).await;
    Ok(())
}

/// Folds every running node's runner, link and network stats. A `full`
/// sample also stores the metrics, samples the heap, checks the
/// invariants, and logs two lines per host (with more counters on the
/// `last` one).
async fn sample(w: &World, full: bool, last: bool) {
    for h in 0..w.spec.hosts.len() {
        let Some(node) = w.node(h) else {
            continue;
        };
        let Ok(Ok(m)) = tokio::time::timeout(CALL_TIMEOUT, node.metrics()).await else {
            continue;
        };
        w.observe_stats(h, &m);
        if full {
            w.log("ctl", metrics_line(w.name(h), &m, last));
            w.log("ctl", runner_line(w.name(h), &m, &w.swarm_keys(h)));
            w.store_metrics(h, m);
        }
    }
    if full {
        w.check_invariants();
    }
}

/// The runner's state in one line: peers, the longest data queues (pushes,
/// requests), queue refusals, request deadlines and flight size summed
/// over channels, LEDBAT's window range and longest CTO, the bytes and
/// blocked links vnic holds, vpn's queued copies, and per swarm the
/// store's next missing chunk, its channels and the push hits and misses.
fn runner_line(name: &str, m: &NodeMetrics, swarms: &[Option<SwarmKey>]) -> String {
    let (mut queued, mut drops) = ([0usize; 2], 0u64);
    let (mut pending, mut flight, mut cto) = (0usize, 0i64, 0i64);
    let (mut cwnd_min, mut cwnd_max) = (i64::MAX, 0i64);
    for p in &m.runner.peers {
        for (a, b) in queued.iter_mut().zip(p.queues.queued_data) {
            *a = (*a).max(b);
        }
        drops += p.queues.dropped;
        for c in p.channels.iter().filter_map(|c| c.stats.as_ref()) {
            let s = &c.scheduler;
            pending += s.pending_requests;
            flight += s.flight_size;
            cto = cto.max(s.cto.as_millis());
            cwnd_min = cwnd_min.min(s.cwnd);
            cwnd_max = cwnd_max.max(s.cwnd);
        }
    }
    let mut s = format!(
        "{name} runner peers {} queued {}/{} drops {drops} pending {pending} flight {flight} cwnd {}-{} cto {cto}",
        m.runner.peers.len(),
        queued[0],
        queued[1],
        if cwnd_max == 0 { 0 } else { cwnd_min },
        cwnd_max,
    );
    let link_pending: usize = m.links.iter().map(|l| l.transmit_pending).sum();
    let blocked = m.links.iter().filter(|l| l.write_blocked).count();
    let vpn_queued: usize = m.networks.iter().map(|n| n.queued).sum();
    s.push_str(&format!(
        " links pending {link_pending} blocked {blocked} vpn queued {vpn_queued}"
    ));
    for sw in &m.runner.swarms {
        let Some(i) = swarms.iter().position(|k| *k == Some(sw.swarm)) else {
            continue;
        };
        let next = if sw.store.ready {
            (sw.store.next.0 / 2).to_string()
        } else {
            "-".to_owned()
        };
        s.push_str(&format!(" | s{i} next {next} ch {}", sw.channels));
        if let Some(p) = &sw.push
            && p.mode == strims_ppspp::PushMode::Hybrid
        {
            s.push_str(&format!(" push {}/{}", p.hits, p.misses));
        }
    }
    s
}

/// The node's links and vnic byte counters (more counters on the `last`
/// sample). Trace lines carry no datagram counts or sizes
/// (`DriverMetrics::udp_*`): WebRTC's unseeded DTLS and SCTP change them
/// between runs of a seed.
fn metrics_line(name: &str, m: &NodeMetrics, last: bool) -> String {
    let d = &m.driver;
    let mut s = format!(
        "{name} links {} up {} down {}",
        m.links.len(),
        d.bytes_written,
        d.bytes_read
    );
    if last {
        s.push_str(&format!(
            " frames {}/{} transfer errors {} vpn frame errors {} accept errors {} events dropped {} vpn sent {} relayed {} delivered {} queue full {}",
            m.host.frame_write_count,
            m.host.frame_read_count,
            d.transfer_errors,
            d.vpn_frame_errors,
            d.accept_errors,
            d.events_dropped,
            m.vpn.messages_sent,
            m.vpn.messages_relayed,
            m.vpn.messages_delivered,
            m.vpn.link_queue_full,
        ));
    }
    s
}

/// A running member of the probed network: its index, node and network.
struct Member {
    h: HostIx,
    node: Node,
    net: strims_node::NetworkId,
}

fn members(w: &World, p: &VpnProbes) -> Vec<Member> {
    let Some(spec) = w.spec.networks.get(p.network) else {
        return Vec::new();
    };
    spec.members
        .iter()
        .filter_map(|&h| {
            Some(Member {
                h,
                node: w.node(h)?,
                net: w.net(h, p.network)?,
            })
        })
        .collect()
}

/// Two distinct running members, chosen by the run's rng.
fn pick_pair(w: &World, p: &VpnProbes) -> Option<(Member, Member)> {
    let mut m = members(w, p);
    if m.len() < 2 {
        return None;
    }
    let (a, b) = w.with_rng(|r| {
        let n = m.len() as u64;
        let a = r.next_u64() % n;
        let b = (a + 1 + r.next_u64() % (n - 1)) % n;
        (a as usize, b as usize)
    });
    let (a, b) = if a < b { (a, b) } else { (b, a) };
    let mb = m.swap_remove(b);
    let ma = m.swap_remove(a);
    if w.with_rng(|r| r.next_u32() % 2 == 0) {
        Some((ma, mb))
    } else {
        Some((mb, ma))
    }
}

async fn probes(w: Rc<World>, p: VpnProbes) {
    let every = [p.unicast_every, p.lookup_every, p.exchange_every];
    // Kinds without an interval never come due.
    let mut next = every.map(|e| if e.is_some() { p.start } else { p.stop });
    let mut id = 0u64;
    loop {
        let (i, at) = next
            .iter()
            .copied()
            .enumerate()
            .min_by_key(|&(_, t)| t)
            .unwrap_or((0, p.stop));
        if at >= p.stop {
            return;
        }
        tokio::time::sleep(at.saturating_sub(sim_now())).await;
        if let Some(n) = next.get_mut(i) {
            *n += every
                .get(i)
                .copied()
                .flatten()
                .unwrap_or(p.stop)
                .max(Duration::from_millis(10));
        }
        let Some((a, b)) = pick_pair(&w, &p) else {
            continue;
        };
        match i {
            0 => {
                id += 1;
                unicast(&w, id, &a, &b).await;
            }
            1 => {
                spawn_local(get(w.clone(), a.node.clone(), a.net, b.h));
                spawn_local(search(w.clone(), a.node, a.net, b.h));
            }
            _ => exchange(&w, &a, &b).await,
        }
    }
}

async fn unicast(w: &World, id: u64, a: &Member, b: &Member) {
    let Some(dst) = w.host_id(b.h) else {
        return;
    };
    w.probe_sent(id, a.h, b.h);
    let body = format!("probe {id}").into_bytes();
    let sent = tokio::time::timeout(
        CALL_TIMEOUT,
        a.node.send(a.net, dst, APP_PORT, APP_PORT, body),
    )
    .await;
    if !matches!(sent, Ok(Ok(()))) {
        w.log(
            "ctl",
            format!("probe {id} {} -> {} not sent", w.name(a.h), w.name(b.h)),
        );
    }
}

async fn get(w: Rc<World>, node: Node, net: strims_node::NetworkId, target: HostIx) {
    let start = sim_now_ms();
    let key = w.hash_table_key(target).public;
    let want = w.hash_table_value(target);
    let options = HashTableGetOptions {
        disable_cache: true,
        timeout: strims_base::time::Duration::from_secs(3),
    };
    let ok = match node
        .hash_table_get(net, key, HASH_TABLE_SALT.to_vec(), options)
        .await
    {
        Ok(mut g) => matches!(
            tokio::time::timeout(LOOKUP_TIMEOUT, g.next()).await,
            Ok(Some(v)) if v == want
        ),
        Err(_) => false,
    };
    w.lookup_done(start, false, ok);
    w.log(
        "ctl",
        format!(
            "get {} {}",
            w.name(target),
            if ok { "ok" } else { "failed" }
        ),
    );
}

async fn search(w: Rc<World>, node: Node, net: strims_node::NetworkId, target: HostIx) {
    let start = sim_now_ms();
    let mut ok = false;
    if let Ok(mut s) = node
        .peer_index_search(net, PEER_INDEX_KEY.to_vec(), w.peer_index_salt(target))
        .await
    {
        let found = tokio::time::timeout(LOOKUP_TIMEOUT, async {
            while let Some(host) = s.next().await {
                if w.ix_of(&host.host_id) == Some(target) {
                    return true;
                }
            }
            false
        })
        .await;
        ok = matches!(found, Ok(true));
    }
    w.lookup_done(start, true, ok);
    w.log(
        "ctl",
        format!(
            "search {} {}",
            w.name(target),
            if ok { "ok" } else { "failed" }
        ),
    );
}

async fn exchange(w: &World, a: &Member, b: &Member) {
    let (Some(dst), Some(key)) = (w.host_id(b.h), w.profile_key(b.h)) else {
        return;
    };
    let linked = match a.node.peers().await {
        Ok(peers) => peers.iter().any(|p| p.profile_key == key.public),
        Err(_) => return,
    };
    if linked {
        return;
    }
    if let Ok(Ok(true)) = tokio::time::timeout(CALL_TIMEOUT, a.node.connect_peer(a.net, dst)).await
    {
        w.with_vpn(|v| v.exchanges += 1);
        w.log(
            "ctl",
            format!("exchange {} -> {}", w.name(a.h), w.name(b.h)),
        );
    }
}

/// Splits lookups into 10 s buckets of start time.
pub(crate) fn lookup_phases(w: &World) -> Vec<LookupPhase> {
    let mut lookups = w.st.borrow().lookups.clone();
    lookups.sort_by_key(|&(t, _, _)| t);
    let mut out: Vec<LookupPhase> = Vec::new();
    for (t, search, ok) in lookups {
        let b = t / 10_000;
        let label = format!("{}-{} s", b * 10, b * 10 + 10);
        if out.last().is_none_or(|p| p.label != label) {
            out.push(LookupPhase {
                label,
                ..LookupPhase::default()
            });
        }
        if let Some(p) = out.last_mut() {
            if search {
                p.searches += 1;
                p.searches_ok += u64::from(ok);
            } else {
                p.gets += 1;
                p.gets_ok += u64::from(ok);
            }
        }
    }
    out
}

/// Splits unicast probes into 10 s buckets of send time.
pub(crate) fn unicast_phases(w: &World) -> Vec<(String, u64, u64)> {
    let st = w.st.borrow();
    let mut out: Vec<(String, u64, u64)> = Vec::new();
    for p in st.probes.values() {
        let b = p.sent_ms / 10_000;
        let label = format!("{}-{} s", b * 10, b * 10 + 10);
        match out.last_mut() {
            Some(last) if last.0 == label => {
                last.1 += 1;
                last.2 += u64::from(p.delivered.is_some());
            }
            _ => out.push((label, 1, u64::from(p.delivered.is_some()))),
        }
    }
    out
}
