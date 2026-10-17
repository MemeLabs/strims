// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The chat frontends on fake nodes (see `strims_testworld`), through a
//! UI's RPC connection: chat server records and their assets, client
//! streams with their split bundles and viewed listings, moderation by
//! alias, whispers, and the UI's settings.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fmt::Debug;

use futures_util::{Stream, StreamExt};
use prost::Message as _;
use strims_chat::frontend::MAX_RESPONSE_BYTES;
use strims_chat::reader::RoomLimits;
use strims_chat::{
    ChatControl, ChatEvent, ChatFrontend, Config, ServerMode, WhisperConfig, WhisperControl,
    bind_frontend,
};
use strims_dao::schema::chat::{ChatUiConfigIgnores, WhisperRecords};
use strims_dao::{Recv, Subscription, Table};
use strims_kv::MemoryKv;
use strims_ppspp::{SwarmId, Uri, UriOptions};
use strims_proto::chat::v1::emote_effect::{Effect, SpriteAnimation};
use strims_proto::chat::v1::list_emotes_request::Part;
use strims_proto::chat::v1::message::DirectoryRef;
use strims_proto::chat::v1::open_client_response::Body as OpenBody;
use strims_proto::chat::v1::watch_ui_config_response::Config as UiBody;
use strims_proto::chat::v1::watch_whispers_response::Body as WhisperBody;
use strims_proto::chat::v1::{
    AssetBundle, ChatFrontendClient, ChatServerFrontendClient, ClientGetMuteRequest,
    ClientMuteRequest, ClientSendMessageRequest, ClientUnmuteRequest, CreateEmoteRequest,
    CreateModifierRequest, CreateServerRequest, CreateTagRequest, DeleteEmoteRequest,
    DeleteModifierRequest, DeleteServerRequest, DeleteTagRequest, DeleteWhisperThreadRequest,
    Emote, EmoteContributor, EmoteEffect, EmoteImage, GetEmoteRequest, GetModifierRequest,
    GetServerIconRequest, GetServerRequest, GetTagRequest, HighlightRequest, IgnoreRequest,
    ListEmoteLabelsRequest, ListEmotesRequest, ListModifiersRequest, ListServersRequest,
    ListTagsRequest, ListWhispersRequest, MarkWhispersReadRequest, Message, MessageState,
    OpenClientRequest, OpenClientResponse, Room, Server, SetUiConfigRequest, SyncAssetsRequest,
    TagRequest, UiConfig, UnhighlightRequest, UnignoreRequest, UntagRequest, UpdateEmoteRequest,
    UpdateEmotesRequest, UpdateModifierRequest, UpdateServerIconRequest, UpdateServerRequest,
    UpdateTagRequest, WatchUiConfigRequest, WatchUiConfigResponse, WatchWhispersRequest,
    WatchWhispersResponse, WhisperRequest, server_event,
};
use strims_proto::network::v1::directory::Listing;
use strims_proto::network::v1::directory::ListingQuery;
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::network::v1::directory::listing_query::Query;
use strims_proto::service::CallError;
use strims_proto::types::{Image, ImageType};
use strims_rpc::{Client, Router, Timer};
use strims_testworld::*;

type Chat = ChatControl<MemoryKv, FakeHost, TokioTimer>;
type Whispers = WhisperControl<MemoryKv, FakeHost, TokioTimer>;

/// A node's chat and whisper controls, and a UI connected to their
/// frontends.
struct Side {
    chat: Chat,
    servers: ChatServerFrontendClient<Client>,
    ui: ChatFrontendClient<Client>,
}

fn start(n: &Node) -> Side {
    start_with(n, Config::default(), None)
}

/// Runs the controls on `n` with `config`, and a frontend whose responses
/// encode to at most `max_response`, if given.
fn start_with(n: &Node, config: Config, max_response: Option<usize>) -> Side {
    let chat = ChatControl::new(
        n.host.clone(),
        n.store.clone(),
        n.profile.clone(),
        n.network.clone(),
        n.transfers.clone(),
        n.directory.clone(),
        n.timer.clone(),
        n.rng(),
        config,
    );
    let whispers: Whispers = WhisperControl::new(
        n.host.clone(),
        n.store.clone(),
        n.profile.clone(),
        n.network.clone(),
        n.timer.clone(),
        n.rng(),
        WhisperConfig::default(),
    );
    let (c, w) = (chat.clone(), whispers.clone());
    n.spawn(async move { c.run().await });
    n.spawn(async move { w.run().await });
    let router = Router::new();
    match max_response {
        Some(max) => ChatFrontend::new(chat.clone(), whispers)
            .with_max_response_bytes(max)
            .bind(&router),
        None => bind_frontend(&router, &chat, &whispers),
    }
    let client = frontend_client(router);
    Side {
        chat,
        servers: ChatServerFrontendClient::new(client.clone()),
        ui: ChatFrontendClient::new(client),
    }
}

/// The message of a call's remote error.
fn refusal<T: Debug>(r: Result<T, CallError>) -> String {
    match r {
        Err(CallError::Remote(e)) => e.message,
        r => panic!("not refused: {r:?}"),
    }
}

/// The next item of a streaming call within `secs` seconds.
async fn next<S, T>(s: &mut S, secs: u64) -> T
where
    S: Stream<Item = Result<T, CallError>> + Unpin,
    T: Debug,
{
    let wait = std::time::Duration::from_secs(secs);
    match tokio::time::timeout(wait, s.next()).await {
        Ok(Some(Ok(t))) => t,
        r => panic!("no item: {r:?}"),
    }
}

