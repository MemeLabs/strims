// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Calls through a connection.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use futures_core::Stream;
use prost::Message;
use strims_proto::service::{CallError, Caller, StreamingMethod, UnaryMethod};

use crate::endpoint::{CallId, RemoteId, Response};
use crate::shared::{SharedRef, with_endpoint};
use crate::wire::{Argument, Encoded};

/// Makes calls through a connection; see [`crate::connect`]. Clones share
/// the connection and call the same remote: the connection's peer, or the
/// address a [`VpnClient`](crate::vpn::VpnClient) gave it.
///
/// A request is queued when its call is made, so calls go out in the order
/// they're made. Dropping a call's future or stream before it ends cancels
/// the call. Once the connection ends, calls fail with
/// [`CallError::Disconnected`].
#[derive(Clone)]
pub struct Client {
    shared: SharedRef,
    to: RemoteId,
    /// Keeps the remote's address while the client exists.
    _pin: Option<Rc<()>>,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl Client {
    pub(crate) fn new(shared: SharedRef) -> Client {
        Client::to_remote(shared, RemoteId::STREAM, None)
    }

    pub(crate) fn to_remote(shared: SharedRef, to: RemoteId, pin: Option<Rc<()>>) -> Client {
        Client {
            shared,
            to,
            _pin: pin,
        }
    }

    pub(crate) fn start(
        &self,
        method: &str,
        streaming: bool,
        req: &impl Argument,
    ) -> Result<Call, CallError> {
        let to = self.to;
        let id = with_endpoint(&self.shared, |e| {
            e.start_call_to(to, method, streaming, req)
        })?;
        Ok(Call {
            shared: self.shared.clone(),
            id,
            done: false,
        })
    }

    /// Calls the unary method named `method` (its full name, as
    /// [`Method::NAME`](strims_proto::service::Method::NAME) gives it) with
    /// the encoded request `req`: the encoded response. For a caller that
    /// knows methods only by name, such as one that reads and writes
    /// messages through the schema's descriptors.
    pub fn call_encoded(
        &self,
        method: &str,
        req: &[u8],
    ) -> impl Future<Output = Result<Vec<u8>, CallError>> + use<> {
        let (call, error) = match self.start(method, false, &Encoded(req)) {
            Ok(call) => (Some(call), None),
            Err(e) => (None, Some(e)),
        };
        UnaryCall {
            call,
            error,
            decode: Ok,
        }
    }

    /// Calls the server-streaming method named `method` with the encoded
    /// request `req`: its encoded responses. See [`Client::call_encoded`].
    pub fn call_streaming_encoded(
        &self,
        method: &str,
        req: &[u8],
    ) -> Result<ClientStream<Vec<u8>>, CallError> {
        self.start(method, true, &Encoded(req))
            .map(|call| ClientStream { call, decode: Ok })
    }
}

/// A typed call's response decoder.
fn decode<T: Message + Default>(b: Vec<u8>) -> Result<T, prost::DecodeError> {
    T::decode(b.as_slice())
}

impl Caller for Client {
    type Stream<T: Message + Default + 'static> = ClientStream<T>;

    fn call_unary<M: UnaryMethod>(
        &self,
        req: &M::Request,
    ) -> impl Future<Output = Result<M::Response, CallError>> + use<M> {
        let (call, error) = match self.start(M::NAME, false, req) {
            Ok(call) => (Some(call), None),
            Err(e) => (None, Some(e)),
        };
        UnaryCall {
            call,
            error,
            decode: decode::<M::Response>,
        }
    }

    fn call_streaming<M: StreamingMethod>(
        &self,
        req: &M::Request,
    ) -> impl Future<Output = Result<Self::Stream<M::Response>, CallError>> + use<M> {
        let stream = self.start(M::NAME, true, req).map(|call| ClientStream {
            call,
            decode: decode::<M::Response>,
        });
        std::future::ready(stream)
    }
}

/// An open call: cancelled on drop unless it ended.
pub(crate) struct Call {
    shared: SharedRef,
    id: CallId,
    done: bool,
}

/// How far an open call has got; see [`Call::progress`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Progress {
    /// The request hasn't left the connection.
    pub(crate) queued: bool,
    /// Part of a Call has arrived on the connection and the rest hasn't.
    pub(crate) receiving: bool,
    /// Bytes the connection has received.
    pub(crate) received: u64,
}

