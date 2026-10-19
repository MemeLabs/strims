// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Certificate requests, issuing and chain verification for
//! `strims.type.{Certificate, CertificateRequest}`.
//!
//! Both are signed envelopes (see [`crate::signed`]). A request is a
//! `CertificateRequestBody` signed by its own key in
//! [`Domain::CertificateRequest`]. A certificate is a `CertificateBody`
//! signed in [`Domain::Certificate`] by its issuer's key, with the issuer's
//! certificate as `parent`. The body names its issuer by serial number, so
//! a signature fixes the chain above it; an empty issuer serial number
//! marks a self-signed certificate. A certificate stored apart from its
//! chain keeps its issuer serial number and has no parent.
//!
//! Key usages are bit flags OR'ed into the `key_usage` enum field, e.g.
//! `KeyUsage::Peer as i32 | KeyUsage::Sign as i32`.
//!
//! Validity times are whole Unix seconds. Conversions saturate: issuing
//! clamps times before the epoch to 0, and verification reads values above
//! `i64::MAX` as `i64::MAX`.

use core::fmt;
use core::ops::Deref;

use prost::Message;
use sha2::{Digest, Sha256};
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_proto::types::{
    Certificate, CertificateBody, CertificateRequest, CertificateRequestBody, Key, KeyType,
    KeyUsage,
};

use crate::sign::{self, Domain, Signer};
use crate::signed::Verified;
use crate::{ED25519_PUBLIC_KEY_SIZE, Error};

/// How far before `now` a new certificate's validity starts.
pub const CERT_PREDATE_DURATION: Duration = Duration::HOUR;

/// About two years.
pub const DEFAULT_CERT_TTL: Duration = Duration::from_secs(2 * 365 * 24 * 60 * 60);

/// Size of a certificate serial number.
pub const SERIAL_NUMBER_SIZE: usize = 16;

/// The most certificates a chain may hold, leaf and root included.
pub const MAX_CHAIN_LENGTH: usize = 16;

/// A request for `key`, signed by it. An empty `subject` leaves it unset.
pub fn new_certificate_request(
    key: &Key,
    usage: i32,
    subject: &str,
) -> Result<Verified<CertificateRequestBody>, Error> {
    let signer = Signer::new(key)?;
    Ok(Verified::sign(
        &signer,
        CertificateRequestBody {
            key: key.public.clone(),
            key_type: key.r#type,
            key_usage: usage,
            subject: subject.to_owned(),
        },
    ))
}

/// Checks that the request asks for no usage outside `usage`, names an
/// ed25519 key, and is signed by that key.
pub fn verify_certificate_request(
    csr: &CertificateRequest,
    usage: i32,
) -> Result<Verified<CertificateRequestBody>, Error> {
    let body =
        CertificateRequestBody::decode(csr.body.as_slice()).map_err(|_| Error::MalformedBody)?;
    if body.key_usage & !usage != 0 {
        return Err(Error::UnsupportedKeyUsage);
    }
    if body.key_type != KeyType::Ed25519 as i32 {
        return Err(Error::UnsupportedKeyType);
    }
    if body.key.len() != ED25519_PUBLIC_KEY_SIZE {
        return Err(Error::InvalidKeyLength);
    }
    Verified::open(csr.clone()).map_err(|_| Error::InvalidCertificateRequestSignature)
}

/// Who signs a new certificate.
#[derive(Clone, Copy, Debug)]
pub enum Issuer<'a> {
    /// The certificate's own key: a root.
    SelfSigned(&'a Key),
    /// `key`, whose certificate is `certificate`. It must allow signing,
    /// and becomes the new certificate's parent.
    Certificate {
        key: &'a Key,
        certificate: &'a Certificate,
    },
}

