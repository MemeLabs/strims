// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Codec tests: message round trips, decoder edge cases, and the reader
//! and writers.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use strims_base::time::{Duration, Time};
use strims_binmap::Bin;

use super::*;

const FF16: [u8; 16] = [0xff; 16];
const FF64: [u8; 64] = [0xff; 64];

fn config() -> ReaderConfig {
    ReaderConfig {
        chunk_size: 16,
        integrity_hash_size: 16,
        integrity_signature_size: 16,
    }
}

fn ms(n: i64) -> Time {
    Time::from_unix_millis(n)
}

const ANNOUNCED: [AnnounceEntry<'static>; 2] = [
    AnnounceEntry {
        stream: 3,
        hits: 60,
        late: 2,
        misses: 4,
        delay: Duration::from_millis(35),
        parent: &[7],
    },
    AnnounceEntry {
        stream: 300,
        hits: 0,
        late: 0,
        misses: 1234,
        delay: Duration::ZERO,
        parent: &FF16,
    },
];

fn round_trip(m: Message<'_>) {
    let mut buf = vec![0u8; m.encoded_len()];
    assert_eq!(
        m.encode(&mut buf),
        Ok(buf.len()),
        "bytes written by encode should match encoded_len: {m:?}"
    );
    assert_eq!(buf[0], m.message_type().as_u8());
    let (d, n) = Message::decode(&config(), &buf).unwrap();
    assert_eq!(n, buf.len(), "decode should read what encode wrote: {m:?}");
    assert_eq!(d, m, "decoded message should match: {m:?}");

    // one byte short never panics and never succeeds with the full length
    if let Ok((_, n)) = Message::decode(&config(), &buf[..buf.len() - 1]) {
        assert!(n < buf.len());
    }
    assert_eq!(
        m.encode(&mut buf[..m.encoded_len() - 1]),
        Err(Error::NotEnoughSpace)
    );
}

/// Every message type round-trips. The primitives (addresses, buffers,
/// timestamps, delay samples, nonces, streams, channels) are only
/// reachable through messages, so each is covered by the message that
/// carries it.
#[test]
fn message_marshal_unmarshal() {
    let swarm_id = [0xffu8; 64];
    let options = [
        ProtocolOption::Version(123),
        ProtocolOption::MinimumVersion(123),
        ProtocolOption::LiveWindow(1 << 20),
        ProtocolOption::ChunkSize(1024),
        ProtocolOption::ChunksPerSignature(64),
        ProtocolOption::StreamCount(64),
        ProtocolOption::ContentIntegrityProtectionMethod(1),
        ProtocolOption::MerkleHashTreeFunction(1),
        ProtocolOption::LiveSignatureAlgorithm(1),
        ProtocolOption::SwarmIdentifier(&swarm_id),
        ProtocolOption::Epoch(EpochOption {
            timestamp: ms(1234),
            signature: &FF16,
        }),
    ];
    for o in &options {
        round_trip(Message::Handshake(Handshake {
            channel_id: 0,
            options: ProtocolOptions::from_slice(core::slice::from_ref(o)),
        }));
    }

    let cases = [
        Message::Handshake(Handshake {
            channel_id: 7,
            options: ProtocolOptions::from_slice(&options),
        }),
        Message::Data(Data {
            bin: Bin(22),
            timestamp: ms(1234),
            data: &FF16,
        }),
        Message::Ack(Ack {
            bin: Bin(22),
            delay_sample: Duration::from_millis(1234),
        }),
        Message::Integrity(Integrity {
            bin: Bin(22),
            hash: &FF16,
        }),
        Message::SignedIntegrity(SignedIntegrity {
            bin: Bin(22),
            timestamp: ms(1234),
            signature: &FF16,
        }),
        Message::Ping(Ping { nonce: 1234 }),
        Message::Pong(Pong {
            nonce: 1234,
            delay: 0,
        }),
        Message::Have(Have { bin: Bin(1234) }),
        Message::Request(Request {
            bin: Bin(1234),
            timestamp: ms(1234),
        }),
        Message::Cancel(Cancel { bin: Bin(1234) }),
        Message::StreamRequest(StreamRequest {
            stream: 16,
            bin: Bin(1234),
        }),
        Message::StreamCancel(StreamCancel { stream: 16 }),
        Message::StreamOpen(StreamOpen {
            stream: 16,
            bin: Bin(1234),
        }),
        Message::StreamClose(StreamClose { stream: 16 }),
        Message::Announce(Announce::ASK),
        Message::Announce(Announce::new(&ANNOUNCED).unwrap()),
        Message::Choke,
        Message::Unchoke,
        Message::Restart,
        Message::End,
    ];
    for m in cases {
        round_trip(m);
    }

    let h = ChannelHeader {
        channel: 16,
        length: 16384,
    };
    let mut b = vec![0u8; h.byte_len()];
    assert_eq!(h.encode(&mut b), Ok(b.len()));
    assert_eq!(ChannelHeader::decode(&b), Ok((h, b.len())));
    let h = ChannelHeader {
        channel: 1234,
        length: 0,
    };
    let mut b = vec![0u8; h.byte_len()];
    assert_eq!(h.encode(&mut b), Ok(b.len()));
    assert_eq!(ChannelHeader::decode(&b), Ok((h, b.len())));
}

