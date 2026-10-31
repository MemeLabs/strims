// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A source over the bodies of FLV video and audio tags, as RTMP carries
//! them: H.264, H.265 or AV1 video and AAC audio, in the legacy layout or
//! Enhanced RTMP's FourCC headers.
//!
//! - **Video** is track [`VIDEO_TRACK`] on a 90 kHz timescale. The legacy
//!   AVC packets (codec id 7) and Enhanced RTMP's `avc1` and `hvc1`
//!   packets carry the avcC or hvcC sequence header and frames with a
//!   signed 24-bit composition time in milliseconds: DTS is the tag time,
//!   PTS the tag time plus the composition time. Enhanced RTMP's `av01`
//!   packets carry the av1C sequence header, which must hold a sequence
//!   header OBU, and temporal units without a composition time, less the
//!   temporal delimiter OBUs they start with. Frames packed without a
//!   composition time present at their decode time. The FLV frame type
//!   marks keyframes. A frame of a codec other than its track's config's is
//!   dropped as one before the track's config.
//! - **Audio** is track [`AUDIO_TRACK`] on a timescale of its output
//!   sample rate. The legacy AAC packets (sound format 10) and Enhanced
//!   RTMP's `mp4a` packets carry the AudioSpecificConfig and raw frames.
//!   Each frame starts a frame's samples after the one before; when that
//!   count drifts from the tag time by more than a frame, it restarts at
//!   the tag time.
//! - A multitrack source ([`FlvSource::multitrack`]) also takes Enhanced
//!   RTMP's multitrack video packets that each carry one track (OneTrack),
//!   of track ids below its track count. Each track has a config, a DTS
//!   clamp and clock steps of its own, and [`FlvSource::video_track`] says
//!   which track an event is of; tags without a multitrack header are
//!   track 0's.
//! - Other codecs, Enhanced RTMP multitrack packets a source doesn't take
//!   (any for a source of one video track, several tracks in one packet,
//!   and audio) and ModEx packets are refused with
//!   [`FlvError::Unsupported`], whose message tells the streamer what
//!   to send. Command frames, sequence ends, Enhanced RTMP metadata, AV1's
//!   MPEG-2 TS sequence starts and empty tags carry nothing and are
//!   skipped.
//!
//! Tag times are 32-bit milliseconds that wrap; each is taken as the time
//! nearest the previous tag's, on one clock for both tracks. A tag more
//! than [`CLOCK_RESTART`] before its track's previous tag is an encoder
//! that started its clock over: the clock moves both tracks' later tags so
//! that this one comes a step of its track's after the latest time either
//! track reached, the step between its track's last two tags (at most
//! [`CLOCK_RESTART`]). A track's
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
use crate::av1::{self, Av1Error};
use crate::avc::{self, AvcError};
use crate::hevc::{self, HevcError};
use crate::media::{AccessUnit, CodecParams, DecoderConfig, TrackId, TrackKind};

/// The video track's id.
pub const VIDEO_TRACK: TrackId = TrackId(1);
/// The audio track's id.
pub const AUDIO_TRACK: TrackId = TrackId(2);
/// The video track's timescale.
pub const VIDEO_TIMESCALE: u32 = 90_000;
/// Video timescale units per tag millisecond.
const VIDEO_UNITS_PER_MS: i64 = VIDEO_TIMESCALE as i64 / 1000;
/// How far, in milliseconds, a tag's time may fall behind its track's
/// previous tag's before the clock takes it as an encoder's clock started
/// over. A track's tags come in decode order, so they step back only by
/// an encoder's jitter, which the DTS clamp absorbs.
pub const CLOCK_RESTART: i64 = 1000;
/// A track's step before its second tag, in milliseconds.
const DEFAULT_STEP: i64 = 1;
/// The most video tracks a multitrack source takes.
pub const MAX_VIDEO_TRACKS: u8 = 8;
/// The Enhanced RTMP multitrack type of a packet carrying one track.
const MULTITRACK_ONE_TRACK: u8 = 0;

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
const VIDEO_MPEG2TS_SEQUENCE_START: u8 = 5;
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
    /// An Enhanced RTMP video FourCC other than `avc1`, `hvc1` and `av01`.
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
    #[error("invalid HEVC sequence header: {0}")]
    Hevc(HevcError),
    #[error("invalid AV1 sequence header: {0}")]
    Av1(Av1Error),
    #[error("invalid AAC sequence header: {0}")]
    Aac(AacError),
    /// A multitrack packet of a video track id at or past the source's
    /// track count.
    #[error("video track {0} is out of range")]
    VideoTrack(u8),
}

