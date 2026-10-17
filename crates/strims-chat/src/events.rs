// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the chat control reports to the app.

use strims_proto::chat::v1::{AssetBundle, ServerEvent};

/// A change in the chat servers the node serves or reads. Subscribers get
/// them through [`ChatControl::subscribe`](crate::ChatControl::subscribe).
///
/// A server's `ServeStarted` comes once its swarms, address and first
/// asset bundle are up, and its `ServeStopped` once they are gone. Its
/// listing's id is reported each time it changes while it serves. A room
/// the node reads without serving it has `ReadStarted` when its swarms
/// are added and `ReadStopped` when they are removed. A subscriber that
/// lags reads [`ChatControl::servers`](crate::ChatControl::servers) again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatEvent {
    /// This process serves the server on the network.
    ServeStarted {
        server_id: u64,
        server_key: Vec<u8>,
        network_key: Vec<u8>,
    },
    /// This process stopped serving it.
    ServeStopped {
        server_id: u64,
        server_key: Vec<u8>,
        network_key: Vec<u8>,
    },
    /// The directory listing of a server this process serves has a new
    /// id: it was published, or published again.
    ListingChanged {
        server_id: u64,
        server_key: Vec<u8>,
        network_key: Vec<u8>,
        listing_id: u64,
    },
    /// The node added the swarms of a server it doesn't serve, for its
    /// client streams.
    ReadStarted {
        server_key: Vec<u8>,
        network_key: Vec<u8>,
    },
    /// The node removed them.
    ReadStopped {
        server_key: Vec<u8>,
        network_key: Vec<u8>,
    },
}

/// What a [`ClientStream`](crate::ClientStream) yields.
#[derive(Clone, Debug, PartialEq)]
// Bundles are rare and events come in batches.
#[allow(clippy::large_enum_variant)]
pub enum ClientEvent {
    /// A record of the server's asset stream. After the stream opens, and
    /// after each discontinuity, the first is unified (`is_delta` false);
    /// the rest are deltas that apply on top of it.
    Assets(AssetBundle),
    /// The next records of the server's event stream, in order. A batch
    /// holds at least one event, and at most
    /// [`Config::max_batch_events`](crate::Config::max_batch_events)
    /// events or about
    /// [`Config::max_batch_bytes`](crate::Config::max_batch_bytes).
    Events(Vec<ServerEvent>),
}
