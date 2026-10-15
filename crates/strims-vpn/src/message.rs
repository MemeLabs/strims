// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The vpn message wire format.
//!
//! A message is a fixed header, a body of `header.length` bytes and a
//! trailer with one entry per host that sent it:
//!
//! ```text
//! dst_id (32) | dst_port (2) | src_port (2) | seq (2) | length (2) | flags (2)
//! body (length bytes)
//! entry 0 (the sender) | entry 1 (first relay) | ...
//!   entry = time (8, ns big-endian) | host_id (32) | ed25519 signature (64)
//! ```
//!
//! Each entry's signature covers every byte before it, including its own
//! time and host id, so it is computed when the hop appends it. ed25519 is
//! deterministic: the same keys and times give the same bytes, which the
//! golden vectors check.
//!
//! Decoding borrows from the input ([`MessageView`]); encoding appends to a
//! caller-owned `Vec` ([`encode_message`], [`append_trailer_entry`]).

use core::fmt;

use strims_base::time::Time;
use strims_crypto::ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use strims_kademlia::{ID, ID_LENGTH};

/// Header bytes.
pub const MESSAGE_HEADER_LEN: usize = ID_LENGTH + 10;
/// The signed part of a trailer entry: time and host id.
pub const TRAILER_PAYLOAD_LEN: usize = Time::BYTE_LEN + ID_LENGTH;
/// ed25519 signature size.
pub const SIGNATURE_LEN: usize = 64;
/// One trailer entry.
pub const TRAILER_ENTRY_LEN: usize = TRAILER_PAYLOAD_LEN + SIGNATURE_LEN;
/// A message is relayed only while it has fewer trailer entries than this,
/// so it travels at most this many hops and carries at most this many
/// entries.
pub const MAX_MESSAGE_HOPS: usize = 5;
/// The sender writes a message to at most this many links; each relay to
/// this many less the entries already in the trailer.
pub const MAX_MESSAGE_REPLICAS: usize = 5;
/// Bytes a message grows by on its way: the trailer entries of every hop.
pub const MAX_TRAILER_LEN: usize = MAX_MESSAGE_HOPS * TRAILER_ENTRY_LEN;

/// The body is compressed with s2. Reserved: this crate neither sends nor
/// accepts it.
pub const M_COMPRESS: u16 = 1 << 0;
/// The body is AES-GCM encrypted for the destination (`message_filter`).
pub const M_ENCRYPT: u16 = 1 << 1;
/// Delivered at every host on the way as well as at `dst_id`.
pub const M_BROADCAST: u16 = 1 << 2;
/// Receivers don't relay: the message goes one hop.
pub const M_NORELAY: u16 = 1 << 3;
/// The flags of [`Host::send`](crate::Host::send) and
/// [`Host::send_proto`](crate::Host::send_proto).
pub const M_STD_FLAGS: u16 = M_ENCRYPT;

/// Codec errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MessageError {
    #[error("buffer too small")]
    BufferTooSmall,
    /// The body doesn't fit the 16-bit length field.
    #[error("message body of {0} bytes exceeds 65535")]
    BodyTooLong(usize),
}

/// The fixed message header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MessageHeader {
    pub dst_id: ID,
    pub dst_port: u16,
    pub src_port: u16,
    pub seq: u16,
    pub length: u16,
    pub flags: u16,
}

impl MessageHeader {
    /// The wire form.
    pub fn to_bytes(&self) -> [u8; MESSAGE_HEADER_LEN] {
        let mut b = [0u8; MESSAGE_HEADER_LEN];
        let (id, rest) = b.split_at_mut(ID_LENGTH);
        id.copy_from_slice(&self.dst_id.to_bytes());
        for (dst, v) in rest.as_chunks_mut::<2>().0.iter_mut().zip([
            self.dst_port,
            self.src_port,
            self.seq,
            self.length,
            self.flags,
        ]) {
            *dst = v.to_be_bytes();
        }
        b
    }

    /// Writes the wire form to the front of `buf`.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, MessageError> {
        buf.get_mut(..MESSAGE_HEADER_LEN)
            .ok_or(MessageError::BufferTooSmall)?
            .copy_from_slice(&self.to_bytes());
        Ok(MESSAGE_HEADER_LEN)
    }

    /// Reads a header from the front of `buf`.
    pub fn decode(buf: &[u8]) -> Result<(MessageHeader, usize), MessageError> {
        let b: &[u8; MESSAGE_HEADER_LEN] = buf
            .get(..MESSAGE_HEADER_LEN)
            .and_then(|b| b.try_into().ok())
            .ok_or(MessageError::BufferTooSmall)?;
        let (id, _) = ID::unmarshal(b).map_err(|_| MessageError::BufferTooSmall)?;
        let be16 = |off: usize| {
            let i = ID_LENGTH + off;
            u16::from_be_bytes([
                b.get(i).copied().unwrap_or(0),
                b.get(i + 1).copied().unwrap_or(0),
            ])
        };
        Ok((
            MessageHeader {
                dst_id: id,
                dst_port: be16(0),
                src_port: be16(2),
                seq: be16(4),
                length: be16(6),
                flags: be16(8),
            },
            MESSAGE_HEADER_LEN,
        ))
    }
}

