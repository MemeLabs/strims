// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A live stream as it reaches an ingest, and an ingest that signs it
//! early.
//!
//! - [`Encoder`] draws a live encoder's access units ([`Au`]): video at
//!   [`FPS`] frames a second in GOPs that open on a keyframe, and AAC frames
//!   of 1024 samples at 48 kHz. A stream of [`AUDIO_KBIT`] or less is
//!   audio alone. The encoder sits next to the ingest, so each access unit
//!   arrives whole at its capture time: a keyframe is one burst.
//! - [`Sign`] is how the ingest signs: the chunks per integrity unit, and
//!   when it signs a unit that hasn't filled, some time after the unit's
//!   first byte ([`Deadline`]).
//! - [`Signer`] lays the access units' records out in chunks as the unit
//!   format does (a header per chunk, a record per access unit, decoder
//!   configs ahead of each keyframe) and keeps the unit in progress, so a
//!   writer knows when to flush and where the padding goes. [`plan`] runs it
//!   over a whole trace.
//!
//! Times are offsets from the stream's start, in whole milliseconds, as a
//! writer signs them.

use core::fmt;
use core::ops::Range;
use std::collections::VecDeque;

use strims_base::rng::DetRng;
use strims_base::time::Duration;

use crate::sample::{exponential, unit};

/// Video frames a second.
pub const FPS: i64 = 30;
/// The audio track's rate, and the largest stream that is audio alone.
pub const AUDIO_KBIT: u32 = 128;
/// Samples per AAC frame, and the sample rate.
const AUDIO_SAMPLES: i64 = 1024;
const AUDIO_HZ: i64 = 48_000;
/// A CBR encoder's keyframe interval.
pub const CBR_GOP: Duration = Duration::from_secs(2);
/// A VBR encoder's scenes: their mean length, the shortest and the longest
/// (a keyframe opens each, and the longest is the encoder's keyframe
/// interval).
pub const SCENE_MEAN: Duration = Duration::from_secs(5);
pub const SCENE_MIN: Duration = Duration::from_millis(500);
pub const SCENE_MAX: Duration = Duration::from_secs(10);
/// A keyframe's size over a P-frame's, the median of its draw.
pub const KEY_RATIO: f64 = 6.0;
/// The log-normal spreads of the keyframe ratio, of each frame's size, and
/// of a VBR scene's complexity.
pub const KEY_SIGMA: f64 = 0.3;
pub const FRAME_SIGMA: f64 = 0.3;
pub const SCENE_SIGMA: f64 = 0.7;
/// The unit format's bytes per record beyond its access unit: its length,
/// track, timestamps and publish time.
pub const RECORD_OVERHEAD: u64 = 16;
/// The decoder configs a keyframe's record carries.
pub const CONFIG_BYTES: u64 = 128;
/// The unit format's header per chunk.
pub const CHUNK_HEADER: u64 = 2;
/// The span over which a tracking deadline measures the input rate.
pub const RATE_WINDOW: Duration = Duration::from_secs(2);
/// The input a tracking deadline needs before it measures a rate.
const RATE_MIN_SPAN: Duration = Duration::from_millis(100);
/// The largest unit a bitrate rule picks, in chunks.
pub const MAX_RULE_UNIT: usize = 16;
/// The largest unit [`Sign::parse`] takes, in chunks.
const MAX_UNIT: usize = 1 << 10;

/// How an encoder spends its bitrate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Constant: every GOP of [`CBR_GOP`] holds the nominal rate's bytes.
    Cbr,
    /// Variable: scenes of random length, each a GOP, at a random
    /// complexity whose mean is the nominal rate.
    Vbr,
}

/// A live encoder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Encoder {
    /// Audio and video together.
    pub kbit: u32,
    pub mode: Mode,
    /// The median keyframe over a P-frame.
    pub key_ratio: f64,
    /// The log-normal spread of a VBR scene's complexity.
    pub scene_sigma: f64,
}

/// An access unit as it reaches the ingest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Au {
    /// From the stream's start.
    pub at: Duration,
    pub len: u32,
    pub key: bool,
    pub video: bool,
}

