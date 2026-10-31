// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The codec stage over an ffmpeg process per transcode, FLV through its
//! pipes both ways.
//!
//! - **Input.** The stream's configs and units go to ffmpeg's standard
//!   input as an FLV file ([`FlvWriter`]): AVC and AAC in the legacy
//!   layout, H.265 and AV1 in Enhanced RTMP's. Tag times start at
//!   [`START_MS`] and count the stream's decode times from its first unit,
//!   so B-frame reordering never takes a decode time below zero, which
//!   ffmpeg's FLV muxer would shift its output by.
//! - **Process.** ffmpeg reads only its pipes (`-protocol_whitelist
//!   pipe`), keeps the input's times (`-copyts`), and decodes the video
//!   once. Each rendition selects its frames from the decoded ones (every
//!   keyframe, and each frame at least a frame of its rate after the last
//!   it kept, less 2 ms of jitter) and scales them, without resampling
//!   their times (`-fps_mode passthrough`), and is encoded with
//!   [`FfmpegSettings::video_encoder`], a keyframe wherever the input has
//!   one (`-force_key_frames source`). The audio is copied. Packets go
//!   out as they come (`-max_interleave_delta 0`, `-flush_packets 1`).
//! - **Output.** ffmpeg writes one FLV file to its standard output:
//!   rendition 0's video in the legacy layout, each later rendition's as
//!   Enhanced RTMP multitrack video of its index, and the audio once. It
//!   is read as it comes ([`FlvTagReader`]) through a multitrack
//!   [`FlvSource`], its times moved back onto the stream's, and its audio
//!   given to every rendition.
//! - **Thumbnails** run an ffmpeg of their own, given the configs and a
//!   keyframe, which writes a JPEG.
//!
//! ffmpeg 7.1 or later writes multitrack FLV; the driver checks its
//! version before it offers the stage.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::future::poll_fn;
use std::rc::Rc;
use std::task::{Context, Poll};

use strims_base::time::Time;

use super::{
    ChildProcess, Codec, CodecEvent, Input, MAX_RENDITIONS, Output, Processes, Refused, Thumbnail,
    Transcode, TranscodePlan, Unit,
};
use crate::flv::{FlvSource, SourceEvent};
use crate::flv_file::{FlvTagReader, FlvWriter, TAG_AUDIO, TAG_VIDEO};
use crate::media::{AccessUnit, DecoderConfig, TrackKind};
use crate::units::MAX_RECORD_LEN;

/// The time, in milliseconds, of a transcode's first input tag.
pub const START_MS: u32 = 1000;
/// The bytes read from ffmpeg's output at a time.
const READ_LEN: usize = 64 * 1024;
/// The units whose publish times a transcode keeps, for the output units
/// of their times.
const MAX_PUBLISHED: usize = 1024;
/// How far below a frame interval a rendition's frames may come, in
/// seconds: the tag times' rounding to milliseconds and an encoder's
/// jitter.
const SELECT_SLACK: f64 = 0.002;

/// How a node runs ffmpeg.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegSettings {
    /// The video encoder and its options: the arguments after `-c:v`.
    /// They apply to every rendition, and each rendition adds its
    /// bitrate. The encoder must take `-force_key_frames source`, and
    /// leave out the stream's captions, which the stage carries itself.
    pub video_encoder: Vec<String>,
    /// The input bytes a transcode queues before it refuses more: about 8
    /// s of an 8 Mbit/s stream.
    pub max_queued: usize,
}

impl Default for FfmpegSettings {
    /// libx264 at the `veryfast` preset, tuned for low latency (no
    /// B-frames, no lookahead, sliced threads: about 0.3 s from a frame
    /// in to its renditions out), with keyframes only where the input has
    /// them and no captions of its own.
    fn default() -> Self {
        let args = [
            "libx264",
            "-preset",
            "veryfast",
            "-tune",
            "zerolatency",
            "-x264-params",
            "scenecut=0:keyint=infinite",
            "-a53cc",
            "0",
        ];
        FfmpegSettings {
            video_encoder: args.into_iter().map(String::from).collect(),
            max_queued: 8 * 1024 * 1024,
        }
    }
}

