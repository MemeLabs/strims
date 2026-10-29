// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Buffer and reader tests: write orders, readers, recovery, and the
//! window's bounds.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::sync::Arc;

use strims_base::time::Time;
use strims_binmap::{Bin, Map};

use super::*;
use crate::codec::{self, Message, MessageWriter};
use crate::store::RewindError;

fn bin_byte(b: Bin, cs: usize) -> usize {
    (b.0 / 2) as usize * cs
}

/// Reads until `n` bytes or `WouldBlock`.
fn read_n(r: &mut BufferReader, b: &Buffer, n: usize) -> Vec<u8> {
    let mut out = vec![];
    let mut tmp = [0u8; 4096];
    while out.len() < n {
        let want = (n - out.len()).min(tmp.len());
        match r.read(b, &mut tmp[..want]) {
            Ok(m) => out.extend_from_slice(&tmp[..m]),
            Err(Error::WouldBlock) => break,
            Err(e) => panic!("read failed: {e}"),
        }
    }
    out
}

/// Chunks written in several orders, then read back in stream order.
#[test]
fn buffer_write_read() {
    struct Case {
        label: &'static str,
        chunk_count: usize,
        chunk_size: usize,
        input_bin: Bin,
        write_order: Vec<Bin>,
        read_size: usize,
    }
    let cases = [
        Case {
            label: "write 65536 bytes read 4096 byte segments",
            chunk_count: 1024,
            chunk_size: 2048,
            input_bin: Bin::new(5, 0),
            write_order: vec![Bin::new(5, 0)],
            read_size: 4096,
        },
        Case {
            label: "write 16384 bytes read 1024 byte segments",
            chunk_count: 1024,
            chunk_size: 1024,
            input_bin: Bin::new(4, 3),
            write_order: vec![Bin::new(4, 3)],
            read_size: 1024,
        },
        Case {
            label: "write 8192 bytes advancing head read 128 byte segments",
            chunk_count: 1024,
            chunk_size: 1024,
            input_bin: Bin::new(3, 128),
            write_order: vec![Bin::new(3, 128)],
            read_size: 128,
        },
        Case {
            label: "write 8192 bytes in order read 128 byte segments",
            chunk_count: 1024,
            chunk_size: 1024,
            input_bin: Bin::new(3, 1),
            write_order: vec![Bin::new(2, 2), Bin::new(2, 3)],
            read_size: 128,
        },
        Case {
            label: "write 8192 bytes in reverse order read 128 byte segments",
            chunk_count: 1024,
            chunk_size: 1024,
            input_bin: Bin::new(3, 0),
            write_order: vec![
                Bin::new(2, 1),
                Bin::new(1, 1),
                Bin::new(0, 1),
                Bin::new(0, 0),
            ],
            read_size: 128,
        },
        Case {
            label: "write 8192 bytes in random order read 128 byte segments",
            chunk_count: 1024,
            chunk_size: 1024,
            input_bin: Bin::new(3, 0),
            write_order: [7, 6, 3, 4, 0, 1, 2, 5]
                .iter()
                .map(|&o| Bin::new(0, o))
                .collect(),
            read_size: 128,
        },
    ];

    for c in cases {
        let mut b = Buffer::new(c.chunk_count, c.chunk_size).unwrap();
        let bin_byte_len = c.input_bin.base_length() as usize * c.chunk_size;
        let bin_data: Vec<u8> = (0..bin_byte_len).map(|i| (i / c.read_size) as u8).collect();

        let mut r = BufferReader::new(&b);
        assert_eq!(r.offset(&b), Err(Error::WouldBlock), "{}", c.label);

        b.set_offset(c.input_bin);
        let off = bin_byte(c.input_bin.base_left(), c.chunk_size);
        for &bin in &c.write_order {
            let l = bin_byte(bin.base_left(), c.chunk_size) - off;
            let h = bin_byte(bin.base_right() + 2, c.chunk_size) - off;
            b.consume(bin, &bin_data[l..h]).unwrap();
        }

        assert_eq!(
            r.offset(&b),
            Ok(off as u64),
            "read offset mismatch: {}",
            c.label
        );

        let mut read_data = vec![0u8; c.read_size];
        for i in 0..bin_byte_len / c.read_size {
            let n = r.read(&b, &mut read_data).unwrap();
            let (l, h) = (i * c.read_size, (i + 1) * c.read_size);
            assert_eq!(
                n, c.read_size,
                "incomplete read from {l} - {h}: {}",
                c.label
            );
            assert_eq!(
                read_data,
                bin_data[l..h],
                "misaligned read from {l} - {h}: {}",
                c.label
            );
        }
        assert_eq!(r.read(&b, &mut read_data), Err(Error::WouldBlock));
    }
}

