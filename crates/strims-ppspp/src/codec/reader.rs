// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Decoding: [`FrameReader`] walks a frame's channel payloads and
//! [`Reader`] a payload's messages.
//!
//! [`Reader`] is an iterator of borrowed [`Message`]s, so the channel layer
//! can `match` on them while it holds its own borrows. A channel handles
//! each message in order and stops at the first error, which is what
//! iterating with `?` does.

use core::iter::FusedIterator;

use super::messages::{ChannelHeader, Message};
use super::{Error, MessageType};

/// The swarm parameters the decoder needs: the data, hash and signature
/// lengths are not on the wire.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReaderConfig {
    pub chunk_size: usize,
    pub integrity_hash_size: usize,
    pub integrity_signature_size: usize,
}

/// Iterator over the messages of one channel payload.
///
/// Ends at the end of the input or at [`MessageType::End`] (which it
/// consumes and doesn't yield). After an error it yields nothing more.
#[derive(Clone)]
pub struct Reader<'a> {
    config: ReaderConfig,
    buf: &'a [u8],
    pos: usize,
    done: bool,
}

impl<'a> Reader<'a> {
    pub fn new(config: ReaderConfig, buf: &'a [u8]) -> Reader<'a> {
        Reader {
            config,
            buf,
            pos: 0,
            done: false,
        }
    }

    /// The number of bytes consumed so far.
    pub fn position(&self) -> usize {
        self.pos
    }
}

impl<'a> Iterator for Reader<'a> {
    type Item = Result<Message<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let rest = self.buf.get(self.pos..).unwrap_or_default();
        if rest.is_empty() {
            self.done = true;
            return None;
        }
        match Message::decode(&self.config, rest) {
            Ok((m, n)) => {
                self.pos += n;
                if m.message_type() == MessageType::End {
                    self.done = true;
                    return None;
                }
                Some(Ok(m))
            }
            Err(e) => {
                self.done = true;
                Some(Err(e))
            }
        }
    }
}

impl FusedIterator for Reader<'_> {}

/// Iterator over the channel payloads of a frame. A header whose length
/// runs past the end of the frame is an error ([`Error::Truncated`]), after
/// which it yields nothing more.
#[derive(Clone)]
pub struct FrameReader<'a> {
    buf: &'a [u8],
    pos: usize,
    done: bool,
}

impl<'a> FrameReader<'a> {
    pub fn new(frame: &'a [u8]) -> FrameReader<'a> {
        FrameReader {
            buf: frame,
            pos: 0,
            done: false,
        }
    }
}

impl<'a> Iterator for FrameReader<'a> {
    type Item = Result<(ChannelHeader, &'a [u8]), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let rest = self.buf.get(self.pos..).unwrap_or_default();
        if rest.is_empty() {
            self.done = true;
            return None;
        }
        let item = ChannelHeader::decode(rest).and_then(|(h, n)| {
            let end = n
                .checked_add(usize::from(h.length))
                .ok_or(Error::Truncated)?;
            let payload = rest.get(n..end).ok_or(Error::Truncated)?;
            self.pos += end;
            Ok((h, payload))
        });
        if item.is_err() {
            self.done = true;
        }
        Some(item)
    }
}

impl FusedIterator for FrameReader<'_> {}
