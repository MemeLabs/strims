// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A node's link log ([`LinkLog`]): what its links' I/O did
//! (`NodeConfig::link_log`), made into the rows of the tables'
//! `link_writes`, `link_reads` and `link_samples`. A testbed node takes
//! the records it had kept at each line's moment ([`LinkLog::take_raw`]),
//! which its line's writer makes rows ([`LinkRowsMaker`]) and writes in the
//! line's records ([`link_log_json`], [`put_link_log`]), off the node's
//! thread; strims-sim's `testbed` module reads them back ([`link_log_of`]). strims-sim takes a simulated node's
//! at its stats samples ([`LinkLog::take`]).
//!
//! The node's code hands each [`LinkRecord`] to the log's sink as it runs,
//! which stamps it with the log's clock and keeps it in a buffer of room
//! for [`LinkLog::CAP`] I/O records and [`LinkLog::SAMPLE_CAP`] samples
//! between two takes, past which they are counted as lost (a lost record
//! shifts the rows of its link after it): the sink never grows it. A take
//! swaps the buffers for new ones as large, and turns the records into
//! rows:
//!
//! - **writes**: each write a link's writer began and its transport took
//!   whole, matched with the node's hand-off of it (the writes of a link go
//!   in the order the node queued them), at its byte offset and ordinal on
//!   the link, with the transport's buffered bytes at its last check and
//!   what held it last;
//! - **reads**: each read the reader returned (a data channel's message as
//!   it arrived, where its transport delivers messages), at its offset and
//!   ordinal, once the node took its last byte, or the link went;
//! - **samples**: the node's readings of a link's transport and queues
//!   every capture interval, and the transport's buffered bytes at each
//!   check of a write that held it (a check that let the write go is the
//!   write's `unsent_before`) and as it woke a held writer.
//!
//! A link's rows wait for vnic to name its peer, and a link that goes
//! unnamed has none. Its ordinal counts the node's links to that peer in
//! the order they were named, across the node's restarts: a link opened
//! under a name an earlier incarnation's link had ends that one first.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::wire::{Wire, put_name, take_name};
use super::{list, put_opt};
use strims_base::time::{Duration, Time};
use strims_node_core::{
    HostId, LinkEvent, LinkKind, LinkRecord, LinkSink, LinkState, SampleSource, WriteWait,
};

/// Why a link sample was read ([`LinkSample::cause`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SampleCause {
    /// The node's reading every capture interval.
    Sample,
    /// The writer's reading as it decided a write ([`LinkEvent::Checked`]).
    Write,
    /// The transport's wake of a held writer ([`LinkEvent::Low`]).
    Low,
}

impl SampleCause {
    /// Its name in the tables: `sample`, `write`, `low`.
    pub fn name(self) -> &'static str {
        match self {
            SampleCause::Sample => "sample",
            SampleCause::Write => "write",
            SampleCause::Low => "low",
        }
    }

    /// The cause named `name` ([`SampleCause::name`]).
    pub fn parse(name: &str) -> Option<SampleCause> {
        [SampleCause::Sample, SampleCause::Write, SampleCause::Low]
            .into_iter()
            .find(|c| c.name() == name)
    }
}

/// A link's transport ([`LinkOpen::kind`]) in the tables: `tcp`, `ws`,
/// `webrtc`; `None` for another.
pub fn kind_name(kind: LinkKind) -> Option<&'static str> {
    match kind {
        LinkKind::Tcp => Some("tcp"),
        LinkKind::Ws => Some("ws"),
        LinkKind::WebRtc => Some("webrtc"),
        LinkKind::Other => None,
    }
}

/// The transport named `name` ([`kind_name`]).
pub fn parse_kind(name: &str) -> Option<LinkKind> {
    [LinkKind::Tcp, LinkKind::Ws, LinkKind::WebRtc]
        .into_iter()
        .find(|&k| kind_name(k) == Some(name))
}

/// A link named in a take ([`LinkRows::links`]): its name in the rows, its
/// transport, its peer, and its ordinal among the node's links to the peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkOpen {
    pub link: u64,
    pub kind: LinkKind,
    pub peer: HostId,
    pub ordinal: u32,
}

/// A write of a link's writer (see the [module docs](self)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkWrite {
    pub link: u64,
    /// When the writer began it.
    pub began: Time,
    /// When the node handed it to the writer; `None` where its hand-off
    /// was lost.
    pub queued: Option<Time>,
    /// When the transport took its last byte.
    pub done: Time,
    /// Its first byte's offset on the link, and its ordinal.
    pub off: u64,
    pub n: u64,
    pub bytes: u64,
    /// The transport's buffered bytes at the writer's last check before
    /// it went; `None` for a transport that doesn't tell.
    pub unsent_before: Option<u64>,
    /// What held it last; `None` if nothing did.
    pub wait: Option<WriteWait>,
}

/// A read of a link's reader, or a message as its transport delivered it
/// (see the [module docs](self)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkRead {
    pub link: u64,
    /// When it was read, or the message arrived.
    pub at: Time,
    /// When the node took its last byte; `None` if the link went first.
    pub taken: Option<Time>,
    pub off: u64,
    pub n: u64,
    pub bytes: u64,
}

