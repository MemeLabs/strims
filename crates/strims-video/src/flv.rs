// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A source over the bodies of FLV video and audio tags, as RTMP carries
//! them: H.264 and AAC, in the legacy layout or Enhanced RTMP's FourCC
//! headers.
//!
//! - **Video** is track [`VIDEO_TRACK`] on a 90 kHz timescale. The legacy
//!   AVC packets (codec id 7) and Enhanced RTMP's `avc1` packets carry the
//!   avcC sequence header and frames with a signed 24-bit composition time
//!   in milliseconds: DTS is the tag time, PTS the tag time plus the
//!   composition time. The FLV frame type marks keyframes.
//! - **Audio** is track [`AUDIO_TRACK`] on a timescale of its output
//!   sample rate. The legacy AAC packets (sound format 10) and Enhanced
//!   RTMP's `mp4a` packets carry the AudioSpecificConfig and raw frames.
//!   Each frame starts a frame's samples after the one before; when that
//!   count drifts from the tag time by more than a frame, it restarts at
//!   the tag time.
//! - Other codecs, Enhanced RTMP multitrack and ModEx packets are refused
//!   with [`FlvError::Unsupported`], whose message tells the streamer what
//!   to send. Command frames, sequence ends, Enhanced RTMP metadata and
//!   empty tags carry nothing and are skipped.
//!
//! Tag times are 32-bit milliseconds that wrap; each is taken as the time
//! nearest the previous tag's, on one clock for both tracks. A track's
//! frames before its first sequence header are dropped, and a DTS below
//! the track's previous one is raised to it. [`FlvMetrics`] counts each
//! of these.
//!
//! A sequence header yields a [`SourceEvent::Config`] when its record
//! differs from the track's current one. A new audio sample rate changes
//! the timescale, and the sample count starts over at the next frame's tag
//! time.

use std::fmt;

use strims_base::time::Time;

use crate::aac::{self, AacError};
use crate::avc::{self, AvcError};
use crate::media::{AccessUnit, CodecParams, DecoderConfig, TrackId, TrackKind};

/// The video track's id.
pub const VIDEO_TRACK: TrackId = TrackId(1);
/// The audio track's id.
pub const AUDIO_TRACK: TrackId = TrackId(2);
/// The video track's timescale.
pub const VIDEO_TIMESCALE: u32 = 90_000;
/// Video timescale units per tag millisecond.
const VIDEO_UNITS_PER_MS: i64 = VIDEO_TIMESCALE as i64 / 1000;

/// FLV video frame types.
const FRAME_KEY: u8 = 1;
const FRAME_COMMAND: u8 = 5;

/// The legacy FLV codec id of AVC.
const CODEC_AVC: u8 = 7;
/// Legacy AVCPacketType values.
const AVC_SEQUENCE_HEADER: u8 = 0;
const AVC_NALU: u8 = 1;
const AVC_END_OF_SEQUENCE: u8 = 2;

/// Set in a video tag's first byte when an Enhanced RTMP header follows.
const VIDEO_EX_HEADER: u8 = 0x80;
/// Enhanced RTMP VideoPacketType values.
const VIDEO_SEQUENCE_START: u8 = 0;
const VIDEO_CODED_FRAMES: u8 = 1;
const VIDEO_SEQUENCE_END: u8 = 2;
const VIDEO_CODED_FRAMES_X: u8 = 3;
const VIDEO_METADATA: u8 = 4;
const VIDEO_MULTITRACK: u8 = 6;
const VIDEO_MOD_EX: u8 = 7;

/// FLV sound formats.
const SOUND_EX_HEADER: u8 = 9;
const SOUND_AAC: u8 = 10;
/// Legacy AACPacketType values.
const AAC_SEQUENCE_HEADER: u8 = 0;
const AAC_RAW: u8 = 1;
/// Enhanced RTMP AudioPacketType values.
const AUDIO_SEQUENCE_START: u8 = 0;
const AUDIO_CODED_FRAMES: u8 = 1;
const AUDIO_SEQUENCE_END: u8 = 2;
const AUDIO_MULTICHANNEL_CONFIG: u8 = 4;
const AUDIO_MULTITRACK: u8 = 5;
const AUDIO_MOD_EX: u8 = 7;

/// The codecs a refusal tells the streamer to send.
const ADVICE: &str = "send H.264 with AAC";

