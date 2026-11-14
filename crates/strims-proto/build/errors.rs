// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The error-code ranges.
//!
//! Each owner of errors that reach the RPC wire codes them in an
//! `ErrorCode` enum of a `<package>.errors` package, within a range no
//! other owner's touch, so a code names one error across the API: a
//! service relaying another's error keeps its code, and the UI names any
//! code without knowing which service sent it. `0` is every enum's
//! `UNDEFINED`, no code.
//!
//! [`check`] fails the build when an `ErrorCode` enum is in no listed
//! package, a value falls outside its package's range, a listed package has
//! no enum, or two ranges overlap.

use std::ops::RangeInclusive;

use prost_types::FileDescriptorSet;

/// The packages whose `ErrorCode` enums code RPC errors, and their ranges.
const RANGES: &[(&str, RangeInclusive<i32>)] = &[
    ("strims.network.v1.errors", 1..=999),
    ("strims.rpc.errors", 1000..=1999),
    ("strims.dao.v1.errors", 2000..=2999),
    ("strims.node.errors", 3000..=3999),
    ("strims.network.v1.directory.errors", 4000..=4999),
];

/// The enum every error-code package declares.
const ENUM: &str = "ErrorCode";

pub(crate) fn check(fds: &FileDescriptorSet) -> Result<(), String> {
    let mut errors = Vec::new();
    for (i, (a, ra)) in RANGES.iter().enumerate() {
        if *ra.start() < 1 {
            errors.push(format!("{a}: range {ra:?} holds 0, which is no code"));
        }
        for (b, rb) in RANGES.iter().skip(i + 1) {
            if ra.start() <= rb.end() && rb.start() <= ra.end() {
                errors.push(format!("{a} {ra:?} and {b} {rb:?} overlap"));
            }
        }
    }
    let mut found = Vec::new();
    for file in &fds.file {
        let package = file.package();
        let nested = file
            .message_type
            .iter()
            .flat_map(|m| &m.enum_type)
            .any(|e| e.name() == ENUM);
        if nested {
            errors.push(format!(
                "{}: {ENUM} is nested in a message; declare it in a .errors package",
                file.name()
            ));
        }
        for e in file.enum_type.iter().filter(|e| e.name() == ENUM) {
            let Some((_, range)) = RANGES.iter().find(|(p, _)| *p == package) else {
                errors.push(format!("{package}.{ENUM} has no range in build/errors.rs"));
                continue;
            };
            found.push(package);
            for v in &e.value {
                let n = v.number();
                if n == 0 {
                    if v.name() != "UNDEFINED" {
                        errors.push(format!("{package}.{ENUM}: 0 must be UNDEFINED"));
                    }
                } else if !range.contains(&n) {
                    errors.push(format!(
                        "{package}.{ENUM}.{} = {n} is outside {range:?}",
                        v.name()
                    ));
                }
            }
        }
    }
    for (p, _) in RANGES {
        if !found.contains(p) {
            errors.push(format!("{p} has a range but no {ENUM}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}
