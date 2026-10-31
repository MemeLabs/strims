// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Chunk streams: [`ChunkReader`] reassembles messages from chunks,
//! [`ChunkWriter`] splits messages into chunks, and [`Control`] carries
//! the protocol control messages both ends of a connection act on.
//!
//! A chunk is a basic header (format and chunk stream id), a message
//! header whose length the format sets (11, 7, 3 or 0 bytes), an extended
//! timestamp when the header's timestamp field is `0xffffff`, and up to a
//! chunk size of payload. A format 3 chunk carries the extended timestamp
//! whenever the chunk stream's last timestamp field was extended. Format 0
//! carries an absolute timestamp; formats 1 and 2 carry a delta, and a
//! format 3 chunk that starts a message repeats the last field as a delta.

use std::collections::{BTreeMap, VecDeque};

use super::RtmpError;

/// The chunk stream of protocol control messages.
pub(crate) const PROTOCOL_CSID: u32 = 2;
/// The chunk size before a Set Chunk Size.
pub(crate) const DEFAULT_CHUNK_SIZE: u32 = 128;
/// The largest chunk size: the top bit of a Set Chunk Size is reserved.
pub(crate) const MAX_CHUNK_SIZE: u32 = 0x7fff_ffff;
/// The largest chunk stream id a 3-byte basic header holds.
pub(crate) const MAX_CSID: u32 = 64 + 0xffff;
/// A timestamp field that defers to the extended timestamp.
const EXTENDED: u32 = 0xff_ffff;
/// The largest message length a message header holds.
const MAX_LENGTH: usize = 0xff_ffff;
/// The message header length of each chunk format.
const MESSAGE_HEADER_LEN: [usize; 4] = [11, 7, 3, 0];
/// A 3-byte basic header, an 11-byte message header and an extended
/// timestamp.
const MAX_HEADER_LEN: usize = 3 + 11 + 4;

pub(crate) const SET_CHUNK_SIZE: u8 = 1;
pub(crate) const ABORT: u8 = 2;
pub(crate) const ACKNOWLEDGEMENT: u8 = 3;
pub(crate) const USER_CONTROL: u8 = 4;
pub(crate) const WINDOW_ACK_SIZE: u8 = 5;
pub(crate) const SET_PEER_BANDWIDTH: u8 = 6;
pub(crate) const AUDIO: u8 = 8;
pub(crate) const VIDEO: u8 = 9;
/// AMF3 data and command messages: a format byte, then values.
pub(crate) const DATA_AMF3: u8 = 15;
pub(crate) const COMMAND_AMF3: u8 = 17;
pub(crate) const DATA_AMF0: u8 = 18;
pub(crate) const COMMAND_AMF0: u8 = 20;
pub(crate) const AGGREGATE: u8 = 22;

/// User control event types.
/// User control Stream Begin: its data is the message stream's id.
pub(crate) const STREAM_BEGIN: u16 = 0;
pub(crate) const PING_REQUEST: u16 = 6;
pub(crate) const PING_RESPONSE: u16 = 7;

/// The bounds a [`ChunkReader`] enforces on peer input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Limits {
    /// The largest audio, video or aggregate message.
    pub(crate) max_media: usize,
    /// The largest message of another type.
    pub(crate) max_command: usize,
    /// The capacity all reassembly buffers may hold together.
    pub(crate) max_partial: usize,
    /// The most chunk streams.
    pub(crate) max_streams: usize,
}

impl Limits {
    fn max_message(&self, type_id: u8) -> usize {
        match type_id {
            AUDIO | VIDEO | AGGREGATE => self.max_media,
            _ => self.max_command,
        }
    }
}

