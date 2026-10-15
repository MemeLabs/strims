// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The interface between the [peer scheduler](crate::scheduler_peer) and
//! the peers' write queues.
//!
//! The scheduler sees a peer through [`PeerTaskQueue`], a trait so the
//! scheduler can be tested against a mock. A channel is named by its peer
//! and its swarm: a peer runs at most one channel per swarm.

use strims_base::time::Time;
use strims_binmap::Bin;

use crate::codec::Stream;
use crate::runner::{PeerKey, SwarmKey};

/// The data queue an entry goes to. A peer's write cycle drains `High`
/// before `Low`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PeerPriority {
    /// Pushes to stream subscribers.
    High = 0,
    /// Answers to requests.
    Low = 1,
}

impl PeerPriority {
    pub(crate) const ALL: [PeerPriority; 2] = [PeerPriority::High, PeerPriority::Low];

    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

/// What a channel scheduler asks of its peer; the channel is
/// `(peer, swarm)`.
pub(crate) trait PeerTaskQueue {
    /// Queues the channel's task runner (`Write`) for the next write cycle,
    /// which starts at the next tick if the peer is idle.
    fn enqueue(&mut self, peer: PeerKey, swarm: SwarmKey);
    /// Queues the channel's task runner and starts a write cycle now.
    fn enqueue_now(&mut self, peer: PeerKey, swarm: SwarmKey);
    /// Queues a bin to send. Doesn't start a write cycle: the caller
    /// enqueues the channel when the bin should go out. A full queue
    /// refuses the bin.
    fn push_data(&mut self, peer: PeerKey, swarm: SwarmKey, b: Bin, t: Time, pri: PeerPriority);
    /// Puts a bin back at the front.
    fn push_front_data(
        &mut self,
        peer: PeerKey,
        swarm: SwarmKey,
        b: Bin,
        t: Time,
        pri: PeerPriority,
    );
    /// Drops queued bins inside `b`.
    fn remove_data(&mut self, peer: PeerKey, swarm: SwarmKey, b: Bin, pri: PeerPriority);
    /// Drops everything the channel queued.
    fn remove_runner(&mut self, peer: PeerKey, swarm: SwarmKey);
}

/// The first bin of stream `s`: chunk `s`.
pub(crate) fn stream_bin_offset(s: Stream) -> Bin {
    Bin(u64::from(s) * 2)
}
