// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Records and their encoding; the crate docs give the layout.

use strims_base::time::Time;

/// The first bytes of a dump.
pub const MAGIC: [u8; 8] = *b"STRIMSPC";
/// The format version this crate writes.
pub const VERSION: u16 = 2;
/// The oldest version this crate reads. Version 1 is version 2 without
/// sample records and with at most 4 fields per tap record.
pub const OLDEST_VERSION: u16 = 1;
/// The most bytes a record's `len` may count.
pub const MAX_RECORD: usize = 256 * 1024;
/// The most frame body bytes a record keeps.
pub const MAX_FRAME: usize = 255 * 1024;
/// The most bytes of a node, peer or swarm name.
pub const MAX_NAME: usize = 64;
/// The most bytes of a header's label.
pub const MAX_LABEL: usize = 256;
/// The most bytes of a frame error's message.
pub const MAX_MESSAGE: usize = 256;
/// The most params a swarm record may carry.
pub const MAX_PARAMS: usize = 32;
/// The most fields a tap record carries.
pub const MAX_TAP_FIELDS: usize = 16;
/// The most fields a sample record carries.
pub const MAX_SAMPLE_FIELDS: usize = 16;

pub(crate) const KIND_HEADER: u8 = 0;
const KIND_PEER: u8 = 1;
const KIND_PEER_GONE: u8 = 2;
const KIND_SWARM: u8 = 3;
const KIND_SWARM_GONE: u8 = 4;
const KIND_CHANNEL_OPEN: u8 = 5;
const KIND_CHANNEL_CLOSE: u8 = 6;
const KIND_FRAME_IN: u8 = 7;
const KIND_FRAME_OUT: u8 = 8;
const KIND_FRAME_ERROR: u8 = 9;
const KIND_TAP: u8 = 10;
const KIND_TAP_LOST: u8 = 11;
const KIND_SAMPLE: u8 = 12;

/// The time source of a capture's records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clock {
    /// A simulation's virtual clock.
    Sim,
    /// The time a node's driver passes its runner.
    Node,
    /// A value this version doesn't know.
    Other(u8),
}

impl Clock {
    fn to_u8(self) -> u8 {
        match self {
            Clock::Sim => 1,
            Clock::Node => 2,
            Clock::Other(v) => v,
        }
    }

    fn from_u8(v: u8) -> Clock {
        match v {
            1 => Clock::Sim,
            2 => Clock::Node,
            v => Clock::Other(v),
        }
    }
}

/// A capture file's header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header<'a> {
    pub clock: Clock,
    /// When the capture started; the first record's time is relative to it.
    pub origin: Time,
    /// The file's place in a rotated capture, from 0.
    pub file: u64,
    /// The capturing node's name; the writer keeps [`MAX_NAME`] bytes.
    pub node: &'a [u8],
    /// A description of the node; the writer keeps [`MAX_LABEL`] bytes.
    pub label: &'a str,
}

/// The direction of a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    In,
    Out,
}

/// What a decoder needs to know of a swarm, and how its node ran it.
/// Fields go on the wire in this order. A zero field is unknown. The first
/// [`SwarmParams::DECODE_FIELDS`] are always present.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwarmParams {
    /// Bytes per chunk.
    pub chunk_size: u64,
    /// Bytes of an INTEGRITY hash (by the Merkle hash function).
    pub integrity_hash_size: u64,
    /// Bytes of a live signature (0 without one).
    pub integrity_signature_size: u64,
    /// Push streams: chunk `i` belongs to stream `i % stream_count`.
    pub stream_count: u64,
    pub chunks_per_signature: u64,
    /// The live window in chunks.
    pub live_window: u64,
    /// The handshake's content integrity protection method.
    pub protection_method: u64,
    /// The handshake's Merkle hash tree function.
    pub merkle_hash_tree_function: u64,
    /// The handshake's live signature algorithm.
    pub live_signature_algorithm: u64,
    /// 1 seed, 2 peer.
    pub scheduling_method: u64,
    /// 1 low latency, 2 best effort, 3 mandatory.
    pub delivery_mode: u64,
    /// 1 circular, 2 elastic.
    pub buffer_layout: u64,
    /// 1 off, 2 eager HAVE, 3 hybrid.
    pub push_mode: u64,
    /// 1 LEDBAT (2, BBR-like, in older captures).
    pub request_controller: u64,
    /// 1 admission, 2 adaptive.
    pub seed_allocation: u64,
    /// 1 equal, 2 by allocation.
    pub seed_qos: u64,
}

