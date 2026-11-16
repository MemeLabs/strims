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
//!   unless [`Network::set_one_way`] (both ways) or
//!   [`Network::set_one_way_from`] (one) overrides it.
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
//!   direction's rng, each end's `p` its profile's loss or, while the end
//!   is in one of its [`Bursts`] (at the message's departure from the
//!   sender's uplink, or its reach of the receiver's access link), the
//!   burst's. Whether a time falls in a burst depends on the end and the
//!   time alone, as jitter does: a phased end's bursts start on their
//!   phase, a random one's in slots of [`BURST_MAX`], each slot's burst
//!   drawn from an rng seeded by the end and the slot. A lost datagram is dropped. A stream message that
//!   lost a packet arrives late by [`retransmission`], and the caller keeps
//!   later stream data behind it. Traffic to a peer outside the network
//!   (a lossless one) draws from the direction from its host to itself,
//!   which no message crosses ([`Network::lost_off_net`]).
//! - **Rate steps.** An uplink runs at its site's rate unless
//!   [`Network::set_up_steps`] gives it rates from given times on. A
//!   message that is still serializing when a step comes pays the new rate
//!   for the rest of its bytes, so a step applies from its time, whatever
//!   the queue holds then.
//! - **Shaper bucket.** An uplink whose site has a [`Bucket`] holds tokens,
//!   in bytes, that fill at the uplink's rate (its steps' from their
//!   times) up to the bucket's size and that the bytes it sends take. A
//!   message that starts through it with `T` tokens there has its last bit
//!   out `max(0, wire − T)` over the rate later, or `wire` over the
//!   bucket's peak if that is later, and leaves the bucket what that time
//!   refilled less what it took. This is the last packet's dequeue in
//!   Linux's tbf, whose packets of a burst go once the tokens cover each
//!   whole: packet `k` of a burst at `T` tokens leaves when the rate has
//!   added `C_k − T` (`C_k` the burst's bytes through it). The queue holds
//!   the bytes not yet out ([`Network::up_backlog`]), a byte limit as tbf's
//!   is. Without a bucket an uplink drains at its rate alone, the model of
//!   a link whose shaper lets no burst through. A rate step leaves the
//!   tokens as they are (tbf's `change` refills its bucket; a real link's
//!   rate changes don't).
//! - **Framing.** An uplink charges each message what its site's
//!   [`Framing`] says its packets cost past their IP bytes, its full TCP
//!   packets and the rest ([`Framing::message`]): its serialization, its
//!   bucket's tokens, its queue's bytes and its stats' `up_bytes` count
//!   those. [`Framing::NONE`] charges the IP bytes alone.
//! - **Slots.** An uplink whose site has [`Slots`] sends what its shaper
//!   passed when the next slot opens ([`SlotState::depart`]): a slot opens
//!   `min` to `max` (uniformly, from an rng forked at the network's
//!   construction for each such site) after the first message ready past
//!   the last, and every message ready before it rides it, as a cable
//!   modem's grants send what its shaper passed. The testbed's link
//!   classes hold them there with a netem of slots alone after the shaper.
//!   The queue's limit counts what waits for the shaper, not for a slot,
//!   and the shaper takes the next message once it has passed the last.
//! - **Capacity.** An uplink given a [`Capacity`] ([`Network::set_up_capacity`])
//!   sends in its delivery opportunities, in place of its rate, bucket and
//!   steps: a message's last bit leaves at the opportunity where the
//!   uplink's [`Cursor`] ends once it has taken the message's bytes, capacity
//!   that passed while nothing waited lost (`hack/testbed/tracelink.c`'s
//!   model). Its queue's bytes are what the cursor holds past the
//!   opportunities that came, counted at the site's rate
//!   ([`Network::up_backlog`], [`Network::up_rate`]), as `run.sh` sets a
//!   trace link's byte limit from the node's rate and queue.

use std::collections::VecDeque;
use std::sync::Arc;

use strims_base::rng::{DetRng, det_rng, fork};
use strims_base::time::{Duration, Time};

use crate::access::{BURST_MAX, Bucket, Bursts, Site};
use crate::capacity::{Capacity, Cursor};
use crate::framing::Framing;
use crate::sample::{exponential, unit};
use crate::slots::{SlotState, Slots};
use crate::wire::{small_packets, tx_time};

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
    /// A datagram the path never loses: one whose contents a simulation
    /// can't reproduce run to run, so that its loss would make the run
    /// irreproducible (a DTLS handshake record).
    Lossless,
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
    /// Bytes through the uplink, as its link charged them (its framing
    /// in).
    pub up_bytes: u64,
    /// The IP bytes of the messages through the uplink, before its
    /// framing.
    pub up_ip_bytes: u64,
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
    /// Packets through the uplink, and the small ones among them
    /// ([`crate::wire::small_packets`]).
    pub up_packets: u64,
    pub small_packets: u64,
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

/// Nanoseconds in a second: bytes times this are byte-nanoseconds, the
/// unit an uplink's serialization and its bucket's tokens are kept in, so
/// a step's share of a message stays exact.
const NS: i128 = 1_000_000_000;

/// When `wire` bytes that start through an uplink at `start` are through:
/// at `rate`, then from each of `steps` (in time order) on at its rate.
fn through(rate: u64, steps: &[(Time, u64)], start: Time, wire: usize) -> Time {
    drain(
        rate,
        steps,
        start,
        i128::try_from(wire).unwrap_or(i128::MAX) * NS,
    )
}

