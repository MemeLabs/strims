// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The whisper control: a whisper port per joined network, the whisper
//! lease, and the delivery queue; see the [module docs](super).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Poll, Waker};

use futures_util::future::LocalBoxFuture;
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, StreamExt};
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_crypto::{certificate_body, membership_trusted};
use strims_dao::events::Record;
use strims_dao::lease;
use strims_dao::schema::chat::{WhisperRecords, WhisperThreads, state_key};
use strims_dao::schema::network::network_key;
use strims_dao::{Recv, ReplicatedStore, StoreEvent, Table};
use strims_directory::{Host, MAX_ALIAS_BYTES, MAX_PEER_KEY_BYTES, Net};
use strims_kv::Kv;
use strims_network::{NetworkControl, NetworkEvent};
use strims_proto::chat::v1::message::Entities;
use strims_proto::chat::v1::{Message, MessageState, WhisperRecord, WhisperThread};
use strims_proto::profile::v1::Profile;
use strims_proto::types::Certificate;
use strims_rpc::vpn::{VpnClient, VpnConfig};
use strims_rpc::{RetryPolicy, Timer};

use super::service::{self, Port};
use super::{Deliveries, WhisperLimits, lease_key, thread_sent};
use crate::entities::{Patterns, extract};
use crate::rate::{RateConfig, RateLimiter};
use crate::text::ParserContext;

/// A network's key.
type NetKey = Vec<u8>;

/// The wait before opening a network's whisper port again after it failed
/// or closed.
const PORT_RETRY: Duration = Duration::from_secs(10);

/// The wait before reading the ENQUEUED records again after the read
/// failed.
const SEED_RETRY: Duration = Duration::from_secs(10);

/// How whispers run on a node.
#[derive(Clone, Debug)]
pub struct WhisperConfig {
    /// The bounds of receipt and delivery.
    pub limits: WhisperLimits,
    /// Each sender's rate of `SendMessage` calls, across the profile's
    /// whisper ports. The default is [`RateConfig::default`].
    pub rate: RateConfig,
    /// The retries of a delivery's call within its attempt. A whisper is
    /// stored once per (id, sender), so its call may repeat. The default
    /// is [`RetryPolicy::vpn`].
    pub call_policy: RetryPolicy,
    /// The whisper ports' vpn connections.
    pub vpn: VpnConfig,
    /// The longest a delivery attempt takes, from resolving the
    /// recipient's address to the answer; 30 s by default.
    pub attempt_deadline: Duration,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        WhisperConfig {
            limits: WhisperLimits::default(),
            rate: RateConfig::default(),
            call_policy: RetryPolicy::vpn(),
            vpn: VpnConfig::default(),
            attempt_deadline: Duration::from_secs(30),
        }
    }
}

/// What the whisper control has done in this process.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WhisperMetrics {
    /// Whispers received and stored.
    pub received: u64,
    /// Whispers received again, answered without a write.
    pub duplicates: u64,
    /// Whispers refused with [`WHISPER_REFUSED`](super::WHISPER_REFUSED).
    pub refused: u64,
    /// Calls over their sender's rate.
    pub rate_limited: u64,
    /// Whispers [`WhisperControl::enqueue`] stored.
    pub enqueued: u64,
    /// Delivery attempts started.
    pub attempts: u64,
    /// Records this process made DELIVERED, and FAILED.
    pub delivered: u64,
    pub failed: u64,
    /// Times this process took the whisper lease.
    pub leases: u64,
    /// Whisper ports opened.
    pub ports_opened: u64,
    /// Records queued for delivery, and attempts outstanding, now.
    pub pending: usize,
    pub in_flight: usize,
    /// Node and store operations that failed.
    pub host_errors: u64,
    pub store_errors: u64,
}

