// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Scheduling properties of `Control`, through the public egress API:
//! weighted shares at every level of the hierarchy, no banked credit, the
//! rate limit over sliding windows, and determinism.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;

use proptest::prelude::*;
use rand_core::Rng;
use strims_base::rng::{DetRng, det_rng};
use strims_base::time::{Duration, Time};
use strims_qos::{ClassId, Control, MAX_WEIGHT, MIN_WEIGHT, NodeId, SessionId};

/// A tree node as the test sees it: its parent and weight.
#[derive(Clone, Copy)]
struct Spec {
    parent: Option<usize>,
    weight: u64,
    session: bool,
}

/// A scheduler built from a list of specs (index 0 is the root) that tracks
/// bytes sent through every node.
struct Tree {
    q: Control,
    specs: Vec<Spec>,
    ids: Vec<NodeId>,
    bytes: Vec<u64>,
    frames: Vec<u64>,
}

impl Tree {
    fn new(q: Control, specs: &[(Option<usize>, u64, bool)]) -> Tree {
        let mut q = q;
        let mut ids = vec![NodeId::Class(q.root())];
        let mut out = vec![Spec {
            parent: None,
            weight: 0,
            session: false,
        }];
        for &(parent, weight, session) in specs.iter().skip(1) {
            let p = parent.unwrap();
            let NodeId::Class(pc) = ids[p] else {
                panic!("parent {p} is a session")
            };
            ids.push(if session {
                NodeId::Session(q.add_session(pc, weight).unwrap())
            } else {
                NodeId::Class(q.add_class(pc, weight).unwrap())
            });
            out.push(Spec {
                parent,
                weight,
                session,
            });
        }
        let n = out.len();
        Tree {
            q,
            specs: out,
            ids,
            bytes: vec![0; n],
            frames: vec![0; n],
        }
    }

    fn session(&self, i: usize) -> SessionId {
        match self.ids[i] {
            NodeId::Session(s) => s,
            NodeId::Class(_) => panic!("{i} is a class"),
        }
    }

    fn index(&self, s: SessionId) -> usize {
        self.ids
            .iter()
            .position(|id| *id == NodeId::Session(s))
            .unwrap()
    }

    fn sessions(&self) -> Vec<usize> {
        (0..self.specs.len())
            .filter(|&i| self.specs[i].session)
            .collect()
    }

    fn children(&self, i: usize) -> Vec<usize> {
        (0..self.specs.len())
            .filter(|&c| self.specs[c].parent == Some(i))
            .collect()
    }

    fn has_session(&self, i: usize) -> bool {
        self.specs[i].session || self.children(i).into_iter().any(|c| self.has_session(c))
    }

    fn account(&mut self, i: usize, len: u64) {
        let mut k = Some(i);
        while let Some(n) = k {
            self.bytes[n] += len;
            self.frames[n] += 1;
            k = self.specs[n].parent;
        }
    }

    /// Dequeues and completes one frame. `actual` maps (session index,
    /// hint) to the bytes sent.
    fn step(&mut self, now: Time, actual: &mut impl FnMut(usize, usize) -> usize) -> Option<usize> {
        let (s, hint) = self.q.dequeue(now)?;
        let i = self.index(s);
        let len = actual(i, hint);
        self.q.complete(s, len, now).unwrap();
        self.account(i, len as u64);
        Some(i)
    }

    /// Largest |bytes_child - share_child * bytes_parent| over the children
    /// of every class whose children are all continuously backlogged, where
    /// share is the weight (clamped to `MIN_WEIGHT..=MAX_WEIGHT`) normalized
    /// over the siblings.
    fn max_deviation(&self) -> f64 {
        let mut worst: f64 = 0.0;
        for p in 0..self.specs.len() {
            // Subtrees without sessions never compete.
            let kids: Vec<usize> = self
                .children(p)
                .into_iter()
                .filter(|&c| self.has_session(c))
                .collect();
            if kids.is_empty() {
                continue;
            }
            let phis: Vec<f64> = kids
                .iter()
                .map(|&c| self.specs[c].weight.clamp(MIN_WEIGHT, MAX_WEIGHT) as f64)
                .collect();
            let sum: f64 = phis.iter().sum();
            for (&c, phi) in kids.iter().zip(phis) {
                let fluid = self.bytes[p] as f64 * phi / sum;
                worst = worst.max((self.bytes[c] as f64 - fluid).abs());
            }
        }
        worst
    }
}

