// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use super::*;

type St = State<u8, u32>;
type Act = Action<u8, u32>;

const NET: u8 = 1;
const NET2: u8 = 2;

fn host(n: u8) -> ID {
    ID::from_bytes(&[n; 32])
}

fn t(secs: i64) -> Time {
    Time::EPOCH + Duration::from_secs(secs)
}

fn config(discovery: bool) -> Config {
    Config {
        discovery,
        ..Config::default()
    }
}

fn swarm_id() -> SwarmId {
    SwarmId::new(b"swarm")
}

fn drain(s: &mut St) -> Vec<Act> {
    std::iter::from_fn(|| s.poll_action()).collect()
}

fn ops(actions: &[Act]) -> Vec<Op<u8, u32>> {
    actions
        .iter()
        .filter_map(|a| match a {
            Action::Op(_, op) => Some(op.clone()),
            _ => None,
        })
        .collect()
}

/// One side of a link: its state, the link id it gave the other side,
/// and the node ops it asked for.
struct Side {
    id: ID,
    s: St,
    link: u64,
    ops: Vec<Op<u8, u32>>,
    closes: usize,
}

/// Two peers on [`NET`], delivering each other's calls in order.
struct Pair {
    a: Side,
    b: Side,
}

impl Pair {
    fn new() -> Pair {
        let (ia, ib) = (host(1), host(2));
        let mut a = Side {
            id: ia,
            s: St::new(ia, &config(false)),
            link: 0,
            ops: Vec::new(),
            closes: 0,
        };
        let mut b = Side {
            id: ib,
            s: St::new(ib, &config(false)),
            link: 0,
            ops: Vec::new(),
            closes: 0,
        };
        a.link = a.s.add_peer(ib);
        b.link = b.s.add_peer(ia);
        a.s.add_network_peer(NET, ib);
        b.s.add_network_peer(NET, ia);
        Pair { a, b }
    }

    /// Delivers queued calls both ways until neither side has any.
    fn settle(&mut self) {
        loop {
            let a = drain(&mut self.a.s);
            let b = drain(&mut self.b.s);
            if a.is_empty() && b.is_empty() {
                return;
            }
            deliver(&mut self.a, &mut self.b, a);
            deliver(&mut self.b, &mut self.a, b);
        }
    }

    /// Each side's channels (ours, the peer's).
    #[allow(clippy::type_complexity)]
    fn running(&self, id: TransferId) -> (Option<(u64, u64)>, Option<(u64, u64)>) {
        (
            self.a.s.running(self.b.id, id),
            self.b.s.running(self.a.id, id),
        )
    }

    fn assert_bound(&self, id: TransferId) {
        let (a, b) = self.running(id);
        let (a, b) = (a.expect("a runs"), b.expect("b runs"));
        assert_eq!(a, (b.1, b.0));
    }
}

/// Carries out `from`'s actions, calling `to`, whose requests always
/// succeed at the node.
fn deliver(from: &mut Side, to: &mut Side, actions: Vec<Act>) {
    for a in actions {
        match a {
            Action::Op(_, op) => from.ops.push(op),
            Action::Announce {
                id, link, channel, ..
            } => {
                let answer = match to.s.handle_announce(from.id, to.link, id, channel) {
                    Answer::Refuse => None,
                    Answer::Channel(c) | Answer::After(c, _) => Some(c),
                };
                from.s
                    .handle_announce_answer(to.id, link, id, channel, answer);
            }
            Action::Close { id, .. } => {
                from.closes += 1;
                to.s.handle_close(from.id, to.link, id);
            }
            Action::Search { .. } => {}
        }
    }
}

fn runs(ops: &[Op<u8, u32>]) -> usize {
    ops.iter().filter(|o| matches!(o, Op::Run { .. })).count()
}

fn stops(ops: &[Op<u8, u32>]) -> usize {
    ops.iter().filter(|o| matches!(o, Op::Stop { .. })).count()
}

