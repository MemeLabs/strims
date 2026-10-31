// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A unit-format ingest on fake nodes (see `strims_testworld`), read back
//! through egress: by another node's stream from the start; by streams
//! that join it live, at their swarm reader's newest chunk, and rewind to
//! their GOP's sync record, or start at the next keyframe when the rewind
//! is refused or late; and by egress's reader over the ingest swarm's
//! stream, joining mid-stream and losing a chunk.

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

use futures_util::FutureExt;
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
use strims_video::units::{END_LINGER, MAX_RECORD_LEN, ReaderEvent, UnitReader};
use strims_video::{
    AccessUnit, ChunkedReader, Config, DecoderConfig, EgressPiece, EgressSettings, EgressStream,
    FlvSource, IngressSettings, NoVideoHost, Rewind, SourceEvent, TrackId, VideoControl,
    VideoError, VideoHost, check_format, units, units_mime,
};
use support::{FakeVideo, TestPublisher};
use tokio::task::JoinHandle;

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

/// What a unit reader made of a stream: units, the gaps it reported by
/// track, and the ends it read.
#[derive(Default)]
struct Read {
    units: Vec<Unit>,
    gaps: Vec<(TrackId, u64)>,
    configs: Vec<DecoderConfig>,
    ends: u64,
}

impl Read {
    fn record(&mut self, r: &mut UnitReader, record: &[u8]) {
        let unit = r.read(record).map(|u| owned(&u));
        while let Some(e) = r.poll_event() {
            match e {
                ReaderEvent::Configs => self.configs = r.configs().to_vec(),
                ReaderEvent::Gap { track, missing } => self.gaps.push((track, missing)),
                ReaderEvent::End => self.ends += 1,
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

/// The record a stream's pieces give next, whole, and whether a
/// discontinuity came before it.
async fn next_record<V: VideoHost>(s: &mut Stream<V>) -> (Vec<u8>, bool) {
    let mut rec = Vec::new();
    let mut skipped = false;
    loop {
        let piece = tokio::time::timeout(Duration::from_secs(60), s.next(usize::MAX))
            .await
            .expect("a piece in time");
        match piece.unwrap() {
            EgressPiece::Data(d) => rec.extend_from_slice(d),
            EgressPiece::RecordEnd => return (rec, skipped),
            EgressPiece::Discontinuity => {
                rec.clear();
                skipped = true;
            }
        }
    }
}

/// Reads a stream's records until none comes for a second.
async fn drain_records<V: VideoHost>(s: &mut Stream<V>) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while let Ok((r, _)) = tokio::time::timeout(Duration::from_secs(1), next_record(s)).await {
        out.push(r);
    }
    out
}

fn is_end(record: &[u8]) -> bool {
    matches!(parse_record(record), Ok(Record::End { .. }))
}

/// Reads a stream's records up to its END: how many came before it, and
/// how many of those after a discontinuity.
async fn records_to_end<V: VideoHost>(s: &mut Stream<V>) -> (usize, usize) {
    let (mut before, mut after) = (0, 0);
    loop {
        let (r, skipped) = next_record(s).await;
        if skipped || after > 0 {
            if is_end(&r) {
                return (before, after);
            }
            after += 1;
        } else if is_end(&r) {
            return (before, after);
        } else {
            before += 1;
        }
    }
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
        let va = start(&a, video.clone(), Config::default());
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
/// stream whole, up to the last unit and the END after it, which the
/// session's end signs.
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
        assert_eq!(check_format(&m.mime_type), Ok(()));
        let mut stream = vb
            .open_stream(
                &m.swarm_uri,
                std::slice::from_ref(&s.net.public),
                &m.mime_type,
            )
            .await
            .unwrap();

        p.send_tags(&script.tags[third..], &script).await;
        p.unpublish().await;
        let read = read_stream(&mut stream, units.last().unwrap()).await;
        assert_eq!(read.units, units);
        assert_eq!(read.configs, configs);
        assert!(read.gaps.is_empty());
        let (end, _) = next_record(&mut stream).await;
        assert_eq!(parse_record(&end).unwrap(), Record::End { gop_back: 0 });
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

/// A session whose connection drops, with no unpublish, keeps its listing
/// and swarm through the resume grace with no END, and ends with its END
/// after it.
#[test]
fn a_dropped_session_ends_with_its_end_after_the_resume_grace() {
    run(async {
        let s = Ingest::new().await;
        let vb = start(&s.b, NoVideoHost, Config::default());
        let mut p = s.publish().await;
        let script = Script::new(&ScriptConfig::default());
        p.send_tags(&script.tags[..script.tags.len() / 2], &script)
            .await;
        wait_for(10, "the listing", || !media(&s.b).is_empty()).await;
        let m = media(&s.b).remove(0);
        let mut stream = vb
            .open_stream(
                &m.swarm_uri,
                std::slice::from_ref(&s.net.public),
                units::MIME_TYPE,
            )
            .await
            .unwrap();
        drop(p);
        let dropped = tokio::time::Instant::now();
        let early =
            tokio::time::timeout(Duration::from_secs(20), records_to_end(&mut stream)).await;
        assert!(early.is_err(), "an END within the grace");
        assert_eq!(s.va.ingress_streams().len(), 1);
        assert!(!media(&s.b).is_empty());

        records_to_end(&mut stream).await;
        let grace = IngressSettings::default().resume_grace;
        assert!(dropped.elapsed() >= grace.to_std().unwrap());
        wait_for(10, "the session's end", || {
            s.va.ingress_streams().is_empty()
        })
        .await;
        wait_for(10, "the listing to go", || media(&s.b).is_empty()).await;
        assert_eq!(s.va.metrics().ingress.replaced, 0);
    });
}

/// A publish to the channel within a dropped session's resume grace takes
/// the session over at once: the stream goes on in the new session's
/// swarm with no END of the old, and its listing with it.
#[test]
fn a_publish_within_the_resume_grace_takes_the_session_over_without_an_end() {
    run(async {
        let s = Ingest::new().await;
        let vb = start(&s.b, NoVideoHost, Config::default());
        let mut p = s.publish().await;
        let script = Script::new(&ScriptConfig::default());
        p.send_tags(&script.tags[..script.tags.len() / 2], &script)
            .await;
        wait_for(10, "the listing", || !media(&s.b).is_empty()).await;
        let m = media(&s.b).remove(0);
        let mut stream = vb
            .open_stream(
                &m.swarm_uri,
                std::slice::from_ref(&s.net.public),
                units::MIME_TYPE,
            )
            .await
            .unwrap();
        drop(p);
        let old = drain_records(&mut stream).await;
        assert!(!old.is_empty() && !old.iter().any(|r| is_end(r)));
        tokio::time::sleep(Duration::from_secs(5)).await;
        let resumed = tokio::time::Instant::now();
        let mut again = s.publish().await;
        assert!(resumed.elapsed() < Duration::from_secs(1));
        assert!(!media(&s.b).is_empty(), "the listing stays");
        let m = s.va.metrics().ingress;
        assert_eq!((m.sessions, m.replaced), (2, 1));
        again.send_tags(&script.tags, &script).await;
        again.unpublish().await;
        let (before, after) = records_to_end(&mut stream).await;
        assert_eq!(before, 0);
        assert!(after > 0);
        wait_for(10, "the session's end", || {
            s.va.ingress_streams().is_empty()
        })
        .await;
    });
}

/// An unpublished session's swarm stays for the END's linger, and a
/// publish to the channel meanwhile starts at once and keeps its own
/// swarm past it.
#[test]
fn a_publish_during_the_end_linger_starts_at_once() {
    run(async {
        let s = Ingest::new().await;
        let mut p = s.publish().await;
        let script = Script::new(&ScriptConfig::default());
        p.send_tags(&script.tags[..script.tags.len() / 3], &script)
            .await;
        p.unpublish().await;
        wait_for(10, "the END", || {
            let stream = s.stream();
            let recs = support_records(&stream);
            recs.last().is_some_and(|r| is_end(r))
        })
        .await;
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(s.va.video_swarms().len(), 1, "the swarm lingers");

        let resumed = tokio::time::Instant::now();
        let mut again = s.publish().await;
        assert!(resumed.elapsed() < Duration::from_secs(1));
        again
            .send_tags(&script.tags[..script.tags.len() / 3], &script)
            .await;
        tokio::time::sleep(END_LINGER.to_std().unwrap()).await;
        assert_eq!(s.va.ingress_streams().len(), 1);
        assert_eq!(s.va.video_swarms().len(), 1);
        assert!(!again.closed);
        let recs = support_records(&s.stream());
        assert!(!recs.is_empty() && !recs.iter().any(|r| is_end(r)));
    });
}

/// The records of a stream from offset 0.
fn support_records(stream: &[u8]) -> Vec<Vec<u8>> {
    let mut d = ChunkedRecordDecoder::new(CHUNK, MAX_RECORD_LEN).unwrap();
    let mut out = Vec::new();
    let mut at = 0;
    loop {
        let (n, item) = d.decode(&stream[at..]);
        at += n;
        match item {
            Some(Item::Record { payload, .. }) => out.push(payload.to_vec()),
            Some(Item::Lost) => panic!("lost records"),
            None => return out,
        }
    }
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
        let piece = c
            .next(32 * 1024)
            .now_or_never()
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

/// The records of a unit-format stream with their offsets, in order.
fn records_of(stream: &[u8]) -> Vec<(u64, Vec<u8>)> {
    let mut d = ChunkedRecordDecoder::new(CHUNK, MAX_RECORD_LEN).unwrap();
    let mut records = Vec::new();
    let mut at = 0;
    loop {
        let (n, item) = d.decode(&stream[at..]);
        at += n;
        match item {
            Some(Item::Record { offset, payload }) => records.push((offset, payload.to_vec())),
            Some(Item::Lost) => panic!("lost records"),
            None => return records,
        }
    }
}

/// The stream offset of each UNIT record of `stream`, by track and DTS,
/// and the offsets of every record, in order.
fn unit_offsets(stream: &[u8]) -> (BTreeMap<(TrackId, i64), u64>, Vec<u64>) {
    let records = records_of(stream);
    let mut units = BTreeMap::new();
    for (offset, r) in &records {
        if let Record::Unit { header, .. } = parse_record(r).unwrap() {
            units.insert((header.track, header.dts), *offset);
        }
    }
    (units, records.iter().map(|(o, _)| *o).collect())
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

/// A reader joining at a finished session's END starts there: the END is
/// a join point, so it yields the END alone, whether the record it reads
/// first is the END or the last GOP's, whose rewind is refused.
#[test]
fn a_reader_joining_at_the_end_reads_the_end() {
    let (stream, _, _) = ingested();
    let records = records_of(&stream);
    let (end, _) = records.last().unwrap();
    assert!(matches!(
        parse_record(&records.last().unwrap().1).unwrap(),
        Record::End { gop_back: 0 }
    ));
    let data = Rc::new(stream);
    let last_unit = records[records.len() - 2].0 as usize / CHUNK * CHUNK;
    for join in [*end as usize, last_unit] {
        let (read, m) = read_snapshot(Snapshot::new(&data, join / CHUNK * CHUNK));
        assert_eq!(read.ends, 1, "joined at {join}");
        assert!(read.units.is_empty(), "joined at {join}");
        assert_eq!(m.records, 1, "joined at {join}");
    }
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

/// Pictures per GOP in the default script: 2 s at 30 fps.
const GOP: usize = 60;

/// What a stream yielded: its records whole, and what a unit reader made
/// of them.
#[derive(Default)]
struct Got {
    records: Vec<Vec<u8>>,
    record: Vec<u8>,
    reader: UnitReader,
    read: Read,
}

impl Got {
    fn take(&mut self, piece: Result<EgressPiece<'_>, VideoError>) {
        match piece {
            Ok(EgressPiece::Data(d)) => self.record.extend_from_slice(d),
            Ok(EgressPiece::RecordEnd) => {
                let r = std::mem::take(&mut self.record);
                self.read.record(&mut self.reader, &r);
                self.records.push(r);
            }
            Ok(EgressPiece::Discontinuity) => {
                panic!("a discontinuity after {} records", self.records.len())
            }
            Err(e) => panic!("{e}"),
        }
    }

    /// Record `i`, parsed.
    fn record(&self, i: usize) -> Record<'_> {
        parse_record(&self.records[i]).unwrap()
    }

    /// Asserts that record `i` is a GOP's sync record and the next is its
    /// keyframe, `key`.
    fn assert_gop_start(&self, i: usize, key: &Unit) {
        assert!(
            matches!(self.record(i), Record::Configs { gop_back: 0, .. }),
            "record {i}: {:?}",
            self.record(i)
        );
        let Record::Unit { header, .. } = self.record(i + 1) else {
            panic!("record {}: {:?}", i + 1, self.record(i + 1));
        };
        assert_eq!(
            (header.track, header.keyframe, header.dts),
            (key.0, true, key.1)
        );
    }
}

/// A viewer's stream on b.
struct Viewer {
    stream: Stream<NoVideoHost>,
    got: Got,
}

impl Viewer {
    /// Takes the stream's first piece, which comes once it joined: how long
    /// that took.
    async fn join(&mut self) -> Duration {
        let start = tokio::time::Instant::now();
        let piece = tokio::time::timeout(Duration::from_secs(10), self.stream.next(usize::MAX))
            .await
            .expect("a first piece in time");
        self.got.take(piece);
        start.elapsed()
    }

    /// Reads until the unit reader returned `last`.
    async fn until(&mut self, last: &Unit) {
        while self.got.read.units.last() != Some(last) {
            let piece = tokio::time::timeout(Duration::from_secs(10), self.stream.next(usize::MAX))
                .await
                .expect("a piece in time");
            self.got.take(piece);
        }
    }

    /// Reads what the stream holds now.
    fn catch_up(&mut self) {
        while let Some(piece) = self.stream.next(usize::MAX).now_or_never() {
            self.got.take(piece);
        }
    }

    /// The reader's rewinds, refused rewinds and discontinuities.
    fn rewinds(&self) -> (u64, u64, u64) {
        let m = self.stream.metrics();
        (m.rewinds, m.refused_rewinds, m.discontinuities)
    }

    /// The reader's rewinds a later sync record overtook.
    fn overtaken(&self) -> u64 {
        self.stream.metrics().overtaken_rewinds
    }
}

/// A unit-format ingest of the default script, sent in steps, and viewed
/// from b, whose readers start at a stream's newest chunk as live joins do.
struct Live {
    s: Ingest,
    vb: Control<NoVideoHost>,
    /// `None` while a task sends.
    p: Option<TestPublisher>,
    script: Rc<Script>,
    configs: Vec<DecoderConfig>,
    units: Vec<Unit>,
    media: Media,
    /// The tags sent so far.
    sent: usize,
}

impl Live {
    /// The ingest, sent through its first GOP and listed on b.
    async fn new() -> Live {
        let s = Ingest::new().await;
        s.b.host.open_readers_live(true);
        let vb = start(&s.b, NoVideoHost, Config::default());
        let p = s.publish().await;
        let script = Script::new(&ScriptConfig::default());
        let (configs, units) = script_units(&script);
        let mut live = Live {
            s,
            vb,
            p: Some(p),
            script: Rc::new(script),
            configs,
            units,
            media: Media::default(),
            sent: 0,
        };
        live.send_to(live.after_picture(0, GOP - 1)).await;
        wait_for(10, "the listing", || !media(&live.s.b).is_empty()).await;
        live.media = media(&live.s.b).remove(0);
        live
    }

    /// The index just past the tag of picture `picture` of GOP `gop`.
    fn after_picture(&self, gop: usize, picture: usize) -> usize {
        // The first video tag is the sequence header.
        let (i, _) = self
            .script
            .tags
            .iter()
            .enumerate()
            .filter(|(_, t)| t.kind == TagKind::Video)
            .nth(1 + gop * GOP + picture)
            .unwrap();
        i + 1
    }

    fn all(&self) -> usize {
        self.script.tags.len()
    }

    fn last(&self) -> Unit {
        self.units.last().unwrap().clone()
    }

    /// The index in the script's units of GOP `gop`'s keyframe.
    fn keyframe(&self, gop: usize) -> usize {
        keyframe(&self.units, gop)
    }

    /// Sends the tags from the last one sent to `to` in a task of its own,
    /// while the test reads.
    fn sending(&mut self, to: usize) -> JoinHandle<TestPublisher> {
        let mut p = self.p.take().expect("the publisher");
        let (from, script) = (self.sent, self.script.clone());
        self.sent = to;
        tokio::task::spawn_local(async move {
            p.send_tags(&script.tags[from..to], &script).await;
            p
        })
    }

    async fn sent(&mut self, task: JoinHandle<TestPublisher>) {
        self.p = Some(task.await.unwrap());
    }

    async fn send_to(&mut self, to: usize) {
        let task = self.sending(to);
        self.sent(task).await;
    }

    /// A stream of the listing on b.
    async fn open(&self) -> Viewer {
        let stream = self
            .vb
            .open_stream(
                &self.media.swarm_uri,
                std::slice::from_ref(&self.s.net.public),
                &self.media.mime_type,
            )
            .await
            .unwrap();
        Viewer {
            stream,
            got: Got::default(),
        }
    }

    /// The start of the stream's newest chunk, where a live reader that
    /// reads first now starts.
    fn newest_chunk(&self) -> u64 {
        (self.s.written() - 1) / CHUNK as u64 * CHUNK as u64
    }

    /// Narrows the swarm's live window, its log's, to `bytes`.
    fn narrow_window(&self, bytes: usize) {
        let w = self.s.world.0.borrow();
        let log = w.logs.get(&(self.s.swarm_id(), String::new())).unwrap();
        log.borrow_mut().window = bytes;
    }
}

/// Where a live reader starting at chunk offset `newest` joins: the first
/// record at or after it, and its GOP's sync record.
fn join_point(records: &[(u64, Vec<u8>)], newest: u64) -> (u64, u64) {
    let (first, r) = records.iter().find(|(o, _)| *o >= newest).unwrap();
    (*first, first - parse_record(r).unwrap().gop_back())
}

/// The offset of the first sync record after offset `after`.
fn next_sync(records: &[(u64, Vec<u8>)], after: u64) -> u64 {
    records
        .iter()
        .find(|(o, r)| *o > after && parse_record(r).unwrap().gop_back() == 0)
        .unwrap()
        .0
}

/// The records from offset `from` on.
fn records_from(records: &[(u64, Vec<u8>)], from: u64) -> Vec<Vec<u8>> {
    records
        .iter()
        .filter(|(o, _)| *o >= from)
        .map(|(_, r)| r.clone())
        .collect()
}

/// A stream opened half way through a GOP rewinds a swarm reader to the
/// GOP's sync record: it yields that CONFIGS record first, then the GOP's
/// keyframe, then every record after them in order, whether the rewind
/// answers at once or late but within its grace after the next GOP's sync
/// record arrived (with the rest of the stream, at once).
#[test]
fn a_viewer_joining_mid_gop_starts_at_its_gops_sync_record() {
    for delay in [Duration::ZERO, Duration::from_millis(500)] {
        run(async move {
            let mut live = Live::new().await;
            live.s.b.host.delay_rewinds(delay);
            live.send_to(live.after_picture(2, 45)).await;
            let mut v = live.open().await;
            let newest = live.newest_chunk();
            let rest = live.sending(live.all());
            let took = v.join().await;
            v.until(&live.last()).await;
            live.sent(rest).await;

            let records = records_of(&live.s.stream());
            let (first, k) = join_point(&records, newest);
            assert!(k < first, "{delay:?}: joined at a sync record");
            assert_eq!(v.got.records, records_from(&records, k), "{delay:?}");
            v.got.assert_gop_start(0, &live.units[live.keyframe(2)]);
            assert_eq!(v.got.read.units, live.units[live.keyframe(2)..]);
            assert_eq!(v.got.read.configs, live.configs);
            assert!(v.got.read.gaps.is_empty());
            assert_eq!(v.rewinds(), (1, 0, 0), "{delay:?}");
            assert!(took >= delay, "{delay:?}: joined after {took:?}");
            assert_eq!(live.s.b.host.cancelled_rewinds(), 0);
        });
    }
}

/// The live window, narrowed for this viewer to less than a GOP.
const NARROW_WINDOW: usize = 128 * 1024;

/// A stream opened half way through a GOP whose sync record left the live
/// window can't rewind: it starts at the next GOP's sync record and
/// keyframe, and drops the records before them.
#[test]
fn a_viewer_whose_gop_left_the_window_starts_at_the_next_keyframe() {
    run(async {
        let mut live = Live::new().await;
        live.narrow_window(NARROW_WINDOW);
        live.send_to(live.after_picture(2, 45)).await;
        let mut v = live.open().await;
        // The window is narrow: the stream is read while the rest is sent.
        let rest = live.sending(live.all());
        v.join().await;
        v.until(&live.last()).await;
        live.sent(rest).await;

        v.got.assert_gop_start(0, &live.units[live.keyframe(3)]);
        assert_eq!(v.got.read.units, live.units[live.keyframe(3)..]);
        assert_eq!(v.got.read.configs, live.configs);
        assert!(v.got.read.gaps.is_empty());
        assert_eq!(v.rewinds(), (0, 1, 0));
        assert_eq!(live.s.b.host.cancelled_rewinds(), 0);
    });
}

/// A stream whose rewind doesn't answer within its grace after the next
/// GOP's sync record arrived drops it, which cancels it, and starts at that
/// sync record and keyframe.
#[test]
fn a_viewer_whose_rewind_answers_late_starts_at_the_next_keyframe_and_cancels_it() {
    run(async {
        let mut live = Live::new().await;
        let late = Duration::from_secs(5);
        live.s.b.host.delay_rewinds(late);
        live.send_to(live.after_picture(2, 45)).await;
        let mut v = live.open().await;
        let newest = live.newest_chunk();
        let rest = live.sending(live.all());
        let took = v.join().await;
        let grace = EgressSettings::default().rewind_grace.to_std().unwrap();
        assert!(took >= grace && took < late, "joined after {took:?}");
        assert_eq!(live.s.b.host.cancelled_rewinds(), 1);
        v.until(&live.last()).await;
        live.sent(rest).await;

        let records = records_of(&live.s.stream());
        let (first, k) = join_point(&records, newest);
        assert!(k < first, "joined at a sync record");
        assert_eq!(
            v.got.records,
            records_from(&records, next_sync(&records, first))
        );
        v.got.assert_gop_start(0, &live.units[live.keyframe(3)]);
        assert_eq!(v.got.read.units, live.units[live.keyframe(3)..]);
        assert_eq!(v.got.read.configs, live.configs);
        assert_eq!(v.rewinds(), (0, 0, 0));
        assert_eq!(v.overtaken(), 1);
        assert_eq!(live.s.b.host.cancelled_rewinds(), 1);
    });
}

/// Two streams of one swarm on a node each have their own swarm reader: a
/// stream that joins later and rewinds to its GOP's sync record leaves the
/// first stream, which read past that record, where it was.
#[test]
fn a_second_stream_rewinding_leaves_the_first_where_it_was() {
    run(async {
        let mut live = Live::new().await;
        live.send_to(live.after_picture(1, 30)).await;
        let mut first = live.open().await;
        let newest_first = live.newest_chunk();
        let more = live.sending(live.after_picture(3, 30));
        first.join().await;
        live.sent(more).await;
        first.catch_up();
        let read_first = first.got.records.len();

        let mut second = live.open().await;
        let newest_second = live.newest_chunk();
        let rest = live.sending(live.all());
        second.join().await;
        let swarms = live.vb.video_swarms();
        assert_eq!(swarms.len(), 1);
        assert_eq!(swarms[0].streams, 2);
        first.until(&live.last()).await;
        second.until(&live.last()).await;
        live.sent(rest).await;

        let records = records_of(&live.s.stream());
        let (_, k_first) = join_point(&records, newest_first);
        let (at_second, k_second) = join_point(&records, newest_second);
        assert!(k_second < at_second, "joined at a sync record");
        // The first stream had read past the second's sync record.
        let (last_read, _) = records
            .iter()
            .filter(|(o, _)| *o >= k_first)
            .nth(read_first - 1)
            .unwrap();
        assert!(*last_read > k_second, "{last_read} <= {k_second}");

        assert_eq!(first.got.records, records_from(&records, k_first));
        assert_eq!(second.got.records, records_from(&records, k_second));
        assert_eq!(first.got.read.units, live.units[live.keyframe(1)..]);
        assert_eq!(second.got.read.units, live.units[live.keyframe(3)..]);
        assert_eq!(first.rewinds(), (1, 0, 0));
        assert_eq!(second.rewinds(), (1, 0, 0));
    });
}
