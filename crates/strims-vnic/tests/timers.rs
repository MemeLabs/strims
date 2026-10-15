// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Link timers through the public API: the handshake deadline, the bound on
//! links in the handshake, and keepalives.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod common;

use common::{Node, Tag, now, pump};
use strims_base::time::{Duration, Time};
use strims_vnic::{
    CloseReason, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_KEEPALIVE_INTERVAL, DEFAULT_MAX_PENDING_LINKS,
    Error, Event, Fill, FrameDecoder, Host, LinkConfig, LinkKind, PeerKey, TcpMuxAcceptor,
    TcpOptions, ports,
};

fn no_handlers(_: &mut Host<Tag>, _: PeerKey) {}

fn plain() -> LinkConfig {
    LinkConfig::new(LinkKind::Other, 1000)
}

fn closed(events: &[Event]) -> Vec<(PeerKey, CloseReason)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::LinkClosed { link, reason } => Some((*link, reason.clone())),
            _ => None,
        })
        .collect()
}

fn drain(node: &mut Node) {
    while let Some(e) = node.host.poll_event() {
        node.events.push(e);
    }
}

/// Two hosts linked at `now()` with plain links.
fn linked() -> (Node, PeerKey, Node, PeerKey) {
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let la = a.host.add_link(now(), plain()).unwrap();
    let lb = b.host.add_link(now(), plain()).unwrap();
    pump(&mut a, la, &mut b, lb, now(), usize::MAX, &mut no_handlers);
    assert!(a.host.peer(la).is_some() && b.host.peer(lb).is_some());
    (a, la, b, lb)
}

/// Two hosts linked at `now()` through the TCP mux and AES handshake.
fn linked_aes() -> (Node, PeerKey, Node, PeerKey) {
    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let b_profile: [u8; 32] = b.host.profile_key().public.as_slice().try_into().unwrap();
    let la = a
        .host
        .add_link(now(), LinkConfig::tcp_dial(&b_profile))
        .unwrap();
    let mut wire = a.take_transmit(la);
    let (n, key) = TcpMuxAcceptor::new().handle_input(&wire).unwrap();
    assert_eq!(key, Some(b_profile));
    let lb = b.host.add_link(now(), LinkConfig::tcp_accept()).unwrap();
    b.input(lb, now(), &mut wire[n..], &mut no_handlers)
        .unwrap();
    pump(&mut a, la, &mut b, lb, now(), 7, &mut no_handlers);
    assert!(a.host.peer(la).is_some() && b.host.peer(lb).is_some());
    (a, la, b, lb)
}

fn at(secs: i64) -> Time {
    now() + Duration::from_secs(secs)
}

// --- handshake deadline -----------------------------------------------------

#[test]
fn defaults() {
    let a = Node::new(1, "a");
    assert_eq!(a.host.handshake_timeout(), Some(DEFAULT_HANDSHAKE_TIMEOUT));
    assert_eq!(DEFAULT_HANDSHAKE_TIMEOUT, Duration::from_secs(25));
    assert_eq!(
        a.host.keepalive_interval(),
        Some(DEFAULT_KEEPALIVE_INTERVAL)
    );
    assert_eq!(DEFAULT_KEEPALIVE_INTERVAL, Duration::from_secs(10));
    // Below the TCP read timeout.
    assert!(DEFAULT_KEEPALIVE_INTERVAL < TcpOptions::default().read_timeout);
    assert_eq!(a.host.max_pending_links(), DEFAULT_MAX_PENDING_LINKS);
    assert_eq!(a.host.poll_timeout(), None);
}

