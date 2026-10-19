// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use prost::Message;
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_kademlia::ID;
use strims_proto::network::v1::directory::event::{
    Body, ListingChange, Unpublish, UserCountChange, UserPresenceChange,
};
use strims_proto::network::v1::directory::listing::{Chat, Content};
use strims_proto::network::v1::directory::listing_query::Query;
use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;
use strims_proto::network::v1::directory::{
    Event, EventBroadcast, Listing as ProtoListing, ListingModeration, ListingQuery,
    ListingSnippet, ListingSnippetImage, SignedListingSnippet,
};
use strims_proto::types::Image;

use crate::view::{Listing, User, View, ViewEvent};
use crate::{Caller, Limits, Server, ServerAction, ViewLimits};

fn chat(n: u8) -> ProtoListing {
    ProtoListing {
        content: Some(Content::Chat(Chat {
            key: vec![n; 32],
            name: format!("chat {n}"),
        })),
    }
}

fn change(id: u64, n: u8) -> Event {
    Event {
        body: Some(Body::ListingChange(ListingChange {
            id,
            listing: Some(chat(n)),
            snippet: Some(SignedListingSnippet::default()),
            moderation: None,
            snippet_images: Vec::new(),
        })),
    }
}

/// `s` as an event carries a snippet the server made: unsigned, with its
/// images apart.
fn unsigned(s: &ListingSnippet) -> (SignedListingSnippet, Vec<Vec<u8>>) {
    let (body, images) = crate::snippet::seal(s, 0);
    let envelope = SignedListingSnippet {
        body: body.encode_to_vec(),
        signature: Vec::new(),
    };
    (envelope, images.into_iter().map(|(_, b)| b).collect())
}

fn count(id: u64, users: u32, recent: u32) -> Event {
    Event {
        body: Some(Body::UserCountChange(UserCountChange {
            id,
            user_count: users,
            recent_user_count: recent,
        })),
    }
}

fn unpublish(id: u64) -> Event {
    Event {
        body: Some(Body::Unpublish(Unpublish { id })),
    }
}

fn online(id: u64, alias: &str, ids: &[u64]) -> Event {
    Event {
        body: Some(Body::UserPresenceChange(UserPresenceChange {
            id,
            alias: alias.into(),
            peer_key: vec![u8::try_from(id).unwrap(); 32],
            online: true,
            listing_ids: ids.to_vec(),
        })),
    }
}

fn offline(id: u64) -> Event {
    Event {
        body: Some(Body::UserPresenceChange(UserPresenceChange {
            id,
            alias: String::new(),
            peer_key: vec![u8::try_from(id).unwrap(); 32],
            online: false,
            listing_ids: Vec::new(),
        })),
    }
}

fn user(id: u64, alias: &str) -> User {
    User {
        id,
        alias: alias.into(),
        peer_key: vec![u8::try_from(id).unwrap(); 32],
    }
}

fn apply(v: &mut View, events: Vec<Event>) -> Vec<ViewEvent> {
    v.handle_broadcast(&EventBroadcast { events });
    v.check().unwrap();
    std::iter::from_fn(|| v.poll_event()).collect()
}

