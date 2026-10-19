// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The debug control: the device's debug config, the mock streams the
//! node writes, and those it reads.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::future::{Future, poll_fn};
use std::pin::pin;
use std::rc::Rc;
use std::task::{Poll, Waker};

use futures_util::future::{Either, LocalBoxFuture, select};
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, StreamExt};
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_crypto::generate_key;
use strims_dao::events::Record;
use strims_dao::schema::debug::DebugConfig;
use strims_dao::schema::network::{Networks, network_key};
use strims_dao::{Read, Recv, ReplicatedStore, Singleton, StoreEvent, Table};
use strims_directory::{DirectoryControl, DirectoryEvent, Host, ListingEvent, ListingKeeper};
use strims_kv::Kv;
use strims_network::{NetworkControl, NetworkEvent};
use strims_ppspp::SwarmId;
use strims_proto::debug::v1::Config as DebugConfigRecord;
use strims_rpc::Timer;
use strims_transfer::{TransferControl, TransferId};

use crate::producer::{Producer, check_start};
use crate::reader::{Reader, ReaderKey, mock_swarm, plan};
use crate::{Config, DebugError, Metrics};

/// A swarm of the node.
pub(crate) type Swarm<H> = <H as strims_transfer::Host>::Swarm;

/// The longest network key taken, in bytes. A network's key is its root
/// certificate's ed25519 key, so a longer one names no network.
const MAX_NETWORK_KEY_BYTES: usize = 64;

/// A signal that ends one task: the task waits on it, others set it.
#[derive(Default)]
pub(crate) struct Stop {
    stopped: Cell<bool>,
    /// Wakes the task.
    waker: RefCell<Option<Waker>>,
}

impl Stop {
    pub(crate) fn stop(&self) {
        self.stopped.set(true);
        let w = self.waker.take();
        if let Some(w) = w {
            w.wake();
        }
    }

    pub(crate) fn is_stopped(&self) -> bool {
        self.stopped.get()
    }

    /// `f`'s output, or `None` once stopped first.
    pub(crate) async fn until<F: Future>(&self, f: F) -> Option<F::Output> {
        let stopped = pin!(poll_fn(|cx| {
            if self.stopped.get() {
                return Poll::Ready(());
            }
            let prev = self.waker.replace(Some(cx.waker().clone()));
            drop(prev);
            Poll::Pending
        }));
        if self.stopped.get() {
            return None;
        }
        match select(pin!(f), stopped).await {
            Either::Left((out, _)) => Some(out),
            Either::Right(((), _)) => None,
        }
    }
}

/// The answer one task sets and another takes.
pub(crate) struct Reply<T> {
    value: RefCell<Option<Result<T, DebugError>>>,
    waker: RefCell<Option<Waker>>,
}

impl<T> Reply<T> {
    fn new() -> Rc<Reply<T>> {
        Rc::new(Reply {
            value: RefCell::new(None),
            waker: RefCell::new(None),
        })
    }

    /// Sets the answer and wakes the taker.
    pub(crate) fn set(&self, v: Result<T, DebugError>) {
        let prev = self.value.replace(Some(v));
        drop(prev);
        let w = self.waker.take();
        if let Some(w) = w {
            w.wake();
        }
    }

    /// The answer, once set.
    async fn take(&self) -> Result<T, DebugError> {
        poll_fn(|cx| {
            if let Some(v) = self.value.take() {
                return Poll::Ready(v);
            }
            let prev = self.waker.replace(Some(cx.waker().clone()));
            drop(prev);
            Poll::Pending
        })
        .await
    }
}

/// The device's debug config as the control last took it.
#[derive(Default)]
struct Records {
    config: DebugConfigRecord,
    /// The read of the config under way: its id, and whether an event
    /// wrote the config since it began.
    read: Option<(u64, bool)>,
    next_read: u64,
}

