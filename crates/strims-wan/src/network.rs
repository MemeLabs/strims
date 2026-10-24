// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The path state of one run.
//!
//! Hosts are indexes into the sites a [`Network`] is built with. Each host
//! has an uplink and a downlink FIFO and a jitter process on each, and each
//! ordered pair of hosts (a direction) has a loss rng, an arrival clamp and
//! an optional latency override.
//!
//! - **Base latency** of a direction is the sender's last mile, half the
//!   city pair's RTT and the receiver's last mile ([`Network::one_way`]),
//!   unless [`Network::set_one_way`] overrides it.
//! - **Jitter.** Each end adds an offset on its uplink and one on its
//!   downlink. An offset holds for a [`JITTER_WINDOW`]: the offset of window
//!   `k` is an exponential draw with the profile's mean jitter from an rng
//!   seeded by the end and `k` alone, so it depends on the time it is read
//!   at and never on how many messages crossed before. The uplink offset is
//!   read when the message departs, the downlink offset when it reaches the
//!   receiver's access link.
//! - **Order.** An arrival never precedes the previous arrival in its
//!   direction, so jitter delays messages without reordering them.
//! - **Loss.** A packet crossing a direction is lost with probability
//!   `1 − (1 − p_sender)(1 − p_receiver)`, drawn once per message from that
//!   direction's rng. A lost datagram is dropped. A stream message that
//!   lost a packet arrives late by [`retransmission`], and the caller keeps
//!   later stream data behind it.
//! - **Rate steps.** An uplink runs at its site's rate unless
//!   [`Network::set_up_steps`] gives it rates from given times on. A
//!   message that is still serializing when a step comes pays the new rate
//!   for the rest of its bytes, so a step applies from its time, whatever
//!   the queue holds then.

use strims_base::rng::{DetRng, det_rng, fork};
use strims_base::time::{Duration, Time};

use crate::access::Site;
use crate::sample::{exponential, unit};
use crate::wire::tx_time;

/// How long an end's jitter offset holds.
pub const JITTER_WINDOW: Duration = Duration::from_millis(20);
/// The minimum retransmission timeout of a TCP sender.
pub const RTO_MIN: Duration = Duration::from_millis(200);

/// Which effects a network applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub jitter: bool,
    pub loss: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            jitter: true,
            loss: true,
        }
    }
}

/// What carries a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Carrier {
    /// A reliable, ordered stream (TCP): loss delays.
    Stream,
    /// A datagram (UDP): loss drops.
    Datagram,
}

/// One message's way across a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transit {
    /// When its last bit left the sender's uplink.
    pub depart: Time,
    /// When it reaches the receiver's downlink, before any retransmission.
    /// Never earlier than the previous arrival in its direction.
    pub arrival: Time,
    /// The extra delay of a stream message that lost a packet (zero
    /// otherwise).
    pub retransmit: Duration,
    /// A datagram that lost its packet.
    pub dropped: bool,
}

/// One host's counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EndStats {
    /// Wire bytes through the uplink.
    pub up_bytes: u64,
    /// Wire bytes through the downlink.
    pub down_bytes: u64,
    /// The longest a message waited for the uplink.
    pub up_queue_max: Duration,
    /// The longest a message waited for the downlink.
    pub down_queue_max: Duration,
    /// Stream messages it sent that lost a packet.
    pub retransmits: u64,
    /// Datagrams it sent that were lost.
    pub drops: u64,
}

/// The extra delay of a stream message of `packets` packets that lost one,
/// on a path of round-trip time `rtt`: duplicate acknowledgements recover
/// a message of three packets or more within a round trip; a two-packet
/// message waits for a tail-loss probe; a lone packet waits for the
/// retransmission timeout.
pub fn retransmission(packets: u32, rtt: Duration) -> Duration {
    match packets {
        0 | 1 => rtt * 2 + RTO_MIN,
        2 => rtt * 2,
        _ => rtt,
    }
}

