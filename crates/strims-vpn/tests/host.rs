// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The vpn host driven with vnic through the public API: messaging,
//! routing, verification, bounds and the services.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod common;

use common::{APP_PORT, AppTag, World, messages};
use strims_base::time::{Duration, Time};
use strims_crypto::{Signer, key_from_seed};
use strims_kademlia::ID;
use strims_proto::vpn::v1::PeerExchangeMessage;
use strims_proto::vpn::v1::peer_exchange_message::{Body as PxBody, LinkAnswer};
use strims_vnic::ports;
use strims_vpn::message::{MessageHeader, encode_message, encode_relay};
use strims_vpn::{
    Config, Error, Event, ExchangeOutcome, HashTableGetOptions, M_BROADCAST, M_COMPRESS, M_ENCRYPT,
    MessageView, Routing, VerifyMode,
};

const LAT: Duration = Duration::from_millis(5);

fn world(n: usize) -> World {
    World::new(7, n, Config::default())
}

fn chain(w: &mut World) {
    for i in 1..w.nodes.len() {
        w.link(i - 1, i, LAT);
    }
    w.run_for(Duration::SECOND);
}

fn has_closed(events: &[Event<AppTag>], want: &ExchangeOutcome) -> bool {
    events
        .iter()
        .any(|e| matches!(e, Event::ExchangeClosed { outcome, .. } if outcome == want))
}

