// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Network membership's state: the networks the node runs and its
//! certificates on them, each linked peer's runs, and the links that bind
//! peers to networks. Sans-IO: the control feeds it the driver's calls,
//! the peers' requests and answers, broker bytes and the time, and carries
//! out its actions.
//!
//! # Runs
//!
//! A run finds the networks two peers share and pairs their bindings.
//!
//! - **Starting.** A local trigger (the peer linking, the network set
//!   changing) starts a run after [`Config::debounce`] by calling the
//!   peer's `Negotiate` with this side's key count. The peer's
//!   `Negotiate` starts one too, answered with this side's own, unless a
//!   run is starting already: then it only records the count, which ends
//!   the exchange of answers. A `Negotiate` from the peer at any other
//!   point means the peer started over, so it ends this side's run.
//! - **PSI.** The session starts once this side's `Negotiate` is answered
//!   and the peer's count is known, and only then may broker bytes go
//!   out: the peer's run then exists to read every byte. Bytes that arrive
//!   earlier wait for the session, up to [`Config::max_broker_buffer`].
//! - **Open.** The receiver reserves a port for each shared key it has no
//!   link for and calls `Open` with them, in the order the PSI returned
//!   them. The sender answers with one binding per received binding, in
//!   the same order, or none if any is invalid; answers are positional, so
//!   both sides' lists must hold the same keys. Both sides then pair the
//!   bindings by network key.
//! - **Failure.** A run whose session stalls ([`Config::stall_timeout`]),
//!   outlasts [`Config::run_timeout`] or fails, or whose calls fail, backs
//!   off and starts again with fresh PSI seeds, up to
//!   [`Config::max_attempts`] runs in a row. A receiver whose cuckoo table
//!   failed completes the run, calls `Open` with no bindings, and starts
//!   again after [`Config::retry_delay`].
//!
//! # Links
//!
//! A key the peer may hold a link for stays out of this side's later
//! `Open`s to it, until a `Close` either way or the peer relinks: once a
//! binding is offered or answered, or a pairing fails partway. A link
//! opens, and the node binds the peer to the network, once both
//! certificates are trusted ([`membership_trusted`]).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use rand_core::Rng;
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_crypto::{
    Error as CryptoError, certificate_chain, certificate_network_key, membership_trusted,
    verify_certificate,
};
use strims_kademlia::ID;
use strims_mpc::{Block, Error as MpcError, PszReceiver, PszSender};
use strims_proto::network::v1::NetworkPeerBinding;
use strims_proto::types::Certificate;

use crate::{Config, Metrics};

/// A network's key: its root certificate's key.
pub(crate) type NetKey = Vec<u8>;

/// A node operation's place in the order the node applies them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OpId(u64);

/// A node operation. The node applies them one at a time, in `OpId`
/// order; ops for a peer's earlier link ([`State::is_current`]) are
/// skipped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Op<N> {
    /// Join the network, and report it with [`State::network_added`].
    AddNetwork {
        key: NetKey,
    },
    /// Leave the network, which unbinds its peers.
    RemoveNetwork {
        net: N,
    },
    /// Reserve vnic ports on the peer, and report them with
    /// [`State::ports_reserved`].
    ReservePorts {
        host: ID,
        link: u64,
        run: u64,
        count: usize,
    },
    ReleasePort {
        host: ID,
        link: u64,
        port: u16,
    },
    /// Bind the peer to the network: frames on `local_port` are the
    /// network's, and it sends to `peer_port`.
    Bind {
        host: ID,
        link: u64,
        net: N,
        local_port: u16,
        peer_port: u16,
    },
    Unbind {
        host: ID,
        link: u64,
        net: N,
    },
}

/// What the control must do. Calls to one peer go out one at a time, in
/// order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Action<N> {
    Op(OpId, Op<N>),
    /// Call the peer's `Negotiate`, and report the answer with
    /// [`State::negotiate_answered`].
    Negotiate {
        host: ID,
        link: u64,
        run: u64,
        count: u32,
    },
    /// Call the peer's `Open`, and report the answer with
    /// [`State::open_answered`].
    Open {
        host: ID,
        link: u64,
        run: u64,
        bindings: Vec<NetworkPeerBinding>,
    },
    /// Call the peer's `Close`, and report the answer with
    /// [`State::close_answered`].
    Close {
        host: ID,
        link: u64,
        key: NetKey,
    },
    /// Call the peer's `UpdateCertificate`.
    UpdateCertificate {
        host: ID,
        link: u64,
        certificate: Certificate,
    },
    /// The number of peers bound to the network changed.
    PeerCount {
        key: NetKey,
        count: usize,
    },
    /// The peer shares the network and its certificate is trusted,
    /// while this side's isn't: the peer can forward a renewal to the
    /// network's CA.
    RenewVia {
        host: ID,
        link: u64,
        key: NetKey,
    },
    /// This side's certificate on the network is due for renewal.
    Renew {
        key: NetKey,
    },
}

/// Why a peer's request is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Refusal {
    #[error("peer not linked")]
    NotLinked,
    #[error("certificate missing")]
    MissingCertificate,
    #[error("invalid certificate")]
    InvalidCertificate,
    #[error("certificate key does not match the peer")]
    OwnerMismatch,
    #[error("provisional certificate")]
    Provisional,
    #[error("network binding not found")]
    BindingNotFound,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Sender,
    Receiver,
}

/// A run's PSI session.
enum Psi {
    Sender(PszSender),
    Receiver(PszReceiver),
}

impl fmt::Debug for Psi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Psi::Sender(_) => f.write_str("Psi::Sender"),
            Psi::Receiver(_) => f.write_str("Psi::Receiver"),
        }
    }
}

/// How a session ended.
enum PsiOutcome {
    /// The sender finished.
    Sent,
    /// The receiver finished with the shared keys.
    Shared(Vec<NetKey>),
    /// The receiver's cuckoo table failed; nothing is left unread.
    CuckooFailed,
    Failed(MpcError),
}

impl Psi {
    fn handle_input(&mut self, data: &[u8]) -> Result<(), MpcError> {
        match self {
            Psi::Sender(s) => s.handle_input(data),
            Psi::Receiver(r) => r.handle_input(data),
        }
    }

    fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize> {
        match self {
            Psi::Sender(s) => s.poll_transmit(buf),
            Psi::Receiver(r) => r.poll_transmit(buf),
        }
    }

