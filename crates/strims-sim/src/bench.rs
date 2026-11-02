// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A TCP bench with none of the node's machinery: each node writes records
//! on a schedule the test gives and reads what its links carry, on real
//! sockets in the Docker testbed (the `testbed_bench` example, through
//! `hack/testbed/bench.sh`) and on the sim's sockets under the WAN model
//! ([`simulate`]), so the model's send buffers, windows and access links
//! can be checked against a real stack write by write.
//!
//! Sockets are set up as the node's links are ([`TcpOptions`] and
//! [`TcpSocketConfig`] defaults: `TCP_NODELAY`, 2 MiB buffers, a 16 KiB
//! `TCP_NOTSENT_LOWAT`), and a link's records leave one write at a time, in
//! order, as vnic's frames do.
//!
//! # Schedule
//!
//! One entry a line, `#` to the end of a line a comment; times are
//! milliseconds from the start, sizes bytes:
//!
//! - `burst <from> <to> <start> <period> <bytes> <write> [<end>]`: from
//!   `start`, every `period` until `end` (the run's end without), node
//!   `from` queues `bytes` for `to` as records of at most `write` bytes.
//! - `probe <from> <to> <start> <period> [<end>]`: a probe record, queued
//!   ahead of the data records waiting on its link, which `to` echoes back
//!   as soon as it reads it, ahead of its own. The probes and echoes
//!   waiting on a link leave in one write, as vnic bundles messages in a
//!   frame.
//!
//! An entry's nodes must be linked.
//!
//! # Records
//!
//! A record is its kind (a byte: data 0, probe 1, echo 2), its length
//! (u32, header included), its sequence number on its link and kind
//! (u32), when it was due and when its write began (i64 µs from the start;
//! an echo carries its probe's), then zeros; integers big-endian. A
//! dialer first writes its node index (u32).
//!
//! # Log
//!
//! Each node logs JSON lines, times in µs from the start:
//!
//! - `{"ev":"w","to":..,"k":..,"seq":..,"len":..,"due":..,"at":..,"done":..}`:
//!   a record written, `at` when its write began and `done` when its last
//!   byte was in the socket.
//! - `{"ev":"r","from":..,"k":..,"seq":..,"len":..,"due":..,"at":..,"got":..}`:
//!   a record read whole, `due` and `at` as it carries them.
//! - `{"ev":"s","to":..,"t":..,..}`: the link's socket every
//!   [`SAMPLE_EVERY`] as its link probe reads it: the kernel's `TCP_INFO`
//!   (`unsent`, `unacked`, `cwnd` in bytes, `srtt` in µs) or the model's
//!   view of the connection (`unsent`, `in_flight`, `pending` and `cwnd`
//!   in bytes, `upq` the uplink queue in µs).
//!
//! [`simulate`] adds a `{"ev":"q","t":..,"wait":..}` line per sample of
//! the node's uplink queue (µs), as the testbed's `tc-0.log` samples the
//! seed's, and a `{"ev":"x","t":..,"bytes":..}` line per sample of the
//! wire bytes its competing uploads sent (as `run.sh`'s `COMPETE` runs
//! them, on the testbed in `compete-<s>.json`).

// The bench's nodes share their state between their tasks on one thread.
#![allow(clippy::disallowed_types)]

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use futures_util::future::join_all;
use strims_base::time::{Duration, Time};
use strims_node::{CaptureSample, LinkProbe, Net, TcpOptions, TcpSocketConfig};
use strims_wan::{Access, City, Options, Site};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Notify;

use crate::net::{SendBuffers, SimNet};
use crate::spec::{Competing, Scenario, SendBuffer};
use crate::wan::{Scheduler, SharedWan, sim_time};

/// A record's header: kind, length, sequence number, due and write times.
pub const HEADER: usize = 1 + 4 + 4 + 8 + 8;
/// How often a node samples each link's socket.
pub const SAMPLE_EVERY: StdDuration = StdDuration::from_millis(10);
/// How long a node keeps reading after its schedule ends.
pub const LINGER: StdDuration = StdDuration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Data,
    Probe,
    Echo,
}