/// The arguments of ffmpeg's transcode of `plan`.
pub fn transcode_args(settings: &FfmpegSettings, plan: &TranscodePlan) -> Vec<String> {
    let mut args = input_args();
    let n = plan.renditions.len();
    let mut graph = format!("[0:v]split={n}");
    for i in 0..n {
        graph.push_str(&format!("[s{i}]"));
    }
    for (i, r) in plan.renditions.iter().enumerate() {
        let interval = (1.0 / f64::from(r.max_frame_rate.max(1)) - SELECT_SLACK).max(0.0);
        let side = r.short_side;
        graph.push_str(&format!(
            ";[s{i}]select='key+isnan(prev_selected_t)+gte(t-prev_selected_t\\,{interval:.4})',\
             scale='if(gt(iw\\,ih)\\,-2\\,{side})':'if(gt(iw\\,ih)\\,{side}\\,-2)'[o{i}]"
        ));
    }
    args.extend(["-filter_complex".into(), graph]);
    for i in 0..n {
        args.extend(["-map".into(), format!("[o{i}]")]);
    }
    for s in [
        "-map",
        "0:a?",
        "-c:a",
        "copy",
        "-fps_mode",
        "passthrough",
        "-c:v",
    ] {
        args.push(s.into());
    }
    args.extend(settings.video_encoder.iter().cloned());
    args.extend(["-force_key_frames".into(), "source".into()]);
    for (i, r) in plan.renditions.iter().enumerate() {
        args.extend([
            format!("-b:v:{i}"),
            r.bitrate.to_string(),
            format!("-maxrate:v:{i}"),
            r.bitrate.to_string(),
            format!("-bufsize:v:{i}"),
            r.bitrate.saturating_mul(2).to_string(),
        ]);
    }
    for s in [
        "-max_interleave_delta",
        "0",
        "-flush_packets",
        "1",
        "-flvflags",
        "no_duration_filesize",
        "-f",
        "flv",
        "pipe:1",
    ] {
        args.push(s.into());
    }
    args
}

/// The arguments of ffmpeg's JPEG of the first picture of its input,
/// `width` pixels wide.
pub fn thumbnail_args(width: u16) -> Vec<String> {
    let mut args = input_args();
    for s in [
        "-frames:v".to_owned(),
        "1".to_owned(),
        "-vf".to_owned(),
        format!("scale={width}:-2"),
        "-c:v".to_owned(),
        "mjpeg".to_owned(),
        "-q:v".to_owned(),
        "5".to_owned(),
        "-f".to_owned(),
        "image2pipe".to_owned(),
        "pipe:1".to_owned(),
    ] {
        args.push(s);
    }
    args
}

