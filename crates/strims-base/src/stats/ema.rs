// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/// Exponential moving average with bias correction for the first samples.
///
/// `alpha` is the weight of each new sample in [`Ema::update`]. The raw
/// average starts at 0, so after `n` updates the weights of the samples sum
/// to `1 - (1 - alpha)^n`; [`Ema::value`] divides by that sum, which makes
/// the average of `n` equal samples exactly that sample. ppspp and ledbat
/// use it for RTT mean and variance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ema {
    value: f64,
    alpha: f64,
    weight: f64,
}

impl Ema {
    /// An empty average.
    pub fn new(alpha: f64) -> Ema {
        Ema {
            value: 0.0,
            alpha,
            weight: 1.0,
        }
    }

    /// The bias-corrected average, or 0 before the first sample.
    pub fn value(&self) -> f64 {
        if self.weight == 1.0 {
            return 0.0;
        }
        self.value / (1.0 - self.weight)
    }

    /// Replaces the average with `v`, as if fully warmed up.
    pub fn set(&mut self, v: f64) {
        self.value = v;
        self.weight = 0.0;
    }

    /// True before the first sample.
    pub fn is_empty(&self) -> bool {
        self.weight == 1.0
    }

    /// Adds a sample.
    pub fn update(&mut self, v: f64) {
        self.value = self.alpha * v + (1.0 - self.alpha) * self.value;
        // `weight` is `(1 - alpha)^n`, the share of the zero start.
        self.weight *= 1.0 - self.alpha;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_until_first_sample() {
        let mut a = Ema::new(0.125);
        assert!(a.is_empty());
        assert_eq!(a.value(), 0.0);
        a.update(8.0);
        assert!(!a.is_empty());
    }

    #[test]
    fn update_applies_bias_corrected_formula() {
        let alpha = 0.25;
        let mut a = Ema::new(alpha);
        let (mut value, mut weight) = (0.0f64, 1.0f64);
        for v in [10.0, 20.0, 15.0, 40.0, 5.0] {
            a.update(v);
            value = alpha * v + (1.0 - alpha) * value;
            weight *= 1.0 - alpha;
            assert_eq!(a.value(), value / (1.0 - weight));
        }
    }

    #[test]
    fn bias_correction_matches_the_closed_form() {
        // Two samples x1, x2: (alpha x2 + alpha (1 - alpha) x1) / (1 - (1 -
        // alpha)^2). With alpha 1/4, 10 and 20: 6.875 / 0.4375 = 110 / 7.
        let mut a = Ema::new(0.25);
        a.update(10.0);
        assert_eq!(a.value(), 10.0);
        a.update(20.0);
        assert!((a.value() - 110.0 / 7.0).abs() < 1e-12, "{}", a.value());
        // Equal samples average to that sample from the first update on.
        let mut b = Ema::new(0.125);
        for n in 1..=50 {
            b.update(42.0);
            assert!((b.value() - 42.0).abs() < 1e-9, "{n}: {}", b.value());
        }
    }

    #[test]
    fn value_is_bit_exact() {
        let mut a = Ema::new(0.125);
        for v in [10.0, 30.0, 25.0, 7.0] {
            a.update(v);
        }
        assert_eq!(a.value(), 17.68377581120944);
    }

    /// The ledbat and scheduler pattern: `set` on the first RTT sample, then
    /// `update`.
    #[test]
    fn set_then_update() {
        let mut a = Ema::new(0.125);
        a.set(100.0);
        assert!(!a.is_empty());
        assert_eq!(a.value(), 100.0);
        a.update(200.0);
        assert_eq!(a.value(), 0.125 * 200.0 + 0.875 * 100.0);
    }
}
