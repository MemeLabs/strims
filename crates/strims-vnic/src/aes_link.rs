// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The X25519 handshake and AES-CFB stream encryption that TCP links use.
//!
//! Each side writes an `AESLinkInit` (protocol version 1, its X25519 public
//! key and a random 16-byte IV) in the clear, and reads the other's. The
//! shared secret `X25519(own private, peer public)` is the AES-256 key for
//! both directions; each direction runs CFB-128 from its writer's IV. After
//! that every byte on the link is encrypted, including `PeerInit`.
//!
//! [`AesLinkHandshake`] is the handshake as a sans-IO state machine and
//! [`AesLink`] the stream transform. Encryption is in place and independent
//! of how the stream is chunked.
//!
//! The stream has no MAC.

use aes::Aes256;
use cfb_mode::cipher::KeyIvInit;
use cfb_mode::{BufDecryptor, BufEncryptor};
use strims_base::rng::DetRng;
use strims_proto::types::Key;
use strims_proto::vnic::v1::AesLinkInit;

use crate::PROTOCOL_VERSION;
use crate::stream::{MessageReader, StreamError, decode_stream_message, encode_stream_message};

/// AES block size, and so the IV length.
pub const AES_LINK_IV_LEN: usize = 16;

/// Errors from the AES link handshake.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AesLinkError {
    #[error("reading aes link init failed: {0}")]
    Read(StreamError),
    #[error("writing aes link init failed: {0}")]
    Write(StreamError),
    /// The peer presented a key other than the one the dialer expected.
    #[error("peer key mismatch")]
    PeerKeyMismatch,
    /// The peer's IV isn't [`AES_LINK_IV_LEN`] bytes.
    #[error("invalid iv length {0}")]
    InvalidIv(usize),
    /// X25519 failed: a key of the wrong length or a low-order point.
    #[error("{0}")]
    KeyAgreement(strims_crypto::Error),
}

/// The AES-CFB transform for one link.
pub struct AesLink {
    enc: BufEncryptor<Aes256>,
    dec: BufDecryptor<Aes256>,
}

impl std::fmt::Debug for AesLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AesLink")
    }
}

impl AesLink {
    /// A transform with `key` for both directions, encrypting from
    /// `write_iv` and decrypting from `read_iv`.
    pub fn new(
        key: &[u8; 32],
        write_iv: &[u8; AES_LINK_IV_LEN],
        read_iv: &[u8; AES_LINK_IV_LEN],
    ) -> AesLink {
        AesLink {
            enc: BufEncryptor::new(key.into(), write_iv.into()),
            dec: BufDecryptor::new(key.into(), read_iv.into()),
        }
    }

    /// Encrypts the next bytes written to the link, in place.
    pub fn encrypt(&mut self, buf: &mut [u8]) {
        self.enc.encrypt(buf);
    }

    /// Decrypts the next bytes read from the link, in place.
    pub fn decrypt(&mut self, buf: &mut [u8]) {
        self.dec.decrypt(buf);
    }
}

/// One side of the AES link handshake.
#[derive(Debug)]
pub struct AesLinkHandshake {
    /// Our X25519 key (`KEY_TYPE_X25519`, 32-byte private and public).
    key: Key,
    iv: [u8; AES_LINK_IV_LEN],
    /// The X25519 public key the dialer expects; `None` accepts any.
    peer_key: Option<[u8; 32]>,
    reader: MessageReader,
}

impl AesLinkHandshake {
    /// Starts a handshake with a random IV drawn from `rng`. `peer_key` is
    /// the X25519 public key the other side must present (the dialer knows
    /// it from the URI), or `None` to accept any (the listener).
    pub fn new(key: Key, peer_key: Option<[u8; 32]>, rng: &mut DetRng) -> AesLinkHandshake {
        let mut iv = [0u8; AES_LINK_IV_LEN];
        rand_core::Rng::fill_bytes(rng, &mut iv);
        AesLinkHandshake::with_iv(key, peer_key, iv)
    }

    /// Like [`AesLinkHandshake::new`] with a given IV, for tests and golden
    /// vectors.
    pub fn with_iv(
        key: Key,
        peer_key: Option<[u8; 32]>,
        iv: [u8; AES_LINK_IV_LEN],
    ) -> AesLinkHandshake {
        AesLinkHandshake {
            key,
            iv,
            peer_key,
            reader: MessageReader::new(),
        }
    }

    /// The init message this side sends.
    pub fn init(&self) -> AesLinkInit {
        AesLinkInit {
            protocol_version: PROTOCOL_VERSION,
            key: self.key.public.clone(),
            iv: self.iv.to_vec(),
        }
    }