impl Kind {
    fn code(self) -> u8 {
        match self {
            Kind::Data => 0,
            Kind::Probe => 1,
            Kind::Echo => 2,
        }
    }

    fn of(code: u8) -> Option<Kind> {
        match code {
            0 => Some(Kind::Data),
            1 => Some(Kind::Probe),
            2 => Some(Kind::Echo),
            _ => None,
        }
    }
}

/// One schedule entry (see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    Burst {
        from: usize,
        to: usize,
        start_ms: u64,
        period_ms: u64,
        bytes: usize,
        write: usize,
        end_ms: Option<u64>,
    },
    Probe {
        from: usize,
        to: usize,
        start_ms: u64,
        period_ms: u64,
        end_ms: Option<u64>,
    },
}

impl Entry {
    fn ends(&self) -> (usize, usize) {
        match *self {
            Entry::Burst { from, to, .. } | Entry::Probe { from, to, .. } => (from, to),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Schedule {
    pub entries: Vec<Entry>,
}

impl Schedule {
    pub fn parse(text: &str) -> Result<Schedule, String> {
        let mut entries = Vec::new();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("");
            let w: Vec<&str> = line.split_whitespace().collect();
            let n = |s: &str| s.parse::<u64>().map_err(|e| format!("{line:?}: {e}"));
            let ix = |s: &str| n(s).map(|v| v as usize);
            let end = |s: Option<&&str>| s.map(|s| n(s)).transpose();
            match w.as_slice() {
                [] => {}
                ["burst", from, to, start, period, bytes, write, rest @ ..] if rest.len() <= 1 => {
                    let (bytes, write) = (ix(bytes)?, ix(write)?);
                    if write < HEADER {
                        return Err(format!("{line:?}: writes take {HEADER} bytes at least"));
                    }
                    entries.push(Entry::Burst {
                        from: ix(from)?,
                        to: ix(to)?,
                        start_ms: n(start)?,
                        period_ms: n(period)?.max(1),
                        bytes,
                        write,
                        end_ms: end(rest.first())?,
                    });
                }
                ["probe", from, to, start, period, rest @ ..] if rest.len() <= 1 => {
                    entries.push(Entry::Probe {
                        from: ix(from)?,
                        to: ix(to)?,
                        start_ms: n(start)?,
                        period_ms: n(period)?.max(1),
                        end_ms: end(rest.first())?,
                    });
                }
                _ => return Err(format!("bad schedule line {line:?}")),
            }
        }
        Ok(Schedule { entries })
    }
}

/// The bench's clock: microseconds since the run's start, negative before
/// it. Timers are the runtime's, so under turmoil they are simulated.
pub trait BenchClock: Clone + 'static {
    fn now_us(&self) -> i64;

    /// The time a link probe reads its state at.
    fn time(&self) -> Time {
        Time::NIL
    }
}

async fn sleep_until<C: BenchClock>(clock: &C, us: i64) {
    let d = us - clock.now_us();
    if d > 0 {
        tokio::time::sleep(StdDuration::from_micros(d.unsigned_abs())).await;
    }
}

/// What one node does: where it listens, whom it dials, and the schedule.
#[derive(Clone, Debug)]
pub struct NodePlan {
    pub me: usize,
    pub listen: String,
    /// The nodes it dials (its linked nodes of lower index) and their
    /// addresses.
    pub dial: Vec<(usize, String)>,
    /// How many nodes dial it.
    pub accepts: usize,
    pub schedule: Schedule,
    /// When the schedule ends, in µs from the start.
    pub end_us: i64,
}

impl NodePlan {
    /// Node `me`'s plan on `links`, `addr` giving each node's address.
    pub fn new(
        me: usize,
        links: &[(usize, usize)],
        listen: String,
        addr: impl Fn(usize) -> String,
        schedule: Schedule,
        end_us: i64,
    ) -> NodePlan {
        let mut dial = Vec::new();
        let mut accepts = 0;
        for &(a, b) in links {
            let other = match (a == me, b == me) {
                (true, false) => b,
                (false, true) => a,
                _ => continue,
            };
            if other < me {
                dial.push((other, addr(other)));
            } else {
                accepts += 1;
            }
        }
        NodePlan {
            me,
            listen,
            dial,
            accepts,
            schedule,
            end_us,
        }
    }
}

/// A record waiting to be written.
#[derive(Clone, Copy, Debug)]
struct Out {
    kind: Kind,
    len: usize,
    seq: u32,
    due: i64,
    /// An echo's probe's write time.
    at: Option<i64>,
}

/// One link's records waiting to be written: probes and echoes ahead of
/// data.
#[derive(Default)]
struct Outbox {
    control: VecDeque<Out>,
    data: VecDeque<Out>,
}

struct LinkState {
    peer: usize,
    out: RefCell<Outbox>,
    ready: Notify,
    data_seq: Cell<u32>,
    probe_seq: Cell<u32>,
}

impl LinkState {
    fn push(&self, o: Out) {
        {
            let mut b = self.out.borrow_mut();
            match o.kind {
                Kind::Data => b.data.push_back(o),
                Kind::Probe | Kind::Echo => b.control.push_back(o),
            }
        }
        self.ready.notify_one();
    }

