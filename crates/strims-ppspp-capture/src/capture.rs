// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A capture of one [`Runner`]: its frames, through the driver, and its
//! tap, drained into tap records.
//!
//! The driver brackets each runner call that a frame record describes with
//! drains, so a call's tap records follow its frame:
//! [`Capture::handle_frame`] and [`Capture::poll_transmit`] do it for the
//! two frame calls. After any other call that changes the runner (adding
//! or removing peers and swarms, timeouts, writes), the driver calls
//! [`Capture::drain`] before the next frame.
//!
//! Tap kinds and their fields, in order (bins are bin numbers, times
//! nanoseconds since the Unix epoch, durations nanoseconds, rates bytes
//! per second, peers dump ids, 0 for none; -1 is unknown):
//!
//! | kind | name | fields |
//! |---|---|---|
//! | 1 | `DataSent` | bin, push (1) or answer (0), queued at |
//! | 2 | `DataRefused` | bin, push |
//! | 3 | `DataDropped` | bin, push |
//! | 4 | `PushStale` | bin, queued at |
//! | 5 | `PushSuppressed` | bin |
//! | 6 | `DataStale` | bin |
//! | 7 | `PushHit` | bin, lag |
//! | 8 | `PushLate` | bin, lag |
//! | 9 | `PushUnsolicited` | bin |
//! | 10 | `PushMiss` | bin, deadline (time), first_seen (time), counted, lag_mean, lag_dev, lag_peak, lag_deadline |
//! | 11 | `RequestTimeout` | bin, requested (time), timeout |
//! | 12 | `Pruned` | stream, rule (`silent`, `lossy`, `both`), silent_misses, since_hit, miss_rate (millionths), expected, pushes, age, lag_mean, lag_dev, lag_peak, lag_deadline |
//! | 13 | `Switched` | stream, from (peer) |
//! | 14 | `OpenTimeout` | stream, waited |
//! | 15 | `Declined` | stream |
//! | 16 | `ShedBy` | stream |
//! | 17 | `Admitted` | stream, rule (`subscribed`, `pin`, `seed_holder`, `seed_barred`, `seed_no_copy`, `seed_streams_held`, `unmeasured`, `measured`), extra, upload_rate, committed, others, stream_rate, capacity, target (thousandths), stream_children, holders, copies, held, per_holder |
//! | 18 | `Rejected` | as `Admitted` |
//! | 19 | `Shed` | stream, cause (`saturated`, `seed_budget`), busy (thousandths), committed, others, capacity, excess, count, room |
//! | 20 | `HolderJoined` | |
//! | 21 | `HolderLeft` | why (`silent`, `unserved`, `past_copies`, `idle`, `closed`), copies, rtt |
//! | 22 | `SeedBudget` | budget, copies, streams, capacity, stream_rate, others |
//! | 23 | `SeedWindow` | copies, pushed, behind |
//! | 24 | `HolderSilent` | |
//! | 25 | `HolderUnserved` | reporters |
//! | 26 | `PushCancelled` | bin |
//! | 27 | `ParentChoice` | stream, action (`graft`, `none`, `vote`, `switch`, `reset`), chosen (peer), chosen_lag, runner_up (peer), runner_up_lag, unready, subscriber, backed_off, stale, votes, parent_have, parent_push |
//! | 28 | `RequestRound` | floor (bin), cap, in_flight, room, fit, candidates, requested |
//! | 29 | `Upload` | rate, busy (thousandths), capacity, hint, others, committed, queue_delay |
//! | 30 | `SeedFirstWave` | capacity, early_rate, others, neighbours, copies |
//! | 31 | `AnswerRate` | rate, reading |
//!
//! Version 1 dumps hold only the fields of kinds 1 to 26 up to `Switched`'s
//! `from`, `SeedBudget`'s `streams` and the first of the others
//! ([`KindInfo::v1_fields`]).
//!
//! [`strims_ppspp::TapKind`] documents what each means.

use std::collections::BTreeMap;
use std::io::{self, Write};

use strims_base::time::{Duration, Time};
use strims_binmap::Bin;
use strims_ppspp::codec::Stream;
use strims_ppspp::{
    Admission, AdmitRule, HolderLeft, LagEstimate, ParentAction, PeerKey, PushMode,
    RequestControllerKind, Runner, SeedAllocation, SeedQos, Shed, Swarm, SwarmKey, TapEvent,
    TapKind, TapLevel,
};

use crate::format::{Clock, Dir, Header, Record, SampleRecord, SwarmParams, TapRecord};
use crate::sample::Sample;
use crate::writer::Writer;

/// How a field reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    /// A number, in the unit its table gives.
    Num,
    /// A peer's id in the dump; an export names the peer.
    Peer,
    /// A time, ns since the Unix epoch; 0 is unknown.
    Time,
    /// The index of one of these names.
    Enum(&'static [&'static str]),
}

