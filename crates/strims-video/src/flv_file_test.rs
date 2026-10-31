// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_base::time::Time;

use super::*;
use crate::flv::{FlvSource, SourceEvent};
use crate::media::{TrackId, TrackKind};
use crate::synth::script::{aac_lc_asc, av1c, avcc, hvcc};

const NOW: Time = Time::from_unix_millis(1_793_275_200_000);

/// A unit's track, DTS, PTS, keyframe flag and payload.
type Unit = (TrackId, i64, i64, bool, Vec<u8>);

fn video(params: CodecParams, codec: &str) -> DecoderConfig {
    DecoderConfig {
        track: TrackId(1),
        kind: TrackKind::Video,
        codec: codec.into(),
        timescale: 90_000,
        params,
    }
}

fn h264() -> DecoderConfig {
    video(
        CodecParams::Avc {
            avcc: avcc(640, 360),
            width: 640,
            height: 360,
        },
        "avc1.64001f",
    )
}

fn h265() -> DecoderConfig {
    video(
        CodecParams::Hevc {
            hvcc: hvcc(640, 360),
            width: 640,
            height: 360,
        },
        "hvc1.1.6.L93.90",
    )
}

fn av1() -> DecoderConfig {
    video(
        CodecParams::Av1 {
            av1c: av1c(640, 360),
            width: 640,
            height: 360,
        },
        "av01.0.08M.08",
    )
}

fn aac() -> DecoderConfig {
    DecoderConfig {
        track: TrackId(2),
        kind: TrackKind::Audio,
        codec: "mp4a.40.2".into(),
        timescale: 48_000,
        params: CodecParams::Aac {
            asc: aac_lc_asc(48_000),
            sample_rate: 48_000,
            channels: 2,
            frame_samples: 1024,
        },
    }
}

/// Units of a stream starting at DTS `first` (90 kHz): a frame every 30
/// ms with B-frame composition offsets, a keyframe every five, and AAC
/// frames up to each: track, DTS, PTS, keyframe, payload.
fn units(first: i64, frames: i64) -> Vec<Unit> {
    let mut out = Vec::new();
    let mut audio = 0i64;
    for k in 0..frames {
        let dts = first + k * 2700;
        let cts = if k % 3 == 1 { 0 } else { 5400 };
        out.push((
            TrackId(1),
            dts,
            dts + cts,
            k % 5 == 0,
            vec![0x32, 1, k as u8],
        ));
        while audio * 1024 * 90_000 / 48_000 <= k * 2700 {
            let dts = first * 48_000 / 90_000 + audio * 1024;
            out.push((TrackId(2), dts, dts, true, vec![0xa0, audio as u8]));
            audio += 1;
        }
    }
    out
}

fn write(video: &DecoderConfig, units: &[Unit]) -> Vec<u8> {
    let audio = aac();
    let mut w = FlvWriter::new(1000);
    let mut out = Vec::new();
    w.header(&[video.clone(), audio.clone()], Some(33.3), &mut out);
    w.config(video, &mut out).unwrap();
    w.config(&audio, &mut out).unwrap();
    for (track, dts, pts, keyframe, payload) in units {
        let config = if *track == video.track { video } else { &audio };
        let u = AccessUnit {
            track: *track,
            dts: *dts,
            pts: *pts,
            keyframe: *keyframe,
            payload,
            published: NOW,
        };
        w.unit(config, &u, &mut out).unwrap();
    }
    out
}

/// What a source makes of a file's tags, read `split` bytes at a time.
fn read(file: &[u8], split: usize) -> (Vec<DecoderConfig>, Vec<Unit>) {
    let mut r = FlvTagReader::new(1 << 20);
    let mut s = FlvSource::new();
    let (mut configs, mut units) = (Vec::new(), Vec::new());
    for chunk in file.chunks(split.max(1)) {
        r.push(chunk);
        while let Some(tag) = r.next_tag().unwrap() {
            let e = match tag.kind {
                TAG_VIDEO => s.video(tag.timestamp, tag.body, NOW).unwrap(),
                TAG_AUDIO => s.audio(tag.timestamp, tag.body, NOW).unwrap(),
                _ => None,
            };
            match e {
                Some(SourceEvent::Config(c)) => configs.push(c),
                Some(SourceEvent::Unit(u)) => {
                    units.push((u.track, u.dts, u.pts, u.keyframe, u.payload.to_vec()));
                }
                None => {}
            }
        }
    }
    assert_eq!(r.buffered(), 0);
    (configs, units)
}