/// root -> { A(1) -> {a1(1), a2(2)}, B(3) -> {b1(1), b2(1), b3(5)}, c(2) }
const HIERARCHY: &[(Option<usize>, u64, bool)] = &[
    (None, 0, false),
    (Some(0), 1, false), // 1: A
    (Some(1), 1, true),  // 2: a1
    (Some(1), 2, true),  // 3: a2
    (Some(0), 3, false), // 4: B
    (Some(4), 1, true),  // 5: b1
    (Some(4), 1, true),  // 6: b2
    (Some(4), 5, true),  // 7: b3
    (Some(0), 2, true),  // 8: c
];

const MTU: usize = 1500;

/// Under continuous backlog with fixed frames, every node's byte count stays
/// within 2 frames of its fluid (GPS) share of its parent's bytes at every
/// prefix of the schedule, at every level.
#[test]
fn hierarchical_shares_fixed_frames() {
    let mut tree = Tree::new(Control::new(), HIERARCHY);
    for i in tree.sessions() {
        tree.q.set_backlog(tree.session(i), 1000).unwrap();
    }
    let mut worst: f64 = 0.0;
    for _ in 0..60_000 {
        tree.step(Time::EPOCH, &mut |_, hint| hint).unwrap();
        worst = worst.max(tree.max_deviation());
    }
    assert!(worst <= 2.0 * 1000.0, "max deviation {worst} bytes");
    // A:B:c = 1:3:2, and within A, a1:a2 = 1:2.
    let total = tree.bytes[0] as f64;
    let share = |i: usize| tree.bytes[i] as f64 / total;
    assert!((share(1) - 1.0 / 6.0).abs() < 1e-3);
    assert!((share(4) - 3.0 / 6.0).abs() < 1e-3);
    assert!((share(8) - 2.0 / 6.0).abs() < 1e-3);
    assert!((tree.bytes[3] as f64 / tree.bytes[1] as f64 - 2.0 / 3.0).abs() < 1e-3);
}

/// Same, with per-session random frame sizes in [64, MTU] that are known in
/// advance (hint == actual): within 2 MTUs at every prefix.
#[test]
fn hierarchical_shares_random_frames() {
    let mut rng = det_rng(11);
    let mut tree = Tree::new(Control::new(), HIERARCHY);
    let mut next_len = BTreeMap::new();
    for i in tree.sessions() {
        let len = 64 + (rng.next_u32() as usize % (MTU - 63));
        next_len.insert(i, len);
        tree.q.set_backlog(tree.session(i), len).unwrap();
    }
    let mut worst: f64 = 0.0;
    for _ in 0..60_000 {
        let i = tree.step(Time::EPOCH, &mut |_, hint| hint).unwrap();
        let len = 64 + (rng.next_u32() as usize % (MTU - 63));
        next_len.insert(i, len);
        tree.q.set_backlog(tree.session(i), len).unwrap();
        worst = worst.max(tree.max_deviation());
    }
    assert!(worst <= 2.0 * MTU as f64, "max deviation {worst} bytes");
}

/// Owners that hint the MTU but fill less: tags are charged with the actual
/// length, so byte shares still follow the weights, within 2 MTUs at every
/// prefix.
#[test]
fn hierarchical_shares_with_inexact_hints() {
    let mut rng = det_rng(12);
    let mut tree = Tree::new(Control::new(), HIERARCHY);
    for i in tree.sessions() {
        tree.q.set_backlog(tree.session(i), MTU).unwrap();
    }
    let mut worst: f64 = 0.0;
    for _ in 0..60_000 {
        tree.step(Time::EPOCH, &mut |_, _| {
            64 + (rng.next_u32() as usize % (MTU - 63))
        })
        .unwrap();
        worst = worst.max(tree.max_deviation());
    }
    assert!(worst <= 2.0 * MTU as f64, "max deviation {worst} bytes");
}

fn pair(w_a: u64, w_b: u64) -> (Control, SessionId, SessionId) {
    let mut q = Control::new();
    let a = q.add_session(q.root(), w_a).unwrap();
    let b = q.add_session(q.root(), w_b).unwrap();
    (q, a, b)
}

