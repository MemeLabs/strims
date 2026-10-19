// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Mock streams on fake nodes (see `strims_testworld`), through the UI's
//! RPC connection: a node lists and writes its mock streams, others read
//! them as their debug config says, and both take them down as streams,
//! configs and networks change; and the metrics snapshots.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::Cell;
use std::fmt::Debug;
use std::future::Future;
use std::rc::Rc;

use futures_util::StreamExt;
use strims_base::time::Duration;
use strims_dao::Table;
use strims_dao::schema::network::Networks;
use strims_debug::{
    Config, DebugControl, MOCK_STREAM_SERVICE, MetricsSource, NoMetrics, ReaderInfo, bind_frontend,
};
use strims_kv::MemoryKv;
use strims_ppspp::{SwarmId, Uri};
use strims_proto::debug::v1::{
    Config as DebugConfig, DebugClient, GetConfigRequest, ListMockStreamsRequest, MemoryMetrics,
    Metrics, MockStream, ReadMetricsRequest, SetConfigRequest, StartMockStreamRequest,
    StopMockStreamRequest, WatchMetricsRequest,
};
use strims_proto::network::v1::directory::listing::Content;
use strims_proto::service::{CallError, Error as RpcError};
use strims_rpc::{Client, Router, Timer};
use strims_testworld::*;
use strims_transfer::TransferId;

type Control = DebugControl<MemoryKv, FakeHost, TokioTimer>;

fn start(n: &Node, config: Config) -> Control {
    let debug = DebugControl::new(
        n.host.clone(),
        n.store.clone(),
        n.network.clone(),
        n.transfers.clone(),
        n.directory.clone(),
        n.timer.clone(),
        n.rng(),
        config,
    );
    let d = debug.clone();
    n.spawn(async move { d.run().await });
    debug
}

/// A UI's client of `debug`'s frontend, with snapshots of `metrics`.
fn connect_ui_with<M: MetricsSource>(debug: &Control, metrics: M) -> DebugClient<Client> {
    let router = Router::new();
    bind_frontend(&router, debug, metrics);
    DebugClient::new(frontend_client(router))
}

fn connect_ui(debug: &Control) -> DebugClient<Client> {
    connect_ui_with(debug, NoMetrics)
}

/// The message of a call's remote error.
fn refusal<T: Debug>(r: Result<T, CallError>) -> String {
    match r {
        Err(CallError::Remote(e)) => e.message,
        r => panic!("not refused: {r:?}"),
    }
}

/// A node, its debug control and its UI.
struct Debugged {
    node: Node,
    debug: Control,
    ui: DebugClient<Client>,
}

impl Debugged {
    fn new(node: Node, config: Config) -> Debugged {
        let debug = start(&node, config);
        let ui = connect_ui(&debug);
        Debugged { node, debug, ui }
    }

    /// Starts a mock stream on `key` of 64 kbps, a segment a second, for
    /// `timeout_ms`: its id.
    async fn start_stream(&self, key: &[u8], timeout_ms: u32) -> u64 {
        let req = StartMockStreamRequest {
            bitrate_kbps: 64,
            segment_interval_ms: 0,
            timeout_ms,
            network_key: key.to_vec(),
        };
        self.ui.start_mock_stream(&req).await.unwrap().id
    }

    async fn stop_stream(&self, id: u64) {
        let req = StopMockStreamRequest { id };
        self.ui.stop_mock_stream(&req).await.unwrap();
    }

    async fn list(&self) -> Vec<MockStream> {
        let req = ListMockStreamsRequest {};
        self.ui.list_mock_streams(&req).await.unwrap().mock_streams
    }

    async fn set_config(&self, enable: bool, key: &[u8]) -> Result<DebugConfig, CallError> {
        let req = SetConfigRequest {
            config: Some(DebugConfig {
                enable_mock_streams: enable,
                mock_stream_network_key: key.to_vec(),
            }),
        };
        Ok(self.ui.set_config(&req).await?.config.unwrap())
    }

    async fn get_config(&self) -> DebugConfig {
        let req = GetConfigRequest {};
        self.ui.get_config(&req).await.unwrap().config.unwrap()
    }