/// A field's name and how it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: &'static str,
    pub kind: FieldKind,
}

/// A tap kind's or a sample source's code, name and fields.
#[derive(Clone, Copy, Debug)]
pub struct KindInfo {
    pub code: u32,
    pub name: &'static str,
    pub fields: &'static [Field],
    /// The fields the kind had in a version 1 dump. Version 1 writers
    /// padded every tap record to 3 fields with zeros, so a reader takes
    /// only these.
    pub v1_fields: usize,
}

impl KindInfo {
    pub(crate) const fn new(code: u32, name: &'static str, fields: &'static [Field]) -> KindInfo {
        KindInfo {
            code,
            name,
            fields,
            v1_fields: fields.len(),
        }
    }

    /// A kind that has more fields than it had in version 1.
    const fn since_v1(self, v1_fields: usize) -> KindInfo {
        KindInfo { v1_fields, ..self }
    }

    /// How many of a record's `n` fields a reader of a version `version`
    /// dump takes.
    pub fn fields_of(&self, version: u16, n: usize) -> usize {
        let n = n.min(self.fields.len());
        if version == 1 {
            n.min(self.v1_fields)
        } else {
            n
        }
    }

    /// Field `i`'s name of `v`, if it is an [`FieldKind::Enum`] that names
    /// it.
    pub fn value_name(&self, i: usize, v: i64) -> Option<&'static str> {
        match self.fields.get(i)?.kind {
            FieldKind::Enum(names) => names.get(usize::try_from(v).ok()?).copied(),
            _ => None,
        }
    }
}

pub(crate) const fn num(name: &'static str) -> Field {
    Field {
        name,
        kind: FieldKind::Num,
    }
}

const fn peer(name: &'static str) -> Field {
    Field {
        name,
        kind: FieldKind::Peer,
    }
}

const fn time(name: &'static str) -> Field {
    Field {
        name,
        kind: FieldKind::Time,
    }
}

const fn named(name: &'static str, names: &'static [&'static str]) -> Field {
    Field {
        name,
        kind: FieldKind::Enum(names),
    }
}

const ADMIT_RULES: &[&str] = &[
    "subscribed",
    "pin",
    "seed_holder",
    "seed_barred",
    "seed_no_copy",
    "seed_streams_held",
    "unmeasured",
    "measured",
];
const ADMISSION: &[Field] = &[
    num("stream"),
    named("rule", ADMIT_RULES),
    num("extra"),
    num("upload_rate"),
    num("committed"),
    num("others"),
    num("stream_rate"),
    num("capacity"),
    num("target"),
    num("stream_children"),
    num("holders"),
    num("copies"),
    num("held"),
    num("per_holder"),
];
const LAG: [Field; 4] = [
    num("lag_mean"),
    num("lag_dev"),
    num("lag_peak"),
    num("lag_deadline"),
];

/// The tap kinds this crate writes.
pub const TAP_KINDS: &[KindInfo] = &[
    KindInfo::new(1, "DataSent", &[num("bin"), num("push"), num("queued")]),
    KindInfo::new(2, "DataRefused", &[num("bin"), num("push")]),
    KindInfo::new(3, "DataDropped", &[num("bin"), num("push")]),
    KindInfo::new(4, "PushStale", &[num("bin"), num("queued")]),
    KindInfo::new(5, "PushSuppressed", &[num("bin")]),
    KindInfo::new(6, "DataStale", &[num("bin")]),
    KindInfo::new(7, "PushHit", &[num("bin"), num("lag")]),
    KindInfo::new(8, "PushLate", &[num("bin"), num("lag")]),
    KindInfo::new(9, "PushUnsolicited", &[num("bin")]),
    KindInfo::new(
        10,
        "PushMiss",
        &[
            num("bin"),
            time("deadline"),
            time("first_seen"),
            num("counted"),
            LAG[0],
            LAG[1],
            LAG[2],
            LAG[3],
        ],
    )
    .since_v1(1),
    KindInfo::new(
        11,
        "RequestTimeout",
        &[num("bin"), time("requested"), num("timeout")],
    )
    .since_v1(1),
    KindInfo::new(
        12,
        "Pruned",
        &[
            num("stream"),
            named("rule", &["silent", "lossy", "both"]),
            num("silent_misses"),
            num("since_hit"),
            num("miss_rate"),
            num("expected"),
            num("pushes"),
            num("age"),
            LAG[0],
            LAG[1],
            LAG[2],
            LAG[3],
        ],
    )
    .since_v1(1),
    KindInfo::new(13, "Switched", &[num("stream"), peer("from")]),
    KindInfo::new(14, "OpenTimeout", &[num("stream"), num("waited")]).since_v1(1),
    KindInfo::new(15, "Declined", &[num("stream")]),
    KindInfo::new(16, "ShedBy", &[num("stream")]),
    KindInfo::new(17, "Admitted", ADMISSION).since_v1(1),
    KindInfo::new(18, "Rejected", ADMISSION).since_v1(1),
    KindInfo::new(
        19,
        "Shed",
        &[
            num("stream"),
            named("cause", &["saturated", "seed_budget"]),
            num("busy"),
            num("committed"),
            num("others"),
            num("capacity"),
            num("excess"),
            num("count"),
            num("room"),
        ],
    )
    .since_v1(1),
    KindInfo::new(20, "HolderJoined", &[]),
    KindInfo::new(
        21,
        "HolderLeft",
        &[
            named(
                "why",
                &["silent", "unserved", "past_copies", "idle", "closed"],
            ),
            num("copies"),
            num("rtt"),
        ],
    )
    .since_v1(0),
    KindInfo::new(
        22,
        "SeedBudget",
        &[
            num("budget"),
            num("copies"),
            num("streams"),
            num("capacity"),
            num("stream_rate"),
            num("others"),
        ],
    )
    .since_v1(3),
    KindInfo::new(
        23,
        "SeedWindow",
        &[num("copies"), num("pushed"), num("behind")],
    ),
    KindInfo::new(24, "HolderSilent", &[]),
    KindInfo::new(25, "HolderUnserved", &[num("reporters")]),
    KindInfo::new(26, "PushCancelled", &[num("bin")]),
    KindInfo::new(
        27,
        "ParentChoice",
        &[
            num("stream"),
            named("action", &["graft", "none", "vote", "switch", "reset"]),
            peer("chosen"),
            num("chosen_lag"),
            peer("runner_up"),
            num("runner_up_lag"),
            num("unready"),
            num("subscriber"),
            num("backed_off"),
            num("stale"),
            num("votes"),
            num("parent_have"),
            num("parent_push"),
        ],
    ),
    KindInfo::new(
        28,
        "RequestRound",
        &[
            num("floor"),
            num("cap"),
            num("in_flight"),
            num("room"),
            num("fit"),
            num("candidates"),
            num("requested"),
        ],
    ),
    KindInfo::new(
        29,
        "Upload",
        &[
            num("rate"),
            num("busy"),
            num("capacity"),
            num("hint"),
            num("others"),
            num("committed"),
            num("queue_delay"),
        ],
    ),
    KindInfo::new(
        30,
        "SeedFirstWave",
        &[
            num("capacity"),
            num("early_rate"),
            num("others"),
            num("neighbours"),
            num("copies"),
        ],
    ),
    KindInfo::new(31, "AnswerRate", &[num("rate"), num("reading")]),
];

/// The kind with `code`.
pub fn tap_kind(code: u32) -> Option<&'static KindInfo> {
    TAP_KINDS.iter().find(|k| k.code == code)
}