/// A chunk stream's last header and its message in progress.
#[derive(Default)]
struct ChunkStream {
    /// The timestamp of the current or last message.
    timestamp: u32,
    /// The last timestamp value: absolute after format 0, a delta after
    /// formats 1-3.
    delta: u32,
    /// Whether the last timestamp field was extended.
    extended: bool,
    length: u32,
    type_id: u8,
    stream_id: u32,
    /// Bytes of the current message still to arrive; 0 between messages.
    left: u32,
    /// The current message's bytes. Growth stops at the message's length;
    /// the capacity stays for later messages until reclaimed.
    buf: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
enum State {
    Header,
    /// Bytes of the current chunk's payload still to arrive.
    Payload {
        csid: u32,
        left: usize,
    },
}

/// What a [`ChunkReader::read`] stopped at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The input ended.
    None,
    /// The peer's window is full: acknowledge this sequence number.
    Ack(u32),
    /// A message is complete; [`ChunkReader::message`] holds it.
    Message,
}

/// A complete message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Message<'a> {
    pub(crate) timestamp: u32,
    pub(crate) type_id: u8,
    pub(crate) stream_id: u32,
    pub(crate) body: &'a [u8],
}

/// Reassembles messages from the chunks of one direction of a
/// connection.
pub(crate) struct ChunkReader {
    limits: Limits,
    chunk_size: usize,
    /// At most `limits.max_streams` entries.
    streams: BTreeMap<u32, ChunkStream>,
    /// The capacity of every stream's buffer, at most `limits.max_partial`.
    allocated: usize,
    /// A header as it arrives.
    header: [u8; MAX_HEADER_LEN],
    header_len: usize,
    state: State,
    /// Bytes read.
    received: u64,
    /// The peer's acknowledgement window; 0 sends none.
    window: u64,
    /// `received` at the last acknowledgement.
    acked: u64,
    ack_due: Option<u32>,
    /// A complete message not yet returned.
    completed: Option<u32>,
    /// The chunk stream holding the message `read` last returned.
    delivered: Option<u32>,
    /// Loop iterations, for the linear-work tests.
    #[cfg(test)]
    pub(crate) steps: u64,
}

fn u24(b: &[u8], at: usize) -> u32 {
    match b.get(at..at + 3) {
        Some(&[x, y, z]) => u32::from_be_bytes([0, x, y, z]),
        _ => 0,
    }
}

fn u32_be(b: &[u8], at: usize) -> u32 {
    match b.get(at..at + 4) {
        Some(&[w, x, y, z]) => u32::from_be_bytes([w, x, y, z]),
        _ => 0,
    }
}

fn u32_le(b: &[u8], at: usize) -> u32 {
    match b.get(at..at + 4) {
        Some(&[w, x, y, z]) => u32::from_le_bytes([w, x, y, z]),
        _ => 0,
    }
}

/// The chunk stream id of a basic header and the header's length, once
/// `h` holds it.
fn basic_header(h: &[u8]) -> Option<(u32, usize)> {
    let b0 = *h.first()?;
    match b0 & 0x3f {
        0 => Some((64 + u32::from(*h.get(1)?), 2)),
        1 => Some((64 + u32::from(*h.get(1)?) + 256 * u32::from(*h.get(2)?), 3)),
        id => Some((u32::from(id), 1)),
    }
}

impl ChunkReader {
    pub(crate) fn new(limits: Limits) -> Self {
        ChunkReader {
            limits,
            chunk_size: DEFAULT_CHUNK_SIZE as usize,
            streams: BTreeMap::new(),
            allocated: 0,
            header: [0; MAX_HEADER_LEN],
            header_len: 0,
            state: State::Header,
            received: 0,
            window: 0,
            acked: 0,
            ack_due: None,
            completed: None,
            delivered: None,
            #[cfg(test)]
            steps: 0,
        }
    }

    /// Reads chunks until the input ends, an acknowledgement is due or a
    /// message completes; returns the bytes consumed and which. The
    /// message [`message`](Self::message) returns is released by the next
    /// call.
    pub(crate) fn read(&mut self, input: &[u8]) -> Result<(usize, Outcome), RtmpError> {
        if let Some(csid) = self.delivered.take()
            && let Some(s) = self.streams.get_mut(&csid)
        {
            s.buf.clear();
        }
        let mut pos = 0;
        while self.ack_due.is_none() && self.completed.is_none() {
            #[cfg(test)]
            {
                self.steps += 1;
            }
            let rest = input.get(pos..).unwrap_or_default();
            if rest.is_empty() {
                break;
            }
            pos += match self.state {
                State::Header => self.read_header(rest)?,
                State::Payload { csid, left } => self.read_payload(csid, left, rest)?,
            };
        }
        let outcome = if let Some(seq) = self.ack_due.take() {
            Outcome::Ack(seq)
        } else if let Some(csid) = self.completed.take() {
            self.delivered = Some(csid);
            Outcome::Message
        } else {
            Outcome::None
        };
        Ok((pos, outcome))
    }

