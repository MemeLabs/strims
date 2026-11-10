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
//!   in bytes, `upq` the uplink queue in µs); over a data channel its
//!   `buffered` amount, and the window model's channel adds its
//!   association's `cwnd` and `in_flight` and the `upq`.
//!
//! In the sim a WebRTC bench's links are the window model's data channels
//! (see `wan`) on the links' TCP connections, with no signalling,
//! or with [`Bench::str0m`] str0m's stack on the sim's UDP sockets.
//!
//! [`simulate`] adds a `{"ev":"q","t":..,"wait":..}` line per sample of
//! the node's uplink queue (µs) every 7 ms, as the testbed's `tc-0.log`
//! samples the seed's, a `{"ev":"x","t":..,"bytes":..,"cwnd":..}` line per sample of
//! the wire bytes its competing uploads sent and their windows (as
//! `run.sh`'s `COMPETE` runs them, on the testbed in `compete-<s>.json`),
//! and a `{"ev":"drops",..}` line with the segments the window model lost
//! to its full access queues, the connections' and the uploads', the pure
//! acknowledgements and SACKs the node's uplink carried (`acks`), and its
//! TCP receivers' acknowledgements by why (`ack_now`, `ack_quick`,
//! `ack_read`, `ack_timer`, `ack_data`: [`crate::ACK_WHYS`]) with the
//! delayed ones' mean and p90 wait (`delack_mean_ms`, `delack_p90_ms`).

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

use crate::net::{SendBuffers, SimNet, SimTcpStream};
use crate::spec::{Competing, Scenario, SendBuffer};
use crate::wan::{Scheduler, SctpProfile, SharedWan, sim_time};

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

pub mod meter;
mod rtc;

pub use rtc::{RtcPlan, read_signal, write_signal};

