// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Tests panic on failure by design.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use strims_base::rng::det_rng;

use super::*;

fn ctx() -> ParserContext {
    let mut ctx = ParserContext::new();
    ctx.set_emotes([
        "INFESTOR",
        "FIDGETLOL",
        "Hhhehhehe",
        "GameOfThrows",
        "Abathur",
        "LUL",
        "PEPE",
        "Kappa",
        "CuckCrab",
        "WEEWOO",
        "POTATO",
        "monkaS",
        "AYAYA",
        "KEKW",
    ]);
    ctx.set_tags(["nsfw", "weeb", "nsfl", "loud"]);
    ctx.set_modifiers([
        "mirror", "flip", "rain", "snow", "rustle", "worth", "love", "spin", "wide", "lag", "hyper",
    ]);
    ctx
}

fn b(start: u32, end: u32) -> Option<Bounds> {
    Some(Bounds { start, end })
}

fn link(url: &str, start: u32, end: u32) -> Link {
    Link {
        bounds: b(start, end),
        url: url.into(),
    }
}

fn emote(name: &str, start: u32, end: u32) -> Emote {
    Emote {
        bounds: b(start, end),
        name: name.into(),
        ..Emote::default()
    }
}

fn emoji(start: u32, end: u32) -> Emoji {
    Emoji {
        bounds: b(start, end),
    }
}

fn tag(name: &str, start: u32, end: u32) -> Tag {
    Tag {
        bounds: b(start, end),
        name: name.into(),
    }
}

fn cases() -> Vec<(&'static str, &'static str, Entities)> {
    vec![
        ("basic", "aaaaaaaaaa", Entities::default()),
        (
            "emote",
            "test PEPE test",
            Entities {
                emotes: vec![emote("PEPE", 5, 9)],
                ..Entities::default()
            },
        ),
        (
            "link",
            "strims.gg",
            Entities {
                links: vec![link("https://strims.gg", 0, 9)],
                ..Entities::default()
            },
        ),
        (
            "link with scheme",
            "http://strims.gg",
            Entities {
                links: vec![link("http://strims.gg", 0, 16)],
                ..Entities::default()
            },
        ),
        (
            "spoiler",
            "spoiler ||dumbledore was gay all along||",
            Entities {
                spoilers: vec![Spoiler { bounds: b(8, 40) }],
                ..Entities::default()
            },
        ),
        (
            "greentext",
            ">implying greentext doesn't work",
            Entities {
                green_text: Some(GenericEntity { bounds: b(0, 32) }),
                ..Entities::default()
            },
        ),
        (
            "self",
            "/me dies",
            Entities {
                self_message: Some(GenericEntity { bounds: b(4, 8) }),
                ..Entities::default()
            },
        ),
        (
            "tag",
            "nsfw loud weeb nsfl google.com",
            Entities {
                links: vec![link("https://google.com", 20, 30)],
                tags: vec![
                    tag("nsfw", 0, 4),
                    tag("loud", 5, 9),
                    tag("weeb", 10, 14),
                    tag("nsfl", 15, 19),
                ],
                ..Entities::default()
            },
        ),
        (
            "code",
            "`hacker mode activated`",
            Entities {
                code_blocks: vec![CodeBlock { bounds: b(0, 23) }],
                ..Entities::default()
            },
        ),
        (
            "entity in link",
            "strims.gg/weeb",
            Entities {
                links: vec![link("https://strims.gg/weeb", 0, 14)],
                ..Entities::default()
            },
        ),
        (
            "unicode flags",
            "🇺🇸🏳️‍🌈🏴🏴󠁧󠁢󠁥󠁮󠁧󠁿🏴‍☠️",
            Entities {
                emojis: vec![
                    emoji(0, 2),
                    emoji(2, 6),
                    emoji(6, 7),
                    emoji(7, 14),
                    emoji(14, 18),
                ],
                ..Entities::default()
            },
        ),
        (
            "characters",
            "🦸‍♂️🧑‍🚀🦍",
            Entities {
                emojis: vec![emoji(0, 4), emoji(4, 7), emoji(7, 8)],
                ..Entities::default()
            },
        ),
        // beyond internal/chat's tests
        (
            "emoji in code and links",
            "é `😀` strims.gg/😀 😀",
            Entities {
                links: vec![link("https://strims.gg/😀", 6, 17)],
                code_blocks: vec![CodeBlock { bounds: b(2, 5) }],
                emojis: vec![emoji(18, 19)],
                ..Entities::default()
            },
        ),
        (
            "longest link",
            "mail first.last@mail.co.uk",
            Entities {
                links: vec![link("https://first.last@mail.co.uk", 5, 26)],
                ..Entities::default()
            },
        ),
        (
            "scheme check is case-sensitive",
            "MAILTO:a@b.com",
            Entities {
                links: vec![link("https://MAILTO:a@b.com", 0, 14)],
                ..Entities::default()
            },
        ),
        (
            "emote with modifier in a link",
            "strims.gg/PEPE:wide PEPE",
            Entities {
                links: vec![link("https://strims.gg/PEPE:wide", 0, 19)],
                emotes: vec![emote("PEPE", 20, 24)],
                ..Entities::default()
            },
        ),
    ]
}