/// What a source has skipped, dropped and adjusted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlvMetrics {
    /// Tags that carry nothing new: command frames, sequence ends,
    /// metadata, empty tags and repeated sequence headers.
    pub skipped: u64,
    /// Frames dropped because their track had no decoder config of their
    /// codec yet.
    pub before_config: u64,
    /// Units whose DTS was raised to the previous unit's.
    pub dts_clamped: u64,
    /// Times the audio sample count restarted at the tag time.
    pub audio_resyncs: u64,
    /// Times the encoder's clock started over.
    pub clock_restarts: u64,
}

/// A track the clock times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// The video track of an Enhanced RTMP track id.
    Video(u8),
    Audio,
}

/// Turns 32-bit tag times into 64-bit ones, each the value nearest the
/// previous tag's, moved past the encoder's restarts.
#[derive(Debug)]
struct Clock {
    /// The previous tag's raw and unwrapped times.
    last: Option<(u32, i64)>,
    /// What the restarts moved the unwrapped times by.
    offset: i64,
    /// The latest time any track reached.
    latest: Option<i64>,
    /// Each track's previous frame's time and its step: the video tracks
    /// by track id, then audio.
    video: Vec<Option<(i64, i64)>>,
    audio: Option<(i64, i64)>,
}

impl Clock {
    fn new(video_tracks: usize) -> Clock {
        Clock {
            last: None,
            offset: 0,
            latest: None,
            video: vec![None; video_tracks],
            audio: None,
        }
    }

    /// The unwrapped time of a tag sent at `ts`.
    fn unwrap(&mut self, ts: u32) -> i64 {
        let unwrapped = match self.last {
            None => i64::from(ts),
            // A wrapped difference read as signed: within 2^31 ms either way.
            Some((raw, ms)) => ms.saturating_add(i64::from(ts.wrapping_sub(raw) as i32)),
        };
        self.last = Some((ts, unwrapped));
        unwrapped
    }

    /// The time of a frame of `slot` at `unwrapped`, and whether it started
    /// the clock over.
    fn place(&mut self, slot: Slot, unwrapped: i64) -> (i64, bool) {
        let mut ms = unwrapped.saturating_add(self.offset);
        let mut none = None;
        let track = match slot {
            Slot::Video(id) => self.video.get_mut(usize::from(id)).unwrap_or(&mut none),
            Slot::Audio => &mut self.audio,
        };
        let restart = match (*track, self.latest) {
            (Some((prev, step)), Some(latest)) if ms < prev.saturating_sub(CLOCK_RESTART) => {
                Some(latest.saturating_add(step))
            }
            _ => None,
        };
        let step = match *track {
            Some((_, step)) if restart.is_some() => step,
            Some((prev, step)) if ms <= prev => step,
            Some((prev, _)) => ms.saturating_sub(prev).min(CLOCK_RESTART),
            None => DEFAULT_STEP,
        };
        if let Some(to) = restart {
            self.offset = self.offset.saturating_add(to.saturating_sub(ms));
            ms = to;
        }
        *track = Some((ms, step));
        self.latest = Some(self.latest.map_or(ms, |l| l.max(ms)));
        (ms, restart.is_some())
    }
}

/// A video codec the source takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VideoCodec {
    Avc,
    Hevc,
    Av1,
}

impl VideoCodec {
    /// The codec of an Enhanced RTMP FourCC.
    fn of(fourcc: &[u8; 4]) -> Option<VideoCodec> {
        match fourcc {
            b"avc1" => Some(VideoCodec::Avc),
            b"hvc1" => Some(VideoCodec::Hevc),
            b"av01" => Some(VideoCodec::Av1),
            _ => None,
        }
    }
}

/// A video frame's place: its track, codec, unwrapped tag time,
/// composition time and keyframe flag.
#[derive(Debug, Clone, Copy)]
struct Frame {
    track: u8,
    codec: VideoCodec,
    ms: i64,
    cts: i64,
    keyframe: bool,
}

#[derive(Debug)]
struct VideoTrack {
    config: DecoderConfig,
    codec: VideoCodec,
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
#[derive(Debug)]
pub struct FlvSource {
    clock: Clock,
    /// The video tracks, by Enhanced RTMP track id. Bound:
    /// [`MAX_VIDEO_TRACKS`].
    video: Vec<Option<VideoTrack>>,
    audio: Option<AudioTrack>,
    metrics: FlvMetrics,
}

impl Default for FlvSource {
    fn default() -> Self {
        FlvSource::multitrack(1)
    }
}

impl FlvSource {
    /// A source of one video track, which refuses multitrack packets.
    pub fn new() -> Self {
        FlvSource::default()
    }

