// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet};
use std::future::poll_fn;
use std::time::Duration as StdDuration;

use strims_base::time::Time;
use strims_video::codec::{
    Codec, CodecEvent, Input, Output, Refused, RenditionPlan, Transcode, TranscodePlan, Unit,
};
use strims_video::flv::{AUDIO_TRACK, VIDEO_TRACK};
use strims_video::flv_file::{FlvTagReader, TAG_AUDIO, TAG_VIDEO};
use strims_video::{DecoderConfig, FlvSource, SourceEvent};

use super::*;

const T0: i64 = 1_793_275_200_000;

#[test]
fn releases_from_7_1_and_development_builds_are_supported() {
    for v in [
        "7.1",
        "7.1.1",
        "n7.1",
        "8.0.1-3ubuntu2",
        "10.0",
        "N-118325-g1b2c3d4",
    ] {
        assert!(supported(v), "{v}");
    }
    for v in [
        "7.0.2",
        "n6.1.1",
        "6.1.1-3ubuntu5",
        "4.4.2-0ubuntu0.22.04.1",
        "7",
        "git",
    ] {
        assert!(!supported(v), "{v}");
    }
}

/// The ffmpeg on PATH, if it has libx264.
fn ffmpeg() -> Option<FfmpegConfig> {
    let config = FfmpegConfig {
        program: "ffmpeg".into(),
        settings: FfmpegSettings::default(),
    };
    let out = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
        .ok()?;
    if !String::from_utf8_lossy(&out.stdout).contains("libx264") {
        eprintln!("skipping: ffmpeg without libx264");
        return None;
    }
    Some(config)
}

#[tokio::test]
async fn the_probe_takes_a_recent_ffmpeg_with_its_encoder() {
    let Some(config) = ffmpeg() else {
        eprintln!("skipping: no ffmpeg on PATH");
        return;
    };
    let version = probe_ffmpeg(&config).await.unwrap();
    assert!(supported(&version), "{version}");
    let missing = FfmpegConfig {
        settings: FfmpegSettings {
            video_encoder: vec!["libnothing".into()],
            ..FfmpegSettings::default()
        },
        ..config
    };
    assert!(matches!(
        probe_ffmpeg(&missing).await,
        Err(ProbeError::NoEncoder { encoder, .. }) if encoder == "libnothing"
    ));
    let absent = FfmpegConfig {
        program: "/nonexistent/ffmpeg".into(),
        ..FfmpegConfig::default()
    };
    assert!(matches!(
        probe_ffmpeg(&absent).await,
        Err(ProbeError::Run { .. })
    ));
    let echo = FfmpegConfig {
        program: "echo".into(),
        ..FfmpegConfig::default()
    };
    assert!(matches!(
        probe_ffmpeg(&echo).await,
        Err(ProbeError::NotFfmpeg { .. })
    ));
}

/// A 3-second 1920x1080 60 fps H.264 stream with B-frames and keyframes
/// at 0, 1.3 and 2.9 s, with AAC, as an FLV source reads it: its configs
/// and units, each published at T0 plus its PTS in milliseconds.
fn source() -> (Vec<DecoderConfig>, Vec<Unit>) {
    let path = std::env::temp_dir().join(format!("strims-node-codec-{}.flv", std::process::id()));
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=1920x1080:rate=60",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000",
            "-t",
            "3",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-b:v",
            "4M",
            "-bf",
            "2",
            "-force_key_frames",
            "0,1.3,2.9",
            "-x264-params",
            "keyint=600:scenecut=0",
            "-c:a",
            "aac",
            "-f",
            "flv",
        ])
        .arg(&path)
        .status()
        .unwrap();
    assert!(status.success());
    let file = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let mut r = FlvTagReader::new(1 << 24);
    r.push(&file);
    let mut s = FlvSource::new();
    let (mut configs, mut units) = (Vec::new(), Vec::new());
    while let Some(tag) = r.next_tag().unwrap() {
        let e = match tag.kind {
            TAG_VIDEO => s.video(tag.timestamp, tag.body, Time::default()),
            TAG_AUDIO => s.audio(tag.timestamp, tag.body, Time::default()),
            _ => continue,
        };
        match e.unwrap() {
            Some(SourceEvent::Config(c)) => configs.push(c),
            Some(SourceEvent::Unit(u)) => {
                let mut u = Unit::of(&u);
                let ms = if u.track == VIDEO_TRACK {
                    u.pts / 90
                } else {
                    u.pts / 48
                };
                u.published = Time::from_unix_millis(T0 + ms);
                units.push(u);
            }
            None => {}
        }
    }
    (configs, units)
}

fn plan() -> TranscodePlan {
    TranscodePlan {
        renditions: vec![
            RenditionPlan {
                short_side: 720,
                max_frame_rate: 60,
                bitrate: 3_000_000,
            },
            RenditionPlan {
                short_side: 480,
                max_frame_rate: 30,
                bitrate: 1_200_000,
            },
        ],
    }
}

