// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Peer RPC between a Go and a Rust node over the vnic channel pair on
//! ports 4 and 5: Go's `rpc.Client`/`rpc.Server` on `RWFDialer`s, and
//! `strims_rpc::peer_connection` on `Node::take_peer_rpc`. Both serve the
//! test Debug service of `crates/strims-rpc/golden/debugsvc`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration as StdDuration;

use futures_util::{StreamExt, stream};
use strims_proto::debug::v1::{
    DebugClient, DebugService, GetConfigRequest, GetConfigResponse, PProfRequest, PProfResponse,
    ReadMetricsRequest, ReadMetricsResponse, StartMockStreamRequest, StartMockStreamResponse,
    WatchMetricsRequest, WatchMetricsResponse, register_debug_service,
};
use strims_proto::service::{
    CallError, Caller, Context as CallContext, Error, Method, ServerStream, UnaryMethod,
};
use strims_rpc::{Client, Config, Router, peer_connection};
use tokio::task::{LocalSet, spawn_local};

use crate::harness::{GoNode, GoOptions, RustNode, RustOptions, TestNode, Transport, helper};

/// The test service, as `debugsvc` serves it. It also records the
/// profile keys of callers whose certificate the transport verified.
#[derive(Default)]
pub(crate) struct TestService {
    pub(crate) cancelled: Rc<Cell<u64>>,
    pub(crate) callers: Rc<RefCell<Vec<Vec<u8>>>>,
}

struct CancelGuard(Rc<Cell<u64>>);

impl Drop for CancelGuard {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

pub(crate) fn data(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i * 7) as u8).collect()
}

impl DebugService for TestService {
    async fn p_prof(&self, ctx: &CallContext, req: PProfRequest) -> Result<PProfResponse, Error> {
        if let Some(member) = ctx
            .certificate()
            .and_then(strims_crypto::certificate_parent)
        {
            self.callers.borrow_mut().push(member.key.clone());
        }
        let name = if req.name.len() > 64 {
            req.name.len().to_string()
        } else {
            req.name.clone()
        };
        Ok(PProfResponse {
            name,
            data: data(req.name.parse().unwrap_or(0)),
        })
    }

    async fn read_metrics(
        &self,
        _: &CallContext,
        _: ReadMetricsRequest,
    ) -> Result<ReadMetricsResponse, Error> {
        let _guard = CancelGuard(self.cancelled.clone());
        std::future::pending().await
    }

    async fn watch_metrics(
        &self,
        _: &CallContext,
        req: WatchMetricsRequest,
    ) -> Result<ServerStream<WatchMetricsResponse>, Error> {
        let item = |i: i32| WatchMetricsResponse {
            data: vec![i as u8],
        };
        match req.interval_ms {
            n if n < 0 => Err(Error::new(3, "negative interval")),
            0 => {
                let guard = CancelGuard(self.cancelled.clone());
                Ok(Box::pin(stream::unfold(
                    (guard, 0),
                    move |(g, i)| async move {
                        tokio::time::sleep(StdDuration::from_millis(1)).await;
                        Some((Ok(item(i)), (g, i + 1)))
                    },
                )))
            }
            n => Ok(Box::pin(stream::iter((0..n).map(move |i| Ok(item(i)))))),
        }
    }

    async fn get_config(
        &self,
        _: &CallContext,
        _: GetConfigRequest,
    ) -> Result<GetConfigResponse, Error> {
        Err(Error::new(7, "no config"))
    }

    async fn start_mock_stream(
        &self,
        _: &CallContext,
        _: StartMockStreamRequest,
    ) -> Result<StartMockStreamResponse, Error> {
        Ok(StartMockStreamResponse {
            id: self.cancelled.get(),
        })
    }
}

/// A method neither service has.
pub(crate) struct Nope;

impl Method for Nope {
    const NAME: &'static str = "strims.debug.v1.Debug.Nope";
    type Request = PProfRequest;
    type Response = PProfResponse;
}

impl UnaryMethod for Nope {}

pub(crate) fn pprof(name: impl Into<String>) -> PProfRequest {
    PProfRequest {
        name: name.into(),
        ..Default::default()
    }
}

pub(crate) fn watch(interval_ms: i32) -> WatchMetricsRequest {
    WatchMetricsRequest {
        interval_ms,
        ..Default::default()
    }
}

/// Polls `count` until it reaches `n`.
pub(crate) async fn wait_count(what: &str, n: u64, mut count: impl AsyncFnMut() -> u64) {
    for _ in 0..500 {
        if count().await >= n {
            return;
        }
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    }
    panic!("{what}: fewer than {n} cancels");
}

