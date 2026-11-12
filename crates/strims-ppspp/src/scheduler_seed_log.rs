// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The seed log: what a seed decided about its neighbours and what became
//! of every chunk it sent, record by record, for a driver's tables to join
//! with what its neighbours held. On only at a seed, with the request log
//! ([`Runner::set_request_log`](crate::Runner::set_request_log)), and
//! taken with [`Runner::take_seed_log`](crate::Runner::take_seed_log); it
//! draws no randomness and no decision reads it, so a run with it traces
//! as one without. A holder sends no STATE summaries here, so a check's
//! `frontier`, `hole` and `state_at` are `None`, the silent check reading
//! its HAVEs, and a send's `declared` is whether its HAVEs announced the
//! chunk.
//!
//! - **Decisions** ([`SeedDecision`]): each STREAM_REQUEST answered and
//!   the rule that answered it, with the stream's subscribers before it;
//!   requests the first wave's ramp held; STREAM_CANCELs and the
//!   subscriptions the seed closed, and why; holders joining, leaving and
//!   barred; each holder's reading at each allocation check (its sampled
//!   push and the STATE it is judged by, its link's queue and delivery
//!   rate, its pushes' wait); each CANCEL and where the seed's push of the
//!   chunk stood then; the allocation's window at each check and each ramp
//!   step; and the answer gate's pace as it changes.
//! - **Sends** ([`SeedSend`]): one per chunk the seed pushed or answered,
//!   once it ended: written to the frame, dropped at write time (stale,
//!   cancelled by the peer, announced by its HAVEs, or unwritable),
//!   refused by the peer's full
//!   queue, or still queued when the channel closed. A push opens when it
//!   is queued; an answer is recorded as it is written or dropped, from
//!   the request's time.
//!
//! Times are the node's clock. Bound: [`SEED_LOG_CAP`] decisions and as many
//! sends between the driver's takes, past which they're counted as lost;
//! the pushes queued and not yet ended, the peers' queues' own bound
//! ([`DATA_QUEUE_CAP`](crate::DATA_QUEUE_CAP) per peer), at most
//! [`SEED_LOG_CAP`] (past which a push opens uncounted, and is counted lost);
//! and per channel the chunks of the store's window it cancelled or was
//! written, forgotten as the window passes them.

use std::collections::BTreeMap;

use strims_base::time::{Duration, Time};
use strims_binmap::Bin;

use crate::codec::Stream;
use crate::runner::PeerKey;
use crate::tap::{AdmitRule, HolderLeft};

/// Records of each kind the seed log holds between the driver's takes.
pub const SEED_LOG_CAP: usize = 1 << 16;

/// One of the seed's decisions about a neighbour (`peer`), or about its
/// allocation (`peer` `None`), at `at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeedDecision {
    pub at: Time,
    pub peer: Option<PeerKey>,
    pub kind: SeedDecisionKind,
}

/// What a [`SeedDecision`] records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedDecisionKind {
    /// A STREAM_REQUEST answered: accepted or declined (STREAM_CLOSE) by
    /// `rule`.
    Request(SeedRequest),
    /// A STREAM_REQUEST the first wave's ramp holds: the copies it opened
    /// have their holders, and the peer holds none. It is answered (a
    /// [`SeedDecisionKind::Request`]) once the ramp ends, its copies grow
    /// or it waited [`PENDING_MAX`](crate::scheduler_peer::push::seed::PENDING_MAX).
    Deferred { stream: Stream },
    /// The peer cancelled its subscription to `stream` (STREAM_CANCEL),
    /// leaving `subscribers`; `kept`: a holder's own cancel of a stream of
    /// its copy, which the seed keeps for a near satellite to take while
    /// no one else does.
    Released {
        stream: Stream,
        subscribers: usize,
        kept: bool,
    },
    /// The seed ended the peer's subscription to `stream` (`why`),
    /// leaving `subscribers`.
    Closed {
        stream: Stream,
        why: SeedClose,
        subscribers: usize,
    },
    /// The peer became a holder (`satellite`: of streams spilled from
    /// other holders, with no copy of its own); `holders` and `copies` are
    /// the holders of a copy and the copies after it.
    HolderJoined {
        satellite: bool,
        rtt: Option<Duration>,
        holders: usize,
        copies: usize,
    },
    /// The peer stopped being a holder (`why`); its subscriptions close
    /// (each a [`SeedDecisionKind::Closed`]) but for a closed channel's,
    /// which end with it.
    HolderLeft {
        why: HolderLeft,
        rtt: Option<Duration>,
        holders: usize,
        copies: usize,
    },
    /// The peer may hold no copy `until` then: its `bars`-th bar, by
    /// `why`; `reporters` voted on an unserved one.
    Barred {
        why: SeedBar,
        until: Time,
        bars: u32,
        reporters: usize,
    },
    /// A holder's reading at an allocation check.
    Check(SeedCheck),
    /// A CANCEL of the peer's of chunk `bin` of `stream`, and where the
    /// seed's push of it stood then.
    Cancel {
        stream: Stream,
        bin: Bin,
        open: CancelOpen,
    },
    /// The allocation's window and budget.
    Window(SeedWindow),
    /// The pace of the node's hybrid answers ([`crate::answer_gate`]) as
    /// it changed: its rate in bytes per second, the node's queueing delay
    /// below the runner it read, and whether the rate binds.
    AnswerRate {
        rate: u64,
        reading: Option<Duration>,
        engaged: bool,
    },
}

