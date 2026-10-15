// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Each host's software: the node, its event policy, link supervision, and
//! the swarm writer and readers.
//!
//! turmoil re-runs [`host_main`] when a host restarts, so everything here
//! starts from the spec and the shared registry: a restarted host joins the
//! same networks and swarms with a new host key, its seed continues the
//! stream from the current time, and its viewers keep accumulating into the
//! same statistics.

use std::rc::Rc;
use std::time::Duration;

use strims_node::{
    NetworkId, NetworkOptions, Node, NodeEvent, NodeEvents, ReadError, SimClock, SwarmId, SwarmKey,
    SwarmWriter, TurmoilNet, WriterOptions,
};
use tokio::task::spawn_local;

use crate::APP_PORT;
use crate::spec::{Binding, Fault, HostIx, LinkSpec};
use crate::stream::{FRAME_LEN, FrameParser, encode_frame};
use crate::world::{HASH_TABLE_SALT, PEER_INDEX_KEY, World, epoch, sim_now, sim_now_ms};

/// How long a dial may take before the supervisor gives up on it.
const DIAL_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) async fn host_main(w: Rc<World>, h: HostIx) -> turmoil::Result {
    let inc = w.begin_incarnation(h);
    let config = w.node_config(h, inc).ok_or("unknown host")?;
    let (node, events) = Node::spawn(config, TurmoilNet, SimClock::turmoil(epoch())).await?;
    // Peers may link before the setup below finishes; they must know us.
    w.register_id(h, node.host_id());

    let mut nets: Vec<Option<NetworkId>> = vec![None; w.spec.networks.len()];
    for (n, spec) in w.spec.networks.iter().enumerate() {
        if !spec.members.contains(&h) {
            continue;
        }
        let key = w.network_keys.get(n).ok_or("network key")?;
        let auto_bind_port = (spec.binding == Binding::Auto).then_some(spec.port);
        let net = node
            .add_network(&key.public, NetworkOptions { auto_bind_port })
            .await?;
        node.set_vpn_handler(net, APP_PORT).await?;
        if let Some(slot) = nets.get_mut(n) {
            *slot = Some(net);
        }
    }
    if let Some(p) = &w.spec.probes
        && let Some(Some(net)) = nets.get(p.network)
    {
        spawn_local(publish(w.clone(), h, node.clone(), *net));
    }

    let mut swarms: Vec<Option<SwarmKey>> = vec![None; w.spec.swarms.len()];
    for (s, spec) in w.spec.swarms.iter().enumerate() {
        let key = w.swarm_keys.get(s).ok_or("swarm key")?;
        let swarm = if spec.seed == h {
            let (swarm, _, writer) = node
                .create_swarm(WriterOptions {
                    swarm_options: spec.options.clone(),
                    key: key.clone(),
                })
                .await?;
            spawn_local(writer_task(w.clone(), h, s, writer));
            swarm
        } else if spec.member(h) {
            let swarm = node
                .add_swarm(SwarmId::new(&key.public), spec.options.clone())
                .await?;
            if spec.viewers.contains(&h) {
                spawn_local(reader_task(w.clone(), h, s, node.clone(), swarm));
            }
            swarm
        } else {
            continue;
        };
        if let Some(slot) = swarms.get_mut(s) {
            *slot = Some(swarm);
        }
    }

    w.register(h, &node, nets.clone(), swarms.clone());
    spawn_local(event_task(w.clone(), h, node.clone(), events, nets, swarms));
    if w.spec.faults.iter().any(|f| match f.fault {
        Fault::Uplink(x, _) | Fault::Connect { from: x, .. } => x == h,
        _ => false,
    }) {
        spawn_local(control(w.clone(), h, node.clone()));
    }
    spawn_local(supervisor(w.clone(), h, node));
    std::future::pending::<()>().await;
    Ok(())
}