    /// The message the last [`read`](Self::read) returned.
    pub(crate) fn message(&self) -> Option<Message<'_>> {
        let s = self.streams.get(&self.delivered?)?;
        Some(Message {
            timestamp: s.timestamp,
            type_id: s.type_id,
            stream_id: s.stream_id,
            body: &s.buf,
        })
    }

    /// Sets the peer's chunk size, at most the largest message.
    pub(crate) fn set_chunk_size(&mut self, size: u32) {
        let size = usize::try_from(size).unwrap_or(usize::MAX);
        self.chunk_size = size.clamp(1, self.limits.max_media.max(self.limits.max_command));
    }

    /// Drops the message in progress on `csid`.
    pub(crate) fn abort(&mut self, csid: u32) {
        if let Some(s) = self.streams.get_mut(&csid)
            && s.left > 0
        {
            s.left = 0;
            s.buf.clear();
        }
    }

    /// Acknowledges every `window` bytes read from now on; 0 stops.
    pub(crate) fn set_window(&mut self, window: u32) {
        self.window = u64::from(window);
    }

    fn header_need(&self) -> usize {
        let h = self.header.get(..self.header_len).unwrap_or_default();
        let Some((csid, basic)) = basic_header(h) else {
            return h.first().map_or(1, |b| if b & 0x3f == 0 { 2 } else { 3 });
        };
        let fmt = usize::from(self.header[0] >> 6);
        let base = basic + MESSAGE_HEADER_LEN.get(fmt).copied().unwrap_or(0);
        if h.len() < base {
            return base;
        }
        let extended = if fmt == 3 {
            self.streams.get(&csid).is_some_and(|s| s.extended)
        } else {
            u24(h, basic) == EXTENDED
        };
        base + if extended { 4 } else { 0 }
    }

    fn read_header(&mut self, input: &[u8]) -> Result<usize, RtmpError> {
        let mut pos = 0;
        let need = loop {
            let need = self.header_need();
            if self.header_len >= need {
                break need;
            }
            let rest = input.get(pos..).unwrap_or_default();
            let n = (need - self.header_len).min(rest.len());
            if n == 0 {
                self.received += pos as u64;
                return Ok(pos);
            }
            if let (Some(dst), Some(src)) = (
                self.header.get_mut(self.header_len..self.header_len + n),
                rest.get(..n),
            ) {
                dst.copy_from_slice(src);
            }
            self.header_len += n;
            pos += n;
        };
        self.received += pos as u64;
        self.header_len = 0;
        self.apply_header(need)?;
        Ok(pos)
    }

    /// Applies a complete header of `len` bytes.
    fn apply_header(&mut self, len: usize) -> Result<(), RtmpError> {
        let h = self.header;
        let Some((csid, basic)) = basic_header(&h) else {
            return Ok(());
        };
        let fmt = h[0] >> 6;
        if !self.streams.contains_key(&csid) {
            if fmt != 0 {
                return Err(RtmpError::NoPreviousHeader(csid));
            }
            if self.streams.len() >= self.limits.max_streams {
                return Err(RtmpError::TooManyChunkStreams(self.limits.max_streams));
            }
            self.streams.insert(csid, ChunkStream::default());
        }
        let Some(s) = self.streams.get_mut(&csid) else {
            return Err(RtmpError::NoPreviousHeader(csid));
        };
        let base = basic
            + MESSAGE_HEADER_LEN
                .get(usize::from(fmt))
                .copied()
                .unwrap_or(0);
        let ext = (len == base + 4).then(|| u32_be(&h, base));
        if fmt < 3 && s.left > 0 {
            return Err(RtmpError::Interrupted(csid));
        }
        match fmt {
            0 => {
                let field = u24(&h, basic);
                s.length = u24(&h, basic + 3);
                s.type_id = h.get(basic + 6).copied().unwrap_or(0);
                s.stream_id = u32_le(&h, basic + 7);
                s.extended = field == EXTENDED;
                s.timestamp = ext.unwrap_or(field);
                s.delta = s.timestamp;
            }
            1 | 2 => {
                let field = u24(&h, basic);
                if fmt == 1 {
                    s.length = u24(&h, basic + 3);
                    s.type_id = h.get(basic + 6).copied().unwrap_or(0);
                }
                s.extended = field == EXTENDED;
                s.delta = ext.unwrap_or(field);
                s.timestamp = s.timestamp.wrapping_add(s.delta);
            }
            _ if s.left > 0 => {
                let left = usize::try_from(s.left).unwrap_or(usize::MAX);
                self.state = State::Payload {
                    csid,
                    left: left.min(self.chunk_size),
                };
                return Ok(());
            }
            _ => {
                if let Some(delta) = ext {
                    s.delta = delta;
                }
                s.timestamp = s.timestamp.wrapping_add(s.delta);
            }
        }
        let max = self.limits.max_message(s.type_id);
        let length = usize::try_from(s.length).unwrap_or(usize::MAX);
        if length > max {
            return Err(RtmpError::MessageTooLarge {
                type_id: s.type_id,
                len: s.length,
                max,
            });
        }
        s.left = s.length;
        if length == 0 {
            self.completed = Some(csid);
            self.end_chunk();
        } else {
            self.state = State::Payload {
                csid,
                left: length.min(self.chunk_size),
            };
        }
        Ok(())
    }

    fn read_payload(&mut self, csid: u32, left: usize, input: &[u8]) -> Result<usize, RtmpError> {
        let n = left.min(input.len());
        self.reserve(csid, n)?;
        let Some(s) = self.streams.get_mut(&csid) else {
            return Err(RtmpError::NoPreviousHeader(csid));
        };
        s.buf.extend_from_slice(input.get(..n).unwrap_or_default());
        s.left = s.left.saturating_sub(u32::try_from(n).unwrap_or(u32::MAX));
        let complete = s.left == 0;
        self.received += n as u64;
        if n < left {
            self.state = State::Payload {
                csid,
                left: left - n,
            };
        } else {
            self.state = State::Header;
            if complete {
                self.completed = Some(csid);
            }
            self.end_chunk();
        }
        Ok(n)
    }

    /// Makes room for `n` more bytes on `csid`, growing its buffer
    /// geometrically up to the message's length.
    fn reserve(&mut self, csid: u32, n: usize) -> Result<(), RtmpError> {
        let Some(s) = self.streams.get(&csid) else {
            return Ok(());
        };
        let need = s.buf.len() + n;
        let cap = s.buf.capacity();
        if need <= cap {
            return Ok(());
        }
        let length = usize::try_from(s.length).unwrap_or(usize::MAX);
        let target = need.max(cap.saturating_mul(2).min(length));
        let grow = target - cap;
        if self.allocated + grow > self.limits.max_partial {
            self.reclaim();
            if self.allocated + grow > self.limits.max_partial {
                return Err(RtmpError::PartialTooLarge(self.limits.max_partial));
            }
        }
        if let Some(s) = self.streams.get_mut(&csid) {
            let len = s.buf.len();
            s.buf.reserve_exact(target - len);
            self.allocated = self.allocated - cap + s.buf.capacity();
        }
        Ok(())
    }

    /// Frees the buffers of chunk streams between messages.
    fn reclaim(&mut self) {
        for s in self.streams.values_mut() {
            #[cfg(test)]
            {
                self.steps += 1;
            }
            if s.left == 0 {
                self.allocated -= s.buf.capacity();
                s.buf = Vec::new();
            }
        }
    }

    fn end_chunk(&mut self) {
        if self.window > 0 && self.received - self.acked >= self.window {
            self.acked = self.received;
            // Sequence numbers wrap at 2^32.
            self.ack_due = Some(self.received as u32);
        }
    }

    #[cfg(test)]
    pub(crate) fn allocated(&self) -> usize {
        self.allocated
    }
}

