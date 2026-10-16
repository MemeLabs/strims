// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Connections over in-memory pipes, driven by a real executor (tokio's
//! current-thread runtime with paused time), so a lost wakeup shows up as a
//! timeout.

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use futures_core::Stream;
use futures_sink::Sink;
use futures_util::StreamExt;
use strims_proto::debug::v1::{
    DebugClient, DebugService, GetConfigRequest, GetConfigResponse, PProfRequest, PProfResponse,
    ReadMetricsRequest, ReadMetricsResponse, WatchMetricsRequest, WatchMetricsResponse,
    register_debug_service,
};
use strims_proto::service::{CallError, Context as CallContext, Error, ServerStream};
use strims_rpc::{Client, Config, Router, connect, peer_connection};
use tokio::task::{JoinHandle, LocalSet, spawn_local};
use tokio::time::timeout;

// --- pipes ------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct PipeClosed;

#[derive(Default)]
struct PipeState {
    queue: VecDeque<Vec<u8>>,
    reader: Option<Waker>,
    writer: Option<Waker>,
    /// The writer is gone: the reader sees the end once the queue drains.
    closed: bool,
    /// While set, the writer takes nothing.
    stalled: bool,
}

/// One direction of an in-memory transport.
#[derive(Clone, Default)]
struct Pipe(Rc<RefCell<PipeState>>);

impl Pipe {
    fn stall(&self, stalled: bool) {
        let w = {
            let mut s = self.0.borrow_mut();
            s.stalled = stalled;
            s.writer.take()
        };
        if let Some(w) = w {
            w.wake();
        }
    }

    fn close(&self) {
        let r = {
            let mut s = self.0.borrow_mut();
            s.closed = true;
            s.reader.take()
        };
        if let Some(r) = r {
            r.wake();
        }
    }
}

struct Reader(Pipe);
struct Writer(Pipe);

impl Stream for Reader {
    type Item = Result<Vec<u8>, PipeClosed>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut s = self.0.0.borrow_mut();
        if let Some(b) = s.queue.pop_front() {
            return Poll::Ready(Some(Ok(b)));
        }
        if s.closed {
            return Poll::Ready(None);
        }
        s.reader = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Sink<Vec<u8>> for Writer {
    type Error = PipeClosed;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), PipeClosed>> {
        let mut s = self.0.0.borrow_mut();
        if s.closed {
            return Poll::Ready(Err(PipeClosed));
        }
        if s.stalled {
            s.writer = Some(cx.waker().clone());
            return Poll::Pending;
        }
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), PipeClosed> {
        let r = {
            let mut s = self.0.0.borrow_mut();
            // split every Call to exercise reassembly
            let mid = item.len() / 2;
            s.queue.push_back(item[..mid].to_vec());
            s.queue.push_back(item[mid..].to_vec());
            s.reader.take()
        };
        if let Some(r) = r {
            r.wake();
        }
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), PipeClosed>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), PipeClosed>> {
        self.0.close();
        Poll::Ready(Ok(()))
    }
}

/// Two ends of a duplex transport, and the pipe each end writes into.
fn duplex() -> ((Reader, Writer), (Reader, Writer), Pipe, Pipe) {
    let (a_to_b, b_to_a) = (Pipe::default(), Pipe::default());
    (
        (Reader(b_to_a.clone()), Writer(a_to_b.clone())),
        (Reader(a_to_b.clone()), Writer(b_to_a.clone())),
        a_to_b,
        b_to_a,
    )
}

/// Runs `f` on a local task set with a timeout.
async fn local<F: Future>(f: F) -> F::Output {
    LocalSet::new()
        .run_until(timeout(Duration::from_secs(60), f))
        .await
        .expect("test timed out")
}

/// Polls `cond` once a millisecond until it holds.
async fn until(mut cond: impl FnMut() -> bool) {
    while !cond() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

/// Serves `router` on one end of a fresh duplex and returns a client of
/// the other end, with both connections' tasks.
fn serve(router: Router, config: Config) -> (Client, JoinHandle<()>, JoinHandle<()>, Pipe, Pipe) {
    let (a, b, a_to_b, b_to_a) = duplex();
    let client_config = Config {
        serve: false,
        ..config.clone()
    };
    let (client, conn) = connect(client_config, a.0, a.1, None);
    let (_, server) = connect(config, b.0, b.1, Some(router));
    let ct = spawn_local(async move {
        let _ = conn.await;
    });
    let st = spawn_local(async move {
        let _ = server.await;
    });
    (client, ct, st, a_to_b, b_to_a)
}

// --- the service under test ----------------------------------------------------

#[derive(Default)]
struct TestService {
    /// Items the last stream produced.
    pulled: Rc<Cell<usize>>,
    /// Set when the last stream is dropped.
    stream_dropped: Rc<Cell<bool>>,
    /// A client this service calls while handling `read_metrics`.
    nested: RefCell<Option<Client>>,
}

/// Counts pulls and records its drop.
struct Counted {
    remaining: Option<usize>,
    pulled: Rc<Cell<usize>>,
    dropped: Rc<Cell<bool>>,
}

impl Stream for Counted {
    type Item = Result<WatchMetricsResponse, Error>;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(r) = &mut self.remaining {
            if *r == 0 {
                return Poll::Ready(None);
            }
            *r -= 1;
        }
        let n = self.pulled.get();
        self.pulled.set(n + 1);
        Poll::Ready(Some(Ok(WatchMetricsResponse {
            data: (n as u32).to_be_bytes().to_vec(),
        })))
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.dropped.set(true);
    }
}