impl SwarmParams {
    /// The fields a swarm record must carry: those a decoder needs.
    pub const DECODE_FIELDS: usize = 6;
    const FIELDS: usize = 16;

    fn to_array(self) -> [u64; Self::FIELDS] {
        [
            self.chunk_size,
            self.integrity_hash_size,
            self.integrity_signature_size,
            self.stream_count,
            self.chunks_per_signature,
            self.live_window,
            self.protection_method,
            self.merkle_hash_tree_function,
            self.live_signature_algorithm,
            self.scheduling_method,
            self.delivery_mode,
            self.buffer_layout,
            self.push_mode,
            self.request_controller,
            self.seed_allocation,
            self.seed_qos,
        ]
    }

    fn from_array(a: [u64; Self::FIELDS]) -> SwarmParams {
        let [
            chunk_size,
            integrity_hash_size,
            integrity_signature_size,
            stream_count,
            chunks_per_signature,
            live_window,
            protection_method,
            merkle_hash_tree_function,
            live_signature_algorithm,
            scheduling_method,
            delivery_mode,
            buffer_layout,
            push_mode,
            request_controller,
            seed_allocation,
            seed_qos,
        ] = a;
        SwarmParams {
            chunk_size,
            integrity_hash_size,
            integrity_signature_size,
            stream_count,
            chunks_per_signature,
            live_window,
            protection_method,
            merkle_hash_tree_function,
            live_signature_algorithm,
            scheduling_method,
            delivery_mode,
            buffer_layout,
            push_mode,
            request_controller,
            seed_allocation,
            seed_qos,
        }
    }
}

/// One scheduler decision. The capture's writer defines the kinds and
/// their fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TapRecord {
    pub kind: u32,
    /// The peer's id, or 0.
    pub peer: u32,
    /// The swarm's id, or 0.
    pub swarm: u32,
    fields: [i64; MAX_TAP_FIELDS],
    len: u8,
}

impl TapRecord {
    /// A record with the first [`MAX_TAP_FIELDS`] of `fields`.
    pub fn new(kind: u32, peer: u32, swarm: u32, fields: &[i64]) -> TapRecord {
        let mut r = TapRecord {
            kind,
            peer,
            swarm,
            ..TapRecord::default()
        };
        for (dst, &src) in r.fields.iter_mut().zip(fields) {
            *dst = src;
            r.len += 1;
        }
        r
    }

    pub fn fields(&self) -> &[i64] {
        self.fields.get(..usize::from(self.len)).unwrap_or_default()
    }

    /// Field `i`, or 0 when the record has fewer.
    pub fn field(&self, i: usize) -> i64 {
        self.fields().get(i).copied().unwrap_or(0)
    }
}

/// One reading of the state below ppspp: a link's or its node's. The
/// capture's writer defines the sources and their fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SampleRecord {
    pub source: u32,
    /// The peer whose link it describes, or 0 for the node.
    pub peer: u32,
    fields: [i64; MAX_SAMPLE_FIELDS],
    len: u8,
}

impl SampleRecord {
    /// A record with the first [`MAX_SAMPLE_FIELDS`] of `fields`.
    pub fn new(source: u32, peer: u32, fields: &[i64]) -> SampleRecord {
        let mut r = SampleRecord {
            source,
            peer,
            ..SampleRecord::default()
        };
        for (dst, &src) in r.fields.iter_mut().zip(fields) {
            *dst = src;
            r.len += 1;
        }
        r
    }

    pub fn fields(&self) -> &[i64] {
        self.fields.get(..usize::from(self.len)).unwrap_or_default()
    }

    /// Field `i`, or 0 when the record has fewer.
    pub fn field(&self, i: usize) -> i64 {
        self.fields().get(i).copied().unwrap_or(0)
    }
}