/// Runs `n` frames of `len` bytes and returns frames sent by `a` and `b`.
/// `rearrive` makes each session go idle and come back after every frame,
/// like a writer that queues one frame at a time.
fn run_pair(q: &mut Control, a: SessionId, b: SessionId, n: usize, rearrive: bool) -> (u64, u64) {
    let (mut na, mut nb) = (0, 0);
    for _ in 0..n {
        let (s, hint) = q.dequeue(Time::EPOCH).unwrap();
        if rearrive {
            q.clear_backlog(s).unwrap();
        }
        q.complete(s, hint, Time::EPOCH).unwrap();
        if rearrive {
            q.set_backlog(s, hint).unwrap();
        }
        if s == a {
            na += 1;
        } else if s == b {
            nb += 1;
        }
    }
    (na, nb)
}

/// Weights 1000 and 50 split frames 20:1.
///
/// The next head is chosen at the next `dequeue`, so an owner that goes idle
/// and returns right after `complete` still competes. It forfeits its lag
/// on each return, as WF2Q+ prescribes for a session that goes idle, so for
/// such owners the ratio holds within 5%.
#[test]
fn weights_apply_to_continuous_and_one_frame_at_a_time_writers() {
    let (mut q, hi, lo) = pair(1000, 50);
    q.set_backlog(hi, 1000).unwrap();
    q.set_backlog(lo, 1000).unwrap();
    let (nh, nl) = run_pair(&mut q, hi, lo, 21_000, false);
    assert!((nh as i64 - 20_000).abs() <= 2, "{nh} {nl}");
    assert!((nl as i64 - 1_000).abs() <= 2, "{nh} {nl}");

    let (mut q, hi, lo) = pair(1000, 50);
    q.set_backlog(hi, 1000).unwrap();
    q.set_backlog(lo, 1000).unwrap();
    let (nh, nl) = run_pair(&mut q, hi, lo, 21_000, true);
    assert!((nl as f64 - 1_000.0).abs() <= 50.0, "{nh} {nl}");
}

/// Weights have full resolution across `1..=MAX_WEIGHT`: two continuously
/// backlogged sessions split frames in the ratio of their weights, within 2
/// frames at every prefix.
#[test]
fn weights_share_proportionally_across_the_range() {
    for (wa, wb, rounds) in [
        (600, 900, 10),
        (501, 1000, 10),
        (950, 1000, 20),
        (1, 1000, 100),
        (50, 1000, 20),
    ] {
        let (mut q, a, b) = pair(wa, wb);
        q.set_backlog(a, 1000).unwrap();
        q.set_backlog(b, 1000).unwrap();
        let n = (wa + wb) * rounds;
        let (mut na, mut nb) = (0i64, 0i64);
        for i in 1..=n as i64 {
            let (x, y) = run_pair(&mut q, a, b, 1, false);
            na += x as i64;
            nb += y as i64;
            let fluid = i * wa as i64 / (wa + wb) as i64;
            assert!((na - fluid).abs() <= 2, "{wa}:{wb} after {i}: a {na}");
        }
        assert_eq!(
            (na, nb),
            ((wa * rounds) as i64, (wb * rounds) as i64),
            "{wa}:{wb}"
        );
    }
}

/// The same split for owners that go idle after every frame and return
/// right after `complete` (one frame at a time). Each return forfeits the
/// session's lag, as WF2Q+ prescribes, so the split is approximate (within
/// 0.2%), which still separates 950 from 1000.
#[test]
fn one_frame_at_a_time_writers_follow_upper_range_weights() {
    for (wa, wb) in [(600, 900), (501, 1000), (950, 1000)] {
        let (mut q, a, b) = pair(wa, wb);
        q.set_backlog(a, 1000).unwrap();
        q.set_backlog(b, 1000).unwrap();
        let n = (wa + wb) as usize * 20;
        let (na, nb) = run_pair(&mut q, a, b, n, true);
        let want = n as f64 * wa as f64 / (wa + wb) as f64;
        assert!(
            (na as f64 - want).abs() <= 0.002 * n as f64,
            "{wa}:{wb}: {na} {nb}, want {want}"
        );
    }
}