/// A WebSocket peer that answers pings (so the transport never times out)
/// but never sends `PeerInit`, and one trickling a byte a second, both hit
/// the deadline.
#[test]
fn handshake_deadline_closes_a_silent_or_trickling_link() {
    let mut a = Node::new(1, "a");
    let silent = a.host.add_link(now(), LinkConfig::ws()).unwrap();
    let trickle = a.host.add_link(at(1), LinkConfig::ws()).unwrap();
    assert_eq!(a.host.pending_link_count(), 2);
    assert_eq!(a.host.poll_timeout(), Some(at(25)));

    // A length prefix promising a big PeerInit, then a byte a second.
    a.input(trickle, at(1), &mut [0xff, 0x7f], &mut no_handlers)
        .unwrap();
    for s in 2..=25 {
        a.input(trickle, at(s), &mut [0], &mut no_handlers).unwrap();
        a.host.handle_timeout(at(s));
    }
    drain(&mut a);
    // `silent` went at 25 s; `trickle` is still inside its 26 s deadline.
    assert_eq!(
        closed(&a.events),
        vec![(silent, CloseReason::Failed(Error::HandshakeTimeout))]
    );
    assert!(!a.host.has_link(silent));
    assert_eq!(a.host.poll_timeout(), Some(at(26)));

    a.host.handle_timeout(at(26));
    drain(&mut a);
    assert_eq!(
        closed(&a.events)[1],
        (trickle, CloseReason::Failed(Error::HandshakeTimeout))
    );
    assert_eq!(a.host.link_count(), 0);
    assert_eq!(a.host.poll_timeout(), None);
    assert_eq!(a.host.metrics().handshake_timeout_count, 2);
}

/// The TCP listener's side waits for the dialer's AES init: same deadline.
#[test]
fn handshake_deadline_covers_the_aes_stage() {
    let mut b = Node::new(2, "b");
    let lb = b.host.add_link(now(), LinkConfig::tcp_accept()).unwrap();
    b.host.handle_timeout(at(25));
    drain(&mut b);
    assert_eq!(
        closed(&b.events),
        vec![(lb, CloseReason::Failed(Error::HandshakeTimeout))]
    );
}

/// A driver that calls only `poll_egress` at `poll_timeout` still runs the
/// deadline. Input arriving after the deadline, before any timer ran, is
/// refused.
#[test]
fn handshake_deadline_without_handle_timeout() {
    let mut a = Node::new(1, "a");
    let l1 = a.host.add_link(now(), plain()).unwrap();
    a.host.poll_egress(at(25), &mut |_| Fill::NONE);
    drain(&mut a);
    assert_eq!(
        closed(&a.events),
        vec![(l1, CloseReason::Failed(Error::HandshakeTimeout))]
    );

    let mut a = Node::new(1, "a");
    let mut b = Node::new(2, "b");
    let la = a.host.add_link(now(), plain()).unwrap();
    let lb = b.host.add_link(now(), plain()).unwrap();
    let mut init = b.take_transmit(lb);
    let err = a
        .input(la, at(25), &mut init, &mut no_handlers)
        .unwrap_err();
    assert_eq!(err, Error::HandshakeTimeout);
    assert_eq!(
        closed(&a.events),
        vec![(la, CloseReason::Failed(Error::HandshakeTimeout))]
    );
    assert!(!a.host.has_link(la));
}

#[test]
fn completed_handshake_clears_the_deadline() {
    let (mut a, la, _b, _lb) = linked();
    // The next deadline is the keepalive check, not the handshake's.
    assert_eq!(a.host.poll_timeout(), Some(at(10)));
    for s in [10, 20, 25, 30, 60] {
        a.host.handle_timeout(at(s));
        a.take_transmit(la);
    }
    drain(&mut a);
    assert!(closed(&a.events).is_empty());
    assert!(a.host.peer(la).is_some());
    assert_eq!(a.host.metrics().handshake_timeout_count, 0);
}

#[test]
fn handshake_timeout_is_configurable() {
    let mut a = Node::new(1, "a");
    let l = a.host.add_link(now(), plain()).unwrap();
    // Applies to links already in the handshake.
    a.host.set_handshake_timeout(Some(Duration::from_secs(5)));
    assert_eq!(a.host.poll_timeout(), Some(at(5)));
    a.host.set_handshake_timeout(None);
    assert_eq!(a.host.poll_timeout(), None);
    a.host.handle_timeout(at(3600));
    assert!(a.host.has_link(l));
    // A duration that isn't positive disables it too.
    a.host.set_handshake_timeout(Some(Duration::ZERO));
    assert_eq!(a.host.handshake_timeout(), None);
    a.host.set_handshake_timeout(Some(Duration::from_secs(5)));
    a.host.handle_timeout(at(5));
    assert!(!a.host.has_link(l));
}

