// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HLS egress on fake nodes (see `strims_testworld`): a capture's fMP4
//! records served as a playlist, its init and segments, on the node's
//! routes; the playlist URL; the refusals, the stream a swarm shares, the
//! cap; streams closing when asked, when idle and when HLS is disabled;
//! records that aren't fMP4; and the UI's `HLSEgress` calls.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fmt::Debug;
use std::mem;

use strims_dao::Subscription;
use strims_kv::MemoryKv;
use strims_ppspp::{SwarmId, Uri};
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::profile::v1::Profile;
use strims_proto::service::CallError;
use strims_proto::video::v1::{
    HlsEgressClient, HlsEgressCloseStreamRequest, HlsEgressConfig, HlsEgressIsSupportedRequest,
    HlsEgressOpenStreamRequest,
};
use strims_rpc::{Router, Timer, connect};
use strims_testworld::*;
use strims_transfer::TransferId;
use strims_video::fmp4::{media_duration, parse_init, split_record};
use strims_video::hls::{HlsResponse, HlsRoutes, playlist_path};
use strims_video::synth::script::{Script, ScriptConfig, expected_records};
use strims_video::{
    Config, CutRule, HlsCloseReason, HlsSettings, NoVideoHost, VideoControl, VideoError,
    VideoEvent, bind_frontend,
};
use tokio::task::spawn_local;

type Control = VideoControl<MemoryKv, FakeHost, NoVideoHost, TokioTimer>;

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
    let video = VideoControl::new(
        n.host.clone(),
        NoVideoHost,
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

async fn set_hls(v: &Control, enabled: bool, public_server_addr: &str) {
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

/// The EXTINF line a segment of `record` is listed with: its media time
/// in milliseconds.
fn extinf(record: &[u8]) -> String {
    let (init, media) = split_record(record).unwrap();
    let d = media_duration(&parse_init(init).unwrap(), media).unwrap();
    let ms = (d.as_nanos() + 500_000) / 1_000_000;
    format!("#EXTINF:{}.{:03},", ms / 1000, ms % 1000)
}

fn records() -> Vec<Vec<u8>> {
    expected_records(&Script::new(&ScriptConfig::default()), CutRule::default())
}

/// Checks the playlist at `path` lists `records` in order, each served
/// with its init and media.
fn assert_served(n: &Node, routes: &HlsRoutes, path: &str, records: &[Vec<u8>]) {
    let segments = listed(&playlist(n, routes, path));
    assert_eq!(segments.len(), records.len());
    let dir = path.strip_suffix("index.m3u8").unwrap();
    for (i, (s, r)) in segments.iter().zip(records).enumerate() {
        assert_eq!(s.extinf, extinf(r), "segment {i}");
        assert!(!s.discontinuity, "segment {i}");
        let (init, media) = split_record(r).unwrap();
        let file = |name: &str| {
            let res = get(n, routes, &format!("{dir}{name}"));
            assert_eq!(res.status, 200, "{name}");
            res.body.to_vec()
        };
        assert_eq!(file(&s.init), init, "init of segment {i}");
        assert_eq!(file(&s.uri), media, "segment {i}");
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

/// A capture's fMP4 records reach another node's HLS stream: its playlist
/// lists each as a segment lasting its media time, served with its init
/// and media.
#[test]
fn a_captures_records_are_served_as_hls_on_another_node() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        let keys = vec![net.public.clone()];
        let va = start(&a, &HlsRoutes::new(), Config::default());
        let routes = HlsRoutes::new();
        let vb = start(&b, &routes, serving());
        assert!(vb.hls_supported());
        set_hls(&vb, true, "").await;
        let id = va
            .open_capture("video/mp4", keys.clone(), ListingSnippet::default())
            .await
            .unwrap();
        let uri = va.captures()[0].swarm_uri.clone();
        let mut events = vb.subscribe();

        let url = vb.open_hls(&uri, &keys).await.unwrap();
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

        let records = records();
        assert!((4..=6).contains(&records.len()), "{}", records.len());
        for r in &records {
            va.append_capture(&id, r.clone(), true).await.unwrap();
        }
        wait_for(30, "every segment", || {
            listed(&playlist(&b, &routes, path(&url))).len() == records.len()
        })
        .await;
        assert_served(&b, &routes, path(&url), &records);
        let m = vb.metrics();
        assert_eq!(
            (m.hls_streams, m.hls_segments, m.hls_skipped_records),
            (1, records.len() as u64, 0)
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

        let url = v.open_hls(&uri, &[]).await.unwrap();
        assert_eq!(url, format!("https://cdn.example.com/media{p}"));
        assert_eq!(v.open_hls(&uri, &[]).await.unwrap(), url);
        assert_eq!(v.hls_streams().len(), 1);
        assert_eq!(v.metrics().hls_streams, 1);
        let swarms = v.video_swarms();
        assert_eq!(swarms.len(), 1);
        assert_eq!((swarms[0].transfer_id, swarms[0].streams), (t, 1));

        set_hls(&v, true, "").await;
        settle().await;
        assert_eq!(v.open_hls(&uri, &[]).await.unwrap(), format!("{BASE}{p}"));
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
            refusal(v.open_hls(&uri, &[]).await),
            "HLS egress is not supported"
        );

        let n = node_in(&world, 2).await;
        let routes = HlsRoutes::new();
        let v = start(&n, &routes, serving());
        assert_eq!(
            refusal(v.open_hls(&uri, &[]).await),
            "HLS egress is disabled"
        );
        set_hls(&v, true, "").await;
        assert_eq!(
            refusal(v.open_hls("magnet:?xt=urn:ppspp:%%", &[]).await),
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
            refusal(v.open_hls(&uri, &[]).await),
            "HLS egress has no server address"
        );
        set_hls(&v, true, "http://hls.example.com").await;
        assert!(
            v.open_hls(&uri, &[])
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
            refusal(v.open_hls(&uri, &[]).await),
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
        let first = v.open_hls(&swarm_uri(1), &[]).await.unwrap();
        v.open_hls(&swarm_uri(2), &[]).await.unwrap();
        assert_eq!(
            v.open_hls(&swarm_uri(3), &[]).await,
            Err(VideoError::TooManyHlsStreams)
        );
        assert_eq!(v.open_hls(&swarm_uri(1), &[]).await.unwrap(), first);
        v.close_hls(&transfer_id(&swarm_uri(1))).unwrap();
        v.open_hls(&swarm_uri(3), &[]).await.unwrap();
        let open: Vec<TransferId> = v.hls_streams().iter().map(|s| s.transfer_id).collect();
        let mut expected = [transfer_id(&swarm_uri(2)), transfer_id(&swarm_uri(3))];
        expected.sort();
        assert_eq!(open, expected);
        settle().await;
        assert_eq!(v.video_swarms().len(), 2);
    });
}

/// Closing a stream unregisters its routes, so its GETs answer 404, and
/// releases its swarm; a capture it read goes on.
#[test]
fn a_closed_stream_answers_404_and_releases_its_swarm() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let (net, _) = owned_network(&n, &[], 10).await;
        let routes = HlsRoutes::new();
        let v = start(&n, &routes, serving());
        set_hls(&v, true, "").await;
        let id = v
            .open_capture(
                "video/mp4",
                vec![net.public.clone()],
                ListingSnippet::default(),
            )
            .await
            .unwrap();
        let uri = v.captures()[0].swarm_uri.clone();
        let url = v.open_hls(&uri, &[]).await.unwrap();
        let records = records();
        for r in &records[..2] {
            v.append_capture(&id, r.clone(), true).await.unwrap();
        }
        wait_for(10, "the segments", || {
            listed(&playlist(&n, &routes, path(&url))).len() == 2
        })
        .await;
        assert_served(&n, &routes, path(&url), &records[..2]);
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
        assert_eq!(v.video_swarms()[0].streams, 0);
        assert_eq!(v.captures().len(), 1);
        v.append_capture(&id, records[2].clone(), true)
            .await
            .unwrap();
    });
}

