// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Host behaviour through the public API: handshakes (plain and AES over the
//! TCP mux), dispatch, peer replacement, failed handshakes, ports, channels
//! and egress blocking.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod common;

use std::collections::VecDeque;

use common::{Node, Tag, now, pump};
use strims_base::time::Duration;
use strims_proto::vnic::v1::PeerInit;
use strims_vnic::host::CloseReason;
use strims_vnic::stream::encode_stream_message;
use strims_vnic::{
    Error, Event, Fill, Host, LinkConfig, LinkKind, PeerInitError, PeerKey, TcpMux, TcpMuxAcceptor,
    WriterId,
};

fn no_handlers(_: &mut Host<Tag>, _: PeerKey) {}

fn handlers(ports: &'static [u16]) -> impl FnMut(&mut Host<Tag>, PeerKey) {
    move |h: &mut Host<Tag>, p: PeerKey| {
        for &port in ports {
            h.set_handler(p, port, Tag::from(port)).unwrap();
        }
    }
}

fn plain(mtu: usize) -> LinkConfig {
    LinkConfig::new(LinkKind::Other, mtu)
}

fn added(events: &[Event]) -> Vec<PeerKey> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::PeerAdded { peer, .. } => Some(*peer),
            _ => None,
        })
        .collect()
}

/// Sends every queued message of each writer, one frame per message.
fn send_all(node: &mut Node, queues: &mut [(WriterId, VecDeque<Vec<u8>>)]) {
    for (w, q) in queues.iter() {
        if !q.is_empty() {
            node.host.set_backlog(*w).unwrap();
        }
    }
    node.host.poll_egress(now(), &mut |f| {
        let q = &mut queues.iter_mut().find(|(w, _)| *w == f.writer).unwrap().1;
        let Some(m) = q.pop_front() else {
            return Fill::NONE;
        };
        f.body[..m.len()].copy_from_slice(&m);
        Fill {
            len: m.len(),
            more: !q.is_empty(),
        }
    });
}

#[test]
fn plain_handshake_and_frames_at_every_chunk_size() {
    for chunk in [1, 2, 3, 5, 64, 1000, usize::MAX] {
        let mut a = Node::new(1, "a");
        let mut b = Node::new(2, "b");
        let la = a.host.add_link(now(), plain(300)).unwrap();
        let lb = b.host.add_link(now(), plain(300)).unwrap();
        let mut on_peer = handlers(&[100, 200]);
        pump(&mut a, la, &mut b, lb, now(), chunk, &mut on_peer);
        assert_eq!(added(&a.events), vec![la]);
        assert_eq!(added(&b.events), vec![lb]);
        assert_eq!(a.host.peer(la).unwrap().host_id(), b.host.id());
        assert_eq!(b.host.peer(lb).unwrap().host_id(), a.host.id());
        assert_eq!(b.host.get_peer(&a.host.id()), Some(lb));
        // The peer's certificate is its profile certificate.
        assert_eq!(
            b.host.peer(lb).unwrap().certificate().key,
            a.host.profile_key().public
        );
        assert_eq!(b.host.peer(lb).unwrap().node_version(), "dev");

        let root = a.host.qos_root();
        let w100 = a.host.open_writer(la, 100, root, 0).unwrap();
        let w200 = a.host.open_writer(la, 200, root, 0).unwrap();
        assert_eq!(a.host.writer_max_body(w100).unwrap(), 296);
        let msgs: Vec<Vec<u8>> = (0..20u8)
            .map(|i| vec![i; usize::from(i) * 13 % 297])
            .collect();
        let mut queues = vec![
            (w100, msgs.iter().cloned().collect()),
            (w200, msgs.iter().rev().cloned().collect()),
        ];
        for _ in 0..20 {
            send_all(&mut a, &mut queues);
            pump(&mut a, la, &mut b, lb, now(), chunk, &mut on_peer);
        }
        let on = |port: u16| -> Vec<Vec<u8>> {
            b.recv
                .iter()
                .filter(|r| r.1 == port)
                .map(|r| r.3.clone())
                .collect()
        };
        // Zero-length messages produce no frame (Fill len 0).
        let want: Vec<Vec<u8>> = msgs.iter().filter(|m| !m.is_empty()).cloned().collect();
        assert_eq!(on(100), want, "chunk {chunk}");
        let want: Vec<Vec<u8>> = msgs
            .iter()
            .rev()
            .filter(|m| !m.is_empty())
            .cloned()
            .collect();
        assert_eq!(on(200), want);
        assert!(b.recv.iter().all(|r| r.0 == lb && Tag::from(r.1) == r.2));
    }
}

