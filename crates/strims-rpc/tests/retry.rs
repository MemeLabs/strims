// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Calls through a `RetryClient` over in-memory links, on tokio's paused
//! clock: attempts, backoff, and the progress that holds a deadline back.

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration as StdDuration;

use futures_core::Stream;
use futures_sink::Sink;
use futures_util::StreamExt;
use strims_base::time::{Duration, Time};
use strims_proto::rpc::fixture::{
    EchoRequest, EchoResponse, FixtureClient, FixtureService, GetRequest, GetResponse,
    WatchRequest, WatchResponse, register_fixture_service,
};
use strims_proto::service::{CallError, Context as CallContext, Error, ServerStream};
use strims_rpc::{Client, Config, RetryClient, RetryPolicy, Router, connect};
use tokio::task::{LocalSet, spawn_local};
use tokio::time::{Instant, Sleep, sleep, timeout};

mod common;
use common::TokioTimer;

// --- links ----------------------------------------------------------------------

/// How long a throttled link takes to carry its rate.
const TICK: StdDuration = StdDuration::from_millis(10);

#[derive(Debug, PartialEq, Eq)]
struct LinkClosed;

#[derive(Default)]
struct LinkState {
    queue: VecDeque<Vec<u8>>,
    reader: Option<Waker>,
    writer: Option<Waker>,
    /// Bytes carried per tick; everything at once if unset.
    rate: Option<usize>,
    /// While set, the writer takes nothing.
    stalled: bool,
    /// Calls written.
    written: usize,
}

/// One direction of an in-memory transport.
#[derive(Clone, Default)]
struct Link(Rc<RefCell<LinkState>>);

impl Link {
    fn throttle(&self, rate: usize) {
        self.0.borrow_mut().rate = Some(rate);
    }

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

    fn written(&self) -> usize {
        self.0.borrow().written
    }
}

struct Reader {
    link: Link,
    /// The next tick of a throttled link.
    tick: Option<Pin<Box<Sleep>>>,
}

impl Stream for Reader {
    type Item = Result<Vec<u8>, LinkClosed>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(tick) = self.tick.as_mut() {
            std::task::ready!(tick.as_mut().poll(cx));
            self.tick = None;
        }
        let (bytes, throttled) = {
            let mut s = self.link.0.borrow_mut();
            let s = &mut *s;
            let Some(front) = s.queue.front_mut() else {
                s.reader = Some(cx.waker().clone());
                return Poll::Pending;
            };
            let bytes = match s.rate {
                Some(rate) if front.len() > rate => front.drain(..rate).collect(),
                _ => s.queue.pop_front().unwrap(),
            };
            (bytes, s.rate.is_some())
        };
        if throttled {
            self.tick = Some(Box::pin(sleep(TICK)));
        }
        Poll::Ready(Some(Ok(bytes)))
    }
}

struct Writer(Link);

impl Sink<Vec<u8>> for Writer {
    type Error = LinkClosed;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), LinkClosed>> {
        let mut s = self.0.0.borrow_mut();
        if s.stalled {
            s.writer = Some(cx.waker().clone());
            return Poll::Pending;
        }
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), LinkClosed> {
        let r = {
            let mut s = self.0.0.borrow_mut();
            s.queue.push_back(item);
            s.written += 1;
            s.reader.take()
        };
        if let Some(r) = r {
            r.wake();
        }
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), LinkClosed>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), LinkClosed>> {
        Poll::Ready(Ok(()))
    }
}

// --- the service under test --------------------------------------------------------

#[derive(Default)]
struct TestService {
    /// Requests the handlers have seen, stamped with the time since `start`.
    requests: RefCell<Vec<StdDuration>>,
    /// Handlers dropped before they answered.
    cancelled: Rc<Cell<usize>>,
    /// The next this many `echo` calls never answer.
    hang: Cell<usize>,
    /// The bytes of each `echo` answer.
    answer_len: Cell<usize>,
    start: Cell<Option<Instant>>,
}

