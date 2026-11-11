// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A live player's playout of what a viewer reads, at a fixed delay:
//! stall and resume, never catching up.
//!
//! A viewer's units (a frame or a chunk, whatever the sim reads) are what
//! the tables' `reads` holds: when each was read, which, and its lag, in
//! whole ms. A unit read twice counts at its first read: the player holds
//! it from then. Its due time is its read time less its lag, and the units
//! due at one time play together, as late as the latest of them.
//! [`play`] plays them in due order at `due + offset`, the offset starting
//! at the delay and rising to a unit's lag when the unit comes later than
//! that: each rise is a stall of its size, from the unit's play time at the
//! old offset until it came. The first unit's own lateness starts playback
//! later instead: playback starts at its due time plus the delay, or when
//! it came if that's later. A unit never read is skipped.
//!
//! `hack/sim-db/macros.sql`'s `playback(delay_ms)` plays the `reads` table
//! the same way, and gives the tables' `stalls`.

/// The playout delays the sims play their viewers at, in ms: delivery's
/// share of the 2 s target and of the 5 s maximum glass to glass,
/// `hack/capture/analyze.py`'s `DELIVERY_TARGET_MS` and `DELIVERY_MAX_MS`.
pub const DELAYS_MS: [i64; 2] = [1250, 4250];

/// A viewer's playout at one delay ([`play`]), in the reads' ms.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Playout {
    /// When playback started and when its last unit played; `None`
    /// without a unit.
    pub span: Option<(i64, i64)>,
    /// Each stall, in order: when it began and how long it lasted.
    pub stalls: Vec<(i64, i64)>,
}

/// Plays `reads`, a viewer's `(t_ms, seq, lag_ms)` in any order, at
/// `delay_ms` (see the [module docs](self)).
pub fn play(reads: &[(i64, u64, i64)], delay_ms: i64) -> Playout {
    let mut first: Vec<(u64, i64, i64)> = reads.iter().map(|&(t, s, l)| (s, t, l)).collect();
    first.sort_unstable();
    first.dedup_by_key(|&mut (s, _, _)| s);
    let mut due: Vec<(i64, i64)> = first.iter().map(|&(_, t, l)| (t - l, l)).collect();
    due.sort_unstable();
    let mut out = Playout::default();
    let mut at: Option<(i64, i64, i64)> = None;
    let mut i = 0;
    while let Some(&(d, _)) = due.get(i) {
        // The units due at `d` play as late as the latest of them.
        let mut lag = i64::MIN;
        while let Some(&(e, l)) = due.get(i)
            && e == d
        {
            lag = lag.max(l);
            i += 1;
        }
        at = Some(match at {
            None => {
                let offset = delay_ms.max(lag);
                (d + offset, d, offset)
            }
            Some((start, _, offset)) if lag > offset => {
                out.stalls.push((d + offset, lag - offset));
                (start, d, lag)
            }
            Some((start, _, offset)) => (start, d, offset),
        });
    }
    out.span = at.map(|(start, d, offset)| (start, d + offset));
    out
}

/// Viewers' playouts at one delay, summed for a run's row
/// ([`Totals::add`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    pub stalls: u64,
    pub stalled_ms: i64,
    /// The time they played for: each one's from its playback's start to
    /// its last unit's play, or to its leaving if that came first.
    pub played_ms: i64,
    /// From each one's join to its playback's start, summed over those
    /// that started.
    pub startup_ms: i64,
    pub started: u64,
}

impl Totals {
    /// Adds a viewer's playout `p`, its join at `join_ms` and its leaving
    /// for good at `left_ms`, all in the reads' ms.
    pub fn add(&mut self, p: &Playout, join_ms: i64, left_ms: Option<i64>) {
        let Some((start, end)) = p.span else {
            return;
        };
        let end = left_ms.map_or(end, |l| l.min(end));
        self.played_ms += end.saturating_sub(start).max(0);
        self.stalls += p.stalls.len() as u64;
        self.stalled_ms += p.stalls.iter().map(|&(_, ms)| ms).sum::<i64>();
        self.startup_ms += start.saturating_sub(join_ms).max(0);
        self.started += 1;
    }

    /// Stalls per minute played; `None` without any time played.
    pub fn stalls_per_min(&self) -> Option<f64> {
        (self.played_ms > 0).then(|| self.stalls as f64 * 60_000.0 / self.played_ms as f64)
    }

    /// The share of the time played that was stalled.
    pub fn stalled_share(&self) -> Option<f64> {
        (self.played_ms > 0).then(|| self.stalled_ms as f64 / self.played_ms as f64)
    }

    /// The mean time from join to playback's start, in ms.
    pub fn startup_ms(&self) -> Option<i64> {
        (self.started > 0).then(|| self.startup_ms / self.started as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_late_unit_stalls_playback_until_it_comes_and_the_delay_stays() {
        // Units due every 100 ms; the third comes 400 ms late at 1250.
        let reads = [
            (1200, 0, 1200),
            (1300, 1, 1200),
            (1850, 2, 1650),
            (1500, 3, 1200),
        ];
        let p = play(&reads, 1250);
        assert_eq!(p.span, Some((1250, 1950)));
        assert_eq!(p.stalls, [(1450, 400)]);
        // No catch-up: the next unit plays at the new offset, and an
        // earlier one at 4250 never stalls.
        assert_eq!(play(&reads, 4250).stalls, []);
    }

    #[test]
    fn the_first_unit_starts_playback_late_rather_than_stalling() {
        let p = play(&[(2000, 0, 2000), (2100, 1, 2000)], 1250);
        assert_eq!(
            p,
            Playout {
                span: Some((2000, 2100)),
                stalls: vec![]
            }
        );
        assert_eq!(play(&[], 1250), Playout::default());
    }

    #[test]
    fn a_unit_counts_at_its_first_read_and_ties_play_together() {
        // Unit 1 read again later, and units 2 and 3 due together, 3 the
        // later: one stall, as late as unit 3.
        let reads = [
            (100, 0, 100),
            (200, 1, 100),
            (9000, 1, 8900),
            (1600, 2, 1400),
            (1700, 3, 1500),
        ];
        assert_eq!(play(&reads, 1250).stalls, [(1450, 250)]);
    }

    #[test]
    fn totals_count_the_time_played_until_the_viewer_left() {
        let reads = [
            (1200, 0, 1200),
            (1300, 1, 1200),
            (1850, 2, 1650),
            (1500, 3, 1200),
        ];
        let p = play(&reads, 1250);
        let mut t = Totals::default();
        t.add(&p, 0, None);
        t.add(&p, 1000, Some(1550));
        t.add(&Playout::default(), 0, None);
        assert_eq!(
            t,
            Totals {
                stalls: 2,
                stalled_ms: 800,
                played_ms: 700 + 300,
                startup_ms: 1250 + 250,
                started: 2,
            }
        );
        assert_eq!(t.stalls_per_min(), Some(120.0));
        assert_eq!(t.stalled_share(), Some(0.8));
        assert_eq!(t.startup_ms(), Some(750));
    }
}
