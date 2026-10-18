// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The RTMP handshake: C0/S0 (the version byte), C1/S1 and C2/S2 (1536
//! bytes each).
//!
//! A packet `P` of the digest handshake carries an HMAC-SHA256 digest at
//! an offset that four of its bytes select: for a base `b`, the offset is
//! `b + 4 + (P[b] + P[b+1] + P[b+2] + P[b+3]) % 728`, and the digest
//! covers `P` without those 32 bytes. A C1 digest is checked at base 772,
//! then 8, under the client's partial key; S1's digest sits at base 8
//! under the server's partial key. The response packet (S2 or C2) is 1504
//! random bytes and an HMAC over them whose key is the HMAC of the peer's
//! digest under the sender's full key.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use strims_base::rng::DetRng;

use super::RtmpError;

/// The version byte of C0 and S0.
pub(crate) const VERSION: u8 = 3;
/// The length of C1, S1, C2 and S2.
pub(crate) const PACKET_LEN: usize = 1536;
const DIGEST_LEN: usize = 32;
/// Digest offsets span this many positions past their base.
const DIGEST_SPAN: usize = 728;
/// The bases a C1 digest may use, in the order they are checked.
const C1_DIGEST_BASES: [usize; 2] = [772, 8];
/// The base of the digest a server writes into S1.
const S1_DIGEST_BASE: usize = 8;
/// The bytes of a response packet its digest covers.
const RESPONSE_SIGNED_LEN: usize = PACKET_LEN - DIGEST_LEN;
/// The version field of a digest S1.
const SERVER_VERSION: [u8; 4] = [0x0d, 0x0e, 0x0a, 0x0d];

/// The Flash Player key: its first 30 bytes are the partial key that
/// signs C1, all 62 sign C2.
pub(crate) const CLIENT_FULL_KEY: [u8; 62] = *b"Genuine Adobe Flash Player 001\
    \xf0\xee\xc2\x4a\x80\x68\xbe\xe8\x2e\x00\xd0\xd1\x02\x9e\x7e\x57\
    \x6e\xec\x5d\x2d\x29\x80\x6f\xab\x93\xb8\xe6\x36\xcf\xeb\x31\xae";
/// The Flash Media Server key: its first 36 bytes are the partial key
/// that signs S1, all 68 sign S2.
pub(crate) const SERVER_FULL_KEY: [u8; 68] = *b"Genuine Adobe Flash Media Server 001\
    \xf0\xee\xc2\x4a\x80\x68\xbe\xe8\x2e\x00\xd0\xd1\x02\x9e\x7e\x57\
    \x6e\xec\x5d\x2d\x29\x80\x6f\xab\x93\xb8\xe6\x36\xcf\xeb\x31\xae";
pub(crate) const CLIENT_PARTIAL_KEY: &[u8] = CLIENT_FULL_KEY.split_at(30).0;
pub(crate) const SERVER_PARTIAL_KEY: &[u8] = SERVER_FULL_KEY.split_at(36).0;

type Packet = [u8; PACKET_LEN];

fn hmac_sha256(key: &[u8], parts: &[&[u8]]) -> [u8; DIGEST_LEN] {
    // invariant: HMAC takes keys of any length.
    #[allow(clippy::expect_used)]
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC takes keys of any length");
    for p in parts {
        mac.update(p);
    }
    mac.finalize().into_bytes().into()
}

/// The digest offset the four bytes at `base` select.
fn digest_offset(packet: &Packet, base: usize) -> usize {
    let sum: usize = packet
        .iter()
        .skip(base)
        .take(4)
        .map(|&b| usize::from(b))
        .sum();
    base + 4 + sum % DIGEST_SPAN
}

/// The digest of `packet` without the 32 bytes at `offset`.
fn packet_digest(key: &[u8], packet: &Packet, offset: usize) -> [u8; DIGEST_LEN] {
    let (head, rest) = packet.split_at(offset.min(PACKET_LEN));
    let tail = rest.get(DIGEST_LEN..).unwrap_or_default();
    hmac_sha256(key, &[head, tail])
}

/// The digest of a C1 (or S1, under the server key) that verifies under
/// `key` at base 772 or 8.
pub(crate) fn find_digest(packet: &Packet, key: &[u8]) -> Option<[u8; DIGEST_LEN]> {
    C1_DIGEST_BASES.iter().find_map(|&base| {
        let offset = digest_offset(packet, base);
        let digest = packet_digest(key, packet, offset);
        let carried = packet.get(offset..offset + DIGEST_LEN)?;
        (carried == digest).then_some(digest)
    })
}

