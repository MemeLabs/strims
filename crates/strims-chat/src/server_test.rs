// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use strims_base::rng::det_rng;
use strims_proto::chat::v1::message::Entities;
use strims_proto::chat::v1::message::entities::{Bounds, Nick};
use strims_proto::types::Certificate;
use strims_proto::types::certificate::ParentOneof;

use super::*;

const T0: Time = Time::from_unix(1_700_000_000, 250_000_000);
const OWNER: &[u8] = b"owner";
const ADMIN: &[u8] = b"admin";

fn record() -> strims_proto::chat::v1::Server {
    strims_proto::chat::v1::Server {
        id: 77,
        network_key: vec![9; 32],
        admin_peer_keys: vec![ADMIN.to_vec()],
        ..Default::default()
    }
}

fn caller(peer_key: &[u8], alias: &str) -> Caller {
    Caller::from_certificate(&Certificate {
        key: vec![peer_key.first().copied().unwrap_or(0); 32],
        parent_oneof: Some(ParentOneof::Parent(Box::new(Certificate {
            key: peer_key.to_vec(),
            subject: alias.into(),
            ..Certificate::default()
        }))),
        ..Certificate::default()
    })
    .expect("a host certificate under a membership")
}

fn user(peer_key: &[u8], alias: &str) -> User {
    User {
        id: 0,
        alias: alias.into(),
        peer_key: peer_key.to_vec(),
    }
}

fn emote(name: &str, enable: bool) -> Emote {
    Emote {
        name: name.into(),
        enable,
        ..Emote::default()
    }
}

fn modifier(name: &str, internal: bool, proc_chance: f64) -> Modifier {
    Modifier {
        name: name.into(),
        internal,
        proc_chance,
        ..Modifier::default()
    }
}

fn server() -> Server {
    let mut s = Server::new(&record(), OWNER, ServerLimits::default(), det_rng(1));
    s.sync_assets(
        &[
            emote("PEPE", true),
            emote("Kappa", true),
            emote("OFF", false),
        ],
        &[modifier("wide", false, 0.0)],
        &[Tag {
            name: "nsfw".into(),
            ..Tag::default()
        }],
    );
    s
}

/// The messages the server queued, failing on any other action.
fn messages(s: &mut Server) -> Vec<Message> {
    std::iter::from_fn(|| s.poll_action())
        .map(|a| match a {
            ServerAction::Event(ServerEvent {
                body: Some(server_event::Body::Message(m)),
            }) => m,
            other => panic!("not a message: {other:?}"),
        })
        .collect()
}

/// The one message the server queued.
fn message(s: &mut Server) -> Message {
    let mut m = messages(s);
    assert_eq!(m.len(), 1);
    m.remove(0)
}

/// The profiles the server queued, failing on any other action.
fn profiles(s: &mut Server) -> Vec<Profile> {
    std::iter::from_fn(|| s.poll_action())
        .map(|a| match a {
            ServerAction::SaveProfile(p) => p,
            other => panic!("not a profile: {other:?}"),
        })
        .collect()
}

fn mute_req(peer_key: &[u8], duration_secs: u32, message: &str) -> MuteRequest {
    MuteRequest {
        peer_key: peer_key.to_vec(),
        duration_secs,
        message: message.into(),
    }
}

fn unmute_req(peer_key: &[u8]) -> UnmuteRequest {
    UnmuteRequest {
        peer_key: peer_key.to_vec(),
    }
}

fn entities(m: &Message) -> &Entities {
    m.entities.as_ref().unwrap()
}

#[test]
fn a_message_carries_the_caller_the_server_time_and_its_entities() {
    let mut p = Patterns::new();
    let mut s = server();
    s.send_message(T0, &caller(b"alice", "Alice"), "PEPE:wide nsfw OFF", &mut p)
        .unwrap();
    let m = message(&mut s);
    assert_eq!(m.server_time, T0.unix_millis());
    assert_eq!(m.peer_key, b"alice");
    assert_eq!(m.nick, "Alice");
    assert_eq!(m.body, "PEPE:wide nsfw OFF");
    let e = entities(&m);
    assert_eq!(e.emotes.len(), 1, "disabled emotes aren't parsed");
    assert_eq!(e.emotes[0].name, "PEPE");
    assert_eq!(e.emotes[0].modifiers, ["wide"]);
    assert!(!e.emotes[0].can_combo);
    assert_eq!(e.tags.len(), 1);
    assert_eq!(s.metrics().messages, 1);
}

