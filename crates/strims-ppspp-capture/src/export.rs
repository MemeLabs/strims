// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A dump as rows: one per frame, one per ppspp message in it, one per
//! tap decision that annotates no message, and one per sample. [`export`]
//! writes them as JSON lines, an object per row, leaving out fields a row
//! doesn't have:
//!
//! | field | value |
//! |---|---|
//! | `t` | the record's time, ns since the Unix epoch |
//! | `node` | the capturing node's name, hex |
//! | `peer` | the peer's name, hex |
//! | `dir` | `in`, `out`, `tap` or `sample` |
//! | `frame` | the frame's number in the dump, from 1; on a tap or sample row, the last frame's |
//! | `swarm` | the swarm's id in the dump |
//! | `channel` | the wire channel id |
//! | `type` | `FRAME`, a message type (`DATA`, `HAVE`, `ACK`, `REQUEST`, `CANCEL`, `INTEGRITY`, `SIGNED_INTEGRITY`, `HANDSHAKE`, `RESTART`, `CHOKE`, `UNCHOKE`, `PING`, `PONG`, `STREAM_REQUEST`, `STREAM_CANCEL`, `STREAM_OPEN`, `STREAM_CLOSE`, `ANNOUNCE`), `UNKNOWN_CHANNEL`, `MALFORMED`, `FRAME_ERROR`, `TapLost`, a tap kind's name ([`TAP_KINDS`](crate::TAP_KINDS)), or a sample source's ([`SAMPLE_KINDS`](crate::SAMPLE_KINDS)) |
//! | `bin` | the bin |
//! | `first`, `last` | its first and last chunk |
//! | `stream` | the stream: a message's, or a single chunk's (`chunk % stream_count`) |
//! | `ts` | DATA, SIGNED_INTEGRITY and REQUEST timestamps, ns |
//! | `len` | bytes: a frame's, a message's with its type byte, or what's left of a payload that didn't decode |
//! | `ann` | on DATA out, `push` or `answer`; on DATA in, `hit`, `late`, `unsolicited` or `stale` |
//! | `wait` | on DATA out and push decisions, ns from queueing to now |
//! | `lag` | on DATA in and push hits and late pushes, ns since the chunk was first seen |
//! | `from` | the previous parent of a switch, hex |
//! | `delay` | ACK and PONG delays, ns |
//! | `nonce` | PING and PONG nonces: a pong carries its ping's |
//! | `count` | on ANNOUNCE, its entries; on `TapLost`, the events lost |
//!
//! A tap row carries its decision's fields, and a sample row its
//! source's, by their tables' names: a peer as its name in hex, a named
//! value as its name, any other as a number. Decisions about data go onto
//! the DATA row they describe, in the frame just before them: `DataSent`
//! on DATA out, and `PushHit`, `PushLate`, `PushUnsolicited` and
//! `DataStale` on DATA in.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{self, Read, Write};

use strims_binmap::Bin;
use strims_ppspp::codec::{FrameReader, Message, Reader as MessageReader, ReaderConfig};

use crate::capture::{FieldKind, KindInfo, tap_kind};
use crate::format::{Dir, MAX_TAP_FIELDS, Record, SampleRecord, SwarmParams, TapRecord};
use crate::reader::{Error, Reader};
use crate::sample::sample_kind;

/// Rows a frame holds while the decisions after it may annotate them;
/// rows past it are written at once.
const MAX_HELD: usize = 4096;
/// The most peers, swarms and channels an export tracks.
const MAX_TRACKED: usize = 1 << 16;
/// Swarm params past these are malformed: a decoder would never use them.
const MAX_CHUNK: u64 = 1 << 24;
const MAX_INTEGRITY: u64 = 1 << 10;

/// What an export did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExportStats {
    pub rows: u64,
    pub frames: u64,
    /// Records skipped, and the error that ended the dump, if any.
    pub errors: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum RowDir {
    In,
    Out,
    #[default]
    Tap,
    Sample,
}

