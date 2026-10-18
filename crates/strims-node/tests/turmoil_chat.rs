// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Chat on whole nodes, under turmoil. Every node runs a profile store and
//! the network, transfer, directory, chat and whisper controls, and serves
//! its peers' RPC links with the network and transfer controls.
//!
//! - o creates a network as "olly", serves its directory and invites m,
//!   who joins as "milly" and links to o. o's profile owns a chat server
//!   on the network, with the emote PEPE and the internal modifier rain,
//!   which procs on half the messages with emotes. o serves it, and its
//!   listing is published.
//! - o opens a client stream, read from the server's own swarms; m finds
//!   the listing in its view, opens a client stream and joins the
//!   listing. Both streams start with the unified asset bundle, and each
//!   yields every record once.
//! - o and m each send a message with PEPE that mentions the other: both
//!   streams carry both, with their emote and nick entities. o posts PEPE
//!   alone and m follows, a combo of two.
//! - o mutes m: m's send is refused as muted, and m's mute shows its end
//!   and message. o unmutes m, and m sends again.
//! - o adds the emote KEK, then disables it: both streams take a delta for
//!   each. A forced sync starts the asset stream over, and both streams
//!   take the unified bundle.
//! - m whispers o and o whispers back, each resolving the other's
//!   certificate by alias at the CA: the sender's whisper is DELIVERED,
//!   and the receiver's UNREAD in a thread with unread whispers.
//! - m parts the listing and closes its stream, and m's room swarms go
//!   after the linger, leaving m's cache of the asset swarm. o mentions m
//!   again, and the message has no nick.
//! - o deletes the chat server, which stops when its stream's linger
//!   ends. m opens the room again: with no server, its stream has the
//!   unified bundle from m's cache. o, which only served, cached nothing.
//!
//! Every host appends what it sees, stamped with simulated time, to one
//! trace. The same seed must give the same trace, and another seed a
//! different one. `STRIMS_TURMOIL_SEED=<s>` runs seed `s` in place of 7.

#![cfg(feature = "turmoil")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use futures_util::StreamExt;
use prost::Message as _;
use sha2::{Digest, Sha256};
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_chat::{
    ChatControl, ChatError, ChatEvent, ChatFailure, ClientEvent, ClientStream, ServerMode,
    WhisperConfig, WhisperControl,
};
use strims_crypto::{key_from_seed, membership_trusted};
use strims_dao::schema::chat::{
    ChatEmotes, ChatModifiers, ChatProfiles, ChatServers, WhisperRecords, WhisperThreads,
    profile_peer_key,
};
use strims_dao::schema::profile::Profile as ProfileRecord;
use strims_dao::seal::StorageKey;
use strims_dao::{ProfileStore, Recv, ReplicatedStore, Singleton, Table};
use strims_directory::{DirectoryControl, DirectoryEvent, ListingEvent};
use strims_kv::MemoryKv;
use strims_network::{NetworkControl, NetworkFrontend};
use strims_node::app::{NodeHost, NodeTimer, PeerServices};
use strims_node::{Clock, HostId, Node, NodeConfig, NodeEvent, SimClock, TurmoilNet};
use strims_proto::chat::v1::{
    AssetBundle, Emote, Message, MessageState, Modifier, Room, Server, WhisperRecord,
    WhisperThread, server_event,
};
use strims_proto::network::v1::ca::ca_find_request::Query as CertQuery;
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::directory::ListingQuery;
use strims_proto::network::v1::directory::listing::Content;
use strims_proto::network::v1::directory::listing_query::Query;
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateNetworkFromInvitationRequest, CreateServerRequest, Invitation,
    NetworkFrontendService,
};
use strims_proto::profile::v1::Profile;
use strims_proto::service::Context;
use strims_transfer::TransferControl;
use tokio::task::{JoinHandle, spawn_local};

const TCP_PORT: u16 = 7000;
/// How long o mutes m, in seconds.
const MUTE_SECS: u32 = 60;

fn epoch() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

type Trace = Rc<RefCell<Vec<String>>>;
type Timer = NodeTimer<SimClock>;
type Network = NetworkControl<MemoryKv, NodeHost, Timer>;
type Directory = DirectoryControl<MemoryKv, NodeHost, Timer>;
type Chat = ChatControl<MemoryKv, NodeHost, Timer>;
type Whispers = WhisperControl<MemoryKv, NodeHost, Timer>;
type Stream = ClientStream<MemoryKv, NodeHost, Timer>;

#[derive(Clone)]
struct World {
    seed: u64,
    trace: Trace,
    ids: Rc<RefCell<BTreeMap<String, HostId>>>,
    invitation: Rc<RefCell<Option<Invitation>>>,
    /// The names of the chat server's emotes and modifiers, by record id.
    assets: Rc<RefCell<BTreeMap<u64, String>>>,
    /// Steps done.
    done: Rc<RefCell<BTreeSet<String>>>,
}

impl World {
    fn new(seed: u64) -> World {
        World {
            seed,
            trace: Rc::default(),
            ids: Rc::default(),
            invitation: Rc::default(),
            assets: Rc::default(),
            done: Rc::default(),
        }
    }

    fn log(&self, clock: &SimClock, name: &str, line: impl Into<String>) {
        let t = (clock.now() - epoch()).as_millis();
        self.trace
            .borrow_mut()
            .push(format!("{t} {name} {}", line.into()));
    }

