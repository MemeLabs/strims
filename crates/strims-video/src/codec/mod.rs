// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The codec stage: a stream's decoder configs and access units in, its
//! renditions' out, and a picture of a keyframe for its snippet.
//!
//! A node's [`VideoHost`](crate::VideoHost) supplies the stage where it
//! has one ([`VideoHost::codec`](crate::VideoHost::codec)): native drivers
//! with an ffmpeg ([`ffmpeg`]); [`NoCodec`] elsewhere.
//!
//! # Transcodes
//!
//! A transcode ([`Codec::transcode`]) makes the H.264 renditions a
//! [`TranscodePlan`] names of one stream, each with the stream's audio as
//! it came. It takes the stream's configs and units as they arrive
//! ([`Transcode::send`]), queuing what its encoder hasn't taken, and
//! yields each rendition's configs and units ([`Transcode::poll_next`]),
//! which a rendition's writer takes as its source's: video track
//! [`VIDEO_TRACK`](crate::flv::VIDEO_TRACK), audio
//! [`AUDIO_TRACK`](crate::flv::AUDIO_TRACK).
//!
//! - **Timeline.** A rendition's units keep the stream's timeline: a
//!   frame a rendition keeps has the source frame's PTS, and each of the
//!   stream's keyframes starts a GOP in every rendition that keeps its
//!   frame. A rendition below the stream's frame rate keeps a subset of
//!   its frames: every keyframe, and each frame at least a frame of the
//!   rendition's rate after the last kept. Units carry the publish time of
//!   the source unit of their time.
//! - **Bounds.** The input queued is bounded: past it, [`Transcode::send`]
//!   refuses ([`Refused::Full`]) and the caller drops units up to its
//!   stream's next keyframe. The output is read as it comes, a tag at a
//!   time: a transcode holds at most one read's events.
//! - **Hostile input.** The stream's bitstream reaches the encoder as
//!   it came, and an encoder that a bitstream subverts writes what it
//!   likes: its output goes through the same parsers and bounds as a
//!   publisher's input.
//! - **End.** [`Transcode::finish`] ends the input, and the output ends
//!   once the encoder wrote what it held. Dropping a transcode stops it.

pub mod ffmpeg;

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use strims_base::time::Time;

use crate::media::{AccessUnit, DecoderConfig, TrackId};

/// The most renditions a transcode makes.
pub const MAX_RENDITIONS: usize = crate::flv::MAX_VIDEO_TRACKS as usize;

/// An access unit that owns its payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub track: TrackId,
    pub dts: i64,
    pub pts: i64,
    pub keyframe: bool,
    pub payload: Vec<u8>,
    pub published: Time,
}

impl Unit {
    /// A copy of `u`.
    pub fn of(u: &AccessUnit<'_>) -> Unit {
        Unit {
            track: u.track,
            dts: u.dts,
            pts: u.pts,
            keyframe: u.keyframe,
            payload: u.payload.to_vec(),
            published: u.published,
        }
    }

    /// The unit as an [`AccessUnit`] borrowing its payload.
    pub fn as_access_unit(&self) -> AccessUnit<'_> {
        AccessUnit {
            track: self.track,
            dts: self.dts,
            pts: self.pts,
            keyframe: self.keyframe,
            payload: &self.payload,
            published: self.published,
        }
    }
}

/// What a transcode takes: one of its stream's decoder configs, or units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input<'a> {
    Config(&'a DecoderConfig),
    Unit(&'a AccessUnit<'a>),
}

/// What a rendition of a transcode yields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    Config(DecoderConfig),
    Unit(Unit),
}

/// An event of a transcode: what one of its renditions yields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecEvent {
    /// The rendition's index in the plan.
    pub rendition: usize,
    pub output: Output,
}

/// Why a transcode didn't take an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    /// Its queue is at its bound: the encoder is behind.
    #[error("the encoder's input queue is full")]
    Full,
    /// The encoder takes no units of this kind or timing.
    #[error("the encoder can't take this unit: {0}")]
    Media(&'static str),
}

