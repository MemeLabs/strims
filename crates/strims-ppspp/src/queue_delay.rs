// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Queueing delay on the node's uplink, from ping round trips.
//!
//! A channel pings once a second for its request controller, and in a
//! hybrid swarm every [`METER_PING`] while it sends data, for the meter
//! alone: with a sample
//! every few hundred milliseconds the least of the recent ones finds the
//! standing queue between bursts.
//!
//! A ping's nonce is our clock when its frame was built, so its round trip
//! includes whatever waits past the runner: the driver's link buffers, the
//! kernel's unsent bytes and congestion window, the access network's queue.
//! A queue on our uplink delays every peer's round trip by about the same
//! amount; one on a peer's path (its downlink, its own upload of the pong)
//! delays only that peer's. So the node's queueing delay is a low quantile,
//! across peers, of how far each peer's recent round trips stand above its
//! base: a slow neighbour, or a minority that delays its pongs, can't move
//! it.
//!
//! Each peer's round trips go into a [`PathDelay`], which keeps the base
//! (the least round trip over ten minutes) and the recent minimum, and
//! [`aggregate`] takes the low quantile. A channel's first ping goes out
//! with its handshake, before any data, so a queue that builds later reads
//! as excess.

pub(crate) use strims_base::path::{PathDelay, aggregate};
use strims_base::time::Duration;

/// While a channel sends data it also pings this often, for the meter.
pub(crate) const METER_PING: Duration = Duration::from_millis(200);
