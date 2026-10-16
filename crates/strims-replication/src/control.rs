// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The replication control: peers, the replicator lease, discovery and
//! log collection.

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::rc::Rc;
use std::task::Poll;

use futures_util::future::{AbortHandle, Abortable, LocalBoxFuture};
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, StreamExt};
use strims_base::rng::DetRng;
use strims_base::time::Time;
use strims_dao::events::Record;
use strims_dao::replication::checkpoints::CheckpointMap;
use strims_dao::replication::{checkpoints, garbage_collect};
use strims_dao::schema::profile::Devices;
use strims_dao::{Recv, ReplicatedStore, StoreEvent, Subscription, Table, lease, vv};
use strims_kademlia::ID;
use strims_kv::Kv;
use strims_proto::dao::v1::VersionVector;
use strims_proto::profile::v1::Profile;
use strims_proto::replication::v1::register_replication_peer_service;
use strims_rpc::{Client, Router, Timer};

use crate::host::{Host, discovery_salt};
use crate::peer::{PeerError, PeerService, replicate_to};
use crate::queue::{Mailbox, PushQueue};
use crate::{Config, Metrics};

/// A linked peer device.
struct Link {
    client: Client,
    /// The peer's replica, once an exchange has named it.
    replica: Rc<Cell<Option<u64>>>,
}

struct Shared<K: Kv, H: Host, T: Timer> {
    store: ReplicatedStore<K>,
    profile: Profile,
    host: H,
    timer: T,
    config: Rc<Config>,
    rng: RefCell<DetRng>,
    links: RefCell<BTreeMap<ID, Link>>,
    changed: Mailbox<()>,
    metrics: Rc<RefCell<Metrics>>,
    publications: RefCell<BTreeMap<H::Network, H::Publication>>,
}

/// Replicates a profile's store with the profile's other devices.
///
/// The driver reports peers as they link ([`ReplicationControl::add_peer`])
/// and unlink, and runs [`ReplicationControl::run`] as a local task.
/// Every linked device of the profile is served whether or not this one
/// replicates; this one replicates while it holds the store's replication
/// lease, so among processes or tabs sharing a store, one does.
pub struct ReplicationControl<K: Kv, H: Host, T: Timer> {
    shared: Rc<Shared<K, H, T>>,
}

impl<K: Kv, H: Host, T: Timer> Clone for ReplicationControl<K, H, T> {
    fn clone(&self) -> Self {
        ReplicationControl {
            shared: self.shared.clone(),
        }
    }
}

/// Why a replicator stopped.
enum Stop {
    /// Another holder took the lease.
    LeaseLost,
    /// This device was deleted from the profile.
    Deleted,
    /// The store failed.
    Store(strims_dao::Error),
}

impl<K: Kv, H: Host, T: Timer> ReplicationControl<K, H, T> {
    /// A control for `store`, whose profile is `profile` (with its private
    /// key, which signs the device's discovery records).
    pub fn new(
        store: ReplicatedStore<K>,
        profile: Profile,
        host: H,
        timer: T,
        rng: DetRng,
        config: Config,
    ) -> Self {
        ReplicationControl {
            shared: Rc::new(Shared {
                store,
                profile,
                host,
                timer,
                config: Rc::new(config),
                rng: RefCell::new(rng),
                links: RefCell::new(BTreeMap::new()),
                changed: Mailbox::new(),
                metrics: Rc::default(),
                publications: RefCell::new(BTreeMap::new()),
            }),
        }
    }

    /// Takes a newly linked peer: if it holds this profile's key, serves it
    /// the replication methods on `router` and replicates to it through
    /// `client`. Resolves to whether the peer is one of the profile's
    /// devices.
    pub fn add_peer(&self, host: ID, profile_key: &[u8], router: &Router, client: Client) -> bool {
        let s = &self.shared;
        let ours = s
            .profile
            .key
            .as_ref()
            .is_some_and(|k| k.public == profile_key);
        if !ours {
            return false;
        }
        let mut router = router.clone();
        register_replication_peer_service(
            &mut router,
            Rc::new(PeerService {
                store: s.store.clone(),
                timer: s.timer.clone(),
                metrics: s.metrics.clone(),
                opened: Cell::new(None),
            }),
        );
        s.links.borrow_mut().insert(
            host,
            Link {
                client,
                replica: Rc::new(Cell::new(None)),
            },
        );
        s.changed.send(());
        true
    }

