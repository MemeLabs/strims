// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! FLV files (Adobe FLV 10.1, annex E): the header and tag framing around
//! the tag bodies [`FlvSource`](crate::FlvSource) reads, read as a file's
//! bytes arrive ([`FlvTagReader`]), and written from a stream's decoder
//! configs and access units ([`FlvWriter`]), for an encoder that reads and
//! writes FLV through pipes.
//!
//! A file is a 9-byte header (`FLV`, version 1, the audio and video
//! flags, the offset of the first tag), a 4-byte zero, then tags, each
//! followed by its size: an 11-byte tag header (type, 24-bit body size,
//! 24-bit timestamp and its upper 8 bits, a 24-bit zero stream id) and
//! the body.

use std::collections::VecDeque;

use crate::amf0::{self, Value};
use crate::media::{AccessUnit, CodecParams, DecoderConfig};

/// The tag type of audio.
pub const TAG_AUDIO: u8 = 8;
/// The tag type of video.
pub const TAG_VIDEO: u8 = 9;
/// The tag type of script data, such as `onMetaData`.
pub const TAG_SCRIPT: u8 = 18;
/// The bytes of a file's header.
const HEADER_LEN: usize = 9;
/// The bytes of a tag's header.
const TAG_HEADER_LEN: usize = 11;
/// The bytes of the size that follows each tag.
const TAG_SIZE_LEN: usize = 4;
/// The largest offset of the first tag a header may give.
const MAX_DATA_OFFSET: u32 = 1024;
/// The header's flags of a file with audio and video.
const FLAGS_AUDIO_VIDEO: u8 = 0x05;
/// A tag type's filter bit: the body is encrypted.
const FILTER: u8 = 0x20;
/// The legacy FLV codec ids of AVC and of AAC.
const CODEC_AVC: u8 = 7;
const SOUND_AAC: u8 = 10;
/// An AAC tag's first byte: AAC, 44 kHz, 16-bit, stereo, which decoders
/// take from the AudioSpecificConfig.
const AAC_FLAGS: u8 = 0xaf;
/// Enhanced RTMP's header bit and packet types.
const EX_HEADER: u8 = 0x80;
const SEQUENCE_START: u8 = 0;
const CODED_FRAMES: u8 = 1;
const CODED_FRAMES_X: u8 = 3;
/// Video frame types.
const FRAME_KEY: u8 = 1;
const FRAME_INTER: u8 = 2;
/// The range of a signed 24-bit composition time.
const MAX_CTS: i64 = (1 << 23) - 1;
/// The largest tag body: its size is 24 bits.
const MAX_BODY: usize = (1 << 24) - 1;

/// Why an FLV file doesn't read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FlvFileError {
    #[error("the file does not start with an FLV header")]
    Signature,
    #[error("FLV version {0} is not 1")]
    Version(u8),
    #[error("the FLV header's data offset {0} is out of range")]
    DataOffset(u32),
    #[error("an FLV tag of {0} bytes is over the bound")]
    TagTooLong(usize),
    #[error("an FLV tag is encrypted")]
    Encrypted,
    #[error("an FLV tag's size {got} is not its length {expected}")]
    TagSize { expected: u32, got: u32 },
}

/// A tag of a file: its type, its 32-bit time in milliseconds, and its
/// body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag<'a> {
    pub kind: u8,
    pub timestamp: u32,
    pub body: &'a [u8],
}

/// Reads an FLV file's tags as its bytes arrive.
///
/// It holds the bytes of at most one tag beyond what the last
/// [`push`](Self::push) brought: a tag over its bound fails the file.
/// A failed file stays failed.
#[derive(Debug)]
pub struct FlvTagReader {
    buf: VecDeque<u8>,
    /// The header was read, and the bytes still to skip before the first
    /// tag.
    header: Option<usize>,
    /// The largest tag body taken.
    max_body: usize,
    failed: Option<FlvFileError>,
    /// The body of the tag [`next_tag`](Self::next_tag) answered last.
    body: Vec<u8>,
}

impl FlvTagReader {
    /// A reader of tags whose bodies are at most `max_body` bytes.
    pub fn new(max_body: usize) -> Self {
        FlvTagReader {
            buf: VecDeque::new(),
            header: None,
            max_body: max_body.min(MAX_BODY),
            failed: None,
            body: Vec::new(),
        }
    }

