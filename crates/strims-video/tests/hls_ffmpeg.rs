// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HLS segments against ffmpeg: real encoders' streams cut by the HLS
//! muxer into a playlist, its inits and segments, and played back by
//! ffmpeg's HLS demuxer. Skipped when `ffmpeg` and `ffprobe` aren't on
//! PATH or lack libx264 or libopus.
//!
//! libx264 (High profile, B-frames, 1-second GOPs) with AAC-LC from
//! ffmpeg's FLV muxer, or with libopus in its CELT mode in place of the
//! AAC, its audio missing for 0.4 s inside a segment and for 0.2 s across
//! the cut at 3 s: ffprobe reads H.264 and the audio codec at 48 kHz
//! stereo, both tracks decode without an error, the audio lasts as long
//! as the video, and it is silent where it was missing and plays the tone
//! elsewhere.
//!
//! Each stream's playlist and files stay in `CARGO_TARGET_TMPDIR`
//! (`hls-ffmpeg-aac`, `hls-ffmpeg-opus`), for other players to try.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use strims_base::time::{Duration, Time};
use strims_video::flv::AUDIO_TRACK;
use strims_video::hls::{HlsMuxer, HlsMuxerConfig, Playlist, PlaylistConfig};
use strims_video::opus::packet_samples;
use strims_video::{
    AccessUnit, CodecParams, DecoderConfig, FlvSource, Muxer, SourceEvent, TrackKind,
};

const EPOCH_MS: i64 = 1_793_275_200_000;
const RATE: i64 = 48_000;
/// The stream's length in seconds.
const SECS: i64 = 6;
/// The holes in the audio, in 48 kHz samples: one inside the segment of
/// the second second, one across the cut at 3 s.
const HOLES: [(i64, i64); 2] = [
    (RATE * 13 / 10, RATE * 17 / 10),
    (RATE * 29 / 10, RATE * 31 / 10),
];

fn tools_with(encoder: &str) -> bool {
    let out = Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output();
    let found = out.is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains(encoder))
        && Command::new("ffprobe").arg("-version").output().is_ok();
    if !found {
        eprintln!("skipped: no ffmpeg and ffprobe with {encoder} on PATH");
    }
    found
}

/// Runs `tool` with `args`, answering its output; panics on its errors.
fn run(tool: &str, args: &[&str]) -> Vec<u8> {
    let out = Command::new(tool)
        .args(["-hide_banner", "-v", "error"])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let errors = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success() && errors.trim().is_empty(),
        "{tool} {args:?}: {errors}"
    );
    out.stdout
}

