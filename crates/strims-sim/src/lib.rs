// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Deterministic turmoil scenarios for whole strims nodes.
//!
//! A [`Scenario`] describes hosts, links (TCP or WebSocket), networks with
//! their bindings, swarms with a seed and viewers, uplink caps, timed
//! [`Fault`]s, vpn probes and [`Invariants`]. [`run`](fn@run) executes it
//! on turmoil with real `strims-node` nodes on `TurmoilNet` and
//! `SimClock`, and returns a [`Report`]: per-viewer delivery and lag,
//! per-host upload, vpn delivery rates, memory, the simulated/wall time
//! ratio, and the SHA-256 of a canonical trace. [`sweep`](fn@sweep) runs a
//! scenario over many seeds at once, prints a replay command for a failing
//! seed, and checks that re-running a seed gives the same trace hash.
//!
//! # How a run composes
//!
//! - Every [`HostSpec`] is a turmoil host running one node. Its software
//!   is the node, an event policy that runs every swarm it shares with a
//!   new peer and binds [`Binding::Explicit`] networks, a supervisor that
//!   dials the [`LinkSpec`]s it is `from` while they are down (the only
//!   reconnect, so links heal after partitions and restarts), and the
//!   writer or readers of its swarms. A restart re-runs all of it from the
//!   spec.
//! - The controller is a turmoil client. It samples every node, checks the
//!   [`Invariants`], generates the [`VpnProbes`] and takes a final
//!   snapshot at [`Scenario::duration`].
//! - [`Fault`]s apply between turmoil steps, at the first step at or after
//!   their time. They are the only source of loss: turmoil's random
//!   `fail_rate` stays off.
//! - turmoil models no bandwidth; [`HostSpec::uplink`] is a run's only
//!   rate limit.
//!
//! | module | role |
//! |---|---|
//! | [`spec`] | the scenario description and its builder helpers |
//! | [`topology`] | seeded link layouts |
//! | `world` | one run's shared state (keys, host registry, stats) |
//! | `host` | each host's software: node, event policy, redials, writer and readers |
//! | `control` | the controller client: samples, vpn probes, final metrics |
//! | `run` | the turmoil step loop and the faults |
//! | [`stream`] | the self-checking stream format and per-viewer accounting |
//! | [`report`] | results and summaries |
//! | [`stats`] | the runner, link and network stats folded over a run |
//! | [`sweep`](mod@sweep) | seed sweeps, replay and the determinism check |
//! | [`memory`] | per-run heap accounting through a tracking allocator |
//!
//! # Determinism
//!
//! Everything a run does derives from its `u64` seed, so a seed replays
//! exactly, in debug and release builds alike:
//!
//! - turmoil's rng (message latencies) is seeded with it, and so is
//!   str0m's thread-local rng, once per simulation;
//! - every key (profile, swarm, network, hash table) is a function of its
//!   kind, the seed and its index, so adding a host or a swarm changes no
//!   other key;
//! - each node's `rng_seed` is a function of the seed, the host and its
//!   incarnation, so a restarted host gets a new host key;
//! - the controller's probe choices come from an rng derived from the seed,
//!   and frame payloads are a function of (seed, swarm, sequence number);
//! - scenario builders take the seed and draw topologies and fault times
//!   from it.
//!
//! The schedule stays fixed because every host, the controller and every
//! task run on the turmoil thread, shared state is `Rc<RefCell<…>>` over
//! `BTreeMap`s, and tasks spawn in a fixed order. Two inputs need care:
//!
//! - **Crashes.** A crash drops the host's sockets in an order that isn't
//!   reproducible, so [`Fault::Crash`] cuts the host off first.
//! - **Sampling.** `Node::metrics` and `Node::peers` are strims-node
//!   queries, answered after passes the node makes anyway. A query that
//!   finds no pass due (an idle node) wakes the node, which can shift when
//!   it answers later samples and so the stamps of their trace lines. The
//!   cadence ([`Scenario::sample_every`], [`Scenario::stats_every`]) is
//!   therefore part of the scenario.
//!
//! Randomness the seed doesn't reach (tungstenite's WebSocket masks,
//! WebRTC's DTLS and SCTP) changes bytes on the wire but not the bytes vnic
//! carries. The trace records vnic byte counts only, never wire bytes or
//! datagram sizes, and no memory or wall-clock figure.

mod control;
mod host;
pub mod memory;
pub mod report;
mod run;
pub mod spec;
pub mod stats;
pub mod stream;
pub mod sweep;
pub mod topology;
mod world;

pub use report::{HostReport, LookupPhase, Percentiles, Report, ViewerReport, VpnReport};
pub use run::run;
pub use spec::{
    Binding, Fault, HostIx, HostSpec, Invariants, LinkSpec, NetworkSpec, Scenario, SwarmSpec,
    Transport, VpnProbes,
};
pub use stats::{Peaks, RunnerReport, SwarmCounters};
pub use sweep::{seeds, sweep};

/// The TCP listener port of every host.
pub const TCP_PORT: u16 = 7000;
/// The WebSocket listener port of every host.
pub const WS_PORT: u16 = 7001;
/// The WebRTC UDP port of hosts with `HostSpec::webrtc`.
pub const UDP_PORT: u16 = 7002;
/// The vpn port vpn probes and their handlers use.
pub const APP_PORT: u16 = 20_000;
/// The simulation's epoch (Unix milliseconds).
pub const EPOCH_MS: i64 = 1_700_000_000_000;
