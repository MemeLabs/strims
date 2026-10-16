// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The interfaces that generated RPC service code is written against.
//!
//! With the `services` feature, every proto service `Foo` in package `p`
//! also gets, in `p`'s module:
//!
//! - `foo_methods`, one marker type per method implementing [`Method`] and
//!   [`UnaryMethod`] or [`StreamingMethod`];
//! - `FooService`, a trait with one method per RPC. Every method has a
//!   default body that answers [`Error::unimplemented`], so a service can
//!   implement a subset;
//! - `register_foo_service`, which adds a `FooService` to a [`Registry`];
//! - `FooClient<C>`, typed calls through a [`Caller`].
//!
//! A server-streaming handler resolves to a [`ServerStream`]. Resolving to
//! `Err` answers the call with that error and nothing else; an `Err` item
//! sends the error and ends the stream; the stream's end closes the call.
//!
//! Futures and streams here carry no `Send` bound: a connection drives its
//! handlers on one task, and the browser target's futures can't be `Send`.
//! The runtime that implements [`Registry`] and [`Caller`] is `strims-rpc`.

use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

use futures_core::Stream;

pub use crate::rpc::Error;

/// An RPC method of a proto service.
pub trait Method: 'static {
    /// `<package>.<Service>.<Method>`, the method name on the wire.
    const NAME: &'static str;
    /// The request message.
    type Request: prost::Message + Default + 'static;
    /// The response message, or each item of a server stream.
    type Response: prost::Message + Default + 'static;
}

/// A method that answers with one response.
pub trait UnaryMethod: Method {}

/// A method that answers with a stream of responses.
pub trait StreamingMethod: Method {}

impl Error {
    /// An error with an application code (`0` when there's none). Proto
    /// error-code enums convert with `Into<i32>`.
    pub fn new(code: impl Into<i32>, message: impl Into<String>) -> Error {
        Error {
            message: message.into(),
            code: code.into(),
        }
    }

    /// The answer of a service method that has no implementation.
    pub fn unimplemented() -> Error {
        Error::new(0, "not implemented")
    }

    /// The answer to a call of a method that isn't registered.
    pub fn method_not_found(method: &str) -> Error {
        Error::new(0, format!("method not found: {method}"))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

/// A server stream's items. An `Err` item ends the stream with that error.
pub type ServerStream<T> = Pin<Box<dyn Stream<Item = Result<T, Error>>>>;

/// What a handler knows about the call it serves.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Context {
    id: u64,
    headers: BTreeMap<String, Vec<u8>>,
}

impl Context {
    /// The context of the caller's call `id`, with the call's headers.
    pub fn new(id: u64, headers: BTreeMap<String, Vec<u8>>) -> Context {
        Context { id, headers }
    }

    /// The caller's id for the call. Ids are unique per caller and
    /// connection, not globally.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The value of the call's header `name`.
    pub fn header(&self, name: &str) -> Option<&[u8]> {
        self.headers.get(name).map(Vec::as_slice)
    }
}

/// Why a call failed on the calling side.
#[derive(Clone, Debug, PartialEq)]
pub enum CallError {
    /// The server answered with an error.
    Remote(Error),
    /// A response didn't decode as the method's response type.
    Decode(prost::DecodeError),
    /// The server ended a unary call without a value.
    Closed,
    /// The server answered that the method returns nothing.
    Undefined,
    /// A response had a kind this build doesn't know.
    UnknownKind(i32),
    /// The connection ended before the call did.
    Disconnected,
    /// Responses arrived faster than the caller took them, so the call was
    /// cancelled.
    Overflow,
    /// The encoded request is larger than the connection sends.
    TooLarge {
        /// The encoded request's length.
        len: usize,
        /// The connection's limit.
        max: usize,
    },
    /// The connection has its maximum number of calls open.
    TooManyCalls,
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallError::Remote(e) => write!(f, "remote error (code {}): {}", e.code, e.message),
            CallError::Decode(e) => write!(f, "decoding the response: {e}"),
            CallError::Closed => f.write_str("call closed without a response"),
            CallError::Undefined => f.write_str("call returned no value"),
            CallError::UnknownKind(k) => write!(f, "response of unknown kind {k}"),
            CallError::Disconnected => f.write_str("connection closed"),
            CallError::Overflow => f.write_str("responses overflowed the call's queue"),
            CallError::TooLarge { len, max } => {
                write!(f, "request of {len} bytes exceeds the {max}-byte limit")
            }
            CallError::TooManyCalls => f.write_str("too many open calls"),
        }
    }
}

impl std::error::Error for CallError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CallError::Remote(e) => Some(e),
            CallError::Decode(e) => Some(e),
            _ => None,
        }
    }
}

/// A set of method handlers that a server dispatches calls to.
pub trait Registry {
    /// Serves `M` with `handler`, replacing an earlier handler for it.
    fn register_unary<M, F, Fut>(&mut self, handler: F)
    where
        M: UnaryMethod,
        F: Fn(Context, M::Request) -> Fut + 'static,
        Fut: Future<Output = Result<M::Response, Error>> + 'static;

    /// Serves the server-streaming `M` with `handler`, replacing an earlier
    /// handler for it.
    fn register_streaming<M, F, Fut>(&mut self, handler: F)
    where
        M: StreamingMethod,
        F: Fn(Context, M::Request) -> Fut + 'static,
        Fut: Future<Output = Result<ServerStream<M::Response>, Error>> + 'static;
}

/// Something that makes calls: a connection's client, or a wrapper around
/// one.
///
/// A call's future borrows neither the caller nor the request:
/// implementations take what they need when the call is made. Dropping a
/// call's future, or its stream, before it ends cancels the call. Callers
/// impose their own deadlines that way.
pub trait Caller {
    /// The responses of a server-streaming call.
    type Stream<T: prost::Message + Default + 'static>: Stream<Item = Result<T, CallError>>
        + Unpin
        + 'static;

    /// Calls the unary `M`.
    fn call_unary<M: UnaryMethod>(
        &self,
        req: &M::Request,
    ) -> impl Future<Output = Result<M::Response, CallError>> + use<Self, M>;

    /// Calls the server-streaming `M`. The future resolves once the call is
    /// started; its stream yields the responses.
    fn call_streaming<M: StreamingMethod>(
        &self,
        req: &M::Request,
    ) -> impl Future<Output = Result<Self::Stream<M::Response>, CallError>> + use<Self, M>;
}