/// A protocol control or user control message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Control {
    SetChunkSize(u32),
    Abort(u32),
    WindowAckSize(u32),
    Ping(u32),
    /// Acknowledgement, Set Peer Bandwidth and other user control events.
    Ignored,
}

impl Control {
    /// Reads a control message; `None` for another type.
    pub(crate) fn parse(type_id: u8, body: &[u8]) -> Result<Option<Control>, RtmpError> {
        let word = |at, name| {
            body.get(at..at + 4)
                .map(|_| u32_be(body, at))
                .ok_or(RtmpError::Truncated(name))
        };
        Ok(Some(match type_id {
            SET_CHUNK_SIZE => {
                let size = word(0, "Set Chunk Size")?;
                if size == 0 || size > MAX_CHUNK_SIZE {
                    return Err(RtmpError::ChunkSize(size));
                }
                Control::SetChunkSize(size)
            }
            ABORT => Control::Abort(word(0, "Abort")?),
            WINDOW_ACK_SIZE => Control::WindowAckSize(word(0, "Window Acknowledgement Size")?),
            USER_CONTROL => match body.get(..2) {
                Some(&[a, b]) if u16::from_be_bytes([a, b]) == PING_REQUEST => {
                    Control::Ping(word(2, "Ping Request")?)
                }
                Some(_) => Control::Ignored,
                None => return Err(RtmpError::Truncated("User Control")),
            },
            ACKNOWLEDGEMENT | SET_PEER_BANDWIDTH => Control::Ignored,
            _ => return Ok(None),
        }))
    }

