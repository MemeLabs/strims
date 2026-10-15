// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/// Welford's online algorithm for mean and variance.
///
/// See <https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford's_online_algorithm>.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Welford {
    count: f64,
    mean: f64,
    m2: f64,
}

impl Welford {
    pub fn new() -> Welford {
        Welford::default()
    }

    pub fn reset(&mut self) {
        *self = Welford::default();
    }

    pub fn update(&mut self, v: f64) {
        self.count += 1.0;
        let d = v - self.mean;
        self.mean += d / self.count;
        let d2 = v - self.mean;
        self.m2 += d * d2;
    }

    /// `(mean, variance, sample_variance)`; both variances are 0 below two
    /// samples.
    pub fn value(&self) -> (f64, f64, f64) {
        if self.count < 2.0 {
            return (self.mean, 0.0, 0.0);
        }
        (
            self.mean,
            self.m2 / self.count,
            self.m2 / (self.count - 1.0),
        )
    }

    pub fn count(&self) -> f64 {
        self.count
    }

    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// The sample variance: NaN for one sample and -0 for none.
    pub fn variance(&self) -> f64 {
        self.m2 / (self.count - 1.0)
    }

    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Combines partial results as if every sample had gone into one.
    pub fn merge(ws: &[Welford]) -> Welford {
        match ws {
            [] => Welford::default(),
            [w] => *w,
            [a, b] => {
                if a.count == 0.0 {
                    return *b;
                }
                if b.count == 0.0 {
                    return *a;
                }
                let count = a.count + b.count;
                let delta = b.mean - a.mean;
                Welford {
                    count,
                    mean: (a.mean * a.count + b.mean * b.count) / count,
                    m2: a.m2 + b.m2 + delta * delta * a.count * b.count / count,
                }
            }
            _ => {
                let (l, r) = ws.split_at(ws.len() >> 1);
                Welford::merge(&[Welford::merge(l), Welford::merge(r)])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::det_rng;
    use rand_core::Rng;

    const E: f64 = 1e-10;

    /// Two-pass mean and sample variance.
    fn mean_variance(vs: &[f64]) -> (f64, f64) {
        let n = vs.len() as f64;
        let mean = vs.iter().sum::<f64>() / n;
        let ss = vs.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>();
        (mean, ss / (n - 1.0))
    }

    #[test]
    fn matches_two_pass_mean_and_variance() {
        let mut rng = det_rng(1);
        let mut w = Welford::new();
        let vs: Vec<f64> = (0..1000)
            .map(|_| 100.0 + (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64 * 50.0)
            .collect();
        for &v in &vs {
            w.update(v);
        }

        let (mean, variance) = mean_variance(&vs);
        assert!((w.mean() - mean).abs() <= E, "mean");
        assert!((w.variance() - variance).abs() <= E, "variance");
    }

    #[test]
    fn merge_matches_two_pass() {
        let n = 1000;
        let mut ws = [Welford::new(); 4];
        let mut vs = vec![];
        for (i, w) in ws.iter_mut().enumerate() {
            for j in 0..n / 4 {
                let v = ((i + 1) * j) as f64;
                vs.push(v);
                w.update(v);
            }
        }

        let w = Welford::merge(&ws);
        let (mean, variance) = mean_variance(&vs);
        assert!((w.mean() - mean).abs() <= E, "mean");
        assert!((w.variance() - variance).abs() <= E, "variance");
    }

    #[test]
    fn merge_of_empty_is_empty() {
        let w = Welford::merge(&[Welford::new(); 4]);
        assert_eq!(w.count(), 0.0);
        assert_eq!(w.mean(), 0.0);
        assert_eq!(w.variance(), 0.0);
    }

    /// Odd counts split unevenly and still agree with a single pass.
    #[test]
    fn merge_odd_splits() {
        for k in 1..8 {
            let mut ws = vec![Welford::new(); k];
            let mut all = Welford::new();
            for (i, w) in ws.iter_mut().enumerate() {
                for j in 0..=i {
                    let v = (i * 7 + j * 3) as f64;
                    w.update(v);
                    all.update(v);
                }
            }
            let m = Welford::merge(&ws);
            assert_eq!(m.count(), all.count());
            assert!((m.mean() - all.mean()).abs() <= E);
            if k > 1 {
                assert!((m.variance() - all.variance()).abs() <= E);
            }
        }
    }

    #[test]
    fn merge_is_bit_exact() {
        let mut ws = [Welford::new(); 3];
        for (i, w) in ws.iter_mut().enumerate() {
            for j in 0..=i * 3 {
                w.update((i * 7 + j * 3) as f64 / 3.0);
            }
        }
        let m = Welford::merge(&ws);
        assert_eq!(m.count(), 12.0);
        assert_eq!(m.mean(), 5.75);
        assert_eq!(m.variance(), 9.679292929292927);
    }

    #[test]
    fn value_below_two_samples() {
        let mut w = Welford::new();
        w.update(3.0);
        assert_eq!(w.value(), (3.0, 0.0, 0.0));
        assert!(w.variance().is_nan());
        w.update(5.0);
        assert_eq!(w.value(), (4.0, 1.0, 2.0));
        w.reset();
        assert_eq!(w, Welford::new());
    }
}