/// The dedup key: seq and the sender's host id.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId(pub [u8; 2 + ID_LENGTH]);

impl MessageId {
    /// The id of message `seq` from `src`.
    pub fn new(seq: u16, src: &ID) -> MessageId {
        let mut b = [0u8; 2 + ID_LENGTH];
        let (s, id) = b.split_at_mut(2);
        s.copy_from_slice(&seq.to_be_bytes());
        id.copy_from_slice(&src.to_bytes());
        MessageId(b)
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageId({self})")
    }
}

/// One host on the message's path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrailerEntry<'a> {
    pub time: Time,
    pub host_id: ID,
    pub signature: &'a [u8; SIGNATURE_LEN],
}

impl<'a> TrailerEntry<'a> {
    /// Reads an entry from the front of `b`.
    pub fn decode(b: &'a [u8]) -> Result<TrailerEntry<'a>, MessageError> {
        let (time, _) = Time::decode(b).map_err(|_| MessageError::BufferTooSmall)?;
        let id: &[u8; ID_LENGTH] = b
            .get(Time::BYTE_LEN..TRAILER_PAYLOAD_LEN)
            .and_then(|b| b.try_into().ok())
            .ok_or(MessageError::BufferTooSmall)?;
        let signature = b
            .get(TRAILER_PAYLOAD_LEN..TRAILER_ENTRY_LEN)
            .and_then(|b| b.try_into().ok())
            .ok_or(MessageError::BufferTooSmall)?;
        Ok(TrailerEntry {
            time,
            host_id: ID::from_bytes(id),
            signature,
        })
    }
}

/// A decoded message borrowing its input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageView<'a> {
    pub header: MessageHeader,
    /// `header.length` bytes; ciphertext when [`M_ENCRYPT`] is set.
    pub body: &'a [u8],
    /// Whole trailer entries.
    trailer: &'a [u8],
    /// Header, body and trailer: what a relay forwards before adding its
    /// own entry.
    raw: &'a [u8],
}

impl<'a> MessageView<'a> {
    /// Decodes a message. Bytes after the last whole trailer entry are
    /// ignored and not relayed.
    pub fn decode(b: &'a [u8]) -> Result<MessageView<'a>, MessageError> {
        let (header, n) = MessageHeader::decode(b)?;
        let body_end = n + usize::from(header.length);
        let body = b.get(n..body_end).ok_or(MessageError::BufferTooSmall)?;
        let rest = b.get(body_end..).unwrap_or_default();
        let hops = rest.len() / TRAILER_ENTRY_LEN;
        let end = body_end + hops * TRAILER_ENTRY_LEN;
        Ok(MessageView {
            header,
            body,
            trailer: b.get(body_end..end).unwrap_or_default(),
            raw: b.get(..end).unwrap_or_default(),
        })
    }

    /// Trailer entries.
    pub fn hops(&self) -> usize {
        self.trailer.len() / TRAILER_ENTRY_LEN
    }

    /// Trailer entry `i`.
    pub fn entry(&self, i: usize) -> Option<TrailerEntry<'a>> {
        let off = i.checked_mul(TRAILER_ENTRY_LEN)?;
        TrailerEntry::decode(self.trailer.get(off..)?).ok()
    }

    /// The trailer entries in path order.
    pub fn entries(&self) -> impl Iterator<Item = TrailerEntry<'a>> + 'a {
        self.trailer
            .as_chunks::<TRAILER_ENTRY_LEN>()
            .0
            .iter()
            .filter_map(|c| TrailerEntry::decode(c).ok())
    }

    /// The host id of trailer entry `i` without decoding the rest.
    pub fn host_id(&self, i: usize) -> Option<ID> {
        let off = i.checked_mul(TRAILER_ENTRY_LEN)? + Time::BYTE_LEN;
        let b: &[u8; ID_LENGTH] = self.trailer.get(off..off + ID_LENGTH)?.try_into().ok()?;
        Some(ID::from_bytes(b))
    }

