// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Moving averages, online variance and histograms.
//!
//! - [`Sma`]: a windowed simple moving average and rate.
//! - [`Ema`]: a bias-corrected exponential moving average.
//! - [`Welford`]: online mean and variance, mergeable.
//! - [`WaitHistogram`]: waits in quarter-octave buckets, for percentiles.
//! - [`ValueHistogram`]: integers in the log sketch's buckets, for
//!   percentiles of gauges.
//! - [`sketch`]: the quantile method of the sims and the lab, the integer
//!   nearest rank read through a log sketch of 128 buckets a doubling.
//!
//! Time-dependent methods take `now`; nothing here reads a clock.

mod ema;
pub mod sketch;
mod sma;
mod values;
mod wait;
mod welford;

pub use ema::Ema;
pub use sma::Sma;
pub use values::ValueHistogram;
pub use wait::{WAIT_BUCKETS, WaitHistogram};
pub use welford::Welford;
