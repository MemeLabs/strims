// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Seeded randomness.
//!
//! Production drivers seed from OS entropy; simulations seed from the test
//! seed. ChaCha12 is a CSPRNG, so key material may also be drawn from it.

use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;

/// The rng type owned by sans-IO state machines.
pub type DetRng = ChaCha12Rng;

/// Creates a rng from a 64 bit seed.
pub fn det_rng(seed: u64) -> DetRng {
    DetRng::seed_from_u64(seed)
}

/// Derives an independent child rng, e.g. one per peer or per swarm, so
/// adding a consumer doesn't perturb the streams seen by existing ones.
pub fn fork(rng: &mut DetRng) -> DetRng {
    let mut seed = <DetRng as SeedableRng>::Seed::default();
    rand_core::Rng::fill_bytes(rng, &mut seed);
    DetRng::from_seed(seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let mut a = det_rng(7);
        let mut b = det_rng(7);
        assert_eq!(
            rand_core::Rng::next_u64(&mut a),
            rand_core::Rng::next_u64(&mut b)
        );
        let mut fa = fork(&mut a);
        let mut fb = fork(&mut b);
        assert_eq!(
            rand_core::Rng::next_u64(&mut fa),
            rand_core::Rng::next_u64(&mut fb)
        );
    }
}
