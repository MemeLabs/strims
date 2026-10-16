// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Storage keys.
//!
//! Every key starts with its record type's [`Namespace`] in base 36, and
//! further parts follow, each after a `:`. Ids render in base 36, byte
//! strings in unpadded standard base64, and strings as they are. The
//! layout is part of the stored format and must not change:
//!
//! | record | key |
//! |---|---|
//! | singleton | `<ns>` |
//! | table row | `<ns>:<id>` |
//! | index entry | `<ns>:<index_hash(key)>:<id>` |
//! | lease | `<mutex ns>:<name>` |
//!
//! Index keys are hashed with the store key ([`index_hash`]), so keys that
//! name peers or contents don't appear in the store in the clear.

use std::fmt::{self, Write as _};

use blake2::Blake2bMac;
use blake2::digest::consts::U8;
use blake2::digest::{KeyInit, Mac};
use data_encoding::BASE64_NOPAD;

/// A record type's key prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Namespace(pub i64);

/// One part of a key after the namespace.
#[derive(Clone, Copy, Debug)]
pub enum Part<'a> {
    /// Rendered in base 36.
    Id(u64),
    /// Rendered as is.
    Str(&'a str),
    /// Rendered in unpadded standard base64.
    Bytes(&'a [u8]),
}

impl Namespace {
    /// The key of a singleton record.
    pub fn key(self) -> String {
        self.to_string()
    }

    /// The key of the table row `id`.
    pub fn record_key(self, id: u64) -> String {
        self.format(&[Part::Id(id)])
    }

    /// The prefix every row of the table shares.
    pub fn prefix(self) -> String {
        self.format(&[Part::Str("")])
    }

    /// The namespace followed by `parts`.
    pub fn format(self, parts: &[Part<'_>]) -> String {
        let mut s = self.to_string();
        for p in parts {
            s.push(':');
            match *p {
                Part::Id(id) => push_base36(&mut s, id),
                Part::Str(v) => s.push_str(v),
                Part::Bytes(b) => s.push_str(&BASE64_NOPAD.encode(b)),
            }
        }
        s
    }

    /// The prefix of every key that starts with the namespace and `parts`.
    pub fn format_prefix(self, parts: &[Part<'_>]) -> String {
        let mut s = self.format(parts);
        s.push(':');
        s
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 < 0 {
            f.write_char('-')?;
        }
        let mut s = String::new();
        push_base36(&mut s, self.0.unsigned_abs());
        f.write_str(&s)
    }
}

/// `n` in lowercase base 36.
pub fn base36(n: u64) -> String {
    let mut s = String::new();
    push_base36(&mut s, n);
    s
}

/// Parses lowercase base 36, as [`base36`] renders it.
pub fn parse_base36(s: &str) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    let mut n: u64 = 0;
    for c in s.bytes() {
        let d = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'z' => c - b'a' + 10,
            _ => return None,
        };
        n = n.checked_mul(36)?.checked_add(u64::from(d))?;
    }
    Some(n)
}

fn push_base36(s: &mut String, mut n: u64) {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut buf = [0u8; 13];
    let mut i = buf.len();
    loop {
        i -= 1;
        // invariant: n % 36 < 36, and 13 digits hold any u64 in base 36.
        #[allow(clippy::indexing_slicing)]
        {
            buf[i] = DIGITS[(n % 36) as usize];
        }
        n /= 36;
        if n == 0 {
            break;
        }
    }
    for &b in buf.get(i..).unwrap_or_default() {
        s.push(char::from(b));
    }
}

/// Size of the hash [`index_hash`] encodes.
pub const INDEX_HASH_SIZE: usize = 8;

/// The key part an index key maps to: an 8-byte BLAKE2b MAC of `key`
/// under `salt` (truncated to 64 bytes), in unpadded standard base64.
pub fn index_hash(salt: &[u8], key: &[u8]) -> String {
    let salt = salt.get(..salt.len().min(64)).unwrap_or_default();
    let mut mac = match Blake2bMac::<U8>::new_from_slice(salt) {
        Ok(mac) => mac,
        // invariant: BLAKE2b accepts keys of 0 to 64 bytes.
        Err(_) => unreachable!("blake2b key of at most 64 bytes"),
    };
    mac.update(key);
    BASE64_NOPAD.encode(&mac.finalize().into_bytes())
}

/// The key of index `ns`'s entry mapping `key` to record `id`, in a store
/// whose key is `salt`.
pub fn index_entry_key(ns: Namespace, salt: &[u8], key: &[u8], id: u64) -> String {
    ns.format(&[Part::Str(&index_hash(salt, key)), Part::Id(id)])
}

/// The prefix of index `ns`'s entries for `key`, in a store whose key is
/// `salt`.
pub fn index_prefix(ns: Namespace, salt: &[u8], key: &[u8]) -> String {
    ns.format_prefix(&[Part::Str(&index_hash(salt, key))])
}

/// The name of profile `id`'s table.
pub fn profile_table(id: u64) -> String {
    format!("profile:{id}")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn namespaces_render_in_base36() {
        assert_eq!(Namespace(1001).key(), "rt");
        assert_eq!(Namespace(2001).key(), "1jl");
        assert_eq!(Namespace(2003).record_key(36), "1jn:10");
        assert_eq!(Namespace(2003).prefix(), "1jn:");
        assert_eq!(Namespace(-36).key(), "-10");
        assert_eq!(base36(u64::MAX), "3w5e11264sgsf");
        assert_eq!(base36(0), "0");
    }

    #[test]
    fn parts_render_by_kind() {
        let ns = Namespace(1001);
        assert_eq!(
            ns.format(&[Part::Str("dao"), Part::Str("upgrade")]),
            "rt:dao:upgrade"
        );
        assert_eq!(
            ns.format(&[Part::Bytes(&[0xff, 0xfe]), Part::Id(35)]),
            "rt://4:z"
        );
        assert_eq!(ns.format_prefix(&[Part::Str("x")]), "rt:x:");
    }

    #[test]
    fn base36_round_trips() {
        for n in [0, 1, 35, 36, 1295, u64::MAX / 7, u64::MAX] {
            assert_eq!(parse_base36(&base36(n)), Some(n));
        }
        assert_eq!(parse_base36(""), None);
        assert_eq!(parse_base36("A"), None);
        assert_eq!(parse_base36("3w5e11264sgsg"), None);
    }

    #[test]
    fn index_hashes_are_keyed() {
        let a = index_hash(&[1; 32], b"key");
        assert_eq!(a.len(), 11);
        assert_ne!(a, index_hash(&[2; 32], b"key"));
        assert_ne!(a, index_hash(&[1; 32], b"other"));
        assert_eq!(index_hash(&[1; 80], b"key"), index_hash(&[1; 64], b"key"));
    }
}
