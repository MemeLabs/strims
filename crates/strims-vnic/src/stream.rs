// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Stream framing: a protobuf message prefixed with its length as a
//! big-endian `u16`. The vnic handshakes (`TCPMuxInit`, `AESLinkInit`,
//! `PeerInit`) use it.

/// Prefix size.
pub const STREAM_HEADER_LEN: usize = 2;

/// The largest message body written: prefix and body together stay within
/// 65535 bytes.
pub const MAX_STREAM_MESSAGE_LEN: usize = u16::MAX as usize - STREAM_HEADER_LEN;

/// Errors from the stream framing.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StreamError {
    /// The message is longer than [`MAX_STREAM_MESSAGE_LEN`].
    #[error("buffer too small")]
    MessageTooLarge,
    /// The body isn't a valid message.
    #[error("proto unmarshal failed: {0}")]
    Decode(String),
}

/// Appends `m` with its length prefix.
pub fn encode_stream_message(
    m: &impl prost::Message,
    out: &mut Vec<u8>,
) -> Result<(), StreamError> {
    let len = m.encoded_len();
    if len > MAX_STREAM_MESSAGE_LEN {
        return Err(StreamError::MessageTooLarge);
    }
    // invariant: checked against MAX_STREAM_MESSAGE_LEN above.
    let prefix = u16::try_from(len).map_err(|_| StreamError::MessageTooLarge)?;
    out.reserve(STREAM_HEADER_LEN + len);
    out.extend_from_slice(&prefix.to_be_bytes());
    m.encode_raw(out);
    Ok(())
}

/// Decodes a message body read by [`MessageReader`].
pub fn decode_stream_message<M: prost::Message + Default>(body: &[u8]) -> Result<M, StreamError> {
    M::decode(body).map_err(|e| StreamError::Decode(e.to_string()))
}

/// Reads one length-prefixed message from a byte stream fed in chunks.
/// Buffers at most `2 + 65535` bytes, and only when the message is split
/// across chunks.
#[derive(Debug, Default)]
pub struct MessageReader {
    header: [u8; STREAM_HEADER_LEN],
    header_len: usize,
    /// Body of a message split across chunks. Bounded by `u16::MAX`.
    body: Vec<u8>,
    want: Option<usize>,
}

/// The outcome of [`MessageReader::push`].
#[derive(Debug, PartialEq, Eq)]
pub struct Pushed<'a> {
    /// Input bytes consumed. Nothing past the end of a completed message is
    /// consumed.
    pub consumed: usize,
    /// The message body, once complete.
    pub message: Option<&'a [u8]>,
}

impl MessageReader {
    pub fn new() -> MessageReader {
        MessageReader::default()
    }

    /// Bytes held from a message split across chunks.
    pub fn buffered(&self) -> usize {
        self.header_len + self.body.len()
    }

    /// Input bytes needed before the reader can make progress past its
    /// current field (the 2-byte prefix, then the body).
    pub fn needed(&self) -> usize {
        match self.want {
            None => STREAM_HEADER_LEN - self.header_len,
            Some(want) => want - self.body.len(),
        }
    }

    /// Consumes input up to the end of the current message. The message is
    /// lent from `data` when it lies entirely inside it.
    pub fn push<'a>(&'a mut self, data: &'a [u8]) -> Pushed<'a> {
        let mut consumed = 0;
        if self.want.is_none() {
            if self.header_len == 0
                && let Some(&[a, b]) = data.get(..STREAM_HEADER_LEN)
            {
                let want = usize::from(u16::from_be_bytes([a, b]));
                if let Some(body) = data.get(STREAM_HEADER_LEN..STREAM_HEADER_LEN + want) {
                    return Pushed {
                        consumed: STREAM_HEADER_LEN + want,
                        message: Some(body),
                    };
                }
            }
            let k = (STREAM_HEADER_LEN - self.header_len).min(data.len());
            if let (Some(dst), Some(src)) = (
                self.header.get_mut(self.header_len..self.header_len + k),
                data.get(..k),
            ) {
                dst.copy_from_slice(src);
            }
            self.header_len += k;
            consumed += k;
            if self.header_len < STREAM_HEADER_LEN {
                return Pushed {
                    consumed,
                    message: None,
                };
            }
            let want = usize::from(u16::from_be_bytes(self.header));
            self.body.clear();
            self.body.reserve(want);
            self.want = Some(want);
        }
        let want = self.want.unwrap_or(0);
        let rest = data.get(consumed..).unwrap_or_default();
        let k = (want - self.body.len()).min(rest.len());
        self.body
            .extend_from_slice(rest.get(..k).unwrap_or_default());
        consumed += k;
        if self.body.len() < want {
            return Pushed {
                consumed,
                message: None,
            };
        }
        self.header_len = 0;
        self.want = None;
        Pushed {
            consumed,
            message: Some(&self.body),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use strims_proto::vnic::v1::TcpMuxInit;

    #[test]
    fn round_trip_at_every_split() {
        let m = TcpMuxInit {
            protocol_version: 1,
            peer_key: vec![7; 32],
        };
        let mut wire = Vec::new();
        encode_stream_message(&m, &mut wire).unwrap();
        assert_eq!(&wire[..2], &[0, 36]);
        wire.extend_from_slice(b"trailing");
        for split in 0..=wire.len() {
            let mut r = MessageReader::new();
            let mut off = 0;
            let mut got = None;
            for chunk in [&wire[..split], &wire[split..]] {
                let mut c = chunk;
                while got.is_none() && !c.is_empty() {
                    let p = r.push(c);
                    off += p.consumed;
                    c = &c[p.consumed..];
                    if let Some(body) = p.message {
                        got = Some(decode_stream_message::<TcpMuxInit>(body).unwrap());
                    }
                }
            }
            assert_eq!(got.as_ref(), Some(&m), "split {split}");
            assert_eq!(&wire[off..], b"trailing");
        }
    }

    #[test]
    fn rejects_oversize() {
        let m = TcpMuxInit {
            protocol_version: 1,
            peer_key: vec![0; MAX_STREAM_MESSAGE_LEN],
        };
        assert_eq!(
            encode_stream_message(&m, &mut Vec::new()),
            Err(StreamError::MessageTooLarge)
        );
    }

    #[test]
    fn empty_message() {
        let mut r = MessageReader::new();
        let p = r.push(&[0, 0, 9]);
        assert_eq!(p.consumed, 2);
        assert_eq!(p.message, Some(&[][..]));
        assert_eq!(r.needed(), 2);
    }
}
