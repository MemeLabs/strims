// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Chat controls on fake nodes (see `strims_testworld`): serving a chat
//! server and the conditions it waits for, the lease, network moves, the
//! listing and its nicks, calls, asset bundles, client streams and a
//! hostile server.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::VecDeque;

use futures_util::StreamExt;
use strims_base::rng::det_rng;
use strims_chat::reader::RoomLimits;
use strims_chat::{
    ADDRESS_SALT, ASSET_FRAME, ASSET_SALT, ChatControl, ChatError, ChatEvent, ChatFailure,
    ClientEvent, ClientStream, Config, EVENT_FRAME, EVENT_SALT, ServerMode, asset_options,
    event_options,
};
use strims_crypto::membership_trusted;
use strims_dao::schema::chat::{ChatEmotes, ChatProfiles, ChatServers, profile_peer_key};
use strims_dao::{Recv, Subscription, Table};
use strims_kv::MemoryKv;
use strims_network::membership::{network_from_invitation_v0, new_invitation_v0};
use strims_ppspp::{SwarmId, WriterOptions};
use strims_proto::chat::v1::message::Entities;
use strims_proto::chat::v1::message::entities::Link;
use strims_proto::chat::v1::{
    AssetBundle, Emote, Message, Room, Server, ServerEvent, server_event,
};
use strims_proto::network::v1::directory::ListingQuery;
use strims_proto::network::v1::directory::listing::{Chat, Content};
use strims_proto::network::v1::directory::listing_query::Query;
use strims_rpc::Timer;
use strims_testworld::*;
use strims_transfer::{RecordWriter, SwarmHost, TransferId};

type Control = ChatControl<MemoryKv, FakeHost, TokioTimer>;
type Stream = ClientStream<MemoryKv, FakeHost, TokioTimer>;

fn chat(node: &Node) -> Control {
    chat_with(node, Config::default())
}

fn chat_with(node: &Node, config: Config) -> Control {
    let c = ChatControl::new(
        node.host.clone(),
        node.store.clone(),
        node.profile.clone(),
        node.network.clone(),
        node.transfers.clone(),
        node.directory.clone(),
        node.timer.clone(),
        node.rng(),
        config,
    );
    let r = c.clone();
    node.spawn(async move { r.run().await });
    c
}

/// A new chat server of `node`'s profile on the network with key
/// `network_key`.
async fn create_server(node: &Node, network_key: &[u8], name: &str) -> Server {
    let key = strims_crypto::generate_key(&mut node.rng());
    let mut tx = node.store.write(node.timer.now()).await.unwrap();
    let id = tx.generate_id().await.unwrap();
    let server = Server {
        id,
        network_key: network_key.to_vec(),
        key: Some(key),
        room: Some(Room {
            name: name.into(),
            css: String::new(),
        }),
        admin_peer_keys: Vec::new(),
    };
    let server = ChatServers::insert(&mut tx, server).await.unwrap();
    tx.commit().await.unwrap();
    server
}

async fn update_server(node: &Node, id: u64, f: impl FnOnce(&mut Server)) {
    let mut tx = node.store.write(node.timer.now()).await.unwrap();
    let mut s = ChatServers::get(&mut tx, id).await.unwrap().unwrap();
    f(&mut s);
    ChatServers::update(&mut tx, s).await.unwrap();
    tx.commit().await.unwrap();
}

async fn create_emote(node: &Node, server_id: u64, name: &str) -> Emote {
    let mut tx = node.store.write(node.timer.now()).await.unwrap();
    let id = tx.generate_id().await.unwrap();
    let e = Emote {
        id,
        server_id,
        name: name.into(),
        enable: true,
        ..Default::default()
    };
    let e = ChatEmotes::insert(&mut tx, e).await.unwrap();
    tx.commit().await.unwrap();
    e
}

fn server_key(s: &Server) -> Vec<u8> {
    s.key.as_ref().unwrap().public.clone()
}

fn by_id(id: u64) -> ListingQuery {
    ListingQuery {
        query: Some(Query::Id(id)),
    }
}

fn mode(c: &Control, key: &[u8]) -> Option<ServerMode> {
    c.servers()
        .into_iter()
        .find(|s| s.server_key == key)
        .map(|s| s.mode)
}