impl TestService {
    fn seen(&self) {
        let since = self.start.get().map(|s| s.elapsed()).unwrap_or_default();
        self.requests.borrow_mut().push(since);
    }

    fn requests(&self) -> usize {
        self.requests.borrow().len()
    }
}

/// Counts a handler dropped before it finished.
struct Unfinished(Option<Rc<Cell<usize>>>);

impl Drop for Unfinished {
    fn drop(&mut self) {
        if let Some(c) = self.0.take() {
            c.set(c.get() + 1);
        }
    }
}

impl FixtureService for TestService {
    async fn echo(&self, ctx: &CallContext, req: EchoRequest) -> Result<EchoResponse, Error> {
        self.seen();
        let mut guard = Unfinished(Some(self.cancelled.clone()));
        if self.hang.get() > 0 {
            self.hang.set(self.hang.get() - 1);
            std::future::pending::<()>().await;
        }
        guard.0 = None;
        Ok(EchoResponse {
            name: format!("{}@{}", req.name, ctx.id()),
            data: vec![7; self.answer_len.get()],
        })
    }

    async fn watch(
        &self,
        _ctx: &CallContext,
        _req: WatchRequest,
    ) -> Result<ServerStream<WatchResponse>, Error> {
        self.seen();
        let items = (0..3u8).map(|n| Ok(WatchResponse { data: vec![n] }));
        Ok(Box::pin(futures_util::stream::iter(items)))
    }

    async fn get(&self, _ctx: &CallContext, _req: GetRequest) -> Result<GetResponse, Error> {
        self.seen();
        Err(Error::new(7, "no config"))
    }
}

struct World {
    service: Rc<TestService>,
    fixture: FixtureClient<RetryClient<TokioTimer>>,
    /// The client's Calls to the server.
    up: Link,
    /// The server's Calls to the client.
    down: Link,
}

/// A client calling a [`TestService`] over a fresh pair of links with
/// `policy`. Must run on a `LocalSet`.
fn world(policy: RetryPolicy) -> World {
    let (up, down) = (Link::default(), Link::default());
    let service = Rc::new(TestService::default());
    service.start.set(Some(Instant::now()));
    let mut router = Router::new();
    register_fixture_service(&mut router, service.clone());
    let reader = |link: &Link| Reader {
        link: link.clone(),
        tick: None,
    };
    let client_config = Config {
        serve: false,
        ..Config::peer()
    };
    let (client, conn) = connect(client_config, reader(&down), Writer(up.clone()), None);
    let (_, server) = connect(
        Config::peer(),
        reader(&up),
        Writer(down.clone()),
        Some(router),
    );
    spawn_local(async move {
        let _ = conn.await;
    });
    spawn_local(async move {
        let _ = server.await;
    });
    World {
        service,
        fixture: FixtureClient::new(retrying(client, policy)),
        up,
        down,
    }
}

fn retrying(client: Client, policy: RetryPolicy) -> RetryClient<TokioTimer> {
    RetryClient::new(client, TokioTimer::new(Time::EPOCH), policy)
}

/// Runs `f` on a local task set with a timeout.
async fn local<F: Future>(f: F) -> F::Output {
    LocalSet::new()
        .run_until(timeout(StdDuration::from_secs(600), f))
        .await
        .expect("test timed out")
}

/// Polls `cond` once a millisecond until it holds.
async fn until(mut cond: impl FnMut() -> bool) {
    while !cond() {
        sleep(StdDuration::from_millis(1)).await;
    }
}

fn echo(name: &str) -> EchoRequest {
    EchoRequest {
        name: name.into(),
        ..Default::default()
    }
}

fn ms(n: u64) -> StdDuration {
    StdDuration::from_millis(n)
}

