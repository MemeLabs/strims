// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use super::*;

const SIZE: usize = 16;

fn pattern(n: usize, seed: usize) -> Vec<u8> {
    (0..n)
        .map(|i| ((i * 131 + seed * 17 + 7) % 256) as u8)
        .collect()
}

fn header(stream: &[u8], size: usize, frame: usize) -> u16 {
    let at = frame * size;
    u16::from_be_bytes([stream[at], stream[at + 1]])
}

fn headers(stream: &[u8], size: usize) -> Vec<u16> {
    (0..stream.len().div_ceil(size))
        .map(|f| header(stream, size, f))
        .collect()
}

/// Appends one record to `out`, checking `encoded_len` and the record's
/// offset. Returns the offset.
fn put(e: &mut ChunkedEncoder, payload: &[u8], out: &mut Vec<u8>) -> u64 {
    let offset = e.next_record_offset();
    let before = e.offset();
    let n = e.encoded_len(payload.len()).unwrap();
    let start = out.len();
    out.resize(start + n, 0xee);
    assert_eq!(e.encode(&[payload], &mut out[start..]), Ok(n));
    assert_eq!(e.offset(), before + n as u64);
    offset
}

/// Ends the frame and continues `skip` frames later, appending the
/// skipped frames to `out` as zeros.
fn skip(e: &mut ChunkedEncoder, frames: usize, out: &mut Vec<u8>) {
    let start = out.len();
    out.resize(start + e.frame_padding(), 0xee);
    assert_eq!(e.end_frame(&mut out[start..]), Ok(out.len() - start));
    let next = e.offset() + (frames * e.frame_size()) as u64;
    assert_eq!(e.continue_at(next), Ok(()));
    out.resize(out.len() + frames * e.frame_size(), 0);
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ev {
    Record(u64, Vec<u8>),
    Lost,
}

fn rec(offset: u64, payload: &[u8]) -> Ev {
    Ev::Record(offset, payload.to_vec())
}

/// The records and losses of `input` from `offset`, fed in `piece` byte
/// pieces, through the piece decoder and the record decoder, which must
/// agree.
fn decode(size: usize, max: usize, offset: u64, input: &[u8], piece: usize) -> Vec<Ev> {
    let mut d = ChunkedDecoder::new(size, max).unwrap();
    d.reset(offset);
    let mut from_pieces = Vec::new();
    let mut current: Option<(u64, usize, Vec<u8>)> = None;
    for mut chunk in input.chunks(piece.max(1)) {
        loop {
            let (n, p) = d.decode(chunk);
            chunk = &chunk[n..];
            match p {
                None => {
                    assert!(chunk.is_empty());
                    break;
                }
                Some(Piece::Start { offset, len }) => {
                    assert!(current.is_none(), "a Start inside a record");
                    assert!((1..=max).contains(&len));
                    current = Some((offset, len, Vec::new()));
                }
                Some(Piece::Data(b)) => {
                    assert!(!b.is_empty());
                    current
                        .as_mut()
                        .expect("data outside a record")
                        .2
                        .extend_from_slice(b);
                }
                Some(Piece::End) => {
                    let (offset, len, payload) = current.take().expect("an End outside a record");
                    assert_eq!(payload.len(), len);
                    from_pieces.push(Ev::Record(offset, payload));
                }
                Some(Piece::Lost) => {
                    current = None;
                    from_pieces.push(Ev::Lost);
                }
            }
        }
    }

    let mut r = ChunkedRecordDecoder::new(size, max).unwrap();
    r.reset(offset);
    let mut from_records = Vec::new();
    for mut chunk in input.chunks(piece.max(1)) {
        loop {
            let (n, item) = r.decode(chunk);
            chunk = &chunk[n..];
            match item {
                None => break,
                Some(Item::Record { offset, payload }) => {
                    from_records.push(Ev::Record(offset, payload.to_vec()))
                }
                Some(Item::Lost) => from_records.push(Ev::Lost),
            }
            assert!(r.buf.capacity() <= max);
        }
    }
    assert_eq!(from_pieces, from_records, "piece and record decoders");
    assert_eq!(d.metrics(), r.metrics());
    from_pieces
}

/// [`decode`] in whole, 7 byte and 1 byte pieces, which must agree.
fn decode_all(size: usize, max: usize, offset: u64, input: &[u8]) -> Vec<Ev> {
    let whole = decode(size, max, offset, input, input.len());
    for piece in [7, 1] {
        assert_eq!(
            decode(size, max, offset, input, piece),
            whole,
            "piece {piece}"
        );
    }
    whole
}

fn metrics(size: usize, max: usize, offset: u64, input: &[u8]) -> ChunkedMetrics {
    let mut r = ChunkedRecordDecoder::new(size, max).unwrap();
    r.reset(offset);
    let mut rest = input;
    loop {
        let (n, item) = r.decode(rest);
        rest = &rest[n..];
        if item.is_none() {
            return *r.metrics();
        }
    }
}

/// A frame of `SIZE` bytes: `header`, `data`, then zeros.
fn frame(header: u16, data: &[u8]) -> Vec<u8> {
    let mut f = header.to_be_bytes().to_vec();
    f.extend_from_slice(data);
    assert!(f.len() <= SIZE);
    f.resize(SIZE, 0);
    f
}

#[test]
fn frame_sizes_outside_the_bounds_are_refused() {
    for size in [0, 1, 2, MAX_FRAME_SIZE + 1] {
        assert_eq!(ChunkedEncoder::new(size), Err(FrameSizeError(size)));
        assert!(ChunkedDecoder::new(size, 1).is_err());
        assert!(ChunkedRecordDecoder::new(size, 1).is_err());
    }
    for size in [MIN_FRAME_SIZE, 4096, MAX_FRAME_SIZE] {
        assert!(ChunkedEncoder::new(size).is_ok());
    }
}

#[test]
fn a_record_at_a_frame_start_begins_at_its_first_boundary() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    assert_eq!(e.next_record_offset(), 2);
    let mut out = Vec::new();
    assert_eq!(put(&mut e, b"abc", &mut out), 2);
    assert_eq!(out, [&[0, 2, 3][..], b"abc"].concat());
    assert_eq!(e.offset(), 6);
    assert_eq!(e.next_record_offset(), 6);
}

