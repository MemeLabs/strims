// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The transfer control: the async shell around [`State`]. It carries
//! out the state's actions: node operations in order, one at a time;
//! calls to peers and peer index searches concurrently.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use futures_util::future::{Either, LocalBoxFuture, select};
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, StreamExt};
use strims_base::time::Time;
use strims_kademlia::ID;
use strims_ppspp::SwarmId;
use strims_proto::service::CallError;
use strims_proto::transfer::v1::transfer_peer_announce_response::Body;
use strims_proto::transfer::v1::{
    TransferPeerAnnounceRequest, TransferPeerAnnounceResponse, TransferPeerClient,
    TransferPeerCloseRequest, register_transfer_peer_service,
};
use strims_rpc::{Client, RetryClient, Router, Timer};

use crate::host::Host;
use crate::peer::PeerService;
use crate::state::{Action, Answer, Op, OpId, State};
use crate::{Config, Metrics, TransferId};

type StateOf<H> = State<<H as Host>::Network, <H as Host>::Swarm>;
type OpOf<H> = Op<<H as Host>::Network, <H as Host>::Swarm>;
type ActionOf<H> = Action<<H as Host>::Network, <H as Host>::Swarm>;
type Publications<H> = BTreeMap<(TransferId, <H as Host>::Network), <H as Host>::Publication>;

/// A linked peer's calls.
struct Link<T> {
    link: u64,
    client: TransferPeerClient<RetryClient<T>>,
}

/// An op a request handler waits on, and its result once applied.
#[derive(Default)]
struct Waiting {
    done: Option<bool>,
    waker: Option<Waker>,
}

type Waiters = Rc<RefCell<BTreeMap<OpId, Waiting>>>;

pub(crate) struct Shared<H: Host, T: Timer> {
    host: H,
    timer: T,
    config: Config,
    state: RefCell<StateOf<H>>,
    /// Bound: the node's peers.
    links: RefCell<BTreeMap<ID, Link<T>>>,
    /// Node operations for `run` to apply in order. Bound: the state's
    /// transitions, which the driver's calls and the peers' bounded
    /// in-flight requests make.
    ops: RefCell<VecDeque<(OpId, OpOf<H>)>>,
    /// Calls and searches for `run` to start. Same bound.
    calls: RefCell<VecDeque<ActionOf<H>>>,
    /// One per request handler in flight.
    waiting: Waiters,
    /// Bound: published (transfer, network) pairs.
    publications: RefCell<Publications<H>>,
    /// Wakes `run`.
    waker: RefCell<Option<Waker>>,
    metrics: RefCell<Metrics>,
}

impl<H: Host, T: Timer> Shared<H, T> {
    /// Applies `f` to the state, queues its actions for `run`, and wakes
    /// it.
    pub(crate) fn update<R>(&self, f: impl FnOnce(&mut StateOf<H>, Time) -> R) -> R {
        let now = self.timer.now();
        let r = {
            let mut st = self.state.borrow_mut();
            let r = f(&mut st, now);
            let mut ops = self.ops.borrow_mut();
            let mut calls = self.calls.borrow_mut();
            while let Some(a) = st.poll_action() {
                match a {
                    Action::Op(id, op) => ops.push_back((id, op)),
                    a => calls.push_back(a),
                }
            }
            r
        };
        let waker = self.waker.borrow_mut().take();
        if let Some(w) = waker {
            w.wake();
        }
        r
    }

    /// Resolves to whether the op succeeded once `run` has applied it.
    /// Create it before yielding after the transition that queued the
    /// op.
    pub(crate) fn wait(&self, op: OpId) -> OpWait {
        self.waiting.borrow_mut().insert(op, Waiting::default());
        OpWait {
            waiting: self.waiting.clone(),
            op,
        }
    }

    /// Takes a peer's announce: how to answer, and the op to wait on
    /// first.
    pub(crate) fn announce(
        &self,
        host: ID,
        link: u64,
        id: TransferId,
        peer_channel: u64,
    ) -> (Option<u64>, Option<OpWait>) {
        self.metrics_mut().announces_received += 1;
        match self.update(|st, _| st.handle_announce(host, link, id, peer_channel)) {
            Answer::Refuse => (None, None),
            Answer::Channel(c) => (Some(c), None),
            Answer::After(c, op) => (Some(c), Some(self.wait(op))),
        }
    }

