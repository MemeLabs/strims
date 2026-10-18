// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Live video.
//!
//! Sources produce access units ([`AccessUnit`]): one coded frame of a
//! track, with its decode and presentation timestamps, keyframe flag and
//! publish time. Each track has a decoder config ([`DecoderConfig`]): an
//! RFC 6381 codec string, the timescale of the track's timestamps and the
//! codec's parameters. A source emits a track's config before the track's
//! first access unit and again whenever it changes. Muxers ([`Muxer`])
//! consume both.
//!
//! - [`media`]: tracks, decoder configs, access units, the [`Muxer`]
//!   trait and the fMP4 MIME type.
//! - [`avc`]: H.264 avcC records and the picture size from their SPS.
//! - [`aac`]: AAC AudioSpecificConfig and the `esds` box.
//! - [`flv`]: the source over FLV tags, legacy and Enhanced RTMP. It takes
//!   H.264 with AAC and refuses other codecs.
//!
//! Every parser takes hostile input: none panics, and decoder configs are
//! bounded by [`MAX_CONFIG_LEN`].

pub mod aac;
pub mod avc;
mod bits;
pub mod flv;
pub mod hls;
pub mod media;
#[cfg(test)]
mod test_util;

pub use flv::{FlvError, FlvMetrics, FlvSource, SourceEvent, Unsupported};
pub use media::{
    AccessUnit, CodecParams, DecoderConfig, MAX_CONFIG_LEN, Muxer, TrackId, TrackKind, fmp4_mime,
};
