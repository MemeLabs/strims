// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! When the first swarm's viewers arrive and leave
//! ([`Scenario::arrivals`], [`Scenario::departures`]).
//!
//! A profile is data in the scenario; the run resolves it at its start
//! ([`resolve`]) into each viewer's [`HostSpec::arrive_at`] and a
//! [`Fault::Depart`] per leaver, from the run's rng after the draws every
//! run makes (see [Determinism](crate#determinism)). The knobs take the
//! ppspp sim's forms, shares in percent and times in seconds, here from the
//! swarm's stream start: a cell's stream starts after its seed has booted,
//! a few seconds into the run, where the ppspp sim's starts a second after
//! its own, so a profile lands at the same point of the stream in both.
//!
//! - **Arrivals** ([`Arrivals`]): with [`Arrivals::T0`] every host boots
//!   with the run. Otherwise the seed (and every host that isn't a viewer
//!   of the first swarm) boots with the run and the viewers boot at their
//!   arrivals, in random order: [`Arrivals::Trickle`] as a Poisson stream,
//!   [`Arrivals::Event`] a warm share before a burst. A viewer that arrives
//!   reads from the live edge ([`HostSpec::arrive_at`]).
//! - **The topology is laid out in arrival order**: the viewer that arrives
//!   `k`-th takes the place of the swarm's `k`-th viewer in the scenario's
//!   links. A layout whose hosts dial earlier ones ([`random_mesh`]) then
//!   dials only hosts already there, so no early arrival waits for its
//!   links' other ends; the hosts keep their own sites, uplinks and
//!   classes.
//! - **Departures** ([`Departures`]): a share of the viewers that arrived
//!   before the window leave for good within it, each at a uniform time
//!   in it, all [`Departure::Graceful`] or all [`Departure::Abrupt`].
//!
//! [`HostSpec::arrive_at`]: crate::HostSpec::arrive_at
//! [`random_mesh`]: crate::topology::random_mesh

use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use rand_core::Rng;
use strims_base::rng::DetRng;

use crate::spec::{Fault, HostIx, Scenario};

/// When the first swarm's viewers boot (see the [module docs](self)).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Arrivals {
    /// Every host boots with the run, as a scenario's hosts do by default.
    #[default]
    T0,
    /// The viewers arrive as a Poisson stream from the stream's start, at
    /// `rate` a second; `None`: all within the stream's first third (a
    /// Poisson stream conditioned on its count, so uniform times in it).
    Trickle { rate: Option<f64> },
    /// `warm_pct` percent of the viewers (rounded) arrive uniformly over
    /// the stream's first `at`, the rest uniformly within `window` from
    /// `at`: a scheduled event's crowd joining a smaller warm swarm.
    Event {
        warm_pct: f64,
        at: Duration,
        window: Duration,
    },
}

/// How a viewer leaves ([`Fault::Depart`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Departure {
    /// As a closed tab: it leaves its swarms and closes every link, so its
    /// peers see the links close; it takes no link again.
    #[default]
    Graceful,
    /// As a lost connection: it goes silent, cut off from every host and
    /// stopped (as [`Fault::Crash`], for good). Its peers notice by
    /// timeouts.
    Abrupt,
}

/// A share of the first swarm's viewers leaving for good (see the
/// [module docs](self)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Departures {
    /// The percentage of the viewers that leave, rounded; at most those
    /// that arrived before `at`.
    pub share_pct: f64,
    /// When the window opens, from the stream's start, and how long it is.
    pub at: Duration,
    pub window: Duration,
    pub how: Departure,
}

impl Arrivals {
    pub fn is_t0(&self) -> bool {
        *self == Arrivals::T0
    }
}

fn secs(s: &str, what: &str) -> Result<Duration, String> {
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite() && *x >= 0.0)
        .map(Duration::from_secs_f64)
        .ok_or_else(|| format!("{s} is no {what} in seconds"))
}

fn percent(s: &str) -> Result<f64, String> {
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|x| (0.0..=100.0).contains(x))
        .ok_or_else(|| format!("{s} is no percentage"))
}