/// A unit's times from the writer's start: its own less `first`'s, plus 1
/// s.
fn shifted(units: &[Unit], first: i64) -> Vec<Unit> {
    units
        .iter()
        .map(|(t, dts, pts, k, p)| {
            let by = if *t == TrackId(1) {
                90_000 - first
            } else {
                48_000 - first * 48_000 / 90_000
            };
            (*t, dts + by, pts + by, *k, p.clone())
        })
        .collect()
}

#[test]
fn every_video_codec_reads_back_through_the_source() {
    for config in [h264(), h265(), av1()] {
        let first = 5_400_000;
        let units = units(first, 20);
        let file = write(&config, &units);
        let (configs, read) = read(&file, file.len());
        assert_eq!(configs, [config.clone(), aac()]);
        let expected: Vec<_> = shifted(&units, first)
            .into_iter()
            // AV1 frames carry no composition time
            .map(|(t, dts, pts, k, p)| {
                let av1 = matches!(config.params, CodecParams::Av1 { .. });
                (t, dts, if av1 && t == TrackId(1) { dts } else { pts }, k, p)
            })
            .collect();
        assert_eq!(read, expected, "{}", config.codec);
    }
}

#[test]
fn a_file_reads_the_same_in_any_pieces() {
    let file = write(&h265(), &units(0, 12));
    let whole = read(&file, file.len());
    for split in [1, 2, 3, 7, 11, 15, 64] {
        assert_eq!(read(&file, split), whole, "{split}-byte pieces");
    }
}

#[test]
fn the_header_names_the_streams_codecs() {
    let mut out = Vec::new();
    FlvWriter::new(0).header(&[h265(), aac()], Some(29.97), &mut out);
    assert_eq!(&out[..13], b"FLV\x01\x05\x00\x00\x00\x09\x00\x00\x00\x00");
    let mut r = FlvTagReader::new(1024);
    r.push(&out);
    let tag = r.next_tag().unwrap().unwrap();
    assert_eq!((tag.kind, tag.timestamp), (TAG_SCRIPT, 0));
    let values = crate::amf0::decode_all(tag.body).unwrap();
    assert_eq!(values[0], Value::String("onMetaData"));
    let m = crate::amf0::Metadata::from_value(&values[1]).unwrap();
    assert_eq!(
        (m.width, m.height, m.framerate),
        (Some(640.0), Some(360.0), Some(29.97))
    );
    assert_eq!(
        m.videocodecid,
        Some(f64::from(u32::from_be_bytes(*b"hvc1")))
    );
    assert_eq!(m.audiocodecid, Some(10.0));
}

#[test]
fn zero_composition_times_pack_hevc_frames_without_one() {
    let config = h265();
    let mut w = FlvWriter::new(0);
    let mut out = Vec::new();
    let unit = |pts| AccessUnit {
        track: TrackId(1),
        dts: 0,
        pts,
        keyframe: false,
        payload: b"p",
        published: NOW,
    };
    w.unit(&config, &unit(0), &mut out).unwrap();
    assert_eq!(&out[11..16], [0x80 | 0x20 | 3, b'h', b'v', b'c', b'1']);
    out.clear();
    w.unit(&config, &unit(-90), &mut out).unwrap();
    assert_eq!(
        &out[11..19],
        [0x80 | 0x20 | 1, b'h', b'v', b'c', b'1', 0xff, 0xff, 0xff]
    );
}

#[test]
fn tag_times_count_from_the_start_and_wrap() {
    let config = h264();
    let mut w = FlvWriter::new(1000);
    let mut out = Vec::new();
    let at = |dts| AccessUnit {
        track: TrackId(1),
        dts,
        pts: dts,
        keyframe: true,
        payload: b"p",
        published: NOW,
    };
    let time = |out: &[u8]| u32::from_be_bytes([out[7], out[4], out[5], out[6]]);
    w.unit(&config, &at(90_000), &mut out).unwrap();
    assert_eq!((time(&out), w.base()), (1000, Some(1000)));
    out.clear();
    // before the first unit by less than the start
    w.unit(&config, &at(45_000), &mut out).unwrap();
    assert_eq!(time(&out), 500);
    out.clear();
    // and by more
    w.unit(&config, &at(-90_000), &mut out).unwrap();
    assert_eq!(time(&out), 0);
    out.clear();
    w.unit(&config, &at((1 << 32) * 90 + 90_000), &mut out)
        .unwrap();
    assert_eq!(time(&out), 1000);
}

