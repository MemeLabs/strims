// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use strims_transfer::{ReadFailure, RewindFailure, SwarmRead, SwarmWrite};
use strims_video::amf0::Metadata;
use strims_video::synth::script::{self, Script, ScriptConfig};
use strims_video::units::UnitSigning;
use strims_video::units::record::{Record, parse_record};
use strims_video::{EgressPiece, unit_ingress_options};

use super::*;

/// An FLV file of `script`'s tags.
fn flv(script: &Script) -> Vec<u8> {
    let mut out = b"FLV\x01\x05\x00\x00\x00\x09".to_vec();
    out.extend_from_slice(&0u32.to_be_bytes());
    for t in &script.tags {
        let kind = match t.kind {
            script::TagKind::Audio => 8u8,
            script::TagKind::Video => 9,
            script::TagKind::Metadata => 18,
        };
        let len = t.body.len() as u32;
        out.push(kind);
        out.extend_from_slice(&len.to_be_bytes()[1..]);
        out.extend_from_slice(&t.timestamp.to_be_bytes()[1..]);
        out.push((t.timestamp >> 24) as u8);
        out.extend_from_slice(&[0, 0, 0]);
        out.extend_from_slice(&t.body);
        out.extend_from_slice(&(len + 11).to_be_bytes());
    }
    out
}

fn script(secs: u32) -> Script {
    Script::new(&ScriptConfig {
        fps: 25,
        gop_frames: 50,
        video_bitrate: 2_000_000,
        duration_secs: secs,
        b_frames: true,
        ..ScriptConfig::default()
    })
}

#[test]
fn an_flv_file_reads_as_its_tags_and_metadata() {
    let s = script(4);
    let file = FlvFile::parse(flv(&s)).unwrap();
    assert_eq!(file.tags().len(), s.tags.len());
    for (t, want) in file.tags().iter().zip(&s.tags) {
        assert_eq!(t.ts_ms, want.timestamp);
        assert_eq!(file.body(t), &want.body[..]);
    }
    let m = file.metadata().unwrap();
    assert_eq!(m.videodatarate, Some(2000.0));
    assert!(file.has(TagKind::Video) && file.has(TagKind::Audio));
}

#[test]
fn a_cut_flv_file_keeps_its_whole_tags_and_bad_ones_are_refused() {
    let s = script(2);
    let bytes = flv(&s);
    let cut = FlvFile::parse(bytes[..bytes.len() - 10].to_vec()).unwrap();
    assert_eq!(cut.tags().len(), s.tags.len() - 1);
    assert!(FlvFile::parse(b"MP4\x01".to_vec()).is_err());
    assert!(FlvFile::parse(b"FLV\x01\x05\x00\x00\x00\x02".to_vec()).is_err());
    // A tag over the ingest's message bound.
    let mut big = b"FLV\x01\x05\x00\x00\x00\x09\x00\x00\x00\x00\x09\xff\xff\xff".to_vec();
    big.extend_from_slice(&[0; 7]);
    assert!(FlvFile::parse(big).is_err());
    // Any bytes after the header parse without a panic.
    let mut x: u32 = 1;
    for n in 0..200usize {
        let mut b = b"FLV\x01\x05\x00\x00\x00\x09".to_vec();
        for _ in 0..n * 7 {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            b.push(x as u8);
        }
        let _ = FlvFile::parse(b);
    }
}

#[test]
fn a_file_signs_as_ingest_signs_its_stream() {
    let meta = |v: Option<f64>, a: Option<f64>| Metadata {
        videodatarate: v,
        audiodatarate: a,
        ..Metadata::default()
    };
    // A CBR stream, and a CRF one whose video rate is 0.
    let cbr = meta(Some(3000.0), Some(160.0));
    let crf = meta(Some(0.0), Some(160.0));
    assert_eq!(signing_of(Some(&cbr), true, true), UnitSigning::LARGE);
    assert_eq!(signing_of(Some(&crf), true, true), UnitSigning::LARGE);
    assert_eq!(
        signing_of(Some(&meta(Some(400.0), Some(96.0))), true, true),
        UnitSigning::SMALL
    );
    assert_eq!(
        signing_of(Some(&meta(None, Some(128.0))), false, true),
        UnitSigning::SMALL
    );
    assert_eq!(signing_of(None, true, true), UnitSigning::LARGE);
}