#[test]
fn announce_binds_when_both_sides_published() {
    for a_first in [true, false] {
        let mut p = Pair::new();
        let ia = p.a.s.add(10, &swarm_id(), Vec::new());
        let ib = p.b.s.add(20, &swarm_id(), Vec::new());
        assert_eq!(ia, ib);
        let (first, second) = if a_first {
            (&mut p.a, &mut p.b)
        } else {
            (&mut p.b, &mut p.a)
        };
        first.s.publish(ia, NET, t(0));
        // The other side hasn't published: refused, nothing runs.
        let calls = drain(&mut first.s);
        deliver(first, second, calls);
        assert!(first.ops.is_empty());
        second.s.publish(ia, NET, t(0));
        p.settle();
        p.assert_bound(ia);
        assert_eq!((runs(&p.a.ops), runs(&p.b.ops)), (1, 1));
        let Op::Run { swarm, .. } = p.a.ops[0] else {
            panic!("a run");
        };
        assert_eq!(swarm, 10);
    }
}

#[test]
fn transfers_published_before_peers_join_are_announced_when_they_do() {
    let mut p = Pair::new();
    p.a.s.remove_network_peer(NET, p.b.id);
    p.b.s.remove_network_peer(NET, p.a.id);
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.b.s.add(20, &swarm_id(), Vec::new());
    p.a.s.publish(id, NET, t(0));
    p.b.s.publish(id, NET, t(0));
    p.settle();
    assert_eq!(p.running(id), (None, None));
    // membership reported before the link exists
    let link = p.a.s.add_peer(p.b.id);
    p.a.link = link;
    p.a.s.add_network_peer(NET, p.b.id);
    p.b.s.add_network_peer(NET, p.a.id);
    p.settle();
    p.assert_bound(id);
}

#[test]
fn simultaneous_announces_bind_once() {
    let mut p = Pair::new();
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.b.s.add(20, &swarm_id(), Vec::new());
    p.a.s.publish(id, NET, t(0));
    p.b.s.publish(id, NET, t(0));
    p.settle();
    p.assert_bound(id);
    assert_eq!((runs(&p.a.ops), runs(&p.b.ops)), (1, 1));
}

#[test]
fn announce_for_an_unpublished_transfer_is_refused() {
    let mut p = Pair::new();
    let unknown = TransferId::new(&swarm_id(), b"other");
    assert_eq!(
        p.a.s.handle_announce(p.b.id, p.a.link, unknown, 7),
        Answer::Refuse
    );
    // added but not published: no peer transfer to answer for
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    assert_eq!(
        p.a.s.handle_announce(p.b.id, p.a.link, id, 7),
        Answer::Refuse
    );
    // published on a network the peer isn't on
    p.a.s.publish(id, NET2, t(0));
    assert_eq!(
        p.a.s.handle_announce(p.b.id, p.a.link, id, 7),
        Answer::Refuse
    );
    // an unknown peer, and a stale link
    p.a.s.publish(id, NET, t(0));
    assert_eq!(p.a.s.handle_announce(host(9), 1, id, 7), Answer::Refuse);
    assert_eq!(
        p.a.s.handle_announce(p.b.id, p.a.link + 1, id, 7),
        Answer::Refuse
    );
    assert!(ops(&drain(&mut p.a.s)).is_empty());
}

#[test]
fn close_stops_the_channel_and_a_later_announce_restarts_it() {
    let mut p = Pair::new();
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.b.s.add(20, &swarm_id(), Vec::new());
    p.a.s.publish(id, NET, t(0));
    p.b.s.publish(id, NET, t(0));
    p.settle();
    let (_, (b_channel, a_channel)) = (p.running(id).0, p.running(id).1.unwrap());
    assert!(p.a.s.handle_close(p.b.id, p.a.link, id).is_some());
    assert_eq!(p.a.s.running(p.b.id, id), None);
    assert_eq!(
        ops(&drain(&mut p.a.s)),
        vec![Op::Stop {
            host: p.b.id,
            link: p.a.link,
            swarm: 10
        }]
    );
    // closing again is a no-op
    assert_eq!(p.a.s.handle_close(p.b.id, p.a.link, id), None);
    let Answer::After(c, _) = p.a.s.handle_announce(p.b.id, p.a.link, id, b_channel) else {
        panic!("restarted");
    };
    assert_eq!(c, a_channel);
    assert_eq!(p.a.s.running(p.b.id, id), Some((a_channel, b_channel)));
}

