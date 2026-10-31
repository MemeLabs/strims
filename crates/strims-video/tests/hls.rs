// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HLS egress on fake nodes (see `strims_testworld`): a unit-format
//! swarm's units served as a playlist, its inits and segments, on the
//! node's routes; a unit-format ingest served as segments within the
//! target, its long GOPs split; the playlist URL; the refusals, the
//! stream a swarm shares, the cap; streams closing when asked, when idle,
//! when HLS is disabled and when their swarm closes; and the UI's
//! `HLSEgress` calls.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::fmt::Debug;
use std::mem;
use std::time::Duration;

use strims_crypto::key_from_seed;
use strims_dao::Subscription;
use strims_kv::MemoryKv;
use strims_ppspp::{SwarmId, Uri, WriterOptions};
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::profile::v1::Profile;
use strims_proto::service::CallError;
use strims_proto::types::Key;
use strims_proto::video::v1::{
    HlsEgressClient, HlsEgressCloseStreamRequest, HlsEgressConfig, HlsEgressIsSupportedRequest,
    HlsEgressOpenStreamRequest, VideoChannel, VideoIngressConfig,
};
use strims_rpc::{Router, Timer, connect};
use strims_testworld::*;
use strims_transfer::{SwarmHost, SwarmWrite, TransferId};
use strims_video::fmp4::{media_duration, parse_init};
use strims_video::hls::{
    HlsMuxer, HlsMuxerConfig, HlsResponse, HlsRoutes, HlsSegment, playlist_path, target_secs,
};
use strims_video::synth::script::{Script, ScriptConfig, Tag, TagKind, mux_tags};
use strims_video::{
    Config, FlvSource, HlsCloseReason, HlsSettings, NoVideoHost, UnitWriter, VideoControl,
    VideoError, VideoEvent, VideoHost, bind_frontend, capture_options, check_format, units,
};
use support::{FakeVideo, TestPublisher};
use tokio::task::spawn_local;

type Control = VideoControl<MemoryKv, FakeHost, NoVideoHost, TokioTimer>;
type IngestControl = VideoControl<MemoryKv, FakeHost, FakeVideo, TokioTimer>;

const BASE: &str = "http://127.0.0.1:8082";

/// A config whose driver serves HLS at [`BASE`].
fn serving() -> Config {
    Config {
        hls: HlsSettings {
            serve: true,
            base_url: format!("{BASE}/"),
            ..HlsSettings::default()
        },
        ..Config::default()
    }
}

fn start(n: &Node, routes: &HlsRoutes, config: Config) -> Control {
    start_as(n, n.profile.clone(), routes, config)
}

fn start_as(n: &Node, profile: Profile, routes: &HlsRoutes, config: Config) -> Control {
    start_with(n, profile, NoVideoHost, routes, config)
}

fn start_with<V: VideoHost>(
    n: &Node,
    profile: Profile,
    host: V,
    routes: &HlsRoutes,
    config: Config,
) -> VideoControl<MemoryKv, FakeHost, V, TokioTimer> {
    let video = VideoControl::new(
        n.host.clone(),
        host,
        n.store.clone(),
        profile,
        n.network.clone(),
        n.transfers.clone(),
        n.directory.clone(),
        routes.clone(),
        n.timer.clone(),
        n.rng(),
        config,
    );
    let v = video.clone();
    n.spawn(async move { v.run().await });
    video
}

async fn set_hls<V: VideoHost>(
    v: &VideoControl<MemoryKv, FakeHost, V, TokioTimer>,
    enabled: bool,
    public_server_addr: &str,
) {
    let c = HlsEgressConfig {
        enabled,
        public_server_addr: public_server_addr.into(),
    };
    v.set_hls_config(c).await.unwrap();
}

/// A swarm no one writes.
fn swarm_uri(i: u8) -> String {
    format!("magnet:?xt=urn:ppspp:{}", SwarmId::new(&[i; 32]))
}

fn transfer_id(uri: &str) -> TransferId {
    TransferId::new(&Uri::parse(uri).unwrap().id, b"")
}

/// The path of a playlist URL at [`BASE`].
fn path(url: &str) -> &str {
    url.strip_prefix(BASE).unwrap()
}

