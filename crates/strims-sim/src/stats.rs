// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The runner, link and network parts of `NodeMetrics`, folded over a run.
//!
//! The controller samples every node every `Scenario::stats_every`.
//!
//! - **Gauges** (data queue lengths, request deadlines, LEDBAT's window,
//!   flight size and CTO, the bytes vnic holds for a link, vpn's queued
//!   copies) keep their extremes over the samples ([`Peaks`]).
//! - **Counters** (data bytes and rejected input per channel, queue
//!   refusals per peer, push/pull counters and duplicates per swarm) are
//!   cumulative per channel, peer or swarm scheduler, and restart at zero
//!   when the runner recreates one (a peer relinks, a swarm loses its last
//!   channel, the host restarts). Each source is tracked on its own and
//!   folded into the host's totals when any of its counters goes down or
//!   the host crashes. What a source counted after its last sample is
//!   lost when it ends, so totals are lower bounds, short by at most one
//!   sample interval per source.
//!
//! Nothing here reads the wall clock or iterates a hash map: everything
//! is a function of the samples, which are a function of the seed.
//!
//! The measures derived from these (seed data share, duplicates, push hit
//! rate) are described in [`crate::report`].

use std::collections::BTreeMap;

use strims_node::{HostId, NodeMetrics, SwarmKey};

/// Extremes of the gauges over every sample of a host (all incarnations).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Peaks {
    /// The longest data queue toward one peer, per priority: pushes, then
    /// requests (ppspp `PeerStats::queued_data`).
    pub queued_data: [usize; 2],
    /// The most request deadlines one channel tracked
    /// (`ChannelSchedulerStats::pending_requests`: requests sent within
    /// the last CTO, answered or not).
    pub pending_requests: usize,
    /// LEDBAT's smallest and largest window on any channel, in bytes.
    pub cwnd_min: Option<i64>,
    pub cwnd_max: i64,
    /// The largest flight size on any channel, in bytes.
    pub flight_max: i64,
    /// The longest congestion timeout (the request timeout) and mean RTT
    /// on any channel, in milliseconds.
    pub cto_max_ms: i64,
    pub rtt_max_ms: i64,
    /// The most bytes vnic held for one link's transport
    /// (`LinkInfo::transmit_pending`).
    pub link_pending: usize,
    /// Link samples whose transport took no more bytes
    /// (`LinkInfo::write_blocked`).
    pub write_blocked: u64,
    /// The most message copies queued on one network's links
    /// (`NetworkInfo::queued`).
    pub vpn_queued: usize,
}

impl Peaks {
    /// Keeps the extremes of both (sums the blocked samples).
    pub fn merge(&mut self, o: &Peaks) {
        for (a, b) in self.queued_data.iter_mut().zip(o.queued_data) {
            *a = (*a).max(b);
        }
        self.pending_requests = self.pending_requests.max(o.pending_requests);
        self.cwnd_min = match (self.cwnd_min, o.cwnd_min) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        self.cwnd_max = self.cwnd_max.max(o.cwnd_max);
        self.flight_max = self.flight_max.max(o.flight_max);
        self.cto_max_ms = self.cto_max_ms.max(o.cto_max_ms);
        self.rtt_max_ms = self.rtt_max_ms.max(o.rtt_max_ms);
        self.link_pending = self.link_pending.max(o.link_pending);
        self.write_blocked += o.write_blocked;
        self.vpn_queued = self.vpn_queued.max(o.vpn_queued);
    }
}

