// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use strims_base::time::Duration;

use super::*;
use crate::fmp4::parse::{media_duration, parse_init};
use crate::hls::{HlsMuxer, HlsMuxerConfig, HlsSegment};
use crate::media::{AccessUnit, CodecParams, DecoderConfig};

/// The HLS segments of `script`'s tags, each unit received at its tag's
/// time, finished at the end.
fn segments(script: &Script) -> Vec<HlsSegment> {
    let mut muxer = HlsMuxer::new(HlsMuxerConfig::default());
    let mut source = FlvSource::new();
    let mut out = Vec::new();
    for tag in &script.tags {
        mux_tags(std::slice::from_ref(tag), &mut source, &mut muxer);
        out.extend(std::iter::from_fn(|| muxer.take_segment()));
    }
    muxer.finish().unwrap();
    out.extend(std::iter::from_fn(|| muxer.take_segment()));
    out
}

/// The configs and units a script's tags yield.
fn source_events(script: &Script) -> (Vec<DecoderConfig>, Vec<(AccessUnit<'_>, TagKind)>) {
    let mut source = FlvSource::new();
    let (mut configs, mut units) = (Vec::new(), Vec::new());
    for tag in &script.tags {
        let event = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, received(tag)),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, received(tag)),
            TagKind::Metadata => continue,
        };
        match event.unwrap() {
            Some(SourceEvent::Config(c)) => configs.push(c),
            Some(SourceEvent::Unit(u)) => units.push((u, tag.kind)),
            None => panic!("a tag yielded nothing"),
        }
    }
    (configs, units)
}

#[test]
fn scripts_are_deterministic() {
    let config = ScriptConfig {
        b_frames: true,
        resize_at_gop: Some(2),
        ..ScriptConfig::default()
    };
    assert_eq!(Script::new(&config), Script::new(&config));
}

#[test]
fn the_source_takes_every_tag() {
    let script = Script::new(&ScriptConfig::default());
    assert_eq!(script.tags[0].kind, TagKind::Metadata);
    let (configs, units) = source_events(&script);
    let [video, audio] = &configs[..] else {
        panic!("two configs")
    };
    assert_eq!(video.codec, "avc1.64001f");
    assert!(matches!(
        video.params,
        CodecParams::Avc {
            width: 1280,
            height: 720,
            ..
        }
    ));
    assert_eq!(audio.codec, "mp4a.40.2");
    assert!(matches!(
        audio.params,
        CodecParams::Aac {
            sample_rate: 48_000,
            channels: 2,
            frame_samples: 1024,
            ..
        }
    ));
    let video_units: Vec<_> = units.iter().filter(|(_, k)| *k == TagKind::Video).collect();
    assert_eq!(video_units.len(), 300);
    assert_eq!(video_units.iter().filter(|(u, _)| u.keyframe).count(), 5);
    let audio_units = units.len() - video_units.len();
    assert_eq!(audio_units, 10 * 48_000 / 1024);
    // About 1 Mb/s of video.
    let bytes: usize = video_units.iter().map(|(u, _)| u.payload.len()).sum();
    assert!((1_200_000..=1_260_000).contains(&bytes), "{bytes}");
}

#[test]
fn tags_are_in_time_order_video_first() {
    let script = Script::new(&ScriptConfig::default());
    for pair in script.tags.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        assert!(a.timestamp <= b.timestamp);
        if a.timestamp == b.timestamp && a.kind == TagKind::Audio && a.body[1] == AAC_RAW {
            assert_ne!(
                b.kind,
                TagKind::Video,
                "audio before video at {}",
                a.timestamp
            );
        }
    }
}

#[test]
fn b_frames_reorder_presentation() {
    let script = Script::new(&ScriptConfig {
        b_frames: true,
        gop_frames: 6,
        audio: false,
        duration_secs: 1,
        ..ScriptConfig::default()
    });
    let (_, units) = source_events(&script);
    let ctos: Vec<i64> = units
        .iter()
        .take(7)
        .map(|(u, _)| (u.pts - u.dts) / 90)
        .collect();
    // I P B P B P(closing) | I: in ms at 30 fps.
    assert_eq!(ctos, [33, 67, 0, 66, 0, 34, 33]);
    let mut pts: Vec<i64> = units.iter().take(6).map(|(u, _)| u.pts).collect();
    pts.sort_unstable();
    pts.dedup();
    assert_eq!(pts.len(), 6);
}