#[test]
fn the_owner_and_admins_moderate() {
    let mut s = server();
    assert!(s.is_moderator(OWNER));
    assert!(s.is_moderator(ADMIN));
    assert!(!s.is_moderator(b"alice"));

    let mut r = record();
    r.admin_peer_keys = vec![b"alice".to_vec()];
    s.set_record(&r);
    assert!(s.is_moderator(OWNER), "the owner moderates without admin");
    assert!(s.is_moderator(b"alice"));
    assert!(!s.is_moderator(ADMIN));
}

#[test]
fn moderation_requires_a_moderator() {
    let mut s = server();
    let alice = caller(b"alice", "alice");
    assert_eq!(
        s.mute(T0, &alice, &mute_req(b"bob", 60, ""), None, 1),
        Err(ChatError::NotModerator)
    );
    assert_eq!(
        s.unmute(T0, &alice, &unmute_req(b"bob"), None),
        Err(ChatError::NotModerator)
    );
    assert_eq!(s.poll_action(), None);
    assert_eq!(s.metrics().refused_moderation, 2);
    assert_eq!(s.check_moderation(&caller(ADMIN, "admin"), b"bob"), Ok(()));
}

#[test]
fn a_mute_creates_the_profile_and_refuses_sends_until_it_ends() {
    let mut p = Patterns::new();
    let mut s = server();
    s.reset_nicks(&[user(b"bob", "Bob")]);
    let owner = caller(OWNER, "owner");
    let bob = caller(b"bob", "Bob");

    s.mute(T0, &owner, &mute_req(b"bob", 60, "calm down"), None, 5)
        .unwrap();
    let saved = profiles(&mut s);
    assert_eq!(
        saved,
        [Profile {
            id: 5,
            server_id: 77,
            peer_key: b"bob".to_vec(),
            alias: "Bob".into(),
            mutes: vec![profile::Mute {
                created_at: T0.unix(),
                duration_secs: 60,
                message: "calm down".into(),
                moderator_peer_key: OWNER.to_vec(),
            }],
            mute_deadline: T0.unix() + 60,
        }]
    );

    assert_eq!(
        s.send_message(T0, &bob, "hi", &mut p),
        Err(ChatError::Muted)
    );
    assert_eq!(
        s.get_mute(T0, &bob),
        GetMuteResponse {
            end_time: T0.unix() + 60,
            message: "calm down".into(),
        }
    );
    assert_eq!(
        s.get_mute(T0, &caller(b"alice", "alice")),
        GetMuteResponse::default()
    );

    // The deadline is whole seconds: T0 has a fraction, so the mute ends
    // just under 60 s later.
    let end = Time::from_unix(T0.unix() + 60, 0);
    assert_eq!(
        s.send_message(end - Duration::NANOSECOND, &bob, "hi", &mut p),
        Err(ChatError::Muted)
    );
    s.send_message(end, &bob, "hi", &mut p).unwrap();
    assert_eq!(message(&mut s).body, "hi");
    assert_eq!(s.get_mute(end, &bob), GetMuteResponse::default());
    assert_eq!(s.metrics().refused_muted, 2);
}

