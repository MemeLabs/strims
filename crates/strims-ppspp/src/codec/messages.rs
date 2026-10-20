// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The messages and their encodings. Each message's `byte_len` is its body
//! length, without the type byte.

use core::fmt;

use strims_base::time::{Duration, Time};
use strims_binmap::Bin;

use super::reader::ReaderConfig;
use super::wire::{
    Dec, Enc, duration_from_wire, duration_to_wire, time_from_wire, time_to_wire, uvarint_len,
    varint_len,
};
use super::{Error, MessageType, ProtocolOptionType};

/// A stream index.
pub type Stream = u16;

/// The epoch handshake option: the swarm's epoch timestamp and its
/// signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EpochOption<'a> {
    pub timestamp: Time,
    pub signature: &'a [u8],
}

impl<'a> EpochOption<'a> {
    /// `None` for a nil time: a swarm without an epoch sends no epoch
    /// option.
    pub fn new(timestamp: Time, signature: &'a [u8]) -> Option<EpochOption<'a>> {
        (!timestamp.is_nil()).then_some(EpochOption {
            timestamp,
            signature,
        })
    }
}

/// One handshake option.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolOption<'a> {
    Version(u8),
    MinimumVersion(u8),
    SwarmIdentifier(&'a [u8]),
    ContentIntegrityProtectionMethod(u8),
    MerkleHashTreeFunction(u8),
    LiveSignatureAlgorithm(u8),
    LiveWindow(u32),
    ChunkSize(u32),
    ChunksPerSignature(u32),
    StreamCount(u16),
    Epoch(EpochOption<'a>),
}

impl<'a> ProtocolOption<'a> {
    pub fn option_type(&self) -> ProtocolOptionType {
        match self {
            ProtocolOption::Version(_) => ProtocolOptionType::Version,
            ProtocolOption::MinimumVersion(_) => ProtocolOptionType::MinimumVersion,
            ProtocolOption::SwarmIdentifier(_) => ProtocolOptionType::SwarmIdentifier,
            ProtocolOption::ContentIntegrityProtectionMethod(_) => {
                ProtocolOptionType::ContentIntegrityProtectionMethod
            }
            ProtocolOption::MerkleHashTreeFunction(_) => ProtocolOptionType::MerkleHashTreeFunction,
            ProtocolOption::LiveSignatureAlgorithm(_) => ProtocolOptionType::LiveSignatureAlgorithm,
            ProtocolOption::LiveWindow(_) => ProtocolOptionType::LiveWindow,
            ProtocolOption::ChunkSize(_) => ProtocolOptionType::ChunkSize,
            ProtocolOption::ChunksPerSignature(_) => ProtocolOptionType::ChunksPerSignature,
            ProtocolOption::StreamCount(_) => ProtocolOptionType::StreamCount,
            ProtocolOption::Epoch(_) => ProtocolOptionType::Epoch,
        }
    }

    /// The body length, without the type byte.
    pub fn byte_len(&self) -> usize {
        match self {
            ProtocolOption::Version(_)
            | ProtocolOption::MinimumVersion(_)
            | ProtocolOption::ContentIntegrityProtectionMethod(_)
            | ProtocolOption::MerkleHashTreeFunction(_)
            | ProtocolOption::LiveSignatureAlgorithm(_) => 1,
            ProtocolOption::StreamCount(_) => 2,
            ProtocolOption::LiveWindow(_)
            | ProtocolOption::ChunkSize(_)
            | ProtocolOption::ChunksPerSignature(_) => 4,
            ProtocolOption::SwarmIdentifier(id) => 2 + id.len(),
            ProtocolOption::Epoch(e) => varint_len(time_to_wire(e.timestamp)) + e.signature.len(),
        }
    }

    fn encode(&self, e: &mut Enc<'_>) -> Result<(), Error> {
        match *self {
            ProtocolOption::Version(v)
            | ProtocolOption::MinimumVersion(v)
            | ProtocolOption::ContentIntegrityProtectionMethod(v)
            | ProtocolOption::MerkleHashTreeFunction(v)
            | ProtocolOption::LiveSignatureAlgorithm(v) => e.u8(v),
            ProtocolOption::StreamCount(v) => e.u16(v),
            ProtocolOption::LiveWindow(v)
            | ProtocolOption::ChunkSize(v)
            | ProtocolOption::ChunksPerSignature(v) => e.u32(v),
            ProtocolOption::SwarmIdentifier(id) => {
                let n = u16::try_from(id.len()).map_err(|_| Error::SwarmIdentifierTooLong)?;
                e.u16(n)?;
                e.bytes(id)
            }
            ProtocolOption::Epoch(ep) => {
                e.varint(time_to_wire(ep.timestamp))?;
                e.bytes(ep.signature)
            }
        }
    }

    /// Decodes the body of an option of type `t`. `ChunkAddressingMethod`,
    /// `SupportedMessages` and `End` are not options with bodies.
    fn decode(
        t: ProtocolOptionType,
        d: &mut Dec<'a>,
        signature_size: usize,
    ) -> Result<ProtocolOption<'a>, Error> {
        Ok(match t {
            ProtocolOptionType::Version => ProtocolOption::Version(d.u8()?),
            ProtocolOptionType::MinimumVersion => ProtocolOption::MinimumVersion(d.u8()?),
            ProtocolOptionType::SwarmIdentifier => {
                let n = d.u16()?;
                ProtocolOption::SwarmIdentifier(d.bytes(usize::from(n))?)
            }
            ProtocolOptionType::ContentIntegrityProtectionMethod => {
                ProtocolOption::ContentIntegrityProtectionMethod(d.u8()?)
            }
            ProtocolOptionType::MerkleHashTreeFunction => {
                ProtocolOption::MerkleHashTreeFunction(d.u8()?)
            }
            ProtocolOptionType::LiveSignatureAlgorithm => {
                ProtocolOption::LiveSignatureAlgorithm(d.u8()?)
            }
            ProtocolOptionType::LiveWindow => ProtocolOption::LiveWindow(d.u32()?),
            ProtocolOptionType::ChunkSize => ProtocolOption::ChunkSize(d.u32()?),
            ProtocolOptionType::ChunksPerSignature => ProtocolOption::ChunksPerSignature(d.u32()?),
            ProtocolOptionType::StreamCount => ProtocolOption::StreamCount(d.u16()?),
            ProtocolOptionType::Epoch => {
                let timestamp = time_from_wire(d.varint()?);
                ProtocolOption::Epoch(EpochOption {
                    timestamp,
                    signature: d.bytes(signature_size)?,
                })
            }
            ProtocolOptionType::ChunkAddressingMethod
            | ProtocolOptionType::SupportedMessages
            | ProtocolOptionType::End => {
                return Err(Error::UnsupportedProtocolOption(t.as_u8()));
            }
        })
    }
}

#[derive(Clone, Copy)]
enum OptionsRepr<'a> {
    List(&'a [ProtocolOption<'a>]),
    /// Validated wire bytes, without the end marker.
    Wire {
        bytes: &'a [u8],
        signature_size: usize,
    },
}

