// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Chat between a Go and a Rust node over a direct link: Go's
//! internal/chat on the helper (`-chat`, the `chat-*` verbs and events),
//! and `strims_chat`'s chat and whisper controls on a Rust node composed
//! as in [`directory`](crate::directory), beside its network, transfer and
//! directory controls. Servers are records in their owner's store; the
//! other side reads them from their event and asset swarms and calls them
//! over vpn.
//!
//! - `go_server_rust_client`: Go's server, with a room, an icon, an
//!   emote, a modifier and a tag, publishes its chat listing, which Rust's
//!   directory view shows. Rust's client stream starts with a unified
//!   bundle, folds to Go's records, and carries Go's earlier message. Rust
//!   sends a message with an emote, its modifier and a tag; Go's server
//!   extracts the entities, and Go's read and Rust's stream show the same
//!   message with Rust's key and alias. Emotes added and disabled on Go
//!   reach Rust as deltas, and Rust's stream follows a unified bundle that
//!   restarts Go's asset stream. Rust's mute is refused until Rust's key
//!   is in the server's admin keys; then Rust mutes Go, whose send is
//!   refused, and unmutes it.
//! - `rust_server_go_client`: the reverse. Go's directory view shows
//!   Rust's chat listing; Go's read starts with a unified bundle, folds to
//!   Rust's records and carries Rust's earlier message. Go's message
//!   reaches both reads with Go's key, alias and entities. The owner mutes
//!   Go: Go's send is refused with the mute's error, and Go's own mute
//!   shows its end and message, until the owner unmutes it. A unified
//!   bundle restarts Rust's asset stream, and Go's read follows it and the
//!   delta after it.
//! - `whispers_both_ways`: Go's whisper to Rust is stored UNREAD in an
//!   unread thread, Rust's whisper to Go is DELIVERED and stored UNREAD by
//!   Go, and once Go ignores Rust, Rust's next whisper is refused and
//!   FAILED on the first attempt.
//!
//! A read that follows a restarted asset stream may wait until the new
//! stream is as long as the old one. Rust's server and its assets are
//! stored in one transaction, so its first stream is a single bundle.

use std::collections::BTreeMap;
use std::fmt::Debug;

use futures_util::StreamExt;
use prost::Message as _;
use strims_base::rng::det_rng;
use strims_chat::{
    ChatControl, ChatError, ChatFailure, ClientEvent, ClientStream, Config, ServerMode,
    WhisperConfig, WhisperControl,
};
use strims_dao::Table;
use strims_dao::schema::chat::{
    ChatEmotes, ChatModifiers, ChatServerIcons, ChatServers, ChatTags, WhisperRecords,
    WhisperThreads,
};
use strims_dao::schema::network::network_key;
use strims_kv::MemoryKv;
use strims_node::app::{NodeHost, NodeTimer};
use strims_node::{Clock, SystemClock};
use strims_proto::chat::v1::message::Entities;
use strims_proto::chat::v1::message::entities::{Bounds, Emote as EmoteEntity, Tag as TagEntity};
use strims_proto::chat::v1::{
    AssetBundle, Emote, EmoteFileType, EmoteImage, EmoteScale, ListWhispersResponse, Message,
    MessageState, Modifier, Room, Server, ServerEvent, ServerIcon, Tag, WhisperRecord,
    server_event,
};
use strims_proto::network::v1::ca::ca_find_request::Query;
use strims_proto::network::v1::directory::Listing;
use strims_proto::network::v1::directory::listing::{Chat as ChatListing, Content};
use strims_proto::network::v1::{CreateInvitationRequest, NetworkFrontendService};
use strims_proto::service::Context;
use strims_proto::types::Image;
use tokio::task::{LocalSet, spawn_local};
use tokio::time::Instant;

use crate::directory::{
    DIR_STEP, RustMember, Timer, arg, go_options, go_until, id_key, image, retry, rust_member,
    until,
};
use crate::harness::{GoEvent, GoNode, GoOptions, helper};

type Chat = ChatControl<MemoryKv, NodeHost, Timer>;
type Whispers = WhisperControl<MemoryKv, NodeHost, Timer>;
type Stream = ClientStream<MemoryKv, NodeHost, Timer>;

/// A message with an emote, its modifier and a tag, and its entities as
/// the server extracts them.
const RICH: &str = "PEPE:wide nsfw hello";

fn rich_entities() -> Entities {
    let bounds = |start, end| Some(Bounds { start, end });
    Entities {
        emotes: vec![EmoteEntity {
            bounds: bounds(0, 9),
            name: "PEPE".into(),
            modifiers: vec!["wide".into()],
            ..Default::default()
        }],
        tags: vec![TagEntity {
            bounds: bounds(10, 14),
            name: "nsfw".into(),
        }],
        ..Default::default()
    }
}

