// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Transfers between a Go and a Rust node over a direct link: Go's
//! internal/transfer on the helper (`-transfer`, the `transfer-*` verbs),
//! and `strims_transfer` through `strims_node::app` on a Rust node. Both
//! bind every peer to one network, and the swarm binds by announce, with
//! no `run_swarm` on either side.
//!
//! - `go_seed_rust_viewer_bind_by_announce`: Go publishes before the
//!   link, so its announce to Rust is refused. Rust publishes after it,
//!   and Go answers Rust's announce with its channel. Rust reads the
//!   stream byte-exact.
//! - `rust_seed_go_viewer_bind_by_announce`: the reverse, with Go's
//!   announce answered by Rust. Go reads the stream byte-exact, then
//!   Rust removes the transfer and closes it with Go.

use std::rc::Rc;
use std::time::Duration as StdDuration;

use strims_crypto::key_from_seed;
use strims_node::app::{NodeHost, NodeTimer, PeerServices};
use strims_node::{
    HostId, NetworkId, NetworkOptions, Node, NodeConfig, NodeEvent, SwarmId, SwarmKey, SystemClock,
    TokioNet, WriterOptions,
};
use strims_transfer::{TransferControl, TransferId};
use strims_vpn::VerifyMode;
use tokio::task::{LocalSet, spawn_local};
use tokio::time::Instant;

use crate::harness::{
    GoEvent, GoNode, GoOptions, STEP, UNIT, VPN_PORT, helper, network_key, stream_byte,
    swarm_options,
};

const LIVE_WINDOW: usize = 1 << 10;
const BYTES: u64 = 512 * 1024;
const RATE: u64 = 512 * 1024;
const SALT: &[u8] = b"interop";

type Transfers = TransferControl<NodeHost, NodeTimer<SystemClock>>;

/// A Rust node serving transfers to its peers.
struct RustTransfers {
    node: Node,
    net: NetworkId,
    transfers: Transfers,
}

async fn rust_node(seed: u8) -> RustTransfers {
    let mut config = NodeConfig::new(key_from_seed(&[seed; 32]), u64::from(seed));
    config.host_key_seed = Some([seed.wrapping_add(0x80); 32]);
    config.label = format!("rust-{seed}");
    config.tcp_listen = Some("127.0.0.1:0".to_owned());
    config.max_peers = 16;
    config.vpn.verify = VerifyMode::Audit;
    let (node, mut events) = Node::spawn(config, TokioNet, SystemClock).await.unwrap();
    let net = node
        .add_network(
            &network_key(),
            NetworkOptions {
                auto_bind_port: Some(VPN_PORT),
            },
        )
        .await
        .unwrap();
    let transfers = TransferControl::new(
        NodeHost::new(node.clone()),
        NodeTimer(SystemClock),
        strims_transfer::Config::default(),
    );
    spawn_local({
        let transfers = transfers.clone();
        async move { transfers.run().await }
    });
    let services = PeerServices::new(node.clone()).with(Rc::new(transfers.clone()));
    spawn_local(async move {
        while let Some(e) = events.recv().await {
            if let NodeEvent::TransferError { .. } = &e {
                eprintln!("rust {seed}: {e:?}");
            }
            services.handle_event(&e).await;
        }
    });
    RustTransfers {
        node,
        net,
        transfers,
    }
}

/// The channel the node runs `swarm` on with `peer`, if any.
async fn node_channel(node: &Node, peer: HostId, swarm: SwarmKey) -> Option<(u64, u64)> {
    let m = node.metrics().await.unwrap();
    m.runner
        .peers
        .iter()
        .filter(|p| p.host_id == peer)
        .flat_map(|p| p.channels.iter())
        .find(|c| c.swarm == swarm)
        .map(|c| (c.channel, c.peer_channel))
}

/// Waits until the transfer runs with `peer` on both the control and the
/// node; its channels.
async fn bound(r: &RustTransfers, peer: HostId, id: TransferId, swarm: SwarmKey) -> (u64, u64) {
    let deadline = Instant::now() + STEP;
    loop {
        if let Some(c) = r.transfers.channels(peer, id)
            && node_channel(&r.node, peer, swarm).await == Some(c)
        {
            return c;
        }
        assert!(
            Instant::now() < deadline,
            "never bound: {:?}",
            r.transfers.metrics()
        );
        tokio::time::sleep(StdDuration::from_millis(50)).await;
    }
}

fn go_options(seed: u8) -> GoOptions {
    let mut o = GoOptions::new(seed);
    o.peer_rpc = true;
    o.transfer = true;
    o.max_peers = 16;
    o
}

