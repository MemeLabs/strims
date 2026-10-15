// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Swarm ids.
//!
//! A swarm id is the swarm's public key, written as unpadded standard
//! base32 (RFC 4648). Ids travel in URIs, so decoding accepts exactly what
//! the golden vectors pin: CR and LF are ignored, and a dangling group of 1,
//! 3 or 6 characters at the end is dropped without an error.

use core::fmt;

/// The id of a swarm: its public key. Ordered bytewise.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SwarmId(Vec<u8>);

/// A string that isn't unpadded base32.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("illegal base32 data at input byte {0}")]
pub struct SwarmIdError(pub usize);

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn decode_char(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'2'..=b'7' => Some(c - b'2' + 26),
        _ => None,
    }
}

impl SwarmId {
    /// A copy of `key`.
    pub fn new(key: &[u8]) -> SwarmId {
        SwarmId(key.to_vec())
    }

    /// Parses unpadded base32 (see the module docs).
    pub fn decode(s: &str) -> Result<SwarmId, SwarmIdError> {
        let mut out = Vec::with_capacity(s.len() * 5 / 8);
        let mut quantum = [0u8; 8];
        let mut j = 0;
        let src = s
            .bytes()
            .enumerate()
            .filter(|&(_, c)| c != b'\r' && c != b'\n');
        for (i, c) in src {
            let v = decode_char(c).ok_or(SwarmIdError(i))?;
            if let Some(q) = quantum.get_mut(j) {
                *q = v;
            }
            j += 1;
            if j == 8 {
                push_quantum(&mut out, &quantum, 8);
                j = 0;
            }
        }
        // only 2, 4, 5 and 7 character tails carry whole bytes; 1, 3 and 6
        // yield nothing
        push_quantum(&mut out, &quantum, j);
        Ok(SwarmId(out))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

/// Packs `dlen` decoded characters of a quantum into bytes.
fn push_quantum(out: &mut Vec<u8>, d: &[u8; 8], dlen: usize) {
    let [d0, d1, d2, d3, d4, d5, d6, d7] = *d;
    let bytes = [
        d0 << 3 | d1 >> 2,
        d1 << 6 | d2 << 1 | d3 >> 4,
        d3 << 4 | d4 >> 1,
        d4 << 7 | d5 << 2 | d6 >> 3,
        d6 << 5 | d7,
    ];
    let n = match dlen {
        8 => 5,
        7 => 4,
        5 => 3,
        4 => 2,
        2 => 1,
        _ => 0,
    };
    out.extend_from_slice(bytes.get(..n).unwrap_or_default());
}

impl From<&[u8]> for SwarmId {
    fn from(b: &[u8]) -> SwarmId {
        SwarmId::new(b)
    }
}

impl AsRef<[u8]> for SwarmId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Unpadded standard base32.
impl fmt::Display for SwarmId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.0.chunks(5) {
            let mut b = [0u8; 5];
            if let Some(dst) = b.get_mut(..chunk.len()) {
                dst.copy_from_slice(chunk);
            }
            let v = u64::from_be_bytes([0, 0, 0, b[0], b[1], b[2], b[3], b[4]]);
            let chars = (chunk.len() * 8).div_ceil(5);
            for i in 0..chars {
                let idx = (v >> (35 - 5 * i)) & 31;
                let c = ALPHABET.get(idx as usize).copied().unwrap_or(b'A');
                fmt::Write::write_char(f, char::from(c))?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for SwarmId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SwarmId({self})")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn base32_round_trip() {
        for n in 0..40u8 {
            let id = SwarmId::new(&(0..n).map(|i| i.wrapping_mul(37)).collect::<Vec<_>>());
            let s = id.to_string();
            assert!(!s.contains('='));
            assert_eq!(SwarmId::decode(&s).unwrap(), id);
        }
    }

    #[test]
    fn decode_ignores_line_breaks_and_dangling_tails() {
        assert_eq!(SwarmId::decode("AE").unwrap().as_bytes(), [1]);
        // a 3 character tail packs nothing, not even its first byte
        assert_eq!(SwarmId::decode("AEB").unwrap().as_bytes(), [] as [u8; 0]);
        assert_eq!(SwarmId::decode("AE\r\nBA").unwrap().as_bytes(), [1, 2]);
        assert_eq!(SwarmId::decode("ae"), Err(SwarmIdError(0)));
        assert_eq!(SwarmId::decode("AE=="), Err(SwarmIdError(2)));
        assert_eq!(SwarmId::decode("").unwrap().as_bytes(), [] as [u8; 0]);
    }
}
