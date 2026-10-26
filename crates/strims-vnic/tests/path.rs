// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The path measure through the public API: when probes go out, how echoes
//! answer them, what an echo yields, and what a forged one doesn't.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod common;

use common::{Node, Tag, now, pump};
use strims_base::path::PathSample;
use strims_base::time::{Duration, Time};
use strims_vnic::control::{self, Record};
use strims_vnic::{
    Event, Fill, FrameDecoder, Host, LinkConfig, LinkKind, PeerKey, WriterId, ports,
};

fn no_handlers(_: &mut Host<Tag>, _: PeerKey) {}

fn plain() -> LinkConfig {
    LinkConfig::new(LinkKind::Other, 1000)
}

fn ms(n: i64) -> Time {
    now() + Duration::from_millis(n)
}

/// Two hosts linked at `now()` with plain links. Nothing has been egressed,
/// so neither has probed yet.
fn linked() -> (Node, PeerKey, Node, PeerKey) {
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let la = a.host.add_link(now(), plain()).unwrap();
    let lb = b.host.add_link(now(), plain()).unwrap();
    pump(&mut a, la, &mut b, lb, now(), usize::MAX, &mut no_handlers);
    a.events.clear();
    b.events.clear();
    (a, la, b, lb)
}

/// The frames on a plain link's wire bytes.
fn frames(wire: &[u8]) -> Vec<(u16, Vec<u8>)> {
    let mut out = Vec::new();
    FrameDecoder::new().decode(wire, &mut |f| out.push((f.port, f.body.to_vec())));
    out
}

/// The control records on a plain link's wire bytes.
fn records(wire: &[u8]) -> Vec<Record> {
    frames(wire)
        .iter()
        .filter(|(p, _)| *p == ports::KEEPALIVE_PORT)
        .flat_map(|(_, b)| control::records(b).collect::<Vec<_>>())
        .collect()
}

fn probes(wire: &[u8]) -> Vec<u32> {
    records(wire)
        .into_iter()
        .filter_map(|r| match r {
            Record::Probe(n) => Some(n),
            Record::Echo(_) => None,
        })
        .collect()
}

fn samples(node: &mut Node) -> Vec<(PeerKey, PathSample)> {
    node.events
        .drain(..)
        .filter_map(|e| match e {
            Event::PathSample { peer, sample } => Some((peer, sample)),
            _ => None,
        })
        .collect()
}

fn idle(node: &mut Node, t: Time) {
    node.host.poll_egress(t, &mut |_| Fill::NONE);
}

fn control_frame(echo: Option<u32>, probe: Option<u32>) -> Vec<u8> {
    let mut buf = [0u8; control::MAX_CONTROL_FRAME_LEN];
    let n = control::encode(echo, probe, &mut buf).unwrap();
    buf[..n].to_vec()
}

#[test]
fn the_first_egress_probes_and_the_peer_echoes_it() {
    let (mut a, la, mut b, lb) = linked();
    idle(&mut a, ms(0));
    let mut wire = a.take_transmit(la);
    let [nonce] = probes(&wire)[..] else {
        panic!("one probe");
    };
    assert_eq!(wire.len(), 9);

    b.input(lb, ms(20), &mut wire, &mut no_handlers).unwrap();
    idle(&mut b, ms(20));
    let mut back = b.take_transmit(lb);
    // The echo, with B's own first probe in the same frame.
    let rs = records(&back);
    assert_eq!(rs.len(), 2);
    assert_eq!(rs[0], Record::Echo(nonce));
    assert!(matches!(rs[1], Record::Probe(_)));
    assert_eq!(back.len(), 14);

    a.input(la, ms(40), &mut back, &mut no_handlers).unwrap();
    let rtt = Duration::from_millis(40);
    assert_eq!(samples(&mut a), vec![(la, PathSample { at: ms(40), rtt })]);
    let path = a.host.link_path(la, ms(40)).unwrap();
    let r = path.reading.unwrap();
    assert_eq!((r.samples, r.rtt, r.base), (1, rtt, Some(rtt)));
    assert_eq!(path.unanswered, 0);
    assert_eq!(a.host.metrics().echo_read_count, 1);
    assert_eq!(b.host.metrics().echo_write_count, 1);
    // B owes nothing more: its probe waits for A's echo.
    idle(&mut b, ms(50));
    assert!(b.take_transmit(lb).is_empty());
}

/// Sends a frame of `len` bytes from `a` to `b` at `t`, and returns the
/// probes that went with it.
fn send(
    a: &mut Node,
    la: PeerKey,
    w: WriterId,
    b: &mut Node,
    lb: PeerKey,
    t: i64,
    len: usize,
) -> usize {
    a.host.set_backlog(w).unwrap();
    a.host.poll_egress(ms(t), &mut |f| {
        f.body[..len].fill(1);
        Fill::last(len)
    });
    let mut wire = a.take_transmit(la);
    b.input(lb, ms(t), &mut wire, &mut no_handlers).unwrap();
    probes(&wire).len()
}

