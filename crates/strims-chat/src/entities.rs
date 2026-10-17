// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Message entities: the links, emoji, emotes, nicks, tags, code blocks,
//! spoilers, greentext and `/me` text in a body, each with rune bounds.
//!
//! [`extract`] finds them in this order:
//! 1. links, by xurls' Relaxed pattern with leftmost-longest matches; a
//!    link without a scheme gets `https://` prepended;
//! 2. the [`text::parse`] tree, skipping nodes (and their children) that
//!    lie within a link;
//! 3. emoji outside links and code blocks.
//!
//! A bound `[start, end)` lies within another when `start` and `end` both
//! do, ends included.

use regex_automata::hybrid::dfa::{Cache as LazyCache, DFA};
use regex_automata::meta::{Cache, Regex};
use regex_automata::nfa::thompson::{self, WhichCaptures};
use regex_automata::{Anchored, Input, MatchKind};
use strims_base::rng::DetRng;
use strims_proto::chat::v1::Modifier;
use strims_proto::chat::v1::message::Entities;
use strims_proto::chat::v1::message::entities::{
    Bounds, CodeBlock, Emoji, Emote, GenericEntity, Link, Nick, Spoiler, Tag,
};

use crate::text::{self, Node, ParserContext, Span, SpanKind, patterns};

/// The most entities [`extract`] puts in one message, counting every
/// link, emote, emoji, nick, tag, code block and spoiler and the greentext
/// and `/me` spans. Entities past the bound are dropped.
pub const MAX_ENTITIES: usize = 1024;

/// The lazy DFA cache of each compiled pattern. A search that outgrows it
/// clears it and continues, so it bounds memory, not results.
const LAZY_CACHE_BYTES: usize = 512 << 10;

/// The size limit of each pattern's compiled NFA.
const NFA_BYTES: usize = 16 << 20;

/// The compiled link, scheme and emoji patterns and their search caches.
///
/// Compiling takes tens of milliseconds and holds a few MiB, so an owner
/// builds one when it first needs it and keeps it. Searches don't grow it
/// past the capped lazy DFA caches.
pub struct Patterns {
    link: Regex,
    link_cache: Cache,
    /// The link pattern as an anchored DFA that reports the longest match.
    link_longest: DFA,
    link_longest_cache: LazyCache,
    scheme: Regex,
    scheme_cache: Cache,
    emoji: Regex,
    emoji_cache: Cache,
}

/// Compiles a leftmost-first pattern.
fn leftmost_first(pattern: &[&str]) -> Regex {
    // invariant: the generated patterns compile within these limits, as
    // the crate's tests check.
    #[allow(clippy::expect_used)]
    Regex::builder()
        .configure(
            Regex::config()
                .which_captures(WhichCaptures::Implicit)
                .nfa_size_limit(Some(NFA_BYTES))
                .hybrid_cache_capacity(LAZY_CACHE_BYTES),
        )
        .build(&pattern.concat())
        .expect("generated pattern")
}

/// Compiles a pattern into a DFA whose anchored searches report the end
/// of the longest match.
fn longest(pattern: &[&str]) -> DFA {
    // invariant: as for `leftmost_first`.
    #[allow(clippy::expect_used)]
    DFA::builder()
        .configure(
            DFA::config()
                .match_kind(MatchKind::All)
                .cache_capacity(LAZY_CACHE_BYTES),
        )
        .thompson(
            thompson::Config::new()
                .which_captures(WhichCaptures::None)
                .nfa_size_limit(Some(NFA_BYTES)),
        )
        .build(&pattern.concat())
        .expect("generated pattern")
}

impl Patterns {
    /// Compiles the patterns.
    pub fn new() -> Self {
        let link = leftmost_first(patterns::LINK);
        let link_longest = longest(patterns::LINK);
        let scheme = leftmost_first(patterns::SCHEME);
        let emoji = leftmost_first(patterns::EMOJI);
        Self {
            link_cache: link.create_cache(),
            link,
            link_longest_cache: link_longest.create_cache(),
            link_longest,
            scheme_cache: scheme.create_cache(),
            scheme,
            emoji_cache: emoji.create_cache(),
            emoji,
        }
    }

