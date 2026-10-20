// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Captures of one node's ppspp traffic: every frame it sends and receives,
//! the lifecycle of its peers, swarms and channels, and the scheduler
//! decisions its runner's tap reports. A dump is one stream of records per
//! capturing node, and it decodes without the code that wrote it.
//!
//! [`Writer`] encodes records into any [`std::io::Write`], and [`Reader`]
//! decodes them from any [`std::io::Read`]. A dump is untrusted input to
//! its reader.
//!
//! # Format
//!
//! Integers are LEB128 varints: `uvarint` is at most 10 bytes and must fit
//! a `u64`, and `svarint` is a zigzag-encoded `uvarint`. `bytes` is a
//! `uvarint` length followed by that many bytes. Version 1:
//!
//! ```text
//! dump   = magic version header record*
//! magic  = "STRIMSPC"
//! version= u16, big-endian
//! header = record of kind 0, first and only once
//! record = len:uvarint kind:u8 dt:svarint body
//! ```
//!
//! `len` counts the bytes after it (`kind`, `dt` and `body`) and is at most
//! [`MAX_RECORD`]. `dt` is the record's time in nanoseconds after the
//! previous record's; the header's `dt` is 0 and its time is its origin.
//! Times are [`strims_base::time::Time`], nanoseconds since the Unix
//! epoch. A reader skips records of kinds it doesn't know, and ignores
//! bytes past the fields it knows in a record of a kind it does, except
//! where a body ends in `rest`.
//!
//! | kind | record | body |
//! |---|---|---|
//! | 0 | header | `clock:u8 origin:i64be file:uvarint node:bytes label:bytes` |
//! | 1 | peer | `peer:uvarint name:bytes` |
//! | 2 | peer gone | `peer:uvarint` |
//! | 3 | swarm | `swarm:uvarint id:bytes n:uvarint param:uvarint{n}` |
//! | 4 | swarm gone | `swarm:uvarint` |
//! | 5 | channel open | `peer:uvarint swarm:uvarint channel:uvarint peer_channel:uvarint` |
//! | 6 | channel close | `peer:uvarint swarm:uvarint` |
//! | 7 | frame in | `peer:uvarint len:uvarint body:rest` |
//! | 8 | frame out | `peer:uvarint len:uvarint body:rest` |
//! | 9 | frame error | `peer:uvarint message:bytes` |
//! | 10 | tap | `kind:uvarint peer:uvarint swarm:uvarint n:uvarint field:svarint{n}` |
//! | 11 | tap lost | `count:uvarint` |
//!
//! - **Header.** `clock` names the time source ([`Clock`]). `origin` is
//!   the time the capture started. `file` counts the files of a rotated
//!   capture from 0. `node` names the capturing node (at most
//!   [`MAX_NAME`] bytes) and `label` describes it in UTF-8 (at most
//!   [`MAX_LABEL`] bytes).
//! - **Peers and swarms** get ids from 1 in the dump, never reused within
//!   a capture; 0 means none. A peer's `name` is at most [`MAX_NAME`]
//!   bytes. A swarm's `id` is its swarm id (at most [`MAX_NAME`] bytes) and
//!   its params are [`SwarmParams`] in the order listed there, at least
//!   [`SwarmParams::DECODE_FIELDS`] and at most [`MAX_PARAMS`] of them.
//! - **Channels.** A frame is a sequence of channel payloads, each headed
//!   by a wire channel id: on frames in, the receiver's `channel`; on
//!   frames out, the peer's `peer_channel`. A channel open maps both to a
//!   swarm until its close.
//! - **Frames** hold the raw ppspp frame body. `len` is its length; a body
//!   over [`MAX_FRAME`] bytes keeps its first [`MAX_FRAME`]. A frame error
//!   follows the frame in that the runner refused, with a message of at
//!   most [`MAX_MESSAGE`] bytes.
//! - **Tap** records carry one scheduler decision about a peer and a swarm
//!   (either may be 0) as up to [`MAX_TAP_FIELDS`] signed fields. The
//!   writer of a capture defines the kinds. Tap records follow the frame of
//!   the runner call that produced them. Tap lost counts events dropped
//!   because the runner's buffer filled between drains.
//!
//! Every file of a capture begins, after its header, with records for the
//! peers, swarms and open channels that exist when it starts, so each file
//! decodes on its own.

mod format;
mod reader;
mod writer;

pub use format::{
    Clock, Dir, Header, MAGIC, MAX_FRAME, MAX_LABEL, MAX_MESSAGE, MAX_NAME, MAX_PARAMS, MAX_RECORD,
    MAX_TAP_FIELDS, Record, SwarmParams, TapRecord, VERSION,
};
pub use reader::{Entry, Error, OwnedHeader, Reader};
pub use writer::Writer;