fn no_transfer_errors(go: &mut GoNode) {
    let errors: Vec<&GoEvent> = go
        .events()
        .iter()
        .filter(|e| matches!(e, GoEvent::TransferError(..)))
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn go_seed_rust_viewer_bind_by_announce() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        120,
        LocalSet::new().run_until(async {
            let key_seed = 0xa5;
            let mut go = GoNode::spawn(&bin, "ga0", go_options(0xa0)).await;
            let id = go
                .cmd(&format!(
                    "swarm-create s {} {LIVE_WINDOW} mandatory",
                    hex::encode([key_seed; 32])
                ))
                .await
                .unwrap();
            let go_transfer = go
                .cmd(&format!("transfer-publish s {}", hex::encode(SALT)))
                .await
                .unwrap();

            let r = rust_node(0xa1).await;
            let swarm_id = SwarmId::new(&key_from_seed(&[key_seed; 32]).public);
            assert_eq!(swarm_id.to_string(), id);
            let swarm = r
                .node
                .add_swarm(swarm_id.clone(), swarm_options(LIVE_WINDOW))
                .await
                .unwrap();
            let mut reader = r.node.open_reader(swarm).await.unwrap();
            assert_eq!(r.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);
            // Go announces as Rust joins the network, before Rust publishes
            tokio::time::sleep(StdDuration::from_secs(1)).await;
            let t = r.transfers.add(swarm, &swarm_id, SALT.to_vec());
            assert_eq!(t.to_string(), go_transfer, "both derive the same id");
            assert!(r.transfers.publish(t, r.net));
            let channels = bound(&r, go.host_id, t, swarm).await;
            eprintln!("bound on {channels:?}: {:?}", r.transfers.metrics());

            go.cmd(&format!("swarm-write s {BYTES} {RATE}"))
                .await
                .unwrap();
            let mut out = vec![0u8; BYTES as usize];
            tokio::time::timeout(StdDuration::from_secs(60), reader.read_exact(&mut out))
                .await
                .expect("the stream arrives")
                .unwrap();
            let bad = (0..BYTES)
                .filter(|&o| out[o as usize] != stream_byte(o))
                .count();
            assert_eq!(bad, 0, "the stream arrives byte-exact");
            let m = r.transfers.metrics();
            assert_eq!(m.announces_accepted, 1, "{m:?}");
            assert_eq!(m.runs, 1, "{m:?}");
            no_transfer_errors(&mut go);
        }),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rust_seed_go_viewer_bind_by_announce() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        120,
        LocalSet::new().run_until(async {
            let key_seed = 0xb5;
            let r = rust_node(0xb1).await;
            let (swarm, swarm_id, mut writer) = r
                .node
                .create_swarm(WriterOptions {
                    swarm_options: swarm_options(LIVE_WINDOW),
                    key: key_from_seed(&[key_seed; 32]),
                })
                .await
                .unwrap();
            let t = r.transfers.add(swarm, &swarm_id, SALT.to_vec());
            assert!(r.transfers.publish(t, r.net));

            let mut go = GoNode::spawn(&bin, "gb0", go_options(0xb0)).await;
            assert_eq!(r.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);
            // Rust announces as Go joins the network, before Go publishes
            tokio::time::sleep(StdDuration::from_secs(1)).await;
            go.cmd(&format!(
                "swarm-add s {} {LIVE_WINDOW} mandatory",
                hex::encode(key_from_seed(&[key_seed; 32]).public)
            ))
            .await
            .unwrap();
            let go_transfer = go
                .cmd(&format!("transfer-publish s {}", hex::encode(SALT)))
                .await
                .unwrap();
            assert_eq!(t.to_string(), go_transfer, "both derive the same id");
            let channels = bound(&r, go.host_id, t, swarm).await;
            eprintln!("bound on {channels:?}: {:?}", r.transfers.metrics());

            go.cmd(&format!("swarm-read s {BYTES}")).await.unwrap();
            let mut buf = vec![0u8; UNIT];
            let mut off = 0u64;
            while off < BYTES {
                let n = (BYTES - off).min(UNIT as u64) as usize;
                for (i, b) in buf[..n].iter_mut().enumerate() {
                    *b = stream_byte(off + i as u64);
                }
                writer.write(&buf[..n]).await.unwrap();
                off += n as u64;
                tokio::time::sleep(StdDuration::from_millis(50)).await;
            }
            writer.flush().await.unwrap();
            let deadline = Instant::now() + StdDuration::from_secs(60);
            let GoEvent::ReadDone(_, res) = go
                .wait_until("read", deadline, |e| matches!(e, GoEvent::ReadDone(..)))
                .await
            else {
                unreachable!()
            };
            res.assert_exact("go viewer", BYTES);
            let m = r.transfers.metrics();
            assert_eq!(m.announces_answered, 1, "{m:?}");

            // Removing the transfer stops it here and closes it with Go.
            assert!(r.transfers.remove(t));
            let deadline = Instant::now() + STEP;
            while node_channel(&r.node, go.host_id, swarm).await.is_some() {
                assert!(Instant::now() < deadline, "the channel never stopped");
                tokio::time::sleep(StdDuration::from_millis(50)).await;
            }
            tokio::time::sleep(StdDuration::from_secs(1)).await;
            let m = r.transfers.metrics();
            assert_eq!((m.stops, m.call_errors), (1, 0), "{m:?}");
            assert!(m.closes_sent >= 1, "{m:?}");
            no_transfer_errors(&mut go);
        }),
    )
    .await;
}
