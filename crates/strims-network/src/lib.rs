// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Network membership: which networks a node runs, which of them each
//! linked peer shares, and the bindings that join the peers to them.
//!
//! A profile's networks come from its store. The node joins each one, and
//! every linked peer negotiates the networks it shares with the node over
//! its peer RPC channel pair, as `strims.network.v1.NetworkPeer`, and its
//! network broker channel:
//!
//! 1. Each side calls `Negotiate { key_count }` once per run.
//! 2. The two sides find their shared network keys by private set
//!    intersection (`strims-mpc`'s PSZ) on the broker channel. The side
//!    with more keys is the PSZ sender, and on a tie the side with the
//!    greater host id; only the receiver learns the shared keys.
//! 3. The receiver calls `Open` with a binding for each shared key it has
//!    no link for: a vnic port it reserved and its membership certificate.
//!    The sender answers with its own `Open`, one binding per received
//!    binding in the same order.
//! 4. A link opens, binding the peer to the network on the two ports, once
//!    both certificates are trusted: signed by the network's root and not
//!    expired. A certificate issued under an invitation isn't, until its
//!    holder renews it at the network's CA; `UpdateCertificate` reports a
//!    renewed one.
//! 5. `Close { key }` ends a link when a side leaves the network.
//!
//! Runs start when a peer links and when the node joins a network. A run
//! that stalls or fails backs off and runs again with fresh PSI seeds.
//!
//! # PSI
//!
//! A PSI session takes tens of milliseconds of CPU, and the control's
//! thread also does the node's latency-sensitive I/O. The sessions run on
//! a [`psi::PsiEngine`]:
//!
//! - [`psi::InlinePsi`], the default, computes on the control's thread,
//!   inside the broker bytes' calls. It is deterministic, so a node's
//!   runs replay exactly from its seeds.
//! - [`psi::RemotePsi`], set with [`NetworkControl::with_psi`], runs them
//!   on a [`psi::PsiServer`] the driver hosts on another thread or worker.
//!   Its sessions complete asynchronously: each reply wakes the control,
//!   which sends the session's bytes and takes its outcome as it does an
//!   inline one.
//!
//! # Certificates
//!
//! A profile's membership of a network starts from [`membership`]: a
//! network it creates, whose CA it runs ([`ca`]), or an invitation from a
//! member. The control renews its certificates at the network's CA, over
//! vpn RPC or through a linked peer ([`services`]), stores them in the
//! profile's record of the network, and reports the outcomes as the
//! user's notifications. Under its membership, the host presents
//! [`NetworkControl::host_certificate`] to the network's vpn services, and
//! [`NetworkControl::find_certificate`] looks up the certificates the CA
//! issued, such as a member's by its alias.
//!
//! # Running
//!
//! [`NetworkControl`] runs this for a profile's node, which the driver
//! supplies through [`Host`]. It runs the networks the profile's store
//! holds and follows its changes. The driver reports peers as they link
//! and unlink ([`NetworkControl::add_peer`]), and runs
//! [`NetworkControl::run`] as a local task. The control reports the
//! networks it runs and their bound peers as [`NetworkEvent`]s: a network
//! starts when the store holds it, and is joined once the node has joined
//! it and its services have started. Services built on a network follow
//! its joins.
//!
//! # Bootstrap
//!
//! [`bootstrap`] links the node to the store's bootstrap clients, the
//! nodes an invitation points its holder to, and hands networks to peers
//! that accept them.
//!
//! # Peers
//!
//! Any peer may be hostile. What a peer's requests and broker bytes can
//! make the control hold is bounded by [`Config`]: its key count, its
//! broker bytes, its bindings and their certificate chains. Runs a peer
//! starts are paced, and PSI sessions run at most
//! [`Config::max_sessions`] at a time. A PSI engine's messages are bounded
//! as the [`psi`] module describes.

pub mod bootstrap;
pub mod ca;
mod control;
mod events;
pub mod frontend;
mod host;
pub mod membership;
mod peer;
pub mod psi;
pub mod services;
mod state;
#[cfg(test)]
mod state_test;

use strims_base::time::Duration;
use strims_rpc::RetryPolicy;

pub use control::{NetworkControl, NetworkInfo};
pub use events::NetworkEvent;
pub use frontend::{NetworkFrontend, bind_frontend};
pub use host::Host;

/// How network membership runs.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// How long a local trigger (a peer linking, a network joining) waits
    /// for others before a run starts.
    pub debounce: Duration,
    /// How long a run waits for the answer to its `Negotiate` and for the
    /// peer's.
    pub negotiate_timeout: Duration,
    /// How long a PSI session waits without broker bytes in either
    /// direction. Between the peer's bytes and the session's answer, both
    /// sides' engines compute: tens of milliseconds a message, whether
    /// inline or remote, far below this. An engine that stops answering
    /// stalls the run as a silent peer does.
    pub stall_timeout: Duration,
    /// The longest a run's PSI takes, from its start.
    pub run_timeout: Duration,
    /// How long a run waits for ports and for the peer's `Open`.
    pub exchange_timeout: Duration,
    /// The wait before running again after the receiver's cuckoo table
    /// failed.
    pub retry_delay: Duration,
    /// The backoff after a failed run, doubling up to `max_backoff`.
    pub min_backoff: Duration,
    pub max_backoff: Duration,
    /// Runs in a row that may fail before the peer waits for the next
    /// trigger.
    pub max_attempts: u32,
    /// The least time between runs the peer starts.
    pub min_peer_run_interval: Duration,
    /// PSI sessions running at once, across peers.
    pub max_sessions: usize,
    /// The most network keys a peer may announce.
    pub max_peer_keys: usize,
    /// Broker bytes held for a run before its session starts.
    pub max_broker_buffer: usize,
    /// The longest certificate chain accepted from a peer.
    pub max_chain_depth: usize,
    /// Retries for calls to peers.
    pub call_policy: RetryPolicy,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            debounce: Duration::from_millis(200),
            negotiate_timeout: Duration::from_secs(10),
            stall_timeout: Duration::from_secs(15),
            run_timeout: Duration::from_secs(120),
            exchange_timeout: Duration::from_secs(15),
            retry_delay: Duration::from_millis(500),
            min_backoff: Duration::SECOND,
            max_backoff: Duration::from_secs(30),
            max_attempts: 6,
            min_peer_run_interval: Duration::SECOND,
            max_sessions: 4,
            max_peer_keys: 256,
            max_broker_buffer: 64 * 1024,
            max_chain_depth: 8,
            call_policy: RetryPolicy::peer(),
        }
    }
}

/// What network membership has done.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Runs started, by either side, and their outcomes.
    pub runs: u64,
    pub runs_completed: u64,
    pub runs_failed: u64,
    /// Runs started again after the receiver's cuckoo table failed.
    pub cuckoo_retries: u64,
    /// Runs the peer started that waited out the pacing interval.
    pub runs_paced: u64,
    /// Links paired with peers, and those opened.
    pub links_paired: u64,
    pub links_opened: u64,
    /// Peer bindings and certificates refused.
    pub bindings_refused: u64,
    /// Broker bytes in and out.
    pub broker_bytes_read: u64,
    pub broker_bytes_written: u64,
    /// Calls to peers that failed.
    pub call_errors: u64,
    /// Node operations that failed.
    pub host_errors: u64,
}