/// One H.264 rendition of a stream's video.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenditionPlan {
    /// The picture's shorter side, in pixels: its height when it is
    /// wider than tall. The longer side keeps the stream's aspect ratio,
    /// rounded to even.
    pub short_side: u16,
    /// The most frames a second it keeps.
    pub max_frame_rate: u32,
    /// Its video bitrate, in bits a second: the encoder's target and its
    /// maximum over a two-second buffer.
    pub bitrate: u64,
}

/// What a transcode makes: renditions, at most [`MAX_RENDITIONS`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscodePlan {
    pub renditions: Vec<RenditionPlan>,
}

/// A thumbnail being made: the JPEG, or why it couldn't be.
pub type Thumbnail<E> = Pin<Box<dyn Future<Output = Result<Vec<u8>, E>>>>;

/// A codec stage: see the [module docs](self).
pub trait Codec: Clone + 'static {
    type Error: fmt::Display;
    type Transcode: Transcode<Error = Self::Error>;

    /// Starts a transcode of one stream into `plan`'s renditions.
    fn transcode(&self, plan: &TranscodePlan) -> Result<Self::Transcode, Self::Error>;

    /// A JPEG of `keyframe`'s picture, a unit of a track of `configs`
    /// decodable on its own, `width` pixels wide and at most `max_bytes`
    /// long.
    fn thumbnail(
        &self,
        configs: &[DecoderConfig],
        keyframe: &AccessUnit<'_>,
        width: u16,
        max_bytes: usize,
    ) -> Thumbnail<Self::Error>;
}

/// A running transcode: see the [module docs](self#transcodes).
pub trait Transcode: Unpin + 'static {
    type Error: fmt::Display;

    /// Queues `input`: a config before the units it applies to, units of
    /// each track in decode order.
    fn send(&mut self, input: Input<'_>) -> Result<(), Refused>;

    /// The input bytes queued that the encoder hasn't taken.
    fn queued_bytes(&self) -> usize;

    /// Ends the input.
    fn finish(&mut self);

    /// Moves the input to the encoder and yields its renditions' events;
    /// `None` once its output ended.
    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<CodecEvent, Self::Error>>>;
}

/// A host without a codec stage: none is ever made.
#[derive(Debug, Clone, Copy)]
pub enum NoCodec {}

/// A transcode of a host without a codec stage: none exists.
#[derive(Debug)]
pub enum NoTranscode {}

impl Codec for NoCodec {
    type Error = std::convert::Infallible;
    type Transcode = NoTranscode;

    fn transcode(&self, _: &TranscodePlan) -> Result<NoTranscode, Self::Error> {
        match *self {}
    }

    fn thumbnail(
        &self,
        _: &[DecoderConfig],
        _: &AccessUnit<'_>,
        _: u16,
        _: usize,
    ) -> Thumbnail<Self::Error> {
        match *self {}
    }
}

impl Transcode for NoTranscode {
    type Error = std::convert::Infallible;

    fn send(&mut self, _: Input<'_>) -> Result<(), Refused> {
        match *self {}
    }

    fn queued_bytes(&self) -> usize {
        match *self {}
    }

    fn finish(&mut self) {
        match *self {}
    }

    fn poll_next(&mut self, _: &mut Context<'_>) -> Poll<Option<Result<CodecEvent, Self::Error>>> {
        match *self {}
    }
}

/// The processes a driver runs for an ffmpeg codec stage
/// ([`ffmpeg::Ffmpeg`]): its ffmpeg, given arguments, with its standard
/// input and output piped.
pub trait Processes: Clone + 'static {
    type Error: fmt::Display;
    type Child: ChildProcess<Error = Self::Error>;

    /// Starts the program with `args`.
    fn spawn(&self, args: &[String]) -> Result<Self::Child, Self::Error>;
}

/// A running process. Dropping it kills the process.
pub trait ChildProcess: Unpin + 'static {
    type Error: fmt::Display;

    /// Writes some of `buf` to the process's standard input: the bytes
    /// taken.
    fn poll_write(&mut self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Self::Error>>;

    /// Closes the process's standard input.
    fn close_input(&mut self);

    /// Reads the process's standard output into `buf`: the bytes read, 0
    /// at its end.
    fn poll_read(
        &mut self,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Self::Error>>;

    /// The last of what the process wrote to its standard error, for
    /// reports of its failure.
    fn diagnostics(&self) -> String;
}

#[cfg(test)]
#[path = "codec_test.rs"]
mod tests;
