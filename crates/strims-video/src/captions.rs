// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Closed captions as ATSC A/53 carries them in H.264 and H.265 video:
//! the `cc_data` of SEI messages of user data registered by ITU-T T.35,
//! read from a unit's SEI, written into one, and carried from a stream's
//! frames to the frames of its renditions.
//!
//! A caption tuple ([`CcTuple`]) is one `cc_data` entry: its marker,
//! `cc_valid` and `cc_type` byte, then its two data bytes. Types 0 and 1
//! are CEA-608 byte pairs of fields 1 and 2; types 2 and 3 are CEA-708
//! (DTVCC) packet data. Readers keep the valid tuples that carry data:
//! not padding, and not 608's null pairs.
//!
//! A unit's tuples come in its SEI's order; a stream's tuples play in its
//! frames' presentation order, which B-frames make other than their
//! decode order.
//!
//! Every function takes hostile input: none panics, and each works in
//! time linear in the unit.

use std::collections::BTreeMap;

use crate::avc::parse_avcc;
use crate::hevc::parse_hvcc;
use crate::media::CodecParams;

/// A `cc_data` entry: `0xF8 | cc_valid << 2 | cc_type`, `cc_data_1`,
/// `cc_data_2`.
pub type CcTuple = [u8; 3];

/// The most tuples read of one unit. A/53 sends at most 31 a frame; 60
/// fields of 608 and 9600 bit/s of 708 take 10 at 60 frames a second.
pub const MAX_UNIT_TUPLES: usize = 64;
/// The most tuples one SEI message holds: `cc_count` has 5 bits.
const MAX_MESSAGE_TUPLES: usize = 31;

/// The H.264 NAL unit type of SEI.
const AVC_SEI: u8 = 6;
/// The H.265 NAL unit type of a prefix SEI.
const HEVC_PREFIX_SEI: u8 = 39;
/// The SEI payload type of user data registered by ITU-T T.35.
const USER_DATA_REGISTERED: u32 = 4;
/// A/53's T.35 header: the United States' country code, the ATSC
/// provider code, the `GA94` user identifier and the `cc_data` type code.
const A53_HEADER: [u8; 8] = [0xb5, 0x00, 0x31, b'G', b'A', b'9', b'4', 0x03];
/// `cc_data`'s flags byte: `process_cc_data_flag` and the reserved bit
/// set, before `cc_count`.
const PROCESS_CC_DATA: u8 = 0x40;
const CC_FLAGS_RESERVED: u8 = 0x80;
/// A tuple's marker bits and its `cc_valid` bit.
const TUPLE_MARKER: u8 = 0xf8;
const CC_VALID: u8 = 0x04;

/// Why captions couldn't be written into a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CaptionError {
    /// The unit isn't NAL units of its length size, or has no slice to
    /// put the captions before.
    #[error("the unit's NAL units are malformed")]
    Malformed,
    /// A NAL unit is too long for the stream's NAL unit length size.
    #[error("the captions' SEI is too long for the NAL unit length size")]
    TooLong,
}

/// How a stream's units hold their NAL units: their codec and length
/// size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NalFormat {
    hevc: bool,
    length_size: u8,
}

impl NalFormat {
    /// The format of units of `params`: `None` for a codec without SEI,
    /// or a config that doesn't parse.
    pub fn of(params: &CodecParams) -> Option<NalFormat> {
        match params {
            CodecParams::Avc { avcc, .. } => parse_avcc(avcc).ok().map(|c| NalFormat {
                hevc: false,
                length_size: c.nal_length_size,
            }),
            CodecParams::Hevc { hvcc, .. } => parse_hvcc(hvcc).ok().map(|c| NalFormat {
                hevc: true,
                length_size: c.nal_length_size,
            }),
            _ => None,
        }
    }

    /// An H.264 format of `length_size`-byte lengths.
    pub fn avc(length_size: u8) -> NalFormat {
        NalFormat {
            hevc: false,
            length_size,
        }
    }

