// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;

const LIMITS: Limits = Limits {
    max_media: 4 << 20,
    max_command: 64 << 10,
    max_partial: 8 << 20,
    max_streams: 64,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Item {
    Msg {
        timestamp: u32,
        type_id: u8,
        stream_id: u32,
        body: Vec<u8>,
    },
    Ack(u32),
}

fn msg(timestamp: u32, type_id: u8, stream_id: u32, body: &[u8]) -> Item {
    Item::Msg {
        timestamp,
        type_id,
        stream_id,
        body: body.to_vec(),
    }
}

/// Reads `input` cut at `cuts`, acting on control messages; returns the
/// messages and acknowledgements in order, and the ping replies.
fn read_cut(
    reader: &mut ChunkReader,
    input: &[u8],
    cuts: &[usize],
) -> Result<(Vec<Item>, Vec<u8>), RtmpError> {
    let mut items = Vec::new();
    let mut writer = ChunkWriter::new(usize::MAX, false);
    let mut start = 0;
    for end in cuts.iter().copied().chain([input.len()]) {
        let mut piece = &input[start..end];
        loop {
            let (n, outcome) = reader.read(piece)?;
            piece = &piece[n..];
            match outcome {
                Outcome::None => break,
                Outcome::Ack(seq) => items.push(Item::Ack(seq)),
                Outcome::Message => {
                    let m = reader.message().unwrap();
                    let (type_id, body) = (m.type_id, m.body.to_vec());
                    items.push(msg(m.timestamp, m.type_id, m.stream_id, m.body));
                    if let Some(c) = Control::parse(type_id, &body)? {
                        c.apply(reader, &mut writer)?;
                    }
                }
            }
        }
        assert!(piece.is_empty());
        start = end;
    }
    let mut out = vec![0; writer.queued()];
    writer.poll_transmit(&mut out);
    Ok((items, out))
}

fn read(input: &[u8]) -> Result<Vec<Item>, RtmpError> {
    read_cut(&mut ChunkReader::new(LIMITS), input, &[]).map(|(items, _)| items)
}

fn pattern(n: usize, seed: u8) -> Vec<u8> {
    (0..n)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect()
}

/// A format 0 header on a 1-byte chunk stream id.
fn fmt0(csid: u8, timestamp: u32, len: u32, type_id: u8, stream_id: u32) -> Vec<u8> {
    let mut h = vec![csid];
    h.extend(&timestamp.min(EXTENDED).to_be_bytes()[1..]);
    h.extend(&len.to_be_bytes()[1..]);
    h.push(type_id);
    h.extend(stream_id.to_le_bytes());
    if timestamp >= EXTENDED {
        h.extend(timestamp.to_be_bytes());
    }
    h
}

#[test]
fn formats_0_to_3_reassemble() {
    let (a, b, c, d) = (
        pattern(32, 1),
        pattern(32, 2),
        pattern(32, 3),
        pattern(32, 4),
    );
    let (e, f) = (pattern(200, 5), pattern(200, 6));
    let mut input = fmt0(3, 1000, 32, AUDIO, 1);
    input.extend(&a);
    input.extend([0x83, 0, 0, 20]);
    input.extend(&b);
    input.push(0xc3);
    input.extend(&c);
    input.push(0xc3);
    input.extend(&d);
    // Format 1: a new length and type; 200 bytes take two chunks.
    input.extend([0x43, 0, 0, 5, 0, 0, 200, VIDEO]);
    input.extend(&e[..128]);
    input.push(0xc3);
    input.extend(&e[128..]);
    // Format 3 starting a message repeats the last delta.
    input.push(0xc3);
    input.extend(&f[..128]);
    input.push(0xc3);
    input.extend(&f[128..]);
    assert_eq!(
        read(&input).unwrap(),
        [
            msg(1000, AUDIO, 1, &a),
            msg(1020, AUDIO, 1, &b),
            msg(1040, AUDIO, 1, &c),
            msg(1060, AUDIO, 1, &d),
            msg(1065, VIDEO, 1, &e),
            msg(1070, VIDEO, 1, &f),
        ]
    );
}

#[test]
fn format_3_after_format_0_repeats_its_timestamp_as_a_delta() {
    let mut input = fmt0(3, 100, 1, AUDIO, 1);
    input.push(1);
    input.extend([0xc3, 2]);
    assert_eq!(
        read(&input).unwrap(),
        [msg(100, AUDIO, 1, &[1]), msg(200, AUDIO, 1, &[2])]
    );
}

#[test]
fn chunk_stream_ids_take_1_2_and_3_byte_headers() {
    let body = [7u8];
    let mut input = Vec::new();
    let headers: [(&[u8], u32); 6] = [
        (&[0x02], 2),
        (&[0x3f], 63),
        (&[0x00, 0x00], 64),
        (&[0x00, 0xff], 319),
        (&[0x01, 0x00, 0x01], 320),
        (&[0x01, 0xff, 0xff], MAX_CSID),
    ];
    for (basic, csid) in headers {
        input.extend(basic);
        input.extend([0, 0, 0, 0, 0, 1, AUDIO]);
        input.extend(csid.to_le_bytes());
        input.extend(body);
        // A format 3 continuation on the same stream.
        let mut cont = basic.to_vec();
        cont[0] |= 0xc0;
        input.extend(cont);
        input.extend(body);
    }
    let expect: Vec<Item> = headers
        .iter()
        .flat_map(|&(_, csid)| [msg(0, AUDIO, csid, &body), msg(0, AUDIO, csid, &body)])
        .collect();
    assert_eq!(read(&input).unwrap(), expect);

    // The writer picks the same headers.
    let mut w = ChunkWriter::new(usize::MAX, false);
    for (_, csid) in headers {
        w.write(csid, 0, AUDIO, csid, &body).unwrap();
    }
    let mut out = vec![0; w.queued()];
    w.poll_transmit(&mut out);
    let expect: Vec<Item> = headers
        .iter()
        .map(|&(_, csid)| msg(0, AUDIO, csid, &body))
        .collect();
    assert_eq!(read(&out).unwrap(), expect);
    assert_eq!(&out[..1], [0x02]);
}

#[test]
fn extended_timestamps_repeat_on_format_3_chunks() {
    let body = pattern(200, 9);
    let ext = 0x0100_0000u32;
    let mut input = fmt0(4, ext, 200, VIDEO, 1);
    input.extend(&body[..128]);
    input.push(0xc4);
    input.extend(ext.to_be_bytes());
    input.extend(&body[128..]);
    // A format 3 header starting a message carries its delta extended.
    input.push(0xc4);
    input.extend(0x0200_0000u32.to_be_bytes());
    input.extend(&body[..128]);
    input.push(0xc4);
    input.extend(0x0200_0000u32.to_be_bytes());
    input.extend(&body[128..]);
    // Format 2 with a small delta clears the extension.
    input.extend([0x84, 0, 0, 33]);
    input.extend(&body[..128]);
    input.push(0xc4);
    input.extend(&body[128..]);
    // Format 1 with an extended delta: the sum wraps.
    input.extend([0x44, 0xff, 0xff, 0xff, 0, 0, 1, AUDIO]);
    input.extend(0xff00_0000u32.to_be_bytes());
    input.push(5);
    assert_eq!(
        read(&input).unwrap(),
        [
            msg(ext, VIDEO, 1, &body),
            msg(0x0300_0000, VIDEO, 1, &body),
            msg(0x0300_0021, VIDEO, 1, &body),
            msg(0x0300_0021u32.wrapping_add(0xff00_0000), AUDIO, 1, &[5]),
        ]
    );
}

#[test]
fn writer_output_reads_back() {
    let mut w = ChunkWriter::new(usize::MAX, true);
    let mut expect = Vec::new();
    let big = pattern(1000, 3);
    let messages: [(u32, u32, u8, u32, &[u8]); 9] = [
        (6, 0, VIDEO, 1, &big),
        (6, 33, VIDEO, 1, &big),
        (6, 66, VIDEO, 1, &big),
        (6, 0x00ff_fffe, VIDEO, 1, &big[..300]),
        (6, 0x0100_0000, VIDEO, 1, &big[..300]),
        (6, 0x0200_0002, VIDEO, 1, &big[..300]),
        (6, 0x0300_0004, VIDEO, 1, &big[..300]),
        (4, 0x0300_0004, AUDIO, 1, &big[..10]),
        (4, 5, AUDIO, 1, &big[..10]),
    ];
    for (csid, ts, type_id, sid, body) in messages {
        w.write(csid, ts, type_id, sid, body).unwrap();
        expect.push(msg(ts, type_id, sid, body));
    }
    let mut out = vec![0; w.queued()];
    w.poll_transmit(&mut out);
    assert_eq!(read(&out).unwrap(), expect);
}

#[test]
fn writer_takes_the_shortest_format() {
    let mut w = ChunkWriter::new(usize::MAX, true);
    w.write(3, 0, VIDEO, 1, &[1, 2, 3]).unwrap();
    w.write(3, 40, VIDEO, 1, &[4, 5, 6]).unwrap();
    w.write(3, 80, VIDEO, 1, &[7, 8, 9]).unwrap();
    w.write(3, 90, AUDIO, 1, &[1]).unwrap();
    w.write(3, 80, AUDIO, 1, &[2]).unwrap();
    w.write(3, 100, AUDIO, 2, &[3]).unwrap();
    let mut out = vec![0; w.queued()];
    w.poll_transmit(&mut out);
    let expect = [
        &[0x03, 0, 0, 0, 0, 0, 3, VIDEO, 1, 0, 0, 0, 1, 2, 3][..],
        &[0x83, 0, 0, 40, 4, 5, 6],
        &[0xc3, 7, 8, 9],
        &[0x43, 0, 0, 10, 0, 0, 1, AUDIO, 1],
        &[0x03, 0, 0, 80, 0, 0, 1, AUDIO, 1, 0, 0, 0, 2],
        &[0x03, 0, 0, 100, 0, 0, 1, AUDIO, 2, 0, 0, 0, 3],
    ]
    .concat();
    assert_eq!(out, expect);
}

#[test]
fn chunk_size_changes_mid_stream() {
    let mut w = ChunkWriter::new(usize::MAX, false);
    let (a, b) = (pattern(300, 1), pattern(3000, 2));
    w.write(3, 0, VIDEO, 1, &a).unwrap();
    w.write(PROTOCOL_CSID, 0, SET_CHUNK_SIZE, 0, &1000u32.to_be_bytes())
        .unwrap();
    w.set_chunk_size(1000);
    w.write(3, 1, VIDEO, 1, &b).unwrap();
    let mut out = vec![0; w.queued()];
    w.poll_transmit(&mut out);
    // 3000 bytes at 1000 take three chunks: two continuation headers.
    assert_eq!(out.len(), (12 + 2 + 300) + (12 + 4) + (12 + 2 + 3000));
    assert_eq!(
        read(&out).unwrap(),
        [
            msg(0, VIDEO, 1, &a),
            msg(0, SET_CHUNK_SIZE, 0, &1000u32.to_be_bytes()),
            msg(1, VIDEO, 1, &b),
        ]
    );
    // A chunk size past the largest message is clamped to it.
    let mut r = ChunkReader::new(LIMITS);
    r.set_chunk_size(MAX_CHUNK_SIZE);
    assert_eq!(r.chunk_size, LIMITS.max_media);
}

#[test]
fn abort_drops_a_partial_message() {
    let body = pattern(300, 4);
    let mut input = fmt0(5, 0, 300, VIDEO, 1);
    input.extend(&body[..128]);
    input.extend(fmt0(2, 0, 4, ABORT, 0));
    input.extend(5u32.to_be_bytes());
    // Without the abort this header would interrupt the message.
    input.extend(fmt0(5, 9, 2, VIDEO, 1));
    input.extend([1, 2]);
    let mut r = ChunkReader::new(LIMITS);
    let (items, _) = read_cut(&mut r, &input, &[]).unwrap();
    assert_eq!(
        items,
        [
            msg(0, ABORT, 0, &5u32.to_be_bytes()),
            msg(9, VIDEO, 1, &[1, 2])
        ]
    );
    let mut input = fmt0(5, 0, 300, VIDEO, 1);
    input.extend(&body[..128]);
    input.extend(fmt0(5, 9, 2, VIDEO, 1));
    assert_eq!(read(&input), Err(RtmpError::Interrupted(5)));
}

#[test]
fn acks_follow_the_window() {
    let mut w = ChunkWriter::new(usize::MAX, false);
    w.write(PROTOCOL_CSID, 0, WINDOW_ACK_SIZE, 0, &1000u32.to_be_bytes())
        .unwrap();
    let body = pattern(1500, 5);
    for ts in 0..3 {
        w.write(6, ts, VIDEO, 1, &body).unwrap();
    }
    let mut out = vec![0; w.queued()];
    w.poll_transmit(&mut out);
    let items = read(&out).unwrap();
    // The window message is 16 bytes; each video message is a 12-byte
    // header and 128-byte chunks with 1-byte headers (12 chunks of 1500).
    // Acks come at the first chunk end at or past each 1000 bytes.
    let mut received = 16u32;
    let mut acked = 0u32;
    let mut expect = vec![msg(0, WINDOW_ACK_SIZE, 0, &1000u32.to_be_bytes())];
    for ts in 0..3 {
        for chunk in 0..12 {
            let payload = if chunk == 11 { 1500 - 11 * 128 } else { 128 };
            received += if chunk == 0 { 12 } else { 1 } + payload;
            if received - acked >= 1000 {
                acked = received;
                expect.push(Item::Ack(received));
            }
        }
        expect.push(msg(ts, VIDEO, 1, &body));
    }
    assert_eq!(items, expect);
    assert_eq!(
        items.iter().filter(|i| matches!(i, Item::Ack(_))).count(),
        4
    );

    // A zero window sends none.
    let mut r = ChunkReader::new(LIMITS);
    r.set_window(0);
    let (items, _) = read_cut(&mut r, &out[16..], &[]).unwrap();
    assert!(items.iter().all(|i| matches!(i, Item::Msg { .. })));
}

#[test]
fn pings_are_answered() {
    let mut input = fmt0(2, 0, 6, USER_CONTROL, 0);
    input.extend([0, 6, 0xde, 0xad, 0xbe, 0xef]);
    // Other user control events are ignored.
    input.extend(fmt0(2, 0, 6, USER_CONTROL, 0));
    input.extend([0, 3, 0, 0, 0, 1]);
    let (_, out) = read_cut(&mut ChunkReader::new(LIMITS), &input, &[]).unwrap();
    let mut expect = fmt0(2, 0, 6, USER_CONTROL, 0);
    expect.extend([0, 7, 0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(out, expect);
}

#[test]
fn control_messages_are_checked() {
    assert_eq!(
        Control::parse(SET_CHUNK_SIZE, &0u32.to_be_bytes()),
        Err(RtmpError::ChunkSize(0))
    );
    assert_eq!(
        Control::parse(SET_CHUNK_SIZE, &0x8000_0000u32.to_be_bytes()),
        Err(RtmpError::ChunkSize(0x8000_0000))
    );
    assert_eq!(
        Control::parse(SET_CHUNK_SIZE, &MAX_CHUNK_SIZE.to_be_bytes()),
        Ok(Some(Control::SetChunkSize(MAX_CHUNK_SIZE)))
    );
    assert_eq!(
        RtmpError::ChunkSize(0x8000_0000).to_string(),
        "chunk size 0x80000000 is not valid"
    );
    let truncated = [
        (SET_CHUNK_SIZE, &[0, 0, 1][..], "Set Chunk Size"),
        (ABORT, &[], "Abort"),
        (WINDOW_ACK_SIZE, &[1], "Window Acknowledgement Size"),
        (USER_CONTROL, &[0], "User Control"),
        (USER_CONTROL, &[0, 6, 1, 2, 3], "Ping Request"),
    ];
    for (type_id, body, name) in truncated {
        assert_eq!(
            Control::parse(type_id, body),
            Err(RtmpError::Truncated(name))
        );
    }
    assert_eq!(
        RtmpError::Truncated("Abort").to_string(),
        "Abort message is truncated"
    );
    assert_eq!(
        Control::parse(ACKNOWLEDGEMENT, &[]),
        Ok(Some(Control::Ignored))
    );
    assert_eq!(
        Control::parse(SET_PEER_BANDWIDTH, &[0; 5]),
        Ok(Some(Control::Ignored))
    );
    assert_eq!(Control::parse(VIDEO, &[]), Ok(None));
}

#[test]
fn headers_need_a_previous_header() {
    for b0 in [0x43, 0x83, 0xc3] {
        assert_eq!(
            read(&[b0, 0, 0, 0, 0, 0, 1, VIDEO, 0, 0, 0, 0]),
            Err(RtmpError::NoPreviousHeader(3))
        );
    }
}

#[test]
fn messages_are_bounded_by_type() {
    let over_media = fmt0(3, 0, (4 << 20) + 1, VIDEO, 1);
    assert_eq!(
        read(&over_media),
        Err(RtmpError::MessageTooLarge {
            type_id: VIDEO,
            len: (4 << 20) + 1,
            max: 4 << 20,
        })
    );
    let over_command = fmt0(3, 0, (64 << 10) + 1, COMMAND_AMF0, 0);
    let e = read(&over_command).unwrap_err();
    assert_eq!(
        e,
        RtmpError::MessageTooLarge {
            type_id: COMMAND_AMF0,
            len: (64 << 10) + 1,
            max: 64 << 10,
        }
    );
    assert_eq!(
        e.to_string(),
        "message type 20 of 65537 bytes exceeds the 65536-byte limit"
    );
    // At the bound, the header is taken.
    assert_eq!(read(&fmt0(3, 0, 64 << 10, COMMAND_AMF0, 0)), Ok(vec![]));
    // A format 1 header is checked too.
    let mut input = fmt0(3, 0, 1, COMMAND_AMF0, 0);
    input.push(0);
    input.extend([0x43, 0, 0, 0, 0x10, 0, 0, DATA_AMF0]);
    assert!(matches!(
        read(&input),
        Err(RtmpError::MessageTooLarge {
            type_id: DATA_AMF0,
            ..
        })
    ));
}

#[test]
fn buffers_grow_as_chunks_arrive() {
    // A declared 4 MiB message holds only what has arrived.
    let mut r = ChunkReader::new(LIMITS);
    let mut input = fmt0(3, 0, 4 << 20, VIDEO, 1);
    input.extend([0; 100]);
    read_cut(&mut r, &input, &[]).unwrap();
    assert_eq!(r.allocated(), 100);
    // Growth doubles up to the message's length.
    let mut r = ChunkReader::new(LIMITS);
    let mut w = ChunkWriter::new(usize::MAX, false);
    w.write(3, 0, VIDEO, 1, &pattern(300, 0)).unwrap();
    let mut out = vec![0; w.queued()];
    w.poll_transmit(&mut out);
    read_cut(&mut r, &out, &[]).unwrap();
    assert_eq!(r.allocated(), 300);
}

#[test]
fn partial_bytes_are_bounded() {
    let limits = Limits {
        max_partial: 1000,
        ..LIMITS
    };
    // Two partial messages that together exceed the bound.
    let mut input = fmt0(3, 0, 600, VIDEO, 1);
    input.extend([0; 128]);
    input.extend(fmt0(4, 0, 600, VIDEO, 1));
    input.extend([0; 128]);
    for _ in 0..3 {
        input.push(0xc3);
        input.extend([0; 128]);
        input.push(0xc4);
        input.extend([0; 128]);
    }
    let e = read_cut(&mut ChunkReader::new(limits), &input, &[]).unwrap_err();
    assert_eq!(e, RtmpError::PartialTooLarge(1000));
    assert_eq!(e.to_string(), "partial messages exceed 1000 bytes");

    // Complete messages give their buffers back when room is needed.
    let mut w = ChunkWriter::new(usize::MAX, false);
    for csid in 3..20 {
        w.write(csid, 0, VIDEO, 1, &pattern(600, 1)).unwrap();
    }
    let mut out = vec![0; w.queued()];
    w.poll_transmit(&mut out);
    let mut r = ChunkReader::new(limits);
    let (items, _) = read_cut(&mut r, &out, &[]).unwrap();
    assert_eq!(items.len(), 17);
    assert!(r.allocated() <= 1000);
}

#[test]
fn chunk_streams_are_bounded() {
    let mut w = ChunkWriter::new(usize::MAX, false);
    for csid in 2..2 + 64 {
        w.write(csid, 0, AUDIO, 1, &[1]).unwrap();
    }
    let mut out = vec![0; w.queued()];
    w.poll_transmit(&mut out);
    let mut r = ChunkReader::new(LIMITS);
    read_cut(&mut r, &out, &[]).unwrap();
    // Known streams stay usable; a 65th is refused.
    read_cut(&mut r, &[0xc2, 1], &[]).unwrap();
    let mut w = ChunkWriter::new(usize::MAX, false);
    w.write(2 + 64, 0, AUDIO, 1, &[1]).unwrap();
    let mut out = vec![0; w.queued()];
    w.poll_transmit(&mut out);
    let e = read_cut(&mut r, &out, &[]).unwrap_err();
    assert_eq!(e, RtmpError::TooManyChunkStreams(64));
    assert_eq!(e.to_string(), "more than 64 chunk streams");
}

#[test]
fn output_is_bounded() {
    let mut w = ChunkWriter::new(100, false);
    w.push_raw(&[0; 50]).unwrap();
    // 12 header bytes and 38 of body fill it exactly.
    w.write(3, 0, AUDIO, 1, &[0; 38]).unwrap();
    assert_eq!(
        w.write(3, 0, AUDIO, 1, &[]),
        Err(RtmpError::OutputFull(100))
    );
    assert_eq!(w.push_raw(&[0]), Err(RtmpError::OutputFull(100)));
    let mut buf = [0; 60];
    assert_eq!(w.poll_transmit(&mut buf), 60);
    assert_eq!(w.queued(), 40);
    w.write(3, 0, AUDIO, 1, &[0; 48]).unwrap();
    assert_eq!(
        RtmpError::OutputFull(100).to_string(),
        "queued output exceeds 100 bytes"
    );
}

/// A stream covering every header form, a chunk size change, an abort, a
/// window and a ping.
fn mixed_stream() -> Vec<u8> {
    let mut w = ChunkWriter::new(usize::MAX, true);
    w.write(PROTOCOL_CSID, 0, WINDOW_ACK_SIZE, 0, &700u32.to_be_bytes())
        .unwrap();
    for i in 0..4 {
        w.write(6, i * 33, VIDEO, 1, &pattern(300, i as u8))
            .unwrap();
        w.write(4, i * 21, AUDIO, 1, &pattern(20, i as u8)).unwrap();
    }
    w.write(PROTOCOL_CSID, 0, SET_CHUNK_SIZE, 0, &77u32.to_be_bytes())
        .unwrap();
    w.set_chunk_size(77);
    w.write(400, 0x0100_0000, VIDEO, 1, &pattern(500, 7))
        .unwrap();
    w.write(400, 0x0200_0000, VIDEO, 1, &pattern(500, 8))
        .unwrap();
    w.write(PROTOCOL_CSID, 0, USER_CONTROL, 0, &[0, 6, 0, 0, 0, 9])
        .unwrap();
    w.write(70, 5, COMMAND_AMF0, 0, &[]).unwrap();
    let mut out = vec![0; w.queued()];
    w.poll_transmit(&mut out);
    // A partial message and its abort.
    out.extend(fmt0(9, 0, 200, VIDEO, 1));
    out.extend(pattern(77, 1));
    out.extend(fmt0(2, 0, 4, ABORT, 0));
    out.extend(9u32.to_be_bytes());
    out
}

#[test]
fn input_split_at_every_byte_gives_the_same_messages() {
    let input = mixed_stream();
    let (whole, replies) = read_cut(&mut ChunkReader::new(LIMITS), &input, &[]).unwrap();
    // 15 messages; the partial one is aborted.
    assert_eq!(
        whole.iter().filter(|i| matches!(i, Item::Ack(_))).count(),
        3
    );
    assert_eq!(whole.len(), 3 + 15);
    assert!(!replies.is_empty());
    for cut in 0..=input.len() {
        let got = read_cut(&mut ChunkReader::new(LIMITS), &input, &[cut]).unwrap();
        assert_eq!(got, (whole.clone(), replies.clone()), "cut at {cut}");
    }
    let every: Vec<usize> = (1..input.len()).collect();
    let got = read_cut(&mut ChunkReader::new(LIMITS), &input, &every).unwrap();
    assert_eq!(got, (whole, replies));
}