/// A reading of a link's transport or queues (see the [module docs](self)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkSample {
    pub link: u64,
    pub at: Time,
    pub source: SampleSource,
    pub cause: SampleCause,
    pub state: LinkState,
}

/// What a take of a [`LinkLog`] gives: the links named since the last
/// take, their rows, and the records the log's caps left out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkRows {
    pub links: Vec<LinkOpen>,
    pub writes: Vec<LinkWrite>,
    pub reads: Vec<LinkRead>,
    pub samples: Vec<LinkSample>,
    pub lost: u64,
}

impl LinkRows {
    /// Whether the take holds nothing.
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
            && self.writes.is_empty()
            && self.reads.is_empty()
            && self.samples.is_empty()
            && self.lost == 0
    }
}

/// A link's I/O record as the log keeps it: a [`LinkEvent`] but a sample,
/// which the log keeps apart, so its buffer's records stay small.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Io {
    Opened(LinkKind),
    Peer(HostId),
    Queued,
    Began(u32),
    Held(WriteWait),
    Checked {
        source: SampleSource,
        buffered: u64,
        low: Option<u64>,
    },
    Low {
        source: SampleSource,
        buffered: u64,
    },
    Done,
    Arrived(u32),
    Read(u32),
    Taken(u32),
    Closed,
}

fn len32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// A write the writer began and its transport hasn't taken whole.
#[derive(Clone, Copy, Debug)]
struct Begun {
    began: Time,
    bytes: u64,
    /// The amount its writer's last check read ...
    checked: Option<u64>,
    /// ... and that check, until a hold follows it, which makes it a
    /// sample.
    check: Option<(Time, SampleSource, u64, Option<u64>)>,
    wait: Option<WriteWait>,
}

/// What the log knows of one link between takes.
#[derive(Debug, Default)]
struct Link {
    kind: Option<LinkKind>,
    named: bool,
    /// The node's hand-offs not yet written, in order. Bound: the link's
    /// write buffers.
    queued: VecDeque<Time>,
    /// The write begun and not yet done. A later `Began` before its `Done`
    /// supersedes it, with the checks and hold logged since, so the write
    /// begins at the later one: the sim's hold start relies on this, logging
    /// a pass's held write's `Began` again where its hold starts (strims-sim
    /// net's `flush_log`).
    begun: Option<Begun>,
    written: u64,
    writes: u64,
    /// Reads (or messages) not yet taken whole: when, their offset, length
    /// and ordinal. Bound: the link's read buffers, or a browser socket's
    /// inbox.
    unread: VecDeque<(Time, u64, u64, u64)>,
    /// Whether the transport delivers messages: they, not the reads, are
    /// the link's reads.
    messages: bool,
    read: u64,
    reads: u64,
    taken: u64,
    /// Rows made before the link's peer was named. Bound: a handshake's
    /// writes and reads.
    early: LinkRows,
}

/// The rows a take is making, and the links the log tracks.
#[derive(Debug, Default)]
struct Assembler {
    /// Bound: the node's links that haven't gone.
    links: BTreeMap<u64, Link>,
    /// The links named so far to each peer.
    ordinals: BTreeMap<HostId, u32>,
    rows: LinkRows,
}

