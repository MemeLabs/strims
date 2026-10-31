// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use strims_base::time::{Duration, Time};

use super::*;
use crate::flv::VIDEO_TRACK;
use crate::media::{CodecParams, TrackId};

fn video(width: u16, height: u16) -> DecoderConfig {
    DecoderConfig {
        track: VIDEO_TRACK,
        kind: TrackKind::Video,
        codec: "avc1.64001f".into(),
        timescale: 90_000,
        params: CodecParams::Avc {
            avcc: Vec::new(),
            width,
            height,
        },
    }
}

fn unit(track: TrackId, keyframe: bool) -> AccessUnit<'static> {
    AccessUnit {
        track,
        dts: 0,
        pts: 0,
        keyframe,
        payload: b"",
        published: Time::default(),
    }
}

fn at(secs: i64) -> Time {
    Time::from_unix_millis(1_700_000_000_000) + Duration::from_secs(secs)
}

fn thumbnails(width: u16, height: u16) -> Thumbnails {
    let mut t = Thumbnails::new(&ThumbnailSettings::default());
    t.config(&video(width, height));
    t
}

#[test]
fn the_first_is_of_a_keyframe_past_the_wait_from_the_first_video_unit() {
    let mut t = thumbnails(1920, 1080);
    let key = unit(VIDEO_TRACK, true);
    let audio = unit(TrackId(2), true);
    assert_eq!(t.take(&audio, at(0)), None, "audio starts no wait");
    assert_eq!(t.take(&unit(VIDEO_TRACK, false), at(1)), None);
    assert_eq!(t.take(&key, at(5)), None, "4 s after the first video unit");
    assert_eq!(t.take(&unit(VIDEO_TRACK, false), at(7)), None);
    let (config, width) = t.take(&key, at(7)).unwrap();
    assert_eq!((config, width), (video(1920, 1080), 640));
    assert_eq!(t.take(&key, at(8)), None, "one at a time");
}

#[test]
fn the_next_comes_an_interval_after_one_made_and_a_retry_after_a_failure() {
    let mut t = thumbnails(1920, 1080);
    let key = unit(VIDEO_TRACK, true);
    t.take(&key, at(0));
    assert!(t.take(&key, at(5)).is_some());
    t.attempt.set(Attempt::Failed);
    assert_eq!(t.take(&key, at(34)), None);
    assert_eq!(t.take(&key, at(35)).map(|(_, w)| w), Some(320));
    t.attempt.set(Attempt::Failed);
    assert_eq!(t.take(&key, at(65)).map(|(_, w)| w), Some(160));
    t.attempt.set(Attempt::Failed);
    assert_eq!(t.take(&key, at(95)).map(|(_, w)| w), Some(160), "the floor");
    t.attempt.set(Attempt::Made);
    assert_eq!(t.take(&key, at(394)), None);
    assert_eq!(t.take(&key, at(395)).map(|(_, w)| w), Some(640));
}

#[test]
fn a_thumbnail_fits_the_box_at_the_pictures_aspect_and_size() {
    let width = |w, h| {
        let mut t = thumbnails(w, h);
        t.take(&unit(VIDEO_TRACK, true), at(0));
        t.take(&unit(VIDEO_TRACK, true), at(5)).unwrap().1
    };
    assert_eq!(width(1920, 1080), 640);
    assert_eq!(width(1080, 1920), 202, "portrait, 360 high");
    assert_eq!(width(2560, 1080), 640, "wider than the box");
    assert_eq!(width(480, 360), 480);
    assert_eq!(width(320, 180), 320, "no wider than the picture");
    assert_eq!(width(99, 64), 98, "even");
}

#[test]
fn a_jpegs_size_is_its_frame_headers() {
    let jpeg = [
        0xff, 0xd8, // SOI
        0xff, 0xe0, 0x00, 0x04, 0x4a, 0x46, // APP0, 2 bytes of data
        0xff, 0xff, 0xdb, 0x00, 0x03, 0x00, // a fill byte, DQT
        0xff, 0xc0, 0x00, 0x0b, 0x08, 0x01, 0x68, 0x02, 0x80, 0x01, 0x01, 0x11, 0x00, // SOF0
        0xff, 0xd9,
    ];
    assert_eq!(jpeg_size(&jpeg), Some((640, 360)));
    assert_eq!(jpeg_size(&jpeg[..20]), None, "truncated");
    assert_eq!(jpeg_size(b"\xff\xd8fake\xff\xd9"), None);
    assert_eq!(
        jpeg_size(b"\xff\xd8\xff\xda\x00\x02"),
        None,
        "no frame header"
    );
    assert_eq!(jpeg_size(&[]), None);
}
