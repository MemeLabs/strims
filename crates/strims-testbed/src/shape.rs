// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The shape of a live stream: what its seed writes and how often, and the
//! swarm's options.

use std::time::Duration;

use strims_ppspp::{DeliveryMode, PushMode, SeedPolicy, SwarmOptions};

use crate::stream::FRAME_LEN;

/// The testbed's stream rate in bytes per second: strims-sim's WAN cases'.
pub const RATE: u64 = 256 * 1024;
/// The testbed's live window, in chunks.
pub const WINDOW: usize = 4096;
/// The testbed's push streams.
pub const STREAMS: usize = 16;

/// A live stream: the seed writes `frames_per_write` frames every
/// `write_interval` into a swarm of `options`.
#[derive(Clone, Debug)]
pub struct LiveShape {
    pub options: SwarmOptions,
    pub write_interval: Duration,
    pub frames_per_write: usize,
}

impl LiveShape {
    /// About `rate` bytes per second: a write every 250 ms of a power-of-two
    /// number of frames (rounded down), one frame per chunk, one signature
    /// per write (so each write publishes at once), a live window of
    /// `window` chunks, mandatory delivery.
    pub fn live(rate: u64, window: usize) -> LiveShape {
        let interval = Duration::from_millis(250);
        let frames = usize::try_from(rate / 4 / FRAME_LEN as u64)
            .unwrap_or(1)
            .max(1);
        let frames = 1usize << frames.ilog2();
        LiveShape {
            options: SwarmOptions {
                chunk_size: FRAME_LEN,
                chunks_per_signature: frames,
                live_window: window,
                delivery_mode: DeliveryMode::MANDATORY,
                ..SwarmOptions::default()
            },
            write_interval: interval,
            frames_per_write: frames,
        }
    }

    /// The testbed's stream of about `rate` bytes per second, shaped like
    /// strims-sim's WAN cases: [`WINDOW`], [`STREAMS`] streams in Hybrid,
    /// the adaptive seed.
    pub fn testbed(rate: u64) -> LiveShape {
        let mut s = LiveShape::live(rate, WINDOW);
        s.options.push_mode = PushMode::Hybrid;
        s.options.stream_count = STREAMS;
        s.options.seed = SeedPolicy::ADAPTIVE;
        s
    }
}