/// A STREAM_REQUEST answered ([`SeedDecisionKind::Request`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeedRequest {
    pub stream: Stream,
    pub granted: bool,
    pub rule: AdmitRule,
    /// The stream's subscribers before the answer.
    pub subscribers: usize,
    /// An adaptive seed's holders of a copy and its copies, the streams
    /// the peer held and the streams a holder may hold (`usize::MAX`
    /// before the budget is measured), as its admission read them; zero
    /// where no admission of the seed's read them (a pin, a subscriber
    /// asking again, a silent peer).
    pub holders: usize,
    pub copies: usize,
    pub held: usize,
    pub per_holder: usize,
}

/// Why the seed ended a subscription ([`SeedDecisionKind::Closed`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedClose {
    /// The peer stopped being a holder.
    Left(HolderLeft),
    /// The peer went silent
    /// ([`SILENT`](crate::scheduler_peer::push::SILENT)).
    Silent,
    /// A spill moved the stream between a holder whose link falls behind
    /// and a satellite (make-before-break).
    Spill,
    /// A pin left the peer out.
    Pin,
    /// The uplink was saturated (a relay's shed).
    Saturated,
    /// The request was declined while the peer was subscribed.
    Declined,
    /// The peer's channel closed.
    Channel,
}

impl SeedClose {
    /// Its name for a table: a holder's leaving's ([`holder_left_name`]),
    /// `silent`, `spill`, `pin`, `saturated`, `declined` or `channel`.
    pub fn name(self) -> &'static str {
        match self {
            SeedClose::Left(why) => holder_left_name(why),
            SeedClose::Silent => "silent",
            SeedClose::Spill => "spill",
            SeedClose::Pin => "pin",
            SeedClose::Saturated => "saturated",
            SeedClose::Declined => "declined",
            SeedClose::Channel => "channel",
        }
    }
}

/// Why the seed barred a holder ([`SeedDecisionKind::Barred`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedBar {
    /// Its STATEs didn't declare its sampled pushes at
    /// [`SILENT_CHECKS`](crate::scheduler_peer::push::seed::SILENT_CHECKS)
    /// checks in a row.
    Silent,
    /// Most of its reporters got less than
    /// [`UNSERVED_SHARE`](crate::scheduler_peer::push::seed::UNSERVED_SHARE)
    /// of what they expected from it.
    Unserved,
}

impl SeedBar {
    /// Its name for a table: `silent` or `unserved`.
    pub fn name(self) -> &'static str {
        match self {
            SeedBar::Silent => "silent",
            SeedBar::Unserved => "unserved",
        }
    }
}

/// Where the seed's push of a chunk to a peer stood at the peer's CANCEL
/// of it ([`SeedDecisionKind::Cancel`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelOpen {
    /// No push of it was queued or written to the peer (one queued later
    /// is dropped at write time all the same).
    None,
    /// A push of it was queued: it is dropped at write time.
    Queued,
    /// A push of it was written already.
    Written,
}

impl CancelOpen {
    /// Its name for a table: `none`, `queued` or `written`.
    pub fn name(self) -> &'static str {
        match self {
            CancelOpen::None => "none",
            CancelOpen::Queued => "queued",
            CancelOpen::Written => "written",
        }
    }
}