/// The Rust node calls the Go node's service.
async fn rust_calls_go(client: Client) {
    let debug = DebugClient::new(client.clone());

    let res = debug.p_prof(&pprof("300000")).await.unwrap();
    assert!(res.data == data(300_000), "a response of several frames");
    let res = debug.p_prof(&pprof("y".repeat(300_000))).await.unwrap();
    assert_eq!(res.name, "300000", "a request of several frames");

    let items: Vec<_> = debug
        .watch_metrics(&watch(5))
        .await
        .unwrap()
        .collect()
        .await;
    let items: Vec<Vec<u8>> = items.into_iter().map(|i| i.unwrap().data).collect();
    assert_eq!(items, [[0], [1], [2], [3], [4]]);

    let go_cancels = async || {
        debug
            .start_mock_stream(&StartMockStreamRequest::default())
            .await
            .unwrap()
            .id
    };
    let mut endless = debug.watch_metrics(&watch(0)).await.unwrap();
    for _ in 0..3 {
        endless.next().await.unwrap().unwrap();
    }
    drop(endless);
    wait_count("go", 1, go_cancels).await;
    let read = debug.read_metrics(&ReadMetricsRequest::default());
    assert!(
        tokio::time::timeout(StdDuration::from_millis(200), read)
            .await
            .is_err()
    );
    wait_count("go", 2, go_cancels).await;

    assert_eq!(
        debug.get_config(&GetConfigRequest::default()).await,
        Err(CallError::Remote(Error::new(7, "no config")))
    );
    let mut negative = debug.watch_metrics(&watch(-1)).await.unwrap();
    assert_eq!(
        negative.next().await,
        Some(Err(CallError::Remote(Error::new(3, "negative interval"))))
    );
    assert_eq!(
        client.call_unary::<Nope>(&pprof("")).await,
        Err(CallError::Remote(Error::method_not_found(Nope::NAME)))
    );
}

/// The Go node calls the Rust node's service.
async fn go_calls_rust(g: &mut GoNode, r: &str, cancelled: &Rc<Cell<u64>>) {
    let mut cmd = async |line: String| g.cmd(&line).await.unwrap();
    assert_eq!(
        cmd(format!("rpc-pprof {r} 300000 0")).await,
        "300000 300000 ok"
    );
    assert_eq!(cmd(format!("rpc-pprof {r} 0 300000")).await, "300000 0 ok");
    assert_eq!(cmd(format!("rpc-watch {r} 5")).await, "5 0001020304");
    assert_eq!(cmd(format!("rpc-watch-cancel {r} 3")).await, "canceled");
    wait_count("rust", 1, async || cancelled.get()).await;
    assert_eq!(cmd(format!("rpc-read-deadline {r} 200")).await, "deadline");
    wait_count("rust", 2, async || cancelled.get()).await;
    assert_eq!(cmd(format!("rpc-cancels {r}")).await, "2");

    let err = |code: i32, message: &str| format!("{code} {}", hex::encode(message));
    assert_eq!(cmd(format!("rpc-config {r}")).await, err(7, "no config"));
    assert_eq!(
        cmd(format!("rpc-negative {r}")).await,
        err(3, "negative interval")
    );
    assert_eq!(
        cmd(format!("rpc-raw {r} {}", Nope::NAME)).await,
        err(0, &format!("method not found: {}", Nope::NAME))
    );
}

async fn peer_rpc(t: Transport, seed: u8) {
    let Some(bin) = helper() else {
        return;
    };
    let mut o = GoOptions::new(seed);
    o.peer_rpc = true;
    let mut g = TestNode::Go(GoNode::spawn(&bin, &format!("g{seed}"), o).await);
    let mut r =
        TestNode::Rust(RustNode::spawn(&format!("r{seed}"), RustOptions::new(seed + 1)).await);
    let (g_id, r_id) = (g.host_id(), r.host_id());
    assert_eq!(r.dial(&g.uri(t)).await, g_id);
    r.wait_peer(g_id).await;
    g.wait_peer(r_id).await;

    let rpc = r.rust().node.take_peer_rpc(g_id).await.unwrap();
    let service = Rc::new(TestService::default());
    let cancelled = service.cancelled.clone();
    LocalSet::new()
        .run_until(async move {
            let mut router = Router::new();
            register_debug_service(&mut router, service);
            let (client, conn) = peer_connection(
                Config::peer(),
                (rpc.client.reader, rpc.client.writer),
                (rpc.server.reader, rpc.server.writer),
                router,
            );
            let conn = spawn_local(conn);
            rust_calls_go(client).await;
            go_calls_rust(g.go(), &hex::encode(r_id.to_bytes()), &cancelled).await;
            conn.abort();
        })
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn peer_rpc_both_ways() {
    crate::bounded(180, async {
        peer_rpc(Transport::Tcp, 0x70).await;
        peer_rpc(Transport::Ws, 0x72).await;
    })
    .await;
}