#[tokio::test(flavor = "current_thread")]
async fn ffmpeg_transcodes_renditions_on_the_streams_timeline() {
    let Some(config) = ffmpeg() else {
        return;
    };
    let (configs, units) = source();
    let host = NodeVideoHost::new(crate::net::TokioNet, Some(&config));
    let mut t = host.codec().unwrap().transcode(&plan()).unwrap();
    for c in &configs {
        t.send(Input::Config(c)).unwrap();
    }
    let mut events: Vec<CodecEvent> = Vec::new();
    tokio::time::timeout(StdDuration::from_secs(120), async {
        for u in &units {
            // a full queue waits for ffmpeg to take some of it
            while t.send(Input::Unit(&u.as_access_unit())) == Err(Refused::Full) {
                let e = poll_fn(|cx| t.poll_next(cx)).await;
                events.push(e.expect("ffmpeg runs").unwrap());
            }
        }
        t.finish();
        while let Some(e) = poll_fn(|cx| t.poll_next(cx)).await {
            events.push(e.unwrap());
        }
    })
    .await
    .expect("ffmpeg ends with its input");

    let source_video: BTreeMap<i64, &Unit> = units
        .iter()
        .filter(|u| u.track == VIDEO_TRACK)
        .map(|u| (u.pts, u))
        .collect();
    let source_keys: BTreeSet<i64> = source_video
        .values()
        .filter(|u| u.keyframe)
        .map(|u| u.pts)
        .collect();
    assert_eq!(source_keys.len(), 3);
    let source_audio: Vec<&[u8]> = units
        .iter()
        .filter(|u| u.track == AUDIO_TRACK)
        .map(|u| u.payload.as_slice())
        .collect();
    for (rendition, (size, frames)) in [((1280, 720), 180), ((854, 480), 90)]
        .into_iter()
        .enumerate()
    {
        let mine = || events.iter().filter(move |e| e.rendition == rendition);
        let sizes: Vec<_> = mine()
            .filter_map(|e| match &e.output {
                Output::Config(c) => c.params.picture_size(),
                Output::Unit(_) => None,
            })
            .collect();
        assert_eq!(sizes, [size], "rendition {rendition}");
        let video: Vec<&Unit> = mine()
            .filter_map(|e| match &e.output {
                Output::Unit(u) if u.track == VIDEO_TRACK => Some(u),
                _ => None,
            })
            .collect();
        assert_eq!(video.len(), frames, "rendition {rendition}");
        assert!(video.windows(2).all(|w| w[0].dts <= w[1].dts));
        let keys: BTreeSet<i64> = video.iter().filter(|u| u.keyframe).map(|u| u.pts).collect();
        assert_eq!(keys, source_keys, "rendition {rendition}'s keyframes");
        for u in &video {
            let s = source_video
                .get(&u.pts)
                .unwrap_or_else(|| panic!("rendition {rendition}: no source frame at {}", u.pts));
            assert_eq!(u.published, s.published);
        }
        let audio: Vec<&[u8]> = mine()
            .filter_map(|e| match &e.output {
                Output::Unit(u) if u.track == AUDIO_TRACK => Some(u.payload.as_slice()),
                _ => None,
            })
            .collect();
        assert_eq!(audio, source_audio, "rendition {rendition}'s audio");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn ffmpeg_makes_a_jpeg_thumbnail_of_a_keyframe() {
    let Some(config) = ffmpeg() else {
        return;
    };
    let (configs, units) = source();
    let codec = NodeVideoHost::new(crate::net::TokioNet, Some(&config))
        .codec()
        .unwrap();
    let key = units
        .iter()
        .rfind(|u| u.keyframe && u.track == VIDEO_TRACK)
        .unwrap();
    let jpeg = tokio::time::timeout(
        StdDuration::from_secs(30),
        codec.thumbnail(&configs, &key.as_access_unit(), 320, 32 * 1024),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(&jpeg[..2], [0xff, 0xd8]);
    assert_eq!(&jpeg[jpeg.len() - 2..], [0xff, 0xd9]);
    // a picture over the bound is refused
    let small = codec
        .thumbnail(&configs, &key.as_access_unit(), 320, 64)
        .await;
    assert!(small.is_err());
}

/// The CC1 captions ffmpeg's decoder reads of an FLV file of `configs`
/// and `units`: each caption's start in milliseconds and its text,
/// without markup.
fn decoded_captions(name: &str, configs: &[DecoderConfig], units: &[Unit]) -> Vec<(i64, String)> {
    use strims_video::flv_file::FlvWriter;
    let mut w = FlvWriter::new(0);
    let mut flv = Vec::new();
    w.header(configs, None, &mut flv);
    for c in configs {
        w.config(c, &mut flv).unwrap();
    }
    for u in units {
        let c = configs.iter().find(|c| c.track == u.track).unwrap();
        w.unit(c, &u.as_access_unit(), &mut flv).unwrap();
    }
    let path = std::env::temp_dir().join(format!("strims-node-{name}-{}.flv", std::process::id()));
    std::fs::write(&path, &flv).unwrap();
    let out = std::process::Command::new("ffmpeg")
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
            let (start, _) = lines.next()?.split_once(" --> ")?;
            // The decoder's SRT marks its font and position up.
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
            Some((ms(start), text.trim().to_owned()))
        })
        .collect()
}

/// A stream's CC1 captions, a field 1 pair every other frame of its 60
/// in presentation order across its B-frames, reach the renditions that
/// ffmpeg, writing none of its own, makes of it: the stage's frames carry
/// the tuples of the stream's frames since their last, so ffmpeg decodes
/// the same captions of the 30 fps rendition as of the stream, each
/// starting within a frame of it.
#[tokio::test(flavor = "current_thread")]
async fn ffmpegs_renditions_carry_the_streams_captions() {
    use std::collections::VecDeque;
    use strims_video::captions::{CaptionCarry, NalFormat};
    use strims_video::synth::captions::{field1, pop_on};

    let Some(config) = ffmpeg() else {
        return;
    };
    let (configs, mut units) = source();
    let video = configs.iter().find(|c| c.track == VIDEO_TRACK).unwrap();
    let nals = NalFormat::of(&video.params).unwrap();
    let mut order: Vec<usize> = (0..units.len())
        .filter(|&i| units[i].track == VIDEO_TRACK)
        .collect();
    order.sort_by_key(|&i| units[i].pts);
    let mut queue = VecDeque::new();
    for (k, &i) in order.iter().enumerate() {
        if k % 90 == 0 {
            queue.extend(pop_on(&format!("CAPTION {}", k / 90)));
        }
        if k % 2 == 1
            && let Some(p) = queue.pop_front()
        {
            let mut out = Vec::new();
            nals.with_cc_sei(&units[i].payload, &[field1(p)], &mut out)
                .unwrap();
            units[i].payload = out;
        }
    }

    let host = NodeVideoHost::new(crate::net::TokioNet, Some(&config));
    let mut t = host.codec().unwrap().transcode(&plan()).unwrap();
    for c in &configs {
        t.send(Input::Config(c)).unwrap();
    }
    let mut carry = CaptionCarry::new(2);
    let mut formats: Vec<Option<NalFormat>> = vec![None; 2];
    let mut events: Vec<CodecEvent> = Vec::new();
    let mut take = |mut e: CodecEvent, events: &mut Vec<CodecEvent>, carry: &mut CaptionCarry| {
        match &mut e.output {
            Output::Config(c) if c.track == VIDEO_TRACK => {
                formats[e.rendition] = NalFormat::of(&c.params);
            }
            Output::Unit(u) if u.track == VIDEO_TRACK => {
                let tuples = carry.take(e.rendition, u.pts, 90_000);
                let f = formats[e.rendition].unwrap();
                let mut out = Vec::new();
                f.with_cc_sei(&u.payload, &tuples, &mut out).unwrap();
                u.payload = out;
            }
            _ => {}
        }
        events.push(e);
    };
    tokio::time::timeout(StdDuration::from_secs(120), async {
        for u in &units {
            while t.send(Input::Unit(&u.as_access_unit())) == Err(Refused::Full) {
                let e = poll_fn(|cx| t.poll_next(cx)).await;
                take(e.expect("ffmpeg runs").unwrap(), &mut events, &mut carry);
            }
            if u.track == VIDEO_TRACK {
                let mut tuples = Vec::new();
                nals.cc_tuples(&u.payload, &mut tuples);
                carry.source(u.pts, 90_000, &tuples);
            }
        }
        t.finish();
        while let Some(e) = poll_fn(|cx| t.poll_next(cx)).await {
            take(e.unwrap(), &mut events, &mut carry);
        }
    })
    .await
    .expect("ffmpeg ends with its input");

    let stream = decoded_captions("cc-stream", &configs, &units);
    assert_eq!(
        stream.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>(),
        ["CAPTION 0", "CAPTION 1"]
    );
    let mut rendition_configs: Vec<DecoderConfig> = Vec::new();
    let mut rendition_units: Vec<Unit> = Vec::new();
    for e in events.iter().filter(|e| e.rendition == 1) {
        match &e.output {
            Output::Config(c) => rendition_configs.push(c.clone()),
            Output::Unit(u) => rendition_units.push(u.clone()),
        }
    }
    let rendition = decoded_captions("cc-480p30", &rendition_configs, &rendition_units);
    assert_eq!(rendition.len(), stream.len(), "{rendition:?} of {stream:?}");
    for ((at, text), (stream_at, stream_text)) in rendition.iter().zip(&stream) {
        assert_eq!(text, stream_text);
        assert!(
            (at - stream_at).abs() <= 34,
            "{text} at {at} ms, the stream's at {stream_at} ms"
        );
    }
}
