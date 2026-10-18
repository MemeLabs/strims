// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The network control: the async shell around [`State`]. It runs the
//! profile's networks as the store has them, carries out the state's node
//! operations in order, one at a time, its calls in order, one at a time
//! per peer, and every peer's broker channel.
//!
//! A PSI session's [`PsiReady`](crate::psi::PsiReady) marks its peer in
//! `psi_ready` and wakes the peer's broker task, which passes the session
//! on to [`State::psi_ready`] before it sends the session's bytes.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Poll, Waker};

use futures_util::future::LocalBoxFuture;
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, SinkExt, StreamExt};
use strims_base::rng::{DetRng, fork};
use strims_base::time::Time;
use strims_dao::events::Record;
use strims_dao::schema::network::{Networks, network_key};
use strims_dao::{Broadcast, Recv, ReplicatedStore, StoreEvent, Subscription, Table};
use strims_kademlia::ID;
use strims_kv::Kv;
use strims_proto::network::v1::ca::ca_find_request::Query;
use strims_proto::network::v1::ca::{CaPeerClient, register_ca_peer_service};
use strims_proto::network::v1::{
    Network, NetworkPeerClient, NetworkPeerCloseRequest, NetworkPeerNegotiateRequest,
    NetworkPeerOpenRequest, NetworkPeerUpdateCertificateRequest, register_network_peer_service,
};
use strims_proto::profile::v1::Profile;
use strims_proto::service::Error as RpcError;
use strims_proto::types::Certificate;
use strims_rpc::{Client, RetryClient, Router, Timer};

use crate::events::NetworkEvent;
use crate::host::Host;
use crate::peer::PeerService;
use crate::psi::PsiEngine;
use crate::services::{self, Forwards, Services, renew_time};
use crate::state::{Action, NetKey, Op, OpId, State};
use crate::{Config, Metrics};

type StateOf<H> = State<<H as Host>::Network>;
type OpOf<H> = Op<<H as Host>::Network>;
type ActionOf<H> = Action<<H as Host>::Network>;

/// Broker bytes per write.
const BROKER_CHUNK: usize = 16 * 1024;

/// A linked peer's calls.
struct Link<T> {
    link: u64,
    client: NetworkPeerClient<RetryClient<T>>,
    ca: CaPeerClient<RetryClient<T>>,
}

pub(crate) struct Shared<K: Kv, H: Host, T: Timer> {
    pub(crate) host: H,
    pub(crate) timer: T,
    pub(crate) config: Config,
    pub(crate) store: ReplicatedStore<K>,
    pub(crate) profile: Profile,
    pub(crate) state: RefCell<StateOf<H>>,
    /// Draws the CA's serial numbers and lease tokens.
    pub(crate) rng: RefCell<DetRng>,
    /// Bound: the node's peers.
    links: RefCell<BTreeMap<ID, Link<T>>>,
    /// The profile's networks by key, as the store last had them. Bound:
    /// the profile's networks.
    pub(crate) records: RefCell<BTreeMap<NetKey, Network>>,
    /// Each network's vpn services. Bound: the profile's networks.
    pub(crate) services: RefCell<BTreeMap<NetKey, Services>>,
    /// Networks with a renewal in flight.
    pub(crate) renewing: RefCell<BTreeSet<NetKey>>,
    pub(crate) forwards: RefCell<Forwards>,
    /// When the CA's limit lets each key renew again. Bound:
    /// `MAX_CA_RENEWALS`.
    pub(crate) ca_renewals: RefCell<BTreeMap<Vec<u8>, Time>>,
    /// Tasks for `run` to start. Bound: the networks' services and
    /// renewals.
    spawned: RefCell<Vec<LocalBoxFuture<'static, Done>>>,
    /// Node operations for `run` to apply in order. Bound: the state's
    /// transitions, which the store's changes, the driver's calls and the
    /// peers' bounded requests make.
    ops: RefCell<VecDeque<(OpId, OpOf<H>)>>,
    /// Calls for `run` to make. Same bound.
    calls: RefCell<VecDeque<ActionOf<H>>>,
    /// Peers whose broker channel `run` has yet to take. Bound: the peers.
    brokers: RefCell<VecDeque<(ID, u64)>>,
    /// Broker tasks waiting for the state to change, and those it woke.
    broker_wakers: RefCell<BTreeMap<ID, Waker>>,
    broker_woken: RefCell<BTreeSet<ID>>,
    /// Peers whose PSI session has new bytes or its outcome, for their
    /// broker tasks. Bound: the peers.
    psi_ready: RefCell<BTreeSet<ID>>,
    events: Broadcast<NetworkEvent>,
    /// Wakes `run`.
    waker: RefCell<Option<Waker>>,
}

