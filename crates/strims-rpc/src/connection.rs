// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The future that runs an [`Endpoint`] over a transport.

use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use futures_sink::Sink;
use futures_util::StreamExt;
use futures_util::future::{AbortHandle, Abortable, LocalBoxFuture, abortable};
use futures_util::stream::FuturesUnordered;
use strims_proto::service::{Context as CallContext, Error as RpcError};

use crate::client::Client;
use crate::endpoint::{Config, Endpoint, Event, InboundId};
use crate::router::Router;
use crate::shared::{Responder, Shared, SharedRef};

/// Rounds of work per poll before the connection yields to other tasks. A
/// round reads at most one input item; a round that wakes a caller ends the
/// poll early.
const ROUND_BUDGET: usize = 32;

/// Why a connection ended.
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError<E> {
    /// The transport failed to deliver input.
    #[error("reading: {0}")]
    Read(E),
    /// The transport failed to take output.
    #[error("writing: {0}")]
    Write(E),
    /// The peer broke the protocol.
    #[error(transparent)]
    Protocol(#[from] crate::Error),
}

/// Runs an [`Endpoint`] with `config` over a transport, serving `router`'s
/// methods (or none, as a client only) and calling through the returned
/// [`Client`].
///
/// The transport is a stream of received bytes, split anywhere, and a sink
/// of Calls: each item the sink takes is one whole Call, so a message
/// transport sends it as one message.
///
/// The connection does its work when the returned future is polled. It ends
/// at the end of the input, with `Ok`, or on a transport or protocol error.
/// Once it ends or is dropped, open calls fail with
/// [`CallError::Disconnected`](strims_proto::service::CallError::Disconnected)
/// and handlers in service are dropped. Handlers run inside the future, so
/// they must not block, and a panicking handler ends the task it runs in.
pub fn connect<R, W, B, E>(
    mut config: Config,
    reader: R,
    writer: W,
    router: Option<Router>,
) -> (Client, Connection<R, W>)
where
    R: Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    W: Sink<Vec<u8>, Error = E> + Unpin,
{
    config.serve &= router.is_some();
    let shared = Shared::new(Endpoint::new(config));
    let client = Client::new(shared.clone());
    let conn = Connection {
        core: Core::new(shared, router),
        io: ByteIo {
            reader,
            writer,
            flush: false,
        },
    };
    (client, conn)
}

/// A running connection; see [`connect`].
#[must_use = "a connection does nothing unless polled"]
pub struct Connection<R, W> {
    core: Core,
    io: ByteIo<R, W>,
}

impl<R, W> fmt::Debug for Connection<R, W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.core.fmt("Connection", f)
    }
}

impl<R, W> Connection<R, W> {
    /// The endpoint's counters.
    pub fn metrics(&self) -> crate::Metrics {
        self.core.metrics()
    }
}

impl<R, W, B, E> Future for Connection<R, W>
where
    R: Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    W: Sink<Vec<u8>, Error = E> + Unpin,
{
    type Output = Result<(), ConnectionError<E>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.core.poll(cx, &mut this.io)
    }
}

/// The transport half of a connection: moves Calls between the endpoint
/// and the transport.
pub(crate) trait Io {
    type Error;

    /// Moves queued Calls into the transport while it takes them, and
    /// returns whether it moved any.
    fn poll_write(&mut self, cx: &mut Context<'_>, shared: &SharedRef)
    -> Result<bool, Self::Error>;

    /// Hands at most one input item to the endpoint: `Ready(None)` at the
    /// end of the input, `Pending` if none is ready.
    fn poll_read(
        &mut self,
        cx: &mut Context<'_>,
        shared: &SharedRef,
    ) -> Poll<Option<Result<(), ConnectionError<Self::Error>>>>;
}

/// A byte stream (or a message transport carrying framed Calls) in, Calls
/// out.
struct ByteIo<R, W> {
    reader: R,
    writer: W,
    /// The sink has items that haven't been flushed.
    flush: bool,
}

impl<R, W, B, E> Io for ByteIo<R, W>
where
    R: Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    W: Sink<Vec<u8>, Error = E> + Unpin,
{
    type Error = E;

    fn poll_write(&mut self, cx: &mut Context<'_>, shared: &SharedRef) -> Result<bool, E> {
        let mut wrote = false;
        loop {
            if shared.borrow().endpoint.transmit_buffered() == 0 {
                break;
            }
            match Pin::new(&mut self.writer).poll_ready(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(e)) => return Err(e),
                Poll::Pending => break,
            }
            let Some(frame) = shared.borrow_mut().endpoint.poll_transmit() else {
                break;
            };
            Pin::new(&mut self.writer).start_send(frame)?;
            self.flush = true;
            wrote = true;
        }
        poll_flush(&mut self.writer, &mut self.flush, cx)?;
        Ok(wrote)
    }