// --- tests --------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn a_call_that_times_out_is_retried_after_the_delay() {
    local(async {
        let w = world(RetryPolicy::peer());
        w.service.hang.set(1);
        let start = Instant::now();
        let res = w.fixture.echo(&echo("heap")).await.unwrap();
        // the first attempt's request was id 1, its cancel id 2
        assert_eq!(res.name, "heap@3");
        assert_eq!(start.elapsed(), ms(1100));
        assert_eq!(*w.service.requests.borrow(), [ms(0), ms(1100)]);
        assert_eq!(w.service.cancelled.get(), 1);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn retries_back_off_then_the_call_times_out() {
    local(async {
        let w = world(RetryPolicy::peer());
        w.service.hang.set(usize::MAX);
        let start = Instant::now();
        let res = w.fixture.echo(&echo("heap")).await;
        assert_eq!(res, Err(CallError::Timeout));
        // 1 s attempts, 100 ms delay doubling
        assert_eq!(start.elapsed(), ms(4700));
        assert_eq!(
            *w.service.requests.borrow(),
            [ms(0), ms(1100), ms(2300), ms(3700)]
        );
        until(|| w.service.cancelled.get() == 4).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn remote_errors_are_not_retried() {
    local(async {
        let w = world(RetryPolicy::peer());
        let res = w.fixture.get(&GetRequest::default()).await;
        assert_eq!(res, Err(CallError::Remote(Error::new(7, "no config"))));
        sleep(StdDuration::from_secs(10)).await;
        assert_eq!(w.service.requests(), 1);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_large_answer_still_arriving_is_not_requested_again() {
    local(async {
        let w = world(RetryPolicy::peer());
        // 100 KB/s: the answer takes 5 s, five attempt timeouts
        w.down.throttle(1000);
        w.service.answer_len.set(500_000);
        let start = Instant::now();
        let res = w.fixture.echo(&echo("heap")).await.unwrap();
        assert_eq!(res.data.len(), 500_000);
        assert!(start.elapsed() >= StdDuration::from_secs(5));
        assert_eq!(w.service.requests(), 1);
        assert_eq!(w.up.written(), 1);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_request_still_queued_is_not_sent_again() {
    local(async {
        let w = world(RetryPolicy::peer());
        w.up.stall(true);
        let up = w.up.clone();
        spawn_local(async move {
            sleep(StdDuration::from_secs(3)).await;
            up.stall(false);
        });
        let start = Instant::now();
        w.fixture.echo(&echo("heap")).await.unwrap();
        assert_eq!(start.elapsed(), ms(3000));
        assert_eq!(w.service.requests(), 1);
        assert_eq!(w.up.written(), 1);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_deadline_caps_an_answer_that_keeps_arriving() {
    local(async {
        let w = world(RetryPolicy {
            deadline: Duration::from_secs(3),
            ..RetryPolicy::peer()
        });
        // 10 KB/s: the answer would take 10 s
        w.down.throttle(100);
        w.service.answer_len.set(100_000);
        let start = Instant::now();
        let res = w.fixture.echo(&echo("heap")).await;
        assert_eq!(res, Err(CallError::Timeout));
        assert_eq!(start.elapsed(), ms(3000));
        assert_eq!(w.service.requests(), 1);
        // the request and its cancel
        until(|| w.up.written() == 2).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn dropping_a_retrying_call_cancels_its_attempt() {
    local(async {
        let w = world(RetryPolicy::peer());
        w.service.hang.set(usize::MAX);
        // the second attempt starts at 1.1 s
        let res = timeout(ms(1500), w.fixture.echo(&echo("heap"))).await;
        assert!(res.is_err());
        until(|| w.service.cancelled.get() == 2).await;
        sleep(StdDuration::from_secs(10)).await;
        assert_eq!(w.service.requests(), 2);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn streams_have_no_deadline() {
    local(async {
        let w = world(RetryPolicy::peer());
        let stream = w.fixture.watch(&WatchRequest::default()).await.unwrap();
        sleep(StdDuration::from_secs(10)).await;
        let items: Vec<_> = stream.map(|r| r.unwrap().data).collect().await;
        assert_eq!(items, [vec![0], vec![1], vec![2]]);
        assert_eq!(w.service.requests(), 1);
    })
    .await;
}