    /// The next write: every probe and echo waiting, or else the next
    /// data record.
    fn pop(&self) -> Vec<Out> {
        let mut b = self.out.borrow_mut();
        if b.control.is_empty() {
            b.data.pop_front().into_iter().collect()
        } else {
            b.control.drain(..).collect()
        }
    }
}

type Task = Pin<Box<dyn Future<Output = ()>>>;

/// Runs node `plan.me` on `net` until [`LINGER`] past its schedule's end,
/// logging to `log` (see the module docs).
pub async fn node<N: Net, C: BenchClock>(
    net: N,
    clock: C,
    plan: NodePlan,
    log: impl Fn(String) + Clone + 'static,
) -> io::Result<()> {
    let listener = net.tcp_bind(plan.listen.clone()).await?;
    let me = plan.me;
    let dials = plan.dial.iter().map(|(peer, addr)| {
        let (net, clock, addr, peer) = (net.clone(), clock.clone(), addr.clone(), *peer);
        async move {
            loop {
                match net.tcp_connect(addr.clone()).await {
                    Ok(mut s) => {
                        setup(&net, &s);
                        s.write_all(&u32::try_from(me).unwrap_or(u32::MAX).to_be_bytes())
                            .await?;
                        return Ok::<_, io::Error>((peer, s));
                    }
                    Err(e) if clock.now_us() > 0 => return Err(e),
                    Err(_) => tokio::time::sleep(StdDuration::from_millis(50)).await,
                }
            }
        }
    });
    let accepts = async {
        let mut got = Vec::new();
        while got.len() < plan.accepts {
            let (mut s, _) = net.tcp_accept(&listener).await?;
            setup(&net, &s);
            let mut ix = [0u8; 4];
            s.read_exact(&mut ix).await?;
            got.push((u32::from_be_bytes(ix) as usize, s));
        }
        Ok::<_, io::Error>(got)
    };
    let (dialed, accepted) = tokio::join!(join_all(dials), accepts);
    let mut streams = accepted?;
    for d in dialed {
        streams.push(d?);
    }
    streams.sort_by_key(|(peer, _)| *peer);

    let mut tasks: Vec<Task> = Vec::new();
    let mut links: Vec<Rc<LinkState>> = Vec::new();
    for (peer, s) in streams {
        let probe = net.link_probe(&s, false);
        let link = Rc::new(LinkState {
            peer,
            out: RefCell::new(Outbox::default()),
            ready: Notify::new(),
            data_seq: Cell::new(0),
            probe_seq: Cell::new(0),
        });
        let (r, w) = tokio::io::split(s);
        tasks.push(Box::pin(writer(
            w,
            link.clone(),
            clock.clone(),
            log.clone(),
        )));
        tasks.push(Box::pin(reader(
            r,
            link.clone(),
            clock.clone(),
            log.clone(),
        )));
        if let Some(p) = probe {
            tasks.push(Box::pin(sampler(p, peer, clock.clone(), log.clone())));
        }
        links.push(link);
    }
    for e in &plan.schedule.entries {
        let (from, to) = e.ends();
        if from != me {
            continue;
        }
        let Some(link) = links.iter().find(|l| l.peer == to) else {
            return Err(io::Error::other(format!("{from} and {to} aren't linked")));
        };
        tasks.push(Box::pin(feed(
            e.clone(),
            link.clone(),
            clock.clone(),
            plan.end_us,
        )));
    }
    let stop = plan.end_us + i64::try_from(LINGER.as_micros()).unwrap_or(i64::MAX);
    tokio::select! {
        _ = join_all(tasks) => {}
        () = sleep_until(&clock, stop) => {}
    }
    Ok(())
}

fn setup<N: Net>(net: &N, s: &N::TcpStream) {
    let _ = net.configure_tcp(s, &TcpOptions::default());
    let _ = net.configure_tcp_send_queue(s, &TcpSocketConfig::default());
}

/// Queues `e`'s records on `link` as they come due.
async fn feed<C: BenchClock>(e: Entry, link: Rc<LinkState>, clock: C, end_us: i64) {
    let (start_ms, period_ms, end_ms) = match e {
        Entry::Burst {
            start_ms,
            period_ms,
            end_ms,
            ..
        }
        | Entry::Probe {
            start_ms,
            period_ms,
            end_ms,
            ..
        } => (start_ms, period_ms, end_ms),
    };
    let end = end_ms.map_or(end_us, |ms| (ms as i64 * 1000).min(end_us));
    let mut due = start_ms as i64 * 1000;
    while due < end {
        sleep_until(&clock, due).await;
        match e {
            Entry::Burst { bytes, write, .. } => {
                let mut left = bytes;
                while left > 0 {
                    let len = left.min(write).max(HEADER);
                    left = left.saturating_sub(len);
                    let seq = link.data_seq.get();
                    link.data_seq.set(seq.wrapping_add(1));
                    link.push(Out {
                        kind: Kind::Data,
                        len,
                        seq,
                        due,
                        at: None,
                    });
                }
            }
            Entry::Probe { .. } => {
                let seq = link.probe_seq.get();
                link.probe_seq.set(seq.wrapping_add(1));
                link.push(Out {
                    kind: Kind::Probe,
                    len: HEADER,
                    seq,
                    due,
                    at: None,
                });
            }
        }
        due += period_ms as i64 * 1000;
    }
}

/// Writes `link`'s records, one at a time.
async fn writer<W: AsyncWrite + Unpin, C: BenchClock>(
    mut w: W,
    link: Rc<LinkState>,
    clock: C,
    log: impl Fn(String),
) {
    let mut buf = Vec::new();
    loop {
        let outs = loop {
            let outs = link.pop();
            if !outs.is_empty() {
                break outs;
            }
            link.ready.notified().await;
        };
        let at = clock.now_us();
        buf.clear();
        for o in &outs {
            let start = buf.len();
            buf.push(o.kind.code());
            buf.extend_from_slice(&u32::try_from(o.len).unwrap_or(u32::MAX).to_be_bytes());
            buf.extend_from_slice(&o.seq.to_be_bytes());
            buf.extend_from_slice(&o.due.to_be_bytes());
            buf.extend_from_slice(&o.at.unwrap_or(at).to_be_bytes());
            buf.resize(start + o.len, 0);
        }
        if w.write_all(&buf).await.is_err() {
            return;
        }
        let done = clock.now_us();
        for o in &outs {
            log(format!(
                r#"{{"ev":"w","to":{},"k":{},"seq":{},"len":{},"due":{},"at":{at},"done":{done}}}"#,
                link.peer,
                o.kind.code(),
                o.seq,
                o.len,
                o.due,
            ));
        }
    }
}

/// Reads `link`'s records, logging each and echoing probes.
async fn reader<R: AsyncRead + Unpin, C: BenchClock>(
    mut r: R,
    link: Rc<LinkState>,
    clock: C,
    log: impl Fn(String),
) {
    let mut buf = vec![0u8; 1 << 16];
    let mut head: Vec<u8> = Vec::with_capacity(HEADER);
    let mut left = 0usize;
    loop {
        let n = match r.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        let mut bytes = buf.get(..n).unwrap_or_default();
        while !bytes.is_empty() {
            if head.len() < HEADER {
                let take = (HEADER - head.len()).min(bytes.len());
                let (h, rest) = bytes.split_at(take);
                head.extend_from_slice(h);
                bytes = rest;
                if head.len() < HEADER {
                    break;
                }
                left = header_len(&head).saturating_sub(HEADER);
            } else {
                let take = left.min(bytes.len());
                left -= take;
                bytes = bytes.get(take..).unwrap_or_default();
            }
            if left == 0 && head.len() == HEADER {
                complete(&head, &link, &clock, &log);
                head.clear();
            }
        }
    }
}

fn header_len(h: &[u8]) -> usize {
    let mut b = [0u8; 4];
    b.copy_from_slice(h.get(1..5).unwrap_or(&[0; 4]));
    u32::from_be_bytes(b) as usize
}

fn complete<C: BenchClock>(h: &[u8], link: &LinkState, clock: &C, log: &impl Fn(String)) {
    let got = clock.now_us();
    let u32_at = |i: usize| {
        let mut b = [0u8; 4];
        b.copy_from_slice(h.get(i..i + 4).unwrap_or(&[0; 4]));
        u32::from_be_bytes(b)
    };
    let i64_at = |i: usize| {
        let mut b = [0u8; 8];
        b.copy_from_slice(h.get(i..i + 8).unwrap_or(&[0; 8]));
        i64::from_be_bytes(b)
    };
    let Some(kind) = h.first().copied().and_then(Kind::of) else {
        return;
    };
    let (len, seq, due, at) = (u32_at(1), u32_at(5), i64_at(9), i64_at(17));
    log(format!(
        r#"{{"ev":"r","from":{},"k":{},"seq":{seq},"len":{len},"due":{due},"at":{at},"got":{got}}}"#,
        link.peer,
        kind.code(),
    ));
    if kind == Kind::Probe {
        link.push(Out {
            kind: Kind::Echo,
            len: HEADER,
            seq,
            due,
            at: Some(at),
        });
    }
}

/// Logs `probe`'s readings of the link to `peer` every [`SAMPLE_EVERY`].
async fn sampler<C: BenchClock>(
    mut probe: Box<dyn LinkProbe>,
    peer: usize,
    clock: C,
    log: impl Fn(String),
) {
    let us = |d: Duration| d.as_micros();
    loop {
        let t = clock.now_us();
        let line = match probe.sample(clock.time()) {
            Some(CaptureSample::Tcp {
                unsent,
                unacked,
                cwnd,
                srtt,
                ..
            }) => format!(
                r#"{{"ev":"s","to":{peer},"t":{t},"unsent":{unsent},"unacked":{unacked},"cwnd":{cwnd},"srtt":{}}}"#,
                us(srtt)
            ),
            Some(CaptureSample::SimConn {
                unsent,
                cwnd,
                in_flight,
                pending,
                uplink_queue,
                ..
            }) => format!(
                r#"{{"ev":"s","to":{peer},"t":{t},"unsent":{unsent},"in_flight":{},"pending":{},"cwnd":{},"upq":{}}}"#,
                in_flight.unwrap_or(0),
                pending.unwrap_or(0),
                cwnd.unwrap_or(0),
                uplink_queue.map_or(0, us),
            ),
            _ => String::new(),
        };
        if !line.is_empty() {
            log(line);
        }
        tokio::time::sleep(SAMPLE_EVERY).await;
    }
}

/// One node's access link in a simulated bench.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BenchSite {
    /// Uplink, bytes per second.
    pub up: u64,
    /// Downlink, bytes per second; `None` keeps the cable profile's.
    pub down: Option<u64>,
    /// The one-way delay its egress adds, as the testbed's netem does.
    pub delay_ms: u64,
    /// The probability it loses a packet it sends or receives, as the
    /// sim's site loss and `run.sh`'s `SEED_LOSS_PCT` do.
    pub loss: f64,
}

