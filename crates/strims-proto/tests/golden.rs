// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors (`golden/vectors.hex`): the protobuf and stable encodings
//! of messages built here. Each line is `<name> <protobuf hex> <stable
//! hex>`, with `=` for an empty encoding and `-` for a message without a
//! stable encoding. prost must decode each encoding back to the message
//! and re-encode it identically. `STRIMS_GOLDEN=update` rewrites the
//! vectors from the code (see `strims-golden`).

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

use prost::Message;
use strims_golden::Golden;
use strims_proto::chat::v1::{
    Emote, EmoteContributor, EmoteEffect, EmoteFileType, EmoteImage, EmoteScale, Modifier,
    StyleSheet, emote_effect, style_sheet,
};
use strims_proto::debug::v1::{
    MetricsFormat, PProfRequest, PProfResponse, ReadMetricsRequest, WatchMetricsRequest,
};
use strims_proto::devtools::v1::ppspp::cap_conn_log::peer_log::{Event, event};
use strims_proto::network::v1::directory::{
    ListingSnippet, ListingSnippetDelta, ListingSnippetImage, listing_snippet_delta,
    listing_snippet_image,
};
use strims_proto::rpc;
use strims_proto::rpc::call::Kind;
use strims_proto::stable::StableEncode;
use strims_proto::types::certificate::ParentOneof;
use strims_proto::types::{
    Cache, CacheMeta, Certificate, CertificateRequest, Image, ImageType, Key, KeyType, KeyUsage,
    cache,
};
use strims_proto::vnic::v1::{AesLinkInit, LinkDescription, PeerInit};
use strims_proto::vpn::v1::{
    HashTableMessage, PeerExchangeMessage, PeerIndexMessage, hash_table_message,
    peer_exchange_message, peer_index_message,
};

const ED25519: i32 = KeyType::Ed25519 as i32;
const PEER: i32 = KeyUsage::Peer as i32;
const BOOTSTRAP: i32 = KeyUsage::Bootstrap as i32;
const SIGN: i32 = KeyUsage::Sign as i32;
const ENCIPHERMENT: i32 = KeyUsage::Encipherment as i32;

/// The vector file and the entries pinned so far.
struct Vectors {
    g: Golden,
    pinned: BTreeSet<String>,
}

impl Vectors {
    fn open() -> Vectors {
        Vectors {
            g: Golden::open(concat!(env!("CARGO_MANIFEST_DIR"), "/golden/vectors.hex")),
            pinned: BTreeSet::new(),
        }
    }

    /// Pins `m`'s protobuf and stable encodings under `name`.
    fn pin<M: Message + Default + PartialEq + Debug + StableEncode>(&mut self, name: &str, m: &M) {
        let stable = m.stable_encode_to_vec();
        assert_eq!(
            m.stable_encoded_len(),
            stable.len(),
            "{name}: stable_encoded_len"
        );
        self.pin_with(name, m, hex::encode(stable));
    }

    /// Pins the protobuf encoding of `m`, a message without a stable
    /// encoding (it contains a map).
    fn pin_proto<M: Message + Default + PartialEq + Debug>(&mut self, name: &str, m: &M) {
        self.pin_with(name, m, "-".to_owned());
    }

    fn pin_with<M: Message + Default + PartialEq + Debug>(
        &mut self,
        name: &str,
        m: &M,
        stable: String,
    ) {
        assert!(self.pinned.insert(name.to_owned()), "{name} pinned twice");
        let proto = m.encode_to_vec();
        assert_eq!(m.encoded_len(), proto.len(), "{name}: encoded_len");
        let back = M::decode(proto.as_slice()).unwrap_or_else(|e| panic!("{name}: decode: {e}"));
        assert_eq!(&back, m, "{name}: decode");
        assert_eq!(back.encode_to_vec(), proto, "{name}: re-encode");
        let e = self.g.entry(name);
        self.g.check_hex(e, 0, proto);
        self.g.check(e, 1, stable);
    }

    /// Fails on entries nothing pinned; update mode removes them.
    fn finish(mut self) {
        let stale: BTreeSet<String> = self
            .g
            .prefixed("")
            .into_iter()
            .map(|e| self.g.name(e).to_owned())
            .filter(|n| !self.pinned.contains(n))
            .collect();
        for name in stale {
            self.g.remove(&name);
        }
    }
}