/// What a tag yields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceEvent<'a> {
    /// A track's new decoder config, before the units it applies to.
    Config(DecoderConfig),
    Unit(AccessUnit<'a>),
}

/// A codec or feature the source refuses. The message names it and tells
/// the streamer what to send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Unsupported {
    /// A legacy FLV video codec id other than AVC.
    VideoCodecId(u8),
    /// An Enhanced RTMP video FourCC other than `avc1`.
    VideoFourCc([u8; 4]),
    /// A legacy FLV sound format other than AAC.
    SoundFormat(u8),
    /// An Enhanced RTMP audio FourCC other than `mp4a`.
    AudioFourCc([u8; 4]),
    /// An Enhanced RTMP multitrack packet.
    Multitrack(TrackKind),
    /// An Enhanced RTMP ModEx packet.
    ModEx(TrackKind),
    /// An AAC object type other than Main, LC, SSR or LTP, or SBR or PS
    /// over one of them.
    AacObjectType(u8),
}

impl fmt::Display for Unsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unsupported::VideoCodecId(id) => match id {
                2 => write!(f, "Sorenson H.263 video is not supported"),
                3 | 6 => write!(f, "Screen video is not supported"),
                4 | 5 => write!(f, "VP6 video is not supported"),
                12 => write!(f, "HEVC (FLV codec id 12) is not supported"),
                13 => write!(f, "AV1 (FLV codec id 13) is not supported"),
                _ => write!(f, "FLV video codec id {id} is not supported"),
            },
            Unsupported::VideoFourCc(fourcc) => match fourcc {
                b"hvc1" => write!(f, "HEVC (hvc1) is not supported"),
                b"av01" => write!(f, "AV1 (av01) is not supported"),
                b"vp09" => write!(f, "VP9 (vp09) is not supported"),
                b"vp08" => write!(f, "VP8 (vp08) is not supported"),
                _ => write!(
                    f,
                    "video codec '{}' is not supported",
                    fourcc.escape_ascii()
                ),
            },
            Unsupported::SoundFormat(format) => match format {
                0 | 3 => write!(f, "PCM audio is not supported"),
                1 => write!(f, "ADPCM audio is not supported"),
                2 | 14 => write!(f, "MP3 audio is not supported"),
                4..=6 => write!(f, "Nellymoser audio is not supported"),
                7 | 8 => write!(f, "G.711 audio is not supported"),
                11 => write!(f, "Speex audio is not supported"),
                _ => write!(f, "FLV sound format {format} is not supported"),
            },
            Unsupported::AudioFourCc(fourcc) => match fourcc {
                b"Opus" => write!(f, "Opus audio is not supported"),
                b".mp3" => write!(f, "MP3 audio is not supported"),
                b"fLaC" => write!(f, "FLAC audio is not supported"),
                b"ac-3" => write!(f, "AC-3 audio is not supported"),
                b"ec-3" => write!(f, "E-AC-3 audio is not supported"),
                _ => write!(
                    f,
                    "audio codec '{}' is not supported",
                    fourcc.escape_ascii()
                ),
            },
            Unsupported::Multitrack(kind) => write!(f, "multitrack {kind} is not supported"),
            Unsupported::ModEx(kind) => {
                write!(f, "Enhanced RTMP ModEx {kind} packets are not supported")
            }
            Unsupported::AacObjectType(aot) => write!(f, "AAC object type {aot} is not supported"),
        }?;
        match self {
            Unsupported::AacObjectType(_) => write!(f, "; {ADVICE}-LC or HE-AAC"),
            _ => write!(f, "; {ADVICE}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FlvError {
    #[error("{0}")]
    Unsupported(Unsupported),
    /// A tag body shorter than its header.
    #[error("{0} tag is truncated")]
    Truncated(TrackKind),
    /// A packet type the codec doesn't define.
    #[error("{kind} packet type {packet_type} is not valid")]
    PacketType { kind: TrackKind, packet_type: u8 },
    #[error("invalid AVC sequence header: {0}")]
    Avc(AvcError),
    #[error("invalid AAC sequence header: {0}")]
    Aac(AacError),
}

/// What a source has skipped, dropped and adjusted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlvMetrics {
    /// Tags that carry nothing new: command frames, sequence ends,
    /// metadata, empty tags and repeated sequence headers.
    pub skipped: u64,
    /// Frames dropped because their track had no decoder config yet.
    pub before_config: u64,
    /// Units whose DTS was raised to the previous unit's.
    pub dts_clamped: u64,
    /// Times the audio sample count restarted at the tag time.
    pub audio_resyncs: u64,
}

