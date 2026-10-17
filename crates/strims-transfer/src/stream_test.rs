// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Record writer and reader tests over in-memory swarm streams.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use prost::Message;
use strims_chunkstream::RecordEncoder;
use strims_proto::transfer::v1::TransferPeerCloseRequest as Msg;

use super::*;

const FRAME: usize = 32;

fn msg(i: u8, len: usize) -> Msg {
    Msg { id: vec![i; len] }
}

/// Messages of 1, 3, 1, 2 and 1 frames.
fn msgs(base: u8) -> Vec<Msg> {
    [10, 60, 20, 40, 5]
        .iter()
        .enumerate()
        .map(|(i, &len)| msg(base + i as u8, len))
        .collect()
}

/// The stream offset of each message's record.
fn starts(msgs: &[Msg]) -> Vec<usize> {
    let enc = RecordEncoder::new(FRAME).unwrap();
    let mut off = 0;
    msgs.iter()
        .map(|m| {
            let start = off;
            off += enc.encoded_len(m.encoded_len()).unwrap();
            start
        })
        .collect()
}

/// What a fake swarm writer has published.
#[derive(Default)]
struct Published {
    unit: usize,
    /// The bytes of the partial unit.
    staged: Vec<u8>,
    /// The stream's whole units.
    stream: Vec<u8>,
    /// The length of each write.
    writes: Vec<usize>,
    resets: usize,
    /// The next write takes this many bytes and fails.
    fail_after: Option<usize>,
}

/// Publishes whole units of what it is given, as a swarm writer does.
struct FakeWriter(Rc<RefCell<Published>>);

impl FakeWriter {
    fn new(unit: usize) -> (FakeWriter, Rc<RefCell<Published>>) {
        let p = Rc::new(RefCell::new(Published {
            unit,
            ..Published::default()
        }));
        (FakeWriter(p.clone()), p)
    }
}

impl SwarmWrite for FakeWriter {
    type Error = String;

    async fn write(&mut self, data: &[u8]) -> Result<(), String> {
        let mut p = self.0.borrow_mut();
        let fail = p.fail_after.take();
        let data = &data[..fail.unwrap_or(data.len()).min(data.len())];
        p.writes.push(data.len());
        p.staged.extend_from_slice(data);
        let whole = p.staged.len() / p.unit * p.unit;
        let units: Vec<u8> = p.staged.drain(..whole).collect();
        p.stream.extend(units);
        match fail {
            Some(_) => Err("write failed".to_owned()),
            None => Ok(()),
        }
    }

    async fn reset(&mut self) -> Result<(), String> {
        let mut p = self.0.borrow_mut();
        p.staged.clear();
        p.stream.clear();
        p.resets += 1;
        Ok(())
    }
}

/// The stream of `msgs` written through a record writer.
async fn stream_of(msgs: &[Msg]) -> Vec<u8> {
    let (w, published) = FakeWriter::new(FRAME);
    let mut w = RecordWriter::new(w, FRAME, 1024).unwrap();
    for m in msgs {
        w.write(m).await.unwrap();
    }
    published.borrow().stream.clone()
}

enum Step {
    /// Bytes from a stream offset.
    Data(u64, Vec<u8>),
    Discontinuity,
}

/// Plays steps back, in reads of at most `max_read` bytes, then closes.
struct FakeReader {
    steps: VecDeque<Step>,
    max_read: usize,
}

impl FakeReader {
    fn new(max_read: usize, steps: impl IntoIterator<Item = Step>) -> FakeReader {
        FakeReader {
            steps: steps.into_iter().collect(),
            max_read,
        }
    }
}

impl SwarmRead for FakeReader {
    type Error = String;

    async fn read_at(&mut self, out: &mut [u8]) -> Result<(u64, usize), ReadFailure<String>> {
        match self.steps.pop_front() {
            None => Err(ReadFailure::Closed),
            Some(Step::Discontinuity) => Err(ReadFailure::Discontinuity),
            Some(Step::Data(offset, data)) => {
                let n = data.len().min(out.len()).min(self.max_read);
                out[..n].copy_from_slice(&data[..n]);
                if n < data.len() {
                    let rest = Step::Data(offset + n as u64, data[n..].to_vec());
                    self.steps.push_front(rest);
                }
                Ok((offset, n))
            }
        }
    }
}

