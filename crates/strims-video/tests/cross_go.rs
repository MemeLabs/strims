// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! fMP4 records checked against Go (`go run -tags golden
//! ./crates/strims-video/golden/fmp4`):
//!
//! - mp4ff decodes the records this crate writes, and finds the tracks,
//!   codecs, fragments, sample counts, durations, sizes, sync samples,
//!   composition offsets and sample bytes the access units predict;
//! - the records Go's RTMP passthru transmuxer writes for the same tag
//!   script parse here (they hold `styp` and `sidx` boxes), and their
//!   `mdat` bytes equal this crate's for the same cut points.
//!
//! Skipped when `go` isn't on PATH.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use strims_base::time::{Duration, Time};
use strims_video::fmp4::{media_duration, parse_init, split_record};
use strims_video::synth::script::{
    Script, ScriptConfig, Tag, TagKind, decode_records, expected_records,
};
use strims_video::{
    CodecParams, CutRule, FlvSource, Muxer, PackedSink, SegmentInfo, Segmenter, SourceEvent,
    TrackKind, VecSink,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn go_available() -> bool {
    match Command::new("go").arg("version").output() {
        Ok(o) if o.status.success() => true,
        _ => {
            eprintln!("skipping: `go` is not on PATH");
            false
        }
    }
}

fn go(args: &[&str], files: &[&Path]) -> String {
    let run = Command::new("go")
        .args([
            "run",
            "-tags",
            "golden",
            "./crates/strims-video/golden/fmp4",
        ])
        .args(args)
        .args(files)
        .current_dir(repo_root())
        .output()
        .expect("run go");
    let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
    assert!(
        run.status.success(),
        "go failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    stdout
}

fn tmp(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name)
}

fn received(tag: &Tag) -> Time {
    Time::from_unix_millis(i64::from(tag.timestamp))
}

/// An access unit with its payload, and the picture size of the video
/// config current when it came.
struct Unit {
    kind: TrackKind,
    dts: i64,
    pts: i64,
    keyframe: bool,
    payload: Vec<u8>,
    picture: (u16, u16),
}

struct Run {
    records: Vec<Vec<u8>>,
    infos: Vec<SegmentInfo>,
    units: Vec<Unit>,
}

/// The script through the source and a segmenter into a packed sink.
fn run(script: &Script, rule: CutRule) -> Run {
    let mut source = FlvSource::new();
    let mut segmenter = Segmenter::new(rule, PackedSink::new());
    let (mut infos, mut units, mut picture) = (Vec::new(), Vec::new(), (0, 0));
    for tag in &script.tags {
        let event = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, received(tag)),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, received(tag)),
            TagKind::Metadata => continue,
        };
        match event.unwrap() {
            Some(SourceEvent::Config(c)) => {
                if let CodecParams::Avc { width, height, .. } = c.params {
                    picture = (width, height);
                }
                segmenter.set_config(&c).unwrap();
            }
            Some(SourceEvent::Unit(u)) => {
                units.push(Unit {
                    kind: if u.track == strims_video::flv::VIDEO_TRACK {
                        TrackKind::Video
                    } else {
                        TrackKind::Audio
                    },
                    dts: u.dts,
                    pts: u.pts,
                    keyframe: u.keyframe,
                    payload: u.payload.to_vec(),
                    picture,
                });
                segmenter.push(&u).unwrap();
            }
            None => {}
        }
        infos.extend(std::iter::from_fn(|| segmenter.take_segment()));
    }
    segmenter.finish().unwrap();
    infos.extend(std::iter::from_fn(|| segmenter.take_segment()));
    let metrics = segmenter.metrics();
    assert_eq!(
        metrics.dropped_before_start + metrics.dropped_pending_config,
        0
    );
    let records = decode_records(segmenter.sink().output());
    assert_eq!(records, expected_records(script, rule));
    assert_eq!(records.len(), infos.len());
    Run {
        records,
        infos,
        units,
    }
}

