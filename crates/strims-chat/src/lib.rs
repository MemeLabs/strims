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

pub mod entities;
pub mod text;