#[test]
fn frames_right_behind_peer_init_wait_for_handlers() {
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let la = a.host.add_link(now(), plain(1000)).unwrap();
    let lb = b.host.add_link(now(), plain(1000)).unwrap();
    // Hold A's PeerInit; let A finish first and send frames.
    let mut held = a.take_transmit(la);
    let mut b_init = b.take_transmit(lb);
    a.input(la, now(), &mut b_init, &mut no_handlers).unwrap();
    let w = a.host.open_writer(la, 7, a.host.qos_root(), 0).unwrap();
    let mut queues = vec![(w, VecDeque::from(vec![b"one".to_vec(), b"two".to_vec()]))];
    send_all(&mut a, &mut queues);
    held.extend(a.take_transmit(la));
    // PeerInit and frames in one read: the frames reach the handler that
    // B's owner installs on PeerAdded.
    let mut on_peer = handlers(&[7]);
    b.input(lb, now(), &mut held, &mut on_peer).unwrap();
    let got: Vec<&[u8]> = b.recv.iter().map(|r| r.3.as_slice()).collect();
    assert_eq!(got, vec![&b"one"[..], b"two"]);
}

fn connect_aes(
    a: &mut Node,
    b: &mut Node,
    on_peer: &mut dyn FnMut(&mut Host<Tag>, PeerKey),
) -> (PeerKey, PeerKey) {
    let b_profile: [u8; 32] = b.host.profile_key().public.as_slice().try_into().unwrap();
    let la = a
        .host
        .add_link(now(), LinkConfig::tcp_dial(&b_profile))
        .unwrap();
    // B: the mux reads TCPMuxInit and picks the host by profile key.
    let mut wire = a.take_transmit(la);
    let mut acc = TcpMuxAcceptor::new();
    let (n, key) = acc.handle_input(&wire[..5]).unwrap();
    assert_eq!(key, None);
    let (m, key) = acc.handle_input(&wire[n..]).unwrap();
    let mut mux = TcpMux::new();
    mux.handle(b_profile, "b");
    assert_eq!(mux.route(&key.unwrap()), Ok(&"b"));
    let lb = b.host.add_link(now(), LinkConfig::tcp_accept()).unwrap();
    b.input(lb, now(), &mut wire[n + m..], on_peer).unwrap();
    pump(a, la, b, lb, now(), 7, on_peer);
    (la, lb)
}

#[test]
fn aes_link_over_tcp_mux() {
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let mut on_peer = handlers(&[9]);
    let (la, lb) = connect_aes(&mut a, &mut b, &mut on_peer);
    assert_eq!(added(&a.events), vec![la]);
    assert_eq!(added(&b.events), vec![lb]);
    assert_eq!(a.host.link_kind(la), Some(LinkKind::Tcp));

    let secret = b"a secret frame body that must not appear in clear".to_vec();
    let w = a.host.open_writer(la, 9, a.host.qos_root(), 0).unwrap();
    let mut queues = vec![(w, VecDeque::from(vec![secret.clone(); 3]))];
    send_all(&mut a, &mut queues);
    let mut wire = a.take_transmit(la);
    assert!(!wire.windows(secret.len()).any(|x| x == secret.as_slice()));
    for c in wire.chunks_mut(11) {
        b.input(lb, now(), c, &mut on_peer).unwrap();
    }
    assert_eq!(b.recv.len(), 3);
    assert!(b.recv.iter().all(|r| r.3 == secret));
}