    /// Acts on the message: the reader takes chunk sizes, aborts and
    /// windows, the writer answers pings.
    pub(crate) fn apply(
        self,
        reader: &mut ChunkReader,
        writer: &mut ChunkWriter,
    ) -> Result<(), RtmpError> {
        match self {
            Control::SetChunkSize(size) => reader.set_chunk_size(size),
            Control::Abort(csid) => reader.abort(csid),
            Control::WindowAckSize(window) => reader.set_window(window),
            Control::Ping(timestamp) => {
                let [a, b] = PING_RESPONSE.to_be_bytes();
                let [c, d, e, f] = timestamp.to_be_bytes();
                writer.write(PROTOCOL_CSID, 0, USER_CONTROL, 0, &[a, b, c, d, e, f])?;
            }
            Control::Ignored => {}
        }
        Ok(())
    }
}

/// The last header written on a chunk stream.
#[derive(Debug, Clone, Copy)]
struct Last {
    timestamp: u32,
    delta: u32,
    extended: bool,
    length: usize,
    type_id: u8,
    stream_id: u32,
}

/// Splits messages into chunks on a bounded output queue.
pub(crate) struct ChunkWriter {
    chunk_size: usize,
    /// The most bytes `out` holds.
    max_queued: usize,
    out: VecDeque<u8>,
    /// Whether messages after the first on a chunk stream take formats
    /// 1-3 when their header allows.
    compress: bool,
    /// The last header per chunk stream the owner writes on.
    last: BTreeMap<u32, Last>,
}

fn basic_len(csid: u32) -> usize {
    match csid {
        0..64 => 1,
        64..320 => 2,
        _ => 3,
    }
}

fn push_basic(out: &mut VecDeque<u8>, fmt: u8, csid: u32) {
    let fmt = fmt << 6;
    match csid {
        0..64 => out.push_back(fmt | csid as u8),
        64..320 => out.extend([fmt, (csid - 64) as u8]),
        _ => {
            let id = csid - 64;
            out.extend([fmt | 1, id as u8, (id >> 8) as u8]);
        }
    }
}

fn push_u24(out: &mut VecDeque<u8>, v: u32) {
    let [_, a, b, c] = v.min(EXTENDED).to_be_bytes();
    out.extend([a, b, c]);
}

