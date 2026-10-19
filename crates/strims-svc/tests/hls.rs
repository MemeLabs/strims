// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HLS over HTTP from a service on localhost, driven as the UI and a
//! player drive it: the UI's calls go through the frontend API over a
//! real WebSocket, the stream comes in over real RTMP, and the player's
//! requests are raw HTTP.
//!
//! A profile signs up, creates a network and a video channel on it, and
//! enables the RTMP ingress on a free port. A synthetic publisher streams
//! 1-second GOPs to the channel's stream key, paced in real time. The
//! profile enables HLS and opens an HLS stream of the channel's swarm; the
//! player polls its playlist until it lists two segments, then fetches
//! the init and every listed segment. They are the parts of a contiguous
//! run of the records the script produces, and each EXTINF is its
//! segment's media time. Unknown streams answer 404, other methods 405, a
//! HEAD has no body, and every answer allows any origin.
//!
//! A service without an HLS listener doesn't support HLS egress, and one
//! whose HLS address is taken doesn't start.
//!
//! Each step prints how long it took.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::rc::Rc;
use std::time::Duration as StdDuration;

use strims_base::rng::det_rng;
use strims_ppspp::{SwarmId, Uri};
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::network::v1::{CreateServerRequest, NetworkFrontendClient};
use strims_proto::video::v1::{
    HlsEgressOpenStreamRequest, VideoChannelCreateRequest, VideoChannelFrontendClient,
    VideoChannelGetRequest, VideoChannelListRequest, VideoIngressClient, VideoIngressConfig,
    VideoIngressGetChannelUrlRequest, VideoIngressIsSupportedRequest, VideoIngressSetConfigRequest,
};
use strims_svc::{Config, Svc, SvcError};
use strims_video::amf0::Value;
use strims_video::fmp4::{media_duration, parse_init, split_record};
use strims_video::synth::publisher::{Publisher, PublisherConfig};
use strims_video::synth::script::{
    MetaValue, Script, ScriptConfig, Tag, TagKind, expected_records,
};
use strims_video::{CutRule, ingress_options};
use support::video::*;
use support::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::spawn_local;
use tokio::time::Instant;

/// The longest the script runs; the test ends long before.
const SCRIPT_SECS: u32 = 120;

/// The EXTINF line of a segment lasting `media`'s time, in milliseconds
/// rounded to the nearest.
fn extinf(init: &[u8], media: &[u8]) -> String {
    let d = media_duration(&parse_init(init).unwrap(), media).unwrap();
    let ms = u64::try_from(d.as_nanos()).unwrap().saturating_add(500_000) / 1_000_000;
    format!("#EXTINF:{}.{:03},", ms / 1000, ms % 1000)
}

/// A publisher's RTMP connection.
struct Publish {
    stream: TcpStream,
    publisher: Publisher,
    input: Vec<u8>,
    output: Vec<u8>,
    /// The server closed the connection.
    closed: bool,
}

impl Publish {
    /// Connects to `addr` and publishes `live/<key>`; `None` if the
    /// connection fails or the publish isn't accepted within `limit`.
    async fn open(addr: &str, key: &str, limit: StdDuration) -> Option<Publish> {
        let stream = TcpStream::connect(addr).await.ok()?;
        let config = PublisherConfig::new("live", key);
        let mut p = Publish {
            stream,
            publisher: Publisher::new(config, &mut det_rng(0x70)),
            input: vec![0; 64 * 1024],
            output: vec![0; 64 * 1024],
            closed: false,
        };
        let end = Instant::now() + limit;
        while !p.publisher.publish_accepted() && !p.closed && Instant::now() < end {
            if let Some(refused) = p.publisher.refused() {
                eprintln!("the publish was refused: {refused:?}");
                return None;
            }
            p.exchange(Instant::now() + StdDuration::from_millis(10))
                .await;
        }
        p.publisher.publish_accepted().then_some(p)
    }

    /// Writes what the publisher queued.
    async fn flush(&mut self) {
        loop {
            let n = self.publisher.poll_transmit(&mut self.output);
            if n == 0 || self.closed {
                return;
            }
            if self.stream.write_all(&self.output[..n]).await.is_err() {
                self.closed = true;
            }
        }
    }

    /// Takes and answers the server's bytes until `deadline`, or until
    /// the server closes the connection.
    async fn exchange(&mut self, deadline: Instant) {
        while !self.closed {
            self.flush().await;
            let read = tokio::select! {
                biased;
                r = self.stream.read(&mut self.input) => Some(r),
                () = tokio::time::sleep_until(deadline) => None,
            };
            match read {
                None => return,
                Some(Ok(0)) | Some(Err(_)) => self.closed = true,
                Some(Ok(n)) => self.publisher.handle_input(&self.input[..n]).unwrap(),
            }
        }
    }

