// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The first swarm's seed's allocation set from outside
//! (`STRIMS_SIM_SEED_PIN`, [`Scenario::seed_pin`]): its policy, and a pin
//! (`Node::pin_seed_allocation`) of the viewers it pushes each stream to,
//! picked from what the seed can't know (the sites' uplinks, cities and
//! last miles), for bounds on its own allocation and comparisons with it.
//!
//! A pinned seed admits (`SeedAllocation::Admission`) and accepts each
//! stream's subscriptions from its pinned viewers alone. Its slots (one
//! stream at one viewer) are the stream slots its upload pays for at the
//! stream's rate: its upload cap, else nine tenths of its site's uplink,
//! over a stream's share of the rate ([`SeedPin::slots_in`]), unless the
//! knob names them. They go to the viewers it has a scenario link to (all
//! of them in an organic scenario, whose links form during the run), at
//! most one per stream and viewer, in whole copies of the stream (every
//! stream), then a part copy of the streams left, except where a case
//! says otherwise. A seed whose upload pays for a copy at each viewer it
//! links to leaves a pin nothing to choose: the bounds take a seed linked
//! to every viewer (`STRIMS_SIM_SEED_LINKS=all`,
//! [`Scenario::link_seed_to_viewers`]). The cases:
//!
//! - `random`: each stream's at viewers drawn for it alone.
//! - `whole`: at viewers drawn once.
//! - `nearest`: at the viewers whose path to the seed is the shortest
//!   (half their cities' round trip and both last miles).
//! - `oracle`: at the honest viewers with the fastest uplinks, among equal
//!   uplinks the nearest the seed's city first, each copy in another
//!   region than the last one's when one is left.
//! - `dc-only`: every stream at the datacenter viewers alone, whatever
//!   the slots.
//!
//! Each takes the seed's uplink shares by allocation with `-qos`
//! (`SeedQos::ByAllocation`), equal shares without. `admission` and
//! `adaptive` (with or without `-qos`) set the policy and pin nothing.
//!
//! The run resolves the pin after the dishonest mix's draw
//! ([`resolve`]), so the oracle knows the hostile viewers; `random` and
//! `whole` draw their viewers from the run's rng there, and no other case
//! draws anything (see [Determinism](crate#determinism)). The seed pins
//! the viewers that are its peers each time a peer links, since a pin
//! names peers.

use std::cmp::Reverse;
use std::fmt;

use rand_core::Rng;
use strims_base::rng::DetRng;
use strims_ppspp::{PushMode, SeedAllocation, SeedPolicy, SeedQos};
use strims_wan::Access;

use crate::spec::{HostClass, HostIx, Scenario};

/// How a pin picks its viewers (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinCase {
    Random,
    Whole,
    Nearest,
    Oracle,
    DcOnly,
}

impl PinCase {
    const ALL: [PinCase; 5] = [
        PinCase::Random,
        PinCase::Whole,
        PinCase::Nearest,
        PinCase::Oracle,
        PinCase::DcOnly,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PinCase::Random => "random",
            PinCase::Whole => "whole",
            PinCase::Nearest => "nearest",
            PinCase::Oracle => "oracle",
            PinCase::DcOnly => "dc-only",
        }
    }
}

/// The first swarm's seed's allocation ([`Scenario::seed_pin`]): its
/// policy, and with a case its pin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedPin {
    /// How the pin picks its viewers; `None` pins nothing.
    pub case: Option<PinCase>,
    pub policy: SeedPolicy,
    /// The slots the knob named; `None` takes what the seed's upload pays
    /// for ([`SeedPin::slots_in`]).
    pub slots: Option<usize>,
    /// Per stream, the viewers the run pinned ([`resolve`]); empty before.
    pub hosts: Vec<Vec<HostIx>>,
}

