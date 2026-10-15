// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Test helpers: [`pump`] shuttles bytes between two sans-IO endpoints,
//! splitting the stream at random points.

use rand_core::Rng;
use strims_base::rng::DetRng;

use crate::session::Endpoint;

/// Everything each side sent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Transcript {
    /// Bytes `a` sent to `b`.
    pub a_to_b: Vec<u8>,
    /// Bytes `b` sent to `a`.
    pub b_to_a: Vec<u8>,
    /// Number of `handle_input` calls, both directions.
    pub deliveries: usize,
}

fn chunk_len(rng: &mut DetRng, max_chunk: usize) -> usize {
    1 + (rng.next_u64() % max_chunk.max(1) as u64) as usize
}

/// Moves everything `from` has to send into `to`, polling and delivering in
/// chunks of 1..=`max_chunk` bytes. Returns whether any bytes moved.
fn transfer(
    from: &mut dyn Endpoint,
    to: &mut dyn Endpoint,
    rng: &mut DetRng,
    max_chunk: usize,
    log: &mut Vec<u8>,
    deliveries: &mut usize,
) -> bool {
    let mut buf = vec![0u8; max_chunk.max(1)];
    let start = log.len();
    loop {
        let n = chunk_len(rng, max_chunk);
        let Some(dst) = buf.get_mut(..n) else {
            break;
        };
        let Some(k) = from.poll_transmit(dst) else {
            break;
        };
        log.extend_from_slice(dst.get(..k).unwrap_or_default());
    }
    let mut rest = log.get(start..).unwrap_or_default();
    while !rest.is_empty() {
        let (chunk, tail) = rest.split_at(chunk_len(rng, max_chunk).min(rest.len()));
        // Errors surface through the endpoint's result.
        let _ = to.handle_input(chunk);
        *deliveries += 1;
        rest = tail;
    }
    log.len() > start
}

/// Runs `a` and `b` against each other until both are done or neither can
/// make progress, delivering bytes in random chunks of at most `max_chunk`.
pub fn pump(
    a: &mut dyn Endpoint,
    b: &mut dyn Endpoint,
    rng: &mut DetRng,
    max_chunk: usize,
) -> Transcript {
    let mut t = Transcript::default();
    loop {
        let ab = transfer(a, b, rng, max_chunk, &mut t.a_to_b, &mut t.deliveries);
        let ba = transfer(b, a, rng, max_chunk, &mut t.b_to_a, &mut t.deliveries);
        if (a.is_done() && b.is_done()) || !(ab || ba) {
            return t;
        }
    }
}
