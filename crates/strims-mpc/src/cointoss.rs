// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Commit-then-reveal coin tossing.
//!
//! The sender commits to each seed with the first block of `AesRng(seed)`,
//! the receiver answers with its own seeds, then the sender opens. Both
//! sides output `own ⊕ peer`.

use crate::Error;
use crate::aes_rng::AesRng;
use crate::block::{Block, xor_block};
use crate::channel::Channel;

/// The committing side.
pub(crate) async fn cointoss_send(
    conn: &mut Channel,
    seeds: &[Block],
) -> Result<Vec<Block>, Error> {
    for seed in seeds {
        conn.write(&AesRng::new(seed).block());
    }
    conn.flush();

    let mut out = Vec::with_capacity(seeds.len());
    for seed in seeds {
        let s = conn.read_block().await?;
        out.push(xor_block(seed, &s));
    }

    for seed in seeds {
        conn.write(seed);
    }
    conn.flush();
    Ok(out)
}

/// The verifying side: fails with [`Error::CommitmentCheckFailed`] if the
/// peer opens a seed that doesn't match its commitment.
pub(crate) async fn cointoss_receive(
    conn: &mut Channel,
    seeds: &[Block],
) -> Result<Vec<Block>, Error> {
    let mut commitments = Vec::with_capacity(seeds.len());
    for _ in seeds {
        commitments.push(conn.read_block().await?);
    }

    for seed in seeds {
        conn.write(seed);
    }
    conn.flush();

    let mut out = Vec::with_capacity(seeds.len());
    for (seed, commitment) in seeds.iter().zip(&commitments) {
        let s = conn.read_block().await?;
        if AesRng::new(&s).block() != *commitment {
            return Err(Error::CommitmentCheckFailed);
        }
        out.push(xor_block(seed, &s));
    }
    Ok(out)
}