/// The items of a streaming call until one `f` picks, within `secs`
/// seconds, that one included.
async fn until<S, T>(s: &mut S, secs: u64, what: &str, f: impl Fn(&T) -> bool) -> Vec<T>
where
    S: Stream<Item = Result<T, CallError>> + Unpin,
    T: Debug,
{
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(secs);
    let mut out = Vec::new();
    loop {
        match tokio::time::timeout_at(deadline, s.next()).await {
            Ok(Some(Ok(t))) => {
                let done = f(&t);
                out.push(t);
                if done {
                    return out;
                }
            }
            r => panic!("no {what} in {out:?}: {r:?}"),
        }
    }
}

/// Whether a streaming call has nothing more within a second.
async fn quiet<S, T>(s: &mut S) -> bool
where
    S: Stream<Item = Result<T, CallError>> + Unpin,
{
    let wait = std::time::Duration::from_secs(1);
    tokio::time::timeout(wait, s.next()).await.is_err()
}

fn key(n: &Node) -> Vec<u8> {
    n.profile_key().public.clone()
}

/// `n`'s alias on network `id`.
async fn alias(n: &Node, id: u64) -> String {
    n.read_network(id).await.certificate.unwrap().subject
}

fn room(name: &str) -> Option<Room> {
    Some(Room {
        name: name.into(),
        css: String::new(),
    })
}

fn image(n: u8, len: usize) -> EmoteImage {
    EmoteImage {
        data: vec![n; len],
        file_type: 1,
        height: 28,
        width: 28,
        scale: 0,
    }
}

fn server_key(s: &Server) -> Vec<u8> {
    s.key.as_ref().unwrap().public.clone()
}

async fn create_server(side: &Side, network_key: &[u8], name: &str) -> Server {
    side.servers
        .create_server(&CreateServerRequest {
            network_key: network_key.to_vec(),
            room: room(name),
        })
        .await
        .unwrap()
        .server
        .unwrap()
}

async fn create_emote(side: &Side, server_id: u64, name: &str, images: Vec<EmoteImage>) -> Emote {
    side.servers
        .create_emote(&CreateEmoteRequest {
            server_id,
            name: name.into(),
            images,
            enable: true,
            ..Default::default()
        })
        .await
        .unwrap()
        .emote
        .unwrap()
}

/// Waits for `chat` to serve the server with key `skey` and publish its
/// listing.
async fn serving(events: &mut Subscription<ChatEvent>, chat: &Chat, skey: &[u8]) {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        match tokio::time::timeout_at(deadline, events.next()).await {
            Ok(Some(Recv::Event(e))) => {
                if matches!(&*e, ChatEvent::ListingChanged { server_key, .. } if server_key == skey)
                {
                    break;
                }
            }
            Ok(Some(Recv::Lagged)) => {}
            r => panic!("not serving: {r:?}"),
        }
    }
    let mode = chat
        .servers()
        .into_iter()
        .find(|s| s.server_key == skey)
        .map(|s| s.mode);
    assert_eq!(mode, Some(ServerMode::Serving));
    sleep_secs(3).await;
}