#[test]
fn entity_cases() {
    let ctx = ctx();
    let mut patterns = Patterns::new();
    for (name, input, want) in cases() {
        assert_eq!(
            extract(input, &ctx, &mut patterns),
            want,
            "{name}: {input:?}"
        );
    }
}

#[test]
fn nicks() {
    let mut ctx = ctx();
    let mut patterns = Patterns::new();
    let message = "hey bob how are you";

    assert!(extract(message, &ctx, &mut patterns).nicks.is_empty());

    assert!(ctx.nicks_mut().insert("bob", b"test".to_vec()));
    let nicks = extract(message, &ctx, &mut patterns).nicks;
    assert_eq!(
        nicks,
        [Nick {
            bounds: b(4, 7),
            nick: "bob".into(),
            peer_key: b"test".to_vec(),
            viewed_listing: None,
        }]
    );

    ctx.nicks_mut().remove("bob");
    assert!(extract(message, &ctx, &mut patterns).nicks.is_empty());
}

fn count(e: &Entities) -> usize {
    e.links.len()
        + e.emotes.len()
        + e.emojis.len()
        + e.nicks.len()
        + e.tags.len()
        + e.code_blocks.len()
        + e.spoilers.len()
        + usize::from(e.green_text.is_some())
        + usize::from(e.self_message.is_some())
}

#[test]
fn entities_are_bounded() {
    let ctx = ctx();
    let mut patterns = Patterns::new();
    for body in [
        "😀".repeat(2 * MAX_ENTITIES),
        "a.io ".repeat(2 * MAX_ENTITIES),
        "PEPE 😀 ".repeat(MAX_ENTITIES),
        "a.io PEPE 😀 `x` ||y|| ".repeat(MAX_ENTITIES),
    ] {
        assert_eq!(count(&extract(&body, &ctx, &mut patterns)), MAX_ENTITIES);
    }
}

#[test]
fn hostile_bodies_extract_without_panicking() {
    let ctx = ctx();
    let mut patterns = Patterns::new();
    for body in [
        String::new(),
        "\u{0}".repeat(100),
        "\u{10ffff}".repeat(100),
        "http://".repeat(500),
        "a.".repeat(1000) + "com",
        "1:".repeat(1000),
        "[".repeat(1000) + "::1",
        "(".repeat(1000) + "a.io/" + &")".repeat(1000),
        "x.com/".to_owned() + &"(a".repeat(1000),
        "\u{200d}\u{fe0f}".repeat(1000),
    ] {
        let e = extract(&body, &ctx, &mut patterns);
        let runes = u32::try_from(body.chars().count()).unwrap();
        for l in &e.links {
            let b = l.bounds.unwrap();
            assert!(b.start < b.end && b.end <= runes);
        }
        for m in &e.emojis {
            let b = m.bounds.unwrap();
            assert!(b.start < b.end && b.end <= runes);
        }
    }
}