/// The same orders, reading as soon as anything is readable.
#[test]
fn buffer_interleaved_write_read() {
    let cs = 64;
    let mut b = Buffer::new(64, cs).unwrap();
    let mut r = BufferReader::new(&b);
    b.set_offset(Bin(0));
    let data: Vec<u8> = (0..16 * cs).map(|i| (i * 7 % 251) as u8).collect();
    let mut got = vec![];
    for o in [3u64, 1, 0, 2, 7, 4, 6, 5, 8, 9, 10, 15, 11, 12, 13, 14] {
        let bin = Bin::new(0, o);
        let l = o as usize * cs;
        b.consume(bin, &data[l..l + cs]).unwrap();
        got.extend(read_n(&mut r, &b, usize::MAX));
    }
    assert_eq!(got, data);
}

/// Independent readers each read the whole window.
#[test]
fn multiple_readers() {
    let (chunk_count, chunk_size) = (1024, 1024);
    let total = chunk_size * chunk_count;

    let mut b = Buffer::new(chunk_count, chunk_size).unwrap();
    b.set_offset(Bin(0));
    let src = vec![0u8; chunk_size];
    for i in 0..chunk_count as u64 {
        b.consume(Bin::new(0, i), &src).unwrap();
    }

    for _ in 0..3 {
        let mut r = BufferReader::new(&b);
        assert_eq!(read_n(&mut r, &b, total).len(), total);
    }
}

/// An underrun is sticky until the reader recovers at the new tail.
#[test]
fn buffer_recover() {
    let mut b = Buffer::new(1024, 16).unwrap();
    b.set_offset(Bin(0));

    let src = [0u8; 16];
    let mut dst = vec![0u8; 128 * 16];

    let mut i = Bin(0);
    while i < Bin(256) {
        b.consume(i, &src).unwrap();
        i = i.layer_right();
    }

    let mut r = BufferReader::new(&b);
    assert_eq!(r.read(&b, &mut dst), Ok(128 * 16));

    b.consume(Bin(4096), &src).unwrap();

    assert_eq!(r.read(&b, &mut dst), Err(Error::BufferUnderrun));
    // sticky until recovered
    assert_eq!(r.read(&b, &mut dst), Err(Error::BufferUnderrun));

    assert_eq!(r.recover(&mut b), Ok(1920 * 16));
    assert_eq!(r.read(&b, &mut dst), Ok(16));
}

/// A read with nothing to read returns `WouldBlock`.
#[test]
fn buffer_read_would_block() {
    let b = Buffer::new(1024, 16).unwrap();
    let mut r = BufferReader::new(&b);
    assert_eq!(r.read(&b, &mut []), Err(Error::WouldBlock));
    assert!(!r.is_readable(&b));
}

/// A closed reader fails every later read and close.
#[test]
fn reader_close() {
    let (chunk_count, chunk_size) = (1024, 1024);
    let total = chunk_size * chunk_count;

    let mut b = Buffer::new(chunk_count, chunk_size).unwrap();
    b.set_offset(Bin(0));
    let src = vec![0u8; chunk_size];
    for i in 0..chunk_count as u64 {
        b.consume(Bin::new(0, i), &src).unwrap();
    }

    let mut r = BufferReader::new(&b);
    assert_eq!(read_n(&mut r, &b, total).len(), total);
    assert_eq!(r.read(&b, &mut []), Err(Error::WouldBlock));

    assert_eq!(r.close(), Ok(()));
    assert_eq!(
        r.read(&b, &mut []),
        Err(Error::Closed),
        "read after close should return closed"
    );
    assert_eq!(r.close(), Err(Error::Closed));
}

#[test]
fn buffer_close_and_reset_reach_readers() {
    let cs = 16;
    let mut b = Buffer::new(16, cs).unwrap();
    b.set_offset(Bin(0));
    b.consume(Bin(0), &[1; 16]).unwrap();
    let mut r = BufferReader::new(&b);
    let seq = b.readable_seq();

    b.reset();
    assert!(b.readable_seq() > seq);
    assert!(r.is_readable(&b));
    assert_eq!(r.read(&b, &mut [0; 16]), Err(Error::StreamReset));
    // a reset reader stays reset; a new one waits for the new offset
    assert_eq!(r.recover(&mut b), Err(Error::StreamReset));
    let mut r2 = BufferReader::new(&b);
    assert_eq!(r2.offset(&b), Err(Error::WouldBlock));
    b.set_offset(Bin(0));
    b.consume(Bin(0), &[2; 16]).unwrap();
    assert_eq!(read_n(&mut r2, &b, 16), vec![2; 16]);

    b.close();
    assert_eq!(r2.read(&b, &mut [0; 16]), Err(Error::Closed));
    assert_eq!(
        BufferReader::new(&b).read(&b, &mut [0; 16]),
        Err(Error::Closed)
    );
}