/// The messages of an `OpenClient` response.
fn messages(r: &OpenClientResponse) -> Vec<Message> {
    match &r.body {
        Some(OpenBody::ServerEvents(e)) => e
            .events
            .iter()
            .map(|e| match &e.body {
                Some(server_event::Body::Message(m)) => m.clone(),
                None => panic!("an event without a message"),
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Reads an `OpenClient` stream until a message with `body` comes.
async fn message<S>(s: &mut S, body: &str) -> Message
where
    S: Stream<Item = Result<OpenClientResponse, CallError>> + Unpin,
{
    let rs = until(s, 10, body, |r| messages(r).iter().any(|m| m.body == body)).await;
    rs.iter()
        .flat_map(messages)
        .find(|m| m.body == body)
        .unwrap()
}

/// A server's records go through the frontend: created with their own
/// keys and ids on an existing server, read, changed and deleted, with
/// their bounds checked; deleting the server deletes its records.
#[test]
fn server_records_round_trip_and_go_with_their_server() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let net = net.public;
        let a = start(&alice);

        let s = create_server(&a, &net, "room").await;
        let k = s.key.clone().unwrap();
        assert_eq!((k.public.len(), k.private.len()), (32, 64));
        assert_eq!(
            (s.network_key.as_slice(), s.room.clone()),
            (&net[..], room("room"))
        );
        let other = create_server(&a, &net, "other").await;
        assert_ne!(server_key(&other), server_key(&s));
        assert_ne!(other.id, s.id);
        let refused = |network_key: Vec<u8>, room: Option<Room>| {
            a.servers
                .create_server(&CreateServerRequest { network_key, room })
        };
        assert_eq!(
            refusal(refused(vec![1; 32], room("x")).await),
            "network not found"
        );
        assert_eq!(
            refusal(refused(net.clone(), room(&"x".repeat(300))).await),
            "room name too long"
        );

        let got = a.servers.get_server(&GetServerRequest { id: s.id }).await;
        assert_eq!(got.unwrap().server, Some(s.clone()));
        let all = a
            .servers
            .list_servers(&ListServersRequest {})
            .await
            .unwrap();
        assert_eq!(all.servers.len(), 2);
        let s = a
            .servers
            .update_server(&UpdateServerRequest {
                id: s.id,
                network_key: net.clone(),
                room: room("lobby"),
            })
            .await
            .unwrap()
            .server
            .unwrap();
        assert_eq!((s.room.clone(), s.key.clone()), (room("lobby"), Some(k)));
        let missing = a.servers.update_server(&UpdateServerRequest {
            id: 999,
            network_key: net.clone(),
            room: room("x"),
        });
        assert_eq!(refusal(missing.await), "chat server not found");

        // icon
        let icon_of = |server_id| {
            a.servers
                .get_server_icon(&GetServerIconRequest { server_id })
        };
        assert_eq!(icon_of(s.id).await.unwrap().server_icon, None);
        let img = Image {
            r#type: ImageType::Png.into(),
            height: 2,
            width: 2,
            data: vec![7; 64],
        };
        let set_icon = |server_id| {
            a.servers.update_server_icon(&UpdateServerIconRequest {
                server_id,
                image: Some(img.clone()),
            })
        };
        let icon = set_icon(s.id).await.unwrap().server_icon.unwrap();
        assert_eq!((icon.id, icon.image.as_ref()), (s.id, Some(&img)));
        assert_eq!(icon_of(s.id).await.unwrap().server_icon, Some(icon));
        assert_eq!(refusal(set_icon(999).await), "chat server not found");

        // emotes
        let e = create_emote(&a, s.id, "PEPE", vec![image(1, 16)]).await;
        assert_eq!((e.server_id, e.name.as_str()), (s.id, "PEPE"));
        let create = |server_id, name: &str| {
            a.servers.create_emote(&CreateEmoteRequest {
                server_id,
                name: name.into(),
                ..Default::default()
            })
        };
        assert_eq!(refusal(create(999, "X").await), "chat server not found");
        assert_eq!(refusal(create(s.id, "").await), "emote name is empty");
        let got = a.servers.get_emote(&GetEmoteRequest { id: e.id }).await;
        assert_eq!(got.unwrap().emote, Some(e.clone()));
        let e = a
            .servers
            .update_emote(&UpdateEmoteRequest {
                server_id: s.id,
                id: e.id,
                name: "PEPEW".into(),
                labels: vec!["frog".into()],
                enable: false,
                ..Default::default()
            })
            .await
            .unwrap()
            .emote
            .unwrap();
        assert_eq!(
            (e.name.as_str(), e.enable, e.images.len()),
            ("PEPEW", false, 0)
        );
        assert_eq!(e.labels, ["frog"]);

        // modifiers
        let m = a
            .servers
            .create_modifier(&CreateModifierRequest {
                server_id: s.id,
                name: "wide".into(),
                proc_chance: 0.5,
                ..Default::default()
            })
            .await
            .unwrap()
            .modifier
            .unwrap();
        let bad = a.servers.create_modifier(&CreateModifierRequest {
            server_id: s.id,
            name: "odd".into(),
            proc_chance: 1.5,
            ..Default::default()
        });
        assert_eq!(refusal(bad.await), "proc chance must be from 0 to 1");
        let m = a
            .servers
            .update_modifier(&UpdateModifierRequest {
                server_id: s.id,
                id: m.id,
                name: "wider".into(),
                priority: 3,
                ..Default::default()
            })
            .await
            .unwrap()
            .modifier
            .unwrap();
        assert_eq!((m.name.as_str(), m.priority), ("wider", 3));
        let got = a
            .servers
            .get_modifier(&GetModifierRequest { id: m.id })
            .await;
        assert_eq!(got.unwrap().modifier, Some(m.clone()));
        let ms = a
            .servers
            .list_modifiers(&ListModifiersRequest { server_id: s.id })
            .await
            .unwrap();
        assert_eq!(ms.modifiers, std::slice::from_ref(&m));

        // tags
        let t = a
            .servers
            .create_tag(&CreateTagRequest {
                server_id: s.id,
                name: "nsfw".into(),
                color: "#f00".into(),
                sensitive: true,
            })
            .await
            .unwrap()
            .tag
            .unwrap();
        let t = a
            .servers
            .update_tag(&UpdateTagRequest {
                server_id: s.id,
                id: t.id,
                name: "nsfl".into(),
                color: "#000".into(),
                sensitive: false,
            })
            .await
            .unwrap()
            .tag
            .unwrap();
        let got = a.servers.get_tag(&GetTagRequest { id: t.id }).await;
        assert_eq!(got.unwrap().tag, Some(t.clone()));
        let ts = a
            .servers
            .list_tags(&ListTagsRequest { server_id: s.id })
            .await
            .unwrap();
        assert_eq!(ts.tags, std::slice::from_ref(&t));
        let delete_tag = |id| {
            a.servers.delete_tag(&DeleteTagRequest {
                server_id: s.id,
                id,
            })
        };
        delete_tag(t.id).await.unwrap();
        assert_eq!(refusal(delete_tag(t.id).await), "tag not found");
        let got = a.servers.get_tag(&GetTagRequest { id: t.id }).await;
        assert_eq!(refusal(got), "tag not found");

        // the server goes with its records
        a.servers
            .delete_server(&DeleteServerRequest { id: s.id })
            .await
            .unwrap();
        let got = a.servers.get_server(&GetServerRequest { id: s.id }).await;
        assert_eq!(refusal(got), "chat server not found");
        let got = a.servers.get_emote(&GetEmoteRequest { id: e.id }).await;
        assert_eq!(refusal(got), "emote not found");
        let got = a
            .servers
            .get_modifier(&GetModifierRequest { id: m.id })
            .await;
        assert_eq!(refusal(got), "modifier not found");
        assert_eq!(icon_of(s.id).await.unwrap().server_icon, None);
        let es = a.servers.list_emotes(&ListEmotesRequest {
            server_id: s.id,
            parts: Vec::new(),
        });
        assert!(es.await.unwrap().emotes.is_empty());
        let all = a
            .servers
            .list_servers(&ListServersRequest {})
            .await
            .unwrap();
        assert_eq!(all.servers, std::slice::from_ref(&other));
        let gone = a.servers.delete_server(&DeleteServerRequest { id: s.id });
        assert_eq!(refusal(gone.await), "chat server not found");
        let gone = a.servers.delete_emote(&DeleteEmoteRequest {
            server_id: s.id,
            id: e.id,
        });
        assert_eq!(refusal(gone.await), "emote not found");
        let gone = a.servers.delete_modifier(&DeleteModifierRequest {
            server_id: s.id,
            id: m.id,
        });
        assert_eq!(refusal(gone.await), "modifier not found");
    });
}

/// `ListEmotes` gives the fields of the parts asked for; `UpdateEmotes`
/// changes the labels and enablement of several emotes at once, all or
/// none; `ListEmoteLabels` gives every label once, sorted. A server holds
/// no more emotes than its readers take.
#[test]
fn emotes_list_by_part_and_change_in_bulk() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let config = Config {
            room: RoomLimits {
                max_emotes: 3,
                ..RoomLimits::default()
            },
            ..Config::default()
        };
        let a = start_with(&alice, config, None);
        let s = create_server(&a, &net.public, "room").await;

        let effect = EmoteEffect {
            effect: Some(Effect::SpriteAnimation(SpriteAnimation {
                frame_count: 4,
                ..Default::default()
            })),
        };
        let create = |name: &str, labels: &[&str], n: u8| {
            a.servers.create_emote(&CreateEmoteRequest {
                server_id: s.id,
                name: name.into(),
                images: vec![image(n, 8)],
                effects: vec![effect.clone()],
                contributor: Some(EmoteContributor {
                    name: "artist".into(),
                    link: String::new(),
                }),
                labels: labels.iter().map(|l| (*l).to_owned()).collect(),
                enable: true,
                ..Default::default()
            })
        };
        let e1 = create("A", &["x"], 1).await.unwrap().emote.unwrap();
        let e2 = create("B", &["y", "x"], 2).await.unwrap().emote.unwrap();
        let e3 = create("C", &[], 3).await.unwrap().emote.unwrap();
        assert_eq!(refusal(create("D", &[], 4).await), "too many emotes");

        let list = |parts: &[Part]| {
            a.servers.list_emotes(&ListEmotesRequest {
                server_id: s.id,
                parts: parts.iter().map(|p| i32::from(*p)).collect(),
            })
        };
        let by_id = |mut es: Vec<Emote>| {
            es.sort_by_key(|e| e.id);
            es
        };
        let all = vec![e1.clone(), e2.clone(), e3.clone()];
        assert_eq!(by_id(list(&[]).await.unwrap().emotes), all);
        assert_eq!(
            by_id(list(&[Part::Meta, Part::Assets]).await.unwrap().emotes),
            all
        );
        let meta = by_id(list(&[Part::Meta]).await.unwrap().emotes);
        for (m, e) in meta.iter().zip(&all) {
            let want = Emote {
                images: Vec::new(),
                effects: Vec::new(),
                ..e.clone()
            };
            assert_eq!(*m, want);
        }
        let assets = by_id(list(&[Part::Assets]).await.unwrap().emotes);
        for (m, e) in assets.iter().zip(&all) {
            let want = Emote {
                id: e.id,
                server_id: e.server_id,
                images: e.images.clone(),
                effects: e.effects.clone(),
                ..Emote::default()
            };
            assert_eq!(*m, want);
        }
        assert_eq!(refusal(list(&[Part::Undefined]).await), "invalid part");

        let update = |ids: Vec<u64>, add: &[&str], remove: &[&str], enable| {
            a.servers.update_emotes(&UpdateEmotesRequest {
                server_id: s.id,
                ids,
                add_labels: add.iter().map(|l| (*l).to_owned()).collect(),
                remove_labels: remove.iter().map(|l| (*l).to_owned()).collect(),
                enable,
            })
        };
        let get = async |id| {
            a.servers
                .get_emote(&GetEmoteRequest { id })
                .await
                .unwrap()
                .emote
                .unwrap()
        };
        update(vec![e1.id, e2.id], &["z", "x", "z"], &["y"], Some(false))
            .await
            .unwrap();
        for id in [e1.id, e2.id] {
            let e = get(id).await;
            assert_eq!(e.labels, ["x", "z"]);
            assert!(!e.enable);
        }
        update(vec![e2.id], &[], &[], None).await.unwrap();
        assert!(!get(e2.id).await.enable);
        update(vec![e2.id], &[], &["x"], Some(true)).await.unwrap();
        let e = get(e2.id).await;
        assert_eq!(e.labels, ["z"]);
        assert!(e.enable);
        assert_eq!(
            refusal(update(vec![e1.id, 999], &["q"], &[], None).await),
            "emote not found"
        );
        assert_eq!(get(e1.id).await.labels, ["x", "z"]);

        let labels = a
            .servers
            .list_emote_labels(&ListEmoteLabelsRequest { server_id: s.id })
            .await
            .unwrap()
            .labels;
        assert_eq!(labels, ["x", "z"]);
    });
}