/// A bench for [`simulate`].
#[derive(Clone, Debug)]
pub struct Bench {
    pub sites: Vec<BenchSite>,
    pub links: Vec<(usize, usize)>,
    pub schedule: Schedule,
    pub secs: u64,
    /// A node's uplink from given seconds after the start, bytes per
    /// second.
    pub up_steps: Vec<(usize, Vec<(f64, u64)>)>,
    /// Bulk uploads from node 0's uplink, from given seconds after the
    /// start for given seconds, to a sink a round trip `compete_rtt_ms`
    /// away, as `run.sh`'s `COMPETE` runs them. They need the window model.
    pub compete: Vec<(f64, f64)>,
    pub compete_rtt_ms: u64,
    pub send_buffer: SendBuffer,
    pub tcp_segment: Option<usize>,
}

/// The defaults of `hack/testbed/run.sh`.
const RUN_SH: [(&str, &str); 7] = [
    ("VIEWERS", "3"),
    ("SEED_UP_MBIT", "10"),
    ("VIEWER_UP_MBIT", "20"),
    ("QUEUE_MS", "100"),
    ("SEED_DELAY_MS", "20"),
    ("VIEWER_DELAY_MS", "10"),
    ("SECS", "180"),
];

impl Bench {
    /// The bench `hack/testbed/run.sh` runs with `settings` (its
    /// `settings` file: `KEY=value` lines), `topology` (its `topology`
    /// file) and `schedule`, with send buffers whose window model is
    /// `window`.
    pub fn testbed(
        settings: &str,
        topology: &str,
        schedule: &str,
        window: bool,
    ) -> Result<Bench, String> {
        let get = |key: &str| {
            settings
                .lines()
                .find_map(|l| l.strip_prefix(key)?.strip_prefix('='))
                .map(str::to_owned)
                .or_else(|| {
                    RUN_SH
                        .iter()
                        .find(|(k, _)| *k == key)
                        .map(|(_, v)| (*v).to_owned())
                })
        };
        let num = |key: &str| -> Result<u64, String> {
            get(key)
                .unwrap_or_default()
                .parse::<u64>()
                .map_err(|e| format!("{key}: {e}"))
        };
        let mbit = |m: f64| (m * 125_000.0) as u64;
        let viewers = num("VIEWERS")? as usize;
        let queue_ms = num("QUEUE_MS")?;
        let slow = get("SLOW_DOWN_KBIT").map(|k| k.parse::<u64>().map_err(|e| e.to_string()));
        let slow = slow.transpose()?;
        let seed_loss = get("SEED_LOSS_PCT")
            .map(|p| p.parse::<f64>().map_err(|e| format!("SEED_LOSS_PCT: {e}")))
            .transpose()?
            .map_or(0.0, |p| p / 100.0);
        let sites = (0..=viewers)
            .map(|i| {
                let (up, delay) = if i == 0 {
                    (num("SEED_UP_MBIT")?, num("SEED_DELAY_MS")?)
                } else {
                    (num("VIEWER_UP_MBIT")?, num("VIEWER_DELAY_MS")?)
                };
                Ok(BenchSite {
                    up: mbit(up as f64),
                    down: slow.filter(|_| i == 1).map(|k| k * 125),
                    delay_ms: delay,
                    loss: if i == 0 { seed_loss } else { 0.0 },
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let topo = strims_testbed::topology::Topology::parse(topology)?;
        let links = if topo.links.is_empty() {
            (1..=viewers).map(|v| (0, v)).collect()
        } else {
            topo.links.clone()
        };
        let mut compete = Vec::new();
        for flow in get("COMPETE").unwrap_or_default().split_whitespace() {
            let (at, d) = flow
                .split_once(':')
                .ok_or_else(|| format!("COMPETE: {flow}"))?;
            let at: f64 = at.parse().map_err(|e| format!("COMPETE: {e}"))?;
            let d: f64 = d.parse().map_err(|e| format!("COMPETE: {e}"))?;
            compete.push((at, d));
        }
        let mut steps = Vec::new();
        for step in get("SEED_UP_STEPS").unwrap_or_default().split_whitespace() {
            let (at, m) = step
                .split_once(':')
                .ok_or_else(|| format!("SEED_UP_STEPS: {step}"))?;
            let at: f64 = at.parse().map_err(|e| format!("SEED_UP_STEPS: {e}"))?;
            let m: f64 = m.parse().map_err(|e| format!("SEED_UP_STEPS: {e}"))?;
            steps.push((at, mbit(m)));
        }
        Ok(Bench {
            sites,
            links,
            schedule: Schedule::parse(schedule)?,
            secs: num("SECS")?,
            up_steps: if steps.is_empty() {
                Vec::new()
            } else {
                vec![(0, steps)]
            },
            compete,
            // The sink's egress delay is 10 ms.
            compete_rtt_ms: num("SEED_DELAY_MS")? + 10,
            send_buffer: SendBuffer {
                uplink_queue: StdDuration::from_millis(queue_ms),
                window,
            },
            tcp_segment: None,
        })
    }
}

/// When a simulated bench's schedule starts, after its links connect.
const SIM_START: StdDuration = StdDuration::from_secs(2);

#[derive(Clone, Copy, Debug)]
struct SimBenchClock;

impl BenchClock for SimBenchClock {
    fn now_us(&self) -> i64 {
        let t = turmoil::sim_elapsed().unwrap_or_default();
        i64::try_from(t.as_micros()).unwrap_or(i64::MAX)
            - i64::try_from(SIM_START.as_micros()).unwrap_or(0)
    }

    fn time(&self) -> Time {
        sim_time(turmoil::sim_elapsed().unwrap_or_default())
    }
}

/// Runs `bench` on the sim's sockets under the WAN model, with fixed
/// delays and no jitter as the calibration scenarios have, and loss only
/// at sites given some: a pair's paths take the mean of its two egress
/// delays both ways. Returns each node's log lines.
pub fn simulate(bench: &Bench) -> Result<Vec<Vec<String>>, String> {
    let n = bench.sites.len();
    let mut spec = Scenario::new("bench");
    for (i, s) in bench.sites.iter().enumerate() {
        let mut site = Site::new(City::Chicago, Access::Cable)
            .with_up(s.up)
            .with_loss(s.loss);
        if let Some(d) = s.down {
            site = site.with_down(d);
        }
        spec.host_at(&format!("n{i}"), site);
    }
    for (h, steps) in &bench.up_steps {
        if let Some(hs) = spec.hosts.get_mut(*h) {
            hs.up_steps = steps
                .iter()
                .map(|&(at, rate)| (SIM_START + StdDuration::from_secs_f64(at), rate))
                .collect();
        }
    }
    if let Some(seed) = spec.hosts.first_mut() {
        seed.competing = bench
            .compete
            .iter()
            .map(|&(at, secs)| Competing {
                start: SIM_START + StdDuration::from_secs_f64(at),
                stop: SIM_START + StdDuration::from_secs_f64(at + secs),
                rtt: StdDuration::from_millis(bench.compete_rtt_ms),
            })
            .collect();
    }
    spec.send_buffer = Some(bench.send_buffer);
    spec.wan = Some(Options {
        jitter: false,
        loss: false,
    });
    spec.tcp_segment = bench.tcp_segment;
    let segment = spec.tcp_segment;
    let end_us = i64::try_from(bench.secs).unwrap_or(0) * 1_000_000;
    let mut builder = turmoil::Builder::new();
    builder
        .rng_seed(1)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_millis(crate::EPOCH_MS.unsigned_abs()))
        .simulation_duration(SIM_START + StdDuration::from_secs(bench.secs) + LINGER * 2)
        .tick_duration(spec.tick);
    if let Some(seg) = segment {
        builder.tcp_capacity(64 * (64 * 1024usize).div_ceil(seg.max(1)));
    }
    let mut sim = builder.build();
    let buffers = SendBuffers::new(bench.send_buffer);
    let wan = SharedWan::default();
    let logs: Rc<RefCell<Vec<Vec<String>>>> = Rc::new(RefCell::new(vec![Vec::new(); n]));
    for me in 0..n {
        let net = SimNet::new(Some(buffers.clone()))
            .with_wan(wan.clone())
            .with_segment(segment);
        let plan = NodePlan::new(
            me,
            &bench.links,
            "0.0.0.0:9000".to_owned(),
            |i| format!("n{i}:9000"),
            bench.schedule.clone(),
            end_us,
        );
        let logs = logs.clone();
        let log = move |line: String| {
            if let Some(l) = logs.borrow_mut().get_mut(me) {
                l.push(line);
            }
        };
        sim.client(format!("n{me}").as_str(), async move {
            node(net, SimBenchClock, plan, log).await?;
            Ok(())
        });
    }
    let options = Options {
        jitter: false,
        loss: bench.sites.iter().any(|s| s.loss > 0.0),
    };
    let mut sched = Scheduler::new(&sim, &spec, options, 1, Some(buffers))?;
    for &(a, b) in &bench.links {
        let (Some(sa), Some(sb)) = (bench.sites.get(a), bench.sites.get(b)) else {
            return Err(format!("link {a} {b}: no such node"));
        };
        let ms = (sa.delay_ms + sb.delay_ms) as f64 / 2.0;
        sched.set_latency(a, b, Duration::from_micros((ms * 1000.0) as i64));
    }
    *wan.lock() = Some(sched);
    loop {
        if let Some(s) = wan.lock().as_mut() {
            s.walk(&sim)?;
        }
        match sim.step() {
            Ok(true) => break,
            Ok(false) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    drop(sim);
    let report = wan.lock().take().map(|s| s.report(&spec));
    let mut logs = logs.take();
    let start = Duration::from(SIM_START);
    for (h, l) in report.iter().flat_map(|r| &r.hosts).zip(logs.iter_mut()) {
        for &(t, bytes) in &h.competing {
            l.push(format!(
                r#"{{"ev":"x","t":{},"bytes":{bytes}}}"#,
                (t - start).as_micros()
            ));
        }
        for &(t, wait) in &h.up_queue {
            l.push(format!(
                r#"{{"ev":"q","t":{},"wait":{}}}"#,
                (t - start).as_micros(),
                wait.as_micros()
            ));
        }
    }
    Ok(logs)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn schedules_take_bursts_and_probes() {
        let s = Schedule::parse(
            "# a comment\nburst 0 1 0 250 65536 16384 # to v1\nprobe 0 2 3 10 900\n\n",
        )
        .unwrap();
        assert_eq!(
            s.entries,
            vec![
                Entry::Burst {
                    from: 0,
                    to: 1,
                    start_ms: 0,
                    period_ms: 250,
                    bytes: 65536,
                    write: 16384,
                    end_ms: None,
                },
                Entry::Probe {
                    from: 0,
                    to: 2,
                    start_ms: 3,
                    period_ms: 10,
                    end_ms: Some(900),
                },
            ]
        );
        assert!(Schedule::parse("burst 0 1 0 250 65536 8").is_err());
        assert!(Schedule::parse("pause 0 1").is_err());
    }
}