    fn profile(&self, name: &str) -> Profile {
        Profile {
            id: u64::from(name.as_bytes()[0]),
            name: name.to_owned(),
            key: Some(self.key(name)),
            device_id: 1,
            ..Default::default()
        }
    }

    fn key(&self, name: &str) -> strims_proto::types::Key {
        let mut s = [name.as_bytes()[0]; 32];
        s[..8].copy_from_slice(&self.seed.to_le_bytes());
        key_from_seed(&s)
    }

    fn uri(&self, name: &str) -> String {
        let ip = turmoil::lookup(name);
        format!(
            "tcp://{ip}:{TCP_PORT}/{}",
            hex::encode(&self.key(name).public)
        )
    }

    fn name(&self, id: &HostId) -> String {
        self.ids
            .borrow()
            .iter()
            .find(|(_, i)| *i == id)
            .map(|(n, _)| n.clone())
            .unwrap_or_default()
    }

    /// The name of asset record `id`.
    fn asset(&self, id: u64) -> String {
        self.assets
            .borrow()
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("#{id}"))
    }

    fn set(&self, step: &str) {
        self.done.borrow_mut().insert(step.to_owned());
    }

    /// Waits up to `secs` seconds for `step` to be done.
    async fn step(&self, step: &str, secs: u64) {
        self.wait(step, secs, || {
            self.done.borrow().contains(step).then_some(())
        })
        .await;
    }

    /// Polls `f` every 100 ms of simulated time until it has a value.
    async fn wait<T>(&self, what: &str, secs: u64, mut f: impl FnMut() -> Option<T>) -> T {
        self.wait_async(what, secs, async || f()).await
    }

    /// Polls `f` every 100 ms of simulated time until it has a value.
    async fn wait_async<T>(
        &self,
        what: &str,
        secs: u64,
        mut f: impl AsyncFnMut() -> Option<T>,
    ) -> T {
        let wait = async {
            loop {
                if let Some(v) = f().await {
                    return v;
                }
                tokio::time::sleep(StdDuration::from_millis(100)).await;
            }
        };
        tokio::time::timeout(StdDuration::from_secs(secs), wait)
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
    }
}

fn by_id(id: u64) -> ListingQuery {
    ListingQuery {
        query: Some(Query::Id(id)),
    }
}

/// A directory event, for the trace.
fn describe_directory(e: &DirectoryEvent) -> String {
    match e {
        DirectoryEvent::Started { .. } => "dir started".into(),
        DirectoryEvent::Stopped { .. } => "dir stopped".into(),
        DirectoryEvent::Listing { event, .. } => match event {
            ListingEvent::Changed(l) => {
                format!("dir listing {} changed users={}", l.id, l.user_count)
            }
            ListingEvent::Unpublished(l) => format!("dir listing {} unpublished", l.id),
            ListingEvent::UserCountChanged { id, user_count, .. } => {
                format!("dir listing {id} users={user_count}")
            }
        },
        DirectoryEvent::User {
            listing_id,
            kind,
            user,
            ..
        } => format!("dir user {kind:?} {listing_id} {}", user.alias),
        DirectoryEvent::AssetBundle { .. } => "dir assets".into(),
        DirectoryEvent::Lagged { .. } => "dir lagged".into(),
    }
}

/// A chat event, for the trace.
fn describe_chat(e: &ChatEvent) -> String {
    match e {
        ChatEvent::ServeStarted { server_id, .. } => format!("chat serves {server_id}"),
        ChatEvent::ServeStopped { server_id, .. } => format!("chat stopped serving {server_id}"),
        ChatEvent::ListingChanged { listing_id, .. } => format!("chat listing {listing_id}"),
        ChatEvent::ReadStarted { .. } => "chat reads".into(),
        ChatEvent::ReadStopped { .. } => "chat stopped reading".into(),
    }
}

fn emote_names(b: &AssetBundle) -> Vec<&str> {
    b.emotes.iter().map(|e| e.name.as_str()).collect()
}

fn modifier_names(b: &AssetBundle) -> Vec<&str> {
    b.modifiers.iter().map(|m| m.name.as_str()).collect()
}

/// An asset bundle, for the trace.
fn describe_bundle(w: &World, b: &AssetBundle) -> String {
    let removed: Vec<String> = b.removed_ids.iter().map(|id| w.asset(*id)).collect();
    format!(
        "assets {} room={} emotes={:?} modifiers={:?} removed={removed:?}",
        if b.is_delta { "delta" } else { "unified" },
        b.room.as_ref().map_or("-", |r| r.name.as_str()),
        emote_names(b),
        modifier_names(b),
    )
}

/// A message, for the trace: its emotes with their modifiers and combo
/// count, and its nicks.
fn describe_message(m: &Message) -> String {
    let e = m.entities.clone().unwrap_or_default();
    let emotes: Vec<String> = e
        .emotes
        .iter()
        .map(|e| {
            let mut s = e.name.clone();
            for m in &e.modifiers {
                s.push(':');
                s.push_str(m);
            }
            if e.combo > 0 {
                s.push_str(&format!(" x{}", e.combo));
            }
            s
        })
        .collect();
    let nicks: Vec<&str> = e.nicks.iter().map(|n| n.nick.as_str()).collect();
    format!(
        "message {}: {:?} emotes={emotes:?} nicks={nicks:?}",
        m.nick, m.body
    )
}