/// Publishes the host's hash table record and peer index entry at
/// startup, before it has links. Publishes repeat only every 10 minutes,
/// so lookups find these records because vpn gives them to links that
/// come up later (see [`strims_vpn::RECORD_PUSH_DELAY`]).
async fn publish(w: Rc<World>, h: HostIx, node: Node, net: NetworkId) {
    let me = w.name(h).to_owned();
    let key = w.hash_table_key(h);
    if let Err(e) = node
        .hash_table_set(net, key, HASH_TABLE_SALT.to_vec(), w.hash_table_value(h))
        .await
    {
        w.log(&me, format!("hash table set failed: {e}"));
    }
    if let Err(e) = node
        .peer_index_publish(net, PEER_INDEX_KEY.to_vec(), w.peer_index_salt(h), APP_PORT)
        .await
    {
        w.log(&me, format!("peer index publish failed: {e}"));
    }
    w.log(&me, "published records");
}

/// The host's event policy: run every shared swarm with a new peer
/// (channel `swarm index + 1` both ways), bind explicit networks, record
/// vpn probes and exchanges, and log it all.
async fn event_task(
    w: Rc<World>,
    h: HostIx,
    node: Node,
    mut events: NodeEvents,
    nets: Vec<Option<NetworkId>>,
    swarms: Vec<Option<SwarmKey>>,
) {
    let me = w.name(h).to_owned();
    while let Some(e) = events.recv().await {
        match e {
            NodeEvent::PeerAdded(p) => {
                let peer = w.ix_of(&p.host_id);
                let dir = if p.dialed { "to" } else { "from" };
                w.log(&me, format!("peer+ {dir} {} {:?}", w.label(peer), p.kind));
                let Some(px) = peer else {
                    continue;
                };
                for (s, swarm) in swarms.iter().enumerate() {
                    let (Some(swarm), Some(spec)) = (swarm, w.spec.swarms.get(s)) else {
                        continue;
                    };
                    if spec.member(px) {
                        let ch = s as u64 + 1;
                        if let Err(e) = node.run_swarm(p.host_id, *swarm, ch, ch).await {
                            w.log(&me, format!("run_swarm {} failed: {e}", w.name(px)));
                        }
                    }
                }
                for (n, net) in nets.iter().enumerate() {
                    let (Some(net), Some(spec)) = (net, w.spec.networks.get(n)) else {
                        continue;
                    };
                    if spec.binding == Binding::Explicit
                        && spec.members.contains(&px)
                        && let Err(e) = node
                            .bind_peer_network(p.host_id, *net, spec.port, spec.port)
                            .await
                    {
                        w.log(&me, format!("bind {} failed: {e}", w.name(px)));
                    }
                }
            }
            NodeEvent::PeerRemoved { host_id } => {
                w.log(&me, format!("peer- {}", w.label(w.ix_of(&host_id))));
            }
            NodeEvent::Message(d) => {
                let body = String::from_utf8_lossy(&d.body);
                if let Some(id) = body.strip_prefix("probe ").and_then(|s| s.parse().ok())
                    && w.probe_delivered(id, d.hops)
                {
                    w.log(
                        &me,
                        format!(
                            "probe {id} from {} hops {}",
                            w.label(w.ix_of(&d.src)),
                            d.hops
                        ),
                    );
                }
            }
            NodeEvent::ExchangeClosed {
                host_id, outcome, ..
            } => {
                w.exchange_closed(&outcome);
                w.log(
                    &me,
                    format!("exchange {} {outcome:?}", w.label(w.ix_of(&host_id))),
                );
            }
            NodeEvent::TransferError { host_id, error } => {
                w.log(
                    &me,
                    format!("transfer error {}: {error}", w.label(w.ix_of(&host_id))),
                );
            }
        }
    }
}

/// Dials this host's links that are down, now and every
/// `Scenario::redial_every`. It is a run's only reconnect: links heal
/// after partitions and restarts through it.
async fn supervisor(w: Rc<World>, h: HostIx, node: Node) {
    let links: Vec<(usize, LinkSpec)> = w
        .spec
        .links
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, l)| l.from == h)
        .collect();
    if links.is_empty() {
        return;
    }
    loop {
        let Ok(peers) = node.peers().await else {
            return;
        };
        for &(l, spec) in &links {
            let Some(key) = w.profile_key(spec.to) else {
                continue;
            };
            let up = peers.iter().any(|p| p.profile_key == key.public);
            if !up && w.start_dial(h, l) {
                spawn_local(dial(w.clone(), h, l, spec, node.clone()));
            }
        }
        tokio::time::sleep(w.spec.redial_every).await;
    }
}

