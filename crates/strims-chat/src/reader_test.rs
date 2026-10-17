// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Tests panic on failure by design.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use strims_proto::chat::v1::emote_effect::{Effect, SpriteAnimation};
use strims_proto::chat::v1::message::Entities;
use strims_proto::chat::v1::message::entities::{Bounds, Link};
use strims_proto::chat::v1::{Emote, EmoteEffect, Modifier, Room, Tag};

use super::*;

fn event(body: &str, links: usize) -> ServerEvent {
    ServerEvent {
        body: Some(Body::Message(strims_proto::chat::v1::Message {
            server_time: 1_700_000_000_000,
            peer_key: vec![1; 32],
            nick: "nick".into(),
            body: body.into(),
            entities: Some(Entities {
                links: (0..links)
                    .map(|i| Link {
                        bounds: Some(Bounds {
                            start: i as u32,
                            end: i as u32 + 1,
                        }),
                        url: "https://a".into(),
                    })
                    .collect(),
                ..Default::default()
            }),
            viewed_listing: None,
        })),
    }
}

fn emote(id: u64) -> Emote {
    Emote {
        id,
        name: format!("E{id}"),
        enable: true,
        ..Default::default()
    }
}

fn unified() -> AssetBundle {
    AssetBundle {
        is_delta: false,
        room: Some(Room {
            name: "lobby".into(),
            css: String::new(),
        }),
        emotes: vec![emote(1), emote(2)],
        ..Default::default()
    }
}

fn delta(id: u64) -> AssetBundle {
    AssetBundle {
        is_delta: true,
        removed_ids: vec![1],
        emotes: vec![emote(id)],
        ..Default::default()
    }
}

#[test]
fn event_reader_yields_messages() {
    let mut r = EventReader::new(RoomLimits::default());
    let e = event("hello", 2);
    assert_eq!(r.handle_record(&e.encode_to_vec()), Some(e.clone()));
    assert_eq!(r.handle_record(&e.encode_to_vec()), Some(e));
    assert_eq!(r.metrics().events, 2);
}

#[test]
fn undecodable_first_record_is_partial() {
    let mut r = EventReader::new(RoomLimits::default());
    let e = event("hello", 2).encode_to_vec();
    // A key of wire type 7.
    let tail = [&[0xff][..], &[0x0b]].concat();
    assert_eq!(r.handle_record(&tail), None);
    assert_eq!(r.metrics().partial_records, 1);
    assert!(r.handle_record(&e).is_some());
    assert_eq!(r.handle_record(&tail), None);
    assert_eq!(r.metrics().malformed_records, 1);

    r.discontinuity();
    assert_eq!(r.handle_record(&e[..e.len() - 1]), None);
    assert_eq!(r.metrics().partial_records, 2);
    assert!(r.handle_record(&e).is_some());
    assert_eq!(r.metrics().events, 2);
}

#[test]
fn messages_over_the_bounds_are_dropped() {
    let limits = RoomLimits::default();
    let mut r = EventReader::new(limits);

    assert!(r.handle_record(&event("x", 1024).encode_to_vec()).is_some());
    assert_eq!(r.handle_record(&event("x", 1025).encode_to_vec()), None);
    assert_eq!(r.metrics().entity_overflows, 1);

    // A decode merges repeated messages and entities, so their entities
    // add up.
    let split = [
        event("x", 600).encode_to_vec(),
        event("x", 600).encode_to_vec(),
    ]
    .concat();
    assert_eq!(r.handle_record(&split), None);
    assert_eq!(r.metrics().entity_overflows, 2);

    let long = "x".repeat(limits.max_body_bytes + 1);
    assert_eq!(r.handle_record(&event(&long, 0).encode_to_vec()), None);
    assert_eq!(r.metrics().long_bodies, 1);

    let huge = "x".repeat(limits.max_event_record);
    assert_eq!(r.handle_record(&event(&huge, 0).encode_to_vec()), None);
    assert_eq!(r.metrics().oversized_records, 1);

    assert_eq!(
        r.handle_record(&ServerEvent::default().encode_to_vec()),
        None
    );
    assert_eq!(r.metrics().empty_events, 1);

    // A message field with a varint wire type.
    assert_eq!(r.handle_record(&[0xc8, 0x3e, 0x01]), None);
    assert_eq!(r.metrics().malformed_records, 1);
    assert_eq!(r.metrics().events, 1);
}

#[test]
fn asset_reader_drops_deltas_until_a_unified_bundle() {
    let mut r = AssetReader::new(RoomLimits::default());
    assert_eq!(r.handle_record(&delta(3).encode_to_vec()), None);
    assert_eq!(r.metrics().deltas_dropped, 1);

    assert_eq!(r.handle_record(&unified().encode_to_vec()), Some(unified()));
    assert_eq!(r.handle_record(&delta(3).encode_to_vec()), Some(delta(3)));

    r.discontinuity();
    assert_eq!(r.handle_record(&delta(4).encode_to_vec()), None);
    assert_eq!(r.handle_record(&delta(5).encode_to_vec()), None);
    assert_eq!(r.metrics().deltas_dropped, 3);
    assert_eq!(r.handle_record(&unified().encode_to_vec()), Some(unified()));
    assert_eq!(r.handle_record(&delta(6).encode_to_vec()), Some(delta(6)));
    assert_eq!(r.metrics().bundles, 4);
}