/// A decision's or a sample's field value.
#[derive(Clone, Copy, Debug)]
enum Val {
    Num(i64),
    Peer(u32),
    Name(&'static str),
}

#[derive(Clone, Copy, Debug, Default)]
struct Row {
    t: i64,
    dir: RowDir,
    peer: u32,
    frame: u64,
    swarm: u32,
    channel: Option<u64>,
    kind: &'static str,
    bin: Option<Bin>,
    stream: Option<u64>,
    ts: Option<i64>,
    len: Option<u64>,
    ann: Option<&'static str>,
    wait: Option<i64>,
    lag: Option<i64>,
    from: Option<u32>,
    delay: Option<i64>,
    nonce: Option<u64>,
    other: [Option<(&'static str, Val)>; MAX_TAP_FIELDS],
}

/// What ties a row to a row in another node's dump.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Join {
    /// DATA sent to `to`: `ann` is `push` or `answer` once known.
    Sent {
        to: String,
        swarm: String,
        bin: u64,
        ann: Option<&'static str>,
    },
    /// DATA received from `from`.
    Received {
        from: String,
        swarm: String,
        bin: u64,
    },
    /// A decision about `child`'s subscription that the child hears of as
    /// `heard` (a `Rejected` as `Declined`, a `Shed` as `ShedBy`).
    Decided {
        child: String,
        swarm: String,
        stream: u64,
        heard: &'static str,
    },
    /// A child's `Declined` or `ShedBy` from `parent`.
    Heard {
        parent: String,
        swarm: String,
        stream: u64,
        kind: &'static str,
    },
}

/// Where a dump's rows go: each as its JSON object without the closing
/// brace, and when the sink asks for them, its join.
pub(crate) trait RowSink {
    fn wants_joins(&self) -> bool {
        false
    }
    fn row(&mut self, t: i64, json: &str, join: Option<Join>) -> io::Result<()>;
}

/// Writes rows as JSON lines.
struct Lines<'w, W: Write>(&'w mut W);

impl<W: Write> RowSink for Lines<'_, W> {
    fn row(&mut self, _: i64, json: &str, _: Option<Join>) -> io::Result<()> {
        self.0.write_all(json.as_bytes())?;
        self.0.write_all(b"}\n")
    }
}

/// One node's dump state, across the files of a rotated capture.
pub(crate) struct Dump<S> {
    pub(crate) sink: S,
    node: String,
    version: u16,
    /// Bound: [`MAX_TRACKED`].
    peers: BTreeMap<u32, String>,
    /// Bound: [`MAX_TRACKED`].
    swarms: BTreeMap<u32, (SwarmParams, String)>,
    /// (peer, frames in, wire channel) to swarm. Bound: [`MAX_TRACKED`].
    channels: BTreeMap<(u32, bool, u64), u32>,
    frame: u64,
    /// The last frame's rows. Bound: [`MAX_HELD`].
    held: Vec<Row>,
    held_from: (u32, RowDir),
    buf: String,
    pub(crate) stats: ExportStats,
}

/// Writes `reader`'s records as JSON lines. Records that don't decode go
/// to `on_error` and are skipped; an error that ends the dump goes there
/// too. Only write errors fail the export.
pub fn export<R: Read, W: Write>(
    reader: &mut Reader<R>,
    out: &mut W,
    on_error: &mut dyn FnMut(&Error),
) -> io::Result<ExportStats> {
    let mut d = Dump::new(Lines(out), &reader.header().node, reader.header().version);
    while let Some(e) = reader.next_record() {
        match e {
            Ok(e) => d.record(e.time.unix_nanos(), &e.record)?,
            Err(e) => {
                d.stats.errors += 1;
                on_error(&e);
            }
        }
    }
    d.flush()?;
    Ok(d.stats)
}

impl<S: RowSink> Dump<S> {
    pub(crate) fn new(sink: S, node: &[u8], version: u16) -> Dump<S> {
        Dump {
            sink,
            node: hex(node),
            version,
            peers: BTreeMap::new(),
            swarms: BTreeMap::new(),
            channels: BTreeMap::new(),
            frame: 0,
            held: Vec::new(),
            held_from: (0, RowDir::Tap),
            buf: String::new(),
            stats: ExportStats::default(),
        }
    }

