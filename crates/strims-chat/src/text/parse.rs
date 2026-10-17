// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
// Derived from chat-parser; see licenses/chat-parser.txt.

//! The parser: builds a [`Span`] tree from the lexer's tokens.

use super::ast::{Emote, Nick, Node, Span, SpanKind, Tag};
use super::context::{NickEntry, ParserContext};
use super::lex::{Lexer, Token, TokenKind};

/// The most nodes and emote modifiers, together, one parse records.
/// Parsing continues past the bound, so span bounds still cover the whole
/// body, but further nodes and modifiers are dropped.
pub const MAX_NODES: usize = 1024;

/// Parses `body` into a [`SpanKind::Message`], [`SpanKind::Greentext`] or
/// [`SpanKind::Me`] span.
///
/// Positions count runes. Work is linear in the body's length, and the
/// tree is at most three spans deep: a spoiler closes at the next `||`, so
/// only the top span holds spoilers, and code spans hold no nodes. The
/// parser recurses once at most.
pub fn parse(ctx: &ParserContext, body: &str) -> Span {
    let input: Vec<char> = body.chars().collect();
    Parser {
        ctx,
        input: &input,
        lexer: Lexer::new(&input),
        tok: Token {
            kind: TokenKind::Eof,
            pos: 0,
            end: 0,
        },
        budget: MAX_NODES,
        scratch: Vec::new(),
    }
    .parse_span(SpanKind::Message)
}

struct Parser<'a> {
    ctx: &'a ParserContext,
    input: &'a [char],
    lexer: Lexer<'a>,
    tok: Token,
    /// Nodes and modifiers left to record.
    budget: usize,
    scratch: Vec<char>,
}

impl<'a> Parser<'a> {
    fn next(&mut self) {
        self.tok = self.lexer.next_token();
    }

    fn pos(&self) -> usize {
        self.tok.pos
    }

    fn lit(&self) -> &'a [char] {
        self.input
            .get(self.tok.pos..self.tok.end)
            .unwrap_or_default()
    }

    fn lit_string(&self) -> String {
        self.lit().iter().collect()
    }

    fn take_budget(&mut self) -> bool {
        match self.budget.checked_sub(1) {
            Some(left) => {
                self.budget = left;
                true
            }
            None => false,
        }
    }

    fn insert(&mut self, span: &mut Span, node: Node) {
        if self.take_budget() {
            span.nodes.push(node);
        }
    }

    fn nick(&mut self, runes: &[char]) -> Option<&'a NickEntry> {
        self.ctx.nicks().get_runes(runes, &mut self.scratch)
    }

    fn parse_emote(&mut self) -> Emote {
        let mut e = Emote {
            name: self.lit_string(),
            modifiers: Vec::new(),
            pos: self.pos(),
            end: 0,
        };
        loop {
            self.next();
            e.end = self.pos();
            if self.tok.kind != TokenKind::Colon {
                return e;
            }
            self.next();
            if !self.ctx.is_modifier(self.lit()) {
                return e;
            }
            if self.take_budget() {
                e.modifiers.push(self.lit_string());
            }
        }
    }

    fn parse_tag(&mut self) -> Tag {
        let name = self.lit_string();
        let pos = self.pos();
        self.next();
        Tag {
            name,
            pos,
            end: self.pos(),
        }
    }

    fn parse_nick(&mut self, entry: &NickEntry) -> Nick {
        let pos = self.pos();
        self.next();
        Nick {
            nick: entry.nick.clone(),
            peer_key: entry.peer_key.clone(),
            pos,
            end: self.pos(),
        }
    }

    /// At an `@`: the nick in the next token, if it is one, with the `@`
    /// included in its bounds.
    fn try_parse_at_nick(&mut self) -> Option<Nick> {
        let at = self.pos();
        self.next();
        let entry = self.nick(self.lit())?;
        let mut n = self.parse_nick(entry);
        n.pos = at;
        Some(n)
    }

    /// At a backtick: everything up to and including the next backtick, or
    /// to the end of the body.
    fn parse_code(&mut self) -> Span {
        let pos = self.pos();
        while self.tok.kind != TokenKind::Eof {
            self.next();
            if self.tok.kind == TokenKind::Backtick {
                self.next();
                break;
            }
        }
        Span {
            kind: SpanKind::Code,
            nodes: Vec::new(),
            pos,
            end: self.pos(),
        }
    }

    fn parse_span(&mut self, kind: SpanKind) -> Span {
        let mut s = Span {
            kind,
            nodes: Vec::new(),
            pos: self.pos(),
            end: 0,
        };
        self.next();

        if kind == SpanKind::Message {
            match self.tok.kind {
                TokenKind::RAngle => {
                    s.kind = SpanKind::Greentext;
                    self.next();
                }
                TokenKind::RSlash => {
                    self.next();
                    if self.lit() == ['m', 'e'] {
                        s.kind = SpanKind::Me;
                        self.next();
                        self.next();
                        s.pos = self.pos();
                    }
                }
                _ => {}
            }
        }

        loop {
            match self.tok.kind {
                TokenKind::Eof => {
                    s.end = self.pos();
                    return s;
                }
                TokenKind::Spoiler if kind == SpanKind::Spoiler => {
                    self.next();
                    s.end = self.pos();
                    return s;
                }
                TokenKind::Spoiler => {
                    let spoiler = self.parse_span(SpanKind::Spoiler);
                    self.insert(&mut s, Node::Span(spoiler));
                }
                TokenKind::Backtick => {
                    let code = self.parse_code();
                    self.insert(&mut s, Node::Span(code));
                }
                TokenKind::At => {
                    if let Some(n) = self.try_parse_at_nick() {
                        self.insert(&mut s, Node::Nick(n));
                    }
                }
                TokenKind::Word => {
                    let lit = self.lit();
                    if self.ctx.is_tag(lit) {
                        let t = self.parse_tag();
                        self.insert(&mut s, Node::Tag(t));
                    } else if self.ctx.is_emote(lit) {
                        let e = self.parse_emote();
                        self.insert(&mut s, Node::Emote(e));
                    } else if let Some(entry) = self.nick(lit) {
                        let n = self.parse_nick(entry);
                        self.insert(&mut s, Node::Nick(n));
                    } else {
                        self.next();
                    }
                }
                _ => self.next(),
            }
        }
    }
}

#[cfg(test)]
#[path = "parse_test.rs"]
mod tests;
