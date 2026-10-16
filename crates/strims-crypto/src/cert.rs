// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Certificate requests, issuing and chain verification for
//! `strims.type.{Certificate, CertificateRequest}`.
//!
//! Signatures cover [`serialize_certificate`] and
//! [`serialize_certificate_request`]. Key usages are bit flags OR'ed into the
//! `key_usage` enum field, e.g. `KeyUsage::Peer as i32 | KeyUsage::Sign as
//! i32`.
//!
//! Validity times are whole Unix seconds. Conversions saturate: issuing
//! clamps times before the epoch to 0, and verification reads values above
//! `i64::MAX` as `i64::MAX`.

use core::fmt;

use ed25519_dalek::Signer;
use sha2::{Digest, Sha256};
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_proto::types::{
    Certificate, CertificateRequest, Key, KeyType, KeyUsage, certificate::ParentOneof,
};

use crate::{ED25519_PUBLIC_KEY_SIZE, Error, key};

/// How far before `now` a new certificate's validity starts.
pub const CERT_PREDATE_DURATION: Duration = Duration::HOUR;

/// About two years.
pub const DEFAULT_CERT_TTL: Duration = Duration::from_secs(2 * 365 * 24 * 60 * 60);

/// Size of a certificate serial number.
const SERIAL_NUMBER_SIZE: usize = 16;

/// Creates a certificate request for `key`, signed by it. An empty
/// `subject` leaves it unset.
pub fn new_certificate_request(
    key: &Key,
    usage: i32,
    subject: &str,
) -> Result<CertificateRequest, Error> {
    let mut csr = CertificateRequest {
        key: key.public.clone(),
        key_type: key.r#type,
        key_usage: usage,
        subject: subject.to_owned(),
        signature: Vec::new(),
    };
    let signing = key::signing_key(key)?;
    csr.signature = signing
        .sign(&serialize_certificate_request(&csr))
        .to_bytes()
        .to_vec();
    Ok(csr)
}

/// Checks that the request asks for no usage outside `usage` and that it's
/// signed by its own key.
pub fn verify_certificate_request(csr: &CertificateRequest, usage: i32) -> Result<(), Error> {
    if csr.key_usage & !usage != 0 {
        return Err(Error::UnsupportedKeyUsage);
    }
    if csr.key_type != KeyType::Ed25519 as i32 {
        return Err(Error::UnsupportedKeyType);
    }
    if csr.key.len() != ED25519_PUBLIC_KEY_SIZE {
        return Err(Error::InvalidKeyLength);
    }
    if !key::verify(
        &csr.key,
        &serialize_certificate_request(csr),
        &csr.signature,
    ) {
        return Err(Error::InvalidCertificateRequestSignature);
    }
    Ok(())
}

/// Issues a certificate for `csr`, signed by `key`, valid from an hour
/// before `now` until `now + valid_duration`, with a random 16-byte serial
/// number. The parent is left unset.
pub fn sign_certificate_request(
    csr: &CertificateRequest,
    valid_duration: Duration,
    key: &Key,
    now: Time,
    rng: &mut DetRng,
) -> Result<Certificate, Error> {
    let mut serial_number = vec![0u8; SERIAL_NUMBER_SIZE];
    rand_core::Rng::fill_bytes(rng, &mut serial_number);
    let mut cert = Certificate {
        key: csr.key.clone(),
        key_type: csr.key_type,
        key_usage: csr.key_usage,
        subject: csr.subject.clone(),
        not_before: unix_seconds(now - CERT_PREDATE_DURATION),
        not_after: unix_seconds(now + valid_duration),
        serial_number,
        signature: Vec::new(),
        parent_oneof: None,
    };
    let signing = key::signing_key(key)?;
    cert.signature = signing
        .sign(&serialize_certificate(&cert))
        .to_bytes()
        .to_vec();
    Ok(cert)
}