    /// The next file of the same node's capture: its version may differ.
    pub(crate) fn set_version(&mut self, version: u16) {
        self.version = version;
    }

    pub(crate) fn record(&mut self, t: i64, r: &Record<'_>) -> io::Result<()> {
        if !matches!(r, Record::Tap(_) | Record::TapLost { .. }) {
            self.flush()?;
        }
        match *r {
            Record::Peer { peer, name } => {
                if self.peers.len() < MAX_TRACKED || self.peers.contains_key(&peer) {
                    self.peers.insert(peer, hex(name));
                }
            }
            Record::Swarm { swarm, id, params } => {
                if self.swarms.len() < MAX_TRACKED || self.swarms.contains_key(&swarm) {
                    self.swarms.insert(swarm, (params, hex(id)));
                }
            }
            Record::SwarmGone { swarm } => {
                self.swarms.remove(&swarm);
            }
            Record::ChannelOpen {
                peer,
                swarm,
                channel,
                peer_channel,
            } => {
                if self.channels.len() + 2 <= MAX_TRACKED {
                    self.channels.insert((peer, true, channel), swarm);
                    self.channels.insert((peer, false, peer_channel), swarm);
                }
            }
            Record::ChannelClose { peer, swarm } => {
                self.channels
                    .retain(|&(p, _, _), &mut s| !(p == peer && s == swarm));
            }
            Record::Frame {
                dir,
                peer,
                len,
                body,
            } => self.frame_rows(t, dir, peer, len, body)?,
            Record::FrameError { peer, .. } => {
                let row = Row {
                    t,
                    dir: RowDir::In,
                    peer,
                    frame: self.frame,
                    kind: "FRAME_ERROR",
                    ..Row::default()
                };
                self.write(&row)?;
            }
            Record::Tap(tr) => self.tap(t, &tr)?,
            Record::TapLost { count } => {
                self.hold(Row {
                    t,
                    frame: self.frame,
                    kind: "TapLost",
                    other: count_field(count as i64),
                    ..Row::default()
                })?;
            }
            Record::Sample(sr) => self.sample(t, &sr)?,
            Record::PeerGone { .. } | Record::Unknown { .. } => {}
        }
        Ok(())
    }

    fn frame_rows(&mut self, t: i64, dir: Dir, peer: u32, len: u64, body: &[u8]) -> io::Result<()> {
        self.frame += 1;
        self.stats.frames += 1;
        let rd = match dir {
            Dir::In => RowDir::In,
            Dir::Out => RowDir::Out,
        };
        self.held_from = (peer, rd);
        let base = Row {
            t,
            dir: rd,
            peer,
            frame: self.frame,
            ..Row::default()
        };
        self.hold(Row {
            kind: "FRAME",
            len: Some(len),
            ..base
        })?;
        for c in FrameReader::new(body) {
            let Ok((h, payload)) = c else {
                self.hold(Row {
                    kind: "MALFORMED",
                    ..base
                })?;
                break;
            };
            let channel = Some(h.channel);
            let swarm = self
                .channels
                .get(&(peer, dir == Dir::In, h.channel))
                .copied();
            let Some((swarm, params)) = swarm
                .and_then(|s| self.swarms.get(&s).map(|p| (s, p.0)))
                .filter(|(_, p)| {
                    p.chunk_size <= MAX_CHUNK
                        && p.integrity_hash_size <= MAX_INTEGRITY
                        && p.integrity_signature_size <= MAX_INTEGRITY
                })
            else {
                self.hold(Row {
                    kind: "UNKNOWN_CHANNEL",
                    channel,
                    len: Some(payload.len() as u64),
                    ..base
                })?;
                continue;
            };
            let cfg = ReaderConfig {
                chunk_size: params.chunk_size as usize,
                integrity_hash_size: params.integrity_hash_size as usize,
                integrity_signature_size: params.integrity_signature_size as usize,
            };
            let mut r = MessageReader::new(cfg, payload);
            let mut at = 0;
            while let Some(m) = r.next() {
                let row = Row {
                    swarm,
                    channel,
                    ..base
                };
                let Ok(m) = m else {
                    self.hold(Row {
                        kind: "MALFORMED",
                        len: Some(payload.len().saturating_sub(at) as u64),
                        ..row
                    })?;
                    break;
                };
                let end = r.position();
                let mut row = message_row(row, &m, &params);
                row.len = Some(end.saturating_sub(at) as u64);
                at = end;
                if !matches!(m, Message::End) {
                    self.hold(row)?;
                }
            }
        }
        Ok(())
    }