/// `stream[from..]` from its offset.
fn from(stream: &[u8], from: usize) -> Step {
    Step::Data(from as u64, stream[from..].to_vec())
}

/// `stream[from..to]` from its offset.
fn range(stream: &[u8], from: usize, to: usize) -> Step {
    Step::Data(from as u64, stream[from..to].to_vec())
}

/// The messages read until the stream closes; `None` for each
/// discontinuity.
async fn drain(r: &mut RecordReader<FakeReader>) -> Vec<Option<Msg>> {
    let mut out = Vec::new();
    loop {
        match r.next_message::<Msg>().await {
            Ok(m) => out.push(Some(m)),
            Err(ReadFailure::Discontinuity) => out.push(None),
            Err(ReadFailure::Closed) => return out,
            Err(e) => panic!("{e}"),
        }
    }
}

fn some(msgs: &[Msg]) -> Vec<Option<Msg>> {
    msgs.iter().cloned().map(Some).collect()
}

#[tokio::test]
async fn records_read_back_from_the_stream_start() {
    let m = msgs(1);
    let stream = stream_of(&m).await;
    for max_read in [1, 7, FRAME, 1 << 20] {
        let reader = FakeReader::new(max_read, [from(&stream, 0)]);
        let mut r = RecordReader::new(reader, FRAME, 1024).unwrap();
        assert_eq!(drain(&mut r).await, some(&m), "reads of {max_read}");
        assert_eq!(r.metrics().decoder.records, m.len() as u64);
        assert_eq!(r.metrics().discontinuities, 0);
    }
}

#[tokio::test]
async fn a_reader_started_mid_frame_drops_the_record_it_joined() {
    let m = msgs(1);
    let stream = stream_of(&m).await;
    let s = starts(&m);
    // inside the data of the second record's first frame
    let reader = FakeReader::new(5, [from(&stream, s[1] + 20)]);
    let mut r = RecordReader::new(reader, FRAME, 1024).unwrap();
    assert_eq!(drain(&mut r).await, some(&m[2..]));
    assert_eq!(r.metrics().decoder.partial, 1);
    assert_eq!(r.metrics().discontinuities, 0);
}

#[tokio::test]
async fn a_discontinuity_is_reported_once_and_reading_resyncs() {
    let (a, b) = (msgs(1), msgs(101));
    let (sa, sb) = (stream_of(&a).await, stream_of(&b).await);
    let (ra, rb) = (starts(&a), starts(&b));
    let reader = FakeReader::new(
        9,
        [
            // ends inside the third record
            range(&sa, 0, ra[2] + 10),
            Step::Discontinuity,
            from(&sb, rb[1] + 20),
        ],
    );
    let mut r = RecordReader::new(reader, FRAME, 1024).unwrap();
    let mut want = some(&a[..2]);
    want.push(None);
    want.extend(some(&b[2..]));
    assert_eq!(drain(&mut r).await, want);
    let metrics = r.metrics();
    assert_eq!(metrics.discontinuities, 1);
    assert_eq!(metrics.decoder.partial, 1);
}

#[tokio::test]
async fn a_gap_between_read_offsets_is_a_discontinuity() {
    let m = msgs(1);
    let stream = stream_of(&m).await;
    let s = starts(&m);
    let reader = FakeReader::new(64, [range(&stream, 0, s[1] + 10), from(&stream, s[3])]);
    let mut r = RecordReader::new(reader, FRAME, 1024).unwrap();
    let mut want = some(&m[..1]);
    want.push(None);
    want.extend(some(&m[3..]));
    assert_eq!(drain(&mut r).await, want);
    assert_eq!(r.metrics().discontinuities, 1);
}

