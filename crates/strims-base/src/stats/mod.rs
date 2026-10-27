// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Moving averages, online variance and wait histograms.
//!
//! - [`Sma`]: a windowed simple moving average and rate.
//! - [`Ema`]: a bias-corrected exponential moving average.
//! - [`Welford`]: online mean and variance, mergeable.
//! - [`WaitHistogram`]: waits in quarter-octave buckets, for percentiles.
//!
//! Time-dependent methods take `now`; nothing here reads a clock.

mod ema;
mod sma;
mod wait;
mod welford;

pub use ema::Ema;
pub use sma::Sma;
pub use wait::{WAIT_BUCKETS, WaitHistogram};
pub use welford::Welford;