/// Upper-range weights at every level of a hierarchy: every node stays
/// within 2 MTUs of its fluid share at every prefix, and the final shares
/// match the weights.
#[test]
fn hierarchical_shares_upper_range_weights() {
    // root -> { A(600) -> {a1(950), a2(1000)}, B(900) -> {b1(501), b2(1000)}, c(50) }
    let mut tree = Tree::new(
        Control::new(),
        &[
            (None, 0, false),
            (Some(0), 600, false), // 1: A
            (Some(1), 950, true),  // 2: a1
            (Some(1), 1000, true), // 3: a2
            (Some(0), 900, false), // 4: B
            (Some(4), 501, true),  // 5: b1
            (Some(4), 1000, true), // 6: b2
            (Some(0), 50, true),   // 7: c
        ],
    );
    let mut rng = det_rng(13);
    for i in tree.sessions() {
        tree.q.set_backlog(tree.session(i), MTU).unwrap();
    }
    let mut worst: f64 = 0.0;
    for _ in 0..60_000 {
        tree.step(Time::EPOCH, &mut |_, _| {
            64 + (rng.next_u32() as usize % (MTU - 63))
        })
        .unwrap();
        worst = worst.max(tree.max_deviation());
    }
    assert!(worst <= 2.0 * MTU as f64, "max deviation {worst} bytes");
    let ratio = |a: usize, b: usize| tree.bytes[a] as f64 / tree.bytes[b] as f64;
    assert!((ratio(1, 4) - 600.0 / 900.0).abs() < 1e-3);
    assert!((ratio(2, 3) - 950.0 / 1000.0).abs() < 1e-3);
    assert!((ratio(5, 6) - 501.0 / 1000.0).abs() < 1e-3);
    assert!((ratio(7, 1) - 50.0 / 600.0).abs() < 1e-3);
}

/// A session that becomes active under a class whose next frame is
/// already chosen (but not in flight) replaces that choice when it would
/// finish first.
#[test]
fn newly_active_session_replaces_its_class_pick() {
    // root -> { C(1) -> {lo(1), hi(1000)}, x(1) }
    let mut q = Control::new();
    let c = q.add_class(q.root(), 1).unwrap();
    let lo = q.add_session(c, 1).unwrap();
    let hi = q.add_session(c, 1000).unwrap();
    let x = q.add_session(q.root(), 1).unwrap();
    q.set_backlog(lo, 1000).unwrap();
    q.set_backlog(x, 1000).unwrap();
    let mut order = Vec::new();
    let mut send = |q: &mut Control| {
        let (s, hint) = q.dequeue(Time::EPOCH).unwrap();
        q.complete(s, hint, Time::EPOCH).unwrap();
        order.push(s);
    };
    send(&mut q); // lo
    // x is next; C has already chosen lo again.
    assert_eq!(q.dequeue(Time::EPOCH).map(|d| d.0), Some(x));
    // hi arrives while x is in flight: C's pick isn't in flight, so hi
    // takes it.
    q.set_backlog(hi, 1000).unwrap();
    send(&mut q); // x
    send(&mut q);
    assert_eq!(order, [lo, x, hi]);

    // The head in flight is never replaced.
    let (mut q, a, b) = pair(1, 1000);
    q.set_backlog(a, 1000).unwrap();
    assert_eq!(q.dequeue(Time::EPOCH).unwrap().0, a);
    q.set_backlog(b, 1000).unwrap();
    assert_eq!(q.dequeue(Time::EPOCH).unwrap().0, a);
    q.complete(a, 1000, Time::EPOCH).unwrap();
    assert_eq!(q.dequeue(Time::EPOCH).unwrap().0, b);
}