#[test]
fn extreme_values_round_trip() {
    for t in [i64::MIN / 1_000_000, -1, 0, i64::MAX / 1_000_000] {
        round_trip(Message::Request(Request {
            bin: Bin::NONE,
            timestamp: ms(t),
        }));
        round_trip(Message::Ack(Ack {
            bin: Bin::ALL,
            delay_sample: Duration::from_millis(t),
        }));
    }
}

#[test]
fn reader_iterates_until_end() {
    let msgs = [
        Message::Have(Have { bin: Bin(4) }),
        Message::Choke,
        Message::End,
        Message::Unchoke,
    ];
    let mut buf = [0u8; 64];
    let mut w = Writer::new(&mut buf);
    for m in &msgs {
        w.write(m).unwrap();
    }
    let n = w.len();
    let mut r = Reader::new(config(), &buf[..n]);
    assert_eq!(r.next(), Some(Ok(msgs[0])));
    assert_eq!(r.next(), Some(Ok(msgs[1])));
    assert_eq!(r.next(), None);
    // stopped after the end marker, which is consumed
    assert_eq!(r.position(), 4);
    assert_eq!(r.next(), None);
}

#[test]
fn reader_errors_are_terminal() {
    let buf = [MessageType::Choke.as_u8(), 0x42, MessageType::Choke.as_u8()];
    let r: Vec<_> = Reader::new(config(), &buf).collect();
    assert_eq!(
        r,
        vec![Ok(Message::Choke), Err(Error::UnsupportedMessageType(0x42))]
    );

    let buf = [MessageType::Have.as_u8(), 0x80];
    let r: Vec<_> = Reader::new(config(), &buf).collect();
    assert_eq!(r, vec![Err(Error::Truncated)]);
}

#[test]
fn handshake_options_decode() {
    // no end marker: options run to the end of the payload
    let buf = [1u8, 0, 0, 0, 9, 0, 2, 1, 1, 3, 3];
    let (m, n) = Message::decode(&config(), &buf).unwrap();
    assert_eq!(n, buf.len());
    let Message::Handshake(h) = m else {
        panic!("not a handshake")
    };
    assert_eq!(h.channel_id, 9);
    assert_eq!(
        h.options.iter().collect::<Vec<_>>(),
        vec![
            ProtocolOption::Version(2),
            ProtocolOption::MinimumVersion(1),
            ProtocolOption::ContentIntegrityProtectionMethod(3)
        ]
    );
    assert_eq!(
        h.options.find(ProtocolOptionType::MinimumVersion),
        Some(ProtocolOption::MinimumVersion(1))
    );
    assert_eq!(h.options.find(ProtocolOptionType::ChunkSize), None);
    let s = h.options.summary();
    assert_eq!(s.version, Some(2));
    assert_eq!(s.chunk_size, None);

    // unknown and bodiless option types are rejected
    for t in [6u8, 8, 13, 254] {
        let buf = [1u8, 0, 0, 0, 0, t, 0, 0, 0, 0];
        assert_eq!(
            Message::decode(&config(), &buf),
            Err(Error::UnsupportedProtocolOption(t))
        );
    }

    // duplicates are kept in order; summary and find take the first
    let opts = [
        ProtocolOption::StreamCount(4),
        ProtocolOption::StreamCount(8),
    ];
    let hs = Message::Handshake(Handshake {
        channel_id: 0,
        options: ProtocolOptions::from_slice(&opts),
    });
    let mut buf = [0u8; 32];
    let n = hs.encode(&mut buf).unwrap();
    let (d, _) = Message::decode(&config(), &buf[..n]).unwrap();
    assert_eq!(d, hs);
    let Message::Handshake(h) = d else {
        panic!("not a handshake")
    };
    assert_eq!(h.options.summary().stream_count, Some(4));
    let mut again = [0u8; 32];
    assert_eq!(d.encode(&mut again), Ok(n));
    assert_eq!(again[..n], buf[..n]);
}