    /// The swarm of mock stream `id`, once listed.
    fn swarm_of(&self, id: u64) -> SwarmId {
        let m = self.debug.mock_streams();
        let s = m.iter().find(|s| s.id == id).unwrap();
        Uri::parse(&s.swarm_uri).unwrap().id
    }

    /// The listing id of mock stream `id`, once published.
    fn listing_of(&self, id: u64) -> Option<u64> {
        let m = self.debug.mock_streams();
        m.iter().find(|s| s.id == id).and_then(|s| s.listing_id)
    }

    /// The reader of the listing `listing_id`.
    fn reader_of(&self, listing_id: u64) -> Option<ReaderInfo> {
        let readers = self.debug.readers();
        readers.into_iter().find(|r| r.listing_id == listing_id)
    }

    /// Whether the reader of the listing `listing_id` has read a segment.
    fn reads(&self, listing_id: u64) -> bool {
        self.reader_of(listing_id).is_some_and(|r| r.segments > 0)
    }

    /// The ids of the mock stream listings of network `network_id`'s
    /// view.
    fn mock_listings(&self, network_id: u64) -> Vec<u64> {
        self.node
            .directory
            .listings(network_id)
            .into_iter()
            .filter(|l| {
                matches!(&l.listing.content,
                    Some(Content::Service(s)) if s.r#type == MOCK_STREAM_SERVICE)
            })
            .map(|l| l.id)
            .collect()
    }

    async fn delete_network(&self, id: u64) {
        let mut tx = self.node.store.write(self.node.timer.now()).await.unwrap();
        Networks::delete(&mut tx, id).await.unwrap();
        tx.commit().await.unwrap();
    }
}

/// Two nodes on network 10, owned by `a`, each running its debug control.
struct Pair {
    a: Debugged,
    b: Debugged,
    key: Vec<u8>,
    _pipes: Vec<Pipe>,
}

async fn pair_with(config: Config) -> Pair {
    let world = World::default();
    let a = node_in(&world, 1).await;
    let b = node_in(&world, 2).await;
    let (net, pipes) = owned_network(&a, &[&b], 10).await;
    Pair {
        a: Debugged::new(a, Config::default()),
        b: Debugged::new(b, config),
        key: net.public,
        _pipes: pipes,
    }
}

async fn pair() -> Pair {
    pair_with(Config::default()).await
}

impl Pair {
    /// A mock stream on `a`, once its listing is in `b`'s view: its id and
    /// listing id.
    async fn listed_stream(&self) -> (u64, u64) {
        let id = self.a.start_stream(&self.key, 0).await;
        wait_for(10, "the listing", || self.a.listing_of(id).is_some()).await;
        let listing = self.a.listing_of(id).unwrap();
        wait_for(10, "the listing in b's view", || {
            self.b.mock_listings(10).contains(&listing)
        })
        .await;
        (id, listing)
    }

