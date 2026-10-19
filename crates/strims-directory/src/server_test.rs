// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;

use prost::Message;
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::{
    DEFAULT_CERT_TTL, Domain, Error as CryptoError, SignableMessage, Signer, certificate_body,
    generate_key, host_certificate, key_from_seed, new_certificate_request,
    sign_certificate_request_with_network, verify_certificate,
};
use strims_kademlia::ID;
use strims_ppspp::{SwarmId, Uri, UriOptions};
use strims_proto::network::v1::directory::event::Body;
use strims_proto::network::v1::directory::listing::{Chat, Content, Embed, Media};
use strims_proto::network::v1::directory::listing_query::Query;
use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;
use strims_proto::network::v1::directory::server_config::Integrations;
use strims_proto::network::v1::directory::server_config::integrations::{
    AngelThump, Swarm, Twitch, YouTube,
};
use strims_proto::network::v1::directory::{
    Event, EventBroadcast, Listing, ListingModeration, ListingQuery, ListingSnippet,
    ListingSnippetDelta, ListingSnippetImage, ServerConfig, UserModeration,
};
use strims_proto::types::{Image, KeyUsage};

use crate::embed::EmbedService;
use crate::server::{
    LoadId, MAX_BROADCAST_RECORD, MAX_DELTAS_PER_MINUTE, MAX_UNVERIFIED, MIN_EMBED_LOAD_SPACING,
    MIN_SNIPPET_BACKOFF, REFRESH_BYTES_PER_TICK,
};
use crate::snippet::{MAX_TEXT_BYTES, diff, split};
use crate::{Caller, DirectoryError, Limits, Server, ServerAction, SubId, Tunables};

const T0: Time = Time::from_unix_millis(1_700_000_000_000);

fn at(secs: i64) -> Time {
    T0 + Duration::from_secs(secs)
}

fn caller(user: u8, host: u8) -> Caller {
    Caller {
        host_key: vec![host; 32],
        host: ID::from_bytes(&[host; 32]),
        peer_key: vec![user; 32],
        alias: format!("user{user}"),
    }
}

fn chat(n: u8) -> Listing {
    Listing {
        content: Some(Content::Chat(Chat {
            key: vec![n; 32],
            name: format!("chat {n}"),
        })),
    }
}

fn swarm(n: u8) -> SwarmId {
    SwarmId::new(&[n; 32])
}

fn media(n: u8) -> Listing {
    Listing {
        content: Some(Content::Media(Media {
            mime_type: "video/mp4".into(),
            swarm_uri: Uri::new(swarm(n), UriOptions::default()).to_string(),
        })),
    }
}

fn by_id(id: u64) -> ListingQuery {
    ListingQuery {
        query: Some(Query::Id(id)),
    }
}

fn by_listing(l: Listing) -> ListingQuery {
    ListingQuery {
        query: Some(Query::Listing(l)),
    }
}

fn config(f: impl FnOnce(&mut ServerConfig)) -> ServerConfig {
    let mut c = ServerConfig::default();
    f(&mut c);
    c
}

/// A server and the events of its broadcasts.
struct Fixture {
    s: Server,
    now: Time,
    actions: Vec<ServerAction>,
}

impl Fixture {
    fn new() -> Fixture {
        Fixture::with(Limits::default(), None)
    }

    fn with(limits: Limits, config: Option<&ServerConfig>) -> Fixture {
        Fixture {
            s: Server::new(limits, config, det_rng(1), T0),
            now: T0,
            actions: Vec::new(),
        }
    }

    fn check(&mut self) {
        self.s.check().unwrap();
        self.actions
            .extend(std::iter::from_fn(|| self.s.poll_action()));
    }

    fn publish(&mut self, c: &Caller, l: Listing) -> Result<u64, DirectoryError> {
        let r = self.s.publish(self.now, c, l, None, None);
        self.check();
        r
    }

    fn join(&mut self, c: &Caller, q: ListingQuery) -> Result<u64, DirectoryError> {
        let r = self.s.join(self.now, c, &q, None);
        self.check();
        r
    }

    fn part(&mut self, c: &Caller, id: u64) -> Result<(), DirectoryError> {
        let r = self.s.part(self.now, c, id);
        self.check();
        r
    }

    fn unpublish(&mut self, c: &Caller, id: u64) -> Result<(), DirectoryError> {
        let r = self.s.unpublish(self.now, c, id);
        self.check();
        r
    }

    fn ping(&mut self, c: &Caller) -> Result<(), DirectoryError> {
        let r = self.s.ping(self.now, c);
        self.check();
        r
    }

    /// Runs the server's timers up to `to`, returning the broadcast
    /// records.
    fn run_until(&mut self, to: Time) -> Vec<EventBroadcast> {
        let mut records = Vec::new();
        while let Some(t) = self.s.poll_timeout() {
            if t > to {
                break;
            }
            self.now = t;
            self.s.handle_timeout(t);
            self.s.check().unwrap();
            for a in std::iter::from_fn(|| self.s.poll_action()) {
                match a {
                    ServerAction::Broadcast(r) => records.extend(r),
                    a => self.actions.push(a),
                }
            }
        }
        self.now = to;
        records
    }

    /// The events of the next broadcast.
    fn tick(&mut self) -> Vec<String> {
        let next = self.s.poll_timeout().unwrap().max(self.now);
        let records = self.run_until(next);
        describe(&records)
    }

    /// The events of the broadcasts up to `secs` from now.
    fn advance(&mut self, secs: i64) -> Vec<String> {
        let records = self.run_until(self.now + Duration::from_secs(secs));
        describe(&records)
    }

    fn take_actions(&mut self) -> Vec<ServerAction> {
        std::mem::take(&mut self.actions)
    }
}

fn describe(records: &[EventBroadcast]) -> Vec<String> {
    records
        .iter()
        .flat_map(|r| &r.events)
        .map(describe_event)
        .collect()
}

