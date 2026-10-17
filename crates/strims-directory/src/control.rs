// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The directory control: a runner per joined network, which follows the
//! network control's events, the calls this node makes to the networks'
//! servers, and what the views hold.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Poll, Waker};

use futures_util::future::{LocalBoxFuture, abortable};
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, StreamExt};
use rand_core::Rng;
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_crypto::membership_trusted;
use strims_dao::schema::network::network_key;
use strims_dao::{Broadcast, Recv, ReplicatedStore, Subscription};
use strims_kv::Kv;
use strims_network::{NetworkControl, NetworkEvent};
use strims_ppspp::SwarmId;
use strims_proto::network::v1::Network;
use strims_proto::network::v1::directory::{
    AssetBundle, DirectoryClient, Listing, ListingModeration, ListingQuery, ListingSnippet,
    UserModeration,
};
use strims_proto::service::{CallError, Error as RpcError};
use strims_rpc::vpn::VpnClient;
use strims_rpc::{RetryClient, Timer};
use strims_transfer::TransferControl;

use crate::events::{DirectoryEvent, ListingEvent, UserEventKind};
use crate::host::{Host, Net};
use crate::server::{Caller, Server};
use crate::services::{self, MemberPort, Reply, Request, ServerState, SnippetHub};
use crate::view::{self, View, ViewEvent};
use crate::{ADDRESS_SALT, Config, DirectoryError, Metrics, SnippetError};

/// A network's key.
pub(crate) type NetKey = Vec<u8>;

/// The wait before a runner first pings its server.
const FIRST_PING: Duration = Duration::from_secs(30);
/// The ping interval range, in seconds, when the asset bundle gives none.
const DEFAULT_MIN_PING_SECS: u32 = 10 * 60;
const DEFAULT_MAX_PING_SECS: u32 = 14 * 60;
/// The shortest ping interval taken from an asset bundle, in seconds.
const MIN_PING_SECS: u32 = 60;
/// The wait before opening a network's member port again after it failed
/// or closed.
pub(crate) const MEMBER_RETRY: Duration = Duration::from_secs(10);

/// Why a call to a network's directory server failed.
///
/// A frontend answers with [`CallFailure::to_rpc`]: a remote error as the
/// server sent it, and anything else with code 0 and this error's message,
/// which for a refusal is the [`DirectoryError`]'s, as the server would
/// have sent it.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum CallFailure {
    /// The node doesn't run the network.
    #[error("network not found")]
    NoNetwork,
    /// The node hasn't joined the network, or the profile's membership
    /// isn't trusted yet, so the node can't call the server.
    #[error("network not joined")]
    NotJoined,
    /// The network's hash table has no directory server address.
    #[error("directory server not found")]
    NoServer,
    /// The server refused the call.
    #[error(transparent)]
    Refused(DirectoryError),
    /// The server answered with another error.
    #[error(transparent)]
    Remote(RpcError),
    /// The call failed on this side: no answer, or a broken connection.
    #[error(transparent)]
    Call(CallError),
    /// A node operation failed.
    #[error("node: {0}")]
    Host(String),
    /// The store of the server this node runs failed.
    #[error(transparent)]
    Store(strims_dao::Error),
}

impl CallFailure {
    /// The RPC error a frontend answers with; see the type docs.
    pub fn to_rpc(&self) -> RpcError {
        match self {
            CallFailure::Remote(e) => e.clone(),
            e => RpcError::new(0, e.to_string()),
        }
    }

    pub(crate) fn from_call(e: CallError) -> CallFailure {
        match e {
            CallError::Remote(e) => match DirectoryError::from_message(&e.message) {
                Some(d) => CallFailure::Refused(d),
                None => CallFailure::Remote(e),
            },
            e => CallFailure::Call(e),
        }
    }
}

