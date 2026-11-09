// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! When the window model's TCP receivers acknowledge what reaches them, as
//! Linux 7.0's do (`net/ipv4/tcp_input.c`, `tcp_output.c`, `tcp.c` and
//! `tcp_timer.c`), with no GRO, as on the Docker testbed. A segment reaches
//! its receiver as its packets: full ones of [`MSS`] and a last that holds
//! the rest, which carries PSH, as the testbed's senders mark every
//! segment's end. Each packet, in turn:
//!
//! - Moves the receiver's estimate of its sender's segment (`rcv_mss`,
//!   `tcp_measure_rcv_mss`): 536 bytes (`TCP_MSS_DEFAULT`) until a packet
//!   at least that long sets it to its length, to the MSS; two short
//!   packets in a row of one length, [`SAME_SHORT`] bytes or more, set it
//!   to theirs. A packet shorter than the estimate marks the connection
//!   pushed (`ICSK_ACK_PUSHED`), a second one pushed twice
//!   (`ICSK_ACK_PUSHED2`); a packet with PSH as long marks it pushed.
//! - Moves the predicted inter-arrival time (ato, `tcp_event_data_recv`):
//!   it starts at [`ATO_MIN`] with the first data, which grants
//!   [`QUICKACKS`] quickacks (`tcp_incr_quickack`: the receive window over
//!   twice `rcv_mss`, at any window the node's buffers give). A packet
//!   within half of `ATO_MIN` of the last halves ato and adds that half,
//!   one within ato halves it and adds the gap, to the retransmission
//!   timeout at most, and one after more than the timeout grants the
//!   quickacks again.
//! - Is acknowledged at once (`__tcp_ack_snd_check`) when more than
//!   `rcv_mss` bytes arrived since the last acknowledgement left
//!   (`rcv_nxt - rcv_wup`; the window's right edge advances, as the node
//!   reads what arrives), or in quickack mode, with quickacks left outside
//!   pingpong mode (`tcp_in_quickack_mode`). A segment past a hole, or
//!   while one stays open, is acknowledged at once (a SACK of the out of
//!   order queue), and so is the one that closes it (`ICSK_ACK_NOW`).
//! - Otherwise starts the delayed acknowledgement's timer
//!   (`tcp_send_delayed_ack`): ato, which past [`DELACK_MIN`] the
//!   receiver's smoothed round trip bounds (`DELACK_MIN` at least), and
//!   [`DELACK_MAX`] in pingpong mode or pushed, [`DELACK_SLOW`] otherwise;
//!   [`DELACK_LIMIT`] bounds it all (`tcp_delack_max`). A running timer
//!   keeps its earlier time, and when that is due within a quarter of the
//!   new one the acknowledgement goes at once.
//! - Is read at once (`__tcp_cleanup_rbuf`): with nothing held out of
//!   order, a read that finds the connection pushed twice, or pushed and
//!   not in pingpong mode, acknowledges at once.
//!
//! The timer's acknowledgement (`tcp_delack_timer_handler`) doubles ato,
//! to the retransmission timeout, outside pingpong mode, and in it leaves
//! the mode and sets ato back to `ATO_MIN`. Any acknowledgement that leaves
//! (`tcp_event_ack_sent`) takes a quickack, the last setting ato back to
//! `ATO_MIN` (`tcp_dec_quickack_mode`), and clears the pushed marks and
//! the timer. Data the receiver sends carries what it owes, and when it
//! leaves within ato of the last data received the connection enters
//! pingpong mode (`tcp_event_data_sent`; 7.0's `tcp_pingpong_thresh` is a
//! single reply).
//!
//! An acknowledgement that goes after a packet within a segment
//! acknowledges the segment's bytes through it; the rest of the segment
//! waits as a piece of its own. The receiver's round trip and timeout are
//! its own as a sender on the connection, or failing a sample, the
//! handshake's: the path's round trip.
//!
//! Left out: GRO, as on the testbed, where offloads are off; at the access
//! links' few Mbit/s a connection's packets reach their receiver a
//! millisecond or more apart, more than a NAPI poll's batch, so it would
//! hand TCP one packet at a time anyway. The quickacks Linux sends while
//! the receive window grows (`tcp_grow_window`), every packet outside
//! pingpong mode until `rcv_ssthresh` reaches what the buffer allows,
//! on the testbed's bench about a connection's first megabyte: a start-up
//! transient a node's links, in pingpong mode, never see. The window
//! updates a read sends when it frees half the window, which the node's
//! prompt reads never close, and compressed SACKs, as the sender learns of
//! its losses from the model.

