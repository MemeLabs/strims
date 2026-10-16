// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The `strims.rpc.Call` envelope on the wire.
//!
//! A request carries the method's full proto name and a `parent_id` of 0.
//! Every answer is a new Call with method [`CALLBACK`] and the request's id
//! as `parent_id`; its kind says what the argument is. A cancel is method
//! [`CANCEL`] with the call's id as `parent_id`, recognised by method alone
//! since peers send it with kind `CANCEL` or `DEFAULT`.

use prost::Message;
use strims_proto::rpc::call::Kind;

/// The method of every answer.
pub const CALLBACK: &str = "CALLBACK";

/// The method of a cancel.
pub const CANCEL: &str = "CANCEL";

/// The longest uvarint encoding of a `u64`.
pub const MAX_VARINT_LEN: usize = 10;

/// How Calls are delimited on a transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Framing {
    /// A byte stream of Calls, each prefixed with its length as a uvarint.
    /// Message transports (a worker's message port, WebSocket messages)
    /// use this too, with each prefixed Call sent as exactly one message.
    Delimited,
    /// One unprefixed Call per transport message.
    Message,
}

/// The encoded length of `v` as a uvarint.
pub fn uvarint_len(v: u64) -> usize {
    // 7 bits per byte; 0 still takes a byte
    let bits = 64 - (v | 1).leading_zeros() as usize;
    bits.div_ceil(7)
}

/// Appends `v` as a uvarint.
pub fn put_uvarint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

/// A uvarint that is longer than [`MAX_VARINT_LEN`] or overflows 64 bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("malformed uvarint")]
pub struct MalformedVarint;

/// Decodes the uvarint at the start of `buf` into its value and length, or
/// `None` if `buf` ends inside it.
pub fn get_uvarint(buf: &[u8]) -> Result<Option<(u64, usize)>, MalformedVarint> {
    let mut v: u64 = 0;
    for (i, &b) in buf.iter().take(MAX_VARINT_LEN).enumerate() {
        if i == MAX_VARINT_LEN - 1 && b > 1 {
            return Err(MalformedVarint);
        }
        v |= u64::from(b & 0x7f) << (7 * i);
        if b < 0x80 {
            return Ok(Some((v, i + 1)));
        }
    }
    if buf.len() >= MAX_VARINT_LEN {
        Err(MalformedVarint)
    } else {
        Ok(None)
    }
}

fn put_key(out: &mut Vec<u8>, field: u32, wire_type: u8) {
    put_uvarint(out, (u64::from(field) << 3) | u64::from(wire_type));
}

const VARINT: u8 = 0;
const LEN: u8 = 2;

/// The length of a field whose value (with its length prefix, if any)
/// takes `len` bytes. The keys of fields 1 to 6 take one byte.
fn field_len(len: usize) -> usize {
    1 + len
}

/// The encoded length of a Call with these fields, without a prefix. Zero
/// values are omitted, and no headers are written.
pub fn call_len(id: u64, parent_id: u64, method: &str, kind: Kind, arg_len: usize) -> usize {
    let mut n = 0;
    if id != 0 {
        n += field_len(uvarint_len(id));
    }
    if parent_id != 0 {
        n += field_len(uvarint_len(parent_id));
    }
    if !method.is_empty() {
        n += field_len(uvarint_len(method.len() as u64) + method.len());
    }
    if kind != Kind::CallKindDefault {
        n += field_len(uvarint_len(kind as u64));
    }
    if arg_len != 0 {
        n += field_len(uvarint_len(arg_len as u64) + arg_len);
    }
    n
}