/// A holder's reading at an allocation check ([`SeedDecisionKind::Check`]),
/// what the seed judges it by: the silent check's sample, the spill's
/// link reading and the window's push waits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeedCheck {
    /// It holds spilled streams and no copy of its own.
    pub satellite: bool,
    /// The streams it is subscribed to.
    pub streams: usize,
    /// The push sampled at the check before last (the first written to it
    /// in that interval), its stream and when it was written, which its
    /// STATEs must declare by this check; `judged`: it was written at least
    /// [`ANNOUNCE_TIMEOUT`](crate::scheduler_peer::push::seed::ANNOUNCE_TIMEOUT)
    /// before, so this check judged it.
    pub due: Option<(Bin, Time)>,
    pub stream: Option<Stream>,
    pub judged: bool,
    /// Its latest STATE's newest chunk of the due push's stream, the
    /// newest before it the peer lacked, and when that STATE arrived;
    /// `None` without one (or without a due push).
    pub frontier: Option<Bin>,
    pub hole: Option<Bin>,
    pub state_at: Option<Time>,
    /// The checks in a row at which its due push wasn't declared, after
    /// this one.
    pub silent: u8,
    /// How far its link's least round trip since the last check stood
    /// above its smallest (the queue below the runner its pushes waited
    /// behind), and the least delivery rate its link read since the last
    /// check, bytes per second.
    pub queue: Option<Duration>,
    pub delivered: Option<u64>,
    /// Since the last check: the shortest wait of a push written to it or
    /// cancelled by it, and whether one went stale.
    pub wait: Option<Duration>,
    pub stale: bool,
    /// Its link's smallest round trip over the last one or two windows.
    pub rtt: Option<Duration>,
}

/// What moved the allocation's window ([`SeedWindow`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowPhase {
    /// An allocation check.
    Check,
    /// The first wave's size, from the early measurements.
    FirstWave,
    /// A step of the first wave's ramp that grew it.
    Ramp,
    /// The ramp ended at a queue below the runner.
    RampEnd,
}

impl WindowPhase {
    /// Its name for a table: `check`, `first_wave`, `ramp` or `ramp_end`.
    pub fn name(self) -> &'static str {
        match self {
            WindowPhase::Check => "check",
            WindowPhase::FirstWave => "first_wave",
            WindowPhase::Ramp => "ramp",
            WindowPhase::RampEnd => "ramp_end",
        }
    }
}

/// The allocation's window and budget ([`SeedDecisionKind::Window`]), and
/// at a check the signals the window followed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeedWindow {
    pub phase: WindowPhase,
    /// The budget in stream slots, whether it is measured, and the
    /// capacity reading it was measured from (bytes per second; 0 until
    /// measured).
    pub budget: usize,
    pub measured: bool,
    pub capacity: u64,
    /// The copies in effect, the streams a holder may take (`usize::MAX`
    /// before the budget is measured), and the holders of a copy.
    pub copies: usize,
    pub streams: usize,
    pub holders: usize,
    /// At a check: the holders with a push written or gone stale since
    /// the last, those of them behind and overloaded, the holders with a
    /// subscription, and whether the uplink was saturated.
    pub pushed: usize,
    pub behind: usize,
    pub overloaded: usize,
    pub in_use: usize,
    pub saturated: bool,
    /// Room for every neighbour's every stream.
    pub unconstrained: bool,
}

/// How a chunk left the seed ([`SeedSend`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendHow {
    /// Pushed to a subscriber of its stream.
    Push,
    /// An answer to the peer's request.
    Answer,
}

impl SendHow {
    /// Its name for a table: `push` or `answer`.
    pub fn name(self) -> &'static str {
        match self {
            SendHow::Push => "push",
            SendHow::Answer => "answer",
        }
    }
}

/// What became of a chunk the seed sent ([`SeedSend`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendFate {
    /// Written to the peer's frame.
    Written,
    /// A push dropped at write time for waiting longer than
    /// [`PUSH_STALE`](crate::scheduler_peer::push::PUSH_STALE).
    Stale,
    /// A push dropped at write time: the peer cancelled the chunk.
    Cancelled,
    /// A push dropped at write time: the peer's HAVEs announced the chunk.
    Suppressed,
    /// The peer's queue was full: never queued.
    Refused,
    /// Dropped at write time: its data was gone, data-less, or too large
    /// for an empty frame.
    Dropped,
    /// A push still queued when the peer's channel closed, or the
    /// scheduler went.
    Closed,
}

impl SendFate {
    /// Its name for a table: `written`, `stale`, `cancelled`,
    /// `suppressed`, `refused`, `dropped` or `closed`.
    pub fn name(self) -> &'static str {
        match self {
            SendFate::Written => "written",
            SendFate::Stale => "stale",
            SendFate::Cancelled => "cancelled",
            SendFate::Suppressed => "suppressed",
            SendFate::Refused => "refused",
            SendFate::Dropped => "dropped",
            SendFate::Closed => "closed",
        }
    }
}