/// Applies `Fault::Uplink` and `Fault::Connect` to the node.
async fn control(w: Rc<World>, h: HostIx, node: Node) {
    let me = w.name(h).to_owned();
    loop {
        if let Some(rate) = w.take_uplink(h)
            && node.set_rate_limit(rate).await.is_err()
        {
            return;
        }
        for (to, n) in w.take_connects(h) {
            let (Some(net), Some(id)) = (w.net(h, n), w.host_id(to)) else {
                w.log(&me, format!("connect {} skipped", w.name(to)));
                continue;
            };
            match node.connect_peer(net, id).await {
                Ok(started) => w.log(&me, format!("connect {} started {started}", w.name(to))),
                Err(e) => w.log(&me, format!("connect {} failed: {e}", w.name(to))),
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn dial(w: Rc<World>, h: HostIx, l: usize, spec: LinkSpec, node: Node) {
    let Some(uri) = w.uri(spec.to, spec.transport) else {
        return;
    };
    let result = match tokio::time::timeout(DIAL_TIMEOUT, node.dial(&uri)).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("timed out".to_owned()),
    };
    w.end_dial(h, l, result);
}

/// Writes `frames_per_write` frames every `write_interval` from `start` to
/// `stop`. The write index comes from the simulated time, so a restarted
/// seed continues the numbering.
async fn writer_task(w: Rc<World>, h: HostIx, s: usize, mut writer: SwarmWriter) {
    let Some(spec) = w.spec.swarms.get(s).cloned() else {
        return;
    };
    let me = w.name(h).to_owned();
    let fpw = spec.frames_per_write as u64;
    let step = spec.write_interval.as_millis().max(1);
    let now = sim_now();
    let mut k = if now <= spec.start {
        0
    } else {
        u64::try_from((now - spec.start).as_millis().div_ceil(step)).unwrap_or(u64::MAX)
    };
    let mut buf = vec![0u8; spec.frames_per_write * FRAME_LEN];
    loop {
        let at = spec.start + spec.write_interval * u32::try_from(k).unwrap_or(u32::MAX);
        if at >= spec.stop {
            break;
        }
        tokio::time::sleep(at.saturating_sub(sim_now())).await;
        if w.take_writer_reset(s) {
            match writer.reset().await {
                Ok(()) => w.log(&me, format!("swarm {s} writer reset")),
                Err(e) => w.log(&me, format!("swarm {s} writer reset failed: {e}")),
            }
        }
        let t = sim_now_ms();
        for (j, f) in buf.chunks_mut(FRAME_LEN).enumerate() {
            encode_frame(w.seed, s as u32, k * fpw + j as u64, t, f);
        }
        if let Err(e) = writer.write(&buf).await {
            w.log(&me, format!("swarm {s} write failed: {e}"));
            return;
        }
        w.mark_written(s, k * fpw..(k + 1) * fpw);
        if k % 40 == 0 {
            w.log(&me, format!("swarm {s} wrote {}", (k + 1) * fpw));
        }
        k += 1;
    }
    w.log(&me, format!("swarm {s} writer done"));
    // Keep the publisher: dropping the writer closes it.
    std::future::pending::<()>().await;
}

async fn reader_task(w: Rc<World>, h: HostIx, s: usize, node: Node, swarm: SwarmKey) {
    let me = w.name(h).to_owned();
    let mut reader = match node.open_reader(swarm).await {
        Ok(r) => r,
        Err(e) => {
            w.log(&me, format!("swarm {s} open reader failed: {e}"));
            return;
        }
    };
    w.viewer_start(s, h);
    let mut buf = vec![0u8; 64 * 1024];
    let mut parser = FrameParser::default();
    loop {
        while w.reading_paused(h) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        match reader.read(&mut buf).await {
            Ok(n) => w.viewer_data(s, h, &mut parser, buf.get(..n).unwrap_or_default()),
            Err(ReadError::Underrun { skipped }) => {
                parser.resync();
                w.viewer_underrun(s, h, skipped);
            }
            Err(ReadError::Reset) => {
                parser.resync();
                w.viewer_reset(s, h);
            }
            Err(e) => {
                w.log(&me, format!("swarm {s} reader ended: {e}"));
                return;
            }
        }
    }
}
