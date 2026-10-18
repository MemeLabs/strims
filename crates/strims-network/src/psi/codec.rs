// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The messages between [`RemotePsi`](super::RemotePsi) and
//! [`PsiServer`](super::PsiServer). Both ends are the same build, so the
//! format carries no version.
//!
//! A message is one transport message: a tag byte, the session's id as a
//! little-endian `u64`, then the tag's fields, with nothing after them.
//! Lengths and counts are little-endian.
//!
//! | tag | direction | fields |
//! |---|---|---|
//! | `START` | to the server | role `u8`, seed `[u8; 16]`, max peer keys `u32`, key count `u32`, then each key as length `u16` and bytes |
//! | `INPUT` | to the server | the peer's bytes, 1 to [`MAX_DATA`] |
//! | `CLOSE` | to the server | none |
//! | `TRANSMIT` | to the client | whether more of the step follows `u8` (0 or 1), then bytes for the peer, 1 to [`MAX_DATA`] |
//! | `OUTCOME` | to the client | kind `u8`; a shared outcome's key count `u32` and keys, or a failure's UTF-8 message |
//!
//! A step is what the server sends for one message: the session's bytes,
//! then its outcome if it has one. The step's last message is a transmit
//! that says no more follows, or the outcome.
//!
//! Decoders take any bytes and never panic: a message that is malformed,
//! has bytes left over, or breaks a bound decodes to `None`.

use strims_mpc::MAX_PEER_SET_SIZE;

use super::{PsiOutcome, PsiRole};

/// The most keys a start message or a shared outcome holds: no peer
/// accepts a larger set.
pub(super) const MAX_KEYS: usize = MAX_PEER_SET_SIZE;

/// The longest key. Network keys are 32-byte public keys.
pub(super) const MAX_KEY_LEN: usize = 64;

/// The most stream bytes in one input or transmit message. Longer runs of
/// bytes go as several messages: the stream may be split anywhere.
pub(super) const MAX_DATA: usize = 64 * 1024;

/// The longest failure message, in bytes. Longer ones are cut at a
/// character boundary.
pub(super) const MAX_ERROR_LEN: usize = 256;

/// The tag and the id.
const HEADER: usize = 1 + 8;

/// A start message's fields before its keys.
const START_FIXED: usize = HEADER + 1 + 16 + 4 + 4;

/// The largest message: a start message with [`MAX_KEYS`] keys of
/// [`MAX_KEY_LEN`] bytes.
pub(super) const MAX_MESSAGE_SIZE: usize = START_FIXED + MAX_KEYS * (2 + MAX_KEY_LEN);

const START: u8 = 0x01;
const INPUT: u8 = 0x02;
const CLOSE: u8 = 0x03;
const TRANSMIT: u8 = 0x81;
const OUTCOME: u8 = 0x82;

const SENDER: u8 = 0;
const RECEIVER: u8 = 1;

const SENT: u8 = 0;
const SHARED: u8 = 1;
const CUCKOO_FAILED: u8 = 2;
const FAILED: u8 = 3;

/// A message to the server.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Request<'a> {
    Start {
        id: u64,
        role: PsiRole,
        seed: [u8; 16],
        max_peer_keys: usize,
        keys: Vec<Vec<u8>>,
    },
    Input {
        id: u64,
        data: &'a [u8],
    },
    Close {
        id: u64,
    },
}

/// A message to the client.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Reply<'a> {
    Transmit {
        id: u64,
        /// More of the step follows.
        more: bool,
        data: &'a [u8],
    },
    Outcome {
        id: u64,
        outcome: PsiOutcome,
    },
}

fn header(tag: u8, id: u64, capacity: usize) -> Vec<u8> {
    let mut b = Vec::with_capacity(HEADER + capacity);
    b.push(tag);
    b.extend_from_slice(&id.to_le_bytes());
    b
}

/// Whether `keys` fit a message: at most [`MAX_KEYS`] of at most
/// [`MAX_KEY_LEN`] bytes each.
fn keys_fit(keys: &[Vec<u8>]) -> bool {
    keys.len() <= MAX_KEYS && keys.iter().all(|k| k.len() <= MAX_KEY_LEN)
}

fn put_keys(b: &mut Vec<u8>, keys: &[Vec<u8>]) {
    // invariant: keys_fit held, so the count fits a u32 and each length a u16
    b.extend_from_slice(&u32::try_from(keys.len()).unwrap_or(u32::MAX).to_le_bytes());
    for k in keys {
        b.extend_from_slice(&u16::try_from(k.len()).unwrap_or(u16::MAX).to_le_bytes());
        b.extend_from_slice(k);
    }
}

/// A start message, or `None` if `keys` break the bounds.
/// `max_peer_keys` is capped at [`MAX_PEER_SET_SIZE`], as the sessions
/// cap it.
pub(super) fn encode_start(
    id: u64,
    role: PsiRole,
    seed: &[u8; 16],
    max_peer_keys: usize,
    keys: &[Vec<u8>],
) -> Option<Vec<u8>> {
    if !keys_fit(keys) {
        return None;
    }
    let size: usize = keys.iter().map(|k| 2 + k.len()).sum();
    let mut b = header(START, id, START_FIXED - HEADER + size);
    b.push(match role {
        PsiRole::Sender => SENDER,
        PsiRole::Receiver => RECEIVER,
    });
    b.extend_from_slice(seed);
    let max = u32::try_from(max_peer_keys.min(MAX_PEER_SET_SIZE)).unwrap_or(u32::MAX);
    b.extend_from_slice(&max.to_le_bytes());
    put_keys(&mut b, keys);
    Some(b)
}