/// The routes' answer to a GET of `path` now.
fn get(n: &Node, routes: &HlsRoutes, path: &str) -> HlsResponse {
    routes.get(n.timer.now(), path)
}

fn playlist(n: &Node, routes: &HlsRoutes, path: &str) -> String {
    let r = get(n, routes, path);
    assert_eq!(r.status, 200, "{path}");
    String::from_utf8(r.body.to_vec()).unwrap()
}

/// A listed segment: its init's URI, EXTINF line and URI, and whether a
/// discontinuity precedes it.
#[derive(Debug, Clone, PartialEq)]
struct Listed {
    init: String,
    extinf: String,
    uri: String,
    discontinuity: bool,
}

fn listed(playlist: &str) -> Vec<Listed> {
    let mut out = Vec::new();
    let (mut init, mut extinf, mut discontinuity) = (String::new(), None, false);
    for line in playlist.lines() {
        if let Some(uri) = line.strip_prefix("#EXT-X-MAP:URI=") {
            init = uri.trim_matches('"').to_owned();
        } else if line == "#EXT-X-DISCONTINUITY" {
            discontinuity = true;
        } else if line.starts_with("#EXTINF:") {
            extinf = Some(line.to_owned());
        } else if !line.starts_with('#') {
            out.push(Listed {
                init: init.clone(),
                extinf: extinf.take().unwrap(),
                uri: line.to_owned(),
                discontinuity: mem::take(&mut discontinuity),
            });
        }
    }
    out
}

/// The EXTINF line a segment lasting `d` is listed with: its media time
/// in milliseconds.
fn extinf(d: strims_base::time::Duration) -> String {
    let ms = (d.as_nanos() + 500_000) / 1_000_000;
    format!("#EXTINF:{}.{:03},", ms / 1000, ms % 1000)
}

/// The segments an HLS stream of `tags`, a script's, read from its start
/// under the default settings makes: their units through an [`HlsMuxer`].
fn segments(tags: &[Tag]) -> Vec<HlsSegment> {
    let s = HlsSettings::default();
    let mut muxer = HlsMuxer::new(HlsMuxerConfig {
        target: target_secs(s.target_duration),
        fragment_duration: s.fragment_duration,
        max_segment_bytes: s.max_segment_bytes,
    });
    let mut source = FlvSource::new();
    let mut out = Vec::new();
    for tag in tags {
        mux_tags(std::slice::from_ref(tag), &mut source, &mut muxer);
        out.extend(std::iter::from_fn(|| muxer.take_segment()));
    }
    out
}

/// The tags of `script` up to and including its first video tag at
/// `ms` or later: those that finish the segments before `ms`.
fn tags_to(script: &Script, ms: u32) -> &[Tag] {
    let i = script
        .tags
        .iter()
        .position(|t| t.kind == TagKind::Video && t.timestamp >= ms)
        .unwrap();
    &script.tags[..=i]
}

/// Checks the playlist at `path` lists `segments` in order, each served
/// with its init and media.
fn assert_served(n: &Node, routes: &HlsRoutes, path: &str, segments: &[HlsSegment]) {
    let listed = listed(&playlist(n, routes, path));
    assert_eq!(listed.len(), segments.len());
    let dir = path.strip_suffix("index.m3u8").unwrap();
    for (i, (l, s)) in listed.iter().zip(segments).enumerate() {
        assert_eq!(l.extinf, extinf(s.duration), "segment {i}");
        assert!(!l.discontinuity, "segment {i}");
        let file = |name: &str| {
            let res = get(n, routes, &format!("{dir}{name}"));
            assert_eq!(res.status, 200, "{name}");
            res.body.to_vec()
        };
        assert_eq!(file(&l.init), *s.init, "init of segment {i}");
        assert_eq!(file(&l.uri), s.media, "segment {i}");
    }
}

/// The HLS stream events a subscription holds now.
fn hls_events(events: &mut Subscription<VideoEvent>) -> Vec<VideoEvent> {
    drain(events)
        .into_iter()
        .filter(|e| {
            matches!(
                e,
                VideoEvent::HlsStreamOpened { .. } | VideoEvent::HlsStreamClosed { .. }
            )
        })
        .collect()
}

fn closed(t: TransferId, reason: HlsCloseReason) -> VideoEvent {
    VideoEvent::HlsStreamClosed {
        transfer_id: t,
        reason,
    }
}