impl Encoder {
    /// An encoder at the default keyframe ratio and scene spread.
    pub fn new(kbit: u32, mode: Mode) -> Encoder {
        Encoder {
            kbit,
            mode,
            key_ratio: KEY_RATIO,
            scene_sigma: SCENE_SIGMA,
        }
    }

    /// `<kbit><c|v>`, then optionally `:k<keyframe ratio>` and
    /// `:s<scene spread>`: `1500v`, `6000c:k10`, `3000v:s1.0`.
    pub fn parse(s: &str) -> Result<Encoder, String> {
        let bad = || format!("{s}: <kbit><c|v>[:k<ratio>][:s<spread>]");
        let mut parts = s.split(':');
        let head = parts.next().ok_or_else(bad)?;
        let (kbit, mode) = match head.char_indices().last() {
            Some((i, 'c')) => (head.get(..i), Mode::Cbr),
            Some((i, 'v')) => (head.get(..i), Mode::Vbr),
            _ => return Err(bad()),
        };
        let kbit: u32 = kbit.and_then(|k| k.parse().ok()).ok_or_else(bad)?;
        if kbit == 0 {
            return Err(bad());
        }
        let mut e = Encoder::new(kbit, mode);
        for p in parts {
            let v = |p: &str| p.get(1..).and_then(|x| x.parse::<f64>().ok());
            match p.chars().next() {
                Some('k') => e.key_ratio = v(p).filter(|&x| x >= 1.0).ok_or_else(bad)?,
                Some('s') => e.scene_sigma = v(p).filter(|&x| x >= 0.0).ok_or_else(bad)?,
                _ => return Err(bad()),
            }
        }
        Ok(e)
    }

    /// Bytes a second, audio and video together.
    pub fn rate(&self) -> u64 {
        u64::from(self.kbit) * 125
    }

    /// The access units captured in `[0, span)`, by arrival (audio first at
    /// equal times). The draws come from `rng`, and their number depends
    /// on the encoder and the span alone.
    pub fn trace(&self, rng: &mut DetRng, span: Duration) -> Vec<Au> {
        let audio_kbit = self.kbit.min(AUDIO_KBIT);
        let mut aus = audio(audio_kbit, span);
        if self.kbit > audio_kbit {
            aus.extend(self.video(rng, u64::from(self.kbit - audio_kbit) * 125, span));
        }
        aus.sort_by_key(|a| (a.at, a.video));
        aus
    }

    fn video(&self, rng: &mut DetRng, rate: u64, span: Duration) -> Vec<Au> {
        let mut out = Vec::new();
        let mut frame: i64 = 0;
        let at = |f: i64| Duration::from_millis(f * 1000 / FPS);
        while at(frame) < span {
            let (frames, complexity) = match self.mode {
                Mode::Cbr => (CBR_GOP.as_millis() * FPS / 1000, 1.0),
                Mode::Vbr => {
                    let len = exponential(rng, SCENE_MEAN).clamp(SCENE_MIN, SCENE_MAX);
                    let s = self.scene_sigma;
                    let c = (s * normal(rng) - s * s / 2.0).exp();
                    ((len.as_millis() * FPS / 1000).max(1), c)
                }
            };
            let key = (self.key_ratio.ln() + KEY_SIGMA * normal(rng)).exp();
            let noise = |rng: &mut DetRng| (FRAME_SIGMA * normal(rng)).exp();
            let weights: Vec<f64> = (0..frames)
                .map(|i| if i == 0 { key } else { 1.0 } * noise(rng))
                .collect();
            let total: f64 = weights.iter().sum();
            let bytes = rate as f64 * frames as f64 / FPS as f64 * complexity;
            for (i, w) in (0_i64..).zip(&weights) {
                let t = at(frame + i);
                if t >= span {
                    break;
                }
                out.push(Au {
                    at: t,
                    len: (bytes * w / total).round().max(1.0) as u32,
                    key: i == 0,
                    video: true,
                });
            }
            frame += frames;
        }
        out
    }
}

impl fmt::Display for Encoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let m = match self.mode {
            Mode::Cbr => 'c',
            Mode::Vbr => 'v',
        };
        write!(f, "{}{m}", self.kbit)?;
        if self.key_ratio != KEY_RATIO {
            write!(f, ":k{}", self.key_ratio)?;
        }
        if self.scene_sigma != SCENE_SIGMA {
            write!(f, ":s{}", self.scene_sigma)?;
        }
        Ok(())
    }
}