    /// The byte ranges of `body`'s links, leftmost-longest.
    ///
    /// A leftmost-first search finds where the next match starts, since
    /// both semantics start at the leftmost position any match starts at;
    /// the anchored DFA then finds the longest match from there.
    fn links(&mut self, body: &str, limit: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut at = 0;
        while out.len() < limit && at < body.len() {
            let input = Input::new(body).range(at..);
            let Some(m) = self.link.search_with(&mut self.link_cache, &input) else {
                break;
            };
            let longest = Input::new(body).range(m.start()..).anchored(Anchored::Yes);
            let end = match self
                .link_longest
                .try_search_fwd(&mut self.link_longest_cache, &longest)
            {
                Ok(Some(half)) => half.offset().max(m.end()),
                _ => m.end(),
            };
            if end > m.start() {
                out.push((m.start(), end));
                at = end;
            } else {
                at = next_boundary(body, m.start());
            }
        }
        out
    }

    /// The byte ranges of `body`'s emoji, leftmost-first.
    fn emoji(&mut self, body: &str, limit: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut at = 0;
        while out.len() < limit && at < body.len() {
            let input = Input::new(body).range(at..);
            let Some(m) = self.emoji.search_with(&mut self.emoji_cache, &input) else {
                break;
            };
            if m.end() > m.start() {
                out.push((m.start(), m.end()));
                at = m.end();
            } else {
                at = next_boundary(body, m.start());
            }
        }
        out
    }

    fn has_scheme(&mut self, url: &str) -> bool {
        self.scheme
            .search_half_with(&mut self.scheme_cache, &Input::new(url))
            .is_some()
    }
}

impl Default for Patterns {
    fn default() -> Self {
        Self::new()
    }
}

/// The byte offset of the char boundary after `at`.
fn next_boundary(body: &str, at: usize) -> usize {
    body.get(at..)
        .and_then(|rest| rest.chars().next())
        .map_or(body.len(), |c| at + c.len_utf8())
}

/// Converts increasing byte offsets into rune offsets.
struct RuneOffsets<'a> {
    body: &'a str,
    byte: usize,
    rune: usize,
}

impl<'a> RuneOffsets<'a> {
    fn new(body: &'a str) -> Self {
        Self {
            body,
            byte: 0,
            rune: 0,
        }
    }

    fn at(&mut self, byte: usize) -> u32 {
        if byte < self.byte {
            self.byte = 0;
            self.rune = 0;
        }
        self.rune += self
            .body
            .get(self.byte..byte)
            .map_or(0, |s| s.chars().count());
        self.byte = byte;
        to_u32(self.rune)
    }

    fn bounds(&mut self, (start, end): (usize, usize)) -> Bounds {
        Bounds {
            start: self.at(start),
            end: self.at(end),
        }
    }
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn rune_bounds(pos: usize, end: usize) -> Option<Bounds> {
    Some(Bounds {
        start: to_u32(pos),
        end: to_u32(end),
    })
}

/// Whether `[start, end)` lies within one of `bounds`, which are sorted
/// and disjoint.
fn within<T>(entities: &[T], bounds: impl Fn(&T) -> Option<Bounds>, start: u32, end: u32) -> bool {
    let i = entities.partition_point(|e| bounds(e).is_some_and(|b| b.start <= start));
    i.checked_sub(1)
        .and_then(|i| entities.get(i))
        .and_then(bounds)
        .is_some_and(|b| b.start <= start && b.end >= end)
}

struct Extraction {
    entities: Entities,
    count: usize,
}

impl Extraction {
    fn room(&mut self) -> bool {
        if self.count < MAX_ENTITIES {
            self.count += 1;
            true
        } else {
            false
        }
    }

    fn in_link(&self, start: u32, end: u32) -> bool {
        within(&self.entities.links, |l| l.bounds, start, end)
    }