#[test]
fn an_unmute_ends_the_mute_and_saves_the_deadline() {
    let mut p = Patterns::new();
    let mut s = server();
    let admin = caller(ADMIN, "admin");
    let bob = caller(b"bob", "bob");

    s.mute(T0, &admin, &mute_req(b"bob", 3600, ""), None, 5)
        .unwrap();
    let muted = profiles(&mut s).remove(0);
    assert!(muted.mutes.is_empty(), "a mute without a message adds none");
    assert_eq!(muted.mute_deadline, T0.unix() + 3600);
    assert_eq!(s.get_mute(T0, &bob).message, "");

    let later = T0 + Duration::SECOND * 10;
    s.unmute(later, &admin, &unmute_req(b"bob"), Some(muted.clone()))
        .unwrap();
    assert_eq!(
        profiles(&mut s),
        [Profile {
            mute_deadline: later.unix(),
            ..muted
        }]
    );
    s.send_message(later, &bob, "free", &mut p).unwrap();
    assert_eq!(s.get_mute(later, &bob), GetMuteResponse::default());
    assert_eq!(s.metrics().unmutes, 1);
}

#[test]
fn an_unmute_without_a_profile_saves_nothing() {
    let mut s = server();
    s.unmute(T0, &caller(OWNER, "owner"), &unmute_req(b"bob"), None)
        .unwrap();
    assert_eq!(s.poll_action(), None);
}

#[test]
fn a_mute_updates_an_existing_profile_keeping_its_latest_mutes() {
    let mut s = server();
    let existing = Profile {
        id: 3,
        server_id: 77,
        peer_key: b"bob".to_vec(),
        alias: "bob".into(),
        mutes: (0..MAX_PROFILE_MUTES as i64)
            .map(|i| profile::Mute {
                created_at: i,
                message: format!("m{i}"),
                ..profile::Mute::default()
            })
            .collect(),
        mute_deadline: 0,
    };
    s.mute(
        T0,
        &caller(OWNER, "owner"),
        &mute_req(b"bob", 1, "again"),
        Some(existing),
        99,
    )
    .unwrap();
    let p = profiles(&mut s).remove(0);
    assert_eq!(p.id, 3);
    assert_eq!(p.mutes.len(), MAX_PROFILE_MUTES);
    assert_eq!(p.mutes[0].message, "m1", "the oldest is dropped");
    assert_eq!(p.mutes.last().unwrap().message, "again");
}

#[test]
fn mute_requests_are_bounded() {
    let mut s = server();
    let owner = caller(OWNER, "owner");
    let long = "x".repeat(MAX_MUTE_MESSAGE_BYTES + 1);
    assert_eq!(
        s.mute(T0, &owner, &mute_req(b"bob", 1, &long), None, 1),
        Err(ChatError::TooLarge)
    );
    let key = vec![1; MAX_PEER_KEY_BYTES + 1];
    assert_eq!(
        s.mute(T0, &owner, &mute_req(&key, 1, ""), None, 1),
        Err(ChatError::TooLarge)
    );
    assert_eq!(
        s.unmute(T0, &owner, &unmute_req(&key), None),
        Err(ChatError::TooLarge)
    );
    let fits = "x".repeat(MAX_MUTE_MESSAGE_BYTES);
    s.mute(T0, &owner, &mute_req(b"bob", 1, &fits), None, 1)
        .unwrap();
}

#[test]
fn a_zero_length_mute_leaves_the_user_unmuted() {
    let mut p = Patterns::new();
    let mut s = server();
    let owner = caller(OWNER, "owner");
    s.mute(T0, &owner, &mute_req(b"bob", 60, ""), None, 1)
        .unwrap();
    s.mute(T0, &owner, &mute_req(b"bob", 0, "warning"), None, 1)
        .unwrap();
    assert_eq!(profiles(&mut s).len(), 2);
    assert_eq!(s.mute_count(), 0);
    s.send_message(T0, &caller(b"bob", "bob"), "ok", &mut p)
        .unwrap();
}