fn serving(c: &Control, key: &[u8]) -> bool {
    mode(c, key) == Some(ServerMode::Serving)
}

/// Whether network `network_id`'s view on `n` lists the chat with `key`
/// named `name`.
fn chat_listed(n: &Node, network_id: u64, key: &[u8], name: &str) -> bool {
    n.directory.listings(network_id).iter().any(|l| {
        l.listing.content
            == Some(Content::Chat(Chat {
                key: key.to_vec(),
                name: name.into(),
            }))
    })
}

/// The next event `f` picks within `secs` seconds.
async fn wait_event(
    events: &mut Subscription<ChatEvent>,
    secs: u64,
    what: &str,
    f: impl Fn(&ChatEvent) -> bool,
) -> ChatEvent {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(secs);
    loop {
        match tokio::time::timeout_at(deadline, events.next()).await {
            Ok(Some(Recv::Event(e))) if f(&e) => return (*e).clone(),
            Ok(Some(_)) => {}
            _ => panic!("timed out waiting for {what}"),
        }
    }
}

/// A client stream, with what it yielded sorted into messages and
/// bundles.
struct Reader {
    stream: Stream,
    messages: VecDeque<Message>,
    bundles: VecDeque<AssetBundle>,
}

impl Reader {
    fn new(stream: Stream) -> Reader {
        Reader {
            stream,
            messages: VecDeque::new(),
            bundles: VecDeque::new(),
        }
    }

    /// Takes the stream's next item within `secs` seconds. False when
    /// none came.
    async fn pump(&mut self, secs: u64) -> bool {
        let wait = std::time::Duration::from_secs(secs);
        match tokio::time::timeout(wait, self.stream.next()).await {
            Ok(Some(ClientEvent::Assets(b))) => self.bundles.push_back(b),
            Ok(Some(ClientEvent::Events(events))) => {
                assert!(!events.is_empty());
                for e in events {
                    match e.body {
                        Some(server_event::Body::Message(m)) => self.messages.push_back(m),
                        None => panic!("an event without a message"),
                    }
                }
            }
            Ok(None) => panic!("the stream ended"),
            Err(_) => return false,
        }
        true
    }

    async fn message(&mut self, secs: u64) -> Message {
        loop {
            if let Some(m) = self.messages.pop_front() {
                return m;
            }
            assert!(self.pump(secs).await, "no message came");
        }
    }

    async fn bundle(&mut self, secs: u64) -> AssetBundle {
        loop {
            if let Some(b) = self.bundles.pop_front() {
                return b;
            }
            assert!(self.pump(secs).await, "no bundle came");
        }
    }

    /// Takes what comes within `secs` seconds of quiet.
    async fn drain(&mut self, secs: u64) {
        while self.pump(secs).await {}
    }
}

/// A network the owner runs with members `alice` and `bob`, and a chat
/// server alice hosts on it: the network key and the server, once it
/// serves and its listing is published.
struct Room3 {
    owner: Node,
    alice: Node,
    bob: Node,
    key: Vec<u8>,
    alice_chat: Control,
    bob_chat: Control,
    server: Server,
    listing_id: u64,
}

async fn room3(config: Config) -> Room3 {
    let world = World::default();
    let owner = node_in(&world, 1).await;
    let alice = node_in(&world, 2).await;
    let bob = node_in(&world, 3).await;
    let (key, _pipes) = owned_network(&owner, &[&alice, &bob], 10).await;
    let key = key.public;
    let alice_chat = chat_with(&alice, config.clone());
    let bob_chat = chat_with(&bob, config);
    let mut events = alice_chat.subscribe();
    let server = create_server(&alice, &key, "room").await;
    let skey = server_key(&server);
    let listing = wait_event(
        &mut events,
        30,
        "the listing",
        |e| matches!(e, ChatEvent::ListingChanged { server_key, .. } if *server_key == skey),
    )
    .await;
    let ChatEvent::ListingChanged { listing_id, .. } = listing else {
        unreachable!()
    };
    sleep_secs(3).await;
    Room3 {
        owner,
        alice,
        bob,
        key,
        alice_chat,
        bob_chat,
        server,
        listing_id,
    }
}