#[test]
fn units_flv_cant_carry_are_refused() {
    let mut w = FlvWriter::new(0);
    let mut out = Vec::new();
    let opus = DecoderConfig {
        params: CodecParams::Opus {
            head: b"OpusHead".to_vec(),
            channels: 2,
            pre_skip: 312,
        },
        ..aac()
    };
    assert_eq!(w.config(&opus, &mut out), Err(FlvWriteError::Codec("Opus")));
    let late = AccessUnit {
        track: TrackId(1),
        dts: 0,
        pts: 90 * (1 << 23),
        keyframe: true,
        payload: b"p",
        published: NOW,
    };
    assert_eq!(
        w.unit(&h264(), &late, &mut out),
        Err(FlvWriteError::CompositionTime(1 << 23))
    );
    let zero = DecoderConfig {
        timescale: 0,
        ..h264()
    };
    assert_eq!(
        w.unit(&zero, &late, &mut out),
        Err(FlvWriteError::Timescale)
    );
    assert!(out.is_empty());
}

#[test]
fn malformed_files_fail_and_stay_failed() {
    let file = write(&h264(), &units(0, 3));
    let first = |bytes: &[u8]| {
        let mut r = FlvTagReader::new(1 << 16);
        r.push(bytes);
        let mut e = Ok(());
        while e.is_ok() {
            match r.next_tag() {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(err) => e = Err(err),
            }
        }
        (e, r)
    };
    let mut bad = file.clone();
    bad[0] = b'G';
    assert_eq!(first(&bad).0, Err(FlvFileError::Signature));
    let mut bad = file.clone();
    bad[3] = 2;
    assert_eq!(first(&bad).0, Err(FlvFileError::Version(2)));
    let mut bad = file.clone();
    bad[8] = 4;
    assert_eq!(first(&bad).0, Err(FlvFileError::DataOffset(4)));
    let mut bad = file.clone();
    bad[13] |= FILTER;
    assert_eq!(first(&bad).0, Err(FlvFileError::Encrypted));
    let mut bad = file.clone();
    let size = u32::from_be_bytes([0, bad[14], bad[15], bad[16]]) as usize;
    bad[13 + 11 + size + 3] ^= 1;
    assert!(matches!(first(&bad).0, Err(FlvFileError::TagSize { .. })));
    let (e, mut r) = first(&file);
    assert_eq!(e, Ok(()));
    let mut small = FlvTagReader::new(8);
    small.push(&file);
    assert!(matches!(small.next_tag(), Err(FlvFileError::TagTooLong(_))));
    small.push(&file);
    assert!(matches!(small.next_tag(), Err(FlvFileError::TagTooLong(_))));
    assert_eq!(small.buffered(), 0);
    r.push(b"x");
    assert_eq!(r.next_tag(), Ok(None));
}

#[test]
fn bytes_between_the_header_and_its_data_offset_are_skipped() {
    let file = write(&h264(), &units(0, 3));
    let mut padded = file[..9].to_vec();
    padded[8] = 12;
    padded.extend_from_slice(&[0xee; 3]);
    padded.extend_from_slice(&file[9..]);
    assert_eq!(read(&padded, 2), read(&file, file.len()));
}

fn proptest_config() -> Config {
    Config {
        cases: 256,
        rng_seed: RngSeed::Fixed(0x0f1f_f11e),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn arbitrary_bytes_never_panic(
        head in proptest::collection::vec(any::<u8>(), 0..64),
        rest in proptest::collection::vec(any::<u8>(), 0..512),
        split in 1usize..64,
    ) {
        let mut r = FlvTagReader::new(256);
        let mut bytes = b"FLV\x01\x05\x00\x00\x00\x09\x00\x00\x00\x00".to_vec();
        bytes.extend_from_slice(&head);
        bytes.extend_from_slice(&rest);
        for chunk in bytes.chunks(split) {
            r.push(chunk);
            while let Ok(Some(tag)) = r.next_tag() {
                prop_assert!(tag.body.len() <= 256);
            }
            prop_assert!(r.buffered() <= 256 + 15 + 64);
        }
    }

    #[test]
    fn written_streams_read_back_in_any_pieces(
        frames in 1i64..40,
        first in 0i64..1_000_000_000,
        split in 1usize..200,
        codec in 0usize..3,
    ) {
        let config = [h264(), h265(), av1()][codec].clone();
        let units = units(first, frames);
        let file = write(&config, &units);
        let (configs, read) = read(&file, split);
        prop_assert_eq!(configs.len(), 2);
        prop_assert_eq!(read.len(), units.len());
    }
}
