// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A dump as rows: one per frame, one per ppspp message in it, and one per
//! tap decision that annotates no message. [`export`] writes them as JSON
//! lines, an object per row, leaving out fields a row doesn't have:
//!
//! | field | value |
//! |---|---|
//! | `t` | the record's time, ns since the Unix epoch |
//! | `node` | the capturing node's name, hex |
//! | `peer` | the peer's name, hex |
//! | `dir` | `in`, `out` or `tap` |
//! | `frame` | the frame's number in the dump, from 1; on a tap row, the last frame's |
//! | `swarm` | the swarm's id in the dump |
//! | `channel` | the wire channel id |
//! | `type` | `FRAME`, a message type (`DATA`, `HAVE`, `ACK`, `REQUEST`, `CANCEL`, `INTEGRITY`, `SIGNED_INTEGRITY`, `HANDSHAKE`, `RESTART`, `CHOKE`, `UNCHOKE`, `PING`, `PONG`, `STREAM_REQUEST`, `STREAM_CANCEL`, `STREAM_OPEN`, `STREAM_CLOSE`), `UNKNOWN_CHANNEL`, `MALFORMED`, `FRAME_ERROR`, `TapLost`, or a tap kind's name ([`TAP_KINDS`](crate::TAP_KINDS)) |
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
//! | `budget`, `copies`, `streams`, `count` | other fields of a decision |
//!
//! A tap row carries its decision's fields by these names. Decisions about
//! data go onto the DATA row they describe, in the frame just before them:
//! `DataSent` on DATA out, and `PushHit`, `PushLate`, `PushUnsolicited`
//! and `DataStale` on DATA in.

use std::collections::BTreeMap;
use std::io::{self, Read, Write};

use strims_binmap::Bin;
use strims_ppspp::codec::{FrameReader, Message, Reader as MessageReader, ReaderConfig};

use crate::capture::tap_kind;
use crate::format::{Dir, Record, SwarmParams, TapRecord};
use crate::reader::{Error, Reader};

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
    other: [Option<(&'static str, i64)>; 3],
}

/// One dump's state.
struct Dump<'w, W: Write> {
    out: &'w mut W,
    node: String,
    /// Bound: [`MAX_TRACKED`].
    peers: BTreeMap<u32, String>,
    /// Bound: [`MAX_TRACKED`].
    swarms: BTreeMap<u32, SwarmParams>,
    /// (peer, frames in, wire channel) to swarm. Bound: [`MAX_TRACKED`].
    channels: BTreeMap<(u32, bool, u64), u32>,
    frame: u64,
    /// The last frame's rows. Bound: [`MAX_HELD`].
    held: Vec<Row>,
    held_from: (u32, RowDir),
    stats: ExportStats,
}

