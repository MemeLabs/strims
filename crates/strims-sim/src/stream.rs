// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The self-checking stream the seeds write ([`strims_testbed::stream`]),
//! how the nodes' stores keep it ([`FrameCodec`]), and what viewers record.
//!
//! A media swarm ([`crate::MediaSpec`]) is self-checking per chunk: a
//! chunk that holds media opens with a [`MEDIA_HEADER`] (its sequence
//! number and the time of its first byte, little-endian), then bytes that
//! follow from the run's seed, the swarm's index and the sequence number,
//! then zeros to its end if its unit was signed early; a chunk of padding
//! alone is zeros. Its stores keep each chunk as [`MediaCodec`] state, and
//! viewers check whole chunks ([`MediaParser`]).

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

/// The bytes that open a media chunk: its sequence number and its first
/// byte's time in ms of simulated time.
pub const MEDIA_HEADER: u64 = 16;

fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Media chunk `seq`'s bytes from chunk position `from` on, into `out`
/// (the header's positions included: the payload is a function of the
/// position).
pub fn fill_media(run_seed: u64, swarm: u32, seq: u64, from: usize, out: &mut [u8]) {
    let key = splitmix(run_seed ^ splitmix(u64::from(swarm) << 48 ^ seq ^ 0x4d45_4449_4100_0000));
    let mut word = None;
    for (i, b) in (from..).zip(out.iter_mut()) {
        let w = i / 8;
        let v = match word {
            Some((at, v)) if at == w => v,
            _ => {
                let v = splitmix(key ^ w as u64);
                word = Some((w, v));
                v
            }
        };
        *b = v.to_le_bytes().get(i % 8).copied().unwrap_or(0);
    }
}

/// The header of media chunk `seq`, first written at `time_ms`.
pub fn media_header(seq: u64, time_ms: u64) -> [u8; 16] {
    let mut h = [0; 16];
    let words = [seq.to_le_bytes(), time_ms.to_le_bytes()];
    h.copy_from_slice(words.as_flattened());
    h
}

/// A media chunk's sequence number, time and bytes up to its last nonzero
/// one; `None` for a chunk of zeros.
fn media_state(chunk: &[u8]) -> Option<(FrameInfo, usize)> {
    let n = chunk.iter().rposition(|&b| b != 0)? + 1;
    let word = |r: std::ops::Range<usize>| {
        chunk
            .get(r)
            .and_then(|w| w.try_into().ok())
            .map_or(0, u64::from_le_bytes)
    };
    Some((
        FrameInfo {
            seq: word(0..8),
            time_ms: word(8..16),
        },
        n,
    ))
}

/// Media chunk `info`'s bytes, `n` of them before the zeros, into `out`.
fn media_chunk(run_seed: u64, swarm: u32, info: FrameInfo, n: usize, out: &mut [u8]) {
    out.fill(0);
    let head = MEDIA_HEADER as usize;
    if let Some(h) = out.get_mut(..head) {
        h.copy_from_slice(&media_header(info.seq, info.time_ms));
    }
    if let Some(p) = out.get_mut(head..n.max(head)) {
        fill_media(run_seed, swarm, info.seq, head, p);
    }
}

/// How a node's store keeps a media swarm's chunks ([`ChunkCodec`]): a
/// chunk as its sequence number, time and the bytes before its zeros, 18
/// bytes of state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaCodec {
    run_seed: u64,
    swarm: u32,
}

impl MediaCodec {
    /// The codec of media swarm `swarm`'s stream in a run of seed
    /// `run_seed`.
    pub fn new(run_seed: u64, swarm: u32) -> MediaCodec {
        MediaCodec { run_seed, swarm }
    }
}

impl ChunkCodec for MediaCodec {
    fn state_len(&self) -> usize {
        18
    }

    /// Panics if `chunk` isn't a media chunk of the swarm, or zeros: a
    /// stream the codec would change fails its run rather than mislead it.
    #[allow(clippy::panic)]
    fn encode(&self, offset: u64, chunk: &[u8], state: &mut [u8]) {
        let (info, n) = media_state(chunk).unwrap_or((FrameInfo { seq: 0, time_ms: 0 }, 0));
        if n > 0 {
            let mut want = vec![0; chunk.len()];
            media_chunk(self.run_seed, self.swarm, info, n, &mut want);
            if want != chunk {
                panic!(
                    "chunk at {offset} of swarm {} isn't a media chunk",
                    self.swarm
                );
            }
        }
        let n = u16::try_from(n).unwrap_or(u16::MAX).to_le_bytes();
        let words = [info.seq.to_le_bytes(), info.time_ms.to_le_bytes()];
        if let Some(dst) = state.get_mut(..16) {
            dst.copy_from_slice(words.as_flattened());
        }
        if let Some(dst) = state.get_mut(16..18) {
            dst.copy_from_slice(&n);
        }
    }

