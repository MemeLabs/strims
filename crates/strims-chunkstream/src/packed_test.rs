// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;
use crate::MAX_FRAME_SIZE;

const SIZE: usize = 16;

fn seq(start: u8, n: usize) -> Vec<u8> {
    (0..n).map(|i| start.wrapping_add(i as u8)).collect()
}

fn header(stream: &[u8], frame: usize) -> u32 {
    let at = frame * SIZE;
    u32::from_be_bytes(stream[at..at + HEADER_LEN].try_into().unwrap())
}

fn headers(stream: &[u8]) -> Vec<u32> {
    (0..stream.len().div_ceil(SIZE))
        .map(|f| header(stream, f))
        .collect()
}

fn frame(header: u32, data: &[u8]) -> Vec<u8> {
    let mut f = header.to_be_bytes().to_vec();
    f.extend_from_slice(data);
    f.resize(SIZE, 0);
    f
}

/// Writes each record and ends it; every end must be accepted.
fn pack(records: &[&[u8]]) -> Vec<u8> {
    let mut e = PackedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    for r in records {
        e.write(r, &mut out);
        e.end_record(&mut out).unwrap();
    }
    out
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Data(Vec<u8>),
    End,
    Corrupt,
}

use Event::{Corrupt, End};

fn data(b: &[u8]) -> Event {
    Event::Data(b.to_vec())
}

/// The fragments of `input` fed in `piece` byte pieces, adjacent data
/// merged.
fn decode(d: &mut PackedDecoder, input: &[u8], piece: usize) -> Vec<Event> {
    let mut events = Vec::new();
    for mut chunk in input.chunks(piece) {
        loop {
            let (n, frag) = d.decode(chunk);
            chunk = &chunk[n..];
            match frag {
                None => {
                    assert!(chunk.is_empty(), "input left without a fragment");
                    break;
                }
                Some(Frag::Data(b)) => match events.last_mut() {
                    Some(Event::Data(v)) => v.extend_from_slice(b),
                    _ => events.push(data(b)),
                },
                Some(Frag::End) => events.push(End),
                Some(Frag::Corrupt) => events.push(Corrupt),
            }
        }
    }
    events
}

fn decode_from(stream: &[u8], offset: usize) -> (Start, Vec<Event>) {
    let mut d = PackedDecoder::new(SIZE).unwrap();
    let start = d.reset(offset as u64);
    (
        start,
        decode(&mut d, &stream[offset..], stream.len().max(1)),
    )
}

#[test]
fn frame_sizes_leave_room_for_data() {
    for size in [0, 1, HEADER_LEN, MAX_FRAME_SIZE + 1, usize::MAX] {
        assert_eq!(PackedEncoder::new(size).err(), Some(SizeError(size)));
        assert!(PackedDecoder::new(size).is_err());
    }
    for size in [HEADER_LEN + 1, 1024, DEFAULT_SIZE] {
        assert_eq!(PackedEncoder::new(size).unwrap().size(), size);
        assert_eq!(PackedDecoder::new(size).unwrap().size(), size);
    }
    assert_eq!(DEFAULT_SIZE, 65531);
}

#[test]
fn records_follow_each_other_in_a_frame() {
    let stream = pack(&[&[1, 2, 3], &seq(10, 30)]);
    let want = [
        frame(0x8000_0007, &[[1, 2, 3].as_slice(), &seq(10, 9)].concat()),
        frame(0, &seq(19, 12)),
        [&0x8000_000d_u32.to_be_bytes()[..], &seq(31, 9)].concat(),
    ]
    .concat();
    assert_eq!(stream, want);
    let (_, events) = decode_from(&stream, 0);
    assert_eq!(events, [data(&[1, 2, 3]), End, data(&seq(10, 30)), End]);
}

#[test]
fn a_record_ending_at_a_frame_end_has_its_end_in_the_next_header() {
    let mut e = PackedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    e.write(&seq(1, 12), &mut out);
    assert_eq!(out, frame(0, &seq(1, 12)));
    e.end_record(&mut out).unwrap();
    assert_eq!(out.len(), SIZE + HEADER_LEN);
    assert_eq!(header(&out, 1), 0x8000_0004);

    e.write(&seq(20, 12), &mut out);
    e.end_record(&mut out).unwrap();
    assert_eq!(headers(&out), [0, 0x8000_0004, 0x8000_0004]);
    let (_, events) = decode_from(&out, 0);
    assert_eq!(events, [data(&seq(1, 12)), End, data(&seq(20, 12)), End]);
}

