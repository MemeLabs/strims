// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The worker's allocator, which counts the heap for the Debug service's
//! snapshots ([`metrics`]), and the module's linear memory, which only
//! grows.

use std::alloc::System;

use cap::Cap;
use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
use strims_node_core::app::MemoryMetrics;

#[global_allocator]
static ALLOC: Cap<&'static StatsAlloc<System>> = Cap::new(&INSTRUMENTED_SYSTEM, usize::MAX);

/// The size of a wasm page.
const PAGE_BYTES: u64 = 65_536;

/// The heap since the worker started, and the linear memory it lives in.
pub fn metrics() -> MemoryMetrics {
    let stats = INSTRUMENTED_SYSTEM.stats();
    MemoryMetrics {
        live_bytes: ALLOC.allocated() as u64,
        peak_bytes: ALLOC.max_allocated() as u64,
        allocated_bytes: ALLOC.total_allocated() as u64,
        allocations: stats.allocations as u64,
        deallocations: stats.deallocations as u64,
        reallocations: stats.reallocations as u64,
        linear_memory_bytes: core::arch::wasm32::memory_size::<0>() as u64 * PAGE_BYTES,
    }
}
