// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The calibration testbed's node (`hack/testbed/`), in parts that build
//! for both of its drivers: strims-sim's native `testbed_node` and the
//! browser node of strims-wasm's `e2e` feature. strims-sim's scenarios
//! write and check the same stream ([`stream`]) and sample the same
//! transfers ([`transfer`]), so a testbed run compares with a simulated
//! one line for line. A run can stream real media instead ([`media`]),
//! which browser viewers play.

#[cfg(not(target_arch = "wasm32"))]
pub mod browser;
pub mod media;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
pub mod node;
pub mod shape;
pub mod stream;
pub mod topology;
pub mod transfer;