#[test]
fn aes_dialer_rejects_the_wrong_listener() {
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let c = Node::new(3, "c");
    // A dials C's key but reaches B.
    let c_profile: [u8; 32] = c.host.profile_key().public.as_slice().try_into().unwrap();
    let la = a
        .host
        .add_link(now(), LinkConfig::tcp_dial(&c_profile))
        .unwrap();
    let mut wire = a.take_transmit(la);
    let (n, _) = TcpMuxAcceptor::new().handle_input(&wire).unwrap();
    let lb = b.host.add_link(now(), LinkConfig::tcp_accept()).unwrap();
    b.input(lb, now(), &mut wire[n..], &mut no_handlers)
        .unwrap();
    let mut back = b.take_transmit(lb);
    let err = a.input(la, now(), &mut back, &mut no_handlers).unwrap_err();
    assert_eq!(
        err,
        Error::AesLink(strims_vnic::AesLinkError::PeerKeyMismatch)
    );
    assert!(!a.host.has_link(la));
    assert!(matches!(
        a.events.last(),
        Some(Event::LinkClosed {
            reason: CloseReason::Failed(_),
            ..
        })
    ));
}

#[test]
fn replaced_peer_does_not_remove_the_new_entry() {
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let la1 = a.host.add_link(now(), plain(500)).unwrap();
    let lb1 = b.host.add_link(now(), plain(500)).unwrap();
    pump(&mut a, la1, &mut b, lb1, now(), 64, &mut no_handlers);
    let la2 = a.host.add_link(now(), plain(500)).unwrap();
    let lb2 = b.host.add_link(now(), plain(500)).unwrap();
    b.events.clear();
    pump(&mut a, la2, &mut b, lb2, now(), 64, &mut no_handlers);
    let a_id = a.host.id();
    assert_eq!(
        b.events,
        vec![
            Event::PeerRemoved {
                peer: lb1,
                host_id: a_id
            },
            Event::LinkClosed {
                link: lb1,
                reason: CloseReason::Replaced
            },
            Event::PeerAdded {
                peer: lb2,
                host_id: a_id
            },
        ]
    );
    assert_eq!(b.host.get_peer(&a_id), Some(lb2));
    assert!(b.host.has_peer(&a_id));
    // The old link's transport closing later changes nothing.
    assert!(!b.host.remove_link(lb1));
    assert_eq!(b.host.get_peer(&a_id), Some(lb2));
    assert_eq!(b.host.peer_count(), 1);
    assert_eq!(b.host.metrics().links_active, 1);
}

#[test]
fn failed_peer_init_is_reported() {
    let mut b = Node::new(2, "b");
    let lb = b.host.add_link(now(), plain(500)).unwrap();
    let mut bad = Vec::new();
    encode_stream_message(
        &PeerInit {
            protocol_version: 1,
            certificate: None,
            node_platform: String::new(),
            node_version: String::new(),
        },
        &mut bad,
    )
    .unwrap();
    let err = b.input(lb, now(), &mut bad, &mut no_handlers).unwrap_err();
    assert_eq!(err, Error::PeerInit(PeerInitError::InvalidPeerCertificate));
    assert_eq!(
        b.events,
        vec![Event::LinkClosed {
            link: lb,
            reason: CloseReason::Failed(err)
        }]
    );
    assert!(!b.host.has_link(lb));
    assert_eq!(
        b.host.handle_link_input(lb, now(), &mut [0], &mut |_| {}),
        Err(Error::UnknownLink)
    );

    // Garbage that isn't a PeerInit.
    let lb = b.host.add_link(now(), plain(500)).unwrap();
    let err = b
        .input(lb, now(), &mut [0, 3, 0xff, 0xff, 0xff], &mut no_handlers)
        .unwrap_err();
    assert!(matches!(err, Error::PeerInit(PeerInitError::Read(_))));
}

