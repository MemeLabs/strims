// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The generated RPC interfaces of `strims.debug.v1.Debug`, driven by a
//! registry that records handlers and a caller that records calls.

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::{Pin, pin};
use std::rc::Rc;
use std::task::{self, Poll, Waker};

use futures_core::Stream;
use prost::Message;
use strims_proto::debug::v1::{
    DebugClient, DebugService, GetConfigRequest, PProfRequest, PProfResponse, ReadMetricsRequest,
    WatchMetricsRequest, WatchMetricsResponse, debug_methods, register_debug_service,
};
use strims_proto::service::{
    CallError, Caller, Context, Error, Registry, ServerStream, StreamingMethod, UnaryMethod,
};

/// Polls `f` once; every future in these tests is ready immediately.
fn ready<F: Future>(f: F) -> F::Output {
    let mut cx = task::Context::from_waker(Waker::noop());
    match pin!(f).poll(&mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => panic!("future is pending"),
    }
}

/// Drains a stream whose items are all ready.
fn collect<S: Stream + Unpin>(mut s: S) -> Vec<S::Item> {
    let mut cx = task::Context::from_waker(Waker::noop());
    let mut out = Vec::new();
    loop {
        match Pin::new(&mut s).poll_next(&mut cx) {
            Poll::Ready(Some(v)) => out.push(v),
            Poll::Ready(None) => return out,
            Poll::Pending => panic!("stream is pending"),
        }
    }
}

type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;
type Unary = Box<dyn Fn(Context, Vec<u8>) -> BoxFuture<Result<Vec<u8>, Error>>>;
type Streaming = Box<dyn Fn(Context, Vec<u8>) -> BoxFuture<Result<Vec<Vec<u8>>, Error>>>;

/// Records handlers, erased to bytes in and bytes out.
#[derive(Default)]
struct Recorder {
    unary: BTreeMap<&'static str, Unary>,
    streaming: BTreeMap<&'static str, Streaming>,
}

impl Registry for Recorder {
    fn register_unary<M, F, Fut>(&mut self, handler: F)
    where
        M: UnaryMethod,
        F: Fn(Context, M::Request) -> Fut + 'static,
        Fut: Future<Output = Result<M::Response, Error>> + 'static,
    {
        let handler = Rc::new(handler);
        self.unary.insert(
            M::NAME,
            Box::new(move |ctx, arg| {
                let handler = Rc::clone(&handler);
                Box::pin(async move {
                    let req = M::Request::decode(arg.as_slice()).unwrap();
                    handler(ctx, req).await.map(|r| r.encode_to_vec())
                })
            }),
        );
    }

    fn register_streaming<M, F, Fut>(&mut self, handler: F)
    where
        M: StreamingMethod,
        F: Fn(Context, M::Request) -> Fut + 'static,
        Fut: Future<Output = Result<ServerStream<M::Response>, Error>> + 'static,
    {
        let handler = Rc::new(handler);
        self.streaming.insert(
            M::NAME,
            Box::new(move |ctx, arg| {
                let handler = Rc::clone(&handler);
                Box::pin(async move {
                    let req = M::Request::decode(arg.as_slice()).unwrap();
                    let items = collect(handler(ctx, req).await?);
                    items
                        .into_iter()
                        .map(|i| i.map(|r| r.encode_to_vec()))
                        .collect()
                })
            }),
        );
    }
}

/// Implements two methods and leaves the rest to the defaults.
struct Partial;

impl DebugService for Partial {
    async fn p_prof(&self, ctx: &Context, req: PProfRequest) -> Result<PProfResponse, Error> {
        Ok(PProfResponse {
            name: format!("{}#{}", req.name, ctx.id()),
            data: ctx.header("x").unwrap_or_default().to_vec(),
        })
    }

    async fn watch_metrics(
        &self,
        _ctx: &Context,
        req: WatchMetricsRequest,
    ) -> Result<ServerStream<WatchMetricsResponse>, Error> {
        if req.interval_ms < 0 {
            return Err(Error::new(3, "negative interval"));
        }
        let mut items: VecDeque<_> = (0..req.interval_ms)
            .map(|i| {
                Ok(WatchMetricsResponse {
                    data: vec![u8::try_from(i).unwrap()],
                })
            })
            .collect();
        items.push_back(Err(Error::new(9, "done")));
        Ok(Box::pin(Items(items)))
    }
}

/// A stream of queued items.
struct Items<T>(VecDeque<T>);

impl<T> Stream for Items<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut task::Context<'_>) -> Poll<Option<T>> {
        Poll::Ready(self.0.pop_front())
    }
}

impl<T> Unpin for Items<T> {}

#[test]
fn markers_name_their_methods() {
    fn unary<M: UnaryMethod>() -> &'static str {
        M::NAME
    }
    fn streaming<M: StreamingMethod>() -> &'static str {
        M::NAME
    }
    assert_eq!(
        unary::<debug_methods::PProf>(),
        "strims.debug.v1.Debug.PProf"
    );
    assert_eq!(
        unary::<debug_methods::StopMockStream>(),
        "strims.debug.v1.Debug.StopMockStream"
    );
    assert_eq!(
        streaming::<debug_methods::WatchMetrics>(),
        "strims.debug.v1.Debug.WatchMetrics"
    );
}

