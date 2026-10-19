// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors (`golden/vectors.hex`) for the keys and hashes this crate
//! derives: password keys, sealed values, index hashes, directory listing
//! and chat profile index keys, and video stream keys. `STRIMS_GOLDEN=update`
//! rewrites them from the code (see `strims-golden`).

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use prost::Message;
use strims_dao::key::index_hash;
use strims_dao::schema::chat::profile_peer_key;
use strims_dao::schema::directory::listing_key;
use strims_dao::schema::video::{format_stream_key, parse_stream_key};
use strims_dao::seal::StorageKey;
use strims_golden::Golden;
use strims_proto::network::v1::directory::Listing;
use strims_proto::profile::v1 as profile;

fn open() -> Golden {
    Golden::open(concat!(env!("CARGO_MANIFEST_DIR"), "/golden/vectors.hex"))
}

fn key32(b: &[u8]) -> [u8; 32] {
    b.try_into().unwrap()
}

fn u64_of(b: &[u8]) -> u64 {
    u64::from_be_bytes(b.try_into().unwrap())
}

#[test]
fn password_keys_match() {
    let mut g = open();
    let entries = g.prefixed("kdf_");
    for &e in &entries {
        let password = String::from_utf8(g.bytes(e, 0)).unwrap();
        let record = profile::StorageKey::decode(g.bytes(e, 1).as_slice()).unwrap();
        let k = StorageKey::from_password(&password, &record).unwrap();
        g.check_hex(e, 2, k.bytes());
    }
    assert_eq!(entries.len(), 4);
}

/// Sealing draws a random nonce, so these vectors pin opening.
#[test]
fn sealed_values_open() {
    let g = open();
    let entries = g.prefixed("seal_");
    for &e in &entries {
        let k = StorageKey::from_bytes(key32(&g.bytes(e, 0)));
        assert_eq!(
            k.open(&g.bytes(e, 2)).unwrap(),
            g.bytes(e, 1),
            "{}",
            g.name(e)
        );
    }
    assert_eq!(entries.len(), 5);
}

#[test]
fn index_hashes_match() {
    let mut g = open();
    let entries = g.prefixed("index_hash_");
    for &e in &entries {
        let hash = index_hash(&g.bytes(e, 0), &g.bytes(e, 1));
        g.check_hex(e, 2, hash);
    }
    assert_eq!(entries.len(), 4);
}

#[test]
fn listing_keys_match_vectors() {
    let mut g = open();
    let entries = g.prefixed("dir_listing_key_");
    for &e in &entries {
        let network_id = u64_of(&g.bytes(e, 0));
        let listing = Listing::decode(g.bytes(e, 1).as_slice()).unwrap();
        g.check_hex(e, 2, listing_key(network_id, Some(&listing)));
    }
    assert_eq!(entries.len(), 24);
}

#[test]
fn chat_profile_peer_keys_match_vectors() {
    let mut g = open();
    let entries = g.prefixed("chat_profile_peer_key_");
    for &e in &entries {
        let server_id = u64_of(&g.bytes(e, 0));
        g.check_hex(e, 2, profile_peer_key(server_id, &g.bytes(e, 1)));
    }
    assert_eq!(entries.len(), 4);
}

/// Each vector's key, from a key seed, a channel id and a token, parses
/// back to the id and a signature of the token by the key.
#[test]
fn stream_keys_match_vectors() {
    let mut g = open();
    let entries: Vec<usize> = g
        .prefixed("video_stream_key_")
        .into_iter()
        .filter(|&e| !g.name(e).starts_with("video_stream_key_parse_"))
        .collect();
    for &e in &entries {
        let name = g.name(e).to_owned();
        let key = strims_crypto::key_from_seed(&key32(&g.bytes(e, 0)));
        let (id, token) = (u64_of(&g.bytes(e, 1)), g.bytes(e, 2));
        let formatted = format_stream_key(id, &token, &key).unwrap();
        let (parsed, signature) = parse_stream_key(&formatted).unwrap();
        assert_eq!(parsed, id, "{name}");
        assert!(
            strims_crypto::sign::verify(
                strims_crypto::Domain::VideoStreamKey,
                &key.public,
                &[&token],
                &signature
            ),
            "{name}"
        );
        g.check_hex(e, 3, formatted);
    }
    assert_eq!(entries.len(), 6);
}

/// Each vector's key parses to the id and signature it lists, and the
/// keys listed without them are rejected.
#[test]
fn stream_keys_parse_to_their_ids_and_signatures() {
    let mut g = open();
    let (mut accepted, mut rejected) = (0, 0);
    for e in g.prefixed("video_stream_key_parse_") {
        let key = String::from_utf8(g.bytes(e, 0)).unwrap();
        match parse_stream_key(&key) {
            Ok((id, signature)) => {
                g.check_hex(e, 1, id.to_be_bytes());
                g.check_hex(e, 2, signature);
                accepted += 1;
            }
            Err(_) => {
                g.check(e, 1, "");
                g.check(e, 2, "");
                rejected += 1;
            }
        }
    }
    assert_eq!((accepted, rejected), (3, 11));
}