impl Assembler {
    fn take_io(&mut self, at: Time, link: u64, io: Io) {
        // A node that restarted names its links afresh.
        if matches!(io, Io::Opened(_)) && self.links.contains_key(&link) {
            self.take_io(at, link, Io::Closed);
        }
        let l = self.links.entry(link).or_default();
        let mut row = LinkRows::default();
        match io {
            Io::Opened(kind) => l.kind = Some(kind),
            Io::Peer(peer) => {
                if !l.named {
                    l.named = true;
                    let n = self.ordinals.entry(peer).or_default();
                    if let Some(kind) = l.kind {
                        self.rows.links.push(LinkOpen {
                            link,
                            kind,
                            peer,
                            ordinal: *n,
                        });
                    }
                    *n += 1;
                    row = std::mem::take(&mut l.early);
                }
            }
            Io::Queued => {
                l.queued.push_back(at);
            }
            Io::Began(bytes) => {
                l.begun = Some(Begun {
                    began: at,
                    bytes: u64::from(bytes),
                    checked: None,
                    check: None,
                    wait: None,
                });
            }
            Io::Held(w) => {
                if let Some(b) = &mut l.begun {
                    b.wait = Some(w);
                    if let Some((t, source, buffered, low)) = b.check.take() {
                        row.samples.push(LinkSample {
                            link,
                            at: t,
                            source,
                            cause: SampleCause::Write,
                            state: LinkState {
                                buffered: Some(buffered),
                                lowat: low,
                                ..LinkState::default()
                            },
                        });
                    }
                }
            }
            Io::Checked {
                source,
                buffered,
                low,
            } => {
                if let Some(b) = &mut l.begun {
                    b.checked = Some(buffered);
                    b.check = Some((at, source, buffered, low));
                }
            }
            Io::Low { source, buffered } => {
                row.samples.push(LinkSample {
                    link,
                    at,
                    source,
                    cause: SampleCause::Low,
                    state: LinkState {
                        buffered: Some(buffered),
                        ..LinkState::default()
                    },
                });
            }
            Io::Done => {
                if let Some(b) = l.begun.take() {
                    row.writes.push(LinkWrite {
                        link,
                        began: b.began,
                        queued: l.queued.pop_front(),
                        done: at,
                        off: l.written,
                        n: l.writes,
                        bytes: b.bytes,
                        unsent_before: b.checked,
                        wait: b.wait,
                    });
                    l.written += b.bytes;
                    l.writes += 1;
                }
            }
            Io::Arrived(bytes) => {
                l.messages = true;
                Self::unread(l, at, bytes);
            }
            Io::Read(bytes) => {
                if !l.messages {
                    Self::unread(l, at, bytes);
                }
            }
            Io::Taken(bytes) => {
                l.taken += u64::from(bytes);
                while let Some(&(t, off, len, n)) = l.unread.front() {
                    if off + len > l.taken {
                        break;
                    }
                    l.unread.pop_front();
                    row.reads.push(LinkRead {
                        link,
                        at: t,
                        taken: Some(at),
                        off,
                        n,
                        bytes: len,
                    });
                }
            }
            Io::Closed => {
                if let Some(l) = self.links.remove(&link)
                    && l.named
                {
                    for (t, off, len, n) in l.unread {
                        self.rows.reads.push(LinkRead {
                            link,
                            at: t,
                            taken: None,
                            off,
                            n,
                            bytes: len,
                        });
                    }
                }
                return;
            }
        }
        if l.named {
            self.rows.writes.append(&mut row.writes);
            self.rows.reads.append(&mut row.reads);
            self.rows.samples.append(&mut row.samples);
        } else {
            l.early.writes.append(&mut row.writes);
            l.early.reads.append(&mut row.reads);
            l.early.samples.append(&mut row.samples);
        }
    }

    fn unread(l: &mut Link, at: Time, bytes: u32) {
        let len = u64::from(bytes);
        l.unread.push_back((at, l.read, len, l.reads));
        l.read += len;
        l.reads += 1;
    }

    fn take_sample(&mut self, s: LinkSample) {
        let Some(l) = self.links.get_mut(&s.link) else {
            return;
        };
        if l.named {
            self.rows.samples.push(s);
        } else {
            l.early.samples.push(s);
        }
    }
}

/// A buffer the log's sink fills and its takes empty. A node's sinks are
/// `Send + Sync`; one thread fills and takes, so the lock never waits.
#[allow(clippy::disallowed_types)]
type Buffer<T> = Arc<std::sync::Mutex<Vec<T>>>;

/// An empty buffer of room for `cap`, which the sink fills without
/// growing it.
#[allow(clippy::disallowed_types)]
fn buffer<T>(cap: usize) -> Buffer<T> {
    Arc::new(std::sync::Mutex::new(Vec::with_capacity(cap)))
}

/// The buffer `b` holds, swapped for an empty one of room for `cap`.
fn swap<T>(b: &Buffer<T>, cap: usize) -> Vec<T> {
    let mut v = b.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    std::mem::replace(&mut *v, Vec::with_capacity(cap))
}

/// Keeps `x` in `b` if it holds fewer than `cap`; else counts it in `lost`.
fn keep<T>(b: &Buffer<T>, cap: usize, lost: &AtomicU64, x: T) {
    let mut v = b.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if v.len() < cap {
        v.push(x);
    } else {
        lost.fetch_add(1, Ordering::Relaxed);
    }
}

/// See the [module docs](self). Clones share the log; only the clone that
/// takes needs to be on the thread that made it.
#[derive(Clone)]
pub struct LinkLog {
    io: Buffer<(Time, u64, Io)>,
    samples: Buffer<LinkSample>,
    lost: Arc<AtomicU64>,
    clock: Arc<dyn Fn() -> Time + Send + Sync>,
    maker: Rc<RefCell<LinkRowsMaker>>,
}

/// The records a raw take took ([`LinkLog::take_raw`]), and those the
/// log left out since the last: rows once a [`LinkRowsMaker`] makes them,
/// wherever that runs ([`Wire`] carries them there).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkTake {
    io: Vec<(Time, u64, Io)>,
    samples: Vec<LinkSample>,
    lost: u64,
}

/// What makes a log's raw takes rows, given them in the order they were
/// taken: the links the log tracks between takes.
#[derive(Debug, Default)]
pub struct LinkRowsMaker(Assembler);

impl LinkRowsMaker {
    /// The rows `take`'s records made (see the [module docs](self)), and
    /// the records left out.
    pub fn rows(&mut self, take: LinkTake) -> LinkRows {
        // The I/O first: a sample's link is known once its records are.
        for (at, link, io) in take.io {
            self.0.take_io(at, link, io);
        }
        for s in take.samples {
            self.0.take_sample(s);
        }
        let mut rows = std::mem::take(&mut self.0.rows);
        rows.lost = take.lost;
        rows
    }
}

impl std::fmt::Debug for LinkLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LinkLog")
    }
}

