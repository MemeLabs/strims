// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Starting a node in the worker: [`spawn`] builds a core node on
//! [`WasmIo`] and runs its task, and [`browser_config`] is the
//! configuration a browser node starts from.

use strims_node_core::{Node, NodeConfig, NodeError, NodeEvents, WebRtcConfig};
use strims_proto::types::Key;
use web_sys::MessagePort;

use crate::io::WasmIo;
use crate::runtime;

/// What [`browser_config`] sets `NodeConfig::node_platform` to.
pub const NODE_PLATFORM: &str = "web/wasm32";

/// The most peer connections a browser node keeps at once. Browsers cap
/// them per page (Chrome at 500), and each holds an ICE agent, DTLS and
/// SCTP state in the window.
pub const MAX_PEER_CONNECTIONS: usize = 256;

/// Builds a node of `config`, signalling WebRTC with the window over
/// `signal`, and spawns its task. The node runs until
/// [`Node::shutdown`] or until every handle is dropped.
///
/// `config` describes a browser node (see [`browser_config`]): no
/// listeners, and with WebRTC enabled, connections the window makes. Only
/// the WebRTC settings that a browser's peer connections take apply: the
/// ICE servers, the gather and connect timeouts, the connection limit and
/// `max_buffered_amount`. `signal` keeps working for later nodes: each
/// node's connection ids are distinct, and the newest node takes the
/// port's messages.
pub fn spawn(config: NodeConfig, signal: MessagePort) -> Result<(Node, NodeEvents), NodeError> {
    let (setup, io) = WasmIo::new(&config, signal)?;
    let (node, events, task) = strims_node_core::start(&config, setup, io)?;
    // Detached: the task ends with the node.
    drop(runtime::spawn(task.run()));
    Ok((node, events))
}

/// A browser node of `profile_key`: `NodeConfig::new`'s defaults (no
/// listeners, no `ws://` dials), plus WebRTC over the window's peer
/// connections with the default ICE servers and at most
/// [`MAX_PEER_CONNECTIONS`], and [`NODE_PLATFORM`].
pub fn browser_config(profile_key: Key, rng_seed: u64) -> NodeConfig {
    let mut webrtc = WebRtcConfig {
        enabled: true,
        ..WebRtcConfig::default()
    };
    webrtc.endpoint.max_connections = MAX_PEER_CONNECTIONS;
    NodeConfig {
        webrtc,
        node_platform: NODE_PLATFORM.to_owned(),
        ..NodeConfig::new(profile_key, rng_seed)
    }
}