    fn hold(&mut self, row: Row) -> io::Result<()> {
        if self.held.len() < MAX_HELD {
            self.held.push(row);
            Ok(())
        } else {
            self.write(&row)
        }
    }

    pub(crate) fn flush(&mut self) -> io::Result<()> {
        let held = std::mem::take(&mut self.held);
        for row in &held {
            self.write(row)?;
        }
        self.held = held;
        self.held.clear();
        Ok(())
    }

    /// A decision: onto the DATA row it describes, or a row of its own.
    fn tap(&mut self, t: i64, tr: &TapRecord) -> io::Result<()> {
        let info = tap_kind(tr.kind);
        let name = info.map_or("TAP_UNKNOWN", |k| k.name);
        let (data_dir, ann) = match name {
            "DataSent" => (
                RowDir::Out,
                if tr.field(1) == 1 { "push" } else { "answer" },
            ),
            "PushHit" => (RowDir::In, "hit"),
            "PushLate" => (RowDir::In, "late"),
            "PushUnsolicited" => (RowDir::In, "unsolicited"),
            "DataStale" => (RowDir::In, "stale"),
            _ => (RowDir::Tap, ""),
        };
        if data_dir != RowDir::Tap && self.held_from == (tr.peer, data_dir) {
            let bin = Bin(tr.field(0) as u64);
            let chunk = bin.base_offset();
            // A send names the DATA's bin; an arrival, one of its chunks.
            let hit = |r: &&mut Row| {
                r.kind == "DATA"
                    && r.swarm == tr.swarm
                    && r.bin.is_some_and(|b| match data_dir {
                        RowDir::Out => b == bin && r.ann.is_none(),
                        _ => b.base_offset() <= chunk && chunk <= b.base_right().0 >> 1,
                    })
            };
            if let Some(row) = self.held.iter_mut().find(hit) {
                row.ann = Some(ann);
                match name {
                    "DataSent" => row.wait = wait(t, tr.field(2)),
                    "PushHit" | "PushLate" => row.lag = Some(tr.field(1)),
                    _ => {}
                }
                return Ok(());
            }
        }
        let mut row = Row {
            t,
            peer: tr.peer,
            frame: self.frame,
            swarm: tr.swarm,
            kind: name,
            ..Row::default()
        };
        if let Some(info) = info {
            let n = info.fields_of(self.version, tr.fields().len());
            let mut other = 0;
            for (i, f) in info.fields.iter().enumerate().take(n) {
                let v = tr.field(i);
                match f.name {
                    "bin" => {
                        let b = Bin(v as u64);
                        row.bin = Some(b);
                        row.stream = chunk_stream(b, self.swarms.get(&tr.swarm).map(|s| &s.0));
                    }
                    "push" => row.ann = Some(if v == 1 { "push" } else { "answer" }),
                    "queued" => row.wait = wait(t, v),
                    "lag" => row.lag = Some(v),
                    "stream" => row.stream = Some(v as u64),
                    "from" => row.from = u32::try_from(v).ok(),
                    _ => {
                        if let Some(slot) = row.other.get_mut(other) {
                            *slot = Some((f.name, value(info, i, v)));
                            other += 1;
                        }
                    }
                }
            }
        }
        self.hold(row)
    }

    fn sample(&mut self, t: i64, sr: &SampleRecord) -> io::Result<()> {
        let info = sample_kind(sr.source);
        let mut row = Row {
            t,
            dir: RowDir::Sample,
            peer: sr.peer,
            frame: self.frame,
            kind: info.map_or("SAMPLE_UNKNOWN", |k| k.name),
            ..Row::default()
        };
        if let Some(info) = info {
            let n = sr.fields().len().min(info.fields.len());
            for (i, (slot, f)) in row.other.iter_mut().zip(info.fields).take(n).enumerate() {
                *slot = Some((f.name, value(info, i, sr.field(i))));
            }
        }
        self.write(&row)
    }