/// A network whose directory runs on this node.
#[derive(Clone, Debug, PartialEq)]
pub struct DirectoryNetwork {
    pub network_id: u64,
    pub network_key: Vec<u8>,
    /// Whether this node serves the directory.
    pub serving: bool,
    /// The latest asset bundle.
    pub asset_bundle: Option<AssetBundle>,
}

/// A network's directory on this node, from the node's join until the
/// network stops. A network joined again gets a new runner.
pub(crate) struct Runner<H: Host> {
    pub(crate) key: NetKey,
    pub(crate) network_id: u64,
    pub(crate) net: Net<H>,
    /// The profile's record of the network, as last reported.
    pub(crate) record: RefCell<Network>,
    pub(crate) view: RefCell<View>,
    pub(crate) bundle: RefCell<Option<AssetBundle>>,
    /// The server, while this node serves.
    pub(crate) server: RefCell<Option<ServerState>>,
    pub(crate) member: RefCell<MemberPort>,
    pub(crate) metrics: RefCell<Metrics>,
    pub(crate) next_ping: Cell<Time>,
    /// When to open the member port again.
    pub(crate) member_retry: Cell<Option<Time>>,
    /// The runner stopped; its mode task winds down.
    pub(crate) stopped: Cell<bool>,
    /// The record's server config changed since the mode task last looked.
    pub(crate) changed: Cell<bool>,
    /// Wakes the mode task.
    pub(crate) waker: RefCell<Option<Waker>>,
    /// The network's runner before this one, until its mode task ends: the
    /// two modes would share swarm ids and transfer ids.
    pub(crate) previous: RefCell<Option<Rc<Runner<H>>>>,
    /// The mode task ended, and the task waiting for it.
    pub(crate) done: Cell<bool>,
    pub(crate) done_waker: RefCell<Option<Waker>>,
}

impl<H: Host> Runner<H> {
    pub(crate) fn wake(&self) {
        let w = self.waker.borrow_mut().take();
        if let Some(w) = w {
            w.wake();
        }
    }

    /// Whether the network's record has a server config.
    pub(crate) fn wants_server(&self) -> bool {
        self.record.borrow().server_config.is_some()
    }

    pub(crate) fn serving(&self) -> bool {
        self.server.borrow().is_some()
    }

    fn member_client(&self) -> Option<VpnClient> {
        match &*self.member.borrow() {
            MemberPort::Open { client, .. } => Some(client.clone()),
            _ => None,
        }
    }
}

pub(crate) struct Shared<K: Kv, H: Host, T: Timer> {
    pub(crate) host: H,
    pub(crate) store: ReplicatedStore<K>,
    pub(crate) network: NetworkControl<K, H, T>,
    pub(crate) transfers: TransferControl<H, T>,
    pub(crate) timer: T,
    pub(crate) config: Config,
    /// Draws lease tokens, the servers' rngs and ping intervals.
    pub(crate) rng: RefCell<DetRng>,
    /// Bound: the profile's networks.
    runners: RefCell<BTreeMap<NetKey, Rc<Runner<H>>>>,
    /// Stopped runners whose mode task hasn't ended. Bound: one per key of
    /// `runners`' bound.
    stopping: RefCell<BTreeMap<NetKey, Rc<Runner<H>>>>,
    pub(crate) snippets: Rc<SnippetHub>,
    events: Broadcast<DirectoryEvent>,
    /// Tasks for `run` to start. Bound: a fixed number per runner, the
    /// servers' snippet subscriptions and the pings in flight.
    spawned: RefCell<Vec<LocalBoxFuture<'static, ()>>>,
    /// Wakes `run`.
    waker: RefCell<Option<Waker>>,
}

impl<K: Kv, H: Host, T: Timer> Shared<K, H, T> {
    /// Has `run` start a task.
    pub(crate) fn spawn(&self, task: LocalBoxFuture<'static, ()>) {
        self.spawned.borrow_mut().push(task);
        self.wake_run();
    }

    fn wake_run(&self) {
        let w = self.waker.borrow_mut().take();
        if let Some(w) = w {
            w.wake();
        }
    }

