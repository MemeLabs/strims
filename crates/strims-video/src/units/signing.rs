// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! How a unit-format stream's swarm signs, chosen from the stream's
//! nominal bitrate when the swarm is created.

use strims_base::time::Duration;

use super::SignTimer;

/// How a unit-format stream's swarm signs: integrity units of
/// `chunks_per_signature` chunks, and a unit that hasn't filled signed
/// `interval` after its first byte ([`SignTimer`]).
///
/// A stream takes it when its swarm is created
/// ([`for_stream`](Self::for_stream)), and keeps it for the swarm's life:
/// the swarm URI carries the unit size. A peer verifies about
/// max(bitrate ÷ unit size, 1 ÷ interval) signatures a second, and no byte
/// waits longer than the interval for its signature. A unit signed early
/// is padded with zeros to its end and the padding is sent, so a stream
/// below unit size ÷ interval is sent at about that rate: 164 kbit/s for
/// [`SMALL`](Self::SMALL), 874 kbit/s for [`LARGE`](Self::LARGE) in 4 KiB
/// chunks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitSigning {
    /// A power of two.
    pub chunks_per_signature: usize,
    pub interval: Duration,
}

impl UnitSigning {
    /// 1-chunk units, a partial one signed after 200 ms.
    pub const SMALL: UnitSigning = UnitSigning {
        chunks_per_signature: 1,
        interval: Duration::from_millis(200),
    };

    /// 8-chunk units, a partial one signed after 300 ms.
    pub const LARGE: UnitSigning = UnitSigning {
        chunks_per_signature: 8,
        interval: Duration::from_millis(300),
    };

    /// The nominal bitrate, in bits a second, below which a stream takes
    /// [`SMALL`](Self::SMALL): about where 1-chunk units of 4 KiB reach 24
    /// verifies a second per peer. [`LARGE`](Self::LARGE) reaches it at
    /// about 6.3 Mbit/s and stays the choice above.
    pub const SMALL_BELOW_BPS: u64 = 750_000;

    /// The signing of a stream whose nominal bitrate, its video and audio
    /// together, is `nominal_bps` bits a second; 0 is none. A stream
    /// below [`SMALL_BELOW_BPS`](Self::SMALL_BELOW_BPS) takes
    /// [`SMALL`](Self::SMALL), one at or above it
    /// [`LARGE`](Self::LARGE). Without a nominal bitrate, a stream of audio
    /// alone (`audio_only`) takes `SMALL`, and one with video, or whose
    /// tracks aren't known, `LARGE`.
    pub fn for_stream(nominal_bps: Option<u64>, audio_only: bool) -> UnitSigning {
        match nominal_bps.filter(|&n| n > 0) {
            Some(n) if n < Self::SMALL_BELOW_BPS => Self::SMALL,
            Some(_) => Self::LARGE,
            None if audio_only => Self::SMALL,
            None => Self::LARGE,
        }
    }

    /// The bytes of an integrity unit in chunks of `chunk_size` bytes.
    pub fn unit_bytes(&self, chunk_size: usize) -> u64 {
        (chunk_size as u64).saturating_mul(self.chunks_per_signature as u64)
    }

    /// The sign timer of a stream in chunks of `chunk_size` bytes, at
    /// stream offset 0.
    pub fn timer(&self, chunk_size: usize) -> SignTimer {
        SignTimer::new(self.unit_bytes(chunk_size), self.interval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nominal_bitrate_below_750_kbits_takes_single_chunk_units() {
        for n in [1, 128_000, 600_000, 749_999] {
            assert_eq!(
                UnitSigning::for_stream(Some(n), false),
                UnitSigning::SMALL,
                "{n}"
            );
        }
        for n in [750_000, 2_660_000, 6_300_000, 50_000_000, u64::MAX] {
            assert_eq!(
                UnitSigning::for_stream(Some(n), true),
                UnitSigning::LARGE,
                "{n}"
            );
        }
    }

    #[test]
    fn without_a_nominal_bitrate_audio_alone_takes_single_chunk_units() {
        for nominal in [None, Some(0)] {
            assert_eq!(UnitSigning::for_stream(nominal, true), UnitSigning::SMALL);
            assert_eq!(UnitSigning::for_stream(nominal, false), UnitSigning::LARGE);
        }
    }

    #[test]
    fn the_timer_signs_units_of_the_chunks_at_the_interval() {
        assert_eq!(UnitSigning::SMALL.unit_bytes(4096), 4096);
        assert_eq!(UnitSigning::LARGE.unit_bytes(4096), 32 * 1024);
        assert_eq!(
            UnitSigning::LARGE.timer(4096),
            SignTimer::new(32 * 1024, Duration::from_millis(300))
        );
    }
}