#[test]
fn reannounce_with_a_new_peer_channel_rebinds() {
    let mut p = Pair::new();
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.b.s.add(20, &swarm_id(), Vec::new());
    p.a.s.publish(id, NET, t(0));
    p.b.s.publish(id, NET, t(0));
    p.settle();
    let (a_channel, b_channel) = p.a.s.running(p.b.id, id).unwrap();
    // the same announce again answers without touching the node
    assert_eq!(
        p.a.s.handle_announce(p.b.id, p.a.link, id, b_channel),
        Answer::Channel(a_channel)
    );
    assert!(drain(&mut p.a.s).is_empty());
    assert!(matches!(
        p.a.s.handle_announce(p.b.id, p.a.link, id, 99),
        Answer::After(c, _) if c == a_channel
    ));
    let host = p.b.id;
    assert_eq!(
        ops(&drain(&mut p.a.s)),
        vec![
            Op::Stop {
                host,
                link: p.a.link,
                swarm: 10
            },
            Op::Run {
                host,
                link: p.a.link,
                id,
                swarm: 10,
                channel: a_channel,
                peer_channel: 99,
                notify: false,
            },
        ]
    );
}

#[test]
fn second_transfer_of_a_running_swarm_is_refused() {
    let mut p = Pair::new();
    let one = p.a.s.add(10, &swarm_id(), b"one".to_vec());
    let two = p.a.s.add(10, &swarm_id(), b"two".to_vec());
    p.b.s.add(20, &swarm_id(), b"one".to_vec());
    p.b.s.add(20, &swarm_id(), b"two".to_vec());
    for id in [one, two] {
        p.a.s.publish(id, NET, t(0));
        p.b.s.publish(id, NET, t(0));
    }
    p.settle();
    p.assert_bound(one);
    assert_eq!(p.running(two), (None, None));
    assert_eq!((runs(&p.a.ops), runs(&p.b.ops)), (1, 1));
}

#[test]
fn a_peer_that_runs_a_busy_swarm_is_closed() {
    let mut p = Pair::new();
    let one = p.a.s.add(10, &swarm_id(), b"one".to_vec());
    let two = p.a.s.add(10, &swarm_id(), b"two".to_vec());
    p.a.s.publish(one, NET, t(0));
    p.a.s.publish(two, NET, t(0));
    let calls = drain(&mut p.a.s);
    let channel = |want: TransferId| {
        calls
            .iter()
            .find_map(|a| match *a {
                Action::Announce { id, channel, .. } if id == want => Some(channel),
                _ => None,
            })
            .unwrap()
    };
    let (c1, c2) = (channel(one), channel(two));
    p.a.s
        .handle_announce_answer(p.b.id, p.a.link, one, c1, Some(51));
    p.a.s
        .handle_announce_answer(p.b.id, p.a.link, two, c2, Some(52));
    let actions = drain(&mut p.a.s);
    assert_eq!(runs(&ops(&actions)), 1);
    assert!(actions.contains(&Action::Close {
        host: p.b.id,
        link: p.a.link,
        id: two
    }));
}