#[test]
fn an_empty_first_record_ends_right_after_the_header() {
    assert_eq!(pack(&[&[]]), [0x80, 0, 0, 4]);
    let stream = pack(&[&[], &seq(1, 13)]);
    assert_eq!(headers(&stream), [0x8000_0004, 0x8000_0005]);
    let (_, events) = decode_from(&stream, 0);
    assert_eq!(events, [End, data(&seq(1, 13)), End]);
}

#[test]
fn a_record_spanning_frames_continues_under_zero_headers() {
    let stream = pack(&[&seq(1, 30), &seq(40, 40)]);
    assert_eq!(headers(&stream), [0, 0, 0x8000_000a, 0, 0, 0x8000_000e]);
    assert_eq!(&stream[36..42], &seq(25, 6)[..]);
    assert_eq!(&stream[42..48], &seq(40, 6)[..]);
    let (_, events) = decode_from(&stream, 0);
    assert_eq!(events, [data(&seq(1, 30)), End, data(&seq(40, 40)), End]);
}

#[test]
fn bytes_after_an_end_are_held_until_the_frame_fills() {
    let mut e = PackedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    e.write(&[1, 2], &mut out);
    assert!(out.is_empty());
    e.end_record(&mut out).unwrap();
    assert_eq!(out, [0x80, 0, 0, 6, 1, 2]);
    e.write(&seq(3, 9), &mut out);
    assert_eq!(out.len(), 6);
    e.write(&[], &mut out);
    assert_eq!(out.len(), 6);
    e.write(&seq(12, 2), &mut out);
    assert_eq!(out, frame(0x8000_0006, &seq(1, 12)));
}

#[test]
fn a_second_end_in_a_frame_is_refused_and_the_record_continues() {
    let mut e = PackedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    e.write(&[1, 2], &mut out);
    e.end_record(&mut out).unwrap();
    e.write(&[3, 4, 5], &mut out);
    assert_eq!(e.end_record(&mut out), Err(FrameHasEnd));
    assert_eq!(out.len(), 6);
    assert_eq!(e.offset(), 9);

    e.write(&seq(6, 7), &mut out);
    assert_eq!(out.len(), SIZE);
    e.end_record(&mut out).unwrap();
    let (_, events) = decode_from(&out, 0);
    assert_eq!(events, [data(&[1, 2]), End, data(&seq(3, 10)), End]);
}

#[test]
fn an_end_is_blocked_until_the_frame_fills() {
    let mut e = PackedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    for len in [0, 1, 11, 12, 100] {
        assert!(!e.end_blocked_after(len));
        assert_eq!(e.unblocking_len(len), len);
    }
    e.write(&[1, 2], &mut out);
    e.end_record(&mut out).unwrap();
    for (len, blocked, unblocking) in [
        (0, true, 10),
        (9, true, 10),
        (10, false, 10),
        (11, false, 11),
        (100, false, 100),
    ] {
        assert_eq!(e.end_blocked_after(len), blocked, "len {len}");
        assert_eq!(e.unblocking_len(len), unblocking, "len {len}");
    }
    e.write(&seq(3, 9), &mut out);
    assert!(e.end_blocked_after(0));
    assert_eq!(e.unblocking_len(0), 1);
    assert_eq!(e.end_record(&mut out), Err(FrameHasEnd));
    e.write(&[12], &mut out);
    assert!(!e.end_blocked_after(0));
    e.end_record(&mut out).unwrap();

    // The end at 4 blocks the frame after a record that filled its frame.
    assert_eq!(header(&out, 1), 0x8000_0004);
    assert!(e.end_blocked_after(11));
    assert_eq!(e.unblocking_len(3), 12);
}