fn describe_event(e: &Event) -> String {
    match &e.body {
        Some(Body::ListingChange(c)) => format!("change {}", c.id),
        Some(Body::Unpublish(u)) => format!("unpublish {}", u.id),
        Some(Body::UserCountChange(c)) => {
            format!("count {} {}/{}", c.id, c.user_count, c.recent_user_count)
        }
        Some(Body::UserPresenceChange(p)) if p.online => {
            format!("online {} {} {:?}", p.id, p.alias, p.listing_ids)
        }
        Some(Body::UserPresenceChange(p)) => format!("offline {} {}", p.id, p.alias),
        Some(Body::Ping(_)) | None => "other".into(),
    }
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

// --- calls ------------------------------------------------------------------

#[test]
fn a_published_listing_and_its_publisher_are_broadcast() {
    let mut f = Fixture::new();
    assert_eq!(f.publish(&caller(1, 1), chat(1)), Ok(1));
    assert_eq!(
        f.tick(),
        s(&["change 1", "count 1 1/1", "online 1 user1 [1]"])
    );
    assert_eq!(f.tick(), Vec::<String>::new());
}

#[test]
fn listings_with_one_key_are_one_listing() {
    let mut f = Fixture::new();
    assert_eq!(f.publish(&caller(1, 1), chat(1)), Ok(1));
    assert_eq!(f.publish(&caller(2, 2), chat(1)), Ok(1));
    assert_eq!(f.publish(&caller(2, 2), chat(2)), Ok(2));
    assert_eq!(f.join(&caller(3, 3), by_listing(chat(2))), Ok(2));
    assert_eq!(
        f.tick(),
        s(&[
            "change 2",
            "count 2 2/2",
            "change 1",
            "count 1 2/2",
            "online 3 user3 [2]",
            "online 2 user2 [1, 2]",
            "online 1 user1 [1]",
        ])
    );
}

#[test]
fn a_repeated_publish_or_join_does_not_count_against_the_quota() {
    let c = config(|c| {
        c.publish_quota = 1;
        c.join_quota = 1;
    });
    let mut f = Fixture::with(Limits::default(), Some(&c));
    let (a, b) = (caller(1, 1), caller(2, 2));
    assert_eq!(f.publish(&a, chat(1)), Ok(1));
    assert_eq!(f.publish(&a, chat(1)), Ok(1));
    assert_eq!(f.publish(&a, chat(2)), Err(DirectoryError::PublishQuota));
    assert_eq!(f.publish(&b, chat(2)), Ok(2));
    assert_eq!(f.join(&b, by_id(1)), Ok(1));
    assert_eq!(f.join(&b, by_id(1)), Ok(1));
    // joining a listing the session publishes holds already
    assert_eq!(f.join(&b, by_id(2)), Ok(2));
    assert_eq!(
        f.join(&caller(2, 3), by_id(1)),
        Err(DirectoryError::ViewQuota)
    );
    assert_eq!(f.join(&a, by_id(2)), Ok(2));
}

#[test]
fn quotas_count_listings_across_a_users_sessions() {
    let c = config(|c| c.publish_quota = 2);
    let mut f = Fixture::with(Limits::default(), Some(&c));
    f.publish(&caller(1, 1), chat(1)).unwrap();
    f.publish(&caller(1, 2), chat(1)).unwrap();
    assert_eq!(
        f.publish(&caller(1, 1), chat(2)),
        Err(DirectoryError::PublishQuota)
    );
    f.unpublish(&caller(1, 2), 1).unwrap();
    assert_eq!(f.publish(&caller(1, 1), chat(2)), Ok(2));
}

#[test]
fn a_failed_join_creates_no_session_or_user() {
    let mut f = Fixture::new();
    let c = caller(1, 1);
    assert_eq!(f.join(&c, by_id(7)), Err(DirectoryError::ListingNotFound));
    assert_eq!(
        f.join(&c, by_listing(chat(1))),
        Err(DirectoryError::ListingNotFound)
    );
    assert_eq!(
        f.join(&c, ListingQuery::default()),
        Err(DirectoryError::ListingNotFound)
    );
    f.publish(&caller(2, 2), chat(1)).unwrap();
    let moderation = UserModeration {
        disable_join: Some(true),
        ..Default::default()
    };
    assert_eq!(
        f.s.join(f.now, &c, &by_id(1), Some(&moderation)),
        Err(DirectoryError::JoinDisabled)
    );
    assert_eq!(f.ping(&c), Err(DirectoryError::SessionNotFound));
    assert_eq!(f.s.metrics().users_created, 1);
    assert_eq!(f.s.metrics().sessions_created, 1);
    assert_eq!(f.s.metrics().refused, 5);
}

#[test]
fn a_failed_publish_creates_no_session_or_user() {
    let mut f = Fixture::new();
    let c = caller(1, 1);
    let embed = Listing {
        content: Some(Content::Embed(Embed {
            service: 2,
            id: "x".into(),
            ..Default::default()
        })),
    };
    let bad_media = Listing {
        content: Some(Content::Media(Media {
            mime_type: String::new(),
            swarm_uri: "magnet:?xt=urn:btih:x".into(),
        })),
    };
    let no_publish = UserModeration {
        disable_publish: Some(true),
        ..Default::default()
    };
    let banned = ListingModeration {
        is_banned: Some(true),
        ..Default::default()
    };
    let now = f.now;
    assert_eq!(
        f.s.publish(now, &c, embed.clone(), Some(&no_publish), Some(&banned)),
        Err(DirectoryError::PublishDisabled)
    );
    assert_eq!(
        f.s.publish(now, &c, embed, None, Some(&banned)),
        Err(DirectoryError::UnsupportedEmbed)
    );
    assert_eq!(
        f.s.publish(now, &c, bad_media, None, None),
        Err(DirectoryError::InvalidSwarmUri)
    );
    assert_eq!(
        f.s.publish(now, &c, Listing::default(), None, None),
        Err(DirectoryError::UnsupportedContent)
    );
    assert_eq!(
        f.s.publish(now, &c, chat(1), None, Some(&banned)),
        Err(DirectoryError::ListingBanned)
    );
    f.check();
    assert_eq!(f.ping(&c), Err(DirectoryError::SessionNotFound));
    assert_eq!(f.s.metrics().users_created, 0);
    assert_eq!(f.tick(), Vec::<String>::new());
}

#[test]
fn joining_a_listing_the_session_publishes_adds_nothing() {
    let mut f = Fixture::new();
    let c = caller(1, 1);
    f.publish(&c, chat(1)).unwrap();
    assert_eq!(f.join(&c, by_id(1)), Ok(1));
    assert_eq!(f.part(&c, 1), Err(DirectoryError::NotViewing));
    assert_eq!(
        f.tick(),
        s(&["change 1", "count 1 1/1", "online 1 user1 [1]"])
    );
}

#[test]
fn publishing_a_viewed_listing_ends_the_view() {
    let mut f = Fixture::new();
    let (a, b) = (caller(1, 1), caller(2, 2));
    f.publish(&a, chat(1)).unwrap();
    f.join(&b, by_id(1)).unwrap();
    f.publish(&b, chat(1)).unwrap();
    assert_eq!(f.part(&b, 1), Err(DirectoryError::NotViewing));
    f.unpublish(&a, 1).unwrap();
    f.tick();
    // b still publishes it
    assert_eq!(f.s.listing_ids(), vec![1]);
    assert_eq!(
        f.s.user_sessions(&b.peer_key)[&b.host_key],
        (vec![1], vec![])
    );
}

#[test]
fn leaving_needs_the_listing_a_session_and_the_role() {
    let mut f = Fixture::new();
    let (a, b) = (caller(1, 1), caller(2, 2));
    assert_eq!(f.unpublish(&a, 1), Err(DirectoryError::ListingNotFound));
    f.publish(&a, chat(1)).unwrap();
    assert_eq!(f.unpublish(&b, 1), Err(DirectoryError::SessionNotFound));
    assert_eq!(f.part(&b, 1), Err(DirectoryError::SessionNotFound));
    f.publish(&b, chat(2)).unwrap();
    assert_eq!(f.unpublish(&b, 1), Err(DirectoryError::NotPublishing));
    assert_eq!(f.part(&b, 1), Err(DirectoryError::NotViewing));
    f.join(&b, by_id(1)).unwrap();
    assert_eq!(f.part(&b, 1), Ok(()));
    assert_eq!(f.part(&b, 1), Err(DirectoryError::NotViewing));
}

#[test]
fn the_last_publisher_leaving_unpublishes_at_the_next_tick() {
    let mut f = Fixture::new();
    let (a, b) = (caller(1, 1), caller(2, 2));
    f.publish(&a, chat(1)).unwrap();
    f.join(&b, by_id(1)).unwrap();
    f.tick();
    f.unpublish(&a, 1).unwrap();
    assert_eq!(
        f.tick(),
        s(&["unpublish 1", "online 2 user2 []", "online 1 user1 []"])
    );
    assert_eq!(f.join(&b, by_id(1)), Err(DirectoryError::ListingNotFound));
    // a new publish is a new listing
    assert_eq!(f.publish(&a, chat(1)), Ok(2));
}

#[test]
fn a_ban_unpublishes_at_the_next_tick() {
    let mut f = Fixture::new();
    let (a, b) = (caller(1, 1), caller(2, 2));
    f.publish(&a, chat(1)).unwrap();
    f.join(&b, by_id(1)).unwrap();
    f.tick();
    let ban = ListingModeration {
        is_banned: Some(true),
        ..Default::default()
    };
    f.s.listing_moderated(f.now, 1, Some(ban.clone()));
    f.check();
    assert_eq!(
        f.tick(),
        s(&["unpublish 1", "online 1 user1 []", "online 2 user2 []"])
    );
    let now = f.now;
    assert_eq!(
        f.s.publish(now, &a, chat(1), None, Some(&ban)),
        Err(DirectoryError::ListingBanned)
    );
}

#[test]
fn a_moderation_change_is_broadcast_as_a_listing_change() {
    let mut f = Fixture::new();
    f.publish(&caller(1, 1), chat(1)).unwrap();
    f.tick();
    let m = ListingModeration {
        category: Some("games".into()),
        is_mature: Some(true),
        ..Default::default()
    };
    f.s.listing_moderated(f.now, 1, Some(m.clone()));
    f.s.listing_moderated(f.now, 9, Some(m.clone()));
    let records = f.run_until(f.now + Duration::SECOND);
    assert_eq!(describe(&records), s(&["change 1", "count 1 1/1"]));
    let Some(Body::ListingChange(c)) = &records[0].events[0].body else {
        panic!()
    };
    assert_eq!(c.moderation, Some(m));
    assert_eq!(c.snippet, Some(ListingSnippet::default()));
}

#[test]
fn an_oversized_category_is_not_kept() {
    let mut f = Fixture::new();
    let long = ListingModeration {
        category: Some("c".repeat(crate::listing::MAX_CATEGORY_BYTES + 1)),
        is_mature: Some(true),
        ..Default::default()
    };
    let now = f.now;
    f.s.publish(now, &caller(1, 1), chat(1), None, Some(&long))
        .unwrap();
    let kept = ListingModeration {
        is_mature: Some(true),
        ..Default::default()
    };
    assert_eq!(f.s.projection().listings[&1].moderation, Some(kept.clone()));
    f.s.listing_moderated(now, 1, Some(long));
    assert_eq!(f.s.projection().listings[&1].moderation, Some(kept));
}

#[test]
fn a_new_listing_takes_its_record_moderation() {
    let mut f = Fixture::new();
    let m = ListingModeration {
        is_mature: Some(true),
        ..Default::default()
    };
    let now = f.now;
    f.s.publish(now, &caller(1, 1), chat(1), None, Some(&m))
        .unwrap();
    assert_eq!(f.s.projection().listings[&1].moderation, Some(m));
}

#[test]
fn disabling_join_or_publish_ends_a_users_memberships() {
    let mut f = Fixture::new();
    let (a, b) = (caller(1, 1), caller(2, 2));
    f.publish(&a, chat(1)).unwrap();
    f.publish(&b, chat(2)).unwrap();
    f.join(&b, by_id(1)).unwrap();
    f.join(&caller(2, 3), by_id(1)).unwrap();
    f.tick();
    f.s.user_moderated(
        f.now,
        &b.peer_key,
        &UserModeration {
            disable_join: Some(true),
            ..Default::default()
        },
    );
    f.check();
    assert_eq!(f.tick(), s(&["count 1 1/2", "online 2 user2 [2]"]));
    f.s.user_moderated(
        f.now,
        &b.peer_key,
        &UserModeration {
            disable_publish: Some(true),
            ..Default::default()
        },
    );
    f.check();
    assert_eq!(f.tick(), s(&["unpublish 2", "online 2 user2 []"]));
    // unknown users and moderations that end nothing are no-ops
    f.s.user_moderated(f.now, &[9; 32], &UserModeration::default());
    f.s.user_moderated(f.now, &b.peer_key, &UserModeration::default());
    f.check();
    assert_eq!(f.tick(), Vec::<String>::new());
}

#[test]
fn sessions_expire_without_pings_and_stay_with_them() {
    let c = config(|c| c.session_timeout = 120);
    let mut f = Fixture::with(Limits::default(), Some(&c));
    let (a, b) = (caller(1, 1), caller(2, 2));
    f.publish(&a, chat(1)).unwrap();
    f.join(&b, by_id(1)).unwrap();
    f.tick();
    let mut events = Vec::new();
    for _ in 0..4 {
        events.extend(f.advance(60));
        f.ping(&a).unwrap();
    }
    assert!(events.contains(&"offline 2 user2".to_owned()), "{events:?}");
    assert!(events.contains(&"count 1 1/2".to_owned()), "{events:?}");
    assert_eq!(f.ping(&b), Err(DirectoryError::SessionNotFound));
    assert_eq!(f.part(&b, 1), Err(DirectoryError::SessionNotFound));
    let events = f.advance(125);
    assert!(events.contains(&"offline 1 user1".to_owned()), "{events:?}");
    assert!(events.contains(&"unpublish 1".to_owned()), "{events:?}");
    assert_eq!(f.s.metrics().sessions_expired, 2);
    assert_eq!(f.s.metrics().users_removed, 2);
}

#[test]
fn an_expired_session_ends_in_one_tick_and_its_listings_go_in_the_next() {
    let c = config(|c| c.session_timeout = 120);
    let mut f = Fixture::with(Limits::default(), Some(&c));
    f.publish(&caller(1, 1), chat(1)).unwrap();
    f.tick();
    let mut ticks = Vec::new();
    for _ in 0..125 {
        let e = f.tick();
        if !e.is_empty() {
            ticks.push(e);
        }
    }
    assert_eq!(ticks, vec![s(&["offline 1 user1"]), s(&["unpublish 1"])]);
}

#[test]
fn sessions_per_user_evict_the_least_recent() {
    let limits = Limits {
        max_sessions_per_user: 2,
        ..Limits::default()
    };
    let mut f = Fixture::with(limits, None);
    f.publish(&caller(1, 1), chat(1)).unwrap();
    f.now = at(1);
    f.publish(&caller(1, 2), chat(2)).unwrap();
    f.now = at(2);
    f.ping(&caller(1, 1)).unwrap();
    f.now = at(3);
    f.publish(&caller(1, 3), chat(3)).unwrap();
    assert_eq!(f.s.metrics().sessions_evicted, 1);
    let sessions = f.s.user_sessions(&[1; 32]);
    assert_eq!(
        sessions.keys().cloned().collect::<Vec<_>>(),
        vec![vec![1; 32], vec![3; 32]]
    );
    assert_eq!(f.ping(&caller(1, 2)), Err(DirectoryError::SessionNotFound));
    let events = f.tick();
    assert!(events.contains(&"unpublish 2".to_owned()), "{events:?}");
    assert!(
        events.contains(&"online 1 user1 [1, 3]".to_owned()),
        "{events:?}"
    );
}

#[test]
fn a_host_that_calls_as_another_user_leaves_its_old_session() {
    let mut f = Fixture::new();
    let old = caller(1, 1);
    let new = caller(2, 1);
    f.publish(&old, chat(1)).unwrap();
    f.tick();
    assert_eq!(f.ping(&new), Err(DirectoryError::SessionNotFound));
    assert_eq!(f.s.metrics().sessions_displaced, 1);
    assert_eq!(f.ping(&old), Err(DirectoryError::SessionNotFound));
    f.publish(&new, chat(2)).unwrap();
    let events = f.tick();
    assert_eq!(
        events,
        s(&[
            "change 2",
            "count 2 1/1",
            "unpublish 1",
            "online 2 user2 [2]",
            "offline 1 user1"
        ])
    );
}

#[test]
fn a_new_alias_reaches_the_users_presence() {
    let mut f = Fixture::new();
    f.publish(&caller(1, 1), chat(1)).unwrap();
    f.tick();
    let mut renamed = caller(1, 2);
    renamed.alias = "bob".into();
    f.join(&renamed, by_id(1)).unwrap();
    assert_eq!(f.tick(), s(&["count 1 1/1", "online 1 bob [1]"]));
}

#[test]
fn recent_users_count_distinct_users_and_expire_after_the_ttl() {
    let c = config(|c| {
        c.session_timeout = 60 * 60;
        c.refresh_interval = 60 * 60;
    });
    let mut f = Fixture::with(Limits::default(), Some(&c));
    let (a, b, c2) = (caller(1, 1), caller(2, 2), caller(3, 3));
    f.publish(&a, chat(1)).unwrap();
    f.join(&caller(1, 9), by_id(1)).unwrap();
    f.join(&b, by_id(1)).unwrap();
    f.join(&c2, by_id(1)).unwrap();
    assert!(f.tick().contains(&"count 1 3/3".to_owned()));
    f.part(&b, 1).unwrap();
    f.part(&c2, 1).unwrap();
    f.join(&c2, by_id(1)).unwrap();
    f.part(&c2, 1).unwrap();
    assert_eq!(f.tick()[0], "count 1 1/3");
    // a keeps pinging; b and c2 go quiet
    for _ in 0..31 {
        f.advance(60);
        f.ping(&a).unwrap();
        f.ping(&b).unwrap();
        f.ping(&c2).unwrap();
    }
    f.join(&caller(4, 4), by_id(1)).unwrap();
    assert_eq!(f.tick()[0], "count 1 2/2");
}

#[test]
fn a_refresh_resends_everything() {
    let c = config(|c| c.refresh_interval = 60);
    let mut f = Fixture::with(Limits::default(), Some(&c));
    f.publish(&caller(1, 1), chat(1)).unwrap();
    f.publish(&caller(2, 2), chat(2)).unwrap();
    f.join(&caller(3, 3), by_id(1)).unwrap();
    f.tick();
    let events = f.advance(60);
    // least recently changed first: the join changed listing 1 last
    assert_eq!(
        events,
        s(&[
            "change 2",
            "count 2 1/1",
            "change 1",
            "count 1 2/2",
            "online 1 user1 [1]",
            "online 2 user2 [2]",
            "online 3 user3 [1]",
        ])
    );
    assert_eq!(f.s.metrics().refreshes, 1);
}

#[test]
fn a_refresh_skips_what_the_same_broadcast_sends() {
    let c = config(|c| c.refresh_interval = 60);
    let mut f = Fixture::with(Limits::default(), Some(&c));
    f.publish(&caller(1, 1), chat(1)).unwrap();
    f.publish(&caller(2, 2), chat(2)).unwrap();
    f.tick();
    f.advance(58);
    f.publish(&caller(1, 1), chat(3)).unwrap();
    f.unpublish(&caller(2, 2), 2).unwrap();
    let events = f.tick();
    assert_eq!(
        events,
        s(&[
            "change 1",
            "count 1 1/1",
            "unpublish 2",
            "change 3",
            "count 3 1/1",
            "online 2 user2 []",
            "online 1 user1 [1, 3]",
        ])
    );
}

#[test]
fn a_refresh_is_paced_by_the_tick_budget() {
    let c = config(|c| c.refresh_interval = 60);
    let mut f = Fixture::with(Limits::default(), Some(&c));
    let big = |n: u8| Listing {
        content: Some(Content::Chat(Chat {
            key: vec![n; 32],
            name: "n".repeat(4000),
        })),
    };
    for n in 0..200 {
        f.publish(&caller(n, n), big(n)).unwrap();
    }
    f.tick();
    let mut records = Vec::new();
    while f.s.metrics().refreshes == 0 {
        records = f.run_until(f.now + Duration::SECOND);
    }
    let first: usize = records.iter().map(Message::encoded_len).sum();
    assert!(first <= REFRESH_BYTES_PER_TICK + 8 * 1024, "{first}");
    assert!(first >= REFRESH_BYTES_PER_TICK - 8 * 1024, "{first}");
    let mut events = describe(&records);
    let mut ticks = 1;
    while events.iter().filter(|e| e.starts_with("online")).count() < 200 {
        let records = f.run_until(f.now + Duration::SECOND);
        assert!(!records.is_empty());
        let bytes: usize = records.iter().map(Message::encoded_len).sum();
        assert!(bytes <= REFRESH_BYTES_PER_TICK + 8 * 1024, "{bytes}");
        events.extend(describe(&records));
        ticks += 1;
    }
    assert!((4..=5).contains(&ticks), "{ticks}");
    assert_eq!(
        events.iter().filter(|e| e.starts_with("change")).count(),
        200
    );
    // the users follow the listings
    let last_change = events.iter().rposition(|e| e.starts_with("change"));
    let first_online = events.iter().position(|e| e.starts_with("online"));
    assert!(last_change < first_online);
    assert_eq!(f.advance(10), Vec::<String>::new());
}

#[test]
fn broadcasts_order_listing_events_before_presence() {
    let mut f = Fixture::new();
    let (a, b) = (caller(1, 1), caller(2, 2));
    f.publish(&a, chat(1)).unwrap();
    f.publish(&b, chat(2)).unwrap();
    f.join(&a, by_id(2)).unwrap();
    f.tick();
    f.unpublish(&b, 2).unwrap();
    f.publish(&b, chat(3)).unwrap();
    f.join(&a, by_id(3)).unwrap();
    let events = f.tick();
    let last_listing = events
        .iter()
        .rposition(|e| !e.starts_with("online") && !e.starts_with("offline"))
        .unwrap();
    let first_presence = events.iter().position(|e| e.starts_with("online")).unwrap();
    assert!(last_listing < first_presence, "{events:?}");
    assert_eq!(
        events,
        s(&[
            "change 3",
            "count 3 2/2",
            "unpublish 2",
            "online 1 user1 [1, 3]",
            "online 2 user2 [3]",
        ])
    );
}

#[test]
fn records_pack_events_up_to_the_record_bound() {
    let mut f = Fixture::new();
    let big = |n: u8| Listing {
        content: Some(Content::Chat(Chat {
            key: vec![n; 32],
            name: "n".repeat(4000),
        })),
    };
    for n in 0..40 {
        f.publish(&caller(n, n), big(n)).unwrap();
    }
    let records = f.run_until(f.now + Duration::SECOND);
    assert!(records.len() >= 3, "{}", records.len());
    for r in &records {
        assert!(
            r.encoded_len() <= MAX_BROADCAST_RECORD,
            "{}",
            r.encoded_len()
        );
    }
    assert_eq!(describe(&records).len(), 120);

    // an event over the bound goes alone
    let mut f = Fixture::new();
    f.publish(&caller(1, 1), chat(1)).unwrap();
    f.publish(&caller(2, 2), media(2)).unwrap();
    let sub = subscribed(&mut f)[0].0;
    let key = key_from_seed(&[1; 32]);
    let snippet = signed(
        ListingSnippet {
            title: "t".repeat(4000),
            thumbnail: Some(image(30 * 1024)),
            channel_logo: Some(image(30 * 1024)),
            ..Default::default()
        },
        &key,
    );
    for d in split(diff(&ListingSnippet::default(), &snippet)) {
        f.s.snippet_delta(f.now, sub, &d);
    }
    let records = f.run_until(f.now + Duration::SECOND);
    assert!(
        records
            .iter()
            .any(|r| r.encoded_len() > MAX_BROADCAST_RECORD)
    );
    for r in records
        .iter()
        .filter(|r| r.encoded_len() > MAX_BROADCAST_RECORD)
    {
        assert_eq!(r.events.len(), 1);
    }
}

#[test]
fn the_directory_refuses_listings_users_and_sessions_beyond_its_limits() {
    let limits = Limits {
        max_listings: 2,
        max_users: 2,
        max_sessions: 3,
        max_sessions_per_user: 2,
        max_recent_users: 2,
    };
    let mut f = Fixture::with(limits, None);
    f.publish(&caller(1, 1), chat(1)).unwrap();
    f.publish(&caller(1, 1), chat(2)).unwrap();
    assert_eq!(f.publish(&caller(1, 1), chat(3)), Err(DirectoryError::Full));
    f.join(&caller(2, 2), by_id(1)).unwrap();
    assert_eq!(f.join(&caller(3, 3), by_id(1)), Err(DirectoryError::Full));
    f.join(&caller(2, 3), by_id(1)).unwrap();
    // sessions are full, but a user at its own limit replaces one
    assert_eq!(f.join(&caller(1, 4), by_id(1)), Err(DirectoryError::Full));
    f.join(&caller(2, 4), by_id(1)).unwrap();
    assert_eq!(f.s.metrics().sessions_evicted, 1);
}

#[test]
fn oversized_callers_are_refused() {
    let mut f = Fixture::new();
    let mut c = caller(1, 1);
    c.alias = "a".repeat(crate::MAX_ALIAS_BYTES + 1);
    assert_eq!(f.publish(&c, chat(1)), Err(DirectoryError::TooLarge));
    let mut c = caller(1, 1);
    c.peer_key = vec![1; crate::MAX_PEER_KEY_BYTES + 1];
    assert_eq!(f.join(&c, by_id(1)), Err(DirectoryError::TooLarge));
    let big = Listing {
        content: Some(Content::Chat(Chat {
            key: vec![1; 32],
            name: "n".repeat(5000),
        })),
    };
    assert_eq!(f.publish(&caller(1, 1), big), Err(DirectoryError::TooLarge));
}

#[test]
fn the_caller_is_the_host_certificate_and_its_membership() {
    let mut rng = det_rng(3);
    let network = generate_key(&mut rng);
    let profile = generate_key(&mut rng);
    let host = generate_key(&mut rng);
    let csr = new_certificate_request(
        &profile,
        KeyUsage::Peer as i32 | KeyUsage::Sign as i32,
        "alice",
    )
    .unwrap();
    let membership = sign_certificate_request_with_network(
        &csr,
        &network,
        "net",
        DEFAULT_CERT_TTL,
        T0,
        &mut rng,
    )
    .unwrap();
    let cert = host_certificate(&host.public, &profile, &membership).unwrap();
    let c = Caller::from_certificate(&cert).unwrap();
    assert_eq!(c.host_key, host.public);
    assert_eq!(c.host, ID::try_from(host.public.as_slice()).unwrap());
    assert_eq!(c.peer_key, profile.public);
    assert_eq!(c.alias, "alice");
    // a host certificate without the membership its body names
    let mut orphan = cert.clone();
    orphan.parent = None;
    let errs = verify_certificate(&orphan, T0).unwrap_err();
    assert!(errs.includes(CryptoError::MissingIssuer));
    assert_eq!(Caller::from_certificate(&orphan), None);
    // a validly signed host certificate whose key isn't a host id
    let mut body = certificate_body(&cert).unwrap();
    body.key.pop();
    let mut short = cert;
    short.body = body.encode_to_vec();
    short.signature = Signer::new(&profile)
        .unwrap()
        .sign(Domain::Certificate, &[&short.body])
        .to_vec();
    verify_certificate(&short, T0).unwrap();
    assert_eq!(Caller::from_certificate(&short), None);
}

#[test]
fn tunables_take_defaults_and_stay_in_range() {
    let d = Tunables::from_config(None);
    assert_eq!(d, Tunables::from_config(Some(&ServerConfig::default())));
    assert_eq!(d.broadcast_interval, Duration::SECOND);
    assert_eq!(d.refresh_interval, Duration::from_secs(300));
    assert_eq!(d.session_timeout, Duration::from_secs(900));
    assert_eq!(d.min_ping_interval, Duration::from_secs(600));
    assert_eq!(d.max_ping_interval, Duration::from_secs(840));
    assert_eq!(d.snippet_timeout, Duration::from_secs(30));
    assert_eq!((d.publish_quota, d.join_quota), (10, 10));
    assert!(!d.swarm);
    assert!(!d.embeds.any());
    assert_eq!(d.embed_load_interval, Duration::MINUTE);

    let huge = Tunables::from_config(Some(&config(|c| {
        c.broadcast_interval = u32::MAX;
        c.refresh_interval = u32::MAX;
        c.session_timeout = u32::MAX;
        c.min_ping_interval = u32::MAX;
        c.max_ping_interval = 1;
        c.load_media_embed_timeout = u32::MAX;
        c.embed_load_interval = u32::MAX;
        c.publish_quota = 1000;
        c.join_quota = 1;
    })));
    assert_eq!(huge.embed_load_interval, Duration::HOUR);
    assert_eq!(huge.broadcast_interval, Duration::MINUTE);
    assert_eq!(huge.refresh_interval, Duration::HOUR);
    assert_eq!(huge.session_timeout, Duration::HOUR * 24);
    assert_eq!(
        huge.min_ping_interval,
        Duration::HOUR * 24 - Duration::MINUTE
    );
    assert_eq!(huge.max_ping_interval, huge.min_ping_interval);
    assert_eq!(huge.snippet_timeout, Duration::MINUTE * 10);
    assert_eq!((huge.publish_quota, huge.join_quota), (64, 1));

    let small = Tunables::from_config(Some(&config(|c| {
        c.broadcast_interval = 30;
        c.refresh_interval = 1;
        c.session_timeout = 1;
        c.min_ping_interval = 1;
        c.max_ping_interval = 1;
        c.embed_load_interval = 1;
    })));
    assert_eq!(small.refresh_interval, Duration::from_secs(30));
    assert_eq!(small.embed_load_interval, Duration::from_secs(30));
    assert_eq!(small.session_timeout, Duration::MINUTE * 2);
    assert_eq!(small.min_ping_interval, Duration::MINUTE);
    assert_eq!(small.max_ping_interval, Duration::MINUTE);

    let short_session = Tunables::from_config(Some(&config(|c| c.session_timeout = 300)));
    assert_eq!(short_session.min_ping_interval, Duration::from_secs(240));
    assert_eq!(short_session.max_ping_interval, Duration::from_secs(240));
}

#[test]
fn the_client_config_advertises_the_swarm_integration_and_quotas() {
    let c = config(|c| {
        c.integrations = Some(Integrations {
            swarm: Some(Swarm { enable: true }),
            ..Default::default()
        });
        c.publish_quota = 3;
        c.min_ping_interval = 100;
        c.max_ping_interval = 200;
    });
    let mut f = Fixture::with(Limits::default(), Some(&c));
    let cc = f.s.client_config();
    let i = cc.integrations.unwrap();
    assert!(i.swarm && !i.angelthump && !i.twitch && !i.youtube);
    assert_eq!(
        (
            cc.publish_quota,
            cc.join_quota,
            cc.min_ping_interval,
            cc.max_ping_interval
        ),
        (3, 10, 100, 200)
    );
    f.s.set_config(None);
    assert!(!f.s.client_config().integrations.unwrap().swarm);
    assert_eq!(f.s.client_config().max_ping_interval, 840);
}

#[test]
fn the_client_config_advertises_the_embed_services_with_credentials() {
    let c = embeds_config(|i| {
        i.angelthump = Some(AngelThump { enable: true });
        i.twitch = Some(Twitch {
            enable: true,
            client_id: "id".into(),
            client_secret: "secret".into(),
        });
        i.youtube = Some(YouTube {
            enable: true,
            public_api_key: String::new(),
        });
    });
    let mut f = Fixture::with(Limits::default(), Some(&c));
    let i = f.s.client_config().integrations.unwrap();
    assert!(i.angelthump && i.twitch && !i.youtube && !i.swarm);
    f.s.set_config(Some(&angelthump_and_youtube()));
    let i = f.s.client_config().integrations.unwrap();
    assert!(i.angelthump && !i.twitch && i.youtube);
}

#[test]
fn an_asset_bundle_over_its_bound_goes_without_the_icon() {
    let f = Fixture::new();
    let icon = |n: usize| Image {
        r#type: 5,
        data: vec![1; n],
        ..Default::default()
    };
    let b = f.s.asset_bundle(Some(icon(1000)));
    assert_eq!(b.icon, Some(icon(1000)));
    assert_eq!(b.directory, Some(f.s.client_config()));
    let b =
        f.s.asset_bundle(Some(icon(crate::server::MAX_ASSET_BUNDLE)));
    assert_eq!(b.icon, None);
}

// --- snippets ---------------------------------------------------------------

fn image(n: usize) -> ListingSnippetImage {
    ListingSnippetImage {
        source_oneof: Some(SourceOneof::Image(Image {
            r#type: 4,
            data: vec![3; n],
            ..Default::default()
        })),
    }
}

fn signed(mut s: ListingSnippet, key: &strims_proto::types::Key) -> ListingSnippet {
    s.sign_message(key).unwrap();
    s
}

fn snippet(title: &str) -> ListingSnippet {
    signed(
        ListingSnippet {
            title: title.into(),
            live: true,
            channel_logo: Some(ListingSnippetImage {
                source_oneof: Some(SourceOneof::Url("https://x/logo.png".into())),
            }),
            ..Default::default()
        },
        &key_from_seed(&[1; 32]),
    )
}

/// The Subscribe actions so far: (sub, listing, host, swarm).
fn subscribed(f: &mut Fixture) -> Vec<(SubId, u64, ID, SwarmId)> {
    f.take_actions()
        .into_iter()
        .filter_map(|a| match a {
            ServerAction::Subscribe {
                sub,
                listing_id,
                host,
                swarm_id,
            } => Some((sub, listing_id, host, swarm_id)),
            _ => None,
        })
        .collect()
}

fn unsubscribed(actions: &[ServerAction]) -> Vec<SubId> {
    actions
        .iter()
        .filter_map(|a| match a {
            ServerAction::Unsubscribe { sub } => Some(*sub),
            _ => None,
        })
        .collect()
}

fn send(f: &mut Fixture, sub: SubId, from: &ListingSnippet, to: &ListingSnippet) {
    for d in split(diff(from, to)) {
        f.s.snippet_delta(f.now, sub, &d);
        f.check();
    }
}

#[test]
fn a_media_listing_subscribes_to_its_first_publisher() {
    let mut f = Fixture::new();
    f.publish(&caller(1, 1), chat(1)).unwrap();
    assert!(subscribed(&mut f).is_empty());
    assert_eq!(f.publish(&caller(2, 2), media(5)), Ok(2));
    f.publish(&caller(3, 3), media(5)).unwrap();
    assert_eq!(
        subscribed(&mut f),
        vec![(SubId(1), 2, ID::from_bytes(&[2; 32]), swarm(5))]
    );
}

#[test]
fn a_verified_snippet_is_shown_and_broadcast() {
    let mut f = Fixture::new();
    f.publish(&caller(1, 1), media(5)).unwrap();
    f.tick();
    let (sub, ..) = subscribed(&mut f)[0].clone();
    let a = snippet("a");
    send(&mut f, sub, &ListingSnippet::default(), &a);
    assert_eq!(f.s.snippet(1), Some(&a));
    let records = f.run_until(f.now + Duration::SECOND);
    assert_eq!(describe(&records), s(&["change 1", "count 1 1/1"]));
    let Some(Body::ListingChange(c)) = &records[0].events[0].body else {
        panic!()
    };
    assert_eq!(c.snippet.as_ref(), Some(&a));
    // updates are diffs from what was sent
    let b = snippet("b");
    send(&mut f, sub, &a, &b);
    assert_eq!(f.s.snippet(1), Some(&b));
    assert_eq!(f.s.metrics().snippets_shown, 2);
    // a delta that changes nothing shows nothing new
    send(&mut f, sub, &b, &b);
    assert_eq!(f.tick(), s(&["change 1", "count 1 1/1"]));
    assert_eq!(f.tick(), Vec::<String>::new());
}

#[test]
fn a_both_images_snippet_shows_once_its_second_delta_arrives() {
    let mut f = Fixture::new();
    f.publish(&caller(1, 1), media(5)).unwrap();
    let (sub, ..) = subscribed(&mut f)[0].clone();
    let a = signed(
        ListingSnippet {
            title: "a".into(),
            thumbnail: Some(image(100)),
            channel_logo: Some(image(50)),
            ..Default::default()
        },
        &key_from_seed(&[2; 32]),
    );
    let parts = split(diff(&ListingSnippet::default(), &a));
    assert_eq!(parts.len(), 2);
    f.s.snippet_delta(f.now, sub, &parts[0]);
    assert_eq!(f.s.snippet(1), Some(&ListingSnippet::default()));
    f.s.snippet_delta(f.now, sub, &parts[1]);
    assert_eq!(f.s.snippet(1), Some(&a));
    assert_eq!(f.s.metrics().snippets_unverified, 1);
}

#[test]
fn a_snippet_that_fails_verification_is_not_shown() {
    let mut f = Fixture::new();
    f.publish(&caller(1, 1), media(5)).unwrap();
    let (sub, ..) = subscribed(&mut f)[0].clone();
    let mut forged = snippet("a");
    forged.title = "forged".into();
    let d = diff(&ListingSnippet::default(), &forged);
    for _ in 0..MAX_UNVERIFIED {
        f.s.snippet_delta(f.now, sub, &d);
        f.check();
    }
    assert_eq!(f.s.snippet(1), Some(&ListingSnippet::default()));
    assert_eq!(unsubscribed(&f.take_actions()), vec![sub]);
    assert_eq!(f.s.metrics().snippets_rejected, 1);
    // deltas of the ended subscription are ignored
    send(&mut f, sub, &ListingSnippet::default(), &snippet("a"));
    assert_eq!(f.s.snippet(1), Some(&ListingSnippet::default()));
    assert_eq!(f.s.metrics().stale_deltas, 1);
    // the retry starts from an empty next snippet
    f.advance(MIN_SNIPPET_BACKOFF.as_secs() * 5 / 4 + 1);
    let (retry, ..) = subscribed(&mut f)[0].clone();
    assert_ne!(retry, sub);
    send(&mut f, retry, &ListingSnippet::default(), &snippet("a"));
    assert_eq!(f.s.snippet(1), Some(&snippet("a")));
}

#[test]
fn a_lost_delta_is_recovered_by_subscribing_again() {
    let mut f = Fixture::new();
    f.publish(&caller(1, 1), media(5)).unwrap();
    let (sub, ..) = subscribed(&mut f)[0].clone();
    let a = snippet("a");
    let b = snippet("b");
    let mut c = b.clone();
    c.description = "c".into();
    let c = signed(c, &key_from_seed(&[1; 32]));
    send(&mut f, sub, &ListingSnippet::default(), &a);
    // a→b is lost; b→c doesn't verify against a
    send(&mut f, sub, &b, &c);
    assert_eq!(f.s.snippet(1), Some(&a));
    // no verified delta within the snippet timeout ends the subscription
    f.advance(31);
    assert_eq!(unsubscribed(&f.take_actions()), vec![sub]);
    assert_eq!(f.s.metrics().snippet_timeouts, 1);
    f.advance(40);
    let (retry, ..) = subscribed(&mut f)[0].clone();
    send(&mut f, retry, &ListingSnippet::default(), &c);
    assert_eq!(f.s.snippet(1), Some(&c));
}

#[test]
fn a_subscription_without_a_verified_delta_times_out_and_backs_off() {
    let mut f = Fixture::new();
    f.publish(&caller(1, 1), media(5)).unwrap();
    let mut subs = vec![subscribed(&mut f)[0].0];
    let mut starts = vec![f.now];
    let mut prev = f.now;
    while subs.len() < 5 {
        let next = f.s.poll_timeout().unwrap();
        f.run_until(next);
        f.ping(&caller(1, 1)).unwrap();
        let new = subscribed(&mut f);
        if let Some((sub, ..)) = new.first() {
            subs.push(*sub);
            starts.push(f.now);
            assert!(f.now > prev);
            prev = f.now;
        }
    }
    let gaps: Vec<i64> = starts
        .windows(2)
        .map(|w| (w[1] - w[0]).as_secs() - 30)
        .collect();
    // 30 s timeout, then 30, 60, 120, 240 s of backoff with up to a
    // quarter of jitter
    for (gap, base) in gaps.iter().zip([30, 60, 120, 240]) {
        assert!((base..=base * 5 / 4 + 1).contains(gap), "{gaps:?}");
    }
}

#[test]
fn an_ended_subscription_is_retried_while_the_host_publishes() {
    let mut f = Fixture::new();
    let c = caller(1, 1);
    f.publish(&c, media(5)).unwrap();
    let (sub, ..) = subscribed(&mut f)[0].clone();
    f.s.snippet_ended(f.now, sub);
    f.s.snippet_ended(f.now, sub);
    f.check();
    assert!(unsubscribed(&f.take_actions()).is_empty());
    assert_eq!(f.s.metrics().snippets_ended, 1);
    f.advance(40);
    let (retry, ..) = subscribed(&mut f)[0].clone();
    f.s.snippet_ended(f.now, retry);
    f.check();
    f.unpublish(&c, 1).unwrap();
    assert!(f.s.snippet_source(1).is_none());
    f.advance(600);
    assert!(subscribed(&mut f).is_empty());
}

#[test]
fn a_delta_flood_ends_the_subscription() {
    let mut f = Fixture::new();
    f.publish(&caller(1, 1), media(5)).unwrap();
    let (sub, ..) = subscribed(&mut f)[0].clone();
    let a = snippet("a");
    send(&mut f, sub, &ListingSnippet::default(), &a);
    for _ in 1..MAX_DELTAS_PER_MINUTE {
        f.s.snippet_delta(f.now, sub, &diff(&a, &a));
    }
    f.check();
    assert!(unsubscribed(&f.take_actions()).is_empty());
    f.s.snippet_delta(f.now, sub, &diff(&a, &a));
    f.check();
    assert_eq!(unsubscribed(&f.take_actions()), vec![sub]);
    assert_eq!(f.s.metrics().snippet_floods, 1);
    // the shown snippet stays
    assert_eq!(f.s.snippet(1), Some(&a));
}

#[test]
fn deltas_spread_over_minutes_are_not_a_flood() {
    let mut f = Fixture::new();
    let c = caller(1, 1);
    f.publish(&c, media(5)).unwrap();
    let (sub, ..) = subscribed(&mut f)[0].clone();
    let a = snippet("a");
    send(&mut f, sub, &ListingSnippet::default(), &a);
    for _ in 0..5 {
        for _ in 0..MAX_DELTAS_PER_MINUTE - 1 {
            f.s.snippet_delta(f.now, sub, &diff(&a, &a));
        }
        f.advance(61);
        f.ping(&c).unwrap();
    }
    assert!(unsubscribed(&f.take_actions()).is_empty());
}

#[test]
fn an_oversized_snippet_ends_the_subscription() {
    let mut f = Fixture::new();
    f.publish(&caller(1, 1), media(5)).unwrap();
    let (sub, ..) = subscribed(&mut f)[0].clone();
    let d = ListingSnippetDelta {
        description: Some("d".repeat(crate::snippet::MAX_TEXT_BYTES + 1)),
        ..Default::default()
    };
    f.s.snippet_delta(f.now, sub, &d);
    f.check();
    assert_eq!(unsubscribed(&f.take_actions()), vec![sub]);
    assert_eq!(f.s.metrics().snippets_oversized, 1);
}

#[test]
fn a_source_that_stops_publishing_moves_to_the_newest_publisher() {
    let mut f = Fixture::new();
    let (a, b, c) = (caller(1, 1), caller(2, 2), caller(3, 3));
    f.publish(&a, media(5)).unwrap();
    f.publish(&b, media(5)).unwrap();
    f.now = at(1);
    f.publish(&c, media(5)).unwrap();
    let (first, ..) = subscribed(&mut f)[0].clone();
    let shown = snippet("a");
    send(&mut f, first, &ListingSnippet::default(), &shown);
    f.unpublish(&a, 1).unwrap();
    let actions = f.take_actions();
    assert_eq!(unsubscribed(&actions), vec![first]);
    f.actions = actions;
    let moved = subscribed(&mut f);
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].2, c.host);
    // the shown snippet stays until the new source verifies
    assert_eq!(f.s.snippet(1), Some(&shown));
    // a source that expires moves too
    f.unpublish(&c, 1).unwrap();
    assert_eq!(subscribed(&mut f)[0].2, b.host);
    f.unpublish(&b, 1).unwrap();
    assert!(subscribed(&mut f).is_empty());
    assert_eq!(f.s.snippet_source(1), None);
}

