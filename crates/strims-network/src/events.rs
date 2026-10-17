// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the network control reports to the app.

use strims_proto::network::v1::Network;

/// A change in the networks the node runs. Subscribers get them through
/// [`NetworkControl::subscribe`](crate::NetworkControl::subscribe).
///
/// A network's events start with `Started` and end with `Stopped`, and
/// `Joined` comes between them at most once: not at all if the node fails
/// to join the network. A network the store deletes and holds again
/// starts over, and the node joins it again under the handle
/// [`Host::add_network`](crate::Host::add_network) gives then. A
/// subscriber that lags reads
/// [`NetworkControl::networks`](crate::NetworkControl::networks) again.
#[derive(Clone, Debug, PartialEq)]
pub enum NetworkEvent {
    /// The node runs the network: the profile's store holds it, and the
    /// node is joining it.
    Started { network: Network },
    /// The node joined the network and started its services:
    /// [`NetworkControl::network`](crate::NetworkControl::network) has its
    /// handle.
    Joined { network: Network },
    /// The node stopped running the network.
    Stopped { network: Network },
    /// The profile's record of the network changed, such as its
    /// certificate, alias, server config or renewal error.
    Updated { network: Network },
    /// The number of peers bound to the network changed.
    PeerCount { network_id: u64, count: usize },
}