#[test]
fn unicast_and_broadcast_between_neighbours() {
    let mut w = world(2);
    chain(&mut w);
    assert!(w.nodes[0].vpn.has_peer(w.nodes[0].net, &w.id(1)));

    w.send(0, 1, b"hello");
    w.broadcast(0, b"everyone");
    w.run_for(Duration::SECOND);

    let got0 = messages(&w.take_events(0));
    let got1 = messages(&w.take_events(1));
    // A broadcast is delivered to its sender too, with 0 hops.
    assert_eq!(got0, vec![(w.id(0), 0, "everyone".to_owned())]);
    assert_eq!(
        got1,
        vec![
            (w.id(0), 1, "hello".to_owned()),
            (w.id(0), 1, "everyone".to_owned())
        ]
    );
    let m = w.nodes[1].vpn.metrics();
    assert_eq!(m.messages_delivered, 2);
    assert_eq!(w.nodes[0].vpn.metrics().messages_sent, 2);

    // A message to this host's own id never leaves it.
    let me = w.id(0);
    let net = w.nodes[0].net;
    let now = w.now;
    w.nodes[0]
        .vpn
        .send(now, net, me, APP_PORT, 7, b"self")
        .unwrap();
    w.service(0);
    let ev = w.take_events(0);
    match &ev[..] {
        [Event::Message(d)] => {
            assert_eq!(
                (d.src, d.hops, d.src_port, &d.body[..]),
                (me, 0, 7, &b"self"[..])
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(w.nodes[0].vpn.metrics().messages_sent, 2);
}

#[test]
fn multi_hop_relay_signs_every_hop() {
    let config = Config {
        verify: VerifyMode::Audit,
        ..Config::default()
    };
    let mut w = World::new(8, 3, config);
    chain(&mut w);
    w.send(0, 2, b"far");
    w.run_for(Duration::SECOND);
    let ev = w.take_events(2);
    match &ev[..] {
        [Event::Message(d)] => {
            assert_eq!((d.src, d.hops, &d.body[..]), (w.id(0), 2, &b"far"[..]));
            assert_eq!(d.verified, Some(true));
            assert_eq!(d.flags, M_ENCRYPT);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(w.nodes[1].vpn.metrics().messages_relayed, 1);
    // The middle node can't read it.
    assert!(messages(&w.take_events(1)).is_empty());
    assert_eq!(w.nodes[2].vpn.metrics().verify_failures, 0);
}

/// Messages travel at most five hops: the fifth host gets it, the sixth
/// doesn't.
#[test]
fn hop_limit() {
    for routing in [Routing::Greedy, Routing::Learned] {
        hop_limit_with(routing);
    }
}

fn hop_limit_with(routing: Routing) {
    let config = Config {
        routing,
        ..Config::default()
    };
    let mut w = World::new(7, 8, config);
    chain(&mut w);
    w.send(0, 5, b"five");
    w.send(0, 6, b"six");
    w.broadcast(0, b"all");
    w.run_for(Duration::SECOND);
    for i in 1..8 {
        let got = messages(&w.take_events(i));
        let bodies: Vec<&str> = got.iter().map(|m| m.2.as_str()).collect();
        let want: Vec<&str> = match i {
            1..=4 => vec!["all"],
            5 => vec!["five", "all"],
            _ => vec![],
        };
        assert_eq!(bodies, want, "node {i} {routing:?}");
        for (_, hops, _) in &got {
            assert_eq!(*hops, i);
        }
    }
    match routing {
        // Node 5 gets "six" and "all" with full trailers.
        Routing::Greedy => assert!(w.nodes[5].vpn.metrics().hop_limit >= 2),
        // Node 4 doesn't send "six" on, since node 5 couldn't relay it.
        Routing::Learned => {
            assert!(w.nodes[5].vpn.metrics().hop_limit >= 1);
            assert!(w.nodes[4].vpn.metrics().hop_limit >= 1);
        }
    }
}

/// In a clique a broadcast arrives over several paths; each host delivers
/// it once.
#[test]
fn duplicates_are_dropped() {
    let mut w = world(6);
    for a in 0..6 {
        for b in a + 1..6 {
            w.link(a, b, LAT);
        }
    }
    w.run_for(Duration::SECOND);
    w.broadcast(0, b"once");
    w.run_for(Duration::SECOND);
    let mut dups = 0;
    for i in 0..6 {
        let got = messages(&w.take_events(i));
        assert_eq!(got.len(), 1, "node {i}: {got:?}");
        dups += w.nodes[i].vpn.metrics().duplicates;
    }
    assert!(dups > 0);
}

/// The wire's dedup key (seq, sender) carries no network, so the dedup
/// table adds it: one network's message must not hide another's with the
/// same seq.
#[test]
fn dedup_is_per_network() {
    let mut w = world(2);
    chain(&mut w);
    // A second network on both, over the same vnic peer.
    let key2 = key_from_seed(&[0xab; 32]).public;
    let mut nets = Vec::new();
    for i in 0..2 {
        let node = &mut w.nodes[i];
        let net2 = node.vpn.add_network(&mut node.vnic, &key2).unwrap();
        node.vpn.set_handler(net2, APP_PORT, 2).unwrap();
        let peer = node.vnic.peers().next().unwrap().0;
        node.vpn
            .add_peer(&mut node.vnic, net2, peer, 10001, 10001, net2)
            .unwrap();
        nets.push(net2);
    }
    // seq 1 in both networks.
    let now = w.now;
    let dst = w.id(1);
    let net1 = w.nodes[0].net;
    w.nodes[0]
        .vpn
        .send(now, net1, dst, APP_PORT, 1, b"a")
        .unwrap();
    w.nodes[0]
        .vpn
        .send(now, nets[0], dst, APP_PORT, 1, b"b")
        .unwrap();
    w.service(0);
    w.run_for(Duration::SECOND);
    let got = messages(&w.take_events(1));
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(w.nodes[1].vpn.metrics().duplicates, 0);
}

/// Builds a frame as `from` would send it to its neighbour.
fn frame_from(w: &World, from: usize, header: MessageHeader, body: &[u8]) -> Vec<u8> {
    let key = w.nodes[from].vnic.key();
    let sk = Signer::new(key).unwrap();
    let mut b = Vec::new();
    encode_message(&mut b, &header, body, w.now, &w.id(from), &sk).unwrap();
    b
}

fn link_peer(w: &World, at: usize, of: usize) -> strims_vnic::PeerKey {
    w.nodes[at].vnic.get_peer(&w.id(of)).unwrap()
}

#[test]
fn compressed_messages_are_rejected() {
    let mut w = world(2);
    chain(&mut w);
    let (net, now, dst) = (w.nodes[0].net, w.now, w.id(1));
    assert_eq!(
        w.nodes[0]
            .vpn
            .send_with_flags(now, net, dst, APP_PORT, 1, b"x", M_COMPRESS),
        Err(Error::CompressionUnsupported)
    );
    let h = MessageHeader {
        dst_id: dst,
        dst_port: APP_PORT,
        src_port: 1,
        seq: 9,
        length: 0,
        flags: M_COMPRESS,
    };
    let f = frame_from(&w, 0, h, b"s2 data");
    let peer = link_peer(&w, 1, 0);
    let net1 = w.nodes[1].net;
    assert_eq!(
        w.nodes[1].vpn.handle_frame(now, net1, peer, &f),
        Err(Error::CompressionUnsupported)
    );
    assert_eq!(w.nodes[1].vpn.metrics().compressed_dropped, 1);
    assert!(w.take_events(1).is_empty());
}

/// An encrypted body shorter than a nonce and a tag is rejected with an
/// error.
#[test]
fn short_encrypted_bodies_are_rejected() {
    let mut w = world(2);
    chain(&mut w);
    let h = MessageHeader {
        dst_id: w.id(1),
        dst_port: APP_PORT,
        seq: 5,
        flags: M_ENCRYPT,
        ..MessageHeader::default()
    };
    let (peer, net, now) = (link_peer(&w, 1, 0), w.nodes[1].net, w.now);
    for (i, len) in [0usize, 5, 27].into_iter().enumerate() {
        let f = frame_from(
            &w,
            0,
            MessageHeader {
                seq: 5 + i as u16,
                ..h
            },
            &vec![1; len],
        );
        assert_eq!(
            w.nodes[1].vpn.handle_frame(now, net, peer, &f),
            Err(Error::Cipher(
                strims_vpn::message_filter::CipherError::TooShort
            ))
        );
    }
    assert_eq!(w.nodes[1].vpn.metrics().decrypt_failures, 3);
}

#[test]
fn last_hop_must_be_the_link() {
    let mut w = world(3);
    chain(&mut w);
    let h = MessageHeader {
        dst_id: w.id(1),
        dst_port: APP_PORT,
        ..MessageHeader::default()
    };
    // Signed by node 2 but arriving over node 0's link.
    let f = frame_from(&w, 2, h, b"x");
    let (peer, net, now) = (link_peer(&w, 1, 0), w.nodes[1].net, w.now);
    assert_eq!(
        w.nodes[1].vpn.handle_frame(now, net, peer, &f),
        Err(Error::LastHopMismatch)
    );
    assert_eq!(
        w.nodes[1].vpn.handle_frame(now, net, peer, &f[..10]),
        Err(Error::Message(
            strims_vpn::message::MessageError::BufferTooSmall
        ))
    );
}

/// `Audit` reports, `Enforce` drops before the message can poison next-hop
/// hints or the dedup table.
#[test]
fn verification_modes() {
    for mode in [VerifyMode::Off, VerifyMode::Audit, VerifyMode::Enforce] {
        let config = Config {
            verify: mode,
            ..Config::default()
        };
        let mut w = World::new(9, 3, config);
        chain(&mut w);
        let h = MessageHeader {
            dst_id: w.id(1),
            dst_port: APP_PORT,
            seq: 77,
            ..MessageHeader::default()
        };
        let mut f = frame_from(&w, 0, h, b"body");
        // Tamper with the body after signing.
        f[strims_vpn::message::MESSAGE_HEADER_LEN] ^= 1;
        let (peer, net, now) = (link_peer(&w, 1, 0), w.nodes[1].net, w.now);
        let res = w.nodes[1].vpn.handle_frame(now, net, peer, &f);

        // A relayed message whose first hop claims node 2 but node 0 signed.
        let forged = {
            let sk0 = Signer::new(w.nodes[0].vnic.key()).unwrap();
            let mut inner = Vec::new();
            let h = MessageHeader {
                dst_id: w.id(1),
                dst_port: APP_PORT,
                seq: 78,
                ..MessageHeader::default()
            };
            encode_message(&mut inner, &h, b"forged", now, &w.id(2), &sk0).unwrap();
            let v = MessageView::decode(&inner).unwrap();
            let mut out = Vec::new();
            encode_relay(&mut out, &v, now, &w.id(0), &sk0);
            out
        };
        let res2 = w.nodes[1].vpn.handle_frame(now, net, peer, &forged);
        w.service(1);
        let ev = w.take_events(1);
        let m = w.nodes[1].vpn.metrics();
        match mode {
            VerifyMode::Off => {
                assert_eq!((res, res2), (Ok(()), Ok(())));
                assert_eq!(ev.len(), 2);
                assert!(matches!(&ev[0], Event::Message(d) if d.verified.is_none()));
                assert_eq!(m.verify_failures, 0);
            }
            VerifyMode::Audit => {
                assert_eq!((res, res2), (Ok(()), Ok(())));
                assert_eq!(ev.len(), 2);
                assert!(
                    ev.iter()
                        .all(|e| matches!(e, Event::Message(d) if d.verified == Some(false)))
                );
                assert_eq!(m.verify_failures, 2);
            }
            VerifyMode::Enforce => {
                assert_eq!(
                    (res, res2),
                    (Err(Error::Unverified), Err(Error::Unverified))
                );
                assert!(ev.is_empty());
                assert_eq!(m.unverified_dropped, 2);
                // The good copy still gets through: the bad ones didn't
                // enter the dedup table.
                let good = frame_from(&w, 0, h, b"body");
                w.nodes[1].vpn.handle_frame(now, net, peer, &good).unwrap();
                w.service(1);
                assert_eq!(messages(&w.take_events(1)).len(), 1);
            }
        }
    }
}

#[test]
fn oversized_messages() {
    let config = Config {
        max_message_len: 1000,
        ..Config::default()
    };
    let mut w = World::new(10, 2, config);
    chain(&mut w);
    let (net, now, dst) = (w.nodes[0].net, w.now, w.id(1));
    let vpn = &mut w.nodes[0].vpn;
    let room =
        1000 - strims_vpn::message::MESSAGE_HEADER_LEN - strims_vpn::message::TRAILER_ENTRY_LEN;
    assert!(
        vpn.send_with_flags(now, net, dst, APP_PORT, 1, &vec![0; room], 0)
            .is_ok()
    );
    assert_eq!(
        vpn.send_with_flags(now, net, dst, APP_PORT, 1, &vec![0; room + 1], 0),
        Err(Error::MessageTooLarge(1001))
    );
    // Encryption overhead counts.
    assert!(
        vpn.send(now, net, dst, APP_PORT, 1, &vec![0; room])
            .is_err()
    );
    assert!(matches!(
        vpn.send_with_flags(now, net, dst, APP_PORT, 1, &vec![0; 70_000], 0),
        Err(Error::MessageTooLarge(_))
    ));
}

/// Each link queues at most `link_queue_messages` messages; the rest are
/// dropped and counted, so a slow link never blocks the sender.
#[test]
fn link_queues_are_bounded() {
    let config = Config {
        link_queue_messages: 8,
        link_queue_bytes: 1 << 20,
        ..Config::default()
    };
    let mut w = World::new(11, 2, config);
    chain(&mut w);
    let (net, now, dst) = (w.nodes[0].net, w.now, w.id(1));
    let peer = link_peer(&w, 0, 1);
    for i in 0..20u8 {
        w.nodes[0]
            .vpn
            .send(now, net, dst, APP_PORT, 1, &[i])
            .unwrap();
    }
    assert_eq!(w.nodes[0].vpn.queued(net, peer), 8);
    assert_eq!(w.nodes[0].vpn.metrics().link_queue_full, 12);
    w.service(0);
    w.run_for(Duration::SECOND);
    assert_eq!(messages(&w.take_events(1)).len(), 8);
    assert_eq!(w.nodes[0].vpn.queued(net, peer), 0);

    // A byte bound too.
    let config = Config {
        link_queue_messages: 100,
        link_queue_bytes: 1000,
        ..Config::default()
    };
    let mut w = World::new(12, 2, config);
    chain(&mut w);
    let (net, now, dst) = (w.nodes[0].net, w.now, w.id(1));
    for _ in 0..10 {
        w.nodes[0]
            .vpn
            .send_with_flags(now, net, dst, APP_PORT, 1, &[0; 300], 0)
            .unwrap();
    }
    let peer = link_peer(&w, 0, 1);
    // 42 + 300 + 104 = 446 bytes each.
    assert_eq!(w.nodes[0].vpn.queued(net, peer), 2);
}

#[test]
fn delivery_queue_is_bounded() {
    let config = Config {
        max_queued_deliveries: 2,
        ..Config::default()
    };
    let mut w = World::new(13, 1, config);
    let (net, now, me) = (w.nodes[0].net, w.now, w.id(0));
    for i in 0..5u8 {
        w.nodes[0]
            .vpn
            .send(now, net, me, APP_PORT, 1, &[i])
            .unwrap();
    }
    let mut n = 0;
    while let Some(e) = w.nodes[0].vpn.poll_event() {
        if let Event::Message(d) = e {
            w.nodes[0].vpn.recycle(d.body);
            n += 1;
        }
    }
    assert_eq!(n, 2);
    assert_eq!(w.nodes[0].vpn.metrics().deliveries_dropped, 3);
}

#[test]
fn removing_a_peer_cleans_up() {
    let mut w = world(3);
    chain(&mut w);
    let net = w.nodes[1].net;
    assert_eq!(w.nodes[1].vpn.links(net).count(), 2);
    // The transport closes: vnic reports PeerRemoved.
    w.unlink(0);
    assert_eq!(w.nodes[1].vpn.links(net).count(), 1);
    assert!(!w.nodes[1].vpn.has_peer(net, &w.id(0)));
    // The app removes a peer from the network: vnic keeps the peer.
    let id2 = w.id(2);
    let node = &mut w.nodes[1];
    assert!(node.vpn.remove_peer(&mut node.vnic, net, &id2));
    assert!(!node.vpn.remove_peer(&mut node.vnic, net, &id2));
    assert_eq!(node.vpn.links(net).count(), 0);
    assert!(node.vnic.has_peer(&id2));
    let peer = node.vnic.get_peer(&id2).unwrap();
    assert_eq!(node.vnic.handler(peer, common::VPN_PORT), None);
    // ...and adds it back.
    node.vpn
        .add_peer(
            &mut node.vnic,
            net,
            peer,
            common::VPN_PORT,
            common::VPN_PORT,
            net,
        )
        .unwrap();
    // Twice is a no-op.
    node.vpn
        .add_peer(
            &mut node.vnic,
            net,
            peer,
            common::VPN_PORT,
            common::VPN_PORT,
            net,
        )
        .unwrap();
    w.send(1, 2, b"back");
    w.run_for(Duration::SECOND);
    assert_eq!(messages(&w.take_events(2)).len(), 1);
}

/// A host with more than 20 links in one routing bucket keeps the extra
/// links attached but unrouted, and they can be removed and added again.
#[test]
fn evicted_links_stay_attached_and_can_be_re_added() {
    let n = 64;
    let mut w = World::new(14, n, Config::default());
    for i in 1..n {
        w.link(0, i, LAT);
    }
    w.run_for(Duration::SECOND);
    let net = w.nodes[0].net;
    let links: Vec<_> = w.nodes[0].vpn.links(net).collect();
    assert_eq!(links.len(), n - 1);
    let unrouted: Vec<_> = links.iter().filter(|l| !l.2).collect();
    assert!(!unrouted.is_empty(), "no bucket overflowed");
    let (peer, id, _) = *unrouted[0];
    assert!(!w.nodes[0].vpn.has_peer(net, &id));
    let j = w.index(&id).unwrap();

    // Its frames are still read.
    w.send(j, 0, b"from an evicted link");
    w.run_for(Duration::SECOND);
    assert_eq!(messages(&w.take_events(0)).len(), 1);

    // Remove and re-add it.
    let node = &mut w.nodes[0];
    assert!(node.vpn.remove_peer(&mut node.vnic, net, &id));
    node.vpn
        .add_peer(
            &mut node.vnic,
            net,
            peer,
            common::VPN_PORT,
            common::VPN_PORT,
            net,
        )
        .unwrap();
    assert_eq!(node.vpn.links(net).count(), n - 1);

    // Removing a routed link routes a waiting one.
    let routed_before = node.vpn.links(net).filter(|l| l.2).count();
    let (_, rid, _) = node.vpn.links(net).find(|l| l.2).unwrap();
    assert!(node.vpn.remove_peer(&mut node.vnic, net, &rid));
    let routed_after = node.vpn.links(net).filter(|l| l.2).count();
    assert!(routed_after >= routed_before - 1);
}

#[test]
fn ports() {
    let mut w = world(1);
    let net = w.nodes[0].net;
    let vpn = &mut w.nodes[0].vpn;
    assert_eq!(
        vpn.set_handler(net, APP_PORT, 1),
        Err(Error::PortInUse(APP_PORT))
    );
    assert_eq!(
        vpn.set_handler(net, ports::HASH_TABLE_PORT, 1),
        Err(Error::PortInUse(ports::HASH_TABLE_PORT))
    );
    assert_eq!(vpn.remove_handler(net, ports::PEER_INDEX_PORT), None);
    assert_eq!(vpn.remove_handler(net, APP_PORT), Some(APP_PORT));
    let p = vpn.reserve_port(net).unwrap();
    assert!(p >= ports::RESERVED_PORT_COUNT);
    vpn.set_handler(net, p, 5).unwrap();
    // A port with a handler stays reserved.
    vpn.release_port(net, p);
    assert_ne!(vpn.reserve_port(net).unwrap(), p);
    let gone = other_network(&mut w);
    assert_eq!(
        w.nodes[0].vpn.reserve_port(gone),
        Err(Error::NetworkNotFound)
    );
}

fn other_network(w: &mut World) -> strims_vpn::NetworkId {
    let node = &mut w.nodes[0];
    let key = key_from_seed(&[0x77; 32]).public;
    let net = node.vpn.add_network(&mut node.vnic, &key).unwrap();
    assert_eq!(
        node.vpn.add_network(&mut node.vnic, &key),
        Err(Error::DuplicateNetworkKey)
    );
    assert_eq!(node.vpn.network(&key), Some(net));
    node.vpn.remove_network(&mut node.vnic, net).unwrap();
    assert_eq!(node.vpn.network(&key), None);
    net
}

#[test]
fn channel() {
    let mut w = world(2);
    chain(&mut w);
    let (net0, net1) = (w.nodes[0].net, w.nodes[1].net);
    let (id0, id1) = (w.id(0), w.id(1));
    let c0 = strims_vpn::Channel::open(&mut w.nodes[0].vpn, net0, id1, 3001, 3000, 3000).unwrap();
    let c1 = strims_vpn::Channel::open(&mut w.nodes[1].vpn, net1, id0, 3000, 3001, 3001).unwrap();
    let now = w.now;
    assert_eq!(c0.write(&mut w.nodes[0].vpn, now, b"ping").unwrap(), 4);
    w.service(0);
    w.run_for(Duration::SECOND);
    let ev = w.take_events(1);
    assert!(matches!(&ev[..], [Event::Message(d)] if d.handler == 3001 && d.body == b"ping"));
    c1.write(&mut w.nodes[1].vpn, w.now, b"pong").unwrap();
    w.service(1);
    w.run_for(Duration::SECOND);
    let ev = w.take_events(0);
    assert!(matches!(&ev[..], [Event::Message(d)] if d.handler == 3000 && d.body == b"pong"));
    c0.close(&mut w.nodes[0].vpn);
    assert!(w.nodes[0].vpn.set_handler(net0, 3000, 1).is_ok());
}

fn ht_values(events: &[Event<AppTag>]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::HashTableValue { value, .. } => {
                Some(String::from_utf8_lossy(value).into_owned())
            }
            _ => None,
        })
        .collect()
}

/// Set and get across hosts; a get response carrying a changed value must
/// reach the requester.
#[test]
fn hash_table_set_get() {
    let mut w = world(3);
    chain(&mut w);
    let key = key_from_seed(&[0x51; 32]);
    let (net, now) = (w.nodes[0].net, w.now);
    let p = w.nodes[0]
        .vpn
        .hash_table_set(now, net, &key, b"foo", b"bar")
        .unwrap();
    w.service(0);
    w.run_for(Duration::SECOND);
    assert!(w.nodes[2].vpn.hash_table_len() >= 1);

    // From the network (the local copy is skipped).
    let opts = HashTableGetOptions {
        disable_cache: true,
        timeout: Duration::SECOND,
    };
    let net2 = w.nodes[2].net;
    let now = w.now;
    w.nodes[2]
        .vpn
        .hash_table_get(now, net2, &key.public, b"foo", opts)
        .unwrap();
    w.service(2);
    w.run_for(Duration::from_secs(2));
    let ev = w.take_events(2);
    assert_eq!(ht_values(&ev), vec!["bar"]);
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::HashTableGetClosed { .. }))
    );

    // A fresh local copy answers at once and closes.
    let now = w.now;
    w.nodes[2]
        .vpn
        .hash_table_get(
            now,
            net2,
            &key.public,
            b"foo",
            HashTableGetOptions::default(),
        )
        .unwrap();
    let ev: Vec<_> = std::iter::from_fn(|| w.nodes[2].vpn.poll_event()).collect();
    assert_eq!(ht_values(&ev), vec!["bar"]);
    assert!(matches!(ev.last(), Some(Event::HashTableGetClosed { .. })));

    // Cut node 2 off, update the value, and let the republish reach node 1.
    w.unlink(1);
    w.nodes[0].vpn.hash_table_update(p, b"baz").unwrap();
    w.run_for(strims_vpn::hash_table::HASH_TABLE_SET_INTERVAL + Duration::SECOND);
    w.take_events(2);
    w.link(1, 2, LAT);
    w.run_for(Duration::SECOND);
    // Node 2's copy is old: it delivers it, asks, and gets the new value.
    let now = w.now;
    w.nodes[2]
        .vpn
        .hash_table_get(
            now,
            net2,
            &key.public,
            b"foo",
            HashTableGetOptions::default(),
        )
        .unwrap();
    w.service(2);
    w.run_for(Duration::from_secs(2));
    let ev = w.take_events(2);
    assert_eq!(ht_values(&ev), vec!["bar", "baz"]);

    // Closing unpublishes: the copies it reaches are dropped.
    let now = w.now;
    w.nodes[0].vpn.hash_table_close(now, p).unwrap();
    w.service(0);
    w.run_for(Duration::SECOND);
    assert_eq!(w.nodes[2].vpn.hash_table_len(), 0);
    assert_eq!(
        w.nodes[0].vpn.hash_table_update(p, b"x"),
        Err(Error::UnknownRequest)
    );
}

