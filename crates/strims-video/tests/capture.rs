// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's captures on fake nodes (see `strims_testworld`), through its
//! RPC connection: decoder configs and access units another node's stream
//! reads back as the unit format, the listings and snippet, early and
//! final signing, the checks and bounds on a capture, and captures
//! belonging to their connection.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fmt::Debug;
use std::time::Duration;

use futures_util::StreamExt;
use strims_dao::Table;
use strims_dao::schema::network::Networks;
use strims_kv::MemoryKv;
use strims_ppspp::Uri;
use strims_proto::network::v1::Network;
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::service::CallError;
use strims_proto::video::v1::capture_config::Codec;
use strims_proto::video::v1::egress_open_stream_response::Body;
use strims_proto::video::v1::{
    CaptureAppendRequest, CaptureClient, CaptureCloseRequest, CaptureConfig, CaptureOpenRequest,
    CaptureUnit, CaptureUpdateRequest, EgressClient, EgressOpenStreamRequest,
    EgressOpenStreamResponse,
};
use strims_rpc::{Client, ClientStream, Router, Timer, connect};
use strims_testworld::*;
use strims_transfer::TransferId;
use strims_video::fmp4::{FragmentWriter, parse_init, write_init};
use strims_video::hls::HlsRoutes;
use strims_video::synth::script::avcc;
use strims_video::units::{MIME_TYPE, ReaderEvent, UnitReader, UnitSigning};
use strims_video::{
    CaptureSettings, CodecParams, Config, DecoderConfig, NoVideoHost, SwarmKind, TrackId,
    VideoControl, bind_frontend, capture_options,
};
use tokio::task::{JoinHandle, spawn_local};

type Control = VideoControl<MemoryKv, FakeHost, NoVideoHost, TokioTimer>;

/// The OpusHead every WebCodecs encoder tested gives for stereo: version
/// 1, pre-skip 312, 48 kHz, family 0.
const OPUS_HEAD: [u8; 19] = *b"OpusHead\x01\x02\x38\x01\x80\xbb\0\0\0\0\0";
/// Chromium's OpenH264 avcC for 1280x720: Constrained High at level 3.1.
const CHROMIUM_AVCC: &str =
    "01640c1fffe1001367640c1fac18d00a00b74dc0434041e1108d4001000468ce3c80fdf8f800";
const VIDEO: u32 = 1;
const AUDIO: u32 = 2;
/// A capture's nominal bitrate with video: 2.5 Mbit/s of H.264 and
/// 128 kbit/s of Opus.
const BITRATE: u64 = 2_628_000;

fn start(n: &Node, config: Config) -> Control {
    let video = VideoControl::new(
        n.host.clone(),
        NoVideoHost,
        n.store.clone(),
        n.profile.clone(),
        n.network.clone(),
        n.transfers.clone(),
        n.directory.clone(),
        HlsRoutes::new(),
        n.timer.clone(),
        n.rng(),
        config,
    );
    let v = video.clone();
    n.spawn(async move { v.run().await });
    video
}

/// A UI's connection to `video`'s frontend: its clients, and the server's
/// task, whose abort closes the connection.
struct Ui {
    capture: CaptureClient<Client>,
    egress: EgressClient<Client>,
    server: JoinHandle<()>,
}

fn connect_ui(video: &Control) -> Ui {
    let router = Router::new();
    bind_frontend(&router, video);
    let (up, down) = (Pipe::default(), Pipe::default());
    let config = strims_rpc::Config::frontend();
    let (_, server) = connect(
        config.clone(),
        Reader(up.clone()),
        Writer(down.clone()),
        Some(router),
    );
    let (client, conn) = connect(config, Reader(down), Writer(up), None);
    let server = spawn_local(async move {
        let _ = server.await;
    });
    spawn_local(async move {
        let _ = conn.await;
    });
    Ui {
        capture: CaptureClient::new(client.clone()),
        egress: EgressClient::new(client),
        server,
    }
}

fn snippet(title: &str) -> ListingSnippet {
    ListingSnippet {
        title: title.into(),
        description: "desc".into(),
        channel_name: "chan".into(),
        live: true,
        ..Default::default()
    }
}

/// The message of a call's remote error.
fn refusal<T: Debug>(r: Result<T, CallError>) -> String {
    match r {
        Err(CallError::Remote(e)) => e.message,
        r => panic!("not refused: {r:?}"),
    }
}

fn h264(avcc: Vec<u8>) -> CaptureConfig {
    CaptureConfig {
        track: VIDEO,
        codec: Codec::H264 as i32,
        config: avcc,
    }
}