/// One host's counters for one swarm (the scenario's swarm index).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwarmCounters {
    /// Data bytes sent to and received from peers on the swarm's channels
    /// (`ChannelInfo::metrics`).
    pub data_sent: u64,
    pub data_received: u64,
    /// Inbound data or chunks that failed verification
    /// (`ChannelMetrics::invalid_data` + `invalid_chunk`).
    pub invalid: u64,
    /// Inbound messages ppspp ignored as invalid
    /// (`ChannelMetrics::rejected`), which honest peers never send.
    pub rejected: u64,
    /// REQUEST and HAVE messages sent.
    pub requests: u64,
    pub haves: u64,
    /// Chunks received that the node already had
    /// (`SwarmSchedulerStats::wasted_chunks`).
    pub wasted_chunks: u64,
    /// `PushStats` counters: chunks expected from a parent, and pushes
    /// that arrived in time (`hits`), didn't (`misses`, released to pull),
    /// arrived after their release (`late`), or came from a peer that
    /// isn't the parent (`unsolicited`).
    pub expected: u64,
    pub hits: u64,
    pub misses: u64,
    pub late: u64,
    pub unsolicited: u64,
    /// Parents replaced (`switched`), cancelled for misses (`pruned`) or
    /// lost because the parent closed the subscription (`shed`).
    pub switched: u64,
    pub pruned: u64,
    pub shed: u64,
    /// Subscriptions this node accepted as a parent, and closed because
    /// its uplink saturated.
    pub accepted: u64,
    pub sheds_sent: u64,
    /// Pushes dropped as stale, and not sent because the child had the
    /// chunk.
    pub stale_drops: u64,
    pub suppressed: u64,
}

impl SwarmCounters {
    /// Adds `o` to `self`.
    pub fn add(&mut self, o: &SwarmCounters) {
        let mut a = self.to_array();
        for (x, y) in a.iter_mut().zip(o.to_array()) {
            *x += y;
        }
        *self = SwarmCounters::from_array(a);
    }

    /// Pushes that arrived in time over pushes expected and resolved.
    pub fn hit_rate(&self) -> Option<f64> {
        let n = self.hits + self.misses;
        (n > 0).then(|| self.hits as f64 / n as f64)
    }

    fn to_array(self) -> [u64; SWARM_COUNTERS] {
        [
            self.data_sent,
            self.data_received,
            self.invalid,
            self.rejected,
            self.requests,
            self.haves,
            self.wasted_chunks,
            self.expected,
            self.hits,
            self.misses,
            self.late,
            self.unsolicited,
            self.switched,
            self.pruned,
            self.shed,
            self.accepted,
            self.sheds_sent,
            self.stale_drops,
            self.suppressed,
        ]
    }

    fn from_array(a: [u64; SWARM_COUNTERS]) -> SwarmCounters {
        let [
            data_sent,
            data_received,
            invalid,
            rejected,
            requests,
            haves,
            wasted_chunks,
            expected,
            hits,
            misses,
            late,
            unsolicited,
            switched,
            pruned,
            shed,
            accepted,
            sheds_sent,
            stale_drops,
            suppressed,
        ] = a;
        SwarmCounters {
            data_sent,
            data_received,
            invalid,
            rejected,
            requests,
            haves,
            wasted_chunks,
            expected,
            hits,
            misses,
            late,
            unsolicited,
            switched,
            pruned,
            shed,
            accepted,
            sheds_sent,
            stale_drops,
            suppressed,
        }
    }
}

/// What the runner, link and network stats showed for one host over the
/// run (all incarnations).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunnerReport {
    pub peaks: Peaks,
    /// Per scenario swarm.
    pub swarms: Vec<SwarmCounters>,
    /// Requests and pushes refused because a peer's data queue was full
    /// (`PeerStats::dropped`).
    pub queue_drops: u64,
}

impl RunnerReport {
    pub fn swarm(&self, s: usize) -> SwarmCounters {
        self.swarms.get(s).copied().unwrap_or_default()
    }

    fn add(&mut self, o: &RunnerReport) {
        self.peaks.merge(&o.peaks);
        if self.swarms.len() < o.swarms.len() {
            self.swarms.resize(o.swarms.len(), SwarmCounters::default());
        }
        for (a, b) in self.swarms.iter_mut().zip(&o.swarms) {
            a.add(b);
        }
        self.queue_drops += o.queue_drops;
    }
}

/// Cumulative counters of one source that restart at zero when the
/// source is recreated.
#[derive(Clone, Copy, Debug)]
struct Tracked<const N: usize> {
    folded: [u64; N],
    last: [u64; N],
}