/// Where a capture's files go. File `index` counts from 0; opening one may
/// retire older files, which bounds what a rotated capture keeps.
pub trait Sink {
    type File: Write;
    fn open(&mut self, index: u64) -> io::Result<Self::File>;
}

impl<F, W> Sink for F
where
    F: FnMut(u64) -> io::Result<W>,
    W: Write,
{
    type File = W;
    fn open(&mut self, index: u64) -> io::Result<W> {
        self(index)
    }
}

/// A capture's settings.
#[derive(Clone, Copy, Debug)]
pub struct Options<'a> {
    pub clock: Clock,
    /// The node's name, kept to [`MAX_NAME`](crate::MAX_NAME) bytes.
    pub node: &'a [u8],
    pub label: &'a str,
    /// Start another file once one passes this size; 0 never does.
    pub max_file_bytes: u64,
    /// Capture the runner's decisions; without them a capture holds the
    /// frames and the lifecycle.
    pub tap: bool,
}

/// What a capture has written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub frames_in: u64,
    pub bytes_in: u64,
    pub frames_out: u64,
    pub bytes_out: u64,
    pub tap_records: u64,
    pub samples: u64,
    pub files: u64,
    /// Bytes written to every file.
    pub written: u64,
}

/// A capture of one runner into a sink's files. After a write fails it
/// writes nothing more, and [`Capture::error`] says why.
pub struct Capture<S: Sink> {
    sink: S,
    writer: Option<Writer<S::File>>,
    clock: Clock,
    node: Vec<u8>,
    label: String,
    max_file_bytes: u64,
    tap: bool,
    /// Bound: the runner's peers (a removed peer leaves).
    peers: BTreeMap<PeerKey, u32>,
    next_peer: u32,
    /// Bound: the runner's swarms.
    swarms: BTreeMap<SwarmKey, u32>,
    next_swarm: u32,
    /// Scratch for a drain. Bound: what one drain yields.
    pending: Vec<TapEvent>,
    stats: Stats,
    /// The bytes of the files before the current one.
    closed: u64,
    error: Option<io::Error>,
    tap_stopped: bool,
}

