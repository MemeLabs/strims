// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The network directory: the listings a network's members publish
//! (media streams, chats, services), who publishes and views each, and
//! the media listings' snippets.
//!
//! One node per network serves the directory: a holder of the network's
//! server config (its root key) that holds the directory lease. It runs a
//! [`Server`], and every member reads the server's event stream into a
//! [`View`]. Both are sans-IO state machines; the control around them
//! does the I/O.
//!
//! # Wire contract
//!
//! These bytes and names are shared with every node of a network:
//!
//! - **Swarms.** The event swarm and the asset swarm both have the network
//!   key as their swarm id, and their transfers use the salts
//!   [`EVENT_SALT`] and [`ASSET_SALT`]: transfer id `sha256(swarm id ‖
//!   salt)`. Their options are [`event_options`] and [`asset_options`].
//! - **Address.** The server publishes a `vpn.v1.NetworkAddress { host_id,
//!   port }` in the network's hash table under (network key,
//!   [`ADDRESS_SALT`]), signed by the root key.
//! - **RPC.** The server serves `strims.network.v1.directory.Directory`
//!   (`Publish`, `Unpublish`, `Join`, `Part`, `Ping`, `ModerateListing`,
//!   `ModerateUser`) over vpn RPC on that port. The caller is the vpn
//!   certificate header: the host certificate (key = host id), its parent
//!   membership certificate (key = the user's peer key, subject = alias),
//!   then the root ([`Caller`]). A session is a host; a user is a peer key.
//!   Errors are answered with the messages of [`DirectoryError`].
//! - **Snippets.** Every member serves
//!   `strims.network.v1.directory.DirectorySnippet.Subscribe` on vpn port
//!   14 (the vnic snippet port, outside the hash table). A subscription to
//!   a swarm is admitted while the swarm's transfer under the empty salt is
//!   published on that network and the member holds a snippet for it. The
//!   stream sends the whole snippet as deltas from the empty snippet, then
//!   a delta per change, each split by [`snippet::split`]. The server
//!   subscribes to one publisher of each media listing and shows the
//!   snippet once its signature verifies.
//! - **Events.** The event swarm carries `EventBroadcast` records, the
//!   asset swarm one `AssetBundle` record, rewritten from the swarm's start
//!   whenever it changes. Records are chunkstream records (a uvarint
//!   length, then the message) in frames of [`EVENT_FRAME`] and
//!   [`ASSET_FRAME`] bytes. Readers start at the swarm's window tail, so
//!   the first record they read may be partial or the tail of an earlier
//!   one; a record that doesn't decode is skipped.
//!
//! # Event semantics
//!
//! The server sends events every broadcast interval, for what changed
//! since the last broadcast:
//!
//! - `ListingChange` (the listing, its snippet and moderation) for a new or
//!   modified listing, followed by its `UserCountChange`;
//! - `UserCountChange` for a listing whose users changed;
//! - `Unpublish` for a listing with no publishers left, or banned;
//! - `UserPresenceChange` with `online` for a user whose listings or alias
//!   changed: the sorted ids of every listing the user's hosts publish or
//!   view; without `online` when the user's last session ended.
//!
//! Every refresh interval it sends the whole state again, paced across
//! broadcasts, so that a reader that started late or lost its place
//! catches up. A reader that loses events (a discontinuity in the event
//! stream) resets its view and rebuilds it from the stream.
//!
//! # Lease
//!
//! Among the processes sharing a store, the holder of the lease
//! [`lease_key`] serves the network's directory.
//!
//! # Running
//!
//! [`DirectoryControl`] runs the directory of every network a node has
//! joined, on the node the driver supplies through [`Host`], beside the
//! node's [`NetworkControl`](strims_network::NetworkControl) and
//! [`TransferControl`](strims_transfer::TransferControl). The driver runs
//! [`DirectoryControl::run`] as a local task. For each joined network:
//!
//! - A node holding the network's server config and the lease serves: it
//!   creates both swarms, publishes their transfers and its address,
//!   writes the asset bundle and every broadcast, and feeds its own view
//!   the broadcasts it writes. Any other node reads both swarms into its
//!   view and asset bundle. A server config gained or lost later switches
//!   the mode; every switch starts the view over.
//! - Once the profile's membership is trusted, the node serves snippets
//!   on [`SNIPPET_PORT`] and calls the server from that port. Its calls to
//!   a server it runs itself go to that server directly, as its host
//!   certificate.
//! - The node pings the server on the schedule the asset bundle's client
//!   config sets, and right after its certificate changes, which keeps
//!   its sessions and updates its alias.
//! - A server loads the snippets of its embed listings from their
//!   services' APIs ([`embed`]) through the [`Fetch`] the driver gives the
//!   control ([`DirectoryControl::with_fetch`]). With none, embed
//!   listings go without snippets.
//!
//! What changes is reported as [`DirectoryEvent`]s; the views and asset
//! bundles can be read at any time. [`bind_frontend`] serves them, and the
//! calls, to the UI. A [`ListingKeeper`] keeps one listing of this host's
//! published for as long as its owner runs it ([`keeper`]).
//!
//! # Bounds
//!
//! Callers, publishers and the server may be hostile. Everything a request
//! can make the server hold is bounded by [`Limits`] and the constants of
//! [`listing`], [`snippet`] and [`server`]; everything an event can make a
//! view hold, by [`ViewLimits`]. Over a bound, a request fails with
//! [`DirectoryError::Full`] or [`DirectoryError::TooLarge`], and an event
//! is dropped and counted in [`ViewMetrics`].