#[test]
fn a_source_whose_host_left_during_its_backoff_moves_on_retry() {
    let mut f = Fixture::new();
    let (a, b) = (caller(1, 1), caller(2, 2));
    f.publish(&a, media(5)).unwrap();
    f.publish(&b, media(5)).unwrap();
    let (sub, ..) = subscribed(&mut f)[0].clone();
    f.s.snippet_ended(f.now, sub);
    // the source leaves: the listing moves to b at once
    f.unpublish(&a, 1).unwrap();
    assert_eq!(subscribed(&mut f)[0].2, b.host);
}

#[test]
fn removing_a_listing_ends_its_subscription() {
    let mut f = Fixture::new();
    let a = caller(1, 1);
    f.publish(&a, media(5)).unwrap();
    let (sub, ..) = subscribed(&mut f)[0].clone();
    f.tick();
    f.s.listing_moderated(
        f.now,
        1,
        Some(ListingModeration {
            is_banned: Some(true),
            ..Default::default()
        }),
    );
    f.tick();
    assert_eq!(unsubscribed(&f.take_actions()), vec![sub]);
    f.s.snippet_delta(f.now, sub, &diff(&ListingSnippet::default(), &snippet("a")));
    f.check();
    assert_eq!(f.s.metrics().stale_deltas, 1);
}