/// A stream whose swarm closes, here a capture on the node, ends with its
/// stream's error.
#[test]
fn a_stream_ends_when_its_swarm_closes() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let (net, _) = owned_network(&n, &[], 10).await;
        let routes = HlsRoutes::new();
        let v = start(&n, &routes, serving());
        set_hls(&v, true, "").await;
        let id = v
            .open_capture(
                "video/mp4",
                vec![net.public.clone()],
                ListingSnippet::default(),
            )
            .await
            .unwrap();
        let url = v.open_hls(&v.captures()[0].swarm_uri, &[]).await.unwrap();
        v.append_capture(&id, records().remove(0), true)
            .await
            .unwrap();
        wait_for(10, "the segment", || {
            listed(&playlist(&n, &routes, path(&url))).len() == 1
        })
        .await;
        let mut events = v.subscribe();

        v.close_capture(&id).await.unwrap();
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
        let url = v.open_hls(&uri, &[]).await.unwrap();
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
            v.open_hls(&swarm_uri(1), &[]).await.unwrap(),
            v.open_hls(&swarm_uri(2), &[]).await.unwrap(),
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
            v.open_hls(&swarm_uri(1), &[]).await,
            Err(VideoError::HlsDisabled)
        );
        settle().await;
        assert!(v.video_swarms().is_empty());

        set_hls(&v, true, "").await;
        assert_eq!(v.open_hls(&swarm_uri(1), &[]).await.unwrap(), urls[0]);
    });
}

/// Records that aren't fMP4, such as a capture's WebM, are skipped and
/// counted; the fMP4 records after them are served.
#[test]
fn records_that_are_not_fmp4_are_skipped() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let (net, _) = owned_network(&n, &[], 10).await;
        let routes = HlsRoutes::new();
        let v = start(&n, &routes, serving());
        set_hls(&v, true, "").await;
        let id = v
            .open_capture(
                "video/webm",
                vec![net.public.clone()],
                ListingSnippet::default(),
            )
            .await
            .unwrap();
        let url = v.open_hls(&v.captures()[0].swarm_uri, &[]).await.unwrap();
        for i in 0..3u8 {
            let mut webm = vec![0x1a, 0x45, 0xdf, 0xa3];
            webm.resize(70_000, i);
            v.append_capture(&id, webm, true).await.unwrap();
        }
        wait_for(10, "the records to be skipped", || {
            v.metrics().hls_skipped_records == 3
        })
        .await;
        assert!(listed(&playlist(&n, &routes, path(&url))).is_empty());

        let records = records();
        for r in &records[..2] {
            v.append_capture(&id, r.clone(), true).await.unwrap();
        }
        wait_for(10, "the segments", || {
            listed(&playlist(&n, &routes, path(&url))).len() == 2
        })
        .await;
        assert_served(&n, &routes, path(&url), &records[..2]);
        let m = v.metrics();
        assert_eq!((m.hls_segments, m.hls_skipped_records), (2, 3));
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
