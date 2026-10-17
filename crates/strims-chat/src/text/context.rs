// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
// Derived from chat-parser; see licenses/chat-parser.txt.

//! The names a parse matches words against.

use std::collections::{BTreeMap, BTreeSet};

use super::tables::TO_LOWER;

/// The most nicks a [`NickIndex`] holds.
pub const MAX_NICKS: usize = 16384;

/// The longest nick, in bytes, a [`NickIndex`] holds.
pub const MAX_NICK_BYTES: usize = 256;

/// An indexed nick and its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NickEntry {
    pub nick: String,
    pub peer_key: Vec<u8>,
}

/// Nicks matched case-insensitively: each is keyed by its runes
/// lowercased one at a time, so nicks differing only in case share an
/// entry. It holds at most [`MAX_NICKS`] nicks of at most
/// [`MAX_NICK_BYTES`] bytes.
#[derive(Clone, Debug, Default)]
pub struct NickIndex {
    entries: BTreeMap<Box<[char]>, NickEntry>,
}

impl NickIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `nick`, replacing any entry with the same lowercase form.
    /// Returns false, leaving the index unchanged, when `nick` is empty or
    /// longer than [`MAX_NICK_BYTES`], or when it's new and the index is
    /// full.
    pub fn insert(&mut self, nick: &str, peer_key: Vec<u8>) -> bool {
        if nick.is_empty() || nick.len() > MAX_NICK_BYTES {
            return false;
        }
        let key = lowercase(nick.chars());
        if self.entries.len() >= MAX_NICKS && !self.entries.contains_key(&key) {
            return false;
        }
        self.entries.insert(
            key,
            NickEntry {
                nick: nick.to_owned(),
                peer_key,
            },
        );
        true
    }

    /// Removes the entry with `nick`'s lowercase form.
    pub fn remove(&mut self, nick: &str) -> Option<NickEntry> {
        self.entries.remove(&lowercase(nick.chars()))
    }

    /// The entry with `nick`'s lowercase form.
    pub fn get(&self, nick: &str) -> Option<&NickEntry> {
        self.entries.get(&lowercase(nick.chars()))
    }

    /// [`Self::get`] for a token's runes, lowercased into `scratch`.
    pub(crate) fn get_runes(&self, runes: &[char], scratch: &mut Vec<char>) -> Option<&NickEntry> {
        scratch.clear();
        scratch.extend(runes.iter().copied().map(to_lower));
        self.entries.get(scratch.as_slice())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

fn lowercase(chars: impl Iterator<Item = char>) -> Box<[char]> {
    chars.map(to_lower).collect()
}

/// The rune's simple lowercase mapping.
fn to_lower(c: char) -> char {
    let r = u32::from(c);
    let i = TO_LOWER.partition_point(|&(lo, ..)| lo <= r);
    let Some(&(lo, hi, stride, delta)) = i.checked_sub(1).and_then(|i| TO_LOWER.get(i)) else {
        return c;
    };
    if r > hi || r.wrapping_sub(lo).checked_rem(stride) != Some(0) {
        return c;
    }
    r.checked_add_signed(delta)
        .and_then(char::from_u32)
        .unwrap_or(c)
}

/// The emotes, emote modifiers, tags and nicks a parse recognizes.
///
/// Emotes, modifiers and tags match words exactly. They hold the server's
/// own asset names, so the owner sets their size. Nicks match
/// case-insensitively; see [`NickIndex`].
#[derive(Clone, Debug, Default)]
pub struct ParserContext {
    emotes: BTreeSet<Box<[char]>>,
    modifiers: BTreeSet<Box<[char]>>,
    tags: BTreeSet<Box<[char]>>,
    nicks: NickIndex,
}

fn rune_set<I, S>(names: I) -> BTreeSet<Box<[char]>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    names
        .into_iter()
        .map(|name| name.as_ref().chars().collect())
        .collect()
}

impl ParserContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the emote names.
    pub fn set_emotes<I, S>(&mut self, names: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.emotes = rune_set(names);
    }

    /// Replaces the modifier names that may follow an emote.
    pub fn set_modifiers<I, S>(&mut self, names: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.modifiers = rune_set(names);
    }

    /// Replaces the tag names.
    pub fn set_tags<I, S>(&mut self, names: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.tags = rune_set(names);
    }

    pub fn nicks(&self) -> &NickIndex {
        &self.nicks
    }

    pub fn nicks_mut(&mut self) -> &mut NickIndex {
        &mut self.nicks
    }

    pub(crate) fn is_emote(&self, runes: &[char]) -> bool {
        self.emotes.contains(runes)
    }

    pub(crate) fn is_modifier(&self, runes: &[char]) -> bool {
        self.modifiers.contains(runes)
    }

    pub(crate) fn is_tag(&self, runes: &[char]) -> bool {
        self.tags.contains(runes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nicks_match_case_insensitively() {
        let mut index = NickIndex::new();
        for nick in ["FOO", "Bar", "baz"] {
            assert!(index.insert(nick, nick.as_bytes().to_vec()));
        }
        assert_eq!(index.get("foo").map(|e| e.nick.as_str()), Some("FOO"));
        assert_eq!(index.get("BAR").map(|e| e.nick.as_str()), Some("Bar"));
        assert!(index.get("qux").is_none());
    }

    #[test]
    fn a_nick_differing_in_case_replaces_the_entry() {
        let mut index = NickIndex::new();
        assert!(index.insert("bob", vec![1]));
        assert!(index.insert("Bob", vec![2]));
        assert_eq!(index.len(), 1);
        assert_eq!(
            index.get("BOB"),
            Some(&NickEntry {
                nick: "Bob".into(),
                peer_key: vec![2]
            })
        );
        assert_eq!(index.remove("bOB").map(|e| e.peer_key), Some(vec![2]));
        assert!(index.is_empty());
    }

    #[test]
    fn lowercasing_follows_the_simple_mapping() {
        assert_eq!(to_lower('A'), 'a');
        assert_eq!(to_lower('a'), 'a');
        assert_eq!(to_lower('\u{212a}'), 'k');
        assert_eq!(to_lower('\u{130}'), 'i');
        assert_eq!(to_lower('\u{1c5}'), '\u{1c6}');
        assert_eq!(to_lower('\u{100}'), '\u{101}');
        assert_eq!(to_lower('\u{101}'), '\u{101}');
        assert_eq!(to_lower('ẞ'), 'ß');
        assert_eq!(to_lower('\u{10400}'), '\u{10428}');
        assert_eq!(to_lower('\0'), '\0');
        assert_eq!(to_lower(char::MAX), char::MAX);
        let mut index = NickIndex::new();
        assert!(index.insert("kelvin", vec![]));
        assert!(index.get("\u{212a}ELVIN").is_some());
    }

    #[test]
    fn the_index_is_bounded() {
        let mut index = NickIndex::new();
        assert!(!index.insert("", vec![]));
        assert!(!index.insert(&"x".repeat(MAX_NICK_BYTES + 1), vec![]));
        assert!(index.insert(&"x".repeat(MAX_NICK_BYTES), vec![]));
        index.clear();
        for i in 0..MAX_NICKS {
            assert!(index.insert(&format!("n{i}"), vec![]));
        }
        assert!(!index.insert("one more", vec![]));
        assert!(
            index.insert("N0", vec![1]),
            "replacing stays within the bound"
        );
        assert_eq!(index.len(), MAX_NICKS);
    }
}
