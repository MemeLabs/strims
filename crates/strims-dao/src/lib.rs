// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A profile's encrypted store.
//!
//! Each profile keeps its records in one [`strims_kv`] table, sealed with
//! the profile's storage key ([`seal`]) under keys laid out by [`key`].

pub mod ids;
pub mod key;
pub mod lease;
pub mod seal;
pub mod vv;