impl LinkLog {
    /// The I/O records a log keeps between two takes.
    pub const CAP: usize = 1 << 15;
    /// The capture interval's samples a log keeps between two takes.
    pub const SAMPLE_CAP: usize = 4096;

    /// An empty log whose sink stamps records by `clock`.
    pub fn new(clock: Arc<dyn Fn() -> Time + Send + Sync>) -> LinkLog {
        LinkLog {
            io: buffer(Self::CAP),
            samples: buffer(Self::SAMPLE_CAP),
            lost: Arc::default(),
            clock,
            maker: Rc::default(),
        }
    }

    /// The sink for the node's config.
    pub fn sink(&self) -> LinkSink {
        let (io, samples, lost, clock) = (
            self.io.clone(),
            self.samples.clone(),
            self.lost.clone(),
            self.clock.clone(),
        );
        LinkSink(Arc::new(
            move |LinkRecord { link, at, event }: LinkRecord| {
                let at = at.unwrap_or_else(|| clock());
                let record = match event {
                    LinkEvent::Opened(kind) => Io::Opened(kind),
                    LinkEvent::Peer(peer) => Io::Peer(peer),
                    LinkEvent::Queued(_) => Io::Queued,
                    LinkEvent::Began(n) => Io::Began(len32(n)),
                    LinkEvent::Held(w) => Io::Held(w),
                    LinkEvent::Checked {
                        source,
                        buffered,
                        low,
                    } => Io::Checked {
                        source,
                        buffered,
                        low,
                    },
                    LinkEvent::Low { source, buffered } => Io::Low { source, buffered },
                    LinkEvent::Done => Io::Done,
                    LinkEvent::Arrived(n) => Io::Arrived(len32(n)),
                    LinkEvent::Read(n) => Io::Read(len32(n)),
                    LinkEvent::Taken(n) => Io::Taken(len32(n)),
                    LinkEvent::Closed => Io::Closed,
                    LinkEvent::Sample { source, state } => {
                        let s = LinkSample {
                            link,
                            at,
                            source,
                            cause: SampleCause::Sample,
                            state,
                        };
                        keep(&samples, Self::SAMPLE_CAP, &lost, s);
                        return;
                    }
                };
                keep(&io, Self::CAP, &lost, (at, link, record));
            },
        ))
    }

    /// The rows the records since the last take made (see the
    /// [module docs](self)), and the records left out.
    pub fn take(&self) -> LinkRows {
        let take = self.take_raw();
        self.maker.borrow_mut().rows(take)
    }

    /// The records kept since the last take, and those left out, whose
    /// rows a [`LinkRowsMaker`] makes. A log's takes all go one way: by
    /// [`LinkLog::take`], or raw to one maker.
    pub fn take_raw(&self) -> LinkTake {
        LinkTake {
            io: swap(&self.io, Self::CAP),
            samples: swap(&self.samples, Self::SAMPLE_CAP),
            lost: self.lost.swap(0, Ordering::Relaxed),
        }
    }
}

/// µs from `origin` to `t`, floored.
fn us(t: Time, origin: Time) -> i64 {
    t.unix_nanos()
        .saturating_sub(origin.unix_nanos())
        .div_euclid(1000)
}

/// The columns of a sample's state, in [`link_log_json`]'s order: unsent,
/// unacked, cwnd, ssthresh, srtt_us, rttvar_us, retrans, delivery_rate,
/// buffered, lowat, uplink_queue_us, link_pending, write_held,
/// qos_backlog.
const STATE_COLUMNS: usize = 14;

/// Writes a sample's state's [`STATE_COLUMNS`], comma-separated.
fn put_state(out: &mut String, s: &LinkState) {
    let n = |v: Option<u64>| v.map(i128::from);
    let us = |d: Option<Duration>| d.map(|d| i128::from(d.as_nanos().div_euclid(1000)));
    let columns: [Option<i128>; STATE_COLUMNS] = [
        n(s.unsent),
        n(s.unacked),
        n(s.cwnd),
        n(s.ssthresh),
        us(s.srtt),
        us(s.rttvar),
        n(s.retrans),
        n(s.delivery_rate),
        n(s.buffered),
        n(s.lowat),
        us(s.uplink_queue),
        n(s.link_pending),
        n(s.write_held),
        n(s.qos_backlog),
    ];
    for (i, c) in columns.into_iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        put_opt(out, c);
    }
}

/// A take's rows as a line's records hold them (`rec.link_log`), their
/// links' peers by `peer` (`None` for a peer that isn't a testbed node:
/// the link goes unnamed, and a reader leaves its rows out), times in µs
/// from `origin`, floored:
/// `{"lost": n, "links": [[link, kind, peer, ordinal], ...], "writes":
/// [[link, t_us, queued_us, done_us, off, n, bytes, unsent_before, wait],
/// ...], "reads": [[link, t_us, taken_us, off, n, bytes], ...],
/// "samples": [[link, t_us, source, cause, state...], ...]}`, the state in
/// [`STATE_COLUMNS`]' order, `null` for none.
pub fn link_log_json(
    rows: &LinkRows,
    peer: impl Fn(&HostId) -> Option<usize>,
    origin: Time,
) -> String {
    let mut s = String::new();
    put_link_log(&mut s, rows, peer, origin);
    s
}