use std::collections::BTreeSet;

use strims_base::time::{Duration, Time};
use strims_wan::wire::MSS;

/// The least predicted inter-arrival time, and the least a delayed
/// acknowledgement waits: Linux's `TCP_ATO_MIN` and `TCP_DELACK_MIN`
/// (`include/net/tcp.h`, 40 ms at any HZ).
pub(super) const ATO_MIN: Duration = Duration::from_millis(40);
const DELACK_MIN: Duration = Duration::from_millis(40);
/// The most a delayed acknowledgement waits in pingpong mode or pushed
/// (`TCP_DELACK_MAX`), and otherwise (`HZ / 2`).
const DELACK_MAX: Duration = Duration::from_millis(200);
const DELACK_SLOW: Duration = Duration::from_millis(500);
/// The most any delayed acknowledgement waits (`tcp_delack_max`):
/// `TCP_DELACK_MAX`, and a jiffy under the least retransmission timeout
/// (HZ 1000, as Ubuntu builds it).
const DELACK_LIMIT: Duration = Duration::from_millis(199);
/// The quickacks the first data and a gap past the retransmission timeout
/// grant (`TCP_MAX_QUICKACKS`).
pub(super) const QUICKACKS: u32 = 16;
/// The receiver's estimate of its sender's segment before a packet that
/// long (`TCP_MSS_DEFAULT`).
const RCV_MSS_INIT: usize = 536;
/// The least a short packet holds for two in a row to set `rcv_mss`:
/// `TCP_MSS_DEFAULT` and a bare TCP header, less the header with
/// timestamps.
const SAME_SHORT: usize = RCV_MSS_INIT + 20 - 32;

/// Why a receiver sent an acknowledgement, its index in
/// [`ACK_WHYS`](crate::report::ACK_WHYS).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AckWhy {
    Now = 0,
    Quick = 1,
    Read = 2,
    Timer = 3,
    Data = 4,
}

/// A segment's bytes waiting for their acknowledgement: the key of the
/// event that brings it to the sender, the segment's sequence number and
/// how many bytes of it.
pub(super) type Piece = (u64, u64, usize);

/// An acknowledgement the receiver sends: why, what it acknowledges, and
/// how long the first of it waited.
#[derive(Debug)]
pub(super) struct Ack {
    pub why: AckWhy,
    pub pieces: Vec<Piece>,
    pub wait: Duration,
}

/// The receiver's own round trip estimate and retransmission timeout.
#[derive(Clone, Copy, Debug)]
pub(super) struct RcvClock {
    pub srtt: Duration,
    pub rto: Duration,
}

/// One connection's receiver (see the module docs): what it owes its
/// sender, the bytes in order since its last acknowledgement and when the
/// first of what it owes arrived, its delayed acknowledgement's timer,
/// when data last arrived and ato, its quickacks left, pingpong mode, the
/// pushed marks (one or two), its `rcv_mss` and the last short packet's
/// length, and the next segment in order (turmoil numbers a connection's
/// segments from 1) and those past a hole. Bound: the connection's
/// segments in flight.
#[derive(Clone, Debug)]
pub(super) struct Owed {
    waiting: Vec<Piece>,
    unacked: usize,
    since: Time,
    due: Option<Time>,
    last: Time,
    ato: Duration,
    quick: u32,
    pingpong: bool,
    pushed: u8,
    rcv_mss: usize,
    last_seg: usize,
    next: u64,
    ahead: BTreeSet<u64>,
}

impl Default for Owed {
    fn default() -> Owed {
        Owed {
            waiting: Vec::new(),
            unacked: 0,
            since: Time::NIL,
            due: None,
            last: Time::NIL,
            ato: Duration::ZERO,
            quick: 0,
            pingpong: false,
            pushed: 0,
            rcv_mss: RCV_MSS_INIT,
            last_seg: 0,
            next: 1,
            ahead: BTreeSet::new(),
        }
    }
}

impl Owed {
    /// When the delayed acknowledgement's timer is due.
    pub(super) fn due(&self) -> Option<Time> {
        self.due
    }

