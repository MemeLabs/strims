// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The chat control: a runner per chat server that the profile owns or
//! that client streams read, the calls this node makes to chat servers,
//! and the member port per network that carries them.

use std::cell::{Cell, RefCell, RefMut};
use std::collections::{BTreeMap, BTreeSet};
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Poll, Waker};

use futures_util::future::{AbortHandle, Aborted, LocalBoxFuture, abortable};
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, StreamExt};
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_crypto::membership_trusted;
use strims_dao::events::Record;
use strims_dao::schema::chat::ChatServers;
use strims_dao::schema::network::network_key;
use strims_dao::{Broadcast, Recv, ReplicatedStore, StoreEvent, Subscription, Table};
use strims_directory::{Caller, DirectoryControl, DirectoryEvent, Host, ListingEvent, Net};
use strims_kv::Kv;
use strims_network::{NetworkControl, NetworkEvent};
use strims_proto::chat::v1::{ChatClient, GetMuteResponse, MuteRequest, UnmuteRequest};
use strims_proto::profile::v1::Profile;
use strims_proto::service::{CallError, Error as RpcError};
use strims_rpc::vpn::{self, VpnClient};
use strims_rpc::{RetryClient, Timer};
use strims_transfer::TransferControl;

use crate::client::ClientStream;
use crate::entities::Patterns;
use crate::events::ChatEvent;
use crate::services::{self, Reply, Request};
use crate::{
    ADDRESS_SALT, ChatError, Config, Metrics, Server, ServerAction, ServerMetrics, runner,
};

/// A network's key.
pub(crate) type NetKey = Vec<u8>;

pub(crate) type Swarm<H> = <H as strims_transfer::Host>::Swarm;

/// The wait before opening a network's member port again after it failed
/// or closed.
const MEMBER_RETRY: Duration = Duration::from_secs(10);

/// Why a chat call or client stream failed.
///
/// A frontend answers with [`ChatFailure::to_rpc`]: a remote error as the
/// server sent it, and anything else with code 0 and this error's message,
/// which for a refusal is the [`ChatError`]'s, as the server would have
/// sent it.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum ChatFailure {
    /// The node doesn't run the network.
    #[error("network not found")]
    NoNetwork,
    /// The node hasn't joined the network, or the profile's membership
    /// isn't trusted yet, so the node can't call the server.
    #[error("network not joined")]
    NotJoined,
    /// The node runs or reads the server on another network.
    #[error("chat server runs on another network")]
    OtherNetwork,
    /// The network's hash table has no address for the server, the
    /// server stopped while the call ran, or the profile has no server with
    /// that id.
    #[error("chat server not found")]
    NoServer,
    /// The node reads [`Config::max_rooms`] rooms.
    #[error("too many chat rooms open")]
    TooManyRooms,
    /// The room has [`Config::max_streams_per_room`] streams.
    #[error("too many streams open on the chat room")]
    TooManyStreams,
    /// The server refused the call.
    #[error(transparent)]
    Refused(ChatError),
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

impl ChatFailure {
    /// The RPC error a frontend answers with; see the type docs.
    pub fn to_rpc(&self) -> RpcError {
        match self {
            ChatFailure::Remote(e) => e.clone(),
            e => RpcError::new(0, e.to_string()),
        }
    }

    pub(crate) fn from_call(e: CallError) -> ChatFailure {
        match e {
            CallError::Remote(e) => match ChatError::from_message(&e.message) {
                Some(c) => ChatFailure::Refused(c),
                None => ChatFailure::Remote(e),
            },
            e => ChatFailure::Call(e),
        }
    }
}

/// What a server's runner does on this node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerMode {
    /// Neither serving nor reading: the network isn't joined or trusted,
    /// another process holds the server's lease, or nothing reads it.
    Idle,
    /// Reading the server's swarms for client streams.
    Reading,
    /// Serving the server.
    Serving,
}

/// A chat server this node runs or reads, as it is now.
#[derive(Clone, Debug, PartialEq)]
pub struct ChatServerStatus {
    pub server_key: Vec<u8>,
    pub network_key: Vec<u8>,
    /// The id of the profile's record of the server, while the profile
    /// owns it.
    pub server_id: Option<u64>,
    pub mode: ServerMode,
    /// Client streams open on the server.
    pub streams: usize,
    /// The server's directory listing, while this process serves it and
    /// the directory has it.
    pub listing_id: Option<u64>,
    /// The server's counters, while this process serves it.
    pub server: Option<ServerMetrics>,
}