pub(crate) struct Shared<K: Kv, H: Host, T: Timer> {
    pub(crate) host: H,
    pub(crate) store: ReplicatedStore<K>,
    pub(crate) network: NetworkControl<K, H, T>,
    pub(crate) transfers: TransferControl<H, T>,
    pub(crate) directory: DirectoryControl<K, H, T>,
    pub(crate) timer: T,
    rng: RefCell<DetRng>,
    pub(crate) config: Config,
    pub(crate) metrics: RefCell<Metrics>,
    records: RefCell<Records>,
    /// The mock streams the node writes, by id. Bound:
    /// [`Config::max_mock_streams`].
    pub(crate) producers: RefCell<BTreeMap<u64, Producer>>,
    /// The mock streams the node reads, by network key and listing id.
    /// Bound: [`Config::max_readers`].
    pub(crate) readers: RefCell<BTreeMap<ReaderKey, Reader>>,
    /// The key of each network whose directory runs, by network id. Bound:
    /// the profile's networks.
    networks: RefCell<BTreeMap<u64, Vec<u8>>>,
    /// The swarms of the mock stream listings in the view of the network
    /// the config reads, by listing id. Bound: the view's listings
    /// ([`ViewLimits::max_listings`](strims_directory::ViewLimits::max_listings)).
    index: RefCell<BTreeMap<u64, SwarmId>>,
    /// The listings of `index` whose reader ended by itself. Bound: those
    /// of `index`, as each reconcile keeps only those.
    ended: RefCell<BTreeSet<u64>>,
    /// The readers are to be planned again.
    reconcile: Cell<bool>,
    /// Tasks for `run` to start. Bound: a read of the config per lag of
    /// the store's events; per mock stream, its task and its listing
    /// keeper; per reader, its task.
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

    /// Counts a failed node call, and the error it answers with.
    pub(crate) fn host_failed(&self, what: &str, e: &impl fmt::Display) -> DebugError {
        self.metrics.borrow_mut().host_errors += 1;
        tracing::warn!(error = %e, "{what} failed");
        DebugError::Host(e.to_string())
    }

    /// Removes `swarm` from the node. Remove its transfer first.
    pub(crate) async fn remove_host_swarm(&self, swarm: Swarm<H>) {
        if let Err(e) = self.host.remove_swarm(swarm).await {
            self.host_failed("removing a mock stream swarm", &e);
        }
    }

    /// Publishes transfer `t` on the network `key` names, if the node has
    /// joined it and it isn't published there.
    pub(crate) fn publish_on_joined(&self, t: TransferId, key: &[u8]) {
        if let Some(net) = self.network.network(key).and_then(|i| i.net)
            && !self.transfers.is_published(t, net)
        {
            self.transfers.publish(t, net);
        }
    }

    /// [`publish_on_joined`](Self::publish_on_joined) for the transfers of
    /// the mock streams on the network `key`, or on any network.
    fn publish_all(&self, key: Option<&[u8]>) {
        let on = |k: &[u8]| key.is_none_or(|key| key == k);
        let mut ts: Vec<(TransferId, Vec<u8>)> = self
            .producers
            .borrow()
            .values()
            .filter(|p| on(&p.network_key))
            .filter_map(|p| Some((p.transfer?, p.network_key.clone())))
            .collect();
        ts.extend(
            self.readers
                .borrow()
                .iter()
                .filter(|((k, _), _)| on(k))
                .filter_map(|((k, _), r)| Some((r.transfer?, k.clone()))),
        );
        for (t, k) in ts {
            self.publish_on_joined(t, &k);
        }
    }

    /// Has `run` plan the readers again, once the events at hand are taken.
    pub(crate) fn schedule_reconcile(&self) {
        self.reconcile.set(true);
        self.wake_run();
    }

    /// Keeps listing `key` from being read again until it changes, if it
    /// is on the network the config reads.
    pub(crate) fn reader_ended(&self, key: &ReaderKey) {
        if self.read_key().as_deref() == Some(key.0.as_slice()) {
            self.ended.borrow_mut().insert(key.1);
        }
    }

    /// The key of the network the config reads mock streams on: none while
    /// it disables reading or names no network.
    fn read_key(&self) -> Option<Vec<u8>> {
        let r = self.records.borrow();
        let c = &r.config;
        (c.enable_mock_streams && !c.mock_stream_network_key.is_empty())
            .then(|| c.mock_stream_network_key.clone())
    }

