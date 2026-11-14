// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A hostile node's hook into the ppspp frames it reads and sends, for
//! adversarial simulations (strims-sim's hostile hosts).
//!
//! A node built with [`NodeConfig::adversary`](crate::NodeConfig::adversary)
//! runs an honest runner and hands every ppspp frame through its
//! [`Adversary`]: each frame a peer sent before the runner reads it
//! ([`Adversary::ingress`]), and each frame the runner wrote before vnic
//! sends it ([`Adversary::egress`]). The adversary can also send on its
//! own: a peer it [wants to transmit to](Adversary::wants_transmit) is
//! offered frames while the runner has nothing for it, and its
//! [deadline](Adversary::poll_timeout) is one of the node's.
//!
//! The runner believes it read and sent what it saw and wrote, so a
//! hostile node's own counters of what it served are fictional; only its
//! links' bytes are real.
//!
//! Its rng forks from the node's after every other state machine's
//! (`State::new`), so a node without one draws as it would.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use strims_base::rng::DetRng;
use strims_base::time::Time;
use strims_kademlia::ID;
use strims_ppspp::SwarmKey;

use crate::state::ChannelIds;

/// What a hostile node does to the ppspp frames between its runner and
/// its peers. Every method is called on the node task, between the state
/// machines' calls; none may block.
pub trait Adversary: Send {
    /// Rewrites `frame[..len]`, a frame `peer` sent, in place before the
    /// runner reads it, and returns the length the runner reads (at most
    /// `len`).
    fn ingress(
        &mut self,
        peer: AdversaryPeer<'_>,
        now: Time,
        frame: &mut [u8],
        len: usize,
    ) -> usize;

    /// Rewrites `frame[..len]`, the frame the runner wrote to `peer` (none
    /// with `len` 0), in place before vnic sends it, and returns the length
    /// sent: up to `frame.len()`, the vnic frame's room, which can exceed
    /// what the runner is given. 0 sends nothing.
    fn egress(&mut self, peer: AdversaryPeer<'_>, now: Time, frame: &mut [u8], len: usize)
    -> usize;

    /// Whether the adversary has something of its own for `peer` at
    /// `now`: the node then offers the peer frames ([`Adversary::egress`])
    /// while the runner has nothing for it.
    fn wants_transmit(&self, _peer: &ID, _now: Time) -> bool {
        false
    }

    /// When to call [`Adversary::handle_timeout`]; one of the node's
    /// deadlines.
    fn poll_timeout(&self) -> Option<Time> {
        None
    }

    /// Runs the adversary's deadline. The node then asks
    /// [`Adversary::wants_transmit`] of every peer.
    fn handle_timeout(&mut self, _now: Time) {}

    /// `peer` is gone: forget what is kept of it.
    fn forget_peer(&mut self, _peer: &ID) {}
}

/// Builds a node's [`Adversary`] from the rng the node forks for it and
/// the node's host id ([`NodeConfig::adversary`](crate::NodeConfig::adversary)):
/// a restart rebuilds the node, and its adversary, from the config.
#[derive(Clone)]
pub struct AdversaryFactory(pub Arc<dyn Fn(DetRng, ID) -> Box<dyn Adversary> + Send + Sync>);

impl fmt::Debug for AdversaryFactory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AdversaryFactory")
    }
}

/// A peer as an [`Adversary`] sees it: its host id, which the runner names
/// it by (its bytes are its ppspp peer id), and the channels of the swarms
/// run with it. A frame of a link the node knows no peer of (before the
/// peer's init, or as the link goes down) still goes through the
/// adversary, its peer named by the zero id with no swarms.
#[derive(Clone, Copy, Debug)]
pub struct AdversaryPeer<'a> {
    pub host_id: &'a ID,
    pub(crate) swarms: &'a BTreeMap<SwarmKey, ChannelIds>,
}

impl AdversaryPeer<'_> {
    /// The swarm whose frames from the peer carry `channel`.
    pub fn swarm_of(&self, channel: u64) -> Option<SwarmKey> {
        self.swarms
            .iter()
            .find(|(_, c)| c.channel == channel)
            .map(|(&k, _)| k)
    }

    /// The swarm whose frames to the peer carry `channel`.
    pub fn swarm_to(&self, channel: u64) -> Option<SwarmKey> {
        self.swarms
            .iter()
            .find(|(_, c)| c.peer_channel == channel)
            .map(|(&k, _)| k)
    }

    /// The channel id frames to the peer carry for `swarm`.
    pub fn peer_channel(&self, swarm: SwarmKey) -> Option<u64> {
        self.swarms.get(&swarm).map(|c| c.peer_channel)
    }

    /// The swarms run with the peer.
    pub fn swarms(&self) -> impl Iterator<Item = SwarmKey> + '_ {
        self.swarms.keys().copied()
    }
}

/// A node's adversary, kept by its state.
pub(crate) struct Kept(pub(crate) Box<dyn Adversary>);

impl fmt::Debug for Kept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Adversary")
    }
}