fn chat_options(seed: u8) -> GoOptions {
    let mut o = go_options(seed);
    o.chat = true;
    o
}

fn room(name: &str) -> Room {
    Room {
        name: name.into(),
        css: String::new(),
    }
}

fn emote(id: u64, server_id: u64, name: &str) -> Emote {
    Emote {
        id,
        server_id,
        name: name.into(),
        images: vec![EmoteImage {
            data: vec![0x89, b'P', b'N', b'G', 1, 2, 3],
            file_type: EmoteFileType::FileTypePng as i32,
            height: 28,
            width: 28,
            scale: EmoteScale::EmoteScale1x as i32,
        }],
        labels: vec!["interop".into()],
        enable: true,
        ..Default::default()
    }
}

fn modifier(id: u64, server_id: u64, name: &str) -> Modifier {
    Modifier {
        id,
        server_id,
        name: name.into(),
        priority: 1,
        ..Default::default()
    }
}

fn tag(id: u64, server_id: u64, name: &str) -> Tag {
    Tag {
        id,
        server_id,
        name: name.into(),
        color: "#ff0000".into(),
        sensitive: true,
    }
}

fn chat_listing(server_key: &[u8], name: &str) -> Listing {
    Listing {
        content: Some(Content::Chat(ChatListing {
            key: server_key.to_vec(),
            name: name.into(),
        })),
    }
}

fn message(e: &ServerEvent) -> Option<&Message> {
    match &e.body {
        Some(server_event::Body::Message(m)) => Some(m),
        None => None,
    }
}

fn now_unix() -> i64 {
    SystemClock.now().unix()
}

/// A room's assets, folded from its bundles: a unified bundle replaces
/// them, a delta removes its removed ids and replaces what it carries.
#[derive(Clone, Default, PartialEq)]
struct Assets {
    room: Option<Room>,
    icon: Option<Image>,
    emotes: BTreeMap<u64, Emote>,
    modifiers: BTreeMap<u64, Modifier>,
    tags: BTreeMap<u64, Tag>,
}

impl Debug for Assets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let emotes: Vec<_> = self.emotes.values().map(|e| (e.id, &e.name)).collect();
        let modifiers: Vec<_> = self.modifiers.values().map(|m| (m.id, &m.name)).collect();
        let tags: Vec<_> = self.tags.values().map(|t| (t.id, &t.name)).collect();
        f.debug_struct("Assets")
            .field("room", &self.room.as_ref().map(|r| &r.name))
            .field("icon_bytes", &self.icon.as_ref().map(|i| i.data.len()))
            .field("emotes", &emotes)
            .field("modifiers", &modifiers)
            .field("tags", &tags)
            .finish()
    }
}

impl Assets {
    fn apply(&mut self, b: &AssetBundle) {
        if !b.is_delta {
            *self = Assets::default();
        }
        for id in &b.removed_ids {
            self.emotes.remove(id);
            self.modifiers.remove(id);
            self.tags.remove(id);
        }
        if b.room.is_some() {
            self.room.clone_from(&b.room);
        }
        if b.icon.is_some() {
            self.icon.clone_from(&b.icon);
        }
        for e in &b.emotes {
            self.emotes.insert(e.id, e.clone());
        }
        for m in &b.modifiers {
            self.modifiers.insert(m.id, m.clone());
        }
        for t in &b.tags {
            self.tags.insert(t.id, t.clone());
        }
    }

    /// The assets of a server's records: its room, its icon, its enabled
    /// emotes, its modifiers and its tags.
    fn of(
        room: &Room,
        icon: &Image,
        emotes: &[Emote],
        modifiers: &[Modifier],
        tags: &[Tag],
    ) -> Assets {
        Assets {
            room: Some(room.clone()),
            icon: Some(icon.clone()),
            emotes: emotes
                .iter()
                .filter(|e| e.enable)
                .map(|e| (e.id, e.clone()))
                .collect(),
            modifiers: modifiers.iter().map(|m| (m.id, m.clone())).collect(),
            tags: tags.iter().map(|t| (t.id, t.clone())).collect(),
        }
    }
}

/// A Rust member with chat and whisper controls.
struct RustChat {
    m: RustMember,
    chat: Chat,
    whispers: Whispers,
}

