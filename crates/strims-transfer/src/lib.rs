// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Transfers: the swarms a node runs with its peers, the channels each
//! runs on, and the search for peers to run them with.
//!
//! A transfer is a swarm under a salt, named by a [`TransferId`]. The app
//! adds it ([`TransferControl::add`]) and publishes it on networks
//! ([`TransferControl::publish`]). Peers that share one of those networks
//! negotiate a ppspp channel for it over their peer RPC channel pair, as
//! `strims.transfer.v1.TransferPeer`:
//!
//! - `Announce { id, channel }` tells the peer that this side publishes
//!   `id` on a network they share, and that its frames for it will arrive
//!   on `channel`. A peer that publishes `id` on a network it shares with
//!   the caller runs the swarm and answers with its own channel; any other
//!   peer answers without one, and announces in turn once it publishes
//!   `id`. Either answer order binds the same channel pair.
//! - `Close { id }` stops the peer's channel. A later announce restarts it.
//!
//! Each side announces every published transfer to every peer of its
//! networks as they link and join, so the side that publishes second
//! binds the channel. Transfers stop and close with a peer once they
//! share no network.
//!
//! With [`Config::discovery`], publishing also advertises the transfer in
//! the network's peer index under (swarm id, salt), and searches the
//! index now and every [`Config::search_interval`] for hosts to link to.
//!
//! # Running
//!
//! [`TransferControl`] runs this for a node, which the driver supplies
//! through [`Host`]. The driver reports peers as they link and unlink
//! ([`TransferControl::add_peer`]), the networks they join and leave, and
//! ppspp failures, and runs [`TransferControl::run`] as a local task.
//!
//! The control owns the swarm channels of the swarms it transfers:
//!
//! - remove a transfer before removing its swarm from the node;
//! - don't run a transferred swarm with a peer through the node directly.
//!
//! # Peers
//!
//! Any peer may be hostile. Its requests only answer or stop peer
//! transfers this side created by announcing to it, so they can't make
//! the control hold more state.

mod control;
mod host;
mod id;
mod peer;
mod state;

use strims_base::time::Duration;
use strims_rpc::RetryPolicy;

pub use control::TransferControl;
pub use host::Host;
pub use id::TransferId;

/// How transfers run.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Advertise published transfers in their networks' peer indexes and
    /// search there for hosts to link to.
    pub discovery: bool,
    /// How often each published transfer is searched for on each network.
    pub search_interval: Duration,
    /// How long a search waits for hosts.
    pub search_timeout: Duration,
    /// The most hosts one search returns, and asks the node to link to.
    pub max_search_hosts: usize,
    /// How long to wait before asking the node to link to a host again.
    pub connect_backoff: Duration,
    /// The most hosts held in the connect backoff.
    pub max_contacted: usize,
    /// Retries for `Announce` and `Close`.
    pub call_policy: RetryPolicy,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            discovery: true,
            search_interval: Duration::from_secs(30),
            search_timeout: Duration::from_secs(10),
            max_search_hosts: 16,
            connect_backoff: Duration::from_secs(60),
            max_contacted: 1024,
            call_policy: RetryPolicy::peer(),
        }
    }
}

/// What a [`TransferControl`] has done.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Announces sent to peers, and those answered with a channel.
    pub announces_sent: u64,
    pub announces_accepted: u64,
    /// Announces from peers, and those answered with a channel.
    pub announces_received: u64,
    pub announces_answered: u64,
    /// Announce and close calls that failed.
    pub call_errors: u64,
    /// Swarm channels started and stopped.
    pub runs: u64,
    pub stops: u64,
    /// Swarm channels the node refused to start.
    pub run_errors: u64,
    /// Closes sent to peers, and received from them.
    pub closes_sent: u64,
    pub closes_received: u64,
    /// Peer index records published.
    pub index_publishes: u64,
    /// Peer index searches, and those that failed or timed out.
    pub searches: u64,
    pub search_errors: u64,
    /// Requests to the node to link to a found host.
    pub connects: u64,
    /// Failed node calls other than swarm starts.
    pub host_errors: u64,
}
