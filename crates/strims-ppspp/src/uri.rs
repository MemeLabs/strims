// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Swarm magnet URIs,
//! `magnet:?xt=urn:ppspp:<id>&x.im=2&x.hf=7&x.sa=2&x.cs=1024&x.ps=64&x.sc=1`.
//!
//! URIs pass between nodes, and caches compare them as strings, so the
//! string form and what [`Uri::parse`] accepts must stay as the golden
//! vectors pin them: the scheme is case-insensitive, the query starts at the
//! first `?` and ends at the first `#`, pairs are split on `&` (a pair
//! containing `;`, or with a bad `%` escape, is skipped), keys and values are
//! unescaped with `+` as space, and the first value of a key wins. Control
//! characters and bad escapes in the fragment are errors. The host and path
//! are not validated.

use core::fmt;
use std::collections::BTreeMap;

use crate::codec::ProtocolOptionType;
use crate::integrity::{
    LiveSignatureAlgorithm, MerkleHashTreeFunction, ProtectionMethod, VerifierOptions,
};
use crate::options::SwarmOptions;
use crate::swarm_id::SwarmId;

/// URI errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UriError {
    /// Not a ppspp magnet URI: another scheme, or no `urn:ppspp:` id.
    #[error("invalid uri")]
    InvalidUri,
    /// Not a URL: a control character, a bad scheme, or a bad escape in
    /// the fragment.
    #[error("malformed url")]
    Malformed,
    #[error("invalid swarm id")]
    InvalidSwarmId,
    /// An option value that isn't a decimal below 2^31.
    #[error("invalid option value")]
    InvalidOption,
}

/// The URI keys of the swarm options, in the order the string form writes
/// them.
const PROTOCOL_OPTIONS: [(ProtocolOptionType, &str); 6] = [
    (ProtocolOptionType::ContentIntegrityProtectionMethod, "x.im"),
    (ProtocolOptionType::MerkleHashTreeFunction, "x.hf"),
    (ProtocolOptionType::LiveSignatureAlgorithm, "x.sa"),
    (ProtocolOptionType::ChunkSize, "x.cs"),
    (ProtocolOptionType::ChunksPerSignature, "x.ps"),
    (ProtocolOptionType::StreamCount, "x.sc"),
];

const SCHEME: &str = "magnet";
const URN_PREFIX: &str = "urn:ppspp:";

/// Option values by type.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UriOptions(pub BTreeMap<ProtocolOptionType, u64>);

impl UriOptions {
    pub fn insert(&mut self, t: ProtocolOptionType, v: u64) {
        self.0.insert(t, v);
    }

    pub fn get(&self, t: ProtocolOptionType) -> Option<u64> {
        self.0.get(&t).copied()
    }

    /// The swarm options these values set; missing ones are zero, which
    /// [`crate::Swarm::new`] replaces with defaults. Enum values are
    /// truncated to a byte.
    pub fn swarm_options(&self) -> SwarmOptions {
        let size = |t| usize::try_from(self.get(t).unwrap_or(0)).unwrap_or(usize::MAX);
        let byte = |t| self.get(t).unwrap_or(0) as u8;
        SwarmOptions {
            chunk_size: size(ProtocolOptionType::ChunkSize),
            chunks_per_signature: size(ProtocolOptionType::ChunksPerSignature),
            stream_count: size(ProtocolOptionType::StreamCount),
            integrity: VerifierOptions {
                protection_method: ProtectionMethod(byte(
                    ProtocolOptionType::ContentIntegrityProtectionMethod,
                )),
                merkle_hash_tree_function: MerkleHashTreeFunction(byte(
                    ProtocolOptionType::MerkleHashTreeFunction,
                )),
                live_signature_algorithm: LiveSignatureAlgorithm(byte(
                    ProtocolOptionType::LiveSignatureAlgorithm,
                )),
            },
            ..Default::default()
        }
    }
}

/// A swarm's magnet URI: its id and the options that define it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Uri {
    pub id: SwarmId,
    pub options: UriOptions,
}

impl Uri {
    pub fn new(id: SwarmId, options: UriOptions) -> Uri {
        Uri { id, options }
    }

    /// Parses a magnet URI (see the module docs).
    pub fn parse(s: &str) -> Result<Uri, UriError> {
        let (scheme, query) = split_url(s)?;
        if !scheme.eq_ignore_ascii_case(SCHEME) {
            return Err(UriError::InvalidUri);
        }

        let xt = query_get(query, "xt").map(lossy).unwrap_or_default();
        let id = xt.strip_prefix(URN_PREFIX).ok_or(UriError::InvalidUri)?;
        let id = SwarmId::decode(id).map_err(|_| UriError::InvalidSwarmId)?;

        let mut options = UriOptions::default();
        for (t, key) in PROTOCOL_OPTIONS {
            if let Some(v) = query_get(query, key) {
                options.insert(t, parse_uint31(&lossy(v)).ok_or(UriError::InvalidOption)?);
            }
        }
        Ok(Uri { id, options })
    }
}

/// The string form; see the module docs.
impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{SCHEME}:?xt={URN_PREFIX}{}", self.id)?;
        for (t, key) in PROTOCOL_OPTIONS {
            if let Some(v) = self.options.get(t) {
                write!(f, "&{key}={v}")?;
            }
        }
        Ok(())
    }
}