/// A swarm written on a node outside the controls in the unit format: a
/// script's units through a [`UnitWriter`] in frames of its chunk size.
struct Publisher {
    id: SwarmId,
    uri: String,
    source: FlvSource,
    units: UnitWriter,
    writer: FakeWriter,
}

impl Publisher {
    async fn new(n: &Node, seed: u8) -> Publisher {
        let options = capture_options();
        let (_, id, writer) = n
            .host
            .create_swarm(WriterOptions {
                swarm_options: options.clone(),
                key: key_from_seed(&[seed; 32]),
            })
            .await
            .unwrap();
        let uri = Uri::new(id.clone(), options.uri_options()).to_string();
        Publisher {
            id,
            uri,
            source: FlvSource::new(),
            units: UnitWriter::new(options.chunk_size).unwrap(),
            writer,
        }
    }

    fn transfer_id(&self) -> TransferId {
        TransferId::new(&self.id, b"")
    }

    /// Writes the units of `tags`, a script's, as each tag comes, with a
    /// pause after each.
    async fn tags(&mut self, tags: &[Tag]) {
        for tag in tags {
            mux_tags(std::slice::from_ref(tag), &mut self.source, &mut self.units);
            let out = self.units.take_output();
            if !out.is_empty() {
                self.writer.write(&out).await.unwrap();
            }
            self.units.restore(out);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
}

const INGEST_ADDR: &str = "127.0.0.1:1935";

/// Creates a channel on network `net` and enables the ingress: the
/// channel's stream key.
async fn open_ingest(v: &IngestControl, video: &FakeVideo, net: &Key) -> String {
    let snippet = ListingSnippet {
        title: "hls".into(),
        ..Default::default()
    };
    let channel: VideoChannel = v.create_channel(&net.public, snippet).await.unwrap();
    v.set_ingress_config(VideoIngressConfig {
        enabled: true,
        server_addr: INGEST_ADDR.into(),
        ..Default::default()
    })
    .await
    .unwrap();
    wait_for(10, "the listener", || {
        video.listening().as_deref() == Some(INGEST_ADDR)
    })
    .await;
    v.channel_url(channel.id).await.unwrap().stream_key
}

/// An RTMP publisher of stream key `key`, publishing.
async fn publish(n: &Node, video: &FakeVideo, key: &str) -> TestPublisher {
    let mut p = TestPublisher::connect(video, "10.0.0.1:1", "live", key, &mut n.rng());
    p.answered().await;
    assert!(p.publisher.publish_accepted());
    p
}

/// The media listings of network 10's view on `n`.
fn media(n: &Node) -> Vec<Media> {
    n.directory
        .listings(10)
        .into_iter()
        .filter_map(|l| match l.listing.content {
            Some(Content::Media(m)) => Some(m),
            _ => None,
        })
        .collect()
}

/// The milliseconds of a listed segment's EXTINF.
fn extinf_millis(extinf: &str) -> u64 {
    let x = extinf
        .strip_prefix("#EXTINF:")
        .unwrap()
        .trim_end_matches(',');
    let (s, f) = x.split_once('.').unwrap();
    s.parse::<u64>().unwrap() * 1000 + f.parse::<u64>().unwrap()
}

/// A unit-format ingest's HLS stream on another node.
struct UnitsHls {
    /// The viewing node, its routes and the playlist's path.
    b: Node,
    routes: HlsRoutes,
    path: String,
    _keep: (Vec<Pipe>, IngestControl, Control),
}

/// A unit-format ingest on one node, its HLS stream opened on another
/// through the UI's `HLSEgress` with the listing's MIME type, and the
/// script published until the playlist lists `segments`.
async fn units_hls(world: &World, script: &Script, segments: usize) -> UnitsHls {
    let a = node_in(world, 1).await;
    let b = node_in(world, 2).await;
    let (net, pipes) = owned_network(&a, &[&b], 10).await;
    let video = FakeVideo::default();
    let va = start_with(
        &a,
        a.profile.clone(),
        video.clone(),
        &HlsRoutes::new(),
        Config::default(),
    );
    let routes = HlsRoutes::new();
    let vb = start(&b, &routes, serving());
    set_hls(&vb, true, "").await;
    let key = open_ingest(&va, &video, &net).await;
    let mut p = publish(&a, &video, &key).await;
    p.send_tags(&script.tags[..16], script).await;
    wait_for(10, "the listing", || !media(&b).is_empty()).await;
    let m = media(&b).remove(0);
    assert_eq!(check_format(&m.mime_type), Ok(()));

    let router = Router::new();
    bind_frontend(&router, &vb);
    let hls = HlsEgressClient::new(frontend_client(router));
    let url = hls
        .open_stream(&HlsEgressOpenStreamRequest {
            swarm_uri: m.swarm_uri.clone(),
            network_keys: vec![net.public.clone()],
            mime_type: m.mime_type.clone(),
        })
        .await
        .unwrap()
        .playlist_url;
    p.send_tags(&script.tags[16..], script).await;
    let path = path(&url).to_owned();
    wait_for(30, "the segments", || {
        listed(&playlist(&b, &routes, &path)).len() >= segments
    })
    .await;
    UnitsHls {
        b,
        routes,
        path,
        _keep: (pipes, va, vb),
    }
}

/// A unit-format swarm's units reach another node's HLS stream: its
/// playlist lists the segments an HLS muxer cuts of them, each lasting its
/// media time, served with its init and media.
#[test]
fn a_unit_format_swarm_is_served_as_hls_on_another_node() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        let keys = vec![net.public.clone()];
        let routes = HlsRoutes::new();
        let vb = start(&b, &routes, serving());
        assert!(vb.hls_supported());
        set_hls(&vb, true, "").await;
        let mut p = Publisher::new(&a, 7).await;
        let id = p.transfer_id();
        let mut events = vb.subscribe();

        let url = vb.open_hls(&p.uri, &keys, units::MIME_TYPE).await.unwrap();
        let key = &b.profile_key().public;
        assert_eq!(
            url,
            format!(
                "{BASE}/hls/{}/{}/index.m3u8",
                hex::encode(key),
                hex::encode(id.as_bytes())
            )
        );
        assert_eq!(path(&url), playlist_path(key, id.as_bytes()));
        assert_eq!(
            hls_events(&mut events),
            [VideoEvent::HlsStreamOpened {
                transfer_id: id,
                playlist_url: url.clone(),
            }]
        );
        assert!(listed(&playlist(&b, &routes, path(&url))).is_empty());
        let info = vb.hls_streams();
        assert_eq!(info.len(), 1);
        assert_eq!((info[0].transfer_id, info[0].reading), (id, true));
        assert_eq!(info[0].path, path(&url));

        // 2 s GOPs over 10 s: the last GOP's segment waits for a keyframe.
        let script = Script::new(&ScriptConfig::default());
        let expected = segments(&script.tags);
        assert_eq!(expected.len(), 4);
        p.tags(&script.tags).await;
        wait_for(30, "every segment", || {
            listed(&playlist(&b, &routes, path(&url))).len() == expected.len()
        })
        .await;
        assert_served(&b, &routes, path(&url), &expected);
        let m = vb.metrics();
        assert_eq!(
            (m.hls_streams, m.hls_segments, m.hls_skipped_records),
            (1, expected.len() as u64, 0)
        );
        assert!(b.transfers.is_published(id, b.net(&net.public)));
    });
}

