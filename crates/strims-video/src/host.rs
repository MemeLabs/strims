// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the video control needs from the node it runs on: a listener for
//! the RTMP ingress, and the codec stage where the node has one.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use futures_sink::Sink;

use crate::codec::{Codec, NoCodec};

/// The node operations behind the video ingress. Only futures traits
/// cross this boundary, so the browser and native drivers both supply
/// one.
///
/// What the [ingress](crate::ingress) relies on:
///
/// - [`listen`](VideoHost::listen) binds `addr` and returns its listener,
///   or fails; the control binds again after
///   [`IngressSettings::bind_retry`](crate::IngressSettings::bind_retry).
/// - The listener yields each connection it accepts. An error, or its
///   end, ends it: the control drops it and binds again after the bind
///   retry. Dropping it stops listening and leaves the connections it
///   took open.
/// - A connection's reader yields its bytes in the pieces they arrive in;
///   its end is the peer's end of the stream, and an error ends the
///   connection.
/// - Its writer takes a piece once [`Sink::poll_ready`] is ready, which
///   waits while the pieces before are being sent: what the control
///   queues for a peer that doesn't read stays within its bound.
///   [`Sink::poll_flush`] resolves once they are sent, and
///   [`Sink::poll_close`] shuts the sending side down.
/// - Dropping the reader and the writer closes the connection; that is
///   how a connection over a cap closes.
/// - [`codec`](VideoHost::codec) is the node's codec stage
///   ([`codec`](crate::codec)), or `None` where it has none: a browser,
///   or a native node without an ffmpeg.
pub trait VideoHost: Clone + 'static {
    type Error: fmt::Display;
    /// The bytes a connection receives, in the pieces they arrive in.
    type Reader: Stream<Item = Result<Vec<u8>, Self::Error>> + Unpin + 'static;
    /// Sends bytes on a connection.
    type Writer: Sink<Vec<u8>, Error = Self::Error> + Unpin + 'static;
    /// The connections a listener accepts. Dropping it closes it.
    type Listener: Stream<Item = Result<Accepted<Self::Reader, Self::Writer>, Self::Error>>
        + Unpin
        + 'static;

    /// Whether [`VideoHost::listen`] can succeed: the host runs TCP
    /// listeners.
    fn rtmp_supported(&self) -> bool;

    /// Listens for TCP connections on `addr`, a `host:port`.
    fn listen(&self, addr: &str) -> impl Future<Output = Result<Self::Listener, Self::Error>>;

    /// The node's codec stage.
    type Codec: Codec;

    /// The node's codec stage, if it has one.
    fn codec(&self) -> Option<Self::Codec>;
}

/// A connection a [`VideoHost::Listener`] accepted.
pub struct Accepted<R, W> {
    pub reader: R,
    pub writer: W,
    /// The remote address: `host:port`, an IPv6 host in brackets. The
    /// per-address cap counts the host.
    pub peer: String,
}

/// A host without TCP listeners, such as a browser: it has no RTMP
/// ingress, and [`VideoHost::listen`] fails.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoVideoHost;

/// Why a [`NoVideoHost`] listen failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("RTMP ingress is not supported on this host")]
pub struct NoRtmp;

/// A connection of a host without listeners: none exists.
#[derive(Debug)]
pub enum NoConnection {}

/// A listener of a host without listeners: none exists.
#[derive(Debug)]
pub enum NoListener {}

impl Stream for NoConnection {
    type Item = Result<Vec<u8>, NoRtmp>;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match *self {}
    }
}

impl Sink<Vec<u8>> for NoConnection {
    type Error = NoRtmp;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), NoRtmp>> {
        match *self {}
    }

    fn start_send(self: Pin<&mut Self>, _: Vec<u8>) -> Result<(), NoRtmp> {
        match *self {}
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), NoRtmp>> {
        match *self {}
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), NoRtmp>> {
        match *self {}
    }
}

impl Stream for NoListener {
    type Item = Result<Accepted<NoConnection, NoConnection>, NoRtmp>;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match *self {}
    }
}

impl VideoHost for NoVideoHost {
    type Error = NoRtmp;
    type Reader = NoConnection;
    type Writer = NoConnection;
    type Listener = NoListener;

    fn rtmp_supported(&self) -> bool {
        false
    }

    fn listen(&self, _: &str) -> impl Future<Output = Result<NoListener, NoRtmp>> {
        std::future::ready(Err(NoRtmp))
    }

    type Codec = NoCodec;

    fn codec(&self) -> Option<NoCodec> {
        None
    }
}

#[cfg(test)]
mod tests {
    use futures_util::FutureExt;

    use super::*;

    #[test]
    fn a_host_without_listeners_refuses_to_listen() {
        assert!(!NoVideoHost.rtmp_supported());
        let res = NoVideoHost.listen("127.0.0.1:1935").now_or_never();
        assert!(matches!(res, Some(Err(NoRtmp))));
    }
}