/// `t0`, `trickle`, `trickle:<per s>` or `event:<warm %>:<at s>:<window
/// s>` (`STRIMS_SIM_ARRIVALS`), the ppspp sim's `SIM_ARRIVALS` forms.
impl FromStr for Arrivals {
    type Err = String;

    fn from_str(s: &str) -> Result<Arrivals, String> {
        let parts: Vec<&str> = s.trim().split(':').collect();
        match parts.as_slice() {
            ["t0"] => Ok(Arrivals::T0),
            ["trickle"] => Ok(Arrivals::Trickle { rate: None }),
            ["trickle", r] => r
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|x| x.is_finite() && *x > 0.0)
                .map(|rate| Arrivals::Trickle { rate: Some(rate) })
                .ok_or_else(|| format!("{r} is no rate a second")),
            ["event", w, at, window] => Ok(Arrivals::Event {
                warm_pct: percent(w)?,
                at: secs(at, "time")?,
                window: secs(window, "window")?,
            }),
            _ => Err("t0, trickle[:<per s>] or event:<warm %>:<at s>:<window s>".to_owned()),
        }
    }
}

/// The knob's form ([`Arrivals::from_str`]).
impl fmt::Display for Arrivals {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arrivals::T0 => f.write_str("t0"),
            Arrivals::Trickle { rate: None } => f.write_str("trickle"),
            Arrivals::Trickle { rate: Some(r) } => write!(f, "trickle:{r}"),
            Arrivals::Event {
                warm_pct,
                at,
                window,
            } => write!(
                f,
                "event:{warm_pct}:{}:{}",
                at.as_secs_f64(),
                window.as_secs_f64()
            ),
        }
    }
}

/// `<share %>:<at s>:<window s>[:abrupt|:graceful]` (`STRIMS_SIM_DEPART`),
/// the ppspp sim's `SIM_DEPART` forms.
impl FromStr for Departures {
    type Err = String;

    fn from_str(s: &str) -> Result<Departures, String> {
        let parts: Vec<&str> = s.trim().split(':').collect();
        let (share_, at, window, how) = match parts.as_slice() {
            [s, a, w] => (s, a, w, Departure::Graceful),
            [s, a, w, "graceful"] => (s, a, w, Departure::Graceful),
            [s, a, w, "abrupt"] => (s, a, w, Departure::Abrupt),
            _ => return Err("<share %>:<at s>:<window s>[:abrupt]".to_owned()),
        };
        Ok(Departures {
            share_pct: percent(share_)?,
            at: secs(at, "time")?,
            window: secs(window, "window")?,
            how,
        })
    }
}

/// The knob's form ([`Departures::from_str`]).
impl fmt::Display for Departures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.share_pct,
            self.at.as_secs_f64(),
            self.window.as_secs_f64()
        )?;
        if self.how == Departure::Abrupt {
            f.write_str(":abrupt")?;
        }
        Ok(())
    }
}

