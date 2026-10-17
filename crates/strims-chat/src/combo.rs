// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Emote combos: runs of messages from distinct users that each post the
//! same emote and nothing else.
//!
//! A message is a combo candidate when its entities hold exactly one
//! emote and that emote spans the whole body, `[0, rune count)`, modifiers
//! included. A candidate gets `can_combo`. It continues the running combo
//! when it posts the same emote, and starts a new one otherwise; any other
//! message ends the combo. From the second message of a combo on, the
//! emote carries `combo` (the number of messages so far) and the
//! modifiers of the combo's most frequent variant, ties going to the
//! variant seen first. A user posts once per combo: a repeat fails with
//! [`ChatError::ComboDuplicate`] and leaves the combo as it was.

use std::collections::BTreeSet;

use strims_proto::chat::v1::message::Entities;
use strims_proto::chat::v1::message::entities::Emote;

use crate::ChatError;

/// The most messages a combo counts. A candidate from a new user beyond
/// it starts a new combo.
pub const MAX_COMBO: usize = 1024;

/// A distinct modifier list of the combo's emote and how many messages
/// posted it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Variant {
    modifiers: Vec<String>,
    count: u32,
}

/// A chat server's running combo, keyed by the posters' peer keys.
#[derive(Clone, Debug, Default)]
pub struct Combos {
    emote: String,
    /// The combo's posters; at most [`MAX_COMBO`].
    participants: BTreeSet<Vec<u8>>,
    /// In the order first seen; at most one per participant.
    variants: Vec<Variant>,
}

impl Combos {
    pub fn new() -> Self {
        Self::default()
    }

    /// The running combo's emote and message count, if one runs.
    pub fn current(&self) -> Option<(&str, usize)> {
        (!self.participants.is_empty()).then_some((self.emote.as_str(), self.participants.len()))
    }

    /// Ends the running combo.
    pub fn reset(&mut self) {
        self.emote.clear();
        self.participants.clear();
        self.variants.clear();
    }

    /// Runs the message `peer_key` posts with `body` and `entities`
    /// through the combo, marking its emote as described in the module
    /// docs.
    pub fn transform(
        &mut self,
        peer_key: &[u8],
        body: &str,
        entities: &mut Entities,
    ) -> Result<(), ChatError> {
        let Some(emote) = lone_emote(body, entities) else {
            self.reset();
            return Ok(());
        };

        if self.emote != emote.name {
            self.reset();
        } else if self.participants.contains(peer_key) {
            return Err(ChatError::ComboDuplicate);
        }
        if self.participants.len() >= MAX_COMBO {
            self.reset();
        }

        if self.participants.is_empty() {
            self.emote.clone_from(&emote.name);
        }
        self.participants.insert(peer_key.to_vec());
        match self
            .variants
            .iter_mut()
            .find(|v| v.modifiers == emote.modifiers)
        {
            Some(v) => v.count += 1,
            None => self.variants.push(Variant {
                modifiers: emote.modifiers.clone(),
                count: 1,
            }),
        }

        emote.can_combo = true;
        let count = self.participants.len();
        if count >= 2 {
            emote.combo = u32::try_from(count).unwrap_or(u32::MAX);
            if let Some(top) = self.top_variant() {
                emote.modifiers.clone_from(&top.modifiers);
            }
        }
        Ok(())
    }

    /// The most frequent variant, the first seen among equals.
    fn top_variant(&self) -> Option<&Variant> {
        self.variants
            .iter()
            .fold(None, |best: Option<&Variant>, v| match best {
                Some(b) if b.count >= v.count => Some(b),
                _ => Some(v),
            })
    }
}

/// The message's emote, when it is the only one and spans the whole body.
fn lone_emote<'a>(body: &str, entities: &'a mut Entities) -> Option<&'a mut Emote> {
    let [emote] = entities.emotes.as_mut_slice() else {
        return None;
    };
    let bounds = emote.bounds?;
    let runes = u32::try_from(body.chars().count()).ok()?;
    (bounds.start == 0 && bounds.end == runes).then_some(emote)
}

#[cfg(test)]
mod tests {
    // Tests panic on failure by design.
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use strims_proto::chat::v1::message::entities::Bounds;

    use super::*;