    fn outcome(&mut self) -> Option<PsiOutcome> {
        match self {
            Psi::Sender(s) => match s.result()? {
                Ok(()) => Some(PsiOutcome::Sent),
                Err(e) => Some(PsiOutcome::Failed(e)),
            },
            Psi::Receiver(r) => match r.take_result()? {
                Ok(keys) => Some(PsiOutcome::Shared(keys)),
                Err(MpcError::CuckooHashMapFull) => Some(PsiOutcome::CuckooFailed),
                Err(e) => Some(PsiOutcome::Failed(e)),
            },
        }
    }
}

/// A peer's binding that passed the checks.
#[derive(Clone, Debug)]
struct PeerBinding {
    key: NetKey,
    port: u16,
    certificate: Certificate,
}

/// What a run does with the ports it reserves.
#[derive(Debug)]
enum Plan {
    /// Receiver: offer the shared keys without a link, in PSI order.
    Offer(Vec<NetKey>),
    /// Sender: answer the receiver's bindings, in its order.
    Answer(Vec<PeerBinding>),
}

#[derive(Debug)]
enum Run {
    Idle,
    /// A local trigger waits out the debounce, a retry its delay, or a
    /// failed run its backoff.
    Pending {
        at: Time,
    },
    Starting {
        run: u64,
        /// This side's keys, sorted.
        keys: Vec<NetKey>,
        /// When this side's `Negotiate` goes out, if it hasn't.
        send_at: Option<Time>,
        answered: bool,
        peer_count: Option<u32>,
        deadline: Time,
        cap: Time,
    },
    /// Waiting for a session slot.
    Queued {
        run: u64,
        keys: Vec<NetKey>,
        peer_count: u32,
        cap: Time,
    },
    Running {
        run: u64,
        role: Role,
        psi: Psi,
        stall: Time,
        cap: Time,
    },
    /// Waiting for ports for this side's `Open`.
    Reserving {
        run: u64,
        plan: Plan,
        deadline: Time,
    },
    /// Receiver: this side's `Open` is out; waiting for the peer's.
    Offered {
        run: u64,
        offered: Vec<(NetKey, u16)>,
        retry: bool,
        deadline: Time,
    },
    /// Sender: waiting for the receiver's `Open`.
    AwaitOpen {
        run: u64,
        deadline: Time,
    },
    /// Sender: this side's answer is out; waiting for it to be answered.
    Answered {
        run: u64,
        keys: Vec<NetKey>,
        deadline: Time,
    },
}

impl Run {
    fn id(&self) -> Option<u64> {
        match *self {
            Run::Idle | Run::Pending { .. } => None,
            Run::Starting { run, .. }
            | Run::Queued { run, .. }
            | Run::Running { run, .. }
            | Run::Reserving { run, .. }
            | Run::Offered { run, .. }
            | Run::AwaitOpen { run, .. }
            | Run::Answered { run, .. } => Some(run),
        }
    }

    fn timeout(&self) -> Option<Time> {
        match *self {
            Run::Idle => None,
            Run::Pending { at } => Some(at),
            Run::Starting {
                send_at,
                deadline,
                cap,
                ..
            } => Some(send_at.map_or(deadline, |s| s.min(deadline)).min(cap)),
            Run::Queued { cap, .. } => Some(cap),
            Run::Running { stall, cap, .. } => Some(stall.min(cap)),
            Run::Reserving { deadline, .. }
            | Run::Offered { deadline, .. }
            | Run::AwaitOpen { deadline, .. }
            | Run::Answered { deadline, .. } => Some(deadline),
        }
    }
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Done,
    /// The receiver's cuckoo table failed: run again soon.
    Retry,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Link {
    /// This side's binding is offered; the peer's hasn't arrived.
    Offered { local_port: u16 },
    Paired {
        local_port: u16,
        peer_port: u16,
        local_trusted: bool,
        peer_trusted: bool,
        open: bool,
    },
    /// The pairing failed partway: the peer may hold its side.
    Dead,
    /// This side left the network and its `Close` is out.
    Closing,
}

impl Link {
    fn local_port(&self) -> Option<u16> {
        match *self {
            Link::Offered { local_port } | Link::Paired { local_port, .. } => Some(local_port),
            Link::Dead | Link::Closing => None,
        }
    }

    fn is_open(&self) -> bool {
        matches!(self, Link::Paired { open: true, .. })
    }
}

#[derive(Debug)]
struct Peer {
    link: u64,
    profile_key: Vec<u8>,
    run: Run,
    /// Runs in a row that failed.
    attempts: u32,
    /// A local trigger arrived during a run: start another once it ends.
    rerun: bool,
    /// When the peer last started a run, which paces its next.
    peer_started: Option<Time>,
    /// Bound: one per local network, plus the ones this side is leaving.
    links: BTreeMap<NetKey, Link>,
    /// Broker bytes that arrived before the session started. Bound:
    /// `max_broker_buffer`.
    broker_in: Vec<u8>,
    /// A finished session's last bytes, still to send. Bound: one
    /// session's final message.
    broker_out: Vec<u8>,
    /// The keys the last run found shared. Bound: the local networks.
    shared: BTreeSet<NetKey>,
}

#[derive(Debug)]
struct LocalNetwork<N> {
    id: u64,
    certificate: Certificate,
    /// `None` until the node has joined it.
    net: Option<N>,
    /// Left while the node was joining it.
    removed: bool,
    /// When this side's certificate is next renewed.
    renew_at: Option<Time>,
}

/// See the [module docs](self).
#[derive(Debug)]
pub(crate) struct State<N> {
    host_id: ID,
    config: Config,
    rng: DetRng,
    /// Bound: the networks the driver reports.
    networks: BTreeMap<NetKey, LocalNetwork<N>>,
    /// Linked peers. Bound: the node's peers.
    peers: BTreeMap<ID, Peer>,
    /// Peers waiting for a session slot, in order. Bound: the peers.
    queue: VecDeque<ID>,
    sessions: usize,
    next_link: u64,
    next_run: u64,
    next_op: u64,
    metrics: Metrics,
    /// Drained by the control after every call.
    actions: VecDeque<Action<N>>,
}

impl<N: Copy + Ord + fmt::Debug> State<N> {
    pub(crate) fn new(host_id: ID, config: &Config, rng: DetRng) -> Self {
        State {
            host_id,
            config: *config,
            rng,
            networks: BTreeMap::new(),
            peers: BTreeMap::new(),
            queue: VecDeque::new(),
            sessions: 0,
            next_link: 0,
            next_run: 0,
            next_op: 0,
            metrics: Metrics::default(),
            actions: VecDeque::new(),
        }
    }

