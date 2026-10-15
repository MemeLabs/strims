// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors from the Go code (`golden/main.go`):
//!
//! - messages marshaled by up to five Go hosts in turn: re-encoded hop by
//!   hop from the same keys and times, byte for byte, signatures included;
//! - Go's `Message.Verify` on tampered copies;
//! - the AES-GCM message filter with fixed keys and nonces, both ways;
//! - signed hash table and peer index records and the service messages
//!   that carry them, and peer exchange messages.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;

use prost::Message as _;
use strims_base::time::Time;
use strims_crypto::{SignableMessage, key_from_seed, signing_key};
use strims_kademlia::ID;
use strims_proto::types::Key;
use strims_proto::vnic::v1::LinkDescription;
use strims_proto::vpn::v1::{
    HashTableMessage, PeerExchangeMessage, PeerIndexMessage, hash_table_message as ht,
    peer_exchange_message as px, peer_index_message as pi,
};
use strims_vpn::message::{MessageHeader, MessageId, encode_message, encode_relay};
use strims_vpn::message_filter::MessageCipher;
use strims_vpn::{MessageView, hash_table_record_hash, peer_index_record_hash};

const VECTORS: &str = include_str!("../golden/vectors.txt");

fn vectors(kind: &str) -> Vec<BTreeMap<&'static str, &'static str>> {
    VECTORS
        .lines()
        .filter_map(|l| {
            let mut parts = l.split(' ');
            (parts.next() == Some(kind)).then(|| {
                parts
                    .map(|kv| kv.split_once('=').unwrap())
                    .collect::<BTreeMap<_, _>>()
            })
        })
        .collect()
}

fn hex(s: &str) -> Vec<u8> {
    hex::decode(s).unwrap()
}

fn key(seed_hex: &str) -> Key {
    let seed: [u8; 32] = hex(seed_hex).try_into().unwrap();
    key_from_seed(&seed)
}

fn id_of(k: &Key) -> ID {
    ID::unmarshal(&k.public).unwrap().0
}

#[test]
fn messages_are_byte_identical_hop_by_hop() {
    let vs = vectors("message");
    assert_eq!(vs.len(), 4);
    for v in vs {
        let body = hex(v["body"]);
        let header = MessageHeader {
            dst_id: id_of(&key(v["dst_seed"])),
            dst_port: v["dst_port"].parse().unwrap(),
            src_port: v["src_port"].parse().unwrap(),
            seq: v["seq"].parse().unwrap(),
            length: 0,
            flags: v["flags"].parse().unwrap(),
        };
        let seeds: Vec<&str> = v["hop_seeds"].split(',').collect();
        let times: Vec<Time> = v["times"]
            .split(',')
            .map(|t| Time::from_unix_nanos(t.parse().unwrap()))
            .collect();
        let wires: Vec<Vec<u8>> = v["wires"].split(',').map(hex).collect();
        assert_eq!(seeds.len(), wires.len());

        let mut prev: Option<Vec<u8>> = None;
        for (hop, (seed, want)) in seeds.iter().zip(&wires).enumerate() {
            let k = key(seed);
            let sk = signing_key(&k).unwrap();
            let mut got = Vec::new();
            match &prev {
                None => {
                    encode_message(&mut got, &header, &body, times[hop], &id_of(&k), &sk).unwrap()
                }
                Some(p) => {
                    let m = MessageView::decode(p).unwrap();
                    encode_relay(&mut got, &m, times[hop], &id_of(&k), &sk);
                }
            }
            assert_eq!(hex::encode(&got), hex::encode(want), "hop {hop}");

            let m = MessageView::decode(want).unwrap();
            assert_eq!(m.raw(), &want[..]);
            assert_eq!(m.hops(), hop + 1);
            assert_eq!(m.body, &body[..]);
            assert_eq!(
                m.header,
                MessageHeader {
                    length: body.len() as u16,
                    ..header
                }
            );
            for j in 0..=hop {
                assert!(m.verify(j), "hop {hop} entry {j}");
                let e = m.entry(j).unwrap();
                assert_eq!(e.host_id, id_of(&key(seeds[j])));
                assert_eq!(e.time, times[j]);
            }
            assert!(!m.verify(hop + 1));
            prev = Some(got);
        }
        let last = MessageView::decode(wires.last().unwrap()).unwrap();
        assert_eq!(
            last.id().unwrap(),
            MessageId(hex(v["id"]).try_into().unwrap())
        );
        assert_eq!(last.src_host_id(), Some(id_of(&key(seeds[0]))));
    }
}

#[test]
fn verify_matches_go_on_tampered_messages() {
    let vs = vectors("verify");
    assert!(vs.len() >= 30);
    for v in vs {
        let mut wire = hex(v["wire"]);
        let pos: usize = v["pos"].parse().unwrap();
        wire[pos] ^= 1;
        let m = MessageView::decode(&wire);
        if v.get("decode") == Some(&"error") {
            assert!(m.is_err(), "pos {pos}");
            continue;
        }
        let m = m.unwrap();
        assert_eq!(m.hops(), v["hops"].parse::<usize>().unwrap(), "pos {pos}");
        let want: Vec<bool> = v["results"].split(',').map(|r| r == "true").collect();
        let got: Vec<bool> = (0..=m.hops()).map(|j| m.verify(j)).collect();
        assert_eq!(got, want, "pos {pos}");
    }
}

