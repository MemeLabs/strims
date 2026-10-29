// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::*;
use crate::fmp4::{ProducerTime, Sample, TrackRun, write_fragment, write_init, write_prft};
use crate::media::{CodecParams, DecoderConfig, TrackId};

fn bx(fourcc: &[u8; 4], content: &[u8]) -> Vec<u8> {
    let mut out = (content.len() as u32 + 8).to_be_bytes().to_vec();
    out.extend_from_slice(fourcc);
    out.extend_from_slice(content);
    out
}

fn full(fourcc: &[u8; 4], version: u8, flags: u32, content: &[u8]) -> Vec<u8> {
    let mut c = ((u32::from(version) << 24) | flags).to_be_bytes().to_vec();
    c.extend_from_slice(content);
    bx(fourcc, &c)
}

fn video_config() -> DecoderConfig {
    DecoderConfig {
        track: TrackId(1),
        kind: TrackKind::Video,
        codec: "avc1.64001f".into(),
        timescale: 90_000,
        params: CodecParams::Avc {
            avcc: vec![1, 0x64, 0, 0x1f],
            width: 16,
            height: 16,
        },
    }
}

fn audio_config(track: u32, rate: u32) -> DecoderConfig {
    DecoderConfig {
        track: TrackId(track),
        kind: TrackKind::Audio,
        codec: "mp4a.40.2".into(),
        timescale: rate,
        params: CodecParams::Aac {
            asc: vec![0x11, 0x90],
            sample_rate: rate,
            channels: 2,
            frame_samples: 1024,
        },
    }
}

fn init(configs: &[DecoderConfig]) -> Vec<u8> {
    let mut out = Vec::new();
    write_init(configs, Duration::ZERO, &mut out).unwrap();
    out
}

fn av_info() -> InitInfo {
    parse_init(&init(&[video_config(), audio_config(2, 48_000)])).unwrap()
}

fn samples(n: usize, duration: u32) -> Vec<Sample> {
    (0..n)
        .map(|i| Sample {
            duration,
            size: 1,
            keyframe: i == 0,
            composition_offset: 0,
        })
        .collect()
}

/// A fragment of `video` samples of 3000 at 90 kHz and `audio` of 1024 at
/// 48 kHz, one byte each.
fn fragment(sequence: u32, video: usize, audio: usize) -> Vec<u8> {
    let (v, a) = (samples(video, 3000), samples(audio, 1024));
    let (vd, ad) = (vec![0; video], vec![0; audio]);
    let runs = [
        TrackRun {
            track: TrackId(1),
            kind: TrackKind::Video,
            base_decode_time: 0,
            samples: &v,
            data: &vd,
        },
        TrackRun {
            track: TrackId(2),
            kind: TrackKind::Audio,
            base_decode_time: 0,
            samples: &a,
            data: &ad,
        },
    ];
    let mut out = Vec::new();
    write_fragment(sequence, &runs, &mut out);
    out
}

