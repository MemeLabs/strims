// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The cue reader's CC1 captions against ffmpeg's decoder of them. Skipped
//! when `ffmpeg` isn't on PATH.
//!
//! ffmpeg encodes a test pattern as H.264 with B-frames into FLV, whose
//! frames, in presentation order, take a field 1 pair each of pop-on
//! captions, then roll-up ones. ffmpeg decodes the captions of the FLV
//! (`movie=…[out0+subcc]`), and a [`CueReader`] reads them of its units in
//! decode order:
//!
//! - each pop-on caption shows from the same frame to the same frame,
//!   with the same text, on ffmpeg's timeline, which starts at the
//!   stream's first frame;
//! - roll-up shows the same lines. ffmpeg times a roll-up caption from the
//!   carriage return before its last line, a line behind the screen, so
//!   the times and the rows shown together differ.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::VecDeque;
use std::process::Command;

use strims_base::time::Time;
use strims_video::captions::NalFormat;
use strims_video::flv_file::FlvWriter;
use strims_video::synth::captions::{field1, pair, pop_on};
use strims_video::text::{CueEvent, CueKind, CueReader};
use strims_video::{AccessUnit, DecoderConfig, FlvSource, SourceEvent, TrackKind};

fn ffmpeg() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn temp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("strims-video-{}-{name}", std::process::id()))
}

/// A unit the source made, owned.
struct Unit {
    config: usize,
    dts: i64,
    pts: i64,
    keyframe: bool,
    payload: Vec<u8>,
}

/// ffmpeg's FLV of 10 s of a 30 fps test pattern in H.264 with B-frames,
/// as the source reads it.
fn source() -> (Vec<DecoderConfig>, Vec<Unit>) {
    let path = temp("cc-source.flv");
    let ok = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg("testsrc2=size=320x240:rate=30:duration=10")
        .args([
            "-c:v", "libx264", "-bf", "2", "-g", "60", "-pix_fmt", "yuv420p",
        ])
        .args(["-f", "flv"])
        .arg(&path)
        .status()
        .unwrap()
        .success();
    assert!(ok, "ffmpeg encodes the test pattern");
    let flv = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let mut source = FlvSource::new();
    let mut configs: Vec<DecoderConfig> = Vec::new();
    let mut units = Vec::new();
    let mut at = u32::from_be_bytes(flv[5..9].try_into().unwrap()) as usize + 4;
    while at + 11 <= flv.len() {
        let kind = flv[at];
        let size = u32::from_be_bytes([0, flv[at + 1], flv[at + 2], flv[at + 3]]) as usize;
        let ts = u32::from_be_bytes([flv[at + 7], flv[at + 4], flv[at + 5], flv[at + 6]]);
        let body = &flv[at + 11..at + 11 + size];
        at += 11 + size + 4;
        if kind != 9 {
            continue;
        }
        match source.video(ts, body, Time::from_unix_millis(0)).unwrap() {
            Some(SourceEvent::Config(c)) => configs.push(c),
            Some(SourceEvent::Unit(u)) => units.push(Unit {
                config: configs.len() - 1,
                dts: u.dts,
                pts: u.pts,
                keyframe: u.keyframe,
                payload: u.payload.to_vec(),
            }),
            None => {}
        }
    }
    (configs, units)
}

/// Pop-on captions, then roll-up ones of two rows, a pair a frame.
fn pairs() -> VecDeque<[u8; 2]> {
    let mut out = VecDeque::new();
    for (k, text) in ["FIRST CAPTION", "SECOND ONE", "THIRD"].iter().enumerate() {
        out.extend(pop_on(text));
        // Pauses between: a frame without pairs per null.
        out.extend(std::iter::repeat_n(pair(0, 0), 20 + 10 * k));
    }
    out.extend([pair(0x14, 0x2c), pair(0x14, 0x2c)]);
    out.extend(std::iter::repeat_n(pair(0, 0), 20));
    out.extend([pair(0x14, 0x25), pair(0x14, 0x25)]);
    out.extend([pair(0x14, 0x70), pair(0x14, 0x70)]);
    for line in ["ROLL ONE", "ROLL TWO", "ROLL THREE"] {
        let mut bytes = line.as_bytes().to_vec();
        if bytes.len() % 2 == 1 {
            bytes.push(0);
        }
        for c in bytes.chunks(2) {
            out.push_back(pair(c[0], c[1]));
        }
        out.extend([pair(0x14, 0x2d), pair(0x14, 0x2d)]);
        out.extend(std::iter::repeat_n(pair(0, 0), 30));
    }
    out
}

/// A caption: its start and end in milliseconds and its text.
type Caption = (i64, i64, String);

