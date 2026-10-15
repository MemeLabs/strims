// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The self-checking stream the seeds write, and what viewers record.
//!
//! A stream is a sequence of [`FRAME_LEN`]-byte frames, one per ppspp
//! chunk. Each frame carries a magic, the swarm index, its sequence number,
//! the simulated time it was written, and a payload derived from (run seed,
//! swarm, seq), so a viewer can check every byte without a copy of the
//! stream and measure each frame's lag. Sequence numbers come from the
//! write time (`seq = write index * frames per write`), so a seed that
//! restarts continues the numbering and viewers can tell exactly which
//! frames they missed.

use crate::report::Percentiles;

/// Bytes per frame; scenarios use it as the chunk size.
pub const FRAME_LEN: usize = 1024;
const MAGIC: [u8; 4] = *b"SIMF";
const HEADER_LEN: usize = 24;

fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn payload_seed(run_seed: u64, swarm: u32, seq: u64) -> u64 {
    splitmix(run_seed ^ splitmix(u64::from(swarm) << 48 ^ seq))
}

fn fill_payload(seed: u64, out: &mut [u8]) {
    let mut s = seed;
    for c in out.chunks_mut(8) {
        s = splitmix(s);
        let b = s.to_le_bytes();
        let n = c.len();
        c.copy_from_slice(b.get(..n).unwrap_or_default());
    }
}

/// Writes frame `seq` of `swarm` into `out` (exactly [`FRAME_LEN`] bytes).
pub fn encode_frame(run_seed: u64, swarm: u32, seq: u64, time_ms: u64, out: &mut [u8]) {
    let Some(out) = out.get_mut(..FRAME_LEN) else {
        return;
    };
    let (header, payload) = out.split_at_mut(HEADER_LEN);
    let fields: [&[u8]; 4] = [
        &MAGIC,
        &swarm.to_le_bytes(),
        &seq.to_le_bytes(),
        &time_ms.to_le_bytes(),
    ];
    let mut off = 0;
    for f in fields {
        if let Some(d) = header.get_mut(off..off + f.len()) {
            d.copy_from_slice(f);
        }
        off += f.len();
    }
    fill_payload(payload_seed(run_seed, swarm, seq), payload);
}

/// A verified frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameInfo {
    pub seq: u64,
    pub time_ms: u64,
}

/// Checks a whole frame: magic, swarm and every payload byte.
pub fn decode_frame(run_seed: u64, swarm: u32, b: &[u8]) -> Option<FrameInfo> {
    let b = b.get(..FRAME_LEN)?;
    let (header, payload) = b.split_at(HEADER_LEN);
    let word = |r: std::ops::Range<usize>| -> Option<u64> {
        let mut a = [0u8; 8];
        let s = header.get(r)?;
        a.get_mut(..s.len())?.copy_from_slice(s);
        Some(u64::from_le_bytes(a))
    };
    if header.get(..4)? != MAGIC || word(4..8)? != u64::from(swarm) {
        return None;
    }
    let seq = word(8..16)?;
    let time_ms = word(16..24)?;
    let mut s = payload_seed(run_seed, swarm, seq);
    for c in payload.chunks(8) {
        s = splitmix(s);
        if c != s.to_le_bytes().get(..c.len())? {
            return None;
        }
    }
    Some(FrameInfo { seq, time_ms })
}

/// Splits a viewer's byte stream into frames. Reads always resume at a
/// chunk boundary (the first read offset, an underrun's tail, a new epoch),
/// so frames should always be aligned; anything else is counted.
#[derive(Debug, Default)]
pub struct FrameParser {
    buf: Vec<u8>,
    start: usize,
}

/// What [`FrameParser::push`] found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parsed {
    Frame(FrameInfo),
    /// [`FRAME_LEN`] bytes at a frame boundary that aren't a valid frame.
    Bad,
}

impl FrameParser {
    pub fn push(&mut self, run_seed: u64, swarm: u32, data: &[u8], mut on: impl FnMut(Parsed)) {
        self.buf.extend_from_slice(data);
        while let Some(f) = self.buf.get(self.start..self.start + FRAME_LEN) {
            on(match decode_frame(run_seed, swarm, f) {
                Some(i) => Parsed::Frame(i),
                None => Parsed::Bad,
            });
            self.start += FRAME_LEN;
        }
        if self.start > 0 && self.start * 2 >= self.buf.len() {
            self.buf.drain(..self.start);
            self.start = 0;
        }
    }

    /// Drops a partial frame (after an underrun or a reset).
    pub fn resync(&mut self) {
        self.buf.clear();
        self.start = 0;
    }
}

/// A growable bitset of frame sequence numbers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SeqSet {
    words: Vec<u64>,
    len: u64,
}

impl SeqSet {
    /// Sets `seq`; returns whether it was new.
    pub fn insert(&mut self, seq: u64) -> bool {
        let Ok(w) = usize::try_from(seq / 64) else {
            return false;
        };
        if w >= self.words.len() {
            self.words.resize(w + 1, 0);
        }
        let bit = 1u64 << (seq % 64);
        let Some(word) = self.words.get_mut(w) else {
            return false;
        };
        let new = *word & bit == 0;
        *word |= bit;
        if new {
            self.len += 1;
        }
        new
    }