impl<K: Kv, H: Host, T: Timer> Shared<K, H, T> {
    /// Applies `f` to the state, queues its actions for `run`, and wakes
    /// it and the broker tasks.
    pub(crate) fn update<R>(&self, f: impl FnOnce(&mut StateOf<H>, Time) -> R) -> R {
        let now = self.timer.now();
        let mut events = Vec::new();
        let r = {
            let mut st = self.state.borrow_mut();
            let r = f(&mut st, now);
            let mut ops = self.ops.borrow_mut();
            let mut calls = self.calls.borrow_mut();
            while let Some(a) = st.poll_action() {
                match a {
                    Action::Op(id, op) => ops.push_back((id, op)),
                    Action::PeerCount { key, count } => {
                        if let Some(n) = self.records.borrow().get(&key) {
                            events.push(NetworkEvent::PeerCount {
                                network_id: n.id,
                                count,
                            });
                        }
                    }
                    a => calls.push_back(a),
                }
            }
            r
        };
        self.events.emit(events);
        let wake: Vec<Waker> = {
            let mut wakers = self.broker_wakers.borrow_mut();
            let mut woken = self.broker_woken.borrow_mut();
            woken.extend(wakers.keys().copied());
            std::mem::take(&mut *wakers).into_values().collect()
        };
        let run = self.waker.borrow_mut().take();
        for w in wake.into_iter().chain(run) {
            w.wake();
        }
        r
    }

    /// The peer's PSI session has new bytes or its outcome: wakes its
    /// broker task. It may run inside a state call, so it touches only the
    /// broker tasks' sets.
    fn notify_psi(&self, host: ID) {
        self.psi_ready.borrow_mut().insert(host);
        let waker = self.broker_wakers.borrow_mut().remove(&host);
        if let Some(w) = waker {
            self.broker_woken.borrow_mut().insert(host);
            w.wake();
        }
    }