#[test]
fn set_mutes_loads_the_active_mutes() {
    let mut s = server();
    let now = T0;
    let p = |key: &[u8], deadline: i64, mutes: Vec<profile::Mute>| Profile {
        peer_key: key.to_vec(),
        mute_deadline: deadline,
        mutes,
        ..Profile::default()
    };
    let m = |created_at: i64, duration_secs: u32, message: &str| profile::Mute {
        created_at,
        duration_secs,
        message: message.into(),
        ..profile::Mute::default()
    };
    let t = now.unix();
    s.set_mutes(
        now,
        &[
            p(b"current", t + 50, vec![m(t - 10, 60, "this one")]),
            p(b"later", t + 100, vec![m(t - 1000, 60, "an old one")]),
            p(b"over", t - 1, vec![m(t - 61, 60, "ended")]),
            p(b"never", 0, vec![]),
        ],
    );
    assert_eq!(s.mute_count(), 2);
    let get = |s: &Server, key: &[u8]| s.get_mute(now, &caller(key, "x"));
    assert_eq!(
        get(&s, b"current"),
        GetMuteResponse {
            end_time: t + 50,
            message: "this one".into()
        }
    );
    assert_eq!(
        get(&s, b"later"),
        GetMuteResponse {
            end_time: t + 100,
            message: String::new()
        },
        "the latest mute didn't set the deadline"
    );
    assert_eq!(get(&s, b"over"), GetMuteResponse::default());

    s.set_mutes(now, &[]);
    assert_eq!(s.mute_count(), 0);
}

#[test]
fn set_mutes_keeps_the_mutes_that_end_last() {
    let mut s = Server::new(
        &record(),
        OWNER,
        ServerLimits {
            max_mutes: 2,
            ..ServerLimits::default()
        },
        det_rng(1),
    );
    let t = T0.unix();
    let profiles: Vec<Profile> = [(b"a", 30), (b"b", 10), (b"c", 20), (b"d", 5)]
        .into_iter()
        .map(|(k, d)| Profile {
            peer_key: k.to_vec(),
            mute_deadline: t + d,
            ..Profile::default()
        })
        .collect();
    s.set_mutes(T0, &profiles);
    assert_eq!(s.mute_count(), 2);
    let end = |k: &[u8]| s.get_mute(T0, &caller(k, "x")).end_time;
    assert_eq!(end(b"a"), t + 30);
    assert_eq!(end(b"c"), t + 20);
    assert_eq!(end(b"b"), 0);
    assert_eq!(end(b"d"), 0);
    assert_eq!(s.metrics().mutes_dropped, 2);
}

#[test]
fn a_mute_past_the_bound_evicts_the_one_that_ends_first() {
    let mut s = Server::new(
        &record(),
        OWNER,
        ServerLimits {
            max_mutes: 2,
            ..ServerLimits::default()
        },
        det_rng(1),
    );
    let owner = caller(OWNER, "owner");
    s.mute(T0, &owner, &mute_req(b"a", 30, ""), None, 1)
        .unwrap();
    s.mute(T0, &owner, &mute_req(b"b", 10, ""), None, 2)
        .unwrap();
    s.mute(T0, &owner, &mute_req(b"c", 5, ""), None, 3).unwrap();
    assert_eq!(s.mute_count(), 2);
    let end = |s: &Server, k: &[u8]| s.get_mute(T0, &caller(k, "x")).end_time;
    assert_eq!(end(&s, b"b"), 0);
    assert_eq!(end(&s, b"c"), T0.unix() + 5);

    // Ended mutes go first.
    let later = T0 + Duration::SECOND * 6;
    s.mute(later, &owner, &mute_req(b"d", 5, ""), None, 4)
        .unwrap();
    assert_eq!(end(&s, b"a"), T0.unix() + 30);
    assert_eq!(s.metrics().mutes_dropped, 1);
}

#[test]
fn sends_are_rate_limited_per_peer() {
    let mut p = Patterns::new();
    let mut s = server();
    let alice = caller(b"alice", "alice");
    for i in 0..5 {
        s.send_message(T0, &alice, &format!("m{i}"), &mut p)
            .unwrap();
    }
    assert_eq!(
        s.send_message(T0, &alice, "m5", &mut p),
        Err(ChatError::TooFast)
    );
    s.send_message(T0, &caller(b"bob", "bob"), "hi", &mut p)
        .unwrap();
    s.send_message(T0 + Duration::SECOND, &alice, "m6", &mut p)
        .unwrap();
    assert_eq!(messages(&mut s).len(), 7);
    assert_eq!(s.metrics().refused_too_fast, 1);
}