/// AAC frames at `kbit` over `[0, span)`, their sizes rounded so the
/// stream keeps its rate exactly.
fn audio(kbit: u32, span: Duration) -> Vec<Au> {
    let per_frame = f64::from(kbit) * 125.0 * AUDIO_SAMPLES as f64 / AUDIO_HZ as f64;
    let mut out = Vec::new();
    for j in 0_i64.. {
        let at = Duration::from_millis(j * AUDIO_SAMPLES * 1000 / AUDIO_HZ);
        if at >= span {
            break;
        }
        let len = ((j + 1) as f64 * per_frame).floor() - (j as f64 * per_frame).floor();
        out.push(Au {
            at,
            len: len.max(1.0) as u32,
            key: false,
            video: false,
        });
    }
    out
}

/// A standard normal draw (Box-Muller, two uniforms).
fn normal(rng: &mut DetRng) -> f64 {
    let u = 1.0 - unit(rng);
    let v = unit(rng);
    (-2.0 * u.ln()).sqrt() * (core::f64::consts::TAU * v).cos()
}

/// The chunks per integrity unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitRule {
    /// This many.
    Fixed(usize),
    /// The power of two nearest the chunks the stream's nominal bitrate
    /// fills in this long, from 1 to [`MAX_RULE_UNIT`].
    FromRate(Duration),
}

/// When a unit that hasn't filled is signed, after its first byte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Deadline {
    /// This long.
    Fixed(Duration),
    /// `factor` times the time the input rate of the last [`RATE_WINDOW`]
    /// takes to fill a unit, within `[lo, hi]`; `hi` until the input spans
    /// 100 ms.
    Track {
        lo: Duration,
        hi: Duration,
        factor: f64,
    },
}

/// How an ingest signs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sign {
    pub unit: UnitRule,
    pub deadline: Deadline,
}

impl Sign {
    /// Comma-separated: `u<chunks>` (a power of two) or `ur<ms>`
    /// ([`UnitRule::FromRate`]), then `t<ms>` or `t<lo>-<hi>x<factor>`
    /// ([`Deadline::Track`]). `off` is no early signing: `None`.
    pub fn parse(s: &str) -> Result<Option<Sign>, String> {
        if s == "off" {
            return Ok(None);
        }
        let bad = || format!("{s}: off, or u<chunks>|ur<ms>,t<ms>|t<lo>-<hi>x<factor>");
        let ms = |x: &str| {
            x.parse::<i64>()
                .ok()
                .filter(|&m| m > 0)
                .map(Duration::from_millis)
        };
        let (mut unit, mut deadline) = (None, None);
        for p in s.split(',') {
            if let Some(r) = p.strip_prefix("ur") {
                unit = Some(UnitRule::FromRate(ms(r).ok_or_else(bad)?));
            } else if let Some(n) = p.strip_prefix('u') {
                let n: usize = n.parse().map_err(|_| bad())?;
                if !n.is_power_of_two() || n > MAX_UNIT {
                    return Err(bad());
                }
                unit = Some(UnitRule::Fixed(n));
            } else if let Some(t) = p.strip_prefix('t') {
                deadline = Some(match t.split_once('-') {
                    None => Deadline::Fixed(ms(t).ok_or_else(bad)?),
                    Some((lo, rest)) => {
                        let (hi, factor) = rest.split_once('x').ok_or_else(bad)?;
                        let (lo, hi) = (ms(lo).ok_or_else(bad)?, ms(hi).ok_or_else(bad)?);
                        let factor: f64 = factor.parse().map_err(|_| bad())?;
                        if lo > hi || factor <= 0.0 {
                            return Err(bad());
                        }
                        Deadline::Track { lo, hi, factor }
                    }
                });
            } else {
                return Err(bad());
            }
        }
        match (unit, deadline) {
            (Some(unit), Some(deadline)) => Ok(Some(Sign { unit, deadline })),
            _ => Err(bad()),
        }
    }