    pub(crate) fn metrics_mut(&self) -> std::cell::RefMut<'_, Metrics> {
        std::cell::RefMut::map(self.state.borrow_mut(), |st| st.metrics_mut())
    }

    fn client(&self, host: ID, link: u64) -> Option<NetworkPeerClient<RetryClient<T>>> {
        let links = self.links.borrow();
        let l = links.get(&host).filter(|l| l.link == link)?;
        Some(l.client.clone())
    }

    pub(crate) fn ca_peer_client(
        &self,
        host: ID,
        link: u64,
    ) -> Option<CaPeerClient<RetryClient<T>>> {
        let links = self.links.borrow();
        let l = links.get(&host).filter(|l| l.link == link)?;
        Some(l.ca.clone())
    }

    /// Has `run` start a task.
    pub(crate) fn spawn(&self, task: LocalBoxFuture<'static, Done>) {
        self.spawned.borrow_mut().push(task);
        if let Some(w) = self.waker.borrow_mut().take() {
            w.wake();
        }
    }

    /// Takes the store's record of a network: starts running it, or
    /// passes on its changes.
    fn apply_network(self: &Rc<Self>, n: Network) {
        let key = network_key(&n);
        let Some(certificate) = n.certificate.clone() else {
            return;
        };
        if key.is_empty() {
            return;
        }
        let prev = self.records.borrow_mut().insert(key.clone(), n.clone());
        let renew_at = renew_time(&n, self.timer.now());
        match prev {
            None => {
                self.update(|st, now| {
                    st.add_network(key.clone(), n.id, certificate, now);
                    st.schedule_renewal(&key, renew_at);
                });
                self.events.emit(vec![NetworkEvent::Started { network: n }]);
            }
            Some(p) if p != n => {
                let renewing = self.renewing.borrow().contains(&key);
                self.update(|st, now| {
                    if p.certificate != n.certificate {
                        st.set_certificate(&key, certificate, now);
                    }
                    if !renewing {
                        st.schedule_renewal(&key, renew_at);
                    }
                });
                if p.certificate != n.certificate {
                    self.update_services(&key);
                }
                if p.server_config.is_some() != n.server_config.is_some() {
                    self.start_services(&key);
                }
                self.events.emit(vec![NetworkEvent::Updated { network: n }]);
            }
            Some(_) => {}
        }
    }

    /// Stops running a network the store deleted.
    fn remove_network(&self, key: &[u8]) {
        let Some(n) = self.records.borrow_mut().remove(key) else {
            return;
        };
        self.stop_services(key);
        self.update(|st, _| st.remove_network(key));
        self.events.emit(vec![NetworkEvent::Stopped { network: n }]);
    }

    /// Reads every network from the store, and runs what it holds.
    async fn reload(self: &Rc<Self>) {
        let networks = async {
            let mut tx = self.store.read().await?;
            Networks::get_all(&mut tx).await
        }
        .await;
        let networks = match networks {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!(error = %e, "loading networks failed");
                return;
            }
        };
        let keys: BTreeSet<NetKey> = networks.iter().map(network_key).collect();
        let gone: Vec<NetKey> = self
            .records
            .borrow()
            .keys()
            .filter(|k| !keys.contains(*k))
            .cloned()
            .collect();
        for key in gone {
            self.remove_network(&key);
        }
        for n in networks {
            self.apply_network(n);
        }
    }

    fn apply_store_event(self: &Rc<Self>, e: &StoreEvent) {
        match e {
            StoreEvent::Changed(Record::Network(n)) => self.apply_network(n.clone()),
            StoreEvent::Deleted(Record::Network(n)) => self.remove_network(&network_key(n)),
            _ => {}
        }
    }
}

/// What a network the node runs looks like now.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkInfo<N> {
    /// The profile's record of it.
    pub network: Network,
    /// The node's handle for it, once joined.
    pub net: Option<N>,
    /// The peers bound to it.
    pub peer_count: usize,
}

/// Runs the network membership of a node; see the [crate docs](crate).
///
/// Clones share the control.
pub struct NetworkControl<K: Kv, H: Host, T: Timer> {
    pub(crate) shared: Rc<Shared<K, H, T>>,
}

impl<K: Kv, H: Host, T: Timer> Clone for NetworkControl<K, H, T> {
    fn clone(&self) -> Self {
        NetworkControl {
            shared: self.shared.clone(),
        }
    }
}

/// What woke `run`.
enum Wake {
    Op,
    Task(Done),
    Store(Option<Recv>),
    Timer,
    Work,
}

/// A finished task.
pub(crate) enum Done {
    Negotiated {
        host: ID,
        link: u64,
        run: u64,
        ok: bool,
    },
    Opened {
        host: ID,
        link: u64,
        run: u64,
        ok: bool,
    },
    Closed {
        host: ID,
        link: u64,
        key: NetKey,
        ok: bool,
    },
    Updated {
        host: ID,
    },
    /// A broker channel ended.
    Broker,
    /// A service task ended.
    Service,
}

impl Done {
    /// The peer a call went to.
    fn call_host(&self) -> Option<ID> {
        match *self {
            Done::Negotiated { host, .. }
            | Done::Opened { host, .. }
            | Done::Closed { host, .. }
            | Done::Updated { host } => Some(host),
            Done::Broker | Done::Service => None,
        }
    }
}