/// `poll_timeout` is exact right after `complete`, before the next
/// `dequeue` has chosen the next head: `None` when nothing is left, and the
/// release time of the next head when rate-limited.
#[test]
fn poll_timeout_is_exact_after_complete() {
    let start = Time::from_unix_millis(1_000);
    let mut q = Control::with_rate_limit(10_000);
    let a = q.add_session(q.root(), 1).unwrap();
    let b = q.add_session(q.root(), 1).unwrap();
    q.set_backlog(a, 4000).unwrap();
    q.set_backlog(b, 4000).unwrap();
    let (s, hint) = q.dequeue(start).unwrap();
    assert_eq!(s, a);
    assert_eq!(
        q.poll_timeout(),
        Some(start),
        "in flight: dequeue again now"
    );
    q.complete(s, hint, start).unwrap();
    // 4000 + 4000 < 10000: b goes right away.
    assert_eq!(q.poll_timeout(), Some(start));
    let (s, hint) = q.dequeue(start).unwrap();
    assert_eq!(s, b);
    q.complete(s, hint, start).unwrap();
    // 8000 queued; a needs 8000 + 4000 < 10000, so just over 2000 bytes
    // must drain: 200 ms plus a nanosecond.
    let due = q.poll_timeout().unwrap();
    assert_eq!(
        due,
        start + Duration::from_millis(200) + Duration::NANOSECOND
    );
    assert_eq!(q.dequeue(due - Duration::NANOSECOND), None);
    assert_eq!(q.poll_timeout(), Some(due));
    let (s, hint) = q.dequeue(due).unwrap();
    assert_eq!(s, a);
    q.clear_backlog(a).unwrap();
    q.clear_backlog(b).unwrap();
    q.complete(s, hint, due).unwrap();
    assert_eq!(q.poll_timeout(), None, "nothing left to send");
    assert_eq!(q.dequeue(due + Duration::HOUR), None);
}

/// A session joining two long-running ones gets its share from its first
/// frames: virtual time never runs ahead of backlogged sessions.
#[test]
fn newcomer_gets_its_share_immediately() {
    let mut q = Control::new();
    let a = q.add_session(q.root(), 1).unwrap();
    let b = q.add_session(q.root(), 1).unwrap();
    let c = q.add_session(q.root(), 1).unwrap();
    q.set_backlog(a, 1000).unwrap();
    q.set_backlog(b, 1000).unwrap();
    run_pair(&mut q, a, b, 100_000, false);
    q.set_backlog(c, 1000).unwrap();
    let mut counts = BTreeMap::new();
    for i in 1..=3000 {
        let (s, hint) = q.dequeue(Time::EPOCH).unwrap();
        q.complete(s, hint, Time::EPOCH).unwrap();
        *counts.entry(s).or_insert(0i64) += 1;
        // c is served at its fair rate from the start.
        let nc = counts.get(&c).copied().unwrap_or(0);
        assert!((nc - i / 3).abs() <= 2, "after {i} frames c has {nc}");
    }
}

/// A session idle (or blocked) for a long time rejoins at the current
/// virtual time: no credit banked while away, and no starvation either.
#[test]
fn idle_and_blocked_sessions_do_not_bank_credit() {
    for mode in ["idle", "blocked"] {
        let (mut q, a, b) = pair(1, 1);
        q.set_backlog(a, 1000).unwrap();
        q.set_backlog(b, 1000).unwrap();
        run_pair(&mut q, a, b, 1_000, false);
        match mode {
            "idle" => q.clear_backlog(b).unwrap(),
            _ => q.set_blocked(b, true).unwrap(),
        }
        let (na, nb) = run_pair(&mut q, a, b, 10_000, false);
        assert_eq!((na, nb), (10_000, 0));
        match mode {
            "idle" => q.set_backlog(b, 1000).unwrap(),
            _ => q.set_blocked(b, false).unwrap(),
        }
        let (mut na, mut nb) = (0i64, 0i64);
        for _ in 0..2000 {
            let (na1, nb1) = run_pair(&mut q, a, b, 1, false);
            na += na1 as i64;
            nb += nb1 as i64;
            assert!((na - nb).abs() <= 2, "{mode}: a {na} b {nb}");
        }
    }
}

/// Blocking and unblocking between every frame (a link that keeps filling)
/// can't earn a session more than its share.
#[test]
fn block_toggling_does_not_launder_debt() {
    let (mut q, a, b) = pair(1, 1);
    q.set_backlog(a, 1000).unwrap();
    q.set_backlog(b, 1000).unwrap();
    let (mut na, mut nb) = (0i64, 0i64);
    for _ in 0..10_000 {
        let (s, hint) = q.dequeue(Time::EPOCH).unwrap();
        q.complete(s, hint, Time::EPOCH).unwrap();
        if s == a {
            na += 1;
        } else {
            nb += 1;
        }
        q.set_blocked(b, true).unwrap();
        q.set_blocked(b, false).unwrap();
        assert!(nb - na <= 2, "a {na} b {nb}");
    }
    assert!((na - nb).abs() <= 2, "a {na} b {nb}");
}