/// The emote names and combo counts, and the nicks and their peer keys,
/// of `m`'s entities.
#[allow(clippy::type_complexity)]
fn entities(m: &Message) -> (Vec<(String, u32)>, Vec<(String, Vec<u8>)>) {
    let e = m.entities.clone().unwrap_or_default();
    (
        e.emotes.into_iter().map(|e| (e.name, e.combo)).collect(),
        e.nicks.into_iter().map(|n| (n.nick, n.peer_key)).collect(),
    )
}

/// Asserts `m` is `nick`'s `body`, with these emotes and combo counts and
/// these nicks and peer keys.
fn check_message(
    m: &Message,
    nick: &str,
    body: &str,
    emotes: &[(&str, u32)],
    nicks: &[(&str, &[u8])],
) {
    assert_eq!((m.nick.as_str(), m.body.as_str()), (nick, body), "{m:?}");
    let (got_emotes, got_nicks) = entities(m);
    let emotes: Vec<(String, u32)> = emotes.iter().map(|(n, c)| ((*n).into(), *c)).collect();
    let nicks: Vec<(String, Vec<u8>)> = nicks
        .iter()
        .map(|(n, k)| ((*n).into(), k.to_vec()))
        .collect();
    assert_eq!((got_emotes, got_nicks), (emotes, nicks), "{m:?}");
}

fn outcome<T>(r: &Result<T, ChatFailure>) -> String {
    match r {
        Ok(_) => "ok".into(),
        Err(e) => format!("failed: {e}"),
    }
}

fn state(r: &WhisperRecord) -> MessageState {
    MessageState::try_from(r.state).unwrap()
}

/// What a client stream yielded so far.
#[derive(Default)]
struct Seen {
    bundles: Vec<AssetBundle>,
    messages: Vec<Message>,
}

/// A client stream, read by a task of its own that logs what it yields.
struct Reader {
    seen: Rc<RefCell<Seen>>,
    task: JoinHandle<()>,
}

impl Reader {
    fn new(h: &Started, mut stream: Stream) -> Reader {
        let seen: Rc<RefCell<Seen>> = Rc::default();
        let (w, clock, me, s) = (h.world.clone(), h.clock, h.name.clone(), seen.clone());
        let task = spawn_local(async move {
            while let Some(e) = stream.next().await {
                match e {
                    ClientEvent::Assets(b) => {
                        w.log(&clock, &me, describe_bundle(&w, &b));
                        s.borrow_mut().bundles.push(b);
                    }
                    ClientEvent::Events(events) => {
                        for e in events {
                            match e.body {
                                Some(server_event::Body::Message(m)) => {
                                    w.log(&clock, &me, describe_message(&m));
                                    s.borrow_mut().messages.push(m);
                                }
                                None => w.log(&clock, &me, "empty event"),
                            }
                        }
                    }
                }
            }
            w.log(&clock, &me, "stream ended");
        });
        Reader { seen, task }
    }

    /// The stream's `n`th asset bundle, from 0, once it comes.
    async fn bundle(&self, w: &World, n: usize) -> AssetBundle {
        w.wait(&format!("asset bundle {n}"), 60, || {
            self.seen.borrow().bundles.get(n).cloned()
        })
        .await
    }

    /// The stream's `n`th message, from 0, once it comes.
    async fn message(&self, w: &World, n: usize) -> Message {
        w.wait(&format!("message {n}"), 60, || {
            self.seen.borrow().messages.get(n).cloned()
        })
        .await
    }

    /// Closes the stream.
    fn close(self) {
        self.task.abort();
    }
}

struct Started {
    world: World,
    name: String,
    node: Node,
    clock: SimClock,
    network: Network,
    frontend: NetworkFrontend<MemoryKv, NodeHost, Timer>,
    directory: Directory,
    chat: Chat,
    whispers: Whispers,
    store: ReplicatedStore<MemoryKv>,
    /// Peers bound to the network, by name.
    bound: Rc<RefCell<BTreeMap<String, bool>>>,
    /// The chat control's events so far.
    chat_events: Rc<RefCell<Vec<ChatEvent>>>,
}