/// A record after the header. Ids are the dump's, from 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Record<'a> {
    Peer {
        peer: u32,
        name: &'a [u8],
    },
    PeerGone {
        peer: u32,
    },
    Swarm {
        swarm: u32,
        id: &'a [u8],
        params: SwarmParams,
    },
    SwarmGone {
        swarm: u32,
    },
    ChannelOpen {
        peer: u32,
        swarm: u32,
        /// The id on frames from the peer.
        channel: u64,
        /// The id on frames to the peer.
        peer_channel: u64,
    },
    ChannelClose {
        peer: u32,
        swarm: u32,
    },
    Frame {
        dir: Dir,
        peer: u32,
        /// The frame's length; `body` holds at most [`MAX_FRAME`] bytes of it.
        len: u64,
        body: &'a [u8],
    },
    FrameError {
        peer: u32,
        message: &'a [u8],
    },
    Tap(TapRecord),
    TapLost {
        count: u64,
    },
    Sample(SampleRecord),
    /// A kind this version doesn't know.
    Unknown {
        kind: u8,
    },
}

impl Record<'_> {
    pub(crate) fn kind(&self) -> u8 {
        match self {
            Record::Peer { .. } => KIND_PEER,
            Record::PeerGone { .. } => KIND_PEER_GONE,
            Record::Swarm { .. } => KIND_SWARM,
            Record::SwarmGone { .. } => KIND_SWARM_GONE,
            Record::ChannelOpen { .. } => KIND_CHANNEL_OPEN,
            Record::ChannelClose { .. } => KIND_CHANNEL_CLOSE,
            Record::Frame { dir: Dir::In, .. } => KIND_FRAME_IN,
            Record::Frame { dir: Dir::Out, .. } => KIND_FRAME_OUT,
            Record::FrameError { .. } => KIND_FRAME_ERROR,
            Record::Tap(_) => KIND_TAP,
            Record::TapLost { .. } => KIND_TAP_LOST,
            Record::Sample(_) => KIND_SAMPLE,
            Record::Unknown { kind } => *kind,
        }
    }

    /// Appends the body, without a frame's bytes, which the writer appends
    /// from the caller's buffer. Returns those bytes.
    pub(crate) fn encode_body<'b>(&'b self, buf: &mut Vec<u8>) -> &'b [u8] {
        match *self {
            Record::Peer { peer, name } => {
                put_uvarint(buf, peer.into());
                put_bytes(buf, name, MAX_NAME);
            }
            Record::PeerGone { peer } => put_uvarint(buf, peer.into()),
            Record::Swarm { swarm, id, params } => {
                put_uvarint(buf, swarm.into());
                put_bytes(buf, id, MAX_NAME);
                put_uvarint(buf, SwarmParams::FIELDS as u64);
                for v in params.to_array() {
                    put_uvarint(buf, v);
                }
            }
            Record::SwarmGone { swarm } => put_uvarint(buf, swarm.into()),
            Record::ChannelOpen {
                peer,
                swarm,
                channel,
                peer_channel,
            } => {
                put_uvarint(buf, peer.into());
                put_uvarint(buf, swarm.into());
                put_uvarint(buf, channel);
                put_uvarint(buf, peer_channel);
            }
            Record::ChannelClose { peer, swarm } => {
                put_uvarint(buf, peer.into());
                put_uvarint(buf, swarm.into());
            }
            Record::Frame {
                peer, len, body, ..
            } => {
                put_uvarint(buf, peer.into());
                put_uvarint(buf, len);
                return body.get(..MAX_FRAME).unwrap_or(body);
            }
            Record::FrameError { peer, message } => {
                put_uvarint(buf, peer.into());
                put_bytes(buf, message, MAX_MESSAGE);
            }
            Record::Tap(t) => {
                put_uvarint(buf, t.kind.into());
                put_uvarint(buf, t.peer.into());
                put_uvarint(buf, t.swarm.into());
                put_uvarint(buf, u64::from(t.len));
                for &f in t.fields() {
                    put_svarint(buf, f);
                }
            }
            Record::TapLost { count } => put_uvarint(buf, count),
            Record::Sample(r) => {
                put_uvarint(buf, r.source.into());
                put_uvarint(buf, r.peer.into());
                put_uvarint(buf, u64::from(r.len));
                for &f in r.fields() {
                    put_svarint(buf, f);
                }
            }
            Record::Unknown { .. } => {}
        }
        &[]
    }

    /// Decodes a record body of `kind` (not the header).
    pub(crate) fn decode(kind: u8, body: &[u8]) -> Result<Record<'_>, &'static str> {
        let mut d = Dec::new(body);
        Ok(match kind {
            KIND_PEER => Record::Peer {
                peer: d.id()?,
                name: d.bytes(MAX_NAME)?,
            },
            KIND_PEER_GONE => Record::PeerGone { peer: d.id()? },
            KIND_SWARM => {
                let swarm = d.id()?;
                let id = d.bytes(MAX_NAME)?;
                let n = d.uvarint()?;
                if n > MAX_PARAMS as u64 {
                    return Err("too many swarm params");
                }
                if n < SwarmParams::DECODE_FIELDS as u64 {
                    return Err("too few swarm params");
                }
                let mut a = [0; SwarmParams::FIELDS];
                for i in 0..n as usize {
                    let v = d.uvarint()?;
                    if let Some(dst) = a.get_mut(i) {
                        *dst = v;
                    }
                }
                Record::Swarm {
                    swarm,
                    id,
                    params: SwarmParams::from_array(a),
                }
            }
            KIND_SWARM_GONE => Record::SwarmGone { swarm: d.id()? },
            KIND_CHANNEL_OPEN => Record::ChannelOpen {
                peer: d.id()?,
                swarm: d.id()?,
                channel: d.uvarint()?,
                peer_channel: d.uvarint()?,
            },
            KIND_CHANNEL_CLOSE => Record::ChannelClose {
                peer: d.id()?,
                swarm: d.id()?,
            },
            KIND_FRAME_IN | KIND_FRAME_OUT => {
                let peer = d.id()?;
                let len = d.uvarint()?;
                let body = d.rest();
                if (body.len() as u64) > len {
                    return Err("frame body longer than its length");
                }
                Record::Frame {
                    dir: if kind == KIND_FRAME_IN {
                        Dir::In
                    } else {
                        Dir::Out
                    },
                    peer,
                    len,
                    body,
                }
            }
            KIND_FRAME_ERROR => Record::FrameError {
                peer: d.id()?,
                message: d.bytes(MAX_MESSAGE)?,
            },
            KIND_TAP => {
                let kind = d.u32()?;
                let peer = d.u32()?;
                let swarm = d.u32()?;
                let n = d.uvarint()?;
                if n > MAX_TAP_FIELDS as u64 {
                    return Err("too many tap fields");
                }
                let mut fields = [0; MAX_TAP_FIELDS];
                let fields = fields.get_mut(..n as usize).unwrap_or_default();
                for f in fields.iter_mut() {
                    *f = d.svarint()?;
                }
                Record::Tap(TapRecord::new(kind, peer, swarm, fields))
            }
            KIND_TAP_LOST => Record::TapLost {
                count: d.uvarint()?,
            },
            KIND_SAMPLE => {
                let source = d.u32()?;
                let peer = d.u32()?;
                let n = d.uvarint()?;
                if n > MAX_SAMPLE_FIELDS as u64 {
                    return Err("too many sample fields");
                }
                let mut fields = [0; MAX_SAMPLE_FIELDS];
                let fields = fields.get_mut(..n as usize).unwrap_or_default();
                for f in fields.iter_mut() {
                    *f = d.svarint()?;
                }
                Record::Sample(SampleRecord::new(source, peer, fields))
            }
            KIND_HEADER => return Err("a second header"),
            kind => Record::Unknown { kind },
        })
    }
}