/// `SyncAssets` has the serving process publish a unified bundle at once
/// on request; a server of the profile that isn't running isn't refused,
/// and one it doesn't have is.
#[test]
fn sync_assets_reaches_the_served_server() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let net = net.public;
        let config = Config {
            max_served: 1,
            ..Config::default()
        };
        let a = start_with(&alice, config, None);
        let mut events = a.chat.subscribe();
        let s = create_server(&a, &net, "room").await;
        let skey = server_key(&s);
        serving(&mut events, &a.chat, &skey).await;

        let mut stream =
            a.ui.open_client(&OpenClientRequest {
                network_key: net.clone(),
                server_key: skey.clone(),
            })
            .await
            .unwrap();
        assert_eq!(
            next(&mut stream, 5).await.body,
            Some(OpenBody::Open(Default::default()))
        );
        let bundle = |r: OpenClientResponse| match r.body {
            Some(OpenBody::AssetBundle(b)) => b,
            b => panic!("not a bundle: {b:?}"),
        };
        let b = bundle(next(&mut stream, 5).await);
        assert!(!b.is_delta);
        assert_eq!(b.room, room("room"));

        let e = create_emote(&a, s.id, "PEPE", Vec::new()).await;
        let d = bundle(next(&mut stream, 5).await);
        assert!(d.is_delta);
        assert_eq!(d.emotes, std::slice::from_ref(&e));

        let sync = |server_id, force_unified_update| {
            a.servers.sync_assets(&SyncAssetsRequest {
                server_id,
                force_unified_update,
            })
        };
        sync(s.id, true).await.unwrap();
        let u = bundle(next(&mut stream, 5).await);
        assert!(!u.is_delta);
        assert_eq!((u.room, u.emotes), (room("room"), vec![e]));

        // over the control's bound of served servers
        let idle = create_server(&a, &net, "idle").await;
        sleep_secs(1).await;
        assert!(
            a.chat
                .servers()
                .iter()
                .all(|st| st.server_id != Some(idle.id))
        );
        sync(idle.id, true).await.unwrap();
        assert_eq!(refusal(sync(999, false).await), "chat server not found");
    });
}