    fn in_code(&self, start: u32, end: u32) -> bool {
        within(&self.entities.code_blocks, |c| c.bounds, start, end)
    }

    fn add_span(&mut self, span: &Span) {
        let bounds = rune_bounds(span.pos, span.end);
        match span.kind {
            SpanKind::Message => {}
            SpanKind::Code => {
                if self.room() {
                    self.entities.code_blocks.push(CodeBlock { bounds });
                }
            }
            SpanKind::Spoiler => {
                if self.room() {
                    self.entities.spoilers.push(Spoiler { bounds });
                }
            }
            SpanKind::Greentext => {
                if self.room() {
                    self.entities.green_text = Some(GenericEntity { bounds });
                }
            }
            SpanKind::Me => {
                if self.room() {
                    self.entities.self_message = Some(GenericEntity { bounds });
                }
            }
        }
        for node in &span.nodes {
            self.add_node(node);
        }
    }

    fn add_node(&mut self, node: &Node) {
        if self.in_link(to_u32(node.pos()), to_u32(node.end())) {
            return;
        }
        match node {
            Node::Span(span) => self.add_span(span),
            Node::Emote(e) => {
                if self.room() {
                    self.entities.emotes.push(Emote {
                        bounds: rune_bounds(e.pos, e.end),
                        name: e.name.clone(),
                        modifiers: e.modifiers.clone(),
                        combo: 0,
                        can_combo: false,
                    });
                }
            }
            Node::Nick(n) => {
                if self.room() {
                    self.entities.nicks.push(Nick {
                        bounds: rune_bounds(n.pos, n.end),
                        nick: n.nick.clone(),
                        peer_key: n.peer_key.clone(),
                        viewed_listing: None,
                    });
                }
            }
            Node::Tag(t) => {
                if self.room() {
                    self.entities.tags.push(Tag {
                        bounds: rune_bounds(t.pos, t.end),
                        name: t.name.clone(),
                    });
                }
            }
        }
    }
}

/// Extracts `body`'s entities, matching emotes, modifiers, tags and nicks
/// against `ctx`.
pub fn extract(body: &str, ctx: &ParserContext, patterns: &mut Patterns) -> Entities {
    let mut x = Extraction {
        entities: Entities::default(),
        count: 0,
    };

    let mut offsets = RuneOffsets::new(body);
    for (start, end) in patterns.links(body, MAX_ENTITIES) {
        let Some(text) = body.get(start..end) else {
            continue;
        };
        let url = if patterns.has_scheme(text) {
            text.to_owned()
        } else {
            format!("https://{text}")
        };
        if x.room() {
            x.entities.links.push(Link {
                bounds: Some(offsets.bounds((start, end))),
                url,
            });
        }
    }

    x.add_span(&text::parse(ctx, body));

    let mut offsets = RuneOffsets::new(body);
    let limit = MAX_ENTITIES.saturating_sub(x.count);
    for range in patterns.emoji(body, limit) {
        let bounds = offsets.bounds(range);
        if !x.in_link(bounds.start, bounds.end) && !x.in_code(bounds.start, bounds.end) && x.room()
        {
            x.entities.emojis.push(Emoji {
                bounds: Some(bounds),
            });
        }
    }

    x.entities
}

/// Adds each of a server's internal `modifiers` to a random emote with
/// probability `proc_chance`, in order. Messages without emotes are left
/// unchanged and draw nothing from `rng`.
pub fn add_internal_modifiers(entities: &mut Entities, modifiers: &[Modifier], rng: &mut DetRng) {
    use rand_core::Rng;

    for m in modifiers {
        let n = entities.emotes.len() as u64;
        if n == 0 {
            return;
        }
        // 53 random bits give a uniform f64 in [0, 1).
        let roll = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        if roll <= m.proc_chance {
            let i = usize::try_from(rng.next_u64() % n).unwrap_or(0);
            if let Some(e) = entities.emotes.get_mut(i) {
                e.modifiers.push(m.name.clone());
            }
        }
    }
}

#[cfg(test)]
#[path = "entities_test.rs"]
mod tests;