impl<S: Sink> std::fmt::Debug for Capture<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Capture")
            .field("stats", &self.stats)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<S: Sink> Capture<S> {
    /// Opens file 0 and writes the header and what the runner has.
    pub fn new(
        sink: S,
        opts: Options<'_>,
        now: Time,
        runner: &mut Runner,
    ) -> io::Result<Capture<S>> {
        let mut c = Capture {
            sink,
            writer: None,
            clock: opts.clock,
            node: opts.node.to_vec(),
            label: opts.label.to_owned(),
            max_file_bytes: opts.max_file_bytes,
            tap: opts.tap,
            peers: BTreeMap::new(),
            next_peer: 1,
            swarms: BTreeMap::new(),
            next_swarm: 1,
            pending: Vec::new(),
            stats: Stats::default(),
            closed: 0,
            error: None,
            tap_stopped: false,
        };
        runner.set_tap(if opts.tap {
            TapLevel::All
        } else {
            TapLevel::Lifecycle
        });
        runner.drain_tap(|_| {});
        if let Err(e) = c.open(0, now, runner) {
            runner.set_tap(TapLevel::Off);
            return Err(e);
        }
        Ok(c)
    }

    pub fn stats(&self) -> Stats {
        let mut s = self.stats;
        s.written = self.closed + self.writer.as_ref().map_or(0, Writer::written);
        s
    }

    /// Why the capture stopped, if it did.
    pub fn error(&self) -> Option<&io::Error> {
        self.error.as_ref()
    }

    /// Flushes the current file.
    pub fn flush(&mut self) -> io::Result<()> {
        match self.writer.as_mut() {
            Some(w) => w.flush(),
            None => Ok(()),
        }
    }

    /// Stops capturing: turns the runner's tap off and flushes.
    pub fn stop(&mut self, runner: &mut Runner) -> io::Result<()> {
        runner.set_tap(TapLevel::Off);
        self.tap_stopped = true;
        let r = self.flush();
        self.writer = None;
        r
    }

    fn open(&mut self, index: u64, now: Time, runner: &Runner) -> io::Result<()> {
        if let Some(mut w) = self.writer.take() {
            self.closed = self.closed.saturating_add(w.written());
            w.flush()?;
        }
        let file = self.sink.open(index)?;
        let w = Writer::new(
            file,
            &Header {
                clock: self.clock,
                origin: now,
                file: index,
                node: &self.node,
                label: &self.label,
            },
        )?;
        self.writer = Some(w);
        self.stats.files = index + 1;
        self.snapshot(now, runner)
    }

    /// Records every peer, swarm and channel the runner has.
    fn snapshot(&mut self, now: Time, runner: &Runner) -> io::Result<()> {
        for (key, name) in runner.peers() {
            let peer = self.peer_id(key);
            self.write(now, &Record::Peer { peer, name })?;
        }
        for (key, swarm) in runner.swarms() {
            let id = self.swarm_id(key);
            self.write(now, &swarm_record(id, swarm))?;
        }
        for (peer, swarm, channel, peer_channel) in runner.channels() {
            let (peer, swarm) = (self.peer_id(peer), self.swarm_id(swarm));
            self.write(
                now,
                &Record::ChannelOpen {
                    peer,
                    swarm,
                    channel,
                    peer_channel,
                },
            )?;
        }
        Ok(())
    }

    fn peer_id(&mut self, key: PeerKey) -> u32 {
        *self.peers.entry(key).or_insert_with(|| {
            let id = self.next_peer;
            self.next_peer = self.next_peer.saturating_add(1);
            id
        })
    }

    fn swarm_id(&mut self, key: SwarmKey) -> u32 {
        *self.swarms.entry(key).or_insert_with(|| {
            let id = self.next_swarm;
            self.next_swarm = self.next_swarm.saturating_add(1);
            id
        })
    }

    fn write(&mut self, now: Time, r: &Record<'_>) -> io::Result<()> {
        match self.writer.as_mut() {
            Some(w) => w.write(now, r),
            None => Ok(()),
        }
    }

    /// Notes a failure: nothing more is written.
    fn fail(&mut self, e: io::Error) {
        if self.error.is_none() {
            self.error = Some(e);
        }
        self.writer = None;
    }

    fn live(&self) -> bool {
        self.writer.is_some()
    }

    /// Writes the runner's tap events, then starts the next file if this
    /// one is full. After a failure, turns the tap off.
    pub fn drain(&mut self, now: Time, runner: &mut Runner) {
        if !self.live() {
            if self.error.is_some() && !self.tap_stopped {
                runner.set_tap(TapLevel::Off);
                self.tap_stopped = true;
            }
            return;
        }
        let mut pending = std::mem::take(&mut self.pending);
        runner.drain_tap(|e| pending.push(e));
        let mut res = Ok(());
        for e in pending.drain(..) {
            res = self.event(now, runner, e);
            if res.is_err() {
                break;
            }
        }
        self.pending = pending;
        if res.is_ok()
            && self.max_file_bytes > 0
            && self
                .writer
                .as_ref()
                .is_some_and(|w| w.written() >= self.max_file_bytes)
        {
            res = self.open(self.stats.files, now, runner);
        }
        if let Err(e) = res {
            self.fail(e);
            runner.set_tap(TapLevel::Off);
            self.tap_stopped = true;
        }
    }

    fn event(&mut self, now: Time, runner: &Runner, e: TapEvent) -> io::Result<()> {
        let peer = e.peer.and_then(|k| self.peers.get(&k).copied());
        let swarm = e.swarm.and_then(|k| self.swarms.get(&k).copied());
        match e.kind {
            TapKind::PeerAdded => {
                if let Some(key) = e.peer
                    && let Some(name) = runner.peer_id(key)
                {
                    let peer = self.peer_id(key);
                    self.write(now, &Record::Peer { peer, name })?;
                }
            }
            TapKind::PeerRemoved => {
                if let Some(peer) = peer {
                    self.write(now, &Record::PeerGone { peer })?;
                }
                if let Some(k) = e.peer {
                    self.peers.remove(&k);
                }
            }
            TapKind::SwarmAdded => {
                if let Some(key) = e.swarm
                    && let Some(s) = runner.swarm(key)
                {
                    let id = self.swarm_id(key);
                    self.write(now, &swarm_record(id, s))?;
                }
            }
            TapKind::SwarmRemoved => {
                if let Some(swarm) = swarm {
                    self.write(now, &Record::SwarmGone { swarm })?;
                }
                if let Some(k) = e.swarm {
                    self.swarms.remove(&k);
                }
            }
            TapKind::ChannelOpened {
                channel,
                peer_channel,
            } => {
                if let (Some(peer), Some(swarm)) = (peer, swarm) {
                    self.write(
                        now,
                        &Record::ChannelOpen {
                            peer,
                            swarm,
                            channel,
                            peer_channel,
                        },
                    )?;
                }
            }
            TapKind::ChannelClosed => {
                if let (Some(peer), Some(swarm)) = (peer, swarm) {
                    self.write(now, &Record::ChannelClose { peer, swarm })?;
                }
            }
            TapKind::Lost { count } => self.write(now, &Record::TapLost { count })?,
            kind if self.tap => {
                let id = |p| i64::from(self.peers.get(&p).copied().unwrap_or(0));
                if let Some(r) = tap_record(kind, peer.unwrap_or(0), swarm.unwrap_or(0), id) {
                    self.stats.tap_records += 1;
                    self.write(now, &Record::Tap(r))?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Writes a sample of the state below ppspp: of `peer`'s link, or of
    /// the node.
    pub fn sample(&mut self, now: Time, peer: Option<PeerKey>, sample: &Sample) {
        if !self.live() {
            return;
        }
        let peer = peer.map_or(0, |p| self.peer_id(p));
        let (source, fields) = sample.encode();
        self.stats.samples += 1;
        let r = Record::Sample(SampleRecord::new(source, peer, &fields));
        if let Err(e) = self.write(now, &r) {
            self.fail(e);
        }
    }

    /// Writes a frame. Peers the capture hasn't seen get an id here; the
    /// drains before a frame normally name them first.
    pub fn frame(&mut self, now: Time, dir: Dir, peer: PeerKey, body: &[u8]) {
        if !self.live() {
            return;
        }
        let peer = self.peer_id(peer);
        let len = body.len() as u64;
        match dir {
            Dir::In => {
                self.stats.frames_in += 1;
                self.stats.bytes_in += len;
            }
            Dir::Out => {
                self.stats.frames_out += 1;
                self.stats.bytes_out += len;
            }
        }
        if let Err(e) = self.write(
            now,
            &Record::Frame {
                dir,
                peer,
                len,
                body,
            },
        ) {
            self.fail(e);
        }
    }

    /// Notes that the runner refused the frame just written.
    pub fn frame_error(&mut self, now: Time, peer: PeerKey, error: &dyn std::fmt::Display) {
        if !self.live() {
            return;
        }
        let peer = self.peer_id(peer);
        let mut message = [0u8; crate::MAX_MESSAGE];
        let n = {
            let mut cur = io::Cursor::new(message.as_mut_slice());
            let _ = write!(cur, "{error}");
            cur.position() as usize
        };
        let message = message.get(..n).unwrap_or_default();
        if let Err(e) = self.write(now, &Record::FrameError { peer, message }) {
            self.fail(e);
        }
    }

    /// [`Runner::handle_frame`], captured: drain, the frame, the call, its
    /// error, drain.
    pub fn handle_frame(
        &mut self,
        runner: &mut Runner,
        peer: PeerKey,
        now: Time,
        body: &[u8],
    ) -> Result<(), strims_ppspp::RunnerError> {
        self.drain(now, runner);
        self.frame(now, Dir::In, peer, body);
        let r = runner.handle_frame(peer, now, body);
        if let Err(e) = &r {
            self.frame_error(now, peer, e);
        }
        self.drain(now, runner);
        r
    }

    /// [`Runner::poll_transmit`], captured: drain, the call, the frame it
    /// wrote, drain.
    pub fn poll_transmit(
        &mut self,
        runner: &mut Runner,
        peer: PeerKey,
        now: Time,
        buf: &mut [u8],
    ) -> usize {
        self.drain(now, runner);
        let n = runner.poll_transmit(peer, now, buf);
        if n > 0 {
            self.frame(now, Dir::Out, peer, buf.get(..n).unwrap_or_default());
        }
        self.drain(now, runner);
        n
    }
}

/// A decision's record, with the fields its kind's table names.
/// `id` gives a peer's id in the dump, 0 if it has none.
fn tap_record(
    kind: TapKind,
    peer: u32,
    swarm: u32,
    id: impl Fn(PeerKey) -> i64,
) -> Option<TapRecord> {
    let b = |b: Bin| b.0 as i64;
    let flag = i64::from;
    let s = i64::from;
    let n = |v: u64| i64::try_from(v).unwrap_or(i64::MAX);
    let t = |t: Time| t.unix_nanos();
    let d = |d: Duration| d.as_nanos();
    let od = |v: Option<Duration>| v.map_or(-1, Duration::as_nanos);
    let on = |v: Option<u64>| v.map_or(-1, n);
    let lag = |l: LagEstimate| [d(l.mean), d(l.dev), d(l.peak), d(l.deadline)];
    let r = |code: u32, fields: &[i64]| Some(TapRecord::new(code, peer, swarm, fields));
    let admission = |code: u32, stream: Stream, a: Admission| {
        let rule = match a.rule {
            AdmitRule::Subscribed => 0,
            AdmitRule::Pin => 1,
            AdmitRule::SeedHolder => 2,
            AdmitRule::SeedBarred => 3,
            AdmitRule::SeedNoCopy => 4,
            AdmitRule::SeedStreamsHeld => 5,
            AdmitRule::Unmeasured => 6,
            AdmitRule::Measured => 7,
        };
        r(
            code,
            &[
                s(stream),
                rule,
                flag(a.extra),
                n(a.upload_rate),
                n(a.committed),
                n(a.others),
                n(a.stream_rate),
                on(a.capacity),
                n(a.target_permille),
                n(a.stream_children),
                n(a.holders),
                n(a.copies),
                n(a.held),
                n(a.per_holder),
            ],
        )
    };
    match kind {
        TapKind::DataSent {
            bin,
            push: p,
            queued,
        } => r(1, &[b(bin), flag(p), t(queued)]),
        TapKind::DataRefused { bin, push: p } => r(2, &[b(bin), flag(p)]),
        TapKind::DataDropped { bin, push: p } => r(3, &[b(bin), flag(p)]),
        TapKind::PushStale { bin, queued } => r(4, &[b(bin), t(queued)]),
        TapKind::PushSuppressed { bin } => r(5, &[b(bin)]),
        TapKind::DataStale { bin } => r(6, &[b(bin)]),
        TapKind::PushHit { bin, lag } => r(7, &[b(bin), d(lag)]),
        TapKind::PushLate { bin, lag } => r(8, &[b(bin), d(lag)]),
        TapKind::PushUnsolicited { bin } => r(9, &[b(bin)]),
        TapKind::PushMiss {
            bin,
            deadline,
            first_seen,
            counted,
            lag: l,
        } => {
            let [m, dv, pk, dl] = lag(l);
            r(
                10,
                &[
                    b(bin),
                    t(deadline),
                    t(first_seen),
                    flag(counted),
                    m,
                    dv,
                    pk,
                    dl,
                ],
            )
        }
        TapKind::RequestTimeout {
            bin,
            requested,
            timeout,
        } => r(11, &[b(bin), t(requested), d(timeout)]),
        TapKind::Pruned { stream, why } => {
            let rule = match (why.silent, why.lossy) {
                (true, true) => 2,
                (false, true) => 1,
                _ => 0,
            };
            let [m, dv, pk, dl] = lag(why.lag);
            r(
                12,
                &[
                    s(stream),
                    rule,
                    n(why.silent_misses),
                    od(why.since_hit),
                    n(why.miss_rate_ppm),
                    n(why.expected),
                    n(why.pushes),
                    od(why.age),
                    m,
                    dv,
                    pk,
                    dl,
                ],
            )
        }
        TapKind::Switched { stream, from } => r(13, &[s(stream), id(from)]),
        TapKind::OpenTimeout { stream, waited } => r(14, &[s(stream), d(waited)]),
        TapKind::Declined { stream } => r(15, &[s(stream)]),
        TapKind::ShedBy { stream } => r(16, &[s(stream)]),
        TapKind::Admitted { stream, why } => admission(17, stream, why),
        TapKind::Rejected { stream, why } => admission(18, stream, why),
        TapKind::Shed { stream, why } => match why {
            Shed::Saturated {
                busy,
                committed,
                others,
                capacity,
                excess,
                count,
            } => r(
                19,
                &[
                    s(stream),
                    0,
                    n(busy),
                    n(committed),
                    n(others),
                    n(capacity),
                    n(excess),
                    n(count),
                    0,
                ],
            ),
            Shed::SeedBudget { room } => r(19, &[s(stream), 1, 0, 0, 0, 0, 0, 0, n(room)]),
        },
        TapKind::HolderJoined => r(20, &[]),
        TapKind::HolderLeft { why, copies, rtt } => {
            let why = match why {
                HolderLeft::Silent => 0,
                HolderLeft::Unserved => 1,
                HolderLeft::PastCopies => 2,
                HolderLeft::Idle => 3,
                HolderLeft::Closed => 4,
            };
            r(21, &[why, n(copies), od(rtt)])
        }
        TapKind::SeedBudget {
            budget,
            copies,
            streams,
            capacity,
            stream_rate,
            others,
        } => r(
            22,
            &[
                n(budget),
                n(copies),
                n(streams),
                on(capacity),
                n(stream_rate),
                n(others),
            ],
        ),
        TapKind::SeedWindow {
            copies,
            pushed,
            behind,
        } => r(23, &[n(copies), n(pushed), n(behind)]),
        TapKind::HolderSilent => r(24, &[]),
        TapKind::HolderUnserved { reporters } => r(25, &[n(reporters)]),
        TapKind::PushCancelled { bin } => r(26, &[b(bin)]),
        TapKind::ParentChoice { stream, choice: c } => {
            let action = match c.action {
                ParentAction::Graft => 0,
                ParentAction::None => 1,
                ParentAction::Vote => 2,
                ParentAction::Switch => 3,
                ParentAction::Reset => 4,
            };
            let who = |x: Option<(PeerKey, Duration)>| x.map_or((0, -1), |(p, l)| (id(p), d(l)));
            let (best, best_lag) = who(c.best);
            let (next, next_lag) = who(c.runner_up);
            r(
                27,
                &[
                    s(stream),
                    action,
                    best,
                    best_lag,
                    next,
                    next_lag,
                    n(c.unready),
                    n(c.subscriber),
                    n(c.backed_off),
                    n(c.stale),
                    n(c.votes),
                    od(c.parent_have),
                    od(c.parent_push),
                ],
            )
        }
        TapKind::RequestRound(q) => r(
            28,
            &[
                b(q.floor),
                n(q.cap),
                n(q.in_flight),
                n(q.room),
                n(q.fit),
                n(q.candidates),
                n(q.requested),
            ],
        ),
        TapKind::Upload(u) => r(
            29,
            &[
                n(u.rate),
                n(u.busy),
                on(u.capacity),
                on(u.hint),
                n(u.others),
                n(u.committed),
                od(u.queue_delay),
            ],
        ),
        TapKind::SeedFirstWave {
            capacity,
            early_rate,
            others,
            neighbours,
            copies,
        } => r(
            30,
            &[
                n(capacity),
                n(early_rate),
                n(others),
                n(neighbours),
                n(copies),
            ],
        ),
        TapKind::AnswerRate { rate, reading } => r(31, &[n(rate), od(reading)]),
        TapKind::PeerAdded
        | TapKind::PeerRemoved
        | TapKind::SwarmAdded
        | TapKind::SwarmRemoved
        | TapKind::ChannelOpened { .. }
        | TapKind::ChannelClosed
        | TapKind::Lost { .. } => None,
    }
}

/// What the dump says of a swarm.
fn swarm_record(id: u32, swarm: &Swarm) -> Record<'_> {
    let o = swarm.options();
    let rc = swarm.reader_config();
    Record::Swarm {
        swarm: id,
        id: swarm.id().as_bytes(),
        params: SwarmParams {
            chunk_size: rc.chunk_size as u64,
            integrity_hash_size: rc.integrity_hash_size as u64,
            integrity_signature_size: rc.integrity_signature_size as u64,
            stream_count: o.stream_count as u64,
            chunks_per_signature: o.chunks_per_signature as u64,
            live_window: o.live_window as u64,
            protection_method: o.integrity.protection_method.0.into(),
            merkle_hash_tree_function: o.integrity.merkle_hash_tree_function.0.into(),
            live_signature_algorithm: o.integrity.live_signature_algorithm.0.into(),
            scheduling_method: o.scheduling_method.0.into(),
            delivery_mode: o.delivery_mode.0.into(),
            buffer_layout: o.buffer_layout.0.into(),
            push_mode: match o.push_mode.resolve() {
                PushMode::Off => 1,
                PushMode::EagerHave => 2,
                PushMode::Hybrid => 3,
                PushMode::Default => 0,
            },
            request_controller: match o.request_controller.resolve() {
                RequestControllerKind::Ledbat => 1,
                RequestControllerKind::BbrLike => 2,
                RequestControllerKind::Default => 0,
            },
            seed_allocation: match o.seed.allocation.resolve() {
                SeedAllocation::Admission => 1,
                SeedAllocation::Adaptive => 2,
                SeedAllocation::Default => 0,
            },
            seed_qos: match o.seed.qos.resolve() {
                SeedQos::Equal => 1,
                SeedQos::ByAllocation => 2,
                SeedQos::Default => 0,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]

    use strims_ppspp::{ParentChoice, Prune, RequestRound, Upload};

    use super::*;

    /// One of each decision the tap reports.
    fn decisions() -> Vec<TapKind> {
        let (b, t, d) = (Bin(4), Time::from_unix_millis(5), Duration::MILLISECOND);
        let peer = PeerKey::default();
        let lag = LagEstimate::default();
        let why = Admission::default();
        vec![
            TapKind::DataSent {
                bin: b,
                push: true,
                queued: t,
            },
            TapKind::DataRefused { bin: b, push: true },
            TapKind::DataDropped { bin: b, push: true },
            TapKind::PushStale { bin: b, queued: t },
            TapKind::PushSuppressed { bin: b },
            TapKind::PushCancelled { bin: b },
            TapKind::DataStale { bin: b },
            TapKind::PushHit { bin: b, lag: d },
            TapKind::PushLate { bin: b, lag: d },
            TapKind::PushUnsolicited { bin: b },
            TapKind::PushMiss {
                bin: b,
                deadline: t,
                first_seen: t,
                counted: true,
                lag,
            },
            TapKind::RequestTimeout {
                bin: b,
                requested: t,
                timeout: d,
            },
            TapKind::RequestRound(RequestRound {
                floor: b,
                cap: 1,
                in_flight: 1,
                room: 1,
                fit: 1,
                candidates: 1,
                requested: 1,
            }),
            TapKind::Pruned {
                stream: 1,
                why: Prune {
                    silent: true,
                    lossy: true,
                    silent_misses: 1,
                    since_hit: None,
                    miss_rate_ppm: 1,
                    expected: 1,
                    pushes: 1,
                    age: Some(d),
                    lag,
                },
            },
            TapKind::Switched {
                stream: 1,
                from: peer,
            },
            TapKind::OpenTimeout {
                stream: 1,
                waited: d,
            },
            TapKind::ParentChoice {
                stream: 1,
                choice: ParentChoice {
                    action: ParentAction::Vote,
                    best: Some((peer, d)),
                    runner_up: None,
                    unready: 1,
                    subscriber: 1,
                    backed_off: 1,
                    stale: 1,
                    votes: 1,
                    parent_have: None,
                    parent_push: Some(d),
                },
            },
            TapKind::Declined { stream: 1 },
            TapKind::ShedBy { stream: 1 },
            TapKind::Admitted { stream: 1, why },
            TapKind::Rejected { stream: 1, why },
            TapKind::Shed {
                stream: 1,
                why: Shed::SeedBudget { room: 1 },
            },
            TapKind::Shed {
                stream: 1,
                why: Shed::Saturated {
                    busy: 1,
                    committed: 1,
                    others: 1,
                    capacity: 1,
                    excess: 1,
                    count: 1,
                },
            },
            TapKind::HolderJoined,
            TapKind::HolderLeft {
                why: HolderLeft::Closed,
                copies: 1,
                rtt: None,
            },
            TapKind::HolderSilent,
            TapKind::HolderUnserved { reporters: 1 },
            TapKind::SeedBudget {
                budget: 1,
                copies: 1,
                streams: 1,
                capacity: None,
                stream_rate: 1,
                others: 1,
            },
            TapKind::SeedFirstWave {
                capacity: 1,
                early_rate: 1,
                others: 1,
                neighbours: 1,
                copies: 1,
            },
            TapKind::SeedWindow {
                copies: 1,
                pushed: 1,
                behind: 1,
            },
            TapKind::Upload(Upload::default()),
            TapKind::AnswerRate {
                rate: 1,
                reading: Some(d),
            },
        ]
    }

    #[test]
    fn every_decision_has_its_tables_fields() {
        let mut seen = std::collections::BTreeSet::new();
        for k in decisions() {
            let r = tap_record(k, 1, 1, |_| 1).unwrap_or_else(|| panic!("{k:?}"));
            let info = tap_kind(r.kind).unwrap_or_else(|| panic!("{k:?}"));
            assert_eq!(r.fields().len(), info.fields.len(), "{k:?}");
            assert!(info.v1_fields <= info.fields.len(), "{k:?}");
            seen.insert(r.kind);
        }
        assert_eq!(seen.len(), TAP_KINDS.len());
    }
}