#[test]
fn a_frame_header_names_the_end_of_the_record_that_reaches_it() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    put(&mut e, b"abc", &mut out);
    // 1 + 20 bytes from offset 6: 10 in frame 0, 11 in frame 1.
    let p = pattern(20, 0);
    assert_eq!(put(&mut e, &p, &mut out), 6);
    assert_eq!(headers(&out, SIZE), [2, 13]);
    assert_eq!(out.len(), 16 + 13);
    assert_eq!(&out[16 + 2..], &p[9..]);
}

#[test]
fn a_record_ending_at_a_frame_end_moves_the_boundary_to_the_next_frame() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    // 1 + 27 bytes: 14 in frame 0 and 14 in frame 1.
    put(&mut e, &pattern(27, 0), &mut out);
    assert_eq!(out.len(), 32);
    assert_eq!(e.offset(), 32);
    assert_eq!(e.frame_padding(), 0);
    assert_eq!(e.next_record_offset(), 34);
    put(&mut e, b"x", &mut out);
    assert_eq!(headers(&out, SIZE), [2, NO_BOUNDARY, 2]);
}

#[test]
fn a_length_may_cross_a_frame_end() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    put(&mut e, &pattern(12, 0), &mut out);
    assert_eq!(e.offset(), 15);
    let p = pattern(200, 1);
    assert_eq!(put(&mut e, &p, &mut out), 15);
    // 200 is 0xc8 0x01: the first byte ends frame 0.
    assert_eq!(out[15], 0xc8);
    assert_eq!(&out[16..19], &[0, 1, 0x01]);
    let ev = decode_all(SIZE, 1000, 0, &out);
    assert_eq!(ev, [rec(2, &pattern(12, 0)), rec(15, &p)]);
    // The payload ends in frame 15, so frames 1 to 14 are inside it.
    let h = headers(&out, SIZE);
    assert!(h[1..h.len() - 1].iter().all(|&h| h == NO_BOUNDARY), "{h:?}");
}