#[test]
fn a_link_to_this_host_is_refused() {
    let mut a = Node::new(1, "a");
    let l1 = a.host.add_link(now(), plain(500)).unwrap();
    let l2 = a.host.add_link(now(), plain(500)).unwrap();
    let mut init1 = a.take_transmit(l1);
    let mut init2 = a.take_transmit(l2);
    let own = Error::PeerInit(PeerInitError::OwnHost);
    assert_eq!(
        a.input(l2, now(), &mut init1, &mut no_handlers),
        Err(own.clone())
    );
    assert_eq!(
        a.input(l1, now(), &mut init2, &mut no_handlers),
        Err(own.clone())
    );
    assert!(added(&a.events).is_empty());
    assert_eq!(
        a.events,
        vec![
            Event::LinkClosed {
                link: l2,
                reason: CloseReason::Failed(own.clone())
            },
            Event::LinkClosed {
                link: l1,
                reason: CloseReason::Failed(own)
            },
        ]
    );
    assert!(!a.host.has_link(l1) && !a.host.has_link(l2));
}

#[test]
fn expired_certificates_are_rejected() {
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let la = a.host.add_link(now(), plain(500)).unwrap();
    let lb = b.host.add_link(now(), plain(500)).unwrap();
    let mut init = a.take_transmit(la);
    let later = now() + Duration::HOUR * 2;
    // Arriving this late would otherwise hit the handshake deadline first.
    b.host.set_handshake_timeout(None);
    let err = b.input(lb, later, &mut init, &mut no_handlers).unwrap_err();
    match err {
        Error::PeerInit(PeerInitError::Certificate(e)) => {
            assert!(e.includes(strims_crypto::Error::NotAfterRange));
        }
        e => panic!("{e:?}"),
    }
}

#[test]
fn pre_handshake_input_is_bounded() {
    let mut b = Node::new(2, "b");
    let lb = b.host.add_link(now(), plain(500)).unwrap();
    // A prefix announcing a maximal PeerInit, then filler.
    let mut data = vec![0xff, 0xff];
    data.extend(std::iter::repeat_n(0x0a, 70_000));
    let mut max = 0;
    let mut err = None;
    for c in data.chunks_mut(4096) {
        match b.input(lb, now(), c, &mut no_handlers) {
            Ok(()) => max = max.max(b.host.link_input_buffered(lb).unwrap()),
            Err(e) => {
                err = Some(e);
                break;
            }
        }
    }
    assert!(max <= 2 + 65535, "{max}");
    // The 65535 filler bytes aren't a valid PeerInit.
    assert!(matches!(err, Some(Error::PeerInit(_))));
}

#[test]
fn ports_handlers_and_reservations() {
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let la = a.host.add_link(now(), plain(500)).unwrap();
    let lb = b.host.add_link(now(), plain(500)).unwrap();
    // Not established yet.
    assert_eq!(a.host.set_handler(la, 1, 1), Err(Error::PeerNotEstablished));
    pump(&mut a, la, &mut b, lb, now(), 64, &mut no_handlers);

    a.host.set_handler(la, 5, 5).unwrap();
    assert_eq!(a.host.set_handler(la, 5, 6), Err(Error::PortInUse(5)));
    assert_eq!(a.host.handler(la, 5), Some(5));
    assert_eq!(a.host.remove_handler(la, 5), Some(5));
    a.host.set_handler(la, 5, 6).unwrap();

    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..2000 {
        let p = a.host.reserve_port(la).unwrap();
        assert!(p >= 1000);
        assert!(seen.insert(p), "port {p} reserved twice");
    }
    for &p in seen.iter().take(10) {
        a.host.release_port(la, p);
    }
    // Ports with handlers are never reserved.
    a.host.set_handler(la, 1234, 0).unwrap();
    for _ in 0..1000 {
        assert_ne!(a.host.reserve_port(la).unwrap(), 1234);
    }
}

