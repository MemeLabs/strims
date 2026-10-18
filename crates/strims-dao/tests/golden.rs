// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors from `golden/main.go` (`go run -tags golden
//! ./crates/strims-dao/golden`), produced by the real Go functions.

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;

use prost::Message;
use strims_dao::key::index_hash;
use strims_dao::schema::chat::profile_peer_key;
use strims_dao::schema::directory::listing_key;
use strims_dao::schema::video::{format_stream_key, parse_stream_key};
use strims_dao::seal::StorageKey;
use strims_proto::network::v1::directory::Listing;
use strims_proto::profile::v1 as profile;

const VECTORS: &str = include_str!("../golden/vectors.hex");

struct Vectors(BTreeMap<&'static str, Vec<Vec<u8>>>);

impl Vectors {
    fn load() -> Self {
        let mut m = BTreeMap::new();
        for line in VECTORS.lines().filter(|l| !l.trim().is_empty()) {
            let mut parts = line.split(' ');
            let name = parts.next().unwrap();
            let values = parts
                .map(|p| {
                    if p == "=" {
                        Vec::new()
                    } else {
                        hex::decode(p).unwrap()
                    }
                })
                .collect();
            m.insert(name, values);
        }
        Vectors(m)
    }

    fn with_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = (&'static str, &'a [Vec<u8>])> + 'a {
        let found = self.0.keys().any(|k| k.starts_with(prefix));
        assert!(found, "no vectors named {prefix}*");
        self.0
            .iter()
            .filter(move |(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (*k, v.as_slice()))
    }
}

fn key32(b: &[u8]) -> [u8; 32] {
    b.try_into().unwrap()
}

#[test]
fn password_keys_match() {
    let v = Vectors::load();
    for (name, f) in v.with_prefix("kdf_") {
        let password = std::str::from_utf8(&f[0]).unwrap();
        let record = profile::StorageKey::decode(f[1].as_slice()).unwrap();
        let k = StorageKey::from_password(password, &record).unwrap();
        assert_eq!(k.bytes().as_slice(), f[2].as_slice(), "{name}");
    }
}

#[test]
fn sealed_values_open() {
    let v = Vectors::load();
    for (name, f) in v.with_prefix("seal_") {
        let k = StorageKey::from_bytes(key32(&f[0]));
        assert_eq!(k.open(&f[2]).unwrap(), f[1], "{name}");
    }
}

#[test]
fn index_hashes_match() {
    let v = Vectors::load();
    for (name, f) in v.with_prefix("index_hash_") {
        let expected = std::str::from_utf8(&f[2]).unwrap();
        assert_eq!(index_hash(&f[0], &f[1]), expected, "{name}");
    }
}

#[test]
fn listing_keys_match_go() {
    let v = Vectors::load();
    let mut n = 0;
    for (name, f) in v.with_prefix("dir_listing_key_") {
        let network_id = u64::from_be_bytes(f[0].as_slice().try_into().unwrap());
        let listing = Listing::decode(f[1].as_slice()).unwrap();
        assert_eq!(
            hex::encode(listing_key(network_id, Some(&listing))),
            hex::encode(&f[2]),
            "{name}: {listing:?}"
        );
        n += 1;
    }
    assert_eq!(n, 24);
}

#[test]
fn chat_profile_peer_keys_match_go() {
    let v = Vectors::load();
    let mut n = 0;
    for (name, f) in v.with_prefix("chat_profile_peer_key_") {
        let server_id = u64::from_be_bytes(f[0].as_slice().try_into().unwrap());
        assert_eq!(profile_peer_key(server_id, &f[1]), f[2], "{name}");
        n += 1;
    }
    assert_eq!(n, 4);
}

#[test]
fn stream_keys_match_go() {
    let v = Vectors::load();
    let mut n = 0;
    for (name, f) in v.with_prefix("video_stream_key_") {
        if name.starts_with("video_stream_key_parse_") {
            continue;
        }
        let key = strims_crypto::key_from_seed(&key32(&f[0]));
        let id = u64::from_be_bytes(f[1].as_slice().try_into().unwrap());
        let formatted = std::str::from_utf8(&f[3]).unwrap();
        assert_eq!(
            format_stream_key(id, &f[2], &key).unwrap(),
            formatted,
            "{name}"
        );
        let (parsed, signature) = parse_stream_key(formatted).unwrap();
        assert_eq!(parsed, id, "{name}");
        assert!(
            strims_crypto::verify(&key.public, &f[2], &signature),
            "{name}"
        );
        n += 1;
    }
    assert_eq!(n, 6);
}

/// Each vector's key parses to the id and signature it lists, and the
/// keys listed without them are rejected.
#[test]
fn stream_keys_parse_as_go_parses_them() {
    let v = Vectors::load();
    let (mut accepted, mut rejected) = (0, 0);
    for (name, f) in v.with_prefix("video_stream_key_parse_") {
        let key = std::str::from_utf8(&f[0]).unwrap();
        let got = parse_stream_key(key);
        if f[1].is_empty() {
            assert!(got.is_err(), "{name}: {key} parsed as {got:?}");
            rejected += 1;
        } else {
            let id = u64::from_be_bytes(f[1].as_slice().try_into().unwrap());
            let (parsed, signature) = got.unwrap_or_else(|e| panic!("{name}: {key}: {e}"));
            assert_eq!(
                (parsed, signature.as_slice()),
                (id, f[2].as_slice()),
                "{name}"
            );
            accepted += 1;
        }
    }
    assert_eq!((accepted, rejected), (3, 11));
}
