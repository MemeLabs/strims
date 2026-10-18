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

/// The bytes a media listing with swarm URI `uri` is keyed by: its swarm
/// id. A URI that doesn't parse ([`strims_ppspp::Uri::parse`]) yields
/// none.
pub fn media_listing_id(uri: &str) -> Vec<u8> {
    strims_ppspp::Uri::parse(uri)
        .map(|u| u.id.as_bytes().to_vec())
        .unwrap_or_default()
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
    fn directory_tables_stay_local_and_unobserved() {
        assert!(DirectoryListingRecords::REPLICATION.is_none());
        assert!(DirectoryUserRecords::REPLICATION.is_none());
        assert!(DirectoryListingRecords::observe(&ListingRecord::default()).is_none());
        assert!(DirectoryUserRecords::observe(&UserRecord::default()).is_none());
    }
}
