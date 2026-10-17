// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The directory moderation records of networks whose server config this
//! device holds: a record per listing and per user of a network's
//! directory, read and written by the directory server.
//!
//! The records stay with the server config, so they don't replicate, and
//! they aren't observable. The owner of a network created here is its
//! directory's admin ([`Networks`](super::network::Networks) writes that
//! record), and deleting a network deletes its directory records.
//!
//! Index keys are stored, so [`listing_key`] and [`media_listing_id`]
//! must keep deriving the same bytes.

use strims_ppspp::query_value;
use strims_proto::network::v1::directory::listing::Content;
use strims_proto::network::v1::directory::{
    Listing, ListingModeration, ListingRecord, UserModeration, UserRecord,
};

use crate::key::Namespace;
use crate::schema::network::network_scoped_key;
use crate::table::{Index, Table, ref_key};

/// A directory's listings, with their moderation.
pub struct DirectoryListingRecords;

impl DirectoryListingRecords {
    /// Listing records by network.
    pub const BY_NETWORK: Index<ListingRecord> =
        Index::new(Namespace(9002), |r| ref_key(r.network_id));
    /// Listing records by [`listing_key`]; at most one per listing.
    pub const BY_LISTING: Index<ListingRecord> = Index::unique(Namespace(9003), |r| {
        listing_key(r.network_id, r.listing.as_ref())
    });
}

impl Table for DirectoryListingRecords {
    type Record = ListingRecord;
    const NS: Namespace = Namespace(9001);
    const INDEXES: &'static [Index<ListingRecord>] = &[
        DirectoryListingRecords::BY_NETWORK,
        DirectoryListingRecords::BY_LISTING,
    ];

    fn id(r: &ListingRecord) -> u64 {
        r.id
    }
}

/// A directory's users, with their moderation.
pub struct DirectoryUserRecords;

impl DirectoryUserRecords {
    /// User records by (network, peer key); at most one per user.
    pub const BY_PEER_KEY: Index<UserRecord> = Index::unique(Namespace(9005), |r| {
        network_scoped_key(r.network_id, &r.peer_key)
    });
    /// User records by network.
    pub const BY_NETWORK: Index<UserRecord> =
        Index::new(Namespace(9006), |r| ref_key(r.network_id));
}

impl Table for DirectoryUserRecords {
    type Record = UserRecord;
    const NS: Namespace = Namespace(9004);
    const INDEXES: &'static [Index<UserRecord>] = &[
        DirectoryUserRecords::BY_NETWORK,
        DirectoryUserRecords::BY_PEER_KEY,
    ];

    fn id(r: &UserRecord) -> u64 {
        r.id
    }
}

const CHAT: u8 = 1;
const EMBED: u8 = 2;
const MEDIA: u8 = 3;
const SERVICE: u8 = 4;

/// The key of `listing` on network `network_id`: the network id
/// (little-endian), the content type (chat 1, embed 2, media 3, service
/// 4), then what identifies the content:
/// - chat: its key;
/// - embed: its service (`i32`, little-endian), then its id;
/// - media: the [`media_listing_id`] of its swarm URI;
/// - service: its type, then its swarm URI.
///
/// A listing without content, or none, is keyed by the network id alone.
pub fn listing_key(network_id: u64, listing: Option<&Listing>) -> Vec<u8> {
    let mut k = network_id.to_le_bytes().to_vec();
    match listing.and_then(|l| l.content.as_ref()) {
        Some(Content::Chat(c)) => {
            k.push(CHAT);
            k.extend_from_slice(&c.key);
        }
        Some(Content::Embed(e)) => {
            k.push(EMBED);
            k.extend_from_slice(&e.service.to_le_bytes());
            k.extend_from_slice(e.id.as_bytes());
        }
        Some(Content::Media(m)) => {
            k.push(MEDIA);
            k.extend_from_slice(&media_listing_id(&m.swarm_uri));
        }
        Some(Content::Service(s)) => {
            k.push(SERVICE);
            k.extend_from_slice(s.r#type.as_bytes());
            k.extend_from_slice(s.swarm_uri.as_bytes());
        }
        None => {}
    }
    k
}

/// The bytes a media listing with swarm URI `uri` is keyed by: its `xt`
/// value after the value's last `:`, without CR and LF, read as padded
/// base32 up to the first decode error. A quantum that fails to decode
/// yields nothing, so a swarm URI's unpadded 32-byte id yields its first
/// 30 bytes. A URI that isn't a URL ([`query_value`]), or has no `xt`,
/// yields none.
pub fn media_listing_id(uri: &str) -> Vec<u8> {
    let Ok(Some(xt)) = query_value(uri, "xt") else {
        return Vec::new();
    };
    let id = match xt.iter().rposition(|&c| c == b':') {
        Some(i) => xt.get(i + 1..).unwrap_or_default(),
        None => &xt,
    };
    padded_base32_prefix(id)
}

const PAD: u8 = b'=';

fn base32_value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'2'..=b'7' => Some(c - b'2' + 26),
        _ => None,
    }
}