    fn peer_name(&self, peer: u32) -> String {
        self.peers
            .get(&peer)
            .cloned()
            .unwrap_or_else(|| format!("?{peer}"))
    }

    fn write(&mut self, r: &Row) -> io::Result<()> {
        self.stats.rows += 1;
        let mut o = std::mem::take(&mut self.buf);
        o.clear();
        self.render(&mut o, r);
        let join = if self.sink.wants_joins() {
            self.join(r)
        } else {
            None
        };
        let res = self.sink.row(r.t, &o, join);
        self.buf = o;
        res
    }

    fn render(&self, o: &mut String, r: &Row) {
        let _ = write!(o, "{{\"t\":{},\"node\":\"{}\"", r.t, self.node);
        if r.peer != 0 {
            let _ = write!(o, ",\"peer\":\"{}\"", self.peer_name(r.peer));
        }
        let dir = match r.dir {
            RowDir::In => "in",
            RowDir::Out => "out",
            RowDir::Tap => "tap",
            RowDir::Sample => "sample",
        };
        let _ = write!(o, ",\"dir\":\"{dir}\",\"frame\":{}", r.frame);
        if r.swarm != 0 {
            let _ = write!(o, ",\"swarm\":{}", r.swarm);
        }
        if let Some(c) = r.channel {
            let _ = write!(o, ",\"channel\":{c}");
        }
        let _ = write!(o, ",\"type\":\"{}\"", r.kind);
        if let Some(b) = r.bin {
            let _ = write!(
                o,
                ",\"bin\":{},\"first\":{},\"last\":{}",
                b.0,
                b.base_offset(),
                b.base_right().0 >> 1
            );
        }
        let nums = [
            ("stream", r.stream.map(|v| v as i64)),
            ("ts", r.ts),
            ("len", r.len.map(|v| v as i64)),
        ];
        for (k, v) in nums {
            if let Some(v) = v {
                let _ = write!(o, ",\"{k}\":{v}");
            }
        }
        if let Some(a) = r.ann {
            let _ = write!(o, ",\"ann\":\"{a}\"");
        }
        for (k, v) in [("wait", r.wait), ("lag", r.lag), ("delay", r.delay)] {
            if let Some(v) = v {
                let _ = write!(o, ",\"{k}\":{v}");
            }
        }
        if let Some(n) = r.nonce {
            let _ = write!(o, ",\"nonce\":{n}");
        }
        if let Some(f) = r.from {
            let _ = write!(o, ",\"from\":\"{}\"", self.peer_name(f));
        }
        for (k, v) in r.other.iter().flatten() {
            let _ = match v {
                Val::Num(v) => write!(o, ",\"{k}\":{v}"),
                Val::Peer(0) => write!(o, ",\"{k}\":null"),
                Val::Peer(p) => write!(o, ",\"{k}\":\"{}\"", self.peer_name(*p)),
                Val::Name(n) => write!(o, ",\"{k}\":\"{n}\""),
            };
        }
    }

    /// What ties the row to another node's dump, if anything.
    fn join(&self, r: &Row) -> Option<Join> {
        let peer = self.peers.get(&r.peer)?.clone();
        let swarm = self.swarms.get(&r.swarm)?.1.clone();
        Some(match (r.dir, r.kind) {
            (RowDir::Out, "DATA") => Join::Sent {
                to: peer,
                swarm,
                bin: r.bin?.0,
                ann: r.ann,
            },
            (RowDir::In, "DATA") => Join::Received {
                from: peer,
                swarm,
                bin: r.bin?.0,
            },
            (RowDir::Tap, "Rejected" | "Shed") => Join::Decided {
                child: peer,
                swarm,
                stream: r.stream?,
                heard: if r.kind == "Shed" {
                    "ShedBy"
                } else {
                    "Declined"
                },
            },
            (RowDir::Tap, kind @ ("Declined" | "ShedBy")) => Join::Heard {
                parent: peer,
                swarm,
                stream: r.stream?,
                kind,
            },
            _ => return None,
        })
    }

