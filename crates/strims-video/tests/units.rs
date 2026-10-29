// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A unit-format ingest on fake nodes (see `strims_testworld`), read back
//! through egress: by another node's stream from the start, and by
//! egress's reader over the ingest swarm's stream, joining mid-stream and
//! losing a chunk.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use strims_base::time::Time;
use strims_chunkstream::chunked::{ChunkedRecordDecoder, Item};
use strims_kv::MemoryKv;
use strims_ppspp::SwarmId;
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::types::Key;
use strims_proto::video::v1::{VideoChannel, VideoIngressConfig};
use strims_testworld::*;
use strims_transfer::{ReadFailure, SwarmRead};
use strims_video::hls::HlsRoutes;
use strims_video::synth::script::{Script, ScriptConfig, TagKind};
use strims_video::units::record::{Record, parse_record};
use strims_video::units::{MAX_RECORD_LEN, ReaderEvent, UnitReader};
use strims_video::{
    AccessUnit, ChunkedReader, Config, DecoderConfig, EgressPiece, EgressStream, FlvSource,
    IngestFormat, IngressSettings, NoVideoHost, Rewind, SourceEvent, StreamFormat, TrackId,
    VideoControl, VideoError, VideoHost, units_mime,
};
use support::{FakeVideo, TestPublisher};

type Control<V> = VideoControl<MemoryKv, FakeHost, V, TokioTimer>;
type Stream<V> = EgressStream<MemoryKv, FakeHost, V, TokioTimer>;

