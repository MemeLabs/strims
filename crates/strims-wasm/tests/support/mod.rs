// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The native harness of the browser tests (see [`browser`]).

#![allow(
    // Each test binary uses part of the harness.
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    // A browser, an HTTP server, threads and the wall clock.
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

pub mod browser;