#[test]
fn an_unpublished_media_listing_stops_its_snippet_source_at_once() {
    let mut f = Fixture::new();
    let a = caller(1, 1);
    f.publish(&a, media(5)).unwrap();
    let (sub, ..) = subscribed(&mut f)[0].clone();
    f.unpublish(&a, 1).unwrap();
    assert_eq!(unsubscribed(&f.take_actions()), vec![sub]);
    assert_eq!(f.tick(), s(&["unpublish 1", "online 1 user1 []"]));
}

// --- embeds -----------------------------------------------------------------

fn embed(service: EmbedService, id: &str) -> Listing {
    crate::embed::listing(service, id)
}

fn embeds_config(f: impl FnOnce(&mut Integrations)) -> ServerConfig {
    config(|c| {
        let mut i = Integrations::default();
        f(&mut i);
        c.integrations = Some(i);
    })
}

fn angelthump_and_youtube() -> ServerConfig {
    embeds_config(|i| {
        i.angelthump = Some(AngelThump { enable: true });
        i.youtube = Some(YouTube {
            enable: true,
            public_api_key: "key".into(),
        });
    })
}

fn embed_fixture() -> Fixture {
    Fixture::with(Limits::default(), Some(&angelthump_and_youtube()))
}

type Ids = BTreeMap<EmbedService, Vec<String>>;

