// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::amf0::Value;
use crate::config::IngressSettings;

#[test]
fn an_ingest_uri_carries_its_options() {
    let o = ingress_options();
    let id = SwarmId::new(&[7; 32]);
    let uri = Uri::new(id.clone(), o.uri_options()).to_string();
    assert_eq!(
        uri,
        format!("magnet:?xt=urn:ppspp:{id}&x.im=2&x.hf=7&x.sa=2&x.cs=4096&x.ps=8&x.sc=16")
    );
    assert_eq!(o.live_window, 4 * 1024);
    assert_eq!(unit_ingress_options(&UnitSigning::LARGE), o);
    let small = unit_ingress_options(&UnitSigning::SMALL);
    assert_eq!(
        Uri::new(id.clone(), small.uri_options()).to_string(),
        format!("magnet:?xt=urn:ppspp:{id}&x.im=2&x.hf=7&x.sa=2&x.cs=4096&x.ps=1&x.sc=16")
    );
    assert_eq!(small.live_window, 4 * 1024);
}

#[test]
fn the_per_address_cap_counts_hosts() {
    for (peer, host) in [
        ("10.0.0.1:1935", "10.0.0.1"),
        ("[::1]:5000", "::1"),
        ("[fe80::1%eth0]:1", "fe80::1%eth0"),
        ("host.example:80", "host.example"),
        ("::1", "::1"),
        ("10.0.0.1", "10.0.0.1"),
        ("10.0.0.1:", "10.0.0.1:"),
        ("", ""),
    ] {
        assert_eq!(peer_host(peer), host, "{peer}");
    }
}

#[test]
fn the_rtmp_server_takes_the_ingress_bounds() {
    let s = IngressSettings::default().rtmp;
    assert_eq!(rtmp_config(&s), RtmpConfig::default());
    let c = rtmp_config(&RtmpSettings {
        max_queued_output: 1234,
        chunk_size: 512,
        ..s
    });
    assert_eq!((c.max_output, c.chunk_size), (1234, 512));
}

#[test]
fn the_publisher_hears_why_the_server_ended_its_stream() {
    let idle = End::Deadline(Deadline::Idle(Duration::from_secs(30)));
    assert_eq!(idle.to_string(), "no media for 30 s");
    assert!(idle.tells_peer() && idle.flushes());
    let media = End::Media("HEVC (hvc1) is not supported; send H.264 with AAC".into());
    assert_eq!(
        media.to_string(),
        "HEVC (hvc1) is not supported; send H.264 with AAC"
    );
    assert!(media.tells_peer());
    for quiet in [End::Unpublished, End::ServerClosed, End::PeerClosed] {
        assert!(!quiet.tells_peer(), "{quiet}");
    }
    assert!(!End::PeerClosed.flushes());
    assert!(!End::Io("reset".into()).flushes());
}