#[test]
fn removing_a_transfer_closes_it_with_every_peer() {
    let mut p = Pair::new();
    let c = host(3);
    let mut sc = St::new(c, &config(false));
    let a_link_c = p.a.s.add_peer(c);
    p.a.s.add_network_peer(NET, c);
    sc.add_peer(p.a.id);
    sc.add_network_peer(NET, p.a.id);
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.b.s.add(20, &swarm_id(), Vec::new());
    sc.add(30, &swarm_id(), Vec::new());
    p.b.s.publish(id, NET, t(0));
    sc.publish(id, NET, t(0));
    p.settle();
    p.a.s.publish(id, NET, t(0));
    for a in drain(&mut p.a.s) {
        if let Action::Announce {
            host,
            link,
            id,
            channel,
        } = a
        {
            let peer = if host == c { &mut sc } else { &mut p.b.s };
            let link_on_peer = if host == c { 1 } else { p.b.link };
            let answer = match peer.handle_announce(p.a.id, link_on_peer, id, channel) {
                Answer::After(c, _) | Answer::Channel(c) => Some(c),
                Answer::Refuse => None,
            };
            p.a.s
                .handle_announce_answer(host, link, id, channel, answer);
        }
    }
    drain(&mut p.a.s);
    assert!(p.a.s.running(p.b.id, id).is_some());
    assert!(p.a.s.running(c, id).is_some());
    assert!(p.a.s.remove(id));
    let actions = drain(&mut p.a.s);
    assert_eq!(stops(&ops(&actions)), 2);
    for (h, link) in [(p.b.id, p.a.link), (c, a_link_c)] {
        assert!(actions.contains(&Action::Close { host: h, link, id }));
    }
    assert_eq!(p.a.s.running(p.b.id, id), None);
    assert!(!p.a.s.remove(id));
    assert!(p.a.s.list().is_empty());
}

#[test]
fn removing_a_transfer_with_an_announce_in_flight_closes_it() {
    let mut p = Pair::new();
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.a.s.publish(id, NET, t(0));
    assert!(matches!(drain(&mut p.a.s)[..], [Action::Announce { .. }]));
    // the peer may run it on that announce
    p.a.s.remove(id);
    assert_eq!(
        drain(&mut p.a.s),
        vec![Action::Close {
            host: p.b.id,
            link: p.a.link,
            id
        }]
    );
}

#[test]
fn leaving_the_last_shared_network_closes_the_peer_transfer() {
    let mut p = Pair::new();
    p.a.s.add_network_peer(NET2, p.b.id);
    p.b.s.add_network_peer(NET2, p.a.id);
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.b.s.add(20, &swarm_id(), Vec::new());
    for net in [NET, NET2] {
        p.a.s.publish(id, net, t(0));
        p.b.s.publish(id, net, t(0));
    }
    p.settle();
    p.assert_bound(id);
    p.a.s.remove_network_peer(NET, p.b.id);
    assert!(drain(&mut p.a.s).is_empty());
    assert!(p.a.s.running(p.b.id, id).is_some());
    p.a.s.remove_network_peer(NET2, p.b.id);
    let actions = drain(&mut p.a.s);
    assert_eq!(stops(&ops(&actions)), 1);
    assert!(actions.iter().any(|a| matches!(a, Action::Close { .. })));
    assert_eq!(p.a.s.running(p.b.id, id), None);

    // the node leaving its last shared network does the same
    let mut p = Pair::new();
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.b.s.add(20, &swarm_id(), Vec::new());
    p.a.s.publish(id, NET, t(0));
    p.b.s.publish(id, NET, t(0));
    p.settle();
    p.a.s.remove_network(NET);
    assert_eq!(stops(&ops(&drain(&mut p.a.s))), 1);
    assert!(!p.a.s.is_published(id, NET));
    assert_eq!(p.a.s.list(), vec![id]);
}

