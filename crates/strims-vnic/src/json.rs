// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Just enough JSON for the WebRTC session description that
//! `LinkDescription.description` carries: `{"type":"offer","sdp":"..."}`.
//!
//! - Encoding escapes `<`, `>`, `&`, U+2028 and U+2029 as `\u` sequences.
//!   The golden vectors pin the encoding byte for byte.
//! - Decoding accepts any JSON object: keys match ASCII case-insensitively,
//!   the last duplicate wins, unknown members of any type are skipped, and a
//!   `null` `sdp` leaves it empty. A `type` must be a known SDP type name.
//!   Browsers' `JSON.stringify(description)` output parses.

use crate::webrtc::{SdpType, SessionDescription};

/// Errors decoding a session description.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum JsonError {
    #[error("invalid json at byte {0}")]
    Syntax(usize),
    #[error("json nested too deeply")]
    TooDeep,
    /// A member had the wrong JSON type.
    #[error("json: cannot unmarshal {0}")]
    Type(&'static str),
    /// The `type` is missing, `null` or not a known SDP type.
    #[error("unknown sdp type")]
    UnknownSdpType,
}

/// Nesting allowed in skipped values.
const MAX_DEPTH: usize = 64;

/// Appends `s` as a JSON string, escaped as the module docs describe.
pub fn write_go_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '<' | '>' | '&' | '\u{2028}' | '\u{2029}' => push_u_escape(out, c as u32),
            c if (c as u32) < 0x20 => push_u_escape(out, c as u32),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn push_u_escape(out: &mut String, v: u32) {
    out.push_str("\\u");
    for shift in [12, 8, 4, 0] {
        out.push(char::from_digit((v >> shift) & 15, 16).unwrap_or('0'));
    }
}

/// Encodes `{"type":…,"sdp":…}`.
pub fn encode_session_description(d: &SessionDescription) -> String {
    let mut out = String::with_capacity(d.sdp.len() + 32);
    out.push_str("{\"type\":");
    write_go_string(&mut out, d.sdp_type.as_str());
    out.push_str(",\"sdp\":");
    write_go_string(&mut out, &d.sdp);
    out.push('}');
    out
}