/// `s` without CR and LF, decoded as padded RFC 4648 base32 in quanta of
/// eight characters, up to the first error: a character outside the
/// upper-case alphabet, an input ending inside a quantum, or bad padding.
/// The result holds the quanta before the error.
///
/// Padding may start within the last eight characters, after at least
/// two characters of its quantum, and must fill the quantum, which then
/// yields its whole bytes (from 2, 4, 5 or 7 characters). Padding ends
/// the input: what follows the quantum is ignored. Padding anywhere else
/// is an invalid character.
fn padded_base32_prefix(s: &[u8]) -> Vec<u8> {
    let src: Vec<u8> = s
        .iter()
        .copied()
        .filter(|&c| c != b'\r' && c != b'\n')
        .collect();
    let mut out = Vec::with_capacity(src.len() / 8 * 5);
    let mut rest = src.as_slice();
    while !rest.is_empty() {
        let mut quantum = [0u8; 8];
        let mut len = 0;
        let mut end = false;
        while len < quantum.len() {
            let Some((&c, tail)) = rest.split_first() else {
                return out;
            };
            rest = tail;
            if c == PAD && len >= 2 && rest.len() < 8 {
                let fill = quantum.len() - 1 - len;
                if rest.len() < fill || rest.iter().take(fill).any(|&c| c != PAD) {
                    return out;
                }
                if matches!(len, 3 | 6) {
                    return out;
                }
                end = true;
                break;
            }
            let Some(v) = base32_value(c) else {
                return out;
            };
            if let Some(q) = quantum.get_mut(len) {
                *q = v;
            }
            len += 1;
        }
        let bits = quantum
            .iter()
            .fold(0u64, |acc, &v| (acc << 5) | u64::from(v))
            .to_be_bytes();
        let n = match len {
            8 => 5,
            7 => 4,
            5 => 3,
            4 => 2,
            _ => 1,
        };
        out.extend(bits.iter().skip(3).take(n));
        if end {
            break;
        }
    }
    out
}

/// A record of `listing` on network `network_id`.
pub fn new_listing_record(
    id: u64,
    network_id: u64,
    listing: Listing,
    moderation: Option<ListingModeration>,
) -> ListingRecord {
    ListingRecord {
        id,
        network_id,
        listing: Some(listing),
        moderation,
        ..Default::default()
    }
}

/// A record of the user with `peer_key` on network `network_id`.
pub fn new_user_record(
    id: u64,
    network_id: u64,
    peer_key: &[u8],
    moderation: UserModeration,
) -> UserRecord {
    UserRecord {
        id,
        network_id,
        peer_key: peer_key.to_vec(),
        moderation: Some(moderation),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_base32_keeps_the_quanta_before_an_error() {
        let decode = |s: &str| padded_base32_prefix(s.as_bytes());
        assert_eq!(decode("MZXW6YTBOI======"), b"foobar");
        assert_eq!(decode("MZXW6YTBOI"), b"fooba");
        assert_eq!(decode("MZXW6YTB\r\nOI======"), b"foobar");
        assert_eq!(decode("MZXW6YTBoi======"), b"fooba");
        assert_eq!(decode("MY======"), b"f");
        assert_eq!(decode("MY======AB"), b"f");
        assert_eq!(decode("MZXQ===="), b"fo");
        assert_eq!(decode("MZXW6==="), b"foo");
        assert_eq!(decode("MZXW6YQ="), b"foob");
        assert_eq!(decode("MZXW6YTBMY======AB"), b"foobaf");
        assert_eq!(decode("MZXW6YTBMY======MY======"), b"fooba");
        for s in ["MY=", "MY=====A", "MZX=====", "M=======", "my======"] {
            assert_eq!(decode(s), b"", "{s}");
        }
    }

    #[test]
    fn directory_tables_stay_local_and_unobserved() {
        assert!(DirectoryListingRecords::REPLICATION.is_none());
        assert!(DirectoryUserRecords::REPLICATION.is_none());
        assert!(DirectoryListingRecords::observe(&ListingRecord::default()).is_none());
        assert!(DirectoryUserRecords::observe(&UserRecord::default()).is_none());
    }
}