#[test]
fn message_cipher_matches_go() {
    let vs = vectors("cipher");
    assert_eq!(vs.len(), 4);
    for v in vs {
        let (a, b) = (key(v["a_seed"]), key(v["b_seed"]));
        let nonce: [u8; 12] = hex(v["nonce"]).try_into().unwrap();
        let plain = hex(v["plain"]);
        let sealed = hex(v["sealed"]);
        let mut got = Vec::new();
        MessageCipher::new(&a, &id_of(&b))
            .unwrap()
            .seal(&nonce, &plain, &mut got)
            .unwrap();
        assert_eq!(hex::encode(&got), hex::encode(&sealed));
        let mut opened = Vec::new();
        MessageCipher::new(&b, &id_of(&a))
            .unwrap()
            .open(&sealed, &mut opened)
            .unwrap();
        assert_eq!(opened, plain);
    }
}

#[test]
fn hash_table_records_and_messages() {
    let vs = vectors("ht_record");
    assert_eq!(vs.len(), 3);
    for v in vs {
        let k = key(v["seed"]);
        let mut r = ht::Record {
            salt: hex(v["salt"]),
            value: hex(v["value"]),
            timestamp: v["timestamp"].parse().unwrap(),
            ..Default::default()
        };
        r.sign_message(&k).unwrap();
        assert_eq!(hex::encode(r.encode_to_vec()), v["record"]);
        assert_eq!(
            hash_table_record_hash(&k.public, &r.salt).to_vec(),
            hex(v["hash"])
        );
        let decoded = ht::Record::decode(&hex(v["record"])[..]).unwrap();
        decoded.verify_message().unwrap();
        assert_eq!(decoded, r);

        let msg = |body| HashTableMessage { body: Some(body) }.encode_to_vec();
        let publish = msg(ht::Body::Publish(ht::Publish {
            record: Some(r.clone()),
        }));
        let unpublish = msg(ht::Body::Unpublish(ht::Unpublish {
            record: Some(r.clone()),
        }));
        let get = msg(ht::Body::GetRequest(ht::GetRequest {
            request_id: 0x0123_4567_89ab_cdef,
            hash: hex(v["hash"]),
            if_modified_since: r.timestamp - 1,
        }));
        let res = msg(ht::Body::GetResponse(ht::GetResponse {
            request_id: 0xfedc_ba98_7654_3210,
            record: Some(r.clone()),
        }));
        for (got, name) in [
            (publish, "publish"),
            (unpublish, "unpublish"),
            (get, "get_request"),
            (res, "get_response"),
        ] {
            assert_eq!(hex::encode(&got), v[name], "{name}");
            let m = HashTableMessage::decode(&got[..]).unwrap();
            assert_eq!(m.encode_to_vec(), got);
        }
    }
}

#[test]
fn peer_index_records_and_messages() {
    let vs = vectors("pi_record");
    assert_eq!(vs.len(), 2);
    let mut records = Vec::new();
    for v in vs {
        let host = key(v["seed"]);
        let hash = peer_index_record_hash(&hex(v["key"]), &hex(v["salt"]));
        let mut r = pi::Record {
            hash: hash.to_vec(),
            host_id: host.public.clone(),
            port: v["port"].parse().unwrap(),
            timestamp: v["timestamp"].parse().unwrap(),
            ..Default::default()
        };
        r.sign_message(&host).unwrap();
        assert_eq!(hex::encode(r.encode_to_vec()), v["record"]);
        pi::Record::decode(&hex(v["record"])[..])
            .unwrap()
            .verify_message()
            .unwrap();
        let msg = |body| PeerIndexMessage { body: Some(body) }.encode_to_vec();
        assert_eq!(
            hex::encode(msg(pi::Body::Publish(pi::Publish {
                record: Some(r.clone())
            }))),
            v["publish"]
        );
        assert_eq!(
            hex::encode(msg(pi::Body::Unpublish(pi::Unpublish {
                record: Some(r.clone())
            }))),
            v["unpublish"]
        );
        assert_eq!(
            hex::encode(msg(pi::Body::SearchRequest(pi::SearchRequest {
                request_id: 99,
                hash: hash.to_vec(),
            }))),
            v["search_request"]
        );
        records.push(r);
    }
    let res = PeerIndexMessage {
        body: Some(pi::Body::SearchResponse(pi::SearchResponse {
            request_id: 99,
            records,
        })),
    };
    assert_eq!(
        hex::encode(res.encode_to_vec()),
        vectors("pi_search_response")[0]["response"]
    );
}

#[test]
fn peer_exchange_messages() {
    let v = &vectors("px")[0];
    let descs = vec![
        LinkDescription {
            interface: "tcp".into(),
            description: "tcp://127.0.0.1:8080/abcd".into(),
        },
        LinkDescription {
            interface: "webrtc".into(),
            description: r#"{"type":"offer","sdp":"v=0\r\n"}"#.into(),
        },
    ];
    let msg = |body| PeerExchangeMessage { body: Some(body) }.encode_to_vec();
    let offer = msg(px::Body::LinkOffer(px::LinkOffer {
        exchange_id: 0x1122_3344_5566_7788,
        descriptions: descs.clone(),
    }));
    let answer = msg(px::Body::LinkAnswer(px::LinkAnswer {
        exchange_id: 0x1122_3344_5566_7788,
        descriptions: descs[1..].to_vec(),
        error_message: String::new(),
    }));
    let reject = msg(px::Body::LinkAnswer(px::LinkAnswer {
        exchange_id: 7,
        descriptions: vec![],
        error_message: strims_vpn::peer_exchange::ERR_NO_NEW_PEERS.into(),
    }));
    assert_eq!(hex::encode(offer), v["offer"]);
    assert_eq!(hex::encode(answer), v["answer"]);
    assert_eq!(hex::encode(reject), v["reject"]);
}
