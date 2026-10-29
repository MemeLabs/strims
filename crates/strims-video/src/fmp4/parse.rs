// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Reading records: the init and media segments of a record, the tracks
//! an init segment declares and the media time a media segment covers.
//!
//! The input is untrusted: no input panics, the work is linear in its
//! length, the results borrow from it, and an init segment declares at
//! most [`MAX_TRACKS`] tracks. Boxes may use 64-bit sizes or extend to
//! the end of their parent.

use std::fmt;

use strims_base::time::Duration;

use crate::fmp4::MAX_TRACKS;
use crate::media::TrackKind;

/// A box type.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FourCc(pub [u8; 4]);

impl fmt::Debug for FourCc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.escape_ascii())
    }
}

impl fmt::Display for FourCc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.escape_ascii())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The record is shorter than its init length prefix says.
    #[error("record is shorter than its init segment length")]
    Prefix,
    /// A box's size is below its header or past the end of its parent.
    #[error("{0} box is truncated")]
    Truncated(FourCc),
    /// A box header is cut off.
    #[error("box header is truncated")]
    Header,
    #[error("{0} box is missing")]
    Missing(FourCc),
    #[error("init segment declares more than {MAX_TRACKS} tracks")]
    TooManyTracks,
}

/// Splits a record, `[u16 BE init length][init segment][media segment]`,
/// into its init and media segments.
pub fn split_record(record: &[u8]) -> Result<(&[u8], &[u8]), ParseError> {
    let (len, rest) = record.split_first_chunk::<2>().ok_or(ParseError::Prefix)?;
    let len = usize::from(u16::from_be_bytes(*len));
    if rest.len() < len {
        return Err(ParseError::Prefix);
    }
    Ok(rest.split_at(len))
}

/// A track an init segment declares.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrackInfo {
    pub id: u32,
    /// The `hdlr` handler type, such as `vide` or `soun`.
    pub handler: [u8; 4],
    /// The `mdhd` timescale.
    pub timescale: u32,
    /// The `trex` default sample duration, 0 without one.
    pub default_duration: u32,
}

impl TrackInfo {
    /// The kind its handler names, if video or audio.
    pub fn kind(&self) -> Option<TrackKind> {
        match &self.handler {
            b"vide" => Some(TrackKind::Video),
            b"soun" => Some(TrackKind::Audio),
            _ => None,
        }
    }
}

/// The tracks of an init segment, in `trak` order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InitInfo {
    tracks: [TrackInfo; MAX_TRACKS],
    len: usize,
}

impl InitInfo {
    pub fn tracks(&self) -> &[TrackInfo] {
        self.tracks.get(..self.len).unwrap_or_default()
    }

    pub fn track(&self, id: u32) -> Option<&TrackInfo> {
        self.tracks().iter().find(|t| t.id == id)
    }

    fn position(&self, id: u32) -> Option<usize> {
        self.tracks().iter().position(|t| t.id == id)
    }
}

/// Reads the tracks an init segment's `moov` declares: each `trak`'s
/// `tkhd` track id, `hdlr` handler type and `mdhd` timescale, and the
/// default duration of the `mvex` `trex` with its id.
pub fn parse_init(init: &[u8]) -> Result<InitInfo, ParseError> {
    let moov = find(init, b"moov")?.ok_or(ParseError::Missing(FourCc(*b"moov")))?;
    let mut info = InitInfo::default();
    let mut mvex = None;
    for b in Boxes::new(moov) {
        let (fourcc, content) = b?;
        match &fourcc.0 {
            b"trak" => {
                let track = parse_trak(content)?;
                let slot = info
                    .tracks
                    .get_mut(info.len)
                    .ok_or(ParseError::TooManyTracks)?;
                *slot = track;
                info.len += 1;
            }
            b"mvex" if mvex.is_none() => mvex = Some(content),
            _ => {}
        }
    }
    for b in Boxes::new(mvex.unwrap_or_default()) {
        let (fourcc, content) = b?;
        if fourcc.0 != *b"trex" {
            continue;
        }
        let (id, duration) = (be32(content, 4), be32(content, 12));
        let (Some(id), Some(duration)) = (id, duration) else {
            return Err(ParseError::Truncated(fourcc));
        };
        if let Some(i) = info.position(id)
            && let Some(t) = info.tracks.get_mut(i)
        {
            t.default_duration = duration;
        }
    }
    Ok(info)
}