async fn rust_chat(seed: u8) -> RustChat {
    let m = rust_member(seed).await;
    let chat = ChatControl::new(
        m.host.clone(),
        m.store.clone(),
        m.profile.clone(),
        m.control.clone(),
        m.transfers.clone(),
        m.directory.clone(),
        NodeTimer(SystemClock),
        det_rng(u64::from(seed) + 3),
        Config::default(),
    );
    spawn_local({
        let chat = chat.clone();
        async move { chat.run().await }
    });
    let whispers = WhisperControl::new(
        m.host.clone(),
        m.store.clone(),
        m.profile.clone(),
        m.control.clone(),
        NodeTimer(SystemClock),
        det_rng(u64::from(seed) + 4),
        WhisperConfig::default(),
    );
    spawn_local({
        let whispers = whispers.clone();
        async move { whispers.run().await }
    });
    RustChat { m, chat, whispers }
}

impl RustChat {
    /// The profile's alias on the network with `key`.
    fn alias(&self, key: &[u8]) -> String {
        self.m
            .control
            .network(key)
            .and_then(|n| n.network.certificate)
            .unwrap()
            .subject
    }

    /// Stores the record `f` makes with a new id.
    async fn insert<T: Table>(&self, f: impl FnOnce(u64) -> T::Record) -> T::Record {
        let mut tx = self.m.store.write(SystemClock.now()).await.unwrap();
        let id = tx.generate_id().await.unwrap();
        let r = T::insert(&mut tx, f(id)).await.unwrap();
        tx.commit().await.unwrap();
        r
    }

    /// Stores a chat server on the network with `network_key`, with
    /// `icon`, the emote PEPE, the modifier wide and the tag nsfw, in one
    /// transaction, so that the server's first bundle carries them all.
    async fn create_server(&self, network_key: &[u8], name: &str, icon: &Image) -> Server {
        let mut tx = self.m.store.write(SystemClock.now()).await.unwrap();
        let id = tx.generate_id().await.unwrap();
        let server = ChatServers::insert(
            &mut tx,
            Server {
                id,
                network_key: network_key.to_vec(),
                key: Some(strims_crypto::generate_key(&mut det_rng(id))),
                room: Some(room(name)),
                admin_peer_keys: Vec::new(),
            },
        )
        .await
        .unwrap();
        let icon = ServerIcon {
            id,
            image: Some(icon.clone()),
        };
        ChatServerIcons::insert(&mut tx, icon).await.unwrap();
        let e = emote(tx.generate_id().await.unwrap(), id, "PEPE");
        ChatEmotes::insert(&mut tx, e).await.unwrap();
        let m = modifier(tx.generate_id().await.unwrap(), id, "wide");
        ChatModifiers::insert(&mut tx, m).await.unwrap();
        let t = tag(tx.generate_id().await.unwrap(), id, "nsfw");
        ChatTags::insert(&mut tx, t).await.unwrap();
        tx.commit().await.unwrap();
        server
    }

    async fn all<T: Table>(&self) -> Vec<T::Record> {
        let mut tx = self.m.store.read().await.unwrap();
        T::get_all(&mut tx).await.unwrap()
    }

    /// Whether network `network_id`'s view lists `listing`.
    fn lists(&self, network_id: u64, listing: &Listing) -> bool {
        self.m
            .directory
            .listings(network_id)
            .iter()
            .any(|l| l.listing == *listing)
    }

    fn open(&self, network_key: &[u8], server_key: &[u8]) -> RustRoom {
        RustRoom {
            stream: self.chat.open_client(network_key, server_key).unwrap(),
            bundles: Vec::new(),
            assets: Assets::default(),
            messages: Vec::new(),
        }
    }

    /// The whisper record with `body`.
    async fn whisper(&self, body: &str) -> Option<WhisperRecord> {
        self.all::<WhisperRecords>()
            .await
            .into_iter()
            .find(|r| r.message.as_ref().is_some_and(|m| m.body == body))
    }