/// The scheme and raw query of `s`, after the URL checks in the module
/// docs.
fn split_url(s: &str) -> Result<(&str, &str), UriError> {
    let (u, frag) = s.split_once('#').unwrap_or((s, ""));
    if u.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return Err(UriError::Malformed);
    }
    unescape(frag, false).ok_or(UriError::Malformed)?;

    let mut scheme = "";
    let mut rest = u;
    for (i, c) in u.bytes().enumerate() {
        match c {
            b'a'..=b'z' | b'A'..=b'Z' => {}
            b'0'..=b'9' | b'+' | b'-' | b'.' if i != 0 => {}
            b':' if i == 0 => return Err(UriError::Malformed),
            b':' => {
                scheme = u.get(..i).unwrap_or_default();
                rest = u.get(i + 1..).unwrap_or_default();
                break;
            }
            _ => break,
        }
    }

    let query = if rest.ends_with('?') && rest.matches('?').count() == 1 {
        ""
    } else {
        rest.split_once('?').map_or("", |(_, q)| q)
    };
    Ok((scheme, query))
}

/// The first value of `key` in `query`. Pairs containing `;` or a bad
/// escape are skipped.
fn query_get(query: &str, key: &str) -> Option<Vec<u8>> {
    for pair in query.split('&') {
        if pair.is_empty() || pair.contains(';') {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let (Some(k), Some(v)) = (unescape(k, true), unescape(v, true)) else {
            continue;
        };
        if k == key.as_bytes() {
            return Some(v);
        }
    }
    None
}

/// `b` as a string, with invalid UTF-8 replaced.
fn lossy(b: Vec<u8>) -> String {
    String::from_utf8(b).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Unescapes `%XX` escapes (and `+` as space in a query); `None` for a
/// malformed escape.
fn unescape(s: &str, query: bool) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while let Some(&c) = b.get(i) {
        match c {
            b'%' => {
                let hi = (*b.get(i + 1)? as char).to_digit(16)?;
                let lo = (*b.get(i + 2)? as char).to_digit(16)?;
                out.push((hi * 16 + lo) as u8);
                i += 3;
            }
            b'+' if query => {
                out.push(b' ');
                i += 1;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    Some(out)
}

/// A decimal below 2^31: ASCII digits only, leading zeros allowed.
fn parse_uint31(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut v: u64 = 0;
    for c in s.bytes() {
        v = v.checked_mul(10)?.checked_add(u64::from(c - b'0'))?;
        if v >= 1 << 31 {
            return None;
        }
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// A URI round-trips through its string form.
    #[test]
    fn uri() {
        let key = strims_crypto::key_from_seed(&[1; 32]);
        let mut options = UriOptions::default();
        options.insert(
            ProtocolOptionType::ContentIntegrityProtectionMethod,
            u64::from(ProtectionMethod::MERKLE_TREE.0),
        );
        options.insert(
            ProtocolOptionType::MerkleHashTreeFunction,
            u64::from(MerkleHashTreeFunction::BLAKE2B256.0),
        );
        options.insert(
            ProtocolOptionType::LiveSignatureAlgorithm,
            u64::from(LiveSignatureAlgorithm::ED25519.0),
        );
        options.insert(ProtocolOptionType::ChunkSize, 1024);
        options.insert(ProtocolOptionType::ChunksPerSignature, 32);
        options.insert(ProtocolOptionType::StreamCount, 32);
        let uri = Uri::new(SwarmId::new(&key.public), options);

        let uri2 = Uri::parse(&uri.to_string()).unwrap();
        assert_eq!(uri, uri2);

        let o = uri2.options.swarm_options();
        assert_eq!(o.chunk_size, 1024);
        assert_eq!(
            o.integrity.merkle_hash_tree_function,
            MerkleHashTreeFunction::BLAKE2B256
        );
        assert_eq!(o.live_window, 0);
    }

    #[test]
    fn option_values_are_decimals_below_2_31() {
        assert_eq!(parse_uint31("0"), Some(0));
        assert_eq!(parse_uint31("01024"), Some(1024));
        assert_eq!(parse_uint31("2147483647"), Some((1 << 31) - 1));
        assert_eq!(parse_uint31("2147483648"), None);
        assert_eq!(parse_uint31("99999999999999999999999"), None);
        for s in ["", "+1", "-1", "1_0", "1 2", "0x10"] {
            assert_eq!(parse_uint31(s), None, "{s}");
        }
    }

    #[test]
    fn query_values_are_the_first_unescaped_bytes_of_their_key() {
        let query_value = |s: &str, key: &str| split_url(s).map(|(_, q)| query_get(q, key));
        let value = |s: &str, key: &str| query_value(s, key).unwrap();
        assert_eq!(
            value("x:?a=%ff%41+b&a=2", "a"),
            Some(vec![0xff, b'A', b' ', b'b'])
        );
        assert_eq!(value("x:?a;b=1&b=%zz&b=2", "b"), Some(b"2".to_vec()));
        assert_eq!(value("x:?%61", "a"), Some(vec![]));
        assert_eq!(value("x:?a=1", "b"), None);
        assert_eq!(value("no scheme?a=1#frag", "a"), Some(b"1".to_vec()));
        for s in ["x:?a=1#%zz", "x:?a=1\n", ":?a=1"] {
            assert_eq!(query_value(s, "a"), Err(UriError::Malformed), "{s:?}");
        }
    }
}