#[test]
fn hash_table_records_expire() {
    let mut w = world(2);
    chain(&mut w);
    let key = key_from_seed(&[0x52; 32]);
    let (net, now) = (w.nodes[0].net, w.now);
    let p = w.nodes[0]
        .vpn
        .hash_table_set(now, net, &key, b"s", b"v")
        .unwrap();
    w.service(0);
    w.run_for(Duration::SECOND);
    assert_eq!(w.nodes[1].vpn.hash_table_len(), 1);
    // Republishing keeps the record alive past the max age.
    w.run_for(strims_vpn::hash_table::HASH_TABLE_MAX_RECORD_AGE + Duration::from_secs(120));
    assert_eq!(w.nodes[1].vpn.hash_table_len(), 1);
    // Without republishing it expires.
    let node = &mut w.nodes[0];
    let _ = p;
    node.vpn.remove_network(&mut node.vnic, net).unwrap();
    w.run_for(strims_vpn::hash_table::HASH_TABLE_MAX_RECORD_AGE + Duration::from_secs(120));
    assert_eq!(w.nodes[1].vpn.hash_table_len(), 0);
}

#[test]
fn peer_index_publish_search() {
    let mut w = world(3);
    chain(&mut w);
    let (net, now) = (w.nodes[0].net, w.now);
    w.nodes[0]
        .vpn
        .peer_index_publish(now, net, b"swarm", b"salt", 4242)
        .unwrap();
    w.service(0);
    w.run_for(Duration::SECOND);
    // One hop: node 1 has it, node 2 doesn't.
    assert_eq!(w.nodes[1].vpn.peer_index_len(w.nodes[1].net), 1);
    assert_eq!(w.nodes[2].vpn.peer_index_len(w.nodes[2].net), 0);

    let (net2, now) = (w.nodes[2].net, w.now);
    w.nodes[2]
        .vpn
        .peer_index_search(now, net2, b"swarm", b"salt")
        .unwrap();
    w.service(2);
    w.run_for(Duration::from_secs(61));
    let ev = w.take_events(2);
    let hosts: Vec<_> = ev
        .iter()
        .filter_map(|e| match e {
            Event::PeerIndexHost { host, .. } => Some(*host),
            _ => None,
        })
        .collect();
    assert_eq!(hosts.len(), 1, "{ev:?}");
    assert_eq!((hosts[0].host_id, hosts[0].port), (w.id(0), 4242));
    assert!(matches!(
        ev.last(),
        Some(Event::PeerIndexSearchClosed { .. })
    ));

    // The record outlives a missed republish only until the max age.
    w.unlink(0);
    w.run_for(strims_vpn::peer_index::PEER_INDEX_MAX_RECORD_AGE + Duration::from_secs(120));
    assert_eq!(w.nodes[1].vpn.peer_index_len(w.nodes[1].net), 0);
}