/// The tasks in flight, and the calls waiting for the one before them to
/// the same peer.
struct Tasks<H: Host> {
    tasks: FuturesUnordered<LocalBoxFuture<'static, Done>>,
    busy: BTreeSet<ID>,
    /// Bound: the state's transitions.
    queued: BTreeMap<ID, VecDeque<ActionOf<H>>>,
}

impl<K: Kv, H: Host, T: Timer> NetworkControl<K, H, T> {
    /// A control for the node behind `host`, running the networks of
    /// `profile` that `store` holds, timed on `timer`.
    pub fn new(
        host: H,
        store: ReplicatedStore<K>,
        profile: Profile,
        timer: T,
        rng: DetRng,
        config: Config,
    ) -> Self {
        let mut rng = rng;
        let state = State::new(host.host_id(), &config, fork(&mut rng));
        let shared = Rc::new_cyclic(|weak: &std::rc::Weak<Shared<K, H, T>>| {
            let weak = weak.clone();
            let mut state = state;
            state.set_psi_wake(Rc::new(move |host| {
                if let Some(s) = weak.upgrade() {
                    s.notify_psi(host);
                }
            }));
            Shared {
                host,
                timer,
                config,
                store,
                profile,
                state: RefCell::new(state),
                rng: RefCell::new(rng),
                links: RefCell::new(BTreeMap::new()),
                records: RefCell::new(BTreeMap::new()),
                services: RefCell::new(BTreeMap::new()),
                renewing: RefCell::new(BTreeSet::new()),
                forwards: RefCell::new(Forwards::default()),
                ca_renewals: RefCell::new(BTreeMap::new()),
                spawned: RefCell::new(Vec::new()),
                ops: RefCell::new(VecDeque::new()),
                calls: RefCell::new(VecDeque::new()),
                brokers: RefCell::new(VecDeque::new()),
                broker_wakers: RefCell::new(BTreeMap::new()),
                broker_woken: RefCell::new(BTreeSet::new()),
                psi_ready: RefCell::new(BTreeSet::new()),
                events: Broadcast::new(),
                waker: RefCell::new(None),
            }
        });
        NetworkControl { shared }
    }

    /// Runs PSI sessions on `engine`, from the next session on. The
    /// default is [`InlinePsi`](crate::psi::InlinePsi); see the [crate
    /// docs](crate#psi).
    pub fn with_psi(self, engine: Rc<dyn PsiEngine>) -> Self {
        let prev = self.shared.state.borrow_mut().set_psi(engine);
        drop(prev);
        self
    }

    // --- peers --------------------------------------------------------------

    /// Takes a newly linked peer, replacing an earlier link to it: serves
    /// it `NetworkPeer` and `CAPeer` on `router`, calls it through
    /// `client`, and runs its broker channel. `profile_key` is the key of
    /// the profile certificate it linked with.
    pub fn add_peer(&self, host: ID, profile_key: &[u8], router: &Router, client: Client) {
        let s = &self.shared;
        let link = s.update(|st, now| st.add_peer(host, profile_key.to_vec(), now));
        let ca = RetryClient::new(client.clone(), s.timer.clone(), services::forward_policy());
        let client = RetryClient::new(client, s.timer.clone(), s.config.call_policy);
        s.links.borrow_mut().insert(
            host,
            Link {
                link,
                client: NetworkPeerClient::new(client),
                ca: CaPeerClient::new(ca),
            },
        );
        let mut router = router.clone();
        let service = Rc::new(PeerService::new(Rc::downgrade(s), host, link));
        register_network_peer_service(&mut router, service.clone());
        register_ca_peer_service(&mut router, service);
        s.brokers.borrow_mut().push_back((host, link));
        s.update(|_, _| ());
    }

    /// Forgets a peer that went away, with its links.
    pub fn remove_peer(&self, host: ID) {
        self.shared.update(|st, now| st.remove_peer(host, now));
        self.shared.links.borrow_mut().remove(&host);
        self.shared.psi_ready.borrow_mut().remove(&host);
        self.shared.forwards.borrow_mut().forget(host);
    }