/// A chat server on this node, from the first reason to run it (an owned
/// record, a client stream) until the last is gone. Its network is fixed:
/// a record that moves to another network gets a new runner.
pub(crate) struct Runner<H: Host> {
    /// The server's public key.
    pub(crate) key: Vec<u8>,
    pub(crate) network_key: NetKey,
    /// The profile's record of the server, while it owns it.
    pub(crate) record: RefCell<Option<strims_proto::chat::v1::Server>>,
    /// The event and asset swarms client streams read, while the mode task
    /// holds them, and the number of times it set them.
    pub(crate) swarms: Cell<Option<(Swarm<H>, Swarm<H>)>>,
    pub(crate) generation: Cell<u64>,
    /// The client streams' wakers, by slot. Bound:
    /// [`Config::max_streams_per_room`].
    pub(crate) streams: RefCell<BTreeMap<u64, Option<Waker>>>,
    pub(crate) next_stream: Cell<u64>,
    /// When the swarms stop lingering after the last stream closed.
    pub(crate) linger_until: Cell<Option<Time>>,
    /// The server, while this process serves it.
    pub(crate) server: RefCell<Option<ServerState>>,
    pub(crate) mode: Cell<ServerMode>,
    /// The id of the server's listing, while it serves.
    pub(crate) listing_id: Cell<Option<u64>>,
    /// When the next asset sync is due, and whether it must be unified.
    pub(crate) assets_due: Cell<Option<Time>>,
    pub(crate) unified_due: Cell<bool>,
    /// The listing keeper must publish, and its wake.
    pub(crate) republish: Cell<bool>,
    pub(crate) keeper_poked: Cell<bool>,
    pub(crate) keeper_waker: RefCell<Option<Waker>>,
    /// The runner stopped; its mode task winds down.
    pub(crate) stopped: Cell<bool>,
    /// Something the mode task looks at changed since it last looked.
    pub(crate) changed: Cell<bool>,
    /// Wakes the mode task.
    pub(crate) waker: RefCell<Option<Waker>>,
    /// The server's runner before this one, until its mode task ends: the
    /// two would share swarm ids, transfer ids and the address.
    pub(crate) previous: RefCell<Option<Rc<Runner<H>>>>,
    /// The mode task ended, and the task waiting for it.
    pub(crate) done: Cell<bool>,
    pub(crate) done_waker: RefCell<Option<Waker>>,
}

impl<H: Host> Runner<H> {
    fn new(
        key: Vec<u8>,
        network_key: NetKey,
        record: Option<strims_proto::chat::v1::Server>,
        previous: Option<Rc<Runner<H>>>,
    ) -> Runner<H> {
        Runner {
            key,
            network_key,
            record: RefCell::new(record),
            swarms: Cell::new(None),
            generation: Cell::new(0),
            streams: RefCell::new(BTreeMap::new()),
            next_stream: Cell::new(0),
            linger_until: Cell::new(None),
            server: RefCell::new(None),
            mode: Cell::new(ServerMode::Idle),
            listing_id: Cell::new(None),
            assets_due: Cell::new(None),
            unified_due: Cell::new(false),
            republish: Cell::new(true),
            keeper_poked: Cell::new(false),
            keeper_waker: RefCell::new(None),
            stopped: Cell::new(false),
            changed: Cell::new(false),
            waker: RefCell::new(None),
            previous: RefCell::new(previous),
            done: Cell::new(false),
            done_waker: RefCell::new(None),
        }
    }

    pub(crate) fn wake(&self) {
        let w = self.waker.borrow_mut().take();
        if let Some(w) = w {
            w.wake();
        }
    }

    /// Has the mode task look at the runner's state again.
    pub(crate) fn poke(&self) {
        self.changed.set(true);
        self.wake();
    }

    pub(crate) fn serving(&self) -> bool {
        self.server.borrow().is_some()
    }

    pub(crate) fn server_id(&self) -> Option<u64> {
        self.record.borrow().as_ref().map(|r| r.id)
    }

    /// Whether streams read the server, or did within the linger.
    pub(crate) fn has_readers(&self) -> bool {
        !self.streams.borrow().is_empty() || self.linger_until.get().is_some()
    }

    /// Sets the swarms streams read, and has the streams look.
    pub(crate) fn set_swarms(&self, swarms: Option<(Swarm<H>, Swarm<H>)>) {
        self.swarms.set(swarms);
        if swarms.is_some() {
            self.generation.set(self.generation.get() + 1);
        }
        self.wake_streams();
    }

    pub(crate) fn wake_streams(&self) {
        let wakers: Vec<Waker> = self
            .streams
            .borrow_mut()
            .values_mut()
            .filter_map(Option::take)
            .collect();
        for w in wakers {
            w.wake();
        }
    }

    /// Has the listing keeper look at the runner's state again.
    pub(crate) fn poke_keeper(&self) {
        self.keeper_poked.set(true);
        let w = self.keeper_waker.borrow_mut().take();
        if let Some(w) = w {
            w.wake();
        }
    }

    /// Has the listing keeper publish the listing again.
    pub(crate) fn request_republish(&self) {
        self.republish.set(true);
        self.poke_keeper();
    }

    /// The network id of the server this process serves.
    pub(crate) fn served_network(&self) -> Option<u64> {
        self.server.borrow().as_ref().map(|st| st.network_id)
    }
}

