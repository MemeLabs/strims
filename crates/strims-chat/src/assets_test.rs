// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Tests panic on failure by design.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use strims_proto::chat::v1::{EmoteImage, Room};
use strims_proto::r#type::Image;

use super::*;

fn emote(id: u64, name: &str) -> Emote {
    Emote {
        id,
        server_id: 1,
        name: name.into(),
        enable: true,
        ..Default::default()
    }
}

fn emote_with_image(id: u64, name: &str, image: Vec<u8>) -> Emote {
    Emote {
        images: vec![EmoteImage {
            data: image,
            ..Default::default()
        }],
        ..emote(id, name)
    }
}

fn modifier(id: u64, name: &str) -> Modifier {
    Modifier {
        id,
        server_id: 1,
        name: name.into(),
        ..Default::default()
    }
}

fn tag(id: u64, name: &str) -> Tag {
    Tag {
        id,
        server_id: 1,
        name: name.into(),
        ..Default::default()
    }
}

fn room(name: &str) -> Room {
    Room {
        name: name.into(),
        css: String::new(),
    }
}

fn image(data: &[u8]) -> Image {
    Image {
        data: data.to_vec(),
        ..Default::default()
    }
}

fn assets() -> Assets {
    Assets {
        server: Server {
            id: 1,
            room: Some(room("lobby")),
            ..Default::default()
        },
        icon: Some(ServerIcon {
            id: 1,
            image: Some(image(b"icon")),
        }),
        emotes: vec![
            emote(10, "PEPE"),
            emote(11, "Kappa"),
            Emote {
                enable: false,
                ..emote(12, "LUL")
            },
        ],
        modifiers: vec![modifier(20, "spin")],
        tags: vec![tag(30, "nsfw")],
    }
}

fn delta() -> AssetBundle {
    AssetBundle {
        is_delta: true,
        ..Default::default()
    }
}

fn framed(b: &AssetBundle) -> usize {
    RecordEncoder::new(ASSET_FRAME_BYTES)
        .unwrap()
        .encoded_len(b.encoded_len())
        .unwrap()
}

fn write(p: &mut AssetPublisher, a: &Assets) -> AssetWrite {
    p.sync(a, false).unwrap().unwrap()
}

#[test]
fn first_sync_is_unified_without_a_reset() {
    let a = assets();
    let mut p = AssetPublisher::new();
    let w = write(&mut p, &a);
    assert!(!w.reset);
    assert_eq!(
        w.bundle,
        AssetBundle {
            is_delta: false,
            removed_ids: vec![],
            room: Some(room("lobby")),
            emotes: vec![a.emotes[0].clone(), a.emotes[1].clone()],
            modifiers: a.modifiers.clone(),
            tags: a.tags.clone(),
            icon: Some(image(b"icon")),
        }
    );
    assert_eq!(p.stream_bytes(), framed(&w.bundle));
    assert_eq!(p.stream_bytes(), ASSET_FRAME_BYTES);

    let mut q = AssetPublisher::new();
    let w = q.sync(&a, true).unwrap().unwrap();
    assert!(!w.reset && !w.bundle.is_delta);
}

#[test]
fn unchanged_assets_write_nothing() {
    let a = assets();
    let mut p = AssetPublisher::new();
    write(&mut p, &a);
    assert_eq!(p.sync(&a, false), Ok(None));
    assert_eq!(p.stream_bytes(), ASSET_FRAME_BYTES);
}

#[test]
fn delta_holds_changed_entries_only() {
    let mut a = assets();
    let mut p = AssetPublisher::new();
    write(&mut p, &a);

    a.emotes[1].name = "Keepo".into();
    a.modifiers[0].priority = 3;
    let w = write(&mut p, &a);
    assert!(!w.reset);
    assert_eq!(
        w.bundle,
        AssetBundle {
            emotes: vec![a.emotes[1].clone()],
            modifiers: vec![a.modifiers[0].clone()],
            ..delta()
        }
    );
    assert_eq!(p.stream_bytes(), 2 * ASSET_FRAME_BYTES);

    a.tags.push(tag(31, "weeb"));
    let w = write(&mut p, &a);
    assert_eq!(
        w.bundle,
        AssetBundle {
            tags: vec![tag(31, "weeb")],
            ..delta()
        }
    );
}