    // --- networks -----------------------------------------------------------

    /// Changes to the networks the node runs, from now on.
    pub fn subscribe(&self) -> Subscription<NetworkEvent> {
        self.shared.events.subscribe()
    }

    /// The networks the node runs, in key order.
    pub fn networks(&self) -> Vec<NetworkInfo<H::Network>> {
        let s = &self.shared;
        let st = s.state.borrow();
        s.records
            .borrow()
            .iter()
            .map(|(key, n)| NetworkInfo {
                network: n.clone(),
                net: st.network(key).map(|(_, net)| net),
                peer_count: st.peer_count(key),
            })
            .collect()
    }

    /// The network whose key is `key`, if the node runs it.
    pub fn network(&self, key: &[u8]) -> Option<NetworkInfo<H::Network>> {
        let s = &self.shared;
        let st = s.state.borrow();
        let n = s.records.borrow().get(key)?.clone();
        Some(NetworkInfo {
            net: st.network(key).map(|(_, net)| net),
            peer_count: st.peer_count(key),
            network: n,
        })
    }

    /// This host's certificate on the network whose key is `key`, under
    /// the profile's current membership certificate: the one the host's
    /// vpn connections on the network present. It changes with the
    /// membership, before the `Updated` event that reports it. `None`
    /// before the network's `Started`, after its `Stopped`, once the node
    /// failed to join it, or for a profile without a key.
    pub fn host_certificate(&self, key: &[u8]) -> Option<Certificate> {
        self.shared.host_certificate(key)
    }

    /// The certificate the CA of the network whose key is `key` issued
    /// that matches `query`: the latest for a subject or a key, or the one
    /// with a serial number. With `full_chain` its parents are filled in
    /// from the CA's logs, and a parent missing from them fails the
    /// search.
    ///
    /// This device answers itself when it holds the network's server
    /// config. Otherwise it calls the CA over vpn, which needs the node to
    /// have joined the network and the profile's membership to be trusted;
    /// the call retries under [`RetryPolicy::vpn`](strims_rpc::RetryPolicy::vpn),
    /// and dropping the future cancels it.
    ///
    /// # Errors
    ///
    /// An [`RpcError`] whose code is a `strims.network.v1.errors.ErrorCode`.
    /// The CA's refusals carry its code, and a search that matches nothing
    /// is `Unknown`, "not found". Everything else is `Unknown` with a
    /// message saying why: a network the node doesn't run or hasn't
    /// joined, a membership not trusted yet, a CA that isn't published, or
    /// a failed call.
    pub async fn find_certificate(
        &self,
        key: &[u8],
        query: Query,
        full_chain: bool,
    ) -> Result<Certificate, RpcError> {
        self.shared.find_at_ca(key, query, full_chain).await
    }

    /// The control's counters.
    pub fn metrics(&self) -> Metrics {
        self.shared.state.borrow().metrics().clone()
    }

    // --- running ------------------------------------------------------------