mod control;
pub mod embed;
mod events;
pub mod fetch;
pub mod frontend;
mod host;
pub mod keeper;
pub mod listing;
pub mod moderation;
pub mod server;
mod services;
pub mod snippet;
pub mod swarms;
pub mod view;

#[cfg(test)]
mod server_test;
#[cfg(test)]
mod sim_test;
#[cfg(test)]
mod view_test;

use strims_dao::key::Part;
use strims_rpc::RetryPolicy;
use strims_rpc::vpn::VpnConfig;

pub use control::{CallFailure, DirectoryControl, DirectoryNetwork};
pub use embed::{EmbedService, EmbedServices};
pub use events::{DirectoryEvent, ListingEvent, UserEventKind};
pub use fetch::{Fetch, FetchError, HttpFetch, HttpRequest, HttpResponse, Method, NoFetch};
pub use frontend::{DirectoryFrontend, bind_frontend};
pub use host::{Host, Net};
pub use keeper::{KeeperConfig, KeeperMetrics, ListingKeeper};
pub use server::{Caller, LoadId, Server, ServerAction, ServerMetrics, SubId, Tunables};
pub use snippet::LocalSnippets;
pub use swarms::{
    ADDRESS_SALT, ASSET_FRAME, ASSET_SALT, EVENT_FRAME, EVENT_SALT, asset_options, event_options,
};
pub use view::{View, ViewEvent, ViewMetrics};

/// The vpn port members serve snippets on and the server subscribes at.
/// It must stay the vnic snippet port.
pub const SNIPPET_PORT: u16 = strims_vnic::ports::SNIPPET_PORT;

/// The longest peer key a caller or event may carry.
pub const MAX_PEER_KEY_BYTES: usize = 64;

/// The longest alias a caller or event may carry: the longest subject the
/// network CA issues.
pub const MAX_ALIAS_BYTES: usize = 256;

/// The key of the lease whose holder serves network `network_id`'s
/// directory.
pub fn lease_key(network_id: u64) -> String {
    strims_dao::lease::key_parts(&[Part::Str("directory"), Part::Id(network_id)])
}

/// What a [`Server`] holds at most.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Listings. A publish of a new listing beyond this fails with
    /// [`DirectoryError::Full`].
    pub max_listings: usize,
    /// Users (peer keys with a session).
    pub max_users: usize,
    /// Sessions (hosts), across users.
    pub max_sessions: usize,
    /// Sessions of one user. A new one beyond this ends the user's least
    /// recently active session.
    pub max_sessions_per_user: usize,
    /// Distinct users a listing counts as recent. Beyond this the least
    /// recent is forgotten.
    pub max_recent_users: usize,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            max_listings: 256,
            max_users: 4096,
            max_sessions: 8192,
            max_sessions_per_user: 8,
            max_recent_users: 1024,
        }
    }
}