#[test]
fn disabled_emote_becomes_a_removal() {
    let mut a = assets();
    let mut p = AssetPublisher::new();
    write(&mut p, &a);

    a.emotes[0].enable = false;
    let w = write(&mut p, &a);
    assert_eq!(
        w.bundle,
        AssetBundle {
            removed_ids: vec![10],
            ..delta()
        }
    );
    assert_eq!(p.sync(&a, false), Ok(None));

    a.emotes[0].enable = true;
    let w = write(&mut p, &a);
    assert_eq!(
        w.bundle,
        AssetBundle {
            emotes: vec![a.emotes[0].clone()],
            ..delta()
        }
    );

    a.emotes[2].enable = true;
    let w = write(&mut p, &a);
    assert_eq!(w.bundle.emotes, vec![a.emotes[2].clone()]);
}

#[test]
fn deleted_entries_become_removals() {
    let mut a = assets();
    let mut p = AssetPublisher::new();
    write(&mut p, &a);

    a.emotes.remove(1);
    a.modifiers.clear();
    a.tags.clear();
    let w = write(&mut p, &a);
    assert_eq!(
        w.bundle,
        AssetBundle {
            removed_ids: vec![11, 20, 30],
            ..delta()
        }
    );
    assert_eq!(p.sync(&a, false), Ok(None));
}

#[test]
fn room_follows_the_server_record() {
    let mut a = assets();
    let mut p = AssetPublisher::new();
    write(&mut p, &a);

    a.server.admin_peer_keys.push(vec![7; 32]);
    let w = write(&mut p, &a);
    assert_eq!(
        w.bundle,
        AssetBundle {
            room: Some(room("lobby")),
            ..delta()
        }
    );

    a.server.room = Some(room("hall"));
    let w = write(&mut p, &a);
    assert_eq!(w.bundle.room, Some(room("hall")));
}

#[test]
fn icon_changes_and_removal() {
    let mut a = assets();
    let mut p = AssetPublisher::new();
    write(&mut p, &a);

    a.icon.as_mut().unwrap().image = Some(image(b"new icon"));
    let w = write(&mut p, &a);
    assert_eq!(
        w.bundle,
        AssetBundle {
            icon: Some(image(b"new icon")),
            ..delta()
        }
    );

    a.icon = None;
    let w = write(&mut p, &a);
    assert_eq!(
        w.bundle,
        AssetBundle {
            removed_ids: vec![1],
            ..delta()
        }
    );
    assert_eq!(p.sync(&a, false), Ok(None));
}

#[test]
fn explicit_unified_sync_starts_the_stream_over() {
    let a = assets();
    let mut p = AssetPublisher::new();
    let first = write(&mut p, &a);
    let w = p.sync(&a, true).unwrap().unwrap();
    assert!(w.reset);
    assert_eq!(w.bundle, first.bundle);
    assert_eq!(p.stream_bytes(), ASSET_FRAME_BYTES);
}

#[test]
fn reset_forces_a_unified_bundle() {
    let mut a = assets();
    let mut p = AssetPublisher::new();
    let first = write(&mut p, &a);
    a.emotes[1].name = "Keepo".into();
    write(&mut p, &a);
    a.emotes[1].name = "Kappa".into();

    p.reset();
    let w = p.sync(&a, false).unwrap().unwrap();
    assert!(w.reset);
    assert_eq!(w.bundle, first.bundle);
    assert_eq!(p.stream_bytes(), ASSET_FRAME_BYTES);
    assert_eq!(p.sync(&a, false), Ok(None));
}

#[test]
fn delta_past_the_window_starts_over_unified() {
    const MIB: usize = 1 << 20;
    let mut a = assets();
    a.emotes[0] = emote_with_image(10, "PEPE", vec![0; 8 * MIB]);
    let mut p = AssetPublisher::new();
    let w = write(&mut p, &a);
    let record = framed(&w.bundle);
    assert!(
        record > 8 * MIB && 4 * record > ASSET_WINDOW_BYTES && 3 * record <= ASSET_WINDOW_BYTES
    );

    for i in 1..3u8 {
        a.emotes[0].images[0].data[0] = i;
        let w = write(&mut p, &a);
        assert!(!w.reset && w.bundle.is_delta);
        assert_eq!(w.bundle.emotes.len(), 1);
    }
    assert_eq!(p.stream_bytes(), 3 * record);

    a.emotes[0].images[0].data[0] = 3;
    let w = write(&mut p, &a);
    assert!(w.reset);
    assert!(!w.bundle.is_delta);
    assert_eq!(w.bundle.emotes.len(), 2);
    assert_eq!(w.bundle.room, Some(room("lobby")));
    assert_eq!(p.stream_bytes(), framed(&w.bundle));
}