    /// Whether network `network_id` is the one the config reads on.
    fn reads_on(&self, network_id: u64, read: Option<&[u8]>) -> bool {
        read.is_some_and(|read| {
            self.networks
                .borrow()
                .get(&network_id)
                .is_some_and(|k| k == read)
        })
    }

    /// Indexes the mock stream listings of the view of the network the
    /// config reads, afresh.
    fn index_view(&self) {
        let network_id = self.read_key().and_then(|key| {
            self.networks
                .borrow()
                .iter()
                .find(|(_, k)| **k == key)
                .map(|(id, _)| *id)
        });
        let index = network_id
            .map(|id| {
                self.directory
                    .listings(id)
                    .into_iter()
                    .filter_map(|l| Some((l.id, mock_swarm(&l.listing)?)))
                    .collect()
            })
            .unwrap_or_default();
        *self.index.borrow_mut() = index;
    }

    /// Takes `c` as the device's config, as a store event wrote it.
    fn config_written(&self, c: DebugConfigRecord) {
        let changed = {
            let mut r = self.records.borrow_mut();
            if let Some((_, written)) = &mut r.read {
                *written = true;
            }
            let changed = r.config != c;
            r.config = c;
            changed
        };
        if changed {
            self.config_changed();
        }
    }

    /// Reads mock streams as the config now says.
    fn config_changed(&self) {
        self.ended.borrow_mut().clear();
        self.index_view();
        self.schedule_reconcile();
    }

    /// Ends the mock streams on the network `key`.
    fn network_deleted(&self, key: &[u8]) {
        let stops: Vec<Rc<Stop>> = self
            .producers
            .borrow()
            .values()
            .filter(|p| p.network_key == key)
            .map(|p| p.stop.clone())
            .collect();
        for s in stops {
            s.stop();
        }
    }

    fn handle_store_event(&self, e: &StoreEvent) {
        match e {
            StoreEvent::Changed(Record::DebugConfig(c)) => self.config_written(c.clone()),
            StoreEvent::Deleted(Record::DebugConfig(_)) => {
                self.config_written(DebugConfig::default_value().unwrap_or_default());
            }
            StoreEvent::Deleted(Record::Network(n)) => self.network_deleted(&network_key(n)),
            _ => {}
        }
    }

    fn handle_network_event(&self, e: &NetworkEvent) {
        if let NetworkEvent::Joined { network } | NetworkEvent::Updated { network } = e {
            self.publish_all(Some(&network_key(network)));
        }
    }

    /// The listing keepers of the mock streams.
    fn keepers(&self) -> Vec<ListingKeeper> {
        self.producers
            .borrow()
            .values()
            .filter_map(|p| p.keeper.clone())
            .collect()
    }

    fn handle_directory_event(&self, e: &DirectoryEvent) {
        for k in self.keepers() {
            k.handle_event(e);
        }
        let read = self.read_key();
        let read = read.as_deref();
        match e {
            DirectoryEvent::Started {
                network_id,
                network_key,
            } => {
                self.networks
                    .borrow_mut()
                    .insert(*network_id, network_key.clone());
                if read == Some(network_key.as_slice()) {
                    // The view starts empty.
                    self.index.borrow_mut().clear();
                    self.ended.borrow_mut().clear();
                    self.schedule_reconcile();
                }
            }
            DirectoryEvent::Stopped {
                network_id,
                network_key,
            } => {
                self.networks.borrow_mut().remove(network_id);
                if read == Some(network_key.as_slice()) {
                    self.index.borrow_mut().clear();
                    self.schedule_reconcile();
                }
            }
            DirectoryEvent::Lagged { network_id } => {
                if self.reads_on(*network_id, read) {
                    self.index_view();
                    self.schedule_reconcile();
                }
            }
            DirectoryEvent::Listing { network_id, event } => {
                if !self.reads_on(*network_id, read) {
                    return;
                }
                let changed = match event {
                    ListingEvent::Changed(l) => {
                        let indexed = {
                            let mut index = self.index.borrow_mut();
                            match mock_swarm(&l.listing) {
                                Some(s) => index.insert(l.id, s.clone()) != Some(s),
                                None => index.remove(&l.id).is_some(),
                            }
                        };
                        // A listing whose reader ended is read again once
                        // it changes.
                        self.ended.borrow_mut().remove(&l.id) || indexed
                    }
                    ListingEvent::Unpublished(l) => {
                        self.ended.borrow_mut().remove(&l.id);
                        self.index.borrow_mut().remove(&l.id).is_some()
                    }
                    ListingEvent::UserCountChanged { .. } => false,
                };
                if changed {
                    self.schedule_reconcile();
                }
            }
            DirectoryEvent::User { .. } | DirectoryEvent::AssetBundle { .. } => {}
        }
    }

