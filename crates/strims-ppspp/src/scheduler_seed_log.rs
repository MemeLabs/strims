// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The seed log: what a seed decided about its neighbours and what became
//! of every chunk it sent, record by record, for a driver's tables to join
//! with what its neighbours held. On only at a seed, with the runner's
//! log that [`Runner::take_seed_log`](crate::Runner::take_seed_log) names,
//! and taken with it; it draws no randomness and no decision reads it, so a
//! run with it traces as one without.
//!
//! The log is the same wherever peers say what they hold in HAVEs or in
//! STATE summaries: what a peer said is the caller's reading
//! ([`SeedSend::declared`], [`SeedCheck`]), and a kind a world doesn't produce
//! (a HAVE-suppressed push, a STATE's frontier) is simply absent.
//!
//! - **Decisions** ([`SeedDecision`]): each STREAM_REQUEST answered and
//!   the rule that answered it, with the stream's subscribers before it;
//!   requests the first wave's ramp held; STREAM_CANCELs and the
//!   subscriptions the seed closed, and why; holders joining, leaving and
//!   barred; each holder's reading at each allocation check (its sampled
//!   push and what the holder said of it, its link's queue and delivery
//!   rate, its pushes' wait); each CANCEL and where the seed's push of the
//!   chunk stood then; the allocation's window at each check and each ramp
//!   step; and the answer gate's pace as it changes.
//! - **Sends** ([`SeedSend`]): one per chunk the seed pushed or answered,
//!   once it ended: written to the frame, dropped at write time (stale,
//!   cancelled by the peer, announced by its HAVEs, or unwritable),
//!   refused by the peer's full queue, or still queued when the channel
//!   closed. A push opens when it
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
    /// the peer held and the streams a holder may hold (`None` before the
    /// budget is measured), as its admission read them; `None` where no
    /// admission of the seed's read them (a pin, a subscriber asking
    /// again, a silent peer).
    pub holders: Option<usize>,
    pub copies: Option<usize>,
    pub held: Option<usize>,
    pub per_holder: Option<usize>,
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
    /// in that interval), its stream and when it was written, which the
    /// holder must say it holds by this check; `judged`: it was written at
    /// least
    /// [`ANNOUNCE_TIMEOUT`](crate::scheduler_peer::push::seed::ANNOUNCE_TIMEOUT)
    /// before, so this check judged it.
    pub due: Option<(Bin, Time)>,
    pub stream: Option<Stream>,
    pub judged: bool,
    /// Where peers send STATE summaries: the holder's latest STATE's
    /// newest chunk of the due push's stream, the newest before it the
    /// peer lacked, and when that STATE arrived; `None` without one (or
    /// without a due push, or where peers send HAVEs).
    pub frontier: Option<Bin>,
    pub hole: Option<Bin>,
    pub state_at: Option<Time>,
    /// The checks in a row at which the holder didn't say it holds its due
    /// push (its HAVEs, or its latest STATE's frontier), after this one.
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
    /// capacity reading it was measured from (bytes per second; `None`
    /// until measured).
    pub budget: usize,
    pub measured: bool,
    pub capacity: Option<u64>,
    /// The copies in effect, the streams a holder may take (`None` before
    /// the budget is measured), and the holders of a copy.
    pub copies: usize,
    pub streams: Option<usize>,
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
    /// A push dropped at write time: the peer's HAVEs announced the chunk
    /// (where peers send HAVEs).
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
    /// At the end, the peer said it holds the chunk: its HAVEs announced
    /// it, or, where peers send STATE summaries, its latest STATE covered
    /// it (at or after its oldest chunk, by its frontier, after its hole).
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
    /// ([`SeedLog::close`]).
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

/// A column's type in the sims' tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnType {
    Int,
    Text,
    Bool,
}

/// A column of the sims' `seed_decisions` and `seed_sends` tables: its
/// name, type and doc. Both sims write every column through
/// [`decision_fields`] and [`send_fields`], and document them with
/// [`doc_row`]'s text, each doc the column's own parenthesis there, so the
/// tables and their docs are one in both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Column {
    pub name: &'static str,
    pub ty: ColumnType,
    pub doc: &'static str,
}

const fn col(name: &'static str, ty: ColumnType, doc: &'static str) -> Column {
    Column { name, ty, doc }
}

use ColumnType::{Bool, Int, Text};