    fn runner(&self, key: &[u8]) -> Option<Rc<Runner<H>>> {
        self.runners.borrow().get(key).cloned()
    }

    fn runner_by_id(&self, network_id: u64) -> Option<Rc<Runner<H>>> {
        self.runners
            .borrow()
            .values()
            .find(|r| r.network_id == network_id)
            .cloned()
    }

    /// Applies `f` to the server `r` runs, queues its actions for the
    /// server task and wakes it. `None` while `r` doesn't serve.
    pub(crate) fn with_server<R>(
        &self,
        r: &Runner<H>,
        f: impl FnOnce(&mut Server, Time) -> R,
    ) -> Option<R> {
        let now = self.timer.now();
        let res = {
            let mut st = r.server.borrow_mut();
            let st = st.as_mut()?;
            let res = f(&mut st.server, now);
            while let Some(a) = st.server.poll_action() {
                st.actions.push_back(a);
            }
            res
        };
        r.wake();
        Some(res)
    }

    /// Reports the events of `r`'s view, unless `r` stopped.
    pub(crate) fn drain_view(&self, r: &Runner<H>) {
        let events: Vec<DirectoryEvent> = {
            let mut v = r.view.borrow_mut();
            std::iter::from_fn(|| v.poll_event())
                .map(|e| directory_event(r.network_id, e))
                .collect()
        };
        if !r.stopped.get() {
            self.events.emit(events);
        }
    }

    /// Starts `r`'s view over.
    pub(crate) fn reset_view(&self, r: &Runner<H>) {
        r.view.borrow_mut().reset();
        self.drain_view(r);
    }

    /// Takes `r`'s asset bundle, and reports it if it changed. A ping due
    /// later than the bundle's longest interval moves into its range.
    pub(crate) fn set_bundle(&self, r: &Runner<H>, bundle: AssetBundle) {
        if r.bundle.borrow().as_ref() == Some(&bundle) {
            return;
        }
        let prev = r.bundle.replace(Some(bundle.clone()));
        drop(prev);
        r.metrics.borrow_mut().asset_bundles += 1;
        let now = self.timer.now();
        if r.next_ping.get() > now + ping_range(r).1 {
            r.next_ping.set(now + self.ping_interval(r));
            self.wake_run();
        }
        if !r.stopped.get() {
            self.events.emit(vec![DirectoryEvent::AssetBundle {
                network_id: r.network_id,
                network_key: r.key.clone(),
                bundle,
            }]);
        }
    }

    /// `r`'s mode task ended.
    pub(crate) fn finished(&self, r: &Rc<Runner<H>>) {
        r.done.set(true);
        let removed = {
            let mut stopping = self.stopping.borrow_mut();
            match stopping.get(&r.key) {
                Some(s) if Rc::ptr_eq(s, r) => stopping.remove(&r.key),
                _ => None,
            }
        };
        drop(removed);
        let w = r.done_waker.borrow_mut().take();
        if let Some(w) = w {
            w.wake();
        }
    }

    // --- networks -----------------------------------------------------------

    fn handle_network_event(self: &Rc<Self>, e: &NetworkEvent) {
        match e {
            NetworkEvent::Started { network }
            | NetworkEvent::Joined { network }
            | NetworkEvent::Updated { network }
            | NetworkEvent::Stopped { network } => self.sync_network(&network_key(network)),
            NetworkEvent::PeerCount { .. } => {}
        }
    }

    /// Runs a runner for every network the node has joined, and only for
    /// those.
    fn sync_all(self: &Rc<Self>) {
        let infos = self.network.networks();
        let joined: BTreeSet<NetKey> = infos
            .iter()
            .filter(|i| i.net.is_some())
            .map(|i| network_key(&i.network))
            .collect();
        let gone: Vec<NetKey> = self
            .runners
            .borrow()
            .keys()
            .filter(|k| !joined.contains(*k))
            .cloned()
            .collect();
        for key in gone {
            self.stop_runner(&key);
        }
        for key in joined {
            self.sync_network(&key);
        }
    }