    /// Runs the profile's networks: follows the store, carries out node
    /// operations and calls, and runs the peers' broker channels. Peers'
    /// requests are answered only while it runs. Never resolves; drop it
    /// to stop.
    pub async fn run(&self) {
        let s = &self.shared;
        let mut store_events = Some(s.store.events().subscribe());
        s.reload().await;
        let mut tasks = Tasks {
            tasks: FuturesUnordered::new(),
            busy: BTreeSet::new(),
            queued: BTreeMap::new(),
        };
        let mut op: Option<LocalBoxFuture<'static, ()>> = None;
        let mut sleep: Option<(Time, Pin<Box<T::Sleep>>)> = None;
        loop {
            let spawned: Vec<_> = s.spawned.borrow_mut().drain(..).collect();
            tasks.tasks.extend(spawned);
            let brokers: Vec<(ID, u64)> = s.brokers.borrow_mut().drain(..).collect();
            for (host, link) in brokers {
                tasks
                    .tasks
                    .push(broker(s.clone(), host, link).boxed_local());
            }
            let new: Vec<ActionOf<H>> = s.calls.borrow_mut().drain(..).collect();
            for a in new {
                self.dispatch(&mut tasks, a);
            }
            if op.is_none() {
                let next = s.ops.borrow_mut().pop_front();
                if let Some((_, o)) = next {
                    op = Some(apply(s.clone(), o).boxed_local());
                }
            }
            let timeout = s.state.borrow().poll_timeout();
            match timeout {
                Some(at) if sleep.as_ref().is_none_or(|(d, _)| *d != at) => {
                    sleep = Some((at, Box::pin(s.timer.sleep_until(at))));
                }
                Some(_) => {}
                None => sleep = None,
            }
            let wake = poll_fn(|cx| {
                *s.waker.borrow_mut() = Some(cx.waker().clone());
                if let Some(f) = &mut op
                    && f.poll_unpin(cx).is_ready()
                {
                    return Poll::Ready(Wake::Op);
                }
                if let Poll::Ready(Some(done)) = tasks.tasks.poll_next_unpin(cx) {
                    return Poll::Ready(Wake::Task(done));
                }
                if let Some(events) = &mut store_events
                    && let Poll::Ready(r) = events.poll_next_unpin(cx)
                {
                    return Poll::Ready(Wake::Store(r));
                }
                if let Some((_, sl)) = &mut sleep
                    && sl.as_mut().poll(cx).is_ready()
                {
                    sleep = None;
                    return Poll::Ready(Wake::Timer);
                }
                let work = !s.calls.borrow().is_empty()
                    || !s.brokers.borrow().is_empty()
                    || !s.spawned.borrow().is_empty()
                    || (op.is_none() && !s.ops.borrow().is_empty());
                if work {
                    return Poll::Ready(Wake::Work);
                }
                Poll::Pending
            })
            .await;
            match wake {
                Wake::Op => op = None,
                Wake::Task(done) => {
                    if let Some(host) = done.call_host() {
                        tasks.busy.remove(&host);
                        self.start_queued(&mut tasks, host);
                    }
                    self.finish(done);
                }
                Wake::Store(Some(Recv::Event(e))) => s.apply_store_event(&e),
                Wake::Store(Some(Recv::Lagged)) => s.reload().await,
                Wake::Store(None) => store_events = None,
                Wake::Timer => s.update(|st, now| st.handle_timeout(now)),
                Wake::Work => {}
            }
        }
    }

    /// The peer a call goes to.
    fn call_host(a: &ActionOf<H>) -> Option<ID> {
        match *a {
            Action::Negotiate { host, .. }
            | Action::Open { host, .. }
            | Action::Close { host, .. }
            | Action::UpdateCertificate { host, .. } => Some(host),
            _ => None,
        }
    }

    /// Starts a call, or queues it behind the one in flight to its peer.
    fn dispatch(&self, tasks: &mut Tasks<H>, a: ActionOf<H>) {
        match Self::call_host(&a) {
            Some(host) if tasks.busy.contains(&host) => {
                tasks.queued.entry(host).or_default().push_back(a);
            }
            Some(host) => {
                if let Some(task) = self.start(a) {
                    tasks.busy.insert(host);
                    tasks.tasks.push(task);
                }
            }
            None => self.act(a),
        }
    }

    /// Starts the next queued call to `host` that still has a link.
    fn start_queued(&self, tasks: &mut Tasks<H>, host: ID) {
        while let Some(a) = tasks.queued.get_mut(&host).and_then(VecDeque::pop_front) {
            if let Some(task) = self.start(a) {
                tasks.busy.insert(host);
                tasks.tasks.push(task);
                break;
            }
        }
        if tasks.queued.get(&host).is_some_and(VecDeque::is_empty) {
            tasks.queued.remove(&host);
        }
    }