    /// Appends the length-prefixed init message to `out`.
    pub fn write_init(&self, out: &mut Vec<u8>) -> Result<(), AesLinkError> {
        encode_stream_message(&self.init(), out).map_err(AesLinkError::Write)
    }

    /// Bytes buffered from a partly received peer init (at most `2 + 65535`).
    pub fn buffered(&self) -> usize {
        self.reader.buffered()
    }

    /// Feeds bytes read from the link before the handshake completes.
    /// Returns the bytes consumed and, once the peer's init is complete, the
    /// link cipher. Bytes after the init are not consumed: they are already
    /// ciphertext.
    pub fn handle_input(&mut self, data: &[u8]) -> Result<(usize, Option<AesLink>), AesLinkError> {
        let pushed = self.reader.push(data);
        let Some(body) = pushed.message else {
            return Ok((pushed.consumed, None));
        };
        let consumed = pushed.consumed;
        let init: AesLinkInit = decode_stream_message(body).map_err(AesLinkError::Read)?;
        let link = self.finish(&init)?;
        Ok((consumed, Some(link)))
    }

    /// Checks the peer's init and derives the link cipher.
    pub fn finish(&self, init: &AesLinkInit) -> Result<AesLink, AesLinkError> {
        if let Some(want) = &self.peer_key
            && init.key.as_slice() != want.as_slice()
        {
            return Err(AesLinkError::PeerKeyMismatch);
        }
        let secret = strims_crypto::aes_link_key(&self.key, &init.key)
            .map_err(AesLinkError::KeyAgreement)?;
        let read_iv: [u8; AES_LINK_IV_LEN] = init
            .iv
            .as_slice()
            .try_into()
            .map_err(|_| AesLinkError::InvalidIv(init.iv.len()))?;
        Ok(AesLink::new(&secret, &self.iv, &read_iv))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use strims_base::rng::det_rng;
    use strims_crypto::{key_from_seed, key_to_curve25519};

    fn x_key(seed: u8) -> Key {
        key_to_curve25519(&key_from_seed(&[seed; 32])).unwrap()
    }

    #[test]
    fn handshake_both_ways_and_chunking_independence() {
        let mut rng = det_rng(1);
        let (ka, kb) = (x_key(1), x_key(2));
        let kb_pub: [u8; 32] = kb.public.as_slice().try_into().unwrap();
        let mut a = AesLinkHandshake::new(ka, Some(kb_pub), &mut rng);
        let mut b = AesLinkHandshake::new(kb, None, &mut rng);
        let (mut wa, mut wb) = (Vec::new(), Vec::new());
        a.write_init(&mut wa).unwrap();
        b.write_init(&mut wb).unwrap();
        wb.extend_from_slice(b"rest");
        let (n, la) = a.handle_input(&wb).unwrap();
        assert_eq!(&wb[n..], b"rest");
        let (_, lb) = b.handle_input(&wa).unwrap();
        let (mut la, mut lb) = (la.unwrap(), lb.unwrap());

        let msg: Vec<u8> = (0..200u8).collect();
        let mut ct = msg.clone();
        la.encrypt(&mut ct);
        assert_ne!(ct, msg);
        // Decrypt in odd chunks.
        let mut pt = ct.clone();
        for c in pt.chunks_mut(7) {
            lb.decrypt(c);
        }
        assert_eq!(pt, msg);
        // Encrypting in chunks gives the same stream as one call.
        let mut l1 = AesLink::new(&[9; 32], &[1; 16], &[2; 16]);
        let mut l2 = AesLink::new(&[9; 32], &[1; 16], &[2; 16]);
        let mut x = msg.clone();
        l1.encrypt(&mut x);
        let mut y = msg.clone();
        for c in y.chunks_mut(5) {
            l2.encrypt(c);
        }
        assert_eq!(x, y);
    }

    #[test]
    fn rejects_wrong_key_and_bad_iv() {
        let mut rng = det_rng(2);
        let a = AesLinkHandshake::new(x_key(1), Some([5; 32]), &mut rng);
        let b = AesLinkHandshake::new(x_key(2), None, &mut rng);
        assert_eq!(
            a.finish(&b.init()).unwrap_err(),
            AesLinkError::PeerKeyMismatch
        );
        let mut init = a.init();
        init.iv.truncate(15);
        assert_eq!(b.finish(&init).unwrap_err(), AesLinkError::InvalidIv(15));
        let mut init = a.init();
        init.key = vec![0; 32];
        assert!(matches!(
            b.finish(&init),
            Err(AesLinkError::KeyAgreement(_))
        ));
    }
}