/// Every ffmpeg's arguments up to its input: errors alone on standard
/// error, pipes only, and an FLV input read as it comes, its times kept.
fn input_args() -> Vec<String> {
    [
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostats",
        "-nostdin",
        "-protocol_whitelist",
        "pipe",
        "-copyts",
        "-fflags",
        "+nobuffer",
        "-probesize",
        "32768",
        "-analyzeduration",
        "0",
        "-f",
        "flv",
        "-i",
        "pipe:0",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// Why ffmpeg's codec stage failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FfmpegError<E: fmt::Display> {
    #[error("ffmpeg didn't start: {0}")]
    Spawn(E),
    #[error("ffmpeg's pipes failed: {0}")]
    Pipe(E),
    /// Its output isn't what a transcode writes.
    #[error("ffmpeg's output is malformed: {0}")]
    Output(String),
    /// A plan of no renditions or more than [`MAX_RENDITIONS`].
    #[error("a transcode makes 1 to {MAX_RENDITIONS} renditions")]
    Plan,
    #[error("ffmpeg made no picture: {0}")]
    NoPicture(String),
    #[error("ffmpeg's picture is over {0} bytes")]
    PictureTooLarge(usize),
    #[error("the keyframe can't go to ffmpeg: {0}")]
    Input(String),
}

/// The codec stage over ffmpeg processes `P` runs; see the
/// [module docs](self).
#[derive(Clone)]
pub struct Ffmpeg<P: Processes> {
    processes: P,
    settings: Rc<FfmpegSettings>,
}

impl<P: Processes> Ffmpeg<P> {
    pub fn new(processes: P, settings: FfmpegSettings) -> Self {
        Ffmpeg {
            processes,
            settings: Rc::new(settings),
        }
    }

    pub fn settings(&self) -> &FfmpegSettings {
        &self.settings
    }
}

impl<P: Processes> Codec for Ffmpeg<P> {
    type Error = FfmpegError<P::Error>;
    type Transcode = FfmpegTranscode<P::Child>;

    fn transcode(&self, plan: &TranscodePlan) -> Result<Self::Transcode, Self::Error> {
        let n = plan.renditions.len();
        if n == 0 || n > MAX_RENDITIONS {
            return Err(FfmpegError::Plan);
        }
        let child = self
            .processes
            .spawn(&transcode_args(&self.settings, plan))
            .map_err(FfmpegError::Spawn)?;
        Ok(FfmpegTranscode::new(child, n, self.settings.max_queued))
    }

    fn thumbnail(
        &self,
        configs: &[DecoderConfig],
        keyframe: &AccessUnit<'_>,
        width: u16,
        max_bytes: usize,
    ) -> Thumbnail<Self::Error> {
        let input = thumbnail_input(configs, keyframe);
        let child = self.processes.spawn(&thumbnail_args(width));
        Box::pin(async move {
            let input = input?;
            let mut child = child.map_err(FfmpegError::Spawn)?;
            let mut written = 0;
            let mut out = Vec::new();
            let mut buf = vec![0; READ_LEN];
            poll_fn(|cx| {
                while written < input.len() {
                    match child.poll_write(cx, input.get(written..).unwrap_or_default()) {
                        Poll::Ready(Ok(0)) => break,
                        Poll::Ready(Ok(n)) => written += n,
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(FfmpegError::Pipe(e))),
                        Poll::Pending => break,
                    }
                }
                if written >= input.len() {
                    child.close_input();
                }
                loop {
                    match child.poll_read(cx, &mut buf) {
                        Poll::Ready(Ok(0)) => return Poll::Ready(Ok(())),
                        Poll::Ready(Ok(n)) => {
                            out.extend_from_slice(buf.get(..n).unwrap_or_default());
                            if out.len() > max_bytes {
                                return Poll::Ready(Err(FfmpegError::PictureTooLarge(max_bytes)));
                            }
                        }
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(FfmpegError::Pipe(e))),
                        Poll::Pending => return Poll::Pending,
                    }
                }
            })
            .await?;
            if out.is_empty() {
                return Err(FfmpegError::NoPicture(child.diagnostics()));
            }
            Ok(out)
        })
    }
}

/// The FLV ffmpeg takes for a thumbnail: the header, the configs and the
/// keyframe.
fn thumbnail_input<E: fmt::Display>(
    configs: &[DecoderConfig],
    keyframe: &AccessUnit<'_>,
) -> Result<Vec<u8>, FfmpegError<E>> {
    let config = configs
        .iter()
        .find(|c| c.track == keyframe.track && c.kind == TrackKind::Video)
        .ok_or_else(|| FfmpegError::Input("no video config of its track".into()))?;
    let mut w = FlvWriter::new(START_MS);
    let mut out = Vec::new();
    w.header(std::slice::from_ref(config), None, &mut out);
    let input = |e: crate::flv_file::FlvWriteError| FfmpegError::Input(e.to_string());
    w.config(config, &mut out).map_err(input)?;
    w.unit(config, keyframe, &mut out).map_err(input)?;
    Ok(out)
}

/// A transcode through an ffmpeg process; see the [module docs](self).
pub struct FfmpegTranscode<C: ChildProcess> {
    child: C,
    renditions: usize,
    writer: FlvWriter,
    /// The stream's configs, by track.
    configs: Vec<DecoderConfig>,
    /// The header and configs are written.
    started: bool,
    /// Input ffmpeg hasn't taken. Bound: `max_queued`, past which `send`
    /// refuses.
    queue: VecDeque<u8>,
    max_queued: usize,
    finishing: bool,
    input_closed: bool,
    reader: FlvTagReader,
    source: FlvSource,
    read_buf: Box<[u8]>,
    /// Events read and not yet yielded: at most a read's.
    events: VecDeque<CodecEvent>,
    /// The publish times of the stream's latest units, by kind (video
    /// first) and presentation time in milliseconds. Bound:
    /// [`MAX_PUBLISHED`].
    published: BTreeMap<(TrackKind, i64), Time>,
    /// The latest publish time sent.
    latest_published: Time,
    /// The output audio's timescale.
    audio_timescale: u32,
    ended: bool,
}