#[test]
fn end_frame_pads_the_frame_and_is_a_no_op_at_a_frame_start() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = [0xee; SIZE];
    assert_eq!(e.frame_padding(), 0);
    assert_eq!(e.end_frame(&mut []), Ok(0));
    let mut stream = Vec::new();
    put(&mut e, b"abc", &mut stream);
    assert_eq!(e.frame_padding(), 10);
    assert_eq!(e.end_frame(&mut out[..9]), Err(BufferTooSmall));
    assert_eq!(e.offset(), 6);
    assert_eq!(e.end_frame(&mut out), Ok(10));
    assert_eq!(&out[..10], &[0; 10]);
    assert_eq!(out[10], 0xee);
    assert_eq!(e.offset(), 16);
    assert_eq!(e.end_frame(&mut out), Ok(0));
}

#[test]
fn continue_at_moves_only_from_a_frame_start_to_a_later_one() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    put(&mut e, b"abc", &mut out);
    assert_eq!(e.continue_at(64), Err(ContinueError::FrameInProgress));
    e.end_frame(&mut [0; SIZE]).unwrap();
    assert_eq!(e.continue_at(65), Err(ContinueError::Unaligned));
    assert_eq!(e.continue_at(0), Err(ContinueError::Behind));
    assert_eq!(e.offset(), 16);
    assert_eq!(e.continue_at(16), Ok(()));
    assert_eq!(e.continue_at(64), Ok(()));
    assert_eq!(e.offset(), 64);
    assert_eq!(e.next_record_offset(), 66);
}

#[test]
fn an_empty_record_or_a_short_buffer_writes_nothing() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = [0xee; 64];
    assert_eq!(e.encode(&[], &mut out), Err(EncodeError::Empty));
    assert_eq!(e.encode(&[b"", b""], &mut out), Err(EncodeError::Empty));
    let n = e.encoded_len(20).unwrap();
    let p = pattern(20, 0);
    assert_eq!(
        e.encode(&[&p], &mut out[..n - 1]),
        Err(EncodeError::BufferTooSmall)
    );
    assert_eq!(out, [0xee; 64]);
    assert_eq!(e, ChunkedEncoder::new(SIZE).unwrap());
    // Parts are one payload.
    assert_eq!(e.encode(&[&p[..5], b"", &p[5..]], &mut out), Ok(n));
    assert_eq!(decode_all(SIZE, 100, 0, &out[..n]), [rec(2, &p)]);
}

#[test]
fn a_record_past_the_end_of_the_stream_overflows() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let last = u64::MAX - u64::MAX % SIZE as u64;
    // The last frame isn't whole, so no record fits in it.
    e.continue_at(last - SIZE as u64).unwrap();
    let mut out = [0; 64];
    assert_eq!(e.encode(&[b"abc"], &mut out), Ok(6));
    e.end_frame(&mut out).unwrap();
    assert_eq!(e.offset(), last);
    e.continue_at(last).unwrap();
    assert_eq!(e.encode(&[b"abc"], &mut out), Err(EncodeError::Overflow));
    assert_eq!(e.offset(), last);
    assert_eq!(e.encoded_len(usize::MAX), None);
}