    /// The sender: trailer entry 0.
    pub fn src_host_id(&self) -> Option<ID> {
        self.host_id(0)
    }

    /// The dedup key, once the message has a sender.
    pub fn id(&self) -> Option<MessageId> {
        Some(MessageId::new(self.header.seq, &self.src_host_id()?))
    }

    /// Whether `host_id` is on the trailer.
    pub fn contains(&self, host_id: &ID) -> bool {
        (0..self.hops()).any(|i| self.host_id(i).as_ref() == Some(host_id))
    }

    /// Header, body and trailer.
    pub fn raw(&self) -> &'a [u8] {
        self.raw
    }

    /// The bytes trailer entry `hop` signs.
    pub fn signed_bytes(&self, hop: usize) -> Option<&'a [u8]> {
        let len = MESSAGE_HEADER_LEN
            + self.body.len()
            + hop.checked_mul(TRAILER_ENTRY_LEN)?
            + TRAILER_PAYLOAD_LEN;
        self.raw.get(..len)
    }

    /// Checks the signature of trailer entry `hop`. A message with no
    /// trailer (a local loopback) verifies.
    pub fn verify(&self, hop: usize) -> bool {
        self.verify_with(hop, |id| VerifyingKey::from_bytes(&id.to_bytes()).ok())
    }

    /// [`MessageView::verify`] with the verifying key supplied by `key`, so
    /// callers can cache decompressed keys.
    pub fn verify_with(&self, hop: usize, key: impl FnOnce(&ID) -> Option<VerifyingKey>) -> bool {
        if self.hops() == 0 {
            return true;
        }
        let (Some(entry), Some(msg)) = (self.entry(hop), self.signed_bytes(hop)) else {
            return false;
        };
        let Some(vk) = key(&entry.host_id) else {
            return false;
        };
        vk.verify(msg, &Signature::from_bytes(entry.signature))
            .is_ok()
    }
}

/// Appends this hop's trailer entry to the message in `buf` (which holds
/// only that message): `now`, the host id and the signature of everything
/// before the signature.
pub fn append_trailer_entry(buf: &mut Vec<u8>, now: Time, host_id: &ID, signer: &SigningKey) {
    append_entry_at(buf, 0, now, host_id, signer);
}

/// Appends a trailer entry for the message starting at `buf[start]`.
fn append_entry_at(buf: &mut Vec<u8>, start: usize, now: Time, host_id: &ID, signer: &SigningKey) {
    buf.extend_from_slice(&now.unix_nanos().to_be_bytes());
    buf.extend_from_slice(&host_id.to_bytes());
    let sig = signer.sign(buf.get(start..).unwrap_or_default());
    buf.extend_from_slice(&sig.to_bytes());
}

/// Appends a new message from this host: the header (with `length` set to
/// the body's), the body and the first trailer entry.
pub fn encode_message(
    buf: &mut Vec<u8>,
    header: &MessageHeader,
    body: &[u8],
    now: Time,
    host_id: &ID,
    signer: &SigningKey,
) -> Result<(), MessageError> {
    let length = u16::try_from(body.len()).map_err(|_| MessageError::BodyTooLong(body.len()))?;
    let start = buf.len();
    buf.reserve(MESSAGE_HEADER_LEN + body.len() + TRAILER_ENTRY_LEN);
    buf.extend_from_slice(&MessageHeader { length, ..*header }.to_bytes());
    buf.extend_from_slice(body);
    append_entry_at(buf, start, now, host_id, signer);
    Ok(())
}