    /// Forgets a peer that unlinked.
    pub fn remove_peer(&self, host: ID) {
        let removed = self.shared.links.borrow_mut().remove(&host);
        if removed.is_some() {
            self.shared.changed.send(());
        }
    }

    /// The control's counters.
    pub fn metrics(&self) -> Metrics {
        self.shared.metrics.borrow().clone()
    }

    /// Replicates whenever this device holds the store's replication
    /// lease. Resolves once this device is deleted from the profile; drop
    /// it to stop sooner.
    pub async fn run(&self) {
        let s = &self.shared;
        let key = lease::key(&["replication"]);
        let token = lease::new_token(&mut s.rng.borrow_mut());
        loop {
            match lease::try_lock(s.store.store(), &key, &token, s.timer.now()).await {
                Ok(()) => {
                    s.metrics.borrow_mut().leases += 1;
                    let stop = Replicator::new(s.clone(), token, key.clone()).run().await;
                    let released =
                        lease::release(s.store.store(), &key, &token, s.timer.now()).await;
                    if let Err(e) = released {
                        tracing::debug!(error = %e, "releasing the replication lease failed");
                    }
                    match stop {
                        Stop::Deleted => return,
                        Stop::LeaseLost => {}
                        Stop::Store(e) => {
                            s.metrics.borrow_mut().store_errors += 1;
                            tracing::warn!(error = %e, "replication stopped");
                        }
                    }
                }
                Err(strims_dao::Error::LeaseBusy) => {}
                Err(e) => {
                    s.metrics.borrow_mut().store_errors += 1;
                    tracing::warn!(error = %e, "taking the replication lease failed");
                }
            }
            let delay = lease::recheck_delay(&mut s.rng.borrow_mut());
            s.timer.sleep_until(s.timer.now() + delay).await;
        }
    }
}

type Task = LocalBoxFuture<
    'static,
    (
        ID,
        Result<Result<(), PeerError>, futures_util::future::Aborted>,
    ),
>;

/// Publications made on networks.
type Published<H> = Vec<(<H as Host>::Network, <H as Host>::Publication)>;

/// A running replication to one peer.
struct Outbound {
    abort: AbortHandle,
    queue: PushQueue,
    replica: Rc<Cell<Option<u64>>>,
}

enum Wake<H: Host> {
    Links,
    Event(Option<Recv>),
    Task(ID, Result<(), PeerError>),
    Scanned(BTreeMap<ID, Time>),
    Published(Published<H>),
    Timer,
}

/// Replication while this device holds the lease.
struct Replicator<K: Kv, H: Host, T: Timer> {
    s: Rc<Shared<K, H, T>>,
    token: lease::Token,
    key: String,
    events: Option<Subscription>,
    checkpoints: CheckpointMap,
    gc_threshold: VersionVector,
    gc_at: Option<Time>,
    refresh_at: Time,
    scan_at: Time,
    retry_at: BTreeMap<ID, Time>,
    gone: BTreeSet<ID>,
    outbound: BTreeMap<ID, Outbound>,
    tasks: FuturesUnordered<Task>,
    scan: Option<LocalBoxFuture<'static, BTreeMap<ID, Time>>>,
    publish: Option<LocalBoxFuture<'static, Published<H>>>,
    backoff: BTreeMap<ID, Time>,
    sleep: Option<(Time, Pin<Box<T::Sleep>>)>,
}

impl<K: Kv, H: Host, T: Timer> Replicator<K, H, T> {
    fn new(s: Rc<Shared<K, H, T>>, token: lease::Token, key: String) -> Self {
        let now = s.timer.now();
        Replicator {
            refresh_at: now + lease::REFRESH,
            scan_at: now,
            s,
            token,
            key,
            events: None,
            checkpoints: CheckpointMap::default(),
            gc_threshold: VersionVector::default(),
            gc_at: Some(now),
            retry_at: BTreeMap::new(),
            gone: BTreeSet::new(),
            outbound: BTreeMap::new(),
            tasks: FuturesUnordered::new(),
            scan: None,
            publish: None,
            backoff: BTreeMap::new(),
            sleep: None,
        }
    }