/// A server whose record is on a network the node hasn't joined, or whose
/// membership isn't trusted yet, waits; it serves once both hold, with
/// its swarms, transfers, address and listing.
#[test]
fn serving_waits_for_the_network_and_trust() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (root_key, _pipes) = owned_network(&owner, &[], 10).await;
        let key = root_key.public.clone();
        let alice_chat = chat(&alice);
        let mut events = alice_chat.subscribe();
        let server = create_server(&alice, &key, "room").await;
        let skey = server_key(&server);
        sleep_secs(5).await;
        assert_eq!(mode(&alice_chat, &skey), Some(ServerMode::Idle));
        assert_eq!(world.address(&skey, ADDRESS_SALT), None);

        // invited while no process of the owner runs: joined, but the
        // membership can't be renewed, so it isn't trusted
        let mut rng = det_rng(99);
        let owner_cert = owner.read_network(10).await.certificate.unwrap();
        let invitation = new_invitation_v0(&owner.profile, &owner_cert, T0, &mut rng).unwrap();
        let joined =
            network_from_invitation_v0(10, &invitation, &alice.profile, "", T0, &mut rng).unwrap();
        owner.stop();
        alice.write_network(joined).await;
        settle().await;
        assert!(alice.host.net(&key).is_some());
        let cert = alice.read_network(10).await.certificate.unwrap();
        assert!(!membership_trusted(&cert, alice.timer.now()));
        assert_eq!(mode(&alice_chat, &skey), Some(ServerMode::Idle));
        assert_eq!(world.address(&skey, ADDRESS_SALT), None);

        alice
            .write_network(signed_member(&root_key, &alice, 10))
            .await;
        wait_event(&mut events, 60, "serving", |e| {
            matches!(e, ChatEvent::ServeStarted { server_key, network_key, .. }
                if *server_key == skey && *network_key == key)
        })
        .await;
        assert_eq!(
            world.address(&skey, ADDRESS_SALT).map(|a| a.host),
            Some(alice.id)
        );
        let swarm = SwarmId::new(&skey);
        assert_eq!(alice.host.swarms_of(&swarm), (2, 0));
        for salt in [EVENT_SALT, ASSET_SALT] {
            assert!(
                alice
                    .transfers
                    .is_published(TransferId::new(&swarm, salt), alice.net(&key))
            );
        }
        // the listing waits for a directory server: another process of
        // the owner
        sleep_secs(10).await;
        assert!(alice_chat.metrics().listing_errors > 0);
        let owner = node_sharing(&world, 5, &owner);
        wait_for(120, "the listing", || {
            chat_listed(&owner, 10, &skey, "room")
        })
        .await;
        let status = alice_chat.servers();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].server_id, Some(server.id));
        assert!(status[0].listing_id.is_some());
        assert_eq!(alice_chat.metrics().serves, 1);
    });
}

/// Of two processes sharing a profile's store, one serves its chat
/// server; when it ends, the other takes over once the lease expires.
#[test]
fn one_process_of_a_shared_store_serves() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let first = node_in(&world, 2).await;
        let second = node_sharing(&world, 4, &first);
        let (key, _pipes) = owned_network(&owner, &[&first], 10).await;
        let key = key.public;
        settle().await;
        let first_chat = chat(&first);
        let second_chat = chat(&second);
        let server = create_server(&first, &key, "room").await;
        let skey = server_key(&server);
        wait_for(30, "a process to serve", || {
            serving(&first_chat, &skey) || serving(&second_chat, &skey)
        })
        .await;
        sleep_secs(30).await;
        let serving_now = [serving(&first_chat, &skey), serving(&second_chat, &skey)];
        assert_eq!(
            serving_now.iter().filter(|s| **s).count(),
            1,
            "{serving_now:?}"
        );
        let (server_node, other, other_chat) = if serving_now[0] {
            (&first, &second, &second_chat)
        } else {
            (&second, &first, &first_chat)
        };
        assert_eq!(mode(other_chat, &skey), Some(ServerMode::Idle));
        assert_eq!(
            world.address(&skey, ADDRESS_SALT).map(|a| a.host),
            Some(server_node.id)
        );

        server_node.stop();
        wait_for(60, "the other process to serve", || {
            serving(other_chat, &skey)
        })
        .await;
        assert_eq!(
            world.address(&skey, ADDRESS_SALT).map(|a| a.host),
            Some(other.id)
        );
        wait_for(60, "the listing", || chat_listed(&owner, 10, &skey, "room")).await;
    });
}

