// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
// Derived from chat-parser; see licenses/chat-parser.txt.

//! Message text parsing.
//!
//! [`parse`] splits a body into a tree: the message (or a greentext or
//! `/me` message) holding code spans, spoilers, emotes with their
//! modifiers, tags and nick mentions, matched against a
//! [`ParserContext`]. Words end at whitespace, dashes, punctuation, math
//! and pattern syntax runes; emote, modifier and tag names match words
//! exactly, and nicks match them case-insensitively.
//!
//! Positions are rune (Unicode scalar value) offsets into the body.
//! Parsing never fails: every body has a tree.

mod ast;
mod context;
mod lex;
mod parse;
pub(crate) mod patterns;
mod tables;

pub use ast::{Emote, Nick, Node, Span, SpanKind, Tag};
pub use context::{MAX_NICK_BYTES, MAX_NICKS, NickEntry, NickIndex, ParserContext};
pub use parse::{MAX_NODES, parse};