/// Writes `reader`'s records as JSON lines. Records that don't decode go
/// to `on_error` and are skipped; an error that ends the dump goes there
/// too. Only write errors fail the export.
pub fn export<R: Read, W: Write>(
    reader: &mut Reader<R>,
    out: &mut W,
    on_error: &mut dyn FnMut(&Error),
) -> io::Result<ExportStats> {
    let node = hex(&reader.header().node);
    let mut d = Dump {
        out,
        node,
        peers: BTreeMap::new(),
        swarms: BTreeMap::new(),
        channels: BTreeMap::new(),
        frame: 0,
        held: Vec::new(),
        held_from: (0, RowDir::Tap),
        stats: ExportStats::default(),
    };
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

impl<W: Write> Dump<'_, W> {
    fn record(&mut self, t: i64, r: &Record<'_>) -> io::Result<()> {
        if !matches!(r, Record::Tap(_) | Record::TapLost { .. }) {
            self.flush()?;
        }
        match *r {
            Record::Peer { peer, name } => {
                if self.peers.len() < MAX_TRACKED || self.peers.contains_key(&peer) {
                    self.peers.insert(peer, hex(name));
                }
            }
            Record::Swarm { swarm, params, .. } => {
                if self.swarms.len() < MAX_TRACKED || self.swarms.contains_key(&swarm) {
                    self.swarms.insert(swarm, params);
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
                    other: [Some(("count", count as i64)), None, None],
                    ..Row::default()
                })?;
            }
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
                .and_then(|s| self.swarms.get(&s).map(|p| (s, *p)))
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

    fn flush(&mut self) -> io::Result<()> {
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
        let mut other = 0;
        let fields = info.map_or(&[][..], |k| k.fields);
        for (i, &f) in fields.iter().enumerate() {
            let v = tr.field(i);
            match f {
                "bin" => {
                    let b = Bin(v as u64);
                    row.bin = Some(b);
                    row.stream = chunk_stream(b, self.swarms.get(&tr.swarm));
                }
                "push" => row.ann = Some(if v == 1 { "push" } else { "answer" }),
                "queued" => row.wait = wait(t, v),
                "lag" => row.lag = Some(v),
                "stream" => row.stream = Some(v as u64),
                "from" => row.from = u32::try_from(v).ok(),
                f => {
                    if let Some(slot) = row.other.get_mut(other) {
                        *slot = Some((f, v));
                        other += 1;
                    }
                }
            }
        }
        self.hold(row)
    }

    fn write(&mut self, r: &Row) -> io::Result<()> {
        self.stats.rows += 1;
        let o = &mut *self.out;
        write!(o, "{{\"t\":{},\"node\":\"{}\"", r.t, self.node)?;
        if r.peer != 0 {
            match self.peers.get(&r.peer) {
                Some(p) => write!(o, ",\"peer\":\"{p}\"")?,
                None => write!(o, ",\"peer\":\"?{}\"", r.peer)?,
            }
        }
        let dir = match r.dir {
            RowDir::In => "in",
            RowDir::Out => "out",
            RowDir::Tap => "tap",
        };
        write!(o, ",\"dir\":\"{dir}\",\"frame\":{}", r.frame)?;
        if r.swarm != 0 {
            write!(o, ",\"swarm\":{}", r.swarm)?;
        }
        if let Some(c) = r.channel {
            write!(o, ",\"channel\":{c}")?;
        }
        write!(o, ",\"type\":\"{}\"", r.kind)?;
        if let Some(b) = r.bin {
            write!(
                o,
                ",\"bin\":{},\"first\":{},\"last\":{}",
                b.0,
                b.base_offset(),
                b.base_right().0 >> 1
            )?;
        }
        let nums = [
            ("stream", r.stream.map(|v| v as i64)),
            ("ts", r.ts),
            ("len", r.len.map(|v| v as i64)),
        ];
        for (k, v) in nums {
            if let Some(v) = v {
                write!(o, ",\"{k}\":{v}")?;
            }
        }
        if let Some(a) = r.ann {
            write!(o, ",\"ann\":\"{a}\"")?;
        }
        for (k, v) in [("wait", r.wait), ("lag", r.lag), ("delay", r.delay)] {
            if let Some(v) = v {
                write!(o, ",\"{k}\":{v}")?;
            }
        }
        if let Some(f) = r.from {
            match self.peers.get(&f) {
                Some(p) => write!(o, ",\"from\":\"{p}\"")?,
                None => write!(o, ",\"from\":\"?{f}\"")?,
            }
        }
        for (k, v) in r.other.iter().flatten() {
            write!(o, ",\"{k}\":{v}")?;
        }
        o.write_all(b"}\n")
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
        Message::Ping(_) => Row {
            kind: "PING",
            ..row
        },
        Message::Pong(p) => Row {
            kind: "PONG",
            delay: i64::try_from(p.delay).ok(),
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
        Message::End => Row { kind: "END", ..row },
    }
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

fn hex(b: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        let _ = write!(s, "{x:02x}");
    }
    s
}
