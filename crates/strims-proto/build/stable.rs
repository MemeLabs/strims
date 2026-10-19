// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Emits `<package>.stable.rs`: `StableEncode` impls in the format
//! documented in `src/stable.rs`.
//!
//! Fields are emitted in descriptor order, which is `.proto` declaration
//! order; each oneof is emitted once, at its first declared member.
//!
//! Messages with a map field, or that reach one through a message-typed
//! field, get no impl, so encoding one is a compile error.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use heck::{ToSnakeCase, ToUpperCamelCase};
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::{DescriptorProto, FieldDescriptorProto, FileDescriptorSet};

use crate::BoxError;

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

/// A message and where prost puts it.
struct Message<'a> {
    desc: &'a DescriptorProto,
    package: &'a str,
    /// Proto names from the outermost message to this one.
    path: Vec<&'a str>,
}

impl Message<'_> {
    /// Rust path relative to the package module, e.g. `foo::Bar`.
    fn rust_path(&self) -> String {
        let mut parts: Vec<String> = Vec::with_capacity(self.path.len());
        if let Some((last, outer)) = self.path.split_last() {
            parts.extend(outer.iter().map(|p| to_snake(p)));
            parts.push(to_upper_camel(last));
        }
        parts.join("::")
    }

    /// Path of the module prost generates for this message's nested types.
    fn module_path(&self) -> String {
        self.path
            .iter()
            .map(|p| to_snake(p))
            .collect::<Vec<_>>()
            .join("::")
    }
}

fn collect<'a>(
    desc: &'a DescriptorProto,
    package: &'a str,
    parent: &[&'a str],
    out: &mut BTreeMap<String, Message<'a>>,
) {
    let mut path = parent.to_vec();
    path.push(desc.name());
    for nested in &desc.nested_type {
        collect(nested, package, &path, out);
    }
    let full_name = format!(".{package}.{}", path.join("."));
    out.insert(
        full_name,
        Message {
            desc,
            package,
            path,
        },
    );
}

fn is_map_entry(desc: &DescriptorProto) -> bool {
    desc.options.as_ref().is_some_and(|o| o.map_entry())
}

/// Rust primitive behind a `google.protobuf` wrapper, which prost maps to
/// `Option<primitive>`. A set wrapper encodes as its `value`.
fn wrapper_kind(type_name: &str) -> Option<Type> {
    Some(match type_name {
        ".google.protobuf.DoubleValue" => Type::Double,
        ".google.protobuf.FloatValue" => Type::Float,
        ".google.protobuf.Int64Value" => Type::Int64,
        ".google.protobuf.UInt64Value" => Type::Uint64,
        ".google.protobuf.Int32Value" => Type::Int32,
        ".google.protobuf.UInt32Value" => Type::Uint32,
        ".google.protobuf.BoolValue" => Type::Bool,
        ".google.protobuf.StringValue" => Type::String,
        ".google.protobuf.BytesValue" => Type::Bytes,
        _ => return None,
    })
}

/// Code that writes the scalar `v` (a reference to the field's Rust type).
fn put_scalar(kind: Type, v: &str) -> Result<String, BoxError> {
    Ok(match kind {
        Type::Double | Type::Float => format!("s.put(&{v}.to_bits().to_be_bytes());"),
        Type::Int32
        | Type::Sint32
        | Type::Sfixed32
        | Type::Enum
        | Type::Int64
        | Type::Sint64
        | Type::Sfixed64
        | Type::Uint32
        | Type::Fixed32
        | Type::Uint64
        | Type::Fixed64 => format!("s.put(&{v}.to_be_bytes());"),
        Type::Bool => format!("s.put(&[u8::from(*{v})]);"),
        Type::String => format!("s.put({v}.as_bytes());"),
        Type::Bytes => format!("s.put({v}.as_slice());"),
        Type::Message | Type::Group => {
            return Err(format!("put_scalar: {kind:?} is not a scalar").into());
        }
    })
}

/// Code that writes a single (non-repeated) value `v`, a reference.
fn put_value(field: &FieldDescriptorProto, v: &str) -> Result<String, BoxError> {
    match field.r#type() {
        Type::Message => match wrapper_kind(field.type_name()) {
            Some(kind) => put_scalar(kind, v),
            None => Ok(format!(
                "crate::stable::StableEncode::stable_encode_into({v}, s);"
            )),
        },
        Type::Group => Err(format!("field {}: groups are unsupported", field.name()).into()),
        kind => put_scalar(kind, v),
    }
}

struct Generator<'a> {
    messages: BTreeMap<String, Message<'a>>,
    /// Messages without a stable encoding: they contain a map, maybe
    /// transitively.
    excluded: BTreeSet<String>,
}

impl<'a> Generator<'a> {
    fn new(fds: &'a FileDescriptorSet) -> Self {
        let mut messages = BTreeMap::new();
        for file in &fds.file {
            for desc in &file.message_type {
                collect(desc, file.package(), &[], &mut messages);
            }
        }

        let mut excluded: BTreeSet<String> = BTreeSet::new();
        loop {
            let before = excluded.len();
            for (name, msg) in &messages {
                if is_map_entry(msg.desc) || excluded.contains(name) {
                    continue;
                }
                let bad = msg.desc.field.iter().any(|f| {
                    f.r#type() == Type::Message
                        && (excluded.contains(f.type_name())
                            || messages
                                .get(f.type_name())
                                .is_some_and(|m| is_map_entry(m.desc)))
                });
                if bad {
                    excluded.insert(name.clone());
                }
            }
            if excluded.len() == before {
                break;
            }
        }
        Generator { messages, excluded }
    }