// --- links in the handshake -------------------------------------------------

#[test]
fn pending_links_are_bounded_by_evicting_the_oldest() {
    let (mut a, la, _b, _lb) = linked();
    a.host.set_max_pending_links(3);
    let p: Vec<PeerKey> = (0..3)
        .map(|i| {
            a.host
                .add_link(now() + Duration::from_millis(i), LinkConfig::ws())
                .unwrap()
        })
        .collect();
    // The established link doesn't count.
    assert_eq!(a.host.pending_link_count(), 3);
    drain(&mut a);
    assert!(closed(&a.events).is_empty());

    let p3 = a.host.add_link(at(1), LinkConfig::ws()).unwrap();
    let p4 = a.host.add_link(at(2), LinkConfig::ws()).unwrap();
    drain(&mut a);
    assert_eq!(
        closed(&a.events),
        vec![
            (p[0], CloseReason::Failed(Error::TooManyPendingLinks)),
            (p[1], CloseReason::Failed(Error::TooManyPendingLinks)),
        ]
    );
    assert_eq!(a.host.pending_link_count(), 3);
    assert!(a.host.has_link(p[2]) && a.host.has_link(p3) && a.host.has_link(p4));
    assert!(a.host.peer(la).is_some());
    assert_eq!(a.host.metrics().handshake_evict_count, 2);

    // Lowering the bound applies at the next add.
    a.host.set_max_pending_links(1);
    let p5 = a.host.add_link(at(3), LinkConfig::ws()).unwrap();
    assert_eq!(a.host.pending_link_count(), 1);
    assert!(a.host.has_link(p5));
    assert_eq!(a.host.metrics().handshake_evict_count, 5);

    // 0 is unbounded.
    a.host.set_max_pending_links(0);
    for _ in 0..DEFAULT_MAX_PENDING_LINKS + 10 {
        a.host.add_link(at(4), LinkConfig::ws()).unwrap();
    }
    assert_eq!(a.host.pending_link_count(), DEFAULT_MAX_PENDING_LINKS + 11);
}

#[test]
fn default_bound_evicts_beyond_the_default() {
    let mut a = Node::new(1, "a");
    let first = a.host.add_link(now(), LinkConfig::ws()).unwrap();
    for _ in 1..DEFAULT_MAX_PENDING_LINKS {
        a.host.add_link(now(), LinkConfig::ws()).unwrap();
    }
    assert!(a.host.has_link(first));
    a.host.add_link(at(1), LinkConfig::ws()).unwrap();
    assert!(!a.host.has_link(first));
    assert_eq!(a.host.pending_link_count(), DEFAULT_MAX_PENDING_LINKS);
}

// --- keepalives -------------------------------------------------------------

/// The frames on a plain link's wire bytes.
fn frames(wire: &[u8]) -> Vec<(u16, Vec<u8>)> {
    let mut out = Vec::new();
    FrameDecoder::new().decode(wire, &mut |f| out.push((f.port, f.body.to_vec())));
    out
}

#[test]
fn idle_links_send_keepalives() {
    let (mut a, la, mut b, lb) = linked();
    assert_eq!(a.host.poll_timeout(), Some(at(10)));
    a.host.handle_timeout(at(10) - Duration::MILLISECOND);
    assert!(a.take_transmit(la).is_empty());

    a.host.handle_timeout(at(10));
    let mut wire = a.take_transmit(la);
    // An empty frame on port 65535.
    assert_eq!(wire, [0xff, 0xff, 0, 0]);
    assert_eq!(frames(&wire), vec![(ports::KEEPALIVE_PORT, vec![])]);
    assert_eq!(a.host.metrics().keepalive_write_count, 1);
    assert_eq!(a.host.poll_timeout(), Some(at(20)));

    // B drops it: not dispatched, not an unhandled frame, but noted.
    assert!(!b.host.peer(lb).unwrap().sends_keepalives());
    b.input(lb, at(10), &mut wire, &mut no_handlers).unwrap();
    assert!(b.recv.is_empty());
    let m = b.host.metrics();
    assert_eq!(m.keepalive_read_count, 1);
    assert_eq!(m.frame_read_count, 0);
    assert_eq!(m.frame_handler_not_found_count, 0);
    assert!(b.host.peer(lb).unwrap().sends_keepalives());

    // Runs through poll_egress too.
    a.host.poll_egress(at(20), &mut |_| Fill::NONE);
    assert_eq!(a.take_transmit(la), [0xff, 0xff, 0, 0]);
}