    /// Carries out an action that isn't a call.
    fn act(&self, a: ActionOf<H>) {
        let s = &self.shared;
        match a {
            Action::Renew { key } => {
                s.spawn(services::renew(s.clone(), key, None).boxed_local());
            }
            Action::RenewVia { host, link, key } => {
                s.spawn(services::renew(s.clone(), key, Some((host, link))).boxed_local());
            }
            _ => {}
        }
    }

    /// The task that makes a call, if its peer is still linked.
    fn start(&self, a: ActionOf<H>) -> Option<LocalBoxFuture<'static, Done>> {
        let s = &self.shared;
        match a {
            Action::Negotiate {
                host,
                link,
                run,
                count,
            } => {
                let client = s.client(host, link)?;
                let call = client.negotiate(&NetworkPeerNegotiateRequest { key_count: count });
                Some(
                    async move {
                        let ok = call.await.is_ok();
                        Done::Negotiated {
                            host,
                            link,
                            run,
                            ok,
                        }
                    }
                    .boxed_local(),
                )
            }
            Action::Open {
                host,
                link,
                run,
                bindings,
            } => {
                let client = s.client(host, link)?;
                let call = client.open(&NetworkPeerOpenRequest { bindings });
                Some(
                    async move {
                        let ok = call.await.is_ok();
                        Done::Opened {
                            host,
                            link,
                            run,
                            ok,
                        }
                    }
                    .boxed_local(),
                )
            }
            Action::Close { host, link, key } => {
                let client = s.client(host, link)?;
                let call = client.close(&NetworkPeerCloseRequest { key: key.clone() });
                Some(
                    async move {
                        let ok = call.await.is_ok();
                        Done::Closed {
                            host,
                            link,
                            key,
                            ok,
                        }
                    }
                    .boxed_local(),
                )
            }
            Action::UpdateCertificate {
                host,
                link,
                certificate,
            } => {
                let client = s.client(host, link)?;
                let call = client.update_certificate(&NetworkPeerUpdateCertificateRequest {
                    certificate: Some(certificate),
                });
                let s = s.clone();
                Some(
                    async move {
                        if let Err(e) = call.await {
                            s.metrics_mut().call_errors += 1;
                            tracing::debug!(peer = %host, error = %e, "certificate update failed");
                        }
                        Done::Updated { host }
                    }
                    .boxed_local(),
                )
            }
            _ => None,
        }
    }

    fn finish(&self, done: Done) {
        let s = &self.shared;
        let failed = matches!(
            done,
            Done::Negotiated { ok: false, .. }
                | Done::Opened { ok: false, .. }
                | Done::Closed { ok: false, .. }
        );
        if failed {
            s.metrics_mut().call_errors += 1;
        }
        match done {
            Done::Negotiated {
                host,
                link,
                run,
                ok,
            } => s.update(|st, now| st.negotiate_answered(host, link, run, ok, now)),
            Done::Opened {
                host,
                link,
                run,
                ok,
            } => s.update(|st, now| st.open_answered(host, link, run, ok, now)),
            Done::Closed {
                host,
                link,
                key,
                ok,
            } => s.update(|st, _| st.close_answered(host, link, &key, ok)),
            Done::Updated { .. } | Done::Broker | Done::Service => {}
        }
    }
}

