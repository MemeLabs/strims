// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! An uplink replayed packet by packet: the packets a node offered its
//! access link, fed one at a time to the model's uplink for that link,
//! each packet's fate as the model's own queue and shaper give it.
//!
//! The Docker testbed's captures hold what each node offered its modem
//! (strims-sim's testbed module reads them), so a testbed run's offers
//! replayed through its node's link spec give the model's departures and
//! drops for the very packets the testbed's shaper carried: the shaper
//! benched open loop, its inputs identical on both sides.
//!
//! An offer enters the uplink's queue at its time. The queue takes it
//! where its charge fits in what the queue has left, and drops it
//! otherwise ([`Network::up_admit`], the rule the sims' WAN scheduler
//! takes every message by); the shaper then sends it
//! ([`Network::occupy_up`]: the link's rate and steps, bucket and peak,
//! framing, slots, or a trace's delivery opportunities). Its backlog is
//! what the uplink held as it came ([`Network::up_held`]).

use std::sync::Arc;

use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};

use crate::access::{Access, Bucket, Site};
use crate::capacity::Capacity;
use crate::city::City;
use crate::network::{Network, Options};
use crate::testbed::Spec;

/// A packet a node offered its uplink: when it reached the uplink's
/// queue, and its IP bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Offer {
    pub at: Time,
    pub ip: usize,
}

/// What became of an [`Offer`] on the model's uplink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fate {
    /// When its last bit left the uplink; `None` where the queue dropped
    /// it.
    pub sent: Option<Time>,
    /// What its link charged it: its IP bytes and its framing.
    pub charged: usize,
    /// What the uplink held as it came, in the bytes its link charges.
    pub backlog: usize,
}

/// A node's uplink as the model shapes it.
#[derive(Clone, Debug)]
pub struct Uplink {
    /// Its site: its rate, bucket, framing and slots.
    pub site: Site,
    /// Its rate (bytes per second) from each time on, in time order.
    pub steps: Vec<(Time, u64)>,
    /// A trace link's delivery opportunities, in place of its rate, bucket
    /// and steps.
    pub capacity: Option<Arc<Capacity>>,
    /// The most its queue holds, in transmission time at its rate.
    pub queue: Duration,
}

impl Uplink {
    /// A node's uplink on link `spec` as the sims' twins build it: `up`
    /// bytes per second, reaching `top` at its highest (its start's or a
    /// step's), the run's `bucket` where the class takes one
    /// ([`Spec::shape`]), and a queue of `queue` at its rate. Fails where
    /// the testbed would.
    pub fn of(
        spec: &Spec,
        up: u64,
        top: u64,
        bucket: Option<Bucket>,
        queue: Duration,
    ) -> Result<Uplink, String> {
        let site = Site::new(City::Chicago, Access::Cable)
            .with_up(up)
            .with_bucket(bucket);
        Ok(Uplink {
            site: spec.shape(site, bucket, top)?,
            steps: Vec::new(),
            capacity: None,
            queue,
        })
    }
}

