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
//! - **Errors.** A refused call is answered with the message of its
//!   [`ChatError`]; a refused whisper with [`whisper::WHISPER_REFUSED`].

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
