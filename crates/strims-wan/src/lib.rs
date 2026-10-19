// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Wide-area paths between simulated hosts, approximating the public
//! internet.
//!
//! A host sits at a [`Site`]: a [`City`] and an [`Access`] link. A message
//! from one site to another
//!
//! 1. waits for the sender's uplink and serializes at its rate (FIFO over
//!    every message the host sends);
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
//! | module | role |
//! |---|---|
//! | [`city`] | cities and their RTT table |
//! | [`access`] | access link profiles, sites and weighted mixes |
//! | [`network`] | uplink and downlink FIFOs, paths, jitter, loss |
//! | [`sample`] | seeded draws |
//! | [`wire`] | packet framing and serialization times |

pub mod access;
pub mod city;
pub mod network;
pub mod sample;
pub mod wire;

pub use access::{Access, GLOBAL_AUDIENCE, Profile, RESIDENTIAL, Site};
pub use city::{City, Region};
pub use network::{Carrier, EndStats, Network, Options, Transit};