    async fn run(mut self) -> Stop {
        self.events = Some(self.s.store.events().subscribe());
        let stop = match self.load_checkpoints().await {
            Ok(None) => self.serve().await,
            Ok(Some(stop)) => stop,
            Err(e) => Stop::Store(e),
        };
        for (_, o) in std::mem::take(&mut self.outbound) {
            o.abort.abort();
        }
        self.scan = None;
        self.publish = None;
        let publications: Vec<_> = std::mem::take(&mut *self.s.publications.borrow_mut())
            .into_values()
            .collect();
        for p in publications {
            if let Err(e) = self.s.host.unpublish(p).await {
                tracing::debug!(error = %e, "withdrawing a discovery record failed");
            }
        }
        stop
    }

    /// Reloads the checkpoints of the profile's live devices. Resolves to a
    /// stop if this device is deleted.
    async fn load_checkpoints(&mut self) -> Result<Option<Stop>, strims_dao::Error> {
        let all = checkpoints(self.s.store.store()).await?;
        let own = self.s.store.replica_id();
        if all.iter().any(|c| c.id == own && c.deleted) {
            return Ok(Some(Stop::Deleted));
        }
        self.checkpoints = CheckpointMap::new(all);
        for device in device_ids(&self.s.store).await? {
            self.checkpoints.ensure(device);
        }
        Ok(None)
    }

    async fn serve(&mut self) -> Stop {
        loop {
            let now = self.s.timer.now();
            self.reconcile(now);
            let deadline = self.deadline();
            match self.wait(deadline).await {
                Wake::Links => {}
                Wake::Event(None) => {
                    return Stop::Store(strims_dao::Error::Invalid("event stream ended"));
                }
                Wake::Event(Some(e)) => {
                    if let Some(stop) = self.on_event(e, now).await {
                        return stop;
                    }
                }
                Wake::Task(peer, result) => self.on_task_end(peer, result, now).await,
                Wake::Scanned(backoff) => {
                    self.scan = None;
                    self.backoff = backoff;
                }
                Wake::Published(results) => {
                    self.publish = None;
                    self.s.publications.borrow_mut().extend(results);
                }
                Wake::Timer => {
                    if let Some(stop) = self.on_timer(now).await {
                        return stop;
                    }
                }
            }
        }
    }

    fn deadline(&self) -> Time {
        let mut d = self.refresh_at.min(self.scan_at);
        if let Some(gc) = self.gc_at {
            d = d.min(gc);
        }
        if let Some(r) = self.retry_at.values().min() {
            d = d.min(*r);
        }
        d
    }

    async fn wait(&mut self, deadline: Time) -> Wake<H> {
        if self.sleep.as_ref().is_none_or(|(at, _)| *at != deadline) {
            self.sleep = Some((deadline, Box::pin(self.s.timer.sleep_until(deadline))));
        }
        let changed = self.s.changed.clone();
        poll_fn(|cx| {
            if changed.poll_recv(cx).is_ready() {
                return Poll::Ready(Wake::Links);
            }
            if let Some(events) = &mut self.events
                && let Poll::Ready(e) = events.poll_next_unpin(cx)
            {
                return Poll::Ready(Wake::Event(e));
            }
            if let Poll::Ready(Some((peer, result))) = self.tasks.poll_next_unpin(cx) {
                let result = result.unwrap_or(Ok(()));
                return Poll::Ready(Wake::Task(peer, result));
            }
            if let Some(scan) = &mut self.scan
                && let Poll::Ready(backoff) = scan.poll_unpin(cx)
            {
                return Poll::Ready(Wake::Scanned(backoff));
            }
            if let Some(publish) = &mut self.publish
                && let Poll::Ready(results) = publish.poll_unpin(cx)
            {
                return Poll::Ready(Wake::Published(results));
            }
            if let Some((_, sleep)) = &mut self.sleep
                && sleep.as_mut().poll(cx).is_ready()
            {
                self.sleep = None;
                return Poll::Ready(Wake::Timer);
            }
            Poll::Pending
        })
        .await
    }

