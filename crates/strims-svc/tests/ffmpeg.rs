// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A service's video against ffmpeg: real encoders publishing over RTMP,
//! and ffmpeg playing the service's HLS back. Skipped when `ffmpeg` and
//! `ffprobe` aren't on PATH.
//!
//! - ffmpeg publishes a test pattern as H.264 (libx264, High profile,
//!   B-frames, 1-second GOPs) with AAC to a channel's RTMP URL, which
//!   ingests it in the unit format. The channel's listing names the format
//!   and the stream's codecs. The service serves the stream as HLS, opened
//!   with the listing's MIME type: every segment lasts its GOP and starts
//!   at a keyframe, ffprobe reads the playlist as H.264 at the published
//!   size with 48 kHz stereo AAC, and ffmpeg decodes both tracks from it
//!   without an error. An init with a segment probes as a whole fMP4 file.
//! - ffmpeg publishes a CRF stream whose keyframes come at scene cuts and
//!   at most 10 s apart. Its segments start
//!   at its keyframes, and each GOP longer than the 4 s target is split
//!   into segments within it, which start mid-GOP and decode after the
//!   GOP's first without an error.
//! - Every reload of every playlist keeps RFC 8216's rules
//!   ([`PlaylistRules`]): a target duration that never changes and that
//!   every segment rounds to at most, three targets listed, and no
//!   `EXT-X-INDEPENDENT-SEGMENTS`.
//! - ffmpeg publishing HEVC over Enhanced RTMP to a node that doesn't
//!   transcode it is refused with the reason, and the channel has no live
//!   stream.
//!
//! `STRIMS_HLS_DUMP=<dir>` writes the scene-cut stream's inits and
//! segments there, with a playlist listing all of them, for players to
//! try. Each step prints how long it took.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;
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
/// The service's HLS target duration, in seconds.
const TARGET: u64 = 4;

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
    /// A channel with an RTMP ingress.
    async fn open() -> Channel {
        let config = Config {
            hls_listen: Some("127.0.0.1:0".into()),
            ..local_config()
        };
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

    /// The MIME type of the channel's listing, once listed.
    async fn listed_mime(&self) -> String {
        let directory = DirectoryFrontendClient::new(self.alice.client());
        eventually("the channel's listing", STEP, || async {
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
                .find_map(|item| match item.listing.as_ref()?.content.as_ref()? {
                    listing::Content::Media(m) if m.swarm_uri == self.swarm_uri() => {
                        Some(m.mime_type.clone())
                    }
                    _ => None,
                })
        })
        .await
    }
}

/// The test pattern and a tone, as ffmpeg's inputs.
fn pattern() -> Vec<String> {
    let mut args = lavfi(&format!("testsrc2=size={WIDTH}x{HEIGHT}:rate=30"));
    args.extend(lavfi("sine=frequency=440:sample_rate=48000"));
    args
}

/// The test pattern with color bars over it at 1.3 to 2 s, 7.5 to 8 s and
/// 19.2 to 19.6 s, and a tone: scene cuts an encoder starts GOPs at.
fn scenes() -> Vec<String> {
    let mut args = lavfi(&format!("testsrc2=size={WIDTH}x{HEIGHT}:rate=30"));
    args.extend(lavfi(&format!("smptehdbars=size={WIDTH}x{HEIGHT}:rate=30")));
    args.extend(lavfi("sine=frequency=440:sample_rate=48000"));
    let bars = "between(t,1.3,2)+between(t,7.5,8)+between(t,19.2,19.6)";
    args.extend([
        "-filter_complex".to_owned(),
        format!("[0:v][1:v]overlay=enable='{bars}'[v]"),
        "-map".to_owned(),
        "[v]".to_owned(),
        "-map".to_owned(),
        "2:a".to_owned(),
    ]);
    args
}

fn lavfi(graph: &str) -> Vec<String> {
    ["-f", "lavfi", "-i", graph].map(str::to_owned).to_vec()
}