fn parse_trak(trak: &[u8]) -> Result<TrackInfo, ParseError> {
    let tkhd = find(trak, b"tkhd")?.ok_or(ParseError::Missing(FourCc(*b"tkhd")))?;
    let mdia = find(trak, b"mdia")?.ok_or(ParseError::Missing(FourCc(*b"mdia")))?;
    let mdhd = find(mdia, b"mdhd")?.ok_or(ParseError::Missing(FourCc(*b"mdhd")))?;
    let hdlr = find(mdia, b"hdlr")?.ok_or(ParseError::Missing(FourCc(*b"hdlr")))?;
    // Version 1 widens the creation and modification times to 64 bits.
    let times = |b: &[u8]| if b.first() == Some(&1) { 16 } else { 8 };
    let id = be32(tkhd, 4 + times(tkhd)).ok_or(ParseError::Truncated(FourCc(*b"tkhd")))?;
    let timescale = be32(mdhd, 4 + times(mdhd)).ok_or(ParseError::Truncated(FourCc(*b"mdhd")))?;
    let handler = hdlr
        .get(8..12)
        .and_then(|h| <[u8; 4]>::try_from(h).ok())
        .ok_or(ParseError::Truncated(FourCc(*b"hdlr")))?;
    Ok(TrackInfo {
        id,
        handler,
        timescale,
        default_duration: 0,
    })
}

/// The media time `media` covers: the sum of the first video track's
/// sample durations over every `moof`, or with no video track, the
/// longest track's. Durations come from the `trun` entries, else the
/// `tfhd` default, else the `trex` default. Boxes other than `moof`
/// (`styp`, `sidx`, `prft`, `mdat`, `free`, `skip` and others) are skipped, as
/// are `traf`s of tracks `info` doesn't declare.
pub fn media_duration(info: &InitInfo, media: &[u8]) -> Result<Duration, ParseError> {
    let mut sums = [0u64; MAX_TRACKS];
    for b in Boxes::new(media) {
        let (fourcc, moof) = b?;
        if fourcc.0 != *b"moof" {
            continue;
        }
        for b in Boxes::new(moof) {
            let (fourcc, traf) = b?;
            if fourcc.0 != *b"traf" {
                continue;
            }
            let Some((id, units)) = traf_duration(info, traf)? else {
                continue;
            };
            if let Some(i) = info.position(id)
                && let Some(sum) = sums.get_mut(i)
            {
                *sum = sum.saturating_add(units);
            }
        }
    }
    let duration = |(t, units): (&TrackInfo, &u64)| units_to_duration(*units, t.timescale);
    let tracks = info.tracks().iter().zip(sums.iter());
    let video = tracks
        .clone()
        .find(|(t, _)| t.kind() == Some(TrackKind::Video));
    Ok(match video {
        Some(v) => duration(v),
        None => tracks.map(duration).max().unwrap_or(Duration::ZERO),
    })
}

/// A `traf`'s track id and the summed durations of its `trun`s; `None`
/// without a `tfhd`.
fn traf_duration(info: &InitInfo, traf: &[u8]) -> Result<Option<(u32, u64)>, ParseError> {
    let Some(tfhd) = find(traf, b"tfhd")? else {
        return Ok(None);
    };
    let truncated = ParseError::Truncated(FourCc(*b"tfhd"));
    let flags = be32(tfhd, 0).ok_or(truncated)? & 0x00ff_ffff;
    let id = be32(tfhd, 4).ok_or(truncated)?;
    let mut default = info.track(id).map_or(0, |t| t.default_duration);
    if flags & 0x08 != 0 {
        // base-data-offset (8 bytes) and sample-description-index (4)
        // come first when present.
        let at = 8 + if flags & 0x01 != 0 { 8 } else { 0 } + if flags & 0x02 != 0 { 4 } else { 0 };
        default = be32(tfhd, at).ok_or(truncated)?;
    }
    let mut units = 0u64;
    for b in Boxes::new(traf) {
        let (fourcc, trun) = b?;
        if fourcc.0 == *b"trun" {
            units = units.saturating_add(trun_duration(trun, default)?);
        }
    }
    Ok(Some((id, units)))
}

