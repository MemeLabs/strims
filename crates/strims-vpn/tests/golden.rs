// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors (`golden/vectors.txt`):
//!
//! - messages marshaled by up to five hosts in turn, hop by hop from the
//!   same keys and times, signatures included;
//! - verification results on tampered copies of a message's last wire
//!   (`verify message=<n>` tampers with the `n`th `message` vector, from 0);
//! - the AES-GCM message filter with fixed keys and nonces, both ways;
//! - signed hash table and peer index records and the service messages
//!   that carry them, and peer exchange messages.
//!
//! `STRIMS_GOLDEN=update` rewrites the computed fields from the code (see
//! `strims-golden`). Verification results and peer exchange messages are
//! asserted, never rewritten.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fmt::Debug;
use std::str::FromStr;

use prost::Message as _;
use strims_base::time::Time;
use strims_crypto::{Signer, Verified, key_from_seed};
use strims_golden::Golden;
use strims_kademlia::ID;
use strims_proto::types::Key;
use strims_proto::vnic::v1::LinkDescription;
use strims_proto::vpn::v1::{
    HashTableMessage, PeerExchangeMessage, PeerIndexMessage, hash_table_message as ht,
    peer_exchange_message as px, peer_index_message as pi,
};
use strims_vpn::message::{MessageHeader, encode_message, encode_relay};
use strims_vpn::message_filter::MessageCipher;
use strims_vpn::{MessageView, hash_table_record_hash, peer_index_record_hash};

fn open() -> Golden {
    Golden::open(concat!(env!("CARGO_MANIFEST_DIR"), "/golden/vectors.txt"))
}

fn num<T: FromStr>(g: &Golden, e: usize, k: &str) -> T
where
    T::Err: Debug,
{
    g.str(e, k).parse().unwrap()
}

fn key(seed: &[u8]) -> Key {
    key_from_seed(&seed.try_into().unwrap())
}

fn id_of(k: &Key) -> ID {
    ID::unmarshal(&k.public).unwrap().0
}

/// A `message` vector's inputs.
struct MessageCase {
    header: MessageHeader,
    body: Vec<u8>,
    hosts: Vec<Key>,
    times: Vec<Time>,
}

impl MessageCase {
    fn read(g: &Golden, e: usize) -> Self {
        let hosts: Vec<Key> = g
            .str(e, "hop_seeds")
            .split(',')
            .map(|s| key(&hex::decode(s).unwrap()))
            .collect();
        let times: Vec<Time> = g
            .str(e, "times")
            .split(',')
            .map(|t| Time::from_unix_nanos(t.parse().unwrap()))
            .collect();
        assert_eq!(hosts.len(), times.len());
        MessageCase {
            header: MessageHeader {
                dst_id: id_of(&key(&g.bytes(e, "dst_seed"))),
                dst_port: num(g, e, "dst_port"),
                src_port: num(g, e, "src_port"),
                seq: num(g, e, "seq"),
                length: 0,
                flags: num(g, e, "flags"),
            },
            body: g.bytes(e, "body"),
            hosts,
            times,
        }
    }

    /// The wire after each hop: the first host marshals the message and
    /// each later one relays the previous hop's wire.
    fn wires(&self) -> Vec<Vec<u8>> {
        let mut wires: Vec<Vec<u8>> = Vec::new();
        for (k, &time) in self.hosts.iter().zip(&self.times) {
            let sk = Signer::new(k).unwrap();
            let mut wire = Vec::new();
            match wires.last() {
                None => encode_message(&mut wire, &self.header, &self.body, time, &id_of(k), &sk)
                    .unwrap(),
                Some(prev) => {
                    let m = MessageView::decode(prev).unwrap();
                    encode_relay(&mut wire, &m, time, &id_of(k), &sk);
                }
            }
            wires.push(wire);
        }
        wires
    }
}

#[test]
fn messages_are_byte_identical_hop_by_hop() {
    let mut g = open();
    let entries = g.all("message");
    assert_eq!(entries.len(), 4);
    for e in entries {
        let c = MessageCase::read(&g, e);
        let wires = c.wires();
        for (hop, wire) in wires.iter().enumerate() {
            let m = MessageView::decode(wire).unwrap();
            assert_eq!(m.raw(), &wire[..]);
            assert_eq!(m.hops(), hop + 1);
            assert_eq!(m.body, &c.body[..]);
            assert_eq!(
                m.header,
                MessageHeader {
                    length: c.body.len() as u16,
                    ..c.header
                }
            );
            for j in 0..=hop {
                assert!(m.verify(j), "hop {hop} entry {j}");
                let entry = m.entry(j).unwrap();
                assert_eq!(entry.host_id, id_of(&c.hosts[j]));
                assert_eq!(entry.time, c.times[j]);
            }
            assert!(!m.verify(hop + 1));
        }
        let last = MessageView::decode(wires.last().unwrap()).unwrap();
        assert_eq!(last.src_host_id(), Some(id_of(&c.hosts[0])));
        g.check_hex(e, "id", last.id().unwrap().0);
        let wires: Vec<String> = wires.iter().map(hex::encode).collect();
        g.check(e, "wires", wires.join(","));
    }
}

