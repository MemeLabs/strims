// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The unit format: one record per access unit, for live video swarms.
//!
//! A swarm of this format carries records in the chunked chunkstream
//! layout ([`strims_chunkstream::chunked`]), in frames of the swarm's
//! chunk size, so frame `i` is chunk `i`. Its listing's MIME type is
//! [`MIME_TYPE`] with the codecs of its tracks ([`units_mime`]). It is a
//! wire format: `golden/units.txt` pins it.
//!
//! - [`record`]: the records' encoding.
//! - [`UnitWriter`]: the [`Muxer`](crate::Muxer) writing access units as
//!   records into a swarm's stream, and [`SignTimer`], which says when the
//!   writer's partial integrity unit is due for signing.
//! - [`UnitReader`]: records in, decoder configs and access units out. It
//!   builds for wasm32.
//!
//! # Records
//!
//! Every record's payload starts the same way:
//!
//! ```text
//! kind        u8       1 CONFIGS, 2 UNIT; never 0
//! gop_back    uvarint  the record's stream offset less its GOP's sync record's
//! header_len  uvarint  the length of the header that follows
//! header      header_len bytes: the kind's fields, in order
//! body        the rest of the record
//! ```
//!
//! Varints are unsigned LEB128; signed values are zigzag encoded first.
//! Writers write the fields a kind has; a reader takes the fields it
//! knows from the header and skips the bytes after them, so a kind gains
//! fields only at its header's end. A reader skips records of a kind it
//! doesn't know, and every kind carries `gop_back`.
//!
//! **UNIT** holds one access unit. Its header:
//!
//! ```text
//! track      uvarint  the track id, nonzero, at most u32::MAX
//! flags      u8       bit 0: a keyframe; the other bits are written 0 and ignored
//! seq        uvarint  the unit's number in its track: 0 for the track's first unit, then +1 each
//! config     uvarint  the version of its track's config it decodes with, at most u32::MAX
//! dts        zigzag   decode time in the track's timescale
//! cts        zigzag   presentation time less decode time
//! published  zigzag   its publish time on the writer's Unix clock, in milliseconds
//! ```
//!
//! Its body is the unit's payload.
//!
//! **CONFIGS** holds the decoder config of every track, as of the record.
//! Its header is empty. Its body is 1 to [`MAX_TRACKS`] entries, each a
//! `uvarint` length and then:
//!
//! ```text
//! track         uvarint  the track id, nonzero and distinct, at most u32::MAX
//! version       uvarint  the config's version: 0 for the track's first, then +1 at each change
//! next_seq      uvarint  the seq of the track's next unit
//! sample_entry  4 bytes  avc1 (an avcC record) or mp4a (an AudioSpecificConfig)
//! timescale     uvarint  units per second of the track's timestamps, nonzero, at most u32::MAX
//! config_len    uvarint  at most MAX_CONFIG_LEN
//! config        config_len bytes
//! ```
//!
//! An entry's bytes after its config are skipped, and a reader skips an
//! entry of a sample entry it doesn't know: that track is unknown to it.
//! A reader derives what else a [`DecoderConfig`](crate::DecoderConfig)
//! holds (its codec string, picture size or sample rate) from the config
//! bytes, and an AAC track's timescale is its output sample rate.
//!
//! # Groups of pictures
//!
//! A GOP starts at each keyframe of the cut track: the video track with
//! the lowest id, or the audio track with the lowest id when there is no
//! video, whose keyframes start one at most every [`AUDIO_SYNC_SPAN`] of
//! its decode time. The GOP's sync record, a CONFIGS record with
//! `gop_back` 0, comes right before that keyframe's record. A changed
//! config writes a CONFIGS record before the next unit. Units before the
//! first GOP aren't written.
//!
//! So a reader that joins at any record learns the offset `K` of its GOP's
//! sync record, the offset of its first byte less `gop_back`; from `K` on
//! it has every config and the GOP's first keyframe. To start there, it
//! resets its layout decoder at `K - K % frame_size` and drops the records
//! whose offset is below `K`.
//!
//! # Bounds
//!
//! A record is at most [`MAX_RECORD_LEN`] bytes, which writers and every
//! reader's layout decoder share. Parsing a record never panics, works in
//! time linear in its length, and holds at most [`MAX_TRACKS`] configs of
//! [`MAX_CONFIG_LEN`](crate::MAX_CONFIG_LEN) bytes.

mod reader;
pub mod record;
mod writer;

pub use reader::{ReaderEvent, UnitReader, UnitReaderMetrics};
pub use record::{ConfigEntry, Record, RecordError, UnitHeader};
pub use writer::{SignTimer, UnitWriteError, UnitWriter, UnitWriterMetrics};

use strims_base::time::Duration;

pub use crate::fmp4::MAX_TRACKS;
pub use crate::media::units_mime;

/// The MIME type's essence: the type and subtype, without parameters.
pub const MIME_TYPE: &str = "video/x.strims-units";

/// The largest record payload, which writers and readers share. A
/// keyframe at a high bitrate can take a few MiB.
pub const MAX_RECORD_LEN: usize = 8 * 1024 * 1024;

/// The least decode time between two GOP starts of an audio cut track.
pub const AUDIO_SYNC_SPAN: Duration = Duration::from_secs(1);

#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
#[path = "units_test.rs"]
mod tests;