    /// Takes the file's next bytes.
    pub fn push(&mut self, bytes: &[u8]) {
        if self.failed.is_none() {
            self.buf.extend(bytes);
        }
    }

    /// The bytes held that no tag answered yet.
    pub fn buffered(&self) -> usize {
        self.buf.len()
    }

    /// The next whole tag; `None` until its bytes have arrived.
    pub fn next_tag(&mut self) -> Result<Option<Tag<'_>>, FlvFileError> {
        if let Some(e) = self.failed {
            return Err(e);
        }
        match self.read() {
            Ok(Some((kind, timestamp))) => Ok(Some(Tag {
                kind,
                timestamp,
                body: &self.body,
            })),
            Ok(None) => Ok(None),
            Err(e) => {
                self.failed = Some(e);
                self.buf.clear();
                Err(e)
            }
        }
    }

    /// Reads the header, then a tag into `body`: its type and time.
    fn read(&mut self) -> Result<Option<(u8, u32)>, FlvFileError> {
        let skip = match self.header {
            Some(skip) => skip,
            None => {
                let Some(h) = self.peek::<HEADER_LEN>() else {
                    return Ok(None);
                };
                if h[..3] != *b"FLV" {
                    return Err(FlvFileError::Signature);
                }
                if h[3] != 1 {
                    return Err(FlvFileError::Version(h[3]));
                }
                let offset = u32::from_be_bytes([h[5], h[6], h[7], h[8]]);
                if !(HEADER_LEN as u32..=MAX_DATA_OFFSET).contains(&offset) {
                    return Err(FlvFileError::DataOffset(offset));
                }
                self.buf.drain(..HEADER_LEN);
                // The rest of the header, then the first tag's 4-byte size.
                let skip = offset as usize - HEADER_LEN + TAG_SIZE_LEN;
                self.header = Some(skip);
                skip
            }
        };
        if skip > 0 {
            let n = skip.min(self.buf.len());
            self.buf.drain(..n);
            self.header = Some(skip - n);
            if n < skip {
                return Ok(None);
            }
        }
        let Some(h) = self.peek::<TAG_HEADER_LEN>() else {
            return Ok(None);
        };
        if h[0] & FILTER != 0 {
            return Err(FlvFileError::Encrypted);
        }
        let kind = h[0] & 0x1f;
        let size = u32::from_be_bytes([0, h[1], h[2], h[3]]) as usize;
        if size > self.max_body {
            return Err(FlvFileError::TagTooLong(size));
        }
        let timestamp = u32::from_be_bytes([h[7], h[4], h[5], h[6]]);
        let total = TAG_HEADER_LEN + size + TAG_SIZE_LEN;
        if self.buf.len() < total {
            return Ok(None);
        }
        self.buf.drain(..TAG_HEADER_LEN);
        self.body.clear();
        self.body.extend(self.buf.drain(..size));
        let mut tail = [0; TAG_SIZE_LEN];
        for (b, v) in tail.iter_mut().zip(self.buf.drain(..TAG_SIZE_LEN)) {
            *b = v;
        }
        let got = u32::from_be_bytes(tail);
        // invariant: size is at most MAX_BODY, so the sum fits 32 bits.
        let expected = (TAG_HEADER_LEN + size) as u32;
        if got != expected {
            return Err(FlvFileError::TagSize { expected, got });
        }
        Ok(Some((kind, timestamp)))
    }

    /// The first `N` bytes held, if that many are.
    fn peek<const N: usize>(&self) -> Option<[u8; N]> {
        let mut out = [0; N];
        let mut bytes = self.buf.iter();
        for b in &mut out {
            *b = *bytes.next()?;
        }
        Some(out)
    }
}

/// Why a unit or config can't be written as an FLV tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FlvWriteError {
    /// A codec FLV doesn't carry here: Opus.
    #[error("FLV carries no {0}")]
    Codec(&'static str),
    #[error("a composition time of {0} ms is out of range")]
    CompositionTime(i64),
    #[error("a tag of {0} bytes is over FLV's bound")]
    TooLong(usize),
    #[error("the unit's config has timescale 0")]
    Timescale,
}

