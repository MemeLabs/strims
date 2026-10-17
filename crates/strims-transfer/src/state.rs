// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The transfer protocol's state: the transfers, the networks they are
//! published on, the peers on those networks, and each peer transfer's
//! channels. Sans-IO: the control feeds it the driver's calls, peers'
//! requests and answers, and the time, and carries out its actions.
//!
//! A peer transfer exists only once this side has announced the transfer
//! to the peer, so a peer's requests never grow the state: it is bounded
//! by the local transfers on each linked peer.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use strims_base::time::{Duration, Time};
use strims_kademlia::ID;
use strims_ppspp::SwarmId;

use crate::{Config, TransferId};

/// A node operation's place in the order the node applies them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OpId(u64);

/// A node operation. The node must apply them one at a time, in `OpId`
/// order: the state assumes each one has taken effect before the next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Op<N, S> {
    /// Runs the swarm with the peer. A failure is reported with
    /// [`State::run_failed`].
    Run {
        host: ID,
        link: u64,
        id: TransferId,
        swarm: S,
        channel: u64,
        peer_channel: u64,
        /// The peer runs its side already, so a failure must close it.
        notify: bool,
    },
    Stop {
        host: ID,
        link: u64,
        swarm: S,
    },
    IndexPublish {
        id: TransferId,
        net: N,
        key: Vec<u8>,
        salt: Vec<u8>,
    },
    IndexUnpublish {
        id: TransferId,
        net: N,
    },
    Connect {
        net: N,
        host: ID,
    },
}

/// What the control must do.
///
/// Announces and closes of one (peer, transfer) must reach the peer in
/// order, each once the one before is answered: a close carries no
/// channel, so the peer can't tell which announce it undoes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Action<N, S> {
    Op(OpId, Op<N, S>),
    /// Call the peer's `Announce` on `link`, and report its answer with
    /// [`State::handle_announce_answer`].
    Announce {
        host: ID,
        link: u64,
        id: TransferId,
        channel: u64,
    },
    /// Call the peer's `Close` on `link`.
    Close {
        host: ID,
        link: u64,
        id: TransferId,
    },
    /// Search the peer index: report what it finds with
    /// [`State::search_found`], and its end with [`State::search_done`].
    Search {
        id: TransferId,
        net: N,
        key: Vec<u8>,
        salt: Vec<u8>,
    },
}