impl DebugService for TestService {
    async fn p_prof(&self, ctx: &CallContext, req: PProfRequest) -> Result<PProfResponse, Error> {
        Ok(PProfResponse {
            name: format!("{}@{}", req.name, ctx.id()),
            data: vec![7; usize::from(req.debug) * 1000],
        })
    }

    async fn read_metrics(
        &self,
        _ctx: &CallContext,
        _req: ReadMetricsRequest,
    ) -> Result<ReadMetricsResponse, Error> {
        let client = self.nested.borrow().clone();
        if let Some(client) = client {
            // start a call on another connection and drop it unfinished
            let debug = DebugClient::new(client);
            let stream = debug
                .watch_metrics(&WatchMetricsRequest::default())
                .await
                .map_err(|e| Error::new(1, e.to_string()))?;
            drop(stream);
        }
        Ok(ReadMetricsResponse { data: vec![1] })
    }

    async fn watch_metrics(
        &self,
        _ctx: &CallContext,
        req: WatchMetricsRequest,
    ) -> Result<ServerStream<WatchMetricsResponse>, Error> {
        if req.interval_ms < 0 {
            return Err(Error::new(3, "negative interval"));
        }
        self.pulled.set(0);
        self.stream_dropped.set(false);
        Ok(Box::pin(Counted {
            remaining: (req.interval_ms > 0).then_some(req.interval_ms as usize),
            pulled: self.pulled.clone(),
            dropped: self.stream_dropped.clone(),
        }))
    }

    async fn get_config(
        &self,
        _ctx: &CallContext,
        _req: GetConfigRequest,
    ) -> Result<GetConfigResponse, Error> {
        Err(Error::new(7, "no config"))
    }
}

fn router_for(service: Rc<TestService>) -> Router {
    let mut router = Router::new();
    register_debug_service(&mut router, service);
    router
}