#[test]
fn metadata_is_an_amf0_ecma_array() {
    let script = Script::new(&ScriptConfig::default());
    let body = &script.tags[0].body;
    assert_eq!(&body[..13], b"\x02\x00\x0aonMetaData");
    assert_eq!(body[13], 0x08);
    assert_eq!(
        u32::from_be_bytes(body[14..18].try_into().unwrap()) as usize,
        script.metadata.len()
    );
    assert!(body.ends_with(&[0, 0, 9]));
    let framerate = b"\x00\x09framerate\x00";
    let at = body
        .windows(framerate.len())
        .position(|w| w == framerate)
        .unwrap();
    let value = &body[at + framerate.len()..at + framerate.len() + 8];
    assert_eq!(f64::from_be_bytes(value.try_into().unwrap()), 30.0);
    assert!(
        script
            .metadata
            .contains(&("framerate".to_owned(), MetaValue::Number(30.0)))
    );
}

#[test]
fn a_scripts_segments_parse_and_its_resize_changes_the_init_once() {
    let script = Script::new(&ScriptConfig {
        resize_at_gop: Some(3),
        ..ScriptConfig::default()
    });
    let segments = segments(&script);
    assert!(segments.len() >= 5, "{}", segments.len());
    let mut total = Duration::ZERO;
    let mut inits: Vec<&[u8]> = Vec::new();
    for s in &segments {
        let info = parse_init(&s.init).unwrap();
        assert_eq!(info.tracks().len(), 2);
        assert_eq!(media_duration(&info, &s.media).unwrap(), s.duration);
        assert!(s.keyframe);
        total += s.duration;
        if inits.last() != Some(&&s.init[..]) {
            inits.push(&s.init);
        }
    }
    assert_eq!(inits.len(), 2);
    // Ten seconds of video: up to the last frame, which lasts as long as
    // the one before it.
    assert_eq!(total.as_millis(), 9_999);
}

#[test]
fn timestamps_may_wrap() {
    let config = ScriptConfig {
        start_ms: u32::MAX - 2_000,
        duration_secs: 4,
        ..ScriptConfig::default()
    };
    let script = Script::new(&config);
    assert!(script.tags.last().unwrap().timestamp < 2_000);
    let wrapped = segments(&script);
    let plain = segments(&Script::new(&ScriptConfig {
        start_ms: 0,
        ..config
    }));
    assert_eq!(wrapped.len(), plain.len());
    for (w, p) in wrapped.iter().zip(&plain) {
        assert_eq!(w.duration, p.duration);
    }
}

#[test]
fn aac_configs_signal_the_rate() {
    assert_eq!(aac_lc_asc(48_000), [0x11, 0x90]);
    assert_eq!(aac_lc_asc(44_100), [0x12, 0x10]);
    // An explicit 24-bit rate: 5 + 4 + 24 + 4 + 3 bits.
    let asc = aac_lc_asc(50_000);
    assert_eq!(asc.len(), 5);
    assert_eq!(crate::aac::parse_asc(&asc).unwrap().sample_rate, 50_000);
}

#[test]
fn media_without_publish_times_differs_only_in_when_units_arrived() {
    let script = Script::new(&ScriptConfig {
        duration_secs: 4,
        b_frames: true,
        ..ScriptConfig::default()
    });
    let on_time = segments(&script);
    // The same units, received a minute later.
    let mut muxer = HlsMuxer::new(HlsMuxerConfig::default());
    let mut source = FlvSource::new();
    let mut late = Vec::new();
    for tag in &script.tags {
        let at = Time::from_unix_millis(60_000 + i64::from(tag.timestamp));
        let event = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, at),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, at),
            TagKind::Metadata => continue,
        };
        match event.unwrap() {
            Some(SourceEvent::Config(c)) => muxer.set_config(&c).unwrap(),
            Some(SourceEvent::Unit(u)) => muxer.push(&u).unwrap(),
            None => {}
        }
        late.extend(std::iter::from_fn(|| muxer.take_segment()));
    }
    muxer.finish().unwrap();
    late.extend(std::iter::from_fn(|| muxer.take_segment()));
    assert_eq!(late.len(), on_time.len());
    for (a, b) in on_time.iter().zip(&late) {
        assert_ne!(a.media, b.media);
        assert_eq!(a.media.len(), b.media.len());
        let masked = media_without_publish_times(&a.media);
        assert_eq!(masked, media_without_publish_times(&b.media));
        let differing = a.media.iter().zip(&masked).filter(|(x, y)| x != y).count();
        assert!(differing > 0);
    }
}