/// Why [`WhisperControl::enqueue`] stored nothing.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum EnqueueError {
    /// The profile has no certificate on the network.
    #[error("network not found")]
    NoNetwork,
    /// The recipient is the profile itself.
    #[error("cannot whisper yourself")]
    ToSelf,
    /// The body is over [`WhisperLimits::max_body_bytes`].
    #[error("message too long")]
    TooLong,
    /// The recipient's key is empty, or a key or the alias is over its
    /// bound.
    #[error("request too large")]
    TooLarge,
    /// [`WhisperLimits::max_pending`] whispers await delivery.
    #[error("too many whispers pending")]
    Full,
    #[error(transparent)]
    Store(#[from] strims_dao::Error),
}

/// A joined network.
struct Joined<H: Host> {
    net: Net<H>,
    network_id: u64,
    /// The profile's membership certificate on the network, as last seen.
    certificate: Option<Certificate>,
    /// The port, while its task runs.
    port: Option<Rc<Port>>,
    /// When to open the port again.
    retry: Option<Time>,
}

pub(crate) struct Shared<K: Kv, H: Host, T: Timer> {
    pub(crate) host: H,
    pub(crate) store: ReplicatedStore<K>,
    pub(crate) profile: Profile,
    pub(crate) network: NetworkControl<K, H, T>,
    pub(crate) timer: T,
    pub(crate) config: WhisperConfig,
    /// Draws the lease token and recheck delays.
    rng: RefCell<DetRng>,
    /// Bound: the profile's networks.
    nets: RefCell<BTreeMap<NetKey, Joined<H>>>,
    /// Whether this process holds the whisper lease.
    holding: Cell<bool>,
    /// Counts the lease changes. An attempt reports to the queue only
    /// under the lease change it started in.
    generation: Cell<u64>,
    /// The records to deliver, while this process holds the lease.
    deliveries: RefCell<Deliveries>,
    /// An ENQUEUED record didn't fit in `deliveries`; they are read again
    /// once there is room.
    overflow: Cell<bool>,
    /// Bound: [`RateConfig::max_keys`].
    pub(crate) limiter: RefCell<RateLimiter>,
    /// Compiled on first use.
    patterns: RefCell<Option<Patterns>>,
    /// The empty context whisper entities are extracted with: whispers
    /// have no emotes, modifiers, tags or nicks.
    context: ParserContext,
    pub(crate) metrics: RefCell<WhisperMetrics>,
    /// Tasks for `run` to start. Bound: one per network's port and its
    /// connection, the lease task, a seed, and the attempts in flight.
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

    pub(crate) fn wake_run(&self) {
        let w = self.waker.borrow_mut().take();
        if let Some(w) = w {
            w.wake();
        }
    }

    pub(crate) fn holding(&self) -> bool {
        self.holding.get()
    }

    /// `body`'s entities.
    pub(crate) fn entities(&self, body: &str) -> Entities {
        let mut p = self.patterns.borrow_mut();
        extract(body, &self.context, p.get_or_insert_with(Patterns::new))
    }

