// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Fragmented MP4 (ISO/IEC 14496-12): init segments, media fragments and
//! the reading of records.
//!
//! A record on a video swarm is one segment:
//! `[u16 BE init length][init segment][media segment]`.
//!
//! - The init segment ([`write_init`]) is `ftyp` and `moov`, declaring one
//!   track per decoder config, with an edit list for a composition delay.
//! - The media segment is one or more fragments, each a `prft`
//!   ([`write_prft`]), a `moof` and its `mdat` ([`write_fragment`]). It
//!   starts at a video keyframe. Segments written here carry no `styp` or
//!   `sidx`; [`parse`] skips them.
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

#[cfg(test)]
pub(crate) use fragment::{NON_SYNC_SAMPLE_FLAGS, SYNC_SAMPLE_FLAGS};
pub use fragment::{
    PRFT_LEN, ProducerTime, Sample, TrackRun, fragment_head_len, fragment_len, ntp_timestamp,
    write_fragment, write_fragment_head, write_prft,
};
pub use init::{InitError, MAX_TRACKS, check_configs, write_init};
pub use parse::{
    FourCc, InitInfo, ParseError, TrackInfo, media_duration, parse_init, split_record,
};
pub(crate) use writer::Written;
pub use writer::{FragmentError, FragmentWriter};

/// Bytes of a `free` box header; a padding box is at least this long.
pub const FREE_HEADER_LEN: usize = boxes::HEADER_LEN;

/// Appends a top-level `free` box of `len` bytes, header included; `len`
/// is at least [`FREE_HEADER_LEN`] and below 4 GiB.
pub fn write_free(len: usize, out: &mut Vec<u8>) {
    let at = boxes::start(out, b"free");
    boxes::zeros(out, len.saturating_sub(FREE_HEADER_LEN));
    boxes::end(out, at);
}