/// A server this process serves.
pub(crate) struct ServerState {
    pub(crate) server: Server,
    pub(crate) server_id: u64,
    pub(crate) network_id: u64,
    /// The server's vpn connection.
    pub(crate) client: VpnClient,
    /// Events the server task hasn't written. Bound: those of the sends
    /// since the task last ran, which the send rates and the connection's
    /// inbound call bound limit.
    pub(crate) events: std::collections::VecDeque<strims_proto::chat::v1::ServerEvent>,
    /// Profiles a moderation call hasn't stored. Bound: moderation calls
    /// store theirs in the transaction that read them, so these are at
    /// most those of one call that was dropped.
    pub(crate) profiles: Vec<strims_proto::chat::v1::Profile>,
    /// The connection ended.
    pub(crate) closed: bool,
}

/// A network's member port, which carries this node's calls to its
/// servers.
struct MemberPort<H: Host> {
    id: u64,
    net: Net<H>,
    client: Option<VpnClient>,
    abort: AbortHandle,
}

pub(crate) struct Shared<K: Kv, H: Host, T: Timer> {
    pub(crate) host: H,
    pub(crate) store: ReplicatedStore<K>,
    pub(crate) profile: Profile,
    pub(crate) network: NetworkControl<K, H, T>,
    pub(crate) transfers: TransferControl<H, T>,
    pub(crate) directory: DirectoryControl<K, H, T>,
    pub(crate) timer: T,
    pub(crate) config: Config,
    /// Draws lease tokens and the servers' rngs.
    pub(crate) rng: RefCell<DetRng>,
    /// Compiled when a server first parses a message.
    patterns: RefCell<Option<Patterns>>,
    /// Bound: [`Config::max_served`] runners with a record, and
    /// [`Config::max_rooms`] without, plus the records deleted while
    /// streams read them.
    runners: RefCell<BTreeMap<Vec<u8>, Rc<Runner<H>>>>,
    /// Stopped runners whose mode task hasn't ended. Bound: one per key of
    /// `runners`' bound.
    stopping: RefCell<BTreeMap<Vec<u8>, Rc<Runner<H>>>>,
    /// Bound: the profile's networks.
    members: RefCell<BTreeMap<NetKey, MemberPort<H>>>,
    /// When to open each network's member port again. Bound: as
    /// `members`.
    member_retry: RefCell<BTreeMap<NetKey, Time>>,
    next_member: Cell<u64>,
    pub(crate) events: Broadcast<ChatEvent>,
    pub(crate) metrics: RefCell<Metrics>,
    /// Tasks for `run` to start. Bound: a fixed number per runner and
    /// member port.
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

    pub(crate) fn emit(&self, e: ChatEvent) {
        self.events.emit(vec![e]);
    }

    pub(crate) fn runner(&self, key: &[u8]) -> Option<Rc<Runner<H>>> {
        self.runners.borrow().get(key).cloned()
    }

    fn runner_by_id(&self, server_id: u64) -> Option<Rc<Runner<H>>> {
        self.runners
            .borrow()
            .values()
            .find(|r| r.server_id() == Some(server_id))
            .cloned()
    }

    fn all_runners(&self) -> Vec<Rc<Runner<H>>> {
        self.runners.borrow().values().cloned().collect()
    }