    /// The network whose key is `key` and its port's client, while the
    /// port is open.
    pub(crate) fn port_client(&self, key: &[u8]) -> Option<(Net<H>, VpnClient)> {
        let nets = self.nets.borrow();
        let n = nets.get(key)?;
        let client = n.port.as_ref()?.client()?;
        Some((n.net, client))
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

    /// Follows every network the node has joined, and only those.
    fn sync_all(self: &Rc<Self>) {
        let joined: BTreeSet<NetKey> = self
            .network
            .networks()
            .iter()
            .filter(|i| i.net.is_some())
            .map(|i| network_key(&i.network))
            .collect();
        let gone: Vec<NetKey> = self
            .nets
            .borrow()
            .keys()
            .filter(|k| !joined.contains(*k))
            .cloned()
            .collect();
        for key in gone.iter().chain(&joined) {
            self.sync_network(key);
        }
    }

    /// Follows the network control's state of network `key`: opens its
    /// port, passes a new certificate on, or closes it.
    fn sync_network(self: &Rc<Self>, key: &[u8]) {
        let joined = self
            .network
            .network(key)
            .and_then(|i| i.net.map(|net| (i.network, net)));
        let mut renewed = None;
        let replaced = {
            let mut nets = self.nets.borrow_mut();
            match (joined, nets.get_mut(key)) {
                (None, Some(_)) => nets.remove(key),
                (None, None) => None,
                (Some((n, net)), Some(j)) if j.net == net && j.network_id == n.id => {
                    if j.certificate != n.certificate {
                        j.certificate = n.certificate;
                        renewed = j.port.clone();
                    }
                    None
                }
                (Some((n, net)), _) => nets.insert(
                    key.to_vec(),
                    Joined {
                        net,
                        network_id: n.id,
                        certificate: n.certificate,
                        port: None,
                        retry: None,
                    },
                ),
            }
        };
        if let Some(p) = replaced.and_then(|j| j.port) {
            p.stop();
        }
        if let Some(p) = renewed
            && let Some(client) = p.client()
            && let Some(cert) = self.network.host_certificate(key)
            && let Err(e) = client.set_certificate(&cert)
        {
            tracing::debug!(error = %e, "updating a whisper port's certificate failed");
        }
        self.ensure_port(key);
    }

    /// Opens network `key`'s port unless it is open or opening, its retry
    /// isn't due, or the profile's membership isn't trusted.
    fn ensure_port(self: &Rc<Self>, key: &[u8]) {
        let now = self.timer.now();
        let start = {
            let mut nets = self.nets.borrow_mut();
            let Some(j) = nets.get_mut(key) else {
                return;
            };
            if j.port.is_some() || j.retry.is_some_and(|t| t > now) {
                return;
            }
            // An untrusted membership waits for the network's next update.
            j.retry = None;
            let trusted = j
                .certificate
                .as_ref()
                .is_some_and(|c| membership_trusted(c, now));
            if !trusted {
                return;
            }
            let p = Rc::new(Port::default());
            j.port = Some(p.clone());
            (j.net, p)
        };
        let (net, p) = start;
        self.spawn(service::run_port(self.clone(), key.to_vec(), net, p).boxed_local());
    }

    /// Port `p` of network `key` closed: it opens again after
    /// [`PORT_RETRY`] unless it was stopped.
    pub(crate) fn port_closed(&self, key: &[u8], p: &Rc<Port>) {
        let now = self.timer.now();
        let closed = {
            let mut nets = self.nets.borrow_mut();
            match nets.get_mut(key) {
                Some(j) if j.port.as_ref().is_some_and(|q| Rc::ptr_eq(q, p)) => {
                    j.retry = Some(now + PORT_RETRY);
                    j.port.take()
                }
                _ => None,
            }
        };
        drop(closed);
        self.wake_run();
    }

    /// Opens the ports whose retry is due.
    fn tick(self: &Rc<Self>) {
        let now = self.timer.now();
        let due: Vec<NetKey> = self
            .nets
            .borrow()
            .iter()
            .filter(|(_, j)| j.retry.is_some_and(|t| t <= now))
            .map(|(k, _)| k.clone())
            .collect();
        for key in due {
            self.ensure_port(&key);
        }
    }

    /// When the next port retry or delivery attempt is due.
    fn next_timer(&self) -> Option<Time> {
        let retry = self.nets.borrow().values().filter_map(|j| j.retry).min();
        let attempt = if self.holding.get() {
            self.deliveries.borrow().poll_timeout()
        } else {
            None
        };
        retry.into_iter().chain(attempt).min()
    }

    fn ports(&self) -> Vec<Rc<Port>> {
        self.nets
            .borrow()
            .values()
            .filter_map(|j| j.port.clone())
            .collect()
    }

    // --- the lease ----------------------------------------------------------

    /// Takes or loses the lease: the ports publish or withdraw their
    /// addresses, and the queue starts over.
    fn set_holding(self: &Rc<Self>, held: bool) {
        if self.holding.replace(held) == held {
            return;
        }
        self.generation.set(self.generation.get().wrapping_add(1));
        let prev = self.deliveries.replace(Deliveries::new(self.config.limits));
        drop(prev);
        self.overflow.set(false);
        if held {
            self.metrics.borrow_mut().leases += 1;
            self.reseed();
        }
        for p in self.ports() {
            p.changed();
        }
        self.wake_run();
    }

    // --- deliveries ---------------------------------------------------------

    /// Queues the ENQUEUED records the store holds.
    fn reseed(self: &Rc<Self>) {
        let generation = self.generation.get();
        self.spawn(seed(self.clone(), generation).boxed_local());
    }

    /// Queues `r` if it is ENQUEUED and fits, aged from its message's
    /// server time.
    fn enqueue_record(&self, r: &WhisperRecord) {
        let now = self.timer.now();
        let sent = r
            .message
            .as_ref()
            .map_or(now, |m| Time::from_unix_millis(m.server_time))
            .min(now);
        let mut d = self.deliveries.borrow_mut();
        if !d.enqueue(sent, r)
            && r.state == MessageState::Enqueued as i32
            && d.len() >= self.config.limits.max_pending
        {
            self.overflow.set(true);
        }
    }

    fn handle_store_event(self: &Rc<Self>, e: &StoreEvent) {
        if !self.holding.get() {
            return;
        }
        match e {
            StoreEvent::Changed(Record::WhisperRecord(r)) => {
                if r.state == MessageState::Enqueued as i32 {
                    self.enqueue_record(r);
                } else {
                    self.deliveries.borrow_mut().cancel(r.id);
                }
            }
            StoreEvent::Deleted(Record::WhisperRecord(r)) => {
                self.deliveries.borrow_mut().cancel(r.id);
            }
            _ => {}
        }
    }

    /// Starts the attempts that are due.
    fn pump(self: &Rc<Self>) {
        if !self.holding.get() {
            return;
        }
        let now = self.timer.now();
        let generation = self.generation.get();
        loop {
            let d = self.deliveries.borrow_mut().poll_send(now);
            let Some(d) = d else {
                break;
            };
            self.metrics.borrow_mut().attempts += 1;
            self.spawn(service::deliver(self.clone(), generation, d).boxed_local());
        }
    }

    /// Reports the outcome of an attempt started under lease change
    /// `generation`: the state to store, if any. Under a later lease
    /// change, only a delivery or a refusal settles the record.
    pub(crate) fn finish(
        self: &Rc<Self>,
        generation: u64,
        id: u64,
        outcome: &service::Outcome,
    ) -> Option<MessageState> {
        use service::Outcome;
        let now = self.timer.now();
        if self.generation.get() != generation {
            return match outcome {
                Outcome::Delivered => Some(MessageState::Delivered),
                Outcome::Refused => Some(MessageState::Failed),
                Outcome::Failed(_) | Outcome::Stale => None,
            };
        }
        let (state, room) = {
            let mut d = self.deliveries.borrow_mut();
            let state = match outcome {
                Outcome::Delivered => d.finished(now, id, true),
                Outcome::Refused => d.refused(id),
                Outcome::Failed(_) => d.finished(now, id, false),
                Outcome::Stale => {
                    d.cancel(id);
                    d.finished(now, id, false)
                }
            };
            (state, d.len() < self.config.limits.max_pending)
        };
        if room && self.overflow.replace(false) {
            self.reseed();
        }
        self.wake_run();
        state
    }

    /// Stores `state` for record `id` while it is still ENQUEUED.
    pub(crate) async fn write_state(&self, id: u64, state: MessageState) {
        let res = async {
            let mut tx = self.store.write(self.timer.now()).await?;
            let Some(mut r) = WhisperRecords::get(&mut tx, id).await? else {
                return Ok(false);
            };
            if r.state != MessageState::Enqueued as i32 {
                return Ok(false);
            }
            r.state = state as i32;
            WhisperRecords::update(&mut tx, r).await?;
            tx.commit().await?;
            Ok::<_, strims_dao::Error>(true)
        }
        .await;
        match res {
            Ok(true) if state == MessageState::Delivered => {
                self.metrics.borrow_mut().delivered += 1
            }
            Ok(true) => self.metrics.borrow_mut().failed += 1,
            Ok(false) => {}
            Err(e) => {
                self.metrics.borrow_mut().store_errors += 1;
                tracing::warn!(whisper = id, error = %e, "storing a whisper's state failed");
            }
        }
    }
}

/// Holds the lease when it can, refreshing it every [`lease::REFRESH`],
/// and tries again after every [`lease::recheck_delay`] when it can't.
async fn hold_lease<K: Kv, H: Host, T: Timer>(s: Rc<Shared<K, H, T>>) {
    let key = lease_key();
    let token = lease::new_token(&mut s.rng.borrow_mut());
    loop {
        let held = match lease::try_lock(s.store.store(), &key, &token, s.timer.now()).await {
            Ok(()) => true,
            Err(strims_dao::Error::LeaseBusy) => false,
            Err(e) => {
                s.metrics.borrow_mut().store_errors += 1;
                tracing::warn!(error = %e, "taking the whisper lease failed");
                false
            }
        };
        s.set_holding(held);
        let wait = if held {
            lease::REFRESH
        } else {
            lease::recheck_delay(&mut s.rng.borrow_mut())
        };
        s.timer.sleep_until(s.timer.now() + wait).await;
    }
}

/// Queues the oldest ENQUEUED records, as many as the queue holds, while
/// the lease hasn't changed since `generation`.
async fn seed<K: Kv, H: Host, T: Timer>(s: Rc<Shared<K, H, T>>, generation: u64) {
    let max = s.config.limits.max_pending;
    loop {
        let res = async {
            let mut tx = s.store.read().await?;
            let key = state_key(MessageState::Enqueued as i32);
            let ids = WhisperRecords::index_ids(&mut tx, &WhisperRecords::BY_STATE, &key).await?;
            let mut records = Vec::new();
            for &id in ids.iter().take(max) {
                records.extend(WhisperRecords::get(&mut tx, id).await?);
            }
            Ok::<_, strims_dao::Error>((records, ids.len() > max))
        }
        .await;
        if s.generation.get() != generation {
            return;
        }
        match res {
            Ok((records, more)) => {
                for r in &records {
                    s.enqueue_record(r);
                }
                if more {
                    s.overflow.set(true);
                }
                s.wake_run();
                return;
            }
            Err(e) => {
                s.metrics.borrow_mut().store_errors += 1;
                tracing::warn!(error = %e, "reading the enqueued whispers failed");
                s.timer.sleep_until(s.timer.now() + SEED_RETRY).await;
                if s.generation.get() != generation {
                    return;
                }
            }
        }
    }
}

/// Runs a profile's whispers on a node; see the [module docs](super).
///
/// Clones share the control.
pub struct WhisperControl<K: Kv, H: Host, T: Timer> {
    pub(crate) shared: Rc<Shared<K, H, T>>,
}

impl<K: Kv, H: Host, T: Timer> Clone for WhisperControl<K, H, T> {
    fn clone(&self) -> Self {
        WhisperControl {
            shared: self.shared.clone(),
        }
    }
}

/// What woke `run`.
enum Wake {
    Task,
    Network(Option<Recv<NetworkEvent>>),
    Store(Option<Recv<StoreEvent>>),
    Timer,
    Work,
}

impl<K: Kv, H: Host, T: Timer> WhisperControl<K, H, T> {
    /// A control for `profile` on the node behind `host`, serving whispers
    /// on the networks `network` joins and keeping them in `store`, timed
    /// on `timer`.
    pub fn new(
        host: H,
        store: ReplicatedStore<K>,
        profile: Profile,
        network: NetworkControl<K, H, T>,
        timer: T,
        rng: DetRng,
        config: WhisperConfig,
    ) -> Self {
        WhisperControl {
            shared: Rc::new(Shared {
                host,
                store,
                profile,
                network,
                timer,
                rng: RefCell::new(rng),
                nets: RefCell::new(BTreeMap::new()),
                holding: Cell::new(false),
                generation: Cell::new(0),
                deliveries: RefCell::new(Deliveries::new(config.limits)),
                overflow: Cell::new(false),
                limiter: RefCell::new(RateLimiter::new(config.rate)),
                patterns: RefCell::new(None),
                context: ParserContext::new(),
                metrics: RefCell::default(),
                spawned: RefCell::new(Vec::new()),
                waker: RefCell::new(None),
                config,
            }),
        }
    }