/// What a [`View`] holds at most. What an event adds beyond these is
/// dropped and counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewLimits {
    /// Listings.
    pub max_listings: usize,
    /// Users.
    pub max_users: usize,
    /// Listing ids of one presence event; the lowest are kept.
    pub max_listing_ids: usize,
    /// Listing ids across users.
    pub max_memberships: usize,
    /// The encoded size of a listing with its snippet and moderation.
    /// Above it the snippet's images are dropped, then the snippet.
    pub max_listing_bytes: usize,
    /// Events waiting for [`View::poll_event`], by count and by size.
    /// Beyond either, events are dropped until the queue drains, and the
    /// queue then reports [`ViewEvent::Lagged`].
    pub max_queued_events: usize,
    pub max_queued_bytes: usize,
}

impl Default for ViewLimits {
    fn default() -> ViewLimits {
        ViewLimits {
            max_listings: 1024,
            max_users: 16384,
            max_listing_ids: 256,
            max_memberships: 128 * 1024,
            max_listing_bytes: 128 * 1024,
            max_queued_events: 64 * 1024,
            max_queued_bytes: 8 * 1024 * 1024,
        }
    }
}

/// How the directory runs on a node.
#[derive(Clone, Debug)]
pub struct Config {
    /// The server's bounds.
    pub limits: Limits,
    /// Each view's bounds.
    pub view: ViewLimits,
    /// Retries for calls to the server.
    pub call_policy: RetryPolicy,
    /// The server's vpn connection.
    pub server_vpn: VpnConfig,
    /// The vpn connection on [`SNIPPET_PORT`], which serves snippets and
    /// calls the server.
    pub member_vpn: VpnConfig,
    /// The longest event record a reader takes. It holds the largest
    /// broadcast record ([`server::MAX_BROADCAST_RECORD`], or one event
    /// above it) with room to spare.
    pub max_event_record: usize,
    /// The longest asset record a reader takes; it holds
    /// [`server::MAX_ASSET_BUNDLE`].
    pub max_asset_record: usize,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            limits: Limits::default(),
            view: ViewLimits::default(),
            call_policy: RetryPolicy::vpn(),
            server_vpn: VpnConfig::default(),
            member_vpn: VpnConfig::default(),
            max_event_record: 2 * 1024 * 1024,
            max_asset_record: 1024 * 1024,
        }
    }
}

/// What a network's directory has done on this node.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// The server's, while this node serves.
    pub server: Option<ServerMetrics>,
    pub view: ViewMetrics,
    /// Times this node started serving.
    pub serves: u64,
    /// Pings sent, and those that failed.
    pub pings: u64,
    pub ping_errors: u64,
    /// Event stream discontinuities, each of which started the view over.
    pub discontinuities: u64,
    /// Asset bundles taken.
    pub asset_bundles: u64,
    /// Snippet subscriptions served, and those refused.
    pub snippets_served: u64,
    pub snippets_refused: u64,
    /// Embed snippet batches the server loaded, and those that failed.
    pub embed_batches: u64,
    pub embed_batch_errors: u64,
    /// Node and store operations that failed.
    pub host_errors: u64,
    pub store_errors: u64,
}

/// Why a directory request failed. Callers receive the message as the RPC
/// error, so the messages are part of the wire format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DirectoryError {
    #[error("listing not found")]
    ListingNotFound,
    #[error("session not found")]
    SessionNotFound,
    #[error("publishing from this account is disabled")]
    PublishDisabled,
    #[error("joining from this account is disabled")]
    JoinDisabled,
    #[error("unsupported embed service")]
    UnsupportedEmbed,
    /// An embed's id isn't of its service's form.
    #[error("invalid embed id")]
    InvalidEmbedId,
    #[error("invalid swarm uri")]
    InvalidSwarmUri,
    #[error("unsupported content type")]
    UnsupportedContent,
    #[error("listing banned")]
    ListingBanned,
    #[error("exceeded concurrent publish quota")]
    PublishQuota,
    #[error("exceeded concurrent view quota")]
    ViewQuota,
    #[error("not publishing this listing")]
    NotPublishing,
    #[error("not viewing this listing")]
    NotViewing,
    #[error("permission denied")]
    PermissionDenied,
    /// The server holds as many listings, users or sessions as it may.
    #[error("directory full")]
    Full,
    /// A field of the request is over its bound.
    #[error("request too large")]
    TooLarge,
}