/// Writes a stream's decoder configs and access units as an FLV file:
/// AVC and AAC in the legacy layout, H.265 and AV1 in Enhanced RTMP's.
///
/// A unit's tag time is its DTS in whole milliseconds, less the first
/// unit's, plus the writer's start time, read modulo 2^32; a unit before
/// the first by more than the start time is written at 0. Composition
/// times are PTS less DTS in milliseconds. A config's sequence header
/// carries the time of the last unit written.
#[derive(Debug, Clone)]
pub struct FlvWriter {
    /// The time of the file's first unit, in milliseconds.
    start: i64,
    /// The first unit's DTS in milliseconds.
    base: Option<i64>,
    /// The last tag's time.
    last: u32,
}

impl FlvWriter {
    /// A writer whose first unit is at `start` milliseconds.
    pub fn new(start: u32) -> Self {
        FlvWriter {
            start: i64::from(start),
            base: None,
            last: start,
        }
    }

    /// The first unit's DTS in milliseconds, once one was written: the
    /// time the writer's `start` stands for.
    pub fn base(&self) -> Option<i64> {
        self.base
    }

    /// Appends the file's header, flagged as audio and video, and an
    /// `onMetaData` of `configs`: each video config's size and codec id
    /// (the legacy id, or the Enhanced RTMP FourCC as a number) and each
    /// audio config's codec id, and `frame_rate` when given.
    pub fn header(&self, configs: &[DecoderConfig], frame_rate: Option<f64>, out: &mut Vec<u8>) {
        out.extend_from_slice(b"FLV");
        out.extend_from_slice(&[1, FLAGS_AUDIO_VIDEO]);
        out.extend_from_slice(&(HEADER_LEN as u32).to_be_bytes());
        out.extend_from_slice(&[0; TAG_SIZE_LEN]);
        let mut fields = Vec::new();
        for c in configs {
            let fourcc = |f: &[u8; 4]| f64::from(u32::from_be_bytes(*f));
            match &c.params {
                CodecParams::Avc { width, height, .. }
                | CodecParams::Hevc { width, height, .. }
                | CodecParams::Av1 { width, height, .. } => {
                    fields.push(("width", Value::Number(f64::from(*width))));
                    fields.push(("height", Value::Number(f64::from(*height))));
                    let id = match c.params {
                        CodecParams::Hevc { .. } => fourcc(b"hvc1"),
                        CodecParams::Av1 { .. } => fourcc(b"av01"),
                        _ => f64::from(CODEC_AVC),
                    };
                    fields.push(("videocodecid", Value::Number(id)));
                }
                CodecParams::Aac { .. } => {
                    fields.push(("audiocodecid", Value::Number(f64::from(SOUND_AAC))));
                }
                CodecParams::Opus { .. } => {
                    fields.push(("audiocodecid", Value::Number(fourcc(b"Opus"))));
                }
            }
        }
        if let Some(rate) = frame_rate {
            fields.push(("framerate", Value::Number(rate)));
        }
        let mut body = Vec::new();
        let encoded = amf0::encode(&Value::String("onMetaData"), &mut body)
            .and_then(|()| amf0::encode(&Value::EcmaArray(fields), &mut body));
        // invariant: the strings are short and the array small.
        #[allow(clippy::expect_used)]
        encoded.expect("onMetaData encodes");
        write_tag(TAG_SCRIPT, 0, &[&body], out);
    }

    /// Appends `config`'s sequence header.
    pub fn config(
        &mut self,
        config: &DecoderConfig,
        out: &mut Vec<u8>,
    ) -> Result<(), FlvWriteError> {
        let record = config.params.record();
        let time = self.last;
        match &config.params {
            CodecParams::Avc { .. } => {
                let head = [(FRAME_KEY << 4) | CODEC_AVC, 0, 0, 0, 0];
                tag(TAG_VIDEO, time, &[&head, record], out)
            }
            CodecParams::Hevc { .. } => {
                let head = [EX_HEADER | (FRAME_KEY << 4) | SEQUENCE_START];
                tag(TAG_VIDEO, time, &[&head, b"hvc1", record], out)
            }
            CodecParams::Av1 { .. } => {
                let head = [EX_HEADER | (FRAME_KEY << 4) | SEQUENCE_START];
                tag(TAG_VIDEO, time, &[&head, b"av01", record], out)
            }
            CodecParams::Aac { .. } => tag(TAG_AUDIO, time, &[&[AAC_FLAGS, 0], record], out),
            CodecParams::Opus { .. } => Err(FlvWriteError::Codec("Opus")),
        }
    }

