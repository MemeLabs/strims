// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors (`golden/vectors.hex`): signatures, certificate chains
//! and requests, signable messages, message checksums, curve25519
//! conversions and key agreement. `STRIMS_GOLDEN=update` rewrites them from
//! the code (see `strims-golden`).

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use prost::Message;
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::cert::{certificate_parent, verify_certificate};
use strims_crypto::{
    DEFAULT_CERT_TTL, Error, SignableMessage, aes_link_key, certificate_network_key, crc32_message,
    key_from_seed, key_to_curve25519, message_cipher_key, new_certificate_request,
    new_self_signed_certificate, public_key_to_curve25519, serialize_certificate,
    serialize_certificate_request, sign, sign_certificate_request, verify,
    verify_certificate_request, x25519,
};
use strims_golden::Golden;
use strims_proto::stable::{Signable, StableEncode};
use strims_proto::types::{Key, KeyUsage, certificate::ParentOneof};
use strims_proto::{chat, network, types, vpn};

fn open() -> Golden {
    Golden::open(concat!(env!("CARGO_MANIFEST_DIR"), "/golden/vectors.hex"))
}

fn key(g: &Golden, i: usize) -> Key {
    let e = g.all(&format!("key_{i}"))[0];
    key_from_seed(&arr32(&g.bytes(e, 0)))
}

fn now(g: &Golden) -> Time {
    let b: [u8; 8] = g.bytes(g.all("now")[0], 0).try_into().unwrap();
    Time::from_unix_nanos(i64::from_be_bytes(b))
}

const SIGN: i32 = KeyUsage::Sign as i32;
const PEER: i32 = KeyUsage::Peer as i32;
const BOOTSTRAP: i32 = KeyUsage::Bootstrap as i32;

fn arr32(b: &[u8]) -> [u8; 32] {
    b.try_into().unwrap()
}

/// The index in a vector name's `_`-separated suffix, e.g. `1` for `sign_1_3`.
fn ids(name: &str, prefix: &str) -> Vec<usize> {
    name[prefix.len()..]
        .split('_')
        .map(|s| s.parse().unwrap())
        .collect()
}

#[test]
fn keys_keep_their_storage_format() {
    let mut g = open();
    for i in 0..4 {
        let e = g.entry(&format!("key_{i}"));
        let seed = g.bytes(e, 0);
        let k = key_from_seed(&arr32(&seed));
        g.check_hex(e, 1, k.encode_to_vec());
        // the private key is the 64-byte expanded key: seed || public
        assert_eq!(&k.private[..32], seed.as_slice());
        assert_eq!(&k.private[32..], k.public.as_slice());
    }
}

#[test]
fn signatures_match_vectors() {
    let mut g = open();
    let entries = g.prefixed("sign_");
    for &e in &entries {
        let i = ids(g.name(e), "sign_")[0];
        let (k, msg) = (key(&g, i), g.bytes(e, 0));
        let sig = sign(&k.private, &msg).unwrap();
        assert!(verify(&k.public, &msg, &sig), "{}", g.name(e));
        g.check_hex(e, 1, sig);
    }
    assert_eq!(entries.len(), 8);
}

#[test]
fn certificate_chain_matches_vectors() {
    let mut g = open();
    let now = now(&g);
    let (network_key, member_key, leaf_key) = (key(&g, 0), key(&g, 1), key(&g, 2));
    let mut rng = det_rng(1);

    let root = new_self_signed_certificate(
        &network_key,
        SIGN,
        DEFAULT_CERT_TTL,
        "network",
        now,
        &mut rng,
    )
    .unwrap();
    let member_csr = new_certificate_request(&member_key, PEER | SIGN, "alias").unwrap();
    let mut member =
        sign_certificate_request(&member_csr, DEFAULT_CERT_TTL, &network_key, now, &mut rng)
            .unwrap();
    member.parent_oneof = Some(ParentOneof::Parent(Box::new(root)));
    let leaf_csr = new_certificate_request(&leaf_key, PEER | BOOTSTRAP, "").unwrap();
    let mut leaf =
        sign_certificate_request(&leaf_csr, Duration::HOUR * 24, &member_key, now, &mut rng)
            .unwrap();
    leaf.parent_oneof = Some(ParentOneof::Parent(Box::new(member)));
    verify_certificate(&leaf, now).unwrap();

    let e = g.entry("cert_chain");
    g.check_hex(e, 0, leaf.encode_to_vec());

    let member = certificate_parent(&leaf).unwrap();
    let root = certificate_parent(member).unwrap();
    assert!(certificate_parent(root).is_none());
    for (name, cert, signer) in [
        ("root", root, &network_key),
        ("member", member, &network_key),
        ("leaf", &leaf, &member_key),
    ] {
        let serialized = serialize_certificate(cert);
        let e = g.entry(&format!("cert_serialized_{name}"));
        g.check_hex(e, 0, &serialized);
        // ed25519 is deterministic: identical bytes, identical signature
        assert_eq!(
            sign(&signer.private, &serialized).unwrap().as_slice(),
            cert.signature.as_slice(),
            "{name}"
        );
    }
    assert_eq!(leaf.key, leaf_key.public);
    assert_eq!(
        certificate_network_key(&leaf),
        network_key.public.as_slice()
    );
    let e = g.entry("cert_network_key");
    g.check_hex(e, 0, certificate_network_key(&leaf));

    // the leaf is valid for a day, predated by an hour
    let errs = verify_certificate(&leaf, now + Duration::HOUR * 25).unwrap_err();
    assert_eq!(errs.errors(), &[Error::NotAfterRange]);
    let errs = verify_certificate(&leaf, now - Duration::HOUR * 2).unwrap_err();
    assert_eq!(errs.errors(), &[Error::NotBeforeRange]);
}

