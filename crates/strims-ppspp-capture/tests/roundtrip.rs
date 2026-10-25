// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use strims_base::time::{Duration, Time};
use strims_ppspp_capture::{
    Clock, Dir, Error, Header, MAX_FRAME, MAX_LABEL, MAX_NAME, MAX_RECORD, MAX_SAMPLE_FIELDS,
    MAX_TAP_FIELDS, Reader, Record, SampleRecord, SwarmParams, TapRecord, Writer,
};

fn t0() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

fn header() -> Header<'static> {
    Header {
        clock: Clock::Sim,
        origin: t0(),
        file: 3,
        node: b"7",
        label: "Chicago cable",
    }
}

fn params() -> SwarmParams {
    SwarmParams {
        chunk_size: 1024,
        integrity_hash_size: 32,
        integrity_signature_size: 64,
        stream_count: 16,
        chunks_per_signature: 64,
        live_window: 1 << 15,
        protection_method: 3,
        merkle_hash_tree_function: 7,
        live_signature_algorithm: 13,
        scheduling_method: 2,
        delivery_mode: 1,
        buffer_layout: 1,
        push_mode: 3,
        request_controller: 1,
        seed_allocation: 2,
        seed_qos: 2,
    }
}

fn records(frame: &[u8]) -> Vec<(Time, Record<'_>)> {
    let ms = Duration::MILLISECOND;
    vec![
        (
            t0(),
            Record::Peer {
                peer: 1,
                name: b"0",
            },
        ),
        (
            t0(),
            Record::Swarm {
                swarm: 1,
                id: &[9; 32],
                params: params(),
            },
        ),
        (
            t0() + ms,
            Record::ChannelOpen {
                peer: 1,
                swarm: 1,
                channel: 8,
                peer_channel: 1,
            },
        ),
        (
            t0() + ms * 2,
            Record::Frame {
                dir: Dir::In,
                peer: 1,
                len: frame.len() as u64,
                body: frame,
            },
        ),
        (
            t0() + ms * 2,
            Record::Tap(TapRecord::new(33, 1, 1, &[42, 1, -5])),
        ),
        // time may step back: a driver's clock isn't monotonic
        (
            t0() + ms,
            Record::Frame {
                dir: Dir::Out,
                peer: 1,
                len: 0,
                body: &[],
            },
        ),
        (
            t0() + ms * 3,
            Record::FrameError {
                peer: 1,
                message: b"bad frame",
            },
        ),
        (t0() + ms * 3, Record::Tap(TapRecord::new(1, 0, 0, &[]))),
        (
            t0() + ms * 3,
            Record::Tap(TapRecord::new(27, 1, 1, &FULL[..MAX_TAP_FIELDS])),
        ),
        (t0() + ms * 4, Record::TapLost { count: 12 }),
        (
            t0() + ms * 4,
            Record::Sample(SampleRecord::new(2, 1, &FULL[..MAX_SAMPLE_FIELDS])),
        ),
        (t0() + ms * 4, Record::Sample(SampleRecord::new(6, 0, &[]))),
        (t0() + ms * 5, Record::ChannelClose { peer: 1, swarm: 1 }),
        (t0() + ms * 5, Record::SwarmGone { swarm: 1 }),
        (t0() + ms * 5, Record::PeerGone { peer: 1 }),
    ]
}

/// Field values of every size and sign.
const FULL: [i64; 16] = [
    0,
    -1,
    1,
    i64::MIN,
    i64::MAX,
    1 << 40,
    -(1 << 40),
    7,
    8,
    9,
    10,
    11,
    12,
    13,
    14,
    15,
];

fn write(header: &Header<'_>, records: &[(Time, Record<'_>)]) -> Vec<u8> {
    let mut w = Writer::new(Vec::new(), header).unwrap();
    for (t, r) in records {
        w.write(*t, r).unwrap();
    }
    assert_eq!(w.written(), w.get_ref().len() as u64);
    w.into_inner()
}

fn read_all(bytes: &[u8]) -> (Reader<&[u8]>, Vec<(Time, String)>) {
    let mut r = Reader::new(bytes).unwrap();
    let mut out = Vec::new();
    while let Some(e) = r.next_record() {
        let e = e.unwrap();
        out.push((e.time, format!("{:?}", e.record)));
    }
    (r, out)
}

#[test]
fn every_record_round_trips() {
    let frame: Vec<u8> = (0..3000u32).map(|i| i as u8).collect();
    let recs = records(&frame);
    let bytes = write(&header(), &recs);
    let mut r = Reader::new(&bytes[..]).unwrap();
    let h = r.header().clone();
    assert_eq!(h.as_header(), header());
    for (t, rec) in &recs {
        let e = r.next_record().unwrap().unwrap();
        assert_eq!((e.time, e.record), (*t, *rec));
    }
    assert!(r.next_record().is_none());
}

#[test]
fn oversized_fields_are_cut_to_their_bounds() {
    let frame = vec![1u8; MAX_FRAME + 100];
    let name = vec![b'n'; MAX_NAME + 1];
    let label = "é".repeat(MAX_LABEL);
    let h = Header {
        node: &name,
        label: &label,
        ..header()
    };
    let bytes = write(
        &h,
        &[(
            t0(),
            Record::Frame {
                dir: Dir::Out,
                peer: 2,
                len: frame.len() as u64,
                body: &frame,
            },
        )],
    );
    let mut r = Reader::new(&bytes[..]).unwrap();
    assert_eq!(r.header().node.len(), MAX_NAME);
    assert!(r.header().label.len() <= MAX_LABEL);
    assert!(r.header().label.chars().all(|c| c == 'é'));
    let e = r.next_record().unwrap().unwrap();
    let Record::Frame { len, body, .. } = e.record else {
        panic!("{e:?}");
    };
    assert_eq!(len, frame.len() as u64);
    assert_eq!(body, &frame[..MAX_FRAME]);
}