    /// Starts replicating to linked peers that aren't being replicated to,
    /// and stops for peers that unlinked.
    fn reconcile(&mut self, now: Time) {
        let links = self.s.links.borrow();
        self.outbound.retain(|peer, o| {
            let linked = links.contains_key(peer);
            if !linked {
                o.abort.abort();
            }
            linked
        });
        self.retry_at.retain(|peer, _| links.contains_key(peer));
        self.gone.retain(|peer| links.contains_key(peer));
        for (peer, link) in links.iter() {
            if self.outbound.contains_key(peer) || self.gone.contains(peer) {
                continue;
            }
            if self.retry_at.get(peer).is_some_and(|at| *at > now) {
                continue;
            }
            self.retry_at.remove(peer);
            let queue = PushQueue::new(self.s.config.push_queue);
            let (abort, registration) = AbortHandle::new_pair();
            let task = replicate_to(
                self.s.store.clone(),
                self.s.timer.clone(),
                self.s.config.clone(),
                self.s.metrics.clone(),
                link.client.clone(),
                queue.clone(),
                link.replica.clone(),
            );
            let peer = *peer;
            self.tasks.push(
                Abortable::new(task, registration)
                    .map(move |r| (peer, r))
                    .boxed_local(),
            );
            self.outbound.insert(
                peer,
                Outbound {
                    abort,
                    queue,
                    replica: link.replica.clone(),
                },
            );
        }
    }

    async fn on_event(&mut self, e: Recv, now: Time) -> Option<Stop> {
        let e = match e {
            Recv::Lagged => {
                for o in self.outbound.values() {
                    o.queue.resync();
                }
                match self.load_checkpoints().await {
                    Ok(None) => {}
                    Ok(Some(stop)) => return Some(stop),
                    Err(e) => return Some(Stop::Store(e)),
                }
                self.gc_at = Some(now + self.s.config.gc_debounce);
                return None;
            }
            Recv::Event(e) => e,
        };
        match &*e {
            StoreEvent::EventLog(l) => {
                for o in self.outbound.values() {
                    o.queue.push(l.clone());
                }
            }
            StoreEvent::LogsApplied { from, logs } => {
                for o in self
                    .outbound
                    .values()
                    .filter(|o| o.replica.get() != Some(*from))
                {
                    for l in logs {
                        o.queue.push(l.clone());
                    }
                }
            }
            StoreEvent::Changed(Record::Device(d)) => self.checkpoints.ensure(d.id),
            StoreEvent::Changed(Record::Checkpoint(c)) => {
                self.checkpoints.set(c.clone());
                if c.deleted && c.id == self.s.store.replica_id() {
                    return Some(Stop::Deleted);
                }
                self.gc_at = Some(now + self.s.config.gc_debounce);
            }
            _ => {}
        }
        None
    }

    async fn on_task_end(&mut self, peer: ID, result: Result<(), PeerError>, now: Time) {
        let Some(o) = self.outbound.remove(&peer) else {
            return;
        };
        let Err(e) = result else {
            return;
        };
        self.s.metrics.borrow_mut().peer_errors += 1;
        tracing::debug!(%peer, error = %e, "replication to peer stopped");
        match e {
            PeerError::Deleted(_) | PeerError::Incompatible(_) | PeerError::Invalid => {
                self.gone.insert(peer);
            }
            PeerError::Call(_) | PeerError::Store(_) => {
                self.retry_at.insert(peer, now + self.s.config.peer_retry);
            }
        }
        drop(o);
    }

    async fn on_timer(&mut self, now: Time) -> Option<Stop> {
        if now >= self.refresh_at {
            match lease::try_lock(self.s.store.store(), &self.key, &self.token, now).await {
                Ok(()) => self.refresh_at = now + lease::REFRESH,
                Err(_) => return Some(Stop::LeaseLost),
            }
        }
        if self.gc_at.is_some_and(|at| now >= at) {
            self.gc_at = None;
            let next = self.checkpoints.min_version();
            if vv::compare(&self.gc_threshold, &next) == Some(Ordering::Less) {
                match garbage_collect(self.s.store.store(), &next, now).await {
                    Ok(n) => {
                        let mut m = self.s.metrics.borrow_mut();
                        m.gc_runs += 1;
                        m.logs_collected += n as u64;
                    }
                    Err(e) => return Some(Stop::Store(e)),
                }
                self.gc_threshold = next;
            }
        }
        if now >= self.scan_at {
            self.scan_at = now + self.s.config.scan_interval;
            if self.publish.is_none() {
                self.publish = Some(publish(self.s.clone()).boxed_local());
            }
            if self.scan.is_none() {
                let connected: BTreeSet<ID> = self.s.links.borrow().keys().copied().collect();
                self.scan =
                    Some(scan(self.s.clone(), self.backoff.clone(), connected).boxed_local());
            }
        }
        None
    }
}