/// Turns 32-bit tag times into 64-bit ones, each the value nearest the
/// previous.
#[derive(Debug, Default)]
struct Clock {
    /// The previous tag's raw and unwrapped times.
    last: Option<(u32, i64)>,
}

impl Clock {
    fn unwrap(&mut self, ts: u32) -> i64 {
        let ms = match self.last {
            None => i64::from(ts),
            // A wrapped difference read as signed: within 2^31 ms either way.
            Some((raw, ms)) => ms.saturating_add(i64::from(ts.wrapping_sub(raw) as i32)),
        };
        self.last = Some((ts, ms));
        ms
    }
}

#[derive(Debug)]
struct VideoTrack {
    config: DecoderConfig,
    last_dts: Option<i64>,
}

#[derive(Debug)]
struct AudioTrack {
    config: DecoderConfig,
    sample_rate: u32,
    frame_samples: u32,
    last_dts: Option<i64>,
}

/// The source state of one FLV stream: its clock and each track's config
/// and last DTS.
#[derive(Debug, Default)]
pub struct FlvSource {
    clock: Clock,
    video: Option<VideoTrack>,
    audio: Option<AudioTrack>,
    metrics: FlvMetrics,
}

impl FlvSource {
    pub fn new() -> Self {
        FlvSource::default()
    }

    pub fn metrics(&self) -> FlvMetrics {
        self.metrics
    }