/// A server whose record moves to another network stops on the first and
/// serves on the second, once the first's swarms and transfers are gone.
#[test]
fn a_moved_server_restarts_on_its_new_network() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (first, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let (second, _) = owned_network(&alice, &[], 11).await;
        let (first, second) = (first.public, second.public);
        let alice_chat = chat(&alice);
        let mut events = alice_chat.subscribe();
        let server = create_server(&alice, &first, "room").await;
        let skey = server_key(&server);
        wait_for(30, "the listing on the first network", || {
            chat_listed(&owner, 10, &skey, "room")
        })
        .await;

        update_server(&alice, server.id, |s| s.network_key = second.clone()).await;
        wait_event(
            &mut events,
            30,
            "the stop on the first network",
            |e| matches!(e, ChatEvent::ServeStopped { network_key, .. } if *network_key == first),
        )
        .await;
        wait_event(
            &mut events,
            30,
            "the start on the second network",
            |e| matches!(e, ChatEvent::ServeStarted { network_key, .. } if *network_key == second),
        )
        .await;
        let swarm = SwarmId::new(&skey);
        assert_eq!(alice.host.swarms_of(&swarm), (2, 0));
        for salt in [EVENT_SALT, ASSET_SALT] {
            let t = TransferId::new(&swarm, salt);
            assert!(alice.transfers.is_published(t, alice.net(&second)));
            assert!(!alice.transfers.is_published(t, alice.net(&first)));
        }
        let status = alice_chat.servers();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].network_key, second);
        wait_for(30, "the listing to move", || {
            chat_listed(&alice, 11, &skey, "room") && !chat_listed(&owner, 10, &skey, "room")
        })
        .await;
        assert_eq!(
            alice_chat.open_client(&first, &skey).err(),
            Some(ChatFailure::OtherNetwork)
        );
    });
}

/// When the directory server's process ends and another takes over, the
/// new directory starts empty and the chat server publishes its listing
/// again.
#[test]
fn the_listing_is_republished_after_the_directory_restarts() {
    run(async {
        let world = World::default();
        let first = node_in(&world, 1).await;
        let second = node_sharing(&world, 4, &first);
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&first, &[&alice], 10).await;
        let key = key.public;
        let alice_chat = chat(&alice);
        let server = create_server(&alice, &key, "room").await;
        let skey = server_key(&server);
        wait_for(30, "the listing", || chat_listed(&alice, 10, &skey, "room")).await;
        let publishes = alice_chat.metrics().listing_publishes;
        let (directory, other) = if first.serving(&key) {
            (&first, &second)
        } else {
            (&second, &first)
        };

        directory.stop_directory();
        wait_for(60, "the other directory", || other.serving(&key)).await;
        wait_for(180, "the listing again", || {
            chat_listed(other, 10, &skey, "room") && chat_listed(&alice, 10, &skey, "room")
        })
        .await;
        assert!(alice_chat.metrics().listing_publishes > publishes);
        let id = alice_chat.servers()[0].listing_id.unwrap();
        assert_eq!(
            other
                .directory
                .listing(10, &by_id(id))
                .unwrap()
                .listing
                .content,
            Some(Content::Chat(Chat {
                key: skey.clone(),
                name: "room".into()
            }))
        );
    });
}

