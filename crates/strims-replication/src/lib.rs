// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Replication of a profile's store between the profile's devices.
//!
//! Every device of a profile is a replica of its store
//! ([`strims_dao::replication`]). Devices find each other through the hash
//! table of each network they belong to, where each publishes its host id
//! under its device id ([`host::discovery_salt`]), signed with the profile
//! key. They replicate over the peer RPC channel pair of every link
//! between them, as `strims.replication.v1.ReplicationPeer`:
//!
//! 1. `Open` exchanges replica ids and the receiver's checkpoint. A
//!    deleted device is refused.
//! 2. A device short of record ids asks for some (`AllocateProfileIDs`).
//! 3. The sender sends the logs the receiver's checkpoint lacks (`Sync`),
//!    or everything to a replica that has seen no other (`Bootstrap`),
//!    split to fit messages.
//! 4. New logs follow as they are written here or applied from another
//!    device.
//!
//! [`ReplicationControl`] runs this for a store, and the driver supplies
//! the node through [`Host`]. [`ReplicationFrontend`] serves the UI's
//! pairing and checkpoint calls.
//!
//! # Peers
//!
//! Only peers holding the profile's key are served, so peers are the
//! profile's own devices and trusted with its contents. Their requests
//! are still bounded ([`limits`]), so a faulty device can't make this one
//! hold unbounded state.

mod control;
mod frontend;
pub mod host;
pub mod limits;
mod peer;
pub mod queue;

use strims_base::time::Duration;
use strims_rpc::RetryPolicy;

pub use control::ReplicationControl;
pub use frontend::ReplicationFrontend;
pub use host::Host;
pub use peer::{MAX_ALLOCATION, PeerError};

/// How replication runs.
#[derive(Clone, Debug)]
pub struct Config {
    /// How often to look up the profile's devices and publish this one.
    pub scan_interval: Duration,
    /// How long to wait before asking the node to link to a host again.
    pub connect_backoff: Duration,
    /// How long to wait before replicating to a peer again after it failed.
    pub peer_retry: Duration,
    /// How long checkpoint changes settle before logs are collected.
    pub gc_debounce: Duration,
    /// The most encoded bytes of logs or records in one request.
    pub message_budget: usize,
    /// The free id count at or below which a device asks a peer for ids.
    pub allocate_below: u64,
    /// The most logs queued for one peer before it needs a resync.
    pub push_queue: usize,
    /// The most devices looked up per scan.
    pub max_scan_devices: usize,
    /// The most hosts one lookup returns.
    pub max_lookup_results: usize,
    /// Retries for `Open` and `AllocateProfileIDs`.
    pub call_policy: RetryPolicy,
    /// Retries for `Sync` and `Bootstrap`, whose requests are large and
    /// whose receivers apply them before answering.
    pub transfer_policy: RetryPolicy,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            scan_interval: Duration::from_secs(10),
            connect_backoff: Duration::from_secs(60),
            peer_retry: Duration::from_secs(10),
            gc_debounce: Duration::SECOND,
            message_budget: 448 * 1024,
            allocate_below: 10_000,
            push_queue: 64,
            max_scan_devices: 64,
            max_lookup_results: 8,
            call_policy: RetryPolicy::peer(),
            transfer_policy: RetryPolicy {
                attempt_timeout: Duration::from_secs(20),
                retries: 1,
                delay: Duration::SECOND,
                backoff: 2,
                deadline: Duration::from_secs(60),
            },
        }
    }
}

/// What a [`ReplicationControl`] has done.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Times this device took the replication lease.
    pub leases: u64,
    /// Peers opened.
    pub opens: u64,
    /// Bootstrap requests sent.
    pub bootstraps_sent: u64,
    /// Bootstrap requests applied.
    pub bootstraps_received: u64,
    /// Records sent in bootstraps.
    pub events_sent: u64,
    /// Records applied from bootstraps.
    pub events_received: u64,
    /// Sync requests sent.
    pub syncs_sent: u64,
    /// Sync requests applied.
    pub syncs_received: u64,
    /// Logs sent.
    pub logs_sent: u64,
    /// Logs applied.
    pub logs_received: u64,
    /// Requests from peers refused.
    pub refused: u64,
    /// Replications to a peer that stopped on an error.
    pub peer_errors: u64,
    /// Store failures that stopped the replicator.
    pub store_errors: u64,
    /// Log collections run.
    pub gc_runs: u64,
    /// Logs collected.
    pub logs_collected: u64,
    /// Discovery records published.
    pub publishes: u64,
    /// Discovery lookups made.
    pub lookups: u64,
    /// Links asked of the node.
    pub connects: u64,
}