    pub(crate) fn node(&self) -> &str {
        &self.node
    }
}

/// Field `i`'s value, as its kind reads it.
fn value(info: &KindInfo, i: usize, v: i64) -> Val {
    match info.fields.get(i).map(|f| f.kind) {
        Some(FieldKind::Peer) => Val::Peer(u32::try_from(v).unwrap_or(0)),
        Some(FieldKind::Enum(_)) => info.value_name(i, v).map_or(Val::Num(v), Val::Name),
        _ => Val::Num(v),
    }
}

/// A message's row, from the frame's.
fn message_row(row: Row, m: &Message<'_>, p: &SwarmParams) -> Row {
    let with_bin = |kind, b: Bin| Row {
        kind,
        bin: Some(b),
        stream: chunk_stream(b, Some(p)),
        ..row
    };
    match m {
        Message::Data(d) => Row {
            ts: Some(d.timestamp.unix_nanos()),
            ..with_bin("DATA", d.bin)
        },
        Message::Have(h) => with_bin("HAVE", h.bin),
        Message::Ack(a) => Row {
            delay: Some(a.delay_sample.as_nanos()),
            ..with_bin("ACK", a.bin)
        },
        Message::Request(q) => Row {
            ts: Some(q.timestamp.unix_nanos()),
            ..with_bin("REQUEST", q.bin)
        },
        Message::Cancel(c) => with_bin("CANCEL", c.bin),
        Message::Integrity(i) => with_bin("INTEGRITY", i.bin),
        Message::SignedIntegrity(s) => Row {
            ts: Some(s.timestamp.unix_nanos()),
            ..with_bin("SIGNED_INTEGRITY", s.bin)
        },
        Message::Handshake(_) => Row {
            kind: "HANDSHAKE",
            ..row
        },
        Message::Restart => Row {
            kind: "RESTART",
            ..row
        },
        Message::Choke => Row {
            kind: "CHOKE",
            ..row
        },
        Message::Unchoke => Row {
            kind: "UNCHOKE",
            ..row
        },
        Message::Ping(p) => Row {
            kind: "PING",
            nonce: Some(p.nonce),
            ..row
        },
        Message::Pong(p) => Row {
            kind: "PONG",
            delay: i64::try_from(p.delay).ok(),
            nonce: Some(p.nonce),
            ..row
        },
        Message::StreamRequest(s) => Row {
            kind: "STREAM_REQUEST",
            bin: Some(s.bin),
            stream: Some(s.stream.into()),
            ..row
        },
        Message::StreamOpen(s) => Row {
            kind: "STREAM_OPEN",
            bin: Some(s.bin),
            stream: Some(s.stream.into()),
            ..row
        },
        Message::StreamCancel(s) => Row {
            kind: "STREAM_CANCEL",
            stream: Some(s.stream.into()),
            ..row
        },
        Message::StreamClose(s) => Row {
            kind: "STREAM_CLOSE",
            stream: Some(s.stream.into()),
            ..row
        },
        Message::Announce(a) => Row {
            kind: "ANNOUNCE",
            other: count_field(a.len() as i64),
            ..row
        },
        Message::End => Row { kind: "END", ..row },
    }
}

/// Fields holding only `count`.
fn count_field(count: i64) -> [Option<(&'static str, Val)>; MAX_TAP_FIELDS] {
    let mut other = [None; MAX_TAP_FIELDS];
    if let Some(f) = other.first_mut() {
        *f = Some(("count", Val::Num(count)));
    }
    other
}

/// A single chunk's stream.
fn chunk_stream(b: Bin, p: Option<&SwarmParams>) -> Option<u64> {
    let n = p?.stream_count;
    (b.base_length() == 1 && n > 0).then(|| b.base_offset() % n)
}

/// The wait since `queued`, unless it is unknown (0).
fn wait(t: i64, queued: i64) -> Option<i64> {
    (queued != 0).then(|| t.saturating_sub(queued))
}

pub(crate) fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        let _ = write!(s, "{x:02x}");
    }
    s
}