#[test]
fn busy_links_send_no_keepalives() {
    let (mut a, la, mut b, lb) = linked();
    b.host.set_handler(lb, 7, 7).unwrap();
    let w = a.host.open_writer(la, 7, a.host.qos_root(), 0).unwrap();
    let mut t = now();
    for _ in 0..30 {
        t += Duration::from_secs(5);
        a.host.handle_timeout(t);
        a.host.set_backlog(w).unwrap();
        a.host.poll_egress(t, &mut |f| {
            f.body[0] = 1;
            Fill::last(1)
        });
        let mut wire = a.take_transmit(la);
        b.input(lb, t, &mut wire, &mut no_handlers).unwrap();
    }
    assert_eq!(b.recv.len(), 30);
    assert_eq!(a.host.metrics().keepalive_write_count, 0);
    assert_eq!(b.host.metrics().keepalive_read_count, 0);
    // The check is lazy: frames don't move the timer, so it wakes 5 s after
    // the last frame, finds that frame, and re-arms from it.
    assert_eq!(a.host.poll_timeout(), Some(t + Duration::from_secs(5)));
    a.host.handle_timeout(t + Duration::from_secs(5));
    assert!(a.take_transmit(la).is_empty());
    assert_eq!(a.host.poll_timeout(), Some(t + DEFAULT_KEEPALIVE_INTERVAL));
}

#[test]
fn keepalives_wait_behind_unsent_bytes() {
    let (mut a, la, _b, _lb) = linked();
    let w = a.host.open_writer(la, 7, a.host.qos_root(), 0).unwrap();
    a.host.set_backlog(w).unwrap();
    a.host.poll_egress(now(), &mut |f| {
        f.body[..3].copy_from_slice(b"abc");
        Fill::last(3)
    });
    let pending = a.host.poll_link_transmit(la).unwrap().to_vec();
    // The transport hasn't taken the frame: nothing to add.
    a.host.handle_timeout(at(10));
    assert_eq!(a.host.poll_link_transmit(la).unwrap(), pending.as_slice());
    assert_eq!(a.host.metrics().keepalive_write_count, 0);
    assert_eq!(a.host.poll_timeout(), Some(at(20)));
    a.take_transmit(la);
    a.host.handle_timeout(at(20));
    assert_eq!(a.take_transmit(la), [0xff, 0xff, 0, 0]);
}

#[test]
fn keepalives_on_aes_links_keep_the_stream_in_step() {
    let (mut a, la, mut b, lb) = linked_aes();
    b.host.set_handler(lb, 9, 9).unwrap();
    a.host.handle_timeout(at(10));
    let mut wire = a.take_transmit(la);
    assert_eq!(wire.len(), 4);
    assert_ne!(wire, [0xff, 0xff, 0, 0], "encrypted");
    b.input(lb, at(10), &mut wire, &mut no_handlers).unwrap();
    assert_eq!(b.host.metrics().keepalive_read_count, 1);
    // A frame after it still decrypts.
    let w = a.host.open_writer(la, 9, a.host.qos_root(), 0).unwrap();
    a.host.set_backlog(w).unwrap();
    a.host.poll_egress(at(11), &mut |f| {
        f.body[..5].copy_from_slice(b"hello");
        Fill::last(5)
    });
    let mut wire = a.take_transmit(la);
    b.input(lb, at(11), &mut wire, &mut no_handlers).unwrap();
    assert_eq!(b.recv, vec![(lb, 9, 9, b"hello".to_vec())]);
}