impl<const N: usize> Default for Tracked<N> {
    fn default() -> Self {
        Tracked {
            folded: [0; N],
            last: [0; N],
        }
    }
}

impl<const N: usize> Tracked<N> {
    fn observe(&mut self, now: [u64; N]) {
        if now.iter().zip(&self.last).any(|(n, l)| n < l) {
            self.fold();
        }
        self.last = now;
    }

    fn fold(&mut self) {
        for (f, l) in self.folded.iter_mut().zip(&mut self.last) {
            *f += std::mem::take(l);
        }
    }

    fn total(&self) -> [u64; N] {
        let mut t = self.folded;
        for (a, b) in t.iter_mut().zip(self.last) {
            *a += b;
        }
        t
    }
}

/// Swarm counters, in [`SwarmCounters`]' field order.
const SWARM_COUNTERS: usize = 19;

/// Channel counters: data sent, data received, invalid, rejected,
/// requests, haves.
const CHANNEL_COUNTERS: usize = 6;

/// One host's view across samples.
#[derive(Clone, Debug, Default)]
pub(crate) struct HostStats {
    /// Totals of finished incarnations.
    totals: RunnerReport,
    /// The current incarnation: peaks, and its sources.
    peaks: Peaks,
    /// By peer and scenario swarm.
    channels: BTreeMap<(HostId, usize), Tracked<CHANNEL_COUNTERS>>,
    /// Queue refusals by peer.
    peers: BTreeMap<HostId, Tracked<1>>,
    /// Scheduler and push counters by scenario swarm.
    swarms: BTreeMap<usize, Tracked<SWARM_COUNTERS>>,
}

/// What one sample showed that the invariants check.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SampleGauges {
    pub queued_data: usize,
    pub pending_requests: usize,
    pub queue_drops: u64,
    pub invalid: u64,
    pub rejected: u64,
    pub vpn_bad_input: u64,
}

impl HostStats {
    /// Folds one sample of the current incarnation. `swarms` maps the
    /// node's swarm keys to scenario swarm indexes.
    pub fn observe(&mut self, m: &NodeMetrics, swarms: &[Option<SwarmKey>]) -> SampleGauges {
        let index = |k: SwarmKey| swarms.iter().position(|s| *s == Some(k));
        let mut g = SampleGauges::default();
        let p = &mut self.peaks;
        for peer in &m.runner.peers {
            let q = peer.queues;
            for (a, b) in p.queued_data.iter_mut().zip(q.queued_data) {
                *a = (*a).max(b);
            }
            g.queued_data = g
                .queued_data
                .max(q.queued_data.iter().copied().max().unwrap_or(0));
            g.queue_drops += q.dropped;
            self.peers
                .entry(peer.host_id)
                .or_default()
                .observe([q.dropped]);
            for ch in &peer.channels {
                let Some(s) = index(ch.swarm) else {
                    continue;
                };
                let tracked = self.channels.entry((peer.host_id, s)).or_default();
                // Without stats (no scheduler right now), the message
                // counters keep their last values, so the gap doesn't read
                // as a restart.
                let [_, _, invalid, rejected, requests, haves] = tracked.last;
                let mut c = [
                    ch.metrics.write.count,
                    ch.metrics.read.count,
                    invalid,
                    rejected,
                    requests,
                    haves,
                ];
                if let Some(st) = &ch.stats {
                    let sc = &st.scheduler;
                    if sc.handshake_received {
                        p.cwnd_min = Some(p.cwnd_min.map_or(sc.cwnd, |x| x.min(sc.cwnd)));
                    }
                    p.cwnd_max = p.cwnd_max.max(sc.cwnd);
                    p.flight_max = p.flight_max.max(sc.flight_size);
                    p.cto_max_ms = p.cto_max_ms.max(sc.cto.as_millis());
                    p.rtt_max_ms = p.rtt_max_ms.max(sc.rtt_mean.as_millis());
                    p.pending_requests = p.pending_requests.max(sc.pending_requests);
                    g.pending_requests = g.pending_requests.max(sc.pending_requests);
                    let invalid = st.inbound.invalid_data + st.inbound.invalid_chunk;
                    g.invalid += invalid;
                    g.rejected += st.inbound.rejected;
                    c = [
                        ch.metrics.write.count,
                        ch.metrics.read.count,
                        invalid,
                        st.inbound.rejected,
                        st.outbound.request,
                        st.outbound.have,
                    ];
                }
                tracked.observe(c);
            }
        }
        for sw in &m.runner.swarms {
            let (Some(s), Some(sched), Some(push)) = (index(sw.swarm), sw.scheduler, &sw.push)
            else {
                continue;
            };
            let c = SwarmCounters {
                wasted_chunks: sched.wasted_chunks,
                expected: push.expected,
                hits: push.hits,
                misses: push.misses,
                late: push.late,
                unsolicited: push.unsolicited,
                switched: push.switched,
                pruned: push.pruned,
                shed: push.shed,
                accepted: push.accepted,
                sheds_sent: push.sheds_sent,
                stale_drops: push.stale_drops,
                suppressed: push.suppressed,
                ..SwarmCounters::default()
            };
            self.swarms.entry(s).or_default().observe(c.to_array());
        }
        for l in &m.links {
            p.link_pending = p.link_pending.max(l.transmit_pending);
            p.write_blocked += u64::from(l.write_blocked);
        }
        for n in &m.networks {
            p.vpn_queued = p.vpn_queued.max(n.queued);
        }
        g.vpn_bad_input = m.vpn.malformed
            + m.vpn.trailer_mismatch
            + m.vpn.verify_failures
            + m.vpn.decrypt_failures;
        g
    }

