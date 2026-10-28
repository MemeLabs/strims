// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The self-checking stream the seeds write ([`strims_testbed::stream`]),
//! how the nodes' stores keep it ([`FrameCodec`]), and what viewers record.

pub use strims_testbed::stream::{
    FRAME_LEN, FrameInfo, FrameParser, Parsed, decode_frame, encode_frame, frame_header,
};

use strims_ppspp::store::ChunkCodec;

use crate::report::Percentiles;

/// How a node's store keeps a swarm's stream ([`ChunkCodec`]): a chunk of
/// whole frames as its first frame's sequence number and write time, 16
/// bytes in place of the chunk. A write's frames follow in sequence with
/// one time, and a chunk never spans writes ([`LiveShape::with_chunks`]),
/// so the rest of the chunk's frames follow from the first's; their
/// payloads follow from the run's seed, the swarm's index and their
/// sequence numbers ([`encode_frame`]). A seed that restarts, or resets its
/// writer, starts its chunks over at offset 0 and its sequence numbers
/// where its clock is, which the state carries.
///
/// [`LiveShape::with_chunks`]: strims_testbed::shape::LiveShape::with_chunks
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameCodec {
    run_seed: u64,
    swarm: u32,
}

impl FrameCodec {
    /// The codec of swarm `swarm`'s stream in a run of seed `run_seed`.
    pub fn new(run_seed: u64, swarm: u32) -> FrameCodec {
        FrameCodec { run_seed, swarm }
    }

    /// Whether chunks of `chunk_size` bytes are whole frames, which the
    /// codec needs.
    pub fn fits(chunk_size: usize) -> bool {
        chunk_size > 0 && chunk_size.is_multiple_of(FRAME_LEN)
    }
}

impl ChunkCodec for FrameCodec {
    fn state_len(&self) -> usize {
        16
    }

    /// Panics if `chunk`'s frame headers aren't the state's, in sequence
    /// from the first: a stream the codec would change fails its run
    /// rather than mislead it. Payloads go unchecked, as every chunk comes
    /// from the seed's writer or a peer the integrity checks verified.
    #[allow(clippy::panic)]
    fn encode(&self, offset: u64, chunk: &[u8], state: &mut [u8]) {
        let Some(first) = frame_header(self.swarm, chunk) else {
            panic!("chunk at {offset} of swarm {} isn't a frame", self.swarm);
        };
        for (i, f) in chunk.chunks(FRAME_LEN).enumerate() {
            let seq = first.seq + i as u64;
            let want = FrameInfo { seq, ..first };
            if f.len() != FRAME_LEN || frame_header(self.swarm, f) != Some(want) {
                panic!(
                    "chunk at {offset} of swarm {}: frame {i} isn't frame {seq} of {} ms",
                    self.swarm, first.time_ms
                );
            }
        }
        let words = [first.seq.to_le_bytes(), first.time_ms.to_le_bytes()];
        if let Some(dst) = state.get_mut(..16) {
            dst.copy_from_slice(words.as_flattened());
        }
    }

    fn decode(&self, _: u64, state: &[u8], out: &mut [u8]) {
        let word = |r: std::ops::Range<usize>| {
            state
                .get(r)
                .and_then(|w| w.try_into().ok())
                .map_or(0, u64::from_le_bytes)
        };
        let (seq, time_ms) = (word(0..8), word(8..16));
        for (i, f) in out.chunks_mut(FRAME_LEN).enumerate() {
            encode_frame(self.run_seed, self.swarm, seq + i as u64, time_ms, f);
        }
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