fn opus() -> CaptureConfig {
    CaptureConfig {
        track: AUDIO,
        codec: Codec::Opus as i32,
        config: OPUS_HEAD.to_vec(),
    }
}

/// The configs of a 640x360 H.264 track and a stereo Opus track.
fn configs() -> Vec<CaptureConfig> {
    vec![h264(avcc(640, 360)), opus()]
}

/// A unit of `n` bytes, its first naming its track and time.
fn unit(track: u32, timestamp_us: i64, keyframe: bool, n: usize) -> CaptureUnit {
    let mut data: Vec<u8> = (0..n)
        .map(|j| (timestamp_us as usize / 1000).wrapping_add(j) as u8)
        .collect();
    data[0] = track as u8;
    CaptureUnit {
        track,
        timestamp_us,
        keyframe,
        data,
    }
}

/// `secs` of media from `from_ms`: 30 fps video with a keyframe each
/// second, and 20 ms Opus packets, in time order.
fn media_units(from_ms: i64, secs: i64) -> Vec<CaptureUnit> {
    let mut units = Vec::new();
    let end = (from_ms + secs * 1000) * 1000;
    let mut frame = 0;
    let mut v = from_ms * 1000;
    let mut a = from_ms * 1000;
    while v < end || a < end {
        if v <= a && v < end {
            let key = frame % 30 == 0;
            units.push(unit(VIDEO, v, key, if key { 9_000 } else { 1_500 }));
            frame += 1;
            v = from_ms * 1000 + frame * 1_000_000 / 30;
        } else {
            units.push(unit(AUDIO, a, true, 160));
            a += 20_000;
        }
    }
    units
}

impl Ui {
    async fn open(&self, keys: &[Vec<u8>], title: &str) -> Result<Vec<u8>, CallError> {
        self.open_with(keys, snippet(title), BITRATE, true).await
    }

    async fn open_with(
        &self,
        keys: &[Vec<u8>],
        s: ListingSnippet,
        bitrate: u64,
        video: bool,
    ) -> Result<Vec<u8>, CallError> {
        let req = CaptureOpenRequest {
            directory_snippet: Some(s),
            network_keys: keys.to_vec(),
            bitrate,
            video,
        };
        Ok(self.capture.open(&req).await?.id)
    }

    async fn append(
        &self,
        id: &[u8],
        configs: &[CaptureConfig],
        units: &[CaptureUnit],
    ) -> Result<(), CallError> {
        let req = CaptureAppendRequest {
            id: id.to_vec(),
            configs: configs.to_vec(),
            units: units.to_vec(),
        };
        self.capture.append(&req).await.map(|_| ())
    }

    /// Appends the configs and a first keyframe of each track, so the
    /// capture writes its first records.
    async fn begin(&self, id: &[u8]) {
        let first = [unit(VIDEO, 0, true, 2_000), unit(AUDIO, 0, true, 160)];
        self.append(id, &configs(), &first).await.unwrap();
    }

    async fn update(&self, id: &[u8], title: &str) -> Result<(), CallError> {
        let req = CaptureUpdateRequest {
            id: id.to_vec(),
            directory_snippet: Some(snippet(title)),
        };
        self.capture.update(&req).await.map(|_| ())
    }

    async fn close(&self, id: &[u8]) -> Result<(), CallError> {
        let req = CaptureCloseRequest { id: id.to_vec() };
        self.capture.close(&req).await.map(|_| ())
    }

    /// Opens a unit-format stream of `uri`: its responses after `Open`, and
    /// the transfer id `Open` carried.
    async fn stream(&self, uri: &str, keys: &[Vec<u8>], mime: &str) -> (Responses, Vec<u8>) {
        let mut s = self
            .egress
            .open_stream(&EgressOpenStreamRequest {
                swarm_uri: uri.into(),
                network_keys: keys.to_vec(),
                mime_type: mime.into(),
            })
            .await
            .unwrap();
        let Body::Open(o) = next(&mut s).await else {
            panic!("not open first");
        };
        (s, o.transfer_id)
    }
}

type Responses = ClientStream<EgressOpenStreamResponse>;

async fn next(s: &mut Responses) -> Body {
    match tokio::time::timeout(Duration::from_secs(10), s.next()).await {
        Ok(Some(Ok(r))) => r.body.unwrap(),
        r => panic!("no response: {r:?}"),
    }
}

