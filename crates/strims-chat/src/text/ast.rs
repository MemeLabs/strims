// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
// Derived from chat-parser; see licenses/chat-parser.txt.

//! The parse tree.
//!
//! Every node covers the runes `pos..end` of the body. `Display` writes a
//! node as an s-expression: `(<kind> <pos> <end> <children>...)` for a span,
//! `(Emote "<name>" <pos> <end> "<modifier>"...)`, `(Tag "<name>" <pos>
//! <end>)` and `(Nick "<nick>" <pos> <end>)`. Strings escape `\` and `"`
//! with a backslash and control runes as `\x{<hex>}`.

use std::fmt;

/// What a [`Span`] marks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanKind {
    /// A whole message.
    Message,
    /// Backtick-quoted code.
    Code,
    /// A whole message starting with `>`.
    Greentext,
    /// Text between `||` pairs.
    Spoiler,
    /// A whole message starting with `/me`. It covers the text after the
    /// command.
    Me,
}

impl SpanKind {
    fn name(self) -> &'static str {
        match self {
            Self::Message => "Message",
            Self::Code => "Code",
            Self::Greentext => "Greentext",
            Self::Spoiler => "Spoiler",
            Self::Me => "Me",
        }
    }
}

/// A run of the body and the nodes in it. Code spans hold no nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub kind: SpanKind,
    pub nodes: Vec<Node>,
    pub pos: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Span(Span),
    Emote(Emote),
    Tag(Tag),
    Nick(Nick),
}

impl Node {
    pub fn pos(&self) -> usize {
        match self {
            Self::Span(n) => n.pos,
            Self::Emote(n) => n.pos,
            Self::Tag(n) => n.pos,
            Self::Nick(n) => n.pos,
        }
    }

    pub fn end(&self) -> usize {
        match self {
            Self::Span(n) => n.end,
            Self::Emote(n) => n.end,
            Self::Tag(n) => n.end,
            Self::Nick(n) => n.end,
        }
    }
}

/// An emote name and the modifiers that follow it, each after a colon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Emote {
    pub name: String,
    pub modifiers: Vec<String>,
    pub pos: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    pub pos: usize,
    pub end: usize,
}

/// A mention of a nick, with or without a leading `@`. `nick` is the
/// indexed spelling, which may differ in case from the body's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nick {
    pub nick: String,
    pub peer_key: Vec<u8>,
    pub pos: usize,
    pub end: usize,
}

struct Quoted<'a>(&'a str);

impl fmt::Display for Quoted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        for c in self.0.chars() {
            match c {
                '\\' | '"' => write!(f, "\\{c}")?,
                c if c < '\u{20}' || c == '\u{7f}' => write!(f, "\\x{{{:X}}}", u32::from(c))?,
                c => write!(f, "{c}")?,
            }
        }
        f.write_str("\"")
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {} {}", self.kind.name(), self.pos, self.end)?;
        for node in &self.nodes {
            write!(f, " {node}")?;
        }
        f.write_str(")")
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Span(n) => n.fmt(f),
            Self::Emote(n) => n.fmt(f),
            Self::Tag(n) => n.fmt(f),
            Self::Nick(n) => n.fmt(f),
        }
    }
}

impl fmt::Display for Emote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(Emote {} {} {}", Quoted(&self.name), self.pos, self.end)?;
        for m in &self.modifiers {
            write!(f, " {}", Quoted(m))?;
        }
        f.write_str(")")
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(Tag {} {} {})", Quoted(&self.name), self.pos, self.end)
    }
}

impl fmt::Display for Nick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(Nick {} {} {})", Quoted(&self.nick), self.pos, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_as_an_s_expression() {
        let span = Span {
            kind: SpanKind::Message,
            pos: 0,
            end: 20,
            nodes: vec![
                Node::Emote(Emote {
                    name: "PEPE".into(),
                    modifiers: vec!["wide".into(), "flip".into()],
                    pos: 0,
                    end: 14,
                }),
                Node::Span(Span {
                    kind: SpanKind::Spoiler,
                    pos: 15,
                    end: 20,
                    nodes: vec![Node::Nick(Nick {
                        nick: "a\"b\\c\n".into(),
                        peer_key: vec![1],
                        pos: 17,
                        end: 18,
                    })],
                }),
                Node::Tag(Tag {
                    name: "nsfw".into(),
                    pos: 19,
                    end: 20,
                }),
            ],
        };
        assert_eq!(
            span.to_string(),
            r#"(Message 0 20 (Emote "PEPE" 0 14 "wide" "flip") (Spoiler 15 20 (Nick "a\"b\\c\x{A}" 17 18)) (Tag "nsfw" 19 20))"#
        );
    }
}
