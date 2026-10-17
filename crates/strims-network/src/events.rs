// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the network control reports to the app.

use strims_proto::network::v1::Network;

/// A change in the networks the node runs. Subscribers get them through
/// [`NetworkControl::subscribe`](crate::NetworkControl::subscribe).
#[derive(Clone, Debug, PartialEq)]
pub enum NetworkEvent {
    /// The node runs the network.
    Started { network: Network },
    /// The node stopped running the network.
    Stopped { network: Network },
    /// The profile's record of the network changed: its certificate,
    /// alias or renewal error.
    Updated { network: Network },
    /// The number of peers bound to the network changed.
    PeerCount { network_id: u64, count: usize },
}