impl SeedPin {
    /// `STRIMS_SIM_SEED_PIN`'s forms: `<case>[-qos][:<slots>]` for a case
    /// of [`PinCase`] (`random`, `whole`, `nearest`, `oracle`, `dc-only`),
    /// `admission[-qos]` or `adaptive[-qos]` for a policy alone, and `off`
    /// or blank for none.
    pub fn parse(v: &str) -> Result<Option<SeedPin>, String> {
        let v = v.trim();
        if v.is_empty() || v == "off" {
            return Ok(None);
        }
        let (name, slots) = match v.split_once(':') {
            Some((name, k)) => {
                let k = k
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| format!("{k:?} is no slot count"))?;
                (name.trim(), Some(k))
            }
            None => (v, None),
        };
        let (base, qos) = match name.strip_suffix("-qos") {
            Some(b) => (b, SeedQos::ByAllocation),
            None => (name, SeedQos::Equal),
        };
        let (case, allocation) = match base {
            "admission" => (None, SeedAllocation::Admission),
            "adaptive" => (None, SeedAllocation::Adaptive),
            _ => (
                Some(
                    PinCase::ALL
                        .into_iter()
                        .find(|c| c.name() == base)
                        .ok_or_else(|| format!("{base} is no seed allocation case"))?,
                ),
                SeedAllocation::Admission,
            ),
        };
        if case.is_none() && slots.is_some() {
            return Err(format!("{base} pins nothing, so it takes no slots"));
        }
        Ok(Some(SeedPin {
            case,
            policy: SeedPolicy { allocation, qos },
            slots,
            hosts: Vec::new(),
        }))
    }

    /// Why `s` can't take the allocation, if it can't: a pin needs a first
    /// swarm in hybrid push, slots (the knob's, or a seed with an upload
    /// cap or a site), and for `nearest`, `oracle` and `dc-only` WAN sites,
    /// `dc-only` a datacenter viewer.
    pub fn check(&self, s: &Scenario) -> Result<(), String> {
        let Some(case) = self.case else {
            return Ok(());
        };
        let sw = s.swarms.first().ok_or("a pin needs a swarm")?;
        if sw.options.push_mode.resolve() != PushMode::Hybrid {
            return Err("a pin needs the first swarm in hybrid push".to_owned());
        }
        if self.slots.is_none() && self.slots_in(s).is_none() {
            return Err(
                "the seed has no upload cap or site to pay for slots: name them".to_owned(),
            );
        }
        let sited = |h: HostIx| s.hosts.get(h).and_then(|x| x.site).is_some();
        let sites = sited(sw.seed) && sw.viewers.iter().all(|&v| sited(v));
        if matches!(case, PinCase::Nearest | PinCase::Oracle | PinCase::DcOnly) && !sites {
            return Err(format!("{} needs WAN sites", case.name()));
        }
        if case == PinCase::DcOnly && datacenters(s, &pool(s)).is_empty() {
            return Err("dc-only needs a datacenter viewer the seed links to".to_owned());
        }
        Ok(())
    }

    /// The stream slots the first swarm's seed's upload pays for: its
    /// upload cap, else nine tenths of its site's uplink (as
    /// [`Scenario::host_at`] caps one), over a stream's share of the
    /// swarm's rate. `None` without either.
    pub fn slots_in(&self, s: &Scenario) -> Option<usize> {
        let sw = s.swarms.first()?;
        let host = s.hosts.get(sw.seed)?;
        let cap = host
            .uplink
            .or_else(|| host.site.map(|x| x.profile.up / 10 * 9))?;
        let per_stream = sw.rate() / sw.options.stream_count.max(1) as f64;
        (per_stream > 0.0).then(|| (cap as f64 / per_stream) as usize)
    }
}

impl fmt::Display for SeedPin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base = match (self.case, self.policy.allocation) {
            (Some(c), _) => c.name(),
            (None, SeedAllocation::Adaptive) => "adaptive",
            (None, _) => "admission",
        };
        let qos = match self.policy.qos {
            SeedQos::ByAllocation => "-qos",
            _ => "",
        };
        write!(f, "{base}{qos}")?;
        if let Some(k) = self.slots {
            write!(f, ":{k}")?;
        }
        Ok(())
    }
}

/// Sets `s`'s first swarm's seed policy to `pin`'s, and keeps the pin for
/// the run to resolve ([`resolve`]).
pub fn apply(s: &mut Scenario, pin: SeedPin) {
    if let Some(sw) = s.swarms.first_mut() {
        sw.options.seed = pin.policy;
    }
    s.seed_pin = Some(pin);
}

/// Picks the pin's viewers ([`SeedPin::hosts`]), drawing `random`'s and
/// `whole`'s from `rng`; nothing without a pin.
pub fn resolve(s: &mut Scenario, rng: &mut DetRng) {
    let Some(pin) = s.seed_pin.as_ref() else {
        return;
    };
    let Some(case) = pin.case else {
        return;
    };
    let Some(sw) = s.swarms.first() else {
        return;
    };
    let streams = sw.options.stream_count.max(1);
    let pool = pool(s);
    let slots = pin
        .slots
        .or_else(|| pin.slots_in(s))
        .unwrap_or(0)
        .min(pool.len() * streams);
    let hosts = match case {
        PinCase::Random => random(&pool, slots, streams, rng),
        PinCase::Whole => whole(&pool, slots, streams, rng),
        PinCase::Nearest => nearest(s, &pool, slots, streams),
        PinCase::Oracle => oracle(s, &pool, slots, streams),
        PinCase::DcOnly => vec![datacenters(s, &pool); streams],
    };
    if let Some(p) = s.seed_pin.as_mut() {
        p.hosts = hosts;
    }
}

