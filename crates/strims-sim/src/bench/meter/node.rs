// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The meter bench's nodes on any [`BenchNet`]: the node under test, its
//! neighbours, and a relay's feeder.
//!
//! Node 0 is the node under test, nodes `1..=n` its neighbours (link `i`
//! is node `i + 1`'s), and with [`Role::Relay`] node `n + 1` the feeder that
//! pushes it a copy of the stream (its link `n`).
//!
//! A link writes one write at a time, as vnic's links do: every control
//! record waiting (STATEs, HAVEs, requests, NOT_YETs, pings, echoes) in
//! one write, or else the next push, or else the next answer. Probes follow
//! vnic's rule ([`Prober`]) and ride the write ahead of its records.

// The bench's nodes share their state between their tasks on one thread.
#![allow(clippy::disallowed_types)]

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use futures_util::future::join_all;
use strims_base::time::{Duration, Time};
use strims_node::CaptureSample;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Notify;

use super::meters::{Budget, Class, Ctx, Meter, Obs, Prober, Report, UnitReport};
use super::plan::{
    Byz, Condition, HAVE_BYTES, NOT_YET_BYTES, PING_BYTES, PING_EVERY, REQUEST_BYTES, Role,
    STATE_BYTES, STATE_EVERY, Traffic, Transport,
};
use crate::bench::{BenchClock, BenchNet, ChannelProbe, LINGER, NodePlan, Schedule, links};

/// A record's header: its kind, length, unit, frame, aux (a probe's
/// nonce) and due time (µs).
pub const HEADER: usize = 1 + 4 + 4 + 2 + 4 + 8;
/// A probe or an echo on the wire.
pub const PROBE_BYTES: usize = HEADER;
/// How often the node under test reads its meters.
pub const TICK: Duration = Duration::from_millis(100);
/// How often it samples its links' unsent bytes.
pub const UNSENT_EVERY: Duration = Duration::from_millis(10);
/// The span of units a copy's rate is measured over.
pub const RATE_SPAN: Duration = Duration::from_secs(10);

/// The bench's clock origin as a [`Time`]: meters read bench time from it.
pub fn time_of(us: i64) -> Time {
    Time::from_unix_millis(1_700_000_000_000) + Duration::from_micros(us)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Push,
    Answer,
    Probe,
    Echo,
    State,
    Have,
    Request,
    NotYet,
    Ping,
    Pad,
}

impl Kind {
    const ALL: [Kind; 10] = [
        Kind::Push,
        Kind::Answer,
        Kind::Probe,
        Kind::Echo,
        Kind::State,
        Kind::Have,
        Kind::Request,
        Kind::NotYet,
        Kind::Ping,
        Kind::Pad,
    ];

    fn code(self) -> u8 {
        self as u8
    }

    fn of(code: u8) -> Option<Kind> {
        Kind::ALL.get(code as usize).copied()
    }

    fn class(self) -> Class {
        match self {
            Kind::Push => Class::Push,
            Kind::Answer => Class::Answer,
            Kind::Probe | Kind::Echo => Class::Probe,
            Kind::Pad => Class::Pad,
            _ => Class::Control,
        }
    }

    fn control(self) -> bool {
        !matches!(self, Kind::Push | Kind::Answer | Kind::Pad)
    }
}

/// A record waiting to be written.
#[derive(Clone, Debug)]
pub struct Rec {
    pub kind: Kind,
    pub len: usize,
    pub unit: u32,
    pub frame: u16,
    pub aux: u32,
    pub due: i64,
    pub body: Vec<u8>,
}

impl Rec {
    fn new(kind: Kind, len: u64, due: i64) -> Rec {
        Rec {
            kind,
            len: (len as usize).max(HEADER),
            unit: 0,
            frame: 0,
            aux: 0,
            due,
            body: Vec::new(),
        }
    }

    fn unit(mut self, unit: u32, frame: u16) -> Rec {
        self.unit = unit;
        self.frame = frame;
        self
    }

    fn encode(&self, buf: &mut Vec<u8>) {
        let len = self.len.max(HEADER + self.body.len());
        let start = buf.len();
        buf.push(self.kind.code());
        buf.extend_from_slice(&u32::try_from(len).unwrap_or(u32::MAX).to_be_bytes());
        buf.extend_from_slice(&self.unit.to_be_bytes());
        buf.extend_from_slice(&self.frame.to_be_bytes());
        buf.extend_from_slice(&self.aux.to_be_bytes());
        buf.extend_from_slice(&self.due.to_be_bytes());
        buf.extend_from_slice(&self.body);
        buf.resize(start + len, 0);
    }

    fn wire_len(&self) -> u64 {
        self.len.max(HEADER + self.body.len()) as u64
    }
}

/// A record as read.
#[derive(Clone, Copy, Debug)]
pub struct Head {
    pub kind: Kind,
    pub len: usize,
    pub unit: u32,
    pub frame: u16,
    pub aux: u32,
    pub due: i64,
}

fn parse_head(h: &[u8]) -> Option<Head> {
    let u32_at = |i: usize| Some(u32::from_be_bytes(h.get(i..i + 4)?.try_into().ok()?));
    Some(Head {
        kind: Kind::of(*h.first()?)?,
        len: u32_at(1)? as usize,
        unit: u32_at(5)?,
        frame: u16::from_be_bytes(h.get(9..11)?.try_into().ok()?),
        aux: u32_at(11)?,
        due: i64::from_be_bytes(h.get(15..23)?.try_into().ok()?),
    })
}

/// A link's records waiting: control ahead of pushes ahead of answers.
#[derive(Default)]
struct Outbox {
    control: VecDeque<Rec>,
    push: VecDeque<Rec>,
    answer: VecDeque<Rec>,
}