#[test]
fn the_seed_takes_ingest_options_and_viewers_the_egress_window() {
    let seed = seed_options(8).unwrap();
    assert_eq!(seed, unit_ingress_options(&UnitSigning::LARGE));
    let viewer = viewer_options(8).unwrap();
    assert_eq!(
        viewer.live_window * viewer.chunk_size,
        EgressSettings::default().window_bytes
    );
    assert_eq!(viewer.uri_options(), seed.uri_options());
    assert!(seed_options(3).is_err());
}

#[test]
fn data_messages_end_at_record_ends_and_fill_to_their_bound() {
    let mut m = DataMessages::new(4);
    let mut out = Vec::new();
    let mut feed = |m: &mut DataMessages, p: EgressPiece<'_>| {
        if let Some(d) = m.piece(p) {
            out.push((d.data.to_vec(), d.segment_end, d.discontinuity));
        }
    };
    feed(&mut m, EgressPiece::Data(b"ab"));
    assert_eq!(m.room(), 2);
    feed(&mut m, EgressPiece::Data(b"cd"));
    assert_eq!(m.room(), 4);
    feed(&mut m, EgressPiece::Data(b"e"));
    feed(&mut m, EgressPiece::RecordEnd);
    feed(&mut m, EgressPiece::Data(b"xy"));
    feed(&mut m, EgressPiece::Discontinuity);
    feed(&mut m, EgressPiece::Data(b"z"));
    feed(&mut m, EgressPiece::RecordEnd);
    feed(&mut m, EgressPiece::RecordEnd);
    assert_eq!(
        out,
        vec![
            (b"abcd".to_vec(), false, false),
            (b"e".to_vec(), true, false),
            (b"z".to_vec(), true, true),
            (Vec::new(), true, false),
        ]
    );
}

/// A Data message's bytes, `segment_end` and `discontinuity`.
type Message = (Vec<u8>, bool, bool);

/// A clock that jumps to each sleep's end, and the Data messages taken.
#[derive(Clone, Default)]
struct Virtual {
    now: Rc<Cell<u64>>,
    media: Rc<RefCell<Vec<Message>>>,
}

impl Runtime for Virtual {
    fn now_ms(&self) -> u64 {
        self.now.get()
    }

    async fn sleep_until_ms(&self, at_ms: u64) {
        self.now.set(self.now.get().max(at_ms));
    }

    fn cpu_ms(&self) -> Option<u64> {
        None
    }

    fn emit(&self, _: String) {}

    fn error(&self, message: String) {
        panic!("{message}");
    }

    fn media(&self, data: &[u8], segment_end: bool, discontinuity: bool) {
        self.media
            .borrow_mut()
            .push((data.to_vec(), segment_end, discontinuity));
    }
}

/// A swarm stream in memory: flushes pad the unit with zeros.
struct Stream {
    bytes: Rc<RefCell<Vec<u8>>>,
    unit: usize,
}

impl SwarmWrite for Stream {
    type Error = String;

    async fn write(&mut self, data: &[u8]) -> Result<(), String> {
        self.bytes.borrow_mut().extend_from_slice(data);
        Ok(())
    }

    async fn flush(&mut self) -> Result<u64, String> {
        let mut b = self.bytes.borrow_mut();
        let end = b.len().next_multiple_of(self.unit);
        b.resize(end, 0);
        Ok(end as u64)
    }

    async fn reset(&mut self) -> Result<(), String> {
        Err("no resets".to_owned())
    }
}

/// Reads a whole stream from `next`, a chunk start, in chunks, and
/// rewinds anywhere before it.
#[derive(Clone)]
struct Reader {
    bytes: Vec<u8>,
    chunk: usize,
    next: usize,
}

impl SwarmRead for Reader {
    type Error = String;