/// `set_offset` never moves the window back over overwritten chunks.
#[test]
fn set_offset_stops_at_tail() {
    let cs = 4;
    let mut b = Buffer::new(4, cs).unwrap();
    b.set_offset(Bin(0));
    for o in 0..8u64 {
        b.consume(Bin::new(0, o), &[o as u8; 4]).unwrap();
    }
    assert_eq!(b.tail(), Bin(8));
    b.set_offset(Bin(14));
    assert_eq!(
        b.tail(),
        Bin(8),
        "the window must not move back over overwritten chunks"
    );
    let mut r = BufferReader::new(&b);
    let got = read_n(&mut r, &b, 64);
    assert_eq!(got, [[4u8; 4], [5; 4], [6; 4], [7; 4]].concat());
}

/// A slow reader whose data is overwritten gets an underrun, never
/// overwritten bytes, and recovers at the tail.
#[test]
fn slow_reader_underruns() {
    let cs = 4;
    let mut b = Buffer::new(4, cs).unwrap();
    b.set_offset(Bin(0));
    let mut r = BufferReader::new(&b);
    for o in 0..6u64 {
        b.consume(Bin::new(0, o), &[o as u8; 4]).unwrap();
    }
    assert_eq!(b.next(), Bin(12));
    assert_eq!(r.read(&b, &mut [0; 4]), Err(Error::BufferUnderrun));
    assert_eq!(r.recover(&mut b), Ok(8));
    assert_eq!(
        read_n(&mut r, &b, 64),
        [[2u8; 4], [3; 4], [4; 4], [5; 4]].concat()
    );
}

/// A read of `n` bytes started at `position() - n`: when the read resyncs
/// the reader to a new read offset, after an underrun (the old position
/// plus the bytes skipped) and in a new stream after a reset.
#[test]
fn a_read_starts_its_length_before_the_position() {
    let cs = 4;
    let mut b = Buffer::new(4, cs).unwrap();
    let mut r = BufferReader::new(&b);
    b.set_offset(Bin(4));
    b.consume(Bin(4), &[2; 4]).unwrap();
    let mut p = [0u8; 2];
    assert_eq!(r.read(&b, &mut p), Ok(2));
    assert_eq!(r.position() - 2, 8, "the read resynced to chunk 2");
    assert_eq!(r.read(&b, &mut p), Ok(2));
    assert_eq!(r.position() - 2, 10);
    let before = r.position();

    for o in 3..8u64 {
        b.consume(Bin::new(0, o), &[o as u8; 4]).unwrap();
    }
    assert_eq!(r.read(&b, &mut p), Err(Error::BufferUnderrun));
    let skipped = r.recover(&mut b).unwrap();
    assert_eq!(skipped, 4);
    assert_eq!(r.read(&b, &mut p), Ok(2));
    assert_eq!(r.position() - 2, before + skipped);
    assert_eq!(p, [4; 2]);

    b.reset();
    assert_eq!(r.read(&b, &mut p), Err(Error::StreamReset));
    let mut r = BufferReader::new(&b);
    b.set_offset(Bin(0));
    b.consume(Bin(0), &[9; 4]).unwrap();
    assert_eq!(r.read(&b, &mut p), Ok(2));
    assert_eq!(r.position() - 2, 0);
    assert_eq!(p, [9; 2]);
}