const ADDR: &str = "127.0.0.1:1935";
/// The ingest swarm's chunk, and its integrity unit.
const CHUNK: usize = 4096;
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

    /// The ingest swarm's stream bytes so far.
    fn stream(&self) -> Vec<u8> {
        let w = self.world.0.borrow();
        let log = w
            .logs
            .get(&(self.swarm_id(), String::new()))
            .unwrap()
            .borrow();
        assert_eq!(log.tail, 0, "the stream fits its window");
        log.data.iter().copied().collect()
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

/// A swarm reader over a stream's bytes, reading a chunk at a time from
/// `pos`, which leaves out the chunk at `lost` (a gap between offsets) and
/// rewinds when `rewinds`.
struct Snapshot {
    data: Rc<Vec<u8>>,
    pos: usize,
    lost: Option<usize>,
    rewinds: bool,
    /// The offsets rewinds were asked for.
    asked: Rc<RefCell<Vec<u64>>>,
}

impl Snapshot {
    fn new(data: &Rc<Vec<u8>>, pos: usize) -> Snapshot {
        Snapshot {
            data: data.clone(),
            pos,
            lost: None,
            rewinds: false,
            asked: Rc::default(),
        }
    }
}

impl SwarmRead for Snapshot {
    type Error = String;

    async fn read_at(&mut self, out: &mut [u8]) -> Result<(u64, usize), ReadFailure<String>> {
        if self.lost == Some(self.pos) {
            self.pos += CHUNK;
        }
        if self.pos >= self.data.len() {
            return Err(ReadFailure::Closed);
        }
        let n = out
            .len()
            .min(CHUNK - self.pos % CHUNK)
            .min(self.data.len() - self.pos);
        out[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        let at = self.pos;
        self.pos += n;
        Ok((at as u64, n))
    }
}

impl Rewind for Snapshot {
    async fn rewind(&mut self, offset: u64) -> Option<u64> {
        self.asked.borrow_mut().push(offset);
        let at = offset as usize / CHUNK * CHUNK;
        if !self.rewinds || at >= self.pos {
            return None;
        }
        self.pos = at;
        Some(at as u64)
    }
}

/// Reads `snapshot` through egress's chunked reader and a unit reader.
fn read_snapshot(snapshot: Snapshot) -> (Read, strims_video::ReaderMetrics) {
    let mut c = ChunkedReader::new(snapshot, CHUNK, 64 * 1024).unwrap();
    let mut r = UnitReader::new();
    let mut read = Read::default();
    let mut rec = Vec::new();
    loop {
        let piece = futures_util::FutureExt::now_or_never(c.next(32 * 1024))
            .expect("a snapshot never waits");
        match piece {
            Ok(EgressPiece::Data(d)) => rec.extend_from_slice(d),
            Ok(EgressPiece::RecordEnd) => read.record(&mut r, &std::mem::take(&mut rec)),
            Ok(EgressPiece::Discontinuity) => {
                rec.clear();
                r.lost();
            }
            Err(VideoError::StreamClosed) => return (read, c.metrics()),
            Err(e) => panic!("{e}"),
        }
    }
}

/// The stream offset of each UNIT record of `stream`, by track and DTS,
/// and the offsets of every record, in order.
fn unit_offsets(stream: &[u8]) -> (BTreeMap<(TrackId, i64), u64>, Vec<u64>) {
    let mut d = ChunkedRecordDecoder::new(CHUNK, MAX_RECORD_LEN).unwrap();
    let mut units = BTreeMap::new();
    let mut all = Vec::new();
    let mut at = 0;
    loop {
        let (n, item) = d.decode(&stream[at..]);
        at += n;
        match item {
            Some(Item::Record { offset, payload }) => {
                all.push(offset);
                if let Record::Unit { header, .. } = parse_record(payload).unwrap() {
                    units.insert((header.track, header.dts), offset);
                }
            }
            Some(Item::Lost) => panic!("lost records"),
            None => return (units, all),
        }
    }
}

/// A finished unit-format ingest of the default script, sent in parts:
/// its stream, and the decoder configs and units the source made.
fn ingested() -> (Vec<u8>, Vec<DecoderConfig>, Vec<Unit>) {
    let out = Rc::new(RefCell::new(None));
    let o = out.clone();
    run(async move {
        let s = Ingest::new().await;
        let mut p = s.publish().await;
        let script = Script::new(&ScriptConfig::default());
        // In parts, each signed early in the pause after it.
        for part in script.tags.chunks(script.tags.len() / 7) {
            p.send_tags(part, &script).await;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        p.unpublish().await;
        wait_for(10, "the session's end", || {
            s.va.ingress_streams().is_empty()
        })
        .await;
        assert!(s.va.metrics().ingress.early_signs >= 7);
        let (configs, units) = script_units(&script);
        *o.borrow_mut() = Some((s.stream(), configs, units));
    });
    out.take().unwrap()
}

const VIDEO: TrackId = TrackId(1);

/// The index in `units` of the `n`-th video keyframe.
fn keyframe(units: &[Unit], n: usize) -> usize {
    units
        .iter()
        .enumerate()
        .filter(|(_, u)| u.0 == VIDEO && u.3)
        .nth(n)
        .unwrap()
        .0
}

/// A reader joining in the middle of a GOP rewinds to the GOP's sync
/// record and starts at its keyframe; when it can't rewind, it starts at
/// the next GOP.
#[test]
fn a_reader_joining_mid_stream_starts_at_its_gops_keyframe() {
    let (stream, configs, units) = ingested();
    let (offsets, _) = unit_offsets(&stream);
    let data = Rc::new(stream);
    // Half way through the third GOP (2 s GOPs at 30 fps).
    let (second, third) = (keyframe(&units, 2), keyframe(&units, 3));
    let middle = units[second..third]
        .iter()
        .filter(|u| u.0 == VIDEO)
        .nth(30)
        .unwrap();
    let join = offsets[&(middle.0, middle.1)] as usize / CHUNK * CHUNK;

    let mut s = Snapshot::new(&data, join);
    s.rewinds = true;
    let asked = s.asked.clone();
    let (read, m) = read_snapshot(s);
    assert_eq!(read.units, units[second..]);
    assert_eq!(read.configs, configs);
    assert!(read.gaps.is_empty());
    assert_eq!((m.rewinds, m.refused_rewinds), (1, 0));
    // It asked for the sync record right before the GOP's keyframe.
    let key = offsets[&(units[second].0, units[second].1)];
    let k = asked.borrow()[0];
    assert!(
        k < key && key - k < 200,
        "sync record at {k}, keyframe at {key}"
    );

    let (read, m) = read_snapshot(Snapshot::new(&data, join));
    assert_eq!(read.units, units[third..]);
    assert_eq!((m.rewinds, m.refused_rewinds), (0, 1));
}

/// A lost chunk costs the records that cross it, and the video up to the
/// next keyframe: every other unit is read, and the gaps count the units
/// lost.
#[test]
fn a_lost_chunk_costs_its_units_and_the_video_to_the_next_keyframe() {
    let (stream, _, units) = ingested();
    let (offsets, records) = unit_offsets(&stream);
    let data = Rc::new(stream);
    let (second, third) = (keyframe(&units, 2), keyframe(&units, 3));
    let middle = units[second..third]
        .iter()
        .filter(|u| u.0 == VIDEO)
        .nth(30)
        .unwrap();
    let lost = offsets[&(middle.0, middle.1)] as usize / CHUNK * CHUNK + CHUNK;
    let mut s = Snapshot::new(&data, 0);
    s.lost = Some(lost);
    let (read, m) = read_snapshot(s);

    // A record crosses the lost chunk when it starts before the chunk's end
    // and the next record starts after the chunk's start.
    let crosses = |u: &Unit| {
        let o = offsets[&(u.0, u.1)];
        let next = records
            .iter()
            .find(|&&r| r > o)
            .copied()
            .unwrap_or(u64::MAX);
        o < (lost + CHUNK) as u64 && next > lost as u64
    };
    let first_lost_video = units
        .iter()
        .position(|u| u.0 == VIDEO && crosses(u))
        .unwrap();
    let expected: Vec<Unit> = units
        .iter()
        .enumerate()
        .filter(|(i, u)| !crosses(u) && !(u.0 == VIDEO && *i > first_lost_video && *i < third))
        .map(|(_, u)| u.clone())
        .collect();
    assert_eq!(read.units, expected);
    let mut missing: BTreeMap<TrackId, u64> = BTreeMap::new();
    for (track, n) in &read.gaps {
        *missing.entry(*track).or_default() += n;
    }
    let mut crossed: BTreeMap<TrackId, u64> = BTreeMap::new();
    for u in units.iter().filter(|u| crosses(u)) {
        *crossed.entry(u.0).or_default() += 1;
    }
    assert_eq!(missing, crossed);
    assert!(crossed[&VIDEO] >= 1);
    assert_eq!(m.discontinuities, 1);
}