    /// A source taking `video_tracks` video tracks, 1 to
    /// [`MAX_VIDEO_TRACKS`] (clamped), of multitrack packets that each
    /// carry one.
    pub fn multitrack(video_tracks: u8) -> Self {
        let n = usize::from(video_tracks.clamp(1, MAX_VIDEO_TRACKS));
        FlvSource {
            clock: Clock::new(n),
            video: (0..n).map(|_| None).collect(),
            audio: None,
            metrics: FlvMetrics::default(),
        }
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
        Ok(self.video_track(ts_ms, body, now)?.map(|(_, e)| e))
    }

    /// Takes the body of a video tag sent at `ts_ms`, received at `now`:
    /// its event, and the track id of the video track it is of.
    pub fn video_track<'a>(
        &mut self,
        ts_ms: u32,
        body: &'a [u8],
        now: Time,
    ) -> Result<Option<(u8, SourceEvent<'a>)>, FlvError> {
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
                self.video_config(0, VideoCodec::Avc, avcc)
            }
            AVC_NALU => {
                let (cts, data) = rest.split_first_chunk::<3>().ok_or(truncated)?;
                let key = frame_type == FRAME_KEY;
                let frame = Frame {
                    track: 0,
                    codec: VideoCodec::Avc,
                    ms,
                    cts: si24(*cts),
                    keyframe: key,
                };
                Ok(self.video_unit(frame, data, now))
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
    ) -> Result<Option<(u8, SourceEvent<'a>)>, FlvError> {
        let frame_type = (first >> 4) & 0x07;
        let packet_type = first & 0x0f;
        match packet_type {
            VIDEO_MOD_EX => {
                return Err(FlvError::Unsupported(Unsupported::ModEx(TrackKind::Video)));
            }
            VIDEO_MULTITRACK => return self.multitrack_video(ms, frame_type, rest, now),
            // A command frame holds a command and no FourCC.
            _ if frame_type == FRAME_COMMAND && packet_type != VIDEO_METADATA => {
                return Ok(self.skip());
            }
            _ => {}
        }
        let truncated = FlvError::Truncated(TrackKind::Video);
        let (fourcc, rest) = rest.split_first_chunk::<4>().ok_or(truncated)?;
        let codec = VideoCodec::of(fourcc)
            .ok_or(FlvError::Unsupported(Unsupported::VideoFourCc(*fourcc)))?;
        self.ex_packet(0, codec, frame_type, packet_type, ms, rest, now)
    }