/// The playlist URL starts with the config's public server address, or
/// the driver's base URL without one; a swarm has one HLS stream, which
/// opening it again answers, whatever address the config names now.
#[test]
fn a_swarm_has_one_hls_stream_under_the_configs_address() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let routes = HlsRoutes::new();
        let v = start(&n, &routes, serving());
        set_hls(&v, true, "https://cdn.example.com/media/").await;
        let uri = swarm_uri(7);
        let t = transfer_id(&uri);
        let p = playlist_path(&n.profile_key().public, t.as_bytes());

        let url = v.open_hls(&uri, &[], units::MIME_TYPE).await.unwrap();
        assert_eq!(url, format!("https://cdn.example.com/media{p}"));
        assert_eq!(v.open_hls(&uri, &[], units::MIME_TYPE).await.unwrap(), url);
        assert_eq!(v.hls_streams().len(), 1);
        assert_eq!(v.metrics().hls_streams, 1);
        let swarms = v.video_swarms();
        assert_eq!(swarms.len(), 1);
        assert_eq!((swarms[0].transfer_id, swarms[0].streams), (t, 1));

        set_hls(&v, true, "").await;
        settle().await;
        assert_eq!(
            v.open_hls(&uri, &[], units::MIME_TYPE).await.unwrap(),
            format!("{BASE}{p}")
        );
        assert_eq!(v.hls_streams().len(), 1);
        assert_eq!(v.metrics().hls_streams, 1);
        assert!(playlist(&n, &routes, &p).starts_with("#EXTM3U\n"));
    });
}