#[test]
fn unpublishing_closes_the_transfer_only_with_peers_it_shares_nowhere_else() {
    let mut p = Pair::new();
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.b.s.add(20, &swarm_id(), Vec::new());
    for net in [NET, NET2] {
        p.a.s.publish(id, net, t(0));
        p.b.s.publish(id, net, t(0));
    }
    p.settle();
    p.assert_bound(id);
    // B is a peer on NET alone, so NET2 shares nothing with it.
    assert!(p.a.s.unpublish(id, NET2));
    assert!(!p.a.s.unpublish(id, NET2));
    assert!(drain(&mut p.a.s).is_empty());
    assert!(p.a.s.is_published(id, NET) && !p.a.s.is_published(id, NET2));
    p.assert_bound(id);

    assert!(p.a.s.unpublish(id, NET));
    let actions = drain(&mut p.a.s);
    assert_eq!(stops(&ops(&actions)), 1);
    assert!(actions.iter().any(|a| matches!(a, Action::Close { .. })));
    assert_eq!(p.a.s.running(p.b.id, id), None);
    assert_eq!(p.a.s.list(), vec![id]);
    assert!(!p.a.s.unpublish(TransferId::default(), NET));

    // Published again, it is announced to the network's peers again.
    p.a.s.publish(id, NET, t(0));
    p.settle();
    p.assert_bound(id);
}

#[test]
fn unpublishing_withdraws_the_peer_index_entry_and_its_searches() {
    let mut s = St::new(host(1), &config(true));
    let id = s.add(10, &swarm_id(), Vec::new());
    s.publish(id, NET, t(0));
    s.publish(id, NET2, t(0));
    drain(&mut s);
    assert!(s.unpublish(id, NET2));
    assert_eq!(
        ops(&drain(&mut s)),
        vec![Op::IndexUnpublish { id, net: NET2 }]
    );
    s.handle_timeout(t(0));
    let searched: Vec<u8> = drain(&mut s)
        .into_iter()
        .filter_map(|a| match a {
            Action::Search { net, .. } => Some(net),
            _ => None,
        })
        .collect();
    assert_eq!(searched, vec![NET]);
}

#[test]
fn frame_error_blocks_the_peer_until_it_relinks() {
    let mut p = Pair::new();
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.b.s.add(20, &swarm_id(), Vec::new());
    p.a.s.publish(id, NET, t(0));
    p.b.s.publish(id, NET, t(0));
    p.settle();
    let (_, b_channel) = p.a.s.running(p.b.id, id).unwrap();
    p.a.s.transfer_failed(p.b.id);
    let actions = drain(&mut p.a.s);
    assert_eq!(stops(&ops(&actions)), 1);
    assert!(actions.contains(&Action::Close {
        host: p.b.id,
        link: p.a.link,
        id
    }));
    assert_eq!(
        p.a.s.handle_announce(p.b.id, p.a.link, id, b_channel),
        Answer::Refuse
    );
    p.a.s.add_network_peer(NET2, p.b.id);
    p.a.s.publish(id, NET2, t(0));
    assert!(drain(&mut p.a.s).is_empty());

    // a new link starts afresh
    p.a.s.remove_peer(p.b.id);
    p.b.s.remove_peer(p.a.id);
    p.a.link = p.a.s.add_peer(p.b.id);
    p.b.link = p.b.s.add_peer(p.a.id);
    p.a.s.add_network_peer(NET, p.b.id);
    p.b.s.add_network_peer(NET, p.a.id);
    p.settle();
    p.assert_bound(id);
}

#[test]
fn stale_announce_answers_are_ignored() {
    let mut p = Pair::new();
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.a.s.publish(id, NET, t(0));
    let old = drain(&mut p.a.s);
    let Some(&Action::Announce { link, channel, .. }) = old.first() else {
        panic!("announced");
    };
    // the transfer is recreated: its new announce has a new channel
    p.a.s.remove(id);
    p.a.s.add(10, &swarm_id(), Vec::new());
    p.a.s.publish(id, NET, t(0));
    drain(&mut p.a.s);
    p.a.s
        .handle_announce_answer(p.b.id, link, id, channel, Some(5));
    assert!(drain(&mut p.a.s).is_empty());
    assert_eq!(p.a.s.running(p.b.id, id), None);

    // an answer from an earlier link
    let channel = p.a.s.peers[&p.b.id].transfers[&id].channel;
    p.a.link = p.a.s.add_peer(p.b.id);
    p.a.s
        .handle_announce_answer(p.b.id, link, id, channel, Some(5));
    assert_eq!(p.a.s.running(p.b.id, id), None);
}

