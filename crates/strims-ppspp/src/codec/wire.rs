// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Primitive encoders and decoders: LEB128 varints (zigzag for signed
//! values), big-endian integers, and millisecond timestamps.

use strims_base::time::{Duration, Time};

use super::Error;

/// The encoded length of `v` as an unsigned varint.
pub const fn uvarint_len(v: u64) -> usize {
    if v == 0 {
        return 1;
    }
    (64 - v.leading_zeros() as usize).div_ceil(7)
}

/// The encoded length of `v` as a signed (zigzag) varint.
pub const fn varint_len(v: i64) -> usize {
    uvarint_len(zigzag(v))
}

const fn zigzag(v: i64) -> u64 {
    let uv = (v as u64) << 1;
    if v < 0 { !uv } else { uv }
}

/// Times go on the wire as milliseconds since the Unix epoch, truncated.
pub(crate) const fn time_to_wire(t: Time) -> i64 {
    t.unix_nanos() / 1_000_000
}

/// `ms * 1e6` nanoseconds, wrapping on overflow. The wrap is part of the
/// v2 encoding: signatures cover the decoded nanosecond value, so every
/// decoder must arrive at the same one.
pub(crate) const fn time_from_wire(ms: i64) -> Time {
    Time::from_unix_nanos(ms.wrapping_mul(1_000_000))
}

pub(crate) const fn duration_to_wire(d: Duration) -> i64 {
    d.as_nanos() / 1_000_000
}

pub(crate) const fn duration_from_wire(ms: i64) -> Duration {
    Duration::from_nanos(ms.wrapping_mul(1_000_000))
}

/// A bounds-checked cursor over an input buffer.
pub(crate) struct Dec<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Dec<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Dec<'a> {
        Dec { buf, pos: 0 }
    }

    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    pub(crate) fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub(crate) fn bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::Truncated)?;
        let b = self.buf.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(b)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let b = self.bytes(N)?;
        b.try_into().map_err(|_| Error::Truncated)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    /// An unsigned varint of at most 10 bytes. Non-minimal encodings are
    /// valid v2 input and decode.
    pub(crate) fn uvarint(&mut self) -> Result<u64, Error> {
        let mut x: u64 = 0;
        let mut s: u32 = 0;
        for i in 0..10 {
            let b = *self
                .buf
                .get(self.pos.saturating_add(i))
                .ok_or(Error::Truncated)?;
            if b < 0x80 {
                if i == 9 && b > 1 {
                    return Err(Error::VarintOverflow);
                }
                self.pos += i + 1;
                return Ok(x | u64::from(b) << s);
            }
            x |= u64::from(b & 0x7f) << s;
            s += 7;
        }
        Err(Error::VarintOverflow)
    }

    /// A zigzag-encoded signed varint.
    pub(crate) fn varint(&mut self) -> Result<i64, Error> {
        let ux = self.uvarint()?;
        let x = (ux >> 1) as i64;
        Ok(if ux & 1 != 0 { !x } else { x })
    }

    /// The unread input, without consuming it.
    pub(crate) fn rest_peek(&self) -> &'a [u8] {
        self.buf.get(self.pos..).unwrap_or_default()
    }
}

/// A bounds-checked cursor over an output buffer.
pub(crate) struct Enc<'b> {
    buf: &'b mut [u8],
    pos: usize,
}

impl<'b> Enc<'b> {
    pub(crate) fn new(buf: &'b mut [u8]) -> Enc<'b> {
        Enc { buf, pos: 0 }
    }

    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    pub(crate) fn bytes(&mut self, b: &[u8]) -> Result<(), Error> {
        let end = self.pos.checked_add(b.len()).ok_or(Error::NotEnoughSpace)?;
        self.buf
            .get_mut(self.pos..end)
            .ok_or(Error::NotEnoughSpace)?
            .copy_from_slice(b);
        self.pos = end;
        Ok(())
    }

    pub(crate) fn u8(&mut self, v: u8) -> Result<(), Error> {
        self.bytes(&[v])
    }

    pub(crate) fn u16(&mut self, v: u16) -> Result<(), Error> {
        self.bytes(&v.to_be_bytes())
    }

    pub(crate) fn u32(&mut self, v: u32) -> Result<(), Error> {
        self.bytes(&v.to_be_bytes())
    }

    pub(crate) fn u64(&mut self, v: u64) -> Result<(), Error> {
        self.bytes(&v.to_be_bytes())
    }

    /// Writes the minimal encoding of `v`.
    pub(crate) fn uvarint(&mut self, mut v: u64) -> Result<(), Error> {
        let mut tmp = [0u8; 10];
        let mut n = 0;
        while v >= 0x80 {
            if let Some(t) = tmp.get_mut(n) {
                *t = (v as u8) | 0x80;
            }
            v >>= 7;
            n += 1;
        }
        if let Some(t) = tmp.get_mut(n) {
            *t = v as u8;
        }
        self.bytes(tmp.get(..=n).unwrap_or_default())
    }

    /// Writes the minimal zigzag encoding of `v`.
    pub(crate) fn varint(&mut self, v: i64) -> Result<(), Error> {
        self.uvarint(zigzag(v))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn varint_lengths_match_encoding() {
        for v in [0u64, 1, 127, 128, 16383, 16384, u64::MAX >> 1, u64::MAX] {
            let mut buf = [0u8; 10];
            let mut e = Enc::new(&mut buf);
            e.uvarint(v).unwrap();
            assert_eq!(e.pos(), uvarint_len(v), "{v}");
            assert_eq!(Dec::new(&buf).uvarint(), Ok(v));
        }
        for v in [0i64, 1, -1, 63, -64, 64, -65, i64::MAX, i64::MIN] {
            let mut buf = [0u8; 10];
            let mut e = Enc::new(&mut buf);
            e.varint(v).unwrap();
            assert_eq!(e.pos(), varint_len(v), "{v}");
            assert_eq!(Dec::new(&buf).varint(), Ok(v));
        }
    }

    #[test]
    fn uvarint_rejects_truncated_and_overflowing_input() {
        assert_eq!(Dec::new(&[]).uvarint(), Err(Error::Truncated));
        assert_eq!(Dec::new(&[0x80]).uvarint(), Err(Error::Truncated));
        // 10th byte > 1 overflows; an 11th continuation byte overflows.
        let mut b = [0xffu8; 10];
        b[9] = 0x02;
        assert_eq!(Dec::new(&b).uvarint(), Err(Error::VarintOverflow));
        assert_eq!(Dec::new(&[0xff; 11]).uvarint(), Err(Error::VarintOverflow));
        b[9] = 0x01;
        assert_eq!(Dec::new(&b).uvarint(), Ok(u64::MAX));
        // non-minimal encodings are accepted
        assert_eq!(Dec::new(&[0x80, 0x00]).uvarint(), Ok(0));
    }
}
