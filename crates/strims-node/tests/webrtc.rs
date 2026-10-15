// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! WebRTC links made by peer exchange, on real localhost sockets (UDP for
//! WebRTC, TCP and WebSocket for the relay).
//!
//! a and c have no listeners, only WebRTC (as browser nodes): a dials b
//! over TCP, c dials b over WebSocket. Then
//!
//! - `connect_peer` from a to c trades session descriptions through b, and
//!   a direct WebRTC link comes up (`PeerAdded` with `LinkKind::WebRtc` on
//!   both sides, a's exchange `Connected`);
//! - b shuts down, so a and c share only the new link;
//! - vpn messages go both ways over it in one hop;
//! - a publishes a 4 MiB ppspp stream that c downloads over it,
//!   byte-exact from offset 0.
//!
//! Plus the configuration errors.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::time::Duration as StdDuration;

use strims_base::rng::det_rng;
use strims_crypto::key_from_seed;
use strims_node::{
    DeliveryMode, ExchangeOutcome, HostId, LinkKind, NetworkId, NetworkOptions, Node, NodeConfig,
    NodeError, NodeEvent, NodeEvents, SwarmId, SwarmOptions, SystemClock, TokioNet, WebRtcConfig,
    WriterOptions,
};

const VPN_PORT: u16 = 10_000;
const APP_PORT: u16 = 20_000;
const STREAM_LEN: usize = 4 * 1024 * 1024;
const STEP: StdDuration = StdDuration::from_secs(20);

struct TestNode {
    node: Node,
    net: NetworkId,
    events: NodeEvents,
    seen: Vec<NodeEvent>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    /// TCP and WebSocket listeners, no WebRTC.
    Relay,
    /// WebRTC only, no listeners.
    Browser,
}

impl TestNode {
    async fn spawn(i: u8, role: Role) -> TestNode {
        TestNode::spawn_with(i, role, |_| {}).await
    }

    async fn spawn_with(i: u8, role: Role, f: impl FnOnce(&mut NodeConfig)) -> TestNode {
        let mut config = NodeConfig::new(key_from_seed(&[i; 32]), u64::from(i));
        config.label = format!("node-{i}");
        match role {
            Role::Relay => {
                config.tcp_listen = Some("127.0.0.1:0".to_owned());
                config.ws_listen = Some("127.0.0.1:0".to_owned());
            }
            Role::Browser => {
                config.webrtc = WebRtcConfig::local(["127.0.0.1:0".to_owned()]);
            }
        }
        config.allow_insecure_ws = true;
        config.max_peers = 16;
        f(&mut config);
        let (node, events) = Node::spawn(config, TokioNet, SystemClock).await.unwrap();
        if role == Role::Browser {
            assert_eq!(node.info().webrtc_addrs.len(), 1);
            assert_eq!(node.tcp_uri(), None);
        }
        let net = node
            .add_network(
                &key_from_seed(&[0xee; 32]).public,
                NetworkOptions {
                    auto_bind_port: Some(VPN_PORT),
                },
            )
            .await
            .unwrap();
        node.set_vpn_handler(net, APP_PORT).await.unwrap();
        TestNode {
            node,
            net,
            events,
            seen: Vec::new(),
        }
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

    async fn peer_added(&mut self, who: &str, peer: HostId) -> strims_node::PeerInfo {
        match self
            .wait(
                who,
                |e| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == peer),
            )
            .await
        {
            NodeEvent::PeerAdded(p) => p,
            _ => unreachable!(),
        }
    }

    async fn message(&mut self, who: &str, body: &[u8]) -> strims_node::Delivery<u16> {
        match self
            .wait(
                who,
                |e| matches!(e, NodeEvent::Message(d) if d.body == body),
            )
            .await
        {
            NodeEvent::Message(d) => d,
            _ => unreachable!(),
        }
    }
}

fn stream_bytes(len: usize) -> Vec<u8> {
    let mut rng = det_rng(4242);
    let mut b = vec![0u8; len];
    rand_core::Rng::fill_bytes(&mut rng, &mut b);
    b
}