/// How to answer a peer's announce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    /// Without a channel: this side hasn't announced the transfer to the
    /// peer, or can't run it.
    Refuse,
    /// With this channel.
    Channel(u64),
    /// With this channel once the op has run; refuse if it fails.
    After(u64, OpId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Channel {
    Idle,
    Running { peer_channel: u64 },
}

#[derive(Debug)]
struct PeerTransfer {
    /// Ours: the channel the peer's frames carry.
    channel: u64,
    state: Channel,
}

#[derive(Debug)]
struct Peer {
    link: u64,
    /// The peer broke the ppspp protocol: nothing runs with it until it
    /// links again.
    failed: bool,
    /// Bound: one per local transfer.
    transfers: BTreeMap<TransferId, PeerTransfer>,
}

#[derive(Debug)]
struct Transfer<N, S> {
    swarm: S,
    swarm_id: SwarmId,
    salt: Vec<u8>,
    /// Bound: the networks the driver reports.
    networks: BTreeSet<N>,
}

#[derive(Debug, Default)]
struct Network {
    /// Bound: the node's peers.
    peers: BTreeSet<ID>,
    /// Bound: the local transfers.
    transfers: BTreeSet<TransferId>,
}

/// See the [module docs](self).
#[derive(Debug)]
pub(crate) struct State<N, S> {
    host_id: ID,
    discovery: bool,
    search_interval: Duration,
    connect_backoff: Duration,
    max_contacted: usize,
    /// Bound: the driver's calls.
    transfers: BTreeMap<TransferId, Transfer<N, S>>,
    /// Bound: the networks the driver reports.
    networks: BTreeMap<N, Network>,
    /// Linked peers. Bound: the node's peers.
    peers: BTreeMap<ID, Peer>,
    next_link: u64,
    /// Channels are never reused, so an answer for an earlier peer
    /// transfer can't match a later one.
    next_channel: u64,
    next_op: u64,
    /// When each published (transfer, network) is next searched. Bound:
    /// transfers × networks.
    searches: BTreeSet<(Time, TransferId, N)>,
    /// Searches in flight, at most one per published pair.
    searching: BTreeSet<(TransferId, N)>,
    /// Hosts asked to link, and until when they aren't asked again.
    /// Bound: `max_contacted`.
    contacted: BTreeMap<ID, Time>,
    /// Drained by the control after every call.
    actions: VecDeque<Action<N, S>>,
}

impl<N: Copy + Ord, S: Copy + Eq> State<N, S> {
    pub(crate) fn new(host_id: ID, config: &Config) -> Self {
        let Config {
            discovery,
            search_interval,
            connect_backoff,
            max_contacted,
            ..
        } = *config;
        State {
            host_id,
            discovery,
            search_interval,
            connect_backoff,
            max_contacted,
            transfers: BTreeMap::new(),
            networks: BTreeMap::new(),
            peers: BTreeMap::new(),
            next_link: 0,
            next_channel: 0,
            next_op: 0,
            searches: BTreeSet::new(),
            searching: BTreeSet::new(),
            contacted: BTreeMap::new(),
            actions: VecDeque::new(),
        }
    }

    pub(crate) fn poll_action(&mut self) -> Option<Action<N, S>> {
        self.actions.pop_front()
    }

    fn push_op(&mut self, op: Op<N, S>) -> OpId {
        self.next_op += 1;
        let id = OpId(self.next_op);
        self.actions.push_back(Action::Op(id, op));
        id
    }

    // --- transfers ----------------------------------------------------------

    /// Adds a transfer of `swarm`. An existing transfer with the same id
    /// is kept as it is.
    pub(crate) fn add(&mut self, swarm: S, swarm_id: &SwarmId, salt: Vec<u8>) -> TransferId {
        let id = TransferId::new(swarm_id, &salt);
        self.transfers.entry(id).or_insert_with(|| Transfer {
            swarm,
            swarm_id: swarm_id.clone(),
            salt,
            networks: BTreeSet::new(),
        });
        id
    }

    pub(crate) fn find(&self, swarm_id: &SwarmId, salt: &[u8]) -> Option<(TransferId, S)> {
        let id = TransferId::new(swarm_id, salt);
        self.transfers.get(&id).map(|t| (id, t.swarm))
    }

    pub(crate) fn list(&self) -> Vec<TransferId> {
        self.transfers.keys().copied().collect()
    }

    pub(crate) fn is_published(&self, id: TransferId, net: N) -> bool {
        self.transfers
            .get(&id)
            .is_some_and(|t| t.networks.contains(&net))
    }

    /// Removes a transfer: stops and closes it with every peer and
    /// withdraws it from the peer index. Returns whether it existed.
    pub(crate) fn remove(&mut self, id: TransferId) -> bool {
        let Some(t) = self.transfers.remove(&id) else {
            return false;
        };
        for &net in &t.networks {
            if let Some(n) = self.networks.get_mut(&net) {
                n.transfers.remove(&id);
            }
            self.withdraw(id, net);
        }
        let hosts: Vec<ID> = self.peers.keys().copied().collect();
        for host in hosts {
            self.drop_peer_transfer(host, id, t.swarm);
        }
        true
    }

    /// Publishes a transfer on `net`: announces it to the network's
    /// peers and, with discovery, advertises it in the network's peer
    /// index and searches there now and every `search_interval`. Returns
    /// whether the transfer exists.
    pub(crate) fn publish(&mut self, id: TransferId, net: N, now: Time) -> bool {
        let Some(t) = self.transfers.get_mut(&id) else {
            return false;
        };
        if !t.networks.insert(net) {
            return true;
        }
        let (key, salt) = (t.swarm_id.as_bytes().to_vec(), t.salt.clone());
        let n = self.networks.entry(net).or_default();
        n.transfers.insert(id);
        let hosts: Vec<ID> = n.peers.iter().copied().collect();
        for host in hosts {
            self.announce(host, id);
        }
        if self.discovery {
            self.push_op(Op::IndexPublish { id, net, key, salt });
            self.searches.insert((now, id, net));
        }
        true
    }

    /// Stops searching for and advertising the transfer on `net`.
    fn withdraw(&mut self, id: TransferId, net: N) {
        self.searches.retain(|&(_, i, n)| (i, n) != (id, net));
        self.searching.remove(&(id, net));
        if self.discovery {
            self.push_op(Op::IndexUnpublish { id, net });
        }
    }

    // --- peers and networks -------------------------------------------------

    /// Takes a newly linked peer, replacing an earlier link to it, and
    /// announces the transfers it shares a network with. Returns the link
    /// id that its requests and our calls on it carry.
    pub(crate) fn add_peer(&mut self, host: ID) -> u64 {
        self.next_link += 1;
        let link = self.next_link;
        self.peers.insert(
            host,
            Peer {
                link,
                failed: false,
                transfers: BTreeMap::new(),
            },
        );
        let shared: Vec<TransferId> = self
            .networks
            .values()
            .filter(|n| n.peers.contains(&host))
            .flat_map(|n| n.transfers.iter().copied())
            .collect();
        for id in shared {
            self.announce(host, id);
        }
        link
    }

    /// Forgets a peer that went away, along with its networks. The node
    /// has dropped its channels.
    pub(crate) fn remove_peer(&mut self, host: ID) {
        self.peers.remove(&host);
        for n in self.networks.values_mut() {
            n.peers.remove(&host);
        }
    }

    /// The peer joined `net`, whether or not it is linked yet.
    pub(crate) fn add_network_peer(&mut self, net: N, host: ID) {
        let n = self.networks.entry(net).or_default();
        if !n.peers.insert(host) {
            return;
        }
        let ids: Vec<TransferId> = n.transfers.iter().copied().collect();
        for id in ids {
            self.announce(host, id);
        }
    }

    /// The peer left `net`: transfers it shares through no other network
    /// stop and close.
    pub(crate) fn remove_network_peer(&mut self, net: N, host: ID) {
        let Some(n) = self.networks.get_mut(&net) else {
            return;
        };
        if !n.peers.remove(&host) {
            return;
        }
        let ids: Vec<TransferId> = n.transfers.iter().copied().collect();
        self.close_unshared(host, &ids);
    }

    /// The node left `net`: its publications are withdrawn, and transfers
    /// shared with its peers through no other network stop and close.
    pub(crate) fn remove_network(&mut self, net: N) {
        let Some(n) = self.networks.remove(&net) else {
            return;
        };
        let ids: Vec<TransferId> = n.transfers.iter().copied().collect();
        for &id in &ids {
            if let Some(t) = self.transfers.get_mut(&id) {
                t.networks.remove(&net);
            }
            self.withdraw(id, net);
        }
        for &host in &n.peers {
            self.close_unshared(host, &ids);
        }
    }

    /// The peer broke the ppspp protocol, and the node stopped its swarm
    /// channels. Its transfers close, and none runs with it until it
    /// links again.
    pub(crate) fn transfer_failed(&mut self, host: ID) {
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        p.failed = true;
        let link = p.link;
        let mut closed = Vec::new();
        for (&id, pt) in &mut p.transfers {
            let running = matches!(pt.state, Channel::Running { .. });
            pt.state = Channel::Idle;
            closed.push((id, running));
        }
        for (id, running) in closed {
            // A run queued before the failure may have restarted it.
            if running && let Some(swarm) = self.transfers.get(&id).map(|t| t.swarm) {
                self.push_op(Op::Stop { host, link, swarm });
            }
            self.actions.push_back(Action::Close { host, link, id });
        }
    }

    fn shared(&self, host: ID, id: TransferId) -> bool {
        self.transfers.get(&id).is_some_and(|t| {
            t.networks.iter().any(|net| {
                self.networks
                    .get(net)
                    .is_some_and(|n| n.peers.contains(&host))
            })
        })
    }

    fn close_unshared(&mut self, host: ID, ids: &[TransferId]) {
        for &id in ids {
            if self.shared(host, id) {
                continue;
            }
            if let Some(swarm) = self.transfers.get(&id).map(|t| t.swarm) {
                self.drop_peer_transfer(host, id, swarm);
            }
        }
    }

    /// Announces `id` to `host` if it is linked, hasn't failed and isn't
    /// running it.
    fn announce(&mut self, host: ID, id: TransferId) {
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        if p.failed {
            return;
        }
        let next_channel = &mut self.next_channel;
        let pt = p.transfers.entry(id).or_insert_with(|| {
            *next_channel += 1;
            PeerTransfer {
                channel: *next_channel,
                state: Channel::Idle,
            }
        });
        if pt.state == Channel::Idle {
            self.actions.push_back(Action::Announce {
                host,
                link: p.link,
                id,
                channel: pt.channel,
            });
        }
    }

    /// Drops a peer transfer. It closes even when it isn't running: the
    /// peer may run it on an announce still in flight, and a close of a
    /// transfer the peer doesn't run changes nothing there.
    fn drop_peer_transfer(&mut self, host: ID, id: TransferId, swarm: S) {
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let link = p.link;
        let Some(pt) = p.transfers.remove(&id) else {
            return;
        };
        if matches!(pt.state, Channel::Running { .. }) {
            self.push_op(Op::Stop { host, link, swarm });
        }
        self.actions.push_back(Action::Close { host, link, id });
    }

    /// Whether another of the peer's transfers runs `swarm`: the runner
    /// allows one channel per swarm per peer.
    fn swarm_busy(&self, host: ID, id: TransferId, swarm: S) -> bool {
        self.peers.get(&host).is_some_and(|p| {
            p.transfers.iter().any(|(&other, pt)| {
                other != id
                    && matches!(pt.state, Channel::Running { .. })
                    && self.transfers.get(&other).is_some_and(|t| t.swarm == swarm)
            })
        })
    }

    /// The peer transfer of a current link that hasn't failed, and the
    /// transfer's swarm.
    fn peer_transfer(
        &mut self,
        host: ID,
        link: u64,
        id: TransferId,
    ) -> Option<(S, &mut PeerTransfer)> {
        let swarm = self.transfers.get(&id)?.swarm;
        let p = self.peers.get_mut(&host)?;
        if p.link != link || p.failed {
            return None;
        }
        Some((swarm, p.transfers.get_mut(&id)?))
    }

    // --- peer requests and answers ------------------------------------------

    /// The peer announces `id`, which its frames will carry on
    /// `peer_channel`.
    pub(crate) fn handle_announce(
        &mut self,
        host: ID,
        link: u64,
        id: TransferId,
        peer_channel: u64,
    ) -> Answer {
        let Some((swarm, pt)) = self.peer_transfer(host, link, id) else {
            return Answer::Refuse;
        };
        let channel = pt.channel;
        match pt.state {
            Channel::Running { peer_channel: pc } if pc == peer_channel => {
                return Answer::Channel(channel);
            }
            Channel::Running { .. } => {
                // The peer started the transfer afresh.
                pt.state = Channel::Running { peer_channel };
                self.push_op(Op::Stop { host, link, swarm });
            }
            Channel::Idle => {
                if self.swarm_busy(host, id, swarm) {
                    return Answer::Refuse;
                }
                if let Some((_, pt)) = self.peer_transfer(host, link, id) {
                    pt.state = Channel::Running { peer_channel };
                }
            }
        }
        let op = self.push_op(Op::Run {
            host,
            link,
            id,
            swarm,
            channel,
            peer_channel,
            notify: false,
        });
        Answer::After(channel, op)
    }

    /// The peer answered our announce of `id` on `channel`: with its
    /// channel if it runs the transfer, or `None`, and then it announces
    /// once it publishes the transfer.
    pub(crate) fn handle_announce_answer(
        &mut self,
        host: ID,
        link: u64,
        id: TransferId,
        channel: u64,
        answer: Option<u64>,
    ) {
        let Some(peer_channel) = answer else {
            return;
        };
        let Some((swarm, pt)) = self.peer_transfer(host, link, id) else {
            return;
        };
        if pt.channel != channel {
            return;
        }
        match pt.state {
            Channel::Running { peer_channel: pc } if pc == peer_channel => return,
            Channel::Running { .. } => {
                pt.state = Channel::Running { peer_channel };
                self.push_op(Op::Stop { host, link, swarm });
            }
            Channel::Idle => {
                if self.swarm_busy(host, id, swarm) {
                    // The peer runs its side.
                    self.actions.push_back(Action::Close { host, link, id });
                    return;
                }
                if let Some((_, pt)) = self.peer_transfer(host, link, id) {
                    pt.state = Channel::Running { peer_channel };
                }
            }
        }
        self.push_op(Op::Run {
            host,
            link,
            id,
            swarm,
            channel,
            peer_channel,
            notify: true,
        });
    }

    /// The peer closed `id`. The peer transfer stays, so a later announce
    /// restarts it. Returns the op that stops the channel.
    pub(crate) fn handle_close(&mut self, host: ID, link: u64, id: TransferId) -> Option<OpId> {
        let (swarm, pt) = self.peer_transfer(host, link, id)?;
        if pt.state == Channel::Idle {
            return None;
        }
        pt.state = Channel::Idle;
        Some(self.push_op(Op::Stop { host, link, swarm }))
    }

    /// A `Run` op failed.
    pub(crate) fn run_failed(
        &mut self,
        host: ID,
        link: u64,
        id: TransferId,
        channel: u64,
        peer_channel: u64,
        notify: bool,
    ) {
        let Some((_, pt)) = self.peer_transfer(host, link, id) else {
            return;
        };
        if pt.channel != channel || pt.state != (Channel::Running { peer_channel }) {
            return;
        }
        pt.state = Channel::Idle;
        if notify {
            self.actions.push_back(Action::Close { host, link, id });
        }
    }

    // --- discovery ----------------------------------------------------------

    /// When [`State::handle_timeout`] is next due.
    pub(crate) fn poll_timeout(&self) -> Option<Time> {
        self.searches.first().map(|&(at, _, _)| at)
    }

    /// Starts the searches that are due, except those still in flight,
    /// and schedules each again `search_interval` from now.
    pub(crate) fn handle_timeout(&mut self, now: Time) {
        let due: Vec<(Time, TransferId, N)> = self
            .searches
            .iter()
            .take_while(|&&(at, _, _)| at <= now)
            .copied()
            .collect();
        for entry @ (_, id, net) in due {
            self.searches.remove(&entry);
            self.searches.insert((now + self.search_interval, id, net));
            let Some(t) = self.transfers.get(&id) else {
                continue;
            };
            if self.searching.insert((id, net)) {
                self.actions.push_back(Action::Search {
                    id,
                    net,
                    key: t.swarm_id.as_bytes().to_vec(),
                    salt: t.salt.clone(),
                });
            }
        }
    }

    /// A search in flight found `host`. Asks the node to link to it
    /// unless it is this host, is linked, or was asked about within
    /// `connect_backoff`.
    pub(crate) fn search_found(&mut self, id: TransferId, net: N, host: ID, now: Time) {
        if !self.searching.contains(&(id, net)) || !self.is_published(id, net) {
            return;
        }
        self.contacted.retain(|_, until| *until > now);
        if host == self.host_id
            || self.peers.contains_key(&host)
            || self.contacted.contains_key(&host)
        {
            return;
        }
        while self.contacted.len() >= self.max_contacted {
            let Some(oldest) = self
                .contacted
                .iter()
                .min_by_key(|&(_, until)| *until)
                .map(|(&h, _)| h)
            else {
                break;
            };
            self.contacted.remove(&oldest);
        }
        self.contacted.insert(host, now + self.connect_backoff);
        self.push_op(Op::Connect { net, host });
    }

    /// A search ended.
    pub(crate) fn search_done(&mut self, id: TransferId, net: N) {
        self.searching.remove(&(id, net));
    }

    // --- applying ops -------------------------------------------------------

    /// Whether `link` is the peer's current link. An op for an earlier
    /// link is moot: the node dropped that link's channels with it.
    pub(crate) fn is_current(&self, host: ID, link: u64) -> bool {
        self.peers.get(&host).is_some_and(|p| p.link == link)
    }

    /// Whether a `Run` op is still wanted when its turn comes. A later
    /// transition that undid it queued its own ops behind it.
    pub(crate) fn wants_run(
        &self,
        host: ID,
        link: u64,
        id: TransferId,
        channel: u64,
        peer_channel: u64,
    ) -> bool {
        self.is_current(host, link) && self.running(host, id) == Some((channel, peer_channel))
    }

    // --- inspection ---------------------------------------------------------

    /// The channels of `id` with `host` while it runs: ours and the
    /// peer's.
    pub(crate) fn running(&self, host: ID, id: TransferId) -> Option<(u64, u64)> {
        let pt = self.peers.get(&host)?.transfers.get(&id)?;
        match pt.state {
            Channel::Running { peer_channel } => Some((pt.channel, peer_channel)),
            Channel::Idle => None,
        }
    }

    /// How many hosts are in the connect backoff.
    #[cfg(test)]
    pub(crate) fn contacted(&self) -> usize {
        self.contacted.len()
    }
}

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