#[test]
fn asset_reader_skips_a_partial_first_record() {
    let mut r = AssetReader::new(RoomLimits::default());
    let u = unified().encode_to_vec();
    assert_eq!(r.handle_record(&u[1..]), None);
    assert_eq!(r.metrics().partial_records, 1);
    assert!(r.handle_record(&u).is_some());
    assert_eq!(r.handle_record(&[0x22, 0x05, 0x01]), None);
    assert_eq!(r.metrics().malformed_records, 1);

    let huge = vec![0; RoomLimits::default().max_asset_record + 1];
    assert_eq!(r.handle_record(&huge), None);
    assert_eq!(r.metrics().oversized_records, 1);
}

#[test]
fn asset_reader_decodes_as_prost_does() {
    let b = AssetBundle {
        is_delta: true,
        removed_ids: vec![7, 1 << 40],
        room: Some(Room {
            name: "lobby".into(),
            css: "x".into(),
        }),
        emotes: vec![emote(1)],
        modifiers: vec![Modifier {
            id: 2,
            name: "spin".into(),
            ..Default::default()
        }],
        tags: vec![Tag {
            id: 3,
            name: "nsfw".into(),
            ..Default::default()
        }],
        icon: Some(strims_proto::r#type::Image {
            data: vec![1, 2, 3],
            ..Default::default()
        }),
    };
    let mut record = unified().encode_to_vec();
    record.extend(b.encode_to_vec());
    // An unpacked removed id and an unknown field.
    record.extend([0x10, 0x09, 0xf8, 0x01, 0x01]);
    let want = AssetBundle::decode(&record[..]).unwrap();
    assert_eq!(want.removed_ids, vec![7, 1 << 40, 9]);

    let mut r = AssetReader::new(RoomLimits::default());
    r.handle_record(&unified().encode_to_vec());
    assert_eq!(r.handle_record(&record), Some(want));
}

#[test]
fn excess_entries_are_truncated_and_counted() {
    let limits = RoomLimits {
        max_emotes: 4,
        max_modifiers: 1,
        max_tags: 2,
        max_removed_ids: 3,
        ..RoomLimits::default()
    };
    let b = AssetBundle {
        removed_ids: (0..10).collect(),
        emotes: (0..10).map(emote).collect(),
        modifiers: (0..3)
            .map(|id| Modifier {
                id,
                ..Default::default()
            })
            .collect(),
        tags: (0..5)
            .map(|id| Tag {
                id,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    let mut r = AssetReader::new(limits);
    let got = r.handle_record(&b.encode_to_vec()).unwrap();
    assert_eq!(got.removed_ids, vec![0, 1, 2]);
    assert_eq!(got.emotes, b.emotes[..4]);
    assert_eq!(got.modifiers, b.modifiers[..1]);
    assert_eq!(got.tags, b.tags[..2]);
    let m = r.metrics();
    assert_eq!(
        (
            m.removed_ids_dropped,
            m.emotes_dropped,
            m.modifiers_dropped,
            m.tags_dropped
        ),
        (7, 6, 2, 3)
    );
}

#[test]
fn entries_past_the_node_bound_are_left_out() {
    // An emote of many empty effects decodes to far more memory than its
    // encoding takes.
    let heavy = Emote {
        effects: vec![EmoteEffect::default(); 300_000],
        ..emote(1)
    };
    let light = Emote {
        effects: vec![
            EmoteEffect {
                effect: Some(Effect::SpriteAnimation(SpriteAnimation::default())),
            };
            3
        ],
        ..emote(2)
    };
    let b = AssetBundle {
        emotes: vec![heavy, light.clone()],
        ..Default::default()
    };
    let record = b.encode_to_vec();
    assert!(record.len() < 1 << 20);

    let mut r = AssetReader::new(RoomLimits::default());
    let got = r.handle_record(&record).unwrap();
    assert_eq!(got.emotes, vec![light.clone()]);
    assert_eq!(r.metrics().emotes_dropped, 1);

    // An emote takes a node for itself and one for its name, and a sprite
    // effect two: 8 for `light` and 2 for a plain emote.
    let tight = RoomLimits {
        max_bundle_nodes: 8 + 2,
        ..RoomLimits::default()
    };
    let b = AssetBundle {
        emotes: vec![light.clone(), light.clone(), emote(3), emote(4)],
        ..Default::default()
    };
    let mut r = AssetReader::new(tight);
    let got = r.handle_record(&b.encode_to_vec()).unwrap();
    assert_eq!(got.emotes, vec![light, emote(3)]);
    assert_eq!(r.metrics().emotes_dropped, 2);
}