#[test]
fn swarm_identifier_length_is_checked() {
    let id = vec![0u8; 65536];
    let opts = [ProtocolOption::SwarmIdentifier(&id)];
    let m = Message::Handshake(Handshake {
        channel_id: 0,
        options: ProtocolOptions::from_slice(&opts),
    });
    let mut buf = vec![0u8; m.encoded_len()];
    assert_eq!(m.encode(&mut buf), Err(Error::SwarmIdentifierTooLong));
}

#[test]
fn data_takes_what_remains() {
    let mut buf = [0u8; 128];
    let full = Message::Data(Data {
        bin: Bin(1),
        timestamp: ms(5),
        data: &FF64[..32],
    });
    let n = full.encode(&mut buf).unwrap();
    // a short payload yields short data
    let (m, used) = Message::decode(&config(), &buf[..n - 10]).unwrap();
    assert_eq!(used, n - 10);
    let Message::Data(d) = m else {
        panic!("not data")
    };
    assert_eq!(d.data.len(), 22);
    // a bin whose size overflows takes the rest of the payload
    let huge = Message::Data(Data {
        bin: Bin::ALL,
        timestamp: ms(5),
        data: &FF64,
    });
    let n = huge.encode(&mut buf).unwrap();
    let (m, used) = Message::decode(&config(), &buf[..n]).unwrap();
    assert_eq!(used, n);
    assert_eq!(m, huge);
}

#[test]
fn writer_reports_not_enough_space() {
    let mut buf = [0u8; 10];
    let mut w = Writer::new(&mut buf);
    assert_eq!(w.write(&Message::Ping(Ping { nonce: 1 })), Ok(9));
    assert_eq!(w.available(), 1);
    assert_eq!(
        w.write(&Message::Ping(Ping { nonce: 2 })),
        Err(Error::NotEnoughSpace)
    );
    assert_eq!(w.len(), 9, "a failed write leaves the buffer as it was");
    assert_eq!(w.write(&Message::Choke), Ok(1));
    assert_eq!(w.available(), 0);
    w.reset();
    assert!(w.is_empty());
}

#[test]
fn channel_writer_frames_payload() {
    let mut frame = [0u8; 64];
    let mut off = 0;
    for (channel, msgs) in [
        (300u64, &[Message::Have(Have { bin: Bin(7) })][..]),
        (1, &[][..]),
        (0, &[Message::Choke, Message::Ping(Ping { nonce: 3 })][..]),
    ] {
        let mut cw = ChannelWriter::new(&mut frame[off..], channel);
        for m in msgs {
            cw.write(m).unwrap();
        }
        off += cw.finish();
    }

    let mut got = vec![];
    for c in FrameReader::new(&frame[..off]) {
        let (h, payload) = c.unwrap();
        let msgs: Vec<_> = Reader::new(config(), payload).map(Result::unwrap).collect();
        got.push((h.channel, msgs));
    }
    assert_eq!(
        got,
        vec![
            (300, vec![Message::Have(Have { bin: Bin(7) })]),
            (0, vec![Message::Choke, Message::Ping(Ping { nonce: 3 })]),
        ]
    );

    // a header claiming more than the frame holds
    assert_eq!(
        FrameReader::new(&[0, 0, 5, 1]).collect::<Vec<_>>(),
        vec![Err(Error::Truncated)]
    );

    // no room for the header
    let mut tiny = [0u8; 2];
    let mut cw = ChannelWriter::new(&mut tiny, 0);
    assert_eq!(cw.available(), 0);
    assert_eq!(cw.write(&Message::Choke), Err(Error::NotEnoughSpace));
    assert_eq!(cw.finish(), 0);
}