/// The LoadEmbeds actions so far.
fn loads(f: &mut Fixture) -> Vec<(LoadId, Ids)> {
    f.take_actions()
        .into_iter()
        .filter_map(|a| match a {
            ServerAction::LoadEmbeds { load, ids } => Some((load, ids)),
            _ => None,
        })
        .collect()
}

fn ids(v: &[(EmbedService, &[&str])]) -> Ids {
    v.iter()
        .map(|(s, ids)| (*s, ids.iter().map(|i| (*i).to_owned()).collect()))
        .collect()
}

/// Runs the server to now, and returns the load that starts.
fn started(f: &mut Fixture) -> LoadId {
    f.advance(0);
    let l = loads(f);
    assert_eq!(l.len(), 1, "{l:?}");
    l[0].0
}

fn loaded(title: &str) -> ListingSnippet {
    ListingSnippet {
        title: title.into(),
        live: true,
        channel_logo: Some(ListingSnippetImage {
            source_oneof: Some(SourceOneof::Url("https://x/logo.png".into())),
        }),
        ..Default::default()
    }
}

fn load_batch(f: &mut Fixture, load: LoadId, service: EmbedService, v: &[(&str, ListingSnippet)]) {
    let snippets = v
        .iter()
        .map(|(id, s)| ((*id).to_owned(), s.clone()))
        .collect();
    f.s.embeds_loaded(f.now, load, service, snippets);
    f.check();
}