#[test]
fn failed_runs_go_idle_and_close_a_peer_that_started() {
    let mut p = Pair::new();
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.a.s.publish(id, NET, t(0));
    let Some(Action::Announce { link, channel, .. }) = drain(&mut p.a.s).pop() else {
        panic!("announced");
    };
    p.a.s
        .handle_announce_answer(p.b.id, link, id, channel, Some(7));
    let Some(Action::Op(
        _,
        Op::Run {
            peer_channel,
            notify,
            ..
        },
    )) = drain(&mut p.a.s).pop()
    else {
        panic!("run");
    };
    assert!(notify);
    p.a.s
        .run_failed(p.b.id, link, id, channel, peer_channel, notify);
    assert_eq!(p.a.s.running(p.b.id, id), None);
    assert_eq!(
        drain(&mut p.a.s),
        vec![Action::Close {
            host: p.b.id,
            link,
            id
        }]
    );
    // a failure the peer doesn't know about stays quiet
    let Answer::After(..) = p.a.s.handle_announce(p.b.id, link, id, 8) else {
        panic!("accepted");
    };
    drain(&mut p.a.s);
    p.a.s.run_failed(p.b.id, link, id, channel, 8, false);
    assert!(drain(&mut p.a.s).is_empty());
    assert_eq!(p.a.s.running(p.b.id, id), None);
}

#[test]
fn ops_of_an_earlier_link_or_an_undone_run_are_not_wanted() {
    let mut p = Pair::new();
    let id = p.a.s.add(10, &swarm_id(), Vec::new());
    p.a.s.publish(id, NET, t(0));
    drain(&mut p.a.s);
    let Answer::After(channel, _) = p.a.s.handle_announce(p.b.id, p.a.link, id, 7) else {
        panic!("accepted");
    };
    assert!(p.a.s.wants_run(p.b.id, p.a.link, id, channel, 7));
    // rebound before the run's turn
    p.a.s.handle_announce(p.b.id, p.a.link, id, 8);
    assert!(!p.a.s.wants_run(p.b.id, p.a.link, id, channel, 7));
    assert!(p.a.s.wants_run(p.b.id, p.a.link, id, channel, 8));
    let old = p.a.link;
    p.a.link = p.a.s.add_peer(p.b.id);
    assert!(!p.a.s.is_current(p.b.id, old));
    assert!(!p.a.s.wants_run(p.b.id, old, id, channel, 8));
    assert!(p.a.s.is_current(p.b.id, p.a.link));
}

#[test]
fn channels_are_never_reused_on_a_link() {
    let mut p = Pair::new();
    let mut seen = BTreeSet::new();
    for round in 0..4u8 {
        let id = p.a.s.add(10, &swarm_id(), vec![round]);
        p.a.s.publish(id, NET, t(0));
        for a in drain(&mut p.a.s) {
            if let Action::Announce { channel, .. } = a {
                assert!(channel != 0);
                assert!(seen.insert(channel), "channel {channel} reused");
            }
        }
        p.a.s.remove(id);
    }
    assert_eq!(seen.len(), 4);
}