/// One end of a link.
pub struct Link {
    ix: usize,
    out: RefCell<Outbox>,
    ready: Notify,
}

impl Link {
    fn new(ix: usize) -> Rc<Link> {
        Rc::new(Link {
            ix,
            out: RefCell::new(Outbox::default()),
            ready: Notify::new(),
        })
    }

    fn push(&self, r: Rec) {
        {
            let mut b = self.out.borrow_mut();
            if r.kind.control() {
                b.control.push_back(r);
            } else if r.kind == Kind::Answer {
                b.answer.push_back(r);
            } else {
                b.push.push_back(r);
            }
        }
        self.ready.notify_one();
    }

    /// The next write's records.
    fn pop(&self) -> Vec<Rec> {
        let mut b = self.out.borrow_mut();
        if !b.control.is_empty() {
            b.control.drain(..).collect()
        } else if let Some(r) = b.push.pop_front() {
            vec![r]
        } else {
            b.answer.pop_front().into_iter().collect()
        }
    }

    /// Drops the data waiting; returns its bytes.
    fn drop_data(&self) -> u64 {
        let mut b = self.out.borrow_mut();
        let n = b.push.iter().chain(&b.answer).map(Rec::wire_len).sum();
        b.push.clear();
        b.answer.clear();
        n
    }
}

/// What a node does with its links' events.
trait Side {
    /// The writer of link `ix` took `recs` for one write.
    fn take(&self, ix: usize, recs: &mut Vec<Rec>, now: i64);
    /// The transport took the write.
    fn wrote(&self, ix: usize, bytes: u64, began: i64, done: i64);
    /// A whole record was read: when the read that brought its first byte
    /// returned, and the one that completed it.
    fn read(&self, ix: usize, head: Head, body: &[u8], first: i64, got: i64);
    /// The link's unsent bytes.
    fn unsent(&self, _ix: usize, _bytes: u64, _now: i64) {}
}

type Task = Pin<Box<dyn Future<Output = ()>>>;

async fn sleep_until<C: BenchClock>(clock: &C, us: i64) {
    let d = us - clock.now_us();
    if d > 0 {
        tokio::time::sleep(StdDuration::from_micros(d.unsigned_abs())).await;
    }
}

fn us(d: Duration) -> i64 {
    d.as_micros()
}

async fn writer<W: AsyncWrite + Unpin, C: BenchClock>(
    mut w: W,
    link: Rc<Link>,
    side: Rc<dyn Side>,
    clock: C,
) {
    let mut buf = Vec::new();
    loop {
        let mut recs = loop {
            let r = link.pop();
            if !r.is_empty() {
                break r;
            }
            link.ready.notified().await;
        };
        let began = clock.now_us();
        side.take(link.ix, &mut recs, began);
        buf.clear();
        for r in &recs {
            r.encode(&mut buf);
        }
        if w.write_all(&buf).await.is_err() {
            return;
        }
        side.wrote(link.ix, buf.len() as u64, began, clock.now_us());
    }
}

async fn reader<R: AsyncRead + Unpin, C: BenchClock>(
    mut r: R,
    link: Rc<Link>,
    side: Rc<dyn Side>,
    clock: C,
) {
    let mut buf = vec![0u8; 1 << 16];
    let mut rec: Vec<u8> = Vec::with_capacity(1 << 14);
    let mut want = HEADER;
    let mut first = 0i64;
    loop {
        let n = match r.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        let got = clock.now_us();
        let mut bytes = buf.get(..n).unwrap_or_default();
        while !bytes.is_empty() {
            if rec.is_empty() {
                first = got;
            }
            let take = (want - rec.len()).min(bytes.len());
            let (h, rest) = bytes.split_at(take);
            rec.extend_from_slice(h);
            bytes = rest;
            if rec.len() < want {
                break;
            }
            let Some(head) = parse_head(&rec) else {
                return;
            };
            if rec.len() == HEADER && head.len > HEADER {
                want = head.len;
                continue;
            }
            side.read(
                link.ix,
                head,
                rec.get(HEADER..).unwrap_or_default(),
                first,
                got,
            );
            rec.clear();
            want = HEADER;
        }
    }
}

async fn tcp_sampler<C: BenchClock>(
    mut probe: Box<dyn strims_node::LinkProbe>,
    ix: usize,
    side: Rc<dyn Side>,
    clock: C,
) {
    loop {
        let unsent = match probe.sample(clock.time()) {
            Some(CaptureSample::Tcp { unsent, .. }) => Some(unsent),
            Some(CaptureSample::SimConn { unsent, .. }) => Some(unsent),
            _ => None,
        };
        if let Some(u) = unsent {
            side.unsent(ix, u, clock.now_us());
        }
        tokio::time::sleep(StdDuration::from_millis(UNSENT_EVERY.as_millis() as u64)).await;
    }
}

async fn channel_sampler<C: BenchClock>(
    probe: ChannelProbe,
    ix: usize,
    side: Rc<dyn Side>,
    clock: C,
) {
    loop {
        let (buffered, _) = probe.sample(clock.time());
        side.unsent(ix, buffered, clock.now_us());
        tokio::time::sleep(StdDuration::from_millis(UNSENT_EVERY.as_millis() as u64)).await;
    }
}

/// Which node a meter bench node is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    Nut,
    Neighbour(usize),
    Feeder,
}

impl Who {
    pub fn of(node: usize, cond: &Condition) -> Who {
        let n = cond.neighbours.len();
        match node {
            0 => Who::Nut,
            i if i <= n => Who::Neighbour(i - 1),
            _ => Who::Feeder,
        }
    }
}

