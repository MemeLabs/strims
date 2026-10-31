// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HEVC and AV1 from real encoders, through the FLV source and the fMP4
//! writers, read back by ffmpeg. Skipped when `ffmpeg` and `ffprobe`
//! aren't on PATH, and a case when its encoder isn't built in.
//!
//! - ffmpeg encodes a test pattern as HEVC (libx265, B-frames) or AV1
//!   (libsvtav1), with AAC, into Enhanced RTMP FLV. Its tags go through an
//!   [`FlvSource`], whose decoder configs become an init segment
//!   ([`write_init`]) and whose access units become fragments
//!   ([`FragmentWriter`]).
//! - ffprobe reads the file as the codec in an `hvc1` or `av01` sample
//!   entry at the encoded size, with AAC beside it, and ffmpeg decodes
//!   every frame without an error.
//!
//! `STRIMS_RENDITIONS_DUMP=<dir>` writes each case's file there, for
//! browsers to try.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::PathBuf;
use std::process::Command;

use strims_base::time::{Duration, Time};
use strims_video::fmp4::{FragmentWriter, write_init};
use strims_video::{DecoderConfig, FlvSource, SourceEvent, TrackKind};

const NOW: Time = Time::from_unix_millis(1_793_275_200_000);
const FRAMES: usize = 60;

fn tool(name: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(name).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The encoders ffmpeg has, or `None` without ffmpeg and ffprobe.
fn encoders() -> Option<String> {
    tool("ffprobe", &["-version"])?;
    tool("ffmpeg", &["-hide_banner", "-encoders"])
}

fn temp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("strims-video-{}-{name}", std::process::id()))
}

/// ffmpeg's FLV of a 30 fps 640x360 test pattern in `video` codec args,
/// with AAC.
fn encode(name: &str, video: &[&str]) -> Vec<u8> {
    let path = temp(&format!("{name}.flv"));
    let frames = FRAMES.to_string();
    let mut args = vec![
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=640x360:rate=30",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:sample_rate=48000",
        "-frames:v",
        &frames,
        "-pix_fmt",
        "yuv420p",
    ];
    args.extend_from_slice(video);
    args.extend_from_slice(&["-c:a", "aac", "-shortest", "-f", "flv"]);
    let out = path.to_str().unwrap();
    args.push(out);
    tool("ffmpeg", &args).expect("ffmpeg encodes the test pattern");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    bytes
}

/// The audio and video tags of an FLV file: video, time, body.
fn tags(flv: &[u8]) -> Vec<(bool, u32, &[u8])> {
    assert_eq!(&flv[..3], b"FLV");
    let mut at = u32::from_be_bytes(flv[5..9].try_into().unwrap()) as usize + 4;
    let mut tags = Vec::new();
    while at + 11 <= flv.len() {
        let kind = flv[at];
        let size = u32::from_be_bytes([0, flv[at + 1], flv[at + 2], flv[at + 3]]) as usize;
        let ts = u32::from_be_bytes([flv[at + 7], flv[at + 4], flv[at + 5], flv[at + 6]]);
        let body = &flv[at + 11..at + 11 + size];
        if kind == 8 || kind == 9 {
            tags.push((kind == 9, ts, body));
        }
        at += 11 + size + 4;
    }
    tags
}