/// A client stream opened late answers `Open`, then the room's unified
/// bundle split into deltas that each fit a response, then the room's
/// recent events.
#[test]
fn a_late_client_gets_the_split_unified_bundle_then_events() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (net, _pipes) = owned_network(&owner, &[&alice, &bob], 10).await;
        let net = net.public;
        let a = start(&alice);
        let max = 16 * 1024;
        let b = start_with(&bob, Config::default(), Some(max));
        let mut events = a.chat.subscribe();
        let s = create_server(&a, &net, "room").await;
        let skey = server_key(&s);
        serving(&mut events, &a.chat, &skey).await;

        let mut emotes = Vec::new();
        for (i, name) in ["A", "B", "C", "D", "E"].iter().enumerate() {
            emotes.push(create_emote(&a, s.id, name, vec![image(i as u8, 5000)]).await);
        }
        a.servers
            .sync_assets(&SyncAssetsRequest {
                server_id: s.id,
                force_unified_update: true,
            })
            .await
            .unwrap();
        sleep_secs(1).await;
        for body in ["one", "two"] {
            a.ui.client_send_message(&ClientSendMessageRequest {
                network_key: net.clone(),
                server_key: skey.clone(),
                body: body.into(),
            })
            .await
            .unwrap();
        }
        sleep_secs(1).await;

        let mut stream =
            b.ui.open_client(&OpenClientRequest {
                network_key: net.clone(),
                server_key: skey.clone(),
            })
            .await
            .unwrap();
        assert_eq!(
            next(&mut stream, 5).await.body,
            Some(OpenBody::Open(Default::default()))
        );
        let mut parts: Vec<AssetBundle> = Vec::new();
        let mut got = Vec::new();
        while got.len() < emotes.len() {
            let r = next(&mut stream, 5).await;
            assert!(r.encoded_len() <= max, "{} bytes", r.encoded_len());
            let Some(OpenBody::AssetBundle(part)) = r.body else {
                panic!("not a bundle part: {r:?}");
            };
            got.extend(part.emotes.iter().cloned());
            parts.push(part);
        }
        assert!(parts.len() > 1);
        assert!(!parts[0].is_delta);
        assert_eq!(parts[0].room, room("room"));
        assert!(parts[1..].iter().all(|p| p.is_delta));
        got.sort_by_key(|e| e.id);
        assert_eq!(got, emotes);

        let r = next(&mut stream, 5).await;
        let bodies: Vec<String> = messages(&r).into_iter().map(|m| m.body).collect();
        assert_eq!(bodies, ["one", "two"], "{r:?}");

        // the default bound takes the bundle whole
        let c = start(&bob);
        let mut stream =
            c.ui.open_client(&OpenClientRequest {
                network_key: net.clone(),
                server_key: skey.clone(),
            })
            .await
            .unwrap();
        next(&mut stream, 5).await;
        let r = next(&mut stream, 5).await;
        let Some(OpenBody::AssetBundle(whole)) = r.body else {
            panic!("not a bundle: {r:?}");
        };
        assert!(!whole.is_delta);
        assert_eq!(whole.emotes, emotes);
        assert!(MAX_RESPONSE_BYTES > whole.encoded_len());

        let refused = b.ui.open_client(&OpenClientRequest {
            network_key: vec![1; 32],
            server_key: skey,
        });
        let mut refused = refused.await.unwrap();
        match refused.next().await {
            Some(Err(CallError::Remote(e))) => assert_eq!(e.message, "network not found"),
            r => panic!("not refused: {r:?}"),
        }
    });
}