async fn start(w: &World, name: &str, index: u64) -> Started {
    let ip: IpAddr = turmoil::lookup(name);
    let profile = w.profile(name);
    let mut config = NodeConfig::new(profile.key.clone().unwrap(), w.seed.wrapping_add(index));
    config.label = name.to_owned();
    config.tcp_listen = Some(format!("0.0.0.0:{TCP_PORT}"));
    config.public_ip = Some(ip);
    config.max_peers = 16;
    let clock = SimClock::turmoil(epoch());
    let (node, mut events) = Node::spawn(config, TurmoilNet, clock).await.unwrap();
    w.ids.borrow_mut().insert(name.to_owned(), node.host_id());

    let store = ProfileStore::create(
        MemoryKv::new(),
        profile.id,
        StorageKey::from_bytes([index as u8; 32]),
        det_rng(w.seed ^ index),
        clock.now(),
    )
    .await
    .unwrap();
    let mut tx = store.write(clock.now()).await.unwrap();
    ProfileRecord::set(&mut tx, profile.clone()).await.unwrap();
    tx.commit().await.unwrap();
    let store = ReplicatedStore::new(store).await.unwrap();

    let host = NodeHost::new(node.clone());
    let network = NetworkControl::new(
        host.clone(),
        store.clone(),
        profile.clone(),
        NodeTimer(clock),
        det_rng(w.seed.wrapping_mul(31) ^ index),
        strims_network::Config::default(),
    );
    let transfers = TransferControl::new(
        host.clone(),
        NodeTimer(clock),
        strims_transfer::Config {
            search_interval: Duration::from_secs(5),
            ..strims_transfer::Config::default()
        },
    );
    let directory = DirectoryControl::new(
        host.clone(),
        store.clone(),
        network.clone(),
        transfers.clone(),
        NodeTimer(clock),
        det_rng(w.seed.wrapping_mul(37) ^ index),
        strims_directory::Config::default(),
    );
    let chat = ChatControl::new(
        host.clone(),
        store.clone(),
        profile.clone(),
        network.clone(),
        transfers.clone(),
        directory.clone(),
        NodeTimer(clock),
        det_rng(w.seed.wrapping_mul(41) ^ index),
        strims_chat::Config::default(),
    );
    let whispers = WhisperControl::new(
        host,
        store.clone(),
        profile,
        network.clone(),
        NodeTimer(clock),
        det_rng(w.seed.wrapping_mul(43) ^ index),
        WhisperConfig::default(),
    );
    spawn_local({
        let network = network.clone();
        async move { network.run().await }
    });
    spawn_local({
        let transfers = transfers.clone();
        async move { transfers.run().await }
    });
    let mut dir_events = directory.subscribe();
    spawn_local({
        let directory = directory.clone();
        async move { directory.run().await }
    });
    let mut events_of_chat = chat.subscribe();
    spawn_local({
        let chat = chat.clone();
        async move { chat.run().await }
    });
    spawn_local({
        let whispers = whispers.clone();
        async move { whispers.run().await }
    });
    let (w2, me) = (w.clone(), name.to_owned());
    spawn_local(async move {
        while let Some(e) = dir_events.next().await {
            let line = match e {
                Recv::Event(e) => describe_directory(&e),
                Recv::Lagged => "dir events lagged".into(),
            };
            w2.log(&clock, &me, line);
        }
    });
    let chat_events: Rc<RefCell<Vec<ChatEvent>>> = Rc::default();
    let (w2, me, seen) = (w.clone(), name.to_owned(), chat_events.clone());
    spawn_local(async move {
        while let Some(e) = events_of_chat.next().await {
            match e {
                Recv::Event(e) => {
                    w2.log(&clock, &me, describe_chat(&e));
                    seen.borrow_mut().push(ChatEvent::clone(&e));
                }
                Recv::Lagged => w2.log(&clock, &me, "chat events lagged"),
            }
        }
    });

    let services = PeerServices::new(node.clone())
        .with(Rc::new(network.clone()))
        .with(Rc::new(transfers.clone()));
    let bound: Rc<RefCell<BTreeMap<String, bool>>> = Rc::default();
    let (w2, me, b2) = (w.clone(), name.to_owned(), bound.clone());
    spawn_local(async move {
        while let Some(e) = events.recv().await {
            match &e {
                NodeEvent::PeerAdded(p) => {
                    w2.log(&clock, &me, format!("peer+ {}", w2.name(&p.host_id)))
                }
                NodeEvent::NetworkPeerAdded { host_id, .. } => {
                    let peer = w2.name(host_id);
                    w2.log(&clock, &me, format!("net+ {peer}"));
                    b2.borrow_mut().insert(peer, true);
                }
                NodeEvent::NetworkPeerRemoved { host_id, .. } => {
                    let peer = w2.name(host_id);
                    w2.log(&clock, &me, format!("net- {peer}"));
                    b2.borrow_mut().insert(peer, false);
                }
                NodeEvent::TransferError { host_id, error } => {
                    w2.log(
                        &clock,
                        &me,
                        format!("transfer error {} {error}", w2.name(host_id)),
                    );
                }
                _ => {}
            }
            services.handle_event(&e).await;
        }
    });
    let frontend = NetworkFrontend::new(network.clone());
    Started {
        world: w.clone(),
        name: name.to_owned(),
        node,
        clock,
        network,
        frontend,
        directory,
        chat,
        whispers,
        store,
        bound,
        chat_events,
    }
}

impl Started {
    fn log(&self, line: impl Into<String>) {
        self.world.log(&self.clock, &self.name, line);
    }

    fn is_bound(&self, peer: &str) -> bool {
        self.bound.borrow().get(peer).copied().unwrap_or(false)
    }

    /// This side's id and key of its one network.
    fn network(&self) -> (u64, Vec<u8>) {
        let n = self.network.networks()[0].network.clone();
        let key = strims_dao::schema::network::network_key(&n);
        (n.id, key)
    }

    fn trusted(&self) -> bool {
        let now = self.clock.now();
        self.network.networks().first().is_some_and(|n| {
            n.network
                .certificate
                .as_ref()
                .is_some_and(|c| membership_trusted(c, now))
        })
    }