    /// Body of `stable_encode_into`.
    fn encode_body(&self, msg: &Message<'_>) -> Result<String, BoxError> {
        let desc = msg.desc;
        let mut out = String::new();
        let mut emitted_oneofs = BTreeSet::new();
        for field in &desc.field {
            let synthetic = field.proto3_optional();
            if let (Some(index), false) = (field.oneof_index, synthetic) {
                if !emitted_oneofs.insert(index) {
                    continue;
                }
                let oneof = usize::try_from(index)
                    .ok()
                    .and_then(|i| desc.oneof_decl.get(i))
                    .ok_or_else(|| format!("{}: bad oneof index", desc.name()))?;
                let enum_path = format!("{}::{}", msg.module_path(), to_upper_camel(oneof.name()));
                writeln!(
                    out,
                    "        if let Some(o) = &self.{} {{",
                    to_snake(oneof.name())
                )?;
                writeln!(out, "            match o {{")?;
                for member in desc.field.iter().filter(|f| f.oneof_index == Some(index)) {
                    writeln!(
                        out,
                        "                {enum_path}::{}(v) => {{ {} }}",
                        to_upper_camel(member.name()),
                        put_value(member, "v")?
                    )?;
                }
                writeln!(out, "            }}")?;
                writeln!(out, "        }}")?;
                continue;
            }

            let name = to_snake(field.name());
            let repeated = field.label() == Label::Repeated;
            let is_message = field.r#type() == Type::Message;
            if repeated {
                writeln!(
                    out,
                    "        for v in &self.{name} {{ {} }}",
                    put_value(field, "v")?
                )?;
            } else if is_message || synthetic {
                writeln!(
                    out,
                    "        if let Some(v) = &self.{name} {{ {} }}",
                    put_value(field, "v")?
                )?;
            } else {
                writeln!(
                    out,
                    "        {}",
                    put_value(field, &format!("(&self.{name})"))?
                )?;
            }
        }
        Ok(out)
    }

    fn generate_package(&self, package: &str) -> Result<String, BoxError> {
        let mut out =
            String::from("// This file is @generated by strims-proto/build/stable.rs.\n\n");
        for (name, msg) in &self.messages {
            if msg.package != package || is_map_entry(msg.desc) || self.excluded.contains(name) {
                continue;
            }
            let path = msg.rust_path();
            let body = self.encode_body(msg)?;
            let sink = if body.is_empty() { "_s" } else { "s" };
            writeln!(out, "impl crate::stable::StableEncode for {path} {{")?;
            writeln!(
                out,
                "    fn stable_encode_into<S: crate::stable::StableSink + ?Sized>(&self, {sink}: &mut S) {{"
            )?;
            out.push_str(&body);
            writeln!(out, "    }}")?;
            writeln!(out, "}}\n")?;
        }
        Ok(out)
    }
}

pub fn generate(fds: &FileDescriptorSet, out_dir: &Path) -> Result<(), BoxError> {
    let generator = Generator::new(fds);
    let packages: BTreeSet<&str> = fds.file.iter().map(|f| f.package()).collect();
    for package in packages {
        let code = generator.generate_package(package)?;
        fs::write(out_dir.join(format!("{package}.stable.rs")), code)?;
    }

    // A record of what was skipped, for reviewers.
    let mut skipped = String::new();
    for name in &generator.excluded {
        writeln!(skipped, "{name}")?;
    }
    fs::write(out_dir.join("stable_skipped.txt"), skipped)?;

    // The golden vectors require protobuf re-encoding to reproduce the
    // received bytes. prost writes a oneof at its smallest member tag, which
    // keeps field order only while no other field's tag lies between a
    // oneof's member tags; warn if a schema change breaks that.
    for (name, msg) in &generator.messages {
        if oneof_interleaves(msg.desc) {
            println!(
                "cargo:warning={name}: oneof tags interleave with other fields; prost re-encodes them out of field-number order"
            );
        }
    }
    Ok(())
}

/// Whether some other field's tag lies strictly between a oneof's smallest
/// and largest member tags.
fn oneof_interleaves(desc: &DescriptorProto) -> bool {
    let real_oneof = |f: &FieldDescriptorProto| f.oneof_index.filter(|_| !f.proto3_optional());
    let mut ranges: BTreeMap<i32, (i32, i32)> = BTreeMap::new();
    for f in &desc.field {
        if let Some(i) = real_oneof(f) {
            let r = ranges.entry(i).or_insert((f.number(), f.number()));
            r.0 = r.0.min(f.number());
            r.1 = r.1.max(f.number());
        }
    }
    ranges.iter().any(|(&i, &(lo, hi))| {
        desc.field
            .iter()
            .any(|f| real_oneof(f) != Some(i) && f.number() > lo && f.number() < hi)
    })
}