    /// Whether `b` runs the swarm of `a`'s mock stream `id`: adds it and
    /// publishes its transfer on the network.
    fn b_runs(&self, swarm: &SwarmId) -> bool {
        let t = TransferId::new(swarm, b"");
        self.b.node.host.swarms_of(swarm) == (0, 1)
            && self
                .b
                .node
                .transfers
                .is_published(t, self.b.node.net(&self.key))
    }
}

/// A mock stream is listed on its network, and a node whose config names
/// the network reads it and reports each segment's latency.
#[test]
fn a_listed_mock_stream_is_read_with_its_latency() {
    run(async {
        let p = pair().await;
        p.b.set_config(true, &p.key).await.unwrap();
        let (id, listing) = p.listed_stream().await;
        let swarm = p.a.swarm_of(id);
        assert_eq!(p.a.node.host.swarms_of(&swarm), (1, 0));
        let t = TransferId::new(&swarm, b"");
        assert!(p.a.node.transfers.is_published(t, p.a.node.net(&p.key)));

        wait_for(10, "a segment read", || p.b.reads(listing)).await;
        let r = p.b.reader_of(listing).unwrap();
        assert_eq!(r.network_key, p.key);
        assert_eq!(r.swarm_id, swarm);
        assert_eq!(r.id, Some(id));
        let latency = r.latency.unwrap();
        assert!(
            latency >= Duration::ZERO && latency < Duration::SECOND,
            "{latency}"
        );
        assert!(r.last_segment_at.is_some());
        assert!(p.b_runs(&swarm));

        let streams = p.a.list().await;
        assert_eq!(streams.len(), 1);
        let s = &streams[0];
        assert_eq!((s.id, s.listing_id), (id, Some(listing)));
        assert_eq!(
            (s.bitrate_kbps, s.segment_interval_ms, s.timeout_ms),
            (64, 1000, 0)
        );
        assert_eq!(s.network_key, p.key);
        assert!(s.segments > 0 && s.bytes_written >= s.segments * 8000);

        let m = p.b.ui.read_metrics(&ReadMetricsRequest {}).await.unwrap();
        let debug = m.metrics.unwrap().debug.unwrap();
        assert_eq!(debug.readers_started, 1);
        assert!(debug.segments_read > 0);
        assert_eq!(debug.readers.len(), 1);
        let read = &debug.readers[0];
        assert_eq!(read.listing_id, listing);
        assert_eq!(read.swarm_id, swarm.as_bytes());
        assert!(
            read.latency
                .is_some_and(|l| (0..1_000_000_000).contains(&l))
        );
        let wrote = p.a.ui.read_metrics(&ReadMetricsRequest {}).await.unwrap();
        let wrote = wrote.metrics.unwrap().debug.unwrap();
        assert_eq!(wrote.mock_streams_started, 1);
        assert!(wrote.segments_written > 0);
        assert_eq!(wrote.mock_streams.len(), 1);
    });
}

/// Enabling the config reads the mock streams already in the view.
#[test]
fn enabling_reads_the_listings_already_in_view() {
    run(async {
        let p = pair().await;
        let (_, listing) = p.listed_stream().await;
        settle().await;
        assert!(p.b.debug.readers().is_empty());

        p.b.set_config(true, &p.key).await.unwrap();
        wait_for(10, "a segment read", || p.b.reads(listing)).await;
    });
}

/// Stopping a mock stream unpublishes its listing, which ends its
/// readers; both nodes remove its swarm.
#[test]
fn stopping_a_mock_stream_ends_its_readers_and_swarms() {
    run(async {
        let p = pair().await;
        p.b.set_config(true, &p.key).await.unwrap();
        let (id, listing) = p.listed_stream().await;
        let swarm = p.a.swarm_of(id);
        wait_for(10, "a segment read", || p.b.reads(listing)).await;

        p.a.stop_stream(id).await;
        wait_for(10, "the stream to end", || {
            p.a.debug.mock_streams().is_empty()
        })
        .await;
        wait_for(10, "the reader to end", || p.b.debug.readers().is_empty()).await;
        assert!(p.a.list().await.is_empty());
        assert!(p.b.mock_listings(10).is_empty());
        let t = TransferId::new(&swarm, b"");
        for n in [&p.a.node, &p.b.node] {
            assert_eq!(n.host.swarms_of(&swarm), (0, 0));
            assert!(!n.transfers.list().contains(&t));
        }
        let m = p.a.debug.metrics();
        assert_eq!((m.mock_streams_started, m.mock_streams_ended), (1, 1));
        let m = p.b.debug.metrics();
        assert_eq!((m.readers_started, m.readers_ended), (1, 1));
    });
}

/// A mock stream ends at its timeout, having written a segment a second.
#[test]
fn a_mock_stream_ends_at_its_timeout() {
    run(async {
        let p = pair().await;
        let started = p.a.node.timer.now();
        let id = p.a.start_stream(&p.key, 5_500).await;
        let swarm = p.a.swarm_of(id);
        wait_for(10, "the stream to end", || {
            p.a.debug.mock_streams().is_empty()
        })
        .await;
        let ended = p.a.node.timer.now() - started;
        assert!(
            ended >= Duration::from_millis(5_500) && ended < Duration::from_secs(6),
            "{ended}"
        );
        // Segments at 0 s to 5 s.
        assert_eq!(p.a.debug.metrics().segments_written, 6);
        assert_eq!(p.a.node.host.swarms_of(&swarm), (0, 0));
        wait_for(10, "the listing to go", || p.b.mock_listings(10).is_empty()).await;
    });
}

/// Disabling the config closes the readers, and the stream goes on.
#[test]
fn disabling_closes_the_readers() {
    run(async {
        let p = pair().await;
        p.b.set_config(true, &p.key).await.unwrap();
        let (id, listing) = p.listed_stream().await;
        let swarm = p.a.swarm_of(id);
        wait_for(10, "a segment read", || p.b.reads(listing)).await;

        p.b.set_config(false, &p.key).await.unwrap();
        wait_for(10, "the reader to end", || p.b.debug.readers().is_empty()).await;
        assert_eq!(p.b.node.host.swarms_of(&swarm), (0, 0));
        settle().await;
        assert!(p.b.debug.readers().is_empty());
        assert_eq!(p.a.debug.mock_streams().len(), 1);

        p.b.set_config(true, &p.key).await.unwrap();
        wait_for(10, "a segment read again", || p.b.reads(listing)).await;
    });
}

/// A config naming another network closes the readers on the first and
/// reads the streams on the second.
#[test]
fn naming_another_network_moves_the_readers() {
    run(async {
        let world = World::default();
        let (a, b, c) = (
            node_in(&world, 1).await,
            node_in(&world, 2).await,
            node_in(&world, 3).await,
        );
        let (first, _ab) = owned_network(&a, &[&b], 10).await;
        let (second, _bc) = owned_network(&b, &[&c], 11).await;
        let (first, second) = (first.public, second.public);
        let a = Debugged::new(a, Config::default());
        let b = Debugged::new(b, Config::default());
        let c = Debugged::new(c, Config::default());
        let on_first = a.start_stream(&first, 0).await;
        let on_second = c.start_stream(&second, 0).await;
        wait_for(10, "the listings", || {
            a.listing_of(on_first).is_some() && c.listing_of(on_second).is_some()
        })
        .await;
        let (l1, l2) = (
            a.listing_of(on_first).unwrap(),
            c.listing_of(on_second).unwrap(),
        );

        b.set_config(true, &first).await.unwrap();
        wait_for(10, "a segment read on the first", || b.reads(l1)).await;
        assert_eq!(b.debug.readers().len(), 1);

        b.set_config(true, &second).await.unwrap();
        wait_for(10, "a segment read on the second", || {
            b.debug
                .readers()
                .iter()
                .all(|r| r.network_key == second && r.segments > 0)
                && !b.debug.readers().is_empty()
        })
        .await;
        let readers = b.debug.readers();
        assert_eq!(readers.len(), 1);
        assert_eq!(
            (readers[0].listing_id, readers[0].id),
            (l2, Some(on_second))
        );
        assert_eq!(b.node.host.swarms_of(&a.swarm_of(on_first)), (0, 0));
    });
}

/// Deleting the network the config names clears its key, which closes the
/// readers.
#[test]
fn deleting_the_network_clears_the_key_and_closes_the_readers() {
    run(async {
        let p = pair().await;
        p.b.set_config(true, &p.key).await.unwrap();
        let (id, listing) = p.listed_stream().await;
        let swarm = p.a.swarm_of(id);
        wait_for(10, "a segment read", || p.b.reads(listing)).await;

        p.b.delete_network(10).await;
        wait_for(10, "the reader to end", || p.b.debug.readers().is_empty()).await;
        let config = p.b.get_config().await;
        assert!(config.enable_mock_streams);
        assert!(config.mock_stream_network_key.is_empty());
        settle().await;
        assert_eq!(p.b.node.host.swarms_of(&swarm), (0, 0));
        assert_eq!(p.a.debug.mock_streams().len(), 1);
    });
}

/// Deleting its network ends a mock stream, taking its swarm down.
#[test]
fn deleting_its_network_ends_a_mock_stream() {
    run(async {
        let p = pair().await;
        let (id, _) = p.listed_stream().await;
        let swarm = p.a.swarm_of(id);

        p.a.delete_network(10).await;
        wait_for(10, "the stream to end", || {
            p.a.debug.mock_streams().is_empty()
        })
        .await;
        assert_eq!(p.a.node.host.swarms_of(&swarm), (0, 0));
        assert!(
            !p.a.node
                .transfers
                .list()
                .contains(&TransferId::new(&swarm, b""))
        );
        assert_eq!(p.a.debug.metrics().mock_streams_ended, 1);
    });
}

/// A node whose config names the network of its own mock stream doesn't
/// read it.
#[test]
fn a_node_does_not_read_its_own_mock_streams() {
    run(async {
        let p = pair().await;
        p.a.set_config(true, &p.key).await.unwrap();
        let (id, listing) = p.listed_stream().await;
        wait_for(10, "the listing in a's view", || {
            p.a.mock_listings(10).contains(&listing)
        })
        .await;
        settle().await;
        assert!(p.a.debug.readers().is_empty());
        assert_eq!(p.a.node.host.swarms_of(&p.a.swarm_of(id)), (1, 0));
        assert_eq!(p.a.debug.metrics().readers_started, 0);
    });
}

/// A node reads at most its bound of mock streams, the lowest listing ids
/// first, and reads another as one ends.
#[test]
fn readers_stop_at_their_bound() {
    run(async {
        let p = pair_with(Config {
            max_readers: 2,
            ..Config::default()
        })
        .await;
        p.b.set_config(true, &p.key).await.unwrap();
        let mut streams = Vec::new();
        for _ in 0..3 {
            streams.push(p.listed_stream().await);
        }
        streams.sort_by_key(|(_, listing)| *listing);
        let (first, second, third) = (streams[0], streams[1], streams[2]);
        wait_for(10, "two readers", || {
            p.b.reads(first.1) && p.b.reads(second.1)
        })
        .await;
        settle().await;
        assert_eq!(p.b.debug.readers().len(), 2);
        assert!(p.b.reader_of(third.1).is_none());

        p.a.stop_stream(first.0).await;
        wait_for(20, "the third read", || p.b.reads(third.1)).await;
        let read: Vec<u64> = p.b.debug.readers().iter().map(|r| r.listing_id).collect();
        assert_eq!(read, [second.1, third.1]);
    });
}

/// A node writes at most its bound of mock streams.
#[test]
fn mock_streams_stop_at_their_bound() {
    run(async {
        let p = pair().await;
        for _ in 0..4 {
            p.a.start_stream(&p.key, 0).await;
        }
        let req = StartMockStreamRequest {
            bitrate_kbps: 64,
            network_key: p.key.clone(),
            ..Default::default()
        };
        assert_eq!(
            refusal(p.a.ui.start_mock_stream(&req).await),
            "too many mock streams"
        );
        let ids: Vec<u64> = p.a.list().await.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), 4);
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
        assert!(!ids.contains(&0));
    });
}