/// Writes [`link_log_json`]'s text to `out`.
pub fn put_link_log(
    out: &mut String,
    rows: &LinkRows,
    peer: impl Fn(&HostId) -> Option<usize>,
    origin: Time,
) {
    let _ = write!(out, "{{\"lost\":{},\"links\":", rows.lost);
    let named = rows
        .links
        .iter()
        .filter_map(|l| Some((l, kind_name(l.kind)?, peer(&l.peer)?)));
    list(out, named, |s, (l, kind, p)| {
        let _ = write!(s, "[{},\"{kind}\",{p},{}]", l.link, l.ordinal);
    });
    out.push_str(",\"writes\":");
    list(out, &rows.writes, |s, w| {
        let _ = write!(s, "[{},{},", w.link, us(w.began, origin));
        put_opt(s, w.queued.map(|t| us(t, origin)));
        let _ = write!(s, ",{},{},{},{},", us(w.done, origin), w.off, w.n, w.bytes);
        put_opt(s, w.unsent_before);
        match w.wait {
            Some(w) => {
                let _ = write!(s, ",\"{}\"]", w.name());
            }
            None => s.push_str(",null]"),
        }
    });
    out.push_str(",\"reads\":");
    list(out, &rows.reads, |s, r| {
        let _ = write!(s, "[{},{},", r.link, us(r.at, origin));
        put_opt(s, r.taken.map(|t| us(t, origin)));
        let _ = write!(s, ",{},{},{}]", r.off, r.n, r.bytes);
    });
    out.push_str(",\"samples\":");
    list(out, &rows.samples, |s, x| {
        let _ = write!(
            s,
            "[{},{},\"{}\",\"{}\",",
            x.link,
            us(x.at, origin),
            x.source.name(),
            x.cause.name(),
        );
        put_state(s, &x.state);
        s.push(']');
    });
    out.push('}');
}

impl Wire for LinkKind {
    fn to_wire(&self, out: &mut Vec<u8>) {
        out.push(match self {
            LinkKind::Tcp => 0,
            LinkKind::Ws => 1,
            LinkKind::WebRtc => 2,
            LinkKind::Other => 3,
        });
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        Some(match u8::from_wire(r)? {
            0 => LinkKind::Tcp,
            1 => LinkKind::Ws,
            2 => LinkKind::WebRtc,
            3 => LinkKind::Other,
            _ => return None,
        })
    }
}

