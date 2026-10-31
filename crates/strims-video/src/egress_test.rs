// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::task::Poll;

use futures_util::FutureExt;
use strims_chunkstream::chunked::{ChunkedRecordDecoder, Item};
use strims_chunkstream::packed::PackedEncoder;

use super::*;
use crate::Muxer;
use crate::units::UnitWriter;
use crate::units::test_support::{audio, unit, video};

/// A scripted read: the offset and bytes, or the failure.
type Read = Result<(u64, Vec<u8>), ReadFailure<String>>;

/// A swarm reader that plays a script of reads, then reports the swarm
/// closed.
#[derive(Default)]
struct Script {
    reads: VecDeque<Read>,
    /// What a rewind answers, and the reads after it when it moves.
    rewind: Option<(Option<u64>, VecDeque<Read>)>,
    /// The offsets rewinds were asked for.
    asked: Rc<RefCell<Vec<u64>>>,
    /// With a budget, reads past it wait until it grows.
    budget: Option<Rc<Cell<usize>>>,
    /// With a gate, a rewind answers once it opens.
    release: Option<Rc<Cell<bool>>>,
    /// With a gate, the grace passes once it opens; without, at once.
    grace: Option<Rc<Cell<bool>>>,
    /// Rewinds dropped before they answered.
    dropped: Rc<Cell<u64>>,
}