/// Issues a certificate for `csr`, valid from an hour before `now` until
/// `now + valid_duration`, with a random 16-byte serial number.
pub fn sign_certificate_request(
    csr: &CertificateRequestBody,
    valid_duration: Duration,
    issuer: Issuer<'_>,
    now: Time,
    rng: &mut DetRng,
) -> Result<Certificate, Error> {
    let mut serial_number = vec![0u8; SERIAL_NUMBER_SIZE];
    rand_core::Rng::fill_bytes(rng, &mut serial_number);
    issue(
        CertificateBody {
            key: csr.key.clone(),
            key_type: csr.key_type,
            key_usage: csr.key_usage,
            subject: csr.subject.clone(),
            not_before: unix_seconds(now - CERT_PREDATE_DURATION),
            not_after: unix_seconds(now + valid_duration),
            serial_number,
            issuer_serial_number: Vec::new(),
        },
        issuer,
    )
}

/// Signs `body`, naming the issuer's serial number in it, and attaches the
/// issuer's certificate.
fn issue(mut body: CertificateBody, issuer: Issuer<'_>) -> Result<Certificate, Error> {
    let (key, parent) = match issuer {
        Issuer::SelfSigned(key) => {
            if key.public != body.key {
                return Err(Error::IssuerMismatch);
            }
            (key, None)
        }
        Issuer::Certificate { key, certificate } => {
            let issuer = certificate_body(certificate)?;
            if issuer.key != key.public {
                return Err(Error::IssuerMismatch);
            }
            if issuer.key_usage & KeyUsage::Sign as i32 == 0 {
                return Err(Error::UnsupportedKeyUsage);
            }
            body.issuer_serial_number = issuer.serial_number;
            (key, Some(Box::new(certificate.clone())))
        }
    };
    let signer = Signer::new(key)?;
    let body = body.encode_to_vec();
    let signature = signer.sign(Domain::Certificate, &[&body]).to_vec();
    Ok(Certificate {
        body,
        signature,
        parent,
    })
}

/// The certificate a host presents on a network: `host_key` (an ed25519
/// public key) signed by the profile key, with usage `KEY_USAGE_SIGN`, the
/// validity period of `membership`, and `membership` as its issuer.
///
/// `membership` is the profile's certificate on the network, which must be
/// for `profile_key` and allow signing. The result is a function of its
/// inputs: the serial number is the first 16 bytes of SHA-256 over the
/// membership serial number and the host key.
pub fn host_certificate(
    host_key: &[u8],
    profile_key: &Key,
    membership: &Certificate,
) -> Result<Certificate, Error> {
    if host_key.len() != ED25519_PUBLIC_KEY_SIZE {
        return Err(Error::InvalidKeyLength);
    }
    let m = certificate_body(membership)?;
    let digest = Sha256::new()
        .chain_update(&m.serial_number)
        .chain_update(host_key)
        .finalize();
    issue(
        CertificateBody {
            key: host_key.to_vec(),
            key_type: KeyType::Ed25519 as i32,
            key_usage: KeyUsage::Sign as i32,
            subject: String::new(),
            not_before: m.not_before,
            not_after: m.not_after,
            serial_number: digest
                .get(..SERIAL_NUMBER_SIZE)
                .unwrap_or_default()
                .to_vec(),
            issuer_serial_number: Vec::new(),
        },
        Issuer::Certificate {
            key: profile_key,
            certificate: membership,
        },
    )
}

/// A self-signed certificate for `key`.
pub fn new_self_signed_certificate(
    key: &Key,
    usage: i32,
    valid_duration: Duration,
    subject: &str,
    now: Time,
    rng: &mut DetRng,
) -> Result<Certificate, Error> {
    let csr = CertificateRequestBody {
        key: key.public.clone(),
        key_type: key.r#type,
        key_usage: usage,
        subject: subject.to_owned(),
    };
    sign_certificate_request(&csr, valid_duration, Issuer::SelfSigned(key), now, rng)
}

/// A network's root certificate: self-signed by the network key for
/// signing, with the network's name as its subject.
pub fn network_certificate(
    key: &Key,
    name: &str,
    valid_duration: Duration,
    now: Time,
    rng: &mut DetRng,
) -> Result<Certificate, Error> {
    new_self_signed_certificate(key, KeyUsage::Sign as i32, valid_duration, name, now, rng)
}

