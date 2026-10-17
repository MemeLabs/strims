// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;
use crate::frame::HEADER_LEN;

const SIZE: usize = 16;
const DATA: usize = SIZE - HEADER_LEN;

fn encode(payloads: &[&[u8]]) -> Vec<u8> {
    let e = RecordEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    for p in payloads {
        let start = out.len();
        out.resize(start + e.encoded_len(p.len()).unwrap(), 0);
        assert_eq!(e.encode(p, &mut out[start..]), Ok(out.len() - start));
    }
    out
}

/// Frames holding `record` as raw record bytes.
fn frames(record: &[u8]) -> Vec<u8> {
    let e = FrameEncoder::new(SIZE).unwrap();
    let mut out = vec![0; e.encoded_len(record.len()).unwrap()];
    e.encode(&[record], &mut out).unwrap();
    out
}

fn decode_all(d: &mut RecordDecoder, mut input: &[u8]) -> Vec<Vec<u8>> {
    let mut records = Vec::new();
    loop {
        let r = d.decode(input);
        input = &input[r.consumed..];
        if !r.record {
            assert!(input.is_empty(), "input left without a record");
            return records;
        }
        records.push(d.record().to_vec());
    }
}

fn read_from(stream: &[u8], offset: usize, max: usize) -> (Vec<Vec<u8>>, DecoderMetrics) {
    let mut d = RecordDecoder::new(SIZE, max).unwrap();
    d.reset(offset as u64);
    let records = decode_all(&mut d, &stream[offset..]);
    (records, *d.metrics())
}

fn seq(start: u8, n: usize) -> Vec<u8> {
    (0..n).map(|i| start.wrapping_add(i as u8)).collect()
}

#[test]
fn a_record_is_its_length_varint_then_its_payload() {
    let mut want = vec![0x80, 0, 0, 8, 3, 1, 2, 3];
    want.resize(SIZE, 0);
    assert_eq!(encode(&[&[1, 2, 3]]), want);

    let e = RecordEncoder::new(SIZE).unwrap();
    assert_eq!(e.encoded_len(0), Some(SIZE));
    assert_eq!(e.encoded_len(DATA - 1), Some(2 * SIZE));
    assert_eq!(e.encoded_len(200), Some((202 / DATA + 1) * SIZE));
    assert_eq!(e.encoded_len(usize::MAX), None);
}

#[test]
fn records_round_trip() {
    let payloads = [seq(1, 3), Vec::new(), seq(9, 11), seq(30, 200), seq(5, 1)];
    let refs: Vec<&[u8]> = payloads.iter().map(Vec::as_slice).collect();
    let (records, metrics) = read_from(&encode(&refs), 0, 200);
    assert_eq!(records, payloads);
    assert_eq!(
        metrics,
        DecoderMetrics {
            records: 5,
            ..DecoderMetrics::default()
        }
    );
}

#[test]
fn an_input_split_anywhere_decodes_the_same() {
    let payloads = [seq(1, 30), seq(50, 11), Vec::new(), seq(90, 5)];
    let refs: Vec<&[u8]> = payloads.iter().map(Vec::as_slice).collect();
    let stream = encode(&refs);
    for piece in 1..=stream.len() {
        let mut d = RecordDecoder::new(SIZE, 64).unwrap();
        let mut records = Vec::new();
        for chunk in stream.chunks(piece) {
            records.extend(decode_all(&mut d, chunk));
        }
        assert_eq!(records, payloads, "piece {piece}");
    }
}

#[test]
fn decode_stops_right_after_a_record_end() {
    let stream = encode(&[&[1, 2, 3], &[4]]);
    let mut d = RecordDecoder::new(SIZE, 64).unwrap();
    let r = d.decode(&stream);
    assert_eq!(
        r,
        Decoded {
            consumed: HEADER_LEN + 4,
            record: true
        }
    );
    assert_eq!(d.record(), [1, 2, 3]);
    let r = d.decode(&stream[r.consumed..]);
    assert_eq!(
        r,
        Decoded {
            consumed: 2 * SIZE - (HEADER_LEN + 4) - (SIZE - HEADER_LEN - 2),
            record: true
        }
    );
    assert_eq!(d.record(), [4]);
}

#[test]
fn the_next_decode_releases_the_record() {
    let stream = encode(&[&[1, 2, 3]]);
    let mut d = RecordDecoder::new(SIZE, 64).unwrap();
    assert!(d.decode(&stream).record);
    assert_eq!(d.record(), [1, 2, 3]);
    assert!(!d.decode(&[]).record);
    assert_eq!(d.record(), []);
}

#[test]
fn a_reader_started_where_a_record_starts_yields_it() {
    let stream = encode(&[&seq(1, 30), &seq(50, 3), &seq(60, 20)]);
    let (records, metrics) = read_from(&stream, 3 * SIZE, 64);
    assert_eq!(records, [seq(50, 3), seq(60, 20)]);
    assert_eq!(metrics.records, 2);
}