/// A handshake's option list.
///
/// Built from a slice for encoding ([`ProtocolOptions::from_slice`]); a
/// decoded handshake keeps the validated wire bytes and parses them on
/// iteration, so decoding allocates nothing and re-encoding reproduces the
/// input exactly. Equality compares the options, not the representation.
#[derive(Clone, Copy)]
pub struct ProtocolOptions<'a>(OptionsRepr<'a>);

impl<'a> ProtocolOptions<'a> {
    pub const fn from_slice(options: &'a [ProtocolOption<'a>]) -> ProtocolOptions<'a> {
        ProtocolOptions(OptionsRepr::List(options))
    }

    pub fn iter(&self) -> ProtocolOptionsIter<'a> {
        ProtocolOptionsIter(match self.0 {
            OptionsRepr::List(l) => IterRepr::List(l.iter()),
            OptionsRepr::Wire {
                bytes,
                signature_size,
            } => IterRepr::Wire {
                dec: Dec::new(bytes),
                signature_size,
            },
        })
    }

    /// The first option of type `t`. Later duplicates are ignored.
    pub fn find(&self, t: ProtocolOptionType) -> Option<ProtocolOption<'a>> {
        self.iter().find(|o| o.option_type() == t)
    }

    /// The first value of each known option.
    pub fn summary(&self) -> HandshakeOptions<'a> {
        let mut s = HandshakeOptions::default();
        for o in self.iter() {
            match o {
                ProtocolOption::Version(v) => {
                    s.version.get_or_insert(v);
                }
                ProtocolOption::MinimumVersion(v) => {
                    s.minimum_version.get_or_insert(v);
                }
                ProtocolOption::SwarmIdentifier(v) => {
                    s.swarm_identifier.get_or_insert(v);
                }
                ProtocolOption::ContentIntegrityProtectionMethod(v) => {
                    s.content_integrity_protection_method.get_or_insert(v);
                }
                ProtocolOption::MerkleHashTreeFunction(v) => {
                    s.merkle_hash_tree_function.get_or_insert(v);
                }
                ProtocolOption::LiveSignatureAlgorithm(v) => {
                    s.live_signature_algorithm.get_or_insert(v);
                }
                ProtocolOption::LiveWindow(v) => {
                    s.live_window.get_or_insert(v);
                }
                ProtocolOption::ChunkSize(v) => {
                    s.chunk_size.get_or_insert(v);
                }
                ProtocolOption::ChunksPerSignature(v) => {
                    s.chunks_per_signature.get_or_insert(v);
                }
                ProtocolOption::StreamCount(v) => {
                    s.stream_count.get_or_insert(v);
                }
                ProtocolOption::Epoch(v) => {
                    s.epoch.get_or_insert(v);
                }
            }
        }
        s
    }

    fn byte_len(&self) -> usize {
        match self.0 {
            OptionsRepr::List(l) => l.iter().map(|o| 1 + o.byte_len()).sum(),
            OptionsRepr::Wire { bytes, .. } => bytes.len(),
        }
    }

    fn encode(&self, e: &mut Enc<'_>) -> Result<(), Error> {
        match self.0 {
            OptionsRepr::List(l) => {
                for o in l {
                    e.u8(o.option_type().as_u8())?;
                    o.encode(e)?;
                }
                Ok(())
            }
            OptionsRepr::Wire { bytes, .. } => e.bytes(bytes),
        }
    }

    /// Reads options up to and including the end marker. A list without
    /// the marker runs to the end of the input.
    fn decode(d: &mut Dec<'a>, signature_size: usize) -> Result<ProtocolOptions<'a>, Error> {
        let mut scan = Dec::new(d.rest_peek());
        let mut len = 0;
        let mut ended = false;
        while !scan.is_empty() {
            let t = scan.u8()?;
            if t == ProtocolOptionType::End.as_u8() {
                ended = true;
                break;
            }
            let t = ProtocolOptionType::from_u8(t).ok_or(Error::UnsupportedProtocolOption(t))?;
            ProtocolOption::decode(t, &mut scan, signature_size)?;
            len = scan.pos();
        }
        let bytes = d.bytes(len)?;
        if ended {
            d.u8()?;
        }
        Ok(ProtocolOptions(OptionsRepr::Wire {
            bytes,
            signature_size,
        }))
    }
}

