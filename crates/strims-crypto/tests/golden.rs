// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors from `golden/main.go` (`go run -tags golden
//! ./crates/strims-crypto/golden`), produced by the real Go functions.

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;

use prost::Message;
use strims_base::time::{Duration, Time};
use strims_crypto::cert::{certificate_parent, verify_certificate};
use strims_crypto::{
    Error, SignableMessage, aes_link_key, certificate_network_key, crc32_message, key_from_seed,
    key_to_curve25519, message_cipher_key, new_certificate_request, public_key_to_curve25519,
    serialize_certificate, serialize_certificate_request, sign, verify, verify_certificate_request,
    x25519,
};
use strims_proto::stable::{Signable, StableEncode};
use strims_proto::types::{Certificate, CertificateRequest, Key, KeyUsage};
use strims_proto::{chat, network, types, vpn};

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

    fn get(&self, name: &str) -> &[Vec<u8>] {
        self.0
            .get(name)
            .unwrap_or_else(|| panic!("missing vector {name}"))
    }

    fn with_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = (&'static str, &'a [Vec<u8>])> + 'a {
        self.0
            .iter()
            .filter(move |(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (*k, v.as_slice()))
    }

    fn key(&self, i: usize) -> Key {
        Key::decode(self.get(&format!("key_{i}"))[1].as_slice()).unwrap()
    }

    fn now(&self) -> Time {
        let b: [u8; 8] = self.get("now")[0].as_slice().try_into().unwrap();
        Time::from_unix_nanos(i64::from_be_bytes(b))
    }
}

const SIGN: i32 = KeyUsage::Sign as i32;
const PEER: i32 = KeyUsage::Peer as i32;
const BOOTSTRAP: i32 = KeyUsage::Bootstrap as i32;

fn arr32(b: &[u8]) -> [u8; 32] {
    b.try_into().unwrap()
}

#[test]
fn keys_match_go_storage_format() {
    let v = Vectors::load();
    for i in 0..4 {
        let [seed, proto] = v.get(&format!("key_{i}")) else {
            panic!()
        };
        let k = key_from_seed(&arr32(seed));
        assert_eq!(
            hex::encode(k.encode_to_vec()),
            hex::encode(proto),
            "key_{i}"
        );
        // the private key is the 64-byte expanded key: seed || public
        assert_eq!(&k.private[..32], seed.as_slice());
        assert_eq!(&k.private[32..], k.public.as_slice());
    }
}

#[test]
fn signatures_match_go() {
    let v = Vectors::load();
    let mut n = 0;
    for (name, values) in v.with_prefix("sign_") {
        let [msg, sig] = values else { panic!() };
        let i: usize = name[5..6].parse().unwrap();
        let k = v.key(i);
        assert_eq!(
            sign(&k.private, msg).unwrap().as_slice(),
            sig.as_slice(),
            "{name}"
        );
        assert!(verify(&k.public, msg, sig), "{name}");
        n += 1;
    }
    assert_eq!(n, 8);
}

#[test]
fn certificate_chain_matches_go() {
    let v = Vectors::load();
    let now = v.now();
    let (network_key, member_key, leaf_key) = (v.key(0), v.key(1), v.key(2));

    let proto = &v.get("cert_chain")[0];
    let leaf = Certificate::decode(proto.as_slice()).unwrap();
    assert_eq!(leaf.encode_to_vec(), *proto);
    verify_certificate(&leaf, now).unwrap();

    let member = certificate_parent(&leaf).unwrap();
    let root = certificate_parent(member).unwrap();
    assert!(certificate_parent(root).is_none());

    for (name, cert, signer) in [
        ("root", root, &network_key),
        ("member", member, &network_key),
        ("leaf", &leaf, &member_key),
    ] {
        let serialized = serialize_certificate(cert);
        assert_eq!(
            hex::encode(&serialized),
            hex::encode(&v.get(&format!("cert_serialized_{name}"))[0]),
            "{name}"
        );
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
        v.get("cert_network_key")[0].as_slice()
    );
    assert_eq!(
        certificate_network_key(&leaf),
        network_key.public.as_slice()
    );

    // the leaf is valid for a day, predated by an hour
    let errs = verify_certificate(&leaf, now + Duration::HOUR * 25).unwrap_err();
    assert_eq!(errs.errors(), &[Error::NotAfterRange]);
    let errs = verify_certificate(&leaf, now - Duration::HOUR * 2).unwrap_err();
    assert_eq!(errs.errors(), &[Error::NotBeforeRange]);
}

#[test]
fn certificate_requests_match_go() {
    let v = Vectors::load();
    for (name, key, usage, subject) in [
        ("csr_member", v.key(1), PEER | SIGN, "alias"),
        ("csr_leaf", v.key(2), PEER | BOOTSTRAP, ""),
    ] {
        let [proto, serialized] = v.get(name) else {
            panic!()
        };
        let go = CertificateRequest::decode(proto.as_slice()).unwrap();
        verify_certificate_request(&go, usage).unwrap();
        assert_eq!(
            verify_certificate_request(&go, PEER),
            Err(Error::UnsupportedKeyUsage),
            "{name}: usage outside the allowed set"
        );
        assert_eq!(serialize_certificate_request(&go), *serialized, "{name}");
        let ours = new_certificate_request(&key, usage, subject).unwrap();
        assert_eq!(ours, go, "{name}");
        assert_eq!(ours.encode_to_vec(), *proto, "{name}");
    }
}

