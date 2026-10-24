// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A peer's write queues, wake state and transfer metrics.
//!
//! The driver pulls frames with
//! [`Runner::poll_transmit`](crate::Runner::poll_transmit) while
//! [`Runner::wants_transmit`](crate::Runner::wants_transmit) is true: a
//! write cycle. [`Wake`] says when the next cycle starts. `enqueue` defers
//! it to the next tick of the runner's clock, `enqueue_now` starts it at
//! once (and queues [`Event::Transmit`](crate::Event::Transmit)), and
//! `push_data` doesn't start one.
//!
//! Bounds: the task queue holds each of the peer's channels at most once,
//! and each data queue at most
//! [`DATA_QUEUE_CAP`](crate::DATA_QUEUE_CAP) live entries; refused entries
//! are counted in [`PeerStats::dropped`].

use std::collections::{BTreeMap, BTreeSet};

use strims_base::stats::Sma;
use strims_base::time::{Duration, Time};
use strims_binmap::Bin;

use crate::peer_data_queue::PeerDataQueue;
use crate::peer_writer_queue::PeerTaskRunnerQueue;
use crate::queue_delay::PathDelay;
use crate::runner::{PeerKey, SwarmKey};
use crate::scheduler::{PeerPriority, PeerTaskQueue};

/// When a peer writes next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wake {
    /// No write cycle pending.
    Idle,
    /// Start a write cycle at the next tick.
    AtTick,
    /// In a write cycle: the driver polls frames.
    Now,
}

/// Bytes and their rate over a minute.
#[derive(Clone, Debug)]
pub(crate) struct TransferMetrics {
    count: u64,
    rate: Sma,
}

impl TransferMetrics {
    pub(crate) fn new(now: Time) -> TransferMetrics {
        TransferMetrics {
            count: 0,
            rate: Sma::new(60, Duration::SECOND, now),
        }
    }

    /// Counts `n` bytes.
    pub(crate) fn add(&mut self, n: u64, now: Time) {
        if n == 0 {
            return;
        }
        self.count = self.count.saturating_add(n);
        self.rate.add(n, now);
    }

    /// Bytes per second over the window.
    pub(crate) fn rate(&mut self, now: Time) -> u64 {
        self.rate.rate(Duration::SECOND, now)
    }

    pub(crate) fn snapshot(&mut self, now: Time) -> TransferMetricsSnapshot {
        TransferMetricsSnapshot {
            count: self.count,
            rate: self.rate(now),
        }
    }
}

/// Data bytes in total, and bytes per second over the last minute.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransferMetricsSnapshot {
    pub count: u64,
    pub rate: u64,
}

/// Data received from and sent to a peer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MetricsSnapshot {
    pub read: TransferMetricsSnapshot,
    pub write: TransferMetricsSnapshot,
}

/// A peer's transfer totals, and per swarm.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PeerMetricsSnapshot {
    pub metrics: MetricsSnapshot,
    pub swarms: Vec<(SwarmKey, MetricsSnapshot)>,
}

/// A peer's queue state, for diagnostics and tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PeerStats {
    pub wants_transmit: bool,
    /// Channels queued to write.
    pub queued_channels: bool,
    /// Live entries per priority (pushes, then requests).
    pub queued_data: [usize; 2],
    /// Ring entries per priority, live or dead.
    pub queue_ring: [usize; 2],
    /// Requests and pushes refused because a queue was full.
    pub dropped: u64,
}

/// A peer's runner state.
#[derive(Debug)]
pub(crate) struct RunnerPeer {
    pub(crate) id: Vec<u8>,
    /// Our channel ids on this peer's frames. Bound: one per swarm.
    pub(crate) channels: BTreeMap<u64, SwarmKey>,
    /// Channels with messages to write. Bound: one entry per channel.
    pub(crate) rq: PeerTaskRunnerQueue<SwarmKey>,
    /// Data to send, by [`PeerPriority`].
    pub(crate) dq: [PeerDataQueue<SwarmKey>; 2],
    pub(crate) wake: Wake,
    pub(crate) read: TransferMetrics,
    pub(crate) write: TransferMetrics,
    /// Data bytes that verified and were new to us: what the upload
    /// weights reward, so a peer can't earn upload with junk or
    /// duplicates. `read` counts every data byte.
    pub(crate) useful: TransferMetrics,
    pub(crate) qos_weight: Option<u64>,
    pub(crate) dropped: u64,
    /// Ping round trips, for the upload meter's queueing delay.
    pub(crate) path: PathDelay,
}

