// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! How the chat control runs, and what it counts.

use strims_base::time::Duration;
use strims_rpc::RetryPolicy;
use strims_rpc::vpn::VpnConfig;

use crate::ServerLimits;
use crate::reader::RoomLimits;

/// The most servers of the profile a control runs at once. A record past
/// it is left unserved, and counted in [`Metrics::servers_dropped`],
/// until it changes while there is room.
pub const MAX_SERVED: usize = 64;
/// The most rooms a control reads without owning their server.
pub const MAX_ROOMS: usize = 32;
/// The most client streams open on one room.
pub const MAX_STREAMS_PER_ROOM: usize = 16;

/// How the chat control runs on a node.
#[derive(Clone, Debug)]
pub struct Config {
    /// What each server this node serves takes and holds.
    pub server: ServerLimits,
    /// What each client stream decodes and holds of a room's records.
    pub room: RoomLimits,
    /// Servers served at once; see [`MAX_SERVED`].
    pub max_served: usize,
    /// Rooms read without owning their server; see [`MAX_ROOMS`].
    pub max_rooms: usize,
    /// Client streams per room; see [`MAX_STREAMS_PER_ROOM`].
    pub max_streams_per_room: usize,
    /// How long a room's swarms stay after its last client stream closes,
    /// so that a stream opened again soon finds them.
    pub linger: Duration,
    /// The wait after a change to a server's assets in the store before
    /// they are published, so that a burst of changes makes one bundle.
    pub asset_debounce: Duration,
    /// The shortest wait between two publishes of a server's listing.
    pub min_republish: Duration,
    /// How often a server checks that the directory still holds its
    /// listing.
    pub listing_check: Duration,
    /// How long a server that stops waits for its listing's unpublish.
    pub unpublish_timeout: Duration,
    /// Retries for `SendMessage` calls to a remote server. An attempt may
    /// reach the server whatever its outcome here, so the policy must not
    /// retry.
    pub send_policy: RetryPolicy,
    /// Retries for the other calls to a remote server.
    pub call_policy: RetryPolicy,
    /// The vpn connection each served server answers calls on.
    pub server_vpn: VpnConfig,
    /// The vpn connection per network that carries this node's calls to
    /// remote servers.
    pub member_vpn: VpnConfig,
    /// The most events a client stream yields in one batch.
    pub max_batch_events: usize,
    /// The encoded bytes a client stream's batch grows to before it is
    /// yielded; a batch may pass it by one event.
    pub max_batch_bytes: usize,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            server: ServerLimits::default(),
            room: RoomLimits::default(),
            max_served: MAX_SERVED,
            max_rooms: MAX_ROOMS,
            max_streams_per_room: MAX_STREAMS_PER_ROOM,
            linger: Duration::from_secs(30),
            asset_debounce: Duration::from_millis(250),
            min_republish: Duration::from_secs(5),
            listing_check: Duration::from_secs(60),
            unpublish_timeout: Duration::from_secs(5),
            send_policy: RetryPolicy {
                attempt_timeout: Duration::from_secs(30),
                retries: 0,
                ..RetryPolicy::vpn()
            },
            call_policy: RetryPolicy::vpn(),
            server_vpn: VpnConfig::default(),
            member_vpn: VpnConfig::default(),
            max_batch_events: 256,
            max_batch_bytes: 4 * 1024 * 1024,
        }
    }
}

/// What the chat control has done.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Times a server started serving, and times a room started reading.
    pub serves: u64,
    pub reads: u64,
    /// Events written to served servers' event streams.
    pub events_written: u64,
    /// Asset bundles written, and those among them that started the
    /// stream over.
    pub asset_bundles: u64,
    pub asset_resets: u64,
    /// Asset syncs that wrote nothing because the unified bundle is over
    /// the stream's window.
    pub assets_too_large: u64,
    /// Listing publishes that succeeded, and those that failed.
    pub listing_publishes: u64,
    pub listing_errors: u64,
    /// Calls made to a server this node serves, and to remote servers.
    pub local_calls: u64,
    pub remote_calls: u64,
    /// Owned servers left unserved for [`Config::max_served`].
    pub servers_dropped: u64,
    /// Client streams refused for [`Config::max_rooms`] or
    /// [`Config::max_streams_per_room`].
    pub streams_refused: u64,
    /// Swarm writes, node operations and store operations that failed.
    pub write_errors: u64,
    pub host_errors: u64,
    pub store_errors: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::{MAX_ASSET_RECORD, MAX_EVENT_RECORD};
    use crate::swarms::ASSET_WINDOW_BYTES;

    #[test]
    fn sends_are_never_retried() {
        assert_eq!(Config::default().send_policy.retries, 0);
    }

    #[test]
    fn readers_take_whole_windows_of_assets() {
        let c = Config::default();
        assert_eq!(c.room.max_asset_record, MAX_ASSET_RECORD);
        assert!(c.room.max_asset_record >= ASSET_WINDOW_BYTES);
        assert_eq!(c.room.max_event_record, MAX_EVENT_RECORD);
    }
}