#[test]
fn bins_and_bounds() {
    let mut b = Buffer::new(16, 4).unwrap();
    assert!(b.bins().is_empty());
    assert_eq!(b.tail(), Bin::NONE);
    b.set_offset(Bin(0));
    b.consume(Bin(2), &[0; 4]).unwrap();
    b.consume(Bin(9), &[0; 8]).unwrap();
    let bins = b.bins();
    assert!(bins.filled_at(Bin(2)) && bins.filled_at(Bin(9)) && bins.empty_at(Bin(0)));

    assert_eq!(b.consume(Bin(4), &[0; 3]), Err(Error::DataLength));
    assert_eq!(
        b.consume(Bin::new(5, 0), &[0; 128]),
        Err(Error::BinOutOfBounds)
    );
    assert_eq!(b.consume(Bin::NONE, &[0; 4]), Err(Error::BinOutOfBounds));
    assert_eq!(b.consume(Bin::ALL, &[]), Err(Error::BinOutOfBounds));

    let mut w = Recorder::default();
    assert_eq!(b.write_data(Bin(2), Time::EPOCH, &mut w), Ok(1 + 1 + 1 + 4));
    assert_eq!(b.write_data(Bin(9), Time::EPOCH, &mut w), Ok(1 + 1 + 1 + 8));
    assert_eq!(
        b.write_data(Bin(0), Time::EPOCH, &mut w),
        Err(Error::BinDataNotSet)
    );
    assert_eq!(
        b.write_data(Bin(1), Time::EPOCH, &mut w),
        Err(Error::BinDataNotSet)
    );
    assert_eq!(
        b.write_data(Bin::NONE, Time::EPOCH, &mut w),
        Err(Error::BinDataNotSet)
    );
    assert_eq!(w.0, vec![Bin(2), Bin(9)]);

    assert_eq!(Buffer::new(3, 4).err(), Some(Error::InvalidSize));
    assert_eq!(Buffer::new(0, 4).err(), Some(Error::InvalidSize));
    assert_eq!(
        Buffer::with_layout(4, 4, BufferLayout(9)).err(),
        Some(Error::UnsupportedLayout)
    );
}

#[derive(Default)]
struct Recorder(Vec<Bin>);

impl MessageWriter for Recorder {
    fn write(&mut self, m: &Message<'_>) -> Result<usize, codec::Error> {
        if let Message::Data(d) = m {
            self.0.push(d.bin);
        }
        Ok(m.encoded_len())
    }
}

/// The ring is pruned as the window moves, so gaps far behind it don't
/// accumulate in the bin map.
#[test]
fn bins_stay_bounded() {
    let cs = 1;
    let mut b = Buffer::new(64, cs).unwrap();
    b.set_offset(Bin(0));
    for o in (0..100_000u64).filter(|o| o % 3 != 0) {
        b.consume(Bin::new(0, o), &[0]).unwrap();
    }
    assert!(b.bins.debug_string().len() < 4096);
}

#[test]
fn cache_import_export() {
    let cs = 8;
    let mut b = Buffer::new(16, cs).unwrap();
    assert_eq!(b.export_cache(), Err(Error::CacheTruncated));
    b.set_offset(Bin(0));
    assert_eq!(b.export_cache(), Err(Error::CacheEmpty));
    let data: Vec<u8> = (0..5 * cs as u8).collect();
    for (i, c) in data.chunks(cs).enumerate() {
        b.consume(Bin(2 * i as u64), c).unwrap();
    }
    let cache = b.export_cache().unwrap();
    assert_eq!(cache, data);

    let mut c = Buffer::new(16, cs).unwrap();
    c.import_cache(&cache).unwrap();
    assert_eq!(c.next(), Bin(10));
    assert_eq!(c.export_cache().unwrap(), data);
    let mut r = BufferReader::new(&c);
    assert_eq!(read_n(&mut r, &c, 1000), data);

    assert_eq!(c.import_cache(&vec![0; 17 * cs]), Err(Error::CacheTooLarge));

    // after the window moves on, the cache would be truncated
    for o in 5..20u64 {
        b.consume(Bin::new(0, o), &[0; 8]).unwrap();
    }
    assert_eq!(b.export_cache(), Err(Error::CacheTruncated));
}

#[test]
fn elastic_layout() {
    let cs = 256;
    let mut b = Buffer::with_layout(8, cs, BufferLayout::ELASTIC).unwrap();
    assert_eq!(b.buf.len(), 1024);
    b.set_offset(Bin(0));
    let chunk = |o: u64| vec![o as u8 + 1; cs];
    for o in 0..8u64 {
        b.consume(Bin::new(0, o), &chunk(o)).unwrap();
    }
    assert_eq!(b.buf.len(), 8 * cs);
    // chunks at or past the window are dropped, never stored over chunk 0
    b.consume(Bin::new(0, 8), &chunk(8)).unwrap();
    b.consume(Bin::new(0, 9), &chunk(9)).unwrap();
    assert_eq!(b.tail(), Bin(0));
    let mut r = BufferReader::new(&b);
    let want: Vec<u8> = (0..8).flat_map(chunk).collect();
    assert_eq!(read_n(&mut r, &b, 100_000), want);

    let cache = b.export_cache().unwrap();
    let mut c = Buffer::with_layout(8, cs, BufferLayout::ELASTIC).unwrap();
    c.import_cache(&cache[..3 * cs]).unwrap();
    assert_eq!(c.buf.len(), 3 * cs);
    assert_eq!(c.export_cache().unwrap(), cache[..3 * cs]);
}