/// A chunk the seed pushed or answered, once its send ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeedSend {
    pub peer: PeerKey,
    pub stream: Stream,
    /// The chunk.
    pub bin: Bin,
    pub how: SendHow,
    /// When the push was queued, or the answered request came (a released
    /// park's: when it was released).
    pub queued: Time,
    pub ended: Time,
    pub fate: SendFate,
    /// The frame bytes written for it, integrity included (an answer of
    /// several chunks: its share); 0 unless written.
    pub bytes: u64,
    /// The peer's first CANCEL of the chunk, if it came before the end.
    pub cancel: Option<Time>,
    /// At the end, the peer's HAVEs announced the chunk.
    pub declared: bool,
    /// At the end, the peer was subscribed to the chunk's stream.
    pub subscribed: bool,
}

/// Its name for a table: `subscribed`, `pin`, `seed_holder`,
/// `seed_barred`, `seed_no_copy`, `seed_streams_held`, `unmeasured`,
/// `measured` or `silent`.
pub fn admit_rule_name(rule: AdmitRule) -> &'static str {
    match rule {
        AdmitRule::Subscribed => "subscribed",
        AdmitRule::Pin => "pin",
        AdmitRule::SeedHolder => "seed_holder",
        AdmitRule::SeedBarred => "seed_barred",
        AdmitRule::SeedNoCopy => "seed_no_copy",
        AdmitRule::SeedStreamsHeld => "seed_streams_held",
        AdmitRule::Unmeasured => "unmeasured",
        AdmitRule::Measured => "measured",
        AdmitRule::Silent => "silent",
    }
}

/// Its name for a table: `silent`, `unserved`, `past_copies`, `idle` or
/// `closed`.
pub fn holder_left_name(why: HolderLeft) -> &'static str {
    match why {
        HolderLeft::Silent => "silent",
        HolderLeft::Unserved => "unserved",
        HolderLeft::PastCopies => "past_copies",
        HolderLeft::Idle => "idle",
        HolderLeft::Closed => "closed",
    }
}

impl SeedDecisionKind {
    /// Its name for a table: `request`, `deferred`, `released`, `closed`,
    /// `holder_joined`, `holder_left`, `barred`, `check`, `cancel`,
    /// `window` or `answer_rate`.
    pub fn name(&self) -> &'static str {
        match self {
            SeedDecisionKind::Request(_) => "request",
            SeedDecisionKind::Deferred { .. } => "deferred",
            SeedDecisionKind::Released { .. } => "released",
            SeedDecisionKind::Closed { .. } => "closed",
            SeedDecisionKind::HolderJoined { .. } => "holder_joined",
            SeedDecisionKind::HolderLeft { .. } => "holder_left",
            SeedDecisionKind::Barred { .. } => "barred",
            SeedDecisionKind::Check(_) => "check",
            SeedDecisionKind::Cancel { .. } => "cancel",
            SeedDecisionKind::Window(_) => "window",
            SeedDecisionKind::AnswerRate { .. } => "answer_rate",
        }
    }
}

/// The seed log (see the [module docs](self)).
#[derive(Debug, Default)]
pub(crate) struct SeedLog {
    /// The decisions and the ended sends since the driver's last take, and
    /// the records the caps left out. Bound: [`SEED_LOG_CAP`] each.
    decisions: Vec<SeedDecision>,
    sends: Vec<SeedSend>,
    lost: u64,
    /// The pushes queued and not yet ended, by peer, chunk and the time
    /// their queue entries carry: how many entries. Bound: the peers'
    /// queues', and [`SEED_LOG_CAP`].
    open: BTreeMap<(PeerKey, Bin, Time), u32>,
    /// By chunk, each peer's first CANCEL of it, and the first write of a
    /// push of it to each peer. Bound: the store's window per channel
    /// ([`SeedLog::forget_before`]).
    cancels: BTreeMap<Bin, Vec<(PeerKey, Time)>>,
    written: BTreeMap<Bin, Vec<(PeerKey, Time)>>,
    /// The answer gate's pace as last logged.
    rate: Option<(u64, Option<Duration>, bool)>,
    /// The allocation check under way: each channel's readings so far.
    /// Bound: the channels.
    check: BTreeMap<PeerKey, SeedCheck>,
}

/// What a peer's channel says of a chunk at a send's end
/// ([`SeedSend::declared`], [`SeedSend::subscribed`]).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SendState {
    pub declared: bool,
    pub subscribed: bool,
}

