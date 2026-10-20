// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The PPSPP wire format: protocol version 2, plus ANNOUNCE (message type
//! 18), a report on the sender's parents or a request for the receiver's.
//!
//! A frame is a sequence of channel payloads, each behind a
//! [`ChannelHeader`]. A channel payload is a sequence of messages, each a
//! type byte followed by its body, optionally ended by
//! [`MessageType::End`].
//!
//! Contracts:
//!
//! - Encodings must stay byte-identical to the v2 encoding pinned by
//!   `golden/vectors.txt`, quirks included: non-minimal varints decode,
//!   timestamps wrap, a [`Data`] payload may be shorter than its bin, and
//!   an option list may lack its end marker.
//! - Decoding never panics and allocates nothing: [`Reader`] is an iterator
//!   of [`Message`] views whose byte strings (data, hashes, signatures,
//!   swarm ids, handshake options, ANNOUNCE entries) borrow from the input,
//!   and re-encoding a decoded message reproduces its input bytes.
//! - Encoders write into caller buffers. A message that doesn't fit fails
//!   with [`Error::NotEnoughSpace`] and writes nothing, so a frame filler
//!   can stop at the first message that doesn't fit.

mod messages;
mod reader;
mod wire;
mod writer;

#[cfg(test)]
mod messages_test;

use core::fmt;

pub use messages::{
    Ack, Announce, AnnounceEntry, AnnounceIter, Cancel, ChannelHeader, Data, EpochOption,
    Handshake, HandshakeOptions, Have, Integrity, MAX_ANNOUNCE_ENTRIES, MAX_PEER_ID, Message, Ping,
    Pong, ProtocolOption, ProtocolOptions, ProtocolOptionsIter, Request, SignedIntegrity, Stream,
    StreamCancel, StreamClose, StreamOpen, StreamRequest,
};
pub use reader::{FrameReader, Reader, ReaderConfig};
pub use wire::{uvarint_len, varint_len};
pub use writer::{ChannelWriter, MESSAGE_TYPE_LEN, MessageWriter, Writer};

/// Codec errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A message type byte with no encoding.
    #[error("unsupported message type")]
    UnsupportedMessageType(u8),
    /// A handshake option type byte with no encoding.
    #[error("unsupported protocol option")]
    UnsupportedProtocolOption(u8),
    /// The message doesn't fit the buffer; nothing was written. The
    /// scheduler relies on this to stop filling a frame.
    #[error("write buffer has insufficient space for message")]
    NotEnoughSpace,
    /// The input ends inside a message or channel payload.
    #[error("message truncated")]
    Truncated,
    /// A varint longer than 64 bits.
    #[error("varint overflows 64 bits")]
    VarintOverflow,
    /// A swarm identifier longer than its 16 bit length prefix allows.
    #[error("swarm identifier too long")]
    SwarmIdentifierTooLong,
    /// An ANNOUNCE past its bounds: too many entries, a stream past 16
    /// bits, or a parent id of the wrong length.
    #[error("invalid announce")]
    InvalidAnnounce,
}

macro_rules! wire_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident { $( $(#[$vmeta:meta])* $variant:ident = $value:literal => $label:literal, )* }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u8)]
        pub enum $name {
            $( $(#[$vmeta])* $variant = $value, )*
        }

        impl $name {
            /// The wire value.
            pub const fn as_u8(self) -> u8 {
                self as u8
            }

            /// The variant for a wire value, if it is defined.
            pub const fn from_u8(v: u8) -> Option<$name> {
                match v {
                    $( $value => Some($name::$variant), )*
                    _ => None,
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(match self {
                    $( $name::$variant => $label, )*
                })
            }
        }
    };
}

wire_enum! {
    /// Message type byte. `Display` gives the message name.
    pub enum MessageType {
        Handshake = 1 => "Handshake",
        Restart = 2 => "Restart",
        Data = 3 => "Data",
        Ack = 4 => "Ack",
        Have = 5 => "Have",
        Integrity = 6 => "Integrity",
        SignedIntegrity = 7 => "SignedIntegrity",
        Request = 8 => "Request",
        Cancel = 9 => "Cancel",
        Choke = 10 => "Choke",
        Unchoke = 11 => "Unchoke",
        Ping = 12 => "Ping",
        Pong = 13 => "Pong",
        StreamRequest = 14 => "StreamRequest",
        StreamCancel = 15 => "StreamCancel",
        StreamOpen = 16 => "StreamOpen",
        StreamClose = 17 => "StreamClose",
        Announce = 18 => "Announce",
        /// Ends a channel payload early. The reader stops at it.
        End = 255 => "End",
    }
}

wire_enum! {
    /// Handshake option type byte.
    ///
    /// `ChunkAddressingMethod` and `SupportedMessages` are defined but have
    /// no encoding; the decoder rejects them with
    /// [`Error::UnsupportedProtocolOption`].
    pub enum ProtocolOptionType {
        Version = 0 => "Version",
        MinimumVersion = 1 => "MinimumVersion",
        SwarmIdentifier = 2 => "SwarmIdentifier",
        ContentIntegrityProtectionMethod = 3 => "ContentIntegrityProtectionMethod",
        MerkleHashTreeFunction = 4 => "MerkleHashTreeFunction",
        LiveSignatureAlgorithm = 5 => "LiveSignatureAlgorithm",
        ChunkAddressingMethod = 6 => "ChunkAddressingMethod",
        LiveWindow = 7 => "LiveWindow",
        SupportedMessages = 8 => "SupportedMessages",
        ChunkSize = 9 => "ChunkSize",
        ChunksPerSignature = 10 => "ChunksPerSignature",
        StreamCount = 11 => "StreamCount",
        Epoch = 12 => "Epoch",
        /// Ends the option list.
        End = 255 => "EndOption",
    }
}
