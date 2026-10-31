// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A service's codec stage against real encoders: ffmpeg publishes over
//! RTMP to a service whose ffmpeg transcodes, and ffprobe reads the
//! renditions' HLS. Skipped when `ffmpeg` and `ffprobe` aren't on PATH,
//! or ffmpeg lacks libx264 or libx265.
//!
//! - An H.264 stream with a 480p30 rung: the snippet lists the channel's
//!   swarm and the rung's, and the rung plays over HLS as 854x480 H.264
//!   with the stream's AAC.
//! - An HEVC stream: its listing names H.264, which plays over HLS at the
//!   stream's size, and the snippet lists its own encoding as a rendition.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::process::Stdio;
use std::time::Duration as StdDuration;

use strims_node::codec::FfmpegConfig;
use strims_proto::network::v1::directory::{
    DirectoryFrontendClient, FrontendGetListingsRequest, ListingRendition, ListingSnippet, listing,
};
use strims_proto::network::v1::{CreateServerRequest, NetworkFrontendClient};
use strims_proto::video::v1::{
    HlsEgressOpenStreamRequest, VideoChannelCreateRequest, VideoChannelFrontendClient,
    VideoIngressClient, VideoIngressConfig, VideoIngressGetChannelUrlRequest,
    VideoIngressSetConfigRequest,
};
use strims_svc::Config;
use strims_video::codec::RenditionPlan;
use support::video::*;
use support::*;
use tokio::process::{Child, Command};

/// How long a step may take.
const STEP: StdDuration = StdDuration::from_secs(60);

fn tools_available() -> bool {
    let encoders = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let probe = std::process::Command::new("ffprobe")
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success());
    if probe && encoders.contains("libx264") && encoders.contains("libx265") {
        return true;
    }
    eprintln!("skipping: ffmpeg with libx264 and libx265, or ffprobe, is not on PATH");
    false
}

/// A service whose codec stage runs the ffmpeg on PATH, with `ladder`,
/// and a channel of it with an RTMP ingress.
struct Channel {
    alice: Profile,
    network_key: Vec<u8>,
    rtmp_url: String,
}

impl Channel {
    async fn open(ladder: Vec<RenditionPlan>) -> Channel {
        let mut config = Config {
            hls_listen: Some("127.0.0.1:0".into()),
            ..local_config()
        };
        config.app.ffmpeg = Some(FfmpegConfig {
            program: "ffmpeg".into(),
            ..FfmpegConfig::default()
        });
        config.app.video.ingress.transcode.ladder = ladder;
        let alice = Profile::sign_up(config).await;
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
                    title: "renditions".into(),
                    ..Default::default()
                }),
                network_key: network_key.clone(),
            })
            .await
            .unwrap()
            .channel
            .unwrap();
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
            .unwrap()
            .url;
        let addr = format!("127.0.0.1:{port}");
        eventually("the ingress to listen", STEP, || {
            let addr = addr.clone();
            async move { tokio::net::TcpStream::connect(&addr).await.ok().map(drop) }
        })
        .await;
        alice.set_hls(true).await;
        Channel {
            alice,
            network_key,
            rtmp_url: url,
        }
    }

    /// The channel's media listing's MIME type and its snippet's
    /// renditions, once it lists `n`.
    async fn renditions(&self, n: usize) -> (String, Vec<ListingRendition>) {
        let directory = DirectoryFrontendClient::new(self.alice.client());
        eventually("the snippet's renditions", STEP, || async {
            let res = directory
                .get_listings(&FrontendGetListingsRequest {
                    content_types: Vec::new(),
                    network_keys: vec![self.network_key.clone()],
                })
                .await
                .ok()?;
            res.listings
                .iter()
                .flat_map(|n| n.listings.iter())
                .find_map(|item| {
                    let listing::Content::Media(m) = item.listing.as_ref()?.content.as_ref()?
                    else {
                        return None;
                    };
                    let renditions = &item.snippet.as_ref()?.renditions;
                    (renditions.len() == n).then(|| (m.mime_type.clone(), renditions.clone()))
                })
        })
        .await
    }

    /// ffprobe's streams of `r`'s HLS, once it has segments.
    async fn probe(&self, r: &ListingRendition) -> Vec<String> {
        let url = self
            .alice
            .hls()
            .open_stream(&HlsEgressOpenStreamRequest {
                swarm_uri: r.swarm_uri.clone(),
                network_keys: vec![self.network_key.clone()],
                mime_type: r.mime_type.clone(),
            })
            .await
            .unwrap()
            .playlist_url;
        eventually("the rendition's HLS to play", STEP, || {
            let url = url.clone();
            async move {
                let out = Command::new("ffprobe")
                    .args([
                        "-v",
                        "error",
                        "-show_entries",
                        "stream=codec_name,width,height",
                        "-of",
                        "csv=p=0",
                        &url,
                    ])
                    .output()
                    .await
                    .ok()?;
                let mut lines: Vec<String> = String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .map(|l| l.trim_end_matches(',').to_owned())
                    .filter(|l| !l.is_empty())
                    .collect();
                lines.sort();
                lines.dedup();
                (out.status.success() && lines.len() >= 2).then_some(lines)
            }
        })
        .await
    }
}