/// An access unit as a reader gave it back.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Read {
    track: u32,
    dts: i64,
    pts: i64,
    keyframe: bool,
    payload: Vec<u8>,
}

/// What a stream's records read back to through a [`UnitReader`]: its
/// configs as of each config event, and its units.
#[derive(Default)]
struct Readback {
    reader: UnitReader,
    configs: Vec<Vec<DecoderConfig>>,
    units: Vec<Read>,
}

impl Readback {
    /// Reads records from `s` until it holds `n` units.
    async fn until(&mut self, s: &mut Responses, n: usize) {
        let mut rec = Vec::new();
        while self.units.len() < n {
            let Body::Data(d) = next(s).await else {
                panic!("not data");
            };
            assert!(!d.discontinuity);
            rec.extend_from_slice(&d.data);
            if !d.segment_end {
                continue;
            }
            let record = std::mem::take(&mut rec);
            let unit = self.reader.read(&record).map(|u| Read {
                track: u.track.0,
                dts: u.dts,
                pts: u.pts,
                keyframe: u.keyframe,
                payload: u.payload.to_vec(),
            });
            while let Some(e) = self.reader.poll_event() {
                match e {
                    ReaderEvent::Configs => self.configs.push(self.reader.configs().to_vec()),
                    e => panic!("{e:?}"),
                }
            }
            self.units.extend(unit);
        }
    }
}

/// `units` as a reader gives them back: decode time is presentation time,
/// in each track's timescale.
fn expected(units: &[CaptureUnit]) -> Vec<Read> {
    units
        .iter()
        .map(|u| {
            let rate = if u.track == VIDEO { 90_000 } else { 48_000 };
            let t = (u.timestamp_us * rate + 500_000).div_euclid(1_000_000);
            Read {
                track: u.track,
                dts: t,
                pts: t,
                keyframe: u.keyframe,
                payload: u.data.clone(),
            }
        })
        .collect()
}

/// The media listings of network `network_id`'s view on `n`: id, content
/// and snippet.
fn media(n: &Node, network_id: u64) -> Vec<(u64, Media, Option<ListingSnippet>)> {
    n.directory
        .listings(network_id)
        .into_iter()
        .filter_map(|l| match l.listing.content {
            Some(Content::Media(m)) => Some((l.id, m, l.snippet)),
            _ => None,
        })
        .collect()
}