/// The `seed_decisions` table's grain, as both sims' doc rows give it
/// before the setting that writes it ([`doc_row`]).
pub const DECISIONS_GRAIN: &str =
    "decision of the seed's seed log ([`strims_ppspp::SeedDecision`])";

/// The `seed_sends` table's grain, as both sims' doc rows give it
/// before the setting that writes it ([`doc_row`]).
pub const SENDS_GRAIN: &str = "chunk the seed pushed or answered, once its send ended, from its seed log ([`strims_ppspp::SeedSend`])";

/// The `seed_decisions` table's columns ([`decision_fields`]); a row's
/// kind says which hold its numbers, the rest empty.
pub const DECISION_COLUMNS: [Column; 43] = [
    col(
        "t_ms",
        Int,
        "when the seed decided, in ms of true time from the stream's start, as the sim's other tables count it",
    ),
    col(
        "node",
        Int,
        "the seed: its host in strims-sim, its node in the ppspp sim",
    ),
    col(
        "n",
        Int,
        "the decision's place in the seed's log, from 0, in the order the seed logged its decisions; with node it keys the table",
    ),
    col(
        "peer",
        Int,
        "the neighbour the decision is about, numbered as node is; -1 for a decision about the allocation, a window or answer_rate row",
    ),
    col(
        "peer_incarnation",
        Int,
        "the neighbour's incarnation then, as the sim's arrivals count them; empty without a peer",
    ),
    col(
        "kind",
        Text,
        "what the seed decided: request, a STREAM_REQUEST answered; deferred, one the first wave's ramp held, answered later by a request row; released, the peer's STREAM_CANCEL of a subscription; closed, a subscription the seed ended; holder_joined and holder_left, the peer took or gave up a copy; barred, the peer may hold none for a while; check, a holder's reading at an allocation check, once a second; cancel, the peer's CANCEL of a chunk; window, the allocation's window; answer_rate, the answer gate's pace as it changed",
    ),
    col(
        "stream",
        Int,
        "the stream of a request, deferred, released or closed row; of a check's due push; of a cancel's chunk",
    ),
    col(
        "seq",
        Int,
        "a chunk: a check's due push, or a cancel's chunk (the CANCEL's bin's first where why is none)",
    ),
    col(
        "why",
        Text,
        "a request's rule: seed_holder, seed_no_copy, seed_barred, seed_streams_held, subscribed (the peer asked again), silent, pin, measured or unmeasured; a closed row's reason: a holder's leaving's (silent, unserved, past_copies, idle or closed), silent (the peer went silent), spill, pin, saturated, declined, or channel (its channel closed: no STREAM_CLOSE went); a holder_left's: silent, unserved, past_copies, idle or closed; a bar's: silent or unserved; a cancel's, where the seed's push of the chunk stood: none, queued (dropped at write time) or written; a window's phase: check, first_wave, ramp or ramp_end",
    ),
    col(
        "granted",
        Bool,
        "a request's: accepted, a STREAM_OPEN following, or declined with a STREAM_CLOSE",
    ),
    col(
        "subscribers",
        Int,
        "the stream's subscribers: a request's before its answer, a released or closed row's left after it",
    ),
    col(
        "holders",
        Int,
        "the holders of a copy: a request's as the seed's admission read them (empty where none did: a pin, a peer asking again, a silent one); a holder_joined's after it; a holder_left's left; a window's",
    ),
    col(
        "copies",
        Int,
        "the copies in effect: a request's as the seed's admission read them (empty where none did); a holder_joined's, a holder_left's or a window's",
    ),
    col(
        "held",
        Int,
        "the streams the peer held: a request's as the seed's admission read them (empty where none did); a check's, the holder's subscriptions",
    ),
    col(
        "per_holder",
        Int,
        "the streams a holder may hold: a request's (empty where no admission read it) or a window's; empty before the budget is measured",
    ),
    col(
        "satellite",
        Bool,
        "a holder_joined's or a check's: the peer holds streams spilled from other holders, with no copy of its own",
    ),
    col(
        "kept",
        Bool,
        "a released row's: a holder's cancel of a stream of its copy, which the seed keeps for a near satellite to take while no one else does",
    ),
    col(
        "rtt_ms",
        Int,
        "the peer's link's smallest round trip over the last one or two windows: a holder_joined's, a holder_left's or a check's",
    ),
    col(
        "until_ms",
        Int,
        "a bar's: the peer holds no copy until then, in ms as t_ms",
    ),
    col(
        "bars",
        Int,
        "a bar's: the peer's bars so far, this one included",
    ),
    col(
        "reporters",
        Int,
        "a bar's: the reporters that voted on the peer at the latest check",
    ),
    col(
        "due_ms",
        Int,
        "a check's: when the due push was written, the first written to the holder in the interval before last, in ms as t_ms",
    ),
    col(
        "judged",
        Bool,
        "a check's: the due push was written at least ANNOUNCE_TIMEOUT before, so this check judged it",
    ),
    col(
        "frontier_seq",
        Int,
        "a check's, where peers send STATE summaries: the holder's latest STATE's newest chunk of the due push's stream; empty without one, or where peers send HAVEs",
    ),
    col(
        "hole_seq",
        Int,
        "a check's: that STATE's newest chunk before its frontier the holder lacked; empty without one",
    ),
    col(
        "state_age_ms",
        Int,
        "a check's: that STATE's age at the check; empty without one",
    ),
    col(
        "silent",
        Int,
        "a check's: the checks in a row at which the holder didn't say it holds its due push (its HAVEs didn't announce it, or its latest STATE's frontier hadn't reached it), after this one; a bar comes at SILENT_CHECKS",
    ),
    col(
        "queue_ms",
        Int,
        "a check's: how far the holder's link's least round trip since the last check stood above its smallest, the queue below the runner its pushes waited behind",
    ),
    col(
        "delivered_bps",
        Int,
        "a check's: the least delivery rate the holder's link read since the last check, in bytes per second",
    ),
    col(
        "wait_ms",
        Int,
        "a check's: the shortest wait in the seed's queue of a push written to the holder or cancelled by it since the last check",
    ),
    col(
        "stale",
        Bool,
        "a check's: a push to the holder went stale since the last check",
    ),
    col("budget", Int, "a window's: the budget in stream slots"),
    col("measured", Bool, "a window's: the budget is measured"),
    col(
        "capacity_bps",
        Int,
        "a window's: the capacity reading the budget was measured from, in bytes per second; empty until measured",
    ),
    col(
        "pushed",
        Int,
        "a window's at a check: the holders with a push written or gone stale since the last check",
    ),
    col(
        "behind",
        Int,
        "a window's at a check: those of them behind, a push waiting past PUSH_WAIT_TARGET or gone stale",
    ),
    col(
        "overloaded",
        Int,
        "a window's at a check: those of them overloaded, a push waiting past PUSH_WAIT_OVERLOAD or gone stale",
    ),
    col(
        "in_use",
        Int,
        "a window's at a check: the holders of a copy with a subscription",
    ),
    col(
        "saturated",
        Bool,
        "a window's at a check: the uplink was busy at least SHED_UTILIZATION of the time",
    ),
    col(
        "unconstrained",
        Bool,
        "a window's: the budget has room for every neighbour's every stream",
    ),
    col(
        "rate_bps",
        Int,
        "an answer_rate's: the pace of the node's hybrid answers, in bytes per second",
    ),
    col(
        "reading_ms",
        Int,
        "an answer_rate's: the node's queueing delay below the runner the answer gate read; empty without a reading",
    ),
    col(
        "engaged",
        Bool,
        "an answer_rate's: the pace binds the answers",
    ),
];