#[tokio::test]
async fn webrtc_link_through_peer_exchange() {
    let mut a = TestNode::spawn(1, Role::Browser).await;
    let mut b = TestNode::spawn(2, Role::Relay).await;
    let mut c = TestNode::spawn(3, Role::Browser).await;
    let (a_id, b_id, c_id) = (a.node.host_id(), b.node.host_id(), c.node.host_id());

    // a and c meet only through b.
    assert_eq!(a.node.dial(b.node.tcp_uri().unwrap()).await.unwrap(), b_id);
    assert_eq!(c.node.dial(b.node.ws_uri().unwrap()).await.unwrap(), b_id);
    a.peer_added("a: b", b_id).await;
    c.peer_added("c: b", b_id).await;
    b.peer_added("b: a", a_id).await;
    b.peer_added("b: c", c_id).await;

    // Peer exchange: a offers, c answers, both through b; only WebRTC
    // descriptions are on offer.
    let started = tokio::time::Instant::now();
    assert!(a.node.connect_peer(a.net, c_id).await.unwrap());
    let pa = a.peer_added("a: c over webrtc", c_id).await;
    assert_eq!((pa.kind, pa.dialed), (LinkKind::WebRtc, true));
    assert_eq!(pa.profile_key, key_from_seed(&[3; 32]).public);
    let pc = c.peer_added("c: a over webrtc", a_id).await;
    assert_eq!((pc.kind, pc.dialed), (LinkKind::WebRtc, false));
    match a
        .wait(
            "a: exchange",
            |e| matches!(e, NodeEvent::ExchangeClosed { host_id, .. } if *host_id == c_id),
        )
        .await
    {
        NodeEvent::ExchangeClosed { outcome, .. } => {
            assert_eq!(outcome, ExchangeOutcome::Connected)
        }
        _ => unreachable!(),
    }
    eprintln!("webrtc link up in {:?}", started.elapsed());

    // Only the new link is left.
    b.node.shutdown().await.unwrap();
    a.wait(
        "a: b gone",
        |e| matches!(e, NodeEvent::PeerRemoved { host_id } if *host_id == b_id),
    )
    .await;
    c.wait(
        "c: b gone",
        |e| matches!(e, NodeEvent::PeerRemoved { host_id } if *host_id == b_id),
    )
    .await;
    for n in [&a, &c] {
        let peers = n.node.peers().await.unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].kind, LinkKind::WebRtc);
    }

    // vpn both ways, directly.
    a.node
        .send(a.net, c_id, APP_PORT, APP_PORT, b"a to c".to_vec())
        .await
        .unwrap();
    let d = c.message("c: message", b"a to c").await;
    assert_eq!((d.src, d.hops), (a_id, 1));
    c.node
        .send(c.net, a_id, APP_PORT, APP_PORT, b"c to a".to_vec())
        .await
        .unwrap();
    let d = a.message("a: message", b"c to a").await;
    assert_eq!((d.src, d.hops), (c_id, 1));

    // A 4 MiB stream a → c over the data channel.
    let options = SwarmOptions {
        live_window: 1 << 13,
        delivery_mode: DeliveryMode::MANDATORY,
        ..Default::default()
    };
    let swarm_key = key_from_seed(&[0x55; 32]);
    let (sa, id, mut writer) = a
        .node
        .create_swarm(WriterOptions {
            swarm_options: options.clone(),
            key: swarm_key.clone(),
        })
        .await
        .unwrap();
    assert_eq!(id, SwarmId::new(&swarm_key.public));
    let sc = c.node.add_swarm(id, options).await.unwrap();
    let mut reader = c.node.open_reader(sc).await.unwrap();
    a.node.run_swarm(c_id, sa, 1, 1).await.unwrap();
    c.node.run_swarm(a_id, sc, 1, 1).await.unwrap();

    let data = stream_bytes(STREAM_LEN);
    let started = tokio::time::Instant::now();
    let write = {
        let data = data.clone();
        tokio::spawn(async move {
            for chunk in data.chunks(64 * 1024) {
                writer.write(chunk).await.unwrap();
            }
            writer.flush().await.unwrap();
            writer
        })
    };
    let mut out = vec![0u8; STREAM_LEN];
    tokio::time::timeout(StdDuration::from_secs(120), reader.read_exact(&mut out))
        .await
        .expect("stream timed out")
        .unwrap();
    let elapsed = started.elapsed();
    let _writer = write.await.unwrap();
    assert!(out == data, "stream bytes differ");

    let ma = a.node.metrics().await.unwrap();
    let mc = c.node.metrics().await.unwrap();
    eprintln!(
        "4 MiB over webrtc in {elapsed:?}; a wrote {} link bytes in {} datagrams ({} udp bytes), c read {} link bytes",
        ma.driver.bytes_written,
        ma.driver.udp_datagrams_written,
        ma.driver.udp_bytes_written,
        mc.driver.bytes_read
    );
    for (name, m) in [("a", &ma), ("c", &mc)] {
        assert_eq!(m.links.len(), 1, "{name}");
        assert_eq!(m.links[0].kind, LinkKind::WebRtc, "{name}");
        assert_eq!(m.driver.webrtc_connections, 1, "{name}");
        assert_eq!(m.driver.webrtc_opened, 1, "{name}");
        assert_eq!(m.driver.transfer_errors, 0, "{name}");
        assert_eq!(m.driver.udp_send_dropped, 0, "{name}");
    }
    // The stream (plus ppspp's overhead) went over the data channel.
    assert!(ma.links[0].metrics.write_bytes > STREAM_LEN as u64);
    assert!(ma.driver.udp_bytes_written > STREAM_LEN as u64);

    // Closing the link closes the connection on both sides.
    assert!(a.node.close_peer(c_id).await.unwrap());
    c.wait(
        "c: a gone",
        |e| matches!(e, NodeEvent::PeerRemoved { host_id } if *host_id == a_id),
    )
    .await;
    let mc = c.node.metrics().await.unwrap();
    assert_eq!(mc.driver.webrtc_closed, 1);
    assert!(mc.links.is_empty());
    a.node.shutdown().await.unwrap();
    c.node.shutdown().await.unwrap();
}