#[test]
fn searches_repeat_every_interval_and_connect_with_backoff() {
    let me = host(1);
    let linked = host(2);
    let found = host(3);
    let mut s = St::new(me, &config(true));
    s.add_peer(linked);
    let id = s.add(10, &swarm_id(), b"salt".to_vec());
    s.publish(id, NET, t(0));
    assert_eq!(
        ops(&drain(&mut s)),
        vec![Op::IndexPublish {
            id,
            net: NET,
            key: b"swarm".to_vec(),
            salt: b"salt".to_vec()
        }]
    );
    assert_eq!(s.poll_timeout(), Some(t(0)));
    s.handle_timeout(t(0));
    assert_eq!(
        drain(&mut s),
        vec![Action::Search {
            id,
            net: NET,
            key: b"swarm".to_vec(),
            salt: b"salt".to_vec()
        }]
    );
    assert_eq!(s.poll_timeout(), Some(t(30)));
    // a search still in flight isn't started again
    s.handle_timeout(t(30));
    assert!(drain(&mut s).is_empty());
    for h in [me, linked, found] {
        s.search_found(id, NET, h, t(31));
    }
    assert_eq!(
        ops(&drain(&mut s)),
        vec![Op::Connect {
            net: NET,
            host: found
        }]
    );
    s.search_done(id, NET);
    // within the backoff the host isn't asked about again
    s.handle_timeout(t(60));
    assert_eq!(drain(&mut s).len(), 1);
    s.search_found(id, NET, found, t(61));
    assert!(drain(&mut s).is_empty());
    s.search_done(id, NET);
    s.handle_timeout(t(90));
    drain(&mut s);
    s.search_found(id, NET, found, t(91));
    assert_eq!(ops(&drain(&mut s)).len(), 1);
    s.search_done(id, NET);
    // hosts reported for a search that has ended are dropped
    s.search_found(id, NET, host(4), t(92));
    assert!(drain(&mut s).is_empty());
}

#[test]
fn contacted_hosts_are_bounded() {
    let mut s = St::new(
        host(1),
        &Config {
            max_contacted: 2,
            ..config(true)
        },
    );
    let id = s.add(10, &swarm_id(), Vec::new());
    s.publish(id, NET, t(0));
    s.handle_timeout(t(0));
    drain(&mut s);
    for h in 10..20 {
        s.search_found(id, NET, host(h), t(1));
    }
    assert_eq!(ops(&drain(&mut s)).len(), 10);
    assert_eq!(s.contacted(), 2);
}

#[test]
fn peer_index_entries_follow_publish_and_remove() {
    let mut s = St::new(host(1), &config(true));
    let id = s.add(10, &swarm_id(), Vec::new());
    assert!(!s.publish(TransferId::default(), NET, t(0)));
    s.publish(id, NET, t(0));
    s.publish(id, NET2, t(0));
    s.publish(id, NET, t(0));
    let published = ops(&drain(&mut s));
    assert_eq!(published.len(), 2);
    assert!(s.is_published(id, NET) && s.is_published(id, NET2));
    s.remove_network(NET2);
    assert_eq!(
        ops(&drain(&mut s)),
        vec![Op::IndexUnpublish { id, net: NET2 }]
    );
    s.handle_timeout(t(0));
    let searched: Vec<u8> = drain(&mut s)
        .into_iter()
        .filter_map(|a| match a {
            Action::Search { net, .. } => Some(net),
            _ => None,
        })
        .collect();
    assert_eq!(searched, vec![NET]);
    s.remove(id);
    assert_eq!(
        ops(&drain(&mut s)),
        vec![Op::IndexUnpublish { id, net: NET }]
    );
    assert_eq!(s.poll_timeout(), None);
}

#[test]
fn without_discovery_nothing_is_advertised_or_searched() {
    let mut s = St::new(host(1), &config(false));
    let id = s.add(10, &swarm_id(), Vec::new());
    s.publish(id, NET, t(0));
    assert!(drain(&mut s).is_empty());
    assert_eq!(s.poll_timeout(), None);
}

#[test]
fn find_and_add_keep_the_first_swarm() {
    let mut s = St::new(host(1), &config(false));
    let id = s.add(10, &swarm_id(), b"x".to_vec());
    assert_eq!(s.add(11, &swarm_id(), b"x".to_vec()), id);
    assert_eq!(s.find(&swarm_id(), b"x"), Some((id, 10)));
    assert_eq!(s.find(&swarm_id(), b"y"), None);
}
