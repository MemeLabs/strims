// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A service's video against ffmpeg: real encoders publishing over RTMP,
//! and ffmpeg playing the service's HLS back. Skipped when `ffmpeg` and
//! `ffprobe` aren't on PATH.
//!
//! - ffmpeg publishes a test pattern as H.264 (libx264, High profile,
//!   B-frames, 1-second GOPs) with AAC to a channel's RTMP URL. The
//!   channel's listing names the stream's codecs. The service serves the
//!   stream as HLS: every segment lasts its GOP, ffprobe reads the
//!   playlist as H.264 at the published size with 48 kHz stereo AAC, and
//!   ffmpeg decodes both tracks from it without an error. An init with a
//!   segment probes as a whole fMP4 file.
//! - ffmpeg publishing HEVC over Enhanced RTMP is refused with the
//!   reason, and the channel has no live stream.
//!
//! Each step prints how long it took.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::process::Stdio;
use std::time::Duration as StdDuration;

use strims_ppspp::{SwarmId, Uri};
use strims_proto::network::v1::directory::{
    DirectoryFrontendClient, FrontendGetListingsRequest, ListingSnippet, listing,
};
use strims_proto::network::v1::{CreateServerRequest, NetworkFrontendClient};
use strims_proto::video::v1::{
    HlsEgressOpenStreamRequest, VideoChannelCreateRequest, VideoChannelFrontendClient,
    VideoIngressClient, VideoIngressConfig, VideoIngressGetChannelUrlRequest,
    VideoIngressListStreamsRequest, VideoIngressSetConfigRequest,
};
use strims_svc::Config;
use strims_video::ingress_options;
use support::video::*;
use support::*;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::time::Instant;

/// The published picture.
const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;

fn tools_available() -> bool {
    let ok = |tool: &str| {
        std::process::Command::new(tool)
            .arg("-version")
            .output()
            .is_ok_and(|o| o.status.success())
    };
    if ok("ffmpeg") && ok("ffprobe") {
        return true;
    }
    eprintln!("skipping: `ffmpeg` or `ffprobe` is not on PATH");
    false
}

/// A profile with a network, a video channel on it, and the RTMP ingress
/// listening on a free port.
struct Channel {
    alice: Profile,
    network_key: Vec<u8>,
    channel_key: Vec<u8>,
    /// `rtmp://127.0.0.1:<port>/live/<stream key>`.
    rtmp_url: String,
}

impl Channel {
    async fn open() -> Channel {
        let alice = Profile::sign_up(Config {
            hls_listen: Some("127.0.0.1:0".into()),
            ..local_config()
        })
        .await;
        let network = NetworkFrontendClient::new(alice.client())
            .create_server(&CreateServerRequest {
                name: "streams".into(),
                ..Default::default()
            })
            .await
            .unwrap()
            .network
            .unwrap();
        let network_key = network_key(network.certificate.as_ref().unwrap());
        let channel = VideoChannelFrontendClient::new(alice.client())
            .create(&VideoChannelCreateRequest {
                directory_listing_snippet: Some(ListingSnippet {
                    title: "ffmpeg test pattern".into(),
                    channel_name: "alice's channel".into(),
                    ..Default::default()
                }),
                network_key: network_key.clone(),
            })
            .await
            .unwrap()
            .channel
            .unwrap();
        // The ingress refuses port 0: take a port the OS picks.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let ingress = VideoIngressClient::new(alice.client());
        ingress
            .set_config(&VideoIngressSetConfigRequest {
                config: Some(VideoIngressConfig {
                    enabled: true,
                    server_addr: format!("127.0.0.1:{port}"),
                    ..Default::default()
                }),
            })
            .await
            .unwrap();
        let url = ingress
            .get_channel_url(&VideoIngressGetChannelUrlRequest { id: channel.id })
            .await
            .unwrap();
        Channel {
            alice,
            network_key,
            channel_key: channel.key.unwrap().public,
            rtmp_url: url.url,
        }
    }

    fn ingress(&self) -> VideoIngressClient<strims_rpc::Client> {
        VideoIngressClient::new(self.alice.client())
    }

    async fn live_streams(&self) -> usize {
        self.ingress()
            .list_streams(&VideoIngressListStreamsRequest {})
            .await
            .unwrap()
            .streams
            .len()
    }

    /// Waits for the ingress to accept a TCP connection, so a publisher
    /// started next finds it listening.
    async fn wait_listening(&self) {
        let addr = self
            .rtmp_url
            .strip_prefix("rtmp://")
            .and_then(|r| r.split('/').next())
            .unwrap()
            .to_owned();
        eventually("the ingress to listen", STEP, || {
            let addr = addr.clone();
            async move { tokio::net::TcpStream::connect(&addr).await.ok().map(drop) }
        })
        .await;
    }