    fn send(&mut self, tag: &Tag, metadata: &[(String, MetaValue)]) {
        match tag.kind {
            TagKind::Video => self.publisher.send_video(tag.timestamp, &tag.body).unwrap(),
            TagKind::Audio => self.publisher.send_audio(tag.timestamp, &tag.body).unwrap(),
            TagKind::Metadata => {
                let fields: Vec<(&str, Value<'_>)> = metadata
                    .iter()
                    .map(|(k, v)| {
                        let v = match v {
                            MetaValue::Number(n) => Value::Number(*n),
                            MetaValue::Bool(b) => Value::Boolean(*b),
                            MetaValue::String(s) => Value::String(s),
                        };
                        (k.as_str(), v)
                    })
                    .collect();
                self.publisher.send_metadata(&fields).unwrap();
            }
        }
    }

    /// Sends the script's tags, each at its time after now, then takes
    /// the server's bytes until it closes the connection.
    async fn stream(mut self, script: Rc<Script>) {
        let start = Instant::now();
        for tag in &script.tags {
            let due = start + StdDuration::from_millis(u64::from(tag.timestamp));
            self.exchange(due).await;
            assert!(!self.closed, "the server closed the publishing connection");
            self.send(tag, &script.metadata);
        }
        self.flush().await;
        self.exchange(Instant::now() + STEP).await;
    }
}

#[test]
fn hls_serves_an_rtmp_stream_to_players() {
    run(async {
        tokio::time::timeout(StdDuration::from_secs(300), async {
            let mut steps = Steps(Instant::now());
            let alice = Profile::sign_up(Config {
                hls_listen: Some("127.0.0.1:0".into()),
                ..local_config()
            })
            .await;
            let hls_addr = alice.svc.hls_addr().unwrap();
            assert_eq!(alice.svc.hls_base_url(), format!("http://{hls_addr}"));
            assert!(alice.hls_supported().await);
            steps.done("alice signs up on a service serving HLS");

            let network = NetworkFrontendClient::new(alice.client())
                .create_server(&CreateServerRequest {
                    name: "streams".into(),
                    ..Default::default()
                })
                .await
                .unwrap()
                .network
                .unwrap();
            let net = network_key(network.certificate.as_ref().unwrap());
            let channels = VideoChannelFrontendClient::new(alice.client());
            let created = channels
                .create(&VideoChannelCreateRequest {
                    directory_listing_snippet: Some(ListingSnippet {
                        title: "synthetic bars".into(),
                        description: "a test pattern".into(),
                        channel_name: "alice's channel".into(),
                        ..Default::default()
                    }),
                    network_key: net.clone(),
                })
                .await
                .unwrap()
                .channel
                .unwrap();
            let channel = channels
                .get(&VideoChannelGetRequest { id: created.id })
                .await
                .unwrap()
                .channel
                .unwrap();
            assert_eq!(channel, created);
            let all = channels.list(&VideoChannelListRequest {}).await.unwrap();
            assert_eq!(all.channels, vec![channel.clone()]);
            let channel_key = channel.key.unwrap().public;
            steps.done("alice creates a network and a video channel");

            // The ingress refuses port 0: take a port the OS picks.
            let port = std::net::TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port();
            let ingress = VideoIngressClient::new(alice.client());
            assert!(
                ingress
                    .is_supported(&VideoIngressIsSupportedRequest {})
                    .await
                    .unwrap()
                    .supported
            );
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
            assert_eq!(url.server_addr, format!("127.0.0.1:{port}"));
            assert_eq!(
                url.url,
                format!("rtmp://127.0.0.1:{port}/live/{}", url.stream_key)
            );

            // The publish waits for the listener and the network's join.
            let script = Rc::new(Script::new(&ScriptConfig {
                gop_frames: 30,
                duration_secs: SCRIPT_SECS,
                ..ScriptConfig::default()
            }));
            let publish = eventually("the ingress to accept the publish", STEP, || {
                let (addr, key) = (url.server_addr.clone(), url.stream_key.clone());
                async move { Publish::open(&addr, &key, StdDuration::from_secs(10)).await }
            })
            .await;
            let streaming = spawn_local(publish.stream(script.clone()));
            steps.done("a publisher streams to the channel's RTMP URL");

            alice.set_hls(true).await;
            let uri = Uri::new(SwarmId::new(&channel_key), ingress_options().uri_options());
            let playlist_url = alice
                .hls()
                .open_stream(&HlsEgressOpenStreamRequest {
                    swarm_uri: uri.to_string(),
                    network_keys: vec![net.clone()],
                })
                .await
                .unwrap()
                .playlist_url;
            // http://<HLS address>/hls/<profile key>/<transfer id>/index.m3u8
            let path = playlist_url
                .strip_prefix(&format!("http://{hls_addr}"))
                .unwrap();
            let profile_dir = format!("/hls/{}/", data_encoding::HEXLOWER.encode(&alice.key));
            let transfer = path
                .strip_prefix(&profile_dir)
                .and_then(|p| p.strip_suffix("/index.m3u8"))
                .unwrap_or_else(|| panic!("{playlist_url}"));
            assert_eq!(transfer.len(), 64, "{playlist_url}");
            assert!(transfer.bytes().all(|b| b.is_ascii_hexdigit()));
            let dir = path.strip_suffix("index.m3u8").unwrap();
            steps.done("alice opens an HLS stream of the channel");

            let (playlist, segments) = eventually("two listed segments", STEP, || async move {
                let r = request(hls_addr, "GET", path).await;
                assert_eq!(r.status, 200, "{}", r.text());
                r.check("the playlist", "application/vnd.apple.mpegurl", "no-cache");
                assert_eq!(r.content_length(), r.body.len());
                let playlist = r.text();
                let segments = listed(&playlist);
                (segments.len() >= 2).then_some((playlist, segments))
            })
            .await;
            eprintln!("{playlist}");
            assert!(
                playlist.starts_with("#EXTM3U\n#EXT-X-VERSION:7\n"),
                "{playlist}"
            );
            steps.done("the playlist lists two segments");

            let init_uri = &segments[0].init;
            assert!(segments.iter().all(|s| s.init == *init_uri), "{playlist}");
            let init = request(hls_addr, "GET", &format!("{dir}{init_uri}")).await;
            assert_eq!(init.status, 200);
            init.check("the init", "video/mp4", "max-age=31536000, immutable");
            assert_eq!(init.content_length(), init.body.len());
            let mut media = Vec::new();
            for s in &segments {
                let r = request(hls_addr, "GET", &format!("{dir}{}", s.uri)).await;
                assert_eq!(r.status, 200, "{}", s.uri);
                r.check(&s.uri, "video/iso.segment", "max-age=31536000, immutable");
                assert_eq!(r.content_length(), r.body.len());
                assert_eq!(s.extinf, extinf(&init.body, &r.body), "{}", s.uri);
                media.push(r.body);
            }

            // The segments are the parts of a contiguous run of the
            // script's records.
            let expected = expected_records(&script, CutRule::default());
            let parts: Vec<(&[u8], &[u8])> =
                expected.iter().map(|r| split_record(r).unwrap()).collect();
            let start = parts
                .iter()
                .position(|(_, m)| *m == media[0].as_slice())
                .expect("the first segment is one of the script's records");
            for (i, m) in media.iter().enumerate() {
                let (want_init, want_media) = parts[start + i];
                assert!(init.body == want_init, "segment {i}'s init");
                assert!(*m == want_media, "segment {i} is record {}", start + i);
            }
            eprintln!(
                "segments are records {start}..{} of {}, {}",
                start + media.len(),
                expected.len(),
                segments
                    .iter()
                    .map(|s| s.extinf.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            steps.done("the init and segments are the script's records");

            // A HEAD answers a GET's headers without its body.
            let last = &segments[segments.len() - 1].uri;
            let head = request(hls_addr, "HEAD", &format!("{dir}{last}")).await;
            assert_eq!(head.status, 200);
            head.check("a HEAD", "video/iso.segment", "max-age=31536000, immutable");
            assert_eq!(head.content_length(), media[media.len() - 1].len());
            assert!(head.body.is_empty());

            // Unknown streams and files answer 404, other methods 405.
            let unknown = format!("{profile_dir}{}/index.m3u8", "00".repeat(32));
            let nope = format!("{dir}nope.ts");
            for p in [unknown.as_str(), &nope, "/", "/favicon.ico"] {
                let r = request(hls_addr, "GET", p).await;
                assert_eq!(r.status, 404, "{p}");
                r.check(p, "text/plain; charset=utf-8", "no-store");
            }
            let post = request(hls_addr, "POST", path).await;
            assert_eq!(post.status, 405);
            assert_eq!(post.header("allow"), Some("GET, HEAD"));
            post.check("a POST", "text/plain; charset=utf-8", "no-store");
            steps.done("HEAD, 404s and 405");

            streaming.abort();
            alice.svc.stop().await;
        })
        .await
        .expect("the scenario took over 300 s");
    });
}

#[test]
fn a_service_without_an_hls_listener_supports_no_hls_egress() {
    run(async {
        let alice = Profile::sign_up(local_config()).await;
        assert_eq!(alice.svc.hls_addr(), None);
        assert_eq!(alice.svc.hls_base_url(), "");
        assert!(!alice.hls_supported().await);
        alice.set_hls(true).await;
        let uri = Uri::new(SwarmId::new(&[7; 32]), ingress_options().uri_options());
        let refused = alice
            .hls()
            .open_stream(&HlsEgressOpenStreamRequest {
                swarm_uri: uri.to_string(),
                network_keys: Vec::new(),
            })
            .await
            .unwrap_err();
        assert!(refused.to_string().contains("not supported"), "{refused}");
        alice.svc.stop().await;
    });
}

#[test]
fn an_hls_address_in_use_fails_the_start() {
    run(async {
        let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let config = Config {
            hls_listen: Some(taken.local_addr().unwrap().to_string()),
            ..local_config()
        };
        let Err(e) = Svc::start(config).await else {
            panic!("the service started on a taken HLS address");
        };
        assert!(matches!(e, SvcError::HlsListen(_)), "{e}");
    });
}
