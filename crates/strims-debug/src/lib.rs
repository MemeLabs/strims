// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Debugging a node: mock streams on a network's directory, and the UI's
//! debug service.
//!
//! A mock stream is a swarm a node writes timestamped segments into at a
//! chosen bitrate, listed on a network's directory. Nodes whose debug
//! config names the network read the mock streams listed there and report
//! each one's latency, which measures how a live stream crosses the
//! network.
//!
//! # Format
//!
//! - **Swarm.** [`mock_stream_options`]: 4 KiB chunks, 8 per signature,
//!   16 streams, a 4 Ki-chunk (16 MiB) live window,
//!   Merkle/BLAKE3/Ed25519, under a fresh ed25519 key. Its transfer
//!   is under the empty salt.
//! - **Listing.** A `Service` listing of type [`MOCK_STREAM_SERVICE`]
//!   whose swarm URI carries every swarm option but the window.
//! - **Segments.** Each is one `strims.debug.v1.MockStreamSegment`
//!   chunkstream record ([`strims_transfer::RecordWriter`]) in
//!   [`MOCK_FRAME`] byte frames, the swarm's integrity unit, so each write
//!   publishes a whole segment. A segment holds the stream's id, the time
//!   it was written in Unix microseconds, and [`padding_len`] zero bytes:
//!   the bitrate's bytes for one segment interval, so the stream holds its
//!   bitrate at any interval. A segment encodes to at most
//!   [`MAX_SEGMENT_BYTES`].
//!
//! # Producing
//!
//! [`DebugControl::start_mock_stream`] starts a mock stream on a network
//! in the profile's store, within the [`Config`] bounds. The stream
//! creates its swarm, adds its transfer and publishes it on the network
//! once the node has joined it, and keeps its listing there with a
//! [`ListingKeeper`](strims_directory::ListingKeeper). Segment `k` is due
//! at the stream's start plus `k` intervals; a segment written late is
//! followed by the next one due after it, so the ticks it missed are
//! skipped.
//!
//! A stream ends when it is stopped ([`DebugControl::stop_mock_stream`]),
//! at its timeout, when a write fails, or when its network is deleted from
//! the store. Its end stops the keeper, which unpublishes the listing,
//! then removes the transfer, then the swarm. Streams belong to the
//! control: they outlive the call that started them. A stream's id is
//! drawn from the control's rng: nonzero, and unique among the node's
//! streams.
//!
//! # Reading
//!
//! The device's debug config ([`strims_dao::schema::debug::DebugConfig`])
//! enables reading and names a network. While it does, the node reads the
//! mock streams listed in that network's directory view: the `Service`
//! listings of type [`MOCK_STREAM_SERVICE`] whose swarm URI carries the
//! mock stream options, the lowest listing ids first, at most
//! [`Config::max_readers`]. A swarm the node already runs, its own mock
//! streams' included, isn't read, and a swarm is read once.
//!
//! A reader adds the swarm with a [`MOCK_WINDOW_BYTES`] window, adds its
//! transfer and publishes it on the network, and reads segments. Each
//! segment's latency is the reader's clock less the segment's timestamp,
//! so clock skew can make it negative. Records that don't decode as
//! segments and the stream's discontinuities are counted.
//!
//! The readers follow the config and the view: the config's changes and
//! the view's listings, as the directory reports them or as a read of the
//! view finds them when the config changes. A reader ends when its listing
//! leaves what the node reads (the listing goes, the config disables
//! reading or names another network, the network is deleted, which clears
//! the config's key), when its swarm closes, or when a read or node call
//! fails. Its end removes the transfer, then the swarm. A listing whose
//! reader ended by itself isn't read again until the listing changes, the
//! network's directory starts over, or the config changes.
//!
//! # Running
//!
//! [`DebugControl`] runs on the node the driver supplies through
//! [`strims_directory::Host`], beside the node's
//! [`NetworkControl`](strims_network::NetworkControl),
//! [`TransferControl`](strims_transfer::TransferControl) and
//! [`DirectoryControl`](strims_directory::DirectoryControl). The driver
//! runs [`DebugControl::run`] as a local task.
//!
//! # Frontend
//!
//! [`bind_frontend`] serves the UI's `strims.debug.v1.Debug` service on
//! the control: the device's debug config, the mock streams, and metrics
//! snapshots. A snapshot is what the driver's [`MetricsSource`] reports,
//! with the time it was taken and the control's [`Metrics`], mock streams
//! and readers; see [`frontend`].
//!
//! # Bounds
//!
//! [`Config`] holds every bound, each documented where it is declared.
//! Collections are bounded where they are declared.