    /// The channel's swarm URI.
    fn swarm_uri(&self) -> String {
        Uri::new(
            SwarmId::new(&self.channel_key),
            ingress_options().uri_options(),
        )
        .to_string()
    }
}

/// Starts ffmpeg publishing a test pattern and a tone to `url`, encoded
/// with `video` (ffmpeg's arguments after the inputs), in real time for
/// at most `secs`.
fn publish(url: &str, video: &[&str], secs: u32) -> Child {
    let size = format!("testsrc2=size={WIDTH}x{HEIGHT}:rate=30");
    let mut args = vec![
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-re",
        "-f",
        "lavfi",
        "-i",
        &size,
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:sample_rate=48000",
    ];
    args.extend_from_slice(video);
    let secs = secs.to_string();
    args.extend_from_slice(&[
        "-c:a", "aac", "-b:a", "128k", "-ar", "48000", "-ac", "2", "-t", &secs, "-f", "flv", url,
    ]);
    Command::new("ffmpeg")
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap()
}

/// H.264 High profile with B-frames and a keyframe every second.
const H264: &[&str] = &[
    "-c:v",
    "libx264",
    "-preset",
    "veryfast",
    "-profile:v",
    "high",
    "-pix_fmt",
    "yuv420p",
    "-bf",
    "2",
    "-g",
    "30",
    "-keyint_min",
    "30",
    "-sc_threshold",
    "0",
    "-b:v",
    "1M",
];