#[test]
fn oversized_unified_bundle_is_too_large() {
    let mut a = assets();
    let mut p = AssetPublisher::new();
    write(&mut p, &a);

    a.emotes[0] = emote_with_image(10, "PEPE", vec![0; ASSET_WINDOW_BYTES]);
    let err = p.sync(&a, false).unwrap_err();
    let AssetError::TooLarge { framed_len } = err;
    assert!(framed_len > ASSET_WINDOW_BYTES);
    assert_eq!(p.stream_bytes(), ASSET_FRAME_BYTES);
    assert!(p.sync(&a, false).is_err());

    a.emotes[0] = emote(10, "PEPE");
    let w = write(&mut p, &a);
    assert!(w.reset);
    assert!(!w.bundle.is_delta);
    assert_eq!(w.bundle.emotes.len(), 2);
}

fn limits() -> RoomLimits {
    RoomLimits::default()
}

#[test]
fn fold_applies_removals_then_upserts() {
    let mut acc = AssetBundle {
        room: Some(room("lobby")),
        emotes: vec![emote(10, "PEPE"), emote(11, "Kappa")],
        modifiers: vec![modifier(20, "spin")],
        tags: vec![tag(30, "nsfw")],
        ..Default::default()
    };
    let d = AssetBundle {
        removed_ids: vec![10, 20, 99],
        room: Some(room("hall")),
        icon: Some(image(b"icon")),
        emotes: vec![emote(11, "Keepo"), emote(12, "LUL"), emote(10, "PEPE2")],
        ..delta()
    };
    assert_eq!(fold(&mut acc, d, &limits()), 0);
    assert_eq!(
        acc,
        AssetBundle {
            room: Some(room("hall")),
            icon: Some(image(b"icon")),
            emotes: vec![emote(11, "Keepo"), emote(12, "LUL"), emote(10, "PEPE2")],
            tags: vec![tag(30, "nsfw")],
            ..Default::default()
        }
    );
}

#[test]
fn fold_of_a_unified_bundle_replaces() {
    let mut acc = AssetBundle {
        room: Some(room("lobby")),
        emotes: vec![emote(10, "PEPE")],
        ..Default::default()
    };
    let u = AssetBundle {
        tags: vec![tag(30, "nsfw")],
        ..Default::default()
    };
    assert_eq!(fold(&mut acc, u.clone(), &limits()), 0);
    assert_eq!(acc, u);
}

#[test]
fn fold_of_deltas_equals_applying_them_in_turn() {
    let base = AssetBundle {
        emotes: vec![emote(5, "A"), emote(10, "B"), emote(11, "C")],
        ..Default::default()
    };
    let d1 = AssetBundle {
        removed_ids: vec![5],
        emotes: vec![emote(10, "B1"), emote(12, "D")],
        ..delta()
    };
    let d2 = AssetBundle {
        removed_ids: vec![12, 11, 5],
        emotes: vec![emote(5, "A2")],
        ..delta()
    };

    let mut seq = base.clone();
    fold(&mut seq, d1.clone(), &limits());
    fold(&mut seq, d2.clone(), &limits());

    let mut combined = d1;
    fold(&mut combined, d2, &limits());
    assert_eq!(combined.removed_ids, vec![5, 12, 11]);
    let mut once = base;
    fold(&mut once, combined, &limits());
    assert_eq!(once, seq);
}