/// The rate an uplink of `rate` with `steps` runs at at `at`.
fn rate_at(rate: u64, steps: &[(Time, u64)], at: Time) -> u64 {
    let next = steps.partition_point(|&(t, _)| t <= at);
    next.checked_sub(1)
        .and_then(|i| steps.get(i))
        .map_or(rate, |&(_, r)| r)
}

/// What an uplink of `rate` with `steps` carries from `from` to `to`, in
/// byte-nanoseconds (none when `to` isn't later).
fn carried(rate: u64, steps: &[(Time, u64)], from: Time, to: Time) -> i128 {
    if to <= from {
        return 0;
    }
    let next = steps.partition_point(|&(t, _)| t <= from);
    let mut rate = rate_at(rate, steps, from).max(1);
    let (mut at, mut sum) = (from, 0i128);
    for &(t, r) in steps.iter().skip(next) {
        if t >= to {
            break;
        }
        sum += i128::from(rate) * i128::from((t - at).as_nanos());
        at = t;
        rate = r.max(1);
    }
    sum + i128::from(rate) * i128::from((to - at).as_nanos())
}

/// When `left` byte-nanoseconds that start through an uplink at `start`
/// are through ([`through`]).
fn drain(rate: u64, steps: &[(Time, u64)], start: Time, mut left: i128) -> Time {
    let next = steps.partition_point(|&(t, _)| t <= start);
    let mut rate = next
        .checked_sub(1)
        .and_then(|i| steps.get(i))
        .map_or(rate, |&(_, r)| r)
        .max(1);
    let mut at = start;
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
    /// The messages handed to the uplink that may not have left it yet:
    /// when each started through it, when its last bit left, and its wire
    /// bytes.
    up_spans: VecDeque<(Time, Time, usize)>,
    /// With a bucket, the tokens it held at `tokens_at`, in
    /// byte-nanoseconds.
    tokens: i128,
    tokens_at: Time,
    /// With slots, their state, and when the last message left them.
    slots: Option<SlotState>,
    slot_last: Time,
    /// With a capacity schedule, it and the uplink's place in it.
    capacity: Option<(Arc<Capacity>, Cursor)>,
    down_free: Time,
    up_jitter: Jitter,
    down_jitter: Jitter,
    bursts: Option<BurstState>,
    stats: EndStats,
}

/// An end's [`Bursts`] and the slots it read last.
#[derive(Clone, Debug)]
struct BurstState {
    bursts: Bursts,
    seed: u64,
    /// The last two slots read and their bursts, as unix nanoseconds.
    seen: [(i64, Option<(i64, i64)>); 2],
}

impl BurstState {
    fn new(bursts: Bursts, seed: u64) -> BurstState {
        BurstState {
            bursts,
            seed,
            seen: [(i64::MIN, None); 2],
        }
    }

    /// A slot's length and offset in unix time: a phased end's period, a
    /// random one's [`BURST_MAX`].
    fn slots(&self) -> (i64, i64) {
        match self.bursts.phase {
            Some(phase) => (self.bursts.every.as_nanos().max(1), phase.as_nanos()),
            None => (BURST_MAX.as_nanos(), 0),
        }
    }

    /// Slot `k`'s burst, if it has one: a phased slot's at its start, a
    /// random slot's with probability [`BURST_MAX`] over `every`, anywhere
    /// in it.
    fn burst(&mut self, k: i64) -> Option<(i64, i64)> {
        if let Some(&(_, b)) = self.seen.iter().find(|&&(slot, _)| slot == k) {
            return b;
        }
        let (slot, phase) = self.slots();
        let mut rng = det_rng(
            self.seed
                .wrapping_add((k as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)),
        );
        let start = match self.bursts.phase {
            Some(_) => Some(k * slot + phase),
            None => (unit(&mut rng) < slot as f64 / self.bursts.every.as_nanos().max(1) as f64)
                .then(|| k * slot + (unit(&mut rng) * slot as f64) as i64),
        };
        let b = start.map(|start| {
            let length = exponential(&mut rng, self.bursts.length)
                .min(BURST_MAX)
                .as_nanos()
                .min(slot);
            (start, start + length)
        });
        self.seen = [self.seen[1], (k, b)];
        b
    }

    /// Whether `t` falls in a burst: one of its slot's or the slot's
    /// before, which may last into it.
    fn covers(&mut self, t: Time) -> bool {
        let ns = t.unix_nanos();
        let (slot, phase) = self.slots();
        let k = (ns - phase).div_euclid(slot);
        [k, k - 1].into_iter().any(|j| {
            self.burst(j)
                .is_some_and(|(start, end)| start <= ns && ns < end)
        })
    }
}

impl End {
    /// The probability that the end loses a packet at `t`.
    fn loss_at(&mut self, t: Time) -> f64 {
        let between = self.site.profile.loss;
        match &mut self.bursts {
            Some(b) => {
                if b.covers(t) {
                    b.bursts.loss.max(between)
                } else {
                    between
                }
            }
            None => between,
        }
    }

    /// Records a message of `wire` bytes handed to the uplink at `at` that
    /// starts through it at `start` and ends at `up_free`, and forgets
    /// those that had left by `at`.
    fn span(&mut self, at: Time, start: Time, wire: usize) {
        while self
            .up_spans
            .front()
            .is_some_and(|&(_, done, _)| done <= at)
        {
            self.up_spans.pop_front();
        }
        self.up_spans.push_back((start, self.up_free, wire));
    }

    /// When a message of `wire` IP bytes in `packets` packets, handed to
    /// the uplink at `sent`, starts through its shaper, once everything
    /// before it has passed the shaper, and the bytes it charges
    /// ([`Framing::message`]).
    fn admit(&mut self, sent: Time, wire: usize, packets: u32) -> (Time, usize) {
        let charged = self.site.profile.framing.message(wire, packets);
        (sent.max(self.up_free), charged)
    }