/// The bench's links: the node under test to each neighbour, and to the
/// feeder.
pub fn bench_links(cond: &Condition) -> Vec<(usize, usize)> {
    let n = cond.neighbours.len();
    let mut l: Vec<(usize, usize)> = (1..=n).map(|i| (0, i)).collect();
    if cond.role == Role::Relay {
        l.push((0, n + 1));
    }
    l
}

/// How many nodes the bench runs.
pub fn bench_nodes(cond: &Condition) -> usize {
    cond.neighbours.len() + 1 + usize::from(cond.role == Role::Relay)
}

/// What a node runs: the plan's addresses and the bench's.
pub struct NodeSetup {
    pub me: usize,
    pub listen: String,
    pub addr: Rc<dyn Fn(usize) -> String>,
    pub cond: Rc<Condition>,
    pub traffic: Rc<Traffic>,
    /// The node under test's meters, the driving one first.
    pub meters: Vec<String>,
    /// Logs every observation, for replays.
    pub record: bool,
    pub end_us: i64,
}

/// Runs one node of the bench until [`LINGER`] past its end.
pub async fn node<N: BenchNet, C: BenchClock>(
    net: N,
    clock: C,
    setup: NodeSetup,
    log: Rc<dyn Fn(String)>,
) -> io::Result<()> {
    let cond = setup.cond.clone();
    let me = setup.me;
    let plan = NodePlan::new(
        me,
        &bench_links(&cond),
        setup.listen.clone(),
        |i| (setup.addr)(i),
        Schedule::default(),
        setup.end_us,
    );
    let streams = links(&net, &clock, &plan).await?;
    let who = Who::of(me, &cond);
    let n = cond.neighbours.len();
    let ix_of = |peer: usize| match who {
        Who::Nut => peer - 1,
        _ => 0,
    };
    let links_n = match who {
        Who::Nut => n + usize::from(cond.role == Role::Relay),
        _ => 1,
    };
    let links: Vec<Rc<Link>> = (0..links_n).map(Link::new).collect();
    let side: Rc<dyn Side>;
    let mut tasks: Vec<Task> = Vec::new();
    match who {
        Who::Nut => {
            let nut = Nut::new(&setup, links.clone(), log.clone())?;
            tasks.extend(nut.clone().tasks(clock.clone()));
            side = nut;
        }
        Who::Neighbour(i) => {
            let rx = Rx::new(&setup, i, links.clone(), log.clone());
            tasks.extend(rx.clone().tasks(clock.clone()));
            side = rx;
        }
        Who::Feeder => {
            let f = Feeder::new(&setup, links.clone());
            tasks.extend(f.clone().tasks(clock.clone()));
            side = f;
        }
    }
    let profile = cond.transport.profile();
    for (peer, s) in streams {
        let ix = ix_of(peer);
        let Some(link) = links.get(ix).cloned() else {
            continue;
        };
        match (cond.transport, profile) {
            (Transport::Channel(_), Some(p)) => {
                let cap = strims_node::WebRtcConfig::default().max_buffered_amount;
                let probe = net
                    .probed_channel(&s, cap, p)
                    .ok_or_else(|| io::Error::other("this net models no data channels"))?;
                let (r, w) = tokio::io::split(s);
                tasks.push(Box::pin(writer(
                    w,
                    link.clone(),
                    side.clone(),
                    clock.clone(),
                )));
                tasks.push(Box::pin(reader(
                    r,
                    link.clone(),
                    side.clone(),
                    clock.clone(),
                )));
                if who == Who::Nut {
                    tasks.push(Box::pin(channel_sampler(
                        probe,
                        ix,
                        side.clone(),
                        clock.clone(),
                    )));
                }
            }
            _ => {
                let probe = net.link_probe(&s, false);
                let (r, w) = tokio::io::split(s);
                tasks.push(Box::pin(writer(
                    w,
                    link.clone(),
                    side.clone(),
                    clock.clone(),
                )));
                tasks.push(Box::pin(reader(
                    r,
                    link.clone(),
                    side.clone(),
                    clock.clone(),
                )));
                if let Some(p) = probe.filter(|_| who == Who::Nut) {
                    tasks.push(Box::pin(tcp_sampler(p, ix, side.clone(), clock.clone())));
                }
            }
        }
    }
    let stop = setup.end_us + i64::try_from(LINGER.as_micros()).unwrap_or(i64::MAX);
    tokio::select! {
        biased;
        _ = join_all(tasks) => {}
        () = sleep_until(&clock, stop) => {}
    }
    Ok(())
}

/// Schedules `f` every `every` from `start` until `end` (µs).
async fn every<C: BenchClock>(
    clock: C,
    start: i64,
    every: Duration,
    end: i64,
    mut f: impl FnMut(i64),
) {
    let mut t = start;
    while t < end {
        sleep_until(&clock, t).await;
        f(clock.now_us());
        t += us(every);
    }
}

fn present_changes(cond: &Condition, i: usize) -> Vec<(i64, bool)> {
    let mut v = Vec::new();
    if let Some(nb) = cond.neighbours.get(i) {
        for &(a, b) in &nb.present {
            v.push((us(a), true));
            v.push((us(b), false));
        }
    }
    v
}

// The node under test.

struct NutState {
    meters: Vec<Box<dyn Meter>>,
    prober: Prober,
    budget: Budget,
    copies: usize,
    /// Copies the driving meter adds for a probe.
    extra: usize,
    children: Vec<usize>,
    present: Vec<bool>,
    /// Units' bytes as they were stored, for a copy's rate.
    stored: VecDeque<(i64, u64)>,
    /// A relay's frames received, by unit.
    held: BTreeMap<u32, u64>,
    /// Bytes taken by class since the last log line.
    taken: [u64; 5],
    last_bytes_log: i64,
}