/// Requests out of bounds, or on a network the profile lacks, are refused.
#[test]
fn start_requests_out_of_bounds_are_refused() {
    run(async {
        let p = pair().await;
        let req = |bitrate_kbps, segment_interval_ms, network_key: &[u8]| StartMockStreamRequest {
            bitrate_kbps,
            segment_interval_ms,
            timeout_ms: 0,
            network_key: network_key.to_vec(),
        };
        let ui = &p.a.ui;
        for (r, why) in [
            (req(50_001, 100, &p.key), "bitrate out of range"),
            (req(64, 50, &p.key), "segment interval out of range"),
            (
                req(50_000, 1000, &p.key),
                "segment too large for the bitrate and interval",
            ),
            (req(64, 1000, b"nope"), "network not found"),
            (req(64, 1000, b""), "network not found"),
        ] {
            assert_eq!(refusal(ui.start_mock_stream(&r).await), why);
        }
        assert!(p.a.debug.mock_streams().is_empty());
    });
}

/// Stopping an id no mock stream has does nothing.
#[test]
fn stopping_an_unknown_mock_stream_does_nothing() {
    run(async {
        let p = pair().await;
        let id = p.a.start_stream(&p.key, 0).await;
        p.a.stop_stream(id.wrapping_add(1)).await;
        p.a.stop_stream(0).await;
        settle().await;
        let ids: Vec<u64> = p.a.list().await.iter().map(|s| s.id).collect();
        assert_eq!(ids, [id]);
    });
}

