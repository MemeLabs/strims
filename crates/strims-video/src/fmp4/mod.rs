// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Fragmented MP4 (ISO/IEC 14496-12): init segments and media fragments,
//! which HLS segments and the players' MediaSource input are made of, and
//! their reading.
//!
//! - The init segment ([`write_init`]) is `ftyp` and `moov`, declaring one
//!   track per decoder config, with an edit list for a composition delay.
//! - A media segment is one or more fragments, each a `prft`
//!   ([`write_prft`]), a `moof` and its `mdat` ([`write_fragment`]).
//!   Segments written here carry no `styp` or `sidx`; [`parse`] skips
//!   them.
//!
//! The writers append to caller buffers and back-patch box sizes. They
//! read no clock and touch no I/O, so they build for wasm32.
//! [`FragmentWriter`] turns access units into the fragments that follow
//! an init, timing each sample by the next of its track.

mod boxes;
mod fragment;
mod init;
pub mod parse;
#[cfg(test)]
pub(crate) mod test_dump;
mod writer;

pub use fragment::{
    PRFT_LEN, ProducerTime, Sample, TrackRun, fragment_head_len, fragment_len, ntp_timestamp,
    write_fragment, write_fragment_head, write_prft,
};
pub use init::{InitError, MAX_TRACKS, check_configs, write_init};
pub use parse::{FourCc, InitInfo, ParseError, TrackInfo, media_duration, parse_init};
pub(crate) use writer::Written;
pub use writer::{FragmentError, FragmentWriter};