/// The CC1 captions ffmpeg decodes of an FLV file, without markup.
fn ffmpeg_captions(configs: &[DecoderConfig], units: &[Unit]) -> Vec<Caption> {
    let mut w = FlvWriter::new(0);
    let mut flv = Vec::new();
    w.header(configs, None, &mut flv);
    w.config(&configs[0], &mut flv).unwrap();
    for u in units {
        let unit = AccessUnit {
            track: configs[u.config].track,
            dts: u.dts,
            pts: u.pts,
            keyframe: u.keyframe,
            payload: &u.payload,
            published: Time::from_unix_millis(0),
        };
        w.unit(&configs[u.config], &unit, &mut flv).unwrap();
    }
    let path = temp("cc-captions.flv");
    std::fs::write(&path, &flv).unwrap();
    let out = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
        .arg(format!("movie={}[out0+subcc]", path.display()))
        .args(["-map", "0:1", "-f", "srt", "pipe:1"])
        .output()
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let srt = String::from_utf8(out.stdout).unwrap();
    let ms = |t: &str| {
        let (hms, millis) = t.trim().split_once(',').unwrap();
        let s = hms
            .split(':')
            .fold(0i64, |a, p| a * 60 + p.parse::<i64>().unwrap());
        s * 1000 + millis.parse::<i64>().unwrap()
    };
    srt.split("\n\n")
        .filter_map(|block| {
            let mut lines = block.lines().skip(1);
            let (start, end) = lines.next()?.split_once(" --> ")?;
            let mut text = String::new();
            let mut markup = None;
            for c in lines.collect::<Vec<_>>().join("\n").chars() {
                match (markup, c) {
                    (None, '<') => markup = Some('>'),
                    (None, '{') => markup = Some('}'),
                    (Some(end), c) if c == end => markup = None,
                    (None, c) => text.push(c),
                    (Some(_), _) => {}
                }
            }
            let text: Vec<&str> = text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect();
            Some((ms(start), ms(end), text.join("\n")))
        })
        .collect()
}

#[test]
fn the_cue_readers_captions_are_ffmpegs() {
    if !ffmpeg() {
        eprintln!("skipped: no ffmpeg");
        return;
    }
    let (configs, mut units) = source();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].kind, TrackKind::Video);
    let nals = NalFormat::of(&configs[0].params).unwrap();
    let mut order: Vec<usize> = (0..units.len()).collect();
    order.sort_by_key(|&i| units[i].pts);
    let mut pairs = pairs();
    assert!(pairs.len() < order.len());
    for &i in &order {
        let Some(p) = pairs.pop_front() else {
            break;
        };
        if p == pair(0, 0) {
            continue;
        }
        let mut out = Vec::new();
        nals.with_cc_sei(&units[i].payload, &[field1(p)], &mut out)
            .unwrap();
        units[i].payload = out;
    }

    let mut r = CueReader::new();
    r.set_configs(&configs);
    // On ffmpeg's timeline.
    let first = units.iter().map(|u| u.pts).min().unwrap() / 90;
    let mut ours: Vec<Caption> = Vec::new();
    let mut ids = Vec::new();
    for u in &units {
        r.push(&AccessUnit {
            track: configs[u.config].track,
            dts: u.dts,
            pts: u.pts,
            keyframe: u.keyframe,
            payload: &u.payload,
            published: Time::from_unix_millis(0),
        });
        while let Some(e) = r.poll_event() {
            match e {
                CueEvent::Show {
                    id,
                    kind,
                    start,
                    end,
                    text,
                } => {
                    assert_eq!(kind, CueKind::Captions);
                    ids.push(id);
                    ours.push((start.as_millis() - first, end.as_millis() - first, text));
                }
                CueEvent::End { id, end } => {
                    let i = ids.iter().position(|&i| i == id).unwrap();
                    ours[i].1 = end.as_millis() - first;
                }
            }
        }
    }
    let theirs = ffmpeg_captions(&configs, &units);
    eprintln!("ours:   {ours:?}\ntheirs: {theirs:?}");
    let pop_on = |c: &[Caption]| -> Vec<Caption> {
        c.iter()
            .filter(|c| !c.2.starts_with("ROLL"))
            .cloned()
            .collect()
    };
    assert_eq!(pop_on(&ours), pop_on(&theirs));
    assert_eq!(pop_on(&ours).len(), 3);
    let lines = |c: &[Caption]| -> std::collections::BTreeSet<String> {
        c.iter()
            .flat_map(|c| c.2.lines().map(str::to_owned).collect::<Vec<_>>())
            .filter(|l| l.starts_with("ROLL"))
            .collect()
    };
    assert_eq!(lines(&ours), lines(&theirs));
    assert_eq!(lines(&ours).len(), 3);
}
