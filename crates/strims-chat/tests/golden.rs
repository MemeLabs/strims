// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors that `golden/main.go` records from the Go implementation
//! (regenerate from the repo root with
//! `go run -tags golden ./crates/strims-chat/golden`): chat-parser's parse
//! trees and internal/chat's encoded entities for chat-parser's fuzz corpus,
//! crafted and generated inputs, under one parser context the file defines,
//! and the runes each case-folded ASCII letter matches.

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeSet;

use prost::Message;
use strims_base::rng::det_rng;
use strims_chat::entities::{Patterns, add_internal_modifiers, extract};
use strims_chat::text::{ParserContext, parse};
use strims_proto::chat::v1::Modifier;
use strims_proto::chat::v1::message::Entities;

const VECTORS: &str = include_str!("../golden/vectors.hex");

struct Input {
    name: &'static str,
    body: String,
    /// `None` for inputs left out of the entity vectors.
    entities: Option<Vec<u8>>,
    /// `None` unless the input has exactly one emote.
    internal: Option<Vec<u8>>,
    ast: &'static str,
}

struct Vectors {
    ctx: ParserContext,
    internal_modifiers: Vec<Modifier>,
    inputs: Vec<Input>,
    /// Each lowercase ASCII letter and the runes it matches case-folded.
    folds: Vec<(char, Vec<char>)>,
}

fn decode_hex(s: &str) -> Vec<u8> {
    if s == "=" {
        return Vec::new();
    }
    hex::decode(s).unwrap()
}

fn optional_hex(s: &str) -> Option<Vec<u8>> {
    (s != "-").then(|| decode_hex(s))
}

fn vectors() -> Vectors {
    let (mut emotes, mut modifiers, mut tags) = (Vec::new(), Vec::new(), Vec::new());
    let mut ctx = ParserContext::new();
    let mut internal_modifiers = Vec::new();
    let mut inputs = Vec::new();
    let mut folds = Vec::new();
    for line in VECTORS.lines().filter(|l| !l.trim().is_empty()) {
        let (kind, rest) = line.split_once(' ').unwrap();
        match kind {
            "emote" => emotes.push(rest),
            "modifier" => modifiers.push(rest),
            "tag" => tags.push(rest),
            "internal_modifier" => internal_modifiers.push(Modifier {
                id: internal_modifiers.len() as u64 + 1,
                name: rest.to_owned(),
                internal: true,
                proc_chance: 1.0,
                ..Modifier::default()
            }),
            "nick" => {
                let (nick, key) = rest.rsplit_once(' ').unwrap();
                assert!(ctx.nicks_mut().insert(nick, decode_hex(key)), "{nick}");
            }
            "input" => {
                let mut parts = rest.splitn(5, ' ');
                let mut next = || parts.next().unwrap();
                inputs.push(Input {
                    name: next(),
                    body: String::from_utf8(decode_hex(next())).unwrap(),
                    entities: optional_hex(next()),
                    internal: optional_hex(next()),
                    ast: next(),
                });
            }
            "fold" => {
                let mut parts = rest.split(' ');
                let letter = parts.next().unwrap().chars().next().unwrap();
                let runes = parts
                    .map(|r| char::from_u32(u32::from_str_radix(r, 16).unwrap()).unwrap())
                    .collect();
                folds.push((letter, runes));
            }
            _ => panic!("unknown vector kind {kind}"),
        }
    }
    ctx.set_emotes(emotes);
    ctx.set_modifiers(modifiers);
    ctx.set_tags(tags);
    Vectors {
        ctx,
        internal_modifiers,
        inputs,
        folds,
    }
}

/// The patterns write case-folded ASCII letters as `(?i:...)`, so `(?i)`
/// must fold each to the same runes as the Go patterns' case folding.
#[test]
fn case_folded_ascii_letters_match_go() {
    let v = vectors();
    assert_eq!(v.folds.len(), 26);
    let mut non_ascii = BTreeSet::new();
    for (letter, runes) in &v.folds {
        let re = regex::Regex::new(&format!("^(?i:{letter})$")).unwrap();
        for c in runes {
            assert!(re.is_match(&c.to_string()), "{letter}: {c:?}");
        }
        for c in (0..128u8).map(char::from) {
            assert_eq!(
                re.is_match(&c.to_string()),
                runes.contains(&c),
                "{letter}: {c:?}"
            );
        }
        non_ascii.extend(runes.iter().copied().filter(|c| !c.is_ascii()));
    }
    let others: String = ('\u{80}'..=char::MAX).collect();
    let found: BTreeSet<char> = regex::Regex::new("(?i:[a-z])")
        .unwrap()
        .find_iter(&others)
        .flat_map(|m| m.as_str().chars())
        .collect();
    assert_eq!(found, non_ascii);
}

#[test]
fn parse_trees_match_go() {
    let v = vectors();
    assert!(v.inputs.len() > 1000, "{} inputs", v.inputs.len());
    for input in &v.inputs {
        assert_eq!(
            parse(&v.ctx, &input.body).to_string(),
            input.ast,
            "{}: {:?}",
            input.name,
            input.body
        );
    }
}

fn check_entities(name: &str, body: &str, got: &Entities, want: &[u8]) {
    let decoded = Entities::decode(want).unwrap_or_else(|e| panic!("{name}: decode: {e}"));
    assert_eq!(*got, decoded, "{name}: {body:?}");
    assert_eq!(
        hex::encode(got.encode_to_vec()),
        hex::encode(want),
        "{name}: encoding of {body:?}"
    );
}

#[test]
fn entities_match_go() {
    let v = vectors();
    let mut patterns = Patterns::new();
    let mut checked = 0;
    for input in &v.inputs {
        let Some(want) = &input.entities else {
            continue;
        };
        let got = extract(&input.body, &v.ctx, &mut patterns);
        check_entities(input.name, &input.body, &got, want);
        checked += 1;
    }
    assert!(checked > 1000, "{checked} entity vectors");
}

#[test]
fn internal_modifiers_match_go() {
    let v = vectors();
    assert!(!v.internal_modifiers.is_empty());
    let mut patterns = Patterns::new();
    let mut rng = det_rng(1);
    let mut checked = 0;
    for input in &v.inputs {
        let Some(want) = &input.internal else {
            continue;
        };
        let mut got = extract(&input.body, &v.ctx, &mut patterns);
        assert_eq!(got.emotes.len(), 1, "{}", input.name);
        add_internal_modifiers(&mut got, &v.internal_modifiers, &mut rng);
        check_entities(input.name, &input.body, &got, want);
        checked += 1;
    }
    assert!(checked > 100, "{checked} internal modifier vectors");
}
