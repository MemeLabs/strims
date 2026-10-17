// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A fake world for app-layer controls' integration tests: fake nodes, each
//! one process of a profile with its network, transfer and directory
//! controls, sharing one world. Time is tokio's paused clock.
//!
//! A node's profile store is a real
//! [`ReplicatedStore`](strims_dao::ReplicatedStore) over
//! [`MemoryKv`](strims_kv::MemoryKv), and its controls are the real ones,
//! on a [`FakeHost`]:
//!
//! - **Links.** In-memory pipes carry each linked peer's RPC channel pair
//!   and broker channel, for network membership.
//! - **Vpn.** The world carries messages between any two nodes' ports on
//!   the same network, whether or not they are linked, drops messages to
//!   closed ports, and gives ports of a chosen number. A node can be made
//!   to open a chosen port under another number.
//! - **Hash table.** One table for all networks holds service addresses by
//!   (key, salt). A publish replaces the address under its (key, salt), and
//!   a node unpublishes only an address it published.
//! - **Swarms.** Every node's swarm with the same id and label (the label
//!   stands for the transfer salt, which tells two swarms with one id
//!   apart) shares one log in the world. A log keeps the last window of the
//!   stream (the swarm's live window in bytes) and its epoch. Creating a
//!   swarm, or resetting its writer, starts a new epoch at offset 0. A
//!   reader starts at the window's tail, reads at absolute offsets, and
//!   fails once with a discontinuity when the epoch changes or the window
//!   passed it. Removing a swarm closes its readers and writer.
//!
//! Not modelled:
//!
//! - latency, loss, reordering and bandwidth: pipes and vpn ports deliver
//!   at once and in order;
//! - vpn routing, hops, encryption and signatures: a delivery carries no
//!   hops and no verified signature;
//! - transfers between nodes: swarms share their log, discovery finds no
//!   one, and peer connections and swarm runs do nothing.
//!
//! Tests run in [`run`]. An app crate's tests build their control from a
//! [`Node`]'s parts (its host, store, profile, controls, timer and a
//! [`Node::rng`] fork), drive it with [`Node::spawn`], and find its
//! services with [`World::address`].

// Test support: a failed setup step fails the test that ran it.
#![allow(clippy::unwrap_used, clippy::panic)]

mod exec;
mod host;
mod node;
mod pipe;
mod world;

pub use exec::{T0, TokioTimer, drain, run, settle, sleep_secs, wait_for};
pub use host::{FakeHost, NodeState, SwarmEntry};
pub use node::{
    Directory, Node, frontend_client, link, node_in, node_sharing, owned_network,
    owned_network_with, signed_member,
};
pub use pipe::{Closed, Pipe, PipeState, Reader, Writer};
pub use world::{
    FakeReader, FakeWriter, Log, LogState, Port, PortState, VpnReader, VpnWriter, World, WorldState,
};
