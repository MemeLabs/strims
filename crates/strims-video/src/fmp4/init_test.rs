// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;
use crate::fmp4::parse_init;
use crate::media::TrackId;

const MATRIX: [u8; 36] = [
    0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, //
    0, 0, 0, 0, 0, 0, 0, 0, 0x40, 0, 0, 0,
];

fn video(track: u32, avcc: &[u8], width: u16, height: u16) -> DecoderConfig {
    DecoderConfig {
        track: TrackId(track),
        kind: TrackKind::Video,
        codec: "avc1.64001f".into(),
        timescale: 90_000,
        params: CodecParams::Avc {
            avcc: avcc.to_vec(),
            width,
            height,
        },
    }
}

fn audio(track: u32, sample_rate: u32, channels: u8) -> DecoderConfig {
    DecoderConfig {
        track: TrackId(track),
        kind: TrackKind::Audio,
        codec: "mp4a.40.2".into(),
        timescale: sample_rate,
        params: CodecParams::Aac {
            asc: vec![0x11, 0x90],
            sample_rate,
            channels,
            frame_samples: 1024,
        },
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
        .unwrap_or_else(|| panic!("{} not found", needle.escape_ascii()))
}

/// The box of type `fourcc` that `data` holds somewhere, header included.
fn find_box<'a>(data: &'a [u8], fourcc: &[u8; 4]) -> &'a [u8] {
    let at = find(data, fourcc) - 4;
    let size = u32::from_be_bytes(data[at..at + 4].try_into().unwrap()) as usize;
    &data[at..at + size]
}

#[test]
fn video_init_layout() {
    let mut out = Vec::new();
    write_init(&[video(1, &[1, 0x64, 0, 0x1f], 320, 240)], &mut out).unwrap();
    #[rustfmt::skip]
    let expected: &[&[u8]] = &[
        // ftyp, 28 bytes: major brand, minor version 512, compatible brands
        &[0, 0, 0, 0x1c], b"ftyp", b"iso5", &[0, 0, 2, 0], b"iso5", b"iso6", b"mp41",
        // moov, 595
        &[0, 0, 2, 0x53], b"moov",
        //   mvhd, 108: version 0, flags 0; creation, modification; timescale 1000;
        //   duration 0; rate 1.0; volume 1.0; reserved
        &[0, 0, 0, 0x6c], b"mvhd", &[0; 4], &[0; 8], &[0, 0, 0x03, 0xe8], &[0; 4],
        &[0, 1, 0, 0], &[1, 0], &[0; 10],
        //     matrix; pre_defined; next_track_ID 2
        &MATRIX, &[0; 24], &[0, 0, 0, 2],
        //   trak, 439
        &[0, 0, 1, 0xb7], b"trak",
        //     tkhd, 92: version 0, flags enabled | in movie; creation, modification;
        //     track_ID 1; reserved, duration, reserved[2], layer, alternate_group;
        //     volume 0; reserved; matrix; width 320.0, height 240.0
        &[0, 0, 0, 0x5c], b"tkhd", &[0, 0, 0, 3], &[0; 8], &[0, 0, 0, 1], &[0; 20],
        &[0, 0], &[0, 0], &MATRIX, &[0x01, 0x40, 0, 0], &[0, 0xf0, 0, 0],
        //     mdia, 339
        &[0, 0, 1, 0x53], b"mdia",
        //       mdhd, 32: version 0; creation, modification; timescale 90000;
        //       duration 0; language "und"; pre_defined
        &[0, 0, 0, 0x20], b"mdhd", &[0; 4], &[0; 8], &[0, 1, 0x5f, 0x90], &[0; 4],
        &[0x55, 0xc4], &[0, 0],
        //       hdlr, 45: version 0; pre_defined; handler_type; reserved[3]; name
        &[0, 0, 0, 0x2d], b"hdlr", &[0; 4], &[0; 4], b"vide", &[0; 12], b"VideoHandler\0",
        //       minf, 254
        &[0, 0, 0, 0xfe], b"minf",
        //         vmhd, 20: version 0, flags 1; graphicsmode, opcolor
        &[0, 0, 0, 0x14], b"vmhd", &[0, 0, 0, 1], &[0; 8],
        //         dinf, 36
        &[0, 0, 0, 0x24], b"dinf",
        //           dref, 28: version 0; entry_count 1
        &[0, 0, 0, 0x1c], b"dref", &[0; 4], &[0, 0, 0, 1],
        //             url, 12: version 0, flags self-contained
        &[0, 0, 0, 0x0c], b"url ", &[0, 0, 0, 1],
        //         stbl, 190
        &[0, 0, 0, 0xbe], b"stbl",
        //           stsd, 114: version 0; entry_count 1
        &[0, 0, 0, 0x72], b"stsd", &[0; 4], &[0, 0, 0, 1],
        //             avc1, 98: reserved[6]; data_reference_index 1;
        //             pre_defined, reserved, pre_defined[3]; width 320, height 240;
        //             72 dpi; reserved; frame_count 1; compressorname;
        //             depth 0x18; pre_defined -1
        &[0, 0, 0, 0x62], b"avc1", &[0; 6], &[0, 1], &[0; 16], &[0x01, 0x40, 0, 0xf0],
        &[0, 0x48, 0, 0], &[0, 0x48, 0, 0], &[0; 4], &[0, 1], &[0; 32], &[0, 0x18],
        &[0xff, 0xff],
        //               avcC, 12: the record
        &[0, 0, 0, 0x0c], b"avcC", &[1, 0x64, 0, 0x1f],
        //           stts, stsc, stco: version 0, no entries; stsz: sample_size 0, none
        &[0, 0, 0, 0x10], b"stts", &[0; 8],
        &[0, 0, 0, 0x10], b"stsc", &[0; 8],
        &[0, 0, 0, 0x14], b"stsz", &[0; 12],
        &[0, 0, 0, 0x10], b"stco", &[0; 8],
        //   mvex, 40
        &[0, 0, 0, 0x28], b"mvex",
        //     trex, 32: version 0; track_ID 1; default_sample_description_index 1;
        //     default duration, size, flags 0
        &[0, 0, 0, 0x20], b"trex", &[0; 4], &[0, 0, 0, 1], &[0, 0, 0, 1], &[0; 12],
    ];
    assert_eq!(out, expected.concat());
}