fn end_load(f: &mut Fixture, load: LoadId) {
    f.s.embeds_load_ended(f.now, load);
    f.check();
}

#[test]
fn embed_listings_are_taken_only_of_enabled_services_with_credentials() {
    use EmbedService::*;
    let mut f = embed_fixture();
    let a = caller(1, 1);
    assert_eq!(f.publish(&a, embed(AngelThump, "psrngafk")), Ok(1));
    assert_eq!(f.publish(&a, embed(YouTube, "DDTGlyJVNVI")), Ok(2));
    for refused in [embed(TwitchStream, "destiny"), embed(TwitchVod, "1")] {
        assert_eq!(
            f.publish(&a, refused),
            Err(DirectoryError::UnsupportedEmbed)
        );
    }
    assert_eq!(
        f.publish(&a, embed(YouTube, "short")),
        Err(DirectoryError::InvalidEmbedId)
    );

    let no_secret = embeds_config(|i| {
        i.twitch = Some(Twitch {
            enable: true,
            client_id: "id".into(),
            client_secret: String::new(),
        });
    });
    let mut f = Fixture::with(Limits::default(), Some(&no_secret));
    assert_eq!(
        f.publish(&a, embed(TwitchStream, "destiny")),
        Err(DirectoryError::UnsupportedEmbed)
    );
    assert_eq!(f.s.metrics().users_created, 0);
}

