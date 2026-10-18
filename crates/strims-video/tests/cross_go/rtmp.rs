// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! joy5's RTMP client (`go run -tags golden ./crates/strims-video/golden/rtmp
//! <addr> <name> <script>`) publishes a scripted stream to [`RtmpServer`]
//! over TCP on 127.0.0.1. The server must yield the publish, the metadata
//! and every tag body and timestamp as joy5 sent them, then the end of the
//! publish; joy5's handshake must accept the server's S1 digest, and S2
//! must carry the response digest. Skipped when `go` isn't on PATH.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    // The test runs a Go subprocess and serves it from a thread.
    clippy::disallowed_methods
)]

use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use strims_base::rng::det_rng;
use strims_video::amf0::{self, Metadata, Value};
use strims_video::rtmp::{Phase, RtmpConfig, RtmpEvent, RtmpServer};

use crate::{go_available, repo_root};

#[derive(Debug, Clone, PartialEq)]
enum Ev {
    Publish { app: String, name: String },
    Metadata(Metadata),
    Video(u32, Vec<u8>),
    Audio(u32, Vec<u8>),
    Unpublish,
}

fn pattern(n: usize, seed: u32) -> Vec<u8> {
    (0..n)
        .map(|i| (i as u32).wrapping_mul(2_654_435_761).wrapping_add(seed) as u8)
        .collect()
}

/// The script's tags: legacy AVC and AAC bodies (sequence headers, then
/// frames with positive and negative composition times), sizes past
/// joy5's 64 KiB chunks, and timestamps past 24 bits.
fn tags() -> Vec<(bool, u32, Vec<u8>)> {
    let mut tags = vec![
        (true, 0, [&[0x17, 0, 0, 0, 0][..], &pattern(40, 1)].concat()),
        (false, 0, vec![0xaf, 0, 0x12, 0x10]),
    ];
    let mut ts = 0u32;
    for i in 0..60u32 {
        let key = i % 20 == 0;
        let size = match i % 20 {
            0 => 150_000,
            7 => 70_000,
            _ => 1000 + 37 * i as usize,
        };
        let cts = [0i32, 66, -33, 0x7f_ffff, -0x80_0000][i as usize % 5];
        let mut video = vec![if key { 0x17 } else { 0x27 }, 1];
        video.extend(&cts.to_be_bytes()[1..]);
        video.extend(pattern(size, i));
        tags.push((true, ts, video));
        tags.push((
            false,
            ts + 5,
            [&[0xaf, 1][..], &pattern(200 + i as usize, i)].concat(),
        ));
        ts += 33;
        if i == 40 {
            ts = 0x00ff_fff0;
        }
    }
    tags
}

const METADATA: &str = "metadata width=1280 height=720 framerate=30 videocodecid=7 \
                        audiocodecid=10 audiosamplerate=48000 encoder=joy5-golden";

fn metadata() -> Metadata {
    Metadata {
        width: Some(1280.0),
        height: Some(720.0),
        framerate: Some(30.0),
        videocodecid: Some(7.0),
        audiocodecid: Some(10.0),
        audiosamplerate: Some(48000.0),
        encoder: Some("joy5-golden".to_owned()),
    }
}

/// Serves one connection until it closes; returns the events, the final
/// phase and whether the handshake took the digest scheme.
fn serve(listener: TcpListener) -> (Vec<Ev>, Phase, bool) {
    let (mut stream, _) = listener.accept().unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .unwrap();
    let mut server = RtmpServer::new(&RtmpConfig::default(), &mut det_rng(1));
    let mut buf = vec![0; 64 << 10];
    let mut out = vec![0; 64 << 10];
    let mut events = Vec::new();
    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let mut input = &buf[..n];
        loop {
            let used = server.handle_input(input).unwrap();
            input = &input[used..];
            let mut yielded = false;
            while let Some(e) = server.poll_event() {
                yielded = true;
                let e = match e {
                    RtmpEvent::Publish { app, name } => Ev::Publish { app, name },
                    RtmpEvent::Metadata(m) => Ev::Metadata(m),
                    RtmpEvent::Video { timestamp, data } => Ev::Video(timestamp, data.to_vec()),
                    RtmpEvent::Audio { timestamp, data } => Ev::Audio(timestamp, data.to_vec()),
                    RtmpEvent::Unpublish => Ev::Unpublish,
                };
                if matches!(e, Ev::Publish { .. }) {
                    server.accept_publish();
                }
                events.push(e);
            }
            if input.is_empty() && !yielded {
                break;
            }
        }
        loop {
            let m = server.poll_transmit(&mut out);
            if m == 0 || stream.write_all(&out[..m]).is_err() {
                break;
            }
        }
    }
    (events, server.phase(), server.digest_handshake())
}

#[test]
fn joy5_publishes_to_the_server() {
    if !go_available() {
        return;
    }
    let tags = tags();
    let mut script = format!("{METADATA}\n");
    for (video, ts, body) in &tags {
        let kind = if *video { "video" } else { "audio" };
        writeln!(script, "{kind} {ts} {}", hex::encode(body)).unwrap();
    }
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rtmp-cross-go-script.txt");
    std::fs::write(&path, &script).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let server = std::thread::spawn(move || serve(listener));
    let go = Command::new("go")
        .args([
            "run",
            "-tags",
            "golden",
            "./crates/strims-video/golden/rtmp",
        ])
        .args([addr.as_str(), "key"])
        .arg(&path)
        .current_dir(repo_root())
        .output()
        .expect("run go");
    let stdout = String::from_utf8(go.stdout).unwrap();
    assert!(
        go.status.success(),
        "go failed: {}",
        String::from_utf8_lossy(&go.stderr)
    );
    let (events, phase, digest) = server.join().unwrap();

    // What joy5 printed: the tags as the script holds them, and its
    // verdict on the server's handshake digests.
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1 + tags.len() + 2);
    let (kind, message) = lines[0].split_once(' ').unwrap();
    assert_eq!(kind, "metadata");
    let message = hex::decode(message).unwrap();
    let values = amf0::decode_all(&message).unwrap();
    assert_eq!(
        values[..2],
        [Value::String("@setDataFrame"), Value::String("onMetaData")]
    );
    assert_eq!(Metadata::from_value(&values[2]), Some(metadata()));
    for ((video, ts, body), line) in tags.iter().zip(&lines[1..]) {
        let kind = if *video { "video" } else { "audio" };
        assert_eq!(*line, format!("{kind} {ts} {}", hex::encode(body)));
    }
    assert_eq!(lines[lines.len() - 2..], ["s1 true", "s2 true"]);
    assert!(digest, "joy5 sends a digest C1");

    // What the server yielded.
    let mut expect = vec![
        Ev::Publish {
            app: "live".to_owned(),
            name: "key".to_owned(),
        },
        Ev::Metadata(metadata()),
    ];
    for (video, ts, body) in tags {
        expect.push(if video {
            Ev::Video(ts, body)
        } else {
            Ev::Audio(ts, body)
        });
    }
    expect.push(Ev::Unpublish);
    assert_eq!(events.len(), expect.len());
    for (i, (got, want)) in events.iter().zip(&expect).enumerate() {
        assert!(got == want, "event {i} differs");
    }
    assert_eq!(phase, Phase::Connecting);
}