    /// Follows the network control's state of the network `key`: starts,
    /// restarts, updates or stops its runner.
    fn sync_network(self: &Rc<Self>, key: &[u8]) {
        let joined = self
            .network
            .network(key)
            .and_then(|i| i.net.map(|net| (i.network, net)));
        let current = self.runner(key);
        match (joined, current) {
            (None, Some(_)) => self.stop_runner(key),
            (None, None) => {}
            (Some((n, net)), Some(r)) if r.net == net && r.network_id == n.id => {
                self.update_runner(&r, n);
            }
            (Some((n, net)), Some(_)) => {
                self.stop_runner(key);
                self.start_runner(key, n, net);
            }
            (Some((n, net)), None) => self.start_runner(key, n, net),
        }
    }

    fn start_runner(self: &Rc<Self>, key: &[u8], n: Network, net: Net<H>) {
        let previous = self.stopping.borrow().get(key).cloned();
        let now = self.timer.now();
        let r = Rc::new(Runner {
            key: key.to_vec(),
            network_id: n.id,
            net,
            record: RefCell::new(n),
            view: RefCell::new(View::new(self.config.view)),
            bundle: RefCell::new(None),
            server: RefCell::new(None),
            member: RefCell::new(MemberPort::Closed),
            metrics: RefCell::default(),
            next_ping: Cell::new(now + FIRST_PING),
            member_retry: Cell::new(None),
            stopped: Cell::new(false),
            changed: Cell::new(false),
            waker: RefCell::new(None),
            previous: RefCell::new(previous),
            done: Cell::new(false),
            done_waker: RefCell::new(None),
        });
        self.runners.borrow_mut().insert(key.to_vec(), r.clone());
        self.events.emit(vec![DirectoryEvent::Started {
            network_id: r.network_id,
            network_key: r.key.clone(),
        }]);
        self.spawn(services::run_mode(self.clone(), r.clone()).boxed_local());
        self.ensure_member_port(&r);
    }

    fn stop_runner(&self, key: &[u8]) {
        let Some(r) = self.runners.borrow_mut().remove(key) else {
            return;
        };
        r.stopped.set(true);
        let member = r.member.replace(MemberPort::Closed);
        member.abort();
        if !r.done.get() {
            let prev = self.stopping.borrow_mut().insert(key.to_vec(), r.clone());
            drop(prev);
        }
        r.wake();
        self.events.emit(vec![DirectoryEvent::Stopped {
            network_id: r.network_id,
            network_key: r.key.clone(),
        }]);
    }

    /// Takes the network's new record: a new certificate goes to the
    /// runner's connections and pings the server, and a changed server
    /// config to the mode task.
    fn update_runner(self: &Rc<Self>, r: &Rc<Runner<H>>, n: Network) {
        let prev = r.record.replace(n);
        let record = r.record.borrow().clone();
        if prev == record {
            return;
        }
        if prev.certificate != record.certificate {
            self.set_certificates(r);
            self.ping(r);
        }
        if prev.server_config != record.server_config {
            r.changed.set(true);
            r.wake();
        }
        self.ensure_member_port(r);
    }

    /// Passes the host's current certificate on to `r`'s connections.
    fn set_certificates(&self, r: &Runner<H>) {
        let Some(cert) = self.network.host_certificate(&r.key) else {
            return;
        };
        let server = r.server.borrow().as_ref().map(|s| s.client.clone());
        for c in r.member_client().into_iter().chain(server) {
            if let Err(e) = c.set_certificate(&cert) {
                tracing::debug!(error = %e, "updating a directory certificate failed");
            }
        }
    }