/// Publishes this device's host id on networks it hasn't yet, and
/// withdraws it from networks the node left.
async fn publish<K: Kv, H: Host, T: Timer>(s: Rc<Shared<K, H, T>>) -> Published<H> {
    let networks = s.host.networks().await;
    let left: Vec<H::Publication> = {
        let mut pubs = s.publications.borrow_mut();
        let keys: Vec<H::Network> = pubs
            .keys()
            .filter(|n| !networks.contains(n))
            .copied()
            .collect();
        keys.into_iter().filter_map(|n| pubs.remove(&n)).collect()
    };
    for p in left {
        if let Err(e) = s.host.unpublish(p).await {
            tracing::debug!(error = %e, "withdrawing a discovery record failed");
        }
    }
    let Some(key) = s.profile.key.clone() else {
        return Vec::new();
    };
    let salt = discovery_salt(s.store.replica_id());
    let host = s.host.host_id().to_bytes().to_vec();
    let mut out = Vec::new();
    for net in networks {
        if s.publications.borrow().contains_key(&net) {
            continue;
        }
        match s.host.publish(net, &key, salt.clone(), host.clone()).await {
            Ok(p) => {
                s.metrics.borrow_mut().publishes += 1;
                out.push((net, p));
            }
            Err(e) => tracing::debug!(error = %e, "publishing a discovery record failed"),
        }
    }
    out
}

/// Looks up the profile's other live devices on every network and asks
/// the node to link to those it isn't linked to. Resolves to the hosts
/// not to ask about again until their time.
async fn scan<K: Kv, H: Host, T: Timer>(
    s: Rc<Shared<K, H, T>>,
    mut backoff: BTreeMap<ID, Time>,
    connected: BTreeSet<ID>,
) -> BTreeMap<ID, Time> {
    let now = s.timer.now();
    backoff.retain(|_, until| *until > now);
    let Some(profile_key) = s.profile.key.as_ref().map(|k| k.public.clone()) else {
        return backoff;
    };
    let devices = match device_ids(&s.store).await {
        Ok(d) => d,
        Err(e) => {
            tracing::debug!(error = %e, "reading devices failed");
            return backoff;
        }
    };
    let networks = s.host.networks().await;
    let own_host = s.host.host_id();
    for device in devices
        .into_iter()
        .filter(|d| *d != s.store.replica_id())
        .take(s.config.max_scan_devices)
    {
        for &net in &networks {
            s.metrics.borrow_mut().lookups += 1;
            let found = s
                .host
                .lookup(
                    net,
                    profile_key.clone(),
                    discovery_salt(device),
                    s.config.max_lookup_results,
                )
                .await;
            let values = match found {
                Ok(v) => v,
                Err(e) => {
                    tracing::debug!(error = %e, "discovery lookup failed");
                    continue;
                }
            };
            for v in values {
                let Ok(host) = ID::try_from(v.as_slice()) else {
                    continue;
                };
                if host == own_host || connected.contains(&host) || backoff.contains_key(&host) {
                    continue;
                }
                backoff.insert(host, now + s.config.connect_backoff);
                match s.host.connect_peer(net, host).await {
                    Ok(_) => s.metrics.borrow_mut().connects += 1,
                    Err(e) => tracing::debug!(error = %e, "linking to a device failed"),
                }
            }
        }
    }
    backoff
}

/// The ids of the profile's devices that aren't deleted.
async fn device_ids<K: Kv>(store: &ReplicatedStore<K>) -> Result<Vec<u64>, strims_dao::Error> {
    let deleted: BTreeSet<u64> = checkpoints(store.store())
        .await?
        .into_iter()
        .filter(|c| c.deleted)
        .map(|c| c.id)
        .collect();
    let mut tx = store.read().await?;
    Ok(Devices::get_all(&mut tx)
        .await?
        .into_iter()
        .map(|d| d.id)
        .filter(|id| !deleted.contains(id))
        .collect())
}
