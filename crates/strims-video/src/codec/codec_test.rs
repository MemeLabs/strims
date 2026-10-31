// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic is a test failure.
#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn a_unit_owns_a_copy_of_its_access_unit() {
    let payload = [1, 2, 3];
    let u = AccessUnit {
        track: TrackId(2),
        dts: -5,
        pts: 7,
        keyframe: true,
        payload: &payload,
        published: Time::from_unix_millis(1_700_000_000_000),
    };
    let owned = Unit::of(&u);
    assert_eq!(owned.payload, payload);
    assert_eq!(owned.as_access_unit(), u);
}

#[test]
fn the_most_renditions_are_the_flv_sources_video_tracks() {
    assert_eq!(MAX_RENDITIONS, usize::from(crate::flv::MAX_VIDEO_TRACKS));
}