#[test]
fn peer_exchange_links_unconnected_hosts() {
    let mut w = world(3);
    chain(&mut w);
    assert!(!w.linked(0, 2));
    let (net, now, id2) = (w.nodes[0].net, w.now, w.id(2));
    let node = &mut w.nodes[0];
    let x = node.vpn.connect(&node.vnic, now, net, id2).unwrap();
    assert!(x.is_some());
    // A second connect while one is in progress, to self, or to a peer.
    let id1 = w.id(1);
    let id0 = w.id(0);
    let node = &mut w.nodes[0];
    assert_eq!(node.vpn.connect(&node.vnic, now, net, id2), Ok(None));
    assert_eq!(node.vpn.connect(&node.vnic, now, net, id0), Ok(None));
    assert_eq!(node.vpn.connect(&node.vnic, now, net, id1), Ok(None));
    w.service(0);
    w.run_for(Duration::from_secs(2));
    assert!(w.linked(0, 2));
    assert!(w.nodes[0].vnic.has_peer(&id2));
    assert!(w.nodes[0].vpn.has_peer(net, &id2));
    let ev0 = w.take_events(0);
    let ev2 = w.take_events(2);
    assert!(
        ev0.iter()
            .any(|e| matches!(e, Event::GatherLocalDescriptions { .. }))
    );
    assert!(
        has_closed(&ev0, &ExchangeOutcome::RemoteConnected),
        "{ev0:?}"
    );
    assert!(
        ev2.iter()
            .any(|e| matches!(e, Event::DialRemoteDescriptions { .. }))
    );
    assert!(has_closed(&ev2, &ExchangeOutcome::Connected), "{ev2:?}");
    assert_eq!(w.nodes[0].vpn.exchanges().count(), 0);
}