/// Starts ffmpeg publishing `inputs` to `url`, its video encoded with
/// `video` and its audio as AAC, in real time for at most `secs`.
fn publish(url: &str, inputs: &[String], video: &[&str], secs: u32) -> Child {
    let mut args: Vec<String> = ["-hide_banner", "-loglevel", "error", "-nostdin", "-re"]
        .map(str::to_owned)
        .to_vec();
    args.extend_from_slice(inputs);
    args.extend(video.iter().map(|s| (*s).to_owned()));
    let secs = secs.to_string();
    args.extend(
        [
            "-c:a", "aac", "-b:a", "128k", "-ar", "48000", "-ac", "2", "-t", &secs, "-f", "flv",
            url,
        ]
        .map(str::to_owned),
    );
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

/// H.264 High profile with B-frames at constant quality, a keyframe at
/// each scene cut and at least every 10 s.
const H264_CRF: &[&str] = &[
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
    "-crf",
    "28",
    "-g",
    "300",
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

/// A file of its own for `bytes`.
fn temp_file(name: &str, bytes: &[u8]) -> PathBuf {
    let file = std::env::temp_dir().join(format!("strims-ffmpeg-{}-{name}", std::process::id()));
    std::fs::write(&file, bytes).unwrap();
    file
}

/// Whether the first video packet of an init and media is a keyframe.
async fn starts_at_keyframe(init: &[u8], media: &[u8]) -> bool {
    let file = temp_file("first.mp4", &[init, media].concat());
    let (ok, out, err) = tool(
        "ffprobe",
        &[
            "-v",
            "error",
            "-select_streams",
            "v",
            "-show_entries",
            "packet=flags",
            "-read_intervals",
            "%+#1",
            "-of",
            "csv=p=0",
            file.to_str().unwrap(),
        ],
    )
    .await;
    let _ = std::fs::remove_file(&file);
    assert!(ok, "ffprobe of a segment: {err}");
    out.lines().next().unwrap_or_default().starts_with('K')
}

/// A channel streaming over RTMP, and the HLS stream of it opened with
/// its listing's MIME type.
struct Live {
    ch: Channel,
    ffmpeg: Child,
    mime: String,
    hls_addr: SocketAddr,
    playlist_url: String,
    /// The playlist's path, and its directory.
    path: String,
    dir: String,
    rules: PlaylistRules,
    /// The inits and segments fetched, by URI.
    files: BTreeMap<String, Vec<u8>>,
}

impl Live {
    /// Opens a channel, publishes `inputs` encoded with `video` for at most
    /// `secs`, and opens its HLS stream.
    async fn start(inputs: &[String], video: &[&str], secs: u32, steps: &mut Steps) -> Live {
        let ch = Channel::open().await;
        let hls_addr = ch.alice.svc.hls_addr().unwrap();
        ch.wait_listening().await;
        steps.done("alice has a channel and an RTMP ingress");

        let mut ffmpeg = publish(&ch.rtmp_url, inputs, video, secs);
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

        let mime = ch.listed_mime().await;
        eprintln!("listing: {mime}");
        steps.done("the channel is listed");

        ch.alice.set_hls(true).await;
        let playlist_url = ch
            .alice
            .hls()
            .open_stream(&HlsEgressOpenStreamRequest {
                swarm_uri: ch.swarm_uri(),
                network_keys: vec![ch.network_key.clone()],
                mime_type: mime.clone(),
            })
            .await
            .unwrap()
            .playlist_url;
        let path = playlist_url
            .strip_prefix(&format!("http://{hls_addr}"))
            .unwrap()
            .to_owned();
        let dir = path.strip_suffix("index.m3u8").unwrap().to_owned();
        Live {
            ch,
            ffmpeg,
            mime,
            hls_addr,
            playlist_url,
            path,
            dir,
            rules: PlaylistRules::default(),
            files: BTreeMap::new(),
        }
    }

    /// Reloads the playlist, checked against the rules: its text and
    /// segments.
    async fn reload(&mut self) -> (String, Vec<Entry>) {
        let r = request(self.hls_addr, "GET", &self.path).await;
        assert_eq!(r.status, 200, "{}", r.text());
        let playlist = r.text();
        let entries = self.rules.check(&playlist);
        (playlist, entries)
    }

    /// Reloads the playlist every 250 ms until `done` holds of its
    /// segments and the rules' record of the reloads.
    async fn until(
        &mut self,
        what: &str,
        mut done: impl FnMut(&[Entry], &PlaylistRules) -> bool,
    ) -> String {
        let deadline = Instant::now() + STEP;
        loop {
            let (playlist, entries) = self.reload().await;
            if done(&entries, &self.rules) {
                return playlist;
            }
            assert!(Instant::now() < deadline, "{what}: {playlist}");
            tokio::time::sleep(StdDuration::from_millis(250)).await;
        }
    }

    /// The bytes of `uri`, beside the playlist, fetched once.
    async fn file(&mut self, uri: &str) -> Vec<u8> {
        if let Some(f) = self.files.get(uri) {
            return f.clone();
        }
        let r = request(self.hls_addr, "GET", &format!("{}{uri}", self.dir)).await;
        assert_eq!(r.status, 200, "{uri}");
        self.files.insert(uri.to_owned(), r.body.clone());
        r.body
    }

    /// Whether the segment `e` starts at a keyframe.
    async fn keyframe(&mut self, e: &Entry) -> bool {
        let init = self.file(&e.listed.init).await;
        let media = self.file(&e.listed.uri).await;
        starts_at_keyframe(&init, &media).await
    }

    /// ffmpeg decodes both tracks of the playlist for 2 s, without an
    /// error when `strict`.
    async fn decodes(&self, strict: bool) {
        let mut args = vec!["-hide_banner", "-v", "error"];
        if strict {
            args.push("-xerror");
        }
        args.extend([
            "-i",
            &self.playlist_url,
            "-map",
            "0",
            "-t",
            "2",
            "-f",
            "null",
            "-",
        ]);
        let (ok, _, err) = tool("ffmpeg", &args).await;
        assert!(
            ok && (!strict || err.is_empty()),
            "ffmpeg decoding the HLS stream: {err}"
        );
    }

    /// Stops publishing and the service.
    async fn end(mut self) {
        self.ffmpeg.kill().await.unwrap();
        eventually("the stream to end", STEP, || async {
            (self.ch.live_streams().await == 0).then_some(())
        })
        .await;
        self.ch.alice.svc.stop().await;
    }
}

/// A stream of 1-second GOPs: every segment a GOP, played back by ffmpeg.
async fn one_second_gops() {
    let mut steps = Steps(Instant::now());
    let mut live = Live::start(&pattern(), H264, 120, &mut steps).await;
    // The listing names the format and the stream's codecs: x264's High
    // profile at its level, and AAC-LC.
    let mime = live.mime.clone();
    assert!(
        mime.starts_with("video/x.strims-units;codecs=\"avc1.6400")
            && mime.ends_with(",mp4a.40.2\""),
        "{mime}"
    );
    steps.done("alice opens the HLS stream with the listing's MIME type");

    let playlist = live
        .until("three listed segments", |entries, _| entries.len() >= 3)
        .await;
    eprintln!("{playlist}");
    assert_eq!(live.rules.target, Some(TARGET));
    // 1-second GOPs, each its own segment, from its keyframe.
    let (_, entries) = live.reload().await;
    for e in &entries {
        assert_eq!(e.listed.extinf, "#EXTINF:1.000,", "{playlist}");
        assert!(!e.discontinuity, "{playlist}");
        assert!(live.keyframe(e).await, "{} starts mid-GOP", e.listed.uri);
    }
    steps.done("the HLS playlist lists 1-second segments from their keyframes");

    let streams = probe_streams(&live.playlist_url).await;
    eprintln!("ffprobe: {streams:?}");
    assert_eq!(
        streams,
        vec![
            "aac,LC,48000,2".to_owned(),
            format!("h264,High,{WIDTH},{HEIGHT}"),
        ],
    );
    live.decodes(true).await;
    steps.done("ffmpeg plays the HLS stream back");

    // An init with one of its segments is a whole fMP4 file.
    let init = live.file(&entries[0].listed.init).await;
    let media = live.file(&entries[0].listed.uri).await;
    let file = temp_file("segment.mp4", &[init, media].concat());
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

    // The playlist keeps the rules as segments slide out of it.
    live.until("segments sliding out", |_, rules| rules.slid)
        .await;
    steps.done("segments slide out of the playlist");
    live.end().await;
}

#[test]
fn ffmpeg_plays_back_a_unit_format_ingest_over_hls() {
    if !tools_available() {
        return;
    }
    run(async {
        tokio::time::timeout(StdDuration::from_secs(300), one_second_gops())
            .await
            .expect("the scenario took over 300 s");
    });
}

#[test]
fn ffmpeg_plays_back_scene_cut_gops_split_within_the_target() {
    if !tools_available() {
        return;
    }
    run(async {
        tokio::time::timeout(StdDuration::from_secs(300), async {
            let mut steps = Steps(Instant::now());
            let mut live = Live::start(&scenes(), H264_CRF, 60, &mut steps).await;
            assert!(
                live.mime.starts_with("video/x.strims-units;"),
                "{}",
                live.mime
            );
            steps.done("alice opens the HLS stream with the listing's MIME type");

            // Seen segments: whether each starts at a keyframe.
            let mut starts: BTreeMap<u64, bool> = BTreeMap::new();
            let deadline = Instant::now() + StdDuration::from_secs(90);
            loop {
                let (playlist, entries) = live.reload().await;
                for e in &entries {
                    if let std::collections::btree_map::Entry::Vacant(v) = starts.entry(e.seq) {
                        v.insert(live.keyframe(e).await);
                    }
                }
                let mid_gop = starts.values().filter(|k| !**k).count();
                let media: u64 = live.rules.seen.values().map(|e| e.millis).sum();
                if mid_gop >= 3 && media >= 16_000 {
                    eprintln!("{playlist}");
                    break;
                }
                assert!(Instant::now() < deadline, "no GOP was split: {playlist}");
                tokio::time::sleep(StdDuration::from_millis(250)).await;
            }
            let seen: Vec<Entry> = live.rules.seen.values().cloned().collect();
            let summary: Vec<(u64, bool)> =
                seen.iter().map(|e| (e.millis, starts[&e.seq])).collect();
            eprintln!("segments (ms, from a keyframe): {summary:?}");
            assert!(seen.iter().all(|e| !e.discontinuity));
            // A GOP over the target: a segment from its keyframe, then
            // ones from within it.
            let split = seen
                .windows(2)
                .position(|w| starts[&w[0].seq] && !starts[&w[1].seq])
                .expect("a keyframe's segment followed by a mid-GOP one");
            assert!(seen[split].millis > 4000, "{summary:?}");
            steps.done("GOPs over the target are split into segments within it");

            // The GOP's segments decode in sequence from its keyframe.
            let run: Vec<Entry> = seen[split..]
                .iter()
                .take_while(|e| e.seq == seen[split].seq || !starts[&e.seq])
                .cloned()
                .collect();
            let init = live.file(&run[0].listed.init).await;
            let mut file = init.clone();
            for e in &run {
                assert_eq!(e.listed.init, run[0].listed.init);
                file.extend(live.file(&e.listed.uri).await);
            }
            let path = temp_file("gop.mp4", &file);
            let path_arg = path.to_str().unwrap().to_owned();
            let (ok, _, err) = tool(
                "ffmpeg",
                &[
                    "-hide_banner",
                    "-v",
                    "error",
                    "-xerror",
                    "-i",
                    &path_arg,
                    "-map",
                    "0",
                    "-f",
                    "null",
                    "-",
                ],
            )
            .await;
            let (_, frames, _) = tool(
                "ffprobe",
                &[
                    "-v",
                    "error",
                    "-ignore_editlist",
                    "1",
                    "-select_streams",
                    "v",
                    "-count_frames",
                    "-show_entries",
                    "stream=nb_read_frames",
                    "-of",
                    "csv=p=0",
                    &path_arg,
                ],
            )
            .await;
            let _ = std::fs::remove_file(&path);
            assert!(ok && err.is_empty(), "ffmpeg decoding a split GOP: {err}");
            let gop_millis: u64 = run.iter().map(|e| e.millis).sum();
            let frames: u64 = frames.trim().parse().unwrap();
            assert!(
                frames.abs_diff(gop_millis * 30 / 1000) <= 1,
                "{frames} frames over {gop_millis} ms"
            );
            steps.done("a split GOP's segments decode in sequence");

            let streams = probe_streams(&live.playlist_url).await;
            assert_eq!(
                streams,
                vec![
                    "aac,LC,48000,2".to_owned(),
                    format!("h264,High,{WIDTH},{HEIGHT}"),
                ],
            );
            live.decodes(false).await;
            steps.done("ffmpeg plays the HLS stream back");

            if let Some(dir) = std::env::var_os("STRIMS_HLS_DUMP") {
                dump(&mut live, &seen, &PathBuf::from(dir)).await;
            }
            live.end().await;
        })
        .await
        .expect("the scenario took over 300 s");
    });
}

/// Writes the inits and segments of `seen` into `dir`, with `vod.m3u8`
/// listing them all.
async fn dump(live: &mut Live, seen: &[Entry], dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    let mut playlist = format!(
        "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:{TARGET}\n\
         #EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-MEDIA-SEQUENCE:{}\n",
        seen[0].seq
    );
    let mut init = String::new();
    for e in seen {
        if e.listed.init != init {
            init = e.listed.init.clone();
            std::fs::write(dir.join(&init), live.file(&init).await).unwrap();
            playlist.push_str(&format!("#EXT-X-MAP:URI=\"{init}\"\n"));
        }
        std::fs::write(dir.join(&e.listed.uri), live.file(&e.listed.uri).await).unwrap();
        playlist.push_str(&format!(
            "#EXTINF:{}.{:03},\n{}\n",
            e.millis / 1000,
            e.millis % 1000,
            e.listed.uri
        ));
    }
    playlist.push_str("#EXT-X-ENDLIST\n");
    std::fs::write(dir.join("vod.m3u8"), playlist).unwrap();
    eprintln!("dumped {} segments to {}", seen.len(), dir.display());
}

#[test]
fn ffmpeg_publishing_hevc_to_a_node_that_does_not_transcode_is_refused_with_the_reason() {
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
            let mut ffmpeg = publish(&ch.rtmp_url, &pattern(), hevc, 30);
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
                err.contains(
                    "HEVC (hvc1) needs a node that transcodes it to H.264, and this one doesn't; \
                     send H.264 with AAC"
                ),
                "{err}"
            );
            assert_eq!(ch.live_streams().await, 0);
            ch.alice.svc.stop().await;
        })
        .await
        .expect("the scenario took over 120 s");
    });
}