    async fn read_at(&mut self, out: &mut [u8]) -> Result<(u64, usize), ReadFailure<String>> {
        let n = self.chunk.min(out.len()).min(self.bytes.len() - self.next);
        if n == 0 {
            return Err(ReadFailure::Closed);
        }
        out[..n].copy_from_slice(&self.bytes[self.next..self.next + n]);
        let at = self.next as u64;
        self.next += n;
        Ok((at, n))
    }

    async fn rewind(&mut self, offset: u64) -> Result<u64, RewindFailure<String>> {
        let at = offset as usize - offset as usize % self.chunk;
        self.next = self.next.min(at);
        Ok(at as u64)
    }
}

/// `stream`'s records from `messages`, which must begin one.
fn records(messages: &[Message]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    for (data, end, discontinuity) in messages {
        assert!(!discontinuity);
        cur.extend_from_slice(data);
        if *end {
            out.push(std::mem::take(&mut cur));
        }
    }
    out
}

#[test]
fn a_viewer_joining_mid_stream_gets_whole_records_from_a_gop_start() {
    let s = script(6);
    let file = FlvFile::parse(flv(&s)).unwrap();
    let signing = signing_of(file.metadata().as_ref(), true, true);
    assert_eq!(signing, UnitSigning::LARGE);
    let chunk = seed_options(signing.chunks_per_signature)
        .unwrap()
        .chunk_size;
    let bytes = Rc::new(RefCell::new(Vec::new()));
    let mut swarm = Stream {
        bytes: bytes.clone(),
        unit: chunk * signing.chunks_per_signature,
    };
    let rt = Virtual::default();
    let start = 1_800_000_000_000;
    rt.now.set(start - 1000);
    let late = Cell::new(None);
    let stats = Cell::new(SeedStats::default());
    block_on(async {
        write(
            &mut swarm,
            chunk,
            signing,
            &file,
            start,
            start + 60_000,
            &rt,
            &late,
            &stats,
        )
        .await
        .unwrap();
    });
    let st = stats.get();
    assert!(st.writer.units > 0 && st.writer.gops == 3, "{st:?}");
    assert_eq!(late.get(), Some(0));
    let bytes = bytes.borrow().clone();
    assert_eq!(bytes.len() % (chunk * signing.chunks_per_signature), 0);

    // Join 3.5 s in, inside the second GOP.
    let reader = Reader {
        next: bytes.len() * 35 / 60 / chunk * chunk,
        bytes,
        chunk,
    };
    let lags = RefCell::new(Vec::new());
    let viewer = RefCell::new(ViewerStats::default());
    block_on(async {
        let spare = reader.clone();
        read(reader, Some(spare), chunk, &rt, &lags, &viewer)
            .await
            .unwrap();
    });
    let v = viewer.borrow();
    assert_eq!(v.join.rewind, Some(RewindOutcome::Granted));
    assert!(v.join.rewind_back.unwrap() > 0);
    assert_eq!(v.reader.losses, 0);
    let recs = records(&rt.media.borrow());
    let first = parse_record(&recs[0]).unwrap();
    assert!(
        matches!(first, Record::Configs { gop_back: 0, .. }),
        "{first:?}"
    );
    let mut published = Vec::new();
    for r in &recs {
        if let Record::Unit { header, .. } = parse_record(r).unwrap() {
            published.push(header.published.unix_millis() as u64 - start);
        }
    }
    // The units from the second GOP's keyframe on, in tag order, each
    // published at its tag time.
    assert_eq!(v.units as usize, published.len());
    assert_eq!(lags.borrow().len(), published.len());
    let key = s
        .tags
        .iter()
        .position(|t| {
            t.kind == script::TagKind::Video
                && t.body[0] >> 4 == 1
                && t.body[1] == 1
                && t.timestamp >= 2000
        })
        .unwrap();
    assert_eq!(s.tags[key].timestamp, 2000);
    let want: Vec<u64> = s.tags[key..]
        .iter()
        .filter(|t| t.kind != script::TagKind::Metadata && t.body[1] != 0)
        .map(|t| u64::from(t.timestamp))
        .collect();
    assert_eq!(published, want);
}

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(f)
}