    /// Reads the networks whose directories run.
    fn read_networks(&self) {
        let networks = self
            .directory
            .networks()
            .into_iter()
            .map(|n| (n.network_id, n.network_key))
            .collect();
        *self.networks.borrow_mut() = networks;
    }

    /// Starts reading the config and the mock streams' networks again, in
    /// place of any read that runs.
    fn start_read(self: &Rc<Self>) {
        let id = {
            let mut r = self.records.borrow_mut();
            let id = r.next_read;
            r.next_read += 1;
            r.read = Some((id, false));
            id
        };
        self.spawn(self.clone().read(id).boxed_local());
    }

    /// Reads the config, unless an event writes it meanwhile, and ends the
    /// mock streams whose network the store doesn't hold.
    async fn read(self: Rc<Self>, id: u64) {
        let keys: BTreeSet<Vec<u8>> = self
            .producers
            .borrow()
            .values()
            .map(|p| p.network_key.clone())
            .collect();
        let read = async {
            let mut tx = self.store.read().await?;
            let config = DebugConfig::get(&mut tx).await?.unwrap_or_default();
            let mut gone = Vec::new();
            for k in keys {
                if !network_stored(&mut tx, &k).await? {
                    gone.push(k);
                }
            }
            Ok::<_, strims_dao::Error>((config, gone))
        }
        .await;
        let (config, gone) = match read {
            Ok(r) => r,
            Err(e) => {
                self.metrics.borrow_mut().store_errors += 1;
                tracing::warn!(error = %e, "reading the debug config failed");
                let mut r = self.records.borrow_mut();
                if r.read.is_some_and(|(i, _)| i == id) {
                    r.read = None;
                }
                return;
            }
        };
        let changed = {
            let mut r = self.records.borrow_mut();
            match r.read {
                Some((i, false)) if i == id => {
                    r.read = None;
                    let changed = r.config != config;
                    r.config = config;
                    changed
                }
                Some((i, true)) if i == id => {
                    r.read = None;
                    false
                }
                _ => false,
            }
        };
        if changed {
            self.config_changed();
        }
        for k in gone {
            self.network_deleted(&k);
        }
        self.schedule_reconcile();
    }

    /// Whether `swarm` is one the node runs but for the readers' own.
    fn swarm_taken(&self, swarm: &SwarmId) -> bool {
        let read = self.readers.borrow().values().any(|r| r.swarm_id == *swarm);
        !read && self.transfers.find(swarm, b"").is_some()
    }

    /// Plans the readers from the config and the view, then ends and
    /// starts readers to match.
    fn reconcile(self: &Rc<Self>) {
        self.reconcile.set(false);
        let (enabled, key) = {
            let r = self.records.borrow();
            (
                r.config.enable_mock_streams,
                r.config.mock_stream_network_key.clone(),
            )
        };
        let wants = {
            let index = self.index.borrow();
            self.ended.borrow_mut().retain(|id| index.contains_key(id));
            let ended = self.ended.borrow();
            plan(
                enabled,
                &key,
                &index,
                self.config.max_readers,
                |id, swarm| ended.contains(&id) || self.swarm_taken(swarm),
            )
        };
        let stops: Vec<Rc<Stop>> = self
            .readers
            .borrow()
            .iter()
            .filter(|((k, id), r)| {
                !r.stop.is_stopped() && (*k != key || wants.get(id) != Some(&r.swarm_id))
            })
            .map(|(_, r)| r.stop.clone())
            .collect();
        for s in stops {
            s.stop();
        }
        for (id, swarm) in wants {
            let reader = (key.clone(), id);
            let stop = {
                let mut readers = self.readers.borrow_mut();
                if readers.contains_key(&reader) {
                    continue;
                }
                if readers.len() >= self.config.max_readers {
                    break;
                }
                let stop = Rc::new(Stop::default());
                readers.insert(
                    reader.clone(),
                    Reader {
                        swarm_id: swarm.clone(),
                        transfer: None,
                        started_at: self.timer.now(),
                        id: None,
                        segments: 0,
                        malformed: 0,
                        discontinuities: 0,
                        latency: None,
                        last_segment_at: None,
                        stop: stop.clone(),
                    },
                );
                stop
            };
            self.metrics.borrow_mut().readers_started += 1;
            let task = self.clone().run_reader(reader, swarm, stop);
            self.spawn(task.boxed_local());
        }
    }
}