/// The chunk at `off` of the codecs' stream: 8 bytes of its own, then a
/// byte per offset.
fn test_chunk(off: u64, cs: usize) -> Vec<u8> {
    let own = (off / cs as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut c: Vec<u8> = (off..off + cs as u64)
        .map(|o| (o.wrapping_mul(31) ^ (o >> 8)) as u8)
        .collect();
    c[..8].copy_from_slice(&own.to_le_bytes());
    c
}

/// The chunks of `b` in the codecs' stream.
fn stream(b: Bin, cs: usize) -> Vec<u8> {
    let first = bin_byte(b.base_left(), cs) as u64;
    (0..b.base_length())
        .flat_map(|k| test_chunk(first + k * cs as u64, cs))
        .collect()
}

/// Makes each chunk from its offset alone.
#[derive(Debug)]
struct OffsetCodec(usize);

impl ChunkCodec for OffsetCodec {
    fn state_len(&self) -> usize {
        0
    }

    fn encode(&self, _: u64, _: &[u8], _: &mut [u8]) {
        panic!("a codec without state encodes nothing");
    }

    fn decode(&self, offset: u64, state: &[u8], out: &mut [u8]) {
        assert!(state.is_empty());
        out.copy_from_slice(&test_chunk(offset, self.0));
    }
}

/// Keeps each chunk's own 8 bytes, and makes the rest from the offset.
#[derive(Debug)]
struct HeadCodec(usize);

impl ChunkCodec for HeadCodec {
    fn state_len(&self) -> usize {
        8
    }

    fn encode(&self, offset: u64, chunk: &[u8], state: &mut [u8]) {
        assert_eq!(offset % self.0 as u64, 0);
        state.copy_from_slice(&chunk[..8]);
    }

    fn decode(&self, offset: u64, state: &[u8], out: &mut [u8]) {
        out[8..].copy_from_slice(&test_chunk(offset, self.0)[8..]);
        out[..8].copy_from_slice(state);
    }
}

/// Every read until `WouldBlock`, `size` bytes at most, recovering from
/// underruns.
fn drain(r: &mut BufferReader, b: &mut Buffer, size: usize) -> Vec<Result<Vec<u8>, Error>> {
    let mut out = vec![];
    let mut p = vec![0u8; size];
    loop {
        match r.read(b, &mut p) {
            Ok(n) => out.push(Ok(p[..n].to_vec())),
            Err(Error::WouldBlock) => return out,
            Err(Error::BufferUnderrun) => {
                out.push(Err(Error::BufferUnderrun));
                match r.recover(b) {
                    Ok(skipped) => out.push(Ok(skipped.to_le_bytes().to_vec())),
                    Err(e) => {
                        out.push(Err(e));
                        return out;
                    }
                }
            }
            Err(e) => {
                out.push(Err(e));
                return out;
            }
        }
    }
}

/// Every `Data` message's bin and bytes.
#[derive(Default)]
struct DataRecorder(Vec<(Bin, Vec<u8>)>);

impl MessageWriter for DataRecorder {
    fn write(&mut self, m: &Message<'_>) -> Result<usize, codec::Error> {
        if let Message::Data(d) = m {
            self.0.push((d.bin, d.data.to_vec()));
        }
        Ok(m.encoded_len())
    }
}

/// A buffer that keeps its chunks' codec states reads, sends and caches
/// what one that stores the chunks does, with state and without, in either
/// layout: out of order, in bins of several chunks, past the window and
/// through an underrun, in reads that start and end mid-chunk, and from an
/// imported cache. It holds no bytes, and states only for a codec that has
/// them.
#[test]
fn codec_matches_stored() {
    let cs = 16;
    let writes = [
        Bin::new(0, 1),
        Bin::new(0, 0),
        Bin::new(0, 3),
        Bin::new(0, 2),
        Bin::new(1, 2),
        Bin::new(0, 7),
        Bin::new(0, 6),
        Bin::new(0, 9),
        Bin::new(0, 8),
        Bin::new(2, 4),
        Bin::new(0, 21),
    ];
    let sends: Vec<Bin> = (0..24)
        .map(|o| Bin::new(0, o))
        .chain([Bin::new(1, 2), Bin::new(2, 1), Bin::new(2, 4)])
        .collect();
    let codecs: [Arc<dyn ChunkCodec>; 2] = [Arc::new(OffsetCodec(cs)), Arc::new(HeadCodec(cs))];
    for codec in codecs {
        for layout in [BufferLayout::CIRCULAR, BufferLayout::ELASTIC] {
            let coded = || {
                Buffer::with_chunk_bytes(8, cs, layout, ChunkBytes::Codec(codec.clone())).unwrap()
            };
            let mut stored = Buffer::with_layout(8, cs, layout).unwrap();
            let mut coding = coded();
            stored.set_offset(Bin(0));
            coding.set_offset(Bin(0));
            // one reader in pieces of a third of a chunk, one in two and a half
            let mut readers: Vec<(usize, BufferReader, BufferReader)> = [5, 40]
                .into_iter()
                .map(|n| (n, BufferReader::new(&stored), BufferReader::new(&coding)))
                .collect();
            let (mut caches, mut underruns) = (0, 0);
            for &b in &writes {
                stored.consume(b, &stream(b, cs)).unwrap();
                coding.consume(b, &stream(b, cs)).unwrap();
                for (n, rs, rc) in &mut readers {
                    let reads = drain(rs, &mut stored, *n);
                    assert_eq!(
                        drain(rc, &mut coding, *n),
                        reads,
                        "{codec:?} {layout:?} {b:?}"
                    );
                    assert_eq!(rc.position(), rs.position());
                    underruns += reads
                        .iter()
                        .filter(|r| **r == Err(Error::BufferUnderrun))
                        .count();
                }

                let (mut ws, mut wc) = (DataRecorder::default(), DataRecorder::default());
                for &s in &sends {
                    assert_eq!(
                        coding.write_data(s, Time::EPOCH, &mut wc),
                        stored.write_data(s, Time::EPOCH, &mut ws),
                    );
                }
                assert_eq!(wc.0, ws.0, "{codec:?} {layout:?} {b:?}");
                assert!(ws.0.iter().all(|(s, d)| *d == stream(*s, cs)));

                let cache = stored.export_cache();
                assert_eq!(coding.export_cache(), cache);
                if let Ok(cache) = cache {
                    caches += 1;
                    let mut stored = Buffer::with_layout(8, cs, layout).unwrap();
                    let mut coding = coded();
                    // a trailing partial chunk is stored but never read
                    let partial = match cache.len() < 8 * cs {
                        true => [&cache[..], &[7; 5]].concat(),
                        false => cache.clone(),
                    };
                    stored.import_cache(&partial).unwrap();
                    coding.import_cache(&partial).unwrap();
                    let mut rs = BufferReader::new(&stored);
                    let mut rc = BufferReader::new(&coding);
                    assert_eq!(
                        drain(&mut rc, &mut coding, 40),
                        drain(&mut rs, &mut stored, 40)
                    );
                    assert_eq!(coding.export_cache(), stored.export_cache());
                }
            }
            assert!(caches > 0);
            assert_eq!(underruns > 0, layout == BufferLayout::CIRCULAR);
            assert!(coding.buf.is_empty());
            let states = codec.state_len() * 8;
            assert_eq!(coding.states.len(), states, "{codec:?} {layout:?}");
        }
    }
}

/// The bytes of chunk `o` in the rewind tests: its index, then a count.
fn chunk_bytes(o: u64, cs: usize) -> Vec<u8> {
    (0..cs)
        .map(|i| (o as u8).wrapping_mul(16).wrapping_add(i as u8))
        .collect()
}

fn stream_bytes(chunks: core::ops::Range<u64>, cs: usize) -> Vec<u8> {
    chunks.flat_map(|o| chunk_bytes(o, cs)).collect()
}

/// A joiner's buffer of 16 chunks of 8 bytes: chunk 13 came first, then
/// chunks 20..24, the read offset is at 20 (the window starts there), and
/// the reader has read to the end.
fn joined() -> (Buffer, BufferReader, Map) {
    let cs = 8;
    let mut b = Buffer::new(16, cs).unwrap();
    let mut had = Map::new();
    for o in [13].into_iter().chain(20..24) {
        b.consume(Bin::new(0, o), &chunk_bytes(o, cs)).unwrap();
        had.set(Bin::new(0, o));
    }
    b.set_offset(Bin::new(0, 23));
    assert_eq!(b.tail(), Bin::new(0, 20));
    let mut r = BufferReader::new(&b);
    assert_eq!(read_n(&mut r, &b, 1000), stream_bytes(20..24, cs));
    (b, r, had)
}

/// A rewind within the window moves only its reader, to the start of the
/// offset's chunk, and the bytes from there come again in order. An
/// offset past the reader's position or before the tail stays refused.
#[test]
fn a_rewind_moves_its_reader_alone_to_the_chunk_start() {
    let cs = 8;
    let mut b = Buffer::new(16, cs).unwrap();
    let mut unready = BufferReader::new(&b);
    assert_eq!(
        unready.rewind(&b, 0),
        Err(RewindError::Read(Error::WouldBlock))
    );
    b.set_offset(Bin(0));
    for o in 0..10 {
        b.consume(Bin::new(0, o), &chunk_bytes(o, cs)).unwrap();
    }
    let mut r = BufferReader::new(&b);
    let mut other = BufferReader::new(&b);
    assert_eq!(read_n(&mut r, &b, 1000), stream_bytes(0..10, cs));
    assert_eq!(read_n(&mut other, &b, 37).len(), 37);

    assert_eq!(r.rewind(&b, 81), Err(RewindError::Ahead));
    assert_eq!(r.rewind(&b, 3 * 8 + 5), Ok(24));
    assert_eq!(r.position(), 24);
    assert_eq!(read_n(&mut r, &b, 1000), stream_bytes(3..10, cs));
    assert_eq!(other.position(), 37, "the other reader stays");
    assert_eq!(read_n(&mut other, &b, 1000), stream_bytes(0..10, cs)[37..]);
    // the reader's own position is the furthest it may go
    assert_eq!(r.rewind(&b, 80), Ok(80));

    // once the window slides on, chunks before its tail are out of reach
    for o in 10..40 {
        b.consume(Bin::new(0, o), &chunk_bytes(o, cs)).unwrap();
    }
    let mut r = BufferReader::new(&b);
    assert_eq!(b.tail(), Bin::new(0, 24));
    read_n(&mut r, &b, 10_000);
    assert_eq!(r.rewind(&b, 23 * 8), Err(RewindError::OutsideWindow));
    assert_eq!(r.rewind(&b, 24 * 8), Ok(24 * 8));
    assert_eq!(
        b.open_backlog(Bin::new(0, 23), &Map::new()),
        Err(RewindError::OutsideWindow),
        "a sliding window's backlog shares its slots"
    );
}

/// A reader whose next read fails keeps that error for the read.
#[test]
fn a_rewind_leaves_a_pending_error_to_the_next_read() {
    let (mut b, mut r, _) = joined();
    b.reset();
    assert_eq!(
        r.rewind(&b, 160),
        Err(RewindError::Read(Error::StreamReset))
    );
    assert_eq!(r.read(&b, &mut [0; 8]), Err(Error::StreamReset));
}

/// A backlog before the tail stores its chunks while the tail, `next` and
/// every reader stay, new readers included. Chunks stored before the
/// window moved past them count as stored. Once all are stored the window
/// starts at the backlog, and a reader moves back into it.
#[test]
fn a_backlog_becomes_the_window_once_it_is_whole() {
    let cs = 8;
    let (mut b, mut r, had) = joined();
    let seq = b.readable_seq();
    b.open_backlog(Bin::new(0, 12), &had).unwrap();
    assert_eq!(b.backlog(), Some((Bin::new(0, 12), Bin::new(0, 20))));
    assert_eq!(b.store_from(), Bin::new(0, 12));
    assert_eq!(b.tail(), Bin::new(0, 20));
    assert!(b.bins_filled_at(Bin::new(0, 13)));
    assert!(!b.bins_filled_at(Bin::new(0, 12)));
    assert!(b.bins_filled_at(Bin::new(0, 11)), "before the backlog");
    assert_eq!(b.readable_seq(), seq);

    for o in [12, 14, 15, 16, 17, 18] {
        b.consume(Bin::new(0, o), &chunk_bytes(o, cs)).unwrap();
    }
    assert_eq!(b.tail(), Bin::new(0, 20));
    assert_eq!(b.next(), Bin::new(0, 24));
    assert_eq!(r.rewind(&b, 12 * 8), Err(RewindError::OutsideWindow));
    let mut fresh = BufferReader::new(&b);
    assert_eq!(fresh.position(), 20 * 8, "new readers start at the tail");
    assert_eq!(b.readable_seq(), seq, "nothing readable changed");

    b.consume(Bin::new(0, 19), &chunk_bytes(19, cs)).unwrap();
    assert_eq!(b.backlog(), None);
    assert_eq!(b.tail(), Bin::new(0, 12));
    assert_ne!(b.readable_seq(), seq);
    assert_eq!(r.rewind(&b, 12 * 8 + 7), Ok(12 * 8));
    assert_eq!(read_n(&mut r, &b, 1000), stream_bytes(12..24, cs));
    assert_eq!(
        read_n(&mut fresh, &b, 1000),
        stream_bytes(20..24, cs),
        "a reader opened before stays"
    );

    // the window goes on from there
    for o in 24..28 {
        b.consume(Bin::new(0, o), &chunk_bytes(o, cs)).unwrap();
    }
    assert_eq!(b.tail(), Bin::new(0, 12));
    assert_eq!(read_n(&mut r, &b, 1000), stream_bytes(24..28, cs));
    b.consume(Bin::new(0, 28), &chunk_bytes(28, cs)).unwrap();
    assert_eq!(b.tail(), Bin::new(0, 13));
}

/// A backlog extends down to an earlier chunk, and one its chunks already
/// fill is whole at once.
#[test]
fn a_backlog_extends_and_completes_from_what_the_buffer_had() {
    let cs = 8;
    let (mut b, _, had) = joined();
    b.open_backlog(Bin::new(0, 16), &had).unwrap();
    b.consume(Bin::new(0, 17), &chunk_bytes(17, cs)).unwrap();
    b.open_backlog(Bin::new(0, 18), &had).unwrap();
    assert_eq!(b.backlog(), Some((Bin::new(0, 16), Bin::new(0, 20))));
    b.open_backlog(Bin::new(0, 14), &had).unwrap();
    assert_eq!(b.backlog(), Some((Bin::new(0, 14), Bin::new(0, 20))));
    assert!(
        b.bins_filled_at(Bin::new(0, 17)),
        "kept across the extension"
    );
    assert!(!b.bins_filled_at(Bin::new(0, 14)));

    let (mut b, mut r, had) = joined();
    b.open_backlog(Bin::new(0, 13), &had).unwrap();
    for o in 14..20 {
        assert!(b.backlog().is_some());
        b.consume(Bin::new(0, o), &chunk_bytes(o, cs)).unwrap();
    }
    assert_eq!(b.tail(), Bin::new(0, 13));
    assert_eq!(r.rewind(&b, 13 * 8), Ok(13 * 8));
    assert_eq!(read_n(&mut r, &b, 1000), stream_bytes(13..24, cs));

    let mut whole = had.clone();
    whole.set(Bin::new(1, 7)); // chunks 14 and 15
    let (mut b, _, _) = joined();
    b.open_backlog(Bin::new(0, 14), &whole).unwrap();
    b.consume(Bin::new(2, 4), &stream_bytes(16..20, cs))
        .unwrap();
    assert_eq!(b.tail(), Bin::new(0, 14));
}

/// A backlog that stops fitting the ring, or whose window moved on, is
/// closed: its chunks count as before the window again and readers stay.
#[test]
fn a_backlog_that_stops_fitting_is_closed() {
    let cs = 8;
    // a backlog a window or more behind the newest chunk never fits
    let (mut b, _, had) = joined();
    assert_eq!(
        b.open_backlog(Bin::new(0, 7), &had),
        Err(RewindError::OutsideWindow)
    );
    assert_eq!(b.backlog(), None);
    b.open_backlog(Bin::new(0, 8), &had).unwrap();

    // the chunk sharing chunk 8's slot closes it
    let seq = b.readable_seq();
    b.consume(Bin::new(0, 9), &chunk_bytes(9, cs)).unwrap();
    b.consume(Bin::new(0, 24), &chunk_bytes(24, cs)).unwrap();
    assert_eq!(b.backlog(), None);
    assert_ne!(b.readable_seq(), seq);
    assert!(b.bins_filled_at(Bin::new(0, 10)));
    assert_eq!(b.tail(), Bin::new(0, 20));
    assert_eq!(b.store_from(), Bin::new(0, 20));

    // a reset, a recovery, a close, and a new read offset drop it too
    for end in 0..4 {
        let (mut b, mut r, had) = joined();
        b.open_backlog(Bin::new(0, 12), &had).unwrap();
        match end {
            0 => b.reset(),
            1 => {
                b.consume(Bin::new(0, 40), &chunk_bytes(40, cs)).unwrap();
                assert_eq!(b.backlog(), None);
                assert_eq!(r.read(&b, &mut [0; 8]), Err(Error::BufferUnderrun));
                r.recover(&mut b).unwrap();
            }
            2 => b.close(),
            _ => b.set_offset(Bin::new(0, 21)),
        }
        assert_eq!(b.backlog(), None, "{end}");
    }
}