fn check_signable<M: Message + Default + Signable + Clone>(v: &Vectors, name: &str, key: &Key) {
    let [proto, unsigned] = v.get(&format!("signable_{name}")) else {
        panic!()
    };
    let go = M::decode(proto.as_slice()).unwrap();
    assert_eq!(go.encode_to_vec(), *proto, "{name}");
    go.verify_message().unwrap();
    assert_eq!(
        hex::encode(go.stable_encode_unsigned_to_vec()),
        hex::encode(unsigned),
        "{name}"
    );
    assert_eq!(go.key(), key.public.as_slice());

    let mut ours = go.clone();
    ours.set_key(&[]);
    ours.set_signature(Vec::new());
    ours.sign_message(key).unwrap();
    assert_eq!(
        hex::encode(ours.signature()),
        hex::encode(go.signature()),
        "{name}"
    );
    assert_eq!(ours.encode_to_vec(), *proto, "{name}");
}

#[test]
fn signable_messages_match_go() {
    let v = Vectors::load();
    check_signable::<vpn::v1::hash_table_message::Record>(&v, "hash_table_record", &v.key(3));
    check_signable::<vpn::v1::peer_index_message::Record>(&v, "peer_index_record", &v.key(1));
    check_signable::<network::v1::directory::ListingSnippet>(&v, "listing_snippet", &v.key(2));
}

fn check_crc<M: Message + Default + StableEncode>(v: &Vectors, name: &str) {
    let [proto, crc] = v.get(&format!("crc32_{name}")) else {
        panic!()
    };
    let m = M::decode(proto.as_slice()).unwrap();
    assert_eq!(
        crc32_message(&m).to_be_bytes().as_slice(),
        crc.as_slice(),
        "{name}"
    );
}

#[test]
fn crc32_messages_match_go() {
    let v = Vectors::load();
    check_crc::<types::Cache>(&v, "swarm_cache");
    check_crc::<types::Cache>(&v, "empty_cache");
    check_crc::<chat::v1::Emote>(&v, "chat_emote");
    check_crc::<chat::v1::Modifier>(&v, "chat_modifier");
    check_crc::<chat::v1::Tag>(&v, "chat_tag");
    check_crc::<chat::v1::Server>(&v, "chat_server");
    check_crc::<chat::v1::ServerIcon>(&v, "chat_server_icon");
    assert_eq!(v.with_prefix("crc32_").count(), 7);
}

#[test]
fn curve25519_conversions_match_go() {
    let v = Vectors::load();
    for i in 0..4 {
        let c = key_to_curve25519(&v.key(i)).unwrap();
        assert_eq!(
            hex::encode(c.encode_to_vec()),
            hex::encode(&v.get(&format!("curve_key_{i}"))[0]),
            "curve_key_{i}"
        );
    }
    let mut n = 0;
    for (name, values) in v.with_prefix("pub2curve_") {
        let [input, output] = values else { panic!() };
        assert_eq!(
            hex::encode(public_key_to_curve25519(&arr32(input))),
            hex::encode(output),
            "{name}"
        );
        n += 1;
    }
    assert_eq!(n, 12);
}

#[test]
fn key_agreement_matches_go() {
    let v = Vectors::load();
    let mut n = 0;
    for (name, values) in v.with_prefix("aes_link_") {
        let ids: Vec<usize> = name["aes_link_".len()..]
            .split('_')
            .map(|s| s.parse().unwrap())
            .collect();
        let (a, b) = (
            key_to_curve25519(&v.key(ids[0])).unwrap(),
            key_to_curve25519(&v.key(ids[1])).unwrap(),
        );
        assert_eq!(
            aes_link_key(&a, &b.public).unwrap().as_slice(),
            values[0].as_slice(),
            "{name}"
        );
        n += 1;
    }
    for (name, values) in v.with_prefix("message_cipher_") {
        let ids: Vec<usize> = name["message_cipher_".len()..]
            .split('_')
            .map(|s| s.parse().unwrap())
            .collect();
        let (a, b) = (v.key(ids[0]), v.key(ids[1]));
        assert_eq!(
            message_cipher_key(&a, &arr32(&b.public))
                .unwrap()
                .as_slice(),
            values[0].as_slice(),
            "{name}"
        );
        n += 1;
    }
    assert_eq!(n, 8);

    let mut errors = 0;
    for (name, values) in v.with_prefix("x25519_") {
        let [scalar, point, out] = values else {
            panic!()
        };
        match x25519(scalar, point) {
            Ok(secret) => assert_eq!(secret.as_slice(), out.as_slice(), "{name}"),
            Err(e) => {
                assert_eq!(out.as_slice(), b"error", "{name}: {e}");
                assert_eq!(e, Error::LowOrderPoint);
                errors += 1;
            }
        }
    }
    assert!(errors > 0, "no low-order cases exercised");
}
