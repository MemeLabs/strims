// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Signed objects that travel as bytes.
//!
//! A signed object is an envelope: its body's encoding in `bytes body` and
//! the signature of those bytes in `bytes signature`, made in the body
//! type's [`Domain`]. Verification covers the bytes as received, so fields
//! of the body this node doesn't know stay covered. Whoever holds or
//! forwards a signed object keeps its envelope; nothing re-encodes a body it
//! verified.
//!
//! [`Verified`] pairs an envelope with its body, decoded once. Each signed
//! body type implements [`SignedBody`] here, which makes this module the
//! registry of signed objects and their domains.

use core::ops::Deref;

use prost::Message;
use strims_proto::network::v1::directory::{ListingSnippetBody, SignedListingSnippet};
use strims_proto::types::{
    Certificate, CertificateBody, CertificateRequest, CertificateRequestBody,
};
use strims_proto::vpn::v1::{hash_table_message, peer_index_message};

use crate::Error;
use crate::sign::{self, Domain, Signer};

/// A signed object's wire form: a body's encoding and its signature.
pub trait Envelope: Message + Default + Clone {
    fn new(body: Vec<u8>, signature: Vec<u8>) -> Self;
    fn body(&self) -> &[u8];
    fn signature(&self) -> &[u8];
}

/// The body of a signed object.
pub trait SignedBody: Message + Default {
    /// The domain its signatures are made in.
    const DOMAIN: Domain;
    /// The envelope that carries it.
    type Envelope: Envelope;
}

/// A body that names the public key that must sign it.
pub trait SelfKeyed: SignedBody {
    fn signer_key(&self) -> &[u8];
}

/// An envelope whose signature verified, with its decoded body.
#[derive(Clone, Debug, PartialEq)]
pub struct Verified<B: SignedBody> {
    body: B,
    envelope: B::Envelope,
}

impl<B: SignedBody> Verified<B> {
    /// Encodes and signs `body`.
    pub fn sign(signer: &Signer, body: B) -> Verified<B> {
        let bytes = body.encode_to_vec();
        let signature = signer.sign(B::DOMAIN, &[&bytes]);
        Verified {
            envelope: B::Envelope::new(bytes, signature.to_vec()),
            body,
        }
    }

    /// Checks that `public` signed the envelope's body, then decodes it.
    pub fn verify(envelope: B::Envelope, public: &[u8]) -> Result<Verified<B>, Error> {
        if !sign::verify(B::DOMAIN, public, &[envelope.body()], envelope.signature()) {
            return Err(Error::InvalidMessageSignature);
        }
        let body = B::decode(envelope.body()).map_err(|_| Error::MalformedBody)?;
        Ok(Verified { body, envelope })
    }

    pub fn body(&self) -> &B {
        &self.body
    }

    /// The envelope, as signed or received.
    pub fn envelope(&self) -> &B::Envelope {
        &self.envelope
    }

    pub fn into_envelope(self) -> B::Envelope {
        self.envelope
    }
}

impl<B: SelfKeyed> Verified<B> {
    /// Decodes the envelope's body, then checks that the key it names
    /// signed it.
    pub fn open(envelope: B::Envelope) -> Result<Verified<B>, Error> {
        let body = B::decode(envelope.body()).map_err(|_| Error::MalformedBody)?;
        if !sign::verify(
            B::DOMAIN,
            body.signer_key(),
            &[envelope.body()],
            envelope.signature(),
        ) {
            return Err(Error::InvalidMessageSignature);
        }
        Ok(Verified { body, envelope })
    }
}

impl<B: SignedBody> Deref for Verified<B> {
    type Target = B;

    fn deref(&self) -> &B {
        &self.body
    }
}

macro_rules! envelope {
    ($envelope:ty) => {
        impl Envelope for $envelope {
            fn new(body: Vec<u8>, signature: Vec<u8>) -> Self {
                Self { body, signature }
            }

            fn body(&self) -> &[u8] {
                &self.body
            }

            fn signature(&self) -> &[u8] {
                &self.signature
            }
        }
    };
}

envelope!(CertificateRequest);
envelope!(SignedListingSnippet);
envelope!(hash_table_message::Record);
envelope!(peer_index_message::Record);

impl Envelope for Certificate {
    /// A certificate without a parent.
    fn new(body: Vec<u8>, signature: Vec<u8>) -> Self {
        Certificate {
            body,
            signature,
            parent: None,
        }
    }

    fn body(&self) -> &[u8] {
        &self.body
    }

    fn signature(&self) -> &[u8] {
        &self.signature
    }
}

/// A certificate, signed by its issuer (see [`crate::cert`]).
impl SignedBody for CertificateBody {
    const DOMAIN: Domain = Domain::Certificate;
    type Envelope = Certificate;
}

/// A certificate request, signed by the key it asks a certificate for.
impl SignedBody for CertificateRequestBody {
    const DOMAIN: Domain = Domain::CertificateRequest;
    type Envelope = CertificateRequest;
}

impl SelfKeyed for CertificateRequestBody {
    fn signer_key(&self) -> &[u8] {
        &self.key
    }
}

/// A directory listing snippet, signed by its listing's media swarm key: the
/// key whose public half is the swarm id.
impl SignedBody for ListingSnippetBody {
    const DOMAIN: Domain = Domain::DirectoryListingSnippet;
    type Envelope = SignedListingSnippet;
}

/// A vpn hash table record, signed by the key it is published under.
impl SignedBody for hash_table_message::RecordBody {
    const DOMAIN: Domain = Domain::VpnHashTableRecord;
    type Envelope = hash_table_message::Record;
}

impl SelfKeyed for hash_table_message::RecordBody {
    fn signer_key(&self) -> &[u8] {
        &self.key
    }
}

/// A vpn peer index record, signed by the host it names.
impl SignedBody for peer_index_message::RecordBody {
    const DOMAIN: Domain = Domain::VpnPeerIndexRecord;
    type Envelope = peer_index_message::Record;
}

impl SelfKeyed for peer_index_message::RecordBody {
    fn signer_key(&self) -> &[u8] {
        &self.host_id
    }
}
