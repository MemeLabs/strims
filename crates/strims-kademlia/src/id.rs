// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! 256 bit Kademlia ids.
//!
//! An [`ID`] is four `u64` words in big-endian word order. The byte (wire)
//! form is the words written big-endian, so byte order, word order and
//! [`Ord`] all agree.

use core::fmt;
use core::ops::BitXor;

use crate::Error;

/// Byte length of an [`ID`].
pub const ID_LENGTH: usize = 32;

/// Bit length of an [`ID`].
pub const ID_BIT_LENGTH: usize = ID_LENGTH * 8;

/// Byte length of [`ID::marshal_json`] output: two quotes around 44 base64
/// characters.
pub const ID_JSON_LENGTH: usize = 46;

/// A 256 bit Kademlia id.
///
/// The derived [`Ord`] compares words lexicographically, which is also
/// big-endian byte order.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ID(pub [u64; 4]);

impl ID {
    /// The all-zero id.
    pub const MIN: ID = ID([0; 4]);
    /// The all-ones id.
    pub const MAX: ID = ID([u64::MAX; 4]);

    /// Builds an id from 32 bytes, big-endian per word.
    pub fn from_bytes(b: &[u8; ID_LENGTH]) -> ID {
        let mut d = [0u64; 4];
        for (w, chunk) in d.iter_mut().zip(b.as_chunks::<8>().0) {
            *w = u64::from_be_bytes(*chunk);
        }
        ID(d)
    }

    /// Returns the 32 byte big-endian form.
    pub fn to_bytes(&self) -> [u8; ID_LENGTH] {
        let mut b = [0u8; ID_LENGTH];
        for (chunk, w) in b.as_chunks_mut::<8>().0.iter_mut().zip(self.0) {
            *chunk = w.to_be_bytes();
        }
        b
    }

    /// Reads an id from the first 32 bytes of `b` and returns it with the
    /// number of bytes read.
    pub fn unmarshal(b: &[u8]) -> Result<(ID, usize), Error> {
        let src: &[u8; ID_LENGTH] = b
            .get(..ID_LENGTH)
            .and_then(|s| s.try_into().ok())
            .ok_or(Error::BufferTooShort)?;
        Ok((ID::from_bytes(src), ID_LENGTH))
    }

    /// Writes the 32 byte form into the start of `b`.
    pub fn marshal(&self, b: &mut [u8]) -> Result<usize, Error> {
        let dst = b.get_mut(..ID_LENGTH).ok_or(Error::BufferTooShort)?;
        dst.copy_from_slice(&self.to_bytes());
        Ok(ID_LENGTH)
    }

    /// XOR distance.
    pub fn xor(&self, o: &ID) -> ID {
        let [a0, a1, a2, a3] = self.0;
        let [b0, b1, b2, b3] = o.0;
        ID([a0 ^ b0, a1 ^ b1, a2 ^ b2, a3 ^ b3])
    }

    /// Same as `self < o`.
    pub fn less(&self, o: &ID) -> bool {
        self < o
    }

    /// Number of leading zero bits, 256 for [`ID::MIN`].
    pub fn leading_zeros(&self) -> u32 {
        let mut n = 0;
        for w in self.0 {
            if w != 0 {
                return n + w.leading_zeros();
            }
            n += 64;
        }
        n
    }

    /// The bytes as a quoted, padded standard base64 string.
    pub fn marshal_json(&self) -> [u8; ID_JSON_LENGTH] {
        let mut out = [b'"'; ID_JSON_LENGTH];
        // invariant: 32 bytes encode to exactly 44 base64 characters, which
        // fill out[1..45].
        #[allow(clippy::indexing_slicing)]
        base64_encode(&self.to_bytes(), &mut out[1..ID_JSON_LENGTH - 1]);
        out
    }

    /// Parses [`ID::marshal_json`] output. Only the canonical encoding is
    /// accepted.
    pub fn unmarshal_json(j: &[u8]) -> Result<ID, Error> {
        let inner = j
            .strip_prefix(b"\"")
            .and_then(|j| j.strip_suffix(b"\""))
            .ok_or(Error::InvalidJson)?;
        let mut b = [0u8; ID_LENGTH];
        base64_decode(inner, &mut b)?;
        Ok(ID::from_bytes(&b))
    }
}

impl BitXor for ID {
    type Output = ID;
    fn bitxor(self, o: ID) -> ID {
        self.xor(&o)
    }
}

impl From<[u8; ID_LENGTH]> for ID {
    fn from(b: [u8; ID_LENGTH]) -> ID {
        ID::from_bytes(&b)
    }
}

impl From<ID> for [u8; ID_LENGTH] {
    fn from(id: ID) -> [u8; ID_LENGTH] {
        id.to_bytes()
    }
}

impl TryFrom<&[u8]> for ID {
    type Error = Error;
    /// Requires exactly 32 bytes; use [`ID::unmarshal`] to read a prefix.
    fn try_from(b: &[u8]) -> Result<ID, Error> {
        let b: &[u8; ID_LENGTH] = b.try_into().map_err(|_| Error::InvalidLength)?;
        Ok(ID::from_bytes(b))
    }
}

/// Lowercase hex of the byte form.
impl fmt::Display for ID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for w in self.0 {
            write!(f, "{w:016x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ID({self})")
    }
}

