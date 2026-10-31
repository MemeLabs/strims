// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The remuxer's silence against ffmpeg: real encoders' streams with
//! audio missing, remuxed, and their audio decoded back by ffmpeg.
//! Skipped when `ffmpeg` isn't on PATH or lacks libx264 or libopus.
//!
//! - libx264 with B-frames and AAC-LC stereo, from ffmpeg's FLV muxer,
//!   its audio missing for half a second and from 4.5 s on: the audio
//!   decodes without an error, lasts until within `AUDIO_FOLLOW` of the
//!   video's end, and is silent where it was missing.
//! - Opus alone, from libopus in its SILK and CELT modes, missing 0.4 s:
//!   it decodes without an error in ffmpeg's decoder and libopus, as long
//!   as it was, and silent in the hole.

#![cfg(not(target_arch = "wasm32"))]
// Test code: a panic or out-of-range index is a test failure.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::PathBuf;
use std::process::{Command, Stdio};

use strims_base::time::Time;
use strims_chunkstream::chunked::{ChunkedRecordDecoder, Item};
use strims_player::remux::AUDIO_FOLLOW;
use strims_player::webcodecs::opus_packet_samples;
use strims_player::{Output, Remuxer};
use strims_video::units::MAX_RECORD_LEN;
use strims_video::{
    AccessUnit, CodecParams, DecoderConfig, FlvSource, Muxer, SourceEvent, TrackId, TrackKind,
    UnitWriter,
};

const FRAME: usize = 4096;
const RATE: i64 = 48_000;
const EPOCH_MS: i64 = 1_793_275_200_000;

fn ffmpeg_with(encoder: &str) -> bool {
    let out = Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output();
    let found = out.is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains(encoder));
    if !found {
        eprintln!("skipped: no ffmpeg with {encoder} on PATH");
    }
    found
}

/// Runs ffmpeg with `args`, answering its output; panics on its errors.
fn ffmpeg(args: &[&str]) -> Vec<u8> {
    let out = Command::new("ffmpeg")
        .args(["-hide_banner", "-v", "error"])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let errors = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success() && errors.trim().is_empty(),
        "ffmpeg {args:?}: {errors}"
    );
    out.stdout
}

#[derive(Debug, Clone)]
enum Event {
    Config(DecoderConfig),
    Unit {
        track: TrackId,
        dts: i64,
        pts: i64,
        keyframe: bool,
        payload: Vec<u8>,
    },
}

/// The tags of an FLV file: type, timestamp, body.
fn flv_tags(flv: &[u8]) -> Vec<(u8, u32, Vec<u8>)> {
    let mut at = u32::from_be_bytes(flv[5..9].try_into().unwrap()) as usize + 4;
    let mut out = Vec::new();
    while at + 11 <= flv.len() {
        let size = u32::from_be_bytes([0, flv[at + 1], flv[at + 2], flv[at + 3]]) as usize;
        let ts = u32::from_be_bytes([flv[at + 7], flv[at + 4], flv[at + 5], flv[at + 6]]);
        out.push((flv[at], ts, flv[at + 11..at + 11 + size].to_vec()));
        at += 11 + size + 4;
    }
    out
}

fn flv_events(flv: &[u8]) -> Vec<Event> {
    let mut source = FlvSource::new();
    let mut out = Vec::new();
    for (kind, ts, body) in flv_tags(flv) {
        let at = Time::from_unix_millis(EPOCH_MS + i64::from(ts));
        let event = match kind {
            9 => source.video(ts, &body, at),
            8 => source.audio(ts, &body, at),
            _ => continue,
        };
        match event.unwrap() {
            Some(SourceEvent::Config(c)) => out.push(Event::Config(c)),
            Some(SourceEvent::Unit(u)) => out.push(Event::Unit {
                track: u.track,
                dts: u.dts,
                pts: u.pts,
                keyframe: u.keyframe,
                payload: u.payload.to_vec(),
            }),
            None => {}
        }
    }
    out
}

