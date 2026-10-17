// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
// Derived from chat-parser; see licenses/chat-parser.txt.

//! The lexer: splits a body's runes into tokens.

use super::tables::{NON_WORD, SPACE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Eof,
    /// `||`
    Spoiler,
    /// A non-word rune with no token of its own.
    Punct,
    Whitespace,
    Word,
    Backtick,
    Colon,
    RAngle,
    At,
    RSlash,
    /// A backslash and the rune after it.
    EscapeSeq,
}

/// A token over the runes `pos..end` of the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) pos: usize,
    pub(crate) end: usize,
}

pub(crate) struct Lexer<'a> {
    input: &'a [char],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(input: &'a [char]) -> Self {
        Self { input, pos: 0 }
    }

    fn accept(&mut self, test: impl Fn(char) -> bool) -> bool {
        match self.input.get(self.pos) {
            Some(&c) if test(c) => {
                self.pos += 1;
                true
            }
            _ => false,
        }
    }

    /// The next token. At the end of the input, every call returns an
    /// empty [`TokenKind::Eof`] token.
    pub(crate) fn next_token(&mut self) -> Token {
        let start = self.pos;
        let Some(&c) = self.input.get(start) else {
            return Token {
                kind: TokenKind::Eof,
                pos: start,
                end: start,
            };
        };
        self.pos += 1;
        let kind = match c {
            '`' => TokenKind::Backtick,
            ':' => TokenKind::Colon,
            '>' => TokenKind::RAngle,
            '@' => TokenKind::At,
            '/' => TokenKind::RSlash,
            '\\' if self.accept(|_| true) => TokenKind::EscapeSeq,
            '|' if self.accept(|c| c == '|') => TokenKind::Spoiler,
            '\\' | '|' => TokenKind::Punct,
            c if is_space(c) => {
                while self.accept(is_space) {}
                TokenKind::Whitespace
            }
            c if is_non_word(c) => TokenKind::Punct,
            _ => {
                while self.accept(|c| !is_non_word(c)) {}
                TokenKind::Word
            }
        };
        Token {
            kind,
            pos: start,
            end: self.pos,
        }
    }
}

fn is_space(c: char) -> bool {
    in_ranges(SPACE, c)
}

fn is_non_word(c: char) -> bool {
    in_ranges(NON_WORD, c)
}

/// Whether `c` is in `ranges`, sorted disjoint inclusive ranges.
fn in_ranges(ranges: &[(u32, u32)], c: char) -> bool {
    let c = u32::from(c);
    let i = ranges.partition_point(|&(lo, _)| lo <= c);
    i.checked_sub(1)
        .and_then(|i| ranges.get(i))
        .is_some_and(|&(_, hi)| c <= hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(input: &str) -> Vec<(TokenKind, usize, String)> {
        let chars: Vec<char> = input.chars().collect();
        let mut lexer = Lexer::new(&chars);
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token();
            let text = chars.get(t.pos..t.end).unwrap_or_default().iter().collect();
            tokens.push((t.kind, t.pos, text));
            if t.kind == TokenKind::Eof {
                return tokens;
            }
        }
    }

    fn tok(kind: TokenKind, pos: usize, text: &str) -> (TokenKind, usize, String) {
        (kind, pos, text.to_owned())
    }

    use TokenKind::*;

    #[test]
    fn at_without_username_in_spoiler() {
        assert_eq!(
            lex("||`||||@||"),
            [
                tok(Spoiler, 0, "||"),
                tok(Backtick, 2, "`"),
                tok(Spoiler, 3, "||"),
                tok(Spoiler, 5, "||"),
                tok(At, 7, "@"),
                tok(Spoiler, 8, "||"),
                tok(Eof, 10, ""),
            ]
        );
    }

    #[test]
    fn emote_with_trailing() {
        assert_eq!(lex("PEPE0"), [tok(Word, 0, "PEPE0"), tok(Eof, 5, "")]);
    }

    #[test]
    fn text_with_code() {
        assert_eq!(
            lex("text `with code`"),
            [
                tok(Word, 0, "text"),
                tok(Whitespace, 4, " "),
                tok(Backtick, 5, "`"),
                tok(Word, 6, "with"),
                tok(Whitespace, 10, " "),
                tok(Word, 11, "code"),
                tok(Backtick, 15, "`"),
                tok(Eof, 16, ""),
            ]
        );
    }

    #[test]
    fn underscores() {
        assert_eq!(
            lex("words_with_underscores"),
            [tok(Word, 0, "words_with_underscores"), tok(Eof, 22, "")]
        );
    }

    #[test]
    fn emoji() {
        assert_eq!(lex("🙈🙉🙊"), [tok(Word, 0, "🙈🙉🙊"), tok(Eof, 3, "")]);
    }

    #[test]
    fn non_ascii_words() {
        assert_eq!(
            lex("日本語のテキスト"),
            [tok(Word, 0, "日本語のテキスト"), tok(Eof, 8, "")]
        );
    }

    #[test]
    fn more_unicode() {
        assert_eq!(
            lex("Ǆ؁‱ஹ௸௵꧄.ဪ꧅⸻𒈙𒐫﷽"),
            [
                tok(Word, 0, "Ǆ؁"),
                tok(Punct, 2, "‱"),
                tok(Word, 3, "ஹ௸௵꧄"),
                tok(Punct, 7, "."),
                tok(Word, 8, "ဪ꧅"),
                tok(Punct, 10, "⸻"),
                tok(Word, 11, "𒈙𒐫﷽"),
                tok(Eof, 14, ""),
            ]
        );
    }

    #[test]
    fn at() {
        assert_eq!(lex("@"), [tok(At, 0, "@"), tok(Eof, 1, "")]);
    }

    #[test]
    fn escapes_and_lone_specials() {
        assert_eq!(
            lex("\\`|a\\"),
            [
                tok(EscapeSeq, 0, "\\`"),
                tok(Punct, 2, "|"),
                tok(Word, 3, "a"),
                tok(Punct, 4, "\\"),
                tok(Eof, 5, ""),
            ]
        );
    }

    #[test]
    fn whitespace_runs_merge() {
        assert_eq!(
            lex("a \t\u{a0}\u{3000}b"),
            [
                tok(Word, 0, "a"),
                tok(Whitespace, 1, " \t\u{a0}\u{3000}"),
                tok(Word, 5, "b"),
                tok(Eof, 6, ""),
            ]
        );
    }

    #[test]
    fn eof_repeats() {
        let chars: Vec<char> = "a".chars().collect();
        let mut lexer = Lexer::new(&chars);
        lexer.next_token();
        for _ in 0..3 {
            assert_eq!(
                lexer.next_token(),
                Token {
                    kind: Eof,
                    pos: 1,
                    end: 1
                }
            );
        }
    }

    #[test]
    fn range_lookups() {
        assert!(is_space(' '));
        assert!(is_space('\u{85}'));
        assert!(!is_space('a'));
        assert!(!is_space('\0'));
        assert!(!is_space(char::MAX));
        assert!(is_non_word(':'));
        assert!(is_non_word('\u{2e3b}'));
        assert!(!is_non_word('_'));
        assert!(!is_non_word('\0'));
        assert!(!is_non_word(char::MAX));
    }
}