    /// The patterns messages are parsed with, compiled on first use.
    pub(crate) fn patterns(&self) -> RefMut<'_, Patterns> {
        RefMut::map(self.patterns.borrow_mut(), |p| {
            p.get_or_insert_with(Patterns::new)
        })
    }

    /// The profile's peer key: the owner of the servers it hosts.
    pub(crate) fn owner_peer_key(&self) -> Vec<u8> {
        self.profile
            .key
            .as_ref()
            .map(|k| k.public.clone())
            .unwrap_or_default()
    }

    /// Applies `f` to the server `r` runs, queues its actions for the
    /// server task and wakes it. `None` while this process doesn't serve
    /// it.
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
                match a {
                    ServerAction::Event(e) => st.events.push_back(e),
                    ServerAction::SaveProfile(p) => st.profiles.push(p),
                }
            }
            res
        };
        r.wake();
        Some(res)
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

    // --- runners ------------------------------------------------------------

    fn start_runner(
        self: &Rc<Self>,
        key: Vec<u8>,
        network_key: NetKey,
        record: Option<strims_proto::chat::v1::Server>,
    ) -> Rc<Runner<H>> {
        let previous = self.stopping.borrow().get(&key).cloned();
        let r = Rc::new(Runner::new(key.clone(), network_key, record, previous));
        let prev = self.runners.borrow_mut().insert(key, r.clone());
        drop(prev);
        self.spawn(runner::run(self.clone(), r.clone()).boxed_local());
        r
    }

    /// Stops the runner of `key`. Its streams end.
    fn stop_runner(&self, key: &[u8]) {
        let Some(r) = self.runners.borrow_mut().remove(key) else {
            return;
        };
        r.stopped.set(true);
        if !r.done.get() {
            let prev = self.stopping.borrow_mut().insert(key.to_vec(), r.clone());
            drop(prev);
        }
        r.wake();
        r.wake_streams();
        r.poke_keeper();
    }

    /// Runners with a record.
    fn owned_count(&self) -> usize {
        self.runners
            .borrow()
            .values()
            .filter(|r| r.record.borrow().is_some())
            .count()
    }

    /// Runners without a record: rooms read for client streams.
    fn room_count(&self) -> usize {
        self.runners
            .borrow()
            .values()
            .filter(|r| r.record.borrow().is_none())
            .count()
    }

    /// Takes the profile's record of a server: starts its runner, moves it
    /// to the record's network, or hands it the changed record.
    fn sync_record(self: &Rc<Self>, record: strims_proto::chat::v1::Server) {
        let Some(key) = record.key.as_ref().map(|k| k.public.clone()) else {
            return;
        };
        let current = self.runner(&key);
        let owned = current
            .as_ref()
            .is_some_and(|r| r.record.borrow().is_some());
        if !owned && self.owned_count() >= self.config.max_served {
            self.metrics.borrow_mut().servers_dropped += 1;
            tracing::warn!(server = record.id, "too many chat servers to serve");
            return;
        }
        match current {
            Some(r) if r.network_key == record.network_key => {
                let prev = r.record.replace(Some(record));
                let changed = prev != *r.record.borrow();
                drop(prev);
                if changed {
                    self.schedule_assets(&r, false);
                    r.poke();
                    r.poke_keeper();
                }
            }
            Some(_) => {
                self.stop_runner(&key);
                let network_key = record.network_key.clone();
                self.start_runner(key, network_key, Some(record));
            }
            None => {
                let network_key = record.network_key.clone();
                self.start_runner(key, network_key, Some(record));
            }
        }
    }

    /// The profile no longer owns `r`'s server: its runner reads it while
    /// streams do, and stops otherwise.
    fn record_gone(&self, r: &Runner<H>) {
        let prev = r.record.replace(None);
        drop(prev);
        r.poke();
        if !r.has_readers() {
            self.stop_runner(&r.key);
        }
    }

    /// Reads every server record again, and syncs the runners with them.
    async fn load_servers(self: &Rc<Self>) {
        let res = async {
            let mut tx = self.store.read().await?;
            ChatServers::get_all(&mut tx).await
        }
        .await;
        let servers = match res {
            Ok(s) => s,
            Err(e) => {
                self.metrics.borrow_mut().store_errors += 1;
                tracing::warn!(error = %e, "reading the chat servers failed");
                return;
            }
        };
        let keys: BTreeSet<Vec<u8>> = servers
            .iter()
            .filter_map(|s| s.key.as_ref().map(|k| k.public.clone()))
            .collect();
        let gone: Vec<Rc<Runner<H>>> = self
            .all_runners()
            .into_iter()
            .filter(|r| r.record.borrow().is_some() && !keys.contains(&r.key))
            .collect();
        for r in gone {
            self.record_gone(&r);
        }
        for s in servers {
            self.sync_record(s);
        }
    }

    /// Schedules an asset sync of `r`'s server.
    fn schedule_assets(&self, r: &Runner<H>, unified: bool) {
        if unified {
            r.unified_due.set(true);
        }
        let due = self.timer.now() + self.config.asset_debounce;
        if r.assets_due.get().is_none_or(|t| t > due) {
            r.assets_due.set(Some(due));
        }
        r.poke();
    }

    fn assets_changed(&self, server_id: u64) {
        if let Some(r) = self.runner_by_id(server_id) {
            self.schedule_assets(&r, false);
        }
    }

    fn handle_store_event(self: &Rc<Self>, e: &StoreEvent) {
        match e {
            StoreEvent::Changed(Record::ChatServer(s)) => self.sync_record(s.clone()),
            StoreEvent::Deleted(Record::ChatServer(s)) => {
                let r = s
                    .key
                    .as_ref()
                    .and_then(|k| self.runner(&k.public))
                    .filter(|r| r.server_id() == Some(s.id));
                if let Some(r) = r {
                    self.record_gone(&r);
                }
            }
            StoreEvent::Changed(Record::ChatServerIcon(i))
            | StoreEvent::Deleted(Record::ChatServerIcon(i)) => self.assets_changed(i.id),
            StoreEvent::Changed(Record::ChatEmote(e))
            | StoreEvent::Deleted(Record::ChatEmote(e)) => self.assets_changed(e.server_id),
            StoreEvent::Changed(Record::ChatModifier(m))
            | StoreEvent::Deleted(Record::ChatModifier(m)) => self.assets_changed(m.server_id),
            StoreEvent::Changed(Record::ChatTag(t)) | StoreEvent::Deleted(Record::ChatTag(t)) => {
                self.assets_changed(t.server_id);
            }
            _ => {}
        }
    }

    /// Catches up after the store's events lagged: reads the records again
    /// and syncs every server's assets.
    async fn store_lagged(self: Rc<Self>) {
        self.load_servers().await;
        for r in self.all_runners() {
            if r.record.borrow().is_some() {
                self.schedule_assets(&r, false);
            }
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

    /// Follows the network control's state of the network `key`: its
    /// member port, its certificate, and the runners on it.
    fn sync_network(self: &Rc<Self>, key: &[u8]) {
        self.sync_member_port(key);
        let cert = self.network.host_certificate(key);
        for r in self.all_runners() {
            if r.network_key != key {
                continue;
            }
            let client = r.server.borrow().as_ref().map(|st| st.client.clone());
            if let (Some(cert), Some(client)) = (&cert, client)
                && let Err(e) = client.set_certificate(cert)
            {
                tracing::debug!(error = %e, "updating a chat server's certificate failed");
            }
            r.poke();
        }
    }

    fn sync_networks(self: &Rc<Self>) {
        let mut keys: BTreeSet<NetKey> = self
            .network
            .networks()
            .iter()
            .map(|i| network_key(&i.network))
            .collect();
        keys.extend(self.members.borrow().keys().cloned());
        keys.extend(
            self.runners
                .borrow()
                .values()
                .map(|r| r.network_key.clone()),
        );
        for k in keys {
            self.sync_network(&k);
        }
    }

    /// Opens the member port of the network `key` once it is joined and
    /// the profile's membership is trusted, and closes it otherwise.
    fn sync_member_port(self: &Rc<Self>, key: &[u8]) {
        let now = self.timer.now();
        let ready = self.network.network(key).and_then(|i| {
            let net = i.net?;
            let trusted = i
                .network
                .certificate
                .as_ref()
                .is_some_and(|c| membership_trusted(c, now));
            trusted.then_some(net)
        });
        let current = self.members.borrow().get(key).map(|m| m.net);
        match (ready, current) {
            (Some(net), Some(cur)) if net == cur => {
                let client = self
                    .members
                    .borrow()
                    .get(key)
                    .and_then(|m| m.client.clone());
                if let (Some(client), Some(cert)) = (client, self.network.host_certificate(key))
                    && let Err(e) = client.set_certificate(&cert)
                {
                    tracing::debug!(error = %e, "updating a chat member certificate failed");
                }
            }
            (ready, current) => {
                if current.is_some() {
                    self.close_member_port(key);
                }
                match ready {
                    Some(net) => {
                        let waiting = self
                            .member_retry
                            .borrow()
                            .get(key)
                            .is_some_and(|t| *t > now);
                        if !waiting {
                            self.open_member_port(key, net);
                        }
                    }
                    None => {
                        self.member_retry.borrow_mut().remove(key);
                    }
                }
            }
        }
    }

    fn close_member_port(&self, key: &[u8]) {
        let removed = self.members.borrow_mut().remove(key);
        if let Some(m) = &removed {
            m.abort.abort();
        }
        drop(removed);
    }

    fn open_member_port(self: &Rc<Self>, key: &[u8], net: Net<H>) {
        self.member_retry.borrow_mut().remove(key);
        let id = self.next_member.get();
        self.next_member.set(id + 1);
        let (task, abort) = abortable(member_port(self.clone(), key.to_vec(), net, id));
        let prev = self.members.borrow_mut().insert(
            key.to_vec(),
            MemberPort {
                id,
                net,
                client: None,
                abort,
            },
        );
        drop(prev);
        self.spawn(task.map(|_: Result<(), Aborted>| ()).boxed_local());
    }

    fn member_client(&self, key: &[u8]) -> Option<VpnClient> {
        self.members
            .borrow()
            .get(key)
            .and_then(|m| m.client.clone())
    }

    // --- the directory ------------------------------------------------------

    /// The runners serving on network `network_id`.
    fn served_on(&self, network_id: u64) -> Vec<Rc<Runner<H>>> {
        self.runners
            .borrow()
            .values()
            .filter(|r| r.served_network() == Some(network_id))
            .cloned()
            .collect()
    }

    fn handle_directory_event(&self, e: &DirectoryEvent) {
        match e {
            DirectoryEvent::Started { network_id, .. } => {
                for r in self.served_on(*network_id) {
                    r.request_republish();
                }
            }
            DirectoryEvent::Listing {
                network_id,
                event: ListingEvent::Unpublished(l),
            } => {
                for r in self.served_on(*network_id) {
                    if r.listing_id.get() == Some(l.id) {
                        r.request_republish();
                    }
                }
            }
            DirectoryEvent::User {
                network_id,
                listing_id,
                kind,
                user,
            } => {
                for r in self.served_on(*network_id) {
                    if r.listing_id.get() == Some(*listing_id) {
                        self.with_server(&r, |srv, _| srv.nick_changed(*kind, user));
                    }
                }
            }
            DirectoryEvent::Lagged { network_id } => self.directory_lagged(Some(*network_id)),
            _ => {}
        }
    }

    /// Reseeds the nicks of the servers on network `network_id`, or on
    /// every network, from the directory's views, and republishes the
    /// listings the views lack.
    fn directory_lagged(&self, network_id: Option<u64>) {
        for r in self.all_runners() {
            let Some(net) = r.served_network() else {
                continue;
            };
            if network_id.is_some_and(|id| id != net) {
                continue;
            }
            let Some(id) = r.listing_id.get() else {
                continue;
            };
            let users = self.directory.listing_users(net, id);
            self.with_server(&r, |srv, _| srv.reset_nicks(&users));
            if !runner::listed(self, &r, net, id) {
                r.request_republish();
            }
        }
    }

    // --- timers -------------------------------------------------------------

    /// When the next linger ends or member port retry is due.
    fn next_timer(&self) -> Option<Time> {
        let lingers = self
            .runners
            .borrow()
            .values()
            .filter_map(|r| r.linger_until.get())
            .min();
        let retries = self.member_retry.borrow().values().min().copied();
        lingers.into_iter().chain(retries).min()
    }

    /// Ends the lingers and opens the member ports that are due.
    fn tick(self: &Rc<Self>) {
        let now = self.timer.now();
        for r in self.all_runners() {
            if r.linger_until.get().is_some_and(|t| t <= now) && r.streams.borrow().is_empty() {
                r.linger_until.set(None);
                if r.record.borrow().is_none() {
                    self.stop_runner(&r.key);
                } else {
                    r.poke();
                }
            }
        }
        let due: Vec<NetKey> = self
            .member_retry
            .borrow()
            .iter()
            .filter(|(_, t)| **t <= now)
            .map(|(k, _)| k.clone())
            .collect();
        for k in due {
            self.member_retry.borrow_mut().remove(&k);
            self.sync_member_port(&k);
        }
    }

    // --- calls --------------------------------------------------------------

    /// Makes a call to the server with `server_key` on the network with
    /// `network_key`: this process's own, as its host certificate, or the
    /// published one, from the network's member port.
    async fn call(
        self: &Rc<Self>,
        network_key: &[u8],
        server_key: &[u8],
        req: Request,
    ) -> Result<Reply, ChatFailure> {
        if let Some(r) = self.runner(server_key)
            && r.serving()
        {
            if r.network_key != network_key {
                return Err(ChatFailure::OtherNetwork);
            }
            let caller = self
                .network
                .host_certificate(network_key)
                .as_ref()
                .and_then(Caller::from_certificate)
                .ok_or(ChatFailure::NotJoined)?;
            self.metrics.borrow_mut().local_calls += 1;
            return services::serve(self, &r, &caller, req).await;
        }
        let info = self
            .network
            .network(network_key)
            .ok_or(ChatFailure::NoNetwork)?;
        let net = info.net.ok_or(ChatFailure::NotJoined)?;
        let client = self
            .member_client(network_key)
            .ok_or(ChatFailure::NotJoined)?;
        let addr = self
            .host
            .resolve_address(net, server_key.to_vec(), ADDRESS_SALT.to_vec())
            .await
            .map_err(|e| ChatFailure::Host(e.to_string()))?
            .ok_or(ChatFailure::NoServer)?;
        let policy = match req {
            Request::SendMessage(_) => self.config.send_policy,
            _ => self.config.call_policy,
        };
        self.metrics.borrow_mut().remote_calls += 1;
        let retry = RetryClient::new(client.to(addr), self.timer.clone(), policy);
        services::call_remote(&ChatClient::new(retry), req).await
    }
}

/// Runs network `key`'s member port until its connection ends.
async fn member_port<K: Kv, H: Host, T: Timer>(
    s: Rc<Shared<K, H, T>>,
    key: NetKey,
    net: Net<H>,
    id: u64,
) {
    let opened = async {
        let cert = s
            .network
            .host_certificate(&key)
            .ok_or("no host certificate on the network")?;
        let (_, reader, writer) = s.host.open_vpn_port(net).await.map_err(|e| e.to_string())?;
        let (client, conn) = vpn::connect(s.config.member_vpn.clone(), &cert, reader, writer, None)
            .map_err(|e| e.to_string())?;
        if let Some(m) = s.members.borrow_mut().get_mut(&key).filter(|m| m.id == id) {
            m.client = Some(client);
        }
        Ok::<_, String>(conn)
    }
    .await;
    match opened {
        Ok(conn) => {
            if let Err(e) = conn.await {
                tracing::debug!(error = %e, "the chat member port failed");
            }
        }
        Err(e) => {
            s.metrics.borrow_mut().host_errors += 1;
            tracing::warn!(error = %e, "opening the chat member port failed");
        }
    }
    let removed = {
        let mut members = s.members.borrow_mut();
        match members.get(&key) {
            Some(m) if m.id == id => members.remove(&key),
            _ => None,
        }
    };
    if removed.is_some() {
        let at = s.timer.now() + MEMBER_RETRY;
        s.member_retry.borrow_mut().insert(key, at);
        s.wake_run();
    }
    drop(removed);
}

/// Runs the chat servers of a profile and the rooms its node reads; see
/// the [crate docs](crate).
///
/// Clones share the control.
pub struct ChatControl<K: Kv, H: Host, T: Timer> {
    pub(crate) shared: Rc<Shared<K, H, T>>,
}

impl<K: Kv, H: Host, T: Timer> Clone for ChatControl<K, H, T> {
    fn clone(&self) -> Self {
        ChatControl {
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
    Timer,
    Work,
}

impl<K: Kv, H: Host, T: Timer> ChatControl<K, H, T> {
    /// A control for the node behind `host`, serving the chat servers of
    /// `profile` that `store` holds on the networks `network` joins, with
    /// their swarms' transfers run by `transfers` and their listings
    /// published through `directory`. Timed on `timer`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: H,
        store: ReplicatedStore<K>,
        profile: Profile,
        network: NetworkControl<K, H, T>,
        transfers: TransferControl<H, T>,
        directory: DirectoryControl<K, H, T>,
        timer: T,
        rng: DetRng,
        config: Config,
    ) -> Self {
        ChatControl {
            shared: Rc::new(Shared {
                host,
                store,
                profile,
                network,
                transfers,
                directory,
                timer,
                config,
                rng: RefCell::new(rng),
                patterns: RefCell::new(None),
                runners: RefCell::new(BTreeMap::new()),
                stopping: RefCell::new(BTreeMap::new()),
                members: RefCell::new(BTreeMap::new()),
                member_retry: RefCell::new(BTreeMap::new()),
                next_member: Cell::new(0),
                events: Broadcast::new(),
                metrics: RefCell::default(),
                spawned: RefCell::new(Vec::new()),
                waker: RefCell::new(None),
            }),
        }
    }

    /// The network control whose networks this control follows.
    pub fn network(&self) -> &NetworkControl<K, H, T> {
        &self.shared.network
    }

    /// The directory control the servers' listings go through.
    pub fn directory(&self) -> &DirectoryControl<K, H, T> {
        &self.shared.directory
    }

    /// The profile's store.
    pub fn store(&self) -> &ReplicatedStore<K> {
        &self.shared.store
    }

    /// Changes to the servers this node serves and reads, from now on.
    pub fn subscribe(&self) -> Subscription<ChatEvent> {
        self.shared.events.subscribe()
    }

    /// The servers this node runs or reads, in key order.
    pub fn servers(&self) -> Vec<ChatServerStatus> {
        self.shared
            .runners
            .borrow()
            .values()
            .map(|r| ChatServerStatus {
                server_key: r.key.clone(),
                network_key: r.network_key.clone(),
                server_id: r.server_id(),
                mode: r.mode.get(),
                streams: r.streams.borrow().len(),
                listing_id: r.listing_id.get(),
                server: r
                    .server
                    .borrow()
                    .as_ref()
                    .map(|st| st.server.metrics().clone()),
            })
            .collect()
    }

    /// The control's counters.
    pub fn metrics(&self) -> Metrics {
        self.shared.metrics.borrow().clone()
    }

    /// Opens a client stream of the server with `server_key` on the
    /// network with `network_key`: its asset bundles and events, read from
    /// this process's own swarms when it serves the server, and from the
    /// network's otherwise. The stream waits while there are none (the
    /// network isn't joined yet, the mode is switching) and ends when the
    /// server's runner stops (its record moved to another network).
    /// Dropping it closes it; the room's swarms linger for
    /// [`Config::linger`] after its last stream.
    ///
    /// # Errors
    ///
    /// The node doesn't run the network, runs or reads the server on
    /// another network, or reads as many rooms or streams on the room as
    /// the [`Config`] bounds allow.
    pub fn open_client(
        &self,
        network_key: &[u8],
        server_key: &[u8],
    ) -> Result<ClientStream<K, H, T>, ChatFailure> {
        let s = &self.shared;
        if s.network.network(network_key).is_none() {
            return Err(ChatFailure::NoNetwork);
        }
        let r = match s.runner(server_key) {
            Some(r) if r.network_key != network_key => return Err(ChatFailure::OtherNetwork),
            Some(r) => r,
            None => {
                if s.room_count() >= s.config.max_rooms {
                    s.metrics.borrow_mut().streams_refused += 1;
                    return Err(ChatFailure::TooManyRooms);
                }
                s.start_runner(server_key.to_vec(), network_key.to_vec(), None)
            }
        };
        if r.streams.borrow().len() >= s.config.max_streams_per_room {
            s.metrics.borrow_mut().streams_refused += 1;
            return Err(ChatFailure::TooManyStreams);
        }
        let slot = r.next_stream.get();
        r.next_stream.set(slot + 1);
        r.streams.borrow_mut().insert(slot, None);
        r.linger_until.set(None);
        r.poke();
        Ok(ClientStream::new(s.clone(), r, slot))
    }

    /// Sends `body` to the server with `server_key` on the network with
    /// `network_key`, as this host. A remote call isn't retried: its one
    /// attempt may reach the server however it ends here.
    pub async fn send_message(
        &self,
        network_key: &[u8],
        server_key: &[u8],
        body: &str,
    ) -> Result<(), ChatFailure> {
        let req = Request::SendMessage(body.to_owned());
        self.shared.call(network_key, server_key, req).await?;
        Ok(())
    }

    /// Mutes the user with `peer_key` on the server for `duration_secs`
    /// from now, with `message`. The profile must moderate the server.
    pub async fn mute(
        &self,
        network_key: &[u8],
        server_key: &[u8],
        peer_key: &[u8],
        duration_secs: u32,
        message: &str,
    ) -> Result<(), ChatFailure> {
        let req = Request::Mute(MuteRequest {
            peer_key: peer_key.to_vec(),
            duration_secs,
            message: message.to_owned(),
        });
        self.shared.call(network_key, server_key, req).await?;
        Ok(())
    }

    /// Ends the mute of the user with `peer_key` on the server. The
    /// profile must moderate the server.
    pub async fn unmute(
        &self,
        network_key: &[u8],
        server_key: &[u8],
        peer_key: &[u8],
    ) -> Result<(), ChatFailure> {
        let req = Request::Unmute(UnmuteRequest {
            peer_key: peer_key.to_vec(),
        });
        self.shared.call(network_key, server_key, req).await?;
        Ok(())
    }

    /// The profile's own mute on the server: its end in unix seconds and
    /// its message, or 0 and no message.
    pub async fn get_mute(
        &self,
        network_key: &[u8],
        server_key: &[u8],
    ) -> Result<GetMuteResponse, ChatFailure> {
        match self
            .shared
            .call(network_key, server_key, Request::GetMute)
            .await?
        {
            Reply::Mute(m) => Ok(m),
            Reply::Done => Ok(GetMuteResponse::default()),
        }
    }

    /// Publishes the assets of the profile's server `server_id` after the
    /// debounce: what changed since the last bundle, or with `unified` a
    /// unified bundle that starts the asset stream over. A process that
    /// doesn't serve the server publishes them once it does.
    ///
    /// # Errors
    ///
    /// [`ChatFailure::NoServer`] when the profile has no server
    /// `server_id` this control runs.
    pub fn sync_assets(&self, server_id: u64, unified: bool) -> Result<(), ChatFailure> {
        let r = self
            .shared
            .runner_by_id(server_id)
            .ok_or(ChatFailure::NoServer)?;
        self.shared.schedule_assets(&r, unified);
        Ok(())
    }

    // --- running ------------------------------------------------------------

    /// Runs the chat servers: follows the store, the network control and
    /// the directory, serves or reads each server, and carries the
    /// streams' and calls' work. Never resolves; drop it to stop.
    pub async fn run(&self) {
        let s = &self.shared;
        let mut store_events = Some(s.store.events().subscribe());
        let mut network_events = Some(s.network.subscribe());
        let mut directory_events = Some(s.directory.subscribe());
        s.load_servers().await;
        s.sync_networks();
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
                Wake::Store(Some(Recv::Event(e))) => s.handle_store_event(&e),
                Wake::Store(Some(Recv::Lagged)) => s.spawn(s.clone().store_lagged().boxed_local()),
                Wake::Store(None) => store_events = None,
                Wake::Network(Some(Recv::Event(e))) => s.handle_network_event(&e),
                Wake::Network(Some(Recv::Lagged)) => s.sync_networks(),
                Wake::Network(None) => network_events = None,
                Wake::Directory(Some(Recv::Event(e))) => s.handle_directory_event(&e),
                Wake::Directory(Some(Recv::Lagged)) => s.directory_lagged(None),
                Wake::Directory(None) => directory_events = None,
                Wake::Timer => s.tick(),
                Wake::Task | Wake::Work => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_remote_chat_error_is_a_refusal() {
        let remote = |m: &str| CallError::Remote(RpcError::new(0, m));
        assert_eq!(
            ChatFailure::from_call(remote("operation requires moderator role")),
            ChatFailure::Refused(ChatError::NotModerator)
        );
        assert_eq!(
            ChatFailure::from_call(remote(
                "cannot send mesasges while muted. mute expires: 2026-10-17 12:00:00 +0000 UTC"
            )),
            ChatFailure::Refused(ChatError::Muted)
        );
        let other = RpcError::new(3, "updating profile failed: record not found");
        assert_eq!(
            ChatFailure::from_call(CallError::Remote(other.clone())),
            ChatFailure::Remote(other.clone())
        );
        assert_eq!(ChatFailure::Remote(other.clone()).to_rpc(), other);
        assert_eq!(
            ChatFailure::Refused(ChatError::TooFast).to_rpc(),
            RpcError::new(0, "sending too fast")
        );
        assert_eq!(
            ChatFailure::from_call(CallError::Timeout),
            ChatFailure::Call(CallError::Timeout)
        );
    }
}