/// A stream opens only where the driver serves HLS, the profile's key
/// routes, the config is enabled and an address is set, and for a valid
/// URI; a refused open registers nothing.
#[test]
fn hls_streams_open_only_where_served_enabled_and_addressed() {
    run(async {
        let world = World::default();
        let refusal = |r: Result<String, VideoError>| r.unwrap_err().to_string();
        let uri = swarm_uri(7);

        let n = node_in(&world, 1).await;
        let v = start(&n, &HlsRoutes::new(), Config::default());
        set_hls(&v, true, "https://cdn.example.com").await;
        assert!(!v.hls_supported());
        assert_eq!(
            refusal(v.open_hls(&uri, &[], units::MIME_TYPE).await),
            "HLS egress is not supported"
        );

        let n = node_in(&world, 2).await;
        let routes = HlsRoutes::new();
        let v = start(&n, &routes, serving());
        assert_eq!(
            refusal(v.open_hls(&uri, &[], units::MIME_TYPE).await),
            "HLS egress is disabled"
        );
        set_hls(&v, true, "").await;
        assert_eq!(
            refusal(
                v.open_hls(&uri, &[], r#"video/mp4;codecs="avc1.64001f""#)
                    .await
            ),
            r#"unsupported stream format "video/mp4": the node reads video/x.strims-units streams"#
        );
        assert_eq!(
            refusal(
                v.open_hls("magnet:?xt=urn:ppspp:%%", &[], units::MIME_TYPE)
                    .await
            ),
            "invalid swarm uri: invalid uri"
        );
        assert!(v.hls_streams().is_empty());
        assert!(v.video_swarms().is_empty());

        let n = node_in(&world, 3).await;
        let config = Config {
            hls: HlsSettings {
                base_url: String::new(),
                ..serving().hls
            },
            ..Config::default()
        };
        let v = start(&n, &HlsRoutes::new(), config);
        set_hls(&v, true, "").await;
        assert_eq!(
            refusal(v.open_hls(&uri, &[], units::MIME_TYPE).await),
            "HLS egress has no server address"
        );
        set_hls(&v, true, "http://hls.example.com").await;
        assert!(
            v.open_hls(&uri, &[], units::MIME_TYPE)
                .await
                .unwrap()
                .starts_with("http://hls.example.com/hls/")
        );

        let n = node_in(&world, 4).await;
        let mut profile = n.profile.clone();
        profile.key.as_mut().unwrap().public.truncate(31);
        let routes = HlsRoutes::new();
        let v = start_as(&n, profile, &routes, serving());
        set_hls(&v, true, "").await;
        assert!(!v.hls_supported());
        assert_eq!(routes.profiles(), 0);
        assert_eq!(
            refusal(v.open_hls(&uri, &[], units::MIME_TYPE).await),
            "HLS egress is unavailable: HLS profile keys are 32 bytes"
        );
    });
}

/// The node serves at most `max_streams` HLS streams; opening a served
/// swarm again isn't another, and a closed stream frees its place.
#[test]
fn hls_streams_are_bounded() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let mut config = serving();
        config.hls.max_streams = 2;
        let v = start(&n, &HlsRoutes::new(), config);
        set_hls(&v, true, "").await;
        let first = v
            .open_hls(&swarm_uri(1), &[], units::MIME_TYPE)
            .await
            .unwrap();
        v.open_hls(&swarm_uri(2), &[], units::MIME_TYPE)
            .await
            .unwrap();
        assert_eq!(
            v.open_hls(&swarm_uri(3), &[], units::MIME_TYPE).await,
            Err(VideoError::TooManyHlsStreams)
        );
        assert_eq!(
            v.open_hls(&swarm_uri(1), &[], units::MIME_TYPE)
                .await
                .unwrap(),
            first
        );
        v.close_hls(&transfer_id(&swarm_uri(1))).unwrap();
        v.open_hls(&swarm_uri(3), &[], units::MIME_TYPE)
            .await
            .unwrap();
        let open: Vec<TransferId> = v.hls_streams().iter().map(|s| s.transfer_id).collect();
        let mut expected = [transfer_id(&swarm_uri(2)), transfer_id(&swarm_uri(3))];
        expected.sort();
        assert_eq!(open, expected);
        settle().await;
        assert_eq!(v.video_swarms().len(), 2);
    });
}