impl PartialEq for ProtocolOptions<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl Eq for ProtocolOptions<'_> {}

impl fmt::Debug for ProtocolOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

enum IterRepr<'a> {
    List(core::slice::Iter<'a, ProtocolOption<'a>>),
    Wire { dec: Dec<'a>, signature_size: usize },
}

/// Iterator over [`ProtocolOptions`].
pub struct ProtocolOptionsIter<'a>(IterRepr<'a>);

impl<'a> Iterator for ProtocolOptionsIter<'a> {
    type Item = ProtocolOption<'a>;

    fn next(&mut self) -> Option<ProtocolOption<'a>> {
        match &mut self.0 {
            IterRepr::List(l) => l.next().copied(),
            IterRepr::Wire {
                dec,
                signature_size,
            } => {
                // The bytes were validated when the handshake was decoded.
                let t = ProtocolOptionType::from_u8(dec.u8().ok()?)?;
                ProtocolOption::decode(t, dec, *signature_size).ok()
            }
        }
    }
}

/// The first value of each known handshake option, the values handshake
/// validation reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HandshakeOptions<'a> {
    pub version: Option<u8>,
    pub minimum_version: Option<u8>,
    pub swarm_identifier: Option<&'a [u8]>,
    pub content_integrity_protection_method: Option<u8>,
    pub merkle_hash_tree_function: Option<u8>,
    pub live_signature_algorithm: Option<u8>,
    pub live_window: Option<u32>,
    pub chunk_size: Option<u32>,
    pub chunks_per_signature: Option<u32>,
    pub stream_count: Option<u16>,
    pub epoch: Option<EpochOption<'a>>,
}