    fn emote(name: &str, modifiers: &[&str], start: u32, end: u32) -> Emote {
        Emote {
            bounds: Some(Bounds { start, end }),
            name: name.into(),
            modifiers: modifiers.iter().map(|m| (*m).into()).collect(),
            ..Emote::default()
        }
    }

    /// The entities of a body holding just `name` with `modifiers`.
    fn lone(name: &str, modifiers: &[&str]) -> (String, Entities) {
        let body = std::iter::once(name)
            .chain(modifiers.iter().copied())
            .collect::<Vec<_>>()
            .join(":");
        let end = body.chars().count() as u32;
        let entities = Entities {
            emotes: vec![emote(name, modifiers, 0, end)],
            ..Entities::default()
        };
        (body, entities)
    }

    /// Posts `name` with `modifiers` as `peer`, returning the emote sent.
    fn post(c: &mut Combos, peer: u8, name: &str, modifiers: &[&str]) -> Result<Emote, ChatError> {
        let (body, mut e) = lone(name, modifiers);
        c.transform(&[peer], &body, &mut e)?;
        Ok(e.emotes.remove(0))
    }

    #[test]
    fn a_lone_emote_starts_a_combo_that_counts_from_the_second_message() {
        let mut c = Combos::new();
        let first = post(&mut c, 1, "PEPE", &[]).unwrap();
        assert!(first.can_combo);
        assert_eq!(first.combo, 0);
        assert_eq!(c.current(), Some(("PEPE", 1)));

        let second = post(&mut c, 2, "PEPE", &[]).unwrap();
        assert!(second.can_combo);
        assert_eq!(second.combo, 2);
        let third = post(&mut c, 3, "PEPE", &[]).unwrap();
        assert_eq!(third.combo, 3);
        assert_eq!(c.current(), Some(("PEPE", 3)));
    }

    #[test]
    fn a_repeat_by_the_same_peer_fails_and_keeps_the_combo() {
        let mut c = Combos::new();
        post(&mut c, 1, "PEPE", &[]).unwrap();
        post(&mut c, 2, "PEPE", &[]).unwrap();
        assert_eq!(
            post(&mut c, 1, "PEPE", &["wide"]),
            Err(ChatError::ComboDuplicate)
        );
        assert_eq!(c.current(), Some(("PEPE", 2)));
        assert_eq!(post(&mut c, 3, "PEPE", &[]).unwrap().combo, 3);
    }

    #[test]
    fn another_emote_starts_a_new_combo() {
        let mut c = Combos::new();
        post(&mut c, 1, "PEPE", &[]).unwrap();
        post(&mut c, 2, "PEPE", &[]).unwrap();
        let other = post(&mut c, 1, "Kappa", &[]).unwrap();
        assert!(other.can_combo);
        assert_eq!(other.combo, 0);
        assert_eq!(c.current(), Some(("Kappa", 1)));
        assert_eq!(post(&mut c, 2, "Kappa", &[]).unwrap().combo, 2);
    }

    #[test]
    fn any_other_message_ends_the_combo() {
        let mut c = Combos::new();
        post(&mut c, 1, "PEPE", &[]).unwrap();
        let mut text = Entities::default();
        c.transform(&[2], "hello", &mut text).unwrap();
        assert_eq!(c.current(), None);
        assert_eq!(text, Entities::default());
        let again = post(&mut c, 1, "PEPE", &[]).unwrap();
        assert_eq!(again.combo, 0, "the earlier poster starts over");
    }

    #[test]
    fn an_emote_among_text_is_no_candidate() {
        let mut c = Combos::new();
        post(&mut c, 1, "PEPE", &[]).unwrap();
        let mut e = Entities {
            emotes: vec![emote("PEPE", &[], 0, 4)],
            ..Entities::default()
        };
        c.transform(&[2], "PEPE lol", &mut e).unwrap();
        assert!(!e.emotes[0].can_combo);
        assert_eq!(c.current(), None);
    }

    #[test]
    fn two_emotes_are_no_candidate() {
        let mut c = Combos::new();
        post(&mut c, 1, "PEPE", &[]).unwrap();
        let mut e = Entities {
            emotes: vec![emote("PEPE", &[], 0, 4), emote("PEPE", &[], 5, 9)],
            ..Entities::default()
        };
        c.transform(&[2], "PEPE PEPE", &mut e).unwrap();
        assert!(e.emotes.iter().all(|e| !e.can_combo));
        assert_eq!(c.current(), None);
    }

