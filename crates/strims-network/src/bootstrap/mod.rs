// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Bootstrap: how a node reaches its first peers, and how a network is
//! handed to a peer that runs no member of it yet.
//!
//! The profile's store holds bootstrap clients: WebSocket URLs of nodes to
//! link to. An invitation carries one, so a node that joins a network
//! from it links to the node that invited it. [`BootstrapControl`] dials
//! every client when it starts and as the store gains them, and dials
//! again, with backoff, when a client's link goes. A client whose URL is
//! one of this node's own is never dialed.
//!
//! Every linked peer is served `strims.network.v1.bootstrap.PeerService`
//! and asked once per link whether it accepts published networks
//! (`GetPublishEnabled`), which this device's bootstrap config decides.
//! The CA of a network can publish the network to a peer that does: it
//! signs a membership for the peer's profile key under the network's root
//! and sends it with `Publish`, and the peer adds the network to its
//! store. Such a membership has no alias, and the CA keeps no record of
//! it.
//!
//! [`bind_frontend`] serves the UI's bootstrap calls,
//! `strims.network.v1.bootstrap.BootstrapFrontend`.
//!
//! # Peers
//!
//! A published certificate is accepted only for this profile's key, under
//! a network root that signed it directly, within
//! [`Config::max_chain_depth`], for a network the store doesn't hold yet,
//! and while the store holds fewer than [`Config::max_networks`].

mod control;
mod frontend;
mod schedule;
mod service;

use std::future::Future;

use strims_base::time::Duration;
use strims_kademlia::ID;
use strims_rpc::RetryPolicy;

pub use control::BootstrapControl;
pub use frontend::{BootstrapFrontend, bind_frontend};

/// Why a dial failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// Dialing again may succeed.
    Retry(String),
    /// Dialing the URL again can't succeed.
    Permanent(String),
}

/// What the bootstrap control needs from the node it runs on.
pub trait Host: 'static {
    /// Links to the node at a client's `uri` and waits for the handshake.
    /// Returns the peer's host id.
    fn dial(&self, uri: &str) -> impl Future<Output = Result<ID, Failure>>;

    /// The URIs other nodes link to this node at.
    fn own_uris(&self) -> Vec<String>;
}

/// How bootstrap runs.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// The wait after a client's first failed dial, doubling with each
    /// failure up to `max_backoff`. Waits are jittered between half and
    /// all of it.
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    /// The least wait before redialing a client whose link went, so a
    /// replaced link isn't dialed again.
    pub relink_grace: Duration,
    /// A client's link that lasts this long resets its backoff.
    pub stable_after: Duration,
    /// How long a dial's host has to appear as a peer.
    pub confirm_timeout: Duration,
    /// The longest a dial takes.
    pub dial_timeout: Duration,
    /// The most clients dialed; the rest, by id, are left out.
    pub max_clients: usize,
    /// Dials at once.
    pub max_dials: usize,
    /// The longest client URL accepted.
    pub max_url_bytes: usize,
    /// The most networks the store may hold for a peer to publish
    /// another.
    pub max_networks: usize,
    /// The longest published certificate chain accepted.
    pub max_chain_depth: usize,
    /// Retries for calls to peers.
    pub call_policy: RetryPolicy,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            initial_backoff: Duration::SECOND,
            max_backoff: Duration::from_secs(300),
            relink_grace: Duration::from_secs(2),
            stable_after: Duration::from_secs(60),
            confirm_timeout: Duration::from_secs(10),
            dial_timeout: Duration::from_secs(60),
            max_clients: 64,
            max_dials: 4,
            max_url_bytes: 2048,
            max_networks: 256,
            max_chain_depth: 8,
            call_policy: RetryPolicy::peer(),
        }
    }
}