#[test]
fn tampered_bytes_fail_every_entry_that_signs_them() {
    let g = open();
    let last_wires: Vec<Vec<u8>> = g
        .all("message")
        .into_iter()
        .map(|e| MessageCase::read(&g, e).wires().pop().unwrap())
        .collect();
    let entries = g.all("verify");
    assert_eq!(entries.len(), 37);
    for e in entries {
        let mut wire = last_wires[num::<usize>(&g, e, "message")].clone();
        let pos: usize = num(&g, e, "pos");
        wire[pos] ^= 1;
        let m = MessageView::decode(&wire);
        if g.opt(e, "decode") == Some("error") {
            assert!(m.is_err(), "pos {pos}");
            continue;
        }
        let m = m.unwrap();
        assert_eq!(m.hops(), num::<usize>(&g, e, "hops"), "pos {pos}");
        let want: Vec<bool> = g
            .str(e, "results")
            .split(',')
            .map(|r| r == "true")
            .collect();
        let got: Vec<bool> = (0..=m.hops()).map(|j| m.verify(j)).collect();
        assert_eq!(got, want, "pos {pos}");
    }
}

#[test]
fn message_cipher_matches_vectors() {
    let mut g = open();
    let entries = g.all("cipher");
    assert_eq!(entries.len(), 4);
    for e in entries {
        let (a, b) = (key(&g.bytes(e, "a_seed")), key(&g.bytes(e, "b_seed")));
        let nonce: [u8; 12] = g.bytes(e, "nonce").try_into().unwrap();
        let plain = g.bytes(e, "plain");
        let mut sealed = Vec::new();
        MessageCipher::new(&a, &id_of(&b))
            .unwrap()
            .seal(&nonce, &plain, &mut sealed)
            .unwrap();
        let mut opened = Vec::new();
        MessageCipher::new(&b, &id_of(&a))
            .unwrap()
            .open(&sealed, &mut opened)
            .unwrap();
        assert_eq!(opened, plain);
        g.check_hex(e, "sealed", &sealed);
    }
}

#[test]
fn hash_table_records_and_messages() {
    let mut g = open();
    let entries = g.all("ht_record");
    assert_eq!(entries.len(), 3);
    for e in entries {
        let k = key(&g.bytes(e, "seed"));
        let signer = Signer::new(&k).unwrap();
        let body = ht::RecordBody {
            key: k.public.clone(),
            salt: g.bytes(e, "salt"),
            value: g.bytes(e, "value"),
            timestamp: num(&g, e, "timestamp"),
            unpublish: false,
        };
        let r = Verified::sign(&signer, body.clone()).into_envelope();
        let gone = Verified::sign(
            &signer,
            ht::RecordBody {
                unpublish: true,
                ..body.clone()
            },
        )
        .into_envelope();
        let hash = hash_table_record_hash(&k.public, &body.salt);
        let record = r.encode_to_vec();
        let decoded = ht::Record::decode(&record[..]).unwrap();
        assert_eq!(*Verified::<ht::RecordBody>::open(decoded).unwrap(), body);
        g.check_hex(e, "hash", hash);
        g.check_hex(e, "record", &record);

        let msg = |body| HashTableMessage { body: Some(body) }.encode_to_vec();
        let publish = msg(ht::Body::Publish(ht::Publish {
            record: Some(r.clone()),
        }));
        let unpublish = msg(ht::Body::Unpublish(ht::Unpublish { record: Some(gone) }));
        let get = msg(ht::Body::GetRequest(ht::GetRequest {
            request_id: 0x0123_4567_89ab_cdef,
            hash: hash.to_vec(),
            if_modified_since: body.timestamp - 1,
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
            let m = HashTableMessage::decode(&got[..]).unwrap();
            assert_eq!(m.encode_to_vec(), got, "{name}");
            g.check_hex(e, name, &got);
        }
    }
}

#[test]
fn peer_index_records_and_messages() {
    let mut g = open();
    let entries = g.all("pi_record");
    assert_eq!(entries.len(), 2);
    let mut records = Vec::new();
    for e in entries {
        let host = key(&g.bytes(e, "seed"));
        let signer = Signer::new(&host).unwrap();
        let hash = peer_index_record_hash(&g.bytes(e, "key"), &g.bytes(e, "salt"));
        let body = pi::RecordBody {
            hash: hash.to_vec(),
            host_id: host.public.clone(),
            port: num(&g, e, "port"),
            timestamp: num(&g, e, "timestamp"),
            unpublish: false,
        };
        let r = Verified::sign(&signer, body.clone()).into_envelope();
        let gone = Verified::sign(
            &signer,
            pi::RecordBody {
                unpublish: true,
                ..body.clone()
            },
        )
        .into_envelope();
        let record = r.encode_to_vec();
        let decoded = pi::Record::decode(&record[..]).unwrap();
        assert_eq!(*Verified::<pi::RecordBody>::open(decoded).unwrap(), body);
        g.check_hex(e, "record", &record);

        let msg = |body| PeerIndexMessage { body: Some(body) }.encode_to_vec();
        let publish = msg(pi::Body::Publish(pi::Publish {
            record: Some(r.clone()),
        }));
        let unpublish = msg(pi::Body::Unpublish(pi::Unpublish { record: Some(gone) }));
        let search = msg(pi::Body::SearchRequest(pi::SearchRequest {
            request_id: 99,
            hash: hash.to_vec(),
        }));
        g.check_hex(e, "publish", publish);
        g.check_hex(e, "unpublish", unpublish);
        g.check_hex(e, "search_request", search);
        records.push(r);
    }
    let res = PeerIndexMessage {
        body: Some(pi::Body::SearchResponse(pi::SearchResponse {
            request_id: 99,
            records,
        })),
    };
    let e = g.entry("pi_search_response");
    g.check_hex(e, "response", res.encode_to_vec());
}

#[test]
fn peer_exchange_messages() {
    let g = open();
    let e = g.all("px")[0];
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
    assert_eq!(hex::encode(offer), g.str(e, "offer"));
    assert_eq!(hex::encode(answer), g.str(e, "answer"));
    assert_eq!(hex::encode(reject), g.str(e, "reject"));
}