struct Nut {
    cond: Rc<Condition>,
    traffic: Rc<Traffic>,
    links: Vec<Rc<Link>>,
    log: Rc<dyn Fn(String)>,
    record: bool,
    end_us: i64,
    names: Vec<String>,
    st: RefCell<NutState>,
}

fn class_ix(c: Class) -> usize {
    match c {
        Class::Push => 0,
        Class::Answer => 1,
        Class::Control => 2,
        Class::Probe => 3,
        Class::Pad => 4,
    }
}

impl Nut {
    fn new(
        setup: &NodeSetup,
        links: Vec<Rc<Link>>,
        log: Rc<dyn Fn(String)>,
    ) -> io::Result<Rc<Nut>> {
        let cond = setup.cond.clone();
        let n = cond.neighbours.len();
        let mut meters = Vec::new();
        for name in &setup.meters {
            meters.push(
                super::meters::build(name, &cond, links.len(), time_of(0))
                    .map_err(io::Error::other)?,
            );
        }
        let names = meters.iter().map(|m| m.name().to_owned()).collect();
        Ok(Rc::new(Nut {
            traffic: setup.traffic.clone(),
            log,
            record: setup.record,
            end_us: setup.end_us,
            names,
            st: RefCell::new(NutState {
                meters,
                prober: Prober::new(links.len()),
                budget: Budget::default(),
                copies: 1,
                extra: 0,
                children: Vec::new(),
                present: (0..links.len())
                    .map(|i| {
                        i >= n
                            || cond
                                .neighbours
                                .get(i)
                                .is_some_and(|nb| nb.present_at(Duration::ZERO))
                    })
                    .collect(),
                stored: VecDeque::new(),
                held: BTreeMap::new(),
                taken: [0; 5],
                last_bytes_log: 0,
            }),
            links,
            cond,
        }))
    }

    fn observe(&self, st: &mut NutState, now: i64, obs: Obs<'_>) {
        let t = time_of(now);
        for m in &mut st.meters {
            m.observe(t, &obs);
        }
        if self.record {
            (self.log)(obs_line(now, &obs));
        }
    }

    /// The neighbours ranked for copies: nearest first.
    fn ranked(&self, st: &NutState) -> Vec<usize> {
        let n = self.cond.neighbours.len();
        let mut r: Vec<usize> = (0..n)
            .filter(|&i| st.present.get(i) == Some(&true))
            .collect();
        r.sort_by_key(|&i| {
            (
                self.cond
                    .neighbours
                    .get(i)
                    .map_or(Duration::ZERO, |nb| nb.delay),
                i,
            )
        });
        r
    }

    fn choose_children(&self, st: &mut NutState) {
        let ranked = self.ranked(st);
        st.children = ranked.into_iter().take(st.copies + st.extra).collect();
    }

    fn queue(&self, st: &mut NutState, ix: usize, rec: Rec, now: i64) {
        let Some(link) = self.links.get(ix) else {
            return;
        };
        let obs = Obs::Queued {
            link: ix,
            bytes: rec.wire_len(),
            class: rec.kind.class(),
            unit: (!rec.kind.control()).then_some(rec.unit),
        };
        self.observe(st, now, obs);
        link.push(rec);
    }

    fn copy_rate(&self, st: &mut NutState, now: i64) -> f64 {
        while st
            .stored
            .front()
            .is_some_and(|&(t, _)| now - t > us(RATE_SPAN))
        {
            st.stored.pop_front();
        }
        let Some(&(first, _)) = st.stored.front() else {
            return 0.0;
        };
        let span = (now - first).max(us(Duration::SECOND)) as f64 / 1e6;
        st.stored.iter().map(|&(_, b)| b).sum::<u64>() as f64 / span
    }

    /// A seed's unit: pushed to each child, and a HAVE to every neighbour
    /// with main's mix.
    fn unit(&self, u: usize, now: i64) {
        let Some(unit) = self.traffic.units.get(u) else {
            return;
        };
        let mut st = self.st.borrow_mut();
        st.stored.push_back((now, unit.bytes()));
        let children = st.children.clone();
        for &c in &children {
            for (f, &len) in unit.frames.iter().enumerate() {
                let rec =
                    Rec::new(Kind::Push, u64::from(len), us(unit.at)).unit(u as u32, f as u16);
                self.queue(&mut st, c, rec, now);
            }
        }
        if self.cond.have {
            self.haves(&mut st, u as u32, now);
        }
    }

    fn haves(&self, st: &mut NutState, u: u32, now: i64) {
        for i in 0..self.cond.neighbours.len() {
            if st.present.get(i) == Some(&true) {
                self.queue(st, i, Rec::new(Kind::Have, HAVE_BYTES, now).unit(u, 0), now);
            }
        }
    }

    /// A relay's frame from its feeder, forwarded to each child.
    fn forward(&self, head: Head, now: i64) {
        let mut st = self.st.borrow_mut();
        *st.held.entry(head.unit).or_default() |= 1 << head.frame.min(63);
        st.stored.push_back((now, head.len as u64));
        let children = st.children.clone();
        for &c in &children {
            let rec = Rec::new(Kind::Push, head.len as u64, head.due).unit(head.unit, head.frame);
            self.queue(&mut st, c, rec, now);
        }
        let last = self
            .traffic
            .units
            .get(head.unit as usize)
            .is_some_and(|u| u.frames.len() == usize::from(head.frame) + 1);
        if last && self.cond.have {
            self.haves(&mut st, head.unit, now);
        }
    }

