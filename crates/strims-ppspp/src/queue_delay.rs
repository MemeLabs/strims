// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Queueing delay on the node's uplink, from the driver's measure of each
//! peer's link.
//!
//! The driver probes every link ([`Runner::set_peer_path`]: vnic's probes
//! and echoes, [`strims_base::path`]). A probe leaves with the frame QoS
//! lets through, and its echo skips the peer's queues, so a link's round
//! trip covers what waits below the runner: the driver's link buffers, the
//! kernel's unsent bytes and congestion window, the access network's queue
//! and the path. A queue on our uplink delays every peer's round trip by
//! about the same amount; one on a peer's path (its downlink, its own
//! uplink) delays only that peer's. So the node's queueing delay is a low
//! quantile ([`aggregate`]), across peers, of how far each link's recent
//! round trips stand above its base ([`PathReading::excess`]): a slow
//! neighbour, or a minority that delays its echoes, can't move it. The time
//! data waits in the runner itself is the meter's busy time.
//!
//! [`Runner::set_peer_path`]: crate::Runner::set_peer_path
//! [`PathReading::excess`]: strims_base::path::PathReading::excess

pub(crate) use strims_base::path::aggregate;