/// ffmpeg publishing a 1280x720 30 fps test pattern and a tone to `url`
/// in real time for `secs`, its video encoded with `video`.
fn publish(url: &str, video: &[&str], secs: u32) -> Child {
    let mut args: Vec<String> = ["-hide_banner", "-loglevel", "error", "-nostdin", "-re"]
        .map(str::to_owned)
        .to_vec();
    for graph in [
        "testsrc2=size=1280x720:rate=30",
        "sine=frequency=440:sample_rate=48000",
    ] {
        args.extend(["-f", "lavfi", "-i", graph].map(str::to_owned));
    }
    args.extend(video.iter().map(|s| (*s).to_owned()));
    let secs = secs.to_string();
    args.extend(
        [
            "-g", "60", "-c:a", "aac", "-b:a", "128k", "-t", &secs, "-f", "flv", url,
        ]
        .map(str::to_owned),
    );
    Command::new("ffmpeg")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap()
}

#[test]
fn an_h264_streams_rung_plays_over_hls() {
    if !tools_available() {
        return;
    }
    run(async {
        tokio::time::timeout(StdDuration::from_secs(180), async {
            let rung = RenditionPlan {
                short_side: 480,
                max_frame_rate: 30,
                bitrate: 800_000,
            };
            let ch = Channel::open(vec![rung]).await;
            let x264 = ["-c:v", "libx264", "-preset", "veryfast", "-b:v", "2M"];
            let _ffmpeg = publish(&ch.rtmp_url, &x264, 30);
            let (mime, renditions) = ch.renditions(2).await;
            assert_eq!(renditions[0].mime_type, mime);
            assert_eq!((renditions[0].width, renditions[0].height), (1280, 720));
            let r = &renditions[1];
            assert!(r.mime_type.contains("avc1."), "{}", r.mime_type);
            assert_eq!((r.width, r.height), (854, 480));
            let streams = ch.probe(r).await;
            assert_eq!(streams, ["aac", "h264,854,480"], "{streams:?}");
            ch.alice.svc.stop().await;
        })
        .await
        .expect("the scenario took over 180 s");
    });
}

#[test]
fn an_hevc_stream_is_listed_and_plays_as_h264() {
    if !tools_available() {
        return;
    }
    run(async {
        tokio::time::timeout(StdDuration::from_secs(180), async {
            let ch = Channel::open(Vec::new()).await;
            let x265 = [
                "-c:v",
                "libx265",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p",
                "-x265-params",
                "log-level=error",
            ];
            let _ffmpeg = publish(&ch.rtmp_url, &x265, 30);
            let (mime, renditions) = ch.renditions(2).await;
            assert!(mime.contains("avc1."), "{mime}");
            assert_eq!(renditions[0].mime_type, mime);
            assert!(
                renditions[1].mime_type.contains("hvc1."),
                "{}",
                renditions[1].mime_type
            );
            let streams = ch.probe(&renditions[0]).await;
            assert_eq!(streams, ["aac", "h264,1280,720"], "{streams:?}");
            ch.alice.svc.stop().await;
        })
        .await
        .expect("the scenario took over 180 s");
    });
}