/// Closing a stream unregisters its routes, so its GETs answer 404, and
/// releases its swarm; the swarm's writer goes on.
#[test]
fn a_closed_stream_answers_404_and_releases_its_swarm() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let routes = HlsRoutes::new();
        let v = start(&n, &routes, serving());
        set_hls(&v, true, "").await;
        let mut p = Publisher::new(&n, 7).await;
        let id = p.transfer_id();
        let url = v.open_hls(&p.uri, &[], units::MIME_TYPE).await.unwrap();
        let script = Script::new(&ScriptConfig::default());
        let first = tags_to(&script, 4000);
        let expected = segments(first);
        assert_eq!(expected.len(), 2);
        p.tags(first).await;
        wait_for(10, "the segments", || {
            listed(&playlist(&n, &routes, path(&url))).len() == 2
        })
        .await;
        assert_served(&n, &routes, path(&url), &expected);
        let segment = listed(&playlist(&n, &routes, path(&url))).remove(0);
        let dir = path(&url).strip_suffix("index.m3u8").unwrap();
        assert_eq!(v.video_swarms()[0].streams, 1);
        let mut events = v.subscribe();

        v.close_hls(&id).unwrap();
        assert!(v.hls_streams().is_empty());
        for p in [
            path(&url).to_owned(),
            format!("{dir}{}", segment.uri),
            format!("{dir}{}", segment.init),
        ] {
            assert_eq!(get(&n, &routes, &p).status, 404, "{p}");
        }
        assert_eq!(
            hls_events(&mut events),
            [closed(id, HlsCloseReason::Closed)]
        );
        assert_eq!(v.close_hls(&id), Err(VideoError::HlsStreamNotFound));
        settle().await;
        assert!(v.video_swarms().is_empty());
        p.tags(&script.tags[first.len()..]).await;
    });
}

/// A stream whose swarm closes, here an ingest session's on the node,
/// ends with its stream's error.
#[test]
fn a_stream_ends_when_its_swarm_closes() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let (net, _pipes) = owned_network(&n, &[], 10).await;
        let video = FakeVideo::default();
        let routes = HlsRoutes::new();
        let v = start_with(&n, n.profile.clone(), video.clone(), &routes, serving());
        set_hls(&v, true, "").await;
        let key = open_ingest(&v, &video, &net).await;
        let mut p = publish(&n, &video, &key).await;
        let script = Script::new(&ScriptConfig::default());
        let half = script.tags.len() / 2;
        p.send_tags(&script.tags[..half], &script).await;
        wait_for(10, "the listing", || !media(&n).is_empty()).await;
        let m = media(&n).remove(0);
        let id = transfer_id(&m.swarm_uri);
        let url = v.open_hls(&m.swarm_uri, &[], &m.mime_type).await.unwrap();
        p.send_tags(&script.tags[half..], &script).await;
        wait_for(10, "a segment", || {
            !listed(&playlist(&n, &routes, path(&url))).is_empty()
        })
        .await;
        let mut events = v.subscribe();

        p.unpublish().await;
        wait_for(10, "the stream to end", || v.hls_streams().is_empty()).await;
        assert_eq!(get(&n, &routes, path(&url)).status, 404);
        assert_eq!(
            hls_events(&mut events),
            [closed(id, HlsCloseReason::Failed(VideoError::StreamClosed))]
        );
    });
}

