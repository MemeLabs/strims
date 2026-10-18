// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Three nodes on real localhost sockets, once over TCP and once over
//! WebSocket: a dials b, b dials c, then
//!
//! - the vnic handshakes complete (dial results and `PeerAdded` events);
//! - a vpn message goes from a to c through b;
//! - c gets a's hash table record;
//! - c links to a through peer exchange (a dials c's advertised URI);
//! - a publishes a 4 MiB ppspp stream that b and c download over the
//!   triangle, byte-exact from offset 0;
//! - every node runs under an upload cap, so QoS (not the socket) is the
//!   bottleneck, and the seed's upload stays within it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::time::Duration as StdDuration;

use futures_util::{SinkExt, StreamExt};

use strims_base::rng::det_rng;
use strims_crypto::key_from_seed;
use strims_node::{
    DeliveryMode, ExchangeOutcome, HashTableGetOptions, HostId, NetworkOptions, Node, NodeConfig,
    NodeEvent, NodeEvents, SwarmId, SwarmOptions, SystemClock, TokioNet, WriterOptions,
};

const VPN_PORT: u16 = 10_000;
const APP_PORT: u16 = 20_000;
/// The upload cap of every node: QoS is the bottleneck, not loopback.
const UPLOAD_CAP: u64 = 2 * 1024 * 1024;
const STREAM_LEN: usize = 4 * 1024 * 1024;
const STEP: StdDuration = StdDuration::from_secs(20);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Tcp,
    Ws,
}

struct TestNode {
    node: Node,
    events: NodeEvents,
    seen: Vec<NodeEvent>,
}

impl TestNode {
    async fn spawn(i: u8, kind: Kind) -> TestNode {
        TestNode::spawn_with(i, kind, |_| {}).await
    }

    async fn spawn_with(i: u8, kind: Kind, f: impl FnOnce(&mut NodeConfig)) -> TestNode {
        let key = key_from_seed(&[i; 32]);
        let mut config = NodeConfig::new(key.clone(), u64::from(i));
        config.label = format!("node-{i}");
        match kind {
            Kind::Tcp => config.tcp_listen = Some("127.0.0.1:0".to_owned()),
            Kind::Ws => config.ws_listen = Some("127.0.0.1:0".to_owned()),
        }
        config.allow_insecure_ws = true;
        config.max_peers = 16;
        config.rate_limit = Some(UPLOAD_CAP);
        f(&mut config);
        let (node, events) = Node::spawn(config, TokioNet, SystemClock).await.unwrap();
        TestNode {
            node,
            events,
            seen: Vec::new(),
        }
    }

    fn uri(&self, kind: Kind) -> String {
        match kind {
            Kind::Tcp => self.node.tcp_uri(),
            Kind::Ws => self.node.ws_uri(),
        }
        .expect("listener uri")
        .to_owned()
    }

    /// Waits for an event matching `f`, keeping the others.
    async fn wait(&mut self, what: &str, mut f: impl FnMut(&NodeEvent) -> bool) -> NodeEvent {
        if let Some(i) = self.seen.iter().position(&mut f) {
            return self.seen.remove(i);
        }
        let res = tokio::time::timeout(STEP, async {
            loop {
                let e = self.events.recv().await.expect("node stopped");
                if f(&e) {
                    return e;
                }
                self.seen.push(e);
            }
        })
        .await;
        res.unwrap_or_else(|_| panic!("timed out waiting for {what}; seen {:?}", self.seen))
    }
}

fn stream_bytes(len: usize) -> Vec<u8> {
    let mut rng = det_rng(42);
    let mut b = vec![0u8; len];
    rand_core::Rng::fill_bytes(&mut rng, &mut b);
    b
}

fn swarm_options() -> SwarmOptions {
    SwarmOptions {
        live_window: 1 << 13,
        delivery_mode: DeliveryMode::MANDATORY,
        ..Default::default()
    }
}

