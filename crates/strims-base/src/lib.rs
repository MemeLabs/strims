// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Deterministic primitives shared by every strims crate.
//!
//! Core crates never read the wall clock or OS entropy. Time is passed in as
//! [`time::Time`], and randomness comes from a seeded [`rng::DetRng`] owned
//! by the state machine that needs it. [`hash`]'s maps are the exception:
//! their keys decide their allocations, never what they hold or yield.

pub mod hash;
pub mod path;
pub mod rng;
pub mod stats;
pub mod time;