/// When `wire` bytes that start through an uplink at `start` are through:
/// at `rate`, then from each of `steps` (in time order) on at its rate.
fn through(rate: u64, steps: &[(Time, u64)], start: Time, wire: usize) -> Time {
    let next = steps.partition_point(|&(t, _)| t <= start);
    let mut rate = next
        .checked_sub(1)
        .and_then(|i| steps.get(i))
        .map_or(rate, |&(_, r)| r)
        .max(1);
    let mut at = start;
    // Byte-nanoseconds, so a step's share of a message stays exact.
    let mut left = i128::try_from(wire).unwrap_or(i128::MAX) * 1_000_000_000;
    for &(t, r) in steps.iter().skip(next) {
        let fits = i128::from(rate) * i128::from((t - at).as_nanos());
        if left <= fits {
            break;
        }
        left -= fits;
        at = t;
        rate = r.max(1);
    }
    at + Duration::from_nanos(i64::try_from(left / i128::from(rate)).unwrap_or(i64::MAX))
}

/// A piecewise-constant jitter offset.
#[derive(Clone, Debug)]
struct Jitter {
    seed: u64,
    mean: Duration,
    window: Option<i64>,
    offset: Duration,
}

impl Jitter {
    fn new(seed: u64, mean: Duration) -> Jitter {
        Jitter {
            seed,
            mean,
            window: None,
            offset: Duration::ZERO,
        }
    }

    fn at(&mut self, t: Time) -> Duration {
        if self.mean <= Duration::ZERO {
            return Duration::ZERO;
        }
        let k = t.unix_nanos().div_euclid(JITTER_WINDOW.as_nanos());
        if self.window != Some(k) {
            let mut rng = det_rng(
                self.seed
                    .wrapping_add((k as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)),
            );
            self.window = Some(k);
            self.offset = exponential(&mut rng, self.mean);
        }
        self.offset
    }
}

#[derive(Clone, Debug)]
struct End {
    site: Site,
    /// The uplink's rate from each time on, in time order.
    up_steps: Vec<(Time, u64)>,
    up_free: Time,
    down_free: Time,
    up_jitter: Jitter,
    down_jitter: Jitter,
    stats: EndStats,
}

#[derive(Clone, Debug)]
struct Dir {
    rng: DetRng,
    last_arrival: Time,
    one_way: Option<Duration>,
}

/// The path state of one run (see the module docs).
#[derive(Clone, Debug)]
pub struct Network {
    options: Options,
    ends: Vec<End>,
    /// Row-major by (sender, receiver).
    dirs: Vec<Dir>,
}

impl Network {
    /// A network of hosts at `sites`. Every rng it uses derives from `rng`,
    /// in host order.
    pub fn new(sites: &[Site], options: Options, rng: &mut DetRng) -> Network {
        use rand_core::Rng;
        let ends = sites
            .iter()
            .map(|&site| {
                let up = rng.next_u64();
                let down = rng.next_u64();
                End {
                    site,
                    up_steps: Vec::new(),
                    up_free: Time::NIL,
                    down_free: Time::NIL,
                    up_jitter: Jitter::new(up, site.profile.jitter),
                    down_jitter: Jitter::new(down, site.profile.jitter),
                    stats: EndStats::default(),
                }
            })
            .collect();
        let dirs = (0..sites.len() * sites.len())
            .map(|_| Dir {
                rng: fork(rng),
                last_arrival: Time::NIL,
                one_way: None,
            })
            .collect();
        Network {
            options,
            ends,
            dirs,
        }
    }

    pub fn len(&self) -> usize {
        self.ends.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }

    pub fn options(&self) -> Options {
        self.options
    }

    pub fn site(&self, h: usize) -> Option<&Site> {
        self.ends.get(h).map(|e| &e.site)
    }

    fn end(&self, h: usize) -> &End {
        #[allow(clippy::indexing_slicing)] // invariant: hosts index `sites`
        &self.ends[h]
    }