    pub fn contains(&self, seq: u64) -> bool {
        usize::try_from(seq / 64)
            .ok()
            .and_then(|w| self.words.get(w))
            .is_some_and(|word| word & (1u64 << (seq % 64)) != 0)
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// One past the largest seq the set can hold without growing.
    pub fn bound(&self) -> u64 {
        self.words.len() as u64 * 64
    }

    /// Members of `self` in `range` that `other` lacks.
    pub fn missing_from(&self, other: &SeqSet, range: std::ops::Range<u64>) -> u64 {
        self.iter_in(range).filter(|&s| !other.contains(s)).count() as u64
    }

    /// Members in `range`.
    pub fn count_in(&self, range: std::ops::Range<u64>) -> u64 {
        self.iter_in(range).count() as u64
    }

    /// Members in `range`, ascending (the range is clipped to the set).
    pub fn iter_in(&self, range: std::ops::Range<u64>) -> impl Iterator<Item = u64> + '_ {
        (range.start..range.end.min(self.bound())).filter(|&s| self.contains(s))
    }
}

/// What one viewer (a host reading one swarm) saw, across restarts.
#[derive(Clone, Debug, Default)]
pub struct ViewerStats {
    pub received: SeqSet,
    /// Frames read, duplicates included.
    pub frames: u64,
    /// `(seq, lag in ms)` for every frame read.
    pub lags: Vec<(u64, u32)>,
    /// Frame-sized blocks that failed verification.
    pub bad_frames: u64,
    /// Frames whose seq wasn't above the previous one in the same
    /// incarnation.
    pub regressions: u64,
    pub underruns: u32,
    pub skipped_bytes: u64,
    pub resets: u32,
    /// One entry per incarnation of the viewer's host.
    pub joins: Vec<Join>,
    last_seq: Option<u64>,
}

/// One incarnation of a viewer: when it started reading, when the first
/// frame arrived, and when it caught up (a frame with lag under
/// [`CAUGHT_UP_MS`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Join {
    pub start_ms: u64,
    pub first_frame_ms: Option<u64>,
    pub first_seq: Option<u64>,
    pub caught_up_ms: Option<u64>,
}

/// A viewer has caught up once a frame arrives this fresh.
pub const CAUGHT_UP_MS: u64 = 1000;

impl ViewerStats {
    /// A new incarnation of the viewer's host starts reading.
    pub fn start(&mut self, now_ms: u64) {
        self.joins.push(Join {
            start_ms: now_ms,
            ..Join::default()
        });
        self.last_seq = None;
    }

    /// Records a parsed block. Returns true for the first frame of an
    /// incarnation.
    pub fn record(&mut self, p: Parsed, now_ms: u64) -> bool {
        let f = match p {
            Parsed::Bad => {
                self.bad_frames += 1;
                return false;
            }
            Parsed::Frame(f) => f,
        };
        self.frames += 1;
        self.received.insert(f.seq);
        if self.last_seq.is_some_and(|l| f.seq <= l) {
            self.regressions += 1;
        }
        self.last_seq = Some(f.seq);
        let lag = u32::try_from(now_ms.saturating_sub(f.time_ms)).unwrap_or(u32::MAX);
        self.lags.push((f.seq, lag));
        let Some(j) = self.joins.last_mut() else {
            return false;
        };
        if j.caught_up_ms.is_none() && u64::from(lag) < CAUGHT_UP_MS {
            j.caught_up_ms = Some(now_ms);
        }
        if j.first_frame_ms.is_none() {
            j.first_frame_ms = Some(now_ms);
            j.first_seq = Some(f.seq);
            return true;
        }
        false
    }

    /// The lags of frames with seq in `range`.
    pub fn lag_in(&self, range: std::ops::Range<u64>) -> Percentiles {
        Percentiles::of(
            self.lags
                .iter()
                .filter(|(s, _)| range.contains(s))
                .map(|&(_, l)| l),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip_and_detect_damage() {
        let mut f = vec![0u8; FRAME_LEN];
        encode_frame(7, 2, 99, 12_345, &mut f);
        assert_eq!(
            decode_frame(7, 2, &f),
            Some(FrameInfo {
                seq: 99,
                time_ms: 12_345
            })
        );
        assert_eq!(decode_frame(8, 2, &f), None);
        assert_eq!(decode_frame(7, 3, &f), None);
        if let Some(b) = f.get_mut(500) {
            *b ^= 1;
        }
        assert_eq!(decode_frame(7, 2, &f), None);
    }

    #[test]
    fn parser_splits_reads_at_any_boundary() {
        let mut stream = vec![0u8; FRAME_LEN * 3];
        for (i, c) in stream.chunks_mut(FRAME_LEN).enumerate() {
            encode_frame(1, 0, i as u64, 0, c);
        }
        let mut p = FrameParser::default();
        let mut seqs = Vec::new();
        for piece in stream.chunks(700) {
            p.push(1, 0, piece, |x| match x {
                Parsed::Frame(f) => seqs.push(f.seq),
                Parsed::Bad => seqs.push(u64::MAX),
            });
        }
        assert_eq!(seqs, vec![0, 1, 2]);
    }

    #[test]
    fn seq_sets_count() {
        let mut a = SeqSet::default();
        let mut b = SeqSet::default();
        for s in [0, 5, 64, 200] {
            assert!(a.insert(s));
        }
        assert!(!a.insert(5));
        b.insert(5);
        assert_eq!(a.len(), 4);
        assert_eq!(a.missing_from(&b, 0..201), 3);
        assert_eq!(a.count_in(1..100), 2);
    }
}