    /// The current incarnation ended.
    pub fn retire(&mut self) {
        let r = self.current();
        self.totals.add(&r);
        self.peaks = Peaks::default();
        self.channels.clear();
        self.peers.clear();
        self.swarms.clear();
    }

    fn current(&self) -> RunnerReport {
        let mut r = RunnerReport {
            peaks: self.peaks,
            ..RunnerReport::default()
        };
        let n = self
            .channels
            .keys()
            .map(|&(_, s)| s + 1)
            .chain(self.swarms.keys().map(|&s| s + 1))
            .max()
            .unwrap_or(0);
        r.swarms = vec![SwarmCounters::default(); n];
        for (&(_, s), t) in &self.channels {
            let [sent, received, invalid, rejected, requests, haves] = t.total();
            if let Some(c) = r.swarms.get_mut(s) {
                c.add(&SwarmCounters {
                    data_sent: sent,
                    data_received: received,
                    invalid,
                    rejected,
                    requests,
                    haves,
                    ..SwarmCounters::default()
                });
            }
        }
        for (&s, t) in &self.swarms {
            if let Some(c) = r.swarms.get_mut(s) {
                c.add(&SwarmCounters::from_array(t.total()));
            }
        }
        r.queue_drops = self.peers.values().map(|t| t.total()[0]).sum();
        r
    }

    /// Totals plus the current incarnation.
    pub fn report(&self) -> RunnerReport {
        let mut r = self.totals.clone();
        r.add(&self.current());
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracked_counters_survive_restarts() {
        let mut t = Tracked::<2>::default();
        t.observe([5, 1]);
        t.observe([9, 1]);
        // The source was recreated: one counter went down.
        t.observe([2, 1]);
        assert_eq!(t.total(), [11, 2]);
        t.fold();
        assert_eq!(t.total(), [11, 2]);
        t.observe([1, 0]);
        assert_eq!(t.total(), [12, 2]);
    }

    #[test]
    fn swarm_counters_round_trip() {
        let c = SwarmCounters {
            data_sent: 1,
            hits: 7,
            suppressed: 3,
            ..SwarmCounters::default()
        };
        assert_eq!(SwarmCounters::from_array(c.to_array()), c);
        let mut d = c;
        d.add(&c);
        assert_eq!((d.data_sent, d.hits, d.suppressed), (2, 14, 6));
        assert_eq!(d.hit_rate(), Some(1.0));
    }
}
