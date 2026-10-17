// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;

const SIZE: usize = 16;

fn encode(size: usize, record: &[u8]) -> Vec<u8> {
    let e = FrameEncoder::new(size).unwrap();
    let mut out = vec![0xee; e.encoded_len(record.len()).unwrap()];
    assert_eq!(e.encode(&[record], &mut out), Ok(out.len()));
    out
}

fn headers(size: usize, frames: &[u8]) -> Vec<u32> {
    frames
        .chunks(size)
        .map(|f| u32::from_be_bytes(f[..HEADER_LEN].try_into().unwrap()))
        .collect()
}

fn frame(header: u32, data: &[u8]) -> Vec<u8> {
    let mut f = header.to_be_bytes().to_vec();
    f.extend_from_slice(data);
    f.resize(SIZE, 0);
    f
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Data(Vec<u8>),
    End,
    Corrupt,
}

/// The fragments of `input` fed in `piece` byte pieces, adjacent data
/// merged.
fn decode(d: &mut FrameDecoder, input: &[u8], piece: usize) -> Vec<Event> {
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
                    _ => events.push(Event::Data(b.to_vec())),
                },
                Some(Frag::End) => events.push(Event::End),
                Some(Frag::Corrupt) => events.push(Event::Corrupt),
            }
        }
    }
    events
}

fn decode_from(stream: &[u8], offset: usize) -> (Start, Vec<Event>) {
    let mut d = FrameDecoder::new(SIZE).unwrap();
    let start = d.reset(offset as u64);
    (start, decode(&mut d, &stream[offset..], stream.len()))
}

fn seq(start: u8, n: usize) -> Vec<u8> {
    (0..n).map(|i| start.wrapping_add(i as u8)).collect()
}

#[test]
fn frame_sizes_leave_room_for_data() {
    for size in [0, 1, HEADER_LEN, MAX_FRAME_SIZE + 1, usize::MAX] {
        assert_eq!(FrameEncoder::new(size), Err(SizeError(size)));
        assert!(FrameDecoder::new(size).is_err());
    }
    for size in [HEADER_LEN + 1, 1024, MAX_FRAME_SIZE] {
        assert_eq!(FrameEncoder::new(size).unwrap().size(), size);
        assert_eq!(FrameDecoder::new(size).unwrap().size(), size);
    }
}

#[test]
fn a_record_ending_inside_a_frame_carries_its_end_offset() {
    let mut want = vec![0x80, 0, 0, 7, 1, 2, 3];
    want.resize(SIZE, 0);
    assert_eq!(encode(SIZE, &[1, 2, 3]), want);
}

#[test]
fn continuation_frames_have_a_zero_header() {
    let frames = encode(SIZE, &seq(1, 30));
    assert_eq!(headers(SIZE, &frames), [0, 0, 0x8000_000a]);
    assert_eq!(&frames[4..16], &seq(1, 12)[..]);
    assert_eq!(&frames[20..32], &seq(13, 12)[..]);
    assert_eq!(&frames[36..42], &seq(25, 6)[..]);
}

#[test]
fn a_record_filling_its_frames_ends_in_an_empty_frame() {
    let frames = encode(SIZE, &seq(1, 24));
    assert_eq!(headers(SIZE, &frames), [0, 0, 0x8000_0004]);
    assert_eq!(&frames[36..], &[0; 12]);
}

#[test]
fn an_empty_record_takes_one_frame() {
    let mut want = vec![0x80, 0, 0, 4];
    want.resize(SIZE, 0);
    assert_eq!(encode(SIZE, &[]), want);
}

#[test]
fn encoding_zeroes_the_padding() {
    let frames = encode(SIZE, &[0xff; 13]);
    assert_eq!(&frames[16..21], &[0x80, 0, 0, 5, 0xff]);
    assert_eq!(&frames[21..], &[0; 11]);
}

#[test]
fn parts_encode_as_their_concatenation() {
    let e = FrameEncoder::new(SIZE).unwrap();
    let record = seq(0, 40);
    let mut out = vec![0; e.encoded_len(40).unwrap()];
    let parts: [&[u8]; 4] = [&record[..3], &[], &record[3..29], &record[29..]];
    assert_eq!(e.encode(&parts, &mut out), Ok(out.len()));
    assert_eq!(out, encode(SIZE, &record));
}

