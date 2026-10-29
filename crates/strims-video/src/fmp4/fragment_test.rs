// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;

const VIDEO: [Sample; 2] = [
    Sample {
        duration: 3000,
        size: 5,
        keyframe: true,
        composition_offset: 3000,
    },
    Sample {
        duration: 3000,
        size: 3,
        keyframe: false,
        composition_offset: -3000,
    },
];
const AUDIO: [Sample; 1] = [Sample {
    duration: 1024,
    size: 2,
    keyframe: false,
    composition_offset: 0,
}];

fn runs() -> [TrackRun<'static>; 2] {
    [
        TrackRun {
            track: TrackId(1),
            kind: TrackKind::Video,
            base_decode_time: 9000,
            samples: &VIDEO,
            data: &[1, 2, 3, 4, 5, 6, 7, 8],
        },
        TrackRun {
            track: TrackId(2),
            kind: TrackKind::Audio,
            base_decode_time: 1024,
            samples: &AUDIO,
            data: &[9, 10],
        },
    ]
}

#[test]
fn fragment_layout() {
    let mut out = Vec::new();
    write_fragment(7, &runs(), &mut out);
    #[rustfmt::skip]
    let expected: &[&[u8]] = &[
        // moof, 192
        &[0, 0, 0, 0xc0], b"moof",
        //   mfhd, 16: version 0; sequence_number 7
        &[0, 0, 0, 0x10], b"mfhd", &[0; 4], &[0, 0, 0, 7],
        //   traf, 96
        &[0, 0, 0, 0x60], b"traf",
        //     tfhd, 16: version 0, flags default-base-is-moof; track_ID 1
        &[0, 0, 0, 0x10], b"tfhd", &[0, 0x02, 0, 0], &[0, 0, 0, 1],
        //     tfdt, 20: version 1; baseMediaDecodeTime 9000
        &[0, 0, 0, 0x14], b"tfdt", &[1, 0, 0, 0], &[0, 0, 0, 0, 0, 0, 0x23, 0x28],
        //     trun, 52: version 1, flags data-offset | duration | size | flags |
        //     composition offset; sample_count 2; data_offset 200 (moof + mdat header)
        &[0, 0, 0, 0x34], b"trun", &[1, 0, 0x0f, 0x01], &[0, 0, 0, 2], &[0, 0, 0, 0xc8],
        //       duration 3000, size 5, sync (depends on none), offset 3000
        &[0, 0, 0x0b, 0xb8], &[0, 0, 0, 5], &[0x02, 0, 0, 0], &[0, 0, 0x0b, 0xb8],
        //       duration 3000, size 3, non-sync (depends on others), offset -3000
        &[0, 0, 0x0b, 0xb8], &[0, 0, 0, 3], &[0x01, 0x01, 0, 0], &[0xff, 0xff, 0xf4, 0x48],
        //   traf, 72
        &[0, 0, 0, 0x48], b"traf",
        //     tfhd, 16: track_ID 2
        &[0, 0, 0, 0x10], b"tfhd", &[0, 0x02, 0, 0], &[0, 0, 0, 2],
        //     tfdt, 20: baseMediaDecodeTime 1024
        &[0, 0, 0, 0x14], b"tfdt", &[1, 0, 0, 0], &[0, 0, 0, 0, 0, 0, 0x04, 0],
        //     trun, 28: version 1, flags data-offset | duration | size; sample_count 1;
        //     data_offset 208, after the video samples
        &[0, 0, 0, 0x1c], b"trun", &[1, 0, 0x03, 0x01], &[0, 0, 0, 1], &[0, 0, 0, 0xd0],
        //       duration 1024, size 2
        &[0, 0, 0x04, 0], &[0, 0, 0, 2],
        // mdat, 18: video samples, then audio
        &[0, 0, 0, 0x12], b"mdat", &[1, 2, 3, 4, 5, 6, 7, 8], &[9, 10],
    ];
    assert_eq!(out, expected.concat());
    assert_eq!(fragment_len(&runs()), out.len());
    assert_eq!(fragment_head_len(&runs()), out.len() - 10);
}

#[test]
fn runs_without_samples_get_no_traf() {
    let [video, audio] = runs();
    let empty = TrackRun {
        samples: &[],
        data: &[],
        ..audio
    };
    let mut with_empty = Vec::new();
    write_fragment(1, &[video, empty], &mut with_empty);
    let mut alone = Vec::new();
    write_fragment(1, &[video], &mut alone);
    assert_eq!(with_empty, alone);
    assert_eq!(fragment_len(&[video, empty]), alone.len());
    assert_eq!(fragment_len(&[]), EMPTY_FRAGMENT_LEN);
}

#[test]
fn head_and_data_make_the_fragment() {
    let mut head = Vec::new();
    write_fragment_head(3, &runs(), &mut head);
    for r in runs() {
        head.extend_from_slice(r.data);
    }
    let mut whole = Vec::new();
    write_fragment(3, &runs(), &mut whole);
    assert_eq!(head, whole);
}

#[test]
fn prft_layout() {
    let mut out = Vec::new();
    write_prft(
        &ProducerTime {
            track: TrackId(2),
            media_time: 0x1_0000_0003,
            // 2026-10-29T12:00:00.250Z.
            published: Time::from_unix_millis(1_793_275_200_250),
        },
        &mut out,
    );
    #[rustfmt::skip]
    let expected: &[&[u8]] = &[
        // prft, 32: version 1, flags 1; reference_track_ID 2
        &[0, 0, 0, 0x20], b"prft", &[1, 0, 0, 1], &[0, 0, 0, 2],
        //   ntp_timestamp: 4_002_264_000 s since 1900, a quarter second
        &4_002_264_000u32.to_be_bytes(), &[0x40, 0, 0, 0],
        //   media_time
        &[0, 0, 0, 1, 0, 0, 0, 3],
    ];
    assert_eq!(out, expected.concat());
    assert_eq!(out.len(), PRFT_LEN);
}

#[test]
fn ntp_timestamps_count_from_1900_and_wrap_with_their_era() {
    assert_eq!(ntp_timestamp(Time::EPOCH), 2_208_988_800 << 32);
    // A millisecond is 2^32 / 1000 of a second, rounded down.
    assert_eq!(
        ntp_timestamp(Time::from_unix_millis(1)),
        (2_208_988_800 << 32) | 4_294_967
    );
    // 2036-02-07T06:28:16Z starts NTP era 1.
    let era1 = Time::from_unix(4_294_967_296 - 2_208_988_800, 0);
    assert_eq!(ntp_timestamp(era1), 0);
    assert_eq!(ntp_timestamp(Time::from_unix(-2_208_988_801, 0)), 0);
    assert_eq!(ntp_timestamp(Time::NIL), 0);
}
