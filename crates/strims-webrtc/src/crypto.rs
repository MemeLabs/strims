// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! str0m's crypto provider, in pure Rust.
//!
//! str0m is built with no crypto feature: its `rust-crypto` feature enables
//! dimpl's `rcgen`, which builds aws-lc (C, and CMake/NASM on Windows). This
//! provider is the same RustCrypto stack without rcgen:
//!
//! - **DTLS**: dimpl with only its `rust-crypto` backend. The certificate
//!   comes from [`crate::cert`] (seeded), and dimpl's rng is seeded per
//!   connection from the certificate's key ([`crate::cert::dtls_rng_seed`]).
//!   ICE has already checked return routability, so the server cookie is
//!   off.
//! - **SHA1-HMAC** (STUN message integrity) and **SHA-256** (fingerprints).
//! - **SRTP**: AES-128-CM-SHA1-80 and AEAD-AES-128/256-GCM. vnic sends no
//!   media, but the ClientHello must carry `use_srtp`, so str0m negotiates a
//!   profile and derives SRTP contexts that are never used.
//!
//! The SRTP ciphers are adapted from `str0m-rust-crypto` 0.6.0
//! (MIT/Apache-2.0). Malformed lengths return errors.

// The provider traits take `Instant`s (made by `crate::clock`).
#![allow(clippy::disallowed_types)]

use std::sync::Arc;
use std::time::Instant;

use aes::cipher::consts::U16;
use aes::cipher::{BlockEncrypt, BlockSizeUser, KeyIvInit, StreamCipher};
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm, Nonce};
use hmac::{Hmac, Mac};
use str0m::config::CryptoProvider;
use str0m::crypto::dtls::{
    DtlsCert, DtlsImplError, DtlsInstance, DtlsOutput, DtlsProvider, DtlsVersion, ProtocolVersion,
};
use str0m::crypto::{
    AeadAes128Gcm, AeadAes128GcmCipher, AeadAes256Gcm, AeadAes256GcmCipher, Aes128CmSha1_80Cipher,
    CryptoError, Sha1HmacProvider, Sha256Provider, SrtpProvider, SupportedAeadAes128Gcm,
    SupportedAeadAes256Gcm, SupportedAes128CmSha1_80,
};

/// The provider every connection uses.
pub(crate) fn provider() -> Arc<CryptoProvider> {
    static SRTP: Srtp = Srtp;
    static SHA1_HMAC: Sha1HmacImpl = Sha1HmacImpl;
    static SHA256: Sha256Impl = Sha256Impl;
    static DTLS: Dtls = Dtls;
    Arc::new(CryptoProvider {
        srtp_provider: &SRTP,
        sha1_hmac_provider: &SHA1_HMAC,
        sha256_provider: &SHA256,
        dtls_provider: &DTLS,
    })
}

// ---- hashes ----------------------------------------------------------------

#[derive(Debug)]
struct Sha1HmacImpl;

impl Sha1HmacProvider for Sha1HmacImpl {
    fn sha1_hmac(&self, key: &[u8], payloads: &[&[u8]]) -> [u8; 20] {
        let Ok(mut mac) = <Hmac<sha1::Sha1> as Mac>::new_from_slice(key) else {
            // HMAC takes keys of any length.
            return [0; 20];
        };
        for p in payloads {
            mac.update(p);
        }
        mac.finalize().into_bytes().into()
    }
}

#[derive(Debug)]
struct Sha256Impl;

impl Sha256Provider for Sha256Impl {
    fn sha256(&self, data: &[u8]) -> [u8; 32] {
        use sha2::Digest;
        sha2::Sha256::digest(data).into()
    }
}

// ---- DTLS ------------------------------------------------------------------

#[derive(Debug)]
struct Dtls;

impl DtlsProvider for Dtls {
    /// Certificates are always supplied (`RtcConfig::set_dtls_cert`).
    fn generate_certificate(&self) -> Option<DtlsCert> {
        None
    }

    fn new_dtls(
        &self,
        cert: &DtlsCert,
        now: Instant,
        dtls_version: DtlsVersion,
        mtu: Option<usize>,
    ) -> Result<Box<dyn DtlsInstance>, CryptoError> {
        let mut builder = dimpl::Config::builder()
            .use_server_cookie(false)
            .with_crypto_provider(dimpl::crypto::rust_crypto::default_provider())
            // TODO: seed the ECDHE keys too, once dimpl takes an rng.
            .dangerously_set_rng_seed(crate::cert::dtls_rng_seed(&cert.private_key));
        if let Some(mtu) = mtu {
            builder = builder.mtu(mtu);
        }
        let config = Arc::new(
            builder
                .build()
                .map_err(|e| CryptoError::Other(format!("dimpl config: {e}")))?,
        );
        let cert = dimpl::DtlsCertificate {
            certificate: cert.certificate.clone(),
            private_key: cert.private_key.clone(),
        };
        let dtls = match dtls_version {
            DtlsVersion::Dtls12 => dimpl::Dtls::new_12(config, cert, now),
            DtlsVersion::Dtls13 => dimpl::Dtls::new_13(config, cert, now),
            DtlsVersion::Auto => dimpl::Dtls::new_auto(config, cert, now),
            other => {
                return Err(CryptoError::Other(format!(
                    "unsupported DTLS version: {other}"
                )));
            }
        };
        Ok(Box::new(DtlsConn(dtls)))
    }
}