/// Whether the store in `tx` holds a network with key `key`.
async fn network_stored<R: Read>(tx: &mut R, key: &[u8]) -> Result<bool, strims_dao::Error> {
    if key.is_empty() || key.len() > MAX_NETWORK_KEY_BYTES {
        return Ok(false);
    }
    Ok(!Networks::index_ids(tx, &Networks::BY_KEY, key)
        .await?
        .is_empty())
}

/// A mock stream the node writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockStreamInfo {
    pub id: u64,
    pub network_key: Vec<u8>,
    /// Empty until its swarm is created.
    pub swarm_uri: String,
    pub bitrate_kbps: u32,
    pub segment_interval_ms: u32,
    /// 0 runs the stream until it is stopped.
    pub timeout_ms: u32,
    pub started_at: Time,
    pub segments: u64,
    pub bytes_written: u64,
    /// The id of its listing on the network's directory, once published.
    pub listing_id: Option<u64>,
}

/// A mock stream the node reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderInfo {
    pub network_key: Vec<u8>,
    pub listing_id: u64,
    pub swarm_id: SwarmId,
    /// The stream's id, from its last segment.
    pub id: Option<u64>,
    pub segments: u64,
    /// Records that didn't decode as segments.
    pub malformed: u64,
    pub discontinuities: u64,
    /// The last segment's latency: when it was read less its timestamp.
    pub latency: Option<Duration>,
    /// When the last segment was read.
    pub last_segment_at: Option<Time>,
    pub started_at: Time,
}

/// Runs the device's debug config, the mock streams the node writes, and
/// those it reads; see the [crate docs](crate).
///
/// Clones share the control.
pub struct DebugControl<K: Kv, H: Host, T: Timer> {
    pub(crate) shared: Rc<Shared<K, H, T>>,
}

impl<K: Kv, H: Host, T: Timer> Clone for DebugControl<K, H, T> {
    fn clone(&self) -> Self {
        DebugControl {
            shared: self.shared.clone(),
        }
    }
}

/// What woke `run`.
enum Wake {
    Task,
    Store(Option<Recv<StoreEvent>>),
    Network(Option<Recv<NetworkEvent>>),
    Directory(Option<Recv<DirectoryEvent>>),
    Reconcile,
    Work,
}