/// The certificate a host presents on a network: `host_key` (an ed25519
/// public key) signed by the profile key, with usage `KEY_USAGE_SIGN`, the
/// validity period of `membership`, and `membership` as its parent.
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
    if membership.key != profile_key.public {
        return Err(Error::IssuerMismatch);
    }
    if membership.key_usage & KeyUsage::Sign as i32 == 0 {
        return Err(Error::UnsupportedKeyUsage);
    }
    let digest = Sha256::new()
        .chain_update(&membership.serial_number)
        .chain_update(host_key)
        .finalize();
    let mut cert = Certificate {
        key: host_key.to_vec(),
        key_type: KeyType::Ed25519 as i32,
        key_usage: KeyUsage::Sign as i32,
        subject: String::new(),
        not_before: membership.not_before,
        not_after: membership.not_after,
        serial_number: digest
            .get(..SERIAL_NUMBER_SIZE)
            .unwrap_or_default()
            .to_vec(),
        signature: Vec::new(),
        parent_oneof: None,
    };
    let signing = key::signing_key(profile_key)?;
    cert.signature = signing
        .sign(&serialize_certificate(&cert))
        .to_bytes()
        .to_vec();
    cert.parent_oneof = Some(ParentOneof::Parent(Box::new(membership.clone())));
    Ok(cert)
}

/// The period in which every certificate in the chain is valid: the latest
/// not-before and the earliest not-after, both inclusive, as
/// [`verify_certificate`] reads them. Empty when the first follows the
/// second.
pub fn chain_validity(cert: &Certificate) -> (Time, Time) {
    let (mut not_before, mut not_after) = (0, u64::MAX);
    let mut next = Some(cert);
    while let Some(cert) = next {
        not_before = not_before.max(cert.not_before);
        not_after = not_after.min(cert.not_after);
        next = certificate_parent(cert);
    }
    (from_unix_seconds(not_before), from_unix_seconds(not_after))
}

/// Whole Unix seconds; times before the epoch clamp to zero.
fn unix_seconds(t: Time) -> u64 {
    u64::try_from(t.unix()).unwrap_or(0)
}

/// The time `secs` after the epoch, saturating at `i64::MAX` seconds.
fn from_unix_seconds(secs: u64) -> Time {
    Time::from_unix(i64::try_from(secs).unwrap_or(i64::MAX), 0)
}

/// The certificate's parent, if the `parent_oneof` holds a full certificate.
pub fn certificate_parent(cert: &Certificate) -> Option<&Certificate> {
    match &cert.parent_oneof {
        Some(ParentOneof::Parent(p)) => Some(p),
        Some(ParentOneof::ParentSerialNumber(_)) | None => None,
    }
}

/// Verifies every certificate in the chain at `now`: each must be signed by
/// its parent, or by itself if it has none, the signer must have
/// `KEY_USAGE_SIGN`, and `now` must fall within its validity period,
/// inclusive.
///
/// A certificate whose `parent_oneof` holds only a serial number is checked
/// as a self-signed root. Chain depth is bounded by prost's decode recursion
/// limit (100) for certificates read from the wire.
pub fn verify_certificate(cert: &Certificate, now: Time) -> Result<(), CertificateErrors> {
    let mut errs = CertificateErrors::default();
    let mut buf = Vec::new();
    let mut next = Some(cert);
    while let Some(cert) = next {
        let parent = certificate_parent(cert);
        let signing_cert = parent.unwrap_or(cert);
        if signing_cert.key_usage & KeyUsage::Sign as i32 == 0 {
            errs.push(Error::UnsupportedKeyUsage);
        }

        buf.clear();
        serialize_certificate_into(cert, &mut buf);

        let mut valid_sig = false;
        if signing_cert.key_type == KeyType::Ed25519 as i32 {
            if signing_cert.key.len() == ED25519_PUBLIC_KEY_SIZE {
                valid_sig = key::verify(&signing_cert.key, &buf, &cert.signature);
            } else {
                errs.push(Error::InvalidKeyLength);
            }
        } else {
            errs.push(Error::UnsupportedKeyType);
        }
        if !valid_sig {
            errs.push(Error::InvalidSignature);
        }

        if now.before(from_unix_seconds(cert.not_before)) {
            errs.push(Error::NotBeforeRange);
        }
        if now.after(from_unix_seconds(cert.not_after)) {
            errs.push(Error::NotAfterRange);
        }
        next = parent;
    }
    errs.into_result()
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
    let csr = new_certificate_request(key, usage, subject)?;
    sign_certificate_request(&csr, valid_duration, key, now, rng)
}

/// The last certificate in the parent chain.
pub fn certificate_root(cert: &Certificate) -> &Certificate {
    let mut cert = cert;
    while let Some(parent) = certificate_parent(cert) {
        cert = parent;
    }
    cert
}

/// The root certificate's key, which identifies the network.
pub fn certificate_network_key(cert: &Certificate) -> &[u8] {
    &certificate_root(cert).key
}