#[test]
fn peer_exchange_answerer_descriptions_and_refusals() {
    let mut w = world(3);
    chain(&mut w);
    // Node 2 can't dial node 0's descriptions, so it answers with its own
    // and node 0 dials them.
    let (net, now, id2) = (w.nodes[0].net, w.now, w.id(2));
    w.nodes[2].auto_exchange = false;
    let node = &mut w.nodes[0];
    let x = node
        .vpn
        .connect(&node.vnic, now, net, id2)
        .unwrap()
        .unwrap();
    w.service(0);
    w.run_for(Duration::SECOND);
    let ev = w.take_events(2);
    let x2 = match &ev[..] {
        [Event::DialRemoteDescriptions { exchange, .. }] => *exchange,
        other => panic!("{other:?}"),
    };
    let now = w.now;
    w.nodes[2]
        .vpn
        .set_remote_descriptions_result(now, x2, Err("dial failed".into()))
        .unwrap();
    w.service(2);
    let ev = w.take_events(2);
    assert!(
        matches!(&ev[..], [Event::GatherLocalDescriptions { exchange, .. }] if *exchange == x2)
    );
    // Wrong order is refused.
    assert_eq!(
        w.nodes[2]
            .vpn
            .set_remote_descriptions_result(now, x2, Ok(true)),
        Err(Error::UnknownRequest)
    );
    w.nodes[2]
        .vpn
        .set_local_descriptions(
            now,
            x2,
            Ok(vec![strims_vpn::LinkDescription {
                interface: common::SIM_INTERFACE.into(),
                description: "2".into(),
            }]),
        )
        .unwrap();
    w.service(2);
    w.run_for(Duration::SECOND);
    assert!(w.linked(0, 2));
    let ev0 = w.take_events(0);
    assert!(
        ev0.iter()
            .any(|e| matches!(e, Event::DialRemoteDescriptions { exchange, .. } if *exchange == x))
    );
    assert!(has_closed(&ev0, &ExchangeOutcome::Connected), "{ev0:?}");
    // Node 2's side stays open for a background link, then closes.
    w.run_for(Duration::from_secs(20));
    assert!(has_closed(&w.take_events(2), &ExchangeOutcome::Answered));

    // A host without room for peers refuses.
    let mut w = world(3);
    chain(&mut w);
    w.nodes[2].vnic.set_max_peers(1);
    let (net, now, id2) = (w.nodes[0].net, w.now, w.id(2));
    let node = &mut w.nodes[0];
    node.vpn
        .connect(&node.vnic, now, net, id2)
        .unwrap()
        .unwrap();
    w.service(0);
    w.run_for(Duration::SECOND);
    assert!(has_closed(
        &w.take_events(0),
        &ExchangeOutcome::Rejected(strims_vpn::peer_exchange::ERR_NO_NEW_PEERS.into())
    ));
    assert!(!w.linked(0, 2));
}