    /// Opens `r`'s member port if it isn't open or opening, and the
    /// profile's membership is trusted.
    pub(crate) fn ensure_member_port(self: &Rc<Self>, r: &Rc<Runner<H>>) {
        if r.stopped.get() || !matches!(*r.member.borrow(), MemberPort::Closed) {
            return;
        }
        let now = self.timer.now();
        let trusted = r
            .record
            .borrow()
            .certificate
            .as_ref()
            .is_some_and(|c| membership_trusted(c, now));
        if !trusted {
            return;
        }
        r.member_retry.set(None);
        let (task, abort) = abortable(services::member_port(self.clone(), r.clone()));
        let prev = r.member.replace(MemberPort::Opening(abort));
        drop(prev);
        self.spawn(task.map(|_| ()).boxed_local());
    }

    // --- pings --------------------------------------------------------------

    /// When the next ping or member port retry is due.
    fn next_timer(&self) -> Option<Time> {
        self.runners
            .borrow()
            .values()
            .flat_map(|r| [Some(r.next_ping.get()), r.member_retry.get()])
            .flatten()
            .min()
    }

    /// Pings the servers and opens the member ports that are due.
    fn tick(self: &Rc<Self>) {
        let now = self.timer.now();
        let runners: Vec<Rc<Runner<H>>> = self.runners.borrow().values().cloned().collect();
        for r in runners {
            if r.next_ping.get() <= now {
                r.next_ping.set(now + self.ping_interval(&r));
                self.ping(&r);
            }
            if r.member_retry.get().is_some_and(|t| t <= now) {
                r.member_retry.set(None);
                self.ensure_member_port(&r);
            }
        }
    }

    /// The wait before `r`'s next ping: the minimum of [`ping_range`] plus
    /// a random share of the span to its maximum.
    fn ping_interval(&self, r: &Runner<H>) -> Duration {
        let (min, max) = ping_range(r);
        let span_ms = u64::try_from((max - min).as_millis()).unwrap_or(0);
        let jitter = self.rng.borrow_mut().next_u64() % (span_ms + 1);
        min + Duration::from_millis(i64::try_from(jitter).unwrap_or(0))
    }

    fn ping(self: &Rc<Self>, r: &Rc<Runner<H>>) {
        let s = self.clone();
        let r = r.clone();
        self.spawn(
            async move {
                let res = s.call_runner(&r, Request::Ping).await;
                {
                    let mut m = r.metrics.borrow_mut();
                    m.pings += 1;
                    m.ping_errors += u64::from(res.is_err());
                }
                if let Err(e) = res {
                    tracing::debug!(network = r.network_id, error = %e, "directory ping failed");
                }
            }
            .boxed_local(),
        );
    }

    // --- calls --------------------------------------------------------------

    async fn call(self: &Rc<Self>, key: &[u8], req: Request) -> Result<Reply, CallFailure> {
        let Some(r) = self.runner(key) else {
            return Err(match self.network.network(key) {
                Some(_) => CallFailure::NotJoined,
                None => CallFailure::NoNetwork,
            });
        };
        self.call_runner(&r, req).await
    }

    /// Makes a call to `r`'s server: this node's own, as its host
    /// certificate, or the published one, from the member port.
    async fn call_runner(
        self: &Rc<Self>,
        r: &Rc<Runner<H>>,
        req: Request,
    ) -> Result<Reply, CallFailure> {
        if r.serving() {
            let caller = self
                .network
                .host_certificate(&r.key)
                .as_ref()
                .and_then(Caller::from_certificate)
                .ok_or(CallFailure::NotJoined)?;
            return services::serve(self, r, &caller, req).await;
        }
        let client = r.member_client().ok_or(CallFailure::NotJoined)?;
        let addr = self
            .host
            .resolve_address(r.net, r.key.clone(), ADDRESS_SALT.to_vec())
            .await
            .map_err(|e| CallFailure::Host(e.to_string()))?
            .ok_or(CallFailure::NoServer)?;
        let retry = RetryClient::new(client.to(addr), self.timer.clone(), self.config.call_policy);
        services::call_remote(&DirectoryClient::new(retry), req).await
    }
}