impl ChunkWriter {
    pub(crate) fn new(max_queued: usize, compress: bool) -> Self {
        ChunkWriter {
            chunk_size: DEFAULT_CHUNK_SIZE as usize,
            max_queued,
            out: VecDeque::new(),
            compress,
            last: BTreeMap::new(),
        }
    }

    /// Chunks later messages at `size`, from 1 to [`MAX_CHUNK_SIZE`].
    pub(crate) fn set_chunk_size(&mut self, size: u32) {
        let size = size.clamp(1, MAX_CHUNK_SIZE);
        self.chunk_size = usize::try_from(size).unwrap_or(usize::MAX);
    }

    /// Bytes queued for [`poll_transmit`](Self::poll_transmit).
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn queued(&self) -> usize {
        self.out.len()
    }

    fn room(&self, n: usize) -> Result<(), RtmpError> {
        if self.out.len().saturating_add(n) > self.max_queued {
            return Err(RtmpError::OutputFull(self.max_queued));
        }
        Ok(())
    }

    /// Queues bytes outside the chunk layer: the handshake.
    pub(crate) fn push_raw(&mut self, bytes: &[u8]) -> Result<(), RtmpError> {
        self.room(bytes.len())?;
        self.out.extend(bytes);
        Ok(())
    }

    /// Queues a message on `csid` (2 to [`MAX_CSID`]).
    pub(crate) fn write(
        &mut self,
        csid: u32,
        timestamp: u32,
        type_id: u8,
        stream_id: u32,
        body: &[u8],
    ) -> Result<(), RtmpError> {
        let csid = csid.clamp(PROTOCOL_CSID, MAX_CSID);
        let len = body.len();
        if len > MAX_LENGTH {
            return Err(RtmpError::MessageTooLarge {
                type_id,
                len: u32::try_from(len).unwrap_or(u32::MAX),
                max: MAX_LENGTH,
            });
        }
        let last = self.last.get(&csid).copied().filter(|_| self.compress);
        let (fmt, field) = match last {
            Some(l) if l.stream_id == stream_id && timestamp >= l.timestamp => {
                let delta = timestamp - l.timestamp;
                if l.length != len || l.type_id != type_id {
                    (1, delta)
                } else if l.delta != delta {
                    (2, delta)
                } else {
                    (3, delta)
                }
            }
            _ => (0, timestamp),
        };
        let extended = match (fmt, last) {
            (3, Some(l)) => l.extended,
            _ => field >= EXTENDED,
        };
        let ext_len = if extended { 4 } else { 0 };
        let chunks = len.div_ceil(self.chunk_size).max(1);
        let total = basic_len(csid)
            + MESSAGE_HEADER_LEN
                .get(usize::from(fmt))
                .copied()
                .unwrap_or(0)
            + (chunks - 1) * basic_len(csid)
            + chunks * ext_len
            + len;
        self.room(total)?;

        push_basic(&mut self.out, fmt, csid);
        if fmt < 3 {
            push_u24(&mut self.out, field);
        }
        if fmt < 2 {
            push_u24(&mut self.out, len as u32);
            self.out.push_back(type_id);
        }
        if fmt == 0 {
            self.out.extend(stream_id.to_le_bytes());
        }
        if extended {
            self.out.extend(field.to_be_bytes());
        }
        for (i, chunk) in body.chunks(self.chunk_size).enumerate() {
            if i > 0 {
                push_basic(&mut self.out, 3, csid);
                if extended {
                    self.out.extend(field.to_be_bytes());
                }
            }
            self.out.extend(chunk);
        }
        if self.compress {
            self.last.insert(
                csid,
                Last {
                    timestamp,
                    delta: field,
                    extended,
                    length: len,
                    type_id,
                    stream_id,
                },
            );
        }
        Ok(())
    }

    /// Moves queued bytes into `buf`; returns how many.
    pub(crate) fn poll_transmit(&mut self, buf: &mut [u8]) -> usize {
        let n = buf.len().min(self.out.len());
        for (dst, src) in buf.iter_mut().zip(self.out.drain(..n)) {
            *dst = src;
        }
        n
    }
}

#[cfg(test)]
#[path = "chunk_test.rs"]
mod tests;