#[test]
fn an_unblocking_length_always_lets_the_record_end() {
    for first in 0..14 {
        for len in 0..30 {
            let mut e = PackedEncoder::new(SIZE).unwrap();
            let mut out = Vec::new();
            e.write(&seq(0, first), &mut out);
            e.end_record(&mut out).unwrap();
            let n = e.unblocking_len(len);
            assert!(n >= len);
            assert!(n == len || e.end_blocked_after(n - 1));
            e.write(&vec![7; n], &mut out);
            assert!(e.end_record(&mut out).is_ok(), "first {first} len {len}");
        }
    }
}

#[test]
fn abandoning_drops_a_record_begun_after_the_frame_end() {
    let mut e = PackedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    e.write(&[1, 2], &mut out);
    e.end_record(&mut out).unwrap();
    e.write(&[3, 4, 5], &mut out);
    assert_eq!(e.end_record(&mut out), Err(FrameHasEnd));
    assert!(e.abandon_record());
    assert_eq!(out.len(), 6);
    assert_eq!(e.offset(), 6);
    assert!(e.abandon_record(), "an empty record is dropped too");

    e.write(&seq(20, 15), &mut out);
    e.end_record(&mut out).unwrap();
    let (_, events) = decode_from(&out, 0);
    assert_eq!(events, [data(&[1, 2]), End, data(&seq(20, 15)), End]);
}

#[test]
fn abandoning_a_record_begun_in_an_earlier_frame_changes_nothing() {
    let mut e = PackedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    e.write(&seq(1, 15), &mut out);
    let offset = e.offset();
    assert!(!e.abandon_record());
    assert_eq!(e.offset(), offset);
    e.end_record(&mut out).unwrap();
    let (_, events) = decode_from(&out, 0);
    assert_eq!(events, [data(&seq(1, 15)), End]);
}

#[test]
fn abandoning_the_first_record_restarts_the_stream() {
    let mut e = PackedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    e.write(&seq(1, 5), &mut out);
    assert!(e.abandon_record());
    assert_eq!(e.offset(), HEADER_LEN as u64);
    e.write(&[9], &mut out);
    e.end_record(&mut out).unwrap();
    assert_eq!(out, [0x80, 0, 0, 5, 9]);
}

#[test]
fn the_offset_is_where_the_next_byte_lands() {
    let mut e = PackedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    assert_eq!(e.offset(), 4);
    e.write(&seq(0, 11), &mut out);
    assert_eq!(e.offset(), 15);
    e.write(&[0], &mut out);
    assert_eq!(e.offset(), 20, "a full frame moves past the next header");
    e.end_record(&mut out).unwrap();
    assert_eq!(e.offset(), 20);
    e.write(&seq(0, 40), &mut out);
    assert_eq!(e.offset(), 20 + 40 + 3 * HEADER_LEN as u64);
}

#[test]
fn fragments_reassemble_from_input_split_anywhere() {
    // Each record fills the frame its predecessor ended in.
    let records = [seq(1, 30), seq(50, 12), seq(90, 6), seq(1, 13)];
    let stream = pack(&records.iter().map(Vec::as_slice).collect::<Vec<_>>());
    let want: Vec<Event> = records.iter().flat_map(|r| [data(r), End]).collect();
    for piece in 1..=stream.len() {
        let mut d = PackedDecoder::new(SIZE).unwrap();
        assert_eq!(decode(&mut d, &stream, piece), want, "piece {piece}");
    }
}

#[test]
fn reset_reports_where_the_offset_falls() {
    let mut d = PackedDecoder::new(SIZE).unwrap();
    for (offset, start) in [
        (0, Start::Frame),
        (16, Start::Frame),
        (1, Start::Header),
        (3, Start::Header),
        (19, Start::Header),
        (4, Start::Data),
        (15, Start::Data),
        (u64::MAX, Start::Data),
    ] {
        assert_eq!(d.reset(offset), start, "offset {offset}");
    }
}

/// Frame 0: record a (3 bytes, end at 7), then b's first 9 bytes. Frame 1:
/// b's last 5 bytes (end at 9), then c's first 7. Frame 2: c's last byte
/// (end at 5).
fn three_records() -> Vec<u8> {
    let stream = pack(&[&[1, 2, 3], &seq(10, 14), &seq(30, 8)]);
    assert_eq!(headers(&stream), [0x8000_0007, 0x8000_0009, 0x8000_0005]);
    stream
}