/// The FLV's media as an fMP4 file: an init of its decoder configs, then
/// its fragments.
fn remux(flv: &[u8]) -> (Vec<u8>, Vec<DecoderConfig>) {
    let mut source = FlvSource::new();
    let mut configs: Vec<DecoderConfig> = Vec::new();
    let mut units = Vec::new();
    for (video, ts, body) in tags(flv) {
        let event = if video {
            source.video(ts, body, NOW)
        } else {
            source.audio(ts, body, NOW)
        };
        match event.unwrap() {
            Some(SourceEvent::Config(c)) => {
                assert!(
                    configs.iter().all(|o| o.track != c.track),
                    "one config each"
                );
                configs.push(c);
            }
            Some(SourceEvent::Unit(u)) => units.push(u),
            None => {}
        }
    }
    configs.sort_by_key(|c| c.track);
    let video = configs.iter().find(|c| c.kind == TrackKind::Video).unwrap();
    let first = units
        .iter()
        .find(|u| u.track == video.track)
        .copied()
        .unwrap();
    assert!(first.keyframe);
    let delay = Duration::from_millis((first.pts - first.dts) / 90);
    let mut out = Vec::new();
    write_init(&configs, delay, &mut out).unwrap();
    let mut w = FragmentWriter::new(Duration::from_millis(100));
    w.set_tracks(&configs, delay).unwrap();
    for u in &units {
        w.push(u, &mut out).unwrap();
    }
    w.flush(None, &mut out);
    (out, configs)
}

fn check(name: &str, video: &[&str], codec: (&str, &str), codec_string: &str) {
    let flv = encode(name, video);
    let (mp4, configs) = remux(&flv);
    assert_eq!(configs.len(), 2);
    assert!(
        configs[0].codec.starts_with(codec_string),
        "{}",
        configs[0].codec
    );
    assert_eq!(configs[0].params.picture_size(), Some((640, 360)));

    let path = temp(&format!("{name}.mp4"));
    std::fs::write(&path, &mp4).unwrap();
    #[allow(clippy::disallowed_methods)] // a test's dump directory
    if let Ok(dir) = std::env::var("STRIMS_RENDITIONS_DUMP") {
        std::fs::write(PathBuf::from(dir).join(format!("{name}.mp4")), &mp4).unwrap();
    }
    let file = path.to_str().unwrap();
    let streams = tool(
        "ffprobe",
        &[
            "-v",
            "error",
            "-count_frames",
            "-show_entries",
            "stream=codec_name,codec_tag_string,width,height,nb_read_frames",
            "-of",
            "csv=p=0",
            file,
        ],
    )
    .unwrap();
    let lines: Vec<&str> = streams.lines().collect();
    assert_eq!(
        lines[0],
        format!("{},{},640,360,{FRAMES}", codec.0, codec.1),
        "{streams}"
    );
    assert!(lines[1].starts_with("aac,mp4a,"), "{streams}");
    let out = Command::new("ffmpeg")
        .args(["-hide_banner", "-v", "error", "-i", file, "-f", "null", "-"])
        .output()
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(out.status.success());
    let errors = String::from_utf8_lossy(&out.stderr);
    assert!(errors.trim().is_empty(), "{name}: {errors}");
}

#[test]
fn ffmpeg_decodes_hevc_remuxed_from_enhanced_rtmp() {
    let Some(encoders) = encoders() else {
        eprintln!("skipping: `ffmpeg` or `ffprobe` is not on PATH");
        return;
    };
    if !encoders.contains("libx265") {
        eprintln!("skipping: ffmpeg has no libx265");
        return;
    }
    let x265 = [
        "-c:v",
        "libx265",
        "-preset",
        "ultrafast",
        "-x265-params",
        "log-level=error:keyint=30:bframes=3",
    ];
    check("hevc", &x265, ("hevc", "hvc1"), "hvc1.1.6.L");
}

#[test]
fn ffmpeg_decodes_av1_remuxed_from_enhanced_rtmp() {
    let Some(encoders) = encoders() else {
        eprintln!("skipping: `ffmpeg` or `ffprobe` is not on PATH");
        return;
    };
    if !encoders.contains("libsvtav1") {
        eprintln!("skipping: ffmpeg has no libsvtav1");
        return;
    }
    let svt = [
        "-c:v",
        "libsvtav1",
        "-preset",
        "12",
        "-g",
        "30",
        "-svtav1-params",
        "loglevel=0",
    ];
    check("av1", &svt, ("av1", "av01"), "av01.0.");
}