/// The `seed_sends` table's columns ([`send_fields`]).
pub const SEND_COLUMNS: [Column; 14] = [
    col(
        "t_ms",
        Int,
        "when the send ended, written to the frame or dropped, in ms of true time from the stream's start, as the sim's other tables count it",
    ),
    col(
        "node",
        Int,
        "the seed: its host in strims-sim, its node in the ppspp sim",
    ),
    col(
        "n",
        Int,
        "the send's place in the seed's log, from 0, in the order the seed's sends ended; with node it keys the table",
    ),
    col("peer", Int, "the neighbour sent to, numbered as node is"),
    col(
        "peer_incarnation",
        Int,
        "the neighbour's incarnation when the chunk was queued, as the sim's arrivals count them",
    ),
    col(
        "how",
        Text,
        "push, to a subscriber of the chunk's stream, or answer, to the peer's request",
    ),
    col("stream", Int, "the chunk's stream"),
    col("seq", Int, "the chunk"),
    col(
        "queued_ms",
        Int,
        "when the push was queued, or the answered request came (a released park's: its release), in ms as t_ms",
    ),
    col(
        "fate",
        Text,
        "written, to the peer's frame; stale, a push dropped at write time for waiting past PUSH_STALE; cancelled, a push dropped at write time, the peer having cancelled the chunk; suppressed, a push dropped at write time, the peer's HAVEs having announced the chunk (where peers send HAVEs); refused, the peer's queue was full; dropped, unwritable at write time (its data gone, data-less, or too large for an empty frame); closed, a push still queued when the peer's channel closed",
    ),
    col(
        "bytes",
        Int,
        "the frame bytes written for the chunk, integrity included (an answer of several chunks: its share); empty unless written",
    ),
    col(
        "cancel_ms",
        Int,
        "the peer's first CANCEL of the chunk, if it came by the end, in ms as t_ms; empty without one",
    ),
    col(
        "declared",
        Bool,
        "at the end, the peer said it holds the chunk: its HAVEs announced it, or, where peers send STATE summaries, its latest STATE covered it (at or after its oldest chunk, by its frontier, after its hole)",
    ),
    col(
        "subscribed",
        Bool,
        "at the end, the peer was subscribed to the chunk's stream",
    ),
];