#[test]
fn links_probe_each_interval_while_busy_and_each_second_while_quiet() {
    let (mut a, la, mut b, lb) = linked();
    b.host.set_handler(lb, 7, 7).unwrap();
    let w = a.host.open_writer(la, 7, a.host.qos_root(), 0).unwrap();
    // 1000 bytes every 25 ms, a chunk's worth in under 200 ms: at 0 (the
    // first egress), 200, 400, 600 and 800 ms.
    let busy: usize = (0..1000)
        .step_by(25)
        .map(|t| send(&mut a, la, w, &mut b, lb, t, 996))
        .sum();
    assert_eq!(busy, 5);
    // A byte every 50 ms: the busy bytes since 800 ms at 1000, then once a
    // second.
    let quiet: usize = (1000..3000)
        .step_by(50)
        .map(|t| send(&mut a, la, w, &mut b, lb, t, 1))
        .sum();
    assert_eq!(quiet, 2);
    assert_eq!(b.recv.len(), 80);
    // Idle, nothing more until the keepalive.
    for t in (3000..12_000).step_by(500) {
        idle(&mut a, ms(t));
        assert!(a.take_transmit(la).is_empty());
    }
}

#[test]
fn idle_links_probe_with_their_keepalives() {
    let (mut a, la, mut b, lb) = linked();
    idle(&mut a, ms(0));
    let mut first = a.take_transmit(la);
    b.input(lb, ms(0), &mut first, &mut no_handlers).unwrap();
    idle(&mut b, ms(0));
    let mut echo = b.take_transmit(lb);
    a.input(la, ms(10), &mut echo, &mut no_handlers).unwrap();
    samples(&mut a);

    // The keepalive clock started at the link's establishment.
    a.host.handle_timeout(ms(10_000));
    let mut wire = a.take_transmit(la);
    assert_eq!(probes(&wire).len(), 1);
    assert_eq!(frames(&wire).len(), 1);
    b.input(lb, ms(10_005), &mut wire, &mut no_handlers)
        .unwrap();
    idle(&mut b, ms(10_005));
    let mut echo = b.take_transmit(lb);
    a.input(la, ms(10_010), &mut echo, &mut no_handlers)
        .unwrap();
    let s = samples(&mut a);
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].1.rtt, Duration::from_millis(10));
    assert_eq!(a.host.poll_timeout(), Some(ms(20_000)));
}

#[test]
fn echoes_go_ahead_of_the_owners_frames() {
    let (mut a, la, mut b, lb) = linked();
    a.host.set_handler(la, 7, 7).unwrap();
    let w = b.host.open_writer(lb, 7, b.host.qos_root(), 0).unwrap();
    idle(&mut a, ms(0));
    let mut wire = a.take_transmit(la);
    let [nonce] = probes(&wire)[..] else {
        panic!("one probe");
    };
    b.input(lb, ms(5), &mut wire, &mut no_handlers).unwrap();
    b.host.set_backlog(w).unwrap();
    b.host.poll_egress(ms(5), &mut |f| {
        f.body[..3].copy_from_slice(b"abc");
        Fill::last(3)
    });
    let back = b.take_transmit(lb);
    let fs = frames(&back);
    assert_eq!(fs.len(), 2);
    assert_eq!(fs[0].0, ports::KEEPALIVE_PORT);
    assert_eq!(control::records(&fs[0].1).next(), Some(Record::Echo(nonce)));
    assert_eq!(fs[1], (7, b"abc".to_vec()));
}

#[test]
fn a_full_buffer_holds_the_echo_until_the_transport_takes_bytes() {
    let (mut a, la, mut b, lb) = linked();
    b.host.set_probe_interval(None);
    a.host.set_handler(la, 7, 7).unwrap();
    let w = b.host.open_writer(lb, 7, b.host.qos_root(), 0).unwrap();
    // Two whole frames fill B's buffer of two MTUs.
    b.host.set_backlog(w).unwrap();
    b.host.poll_egress(ms(0), &mut |f| {
        let n = f.body.len();
        Fill::more(n)
    });
    assert_eq!(b.host.poll_link_transmit(lb).unwrap().len(), 2000);

    idle(&mut a, ms(0));
    let mut wire = a.take_transmit(la);
    b.input(lb, ms(10), &mut wire, &mut no_handlers).unwrap();
    idle(&mut b, ms(10));
    assert_eq!(b.host.metrics().echo_write_count, 0);
    assert_eq!(b.host.poll_link_transmit(lb).unwrap().len(), 2000);

    // The transport takes some bytes 30 ms later; the echo goes next.
    b.host.advance_link_transmit(lb, 100).unwrap();
    idle(&mut b, ms(40));
    assert_eq!(b.host.metrics().echo_write_count, 1);
    assert_eq!(
        b.host.link_path(lb, ms(40)).unwrap().last_hold,
        Duration::from_millis(30)
    );
    let mut back = b.take_transmit(lb);
    a.input(la, ms(50), &mut back, &mut no_handlers).unwrap();
    // The round trip includes the hold: it is B's claim, not subtracted.
    assert_eq!(samples(&mut a)[0].1.rtt, Duration::from_millis(50));
}