    /// Appends the caption tuples of `unit`'s SEI to `out`, in their
    /// order, at most [`MAX_UNIT_TUPLES`]: the valid ones that carry
    /// data. A malformed NAL unit ends the reading there.
    pub fn cc_tuples(&self, unit: &[u8], out: &mut Vec<CcTuple>) {
        let mut taken = 0;
        let mut rbsp = Vec::new();
        for nal in Nals::new(unit, self.length_size) {
            let Ok((_, nal)) = nal else {
                return;
            };
            let header = if self.hevc { 2 } else { 1 };
            let Some(&first) = nal.first() else {
                continue;
            };
            let sei = if self.hevc {
                (first >> 1) & 0x3f == HEVC_PREFIX_SEI
            } else {
                first & 0x1f == AVC_SEI
            };
            if !sei {
                continue;
            }
            unescape(nal.get(header..).unwrap_or_default(), &mut rbsp);
            for (kind, payload) in SeiMessages(&rbsp) {
                if kind != USER_DATA_REGISTERED {
                    continue;
                }
                let Some(cc) = payload.strip_prefix(&A53_HEADER[..]) else {
                    continue;
                };
                let (Some(&flags), Some(entries)) = (cc.first(), cc.get(2..)) else {
                    continue;
                };
                if flags & PROCESS_CC_DATA == 0 {
                    continue;
                }
                let count = usize::from(flags & 0x1f);
                for &[b, d1, d2] in entries.as_chunks::<3>().0.iter().take(count) {
                    let kind = b & 0x03;
                    let null = kind < 2 && d1 & 0x7f == 0 && d2 & 0x7f == 0;
                    if b & CC_VALID == 0 || null {
                        continue;
                    }
                    if taken == MAX_UNIT_TUPLES {
                        return;
                    }
                    out.push([TUPLE_MARKER | CC_VALID | kind, d1, d2]);
                    taken += 1;
                }
            }
        }
    }

    /// Whether `unit`'s SEI holds a caption tuple.
    pub fn has_captions(&self, unit: &[u8]) -> bool {
        let mut t = Vec::new();
        self.cc_tuples(unit, &mut t);
        !t.is_empty()
    }

    /// Writes into `out` the H.264 `unit` with an SEI NAL unit of
    /// `tuples` before its first slice, in A/53 messages of at most 31
    /// tuples each; `unit` as it is without tuples.
    ///
    /// # Errors
    ///
    /// An H.265 format, a malformed unit or one without a slice
    /// ([`CaptionError::Malformed`]), and an SEI too long for the length
    /// size ([`CaptionError::TooLong`]); `out` is then unchanged.
    pub fn with_cc_sei(
        &self,
        unit: &[u8],
        tuples: &[CcTuple],
        out: &mut Vec<u8>,
    ) -> Result<(), CaptionError> {
        if self.hevc {
            return Err(CaptionError::Malformed);
        }
        if tuples.is_empty() {
            out.extend_from_slice(unit);
            return Ok(());
        }
        let mut slice = None;
        for nal in Nals::new(unit, self.length_size) {
            let (at, nal) = nal.map_err(|_| CaptionError::Malformed)?;
            if slice.is_none() && nal.first().is_some_and(|b| (1..=5).contains(&(b & 0x1f))) {
                slice = Some(at);
            }
        }
        let slice = slice.ok_or(CaptionError::Malformed)?;
        let sei = sei_nal(tuples);
        let len = u32::try_from(sei.len()).map_err(|_| CaptionError::TooLong)?;
        let size = usize::from(self.length_size);
        if size < 4 && len >> (8 * size) != 0 {
            return Err(CaptionError::TooLong);
        }
        out.reserve(unit.len() + size + sei.len());
        out.extend_from_slice(unit.get(..slice).unwrap_or_default());
        out.extend_from_slice(len.to_be_bytes().get(4 - size..).unwrap_or_default());
        out.extend_from_slice(&sei);
        out.extend_from_slice(unit.get(slice..).unwrap_or_default());
        Ok(())
    }
}