#[test]
fn register_adds_every_method_by_kind() {
    let mut r = Recorder::default();
    register_debug_service(&mut r, Rc::new(Partial));
    assert_eq!(
        r.unary.keys().copied().collect::<Vec<_>>(),
        [
            "strims.debug.v1.Debug.GetConfig",
            "strims.debug.v1.Debug.PProf",
            "strims.debug.v1.Debug.ReadMetrics",
            "strims.debug.v1.Debug.SetConfig",
            "strims.debug.v1.Debug.StartMockStream",
            "strims.debug.v1.Debug.StopMockStream",
        ]
    );
    assert_eq!(
        r.streaming.keys().copied().collect::<Vec<_>>(),
        ["strims.debug.v1.Debug.WatchMetrics"]
    );
}

#[test]
fn handlers_reach_the_service_with_their_context() {
    let mut r = Recorder::default();
    register_debug_service(&mut r, Rc::new(Partial));
    let ctx = Context::new(7, BTreeMap::from([("x".to_owned(), vec![1, 2])]));
    let req = PProfRequest {
        name: "heap".into(),
        ..Default::default()
    };
    let res = ready(r.unary["strims.debug.v1.Debug.PProf"](
        ctx,
        req.encode_to_vec(),
    ))
    .unwrap();
    let res = PProfResponse::decode(res.as_slice()).unwrap();
    assert_eq!(res.name, "heap#7");
    assert_eq!(res.data, [1, 2]);
}

#[test]
fn unimplemented_methods_answer_not_implemented() {
    let mut r = Recorder::default();
    register_debug_service(&mut r, Rc::new(Partial));
    let err = ready(r.unary["strims.debug.v1.Debug.GetConfig"](
        Context::default(),
        GetConfigRequest::default().encode_to_vec(),
    ))
    .unwrap_err();
    assert_eq!(err, Error::unimplemented());
    assert_eq!(err.to_string(), "not implemented");
}

#[test]
fn streaming_handlers_yield_items_then_errors() {
    let mut r = Recorder::default();
    register_debug_service(&mut r, Rc::new(Partial));
    let watch = &r.streaming["strims.debug.v1.Debug.WatchMetrics"];

    let req = WatchMetricsRequest {
        interval_ms: 2,
        ..Default::default()
    };
    let err = ready(watch(Context::default(), req.encode_to_vec())).unwrap_err();
    // the stream's error item surfaces after its two items
    assert_eq!(err, Error::new(9, "done"));

    let req = WatchMetricsRequest {
        interval_ms: -1,
        ..Default::default()
    };
    let err = ready(watch(Context::default(), req.encode_to_vec())).unwrap_err();
    assert_eq!(err.code, 3);
}

/// Records each call and answers with defaults.
#[derive(Clone, Default)]
struct Echo {
    calls: Rc<RefCell<Vec<&'static str>>>,
}

impl Caller for Echo {
    type Stream<T: Message + Default + 'static> = Items<Result<T, CallError>>;

    fn call_unary<M: UnaryMethod>(
        &self,
        _req: &M::Request,
    ) -> impl Future<Output = Result<M::Response, CallError>> {
        self.calls.borrow_mut().push(M::NAME);
        std::future::ready(Ok(M::Response::default()))
    }

    fn call_streaming<M: StreamingMethod>(
        &self,
        _req: &M::Request,
    ) -> impl Future<Output = Result<Self::Stream<M::Response>, CallError>> {
        self.calls.borrow_mut().push(M::NAME);
        let items = VecDeque::from([Ok(M::Response::default()), Err(CallError::Disconnected)]);
        std::future::ready(Ok(Items(items)))
    }
}

#[test]
fn clients_call_through_the_method_markers() {
    let echo = Echo::default();
    let client = DebugClient::new(echo.clone());
    ready(client.p_prof(&PProfRequest::default())).unwrap();
    ready(client.read_metrics(&ReadMetricsRequest::default())).unwrap();
    let stream = ready(client.watch_metrics(&WatchMetricsRequest::default())).unwrap();
    assert_eq!(
        collect(stream),
        [
            Ok(WatchMetricsResponse::default()),
            Err(CallError::Disconnected)
        ]
    );
    assert_eq!(
        *echo.calls.borrow(),
        [
            "strims.debug.v1.Debug.PProf",
            "strims.debug.v1.Debug.ReadMetrics",
            "strims.debug.v1.Debug.WatchMetrics",
        ]
    );
}

#[test]
fn errors_display_their_message() {
    assert_eq!(
        Error::method_not_found("a.B.C").to_string(),
        "method not found: a.B.C"
    );
    assert_eq!(
        CallError::Remote(Error::new(5, "boom")).to_string(),
        "remote error (code 5): boom"
    );
}