/// `n` bytes counting up from `start`, wrapping.
fn seq(start: u8, n: usize) -> Vec<u8> {
    std::iter::successors(Some(start), |b| Some(b.wrapping_add(1)))
        .take(n)
        .collect()
}

fn strings(s: &[&str]) -> Vec<String> {
    s.iter().map(|&s| s.to_owned()).collect()
}

fn root_cert() -> Certificate {
    Certificate {
        key: seq(0x10, 32),
        key_type: ED25519,
        key_usage: SIGN,
        subject: "root".to_owned(),
        not_before: 1_600_000_000,
        not_after: 1_900_000_000,
        serial_number: seq(0x30, 16),
        signature: seq(0x40, 64),
        parent_oneof: None,
    }
}

/// leaf -> intermediate -> root.
fn cert_chain() -> Certificate {
    let intermediate = Certificate {
        key: seq(0x50, 32),
        key_type: ED25519,
        key_usage: SIGN | PEER,
        subject: "intermediate".to_owned(),
        not_before: 1_600_000_100,
        not_after: 1_800_000_000,
        serial_number: seq(0x70, 16),
        signature: seq(0x80, 64),
        parent_oneof: Some(ParentOneof::Parent(Box::new(root_cert()))),
    };
    Certificate {
        key: seq(0xa0, 32),
        key_type: ED25519,
        key_usage: PEER | BOOTSTRAP,
        subject: "leaf ✓".to_owned(),
        not_before: 1_700_000_000,
        not_after: 1_700_086_400,
        serial_number: seq(0xc0, 16),
        signature: seq(0x01, 64),
        parent_oneof: Some(ParentOneof::Parent(Box::new(intermediate))),
    }
}

fn certificates(v: &mut Vectors) {
    v.pin("type_certificate_chain", &cert_chain());
    v.pin(
        "type_certificate_parent_serial",
        &Certificate {
            key: seq(0x01, 32),
            key_type: ED25519,
            key_usage: PEER,
            subject: String::new(),
            not_before: 1,
            not_after: u64::MAX,
            serial_number: seq(0x02, 16),
            signature: seq(0x03, 64),
            parent_oneof: Some(ParentOneof::ParentSerialNumber(seq(0x30, 16))),
        },
    );
    v.pin(
        "type_certificate_request",
        &CertificateRequest {
            key: seq(0x10, 32),
            key_type: ED25519,
            key_usage: PEER | ENCIPHERMENT,
            subject: "csr".to_owned(),
            signature: seq(0x20, 64),
        },
    );
    v.pin(
        "type_key",
        &Key {
            r#type: ED25519,
            private: seq(0x00, 64),
            public: seq(0x20, 32),
        },
    );
    v.pin(
        "type_image_negative_enum",
        &Image {
            r#type: -1,
            height: 1,
            width: 2,
            data: Vec::new(),
        },
    );
}

fn vnic_messages(v: &mut Vectors) {
    v.pin(
        "vnic_peer_init",
        &PeerInit {
            protocol_version: 1,
            certificate: Some(cert_chain()),
            node_platform: "linux".to_owned(),
            node_version: "v1.2.3".to_owned(),
        },
    );
    v.pin(
        "vnic_peer_init_empty_certificate",
        &PeerInit {
            protocol_version: 1,
            certificate: Some(Certificate::default()),
            ..PeerInit::default()
        },
    );
    v.pin(
        "vnic_aes_link_init",
        &AesLinkInit {
            protocol_version: 1,
            key: seq(0x40, 32),
            iv: seq(0x60, 16),
        },
    );
}

fn hash_table_record() -> hash_table_message::Record {
    hash_table_message::Record {
        key: seq(0x11, 32),
        salt: b"salt".to_vec(),
        value: seq(0x00, 200),
        timestamp: 1_700_000_000_123,
        signature: seq(0x22, 64),
    }
}

fn peer_index_record(i: u8) -> peer_index_message::Record {
    peer_index_message::Record {
        hash: seq(i, 32),
        host_id: seq(i + 1, 32),
        port: 1000 + u32::from(i),
        timestamp: -i64::from(i) * 1000,
        key: seq(i + 2, 32),
        signature: seq(i + 3, 64),
    }
}