#[test]
fn keepalive_interval_is_configurable() {
    let (mut a, la, _b, _lb) = linked();
    a.host.set_keepalive_interval(Some(Duration::from_secs(3)));
    assert_eq!(a.host.poll_timeout(), Some(at(3)));
    a.host.handle_timeout(at(3));
    assert_eq!(a.take_transmit(la).len(), 4);
    a.host.set_keepalive_interval(None);
    assert_eq!(a.host.poll_timeout(), None);
    a.host.handle_timeout(at(3600));
    assert!(a.take_transmit(la).is_empty());
    a.host.set_keepalive_interval(Some(Duration::ZERO));
    assert_eq!(a.host.keepalive_interval(), None);
}

#[test]
fn keepalive_port_is_reserved() {
    let (mut a, la, _b, _lb) = linked();
    let root = a.host.qos_root();
    let k = ports::KEEPALIVE_PORT;
    assert_eq!(a.host.set_handler(la, k, 0), Err(Error::PortInUse(k)));
    assert_eq!(a.host.open_writer(la, k, root, 0), Err(Error::PortInUse(k)));
    assert_eq!(a.host.channel(la, k, root, 0), Err(Error::PortInUse(k)));
    assert_eq!(
        a.host.channel_pair(la, k, 4, root, 0, 1),
        Err(Error::PortInUse(k))
    );
    // The failed pair rolled back its first handler.
    assert_eq!(a.host.handler(la, 4), None);
}

/// A driver that closes a link after `TcpOptions::read_timeout` (25 s)
/// without input, over five idle minutes of virtual time. Returns the time
/// each side's driver timed out, if it did.
fn idle_run(keepalive_a: bool, keepalive_b: bool) -> (Node, PeerKey, [Option<Time>; 2]) {
    let (mut a, la, mut b, lb) = linked();
    if !keepalive_a {
        a.host.set_keepalive_interval(None);
    }
    if !keepalive_b {
        b.host.set_keepalive_interval(None);
    }
    let read_timeout = TcpOptions::default().read_timeout;
    let mut last_read = [now(); 2];
    let mut timed_out = [None; 2];
    let end = now() + Duration::from_secs(300);
    loop {
        let t = [
            a.host.poll_timeout(),
            b.host.poll_timeout(),
            Some(last_read[0] + read_timeout),
            Some(last_read[1] + read_timeout),
        ]
        .into_iter()
        .flatten()
        .min()
        .unwrap();
        if t > end {
            break;
        }
        a.host.handle_timeout(t);
        b.host.handle_timeout(t);
        let mut ab = a.take_transmit(la);
        let mut ba = b.take_transmit(lb);
        if !ab.is_empty() {
            b.input(lb, t, &mut ab, &mut no_handlers).unwrap();
            last_read[1] = t;
        }
        if !ba.is_empty() {
            a.input(la, t, &mut ba, &mut no_handlers).unwrap();
            last_read[0] = t;
        }
        if last_read[0] + read_timeout <= t || last_read[1] + read_timeout <= t {
            for (i, r) in last_read.iter().enumerate() {
                if *r + read_timeout <= t {
                    timed_out[i] = Some(t);
                }
            }
            break;
        }
    }
    (a, la, timed_out)
}

#[test]
fn keepalives_keep_idle_links_under_a_read_timeout() {
    // Both send keepalives: five idle minutes, no timeout.
    let (_, _, timed_out) = idle_run(true, true);
    assert_eq!(timed_out, [None, None]);

    // Neither does: both read timeouts fire at 25 s.
    let (_, _, timed_out) = idle_run(false, false);
    assert_eq!(timed_out, [Some(at(25)), Some(at(25))]);

    // Only A does: B's read deadline stays fresh, A's driver times out B's
    // silence, and A can tell B sends no keepalives.
    let (a, la, timed_out) = idle_run(true, false);
    assert_eq!(timed_out, [Some(at(25)), None]);
    assert!(!a.host.peer(la).unwrap().sends_keepalives());
}