#[test]
fn embed_loads_run_one_at_a_time_at_their_interval() {
    use EmbedService::*;
    let mut f = embed_fixture();
    let a = caller(1, 1);
    f.publish(&a, chat(1)).unwrap();
    f.advance(30);
    assert!(loads(&mut f).is_empty());

    // a new embed listing starts a load at once
    f.publish(&a, embed(AngelThump, "a")).unwrap();
    assert_eq!(f.s.poll_timeout(), Some(f.now));
    f.advance(0);
    let l = loads(&mut f);
    assert_eq!(l.len(), 1);
    let (first, first_ids) = &l[0];
    assert_eq!(*first_ids, ids(&[(AngelThump, &["a"])]));
    let first_at = f.now;

    // another waits for the load in flight, and for the spacing
    f.publish(&a, embed(YouTube, "DDTGlyJVNVI")).unwrap();
    f.advance(5);
    assert!(loads(&mut f).is_empty());
    end_load(&mut f, *first);
    f.advance(4);
    assert!(loads(&mut f).is_empty());
    f.advance(1);
    assert_eq!(f.now, first_at + MIN_EMBED_LOAD_SPACING);
    let l = loads(&mut f);
    assert_eq!(l.len(), 1);
    let (second, second_ids) = &l[0];
    assert_eq!(
        *second_ids,
        ids(&[(AngelThump, &["a"]), (YouTube, &["DDTGlyJVNVI"])])
    );
    assert_eq!(f.s.embed_load(), Some(*second));
    end_load(&mut f, *second);

    // then one every interval
    f.advance(59);
    assert!(loads(&mut f).is_empty());
    f.advance(1);
    let third = loads(&mut f);
    assert_eq!(third.len(), 1);
    end_load(&mut f, third[0].0);

    // without embed listings, loads stop
    f.unpublish(&a, 2).unwrap();
    f.unpublish(&a, 3).unwrap();
    f.advance(120);
    assert!(loads(&mut f).is_empty());
    assert!(f.s.poll_timeout().unwrap() <= f.now + Duration::SECOND);
    assert_eq!(f.s.metrics().embed_loads, 3);
}