    /// Whether the user with `peer_key` is in listing `id` of this side's
    /// view, with this alias.
    fn present(&self, id: u64, peer_key: &[u8], alias: &str) -> bool {
        let (network_id, _) = self.network();
        self.directory
            .listing_users(network_id, id)
            .iter()
            .any(|u| u.peer_key == peer_key && u.alias == alias)
    }

    /// Writes a chat server of the profile on the network with key `net`,
    /// with the emote PEPE and the internal modifier rain, as the frontend
    /// would.
    async fn create_chat(&self, net: &[u8]) -> Server {
        let mut tx = self.store.write(self.clock.now()).await.unwrap();
        let id = tx.generate_id().await.unwrap();
        let server = Server {
            id,
            network_key: net.to_vec(),
            key: Some(self.world.key("chat")),
            room: Some(Room {
                name: "lobby".into(),
                css: String::new(),
            }),
            admin_peer_keys: Vec::new(),
        };
        let server = ChatServers::insert(&mut tx, server).await.unwrap();
        let emote = Emote {
            id: tx.generate_id().await.unwrap(),
            server_id: id,
            name: "PEPE".into(),
            enable: true,
            ..Default::default()
        };
        let emote = ChatEmotes::insert(&mut tx, emote).await.unwrap();
        let rain = Modifier {
            id: tx.generate_id().await.unwrap(),
            server_id: id,
            name: "rain".into(),
            internal: true,
            proc_chance: 0.5,
            ..Default::default()
        };
        let rain = ChatModifiers::insert(&mut tx, rain).await.unwrap();
        tx.commit().await.unwrap();
        self.world
            .assets
            .borrow_mut()
            .extend([(emote.id, emote.name), (rain.id, rain.name)]);
        server
    }

    /// Adds the enabled emote `name` to server `server_id`.
    async fn add_emote(&self, server_id: u64, name: &str) -> Emote {
        let mut tx = self.store.write(self.clock.now()).await.unwrap();
        let emote = Emote {
            id: tx.generate_id().await.unwrap(),
            server_id,
            name: name.into(),
            enable: true,
            ..Default::default()
        };
        let emote = ChatEmotes::insert(&mut tx, emote).await.unwrap();
        tx.commit().await.unwrap();
        self.world
            .assets
            .borrow_mut()
            .insert(emote.id, emote.name.clone());
        emote
    }

    async fn update_emote(&self, emote: Emote) {
        let mut tx = self.store.write(self.clock.now()).await.unwrap();
        ChatEmotes::update(&mut tx, emote).await.unwrap();
        tx.commit().await.unwrap();
    }

    /// The deadline, relative to the epoch in seconds, and the mute
    /// messages of the chat profile of the user with `peer_key` on server
    /// `server_id`.
    async fn chat_profile(&self, server_id: u64, peer_key: &[u8]) -> (i64, Vec<String>) {
        let mut tx = self.store.read().await.unwrap();
        let key = profile_peer_key(server_id, peer_key);
        let p = ChatProfiles::index_get(&mut tx, &ChatProfiles::BY_PEER_KEY, &key)
            .await
            .unwrap()
            .unwrap();
        (
            p.mute_deadline - epoch().unix(),
            p.mutes.into_iter().map(|m| m.message).collect(),
        )
    }

    /// Sends `body` to the chat server with `server_key` on the network
    /// with key `net`, and logs the outcome.
    async fn send(&self, net: &[u8], server_key: &[u8], body: &str) -> Result<(), ChatFailure> {
        let res = self.chat.send_message(net, server_key, body).await;
        self.log(format!("send {body:?}: {}", outcome(&res)));
        res
    }

    /// Resolves the certificate of `alias` on the network with key `net`
    /// at its CA, and whispers `body` to it from the chat server with
    /// `server_key`. Resolves to the record once it's DELIVERED.
    async fn whisper(
        &self,
        net: &[u8],
        server_key: &[u8],
        alias: &str,
        body: &str,
    ) -> WhisperRecord {
        let cert = self
            .network
            .find_certificate(net, CertQuery::Subject(alias.into()), false)
            .await
            .unwrap();
        let r = self
            .whispers
            .enqueue(net, server_key, &cert.key, &cert.subject, body)
            .await
            .unwrap();
        assert_eq!(state(&r), MessageState::Enqueued);
        self.log(format!("whispers {alias} {body:?}"));
        let id = r.id;
        let r = self
            .world
            .wait_async("the whisper's delivery", 120, async || {
                let r = self.whisper_record(id).await;
                (state(&r) != MessageState::Enqueued).then_some(r)
            })
            .await;
        self.log(format!("whisper to {alias}: {:?}", state(&r)));
        assert_eq!(state(&r), MessageState::Delivered);
        r
    }

    async fn whisper_record(&self, id: u64) -> WhisperRecord {
        let mut tx = self.store.read().await.unwrap();
        WhisperRecords::get(&mut tx, id).await.unwrap().unwrap()
    }