impl Header<'_> {
    pub(crate) fn encode_body(&self, buf: &mut Vec<u8>) {
        buf.push(self.clock.to_u8());
        buf.extend_from_slice(&self.origin.unix_nanos().to_be_bytes());
        put_uvarint(buf, self.file);
        put_bytes(buf, self.node, MAX_NAME);
        let mut n = self.label.len().min(MAX_LABEL);
        while !self.label.is_char_boundary(n) {
            n -= 1;
        }
        put_bytes(
            buf,
            self.label.as_bytes().get(..n).unwrap_or_default(),
            MAX_LABEL,
        );
    }
}

/// A decoded header body, its label not yet checked as UTF-8.
pub(crate) struct RawHeader<'a> {
    pub(crate) clock: Clock,
    pub(crate) origin: Time,
    pub(crate) file: u64,
    pub(crate) node: &'a [u8],
    pub(crate) label: &'a [u8],
}

impl RawHeader<'_> {
    pub(crate) fn decode(body: &[u8]) -> Result<RawHeader<'_>, &'static str> {
        let mut d = Dec::new(body);
        Ok(RawHeader {
            clock: Clock::from_u8(d.u8()?),
            origin: Time::from_unix_nanos(i64::from_be_bytes(d.array()?)),
            file: d.uvarint()?,
            node: d.bytes(MAX_NAME)?,
            label: d.bytes(MAX_LABEL)?,
        })
    }
}