    /// Segment `seq` of `len` bytes in `packets` packets, record `key`, is
    /// through the receiver's downlink at `t`: the acknowledgements it
    /// sends at once, in order. A piece cut from the segment takes the
    /// next of `fresh`'s keys.
    pub(super) fn arrive(
        &mut self,
        t: Time,
        key: u64,
        (seq, len, packets): (u64, usize, u32),
        clock: RcvClock,
        fresh: &mut u64,
    ) -> Vec<Ack> {
        if seq != self.next {
            // Past a hole (or, which the model never sends, again).
            if seq > self.next {
                self.ahead.insert(seq);
            }
            self.hold(t, (key, seq, len));
            return vec![self.ack(t, AckWhy::Now)];
        }
        let mut gap = !self.ahead.is_empty();
        self.next = seq + 1;
        while self.ahead.remove(&self.next) {
            self.next += 1;
        }
        let n = packets.max(1) as usize;
        let mut acks = Vec::new();
        let mut piece = (key, seq, 0);
        for k in 0..n {
            let p = if k + 1 < n {
                MSS
            } else {
                len.saturating_sub((n - 1) * MSS)
            };
            self.measure(p, k + 1 == n);
            self.inter_arrival(t, clock.rto);
            if self.waiting.is_empty() && piece.2 == 0 {
                self.since = t;
            }
            piece.2 += p;
            self.unacked += p;
            let why = if self.unacked > self.rcv_mss || std::mem::take(&mut gap) {
                Some(AckWhy::Now)
            } else if self.quick > 0 && !self.pingpong {
                Some(AckWhy::Quick)
            } else {
                self.delay(t, clock).or_else(|| self.read())
            };
            if let Some(why) = why {
                self.waiting.push(piece);
                acks.push(self.ack(t, why));
                piece = (*fresh, seq, 0);
                if k + 1 < n {
                    *fresh += 1;
                }
            }
        }
        if piece.2 > 0 {
            self.waiting.push(piece);
        }
        if !self.ahead.is_empty() && !self.waiting.is_empty() {
            acks.push(self.ack(t, AckWhy::Now));
        }
        acks
    }

    /// The delayed acknowledgement's timer fires at `t`, the receiver's
    /// timeout `rto`: the acknowledgement it sends, if it still owes one.
    pub(super) fn timer(&mut self, t: Time, rto: Duration) -> Option<Ack> {
        if self.due != Some(t) {
            return None;
        }
        self.due = None;
        if self.waiting.is_empty() {
            return None;
        }
        if self.pingpong {
            self.pingpong = false;
            self.ato = ATO_MIN;
        } else {
            self.ato = (self.ato * 2).min(rto);
        }
        Some(self.ack(t, AckWhy::Timer))
    }

    /// The receiver sends data at `t`: the acknowledgement it carries, if
    /// it owes one.
    pub(super) fn data_sent(&mut self, t: Time) -> Option<Ack> {
        let ack = (!self.waiting.is_empty()).then(|| self.ack(t, AckWhy::Data));
        // The model sends a walk's writes a step back, where data it
        // reads may have arrived since.
        if !self.last.is_nil() && (t <= self.last || t - self.last < self.ato) {
            self.pingpong = true;
        }
        ack
    }

    /// `tcp_measure_rcv_mss` for a packet of `p` bytes, with PSH at a
    /// segment's `end`.
    fn measure(&mut self, p: usize, end: bool) {
        let lss = std::mem::take(&mut self.last_seg);
        if p >= self.rcv_mss {
            self.rcv_mss = p.min(MSS);
            if end {
                self.pushed = self.pushed.max(1);
            }
            return;
        }
        if p >= SAME_SHORT {
            self.last_seg = p;
            if p == lss {
                self.rcv_mss = p;
                return;
            }
        }
        self.pushed = if self.pushed > 0 { 2 } else { 1 };
    }

    /// `tcp_event_data_recv`'s ato and quickacks for a packet at `t`.
    fn inter_arrival(&mut self, t: Time, rto: Duration) {
        if self.ato == Duration::ZERO {
            self.quick = self.quick.max(QUICKACKS);
            self.ato = ATO_MIN;
        } else {
            let m = t - self.last;
            if m <= ATO_MIN / 2 {
                self.ato = self.ato / 2 + ATO_MIN / 2;
            } else if m < self.ato {
                self.ato = (self.ato / 2 + m).min(rto);
            } else if m > rto {
                self.quick = self.quick.max(QUICKACKS);
            }
        }
        self.last = t;
    }