/// An input message. `data` must hold 1 to [`MAX_DATA`] bytes.
pub(super) fn encode_input(id: u64, data: &[u8]) -> Vec<u8> {
    let mut b = header(INPUT, id, data.len());
    b.extend_from_slice(data);
    b
}

pub(super) fn encode_close(id: u64) -> Vec<u8> {
    header(CLOSE, id, 0)
}

/// A transmit message. `data` must hold 1 to [`MAX_DATA`] bytes.
pub(super) fn encode_transmit(id: u64, more: bool, data: &[u8]) -> Vec<u8> {
    let mut b = header(TRANSMIT, id, 1 + data.len());
    b.push(u8::from(more));
    b.extend_from_slice(data);
    b
}

/// An outcome message. A shared outcome's keys are a subset of a start
/// message's, so they fit; a failure's message is cut to
/// [`MAX_ERROR_LEN`].
pub(super) fn encode_outcome(id: u64, outcome: &PsiOutcome) -> Vec<u8> {
    match outcome {
        PsiOutcome::Sent => {
            let mut b = header(OUTCOME, id, 1);
            b.push(SENT);
            b
        }
        PsiOutcome::Shared(keys) if keys_fit(keys) => {
            let size: usize = keys.iter().map(|k| 2 + k.len()).sum();
            let mut b = header(OUTCOME, id, 5 + size);
            b.push(SHARED);
            put_keys(&mut b, keys);
            b
        }
        PsiOutcome::Shared(_) => encode_outcome(
            id,
            &PsiOutcome::Failed("psi outcome exceeds the engine's bounds".into()),
        ),
        PsiOutcome::CuckooFailed => {
            let mut b = header(OUTCOME, id, 1);
            b.push(CUCKOO_FAILED);
            b
        }
        PsiOutcome::Failed(e) => {
            let e = e
                .get(..e.floor_char_boundary(MAX_ERROR_LEN))
                .unwrap_or_default();
            let mut b = header(OUTCOME, id, 1 + e.len());
            b.push(FAILED);
            b.extend_from_slice(e.as_bytes());
            b
        }
    }
}

/// Reads fields off the front of a message.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        let (head, tail) = self.0.split_at_checked(n)?;
        self.0 = tail;
        Some(head)
    }

    fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.bytes(N)?.try_into().ok()
    }

    fn u8(&mut self) -> Option<u8> {
        Some(u8::from_le_bytes(self.array()?))
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.array()?))
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.array()?))
    }

    /// The rest, which must hold 1 to [`MAX_DATA`] bytes.
    fn data(self) -> Option<&'a [u8]> {
        (1..=MAX_DATA).contains(&self.0.len()).then_some(self.0)
    }

    fn keys(&mut self) -> Option<Vec<Vec<u8>>> {
        let count = usize::try_from(self.u32()?).ok()?;
        if count > MAX_KEYS {
            return None;
        }
        // each key takes at least its length's two bytes
        let mut keys = Vec::with_capacity(count.min(self.0.len() / 2));
        for _ in 0..count {
            let len = usize::from(self.u16()?);
            if len > MAX_KEY_LEN {
                return None;
            }
            keys.push(self.bytes(len)?.to_vec());
        }
        Some(keys)
    }

    fn end(self) -> Option<()> {
        self.0.is_empty().then_some(())
    }
}

/// The tag and id of a message within [`MAX_MESSAGE_SIZE`].
fn open(msg: &[u8]) -> Option<(u8, u64, Reader<'_>)> {
    if msg.len() > MAX_MESSAGE_SIZE {
        return None;
    }
    let mut r = Reader(msg);
    let tag = r.u8()?;
    let id = r.u64()?;
    Some((tag, id, r))
}

pub(super) fn decode_request(msg: &[u8]) -> Option<Request<'_>> {
    let (tag, id, mut r) = open(msg)?;
    match tag {
        START => {
            let role = match r.u8()? {
                SENDER => PsiRole::Sender,
                RECEIVER => PsiRole::Receiver,
                _ => return None,
            };
            let seed = r.array()?;
            let max_peer_keys = usize::try_from(r.u32()?).ok()?;
            if max_peer_keys > MAX_PEER_SET_SIZE {
                return None;
            }
            let keys = r.keys()?;
            r.end()?;
            Some(Request::Start {
                id,
                role,
                seed,
                max_peer_keys,
                keys,
            })
        }
        INPUT => Some(Request::Input {
            id,
            data: r.data()?,
        }),
        CLOSE => {
            r.end()?;
            Some(Request::Close { id })
        }
        _ => None,
    }
}

pub(super) fn decode_reply(msg: &[u8]) -> Option<Reply<'_>> {
    let (tag, id, mut r) = open(msg)?;
    match tag {
        TRANSMIT => {
            let more = match r.u8()? {
                0 => false,
                1 => true,
                _ => return None,
            };
            Some(Reply::Transmit {
                id,
                more,
                data: r.data()?,
            })
        }
        OUTCOME => {
            let outcome = match r.u8()? {
                SENT => PsiOutcome::Sent,
                SHARED => PsiOutcome::Shared(r.keys()?),
                CUCKOO_FAILED => PsiOutcome::CuckooFailed,
                FAILED => {
                    if r.0.len() > MAX_ERROR_LEN {
                        return None;
                    }
                    let e = std::str::from_utf8(r.0).ok()?.to_owned();
                    r.0 = &[];
                    PsiOutcome::Failed(e)
                }
                _ => return None,
            };
            r.end()?;
            Some(Reply::Outcome { id, outcome })
        }
        _ => None,
    }
}