/// A config's network key is empty or names one of the profile's
/// networks, and a request without a config stores the default.
#[test]
fn set_config_refuses_unknown_networks() {
    run(async {
        let p = pair().await;
        assert_eq!(p.b.get_config().await, DebugConfig::default());
        assert_eq!(
            refusal(p.b.set_config(true, b"nope").await),
            "network not found"
        );
        assert_eq!(p.b.get_config().await, DebugConfig::default());
        let on = p.b.set_config(true, &p.key).await.unwrap();
        assert_eq!(p.b.get_config().await, on);
        p.b.set_config(true, b"").await.unwrap();
        assert!(p.b.get_config().await.mock_stream_network_key.is_empty());
        let req = SetConfigRequest { config: None };
        p.b.ui.set_config(&req).await.unwrap();
        assert_eq!(p.b.get_config().await, DebugConfig::default());
    });
}

/// A source whose snapshots count up in `live_bytes`, or that fails from
/// its `fail_at`th call.
struct Counting {
    calls: Rc<Cell<u64>>,
    fail_at: u64,
}

impl MetricsSource for Counting {
    fn metrics(&self) -> impl Future<Output = Result<Metrics, RpcError>> {
        let n = self.calls.get() + 1;
        self.calls.set(n);
        let r = if n >= self.fail_at {
            Err(RpcError::new(0, "source failed"))
        } else {
            Ok(Metrics {
                memory: Some(MemoryMetrics {
                    live_bytes: n,
                    ..Default::default()
                }),
                ..Default::default()
            })
        };
        std::future::ready(r)
    }
}