    /// Waits until the whisper record with `body` is in `state`.
    async fn whisper_in(&self, body: &str, state: MessageState) -> WhisperRecord {
        let deadline = Instant::now() + DIR_STEP;
        loop {
            let r = self.whisper(body).await;
            if let Some(r) = &r
                && r.state == state as i32
            {
                return r.clone();
            }
            assert!(
                Instant::now() < deadline,
                "rust's whisper {body:?} isn't {state:?}: {r:?}"
            );
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}

/// A Rust client stream and what it has yielded.
struct RustRoom {
    stream: Stream,
    bundles: Vec<AssetBundle>,
    assets: Assets,
    messages: Vec<Message>,
}

impl RustRoom {
    /// Reads the stream until `f` has a value.
    async fn until<T>(&mut self, what: &str, mut f: impl FnMut(&RustRoom) -> Option<T>) -> T {
        let deadline = Instant::now() + DIR_STEP;
        loop {
            if let Some(v) = f(self) {
                return v;
            }
            match tokio::time::timeout_at(deadline, self.stream.next()).await {
                Ok(Some(ClientEvent::Assets(b))) => {
                    self.assets.apply(&b);
                    self.bundles.push(b);
                }
                Ok(Some(ClientEvent::Events(events))) => {
                    self.messages
                        .extend(events.iter().filter_map(message).cloned());
                }
                Ok(None) => panic!("rust's client stream ended waiting for {what}"),
                Err(_) => panic!(
                    "timed out waiting for {what} in rust's client stream: {:?}, {:?}",
                    self.assets, self.messages
                ),
            }
        }
    }

    async fn message(&mut self, body: &str) -> Message {
        self.until(&format!("the message {body:?}"), |r| {
            r.messages.iter().find(|m| m.body == body).cloned()
        })
        .await
    }

    /// Reads the stream until a bundle matching `f`, and with it folded
    /// the assets equal `want`.
    async fn assets(&mut self, what: &str, want: &Assets, f: impl Fn(&AssetBundle) -> bool) {
        let from = self.bundles.len();
        self.until(what, |r| {
            (r.bundles[from..].iter().any(&f) && r.assets == *want).then_some(())
        })
        .await;
    }
}

/// Go's records of its server `server_id`, as the verb `verb` lists them.
async fn go_records<T: prost::Message + Default>(go: &mut GoNode, verb: &str, id: u64) -> Vec<T> {
    let reply = go.cmd(&format!("{verb} {id}")).await.unwrap();
    reply
        .split(',')
        .filter(|r| *r != "-")
        .map(|r| T::decode(hex::decode(r).unwrap().as_slice()).unwrap())
        .collect()
}

/// The assets of Go's server `server_id`.
async fn go_server_assets(go: &mut GoNode, server_id: u64) -> Assets {
    let server = Server::decode(
        hex::decode(go.cmd(&format!("chat-server {server_id}")).await.unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    let icon = ServerIcon::decode(
        hex::decode(go.cmd(&format!("chat-icon {server_id}")).await.unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    Assets::of(
        server.room.as_ref().unwrap(),
        icon.image.as_ref().unwrap(),
        &go_records::<Emote>(go, "chat-emotes", server_id).await,
        &go_records::<Modifier>(go, "chat-modifiers", server_id).await,
        &go_records::<Tag>(go, "chat-tags", server_id).await,
    )
}

/// Takes the message with `body` from Go's read of the server with
/// `server_key`.
async fn go_message(go: &mut GoNode, server_key: &[u8], body: &str) -> Message {
    let deadline = Instant::now() + DIR_STEP;
    let what = format!("the message {body:?} in go's read");
    let has = |e: &ServerEvent| message(e).is_some_and(|m| m.body == body);
    match go
        .wait_until(
            &what,
            deadline,
            |e| matches!(e, GoEvent::ChatEvent(k, ev) if k == server_key && has(ev)),
        )
        .await
    {
        GoEvent::ChatEvent(_, e) => message(&e).unwrap().clone(),
        _ => unreachable!(),
    }
}

/// Folds the bundles of Go's read of the server with `server_key` into
/// `assets` until the bundles folded satisfy `f` and the assets equal
/// `want`.
async fn go_assets(
    go: &mut GoNode,
    server_key: &[u8],
    assets: &mut Assets,
    what: &str,
    want: &Assets,
    f: impl Fn(&[AssetBundle]) -> bool,
) {
    let deadline = Instant::now() + DIR_STEP;
    let mut folded = Vec::new();
    loop {
        let events = go.events();
        let mut i = 0;
        while i < events.len() {
            match &events[i] {
                GoEvent::ChatAssets(k, _) if k == server_key => {
                    let GoEvent::ChatAssets(_, b) = events.remove(i) else {
                        unreachable!()
                    };
                    assets.apply(&b);
                    folded.push(b);
                }
                _ => i += 1,
            }
        }
        if f(&folded) && assets == want {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "go: timed out waiting for {what}: folded {} bundles into {assets:?}, want {want:?}",
            folded.len()
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

/// Retries Go's call `line` until it works, while the vpn route, the
/// server's address and Go's membership get ready. A send that failed may
/// still have arrived.
async fn go_call(go: &mut GoNode, line: &str) -> String {
    retry(line, async || go.cmd(line).await).await
}

/// Go's alias on its network `network_id`.
async fn go_alias(go: &mut GoNode, network_id: u64) -> String {
    let cert = go.cmd(&format!("net-cert {network_id}")).await.unwrap();
    let subject = hex::decode(cert.split(' ').nth(1).unwrap()).unwrap();
    String::from_utf8(subject).unwrap()
}

/// The Go user's mute on the server, from Go's GetMute call: its end in
/// unix seconds and its message.
async fn go_mute(go: &mut GoNode, key_hex: &str, server_key_hex: &str) -> (i64, String) {
    let reply = go_call(go, &format!("chat-get-mute {key_hex} {server_key_hex}")).await;
    let (end, msg) = reply.split_once(' ').unwrap();
    let msg = if msg == "-" {
        Vec::new()
    } else {
        hex::decode(msg).unwrap()
    };
    (end.parse().unwrap(), String::from_utf8(msg).unwrap())
}

/// Go's whisper thread with the peer `peer_key`, and its records.
async fn go_whispers(go: &mut GoNode, peer_key: &[u8]) -> ListWhispersResponse {
    let reply = go
        .cmd(&format!("chat-whispers {}", hex::encode(peer_key)))
        .await
        .unwrap();
    ListWhispersResponse::decode(hex::decode(reply).unwrap().as_slice()).unwrap()
}

/// Takes Go's write of its whisper record `id` in `state`.
async fn go_whisper_state(go: &mut GoNode, id: u64, state: MessageState) {
    let deadline = Instant::now() + DIR_STEP;
    go.wait_until(
        &format!("go's whisper {id} {state:?}"),
        deadline,
        |e| matches!(e, GoEvent::WhisperRecord(r) if r.id == id && r.state == state as i32),
    )
    .await;
}

fn hex_arg(s: &str) -> String {
    if s.is_empty() {
        "-".to_owned()
    } else {
        hex::encode(s)
    }
}

fn assert_from<D: Debug>(m: &Message, peer_key: &[u8], nick: &str, ctx: D) {
    assert_eq!(m.peer_key, peer_key, "{ctx:?}");
    assert_eq!(m.nick, nick, "{ctx:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn go_server_rust_client() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        300,
        LocalSet::new().run_until(async {
            let mut go = GoNode::spawn(&bin, "gc0", chat_options(0xc0)).await;
            let (go_net, key_hex) = id_key(
                &go.cmd(&format!("net-create {}", hex::encode("chat")))
                    .await
                    .unwrap(),
            );
            let key = hex::decode(&key_hex).unwrap();
            let (server_id, server_key_hex) = id_key(
                &go.cmd(&format!(
                    "chat-server-create {key_hex} {}",
                    arg(&room("go room"))
                ))
                .await
                .unwrap(),
            );
            let server_key = hex::decode(&server_key_hex).unwrap();
            go.cmd(&format!(
                "chat-icon-set {server_id} {}",
                arg(&image(2 * 1024, 0x44))
            ))
            .await
            .unwrap();
            for line in [
                format!("chat-emote-create {}", arg(&emote(0, server_id, "PEPE"))),
                format!(
                    "chat-modifier-create {}",
                    arg(&modifier(0, server_id, "wide"))
                ),
                format!("chat-tag-create {}", arg(&tag(0, server_id, "nsfw"))),
            ] {
                go.cmd(&line).await.unwrap();
            }
            go.cmd(&format!("chat-read {key_hex} {server_key_hex}"))
                .await
                .unwrap();
            let invitation = go.cmd(&format!("net-invite {go_net}")).await.unwrap();

            let r = rust_chat(0xc1).await;
            let net_id = r.m.join(hex::decode(&invitation).unwrap()).await.id;
            assert_eq!(r.m.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);
            until("rust to bind go and renew", || {
                (r.m.is_bound(go.host_id) && r.m.trusted(net_id)).then_some(())
            })
            .await;
            // the server answers calls once its address is published,
            // before its listing
            let listing = chat_listing(&server_key, "go room");
            until("go's chat listing in rust's view", || {
                r.lists(net_id, &listing).then_some(())
            })
            .await;
            let go_send =
                |body: &str| format!("chat-send {key_hex} {server_key_hex} {}", hex_arg(body));
            go_call(&mut go, &go_send("hi from go")).await;
            let go_alias = go_alias(&mut go, go_net).await;
            let m = go_message(&mut go, &server_key, "hi from go").await;
            assert_from(&m, &go.profile_key, &go_alias, &m);

            // the stream starts with a unified bundle and the recent events
            let mut client = r.open(&key, &server_key);
            let want = go_server_assets(&mut go, server_id).await;
            client.assets("go's assets", &want, |_| true).await;
            assert!(!client.bundles[0].is_delta, "{:?}", client.bundles[0]);
            let m = client.message("hi from go").await;
            assert_from(&m, &go.profile_key, &go_alias, &m);

            // rust's message, with the entities go's server extracts
            retry("rust's send", async || {
                r.chat
                    .send_message(&key, &server_key, RICH)
                    .await
                    .map_err(|e| e.to_string())
            })
            .await;
            let at_go = go_message(&mut go, &server_key, RICH).await;
            assert_from(&at_go, &r.m.peer_key(), &r.alias(&key), &at_go);
            assert_eq!(at_go.entities, Some(rich_entities()));
            assert_eq!(client.message(RICH).await, at_go);

            // emote changes reach rust as deltas
            let kappa = emote(0, server_id, "Kappa");
            let kappa_id: u64 = go
                .cmd(&format!("chat-emote-create {}", arg(&kappa)))
                .await
                .unwrap()
                .parse()
                .unwrap();
            let want = go_server_assets(&mut go, server_id).await;
            assert_eq!(want.emotes.len(), 2, "{want:?}");
            client
                .assets("go's new emote", &want, |b| {
                    b.is_delta && b.emotes.iter().any(|e| e.id == kappa_id)
                })
                .await;
            let disabled = Emote {
                id: kappa_id,
                enable: false,
                ..kappa
            };
            go.cmd(&format!("chat-emote-update {}", arg(&disabled)))
                .await
                .unwrap();
            let want = go_server_assets(&mut go, server_id).await;
            client
                .assets("go's disabled emote", &want, |b| {
                    b.is_delta && b.removed_ids.contains(&kappa_id)
                })
                .await;

            // a unified bundle restarts go's asset stream, and rust's
            // stream follows it
            let lul_id: u64 = go
                .cmd(&format!(
                    "chat-emote-create {}",
                    arg(&emote(0, server_id, "LUL"))
                ))
                .await
                .unwrap()
                .parse()
                .unwrap();
            go.cmd(&format!("chat-sync {server_id} 1")).await.unwrap();
            let want = go_server_assets(&mut go, server_id).await;
            client
                .assets("go's unified bundle", &want, |b| {
                    !b.is_delta && b.emotes.iter().any(|e| e.id == lul_id)
                })
                .await;

            // moderation by the server's admin keys
            assert_eq!(
                r.chat
                    .mute(&key, &server_key, &go.profile_key, 60, "hush")
                    .await,
                Err(ChatFailure::Refused(ChatError::NotModerator))
            );
            go.cmd(&format!(
                "chat-server-update {server_id} {key_hex} {} {}",
                arg(&room("go room")),
                hex::encode(r.m.peer_key())
            ))
            .await
            .unwrap();
            retry("rust's mute", async || {
                r.chat
                    .mute(&key, &server_key, &go.profile_key, 60, "hush")
                    .await
                    .map_err(|e| e.to_string())
            })
            .await;
            let refused = go.cmd(&go_send("while muted")).await.unwrap_err();
            assert_eq!(
                ChatError::from_message(&refused),
                Some(ChatError::Muted),
                "{refused}"
            );
            r.chat
                .unmute(&key, &server_key, &go.profile_key)
                .await
                .unwrap();
            go.cmd(&go_send("unmuted")).await.unwrap();
            let m = client.message("unmuted").await;
            assert_from(&m, &go.profile_key, &go_alias, &m);
            assert!(!client.messages.iter().any(|m| m.body == "while muted"));

            let status = r.chat.servers();
            assert_eq!(status.len(), 1, "{status:?}");
            assert_eq!(status[0].mode, ServerMode::Reading, "{status:?}");
            let m = client.stream.metrics();
            eprintln!("rust's client stream: {m:?}");
        }),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rust_server_go_client() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        300,
        LocalSet::new().run_until(async {
            let r = rust_chat(0xc5).await;
            let owned = r.m.create("chat", None).await;
            let net_id = owned.id;
            let key = network_key(&owned);
            let key_hex = hex::encode(&key);
            let icon = image(2 * 1024, 0x55);
            let server = r.create_server(&key, "rust room", &icon).await;
            let server_key = server.key.as_ref().unwrap().public.clone();
            let server_key_hex = hex::encode(&server_key);
            let rust_assets = async || {
                Assets::of(
                    &room("rust room"),
                    &icon,
                    &r.all::<ChatEmotes>().await,
                    &r.all::<ChatModifiers>().await,
                    &r.all::<ChatTags>().await,
                )
            };
            until("rust to serve", || {
                let s = r.chat.servers();
                let s = s.first()?;
                (s.mode == ServerMode::Serving && s.listing_id.is_some()).then_some(())
            })
            .await;
            r.chat
                .send_message(&key, &server_key, "hi from rust")
                .await
                .unwrap();
            let invitation =
                r.m.frontend
                    .create_invitation(
                        &Context::default(),
                        CreateInvitationRequest {
                            network_id: net_id,
                            bootstrap_client_id: 0,
                        },
                    )
                    .await
                    .unwrap()
                    .invitation
                    .unwrap();

            let mut go = GoNode::spawn(&bin, "gc1", chat_options(0xc4)).await;
            let (go_net, joined_key) = id_key(
                &go.cmd(&format!(
                    "net-join {}",
                    hex::encode(invitation.encode_to_vec())
                ))
                .await
                .unwrap(),
            );
            assert_eq!(joined_key, key_hex);
            assert_eq!(r.m.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);
            until("rust to bind go", || r.m.is_bound(go.host_id).then_some(())).await;
            let listing = chat_listing(&server_key, "rust room");
            go_until(&mut go, go_net, "rust's chat listing", |ls| {
                ls.iter().any(|l| l.listing.as_ref() == Some(&listing))
            })
            .await;

            // go's read starts with a unified bundle and the recent events
            go.cmd(&format!("chat-read {key_hex} {server_key_hex}"))
                .await
                .unwrap();
            let mut go_seen = Assets::default();
            go_assets(
                &mut go,
                &server_key,
                &mut go_seen,
                "rust's assets",
                &rust_assets().await,
                |bs| bs.first().is_some_and(|b| !b.is_delta),
            )
            .await;
            let m = go_message(&mut go, &server_key, "hi from rust").await;
            assert_from(&m, &r.m.peer_key(), &r.alias(&key), &m);

            // go's message, with the entities rust's server extracts
            let mut client = r.open(&key, &server_key);
            let go_send =
                |body: &str| format!("chat-send {key_hex} {server_key_hex} {}", hex_arg(body));
            go_call(&mut go, &go_send(RICH)).await;
            let at_go = go_message(&mut go, &server_key, RICH).await;
            let go_alias = go_alias(&mut go, go_net).await;
            assert_from(&at_go, &go.profile_key, &go_alias, &at_go);
            assert_eq!(at_go.entities, Some(rich_entities()));
            assert_eq!(client.message(RICH).await, at_go);

            // the owner mutes go
            let before = now_unix();
            r.chat
                .mute(&key, &server_key, &go.profile_key, 600, "cool it")
                .await
                .unwrap();
            let after = now_unix();
            assert_eq!(
                go.cmd(&go_send("while muted")).await,
                Err(ChatError::Muted.to_string())
            );
            let (end, msg) = go_mute(&mut go, &key_hex, &server_key_hex).await;
            assert!(
                (before + 600..=after + 600).contains(&end),
                "{before} {end} {after}"
            );
            assert_eq!(msg, "cool it");
            r.chat
                .unmute(&key, &server_key, &go.profile_key)
                .await
                .unwrap();
            assert_eq!(
                go_mute(&mut go, &key_hex, &server_key_hex).await,
                (0, String::new())
            );
            go.cmd(&go_send("unmuted")).await.unwrap();
            go_message(&mut go, &server_key, "unmuted").await;
            client.message("unmuted").await;
            assert!(!client.messages.iter().any(|m| m.body == "while muted"));

            // a unified bundle that restarts the asset stream, then a delta
            let lul = r
                .insert::<ChatEmotes>(|id| emote(id, server.id, "LUL"))
                .await;
            r.chat.sync_assets(server.id, true).unwrap();
            let want = rust_assets().await;
            let unified = |b: &AssetBundle| !b.is_delta && b.emotes.iter().any(|e| e.id == lul.id);
            client.assets("rust's unified bundle", &want, unified).await;
            let kappa = r
                .insert::<ChatEmotes>(|id| emote(id, server.id, "Kappa"))
                .await;
            let want = rust_assets().await;
            assert_eq!(want.emotes.len(), 3, "{want:?}");
            let delta = |b: &AssetBundle| b.is_delta && b.emotes.iter().any(|e| e.id == kappa.id);
            client.assets("rust's delta", &want, delta).await;
            go_assets(
                &mut go,
                &server_key,
                &mut go_seen,
                "rust's unified bundle and delta",
                &want,
                |bs| bs.iter().any(unified) && bs.iter().any(delta),
            )
            .await;

            let status = r.chat.servers();
            eprintln!("rust's server: {status:?}");
            let s = status[0].server.as_ref().unwrap();
            assert_eq!((s.mutes, s.unmutes, s.refused_muted), (1, 1, 1), "{s:?}");
        }),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn whispers_both_ways() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        300,
        LocalSet::new().run_until(async {
            let mut go = GoNode::spawn(&bin, "gc2", chat_options(0xc8)).await;
            let (go_net, key_hex) = id_key(
                &go.cmd(&format!("net-create {}", hex::encode("whispers")))
                    .await
                    .unwrap(),
            );
            let key = hex::decode(&key_hex).unwrap();
            let invitation = go.cmd(&format!("net-invite {go_net}")).await.unwrap();
            let r = rust_chat(0xc9).await;
            let net_id = r.m.join(hex::decode(&invitation).unwrap()).await.id;
            assert_eq!(r.m.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);
            until(
                "rust to bind go, renew and publish its whisper address",
                || {
                    (r.m.is_bound(go.host_id) && r.m.trusted(net_id) && r.whispers.published(&key))
                        .then_some(())
                },
            )
            .await;
            let rust_key = r.m.peer_key();
            let rust_alias = r.alias(&key);

            // go to rust
            let id: u64 = go_call(
                &mut go,
                &format!(
                    "chat-whisper {key_hex} {} - {}",
                    hex::encode(&rust_key),
                    hex_arg("psst, strims.gg")
                ),
            )
            .await
            .parse()
            .unwrap();
            go_whisper_state(&mut go, id, MessageState::Delivered).await;
            let got = r.whisper_in("psst, strims.gg", MessageState::Unread).await;
            assert_eq!(got.peer_key, go.profile_key);
            assert_eq!(got.peer_message_id, id);
            assert_eq!(got.network_key, key);
            let m = got.message.as_ref().unwrap();
            let go_alias = go_alias(&mut go, go_net).await;
            assert_from(m, &go.profile_key, &go_alias, m);
            assert_eq!(
                m.entities.as_ref().unwrap().links[0].url,
                "https://strims.gg"
            );
            let threads = r.all::<WhisperThreads>().await;
            assert_eq!(threads.len(), 1, "{threads:?}");
            assert!(threads[0].has_unread, "{threads:?}");
            assert_eq!(threads[0].peer_key, go.profile_key);
            assert_eq!(threads[0].last_message_id, got.id);
            assert_eq!(threads[0].alias, m.nick);
            assert_eq!(r.whispers.metrics().received, 1);

            // rust to go
            let cert = retry("go's certificate at go's CA", async || {
                r.m.control
                    .find_certificate(&key, Query::Key(go.profile_key.clone()), false)
                    .await
                    .map_err(|e| e.to_string())
            })
            .await;
            assert_eq!(cert.key, go.profile_key);
            let sent = r
                .whispers
                .enqueue(&key, b"", &go.profile_key, &cert.subject, "hi go")
                .await
                .unwrap();
            r.whisper_in("hi go", MessageState::Delivered).await;
            let deadline = Instant::now() + DIR_STEP;
            let GoEvent::WhisperRecord(at_go) = go
                .wait_until(
                    "rust's whisper in go's store",
                    deadline,
                    |e| matches!(e, GoEvent::WhisperRecord(w) if w.peer_message_id == sent.id),
                )
                .await
            else {
                unreachable!()
            };
            assert_eq!(at_go.state, MessageState::Unread as i32, "{at_go:?}");
            assert_eq!(at_go.peer_key, rust_key);
            let m = at_go.message.as_ref().unwrap();
            assert_eq!(
                (m.body.as_str(), m.nick.as_str()),
                ("hi go", rust_alias.as_str())
            );
            let listed = go_whispers(&mut go, &rust_key).await;
            assert!(listed.thread.unwrap().has_unread);
            assert!(listed.whispers.iter().any(|w| w.id == at_go.id));

            // go ignores rust: the refusal is final
            go.cmd(&format!(
                "chat-ignore {} {} -",
                hex::encode(&rust_key),
                hex_arg(&rust_alias)
            ))
            .await
            .unwrap();
            let attempts = r.whispers.metrics().attempts;
            r.whispers
                .enqueue(&key, b"", &go.profile_key, &cert.subject, "ignored")
                .await
                .unwrap();
            r.whisper_in("ignored", MessageState::Failed).await;
            let m = r.whispers.metrics();
            assert_eq!(m.attempts, attempts + 1, "{m:?}");
            assert_eq!((m.delivered, m.failed), (1, 1), "{m:?}");
            let listed = go_whispers(&mut go, &rust_key).await;
            assert!(
                !listed
                    .whispers
                    .iter()
                    .any(|w| w.message.as_ref().is_some_and(|m| m.body == "ignored")),
                "{listed:?}"
            );
        }),
    )
    .await;
}