#[test]
fn unhandled_ports_are_dropped_and_counted() {
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let la = a.host.add_link(now(), plain(500)).unwrap();
    let lb = b.host.add_link(now(), plain(500)).unwrap();
    let mut on_peer = handlers(&[1]);
    pump(&mut a, la, &mut b, lb, now(), 64, &mut on_peer);
    let w1 = a.host.open_writer(la, 1, a.host.qos_root(), 0).unwrap();
    let w2 = a.host.open_writer(la, 2, a.host.qos_root(), 0).unwrap();
    let mut queues = vec![
        (w1, VecDeque::from(vec![vec![1]])),
        (w2, VecDeque::from(vec![vec![2], vec![3]])),
    ];
    send_all(&mut a, &mut queues);
    pump(&mut a, la, &mut b, lb, now(), 64, &mut on_peer);
    assert_eq!(b.recv.len(), 1);
    let m = b.host.metrics();
    assert_eq!(m.frame_read_count, 3);
    assert_eq!(m.frame_handler_not_found_count, 2);
}

#[test]
fn channel_pairs_connect_client_and_server() {
    const CLIENT: u16 = strims_vnic::ports::PEER_RPC_CLIENT_PORT;
    const SERVER: u16 = strims_vnic::ports::PEER_RPC_SERVER_PORT;
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let la = a.host.add_link(now(), plain(500)).unwrap();
    let lb = b.host.add_link(now(), plain(500)).unwrap();
    pump(&mut a, la, &mut b, lb, now(), 64, &mut no_handlers);
    // Tags: 10 = client half, 20 = server half.
    let (a_client, _a_server) = a
        .host
        .channel_pair(la, CLIENT, SERVER, a.host.qos_root(), 10, 20)
        .unwrap();
    let (_b_client, b_server) = b
        .host
        .channel_pair(lb, CLIENT, SERVER, b.host.qos_root(), 10, 20)
        .unwrap();
    // A second pair on the same ports fails and leaves the first intact.
    assert_eq!(
        a.host
            .channel_pair(la, CLIENT, SERVER, a.host.qos_root(), 1, 2),
        Err(Error::PortInUse(SERVER))
    );

    let mut q = vec![(a_client, VecDeque::from(vec![b"request".to_vec()]))];
    send_all(&mut a, &mut q);
    pump(&mut a, la, &mut b, lb, now(), 64, &mut no_handlers);
    assert_eq!(b.recv, vec![(lb, CLIENT, 20, b"request".to_vec())]);

    let mut q = vec![(b_server, VecDeque::from(vec![b"response".to_vec()]))];
    send_all(&mut b, &mut q);
    pump(&mut a, la, &mut b, lb, now(), 64, &mut no_handlers);
    assert_eq!(a.recv, vec![(la, SERVER, 10, b"response".to_vec())]);

    // Closing a channel removes the handler it installed (its read port).
    a.host.close_channel(a_client).unwrap();
    assert_eq!(a.host.handler(la, SERVER), None);
    assert_eq!(a.host.handler(la, CLIENT), Some(20));

    let ch = a.host.channel(la, 77, a.host.qos_root(), 77).unwrap();
    assert_eq!(a.host.writer(ch), Some((la, 77)));
    assert_eq!(a.host.handler(la, 77), Some(77));
}