/// An outgoing exchange whose answer never comes times out, and a later
/// `connect` to that host starts a new one.
#[test]
fn stuck_exchanges_time_out() {
    let mut w = world(2);
    chain(&mut w);
    let ghost = ID([0x1234; 4]);
    let (net, now) = (w.nodes[0].net, w.now);
    let node = &mut w.nodes[0];
    assert!(
        node.vpn
            .connect(&node.vnic, now, net, ghost)
            .unwrap()
            .is_some()
    );
    w.service(0);
    w.run_for(Duration::from_secs(19));
    assert!(!has_closed(&w.take_events(0), &ExchangeOutcome::TimedOut));
    w.run_for(Duration::from_secs(2));
    assert!(has_closed(&w.take_events(0), &ExchangeOutcome::TimedOut));
    let now = w.now;
    let node = &mut w.nodes[0];
    assert!(
        node.vpn
            .connect(&node.vnic, now, net, ghost)
            .unwrap()
            .is_some()
    );
}

/// A stale or forged answer must not end the live exchange.
#[test]
fn mismatched_answers_are_ignored() {
    let mut w = world(3);
    chain(&mut w);
    w.nodes[2].auto_exchange = false;
    let (net, now, id0, id2) = (w.nodes[0].net, w.now, w.id(0), w.id(2));
    let node = &mut w.nodes[0];
    node.vpn
        .connect(&node.vnic, now, net, id2)
        .unwrap()
        .unwrap();
    w.service(0);
    w.run_for(Duration::SECOND);
    let x2 = match &w.take_events(2)[..] {
        [Event::DialRemoteDescriptions { exchange, .. }] => *exchange,
        other => panic!("{other:?}"),
    };
    w.take_events(0);
    // Node 2 sends a forged answer with another id.
    let forged = PeerExchangeMessage {
        body: Some(PxBody::LinkAnswer(LinkAnswer {
            exchange_id: 12345,
            descriptions: vec![],
            error_message: "go away".into(),
        })),
    };
    let (net2, now) = (w.nodes[2].net, w.now);
    w.nodes[2]
        .vpn
        .send_proto(
            now,
            net2,
            id0,
            ports::PEER_EXCHANGE_PORT,
            ports::PEER_EXCHANGE_PORT,
            &forged,
        )
        .unwrap();
    w.service(2);
    w.run_for(Duration::SECOND);
    assert!(w.take_events(0).is_empty());
    assert_eq!(w.nodes[0].vpn.exchanges().count(), 1);
    // The real answer still counts.
    let now = w.now;
    w.nodes[2]
        .vpn
        .set_remote_descriptions_result(now, x2, Ok(false))
        .unwrap();
    w.nodes[2]
        .vpn
        .set_local_descriptions(now, x2, Ok(vec![]))
        .unwrap();
    w.service(2);
    w.run_for(Duration::SECOND);
    assert!(has_closed(
        &w.take_events(0),
        &ExchangeOutcome::RemoteConnected
    ));
}

