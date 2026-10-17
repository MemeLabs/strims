// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! vpn ports on one node, sending to itself.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::time::Duration as StdDuration;

use futures_util::{SinkExt, StreamExt};
use strims_crypto::key_from_seed;
use strims_node::{
    HostAddr, NetworkOptions, Node, NodeConfig, NodeError, SystemClock, TokioNet, VpnPort,
};

const STEP: StdDuration = StdDuration::from_secs(10);

/// Polls `f` every 10 ms until it returns `Some`.
async fn eventually<T, F: Future<Output = Option<T>>>(mut f: impl FnMut() -> F) -> T {
    tokio::time::timeout(STEP, async {
        loop {
            if let Some(v) = f().await {
                return v;
            }
            tokio::time::sleep(StdDuration::from_millis(10)).await;
        }
    })
    .await
    .expect("timed out")
}

#[tokio::test]
async fn ports_deliver_drop_and_close() {
    let mut config = NodeConfig::new(key_from_seed(&[1; 32]), 1);
    config.vpn_ports.inbound_buffer = 100;
    let (node, _events) = Node::spawn(config, TokioNet, SystemClock).await.unwrap();
    let net = node
        .add_network(
            &key_from_seed(&[0xee; 32]).public,
            NetworkOptions::default(),
        )
        .await
        .unwrap();

    let VpnPort {
        port,
        mut reader,
        mut writer,
        ..
    } = node.open_vpn_port(net, Some(2000)).await.unwrap();
    assert_eq!(port, 2000);
    // the port is taken, and only its reader frees it
    assert_eq!(
        node.set_vpn_handler(net, 2000).await,
        Err(NodeError::Vpn(strims_vpn::Error::PortInUse(2000)))
    );
    assert!(!node.remove_vpn_handler(net, 2000).await.unwrap());
    let mut ephemeral = node.open_vpn_port(net, None).await.unwrap();
    assert!(ephemeral.port >= 1000);

    let me = HostAddr::new(node.host_id(), 2000);
    writer.send((me, b"hello".to_vec())).await.unwrap();
    let d = reader.next().await.unwrap().unwrap();
    assert_eq!(
        (d.src, d.src_port, d.body.as_slice()),
        (me.host, 2000, &b"hello"[..])
    );

    // past the inbound buffer, messages are dropped
    for _ in 0..3 {
        writer.send((me, vec![7; 60])).await.unwrap();
    }
    let drops = eventually(|| async {
        let m = node.metrics().await.unwrap().driver;
        (m.vpn_port_drops >= 2).then_some(m.vpn_port_drops)
    })
    .await;
    assert_eq!(drops, 2);
    assert_eq!(reader.next().await.unwrap().unwrap().body, vec![7; 60]);

    // vpn refuses what doesn't fit a frame
    writer.send((me, vec![0; 70_000])).await.unwrap();
    eventually(|| async {
        (node.metrics().await.unwrap().driver.vpn_send_errors == 1).then_some(())
    })
    .await;

    // dropping the reader closes the port
    drop(reader);
    let again = eventually(|| async { node.open_vpn_port(net, Some(2000)).await.ok() }).await;
    assert_eq!(
        writer.send((me, b"late".to_vec())).await,
        Err(NodeError::Closed)
    );

    // removing the network ends the readers
    node.remove_network(net).await.unwrap();
    assert!(ephemeral.reader.next().await.is_none());
    drop(again);
}

/// vpn's own services hold their ports, and the directory's snippet port
/// is free for the app to open by number.
#[tokio::test]
async fn service_ports_are_taken_and_the_snippet_port_opens() {
    let config = NodeConfig::new(key_from_seed(&[2; 32]), 2);
    let (node, _events) = Node::spawn(config, TokioNet, SystemClock).await.unwrap();
    let net = node
        .add_network(
            &key_from_seed(&[0xee; 32]).public,
            NetworkOptions::default(),
        )
        .await
        .unwrap();
    for port in [10, 11, 12] {
        assert_eq!(
            node.open_vpn_port(net, Some(port)).await.err(),
            Some(NodeError::Vpn(strims_vpn::Error::PortInUse(port))),
            "port {port}"
        );
    }
    let snippets = node
        .open_vpn_port(net, Some(strims_directory::SNIPPET_PORT))
        .await
        .unwrap();
    assert_eq!(snippets.port, 14);
    assert_eq!(
        node.open_vpn_port(net, Some(14)).await.err(),
        Some(NodeError::Vpn(strims_vpn::Error::PortInUse(14)))
    );
}
