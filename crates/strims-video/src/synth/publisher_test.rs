// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use strims_base::rng::det_rng;

use super::*;
use crate::amf0::Metadata;
use crate::rtmp::test_util::{Answer, Ev, pump};
use crate::rtmp::{Phase, RtmpServer};

/// Tag bodies of every size class: inside one chunk, across several, and
/// past the server's chunk size.
fn script() -> Vec<(bool, u32, Vec<u8>)> {
    let mut tags = Vec::new();
    let body = |n: usize, seed: u32| -> Vec<u8> {
        (0..n)
            .map(|i| (i as u32).wrapping_mul(2_654_435_761).wrapping_add(seed) as u8)
            .collect()
    };
    tags.push((true, 0, body(40, 1)));
    tags.push((false, 0, body(7, 2)));
    for i in 1..40u32 {
        let size = [10, 127, 128, 129, 4095, 4096, 4097, 20_000, 300_000][i as usize % 9];
        tags.push((true, i * 33, body(size, i)));
        tags.push((false, i * 21, body(6 + i as usize, i)));
        // Same size and spacing: format 3 headers.
        tags.push((false, i * 21 + 21, body(6 + i as usize, i + 1)));
    }
    // Extended timestamps, and a timestamp going back.
    tags.push((true, 0x00ff_ffff, body(5000, 90)));
    tags.push((true, 0x0100_0021, body(5000, 91)));
    tags.push((true, 0x0200_0042, body(5000, 92)));
    tags.push((false, 0x0100_0000, body(9, 93)));
    tags.push((true, 10, body(9000, 94)));
    tags
}

fn session(config: PublisherConfig, seed: u64) {
    let digest = config.digest_handshake;
    let mut rng = det_rng(seed);
    let mut p = Publisher::new(config, &mut rng);
    let mut server = RtmpServer::new(&RtmpConfig::default(), &mut rng);
    let mut sent = Vec::new();
    let events = pump(&mut p, &mut server, Answer::Accept, &mut sent).unwrap();
    assert_eq!(
        events,
        [Ev::Publish {
            app: "live".to_owned(),
            name: "key".to_owned(),
        }]
    );
    assert!(p.handshake_done());
    assert_eq!(server.digest_handshake(), digest);
    assert_eq!(p.server_digest(), digest.then_some(true));
    assert!(p.connected());
    assert!(p.publish_accepted());
    assert_eq!(p.refused(), None);
    assert_eq!(p.stream_id(), 1);
    assert_eq!(server.phase(), Phase::Publishing);
    assert_eq!(server.tc_url(), Some("rtmp://localhost/live"));

    p.send_metadata(&[
        ("width", Value::Number(1280.0)),
        ("height", Value::Number(720.0)),
        ("framerate", Value::Number(30.0)),
        ("videocodecid", Value::Number(7.0)),
        ("audiocodecid", Value::Number(10.0)),
        ("audiosamplerate", Value::Number(48000.0)),
        ("encoder", Value::String("synth")),
    ])
    .unwrap();
    let mut expect = vec![Ev::Metadata(Metadata {
        width: Some(1280.0),
        height: Some(720.0),
        framerate: Some(30.0),
        videocodecid: Some(7.0),
        audiocodecid: Some(10.0),
        audiosamplerate: Some(48000.0),
        encoder: Some("synth".to_owned()),
    })];
    let mut events = Vec::new();
    for (video, ts, body) in script() {
        if video {
            p.send_video(ts, &body).unwrap();
            expect.push(Ev::Video(ts, body));
        } else {
            p.send_audio(ts, &body).unwrap();
            expect.push(Ev::Audio(ts, body));
        }
        // Drain now and then, so chunks of both streams interleave in
        // the server's input.
        if p.queued() > 100_000 {
            events.extend(pump(&mut p, &mut server, Answer::Accept, &mut sent).unwrap());
        }
    }
    p.unpublish().unwrap();
    expect.push(Ev::Unpublish);
    events.extend(pump(&mut p, &mut server, Answer::Accept, &mut sent).unwrap());
    assert!(events == expect, "events differ");
    assert_eq!(server.phase(), Phase::Connecting);
}

#[test]
fn digest_session_carries_the_stream_byte_identically() {
    session(PublisherConfig::new("live", "key"), 1);
}

#[test]
fn plain_session_carries_the_stream_byte_identically() {
    session(
        PublisherConfig {
            digest_handshake: false,
            ..PublisherConfig::new("live", "key")
        },
        2,
    );
}

#[test]
fn sessions_at_other_chunk_sizes_carry_the_stream() {
    for chunk_size in [1, 128, 60_000, 1 << 20] {
        session(
            PublisherConfig {
                chunk_size,
                name: "key?token=abc".to_owned(),
                ..PublisherConfig::new("live", "key")
            },
            u64::from(chunk_size),
        );
    }
}

#[test]
fn publisher_reports_a_refusal() {
    let mut rng = det_rng(3);
    let mut p = Publisher::new(PublisherConfig::new("live", "bad"), &mut rng);
    let mut server = RtmpServer::new(&RtmpConfig::default(), &mut rng);
    pump(
        &mut p,
        &mut server,
        Answer::Refuse("NetStream.Publish.BadName"),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(p.connected());
    assert!(!p.publish_accepted());
    assert_eq!(p.refused(), Some(("NetStream.Publish.BadName", "refused")));
    assert_eq!(server.phase(), Phase::Closed);
}

#[test]
fn same_seed_same_bytes() {
    let run = || {
        let mut rng = det_rng(4);
        let mut p = Publisher::new(PublisherConfig::new("live", "key"), &mut rng);
        let mut server = RtmpServer::new(&RtmpConfig::default(), &mut rng);
        let mut sent = Vec::new();
        pump(&mut p, &mut server, Answer::Accept, &mut sent).unwrap();
        p.send_video(0, &[1, 2, 3]).unwrap();
        pump(&mut p, &mut server, Answer::Accept, &mut sent).unwrap();
        sent
    };
    assert_eq!(run(), run());
}
