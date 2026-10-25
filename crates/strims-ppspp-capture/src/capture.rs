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
//! nanoseconds since the Unix epoch, durations nanoseconds, peers dump
//! ids):
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
//! | 10 | `PushMiss` | bin |
//! | 11 | `RequestTimeout` | bin |
//! | 12 | `Pruned` | stream |
//! | 13 | `Switched` | stream, from peer |
//! | 14 | `OpenTimeout` | stream |
//! | 15 | `Declined` | stream |
//! | 16 | `ShedBy` | stream |
//! | 17 | `Admitted` | stream |
//! | 18 | `Rejected` | stream |
//! | 19 | `Shed` | stream |
//! | 20 | `HolderJoined` | |
//! | 21 | `HolderLeft` | |
//! | 22 | `SeedBudget` | budget, copies, streams |
//! | 23 | `SeedWindow` | copies, pushed, behind |
//! | 24 | `HolderSilent` | |
//! | 25 | `HolderUnserved` | reporters |
//! | 26 | `PushCancelled` | bin |
//!
//! [`strims_ppspp::TapKind`] documents what each means.

use std::collections::BTreeMap;
use std::io::{self, Write};

use strims_base::time::Time;
use strims_binmap::Bin;
use strims_ppspp::{
    PeerKey, PushMode, RequestControllerKind, Runner, SeedAllocation, SeedQos, Swarm, SwarmKey,
    TapEvent, TapKind, TapLevel,
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
    KindInfo::new(10, "PushMiss", &[num("bin")]),
    KindInfo::new(11, "RequestTimeout", &[num("bin")]),
    KindInfo::new(12, "Pruned", &[num("stream")]),
    KindInfo::new(13, "Switched", &[num("stream"), peer("from")]),
    KindInfo::new(14, "OpenTimeout", &[num("stream")]),
    KindInfo::new(15, "Declined", &[num("stream")]),
    KindInfo::new(16, "ShedBy", &[num("stream")]),
    KindInfo::new(17, "Admitted", &[num("stream")]),
    KindInfo::new(18, "Rejected", &[num("stream")]),
    KindInfo::new(19, "Shed", &[num("stream")]),
    KindInfo::new(20, "HolderJoined", &[]),
    KindInfo::new(21, "HolderLeft", &[]),
    KindInfo::new(
        22,
        "SeedBudget",
        &[num("budget"), num("copies"), num("streams")],
    ),
    KindInfo::new(
        23,
        "SeedWindow",
        &[num("copies"), num("pushed"), num("behind")],
    ),
    KindInfo::new(24, "HolderSilent", &[]),
    KindInfo::new(25, "HolderUnserved", &[num("reporters")]),
    KindInfo::new(26, "PushCancelled", &[num("bin")]),
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
                if let Some(r) = self.tap_record(kind, peer.unwrap_or(0), swarm.unwrap_or(0)) {
                    self.stats.tap_records += 1;
                    self.write(now, &Record::Tap(r))?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// A decision's record, with the fields its kind's table names.
    fn tap_record(&self, kind: TapKind, peer: u32, swarm: u32) -> Option<TapRecord> {
        let b = |b: Bin| b.0 as i64;
        let push = |p: bool| i64::from(p);
        let s = i64::from;
        let r = |code: u32, fields: &[i64]| Some(TapRecord::new(code, peer, swarm, fields));
        match kind {
            TapKind::DataSent {
                bin,
                push: p,
                queued,
            } => r(1, &[b(bin), push(p), queued.unix_nanos()]),
            TapKind::DataRefused { bin, push: p } => r(2, &[b(bin), push(p)]),
            TapKind::DataDropped { bin, push: p } => r(3, &[b(bin), push(p)]),
            TapKind::PushStale { bin, queued } => r(4, &[b(bin), queued.unix_nanos()]),
            TapKind::PushSuppressed { bin } => r(5, &[b(bin)]),
            TapKind::DataStale { bin } => r(6, &[b(bin)]),
            TapKind::PushHit { bin, lag } => r(7, &[b(bin), lag.as_nanos()]),
            TapKind::PushLate { bin, lag } => r(8, &[b(bin), lag.as_nanos()]),
            TapKind::PushUnsolicited { bin } => r(9, &[b(bin)]),
            TapKind::PushMiss { bin } => r(10, &[b(bin)]),
            TapKind::RequestTimeout { bin } => r(11, &[b(bin)]),
            TapKind::Pruned { stream } => r(12, &[s(stream)]),
            TapKind::Switched { stream, from } => r(13, &[s(stream), self.peer_field(from)]),
            TapKind::OpenTimeout { stream } => r(14, &[s(stream)]),
            TapKind::Declined { stream } => r(15, &[s(stream)]),
            TapKind::ShedBy { stream } => r(16, &[s(stream)]),
            TapKind::Admitted { stream } => r(17, &[s(stream)]),
            TapKind::Rejected { stream } => r(18, &[s(stream)]),
            TapKind::Shed { stream } => r(19, &[s(stream)]),
            TapKind::HolderJoined => r(20, &[]),
            TapKind::HolderLeft => r(21, &[]),
            TapKind::SeedBudget {
                budget,
                copies,
                streams,
            } => r(22, &[budget as i64, copies as i64, streams as i64]),
            TapKind::SeedWindow {
                copies,
                pushed,
                behind,
            } => r(23, &[copies as i64, pushed as i64, behind as i64]),
            TapKind::HolderSilent => r(24, &[]),
            TapKind::HolderUnserved { reporters } => r(25, &[reporters as i64]),
            TapKind::PushCancelled { bin } => r(26, &[b(bin)]),
            _ => None,
        }
    }

    /// A peer's dump id as a field, 0 if the capture doesn't know it.
    fn peer_field(&self, peer: PeerKey) -> i64 {
        i64::from(self.peers.get(&peer).copied().unwrap_or(0))
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