const BASE64_STD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Padded standard base64. `dst` must hold
/// exactly `4 * ceil(src.len() / 3)` bytes.
fn base64_encode(src: &[u8], dst: &mut [u8]) {
    let sym = |v: u32| BASE64_STD.get((v & 0x3f) as usize).copied().unwrap_or(b'=');
    for (chunk, out) in src.chunks(3).zip(dst.chunks_mut(4)) {
        let b0 = chunk.first().copied().unwrap_or(0) as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let v = (b0 << 16) | (b1 << 8) | b2;
        let syms = [
            sym(v >> 18),
            sym(v >> 12),
            if chunk.len() > 1 { sym(v >> 6) } else { b'=' },
            if chunk.len() > 2 { sym(v) } else { b'=' },
        ];
        for (o, s) in out.iter_mut().zip(syms) {
            *o = s;
        }
    }
}

/// Strict padded standard base64 decode into exactly `dst.len()` bytes.
fn base64_decode(src: &[u8], dst: &mut [u8]) -> Result<(), Error> {
    if src.len() != dst.len().div_ceil(3) * 4 {
        return Err(Error::InvalidJson);
    }
    let val = |c: u8| -> Result<u32, Error> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(Error::InvalidJson),
        }
    };
    for (quad, out) in src.as_chunks::<4>().0.iter().zip(dst.chunks_mut(3)) {
        // `out` is shorter than 3 only for the final quantum, which must be
        // padded to match.
        let pad = 3 - out.len();
        let mut v = 0u32;
        for (i, &c) in quad.iter().enumerate() {
            v <<= 6;
            if i >= 4 - pad {
                if c != b'=' {
                    return Err(Error::InvalidJson);
                }
            } else {
                v |= val(c)?;
            }
        }
        let bytes = [(v >> 16) as u8, (v >> 8) as u8, v as u8];
        // Padding bits must be zero, so each id has exactly one encoding.
        if bytes.iter().skip(out.len()).any(|&b| b != 0) {
            return Err(Error::InvalidJson);
        }
        for (o, b) in out.iter_mut().zip(bytes) {
            *o = b;
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn xor() {
        let a = ID([u64::MAX; 4]);
        let b = ID([0x0000ffff0000ffff; 4]);
        let c = ID([0xffff0000ffff0000; 4]);
        assert_eq!(a.xor(&b), c);
        assert_eq!(a ^ b, c);
    }

    // b0 is sha256("test").
    #[test]
    fn marshal_unmarshal() {
        let b0: [u8; 32] = [
            0x9f, 0x86, 0xd0, 0x81, 0x88, 0x4c, 0x7d, 0x65, 0x9a, 0x2f, 0xea, 0xa0, 0xc5, 0x5a,
            0xd0, 0x15, 0xa3, 0xbf, 0x4f, 0x1b, 0x2b, 0x0b, 0x82, 0x2c, 0xd1, 0x5d, 0x6c, 0x15,
            0xb0, 0xf0, 0x0a, 0x08,
        ];
        let (id0, n) = ID::unmarshal(&b0).unwrap();
        assert_eq!(n, 32);
        let b1 = id0.to_bytes();
        let (id1, _) = ID::unmarshal(&b1).unwrap();
        assert_eq!(id0, id1);
        assert_eq!(b0, b1);
        assert_eq!(id0.0[0], 0x9f86d081884c7d65);
        assert_eq!(
            id0.to_string(),
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }

    #[test]
    fn short_buffers() {
        assert_eq!(ID::unmarshal(&[0; 31]), Err(Error::BufferTooShort));
        assert_eq!(ID::MAX.marshal(&mut [0; 31]), Err(Error::BufferTooShort));
        let mut buf = [0u8; 40];
        assert_eq!(ID::MAX.marshal(&mut buf), Ok(32));
        assert_eq!(ID::unmarshal(&buf), Ok((ID::MAX, 32)));
        assert_eq!(ID::try_from(&buf[..]), Err(Error::InvalidLength));
    }

    #[test]
    fn ordering_matches_byte_order() {
        let a = ID([1, 0, 0, 0]);
        let b = ID([0, u64::MAX, u64::MAX, u64::MAX]);
        assert!(b.less(&a));
        assert!(!a.less(&a));
        assert!(b.to_bytes() < a.to_bytes());
    }

    #[test]
    fn leading_zeros() {
        assert_eq!(ID::MIN.leading_zeros(), 256);
        assert_eq!(ID::MAX.leading_zeros(), 0);
        assert_eq!(ID([0, 0, 1, 0]).leading_zeros(), 128 + 63);
    }

    #[test]
    fn json() {
        let id = ID([1, 2, 3, 4]);
        let j = id.marshal_json();
        assert_eq!(&j, b"\"AAAAAAAAAAEAAAAAAAAAAgAAAAAAAAADAAAAAAAAAAQ=\"");
        assert_eq!(ID::unmarshal_json(&j), Ok(id));
        assert_eq!(ID::unmarshal_json(&j[1..]), Err(Error::InvalidJson));
        assert_eq!(
            ID::unmarshal_json(b"\"AAAAAAAAAAEAAAAAAAAAAgAAAAAAAAADAAAAAAAAAAR=\""),
            Err(Error::InvalidJson),
            "non-canonical trailing bits"
        );
        assert_eq!(
            ID::unmarshal_json(b"\"AAAAAAAAAAEAAAAAAAAAAgAAAAAAAAADAAAAAAAAAAQ\""),
            Err(Error::InvalidJson),
            "missing padding"
        );
    }
}