impl Call {
    pub(crate) fn progress(&self) -> Progress {
        let s = self.shared.borrow();
        Progress {
            queued: s.endpoint.is_request_queued(self.id),
            receiving: s.endpoint.is_receiving(),
            received: s.endpoint.received_bytes(),
        }
    }

    /// A unary call's result, or `Pending` with `cx` registered to wake.
    pub(crate) fn poll_unary<T: Message + Default>(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<T, CallError>> {
        self.poll_unary_with(cx, decode::<T>)
    }

    /// A unary call's result, its response decoded by `decode`.
    fn poll_unary_with<T>(
        &mut self,
        cx: &mut Context<'_>,
        decode: fn(Vec<u8>) -> Result<T, prost::DecodeError>,
    ) -> Poll<Result<T, CallError>> {
        let res = match self.poll_response(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Response::Item(b)) => decode(b).map_err(CallError::Decode),
            Poll::Ready(Response::End) => Err(CallError::Closed),
            Poll::Ready(Response::Failed(e)) => Err(e),
        };
        self.done = true;
        Poll::Ready(res)
    }

    /// The call's next response, or `Pending` with `cx` registered to wake.
    fn poll_response(&mut self, cx: &mut Context<'_>) -> Poll<Response> {
        // the replaced waker is dropped after the borrow
        let (res, _old) = {
            let mut s = self.shared.borrow_mut();
            match s.endpoint.poll_response(self.id) {
                Some(r) => (Poll::Ready(r), s.call_wakers.remove(&self.id)),
                None => (
                    Poll::Pending,
                    s.call_wakers.insert(self.id, cx.waker().clone()),
                ),
            }
        };
        if let Poll::Ready(r) = &res
            && !matches!(r, Response::Item(_))
        {
            self.done = true;
        }
        res
    }

    fn cancel(&mut self) {
        if !self.done {
            self.done = true;
            let id = self.id;
            with_endpoint(&self.shared, |e| e.cancel(id));
        }
    }
}

impl Drop for Call {
    fn drop(&mut self) {
        let waker = self.shared.borrow_mut().call_wakers.remove(&self.id);
        drop(waker);
        self.cancel();
    }
}

/// A unary call's response.
struct UnaryCall<T> {
    /// The call, until it resolves.
    call: Option<Call>,
    /// Why the call couldn't start.
    error: Option<CallError>,
    /// Turns the encoded response into the call's result.
    decode: fn(Vec<u8>) -> Result<T, prost::DecodeError>,
}

impl<T> Future for UnaryCall<T> {
    type Output = Result<T, CallError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(e) = self.error.take() {
            return Poll::Ready(Err(e));
        }
        let decode = self.decode;
        // polled again after resolving
        let Some(call) = self.call.as_mut() else {
            return Poll::Ready(Err(CallError::Disconnected));
        };
        let res = std::task::ready!(call.poll_unary_with(cx, decode));
        self.call = None;
        Poll::Ready(res)
    }
}

/// The responses of a server-streaming call. Dropping it before it ends
/// cancels the call.
///
/// A response that doesn't decode ends the stream with
/// [`CallError::Decode`] and cancels the call.
pub struct ClientStream<T> {
    call: Call,
    /// Turns each encoded response into an item.
    decode: fn(Vec<u8>) -> Result<T, prost::DecodeError>,
}

impl<T> fmt::Debug for ClientStream<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientStream")
            .field("call", &self.call.id)
            .field("done", &self.call.done)
            .finish()
    }
}

impl<T> Stream for ClientStream<T> {
    type Item = Result<T, CallError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.call.done {
            return Poll::Ready(None);
        }
        match self.call.poll_response(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Response::Item(b)) => match (self.decode)(b) {
                Ok(item) => Poll::Ready(Some(Ok(item))),
                Err(e) => {
                    self.call.cancel();
                    Poll::Ready(Some(Err(CallError::Decode(e))))
                }
            },
            Poll::Ready(Response::End) => Poll::Ready(None),
            Poll::Ready(Response::Failed(e)) => Poll::Ready(Some(Err(e))),
        }
    }
}
