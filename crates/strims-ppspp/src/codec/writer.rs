// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Encoding into caller buffers. The caller lends the frame buffer for the
//! duration of one fill (`poll_transmit`). Every write checks the space
//! first and fails with [`Error::NotEnoughSpace`] without writing anything,
//! so a failed write leaves the buffer as it was.

use super::Error;
use super::messages::{ChannelHeader, Message};
use super::wire::uvarint_len;

/// The length of a message's type byte.
pub const MESSAGE_TYPE_LEN: usize = 1;

/// Something messages can be written to: [`Writer`], [`ChannelWriter`], or
/// the channel layer's metrics wrapper. Integrity, data and scheduler
/// messages all go through it.
pub trait MessageWriter {
    /// Writes `m` with its type byte. Returns the bytes written, or
    /// [`Error::NotEnoughSpace`] having written nothing.
    fn write(&mut self, m: &Message<'_>) -> Result<usize, Error>;
}

impl<W: MessageWriter + ?Sized> MessageWriter for &mut W {
    fn write(&mut self, m: &Message<'_>) -> Result<usize, Error> {
        (**self).write(m)
    }
}

/// Writes messages back to back into a borrowed buffer.
#[derive(Debug)]
pub struct Writer<'b> {
    buf: &'b mut [u8],
    off: usize,
}

impl<'b> Writer<'b> {
    pub fn new(buf: &'b mut [u8]) -> Writer<'b> {
        Writer { buf, off: 0 }
    }

    /// Bytes written.
    pub fn len(&self) -> usize {
        self.off
    }

    /// True if nothing was written.
    pub fn is_empty(&self) -> bool {
        self.off == 0
    }

    /// Space left.
    pub fn available(&self) -> usize {
        self.buf.len().saturating_sub(self.off)
    }

    /// Discards everything written.
    pub fn reset(&mut self) {
        self.off = 0;
    }

    /// The bytes written so far.
    pub fn as_bytes(&self) -> &[u8] {
        self.buf.get(..self.off).unwrap_or_default()
    }
}

impl MessageWriter for Writer<'_> {
    fn write(&mut self, m: &Message<'_>) -> Result<usize, Error> {
        let rest = self.buf.get_mut(self.off..).ok_or(Error::NotEnoughSpace)?;
        let n = m.encode(rest)?;
        self.off += n;
        Ok(n)
    }
}

/// Writes one channel payload into a frame buffer, leaving room for its
/// [`ChannelHeader`] and filling that in on [`ChannelWriter::finish`].
/// Metrics belong to the channel layer's wrapper.
///
/// The payload is capped at `u16::MAX` bytes, the most a header can
/// describe.
#[derive(Debug)]
pub struct ChannelWriter<'b> {
    header: ChannelHeader,
    header_len: usize,
    buf: &'b mut [u8],
    off: usize,
    limit: usize,
}

impl<'b> ChannelWriter<'b> {
    pub fn new(buf: &'b mut [u8], channel: u64) -> ChannelWriter<'b> {
        let header_len = uvarint_len(channel) + 2;
        let limit = buf
            .len()
            .min(header_len.saturating_add(usize::from(u16::MAX)));
        ChannelWriter {
            header: ChannelHeader { channel, length: 0 },
            header_len,
            buf,
            off: header_len,
            limit,
        }
    }

    /// Payload bytes written.
    pub fn payload_len(&self) -> usize {
        self.off - self.header_len
    }

    /// The frame bytes [`ChannelWriter::finish`] would use: 0 if nothing
    /// was written, else header plus payload.
    pub fn len(&self) -> usize {
        if self.is_empty() { 0 } else { self.off }
    }

    pub fn is_empty(&self) -> bool {
        self.off == self.header_len
    }

    /// Payload space left.
    pub fn available(&self) -> usize {
        self.limit.saturating_sub(self.off)
    }

    /// Discards the payload.
    pub fn reset(&mut self) {
        self.off = self.header_len;
    }

    /// Writes the header and returns the bytes used in the frame buffer, or
    /// 0 (and no header) if no message was written: an empty channel
    /// payload never goes on the wire.
    pub fn finish(mut self) -> usize {
        if self.is_empty() {
            return 0;
        }
        // invariant: the payload is capped at u16::MAX by `limit`.
        self.header.length = u16::try_from(self.payload_len()).unwrap_or(u16::MAX);
        match self.header.encode(self.buf) {
            Ok(_) => self.off,
            Err(_) => 0,
        }
    }
}

impl MessageWriter for ChannelWriter<'_> {
    fn write(&mut self, m: &Message<'_>) -> Result<usize, Error> {
        let rest = self
            .buf
            .get_mut(self.off..self.limit)
            .ok_or(Error::NotEnoughSpace)?;
        let n = m.encode(rest)?;
        self.off += n;
        Ok(n)
    }
}