/// What one node does: where it listens, whom it dials, and the schedule.
#[derive(Clone, Debug)]
pub struct NodePlan {
    pub me: usize,
    pub listen: String,
    /// With an endpoint, the links carry their records over WebRTC data
    /// channels, signalled over their TCP connections.
    pub webrtc: Option<RtcPlan>,
    /// The links carry their records over the sim's modelled data channels
    /// ([`BenchNet::probed_channel`]), on their TCP connections, sent as
    /// `profile`'s stack sends.
    pub channels: bool,
    pub profile: SctpProfile,
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
            webrtc: None,
            channels: false,
            profile: SctpProfile::STR0M,
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

/// Makes node `plan.me`'s TCP links: dials its dials until they connect
/// (or fail after the start) and accepts the rest. Returns them by peer.
pub async fn links<N: Net, C: BenchClock>(
    net: &N,
    clock: &C,
    plan: &NodePlan,
) -> io::Result<Vec<(usize, N::TcpStream)>> {
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
            setup(net, &s);
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
    Ok(streams)
}

/// A [`Net`] whose TCP connections may carry a modelled data channel.
pub trait BenchNet: Net {
    /// Carries `s`'s writes as a data channel buffering `cap` bytes at
    /// most, sent as `profile`'s SCTP sends: the probe of its buffered
    /// amount and window, or `None` where the net models no channels.
    fn probed_channel(
        &self,
        _s: &Self::TcpStream,
        _cap: usize,
        _profile: SctpProfile,
    ) -> Option<ChannelProbe> {
        None
    }
}

impl BenchNet for strims_node::TokioNet {}

impl BenchNet for SimNet {
    fn probed_channel(
        &self,
        s: &SimTcpStream,
        cap: usize,
        profile: SctpProfile,
    ) -> Option<ChannelProbe> {
        let conn = self.open_channel_as(s, cap as u64, profile, false)?;
        Some(ChannelProbe {
            conn,
            buffers: self.buffers()?,
            wan: self.wan()?,
        })
    }
}

/// A modelled data channel's view: its buffered amount, and its
/// association's window, bytes in flight and sender's uplink queue.
pub struct ChannelProbe {
    conn: crate::net::ConnKey,
    buffers: crate::net::SendBuffers,
    wan: SharedWan,
}

impl ChannelProbe {
    fn sample(&self, now: Time) -> (u64, crate::wan::ConnState) {
        let buffered = self.buffers.buffered(self.conn).unwrap_or(0);
        let state = self
            .wan
            .lock()
            .as_ref()
            .map(|s| s.conn_state(self.conn, now))
            .unwrap_or_default();
        (buffered, state)
    }
}

/// Runs node `plan.me` on `net` until [`LINGER`] past its schedule's end,
/// logging to `log` (see the module docs).
pub async fn node<N: BenchNet, C: BenchClock>(
    net: N,
    clock: C,
    plan: NodePlan,
    log: impl Fn(String) + Clone + 'static,
) -> io::Result<()> {
    let me = plan.me;
    let streams = links(&net, &clock, &plan).await?;
    let stop = plan.end_us + i64::try_from(LINGER.as_micros()).unwrap_or(i64::MAX);

    let (endpoint, driver): (Option<rtc::Shared>, Task) = match &plan.webrtc {
        Some(p) => {
            let (shared, driver) = rtc::start(&net, p, me, &clock).await?;
            (Some(shared), driver)
        }
        None => (None, Box::pin(std::future::pending())),
    };
    let run = async {
        let mut tasks: Vec<Task> = Vec::new();
        let mut links: Vec<Rc<LinkState>> = Vec::new();
        for (peer, mut s) in streams {
            let link = Rc::new(LinkState {
                peer,
                out: RefCell::new(Outbox::default()),
                ready: Notify::new(),
                data_seq: Cell::new(0),
                probe_seq: Cell::new(0),
            });
            match &endpoint {
                Some(shared) => {
                    let dc = rtc::connect(shared, &mut s, peer < me, &clock).await?;
                    tasks.push(Box::pin(writer(
                        dc.handle(),
                        link.clone(),
                        clock.clone(),
                        log.clone(),
                    )));
                    tasks.push(Box::pin(rtc_sampler(
                        dc.handle(),
                        peer,
                        clock.clone(),
                        log.clone(),
                    )));
                    tasks.push(Box::pin(reader(
                        dc,
                        link.clone(),
                        clock.clone(),
                        log.clone(),
                    )));
                }
                None if plan.channels => {
                    let cap = strims_node::WebRtcConfig::default().max_buffered_amount;
                    let probe = net
                        .probed_channel(&s, cap, plan.profile)
                        .ok_or_else(|| io::Error::other("this net models no data channels"))?;
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
                    tasks.push(Box::pin(channel_sampler(
                        probe,
                        peer,
                        clock.clone(),
                        log.clone(),
                    )));
                }
                None => {
                    let probe = net.link_probe(&s, false);
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
                }
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
        join_all(tasks).await;
        Ok::<_, io::Error>(())
    };
    tokio::select! {
        biased;
        r = run => r?,
        () = driver => {}
        () = sleep_until(&clock, stop) => {}
    }
    Ok(())
}

/// Logs a modelled data channel's buffered amount every [`SAMPLE_EVERY`],
/// with its association's window and bytes in flight and its uplink queue
/// (µs).
async fn channel_sampler<C: BenchClock>(
    probe: ChannelProbe,
    peer: usize,
    clock: C,
    log: impl Fn(String),
) {
    loop {
        let (buffered, s) = probe.sample(clock.time());
        log(format!(
            r#"{{"ev":"s","to":{peer},"t":{},"buffered":{buffered},"cwnd":{},"in_flight":{},"upq":{}}}"#,
            clock.now_us(),
            s.cwnd.unwrap_or(0),
            s.in_flight.unwrap_or(0),
            s.uplink_queue.map_or(0, |d| d.as_micros()),
        ));
        tokio::time::sleep(SAMPLE_EVERY).await;
    }
}

/// Logs a data channel's buffered amount every [`SAMPLE_EVERY`].
async fn rtc_sampler<C: BenchClock>(
    dc: rtc::DcStream,
    peer: usize,
    clock: C,
    log: impl Fn(String),
) {
    loop {
        log(format!(
            r#"{{"ev":"s","to":{peer},"t":{},"buffered":{}}}"#,
            clock.now_us(),
            dc.buffered()
        ));
        tokio::time::sleep(SAMPLE_EVERY).await;
    }
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
    /// The links carry their records over WebRTC data channels (the
    /// nodes ran with `--webrtc`, in `run.sh`'s `NODE_ARGS`): the window
    /// model's data channels, or with `str0m` str0m's stack on the sim's
    /// UDP sockets.
    pub webrtc: bool,
    pub str0m: bool,
    /// Each node's SCTP stack in the window model's data channels: its
    /// kind's in the topology (the browsers' dcSCTP, or str0m's for a
    /// native node).
    pub stacks: Vec<SctpProfile>,
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
            webrtc: get("NODE_ARGS").is_some_and(|a| a.split_whitespace().any(|w| w == "--webrtc")),
            str0m: false,
            stacks: (0..=viewers)
                .map(|i| match topo.node(i).map(|n| n.kind) {
                    Some(strims_testbed::topology::Kind::Chromium) => SctpProfile::CHROMIUM,
                    Some(strims_testbed::topology::Kind::Firefox) => SctpProfile::FIREFOX,
                    _ => SctpProfile::STR0M,
                })
                .collect(),
            send_buffer: SendBuffer {
                uplink_queue: StdDuration::from_millis(queue_ms),
                window,
            },
            tcp_segment: None,
        })
    }
}

