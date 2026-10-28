// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The answer gate: a [`PushMode::Hybrid`](crate::PushMode::Hybrid) node
//! answers pull requests only from the uplink its pushes leave.
//!
//! An uplink byte goes first to what only this node sends. Behind a seed
//! with little margin, answers compete with pushes for its uplink, and a
//! lower priority within a peer's frame doesn't stop them: across peers
//! the driver's QoS decides, and below the runner the access link splits
//! the uplink by transport share, so answer bytes already handed down sit
//! ahead of the next push. Viewers whose pushes come late prune their
//! parents and pull more, and the pull load locks in.
//!
//! So a hybrid swarm's answers queue apart, below pushes and below the
//! answers of swarms that don't push, and none is written while a push
//! waits for any peer whose push queue stood empty within
//! [`BLOCKED_SPAN`]. A peer whose pushes have waited longer is blocked on
//! its own link (a slow viewer whose transport doesn't drain), so it holds
//! no answer back. When every peer's pushes back up (an overcommitted
//! uplink), answers go as the driver polls. Peers whose answers the gate
//! held are woken as soon as it opens.
//!
//! The gate reads only the node's own queues: it needs no capacity
//! estimate or rate limit, and trusts no requester. A peer that stops
//! reading holds back no answer for longer than [`BLOCKED_SPAN`].

use slotmap::SlotMap;
use strims_base::time::{Duration, Time};

use crate::peer::RunnerPeer;
use crate::runner::PeerKey;
use crate::scheduler::PeerPriority;

/// How long a peer's oldest queued push may have waited for its pushes to
/// hold answers back.
pub const BLOCKED_SPAN: Duration = Duration::from_millis(500);

/// Whether a hybrid swarm's answer may be written now: no peer's oldest
/// queued push is younger than [`BLOCKED_SPAN`]. Drops the dead entries
/// ahead of each peer's oldest live push.
pub(crate) fn admits(peers: &mut SlotMap<PeerKey, RunnerPeer>, now: Time) -> bool {
    !peers.values_mut().any(|p| {
        p.queue_mut(PeerPriority::High)
            .front_time()
            .is_some_and(|t| now - t < BLOCKED_SPAN)
    })
}