    fn poll_read(
        &mut self,
        cx: &mut Context<'_>,
        shared: &SharedRef,
    ) -> Poll<Option<Result<(), ConnectionError<E>>>> {
        match Pin::new(&mut self.reader).poll_next(cx) {
            Poll::Ready(Some(Ok(data))) => {
                let res = shared.borrow_mut().endpoint.handle_input(data.as_ref());
                Poll::Ready(Some(res.map_err(ConnectionError::from)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(ConnectionError::Read(e)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Flushes `writer` if `pending` says it holds unflushed items.
pub(crate) fn poll_flush<W: Sink<T> + Unpin, T>(
    writer: &mut W,
    pending: &mut bool,
    cx: &mut Context<'_>,
) -> Result<(), W::Error> {
    if *pending {
        match Pin::new(writer).poll_flush(cx) {
            Poll::Ready(Ok(())) => *pending = false,
            Poll::Ready(Err(e)) => return Err(e),
            Poll::Pending => {}
        }
    }
    Ok(())
}

/// What a connection does whatever its transport: dispatches the
/// endpoint's events, runs handlers, and closes everything when it ends.
pub(crate) struct Core {
    shared: SharedRef,
    router: Option<Router>,
    /// Handlers in service. Bounded by the endpoint's inbound limit.
    handlers: FuturesUnordered<Abortable<LocalBoxFuture<'static, InboundId>>>,
    aborts: BTreeMap<InboundId, AbortHandle>,
    ended: bool,
}

impl Core {
    pub(crate) fn new(shared: SharedRef, router: Option<Router>) -> Core {
        Core {
            shared,
            router,
            handlers: FuturesUnordered::new(),
            aborts: BTreeMap::new(),
            ended: false,
        }
    }

    pub(crate) fn shared(&self) -> &SharedRef {
        &self.shared
    }

    pub(crate) fn fmt(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(name)
            .field("handlers", &self.aborts.len())
            .field("ended", &self.ended)
            .finish_non_exhaustive()
    }

    pub(crate) fn metrics(&self) -> crate::Metrics {
        self.shared.borrow().endpoint.metrics()
    }

    /// Closes the endpoint and wakes everything waiting on it.
    fn close(&mut self) {
        let (wakers, _driver) = {
            let mut s = self.shared.borrow_mut();
            s.endpoint.close();
            let mut w: Vec<Waker> = std::mem::take(&mut s.call_wakers).into_values().collect();
            w.append(&mut s.congested);
            (w, s.driver.take())
        };
        for w in wakers {
            w.wake();
        }
        for (_, h) in std::mem::take(&mut self.aborts) {
            h.abort();
        }
        self.ended = true;
    }

    /// Dispatches the endpoint's events. Returns whether there were any,
    /// and whether they woke a caller.
    fn handle_events(&mut self) -> (bool, bool) {
        let events: Vec<Event> = {
            let mut s = self.shared.borrow_mut();
            std::iter::from_fn(|| s.endpoint.poll_event()).collect()
        };
        let any = !events.is_empty();
        let mut woke = false;
        for ev in events {
            match ev {
                Event::Request {
                    call,
                    from: _,
                    id,
                    method,
                    argument,
                    headers,
                    certificate,
                } => {
                    let ctx = CallContext::new(id, headers).with_certificate(certificate);
                    self.start_handler(call, ctx, &method, argument);
                }
                Event::Cancelled { call } => {
                    if let Some(h) = self.aborts.remove(&call) {
                        h.abort();
                    }
                }
                Event::Response { call } => {
                    let waker = self.shared.borrow_mut().call_wakers.remove(&call);
                    if let Some(w) = waker {
                        w.wake();
                        woke = true;
                    }
                }
            }
        }
        (any, woke)
    }

    fn start_handler(
        &mut self,
        call: InboundId,
        ctx: CallContext,
        method: &str,
        argument: Vec<u8>,
    ) {
        let Some(handler) = self.router.as_ref().and_then(|r| r.get(method)) else {
            let e = RpcError::method_not_found(method);
            self.shared.borrow_mut().endpoint.reply_error(call, &e);
            return;
        };
        let responder = Responder::new(self.shared.clone(), call);
        let fut = handler(ctx, argument, responder);
        let (fut, abort) = abortable(Box::pin(async move {
            fut.await;
            call
        }) as LocalBoxFuture<'static, InboundId>);
        self.handlers.push(fut);
        self.aborts.insert(call, abort);
    }

    /// Wakes the server streams waiting for the transmit queue to drain,
    /// once it has.
    fn wake_uncongested(&self) {
        let waiting: Vec<Waker> = {
            let mut s = self.shared.borrow_mut();
            if s.endpoint.is_congested() {
                Vec::new()
            } else {
                std::mem::take(&mut s.congested)
            }
        };
        for w in waiting {
            w.wake();
        }
    }

    /// Runs rounds of work over `io`. The connection ends at the end of
    /// the input, or on a transport or protocol error.
    pub(crate) fn poll<T: Io>(
        &mut self,
        cx: &mut Context<'_>,
        io: &mut T,
    ) -> Poll<Result<(), ConnectionError<T::Error>>> {
        if self.ended {
            return Poll::Ready(Ok(()));
        }
        let _old = self.shared.borrow_mut().driver.replace(cx.waker().clone());
        for _ in 0..ROUND_BUDGET {
            let mut progress = match io.poll_write(cx, &self.shared) {
                Ok(wrote) => {
                    if wrote {
                        self.wake_uncongested();
                    }
                    wrote
                }
                Err(e) => {
                    self.close();
                    return Poll::Ready(Err(ConnectionError::Write(e)));
                }
            };

            if !self.shared.borrow().endpoint.is_congested() {
                match io.poll_read(cx, &self.shared) {
                    Poll::Ready(Some(Ok(()))) => progress = true,
                    Poll::Ready(Some(Err(e))) => {
                        self.close();
                        return Poll::Ready(Err(e));
                    }
                    Poll::Ready(None) => {
                        self.close();
                        return Poll::Ready(Ok(()));
                    }
                    Poll::Pending => {}
                }
            }

            let (evented, woke) = self.handle_events();
            progress |= evented;

            while let Poll::Ready(Some(done)) = self.handlers.poll_next_unpin(cx) {
                if let Ok(call) = done {
                    self.aborts.remove(&call);
                }
                progress = true;
            }

            if woke {
                // let callers take their responses before more arrive, so
                // a reader that keeps up never overflows its call
                break;
            }
            if !progress {
                return Poll::Pending;
            }
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        if !self.ended {
            self.close();
        }
    }
}