    /// The whisper the profile with `peer_key` sent, and the thread it's
    /// in, once one is stored.
    async fn whisper_from(&self, peer_key: &[u8]) -> (WhisperRecord, WhisperThread) {
        let r = self
            .world
            .wait_async("a whisper", 120, async || {
                let mut tx = self.store.read().await.unwrap();
                let rs = WhisperRecords::get_all(&mut tx).await.unwrap();
                rs.into_iter().find(|r| {
                    r.peer_key == peer_key
                        && r.message.as_ref().is_some_and(|m| m.peer_key == peer_key)
                })
            })
            .await;
        let mut tx = self.store.read().await.unwrap();
        let t = WhisperThreads::index_get(&mut tx, &WhisperThreads::BY_PEER_KEY, peer_key)
            .await
            .unwrap()
            .unwrap();
        let m = r.message.as_ref().unwrap();
        self.log(format!(
            "whisper from {}: {:?} {:?}; thread {} unread={}",
            m.nick,
            m.body,
            state(&r),
            t.alias,
            t.has_unread
        ));
        (r, t)
    }
}

async fn owner(w: World) -> turmoil::Result {
    let h = start(&w, "o", 1).await;
    let created = h
        .frontend
        .create_server(
            &Context::default(),
            CreateServerRequest {
                name: "net".into(),
                alias: "olly".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .network
        .unwrap();
    w.wait("o to serve the directory", 60, || {
        h.directory
            .networks()
            .first()
            .is_some_and(|n| n.serving)
            .then_some(())
    })
    .await;
    h.log("serves the directory");
    let invitation = h
        .frontend
        .create_invitation(
            &Context::default(),
            CreateInvitationRequest {
                network_id: created.id,
                bootstrap_client_id: 0,
            },
        )
        .await
        .unwrap()
        .invitation
        .unwrap();
    *w.invitation.borrow_mut() = Some(invitation);
    let (_, net) = h.network();
    let (o_key, m_key) = (w.key("o").public, w.key("m").public);

    let server = h.create_chat(&net).await;
    let skey = server.key.as_ref().unwrap().public.clone();
    let listing_id = w
        .wait("o to serve the chat and publish its listing", 60, || {
            let s = h.chat.servers().into_iter().next()?;
            (s.mode == ServerMode::Serving).then_some(s.listing_id?)
        })
        .await;
    let stream = Reader::new(&h, h.chat.open_client(&net, &skey).unwrap());
    let b = stream.bundle(&w, 0).await;
    assert!(!b.is_delta, "{b:?}");
    assert_eq!(b.room.as_ref().unwrap().name, "lobby");
    assert_eq!(
        (emote_names(&b), modifier_names(&b)),
        (vec!["PEPE"], vec!["rain"])
    );

    // messages both ways, with emotes and nicks; then a combo
    w.step("m joined", 300).await;
    w.wait("milly in o's view of the chat", 60, || {
        h.present(listing_id, &m_key, "milly").then_some(())
    })
    .await;
    tokio::time::sleep(StdDuration::from_secs(1)).await;
    h.send(&net, &skey, "hi milly PEPE").await.unwrap();
    let m = stream.message(&w, 0).await;
    check_message(
        &m,
        "olly",
        "hi milly PEPE",
        &[("PEPE", 0)],
        &[("milly", &m_key)],
    );
    let m = stream.message(&w, 1).await;
    check_message(
        &m,
        "milly",
        "hey olly PEPE",
        &[("PEPE", 0)],
        &[("olly", &o_key)],
    );
    h.send(&net, &skey, "PEPE").await.unwrap();
    let m = stream.message(&w, 2).await;
    check_message(&m, "olly", "PEPE", &[("PEPE", 0)], &[]);
    assert!(m.entities.unwrap().emotes[0].can_combo);
    let m = stream.message(&w, 3).await;
    check_message(&m, "milly", "PEPE", &[("PEPE", 2)], &[]);

    // a mute, then an unmute
    let res = h.chat.mute(&net, &skey, &m_key, MUTE_SECS, "cool it").await;
    h.log(format!("mutes milly: {}", outcome(&res)));
    res.unwrap();
    let (deadline, mutes) = h.chat_profile(server.id, &m_key).await;
    h.log(format!(
        "milly's chat profile: deadline={deadline} mutes={mutes:?}"
    ));
    assert_eq!(mutes, ["cool it"]);
    w.set("muted");
    w.step("m saw its mute", 60).await;
    let res = h.chat.unmute(&net, &skey, &m_key).await;
    h.log(format!("unmutes milly: {}", outcome(&res)));
    res.unwrap();
    let (deadline, _) = h.chat_profile(server.id, &m_key).await;
    h.log(format!("milly's chat profile: deadline={deadline}"));
    assert!(deadline <= (h.clock.now() - epoch()).as_secs());
    w.set("unmuted");
    let m = stream.message(&w, 4).await;
    check_message(&m, "milly", "back", &[], &[]);

    // an emote added and disabled, then a forced unified bundle
    let kek = h.add_emote(server.id, "KEK").await;
    let b = stream.bundle(&w, 1).await;
    assert!(b.is_delta && b.removed_ids.is_empty(), "{b:?}");
    assert_eq!(emote_names(&b), ["KEK"]);
    w.step("m saw KEK", 60).await;
    h.update_emote(Emote {
        enable: false,
        ..kek.clone()
    })
    .await;
    let b = stream.bundle(&w, 2).await;
    assert!(b.is_delta && b.emotes.is_empty(), "{b:?}");
    assert_eq!(b.removed_ids, [kek.id]);
    w.step("m saw KEK go", 60).await;
    h.chat.sync_assets(server.id, true).unwrap();
    let b = stream.bundle(&w, 3).await;
    assert!(!b.is_delta, "{b:?}");
    assert_eq!(
        (emote_names(&b), modifier_names(&b)),
        (vec!["PEPE"], vec!["rain"])
    );
    assert_eq!(h.chat.metrics().asset_resets, 1);
    w.step("m saw the unified bundle", 60).await;

    // whispers both ways
    let (r, t) = h.whisper_from(&m_key).await;
    assert_eq!(state(&r), MessageState::Unread);
    assert_eq!(r.message.as_ref().unwrap().body, "psst olly");
    assert_eq!((t.alias.as_str(), t.has_unread), ("milly", true));
    let r = h.whisper(&net, &skey, "milly", "psst milly").await;
    assert_eq!(r.peer_key, m_key);

    // m parts: its nick goes
    w.step("m parted", 120).await;
    w.wait("milly to go from o's view of the chat", 60, || {
        (!h.present(listing_id, &m_key, "milly")).then_some(())
    })
    .await;
    tokio::time::sleep(StdDuration::from_secs(1)).await;
    h.send(&net, &skey, "bye milly").await.unwrap();
    let m = stream.message(&w, 5).await;
    check_message(&m, "olly", "bye milly", &[], &[]);

    let s = h.chat.servers()[0].server.clone().unwrap();
    h.log(format!(
        "server: messages={} combo messages={} refused muted={}",
        s.messages, s.combo_messages, s.refused_muted
    ));
    assert_eq!(
        (s.messages, s.combo_messages, s.refused_muted),
        (6, 1, 1),
        "{s:?}"
    );
    let cache_writes = h.chat.metrics().cache_writes;
    assert_eq!(cache_writes, 0, "a served room is cached");

    // the server goes
    w.step("m stopped reading", 120).await;
    stream.close();
    let mut tx = h.store.write(h.clock.now()).await.unwrap();
    assert!(ChatServers::delete(&mut tx, server.id).await.unwrap());
    tx.commit().await.unwrap();
    h.log("deleted the chat server");
    w.wait("o to stop running the chat", 120, || {
        h.chat.servers().is_empty().then_some(())
    })
    .await;
    h.log("stopped running the chat");
    w.set("server gone");
    w.step("m done", 120).await;
    w.set("o done");
    Ok(())
}

async fn member(w: World) -> turmoil::Result {
    let h = start(&w, "m", 2).await;
    let invitation = w
        .wait("an invitation", 60, || w.invitation.borrow().clone())
        .await;
    h.frontend
        .create_network_from_invitation(
            &Context::default(),
            CreateNetworkFromInvitationRequest {
                alias: "milly".into(),
                invitation: Some(InvitationField::InvitationBytes(invitation.encode_to_vec())),
            },
        )
        .await
        .unwrap();
    h.node.dial(&w.uri("o")).await.unwrap();
    w.wait("m to bind with o", 120, || {
        (h.is_bound("o") && h.trusted()).then_some(())
    })
    .await;
    h.log("renewed and bound with o");
    let (network_id, net) = h.network();
    let (o_key, m_key) = (w.key("o").public, w.key("m").public);

    let (listing_id, skey) = w
        .wait("the chat in m's view", 120, || {
            h.directory
                .listings(network_id)
                .into_iter()
                .find_map(|l| match l.listing.content {
                    Some(Content::Chat(c)) => Some((l.id, c.key)),
                    _ => None,
                })
        })
        .await;
    h.log(format!("found the chat as {listing_id}"));
    let stream = Reader::new(&h, h.chat.open_client(&net, &skey).unwrap());
    // calls fail until the member port opens and the address resolves
    loop {
        match h.directory.join(&net, by_id(listing_id)).await {
            Ok(id) => {
                assert_eq!(id, listing_id);
                break;
            }
            Err(e) => {
                assert!(
                    h.clock.now() - epoch() < Duration::from_secs(300),
                    "joining never worked: {e}"
                );
                tokio::time::sleep(StdDuration::from_secs(1)).await;
            }
        }
    }
    h.log("joined the chat");
    let b = stream.bundle(&w, 0).await;
    assert!(!b.is_delta, "{b:?}");
    assert_eq!(b.room.as_ref().unwrap().name, "lobby");
    assert_eq!(
        (emote_names(&b), modifier_names(&b)),
        (vec!["PEPE"], vec!["rain"])
    );
    w.set("m joined");

    let m = stream.message(&w, 0).await;
    check_message(
        &m,
        "olly",
        "hi milly PEPE",
        &[("PEPE", 0)],
        &[("milly", &m_key)],
    );
    h.send(&net, &skey, "hey olly PEPE").await.unwrap();
    let m = stream.message(&w, 1).await;
    check_message(
        &m,
        "milly",
        "hey olly PEPE",
        &[("PEPE", 0)],
        &[("olly", &o_key)],
    );
    let m = stream.message(&w, 2).await;
    check_message(&m, "olly", "PEPE", &[("PEPE", 0)], &[]);
    h.send(&net, &skey, "PEPE").await.unwrap();
    let m = stream.message(&w, 3).await;
    check_message(&m, "milly", "PEPE", &[("PEPE", 2)], &[]);

    w.step("muted", 60).await;
    assert_eq!(
        h.send(&net, &skey, "am I muted").await,
        Err(ChatFailure::Refused(ChatError::Muted))
    );
    let mute = h.chat.get_mute(&net, &skey).await.unwrap();
    let end = mute.end_time - epoch().unix();
    h.log(format!("muted until {end}: {:?}", mute.message));
    let now = (h.clock.now() - epoch()).as_secs();
    assert!(now < end && end <= now + i64::from(MUTE_SECS), "{mute:?}");
    assert_eq!(mute.message, "cool it");
    w.set("m saw its mute");
    w.step("unmuted", 60).await;
    let mute = h.chat.get_mute(&net, &skey).await.unwrap();
    h.log(format!("muted until {}", mute.end_time));
    assert_eq!((mute.end_time, mute.message.as_str()), (0, ""));
    h.send(&net, &skey, "back").await.unwrap();
    let m = stream.message(&w, 4).await;
    check_message(&m, "milly", "back", &[], &[]);

    let b = stream.bundle(&w, 1).await;
    assert!(b.is_delta && b.removed_ids.is_empty(), "{b:?}");
    assert_eq!(emote_names(&b), ["KEK"]);
    w.set("m saw KEK");
    let b = stream.bundle(&w, 2).await;
    assert!(b.is_delta && b.emotes.is_empty(), "{b:?}");
    assert_eq!(b.removed_ids.len(), 1);
    assert_eq!(w.asset(b.removed_ids[0]), "KEK");
    w.set("m saw KEK go");
    let b = stream.bundle(&w, 3).await;
    assert!(!b.is_delta, "{b:?}");
    assert_eq!(
        (emote_names(&b), modifier_names(&b)),
        (vec!["PEPE"], vec!["rain"])
    );
    w.set("m saw the unified bundle");

    let r = h.whisper(&net, &skey, "olly", "psst olly").await;
    assert_eq!(r.peer_key, o_key);
    let (r, t) = h.whisper_from(&o_key).await;
    assert_eq!(state(&r), MessageState::Unread);
    assert_eq!(r.message.as_ref().unwrap().body, "psst milly");
    assert_eq!((t.alias.as_str(), t.has_unread), ("olly", true));

    h.directory.part(&net, listing_id).await.unwrap();
    stream.close();
    h.log("parted the chat and closed its stream");
    w.set("m parted");
    w.wait("m to stop reading the chat", 120, || {
        h.chat_events
            .borrow()
            .iter()
            .any(|e| matches!(e, ChatEvent::ReadStopped { .. }))
            .then_some(())
    })
    .await;
    assert!(h.chat.servers().is_empty());
    let metrics = h.chat.metrics();
    h.log(format!("cache writes={}", metrics.cache_writes));
    assert!(metrics.cache_writes > 0, "{metrics:?}");
    w.set("m stopped reading");

    // with the server gone, the room's bundle comes from m's cache
    w.step("server gone", 180).await;
    let stream = Reader::new(&h, h.chat.open_client(&net, &skey).unwrap());
    let b = stream.bundle(&w, 0).await;
    assert!(!b.is_delta, "{b:?}");
    assert_eq!(
        (emote_names(&b), modifier_names(&b)),
        (vec!["PEPE"], vec!["rain"])
    );
    let metrics = h.chat.metrics();
    assert_eq!(
        (metrics.cache_loads, metrics.cache_refusals),
        (1, 0),
        "{metrics:?}"
    );
    stream.close();
    w.set("m done");
    w.step("o done", 120).await;
    Ok(())
}

fn run(seed: u64) -> Vec<String> {
    let w = World::new(seed);
    let mut sim = turmoil::Builder::new()
        .rng_seed(seed)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_secs(1_700_000_000))
        .simulation_duration(StdDuration::from_secs(900))
        .min_message_latency(StdDuration::from_millis(1))
        .max_message_latency(StdDuration::from_millis(15))
        .build();
    sim.client("o", owner(w.clone()));
    sim.client("m", member(w.clone()));
    let res = sim.run();
    let trace = w.trace.borrow().clone();
    if let Err(e) = res {
        for l in &trace {
            eprintln!("{l}");
        }
        panic!("seed {seed}: {e}\n{}", replay(seed));
    }
    trace
}

fn digest(trace: &[String]) -> String {
    let mut h = Sha256::new();
    for l in trace {
        h.update(l.as_bytes());
        h.update(b"\n");
    }
    hex::encode(h.finalize())
}

fn replay(seed: u64) -> String {
    format!(
        "replay: STRIMS_TURMOIL_SEED={seed} cargo test -p strims-node --features turmoil \
         --test turmoil_chat members_chat_and_whisper_on_whole_nodes -- --exact --nocapture"
    )
}

#[test]
fn members_chat_and_whisper_on_whole_nodes() {
    // a seed to replay, from the replay command
    #[allow(clippy::disallowed_methods)]
    let seed = std::env::var("STRIMS_TURMOIL_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);
    let a = run(seed);
    for l in &a {
        eprintln!("{l}");
    }
    eprintln!("seed {seed}: {} lines, {}", a.len(), digest(&a));
    let b = run(seed);
    assert_eq!(
        digest(&a),
        digest(&b),
        "same seed, different traces\n{}",
        replay(seed)
    );
    assert_ne!(
        digest(&a),
        digest(&run(seed + 1)),
        "different seeds, same trace\n{}",
        replay(seed)
    );
}
