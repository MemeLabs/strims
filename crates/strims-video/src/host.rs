// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the video control needs from the node it runs on: a listener for
//! the RTMP ingress.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use futures_sink::Sink;

/// The node operations behind the video ingress. Only futures traits
/// cross this boundary, so the browser and native drivers both supply
/// one.
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
}

/// A connection a [`VideoHost::Listener`] accepted.
pub struct Accepted<R, W> {
    pub reader: R,
    pub writer: W,
    /// The remote address, as the host names it.
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