    /// The chunks per unit for a stream of `rate` bytes a second in
    /// `chunk`-byte chunks.
    pub fn chunks(&self, rate: u64, chunk: usize) -> usize {
        match self.unit {
            UnitRule::Fixed(n) => n,
            UnitRule::FromRate(fill) => {
                let c = rate as f64 * fill.as_secs_f64() / chunk.max(1) as f64;
                let log = c.max(f64::MIN_POSITIVE).log2().round();
                let n = 2f64.powf(log.clamp(0.0, MAX_RULE_UNIT.ilog2() as f64));
                n as usize
            }
        }
    }
}

impl fmt::Display for Sign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.unit {
            UnitRule::Fixed(n) => write!(f, "u{n}")?,
            UnitRule::FromRate(d) => write!(f, "ur{}", d.as_millis())?,
        }
        match self.deadline {
            Deadline::Fixed(d) => write!(f, ",t{}", d.as_millis()),
            Deadline::Track { lo, hi, factor } => {
                write!(f, ",t{}-{}x{factor}", lo.as_millis(), hi.as_millis())
            }
        }
    }
}

/// An ingest's writer side: where each record lands in the stream, and
/// the unit in progress.
///
/// Records go one after another, a chunk header ahead of each chunk's
/// first byte. A unit is signed once its last byte is written; a unit in
/// progress is due at its deadline, and [`Signer::flush`] pads it to its
/// end, as `strims_ppspp::Writer::flush` does.
#[derive(Clone, Debug)]
pub struct Signer {
    deadline: Deadline,
    chunk: u64,
    unit: u64,
    header: u64,
    offset: u64,
    due: Option<Duration>,
    /// Input bytes by arrival over the last [`RATE_WINDOW`], their sum,
    /// and the first arrival.
    window: VecDeque<(Duration, u64)>,
    window_bytes: u64,
    since: Option<Duration>,
}

/// Where a record went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    /// Its stream offset and its bytes, the chunk headers it opened
    /// included.
    pub offset: u64,
    pub len: u64,
    /// The units it completed: they are signed as it is written.
    pub signed: u64,
}

impl Signer {
    /// A signer of `chunks`-chunk units of `chunk`-byte chunks, each
    /// opening with `header` bytes.
    pub fn new(deadline: Deadline, chunks: usize, chunk: usize, header: u64) -> Signer {
        let chunk = chunk.max(1) as u64;
        Signer {
            deadline,
            chunk,
            unit: chunk * chunks.max(1) as u64,
            header: header.min(chunk - 1),
            offset: 0,
            due: None,
            window: VecDeque::new(),
            window_bytes: 0,
            since: None,
        }
    }

    /// The stream offset of the next byte.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Bytes per unit.
    pub fn unit_bytes(&self) -> u64 {
        self.unit
    }

    /// When the unit in progress is due to be signed, if one is.
    pub fn due(&self) -> Option<Duration> {
        self.due
    }

    /// Places the record of `au` (the decoder configs ahead of a
    /// keyframe's): a header at each chunk it opens, its bytes, and the
    /// deadline of the unit it leaves in progress.
    pub fn record(&mut self, au: &Au) -> Placed {
        let mut left =
            u64::from(au.len) + RECORD_OVERHEAD + if au.key && au.video { CONFIG_BYTES } else { 0 };
        let start = self.offset;
        let unit_before = start / self.unit;
        let opened = start.is_multiple_of(self.unit);
        while left > 0 {
            if self.offset.is_multiple_of(self.chunk) {
                self.offset += self.header;
            }
            let room = self.chunk - self.offset % self.chunk;
            let take = room.min(left);
            self.offset += take;
            left -= take;
        }
        let len = self.offset - start;
        self.observe(au.at, len);
        let signed = self.offset / self.unit - unit_before;
        if self.offset.is_multiple_of(self.unit) {
            self.due = None;
        } else if signed > 0 || opened {
            self.due = Some(au.at + self.wait(au.at));
        }
        Placed {
            offset: start,
            len,
            signed,
        }
    }

    /// Signs the unit in progress, padded to its end: returns the padding,
    /// empty with no unit in progress.
    pub fn flush(&mut self) -> Range<u64> {
        let start = self.offset;
        self.offset = start.div_ceil(self.unit) * self.unit;
        self.due = None;
        start..self.offset
    }

    fn observe(&mut self, at: Duration, len: u64) {
        self.since.get_or_insert(at);
        self.window.push_back((at, len));
        self.window_bytes += len;
        while let Some(&(t, n)) = self.window.front() {
            if at - t < RATE_WINDOW {
                break;
            }
            self.window.pop_front();
            self.window_bytes -= n;
        }
    }