    pub(crate) fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    pub(crate) fn metrics_mut(&mut self) -> &mut Metrics {
        &mut self.metrics
    }

    /// The next thing to do, in order.
    pub(crate) fn poll_action(&mut self) -> Option<Action<N>> {
        self.actions.pop_front()
    }

    fn op(&mut self, op: Op<N>) {
        let id = OpId(self.next_op);
        self.next_op += 1;
        self.actions.push_back(Action::Op(id, op));
    }

    // --- networks -----------------------------------------------------------

    /// Starts running network `id`, whose key is `key`, with this side's
    /// `certificate` on it: the node joins it, and then every peer runs
    /// again. A network already running gets the certificate.
    pub(crate) fn add_network(
        &mut self,
        key: NetKey,
        id: u64,
        certificate: Certificate,
        now: Time,
    ) {
        if let Some(n) = self.networks.get_mut(&key) {
            n.id = id;
            n.removed = false;
            self.set_certificate(&key, certificate, now);
            return;
        }
        self.networks.insert(
            key.clone(),
            LocalNetwork {
                id,
                certificate,
                net: None,
                removed: false,
                renew_at: None,
            },
        );
        self.op(Op::AddNetwork { key });
    }

    /// The node joined the network as `net`, or failed to (`None`).
    pub(crate) fn network_added(&mut self, key: &[u8], net: Option<N>, now: Time) {
        let Some(n) = self.networks.get_mut(key) else {
            if let Some(net) = net {
                self.op(Op::RemoveNetwork { net });
            }
            return;
        };
        match net {
            None => {
                self.networks.remove(key);
            }
            Some(net) if n.removed => {
                self.networks.remove(key);
                self.op(Op::RemoveNetwork { net });
            }
            Some(net) => {
                n.net = Some(net);
                self.trigger_all(now);
            }
        }
    }

    /// Replaces this side's certificate on a network: peers with a link
    /// get it, and links whose certificates are now both trusted open.
    pub(crate) fn set_certificate(&mut self, key: &[u8], certificate: Certificate, now: Time) {
        let Some(n) = self.networks.get_mut(key) else {
            return;
        };
        if n.certificate == certificate {
            return;
        }
        n.certificate = certificate.clone();
        let net = n.net;
        let trusted = membership_trusted(&certificate, now);
        let mut opened = Vec::new();
        for (host, p) in &mut self.peers {
            let Some(link) = p.links.get_mut(key) else {
                continue;
            };
            if matches!(link, Link::Dead | Link::Closing) {
                continue;
            }
            self.actions.push_back(Action::UpdateCertificate {
                host: *host,
                link: p.link,
                certificate: certificate.clone(),
            });
            if let Link::Paired {
                local_trusted,
                peer_trusted,
                open,
                local_port,
                peer_port,
            } = link
            {
                *local_trusted = trusted;
                if trusted && *peer_trusted && !*open {
                    *open = true;
                    opened.push((*host, p.link, *local_port, *peer_port));
                }
            }
        }
        self.open_links(key, net, opened);
    }

    /// Stops running a network: peers with a link get a `Close`, and the
    /// node leaves it.
    pub(crate) fn remove_network(&mut self, key: &[u8]) {
        let Some(n) = self.networks.get_mut(key) else {
            return;
        };
        let Some(net) = n.net else {
            n.removed = true;
            return;
        };
        self.networks.remove(key);
        for (host, p) in &mut self.peers {
            p.shared.remove(key);
            let Some(link) = p.links.get_mut(key) else {
                continue;
            };
            if let Some(port) = link.local_port() {
                let op = Op::ReleasePort {
                    host: *host,
                    link: p.link,
                    port,
                };
                let id = OpId(self.next_op);
                self.next_op += 1;
                self.actions.push_back(Action::Op(id, op));
            }
            *link = Link::Closing;
            self.actions.push_back(Action::Close {
                host: *host,
                link: p.link,
                key: key.to_vec(),
            });
        }
        // the node unbinds the network's peers as it leaves it
        self.op(Op::RemoveNetwork { net });
        self.actions.push_back(Action::PeerCount {
            key: key.to_vec(),
            count: 0,
        });
    }

    /// Renews this side's certificate on the network at `at`, or not.
    pub(crate) fn schedule_renewal(&mut self, key: &[u8], at: Option<Time>) {
        if let Some(n) = self.networks.get_mut(key) {
            n.renew_at = at;
        }
    }

    /// The peers that share the network and hold a trusted certificate on
    /// it, with their links.
    pub(crate) fn trusted_peers(&self, key: &[u8]) -> Vec<(ID, u64)> {
        self.peers
            .iter()
            .filter(|(_, p)| {
                matches!(
                    p.links.get(key),
                    Some(Link::Paired {
                        peer_trusted: true,
                        ..
                    })
                )
            })
            .map(|(h, p)| (*h, p.link))
            .collect()
    }

    /// This side's certificate on a network.
    pub(crate) fn certificate(&self, key: &[u8]) -> Option<&Certificate> {
        self.networks.get(key).map(|n| &n.certificate)
    }

    /// The network's id and the node's handle for it, once joined.
    pub(crate) fn network(&self, key: &[u8]) -> Option<(u64, N)> {
        let n = self.networks.get(key)?;
        Some((n.id, n.net?))
    }