impl Wire for SampleSource {
    fn to_wire(&self, out: &mut Vec<u8>) {
        put_name(self.name(), out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        SampleSource::parse(take_name(r)?)
    }
}

impl Wire for WriteWait {
    fn to_wire(&self, out: &mut Vec<u8>) {
        put_name(self.name(), out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        WriteWait::parse(take_name(r)?)
    }
}

impl Wire for Io {
    fn to_wire(&self, out: &mut Vec<u8>) {
        match *self {
            Io::Opened(kind) => {
                out.push(0);
                kind.to_wire(out);
            }
            Io::Peer(peer) => {
                out.push(1);
                peer.to_wire(out);
            }
            Io::Queued => out.push(2),
            Io::Began(n) => {
                out.push(3);
                n.to_wire(out);
            }
            Io::Held(w) => {
                out.push(4);
                w.to_wire(out);
            }
            Io::Checked {
                source,
                buffered,
                low,
            } => {
                out.push(5);
                (source, buffered, low).to_wire(out);
            }
            Io::Low { source, buffered } => {
                out.push(6);
                (source, buffered).to_wire(out);
            }
            Io::Done => out.push(7),
            Io::Arrived(n) => {
                out.push(8);
                n.to_wire(out);
            }
            Io::Read(n) => {
                out.push(9);
                n.to_wire(out);
            }
            Io::Taken(n) => {
                out.push(10);
                n.to_wire(out);
            }
            Io::Closed => out.push(11),
        }
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        Some(match u8::from_wire(r)? {
            0 => Io::Opened(Wire::from_wire(r)?),
            1 => Io::Peer(Wire::from_wire(r)?),
            2 => Io::Queued,
            3 => Io::Began(Wire::from_wire(r)?),
            4 => Io::Held(Wire::from_wire(r)?),
            5 => {
                let (source, buffered, low) = Wire::from_wire(r)?;
                Io::Checked {
                    source,
                    buffered,
                    low,
                }
            }
            6 => {
                let (source, buffered) = Wire::from_wire(r)?;
                Io::Low { source, buffered }
            }
            7 => Io::Done,
            8 => Io::Arrived(Wire::from_wire(r)?),
            9 => Io::Read(Wire::from_wire(r)?),
            10 => Io::Taken(Wire::from_wire(r)?),
            11 => Io::Closed,
            _ => return None,
        })
    }
}

impl Wire for LinkState {
    fn to_wire(&self, out: &mut Vec<u8>) {
        let s = self;
        (s.unsent, s.unacked, s.cwnd, s.ssthresh).to_wire(out);
        (s.srtt, s.rttvar, s.retrans, s.delivery_rate).to_wire(out);
        (s.buffered, s.lowat, s.uplink_queue, s.link_pending).to_wire(out);
        (s.write_held, s.qos_backlog).to_wire(out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        let (unsent, unacked, cwnd, ssthresh) = Wire::from_wire(r)?;
        let (srtt, rttvar, retrans, delivery_rate) = Wire::from_wire(r)?;
        let (buffered, lowat, uplink_queue, link_pending) = Wire::from_wire(r)?;
        let (write_held, qos_backlog) = Wire::from_wire(r)?;
        Some(LinkState {
            unsent,
            unacked,
            cwnd,
            ssthresh,
            srtt,
            rttvar,
            retrans,
            delivery_rate,
            buffered,
            lowat,
            uplink_queue,
            link_pending,
            write_held,
            qos_backlog,
        })
    }
}

impl Wire for LinkSample {
    fn to_wire(&self, out: &mut Vec<u8>) {
        (self.link, self.at, self.source).to_wire(out);
        put_name(self.cause.name(), out);
        self.state.to_wire(out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        let (link, at, source) = Wire::from_wire(r)?;
        Some(LinkSample {
            link,
            at,
            source,
            cause: SampleCause::parse(take_name(r)?)?,
            state: Wire::from_wire(r)?,
        })
    }
}

impl Wire for LinkTake {
    fn to_wire(&self, out: &mut Vec<u8>) {
        self.io.to_wire(out);
        self.samples.to_wire(out);
        self.lost.to_wire(out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        let (io, samples, lost) = Wire::from_wire(r)?;
        Some(LinkTake { io, samples, lost })
    }
}

/// A link a line names ([`link_log_of`]): its name in the rows, transport,
/// peer (a testbed node's index) and ordinal to that peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkNamed {
    pub link: u64,
    pub kind: LinkKind,
    pub peer: usize,
    pub ordinal: u32,
}

/// What [`link_log_of`] reads back: the links a line names, its rows, and its
/// lost count.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkLines {
    pub links: Vec<LinkNamed>,
    pub writes: Vec<LinkWrite>,
    pub reads: Vec<LinkRead>,
    pub samples: Vec<LinkSample>,
    pub lost: u64,
}

/// A line's `rec.link_log` ([`link_log_json`]) read back, times from
/// `origin`; `None` for one that isn't that object. A row that doesn't
/// parse is left out.
pub fn link_log_of(v: &serde_json::Value, origin: Time) -> Option<LinkLines> {
    let at = |us: i64| origin + Duration::from_micros(us);
    let arr = |k: &str| v.get(k).and_then(serde_json::Value::as_array);
    let lost = v.get("lost")?.as_u64()?;
    let int = |r: &serde_json::Value, k: usize| r.get(k).and_then(serde_json::Value::as_i64);
    let uint = |r: &serde_json::Value, k: usize| r.get(k).and_then(serde_json::Value::as_u64);
    let links = arr("links")?
        .iter()
        .filter_map(|r| {
            Some(LinkNamed {
                link: uint(r, 0)?,
                kind: parse_kind(r.get(1)?.as_str()?)?,
                peer: usize::try_from(uint(r, 2)?).ok()?,
                ordinal: u32::try_from(uint(r, 3)?).ok()?,
            })
        })
        .collect();
    let writes = arr("writes")?
        .iter()
        .filter_map(|r| {
            Some(LinkWrite {
                link: uint(r, 0)?,
                began: at(int(r, 1)?),
                queued: int(r, 2).map(at),
                done: at(int(r, 3)?),
                off: uint(r, 4)?,
                n: uint(r, 5)?,
                bytes: uint(r, 6)?,
                unsent_before: uint(r, 7),
                wait: r
                    .get(8)
                    .and_then(serde_json::Value::as_str)
                    .and_then(WriteWait::parse),
            })
        })
        .collect();
    let reads = arr("reads")?
        .iter()
        .filter_map(|r| {
            Some(LinkRead {
                link: uint(r, 0)?,
                at: at(int(r, 1)?),
                taken: int(r, 2).map(at),
                off: uint(r, 3)?,
                n: uint(r, 4)?,
                bytes: uint(r, 5)?,
            })
        })
        .collect();
    let samples = arr("samples")?
        .iter()
        .filter_map(|r| {
            let u = |k: usize| uint(r, 4 + k);
            let d = |k: usize| int(r, 4 + k).map(Duration::from_micros);
            Some(LinkSample {
                link: uint(r, 0)?,
                at: at(int(r, 1)?),
                source: SampleSource::parse(r.get(2)?.as_str()?)?,
                cause: SampleCause::parse(r.get(3)?.as_str()?)?,
                state: LinkState {
                    unsent: u(0),
                    unacked: u(1),
                    cwnd: u(2),
                    ssthresh: u(3),
                    srtt: d(4),
                    rttvar: d(5),
                    retrans: u(6),
                    delivery_rate: u(7),
                    buffered: u(8),
                    lowat: u(9),
                    uplink_queue: d(10),
                    link_pending: u(11),
                    write_held: u(12),
                    qos_backlog: u(13),
                },
            })
        })
        .collect();
    Some(LinkLines {
        links,
        writes,
        reads,
        samples,
        lost,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use std::sync::atomic::AtomicI64;

    use super::*;

    /// A log whose clock reads `now` ms past an epoch.
    fn log() -> (LinkLog, Arc<AtomicI64>) {
        let now = Arc::new(AtomicI64::new(0));
        let n = now.clone();
        let log = LinkLog::new(Arc::new(move || {
            Time::from_unix_millis(1_700_000_000_000 + n.load(Ordering::Relaxed))
        }));
        (log, now)
    }

    fn t(ms: i64) -> Time {
        Time::from_unix_millis(1_700_000_000_000 + ms)
    }

    fn peer(k: u64) -> HostId {
        HostId([k, 0, 0, 0])
    }

    /// Writes pair with their hand-offs in order, at running offsets and
    /// ordinals, with the last check's amount and the last hold; reads
    /// become rows once taken whole, a data channel's messages being its
    /// reads; rows before the peer is named wait for it; a link's ordinal
    /// counts the links named to its peer; samples keep their causes.
    #[test]
    fn a_link_log_makes_rows_in_order() {
        let (log, now) = log();
        let sink = log.sink();
        let at = |ms: i64| now.store(ms, Ordering::Relaxed);
        at(0);
        sink.record(1, LinkEvent::Opened(LinkKind::WebRtc));
        sink.record(1, LinkEvent::Queued(100));
        at(1);
        sink.record(1, LinkEvent::Began(100));
        sink.record(1, LinkEvent::Done);
        sink.record(1, LinkEvent::Peer(peer(7)));
        at(2);
        sink.record(1, LinkEvent::Queued(50));
        sink.record(1, LinkEvent::Queued(60));
        at(3);
        sink.record(1, LinkEvent::Began(50));
        sink.record(
            1,
            LinkEvent::Checked {
                source: SampleSource::WebRtc,
                buffered: 900,
                low: Some(500),
            },
        );
        sink.record(1, LinkEvent::Held(WriteWait::Channel));
        at(5);
        sink.record(
            1,
            LinkEvent::Low {
                source: SampleSource::WebRtc,
                buffered: 400,
            },
        );
        sink.record(
            1,
            LinkEvent::Checked {
                source: SampleSource::WebRtc,
                buffered: 400,
                low: None,
            },
        );
        sink.record(1, LinkEvent::Held(WriteWait::Host));
        at(9);
        sink.record(1, LinkEvent::Done);
        sink.record(1, LinkEvent::Arrived(30));
        sink.record(1, LinkEvent::Arrived(20));
        sink.record(1, LinkEvent::Read(40));
        sink.record(1, LinkEvent::Taken(40));
        let rows = log.take();
        assert_eq!(
            rows.links,
            [LinkOpen {
                link: 1,
                kind: LinkKind::WebRtc,
                peer: peer(7),
                ordinal: 0
            }]
        );
        assert_eq!(rows.writes.len(), 2);
        assert_eq!(
            rows.writes[0],
            LinkWrite {
                link: 1,
                began: t(1),
                queued: Some(t(0)),
                done: t(1),
                off: 0,
                n: 0,
                bytes: 100,
                unsent_before: None,
                wait: None,
            }
        );
        assert_eq!(
            rows.writes[1],
            LinkWrite {
                link: 1,
                began: t(3),
                queued: Some(t(2)),
                done: t(9),
                off: 100,
                n: 1,
                bytes: 50,
                unsent_before: Some(400),
                wait: Some(WriteWait::Host),
            }
        );
        // The first message is taken whole; the second waits.
        assert_eq!(
            rows.reads,
            [LinkRead {
                link: 1,
                at: t(9),
                taken: Some(t(9)),
                off: 0,
                n: 0,
                bytes: 30
            }]
        );
        let causes: Vec<_> = rows.samples.iter().map(|s| s.cause).collect();
        assert_eq!(
            causes,
            [SampleCause::Write, SampleCause::Low, SampleCause::Write]
        );
        assert_eq!(rows.samples[0].state.lowat, Some(500));
        assert_eq!(rows.lost, 0);

        // The link goes: its untaken message is a read never taken. A
        // second link to the same peer is its next ordinal; its stream
        // reads are its reads.
        at(10);
        sink.record(1, LinkEvent::Closed);
        sink.record(2, LinkEvent::Opened(LinkKind::Tcp));
        sink.record(2, LinkEvent::Peer(peer(7)));
        sink.record(2, LinkEvent::Read(10));
        sink.record(2, LinkEvent::Taken(10));
        sink.record(
            2,
            LinkEvent::Sample {
                source: SampleSource::Tcp,
                state: LinkState {
                    unsent: Some(3),
                    ..LinkState::default()
                },
            },
        );
        let rows = log.take();
        assert_eq!(rows.links[0].ordinal, 1);
        assert_eq!(
            rows.reads,
            [
                LinkRead {
                    link: 1,
                    at: t(9),
                    taken: None,
                    off: 30,
                    n: 1,
                    bytes: 20
                },
                LinkRead {
                    link: 2,
                    at: t(10),
                    taken: Some(t(10)),
                    off: 0,
                    n: 0,
                    bytes: 10
                }
            ]
        );
        assert_eq!(rows.samples.len(), 1);
        assert_eq!(rows.samples[0].cause, SampleCause::Sample);
    }

    /// Records past the cap between takes are counted lost, and a take
    /// empties the log.
    #[test]
    fn a_link_log_keeps_its_cap_between_takes() {
        let (log, _) = log();
        let sink = log.sink();
        sink.record(1, LinkEvent::Opened(LinkKind::Tcp));
        for _ in 0..LinkLog::CAP + 4 {
            sink.record(1, LinkEvent::Queued(1));
        }
        assert_eq!(log.take().lost, 5);
        assert!(log.take().is_empty());
    }

    /// A raw take takes the records kept by then; the next takes those
    /// after. Its records cross as bytes, and a maker given the takes in
    /// order makes the rows a take of the log would.
    #[test]
    fn a_raw_take_takes_what_was_kept_by_then() {
        let records = |log: &LinkLog| {
            let sink = log.sink();
            sink.record(1, LinkEvent::Opened(LinkKind::Tcp));
            sink.record(1, LinkEvent::Peer(peer(1)));
            for _ in 0..3 {
                sink.record(1, LinkEvent::Queued(1));
                sink.record(1, LinkEvent::Began(1));
                sink.record(
                    1,
                    LinkEvent::Checked {
                        source: SampleSource::Tcp,
                        buffered: 9,
                        low: Some(4),
                    },
                );
                sink.record(1, LinkEvent::Held(WriteWait::Notsent));
                sink.record(1, LinkEvent::Done);
                sink.record(1, LinkEvent::Read(2));
                sink.record(1, LinkEvent::Taken(2));
                sink.record(
                    1,
                    LinkEvent::Sample {
                        source: SampleSource::Tcp,
                        state: LinkState {
                            srtt: Some(Duration::from_micros(800)),
                            cwnd: Some(10),
                            ..LinkState::default()
                        },
                    },
                );
            }
        };
        let (marked, _) = log();
        let sink = marked.sink();
        sink.record(1, LinkEvent::Opened(LinkKind::Tcp));
        sink.record(1, LinkEvent::Peer(peer(1)));
        sink.record(1, LinkEvent::Queued(1));
        let first = marked.take_raw();
        sink.record(1, LinkEvent::Began(1));
        sink.record(1, LinkEvent::Done);
        assert_eq!((first.io.len(), first.samples.len()), (3, 0));
        let rest = marked.take_raw();
        assert_eq!(rest.io.len(), 2);
        let mut maker = LinkRowsMaker::default();
        assert!(maker.rows(first).writes.is_empty());
        assert_eq!(maker.rows(rest).writes.len(), 1);

        let (whole, _) = log();
        records(&whole);
        let (raw, _) = log();
        records(&raw);
        let take = raw.take_raw();
        let mut bytes = Vec::new();
        take.to_wire(&mut bytes);
        let mut r = bytes.as_slice();
        let back = LinkTake::from_wire(&mut r).unwrap();
        assert!(r.is_empty());
        assert_eq!(back, take);
        let rows = LinkRowsMaker::default().rows(back);
        assert_eq!(rows, whole.take());
        assert_eq!(
            (rows.writes.len(), rows.reads.len(), rows.samples.len()),
            (3, 3, 6)
        );
    }

    /// A take's rows read back as written, a peer that isn't a node's left
    /// out.
    #[test]
    fn link_rows_read_back_as_written() {
        let (log, now) = log();
        let sink = log.sink();
        for (link, p) in [(1, 3), (2, 99)] {
            sink.record(link, LinkEvent::Opened(LinkKind::Ws));
            sink.record(link, LinkEvent::Peer(peer(p)));
        }
        now.store(2, Ordering::Relaxed);
        sink.record(1, LinkEvent::Queued(8));
        sink.record(1, LinkEvent::Began(8));
        sink.record(1, LinkEvent::Held(WriteWait::Notsent));
        sink.record(1, LinkEvent::Done);
        sink.record(1, LinkEvent::Read(5));
        sink.record(1, LinkEvent::Taken(5));
        sink.record(
            1,
            LinkEvent::Sample {
                source: SampleSource::Ws,
                state: LinkState {
                    srtt: Some(Duration::from_micros(1500)),
                    buffered: Some(4),
                    ..LinkState::default()
                },
            },
        );
        let rows = log.take();
        let origin = t(1);
        let json = link_log_json(&rows, |p| (p.0[0] < 10).then_some(p.0[0] as usize), origin);
        let back = link_log_of(&serde_json::from_str(&json).unwrap(), origin).unwrap();
        assert_eq!(
            back.links,
            [LinkNamed {
                link: 1,
                kind: LinkKind::Ws,
                peer: 3,
                ordinal: 0
            }]
        );
        assert_eq!(back.writes, rows.writes);
        assert_eq!(back.reads, rows.reads);
        assert_eq!(back.samples, rows.samples);
        assert_eq!(back.lost, 0);
    }
}