    /// Whether this process holds the whisper lease: it publishes the
    /// profile's whisper addresses and delivers its whispers.
    pub fn holds_lease(&self) -> bool {
        self.shared.holding()
    }

    /// Whether the profile's whisper address is published from this
    /// process on the network whose key is `network_key`.
    pub fn published(&self, network_key: &[u8]) -> bool {
        self.shared
            .nets
            .borrow()
            .get(network_key)
            .and_then(|j| j.port.as_ref())
            .is_some_and(|p| p.is_published())
    }

    /// What the control has done in this process.
    pub fn metrics(&self) -> WhisperMetrics {
        let s = &self.shared;
        let mut m = s.metrics.borrow().clone();
        let d = s.deliveries.borrow();
        m.pending = d.len();
        m.in_flight = d.in_flight();
        m
    }

    /// Stores a whisper of `body` to the profile with `peer_key` on the
    /// network whose key is `network_key`, ENQUEUED for delivery, and
    /// updates the recipient's thread: `alias` is the recipient's
    /// certificate subject, and `server_key` the chat server the whisper
    /// is sent from, if any. Resolves to the stored record.
    ///
    /// # Errors
    ///
    /// [`EnqueueError`]: the profile isn't a member of the network, the
    /// recipient is the profile, the body or a key is over its bound,
    /// [`WhisperLimits::max_pending`] whispers await delivery, or the
    /// store failed.
    pub async fn enqueue(
        &self,
        network_key: &[u8],
        server_key: &[u8],
        peer_key: &[u8],
        alias: &str,
        body: &str,
    ) -> Result<WhisperRecord, EnqueueError> {
        let s = &self.shared;
        let limits = s.config.limits;
        if body.len() > limits.max_body_bytes {
            return Err(EnqueueError::TooLong);
        }
        if peer_key.is_empty()
            || peer_key.len() > MAX_PEER_KEY_BYTES
            || server_key.len() > MAX_PEER_KEY_BYTES
            || alias.len() > MAX_ALIAS_BYTES
        {
            return Err(EnqueueError::TooLarge);
        }
        let own = s
            .network
            .network(network_key)
            .and_then(|i| i.network.certificate)
            .and_then(|c| certificate_body(&c).ok())
            .ok_or(EnqueueError::NoNetwork)?;
        if own.key == peer_key {
            return Err(EnqueueError::ToSelf);
        }
        let entities = s.entities(body);
        let now = s.timer.now();
        let mut tx = s.store.write(now).await?;
        let key = state_key(MessageState::Enqueued as i32);
        let pending = WhisperRecords::index_ids(&mut tx, &WhisperRecords::BY_STATE, &key).await?;
        if pending.len() >= limits.max_pending {
            return Err(EnqueueError::Full);
        }
        let record = WhisperRecord {
            id: tx.generate_id().await?,
            network_key: network_key.to_vec(),
            server_key: server_key.to_vec(),
            peer_key: peer_key.to_vec(),
            state: MessageState::Enqueued as i32,
            message: Some(Message {
                server_time: now.unix_millis(),
                peer_key: own.key,
                nick: own.subject,
                body: body.to_owned(),
                entities: Some(entities),
                ..Default::default()
            }),
            ..Default::default()
        };
        let record = WhisperRecords::insert(&mut tx, record).await?;
        let ix = &WhisperThreads::BY_PEER_KEY;
        let mut thread = match WhisperThreads::index_get(&mut tx, ix, peer_key).await? {
            Some(t) => t,
            None => WhisperThread {
                id: tx.generate_id().await?,
                peer_key: peer_key.to_vec(),
                ..Default::default()
            },
        };
        thread_sent(&mut thread, alias, record.id, now);
        WhisperThreads::upsert(&mut tx, thread).await?;
        tx.commit().await?;
        s.metrics.borrow_mut().enqueued += 1;
        Ok(record)
    }

