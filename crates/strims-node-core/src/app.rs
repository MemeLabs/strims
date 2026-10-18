// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The app layer on a node: the node as the host of the services a profile
//! runs, every peer's RPC link served with the app's [`PeerHandler`]s, and
//! a signed-in profile's node and controls composed on a driver's
//! [`AppPlatform`].
//!
//! # Hosts
//!
//! A profile's services share one [`NodeHost`]: replication, transfers and
//! their record streams, network membership (which adds and removes the
//! networks it uses), bootstrap (which dials the profile's bootstrap
//! clients), the directory, and the profile's vnic limits. The directory
//! control is built on the profile's network control and transfer control,
//! and its swarms move only while the transfer control learns every peer:
//! give [`PeerServices`] both controls. The chat and video controls are
//! built on those and the directory control, the whisper control on the
//! network control.
//!
//! # Composition
//!
//! A driver's `SessionFactory` delegates to two calls:
//!
//! - [`ProfileSlot::start`] starts a signed-in profile's node and every
//!   control on an [`AppPlatform`], and serves every peer through
//!   [`PeerServices`]. A slot runs one profile at a time, since the node
//!   listens on the platform's addresses: another profile's start fails
//!   with [`StartError::ProfileRunning`]. The node starts under the
//!   profile's vnic config, within [`AppPlatform::ceilings`], and follows
//!   the config's changes.
//! - [`bind_profile`] serves the session's frontends on a frontend
//!   connection's router.
//!
//! The platform supplies what differs between drivers: entropy, the node's
//! config and its spawn, the vnic ceilings, the services' configs, the
//! timer, the directory's HTTP fetcher, the video control's RTMP host, the
//! HLS routes, the task spawner, and the engine the network control runs
//! its PSI sessions on.
//!
//! # Tasks
//!
//! App-layer futures aren't `Send`, so every task runs on the platform's
//! [`Spawn`] as a local task: the controls, the peers' connections and the
//! node's event loop. Dropping a [`ProfileRuntime`] aborts its tasks and
//! hands the node to [`Spawn::shutdown`].
//!
//! # Determinism
//!
//! On a deterministic driver the same [`AppPlatform::seed`] replays the
//! same profile run. The seed builds one `DetRng`, whose first `u64` seeds
//! the node ([`crate::NodeConfig::rng_seed`]) and from which the controls
//! fork in the order replication, network, bootstrap, directory, chat,
//! whispers, video (transfers draw none). Tasks spawn in this order: the
//! controls' `run`s (replication, network, bootstrap, transfers,
//! directory, chat, whispers, video), the vnic config's follower, then the
//! event loop. Both orders are part of the run. So is the PSI engine: the
//! default [`AppPlatform::psi`] computes inline and replays, and an engine
//! whose sessions answer from another thread doesn't.

mod host;
mod peers;
mod platform;
mod profile;

pub use host::{IndexHosts, NodeHost};
pub use peers::{PeerHandler, PeerServices};
pub use platform::{AppPlatform, ServicesConfig, Spawn, StartError, TaskHandle};
pub use profile::{ProfileRuntime, ProfileSlot, bind_profile};