fn vpn_messages(v: &mut Vectors) {
    use hash_table_message as ht;
    use peer_exchange_message as px;
    use peer_index_message as pi;

    let hash_table = |body| HashTableMessage { body: Some(body) };
    v.pin("vpn_hash_table_record", &hash_table_record());
    v.pin(
        "vpn_hash_table_publish",
        &hash_table(ht::Body::Publish(ht::Publish {
            record: Some(hash_table_record()),
        })),
    );
    v.pin(
        "vpn_hash_table_unpublish",
        &hash_table(ht::Body::Unpublish(ht::Unpublish {
            record: Some(hash_table_record()),
        })),
    );
    v.pin(
        "vpn_hash_table_get_request",
        &hash_table(ht::Body::GetRequest(ht::GetRequest {
            request_id: u64::MAX - 1,
            hash: seq(0x33, 32),
            if_modified_since: -5,
        })),
    );
    v.pin(
        "vpn_hash_table_get_response",
        &hash_table(ht::Body::GetResponse(ht::GetResponse {
            request_id: 7,
            record: Some(hash_table_record()),
        })),
    );
    v.pin(
        "vpn_hash_table_publish_empty_record",
        &hash_table(ht::Body::Publish(ht::Publish {
            record: Some(ht::Record::default()),
        })),
    );

    let peer_index = |body| PeerIndexMessage { body: Some(body) };
    v.pin("vpn_peer_index_record", &peer_index_record(5));
    v.pin(
        "vpn_peer_index_publish",
        &peer_index(pi::Body::Publish(pi::Publish {
            record: Some(peer_index_record(1)),
        })),
    );
    v.pin(
        "vpn_peer_index_search_request",
        &peer_index(pi::Body::SearchRequest(pi::SearchRequest {
            request_id: 99,
            hash: seq(0x44, 32),
        })),
    );
    v.pin(
        "vpn_peer_index_search_response",
        &peer_index(pi::Body::SearchResponse(pi::SearchResponse {
            request_id: 99,
            records: vec![
                peer_index_record(1),
                peer_index_record(100),
                peer_index_record(200),
            ],
        })),
    );

    let link = |interface: &str, description: &str| LinkDescription {
        interface: interface.to_owned(),
        description: description.to_owned(),
    };
    let peer_exchange = |body| PeerExchangeMessage { body: Some(body) };
    v.pin(
        "vpn_peer_exchange_link_offer",
        &peer_exchange(px::Body::LinkOffer(px::LinkOffer {
            exchange_id: 1 << 40,
            descriptions: vec![
                link("webrtc", r#"{"type":"offer","sdp":"v=0"}"#),
                link("tcp", "tcp://127.0.0.1:8083/abcd"),
            ],
        })),
    );
    v.pin(
        "vpn_peer_exchange_link_answer",
        &peer_exchange(px::Body::LinkAnswer(px::LinkAnswer {
            exchange_id: 2,
            descriptions: vec![link("webrtc", "answer")],
            error_message: "partial failure".to_owned(),
        })),
    );
    v.pin(
        "vpn_peer_exchange_rejection",
        &peer_exchange(px::Body::Rejection(px::Rejection { mediation_id: 3 })),
    );
}

fn swarm_caches(v: &mut Vectors) {
    v.pin(
        "type_swarm_cache",
        &Cache {
            id: 42,
            uri: "magnet:?xt=urn:ppspp:abcd".to_owned(),
            integrity: Some(cache::Integrity {
                sign_all_integrity: Some(cache::SignAllIntegrity {
                    timestamps: vec![1, -2, 1 << 40],
                    signatures: seq(0x00, 128),
                }),
                merkle_integrity: Some(cache::MerkleIntegrity {
                    timestamps: vec![3],
                    signatures: vec![seq(0x01, 64), Vec::new(), seq(0x02, 64)],
                }),
            }),
            data: seq(0x10, 300),
            epoch: Some(cache::Epoch {
                timestamp: 1_700_000_000_000,
                signature: seq(0x05, 64),
            }),
        },
    );
    v.pin(
        "type_swarm_cache_meta",
        &CacheMeta {
            id: 1,
            swarm_id: seq(0x01, 32),
            swarm_salt: b"salt".to_vec(),
            checksum: 0xdead_beef,
        },
    );
}

fn style_sheet() -> StyleSheet {
    StyleSheet {
        scss: ".a { b: c }".to_owned(),
        css: ".a{b:c}".to_owned(),
        assets: vec![
            style_sheet::Asset {
                name: "one".to_owned(),
                image: Some(Image {
                    r#type: ImageType::Png as i32,
                    height: 28,
                    width: 32,
                    data: seq(0x89, 8),
                }),
            },
            style_sheet::Asset {
                name: "two".to_owned(),
                image: None,
            },
        ],
    }
}

fn chat_messages(v: &mut Vectors) {
    let effect = |e| EmoteEffect { effect: Some(e) };
    v.pin(
        "chat_emote",
        &Emote {
            id: 1,
            server_id: 2,
            name: "PEPE".to_owned(),
            images: vec![
                EmoteImage {
                    data: seq(0x00, 10),
                    file_type: EmoteFileType::FileTypePng as i32,
                    height: 28,
                    width: 28,
                    scale: EmoteScale::EmoteScale1x as i32,
                },
                EmoteImage {
                    data: seq(0x10, 20),
                    file_type: EmoteFileType::FileTypeGif as i32,
                    height: 56,
                    width: 56,
                    scale: EmoteScale::EmoteScale2x as i32,
                },
            ],
            effects: vec![
                effect(emote_effect::Effect::CustomCss(emote_effect::CustomCss {
                    style_sheet: Some(style_sheet()),
                    extra_wrap_count: 2,
                    wrap_adjacent: true,
                })),
                effect(emote_effect::Effect::SpriteAnimation(
                    emote_effect::SpriteAnimation {
                        frame_count: 4,
                        duration_ms: 400,
                        loop_forever: true,
                        ..emote_effect::SpriteAnimation::default()
                    },
                )),
                effect(emote_effect::Effect::DefaultModifiers(
                    emote_effect::DefaultModifiers {
                        modifiers: strings(&["wide", "spin"]),
                    },
                )),
                EmoteEffect::default(),
            ],
            contributor: Some(EmoteContributor {
                name: "someone".to_owned(),
                link: "https://example.com".to_owned(),
            }),
            labels: strings(&["a", "", "b"]),
            enable: true,
        },
    );
    v.pin(
        "chat_modifier",
        &Modifier {
            id: 3,
            server_id: 4,
            name: "spin".to_owned(),
            priority: 10,
            internal: true,
            extra_wrap_count: 1,
            proc_chance: 0.125,
            style_sheet: Some(style_sheet()),
        },
    );
}

fn directory_snippets(v: &mut Vectors) {
    use listing_snippet_image::SourceOneof;

    let image = |source| ListingSnippetImage {
        source_oneof: Some(source),
    };
    v.pin(
        "directory_listing_snippet",
        &ListingSnippet {
            title: "title".to_owned(),
            description: "description".to_owned(),
            tags: strings(&["one", "two"]),
            category: "category".to_owned(),
            channel_name: "channel".to_owned(),
            user_count: 12345,
            live: true,
            is_mature: false,
            thumbnail: Some(image(SourceOneof::Image(Image {
                r#type: ImageType::Jpeg as i32,
                height: 720,
                width: 1280,
                data: seq(0xff, 16),
            }))),
            channel_logo: Some(image(SourceOneof::Url(
                "https://example.com/logo.png".to_owned(),
            ))),
            video_height: 720,
            video_width: 1280,
            theme_color: 0xff00_ff00,
            start_time: -1,
            key: seq(0x07, 32),
            signature: seq(0x08, 64),
        },
    );
    // Wrappers set to their defaults (`title`, `user_count`, `live`) encode
    // as set, distinct from the unset ones.
    v.pin(
        "directory_listing_snippet_delta",
        &ListingSnippetDelta {
            title: Some(String::new()),
            description: Some("new description".to_owned()),
            user_count: Some(0),
            live: Some(false),
            is_mature: Some(true),
            key: Some(seq(0x01, 32)),
            theme_color: Some(7),
            start_time: Some(-100),
            tags_oneof: Some(listing_snippet_delta::TagsOneof::Tags(
                listing_snippet_delta::Tags {
                    tags: strings(&["x"]),
                },
            )),
            thumbnail_oneof: Some(listing_snippet_delta::ThumbnailOneof::Thumbnail(
                ListingSnippetImage::default(),
            )),
            ..ListingSnippetDelta::default()
        },
    );
}

fn devtools_events(v: &mut Vectors) {
    v.pin(
        "devtools_capconn_event",
        &Event {
            code: event::Code::EventCodeReadErr as i32,
            timestamp: -1_700_000_000_000_000_000,
            message_types: vec![
                event::MessageType::Data as i32,
                event::MessageType::End as i32,
                0,
            ],
            message_addresses: vec![0, 1, u64::MAX],
        },
    );
}

/// A call carrying `arg`, encoded, and no headers.
fn rpc_call(id: u64, parent_id: u64, method: &str, kind: Kind, arg: &impl Message) -> rpc::Call {
    rpc::Call {
        id,
        parent_id,
        method: method.to_owned(),
        kind: kind as i32,
        argument: arg.encode_to_vec(),
        headers: BTreeMap::new(),
    }
}

fn rpc_messages(v: &mut Vectors) {
    let error = |message: &str, code| rpc::Error {
        message: message.to_owned(),
        code,
    };
    v.pin_proto(
        "rpc_call_request",
        &rpc_call(
            1,
            0,
            "strims.debug.v1.Debug.PProf",
            Kind::CallKindDefault,
            &PProfRequest {
                name: "heap".to_owned(),
                debug: true,
                gc: false,
            },
        ),
    );
    v.pin_proto(
        "rpc_call_callback",
        &rpc_call(
            2,
            1,
            "CALLBACK",
            Kind::CallKindDefault,
            &PProfResponse {
                name: "heap".to_owned(),
                data: seq(0x00, 40),
            },
        ),
    );
    v.pin_proto(
        "rpc_call_error",
        &rpc_call(3, 1, "CALLBACK", Kind::CallKindError, &error("boom", 5)),
    );
    v.pin_proto(
        "rpc_call_error_negative_code",
        &rpc_call(
            4,
            1,
            "CALLBACK",
            Kind::CallKindError,
            &error("negative", -3),
        ),
    );
    v.pin_proto(
        "rpc_call_close",
        &rpc_call(5, 1, "CALLBACK", Kind::CallKindClose, &rpc::Close {}),
    );
    v.pin_proto(
        "rpc_call_undefined",
        &rpc_call(
            6,
            1,
            "CALLBACK",
            Kind::CallKindUndefined,
            &rpc::Undefined {},
        ),
    );
    v.pin_proto(
        "rpc_call_cancel",
        &rpc_call(7, 1, "CANCEL", Kind::CallKindCancel, &rpc::Cancel {}),
    );
    v.pin_proto(
        "rpc_call_cancel_default_kind",
        &rpc_call(8, 1, "CANCEL", Kind::CallKindDefault, &rpc::Cancel {}),
    );
    let mut header = rpc_call(
        9,
        0,
        "strims.debug.v1.Debug.ReadMetrics",
        Kind::CallKindDefault,
        &ReadMetricsRequest {
            format: MetricsFormat::ProtoDelim as i32,
        },
    );
    header
        .headers
        .insert("certificate".to_owned(), cert_chain().encode_to_vec());
    v.pin_proto("rpc_call_header", &header);
    v.pin_proto(
        "rpc_call_max_ids",
        &rpc_call(
            u64::MAX,
            u64::MAX - 1,
            "strims.debug.v1.Debug.WatchMetrics",
            Kind::CallKindDefault,
            &WatchMetricsRequest {
                format: MetricsFormat::Text as i32,
                interval_ms: -1,
            },
        ),
    );
    v.pin_proto("rpc_call_empty", &rpc::Call::default());
    v.pin_proto(
        "rpc_error",
        &error("method not found: strims.debug.v1.Debug.Nope", 7),
    );
}

#[test]
fn messages_keep_their_encodings() {
    let mut v = Vectors::open();
    certificates(&mut v);
    vnic_messages(&mut v);
    vpn_messages(&mut v);
    swarm_caches(&mut v);
    chat_messages(&mut v);
    directory_snippets(&mut v);
    devtools_events(&mut v);
    rpc_messages(&mut v);
    v.finish();
}