    /// The keys of the networks the node has joined, sorted.
    pub(crate) fn keys(&self) -> Vec<NetKey> {
        self.networks
            .iter()
            .filter(|(_, n)| n.net.is_some())
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// The peers bound to the network.
    pub(crate) fn peer_count(&self, key: &[u8]) -> usize {
        self.peers
            .values()
            .filter(|p| p.links.get(key).is_some_and(Link::is_open))
            .count()
    }

    // --- peers --------------------------------------------------------------

    /// Takes a newly linked peer, replacing an earlier link to it, and
    /// starts a run with it. Returns the link's id.
    pub(crate) fn add_peer(&mut self, host: ID, profile_key: Vec<u8>, now: Time) -> u64 {
        self.remove_peer(host, now);
        self.next_link += 1;
        let link = self.next_link;
        self.peers.insert(
            host,
            Peer {
                link,
                profile_key,
                run: Run::Idle,
                attempts: 0,
                rerun: false,
                peer_started: None,
                links: BTreeMap::new(),
                broker_in: Vec::new(),
                broker_out: Vec::new(),
                shared: BTreeSet::new(),
            },
        );
        self.trigger(host, now);
        link
    }

    /// Forgets a peer that went away. The node dropped its bindings and
    /// ports with it.
    pub(crate) fn remove_peer(&mut self, host: ID, now: Time) {
        let Some(p) = self.peers.remove(&host) else {
            return;
        };
        if matches!(p.run, Run::Running { .. }) {
            self.sessions = self.sessions.saturating_sub(1);
        }
        self.queue.retain(|h| *h != host);
        for (key, link) in &p.links {
            if link.is_open() {
                self.count_changed(key);
            }
        }
        self.start_queued(now);
    }

    /// Whether `link` is the peer's current link.
    pub(crate) fn is_current(&self, host: ID, link: u64) -> bool {
        self.peers.get(&host).is_some_and(|p| p.link == link)
    }

    /// Whether the last run with the peer found the network shared.
    pub(crate) fn shares(&self, host: ID, link: u64, key: &[u8]) -> bool {
        self.peers
            .get(&host)
            .is_some_and(|p| p.link == link && p.shared.contains(key))
    }

    /// The peer's profile key.
    pub(crate) fn profile_key(&self, host: ID, link: u64) -> Option<&[u8]> {
        let p = self.peers.get(&host).filter(|p| p.link == link)?;
        Some(&p.profile_key)
    }

    fn trigger_all(&mut self, now: Time) {
        let hosts: Vec<ID> = self.peers.keys().copied().collect();
        for host in hosts {
            self.trigger(host, now);
        }
    }

    /// A local reason to run with the peer.
    fn trigger(&mut self, host: ID, now: Time) {
        let debounce = self.config.debounce;
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        p.attempts = 0;
        match p.run {
            Run::Idle => p.run = Run::Pending { at: now + debounce },
            Run::Pending { .. } => {}
            _ => p.rerun = true,
        }
    }

    // --- runs ---------------------------------------------------------------

    /// Starts a run with this side's keys: its `Negotiate` goes out now,
    /// or at `send_at`.
    fn start_run(&mut self, host: ID, now: Time, peer_count: Option<u32>, send_at: Option<Time>) {
        let keys = self.keys();
        let negotiate_timeout = self.config.negotiate_timeout;
        let cap = now + self.config.run_timeout;
        self.next_run += 1;
        let run = self.next_run;
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        p.broker_in.clear();
        p.rerun = false;
        self.metrics.runs += 1;
        let start = send_at.unwrap_or(now);
        p.run = Run::Starting {
            run,
            keys,
            send_at: Some(start),
            answered: false,
            peer_count,
            deadline: start + negotiate_timeout,
            cap: cap.max(start + negotiate_timeout),
        };
        self.send_negotiate(host, now);
    }

    /// Sends a starting run's `Negotiate` once its time has come.
    fn send_negotiate(&mut self, host: ID, now: Time) {
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let Run::Starting {
            run,
            ref keys,
            ref mut send_at,
            ..
        } = p.run
        else {
            return;
        };
        if send_at.is_none_or(|at| now.before(at)) {
            return;
        }
        *send_at = None;
        let count = u32::try_from(keys.len()).unwrap_or(u32::MAX);
        self.actions.push_back(Action::Negotiate {
            host,
            link: p.link,
            run,
            count,
        });
    }

    /// Takes the peer's `Negotiate`; see the [module docs](self).
    pub(crate) fn handle_negotiate(
        &mut self,
        host: ID,
        link: u64,
        count: u32,
        now: Time,
    ) -> Result<(), Refusal> {
        let interval = self.config.min_peer_run_interval;
        let p = self
            .peers
            .get_mut(&host)
            .filter(|p| p.link == link)
            .ok_or(Refusal::NotLinked)?;
        if let Run::Starting { peer_count, .. } = &mut p.run {
            *peer_count = Some(count);
            self.try_start_session(host, now);
            return Ok(());
        }
        let send_at = p
            .peer_started
            .map(|t| t + interval)
            .filter(|at| now.before(*at));
        p.peer_started = Some(send_at.unwrap_or(now));
        if send_at.is_some() {
            self.metrics.runs_paced += 1;
        }
        self.abort(host);
        self.start_run(host, now, Some(count), send_at);
        Ok(())
    }

    /// The answer to this side's `Negotiate`.
    pub(crate) fn negotiate_answered(
        &mut self,
        host: ID,
        link: u64,
        run: u64,
        ok: bool,
        now: Time,
    ) {
        let Some(p) = self.peers.get_mut(&host).filter(|p| p.link == link) else {
            return;
        };
        let Run::Starting {
            run: current,
            ref mut answered,
            ..
        } = p.run
        else {
            return;
        };
        if current != run {
            return;
        }
        if !ok {
            self.finish(host, Outcome::Failed, now);
            return;
        }
        *answered = true;
        self.try_start_session(host, now);
    }

    /// Starts a starting run's session once both counts are in, or ends
    /// the run if either side has no keys.
    fn try_start_session(&mut self, host: ID, now: Time) {
        let max_peer_keys = self.config.max_peer_keys;
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let Run::Starting {
            run,
            ref mut keys,
            send_at: None,
            answered: true,
            peer_count: Some(peer_count),
            cap,
            ..
        } = p.run
        else {
            return;
        };
        if keys.is_empty() || peer_count == 0 {
            self.finish(host, Outcome::Done, now);
            return;
        }
        if peer_count as usize > max_peer_keys {
            p.attempts = self.config.max_attempts;
            self.finish(host, Outcome::Failed, now);
            return;
        }
        let keys = std::mem::take(keys);
        p.run = Run::Queued {
            run,
            keys,
            peer_count,
            cap,
        };
        self.queue.push_back(host);
        self.start_queued(now);
    }

    /// Starts queued sessions while there are slots.
    fn start_queued(&mut self, now: Time) {
        while self.sessions < self.config.max_sessions {
            let Some(host) = self.queue.pop_front() else {
                return;
            };
            self.start_session(host, now);
        }
    }

    fn start_session(&mut self, host: ID, now: Time) {
        let host_id = self.host_id;
        let stall_timeout = self.config.stall_timeout;
        let max_peer_keys = self.config.max_peer_keys;
        let mut seed = Block::default();
        self.rng.fill_bytes(&mut seed);
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let Run::Queued {
            run,
            ref mut keys,
            peer_count,
            cap,
        } = p.run
        else {
            return;
        };
        let keys = std::mem::take(keys);
        let count = keys.len();
        let peer_count = peer_count as usize;
        let role = if count > peer_count || (count == peer_count && host < host_id) {
            Role::Sender
        } else {
            Role::Receiver
        };
        let psi = match role {
            Role::Sender => Psi::Sender(PszSender::with_peer_limit(keys, seed, max_peer_keys)),
            Role::Receiver => {
                Psi::Receiver(PszReceiver::with_peer_limit(keys, seed, max_peer_keys))
            }
        };
        p.run = Run::Running {
            run,
            role,
            psi,
            stall: now + stall_timeout,
            cap,
        };
        self.sessions += 1;
        let buffered = std::mem::take(&mut p.broker_in);
        if !buffered.is_empty() {
            self.session_input(host, &buffered, now);
        }
    }

    /// Bytes from the peer's broker channel.
    pub(crate) fn handle_broker_input(&mut self, host: ID, link: u64, data: &[u8], now: Time) {
        let max = self.config.max_broker_buffer;
        let Some(p) = self.peers.get_mut(&host).filter(|p| p.link == link) else {
            return;
        };
        self.metrics.broker_bytes_read += data.len() as u64;
        match p.run {
            Run::Starting { .. } | Run::Queued { .. } => {
                if p.broker_in.len() + data.len() > max {
                    self.finish(host, Outcome::Failed, now);
                    return;
                }
                p.broker_in.extend_from_slice(data);
            }
            Run::Running { .. } => self.session_input(host, data, now),
            // nothing of this side's runs reads them
            _ => {}
        }
    }

    fn session_input(&mut self, host: ID, data: &[u8], now: Time) {
        let stall_timeout = self.config.stall_timeout;
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let Run::Running {
            ref mut psi,
            ref mut stall,
            ..
        } = p.run
        else {
            return;
        };
        *stall = now + stall_timeout;
        // an error is also the session's outcome
        let _ = psi.handle_input(data);
        self.check_session(host, now);
    }

    /// Bytes for the peer's broker channel, while a session runs.
    pub(crate) fn poll_broker_transmit(
        &mut self,
        host: ID,
        link: u64,
        buf: &mut [u8],
        now: Time,
    ) -> Option<usize> {
        let stall_timeout = self.config.stall_timeout;
        let p = self.peers.get_mut(&host).filter(|p| p.link == link)?;
        let n = if p.broker_out.is_empty() {
            let Run::Running {
                ref mut psi,
                ref mut stall,
                ..
            } = p.run
            else {
                return None;
            };
            let n = psi.poll_transmit(buf)?;
            *stall = now + stall_timeout;
            n
        } else {
            let n = p.broker_out.len().min(buf.len());
            let (dst, _) = buf.split_at_mut(n);
            dst.copy_from_slice(p.broker_out.get(..n)?);
            p.broker_out.drain(..n);
            n
        };
        if n == 0 {
            return None;
        }
        self.metrics.broker_bytes_written += n as u64;
        Some(n)
    }

    /// Moves a finished session's run on.
    fn check_session(&mut self, host: ID, now: Time) {
        let exchange_timeout = self.config.exchange_timeout;
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let Run::Running {
            run,
            role,
            ref mut psi,
            ..
        } = p.run
        else {
            return;
        };
        let Some(outcome) = psi.outcome() else {
            return;
        };
        // the session's last message still has to go out
        let mut buf = [0u8; 4096];
        while let Some(n) = psi.poll_transmit(&mut buf) {
            p.broker_out
                .extend_from_slice(buf.get(..n).unwrap_or_default());
        }
        self.sessions = self.sessions.saturating_sub(1);
        p.run = Run::Idle;
        let deadline = now + exchange_timeout;
        match (role, outcome) {
            (Role::Sender, PsiOutcome::Sent) => {
                p.run = Run::AwaitOpen { run, deadline };
            }
            (Role::Receiver, PsiOutcome::Shared(keys)) => {
                p.shared = keys.iter().cloned().collect();
                let offer: Vec<NetKey> = keys
                    .into_iter()
                    .filter(|k| !p.links.contains_key(k) && self.networks.contains_key(k))
                    .collect();
                self.offer(host, run, offer, false, now);
            }
            (Role::Receiver, PsiOutcome::CuckooFailed) => {
                self.offer(host, run, Vec::new(), true, now);
            }
            (_, outcome) => {
                if let PsiOutcome::Failed(e) = outcome {
                    tracing::debug!(peer = %host, error = %e, "network psi failed");
                }
                self.finish(host, Outcome::Failed, now);
            }
        }
        self.start_queued(now);
    }

    /// Receiver: offers bindings for `keys`, reserving their ports first.
    fn offer(&mut self, host: ID, run: u64, keys: Vec<NetKey>, retry: bool, now: Time) {
        let deadline = now + self.config.exchange_timeout;
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let link = p.link;
        if keys.is_empty() {
            self.actions.push_back(Action::Open {
                host,
                link,
                run,
                bindings: Vec::new(),
            });
            let outcome = if retry { Outcome::Retry } else { Outcome::Done };
            self.finish(host, outcome, now);
            return;
        }
        let count = keys.len();
        p.run = Run::Reserving {
            run,
            plan: Plan::Offer(keys),
            deadline,
        };
        self.op(Op::ReservePorts {
            host,
            link,
            run,
            count,
        });
    }

    /// The ports for a run's `Open`, or `None` if the node couldn't
    /// reserve them.
    pub(crate) fn ports_reserved(
        &mut self,
        host: ID,
        link: u64,
        run: u64,
        ports: Option<Vec<u16>>,
        now: Time,
    ) {
        let current = self
            .peers
            .get(&host)
            .filter(|p| p.link == link)
            .and_then(|p| match p.run {
                Run::Reserving { run: r, .. } if r == run => Some(()),
                _ => None,
            });
        if current.is_none() {
            for port in ports.into_iter().flatten() {
                self.op(Op::ReleasePort { host, link, port });
            }
            return;
        }
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let Run::Reserving { plan, .. } = std::mem::replace(&mut p.run, Run::Idle) else {
            return;
        };
        let ports = ports.filter(|ports| {
            ports.len()
                == match &plan {
                    Plan::Offer(k) => k.len(),
                    Plan::Answer(b) => b.len(),
                }
        });
        match (plan, ports) {
            (Plan::Offer(keys), Some(ports)) => self.send_offer(host, run, keys, ports, now),
            (Plan::Answer(bindings), Some(ports)) => {
                self.send_answer(host, run, bindings, ports, now)
            }
            (_, ports) => {
                for port in ports.into_iter().flatten() {
                    self.op(Op::ReleasePort { host, link, port });
                }
                // the peer waits for an Open
                self.actions.push_back(Action::Open {
                    host,
                    link,
                    run,
                    bindings: Vec::new(),
                });
                self.metrics.host_errors += 1;
                self.finish(host, Outcome::Failed, now);
            }
        }
    }

    fn send_offer(&mut self, host: ID, run: u64, keys: Vec<NetKey>, ports: Vec<u16>, now: Time) {
        let deadline = now + self.config.exchange_timeout;
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let mut offered = Vec::new();
        let mut bindings = Vec::new();
        let mut unused = Vec::new();
        for (key, port) in keys.into_iter().zip(ports) {
            match self.networks.get(&key) {
                Some(n) if !p.links.contains_key(&key) => {
                    bindings.push(NetworkPeerBinding {
                        port: u32::from(port),
                        certificate: Some(n.certificate.clone()),
                    });
                    p.links
                        .insert(key.clone(), Link::Offered { local_port: port });
                    offered.push((key, port));
                }
                _ => unused.push(port),
            }
        }
        let link = p.link;
        p.run = Run::Offered {
            run,
            offered,
            retry: false,
            deadline,
        };
        for port in unused {
            self.op(Op::ReleasePort { host, link, port });
        }
        self.actions.push_back(Action::Open {
            host,
            link,
            run,
            bindings,
        });
    }

    fn send_answer(
        &mut self,
        host: ID,
        run: u64,
        received: Vec<PeerBinding>,
        ports: Vec<u16>,
        now: Time,
    ) {
        let deadline = now + self.config.exchange_timeout;
        let mut bindings = Vec::new();
        let mut keys = Vec::new();
        for (b, port) in received.into_iter().zip(ports) {
            let Some(certificate) = self.certificate(&b.key).cloned() else {
                continue;
            };
            bindings.push(NetworkPeerBinding {
                port: u32::from(port),
                certificate: Some(certificate.clone()),
            });
            keys.push(b.key.clone());
            self.pair(host, b, port, &certificate, now);
        }
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let link = p.link;
        p.run = Run::Answered {
            run,
            keys,
            deadline,
        };
        self.actions.push_back(Action::Open {
            host,
            link,
            run,
            bindings,
        });
    }

    /// Takes the peer's `Open`. Only a run that has finished its session
    /// expects one; any other is left over from an earlier run.
    pub(crate) fn handle_open(
        &mut self,
        host: ID,
        link: u64,
        bindings: Vec<NetworkPeerBinding>,
        now: Time,
    ) -> Result<(), Refusal> {
        let p = self
            .peers
            .get_mut(&host)
            .filter(|p| p.link == link)
            .ok_or(Refusal::NotLinked)?;
        match p.run {
            Run::Offered { .. } => self.take_answer(host, bindings, now),
            Run::AwaitOpen { run, .. } => self.take_offer(host, run, bindings, now),
            _ => {}
        }
        Ok(())
    }

    /// Checks a peer's binding: an unexpired-or-expired but otherwise
    /// valid certificate of the peer's on one of `keys`, and a port.
    fn check_binding(
        &self,
        profile_key: &[u8],
        keys: &BTreeSet<&NetKey>,
        b: &NetworkPeerBinding,
        now: Time,
    ) -> Option<PeerBinding> {
        let certificate = b.certificate.as_ref()?;
        if certificate_chain(certificate).len() > self.config.max_chain_depth
            || certificate.key != profile_key
        {
            return None;
        }
        let key = certificate_network_key(certificate).to_vec();
        if !keys.contains(&key) {
            return None;
        }
        let port = u16::try_from(b.port).ok()?;
        if let Err(e) = verify_certificate(certificate, now)
            && e.errors() != [CryptoError::NotAfterRange]
        {
            return None;
        }
        Some(PeerBinding {
            key,
            port,
            certificate: certificate.clone(),
        })
    }

    /// Sender: takes the receiver's bindings, answering every one or
    /// none.
    fn take_offer(&mut self, host: ID, run: u64, bindings: Vec<NetworkPeerBinding>, now: Time) {
        let Some(p) = self.peers.get(&host) else {
            return;
        };
        let link = p.link;
        let keys: BTreeSet<&NetKey> = self
            .networks
            .iter()
            .filter(|(_, n)| n.net.is_some())
            .map(|(k, _)| k)
            .collect();
        let mut checked = Vec::new();
        let mut seen = BTreeSet::new();
        let mut valid = bindings.len() <= keys.len();
        for b in bindings.iter().take(keys.len()) {
            match self.check_binding(&p.profile_key, &keys, b, now) {
                Some(b) if seen.insert(b.key.clone()) => checked.push(b),
                _ => valid = false,
            }
        }
        if !valid {
            self.metrics.bindings_refused += 1;
            self.actions.push_back(Action::Open {
                host,
                link,
                run,
                bindings: Vec::new(),
            });
            self.finish(host, Outcome::Failed, now);
            return;
        }
        let shared: BTreeSet<NetKey> = checked.iter().map(|b| b.key.clone()).collect();
        if let Some(p) = self.peers.get_mut(&host) {
            p.shared = shared;
        }
        if checked.is_empty() {
            self.actions.push_back(Action::Open {
                host,
                link,
                run,
                bindings: Vec::new(),
            });
            self.finish(host, Outcome::Done, now);
            return;
        }
        let deadline = now + self.config.exchange_timeout;
        let count = checked.len();
        if let Some(p) = self.peers.get_mut(&host) {
            p.run = Run::Reserving {
                run,
                plan: Plan::Answer(checked),
                deadline,
            };
        }
        self.op(Op::ReservePorts {
            host,
            link,
            run,
            count,
        });
    }

    /// Receiver: pairs the sender's answer with this side's offer.
    fn take_answer(&mut self, host: ID, bindings: Vec<NetworkPeerBinding>, now: Time) {
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let Run::Offered { offered, retry, .. } = std::mem::replace(&mut p.run, Run::Idle) else {
            return;
        };
        let link = p.link;
        let profile_key = p.profile_key.clone();
        let answered_none = bindings.is_empty();
        let keys: BTreeSet<&NetKey> = offered.iter().map(|(k, _)| k).collect();
        let mut by_key: BTreeMap<NetKey, PeerBinding> = BTreeMap::new();
        for b in bindings.iter().take(offered.len()) {
            match self.check_binding(&profile_key, &keys, b, now) {
                Some(b) => {
                    by_key.entry(b.key.clone()).or_insert(b);
                }
                None => self.metrics.bindings_refused += 1,
            }
        }
        for (key, port) in offered {
            let certificate = self.certificate(&key).cloned();
            match (by_key.remove(&key), certificate) {
                (Some(b), Some(certificate)) => self.pair(host, b, port, &certificate, now),
                (_, _) => {
                    self.op(Op::ReleasePort { host, link, port });
                    if let Some(p) = self.peers.get_mut(&host) {
                        if answered_none {
                            p.links.remove(&key);
                        } else {
                            p.links.insert(key, Link::Dead);
                        }
                    }
                }
            }
        }
        let outcome = if retry { Outcome::Retry } else { Outcome::Done };
        self.finish(host, outcome, now);
    }

    /// Pairs this side's binding on `local_port` with the peer's, opening
    /// the link if both certificates are trusted. Replaces a link the key
    /// had.
    fn pair(&mut self, host: ID, b: PeerBinding, local_port: u16, ours: &Certificate, now: Time) {
        let net = self.networks.get(&b.key).and_then(|n| n.net);
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let link = p.link;
        let old = p.links.get(&b.key).copied();
        let local_trusted = membership_trusted(ours, now);
        let peer_trusted = membership_trusted(&b.certificate, now);
        let open = local_trusted && peer_trusted && net.is_some();
        p.links.insert(
            b.key.clone(),
            Link::Paired {
                local_port,
                peer_port: b.port,
                local_trusted,
                peer_trusted,
                open,
            },
        );
        self.metrics.links_paired += 1;
        if let Some(old) = old {
            if let (true, Some(net)) = (old.is_open(), net) {
                self.op(Op::Unbind { host, link, net });
            }
            if let Some(port) = old.local_port().filter(|p| *p != local_port) {
                self.op(Op::ReleasePort { host, link, port });
            }
        }
        if let (true, Some(net)) = (open, net) {
            self.metrics.links_opened += 1;
            self.op(Op::Bind {
                host,
                link,
                net,
                local_port,
                peer_port: b.port,
            });
        }
        if old.is_some_and(|l| l.is_open()) != open {
            self.count_changed(&b.key);
        }
        if !local_trusted && peer_trusted && net.is_some() {
            self.actions.push_back(Action::RenewVia {
                host,
                link,
                key: b.key,
            });
        }
    }

    /// The answer to this side's `Open`.
    pub(crate) fn open_answered(&mut self, host: ID, link: u64, run: u64, ok: bool, now: Time) {
        let Some(p) = self.peers.get_mut(&host).filter(|p| p.link == link) else {
            return;
        };
        if p.run.id() != Some(run) {
            return;
        }
        match p.run {
            Run::Answered { .. } if ok => self.finish(host, Outcome::Done, now),
            Run::Answered { ref keys, .. } => {
                let keys = keys.clone();
                for key in keys {
                    self.kill_link(host, &key);
                }
                self.finish(host, Outcome::Failed, now);
            }
            Run::Offered { .. } if !ok => {
                self.abandon_offer(host);
                self.finish(host, Outcome::Failed, now);
            }
            _ => {}
        }
    }

    /// Marks a link the peer may still hold as dead, unbinding it.
    fn kill_link(&mut self, host: ID, key: &[u8]) {
        let net = self.networks.get(key).and_then(|n| n.net);
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let link = p.link;
        let Some(l) = p.links.insert(key.to_vec(), Link::Dead) else {
            return;
        };
        if let (true, Some(net)) = (l.is_open(), net) {
            self.op(Op::Unbind { host, link, net });
            self.count_changed(key);
        }
        if let Some(port) = l.local_port() {
            self.op(Op::ReleasePort { host, link, port });
        }
    }

    /// Receiver: gives up an offer the peer may have taken.
    fn abandon_offer(&mut self, host: ID) {
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        let Run::Offered { ref offered, .. } = p.run else {
            return;
        };
        let keys: Vec<NetKey> = offered.iter().map(|(k, _)| k.clone()).collect();
        for key in keys {
            self.kill_link(host, &key);
        }
    }

    /// Ends the peer's run, with what comes next.
    fn finish(&mut self, host: ID, outcome: Outcome, now: Time) {
        let Config {
            retry_delay,
            min_backoff,
            max_backoff,
            max_attempts,
            debounce,
            ..
        } = self.config;
        let jitter = self.rng.next_u64();
        let Some(p) = self.peers.get_mut(&host) else {
            return;
        };
        if matches!(p.run, Run::Running { .. }) {
            self.sessions = self.sessions.saturating_sub(1);
        }
        self.queue.retain(|h| *h != host);
        p.broker_in.clear();
        p.run = Run::Idle;
        match outcome {
            Outcome::Done => {
                self.metrics.runs_completed += 1;
                p.attempts = 0;
                if p.rerun {
                    p.rerun = false;
                    p.run = Run::Pending { at: now + debounce };
                }
            }
            Outcome::Retry | Outcome::Failed => {
                if outcome == Outcome::Retry {
                    self.metrics.cuckoo_retries += 1;
                } else {
                    self.metrics.runs_failed += 1;
                }
                p.attempts += 1;
                if p.attempts >= max_attempts {
                    p.attempts = 0;
                    p.rerun = false;
                } else {
                    let wait = if outcome == Outcome::Retry {
                        retry_delay
                    } else {
                        backoff(min_backoff, max_backoff, p.attempts, jitter)
                    };
                    p.run = Run::Pending { at: now + wait };
                }
            }
        }
        self.start_queued(now);
    }

    /// Ends the peer's run because the peer started over.
    fn abort(&mut self, host: ID) {
        let Some(p) = self.peers.get(&host) else {
            return;
        };
        match p.run {
            Run::Running { .. } => {
                self.sessions = self.sessions.saturating_sub(1);
            }
            Run::Offered { .. } => self.abandon_offer(host),
            _ => {}
        }
        self.queue.retain(|h| *h != host);
        if let Some(p) = self.peers.get_mut(&host) {
            p.run = Run::Idle;
            p.broker_in.clear();
            p.broker_out.clear();
        }
    }

    // --- links --------------------------------------------------------------

    /// Takes the peer's `Close`: its link to the network ends.
    pub(crate) fn handle_close(&mut self, host: ID, link: u64, key: &[u8]) -> Result<(), Refusal> {
        let net = self.networks.get(key).and_then(|n| n.net);
        let p = self
            .peers
            .get_mut(&host)
            .filter(|p| p.link == link)
            .ok_or(Refusal::NotLinked)?;
        let Some(l) = p.links.remove(key) else {
            return Ok(());
        };
        if let (true, Some(net)) = (l.is_open(), net) {
            self.op(Op::Unbind { host, link, net });
            self.count_changed(key);
        }
        if let Some(port) = l.local_port() {
            self.op(Op::ReleasePort { host, link, port });
        }
        Ok(())
    }

    /// The answer to this side's `Close`. A failed one leaves the key out
    /// of runs with the peer: it may still hold its link.
    pub(crate) fn close_answered(&mut self, host: ID, link: u64, key: &[u8], ok: bool) {
        let Some(p) = self.peers.get_mut(&host).filter(|p| p.link == link) else {
            return;
        };
        if ok && p.links.get(key) == Some(&Link::Closing) {
            p.links.remove(key);
        }
    }

    /// Takes the peer's renewed certificate on a network it has a link
    /// on.
    pub(crate) fn handle_update_certificate(
        &mut self,
        host: ID,
        link: u64,
        certificate: Option<Certificate>,
        now: Time,
    ) -> Result<(), Refusal> {
        let certificate = certificate.ok_or(Refusal::MissingCertificate)?;
        let depth = self.config.max_chain_depth;
        let p = self
            .peers
            .get_mut(&host)
            .filter(|p| p.link == link)
            .ok_or(Refusal::NotLinked)?;
        if certificate_chain(&certificate).len() > depth
            || verify_certificate(&certificate, now).is_err()
        {
            return Err(Refusal::InvalidCertificate);
        }
        if certificate.key != p.profile_key {
            return Err(Refusal::OwnerMismatch);
        }
        if !membership_trusted(&certificate, now) {
            return Err(Refusal::Provisional);
        }
        let key = certificate_network_key(&certificate).to_vec();
        let net = self.networks.get(&key).and_then(|n| n.net);
        let Some(Link::Paired {
            local_port,
            peer_port,
            local_trusted,
            peer_trusted,
            open,
        }) = p.links.get_mut(&key)
        else {
            return Err(Refusal::BindingNotFound);
        };
        *peer_trusted = true;
        if *local_trusted && !*open && net.is_some() {
            *open = true;
            let opened = vec![(host, p.link, *local_port, *peer_port)];
            self.open_links(&key, net, opened);
        }
        Ok(())
    }

    /// Binds newly opened links.
    fn open_links(&mut self, key: &[u8], net: Option<N>, opened: Vec<(ID, u64, u16, u16)>) {
        let Some(net) = net else {
            return;
        };
        if opened.is_empty() {
            return;
        }
        for (host, link, local_port, peer_port) in opened {
            self.metrics.links_opened += 1;
            self.op(Op::Bind {
                host,
                link,
                net,
                local_port,
                peer_port,
            });
        }
        self.count_changed(key);
    }

    fn count_changed(&mut self, key: &[u8]) {
        let count = self.peer_count(key);
        self.actions.push_back(Action::PeerCount {
            key: key.to_vec(),
            count,
        });
    }

    // --- time ---------------------------------------------------------------

    /// When [`State::handle_timeout`] has something to do.
    pub(crate) fn poll_timeout(&self) -> Option<Time> {
        let runs = self.peers.values().filter_map(|p| p.run.timeout());
        let renewals = self.networks.values().filter_map(|n| n.renew_at);
        runs.chain(renewals).min()
    }

    pub(crate) fn handle_timeout(&mut self, now: Time) {
        for (key, n) in &mut self.networks {
            if n.renew_at.is_some_and(|t| !now.before(t)) {
                n.renew_at = None;
                self.actions.push_back(Action::Renew { key: key.clone() });
            }
        }
        let due: Vec<ID> = self
            .peers
            .iter()
            .filter(|(_, p)| p.run.timeout().is_some_and(|t| !now.before(t)))
            .map(|(h, _)| *h)
            .collect();
        for host in due {
            let Some(p) = self.peers.get(&host) else {
                continue;
            };
            match p.run {
                Run::Pending { .. } => {
                    if self.keys().is_empty() {
                        if let Some(p) = self.peers.get_mut(&host) {
                            p.run = Run::Idle;
                        }
                    } else {
                        self.start_run(host, now, None, None);
                    }
                }
                Run::Starting {
                    send_at: Some(at),
                    deadline,
                    cap,
                    ..
                } if !now.before(at) && now.before(deadline) && now.before(cap) => {
                    self.send_negotiate(host, now);
                }
                Run::Offered { .. } => {
                    self.abandon_offer(host);
                    self.finish(host, Outcome::Failed, now);
                }
                Run::Reserving { run, .. } | Run::AwaitOpen { run, .. } => {
                    let link = p.link;
                    // the peer may wait for an answer
                    self.actions.push_back(Action::Open {
                        host,
                        link,
                        run,
                        bindings: Vec::new(),
                    });
                    self.finish(host, Outcome::Failed, now);
                }
                Run::Answered { ref keys, .. } => {
                    let keys = keys.clone();
                    for key in keys {
                        self.kill_link(host, &key);
                    }
                    self.finish(host, Outcome::Failed, now);
                }
                _ => self.finish(host, Outcome::Failed, now),
            }
        }
    }
}

#[cfg(test)]
impl<N: Copy + Ord + fmt::Debug> State<N> {
    /// The peer's link to the network: (local port, peer port, open), once
    /// paired.
    pub(crate) fn link_ports(&self, host: ID, key: &[u8]) -> Option<(u16, u16, bool)> {
        match self.peers.get(&host)?.links.get(key)? {
            Link::Paired {
                local_port,
                peer_port,
                open,
                ..
            } => Some((*local_port, *peer_port, *open)),
            _ => None,
        }
    }

    /// Whether the peer has any record for the network.
    pub(crate) fn has_link(&self, host: ID, key: &[u8]) -> bool {
        self.peers
            .get(&host)
            .is_some_and(|p| p.links.contains_key(key))
    }

    /// Whether no run with the peer is under way or due.
    pub(crate) fn is_idle(&self, host: ID) -> bool {
        self.peers
            .get(&host)
            .is_some_and(|p| matches!(p.run, Run::Idle) && p.broker_out.is_empty())
    }
}

/// The wait after `attempts` failed runs: doubling from `min` up to
/// `max`, then between half and all of it.
fn backoff(min: Duration, max: Duration, attempts: u32, jitter: u64) -> Duration {
    let shift = attempts.saturating_sub(1).min(16);
    let full = (min * (1i64 << shift)).min(max);
    let half = full.as_nanos() / 2;
    let span = u64::try_from(half).unwrap_or(0).max(1);
    let extra = i64::try_from(jitter % span).unwrap_or(0);
    Duration::from_nanos(half + extra)
}