/// The columns a sim fills for a row ([`decision_fields`], [`send_fields`]):
/// its time in the tables' ms, the seed (`node`), the record's place in the
/// seed's log of its kind (`n`: the order the sim took the records, a
/// decision's and a send's apart), and the peer with its incarnation then,
/// each numbered as the sim numbers its hosts or nodes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowHead {
    pub t_ms: i64,
    pub node: u64,
    pub n: u64,
    pub peer: Option<u64>,
    pub peer_incarnation: Option<u64>,
}

impl RowHead {
    fn put(&self, row: &mut BTreeMap<&'static str, String>) {
        row.insert("t_ms", self.t_ms.to_string());
        row.insert("node", self.node.to_string());
        row.insert("n", self.n.to_string());
        row.insert("peer", self.peer.map_or("-1".to_owned(), |p| p.to_string()));
        row.insert("peer_incarnation", opt(self.peer_incarnation));
    }
}

fn opt<T: ToString>(x: Option<T>) -> String {
    x.map_or(String::new(), |x| x.to_string())
}

fn ordered(columns: &[Column], mut row: BTreeMap<&'static str, String>) -> Vec<String> {
    columns
        .iter()
        .map(|c| row.remove(c.name).unwrap_or_default())
        .collect()
}

/// `d`'s row by [`DECISION_COLUMNS`] (whose docs say what each holds),
/// `ms` reading the node's clock as the table's ms; empty where its kind
/// has none.
pub fn decision_fields(head: &RowHead, d: &SeedDecision, ms: &dyn Fn(Time) -> i64) -> Vec<String> {
    use SeedDecisionKind as K;
    let mut row: BTreeMap<&'static str, String> = BTreeMap::new();
    head.put(&mut row);
    let mut put = |c: &'static str, v: String| {
        row.insert(c, v);
    };
    let dur = |x: Option<Duration>| opt(x.map(|x| x.as_millis()));
    let seq = |b: Bin| b.base_offset().to_string();
    put("kind", d.kind.name().to_owned());
    match d.kind {
        K::Request(q) => {
            put("stream", q.stream.to_string());
            put("why", admit_rule_name(q.rule).to_owned());
            put("granted", q.granted.to_string());
            put("subscribers", q.subscribers.to_string());
            put("holders", opt(q.holders));
            put("copies", opt(q.copies));
            put("held", opt(q.held));
            put("per_holder", opt(q.per_holder));
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
            put("stream", opt(c.stream));
            put("judged", c.judged.to_string());
            put("frontier_seq", opt(c.frontier.map(seq)));
            put("hole_seq", opt(c.hole.map(seq)));
            put("state_age_ms", dur(c.state_at.map(|s| d.at - s)));
            put("silent", c.silent.to_string());
            put("queue_ms", dur(c.queue));
            put("delivered_bps", opt(c.delivered));
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
            put("capacity_bps", opt(w.capacity));
            put("copies", w.copies.to_string());
            put("per_holder", opt(w.streams));
            put("holders", w.holders.to_string());
            put("unconstrained", w.unconstrained.to_string());
            // The signals are a check's alone.
            if w.phase == WindowPhase::Check {
                put("pushed", w.pushed.to_string());
                put("behind", w.behind.to_string());
                put("overloaded", w.overloaded.to_string());
                put("in_use", w.in_use.to_string());
                put("saturated", w.saturated.to_string());
            }
        }
        K::AnswerRate {
            rate,
            reading,
            engaged,
        } => {
            put("rate_bps", rate.to_string());
            put("reading_ms", dur(reading));
            put("engaged", engaged.to_string());
        }
    }
    ordered(&DECISION_COLUMNS, row)
}