    /// A multitrack video tag, after its first byte: the multitrack type
    /// and packet type, the FourCC, the track id, and the track's packet.
    fn multitrack_video<'a>(
        &mut self,
        ms: i64,
        frame_type: u8,
        rest: &'a [u8],
        now: Time,
    ) -> Result<Option<(u8, SourceEvent<'a>)>, FlvError> {
        let refused = FlvError::Unsupported(Unsupported::Multitrack(TrackKind::Video));
        if self.video.len() < 2 {
            return Err(refused);
        }
        let truncated = FlvError::Truncated(TrackKind::Video);
        let (&kinds, rest) = rest.split_first().ok_or(truncated)?;
        if kinds >> 4 != MULTITRACK_ONE_TRACK {
            return Err(refused);
        }
        let packet_type = kinds & 0x0f;
        if matches!(packet_type, VIDEO_MULTITRACK | VIDEO_MOD_EX) {
            return Err(FlvError::PacketType {
                kind: TrackKind::Video,
                packet_type,
            });
        }
        let (fourcc, rest) = rest.split_first_chunk::<4>().ok_or(truncated)?;
        let codec = VideoCodec::of(fourcc)
            .ok_or(FlvError::Unsupported(Unsupported::VideoFourCc(*fourcc)))?;
        let (&track, rest) = rest.split_first().ok_or(truncated)?;
        if usize::from(track) >= self.video.len() {
            return Err(FlvError::VideoTrack(track));
        }
        if frame_type == FRAME_COMMAND && packet_type != VIDEO_METADATA {
            return Ok(self.skip());
        }
        self.ex_packet(track, codec, frame_type, packet_type, ms, rest, now)
    }

    /// The Enhanced RTMP packet of `packet_type` that `rest` holds, of
    /// video track `track` in `codec`.
    #[allow(clippy::too_many_arguments)]
    fn ex_packet<'a>(
        &mut self,
        track: u8,
        codec: VideoCodec,
        frame_type: u8,
        packet_type: u8,
        ms: i64,
        rest: &'a [u8],
        now: Time,
    ) -> Result<Option<(u8, SourceEvent<'a>)>, FlvError> {
        let truncated = FlvError::Truncated(TrackKind::Video);
        let frame = |cts| Frame {
            track,
            codec,
            ms,
            cts,
            keyframe: frame_type == FRAME_KEY,
        };
        match packet_type {
            VIDEO_SEQUENCE_START => self.video_config(track, codec, rest),
            // AV1's frames carry no composition time.
            VIDEO_CODED_FRAMES if codec != VideoCodec::Av1 => {
                let (cts, data) = rest.split_first_chunk::<3>().ok_or(truncated)?;
                Ok(self.video_unit(frame(si24(*cts)), data, now))
            }
            VIDEO_CODED_FRAMES | VIDEO_CODED_FRAMES_X => Ok(self.video_unit(frame(0), rest, now)),
            VIDEO_SEQUENCE_END | VIDEO_METADATA => Ok(self.skip()),
            VIDEO_MPEG2TS_SEQUENCE_START if codec == VideoCodec::Av1 => Ok(self.skip()),
            _ => Err(FlvError::PacketType {
                kind: TrackKind::Video,
                packet_type,
            }),
        }
    }

    fn video_config<'a>(
        &mut self,
        track: u8,
        codec: VideoCodec,
        record: &[u8],
    ) -> Result<Option<(u8, SourceEvent<'a>)>, FlvError> {
        let Some(slot) = self.video.get(usize::from(track)) else {
            return Err(FlvError::VideoTrack(track));
        };
        if slot
            .as_ref()
            .is_some_and(|t| t.codec == codec && t.config.params.record() == record)
        {
            return Ok(self.skip());
        }
        let (codec_string, params) = match codec {
            VideoCodec::Avc => {
                let parsed = avc::parse_avcc(record).map_err(FlvError::Avc)?;
                let params = CodecParams::Avc {
                    avcc: record.to_vec(),
                    width: parsed.sps.width,
                    height: parsed.sps.height,
                };
                (parsed.codec(), params)
            }
            VideoCodec::Hevc => {
                let parsed = hevc::parse_hvcc(record).map_err(FlvError::Hevc)?;
                let params = CodecParams::Hevc {
                    hvcc: record.to_vec(),
                    width: parsed.sps.width,
                    height: parsed.sps.height,
                };
                (parsed.codec(), params)
            }
            VideoCodec::Av1 => {
                let parsed = av1::parse_av1c(record).map_err(FlvError::Av1)?;
                let params = CodecParams::Av1 {
                    av1c: record.to_vec(),
                    width: parsed.width,
                    height: parsed.height,
                };
                (parsed.codec(), params)
            }
        };
        let config = DecoderConfig {
            track: VIDEO_TRACK,
            kind: TrackKind::Video,
            codec: codec_string,
            timescale: VIDEO_TIMESCALE,
            params,
        };
        let Some(slot) = self.video.get_mut(usize::from(track)) else {
            return Err(FlvError::VideoTrack(track));
        };
        let last_dts = slot.as_ref().and_then(|t| t.last_dts);
        *slot = Some(VideoTrack {
            config: config.clone(),
            codec,
            last_dts,
        });
        Ok(Some((track, SourceEvent::Config(config))))
    }

    fn video_unit<'a>(
        &mut self,
        frame: Frame,
        payload: &'a [u8],
        now: Time,
    ) -> Option<(u8, SourceEvent<'a>)> {
        let Some(Some(track)) = self
            .video
            .get_mut(usize::from(frame.track))
            .filter(|t| t.as_ref().is_some_and(|t| t.codec == frame.codec))
        else {
            self.metrics.before_config += 1;
            return None;
        };
        let payload = match frame.codec {
            VideoCodec::Av1 => av1::strip_temporal_delimiters(payload),
            VideoCodec::Avc | VideoCodec::Hevc => payload,
        };
        if payload.is_empty() {
            self.metrics.skipped += 1;
            return None;
        }
        let (ms, restarted) = self.clock.place(Slot::Video(frame.track), frame.ms);
        if restarted {
            self.metrics.clock_restarts += 1;
        }
        let dts = clamp(
            &mut track.last_dts,
            ms.saturating_mul(VIDEO_UNITS_PER_MS),
            &mut self.metrics,
        );
        let unit = AccessUnit {
            track: VIDEO_TRACK,
            dts,
            pts: ms
                .saturating_add(frame.cts)
                .saturating_mul(VIDEO_UNITS_PER_MS),
            keyframe: frame.keyframe,
            payload,
            published: now,
        };
        Some((frame.track, SourceEvent::Unit(unit)))
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
            .is_some_and(|t| t.config.params.record() == asc)
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
        let (ms, restarted) = self.clock.place(Slot::Audio, ms);
        if restarted {
            self.metrics.clock_restarts += 1;
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

    fn skip<T>(&mut self) -> Option<T> {
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

#[cfg(test)]
#[path = "flv_test.rs"]
mod tests;