fn ms(n: i64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn split_record_parts() {
    let record = [0, 3, b'a', b'b', b'c', b'd', b'e'];
    assert_eq!(split_record(&record), Ok((&b"abc"[..], &b"de"[..])));
    assert_eq!(split_record(&[0, 0]), Ok((&[][..], &[][..])));
    for bad in [&[][..], &[0], &[0, 5, 1, 2]] {
        assert_eq!(split_record(bad), Err(ParseError::Prefix));
    }
}

#[test]
fn parse_init_reads_tracks() {
    let info = av_info();
    assert_eq!(
        info.tracks(),
        [
            TrackInfo {
                id: 1,
                handler: *b"vide",
                timescale: 90_000,
                default_duration: 0,
            },
            TrackInfo {
                id: 2,
                handler: *b"soun",
                timescale: 48_000,
                default_duration: 0,
            },
        ]
    );
    assert_eq!(info.tracks()[0].kind(), Some(TrackKind::Video));
    assert_eq!(info.tracks()[1].kind(), Some(TrackKind::Audio));
    assert_eq!(info.track(2).unwrap().timescale, 48_000);
    assert!(info.track(3).is_none());
}

/// A `trak` of `id` with version 1 `tkhd` and `mdhd`.
fn trak_v1(id: u32, handler: &[u8; 4], timescale: u32) -> Vec<u8> {
    let mut tkhd = vec![0; 16];
    tkhd.extend_from_slice(&id.to_be_bytes());
    tkhd.extend_from_slice(&[0; 80]);
    let mut mdhd = vec![0; 16];
    mdhd.extend_from_slice(&timescale.to_be_bytes());
    mdhd.extend_from_slice(&[0; 12]);
    let mut hdlr = vec![0; 4];
    hdlr.extend_from_slice(handler);
    hdlr.extend_from_slice(&[0; 13]);
    let mdia = [full(b"mdhd", 1, 0, &mdhd), full(b"hdlr", 0, 0, &hdlr)].concat();
    bx(
        b"trak",
        &[full(b"tkhd", 1, 3, &tkhd), bx(b"mdia", &mdia)].concat(),
    )
}

fn trex(id: u32, duration: u32) -> Vec<u8> {
    let mut c = id.to_be_bytes().to_vec();
    c.extend_from_slice(&1u32.to_be_bytes());
    c.extend_from_slice(&duration.to_be_bytes());
    c.extend_from_slice(&[0; 8]);
    full(b"trex", 0, 0, &c)
}

#[test]
fn parse_init_reads_version_1_boxes_and_trex_defaults() {
    let moov = [
        bx(b"udta", b"anything"),
        trak_v1(5, b"vide", 15_360),
        trak_v1(6, b"soun", 44_100),
        bx(b"mvex", &[trex(6, 1024), trex(5, 512), trex(9, 7)].concat()),
    ]
    .concat();
    let info = parse_init(&bx(b"moov", &moov)).unwrap();
    assert_eq!(
        info.tracks(),
        [
            TrackInfo {
                id: 5,
                handler: *b"vide",
                timescale: 15_360,
                default_duration: 512,
            },
            TrackInfo {
                id: 6,
                handler: *b"soun",
                timescale: 44_100,
                default_duration: 1024,
            },
        ]
    );
}

#[test]
fn parse_init_errors() {
    assert_eq!(
        parse_init(&bx(b"ftyp", b"iso5")),
        Err(ParseError::Missing(FourCc(*b"moov")))
    );
    let no_mdhd = bx(
        b"trak",
        &[full(b"tkhd", 0, 3, &[0; 80]), bx(b"mdia", &[])].concat(),
    );
    assert_eq!(
        parse_init(&bx(b"moov", &no_mdhd)),
        Err(ParseError::Missing(FourCc(*b"mdhd")))
    );
    let mut truncated = init(&[video_config()]);
    truncated.truncate(truncated.len() - 1);
    assert_eq!(
        parse_init(&truncated),
        Err(ParseError::Truncated(FourCc(*b"moov")))
    );
    let nine: Vec<u8> = (1..=9).flat_map(|i| trak_v1(i, b"soun", 8000)).collect();
    assert_eq!(
        parse_init(&bx(b"moov", &nine)),
        Err(ParseError::TooManyTracks)
    );
    let eight: Vec<u8> = (1..=8).flat_map(|i| trak_v1(i, b"soun", 8000)).collect();
    assert_eq!(parse_init(&bx(b"moov", &eight)).unwrap().tracks().len(), 8);
}

#[test]
fn media_duration_sums_the_video_track() {
    let info = av_info();
    // 30 samples of 1/30 s, against audio of 50 * 1024 / 48000 s.
    assert_eq!(
        media_duration(&info, &fragment(1, 30, 50)).unwrap(),
        Duration::SECOND
    );
    assert_eq!(
        media_duration(&info, &fragment(1, 2, 0)).unwrap(),
        Duration::from_nanos(66_666_666)
    );
}

#[test]
fn media_duration_sums_every_moof() {
    let info = av_info();
    let media = [fragment(1, 3, 1), fragment(2, 6, 1), fragment(3, 21, 0)].concat();
    assert_eq!(media_duration(&info, &media).unwrap(), Duration::SECOND);
}

#[test]
fn media_duration_sums_fragments_after_prft_boxes_and_padding() {
    // An init with edit lists declares the same tracks.
    let mut init = Vec::new();
    let configs = [video_config(), audio_config(2, 48_000)];
    write_init(&configs, Duration::from_millis(66), &mut init).unwrap();
    let info = parse_init(&init).unwrap();
    assert_eq!(info, av_info());
    let mut media = Vec::new();
    for sequence in 1..=30 {
        write_prft(
            &ProducerTime {
                track: TrackId(1),
                media_time: 0,
                published: strims_base::time::Time::EPOCH,
            },
            &mut media,
        );
        media.extend_from_slice(&fragment(sequence, 1, 1));
    }
    media.extend_from_slice(&bx(b"free", &[0; 40]));
    assert_eq!(media_duration(&info, &media).unwrap(), Duration::SECOND);
}

#[test]
fn media_duration_skips_other_top_level_boxes() {
    let info = av_info();
    let styp = bx(b"styp", b"msdh\0\0\0\0msdhmsix");
    let sidx = full(b"sidx", 1, 0, &[0; 32]);
    let media = [
        styp,
        sidx.clone(),
        sidx,
        fragment(1, 15, 0),
        bx(b"free", &[0; 100]),
        bx(b"skip", &[]),
        bx(b"abcd", b"?"),
        fragment(2, 15, 0),
    ]
    .concat();
    assert_eq!(media_duration(&info, &media).unwrap(), Duration::SECOND);
}

/// A `moof` of one `traf` for `track`: a `tfhd` with `default` duration
/// if given, and a `trun` of `count` samples with only sizes.
fn sizes_only_moof(track: u32, default: Option<u32>, count: u32) -> Vec<u8> {
    let mut tfhd = track.to_be_bytes().to_vec();
    let mut flags = 0x02_0000;
    if let Some(d) = default {
        flags |= 0x08;
        tfhd.extend_from_slice(&d.to_be_bytes());
    }
    let mut trun = count.to_be_bytes().to_vec();
    trun.extend((0..count).flat_map(|_| 1u32.to_be_bytes()));
    let traf = [
        full(b"tfhd", 0, flags, &tfhd),
        full(b"trun", 0, 0x200, &trun),
    ]
    .concat();
    bx(b"moof", &bx(b"traf", &traf))
}

#[test]
fn media_duration_takes_tfhd_then_trex_defaults() {
    let moov = [trak_v1(1, b"vide", 1000), bx(b"mvex", &trex(1, 40))].concat();
    let info = parse_init(&bx(b"moov", &moov)).unwrap();
    assert_eq!(
        media_duration(&info, &sizes_only_moof(1, Some(20), 3)).unwrap(),
        ms(60)
    );
    assert_eq!(
        media_duration(&info, &sizes_only_moof(1, None, 3)).unwrap(),
        ms(120)
    );
    // base-data-offset and sample-description-index come before the
    // default duration.
    let mut tfhd = 1u32.to_be_bytes().to_vec();
    tfhd.extend_from_slice(&[0; 12]);
    tfhd.extend_from_slice(&5u32.to_be_bytes());
    let traf = [
        full(b"tfhd", 0, 0x0b, &tfhd),
        full(b"trun", 0, 0, &2u32.to_be_bytes()),
    ]
    .concat();
    let moof = bx(b"moof", &bx(b"traf", &traf));
    assert_eq!(media_duration(&info, &moof).unwrap(), ms(10));
}

#[test]
fn media_duration_without_video_takes_the_longest_track() {
    let info = parse_init(&init(&[audio_config(2, 48_000), audio_config(3, 8_000)])).unwrap();
    let a = samples(10, 1024);
    let b = samples(1, 1024);
    let data = [0; 10];
    let runs = [
        TrackRun {
            track: TrackId(2),
            kind: TrackKind::Audio,
            base_decode_time: 0,
            samples: &a,
            data: &data,
        },
        TrackRun {
            track: TrackId(3),
            kind: TrackKind::Audio,
            base_decode_time: 0,
            samples: &b,
            data: &data[..1],
        },
    ];
    let mut media = Vec::new();
    write_fragment(1, &runs, &mut media);
    // 10240 / 48000 s against 1024 / 8000 s.
    assert_eq!(
        media_duration(&info, &media).unwrap(),
        Duration::from_nanos(213_333_333)
    );
}

#[test]
fn media_duration_ignores_undeclared_tracks() {
    let info = parse_init(&init(&[video_config()])).unwrap();
    assert_eq!(
        media_duration(&info, &sizes_only_moof(4, Some(9000), 1)).unwrap(),
        Duration::ZERO
    );
    assert_eq!(media_duration(&info, &[]).unwrap(), Duration::ZERO);
}

#[test]
fn boxes_with_64_bit_and_open_ended_sizes() {
    let info = av_info();
    let mut large = 1u32.to_be_bytes().to_vec();
    large.extend_from_slice(b"mdat");
    large.extend_from_slice(&20u64.to_be_bytes());
    large.extend_from_slice(&[0; 4]);
    let mut open = 0u32.to_be_bytes().to_vec();
    open.extend_from_slice(b"moof");
    open.extend_from_slice(&fragment(2, 30, 0)[8..]);
    let media = [fragment(1, 30, 0), large, open].concat();
    assert_eq!(media_duration(&info, &media).unwrap(), Duration::SECOND * 2);
}

#[test]
fn malformed_media_is_an_error() {
    let info = av_info();
    let mut f = fragment(1, 3, 0);
    f.truncate(f.len() - 1);
    assert_eq!(
        media_duration(&info, &f),
        Err(ParseError::Truncated(FourCc(*b"mdat")))
    );
    assert_eq!(
        media_duration(&info, &[0, 0, 0, 4, b'f']),
        Err(ParseError::Header)
    );
    assert_eq!(
        media_duration(&info, &[0, 0, 0, 4, b'f', b'r', b'e', b'e']),
        Err(ParseError::Truncated(FourCc(*b"free")))
    );
    // A sample count beyond the entries.
    let traf = [
        full(b"tfhd", 0, 0, &1u32.to_be_bytes()),
        full(b"trun", 0, 0x100, &[0, 0, 0, 3, 0, 0, 0, 1]),
    ]
    .concat();
    assert_eq!(
        media_duration(&info, &bx(b"moof", &bx(b"traf", &traf))),
        Err(ParseError::Truncated(FourCc(*b"trun")))
    );
}

fn proptest_config() -> Config {
    Config {
        cases: 1024,
        rng_seed: RngSeed::Fixed(0x6d70_3466),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn arbitrary_input_never_panics(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        let info = av_info();
        let _ = split_record(&data);
        let _ = parse_init(&data);
        let _ = media_duration(&info, &data);
        if let Ok(parsed) = parse_init(&data) {
            let _ = media_duration(&parsed, &data);
        }
    }

    #[test]
    fn corrupted_records_never_panic(
        flips in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..8),
        cut in any::<prop::sample::Index>(),
    ) {
        let init = init(&[video_config(), audio_config(2, 48_000)]);
        let mut record = (init.len() as u16).to_be_bytes().to_vec();
        record.extend_from_slice(&init);
        record.extend_from_slice(&fragment(1, 4, 3));
        for (at, byte) in flips {
            let i = at.index(record.len());
            record[i] ^= byte;
        }
        record.truncate(cut.index(record.len() + 1));
        if let Ok((init, media)) = split_record(&record) {
            if let Ok(info) = parse_init(init) {
                let _ = media_duration(&info, media);
            }
            let _ = media_duration(&InitInfo::default(), media);
        }
    }
}