    /// Marks the whispers from the profile with `peer_key` READ, and its
    /// thread read.
    pub async fn mark_read(&self, peer_key: &[u8]) -> Result<(), strims_dao::Error> {
        let s = &self.shared;
        let mut tx = s.store.write(s.timer.now()).await?;
        let ix = &WhisperThreads::BY_PEER_KEY;
        if let Some(mut t) = WhisperThreads::index_get(&mut tx, ix, peer_key).await?
            && t.has_unread
        {
            t.has_unread = false;
            WhisperThreads::update(&mut tx, t).await?;
        }
        let ix = &WhisperRecords::UNREAD_BY_PEER_KEY;
        for id in WhisperRecords::index_ids(&mut tx, ix, peer_key).await? {
            if let Some(mut r) = WhisperRecords::get(&mut tx, id).await? {
                r.state = MessageState::Read as i32;
                WhisperRecords::update(&mut tx, r).await?;
            }
        }
        tx.commit().await
    }

    /// Runs the whispers: follows the network control and the store,
    /// serves the whisper ports, contends for the lease, and delivers
    /// while it holds it. Never resolves; drop it to stop.
    pub async fn run(&self) {
        let s = &self.shared;
        let mut networks = Some(s.network.subscribe());
        let mut store = Some(s.store.events().subscribe());
        s.sync_all();
        s.spawn(hold_lease(s.clone()).boxed_local());
        let mut tasks: FuturesUnordered<LocalBoxFuture<'static, ()>> = FuturesUnordered::new();
        let mut sleep: Option<(Time, Pin<Box<T::Sleep>>)> = None;
        loop {
            s.pump();
            let spawned: Vec<_> = s.spawned.borrow_mut().drain(..).collect();
            tasks.extend(spawned);
            let wake = poll_fn(|cx| {
                let prev = s.waker.replace(Some(cx.waker().clone()));
                drop(prev);
                if let Poll::Ready(Some(())) = tasks.poll_next_unpin(cx) {
                    return Poll::Ready(Wake::Task);
                }
                if let Some(ev) = &mut networks
                    && let Poll::Ready(r) = ev.poll_next_unpin(cx)
                {
                    return Poll::Ready(Wake::Network(r));
                }
                if let Some(ev) = &mut store
                    && let Poll::Ready(r) = ev.poll_next_unpin(cx)
                {
                    return Poll::Ready(Wake::Store(r));
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
                Wake::Network(Some(Recv::Event(e))) => s.handle_network_event(&e),
                Wake::Network(Some(Recv::Lagged)) => s.sync_all(),
                Wake::Network(None) => networks = None,
                Wake::Store(Some(Recv::Event(e))) => s.handle_store_event(&e),
                Wake::Store(Some(Recv::Lagged)) => {
                    if s.holding() {
                        s.reseed();
                    }
                }
                Wake::Store(None) => store = None,
                Wake::Timer => s.tick(),
                Wake::Task | Wake::Work => {}
            }
        }
    }
}
