// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Seeded draws.

use rand_core::Rng;
use strims_base::rng::DetRng;
use strims_base::time::Duration;

/// A uniform draw from `[0, 1)` with 53 bits of precision.
pub fn unit(rng: &mut DetRng) -> f64 {
    (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

/// True with probability `p`. Draws nothing when `p` is not positive.
pub fn chance(rng: &mut DetRng, p: f64) -> bool {
    p > 0.0 && unit(rng) < p
}

/// An exponentially distributed duration with mean `mean`. Draws nothing
/// when `mean` is not positive.
pub fn exponential(rng: &mut DetRng, mean: Duration) -> Duration {
    if mean <= Duration::ZERO {
        return Duration::ZERO;
    }
    let x = -(1.0 - unit(rng)).ln();
    Duration::from_nanos((x * mean.as_nanos() as f64) as i64)
}

/// An item drawn with probability proportional to its weight; `None` when
/// every weight is zero.
pub fn pick<T: Copy>(rng: &mut DetRng, weighted: &[(T, u32)]) -> Option<T> {
    let total: u64 = weighted.iter().map(|&(_, w)| u64::from(w)).sum();
    if total == 0 {
        return None;
    }
    let mut x = rng.next_u64() % total;
    for &(t, w) in weighted {
        let w = u64::from(w);
        if x < w {
            return Some(t);
        }
        x -= w;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use strims_base::rng::det_rng;

    #[test]
    fn exponential_has_its_mean() {
        let mut rng = det_rng(1);
        let mean = Duration::from_millis(10);
        let n = 20_000;
        let sum: i64 = (0..n).map(|_| exponential(&mut rng, mean).as_nanos()).sum();
        let got = sum as f64 / n as f64 / 1e6;
        assert!((9.6..10.4).contains(&got), "{got}");
        assert_eq!(exponential(&mut rng, Duration::ZERO), Duration::ZERO);
    }

    #[test]
    fn pick_follows_weights() {
        let mut rng = det_rng(2);
        let mut counts = [0u32; 3];
        for _ in 0..30_000 {
            let i = pick(&mut rng, &[(0usize, 1), (1, 2), (2, 0)]).unwrap_or(2);
            if let Some(c) = counts.get_mut(i) {
                *c += 1;
            }
        }
        assert_eq!(counts[2], 0);
        assert!((9_500..10_500).contains(&counts[0]), "{counts:?}");
        assert_eq!(pick::<u8>(&mut rng, &[(1, 0)]), None);
    }
}