/// Issues a membership certificate for `csr`, signed by the network `key`,
/// under a new root certificate for the network (see
/// [`network_certificate`]).
pub fn sign_certificate_request_with_network(
    csr: &CertificateRequestBody,
    key: &Key,
    name: &str,
    valid_duration: Duration,
    now: Time,
    rng: &mut DetRng,
) -> Result<Certificate, Error> {
    let root = network_certificate(key, name, valid_duration, now, rng)?;
    sign_certificate_request(
        csr,
        valid_duration,
        Issuer::Certificate {
            key,
            certificate: &root,
        },
        now,
        rng,
    )
}

/// Whole Unix seconds; times before the epoch clamp to zero.
fn unix_seconds(t: Time) -> u64 {
    u64::try_from(t.unix()).unwrap_or(0)
}

/// The time `secs` after the epoch, saturating at `i64::MAX` seconds.
fn from_unix_seconds(secs: u64) -> Time {
    Time::from_unix(i64::try_from(secs).unwrap_or(i64::MAX), 0)
}

/// The certificate's own body, decoded. Signatures aren't checked.
pub fn certificate_body(cert: &Certificate) -> Result<CertificateBody, Error> {
    CertificateBody::decode(cert.body.as_slice()).map_err(|_| Error::MalformedBody)
}

/// The certificate's parent, if it carries one.
pub fn certificate_parent(cert: &Certificate) -> Option<&Certificate> {
    cert.parent.as_deref()
}

/// The last certificate in the parent chain.
pub fn certificate_root(cert: &Certificate) -> &Certificate {
    let mut cert = cert;
    while let Some(parent) = certificate_parent(cert) {
        cert = parent;
    }
    cert
}

/// A certificate and its parents with their bodies decoded, leaf first. It
/// derefs to the leaf's body.
///
/// [`CertificateChain::decode`] checks only the chain's shape; a chain
/// from [`verify_certificate`] has had every signature checked too.
#[derive(Clone, Debug, PartialEq)]
pub struct CertificateChain {
    certificate: Certificate,
    /// Leaf first; never empty, at most [`MAX_CHAIN_LENGTH`].
    bodies: Vec<CertificateBody>,
}

impl CertificateChain {
    /// Decodes every body in the chain. Each parent must be the
    /// certificate its child's body names as issuer
    /// ([`Error::IssuerMismatch`]), and the chain may hold at most
    /// [`MAX_CHAIN_LENGTH`] certificates ([`Error::ChainTooLong`]).
    pub fn decode(cert: &Certificate) -> Result<CertificateChain, Error> {
        let mut bodies: Vec<CertificateBody> = Vec::new();
        let mut next = Some(cert);
        while let Some(c) = next {
            if bodies.len() == MAX_CHAIN_LENGTH {
                return Err(Error::ChainTooLong);
            }
            let body = certificate_body(c)?;
            if let Some(child) = bodies.last()
                && child.issuer_serial_number != body.serial_number
            {
                return Err(Error::IssuerMismatch);
            }
            bodies.push(body);
            next = certificate_parent(c);
        }
        Ok(CertificateChain {
            certificate: cert.clone(),
            bodies,
        })
    }

    /// The certificate as signed.
    pub fn certificate(&self) -> &Certificate {
        &self.certificate
    }

    pub fn into_certificate(self) -> Certificate {
        self.certificate
    }

    /// The bodies from the leaf to the root.
    pub fn bodies(&self) -> &[CertificateBody] {
        &self.bodies
    }

    /// The leaf's body.
    pub fn leaf(&self) -> &CertificateBody {
        // invariant: a chain holds at least its leaf.
        #[allow(clippy::indexing_slicing)]
        &self.bodies[0]
    }

    /// The last body in the chain.
    pub fn root(&self) -> &CertificateBody {
        self.bodies.last().unwrap_or_else(|| self.leaf())
    }

    /// The root's key, which identifies the network.
    pub fn network_key(&self) -> &[u8] {
        &self.root().key
    }

    /// The chain above the leaf, if it carries a parent.
    pub fn parent(&self) -> Option<CertificateChain> {
        let parent = certificate_parent(&self.certificate)?;
        Some(CertificateChain {
            certificate: parent.clone(),
            bodies: self.bodies.get(1..)?.to_vec(),
        })
    }