#[test]
fn a_loaded_snippet_is_applied_only_when_it_changes() {
    use EmbedService::*;
    let mut f = embed_fixture();
    let a = caller(1, 1);
    let id = f.publish(&a, embed(AngelThump, "a")).unwrap();
    let load = started(&mut f);
    assert_eq!(
        f.tick(),
        s(&["change 1", "count 1 1/1", "online 1 user1 [1]"])
    );

    load_batch(
        &mut f,
        load,
        AngelThump,
        &[("a", loaded("t")), ("other", loaded("x"))],
    );
    assert_eq!(f.s.snippet(id), Some(&loaded("t")));
    assert_eq!(f.tick(), s(&["change 1", "count 1 1/1"]));
    load_batch(&mut f, load, AngelThump, &[("a", loaded("t"))]);
    assert_eq!(f.tick(), Vec::<String>::new());
    assert_eq!(f.s.metrics().embed_snippets_shown, 1);

    let oversized = loaded(&"x".repeat(MAX_TEXT_BYTES + 1));
    load_batch(&mut f, load, AngelThump, &[("a", oversized)]);
    assert_eq!(f.s.snippet(id), Some(&loaded("t")));
    assert_eq!(f.s.metrics().embed_snippets_rejected, 1);

    // batches of another service, or of a load that ended, change nothing
    load_batch(&mut f, load, YouTube, &[("a", loaded("y"))]);
    load_batch(&mut f, load, TwitchStream, &[("a", loaded("y"))]);
    end_load(&mut f, load);
    load_batch(&mut f, load, AngelThump, &[("a", loaded("late"))]);
    assert_eq!(f.s.snippet(id), Some(&loaded("t")));
    assert_eq!(f.s.metrics().stale_embed_batches, 2);
    assert_eq!(f.tick(), Vec::<String>::new());
}

#[test]
fn a_failed_batch_keeps_the_snippet() {
    use EmbedService::*;
    let mut f = embed_fixture();
    let a = caller(1, 1);
    let at = f.publish(&a, embed(AngelThump, "a")).unwrap();
    let yt = f.publish(&a, embed(YouTube, "DDTGlyJVNVI")).unwrap();
    let load = started(&mut f);
    load_batch(&mut f, load, AngelThump, &[("a", loaded("a1"))]);
    load_batch(&mut f, load, YouTube, &[("DDTGlyJVNVI", loaded("y1"))]);
    end_load(&mut f, load);
    f.tick();

    // the next load's AngelThump batch fails, and its YouTube batch
    // answers nothing for the video
    f.advance(59);
    let load = started(&mut f);
    load_batch(&mut f, load, YouTube, &[]);
    end_load(&mut f, load);
    assert_eq!(f.s.snippet(at), Some(&loaded("a1")));
    assert_eq!(f.s.snippet(yt), Some(&loaded("y1")));
    assert_eq!(f.tick(), Vec::<String>::new());
}

#[test]
fn disabling_an_embed_service_removes_its_listings() {
    use EmbedService::{AngelThump as At, YouTube as Yt};
    let mut f = embed_fixture();
    let a = caller(1, 1);
    let at = f.publish(&a, embed(At, "a")).unwrap();
    let yt = f.publish(&a, embed(Yt, "DDTGlyJVNVI")).unwrap();
    let c = f.publish(&a, chat(1)).unwrap();
    let load = started(&mut f);
    f.tick();

    let youtube_only = embeds_config(|i| {
        i.youtube = Some(YouTube {
            enable: true,
            public_api_key: "key".into(),
        });
    });
    f.s.set_config(Some(&youtube_only));
    f.check();
    assert_eq!(
        f.tick(),
        s(&[
            &format!("unpublish {at}"),
            &format!("online 1 user1 [{yt}, {c}]")
        ])
    );
    assert_eq!(f.s.listing_ids(), [yt, c]);
    assert_eq!(
        f.publish(&a, embed(At, "a")),
        Err(DirectoryError::UnsupportedEmbed)
    );
    load_batch(&mut f, load, At, &[("a", loaded("a1"))]);
    assert_eq!(f.s.metrics().stale_embed_batches, 1);
    end_load(&mut f, load);
    f.advance(60);
    assert_eq!(loads(&mut f)[0].1, ids(&[(Yt, &["DDTGlyJVNVI"])]));
}