/// Signs `packet` with a digest at the offset `base` selects; returns it.
fn write_digest(packet: &mut Packet, base: usize, key: &[u8]) -> [u8; DIGEST_LEN] {
    let offset = digest_offset(packet, base);
    let digest = packet_digest(key, packet, offset);
    if let Some(d) = packet.get_mut(offset..offset + DIGEST_LEN) {
        d.copy_from_slice(&digest);
    }
    digest
}

/// The digest that ends a response packet: an HMAC over its first 1504
/// bytes, keyed by the HMAC of `peer_digest` under `full_key`.
pub(crate) fn response_digest(
    full_key: &[u8],
    peer_digest: &[u8; DIGEST_LEN],
    packet: &Packet,
) -> [u8; DIGEST_LEN] {
    let key = hmac_sha256(full_key, &[peer_digest]);
    let (signed, _) = packet.split_at(RESPONSE_SIGNED_LEN);
    hmac_sha256(&key, &[signed])
}

/// Whether a response packet ends with the digest `response_digest`
/// gives.
#[cfg(any(test, feature = "test-support"))]
pub(crate) fn response_valid(
    full_key: &[u8],
    peer_digest: &[u8; DIGEST_LEN],
    packet: &Packet,
) -> bool {
    packet.get(RESPONSE_SIGNED_LEN..) == Some(&response_digest(full_key, peer_digest, packet)[..])
}

/// A response packet signed over `peer_digest`.
fn signed_response(full_key: &[u8], peer_digest: &[u8; DIGEST_LEN], rng: &mut DetRng) -> Packet {
    let mut p = [0; PACKET_LEN];
    rand_core::Rng::fill_bytes(rng, &mut p);
    let digest = response_digest(full_key, peer_digest, &p);
    if let Some(d) = p.get_mut(RESPONSE_SIGNED_LEN..) {
        d.copy_from_slice(&digest);
    }
    p
}

fn fill(buf: &mut Vec<u8>, want: usize, input: &[u8]) -> usize {
    let n = want.saturating_sub(buf.len()).min(input.len());
    buf.extend_from_slice(input.get(..n).unwrap_or_default());
    n
}

enum ServerState {
    C0,
    C1,
    /// C2 bytes still to skip.
    C2(usize),
    Done,
}

/// The server's side: reads C0 and C1, replies S0 S1 S2, skips C2.
pub(crate) struct ServerHandshake {
    state: ServerState,
    /// C1 as it arrives, at most [`PACKET_LEN`] bytes.
    c1: Vec<u8>,
    /// Whether C1 carried a valid digest.
    digest: bool,
}

impl ServerHandshake {
    pub(crate) fn new() -> Self {
        ServerHandshake {
            state: ServerState::C0,
            c1: Vec::new(),
            digest: false,
        }
    }

    pub(crate) fn is_done(&self) -> bool {
        matches!(self.state, ServerState::Done)
    }

    /// Whether the handshake took the digest scheme.
    pub(crate) fn digest(&self) -> bool {
        self.digest
    }

    /// Reads handshake bytes; returns the bytes consumed and, once C1 is
    /// complete, S0 S1 S2.
    pub(crate) fn read(
        &mut self,
        input: &[u8],
        rng: &mut DetRng,
    ) -> Result<(usize, Option<Vec<u8>>), RtmpError> {
        let mut pos = 0;
        let mut reply = None;
        loop {
            let rest = input.get(pos..).unwrap_or_default();
            match self.state {
                ServerState::C0 => {
                    let Some(&version) = rest.first() else { break };
                    if version != VERSION {
                        return Err(RtmpError::Version(version));
                    }
                    pos += 1;
                    self.state = ServerState::C1;
                }
                ServerState::C1 => {
                    pos += fill(&mut self.c1, PACKET_LEN, rest);
                    let Ok(c1) = <&Packet>::try_from(self.c1.as_slice()) else {
                        break;
                    };
                    let (r, digest) = server_reply(c1, rng);
                    reply = Some(r);
                    self.digest = digest;
                    self.c1 = Vec::new();
                    self.state = ServerState::C2(PACKET_LEN);
                }
                ServerState::C2(left) => {
                    if rest.is_empty() {
                        break;
                    }
                    let n = left.min(rest.len());
                    pos += n;
                    self.state = match left - n {
                        0 => ServerState::Done,
                        left => ServerState::C2(left),
                    };
                }
                ServerState::Done => break,
            }
        }
        Ok((pos, reply))
    }
}