    fn decode(&self, _: u64, state: &[u8], out: &mut [u8]) {
        let word = |r: std::ops::Range<usize>| {
            state
                .get(r)
                .and_then(|w| w.try_into().ok())
                .map_or(0, u64::from_le_bytes)
        };
        let n = state
            .get(16..18)
            .and_then(|w| w.try_into().ok())
            .map_or(0, u16::from_le_bytes);
        if n == 0 {
            out.fill(0);
            return;
        }
        let info = FrameInfo {
            seq: word(0..8),
            time_ms: word(8..16),
        };
        media_chunk(self.run_seed, self.swarm, info, usize::from(n), out);
    }
}

/// Reassembles a media swarm's chunks from reads at stream offsets, and
/// checks each: a chunk that holds media parses as its frame, or as bad if
/// its bytes aren't the swarm's; a chunk of zeros is padding and parses as
/// nothing. A read that doesn't follow the last one drops the chunk in
/// progress, and parsing resumes at the next chunk boundary.
#[derive(Clone, Debug, Default)]
pub struct MediaParser {
    chunk: Vec<u8>,
    /// The stream offset of the next byte the chunk in progress takes.
    next: Option<u64>,
}

impl MediaParser {
    /// Feeds `data`, read at stream offset `offset`, in chunks of
    /// `chunk_size` bytes.
    pub fn push(
        &mut self,
        run_seed: u64,
        swarm: u32,
        chunk_size: usize,
        offset: u64,
        data: &[u8],
        mut on: impl FnMut(Parsed),
    ) {
        let size = chunk_size.max(1) as u64;
        if self.next != Some(offset) {
            self.chunk.clear();
        }
        let (mut data, mut pos) = (data, offset);
        while !data.is_empty() {
            if self.chunk.is_empty() && !pos.is_multiple_of(size) {
                let skip = usize::try_from(size - pos % size).unwrap_or(usize::MAX);
                let skip = skip.min(data.len());
                data = data.get(skip..).unwrap_or_default();
                pos += skip as u64;
                continue;
            }
            let take = (chunk_size - self.chunk.len()).min(data.len());
            let (head, rest) = data.split_at(take);
            self.chunk.extend_from_slice(head);
            data = rest;
            pos += take as u64;
            if self.chunk.len() < chunk_size {
                break;
            }
            if let Some((info, n)) = media_state(&self.chunk) {
                let mut want = vec![0; chunk_size];
                media_chunk(run_seed, swarm, info, n, &mut want);
                on(if want == self.chunk {
                    Parsed::Frame(info)
                } else {
                    Parsed::Bad
                });
            }
            self.chunk.clear();
        }
        self.next = Some(pos);
    }

    /// Drops the chunk in progress: the next read starts over at a chunk
    /// boundary.
    pub fn resync(&mut self) {
        self.chunk.clear();
        self.next = None;
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
    #![allow(clippy::indexing_slicing)]

    use super::*;

    /// A media stream of `chunks` chunks: chunk `i` holds `i * 300 + 20`
    /// bytes (all of it past 13 chunks), chunk 3 is padding alone.
    fn media_stream(chunk: usize, chunks: usize) -> Vec<u8> {
        let mut out = vec![0u8; chunk * chunks];
        for (i, c) in out.chunks_mut(chunk).enumerate() {
            if i == 3 {
                continue;
            }
            let n = (i * 300 + 20).min(chunk);
            c[..16].copy_from_slice(&media_header(i as u64, 1000 + i as u64));
            fill_media(7, 2, i as u64, 16, &mut c[16..n]);
        }
        out
    }

    #[test]
    fn media_chunks_round_trip_through_the_codec() {
        let codec = MediaCodec::new(7, 2);
        let stream = media_stream(4096, 16);
        for (i, c) in stream.chunks(4096).enumerate() {
            let mut state = [0u8; 18];
            codec.encode(i as u64 * 4096, c, &mut state);
            let mut out = vec![1u8; 4096];
            codec.decode(i as u64 * 4096, &state, &mut out);
            assert_eq!(out, c, "chunk {i}");
        }
    }

    #[test]
    fn media_parsers_check_whole_chunks_and_resync() {
        let stream = media_stream(4096, 16);
        let mut got = Vec::new();
        let mut p = MediaParser::default();
        // Odd pieces, from mid-chunk 0: chunk 0 is skipped, 3 is padding.
        let mut at = 100;
        for piece in stream[100..7 * 4096 + 2000].chunks(777) {
            p.push(7, 2, 4096, at, piece, |x| got.push(x));
            at += piece.len() as u64;
        }
        // A gap: the chunk in progress (7) goes, 9 on parse.
        p.push(
            7,
            2,
            4096,
            9 * 4096 - 5,
            &stream[9 * 4096 - 5..11 * 4096],
            |x| got.push(x),
        );
        let seqs: Vec<u64> = got
            .iter()
            .map(|x| match x {
                Parsed::Frame(f) => f.seq,
                Parsed::Bad => u64::MAX,
            })
            .collect();
        assert_eq!(seqs, [1, 2, 4, 5, 6, 9, 10]);
        let mut bad = stream[4096..2 * 4096].to_vec();
        bad[200] ^= 1;
        let mut got = Vec::new();
        MediaParser::default().push(7, 2, 4096, 4096, &bad, |x| got.push(x));
        assert_eq!(got, [Parsed::Bad]);
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
