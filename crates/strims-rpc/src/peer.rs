// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! RPC between two nodes over a pair of channels.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use futures_sink::Sink;

use crate::client::Client;
use crate::connection::{Connection, ConnectionError, connect};
use crate::endpoint::Config;
use crate::router::Router;

/// Runs peer RPC over a channel pair: calls go out on the `client` channel
/// and `router` serves what arrives on the `server` channel. Each channel
/// is a reader of received bytes and a writer of Calls, as for
/// [`connect`].
///
/// The returned future ends when either channel's connection does.
pub fn peer_connection<R, W, B, E>(
    config: Config,
    client: (R, W),
    server: (R, W),
    router: Router,
) -> (Client, PeerConnection<R, W>)
where
    R: Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    W: Sink<Vec<u8>, Error = E> + Unpin,
{
    let client_config = Config {
        serve: false,
        ..config.clone()
    };
    let server_config = Config {
        serve: true,
        ..config
    };
    let (caller, client) = connect(client_config, client.0, client.1, None);
    // the server channel makes no calls of its own
    let (_, server) = connect(server_config, server.0, server.1, Some(router));
    (caller, PeerConnection { client, server })
}

/// A running channel pair; see [`peer_connection`].
#[derive(Debug)]
#[must_use = "a connection does nothing unless polled"]
pub struct PeerConnection<R, W> {
    client: Connection<R, W>,
    server: Connection<R, W>,
}

impl<R, W> PeerConnection<R, W> {
    /// The calling channel's counters.
    pub fn client_metrics(&self) -> crate::Metrics {
        self.client.metrics()
    }

    /// The serving channel's counters.
    pub fn server_metrics(&self) -> crate::Metrics {
        self.server.metrics()
    }
}

impl<R, W, B, E> Future for PeerConnection<R, W>
where
    R: Stream<Item = Result<B, E>> + Unpin,
    B: AsRef<[u8]>,
    W: Sink<Vec<u8>, Error = E> + Unpin,
{
    type Output = Result<(), ConnectionError<E>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if let Poll::Ready(r) = Pin::new(&mut this.client).poll(cx) {
            return Poll::Ready(r);
        }
        Pin::new(&mut this.server).poll(cx)
    }
}