/// A uniform draw in `[0, 1)`.
fn unit(rng: &mut DetRng) -> f64 {
    (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

/// `x` seconds, to the millisecond.
fn ms(x: f64) -> Duration {
    Duration::from_millis((x.max(0.0) * 1000.0) as u64)
}

/// `v` in random order (Fisher-Yates).
fn shuffle<T>(rng: &mut DetRng, v: &mut [T]) {
    for i in (1..v.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
}

/// `n` uniform times in `[from, from + span)`, ascending.
fn uniform_times(rng: &mut DetRng, n: usize, from: Duration, span: Duration) -> Vec<Duration> {
    let mut t: Vec<Duration> = (0..n)
        .map(|_| from + ms(unit(rng) * span.as_secs_f64()))
        .collect();
    t.sort_unstable();
    t
}

/// Resolves the scenario's profiles (see the [module docs](self)): sets the
/// first swarm's viewers' arrivals, lays the links out in arrival order and
/// adds the departures' faults. Draws nothing for [`Arrivals::T0`] and no
/// departures.
pub fn resolve(s: &mut Scenario, rng: &mut DetRng) {
    let Some((viewers, start, stop)) = s
        .swarms
        .first()
        .map(|sw| (sw.viewers.clone(), sw.start, sw.stop))
    else {
        return;
    };
    let span = stop.saturating_sub(start);
    let n = viewers.len();
    if !s.arrivals.is_t0() && n > 0 {
        let mut order = viewers.clone();
        shuffle(rng, &mut order);
        let times = match s.arrivals {
            Arrivals::T0 => Vec::new(),
            Arrivals::Trickle { rate: None } => uniform_times(rng, n, start, span / 3),
            Arrivals::Trickle { rate: Some(r) } => {
                let mut t = 0.0;
                (0..n)
                    .map(|_| {
                        t += -(1.0 - unit(rng)).ln() / r;
                        start + ms(t)
                    })
                    .collect()
            }
            Arrivals::Event {
                warm_pct,
                at,
                window,
            } => {
                let m = ((n as f64 * warm_pct / 100.0).round() as usize).min(n);
                let mut t = uniform_times(rng, m, start, at);
                t.extend(uniform_times(rng, n - m, start + at, window));
                t
            }
        };
        for (&h, &t) in order.iter().zip(&times) {
            if let Some(spec) = s.hosts.get_mut(h) {
                spec.arrive_at = Some(t);
            }
        }
        lay_out(s, &viewers);
    }
    if let Some(d) = s.departures
        && n > 0
    {
        let open = start + d.at;
        let mut eligible: Vec<HostIx> = viewers
            .iter()
            .copied()
            .filter(|&h| {
                s.hosts
                    .get(h)
                    .is_some_and(|x| x.arrive_at.unwrap_or_default() < open)
            })
            .collect();
        let m = ((n as f64 * d.share_pct / 100.0).round() as usize).min(eligible.len());
        // A partial shuffle: the first `m` are a uniform draw.
        for i in 0..m {
            let j = i + (rng.next_u64() % (eligible.len() - i) as u64) as usize;
            eligible.swap(i, j);
        }
        for &h in eligible.iter().take(m) {
            let t = open + ms(unit(rng) * d.window.as_secs_f64());
            s.at(t, Fault::Depart(h, d.how));
        }
    }
}

/// Lays the scenario's links out in arrival order: the swarm's `k`-th
/// viewer to arrive (ties in `viewers`' order) takes the place of
/// `viewers[k]` at either end of every link.
fn lay_out(s: &mut Scenario, viewers: &[HostIx]) {
    let mut by_arrival: Vec<(Duration, usize, HostIx)> = viewers
        .iter()
        .enumerate()
        .map(|(k, &h)| {
            let at = s.hosts.get(h).and_then(|x| x.arrive_at).unwrap_or_default();
            (at, k, h)
        })
        .collect();
    by_arrival.sort_unstable();
    let place: std::collections::BTreeMap<HostIx, HostIx> = viewers
        .iter()
        .zip(&by_arrival)
        .map(|(&h, &(_, _, arrival))| (h, arrival))
        .collect();
    let to = |h: HostIx| place.get(&h).copied().unwrap_or(h);
    for l in &mut s.links {
        l.from = to(l.from);
        l.to = to(l.to);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing, clippy::unwrap_used)]

    use super::*;
    use crate::spec::SwarmSpec;
    use crate::topology::random_mesh;
    use strims_base::rng::det_rng;

    /// A seed and 12 viewers in a random mesh, streaming from 5 s to 65 s.
    fn mesh() -> Scenario {
        let mut s = Scenario::new("t");
        let seed = s.host("seed", None);
        let vs = s.hosts("v", 12, None);
        let all: Vec<HostIx> = std::iter::once(seed).chain(vs.iter().copied()).collect();
        random_mesh(&mut s, &mut det_rng(1), &all, 3, 0.0);
        s.swarm(SwarmSpec::live(
            seed,
            vs,
            1024,
            Duration::from_secs(5),
            Duration::from_secs(65),
        ));
        s
    }

    #[test]
    fn profiles_parse_and_print_their_knobs() {
        for k in ["t0", "trickle", "trickle:0.5", "event:20:20:10"] {
            assert_eq!(k.parse::<Arrivals>().unwrap().to_string(), k);
        }
        for k in ["60:40:10", "60:40:10:abrupt", "12.5:40:10"] {
            assert_eq!(k.parse::<Departures>().unwrap().to_string(), k);
        }
        assert!("trickle:0".parse::<Arrivals>().is_err());
        assert!("event:150:20:10".parse::<Arrivals>().is_err());
        assert!("60:40".parse::<Departures>().is_err());
        assert!("60:40:10:soon".parse::<Departures>().is_err());
    }

    #[test]
    fn t0_draws_nothing_and_changes_nothing() {
        let mut s = mesh();
        let links = s.links.clone();
        let mut rng = det_rng(9);
        resolve(&mut s, &mut rng);
        assert_eq!(rng.next_u64(), det_rng(9).next_u64());
        assert_eq!(s.links, links);
        assert!(s.hosts.iter().all(|h| h.arrive_at.is_none()));
        assert!(s.faults.is_empty());
    }

    #[test]
    fn a_trickle_arrives_within_the_first_third_and_dials_only_hosts_there() {
        let mut s = mesh();
        s.arrivals = Arrivals::Trickle { rate: None };
        let before = s.links.len();
        resolve(&mut s, &mut det_rng(3));
        let at = |h: HostIx| s.hosts[h].arrive_at.unwrap_or_default();
        let vs = s.swarms[0].viewers.clone();
        assert!(s.hosts[0].arrive_at.is_none());
        for &v in &vs {
            let t = at(v);
            assert!(
                t >= Duration::from_secs(5) && t < Duration::from_secs(25),
                "{t:?}"
            );
        }
        // Random order: not the viewers' own.
        let mut order = vs.clone();
        order.sort_by_key(|&h| (at(h), h));
        assert_ne!(order, vs);
        // Every link's dialer arrives after the host it dials.
        assert_eq!(s.links.len(), before);
        for l in &s.links {
            assert!(at(l.from) >= at(l.to), "{l:?}");
        }
        // The first to arrive dials the seed.
        assert!(s.links.iter().any(|l| l.from == order[0] && l.to == 0));
    }

    #[test]
    fn an_event_bursts_into_its_warm_share() {
        let mut s = mesh();
        s.arrivals = "event:25:20:10".parse().unwrap();
        resolve(&mut s, &mut det_rng(4));
        let mut t: Vec<Duration> = s.swarms[0]
            .viewers
            .iter()
            .map(|&h| s.hosts[h].arrive_at.unwrap())
            .collect();
        t.sort_unstable();
        let warm = t.iter().filter(|&&x| x < Duration::from_secs(25)).count();
        assert_eq!(warm, 3);
        assert!(t[3..].iter().all(|&x| x >= Duration::from_secs(25)));
        assert!(t[11] < Duration::from_secs(35));
    }

    #[test]
    fn departures_take_viewers_already_there() {
        let mut s = mesh();
        s.arrivals = Arrivals::Trickle { rate: None };
        s.departures = Some("50:15:10:abrupt".parse().unwrap());
        resolve(&mut s, &mut det_rng(5));
        let open = Duration::from_secs(20);
        let there = s.swarms[0]
            .viewers
            .iter()
            .filter(|&&h| s.hosts[h].arrive_at.unwrap() < open)
            .count();
        let leaving: Vec<_> = s
            .faults
            .iter()
            .filter_map(|f| match f.fault {
                Fault::Depart(h, how) => Some((h, f.at, how)),
                _ => None,
            })
            .collect();
        assert_eq!(leaving.len(), there.min(6));
        for (h, at, how) in leaving {
            assert!(s.hosts[h].arrive_at.unwrap() < open);
            assert!(at >= open && at < open + Duration::from_secs(10));
            assert_eq!(how, Departure::Abrupt);
        }
    }
}
