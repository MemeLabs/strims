// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Media fragments: a `moof` with a `traf` per track, then the `mdat`
//! holding the tracks' samples in `traf` order.

use crate::fmp4::boxes::{self, FULL_HEADER_LEN, HEADER_LEN, end, start, start_full};
use crate::media::{TrackId, TrackKind};

/// `tfhd` flags: data offsets count from the `moof`'s first byte.
const TFHD_DEFAULT_BASE_IS_MOOF: u32 = 0x02_0000;
/// `trun` flags.
const TRUN_DATA_OFFSET: u32 = 0x00_0001;
const TRUN_DURATION: u32 = 0x00_0100;
const TRUN_SIZE: u32 = 0x00_0200;
const TRUN_FLAGS: u32 = 0x00_0400;
const TRUN_COMPOSITION_OFFSET: u32 = 0x00_0800;
const TRUN_VIDEO: u32 =
    TRUN_DATA_OFFSET | TRUN_DURATION | TRUN_SIZE | TRUN_FLAGS | TRUN_COMPOSITION_OFFSET;
const TRUN_AUDIO: u32 = TRUN_DATA_OFFSET | TRUN_DURATION | TRUN_SIZE;
/// Sample flags of a sync sample: sample_depends_on 2 (no other).
pub(crate) const SYNC_SAMPLE_FLAGS: u32 = 0x0200_0000;
/// Sample flags of other video samples: sample_depends_on 1 (others) and
/// sample_is_non_sync_sample.
pub(crate) const NON_SYNC_SAMPLE_FLAGS: u32 = 0x0101_0000;

/// Bytes of `moof`'s header and `mfhd`.
const MOOF_FIXED_LEN: usize = HEADER_LEN + FULL_HEADER_LEN + 4;
/// Bytes of a `traf` before its `trun` entries: its header, `tfhd`,
/// `tfdt` v1, and `trun`'s header, sample count and data offset.
const TRAF_FIXED_LEN: usize =
    HEADER_LEN + (FULL_HEADER_LEN + 4) + (FULL_HEADER_LEN + 8) + (FULL_HEADER_LEN + 8);

/// One sample's `trun` entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sample {
    /// In the track's timescale.
    pub duration: u32,
    pub size: u32,
    /// A sync sample; written for video only, where other samples are
    /// marked non-sync.
    pub keyframe: bool,
    /// Presentation time less decode time; written for video only.
    pub composition_offset: i32,
}

/// A track's samples in a fragment.
#[derive(Debug, Clone, Copy)]
pub struct TrackRun<'a> {
    pub track: TrackId,
    pub kind: TrackKind,
    /// The decode time of the first sample, in the track's timescale.
    pub base_decode_time: u64,
    pub samples: &'a [Sample],
    /// The samples' bytes back to back, as long as their sizes sum to.
    pub data: &'a [u8],
}

impl Default for TrackRun<'_> {
    fn default() -> Self {
        TrackRun {
            track: TrackId(0),
            kind: TrackKind::Video,
            base_decode_time: 0,
            samples: &[],
            data: &[],
        }
    }
}

/// Bytes of a `trun` entry: duration, size, flags and composition offset
/// for video; duration and size for audio.
pub(crate) fn entry_len(kind: TrackKind) -> usize {
    match kind {
        TrackKind::Video => 16,
        TrackKind::Audio => 8,
    }
}

/// Bytes a track with `samples` adds to a `moof`: none without samples.
pub(crate) fn traf_len(kind: TrackKind, samples: usize) -> usize {
    if samples == 0 {
        0
    } else {
        TRAF_FIXED_LEN + samples * entry_len(kind)
    }
}