/// How often a simulated bench samples its nodes' uplink queues: a prime
/// number of milliseconds, so a schedule's period (whole milliseconds, a
/// multiple of 100 ms's samples) sees the queue at every phase, as the
/// testbed's `tc` samples, whose loop drifts, do.
const QUEUE_EVERY: StdDuration = StdDuration::from_millis(7);

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
    let segment = spec.segment();
    let end_us = i64::try_from(bench.secs).unwrap_or(0) * 1_000_000;
    // str0m draws ICE credentials and SCTP details from a thread-local rng.
    strims_node::seed_str0m_thread_rng(1);
    let mut builder = turmoil::Builder::new();
    builder
        .rng_seed(1)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_millis(crate::EPOCH_MS.unsigned_abs()))
        .simulation_duration(SIM_START + StdDuration::from_secs(bench.secs) + LINGER * 2)
        .tick_duration(spec.tick);
    let channels = bench.webrtc && !bench.str0m;
    if channels && !bench.send_buffer.window {
        return Err("modelled data channels need the window model".to_owned());
    }
    if let Some(seg) = segment {
        builder.tcp_capacity(64 * (64 * 1024usize).div_ceil(seg.max(1)));
    }
    if channels {
        // A channel's chunks, as many as its cap and the reader's buffer
        // hold.
        let least = bench.stacks.iter().map(|p| p.max_payload).min();
        let chunk = least.unwrap_or(SctpProfile::STR0M.max_payload);
        builder.tcp_capacity(64 * (64 * 1024usize).div_ceil(chunk));
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
        let webrtc = bench.webrtc && bench.str0m;
        let profile = bench.stacks.get(me).copied().unwrap_or(SctpProfile::STR0M);
        sim.client(format!("n{me}").as_str(), async move {
            let mut plan = plan;
            plan.channels = channels;
            plan.profile = profile;
            if webrtc {
                plan.webrtc = Some(RtcPlan {
                    udp: "0.0.0.0:9001".to_owned(),
                    ip: turmoil::lookup(format!("n{me}")),
                });
            }
            node(net, SimBenchClock, plan, log).await?;
            Ok(())
        });
    }
    let options = Options {
        jitter: false,
        loss: bench.sites.iter().any(|s| s.loss > 0.0),
    };
    let mut sched = Scheduler::new(&sim, &spec, options, 1, Some(buffers))?;
    sched.sample_queues_every(Duration::from(QUEUE_EVERY));
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
        let whys: Vec<String> = crate::ACK_WHYS
            .iter()
            .zip(h.ack_whys)
            .map(|(why, n)| format!(r#","ack_{why}":{n}"#))
            .collect();
        let waits = &h.ack_waits;
        l.push(format!(
            r#"{{"ev":"drops","up_packets":{},"queue_drops":{},"competing_drops":{},"acks":{}{},"delack_mean_ms":{},"delack_p90_ms":{}}}"#,
            h.stats.up_bytes / 1500,
            h.queue_drops,
            h.competing_drops,
            h.acks,
            whys.concat(),
            waits.mean_ms().map_or("null".to_owned(), |m| format!("{m:.2}")),
            waits
                .quantile_ms(0.9)
                .map_or("null".to_owned(), |p| p.to_string()),
        ));
        for &(t, bytes) in &h.competing {
            let cwnd = h
                .competing_cwnd
                .iter()
                .find(|&&(at, _)| at == t)
                .map_or(0, |&(_, w)| w);
            l.push(format!(
                r#"{{"ev":"x","t":{},"bytes":{bytes},"cwnd":{cwnd}}}"#,
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
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    /// `key`'s integer in a log line.
    fn field(line: &str, key: &str) -> Option<i64> {
        let at = line.find(&format!("\"{key}\":"))? + key.len() + 3;
        let rest = line.get(at..)?;
        let end = rest
            .find(|c: char| c != '-' && !c.is_ascii_digit())
            .unwrap_or(rest.len());
        rest.get(..end)?.parse().ok()
    }

    /// `xs`' value at quantile `q`.
    fn quantile(mut xs: Vec<i64>, q: f64) -> i64 {
        xs.sort_unstable();
        xs[((xs.len() as f64 * q) as usize).min(xs.len() - 1)]
    }

    /// `schedule` on the testbed's settings `settings`, with the window
    /// model.
    fn run(settings: &str, schedule: &str) -> Vec<Vec<String>> {
        let bench = Bench::testbed(settings, "", schedule, true).unwrap();
        simulate(&bench).unwrap()
    }

    fn run_str0m(settings: &str, schedule: &str) -> Vec<Vec<String>> {
        let mut bench = Bench::testbed(settings, "", schedule, true).unwrap();
        bench.str0m = true;
        simulate(&bench).unwrap()
    }

    /// The lines of `log` of event `ev` from 2 s on (by `t`, or `at` for
    /// records).
    fn events<'a>(log: &'a [String], ev: &str) -> impl Iterator<Item = &'a String> {
        let tag = format!("\"ev\":\"{ev}\"");
        log.iter().filter(move |l| {
            l.contains(&tag)
                && field(l, "t")
                    .or(field(l, "at"))
                    .is_some_and(|t| t >= 2_000_000)
        })
    }

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

    /// On the testbed (`hack/testbed/bench/over.sched`) a socket asked for
    /// more than its 5 Mbit/s uplink takes drains its unsent bytes to
    /// under half the mark before the writer may write again, then takes
    /// a GSO buffer: 14 KiB unsent at p10 and 61 at p90, and its writes
    /// wait 101 ms at p90.
    #[test]
    fn a_full_socket_drains_to_half_its_mark_before_it_refills() {
        let logs = run(
            "VIEWERS=1\nSEED_UP_MBIT=5\nSECS=10",
            "burst 0 1 0 250 163840 16384\nprobe 0 1 3 10",
        );
        let unsent: Vec<i64> = events(&logs[0], "s")
            .filter_map(|l| field(l, "unsent"))
            .collect();
        let (lo, hi) = (quantile(unsent.clone(), 0.1), quantile(unsent, 0.9));
        assert!(lo < 16 * 1024 && hi > 48 * 1024, "unsent p10 {lo} p90 {hi}");
        let waits: Vec<i64> = events(&logs[0], "w")
            .filter(|l| field(l, "k") == Some(0))
            .map(|l| field(l, "done").unwrap() - field(l, "at").unwrap())
            .collect();
        let wait = quantile(waits, 0.9);
        assert!(
            (60_000..140_000).contains(&wait),
            "writes wait {wait} µs at p90"
        );
    }

    /// Over the window model's data channel
    /// (`hack/testbed/bench/rtc-over.sched`) a link asked for 105 % of a
    /// 5 Mbit/s uplink takes about the uplink, every
    /// record arrives in order, and the channel holds up to the node's
    /// 256 KiB cap and refills from half of it: on the testbed 554 KB/s
    /// read, 142-272 KiB buffered at p10/max.
    #[test]
    fn a_data_channel_takes_its_uplink_and_holds_the_nodes_cap() {
        takes_its_uplink_and_holds_the_cap(&run(
            "VIEWERS=1\nSEED_UP_MBIT=5\nSECS=10\nNODE_ARGS=--webrtc",
            "burst 0 1 0 250 163840 16384\nprobe 0 1 3 10",
        ));
    }

    /// The same over str0m's stack on the sim's UDP sockets.
    #[test]
    fn str0m_takes_its_uplink_and_holds_the_nodes_cap() {
        takes_its_uplink_and_holds_the_cap(&run_str0m(
            "VIEWERS=1\nSEED_UP_MBIT=5\nSECS=10\nNODE_ARGS=--webrtc",
            "burst 0 1 0 250 163840 16384\nprobe 0 1 3 10",
        ));
    }

    fn takes_its_uplink_and_holds_the_cap(logs: &[Vec<String>]) {
        let reads: Vec<i64> = events(&logs[1], "r")
            .filter(|l| field(l, "k") == Some(0))
            .filter_map(|l| field(l, "seq"))
            .collect();
        assert!(
            reads.windows(2).all(|w| w[1] == w[0] + 1),
            "records out of order"
        );
        // 16 KiB records read over the 8 s from 2 s on.
        let read = events(&logs[1], "r")
            .filter(|l| field(l, "k") == Some(0))
            .filter(|l| field(l, "got").is_some_and(|t| (2_000_000..10_000_000).contains(&t)))
            .count();
        let rate = read as i64 * 16_384 / 8;
        assert!((480_000..640_000).contains(&rate), "{rate} B/s read");
        let buffered: Vec<i64> = events(&logs[0], "s")
            .filter_map(|l| field(l, "buffered"))
            .collect();
        let (lo, hi) = (quantile(buffered.clone(), 0.1), quantile(buffered, 1.0));
        assert!(
            lo > 64 * 1024 && hi <= 280 * 1024,
            "buffered p10 {lo} max {hi}"
        );
        let echoes = events(&logs[0], "r")
            .filter(|l| field(l, "k") == Some(2))
            .count();
        assert!(echoes > 700, "{echoes} echoes");
    }

    /// Over the window model's data channel
    /// (`hack/testbed/bench/rtc-single.sched`) one link at a fifth of a
    /// 10 Mbit/s uplink sends each 64 KiB burst within its window, so the
    /// uplink queue holds the burst, and its last record arrives a burst's
    /// transmission and a one-way trip after it was written: on the
    /// testbed the queue peaked at 55 ms, and records arrived 76 ms after
    /// they were written at p90.
    #[test]
    fn a_data_channel_sends_a_burst_within_its_window() {
        let logs = run(
            "VIEWERS=1\nSEED_UP_MBIT=10\nSECS=10\nNODE_ARGS=--webrtc",
            "burst 0 1 0 250 65536 16384\nprobe 0 1 3 10",
        );
        let queue: Vec<i64> = events(&logs[0], "q")
            .filter_map(|l| field(l, "wait"))
            .collect();
        let peak = quantile(queue, 1.0);
        assert!(
            (45_000..60_000).contains(&peak),
            "queue peaked at {peak} us"
        );
        let owd: Vec<i64> = events(&logs[1], "r")
            .filter(|l| field(l, "k") == Some(0))
            .filter_map(|l| Some(field(l, "got")? - field(l, "at")?))
            .collect();
        let p90 = quantile(owd, 0.9);
        assert!((65_000..85_000).contains(&p90), "one-way p90 {p90} us");
    }

    /// The same link from a Chromium node
    /// (`hack/testbed/bench/rtc-single-chromium.sched`): dcSCTP sends four
    /// packets a send opportunity, so its burst waits in its own queue more
    /// than in the uplink's, as on the testbed (the queue peaked at 17-18
    /// ms where str0m's took 55), and arrives as soon.
    #[test]
    fn a_chromium_channel_holds_its_burst() {
        let topology = "node 0 10.77.0.10:4000 chromium\nnode 1 10.77.0.11:4000\nlink 0 1\n";
        let bench = Bench::testbed(
            "VIEWERS=1\nSEED_UP_MBIT=10\nSECS=10\nNODE_ARGS=--webrtc",
            topology,
            "burst 0 1 0 250 65536 16384\nprobe 0 1 3 10",
            true,
        )
        .unwrap();
        let logs = simulate(&bench).unwrap();
        let queue: Vec<i64> = events(&logs[0], "q")
            .filter_map(|l| field(l, "wait"))
            .collect();
        let peak = quantile(queue, 1.0);
        assert!(
            (10_000..25_000).contains(&peak),
            "queue peaked at {peak} us"
        );
        let owd: Vec<i64> = events(&logs[1], "r")
            .filter(|l| field(l, "k") == Some(0))
            .filter_map(|l| Some(field(l, "got")? - field(l, "at")?))
            .collect();
        let p90 = quantile(owd, 0.9);
        assert!((65_000..90_000).contains(&p90), "one-way p90 {p90} us");
    }

    /// The testbed's seed shape (`hack/testbed/bench/holders.sched`): two
    /// holders take 64 KiB bursts every 250 ms on a 5 Mbit/s uplink, eight
    /// idle links a 128-byte write after each burst starts, and every link
    /// a probe every 200 ms. The holder
    /// written first gets its burst into the uplink's queue ahead of the
    /// other's, which meets the queue full: their windows sat at 53 and 16
    /// KiB at p50. The idle links' small writes fit in the queue's last
    /// room, and arrived 114 ms after they were written at p50, 121 at
    /// p90.
    #[test]
    fn the_socket_written_first_takes_the_queue_and_small_writes_fit() {
        let mut schedule = String::from("burst 0 1 0 250 65536 4096\nburst 0 2 0 250 65536 4096\n");
        for v in 3..=10 {
            schedule += &format!("burst 0 {v} 1 250 128 128\n");
        }
        for (v, phase) in (1..=10).zip([7, 47, 17, 37, 57, 77, 97, 117, 137, 157]) {
            schedule += &format!("probe 0 {v} {phase} 200\n");
        }
        let logs = run("VIEWERS=10\nSEED_UP_MBIT=5\nSECS=30", &schedule);
        let cwnd = |to: i64| {
            let xs = events(&logs[0], "s")
                .filter(|l| field(l, "to") == Some(to))
                .filter_map(|l| field(l, "cwnd"))
                .collect();
            quantile(xs, 0.5)
        };
        let (first, second) = (cwnd(1), cwnd(2));
        assert!(
            first > 2 * second,
            "the holders' windows {first} and {second} bytes at p50"
        );
        let owd: Vec<i64> = logs[3..]
            .iter()
            .flat_map(|log| events(log, "r"))
            .filter(|l| field(l, "k") == Some(0))
            .map(|l| field(l, "got").unwrap() - field(l, "at").unwrap())
            .collect();
        let (p50, p90) = (quantile(owd.clone(), 0.5), quantile(owd, 0.9));
        assert!(
            (95_000..130_000).contains(&p50) && p90 < 140_000,
            "idle links' writes take {p50} µs at p50, {p90} at p90"
        );
    }
}
