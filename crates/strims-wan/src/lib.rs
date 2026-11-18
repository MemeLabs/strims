// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Wide-area paths between simulated hosts, approximating the public
//! internet.
//!
//! A host sits at a [`Site`]: a [`City`] and an [`Access`] link. A message
//! from one site to another
//!
//! 1. waits for the sender's uplink and serializes at its rate (FIFO over
//!    every message the host sends), or with a shaper bucket
//!    ([`Bucket`]) passes what its tokens cover at once;
//! 2. crosses the path: both last miles plus half the city pair's RTT
//!    ([`City::rtt`]), plus each end's jitter;
//! 3. may lose packets: a lost datagram is gone, a stream message arrives a
//!    retransmission later and in order;
//! 4. waits for the receiver's downlink and serializes at its rate (FIFO in
//!    arrival order).
//!
//! [`Network`] holds that state for one run. Every draw comes from rngs
//! derived from the one it is built with, so a seed replays exactly.
//!
//! [`live`] is what a live stream's seed writes: a live encoder's access
//! units, and an ingest that signs them early. [`playout`] is how a
//! viewer's player plays what it reads.
//! [`vocab`] is the sims' event vocabulary, generated from `vocab.toml`:
//! the kinds their `events` tables hold.
//!
//! | module | role |
//! |---|---|
//! | [`city`] | cities and their RTT table |
//! | [`framing`] | what an access link charges a packet past its IP bytes |
//! | [`live`] | a live encoder's access units and early signing |
//! | [`access`] | access link profiles, shaper buckets, sites and weighted mixes |
//! | [`capacity`] | an uplink capacity as a schedule of delivery opportunities |
//! | [`network`] | uplink and downlink FIFOs, uplink buckets, paths, jitter, loss |
//! | [`playout`] | a live player's stalls at a fixed delay |
//! | [`replay`] | an uplink replayed packet by packet, the shaper benched open loop |
//! | [`sample`] | seeded draws |
//! | [`slots`] | medium access in slots (a cable modem's grants, a Wi-Fi station's aggregates) |
//! | [`testbed`] | the Docker testbed's access link specs, for its links and the sims' twins |
//! | [`wire`] | packet framing and serialization times |
//! | [`vocab`] | the sims' event vocabulary (`vocab.toml`), and the kinds of their `events` tables |

pub mod access;
pub mod capacity;
pub mod city;
pub mod framing;
pub mod live;
pub mod network;
pub mod playout;
pub mod replay;
pub mod sample;
pub mod slots;
pub mod testbed;
#[rustfmt::skip]
pub mod vocab;
pub mod wire;

pub use access::{Access, Bucket, GLOBAL_AUDIENCE, Profile, RESIDENTIAL, Site};
pub use city::{City, Region};
pub use network::{Carrier, EndStats, Network, Options, Transit};