fn media(n: u8) -> Listing {
    let id = SwarmId::new(&[n; 32]);
    Listing {
        content: Some(Content::Media(Media {
            mime_type: "video/mp4".into(),
            swarm_uri: Uri::new(id, UriOptions::default()).to_string(),
        })),
    }
}

/// A message, and each nick it mentions, carries the media listing its
/// user is in on a network's directory, and none when there is none.
#[test]
fn messages_carry_the_listings_their_users_view() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (net, _pipes) = owned_network(&owner, &[&alice, &bob], 10).await;
        let net = net.public;
        // aliases that lex as one word
        alice
            .change_network(10, |n| n.alias = "alicia".into())
            .await;
        bob.change_network(10, |n| n.alias = "bobby".into()).await;
        settle().await;
        let a = start(&alice);
        let b = start(&bob);
        let mut events = a.chat.subscribe();
        let s = create_server(&a, &net, "room").await;
        let skey = server_key(&s);
        serving(&mut events, &a.chat, &skey).await;
        let listing_id = a.chat.servers()[0].listing_id.unwrap();
        let by_id = ListingQuery {
            query: Some(Query::Id(listing_id)),
        };
        bob.directory.join(&net, by_id).await.unwrap();
        let media_id = alice.directory.publish(&net, media(7)).await.unwrap();
        sleep_secs(5).await;

        let mut stream =
            b.ui.open_client(&OpenClientRequest {
                network_key: net.clone(),
                server_key: skey.clone(),
            })
            .await
            .unwrap();
        let want = DirectoryRef {
            directory_id: media_id,
            network_key: net.clone(),
            listing: Some(media(7)),
            theme_color: 0,
        };
        let send = |ui: &ChatFrontendClient<Client>, body: &str| {
            ui.client_send_message(&ClientSendMessageRequest {
                network_key: net.clone(),
                server_key: skey.clone(),
                body: body.into(),
            })
        };
        send(&b.ui, "hi alicia").await.unwrap();
        let m = message(&mut stream, "hi alicia").await;
        assert_eq!(m.peer_key, key(&bob));
        assert_eq!(m.viewed_listing, None);
        let nicks = m.entities.unwrap().nicks;
        assert_eq!(nicks.len(), 1, "{nicks:?}");
        assert_eq!(nicks[0].peer_key, key(&alice));
        assert_eq!(nicks[0].viewed_listing, Some(want.clone()));

        send(&a.ui, "hello bobby").await.unwrap();
        let m = message(&mut stream, "hello bobby").await;
        assert_eq!(m.viewed_listing, Some(want));
        let nicks = m.entities.unwrap().nicks;
        assert_eq!(nicks.len(), 1, "{nicks:?}");
        assert_eq!(nicks[0].peer_key, key(&bob));
        assert_eq!(nicks[0].viewed_listing, None);
    });
}

/// A moderator mutes and unmutes a user by alias, with a duration string;
/// the user reads its mute.
#[test]
fn a_moderator_mutes_by_alias() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (net, _pipes) = owned_network(&owner, &[&alice, &bob], 10).await;
        let net = net.public;
        let a = start(&alice);
        let b = start(&bob);
        let mut events = a.chat.subscribe();
        let s = create_server(&a, &net, "room").await;
        let skey = server_key(&s);
        serving(&mut events, &a.chat, &skey).await;
        let bob_alias = alias(&bob, 10).await;

        let mute = |ui: &ChatFrontendClient<Client>, alias: &str, duration: &str| {
            ui.client_mute(&ClientMuteRequest {
                network_key: net.clone(),
                server_key: skey.clone(),
                alias: alias.into(),
                duration: duration.into(),
                message: "spam".into(),
            })
        };
        let send = |ui: &ChatFrontendClient<Client>, body: &str| {
            ui.client_send_message(&ClientSendMessageRequest {
                network_key: net.clone(),
                server_key: skey.clone(),
                body: body.into(),
            })
        };
        let get_mute = |ui: &ChatFrontendClient<Client>| {
            ui.client_get_mute(&ClientGetMuteRequest {
                network_key: net.clone(),
                server_key: skey.clone(),
            })
        };

        let bad = refusal(mute(&a.ui, &bob_alias, "ten minutes").await);
        assert!(bad.starts_with("parsing duration failed"), "{bad}");
        assert_eq!(
            refusal(mute(&a.ui, &bob_alias, "-5m").await),
            "duration must not be negative"
        );
        let unknown = refusal(mute(&a.ui, "nobody", "5m").await);
        assert!(unknown.starts_with("finding peer cert failed"), "{unknown}");
        let alice_alias = alias(&alice, 10).await;
        assert_eq!(
            refusal(mute(&b.ui, &alice_alias, "5m").await),
            "operation requires moderator role"
        );

        mute(&a.ui, &bob_alias, "10m").await.unwrap();
        assert_eq!(
            refusal(send(&b.ui, "still here").await),
            "cannot send messages while muted"
        );
        let m = get_mute(&b.ui).await.unwrap();
        let now = bob.timer.now().unix();
        assert!(
            (now + 599..=now + 600).contains(&m.end_time),
            "{m:?} at {now}"
        );
        assert_eq!(m.message, "spam");
        assert_eq!(get_mute(&a.ui).await.unwrap().end_time, 0);

        a.ui.client_unmute(&ClientUnmuteRequest {
            network_key: net.clone(),
            server_key: skey.clone(),
            alias: bob_alias,
        })
        .await
        .unwrap();
        assert_eq!(get_mute(&b.ui).await.unwrap().end_time, 0);
        sleep_secs(10).await;
        send(&b.ui, "back").await.unwrap();
    });
}