/// Bytes of the `moof` and `mdat` header of [`write_fragment_head`].
pub fn fragment_head_len(runs: &[TrackRun<'_>]) -> usize {
    moof_len(runs) + HEADER_LEN
}

/// Bytes of the fragment [`write_fragment`] writes.
pub fn fragment_len(runs: &[TrackRun<'_>]) -> usize {
    fragment_head_len(runs) + data_len(runs)
}

/// Bytes of the fragment of an empty `moof`, with no `traf`s and an empty
/// `mdat`: the base that [`traf_len`] and the samples' bytes add to.
pub(crate) const EMPTY_FRAGMENT_LEN: usize = MOOF_FIXED_LEN + HEADER_LEN;

fn moof_len(runs: &[TrackRun<'_>]) -> usize {
    MOOF_FIXED_LEN
        + runs
            .iter()
            .map(|r| traf_len(r.kind, r.samples.len()))
            .sum::<usize>()
}

fn data_len(runs: &[TrackRun<'_>]) -> usize {
    runs.iter()
        .filter(|r| !r.samples.is_empty())
        .map(|r| r.data.len())
        .sum()
}

/// Appends the fragment of `runs`: [`write_fragment_head`], then each
/// run's data.
pub fn write_fragment(sequence: u32, runs: &[TrackRun<'_>], out: &mut Vec<u8>) {
    write_fragment_head(sequence, runs, out);
    for run in runs.iter().filter(|r| !r.samples.is_empty()) {
        out.extend_from_slice(run.data);
    }
}

/// Appends a fragment's `moof` and its `mdat` header; the runs' data,
/// back to back in run order, completes the `mdat`.
///
/// - `mfhd` holds `sequence`.
/// - Each run with samples gets a `traf`: `tfhd` (default-base-is-moof),
///   `tfdt` v1 with the base decode time, and `trun` v1 whose data offset
///   points at the run's data in the `mdat`. Video entries carry
///   duration, size, flags (sync, or non-sync depending on others) and a
///   signed composition offset; audio entries duration and size.
///
/// A fragment stays below 2 GiB: data offsets are signed 32-bit.
pub fn write_fragment_head(sequence: u32, runs: &[TrackRun<'_>], out: &mut Vec<u8>) {
    let moof_len = moof_len(runs);
    let mut data_offset = moof_len + HEADER_LEN;
    let moof = start(out, b"moof");
    let mfhd = start_full(out, b"mfhd", 0, 0);
    boxes::u32(out, sequence);
    end(out, mfhd);
    for run in runs.iter().filter(|r| !r.samples.is_empty()) {
        let traf = start(out, b"traf");
        let tfhd = start_full(out, b"tfhd", 0, TFHD_DEFAULT_BASE_IS_MOOF);
        boxes::u32(out, run.track.0);
        end(out, tfhd);
        let tfdt = start_full(out, b"tfdt", 1, 0);
        boxes::u64(out, run.base_decode_time);
        end(out, tfdt);
        let flags = match run.kind {
            TrackKind::Video => TRUN_VIDEO,
            TrackKind::Audio => TRUN_AUDIO,
        };
        let trun = start_full(out, b"trun", 1, flags);
        boxes::u32(out, u32::try_from(run.samples.len()).unwrap_or(u32::MAX));
        boxes::i32(out, i32::try_from(data_offset).unwrap_or(i32::MAX));
        for s in run.samples {
            boxes::u32(out, s.duration);
            boxes::u32(out, s.size);
            if run.kind == TrackKind::Video {
                boxes::u32(
                    out,
                    if s.keyframe {
                        SYNC_SAMPLE_FLAGS
                    } else {
                        NON_SYNC_SAMPLE_FLAGS
                    },
                );
                boxes::i32(out, s.composition_offset);
            }
        }
        end(out, trun);
        end(out, traf);
        data_offset += run.data.len();
    }
    end(out, moof);
    debug_assert_eq!(out.len() - moof, moof_len);
    let mdat = HEADER_LEN + data_len(runs);
    boxes::u32(out, u32::try_from(mdat).unwrap_or(u32::MAX));
    out.extend_from_slice(b"mdat");
}

#[cfg(test)]
#[path = "fragment_test.rs"]
mod tests;