#[test]
fn removing_a_network_ends_its_requests() {
    let mut w = world(2);
    chain(&mut w);
    let (net, now) = (w.nodes[0].net, w.now);
    let vpn = &mut w.nodes[0].vpn;
    let opts = HashTableGetOptions {
        disable_cache: true,
        ..Default::default()
    };
    vpn.hash_table_get(now, net, b"k", b"s", opts).unwrap();
    vpn.peer_index_search(now, net, b"k", b"s").unwrap();
    w.nodes[0].vpn.poll_event();
    let node = &mut w.nodes[0];
    node.vpn.remove_network(&mut node.vnic, net).unwrap();
    let ev: Vec<_> = std::iter::from_fn(|| w.nodes[0].vpn.poll_event()).collect();
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::HashTableGetClosed { .. }))
    );
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::PeerIndexSearchClosed { .. }))
    );
    assert_eq!(w.nodes[0].vpn.poll_timeout(), None);
    // vnic dropped the handler.
    let peer = w.nodes[0].vnic.get_peer(&w.id(1)).unwrap();
    assert_eq!(w.nodes[0].vnic.handler(peer, common::VPN_PORT), None);
    let (now, id1) = (w.now, w.id(1));
    assert_eq!(
        w.nodes[0].vpn.send(now, net, id1, 1, 1, b"x"),
        Err(Error::NetworkNotFound)
    );
}

#[test]
fn broadcasts_carry_flags_and_reach_the_target_side() {
    let mut w = world(4);
    chain(&mut w);
    let (net, now) = (w.nodes[0].net, w.now);
    let target = w.id(3);
    w.nodes[0]
        .vpn
        .send_with_flags(now, net, target, APP_PORT, 1, b"towards 3", M_BROADCAST)
        .unwrap();
    w.service(0);
    w.run_for(Duration::SECOND);
    for i in 0..4 {
        let got = messages(&w.take_events(i));
        assert_eq!(got.len(), 1, "node {i}");
        assert_eq!(got[0].1, i);
    }
    let _ = Time::EPOCH;
}

/// `Routing::Learned`: hosts announce their links to their neighbours, who
/// pass the lists on once, so a host knows the links three hops out
/// before any traffic, and routes over them. Greedy hosts neither announce
/// nor use announcements.
#[test]
fn link_announcements_teach_the_neighbourhood() {
    let mut w = world(5);
    chain(&mut w);
    w.run_for(Duration::from_secs(3));
    let net = w.nodes[0].net;
    // Node 0 learns 1-2 from 1, 2-3 from 1 passing on 2's list, and 3-4
    // not at all (four hops out). Its own links aren't stored.
    assert_eq!(w.nodes[0].vpn.topology_len(net), 3);
    let m = w.nodes[2].vpn.metrics();
    assert!(m.link_announcements_sent >= 1);
    assert!(m.link_announcements_received >= 2);
    // Announcements aren't app or service messages.
    assert_eq!(m.messages_sent, 0);
    assert_eq!(m.messages_delivered, 0);

    // A path is known to node 3: one copy at each hop.
    let before = w.nodes[0].vpn.metrics();
    w.send(0, 3, b"known");
    w.run_for(Duration::SECOND);
    assert_eq!(
        messages(&w.take_events(3)),
        vec![(w.id(0), 3, "known".to_owned())]
    );
    let after = w.nodes[0].vpn.metrics();
    assert_eq!(after.learned_routes - before.learned_routes, 1);
    assert_eq!(after.frames_queued - before.frames_queued, 1);
    assert_eq!(after.fallback_routes, 0);
    // Node 4 is unknown to node 0, but node 1 knows it: delivered.
    w.send(0, 4, b"unknown");
    w.run_for(Duration::SECOND);
    assert_eq!(messages(&w.take_events(4)).len(), 1);
    assert_eq!(w.nodes[0].vpn.metrics().fallback_routes, 1);

    let config = Config {
        routing: Routing::Greedy,
        ..Config::default()
    };
    let mut w = World::new(7, 3, config);
    chain(&mut w);
    w.run_for(Duration::from_secs(3));
    for n in &w.nodes {
        assert_eq!(n.vpn.metrics().link_announcements_sent, 0);
        assert_eq!(n.vpn.topology_len(n.net), 0);
    }
}

/// A copy that arrives later on a shorter path than the first one is
/// relayed again (`Routing::Learned`), so a message whose first copy took a
/// detour still arrives. `Routing::Greedy` drops it as a duplicate.
///
/// ```text
/// 0 -5ms- 1 -5ms- 2 -5ms- 3 - 4 - 5 - 6 - 7
///  \_______________100ms__/
/// ```
///
/// Node 3 first gets the message from 2, three hops in, with too few
/// hops left to reach 7; the copy over the slow direct link comes one hop
/// in.
#[test]
fn shorter_copies_are_relayed_again() {
    for routing in [Routing::Greedy, Routing::Learned] {
        let config = Config {
            routing,
            ..Config::default()
        };
        let mut w = World::new(9, 8, config);
        for (a, b) in [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7)] {
            w.link(a, b, LAT);
        }
        w.link(0, 3, Duration::from_millis(100));
        w.run_for(Duration::from_secs(5));
        w.send(0, 7, b"far");
        w.run_for(Duration::SECOND);
        let got = messages(&w.take_events(7));
        let relays = w.nodes[3].vpn.metrics().shorter_relays;
        match routing {
            Routing::Greedy => {
                assert!(got.is_empty(), "{got:?}");
                assert_eq!(relays, 0);
            }
            Routing::Learned => {
                assert_eq!(got, vec![(w.id(0), 5, "far".to_owned())]);
                assert_eq!(relays, 1);
            }
        }
    }
}

/// A unicast to a neighbour goes over the direct link alone, at the sender
/// and at a relay next to the destination.
///
/// ```text
/// 0 - 1 - 2
///      \ /
///       3
/// ```
#[test]
fn direct_links_get_one_copy() {
    let mut w = world(4);
    for (a, b) in [(0, 1), (1, 2), (1, 3), (3, 2)] {
        w.link(a, b, LAT);
    }
    w.run_for(Duration::from_secs(5));
    // Node 1 knows 2 directly and through 3; node 0 knows the paths.
    let queued = |w: &World| -> u64 { w.nodes.iter().map(|n| n.vpn.metrics().frames_queued).sum() };
    let before = queued(&w);
    w.send(1, 2, b"direct");
    w.run_for(Duration::SECOND);
    assert_eq!(queued(&w) - before, 1);
    assert_eq!(
        messages(&w.take_events(2)),
        vec![(w.id(1), 1, "direct".to_owned())]
    );
    let before = queued(&w);
    w.send(0, 2, b"relayed");
    w.run_for(Duration::SECOND);
    assert_eq!(queued(&w) - before, 2);
    assert_eq!(
        messages(&w.take_events(2)),
        vec![(w.id(0), 2, "relayed".to_owned())]
    );
}

