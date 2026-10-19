// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Generated protobuf types for `schema/**/*.proto`, plus the `strims.rpc`
//! envelope ([`rpc`]) and `strims.rpc.fixture` ([`rpc::fixture`]), the
//! service the RPC tests serve, from the crate's own `proto/`.
//!
//! The build script compiles both with protox and prost-build; see
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
//!   Stored and transmitted `Any` values carry this domain, so it must not
//!   change.
//!
//! Decoding drops unknown fields, so re-encoding a message from a peer with
//! a newer schema loses the fields this build doesn't know.
//!
//! [`rpc`] is the RPC envelope: every request, response and cancel is one
//! [`rpc::Call`] carrying an encoded argument. With the `services` feature,
//! every package also has interfaces for its RPC services; see `service`.

#[cfg(feature = "services")]
pub mod service;

include!(concat!(env!("OUT_DIR"), "/tree.rs"));

pub use r#type as types;

#[cfg(test)]
mod tests {
    use prost::Name;

    #[test]
    fn type_urls_use_the_googleapis_domain() {
        assert_eq!(
            crate::rpc::Call::type_url(),
            "type.googleapis.com/strims.rpc.Call"
        );
        assert_eq!(
            crate::types::Certificate::type_url(),
            "type.googleapis.com/strims.type.Certificate"
        );
    }

    #[test]
    fn rpc_kinds_keep_their_wire_values() {
        use crate::rpc::call::Kind;
        assert_eq!(Kind::CallKindDefault as i32, 0);
        assert_eq!(Kind::CallKindError as i32, 1);
        assert_eq!(Kind::CallKindCancel as i32, 2);
        assert_eq!(Kind::CallKindClose as i32, 3);
        assert_eq!(Kind::CallKindUndefined as i32, 4);
    }
}