impl SeedLog {
    /// Records a decision about `peer` (`None`: about the allocation).
    pub(crate) fn decide(&mut self, at: Time, peer: Option<PeerKey>, kind: SeedDecisionKind) {
        if self.decisions.len() >= SEED_LOG_CAP {
            self.lost += 1;
        } else {
            self.decisions.push(SeedDecision { at, peer, kind });
        }
    }

    fn push_send(&mut self, s: SeedSend) {
        if self.sends.len() >= SEED_LOG_CAP {
            self.lost += 1;
        } else {
            self.sends.push(s);
        }
    }

    /// A push of chunk `bin` to `peer` was queued at `now` with the entry
    /// time `t`.
    pub(crate) fn push_queued(&mut self, peer: PeerKey, bin: Bin, t: Time) {
        if let Some(n) = self.open.get_mut(&(peer, bin, t)) {
            *n = n.saturating_add(1);
        } else if self.open.len() >= SEED_LOG_CAP {
            self.lost += 1;
        } else {
            self.open.insert((peer, bin, t), 1);
        }
    }

    /// The peer's first CANCEL of `bin`, if one came.
    fn cancelled_at(&self, peer: PeerKey, bin: Bin) -> Option<Time> {
        self.cancels
            .get(&bin)?
            .iter()
            .find(|&&(p, _)| p == peer)
            .map(|&(_, t)| t)
    }

