// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Generated protobuf types for `schema/**/*.proto`.
//!
//! The build script compiles the schema with protox and prost-build; see
//! `build/main.rs`.
//!
//! Modules mirror the proto packages without the common `strims.` prefix:
//! `strims.vnic.v1` is [`vnic::v1`], `strims.network.v1.bootstrap` is
//! [`network::v1::bootstrap`] and `strims.type` is `r#type` (also
//! re-exported as [`types`], since `type` is a keyword).
//!
//! prost conventions:
//! - enum fields are `i32`; use e.g. `KeyType::Ed25519 as i32`;
//! - `bytes` fields are `Vec<u8>`; the hot wire paths (ppspp chunks, vnic
//!   frames, vpn bodies) don't go through protobuf;
//! - `map` fields are `BTreeMap`, so iteration and encoding order are
//!   deterministic;
//! - `google.protobuf.*Value` wrappers are `Option<primitive>`, and
//!   `Some(false)` or `Some("")` encodes as a set wrapper;
//! - a oneof is an `Option<message::OneofName>` enum field;
//! - recursive fields are boxed (`Certificate.parent`);
//! - type URLs are `type.googleapis.com/<full name>` (`prost::Name`).
//!
//! Decoding drops unknown fields, so re-encoding a message from a peer with
//! a newer schema loses the fields this build doesn't know.
//!
//! The stable encoding that signatures cover is in [`stable`].

pub mod stable;

include!(concat!(env!("OUT_DIR"), "/tree.rs"));

pub use r#type as types;