/// Decodes a session description, as the module docs describe.
pub fn decode_session_description(s: &str) -> Result<SessionDescription, JsonError> {
    let mut p = Parser {
        b: s.as_bytes(),
        i: 0,
    };
    p.ws();
    p.expect(b'{')?;
    let mut sdp_type = None;
    let mut sdp = String::new();
    p.ws();
    if p.peek() == Some(b'}') {
        p.i += 1;
    } else {
        loop {
            p.ws();
            let key = p.string()?;
            p.ws();
            p.expect(b':')?;
            p.ws();
            if key.eq_ignore_ascii_case("type") {
                sdp_type = match p.peek() {
                    Some(b'"') => {
                        Some(SdpType::parse(&p.string()?).ok_or(JsonError::UnknownSdpType)?)
                    }
                    Some(b'n') => return Err(JsonError::UnknownSdpType),
                    _ => return Err(JsonError::Type("non-string into SDPType")),
                };
            } else if key.eq_ignore_ascii_case("sdp") {
                match p.peek() {
                    Some(b'"') => sdp = p.string()?,
                    Some(b'n') => p.literal(b"null")?,
                    _ => return Err(JsonError::Type("non-string into Go struct field sdp")),
                }
            } else {
                p.skip_value(0)?;
            }
            p.ws();
            match p.next() {
                Some(b',') => continue,
                Some(b'}') => break,
                _ => return Err(JsonError::Syntax(p.i)),
            }
        }
    }
    p.ws();
    if p.i != p.b.len() {
        return Err(JsonError::Syntax(p.i));
    }
    let sdp_type = sdp_type.ok_or(JsonError::UnknownSdpType)?;
    Ok(SessionDescription { sdp_type, sdp })
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.i += 1;
        Some(c)
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), JsonError> {
        if self.next() == Some(c) {
            Ok(())
        } else {
            Err(JsonError::Syntax(self.i))
        }
    }

    fn literal(&mut self, lit: &[u8]) -> Result<(), JsonError> {
        if self.b.get(self.i..self.i + lit.len()) == Some(lit) {
            self.i += lit.len();
            Ok(())
        } else {
            Err(JsonError::Syntax(self.i))
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let mut v = 0;
        for _ in 0..4 {
            let d = self
                .next()
                .and_then(|c| char::from(c).to_digit(16))
                .ok_or(JsonError::Syntax(self.i))?;
            v = v << 4 | d;
        }
        Ok(v)
    }

    /// A string; lone surrogates become U+FFFD.
    fn string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut out = Vec::new();
        loop {
            let start = self.i;
            while let Some(c) = self.peek() {
                if c == b'"' || c == b'\\' || c < 0x20 {
                    break;
                }
                self.i += 1;
            }
            out.extend_from_slice(self.b.get(start..self.i).unwrap_or_default());
            match self.next() {
                Some(b'"') => break,
                Some(b'\\') => {
                    let c = match self.next() {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'/') => '/',
                        Some(b'b') => '\u{8}',
                        Some(b'f') => '\u{c}',
                        Some(b'n') => '\n',
                        Some(b'r') => '\r',
                        Some(b't') => '\t',
                        Some(b'u') => {
                            let hi = self.hex4()?;
                            if (0xd800..0xdc00).contains(&hi)
                                && self.b.get(self.i..self.i + 2) == Some(b"\\u")
                            {
                                let save = self.i;
                                self.i += 2;
                                let lo = self.hex4()?;
                                if (0xdc00..0xe000).contains(&lo) {
                                    let v = 0x10000 + ((hi - 0xd800) << 10) + (lo - 0xdc00);
                                    char::from_u32(v).unwrap_or('\u{fffd}')
                                } else {
                                    self.i = save;
                                    '\u{fffd}'
                                }
                            } else {
                                char::from_u32(hi).unwrap_or('\u{fffd}')
                            }
                        }
                        _ => return Err(JsonError::Syntax(self.i)),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
                _ => return Err(JsonError::Syntax(self.i)),
            }
        }
        // The input is a &str and escapes produce whole chars.
        String::from_utf8(out).map_err(|_| JsonError::Syntax(self.i))
    }

    fn number(&mut self) -> Result<(), JsonError> {
        let digits = |p: &mut Self| {
            let s = p.i;
            while matches!(p.peek(), Some(b'0'..=b'9')) {
                p.i += 1;
            }
            p.i > s
        };
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                digits(self);
            }
            _ => return Err(JsonError::Syntax(self.i)),
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            if !digits(self) {
                return Err(JsonError::Syntax(self.i));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !digits(self) {
                return Err(JsonError::Syntax(self.i));
            }
        }
        Ok(())
    }

    fn skip_value(&mut self, depth: usize) -> Result<(), JsonError> {
        if depth > MAX_DEPTH {
            return Err(JsonError::TooDeep);
        }
        match self.peek() {
            Some(b'"') => self.string().map(drop),
            Some(b't') => self.literal(b"true"),
            Some(b'f') => self.literal(b"false"),
            Some(b'n') => self.literal(b"null"),
            Some(b'{') => {
                self.i += 1;
                self.ws();
                if self.peek() == Some(b'}') {
                    self.i += 1;
                    return Ok(());
                }
                loop {
                    self.ws();
                    self.string()?;
                    self.ws();
                    self.expect(b':')?;
                    self.ws();
                    self.skip_value(depth + 1)?;
                    self.ws();
                    match self.next() {
                        Some(b',') => continue,
                        Some(b'}') => return Ok(()),
                        _ => return Err(JsonError::Syntax(self.i)),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                self.ws();
                if self.peek() == Some(b']') {
                    self.i += 1;
                    return Ok(());
                }
                loop {
                    self.ws();
                    self.skip_value(depth + 1)?;
                    self.ws();
                    match self.next() {
                        Some(b',') => continue,
                        Some(b']') => return Ok(()),
                        _ => return Err(JsonError::Syntax(self.i)),
                    }
                }
            }
            _ => self.number(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn encodes_with_html_and_line_separator_escapes() {
        let d = SessionDescription {
            sdp_type: SdpType::Offer,
            sdp: "v=0\r\na=<&>\u{2028}\"\\\u{1}\u{8}\u{c}\té".to_owned(),
        };
        assert_eq!(
            encode_session_description(&d),
            r#"{"type":"offer","sdp":"v=0\r\na=\u003c\u0026\u003e\u2028\"\\\u0001\b\f\té"}"#
        );
        assert_eq!(
            decode_session_description(&encode_session_description(&d)).unwrap(),
            d
        );
    }

    #[test]
    fn decodes_any_object_with_a_known_type() {
        let d = decode_session_description(
            " {\"SDP\" : \"a\\/b\\ud83d\\ude00\\ud800x\", \"extra\": [1, -2.5e3, {\"k\": null}, true], \"Type\":\"ANSWER\"} ",
        )
        .unwrap();
        assert_eq!(d.sdp_type, SdpType::Answer);
        assert_eq!(d.sdp, "a/b\u{1f600}\u{fffd}x");
        // last duplicate wins; null sdp leaves it empty
        let d =
            decode_session_description(r#"{"type":"offer","type":"pranswer","sdp":null}"#).unwrap();
        assert_eq!(d.sdp_type, SdpType::Pranswer);
        assert_eq!(d.sdp, "");
        assert_eq!(
            decode_session_description(r#"{"sdp":"x"}"#),
            Err(JsonError::UnknownSdpType)
        );
        assert_eq!(
            decode_session_description(r#"{"type":"bogus"}"#),
            Err(JsonError::UnknownSdpType)
        );
        assert_eq!(
            decode_session_description(r#"{"type":null}"#),
            Err(JsonError::UnknownSdpType)
        );
        assert!(decode_session_description(r#"{"type":"offer"} x"#).is_err());
        assert!(decode_session_description(r#"{"type":"offer","sdp":1}"#).is_err());
        assert!(decode_session_description("{\"type\":\"offer\",\"sdp\":\"\u{1}\"}").is_err());
        let deep = format!("{{\"x\":{}{}}}", "[".repeat(100), "]".repeat(100));
        assert_eq!(decode_session_description(&deep), Err(JsonError::TooDeep));
    }
}