/// Runs a peer's broker channel: sends what the state has for it, and
/// hands it what arrives, until the channel or the link ends.
async fn broker<K: Kv, H: Host, T: Timer>(s: Rc<Shared<K, H, T>>, host: ID, link: u64) -> Done {
    let (mut reader, mut writer) = match s.host.take_broker(host).await {
        Ok(rw) => rw,
        Err(e) => {
            tracing::debug!(peer = %host, error = %e, "taking the broker channel failed");
            return Done::Broker;
        }
    };
    let mut buf = vec![0u8; BROKER_CHUNK];
    loop {
        if s.psi_ready.borrow_mut().remove(&host) {
            s.update(|st, now| st.psi_ready(host, link, now));
        }
        loop {
            let now = s.timer.now();
            let n = s
                .state
                .borrow_mut()
                .poll_broker_transmit(host, link, &mut buf, now);
            let Some(n) = n else {
                break;
            };
            let chunk = buf.get(..n).unwrap_or_default().to_vec();
            if let Err(e) = writer.send(chunk).await {
                tracing::debug!(peer = %host, error = %e, "broker channel closed");
                return Done::Broker;
            }
        }
        let next = poll_fn(|cx| {
            if let Poll::Ready(r) = reader.poll_next_unpin(cx) {
                return Poll::Ready(Some(r));
            }
            if s.broker_woken.borrow_mut().remove(&host) || s.psi_ready.borrow().contains(&host) {
                return Poll::Ready(None);
            }
            s.broker_wakers
                .borrow_mut()
                .insert(host, cx.waker().clone());
            Poll::Pending
        })
        .await;
        if !s.state.borrow().is_current(host, link) {
            return Done::Broker;
        }
        match next {
            Some(Some(Ok(data))) => {
                s.update(|st, now| st.handle_broker_input(host, link, &data, now));
            }
            Some(Some(Err(e))) => {
                tracing::debug!(peer = %host, error = %e, "broker channel failed");
                return Done::Broker;
            }
            Some(None) => return Done::Broker,
            None => {}
        }
    }
}

/// Applies a node operation.
async fn apply<K: Kv, H: Host, T: Timer>(s: Rc<Shared<K, H, T>>, op: OpOf<H>) {
    let current = |host: ID, link: u64| s.state.borrow().is_current(host, link);
    match op {
        Op::AddNetwork { key } => {
            let net = match s.host.add_network(&key).await {
                Ok(net) => Some(net),
                Err(e) => {
                    s.metrics_mut().host_errors += 1;
                    tracing::warn!(error = %e, "joining a network failed");
                    None
                }
            };
            s.update(|st, now| st.network_added(&key, net, now));
            s.start_services(&key);
            let joined = s.state.borrow().network(&key).is_some();
            let network = s.records.borrow().get(&key).cloned();
            if let (true, Some(network)) = (joined, network) {
                s.events.emit(vec![NetworkEvent::Joined { network }]);
            }
        }
        Op::RemoveNetwork { net } => {
            if let Err(e) = s.host.remove_network(net).await {
                s.metrics_mut().host_errors += 1;
                tracing::warn!(error = %e, "leaving a network failed");
            }
        }
        Op::ReservePorts {
            host,
            link,
            run,
            count,
        } => {
            let mut ports = Vec::with_capacity(count);
            let mut failed = !current(host, link);
            while !failed && ports.len() < count {
                match s.host.reserve_peer_port(host).await {
                    Ok(p) => ports.push(p),
                    Err(e) => {
                        tracing::debug!(peer = %host, error = %e, "reserving a port failed");
                        failed = true;
                    }
                }
            }
            if failed {
                for port in ports.drain(..) {
                    let _ = s.host.release_peer_port(host, port).await;
                }
            }
            let ports = (!failed).then_some(ports);
            s.update(|st, now| st.ports_reserved(host, link, run, ports, now));
        }
        Op::ReleasePort { host, link, port } => {
            if current(host, link) {
                let _ = s.host.release_peer_port(host, port).await;
            }
        }
        Op::Bind {
            host,
            link,
            net,
            local_port,
            peer_port,
        } => {
            if current(host, link)
                && let Err(e) = s.host.bind_peer(host, net, local_port, peer_port).await
            {
                s.metrics_mut().host_errors += 1;
                tracing::debug!(peer = %host, error = %e, "binding a peer failed");
            }
        }
        Op::Unbind { host, link, net } => {
            if current(host, link)
                && let Err(e) = s.host.unbind_peer(host, net).await
            {
                s.metrics_mut().host_errors += 1;
                tracing::debug!(peer = %host, error = %e, "unbinding a peer failed");
            }
        }
    }
}