#[derive(Debug, Clone)]
enum Event {
    Config(DecoderConfig),
    Unit {
        config: usize,
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

/// An FLV file's configs and units, each unit naming its config's
/// position among the events' configs.
fn flv_events(flv: &[u8]) -> Vec<Event> {
    let mut source = FlvSource::new();
    let mut out = Vec::new();
    let mut configs: Vec<DecoderConfig> = Vec::new();
    for (kind, ts, body) in flv_tags(flv) {
        let at = Time::from_unix_millis(EPOCH_MS + i64::from(ts));
        let event = match kind {
            9 => source.video(ts, &body, at),
            8 => source.audio(ts, &body, at),
            _ => continue,
        };
        match event.unwrap() {
            Some(SourceEvent::Config(c)) => {
                configs.push(c.clone());
                out.push(Event::Config(c));
            }
            Some(SourceEvent::Unit(u)) => out.push(Event::Unit {
                config: configs.iter().rposition(|c| c.track == u.track).unwrap(),
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

/// A test pattern as H.264 with a 440 Hz tone as AAC, in FLV.
fn flv() -> Vec<u8> {
    run(
        "ffmpeg",
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc2=size=640x360:rate=30:duration={SECS}"),
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=440:sample_rate=48000:duration={SECS}"),
            "-c:v",
            "libx264",
            "-profile:v",
            "high",
            "-bf",
            "2",
            "-g",
            "30",
            "-keyint_min",
            "30",
            "-sc_threshold",
            "0",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-ac",
            "2",
            "-b:a",
            "128k",
            "-f",
            "flv",
            "pipe:1",
        ],
    )
}

/// The same tone as Opus, in 20 ms CELT packets: its OpusHead and
/// packets.
fn opus() -> (Vec<u8>, Vec<Vec<u8>>) {
    let ogg = run(
        "ffmpeg",
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=440:sample_rate=48000:duration={SECS}"),
            "-ac",
            "2",
            "-c:a",
            "libopus",
            "-application",
            "audio",
            "-b:a",
            "96k",
            "-frame_duration",
            "20",
            "-f",
            "ogg",
            "pipe:1",
        ],
    );
    let mut packets = ogg_packets(&ogg);
    let head = packets.remove(0);
    // The comment header.
    packets.remove(0);
    (head, packets)
}

/// The FLV's configs, then its units in time order, those of the audio
/// in `HOLES` left out; with `opus`, its Opus packets in place of its AAC.
fn stream(flv: &[u8], opus: Option<(Vec<u8>, Vec<Vec<u8>>)>) -> Vec<Event> {
    let mut configs: Vec<DecoderConfig> = Vec::new();
    let (mut video, mut audio) = (Vec::new(), Vec::new());
    for e in flv_events(flv) {
        match e {
            Event::Config(c) => configs.push(c),
            Event::Unit { config, .. } if configs[config].kind == TrackKind::Audio => audio.push(e),
            u => video.push(u),
        }
    }
    if let Some((head, packets)) = opus {
        configs.retain(|c| c.kind == TrackKind::Video);
        let video_config = configs.len() - 1;
        for u in &mut video {
            if let Event::Unit { config, .. } = u {
                *config = video_config;
            }
        }
        configs.push(DecoderConfig {
            track: AUDIO_TRACK,
            kind: TrackKind::Audio,
            codec: "Opus".into(),
            timescale: 48_000,
            params: CodecParams::Opus {
                head,
                channels: 2,
                pre_skip: 312,
            },
        });
        let mut dts = 0;
        audio = packets
            .into_iter()
            .map(|p| {
                let u = Event::Unit {
                    config: configs.len() - 1,
                    dts,
                    pts: dts,
                    keyframe: true,
                    payload: p,
                };
                if let Event::Unit { payload, .. } = &u {
                    dts += i64::from(packet_samples(payload).unwrap());
                }
                u
            })
            .collect();
    }
    let missing = |dts: i64| HOLES.iter().any(|(from, to)| (*from..*to).contains(&dts));
    audio.retain(|e| !matches!(e, Event::Unit { dts, .. } if missing(*dts)));
    let time = |e: &Event| match e {
        Event::Unit { config, dts, .. } => {
            i128::from(*dts) * 1_000_000 / i128::from(configs[*config].timescale)
        }
        Event::Config(_) => 0,
    };
    let mut out: Vec<Event> = configs.iter().cloned().map(Event::Config).collect();
    let (mut v, mut a) = (video.into_iter().peekable(), audio.into_iter().peekable());
    loop {
        let take_video = match (v.peek(), a.peek()) {
            (Some(x), Some(y)) => time(x) <= time(y),
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => return out,
        };
        out.extend(if take_video { v.next() } else { a.next() });
    }
}

/// Cuts `events` into segments and writes them to `dir` with a playlist
/// listing them all, which ends: `index.m3u8`, `init-<id>.mp4` and
/// `<seq>.m4s`.
fn write_hls(events: &[Event], dir: &Path) -> Playlist {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    let mut muxer = HlsMuxer::new(HlsMuxerConfig::default());
    let mut playlist = Playlist::new(PlaylistConfig {
        listed: Duration::from_secs(60),
        retained: Duration::from_secs(60),
        ..PlaylistConfig::default()
    });
    let mut configs = Vec::new();
    let take = |muxer: &mut HlsMuxer, playlist: &mut Playlist| {
        while let Some(s) = muxer.take_segment() {
            if s.discontinuity {
                playlist.discontinuity();
            }
            playlist.push_segment(&s.init, &s.media, s.duration);
        }
    };
    for e in events {
        match e {
            Event::Config(c) => {
                muxer.set_config(c).unwrap();
                configs.push(c.clone());
            }
            Event::Unit {
                config,
                dts,
                pts,
                keyframe,
                payload,
            } => {
                let c: &DecoderConfig = &configs[*config];
                let ms = dts * 1000 / i64::from(c.timescale);
                muxer
                    .push(&AccessUnit {
                        track: c.track,
                        dts: *dts,
                        pts: *pts,
                        keyframe: *keyframe,
                        payload,
                        published: Time::from_unix_millis(EPOCH_MS + ms),
                    })
                    .unwrap();
            }
        }
        take(&mut muxer, &mut playlist);
    }
    muxer.finish().unwrap();
    take(&mut muxer, &mut playlist);
    playlist.end();
    let text = playlist.render();
    for line in text.lines() {
        if let Some(seq) = line.strip_suffix(".m4s") {
            let seq: u64 = seq.parse().unwrap();
            std::fs::write(dir.join(line), &*playlist.segment(seq).unwrap()).unwrap();
        } else if let Some(map) = line.strip_prefix("#EXT-X-MAP:URI=\"") {
            let name = map.trim_end_matches('"');
            let id: u64 = name
                .strip_prefix("init-")
                .and_then(|n| n.strip_suffix(".mp4"))
                .unwrap()
                .parse()
                .unwrap();
            std::fs::write(dir.join(name), &*playlist.init(id).unwrap()).unwrap();
        }
    }
    std::fs::write(dir.join("index.m3u8"), &text).unwrap();
    playlist
}

/// The playlist's audio, decoded by ffmpeg, mono at 48 kHz.
fn decoded_audio(playlist: &Path) -> Vec<i16> {
    let file = playlist.to_str().unwrap();
    let pcm = run(
        "ffmpeg",
        &[
            "-i", file, "-map", "0:a:0", "-ac", "1", "-ar", "48000", "-f", "s16le", "pipe:1",
        ],
    );
    pcm.as_chunks::<2>()
        .0
        .iter()
        .map(|b| i16::from_le_bytes(*b))
        .collect()
}

/// The runs of `pcm` quieter than `floor` for at least 10 ms: their
/// first sample and length.
fn quiet_runs(pcm: &[i16], floor: i16) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut start = None;
    for (i, s) in pcm.iter().chain([&i16::MAX]).enumerate() {
        match (s.unsigned_abs() < floor.unsigned_abs(), start) {
            (true, None) => start = Some(i),
            (false, Some(from)) => {
                if i - from >= 480 {
                    runs.push((from, i - from));
                }
                start = None;
            }
            _ => {}
        }
    }
    runs
}

fn check(name: &str, codec: &str, events: &[Event]) {
    let dir: PathBuf = [env!("CARGO_TARGET_TMPDIR"), name].iter().collect();
    let playlist = write_hls(events, &dir);
    assert!(playlist.ended());
    let index = dir.join("index.m3u8");
    let file = index.to_str().unwrap();
    let streams = run(
        "ffprobe",
        &[
            "-show_entries",
            "stream=codec_name,sample_rate,channels",
            "-of",
            "csv=p=0",
            file,
        ],
    );
    let streams = String::from_utf8(streams).unwrap();
    // The HLS demuxer's program lists its streams again.
    let mut streams: Vec<&str> = streams
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    streams.dedup();
    streams.truncate(2);
    assert_eq!(streams, ["h264", &format!("{codec},48000,2")], "{name}");
    run("ffmpeg", &["-i", file, "-map", "0:v:0", "-f", "null", "-"]);

    let pcm = decoded_audio(&index);
    let secs = pcm.len() as f64 / 48_000.0;
    assert!(
        (secs - SECS as f64).abs() < 0.1,
        "{name}: {secs} s of audio"
    );
    // The tone's peak is about 4100; silence decodes to almost nothing.
    let quiet = quiet_runs(&pcm, 40);
    let lens: Vec<usize> = quiet.iter().map(|(_, n)| *n).collect();
    for (from, to) in HOLES {
        let want = (to - from) as usize;
        assert!(
            lens.iter().any(|n| n.abs_diff(want) <= 2048),
            "{name}: quiet runs {quiet:?}, want one of {want}"
        );
    }
    assert!(
        quiet.len() <= HOLES.len() + 1,
        "{name}: quiet runs {quiet:?}"
    );
}

#[test]
fn aac_hls_with_holes_decodes_in_ffmpeg() {
    if !tools_with("libx264") {
        return;
    }
    let flv = flv();
    check("hls-ffmpeg-aac", "aac", &stream(&flv, None));
}

#[test]
fn opus_hls_with_holes_decodes_in_ffmpeg() {
    if !tools_with("libx264") || !tools_with("libopus") {
        return;
    }
    let flv = flv();
    check("hls-ffmpeg-opus", "opus", &stream(&flv, Some(opus())));
}