/// Waits until `gate` opens.
async fn opened(gate: &Rc<Cell<bool>>) {
    poll_fn(|_| {
        if gate.get() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
}

/// Counts a rewind dropped while armed.
struct DropCount {
    count: Rc<Cell<u64>>,
    armed: bool,
}

impl Drop for DropCount {
    fn drop(&mut self) {
        if self.armed {
            self.count.set(self.count.get() + 1);
        }
    }
}

impl Script {
    /// Reads of `stream[from..to]` in pieces of `chunk` bytes.
    fn reads(mut self, stream: &[u8], from: usize, to: usize, chunk: usize) -> Script {
        let mut at = from;
        while at < to {
            let end = to.min(at + chunk);
            self.reads
                .push_back(Ok((at as u64, stream[at..end].to_vec())));
            at = end;
        }
        self
    }

    fn fail(mut self, f: ReadFailure<String>) -> Script {
        self.reads.push_back(Err(f));
        self
    }

    /// A rewind answers `answer`, then reads `then`'s reads when it moved.
    fn rewound(mut self, answer: Option<u64>, then: Script) -> Script {
        self.rewind = Some((answer, then.reads));
        self
    }
}

impl Rewind for Script {
    async fn rewind(&mut self, offset: u64) -> Option<u64> {
        self.asked.borrow_mut().push(offset);
        let mut armed = DropCount {
            count: self.dropped.clone(),
            armed: true,
        };
        if let Some(gate) = &self.release {
            opened(gate).await;
        }
        armed.armed = false;
        let (answer, reads) = self.rewind.take()?;
        if answer.is_some() {
            self.reads = reads;
        }
        answer
    }

    fn grace(&self) -> impl Future<Output = ()> + 'static {
        let gate = self.grace.clone();
        async move {
            if let Some(gate) = gate {
                opened(&gate).await;
            }
        }
    }
}

impl SwarmRead for Script {
    type Error = String;

    async fn read_at(&mut self, out: &mut [u8]) -> Result<(u64, usize), ReadFailure<String>> {
        if let Some(budget) = &self.budget {
            poll_fn(|_| {
                if budget.get() > 0 {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;
            budget.set(budget.get() - 1);
        }
        let (offset, data) = match self.reads.pop_front() {
            None => return Err(ReadFailure::Closed),
            Some(r) => r?,
        };
        let n = data.len().min(out.len());
        out[..n].copy_from_slice(&data[..n]);
        if n < data.len() {
            self.reads
                .push_front(Ok((offset + n as u64, data[n..].to_vec())));
        }
        Ok((offset, n))
    }
}

/// `n` bytes of record `i`.
fn record(i: u8, n: usize) -> Vec<u8> {
    (0..n)
        .map(|j| i.wrapping_mul(31).wrapping_add(j as u8))
        .collect()
}

/// The packed stream of `records`, and each record's start offset.
fn packed(records: &[Vec<u8>]) -> (Vec<u8>, Vec<usize>) {
    let mut e = PackedEncoder::new(DEFAULT_SIZE).unwrap();
    let mut out = Vec::new();
    let mut starts = Vec::new();
    for r in records {
        starts.push(e.offset() as usize);
        e.write(r, &mut out);
        e.end_record(&mut out).unwrap();
    }
    (out, starts)
}

/// Records over a frame each, so each ends in a later frame than the one
/// before; the first fills its frame exactly.
fn records(n: u8) -> Vec<Vec<u8>> {
    (0..n)
        .map(|i| match i {
            0 => record(0, DEFAULT_SIZE - 4),
            i => record(i, 70_000 + usize::from(i) * 1000),
        })
        .collect()
}

#[derive(Debug, PartialEq)]
enum Out {
    Record(Vec<u8>),
    Discontinuity,
}

/// What `r` yields until its swarm closes, with each data piece at most
/// `max` bytes, and the failure that ended it. Bytes after the last end
/// are dropped.
fn collect<R: SwarmRead>(r: &mut PackedReader<R>, max: usize) -> (Vec<Out>, VideoError) {
    let mut out = Vec::new();
    let mut rec = Vec::new();
    loop {
        match r.next(max).now_or_never().expect("the script never waits") {
            Ok(EgressPiece::Data(d)) => {
                assert!(!d.is_empty() && d.len() <= max, "{} bytes", d.len());
                rec.extend_from_slice(d);
            }
            Ok(EgressPiece::RecordEnd) => out.push(Out::Record(std::mem::take(&mut rec))),
            Ok(EgressPiece::Discontinuity) => {
                rec.clear();
                out.push(Out::Discontinuity);
            }
            Err(e) => return (out, e),
        }
    }
}

fn whole(rs: &[Vec<u8>]) -> Vec<Out> {
    rs.iter().cloned().map(Out::Record).collect()
}

#[test]
fn records_read_from_the_start_come_whole() {
    let rs = records(4);
    let (s, _) = packed(&rs);
    for (chunk, max) in [
        (1000, 32 * 1024),
        (65_536, 100),
        (7, 1),
        (s.len(), usize::MAX),
    ] {
        let mut r = PackedReader::new(Script::default().reads(&s, 0, s.len(), chunk), 64 * 1024);
        let (out, end) = collect(&mut r, max);
        assert_eq!(out, whole(&rs), "chunk {chunk} max {max}");
        assert_eq!(end, VideoError::StreamClosed);
        assert_eq!(r.metrics().records, 4);
        assert_eq!(r.metrics().discontinuities, 0);
    }
}

#[test]
fn a_start_inside_a_record_drops_it() {
    let rs = records(4);
    let (s, starts) = packed(&rs);
    // Record 1 starts after frame 1's header, record 2 in frame 2, which
    // holds record 1's end.
    assert_eq!(starts[1], DEFAULT_SIZE + 4);
    assert!(starts[2] > 2 * DEFAULT_SIZE && starts[2] < 3 * DEFAULT_SIZE);
    // A start inside frame 2's data misses its header, so record 1's end
    // there too: record 2's end closes the partial record.
    for (from, first) in [(10_000, 1), (starts[1] + 1, 2), (starts[2] - 1, 3)] {
        let mut r = PackedReader::new(Script::default().reads(&s, from, s.len(), 4096), 64 * 1024);
        let (out, _) = collect(&mut r, 32 * 1024);
        assert_eq!(out, whole(&rs[first..]), "from {from}");
    }
}

#[test]
fn a_start_inside_a_header_drops_up_to_the_next_whole_record() {
    let rs = records(4);
    let (s, _) = packed(&rs);
    // Frame 1's header holds record 0's end; its data, record 1's start.
    let mut r = PackedReader::new(
        Script::default().reads(&s, DEFAULT_SIZE + 2, s.len(), 4096),
        64 * 1024,
    );
    let (out, _) = collect(&mut r, 32 * 1024);
    assert_eq!(out, whole(&rs[2..]));
}

#[test]
fn a_gap_between_reads_is_one_discontinuity_before_the_next_whole_record() {
    let rs = records(6);
    let (s, starts) = packed(&rs);
    let mid = |i: usize| (starts[i] + starts[i + 1]) / 2;
    let script = Script::default()
        .reads(&s, 0, mid(1), 5000)
        .reads(&s, mid(2), s.len(), 5000);
    let mut r = PackedReader::new(script, 64 * 1024);
    let (out, _) = collect(&mut r, 32 * 1024);
    let mut expected = whole(&rs[..1]);
    expected.push(Out::Discontinuity);
    expected.extend(whole(&rs[3..]));
    assert_eq!(out, expected);
    assert_eq!(r.metrics().discontinuities, 1);
}

#[test]
fn a_reported_discontinuity_restarts_at_the_readers_new_offset() {
    let old = records(3);
    let new: Vec<Vec<u8>> = (10..13).map(|i| record(i, 80_000)).collect();
    let (a, starts) = packed(&old);
    let (b, _) = packed(&new);
    let script = Script::default()
        .reads(&a, 0, starts[2] + 100, 3000)
        .fail(ReadFailure::Discontinuity)
        .reads(&b, 0, b.len(), 3000);
    let mut r = PackedReader::new(script, 64 * 1024);
    let (out, _) = collect(&mut r, 32 * 1024);
    let mut expected = whole(&old[..2]);
    expected.push(Out::Discontinuity);
    expected.extend(whole(&new));
    assert_eq!(out, expected);
}

#[test]
fn a_discontinuity_before_anything_was_yielded_is_not_reported() {
    let rs = records(4);
    let (s, starts) = packed(&rs);
    let script = Script::default()
        .reads(&s, 1000, 2000, 500)
        .fail(ReadFailure::Discontinuity)
        .reads(&s, starts[1] + 10, s.len(), 4096);
    let mut r = PackedReader::new(script, 64 * 1024);
    let (out, _) = collect(&mut r, 32 * 1024);
    assert_eq!(out, whole(&rs[2..]));
    assert_eq!(r.metrics().discontinuities, 1);
}

#[test]
fn a_corrupt_frame_drops_the_records_it_touches() {
    let rs = records(5);
    let (mut s, starts) = packed(&rs);
    // Frame 2 holds record 1's end and record 2's start; an end past the
    // frame is corrupt.
    let frame = 2 * DEFAULT_SIZE;
    assert!(starts[1] < frame && starts[2] > frame && starts[2] < frame + DEFAULT_SIZE);
    s[frame..frame + 4].copy_from_slice(&(0x8000_0000u32 | 0xffff).to_be_bytes());
    let mut r = PackedReader::new(Script::default().reads(&s, 0, s.len(), 4096), 64 * 1024);
    let (out, _) = collect(&mut r, 32 * 1024);
    let mut expected = whole(&rs[..1]);
    expected.push(Out::Discontinuity);
    expected.extend(whole(&rs[3..]));
    assert_eq!(out, expected);
    assert_eq!(r.metrics().corrupt_frames, 1);
}

#[test]
fn a_replaced_reader_reads_the_new_stream_after_a_discontinuity() {
    let old = records(3);
    let new = records(2);
    let (a, starts) = packed(&old);
    let (b, _) = packed(&new);
    let script = Script::default().reads(&a, 0, starts[2] + 100, 4096);
    let mut r = PackedReader::new(script, 64 * 1024);
    let (out, end) = collect(&mut r, 32 * 1024);
    assert_eq!((out, end), (whole(&old[..2]), VideoError::StreamClosed));
    r.replace(Script::default().reads(&b, 0, b.len(), 4096));
    let (out, _) = collect(&mut r, 32 * 1024);
    let mut expected = vec![Out::Discontinuity];
    expected.extend(whole(&new));
    assert_eq!(out, expected);
}

#[test]
fn a_failed_read_fails_one_call_and_reading_goes_on() {
    let rs = records(4);
    let (s, starts) = packed(&rs);
    let script = Script::default()
        .reads(&s, 0, starts[1] + 10, 4096)
        .fail(ReadFailure::Failed("disk".into()))
        .reads(&s, starts[2] + 10, s.len(), 4096);
    let mut r = PackedReader::new(script, 64 * 1024);
    let (out, e) = collect(&mut r, 32 * 1024);
    assert_eq!(out, whole(&rs[..1]));
    assert_eq!(e, VideoError::StreamRead("disk".into()));
    let (out, e) = collect(&mut r, 32 * 1024);
    let mut expected = vec![Out::Discontinuity];
    expected.extend(whole(&rs[3..]));
    assert_eq!((out, e), (expected, VideoError::StreamClosed));
}

#[test]
fn live_windows_are_the_most_chunks_that_fit_as_a_power_of_two() {
    let w = 32 * 1024 * 1024;
    assert_eq!(live_window(w, 1024), 32 * 1024);
    assert_eq!(live_window(w, 1000), 32 * 1024);
    assert_eq!(live_window(w, 65_536), 512);
    assert_eq!(live_window(10, 1024), 1);
    assert_eq!(live_window(usize::MAX, 1), 1 << 31);
}

#[test]
fn stream_options_take_the_uri_within_the_bounds() {
    let s = EgressSettings::default();
    let id = SwarmId::new(&[7; 32]);
    let uri = |q: &str| format!("magnet:?xt=urn:ppspp:{id}{q}");
    let (u, o) = check_stream(
        &s,
        &uri("&x.im=2&x.hf=4&x.sa=2&x.cs=1024&x.ps=32&x.sc=16"),
        &[],
    )
    .unwrap();
    assert_eq!(u.id, id);
    assert_eq!(
        (
            o.chunk_size,
            o.chunks_per_signature,
            o.stream_count,
            o.live_window
        ),
        (1024, 32, 16, 32 * 1024)
    );
    let (_, d) = check_stream(&s, &uri(""), &[]).unwrap();
    assert_eq!(
        (d.chunk_size, d.chunks_per_signature, d.stream_count),
        (1024, 64, 1)
    );

    let refused = [
        ("&x.cs=255", "chunk size"),
        ("&x.cs=65537", "chunk size"),
        ("&x.ps=8192", "chunks per signature"),
        ("&x.ps=3", "chunks per signature"),
        ("&x.cs=65536&x.ps=1024", "chunks per signature"),
        ("&x.sc=65", "stream count"),
        ("&x.im=9", "integrity protection method"),
        ("&x.im=258", "integrity protection method"),
        ("&x.hf=6", "merkle hash tree function"),
        ("&x.sa=3", "live signature algorithm"),
    ];
    for (q, what) in refused {
        assert_eq!(
            check_stream(&s, &uri(q), &[]).unwrap_err(),
            VideoError::SwarmOption(what),
            "{q}"
        );
    }
    let short = format!("magnet:?xt=urn:ppspp:{}", SwarmId::new(&[7; 16]));
    assert_eq!(
        check_stream(&s, &short, &[]).unwrap_err(),
        VideoError::SwarmOption("swarm id")
    );
    // Sign-all swarms take any count of chunks per signature.
    assert!(check_stream(&s, &uri("&x.im=3&x.ps=3"), &[]).is_ok());

    assert!(matches!(
        check_stream(&s, "not a uri", &[]),
        Err(VideoError::Uri(_))
    ));
    let long = uri(&"&x".repeat(1024));
    assert_eq!(
        check_stream(&s, &long, &[]).unwrap_err(),
        VideoError::UriTooLong
    );
    let keys = vec![vec![1; 32]; 17];
    assert_eq!(
        check_stream(&s, &uri(""), &keys).unwrap_err(),
        VideoError::NetworkKeyCount
    );
    assert_eq!(
        check_stream(&s, &uri(""), &[vec![1; 65]]).unwrap_err(),
        VideoError::NetworkKeyTooLong
    );
}

// --- the chunked layout -------------------------------------------------------

const CHUNK: usize = 4096;

/// A unit-format stream in 4 KiB frames: video at 30 fps in GOPs of 30
/// pictures, keyframes of 20 KiB and pictures of 2 KiB, and AAC frames
/// between, signed early every 25 units. Its records with their offsets.
fn units_stream(pictures: i64) -> (Vec<u8>, Vec<(u64, Vec<u8>)>) {
    let mut w = UnitWriter::new(CHUNK).unwrap();
    w.set_config(&video(1)).unwrap();
    w.set_config(&audio(2)).unwrap();
    let mut stream = Vec::new();
    let mut n = 0i64;
    let mut pushed = 0;
    let big = vec![0x5a; 20 * 1024];
    for k in 0..pictures {
        let key = k % 30 == 0;
        let mut u = unit(1, k * 3000, key, 2048);
        if key {
            u.payload = &big;
        }
        w.push(&u).unwrap();
        pushed += 1;
        while n * 1024 * 90_000 / 48_000 <= k * 3000 {
            w.push(&unit(2, n * 1024, true, 300)).unwrap();
            n += 1;
            pushed += 1;
        }
        if pushed % 25 == 0 {
            w.end_frame();
            stream.extend(w.take_output());
            let unit = 8 * CHUNK;
            let next = stream.len().div_ceil(unit) * unit;
            stream.resize(next, 0);
            w.continue_at(next as u64).unwrap();
        }
    }
    stream.extend(w.take_output());
    let mut d = ChunkedRecordDecoder::new(CHUNK, units::MAX_RECORD_LEN).unwrap();
    let mut records = Vec::new();
    let mut at = 0;
    loop {
        let (used, item) = d.decode(&stream[at..]);
        at += used;
        match item {
            Some(Item::Record { offset, payload }) => records.push((offset, payload.to_vec())),
            Some(Item::Lost) => panic!("lost"),
            None => break,
        }
    }
    (stream, records)
}

fn collect_chunked<R: SwarmRead + Rewind>(
    r: &mut ChunkedReader<R>,
    max: usize,
) -> (Vec<Out>, VideoError) {
    let mut out = Vec::new();
    let mut rec = Vec::new();
    loop {
        match r.next(max).now_or_never().expect("the script never waits") {
            Ok(EgressPiece::Data(d)) => {
                assert!(!d.is_empty() && d.len() <= max, "{} bytes", d.len());
                rec.extend_from_slice(d);
            }
            Ok(EgressPiece::RecordEnd) => out.push(Out::Record(std::mem::take(&mut rec))),
            Ok(EgressPiece::Discontinuity) => {
                rec.clear();
                out.push(Out::Discontinuity);
            }
            Err(e) => return (out, e),
        }
    }
}

fn records_from(records: &[(u64, Vec<u8>)], from: u64) -> Vec<Out> {
    records
        .iter()
        .filter(|(o, _)| *o >= from)
        .map(|(_, r)| Out::Record(r.clone()))
        .collect()
}

/// The offset of the GOP's sync record of the first record at or after
/// `from`'s next frame start.
fn gop_of_first(records: &[(u64, Vec<u8>)], from: u64) -> (u64, u64) {
    let frame = from.div_ceil(CHUNK as u64) * CHUNK as u64;
    let (offset, r) = records.iter().find(|(o, _)| *o >= frame).unwrap();
    let k = offset - units::record::parse_record(r).unwrap().gop_back();
    (*offset, k)
}

#[test]
fn unit_records_read_from_the_start_come_whole() {
    let (s, recs) = units_stream(70);
    for (chunk, max) in [
        (1000, 32 * 1024),
        (CHUNK, 100),
        (7, 1),
        (s.len(), usize::MAX),
    ] {
        let script = Script::default().reads(&s, 0, s.len(), chunk);
        let mut r = ChunkedReader::new(script, CHUNK, 64 * 1024).unwrap();
        let (out, end) = collect_chunked(&mut r, max);
        assert_eq!(out, records_from(&recs, 0), "chunk {chunk} max {max}");
        assert_eq!(end, VideoError::StreamClosed);
        let m = r.metrics();
        assert_eq!(
            (m.records, m.discontinuities, m.losses),
            (recs.len() as u64, 0, 0)
        );
        // The first record is a sync record: nothing to rewind to.
        assert_eq!((m.rewinds, m.refused_rewinds), (0, 0));
    }
}

#[test]
fn a_join_mid_gop_rewinds_to_its_sync_record() {
    let (s, recs) = units_stream(70);
    // Join in the second GOP, past its keyframe.
    let join = (recs
        .iter()
        .filter(|(_, r)| r.len() > 10_000)
        .nth(1)
        .unwrap()
        .0 as usize
        + 3 * CHUNK)
        / CHUNK
        * CHUNK;
    let (first, k) = gop_of_first(&recs, join as u64);
    assert!(k < first);
    let resume = k - k % CHUNK as u64;
    let asked = Rc::new(RefCell::new(Vec::new()));
    for max in [32 * 1024, 3] {
        let script = Script {
            asked: asked.clone(),
            ..Script::default().reads(&s, join, s.len(), 5000).rewound(
                Some(resume),
                Script::default().reads(&s, resume as usize, s.len(), 5000),
            )
        };
        let mut r = ChunkedReader::new(script, CHUNK, 64 * 1024).unwrap();
        let (out, _) = collect_chunked(&mut r, max);
        assert_eq!(out, records_from(&recs, k));
        let m = r.metrics();
        assert_eq!((m.rewinds, m.refused_rewinds, m.discontinuities), (1, 0, 0));
    }
    assert_eq!(*asked.borrow(), [k, k]);
}

#[test]
fn a_join_that_can_not_rewind_starts_at_the_next_sync_record() {
    let (s, recs) = units_stream(70);
    let join = s.len() / 2 / CHUNK * CHUNK;
    let (first, k) = gop_of_first(&recs, join as u64);
    assert!(k < first);
    let next = next_sync(&recs, first);
    // Refused, and an answer past the offset asked for, which can't be
    // used; by the reader itself or by a spare.
    for answer in [None, Some(first)] {
        for spared in [false, true] {
            let rewinding = Script::default().rewound(
                answer,
                Script::default().reads(&s, first as usize, s.len(), 5000),
            );
            let main = Script::default().reads(&s, join, s.len(), 5000);
            let mut r = if spared {
                let mut r = ChunkedReader::new(main, CHUNK, 64 * 1024).unwrap();
                r.set_spare(rewinding);
                r
            } else {
                let script = Script {
                    reads: main.reads,
                    ..rewinding
                };
                ChunkedReader::new(script, CHUNK, 64 * 1024).unwrap()
            };
            let (out, _) = collect_chunked(&mut r, 32 * 1024);
            assert_eq!(out, records_from(&recs, next), "{answer:?} {spared}");
            let m = r.metrics();
            assert_eq!(
                (m.rewinds, m.refused_rewinds, m.overtaken_rewinds),
                (0, 1, 0),
                "{answer:?} {spared}"
            );
        }
    }
}

/// A join in the second GOP, past its keyframe.
struct MidGop {
    s: Vec<u8>,
    recs: Vec<(u64, Vec<u8>)>,
    /// Where the reader starts.
    join: usize,
    /// The first whole record, its GOP's sync record, where a rewind to it
    /// resumes, and the next GOP's sync record.
    first: u64,
    k: u64,
    resume: u64,
    next: u64,
}

fn mid_gop() -> MidGop {
    let (s, recs) = units_stream(70);
    let join = (recs
        .iter()
        .filter(|(_, r)| r.len() > 10_000)
        .nth(1)
        .unwrap()
        .0 as usize
        + 3 * CHUNK)
        / CHUNK
        * CHUNK;
    let (first, k) = gop_of_first(&recs, join as u64);
    assert!(k < first);
    let next = next_sync(&recs, first);
    MidGop {
        resume: k - k % CHUNK as u64,
        s,
        recs,
        join,
        first,
        k,
        next,
    }
}

impl MidGop {
    /// The stream's reader, from the join on.
    fn reader(&self) -> Script {
        Script::default().reads(&self.s, self.join, self.s.len(), 5000)
    }

    /// A spare whose rewind moves it to the GOP's sync record.
    fn spare(&self) -> Script {
        Script::default().rewound(
            Some(self.resume),
            Script::default().reads(&self.s, self.resume as usize, self.s.len(), 5000),
        )
    }
}

/// The offset of the first sync record after offset `after`.
fn next_sync(records: &[(u64, Vec<u8>)], after: u64) -> u64 {
    records
        .iter()
        .find(|(o, r)| *o > after && units::record::parse_record(r).unwrap().gop_back() == 0)
        .unwrap()
        .0
}

/// Takes `r`'s pieces into `out` until a call waits (and is dropped) or
/// fails: the failure, if one ended it.
fn pull<R: SwarmRead + Rewind>(
    r: &mut ChunkedReader<R>,
    out: &mut Vec<Out>,
    rec: &mut Vec<u8>,
) -> Option<VideoError> {
    loop {
        match r.next(32 * 1024).now_or_never()? {
            Ok(EgressPiece::Data(d)) => rec.extend_from_slice(d),
            Ok(EgressPiece::RecordEnd) => out.push(Out::Record(std::mem::take(rec))),
            Ok(EgressPiece::Discontinuity) => {
                rec.clear();
                out.push(Out::Discontinuity);
            }
            Err(e) => return Some(e),
        }
    }
}

/// The reader's rewinds that landed, were refused and were overtaken.
fn joins<R: SwarmRead + Rewind>(r: &ChunkedReader<R>) -> (u64, u64, u64) {
    let m = r.metrics();
    (m.rewinds, m.refused_rewinds, m.overtaken_rewinds)
}

#[test]
fn a_join_whose_spare_rewinds_first_starts_at_its_gops_sync_record() {
    let g = mid_gop();
    let asked = Rc::new(RefCell::new(Vec::new()));
    let mut r = ChunkedReader::new(g.reader(), CHUNK, 64 * 1024).unwrap();
    r.set_spare(Script {
        asked: asked.clone(),
        ..g.spare()
    });
    let (out, end) = collect_chunked(&mut r, 32 * 1024);
    assert_eq!(out, records_from(&g.recs, g.k));
    assert_eq!(end, VideoError::StreamClosed);
    assert_eq!(joins(&r), (1, 0, 0));
    assert_eq!(*asked.borrow(), [g.k]);
}

/// The next GOP's sync record comes while the spare's rewind waits: once
/// the rewind's grace passes, the join starts there and drops the rewind;
/// a rewind that lands within the grace wins.
#[test]
fn a_join_takes_the_next_sync_record_once_the_rewinds_grace_passes() {
    for lands in [false, true] {
        let g = mid_gop();
        let (release, grace, dropped) = (Rc::default(), Rc::default(), Rc::default());
        let mut r = ChunkedReader::new(g.reader(), CHUNK, 64 * 1024).unwrap();
        r.set_spare(Script {
            release: Some(Rc::clone(&release)),
            dropped: Rc::clone(&dropped),
            ..g.spare()
        });
        let r_grace = Rc::clone(&grace);
        let main = std::mem::take(&mut r.reader);
        r.reader = Script {
            grace: Some(r_grace),
            ..main
        };
        let (mut out, mut rec) = (Vec::new(), Vec::new());
        // The reader is at the next sync record, the grace running.
        assert_eq!(pull(&mut r, &mut out, &mut rec), None);
        assert!(out.is_empty() && rec.is_empty());
        if lands {
            release.set(true);
        } else {
            grace.set(true);
        }
        assert_eq!(
            pull(&mut r, &mut out, &mut rec),
            Some(VideoError::StreamClosed)
        );
        if lands {
            assert_eq!(out, records_from(&g.recs, g.k));
            assert_eq!(joins(&r), (1, 0, 0));
            assert_eq!(dropped.get(), 0);
        } else {
            assert_eq!(out, records_from(&g.recs, g.next));
            assert_eq!(joins(&r), (0, 0, 1));
            assert_eq!(dropped.get(), 1, "the rewind dropped");
        }
    }
}

/// Calls dropped while the join races, a read and the rewind both waiting,
/// lose neither the spare nor its rewind: once it lands, the stream starts
/// at the GOP's sync record, after one rewind.
#[test]
fn a_call_dropped_while_its_join_races_keeps_the_spare() {
    let g = mid_gop();
    // Reads up to the first record's head, and none to the next sync record.
    let reads = (g.first as usize - g.join) / 5000 + 2;
    assert!(g.join + reads * 5000 < g.next as usize);
    let (budget, release, asked) = (Rc::new(Cell::new(reads)), Rc::default(), Rc::default());
    let mut r = ChunkedReader::new(
        Script {
            budget: Some(Rc::clone(&budget)),
            ..g.reader()
        },
        CHUNK,
        64 * 1024,
    )
    .unwrap();
    r.set_spare(Script {
        release: Some(Rc::clone(&release)),
        asked: Rc::clone(&asked),
        ..g.spare()
    });
    let (mut out, mut rec) = (Vec::new(), Vec::new());
    for _ in 0..3 {
        assert_eq!(pull(&mut r, &mut out, &mut rec), None);
    }
    assert!(out.is_empty());
    assert_eq!(*asked.borrow(), [g.k]);
    release.set(true);
    assert_eq!(
        pull(&mut r, &mut out, &mut rec),
        Some(VideoError::StreamClosed)
    );
    assert_eq!(out, records_from(&g.recs, g.k));
    assert_eq!(joins(&r), (1, 0, 0));
    assert_eq!(*asked.borrow(), [g.k]);
    // The reader read no more: the rest came from the spare.
    assert_eq!(budget.get(), 0);
}

/// A reader's own rewind whose call is dropped counts as refused, and the
/// join waits for the next sync record.
#[test]
fn a_dropped_rewind_waits_for_the_next_sync_record() {
    let g = mid_gop();
    let (release, dropped) = (Rc::default(), Rc::default());
    let script = Script {
        reads: g.reader().reads,
        release: Some(Rc::clone(&release)),
        dropped: Rc::clone(&dropped),
        ..g.spare()
    };
    let mut r = ChunkedReader::new(script, CHUNK, 64 * 1024).unwrap();
    let (mut out, mut rec) = (Vec::new(), Vec::new());
    assert_eq!(pull(&mut r, &mut out, &mut rec), None);
    assert_eq!(dropped.get(), 1);
    assert_eq!(
        pull(&mut r, &mut out, &mut rec),
        Some(VideoError::StreamClosed)
    );
    assert_eq!(out, records_from(&g.recs, g.next));
    assert_eq!(joins(&r), (0, 1, 0));
}

#[test]
fn a_join_at_a_sync_record_drops_its_spare() {
    let (s, recs) = units_stream(70);
    let asked = Rc::new(RefCell::new(Vec::new()));
    let mut r = ChunkedReader::new(
        Script::default().reads(&s, 0, s.len(), 5000),
        CHUNK,
        64 * 1024,
    )
    .unwrap();
    r.set_spare(Script {
        asked: asked.clone(),
        ..Script::default()
    });
    let (out, _) = collect_chunked(&mut r, 32 * 1024);
    assert_eq!(out, records_from(&recs, 0));
    assert!(r.spare.is_none());
    assert!(asked.borrow().is_empty());
    assert_eq!(joins(&r), (0, 0, 0));
}

#[test]
fn a_lost_chunk_is_one_discontinuity_before_the_next_whole_record() {
    let (s, recs) = units_stream(70);
    // Lose a chunk inside the second GOP's keyframe.
    let key = recs
        .iter()
        .filter(|(_, r)| r.len() > 10_000)
        .nth(1)
        .unwrap()
        .0 as usize;
    let lost = (key / CHUNK + 2) * CHUNK;
    let script = Script::default()
        .reads(&s, 0, lost, 3000)
        .reads(&s, lost + CHUNK, s.len(), 3000);
    let mut r = ChunkedReader::new(script, CHUNK, 64 * 1024).unwrap();
    let (out, _) = collect_chunked(&mut r, 32 * 1024);
    let mut expected = records_from(&recs, 0);
    expected.retain(|o| match o {
        Out::Record(r) => recs
            .iter()
            .any(|(off, rr)| rr == r && ((*off as usize) < key || *off as usize >= lost + CHUNK)),
        Out::Discontinuity => false,
    });
    let before = recs.iter().filter(|(o, _)| (*o as usize) < key).count();
    expected.insert(before, Out::Discontinuity);
    assert_eq!(out, expected);
    let m = r.metrics();
    assert_eq!((m.discontinuities, m.rewinds), (1, 0));
}

#[test]
fn a_corrupt_chunked_frame_drops_the_records_it_touches() {
    let (mut s, recs) = units_stream(70);
    let key = recs
        .iter()
        .filter(|(_, r)| r.len() > 10_000)
        .nth(1)
        .unwrap()
        .0 as usize;
    let bad = (key / CHUNK + 1) * CHUNK;
    // Inside the keyframe a header names a boundary: it disagrees.
    s[bad..bad + 2].copy_from_slice(&100u16.to_be_bytes());
    let script = Script::default().reads(&s, 0, s.len(), 4096);
    let mut r = ChunkedReader::new(script, CHUNK, 64 * 1024).unwrap();
    let (out, _) = collect_chunked(&mut r, 32 * 1024);
    let before = recs.iter().filter(|(o, _)| (*o as usize) < key).count();
    assert_eq!(out[..before], records_from(&recs, 0)[..before]);
    assert_eq!(out[before], Out::Discontinuity);
    let m = r.metrics();
    assert_eq!((m.corrupt_frames, m.losses), (1, 1));
}

#[test]
fn a_replaced_reader_joins_the_new_stream_again() {
    let (a, old) = units_stream(40);
    let (b, new) = units_stream(70);
    let join = b.len() / 2 / CHUNK * CHUNK;
    let (_, k) = gop_of_first(&new, join as u64);
    let resume = k - k % CHUNK as u64;
    let mut r = ChunkedReader::new(
        Script::default().reads(&a, 0, a.len(), 4096),
        CHUNK,
        64 * 1024,
    )
    .unwrap();
    let (out, _) = collect_chunked(&mut r, 32 * 1024);
    assert_eq!(out, records_from(&old, 0));
    r.replace(Script::default().reads(&b, join, b.len(), 4096).rewound(
        Some(resume),
        Script::default().reads(&b, resume as usize, b.len(), 4096),
    ));
    let (out, _) = collect_chunked(&mut r, 32 * 1024);
    let mut expected = vec![Out::Discontinuity];
    expected.extend(records_from(&new, k));
    assert_eq!(out, expected);
    assert_eq!(r.metrics().rewinds, 1);
    // A replaced reader with a spare races its join again.
    r.replace(Script::default().reads(&b, join, b.len(), 4096));
    r.set_spare(Script::default().rewound(
        Some(resume),
        Script::default().reads(&b, resume as usize, b.len(), 4096),
    ));
    let (out, _) = collect_chunked(&mut r, 32 * 1024);
    assert_eq!(out, expected);
    assert_eq!(r.metrics().rewinds, 2);
}

#[test]
fn stream_formats_follow_the_listings_mime_type() {
    assert_eq!(
        StreamFormat::from_mime(r#"video/x.strims-units;codecs="avc1.64001f,mp4a.40.2""#),
        StreamFormat::Units
    );
    assert_eq!(
        StreamFormat::from_mime("Video/X.Strims-Units"),
        StreamFormat::Units
    );
    for packed in ["", r#"video/mp4;codecs="avc1.64001f""#] {
        assert_eq!(
            StreamFormat::from_mime(packed),
            StreamFormat::Packed,
            "{packed}"
        );
    }
}

/// A clock that stands at `now`, whose sleeps never wake.
#[derive(Clone)]
struct Still {
    now: Time,
}

impl Timer for Still {
    type Sleep = futures_util::future::Either<std::future::Ready<()>, std::future::Pending<()>>;

    fn now(&self) -> Time {
        self.now
    }

    fn sleep_until(&self, deadline: Time) -> Self::Sleep {
        if deadline <= self.now {
            futures_util::future::Either::Left(std::future::ready(()))
        } else {
            futures_util::future::Either::Right(std::future::pending())
        }
    }
}

#[test]
fn a_rewind_answers_until_its_deadline() {
    let t = Still {
        now: Time::from_unix_millis(10_000),
    };
    let later = t.now + Duration::from_secs(2);
    let ready = within(&t, later, std::future::ready(Some(4096))).now_or_never();
    assert_eq!(ready, Some(Some(4096)));
    let stuck = within(&t, later, std::future::pending()).now_or_never();
    assert_eq!(stuck, None);
    let past = within(&t, t.now, std::future::pending()).now_or_never();
    assert_eq!(past, Some(None));
    // A host reader's swarm reader doesn't rewind; its grace waits.
    let mut h = HostReader {
        reader: Script::default(),
        timer: t.clone(),
        timeout: Duration::from_secs(2),
        grace: Duration::from_millis(700),
    };
    assert_eq!(Rewind::rewind(&mut h, 0).now_or_never(), Some(None));
    assert_eq!(h.grace().now_or_never(), None);
    h.grace = Duration::ZERO;
    assert_eq!(h.grace().now_or_never(), Some(()));
}