#[test]
fn skipped_frames_read_as_padding_or_as_a_gap() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    put(&mut e, &pattern(20, 0), &mut out);
    skip(&mut e, 2, &mut out);
    assert_eq!(put(&mut e, b"x", &mut out), 66);
    let want = [rec(2, &pattern(20, 0)), rec(66, b"x")];
    // Zeros: frames 2 and 3 are padding frames.
    assert_eq!(headers(&out, SIZE), [2, 9, 0, 0, 2]);
    assert_eq!(decode_all(SIZE, 100, 0, &out), want);
    assert_eq!(metrics(SIZE, 100, 0, &out).records, 2);
    // A gap: the reader resets at frame 4.
    let mut ev = decode_all(SIZE, 100, 0, &out[..32]);
    ev.extend(decode_all(SIZE, 100, 64, &out[64..]));
    assert_eq!(ev, want);
    // A join in the padding.
    assert_eq!(decode_all(SIZE, 100, 32, &out[32..]), [rec(66, b"x")]);
}

#[test]
fn a_header_may_name_a_boundary_that_holds_padding() {
    // A record ends at 9 in frame 1, and padding follows.
    let p = pattern(20, 0);
    let stream = [
        frame(2, &[&[20][..], &p[..13]].concat()),
        frame(9, &p[13..]),
    ]
    .concat();
    assert_eq!(decode_all(SIZE, 100, 0, &stream), [rec(2, &p)]);
    assert_eq!(decode_all(SIZE, 100, 16, &stream[16..]), []);
    assert_eq!(
        metrics(SIZE, 100, 16, &stream[16..]),
        ChunkedMetrics::default()
    );
}

#[test]
fn a_padding_frame_cuts_the_record_in_progress() {
    let p = pattern(20, 0);
    let stream = [
        frame(2, &[&[20][..], &p[..13]].concat()),
        frame(0, &[]),
        frame(2, b"\x01x"),
    ]
    .concat();
    assert_eq!(decode_all(SIZE, 100, 0, &stream), [Ev::Lost, rec(34, b"x")]);
    let m = metrics(SIZE, 100, 0, &stream);
    assert_eq!((m.records, m.lost, m.corrupt), (1, 1, 0));
}

#[test]
fn a_header_that_disagrees_costs_its_frame() {
    let p = pattern(20, 0);
    let head = frame(2, &[&[20][..], &p[..13]].concat());
    // Frame 1 holds the last 7 bytes, so its header must be 9, then a
    // record at 9.
    let tail = [&p[13..], &[1, b'y'][..]].concat();
    let good = [head.clone(), frame(9, &tail), frame(2, b"\x01x")].concat();
    assert_eq!(
        decode_all(SIZE, 100, 0, &good),
        [rec(2, &p), rec(25, b"y"), rec(34, b"x")]
    );
    for h in [1, 2, 8, 10, 15] {
        let bad = [head.clone(), frame(h, &tail), frame(2, b"\x01x")].concat();
        assert_eq!(
            decode_all(SIZE, 100, 0, &bad),
            [Ev::Lost, rec(34, b"x")],
            "{h}"
        );
        let m = metrics(SIZE, 100, 0, &bad);
        assert_eq!((m.records, m.lost, m.corrupt), (1, 1, 1), "{h}");
    }
    // At a boundary, a frame's header is 2 or 0.
    let bad = [frame(2, &[13]), frame(3, b"\x01x"), frame(2, b"\x01z")].concat();
    assert_eq!(
        decode_all(SIZE, 100, 0, &bad),
        [rec(2, &[0; 13]), Ev::Lost, rec(34, b"z")]
    );
    let stream = [frame(3, &[0, 1, b'x'])].concat();
    assert_eq!(decode_all(SIZE, 100, 0, &stream), [Ev::Lost]);
}

#[test]
fn an_invalid_header_costs_its_frame() {
    for h in [16, 17, 0x8000, u16::MAX] {
        let stream = [frame(h, b"\x01x"), frame(2, b"\x01y")].concat();
        assert_eq!(
            decode_all(SIZE, 100, 0, &stream),
            [Ev::Lost, rec(18, b"y")],
            "{h}"
        );
        assert_eq!(decode_all(SIZE, 100, 16, &stream[16..]), [rec(18, b"y")]);
        let joined = decode_all(
            SIZE,
            100,
            32,
            &[frame(h, b"\x01x"), frame(2, b"\x01y")].concat(),
        );
        assert_eq!(joined, [Ev::Lost, rec(50, b"y")], "{h}");
        assert_eq!(metrics(SIZE, 100, 0, &stream).corrupt, 1);
    }
}