/// An H.264 SEI NAL unit of A/53 messages of `tuples`, emulation
/// prevented.
fn sei_nal(tuples: &[CcTuple]) -> Vec<u8> {
    let mut rbsp = Vec::new();
    for chunk in tuples.chunks(MAX_MESSAGE_TUPLES) {
        // The T.35 header, the flags and em_data bytes, the tuples and the
        // marker byte: at most 106 bytes, a one-byte size.
        let size = A53_HEADER.len() + 2 + 3 * chunk.len() + 1;
        rbsp.push(USER_DATA_REGISTERED as u8);
        rbsp.push(size as u8);
        rbsp.extend_from_slice(&A53_HEADER);
        rbsp.push(CC_FLAGS_RESERVED | PROCESS_CC_DATA | chunk.len() as u8);
        rbsp.push(0xff);
        for t in chunk {
            rbsp.extend_from_slice(t);
        }
        rbsp.push(0xff);
    }
    // rbsp_trailing_bits.
    rbsp.push(0x80);
    let mut nal = vec![AVC_SEI];
    let mut zeros = 0;
    for &b in &rbsp {
        if zeros >= 2 && b <= 0x03 {
            nal.push(0x03);
            zeros = 0;
        }
        nal.push(b);
        zeros = if b == 0 { zeros + 1 } else { 0 };
    }
    nal
}

/// `escaped` without its emulation prevention bytes, into `out`.
fn unescape(escaped: &[u8], out: &mut Vec<u8>) {
    out.clear();
    let mut zeros = 0;
    for &b in escaped {
        if zeros >= 2 && b == 0x03 {
            zeros = 0;
            continue;
        }
        out.push(b);
        zeros = if b == 0 { zeros + 1 } else { 0 };
    }
}

/// The length-prefixed NAL units of a unit, each with its offset.
struct Nals<'a> {
    unit: &'a [u8],
    at: usize,
    length_size: usize,
}

impl<'a> Nals<'a> {
    fn new(unit: &'a [u8], length_size: u8) -> Self {
        Nals {
            unit,
            at: 0,
            length_size: usize::from(length_size),
        }
    }
}

impl<'a> Iterator for Nals<'a> {
    /// A NAL unit's offset and bytes, or a length past the unit's end.
    type Item = Result<(usize, &'a [u8]), ()>;

    fn next(&mut self) -> Option<Self::Item> {
        let rest = self.unit.get(self.at..)?;
        if rest.is_empty() {
            return None;
        }
        let at = self.at;
        let Some(head) = rest
            .get(..self.length_size)
            .filter(|_| self.length_size > 0)
        else {
            self.at = self.unit.len();
            return Some(Err(()));
        };
        let len = head.iter().fold(0usize, |n, &b| (n << 8) | usize::from(b));
        let start = self.length_size;
        let Some(nal) = start.checked_add(len).and_then(|end| rest.get(start..end)) else {
            self.at = self.unit.len();
            return Some(Err(()));
        };
        self.at += start + len;
        Some(Ok((at, nal)))
    }
}

/// The SEI messages of an SEI RBSP: each payload's type and bytes.
struct SeiMessages<'a>(&'a [u8]);

impl<'a> SeiMessages<'a> {
    /// A value coded as 0xFF bytes and a last byte below 0xFF.
    fn value(&mut self) -> Option<u32> {
        let mut v: u32 = 0;
        loop {
            let (&b, rest) = self.0.split_first()?;
            self.0 = rest;
            v = v.checked_add(u32::from(b))?;
            if b != 0xff {
                return Some(v);
            }
        }
    }
}