/// A WebRTC link stops writing at `max_buffered_amount`, so a fast
/// publisher's data waits in QoS, and the stream still arrives whole.
#[tokio::test]
async fn webrtc_links_cap_the_buffered_amount() {
    const CAP: usize = 32 * 1024;
    const LEN: usize = 2 * 1024 * 1024;
    let mut a =
        TestNode::spawn_with(1, Role::Browser, |c| c.webrtc.max_buffered_amount = CAP).await;
    let b = TestNode::spawn(2, Role::Relay).await;
    let mut c = TestNode::spawn(3, Role::Browser).await;
    let (a_id, b_id, c_id) = (a.node.host_id(), b.node.host_id(), c.node.host_id());
    assert_eq!(a.node.dial(b.node.tcp_uri().unwrap()).await.unwrap(), b_id);
    assert_eq!(c.node.dial(b.node.ws_uri().unwrap()).await.unwrap(), b_id);
    assert!(a.node.connect_peer(a.net, c_id).await.unwrap());
    assert_eq!(a.peer_added("a: c", c_id).await.kind, LinkKind::WebRtc);
    c.peer_added("c: a", a_id).await;

    let options = SwarmOptions {
        live_window: 1 << 12,
        delivery_mode: DeliveryMode::MANDATORY,
        ..Default::default()
    };
    let swarm_key = key_from_seed(&[0x56; 32]);
    let (sa, id, mut writer) = a
        .node
        .create_swarm(WriterOptions {
            swarm_options: options.clone(),
            key: swarm_key,
        })
        .await
        .unwrap();
    let sc = c.node.add_swarm(id, options).await.unwrap();
    let mut reader = c.node.open_reader(sc).await.unwrap();
    a.node.run_swarm(c_id, sa, 1, 1).await.unwrap();
    c.node.run_swarm(a_id, sc, 1, 1).await.unwrap();

    let data = stream_bytes(LEN);
    let write = {
        let data = data.clone();
        tokio::spawn(async move {
            for chunk in data.chunks(64 * 1024) {
                writer.write(chunk).await.unwrap();
            }
            writer.flush().await.unwrap();
            writer
        })
    };
    let sample = {
        let node = a.node.clone();
        tokio::spawn(async move {
            let (mut max, mut blocked, mut samples) = (0, 0, 0);
            loop {
                let Ok(m) = node.metrics().await else { break };
                let Some(l) = m.links.iter().find(|l| l.kind == LinkKind::WebRtc) else {
                    break;
                };
                max = max.max(l.buffered_amount);
                blocked += usize::from(l.write_blocked);
                samples += 1;
                if l.metrics.write_bytes > LEN as u64 {
                    break;
                }
                tokio::time::sleep(StdDuration::from_millis(1)).await;
            }
            (max, blocked, samples)
        })
    };
    let mut out = vec![0u8; LEN];
    tokio::time::timeout(StdDuration::from_secs(120), reader.read_exact(&mut out))
        .await
        .expect("stream timed out")
        .unwrap();
    let _writer = write.await.unwrap();
    assert!(out == data, "stream bytes differ");
    let (max, blocked, samples) = sample.await.unwrap();
    eprintln!(
        "buffered amount: max {max} of {CAP} over {samples} samples, {blocked} samples at the cap"
    );
    assert!(max <= CAP, "{max} > {CAP}");
    assert!(blocked > 0, "the cap never bound");
    for n in [&a, &b, &c] {
        n.node.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn webrtc_config_errors() {
    let config = |webrtc: WebRtcConfig| {
        let mut c = NodeConfig::new(key_from_seed(&[9; 32]), 9);
        c.webrtc = webrtc;
        c
    };
    // Enabled without a socket.
    let err = Node::spawn(config(WebRtcConfig::local([])), TokioNet, SystemClock)
        .await
        .unwrap_err();
    assert!(matches!(err, NodeError::Config(_)), "{err:?}");
    // A wildcard socket advertises nothing without a host ip...
    let err = Node::spawn(
        config(WebRtcConfig::local(["0.0.0.0:0".to_owned()])),
        TokioNet,
        SystemClock,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, NodeError::Config(_)), "{err:?}");
    // ...and works with one (the node's public ip by default).
    let mut c = config(WebRtcConfig::local(["0.0.0.0:0".to_owned()]));
    c.public_ip = Some("127.0.0.1".parse().unwrap());
    let (node, _events) = Node::spawn(c, TokioNet, SystemClock).await.unwrap();
    assert!(node.info().webrtc_addrs[0].ip().is_unspecified());
    node.shutdown().await.unwrap();
}