#[test]
fn certificate_requests_match_vectors() {
    let mut g = open();
    for (name, i, usage, subject) in [
        ("csr_member", 1, PEER | SIGN, "alias"),
        ("csr_leaf", 2, PEER | BOOTSTRAP, ""),
    ] {
        let csr = new_certificate_request(&key(&g, i), usage, subject).unwrap();
        verify_certificate_request(&csr, usage).unwrap();
        assert_eq!(
            verify_certificate_request(&csr, PEER),
            Err(Error::UnsupportedKeyUsage),
            "{name}: usage outside the allowed set"
        );
        let e = g.entry(name);
        g.check_hex(e, 0, csr.encode_to_vec());
        g.check_hex(e, 1, serialize_certificate_request(&csr));
    }
}

/// Signs the vector's message, with its key and signature cleared, by `key`.
fn check_signable<M: Message + Default + Signable + Clone>(g: &mut Golden, name: &str, key: &Key) {
    let e = g.entry(&format!("signable_{name}"));
    let mut m = M::decode(g.bytes(e, 0).as_slice()).unwrap();
    m.set_key(&[]);
    m.set_signature(Vec::new());
    m.sign_message(key).unwrap();
    m.verify_message().unwrap();
    assert_eq!(m.key(), key.public.as_slice());
    g.check_hex(e, 0, m.encode_to_vec());
    g.check_hex(e, 1, m.stable_encode_unsigned_to_vec());
}

#[test]
fn signable_messages_match_vectors() {
    let mut g = open();
    let (k1, k2, k3) = (key(&g, 1), key(&g, 2), key(&g, 3));
    check_signable::<vpn::v1::hash_table_message::Record>(&mut g, "hash_table_record", &k3);
    check_signable::<vpn::v1::peer_index_message::Record>(&mut g, "peer_index_record", &k1);
    check_signable::<network::v1::directory::ListingSnippet>(&mut g, "listing_snippet", &k2);
}

/// Checksums the vector's message.
fn check_crc<M: Message + Default + StableEncode>(g: &mut Golden, name: &str) {
    let e = g.entry(&format!("crc32_{name}"));
    let m = M::decode(g.bytes(e, 0).as_slice()).unwrap();
    g.check_hex(e, 1, crc32_message(&m).to_be_bytes());
}

#[test]
fn crc32_messages_match_vectors() {
    let mut g = open();
    check_crc::<types::Cache>(&mut g, "swarm_cache");
    check_crc::<types::Cache>(&mut g, "empty_cache");
    check_crc::<chat::v1::Emote>(&mut g, "chat_emote");
    check_crc::<chat::v1::Modifier>(&mut g, "chat_modifier");
    check_crc::<chat::v1::Tag>(&mut g, "chat_tag");
    check_crc::<chat::v1::Server>(&mut g, "chat_server");
    check_crc::<chat::v1::ServerIcon>(&mut g, "chat_server_icon");
    assert_eq!(g.prefixed("crc32_").len(), 7);
}

#[test]
fn curve25519_conversions_match_vectors() {
    let mut g = open();
    for i in 0..4 {
        let c = key_to_curve25519(&key(&g, i)).unwrap();
        let e = g.entry(&format!("curve_key_{i}"));
        g.check_hex(e, 0, c.encode_to_vec());
    }
    let entries = g.prefixed("pub2curve_");
    for &e in &entries {
        let input = g.bytes(e, 0);
        g.check_hex(e, 1, public_key_to_curve25519(&arr32(&input)));
    }
    assert_eq!(entries.len(), 12);
}

#[test]
fn key_agreement_matches_vectors() {
    let mut g = open();
    let mut n = 0;
    for e in g.prefixed("aes_link_") {
        let ids = ids(g.name(e), "aes_link_");
        let (a, b) = (
            key_to_curve25519(&key(&g, ids[0])).unwrap(),
            key_to_curve25519(&key(&g, ids[1])).unwrap(),
        );
        g.check_hex(e, 0, aes_link_key(&a, &b.public).unwrap());
        n += 1;
    }
    for e in g.prefixed("message_cipher_") {
        let ids = ids(g.name(e), "message_cipher_");
        let (a, b) = (key(&g, ids[0]), key(&g, ids[1]));
        g.check_hex(e, 0, message_cipher_key(&a, &arr32(&b.public)).unwrap());
        n += 1;
    }
    assert_eq!(n, 8);

    let mut errors = 0;
    for e in g.prefixed("x25519_") {
        let (scalar, point) = (g.bytes(e, 0), g.bytes(e, 1));
        match x25519(&scalar, &point) {
            Ok(secret) => g.check_hex(e, 2, secret),
            Err(err) => {
                assert_eq!(err, Error::LowOrderPoint);
                g.check_hex(e, 2, b"error");
                errors += 1;
            }
        }
    }
    assert!(errors > 0, "no low-order cases exercised");
}