/// The server's nicks are its listing's users: a member's alias is a nick
/// mention once it joins, follows its renames, and stops being one when
/// it parts.
#[test]
fn nicks_follow_the_listing_users() {
    run(async {
        let r = room3(Config::default()).await;
        let skey = server_key(&r.server);
        let bob_key = r.bob.profile_key().public.clone();
        let owner_key = r.owner.profile_key().public.clone();
        // the profiles' names don't lex as one word
        r.bob.change_network(10, |n| n.alias = "bobby".into()).await;
        settle().await;
        let mut reader = Reader::new(r.alice_chat.open_client(&r.key, &skey).unwrap());
        let nicks = async |reader: &mut Reader, body: &str| {
            r.alice_chat
                .send_message(&r.key, &skey, body)
                .await
                .unwrap();
            let m = reader.message(5).await;
            assert_eq!(m.body, body);
            let nicks: Vec<(String, Vec<u8>)> = m
                .entities
                .unwrap()
                .nicks
                .into_iter()
                .map(|n| (n.nick, n.peer_key))
                .collect();
            nicks
        };

        assert!(nicks(&mut reader, "hi bobby").await.is_empty());
        r.bob
            .directory
            .join(&r.key, by_id(r.listing_id))
            .await
            .unwrap();
        r.owner
            .directory
            .join(&r.key, by_id(r.listing_id))
            .await
            .unwrap();
        sleep_secs(5).await;
        assert_eq!(
            nicks(&mut reader, "hi bobby").await,
            [("bobby".to_owned(), bob_key.clone())]
        );

        r.owner
            .change_network(10, |n| n.alias = "olly".into())
            .await;
        settle().await;
        assert_eq!(
            nicks(&mut reader, "hi olly").await,
            [("olly".to_owned(), owner_key)]
        );

        r.bob.directory.part(&r.key, r.listing_id).await.unwrap();
        sleep_secs(5).await;
        assert!(nicks(&mut reader, "bye bobby").await.is_empty());
    });
}

/// Calls reach the server from its own node and from a member's, as the
/// caller's peer key and alias, and are refused as the server refuses
/// them.
#[test]
fn calls_reach_the_server_locally_and_remotely() {
    run(async {
        let r = room3(Config::default()).await;
        let skey = server_key(&r.server);
        let alice_key = r.alice.profile_key().public.clone();
        let bob_key = r.bob.profile_key().public.clone();
        let mut reader = Reader::new(r.alice_chat.open_client(&r.key, &skey).unwrap());

        r.bob_chat
            .send_message(&r.key, &skey, "hello")
            .await
            .unwrap();
        let m = reader.message(5).await;
        assert_eq!((m.body.as_str(), &m.peer_key), ("hello", &bob_key));
        assert_eq!(m.nick, "node-3");
        r.alice_chat
            .send_message(&r.key, &skey, "hi")
            .await
            .unwrap();
        let m = reader.message(5).await;
        assert_eq!((m.body.as_str(), &m.peer_key), ("hi", &alice_key));
        assert_eq!(r.alice_chat.metrics().local_calls, 1);
        assert_eq!(r.bob_chat.metrics().remote_calls, 1);

        let long = "x".repeat(3000);
        assert_eq!(
            r.bob_chat.send_message(&r.key, &skey, &long).await,
            Err(ChatFailure::Refused(ChatError::MessageTooLong))
        );
        assert_eq!(
            r.bob_chat.mute(&r.key, &skey, &alice_key, 60, "no").await,
            Err(ChatFailure::Refused(ChatError::NotModerator))
        );

        // the owner mutes bob
        r.alice_chat
            .mute(&r.key, &skey, &bob_key, 60, "spam")
            .await
            .unwrap();
        assert_eq!(
            r.bob_chat.send_message(&r.key, &skey, "still here").await,
            Err(ChatFailure::Refused(ChatError::Muted))
        );
        let mute = r.bob_chat.get_mute(&r.key, &skey).await.unwrap();
        assert!(mute.end_time > r.bob.timer.now().unix(), "{mute:?}");
        assert_eq!(mute.message, "spam");
        assert_eq!(
            r.alice_chat.get_mute(&r.key, &skey).await.unwrap().end_time,
            0
        );
        let profile = {
            let mut tx = r.alice.store.read().await.unwrap();
            let k = profile_peer_key(r.server.id, &bob_key);
            ChatProfiles::index_get(&mut tx, &ChatProfiles::BY_PEER_KEY, &k)
                .await
                .unwrap()
                .unwrap()
        };
        assert_eq!(profile.mutes.len(), 1);
        assert_eq!(profile.mutes[0].message, "spam");
        assert_eq!(profile.mutes[0].moderator_peer_key, alice_key);

        r.alice_chat.unmute(&r.key, &skey, &bob_key).await.unwrap();
        assert_eq!(
            r.bob_chat.get_mute(&r.key, &skey).await.unwrap().end_time,
            0
        );
        sleep_secs(10).await;
        r.bob_chat
            .send_message(&r.key, &skey, "back")
            .await
            .unwrap();
        assert_eq!(reader.message(5).await.body, "back");

        // an admin moderates remotely
        update_server(&r.alice, r.server.id, |s| {
            s.admin_peer_keys.push(bob_key.clone())
        })
        .await;
        sleep_secs(1).await;
        r.bob_chat
            .mute(&r.key, &skey, &alice_key, 60, "")
            .await
            .unwrap();
        assert_eq!(
            r.alice_chat.send_message(&r.key, &skey, "hm").await,
            Err(ChatFailure::Refused(ChatError::Muted))
        );
        r.bob_chat.unmute(&r.key, &skey, &alice_key).await.unwrap();
        sleep_secs(10).await;
        r.alice_chat
            .send_message(&r.key, &skey, "ok")
            .await
            .unwrap();
        assert_eq!(reader.message(5).await.body, "ok");

        // a running combo refuses a repeat
        create_emote(&r.alice, r.server.id, "PEPE").await;
        sleep_secs(1).await;
        r.bob_chat
            .send_message(&r.key, &skey, "PEPE")
            .await
            .unwrap();
        r.alice_chat
            .send_message(&r.key, &skey, "PEPE")
            .await
            .unwrap();
        assert_eq!(
            r.bob_chat.send_message(&r.key, &skey, "PEPE").await,
            Err(ChatFailure::Refused(ChatError::ComboDuplicate))
        );
        reader.message(5).await;
        let m = reader.message(5).await;
        assert_eq!(m.entities.unwrap().emotes[0].combo, 2);

        // too fast
        sleep_secs(10).await;
        let mut refused = None;
        for i in 0..10 {
            if let Err(e) = r
                .bob_chat
                .send_message(&r.key, &skey, &format!("m{i}"))
                .await
            {
                refused = Some(e);
                break;
            }
        }
        assert_eq!(refused, Some(ChatFailure::Refused(ChatError::TooFast)));
        let server = r.alice_chat.servers()[0].server.clone().unwrap();
        assert_eq!(server.refused_too_fast, 1, "{server:?}");
    });
}