    fn request(&self, ix: usize, head: Head, now: i64) {
        let mut st = self.st.borrow_mut();
        if st.present.get(ix) != Some(&true) || st.children.contains(&ix) {
            return;
        }
        let Some(unit) = self.traffic.units.get(head.unit as usize) else {
            return;
        };
        let held = match self.cond.role {
            Role::Seed => now >= us(unit.at),
            Role::Relay => st
                .held
                .get(&head.unit)
                .is_some_and(|m| m & (1 << head.frame.min(63)) != 0),
        };
        let len = unit
            .frames
            .get(usize::from(head.frame))
            .copied()
            .unwrap_or(0);
        let rec = if held && len > 0 {
            Rec::new(Kind::Answer, u64::from(len), us(unit.at)).unit(head.unit, head.frame)
        } else {
            Rec::new(Kind::NotYet, NOT_YET_BYTES, now).unit(head.unit, head.frame)
        };
        self.queue(&mut st, ix, rec, now);
    }

    fn presence(&self, ix: usize, present: bool, now: i64) {
        let mut st = self.st.borrow_mut();
        if let Some(p) = st.present.get_mut(ix) {
            *p = present;
        }
        self.observe(&mut st, now, Obs::Present { link: ix, present });
        if !present && let Some(link) = self.links.get(ix) {
            let bytes = link.drop_data();
            if bytes > 0 {
                self.observe(&mut st, now, Obs::Dropped { link: ix, bytes });
            }
        }
        self.choose_children(&mut st);
    }

    fn ctx(&self, st: &mut NutState, now: i64) -> Ctx {
        Ctx {
            role: self.cond.role,
            cut: st.budget.cut,
            copies: st.copies,
            copy_rate: self.copy_rate(st, now),
            elapsed: Duration::from_micros(now),
        }
    }