/// Events as short strings.
fn kinds(events: &[ViewEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| match e {
            ViewEvent::ListingChanged(l) => format!("changed {}", l.id),
            ViewEvent::ListingUnpublished(l) => format!("unpublished {}", l.id),
            ViewEvent::UserCountChanged {
                id,
                user_count,
                recent_user_count,
            } => format!("count {id} {user_count}/{recent_user_count}"),
            ViewEvent::UserJoined { listing, user } => {
                format!("join {listing} {} {}", user.id, user.alias)
            }
            ViewEvent::UserParted { listing, user } => {
                format!("part {listing} {} {}", user.id, user.alias)
            }
            ViewEvent::UserRenamed { listing, user } => {
                format!("rename {listing} {} {}", user.id, user.alias)
            }
            ViewEvent::Lagged => "lagged".into(),
        })
        .collect()
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

fn ids(users: Vec<User>) -> Vec<u64> {
    users.iter().map(|u| u.id).collect()
}

#[test]
fn a_listing_change_adds_or_replaces_a_listing_keeping_its_counts() {
    let mut v = View::new(ViewLimits::default());
    let e = apply(&mut v, vec![change(1, 1), count(1, 3, 4)]);
    assert_eq!(kinds(&e), s(&["changed 1", "count 1 3/4"]));
    let moderation = ListingModeration {
        is_mature: Some(true),
        ..Default::default()
    };
    let replaced = Event {
        body: Some(Body::ListingChange(ListingChange {
            id: 1,
            listing: Some(chat(2)),
            snippet: None,
            moderation: Some(moderation.clone()),
            snippet_images: Vec::new(),
        })),
    };
    let e = apply(&mut v, vec![replaced]);
    let expected = Listing {
        id: 1,
        listing: chat(2),
        snippet: None,
        moderation: Some(moderation),
        user_count: 3,
        recent_user_count: 4,
    };
    assert_eq!(e, vec![ViewEvent::ListingChanged(expected.clone())]);
    assert_eq!(v.listing(1), Some(&expected));
    // the listing is found by its new content only
    let q = |n| ListingQuery {
        query: Some(Query::Listing(chat(n))),
    };
    assert_eq!(v.listing_by_query(&q(2)), Some(&expected));
    assert_eq!(v.listing_by_query(&q(1)), None);
}

#[test]
fn counts_of_unknown_listings_are_skipped() {
    let mut v = View::new(ViewLimits::default());
    assert!(apply(&mut v, vec![count(5, 1, 1), unpublish(5)]).is_empty());
    assert_eq!(v.metrics().unknown_listings, 2);
}

#[test]
fn a_presence_joins_renames_and_parts_in_order() {
    let mut v = View::new(ViewLimits::default());
    apply(&mut v, vec![change(1, 1), change(2, 2), change(3, 3)]);
    let e = apply(&mut v, vec![online(7, "a", &[2, 1, 2])]);
    assert_eq!(kinds(&e), s(&["join 1 7 a", "join 2 7 a"]));
    let e = apply(&mut v, vec![online(7, "b", &[2, 3])]);
    assert_eq!(
        kinds(&e),
        s(&["rename 1 7 b", "rename 2 7 b", "join 3 7 b", "part 1 7 b"])
    );
    assert_eq!(ids(v.listing_users(1)), Vec::<u64>::new());
    assert_eq!(ids(v.listing_users(2)), vec![7]);
    assert_eq!(ids(v.listing_users(3)), vec![7]);
    assert_eq!(v.users().cloned().collect::<Vec<_>>(), vec![user(7, "b")]);
    // the same presence again changes nothing
    assert!(apply(&mut v, vec![online(7, "b", &[3, 2])]).is_empty());
}

#[test]
fn a_part_removes_the_user_from_the_listing_it_leaves() {
    let mut v = View::new(ViewLimits::default());
    apply(
        &mut v,
        vec![
            change(1, 1),
            change(2, 2),
            online(7, "a", &[1, 2]),
            online(8, "b", &[1]),
        ],
    );
    apply(&mut v, vec![online(7, "a", &[2])]);
    assert_eq!(ids(v.listing_users(1)), vec![8]);
    assert_eq!(ids(v.listing_users(2)), vec![7]);
    assert_eq!(v.listings_of_peer(&[7; 32]).len(), 1);
    assert_eq!(v.listings_of_peer(&[7; 32])[0].id, 2);
}

#[test]
fn an_offline_presence_parts_and_removes_the_user() {
    let mut v = View::new(ViewLimits::default());
    apply(
        &mut v,
        vec![change(1, 1), change(2, 2), online(7, "a", &[1, 2])],
    );
    let e = apply(&mut v, vec![offline(7), offline(9)]);
    assert_eq!(kinds(&e), s(&["part 1 7 a", "part 2 7 a"]));
    assert_eq!(v.users().count(), 0);
    assert!(v.listings_of_peer(&[7; 32]).is_empty());
}

#[test]
fn a_user_joins_a_listing_named_before_the_listing_arrives() {
    let mut v = View::new(ViewLimits::default());
    let e = apply(&mut v, vec![online(7, "a", &[1]), online(8, "b", &[1, 2])]);
    assert!(e.is_empty());
    let e = apply(&mut v, vec![change(1, 1)]);
    assert_eq!(kinds(&e), s(&["changed 1", "join 1 7 a", "join 1 8 b"]));
    assert_eq!(ids(v.listing_users(1)), vec![7, 8]);
}

#[test]
fn an_unpublish_ends_the_listings_memberships_without_parts() {
    let mut v = View::new(ViewLimits::default());
    apply(&mut v, vec![change(1, 1), online(7, "a", &[1])]);
    let e = apply(&mut v, vec![unpublish(1)]);
    assert_eq!(kinds(&e), s(&["unpublished 1"]));
    assert!(v.listings_of_peer(&[7; 32]).is_empty());
    // a listing with the same id later starts without the user
    let e = apply(&mut v, vec![change(1, 1)]);
    assert_eq!(kinds(&e), s(&["changed 1"]));
}

#[test]
fn a_reset_parts_every_user_and_unpublishes_every_listing() {
    let mut v = View::new(ViewLimits::default());
    apply(
        &mut v,
        vec![
            change(1, 1),
            change(2, 2),
            online(7, "a", &[1, 2]),
            online(8, "b", &[2]),
        ],
    );
    v.reset();
    v.check().unwrap();
    let e: Vec<ViewEvent> = std::iter::from_fn(|| v.poll_event()).collect();
    assert_eq!(
        kinds(&e),
        s(&[
            "part 1 7 a",
            "unpublished 1",
            "part 2 7 a",
            "part 2 8 b",
            "unpublished 2"
        ])
    );
    assert_eq!(v.listings().count(), 0);
    assert_eq!(v.users().count(), 0);
    assert_eq!(v.metrics().resets, 1);
}

#[test]
fn listings_are_found_by_id_or_listing() {
    let mut v = View::new(ViewLimits::default());
    apply(&mut v, vec![change(4, 1)]);
    let by_id = ListingQuery {
        query: Some(Query::Id(4)),
    };
    let by_listing = ListingQuery {
        query: Some(Query::Listing(chat(1))),
    };
    assert_eq!(v.listing_by_query(&by_id).map(|l| l.id), Some(4));
    assert_eq!(v.listing_by_query(&by_listing).map(|l| l.id), Some(4));
    assert_eq!(v.listing_by_query(&ListingQuery::default()), None);
}

#[test]
fn malformed_events_and_records_are_skipped_and_counted() {
    let mut v = View::new(ViewLimits::default());
    let no_listing = Event {
        body: Some(Body::ListingChange(ListingChange {
            id: 1,
            ..Default::default()
        })),
    };
    let mut long_alias = online(7, "a", &[]);
    if let Some(Body::UserPresenceChange(p)) = &mut long_alias.body {
        p.alias = "a".repeat(crate::MAX_ALIAS_BYTES + 1);
    }
    let mut long_key = online(7, "a", &[]);
    if let Some(Body::UserPresenceChange(p)) = &mut long_key.body {
        p.peer_key = vec![1; crate::MAX_PEER_KEY_BYTES + 1];
    }
    let e = apply(
        &mut v,
        vec![no_listing, long_alias, long_key, Event { body: None }],
    );
    assert!(e.is_empty());
    assert_eq!(v.metrics().malformed_events, 4);
    assert!(!v.handle_record(&[0xff, 0xff, 0xff]));
    assert_eq!(v.metrics().malformed_records, 1);
    let good = EventBroadcast {
        events: vec![change(1, 1)],
    }
    .encode_to_vec();
    assert!(v.handle_record(&good));
    assert_eq!(v.listing(1).map(|l| l.id), Some(1));
}

#[test]
fn listings_and_users_beyond_the_limits_are_dropped() {
    let limits = ViewLimits {
        max_listings: 2,
        max_users: 2,
        max_listing_ids: 3,
        max_memberships: 5,
        ..ViewLimits::default()
    };
    let mut v = View::new(limits);
    apply(&mut v, vec![change(1, 1), change(2, 2), change(3, 3)]);
    assert_eq!(v.listings().count(), 2);
    assert_eq!(v.metrics().listings_dropped, 1);
    // updates of held listings still apply
    apply(&mut v, vec![change(2, 9)]);
    assert_eq!(v.listing(2).unwrap().listing, chat(9));

    apply(
        &mut v,
        vec![
            online(7, "a", &[1, 2, 10, 11, 12]),
            online(8, "b", &[1, 2, 13]),
            online(9, "c", &[1]),
        ],
    );
    assert_eq!(v.users().count(), 2);
    assert_eq!(v.metrics().users_dropped, 1);
    // 7 keeps its lowest 3 ids; 8 has room for 2 of its 3
    assert_eq!(v.metrics().ids_dropped, 2 + 1);
    assert_eq!(ids(v.listing_users(1)), vec![7, 8]);
    assert_eq!(ids(v.listing_users(2)), vec![7, 8]);
    // ids freed by a part make room
    apply(
        &mut v,
        vec![online(7, "a", &[]), online(8, "b", &[1, 2, 13])],
    );
    assert_eq!(v.listing_users(1).len(), 1);
}

fn image(n: usize) -> ListingSnippetImage {
    ListingSnippetImage {
        source_oneof: Some(SourceOneof::Image(Image {
            data: vec![1; n],
            ..Default::default()
        })),
    }
}

#[test]
fn an_oversized_listing_drops_its_images_then_its_snippet() {
    let limits = ViewLimits {
        max_listing_bytes: 4096,
        ..ViewLimits::default()
    };
    let mut v = View::new(limits);
    let with = |id: u64, title: usize, img: usize, name: usize| {
        let (snippet, snippet_images) = unsigned(&ListingSnippet {
            title: "t".repeat(title),
            thumbnail: Some(image(img)),
            ..Default::default()
        });
        Event {
            body: Some(Body::ListingChange(ListingChange {
                id,
                listing: Some(ProtoListing {
                    content: Some(Content::Chat(Chat {
                        key: vec![1; 32],
                        name: "n".repeat(name),
                    })),
                }),
                snippet: Some(snippet),
                moderation: None,
                snippet_images,
            })),
        }
    };
    apply(
        &mut v,
        vec![
            with(1, 10, 10, 10),
            with(2, 10, 5000, 10),
            with(3, 5000, 10, 10),
            with(4, 10, 10, 5000),
        ],
    );
    let snippet = |id| v.listing(id).unwrap().snippet.clone();
    assert!(snippet(1).unwrap().thumbnail.is_some());
    assert_eq!(snippet(2).unwrap().thumbnail, None);
    assert_eq!(snippet(2).unwrap().title.len(), 10);
    assert_eq!(snippet(3), None);
    assert!(v.listing(4).is_none());
    let m = v.metrics();
    assert_eq!(
        (m.images_dropped, m.snippets_dropped, m.oversized_listings),
        (3, 2, 1)
    );
}

#[test]
fn a_snippet_takes_its_images_by_digest() {
    let mut v = View::new(ViewLimits::default());
    let content = ListingSnippet {
        title: "t".into(),
        thumbnail: Some(image(10)),
        channel_logo: Some(image(20)),
        ..Default::default()
    };
    let (snippet, mut images) = unsigned(&content);
    images.reverse();
    images.insert(1, image(30).encode_to_vec());
    let event = |images: Vec<Vec<u8>>| Event {
        body: Some(Body::ListingChange(ListingChange {
            id: 1,
            listing: Some(chat(1)),
            snippet: Some(snippet.clone()),
            moderation: None,
            snippet_images: images,
        })),
    };
    apply(&mut v, vec![event(images.clone())]);
    assert_eq!(v.listing(1).unwrap().snippet, Some(content));
    // without an image it names, the listing goes without its snippet
    images.remove(0);
    apply(&mut v, vec![event(images)]);
    assert_eq!(v.listing(1).unwrap().snippet, None);
    assert_eq!(v.metrics().malformed_snippets, 1);
}

#[test]
fn a_full_event_queue_drops_events_and_reports_lagged_once_drained() {
    let limits = ViewLimits {
        max_queued_events: 3,
        ..ViewLimits::default()
    };
    let mut v = View::new(limits);
    v.handle_broadcast(&EventBroadcast {
        events: vec![
            change(1, 1),
            change(2, 2),
            change(3, 3),
            change(4, 4),
            count(1, 1, 1),
        ],
    });
    v.check().unwrap();
    let e: Vec<ViewEvent> = std::iter::from_fn(|| v.poll_event()).collect();
    assert_eq!(
        kinds(&e),
        s(&["changed 1", "changed 2", "changed 3", "lagged"])
    );
    assert_eq!(v.metrics().events_dropped, 2);
    // the view itself holds everything
    assert_eq!(v.listings().count(), 4);
    assert_eq!(v.listing(1).unwrap().user_count, 1);
    // events flow again once drained
    let e = apply(&mut v, vec![count(2, 5, 5)]);
    assert_eq!(kinds(&e), s(&["count 2 5/5"]));

    let limits = ViewLimits {
        max_queued_bytes: 300,
        ..ViewLimits::default()
    };
    let mut v = View::new(limits);
    let e = apply(&mut v, vec![change(1, 1), change(2, 2), change(3, 3)]);
    assert_eq!(kinds(&e), s(&["changed 1", "changed 2", "lagged"]));
}

#[test]
fn a_view_of_the_server_s_broadcasts_holds_the_server_s_projection() {
    let t0 = Time::from_unix_millis(1_700_000_000_000);
    let mut server = Server::new(Limits::default(), None, det_rng(1), t0);
    let mut view = View::new(ViewLimits::default());
    let caller = |user: u8, host: u8| Caller {
        host_key: vec![host; 32],
        host: ID::from_bytes(&[host; 32]),
        peer_key: vec![user; 32],
        alias: format!("u{user}"),
    };
    let mut now = t0;
    let feed = |server: &mut Server, view: &mut View, now: Time| {
        server.handle_timeout(now);
        while let Some(a) = server.poll_action() {
            if let ServerAction::Broadcast(records) = a {
                for r in records {
                    assert!(view.handle_record(&r.encode_to_vec()));
                }
            }
        }
        view.check().unwrap();
        while view.poll_event().is_some() {}
    };
    server
        .publish(now, &caller(1, 1), chat(1), None, None)
        .unwrap();
    server
        .publish(now, &caller(2, 2), chat(2), None, None)
        .unwrap();
    let by_id = |id| ListingQuery {
        query: Some(Query::Id(id)),
    };
    server.join(now, &caller(3, 3), &by_id(1), None).unwrap();
    server.join(now, &caller(3, 4), &by_id(2), None).unwrap();
    server.join(now, &caller(2, 5), &by_id(1), None).unwrap();
    for _ in 0..3 {
        now += Duration::SECOND;
        feed(&mut server, &mut view, now);
    }
    assert_eq!(view.projection(), server.projection());
    assert_eq!(view.listing_users(1).len(), 3);

    server.part(now, &caller(3, 3), 1).unwrap();
    server.unpublish(now, &caller(2, 2), 2).unwrap();
    for _ in 0..3 {
        now += Duration::SECOND;
        feed(&mut server, &mut view, now);
    }
    assert_eq!(view.projection(), server.projection());
    assert_eq!(view.listings().count(), 1);

    // a view that starts late catches up at the refresh
    let mut late = View::new(ViewLimits::default());
    for _ in 0..300 {
        now += Duration::SECOND;
        feed(&mut server, &mut late, now);
    }
    assert_eq!(late.projection(), server.projection());
}