#[tokio::test]
async fn an_oversized_record_is_skipped() {
    let m = msgs(1);
    let stream = stream_of(&m).await;
    // only the 40 and 60 byte ids exceed it
    let max = msg(0, 30).encoded_len();
    let reader = FakeReader::new(16, [from(&stream, 0)]);
    let mut r = RecordReader::new(reader, FRAME, max).unwrap();
    assert_eq!(
        drain(&mut r).await,
        some(&[m[0].clone(), m[2].clone(), m[4].clone()])
    );
    assert_eq!(r.metrics().decoder.oversized, 2);
}

#[tokio::test]
async fn an_oversized_message_is_not_written() {
    let (w, published) = FakeWriter::new(FRAME);
    let big = msg(0, 60);
    let mut w = RecordWriter::new(w, FRAME, big.encoded_len() - 1).unwrap();
    assert_eq!(
        w.write(&big).await,
        Err(WriteFailure::TooLarge {
            len: big.encoded_len()
        })
    );
    w.write(&msg(1, 10)).await.unwrap();
    let p = published.borrow();
    assert_eq!(p.writes, [FRAME]);
}

#[tokio::test]
async fn a_garbage_first_record_after_a_reset_is_skipped() {
    let (a, b) = (msgs(1), msgs(101));
    let sa = stream_of(&a).await;
    // a record whose payload declares 127 bytes it doesn't have
    let enc = RecordEncoder::new(FRAME).unwrap();
    let mut sb = vec![0; FRAME];
    assert_eq!(enc.encode(&[0x0a, 0x7f], &mut sb), Ok(FRAME));
    sb.extend(stream_of(&b).await);
    let reader = FakeReader::new(13, [from(&sa, 0), Step::Discontinuity, from(&sb, 0)]);
    let mut r = RecordReader::new(reader, FRAME, 1024).unwrap();
    let mut want = some(&a);
    want.push(None);
    want.extend(some(&b));
    assert_eq!(drain(&mut r).await, want);
    assert_eq!(r.metrics().malformed, 1);
}

/// Each record is written whole frames at a time, so a writer that
/// publishes whole units leaves none partial when the unit divides the
/// frame.
#[tokio::test]
async fn records_are_written_in_whole_units() {
    let m = msgs(1);
    for unit in [FRAME, FRAME / 2, FRAME / 4] {
        let (w, published) = FakeWriter::new(unit);
        let mut w = RecordWriter::new(w, FRAME, 1024).unwrap();
        let mut total = 0;
        for x in &m {
            w.write(x).await.unwrap();
            let p = published.borrow();
            let n = *p.writes.last().unwrap();
            assert_eq!(n % FRAME, 0);
            total += n;
            assert!(p.staged.is_empty(), "a partial unit of {unit}");
            assert_eq!(p.stream.len(), total);
        }
        let stream = published.borrow().stream.clone();
        let mut r =
            RecordReader::new(FakeReader::new(64, [from(&stream, 0)]), FRAME, 1024).unwrap();
        assert_eq!(drain(&mut r).await, some(&m));
    }
}

#[tokio::test]
async fn a_failed_write_starts_the_stream_over_before_the_next() {
    let m = msgs(1);
    let (w, published) = FakeWriter::new(FRAME);
    let mut w = RecordWriter::new(w, FRAME, 1024).unwrap();
    w.write(&m[0]).await.unwrap();
    published.borrow_mut().fail_after = Some(FRAME + 3);
    assert!(matches!(w.write(&m[1]).await, Err(WriteFailure::Failed(_))));
    assert_eq!(published.borrow().resets, 0);
    w.write(&m[2]).await.unwrap();
    w.write(&m[3]).await.unwrap();
    let stream = {
        let p = published.borrow();
        assert_eq!(p.resets, 1);
        p.stream.clone()
    };
    let mut r = RecordReader::new(FakeReader::new(64, [from(&stream, 0)]), FRAME, 1024).unwrap();
    assert_eq!(drain(&mut r).await, some(&m[2..4]));
}
