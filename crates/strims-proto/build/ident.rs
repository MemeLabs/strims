// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Rust identifiers for generated items, spelled the way prost-build
//! spells them.

use heck::{ToSnakeCase, ToUpperCamelCase};

/// prost-build's `ident::sanitize_identifier` (not public), so generated
/// names match prost's exactly.
pub fn sanitize_identifier(ident: &str) -> String {
    match ident {
        "as" | "break" | "const" | "continue" | "else" | "enum" | "false" | "fn" | "for" | "if"
        | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub" | "ref"
        | "return" | "static" | "struct" | "trait" | "true" | "type" | "unsafe" | "use"
        | "where" | "while" | "dyn" | "abstract" | "become" | "box" | "do" | "final" | "macro"
        | "override" | "priv" | "typeof" | "unsized" | "virtual" | "yield" | "async" | "await"
        | "try" | "gen" => format!("r#{ident}"),
        "_" | "super" | "self" | "Self" | "extern" | "crate" => format!("{ident}_"),
        s if s.starts_with(|c: char| c.is_numeric()) => format!("_{ident}"),
        _ => ident.to_owned(),
    }
}

pub fn to_snake(s: &str) -> String {
    sanitize_identifier(&s.to_snake_case())
}

pub fn to_upper_camel(s: &str) -> String {
    sanitize_identifier(&s.to_upper_camel_case())
}
