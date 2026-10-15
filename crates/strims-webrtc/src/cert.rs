// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Self-signed ECDSA P-256 DTLS certificates, drawn from a `DetRng`.
//!
//! Every connection gets a fresh certificate, authenticated only by the SDP
//! fingerprint. It is written by hand from the connection's seeded rng, so
//! simulations get the same certificates (and fingerprints) every run, and
//! so the build needs neither rcgen (which builds aws-lc) nor the OS rng:
//!
//! - X.509 v3, no extensions, signed with ecdsa-with-SHA256 (RFC 6979
//!   deterministic nonces);
//! - subject and issuer `CN=strims`;
//! - a random positive 16-byte serial: issuer and serial must be unique
//!   together, and every certificate has the same issuer;
//! - valid from a day before `now` to a year after it;
//! - the private key as PKCS#8 DER, which dimpl's key provider loads.

use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};

/// A DER certificate and its PKCS#8 DER private key.
#[derive(Clone)]
pub(crate) struct Identity {
    pub(crate) certificate: Vec<u8>,
    pub(crate) private_key: Vec<u8>,
}

const OID_EC_PUBLIC_KEY: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01];
const OID_PRIME256V1: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07];
const OID_ECDSA_WITH_SHA256: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02];
const OID_COMMON_NAME: &[u8] = &[0x55, 0x04, 0x03];
const COMMON_NAME: &[u8] = b"strims";

const TAG_INTEGER: u8 = 0x02;
const TAG_BIT_STRING: u8 = 0x03;
const TAG_OCTET_STRING: u8 = 0x04;
const TAG_OID: u8 = 0x06;
const TAG_UTF8_STRING: u8 = 0x0C;
const TAG_UTC_TIME: u8 = 0x17;
const TAG_GENERALIZED_TIME: u8 = 0x18;
const TAG_SEQUENCE: u8 = 0x30;
const TAG_SET: u8 = 0x31;
const TAG_CONTEXT_0: u8 = 0xA0;
const TAG_CONTEXT_1: u8 = 0xA1;

/// Generates a certificate valid around `now`.
pub(crate) fn generate(rng: &mut DetRng, now: Time) -> Identity {
    let key = signing_key(rng);
    let point = key.verifying_key().to_encoded_point(false);
    let point = point.as_bytes();

    let mut serial = [0u8; 16];
    rand_core::Rng::fill_bytes(rng, &mut serial);
    if let Some(b) = serial.first_mut() {
        // Positive, and exactly 16 bytes once encoded.
        *b = (*b & 0x7F) | 0x40;
    }

    let algorithm = tlv(TAG_SEQUENCE, &tlv(TAG_OID, OID_ECDSA_WITH_SHA256));
    let name = tlv(
        TAG_SEQUENCE,
        &tlv(
            TAG_SET,
            &tlv(
                TAG_SEQUENCE,
                &[
                    tlv(TAG_OID, OID_COMMON_NAME),
                    tlv(TAG_UTF8_STRING, COMMON_NAME),
                ]
                .concat(),
            ),
        ),
    );
    let validity = tlv(
        TAG_SEQUENCE,
        &[
            x509_time(now - Duration::HOUR * 24),
            x509_time(now + Duration::HOUR * 24 * 365),
        ]
        .concat(),
    );
    let spki = tlv(
        TAG_SEQUENCE,
        &[
            ec_algorithm(),
            tlv(TAG_BIT_STRING, &[&[0u8][..], point].concat()),
        ]
        .concat(),
    );
    let tbs = tlv(
        TAG_SEQUENCE,
        &[
            tlv(TAG_CONTEXT_0, &tlv(TAG_INTEGER, &[2])),
            unsigned_integer(&serial),
            algorithm.clone(),
            name.clone(),
            validity,
            name,
            spki,
        ]
        .concat(),
    );

    let signature: Signature = key.sign(&tbs);
    let (r, s) = signature.split_bytes();
    let signature = tlv(
        TAG_SEQUENCE,
        &[unsigned_integer(&r), unsigned_integer(&s)].concat(),
    );
    let certificate = tlv(
        TAG_SEQUENCE,
        &[
            tbs,
            algorithm,
            tlv(TAG_BIT_STRING, &[&[0u8][..], &signature].concat()),
        ]
        .concat(),
    );

    let secret = key.to_bytes();
    let ec_private_key = tlv(
        TAG_SEQUENCE,
        &[
            tlv(TAG_INTEGER, &[1]),
            tlv(TAG_OCTET_STRING, &secret),
            tlv(
                TAG_CONTEXT_1,
                &tlv(TAG_BIT_STRING, &[&[0u8][..], point].concat()),
            ),
        ]
        .concat(),
    );
    let private_key = tlv(
        TAG_SEQUENCE,
        &[
            tlv(TAG_INTEGER, &[0]),
            ec_algorithm(),
            tlv(TAG_OCTET_STRING, &ec_private_key),
        ]
        .concat(),
    );

    Identity {
        certificate,
        private_key,
    }
}