    /// Appends `unit`, of a track of `config`.
    pub fn unit(
        &mut self,
        config: &DecoderConfig,
        unit: &AccessUnit<'_>,
        out: &mut Vec<u8>,
    ) -> Result<(), FlvWriteError> {
        let ms = |t: i64| {
            let ms = i128::from(t) * 1000 / i128::from(config.timescale);
            i64::try_from(ms).unwrap_or(if t < 0 { i64::MIN } else { i64::MAX })
        };
        if config.timescale == 0 {
            return Err(FlvWriteError::Timescale);
        }
        let (dts, pts) = (ms(unit.dts), ms(unit.pts));
        let cts = pts.saturating_sub(dts);
        if cts.abs() > MAX_CTS {
            return Err(FlvWriteError::CompositionTime(cts));
        }
        let base = *self.base.get_or_insert(dts);
        let time = self.start.saturating_add(dts.saturating_sub(base)).max(0);
        // Tag times are 32 bits and wrap.
        let time = (time & 0xffff_ffff) as u32;
        let frame = if unit.keyframe {
            FRAME_KEY
        } else {
            FRAME_INTER
        };
        let [_, c0, c1, c2] = (cts as i32).to_be_bytes();
        let payload = unit.payload;
        let written = match &config.params {
            CodecParams::Avc { .. } => {
                let head = [(frame << 4) | CODEC_AVC, 1, c0, c1, c2];
                tag(TAG_VIDEO, time, &[&head, payload], out)
            }
            CodecParams::Hevc { .. } if cts != 0 => {
                let head = [EX_HEADER | (frame << 4) | CODED_FRAMES];
                tag(
                    TAG_VIDEO,
                    time,
                    &[&head, b"hvc1", &[c0, c1, c2], payload],
                    out,
                )
            }
            CodecParams::Hevc { .. } => {
                let head = [EX_HEADER | (frame << 4) | CODED_FRAMES_X];
                tag(TAG_VIDEO, time, &[&head, b"hvc1", payload], out)
            }
            CodecParams::Av1 { .. } => {
                let head = [EX_HEADER | (frame << 4) | CODED_FRAMES];
                tag(TAG_VIDEO, time, &[&head, b"av01", payload], out)
            }
            CodecParams::Aac { .. } => tag(TAG_AUDIO, time, &[&[AAC_FLAGS, 1], payload], out),
            CodecParams::Opus { .. } => Err(FlvWriteError::Codec("Opus")),
        };
        if written.is_ok() {
            self.last = time;
        }
        written
    }
}

/// Appends a tag of `parts` if its body fits FLV's bound.
fn tag(kind: u8, time: u32, parts: &[&[u8]], out: &mut Vec<u8>) -> Result<(), FlvWriteError> {
    let len: usize = parts.iter().map(|p| p.len()).sum();
    if len > MAX_BODY {
        return Err(FlvWriteError::TooLong(len));
    }
    write_tag(kind, time, parts, out);
    Ok(())
}

/// Appends a tag of `parts`, whose length is at most [`MAX_BODY`].
fn write_tag(kind: u8, time: u32, parts: &[&[u8]], out: &mut Vec<u8>) {
    let len: usize = parts.iter().map(|p| p.len()).sum();
    // invariant: callers keep the body within 24 bits.
    let [_, s0, s1, s2] = (len as u32).to_be_bytes();
    let [t3, t0, t1, t2] = time.to_be_bytes();
    out.extend_from_slice(&[kind, s0, s1, s2, t0, t1, t2, t3, 0, 0, 0]);
    for p in parts {
        out.extend_from_slice(p);
    }
    out.extend_from_slice(&((TAG_HEADER_LEN + len) as u32).to_be_bytes());
}

#[cfg(test)]
#[path = "flv_file_test.rs"]
mod tests;