#[test]
fn a_length_that_is_not_a_minimal_u64_varint_costs_its_frame() {
    let next = frame(2, b"\x01y");
    for length in [
        &[0x81, 0x00][..],
        &[0x80, 0x80, 0x00],
        &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02],
        &[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80],
    ] {
        let stream = [frame(2, &[length, b"x"].concat()), next.clone()].concat();
        assert_eq!(
            decode_all(SIZE, 100, 0, &stream),
            [Ev::Lost, rec(18, b"y")],
            "{length:x?}"
        );
        assert_eq!(metrics(SIZE, 100, 0, &stream).corrupt, 1);
    }
    // The largest u64 is a valid length, and oversized.
    let max = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01];
    let stream = [frame(2, &max), frame(NO_BOUNDARY, &[])].concat();
    assert_eq!(decode_all(SIZE, 100, 0, &stream), [Ev::Lost]);
    let m = metrics(SIZE, 100, 0, &stream);
    assert_eq!((m.oversized, m.corrupt), (1, 0));
}

#[test]
fn a_length_crossing_a_frame_end_is_checked_against_the_frames_header() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    let (a, b) = (pattern(12, 0), pattern(129, 1));
    let want = [
        rec(put(&mut e, &a, &mut out), &a),
        rec(put(&mut e, &b, &mut out), &b),
        rec(put(&mut e, b"y", &mut out), b"y"),
    ];
    // 129 is 0x81 0x01: its first byte ends frame 0, and frame 1 is all
    // inside the record.
    assert_eq!(&out[15..19], &[0x81, 0, 1, 0x01]);
    assert_eq!(decode_all(SIZE, 200, 0, &out), want);
    for h in [0, 2, 3, 5, 15] {
        let mut bad = out.clone();
        bad[16..18].copy_from_slice(&u16::to_be_bytes(h));
        assert_eq!(
            decode_all(SIZE, 200, 0, &bad),
            [want[0].clone(), Ev::Lost, want[2].clone()],
            "{h}"
        );
        let m = metrics(SIZE, 200, 0, &bad);
        assert_eq!((m.records, m.lost), (2, 0), "{h}");
        assert_eq!(m.corrupt, u64::from(h != 0), "{h}");
    }
}

#[test]
fn a_length_crossing_frames_has_every_header_checked() {
    // Frames of 3 bytes hold one data byte, so a 3 byte length spans 3
    // frames: 16384 is 0x80 0x80 0x01.
    let size = 3;
    let mut e = ChunkedEncoder::new(size).unwrap();
    let mut out = Vec::new();
    let p = pattern(16_384, 0);
    put(&mut e, &p, &mut out);
    put(&mut e, b"x", &mut out);
    assert_eq!(&headers(&out[..9], size), &[2, NO_BOUNDARY, NO_BOUNDARY]);
    let want = [rec(2, &p), rec(out.len() as u64 - 4, b"x")];
    assert_eq!(decode(size, 1 << 20, 0, &out, out.len()), want);
    assert_eq!(decode(size, 1 << 20, 0, &out, 1), want);
    let mut bad = out.clone();
    bad[4] = 2;
    assert_eq!(
        decode(size, 1 << 20, 0, &bad, bad.len()),
        [Ev::Lost, want[1].clone()]
    );
}