/// A uniformly random P-256 scalar: 32 bytes, redrawn in the (2^-32)
/// case that they are zero or not below the group order.
fn signing_key(rng: &mut DetRng) -> SigningKey {
    loop {
        let mut b = [0u8; 32];
        rand_core::Rng::fill_bytes(rng, &mut b);
        if let Ok(k) = SigningKey::from_slice(&b) {
            return k;
        }
    }
}

fn ec_algorithm() -> Vec<u8> {
    tlv(
        TAG_SEQUENCE,
        &[
            tlv(TAG_OID, OID_EC_PUBLIC_KEY),
            tlv(TAG_OID, OID_PRIME256V1),
        ]
        .concat(),
    )
}

fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let n = content.len();
    let mut out = Vec::with_capacity(n + 4);
    out.push(tag);
    // Everything here is far below 64 KiB.
    let [hi, lo] = u16::try_from(n).unwrap_or(u16::MAX).to_be_bytes();
    match n {
        0..=0x7F => out.push(lo),
        0x80..=0xFF => out.extend_from_slice(&[0x81, lo]),
        _ => out.extend_from_slice(&[0x82, hi, lo]),
    }
    out.extend_from_slice(content);
    out
}

/// A DER INTEGER for an unsigned big-endian value.
fn unsigned_integer(b: &[u8]) -> Vec<u8> {
    let start = b.iter().position(|&x| x != 0).unwrap_or(b.len());
    let v = b.get(start..).unwrap_or(&[]);
    let mut c = Vec::with_capacity(v.len() + 1);
    if v.first().is_none_or(|&x| x & 0x80 != 0) {
        c.push(0);
    }
    c.extend_from_slice(v);
    tlv(TAG_INTEGER, &c)
}

/// UTCTime for 1950..=2049, else GeneralizedTime (RFC 5280 §4.1.2.5).
fn x509_time(t: Time) -> Vec<u8> {
    let secs = t.unix_nanos().div_euclid(1_000_000_000);
    let (y, mo, d) = civil_from_days(secs.div_euclid(86_400));
    let s = secs.rem_euclid(86_400);
    let (h, mi, sec) = (s / 3600, s % 3600 / 60, s % 60);
    let y = y.clamp(0, 9999);
    if (1950..2050).contains(&y) {
        let v = format!("{:02}{mo:02}{d:02}{h:02}{mi:02}{sec:02}Z", y % 100);
        tlv(TAG_UTC_TIME, v.as_bytes())
    } else {
        let v = format!("{y:04}{mo:02}{d:02}{h:02}{mi:02}{sec:02}Z");
        tlv(TAG_GENERALIZED_TIME, v.as_bytes())
    }
}