/// Whether `now`, in whole seconds, is outside the certificate's validity
/// period or on either boundary.
pub fn cert_is_expired(cert: &Certificate, now: Time) -> bool {
    let now = unix_seconds(now);
    cert.not_after <= now || cert.not_before >= now
}

/// The bytes a certificate's signature covers: key, key type (u32 BE), key
/// usage (u32 BE), subject, not before (u64 BE), not after (u64 BE) and
/// serial number.
///
/// Fields are concatenated without length prefixes, so the signature
/// doesn't bind field boundaries: the subject can absorb the validity times,
/// and the validity times can be read from the serial number.
pub fn serialize_certificate(cert: &Certificate) -> Vec<u8> {
    let mut b = Vec::new();
    serialize_certificate_into(cert, &mut b);
    b
}

/// [`serialize_certificate`] appending to `b`.
pub fn serialize_certificate_into(cert: &Certificate, b: &mut Vec<u8>) {
    b.reserve(24 + cert.key.len() + cert.subject.len() + cert.serial_number.len());
    b.extend_from_slice(&cert.key);
    b.extend_from_slice(&cert.key_type.to_be_bytes());
    b.extend_from_slice(&cert.key_usage.to_be_bytes());
    b.extend_from_slice(cert.subject.as_bytes());
    b.extend_from_slice(&cert.not_before.to_be_bytes());
    b.extend_from_slice(&cert.not_after.to_be_bytes());
    b.extend_from_slice(&cert.serial_number);
}