/// A capture's configs and units reach a stream on another node, by the
/// URI and MIME type its listing carries, as the unit format: the configs
/// derived from their records, and every unit with its timestamps in its
/// track's timescale. They mux into fMP4 with an Opus sample entry.
/// Closing the capture takes down its listing, transfer and swarm.
#[test]
fn a_captures_units_are_read_by_another_nodes_stream() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        let keys = [net.public.clone()];
        let va = start(&a, Config::default());
        let vb = start(&b, Config::default());
        let (ua, ub) = (connect_ui(&va), connect_ui(&vb));

        let id = ua.open(&keys, "live").await.unwrap();
        let tid = TransferId::from_bytes(&id).unwrap();
        let c = va.captures();
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].id, c[0].mime_type.as_deref()), (tid, None));
        let swarm = Uri::parse(&c[0].swarm_uri).unwrap().id;
        assert_eq!(TransferId::new(&swarm, b""), tid);
        assert!(
            c[0].swarm_uri
                .ends_with("&x.im=2&x.hf=7&x.sa=2&x.cs=4096&x.ps=8&x.sc=16")
        );
        assert!(a.transfers.is_published(tid, a.net(&net.public)));
        assert_eq!(a.host.swarms_of(&swarm), (1, 0));
        let s = &va.video_swarms()[0];
        assert_eq!((s.kind, s.transfer_id), (SwarmKind::Capture, tid));

        let units = media_units(0, 3);
        ua.append(&id, &configs(), &units[..40]).await.unwrap();
        for part in units[40..].chunks(25) {
            ua.append(&id, &[], part).await.unwrap();
        }
        let mime = format!(r#"{MIME_TYPE};codecs="avc1.64001f,Opus""#);
        assert_eq!(va.captures()[0].mime_type.as_deref(), Some(mime.as_str()));
        wait_for(10, "the listing", || !media(&b, 10).is_empty()).await;
        let (_, m, _) = media(&b, 10).remove(0);
        assert_eq!(
            (m.mime_type.as_str(), &m.swarm_uri),
            (mime.as_str(), &c[0].swarm_uri)
        );

        let (mut stream, stream_id) = ub.stream(&m.swarm_uri, &keys, &m.mime_type).await;
        assert_eq!(stream_id, id);
        let mut back = Readback::default();
        back.until(&mut stream, units.len()).await;
        assert_eq!(back.units, expected(&units));
        assert_eq!(back.configs.len(), 1);
        let read = &back.configs[0];
        assert_eq!(read.len(), 2);
        assert_eq!(
            (read[0].track, read[0].codec.as_str(), read[0].timescale),
            (TrackId(VIDEO), "avc1.64001f", 90_000)
        );
        assert_eq!(
            (read[1].track, read[1].codec.as_str(), read[1].timescale),
            (TrackId(AUDIO), "Opus", 48_000)
        );
        assert_eq!(
            read[1].params,
            CodecParams::Opus {
                head: OPUS_HEAD.to_vec(),
                channels: 2,
                pre_skip: 312
            }
        );
        let m = back.reader.metrics();
        assert_eq!((m.gaps, m.malformed), (0, 0), "{m:?}");
        let w = va.captures()[0].writer;
        assert_eq!((w.units, w.gops), (units.len() as u64, 3));
        assert_eq!(va.metrics().dropped_records, 0);

        // The units mux into fMP4: an init with an Opus sample entry, and
        // fragments of every unit.
        let mut init = Vec::new();
        write_init(read, strims_base::time::Duration::ZERO, &mut init).unwrap();
        let info = parse_init(&init).unwrap();
        assert_eq!(info.track(AUDIO).unwrap().timescale, 48_000);
        assert!(init.windows(4).any(|w| w == b"dOps"));
        let mut fragments = FragmentWriter::new(strims_base::time::Duration::from_millis(100));
        fragments
            .set_tracks(read, strims_base::time::Duration::ZERO)
            .unwrap();
        let mut out = Vec::new();
        for (r, u) in back.units.iter().zip(&units) {
            let unit = strims_video::AccessUnit {
                track: TrackId(r.track),
                dts: r.dts,
                pts: r.pts,
                keyframe: r.keyframe,
                payload: &r.payload,
                published: strims_base::time::Time::from_unix_millis(0),
            };
            fragments.push(&unit, &mut out).unwrap();
            assert_eq!(r.payload, u.data);
        }
        fragments.flush(None, &mut out);
        let payloads: usize = units.iter().map(|u| u.data.len()).sum();
        assert!(out.len() > payloads);

        ua.close(&id).await.unwrap();
        assert!(va.captures().is_empty());
        assert!(va.video_swarms().is_empty());
        assert!(!a.transfers.list().contains(&tid));
        settle().await;
        assert_eq!(a.host.swarms_of(&swarm), (0, 0));
        wait_for(10, "the listing to go", || media(&b, 10).is_empty()).await;
        assert_eq!(
            refusal(
                ua.append(&id, &[], &[unit(VIDEO, 9_000_000, true, 10)])
                    .await
            ),
            "capture not found"
        );
    });
}

/// A capture lists nothing until its first record is written, and lists
/// its stream's codecs: a config of other codecs lists the new MIME type.
#[test]
fn the_listing_follows_the_first_record_and_the_codecs() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (net, _) = owned_network(&a, &[], 10).await;
        let va = start(&a, Config::default());
        let ua = connect_ui(&va);
        let id = ua
            .open(std::slice::from_ref(&net.public), "soon")
            .await
            .unwrap();
        // Configs alone, and units before a keyframe, write no records.
        ua.append(&id, &configs(), &[unit(AUDIO, 0, true, 100)])
            .await
            .unwrap();
        ua.append(&id, &[], &[unit(VIDEO, 0, false, 100)])
            .await
            .unwrap();
        settle().await;
        assert!(media(&a, 10).is_empty());
        let c = va.captures().remove(0);
        assert_eq!(c.mime_type, None);
        assert_eq!(c.listings, [(net.public.clone(), None)]);
        assert_eq!((c.writer.records, c.writer.dropped_before_start), (0, 2));
        assert_eq!(va.metrics().dropped_records, 2);

        ua.append(&id, &[], &[unit(VIDEO, 33_333, true, 100)])
            .await
            .unwrap();
        let first = format!(r#"{MIME_TYPE};codecs="avc1.64001f,Opus""#);
        wait_for(10, "the listing", || {
            media(&a, 10)
                .first()
                .is_some_and(|(_, m, _)| m.mime_type == first)
        })
        .await;
        assert!(va.captures()[0].listings[0].1.is_some());

        let chromium = h264(hex::decode(CHROMIUM_AVCC).unwrap());
        ua.append(&id, &[chromium], &[unit(VIDEO, 66_667, true, 100)])
            .await
            .unwrap();
        let second = format!(r#"{MIME_TYPE};codecs="avc1.640c1f,Opus""#);
        wait_for(10, "the new listing", || {
            media(&a, 10)
                .first()
                .is_some_and(|(_, m, _)| m.mime_type == second)
        })
        .await;
        assert_eq!(media(&a, 10).len(), 1);
        assert_eq!(va.captures()[0].mime_type, Some(second));
    });
}