    /// `tcp_send_delayed_ack` at `t`: starts or moves the timer, or, when
    /// the running one is due within a quarter of the wait, sends now.
    fn delay(&mut self, t: Time, clock: RcvClock) -> Option<AckWhy> {
        let mut ato = self.ato;
        if ato > DELACK_MIN {
            let most = if self.pingpong || self.pushed > 0 {
                DELACK_MAX
            } else {
                DELACK_SLOW
            };
            ato = ato.min(most.min(clock.srtt.max(DELACK_MIN)));
        }
        let ato = ato.min(DELACK_LIMIT);
        match self.due {
            Some(due) if due <= t + ato / 4 => Some(AckWhy::Timer),
            Some(due) => {
                self.due = Some(due.min(t + ato));
                None
            }
            None => {
                self.due = Some(t + ato);
                None
            }
        }
    }

    /// `__tcp_cleanup_rbuf` as the node reads a packet.
    fn read(&self) -> Option<AckWhy> {
        let pushed = self.pushed == 2 || (self.pushed == 1 && !self.pingpong);
        (self.ahead.is_empty() && pushed).then_some(AckWhy::Read)
    }

    /// Owes `piece`, which arrived at `t`.
    fn hold(&mut self, t: Time, piece: Piece) {
        if self.waiting.is_empty() {
            self.since = t;
        }
        self.waiting.push(piece);
    }

