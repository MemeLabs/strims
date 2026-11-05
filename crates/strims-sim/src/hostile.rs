// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Hostile hosts ([`HostClass::Hostile`](crate::HostClass)): what each
//! kind does, as a rewrite of the ppspp frames it sends
//! (`NodeConfig::hostile_egress`). Each takes the stream like an honest
//! viewer; what it sends back differs.
//!
//! - `free-rider`: gives nothing back. Its frames carry no DATA, INTEGRITY
//!   or HAVE, so no peer asks it for a chunk or grafts onto it.
//! - `liar`: announces what it has (its HAVEs go out) and serves none of
//!   it (no DATA or INTEGRITY): peers ask it and graft onto it in vain.

use std::sync::Arc;

use strims_node::config::HostileEgress;
use strims_ppspp::SwarmOptions;
use strims_ppspp::codec::{
    ChannelWriter, FrameReader, Message, MessageWriter, Reader, ReaderConfig,
};

/// The kinds a host can be ([`egress`]).
pub const KINDS: &[&str] = &["free-rider", "liar"];

/// The frame rewrite of a host of `kind` in swarms of `options`; `None`
/// for a kind it doesn't know.
pub fn egress(kind: &str, options: &SwarmOptions) -> Option<HostileEgress> {
    let drop_have = match kind {
        "free-rider" => true,
        "liar" => false,
        _ => return None,
    };
    let o = &options.integrity;
    let cfg = ReaderConfig {
        chunk_size: options.chunk_size,
        integrity_hash_size: o.merkle_hash_tree_function.hash_size().unwrap_or(0),
        integrity_signature_size: o.live_signature_algorithm.signature_size().unwrap_or(0),
    };
    Some(HostileEgress(Arc::new(move |buf: &mut [u8], n: usize| {
        rewrite(buf, n, cfg, drop_have)
    })))
}

/// The frame in `buf[..n]` without its DATA and INTEGRITY messages, and
/// its HAVEs with `drop_have`. A frame it can't read (a handshake's)
/// goes as it was.
fn rewrite(buf: &mut [u8], n: usize, cfg: ReaderConfig, drop_have: bool) -> usize {
    let Some(frame) = buf.get(..n).map(<[u8]>::to_vec) else {
        return n;
    };
    let mut off = 0;
    for c in FrameReader::new(&frame) {
        let Ok((h, payload)) = c else {
            return n;
        };
        let Some(out) = buf.get_mut(off..) else {
            return n;
        };
        let mut cw = ChannelWriter::new(out, h.channel);
        for m in Reader::new(cfg, payload) {
            // A handshake can carry options the reader doesn't know.
            let Ok(m) = m else {
                return n;
            };
            match m {
                Message::Data(_) | Message::Integrity(_) | Message::SignedIntegrity(_) => {}
                Message::Have(_) if drop_have => {}
                m => {
                    if cw.write(&m).is_err() {
                        break;
                    }
                }
            }
        }
        off += cw.finish();
    }
    off
}
