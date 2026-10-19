// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The stable encoding: a simple binary format for protobuf messages with
//! ordered fields.
//!
//! CRC32 checksums are computed over this encoding, so its byte layout is
//! part of the protocol. Each message writes its fields in `.proto`
//! declaration order, which can differ from field-number order; a oneof
//! sits where its first member is declared.
//!
//! - bool: one byte; 32/64-bit ints, enums and floats: big-endian bits;
//! - string and bytes: the raw bytes, with no length prefix;
//! - repeated: the elements back to back;
//! - message, wrapper or oneof: the contents when set, nothing when unset.
//!
//! Nothing marks field boundaries: bytes can move between adjacent string
//! fields without changing the encoding.
//!
//! The build script generates the impls from the schema descriptors
//! (`build/stable.rs`). The checksum function lives in `strims-crypto`.

/// Receives the encoded bytes. Implemented for `Vec<u8>` and
/// [`LenCounter`]; a hasher can implement it to checksum without buffering.
pub trait StableSink {
    fn put(&mut self, bytes: &[u8]);
}

impl StableSink for Vec<u8> {
    fn put(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes);
    }
}

/// A sink that only counts bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LenCounter(pub usize);

impl StableSink for LenCounter {
    fn put(&mut self, bytes: &[u8]) {
        self.0 = self.0.saturating_add(bytes.len());
    }
}

/// The stable encoding. Implemented for every generated message
/// except those that contain a map field, directly or through a nested
/// message: maps have no defined order, so they have no stable encoding.
/// The build lists the skipped messages in `$OUT_DIR/stable_skipped.txt`.
pub trait StableEncode {
    /// Writes the encoding of `self` into `s`.
    fn stable_encode_into<S: StableSink + ?Sized>(&self, s: &mut S);

    /// Length of the encoding.
    fn stable_encoded_len(&self) -> usize {
        let mut n = LenCounter::default();
        self.stable_encode_into(&mut n);
        n.0
    }

    /// The encoding as a new vector.
    fn stable_encode_to_vec(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(self.stable_encoded_len());
        self.stable_encode_into(&mut b);
        b
    }
}

/// prost boxes recursive fields (e.g. `Certificate.parent`).
impl<T: StableEncode + ?Sized> StableEncode for Box<T> {
    fn stable_encode_into<S: StableSink + ?Sized>(&self, s: &mut S) {
        (**self).stable_encode_into(s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vpn::v1::hash_table_message;

    #[test]
    fn nested_messages_encode_only_when_set() {
        let mut r = crate::vpn::v1::HashTableMessage::default();
        assert!(r.stable_encode_to_vec().is_empty());
        r.body = Some(hash_table_message::Body::GetResponse(
            hash_table_message::GetResponse::default(),
        ));
        // an empty response still has its 8-byte request id
        assert_eq!(r.stable_encode_to_vec(), [0u8; 8]);
    }

    #[test]
    fn module_paths_and_type_names() {
        use prost::Name;
        assert_eq!(
            crate::vnic::v1::PeerInit::full_name(),
            "strims.vnic.v1.PeerInit"
        );
        assert_eq!(
            crate::network::v1::bootstrap::BootstrapClient::full_name(),
            "strims.network.v1.bootstrap.BootstrapClient"
        );
        assert_eq!(
            crate::r#type::Key::type_url(),
            "type.googleapis.com/strims.type.Key"
        );
        assert_eq!(
            crate::devtools::v1::ppspp::CapConnLog::full_name(),
            "strims.devtools.v1.ppspp.CapConnLog"
        );
    }
}