    /// An acknowledgement of everything owed leaves at `t`, for `why`
    /// (`tcp_event_ack_sent`).
    fn ack(&mut self, t: Time, why: AckWhy) -> Ack {
        if self.quick > 0 {
            if self.quick <= 1 {
                self.quick = 0;
                self.ato = ATO_MIN;
            } else {
                self.quick -= 1;
            }
        }
        self.pushed = 0;
        self.due = None;
        self.unacked = 0;
        Ack {
            why,
            pieces: std::mem::take(&mut self.waiting),
            wait: t - self.since,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use strims_wan::wire::tcp_wire;

    use AckWhy::{Data, Now, Quick, Read, Timer};

    /// A receiver, the acknowledgements it sent (ms, why, bytes), and the
    /// time.
    struct Rx {
        o: Owed,
        clock: RcvClock,
        seq: u64,
        fresh: u64,
        acks: Vec<(i64, AckWhy, usize)>,
        t: Time,
    }

    const START: Time = Time::EPOCH;

    impl Rx {
        /// On a 30 ms path: a 230 ms timeout.
        fn new() -> Rx {
            Rx::with_rtt(Duration::from_millis(30))
        }

        fn with_rtt(rtt: Duration) -> Rx {
            Rx {
                o: Owed::default(),
                clock: RcvClock {
                    srtt: rtt,
                    rto: rtt + (rtt * 2).max(Duration::from_millis(200)),
                },
                seq: 1,
                fresh: 1 << 40,
                acks: Vec::new(),
                t: START,
            }
        }

        /// The clock moves to `ms`, the timer firing on the way.
        fn until(&mut self, ms: i64) {
            let to = START + Duration::from_millis(ms);
            if let Some(due) = self.o.due()
                && due <= to
            {
                self.t = due;
                if let Some(a) = self.o.timer(due, self.clock.rto) {
                    self.log(a);
                }
            }
            self.t = to;
        }

        fn log(&mut self, a: Ack) {
            let bytes = a.pieces.iter().map(|p| p.2).sum();
            self.acks.push(((self.t - START).as_millis(), a.why, bytes));
        }

        /// A segment of `len` bytes arrives at `ms`, its key its sequence
        /// number.
        fn seg(&mut self, ms: i64, len: usize) {
            self.until(ms);
            let seg = (self.seq, len, tcp_wire(len).1);
            let acks = self
                .o
                .arrive(self.t, self.seq, seg, self.clock, &mut self.fresh);
            self.seq += 1;
            for a in acks {
                self.log(a);
            }
        }

        /// A 4 KiB write at `ms`: a segment of two full packets, and one
        /// of the 1200 bytes left a millisecond on.
        fn chunk(&mut self, ms: i64) {
            self.seg(ms, 2 * MSS);
            self.seg(ms + 1, 4096 - 2 * MSS);
        }

        /// The receiver sends data at `ms`.
        fn reply(&mut self, ms: i64) {
            self.until(ms);
            if let Some(a) = self.o.data_sent(self.t) {
                self.log(a);
            }
        }

        /// The first data, 16 full packets a millisecond apart from 0 ms,
        /// spends its quickacks.
        fn warm(&mut self) {
            for k in 0..QUICKACKS {
                self.seg(i64::from(k), MSS);
            }
            assert_eq!(self.whys(), [Quick; QUICKACKS as usize]);
            self.acks.clear();
        }

        /// Two full packets at `ms` and a reply a millisecond on: pingpong
        /// mode.
        fn interactive(&mut self, ms: i64) {
            self.seg(ms, 2 * MSS);
            self.reply(ms + 1);
            assert!(self.o.pingpong);
            self.acks.clear();
        }

        fn whys(&self) -> Vec<AckWhy> {
            self.acks.iter().map(|a| a.1).collect()
        }
    }

    #[test]
    fn a_quiet_receiver_acknowledges_4k_pushes_by_bytes_and_on_read() {
        // Past its quickacks, a receiver that sends nothing back takes a
        // 4 KiB write's two full packets in one acknowledgement and its
        // short tail as its reader drains it, as the testbed's quiet link
        // does (hack/testbed/bench/acks.sched): two a write, none waiting.
        let mut rx = Rx::new();
        rx.warm();
        for k in 0..8 {
            rx.chunk(30 + 4 * k);
        }
        rx.until(500);
        assert_eq!(rx.whys(), [Now, Read].repeat(8), "{:?}", rx.acks);
        let bytes: Vec<usize> = rx.acks.iter().map(|a| a.2).collect();
        assert_eq!(bytes, [2 * MSS, 1200].repeat(8));
    }

    #[test]
    fn interactive_traffic_holds_short_tails() {
        // A receiver that replies within ato is in pingpong mode: a read
        // of a lone short tail no longer acknowledges it, and the next
        // write's first packet takes it past rcv_mss. Over a burst the
        // acknowledgements go every two packets, three for two writes, as
        // on the testbed's link with HAVEs coming back; the reply carries
        // the burst's last tail.
        let mut rx = Rx::new();
        rx.warm();
        rx.interactive(30);
        for k in 0..3 {
            rx.chunk(40 + 4 * k);
        }
        rx.reply(60);
        assert_eq!(rx.whys(), [Now, Now, Now, Now, Data]);
        let bytes: Vec<usize> = rx.acks.iter().map(|a| a.2).collect();
        assert_eq!(bytes, [2 * MSS, 1200 + MSS, MSS + 1200, 2 * MSS, 1200]);
    }

    #[test]
    fn odd_full_packets_wait_for_the_next_or_the_timer() {
        // In pingpong mode a lone full packet waits for a second, and a
        // write of three full packets leaves its last for the 40 ms timer,
        // which ends the mode.
        let mut rx = Rx::new();
        rx.warm();
        rx.interactive(30);
        rx.seg(40, MSS);
        rx.seg(45, MSS);
        rx.seg(50, 2 * MSS);
        rx.seg(51, MSS);
        rx.until(200);
        assert_eq!(
            rx.acks,
            [(45, Now, 2 * MSS), (50, Now, 2 * MSS), (91, Timer, MSS)]
        );
        assert!(!rx.o.pingpong);
        // Outside it, the reader takes a lone full packet, which ends its
        // segment, with PSH, as pushed.
        rx.acks.clear();
        rx.seg(220, MSS);
        assert_eq!(rx.acks, [(220, Read, MSS)]);
    }

    #[test]
    fn an_idle_gap_grants_quickacks() {
        // Past the retransmission timeout (230 ms) without data, the
        // receiver acknowledges the next 16 packets at once, a segment's
        // first too, and then as before.
        let mut rx = Rx::new();
        rx.warm();
        for k in 0..9 {
            rx.seg(1000 + 10 * k, 2 * MSS);
        }
        let mut quick = vec![Quick; 16];
        quick.push(Now);
        assert_eq!(rx.whys(), quick);
        // In pingpong mode they don't apply: data carries the next
        // acknowledgement.
        let mut rx = Rx::new();
        rx.warm();
        rx.interactive(30);
        rx.seg(1000, MSS);
        rx.reply(1010);
        assert_eq!(rx.acks, [(1010, Data, MSS)]);
    }

    #[test]
    fn a_missed_delayed_ack_inflates_ato() {
        // Outside pingpong mode a packet waits for the timer only when it
        // isn't pushed: the second of two short ones of one length (524
        // bytes or more), which sets rcv_mss to theirs. Each timer that
        // fires doubles ato, to the timeout (300 ms on a 100 ms path), and
        // the receiver's round trip bounds the wait.
        let mut rx = Rx::with_rtt(Duration::from_millis(100));
        rx.warm();
        for (k, len) in [1000, 1000, 800, 800, 600, 600].into_iter().enumerate() {
            rx.seg(100 + 250 * k as i64, len);
        }
        rx.until(2000);
        assert_eq!(rx.whys(), [Read, Timer, Read, Timer, Read, Timer]);
        let waits: Vec<i64> = rx
            .acks
            .iter()
            .filter(|a| a.1 == Timer)
            .map(|a| (a.0 - 100) % 250)
            .collect();
        assert_eq!(waits, [40, 80, 100]);
        assert_eq!(rx.o.ato, Duration::from_millis(300));
    }

    #[test]
    fn pingpong_starts_with_a_reply_and_ends_with_a_missed_timer() {
        // A reply within ato puts the receiver in pingpong mode: its short
        // tails then wait for the timer, which leaves the mode, so the next
        // tail is acknowledged on read again; a reply later than ato
        // doesn't enter it.
        let mut rx = Rx::new();
        rx.warm();
        rx.seg(50, 1200);
        assert_eq!(rx.acks, [(50, Read, 1200)]);
        rx.reply(60);
        assert!(rx.o.pingpong);
        rx.acks.clear();
        rx.seg(100, 1200);
        rx.until(200);
        assert_eq!(rx.acks, [(140, Timer, 1200)]);
        assert!(!rx.o.pingpong);
        rx.acks.clear();
        rx.seg(250, 1200);
        assert_eq!(rx.acks, [(250, Read, 1200)]);
        rx.reply(400);
        assert!(!rx.o.pingpong);
    }

    #[test]
    fn two_short_segments_go_as_a_pair() {
        // Small segments (a viewer's HAVEs to the seed) leave rcv_mss
        // where it is; in pingpong mode the first marks the connection
        // pushed and waits, the second marks it pushed twice and its read
        // acknowledges both.
        let mut rx = Rx::new();
        rx.warm();
        rx.interactive(30);
        for k in 0..6 {
            rx.seg(100 + 15 * k, 30);
        }
        assert_eq!(rx.whys(), [Read; 3]);
        assert!(rx.acks.iter().all(|a| a.2 == 60));
    }

    #[test]
    fn segments_past_a_hole_are_acknowledged_at_once() {
        let mut rx = Rx::new();
        rx.warm();
        rx.interactive(30);
        let hole = rx.seq;
        rx.seq += 1;
        rx.seg(100, 1200);
        rx.seg(101, 1200);
        assert_eq!(rx.whys(), [Now, Now]);
        // The resent segment closes the hole: at once too, and then the
        // receiver holds a short tail again.
        let after = rx.seq;
        rx.seq = hole;
        rx.seg(200, 1200);
        rx.seq = after;
        assert_eq!(rx.whys(), [Now, Now, Now]);
        rx.seg(300, 1200);
        assert_eq!(rx.whys(), [Now, Now, Now]);
    }

    #[test]
    fn an_acknowledgement_within_a_segment_cuts_it() {
        // A held tail and a two-packet segment: the acknowledgement after
        // the segment's first packet takes the tail and that packet, the
        // second waits as a piece of its own, under a key of its own.
        let mut rx = Rx::new();
        rx.warm();
        rx.interactive(30);
        rx.seg(40, 1200);
        let (seq, fresh) = (rx.seq, rx.fresh);
        rx.seg(50, 2 * MSS);
        assert_eq!(rx.acks, [(50, Now, 1200 + MSS)]);
        assert_eq!(rx.o.waiting, [(fresh, seq, MSS)]);
        assert_eq!(rx.fresh, fresh + 1);
    }
}