/// S0 S1 S2 for `c1`, and whether they take the digest scheme.
fn server_reply(c1: &Packet, rng: &mut DetRng) -> (Vec<u8>, bool) {
    let mut s1 = [0; PACKET_LEN];
    if let Some(random) = s1.get_mut(8..) {
        rand_core::Rng::fill_bytes(rng, random);
    }
    let c1_digest = find_digest(c1, CLIENT_PARTIAL_KEY);
    let s2 = match &c1_digest {
        Some(c1_digest) => {
            if let Some(v) = s1.get_mut(4..8) {
                v.copy_from_slice(&SERVER_VERSION);
            }
            write_digest(&mut s1, S1_DIGEST_BASE, SERVER_PARTIAL_KEY);
            signed_response(&SERVER_FULL_KEY, c1_digest, rng)
        }
        None => *c1,
    };
    let mut out = Vec::with_capacity(1 + 2 * PACKET_LEN);
    out.push(VERSION);
    out.extend_from_slice(&s1);
    out.extend_from_slice(&s2);
    (out, c1_digest.is_some())
}

/// The client's side: sends C0 C1, reads S0 S1 S2, replies C2.
#[cfg(any(test, feature = "test-support"))]
pub(crate) struct ClientHandshake {
    /// The digest C1 carries, in the digest scheme.
    c1_digest: Option<[u8; DIGEST_LEN]>,
    /// S0 S1 S2 as they arrive.
    s: Vec<u8>,
    done: bool,
    server_digest: Option<bool>,
}

#[cfg(any(test, feature = "test-support"))]
impl ClientHandshake {
    /// The version of a digest C1.
    const VERSION: [u8; 4] = [9, 0, 124, 2];

    /// Starts a handshake; returns it and C0 C1. A digest C1 carries its
    /// digest at base 8.
    pub(crate) fn new(digest: bool, rng: &mut DetRng) -> (Self, Vec<u8>) {
        let mut c1 = [0; PACKET_LEN];
        if let Some(random) = c1.get_mut(8..) {
            rand_core::Rng::fill_bytes(rng, random);
        }
        let c1_digest = digest.then(|| {
            if let Some(v) = c1.get_mut(4..8) {
                v.copy_from_slice(&Self::VERSION);
            }
            write_digest(&mut c1, 8, CLIENT_PARTIAL_KEY)
        });
        let mut hello = Vec::with_capacity(1 + PACKET_LEN);
        hello.push(VERSION);
        hello.extend_from_slice(c1.as_slice());
        let h = ClientHandshake {
            c1_digest,
            s: Vec::new(),
            done: false,
            server_digest: None,
        };
        (h, hello)
    }

    pub(crate) fn is_done(&self) -> bool {
        self.done
    }

    /// Whether S1 and S2 carried valid digests; `None` before S2 or for a
    /// plain C1.
    pub(crate) fn server_digest(&self) -> Option<bool> {
        self.server_digest
    }

    /// Reads S0 S1 S2; returns the bytes consumed and, once complete, C2.
    pub(crate) fn read(
        &mut self,
        input: &[u8],
        rng: &mut DetRng,
    ) -> Result<(usize, Option<Vec<u8>>), RtmpError> {
        if self.done {
            return Ok((0, None));
        }
        let n = fill(&mut self.s, 1 + 2 * PACKET_LEN, input);
        let Some((&s0, rest)) = self.s.split_first() else {
            return Ok((n, None));
        };
        if s0 != VERSION {
            return Err(RtmpError::Version(s0));
        }
        let (Ok(s1), Ok(s2)) = (
            <&Packet>::try_from(rest.get(..PACKET_LEN).unwrap_or_default()),
            <&Packet>::try_from(rest.get(PACKET_LEN..).unwrap_or_default()),
        ) else {
            return Ok((n, None));
        };
        self.done = true;
        let s1_digest = find_digest(s1, SERVER_PARTIAL_KEY);
        if let Some(c1_digest) = &self.c1_digest {
            self.server_digest =
                Some(s1_digest.is_some() && response_valid(&SERVER_FULL_KEY, c1_digest, s2));
        }
        let c2 = match (&self.c1_digest, s1_digest) {
            (Some(_), Some(s1_digest)) => signed_response(&CLIENT_FULL_KEY, &s1_digest, rng),
            _ => *s1,
        };
        Ok((n, Some(c2.to_vec())))
    }
}

#[cfg(test)]
#[path = "handshake_test.rs"]
mod tests;