#[test]
fn bodies_are_bounded() {
    let mut p = Patterns::new();
    let mut s = server();
    let alice = caller(b"alice", "alice");
    let long = "x".repeat(MAX_BODY_BYTES + 1);
    for _ in 0..10 {
        assert_eq!(
            s.send_message(T0, &alice, &long, &mut p),
            Err(ChatError::MessageTooLong)
        );
    }
    let fits = "é".repeat(MAX_BODY_BYTES / 2);
    s.send_message(T0, &alice, &fits, &mut p).unwrap();
    assert_eq!(
        message(&mut s).body,
        fits,
        "an oversized send spends no rate"
    );
    assert_eq!(s.metrics().refused_too_long, 10);
}

#[test]
fn callers_are_bounded() {
    let mut p = Patterns::new();
    let mut s = server();
    let long = caller(b"alice", &"a".repeat(MAX_ALIAS_BYTES + 1));
    assert_eq!(
        s.send_message(T0, &long, "hi", &mut p),
        Err(ChatError::TooLarge)
    );
    assert_eq!(s.metrics().refused_too_large, 1);
}

#[test]
fn combos_run_across_users_and_refuse_repeats() {
    let mut p = Patterns::new();
    let mut s = server();
    let alice = caller(b"alice", "alice");
    let bob = caller(b"bob", "bob");
    s.send_message(T0, &alice, "PEPE", &mut p).unwrap();
    s.send_message(T0, &bob, "PEPE:wide", &mut p).unwrap();
    assert_eq!(
        s.send_message(T0, &alice, "PEPE", &mut p),
        Err(ChatError::ComboDuplicate)
    );
    let m = messages(&mut s);
    let first = &entities(&m[0]).emotes[0];
    assert!(first.can_combo);
    assert_eq!(first.combo, 0);
    let second = &entities(&m[1]).emotes[0];
    assert_eq!(second.combo, 2);
    assert!(
        second.modifiers.is_empty(),
        "the tie goes to the first variant"
    );
    assert_eq!(s.metrics().combo_messages, 1);
    assert_eq!(s.metrics().refused_combo, 1);

    s.send_message(T0, &alice, "PEPE and text", &mut p).unwrap();
    s.send_message(T0, &alice, "PEPE", &mut p).unwrap();
    let m = messages(&mut s);
    assert_eq!(entities(&m[1]).emotes[0].combo, 0, "text ended the combo");
}

fn nick(name: &str, peer_key: &[u8], start: u32, end: u32) -> Nick {
    Nick {
        bounds: Some(Bounds { start, end }),
        nick: name.into(),
        peer_key: peer_key.to_vec(),
        viewed_listing: None,
    }
}

fn nicks_of(s: &mut Server, p: &mut Patterns, body: &str) -> Vec<Nick> {
    // Each check sends as its own peer to stay within the send rate.
    let n = s.metrics().messages;
    let key = n.to_be_bytes();
    s.send_message(T0, &caller(&key, "checker"), body, p)
        .unwrap();
    entities(&message(s)).nicks.clone()
}

#[test]
fn nicks_follow_the_listing_users() {
    let mut p = Patterns::new();
    let mut s = server();
    s.reset_nicks(&[user(b"alice", "Alice"), user(b"bob", "bob")]);
    assert_eq!(
        nicks_of(&mut s, &mut p, "hi alice and BOB"),
        [nick("Alice", b"alice", 3, 8), nick("bob", b"bob", 13, 16)]
    );

    s.nick_changed(UserEventKind::Joined, &user(b"carol", "carol"));
    assert_eq!(
        nicks_of(&mut s, &mut p, "carol"),
        [nick("carol", b"carol", 0, 5)]
    );

    s.nick_changed(UserEventKind::Renamed, &user(b"alice", "alicia"));
    assert_eq!(nicks_of(&mut s, &mut p, "alice"), []);
    assert_eq!(
        nicks_of(&mut s, &mut p, "alicia"),
        [nick("alicia", b"alice", 0, 6)]
    );

    s.nick_changed(UserEventKind::Parted, &user(b"bob", "bob"));
    assert_eq!(nicks_of(&mut s, &mut p, "bob"), []);

    s.reset_nicks(&[user(b"dave", "dave")]);
    assert_eq!(nicks_of(&mut s, &mut p, "alicia carol"), []);
    assert_eq!(
        nicks_of(&mut s, &mut p, "dave"),
        [nick("dave", b"dave", 0, 4)]
    );
    assert_eq!(s.parser_context().nicks().len(), 1);
}