#[test]
fn full_link_blocks_its_writers_until_drained() {
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let mut cfg = plain(100);
    cfg.send_buffer = 250;
    let la = a.host.add_link(now(), cfg.clone()).unwrap();
    let lb = b.host.add_link(now(), cfg).unwrap();
    pump(&mut a, la, &mut b, lb, now(), 64, &mut no_handlers);
    let w = a.host.open_writer(la, 1, a.host.qos_root(), 0).unwrap();
    a.host.set_backlog(w).unwrap();
    let mut fill_full = |f: strims_vnic::EgressFrame<'_, Tag>| {
        f.body.fill(0xaa);
        Fill::more(f.body.len())
    };
    // 250 bytes hold two 100-byte frames; then less than an MTU is free.
    assert_eq!(a.host.poll_egress(now(), &mut fill_full), 2);
    assert_eq!(a.host.poll_egress(now(), &mut fill_full), 0);
    let session = a.host.writer_session(w).unwrap();
    assert!(a.host.qos().is_blocked(session).unwrap());
    assert!(a.host.qos().is_backlogged(session).unwrap());
    // A writer opened on a blocked link starts blocked.
    let w2 = a.host.open_writer(la, 2, a.host.qos_root(), 0).unwrap();
    assert!(
        a.host
            .qos()
            .is_blocked(a.host.writer_session(w2).unwrap())
            .unwrap()
    );
    // Draining one frame frees an MTU.
    a.host.advance_link_transmit(la, 100).unwrap();
    assert!(!a.host.qos().is_blocked(session).unwrap());
    assert_eq!(a.host.poll_egress(now(), &mut fill_full), 1);
    assert_eq!(
        a.host.advance_link_transmit(la, 1000),
        Err(Error::AdvancePastEnd)
    );
    let m = a.host.link_metrics(la).unwrap();
    assert!(m.write_bytes > 0 && m.read_bytes > 0);
}

#[test]
fn closing_peers_and_classes() {
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let la = a.host.add_link(now(), plain(500)).unwrap();
    let lb = b.host.add_link(now(), plain(500)).unwrap();
    pump(&mut a, la, &mut b, lb, now(), 64, &mut no_handlers);

    // Removing a class closes the writers and channels under it.
    let class = a.host.add_qos_class(a.host.qos_root(), 1).unwrap();
    let net = a.host.add_qos_class(class, 1).unwrap();
    let ch = a.host.channel(la, 12, net, 12).unwrap();
    let w = a.host.open_writer(la, 13, net, 13).unwrap();
    a.host.remove_qos_class(class).unwrap();
    assert_eq!(a.host.writer(ch), None);
    assert_eq!(a.host.writer(w), None);
    assert_eq!(a.host.handler(la, 12), None);
    assert_eq!(a.host.set_backlog(w), Err(Error::UnknownWriter));

    let w = a.host.open_writer(la, 13, a.host.qos_root(), 13).unwrap();
    a.events.clear();
    assert!(a.host.close_peer(la));
    while let Some(e) = a.host.poll_event() {
        a.events.push(e);
    }
    assert_eq!(
        a.events,
        vec![
            Event::PeerRemoved {
                peer: la,
                host_id: b.host.id()
            },
            Event::LinkClosed {
                link: la,
                reason: CloseReason::Closed
            },
        ]
    );
    assert_eq!(a.host.writer(w), None);
    assert_eq!(a.host.peer_count(), 0);

    // The driver reports B's transport closed: no LinkClosed for it.
    b.events.clear();
    assert!(b.host.remove_link(lb));
    while let Some(e) = b.host.poll_event() {
        b.events.push(e);
    }
    assert_eq!(
        b.events,
        vec![Event::PeerRemoved {
            peer: lb,
            host_id: a.host.id()
        }]
    );
}

#[test]
fn rejects_bad_link_configs() {
    let mut a = Node::new(1, "a");
    assert_eq!(a.host.add_link(now(), plain(4)), Err(Error::InvalidMtu(4)));
    assert_eq!(
        a.host.add_link(now(), plain(65540)),
        Err(Error::InvalidMtu(65540))
    );
    assert!(a.host.add_link(now(), plain(65539)).is_ok());
    let mut cfg = plain(100);
    cfg.send_buffer = 99;
    assert_eq!(
        a.host.add_link(now(), cfg),
        Err(Error::SendBufferTooSmall(99))
    );
    a.host.set_max_peers(0);
    assert_eq!(a.host.max_peers(), i32::MAX as usize);
}