/// An append that breaks a capture's rules is refused whole with its
/// reason, and the stream goes on as before it: a reader sees every
/// accepted unit, in sequence.
#[test]
fn a_refused_append_writes_nothing_and_the_capture_goes_on() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (net, _) = owned_network(&a, &[], 10).await;
        let keys = [net.public.clone()];
        let va = start(&a, Config::default());
        let ua = connect_ui(&va);
        let id = ua.open(&keys, "picky").await.unwrap();
        ua.begin(&id).await;
        let uri = va.captures()[0].swarm_uri.clone();
        let mime = va.captures()[0].mime_type.clone().unwrap();
        let (mut stream, _) = ua.stream(&uri, &[], &mime).await;
        let records = va.captures()[0].writer.records;

        let good = unit(VIDEO, 33_333, false, 50);
        let cases: Vec<(Vec<CaptureConfig>, Vec<CaptureUnit>, &str)> = vec![
            (
                vec![],
                vec![good.clone(), unit(3, 40_000, true, 10)],
                "track 3 has no decoder config",
            ),
            (
                vec![],
                vec![good.clone(), unit(AUDIO, 0, true, 10)],
                "track 2: timestamp 0 us is not after the track's last unit",
            ),
            (
                vec![CaptureConfig { codec: 7, ..opus() }],
                vec![good.clone()],
                "track 2: codec 7 is not supported",
            ),
            (
                vec![CaptureConfig {
                    config: b"OpusHead\x10".to_vec(),
                    ..opus()
                }],
                vec![good.clone()],
                "track 2: OpusHead is truncated",
            ),
            (
                vec![h264(vec![1, 0x64, 0, 0x1f, 0xff, 0xe0, 0])],
                vec![good.clone()],
                "track 1: avcC holds no SPS",
            ),
            (
                vec![],
                vec![CaptureUnit {
                    data: vec![],
                    ..good.clone()
                }],
                "track 1: a unit has no data",
            ),
        ];
        for (configs, units, reason) in cases {
            assert_eq!(
                refusal(ua.append(&id, &configs, &units).await),
                format!("invalid capture media: {reason}")
            );
        }
        assert_eq!(va.captures()[0].writer.records, records);

        let rest = [good, unit(AUDIO, 20_000, true, 160)];
        ua.append(&id, &[], &rest).await.unwrap();
        let mut back = Readback::default();
        back.until(&mut stream, 4).await;
        let first = [unit(VIDEO, 0, true, 2_000), unit(AUDIO, 0, true, 160)];
        assert_eq!(back.units, expected(&[&first[..], &rest[..]].concat()));
        assert_eq!(back.reader.metrics().gaps, 0);
    });
}