/// The handshake: the sender's channel id and its options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handshake<'a> {
    pub channel_id: u32,
    pub options: ProtocolOptions<'a>,
}

impl Handshake<'_> {
    pub fn byte_len(&self) -> usize {
        4 + self.options.byte_len() + 1
    }
}

/// A chunk (or run of chunks) of the stream. `data` borrows from the
/// decoded input.
///
/// The decoder takes `bin.base_length() * chunk_size` bytes or whatever
/// remains of the payload if that is less, so `data` can be short.
/// Verifiers reject short data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Data<'a> {
    pub bin: Bin,
    pub timestamp: Time,
    pub data: &'a [u8],
}

impl Data<'_> {
    pub fn byte_len(&self) -> usize {
        uvarint_len(self.bin.0) + varint_len(time_to_wire(self.timestamp)) + self.data.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ack {
    pub bin: Bin,
    pub delay_sample: Duration,
}

/// A claim that the sender stores every chunk of `bin`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Have {
    pub bin: Bin,
}

/// Withdraws requests for the chunks of `bin`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cancel {
    pub bin: Bin,
}

/// One hash of the Merkle tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Integrity<'a> {
    pub bin: Bin,
    pub hash: &'a [u8],
}

/// A signed Merkle root, or a sign-all chunk signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignedIntegrity<'a> {
    pub bin: Bin,
    pub timestamp: Time,
    pub signature: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub bin: Bin,
    pub timestamp: Time,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ping {
    pub nonce: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pong {
    pub nonce: u64,
    pub delay: u64,
}

/// Asks the receiver to push `stream` from `bin` on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamRequest {
    pub stream: Stream,
    pub bin: Bin,
}

/// Ends a subscription to `stream`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamCancel {
    pub stream: Stream,
}

/// Accepts a subscription: the sender pushes `stream` from `bin` on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamOpen {
    pub stream: Stream,
    pub bin: Bin,
}

/// The sender is not, or no longer, pushing `stream`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamClose {
    pub stream: Stream,
}

/// The most entries an [`Announce`] holds.
pub const MAX_ANNOUNCE_ENTRIES: usize = 32;
/// The longest peer id an [`AnnounceEntry`] names.
pub const MAX_PEER_ID: usize = 32;

/// How one parent pushed one stream to the sender over its last report
/// interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnounceEntry<'a> {
    pub stream: Stream,
    /// Pushes that arrived before their deadline.
    pub hits: u64,
    /// Pushes that arrived after their chunk was released to pull.
    pub late: u64,
    /// Deadlines that passed; a late push was a miss first.
    pub misses: u64,
    /// The hits' mean push delay after their unit was first announced to
    /// the sender, in whole milliseconds on the wire.
    pub delay: Duration,
    /// The parent, by the peer id the sender's driver knows it by:
    /// 1..=[`MAX_PEER_ID`] bytes.
    pub parent: &'a [u8],
}

impl<'a> AnnounceEntry<'a> {
    /// Whether the entry can be encoded: its parent id's length is in
    /// bounds.
    pub fn is_valid(&self) -> bool {
        (1..=MAX_PEER_ID).contains(&self.parent.len())
    }

    pub fn byte_len(&self) -> usize {
        uvarint_len(u64::from(self.stream))
            + uvarint_len(self.hits)
            + uvarint_len(self.late)
            + uvarint_len(self.misses)
            + uvarint_len(delay_to_wire(self.delay))
            + 1
            + self.parent.len()
    }