#[test]
fn a_decoder_started_at_a_later_frame_start_reads_a_record_tail_first() {
    let stream = three_records();
    let (start, events) = decode_from(&stream, SIZE);
    assert_eq!(start, Start::Frame);
    assert_eq!(events, [data(&seq(19, 5)), End, data(&seq(30, 8)), End]);
}

#[test]
fn a_decoder_started_at_a_frame_start_after_a_filled_frame_reads_an_empty_tail() {
    let stream = pack(&[&seq(1, 12), &seq(20, 12)]);
    let (_, events) = decode_from(&stream, SIZE);
    assert_eq!(events, [End, data(&seq(20, 12)), End]);
}

#[test]
fn a_decoder_started_inside_a_header_skips_to_the_next_frame() {
    let stream = three_records();
    for offset in [SIZE + 1, SIZE + 3] {
        let (start, events) = decode_from(&stream, offset);
        assert_eq!(start, Start::Header);
        assert_eq!(events, [data(&[37]), End], "offset {offset}");
    }
}

#[test]
fn a_decoder_started_in_frame_data_reads_to_the_first_end_it_reads() {
    let stream = three_records();
    // The end at 7 is in the header it didn't read.
    let (start, events) = decode_from(&stream, 5);
    assert_eq!(start, Start::Data);
    let mut first = vec![2, 3];
    first.extend_from_slice(&seq(10, 14));
    assert_eq!(events, [data(&first), End, data(&seq(30, 8)), End]);

    let (_, events) = decode_from(&stream, SIZE + 10);
    assert_eq!(events, [data(&seq(31, 7)), End]);
}

#[test]
fn a_decoder_at_the_end_of_a_frame_or_stream_yields_nothing_yet() {
    let stream = three_records();
    let (_, events) = decode_from(&stream, SIZE - 1);
    assert_eq!(events, [data(&seq(18, 6)), End, data(&seq(30, 8)), End]);
    let (_, events) = decode_from(&stream, stream.len());
    assert_eq!(events, []);
}

#[test]
fn an_end_offset_outside_the_frame_marks_it_corrupt() {
    let stream = [
        frame(0, &seq(1, 12)),
        frame(0x8000_0003, &seq(1, 12)),
        frame(0x8000_0011, &seq(1, 12)),
        frame(0xffff_ffff, &seq(1, 12)),
        frame(0, &seq(40, 12)),
        frame(0x8000_0006, &seq(7, 12)),
    ]
    .concat();
    let (_, events) = decode_from(&stream, 0);
    assert_eq!(
        events,
        [
            data(&seq(1, 12)),
            Corrupt,
            Corrupt,
            Corrupt,
            data(&[seq(40, 12), seq(7, 2)].concat()),
            End,
            data(&seq(9, 10)),
        ]
    );
}

#[test]
fn an_end_at_the_frame_size_ends_the_record_with_its_frame() {
    let stream = [frame(0x8000_0010, &seq(1, 12)), vec![0x80, 0, 0, 5, 9, 10]].concat();
    let (_, events) = decode_from(&stream, 0);
    assert_eq!(
        events,
        [data(&seq(1, 12)), End, data(&[9]), End, data(&[10])]
    );
}

#[test]
fn a_continuation_header_needs_only_a_clear_flag() {
    let stream = [frame(0x0000_1234, &seq(1, 12)), frame(0x8000_0005, &[13])].concat();
    let (_, events) = decode_from(&stream, 0);
    assert_eq!(events, [data(&seq(1, 13)), End, data(&[0; 11])]);
}

#[test]
fn an_end_is_reported_before_more_input_arrives() {
    let stream = pack(&[&[1, 2]]);
    let mut d = PackedDecoder::new(SIZE).unwrap();
    assert_eq!(d.decode(&stream), (6, Some(Frag::Data(&[1, 2]))));
    assert_eq!(d.decode(&[]), (0, Some(Frag::End)));
    assert_eq!(d.decode(&[]), (0, None));
}
