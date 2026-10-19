// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A service's Debug frontend over its API: metrics snapshots of the
//! profile's node, its controls and the platform's heap.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::time::Duration as StdDuration;

use futures_util::StreamExt;
use strims_node::app::MemoryMetrics;
use strims_proto::debug::v1::{DebugClient, ReadMetricsRequest, WatchMetricsRequest};
use strims_rpc::Client;
use support::video::Profile;
use support::*;

const HEAP: MemoryMetrics = MemoryMetrics {
    live_bytes: 7,
    peak_bytes: 11,
    allocated_bytes: 13,
    allocations: 17,
    deallocations: 19,
    reallocations: 23,
    linear_memory_bytes: 0,
};

fn heap() -> MemoryMetrics {
    HEAP
}

fn debug(client: Client) -> DebugClient<Client> {
    DebugClient::new(client)
}

#[test]
fn the_snapshot_reports_the_node_and_the_platforms_memory() {
    run(async {
        let mut config = local_config();
        config.app.memory = Some(heap);
        let alice = Profile::sign_up(config).await;

        let m = debug(alice.client())
            .read_metrics(&ReadMetricsRequest {})
            .await
            .unwrap()
            .metrics
            .unwrap();
        let memory = m.memory.unwrap();
        assert_eq!(
            (memory.live_bytes, memory.peak_bytes, memory.allocated_bytes),
            (7, 11, 13)
        );
        assert_eq!(
            (
                memory.allocations,
                memory.deallocations,
                memory.reallocations
            ),
            (17, 19, 23)
        );
        assert!(m.time > 0);
        let driver = m.node.unwrap().driver.unwrap();
        assert!(driver.loop_iterations > 0 && driver.queries > 0);
        assert!(m.network.is_some() && m.transfer.is_some() && m.video.is_some());
        assert!(m.debug.unwrap().mock_streams.is_empty());
        alice.svc.stop().await;
    });
}

#[test]
fn a_service_without_heap_counters_leaves_the_heap_out() {
    run(async {
        let alice = Profile::sign_up(local_config()).await;
        let m = debug(alice.client())
            .read_metrics(&ReadMetricsRequest {})
            .await
            .unwrap()
            .metrics
            .unwrap();
        assert!(m.memory.is_none());
        assert!(m.node.is_some());
        alice.svc.stop().await;
    });
}

#[test]
fn watch_metrics_answers_snapshots_over_the_api() {
    run(async {
        let alice = Profile::sign_up(local_config()).await;
        let mut watch = debug(alice.client())
            .watch_metrics(&WatchMetricsRequest { interval_ms: 250 })
            .await
            .unwrap();
        let mut times = Vec::new();
        for _ in 0..3 {
            let next = tokio::time::timeout(StdDuration::from_secs(10), watch.next()).await;
            let m = next.unwrap().unwrap().unwrap().metrics.unwrap();
            assert!(m.node.is_some());
            times.push(m.time);
        }
        for pair in times.windows(2) {
            assert!(
                pair[1] - pair[0] >= 200_000_000,
                "snapshots {times:?} came faster than the interval"
            );
        }
        drop(watch);
        alice.svc.stop().await;
    });
}