    /// Writes the entry into `buf` and returns its length. Fails with
    /// [`Error::InvalidAnnounce`] for an invalid entry, and with
    /// [`Error::NotEnoughSpace`] if `buf` is shorter than
    /// [`AnnounceEntry::byte_len`], before writing anything.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, Error> {
        if !self.is_valid() {
            return Err(Error::InvalidAnnounce);
        }
        let buf = buf
            .get_mut(..self.byte_len())
            .ok_or(Error::NotEnoughSpace)?;
        let mut e = Enc::new(buf);
        self.write(&mut e)?;
        Ok(e.pos())
    }

    fn write(&self, e: &mut Enc<'_>) -> Result<(), Error> {
        let n = u8::try_from(self.parent.len()).map_err(|_| Error::InvalidAnnounce)?;
        e.uvarint(u64::from(self.stream))?;
        e.uvarint(self.hits)?;
        e.uvarint(self.late)?;
        e.uvarint(self.misses)?;
        e.uvarint(delay_to_wire(self.delay))?;
        e.u8(n)?;
        e.bytes(self.parent)
    }

    fn decode(d: &mut Dec<'a>) -> Result<AnnounceEntry<'a>, Error> {
        let stream = Stream::try_from(d.uvarint()?).map_err(|_| Error::InvalidAnnounce)?;
        let hits = d.uvarint()?;
        let late = d.uvarint()?;
        let misses = d.uvarint()?;
        let delay = delay_from_wire(d.uvarint()?);
        let n = usize::from(d.u8()?);
        if !(1..=MAX_PEER_ID).contains(&n) {
            return Err(Error::InvalidAnnounce);
        }
        Ok(AnnounceEntry {
            stream,
            hits,
            late,
            misses,
            delay,
            parent: d.bytes(n)?,
        })
    }
}

/// Whole milliseconds, negative as zero.
fn delay_to_wire(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(0)
}

/// Saturates past `i64::MAX` milliseconds.
fn delay_from_wire(ms: u64) -> Duration {
    Duration::from_millis(i64::try_from(ms).unwrap_or(i64::MAX))
}

#[derive(Clone, Copy)]
enum AnnounceRepr<'a> {
    List(&'a [AnnounceEntry<'a>]),
    /// `count` validated entries.
    Wire {
        count: u8,
        bytes: &'a [u8],
    },
}

/// A report on the sender's parents, one entry per parent and stream;
/// without entries, a request for the receiver's reports.
///
/// Built from entries for encoding ([`Announce::new`],
/// [`Announce::from_entry_bytes`]); a decoded one keeps the validated wire
/// bytes and parses them on iteration, so decoding allocates nothing and
/// re-encoding reproduces the input exactly. Equality compares the
/// entries, not the representation.
#[derive(Clone, Copy)]
pub struct Announce<'a>(AnnounceRepr<'a>);

impl<'a> Announce<'a> {
    /// The request for reports.
    pub const ASK: Announce<'static> = Announce(AnnounceRepr::List(&[]));