#[test]
fn an_oversized_record_is_skipped_in_sync() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    put(&mut e, b"abc", &mut out);
    put(&mut e, &pattern(40, 0), &mut out);
    let x = put(&mut e, b"x", &mut out);
    put(&mut e, &pattern(41, 1), &mut out);
    put(&mut e, &pattern(40, 2), &mut out);
    let y = put(&mut e, b"y", &mut out);
    let ev = decode_all(SIZE, 39, 0, &out);
    assert_eq!(
        ev,
        [
            rec(2, b"abc"),
            Ev::Lost,
            rec(x, b"x"),
            Ev::Lost,
            rec(y, b"y")
        ]
    );
    let m = metrics(SIZE, 39, 0, &out);
    assert_eq!((m.records, m.lost, m.oversized, m.corrupt), (3, 0, 3, 0));
    let ev = decode_all(SIZE, 40, 0, &out);
    assert_eq!(ev.iter().filter(|e| **e == Ev::Lost).count(), 1);
}

#[test]
fn a_join_after_a_padding_frame_reports_skipped_record_bytes() {
    let stream = [
        frame(0, &[]),
        frame(4, &[7, 7, 1, b'x']),
        frame(2, b"\x01y"),
    ]
    .concat();
    assert_eq!(
        decode_all(SIZE, 100, 0, &stream),
        [Ev::Lost, rec(20, b"x"), rec(34, b"y")]
    );
    let stream = [frame(0, &[]), frame(NO_BOUNDARY, &[]), frame(2, b"\x01y")].concat();
    assert_eq!(decode_all(SIZE, 100, 0, &stream), [Ev::Lost, rec(34, b"y")]);
    // A reset's join skips without a loss.
    let stream = [
        frame(4, &[7, 7, 1, b'x']),
        frame(NO_BOUNDARY, &[]),
        frame(2, b"\x01y"),
    ]
    .concat();
    assert_eq!(
        decode_all(SIZE, 100, 16, &stream),
        [rec(20, b"x"), Ev::Lost, rec(50, b"y")]
    );
    assert_eq!(decode_all(SIZE, 100, 32, &stream[16..]), [rec(50, b"y")]);
}

#[test]
fn a_loss_is_reported_once_between_records() {
    let stream = [
        frame(2, b"\x01a"),
        frame(17, &[]),
        frame(0, &[]),
        frame(NO_BOUNDARY, &[]),
        frame(18, &[]),
        frame(4, &[9, 9, 1, b'b']),
    ]
    .concat();
    assert_eq!(
        decode_all(SIZE, 100, 0, &stream),
        [rec(2, b"a"), Ev::Lost, rec(84, b"b")]
    );
    let m = metrics(SIZE, 100, 0, &stream);
    assert_eq!((m.records, m.lost, m.corrupt), (2, 0, 2));
}

#[test]
fn a_reset_inside_a_frame_joins_at_the_next_frame() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    let a = put(&mut e, b"abc", &mut out);
    let b = put(&mut e, &pattern(30, 0), &mut out);
    let c = put(&mut e, b"x", &mut out);
    assert_eq!((a, b, c), (2, 6, 41));
    for offset in 1..out.len() {
        let next = offset.next_multiple_of(SIZE) as u64;
        let want: Vec<Ev> = [rec(a, b"abc"), rec(b, &pattern(30, 0)), rec(c, b"x")]
            .into_iter()
            .filter(|r| matches!(r, Ev::Record(o, _) if *o >= next))
            .collect();
        assert_eq!(
            decode_all(SIZE, 100, offset as u64, &out[offset..]),
            want,
            "{offset}"
        );
    }
}

#[test]
fn a_reset_drops_the_record_in_progress() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    put(&mut e, &pattern(30, 0), &mut out);
    let mut r = ChunkedRecordDecoder::new(SIZE, 100).unwrap();
    assert_eq!(r.decode(&out[..20]), (20, None));
    r.reset(0);
    assert_eq!(r.metrics().lost, 1);
    let (n, item) = r.decode(&out);
    assert_eq!(n, out.len());
    assert_eq!(
        item,
        Some(Item::Record {
            offset: 2,
            payload: &pattern(30, 0)
        })
    );
}

