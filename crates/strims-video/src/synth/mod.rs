// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Synthetic streams and clients for tests.
//!
//! - [`script`]: deterministic FLV tag scripts and the records they
//!   produce.
//! - [`publisher`]: a sans-IO RTMP publisher client.

pub mod publisher;
pub mod script;