    /// The period in which every certificate in the chain is valid: the
    /// latest not-before and the earliest not-after, both inclusive, as
    /// [`verify_certificate`] reads them. Empty when the first follows the
    /// second.
    pub fn validity(&self) -> (Time, Time) {
        let not_before = self.bodies.iter().map(|b| b.not_before).max();
        let not_after = self.bodies.iter().map(|b| b.not_after).min();
        (
            from_unix_seconds(not_before.unwrap_or(0)),
            from_unix_seconds(not_after.unwrap_or(u64::MAX)),
        )
    }

    /// Whether a membership certificate is trusted at `now`: its network's
    /// root signed it directly and it hasn't expired. A certificate issued
    /// under an invitation, below another member, isn't. Signatures aren't
    /// checked here.
    pub fn membership_trusted(&self, now: Time) -> bool {
        self.parent_key() == Some(self.network_key())
            && !now.after(from_unix_seconds(self.leaf().not_after))
    }

    fn parent_key(&self) -> Option<&[u8]> {
        self.bodies.get(1).map(|b| b.key.as_slice())
    }

    /// Whether `now`, in whole seconds, is outside the leaf's validity
    /// period or on either boundary.
    pub fn is_expired(&self, now: Time) -> bool {
        let now = unix_seconds(now);
        self.leaf().not_after <= now || self.leaf().not_before >= now
    }
}

impl Deref for CertificateChain {
    type Target = CertificateBody;

    fn deref(&self) -> &CertificateBody {
        self.leaf()
    }
}

/// Whether `cert` is a membership trusted at `now`
/// ([`CertificateChain::membership_trusted`]); false if it doesn't decode.
pub fn membership_trusted(cert: &Certificate, now: Time) -> bool {
    CertificateChain::decode(cert).is_ok_and(|c| c.membership_trusted(now))
}

/// Verifies every certificate in the chain at `now`: each must be signed
/// by its parent, or by itself if its body names no issuer, the signer must
/// have `KEY_USAGE_SIGN`, and `now` must fall within its validity period,
/// inclusive. A body that names an issuer the chain doesn't carry fails
/// with [`Error::MissingIssuer`].
pub fn verify_certificate(
    cert: &Certificate,
    now: Time,
) -> Result<CertificateChain, CertificateErrors> {
    let chain = CertificateChain::decode(cert).map_err(CertificateErrors::from)?;
    let mut errs = CertificateErrors::default();
    let mut level = Some(cert);
    for (i, body) in chain.bodies.iter().enumerate() {
        let Some(c) = level else { break };
        let signer = match chain.bodies.get(i + 1) {
            Some(parent) => parent,
            None if body.issuer_serial_number.is_empty() => body,
            None => {
                errs.push(Error::MissingIssuer);
                body
            }
        };
        if signer.key_usage & KeyUsage::Sign as i32 == 0 {
            errs.push(Error::UnsupportedKeyUsage);
        }
        if signer.key_type != KeyType::Ed25519 as i32 {
            errs.push(Error::UnsupportedKeyType);
        } else if signer.key.len() != ED25519_PUBLIC_KEY_SIZE {
            errs.push(Error::InvalidKeyLength);
        } else if !sign::verify(Domain::Certificate, &signer.key, &[&c.body], &c.signature) {
            errs.push(Error::InvalidSignature);
        }
        if now.before(from_unix_seconds(body.not_before)) {
            errs.push(Error::NotBeforeRange);
        }
        if now.after(from_unix_seconds(body.not_after)) {
            errs.push(Error::NotAfterRange);
        }
        level = certificate_parent(c);
    }
    errs.into_result().map(|()| chain)
}

/// The failures [`verify_certificate`] found anywhere in a chain, in order
/// of first occurrence. Holds at most one entry per [`Error`] variant.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CertificateErrors(Vec<Error>);

impl CertificateErrors {
    fn push(&mut self, e: Error) {
        if !self.0.contains(&e) {
            self.0.push(e);
        }
    }

    fn into_result(self) -> Result<(), CertificateErrors> {
        if self.0.is_empty() { Ok(()) } else { Err(self) }
    }

