// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! No integrity protection: the `None` variants of
//! [`SignatureSigner`](super::SignatureSigner),
//! [`SignatureVerifier`](super::SignatureVerifier),
//! [`SwarmVerifier`](super::SwarmVerifier) and
//! [`ChannelVerifier`](super::ChannelVerifier): signing yields an empty
//! signature, verification always succeeds and nothing is stored.

/// The signature length without live signatures.
pub const NONE_SIGNATURE_SIZE: usize = 0;