/// The range of `r`'s ping intervals: the asset bundle's, defaults for
/// what it doesn't set, at least [`MIN_PING_SECS`].
fn ping_range<H: Host>(r: &Runner<H>) -> (Duration, Duration) {
    let (min, max) = r
        .bundle
        .borrow()
        .as_ref()
        .and_then(|b| b.directory.as_ref())
        .map_or((0, 0), |c| (c.min_ping_interval, c.max_ping_interval));
    let or = |v: u32, default: u32| if v == 0 { default } else { v };
    let min = or(min, DEFAULT_MIN_PING_SECS).max(MIN_PING_SECS);
    let max = or(max, DEFAULT_MAX_PING_SECS).max(min);
    let secs = |s: u32| Duration::from_secs(i64::from(s));
    (secs(min), secs(max))
}

fn directory_event(network_id: u64, e: ViewEvent) -> DirectoryEvent {
    let listing = |event| DirectoryEvent::Listing { network_id, event };
    let user = |listing_id, kind, user| DirectoryEvent::User {
        network_id,
        listing_id,
        kind,
        user,
    };
    match e {
        ViewEvent::ListingChanged(l) => listing(ListingEvent::Changed(l)),
        ViewEvent::ListingUnpublished(l) => listing(ListingEvent::Unpublished(l)),
        ViewEvent::UserCountChanged {
            id,
            user_count,
            recent_user_count,
        } => listing(ListingEvent::UserCountChanged {
            id,
            user_count,
            recent_user_count,
        }),
        ViewEvent::UserJoined { listing, user: u } => user(listing, UserEventKind::Joined, u),
        ViewEvent::UserParted { listing, user: u } => user(listing, UserEventKind::Parted, u),
        ViewEvent::UserRenamed { listing, user: u } => user(listing, UserEventKind::Renamed, u),
        ViewEvent::Lagged => DirectoryEvent::Lagged { network_id },
    }
}

/// Runs the directories of a node's networks; see the [crate docs](crate).
///
/// Clones share the control.
pub struct DirectoryControl<K: Kv, H: Host, T: Timer> {
    pub(crate) shared: Rc<Shared<K, H, T>>,
}

impl<K: Kv, H: Host, T: Timer> Clone for DirectoryControl<K, H, T> {
    fn clone(&self) -> Self {
        DirectoryControl {
            shared: self.shared.clone(),
        }
    }
}

/// What woke `run`.
enum Wake {
    Task,
    Event(Option<Recv<NetworkEvent>>),
    Timer,
    Work,
}

impl<K: Kv, H: Host, T: Timer> DirectoryControl<K, H, T> {
    /// A control for the node behind `host`, running the directories of
    /// the networks `network` joins, with their swarms' transfers run by
    /// `transfers`, and the moderation records of the networks it serves
    /// in `store`. Timed on `timer`.
    pub fn new(
        host: H,
        store: ReplicatedStore<K>,
        network: NetworkControl<K, H, T>,
        transfers: TransferControl<H, T>,
        timer: T,
        rng: DetRng,
        config: Config,
    ) -> Self {
        DirectoryControl {
            shared: Rc::new(Shared {
                host,
                store,
                network,
                transfers,
                timer,
                config,
                rng: RefCell::new(rng),
                runners: RefCell::new(BTreeMap::new()),
                stopping: RefCell::new(BTreeMap::new()),
                snippets: Rc::new(SnippetHub::default()),
                events: Broadcast::new(),
                spawned: RefCell::new(Vec::new()),
                waker: RefCell::new(None),
            }),
        }
    }

    /// The network control whose networks this control follows.
    pub fn network(&self) -> &NetworkControl<K, H, T> {
        &self.shared.network
    }

    /// Changes to the directories, from now on.
    pub fn subscribe(&self) -> Subscription<DirectoryEvent> {
        self.shared.events.subscribe()
    }

    // --- views --------------------------------------------------------------

