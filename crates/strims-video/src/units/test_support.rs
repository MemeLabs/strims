// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Configs and units for the unit format's tests.

// Test support: a panic is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use strims_base::time::Time;

use crate::media::{AccessUnit, CodecParams, DecoderConfig, TrackId, TrackKind};
use crate::synth::script::{aac_lc_asc, avcc};

/// Payload bytes: a unit's payload is a prefix.
static PAYLOAD: [u8; 4096] = {
    let mut p = [0; 4096];
    let mut i = 0;
    while i < p.len() {
        p[i] = (i * 7 % 251) as u8;
        i += 1;
    }
    p
};

/// An H.264 track of a 640x360 picture at 90 kHz.
pub(crate) fn video(track: u32) -> DecoderConfig {
    DecoderConfig {
        track: TrackId(track),
        kind: TrackKind::Video,
        codec: "avc1.64001f".into(),
        timescale: 90_000,
        params: CodecParams::Avc {
            avcc: avcc(640, 360),
            width: 640,
            height: 360,
        },
    }
}

/// An AAC LC stereo track at 48 kHz.
pub(crate) fn audio(track: u32) -> DecoderConfig {
    DecoderConfig {
        track: TrackId(track),
        kind: TrackKind::Audio,
        codec: "mp4a.40.2".into(),
        timescale: 48_000,
        params: CodecParams::Aac {
            asc: aac_lc_asc(48_000),
            sample_rate: 48_000,
            channels: 2,
            frame_samples: 1024,
        },
    }
}

/// A unit of `len` payload bytes (at most 4096) at `dts`, shown at its
/// decode time, published at `dts` milliseconds past an epoch.
pub(crate) fn unit(track: u32, dts: i64, keyframe: bool, len: usize) -> AccessUnit<'static> {
    AccessUnit {
        track: TrackId(track),
        dts,
        pts: dts,
        keyframe,
        payload: &PAYLOAD[..len],
        published: Time::from_unix_millis(1_793_275_200_000 + dts),
    }
}