impl<C: ChildProcess> FfmpegTranscode<C> {
    fn new(child: C, renditions: usize, max_queued: usize) -> Self {
        FfmpegTranscode {
            child,
            renditions,
            writer: FlvWriter::new(START_MS),
            configs: Vec::new(),
            started: false,
            queue: VecDeque::new(),
            max_queued,
            finishing: false,
            input_closed: false,
            reader: FlvTagReader::new(MAX_RECORD_LEN),
            source: FlvSource::multitrack(u8::try_from(renditions).unwrap_or(u8::MAX)),
            read_buf: vec![0; READ_LEN].into_boxed_slice(),
            events: VecDeque::new(),
            published: BTreeMap::new(),
            latest_published: Time::default(),
            audio_timescale: 0,
            ended: false,
        }
    }

    /// The milliseconds an output time is ahead of the stream's: the
    /// start time less the first unit's DTS.
    fn shift_ms(&self) -> i64 {
        i64::from(START_MS) - self.writer.base().unwrap_or(0)
    }

    /// Takes ffmpeg's output tags, as events.
    fn read_tags(&mut self) -> Result<(), String> {
        loop {
            let tag = match self.reader.next_tag() {
                Ok(Some(tag)) => tag,
                Ok(None) => return Ok(()),
                Err(e) => return Err(e.to_string()),
            };
            let latest = self.latest_published;
            let (rendition, event) = match tag.kind {
                TAG_VIDEO => {
                    let e = self.source.video_track(tag.timestamp, tag.body, latest);
                    match e.map_err(|e| e.to_string())? {
                        Some((track, event)) => (Some(usize::from(track)), Output::of(event)),
                        None => continue,
                    }
                }
                TAG_AUDIO => {
                    let e = self.source.audio(tag.timestamp, tag.body, latest);
                    match e.map_err(|e| e.to_string())? {
                        Some(event) => (None, Output::of(event)),
                        None => continue,
                    }
                }
                _ => continue,
            };
            let output = self.on_stream(event);
            match rendition {
                Some(rendition) => self.events.push_back(CodecEvent { rendition, output }),
                // The audio is every rendition's.
                None => {
                    for rendition in 0..self.renditions {
                        self.events.push_back(CodecEvent {
                            rendition,
                            output: output.clone(),
                        });
                    }
                }
            }
        }
    }

    /// An output event on the stream's timeline: a unit's times moved back
    /// by the input's shift, and its publish time the stream's unit's of
    /// its time or the latest before it.
    fn on_stream(&mut self, event: Output) -> Output {
        let mut unit = match event {
            Output::Config(c) => {
                if c.kind == TrackKind::Audio {
                    self.audio_timescale = c.timescale;
                }
                return Output::Config(c);
            }
            Output::Unit(u) => u,
        };
        let video = unit.track == crate::flv::VIDEO_TRACK;
        let (kind, timescale) = if video {
            (TrackKind::Video, crate::flv::VIDEO_TIMESCALE)
        } else {
            (TrackKind::Audio, self.audio_timescale)
        };
        let shift = units_of_ms(self.shift_ms(), timescale);
        unit.dts = unit.dts.saturating_sub(shift);
        unit.pts = unit.pts.saturating_sub(shift);
        let at = ms_of_units(unit.pts, timescale);
        unit.published = self
            .published
            .range((kind, i64::MIN)..=(kind, at))
            .next_back()
            .map_or(self.latest_published, |(_, t)| *t);
        Output::Unit(unit)
    }

    /// Writes queued input while ffmpeg takes it, and closes its input
    /// once it took the last.
    fn poll_input(&mut self, cx: &mut Context<'_>) -> Result<(), C::Error> {
        while !self.queue.is_empty() {
            let (front, _) = self.queue.as_slices();
            match self.child.poll_write(cx, front) {
                Poll::Ready(Ok(0)) | Poll::Pending => break,
                Poll::Ready(Ok(n)) => {
                    self.queue.drain(..n.min(front.len()));
                }
                Poll::Ready(Err(e)) => return Err(e),
            }
        }
        if self.finishing && self.queue.is_empty() && !self.input_closed {
            self.child.close_input();
            self.input_closed = true;
        }
        Ok(())
    }
}

