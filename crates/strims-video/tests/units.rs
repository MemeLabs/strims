// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A unit-format ingest on fake nodes (see `strims_testworld`), read back
//! through another node's egress stream.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::time::Duration;

use strims_base::time::Time;
use strims_kv::MemoryKv;
use strims_ppspp::SwarmId;
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::types::Key;
use strims_proto::video::v1::{VideoChannel, VideoIngressConfig};
use strims_testworld::*;
use strims_video::hls::HlsRoutes;
use strims_video::synth::script::{Script, ScriptConfig, TagKind};
use strims_video::units::{ReaderEvent, UnitReader};
use strims_video::{
    AccessUnit, Config, DecoderConfig, EgressPiece, EgressStream, FlvSource, IngestFormat,
    IngressSettings, NoVideoHost, SourceEvent, StreamFormat, TrackId, VideoControl, VideoError,
    VideoHost, units_mime,
};
use support::{FakeVideo, TestPublisher};

type Control<V> = VideoControl<MemoryKv, FakeHost, V, TokioTimer>;
type Stream<V> = EgressStream<MemoryKv, FakeHost, V, TokioTimer>;

const ADDR: &str = "127.0.0.1:1935";
/// The ingest swarm's integrity unit.
const UNIT: u64 = 8 * 4096;

/// A unit as the source made it, without its publish time: track, DTS,
/// PTS, keyframe and payload.
type Unit = (TrackId, i64, i64, bool, Vec<u8>);

fn owned(u: &AccessUnit<'_>) -> Unit {
    (u.track, u.dts, u.pts, u.keyframe, u.payload.to_vec())
}

fn start<V: VideoHost>(n: &Node, video: V, config: Config) -> Control<V> {
    let control = VideoControl::new(
        n.host.clone(),
        video,
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
    let c = control.clone();
    n.spawn(async move { c.run().await });
    control
}

fn units_config() -> Config {
    Config {
        ingress: IngressSettings {
            format: IngestFormat::Units,
            ..IngressSettings::default()
        },
        ..Config::default()
    }
}

/// The decoder configs and units the source makes of `script`, in order.
fn script_units(script: &Script) -> (Vec<DecoderConfig>, Vec<Unit>) {
    let mut source = FlvSource::new();
    let mut configs = Vec::new();
    let mut units = Vec::new();
    let at = Time::from_unix_millis(0);
    for tag in &script.tags {
        let e = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, at),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, at),
            TagKind::Metadata => continue,
        };
        match e.unwrap() {
            Some(SourceEvent::Config(c)) => configs.push(c),
            Some(SourceEvent::Unit(u)) => units.push(owned(&u)),
            None => {}
        }
    }
    (configs, units)
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

/// What a unit reader made of a stream: units, and the gaps it reported
/// by track.
#[derive(Default)]
struct Read {
    units: Vec<Unit>,
    gaps: Vec<(TrackId, u64)>,
    configs: Vec<DecoderConfig>,
}

impl Read {
    fn record(&mut self, r: &mut UnitReader, record: &[u8]) {
        let unit = r.read(record).map(|u| owned(&u));
        while let Some(e) = r.poll_event() {
            match e {
                ReaderEvent::Configs => self.configs = r.configs().to_vec(),
                ReaderEvent::Gap { track, missing } => self.gaps.push((track, missing)),
            }
        }
        self.units.extend(unit);
    }
}

/// Reads a stream's pieces through a unit reader until it holds a unit
/// equal to `last`, or the stream ends.
async fn read_stream<V: VideoHost>(s: &mut Stream<V>, last: &Unit) -> Read {
    let mut r = UnitReader::new();
    let mut read = Read::default();
    let mut rec = Vec::new();
    while read.units.last() != Some(last) {
        let piece = tokio::time::timeout(Duration::from_secs(10), s.next(usize::MAX))
            .await
            .expect("a piece in time");
        match piece {
            Ok(EgressPiece::Data(d)) => rec.extend_from_slice(d),
            Ok(EgressPiece::RecordEnd) => read.record(&mut r, &std::mem::take(&mut rec)),
            Ok(EgressPiece::Discontinuity) => {
                rec.clear();
                r.lost();
            }
            Err(VideoError::StreamClosed) => break,
            Err(e) => panic!("{e}"),
        }
    }
    read
}