    /// Whether `e` is among the failures.
    pub fn includes(&self, e: Error) -> bool {
        self.0.contains(&e)
    }

    pub fn errors(&self) -> &[Error] {
        &self.0
    }
}

impl From<Error> for CertificateErrors {
    fn from(e: Error) -> Self {
        CertificateErrors(vec![e])
    }
}

impl fmt::Display for CertificateErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{e}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CertificateErrors {}

#[cfg(test)]
mod tests {
    // Tests panic on failure by design.
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )]

    use super::*;
    use crate::generate_key;
    use strims_base::rng::det_rng;

    const SIGN: i32 = KeyUsage::Sign as i32;
    const PEER: i32 = KeyUsage::Peer as i32;
    const BOOTSTRAP: i32 = KeyUsage::Bootstrap as i32;

    fn now() -> Time {
        Time::from_unix(1_700_000_000, 0)
    }

    /// Re-signs `cert` after `f` changes its body, as its issuer `key`.
    fn resign(cert: &Certificate, key: &Key, f: impl FnOnce(&mut CertificateBody)) -> Certificate {
        let mut body = certificate_body(cert).unwrap();
        f(&mut body);
        let body = body.encode_to_vec();
        let signature = Signer::new(key)
            .unwrap()
            .sign(Domain::Certificate, &[&body])
            .to_vec();
        Certificate {
            body,
            signature,
            parent: cert.parent.clone(),
        }
    }

    #[test]
    fn requests_need_an_ed25519_key() {
        let k = generate_key(&mut det_rng(1));
        let csr = new_certificate_request(&k, SIGN, "s").unwrap();
        assert_eq!(csr.key, k.public);
        assert_eq!(csr.subject, "s");
        let short = Key {
            private: k.private[..59].to_vec(),
            ..k.clone()
        };
        assert_eq!(
            new_certificate_request(&short, SIGN, "").err(),
            Some(Error::InvalidKeyLength)
        );
        let x = Key {
            r#type: KeyType::X25519 as i32,
            ..k
        };
        assert_eq!(
            new_certificate_request(&x, SIGN, "").err(),
            Some(Error::UnsupportedKeyType)
        );
    }

    #[test]
    fn requests_verify_against_their_own_key() {
        let k = generate_key(&mut det_rng(1));
        let csr = new_certificate_request(&k, PEER | SIGN, "s").unwrap();
        let env = csr.envelope().clone();
        assert_eq!(verify_certificate_request(&env, PEER | SIGN).unwrap(), csr);
        assert_eq!(
            verify_certificate_request(&env, PEER).err(),
            Some(Error::UnsupportedKeyUsage)
        );

        let resigned = |f: &dyn Fn(&mut CertificateRequestBody)| {
            let mut body = csr.body().clone();
            f(&mut body);
            Verified::sign(&Signer::new(&k).unwrap(), body).into_envelope()
        };
        let short = resigned(&|b| b.key.truncate(31));
        assert_eq!(
            verify_certificate_request(&short, PEER | SIGN).err(),
            Some(Error::InvalidKeyLength)
        );
        let x = resigned(&|b| b.key_type = KeyType::X25519 as i32);
        assert_eq!(
            verify_certificate_request(&x, PEER | SIGN).err(),
            Some(Error::UnsupportedKeyType)
        );
        let other = generate_key(&mut det_rng(2));
        let forged = resigned(&|b| b.key = other.public.clone());
        assert_eq!(
            verify_certificate_request(&forged, PEER | SIGN).err(),
            Some(Error::InvalidCertificateRequestSignature)
        );
        let garbage = CertificateRequest {
            body: vec![0xff; 3],
            signature: env.signature.clone(),
        };
        assert_eq!(
            verify_certificate_request(&garbage, PEER | SIGN).err(),
            Some(Error::MalformedBody)
        );
    }

    /// A network root, a member signed by it, and a leaf signed by the
    /// member.
    fn chain() -> (Certificate, Key, Key, Key) {
        let mut rng = det_rng(3);
        let (network_key, member_key, leaf_key) = (
            generate_key(&mut rng),
            generate_key(&mut rng),
            generate_key(&mut rng),
        );
        let root = new_self_signed_certificate(
            &network_key,
            SIGN,
            DEFAULT_CERT_TTL,
            "network",
            now(),
            &mut rng,
        )
        .unwrap();
        let member_csr = new_certificate_request(&member_key, PEER | SIGN, "alias").unwrap();
        let member = sign_certificate_request(
            &member_csr,
            DEFAULT_CERT_TTL,
            Issuer::Certificate {
                key: &network_key,
                certificate: &root,
            },
            now(),
            &mut rng,
        )
        .unwrap();
        let leaf_csr = new_certificate_request(&leaf_key, PEER | BOOTSTRAP, "").unwrap();
        let leaf = sign_certificate_request(
            &leaf_csr,
            Duration::HOUR * 24,
            Issuer::Certificate {
                key: &member_key,
                certificate: &member,
            },
            now(),
            &mut rng,
        )
        .unwrap();
        (leaf, network_key, member_key, leaf_key)
    }

    #[test]
    fn chain_verifies() {
        let (leaf, network_key, member_key, leaf_key) = chain();
        let c = verify_certificate(&leaf, now()).unwrap();
        assert_eq!(c.network_key(), network_key.public.as_slice());
        assert_eq!(c.root().subject, "network");
        assert_eq!(c.key, leaf_key.public);
        assert_eq!(c.not_before, 1_700_000_000 - 3600);
        assert_eq!(c.not_after, 1_700_000_000 + 86_400);
        assert_eq!(c.serial_number.len(), SERIAL_NUMBER_SIZE);
        let keys: Vec<&[u8]> = c.bodies().iter().map(|b| b.key.as_slice()).collect();
        assert_eq!(
            keys,
            [
                leaf_key.public.as_slice(),
                member_key.public.as_slice(),
                network_key.public.as_slice()
            ]
        );
        assert_eq!(c.issuer_serial_number, c.bodies()[1].serial_number);
        assert!(c.root().issuer_serial_number.is_empty());
        assert_eq!(c.parent().unwrap().key, member_key.public);
        assert_eq!(c.certificate(), &leaf);
    }

    #[test]
    fn issuers_must_hold_the_key_and_sign() {
        let (leaf, network_key, member_key, leaf_key) = chain();
        let member = certificate_parent(&leaf).unwrap();
        let mut rng = det_rng(9);
        let csr = new_certificate_request(&leaf_key, PEER, "").unwrap();
        let issue = |key: &Key, certificate: &Certificate, rng: &mut DetRng| {
            sign_certificate_request(
                &csr,
                DEFAULT_CERT_TTL,
                Issuer::Certificate { key, certificate },
                now(),
                rng,
            )
        };
        assert_eq!(
            issue(&network_key, member, &mut rng).err(),
            Some(Error::IssuerMismatch)
        );
        assert_eq!(
            issue(&leaf_key, &leaf, &mut rng).err(),
            Some(Error::UnsupportedKeyUsage)
        );
        assert!(issue(&member_key, member, &mut rng).is_ok());
        assert_eq!(
            sign_certificate_request(
                &csr,
                DEFAULT_CERT_TTL,
                Issuer::SelfSigned(&member_key),
                now(),
                &mut rng
            )
            .err(),
            Some(Error::IssuerMismatch)
        );
    }

    #[test]
    fn only_root_signed_unexpired_memberships_are_trusted() {
        let (leaf, ..) = chain();
        let leaf = CertificateChain::decode(&leaf).unwrap();
        let member = leaf.parent().unwrap();
        assert!(member.membership_trusted(now()));
        assert!(!leaf.membership_trusted(now()));
        assert!(!member.parent().unwrap().membership_trusted(now()));

        let not_after = Time::from_unix(i64::try_from(member.not_after).unwrap(), 0);
        assert!(member.membership_trusted(not_after));
        assert!(!member.membership_trusted(not_after + Duration::SECOND));
    }

    #[test]
    fn network_signed_memberships_carry_a_fresh_root() {
        let mut rng = det_rng(4);
        let (network_key, member_key) = (generate_key(&mut rng), generate_key(&mut rng));
        let csr = new_certificate_request(&member_key, PEER | SIGN, "alias").unwrap();
        let sign = |rng: &mut DetRng| {
            sign_certificate_request_with_network(
                &csr,
                &network_key,
                "net",
                DEFAULT_CERT_TTL,
                now(),
                rng,
            )
            .unwrap()
        };
        let (a, b) = (sign(&mut rng), sign(&mut rng));
        let a = verify_certificate(&a, now()).unwrap();
        let b = CertificateChain::decode(&b).unwrap();
        assert!(a.membership_trusted(now()));
        assert_eq!(a.subject, "alias");
        let (ra, rb) = (a.root(), b.root());
        assert_eq!(ra.subject, "net");
        assert_eq!(ra.key_usage, SIGN);
        assert_eq!(ra.key, rb.key);
        assert_ne!(ra.serial_number, rb.serial_number);
    }

    #[test]
    fn chain_errors() {
        let (leaf, _, member_key, _) = chain();

        // the leaf's validity window
        let errs = verify_certificate(&leaf, now() + Duration::HOUR * 25).unwrap_err();
        assert_eq!(errs.errors(), &[Error::NotAfterRange]);
        let errs = verify_certificate(&leaf, now() - Duration::HOUR * 2).unwrap_err();
        assert_eq!(errs.errors(), &[Error::NotBeforeRange]);
        // boundaries are inclusive, at nanosecond precision
        verify_certificate(&leaf, Time::from_unix(1_700_000_000 + 86_400, 0)).unwrap();
        assert!(verify_certificate(&leaf, Time::from_unix(1_700_000_000 + 86_400, 1)).is_err());

        // tampering anywhere in the chain
        let mut t = leaf.clone();
        let mut body = certificate_body(&t).unwrap();
        body.subject = "mallory".into();
        t.body = body.encode_to_vec();
        assert_eq!(
            verify_certificate(&t, now()).unwrap_err().errors(),
            &[Error::InvalidSignature]
        );
        let mut t = leaf.clone();
        let member = t.parent.as_mut().unwrap();
        let mut body = certificate_body(member).unwrap();
        body.key_usage = PEER;
        member.body = body.encode_to_vec();
        let errs = verify_certificate(&t, now()).unwrap_err();
        assert!(
            errs.includes(Error::UnsupportedKeyUsage) && errs.includes(Error::InvalidSignature)
        );
        assert_eq!(
            errs.to_string(),
            "unsupported key usage, invalid certificate signature"
        );

        // a certificate without the issuer its body names
        let mut t = leaf.clone();
        t.parent = None;
        let errs = verify_certificate(&t, now()).unwrap_err();
        assert!(errs.includes(Error::MissingIssuer));
        assert!(errs.includes(Error::InvalidSignature));
        // the signature fixes the issuer: another certificate for the
        // member's key can't stand in for its parent
        let member = certificate_parent(&leaf).unwrap();
        let mut t = leaf.clone();
        t.parent = Some(Box::new(resign(member, &member_key, |b| {
            b.serial_number = vec![1; 16];
        })));
        assert_eq!(
            verify_certificate(&t, now()).unwrap_err().errors(),
            &[Error::IssuerMismatch]
        );
        assert_eq!(
            CertificateChain::decode(&t).err(),
            Some(Error::IssuerMismatch)
        );
    }

    #[test]
    fn requests_and_certificates_sign_in_separate_domains() {
        let k = generate_key(&mut det_rng(7));
        let csr = new_certificate_request(&k, SIGN, "s").unwrap();
        // The request's signed bytes, presented as a self-signed
        // certificate whose body decodes from them.
        let env = csr.envelope();
        let forged = Certificate {
            body: env.body.clone(),
            signature: env.signature.clone(),
            parent: None,
        };
        let errs = verify_certificate(&forged, now()).unwrap_err();
        assert!(errs.includes(Error::InvalidSignature));
    }

    #[test]
    fn chains_are_bounded() {
        let mut rng = det_rng(8);
        let k = generate_key(&mut rng);
        let mut c =
            new_self_signed_certificate(&k, SIGN, DEFAULT_CERT_TTL, "", now(), &mut rng).unwrap();
        for _ in 1..MAX_CHAIN_LENGTH {
            let csr = new_certificate_request(&k, SIGN, "").unwrap();
            c = sign_certificate_request(
                &csr,
                DEFAULT_CERT_TTL,
                Issuer::Certificate {
                    key: &k,
                    certificate: &c,
                },
                now(),
                &mut rng,
            )
            .unwrap();
        }
        verify_certificate(&c, now()).unwrap();
        let csr = new_certificate_request(&k, SIGN, "").unwrap();
        let c = sign_certificate_request(
            &csr,
            DEFAULT_CERT_TTL,
            Issuer::Certificate {
                key: &k,
                certificate: &c,
            },
            now(),
            &mut rng,
        )
        .unwrap();
        assert_eq!(
            verify_certificate(&c, now()).unwrap_err().errors(),
            &[Error::ChainTooLong]
        );
    }

    #[test]
    fn host_certificates_extend_a_membership_chain() {
        let (leaf, network_key, member_key, leaf_key) = chain();
        let member = certificate_parent(&leaf).unwrap();
        let host = host_certificate(&leaf_key.public, &member_key, member).unwrap();
        let c = verify_certificate(&host, now()).unwrap();
        let m = certificate_body(member).unwrap();
        assert_eq!(c.key, leaf_key.public);
        assert_eq!(c.key_usage, SIGN);
        assert_eq!(certificate_parent(&host), Some(member));
        assert_eq!(c.network_key(), network_key.public.as_slice());
        assert_eq!((c.not_before, c.not_after), (m.not_before, m.not_after));
        assert_eq!(c.serial_number.len(), SERIAL_NUMBER_SIZE);
        assert_eq!(c.issuer_serial_number, m.serial_number);
        assert_eq!(
            host_certificate(&leaf_key.public, &member_key, member).unwrap(),
            host
        );

        assert_eq!(
            host_certificate(&leaf_key.public, &leaf_key, member),
            Err(Error::IssuerMismatch)
        );
        assert_eq!(
            host_certificate(&leaf_key.public[..31], &member_key, member),
            Err(Error::InvalidKeyLength)
        );
        let peer_only = resign(member, &network_key, |b| b.key_usage = PEER);
        assert_eq!(
            host_certificate(&leaf_key.public, &member_key, &peer_only),
            Err(Error::UnsupportedKeyUsage)
        );
    }

    #[test]
    fn chain_validity_is_the_intersection_of_the_chain() {
        let (leaf, ..) = chain();
        let (not_before, not_after) = CertificateChain::decode(&leaf).unwrap().validity();
        assert_eq!(not_before, Time::from_unix(1_700_000_000 - 3600, 0));
        assert_eq!(not_after, Time::from_unix(1_700_000_000 + 86_400, 0));
        verify_certificate(&leaf, not_before).unwrap();
        verify_certificate(&leaf, not_after).unwrap();
    }

    #[test]
    fn validity_overflow_saturates() {
        let mut rng = det_rng(4);
        let k = generate_key(&mut rng);
        let c =
            new_self_signed_certificate(&k, SIGN, DEFAULT_CERT_TTL, "", now(), &mut rng).unwrap();
        let c = resign(&c, &k, |b| b.not_after = u64::MAX);
        verify_certificate(&c, now()).unwrap();
        let c = resign(&c, &k, |b| b.not_before = u64::MAX);
        assert_eq!(
            verify_certificate(&c, now()).unwrap_err().errors(),
            &[Error::NotBeforeRange]
        );
    }

    #[test]
    fn expired_outside_validity_period() {
        let (leaf, ..) = chain();
        let leaf = CertificateChain::decode(&leaf).unwrap();
        assert!(!leaf.is_expired(now()));
        assert!(leaf.is_expired(now() + Duration::HOUR * 25));
        assert!(leaf.is_expired(now() - Duration::HOUR * 2));
        assert!(leaf.is_expired(Time::from_unix(1_700_000_000 + 86_400, 0)));
    }
}