/// The bytes a certificate request's signature covers: key, key type (u32
/// BE), key usage (u32 BE) and subject.
pub fn serialize_certificate_request(csr: &CertificateRequest) -> Vec<u8> {
    let mut b = Vec::with_capacity(8 + csr.key.len() + csr.subject.len());
    b.extend_from_slice(&csr.key);
    b.extend_from_slice(&csr.key_type.to_be_bytes());
    b.extend_from_slice(&csr.key_usage.to_be_bytes());
    b.extend_from_slice(csr.subject.as_bytes());
    b
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

    struct TestCase {
        name: &'static str,
        req: CertificateRequest,
        cert: Certificate,
        key: Key,
        err: Option<Error>,
    }

    fn build_test_cases() -> Vec<TestCase> {
        let mut rng = det_rng(1);
        let k = generate_key(&mut rng);

        let success_csr = new_certificate_request(&k, SIGN, "").unwrap();

        let mut invalid_len_csr = new_certificate_request(&k, SIGN, "").unwrap();
        invalid_len_csr.key.truncate(k.public.len() - 5);

        let mut invalid_type_csr = new_certificate_request(&k, SIGN, "").unwrap();
        invalid_type_csr.key_type = KeyType::X25519 as i32;

        let success_cert =
            sign_certificate_request(&success_csr, DEFAULT_CERT_TTL, &k, now(), &mut rng).unwrap();

        let mut invalid_len_cert =
            sign_certificate_request(&success_csr, DEFAULT_CERT_TTL, &k, now(), &mut rng).unwrap();
        invalid_len_cert.key = k.private[..k.private.len() - 5].to_vec();

        let mut invalid_type_cert =
            sign_certificate_request(&success_csr, DEFAULT_CERT_TTL, &k, now(), &mut rng).unwrap();
        invalid_type_cert.key_type = KeyType::X25519 as i32;

        vec![
            TestCase {
                name: "success",
                req: success_csr,
                key: k.clone(),
                cert: success_cert,
                err: None,
            },
            TestCase {
                name: "invalid key length",
                req: invalid_len_csr,
                key: Key {
                    private: k.private[..k.private.len() - 5].to_vec(),
                    ..k.clone()
                },
                cert: invalid_len_cert,
                err: Some(Error::InvalidKeyLength),
            },
            TestCase {
                name: "invalid key type (x25519)",
                req: invalid_type_csr,
                key: Key {
                    r#type: KeyType::X25519 as i32,
                    ..k
                },
                cert: invalid_type_cert,
                err: Some(Error::UnsupportedKeyType),
            },
        ]
    }

    #[test]
    fn test_new_certificate_request() {
        for tc in build_test_cases() {
            let res = new_certificate_request(&tc.key, 0, "");
            assert_eq!(res.err(), tc.err, "{}", tc.name);
        }
    }

    #[test]
    fn test_verify_certificate_request() {
        for tc in build_test_cases() {
            let res = verify_certificate_request(&tc.req, SIGN);
            assert_eq!(res.err(), tc.err, "{}", tc.name);
        }
    }

    #[test]
    fn test_sign_certificate_request() {
        for tc in build_test_cases() {
            let res = sign_certificate_request(
                &tc.req,
                DEFAULT_CERT_TTL,
                &tc.key,
                now(),
                &mut det_rng(2),
            );
            match tc.err {
                Some(e) => assert_eq!(res.err(), Some(e), "{}", tc.name),
                None => assert!(!res.unwrap().key.is_empty(), "{}", tc.name),
            }
        }
    }

    #[test]
    fn test_verify_certificate() {
        for tc in build_test_cases() {
            let res = verify_certificate(&tc.cert, now());
            match tc.err {
                Some(e) => {
                    let errs = res.expect_err(tc.name);
                    assert!(
                        errs.includes(e),
                        "{}: received errors: {errs}, expected: {e}",
                        tc.name
                    );
                }
                None => assert_eq!(res, Ok(()), "{}", tc.name),
            }
        }
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
        verify_certificate_request(&member_csr, PEER | SIGN).unwrap();
        let mut member =
            sign_certificate_request(&member_csr, DEFAULT_CERT_TTL, &network_key, now(), &mut rng)
                .unwrap();
        member.parent_oneof = Some(ParentOneof::Parent(Box::new(root)));

        let leaf_csr = new_certificate_request(&leaf_key, PEER | BOOTSTRAP, "").unwrap();
        let mut leaf =
            sign_certificate_request(&leaf_csr, Duration::HOUR * 24, &member_key, now(), &mut rng)
                .unwrap();
        leaf.parent_oneof = Some(ParentOneof::Parent(Box::new(member)));
        (leaf, network_key, member_key, leaf_key)
    }

    #[test]
    fn chain_verifies() {
        let (leaf, network_key, _, leaf_key) = chain();
        verify_certificate(&leaf, now()).unwrap();
        assert_eq!(
            certificate_network_key(&leaf),
            network_key.public.as_slice()
        );
        assert_eq!(certificate_root(&leaf).subject, "network");
        assert_eq!(leaf.key, leaf_key.public);
        assert_eq!(leaf.not_before, 1_700_000_000 - 3600);
        assert_eq!(leaf.not_after, 1_700_000_000 + 86_400);
        assert_eq!(leaf.serial_number.len(), 16);
    }

    #[test]
    fn chain_errors() {
        let (leaf, ..) = chain();

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
        t.subject = "mallory".into();
        assert_eq!(
            verify_certificate(&t, now()).unwrap_err().errors(),
            &[Error::InvalidSignature]
        );
        let mut t = leaf.clone();
        let Some(ParentOneof::Parent(member)) = &mut t.parent_oneof else {
            panic!()
        };
        member.key_usage = PEER;
        let errs = verify_certificate(&t, now()).unwrap_err();
        assert!(
            errs.includes(Error::UnsupportedKeyUsage) && errs.includes(Error::InvalidSignature)
        );
        assert_eq!(
            errs.to_string(),
            "unsupported key usage, invalid certificate signature"
        );

        // a serial-number parent is checked as self-signed
        let mut t = leaf.clone();
        t.parent_oneof = Some(ParentOneof::ParentSerialNumber(vec![0; 16]));
        let errs = verify_certificate(&t, now()).unwrap_err();
        assert_eq!(
            errs.errors(),
            &[Error::UnsupportedKeyUsage, Error::InvalidSignature]
        );
    }

    #[test]
    fn host_certificates_extend_a_membership_chain() {
        let (_, network_key, member_key, leaf_key) = chain();
        let (leaf, ..) = chain();
        let member = certificate_parent(&leaf).unwrap();
        let host = host_certificate(&leaf_key.public, &member_key, member).unwrap();
        verify_certificate(&host, now()).unwrap();
        assert_eq!(host.key, leaf_key.public);
        assert_eq!(host.key_usage, SIGN);
        assert_eq!(certificate_parent(&host), Some(member));
        assert_eq!(
            certificate_network_key(&host),
            network_key.public.as_slice()
        );
        assert_eq!(
            (host.not_before, host.not_after),
            (member.not_before, member.not_after)
        );
        assert_eq!(host.serial_number.len(), SERIAL_NUMBER_SIZE);
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
        let peer_only = Certificate {
            key_usage: PEER,
            ..member.clone()
        };
        assert_eq!(
            host_certificate(&leaf_key.public, &member_key, &peer_only),
            Err(Error::UnsupportedKeyUsage)
        );
    }

    #[test]
    fn chain_validity_is_the_intersection_of_the_chain() {
        let (leaf, ..) = chain();
        let (not_before, not_after) = chain_validity(&leaf);
        assert_eq!(not_before, Time::from_unix(1_700_000_000 - 3600, 0));
        assert_eq!(not_after, Time::from_unix(1_700_000_000 + 86_400, 0));
        verify_certificate(&leaf, not_before).unwrap();
        verify_certificate(&leaf, not_after).unwrap();
    }

    #[test]
    fn validity_overflow_saturates() {
        let mut rng = det_rng(4);
        let k = generate_key(&mut rng);
        let mut c =
            new_self_signed_certificate(&k, SIGN, DEFAULT_CERT_TTL, "", now(), &mut rng).unwrap();
        c.not_after = u64::MAX;
        c.signature = key::sign(&k.private, &serialize_certificate(&c))
            .unwrap()
            .to_vec();
        verify_certificate(&c, now()).unwrap();
        c.not_before = u64::MAX;
        c.signature = key::sign(&k.private, &serialize_certificate(&c))
            .unwrap()
            .to_vec();
        assert_eq!(
            verify_certificate(&c, now()).unwrap_err().errors(),
            &[Error::NotBeforeRange]
        );
    }

    #[test]
    fn serialization_is_malleable() {
        // The signed bytes have no length prefixes, so the subject can absorb
        // the validity times and the validity times can be read from the
        // serial number, under the same signature. Times whose bytes are
        // ASCII keep the forged subject valid UTF-8.
        let now = Time::from_unix(0x4040_4040 + 3600, 0);
        let ttl = Duration::from_secs(0x4141_4141 - 0x4040_4040 - 3600);
        let mut rng = det_rng(5);
        let k = generate_key(&mut rng);
        let c = new_self_signed_certificate(&k, SIGN, ttl, "s", now, &mut rng).unwrap();
        assert_eq!((c.not_before, c.not_after), (0x4040_4040, 0x4141_4141));

        let (nb, na) = c.serial_number.split_at(8);
        let forged = Certificate {
            subject: "s\0\0\0\0@@@@\0\0\0\0AAAA".into(),
            not_before: u64::from_be_bytes(nb.try_into().unwrap()),
            not_after: u64::from_be_bytes(na.try_into().unwrap()),
            serial_number: Vec::new(),
            ..c.clone()
        };
        assert_eq!(serialize_certificate(&forged), serialize_certificate(&c));
        // Saturating time conversion rejects the forgery: a not-before read
        // from random serial bytes is practically never in the past.
        let errs = verify_certificate(&forged, now).unwrap_err();
        assert_eq!(errs.errors(), &[Error::NotBeforeRange]);
    }

    #[test]
    fn expired_outside_validity_period() {
        let (leaf, ..) = chain();
        assert!(!cert_is_expired(&leaf, now()));
        assert!(cert_is_expired(&leaf, now() + Duration::HOUR * 25));
        assert!(cert_is_expired(&leaf, now() - Duration::HOUR * 2));
        assert!(cert_is_expired(
            &leaf,
            Time::from_unix(1_700_000_000 + 86_400, 0)
        ));
    }

    #[test]
    fn serialize_layout() {
        let c = Certificate {
            key: vec![1, 2],
            key_type: 1,
            key_usage: 4,
            subject: "s".into(),
            not_before: 5,
            not_after: 6,
            serial_number: vec![7],
            signature: vec![9; 64],
            parent_oneof: None,
        };
        assert_eq!(
            serialize_certificate(&c),
            [
                &[1, 2, 0, 0, 0, 1, 0, 0, 0, 4, b's'][..],
                &5u64.to_be_bytes(),
                &6u64.to_be_bytes(),
                &[7]
            ]
            .concat()
        );
        let csr = CertificateRequest {
            key: vec![1],
            key_type: 1,
            key_usage: 3,
            subject: "ab".into(),
            signature: vec![],
        };
        assert_eq!(
            serialize_certificate_request(&csr),
            [1, 0, 0, 0, 1, 0, 0, 0, 3, b'a', b'b']
        );
    }
}