#[test]
fn search_caches_stay_bounded() {
    use rand_core::Rng;

    const FRAGMENTS: &[&str] = &[
        "a", "ü", "日本", "0", "1", "255", "fe80", ".", ".com", ".io", ".рф", "@", ":", "::",
        "://", "/", "[", "]", "(", ")", "%", "-", "_", "#", "?", "=", "&", "!", ",", " ", "http",
        "mailto:", "xn--p1ai", "😀", "👍🏽", "🇺🇸", "\u{200d}", "\u{fe0f}",
    ];
    let ctx = ctx();
    let mut patterns = Patterns::new();
    let mut rng = det_rng(5);
    let mut peak = [0; 3];
    for _ in 0..2000 {
        let mut body = String::new();
        for _ in 0..1 + rng.next_u64() % 64 {
            body.push_str(FRAGMENTS[(rng.next_u64() % FRAGMENTS.len() as u64) as usize]);
        }
        extract(&body, &ctx, &mut patterns);
        for (peak, usage) in peak.iter_mut().zip([
            patterns.link_cache.memory_usage(),
            patterns.link_longest_cache.memory_usage(),
            patterns.emoji_cache.memory_usage(),
        ]) {
            *peak = usage.max(*peak);
        }
    }
    // A meta cache holds a forward and a reverse lazy DFA cache beside its
    // NFA simulations' state.
    let [link, longest, emoji] = peak;
    assert!(longest <= LAZY_CACHE_BYTES, "{longest}");
    assert!(link <= 3 * LAZY_CACHE_BYTES, "{link}");
    assert!(emoji <= 3 * LAZY_CACHE_BYTES, "{emoji}");
}

fn modifier(name: &str, proc_chance: f64) -> Modifier {
    Modifier {
        name: name.into(),
        internal: true,
        proc_chance,
        ..Modifier::default()
    }
}

#[test]
fn internal_modifiers_proc_by_chance() {
    let ctx = ctx();
    let mut patterns = Patterns::new();
    let base = extract("PEPE:wide LUL Kappa", &ctx, &mut patterns);
    let modifiers = [
        modifier("always", 1.0),
        modifier("never", 0.0),
        modifier("half", 0.5),
    ];

    let mut a = base.clone();
    add_internal_modifiers(&mut a, &modifiers, &mut det_rng(7));
    let mut b = base.clone();
    add_internal_modifiers(&mut b, &modifiers, &mut det_rng(7));
    assert_eq!(a, b, "same seed, same modifiers");

    let added: Vec<&str> = a
        .emotes
        .iter()
        .flat_map(|e| e.modifiers.iter().map(String::as_str))
        .filter(|m| *m != "wide")
        .collect();
    assert!(added.contains(&"always"));
    assert!(!added.contains(&"never"));

    let mut halves = 0;
    let mut rng = det_rng(1);
    for _ in 0..1000 {
        let mut e = base.clone();
        add_internal_modifiers(&mut e, &[modifier("half", 0.5)], &mut rng);
        halves += e
            .emotes
            .iter()
            .filter(|e| e.modifiers.iter().any(|m| m == "half"))
            .count();
    }
    assert!((400..600).contains(&halves), "{halves}");
}

#[test]
fn internal_modifiers_skip_messages_without_emotes() {
    let mut e = Entities::default();
    let mut rng = det_rng(3);
    add_internal_modifiers(&mut e, &[modifier("always", 1.0)], &mut rng);
    assert_eq!(e, Entities::default());
    let mut fresh = det_rng(3);
    assert_eq!(
        rand_core::Rng::next_u64(&mut rng),
        rand_core::Rng::next_u64(&mut fresh)
    );
}
