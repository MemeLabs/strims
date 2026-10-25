// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Who serves whom in one swarm: a node's transfers at one sample.

use strims_node_core::{HostId, NodeMetrics, SwarmKey};

/// A node's transfers in one swarm at one sample, as its runner counts
/// them: cumulative, so they restart at zero with the swarm's scheduler or
/// a peer's channel. The testbed's nodes log one each second, and
/// strims-sim's `calibration_star` the same, so who serves whom in a real
/// swarm compares line for line with the simulated one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TransferSample {
    /// Chunks written to peers, and of them pushes to subscribers.
    pub chunks_sent: u64,
    pub pushed_chunks: u64,
    /// Subscriptions served, and the streams with an open parent.
    pub children: usize,
    pub parents: usize,
    /// As a child: pushes before their deadline, deadlines released to
    /// pull, and pushes after their release.
    pub hits: u64,
    pub misses: u64,
    pub late: u64,
    /// As a child: subscriptions opened, declined, unanswered, closed by
    /// their parent, cancelled for misses, and replaced by a faster peer.
    pub opens: u64,
    pub declined: u64,
    pub open_timeouts: u64,
    pub shed: u64,
    pub pruned: u64,
    pub switched: u64,
    /// As a parent: subscriptions accepted and declined, subscribers
    /// closed for a saturated uplink, and pushes dropped as stale.
    pub accepted: u64,
    pub rejected: u64,
    pub sheds_sent: u64,
    pub stale_drops: u64,
    /// Per stream with an open parent, the parent's mean push delay and
    /// the deadline it gets, in milliseconds.
    pub parent_lag: Vec<Option<(u64, u64)>>,
    /// Each peer running the swarm, in link order.
    pub peers: Vec<PeerTransfer>,
}

/// One peer's channel in a [`TransferSample`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PeerTransfer {
    /// The peer's index, as the caller maps host ids (`None`: unknown).
    pub peer: Option<usize>,
    /// Data bytes written to it and read from it.
    pub sent: u64,
    pub received: u64,
    /// REQUEST messages it sent us, and we sent it.
    pub requests_in: u64,
    pub requests_out: u64,
    /// Its data queues (all swarms): pushes, then request answers.
    pub queued: [usize; 2],
    /// Our request window toward it: the cap and the bytes requested and
    /// not yet received, the round trip the window reads and the request
    /// timeout (ms), and the request deadlines outstanding.
    pub cwnd: i64,
    pub flight: i64,
    pub rtt_ms: u64,
    pub cto_ms: u64,
    pub pending: usize,
}

impl TransferSample {
    /// `swarm`'s transfers in `m`; `None` while it has no scheduler.
    pub fn of(
        m: &NodeMetrics,
        swarm: SwarmKey,
        index_of: impl Fn(&HostId) -> Option<usize>,
    ) -> Option<TransferSample> {
        let sw = m.runner.swarms.iter().find(|s| s.swarm == swarm)?;
        let (sched, push) = (sw.scheduler?, sw.push.as_ref()?);
        let peers = m
            .runner
            .peers
            .iter()
            .filter_map(|p| {
                let ch = p.channels.iter().find(|c| c.swarm == swarm)?;
                let (requests_in, requests_out) = ch
                    .stats
                    .map_or((0, 0), |s| (s.inbound.request, s.outbound.request));
                let w = ch.stats.map(|s| s.scheduler);
                Some(PeerTransfer {
                    peer: index_of(&p.host_id),
                    sent: ch.metrics.write.count,
                    received: ch.metrics.read.count,
                    requests_in,
                    requests_out,
                    queued: p.queues.queued_data,
                    cwnd: w.map_or(0, |w| w.cwnd),
                    flight: w.map_or(0, |w| w.flight_size),
                    rtt_ms: w.map_or(0, |w| w.rtt_mean.as_millis() as u64),
                    cto_ms: w.map_or(0, |w| w.cto.as_millis() as u64),
                    pending: w.map_or(0, |w| w.pending_requests),
                })
            })
            .collect();
        Some(TransferSample {
            chunks_sent: sched.chunks_sent,
            pushed_chunks: sched.pushed_chunks,
            children: push.children,
            parents: push.parents.iter().filter(|p| p.is_some()).count(),
            hits: push.hits,
            misses: push.misses,
            late: push.late,
            opens: push.opens,
            declined: push.declined,
            open_timeouts: push.open_timeouts,
            shed: push.shed,
            pruned: push.pruned,
            switched: push.switched,
            accepted: push.accepted,
            rejected: push.rejected,
            sheds_sent: push.sheds_sent,
            stale_drops: push.stale_drops,
            parent_lag: push
                .parent_lag
                .iter()
                .map(|l| l.map(|(d, dl)| (d.as_millis() as u64, dl.as_millis() as u64)))
                .collect(),
            peers,
        })
    }

    /// One JSON object, fields named as here.
    pub fn json(&self) -> String {
        let peers: Vec<String> = self
            .peers
            .iter()
            .map(|p| {
                format!(
                    "{{\"peer\":{},\"sent\":{},\"received\":{},\"requests_in\":{},\"requests_out\":{},\"queued\":[{},{}],\"cwnd\":{},\"flight\":{},\"rtt_ms\":{},\"cto_ms\":{},\"pending\":{}}}",
                    p.peer.map_or("null".to_owned(), |i| i.to_string()),
                    p.sent,
                    p.received,
                    p.requests_in,
                    p.requests_out,
                    p.queued[0],
                    p.queued[1],
                    p.cwnd,
                    p.flight,
                    p.rtt_ms,
                    p.cto_ms,
                    p.pending,
                )
            })
            .collect();
        let lag: Vec<String> = self
            .parent_lag
            .iter()
            .map(|l| l.map_or("null".to_owned(), |(d, dl)| format!("[{d},{dl}]")))
            .collect();
        format!(
            "{{\"chunks_sent\":{},\"pushed_chunks\":{},\"children\":{},\"parents\":{},\"hits\":{},\"misses\":{},\"late\":{},\"opens\":{},\"declined\":{},\"open_timeouts\":{},\"shed\":{},\"pruned\":{},\"switched\":{},\"accepted\":{},\"rejected\":{},\"sheds_sent\":{},\"stale_drops\":{},\"parent_lag\":[{}],\"peers\":[{}]}}",
            self.chunks_sent,
            self.pushed_chunks,
            self.children,
            self.parents,
            self.hits,
            self.misses,
            self.late,
            self.opens,
            self.declined,
            self.open_timeouts,
            self.shed,
            self.pruned,
            self.switched,
            self.accepted,
            self.rejected,
            self.sheds_sent,
            self.stale_drops,
            lag.join(","),
            peers.join(","),
        )
    }
}