    /// The send of chunk `bin` of `stream` to `peer` (`how`), from the
    /// entry time `t`, ended at `now` (`fate`, `bytes` written). A push
    /// ends its open entry, if it opened in the log.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn ended(
        &mut self,
        peer: PeerKey,
        stream: Stream,
        bin: Bin,
        how: SendHow,
        t: Time,
        fate: SendFate,
        bytes: u64,
        st: SendState,
        now: Time,
    ) {
        if how == SendHow::Push {
            let key = (peer, bin, t);
            if let Some(n) = self.open.get_mut(&key) {
                *n = n.saturating_sub(1);
                if *n == 0 {
                    self.open.remove(&key);
                }
            }
            if fate == SendFate::Written {
                let w = self.written.entry(bin).or_default();
                if !w.iter().any(|&(p, _)| p == peer) {
                    w.push((peer, now));
                }
            }
        }
        let cancel = self.cancelled_at(peer, bin).filter(|&c| c <= now);
        self.push_send(SeedSend {
            peer,
            stream,
            bin,
            how,
            queued: t,
            ended: now,
            fate,
            bytes,
            cancel,
            declared: st.declared,
            subscribed: st.subscribed,
        });
    }

    /// A push of `bin` to `peer` the peer's full queue refused at `now`.
    pub(crate) fn refused(
        &mut self,
        peer: PeerKey,
        stream: Stream,
        bin: Bin,
        how: SendHow,
        st: SendState,
        now: Time,
    ) {
        let cancel = self.cancelled_at(peer, bin);
        self.push_send(SeedSend {
            peer,
            stream,
            bin,
            how,
            queued: now,
            ended: now,
            fate: SendFate::Refused,
            bytes: 0,
            cancel,
            declared: st.declared,
            subscribed: st.subscribed,
        });
    }

    /// The peer's CANCEL of `bin` came at `now`: for each of its chunks,
    /// where our push of it stood ([`CancelOpen`]), a decision each for
    /// the chunks we queued or wrote to it, one for the bin if none;
    /// `stream` names a chunk's stream.
    pub(crate) fn cancel(
        &mut self,
        peer: PeerKey,
        bin: Bin,
        stream: impl Fn(Bin) -> Stream,
        now: Time,
    ) {
        let (mut x, r) = bin.base();
        let mut any = false;
        while x <= r {
            let c = self.cancels.entry(x).or_default();
            if !c.iter().any(|&(p, _)| p == peer) {
                c.push((peer, now));
            }
            let queued = self
                .open
                .range((peer, x, Time::NIL)..=(peer, x, Time::MAX))
                .next()
                .is_some();
            let written = self
                .written
                .get(&x)
                .is_some_and(|w| w.iter().any(|&(p, _)| p == peer));
            let open = if queued {
                CancelOpen::Queued
            } else if written {
                CancelOpen::Written
            } else {
                CancelOpen::None
            };
            if open != CancelOpen::None {
                any = true;
                let stream = stream(x);
                self.decide(
                    now,
                    Some(peer),
                    SeedDecisionKind::Cancel {
                        stream,
                        bin: x,
                        open,
                    },
                );
            }
            x += 2;
        }
        if !any {
            let stream = stream(bin.base_left());
            self.decide(
                now,
                Some(peer),
                SeedDecisionKind::Cancel {
                    stream,
                    bin,
                    open: CancelOpen::None,
                },
            );
        }
    }

    /// Ends every push still open to `peer`, or to every peer, as closed
    /// at `now`; `stream` names a chunk's stream, `state` what its
    /// channel says of it.
    pub(crate) fn close(
        &mut self,
        peer: Option<PeerKey>,
        stream: impl Fn(Bin) -> Stream,
        state: impl Fn(PeerKey, Bin) -> SendState,
        now: Time,
    ) {
        let keys: Vec<((PeerKey, Bin, Time), u32)> = match peer {
            Some(p) => self
                .open
                .range((p, Bin(0), Time::NIL)..=(p, Bin(u64::MAX), Time::MAX))
                .map(|(&k, &n)| (k, n))
                .collect(),
            None => self.open.iter().map(|(&k, &n)| (k, n)).collect(),
        };
        for ((p, b, t), n) in keys {
            for _ in 0..n {
                self.ended(
                    p,
                    stream(b),
                    b,
                    SendHow::Push,
                    t,
                    SendFate::Closed,
                    0,
                    state(p, b),
                    now,
                );
            }
        }
    }

    /// An allocation check starts: the last one's readings go.
    pub(crate) fn check_begin(&mut self) {
        self.check.clear();
    }

    /// The readings of `peer` at the check under way.
    pub(crate) fn check_of(&mut self, peer: PeerKey) -> &mut SeedCheck {
        self.check.entry(peer).or_default()
    }

    /// Takes `peer`'s readings at the check under way, a holder's: its
    /// [`SeedDecisionKind::Check`].
    pub(crate) fn checked(&mut self, peer: PeerKey, now: Time) {
        let c = self.check.remove(&peer).unwrap_or_default();
        self.decide(now, Some(peer), SeedDecisionKind::Check(c));
    }

    /// Forgets the cancels and writes of chunks before `start`, the
    /// store's window.
    pub(crate) fn forget_before(&mut self, start: Bin) {
        self.cancels = self.cancels.split_off(&start);
        self.written = self.written.split_off(&start);
    }

    /// The answer gate's pace at `now`: a decision if it changed.
    pub(crate) fn answer_rate(
        &mut self,
        rate: u64,
        reading: Option<Duration>,
        engaged: bool,
        now: Time,
    ) {
        let pace = (rate, reading, engaged);
        if self.rate != Some(pace) {
            self.rate = Some(pace);
            self.decide(
                now,
                None,
                SeedDecisionKind::AnswerRate {
                    rate,
                    reading,
                    engaged,
                },
            );
        }
    }

    /// The records since the last take, and how many the caps left out.
    pub(crate) fn take(&mut self) -> SeedLogTake {
        SeedLogTake {
            decisions: std::mem::take(&mut self.decisions),
            sends: std::mem::take(&mut self.sends),
            lost: std::mem::take(&mut self.lost),
        }
    }

    /// Whether it holds no record, open or ended, and counted none lost.
    pub(crate) fn is_empty(&self) -> bool {
        self.decisions.is_empty() && self.sends.is_empty() && self.lost == 0 && self.open.is_empty()
    }

    /// Adds the records of a scheduler that goes, its open pushes closed
    /// ([`super::PeerSwarmScheduler::take_seed_log`]).
    pub(crate) fn absorb(&mut self, o: SeedLogTake) {
        for d in o.decisions {
            self.decide(d.at, d.peer, d.kind);
        }
        for s in o.sends {
            self.push_send(s);
        }
        self.lost += o.lost;
    }
}

/// A seed log's records since the driver's last take
/// ([`Runner::take_seed_log`](crate::Runner::take_seed_log)), and how many
/// its caps left out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SeedLogTake {
    pub decisions: Vec<SeedDecision>,
    pub sends: Vec<SeedSend>,
    pub lost: u64,
}

impl SeedLogTake {
    /// Adds `o`'s records behind these.
    pub fn extend(&mut self, o: SeedLogTake) {
        self.decisions.extend(o.decisions);
        self.sends.extend(o.sends);
        self.lost += o.lost;
    }
}

/// A column of the sims' `seed_decisions` and `seed_sends` tables: its name
/// and type (`int`, `text` or `bool`). The sims write a record's time,
/// their hosts and the peer's incarnation first (`t_ms`, `node`, `peer`,
/// `peer_incarnation`), then these, from [`decision_fields`] and
/// [`send_fields`], so both write the same columns.
pub type Column = (&'static str, &'static str);