/// Whispers go out by alias and by key; both sides watch their threads
/// and whispers change, list them, mark them read and delete the thread
/// with its whispers.
#[test]
fn whispers_are_sent_watched_listed_and_deleted() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let net = net.public;
        let o = start(&owner);
        let a = start(&alice);
        sleep_secs(1).await;
        let mut ow = o.ui.watch_whispers(&WatchWhispersRequest {}).await.unwrap();
        let mut aw = a.ui.watch_whispers(&WatchWhispersRequest {}).await.unwrap();
        assert!(quiet(&mut ow).await && quiet(&mut aw).await);

        let whisper = |alias: &str, peer_key: Vec<u8>, body: &str| {
            a.ui.whisper(&WhisperRequest {
                network_key: net.clone(),
                server_key: Vec::new(),
                alias: alias.into(),
                peer_key,
                body: body.into(),
            })
        };
        let owner_alias = alias(&owner, 10).await;
        whisper(&owner_alias, Vec::new(), "hi").await.unwrap();
        whisper("", key(&owner), "again").await.unwrap();
        assert_eq!(
            refusal(whisper("", Vec::new(), "to whom").await),
            "whisper subject undefined"
        );
        let unknown = refusal(whisper("nobody", Vec::new(), "hm").await);
        assert!(unknown.starts_with("finding peer cert failed"), "{unknown}");

        // the sender's side
        let delivered = |r: &WatchWhispersResponse| {
            matches!(&r.body, Some(WhisperBody::WhisperUpdate(w))
                if w.state == MessageState::Delivered as i32
                    && w.message.as_ref().unwrap().body == "again")
        };
        let rs = until(&mut aw, 10, "the delivery", delivered).await;
        assert!(rs.iter().all(|r| r.peer_key == key(&owner)), "{rs:?}");
        let thread = rs.iter().find_map(|r| match &r.body {
            Some(WhisperBody::ThreadUpdate(t)) => Some(t.clone()),
            _ => None,
        });
        let thread = thread.unwrap();
        assert_eq!(
            (thread.alias.as_str(), thread.has_unread),
            (owner_alias.as_str(), false)
        );

        // the receiver's side
        let unread = |r: &WatchWhispersResponse| matches!(&r.body, Some(WhisperBody::ThreadUpdate(t)) if t.unread_count == 2);
        let rs = until(&mut ow, 10, "two unread", unread).await;
        assert!(rs.iter().all(|r| r.peer_key == key(&alice)), "{rs:?}");
        let list =
            o.ui.list_whispers(&ListWhispersRequest {
                peer_key: key(&alice),
            })
            .await
            .unwrap();
        let t = list.thread.unwrap();
        assert!(t.has_unread);
        assert_eq!(t.alias, alias(&alice, 10).await);
        let bodies: Vec<&str> = list
            .whispers
            .iter()
            .map(|w| w.message.as_ref().unwrap().body.as_str())
            .collect();
        assert_eq!(bodies, ["hi", "again"]);
        assert!(
            list.whispers
                .iter()
                .all(|w| w.state == MessageState::Unread as i32)
        );
        let missing = o.ui.list_whispers(&ListWhispersRequest {
            peer_key: vec![9; 32],
        });
        assert_eq!(refusal(missing.await), "whisper thread not found");

        // a late watcher reads the threads with their unread counts
        let mut late = o.ui.watch_whispers(&WatchWhispersRequest {}).await.unwrap();
        let r = next(&mut late, 5).await;
        match r.body {
            Some(WhisperBody::ThreadUpdate(t)) => assert_eq!(t.unread_count, 2),
            b => panic!("not a thread: {b:?}"),
        }
        assert!(quiet(&mut late).await);

        o.ui.mark_whispers_read(&MarkWhispersReadRequest {
            peer_key: key(&alice),
        })
        .await
        .unwrap();
        let read = |r: &WatchWhispersResponse| matches!(&r.body, Some(WhisperBody::WhisperUpdate(w)) if w.state == MessageState::Read as i32);
        let mut rs = until(&mut ow, 5, "a read whisper", read).await;
        rs.extend(until(&mut ow, 5, "another read whisper", read).await);
        assert!(
            rs.iter().any(|r| matches!(&r.body,
                Some(WhisperBody::ThreadUpdate(t)) if !t.has_unread && t.unread_count == 0)),
            "{rs:?}"
        );

        let delete = || {
            o.ui.delete_whisper_thread(&DeleteWhisperThreadRequest {
                peer_key: key(&alice),
            })
        };
        delete().await.unwrap();
        let gone = |r: &WatchWhispersResponse| matches!(r.body, Some(WhisperBody::ThreadDelete(_)));
        let rs = until(&mut ow, 5, "the thread's deletion", gone).await;
        let deleted = rs
            .iter()
            .filter(|r| matches!(r.body, Some(WhisperBody::WhisperDelete(_))))
            .count();
        assert_eq!(deleted, 2, "{rs:?}");
        let mut tx = owner.store.read().await.unwrap();
        assert!(WhisperRecords::get_all(&mut tx).await.unwrap().is_empty());
        drop(tx);
        assert_eq!(refusal(delete().await), "whisper thread not found");
        let missing = o.ui.list_whispers(&ListWhispersRequest {
            peer_key: key(&alice),
        });
        assert_eq!(refusal(missing.await), "whisper thread not found");
    });
}