    /// `None` past [`MAX_ANNOUNCE_ENTRIES`] or with an invalid entry.
    pub fn new(entries: &'a [AnnounceEntry<'a>]) -> Option<Announce<'a>> {
        (entries.len() <= MAX_ANNOUNCE_ENTRIES && entries.iter().all(AnnounceEntry::is_valid))
            .then_some(Announce(AnnounceRepr::List(entries)))
    }

    /// `count` entries in `bytes`, each written by [`AnnounceEntry::encode`];
    /// `None` unless `bytes` holds exactly that.
    pub fn from_entry_bytes(count: usize, bytes: &'a [u8]) -> Option<Announce<'a>> {
        let count = u8::try_from(count)
            .ok()
            .filter(|&n| usize::from(n) <= MAX_ANNOUNCE_ENTRIES)?;
        let mut d = Dec::new(bytes);
        for _ in 0..count {
            AnnounceEntry::decode(&mut d).ok()?;
        }
        d.is_empty()
            .then_some(Announce(AnnounceRepr::Wire { count, bytes }))
    }

    pub fn len(&self) -> usize {
        match self.0 {
            AnnounceRepr::List(l) => l.len(),
            AnnounceRepr::Wire { count, .. } => usize::from(count),
        }
    }

    /// A request for reports.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> AnnounceIter<'a> {
        AnnounceIter(match self.0 {
            AnnounceRepr::List(l) => AnnounceIterRepr::List(l.iter()),
            AnnounceRepr::Wire { count, bytes } => AnnounceIterRepr::Wire {
                dec: Dec::new(bytes),
                left: count,
            },
        })
    }

    fn byte_len(&self) -> usize {
        1 + match self.0 {
            AnnounceRepr::List(l) => l.iter().map(AnnounceEntry::byte_len).sum(),
            AnnounceRepr::Wire { bytes, .. } => bytes.len(),
        }
    }

    fn encode(&self, e: &mut Enc<'_>) -> Result<(), Error> {
        let n = u8::try_from(self.len()).map_err(|_| Error::InvalidAnnounce)?;
        e.u8(n)?;
        match self.0 {
            AnnounceRepr::List(l) => l.iter().try_for_each(|x| x.write(e)),
            AnnounceRepr::Wire { bytes, .. } => e.bytes(bytes),
        }
    }

    fn decode(d: &mut Dec<'a>) -> Result<Announce<'a>, Error> {
        let count = d.u8()?;
        if usize::from(count) > MAX_ANNOUNCE_ENTRIES {
            return Err(Error::InvalidAnnounce);
        }
        let mut scan = Dec::new(d.rest_peek());
        for _ in 0..count {
            AnnounceEntry::decode(&mut scan)?;
        }
        Ok(Announce(AnnounceRepr::Wire {
            count,
            bytes: d.bytes(scan.pos())?,
        }))
    }
}

impl PartialEq for Announce<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl Eq for Announce<'_> {}

impl fmt::Debug for Announce<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

enum AnnounceIterRepr<'a> {
    List(core::slice::Iter<'a, AnnounceEntry<'a>>),
    Wire { dec: Dec<'a>, left: u8 },
}

/// Iterator over an [`Announce`]'s entries.
pub struct AnnounceIter<'a>(AnnounceIterRepr<'a>);

impl<'a> Iterator for AnnounceIter<'a> {
    type Item = AnnounceEntry<'a>;

    fn next(&mut self) -> Option<AnnounceEntry<'a>> {
        match &mut self.0 {
            AnnounceIterRepr::List(l) => l.next().copied(),
            AnnounceIterRepr::Wire { dec, left } => {
                *left = left.checked_sub(1)?;
                // The bytes were validated when the message was decoded.
                AnnounceEntry::decode(dec).ok()
            }
        }
    }
}