#[test]
fn audio_sample_entry_layout() {
    let mut out = Vec::new();
    write_init(&[audio(2, 48_000, 2)], &mut out).unwrap();
    #[rustfmt::skip]
    let expected: &[&[u8]] = &[
        // mp4a, 75: reserved[6]; data_reference_index 1; reserved[2];
        // channelcount 2; samplesize 16; pre_defined, reserved; samplerate 48000.0
        &[0, 0, 0, 0x4b], b"mp4a", &[0; 6], &[0, 1], &[0; 8], &[0, 2], &[0, 0x10], &[0; 4],
        &[0xbb, 0x80, 0, 0],
        //   esds, 39: version 0; ES_Descriptor; DecoderConfigDescriptor (MPEG-4
        //   audio, stream type audio); DecoderSpecificInfo: the ASC;
        //   SLConfigDescriptor predefined 2
        &[0, 0, 0, 0x27], b"esds", &[0; 4], &[0x03, 0x19, 0, 0, 0], &[0x04, 0x11, 0x40, 0x15],
        &[0; 11], &[0x05, 0x02, 0x11, 0x90], &[0x06, 0x01, 0x02],
    ];
    assert_eq!(find_box(&out, b"mp4a"), expected.concat());
    // tkhd volume 1.0, no picture size.
    let tkhd = find_box(&out, b"tkhd");
    assert_eq!(tkhd[44..46], [1, 0]);
    assert_eq!(tkhd[84..92], [0; 8]);
    assert_eq!(
        find_box(&out, b"smhd"),
        [&[0, 0, 0, 0x10][..], b"smhd", &[0; 8]].concat()
    );
    let hdlr = find_box(&out, b"hdlr");
    assert_eq!(&hdlr[16..20], b"soun");
    assert_eq!(&hdlr[32..], b"SoundHandler\0");
    assert_eq!(&find_box(&out, b"mdhd")[20..24], 48_000u32.to_be_bytes());
}

#[test]
fn audio_rate_above_16_bits_is_halved_and_mdhd_keeps_it() {
    let mut out = Vec::new();
    write_init(&[audio(2, 96_000, 2)], &mut out).unwrap();
    let mp4a = find_box(&out, b"mp4a");
    assert_eq!(mp4a[32..36], [0xbb, 0x80, 0, 0]);
    assert_eq!(&find_box(&out, b"mdhd")[20..24], 96_000u32.to_be_bytes());
}

#[test]
fn audio_without_channel_count_declares_stereo() {
    let mut out = Vec::new();
    write_init(&[audio(2, 44_100, 0)], &mut out).unwrap();
    assert_eq!(find_box(&out, b"mp4a")[24..26], [0, 2]);
}

#[test]
fn tracks_follow_config_order_and_next_track_id_follows_the_highest() {
    let mut out = Vec::new();
    let configs = [video(3, &[1, 2, 3], 16, 16), audio(7, 48_000, 2)];
    write_init(&configs, &mut out).unwrap();
    let info = parse_init(&out).unwrap();
    let ids: Vec<u32> = info.tracks().iter().map(|t| t.id).collect();
    assert_eq!(ids, [3, 7]);
    assert_eq!(find_box(&out, b"mvhd")[104..108], 8u32.to_be_bytes());
    let mvex = find_box(&out, b"mvex");
    assert_eq!(mvex[20..24], 3u32.to_be_bytes());
    assert_eq!(mvex[52..56], 7u32.to_be_bytes());
}

#[test]
fn invalid_config_sets_are_refused_and_leave_the_buffer() {
    let mut out = vec![0xaa];
    let v = video(1, &[1], 16, 16);
    let cases: Vec<(Vec<DecoderConfig>, InitError)> = vec![
        (
            (1..=9).map(|i| audio(i, 48_000, 2)).collect(),
            InitError::TooManyTracks,
        ),
        (vec![video(0, &[1], 16, 16)], InitError::TrackIdZero),
        (
            vec![v.clone(), audio(1, 48_000, 2)],
            InitError::DuplicateTrack(1),
        ),
        (
            vec![DecoderConfig {
                timescale: 0,
                ..v.clone()
            }],
            InitError::ZeroTimescale(1),
        ),
        (
            vec![DecoderConfig {
                kind: TrackKind::Audio,
                ..v.clone()
            }],
            InitError::KindMismatch {
                track: 1,
                kind: TrackKind::Audio,
            },
        ),
    ];
    for (configs, error) in cases {
        assert_eq!(write_init(&configs, &mut out), Err(error));
        assert_eq!(out, [0xaa]);
    }
    let long = DecoderConfig {
        params: CodecParams::Aac {
            asc: vec![0; crate::MAX_CONFIG_LEN + 1],
            sample_rate: 48_000,
            channels: 2,
            frame_samples: 1024,
        },
        ..audio(2, 48_000, 2)
    };
    assert!(matches!(
        write_init(&[v, long], &mut out),
        Err(InitError::Aac { track: 2, .. })
    ));
    assert_eq!(out, [0xaa]);
}