/// A stream no request reaches for the idle timeout closes, at the next
/// sweep; requests keep it open.
#[test]
fn an_idle_stream_closes() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let routes = HlsRoutes::new();
        let v = start(&n, &routes, serving());
        set_hls(&v, true, "").await;
        let uri = swarm_uri(7);
        let t = transfer_id(&uri);
        let url = v.open_hls(&uri, &[], units::MIME_TYPE).await.unwrap();
        let mut events = v.subscribe();

        for _ in 0..4 {
            sleep_secs(45).await;
            playlist(&n, &routes, path(&url));
        }
        assert_eq!(v.hls_streams().len(), 1);
        let last = n.timer.now();
        assert_eq!(v.hls_streams()[0].last_access, last);
        sleep_secs(55).await;
        assert_eq!(v.hls_streams().len(), 1);
        sleep_secs(16).await;
        assert!(v.hls_streams().is_empty());
        assert_eq!(get(&n, &routes, path(&url)).status, 404);
        assert_eq!(hls_events(&mut events), [closed(t, HlsCloseReason::Idle)]);
        settle().await;
        assert!(v.video_swarms().is_empty());
    });
}

/// Disabling HLS closes every stream, and no stream opens until it is
/// enabled again.
#[test]
fn disabling_hls_closes_every_stream() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let routes = HlsRoutes::new();
        let v = start(&n, &routes, serving());
        set_hls(&v, true, "").await;
        let urls = [
            v.open_hls(&swarm_uri(1), &[], units::MIME_TYPE)
                .await
                .unwrap(),
            v.open_hls(&swarm_uri(2), &[], units::MIME_TYPE)
                .await
                .unwrap(),
        ];
        let mut events = v.subscribe();

        set_hls(&v, false, "").await;
        wait_for(10, "the streams to close", || v.hls_streams().is_empty()).await;
        for url in &urls {
            assert_eq!(get(&n, &routes, path(url)).status, 404);
        }
        let mut expected = [
            closed(transfer_id(&swarm_uri(1)), HlsCloseReason::Disabled),
            closed(transfer_id(&swarm_uri(2)), HlsCloseReason::Disabled),
        ];
        let mut got = hls_events(&mut events);
        let order = |e: &VideoEvent| match e {
            VideoEvent::HlsStreamClosed { transfer_id, .. } => *transfer_id,
            _ => TransferId::default(),
        };
        got.sort_by_key(order);
        expected.sort_by_key(order);
        assert_eq!(got, expected);
        assert_eq!(
            v.open_hls(&swarm_uri(1), &[], units::MIME_TYPE).await,
            Err(VideoError::HlsDisabled)
        );
        settle().await;
        assert!(v.video_swarms().is_empty());

        set_hls(&v, true, "").await;
        assert_eq!(
            v.open_hls(&swarm_uri(1), &[], units::MIME_TYPE)
                .await
                .unwrap(),
            urls[0]
        );
    });
}

/// A unit-format ingest's access units are muxed into segments cut at
/// its keyframes, each lasting its GOP, under a fixed target, with an
/// init declaring both tracks.
#[test]
fn a_unit_format_ingest_is_served_as_segments_cut_at_its_keyframes() {
    run(async {
        let world = World::default();
        // 2 s GOPs over 12 s.
        let script = Script::new(&ScriptConfig {
            duration_secs: 12,
            ..ScriptConfig::default()
        });
        let UnitsHls {
            b, routes, path, ..
        } = units_hls(&world, &script, 5).await;
        let text = playlist(&b, &routes, &path);
        assert!(text.contains("#EXT-X-TARGETDURATION:4\n"), "{text}");
        assert!(!text.contains("INDEPENDENT"), "{text}");
        let segments = listed(&text);
        assert!(segments.len() >= 5, "{text}");
        let dir = path.strip_suffix("index.m3u8").unwrap();
        let init = get(&b, &routes, &format!("{dir}{}", segments[0].init)).body;
        let info = parse_init(&init).unwrap();
        assert_eq!(info.tracks().len(), 2);
        for s in &segments {
            assert_eq!(s.extinf, "#EXTINF:2.000,", "{text}");
            assert!(!s.discontinuity, "{text}");
            let media = get(&b, &routes, &format!("{dir}{}", s.uri)).body;
            assert_eq!(
                media_duration(&info, &media).unwrap(),
                strims_base::time::Duration::from_secs(2)
            );
        }
    });
}