/// The packets of an Ogg stream.
fn ogg_packets(d: &[u8]) -> Vec<Vec<u8>> {
    let (mut out, mut packet, mut at) = (Vec::new(), Vec::new(), 0);
    while at + 27 <= d.len() {
        let n = d[at + 26] as usize;
        let mut body = at + 27 + n;
        for &len in &d[at + 27..at + 27 + n] {
            packet.extend_from_slice(&d[body..body + len as usize]);
            body += len as usize;
            if len < 255 {
                out.push(std::mem::take(&mut packet));
            }
        }
        at = body;
    }
    out
}

/// The remuxed fMP4 of `events`: the init and every fragment after it.
fn remux(events: &[Event]) -> (Vec<u8>, Remuxer) {
    let mut w = UnitWriter::new(FRAME).unwrap();
    for e in events {
        match e {
            Event::Config(c) => w.set_config(c).unwrap(),
            Event::Unit {
                track,
                dts,
                pts,
                keyframe,
                payload,
            } => w
                .push(&AccessUnit {
                    track: *track,
                    dts: *dts,
                    pts: *pts,
                    keyframe: *keyframe,
                    payload,
                    published: Time::from_unix_millis(EPOCH_MS + dts / 48),
                })
                .unwrap(),
        }
    }
    w.finish().unwrap();
    let stream = w.output().to_vec();
    let mut d = ChunkedRecordDecoder::new(FRAME, MAX_RECORD_LEN).unwrap();
    let mut r = Remuxer::new();
    let mut mp4 = Vec::new();
    let mut at = 0;
    let take = |r: &mut Remuxer, mp4: &mut Vec<u8>| {
        while let Some(o) = r.poll_output() {
            match o {
                Output::Init { data, .. } => {
                    assert!(mp4.is_empty(), "a second init");
                    mp4.extend_from_slice(data);
                }
                Output::Media { data, .. } => mp4.extend_from_slice(data),
                _ => {}
            }
        }
    };
    loop {
        let (n, item) = d.decode(&stream[at..]);
        at += n;
        match item {
            Some(Item::Record { payload, .. }) => {
                r.message(payload, true, false);
                take(&mut r, &mut mp4);
            }
            Some(Item::Lost) => panic!("lost records"),
            None => break,
        }
    }
    r.finish();
    take(&mut r, &mut mp4);
    (mp4, r)
}

/// The first audio track's samples, decoded by ffmpeg (with `decoder`
/// when given), mono at 48 kHz, from its first packet on.
fn decode(name: &str, mp4: &[u8], decoder: Option<&str>) -> Vec<i16> {
    let path: PathBuf = [env!("CARGO_TARGET_TMPDIR"), name].iter().collect();
    std::fs::write(&path, mp4).unwrap();
    let mut args = vec!["-ignore_editlist", "1"];
    if let Some(d) = decoder {
        args.extend(["-c:a", d]);
    }
    let file = path.to_str().unwrap();
    args.extend([
        "-i", file, "-map", "0:a:0", "-ac", "1", "-f", "s16le", "pipe:1",
    ]);
    let pcm = ffmpeg(&args);
    pcm.as_chunks::<2>()
        .0
        .iter()
        .map(|b| i16::from_le_bytes(*b))
        .collect()
}

/// The loudest sample of `pcm` in `[from, to)`.
fn peak(pcm: &[i16], from: i64, to: i64) -> i32 {
    let (from, to) = (from.max(0) as usize, (to.max(0) as usize).min(pcm.len()));
    pcm[from..to.max(from)]
        .iter()
        .map(|s| i32::from(*s).abs())
        .max()
        .unwrap_or(0)
}