/// Each of `offers`' fate on `link`, in the order given, which is the
/// order its queue takes them in (times that never go back). Slots draw
/// from `rng`, as a network's do from the one it is built with.
pub fn replay(link: &Uplink, offers: &[Offer], rng: &mut DetRng) -> Vec<Fate> {
    let mut net = Network::new(
        &[link.site],
        Options {
            jitter: false,
            loss: false,
        },
        rng,
    );
    if !link.steps.is_empty() {
        net.set_up_steps(0, link.steps.clone());
    }
    if let Some(c) = &link.capacity {
        net.set_up_capacity(0, c.clone());
    }
    offers
        .iter()
        .map(|o| {
            let backlog = net.up_held(0, o.at);
            let charged = net.up_charge(0, o.ip, 1);
            let took = net.up_admit(0, o.at, link.queue, o.ip, 1);
            let sent = (took.packets > 0).then(|| net.occupy_up(o.at, 0, o.ip, 1));
            Fate {
                sent,
                charged,
                backlog,
            }
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use strims_base::rng::det_rng;

    use super::*;
    use crate::framing::Framing;
    use crate::wire::tx_time;

    /// 10 Mbit/s, in bytes per second.
    const RATE: u64 = 1_250_000;
    /// A full frame (1500 IP bytes and Ethernet's 14) at [`RATE`]: 1.2112
    /// ms.
    const FRAME_NS: i64 = 1_211_200;

    fn at(ns: i64) -> Time {
        Time::from_unix_nanos(1_000_000_000_000) + Duration::from_nanos(ns)
    }

    fn link(spec: &str, bucket: Option<Bucket>, queue_ms: i64) -> Uplink {
        Uplink::of(
            &Spec::parse(spec).unwrap(),
            RATE,
            RATE,
            bucket,
            Duration::from_millis(queue_ms),
        )
        .unwrap()
    }

    /// The fates of offers of (ns, IP bytes).
    fn run(link: &Uplink, offers: &[(i64, usize)]) -> Vec<Fate> {
        let offers: Vec<Offer> = offers
            .iter()
            .map(|&(ns, ip)| Offer { at: at(ns), ip })
            .collect();
        replay(link, &offers, &mut det_rng(1))
    }

    /// When each fate's last bit left, in ns after `at(0)`.
    fn sent_ns(fates: &[Fate]) -> Vec<Option<i64>> {
        fates
            .iter()
            .map(|f| f.sent.map(|t| (t - at(0)).as_nanos()))
            .collect()
    }

    #[test]
    fn a_tbf_sends_a_frame_at_its_rate_once_its_bucket_is_spent() {
        // tbf's bucket of two full frames lets two out at once; each next
        // waits for the rate to add its 1514 bytes.
        assert_eq!(tx_time(1514, RATE).as_nanos(), FRAME_NS);
        let tbf = link("tbf", Some(Bucket::of(3028)), 100);
        let f = run(&tbf, &[(0, 1500); 4]);
        assert_eq!(
            sent_ns(&f),
            [Some(0), Some(0), Some(FRAME_NS), Some(2 * FRAME_NS)]
        );
        assert!(f.iter().all(|x| x.charged == 1514));
        // What waited as each came: the first two left at once and the
        // third waits, so the fourth finds it.
        assert_eq!(
            f.iter().map(|x| x.backlog).collect::<Vec<_>>(),
            [0, 0, 0, 1514]
        );
    }

    #[test]
    fn a_bucket_refills_at_the_rate_while_the_link_idles() {
        let tbf = link("tbf", Some(Bucket::of(3028)), 100);
        // A frame's time after the bucket is spent, it holds a frame's
        // tokens: the third goes at once, the fourth waits a frame's time.
        // Long idle, it is full again: two go at once, the next a frame on.
        let idle = 10 * FRAME_NS;
        let f = run(
            &tbf,
            &[
                (0, 1500),
                (0, 1500),
                (FRAME_NS, 1500),
                (FRAME_NS, 1500),
                (idle, 1500),
                (idle, 1500),
                (idle, 1500),
            ],
        );
        assert_eq!(
            sent_ns(&f),
            [
                Some(0),
                Some(0),
                Some(FRAME_NS),
                Some(2 * FRAME_NS),
                Some(idle),
                Some(idle),
                Some(idle + FRAME_NS)
            ]
        );
    }

    #[test]
    fn a_peak_paces_what_the_bucket_lets_out() {
        let peak = 100 * 125_000;
        let tbf = Uplink::of(
            &Spec::parse("tbf").unwrap(),
            RATE,
            RATE,
            Some(Bucket {
                bytes: 3028,
                peak: Some(peak),
            }),
            Duration::from_millis(100),
        )
        .unwrap();
        // Each frame at 100 Mbit/s: 121.12 µs.
        let at_peak = tx_time(1514, peak).as_nanos();
        assert_eq!(at_peak, 121_120);
        let f = run(&tbf, &[(0, 1500); 2]);
        assert_eq!(sent_ns(&f), [Some(at_peak), Some(2 * at_peak)]);
    }

    #[test]
    fn a_full_queue_drops_what_it_has_no_room_for() {
        // 10 ms at 10 Mbit/s holds 12500 bytes: the bucket lets two frames
        // out at once, the queue takes 8 more (12112 bytes) and drops the
        // next two; a short packet still fits in what is left.
        let tbf = link("tbf", Some(Bucket::of(3028)), 10);
        let mut offers = vec![(0, 1500); 12];
        offers.push((0, 200));
        offers.push((20_000_000, 1500));
        let f = run(&tbf, &offers);
        let kept: Vec<bool> = f.iter().map(|x| x.sent.is_some()).collect();
        let mut want = vec![true; 10];
        want.extend([false, false, true, true]);
        assert_eq!(kept, want);
        assert_eq!(f[10].backlog, 8 * 1514);
        assert_eq!(f[10].charged, 1514);
        // A dropped packet takes nothing: the short one goes after the
        // tenth, and once the queue drains it takes a frame again.
        assert_eq!(
            sent_ns(&f)[12],
            Some(8 * FRAME_NS + tx_time(214, RATE).as_nanos())
        );
        assert_eq!(f[13].backlog, 0);
    }

    #[test]
    fn each_class_charges_its_framing() {
        // A 64-byte IP packet: Ethernet's 14 bytes under tbf, ADSL's 40 in
        // ATM cells (159 bytes), DOCSIS' 18 under cable, fibre's 18.
        for (spec, charge) in [
            ("tbf", 78),
            ("dsl", 159),
            ("cable,aqm=fifo,slot=off", 82),
            ("fiber", 82),
        ] {
            let l = link(spec, Some(Bucket::of(3028)), 100);
            let f = run(&l, &[(0, 64)]);
            assert_eq!(f[0].charged, charge, "{spec}");
            assert_eq!(
                f[0].charged,
                Spec::parse(spec).unwrap().framing().packet(64),
                "{spec}"
            );
        }
        // A DSL link's bucket is 3500 bytes: two full ADSL frames (1749
        // bytes of cells each) leave at once, the third once the rate has
        // added what the bucket lacked of it.
        assert_eq!(Framing::ADSL_PPPOE.packet(1500), 1749);
        let dsl = link("dsl", None, 100);
        let f = run(&dsl, &[(0, 1500); 3]);
        assert_eq!(
            sent_ns(&f),
            [
                Some(0),
                Some(0),
                Some(tx_time(3 * 1749 - 3500, RATE).as_nanos())
            ]
        );
    }

    #[test]
    fn a_step_takes_its_rate_from_its_time() {
        let mut tbf = link("tbf", Some(Bucket::of(3028)), 1000);
        tbf.steps = vec![(at(1_000_000), RATE / 2)];
        // Two leave at once; the third is 1 ms into its frame's 1.2112 ms
        // when the rate halves, so its last 0.2112 ms of bytes take twice
        // as long.
        let f = run(&tbf, &[(0, 1500); 3]);
        assert_eq!(sent_ns(&f)[2], Some(1_000_000 + 2 * 211_200));
    }

    #[test]
    fn slots_send_what_the_shaper_passed_at_the_next_slot() {
        // Slots of exactly 2 ms: a frame the shaper passes at once waits
        // for the slot that opens 2 ms after it is ready.
        let l = link("tbf,slot=2", Some(Bucket::of(3028)), 100);
        let f = run(&l, &[(0, 1500)]);
        assert_eq!(sent_ns(&f), [Some(2_000_000)]);
    }
}