#[test]
fn a_reader_started_mid_frame_drops_the_partial_record() {
    let stream = encode(&[&seq(1, 30), &seq(50, 3)]);
    let (records, metrics) = read_from(&stream, 8, 64);
    assert_eq!(records, [seq(50, 3)]);
    assert_eq!(metrics.partial, 1);
    assert_eq!(metrics.length_mismatch, 0);
}

#[test]
fn a_reader_started_inside_a_last_frame_drops_the_next_record_too() {
    let stream = encode(&[&seq(1, 5), &seq(20, 5), &seq(40, 5)]);
    let (records, metrics) = read_from(&stream, 8, 64);
    assert_eq!(records, [seq(40, 5)]);
    assert_eq!(metrics.partial, 1);
}

#[test]
fn a_reader_started_right_after_a_first_header_yields_a_record_spanning_frames() {
    let stream = encode(&[&seq(1, 30), &seq(50, 3)]);
    let (records, metrics) = read_from(&stream, HEADER_LEN, 64);
    assert_eq!(records, [seq(1, 30), seq(50, 3)]);
    assert_eq!(metrics.partial, 0);
}

#[test]
fn a_reader_started_inside_a_header_skips_to_the_next_frame() {
    let stream = encode(&[&seq(1, 5), &seq(50, 3)]);
    let (records, metrics) = read_from(&stream, 2, 64);
    assert_eq!(records, [seq(50, 3)]);
    assert_eq!(
        metrics,
        DecoderMetrics {
            records: 1,
            ..DecoderMetrics::default()
        }
    );
}

#[test]
fn a_reader_started_at_a_continuation_frame_drops_the_record_tail() {
    let stream = encode(&[&[0xff; 30], &seq(50, 3)]);
    let (records, metrics) = read_from(&stream, SIZE, 64);
    assert_eq!(records, [seq(50, 3)]);
    assert_eq!(metrics.length_mismatch, 1);
}

#[test]
fn a_reader_started_at_an_empty_end_frame_yields_the_next_record() {
    // An 11 byte payload makes a 12 byte record, which fills its frame.
    let stream = encode(&[&seq(1, DATA - 1), &seq(50, 3)]);
    assert_eq!(&stream[SIZE..SIZE + HEADER_LEN], &[0x80, 0, 0, 4]);
    let (records, metrics) = read_from(&stream, SIZE, 64);
    assert_eq!(records, [seq(50, 3)]);
    assert_eq!(metrics.length_mismatch, 1);
}

#[test]
fn a_length_that_disagrees_with_the_record_drops_it() {
    let stream = [
        frames(&[5, 1, 2]),
        frames(&[1, 1, 2]),
        frames(&[]),
        frames(&[0x80]),
        encode(&[&[7]]),
    ]
    .concat();
    let (records, metrics) = read_from(&stream, 0, 64);
    assert_eq!(records, [[7]]);
    assert_eq!(metrics.length_mismatch, 4);
}

#[test]
fn a_record_of_max_length_is_accepted() {
    let (records, _) = read_from(&encode(&[&seq(1, 20)]), 0, 20);
    assert_eq!(records, [seq(1, 20)]);
}

#[test]
fn a_record_longer_than_max_is_dropped_at_its_end() {
    let stream = encode(&[&seq(1, 21), &seq(50, 20)]);
    let (records, metrics) = read_from(&stream, 0, 20);
    assert_eq!(records, [seq(50, 20)]);
    assert_eq!(metrics.oversized, 1);
}

#[test]
fn buffering_stays_within_max_record() {
    let max = 100;
    let payload = vec![0xab; 10_000];
    let stream = encode(&[&payload, &[1]]);
    let mut d = RecordDecoder::new(SIZE, max).unwrap();
    let mut records = Vec::new();
    for chunk in stream.chunks(7) {
        records.extend(decode_all(&mut d, chunk));
        assert!(d.buf.capacity() <= max + MAX_UVARINT_LEN);
    }
    assert_eq!(records, [[1]]);
    assert_eq!(d.metrics().oversized, 1);
}

#[test]
fn a_corrupt_frame_drops_the_record_in_progress() {
    let mut stream = encode(&[&[0xff; 30], &seq(50, 3)]);
    stream[SIZE..SIZE + HEADER_LEN].copy_from_slice(&0x8000_0020u32.to_be_bytes());
    let (records, metrics) = read_from(&stream, 0, 64);
    assert_eq!(records, [seq(50, 3)]);
    assert_eq!(metrics.corrupt, 1);
    // The record's last frame then reads as a record of its own.
    assert_eq!(metrics.length_mismatch, 1);
}

#[test]
fn reset_drops_the_record_in_progress() {
    let stream = encode(&[&seq(1, 30), &seq(50, 3)]);
    let mut d = RecordDecoder::new(SIZE, 64).unwrap();
    assert_eq!(
        decode_all(&mut d, &stream[..SIZE + 5]),
        Vec::<Vec<u8>>::new()
    );
    d.reset(3 * SIZE as u64);
    assert_eq!(decode_all(&mut d, &stream[3 * SIZE..]), [seq(50, 3)]);
    assert_eq!(
        *d.metrics(),
        DecoderMetrics {
            records: 1,
            ..DecoderMetrics::default()
        }
    );
}