#[test]
fn aac_holes_decode_as_silence_in_ffmpeg() {
    if !ffmpeg_with("libx264") {
        return;
    }
    let flv = ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=320x240:rate=30",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:sample_rate=48000",
        "-t",
        "6",
        "-c:v",
        "libx264",
        "-bf",
        "2",
        "-g",
        "60",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-ac",
        "2",
        "-f",
        "flv",
        "pipe:1",
    ]);
    let events = flv_events(&flv);
    let audio = |e: &Event| matches!(e, Event::Unit { track, .. } if *track == TrackId(2));
    let first = events
        .iter()
        .find_map(|e| match e {
            Event::Unit { track, dts, .. } if *track == TrackId(2) => Some(*dts),
            _ => None,
        })
        .unwrap();
    // Audio missing over [2, 2.5) s and from 4.5 s on.
    let (hole, stop) = ((2 * RATE, 2 * RATE + RATE / 2), 9 * RATE / 2);
    let kept: Vec<Event> = events
        .iter()
        .filter(|e| match e {
            Event::Unit { dts, .. } if audio(e) => !(hole.0..hole.1).contains(dts) && *dts < stop,
            _ => true,
        })
        .cloned()
        .collect();
    let (mp4, r) = remux(&kept);
    let m = r.metrics();
    assert!(m.silence_fills >= 2 && m.audio_holes == 0, "{m:?}");
    let pcm = decode("aac-holes.mp4", &mp4, None);
    // The tone plays before and after the hole, and silence is digital
    // zero past the frame that overlaps the last one before it.
    let at = |t: i64| t - first;
    assert!(peak(&pcm, at(RATE), at(2 * RATE - 2048)) > 1000);
    assert!(peak(&pcm, at(3 * RATE), at(4 * RATE)) > 1000);
    assert_eq!(peak(&pcm, at(hole.0 + 2048), at(hole.1 - 1024)), 0);
    assert_eq!(peak(&pcm, at(stop + 2048), pcm.len() as i64), 0);
    // Silence follows the video to its end, less AUDIO_FOLLOW and a frame.
    let follow = AUDIO_FOLLOW.as_nanos() * RATE / 1_000_000_000;
    let ends = pcm.len() as i64 + first;
    assert!(
        ends >= 6 * RATE - follow - 2 * 1024 - RATE / 10,
        "audio ends at {ends}"
    );
}

#[test]
fn opus_holes_decode_as_silence_in_ffmpeg_and_libopus() {
    if !ffmpeg_with("libopus") {
        return;
    }
    for (mode, bitrate) in [("voip", "16k"), ("audio", "96k")] {
        let ogg = ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000",
            "-t",
            "4",
            "-ac",
            "2",
            "-c:a",
            "libopus",
            "-application",
            mode,
            "-b:a",
            bitrate,
            "-frame_duration",
            "20",
            "-f",
            "ogg",
            "pipe:1",
        ]);
        let packets = ogg_packets(&ogg);
        let head = packets[0].clone();
        let mut events = vec![Event::Config(DecoderConfig {
            track: TrackId(2),
            kind: TrackKind::Audio,
            codec: "Opus".into(),
            timescale: 48_000,
            params: CodecParams::Opus {
                head,
                channels: 2,
                pre_skip: 312,
            },
        })];
        let hole = (RATE * 3 / 2, RATE * 19 / 10);
        let mut dts = 0;
        for p in &packets[2..] {
            if !(hole.0..hole.1).contains(&dts) {
                events.push(Event::Unit {
                    track: TrackId(2),
                    dts,
                    pts: dts,
                    keyframe: true,
                    payload: p.clone(),
                });
            }
            dts += i64::from(opus_packet_samples(p).unwrap());
        }
        let (mp4, r) = remux(&events);
        assert_eq!(r.metrics().silence_units, 20, "{mode}");
        for decoder in [None, Some("libopus")] {
            let pcm = decode(&format!("opus-{mode}.mp4"), &mp4, decoder);
            // ffmpeg drops the pre-skip; packets of 960 samples.
            let at = |t: i64| t - 312;
            assert!(
                (pcm.len() as i64 - at(dts)).abs() <= 960,
                "{mode} {decoder:?}: {} samples of {dts}",
                pcm.len()
            );
            assert!(peak(&pcm, at(RATE / 2), at(hole.0 - 960)) > 1000);
            assert!(peak(&pcm, at(hole.1 + 960), at(3 * RATE)) > 1000);
            assert_eq!(
                peak(&pcm, at(hole.0 + 2 * 960), at(hole.1 - 960)),
                0,
                "{mode} {decoder:?}"
            );
        }
    }
}