/// A capture's appends are bounded: units per append, queued bytes and
/// rate; its network keys and snippet are checked, and the node runs at
/// most `max_captures`.
#[test]
fn captures_are_checked_and_bounded() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (net, _) = owned_network(&a, &[], 10).await;
        let config = Config {
            capture: CaptureSettings {
                max_captures: 2,
                max_network_keys: 2,
                max_units_per_append: 4,
                max_queued_bytes: 8_000,
                max_rate: 10_000,
                max_burst: 20_000,
            },
            ..Config::default()
        };
        let va = start(&a, config);
        let ua = connect_ui(&va);
        let key = net.public.clone();
        assert_eq!(
            refusal(ua.open(&[], "x").await),
            "wrong number of network keys"
        );
        let three = [key.clone(), vec![2; 32], vec![3; 32]];
        assert_eq!(
            refusal(ua.open(&three, "x").await),
            "wrong number of network keys"
        );
        assert_eq!(
            refusal(ua.open(&[vec![2; 32]], "x").await),
            "network not found"
        );
        let large = snippet(&"t".repeat(9 * 1024));
        assert_eq!(
            refusal(
                ua.open_with(std::slice::from_ref(&key), large, BITRATE, true)
                    .await
            ),
            "snippet too large"
        );
        assert!(va.captures().is_empty());

        // A key named twice counts once.
        let first = ua.open(&[key.clone(), key.clone()], "1").await.unwrap();
        assert_eq!(va.captures()[0].listings.len(), 1);
        let second = ua.open(std::slice::from_ref(&key), "2").await.unwrap();
        assert_eq!(
            refusal(ua.open(std::slice::from_ref(&key), "3").await),
            "too many captures"
        );

        let five: Vec<CaptureUnit> = (0..5).map(|i| unit(AUDIO, i * 20_000, true, 10)).collect();
        assert_eq!(
            refusal(ua.append(&second, &configs(), &five).await),
            "invalid capture media: 5 units are more than an append's 4"
        );
        // The burst takes 20000 bytes: three appends of about 6.5 KB, then
        // the rate's 10 KB a second.
        let big = |t: i64| [unit(VIDEO, t, true, 3_000), unit(AUDIO, t, true, 3_000)];
        ua.append(&second, &configs(), &big(0)).await.unwrap();
        ua.append(&second, &[], &big(33_333)).await.unwrap();
        ua.append(&second, &[], &big(66_667)).await.unwrap();
        assert_eq!(
            refusal(ua.append(&second, &[], &big(100_000)).await),
            "capture over its rate"
        );
        tokio::time::sleep(Duration::from_millis(700)).await;
        ua.append(&second, &[], &big(100_000)).await.unwrap();
        // The other capture has a bucket of its own; an append over the
        // queue's bytes is refused before it waits.
        assert_eq!(
            refusal(
                ua.append(&first, &configs(), &[unit(VIDEO, 0, true, 8_001)])
                    .await
            ),
            "capture busy"
        );
        ua.append(&first, &configs(), &big(0)).await.unwrap();

        ua.close(&first).await.unwrap();
        ua.open(std::slice::from_ref(&key), "3").await.unwrap();
        assert_eq!(va.captures().len(), 2);
        assert_eq!(
            a.host
                .swarms_of(&Uri::parse(&va.captures()[0].swarm_uri).unwrap().id),
            (1, 0)
        );
    });
}

/// A capture with video signs `UnitSigning::LARGE`: its partial integrity
/// unit of 8 chunks is signed 300 ms after its first byte, padded to the
/// unit's end, and the next unit's bytes start where the padding ends.
#[test]
fn a_partial_unit_is_signed_at_the_sign_interval() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (net, _) = owned_network(&a, &[], 10).await;
        let va = start(&a, Config::default());
        let ua = connect_ui(&va);
        let id = ua
            .open(std::slice::from_ref(&net.public), "slow")
            .await
            .unwrap();
        let swarm = Uri::parse(&va.captures()[0].swarm_uri).unwrap().id;
        assert_eq!(va.captures()[0].signing, UnitSigning::LARGE);
        let label = capture_options().label;
        let unit_bytes = 8 * 4096;
        ua.begin(&id).await;
        let written = world.written(&swarm, &label);
        assert!(written > 0 && written % unit_bytes != 0, "{written}");
        assert_eq!(va.captures()[0].early_signs, 0);

        tokio::time::sleep(Duration::from_millis(250)).await;
        assert_eq!(va.captures()[0].early_signs, 0);
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(va.captures()[0].early_signs, 1);
        assert_eq!(world.written(&swarm, &label), unit_bytes);

        // The next unit's records start at its first frame, and read back.
        ua.append(&id, &[], &[unit(VIDEO, 33_333, false, 100)])
            .await
            .unwrap();
        assert!(world.written(&swarm, &label) > unit_bytes);
        let mime = va.captures()[0].mime_type.clone().unwrap();
        let uri = va.captures()[0].swarm_uri.clone();
        let (mut stream, _) = ua.stream(&uri, &[], &mime).await;
        let mut back = Readback::default();
        back.until(&mut stream, 3).await;
        assert_eq!(
            back.units[2],
            expected(&[unit(VIDEO, 33_333, false, 100)])[0]
        );
    });
}