    /// Takes the body of a video tag sent at `ts_ms`, received at `now`.
    pub fn video<'a>(
        &mut self,
        ts_ms: u32,
        body: &'a [u8],
        now: Time,
    ) -> Result<Option<SourceEvent<'a>>, FlvError> {
        let ms = self.clock.unwrap(ts_ms);
        let Some((&first, rest)) = body.split_first() else {
            return Ok(self.skip());
        };
        if first & VIDEO_EX_HEADER != 0 {
            return self.ex_video(ms, first, rest, now);
        }
        let frame_type = first >> 4;
        if frame_type == FRAME_COMMAND {
            return Ok(self.skip());
        }
        let codec = first & 0x0f;
        if codec != CODEC_AVC {
            return Err(FlvError::Unsupported(Unsupported::VideoCodecId(codec)));
        }
        let truncated = FlvError::Truncated(TrackKind::Video);
        let (&packet_type, rest) = rest.split_first().ok_or(truncated)?;
        match packet_type {
            AVC_END_OF_SEQUENCE => Ok(self.skip()),
            AVC_SEQUENCE_HEADER => {
                let (_, avcc) = rest.split_first_chunk::<3>().ok_or(truncated)?;
                self.video_config(avcc)
            }
            AVC_NALU => {
                let (cts, data) = rest.split_first_chunk::<3>().ok_or(truncated)?;
                Ok(self.video_unit(ms, si24(*cts), frame_type == FRAME_KEY, data, now))
            }
            _ => Err(FlvError::PacketType {
                kind: TrackKind::Video,
                packet_type,
            }),
        }
    }

    /// A video tag with an Enhanced RTMP header: frame type and packet
    /// type in `first`, then the FourCC.
    fn ex_video<'a>(
        &mut self,
        ms: i64,
        first: u8,
        rest: &'a [u8],
        now: Time,
    ) -> Result<Option<SourceEvent<'a>>, FlvError> {
        let frame_type = (first >> 4) & 0x07;
        let packet_type = first & 0x0f;
        match packet_type {
            VIDEO_MOD_EX => {
                return Err(FlvError::Unsupported(Unsupported::ModEx(TrackKind::Video)));
            }
            VIDEO_MULTITRACK => {
                return Err(FlvError::Unsupported(Unsupported::Multitrack(
                    TrackKind::Video,
                )));
            }
            // A command frame holds a command and no FourCC.
            _ if frame_type == FRAME_COMMAND && packet_type != VIDEO_METADATA => {
                return Ok(self.skip());
            }
            _ => {}
        }
        let truncated = FlvError::Truncated(TrackKind::Video);
        let (fourcc, rest) = rest.split_first_chunk::<4>().ok_or(truncated)?;
        if fourcc != b"avc1" {
            return Err(FlvError::Unsupported(Unsupported::VideoFourCc(*fourcc)));
        }
        match packet_type {
            VIDEO_SEQUENCE_START => self.video_config(rest),
            VIDEO_CODED_FRAMES => {
                let (cts, data) = rest.split_first_chunk::<3>().ok_or(truncated)?;
                Ok(self.video_unit(ms, si24(*cts), frame_type == FRAME_KEY, data, now))
            }
            VIDEO_CODED_FRAMES_X => Ok(self.video_unit(ms, 0, frame_type == FRAME_KEY, rest, now)),
            VIDEO_SEQUENCE_END | VIDEO_METADATA => Ok(self.skip()),
            _ => Err(FlvError::PacketType {
                kind: TrackKind::Video,
                packet_type,
            }),
        }
    }

    fn video_config<'a>(&mut self, avcc: &[u8]) -> Result<Option<SourceEvent<'a>>, FlvError> {
        if self
            .video
            .as_ref()
            .is_some_and(|t| config_bytes(&t.config) == avcc)
        {
            return Ok(self.skip());
        }
        let parsed = avc::parse_avcc(avcc).map_err(FlvError::Avc)?;
        let config = DecoderConfig {
            track: VIDEO_TRACK,
            kind: TrackKind::Video,
            codec: parsed.codec(),
            timescale: VIDEO_TIMESCALE,
            params: CodecParams::Avc {
                avcc: avcc.to_vec(),
                width: parsed.sps.width,
                height: parsed.sps.height,
            },
        };
        let last_dts = self.video.as_ref().and_then(|t| t.last_dts);
        self.video = Some(VideoTrack {
            config: config.clone(),
            last_dts,
        });
        Ok(Some(SourceEvent::Config(config)))
    }

    fn video_unit<'a>(
        &mut self,
        ms: i64,
        cts: i64,
        keyframe: bool,
        payload: &'a [u8],
        now: Time,
    ) -> Option<SourceEvent<'a>> {
        let Some(track) = &mut self.video else {
            self.metrics.before_config += 1;
            return None;
        };
        if payload.is_empty() {
            self.metrics.skipped += 1;
            return None;
        }
        let dts = clamp(
            &mut track.last_dts,
            ms.saturating_mul(VIDEO_UNITS_PER_MS),
            &mut self.metrics,
        );
        Some(SourceEvent::Unit(AccessUnit {
            track: VIDEO_TRACK,
            dts,
            pts: ms.saturating_add(cts).saturating_mul(VIDEO_UNITS_PER_MS),
            keyframe,
            payload,
            published: now,
        }))
    }

    /// Takes the body of an audio tag sent at `ts_ms`, received at `now`.
    pub fn audio<'a>(
        &mut self,
        ts_ms: u32,
        body: &'a [u8],
        now: Time,
    ) -> Result<Option<SourceEvent<'a>>, FlvError> {
        let ms = self.clock.unwrap(ts_ms);
        let Some((&first, rest)) = body.split_first() else {
            return Ok(self.skip());
        };
        match first >> 4 {
            SOUND_AAC => {
                let (&packet_type, data) = rest
                    .split_first()
                    .ok_or(FlvError::Truncated(TrackKind::Audio))?;
                match packet_type {
                    AAC_SEQUENCE_HEADER => self.audio_config(data),
                    AAC_RAW => Ok(self.audio_unit(ms, data, now)),
                    _ => Err(FlvError::PacketType {
                        kind: TrackKind::Audio,
                        packet_type,
                    }),
                }
            }
            SOUND_EX_HEADER => self.ex_audio(ms, first & 0x0f, rest, now),
            format => Err(FlvError::Unsupported(Unsupported::SoundFormat(format))),
        }
    }

    /// An audio tag with an Enhanced RTMP header: the packet type, then
    /// the FourCC.
    fn ex_audio<'a>(
        &mut self,
        ms: i64,
        packet_type: u8,
        rest: &'a [u8],
        now: Time,
    ) -> Result<Option<SourceEvent<'a>>, FlvError> {
        match packet_type {
            AUDIO_MOD_EX => {
                return Err(FlvError::Unsupported(Unsupported::ModEx(TrackKind::Audio)));
            }
            AUDIO_MULTITRACK => {
                return Err(FlvError::Unsupported(Unsupported::Multitrack(
                    TrackKind::Audio,
                )));
            }
            _ => {}
        }
        let (fourcc, data) = rest
            .split_first_chunk::<4>()
            .ok_or(FlvError::Truncated(TrackKind::Audio))?;
        if fourcc != b"mp4a" {
            return Err(FlvError::Unsupported(Unsupported::AudioFourCc(*fourcc)));
        }
        match packet_type {
            AUDIO_SEQUENCE_START => self.audio_config(data),
            AUDIO_CODED_FRAMES => Ok(self.audio_unit(ms, data, now)),
            // The channel order is the AudioSpecificConfig's.
            AUDIO_SEQUENCE_END | AUDIO_MULTICHANNEL_CONFIG => Ok(self.skip()),
            _ => Err(FlvError::PacketType {
                kind: TrackKind::Audio,
                packet_type,
            }),
        }
    }

    fn audio_config<'a>(&mut self, asc: &[u8]) -> Result<Option<SourceEvent<'a>>, FlvError> {
        if self
            .audio
            .as_ref()
            .is_some_and(|t| config_bytes(&t.config) == asc)
        {
            return Ok(self.skip());
        }
        let parsed = aac::parse_asc(asc).map_err(|e| match e {
            AacError::ObjectType(aot) => FlvError::Unsupported(Unsupported::AacObjectType(aot)),
            e => FlvError::Aac(e),
        })?;
        let config = DecoderConfig {
            track: AUDIO_TRACK,
            kind: TrackKind::Audio,
            codec: parsed.codec(),
            timescale: parsed.sample_rate,
            params: CodecParams::Aac {
                asc: asc.to_vec(),
                sample_rate: parsed.sample_rate,
                channels: parsed.channels,
                frame_samples: parsed.frame_samples,
            },
        };
        // A DTS counts samples at the old rate; another rate starts over.
        let last_dts = self
            .audio
            .as_ref()
            .filter(|t| t.sample_rate == parsed.sample_rate)
            .and_then(|t| t.last_dts);
        self.audio = Some(AudioTrack {
            config: config.clone(),
            sample_rate: parsed.sample_rate,
            frame_samples: parsed.frame_samples,
            last_dts,
        });
        Ok(Some(SourceEvent::Config(config)))
    }

    fn audio_unit<'a>(&mut self, ms: i64, payload: &'a [u8], now: Time) -> Option<SourceEvent<'a>> {
        let Some(track) = &mut self.audio else {
            self.metrics.before_config += 1;
            return None;
        };
        if payload.is_empty() {
            self.metrics.skipped += 1;
            return None;
        }
        let at_tag_time = (i128::from(ms) * i128::from(track.sample_rate)).div_euclid(1000);
        let at_tag_time =
            i64::try_from(at_tag_time).unwrap_or(if ms < 0 { i64::MIN } else { i64::MAX });
        let frame = i64::from(track.frame_samples);
        let counted = match track.last_dts {
            None => at_tag_time,
            Some(last) => {
                let next = last.saturating_add(frame);
                if next.abs_diff(at_tag_time) > frame.unsigned_abs() {
                    self.metrics.audio_resyncs += 1;
                    at_tag_time
                } else {
                    next
                }
            }
        };
        let dts = clamp(&mut track.last_dts, counted, &mut self.metrics);
        Some(SourceEvent::Unit(AccessUnit {
            track: AUDIO_TRACK,
            dts,
            pts: dts,
            keyframe: true,
            payload,
            published: now,
        }))
    }

    fn skip<'a>(&mut self) -> Option<SourceEvent<'a>> {
        self.metrics.skipped += 1;
        None
    }
}

/// `dts`, or the track's previous DTS when that is later; records the
/// result as the previous DTS.
fn clamp(last: &mut Option<i64>, dts: i64, metrics: &mut FlvMetrics) -> i64 {
    let dts = match *last {
        Some(prev) if dts < prev => {
            metrics.dts_clamped += 1;
            prev
        }
        _ => dts,
    };
    *last = Some(dts);
    dts
}

/// A signed 24-bit big-endian number.
fn si24([a, b, c]: [u8; 3]) -> i64 {
    i64::from(i32::from_be_bytes([a, b, c, 0]) >> 8)
}

/// The codec record a config was built from.
fn config_bytes(config: &DecoderConfig) -> &[u8] {
    match &config.params {
        CodecParams::Avc { avcc, .. } => avcc,
        CodecParams::Aac { asc, .. } => asc,
    }
}

#[cfg(test)]
#[path = "flv_test.rs"]
mod tests;