/// Asset changes in the store reach client streams as deltas after the
/// debounce, and a forced sync as a unified bundle.
#[test]
fn asset_changes_reach_streams_as_deltas() {
    run(async {
        let r = room3(Config::default()).await;
        let skey = server_key(&r.server);
        let mut reader = Reader::new(r.alice_chat.open_client(&r.key, &skey).unwrap());
        let b = reader.bundle(5).await;
        assert!(!b.is_delta);
        assert_eq!(b.room.unwrap().name, "room");
        assert!(b.emotes.is_empty());

        let a = create_emote(&r.alice, r.server.id, "A").await;
        let bb = create_emote(&r.alice, r.server.id, "B").await;
        let d = reader.bundle(5).await;
        assert!(d.is_delta);
        let names: Vec<&str> = d.emotes.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["A", "B"]);

        {
            let mut tx = r.alice.store.write(r.alice.timer.now()).await.unwrap();
            ChatEmotes::update(
                &mut tx,
                Emote {
                    enable: false,
                    ..a.clone()
                },
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        }
        let d = reader.bundle(5).await;
        assert!(d.is_delta);
        assert_eq!(d.removed_ids, [a.id]);
        assert!(d.emotes.is_empty());

        {
            let mut tx = r.alice.store.write(r.alice.timer.now()).await.unwrap();
            ChatEmotes::delete(&mut tx, bb.id).await.unwrap();
            tx.commit().await.unwrap();
        }
        let d = reader.bundle(5).await;
        assert_eq!(d.removed_ids, [bb.id]);

        update_server(&r.alice, r.server.id, |s| {
            s.room.as_mut().unwrap().name = "lobby".into()
        })
        .await;
        let d = reader.bundle(5).await;
        assert!(d.is_delta);
        assert_eq!(d.room.unwrap().name, "lobby");
        wait_for(30, "the renamed listing", || {
            chat_listed(&r.owner, 10, &skey, "lobby") && !chat_listed(&r.owner, 10, &skey, "room")
        })
        .await;

        r.alice_chat.sync_assets(r.server.id, true).unwrap();
        let u = reader.bundle(5).await;
        assert!(!u.is_delta);
        assert_eq!(u.room.unwrap().name, "lobby");
        assert!(u.emotes.is_empty());
        assert_eq!(r.alice_chat.metrics().asset_resets, 1);
        assert_eq!(
            r.alice_chat.sync_assets(r.server.id + 1000, true),
            Err(ChatFailure::NoServer)
        );
    });
}