    fn tick(&self, now: i64) {
        let mut st = self.st.borrow_mut();
        let ctx = self.ctx(&mut st, now);
        let t = time_of(now);
        let mut drive = None;
        let st = &mut *st;
        for (i, m) in st.meters.iter_mut().enumerate() {
            let r = m.reading(t, &ctx);
            if i == 0 {
                drive = Some(r.capacity);
            }
            let diag: Vec<String> = r
                .diag
                .iter()
                .map(|(k, v)| format!(r#","{k}":{}"#, num(*v)))
                .collect();
            (self.log)(format!(
                r#"{{"ev":"m","t":{now},"meter":"{}","cap":{}{}}}"#,
                self.names.get(i).map_or("", String::as_str),
                r.capacity.map_or("null".to_owned(), num),
                diag.concat(),
            ));
        }
        if st.budget.due(t) {
            let copies = match &self.cond.copies {
                Some(sched) => sched
                    .iter()
                    .rev()
                    .find(|&&(at, _)| us(at) <= now)
                    .map_or(1, |&(_, k)| k),
                None => st.budget.check(t, drive.flatten(), ctx.copy_rate),
            };
            if self.cond.copies.is_some() {
                let _ = st.budget.check(t, None, ctx.copy_rate);
            }
            st.copies = copies;
            self.choose_children(st);
            (self.log)(format!(
                r#"{{"ev":"k","t":{now},"copies":{},"budget":{},"children":{},"present":{},"extra":{},"rate":{}}}"#,
                st.copies,
                st.budget.budget,
                st.children.len(),
                self.ranked(st).len(),
                st.extra,
                num(ctx.copy_rate),
            ));
        }
        let extra = st.meters.first_mut().map_or(0, |m| m.extra_copies(t, &ctx));
        if extra != st.extra {
            st.extra = extra;
            self.choose_children(st);
        }
        let pads = st
            .meters
            .first_mut()
            .map(|m| m.padding(t, &ctx))
            .unwrap_or_default();
        for (ix, bytes) in pads {
            let rec = Rec::new(Kind::Pad, bytes, now);
            self.queue(st, ix, rec, now);
        }
        if now - st.last_bytes_log >= 1_000_000 {
            (self.log)(format!(
                r#"{{"ev":"b","t":{now},"push":{},"answer":{},"control":{},"probe":{},"pad":{}}}"#,
                st.taken[0], st.taken[1], st.taken[2], st.taken[3], st.taken[4],
            ));
            st.taken = [0; 5];
            st.last_bytes_log = now;
        }
    }

    fn tasks<C: BenchClock>(self: Rc<Nut>, clock: C) -> Vec<Task> {
        let mut tasks: Vec<Task> = Vec::new();
        let end = self.end_us;
        let n = self.cond.neighbours.len();
        {
            let me = self.clone();
            let c = clock.clone();
            tasks.push(Box::pin(every(c, 0, TICK, end, move |now| me.tick(now))));
        }
        if self.cond.role == Role::Seed {
            let me = self.clone();
            let c = clock.clone();
            tasks.push(Box::pin(async move {
                for (u, unit) in me.traffic.units.iter().enumerate() {
                    if us(unit.at) >= end {
                        break;
                    }
                    sleep_until(&c, us(unit.at)).await;
                    me.unit(u, c.now_us());
                }
            }));
        }
        for ix in 0..self.links.len() {
            let me = self.clone();
            let c = clock.clone();
            let phase = (ix as i64 * 37_000) % us(STATE_EVERY);
            tasks.push(Box::pin(every(c, phase, STATE_EVERY, end, move |now| {
                let mut st = me.st.borrow_mut();
                if st.present.get(ix) == Some(&true) {
                    me.queue(&mut st, ix, Rec::new(Kind::State, STATE_BYTES, now), now);
                }
            })));
            let me = self.clone();
            let c = clock.clone();
            let phase = (ix as i64 * 71_000) % us(PING_EVERY);
            tasks.push(Box::pin(every(c, phase, PING_EVERY, end, move |now| {
                let mut st = me.st.borrow_mut();
                if st.present.get(ix) == Some(&true) {
                    me.queue(&mut st, ix, Rec::new(Kind::Ping, PING_BYTES, now), now);
                }
            })));
            if ix < n {
                let changes = present_changes(&self.cond, ix);
                let me = self.clone();
                let c = clock.clone();
                tasks.push(Box::pin(async move {
                    for (at, present) in changes {
                        if at <= 0 || at >= end {
                            continue;
                        }
                        sleep_until(&c, at).await;
                        me.presence(ix, present, c.now_us());
                    }
                }));
            }
        }
        tasks
    }
}

fn num(x: f64) -> String {
    if x.is_finite() {
        format!("{x:.1}")
    } else {
        "null".to_owned()
    }
}

fn obs_line(now: i64, obs: &Obs<'_>) -> String {
    match *obs {
        Obs::Queued {
            link,
            bytes,
            class,
            unit,
        } => format!(
            r#"{{"ev":"o","t":{now},"o":"queued","link":{link},"bytes":{bytes},"class":"{}","unit":{}}}"#,
            class.name(),
            unit.map_or(-1, i64::from)
        ),
        Obs::Taken {
            link,
            bytes,
            class,
            unit,
        } => format!(
            r#"{{"ev":"o","t":{now},"o":"taken","link":{link},"bytes":{bytes},"class":"{}","unit":{}}}"#,
            class.name(),
            unit.map_or(-1, i64::from)
        ),
        Obs::Dropped { link, bytes } => {
            format!(r#"{{"ev":"o","t":{now},"o":"dropped","link":{link},"bytes":{bytes}}}"#)
        }
        Obs::Wrote { link, bytes, began } => format!(
            r#"{{"ev":"o","t":{now},"o":"wrote","link":{link},"bytes":{bytes},"began":{}}}"#,
            (began - time_of(0)).as_micros()
        ),
        Obs::Probe { link, nonce } => {
            format!(r#"{{"ev":"o","t":{now},"o":"probe","link":{link},"nonce":{nonce}}}"#)
        }
        Obs::Echo { link, nonce } => {
            format!(r#"{{"ev":"o","t":{now},"o":"echo","link":{link},"nonce":{nonce}}}"#)
        }
        Obs::Report { link, report } => {
            let units: Vec<String> = report
                .units
                .iter()
                .map(|u| {
                    let f: Vec<String> =
                        u.frames.iter().map(|(t, b)| format!("[{t},{b}]")).collect();
                    format!(
                        "[{},{},{},{},[{}]]",
                        u.unit,
                        u.first_us,
                        u.last_us,
                        u.bytes,
                        f.join(",")
                    )
                })
                .collect();
            format!(
                r#"{{"ev":"o","t":{now},"o":"report","link":{link},"bytes":{},"from":{},"to":{},"units":[{}]}}"#,
                report.bytes,
                report.from_us,
                report.to_us,
                units.join(",")
            )
        }
        Obs::Unsent { link, bytes } => {
            format!(r#"{{"ev":"o","t":{now},"o":"unsent","link":{link},"bytes":{bytes}}}"#)
        }
        Obs::Present { link, present } => {
            format!(r#"{{"ev":"o","t":{now},"o":"present","link":{link},"present":{present}}}"#)
        }
    }
}

impl Side for Nut {
    fn take(&self, ix: usize, recs: &mut Vec<Rec>, now: i64) {
        let mut st = self.st.borrow_mut();
        let st = &mut *st;
        let owners = recs.iter().any(|r| r.kind != Kind::Echo);
        if owners && let Some(nonce) = st.prober.due(ix, time_of(now)) {
            let mut p = Rec::new(Kind::Probe, PROBE_BYTES as u64, now);
            p.aux = nonce;
            recs.insert(0, p);
        }
        for r in recs.iter() {
            let class = r.kind.class();
            let bytes = r.wire_len();
            st.prober.wrote(ix, bytes, class != Class::Probe);
            if let Some(slot) = st.taken.get_mut(class_ix(class)) {
                *slot += bytes;
            }
            let unit = (!r.kind.control()).then_some(r.unit);
            self.observe(
                st,
                now,
                Obs::Taken {
                    link: ix,
                    bytes,
                    class,
                    unit,
                },
            );
            if r.kind == Kind::Probe {
                st.prober.probed(ix, r.aux, time_of(now));
                self.observe(
                    st,
                    now,
                    Obs::Probe {
                        link: ix,
                        nonce: r.aux,
                    },
                );
            }
        }
    }

    fn wrote(&self, ix: usize, bytes: u64, began: i64, done: i64) {
        let mut st = self.st.borrow_mut();
        self.observe(
            &mut st,
            done,
            Obs::Wrote {
                link: ix,
                bytes,
                began: time_of(began),
            },
        );
    }

    fn read(&self, ix: usize, head: Head, body: &[u8], _first: i64, got: i64) {
        match head.kind {
            Kind::Echo => {
                let mut st = self.st.borrow_mut();
                st.prober.echo(ix, head.aux, time_of(got));
                self.observe(
                    &mut st,
                    got,
                    Obs::Echo {
                        link: ix,
                        nonce: head.aux,
                    },
                );
            }
            Kind::State => {
                if let Some(report) = decode_report(body) {
                    let mut st = self.st.borrow_mut();
                    self.observe(
                        &mut st,
                        got,
                        Obs::Report {
                            link: ix,
                            report: &report,
                        },
                    );
                }
            }
            Kind::Request => self.request(ix, head, got),
            Kind::Push if self.cond.role == Role::Relay && ix == self.cond.neighbours.len() => {
                self.forward(head, got);
            }
            _ => {}
        }
    }

    fn unsent(&self, ix: usize, bytes: u64, now: i64) {
        let mut st = self.st.borrow_mut();
        self.observe(&mut st, now, Obs::Unsent { link: ix, bytes });
    }
}

fn encode_report(r: &Report) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&(r.units.len() as u16).to_be_bytes());
    for u in &r.units {
        b.extend_from_slice(&u.unit.to_be_bytes());
        b.extend_from_slice(&u.first_us.to_be_bytes());
        b.extend_from_slice(&u.last_us.to_be_bytes());
        b.extend_from_slice(&u.bytes.to_be_bytes());
        b.push(u.frames.len().min(255) as u8);
        for (t, n) in u.frames.iter().take(255) {
            b.extend_from_slice(&t.to_be_bytes());
            b.extend_from_slice(&n.to_be_bytes());
        }
    }
    b.extend_from_slice(&r.bytes.to_be_bytes());
    b.extend_from_slice(&r.from_us.to_be_bytes());
    b.extend_from_slice(&r.to_us.to_be_bytes());
    b
}

fn decode_report(b: &[u8]) -> Option<Report> {
    let mut at = 0usize;
    let mut take = |n: usize| {
        let s = b.get(at..at + n)?;
        at += n;
        Some(s)
    };
    let n = u16::from_be_bytes(take(2)?.try_into().ok()?);
    let mut units = Vec::new();
    for _ in 0..n {
        let unit = u32::from_be_bytes(take(4)?.try_into().ok()?);
        let first_us = i64::from_be_bytes(take(8)?.try_into().ok()?);
        let last_us = i64::from_be_bytes(take(8)?.try_into().ok()?);
        let bytes = u32::from_be_bytes(take(4)?.try_into().ok()?);
        let k = *take(1)?.first()?;
        let mut frames = Vec::new();
        for _ in 0..k {
            let t = i64::from_be_bytes(take(8)?.try_into().ok()?);
            frames.push((t, u32::from_be_bytes(take(4)?.try_into().ok()?)));
        }
        units.push(UnitReport {
            unit,
            first_us,
            last_us,
            bytes,
            frames,
        });
    }
    let bytes = u64::from_be_bytes(take(8)?.try_into().ok()?);
    let from_us = i64::from_be_bytes(take(8)?.try_into().ok()?);
    let to_us = i64::from_be_bytes(take(8)?.try_into().ok()?);
    Some(Report {
        units,
        bytes,
        from_us,
        to_us,
    })
}

// A neighbour.

#[derive(Default)]
struct Arrival {
    first: i64,
    last: i64,
    bytes: u64,
    frames: Vec<(i64, u32)>,
    got: u64,
}

#[derive(Default)]
struct RxState {
    arrivals: BTreeMap<u32, Arrival>,
    report: Vec<UnitReport>,
    bytes: u64,
    last_report: i64,
    /// Echoes held back, by when they go.
    held: VecDeque<(i64, u32)>,
}

struct Rx {
    i: usize,
    cond: Rc<Condition>,
    traffic: Rc<Traffic>,
    link: Rc<Link>,
    log: Rc<dyn Fn(String)>,
    end_us: i64,
    held: Notify,
    st: RefCell<RxState>,
}

impl Rx {
    fn new(setup: &NodeSetup, i: usize, links: Vec<Rc<Link>>, log: Rc<dyn Fn(String)>) -> Rc<Rx> {
        let link = links.into_iter().next().unwrap_or_else(|| Link::new(0));
        Rc::new(Rx {
            i,
            cond: setup.cond.clone(),
            traffic: setup.traffic.clone(),
            link,
            log,
            end_us: setup.end_us,
            held: Notify::new(),
            st: RefCell::new(RxState::default()),
        })
    }

    fn byz(&self) -> Option<Byz> {
        self.cond.neighbours.get(self.i).and_then(|nb| nb.byz)
    }

    fn present(&self, now: i64) -> bool {
        self.cond
            .neighbours
            .get(self.i)
            .is_some_and(|nb| nb.present_at(Duration::from_micros(now)))
    }

    fn offset(&self) -> i64 {
        self.cond
            .neighbours
            .get(self.i)
            .map_or(0, |nb| us(nb.clock_offset))
    }

    fn echo(&self, nonce: u32) {
        let mut b = self.link.out.borrow_mut();
        if let Some(e) = b.control.iter_mut().find(|r| r.kind == Kind::Echo) {
            e.aux = nonce;
            return;
        }
        let mut r = Rec::new(Kind::Echo, PROBE_BYTES as u64, 0);
        r.aux = nonce;
        b.control.push_back(r);
        drop(b);
        self.link.ready.notify_one();
    }

    fn state(&self, now: i64) {
        if !self.present(now) {
            return;
        }
        let mut st = self.st.borrow_mut();
        let mut rec = Rec::new(Kind::State, STATE_BYTES, now);
        if self.byz() != Some(Byz::Silent) {
            let off = self.offset();
            let mut units = std::mem::take(&mut st.report);
            if let Some(Byz::Lie(f)) = self.byz() {
                for u in &mut units {
                    let scale = |t: i64| u.first_us + ((t - u.first_us) as f64 * f) as i64;
                    u.last_us = scale(u.last_us);
                    u.frames = u.frames.iter().map(|&(t, b)| (scale(t), b)).collect();
                }
            }
            for u in &mut units {
                u.first_us += off;
                u.last_us += off;
                for (f, _) in &mut u.frames {
                    *f += off;
                }
            }
            let report = Report {
                units,
                bytes: st.bytes,
                from_us: st.last_report + off,
                to_us: now + off,
            };
            rec.body = encode_report(&report);
        } else {
            st.report.clear();
        }
        st.bytes = 0;
        st.last_report = now;
        drop(st);
        self.link.push(rec);
    }

    fn tasks<C: BenchClock>(self: Rc<Rx>, clock: C) -> Vec<Task> {
        let mut tasks: Vec<Task> = Vec::new();
        let end = self.end_us;
        let phase = (self.i as i64 * 53_000 + 11_000) % us(STATE_EVERY);
        let me = self.clone();
        tasks.push(Box::pin(every(
            clock.clone(),
            phase,
            STATE_EVERY,
            end,
            move |now| me.state(now),
        )));
        let me = self.clone();
        let phase = (self.i as i64 * 97_000 + 29_000) % us(PING_EVERY);
        tasks.push(Box::pin(every(
            clock.clone(),
            phase,
            PING_EVERY,
            end,
            move |now| {
                if me.present(now) {
                    me.link.push(Rec::new(Kind::Ping, PING_BYTES, now));
                }
            },
        )));
        let me = self.clone();
        let c = clock.clone();
        tasks.push(Box::pin(async move {
            let Some(reqs) = me.traffic.requests.get(me.i) else {
                return;
            };
            for r in reqs {
                if us(r.at) >= end {
                    break;
                }
                sleep_until(&c, us(r.at)).await;
                if me.present(c.now_us()) {
                    me.link.push(
                        Rec::new(Kind::Request, REQUEST_BYTES, us(r.at)).unit(r.unit, r.frame),
                    );
                }
            }
        }));
        let me = self;
        let c = clock;
        tasks.push(Box::pin(async move {
            loop {
                let next = me.st.borrow().held.front().copied();
                match next {
                    None => me.held.notified().await,
                    Some((at, nonce)) => {
                        sleep_until(&c, at).await;
                        me.st.borrow_mut().held.pop_front();
                        me.echo(nonce);
                    }
                }
            }
        }));
        tasks
    }
}

impl Side for Rx {
    fn take(&self, _ix: usize, _recs: &mut Vec<Rec>, _now: i64) {}

    fn wrote(&self, _ix: usize, _bytes: u64, _began: i64, _done: i64) {}

    fn read(&self, _ix: usize, head: Head, _body: &[u8], first: i64, got: i64) {
        let mut st = self.st.borrow_mut();
        st.bytes += head.len as u64;
        match head.kind {
            Kind::Probe => match self.byz() {
                Some(Byz::Silent) => {}
                Some(Byz::Delay(d)) => {
                    st.held.push_back((got + us(d), head.aux));
                    drop(st);
                    self.held.notify_one();
                }
                _ => {
                    drop(st);
                    self.echo(head.aux);
                }
            },
            Kind::Push => {
                let frames = self
                    .traffic
                    .units
                    .get(head.unit as usize)
                    .map_or(1, |u| u.frames.len()) as u64;
                let a = st.arrivals.entry(head.unit).or_default();
                if a.got == 0 {
                    a.first = first;
                }
                a.last = got;
                a.bytes += head.len as u64;
                a.frames.push((got, head.len as u32));
                a.got += 1;
                if a.got >= frames {
                    let a = st.arrivals.remove(&head.unit).unwrap_or_default();
                    (self.log)(format!(
                        r#"{{"ev":"u","unit":{},"at":{},"first":{},"last":{},"bytes":{}}}"#,
                        head.unit, head.due, a.first, a.last, a.bytes
                    ));
                    st.report.push(UnitReport {
                        unit: head.unit,
                        first_us: a.first,
                        last_us: a.last,
                        bytes: a.bytes as u32,
                        frames: a.frames,
                    });
                }
            }
            Kind::Answer => {
                (self.log)(format!(
                    r#"{{"ev":"a","unit":{},"frame":{},"at":{},"got":{got}}}"#,
                    head.unit, head.frame, head.due
                ));
            }
            _ => {}
        }
    }
}

// A relay's feeder: pushes it every unit.

struct Feeder {
    traffic: Rc<Traffic>,
    link: Rc<Link>,
    end_us: i64,
}

impl Feeder {
    fn new(setup: &NodeSetup, links: Vec<Rc<Link>>) -> Rc<Feeder> {
        Rc::new(Feeder {
            traffic: setup.traffic.clone(),
            link: links.into_iter().next().unwrap_or_else(|| Link::new(0)),
            end_us: setup.end_us,
        })
    }

    fn tasks<C: BenchClock>(self: Rc<Feeder>, clock: C) -> Vec<Task> {
        let mut tasks: Vec<Task> = Vec::new();
        let end = self.end_us;
        let me = self.clone();
        tasks.push(Box::pin(every(
            clock.clone(),
            7_000,
            STATE_EVERY,
            end,
            move |now| {
                me.link.push(Rec::new(Kind::State, STATE_BYTES, now));
            },
        )));
        let me = self;
        let c = clock;
        tasks.push(Box::pin(async move {
            for (u, unit) in me.traffic.units.iter().enumerate() {
                if us(unit.at) >= end {
                    break;
                }
                sleep_until(&c, us(unit.at)).await;
                for (f, &len) in unit.frames.iter().enumerate() {
                    me.link.push(
                        Rec::new(Kind::Push, u64::from(len), us(unit.at)).unit(u as u32, f as u16),
                    );
                }
            }
        }));
        tasks
    }
}

impl Side for Feeder {
    fn take(&self, _ix: usize, _recs: &mut Vec<Rec>, _now: i64) {}

    fn wrote(&self, _ix: usize, _bytes: u64, _began: i64, _done: i64) {}

    fn read(&self, _ix: usize, head: Head, _body: &[u8], _first: i64, _got: i64) {
        if head.kind == Kind::Probe {
            let mut b = self.link.out.borrow_mut();
            if let Some(e) = b.control.iter_mut().find(|r| r.kind == Kind::Echo) {
                e.aux = head.aux;
                return;
            }
            let mut r = Rec::new(Kind::Echo, PROBE_BYTES as u64, 0);
            r.aux = head.aux;
            b.control.push_back(r);
            drop(b);
            self.link.ready.notify_one();
        }
    }
}
