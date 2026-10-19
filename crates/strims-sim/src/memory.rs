// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Per-run heap accounting.
//!
//! A test binary that wants memory numbers installs the tracking allocator:
//!
//! ```ignore
//! #[global_allocator]
//! static ALLOC: strims_sim::memory::Allocator<std::alloc::System> =
//!     strims_sim::memory::Allocator::system();
//! ```
//!
//! Each run then registers an allocation group and enters it on its
//! thread. A turmoil simulation runs every host on that one thread, so the
//! group sees every allocation of the run and nothing from runs on other
//! threads: the other runs of its sweep, or tests running in parallel.
//! Without the allocator the counters stay at zero and reports carry no
//! memory section. Memory numbers never enter the trace: allocator
//! behaviour is not part of the determinism contract.

use std::sync::Once;
use std::sync::atomic::{AtomicI64, Ordering};

pub use tracking_allocator::Allocator;
use tracking_allocator::{
    AllocationGroupId, AllocationGroupToken, AllocationGuard, AllocationRegistry, AllocationTracker,
};

/// Allocation groups with counters; later groups aren't counted.
const GROUPS: usize = 4096;

static LIVE: [AtomicI64; GROUPS] = [const { AtomicI64::new(0) }; GROUPS];
static PEAK: [AtomicI64; GROUPS] = [const { AtomicI64::new(0) }; GROUPS];

fn slot(id: &AllocationGroupId) -> Option<usize> {
    let i = id.as_usize().get();
    (i < GROUPS).then_some(i)
}

struct Tracker;

impl AllocationTracker for Tracker {
    fn allocated(&self, _addr: usize, _size: usize, wrapped: usize, group: AllocationGroupId) {
        if let Some(i) = slot(&group)
            && let (Some(live), Some(peak)) = (LIVE.get(i), PEAK.get(i))
        {
            let n = i64::try_from(wrapped).unwrap_or(i64::MAX);
            let now = live.fetch_add(n, Ordering::Relaxed).saturating_add(n);
            peak.fetch_max(now, Ordering::Relaxed);
        }
    }

    fn deallocated(
        &self,
        _addr: usize,
        _size: usize,
        wrapped: usize,
        source: AllocationGroupId,
        _current: AllocationGroupId,
    ) {
        if let Some(i) = slot(&source)
            && let Some(live) = LIVE.get(i)
        {
            live.fetch_sub(
                i64::try_from(wrapped).unwrap_or(i64::MAX),
                Ordering::Relaxed,
            );
        }
    }
}

fn install() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        // Fails only if another tracker is set, which leaves the counters
        // at zero (no memory report).
        if AllocationRegistry::set_global_tracker(Tracker).is_ok() {
            AllocationRegistry::enable_tracking();
        }
    });
}

/// One run's allocation group.
pub struct MemoryGroup {
    token: AllocationGroupToken,
    slot: Option<usize>,
}

impl MemoryGroup {
    pub fn new() -> Option<MemoryGroup> {
        install();
        let token = AllocationGroupToken::register()?;
        let slot = slot(&token.id());
        Some(MemoryGroup { token, slot })
    }

    /// Counts this thread's allocations in the group until the guard
    /// drops.
    pub fn enter(&mut self) -> AllocationGuard<'_> {
        self.token.enter()
    }

    pub fn counters(&self) -> GroupCounters {
        GroupCounters { slot: self.slot }
    }
}

/// Reads a group's counters (cheap to copy into the simulation).
#[derive(Clone, Copy, Debug)]
pub struct GroupCounters {
    slot: Option<usize>,
}

impl GroupCounters {
    /// Live heap bytes (allocator overhead included).
    pub fn live(&self) -> i64 {
        self.slot
            .and_then(|i| LIVE.get(i))
            .map_or(0, |a| a.load(Ordering::Relaxed))
    }

    pub fn peak(&self) -> i64 {
        self.slot
            .and_then(|i| PEAK.get(i))
            .map_or(0, |a| a.load(Ordering::Relaxed))
    }
}