/// A stream opened late reads the asset stream from its unified bundle
/// on, then the recent events; a member reads through its own swarms.
#[test]
fn a_late_stream_gets_the_unified_bundle_then_deltas_and_recent_events() {
    run(async {
        let r = room3(Config::default()).await;
        let skey = server_key(&r.server);
        create_emote(&r.alice, r.server.id, "A").await;
        sleep_secs(1).await;
        create_emote(&r.alice, r.server.id, "B").await;
        sleep_secs(1).await;
        for body in ["one", "two"] {
            r.alice_chat
                .send_message(&r.key, &skey, body)
                .await
                .unwrap();
        }
        sleep_secs(1).await;

        let mut events = r.bob_chat.subscribe();
        let mut reader = Reader::new(r.bob_chat.open_client(&r.key, &skey).unwrap());
        let u = reader.bundle(5).await;
        assert!(!u.is_delta);
        assert!(u.emotes.is_empty());
        let names: Vec<String> = [reader.bundle(5).await, reader.bundle(5).await]
            .iter()
            .flat_map(|b| {
                assert!(b.is_delta);
                b.emotes.iter().map(|e| e.name.clone())
            })
            .collect();
        assert_eq!(names, ["A", "B"]);
        assert_eq!(reader.message(5).await.body, "one");
        assert_eq!(reader.message(5).await.body, "two");
        wait_event(
            &mut events,
            5,
            "reading",
            |e| matches!(e, ChatEvent::ReadStarted { server_key, .. } if *server_key == skey),
        )
        .await;
        assert_eq!(r.bob.host.swarms_of(&SwarmId::new(&skey)), (0, 2));

        r.bob_chat
            .send_message(&r.key, &skey, "three")
            .await
            .unwrap();
        assert_eq!(reader.message(5).await.body, "three");
    });
}

/// A room's swarms stay while streams read it and for the linger after
/// the last, and a stream opened within the linger finds them; streams
/// per room are bounded.
#[test]
fn streams_share_a_rooms_swarms_until_the_linger_ends() {
    run(async {
        let r = room3(Config::default()).await;
        let skey = server_key(&r.server);
        let swarm = SwarmId::new(&skey);
        let mut events = r.bob_chat.subscribe();
        let a = r.bob_chat.open_client(&r.key, &skey).unwrap();
        let b = r.bob_chat.open_client(&r.key, &skey).unwrap();
        wait_event(&mut events, 5, "reading", |e| {
            matches!(e, ChatEvent::ReadStarted { .. })
        })
        .await;
        assert_eq!(r.bob.host.swarms_of(&swarm), (0, 2));
        assert_eq!(r.bob_chat.servers()[0].streams, 2);

        drop(a);
        sleep_secs(60).await;
        assert_eq!(r.bob.host.swarms_of(&swarm), (0, 2));
        drop(b);
        sleep_secs(29).await;
        assert_eq!(r.bob.host.swarms_of(&swarm), (0, 2));
        let again = r.bob_chat.open_client(&r.key, &skey).unwrap();
        sleep_secs(60).await;
        assert_eq!(r.bob.host.swarms_of(&swarm), (0, 2));
        assert!(drain(&mut events).is_empty());

        drop(again);
        sleep_secs(29).await;
        assert_eq!(r.bob.host.swarms_of(&swarm), (0, 2));
        sleep_secs(2).await;
        assert_eq!(r.bob.host.swarms_of(&swarm), (0, 0));
        let got = drain(&mut events);
        assert!(
            got.iter().any(
                |e| matches!(e, ChatEvent::ReadStopped { server_key, .. } if *server_key == skey)
            ),
            "{got:?}"
        );
        assert!(r.bob_chat.servers().is_empty());
        for salt in [EVENT_SALT, ASSET_SALT] {
            assert!(r.bob.transfers.find(&swarm, salt).is_none());
        }

        let streams: Vec<Stream> = (0..16)
            .map(|_| r.bob_chat.open_client(&r.key, &skey).unwrap())
            .collect();
        assert_eq!(
            r.bob_chat.open_client(&r.key, &skey).err(),
            Some(ChatFailure::TooManyStreams)
        );
        assert_eq!(
            r.bob_chat.open_client(b"other network", &skey).err(),
            Some(ChatFailure::NoNetwork)
        );
        drop(streams);
    });
}