/// Appends a Call carrying `arg` to `out`, prefixed with its length under
/// [`Framing::Delimited`], and returns the Call's length (without the
/// prefix).
///
/// The bytes are those of `strims.rpc.Call` with the same fields and no
/// headers, written in one pass without copying the argument.
pub fn encode_call(
    out: &mut Vec<u8>,
    framing: Framing,
    id: u64,
    parent_id: u64,
    method: &str,
    kind: Kind,
    arg: &impl Message,
) -> usize {
    let arg_len = arg.encoded_len();
    let len = call_len(id, parent_id, method, kind, arg_len);
    if framing == Framing::Delimited {
        out.reserve(uvarint_len(len as u64) + len);
        put_uvarint(out, len as u64);
    } else {
        out.reserve(len);
    }
    if id != 0 {
        put_key(out, 1, VARINT);
        put_uvarint(out, id);
    }
    if parent_id != 0 {
        put_key(out, 2, VARINT);
        put_uvarint(out, parent_id);
    }
    if !method.is_empty() {
        put_key(out, 3, LEN);
        put_uvarint(out, method.len() as u64);
        out.extend_from_slice(method.as_bytes());
    }
    if kind != Kind::CallKindDefault {
        put_key(out, 4, VARINT);
        put_uvarint(out, kind as u64);
    }
    if arg_len != 0 {
        put_key(out, 5, LEN);
        put_uvarint(out, arg_len as u64);
        arg.encode_raw(out);
    }
    len
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use strims_proto::debug::v1::PProfRequest;
    use strims_proto::rpc::{Call, Close, Error};

    /// `encode_call` against prost's encoding of the same Call, in both
    /// framings.
    fn check(id: u64, parent_id: u64, method: &str, kind: Kind, arg: &impl Message) {
        let want = Call {
            id,
            parent_id,
            method: method.to_owned(),
            kind: kind as i32,
            argument: arg.encode_to_vec(),
            headers: Default::default(),
        }
        .encode_to_vec();

        let mut out = Vec::new();
        let len = encode_call(&mut out, Framing::Message, id, parent_id, method, kind, arg);
        assert_eq!(hex::encode(&out), hex::encode(&want));
        assert_eq!(len, want.len());
        assert_eq!(
            call_len(id, parent_id, method, kind, arg.encoded_len()),
            want.len()
        );

        let mut out = Vec::new();
        encode_call(
            &mut out,
            Framing::Delimited,
            id,
            parent_id,
            method,
            kind,
            arg,
        );
        let mut prefixed = Vec::new();
        put_uvarint(&mut prefixed, want.len() as u64);
        prefixed.extend_from_slice(&want);
        assert_eq!(out, prefixed);
    }

    #[test]
    fn calls_encode_as_prost_does() {
        let req = PProfRequest {
            name: "heap".into(),
            debug: true,
            gc: false,
        };
        check(
            1,
            0,
            "strims.debug.v1.Debug.PProf",
            Kind::CallKindDefault,
            &req,
        );
        let err = Error {
            message: "boom".into(),
            code: -3,
        };
        check(u64::MAX, u64::MAX - 1, CALLBACK, Kind::CallKindError, &err);
        check(5, 1, CALLBACK, Kind::CallKindClose, &Close {});
        check(7, 1, CANCEL, Kind::CallKindCancel, &Close {});
        check(0, 0, "", Kind::CallKindDefault, &Close {});
        let big = PProfRequest {
            name: "x".repeat(300),
            ..Default::default()
        };
        check(128, 16_384, &"m".repeat(200), Kind::CallKindUndefined, &big);
    }

    #[test]
    fn uvarints_round_trip_at_every_length() {
        let mut v = 0u64;
        loop {
            for x in [v.saturating_sub(1), v, v.saturating_add(1)] {
                let mut out = Vec::new();
                put_uvarint(&mut out, x);
                assert_eq!(out.len(), uvarint_len(x));
                assert_eq!(get_uvarint(&out), Ok(Some((x, out.len()))));
                for cut in 0..out.len() {
                    assert_eq!(get_uvarint(&out[..cut]), Ok(None));
                }
            }
            if v == u64::MAX {
                break;
            }
            v = (v << 7) | 0x7f;
        }
    }

    #[test]
    fn overlong_or_overflowing_uvarints_are_malformed() {
        assert_eq!(get_uvarint(&[0x80; 10]), Err(MalformedVarint));
        assert_eq!(get_uvarint(&[0x80; 11]), Err(MalformedVarint));
        // the tenth byte may carry only the 64th bit
        let mut over = [0xff; 10];
        over[9] = 0x02;
        assert_eq!(get_uvarint(&over), Err(MalformedVarint));
        over[9] = 0x01;
        assert_eq!(get_uvarint(&over), Ok(Some((u64::MAX, 10))));
    }
}