/// A capture of audio alone, below 750 kbit/s, signs `UnitSigning::SMALL`:
/// 1-chunk integrity units, a partial one signed 200 ms after its first
/// byte. Without a nominal bitrate, the tracks pick.
#[test]
fn a_low_rate_capture_takes_one_chunk_units() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (net, _) = owned_network(&a, &[], 10).await;
        let va = start(&a, Config::default());
        let ua = connect_ui(&va);
        let keys = std::slice::from_ref(&net.public);
        let id = ua
            .open_with(keys, snippet("radio"), 128_000, false)
            .await
            .unwrap();
        let uri = va.captures()[0].swarm_uri.clone();
        assert!(uri.ends_with("&x.cs=4096&x.ps=1&x.sc=16"), "{uri}");
        assert_eq!(va.captures()[0].signing, UnitSigning::SMALL);
        for (bitrate, video, signing) in [
            (0, false, UnitSigning::SMALL),
            (0, true, UnitSigning::LARGE),
            (128_000, true, UnitSigning::SMALL),
        ] {
            let other = ua
                .open_with(keys, snippet("other"), bitrate, video)
                .await
                .unwrap();
            let tid = TransferId::from_bytes(&other).unwrap();
            let c = va.captures().into_iter().find(|c| c.id == tid).unwrap();
            assert_eq!(c.signing, signing, "{bitrate} {video}");
            ua.close(&other).await.unwrap();
        }
        let swarm = Uri::parse(&uri).unwrap().id;
        let label = capture_options().label;
        ua.append(&id, &[opus()], &[unit(AUDIO, 0, true, 160)])
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(va.captures()[0].early_signs, 0);
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(va.captures()[0].early_signs, 1);
        assert_eq!(world.written(&swarm, &label), 4096);
        assert_eq!(
            va.captures()[0].mime_type.as_deref(),
            Some(r#"video/x.strims-units;codecs="Opus""#)
        );
    });
}

/// Closing a capture has the swarm sign its last partial unit before the
/// swarm goes.
#[test]
fn closing_signs_the_last_partial_unit() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (net, _) = owned_network(&a, &[], 10).await;
        let va = start(&a, Config::default());
        let ua = connect_ui(&va);
        let id = ua
            .open(std::slice::from_ref(&net.public), "last")
            .await
            .unwrap();
        let swarm = Uri::parse(&va.captures()[0].swarm_uri).unwrap().id;
        let label = capture_options().label;
        // Closed before the partial unit's 300 ms are up.
        ua.begin(&id).await;
        let written = world.written(&swarm, &label);
        assert!(written % (8 * 4096) != 0, "{written}");
        assert_eq!(va.captures()[0].early_signs, 0);
        ua.close(&id).await.unwrap();
        assert_eq!(world.written(&swarm, &label), 8 * 4096);
    });
}

/// A capture on two networks keeps a listing on each one's directory.
#[test]
fn a_capture_on_two_networks_has_a_listing_on_each() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (n10, _) = owned_network(&a, &[], 10).await;
        let (n11, _) = owned_network(&a, &[], 11).await;
        let va = start(&a, Config::default());
        let ua = connect_ui(&va);
        let keys = [n10.public.clone(), n11.public.clone()];
        let id = ua.open(&keys, "twice").await.unwrap();
        let tid = TransferId::from_bytes(&id).unwrap();
        for k in &keys {
            assert!(a.transfers.is_published(tid, a.net(k)));
        }
        ua.begin(&id).await;

        wait_for(10, "both listings", || {
            va.captures()[0].listings.iter().all(|(_, l)| l.is_some())
                && !media(&a, 10).is_empty()
                && !media(&a, 11).is_empty()
        })
        .await;
        let c = va.captures().remove(0);
        for (network_id, key) in [(10, &n10.public), (11, &n11.public)] {
            let (_, listing) = c.listings.iter().find(|(k, _)| k == key).unwrap();
            let listed = media(&a, network_id);
            assert_eq!(listed.len(), 1, "network {network_id}");
            assert_eq!(Some(listed[0].0), *listing);
            assert_eq!(listed[0].1.swarm_uri, c.swarm_uri);
        }
    });
}

/// A capture's transfer goes on its network again as the node joins it
/// again.
#[test]
fn a_capture_goes_on_its_network_as_the_node_joins_it() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let key = key.public;
        let va = start(&alice, Config::default());
        let ua = connect_ui(&va);
        let id = ua.open(std::slice::from_ref(&key), "again").await.unwrap();
        let tid = TransferId::from_bytes(&id).unwrap();
        let first = alice.net(&key);
        assert!(alice.transfers.is_published(tid, first));

        let record = alice.read_network(10).await;
        let mut tx = alice.store.write(alice.timer.now()).await.unwrap();
        Networks::delete(&mut tx, 10).await.unwrap();
        tx.commit().await.unwrap();
        wait_for(10, "the network to stop", || alice.host.net(&key).is_none()).await;
        alice.write_network(Network { id: 110, ..record }).await;
        wait_for(120, "the transfer published again", || {
            alice
                .host
                .net(&key)
                .is_some_and(|net| net != first && alice.transfers.is_published(tid, net))
        })
        .await;
        assert_eq!(va.captures().len(), 1);
    });
}