impl Output {
    fn of(event: SourceEvent<'_>) -> Output {
        match event {
            SourceEvent::Config(c) => Output::Config(c),
            SourceEvent::Unit(u) => Output::Unit(Unit::of(&u)),
        }
    }
}

/// `ms` milliseconds in units of `timescale`, rounded down.
fn units_of_ms(ms: i64, timescale: u32) -> i64 {
    let units = (i128::from(ms) * i128::from(timescale)).div_euclid(1000);
    i64::try_from(units).unwrap_or(if ms < 0 { i64::MIN } else { i64::MAX })
}

/// `units` of `timescale` in milliseconds, rounded down; 0 for timescale
/// 0.
fn ms_of_units(units: i64, timescale: u32) -> i64 {
    if timescale == 0 {
        return 0;
    }
    let ms = (i128::from(units) * 1000).div_euclid(i128::from(timescale));
    i64::try_from(ms).unwrap_or(if units < 0 { i64::MIN } else { i64::MAX })
}

impl<C: ChildProcess> Transcode for FfmpegTranscode<C> {
    type Error = FfmpegError<C::Error>;

    fn send(&mut self, input: Input<'_>) -> Result<(), Refused> {
        if self.finishing {
            return Ok(());
        }
        if self.queue.len() >= self.max_queued {
            return Err(Refused::Full);
        }
        let mut bytes = Vec::new();
        match input {
            Input::Config(c) => {
                match self.configs.iter_mut().find(|o| o.track == c.track) {
                    Some(o) => *o = c.clone(),
                    None => self.configs.push(c.clone()),
                }
                if self.started {
                    self.writer
                        .config(c, &mut bytes)
                        .map_err(|_| Refused::Media("FLV carries no such config"))?;
                }
            }
            Input::Unit(u) => {
                let Some(config) = self.configs.iter().find(|c| c.track == u.track) else {
                    return Err(Refused::Media("a unit before its track's config"));
                };
                if !self.started {
                    self.writer.header(&self.configs, None, &mut bytes);
                    for c in &self.configs {
                        self.writer
                            .config(c, &mut bytes)
                            .map_err(|_| Refused::Media("FLV carries no such config"))?;
                    }
                    self.started = true;
                }
                self.writer
                    .unit(config, u, &mut bytes)
                    .map_err(|_| Refused::Media("FLV can't carry the unit's times"))?;
                let at = ms_of_units(u.pts, config.timescale);
                self.published.insert((config.kind, at), u.published);
                while self.published.len() > MAX_PUBLISHED {
                    self.published.pop_first();
                }
                self.latest_published = self.latest_published.max(u.published);
            }
        }
        self.queue.extend(bytes);
        Ok(())
    }

    fn queued_bytes(&self) -> usize {
        self.queue.len()
    }

    fn finish(&mut self) {
        self.finishing = true;
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<CodecEvent, Self::Error>>> {
        loop {
            if let Err(e) = self.poll_input(cx) {
                return Poll::Ready(Some(Err(FfmpegError::Pipe(e))));
            }
            if let Some(e) = self.events.pop_front() {
                return Poll::Ready(Some(Ok(e)));
            }
            if self.ended {
                return Poll::Ready(None);
            }
            match self.child.poll_read(cx, &mut self.read_buf) {
                Poll::Ready(Ok(0)) => self.ended = true,
                Poll::Ready(Ok(n)) => {
                    self.reader.push(self.read_buf.get(..n).unwrap_or_default());
                    if let Err(e) = self.read_tags() {
                        self.ended = true;
                        return Poll::Ready(Some(Err(FfmpegError::Output(e))));
                    }
                }
                Poll::Ready(Err(e)) => {
                    self.ended = true;
                    return Poll::Ready(Some(Err(FfmpegError::Pipe(e))));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
#[path = "ffmpeg_test.rs"]
mod tests;