fn fnv1a64(parts: &[&Unit]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in parts.iter().flat_map(|u| &u.payload) {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// What `-check` prints for `run`'s records, numbered from `first`.
fn expected_check(run: &Run, first: usize) -> String {
    let video: Vec<&Unit> = run
        .units
        .iter()
        .filter(|u| u.kind == TrackKind::Video)
        .collect();
    let audio: Vec<&Unit> = run
        .units
        .iter()
        .filter(|u| u.kind == TrackKind::Audio)
        .collect();
    let (mut v, mut a) = (0, 0);
    let mut out = String::new();
    for (i, (record, info)) in run.records.iter().zip(&run.infos).enumerate() {
        let init_len = usize::from(u16::from_be_bytes([record[0], record[1]]));
        let vs = &video[v..v + info.video_samples as usize];
        let as_ = &audio[a..a + info.audio_samples as usize];
        v += vs.len();
        a += as_.len();
        let (w, h) = vs[0].picture;
        writeln!(
            out,
            "record {} init {init_len} media {}",
            first + i,
            record.len() - 2 - init_len
        )
        .unwrap();
        out.push_str("ftyp iso5 512 iso5,iso6,mp41\n");
        writeln!(out, "track 1 vide 90000 und avc1.64001f {w}x{h} trex 1 0").unwrap();
        out.push_str("track 2 soun 48000 und mp4a.40.2 2ch 48000Hz trex 1 0\n");
        let free = if info.padding > 0 { ",free" } else { "" };
        writeln!(out, "boxes moof,mdat{free}").unwrap();
        writeln!(out, "moof {}", i + 1).unwrap();

        let last = vs[vs.len() - 1];
        let end = match video.get(v) {
            Some(next) => next.dts,
            None => {
                let previous = vs
                    .len()
                    .checked_sub(2)
                    .map_or(3000, |p| last.dts - vs[p].dts);
                last.dts + previous
            }
        };
        let size: usize = vs.iter().map(|u| u.payload.len()).sum();
        let sync = vs.iter().filter(|u| u.keyframe).count();
        let cto: i64 = vs.iter().map(|u| u.pts - u.dts).sum();
        writeln!(
            out,
            "traf 1 tfdt {} samples {} dur {} size {size} sync {sync} cto {cto} fnv {:016x}",
            vs[0].dts,
            vs.len(),
            end - vs[0].dts,
            fnv1a64(vs)
        )
        .unwrap();
        if !as_.is_empty() {
            let size: usize = as_.iter().map(|u| u.payload.len()).sum();
            writeln!(
                out,
                "traf 2 tfdt {} samples {} dur {} size {size} sync 0 cto 0 fnv {:016x}",
                as_[0].dts,
                as_.len(),
                as_.len() * 1024,
                fnv1a64(as_)
            )
            .unwrap();
        }
    }
    out
}

fn write_records(path: &Path, records: &[&[u8]]) {
    let text: String = records.iter().map(|r| hex::encode(r) + "\n").collect();
    std::fs::write(path, text).unwrap();
}

#[test]
fn go_decodes_rust_records() {
    if !go_available() {
        return;
    }
    let plain = run(
        &Script::new(&ScriptConfig {
            duration_secs: 6,
            ..ScriptConfig::default()
        }),
        CutRule::default(),
    );
    // A low-bitrate stream with B-frames whose resize lands on a blocked
    // end, so a record carries `free` padding.
    let padded = (1..30)
        .map(|g| {
            let script = Script::new(&ScriptConfig {
                video_bitrate: 150_000,
                audio_bitrate: 32_000,
                gop_frames: 30,
                duration_secs: 30,
                b_frames: true,
                resize_at_gop: Some(g),
                ..ScriptConfig::default()
            });
            run(&script, CutRule::default())
        })
        .find(|r| r.infos.iter().any(|i| i.padding > 0))
        .expect("a resize on a blocked end");

    let records: Vec<&[u8]> = plain
        .records
        .iter()
        .chain(&padded.records)
        .map(Vec::as_slice)
        .collect();
    let path = tmp("strims-video-cross-go-records.hex");
    write_records(&path, &records);
    let output = go(&["-check"], &[&path]);
    let expected = expected_check(&plain, 0) + &expected_check(&padded, plain.records.len());
    assert_eq!(output, expected);
}

/// The top-level boxes of a segment: type and content.
fn boxes(mut data: &[u8]) -> Vec<(String, &[u8])> {
    let mut out = Vec::new();
    while !data.is_empty() {
        let size = u32::from_be_bytes(data[..4].try_into().unwrap()) as usize;
        let fourcc = String::from_utf8_lossy(&data[4..8]).into_owned();
        out.push((fourcc, &data[8..size]));
        data = &data[size..];
    }
    out
}

fn mdat(media: &[u8]) -> &[u8] {
    boxes(media)
        .into_iter()
        .find(|(t, _)| t == "mdat")
        .map(|(_, c)| c)
        .expect("an mdat")
}

#[test]
fn go_passthru_records_parse_and_carry_the_same_samples() {
    if !go_available() {
        return;
    }
    let script = Script::new(&ScriptConfig::default());
    let mut text = String::new();
    for tag in &script.tags {
        let kind = match tag.kind {
            TagKind::Metadata => "meta",
            TagKind::Video => "video",
            TagKind::Audio => "audio",
        };
        writeln!(text, "{kind} {} {}", tag.timestamp, hex::encode(&tag.body)).unwrap();
    }
    let script_path = tmp("strims-video-cross-go-script.txt");
    std::fs::write(&script_path, text).unwrap();
    let out_path = tmp("strims-video-cross-go-passthru.hex");
    go(&["-passthru"], &[&script_path, &out_path]);
    let go_records: Vec<Vec<u8>> = std::fs::read_to_string(&out_path)
        .unwrap()
        .lines()
        .map(|l| hex::decode(l).unwrap())
        .collect();
    assert!(go_records.len() >= 3, "{} records", go_records.len());

    let mut source = FlvSource::new();
    let mut segmenter = Segmenter::new(CutRule::default(), VecSink::new());
    for tag in &script.tags {
        let event = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, received(tag)),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, received(tag)),
            TagKind::Metadata => continue,
        };
        match event.unwrap() {
            Some(SourceEvent::Config(c)) => segmenter.set_config(&c).unwrap(),
            Some(SourceEvent::Unit(u)) => segmenter.push(&u).unwrap(),
            None => {}
        }
    }
    segmenter.finish().unwrap();
    let rust_records = segmenter.into_sink().records;
    assert!(rust_records.len() > go_records.len());

    let frame = Duration::from_millis(34);
    let slack = Duration::from_millis(5);
    for (i, (g, r)) in go_records.iter().zip(&rust_records).enumerate() {
        let (go_init, go_media) = split_record(g).unwrap();
        let info = parse_init(go_init).unwrap();
        let tracks: Vec<(u32, [u8; 4])> = info.tracks().iter().map(|t| (t.id, t.handler)).collect();
        assert_eq!(tracks, [(1, *b"vide"), (2, *b"soun")]);
        // About 512 video units per frame, computed in floating point.
        assert!(info.tracks()[0].timescale.abs_diff(30 * 512) <= 1);
        assert_eq!(info.tracks()[1].timescale, 48_000);
        let types: Vec<String> = boxes(go_media).into_iter().map(|(t, _)| t).collect();
        assert_eq!(
            types,
            ["styp", "sidx", "sidx", "moof", "mdat"],
            "record {i}"
        );
        let go_duration = media_duration(&info, go_media).unwrap();

        let (rust_init, rust_media) = split_record(r).unwrap();
        assert_eq!(mdat(go_media), mdat(rust_media), "record {i}");
        let rust_duration = media_duration(&parse_init(rust_init).unwrap(), rust_media).unwrap();
        // The transmuxer times a sample by the one before it, so its first
        // record lacks a frame.
        let gap = (rust_duration - go_duration).abs();
        if i == 0 {
            assert!(
                gap <= frame + slack,
                "record 0: {go_duration} {rust_duration}"
            );
        } else {
            assert!(gap <= slack, "record {i}: {go_duration} {rust_duration}");
        }
    }
}
