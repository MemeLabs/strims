// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors that `golden/main.go` records from the Go implementation
//! (regenerate from the repo root with
//! `go run -tags golden ./crates/strims-proto/golden`). prost must decode
//! each vector's protobuf bytes and re-encode them identically, and
//! `StableEncode` must reproduce the recorded stable encoding.

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use prost::Message;
use strims_proto::stable::StableEncode;
use strims_proto::{chat, devtools, network, types, vnic, vpn};

const VECTORS: &str = include_str!("../golden/vectors.hex");

struct Vector {
    name: &'static str,
    proto: Vec<u8>,
    /// `None` for messages without a stable encoding (they contain maps).
    stable: Option<Vec<u8>>,
}

fn decode_hex(s: &str) -> Vec<u8> {
    if s == "=" {
        return Vec::new();
    }
    hex::decode(s).unwrap()
}

fn vectors() -> Vec<Vector> {
    VECTORS
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let mut parts = line.split(' ');
            let name = parts.next().unwrap();
            let proto = decode_hex(parts.next().unwrap());
            let stable = match parts.next().unwrap() {
                "-" => None,
                s => Some(decode_hex(s)),
            };
            Vector {
                name,
                proto,
                stable,
            }
        })
        .collect()
}

fn check<M: Message + Default + StableEncode>(v: &Vector) -> M {
    let m = M::decode(v.proto.as_slice()).unwrap_or_else(|e| panic!("{}: decode: {e}", v.name));
    assert_eq!(
        hex::encode(m.encode_to_vec()),
        hex::encode(&v.proto),
        "{}: protobuf re-encode",
        v.name
    );
    assert_eq!(m.encoded_len(), v.proto.len(), "{}: encoded_len", v.name);
    let stable = v
        .stable
        .as_ref()
        .unwrap_or_else(|| panic!("{}: no recorded stable encoding", v.name));
    assert_eq!(
        hex::encode(m.stable_encode_to_vec()),
        hex::encode(stable),
        "{}: stable encoding",
        v.name
    );
    assert_eq!(
        m.stable_encoded_len(),
        stable.len(),
        "{}: stable_encoded_len",
        v.name
    );
    m
}

#[test]
fn golden_round_trip() {
    let vectors = vectors();
    assert!(vectors.len() >= 28, "golden file truncated");
    for v in &vectors {
        match v.name {
            "type_certificate_chain" => {
                let c: types::Certificate = check(v);
                // three levels: leaf -> intermediate -> root
                let Some(types::certificate::ParentOneof::Parent(ca)) = &c.parent_oneof else {
                    panic!("leaf has no parent");
                };
                let Some(types::certificate::ParentOneof::Parent(root)) = &ca.parent_oneof else {
                    panic!("intermediate has no parent");
                };
                assert_eq!(root.subject, "root");
                assert!(root.parent_oneof.is_none());
                assert_eq!(c.subject, "leaf ✓");
            }
            "type_certificate_parent_serial" => {
                let c: types::Certificate = check(v);
                assert!(matches!(
                    c.parent_oneof,
                    Some(types::certificate::ParentOneof::ParentSerialNumber(_))
                ));
                assert_eq!(c.not_after, u64::MAX);
            }
            "type_certificate_request" => {
                check::<types::CertificateRequest>(v);
            }
            "type_key" => {
                let k: types::Key = check(v);
                assert_eq!(k.r#type, types::KeyType::Ed25519 as i32);
            }
            "type_image_negative_enum" => {
                let i: types::Image = check(v);
                assert_eq!(i.r#type, -1);
            }
            "vnic_peer_init" | "vnic_peer_init_empty_certificate" => {
                let p: vnic::v1::PeerInit = check(v);
                assert!(p.certificate.is_some());
            }
            "vnic_aes_link_init" => {
                check::<vnic::v1::AesLinkInit>(v);
            }
            "vpn_hash_table_record" => {
                check::<vpn::v1::hash_table_message::Record>(v);
            }
            "vpn_hash_table_publish"
            | "vpn_hash_table_unpublish"
            | "vpn_hash_table_get_request"
            | "vpn_hash_table_get_response"
            | "vpn_hash_table_publish_empty_record" => {
                let m: vpn::v1::HashTableMessage = check(v);
                assert!(m.body.is_some());
            }
            "vpn_peer_index_record" => {
                check::<vpn::v1::peer_index_message::Record>(v);
            }
            "vpn_peer_index_publish"
            | "vpn_peer_index_search_request"
            | "vpn_peer_index_search_response" => {
                let m: vpn::v1::PeerIndexMessage = check(v);
                if let Some(vpn::v1::peer_index_message::Body::SearchResponse(r)) = &m.body {
                    assert_eq!(r.records.len(), 3);
                }
            }
            "vpn_peer_exchange_link_offer"
            | "vpn_peer_exchange_link_answer"
            | "vpn_peer_exchange_rejection" => {
                check::<vpn::v1::PeerExchangeMessage>(v);
            }
            "type_swarm_cache" => {
                let c: types::Cache = check(v);
                let integrity = c.integrity.as_ref().unwrap();
                assert_eq!(
                    integrity
                        .merkle_integrity
                        .as_ref()
                        .unwrap()
                        .signatures
                        .len(),
                    3
                );
            }
            "type_swarm_cache_meta" => {
                check::<types::CacheMeta>(v);
            }
            "chat_emote" => {
                let e: chat::v1::Emote = check(v);
                assert_eq!(e.effects.len(), 4);
            }
            "chat_modifier" => {
                let m: chat::v1::Modifier = check(v);
                assert_eq!(m.proc_chance, 0.125);
            }
            "directory_listing_snippet" => {
                check::<network::v1::directory::ListingSnippet>(v);
            }
            "directory_listing_snippet_delta" => {
                let d: network::v1::directory::ListingSnippetDelta = check(v);
                // wrappers distinguish "set to the default" from "unset"
                assert_eq!(d.title.as_deref(), Some(""));
                assert_eq!(d.live, Some(false));
                assert_eq!(d.category, None);
            }
            "devtools_capconn_event" => {
                check::<devtools::v1::ppspp::cap_conn_log::peer_log::Event>(v);
            }
            other => panic!("unhandled vector {other}"),
        }
    }
}