/// `s`'s row by [`SEND_COLUMNS`] (whose docs say what each holds), `ms`
/// reading the node's clock as the table's ms; `head.t_ms` is the send's
/// end.
pub fn send_fields(head: &RowHead, s: &SeedSend, ms: &dyn Fn(Time) -> i64) -> Vec<String> {
    let mut row: BTreeMap<&'static str, String> = BTreeMap::new();
    head.put(&mut row);
    let written = s.fate == SendFate::Written;
    for (c, v) in [
        ("how", s.how.name().to_owned()),
        ("stream", s.stream.to_string()),
        ("seq", s.bin.base_offset().to_string()),
        ("queued_ms", ms(s.queued).to_string()),
        ("fate", s.fate.name().to_owned()),
        ("bytes", opt(written.then_some(s.bytes))),
        ("cancel_ms", opt(s.cancel.map(ms))),
        ("declared", s.declared.to_string()),
        ("subscribed", s.subscribed.to_string()),
    ] {
        row.insert(c, v);
    }
    ordered(&SEND_COLUMNS, row)
}

/// A table's doc row, as both sims' rustdoc carries it and `hack/sim-db`'s
/// catalog reads it: `| `name` | grain, with `knob` | `column` (doc), ... |`,
/// `knob` the sim's setting that writes the table, each column's doc its
/// own parenthesis.
pub fn doc_row(name: &str, grain: &str, knob: &str, columns: &[Column]) -> String {
    let cells: Vec<String> = columns
        .iter()
        .map(|c| format!("`{}` ({})", c.name, c.doc))
        .collect();
    format!(
        "| `{name}` | {grain}, with `{knob}` | {} |",
        cells.join(", ")
    )
}

/// Both tables' doc rows ([`doc_row`]) in a sim whose setting `knob`
/// writes them (strims-sim's `STRIMS_SIM_SPEC=1`, the ppspp sim's
/// `SIM_SPEC=1`): `seed_decisions`', then `seed_sends`'.
pub fn doc_rows(knob: &str) -> [String; 2] {
    [
        doc_row("seed_decisions", DECISIONS_GRAIN, knob, &DECISION_COLUMNS),
        doc_row("seed_sends", SENDS_GRAIN, knob, &SEND_COLUMNS),
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Each column's doc is its own: its parenthesis balanced and free of
    /// what would end it or the row (`|`, a backtick), every name once.
    #[test]
    fn every_column_has_a_doc_of_its_own() {
        for cols in [&DECISION_COLUMNS[..], &SEND_COLUMNS[..]] {
            let mut names: Vec<&str> = cols.iter().map(|c| c.name).collect();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), cols.len());
            for c in cols {
                assert!(!c.doc.is_empty(), "{}", c.name);
                assert!(!c.doc.contains(['|', '`']), "{}", c.name);
                let mut depth = 0i32;
                for ch in c.doc.chars() {
                    depth += match ch {
                        '(' => 1,
                        ')' => -1,
                        _ => 0,
                    };
                    assert!(depth >= 0, "{}", c.name);
                }
                assert_eq!(depth, 0, "{}", c.name);
            }
        }
    }

    /// A row fills every column, the head's from the sim's part.
    #[test]
    fn a_row_fills_the_columns_in_order() {
        let head = RowHead {
            t_ms: 7,
            node: 0,
            n: 3,
            peer: None,
            peer_incarnation: None,
        };
        let d = SeedDecision {
            at: Time::EPOCH,
            peer: None,
            kind: SeedDecisionKind::AnswerRate {
                rate: 1000,
                reading: None,
                engaged: true,
            },
        };
        let row = decision_fields(&head, &d, &|_| 0);
        assert_eq!(row.len(), DECISION_COLUMNS.len());
        let at = |name: &str| {
            let i = DECISION_COLUMNS
                .iter()
                .position(|c| c.name == name)
                .unwrap();
            row[i].as_str()
        };
        assert_eq!(
            [
                at("t_ms"),
                at("n"),
                at("peer"),
                at("peer_incarnation"),
                at("kind"),
                at("rate_bps"),
                at("engaged")
            ],
            ["7", "3", "-1", "", "answer_rate", "1000", "true"]
        );
        assert_eq!(at("reading_ms"), "");
    }
}