/// A server's oversized, malformed and over-bound records are dropped
/// and counted by its clients, which go on with the records after them.
#[test]
fn a_hostile_servers_records_are_dropped() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let eve = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (key, _pipes) = owned_network(&owner, &[&eve, &bob], 10).await;
        let key = key.public;

        let server = strims_crypto::generate_key(&mut eve.rng());
        let skey = server.public.clone();
        let (_, _, events) = eve
            .host
            .create_swarm(WriterOptions {
                swarm_options: event_options(&skey),
                key: server.clone(),
            })
            .await
            .unwrap();
        let (_, _, assets) = eve
            .host
            .create_swarm(WriterOptions {
                swarm_options: asset_options(&skey),
                key: server,
            })
            .await
            .unwrap();
        let mut events = RecordWriter::new(events, EVENT_FRAME, 1 << 20).unwrap();
        let mut assets = RecordWriter::new(assets, ASSET_FRAME, 1 << 20).unwrap();
        let message = |body: String, entities: Option<Entities>| ServerEvent {
            body: Some(server_event::Body::Message(Message {
                body,
                entities,
                ..Default::default()
            })),
        };
        events
            .write(&message("x".repeat(100 * 1024), None))
            .await
            .unwrap();
        events.write(&ServerEvent { body: None }).await.unwrap();
        let links = Entities {
            links: vec![Link::default(); 2000],
            ..Default::default()
        };
        events
            .write(&message("links".into(), Some(links)))
            .await
            .unwrap();
        events.write(&message("hi".into(), None)).await.unwrap();
        let emote = |id: u64| Emote {
            id,
            name: format!("e{id}"),
            enable: true,
            ..Default::default()
        };
        assets
            .write(&AssetBundle {
                is_delta: true,
                emotes: vec![emote(1)],
                ..Default::default()
            })
            .await
            .unwrap();
        assets
            .write(&AssetBundle {
                emotes: (1..=5).map(emote).collect(),
                ..Default::default()
            })
            .await
            .unwrap();

        let config = Config {
            room: RoomLimits {
                max_emotes: 2,
                ..RoomLimits::default()
            },
            ..Config::default()
        };
        let bob_chat = chat_with(&bob, config);
        let mut reader = Reader::new(bob_chat.open_client(&key, &skey).unwrap());
        let b = reader.bundle(5).await;
        assert!(!b.is_delta);
        assert_eq!(b.emotes.len(), 2);
        assert_eq!(reader.message(5).await.body, "hi");
        reader.drain(2).await;
        assert!(reader.messages.is_empty() && reader.bundles.is_empty());
        let m = reader.stream.metrics();
        assert_eq!(m.events.events, 1, "{m:?}");
        assert_eq!(m.events.empty_events, 1, "{m:?}");
        assert_eq!(m.events.entity_overflows, 1, "{m:?}");
        assert_eq!(m.assets.deltas_dropped, 1, "{m:?}");
        assert_eq!(m.assets.emotes_dropped, 3, "{m:?}");

        // the stream goes on with later records
        events.write(&message("again".into(), None)).await.unwrap();
        assert_eq!(reader.message(5).await.body, "again");
    });
}