#[test]
fn a_short_output_buffer_is_refused() {
    let e = FrameEncoder::new(SIZE).unwrap();
    let mut out = vec![0xee; 2 * SIZE - 1];
    assert_eq!(e.encode(&[&[1; 20]], &mut out), Err(BufferTooSmall));
    let mut out = vec![0xee; 3 * SIZE];
    assert_eq!(e.encode(&[&[1; 20]], &mut out), Ok(2 * SIZE));
    assert_eq!(&out[2 * SIZE..], &[0xee; SIZE]);
}

#[test]
fn an_encoded_length_beyond_usize_is_none() {
    let e = FrameEncoder::new(HEADER_LEN + 1).unwrap();
    assert_eq!(e.encoded_len(usize::MAX), None);
    assert_eq!(e.encoded_len(0), Some(HEADER_LEN + 1));
}

#[test]
fn fragments_reassemble_from_input_split_anywhere() {
    let records = [seq(1, 30), Vec::new(), seq(50, 12), seq(90, 5)];
    let stream: Vec<u8> = records.iter().flat_map(|r| encode(SIZE, r)).collect();
    let mut want = Vec::new();
    for r in &records {
        if !r.is_empty() {
            want.push(Event::Data(r.clone()));
        }
        want.push(Event::End);
    }
    for piece in 1..=stream.len() {
        let mut d = FrameDecoder::new(SIZE).unwrap();
        assert_eq!(decode(&mut d, &stream, piece), want, "piece {piece}");
    }
}

#[test]
fn reset_reports_where_the_offset_falls() {
    let mut d = FrameDecoder::new(SIZE).unwrap();
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

#[test]
fn a_decoder_started_inside_a_header_skips_to_the_next_frame() {
    let stream = [encode(SIZE, &seq(1, 5)), encode(SIZE, &seq(9, 3))].concat();
    let (start, events) = decode_from(&stream, 2);
    assert_eq!(start, Start::Header);
    assert_eq!(events, [Event::Data(seq(9, 3)), Event::End]);
}

#[test]
fn a_decoder_started_in_frame_data_reads_to_the_next_record_end() {
    let stream = [encode(SIZE, &seq(1, 30)), encode(SIZE, &seq(90, 3))].concat();
    let (start, events) = decode_from(&stream, SIZE + 6);
    assert_eq!(start, Start::Data);
    assert_eq!(
        events,
        [
            Event::Data(seq(15, 16)),
            Event::End,
            Event::Data(seq(90, 3)),
            Event::End
        ]
    );
}

#[test]
fn a_decoder_started_in_a_last_frame_runs_into_the_next_record() {
    let stream = [encode(SIZE, &seq(1, 5)), encode(SIZE, &seq(90, 3))].concat();
    let (_, events) = decode_from(&stream, 6);
    let mut first = seq(3, 3);
    first.extend_from_slice(&[0; 7]);
    first.extend_from_slice(&seq(90, 3));
    assert_eq!(events, [Event::Data(first), Event::End]);
}

#[test]
fn padding_after_a_record_end_is_skipped() {
    let mut stream = encode(SIZE, &seq(1, 2));
    stream[10..].fill(0xff);
    let mut d = FrameDecoder::new(SIZE).unwrap();
    assert_eq!(
        decode(&mut d, &stream, 1),
        [Event::Data(seq(1, 2)), Event::End]
    );
}

#[test]
fn an_end_offset_outside_the_frame_marks_it_corrupt() {
    let stream = [
        frame(0, &seq(1, 12)),
        frame(0x8000_0003, &seq(1, 12)),
        frame(0x8000_0011, &seq(1, 12)),
        frame(0xffff_ffff, &seq(1, 12)),
        frame(0x8000_0006, &seq(7, 2)),
    ]
    .concat();
    let (_, events) = decode_from(&stream, 0);
    assert_eq!(
        events,
        [
            Event::Data(seq(1, 12)),
            Event::Corrupt,
            Event::Corrupt,
            Event::Corrupt,
            Event::Data(seq(7, 2)),
            Event::End,
        ]
    );
}

#[test]
fn an_end_at_the_frame_size_ends_the_record_with_its_frame() {
    let stream = [frame(0x8000_0010, &seq(1, 12)), frame(0x8000_0005, &[9])].concat();
    let (_, events) = decode_from(&stream, 0);
    assert_eq!(
        events,
        [
            Event::Data(seq(1, 12)),
            Event::End,
            Event::Data(vec![9]),
            Event::End
        ]
    );
}

#[test]
fn a_continuation_header_needs_only_a_clear_flag() {
    let stream = [frame(0x0000_1234, &seq(1, 12)), frame(0x8000_0005, &[13])].concat();
    let (_, events) = decode_from(&stream, 0);
    assert_eq!(events, [Event::Data(seq(1, 13)), Event::End]);
}
