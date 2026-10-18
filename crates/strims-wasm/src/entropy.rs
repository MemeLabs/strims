// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Seeds from the browser's CSPRNG, `crypto.getRandomValues`.
//!
//! Everything else draws from `DetRng`s forked from a seed, so this is the
//! crate's only source of entropy.

use crate::js;

/// 32 bytes from `crypto.getRandomValues`, for `DetRng::from_seed`. Fails
/// with the browser's error where the global has no `crypto`.
pub fn seed() -> Result<[u8; 32], String> {
    let crypto = js::scope()
        .crypto()
        .ok_or_else(|| "crypto.getRandomValues is unavailable".to_owned())?;
    let mut seed = [0u8; 32];
    crypto
        .get_random_values_with_u8_array(&mut seed)
        .map_err(|e| js::describe(&e))?;
    Ok(seed)
}
