// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
// Derived from chat-parser; see licenses/chat-parser.txt.

// Tests panic on failure by design.
#![allow(clippy::panic)]

use super::*;

fn ctx() -> ParserContext {
    let mut ctx = ParserContext::new();
    ctx.set_emotes(["PEPE", "CuckCrab"]);
    ctx.set_modifiers(["wide", "rustle", "spin"]);
    ctx.set_tags(["nsfw"]);
    for nick in ["abeous", "jeanpierrepratt", "wrxst"] {
        assert!(ctx.nicks_mut().insert(nick, nick.as_bytes().to_vec()));
    }
    ctx
}

const CASES: &[(&str, &str, &str)] = &[
    (
        "at without username in spoiler",
        "||`||||@||",
        "(Message 0 10 (Spoiler 0 10 (Code 2 10)))",
    ),
    ("emote with trailing", "PEPE0", "(Message 0 5)"),
    (
        "text with code",
        "text `with code`",
        "(Message 0 16 (Code 5 16))",
    ),
    ("just code", "`just code`", "(Message 0 11 (Code 0 11))"),
    (
        "unclosed code tag",
        "text `code?",
        "(Message 0 11 (Code 5 11))",
    ),
    ("avoid out of range", "text `", "(Message 0 6 (Code 5 6))"),
    ("just text", "why even test this case?", "(Message 0 24)"),
    (
        "text and spoiler",
        "text ||and a spoiler||",
        "(Message 0 22 (Spoiler 5 22))",
    ),
    (
        "just spoiler",
        "||spoiler||",
        "(Message 0 11 (Spoiler 0 11))",
    ),
    (
        "code and spoiler",
        "`code` and ||spoiler||",
        "(Message 0 22 (Code 0 6) (Spoiler 11 22))",
    ),
    (
        "spoiler and code",
        "||spoiler|| and `code`",
        "(Message 0 22 (Spoiler 0 11) (Code 16 22))",
    ),
    ("empty code", "``", "(Message 0 2 (Code 0 2))"),
    ("empty spoiler", "||||", "(Message 0 4 (Spoiler 0 4))"),
    ("spoiler out of range", "|", "(Message 0 1)"),
    ("spoiler meme", "|||", "(Message 0 3 (Spoiler 0 3))"),
    ("just emote", "PEPE", r#"(Message 0 4 (Emote "PEPE" 0 4))"#),
    (
        "text and emote",
        "haha PEPE test",
        r#"(Message 0 14 (Emote "PEPE" 5 9))"#,
    ),
    (
        "emote with modifier",
        "PEPE:wide",
        r#"(Message 0 9 (Emote "PEPE" 0 9 "wide"))"#,
    ),
    (
        "text and emote with modifier",
        "haha PEPE:wide test",
        r#"(Message 0 19 (Emote "PEPE" 5 14 "wide"))"#,
    ),
    (
        "emote in spoiler",
        "test ||spoiler PEPE ||",
        r#"(Message 0 22 (Spoiler 5 22 (Emote "PEPE" 15 19)))"#,
    ),
    (
        "emote at the end of a spoiler",
        "test ||spoiler PEPE||",
        r#"(Message 0 21 (Spoiler 5 21 (Emote "PEPE" 15 19)))"#,
    ),
    (
        "emote in spoiler with mod",
        "||spoiler PEPE:wide||",
        r#"(Message 0 21 (Spoiler 0 21 (Emote "PEPE" 10 19 "wide")))"#,
    ),
    (
        "emote with mod in middle of spoiler",
        "||spoiler PEPE:wide spoiler||",
        r#"(Message 0 29 (Spoiler 0 29 (Emote "PEPE" 10 19 "wide")))"#,
    ),
    (
        "uneven spoiler",
        "test ||spoiler uneven",
        "(Message 0 21 (Spoiler 5 21))",
    ),
    (
        "uneven code",
        "test `spoiler uneven",
        "(Message 0 20 (Code 5 20))",
    ),
    (
        "lots of stuff",
        "text and `code PEPE` and maybe ||a spoiler PEPE:wide CuckCrab|| `...`",
        r#"(Message 0 69 (Code 9 20) (Spoiler 31 63 (Emote "PEPE" 43 52 "wide") (Emote "CuckCrab" 53 61)) (Code 64 69))"#,
    ),
    (
        "code around spoiler markers",
        "`||`||`Abathur:flip `||",
        "(Message 0 23 (Code 0 4) (Spoiler 4 23 (Code 6 21)))",
    ),
    (
        "greentext",
        ">implying this lexer works",
        "(Greentext 0 26)",
    ),
    (
        "greentext after text",
        "text >greentext ||spoiler|| greentext agane",
        "(Message 0 43 (Spoiler 16 27))",
    ),
    (
        "greentext after text with nodes",
        "text >greentext ||spoiler|| PEPE CuckCrab:spin greentext `code` agane",
        r#"(Message 0 69 (Spoiler 16 27) (Emote "PEPE" 28 32) (Emote "CuckCrab" 33 46 "spin") (Code 57 63))"#,
    ),
    (
        "username",
        "jeanpierrepratt hi",
        r#"(Message 0 18 (Nick "jeanpierrepratt" 0 15))"#,
    ),
    (
        "username with at",
        "@abeous hi",
        r#"(Message 0 10 (Nick "abeous" 0 7))"#,
    ),
    (
        "incorrectly capitalized username",
        "@ABEOUS hi",
        r#"(Message 0 10 (Nick "abeous" 0 7))"#,
    ),
    (
        "username in spoiler",
        "hi ||@wrxst||",
        r#"(Message 0 13 (Spoiler 3 13 (Nick "wrxst" 5 11)))"#,
    ),
    ("emoji", "🙈🙉🙊", "(Message 0 3)"),
    ("non ascii words", "日本語のテキスト", "(Message 0 8)"),
    ("more non ascii chars", "Ǆ؁‱ஹ௸௵꧄.ဪ꧅⸻𒈙𒐫﷽", "(Message 0 14)"),
    (
        "code spoiler mashup",
        "||`||`",
        "(Message 0 6 (Spoiler 0 6 (Code 2 6)))",
    ),
    ("at", "@", "(Message 0 1)"),
    ("me", "/me test", "(Me 4 8)"),
    ("me with multiple spaces", "/me    test", "(Me 7 11)"),
    (
        "escape sequences",
        "\\` test `co\\`de`",
        "(Message 0 16 (Code 8 16))",
    ),
    ("backslash", "\\", "(Message 0 1)"),
    // beyond chat-parser's tests
    ("empty", "", "(Message 0 0)"),
    ("bare me", "/me", "(Me 3 3)"),
    ("me needs the exact word", "/Me test", "(Message 0 8)"),
    (
        "tag before emote",
        "nsfw PEPE nsfw:wide",
        r#"(Message 0 19 (Tag "nsfw" 0 4) (Emote "PEPE" 5 9) (Tag "nsfw" 10 14))"#,
    ),
    (
        "unknown modifier ends the emote",
        "PEPE:nope:wide",
        r#"(Message 0 14 (Emote "PEPE" 0 4))"#,
    ),
    (
        "emote after a colon",
        "PEPE:PEPE",
        r#"(Message 0 9 (Emote "PEPE" 0 4) (Emote "PEPE" 5 9))"#,
    ),
    (
        "chained modifiers",
        "PEPE:wide:spin:rustle",
        r#"(Message 0 21 (Emote "PEPE" 0 21 "wide" "spin" "rustle"))"#,
    ),
    (
        "at before a non-nick",
        "@PEPE",
        r#"(Message 0 5 (Emote "PEPE" 1 5))"#,
    ),
];

#[test]
fn chat_parser_cases() {
    let ctx = ctx();
    for (name, input, want) in CASES {
        assert_eq!(parse(&ctx, input).to_string(), *want, "{name}: {input:?}");
    }
}

#[test]
fn nicks_carry_their_peer_key() {
    let span = parse(&ctx(), "hi WRXST");
    let [Node::Nick(n)] = span.nodes.as_slice() else {
        panic!("{span}");
    };
    assert_eq!(n.nick, "wrxst");
    assert_eq!(n.peer_key, b"wrxst");
}

#[test]
fn nodes_beyond_the_bound_are_dropped() {
    let ctx = ctx();
    let body = "PEPE:wide ".repeat(MAX_NODES);
    let span = parse(&ctx, &body);
    assert_eq!(span.end, body.chars().count());
    let modifiers: usize = span
        .nodes
        .iter()
        .map(|n| match n {
            Node::Emote(e) => e.modifiers.len(),
            _ => 0,
        })
        .sum();
    assert_eq!(span.nodes.len() + modifiers, MAX_NODES);

    let body = "||PEPE|| ".repeat(MAX_NODES);
    let span = parse(&ctx, &body);
    let nested: usize = span
        .nodes
        .iter()
        .map(|n| match n {
            Node::Span(s) => s.nodes.len(),
            _ => 0,
        })
        .sum();
    assert_eq!(span.nodes.len() + nested, MAX_NODES);
}

#[test]
fn trees_are_at_most_three_spans_deep() {
    fn depth(span: &Span) -> usize {
        1 + span
            .nodes
            .iter()
            .filter_map(|n| match n {
                Node::Span(s) => Some(depth(s)),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }
    let ctx = ctx();
    for body in [
        "||".repeat(1000),
        "|| `".repeat(1000),
        "`||".repeat(1000),
        ">||`||`".repeat(500),
    ] {
        assert!(depth(&parse(&ctx, &body)) <= 3, "{body}");
    }
}