/// Runs a tool to completion; its exit status and its stdout and stderr.
async fn tool(name: &str, args: &[&str]) -> (bool, String, String) {
    let out = tokio::time::timeout(
        STEP,
        Command::new(name)
            .args(args)
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap_or_else(|_| panic!("{name} {args:?} ran over {STEP:?}"))
    .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// ffprobe's `codec_name,width,height,sample_rate,channels` lines for
/// `input`'s streams, as CSV.
async fn probe_streams(input: &str) -> Vec<String> {
    let (ok, out, err) = tool(
        "ffprobe",
        &[
            "-v",
            "error",
            "-show_entries",
            "stream=codec_name,profile,width,height,sample_rate,channels",
            "-of",
            "csv=p=0",
            input,
        ],
    )
    .await;
    assert!(ok, "ffprobe {input}: {err}");
    // A playlist's streams are listed under its program and again on
    // their own.
    let mut lines: Vec<String> = out
        .lines()
        .map(|l| l.trim_end_matches(',').to_owned())
        .filter(|l| !l.is_empty())
        .collect();
    lines.sort();
    lines.dedup();
    lines
}

#[test]
fn ffmpeg_publishes_and_plays_back_over_hls() {
    if !tools_available() {
        return;
    }
    run(async {
        tokio::time::timeout(StdDuration::from_secs(300), async {
            let mut steps = Steps(Instant::now());
            let ch = Channel::open().await;
            let hls_addr = ch.alice.svc.hls_addr().unwrap();
            ch.wait_listening().await;
            steps.done("alice has a channel and an RTMP ingress");

            let mut ffmpeg = publish(&ch.rtmp_url, H264, 120);
            let deadline = Instant::now() + STEP;
            while ch.live_streams().await == 0 {
                if let Ok(Some(status)) = ffmpeg.try_wait() {
                    let mut err = String::new();
                    let _ = ffmpeg.stderr.take().unwrap().read_to_string(&mut err).await;
                    panic!("ffmpeg exited with {status}: {err}");
                }
                assert!(Instant::now() < deadline, "the channel didn't go live");
                tokio::time::sleep(StdDuration::from_millis(250)).await;
            }
            steps.done("ffmpeg publishes H.264 with AAC");

            // The listing names the stream's codecs: x264's High profile
            // at its level, and AAC-LC.
            let directory = DirectoryFrontendClient::new(ch.alice.client());
            let mime = eventually("the channel's listing", STEP, || async {
                let res = directory
                    .get_listings(&FrontendGetListingsRequest {
                        content_types: Vec::new(),
                        network_keys: vec![ch.network_key.clone()],
                    })
                    .await
                    .ok()?;
                res.listings
                    .iter()
                    .flat_map(|n| n.listings.iter())
                    .find_map(|item| match item.listing.as_ref()?.content.as_ref()? {
                        listing::Content::Media(m) if m.swarm_uri == ch.swarm_uri() => {
                            Some(m.mime_type.clone())
                        }
                        _ => None,
                    })
            })
            .await;
            eprintln!("listing: {mime}");
            assert!(
                mime.starts_with("video/mp4;codecs=\"avc1.6400") && mime.ends_with(",mp4a.40.2\""),
                "{mime}"
            );
            steps.done("the listing names the codecs");

            ch.alice.set_hls(true).await;
            let playlist_url = ch
                .alice
                .hls()
                .open_stream(&HlsEgressOpenStreamRequest {
                    swarm_uri: ch.swarm_uri(),
                    network_keys: vec![ch.network_key.clone()],
                })
                .await
                .unwrap()
                .playlist_url;
            let path = playlist_url
                .strip_prefix(&format!("http://{hls_addr}"))
                .unwrap()
                .to_owned();
            let dir = path.strip_suffix("index.m3u8").unwrap().to_owned();
            let (playlist, segments) = eventually("three listed segments", STEP, || async {
                let r = request(hls_addr, "GET", &path).await;
                assert_eq!(r.status, 200, "{}", r.text());
                let playlist = r.text();
                let segments = listed(&playlist);
                (segments.len() >= 3).then_some((playlist, segments))
            })
            .await;
            eprintln!("{playlist}");
            // 1-second GOPs, each its own segment.
            for s in &segments {
                assert_eq!(s.extinf, "#EXTINF:1.000,", "{playlist}");
            }
            steps.done("the HLS playlist lists 1-second segments");

            let streams = probe_streams(&playlist_url).await;
            eprintln!("ffprobe: {streams:?}");
            assert_eq!(
                streams,
                vec![
                    "aac,LC,48000,2".to_owned(),
                    format!("h264,High,{WIDTH},{HEIGHT}"),
                ],
            );
            let (ok, _, err) = tool(
                "ffmpeg",
                &[
                    "-hide_banner",
                    "-v",
                    "error",
                    "-xerror",
                    "-i",
                    &playlist_url,
                    "-map",
                    "0",
                    "-t",
                    "2",
                    "-f",
                    "null",
                    "-",
                ],
            )
            .await;
            assert!(
                ok && err.is_empty(),
                "ffmpeg decoding the HLS stream: {err}"
            );
            steps.done("ffmpeg plays the HLS stream back");

            // An init with one of its segments is a whole fMP4 file.
            let init = request(hls_addr, "GET", &format!("{dir}{}", segments[0].init)).await;
            let media = request(hls_addr, "GET", &format!("{dir}{}", segments[0].uri)).await;
            assert_eq!((init.status, media.status), (200, 200));
            let file =
                std::env::temp_dir().join(format!("strims-ffmpeg-{}.mp4", std::process::id()));
            std::fs::write(&file, [init.body, media.body].concat()).unwrap();
            let file_arg = file.to_str().unwrap().to_owned();
            // Counted past the init's edit list: under it, the audio a
            // file starts with falls within the composition delay and
            // decodes to nothing.
            let (ok, out, err) = tool(
                "ffprobe",
                &[
                    "-v",
                    "error",
                    "-ignore_editlist",
                    "1",
                    "-count_frames",
                    "-show_entries",
                    "stream=codec_name,nb_read_frames",
                    "-of",
                    "csv=p=0",
                    &file_arg,
                ],
            )
            .await;
            let _ = std::fs::remove_file(&file);
            assert!(ok && err.is_empty(), "ffprobe of a segment: {err}");
            eprintln!("a segment's frames: {out:?}");
            for line in out.lines() {
                let (codec, frames) = line.split_once(',').unwrap();
                let frames: u32 = frames.parse().unwrap();
                match codec {
                    "h264" => assert_eq!(frames, 30, "{out}"),
                    "aac" => assert!((45..=48).contains(&frames), "{out}"),
                    other => panic!("an unexpected stream {other}: {out}"),
                }
            }
            steps.done("a segment probes as a whole fMP4 file");

            ffmpeg.kill().await.unwrap();
            eventually("the stream to end", STEP, || async {
                (ch.live_streams().await == 0).then_some(())
            })
            .await;
            ch.alice.svc.stop().await;
        })
        .await
        .expect("the scenario took over 300 s");
    });
}

#[test]
fn ffmpeg_publishing_hevc_is_refused_with_the_reason() {
    if !tools_available() {
        return;
    }
    run(async {
        tokio::time::timeout(StdDuration::from_secs(120), async {
            let ch = Channel::open().await;
            ch.wait_listening().await;
            let hevc = &[
                "-c:v",
                "libx265",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p",
            ];
            let mut ffmpeg = publish(&ch.rtmp_url, hevc, 30);
            let status = tokio::time::timeout(STEP, ffmpeg.wait())
                .await
                .expect("ffmpeg ends when the server refuses its stream")
                .unwrap();
            let mut err = String::new();
            ffmpeg
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut err)
                .await
                .unwrap();
            eprintln!("ffmpeg: {status}: {err}");
            assert!(!status.success());
            assert!(
                err.contains("HEVC (hvc1) is not supported; send H.264 with AAC"),
                "{err}"
            );
            assert_eq!(ch.live_streams().await, 0);
            ch.alice.svc.stop().await;
        })
        .await
        .expect("the scenario took over 120 s");
    });
}