impl<'a> Iterator for SeiMessages<'a> {
    type Item = (u32, &'a [u8]);

    fn next(&mut self) -> Option<(u32, &'a [u8])> {
        // The trailing bits' byte, 0x80, ends the messages.
        if self.0.len() < 2 {
            return None;
        }
        let kind = self.value()?;
        let size = usize::try_from(self.value()?).ok()?;
        let payload = self.0.get(..size)?;
        self.0 = self.0.get(size..).unwrap_or_default();
        Some((kind, payload))
    }
}

/// The most source frames' tuples a [`CaptionCarry`] holds: 10 s at 60
/// frames a second, past any encoder's delay.
pub const MAX_CARRIED_FRAMES: usize = 600;

/// Carries a stream's caption tuples to its renditions' frames, which keep
/// a subset of the stream's frames, at the times of the stream's.
///
/// Each frame of a rendition takes the tuples of the stream's frames
/// since its last, up to its presentation time, in presentation order.
///
/// # Bounds
///
/// At most [`MAX_CARRIED_FRAMES`] frames' tuples, each at most
/// [`MAX_UNIT_TUPLES`]: the oldest go first, and a frame's go once every
/// rendition took them.
#[derive(Debug, Default)]
pub struct CaptionCarry {
    /// The stream's frames' tuples, by presentation time in nanoseconds.
    frames: BTreeMap<i64, Vec<CcTuple>>,
    /// Each rendition's last frame's presentation time, in nanoseconds,
    /// once it had one since the last restart.
    taken: Vec<Option<i64>>,
}

impl CaptionCarry {
    /// A carry to `renditions` renditions.
    pub fn new(renditions: usize) -> CaptionCarry {
        CaptionCarry {
            frames: BTreeMap::new(),
            taken: vec![None; renditions],
        }
    }

    /// Forgets every frame and each rendition's last, as when the
    /// renditions start again.
    pub fn restart(&mut self) {
        self.frames.clear();
        self.taken.iter_mut().for_each(|t| *t = None);
    }

    /// Takes a stream frame's tuples, at its presentation time `pts` of
    /// `timescale`.
    pub fn source(&mut self, pts: i64, timescale: u32, tuples: &[CcTuple]) {
        if tuples.is_empty() {
            return;
        }
        let at = nanos(pts, timescale);
        let held = self.frames.entry(at).or_default();
        let room = MAX_UNIT_TUPLES.saturating_sub(held.len());
        held.extend(tuples.iter().take(room));
        while self.frames.len() > MAX_CARRIED_FRAMES {
            self.frames.pop_first();
        }
    }

    /// The tuples of `rendition`'s frame at `pts` of `timescale`: those
    /// of the stream's frames after its last frame and up to this one, in
    /// presentation order.
    pub fn take(&mut self, rendition: usize, pts: i64, timescale: u32) -> Vec<CcTuple> {
        let at = nanos(pts, timescale);
        let Some(last) = self.taken.get_mut(rendition) else {
            return Vec::new();
        };
        let from = *last;
        *last = Some(at);
        let mut tuples = Vec::new();
        let range = match from {
            Some(f) if f >= at => return tuples,
            Some(f) => self.frames.range(f + 1..=at),
            None => self.frames.range(..=at),
        };
        for t in range.map(|(_, t)| t) {
            tuples.extend_from_slice(t);
        }
        // Frames every rendition took are done with.
        if let Some(Some(done)) = self.taken.iter().min() {
            let done = *done;
            while self
                .frames
                .first_key_value()
                .is_some_and(|(&k, _)| k <= done)
            {
                self.frames.pop_first();
            }
        }
        tuples
    }
}

/// `units` of `timescale` in nanoseconds, saturating; 0 for timescale 0.
fn nanos(units: i64, timescale: u32) -> i64 {
    if timescale == 0 {
        return 0;
    }
    let n = i128::from(units) * 1_000_000_000 / i128::from(timescale);
    i64::try_from(n).unwrap_or(if n < 0 { i64::MIN } else { i64::MAX })
}

#[cfg(test)]
#[path = "captions_test.rs"]
mod tests;