impl DirectoryError {
    /// Every error.
    const ALL: [DirectoryError; 16] = [
        DirectoryError::ListingNotFound,
        DirectoryError::SessionNotFound,
        DirectoryError::PublishDisabled,
        DirectoryError::JoinDisabled,
        DirectoryError::UnsupportedEmbed,
        DirectoryError::InvalidEmbedId,
        DirectoryError::InvalidSwarmUri,
        DirectoryError::UnsupportedContent,
        DirectoryError::ListingBanned,
        DirectoryError::PublishQuota,
        DirectoryError::ViewQuota,
        DirectoryError::NotPublishing,
        DirectoryError::NotViewing,
        DirectoryError::PermissionDenied,
        DirectoryError::Full,
        DirectoryError::TooLarge,
    ];

    /// The error whose message is `message`.
    pub fn from_message(message: &str) -> Option<DirectoryError> {
        DirectoryError::ALL
            .into_iter()
            .find(|e| e.to_string() == message)
    }
}

/// Why a snippet was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SnippetError {
    /// The publisher holds [`snippet::MAX_LOCAL_SNIPPETS`] snippets.
    #[error("too many snippets")]
    Full,
    /// An image, the text or the whole snippet is over its bound.
    #[error("snippet too large")]
    TooLarge,
    /// The signature doesn't verify against the snippet's key.
    #[error("invalid snippet signature")]
    InvalidSignature,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_are_bounded_by_the_ca_subject_limit() {
        assert_eq!(MAX_ALIAS_BYTES, strims_network::ca::MAX_SUBJECT_LEN);
    }

    #[test]
    fn the_lease_key_names_the_directory_and_network() {
        assert_eq!(
            lease_key(36),
            strims_dao::lease::key_parts(&[Part::Str("directory"), Part::Id(36)])
        );
        assert!(lease_key(36).ends_with(":directory:10"));
    }

    #[test]
    fn readers_take_the_largest_records() {
        let c = Config::default();
        assert!(c.max_event_record >= 2 * server::MAX_EVENT_BYTES);
        assert!(c.max_event_record >= server::MAX_BROADCAST_RECORD);
        assert!(c.max_asset_record >= server::MAX_ASSET_BUNDLE);
    }

    #[test]
    fn error_messages_match_the_wire() {
        assert_eq!(
            DirectoryError::ListingNotFound.to_string(),
            "listing not found"
        );
        assert_eq!(
            DirectoryError::PublishQuota.to_string(),
            "exceeded concurrent publish quota"
        );
        assert_eq!(DirectoryError::Full.to_string(), "directory full");
    }

    #[test]
    fn every_error_is_found_by_its_message() {
        for e in DirectoryError::ALL {
            assert_eq!(DirectoryError::from_message(&e.to_string()), Some(e));
        }
        assert_eq!(DirectoryError::from_message("listing not found!"), None);
    }

    #[test]
    fn a_remote_directory_error_is_a_refusal() {
        use strims_proto::service::{CallError, Error as RpcError};
        let remote = |m: &str| CallError::Remote(RpcError::new(0, m));
        assert_eq!(
            CallFailure::from_call(remote("listing banned")),
            CallFailure::Refused(DirectoryError::ListingBanned)
        );
        let other = RpcError::new(3, "something else");
        assert_eq!(
            CallFailure::from_call(CallError::Remote(other.clone())),
            CallFailure::Remote(other.clone())
        );
        assert_eq!(CallFailure::Remote(other.clone()).to_rpc(), other);
        assert_eq!(
            CallFailure::Refused(DirectoryError::PermissionDenied).to_rpc(),
            RpcError::new(0, "permission denied")
        );
        assert_eq!(
            CallFailure::from_call(CallError::Timeout),
            CallFailure::Call(CallError::Timeout)
        );
    }
}