    /// When a message the shaper passed at `passed` leaves: when its slot
    /// opens, if the uplink has slots, the slot stage looking at it as it
    /// comes or once the message before it has left.
    fn grant(&mut self, passed: Time) -> Time {
        match &mut self.slots {
            Some(s) => {
                let depart = s.depart(passed, passed.max(self.slot_last));
                self.slot_last = depart;
                depart
            }
            None => passed,
        }
    }

    /// When `wire` bytes that start through the uplink at `start`, no
    /// earlier than everything before them has left, are through: at the
    /// rate, or with a bucket what its tokens cover at once (or at its
    /// peak) and the rest at the rate, or in its capacity's opportunities
    /// (see the module docs).
    fn serve(&mut self, start: Time, wire: usize) -> Time {
        if let Some((cap, cursor)) = &mut self.capacity {
            return cursor.take(cap, start, wire);
        }
        let rate = self.site.profile.up;
        let Some(Bucket { bytes, peak }) = self.site.profile.bucket else {
            return through(rate, &self.up_steps, start, wire);
        };
        let cap = i128::from(bytes) * NS;
        let have = (self.tokens + carried(rate, &self.up_steps, self.tokens_at, start)).min(cap);
        let need = i128::try_from(wire).unwrap_or(i128::MAX) * NS;
        let mut done = drain(rate, &self.up_steps, start, (need - have).max(0));
        if let Some(peak) = peak {
            done = done.max(start + tx_time(wire, peak));
        }
        self.tokens = (have - need + carried(rate, &self.up_steps, start, done)).clamp(0, cap);
        self.tokens_at = done;
        done
    }
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
                let bursts = site
                    .profile
                    .bursts
                    .map(|b| BurstState::new(b, rng.next_u64()));
                let slots = site.profile.slots.map(|s| SlotState::new(s, fork(rng)));
                End {
                    site,
                    up_steps: Vec::new(),
                    up_free: Time::NIL,
                    up_spans: VecDeque::new(),
                    tokens: i128::from(site.profile.bucket.map_or(0, |b| b.bytes)) * NS,
                    tokens_at: Time::NIL,
                    slots,
                    slot_last: Time::NIL,
                    capacity: None,
                    down_free: Time::NIL,
                    up_jitter: Jitter::new(up, site.profile.jitter),
                    down_jitter: Jitter::new(down, site.profile.jitter),
                    bursts,
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

    /// When `h`'s uplink's shaper has passed everything handed to it so
    /// far (its slots, if it has any, may hold some of it later).
    pub fn up_free(&self, h: usize) -> Time {
        self.end(h).up_free
    }

    /// `h`'s uplink rate at `at`, in bytes per second: with a capacity
    /// schedule, its site's, which its queue's bytes count at.
    pub fn up_rate(&self, h: usize, at: Time) -> u64 {
        let end = self.end(h);
        if end.capacity.is_some() {
            return end.site.profile.up;
        }
        rate_at(end.site.profile.up, &end.up_steps, at)
    }

    /// What `h`'s uplink charges for a message of `wire` IP bytes in
    /// `packets` packets ([`Framing::message`]).
    pub fn up_charge(&self, h: usize, wire: usize, packets: u32) -> usize {
        self.end(h).site.profile.framing.message(wire, packets)
    }

    /// `h`'s uplink's framing.
    pub fn up_framing(&self, h: usize) -> Framing {
        self.end(h).site.profile.framing
    }

    /// `h`'s uplink sends in `capacity`'s delivery opportunities from now
    /// on, in place of its rate, bucket and steps.
    pub fn set_up_capacity(&mut self, h: usize, capacity: Arc<Capacity>) {
        self.end_mut(h).capacity = Some((capacity, Cursor::default()));
    }

    /// `h`'s uplink's capacity schedule, if it has one.
    pub fn up_capacity(&self, h: usize) -> Option<&Capacity> {
        self.end(h).capacity.as_ref().map(|(c, _)| c.as_ref())
    }

    /// `h`'s uplink's slots, if it has any.
    pub fn up_slots(&self, h: usize) -> Option<Slots> {
        self.end(h).site.profile.slots
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
        self.set_one_way_from(a, b, d);
        self.set_one_way_from(b, a, d);
    }

    /// Overrides the base latency from `a` to `b` alone, as a delay on
    /// `a`'s egress does; `None` restores the sites' latency.
    pub fn set_one_way_from(&mut self, a: usize, b: usize, d: Option<Duration>) {
        self.dir_mut(a, b).one_way = d;
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
        let (start, charged) = end.admit(sent, wire, packets);
        end.up_free = end.serve(start, charged);
        end.span(sent, start, charged);
        end.stats.up_bytes = end.stats.up_bytes.saturating_add(charged as u64);
        end.stats.up_ip_bytes = end.stats.up_ip_bytes.saturating_add(wire as u64);
        end.stats.up_packets = end.stats.up_packets.saturating_add(u64::from(packets));
        end.stats.small_packets = end
            .stats
            .small_packets
            .saturating_add(u64::from(small_packets(wire, packets)));
        end.stats.up_queue_max = end.stats.up_queue_max.max(start - sent);
        let depart = end.grant(end.up_free);
        self.transit(depart, a, b, packets, carrier)
    }

    /// `wire` bytes in `packets` packets of traffic that isn't between
    /// hosts, handed to `h`'s uplink at `at`: they wait for the uplink and
    /// serialize at its rate like a message, and count in no host's stats.
    /// Returns when their last bit left.
    pub fn occupy_up(&mut self, at: Time, h: usize, wire: usize, packets: u32) -> Time {
        let end = self.end_mut(h);
        let (start, charged) = end.admit(at, wire, packets);
        end.up_free = end.serve(start, charged);
        end.span(at, start, charged);
        end.grant(end.up_free)
    }

    /// How long `h`'s uplink takes from `at` to send what it holds,
    /// counting the message it is sending whole: an access queue frees a
    /// message's bytes once its last bit has left (Linux's tbf frees a
    /// GSO buffer once its tokens are in), so what it has room for grows
    /// in steps of what it sends. With a bucket, the bytes it holds at the
    /// rate at `at`: the queue's limit is in bytes, which the bucket's
    /// tokens send sooner than the rate does.
    pub fn up_backlog(&mut self, h: usize, at: Time) -> Duration {
        let end = self.end_mut(h);
        while end.up_spans.front().is_some_and(|&(_, done, _)| done <= at) {
            end.up_spans.pop_front();
        }
        if let Some((cap, cursor)) = &end.capacity {
            let held = usize::try_from(cursor.held(cap, at)).unwrap_or(usize::MAX);
            return tx_time(held, end.site.profile.up);
        }
        if end.site.profile.bucket.is_some() {
            let held: usize = end.up_spans.iter().map(|&(_, _, wire)| wire).sum();
            return tx_time(held, rate_at(end.site.profile.up, &end.up_steps, at));
        }
        match end.up_spans.front() {
            Some(&(start, _, _)) if start <= at => end.up_free - start,
            _ => (end.up_free - at).max(Duration::ZERO),
        }
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
        let p = if options.loss {
            1.0 - (1.0 - self.end_mut(a).loss_at(depart)) * (1.0 - self.end_mut(b).loss_at(reach))
        } else {
            0.0
        };
        let rtt = self.rtt(a, b);
        let dir = self.dir_mut(a, b);
        let arrival = (reach + up_jitter + down_jitter).max(dir.last_arrival);
        dir.last_arrival = arrival;
        let lost = options.loss
            && carrier != Carrier::Lossless
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
                Carrier::Datagram | Carrier::Lossless => {
                    t.dropped = true;
                    stats.drops += 1;
                }
            }
        }
        t
    }

    /// Whether a message of `packets` packets from `h` to a lossless peer
    /// outside the network, leaving `h`'s uplink at `at`, loses a packet on
    /// `h`'s access link.
    pub fn lost_off_net(&mut self, at: Time, h: usize, packets: u32) -> bool {
        if !self.options.loss {
            return false;
        }
        let p = self.end_mut(h).loss_at(at);
        if p <= 0.0 {
            return false;
        }
        let dir = self.dir_mut(h, h);
        unit(&mut dir.rng) < 1.0 - (1.0 - p).powi(i32::try_from(packets).unwrap_or(i32::MAX))
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
        end.up_spans.clear();
        // What it forgets takes no tokens past `now`, and no capacity.
        end.tokens_at = end.tokens_at.min(now);
        end.slot_last = end.slot_last.min(now);
        if let Some((cap, cursor)) = &mut end.capacity {
            cursor.forget(cap, now);
        }
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
        // One direction alone, as an egress delay sets it.
        n.set_one_way_from(0, 1, Some(Duration::from_millis(20)));
        n.set_one_way_from(1, 0, Some(Duration::from_millis(10)));
        assert_eq!(n.one_way(0, 1), Duration::from_millis(20));
        assert_eq!(n.one_way(1, 0), Duration::from_millis(10));
        assert_eq!(n.rtt(0, 1), Duration::from_millis(30));
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
        assert_eq!(n.occupy_up(t(0), 0, 1250, 1), t(10));
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
            Site::new(City::Dallas, Access::Lte).with_loss(0.002),
            Site::new(City::Dallas, Access::Starlink).with_loss(0.003),
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

        // A lossless datagram crosses the same path whole.
        let lossless = (0..count)
            .filter(|&i| {
                n.send(t(count + i), 0, 1, wire, packets, Carrier::Lossless)
                    .dropped
            })
            .count();
        assert_eq!(lossless, 0);
        assert_eq!(n.stats(0).drops, drops as u64);

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
    fn an_uplink_frees_a_message_once_it_has_left() {
        let sites = [
            Site::new(City::Dallas, Access::Fiber).with_up(1_000_000),
            Site::new(City::Dallas, Access::Fiber),
        ];
        let mut n = net(
            &sites,
            Options {
                jitter: false,
                loss: false,
            },
        );
        // Two 3000-byte messages at 1 MB/s: 0-3 ms and 3-6 ms.
        n.send(t(0), 0, 1, 3000, 2, Carrier::Stream);
        n.occupy_up(t(0), 0, 3000, 2);
        let ms = Duration::from_millis;
        // The first counts whole until its last bit is out at 3 ms.
        assert_eq!(n.up_backlog(0, t(1)), ms(6));
        assert_eq!(n.up_backlog(0, t(2)), ms(6));
        assert_eq!(n.up_backlog(0, t(3)), ms(3));
        assert_eq!(n.up_backlog(0, t(5)), ms(3));
        assert_eq!(n.up_backlog(0, t(6)), Duration::ZERO);
        assert_eq!(n.up_backlog(0, t(9)), Duration::ZERO);
    }

    #[test]
    fn off_net_traffic_loses_at_its_hosts_rate() {
        let sites = [
            Site::new(City::Dallas, Access::Starlink).with_loss(0.003),
            Site::new(City::Dallas, Access::Lte).with_loss(0.002),
        ];
        let lossy = Options {
            jitter: false,
            loss: true,
        };
        let mut n = net(&sites, lossy);
        let mut m = net(&sites, lossy);
        let p = 1.0 - (1.0 - 0.003_f64).powi(2);
        let count: u32 = 100_000;
        let mut lost: u32 = 0;
        for i in 0..i64::from(count) {
            if n.lost_off_net(t(i), 0, 2) {
                lost += 1;
            }
            // The directions between hosts draw as if it weren't there.
            let (a, b) = (
                n.send(t(i), 0, 1, 1500, 1, Carrier::Datagram),
                m.send(t(i), 0, 1, 1500, 1, Carrier::Datagram),
            );
            assert_eq!(a.dropped, b.dropped);
        }
        let mean = p * f64::from(count);
        let sigma = (mean * (1.0 - p)).sqrt();
        assert!(
            (f64::from(lost) - mean).abs() < 3.0 * sigma,
            "{lost} vs {mean}"
        );
        let mut quiet = net(
            &sites,
            Options {
                jitter: false,
                loss: false,
            },
        );
        assert!((0..1_000).all(|i| !quiet.lost_off_net(t(i), 0, 2)));
    }

    /// Datagrams of one packet from a bursty `site` to a lossless one, one
    /// a millisecond for `secs`: whether each was lost.
    fn losses(site: Site, secs: i64) -> Vec<bool> {
        let sites = [site, Site::new(City::Dallas, Access::Fiber).with_loss(0.0)];
        let mut n = net(
            &sites,
            Options {
                jitter: false,
                loss: true,
            },
        );
        (0..secs * 1000)
            .map(|i| n.send(t(i), 0, 1, 1200, 1, Carrier::Datagram).dropped)
            .collect()
    }

    #[test]
    fn bursty_links_lose_their_mean_in_runs() {
        for access in [Access::Lte, Access::Starlink] {
            let site = Site::new(City::Dallas, access);
            let lost = losses(site, 600);
            let rate = lost.iter().filter(|&&l| l).count() as f64 / lost.len() as f64;
            let mean = site
                .profile
                .bursts
                .map_or(0.0, |b| b.mean_loss(site.profile.loss));
            assert!(
                (rate - mean).abs() < mean / 4.0,
                "{access}: {rate} against {mean}"
            );
            // A loss follows a loss far more often than the mean.
            let pairs = lost.windows(2).filter(|w| w.first() == Some(&true)).count() as f64;
            let runs = lost
                .windows(2)
                .filter(|w| matches!(w, [true, true]))
                .count() as f64;
            assert!(
                runs / pairs > 20.0 * mean,
                "{access}: {} after a loss",
                runs / pairs
            );
        }
    }

    #[test]
    fn phased_bursts_come_at_their_phase() {
        let mut site = Site::new(City::Dallas, Access::Starlink);
        site.profile.loss = 0.0;
        let periods = site
            .profile
            .bursts
            .and_then(|b| Some((b.every.as_nanos(), b.phase?.as_nanos())));
        assert!(periods.is_some());
        let Some((every, phase)) = periods else {
            return;
        };
        let lost = losses(site, 600);
        for (i, _) in (0_i64..).zip(&lost).filter(|&(_, &l)| l) {
            let at = t(i).unix_nanos();
            let into = (at - phase).rem_euclid(every);
            assert!(
                into < BURST_MAX.as_nanos(),
                "a loss {into} ns into a period"
            );
        }
        assert!(lost.iter().any(|&l| l));
    }

    #[test]
    fn bursts_depend_on_the_time_alone() {
        let bursts = Access::Lte.profile().bursts;
        assert!(bursts.is_some());
        let Some(b) = bursts else {
            return;
        };
        let times: Vec<Time> = (0..20_000).map(|i| t(i * 7)).collect();
        let mut forward = BurstState::new(b, 11);
        let ahead: Vec<bool> = times.iter().map(|&x| forward.covers(x)).collect();
        let mut backward = BurstState::new(b, 11);
        let mut behind: Vec<bool> = times.iter().rev().map(|&x| backward.covers(x)).collect();
        behind.reverse();
        assert_eq!(ahead, behind);
        assert!(ahead.iter().any(|&x| x) && ahead.iter().any(|&x| !x));
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

    /// Linux's tbf, packet by packet, as `net/sched/sch_tbf.c`'s
    /// `tbf_dequeue` keeps it: tokens are nanoseconds of transmission at
    /// the rate, at most the buffer's (`burst` at the rate); the head packet
    /// leaves once `min(buffer, tokens + (now − t_c))` covers its own time,
    /// taking it. With a peakrate, a second bucket of `mtu` at the peak
    /// must cover it too.
    struct Tbf {
        rate: u64,
        buffer: i64,
        tokens: i64,
        /// The peak and its bucket (`mtu` at the peak), and its tokens.
        peak: Option<(u64, i64)>,
        ptokens: i64,
        t_c: i64,
        last: i64,
    }

    impl Tbf {
        fn new(rate: u64, burst: u64) -> Tbf {
            Tbf::with_peak(rate, burst, None)
        }

        /// tbf's `peakrate <peak> mtu 3028`, as `run.sh` sets it.
        fn with_peak(rate: u64, burst: u64, peak: Option<u64>) -> Tbf {
            let buffer = Tbf::l2t(rate, burst as usize);
            let peak = peak.map(|p| (p, Tbf::l2t(p, PEAK_MTU)));
            Tbf {
                rate,
                buffer,
                tokens: buffer,
                peak,
                ptokens: peak.map_or(0, |(_, mtu)| mtu),
                t_c: 0,
                last: 0,
            }
        }

        fn l2t(rate: u64, len: usize) -> i64 {
            len as i64 * 1_000_000_000 / rate as i64
        }

        /// When a packet of `len` bytes enqueued at `at` (ns) behind those
        /// before it is dequeued.
        fn packet(&mut self, at: i64, len: usize) -> i64 {
            let cost = Tbf::l2t(self.rate, len);
            assert!(cost <= self.buffer, "a packet past the bucket");
            let mut now = at.max(self.last);
            loop {
                let elapsed = (now - self.t_c).min(self.buffer);
                let toks = (self.tokens + elapsed).min(self.buffer) - cost;
                let ptoks = self.peak.map_or(0, |(p, mtu)| {
                    (self.ptokens + elapsed).min(mtu) - Tbf::l2t(p, len)
                });
                if toks >= 0 && ptoks >= 0 {
                    self.tokens = toks;
                    self.ptokens = ptoks;
                    self.t_c = now;
                    self.last = now;
                    return now;
                }
                // The watchdog fires once both deficits have refilled.
                now += (-toks).max(-ptoks);
            }
        }
    }

    /// Messages of `sizes` bytes, as packets of at most a frame, handed to
    /// an uplink of `rate` with a `burst`-byte bucket at `gaps` apart (ns):
    /// each message's departure in the model and its last packet's in
    /// [`Tbf`].
    fn against_tbf(rate: u64, burst: u64, msgs: &[(i64, usize)]) -> (Vec<Time>, Vec<Time>) {
        let site = Site::new(City::Frankfurt, Access::Fiber)
            .with_up(rate)
            .with_bucket(Some(Bucket::of(burst)));
        let mut n = net(&[site, Site::new(City::Frankfurt, Access::Fiber)], QUIET);
        let mut tbf = Tbf::new(rate, burst);
        let base = t(0).unix_nanos();
        let (mut model, mut reference) = (Vec::new(), Vec::new());
        let mut at = 0;
        for &(gap, wire) in msgs {
            at += gap;
            let sent = Time::from_unix_nanos(base + at);
            model.push(
                n.send(
                    sent,
                    0,
                    1,
                    wire,
                    wire.div_ceil(1514) as u32,
                    Carrier::Stream,
                )
                .depart,
            );
            let mut left = wire;
            let mut out = 0;
            while left > 0 {
                let len = left.min(1514);
                out = tbf.packet(at, len);
                left -= len;
            }
            reference.push(Time::from_unix_nanos(base + out));
        }
        (model, reference)
    }

    #[test]
    fn a_bucket_departs_as_tbf_dequeues() {
        // At 10 and 20 Mbit/s a byte takes 800 and 400 ns: both sides'
        // arithmetic is exact.
        for (rate, burst) in [(1_250_000, 3028), (2_500_000, 1600), (2_500_000, 65_536)] {
            // After idle, a leg's packets: a full bucket's bytes at once,
            // the rest at the rate.
            let leg = [(0, 200), (0, 1514), (0, 1514), (0, 1514)];
            let (model, reference) = against_tbf(rate, burst, &leg);
            assert_eq!(model, reference, "{rate} {burst}");
            let wire: usize = leg.iter().map(|&(_, w)| w).sum();
            let deficit = (wire as u64).saturating_sub(burst);
            assert_eq!(
                model.last().map(|&d| d - t(0)),
                Some(tx_time(deficit as usize, rate)),
                "{rate} {burst}"
            );
            // Messages of every size from back to back to long idle, one
            // packet and several.
            let mut rng = det_rng(7);
            let msgs: Vec<(i64, usize)> = (0..4_000)
                .map(|_| {
                    let gap = match unit(&mut rng) {
                        x if x < 0.5 => 0,
                        x if x < 0.8 => (unit(&mut rng) * 2e6) as i64,
                        _ => (unit(&mut rng) * 50e6) as i64,
                    };
                    let wire = 40 + (unit(&mut rng) * 3.0 * 1514.0) as usize;
                    (gap, wire)
                })
                .collect();
            let (model, reference) = against_tbf(rate, burst, &msgs);
            assert_eq!(model, reference, "{rate} {burst}");
        }
    }

    #[test]
    fn without_a_bucket_an_uplink_is_unchanged() {
        let up = 1_250_000;
        let plain = [
            Site::new(City::Frankfurt, Access::Fiber).with_up(up),
            Site::new(City::Frankfurt, Access::Fiber),
        ];
        let mut a = net(&plain, QUIET);
        let mut rng = det_rng(3);
        let mut at = t(0);
        for _ in 0..2_000 {
            at += Duration::from_micros((unit(&mut rng) * 3_000.0) as i64);
            let wire = 40 + (unit(&mut rng) * 5_000.0) as usize;
            let start = at.max(a.up_free(0));
            let d = a.send(at, 0, 1, wire, 1, Carrier::Stream).depart;
            assert_eq!(d, through(up, &[], start, wire));
        }
    }

    /// The peak bucket `run.sh` gives tbf: two frames.
    const PEAK_MTU: usize = 3028;

    /// tbf's peakrate against the model's peak: an uplink of `rate` with a
    /// `burst`-byte bucket sending at `peak`, messages of every size from
    /// back to back to long idle. The model's message leaves with its last
    /// bit on a line at the peak; tbf dequeues a packet once the peak's
    /// bucket ([`PEAK_MTU`]) covers it, so a train its peak paces leaves
    /// that bucket at the peak sooner there, and no more: the model is
    /// never earlier, and later by at most the bucket. Departures the rate
    /// paces are the same.
    #[test]
    fn a_peak_departs_as_tbf_peakrate_dequeues() {
        for (rate, burst, peak) in [
            (1_250_000, 65_536, 12_500_000), // 10 Mbit/s, 64 KB, 100 Mbit/s
            (1_250_000, 312_500, 5_000_000), // 10, 312.5 KB, 40
            (625_000, 312_500, 6_250_000),   // 5, 312.5 KB, 50
        ] {
            let site = Site::new(City::Frankfurt, Access::Cable)
                .with_up(rate)
                .with_bucket(Some(Bucket {
                    bytes: burst,
                    peak: Some(peak),
                }));
            let mut n = net(&[site, Site::new(City::Frankfurt, Access::Fiber)], QUIET);
            let mut tbf = Tbf::with_peak(rate, burst, Some(peak));
            let frame = tx_time(PEAK_MTU, peak);
            let base = t(0).unix_nanos();
            let mut rng = det_rng(11);
            let mut at = 0;
            let (mut paced, mut by_rate) = (0, 0);
            for _ in 0..4_000 {
                at += match unit(&mut rng) {
                    x if x < 0.5 => 0,
                    x if x < 0.8 => (unit(&mut rng) * 2e6) as i64,
                    _ => (unit(&mut rng) * 300e6) as i64,
                };
                let wire = 40 + (unit(&mut rng) * 16.0 * 1514.0) as usize;
                let sent = Time::from_unix_nanos(base + at);
                let model = n
                    .send(
                        sent,
                        0,
                        1,
                        wire,
                        wire.div_ceil(1514) as u32,
                        Carrier::Stream,
                    )
                    .depart;
                let mut left = wire;
                let mut out = 0;
                while left > 0 {
                    let len = left.min(1514);
                    out = tbf.packet(at, len);
                    left -= len;
                }
                let late = model - Time::from_unix_nanos(base + out);
                assert!(
                    late >= Duration::ZERO,
                    "{rate} {burst} {peak}: {late:?} early"
                );
                assert!(late <= frame, "{rate} {burst} {peak}: {late:?} late");
                if late == Duration::ZERO {
                    by_rate += 1;
                } else {
                    paced += 1;
                }
            }
            // Both regimes ran.
            assert!(
                paced > 20 && by_rate > 20,
                "{rate} {burst} {peak}: {paced} {by_rate}"
            );
        }
    }

    #[test]
    fn a_peak_paces_what_the_bucket_covers() {
        let (rate, peak) = (1_250_000, 12_500_000); // 10 and 100 Mbit/s
        let site = Site::new(City::Frankfurt, Access::Cable)
            .with_up(rate)
            .with_bucket(Some(Bucket {
                bytes: 100_000,
                peak: Some(peak),
            }));
        let mut n = net(&[site, Site::new(City::Frankfurt, Access::Fiber)], QUIET);
        // 50 KB of a full bucket leave at the peak: 4 ms, not at once.
        let d = n.send(t(0), 0, 1, 50_000, 34, Carrier::Stream).depart;
        assert_eq!(d, t(4));
        // The bucket held 100 KB, took 50 and refilled 5 meanwhile: 55 KB
        // at once at 4 ms, so 100 KB more leave at the rate past them, 36
        // ms later, or at the peak in 8 ms, whichever is later.
        let d = n.send(t(4), 0, 1, 100_000, 67, Carrier::Stream).depart;
        assert_eq!(d, t(4 + 36));
    }

    #[test]
    fn a_bucket_refills_at_the_stepped_rate() {
        let up = 1_000_000; // 1 MB/s: a byte a microsecond
        let site = Site::new(City::Frankfurt, Access::Fiber)
            .with_up(up)
            .with_bucket(Some(Bucket::of(10_000)));
        let mut n = net(&[site, Site::new(City::Frankfurt, Access::Fiber)], QUIET);
        n.set_up_steps(0, vec![(t(10), up / 2)]);
        // Empty the bucket at 0 ms: 10 KB at once.
        assert_eq!(n.send(t(0), 0, 1, 10_000, 7, Carrier::Stream).depart, t(0));
        // By 12 ms it refilled 10 ms at 1 MB/s and 2 at 0.5: 11 KB, capped
        // at 10; the 4 KB past them go at the half rate.
        assert_eq!(
            n.send(t(12), 0, 1, 14_000, 10, Carrier::Stream).depart,
            t(20)
        );
        // From empty at 20 ms, 4 ms refill 2 KB: a 3 KB message waits 2 ms.
        assert_eq!(n.send(t(24), 0, 1, 3_000, 2, Carrier::Stream).depart, t(26));
    }

    #[test]
    fn a_bucketed_queue_holds_bytes() {
        let up = 1_000_000;
        let site = Site::new(City::Dallas, Access::Fiber)
            .with_up(up)
            .with_bucket(Some(Bucket::of(3_000)));
        let mut n = net(&[site, Site::new(City::Dallas, Access::Fiber)], QUIET);
        let ms = Duration::from_millis;
        // The first 3000 bytes leave on the bucket at once; the next 3000
        // wait for the rate, until 3 ms, and count whole till then.
        assert_eq!(n.send(t(0), 0, 1, 3000, 2, Carrier::Stream).depart, t(0));
        assert_eq!(n.occupy_up(t(0), 0, 3000, 2), t(3));
        assert_eq!(n.up_backlog(0, t(0)), ms(3));
        assert_eq!(n.up_backlog(0, t(2)), ms(3));
        assert_eq!(n.up_backlog(0, t(3)), Duration::ZERO);
        // A host that loses power while its queue drains forgets the queue
        // and keeps the bucket as it was, empty: 1000 bytes then wait the
        // 1 ms the rate takes to refill them.
        n.send(t(10), 0, 1, 3000, 2, Carrier::Stream);
        n.send(t(10), 0, 1, 3000, 2, Carrier::Stream);
        n.reset(0, t(11));
        assert_eq!(n.send(t(11), 0, 1, 1000, 1, Carrier::Stream).depart, t(12));
    }

    #[test]
    fn an_uplink_charges_its_framing() {
        use crate::framing::Framing;
        let up = 1_514_000; // a 1514-byte frame a ms
        let site = |f| {
            Site::new(City::Dallas, Access::Fiber)
                .with_up(up)
                .with_framing(f)
        };
        let mut n = net(
            &[
                site(Framing::ETHERNET),
                site(Framing::ADSL_PPPOE),
                site(Framing::NONE),
            ],
            QUIET,
        );
        // Two full TCP packets: 2 x 1514 bytes of Ethernet, 2 x 1749 of
        // cells, 3000 of IP.
        assert_eq!(n.send(t(0), 0, 2, 3000, 2, Carrier::Stream).depart, t(2));
        assert_eq!(n.up_charge(1, 3000, 2), 3498);
        assert_eq!(n.stats(0).up_bytes, 3028);
        n.send(t(0), 1, 2, 3000, 2, Carrier::Stream);
        assert_eq!(n.up_free(1), t(0) + tx_time(3498, up));
        assert_eq!(
            n.send(t(10), 2, 0, 3000, 2, Carrier::Stream).depart,
            t(10) + tx_time(3000, up)
        );
        // A queue's bytes are what it charges.
        n.occupy_up(t(20), 1, 64, 1);
        assert_eq!(n.up_backlog(1, t(20)), tx_time(159, up));
    }

    #[test]
    fn slots_send_what_the_shaper_passed_when_the_next_opens() {
        use crate::slots::Slots;
        let ms = Duration::from_millis;
        let rate = 1_500_000; // a 1500-byte message a ms
        let site = Site::new(City::Dallas, Access::Fiber)
            .with_up(rate)
            .with_slots(Some(Slots {
                min: ms(2),
                max: ms(2),
            }));
        let mut n = net(&[site, Site::new(City::Dallas, Access::Fiber)], QUIET);
        // The shaper passes the first at 1 ms; a slot drawn then opens at
        // 3 ms, and the second, passed at 2 ms, rides it.
        assert_eq!(n.send(t(0), 0, 1, 1500, 1, Carrier::Datagram).depart, t(3));
        assert_eq!(n.send(t(0), 0, 1, 1500, 1, Carrier::Datagram).depart, t(3));
        // The shaper took the second at 1 ms, not at the first's slot.
        assert_eq!(n.up_free(0), t(2));
        // What waits for a slot isn't in the queue: the shaper has passed
        // both by 2 ms.
        assert_eq!(n.up_backlog(0, t(2)), Duration::ZERO);
        assert_eq!(n.up_backlog(0, t(1)), ms(1));
        // Passed after that slot: the next, drawn as it is passed.
        assert_eq!(
            n.send(t(10), 0, 1, 1500, 1, Carrier::Datagram).depart,
            t(13)
        );
        assert_eq!(n.up_slots(0).map(|s| s.min), Some(ms(2)));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn a_capacity_schedule_replaces_the_rate() {
        use crate::capacity::Capacity;
        // A 1500-byte opportunity every 2 ms, from t(0).
        let cap = Capacity::mahimahi("2\n4\n6\n8\n10\n", 1500, 1.0)
            .unwrap()
            .starting(t(0));
        let site = Site::new(City::Dallas, Access::Fiber).with_up(1_000_000_000);
        let mut n = net(&[site, Site::new(City::Dallas, Access::Fiber)], QUIET);
        n.set_up_capacity(0, std::sync::Arc::new(cap));
        assert_eq!(n.up_rate(0, t(0)), 1_000_000_000);
        // At 1 ms the next opportunity is 2 ms; 3000 bytes take two.
        assert_eq!(n.send(t(1), 0, 1, 3000, 2, Carrier::Stream).depart, t(4));
        assert_eq!(n.up_backlog(0, t(1)), tx_time(3000, 1_000_000_000));
        // The 6 and 8 ms opportunities pass unused while it idles.
        assert_eq!(n.send(t(9), 0, 1, 100, 1, Carrier::Stream).depart, t(10));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn a_reset_empties_a_capacity_uplink() {
        use crate::capacity::Capacity;
        // A 1500-byte opportunity every ms.
        let cap = Capacity::mahimahi("1\n", 1500, 1.0).unwrap().starting(t(0));
        let site = Site::new(City::Dallas, Access::Fiber).with_up(1_500_000);
        let mut n = net(&[site, Site::new(City::Dallas, Access::Fiber)], QUIET);
        n.set_up_capacity(0, std::sync::Arc::new(cap));
        // 225 KB take the opportunities of 0-149 ms; a power cut at 1 ms
        // forgets them, and 100
        // bytes at 2 ms take the next.
        assert_eq!(
            n.send(t(0), 0, 1, 225_000, 150, Carrier::Stream).depart,
            t(149)
        );
        n.reset(0, t(1));
        assert_eq!(n.up_backlog(0, t(1)), Duration::ZERO);
        let d = n.send(t(2), 0, 1, 100, 1, Carrier::Stream).depart;
        assert!(d <= t(3), "{d:?}");
    }
}