async fn scenario(kind: Kind) {
    let mut a = TestNode::spawn(1, kind).await;
    let mut b = TestNode::spawn(2, kind).await;
    let mut c = TestNode::spawn(3, kind).await;
    let (a_id, b_id, c_id) = (a.node.host_id(), b.node.host_id(), c.node.host_id());

    // Networks first, so peers are bound as they connect.
    let network_key = key_from_seed(&[0xee; 32]).public;
    let mut nets = Vec::new();
    for n in [&a, &b, &c] {
        let net = n
            .node
            .add_network(
                &network_key,
                NetworkOptions {
                    auto_bind_port: Some(VPN_PORT),
                },
            )
            .await
            .unwrap();
        n.node.set_vpn_handler(net, APP_PORT).await.unwrap();
        nets.push(net);
    }

    // Handshakes.
    assert_eq!(a.node.dial(&b.uri(kind)).await.unwrap(), b_id);
    assert_eq!(b.node.dial(&c.uri(kind)).await.unwrap(), c_id);
    let added = |peer: HostId| move |e: &NodeEvent| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == peer);
    a.wait("a: peer b", added(b_id)).await;
    b.wait("b: peer a", added(a_id)).await;
    b.wait("b: peer c", added(c_id)).await;
    c.wait("c: peer b", added(b_id)).await;
    let peers = b.node.peers().await.unwrap();
    assert_eq!(peers.len(), 2);
    let expected = match kind {
        Kind::Tcp => strims_node::LinkKind::Tcp,
        Kind::Ws => strims_node::LinkKind::Ws,
    };
    assert!(peers.iter().all(|p| p.kind == expected));

    // vpn: a to c, relayed by b.
    a.node
        .send(nets[0], c_id, APP_PORT, APP_PORT, b"hello c".to_vec())
        .await
        .unwrap();
    let NodeEvent::Message(m) = c
        .wait("message", |e| matches!(e, NodeEvent::Message(_)))
        .await
    else {
        unreachable!()
    };
    assert_eq!(m.body, b"hello c");
    assert_eq!(m.src, a_id);
    assert!(m.hops >= 2, "relayed through b: {m:?}");

    // Hash table: a publishes, c reads it from the network.
    let record_key = key_from_seed(&[0xaa; 32]);
    a.node
        .hash_table_set(
            nets[0],
            record_key.clone(),
            b"salt".to_vec(),
            b"value".to_vec(),
        )
        .await
        .unwrap();
    let value = tokio::time::timeout(STEP, async {
        loop {
            let mut get = c
                .node
                .hash_table_get(
                    nets[2],
                    record_key.public.clone(),
                    b"salt".to_vec(),
                    HashTableGetOptions {
                        disable_cache: true,
                        timeout: strims_base::time::Duration::from_secs(1),
                    },
                )
                .await
                .unwrap();
            if let Some(v) = get.next().await {
                return v;
            }
        }
    })
    .await
    .expect("hash table get");
    assert_eq!(value, b"value");

    // Peer index: a advertises a port for a key; c finds a.
    a.node
        .peer_index_publish(nets[0], b"swarm".to_vec(), b"salt".to_vec(), 4242)
        .await
        .unwrap();
    let host = tokio::time::timeout(STEP, async {
        loop {
            let mut search = c
                .node
                .peer_index_search(nets[2], b"swarm".to_vec(), b"salt".to_vec())
                .await
                .unwrap();
            while let Some(h) = search.next().await {
                if h.host_id == a_id {
                    return h;
                }
            }
        }
    })
    .await
    .expect("peer index search");
    assert_eq!(host.port, 4242);

    // Peer exchange: c asks a to link; a dials c's listener.
    assert!(c.node.connect_peer(nets[2], a_id).await.unwrap());
    let NodeEvent::ExchangeClosed { outcome, .. } = c
        .wait(
            "exchange closed",
            |e| matches!(e, NodeEvent::ExchangeClosed { host_id, .. } if *host_id == a_id),
        )
        .await
    else {
        unreachable!()
    };
    assert_eq!(outcome, ExchangeOutcome::RemoteConnected);
    c.wait(
        "peer a",
        |e| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == a_id && !p.dialed),
    )
    .await;
    a.wait(
        "peer c",
        |e| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == c_id && p.dialed),
    )
    .await;

    // ppspp over the triangle.
    let (sa, swarm_id, mut writer) = a
        .node
        .create_swarm(WriterOptions {
            swarm_options: swarm_options(),
            key: key_from_seed(&[0x55; 32]),
        })
        .await
        .unwrap();
    assert_eq!(swarm_id, SwarmId::new(&key_from_seed(&[0x55; 32]).public));
    let sb = b
        .node
        .add_swarm(swarm_id.clone(), swarm_options())
        .await
        .unwrap();
    let sc = c
        .node
        .add_swarm(swarm_id.clone(), swarm_options())
        .await
        .unwrap();
    for (n, s, peers) in [
        (&a, sa, [b_id, c_id]),
        (&b, sb, [a_id, c_id]),
        (&c, sc, [a_id, b_id]),
    ] {
        for p in peers {
            n.node.run_swarm(p, s, 1, 1).await.unwrap();
        }
    }
    let mut readers = [
        b.node.open_reader(sb).await.unwrap(),
        c.node.open_reader(sc).await.unwrap(),
    ];

    let data = stream_bytes(STREAM_LEN);
    let started = tokio::time::Instant::now();
    let write = async {
        for chunk in data.chunks(64 * 1024) {
            writer.write(chunk).await.unwrap();
            tokio::time::sleep(StdDuration::from_millis(5)).await;
        }
        writer.flush().await.unwrap();
    };
    let read = async {
        let mut outs = Vec::new();
        for r in readers.iter_mut() {
            let mut out = vec![0u8; STREAM_LEN];
            r.read_exact(&mut out).await.unwrap();
            outs.push(out);
        }
        outs
    };
    let (_, outs) = tokio::time::timeout(StdDuration::from_secs(60), async {
        tokio::join!(write, read)
    })
    .await
    .expect("stream delivery");
    let elapsed = started.elapsed().as_secs_f64();
    for out in &outs {
        assert!(out == &data, "stream bytes differ");
    }

    // The seed's upload stays under the cap (plus QoS's one-second burst).
    let ma = a.node.metrics().await.unwrap();
    let sent = ma.driver.bytes_written as f64;
    let cap = UPLOAD_CAP as f64;
    assert!(
        sent <= cap * (elapsed + 1.0) + 65_536.0,
        "seed sent {sent} bytes in {elapsed:.2}s over a cap of {cap} B/s"
    );
    // And the cap binds: one full copy must leave the seed, so delivery
    // can't beat the cap (less its one-second burst).
    assert!(sent >= STREAM_LEN as f64);
    assert!(
        elapsed >= (STREAM_LEN as f64 - cap) / cap,
        "{elapsed:.2}s is faster than the cap allows"
    );
    assert_eq!(ma.links.len(), 2);
    assert_eq!(ma.driver.transfer_errors, 0);

    // The runner metrics describe the swarm the seed publishes: its store
    // is ready, it has one swarm with a channel to each of the two peers,
    // and each channel's scheduler has an open congestion window.
    assert_eq!(ma.runner.swarms.len(), 1);
    let swarm = &ma.runner.swarms[0];
    assert!(swarm.store.ready);
    assert_eq!(swarm.channels, 2);
    assert!(swarm.scheduler.is_some());
    assert_eq!(ma.runner.peers.len(), 2);
    for peer in &ma.runner.peers {
        let ch = peer
            .channels
            .iter()
            .find(|c| c.swarm == sa)
            .expect("a channel for the seed's swarm");
        let stats = ch.stats.expect("channel stats");
        assert!(stats.scheduler.cwnd > 0, "cwnd {}", stats.scheduler.cwnd);
        // The seed only sends data on the swarm; its out counters moved.
        assert!(ch.metrics.write.count > 0);
        assert!(peer.qos_weight.is_some());
        assert!(peer.writer.is_some());
    }
    // Each viewer, in turn, read the stream: its channel has verified data.
    let mb = b.node.metrics().await.unwrap();
    let vch = mb
        .runner
        .peers
        .iter()
        .flat_map(|p| &p.channels)
        .find(|c| c.metrics.read.count > 0)
        .expect("a viewer channel that read data");
    assert!(vch.stats.unwrap().inbound.data > 0);
    eprintln!(
        "{kind:?}: {} bytes delivered to 2 peers in {elapsed:.2}s; seed sent {sent} bytes",
        STREAM_LEN
    );

    for n in [&a, &b, &c] {
        n.node.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn three_nodes_over_tcp() {
    scenario(Kind::Tcp).await;
}

#[tokio::test]
async fn three_nodes_over_websocket() {
    scenario(Kind::Ws).await;
}

#[tokio::test]
async fn dial_errors_are_reported() {
    let t = TestNode::spawn(9, Kind::Tcp).await;
    // Nothing listens there.
    let key = hex::encode([1u8; 32]);
    let err = t
        .node
        .dial(&format!("tcp://127.0.0.1:1/{key}"))
        .await
        .unwrap_err();
    assert!(matches!(err, strims_node::NodeError::Dial(_)), "{err:?}");
    // A listener for another profile key closes the connection.
    let other = TestNode::spawn(10, Kind::Tcp).await;
    let addr = other.node.info().tcp_addr.unwrap();
    let err = t
        .node
        .dial(&format!("tcp://{addr}/{key}"))
        .await
        .unwrap_err();
    assert!(matches!(err, strims_node::NodeError::Dial(_)), "{err:?}");
    assert!(t.node.dial("udp://x").await.is_err());
    assert!(matches!(
        t.node.dial("wss://127.0.0.1:1/x").await,
        Err(strims_node::NodeError::Dial(_))
    ));
    let m = t.node.metrics().await.unwrap();
    assert_eq!(m.driver.tcp_dials, 2);
    assert_eq!(m.driver.tcp_dial_errors, 2);
    assert_eq!(m.driver.ws_dial_errors, 1);
    let m = other.node.metrics().await.unwrap();
    assert_eq!(m.driver.accept_errors, 1);
    assert_eq!(m.links.len(), 0);

    // After shutdown, calls fail and the event stream ends.
    let mut t = t;
    t.node.shutdown().await.unwrap();
    assert_eq!(t.node.peers().await, Err(strims_node::NodeError::Stopped));
    assert!(
        tokio::time::timeout(STEP, t.events.recv())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn dialing_this_node_is_refused() {
    for (i, kind) in [(11, Kind::Tcp), (12, Kind::Ws)] {
        let mut t = TestNode::spawn(i, kind).await;
        let err = t.node.dial(&t.uri(kind)).await.unwrap_err();
        assert_eq!(err, strims_node::NodeError::OwnHost, "{kind:?}");
        // Both ends of the loop fail their handshakes without a peer.
        let settle = tokio::time::timeout(StdDuration::from_millis(300), async {
            while let Some(e) = t.events.recv().await {
                t.seen.push(e);
            }
        })
        .await;
        assert!(settle.is_err(), "node stopped");
        assert!(
            !t.seen
                .iter()
                .any(|e| matches!(e, NodeEvent::PeerAdded(_) | NodeEvent::PeerRemoved { .. })),
            "{kind:?}: {:?}",
            t.seen
        );
        assert!(t.node.peers().await.unwrap().is_empty());
    }
}

/// A reader that falls behind the live window gets `Underrun`, skips to the
/// tail, and keeps reading; a reset is reported and reading restarts. Read
/// offsets are contiguous, an underrun moves them on by the bytes skipped,
/// and a reset restarts them at 0.
#[tokio::test]
async fn slow_readers_recover() {
    let t = TestNode::spawn(11, Kind::Tcp).await;
    let options = SwarmOptions {
        live_window: 128,
        ..Default::default()
    };
    let (swarm, _, mut writer) = t
        .node
        .create_swarm(WriterOptions {
            swarm_options: options,
            key: key_from_seed(&[0x66; 32]),
        })
        .await
        .unwrap();
    let mut reader = t.node.open_reader(swarm).await.unwrap();
    let data = stream_bytes(512 * 1024);
    let unit = 64 * 1024;
    writer.write(&data[..unit]).await.unwrap();

    let mut out = vec![0u8; 4096];
    let mut next = 0;
    while next < 3000 {
        let (off, n) = reader.read_at(&mut out[..1000]).await.unwrap();
        assert_eq!(off, next, "offsets are contiguous");
        assert_eq!(&out[..n], &data[off as usize..off as usize + n]);
        next = off + n as u64;
    }

    writer.write(&data[unit..]).await.unwrap();
    let skipped = match reader.read_at(&mut out).await {
        Err(strims_node::ReadError::Underrun { skipped }) => skipped,
        r => panic!("expected an underrun, got {r:?}"),
    };
    assert!(skipped > 0);
    // The reader goes on from the window's tail.
    let (off, n) = reader.read_at(&mut out).await.unwrap();
    assert_eq!(off, next + skipped, "an underrun skips `skipped` bytes");
    assert!(n > 0);
    assert_eq!(&out[..n], &data[off as usize..off as usize + n]);
    let (after, _) = reader.read_at(&mut out).await.unwrap();
    assert_eq!(after, off + n as u64);

    writer.reset().await.unwrap();
    writer.write(&data[..unit]).await.unwrap();
    let mut saw_reset = false;
    for _ in 0..8 {
        match reader.read_at(&mut out).await {
            Err(strims_node::ReadError::Reset) => {
                saw_reset = true;
                break;
            }
            Ok(_) | Err(strims_node::ReadError::Underrun { .. }) => {}
            Err(e) => panic!("{e:?}"),
        }
    }
    assert!(saw_reset);
    let (off, n) = reader.read_at(&mut out).await.unwrap();
    assert_eq!(off, 0, "the new stream starts at 0");
    assert_eq!(&out[..n], &data[..n]);
    t.node.remove_swarm(swarm).await.unwrap();
    assert_eq!(
        reader.read(&mut out).await,
        Err(strims_node::ReadError::Closed)
    );
    assert!(writer.write(b"x").await.is_err());
}

/// Reads `len` bytes from a channel, whatever the frame boundaries.
async fn read_bytes(reader: &mut strims_node::RpcReader, len: usize) -> Vec<u8> {
    let mut got = Vec::new();
    while got.len() < len {
        let chunk = tokio::time::timeout(STEP, reader.next())
            .await
            .expect("bytes in time")
            .expect("channel open")
            .unwrap();
        got.extend(chunk);
    }
    got
}

/// Each side's broker channel carries a byte stream to the other's,
/// beside the RPC pair.
#[tokio::test]
async fn broker_channel_carries_bytes_both_ways() {
    let a = TestNode::spawn(21, Kind::Tcp).await;
    let b = TestNode::spawn(22, Kind::Tcp).await;
    let b_id = a.node.dial(&b.uri(Kind::Tcp)).await.unwrap();
    let a_id = a.node.host_id();
    let mut ab = a.node.take_peer_broker(b_id).await.unwrap();
    let mut ba = b.node.take_peer_broker(a_id).await.unwrap();
    assert_eq!(
        a.node.take_peer_broker(b_id).await.unwrap_err(),
        strims_node::NodeError::RpcTaken
    );

    let data = stream_bytes(100_000);
    for chunk in data.chunks(7_000) {
        ab.writer.send(chunk.to_vec()).await.unwrap();
    }
    assert_eq!(read_bytes(&mut ba.reader, data.len()).await, data);
    ba.writer.send(b"answer".to_vec()).await.unwrap();
    assert_eq!(read_bytes(&mut ab.reader, 6).await, b"answer");

    let m = b.node.metrics().await.unwrap();
    assert_eq!(m.driver.broker_bytes_read, data.len() as u64);
    assert_eq!(m.driver.broker_bytes_written, 6);
}

/// A broker channel whose reader falls behind closes alone: the RPC pair
/// keeps working.
#[tokio::test]
async fn a_broker_overflow_closes_only_the_broker_channel() {
    let a = TestNode::spawn(23, Kind::Tcp).await;
    let b = TestNode::spawn_with(24, Kind::Tcp, |c| c.peer_rpc.inbound_buffer = 16 * 1024).await;
    let b_id = a.node.dial(&b.uri(Kind::Tcp)).await.unwrap();
    let a_id = a.node.host_id();
    let mut ab = a.node.take_peer_broker(b_id).await.unwrap();
    let mut ba = b.node.take_peer_broker(a_id).await.unwrap();

    ab.writer.send(stream_bytes(64 * 1024)).await.unwrap();
    let overflowed = tokio::time::timeout(STEP, async {
        loop {
            match ba.reader.next().await {
                Some(Ok(_)) => {}
                Some(Err(e)) => return Some(e),
                None => return None,
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(overflowed, Some(strims_node::NodeError::RpcOverflow));
    let m = b.node.metrics().await.unwrap();
    assert_eq!((m.driver.broker_overflows, m.driver.rpc_overflows), (1, 0));

    let mut a_rpc = a.node.take_peer_rpc(b_id).await.unwrap();
    let mut b_rpc = b.node.take_peer_rpc(a_id).await.unwrap();
    a_rpc.client.writer.send(b"call".to_vec()).await.unwrap();
    assert_eq!(read_bytes(&mut b_rpc.server.reader, 4).await, b"call");
}