    fn end_mut(&mut self, h: usize) -> &mut End {
        #[allow(clippy::indexing_slicing)] // invariant: hosts index `sites`
        &mut self.ends[h]
    }

    fn dir(&self, a: usize, b: usize) -> &Dir {
        let n = self.ends.len();
        #[allow(clippy::indexing_slicing)] // invariant: hosts index `sites`
        &self.dirs[a * n + b]
    }

    fn dir_mut(&mut self, a: usize, b: usize) -> &mut Dir {
        let n = self.ends.len();
        #[allow(clippy::indexing_slicing)] // invariant: hosts index `sites`
        &mut self.dirs[a * n + b]
    }

    /// When `h`'s uplink has sent everything handed to it so far.
    pub fn up_free(&self, h: usize) -> Time {
        self.end(h).up_free
    }

    /// When `h`'s downlink has delivered everything that reached it so far.
    pub fn down_free(&self, h: usize) -> Time {
        self.end(h).down_free
    }

    /// The base latency from `a` to `b`.
    pub fn one_way(&self, a: usize, b: usize) -> Duration {
        if let Some(d) = self.dir(a, b).one_way {
            return d;
        }
        let (sa, sb) = (&self.end(a).site, &self.end(b).site);
        sa.profile.last_mile + sa.city.rtt(sb.city) / 2 + sb.profile.last_mile
    }

    /// The base round-trip time between `a` and `b`.
    pub fn rtt(&self, a: usize, b: usize) -> Duration {
        self.one_way(a, b) + self.one_way(b, a)
    }

    /// Overrides the base latency between `a` and `b`, both ways; `None`
    /// restores the sites' latency.
    pub fn set_one_way(&mut self, a: usize, b: usize, d: Option<Duration>) {
        self.dir_mut(a, b).one_way = d;
        self.dir_mut(b, a).one_way = d;
    }

    /// From each of `steps`' times on, `h`'s uplink runs at that step's
    /// rate (bytes per second) instead of its site's.
    pub fn set_up_steps(&mut self, h: usize, mut steps: Vec<(Time, u64)>) {
        steps.sort_by_key(|&(t, _)| t);
        self.end_mut(h).up_steps = steps;
    }

    /// Sends `wire` bytes in `packets` packets from `a` to `b`, handed to
    /// `a`'s uplink at `sent`: they wait for the uplink, serialize at its
    /// rate, then cross the path ([`Network::transit`]). Calls for one
    /// sender come in the order its uplink serves them.
    pub fn send(
        &mut self,
        sent: Time,
        a: usize,
        b: usize,
        wire: usize,
        packets: u32,
        carrier: Carrier,
    ) -> Transit {
        let end = self.end_mut(a);
        let start = sent.max(end.up_free);
        end.up_free = through(end.site.profile.up, &end.up_steps, start, wire);
        end.stats.up_bytes = end.stats.up_bytes.saturating_add(wire as u64);
        end.stats.up_queue_max = end.stats.up_queue_max.max(start - sent);
        let depart = end.up_free;
        self.transit(depart, a, b, packets, carrier)
    }

    /// `wire` bytes of traffic that isn't between hosts, handed to `h`'s
    /// uplink at `at`: they wait for the uplink and serialize at its rate
    /// like a message, and count in no host's stats. Returns when their
    /// last bit left.
    pub fn occupy_up(&mut self, at: Time, h: usize, wire: usize) -> Time {
        let end = self.end_mut(h);
        let start = at.max(end.up_free);
        end.up_free = through(end.site.profile.up, &end.up_steps, start, wire);
        end.up_free
    }

