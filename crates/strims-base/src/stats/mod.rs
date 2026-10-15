// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Moving averages and online variance.
//!
//! - [`Sma`]: a windowed simple moving average and rate.
//! - [`Ema`]: a bias-corrected exponential moving average.
//! - [`Welford`]: online mean and variance, mergeable.
//!
//! Time-dependent methods take `now`; nothing here reads a clock.

mod ema;
mod sma;
mod welford;

pub use ema::Ema;
pub use sma::Sma;
pub use welford::Welford;
