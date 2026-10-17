// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The chat service.
//!
//! A chat server runs each message it accepts through the message text
//! pipeline:
//! - [`text`] parses the body into spans, emotes with their modifiers, tags
//!   and nick mentions;
//! - [`entities`] extracts the message's entities (links, emoji and the
//!   parsed nodes) with rune bounds, and adds the server's internal emote
//!   modifiers;
//! - [`combo`] marks runs of the same lone emote.
//!
//! [`Server`] is the sans-IO state of a server: sends with their bounds
//! and send rates ([`rate`]), moderation and mutes, and the nicks of its
//! directory listing. [`duration`] parses the duration strings mutes and
//! ignores are given in.
//!
//! A server publishes its room, icon, emotes, modifiers and tags as
//! [`assets`] bundles: a unified bundle after each stream reset, then
//! deltas. A client decodes the event and asset records through the
//! bounded readers in [`reader`].
//!
//! [`whisper`] decides which whispers a profile takes, and schedules the
//! delivery of those it sends.
//!
//! # Whispers
//!
//! [`WhisperControl`] runs a profile's whispers on a node, beside its
//! [`NetworkControl`](strims_network::NetworkControl). The driver runs
//! [`WhisperControl::run`] as a local task.
//!
//! - It serves `chat.v1.Whisper` on a whisper port of every network the
//!   node has joined with a trusted membership.
//! - Among the processes sharing the profile's store, the holder of the
//!   lease [`whisper::lease_key`] publishes those ports' addresses and
//!   delivers the profile's whispers.
//! - A received whisper is stored UNREAD with its sender's thread marked
//!   unread, unless the sender is ignored or over a bound
//!   ([`whisper::check_receive`]). A whisper received again is taken
//!   without a write.
//! - [`WhisperControl::enqueue`] stores a whisper ENQUEUED. Delivery
//!   retries on the [`whisper::Deliveries`] schedule until the recipient
//!   takes it (DELIVERED) or refuses it, or the attempts or time run out
//!   (FAILED). A state is written only over ENQUEUED, so a record the user
//!   deleted or changed meanwhile keeps its own.
//!
//! **Bounds.** [`WhisperLimits`](whisper::WhisperLimits) bounds what a
//! sender can make a profile store (body bytes, unread whispers per
//! sender, threads) and the delivery queue (records pending and attempts
//! in flight; [`WhisperControl::enqueue`] refuses past the pending
//! bound). Each sender's calls are rate limited by
//! [`WhisperConfig::rate`], over at most
//! [`RateConfig::max_keys`](rate::RateConfig::max_keys) senders, and each
//! delivery attempt ends within [`WhisperConfig::attempt_deadline`].
//!
//! # Wire contract
//!
//! - **Swarms.** A server's event and asset swarms have the server's
//!   public key as their swarm id, the options [`event_options`] and
//!   [`asset_options`], and the transfer salts [`EVENT_SALT`] and
//!   [`ASSET_SALT`]. The event swarm carries `ServerEvent` records in
//!   frames of [`EVENT_FRAME`] bytes, the asset swarm `AssetBundle`
//!   records in frames of [`ASSET_FRAME`] bytes.
//! - **Addresses.** A server publishes its address under (server key,
//!   [`ADDRESS_SALT`]) and serves `chat.v1.Chat` there; a profile
//!   publishes its whisper address under (profile key, [`WHISPER_SALT`]).
//! - **Whispers.** A profile serves `chat.v1.Whisper.SendMessage` at its
//!   whisper address. The caller's vpn certificate names the sender: its
//!   membership certificate's key and subject, under the network's root.
//!   A whisper's `id` is the sender's record id, the same on every
//!   attempt, and a receiver stores one whisper per (`id`, sender).
//! - **Errors.** A refused call is answered with the message of its
//!   [`ChatError`]; a refused whisper with [`whisper::WHISPER_REFUSED`],
//!   and one over its sender's rate with [`ChatError::TooFast`]'s.
//! - **Leases.** The holder of [`whisper::lease_key`] runs the profile's
//!   whispers among the processes sharing its store.

pub mod assets;
pub mod combo;
pub mod duration;
pub mod entities;
mod error;
pub mod rate;
pub mod reader;
pub mod server;
pub mod swarms;
pub mod text;
pub mod whisper;

pub use error::ChatError;
pub use server::{Server, ServerAction, ServerLimits, ServerMetrics};
pub use swarms::{
    ADDRESS_SALT, ASSET_FRAME, ASSET_SALT, ASSET_WINDOW_BYTES, EVENT_FRAME, EVENT_SALT,
    WHISPER_SALT, asset_options, event_options,
};
pub use whisper::{EnqueueError, WhisperConfig, WhisperControl, WhisperMetrics};