/// Appends a relayed copy of `msg`: its bytes plus this hop's entry.
pub fn encode_relay(
    buf: &mut Vec<u8>,
    msg: &MessageView<'_>,
    now: Time,
    host_id: &ID,
    signer: &SigningKey,
) {
    let start = buf.len();
    buf.reserve(msg.raw().len() + TRAILER_ENTRY_LEN);
    buf.extend_from_slice(msg.raw());
    append_entry_at(buf, start, now, host_id, signer);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use strims_crypto::{key_from_seed, signing_key};

    fn host(seed: u8) -> (ID, SigningKey) {
        let k = key_from_seed(&[seed; 32]);
        let (id, _) = ID::unmarshal(&k.public).unwrap();
        (id, signing_key(&k).unwrap())
    }

    fn header() -> MessageHeader {
        MessageHeader {
            dst_id: ID([u64::MAX; 4]),
            dst_port: 101,
            src_port: 102,
            seq: 103,
            length: 4,
            flags: M_STD_FLAGS,
        }
    }

    #[test]
    fn marshal_unmarshal() {
        let (id, sk) = host(1);
        let mut b = Vec::new();
        let t = Time::from_unix_millis(1_700_000_000_123);
        encode_message(&mut b, &header(), b"test", t, &id, &sk).unwrap();
        assert_eq!(b.len(), MESSAGE_HEADER_LEN + 4 + TRAILER_ENTRY_LEN);
        let m = MessageView::decode(&b).unwrap();
        assert_eq!(m.header, header());
        assert_eq!(m.body, b"test");
        assert_eq!(m.hops(), 1);
        let e = m.entry(0).unwrap();
        assert_eq!((e.time, e.host_id), (t, id));
        assert_eq!(m.src_host_id(), Some(id));
        assert_eq!(m.id(), Some(MessageId::new(103, &id)));
        assert_eq!(m.raw(), &b[..]);
    }

    /// Two hops, both verify.
    #[test]
    fn verify_two_hops() {
        let (id0, sk0) = host(1);
        let (id1, sk1) = host(2);
        let mut b0 = Vec::new();
        encode_message(
            &mut b0,
            &header(),
            b"test",
            Time::from_unix_millis(1),
            &id0,
            &sk0,
        )
        .unwrap();
        let m0 = MessageView::decode(&b0).unwrap();
        let mut b1 = Vec::new();
        encode_relay(&mut b1, &m0, Time::from_unix_millis(2), &id1, &sk1);
        let m1 = MessageView::decode(&b1).unwrap();
        assert_eq!(m1.hops(), 2);
        assert!(m1.verify(0));
        assert!(m1.verify(1));
        assert!(!m1.verify(2));
        assert!(m1.contains(&id0) && m1.contains(&id1));
        assert!(!m1.contains(&ID::MIN));

        // Every byte is covered by hop 1, and the prefix by hop 0.
        for i in 0..b1.len() {
            let mut t = b1.clone();
            t[i] ^= 1;
            let Ok(m) = MessageView::decode(&t) else {
                continue;
            };
            if m.hops() != 2 {
                continue;
            }
            // Hop 0 covers the header, body and its own entry (signed
            // payload plus the signature itself).
            if i < MESSAGE_HEADER_LEN + 4 + TRAILER_ENTRY_LEN {
                assert!(!m.verify(0), "byte {i}");
            } else {
                assert!(m.verify(0), "byte {i}");
            }
            if i < b1.len() - SIGNATURE_LEN {
                assert!(!m.verify(1), "byte {i}");
            }
        }
    }

    #[test]
    fn decode_edge_cases() {
        assert_eq!(
            MessageView::decode(&[0u8; MESSAGE_HEADER_LEN - 1]),
            Err(MessageError::BufferTooSmall)
        );
        let mut h = header();
        h.length = 10;
        let mut b = h.to_bytes().to_vec();
        b.extend_from_slice(&[0u8; 9]);
        assert_eq!(MessageView::decode(&b), Err(MessageError::BufferTooSmall));
        b.push(0);
        let m = MessageView::decode(&b).unwrap();
        assert_eq!((m.hops(), m.src_host_id(), m.id()), (0, None, None));
        // A message without a trailer (a loopback) verifies.
        assert!(m.verify(0));
        // Trailing bytes short of an entry are ignored.
        b.extend_from_slice(&[7u8; TRAILER_ENTRY_LEN + 3]);
        let m = MessageView::decode(&b).unwrap();
        assert_eq!(m.hops(), 1);
        assert_eq!(m.raw().len(), b.len() - 3);
        assert!(!m.verify(0));
        let long = vec![0u8; 65536];
        let (id, sk) = host(1);
        assert_eq!(
            encode_message(&mut Vec::new(), &h, &long, Time::EPOCH, &id, &sk),
            Err(MessageError::BodyTooLong(65536))
        );
    }

    #[test]
    fn header_round_trip() {
        let h = MessageHeader {
            dst_id: ID([1, 2, 3, 4]),
            dst_port: 0x0102,
            src_port: 0x0304,
            seq: 0x0506,
            length: 0x0708,
            flags: 0x090a,
        };
        let mut b = [0u8; MESSAGE_HEADER_LEN];
        assert_eq!(h.encode(&mut b), Ok(MESSAGE_HEADER_LEN));
        assert_eq!(&b[32..], &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(MessageHeader::decode(&b), Ok((h, MESSAGE_HEADER_LEN)));
        assert_eq!(h.encode(&mut b[1..]), Err(MessageError::BufferTooSmall));
        assert_eq!(
            MessageId::new(0x0102, &ID([0; 4])).to_string(),
            format!("0102{}", "00".repeat(32))
        );
    }
}