#[test]
fn forged_and_replayed_echoes_are_ignored() {
    let (mut a, la, mut b, lb) = linked();
    idle(&mut a, ms(0));
    let mut wire = a.take_transmit(la);
    let [nonce] = probes(&wire)[..] else {
        panic!("one probe");
    };

    // A guess: ignored, and the probe it might have matched is void.
    let mut forged = control_frame(Some(nonce.wrapping_add(1)), None);
    a.input(la, ms(10), &mut forged, &mut no_handlers).unwrap();
    assert!(samples(&mut a).is_empty());
    let path = a.host.link_path(la, ms(10)).unwrap();
    assert_eq!((path.unknown_echoes, path.unanswered), (1, 0));
    assert_eq!(a.host.metrics().echo_unknown_count, 1);
    let mut late = control_frame(Some(nonce), None);
    a.input(la, ms(20), &mut late, &mut no_handlers).unwrap();
    assert!(samples(&mut a).is_empty());
    assert!(a.host.link_path(la, ms(20)).unwrap().reading.is_none());

    // A real echo of a later probe counts once; its replay doesn't.
    b.input(lb, ms(0), &mut wire, &mut no_handlers).unwrap();
    a.host.handle_timeout(ms(10_000));
    let mut wire = a.take_transmit(la);
    b.input(lb, ms(10_000), &mut wire, &mut no_handlers)
        .unwrap();
    idle(&mut b, ms(10_000));
    let back = b.take_transmit(lb);
    a.input(la, ms(10_020), &mut back.clone(), &mut no_handlers)
        .unwrap();
    assert_eq!(samples(&mut a).len(), 1);
    let echo_only: Vec<u8> = {
        let rs = records(&back);
        let Record::Echo(n) = rs[0] else {
            panic!("an echo first");
        };
        control_frame(Some(n), None)
    };
    a.input(la, ms(10_030), &mut echo_only.clone(), &mut no_handlers)
        .unwrap();
    assert!(samples(&mut a).is_empty());
    assert_eq!(a.host.link_path(la, ms(10_030)).unwrap().unknown_echoes, 3);
}

#[test]
fn a_replacing_link_measures_afresh() {
    let (mut a, la, mut b, lb) = linked();
    idle(&mut a, ms(0));
    let mut wire = a.take_transmit(la);
    b.input(lb, ms(0), &mut wire, &mut no_handlers).unwrap();
    idle(&mut b, ms(0));
    let mut back = b.take_transmit(lb);
    a.input(la, ms(30), &mut back, &mut no_handlers).unwrap();
    assert!(a.host.link_path(la, ms(30)).unwrap().reading.is_some());

    let la2 = a.host.add_link(ms(100), plain()).unwrap();
    let lb2 = b.host.add_link(ms(100), plain()).unwrap();
    pump(
        &mut a,
        la2,
        &mut b,
        lb2,
        ms(100),
        usize::MAX,
        &mut no_handlers,
    );
    assert!(a.host.link_path(la, ms(100)).is_none());
    let fresh = a.host.link_path(la2, ms(100)).unwrap();
    assert!(fresh.reading.is_none());
    assert_eq!((fresh.written, fresh.unanswered), (0, 0));
}

#[test]
fn a_host_that_doesnt_probe_still_echoes() {
    let (mut a, la, mut b, lb) = linked();
    a.host.set_probe_interval(None);
    idle(&mut a, ms(0));
    assert!(a.take_transmit(la).is_empty());
    idle(&mut b, ms(0));
    let mut wire = b.take_transmit(lb);
    let [nonce] = probes(&wire)[..] else {
        panic!("one probe");
    };
    a.input(la, ms(5), &mut wire, &mut no_handlers).unwrap();
    idle(&mut a, ms(5));
    let back = a.take_transmit(la);
    assert_eq!(records(&back), vec![Record::Echo(nonce)]);
    assert_eq!(a.host.metrics().probe_write_count, 0);
}

#[test]
fn probes_count_every_byte_the_link_buffers() {
    let (mut a, la, mut b, lb) = linked();
    b.host.set_handler(lb, 7, 7).unwrap();
    let w = a.host.open_writer(la, 7, a.host.qos_root(), 0).unwrap();
    a.host.set_backlog(w).unwrap();
    a.host.poll_egress(ms(0), &mut |f| {
        f.body[..100].fill(1);
        Fill::last(100)
    });
    let mut wire = a.take_transmit(la);
    // The probe (9) and the frame (4 + 100).
    assert_eq!(a.host.link_path(la, ms(0)).unwrap().written, 113);
    b.input(lb, ms(0), &mut wire, &mut no_handlers).unwrap();
    a.host.set_backlog(w).unwrap();
    a.host.poll_egress(ms(10), &mut |f| {
        f.body[..50].fill(1);
        Fill::last(50)
    });
    a.take_transmit(la);
    idle(&mut b, ms(10));
    let mut back = b.take_transmit(lb);
    a.input(la, ms(20), &mut back, &mut no_handlers).unwrap();
    let r = a.host.link_path(la, ms(20)).unwrap().reading.unwrap();
    // Written after the probe, by the echo's arrival: 104 + 54.
    assert_eq!(r.in_flight, 158);
    assert_eq!(r.written_since, 0);
}