#[test]
fn a_live_snippet_keeps_the_channels_fields() {
    let c = VideoChannel {
        id: 3,
        directory_listing_snippet: Some(ListingSnippet {
            title: "title".into(),
            channel_name: "name".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let s = live_snippet(&c, 1_700_000_000);
    assert_eq!(
        (s.title.as_str(), s.channel_name.as_str()),
        ("title", "name")
    );
    assert_eq!((s.start_time, s.live), (1_700_000_000, true));
}

const T0: Time = Time::from_unix_millis(1_700_000_000_000);
const WAIT: Duration = Duration::from_secs(1);

/// The metadata of an `onMetaData` of `fields`.
fn metadata(fields: &[(&'static str, f64)]) -> Metadata {
    let v = Value::EcmaArray(fields.iter().map(|&(k, n)| (k, Value::Number(n))).collect());
    Metadata::from_value(&v).unwrap_or_default()
}

/// OBS's fields, at `video` and `audio` kbit/s.
fn obs(video: f64, audio: f64) -> Metadata {
    metadata(&[
        ("width", 1280.0),
        ("height", 720.0),
        ("videodatarate", video),
        ("framerate", 30.0),
        ("videocodecid", 7.0),
        ("audiodatarate", audio),
        ("audiosamplerate", 48000.0),
        ("audiocodecid", 10.0),
    ])
}

/// A choice that took `m`, settled or not at its start.
fn after(m: &Metadata) -> SignChoice {
    let mut c = SignChoice::new(WAIT);
    c.metadata(m);
    c
}

#[test]
fn metadata_stating_its_bitrates_settles_the_signing_at_once() {
    for (video, audio, signing) in [
        (2500.0, 160.0, UnitSigning::LARGE),
        (6000.0, 320.0, UnitSigning::LARGE),
        (650.0, 128.0, UnitSigning::LARGE),
        (400.0, 96.0, UnitSigning::SMALL),
        (600.0, 128.0, UnitSigning::SMALL),
    ] {
        let c = after(&obs(video, audio));
        assert_eq!(c.settled(T0, 0), Some(signing), "{video} + {audio}");
        assert_eq!(c.deadline(), None);
    }
}

/// ffmpeg states a video bitrate of 0 for a CRF stream and an audio one
/// for its AAC: the audio's alone is no nominal bitrate of the stream.
#[test]
fn a_stream_with_video_of_no_stated_bitrate_takes_eight_chunk_units() {
    let crf = metadata(&[
        ("width", 1280.0),
        ("height", 720.0),
        ("videodatarate", 0.0),
        ("videocodecid", 7.0),
        ("audiodatarate", 125.0),
        ("audiocodecid", 10.0),
    ]);
    assert_eq!(after(&crf).settled(T0, 0), Some(UnitSigning::LARGE));
    // Video at a stated rate, and audio named without one: the rates
    // aren't the stream's, unless they reach the bound alone.
    let mut no_audio_rate = obs(400.0, 0.0);
    no_audio_rate.audiodatarate = None;
    assert_eq!(
        after(&no_audio_rate).settled(T0, 0),
        Some(UnitSigning::LARGE)
    );
    let mut high_video = obs(800.0, 0.0);
    high_video.audiodatarate = None;
    assert_eq!(after(&high_video).settled(T0, 0), Some(UnitSigning::LARGE));
}

#[test]
fn audio_alone_named_by_metadata_takes_single_chunk_units() {
    for m in [
        metadata(&[("audiodatarate", 125.0), ("audiocodecid", 10.0)]),
        metadata(&[("audiodatarate", 0.0), ("audiocodecid", 10.0)]),
        metadata(&[("audiosamplerate", 44100.0)]),
    ] {
        assert_eq!(after(&m).settled(T0, 0), Some(UnitSigning::SMALL), "{m:?}");
    }
    // Audio stated at a high rate is a high nominal bitrate.
    let loud = metadata(&[("audiodatarate", 1000.0), ("audiocodecid", 10.0)]);
    assert_eq!(after(&loud).settled(T0, 0), Some(UnitSigning::LARGE));
}

#[test]
fn rates_not_finite_and_positive_are_no_rates() {
    for bad in [f64::NAN, -400.0, f64::INFINITY, f64::NEG_INFINITY, 1e-9] {
        let video = metadata(&[("videodatarate", bad), ("audiodatarate", 96.0)]);
        assert_eq!(
            after(&video).settled(T0, 0),
            Some(UnitSigning::LARGE),
            "{bad}"
        );
        let audio = metadata(&[("audiodatarate", bad)]);
        assert_eq!(
            after(&audio).settled(T0, 0),
            Some(UnitSigning::SMALL),
            "{bad}"
        );
    }
    // A rate past u64 saturates.
    let huge = metadata(&[("videodatarate", 1e300), ("audiodatarate", 96.0)]);
    assert_eq!(after(&huge).settled(T0, 0), Some(UnitSigning::LARGE));
}

#[test]
fn metadata_naming_no_track_leaves_the_signing_to_the_decoder_configs() {
    let mut c = after(&metadata(&[("duration", 0.0)]));
    c.metadata(&Metadata {
        encoder: Some("obs-output module".into()),
        ..Metadata::default()
    });
    assert_eq!(c.settled(T0, 0), None);
    c.config(TrackKind::Video);
    assert_eq!(c.settled(T0, 0), Some(UnitSigning::LARGE));
}

#[test]
fn without_metadata_a_video_config_settles_the_signing_at_once() {
    let mut c = SignChoice::new(WAIT);
    assert_eq!(c.settled(T0, 0), None);
    c.config(TrackKind::Audio);
    c.config(TrackKind::Video);
    assert_eq!(c.settled(T0, 0), Some(UnitSigning::LARGE));
}

#[test]
fn audio_alone_without_metadata_waits_from_its_first_unit() {
    let mut c = SignChoice::new(WAIT);
    c.config(TrackKind::Audio);
    // Nothing waits before the first unit.
    let later = T0 + Duration::from_secs(5);
    assert_eq!((c.deadline(), c.settled(later, 0)), (None, None));
    c.unit(later);
    c.unit(later + Duration::from_millis(500));
    assert_eq!(c.deadline(), Some(later + WAIT));
    assert_eq!(c.settled(later + Duration::from_millis(999), 4096), None);
    assert_eq!(c.settled(later + WAIT, 4096), Some(UnitSigning::SMALL));
}

#[test]
fn a_video_config_or_metadata_within_the_wait_settles_it() {
    let mut waiting = SignChoice::new(WAIT);
    waiting.config(TrackKind::Audio);
    waiting.unit(T0);
    let at = T0 + Duration::from_millis(200);
    assert_eq!(waiting.settled(at, 0), None);

    let mut video = waiting.clone();
    video.config(TrackKind::Video);
    assert_eq!(video.settled(at, 0), Some(UnitSigning::LARGE));

    let mut stated = waiting.clone();
    stated.metadata(&obs(2500.0, 160.0));
    assert_eq!(stated.settled(at, 0), Some(UnitSigning::LARGE));
    let mut stated = waiting;
    stated.metadata(&metadata(&[("audiodatarate", 128.0)]));
    assert_eq!(stated.settled(at, 0), Some(UnitSigning::SMALL));
}

#[test]
fn output_held_past_its_bound_settles_on_what_is_known() {
    let mut c = SignChoice::new(WAIT);
    c.config(TrackKind::Audio);
    c.unit(T0);
    assert_eq!(c.settled(T0, MAX_HELD - 1), None);
    assert_eq!(c.settled(T0, MAX_HELD), Some(UnitSigning::SMALL));
}

#[test]
fn a_stream_of_no_known_track_takes_eight_chunk_units() {
    let mut c = SignChoice::new(WAIT);
    c.unit(T0);
    assert_eq!(c.settled(T0 + WAIT, 0), Some(UnitSigning::LARGE));
    c.metadata(&metadata(&[("duration", 0.0)]));
    assert_eq!(c.choose(), UnitSigning::LARGE);
}