/// The first swarm's viewers the seed may push to: those it has a
/// scenario link with, every one in an organic scenario.
fn pool(s: &Scenario) -> Vec<HostIx> {
    let Some(sw) = s.swarms.first() else {
        return Vec::new();
    };
    let seed = sw.seed;
    let linked = |v: HostIx| {
        s.discovery.is_some()
            || s.links
                .iter()
                .any(|l| (l.from, l.to) == (seed, v) || (l.from, l.to) == (v, seed))
    };
    sw.viewers.iter().copied().filter(|&v| linked(v)).collect()
}

/// `slots` over `streams`: whole copies, and the streams of one more.
fn copies(slots: usize, streams: usize) -> (usize, usize) {
    (slots / streams.max(1), slots % streams.max(1))
}

/// Per stream, the first of `order` its copies take: `full` of them, and
/// one more for the first `rem` streams.
fn take_copies(order: &[HostIx], slots: usize, streams: usize) -> Vec<Vec<HostIx>> {
    let (full, rem) = copies(slots, streams);
    (0..streams)
        .map(|i| {
            order
                .iter()
                .copied()
                .take(full + usize::from(i < rem))
                .collect()
        })
        .collect()
}

fn shuffle(v: &mut [HostIx], rng: &mut DetRng) {
    for i in (1..v.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
}

fn random(pool: &[HostIx], slots: usize, streams: usize, rng: &mut DetRng) -> Vec<Vec<HostIx>> {
    let (full, rem) = copies(slots, streams);
    (0..streams)
        .map(|i| {
            let mut v = pool.to_vec();
            shuffle(&mut v, rng);
            v.truncate(full + usize::from(i < rem));
            v
        })
        .collect()
}

fn whole(pool: &[HostIx], slots: usize, streams: usize, rng: &mut DetRng) -> Vec<Vec<HostIx>> {
    let mut v = pool.to_vec();
    shuffle(&mut v, rng);
    take_copies(&v, slots, streams)
}

fn nearest(s: &Scenario, pool: &[HostIx], slots: usize, streams: usize) -> Vec<Vec<HostIx>> {
    let site = |h: HostIx| s.hosts.get(h).and_then(|x| x.site);
    let Some(seed) = s.swarms.first().and_then(|sw| site(sw.seed)) else {
        return vec![Vec::new(); streams];
    };
    let mut v = pool.to_vec();
    v.sort_by_key(|&h| {
        let path = site(h)
            .map(|x| x.city.rtt(seed.city) / 2 + x.profile.last_mile + seed.profile.last_mile);
        (path, h)
    });
    take_copies(&v, slots, streams)
}

fn oracle(s: &Scenario, pool: &[HostIx], slots: usize, streams: usize) -> Vec<Vec<HostIx>> {
    let site = |h: HostIx| s.hosts.get(h).and_then(|x| x.site);
    let Some(seed) = s.swarms.first().and_then(|sw| site(sw.seed)) else {
        return vec![Vec::new(); streams];
    };
    let honest = |h: HostIx| s.hosts.get(h).is_some_and(|x| x.class == HostClass::Honest);
    let mut left: Vec<(u64, HostIx)> = pool
        .iter()
        .copied()
        .filter(|&h| honest(h))
        .filter_map(|h| Some((site(h)?.profile.up, h)))
        .collect();
    left.sort_by_key(|&(up, h)| (Reverse(up), site(h).map(|x| x.city.rtt(seed.city)), h));
    let (full, rem) = copies(slots, streams);
    let mut holders = Vec::new();
    let mut last = None;
    while holders.len() < full + usize::from(rem > 0) {
        let Some(&(best, _)) = left.first() else {
            break;
        };
        let region = |h: HostIx| site(h).map(|x| x.city.region());
        let k = left
            .iter()
            .take_while(|&&(up, _)| up == best)
            .position(|&(_, h)| region(h) != last)
            .unwrap_or(0);
        let (_, h) = left.remove(k);
        last = region(h);
        holders.push(h);
    }
    take_copies(&holders, slots, streams)
}

/// `of`'s viewers on datacenter access.
fn datacenters(s: &Scenario, of: &[HostIx]) -> Vec<HostIx> {
    of.iter()
        .copied()
        .filter(|&h| {
            s.hosts
                .get(h)
                .and_then(|x| x.site)
                .is_some_and(|x| x.access == Access::Datacenter)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use strims_base::rng::det_rng;
    use strims_wan::{City, Region, Site};

    use super::*;
    use crate::spec::{SwarmSpec, Transport};

    const RATE: u64 = 256 * 1024;

    /// A seed on cable in Chicago and viewers at `sites`, the seed linked
    /// to those `linked` says, in 16 streams at 256 KiB/s in hybrid push.
    fn wan(sites: &[Site], linked: impl Fn(usize) -> bool) -> Scenario {
        let mut s = Scenario::new("pins");
        let seed = s.host_at("seed", Site::new(City::Chicago, Access::Cable));
        let vs = s.hosts_at("v", sites);
        for (i, &v) in vs.iter().enumerate() {
            if linked(i) {
                s.link(v, seed, Transport::Tcp);
            }
        }
        let mut sw = SwarmSpec::live(
            seed,
            vs,
            RATE,
            std::time::Duration::ZERO,
            std::time::Duration::from_secs(10),
        )
        .with_push_mode(PushMode::Hybrid);
        sw.options.stream_count = 16;
        s.swarm(sw);
        s
    }

    fn pinned(s: &mut Scenario, knob: &str, seed: u64) -> Vec<Vec<HostIx>> {
        let pin = SeedPin::parse(knob).unwrap().unwrap();
        pin.check(s).unwrap();
        apply(s, pin);
        resolve(s, &mut det_rng(seed));
        s.seed_pin.clone().unwrap().hosts
    }

    #[test]
    fn the_knob_takes_cases_policies_and_slots() {
        assert_eq!(SeedPin::parse("off"), Ok(None));
        assert_eq!(SeedPin::parse(" "), Ok(None));
        let p = SeedPin::parse("oracle-qos:64").unwrap().unwrap();
        assert_eq!(p.case, Some(PinCase::Oracle));
        assert_eq!(p.policy.qos, SeedQos::ByAllocation);
        assert_eq!(p.policy.allocation, SeedAllocation::Admission);
        assert_eq!(p.slots, Some(64));
        assert_eq!(p.to_string(), "oracle-qos:64");
        let p = SeedPin::parse("adaptive").unwrap().unwrap();
        assert_eq!(
            (p.case, p.policy.allocation),
            (None, SeedAllocation::Adaptive)
        );
        assert_eq!(p.policy.qos, SeedQos::Equal);
        for v in [
            "random",
            "random-qos",
            "whole",
            "whole-qos",
            "nearest",
            "nearest-qos",
            "oracle",
            "oracle-qos",
            "dc-only",
            "dc-only-qos:3",
            "admission",
            "admission-qos",
            "adaptive",
            "adaptive-qos",
        ] {
            assert_eq!(SeedPin::parse(v).unwrap().unwrap().to_string(), v);
        }
        for bad in ["fast", "oracle:", "oracle:x", "admission:4", "qos"] {
            assert!(SeedPin::parse(bad).is_err(), "{bad}");
        }
    }

    /// The seed's cable uplink at nine tenths pays for more slots of a
    /// sixteenth of the stream than its five linked viewers hold; a pin
    /// goes only to those, one slot per stream each at most.
    #[test]
    fn slots_are_what_the_seeds_upload_pays_for_at_its_linked_viewers() {
        let sites = vec![Site::new(City::NewYork, Access::Cable); 12];
        let mut s = wan(&sites, |i| i < 5);
        let pin = SeedPin::parse("whole").unwrap().unwrap();
        let up = Site::new(City::Chicago, Access::Cable).profile.up / 10 * 9;
        let slots = (up as f64 / (s.swarms[0].rate() / 16.0)) as usize;
        assert!(slots > 5 * 16, "{slots}");
        assert_eq!(pin.slots_in(&s), Some(slots));
        let hosts = pinned(&mut s, "whole", 1);
        let linked: Vec<HostIx> = (1..6).collect();
        assert_eq!(hosts.len(), 16);
        for h in &hosts {
            let mut h = h.clone();
            h.sort_unstable();
            assert_eq!(h, linked, "every linked viewer holds every stream");
        }
    }

    /// Linking the seed to every viewer adds a link to each it had none
    /// with, and the pin may then take any of them.
    #[test]
    fn a_seed_linked_to_every_viewer_may_pin_any() {
        let sites = vec![Site::new(City::NewYork, Access::Cable); 6];
        let mut s = wan(&sites, |i| i < 2);
        assert_eq!(pool(&s), vec![1, 2]);
        s.link_seed_to_viewers();
        assert_eq!(s.links.len(), 6);
        assert_eq!(pool(&s), (1..=6).collect::<Vec<_>>());
        s.link_seed_to_viewers();
        assert_eq!(s.links.len(), 6, "once each");
    }

    /// Whole copies first: every stream at the same viewers, then the
    /// remainder's streams at one more.
    #[test]
    fn whole_copies_then_a_part_copy() {
        let sites = vec![Site::new(City::NewYork, Access::Cable); 12];
        let mut s = wan(&sites, |_| true);
        let hosts = pinned(&mut s, "whole:37", 5);
        let first = &hosts[0];
        assert_eq!(first.len(), 3);
        for (i, h) in hosts.iter().enumerate() {
            assert_eq!(h.len(), 2 + usize::from(i < 5));
            assert_eq!(&first[..2], &h[..2]);
        }
        assert_eq!(hosts.iter().map(Vec::len).sum::<usize>(), 37);
    }

    /// Random pins each stream on its own draw; only random and whole draw
    /// from the run's rng.
    #[test]
    fn only_random_and_whole_draw() {
        let sites = vec![Site::new(City::NewYork, Access::Cable); 12];
        let mut s = wan(&sites, |_| true);
        let hosts = pinned(&mut s, "random:48", 9);
        assert!(hosts.iter().all(|h| h.len() == 3));
        assert!(hosts.iter().any(|h| h != &hosts[0]), "{hosts:?}");
        for knob in ["nearest:48", "oracle:48", "dc-only", "admission"] {
            let mut sites = sites.clone();
            sites.push(Site::new(City::NewYork, Access::Datacenter));
            let mut s = wan(&sites, |_| true);
            let pin = SeedPin::parse(knob).unwrap().unwrap();
            apply(&mut s, pin);
            let mut rng = det_rng(3);
            resolve(&mut s, &mut rng);
            assert_eq!(rng.next_u64(), det_rng(3).next_u64(), "{knob} drew");
        }
    }

    /// Nearest goes by the path to the seed: the viewers in the seed's city
    /// before those across the ocean.
    #[test]
    fn nearest_takes_the_shortest_paths() {
        let mut sites = vec![Site::new(City::London, Access::Cable); 4];
        sites.extend([Site::new(City::Chicago, Access::Cable); 3]);
        let mut s = wan(&sites, |_| true);
        let hosts = pinned(&mut s, "nearest:48", 1);
        for h in &hosts {
            let mut h = h.clone();
            h.sort_unstable();
            assert_eq!(h, vec![5, 6, 7]);
        }
    }

    /// The oracle takes the fastest honest uplinks, alternating regions
    /// among equals; dc-only takes the datacenter viewers.
    #[test]
    fn the_oracle_takes_the_fastest_honest_uplinks() {
        let sites = [
            Site::new(City::London, Access::Fiber),
            Site::new(City::Chicago, Access::Fiber),
            Site::new(City::Chicago, Access::Fiber),
            Site::new(City::Chicago, Access::Dsl),
            Site::new(City::NewYork, Access::Datacenter),
        ];
        let mut s = wan(&sites, |_| true);
        s.make_hostile(5, "liar", 0);
        let hosts = pinned(&mut s, "oracle:48", 1);
        let region = |h: HostIx| s.hosts[h].site.unwrap().city.region();
        assert!(hosts.iter().all(|h| h.len() == 3));
        assert!(!hosts[0].contains(&5), "no liar");
        assert_eq!(region(hosts[0][0]), Region::NorthAmerica);
        assert_eq!(region(hosts[0][1]), Region::Europe);
        assert!(!hosts[0].contains(&4), "the slowest uplink is left");
        let mut s = wan(&sites, |_| true);
        let hosts = pinned(&mut s, "dc-only-qos", 1);
        assert!(hosts.iter().all(|h| h == &vec![5]));
        assert_eq!(s.swarms[0].options.seed.qos, SeedQos::ByAllocation);
    }

    #[test]
    fn a_pin_needs_what_it_reads() {
        let sites = vec![Site::new(City::NewYork, Access::Cable); 3];
        let s = wan(&sites, |_| true);
        let dc = SeedPin::parse("dc-only").unwrap().unwrap();
        assert!(dc.check(&s).is_err());
        let mut pull = s.clone();
        pull.swarms[0].options.push_mode = PushMode::Off;
        assert!(
            SeedPin::parse("random")
                .unwrap()
                .unwrap()
                .check(&pull)
                .is_err()
        );
        assert!(
            SeedPin::parse("admission")
                .unwrap()
                .unwrap()
                .check(&pull)
                .is_ok()
        );
    }
}