#[test]
fn a_truncated_dump_ends_with_an_error() {
    let frame = [5u8; 100];
    let bytes = write(&header(), &records(&frame));
    let (_, all) = read_all(&bytes);
    for cut in 0..bytes.len() {
        match Reader::new(&bytes[..cut]) {
            Err(_) => continue,
            Ok(mut r) => {
                let mut n = 0;
                let mut last = None;
                while let Some(e) = r.next_record() {
                    match e {
                        Ok(e) => {
                            assert_eq!(all[n], (e.time, format!("{:?}", e.record)));
                            n += 1;
                        }
                        Err(e) => last = Some(e),
                    }
                }
                assert!(n < all.len(), "cut {cut}");
                assert!(
                    matches!(last, None | Some(Error::Truncated { .. })),
                    "cut {cut}: {last:?}"
                );
            }
        }
    }
}

#[test]
fn a_malformed_record_is_skipped() {
    let mut w = Writer::new(Vec::new(), &header()).unwrap();
    w.write(
        t0(),
        &Record::Peer {
            peer: 1,
            name: b"a",
        },
    )
    .unwrap();
    let mark = w.get_ref().len();
    w.write(t0(), &Record::PeerGone { peer: 1 }).unwrap();
    w.write(t0(), &Record::TapLost { count: 1 }).unwrap();
    let mut bytes = w.into_inner();
    // PeerGone's id becomes 0: [len, kind, dt, id]
    bytes[mark + 3] = 0;
    let mut r = Reader::new(&bytes[..]).unwrap();
    assert!(r.next_record().unwrap().is_ok());
    let e = r.next_record().unwrap().unwrap_err();
    assert!(e.is_recoverable(), "{e}");
    let e = r.next_record().unwrap().unwrap();
    assert_eq!(e.record, Record::TapLost { count: 1 });
    assert!(r.next_record().is_none());
}

#[test]
fn an_oversized_length_ends_the_dump() {
    let mut bytes = write(&header(), &[]);
    let mut len = Vec::new();
    let mut v = (MAX_RECORD + 1) as u64;
    while v >= 0x80 {
        len.push(v as u8 | 0x80);
        v >>= 7;
    }
    len.push(v as u8);
    bytes.extend_from_slice(&len);
    bytes.extend_from_slice(&[0; 64]);
    let mut r = Reader::new(&bytes[..]).unwrap();
    assert!(matches!(
        r.next_record(),
        Some(Err(Error::Oversized { .. }))
    ));
    assert!(r.next_record().is_none());
}

#[test]
fn unknown_kinds_and_trailing_fields_are_skipped() {
    let mut bytes = write(&header(), &[]);
    // kind 200 with a body, then a PeerGone with an extra field
    bytes.extend_from_slice(&[4, 200, 0, 1, 2]);
    bytes.extend_from_slice(&[4, 2, 0, 5, 99]);
    let mut r = Reader::new(&bytes[..]).unwrap();
    assert_eq!(
        r.next_record().unwrap().unwrap().record,
        Record::Unknown { kind: 200 }
    );
    assert_eq!(
        r.next_record().unwrap().unwrap().record,
        Record::PeerGone { peer: 5 }
    );
    assert!(r.next_record().is_none());
}

#[test]
fn other_files_are_refused() {
    assert!(matches!(Reader::new(&b"STRIMS"[..]), Err(Error::BadMagic)));
    assert!(matches!(
        Reader::new(&b"NOTACAPTURE"[..]),
        Err(Error::BadMagic)
    ));
    let mut bytes = write(&header(), &[]);
    bytes[9] = 3;
    assert!(matches!(
        Reader::new(&bytes[..]),
        Err(Error::UnsupportedVersion(3))
    ));
    bytes[9] = 0;
    assert!(matches!(
        Reader::new(&bytes[..]),
        Err(Error::UnsupportedVersion(0))
    ));
    assert!(matches!(
        Reader::new(&b"STRIMSPC\x00\x01"[..]),
        Err(Error::BadHeader(_))
    ));
}

#[test]
fn version_1_dumps_still_decode() {
    // Version 1 is version 2 without samples and with at most 4 tap
    // fields; its writers padded tap records to 3.
    let recs = [
        (
            t0(),
            Record::Peer {
                peer: 1,
                name: b"0",
            },
        ),
        (t0(), Record::Tap(TapRecord::new(10, 1, 0, &[6, 0, 0]))),
        (t0(), Record::TapLost { count: 1 }),
    ];
    let mut bytes = write(&header(), &recs);
    bytes[9] = 1;
    let mut r = Reader::new(&bytes[..]).unwrap();
    assert_eq!(r.header().version, 1);
    for (t, rec) in &recs {
        let e = r.next_record().unwrap().unwrap();
        assert_eq!((e.time, e.record), (*t, *rec));
    }
    assert!(r.next_record().is_none());
}

#[test]
fn records_past_the_field_limits_are_malformed() {
    let mut w = Writer::new(Vec::new(), &header()).unwrap();
    w.write(t0(), &Record::Sample(SampleRecord::new(1, 0, &[1])))
        .unwrap();
    let mut bytes = w.into_inner();
    // [len, kind, dt, source, peer, n, field]: n becomes 17
    let at = bytes.len() - 2;
    bytes[at] = (MAX_SAMPLE_FIELDS + 1) as u8;
    let mut r = Reader::new(&bytes[..]).unwrap();
    let e = r.next_record().unwrap().unwrap_err();
    assert!(e.is_recoverable(), "{e}");
}
