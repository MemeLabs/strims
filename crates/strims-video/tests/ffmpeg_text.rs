// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Subtitles from a real publisher: ffmpeg publishes a test pattern with
//! an SRT file's subtitles as `onTextData` (`-c:s text`) over RTMP to an
//! [`RtmpServer`], and the [`FlvSource`] makes cues of them. Skipped when
//! `ffmpeg` isn't on PATH.
//!
//! - The stream's metadata declares a data stream.
//! - Each subtitle is a cue that shows from its start, on the video's
//!   timeline (whose first frame presents at the time ffmpeg moves the
//!   stream by, for its B-frames), for its reading time, its lines joined
//!   by LF, its styling left out.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::time::Duration;

use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_video::rtmp::{RtmpConfig, RtmpEvent, RtmpServer};
use strims_video::text::{self, parse_cue};
use strims_video::{FlvSource, SourceEvent};

const SRT: &str = "1
00:00:00,500 --> 00:00:01,500
Hello

2
00:00:01,800 --> 00:00:03,000
two lines
of <i>text</i>

3
00:00:03,100 --> 00:00:03,400
Bye
";

fn ffmpeg() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// A cue made of an `onTextData`: its PTS, duration and text.
type Got = (i64, u64, String);

#[test]
fn ffmpegs_subtitles_over_rtmp_are_cues() {
    if !ffmpeg() {
        eprintln!("skipped: no ffmpeg");
        return;
    }
    let srt = std::env::temp_dir().join(format!("strims-video-{}-subs.srt", std::process::id()));
    std::fs::write(&srt, SRT).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!(
        "rtmp://127.0.0.1:{}/live/key",
        listener.local_addr().unwrap().port()
    );
    let mut child = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
        .arg("testsrc2=size=320x240:rate=30:duration=4")
        .arg("-i")
        .arg(&srt)
        .args(["-map", "0:v", "-map", "1:s", "-c:v", "libx264", "-g", "30"])
        .args(["-pix_fmt", "yuv420p", "-c:s", "text", "-f", "flv"])
        .arg(&url)
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let (mut conn, _) = listener.accept().unwrap();
    conn.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let mut server = RtmpServer::new(&RtmpConfig::default(), &mut det_rng(1));
    let mut source = FlvSource::new();
    let mut datastream = false;
    let mut first_frame = None;
    let mut got: Vec<Got> = Vec::new();
    let mut input = Vec::new();
    let mut buf = vec![0; 64 * 1024];
    // Reads that time out, 200 ms each: a minute of them gives up.
    let mut idle = 0;
    'conn: while idle < 300 {
        match conn.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => input.extend_from_slice(&buf[..n]),
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => idle += 1,
            Err(_) => break,
        }
        let mut pos = 0;
        loop {
            let n = server.handle_input(&input[pos..]).unwrap();
            pos += n;
            let mut evented = false;
            while let Some(e) = server.poll_event() {
                evented = true;
                match e {
                    RtmpEvent::Publish { .. } => {
                        server.accept_publish();
                        break;
                    }
                    RtmpEvent::Metadata(m) => datastream |= m.datastream,
                    RtmpEvent::Video { timestamp, data } => {
                        let at = Time::from_unix_millis(0);
                        if let Ok(Some(SourceEvent::Unit(u))) = source.video(timestamp, data, at) {
                            first_frame.get_or_insert(u.pts / 90);
                        }
                    }
                    RtmpEvent::Text { timestamp, text } => {
                        let u = source.text(timestamp, &text, Time::from_unix_millis(0));
                        let c = parse_cue(u.payload).unwrap();
                        got.push((u.pts, c.duration, c.text.to_owned()));
                    }
                    RtmpEvent::Unpublish => break 'conn,
                    _ => {}
                }
            }
            let mut out = vec![0; 64 * 1024];
            loop {
                let n = server.poll_transmit(&mut out);
                if n == 0 {
                    break;
                }
                conn.write_all(&out[..n]).unwrap();
            }
            if n == 0 && !evented {
                break;
            }
        }
        input.drain(..pos);
    }
    drop(conn);
    let _ = child.kill();
    let _ = child.wait();
    std::fs::remove_file(&srt).unwrap();
    assert!(datastream, "the metadata declares a data stream");
    let shift = first_frame.expect("video");
    let want: Vec<Got> = [(500, "Hello"), (1800, "two lines\nof text"), (3100, "Bye")]
        .into_iter()
        .map(|(at, t)| {
            (
                at + shift,
                u64::try_from(text::estimate(t).as_millis()).unwrap(),
                t.to_owned(),
            )
        })
        .collect();
    assert_eq!(got, want);
}