/// Days since 1970-01-01 to a proleptic Gregorian date (H. Hinnant's
/// `civil_from_days`).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z.saturating_add(719_468);
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// A per-connection seed for dimpl's non-cryptographic rng (handshake
/// randoms, retransmission jitter, AES-GCM explicit nonces), derived from
/// the connection's private key: deterministic in simulations, unknown to
/// anyone who doesn't hold the key, and distinct per connection.
pub(crate) fn dtls_rng_seed(private_key: &[u8]) -> u64 {
    use sha2::Digest;
    let h = sha2::Sha256::new()
        .chain_update(b"strims-webrtc dtls rng\0")
        .chain_update(private_key)
        .finalize();
    let mut b = [0u8; 8];
    b.iter_mut().zip(h.iter()).for_each(|(o, i)| *o = *i);
    u64::from_be_bytes(b)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

    use p256::ecdsa::VerifyingKey;
    use p256::ecdsa::signature::Verifier;
    use strims_base::rng::det_rng;

    use super::*;

    /// Splits one DER element off the front: (tag, content, rest).
    fn take(b: &[u8]) -> (u8, &[u8], &[u8]) {
        let tag = b[0];
        let (len, hdr) = match b[1] {
            n @ 0..=0x7F => (n as usize, 2),
            0x81 => (b[2] as usize, 3),
            0x82 => (u16::from_be_bytes([b[2], b[3]]) as usize, 4),
            _ => panic!("length"),
        };
        (tag, &b[hdr..hdr + len], &b[hdr + len..])
    }

    #[test]
    fn certificate_verifies_and_is_deterministic() {
        let now = Time::from_unix(1_760_400_000, 0); // 2025-10-14
        let id = generate(&mut det_rng(1), now);
        let again = generate(&mut det_rng(1), now);
        assert_eq!(id.certificate, again.certificate);
        assert_eq!(id.private_key, again.private_key);
        assert_ne!(generate(&mut det_rng(2), now).certificate, id.certificate);

        // Certificate ::= SEQUENCE { tbs, alg, sig }
        let (tag, cert, rest) = take(&id.certificate);
        assert_eq!((tag, rest.len()), (TAG_SEQUENCE, 0));
        let (_, tbs_content, rest) = take(cert);
        let tbs_full = &cert[..cert.len() - rest.len()];
        let (_, _alg, rest) = take(rest);
        let (tag, sig_bits, rest) = take(rest);
        assert_eq!((tag, rest.len(), sig_bits[0]), (TAG_BIT_STRING, 0, 0));

        // Walk the TBS to the public key.
        let (_, version, r) = take(tbs_content);
        assert_eq!(version, &[2, 1, 2]);
        let (_, serial, r) = take(r);
        assert_eq!(serial.len(), 16);
        let (_, _, r) = take(r); // signature alg
        let (_, _, r) = take(r); // issuer
        let (_, validity, r) = take(r);
        let (t0, not_before, v) = take(validity);
        let (_, not_after, _) = take(v);
        assert_eq!(t0, TAG_UTC_TIME);
        assert_eq!(not_before, b"251013000000Z");
        assert_eq!(not_after, b"261014000000Z");
        let (_, _, r) = take(r); // subject
        let (_, spki, r) = take(r);
        assert!(r.is_empty());
        let (_, _, r) = take(spki);
        let (_, point, _) = take(r);
        let vk = VerifyingKey::from_sec1_bytes(&point[1..]).unwrap();

        let sig = Signature::from_der(&sig_bits[1..]).unwrap();
        vk.verify(tbs_full, &sig).unwrap();

        // The PKCS#8 key holds the same key pair.
        let (_, p8, _) = take(&id.private_key);
        let (_, _, r) = take(p8); // version
        let (_, _, r) = take(r); // algorithm
        let (_, ec, _) = take(r);
        let (_, ec, _) = take(ec);
        let (_, _, r) = take(ec);
        let (_, secret, _) = take(r);
        let sk = SigningKey::from_slice(secret).unwrap();
        assert_eq!(sk.verifying_key(), &vk);
    }

    #[test]
    fn generalized_time_outside_utc_range() {
        assert_eq!(
            x509_time(Time::from_unix(2_556_143_999, 0)), // 2050-12-31 23:59:59
            tlv(TAG_GENERALIZED_TIME, b"20501231235959Z")
        );
        assert_eq!(x509_time(Time::EPOCH), tlv(TAG_UTC_TIME, b"700101000000Z"));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
    }

    #[test]
    fn integers_are_minimal_and_positive() {
        assert_eq!(unsigned_integer(&[0, 0, 0x7F]), vec![2, 1, 0x7F]);
        assert_eq!(unsigned_integer(&[0x80]), vec![2, 2, 0, 0x80]);
        assert_eq!(unsigned_integer(&[0, 0]), vec![2, 1, 0]);
        assert_eq!(tlv(4, &[0; 200])[..3], [4, 0x81, 200]);
        assert_eq!(tlv(4, &[0; 300])[..4], [4, 0x82, 1, 44]);
    }
}
