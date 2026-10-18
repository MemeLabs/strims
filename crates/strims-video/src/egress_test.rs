// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::VecDeque;

use futures_util::FutureExt;
use strims_chunkstream::packed::PackedEncoder;

use super::*;

/// A scripted read: the offset and bytes, or the failure.
type Read = Result<(u64, Vec<u8>), ReadFailure<String>>;

/// A swarm reader that plays a script of reads, then reports the swarm
/// closed.
#[derive(Default)]
struct Script {
    reads: VecDeque<Read>,
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
}

impl SwarmRead for Script {
    type Error = String;

    async fn read_at(&mut self, out: &mut [u8]) -> Result<(u64, usize), ReadFailure<String>> {
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