/// The snippet is the capture's, signed with its key, and its views show
/// it and its updates.
#[test]
fn the_snippet_is_signed_with_the_capture_key() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        let va = start(&a, Config::default());
        let ua = connect_ui(&va);
        let id = ua
            .open(std::slice::from_ref(&net.public), "first")
            .await
            .unwrap();
        ua.begin(&id).await;
        let swarm = Uri::parse(&va.captures()[0].swarm_uri).unwrap().id;

        let shown = |title: &'static str| {
            let b = &b;
            move || {
                media(b, 10)
                    .first()
                    .and_then(|(_, _, s)| s.as_ref())
                    .is_some_and(|s| s.title == title)
            }
        };
        wait_for(10, "the snippet", shown("first")).await;
        let (_, m, s) = media(&b, 10).remove(0);
        assert_eq!(Uri::parse(&m.swarm_uri).unwrap().id, swarm);
        let s = s.unwrap();
        assert_eq!((s.description.as_str(), s.live), ("desc", true));
        // the server shows only snippets the swarm's key signed
        let server = || a.directory.metrics(&net.public).unwrap().server.unwrap();
        assert_eq!(server().snippets_unverified, 0, "{:?}", server());

        ua.update(&id, "second").await.unwrap();
        wait_for(10, "the update", shown("second")).await;
        assert_eq!(
            (server().snippets_shown, server().snippets_unverified),
            (2, 0),
            "{:?}",
            server()
        );

        let long = "x".repeat(9 * 1024);
        assert_eq!(refusal(ua.update(&id, &long).await), "snippet too large");
    });
}

/// A frontend connection's captures close with it.
#[test]
fn closing_the_connection_closes_its_captures() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, _pipes) = owned_network(&a, &[&b], 10).await;
        let va = start(&a, Config::default());
        let ua = connect_ui(&va);
        let kept = connect_ui(&va);
        let keys = [net.public.clone()];
        let id = ua.open(&keys, "going").await.unwrap();
        let other = kept.open(&keys, "staying").await.unwrap();
        ua.begin(&id).await;
        kept.begin(&other).await;
        let tid = TransferId::from_bytes(&id).unwrap();
        let going = va.captures().into_iter().find(|c| c.id == tid).unwrap();
        let swarm = Uri::parse(&going.swarm_uri).unwrap().id;
        wait_for(10, "the listings", || media(&b, 10).len() == 2).await;

        let server = ua.server;
        server.abort();
        let _ = server.await;
        wait_for(10, "the capture to close", || va.captures().len() == 1).await;
        assert_eq!(va.captures()[0].id.as_bytes().to_vec(), other);
        settle().await;
        assert!(!a.transfers.list().contains(&tid));
        assert_eq!(a.host.swarms_of(&swarm), (0, 0));
        let listed = media(&b, 10);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].1.swarm_uri, va.captures()[0].swarm_uri);
        kept.append(&other, &[], &[unit(VIDEO, 33_333, false, 100)])
            .await
            .unwrap();
    });
}

/// Another connection can't append to, update or close a capture it
/// didn't open.
#[test]
fn a_capture_answers_only_its_own_connection() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (net, _) = owned_network(&a, &[], 10).await;
        let va = start(&a, Config::default());
        let (owner, other) = (connect_ui(&va), connect_ui(&va));
        let id = owner
            .open(std::slice::from_ref(&net.public), "mine")
            .await
            .unwrap();

        let not_found = "capture not found";
        let u = [unit(VIDEO, 0, true, 10)];
        assert_eq!(refusal(other.append(&id, &configs(), &u).await), not_found);
        assert_eq!(refusal(other.update(&id, "theirs").await), not_found);
        assert_eq!(refusal(other.close(&id).await), not_found);
        assert_eq!(
            refusal(owner.append(&[1; 5], &configs(), &u).await),
            not_found
        );
        assert_eq!(
            refusal(owner.append(&[1; 32], &configs(), &u).await),
            not_found
        );
        assert_eq!(va.captures().len(), 1);

        owner.append(&id, &configs(), &u).await.unwrap();
        owner.update(&id, "still mine").await.unwrap();
        owner.close(&id).await.unwrap();
        assert_eq!(refusal(owner.close(&id).await), not_found);
        assert!(va.captures().is_empty());
    });
}