fn trun_duration(trun: &[u8], default: u32) -> Result<u64, ParseError> {
    let truncated = ParseError::Truncated(FourCc(*b"trun"));
    let flags = be32(trun, 0).ok_or(truncated)? & 0x00ff_ffff;
    let count = be32(trun, 4).ok_or(truncated)?;
    if flags & 0x100 == 0 {
        return Ok(u64::from(count) * u64::from(default));
    }
    // Optional data offset and first-sample flags, then the entries, each
    // starting with its duration.
    let at = 8 + if flags & 0x01 != 0 { 4 } else { 0 } + if flags & 0x04 != 0 { 4 } else { 0 };
    let entry = 4 * (flags & 0xf00).count_ones() as usize;
    let entries = trun.get(at..).ok_or(truncated)?;
    let count = usize::try_from(count).map_err(|_| truncated)?;
    if count > entries.len() / entry {
        return Err(truncated);
    }
    Ok(entries
        .chunks_exact(entry)
        .take(count)
        .filter_map(|e| be32(e, 0))
        .map(u64::from)
        .sum())
}

/// `units` of a `timescale` as a duration, saturating; zero for timescale 0.
fn units_to_duration(units: u64, timescale: u32) -> Duration {
    if timescale == 0 {
        return Duration::ZERO;
    }
    let nanos = i128::from(units) * 1_000_000_000 / i128::from(timescale);
    Duration::from_nanos(i64::try_from(nanos).unwrap_or(i64::MAX))
}

/// The content of the first box of type `fourcc` among `data`'s boxes.
fn find<'a>(data: &'a [u8], fourcc: &[u8; 4]) -> Result<Option<&'a [u8]>, ParseError> {
    for b in Boxes::new(data) {
        let (t, content) = b?;
        if t.0 == *fourcc {
            return Ok(Some(content));
        }
    }
    Ok(None)
}

fn be32(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_be_bytes(bytes.try_into().ok()?))
}

/// The boxes of `data` back to back: each box's type and content (after
/// its header). Stops after the first error.
#[derive(Debug, Clone)]
pub(crate) struct Boxes<'a> {
    data: &'a [u8],
}

impl<'a> Boxes<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Boxes { data }
    }

    fn next_box(&mut self) -> Result<(FourCc, &'a [u8]), ParseError> {
        let (size, rest) = self
            .data
            .split_first_chunk::<4>()
            .ok_or(ParseError::Header)?;
        let (fourcc, rest) = rest.split_first_chunk::<4>().ok_or(ParseError::Header)?;
        let fourcc = FourCc(*fourcc);
        let (size, header, rest) = match u32::from_be_bytes(*size) {
            // To the end of the parent.
            0 => (8 + rest.len() as u64, 8, rest),
            1 => {
                let (large, rest) = rest.split_first_chunk::<8>().ok_or(ParseError::Header)?;
                (u64::from_be_bytes(*large), 16, rest)
            }
            n => (u64::from(n), 8, rest),
        };
        let content = size
            .checked_sub(header)
            .and_then(|n| usize::try_from(n).ok())
            .and_then(|n| rest.split_at_checked(n));
        let (content, rest) = content.ok_or(ParseError::Truncated(fourcc))?;
        self.data = rest;
        Ok((fourcc, content))
    }
}

impl<'a> Iterator for Boxes<'a> {
    type Item = Result<(FourCc, &'a [u8]), ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }
        let item = self.next_box();
        if item.is_err() {
            self.data = &[];
        }
        Some(item)
    }
}

#[cfg(test)]
#[path = "parse_test.rs"]
mod tests;