/// Drives the clock with `poll_timeout` and checks that every window of
/// length W starting at a send carries at most limit * (W + 1s) bytes plus
/// one frame (the bucket holds one second), and that the long-run rate
/// matches the limit.
#[test]
fn rate_limit_holds_over_windows() {
    const LIMIT: u64 = 64_000;
    let mut rng = det_rng(5);
    let mut tree = Tree::new(Control::with_rate_limit(LIMIT), HIERARCHY);
    for i in tree.sessions() {
        tree.q.set_backlog(tree.session(i), MTU).unwrap();
    }
    let start = Time::from_unix_millis(1_700_000_000_000);
    let end = start + Duration::from_secs(30);
    let mut now = start;
    let mut log: Vec<(Time, u64)> = Vec::new();
    while now < end {
        let before = tree.bytes[0];
        while tree
            .step(now, &mut |_, _| {
                200 + (rng.next_u32() as usize % (MTU - 199))
            })
            .is_some()
        {}
        if tree.bytes[0] > before {
            log.push((now, tree.bytes[0] - before));
        }
        let wake = tree.q.poll_timeout().expect("always backlogged");
        assert!(wake > now, "dequeue returned None before the release time");
        now = wake;
    }
    for window in [
        Duration::from_millis(100),
        Duration::SECOND,
        Duration::from_secs(5),
    ] {
        for (i, &(t0, _)) in log.iter().enumerate() {
            let sent: u64 = log[i..]
                .iter()
                .take_while(|(t, _)| *t < t0 + window)
                .map(|(_, n)| n)
                .sum();
            let bound = LIMIT as f64 * (window.as_secs_f64() + 1.0) + MTU as f64;
            assert!(sent as f64 <= bound, "{sent} bytes in {window} from {t0}");
        }
    }
    // After the initial one-second burst, sends are paced: one frame each.
    assert!(
        log.iter()
            .skip(log.len() / 2)
            .all(|(_, n)| *n <= MTU as u64)
    );
    let total = tree.bytes[0] as f64;
    let secs = (end - start).as_secs_f64();
    assert!(total <= LIMIT as f64 * (secs + 1.0) + MTU as f64, "{total}");
    assert!(total >= LIMIT as f64 * secs - MTU as f64, "{total}");
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x9f5),
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// Random hierarchies (up to 4 levels), random weights in [1, 1500]
    /// and random fixed frame sizes: every node stays within 2 MTUs of its
    /// fluid share of its parent at every prefix of 4000 frames.
    #[test]
    fn random_hierarchy_shares(
        classes in proptest::collection::vec((any::<prop::sample::Index>(), 1u64..1500), 0..5),
        sessions in proptest::collection::vec(
            (any::<prop::sample::Index>(), 1u64..1500, 64usize..=MTU), 2..10),
    ) {
        let mut specs = vec![(None, 0, false)];
        for (p, w) in &classes {
            specs.push((Some(p.index(specs.len())), *w, false));
        }
        let n_classes = specs.len();
        let mut lens = Vec::new();
        for (p, w, len) in &sessions {
            specs.push((Some(p.index(n_classes)), *w, true));
            lens.push(*len);
        }
        let mut tree = Tree::new(Control::new(), &specs);
        for (i, len) in tree.sessions().into_iter().zip(&lens) {
            tree.q.set_backlog(tree.session(i), *len).unwrap();
        }
        let mut worst: f64 = 0.0;
        for _ in 0..4000 {
            tree.step(Time::EPOCH, &mut |_, hint| hint).unwrap();
            worst = worst.max(tree.max_deviation());
        }
        prop_assert!(worst <= 2.0 * MTU as f64, "max deviation {} bytes", worst);
    }
}

