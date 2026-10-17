// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the directory control reports to the app.

use strims_proto::network::v1::directory::AssetBundle;

use crate::view::{Listing, User};

/// A change in the directories the node runs. Subscribers get them
/// through [`DirectoryControl::subscribe`](crate::DirectoryControl::subscribe).
///
/// A network's events start with `Started`, when its view starts empty,
/// and end with `Stopped`, after which the network's view is gone. A
/// network the node joins again starts over. A subscriber that lags, or
/// gets `Lagged`, reads the views again.
#[derive(Clone, Debug, PartialEq)]
// Subscribers receive each event behind one shared `Rc`.
#[allow(clippy::large_enum_variant)]
pub enum DirectoryEvent {
    /// The directory of a network the node joined runs, with an empty
    /// view.
    Started {
        network_id: u64,
        network_key: Vec<u8>,
    },
    /// The directory stopped with the network.
    Stopped {
        network_id: u64,
        network_key: Vec<u8>,
    },
    /// A listing of the network's view changed.
    Listing {
        network_id: u64,
        event: ListingEvent,
    },
    /// A user of a listing of the network's view changed.
    User {
        network_id: u64,
        listing_id: u64,
        kind: UserEventKind,
        user: User,
    },
    /// The network's asset bundle changed.
    AssetBundle {
        network_id: u64,
        network_key: Vec<u8>,
        bundle: AssetBundle,
    },
    /// The network's view dropped events for its queue's bound: read it
    /// again.
    Lagged { network_id: u64 },
}

/// A change of a listing.
#[derive(Clone, Debug, PartialEq)]
pub enum ListingEvent {
    /// The listing was added, or its listing, snippet or moderation
    /// changed.
    Changed(Listing),
    /// The listing was removed.
    Unpublished(Listing),
    /// The listing's user counts changed.
    UserCountChanged {
        id: u64,
        user_count: u32,
        recent_user_count: u32,
    },
}

/// How a listing's user changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserEventKind {
    Joined,
    Parted,
    /// The user took a new alias.
    Renamed,
}