/// The `seed_decisions` table's columns past the sims' own; see
/// [`decision_fields`] for what each holds.
pub const DECISION_COLUMNS: [Column; 38] = [
    ("kind", "text"),
    ("stream", "int"),
    ("seq", "int"),
    ("why", "text"),
    ("granted", "bool"),
    ("subscribers", "int"),
    ("holders", "int"),
    ("copies", "int"),
    ("held", "int"),
    ("per_holder", "int"),
    ("satellite", "bool"),
    ("kept", "bool"),
    ("rtt_ms", "int"),
    ("until_ms", "int"),
    ("bars", "int"),
    ("reporters", "int"),
    ("due_ms", "int"),
    ("judged", "bool"),
    ("frontier_seq", "int"),
    ("hole_seq", "int"),
    ("state_age_ms", "int"),
    ("silent", "int"),
    ("queue_ms", "int"),
    ("delivered", "int"),
    ("wait_ms", "int"),
    ("stale", "bool"),
    ("budget", "int"),
    ("measured", "bool"),
    ("capacity", "int"),
    ("pushed", "int"),
    ("behind", "int"),
    ("overloaded", "int"),
    ("in_use", "int"),
    ("saturated", "bool"),
    ("unconstrained", "bool"),
    ("rate", "int"),
    ("reading_ms", "int"),
    ("engaged", "bool"),
];

/// The `seed_sends` table's columns past the sims' own; see
/// [`send_fields`] for what each holds.
pub const SEND_COLUMNS: [Column; 9] = [
    ("how", "text"),
    ("stream", "int"),
    ("seq", "int"),
    ("queued_ms", "int"),
    ("fate", "text"),
    ("bytes", "int"),
    ("cancel_ms", "int"),
    ("declared", "bool"),
    ("subscribed", "bool"),
];