/// A node with network 10 and a channel on it, ingesting in the unit
/// format, and a second node to view it.
struct Ingest {
    world: World,
    a: Node,
    b: Node,
    net: Key,
    video: FakeVideo,
    va: Control<FakeVideo>,
    channel: VideoChannel,
    key: String,
    _pipes: Vec<Pipe>,
}

impl Ingest {
    async fn new() -> Ingest {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = node_in(&world, 2).await;
        let (net, pipes) = owned_network(&a, &[&b], 10).await;
        let video = FakeVideo::default();
        let va = start(&a, video.clone(), units_config());
        let snippet = ListingSnippet {
            title: "units".into(),
            ..Default::default()
        };
        let channel = va.create_channel(&net.public, snippet).await.unwrap();
        va.set_ingress_config(VideoIngressConfig {
            enabled: true,
            server_addr: ADDR.into(),
            ..Default::default()
        })
        .await
        .unwrap();
        wait_for(10, "the listener", || {
            video.listening().as_deref() == Some(ADDR)
        })
        .await;
        let key = va.channel_url(channel.id).await.unwrap().stream_key;
        Ingest {
            world,
            a,
            b,
            net,
            video,
            va,
            channel,
            key,
            _pipes: pipes,
        }
    }

    async fn publish(&self) -> TestPublisher {
        let mut p = TestPublisher::connect(
            &self.video,
            "10.0.0.1:1",
            "live",
            &self.key,
            &mut self.a.rng(),
        );
        p.answered().await;
        assert!(p.publisher.publish_accepted());
        p
    }

    fn swarm_id(&self) -> SwarmId {
        SwarmId::new(&self.channel.key.as_ref().unwrap().public)
    }

    fn written(&self) -> u64 {
        self.world.written(&self.swarm_id(), "")
    }
}

/// A unit-format ingest is listed with its MIME type, signs a partial
/// integrity unit once the stream goes quiet, and reaches another node's
/// stream whole, up to the last unit, which the session's end signs.
#[test]
fn a_unit_format_ingest_reads_back_through_egress() {
    run(async {
        let s = Ingest::new().await;
        let vb = start(&s.b, NoVideoHost, Config::default());
        let mut p = s.publish().await;
        let script = Script::new(&ScriptConfig::default());
        let (configs, units) = script_units(&script);
        let third = script.tags.len() / 3;
        p.send_tags(&script.tags[..third], &script).await;

        // Quiet past the sign interval: the partial unit is padded out.
        let before = s.va.metrics().ingress.early_signs;
        assert_ne!(s.written() % UNIT, 0);
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(s.va.metrics().ingress.early_signs, before + 1);
        assert_eq!(s.written() % UNIT, 0);

        wait_for(10, "the listing", || !media(&s.b).is_empty()).await;
        let m = media(&s.b).remove(0);
        assert_eq!(m.mime_type, units_mime(&configs));
        assert_eq!(
            m.mime_type,
            r#"video/x.strims-units;codecs="avc1.64001f,mp4a.40.2""#
        );
        assert_eq!(StreamFormat::from_mime(&m.mime_type), StreamFormat::Units);
        let mut stream = vb
            .open_stream(
                &m.swarm_uri,
                std::slice::from_ref(&s.net.public),
                StreamFormat::from_mime(&m.mime_type),
            )
            .await
            .unwrap();

        p.send_tags(&script.tags[third..], &script).await;
        p.unpublish().await;
        let read = read_stream(&mut stream, units.last().unwrap()).await;
        assert_eq!(read.units, units);
        assert_eq!(read.configs, configs);
        assert!(read.gaps.is_empty());
        let r = stream.metrics();
        assert_eq!((r.discontinuities, r.rewinds, r.refused_rewinds), (0, 0, 0));

        wait_for(10, "the session's end", || {
            s.va.ingress_streams().is_empty()
        })
        .await;
        // The end signed the last partial unit, and the stream's bytes
        // are all counted.
        assert_eq!(s.written() % UNIT, 0);
        let m = s.va.metrics().ingress;
        assert_eq!(m.bytes, s.written());
        assert!(m.early_signs > before + 1);
        assert_eq!((m.dropped_units, m.media_errors, m.write_errors), (0, 0, 0));
    });
}