    /// The networks whose directories run, in key order.
    pub fn networks(&self) -> Vec<DirectoryNetwork> {
        self.shared
            .runners
            .borrow()
            .values()
            .map(|r| DirectoryNetwork {
                network_id: r.network_id,
                network_key: r.key.clone(),
                serving: r.serving(),
                asset_bundle: r.bundle.borrow().clone(),
            })
            .collect()
    }

    /// The listings of network `network_id`'s view, in id order.
    pub fn listings(&self, network_id: u64) -> Vec<view::Listing> {
        self.shared
            .runner_by_id(network_id)
            .map(|r| r.view.borrow().listings().cloned().collect())
            .unwrap_or_default()
    }

    /// The listing of network `network_id`'s view that `query` names: by
    /// id, or by listing.
    pub fn listing(&self, network_id: u64, query: &ListingQuery) -> Option<view::Listing> {
        let r = self.shared.runner_by_id(network_id)?;
        let view = r.view.borrow();
        view.listing_by_query(query).cloned()
    }

    /// The users of network `network_id`'s view, in id order.
    pub fn users(&self, network_id: u64) -> Vec<view::User> {
        self.shared
            .runner_by_id(network_id)
            .map(|r| r.view.borrow().users().cloned().collect())
            .unwrap_or_default()
    }

    /// The users in listing `listing_id` of network `network_id`'s view.
    pub fn listing_users(&self, network_id: u64, listing_id: u64) -> Vec<view::User> {
        self.shared
            .runner_by_id(network_id)
            .map(|r| r.view.borrow().listing_users(listing_id))
            .unwrap_or_default()
    }

    /// The listings the user with `peer_key` is in, by network key, for
    /// the networks where there are any.
    pub fn listings_by_peer_key(&self, peer_key: &[u8]) -> Vec<(Vec<u8>, Vec<view::Listing>)> {
        self.shared
            .runners
            .borrow()
            .values()
            .filter_map(|r| {
                let listings = r.view.borrow().listings_of_peer(peer_key);
                (!listings.is_empty()).then(|| (r.key.clone(), listings))
            })
            .collect()
    }

    /// The latest asset bundle of each network that has one: its id, its
    /// key and the bundle.
    pub fn asset_bundles(&self) -> Vec<(u64, Vec<u8>, AssetBundle)> {
        self.shared
            .runners
            .borrow()
            .values()
            .filter_map(|r| {
                let b = r.bundle.borrow().clone()?;
                Some((r.network_id, r.key.clone(), b))
            })
            .collect()
    }

    /// What the directory of the network whose key is `network_key` has
    /// done on this node, while it runs.
    pub fn metrics(&self, network_key: &[u8]) -> Option<Metrics> {
        let r = self.shared.runner(network_key)?;
        let mut m = r.metrics.borrow().clone();
        m.view = r.view.borrow().metrics().clone();
        m.server = r
            .server
            .borrow()
            .as_ref()
            .map(|s| s.server.metrics().clone());
        Some(m)
    }

    // --- snippets -----------------------------------------------------------

    /// Serves `snippet` as the snippet of the swarm `swarm_id` to the
    /// subscribers of every network the swarm's transfer under the empty
    /// salt is published on. Subscribers get the change.
    ///
    /// # Errors
    ///
    /// The snippet fails [`snippet::check`](crate::snippet::check) or
    /// doesn't verify, or this node serves
    /// [`MAX_LOCAL_SNIPPETS`](crate::snippet::MAX_LOCAL_SNIPPETS) others.
    pub fn push_snippet(
        &self,
        swarm_id: &SwarmId,
        snippet: ListingSnippet,
    ) -> Result<(), SnippetError> {
        self.shared.snippets.push(swarm_id, snippet)
    }

    /// Stops serving the swarm's snippet, ending its subscriptions.
    pub fn delete_snippet(&self, swarm_id: &SwarmId) {
        self.shared.snippets.delete(swarm_id);
    }

    // --- calls --------------------------------------------------------------