/// A message.
///
/// [`Reader`](super::Reader) never yields [`Message::End`]: it stops there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message<'a> {
    Handshake(Handshake<'a>),
    Restart,
    Data(Data<'a>),
    Ack(Ack),
    Have(Have),
    Integrity(Integrity<'a>),
    SignedIntegrity(SignedIntegrity<'a>),
    Request(Request),
    Cancel(Cancel),
    Choke,
    Unchoke,
    Ping(Ping),
    Pong(Pong),
    StreamRequest(StreamRequest),
    StreamCancel(StreamCancel),
    StreamOpen(StreamOpen),
    StreamClose(StreamClose),
    Announce(Announce<'a>),
    End,
}

impl<'a> Message<'a> {
    pub fn message_type(&self) -> MessageType {
        match self {
            Message::Handshake(_) => MessageType::Handshake,
            Message::Restart => MessageType::Restart,
            Message::Data(_) => MessageType::Data,
            Message::Ack(_) => MessageType::Ack,
            Message::Have(_) => MessageType::Have,
            Message::Integrity(_) => MessageType::Integrity,
            Message::SignedIntegrity(_) => MessageType::SignedIntegrity,
            Message::Request(_) => MessageType::Request,
            Message::Cancel(_) => MessageType::Cancel,
            Message::Choke => MessageType::Choke,
            Message::Unchoke => MessageType::Unchoke,
            Message::Ping(_) => MessageType::Ping,
            Message::Pong(_) => MessageType::Pong,
            Message::StreamRequest(_) => MessageType::StreamRequest,
            Message::StreamCancel(_) => MessageType::StreamCancel,
            Message::StreamOpen(_) => MessageType::StreamOpen,
            Message::StreamClose(_) => MessageType::StreamClose,
            Message::Announce(_) => MessageType::Announce,
            Message::End => MessageType::End,
        }
    }

    /// The body length, without the type byte.
    pub fn byte_len(&self) -> usize {
        match self {
            Message::Handshake(m) => m.byte_len(),
            Message::Restart | Message::Choke | Message::Unchoke | Message::End => 0,
            Message::Data(m) => m.byte_len(),
            Message::Ack(m) => uvarint_len(m.bin.0) + varint_len(duration_to_wire(m.delay_sample)),
            Message::Have(Have { bin }) | Message::Cancel(Cancel { bin }) => uvarint_len(bin.0),
            Message::Integrity(m) => uvarint_len(m.bin.0) + m.hash.len(),
            Message::SignedIntegrity(m) => {
                uvarint_len(m.bin.0) + varint_len(time_to_wire(m.timestamp)) + m.signature.len()
            }
            Message::Request(m) => uvarint_len(m.bin.0) + varint_len(time_to_wire(m.timestamp)),
            Message::Ping(_) => 8,
            Message::Pong(_) => 16,
            Message::StreamRequest(StreamRequest { bin, .. })
            | Message::StreamOpen(StreamOpen { bin, .. }) => 2 + uvarint_len(bin.0),
            Message::StreamCancel(_) | Message::StreamClose(_) => 2,
            Message::Announce(m) => m.byte_len(),
        }
    }

    /// The wire length: type byte plus body.
    pub fn encoded_len(&self) -> usize {
        super::MESSAGE_TYPE_LEN + self.byte_len()
    }

    /// Writes the type byte and body into `buf` and returns the length.
    /// Fails with [`Error::NotEnoughSpace`] before writing anything if
    /// `buf` is shorter than [`Message::encoded_len`].
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, Error> {
        let n = self.encoded_len();
        let buf = buf.get_mut(..n).ok_or(Error::NotEnoughSpace)?;
        let mut e = Enc::new(buf);
        e.u8(self.message_type().as_u8())?;
        match *self {
            Message::Handshake(m) => {
                e.u32(m.channel_id)?;
                m.options.encode(&mut e)?;
                e.u8(ProtocolOptionType::End.as_u8())?;
            }
            Message::Restart | Message::Choke | Message::Unchoke | Message::End => {}
            Message::Data(m) => {
                e.uvarint(m.bin.0)?;
                e.varint(time_to_wire(m.timestamp))?;
                e.bytes(m.data)?;
            }
            Message::Ack(m) => {
                e.uvarint(m.bin.0)?;
                e.varint(duration_to_wire(m.delay_sample))?;
            }
            Message::Have(Have { bin }) | Message::Cancel(Cancel { bin }) => e.uvarint(bin.0)?,
            Message::Integrity(m) => {
                e.uvarint(m.bin.0)?;
                e.bytes(m.hash)?;
            }
            Message::SignedIntegrity(m) => {
                e.uvarint(m.bin.0)?;
                e.varint(time_to_wire(m.timestamp))?;
                e.bytes(m.signature)?;
            }
            Message::Request(m) => {
                e.uvarint(m.bin.0)?;
                e.varint(time_to_wire(m.timestamp))?;
            }
            Message::Ping(m) => e.u64(m.nonce)?,
            Message::Pong(m) => {
                e.u64(m.nonce)?;
                e.u64(m.delay)?;
            }
            Message::StreamRequest(StreamRequest { stream, bin })
            | Message::StreamOpen(StreamOpen { stream, bin }) => {
                e.u16(stream)?;
                e.uvarint(bin.0)?;
            }
            Message::StreamCancel(StreamCancel { stream })
            | Message::StreamClose(StreamClose { stream }) => e.u16(stream)?,
            Message::Announce(m) => m.encode(&mut e)?,
        }
        Ok(e.pos())
    }

    /// Reads one message, type byte included, from the start of `buf`.
    /// Returns the message and the number of bytes it used.
    pub fn decode(config: &ReaderConfig, buf: &'a [u8]) -> Result<(Message<'a>, usize), Error> {
        let mut d = Dec::new(buf);
        let t = d.u8()?;
        let t = MessageType::from_u8(t).ok_or(Error::UnsupportedMessageType(t))?;
        let m = match t {
            MessageType::Handshake => {
                let channel_id = d.u32()?;
                let options = ProtocolOptions::decode(&mut d, config.integrity_signature_size)?;
                Message::Handshake(Handshake {
                    channel_id,
                    options,
                })
            }
            MessageType::Restart => Message::Restart,
            MessageType::Data => {
                let bin = Bin(d.uvarint()?);
                let timestamp = time_from_wire(d.varint()?);
                let n = usize::try_from(bin.base_length())
                    .unwrap_or(usize::MAX)
                    .saturating_mul(config.chunk_size)
                    .min(d.remaining());
                Message::Data(Data {
                    bin,
                    timestamp,
                    data: d.bytes(n)?,
                })
            }
            MessageType::Ack => Message::Ack(Ack {
                bin: Bin(d.uvarint()?),
                delay_sample: duration_from_wire(d.varint()?),
            }),
            MessageType::Have => Message::Have(Have {
                bin: Bin(d.uvarint()?),
            }),
            MessageType::Integrity => Message::Integrity(Integrity {
                bin: Bin(d.uvarint()?),
                hash: d.bytes(config.integrity_hash_size)?,
            }),
            MessageType::SignedIntegrity => {
                let bin = Bin(d.uvarint()?);
                let timestamp = time_from_wire(d.varint()?);
                Message::SignedIntegrity(SignedIntegrity {
                    bin,
                    timestamp,
                    signature: d.bytes(config.integrity_signature_size)?,
                })
            }
            MessageType::Request => Message::Request(Request {
                bin: Bin(d.uvarint()?),
                timestamp: time_from_wire(d.varint()?),
            }),
            MessageType::Cancel => Message::Cancel(Cancel {
                bin: Bin(d.uvarint()?),
            }),
            MessageType::Choke => Message::Choke,
            MessageType::Unchoke => Message::Unchoke,
            MessageType::Ping => Message::Ping(Ping { nonce: d.u64()? }),
            MessageType::Pong => Message::Pong(Pong {
                nonce: d.u64()?,
                delay: d.u64()?,
            }),
            MessageType::StreamRequest => Message::StreamRequest(StreamRequest {
                stream: d.u16()?,
                bin: Bin(d.uvarint()?),
            }),
            MessageType::StreamCancel => Message::StreamCancel(StreamCancel { stream: d.u16()? }),
            MessageType::StreamOpen => Message::StreamOpen(StreamOpen {
                stream: d.u16()?,
                bin: Bin(d.uvarint()?),
            }),
            MessageType::StreamClose => Message::StreamClose(StreamClose { stream: d.u16()? }),
            MessageType::Announce => Message::Announce(Announce::decode(&mut d)?),
            MessageType::End => Message::End,
        };
        Ok((m, d.pos()))
    }
}

/// The header in front of each channel payload in a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChannelHeader {
    /// The receiver's channel id.
    pub channel: u64,
    /// The payload length.
    pub length: u16,
}

impl ChannelHeader {
    pub fn byte_len(&self) -> usize {
        uvarint_len(self.channel) + 2
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, Error> {
        let mut e = Enc::new(buf);
        e.uvarint(self.channel)?;
        e.u16(self.length)?;
        Ok(e.pos())
    }

    pub fn decode(buf: &[u8]) -> Result<(ChannelHeader, usize), Error> {
        let mut d = Dec::new(buf);
        let channel = d.uvarint()?;
        let length = d.u16()?;
        Ok((ChannelHeader { channel, length }, d.pos()))
    }
}