/// A unit-format ingest's 10 s GOPs are split into segments that round
/// to at most the 4 s target.
#[test]
fn a_unit_format_ingests_long_gops_are_split_within_the_target() {
    run(async {
        let world = World::default();
        let script = Script::new(&ScriptConfig {
            gop_frames: 300,
            duration_secs: 24,
            ..ScriptConfig::default()
        });
        let UnitsHls {
            b, routes, path, ..
        } = units_hls(&world, &script, 5).await;
        let text = playlist(&b, &routes, &path);
        let millis: Vec<u64> = listed(&text)
            .iter()
            .map(|s| extinf_millis(&s.extinf))
            .collect();
        assert!(millis.len() >= 5, "{text}");
        assert!(millis.iter().all(|&m| (m + 500) / 1000 <= 4), "{text}");
        // Each 10 s GOP: two segments of 134 frames and the rest.
        assert!(
            millis[..2].iter().all(|m| (4450..4480).contains(m)),
            "{text}"
        );
        assert_eq!(millis[..3].iter().sum::<u64>(), 10_000, "{text}");
        assert!(text.contains("#EXT-X-TARGETDURATION:4\n"));
        assert!(!text.contains("INDEPENDENT"));
    });
}

/// The message of a call's remote error.
fn remote<T: Debug>(r: Result<T, CallError>) -> String {
    match r {
        Err(CallError::Remote(e)) => e.message,
        r => panic!("not refused: {r:?}"),
    }
}

/// `HLSEgress` opens a stream at its playlist URL, and closes it by its
/// transfer id; the stream outlives the connection that opened it.
#[test]
fn the_frontend_opens_and_closes_streams() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let routes = HlsRoutes::new();
        let v = start(&n, &routes, serving());
        set_hls(&v, true, "").await;

        // A connection whose server task can be aborted.
        let router = Router::new();
        bind_frontend(&router, &v);
        let (up, down) = (Pipe::default(), Pipe::default());
        let rpc = strims_rpc::Config::frontend();
        let (_, server) = connect(
            rpc.clone(),
            Reader(up.clone()),
            Writer(down.clone()),
            Some(router),
        );
        let (client, conn) = connect(rpc, Reader(down), Writer(up), None);
        let server = spawn_local(async move {
            let _ = server.await;
        });
        spawn_local(async move {
            let _ = conn.await;
        });
        let hls = HlsEgressClient::new(client);
        let supported = hls.is_supported(&HlsEgressIsSupportedRequest {});
        assert!(supported.await.unwrap().supported);

        let uri = swarm_uri(7);
        let open = |swarm_uri: &str| HlsEgressOpenStreamRequest {
            swarm_uri: swarm_uri.into(),
            network_keys: Vec::new(),
            mime_type: units::MIME_TYPE.into(),
        };
        let url = hls.open_stream(&open(&uri)).await.unwrap().playlist_url;
        let t = transfer_id(&uri);
        assert_eq!(
            url,
            format!(
                "{BASE}{}",
                playlist_path(&n.profile_key().public, t.as_bytes())
            )
        );
        assert_eq!(get(&n, &routes, path(&url)).status, 200);
        assert_eq!(
            remote(hls.open_stream(&open("not a uri")).await),
            "invalid swarm uri: invalid uri"
        );
        let mp4 = HlsEgressOpenStreamRequest {
            mime_type: "video/mp4".into(),
            ..open(&swarm_uri(8))
        };
        assert_eq!(
            remote(hls.open_stream(&mp4).await),
            r#"unsupported stream format "video/mp4": the node reads video/x.strims-units streams"#
        );
        let close = |id: &[u8]| HlsEgressCloseStreamRequest {
            transfer_id: id.to_vec(),
        };
        hls.close_stream(&close(t.as_bytes())).await.unwrap();
        assert_eq!(get(&n, &routes, path(&url)).status, 404);
        assert!(v.hls_streams().is_empty());
        let not_found = "HLS stream not found";
        assert_eq!(
            remote(hls.close_stream(&close(t.as_bytes())).await),
            not_found
        );
        assert_eq!(remote(hls.close_stream(&close(&[1; 5])).await), not_found);

        let url = hls.open_stream(&open(&uri)).await.unwrap().playlist_url;
        server.abort();
        let _ = server.await;
        settle().await;
        assert_eq!(v.hls_streams().len(), 1);
        assert_eq!(get(&n, &routes, path(&url)).status, 200);
    });
}