impl<K: Kv, H: Host, T: Timer> DebugControl<K, H, T> {
    /// A control for the node behind `host`, keeping the device's debug
    /// config in `store`. Mock streams go on the networks `network` joins,
    /// as transfers run by `transfers`, listed through `directory`. Timed
    /// on `timer`; mock stream keys and ids are drawn from `rng`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: H,
        store: ReplicatedStore<K>,
        network: NetworkControl<K, H, T>,
        transfers: TransferControl<H, T>,
        directory: DirectoryControl<K, H, T>,
        timer: T,
        rng: DetRng,
        config: Config,
    ) -> Self {
        DebugControl {
            shared: Rc::new(Shared {
                host,
                store,
                network,
                transfers,
                directory,
                timer,
                rng: RefCell::new(rng),
                config,
                metrics: RefCell::default(),
                records: RefCell::default(),
                producers: RefCell::default(),
                readers: RefCell::default(),
                networks: RefCell::default(),
                index: RefCell::default(),
                ended: RefCell::default(),
                reconcile: Cell::new(false),
                spawned: RefCell::new(Vec::new()),
                waker: RefCell::new(None),
            }),
        }
    }

    /// The control's counters.
    pub fn metrics(&self) -> Metrics {
        *self.shared.metrics.borrow()
    }

    /// The device's debug config.
    pub async fn config(&self) -> Result<DebugConfigRecord, DebugError> {
        let mut tx = self.shared.store.read().await?;
        Ok(DebugConfig::get(&mut tx).await?.unwrap_or_default())
    }

    /// Stores `c` as the device's debug config.
    ///
    /// # Errors
    ///
    /// The config's network key is set and names none of the profile's
    /// networks, or the store failed.
    pub async fn set_config(&self, c: DebugConfigRecord) -> Result<DebugConfigRecord, DebugError> {
        let s = &self.shared;
        let mut tx = s.store.write(s.timer.now()).await?;
        if !c.mock_stream_network_key.is_empty()
            && !network_stored(&mut tx, &c.mock_stream_network_key).await?
        {
            return Err(DebugError::NetworkNotFound);
        }
        let c = DebugConfig::set(&mut tx, c).await?;
        tx.commit().await?;
        Ok(c)
    }

    /// Starts a mock stream of `bitrate_kbps` on the network `network_key`
    /// names, writing a segment every `segment_interval_ms` (0 for the
    /// default) for `timeout_ms` (0 until it is stopped): its id, once its
    /// swarm is created. See the [crate docs](crate#producing).
    ///
    /// # Errors
    ///
    /// The request is out of the [`Config`] bounds, the node writes its
    /// most mock streams, the key names none of the profile's networks, or
    /// a node call or the store failed.
    pub async fn start_mock_stream(
        &self,
        bitrate_kbps: u32,
        segment_interval_ms: u32,
        timeout_ms: u32,
        network_key: Vec<u8>,
    ) -> Result<u64, DebugError> {
        let s = &self.shared;
        let start = check_start(&s.config, bitrate_kbps, segment_interval_ms, timeout_ms)?;
        if network_key.is_empty() || network_key.len() > MAX_NETWORK_KEY_BYTES {
            return Err(DebugError::NetworkNotFound);
        }
        // The stream is held before its network is read, so a deletion
        // the read doesn't see ends it as a store event.
        let (id, key) = {
            let mut producers = s.producers.borrow_mut();
            if producers.len() >= s.config.max_mock_streams {
                return Err(DebugError::TooManyMockStreams);
            }
            let mut rng = s.rng.borrow_mut();
            let id = loop {
                let id = rand_core::Rng::next_u64(&mut *rng);
                if id != 0 && !producers.contains_key(&id) {
                    break id;
                }
            };
            let key = generate_key(&mut rng);
            producers.insert(
                id,
                Producer {
                    network_key: network_key.clone(),
                    start,
                    started_at: s.timer.now(),
                    swarm_uri: String::new(),
                    transfer: None,
                    keeper: None,
                    segments: 0,
                    bytes_written: 0,
                    stop: Rc::new(Stop::default()),
                },
            );
            (id, key)
        };
        let stored = async {
            let mut tx = s.store.read().await?;
            network_stored(&mut tx, &network_key).await
        }
        .await;
        if !matches!(stored, Ok(true)) {
            s.producers.borrow_mut().remove(&id);
            return Err(match stored {
                Err(e) => e.into(),
                Ok(_) => DebugError::NetworkNotFound,
            });
        }
        let reply = Reply::new();
        s.spawn(s.clone().run_producer(id, key, reply.clone()).boxed_local());
        reply.take().await
    }

    /// Stops mock stream `id`, if the node writes one.
    pub fn stop_mock_stream(&self, id: u64) {
        let stop = self
            .shared
            .producers
            .borrow()
            .get(&id)
            .map(|p| p.stop.clone());
        if let Some(s) = stop {
            s.stop();
        }
    }

    /// The mock streams the node writes, those ending included, in id
    /// order.
    pub fn mock_streams(&self) -> Vec<MockStreamInfo> {
        self.shared
            .producers
            .borrow()
            .iter()
            .map(|(id, p)| MockStreamInfo {
                id: *id,
                network_key: p.network_key.clone(),
                swarm_uri: p.swarm_uri.clone(),
                bitrate_kbps: p.start.bitrate_kbps,
                segment_interval_ms: p.start.segment_interval_ms,
                timeout_ms: p.start.timeout_ms,
                started_at: p.started_at,
                segments: p.segments,
                bytes_written: p.bytes_written,
                listing_id: p.keeper.as_ref().and_then(ListingKeeper::listing_id),
            })
            .collect()
    }

    /// The mock streams the node reads, those ending included, in network
    /// key, then listing id order.
    pub fn readers(&self) -> Vec<ReaderInfo> {
        self.shared
            .readers
            .borrow()
            .iter()
            .map(|((key, listing_id), r)| ReaderInfo {
                network_key: key.clone(),
                listing_id: *listing_id,
                swarm_id: r.swarm_id.clone(),
                id: r.id,
                segments: r.segments,
                malformed: r.malformed,
                discontinuities: r.discontinuities,
                latency: r.latency,
                last_segment_at: r.last_segment_at,
                started_at: r.started_at,
            })
            .collect()
    }

    /// Runs the control: follows the store's debug config and network
    /// deletions, publishes the mock streams' transfers on networks as the
    /// node joins them, feeds the listing keepers the directory's events,
    /// reads the mock streams the config and the view name, and carries the
    /// control's tasks: mock streams, readers, listing keepers and config
    /// reads. Never resolves; drop it to stop. Run it once per control.
    pub async fn run(&self) {
        let s = &self.shared;
        let mut store_events = Some(s.store.events().subscribe());
        let mut network_events = Some(s.network.subscribe());
        let mut directory_events = Some(s.directory.subscribe());
        s.start_read();
        s.read_networks();
        s.publish_all(None);
        let mut tasks: FuturesUnordered<LocalBoxFuture<'static, ()>> = FuturesUnordered::new();
        loop {
            let spawned: Vec<_> = s.spawned.borrow_mut().drain(..).collect();
            tasks.extend(spawned);
            let wake = poll_fn(|cx| {
                *s.waker.borrow_mut() = Some(cx.waker().clone());
                if let Poll::Ready(Some(())) = tasks.poll_next_unpin(cx) {
                    return Poll::Ready(Wake::Task);
                }
                if let Some(ev) = &mut store_events
                    && let Poll::Ready(r) = ev.poll_next_unpin(cx)
                {
                    return Poll::Ready(Wake::Store(r));
                }
                if let Some(ev) = &mut network_events
                    && let Poll::Ready(r) = ev.poll_next_unpin(cx)
                {
                    return Poll::Ready(Wake::Network(r));
                }
                if let Some(ev) = &mut directory_events
                    && let Poll::Ready(r) = ev.poll_next_unpin(cx)
                {
                    return Poll::Ready(Wake::Directory(r));
                }
                // After the events at hand, so that a burst of them is
                // reconciled once.
                if s.reconcile.get() {
                    return Poll::Ready(Wake::Reconcile);
                }
                if !s.spawned.borrow().is_empty() {
                    return Poll::Ready(Wake::Work);
                }
                Poll::Pending
            })
            .await;
            match wake {
                Wake::Store(Some(Recv::Event(e))) => s.handle_store_event(&e),
                Wake::Store(Some(Recv::Lagged)) => {
                    s.metrics.borrow_mut().store_lags += 1;
                    s.start_read();
                }
                Wake::Store(None) => store_events = None,
                Wake::Network(Some(Recv::Event(e))) => s.handle_network_event(&e),
                Wake::Network(Some(Recv::Lagged)) => s.publish_all(None),
                Wake::Network(None) => network_events = None,
                Wake::Directory(Some(Recv::Event(e))) => s.handle_directory_event(&e),
                Wake::Directory(Some(Recv::Lagged)) => {
                    for k in s.keepers() {
                        k.check();
                    }
                    s.read_networks();
                    s.index_view();
                    s.schedule_reconcile();
                }
                Wake::Directory(None) => directory_events = None,
                Wake::Reconcile => s.reconcile(),
                Wake::Task | Wake::Work => {}
            }
        }
    }
}
