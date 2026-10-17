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
//!   modifiers.
//!
//! A server publishes its room, icon, emotes, modifiers and tags as
//! [`assets`] bundles: a unified bundle after each stream reset, then
//! deltas. A client decodes the event and asset records through the
//! bounded readers in [`reader`].
//!
//! [`whisper`] decides which whispers a profile takes, and schedules the
//! delivery of those it sends.

pub mod assets;
pub mod entities;
pub mod reader;
pub mod text;
pub mod whisper;