struct DtlsConn(dimpl::Dtls);

impl std::fmt::Debug for DtlsConn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DtlsConn")
    }
}

impl DtlsInstance for DtlsConn {
    fn set_active(&mut self, active: bool) {
        self.0.set_active(active);
    }

    fn handle_packet(&mut self, packet: &[u8]) -> Result<(), DtlsImplError> {
        self.0.handle_packet(packet)
    }

    fn poll_output<'a>(&mut self, buf: &'a mut [u8]) -> DtlsOutput<'a> {
        self.0.poll_output(buf)
    }

    fn handle_timeout(&mut self, now: Instant) -> Result<(), DtlsImplError> {
        self.0.handle_timeout(now)
    }

    fn send_application_data(&mut self, data: &[u8]) -> Result<(), DtlsImplError> {
        self.0.send_application_data(data)
    }

    fn is_active(&self) -> bool {
        self.0.is_active()
    }

    fn protocol_version(&self) -> Option<ProtocolVersion> {
        self.0.protocol_version()
    }

    fn is_closing(&self) -> bool {
        self.0.is_closing()
    }

    fn is_closed(&self) -> bool {
        self.0.is_closed()
    }

    fn close(&mut self) -> Result<(), DtlsImplError> {
        self.0.close()
    }
}

// ---- SRTP ------------------------------------------------------------------

type Aes128Ctr = ctr::Ctr128BE<aes::Aes128>;

fn short(what: &str) -> CryptoError {
    CryptoError::Other(format!("{what}: buffer too short"))
}

struct CmSha1Cipher {
    key: [u8; 16],
}

impl std::fmt::Debug for CmSha1Cipher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CmSha1Cipher")
    }
}

impl Aes128CmSha1_80Cipher for CmSha1Cipher {
    fn encrypt(
        &mut self,
        iv: &[u8; 16],
        input: &[u8],
        output: &mut [u8],
    ) -> Result<(), CryptoError> {
        let out = output
            .get_mut(..input.len())
            .ok_or_else(|| short("aes-cm"))?;
        out.copy_from_slice(input);
        Aes128Ctr::new(&self.key.into(), iv.into()).apply_keystream(out);
        Ok(())
    }

    fn decrypt(
        &mut self,
        iv: &[u8; 16],
        input: &[u8],
        output: &mut [u8],
    ) -> Result<(), CryptoError> {
        // CTR mode is its own inverse.
        self.encrypt(iv, input, output)
    }
}

fn gcm_seal<C: Aead>(
    cipher: &C,
    iv: &[u8],
    aad: &[u8],
    input: &[u8],
    output: &mut [u8],
) -> Result<(), CryptoError> {
    let sealed = cipher
        .encrypt(Nonce::from_slice(iv), Payload { msg: input, aad })
        .map_err(|_| CryptoError::Other("aes-gcm seal".into()))?;
    output
        .get_mut(..sealed.len())
        .ok_or_else(|| short("aes-gcm"))?
        .copy_from_slice(&sealed);
    Ok(())
}

fn gcm_open<C: Aead>(
    cipher: &C,
    iv: &[u8],
    aads: &[&[u8]],
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, CryptoError> {
    let aad = aads.concat();
    let opened = cipher
        .decrypt(
            Nonce::from_slice(iv),
            Payload {
                msg: input,
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::Other("aes-gcm open".into()))?;
    output
        .get_mut(..opened.len())
        .ok_or_else(|| short("aes-gcm"))?
        .copy_from_slice(&opened);
    Ok(opened.len())
}

struct Gcm128(Aes128Gcm);
struct Gcm256(Aes256Gcm);

impl std::fmt::Debug for Gcm128 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Gcm128")
    }
}

impl std::fmt::Debug for Gcm256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Gcm256")
    }
}

impl AeadAes128GcmCipher for Gcm128 {
    fn encrypt(
        &mut self,
        iv: &[u8; AeadAes128Gcm::IV_LEN],
        aad: &[u8],
        input: &[u8],
        output: &mut [u8],
    ) -> Result<(), CryptoError> {
        gcm_seal(&self.0, iv, aad, input, output)
    }

    fn decrypt(
        &mut self,
        iv: &[u8; AeadAes128Gcm::IV_LEN],
        aads: &[&[u8]],
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, CryptoError> {
        gcm_open(&self.0, iv, aads, input, output)
    }
}

impl AeadAes256GcmCipher for Gcm256 {
    fn encrypt(
        &mut self,
        iv: &[u8; AeadAes256Gcm::IV_LEN],
        aad: &[u8],
        input: &[u8],
        output: &mut [u8],
    ) -> Result<(), CryptoError> {
        gcm_seal(&self.0, iv, aad, input, output)
    }

