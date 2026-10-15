// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Buffer and reader tests: write orders, readers, recovery, and the
//! window's bounds.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use strims_base::time::Time;
use strims_binmap::Bin;

use super::*;
use crate::codec::{self, Message, MessageWriter};

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
