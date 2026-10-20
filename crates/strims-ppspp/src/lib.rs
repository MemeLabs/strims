// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Sans-IO PPSPP (RFC 7574) for live streams: the v2 wire format, content
//! integrity, the live chunk store, and a multi-peer runner with pull and
//! push/pull scheduling under a per-channel request window.
//!
//! The crate does no I/O, spawns nothing and reads no clock. A driver feeds
//! it frames and time and drains frames and events, so the same code runs
//! natively, in wasm and under deterministic simulation.
//!
//! # Modules
//!
//! - [`codec`]: the wire format. Decoding yields borrowed
//!   [`codec::Message`] views; encoders write into caller buffers.
//! - [`merkle`]: the hash tree.
//! - [`integrity`]: Merkle, sign-all and no content integrity, with ed25519
//!   or no live signatures.
//! - [`store`]: the preallocated live-window chunk buffer and its
//!   non-blocking readers.
//! - [`Swarm`] owns a stream's store, verifier and [`Epoch`]; a [`Writer`]
//!   publishes into it; [`SwarmId`], [`Uri`] and [`SwarmOptions`] name and
//!   configure it.
//! - [`Runner`]: swarms, peers and their channels behind keys; the driver
//!   surface.
//! - [`channel`]: handshake validation and per-message counters.
//! - [`scheduler_peer`]: the pull scheduler, and in
//!   [`scheduler_peer::push`] the push/pull hybrid selected by [`PushMode`].
//! - [`controller`]: the per-channel request window, [`ledbat`] (RFC 6817)
//!   or [`bbr::BbrLike`], selected by [`SwarmOptions::request_controller`].
//!
//! # Driving the runner
//!
//! A [`Runner`] serves one node. The driver adds peers and runs swarm
//! channels on them, passes each frame body from a peer to
//! [`Runner::handle_frame`], fills outgoing frames with
//! [`Runner::poll_transmit`] while [`Runner::wants_transmit`], calls
//! [`Runner::handle_timeout`] at [`Runner::poll_timeout`], and drains
//! [`Runner::poll_event`] after every call: transmit backlogs and upload
//! weights go to its transport, readable swarms wake their readers. Local
//! chunks are published through [`Runner::write`] and reach the scheduler
//! before it returns. [`Runner`]'s docs give the full contract.
//!
//! # Hostile peers
//!
//! Peer input is untrusted. Decoding never panics, and malformed input
//! fails decoding or verification. Every bin from the wire passes [`peer_bin_ok`] (below
//! [`MAX_BIN`], at most a window of chunks) or a narrower check before it
//! reaches a buffer index or a binmap walk. Every per-message path costs at
//! most O(live window), usually O(new bins), and every collection a peer
//! can grow is bounded: HAVE history to two windows, outstanding requests
//! and pending signatures to one window, data queues to [`DATA_QUEUE_CAP`]
//! entries per priority, extra messages to a fixed set per stream. A peer
//! that announces chunks and withholds them, or sends invalid data, only
//! delays them: they go to the other channels while the failing one backs
//! off. [`scheduler_peer`] lists the scheduler's bounds;
//! `tests/codec_fuzz.rs` and `tests/runner_fuzz.rs` check them.
//!
//! # Compatibility
//!
//! The wire format, signed messages, Merkle digests, swarm ids and URIs
//! must stay byte-identical to protocol v2 plus ANNOUNCE (message type
//! 18, the reports an adaptive seed asks for); `tests/golden.rs` pins
//! them.
//! Local options ([`PushMode`], the request controller, the delivery and
//! scheduling methods) never reach the wire, so nodes with different local
//! options interoperate.

pub mod bbr;
mod bin_queue;
pub mod channel;
pub mod codec;
mod constants;
pub mod controller;
mod epoch;
pub mod integrity;
pub mod ledbat;
pub mod merkle;
mod options;
mod peer;
mod peer_data_queue;
mod peer_writer_queue;
mod runner;
mod scheduler;
pub mod scheduler_peer;
pub mod store;
mod swarm;
mod swarm_id;
mod tap;
mod time_set;
mod uri;
mod writer;

pub use constants::{MINIMUM_PROTOCOL_VERSION, PROTOCOL_VERSION};
pub use controller::RequestControllerKind;
pub use epoch::{Epoch, EpochError};
pub use options::{
    DeliveryMode, PushMode, SchedulingMethod, SeedAllocation, SeedPolicy, SeedQos, SwarmOptions,
};
pub use swarm::{Published, Swarm, SwarmError, SwarmEvent, SwarmHandshake};
pub use swarm_id::{SwarmId, SwarmIdError};
pub use uri::{Uri, UriError, UriOptions};
pub use writer::{Writer, WriterOptions};

pub use channel::{ChannelMetrics, HandshakeError};
pub use peer::{MetricsSnapshot, PeerMetricsSnapshot, PeerStats, TransferMetricsSnapshot};
pub use peer_data_queue::DATA_QUEUE_CAP;
pub use runner::{
    ChannelStats, Event, MAX_PEER_QOS_WEIGHT, MIN_PEER_QOS_WEIGHT, PEER_QOS_UPDATE_INTERVAL,
    PeerKey, Runner, RunnerError, SwarmKey, SwarmMut, TICK_INTERVAL,
};
pub use scheduler_peer::push::PushStats;
pub use scheduler_peer::push::seed::{
    AllocCounters, ReportCounters, SeedAllocationStats, SeedNeighbour,
};
pub use scheduler_peer::{ChannelSchedulerStats, SwarmSchedulerStats};
pub use tap::{TAP_CAP, TapEvent, TapKind, TapLevel};

/// The first bin a peer may name. Bins at or past it start at chunk 2^61,
/// which no stream reaches; refusing them keeps every binmap's root below
/// [`Bin::ALL`](strims_binmap::Bin::ALL), where `strims-binmap` documents
/// the quirks of a full bin space.
pub const MAX_BIN: strims_binmap::Bin = strims_binmap::Bin(1 << 62);

/// A bin from a peer is usable if it is below [`MAX_BIN`] and spans at
/// most `max_chunks` chunks. Checked before any bin from the wire reaches a
/// buffer index or a binmap walk.
pub fn peer_bin_ok(b: strims_binmap::Bin, max_chunks: u64) -> bool {
    b < MAX_BIN && b.base_length() <= max_chunks
}
