// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Silent audio frames, which fill the holes of an audio track.
//!
//! - AAC: a raw data block per frame with every channel element's
//!   spectrum empty (`max_sfb` 0), for the channel configurations 1 to 7
//!   of a core coder without explicitly signalled SBR or PS. Its decoded
//!   frame is the overlap of the frame before, then zeros.
//! - Opus: CELT-only fullband packets of 20, 10, 5 and 2.5 ms whose frame
//!   is the silence flag alone, one per stream of the OpusHead's mapping;
//!   every stream but the last in self-delimiting framing (RFC 6716
//!   appendix B).

use crate::aac::{AOT_PS, AOT_SBR, parse_asc};
use crate::media::{CodecParams, DecoderConfig};
use crate::opus::parse_opus_head;

/// A CELT frame with the silence flag set (RFC 6716 4.3, Table 56).
const CELT_SILENCE: [u8; 2] = [0xff, 0xfe];
/// The CELT-only fullband configs of 20, 10, 5 and 2.5 ms (RFC 6716 3.1)
/// and their 48 kHz samples.
const CELT_FRAMES: [(u8, u32); 4] = [(31, 960), (30, 480), (29, 240), (28, 120)];

/// The silent frames of a track's config, each with the samples it lasts
/// in the track's timescale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Silence {
    /// Longest first; each lasts a whole number of the last's samples.
    frames: Vec<(Vec<u8>, u32)>,
}

impl Silence {
    /// The silent frames of `config`; `None` for video and for an audio
    /// config they aren't known for (HE-AAC, AAC channels a program config
    /// element defines, an OpusHead that doesn't parse).
    pub fn for_config(config: &DecoderConfig) -> Option<Silence> {
        match &config.params {
            CodecParams::Aac { asc, .. } => aac(asc),
            CodecParams::Opus { head, .. } => opus(head),
            _ => None,
        }
    }

    /// The samples of the shortest frame: covers are multiples of it.
    pub fn step(&self) -> u32 {
        self.frames.last().map_or(0, |f| f.1)
    }

    /// The frames that last `samples`, rounded to the nearest step, the
    /// shortest first: payloads and the samples each lasts. The last is a
    /// longest one when the cover holds one, so a writer that times an
    /// untimed last sample by the one before gives it its own length.
    pub fn cover(&self, samples: i64) -> impl Iterator<Item = (&[u8], u32)> {
        let step = i64::from(self.step().max(1));
        let mut steps = samples.saturating_add(step / 2).max(0) / step;
        let counts: Vec<usize> = self
            .frames
            .iter()
            .map(|(_, n)| {
                let per = i64::from(*n) / step;
                let count = if per > 0 { steps / per } else { 0 };
                steps -= count * per;
                usize::try_from(count).unwrap_or(0)
            })
            .collect();
        self.frames
            .iter()
            .zip(counts)
            .rev()
            .flat_map(|((payload, n), count)| std::iter::repeat_n((payload.as_slice(), *n), count))
    }
}

/// Channel elements, as ISO/IEC 14496-3 Table 1.19 lays out each channel
/// configuration: S a single channel, C a channel pair, L an LFE channel.
const AAC_LAYOUTS: [&[u8]; 7] = [b"S", b"C", b"SC", b"SCS", b"SCC", b"SCCL", b"SCCCL"];

fn aac(asc: &[u8]) -> Option<Silence> {
    let asc = parse_asc(asc).ok()?;
    if asc.object_type == AOT_SBR
        || asc.object_type == AOT_PS
        || asc.sample_rate != asc.core_sample_rate
    {
        return None;
    }
    let layout = AAC_LAYOUTS.get(usize::from(asc.channel_config).checked_sub(1)?)?;
    let mut w = BitWriter::default();
    let mut tags = [0u32; 3];
    for &element in *layout {
        let (id, kind) = match element {
            b'S' => (0, 0),
            b'C' => (1, 1),
            _ => (3, 2),
        };
        let tag = tags.get_mut(kind)?;
        w.put(id, 3);
        w.put(*tag, 4);
        *tag += 1;
        if id == 1 {
            // common_window 0: each channel its own stream.
            w.put(0, 1);
            empty_channel_stream(&mut w);
        }
        empty_channel_stream(&mut w);
    }
    // ID_END.
    w.put(7, 3);
    Some(Silence {
        frames: vec![(w.finish(), asc.frame_samples)],
    })
}

/// An individual_channel_stream of a long window with no scale factor
/// bands: global_gain, ics_info (reserved bit, ONLY_LONG_SEQUENCE, window
/// shape, max_sfb 0, no predictor), and no pulse, TNS or gain control data.
fn empty_channel_stream(w: &mut BitWriter) {
    w.put(100, 8);
    w.put(0, 1 + 2 + 1 + 6 + 1);
    w.put(0, 3);
}

fn opus(head: &[u8]) -> Option<Silence> {
    let head = parse_opus_head(head).ok()?;
    let streams = head.streams.max(1);
    let frames = CELT_FRAMES
        .iter()
        .map(|&(config, samples)| {
            let mut packet = Vec::new();
            for s in 0..streams {
                packet.push((config << 3) | (u8::from(s < head.coupled) << 2));
                if s + 1 < streams {
                    packet.push(CELT_SILENCE.len() as u8);
                }
                packet.extend_from_slice(&CELT_SILENCE);
            }
            (packet, samples)
        })
        .collect();
    Some(Silence { frames })
}

/// Writes bits MSB first, the last byte padded with zeros.
#[derive(Debug, Default)]
struct BitWriter {
    bytes: Vec<u8>,
    bits: u32,
}

impl BitWriter {
    fn put(&mut self, value: u32, n: u32) {
        for i in (0..n).rev() {
            if self.bits.is_multiple_of(8) {
                self.bytes.push(0);
            }
            let bit = u8::from(value >> i & 1 == 1);
            if let Some(b) = self.bytes.last_mut() {
                *b |= bit << (7 - self.bits % 8);
            }
            self.bits += 1;
        }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
#[path = "silence_test.rs"]
mod tests;