pub(crate) fn put_uvarint(buf: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        buf.push(v as u8 | 0x80);
        v >>= 7;
    }
    buf.push(v as u8);
}

/// `v` as a uvarint in a fixed array, and its length.
pub(crate) fn uvarint_array(mut v: u64) -> ([u8; 10], usize) {
    let mut a = [0u8; 10];
    let mut n = 0;
    for dst in a.iter_mut() {
        n += 1;
        if v < 0x80 {
            *dst = v as u8;
            break;
        }
        *dst = v as u8 | 0x80;
        v >>= 7;
    }
    (a, n)
}

pub(crate) fn put_svarint(buf: &mut Vec<u8>, v: i64) {
    put_uvarint(buf, ((v << 1) ^ (v >> 63)) as u64);
}

/// Appends at most `max` bytes of `b`, length first.
fn put_bytes(buf: &mut Vec<u8>, b: &[u8], max: usize) {
    let b = b.get(..max).unwrap_or(b);
    put_uvarint(buf, b.len() as u64);
    buf.extend_from_slice(b);
}

/// A bounds-checked cursor over a record.
pub(crate) struct Dec<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Dec<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Dec<'a> {
        Dec { buf, pos: 0 }
    }

    pub(crate) fn u8(&mut self) -> Result<u8, &'static str> {
        let b = *self.buf.get(self.pos).ok_or("truncated")?;
        self.pos += 1;
        Ok(b)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], &'static str> {
        let end = self.pos.checked_add(N).ok_or("truncated")?;
        let s = self.buf.get(self.pos..end).ok_or("truncated")?;
        self.pos = end;
        s.try_into().map_err(|_| "truncated")
    }

    /// An unsigned varint of at most 10 bytes that fits a `u64`.
    pub(crate) fn uvarint(&mut self) -> Result<u64, &'static str> {
        let mut x: u64 = 0;
        for i in 0..10 {
            let b = self.u8()?;
            if b < 0x80 {
                if i == 9 && b > 1 {
                    return Err("varint overflows");
                }
                return Ok(x | u64::from(b) << (7 * i));
            }
            x |= u64::from(b & 0x7f) << (7 * i);
        }
        Err("varint overflows")
    }

    pub(crate) fn svarint(&mut self) -> Result<i64, &'static str> {
        let u = self.uvarint()?;
        Ok((u >> 1) as i64 ^ -((u & 1) as i64))
    }

    fn u32(&mut self) -> Result<u32, &'static str> {
        u32::try_from(self.uvarint()?).map_err(|_| "id past u32")
    }

    /// A peer or swarm id: nonzero.
    fn id(&mut self) -> Result<u32, &'static str> {
        match self.u32()? {
            0 => Err("zero id"),
            id => Ok(id),
        }
    }

    fn bytes(&mut self, max: usize) -> Result<&'a [u8], &'static str> {
        let n = self.uvarint()?;
        if n > max as u64 {
            return Err("string too long");
        }
        let end = self.pos.checked_add(n as usize).ok_or("truncated")?;
        let s = self.buf.get(self.pos..end).ok_or("truncated")?;
        self.pos = end;
        Ok(s)
    }

    /// The unread bytes, without consuming them.
    pub(crate) fn rest_slice(&self) -> &'a [u8] {
        self.buf.get(self.pos..).unwrap_or_default()
    }

    fn rest(&mut self) -> &'a [u8] {
        let s = self.buf.get(self.pos..).unwrap_or_default();
        self.pos = self.buf.len();
        s
    }
}