    fn decrypt(
        &mut self,
        iv: &[u8; AeadAes256Gcm::IV_LEN],
        aads: &[&[u8]],
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, CryptoError> {
        gcm_open(&self.0, iv, aads, input, output)
    }
}

#[derive(Debug)]
struct CmSha1Factory;
#[derive(Debug)]
struct Gcm128Factory;
#[derive(Debug)]
struct Gcm256Factory;

impl SupportedAes128CmSha1_80 for CmSha1Factory {
    fn create_cipher(&self, key: [u8; 16], _encrypt: bool) -> Box<dyn Aes128CmSha1_80Cipher> {
        Box::new(CmSha1Cipher { key })
    }
}

impl SupportedAeadAes128Gcm for Gcm128Factory {
    fn create_cipher(&self, key: [u8; 16], _encrypt: bool) -> Box<dyn AeadAes128GcmCipher> {
        Box::new(Gcm128(Aes128Gcm::new(GenericArray::from_slice(&key))))
    }
}

impl SupportedAeadAes256Gcm for Gcm256Factory {
    fn create_cipher(&self, key: [u8; 32], _encrypt: bool) -> Box<dyn AeadAes256GcmCipher> {
        Box::new(Gcm256(Aes256Gcm::new(GenericArray::from_slice(&key))))
    }
}

#[derive(Debug)]
struct Srtp;

/// One AES-ECB round over a 16-byte block plus its PKCS#7 padding block,
/// the 32 bytes str0m's SRTP key derivation expects. Wrong lengths (never
/// produced by str0m) leave `output` unchanged.
fn ecb_round<C: BlockEncrypt + BlockSizeUser<BlockSize = U16>>(
    cipher: &C,
    input: &[u8],
    output: &mut [u8],
) {
    let (Ok(block), Some(out)) = (<[u8; 16]>::try_from(input), output.get_mut(..32)) else {
        return;
    };
    let (first, second) = out.split_at_mut(16);
    let mut b = GenericArray::from(block);
    cipher.encrypt_block(&mut b);
    first.copy_from_slice(&b);
    let mut p = GenericArray::from([0x10u8; 16]);
    cipher.encrypt_block(&mut p);
    second.copy_from_slice(&p);
}

impl SrtpProvider for Srtp {
    fn aes_128_cm_sha1_80(&self) -> &'static dyn SupportedAes128CmSha1_80 {
        &CmSha1Factory
    }

    fn aead_aes_128_gcm(&self) -> &'static dyn SupportedAeadAes128Gcm {
        &Gcm128Factory
    }

    fn aead_aes_256_gcm(&self) -> &'static dyn SupportedAeadAes256Gcm {
        &Gcm256Factory
    }

    fn srtp_aes_128_ecb_round(&self, key: &[u8], input: &[u8], output: &mut [u8]) {
        if let Ok(c) = aes::Aes128::new_from_slice(key) {
            ecb_round(&c, input, output);
        }
    }

    fn srtp_aes_256_ecb_round(&self, key: &[u8], input: &[u8], output: &mut [u8]) {
        if let Ok(c) = aes::Aes256::new_from_slice(key) {
            ecb_round(&c, input, output);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn hashes() {
        let p = provider();
        // RFC 2202 test case 2.
        assert_eq!(
            hex::encode(
                p.sha1_hmac_provider
                    .sha1_hmac(b"Jefe", &[b"what do ya want ", b"for nothing?"])
            ),
            "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"
        );
        assert_eq!(
            hex::encode(p.sha256_provider.sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn srtp_ciphers_round_trip() {
        let p = provider();
        let mut c = p
            .srtp_provider
            .aes_128_cm_sha1_80()
            .create_cipher([1; 16], true);
        let mut out = [0u8; 5];
        c.encrypt(&[2; 16], b"hello", &mut out).unwrap();
        let mut back = [0u8; 5];
        c.decrypt(&[2; 16], &out, &mut back).unwrap();
        assert_eq!(&back, b"hello");
        assert!(c.encrypt(&[2; 16], b"hello", &mut [0u8; 4]).is_err());

        let mut g = p
            .srtp_provider
            .aead_aes_128_gcm()
            .create_cipher([3; 16], true);
        let mut sealed = [0u8; 5 + 16];
        g.encrypt(&[4; 12], &[5; 12], b"hello", &mut sealed)
            .unwrap();
        let mut opened = [0u8; 21];
        let n = g
            .decrypt(&[4; 12], &[&[5; 6], &[5; 6]], &sealed, &mut opened)
            .unwrap();
        assert_eq!(&opened[..n], b"hello");
        assert!(
            g.decrypt(&[4; 12], &[&[6; 12]], &sealed, &mut opened)
                .is_err()
        );

        let mut out = [0u8; 32];
        p.srtp_provider
            .srtp_aes_128_ecb_round(&[0; 16], &[0; 16], &mut out);
        // FIPS-197 style: AES-128(0^128) under the zero key.
        assert_eq!(hex::encode(&out[..16]), "66e94bd4ef8a2c3b884cfa59ca342b2e");
    }
}