    /// A message of `packets` packets crossing from `a` to `b`, its last
    /// bit having left `a`'s uplink at `depart`.
    pub fn transit(
        &mut self,
        depart: Time,
        a: usize,
        b: usize,
        packets: u32,
        carrier: Carrier,
    ) -> Transit {
        let options = self.options;
        let base = self.one_way(a, b);
        let reach = depart + base;
        let (up_jitter, down_jitter) = if options.jitter {
            (
                self.end_mut(a).up_jitter.at(depart),
                self.end_mut(b).down_jitter.at(reach),
            )
        } else {
            (Duration::ZERO, Duration::ZERO)
        };
        let p = 1.0 - (1.0 - self.end(a).site.profile.loss) * (1.0 - self.end(b).site.profile.loss);
        let rtt = self.rtt(a, b);
        let dir = self.dir_mut(a, b);
        let arrival = (reach + up_jitter + down_jitter).max(dir.last_arrival);
        dir.last_arrival = arrival;
        let lost = options.loss
            && p > 0.0
            && unit(&mut dir.rng)
                < 1.0 - (1.0 - p).powi(i32::try_from(packets).unwrap_or(i32::MAX));
        let mut t = Transit {
            depart,
            arrival,
            retransmit: Duration::ZERO,
            dropped: false,
        };
        if lost {
            let stats = &mut self.end_mut(a).stats;
            match carrier {
                Carrier::Stream => {
                    t.retransmit = retransmission(packets, rtt);
                    stats.retransmits += 1;
                }
                Carrier::Datagram => {
                    t.dropped = true;
                    stats.drops += 1;
                }
            }
        }
        t
    }

    /// `wire` bytes reaching `b`'s downlink at `at`: they wait for the
    /// downlink and serialize at its rate. Returns when the last bit is
    /// through. Calls for one receiver come in arrival order.
    pub fn receive(&mut self, at: Time, b: usize, wire: usize) -> Time {
        let end = self.end_mut(b);
        let start = at.max(end.down_free);
        end.down_free = start + tx_time(wire, end.site.profile.down);
        end.stats.down_bytes = end.stats.down_bytes.saturating_add(wire as u64);
        end.stats.down_queue_max = end.stats.down_queue_max.max(start - at);
        end.down_free
    }

    /// Forgets what `h`'s links and the directions to and from it hold, as
    /// when the host loses power: from `now`, its FIFOs are empty and no
    /// arrival waits behind an earlier one.
    pub fn reset(&mut self, h: usize, now: Time) {
        let end = self.end_mut(h);
        end.up_free = now;
        end.down_free = now;
        for x in 0..self.ends.len() {
            self.dir_mut(h, x).last_arrival = Time::NIL;
            self.dir_mut(x, h).last_arrival = Time::NIL;
        }
    }

    /// Forgets the arrival clamps between `a` and `b`, both ways, as when
    /// the path between them is cut.
    pub fn cut(&mut self, a: usize, b: usize) {
        self.dir_mut(a, b).last_arrival = Time::NIL;
        self.dir_mut(b, a).last_arrival = Time::NIL;
    }