/// The UI's settings: the config, highlights, tags and ignores are set by
/// alias, watched, replaced per peer and removed by key or alias.
#[test]
fn ui_settings_are_set_watched_and_removed() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (net, _pipes) = owned_network(&owner, &[&alice, &bob], 10).await;
        let net = net.public;
        let a = start(&alice);
        let (owner_alias, bob_alias) = (alias(&owner, 10).await, alias(&bob, 10).await);
        let mut w =
            a.ui.watch_ui_config(&WatchUiConfigRequest {})
                .await
                .unwrap();
        assert!(quiet(&mut w).await);
        let body = |r: WatchUiConfigResponse| r.config.unwrap();

        a.ui.set_ui_config(&SetUiConfigRequest {
            ui_config: Some(UiConfig {
                show_time: true,
                max_lines: 250,
                ..Default::default()
            }),
        })
        .await
        .unwrap();
        match body(next(&mut w, 5).await) {
            UiBody::UiConfig(c) => assert_eq!((c.show_time, c.max_lines), (true, 250)),
            b => panic!("not the config: {b:?}"),
        }

        let ignore = |alias: &str, duration: &str| {
            a.ui.ignore(&IgnoreRequest {
                network_key: net.clone(),
                alias: alias.into(),
                duration: duration.into(),
            })
        };
        ignore(&bob_alias, "1h").await.unwrap();
        let first = match body(next(&mut w, 5).await) {
            UiBody::UiConfigIgnore(i) => i,
            b => panic!("not an ignore: {b:?}"),
        };
        let now = alice.timer.now().unix();
        assert_eq!(
            (first.peer_key.clone(), first.alias.as_str()),
            (key(&bob), bob_alias.as_str())
        );
        assert!(
            (now + 3599..=now + 3600).contains(&first.deadline),
            "{first:?}"
        );
        ignore(&bob_alias, "").await.unwrap();
        match body(next(&mut w, 5).await) {
            UiBody::UiConfigIgnore(i) => assert_eq!((i.id, i.deadline), (first.id, 0)),
            b => panic!("not an ignore: {b:?}"),
        }
        let mut tx = alice.store.read().await.unwrap();
        assert_eq!(
            ChatUiConfigIgnores::get_all(&mut tx).await.unwrap().len(),
            1
        );
        drop(tx);
        let bad = refusal(ignore(&bob_alias, "soon").await);
        assert!(bad.starts_with("parsing duration failed"), "{bad}");

        a.ui.highlight(&HighlightRequest {
            network_key: net.clone(),
            alias: owner_alias.clone(),
        })
        .await
        .unwrap();
        match body(next(&mut w, 5).await) {
            UiBody::UiConfigHighlight(h) => assert_eq!(h.peer_key, key(&owner)),
            b => panic!("not a highlight: {b:?}"),
        }
        a.ui.tag(&TagRequest {
            network_key: net.clone(),
            alias: owner_alias.clone(),
            color: "red".into(),
        })
        .await
        .unwrap();
        match body(next(&mut w, 5).await) {
            UiBody::UiConfigTag(t) => {
                assert_eq!((t.peer_key, t.color.as_str()), (key(&owner), "red"))
            }
            b => panic!("not a tag: {b:?}"),
        }

        // a late watcher reads them all
        let mut late =
            a.ui.watch_ui_config(&WatchUiConfigRequest {})
                .await
                .unwrap();
        let mut got = Vec::new();
        for _ in 0..4 {
            got.push(match body(next(&mut late, 5).await) {
                UiBody::UiConfig(_) => "config",
                UiBody::UiConfigHighlight(_) => "highlight",
                UiBody::UiConfigTag(_) => "tag",
                UiBody::UiConfigIgnore(_) => "ignore",
                b => panic!("not a setting: {b:?}"),
            });
        }
        assert_eq!(got, ["config", "highlight", "tag", "ignore"]);
        assert!(quiet(&mut late).await);

        a.ui.unignore(&UnignoreRequest {
            network_key: net.clone(),
            alias: String::new(),
            peer_key: key(&bob),
        })
        .await
        .unwrap();
        match body(next(&mut w, 5).await) {
            UiBody::UiConfigIgnoreDelete(i) => assert_eq!(i.peer_key, key(&bob)),
            b => panic!("not an unignore: {b:?}"),
        }
        a.ui.unhighlight(&UnhighlightRequest {
            network_key: net.clone(),
            alias: owner_alias.clone(),
            peer_key: Vec::new(),
        })
        .await
        .unwrap();
        match body(next(&mut w, 5).await) {
            UiBody::UiConfigHighlightDelete(h) => assert_eq!(h.peer_key, key(&owner)),
            b => panic!("not an unhighlight: {b:?}"),
        }
        let untag = || {
            a.ui.untag(&UntagRequest {
                network_key: net.clone(),
                alias: owner_alias.clone(),
                peer_key: Vec::new(),
            })
        };
        untag().await.unwrap();
        match body(next(&mut w, 5).await) {
            UiBody::UiConfigTagDelete(t) => assert_eq!(t.peer_key, key(&owner)),
            b => panic!("not an untag: {b:?}"),
        }
        assert_eq!(refusal(untag().await), "tag not found");
        assert!(quiet(&mut w).await);
    });
}