/// `d`'s fields by [`DECISION_COLUMNS`], `ms` reading the node's clock as
/// the table's ms; empty where its kind has none. `kind` is
/// [`SeedDecisionKind::name`], and by kind:
///
/// - `request`: `stream`, `why` (the rule, [`admit_rule_name`]),
///   `granted`, `subscribers` (before), `holders`, `copies`, `held`,
///   `per_holder` (empty before the budget is measured);
/// - `deferred`: `stream`;
/// - `released`: `stream`, `subscribers` (after), `kept`;
/// - `closed`: `stream`, `why` ([`SeedClose::name`]), `subscribers`
///   (after);
/// - `holder_joined`: `satellite`, `rtt_ms`, `holders`, `copies`;
/// - `holder_left`: `why` ([`holder_left_name`]), `rtt_ms`, `holders`,
///   `copies`;
/// - `barred`: `why` ([`SeedBar::name`]), `until_ms`, `bars`,
///   `reporters`;
/// - `check` ([`SeedCheck`]): `satellite`, `held` (its streams), `stream`,
///   `seq` and `due_ms` (the due push), `judged`, `frontier_seq`,
///   `hole_seq`, `state_age_ms` (the decision's time less the STATE's),
///   `silent`, `queue_ms`, `delivered`, `wait_ms`, `stale`, `rtt_ms`;
/// - `cancel`: `stream`, `seq` (the chunk, or the CANCEL's bin's first
///   with `none`), `why` ([`CancelOpen::name`]);
/// - `window` ([`SeedWindow`]): `why` ([`WindowPhase::name`]), `budget`,
///   `measured`, `capacity`, `copies`, `per_holder`, `holders`, `pushed`,
///   `behind`, `overloaded`, `in_use`, `saturated`, `unconstrained`;
/// - `answer_rate`: `rate`, `reading_ms`, `engaged`.
pub fn decision_fields(d: &SeedDecision, ms: &dyn Fn(Time) -> i64) -> Vec<String> {
    use SeedDecisionKind as K;
    let mut row: BTreeMap<&'static str, String> = BTreeMap::new();
    let mut put = |c: &'static str, v: String| {
        row.insert(c, v);
    };
    let dur = |x: Option<Duration>| x.map_or(String::new(), |x| x.as_millis().to_string());
    let count = |n: usize| {
        if n == usize::MAX {
            String::new()
        } else {
            n.to_string()
        }
    };
    let seq = |b: Bin| b.base_offset().to_string();
    put("kind", d.kind.name().to_owned());
    match d.kind {
        K::Request(q) => {
            put("stream", q.stream.to_string());
            put("why", admit_rule_name(q.rule).to_owned());
            put("granted", q.granted.to_string());
            put("subscribers", q.subscribers.to_string());
            put("holders", q.holders.to_string());
            put("copies", q.copies.to_string());
            put("held", q.held.to_string());
            put("per_holder", count(q.per_holder));
        }
        K::Deferred { stream } => put("stream", stream.to_string()),
        K::Released {
            stream,
            subscribers,
            kept,
        } => {
            put("stream", stream.to_string());
            put("subscribers", subscribers.to_string());
            put("kept", kept.to_string());
        }
        K::Closed {
            stream,
            why,
            subscribers,
        } => {
            put("stream", stream.to_string());
            put("why", why.name().to_owned());
            put("subscribers", subscribers.to_string());
        }
        K::HolderJoined {
            satellite,
            rtt,
            holders,
            copies,
        } => {
            put("satellite", satellite.to_string());
            put("rtt_ms", dur(rtt));
            put("holders", holders.to_string());
            put("copies", copies.to_string());
        }
        K::HolderLeft {
            why,
            rtt,
            holders,
            copies,
        } => {
            put("why", holder_left_name(why).to_owned());
            put("rtt_ms", dur(rtt));
            put("holders", holders.to_string());
            put("copies", copies.to_string());
        }
        K::Barred {
            why,
            until,
            bars,
            reporters,
        } => {
            put("why", why.name().to_owned());
            put("until_ms", ms(until).to_string());
            put("bars", bars.to_string());
            put("reporters", reporters.to_string());
        }
        K::Check(c) => {
            put("satellite", c.satellite.to_string());
            put("held", c.streams.to_string());
            if let Some((bin, at)) = c.due {
                put("seq", seq(bin));
                put("due_ms", ms(at).to_string());
            }
            if let Some(s) = c.stream {
                put("stream", s.to_string());
            }
            put("judged", c.judged.to_string());
            if let Some(b) = c.frontier {
                put("frontier_seq", seq(b));
            }
            if let Some(b) = c.hole {
                put("hole_seq", seq(b));
            }
            put("state_age_ms", dur(c.state_at.map(|s| d.at - s)));
            put("silent", c.silent.to_string());
            put("queue_ms", dur(c.queue));
            put(
                "delivered",
                c.delivered.map_or(String::new(), |x| x.to_string()),
            );
            put("wait_ms", dur(c.wait));
            put("stale", c.stale.to_string());
            put("rtt_ms", dur(c.rtt));
        }
        K::Cancel { stream, bin, open } => {
            put("stream", stream.to_string());
            put("seq", seq(bin.base_left()));
            put("why", open.name().to_owned());
        }
        K::Window(w) => {
            put("why", w.phase.name().to_owned());
            put("budget", w.budget.to_string());
            put("measured", w.measured.to_string());
            put("capacity", w.capacity.to_string());
            put("copies", w.copies.to_string());
            put("per_holder", count(w.streams));
            put("holders", w.holders.to_string());
            put("pushed", w.pushed.to_string());
            put("behind", w.behind.to_string());
            put("overloaded", w.overloaded.to_string());
            put("in_use", w.in_use.to_string());
            put("saturated", w.saturated.to_string());
            put("unconstrained", w.unconstrained.to_string());
        }
        K::AnswerRate {
            rate,
            reading,
            engaged,
        } => {
            put("rate", rate.to_string());
            put("reading_ms", dur(reading));
            put("engaged", engaged.to_string());
        }
    }
    DECISION_COLUMNS
        .iter()
        .map(|(c, _)| row.remove(c).unwrap_or_default())
        .collect()
}

/// `s`'s fields by [`SEND_COLUMNS`], `ms` reading the node's clock as the
/// table's ms: `how` ([`SendHow::name`]), `stream`, `seq`, `queued_ms`,
/// `fate` ([`SendFate::name`]: `suppressed` here, in the HAVE world, for a
/// push the peer's HAVEs announced), `bytes` (empty unless written),
/// `cancel_ms` (empty without a CANCEL by the end), `declared`,
/// `subscribed`. The sims' `t_ms` is the end's.
pub fn send_fields(s: &SeedSend, ms: &dyn Fn(Time) -> i64) -> Vec<String> {
    vec![
        s.how.name().to_owned(),
        s.stream.to_string(),
        s.bin.base_offset().to_string(),
        ms(s.queued).to_string(),
        s.fate.name().to_owned(),
        if s.fate == SendFate::Written {
            s.bytes.to_string()
        } else {
            String::new()
        },
        s.cancel.map_or(String::new(), |c| ms(c).to_string()),
        s.declared.to_string(),
        s.subscribed.to_string(),
    ]
}