/// A snapshot holds the source's metrics, the time and the debug
/// control's; a watch sends one at once, then one every clamped interval,
/// and ends with the source's error.
#[test]
fn watch_metrics_clamps_its_interval() {
    run(async {
        let world = World::default();
        let n = Debugged::new(node_in(&world, 1).await, Config::default());
        let calls = Rc::new(Cell::new(0));
        let ui = connect_ui_with(
            &n.debug,
            Counting {
                calls: calls.clone(),
                fail_at: u64::MAX,
            },
        );
        let now = n.node.timer.now();
        let m = ui.read_metrics(&ReadMetricsRequest {}).await.unwrap();
        let m = m.metrics.unwrap();
        assert_eq!(m.time, now.unix_nanos());
        assert_eq!(m.memory.unwrap().live_bytes, 1);
        assert!(m.debug.is_some());

        let ms = |n: i64| Duration::from_millis(n).as_nanos();
        for (interval_ms, gap) in [(1, ms(250)), (0, ms(1000)), (-3, ms(1000)), (700, ms(700))]
            .into_iter()
            .chain([(i32::MAX, ms(60_000))])
        {
            let start = n.node.timer.now().unix_nanos();
            let req = WatchMetricsRequest { interval_ms };
            let mut s = ui.watch_metrics(&req).await.unwrap();
            let mut times = Vec::new();
            for _ in 0..3 {
                let r = s.next().await.unwrap().unwrap();
                times.push(r.metrics.unwrap().time);
            }
            assert_eq!(times[0], start, "{interval_ms}");
            assert_eq!(times[1] - times[0], gap, "{interval_ms}");
            assert_eq!(times[2] - times[1], gap, "{interval_ms}");
        }

        let failing = connect_ui_with(
            &n.debug,
            Counting {
                calls: Rc::new(Cell::new(0)),
                fail_at: 3,
            },
        );
        let req = WatchMetricsRequest { interval_ms: 250 };
        let mut s = failing.watch_metrics(&req).await.unwrap();
        for live in [1, 2] {
            let r = s.next().await.unwrap().unwrap();
            assert_eq!(r.metrics.unwrap().memory.unwrap().live_bytes, live);
        }
        match s.next().await {
            Some(Err(CallError::Remote(e))) => assert_eq!(e.message, "source failed"),
            r => panic!("not the source's error: {r:?}"),
        }
        assert!(s.next().await.is_none());
    });
}