#[test]
fn fold_keeps_within_its_bounds() {
    let small = RoomLimits {
        max_emotes: 2,
        max_removed_ids: 1,
        ..limits()
    };
    let mut acc = AssetBundle {
        emotes: vec![emote(10, "A")],
        ..Default::default()
    };
    let d = AssetBundle {
        emotes: vec![emote(11, "B"), emote(12, "C"), emote(10, "A2")],
        ..delta()
    };
    assert_eq!(fold(&mut acc, d, &small), 1);
    assert_eq!(acc.emotes, vec![emote(10, "A2"), emote(11, "B")]);

    let mut acc = delta();
    let d = AssetBundle {
        removed_ids: vec![1, 2, 1],
        ..delta()
    };
    assert_eq!(fold(&mut acc, d, &small), 1);
    assert_eq!(acc.removed_ids, vec![1]);

    let big = emote_with_image(13, "BIG", vec![0; 200]);
    let tight = RoomLimits {
        max_asset_record: field_len(&emote(10, "A")) + 50,
        ..limits()
    };
    let mut acc = AssetBundle {
        emotes: vec![emote(10, "A")],
        ..Default::default()
    };
    let d = AssetBundle {
        room: Some(room("lobby")),
        emotes: vec![big.clone()],
        ..delta()
    };
    assert_eq!(fold(&mut acc, d, &tight), 1);
    assert_eq!(acc.room, Some(room("lobby")));
    assert_eq!(acc.emotes, vec![emote(10, "A")]);
    let u = AssetBundle {
        emotes: vec![big],
        ..Default::default()
    };
    assert_eq!(fold(&mut acc, u, &tight), 1);
    assert_eq!(acc, AssetBundle::default());
}

fn sample(is_delta: bool, removed: usize) -> AssetBundle {
    AssetBundle {
        is_delta,
        removed_ids: (0..removed as u64).map(|i| 1_000_000 + i * 7919).collect(),
        room: Some(room("lobby")),
        icon: Some(image(&[9; 300])),
        emotes: (0..50)
            .map(|i| emote_with_image(100 + i, &format!("E{i}"), vec![i as u8; 10 * i as usize]))
            .collect(),
        modifiers: (0..20)
            .map(|i| modifier(200 + i, &format!("m{i}")))
            .collect(),
        tags: (0..20).map(|i| tag(300 + i, &format!("t{i}"))).collect(),
    }
}

fn rejoin(parts: &[AssetBundle]) -> AssetBundle {
    let mut acc = parts[0].clone();
    for p in &parts[1..] {
        assert_eq!(fold(&mut acc, p.clone(), &limits()), 0);
    }
    acc
}

#[test]
fn split_parts_fold_back_to_the_bundle() {
    for (bundle, max) in [
        (sample(false, 0), 1000),
        (sample(true, 0), 1000),
        (sample(true, 400), 1000),
        (sample(false, 0), 600),
        (
            AssetBundle {
                removed_ids: sample(true, 400).removed_ids,
                tags: (0..5).map(|i| tag(300 + i, &format!("t{i}"))).collect(),
                ..delta()
            },
            64,
        ),
    ] {
        let s = split(bundle.clone(), max);
        assert_eq!(s.dropped, 0);
        assert!(s.parts.len() > 1);
        assert_eq!(s.parts[0].is_delta, bundle.is_delta);
        for p in &s.parts {
            assert!(p.encoded_len() <= max, "{} > {max}", p.encoded_len());
        }
        for p in &s.parts[1..] {
            assert!(p.is_delta);
        }
        assert_eq!(rejoin(&s.parts), bundle);
    }
}

#[test]
fn split_of_a_bundle_that_fits_is_the_bundle() {
    let b = sample(true, 3);
    let s = split(b.clone(), b.encoded_len());
    assert_eq!(
        s,
        Split {
            parts: vec![b],
            dropped: 0
        }
    );

    let empty = AssetBundle::default();
    let s = split(empty.clone(), 0);
    assert_eq!(
        s,
        Split {
            parts: vec![empty],
            dropped: 0
        }
    );
}

#[test]
fn split_drops_items_that_fit_no_part() {
    let mut b = sample(false, 0);
    let big = emote_with_image(999, "BIG", vec![1; 2000]);
    b.emotes.insert(10, big);
    let s = split(b.clone(), 1000);
    assert_eq!(s.dropped, 1);
    for p in &s.parts {
        assert!(p.encoded_len() <= 1000);
    }
    b.emotes.remove(10);
    assert_eq!(rejoin(&s.parts), b);

    let s = split(sample(true, 2), 5);
    assert_eq!(s.parts, vec![delta()]);
    assert_eq!(s.dropped, 2 + 1 + 1 + 50 + 20 + 20);
}