    /// Takes a peer's close: the op to wait on, if it stops a channel.
    pub(crate) fn close(&self, host: ID, link: u64, id: TransferId) -> Option<OpWait> {
        self.metrics_mut().closes_received += 1;
        let op = self.update(|st, _| st.handle_close(host, link, id))?;
        Some(self.wait(op))
    }

    pub(crate) fn metrics_mut(&self) -> std::cell::RefMut<'_, Metrics> {
        self.metrics.borrow_mut()
    }

    fn complete(&self, op: OpId, ok: bool) {
        let waker = self.waiting.borrow_mut().get_mut(&op).and_then(|w| {
            w.done = Some(ok);
            w.waker.take()
        });
        if let Some(w) = waker {
            w.wake();
        }
    }

    fn client(&self, host: ID, link: u64) -> Option<TransferPeerClient<RetryClient<T>>> {
        let links = self.links.borrow();
        let l = links.get(&host).filter(|l| l.link == link)?;
        Some(l.client.clone())
    }
}

/// See [`Shared::wait`].
pub(crate) struct OpWait {
    waiting: Waiters,
    op: OpId,
}

impl Future for OpWait {
    type Output = bool;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<bool> {
        let mut waiting = self.waiting.borrow_mut();
        let Some(w) = waiting.get_mut(&self.op) else {
            return Poll::Ready(false);
        };
        if let Some(ok) = w.done {
            waiting.remove(&self.op);
            return Poll::Ready(ok);
        }
        w.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Drop for OpWait {
    fn drop(&mut self) {
        self.waiting.borrow_mut().remove(&self.op);
    }
}

/// Runs the transfers of a node; see the [crate docs](crate).
///
/// Clones share the control.
pub struct TransferControl<H: Host, T: Timer> {
    shared: Rc<Shared<H, T>>,
}

impl<H: Host, T: Timer> Clone for TransferControl<H, T> {
    fn clone(&self) -> Self {
        TransferControl {
            shared: self.shared.clone(),
        }
    }
}

/// What woke `run`.
enum Wake<H: Host> {
    Op(OpId, bool),
    Task(Done<H>),
    Timer,
    Work,
}

/// A finished call or search.
enum Done<H: Host> {
    Announced {
        host: ID,
        link: u64,
        id: TransferId,
        channel: u64,
        result: Result<TransferPeerAnnounceResponse, CallError>,
    },
    Closed {
        host: ID,
        id: TransferId,
        result: Result<(), CallError>,
    },
    Searched {
        id: TransferId,
        net: H::Network,
        ok: bool,
    },
}

impl<H: Host> Done<H> {
    /// The (peer, transfer) a call was for.
    fn call_key(&self) -> Option<(ID, TransferId)> {
        match *self {
            Done::Announced { host, id, .. } | Done::Closed { host, id, .. } => Some((host, id)),
            Done::Searched { .. } => None,
        }
    }
}

/// The calls and searches in flight, and the calls waiting for the one
/// before them on the same (peer, transfer) (see [`Action`]).
struct Calls<H: Host> {
    tasks: FuturesUnordered<LocalBoxFuture<'static, Done<H>>>,
    busy: BTreeSet<(ID, TransferId)>,
    /// Bound: the state's transitions.
    queued: BTreeMap<(ID, TransferId), VecDeque<ActionOf<H>>>,
}

impl<H: Host, T: Timer> TransferControl<H, T> {
    /// A control for the node behind `host`, timed on `timer`.
    pub fn new(host: H, timer: T, config: Config) -> Self {
        let state = State::new(host.host_id(), &config);
        TransferControl {
            shared: Rc::new(Shared {
                host,
                timer,
                config,
                state: RefCell::new(state),
                links: RefCell::new(BTreeMap::new()),
                ops: RefCell::new(VecDeque::new()),
                calls: RefCell::new(VecDeque::new()),
                waiting: Rc::default(),
                publications: RefCell::new(BTreeMap::new()),
                waker: RefCell::new(None),
                metrics: RefCell::default(),
            }),
        }
    }

    // --- transfers ----------------------------------------------------------

    /// Adds a transfer of `swarm`, whose id is `swarm_id`, under `salt`.
    /// An existing transfer with the same id is kept as it is.
    pub fn add(&self, swarm: H::Swarm, swarm_id: &SwarmId, salt: Vec<u8>) -> TransferId {
        self.shared.update(|st, _| st.add(swarm, swarm_id, salt))
    }

    /// The transfer of `swarm_id` under `salt`, and its swarm.
    pub fn find(&self, swarm_id: &SwarmId, salt: &[u8]) -> Option<(TransferId, H::Swarm)> {
        self.shared.state.borrow().find(swarm_id, salt)
    }

    /// Every transfer, in id order.
    pub fn list(&self) -> Vec<TransferId> {
        self.shared.state.borrow().list()
    }

    /// Removes a transfer: stops and closes it with every peer and
    /// withdraws it from the peer index. Returns whether it existed.
    pub fn remove(&self, id: TransferId) -> bool {
        self.shared.update(|st, _| st.remove(id))
    }

    /// Publishes a transfer on `net`: announces it to the network's
    /// peers, and with [`Config::discovery`] advertises and searches for
    /// it there. Returns whether the transfer exists.
    pub fn publish(&self, id: TransferId, net: H::Network) -> bool {
        self.shared.update(|st, now| st.publish(id, net, now))
    }

    pub fn is_published(&self, id: TransferId, net: H::Network) -> bool {
        self.shared.state.borrow().is_published(id, net)
    }

    /// The channels a transfer runs on with `host` while it runs: ours
    /// (on the peer's frames) and the peer's (on ours).
    pub fn channels(&self, host: ID, id: TransferId) -> Option<(u64, u64)> {
        self.shared.state.borrow().running(host, id)
    }

    // --- peers and networks -------------------------------------------------

    /// Takes a newly linked peer, replacing an earlier link to it: serves
    /// it `TransferPeer` on `router` and calls it through `client`.
    pub fn add_peer(&self, host: ID, router: &Router, client: Client) {
        let s = &self.shared;
        let link = s.update(|st, _| st.add_peer(host));
        let client = RetryClient::new(client, s.timer.clone(), s.config.call_policy);
        s.links.borrow_mut().insert(
            host,
            Link {
                link,
                client: TransferPeerClient::new(client),
            },
        );
        let mut router = router.clone();
        register_transfer_peer_service(
            &mut router,
            Rc::new(PeerService::new(Rc::downgrade(s), host, link)),
        );
    }

    /// Forgets a peer that went away, along with its networks.
    pub fn remove_peer(&self, host: ID) {
        self.shared.update(|st, _| st.remove_peer(host));
        self.shared.links.borrow_mut().remove(&host);
    }

    /// The peer joined `net`. It may be reported before or after it
    /// links.
    pub fn add_network_peer(&self, net: H::Network, host: ID) {
        self.shared.update(|st, _| st.add_network_peer(net, host));
    }

    /// The peer left `net`: transfers it shares through no other network
    /// stop and close.
    pub fn remove_network_peer(&self, net: H::Network, host: ID) {
        self.shared
            .update(|st, _| st.remove_network_peer(net, host));
    }

    /// The node left `net`: its publications there are withdrawn, and
    /// transfers shared with its peers through no other network stop
    /// and close. Its transfers stay, unpublished on `net`.
    pub fn remove_network(&self, net: H::Network) {
        self.shared.update(|st, _| st.remove_network(net));
    }

    /// The peer broke the ppspp protocol and the node stopped its swarm
    /// channels: its transfers close, and none runs with it until it
    /// links again.
    pub fn transfer_failed(&self, host: ID) {
        self.shared.update(|st, _| st.transfer_failed(host));
    }

    /// The control's counters.
    pub fn metrics(&self) -> Metrics {
        self.shared.metrics.borrow().clone()
    }

    // --- running ------------------------------------------------------------

    /// Carries out the transfers' node operations, calls and searches.
    /// Peers' announces are answered only while it runs. Never resolves;
    /// drop it to stop.
    pub async fn run(&self) {
        let s = &self.shared;
        let mut calls = Calls {
            tasks: FuturesUnordered::new(),
            busy: BTreeSet::new(),
            queued: BTreeMap::new(),
        };
        let mut op: Option<LocalBoxFuture<'static, (OpId, bool)>> = None;
        let mut sleep: Option<(Time, Pin<Box<T::Sleep>>)> = None;
        loop {
            let new: Vec<ActionOf<H>> = s.calls.borrow_mut().drain(..).collect();
            for a in new {
                self.dispatch(&mut calls, a);
            }
            if op.is_none() {
                let next = s.ops.borrow_mut().pop_front();
                if let Some((id, o)) = next {
                    op = Some(apply(s.clone(), id, o).boxed_local());
                }
            }
            match s.state.borrow().poll_timeout() {
                Some(at) if sleep.as_ref().is_none_or(|(d, _)| *d != at) => {
                    sleep = Some((at, Box::pin(s.timer.sleep_until(at))));
                }
                Some(_) => {}
                None => sleep = None,
            }
            let wake = poll_fn(|cx| {
                *s.waker.borrow_mut() = Some(cx.waker().clone());
                if let Some(f) = &mut op
                    && let Poll::Ready((id, ok)) = f.poll_unpin(cx)
                {
                    return Poll::Ready(Wake::Op(id, ok));
                }
                if let Poll::Ready(Some(done)) = calls.tasks.poll_next_unpin(cx) {
                    return Poll::Ready(Wake::Task(done));
                }
                if let Some((_, sl)) = &mut sleep
                    && sl.as_mut().poll(cx).is_ready()
                {
                    sleep = None;
                    return Poll::Ready(Wake::Timer);
                }
                if !s.calls.borrow().is_empty() || (op.is_none() && !s.ops.borrow().is_empty()) {
                    return Poll::Ready(Wake::Work);
                }
                Poll::Pending
            })
            .await;
            match wake {
                Wake::Op(id, ok) => {
                    op = None;
                    s.complete(id, ok);
                }
                Wake::Task(done) => {
                    if let Some(key) = done.call_key() {
                        calls.busy.remove(&key);
                        self.start_queued(&mut calls, key);
                    }
                    self.finish(done);
                }
                Wake::Timer => s.update(|st, now| st.handle_timeout(now)),
                Wake::Work => {}
            }
        }
    }

    /// Starts a call or search, or queues a call behind the one in
    /// flight for its (peer, transfer).
    fn dispatch(&self, calls: &mut Calls<H>, a: ActionOf<H>) {
        let key = match a {
            Action::Announce { host, id, .. } | Action::Close { host, id, .. } => Some((host, id)),
            Action::Search { .. } | Action::Op(..) => None,
        };
        match key {
            Some(key) if calls.busy.contains(&key) => {
                calls.queued.entry(key).or_default().push_back(a);
            }
            Some(key) => {
                if let Some(task) = self.start(a) {
                    calls.busy.insert(key);
                    calls.tasks.push(task);
                }
            }
            None => calls.tasks.extend(self.start(a)),
        }
    }

    /// Starts the next queued call for `key` that still has a peer to
    /// call.
    fn start_queued(&self, calls: &mut Calls<H>, key: (ID, TransferId)) {
        while let Some(a) = calls.queued.get_mut(&key).and_then(VecDeque::pop_front) {
            if let Some(task) = self.start(a) {
                calls.busy.insert(key);
                calls.tasks.push(task);
                break;
            }
        }
        if calls.queued.get(&key).is_some_and(VecDeque::is_empty) {
            calls.queued.remove(&key);
        }
    }

    /// The task that carries out a call or search, if it still has a
    /// peer to call.
    fn start(&self, a: ActionOf<H>) -> Option<LocalBoxFuture<'static, Done<H>>> {
        let s = &self.shared;
        match a {
            Action::Announce {
                host,
                link,
                id,
                channel,
            } => {
                let client = s.client(host, link)?;
                s.metrics_mut().announces_sent += 1;
                let req = TransferPeerAnnounceRequest {
                    id: id.as_bytes().to_vec(),
                    channel,
                };
                let call = client.announce(&req);
                Some(
                    async move {
                        Done::Announced {
                            host,
                            link,
                            id,
                            channel,
                            result: call.await,
                        }
                    }
                    .boxed_local(),
                )
            }
            Action::Close { host, link, id } => {
                let client = s.client(host, link)?;
                s.metrics_mut().closes_sent += 1;
                let req = TransferPeerCloseRequest {
                    id: id.as_bytes().to_vec(),
                };
                let call = client.close(&req);
                Some(
                    async move {
                        Done::Closed {
                            host,
                            id,
                            result: call.await.map(drop),
                        }
                    }
                    .boxed_local(),
                )
            }
            Action::Search { id, net, key, salt } => {
                s.metrics_mut().searches += 1;
                let s = s.clone();
                Some(
                    async move {
                        let ok = search(&s, id, net, key, salt).await;
                        Done::Searched { id, net, ok }
                    }
                    .boxed_local(),
                )
            }
            Action::Op(..) => None,
        }
    }

    fn finish(&self, done: Done<H>) {
        let s = &self.shared;
        match done {
            Done::Announced {
                host,
                link,
                id,
                channel,
                result,
            } => {
                let answer = match result {
                    Ok(res) => match res.body {
                        // channel 0 means none
                        Some(Body::Channel(c)) if c != 0 => Some(c),
                        _ => None,
                    },
                    Err(e) => {
                        s.metrics_mut().call_errors += 1;
                        tracing::debug!(peer = %host, transfer = %id, error = %e, "announce failed");
                        return;
                    }
                };
                if answer.is_some() {
                    s.metrics_mut().announces_accepted += 1;
                }
                s.update(|st, _| st.handle_announce_answer(host, link, id, channel, answer));
            }
            Done::Closed { host, result, .. } => {
                if let Err(e) = result {
                    s.metrics_mut().call_errors += 1;
                    tracing::debug!(peer = %host, error = %e, "close failed");
                }
            }
            Done::Searched { id, net, ok } => {
                if !ok {
                    s.metrics_mut().search_errors += 1;
                }
                s.update(|st, _| st.search_done(id, net));
            }
        }
    }
}

/// Runs a peer index search for `search_timeout`, reporting at most
/// `max_search_hosts` hosts as they arrive. Resolves to whether it
/// started.
async fn search<H: Host, T: Timer>(
    s: &Shared<H, T>,
    id: TransferId,
    net: H::Network,
    key: Vec<u8>,
    salt: Vec<u8>,
) -> bool {
    let collect = pin!(async {
        let mut found = s.host.index_search(net, key, salt).await?;
        let mut taken = Vec::new();
        while taken.len() < s.config.max_search_hosts {
            let Some(host) = found.next().await else {
                break;
            };
            if !taken.contains(&host) {
                taken.push(host);
                s.update(|st, now| st.search_found(id, net, host, now));
            }
        }
        Ok(())
    });
    let sleep = pin!(s.timer.sleep_until(s.timer.now() + s.config.search_timeout));
    let started: Result<(), H::Error> = match select(collect, sleep).await {
        Either::Left((r, _)) => r,
        Either::Right(_) => Ok(()),
    };
    if let Err(e) = started {
        tracing::debug!(error = %e, "peer index search failed");
        return false;
    }
    true
}

/// Applies a node operation. Resolves to its id and whether it
/// succeeded.
async fn apply<H: Host, T: Timer>(s: Rc<Shared<H, T>>, op_id: OpId, op: OpOf<H>) -> (OpId, bool) {
    let ok = match op {
        Op::Run {
            host,
            link,
            id,
            swarm,
            channel,
            peer_channel,
            notify,
        } => {
            let wanted = s
                .state
                .borrow()
                .wants_run(host, link, id, channel, peer_channel);
            if !wanted {
                return (op_id, false);
            }
            match s.host.run_swarm(host, swarm, channel, peer_channel).await {
                Ok(()) => {
                    s.metrics_mut().runs += 1;
                    true
                }
                Err(e) => {
                    s.metrics_mut().run_errors += 1;
                    tracing::debug!(peer = %host, transfer = %id, error = %e, "running a swarm failed");
                    s.update(|st, _| st.run_failed(host, link, id, channel, peer_channel, notify));
                    false
                }
            }
        }
        Op::Stop { host, link, swarm } => {
            if !s.state.borrow().is_current(host, link) {
                return (op_id, true);
            }
            match s.host.stop_swarm(host, swarm).await {
                Ok(()) => s.metrics_mut().stops += 1,
                Err(e) => {
                    s.metrics_mut().host_errors += 1;
                    tracing::debug!(peer = %host, error = %e, "stopping a swarm failed");
                }
            }
            true
        }
        Op::IndexPublish { id, net, key, salt } => {
            match s.host.index_publish(net, key, salt).await {
                Ok(p) => {
                    s.metrics_mut().index_publishes += 1;
                    s.publications.borrow_mut().insert((id, net), p);
                }
                Err(e) => {
                    s.metrics_mut().host_errors += 1;
                    tracing::debug!(transfer = %id, error = %e, "publishing to the peer index failed");
                }
            }
            true
        }
        Op::IndexUnpublish { id, net } => {
            let p = s.publications.borrow_mut().remove(&(id, net));
            if let Some(p) = p
                && let Err(e) = s.host.index_unpublish(p).await
            {
                s.metrics_mut().host_errors += 1;
                tracing::debug!(transfer = %id, error = %e, "withdrawing from the peer index failed");
            }
            true
        }
        Op::Connect { net, host } => {
            match s.host.connect_peer(net, host).await {
                Ok(_) => s.metrics_mut().connects += 1,
                Err(e) => {
                    s.metrics_mut().host_errors += 1;
                    tracing::debug!(peer = %host, error = %e, "linking to a found host failed");
                }
            }
            true
        }
    };
    (op_id, ok)
}
