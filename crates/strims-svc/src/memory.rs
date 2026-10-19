// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The process's allocator, which counts the heap for the Debug service's
//! snapshots ([`metrics`]). The counters are atomics updated on every
//! allocation and never block.

use std::alloc::System;

use cap::Cap;
use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
use strims_node::app::MemoryMetrics;

#[global_allocator]
static ALLOC: Cap<&'static StatsAlloc<System>> = Cap::new(&INSTRUMENTED_SYSTEM, usize::MAX);

/// The heap since the process started.
pub(crate) fn metrics() -> MemoryMetrics {
    let stats = INSTRUMENTED_SYSTEM.stats();
    MemoryMetrics {
        live_bytes: ALLOC.allocated() as u64,
        peak_bytes: ALLOC.max_allocated() as u64,
        allocated_bytes: ALLOC.total_allocated() as u64,
        allocations: stats.allocations as u64,
        deallocations: stats.deallocations as u64,
        reallocations: stats.reallocations as u64,
        linear_memory_bytes: 0,
    }
}