    #[test]
    fn the_whole_body_is_measured_in_runes() {
        // "ÄÖÜ" is 3 runes and 6 bytes.
        let mut c = Combos::new();
        let mut runes = Entities {
            emotes: vec![emote("ÄÖÜ", &[], 0, 3)],
            ..Entities::default()
        };
        c.transform(&[1], "ÄÖÜ", &mut runes).unwrap();
        assert!(runes.emotes[0].can_combo);

        let mut bytes = Entities {
            emotes: vec![emote("ÄÖÜ", &[], 0, 6)],
            ..Entities::default()
        };
        c.transform(&[2], "ÄÖÜ", &mut bytes).unwrap();
        assert!(!bytes.emotes[0].can_combo);
        assert_eq!(c.current(), None);
    }

    #[test]
    fn an_emote_without_bounds_is_no_candidate() {
        let mut c = Combos::new();
        let mut e = Entities {
            emotes: vec![Emote {
                name: "PEPE".into(),
                ..Emote::default()
            }],
            ..Entities::default()
        };
        c.transform(&[1], "PEPE", &mut e).unwrap();
        assert!(!e.emotes[0].can_combo);
    }

    #[test]
    fn the_first_message_keeps_its_own_modifiers() {
        let mut c = Combos::new();
        let first = post(&mut c, 1, "PEPE", &["wide"]).unwrap();
        assert_eq!(first.modifiers, ["wide"]);
    }

    #[test]
    fn the_variant_is_the_most_frequent_with_ties_to_the_first_seen() {
        let mut c = Combos::new();
        post(&mut c, 1, "PEPE", &["wide"]).unwrap();
        // wide 1, plain 1: the tie goes to wide.
        assert_eq!(post(&mut c, 2, "PEPE", &[]).unwrap().modifiers, ["wide"]);
        // wide 1, plain 2.
        let third = post(&mut c, 3, "PEPE", &[]).unwrap();
        assert!(third.modifiers.is_empty());
        // wide 2, plain 2: the tie goes to wide again.
        assert_eq!(
            post(&mut c, 4, "PEPE", &["wide"]).unwrap().modifiers,
            ["wide"]
        );
        // wide 2, plain 2, flip:wide 3.
        for peer in 5..8 {
            post(&mut c, peer, "PEPE", &["flip", "wide"]).unwrap();
        }
        assert_eq!(
            post(&mut c, 8, "PEPE", &["wide"]).unwrap().modifiers,
            ["wide"],
            "wide 3, flip:wide 3: the tie goes to the first seen"
        );
        assert_eq!(
            post(&mut c, 9, "PEPE", &["flip", "wide"])
                .unwrap()
                .modifiers,
            ["flip", "wide"]
        );
    }

    #[test]
    fn modifier_order_makes_a_distinct_variant() {
        let mut c = Combos::new();
        post(&mut c, 1, "PEPE", &["wide", "flip"]).unwrap();
        post(&mut c, 2, "PEPE", &["flip", "wide"]).unwrap();
        let third = post(&mut c, 3, "PEPE", &["flip", "wide"]).unwrap();
        assert_eq!(third.modifiers, ["flip", "wide"]);
    }

    #[test]
    fn the_combo_starts_over_past_its_bound() {
        let mut c = Combos::new();
        for i in 0..MAX_COMBO {
            let key = (i as u32).to_be_bytes();
            let (body, mut e) = lone("PEPE", &[]);
            c.transform(&key, &body, &mut e).unwrap();
        }
        assert_eq!(c.current(), Some(("PEPE", MAX_COMBO)));
        let (body, mut e) = lone("PEPE", &[]);
        c.transform(b"new", &body, &mut e).unwrap();
        assert_eq!(e.emotes[0].combo, 0);
        assert_eq!(c.current(), Some(("PEPE", 1)));

        // A repeat still fails at the bound.
        let mut c = Combos::new();
        for i in 0..MAX_COMBO {
            let key = (i as u32).to_be_bytes();
            let (body, mut e) = lone("PEPE", &[]);
            c.transform(&key, &body, &mut e).unwrap();
        }
        let (body, mut e) = lone("PEPE", &[]);
        assert_eq!(
            c.transform(&0u32.to_be_bytes(), &body, &mut e),
            Err(ChatError::ComboDuplicate)
        );
    }
}