impl RunnerPeer {
    pub(crate) fn new(id: &[u8], now: Time) -> RunnerPeer {
        RunnerPeer {
            id: id.to_vec(),
            channels: BTreeMap::new(),
            rq: PeerTaskRunnerQueue::default(),
            dq: [PeerDataQueue::default(), PeerDataQueue::default()],
            wake: Wake::Idle,
            read: TransferMetrics::new(now),
            write: TransferMetrics::new(now),
            useful: TransferMetrics::new(now),
            qos_weight: None,
            dropped: 0,
            path: PathDelay::default(),
        }
    }

    /// True if nothing is queued.
    pub(crate) fn is_idle(&self) -> bool {
        self.rq.is_empty() && self.dq.iter().all(|q| q.is_empty())
    }

    pub(crate) fn stats(&self) -> PeerStats {
        PeerStats {
            wants_transmit: self.wake == Wake::Now,
            queued_channels: !self.rq.is_empty(),
            queued_data: [self.dq[0].len(), self.dq[1].len()],
            queue_ring: [self.dq[0].ring_len(), self.dq[1].ring_len()],
            dropped: self.dropped,
        }
    }
}

/// The peers as the scheduler sees them. `transmit` collects peers that
/// started a write cycle, for [`Event::Transmit`](crate::Event::Transmit).
pub(crate) struct Queues<'a> {
    pub(crate) peers: &'a mut slotmap::SlotMap<PeerKey, RunnerPeer>,
    pub(crate) transmit: &'a mut BTreeSet<PeerKey>,
}

impl PeerTaskQueue for Queues<'_> {
    fn enqueue(&mut self, peer: PeerKey, swarm: SwarmKey) {
        if let Some(p) = self.peers.get_mut(peer)
            && p.rq.push(swarm)
            && p.wake == Wake::Idle
        {
            p.wake = Wake::AtTick;
        }
    }

    fn enqueue_now(&mut self, peer: PeerKey, swarm: SwarmKey) {
        if let Some(p) = self.peers.get_mut(peer) {
            p.rq.push(swarm);
            if p.wake != Wake::Now {
                p.wake = Wake::Now;
                self.transmit.insert(peer);
            }
        }
    }

    fn push_data(
        &mut self,
        peer: PeerKey,
        swarm: SwarmKey,
        b: Bin,
        t: Time,
        pri: PeerPriority,
    ) -> bool {
        if let Some(p) = self.peers.get_mut(peer)
            && let Some(q) = p.dq.get_mut(pri.index())
            && !q.push(swarm, b, t)
        {
            p.dropped = p.dropped.saturating_add(1);
            return false;
        }
        true
    }

    fn push_front_data(
        &mut self,
        peer: PeerKey,
        swarm: SwarmKey,
        b: Bin,
        t: Time,
        pri: PeerPriority,
    ) {
        if let Some(q) = self
            .peers
            .get_mut(peer)
            .and_then(|p| p.dq.get_mut(pri.index()))
        {
            q.push_front(swarm, b, t);
        }
    }

    fn remove_data(&mut self, peer: PeerKey, swarm: SwarmKey, b: Bin, pri: PeerPriority) {
        if let Some(q) = self
            .peers
            .get_mut(peer)
            .and_then(|p| p.dq.get_mut(pri.index()))
        {
            q.remove(swarm, b);
        }
    }

    fn remove_runner(&mut self, peer: PeerKey, swarm: SwarmKey) {
        if let Some(p) = self.peers.get_mut(peer) {
            p.rq.remove(swarm);
            for q in &mut p.dq {
                q.remove_all(swarm);
            }
        }
    }
}