    /// Publishes `listing` on the directory of the network whose key is
    /// `network_key`, from this host: the listing's id.
    pub async fn publish(&self, network_key: &[u8], listing: Listing) -> Result<u64, CallFailure> {
        let r = self
            .shared
            .call(network_key, Request::Publish(listing))
            .await?;
        Ok(r.id())
    }

    /// Stops publishing listing `id` from this host.
    pub async fn unpublish(&self, network_key: &[u8], id: u64) -> Result<(), CallFailure> {
        self.shared
            .call(network_key, Request::Unpublish(id))
            .await?;
        Ok(())
    }

    /// Views the listing `query` names from this host: its id.
    pub async fn join(&self, network_key: &[u8], query: ListingQuery) -> Result<u64, CallFailure> {
        let r = self.shared.call(network_key, Request::Join(query)).await?;
        Ok(r.id())
    }

    /// Stops viewing listing `id` from this host.
    pub async fn part(&self, network_key: &[u8], id: u64) -> Result<(), CallFailure> {
        self.shared.call(network_key, Request::Part(id)).await?;
        Ok(())
    }

    /// Sets the fields `moderation` sets of listing `id`'s moderation.
    /// The profile must be a moderator of the directory.
    pub async fn moderate_listing(
        &self,
        network_key: &[u8],
        id: u64,
        moderation: Option<ListingModeration>,
    ) -> Result<(), CallFailure> {
        let req = Request::ModerateListing(id, moderation);
        self.shared.call(network_key, req).await?;
        Ok(())
    }

    /// Sets the fields `moderation` sets of the moderation of the user
    /// with `peer_key`. The profile must be a moderator of the directory,
    /// and an admin to change who moderates or administers it.
    pub async fn moderate_user(
        &self,
        network_key: &[u8],
        peer_key: &[u8],
        moderation: Option<UserModeration>,
    ) -> Result<(), CallFailure> {
        let req = Request::ModerateUser(peer_key.to_vec(), moderation);
        self.shared.call(network_key, req).await?;
        Ok(())
    }

    // --- running ------------------------------------------------------------

    /// Runs the directories: follows the network control, serves or reads
    /// each network's directory, pings the servers and serves snippets.
    /// Never resolves; drop it to stop.
    pub async fn run(&self) {
        let s = &self.shared;
        let mut events = Some(s.network.subscribe());
        s.sync_all();
        let mut tasks: FuturesUnordered<LocalBoxFuture<'static, ()>> = FuturesUnordered::new();
        let mut sleep: Option<(Time, Pin<Box<T::Sleep>>)> = None;
        loop {
            let spawned: Vec<_> = s.spawned.borrow_mut().drain(..).collect();
            tasks.extend(spawned);
            let wake = poll_fn(|cx| {
                *s.waker.borrow_mut() = Some(cx.waker().clone());
                if let Poll::Ready(Some(())) = tasks.poll_next_unpin(cx) {
                    return Poll::Ready(Wake::Task);
                }
                if let Some(ev) = &mut events
                    && let Poll::Ready(r) = ev.poll_next_unpin(cx)
                {
                    return Poll::Ready(Wake::Event(r));
                }
                let at = s.next_timer();
                if sleep.as_ref().map(|(d, _)| *d) != at {
                    sleep = at.map(|at| (at, Box::pin(s.timer.sleep_until(at))));
                }
                if let Some((_, sl)) = &mut sleep
                    && sl.as_mut().poll(cx).is_ready()
                {
                    sleep = None;
                    return Poll::Ready(Wake::Timer);
                }
                if !s.spawned.borrow().is_empty() {
                    return Poll::Ready(Wake::Work);
                }
                Poll::Pending
            })
            .await;
            match wake {
                Wake::Event(Some(Recv::Event(e))) => s.handle_network_event(&e),
                Wake::Event(Some(Recv::Lagged)) => s.sync_all(),
                Wake::Event(None) => events = None,
                Wake::Timer => s.tick(),
                Wake::Task | Wake::Work => {}
            }
        }
    }
}