// --- tests --------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn unary_calls_and_errors_reach_the_caller() {
    local(async {
        let (client, ..) = serve(router_for(Rc::default()), Config::default());
        let debug = DebugClient::new(client);

        let res = debug
            .p_prof(&PProfRequest {
                name: "heap".into(),
                debug: true,
                gc: false,
            })
            .await
            .unwrap();
        assert_eq!(res.name, "heap@1");
        assert_eq!(res.data.len(), 1000);

        assert_eq!(
            debug.get_config(&GetConfigRequest::default()).await,
            Err(CallError::Remote(Error::new(7, "no config")))
        );
        // a method the service leaves to the default
        assert_eq!(
            debug.set_config(&Default::default()).await,
            Err(CallError::Remote(Error::unimplemented()))
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn streams_deliver_every_item_then_end() {
    local(async {
        let (client, ..) = serve(router_for(Rc::default()), Config::default());
        let debug = DebugClient::new(client);
        let items: Vec<_> = debug
            .watch_metrics(&WatchMetricsRequest {
                interval_ms: 50,
                ..Default::default()
            })
            .await
            .unwrap()
            .collect()
            .await;
        assert_eq!(items.len(), 50);
        for (i, item) in items.into_iter().enumerate() {
            assert_eq!(item.unwrap().data, (i as u32).to_be_bytes());
        }

        let mut failed = debug
            .watch_metrics(&WatchMetricsRequest {
                interval_ms: -1,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(
            failed.next().await,
            Some(Err(CallError::Remote(Error::new(3, "negative interval"))))
        );
        assert_eq!(failed.next().await, None);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn dropping_a_stream_drops_its_handler() {
    local(async {
        let service = Rc::new(TestService::default());
        let (client, ..) = serve(router_for(service.clone()), Config::default());
        let debug = DebugClient::new(client);
        // endless
        let mut stream = debug
            .watch_metrics(&WatchMetricsRequest::default())
            .await
            .unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        drop(stream);
        until(|| service.stream_dropped.get()).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn unregistered_methods_are_not_found_until_registered() {
    local(async {
        let mut router = Router::new();
        let (client, ..) = serve(router.clone(), Config::default());
        let debug = DebugClient::new(client);
        assert_eq!(
            debug.p_prof(&PProfRequest::default()).await,
            Err(CallError::Remote(Error::method_not_found(
                "strims.debug.v1.Debug.PProf"
            )))
        );
        register_debug_service(&mut router, Rc::new(TestService::default()));
        assert!(debug.p_prof(&PProfRequest::default()).await.is_ok());
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_end_of_input_fails_open_calls() {
    local(async {
        let (a, b, _, b_to_a) = duplex();
        let (client, conn) = connect(Config::default(), a.0, a.1, None);
        let conn = spawn_local(conn);
        // nobody serves the other end
        let pending = DebugClient::new(client.clone());
        let call = spawn_local(async move { pending.p_prof(&PProfRequest::default()).await });
        tokio::time::sleep(Duration::from_millis(10)).await;
        drop(b);
        b_to_a.close();
        assert!(conn.await.unwrap().is_ok());
        assert_eq!(call.await.unwrap(), Err(CallError::Disconnected));
        // and later calls fail at once
        assert_eq!(
            DebugClient::new(client)
                .p_prof(&PProfRequest::default())
                .await,
            Err(CallError::Disconnected)
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn dropping_the_connection_fails_open_calls() {
    local(async {
        let (a, _b, ..) = duplex();
        let (client, conn) = connect(Config::default(), a.0, a.1, None);
        let conn = spawn_local(async move {
            let _ = conn.await;
        });
        let debug = DebugClient::new(client);
        let call = spawn_local(async move { debug.p_prof(&PProfRequest::default()).await });
        tokio::time::sleep(Duration::from_millis(10)).await;
        conn.abort();
        assert_eq!(call.await.unwrap(), Err(CallError::Disconnected));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_stalled_transport_stops_pulling_server_streams() {
    local(async {
        let service = Rc::new(TestService::default());
        let config = Config {
            transmit_high_water: 1000,
            ..Config::default()
        };
        let (client, _, _, _, b_to_a) = serve(router_for(service.clone()), config);
        b_to_a.stall(true);
        let debug = DebugClient::new(client);
        let stream = debug
            .watch_metrics(&WatchMetricsRequest {
                interval_ms: 5000,
                ..Default::default()
            })
            .await
            .unwrap();
        // the server fills its transmit queue to the high water mark
        until(|| service.pulled.get() > 0).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        let pulled = service.pulled.get();
        // items encode to about 14 bytes with their prefix
        assert!(
            pulled < 100,
            "pulled {pulled} items into a stalled transport"
        );

        b_to_a.stall(false);
        let items: Vec<_> = stream.collect().await;
        assert_eq!(items.len(), 5000);
        assert!(items.iter().all(Result::is_ok));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn peers_call_each_other_over_channel_pairs() {
    local(async {
        // a's client channel is b's server channel, and the reverse
        let (a_client, b_server, ..) = duplex();
        let (b_client, a_server, ..) = duplex();
        let (a, a_conn) = peer_connection(
            Config::peer(),
            a_client,
            a_server,
            router_for(Rc::default()),
        );
        let (b, b_conn) = peer_connection(
            Config::peer(),
            b_client,
            b_server,
            router_for(Rc::default()),
        );
        spawn_local(a_conn);
        spawn_local(b_conn);
        let from_a = DebugClient::new(a)
            .p_prof(&PProfRequest {
                name: "a".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        let from_b = DebugClient::new(b)
            .p_prof(&PProfRequest {
                name: "b".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!((from_a.name.as_str(), from_b.name.as_str()), ("a@1", "b@1"));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn handlers_can_start_and_drop_calls_on_connections() {
    local(async {
        // the handler calls back over its own transport's other connection
        let service = Rc::new(TestService::default());
        let (a, b, ..) = duplex();
        let (a_client, a_conn) = connect(
            Config::default(),
            a.0,
            a.1,
            Some(router_for(service.clone())),
        );
        let (b_client, b_conn) = connect(
            Config::default(),
            b.0,
            b.1,
            Some(router_for(service.clone())),
        );
        *service.nested.borrow_mut() = Some(b_client);
        spawn_local(a_conn);
        spawn_local(b_conn);
        let res = DebugClient::new(a_client)
            .read_metrics(&ReadMetricsRequest::default())
            .await
            .unwrap();
        assert_eq!(res.data, [1]);
        // the dropped nested stream was cancelled at its server
        until(|| service.stream_dropped.get()).await;
    })
    .await;
}