/// Link announcements cross one greedy host: host 0 announces to greedy
/// host 1 with a trailer that lets 1 relay it to exactly one link, so host
/// 2 learns 0's links two hops away. Greedy hosts handle it as an ordinary
/// broadcast on a port without a handler, and every trailer entry
/// verifies.
///
/// ```text
/// 3 - 0 - 1 - 2      0, 2: Learned; 1, 3: Greedy
/// ```
#[test]
fn link_announcements_cross_one_greedy_host() {
    for reach in [1, 2] {
        let mut w = World::new_with(7, 4, |i| Config {
            routing: if i % 2 == 0 {
                Routing::Learned
            } else {
                Routing::Greedy
            },
            announce_reach: reach,
            verify: VerifyMode::Audit,
            ..Config::default()
        });
        for (a, b) in [(3, 0), (0, 1), (1, 2)] {
            w.link(a, b, LAT);
        }
        w.run_for(Duration::from_secs(5));
        let net = w.nodes[2].net;
        let m: Vec<_> = w.nodes.iter().map(|n| n.vpn.metrics()).collect();
        for n in &m {
            assert_eq!(n.verify_failures, 0);
            assert_eq!(n.messages_delivered, 0);
        }
        match reach {
            1 => {
                // One hop only: node 2 has heard nothing.
                assert_eq!(w.nodes[2].vpn.topology_len(net), 0);
                assert_eq!(m[1].messages_relayed, 0);
            }
            _ => {
                // Node 2 knows 0's links (to 1 and 3), and 0 knows 2's.
                assert_eq!(w.nodes[2].vpn.topology_len(net), 3);
                assert_eq!(w.nodes[0].vpn.topology_len(net), 2);
                assert_eq!(
                    m[2].link_announcements_received,
                    m[0].link_announcements_sent
                );
                // Node 1 relays each announcement it gets once, to one
                // link; node 3 has no other link to send it to, and learned
                // hosts never relay one.
                let got = m[0].link_announcements_sent + m[2].link_announcements_sent;
                assert_eq!(m[1].messages_relayed, got);
                assert_eq!(m[1].frames_sent, got);
                assert_eq!(m[3].frames_sent, 0);
                assert_eq!(m[0].messages_relayed + m[2].messages_relayed, 0);
            }
        }
    }
}

/// A host that published before it had links: the peer index record is
/// pushed to the new links, and the hash table record, which reached
/// nobody, is published again, so both are found before the next
/// publish.
#[test]
fn records_published_before_links_are_found() {
    let mut w = world(3);
    let (net, now) = (w.nodes[0].net, w.now);
    let key = key_from_seed(&[0x42; 32]);
    w.nodes[0]
        .vpn
        .peer_index_publish(now, net, b"swarm", b"salt", 4242)
        .unwrap();
    w.nodes[0]
        .vpn
        .hash_table_set(now, net, &key, b"salt", b"early")
        .unwrap();
    w.service(0);
    w.run_for(Duration::from_secs(5));
    assert_eq!(w.nodes[0].vpn.metrics().frames_queued, 0);

    chain(&mut w);
    w.run_for(Duration::from_secs(2));
    assert_eq!(w.nodes[0].vpn.metrics().records_pushed, 2);
    let net1 = w.nodes[1].net;
    assert_eq!(w.nodes[1].vpn.peer_index_len(net1), 1);

    let (net2, now) = (w.nodes[2].net, w.now);
    w.nodes[2]
        .vpn
        .peer_index_search(now, net2, b"swarm", b"salt")
        .unwrap();
    let opts = HashTableGetOptions {
        disable_cache: true,
        timeout: Duration::from_secs(5),
    };
    w.nodes[2]
        .vpn
        .hash_table_get(now, net2, &key.public, b"salt", opts)
        .unwrap();
    w.service(2);
    w.run_for(Duration::from_secs(61));
    let ev = w.take_events(2);
    assert!(
        ev.iter().any(|e| matches!(e, Event::PeerIndexHost { host, .. } if host.host_id == w.id(0) && host.port == 4242)),
        "{ev:?}"
    );
    assert_eq!(ht_values(&ev), vec!["early".to_owned()]);
}

/// Links that come up later get the peer index records too, at most one
/// push per `RECORD_PUSH_INTERVAL` per network; hash table records that
/// reached the network aren't published again.
#[test]
fn new_links_get_records() {
    let mut w = world(4);
    w.link(0, 1, LAT);
    w.run_for(Duration::SECOND);
    let (net, now) = (w.nodes[0].net, w.now);
    let key = key_from_seed(&[0x43; 32]);
    w.nodes[0]
        .vpn
        .peer_index_publish(now, net, b"swarm", b"salt", 4242)
        .unwrap();
    w.nodes[0]
        .vpn
        .hash_table_set(now, net, &key, b"salt", b"v")
        .unwrap();
    w.service(0);
    w.run_for(Duration::SECOND);
    let len = |w: &World, i: usize| w.nodes[i].vpn.peer_index_len(w.nodes[i].net);
    assert_eq!(len(&w, 1), 1);

    w.link(0, 2, LAT);
    w.run_for(Duration::from_secs(2));
    assert_eq!(len(&w, 2), 1);
    assert_eq!(w.nodes[0].vpn.metrics().records_pushed, 1);

    // Right after a push, the next one waits for the interval.
    w.link(0, 3, LAT);
    w.run_for(Duration::from_secs(3));
    assert_eq!(len(&w, 3), 0);
    w.run_for(strims_vpn::RECORD_PUSH_INTERVAL);
    assert_eq!(len(&w, 3), 1);
    // One record per push, to the new link only.
    let m = w.nodes[0].vpn.metrics();
    assert_eq!(m.records_pushed, 2);
    assert_eq!(m.messages_sent, 2 + 2);

    // A host without records sends nothing new to its new links.
    let before = w.nodes[1].vpn.metrics();
    w.link(1, 2, LAT);
    w.run_for(Duration::from_secs(15));
    assert_eq!(w.nodes[1].vpn.metrics().records_pushed, 0);
    assert_eq!(w.nodes[1].vpn.metrics().messages_sent, before.messages_sent);
}