mod control;
mod error;
pub mod frontend;
mod producer;
mod reader;

use strims_base::time::Duration;
use strims_directory::KeeperConfig;

pub use control::{DebugControl, MockStreamInfo, ReaderInfo};
pub use error::DebugError;
pub use frontend::{DebugFrontend, MetricsSource, NoMetrics, bind_frontend};
pub use producer::{mock_stream_options, padding_len};

/// The service type of a mock stream's listing.
pub const MOCK_STREAM_SERVICE: &str = "mock_stream";

/// The frame size of a mock stream's records: its swarm's integrity unit,
/// 32 chunks of 1 KiB.
pub const MOCK_FRAME: usize = 32 * 1024;

/// The most bytes a mock stream segment encodes to, and the largest record
/// a reader takes.
pub const MAX_SEGMENT_BYTES: usize = 2 * 1024 * 1024;

/// A reader's live window, in bytes.
pub const MOCK_WINDOW_BYTES: usize = 16 * 1024 * 1024;

/// How the debug control runs, and the bounds on what callers make it
/// hold or do.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Mock streams the node writes at once, those ending included.
    pub max_mock_streams: usize,
    /// Mock streams the node reads at once, those ending included. Each
    /// holds a [`MOCK_WINDOW_BYTES`] window.
    pub max_readers: usize,
    /// The highest bitrate a mock stream takes, in kbps. 0 writes segments
    /// without padding.
    pub max_bitrate_kbps: u32,
    /// The shortest and longest segment interval a mock stream takes.
    pub min_segment_interval: Duration,
    pub max_segment_interval: Duration,
    /// The interval of a request that names none.
    pub default_segment_interval: Duration,
    /// The longest timeout a mock stream takes.
    pub max_timeout: Duration,
    /// The shortest and longest interval between a metrics watch's
    /// snapshots.
    pub min_watch_interval: Duration,
    pub max_watch_interval: Duration,
    /// The interval of a watch that names none.
    pub default_watch_interval: Duration,
    /// How the mock streams' listing keepers pace their calls.
    pub keeper: KeeperConfig,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            max_mock_streams: 4,
            max_readers: 4,
            max_bitrate_kbps: 50_000,
            min_segment_interval: Duration::from_millis(100),
            max_segment_interval: Duration::from_secs(10),
            default_segment_interval: Duration::SECOND,
            max_timeout: Duration::HOUR * 24,
            min_watch_interval: Duration::from_millis(250),
            max_watch_interval: Duration::MINUTE,
            default_watch_interval: Duration::SECOND,
            keeper: KeeperConfig::default(),
        }
    }
}

/// What the debug control has done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Mock streams whose swarm was created, and those that ended since.
    pub mock_streams_started: u64,
    pub mock_streams_ended: u64,
    /// Segments the node's mock streams wrote, and writes that failed.
    pub segments_written: u64,
    pub write_errors: u64,
    /// Readers started, and those that ended since.
    pub readers_started: u64,
    pub readers_ended: u64,
    /// Segments the readers read, and reads that failed.
    pub segments_read: u64,
    pub read_errors: u64,
    /// Node calls that failed: creating, adding or removing a swarm, or
    /// opening its reader.
    pub host_errors: u64,
    /// Store reads that failed while following the debug config.
    pub store_errors: u64,
    /// Times the store's events lagged and the config was read again.
    pub store_lags: u64,
}