    /// How long a unit opened at `at` may wait.
    fn wait(&self, at: Duration) -> Duration {
        match self.deadline {
            Deadline::Fixed(d) => d,
            Deadline::Track { lo, hi, factor } => {
                let span = self
                    .since
                    .map_or(Duration::ZERO, |s| at - s)
                    .min(RATE_WINDOW);
                if span < RATE_MIN_SPAN || self.window_bytes == 0 {
                    return hi;
                }
                let rate = self.window_bytes as f64 / span.as_secs_f64();
                let fill = Duration::from_secs_f64(factor * self.unit as f64 / rate);
                fill.clamp(lo, hi).truncate(Duration::MILLISECOND)
            }
        }
    }
}

/// What a writer does, in time order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// Writes `len` stream bytes at `offset`: an access unit's record and
    /// the headers of the chunks it opens.
    Write { at: Duration, offset: u64, len: u64 },
    /// Signs the unit in progress; the padding runs to `next`, where the
    /// next unit starts.
    Flush { at: Duration, next: u64 },
}

/// A trace's whole stream: what the writer does, and per chunk when its
/// earliest media byte arrived and when its unit was signed.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub ops: Vec<Op>,
    /// Per chunk: the arrival of its earliest record byte, `None` for a
    /// chunk of padding alone.
    pub media_at: Vec<Option<Duration>>,
    /// Per chunk: when its unit was signed.
    pub signed_at: Vec<Duration>,
    /// The padding, by stream offset, in order.
    pub pads: Vec<Range<u64>>,
    pub chunk: u64,
    pub unit_chunks: u64,
    /// Access unit bytes, and stream bytes (records, headers and padding).
    pub media_bytes: u64,
    pub stream_bytes: u64,
}

impl Plan {
    /// Units signed.
    pub fn units(&self) -> u64 {
        self.stream_bytes / (self.chunk * self.unit_chunks).max(1)
    }

    /// Whether the byte at `offset` is padding.
    pub fn is_pad(&self, offset: u64) -> bool {
        let i = self.pads.partition_point(|p| p.end <= offset);
        self.pads.get(i).is_some_and(|p| p.contains(&offset))
    }

    /// Per chunk holding media, its sign time minus its earliest media
    /// byte's arrival, sorted.
    pub fn sign_waits(&self) -> Vec<Duration> {
        let mut w: Vec<Duration> = self
            .media_at
            .iter()
            .zip(&self.signed_at)
            .filter_map(|(m, &s)| m.map(|m| s - m))
            .collect();
        w.sort_unstable();
        w
    }
}

/// Runs `signer` over `trace`: each access unit is written at its arrival,
/// and each unit in progress flushed at its deadline, the last one too.
pub fn plan(trace: &[Au], mut signer: Signer) -> Plan {
    let chunk = signer.chunk;
    let unit_chunks = signer.unit / chunk;
    let mut p = Plan {
        chunk,
        unit_chunks,
        ..Plan::default()
    };
    let flush = |p: &mut Plan, signer: &mut Signer, at: Duration| {
        let pad = signer.flush();
        sign_through(p, pad.end, at);
        p.ops.push(Op::Flush { at, next: pad.end });
        if !pad.is_empty() {
            p.pads.push(pad);
        }
    };
    for au in trace {
        while let Some(due) = signer.due().filter(|&d| d <= au.at) {
            flush(&mut p, &mut signer, due);
        }
        let placed = signer.record(au);
        let end = placed.offset + placed.len;
        let (first, last) = (placed.offset / chunk, (end - 1) / chunk);
        let need = usize::try_from(last + 1).unwrap_or(usize::MAX);
        if p.media_at.len() < need {
            p.media_at.resize(need, None);
        }
        for c in p.media_at.iter_mut().take(need).skip(first as usize) {
            c.get_or_insert(au.at);
        }
        p.media_bytes += u64::from(au.len);
        if placed.signed > 0 {
            sign_through(&mut p, end / signer.unit * signer.unit, au.at);
        }
        p.ops.push(Op::Write {
            at: au.at,
            offset: placed.offset,
            len: placed.len,
        });
    }
    if let Some(due) = signer.due() {
        flush(&mut p, &mut signer, due);
    }
    p.stream_bytes = signer.offset();
    p.media_at
        .resize(usize::try_from(p.stream_bytes / chunk).unwrap_or(0), None);
    p
}

