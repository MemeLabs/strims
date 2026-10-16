// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! RPC runtime for the services that `strims-proto` generates with its
//! `services` feature.
//!
//! # Wire format
//!
//! Every request, response and cancel is one `strims.rpc.Call` ([`wire`]).
//! A request names its method `<package>.<Service>.<Method>` and carries
//! the encoded request message. The server answers a unary call with one
//! value or one error, and a server-streaming call with any number of
//! values followed by a close or an error. Either side may cancel a call;
//! a cancelled server stops without answering.
//!
//! On byte streams and on message transports each Call is prefixed with
//! its length as a uvarint ([`Framing::Delimited`]). On message transports
//! every prefixed Call goes out as exactly one message, since receivers
//! may parse each message on its own. [`Framing::Message`] sends one
//! unprefixed Call per transport message.
//!
//! # Layers
//!
//! - [`Endpoint`] is a sans-IO state machine for one side of one transport:
//!   framing, the call tables, and every bound on what a peer can make it
//!   hold. It has no timers: deadlines belong to callers, who cancel.
//! - Errors a handler returns reach the caller as
//!   [`strims_proto::service::CallError::Remote`], with their code.
//!
//! # Hostile peers
//!
//! A peer controls what arrives, so every queue it can grow is bounded by
//! a [`Config`] limit, and malformed Calls are skipped. A Call over the
//! size limit closes the endpoint: the stream can't be resynchronised.

pub mod endpoint;
pub mod frame;
pub mod wire;

pub use endpoint::{CallId, Config, Endpoint, Error, Event, InboundId, Metrics, Response};
pub use frame::{FrameDecoder, FrameError};
pub use wire::Framing;