    pub fn stats(&self, h: usize) -> EndStats {
        self.end(h).stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::Access;
    use crate::city::City;
    use crate::wire::{tcp_wire, udp_wire};

    fn t(ms: i64) -> Time {
        Time::from_unix_millis(1_700_000_000_000 + ms)
    }

    fn net(sites: &[Site], options: Options) -> Network {
        Network::new(sites, options, &mut det_rng(9))
    }

    const QUIET: Options = Options {
        jitter: false,
        loss: false,
    };

    #[test]
    fn one_way_adds_both_last_miles() {
        let n = net(
            &[
                Site::new(City::London, Access::Fiber),
                Site::new(City::NewYork, Access::Cable),
            ],
            QUIET,
        );
        assert_eq!(n.one_way(0, 1), Duration::from_millis(2 + 36 + 7));
        assert_eq!(n.rtt(0, 1), Duration::from_millis(90));
    }

    #[test]
    fn overrides_apply_both_ways_and_restore() {
        let mut n = net(
            &[
                Site::new(City::London, Access::Fiber),
                Site::new(City::Tokyo, Access::Fiber),
            ],
            QUIET,
        );
        let base = n.one_way(0, 1);
        n.set_one_way(0, 1, Some(Duration::from_millis(5)));
        assert_eq!(n.one_way(1, 0), Duration::from_millis(5));
        n.set_one_way(1, 0, None);
        assert_eq!(n.one_way(0, 1), base);
    }

    #[test]
    fn uplinks_and_downlinks_serialize_in_order() {
        let up = 125_000; // 1 Mbit/s
        let sites = [
            Site::new(City::Frankfurt, Access::Fiber).with_up(up),
            Site::new(City::Frankfurt, Access::Fiber).with_down(up),
            Site::new(City::Frankfurt, Access::Fiber),
        ];
        let mut n = net(&sites, QUIET);
        let base = n.one_way(0, 1);
        let a = n.send(t(0), 0, 1, 1250, 1, Carrier::Stream);
        let b = n.send(t(0), 0, 1, 1250, 1, Carrier::Stream);
        assert_eq!(a.depart, t(10));
        assert_eq!(b.depart, t(20));
        assert_eq!(b.arrival, t(20) + base);
        assert_eq!(n.stats(0).up_queue_max, Duration::from_millis(10));
        // Host 1's downlink takes 10 ms per 1250 bytes, whoever sends them.
        let x = n.receive(t(100), 1, 1250);
        let y = n.receive(t(100), 1, 1250);
        assert_eq!((x, y), (t(110), t(120)));
        assert_eq!(n.stats(1).down_queue_max, Duration::from_millis(10));
        assert_eq!(n.stats(1).down_bytes, 2500);
        n.reset(1, t(105));
        assert_eq!(n.receive(t(105), 1, 1250), t(115));
    }

    #[test]
    fn uplink_steps_apply_from_their_time() {
        let up = 125_000; // 1 Mbit/s: 1250 bytes in 10 ms
        let sites = [
            Site::new(City::Frankfurt, Access::Fiber).with_up(up),
            Site::new(City::Frankfurt, Access::Fiber),
        ];
        let mut n = net(&sites, QUIET);
        n.set_up_steps(0, vec![(t(100), up), (t(15), up / 2)]);
        let mut depart = |at| n.send(t(at), 0, 1, 1250, 1, Carrier::Stream).depart;
        assert_eq!(depart(0), t(10));
        // 5 ms at the full rate carry half, the half rate the rest in 10.
        assert_eq!(depart(0), t(25));
        assert_eq!(depart(0), t(45));
        // Back to the full rate at 100 ms, mid-message.
        assert_eq!(depart(90), t(105));
        assert_eq!(depart(200), t(210));
    }

    #[test]
    fn other_traffic_shares_the_uplink_and_counts_nowhere() {
        let up = 125_000;
        let sites = [
            Site::new(City::Frankfurt, Access::Fiber).with_up(up),
            Site::new(City::Frankfurt, Access::Fiber),
        ];
        let mut n = net(&sites, QUIET);
        assert_eq!(n.occupy_up(t(0), 0, 1250), t(10));
        assert_eq!(n.send(t(0), 0, 1, 1250, 1, Carrier::Stream).depart, t(20));
        assert_eq!(n.stats(0).up_bytes, 1250);
    }

    #[test]
    fn without_steps_an_uplink_serializes_at_its_sites_rate() {
        for rate in [1, 999, 125_000, 2_500_000, 125_000_000] {
            for wire in [0, 1, 53, 1500, 65_535, 1 << 30] {
                assert_eq!(through(rate, &[], t(3), wire), t(3) + tx_time(wire, rate));
            }
        }
    }

    #[test]
    fn arrivals_keep_order_under_jitter() {
        let sites = [
            Site::new(City::Mumbai, Access::Lte),
            Site::new(City::Singapore, Access::Starlink),
        ];
        let mut n = net(
            &sites,
            Options {
                jitter: true,
                loss: false,
            },
        );
        let base = n.one_way(0, 1);
        let mut last = Time::NIL;
        let mut spread = Duration::ZERO;
        for i in 0..5_000 {
            let tr = n.transit(t(i), 0, 1, 1, Carrier::Datagram);
            assert!(tr.arrival >= last);
            assert!(tr.arrival >= t(i) + base);
            spread = spread.max(tr.arrival - t(i) - base);
            last = tr.arrival;
        }
        assert!(spread > Duration::from_millis(20), "{spread}");
    }

    #[test]
    fn jitter_depends_only_on_time() {
        let mut a = Jitter::new(5, Duration::from_millis(10));
        let mut b = Jitter::new(5, Duration::from_millis(10));
        let times: Vec<Time> = (0..200).map(|i| t(i * 7)).collect();
        let forward: Vec<Duration> = times.iter().map(|&x| a.at(x)).collect();
        let backward: Vec<Duration> = times.iter().rev().map(|&x| b.at(x)).collect();
        assert!(forward.iter().eq(backward.iter().rev()));
        // Offsets hold over a window and change between windows.
        assert_eq!(a.at(t(0)), a.at(t(19)));
        assert!((0..50).any(|k| a.at(t(20 * k)) != a.at(t(20 * k + 20))));
    }

    #[test]
    fn loss_matches_its_rate() {
        let sites = [
            Site::new(City::Dallas, Access::Lte),
            Site::new(City::Dallas, Access::Starlink),
        ];
        let mut n = net(
            &sites,
            Options {
                jitter: false,
                loss: true,
            },
        );
        let p = 1.0 - (1.0 - 0.002) * (1.0 - 0.003);
        let count = 100_000;
        let (wire, packets) = udp_wire(1200);
        let drops = (0..count)
            .filter(|&i| n.send(t(i), 0, 1, wire, packets, Carrier::Datagram).dropped)
            .count() as f64;
        let mean = p * count as f64;
        let sigma = (mean * (1.0 - p)).sqrt();
        assert!((drops - mean).abs() < 3.0 * sigma, "{drops} vs {mean}");
        assert_eq!(n.stats(0).drops, drops as u64);
        assert_eq!(n.stats(1).drops, 0);

        // A large stream message is hit more often, and delayed, never
        // dropped.
        let (wire, packets) = tcp_wire(65_535);
        let mut hits = 0;
        for i in 0..2_000 {
            let tr = n.send(t(200_000 + i * 100), 0, 1, wire, packets, Carrier::Stream);
            assert!(!tr.dropped);
            if tr.retransmit > Duration::ZERO {
                assert_eq!(tr.retransmit, n.rtt(0, 1));
                hits += 1;
            }
        }
        let expected = 2_000.0 * (1.0 - (1.0 - p).powi(46));
        assert!(
            (f64::from(hits) - expected).abs() < 0.25 * expected,
            "{hits} vs {expected}"
        );
    }

    #[test]
    fn retransmissions_by_message_size() {
        let rtt = Duration::from_millis(100);
        assert_eq!(retransmission(1, rtt), Duration::from_millis(400));
        assert_eq!(retransmission(2, rtt), Duration::from_millis(200));
        assert_eq!(retransmission(3, rtt), rtt);
    }

    #[test]
    fn same_seed_same_transits() {
        let sites: Vec<Site> = City::ALL
            .iter()
            .zip(Access::ALL.iter().cycle())
            .map(|(&c, &a)| Site::new(c, a))
            .collect();
        let run = |seed| {
            let mut n = Network::new(&sites, Options::default(), &mut det_rng(seed));
            (0..3_000)
                .map(|i: i64| {
                    let a = (i as usize * 7) % sites.len();
                    let b = (i as usize * 11 + 1) % sites.len();
                    n.send(t(i), a, b, 1500, 1, Carrier::Datagram)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(run(1), run(1));
        assert_ne!(run(1), run(2));
    }
}