/// Marks the chunks before stream offset `end` (a unit boundary) not yet
/// signed as signed at `at`.
fn sign_through(p: &mut Plan, end: u64, at: Duration) {
    let n = usize::try_from(end / p.chunk).unwrap_or(0);
    if p.signed_at.len() < n {
        p.signed_at.resize(n, at);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

    use super::*;
    use strims_base::rng::det_rng;

    const CHUNK: usize = 4096;

    fn kbit_of(trace: &[Au], span: Duration) -> f64 {
        trace.iter().map(|a| f64::from(a.len)).sum::<f64>() * 8.0 / span.as_secs_f64() / 1000.0
    }

    #[test]
    fn cbr_holds_its_rate_and_gop() {
        let span = Duration::from_secs(60);
        let t = Encoder::new(3000, Mode::Cbr).trace(&mut det_rng(1), span);
        let got = kbit_of(&t, span);
        assert!((2970.0..3030.0).contains(&got), "{got}");
        let keys: Vec<Duration> = t.iter().filter(|a| a.key).map(|a| a.at).collect();
        assert_eq!(keys.len(), 30);
        assert!(keys.windows(2).all(|w| w[1] - w[0] == CBR_GOP));
        assert!(t.windows(2).all(|w| w[0].at <= w[1].at));
    }

    #[test]
    fn vbr_keeps_its_mean_and_cuts_scenes() {
        let span = Duration::from_secs(600);
        let mut kbit = Vec::new();
        for seed in 1..=8 {
            let t = Encoder::new(1500, Mode::Vbr).trace(&mut det_rng(seed), span);
            kbit.push(kbit_of(&t, span));
            let keys: Vec<Duration> = t.iter().filter(|a| a.key).map(|a| a.at).collect();
            assert!(
                keys.windows(2).all(|w| w[1] - w[0] <= SCENE_MAX),
                "{keys:?}"
            );
        }
        let mean = kbit.iter().sum::<f64>() / kbit.len() as f64;
        assert!((1300.0..1700.0).contains(&mean), "{kbit:?}");
        assert!(kbit.iter().any(|&k| (k - 1500.0).abs() > 30.0), "{kbit:?}");
    }

    #[test]
    fn low_rates_are_audio_alone() {
        let span = Duration::from_secs(10);
        let t = Encoder::new(128, Mode::Cbr).trace(&mut det_rng(1), span);
        assert!(t.iter().all(|a| !a.video));
        assert!((127.0..129.0).contains(&kbit_of(&t, span)));
        let t = Encoder::new(512, Mode::Cbr).trace(&mut det_rng(1), span);
        assert!(t.iter().any(|a| a.video) && t.iter().any(|a| !a.video));
    }

    #[test]
    fn a_seed_replays_its_trace() {
        let e = Encoder::new(6000, Mode::Vbr);
        let span = Duration::from_secs(30);
        assert_eq!(
            e.trace(&mut det_rng(5), span),
            e.trace(&mut det_rng(5), span)
        );
    }

    #[test]
    fn encoders_and_policies_parse_and_print() {
        for s in ["1500v", "6000c:k10", "3000v:s1"] {
            assert_eq!(Encoder::parse(s).unwrap().to_string(), s);
        }
        assert!(Encoder::parse("1500x").is_err() && Encoder::parse("c").is_err());
        for s in ["u8,t200", "u1,t250", "ur200,t300", "ur150,t100-500x1.5"] {
            assert_eq!(Sign::parse(s).unwrap().unwrap().to_string(), s);
        }
        assert_eq!(Sign::parse("off"), Ok(None));
        for s in ["u3,t200", "u8", "t200", "u8,t500-100x1", "u8,t0"] {
            assert!(Sign::parse(s).is_err(), "{s}");
        }
    }

    #[test]
    fn bitrate_rules_pick_the_nearest_power_of_two() {
        let sign = Sign::parse("ur200,t300").unwrap().unwrap();
        let chunks = |kbit: u64| sign.chunks(kbit * 125, CHUNK);
        // 200 ms of each fills 0.78, 3.1, 9.2, 18 and 37 chunks.
        assert_eq!([128, 512, 1500, 3000, 6000].map(chunks), [1, 4, 8, 16, 16]);
    }

    fn plan_of(kbit: u32, mode: Mode, sign: &str, secs: i64) -> Plan {
        let e = Encoder::new(kbit, mode);
        let s = Sign::parse(sign).unwrap().unwrap();
        let t = e.trace(&mut det_rng(1), Duration::from_secs(secs));
        let n = s.chunks(e.rate(), CHUNK);
        plan(&t, Signer::new(s.deadline, n, CHUNK, CHUNK_HEADER))
    }

    #[test]
    fn no_media_byte_waits_past_the_deadline() {
        for sign in ["u1,t100", "u8,t200", "u16,t500"] {
            for kbit in [128, 1500, 6000] {
                let p = plan_of(kbit, Mode::Vbr, sign, 30);
                let worst = p.sign_waits().last().copied().unwrap();
                let t = match Sign::parse(sign).unwrap().unwrap().deadline {
                    Deadline::Fixed(t) => t,
                    Deadline::Track { hi, .. } => hi,
                };
                assert!(worst <= t, "{sign} at {kbit}: {worst:?}");
            }
        }
    }

    #[test]
    fn units_that_fill_in_time_need_no_padding() {
        // 6 Mbit/s fills 8 chunks in 44 ms.
        let p = plan_of(6000, Mode::Cbr, "u8,t200", 30);
        let padded: u64 = p.pads.iter().map(|r| r.end - r.start).sum();
        assert!(
            padded < p.stream_bytes / 100,
            "{padded} of {}",
            p.stream_bytes
        );
        // 128 kbit/s fills 2 chunks in 0.5 s: flushed at 200 ms, padded.
        let p = plan_of(128, Mode::Cbr, "u2,t200", 30);
        let ratio = p.stream_bytes as f64 / p.media_bytes as f64;
        assert!((2.2..2.6).contains(&ratio), "{ratio}");
    }

    #[test]
    fn plans_follow_the_writers_flush() {
        let p = plan_of(512, Mode::Vbr, "u4,t100", 20);
        let unit = p.chunk * p.unit_chunks;
        let mut offset = 0;
        for op in &p.ops {
            match *op {
                Op::Write { offset: o, len, .. } => {
                    assert_eq!(o, offset);
                    offset += len;
                }
                Op::Flush { next, .. } => {
                    assert_eq!(next, offset.div_ceil(unit) * unit);
                    offset = next;
                }
            }
        }
        assert_eq!(offset, p.stream_bytes);
        assert!(p.stream_bytes.is_multiple_of(unit));
        assert_eq!(p.signed_at.len() as u64, p.stream_bytes / p.chunk);
        assert_eq!(p.media_at.len(), p.signed_at.len());
        for pad in &p.pads {
            assert!(p.is_pad(pad.start) && p.is_pad(pad.end - 1) && !p.is_pad(pad.end));
        }
        // Every unit starts with media.
        assert!(
            p.media_at
                .iter()
                .step_by(p.unit_chunks as usize)
                .all(Option::is_some)
        );
    }

    #[test]
    fn tracking_deadlines_follow_the_fill_time() {
        // 1.5 Mbit/s fills 8 chunks in about 175 ms: 1.5 times that.
        let s = Sign::parse("u8,t100-500x1.5").unwrap().unwrap();
        let e = Encoder::new(1500, Mode::Cbr);
        let t = e.trace(&mut det_rng(1), Duration::from_secs(10));
        let mut signer = Signer::new(s.deadline, 8, CHUNK, CHUNK_HEADER);
        let mut waits = Vec::new();
        for au in &t {
            let opens = signer.offset().is_multiple_of(signer.unit_bytes());
            let placed = signer.record(au);
            if let Some(d) = signer.due().filter(|_| opens || placed.signed > 0) {
                waits.push(d - au.at);
            }
        }
        let late: Vec<Duration> = waits.split_off(waits.len() / 2);
        assert!(
            late.iter().all(|w| (200..=330).contains(&w.as_millis())),
            "{late:?}"
        );
    }
}