#[test]
fn channel_writer_caps_payload_at_u16() {
    let mut frame = vec![0u8; 70_000];
    let mut cw = ChannelWriter::new(&mut frame, 0);
    assert_eq!(cw.available(), usize::from(u16::MAX));
    let data = vec![1u8; 65_000];
    cw.write(&Message::Data(Data {
        bin: Bin(0),
        timestamp: ms(0),
        data: &data,
    }))
    .unwrap();
    assert!(cw.available() < 600);
    assert_eq!(
        cw.write(&Message::Data(Data {
            bin: Bin(2),
            timestamp: ms(0),
            data: &data[..600],
        })),
        Err(Error::NotEnoughSpace)
    );
    let n = cw.finish();
    let (h, hn) = ChannelHeader::decode(&frame).unwrap();
    assert_eq!(usize::from(h.length) + hn, n);
}

#[test]
fn type_names_and_wire_values() {
    assert_eq!(MessageType::SignedIntegrity.to_string(), "SignedIntegrity");
    assert_eq!(MessageType::End.to_string(), "End");
    assert_eq!(ProtocolOptionType::End.to_string(), "EndOption");
    assert_eq!(MessageType::from_u8(0), None);
    assert_eq!(MessageType::from_u8(18), Some(MessageType::Announce));
    assert_eq!(MessageType::from_u8(19), None);
    assert_eq!(
        ProtocolOptionType::from_u8(12),
        Some(ProtocolOptionType::Epoch)
    );
}

#[test]
fn announce_bounds_are_decode_errors() {
    fn decode(b: &[u8]) -> Result<(), Error> {
        Message::decode(&config(), b).map(|_| ())
    }
    let entry = |stream: u64, id_len: u8| {
        let mut e = vec![0x80 | (stream & 0x7f) as u8, (stream >> 7) as u8];
        e.extend([1, 0, 0, 5, id_len]);
        e.extend(std::iter::repeat_n(9u8, usize::from(id_len)));
        e
    };
    let message = |entries: &[Vec<u8>]| {
        let mut m = vec![MessageType::Announce.as_u8(), entries.len() as u8];
        for e in entries {
            m.extend(e);
        }
        m
    };
    let ok = entry(5, MAX_PEER_ID as u8);
    assert!(decode(&message(std::slice::from_ref(&ok))).is_ok());
    assert_eq!(
        decode(&message(&[entry(5, 0)])),
        Err(Error::InvalidAnnounce)
    );
    assert_eq!(
        decode(&message(&[entry(5, MAX_PEER_ID as u8 + 1)])),
        Err(Error::InvalidAnnounce)
    );
    // A stream past 16 bits.
    let mut wide = vec![0xff, 0xff, 0x04];
    wide.extend([1, 0, 0, 5, 1, 9]);
    assert_eq!(decode(&message(&[wide])), Err(Error::InvalidAnnounce));
    let many = vec![ok; MAX_ANNOUNCE_ENTRIES + 1];
    assert_eq!(decode(&message(&many)), Err(Error::InvalidAnnounce));
    // A count past the entries is truncated input.
    let mut short = message(&[entry(5, 1)]);
    short[1] = 2;
    assert_eq!(decode(&short), Err(Error::Truncated));
}

#[test]
fn announce_entries_are_built_from_their_encodings() {
    let mut bytes = Vec::new();
    for e in &ANNOUNCED {
        let start = bytes.len();
        bytes.resize(start + e.byte_len(), 0);
        assert_eq!(e.encode(&mut bytes[start..]), Ok(e.byte_len()));
    }
    let a = Announce::from_entry_bytes(2, &bytes).unwrap();
    assert_eq!(a, Announce::new(&ANNOUNCED).unwrap());
    assert!(Announce::from_entry_bytes(1, &bytes).is_none());
    assert!(Announce::from_entry_bytes(3, &bytes).is_none());
    let unnamed = AnnounceEntry {
        parent: &[],
        ..ANNOUNCED[0]
    };
    assert!(Announce::new(&[unnamed]).is_none());
    assert_eq!(unnamed.encode(&mut [0; 64]), Err(Error::InvalidAnnounce));
    let mut small = [0u8; 4];
    assert_eq!(ANNOUNCED[0].encode(&mut small), Err(Error::NotEnoughSpace));
    assert_eq!(small, [0; 4]);
}