/// A whole class that goes idle rejoins at its parent's current virtual
/// time: its sessions get the class's share straight away, no more.
#[test]
fn idle_class_does_not_bank_credit() {
    let mut tree = Tree::new(
        Control::new(),
        &[
            (None, 0, false),
            (Some(0), 1, false), // 1: A
            (Some(1), 1, true),  // 2
            (Some(1), 1, true),  // 3
            (Some(0), 1, false), // 4: B
            (Some(4), 1, true),  // 5
        ],
    );
    for i in [2, 3, 5] {
        tree.q.set_backlog(tree.session(i), 1000).unwrap();
    }
    for _ in 0..1000 {
        tree.step(Time::EPOCH, &mut |_, hint| hint).unwrap();
    }
    tree.q.clear_backlog(tree.session(2)).unwrap();
    tree.q.clear_backlog(tree.session(3)).unwrap();
    for _ in 0..10_000 {
        assert_eq!(tree.step(Time::EPOCH, &mut |_, hint| hint), Some(5));
    }
    tree.q.set_backlog(tree.session(2), 1000).unwrap();
    tree.q.set_backlog(tree.session(3), 1000).unwrap();
    let (a0, b0) = (tree.frames[1], tree.frames[4]);
    for _ in 0..2000 {
        tree.step(Time::EPOCH, &mut |_, hint| hint).unwrap();
        let (a, b) = ((tree.frames[1] - a0) as i64, (tree.frames[4] - b0) as i64);
        assert!((a - b).abs() <= 2, "A {a} B {b}");
    }
}

/// A random workload exercising every call; returns the dequeue trace.
/// Checks that `poll_timeout` predicts every `dequeue`.
fn random_workload(seed: u64) -> Vec<(i64, usize, usize)> {
    let mut rng: DetRng = det_rng(seed);
    let mut q = Control::with_rate_limit(200_000);
    let mut classes: Vec<ClassId> = vec![q.root()];
    let mut sessions: Vec<SessionId> = Vec::new();
    let mut now = Time::from_unix_millis(1_000);
    let mut trace = Vec::new();
    let mut r = |n: u32| rng.next_u32() % n;
    for _ in 0..20_000 {
        match r(16) {
            0 if classes.len() < 12 => {
                let p = classes[r(classes.len() as u32) as usize];
                classes.push(q.add_class(p, 1 + u64::from(r(1200))).unwrap());
            }
            1 | 2 if sessions.len() < 40 => {
                let p = classes[r(classes.len() as u32) as usize];
                sessions.push(q.add_session(p, 1 + u64::from(r(1200))).unwrap());
            }
            3 if !sessions.is_empty() => {
                let i = r(sessions.len() as u32) as usize;
                q.remove(sessions.swap_remove(i)).unwrap();
            }
            4 if classes.len() > 1 => {
                let i = 1 + r(classes.len() as u32 - 1) as usize;
                q.remove(classes.swap_remove(i)).unwrap();
                classes.retain(|c| q.contains(*c));
                sessions.retain(|s| q.contains(*s));
            }
            5 if !sessions.is_empty() => {
                let s = sessions[r(sessions.len() as u32) as usize];
                q.set_weight(s, u64::from(r(1200))).unwrap();
            }
            6..=8 if !sessions.is_empty() => {
                let s = sessions[r(sessions.len() as u32) as usize];
                q.set_backlog(s, 1 + r(MTU as u32) as usize).unwrap();
            }
            9 if !sessions.is_empty() => {
                let s = sessions[r(sessions.len() as u32) as usize];
                q.clear_backlog(s).unwrap();
            }
            10 if !sessions.is_empty() => {
                let s = sessions[r(sessions.len() as u32) as usize];
                q.set_blocked(s, r(2) == 0).unwrap();
            }
            11 => now += Duration::from_micros(i64::from(r(20_000))),
            _ => {
                // `poll_timeout` is exact: dequeue succeeds iff it is due.
                let due = q.poll_timeout();
                let idle = q.in_flight().is_none();
                let got = q.dequeue(now);
                if idle {
                    assert_eq!(got.is_some(), due.is_some_and(|t| t <= now));
                }
                if let Some((s, hint)) = got {
                    let i = sessions.iter().position(|x| *x == s).unwrap();
                    let len = r(hint as u32 + 1) as usize;
                    q.complete(s, len, now).unwrap();
                    trace.push((now.unix_nanos(), i, len));
                } else if let Some(t) = q.poll_timeout() {
                    now = now.max(t);
                }
            }
        }
    }
    trace
}

#[test]
fn same_inputs_same_dequeue_sequence() {
    let a = random_workload(42);
    let b = random_workload(42);
    assert!(a.len() > 1000, "workload too idle: {}", a.len());
    assert_eq!(a, b);
    assert_ne!(a, random_workload(43));
}