#[test]
fn records_of_every_size_round_trip_from_every_start_in_a_frame() {
    let size = 7;
    let max = 300;
    for at in HEADER_LEN..size {
        for len in 1..=max {
            let mut e = ChunkedEncoder::new(size).unwrap();
            let mut out = Vec::new();
            let mut want = Vec::new();
            if at > HEADER_LEN {
                // A filler record from offset 2 that ends in frame 1 at
                // `at`: 5 data bytes in frame 0, `at - 2` in frame 1.
                let filler = pattern(size - HEADER_LEN + at - HEADER_LEN - 1, 9);
                want.push(rec(put(&mut e, &filler, &mut out), &filler));
                assert_eq!(e.next_record_offset() % size as u64, at as u64);
            }
            let p = pattern(len, len);
            want.push(rec(put(&mut e, &p, &mut out), &p));
            want.push(rec(put(&mut e, b"z", &mut out), b"z"));
            assert_eq!(
                decode(size, max, 0, &out, out.len()),
                want,
                "at {at} len {len}"
            );
            assert_eq!(decode(size, max, 0, &out, 5), want, "at {at} len {len}");
        }
    }
}

#[test]
fn records_around_frame_multiples_round_trip_in_4096_byte_frames() {
    let size = 4096;
    let mut e = ChunkedEncoder::new(size).unwrap();
    let mut out = Vec::new();
    let mut want = Vec::new();
    let mut lens = vec![1, 127, 128, 16_383, 16_384, 300_000];
    for k in 1..=3 {
        let n = k * (size - HEADER_LEN);
        lens.extend([n - 3, n - 2, n - 1, n, n + 1]);
    }
    for (i, &len) in lens.iter().enumerate() {
        let p = pattern(len, i);
        want.push(rec(put(&mut e, &p, &mut out), &p));
        if i % 4 == 3 {
            skip(&mut e, i % 3, &mut out);
        }
    }
    let ev = decode(size, 300_000, 0, &out, out.len());
    assert_eq!(ev, want);
    assert_eq!(decode(size, 300_000, 0, &out, 1000), want);
    for f in 1..out.len() / size {
        let start = (f * size) as u64;
        let tail: Vec<Ev> = want
            .iter()
            .filter(|r| matches!(r, Ev::Record(o, _) if *o >= start))
            .cloned()
            .collect();
        assert_eq!(
            decode(size, 300_000, start, &out[f * size..], 4096),
            tail,
            "frame {f}"
        );
    }
}

#[test]
fn a_whole_record_in_one_piece_is_borrowed_from_the_input() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    put(&mut e, b"abc", &mut out);
    put(&mut e, &pattern(30, 0), &mut out);
    let mut r = ChunkedRecordDecoder::new(SIZE, 30).unwrap();
    let range = out.as_ptr_range();
    let (n, item) = r.decode(&out);
    let Some(Item::Record { payload, .. }) = item else {
        panic!("{item:?}");
    };
    assert!(range.contains(&payload.as_ptr()), "the record is borrowed");
    let rest = &out[n..];
    let (_, item) = r.decode(rest);
    let Some(Item::Record { payload, .. }) = item else {
        panic!("{item:?}");
    };
    assert_eq!(payload, pattern(30, 0));
    assert!(
        !range.contains(&payload.as_ptr()),
        "the spanning record is copied"
    );
    assert!(r.buf.capacity() <= 30);
}

#[test]
fn the_record_buffer_stays_within_max_record() {
    let mut e = ChunkedEncoder::new(SIZE).unwrap();
    let mut out = Vec::new();
    for len in [10, 20, 33, 100, 99, 101] {
        put(&mut e, &pattern(len, len), &mut out);
    }
    let mut r = ChunkedRecordDecoder::new(SIZE, 100).unwrap();
    for piece in out.chunks(3) {
        let mut piece = piece;
        loop {
            let (n, item) = r.decode(piece);
            piece = &piece[n..];
            let done = item.is_none();
            assert!(r.buf.capacity() <= 100);
            if done {
                break;
            }
        }
    }
    let m = r.metrics();
    assert_eq!((m.records, m.oversized), (5, 1));
}