#[test]
fn a_part_keeps_a_nick_another_user_took() {
    let mut p = Patterns::new();
    let mut s = server();
    s.reset_nicks(&[user(b"a", "same")]);
    s.nick_changed(UserEventKind::Joined, &user(b"b", "SAME"));
    s.nick_changed(UserEventKind::Parted, &user(b"a", "same"));
    assert_eq!(nicks_of(&mut s, &mut p, "same"), [nick("SAME", b"b", 0, 4)]);
}

#[test]
fn a_mute_created_profile_takes_the_listing_alias() {
    let mut s = server();
    s.reset_nicks(&[user(b"bob", "Bob")]);
    let owner = caller(OWNER, "owner");
    s.mute(T0, &owner, &mute_req(b"bob", 1, ""), None, 1)
        .unwrap();
    s.mute(T0, &owner, &mute_req(b"zed", 1, ""), None, 2)
        .unwrap();
    let saved = profiles(&mut s);
    assert_eq!(saved[0].alias, "Bob");
    assert_eq!(saved[1].alias, "", "a user outside the listing");
}

/// The messages two servers with `seed` send for the same calls.
fn internal_run(seed: u64) -> Vec<Message> {
    let mut p = Patterns::new();
    let mut s = Server::new(&record(), OWNER, ServerLimits::default(), det_rng(seed));
    s.sync_assets(
        &[emote("PEPE", true), emote("Kappa", true)],
        &[
            modifier("wide", false, 0.0),
            modifier("rare", true, 0.5),
            modifier("always", true, 1.0),
        ],
        &[],
    );
    for i in 0..20u8 {
        let body = if i % 2 == 0 {
            "PEPE Kappa"
        } else {
            "Kappa PEPE PEPE"
        };
        s.send_message(T0, &caller(&[i], "u"), body, &mut p)
            .unwrap();
    }
    messages(&mut s)
}

#[test]
fn internal_modifiers_are_deterministic_for_a_seed() {
    let a = internal_run(42);
    assert_eq!(a, internal_run(42));
    let modifiers: Vec<&str> = a
        .iter()
        .flat_map(|m| &entities(m).emotes)
        .flat_map(|e| e.modifiers.iter().map(String::as_str))
        .collect();
    assert_eq!(modifiers.iter().filter(|m| **m == "always").count(), 20);
    let rare = modifiers.iter().filter(|m| **m == "rare").count();
    assert!(rare > 0 && rare < 20, "rare procs at random: {rare}");
    assert!(
        !modifiers.contains(&"wide"),
        "visible modifiers aren't added"
    );
}

#[test]
fn internal_modifiers_are_not_parsed() {
    let mut p = Patterns::new();
    let mut s = server();
    s.sync_assets(&[emote("PEPE", true)], &[modifier("rare", true, 0.0)], &[]);
    s.send_message(T0, &caller(b"x", "x"), "PEPE:rare", &mut p)
        .unwrap();
    let m = message(&mut s);
    let e = &entities(&m).emotes;
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].bounds, Some(Bounds { start: 0, end: 4 }));
    assert!(e[0].modifiers.is_empty());
}

#[test]
fn truncation_keeps_char_boundaries() {
    assert_eq!(truncate("héllo", 2), "h");
    assert_eq!(truncate("héllo", 3), "hé");
    assert_eq!(truncate("hi", 10), "hi");
}
