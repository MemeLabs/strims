// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The tap: what the runner and its schedulers decide that the wire
//! doesn't show, for a driver that captures a node's traffic.
//!
//! Off by default. While off, a decision costs one flag check and the
//! buffers hold no memory. While on, events go into buffers that keep
//! their capacity between drains, so after warm-up they don't allocate;
//! [`Runner::drain_tap`](crate::Runner::drain_tap) empties them. The tap
//! only observes: it draws no randomness and changes no state the
//! schedulers read.
//!
//! A decision carries the rule that made it and the numbers it compared,
//! as the scheduler had them. Rates are bytes per second, rounded down;
//! `None` is unknown.

use strims_base::time::{Duration, Time};
use strims_binmap::Bin;

use crate::codec::Stream;
use crate::runner::{PeerKey, SwarmKey};

/// Events a buffer holds between drains; later ones are counted as lost.
pub const TAP_CAP: usize = 4096;

/// What the tap reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TapLevel {
    #[default]
    Off,
    /// Peers, swarms and channels coming and going.
    Lifecycle,
    /// The lifecycle and every decision.
    All,
}

/// One event: what happened, in which swarm, about which peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TapEvent {
    pub swarm: Option<SwarmKey>,
    pub peer: Option<PeerKey>,
    pub kind: TapKind,
}

/// What happened. A chunk bin is a single chunk; a data bin may span
/// several. "The parent" is the peer pushing us a stream, "the child" the
/// peer we push one to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapKind {
    /// [`Runner::add_peer`](crate::Runner::add_peer).
    PeerAdded,
    /// [`Runner::remove_peer`](crate::Runner::remove_peer).
    PeerRemoved,
    /// [`Runner::add_swarm`](crate::Runner::add_swarm).
    SwarmAdded,
    /// [`Runner::remove_swarm`](crate::Runner::remove_swarm).
    SwarmRemoved,
    /// A channel started: the peer's frames carry `channel`, ours
    /// `peer_channel`.
    ChannelOpened {
        channel: u64,
        peer_channel: u64,
    },
    ChannelClosed,

    /// Data written to the peer: a push to a subscriber, or an answer to
    /// its request. `queued` is when a hybrid push was queued, or the
    /// request's time for an answer ([`Time::EPOCH`] when unknown).
    DataSent {
        bin: Bin,
        push: bool,
        queued: Time,
    },
    /// The peer's data queue was full, so the bin wasn't queued.
    DataRefused {
        bin: Bin,
        push: bool,
    },
    /// A queued bin was dropped at write time: its integrity or data
    /// couldn't be written, or one chunk doesn't fit an empty frame.
    DataDropped {
        bin: Bin,
        push: bool,
    },
    /// A push dropped at write time for waiting too long since `queued`.
    PushStale {
        bin: Bin,
        queued: Time,
    },
    /// A push dropped at write time: the child announced the chunk.
    PushSuppressed {
        bin: Bin,
    },
    /// A push dropped at write time: the child cancelled the chunk.
    PushCancelled {
        bin: Bin,
    },

    /// Data from the peer behind our window.
    DataStale {
        bin: Bin,
    },
    /// A chunk the parent pushed, first to arrive. `lag` is since we first
    /// learned of the chunk.
    PushHit {
        bin: Bin,
        lag: Duration,
    },
    /// A chunk the parent pushed after its deadline released it to pull.
    PushLate {
        bin: Bin,
        lag: Duration,
    },
    /// A push of a stream the peer isn't our parent for.
    PushUnsolicited {
        bin: Bin,
    },
    /// A chunk expected from the stream's parent (the event's peer, if it
    /// still is one) missed its `deadline` and went to pull. `first_seen`
    /// is when we first learned of it; the deadline was the parent's push
    /// delay estimate then, on top. `counted` is whether the miss counted
    /// against the open parent (the chunk is past its start); `lag` is that
    /// parent's estimate now.
    PushMiss {
        bin: Bin,
        deadline: Time,
        first_seen: Time,
        counted: bool,
        lag: LagEstimate,
    },
    /// A request to the peer went unanswered past its timeout: it was sent
    /// at `requested`, with `timeout` to answer.
    RequestTimeout {
        bin: Bin,
        requested: Time,
        timeout: Duration,
    },
    /// A write cycle requested chunks of the peer ([`Request`](crate::codec::Request)).
    RequestRound(RequestRound),

    /// We cancelled the parent of `stream` for its misses.
    Pruned {
        stream: Stream,
        why: Prune,
    },
    /// The peer opened `stream` and became its parent, replacing `from`.
    Switched {
        stream: Stream,
        from: PeerKey,
    },
    /// The peer didn't open `stream` in time: we asked `waited` ago.
    OpenTimeout {
        stream: Stream,
        waited: Duration,
    },
    /// A parent check of `stream` weighed the candidates: the event's
    /// peer is the one it chose, if any.
    ParentChoice {
        stream: Stream,
        choice: ParentChoice,
    },
    /// The peer declined our request for `stream`.
    Declined {
        stream: Stream,
    },
    /// The parent of `stream` closed it.
    ShedBy {
        stream: Stream,
    },

    /// We accepted the child's request for `stream`.
    Admitted {
        stream: Stream,
        why: Admission,
    },
    /// We refused the child's request for `stream`.
    Rejected {
        stream: Stream,
        why: Admission,
    },
    /// We closed the child's subscription to `stream`: our uplink is
    /// saturated, or a seed's budget no longer covers it.
    Shed {
        stream: Stream,
        why: Shed,
    },

    /// The seed gave the peer a copy to hold.
    HolderJoined,
    /// The seed took the peer's copy back. `copies` is the seed's copies
    /// then, and `rtt` the peer's smallest recent ping round trip.
    HolderLeft {
        why: HolderLeft,
        copies: u64,
        rtt: Option<Duration>,
    },
    /// The seed barred the peer from holding a copy: it didn't announce
    /// what it was pushed.
    HolderSilent,
    /// The seed barred the peer from holding a copy: most of the
    /// `reporters` that named it as their parent saw it push nothing.
    HolderUnserved {
        reporters: u64,
    },
    /// The seed's budget changed: subscriptions its measured upload pays
    /// for, copies and streams per holder; and the capacity, a
    /// subscriber's cost and the node's other swarms' push rate it was
    /// computed from.
    SeedBudget {
        budget: u64,
        copies: u64,
        streams: u64,
        capacity: Option<u64>,
        stream_rate: u64,
        others: u64,
    },
    /// An adaptive seed sized its first wave: the `copies` that the
    /// capacity, less the node's other swarms' push rate, pays for at the
    /// store rate since the first chunk (`early_rate`), at most one per
    /// neighbour.
    SeedFirstWave {
        capacity: u64,
        early_rate: u64,
        others: u64,
        neighbours: u64,
        copies: u64,
    },
    /// The runner's upload measurement as this swarm took it at a tick,
    /// when it changed.
    Upload(Upload),
    /// An adaptive seed's check: its window in copies, and of the holders
    /// pushed to since the last check, those behind.
    SeedWindow {
        copies: u64,
        pushed: u64,
        behind: u64,
    },

    /// Events dropped because a buffer filled between drains.
    Lost {
        count: u64,
    },
}

/// A push parent's delay estimate: mean, mean deviation and decaying
/// peak of its pushes' delay after a chunk was first seen, and the
/// deadline they give.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LagEstimate {
    pub mean: Duration,
    pub dev: Duration,
    pub peak: Duration,
    pub deadline: Duration,
}

/// The rule that decided a child's request for a stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AdmitRule {
    /// The child already takes the stream from us.
    #[default]
    Subscribed,
    /// A pin names the peers ([`Runner::pin_seed_allocation`](crate::Runner::pin_seed_allocation)).
    Pin,
    /// An adaptive seed: the peer holds a copy, with room for the stream.
    SeedHolder,
    /// An adaptive seed: the peer is barred from holding a copy.
    SeedBarred,
    /// An adaptive seed: every copy has a holder.
    SeedNoCopy,
    /// An adaptive seed: the peer holds its share of streams already.
    SeedStreamsHeld,
    /// No capacity measured yet: at most
    /// [`UNMEASURED_FANOUT`](crate::scheduler_peer::push::UNMEASURED_FANOUT) children of the
    /// interior stream.
    Unmeasured,
    /// The upload rate or the committed push rate, whichever is higher,
    /// plus a subscriber, against the capacity at the target.
    Measured,
}

/// What a decision on a child's request compared.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Admission {
    pub rule: AdmitRule,
    /// The stream isn't the interior one: the extra target applies.
    pub extra: bool,
    /// The upload meter's rate; the push rate committed (our children at
    /// a subscriber's cost, plus the node's other swarms'); the other
    /// swarms' share of it; a subscriber's cost; the capacity estimate;
    /// and the utilization target in thousandths.
    pub upload_rate: u64,
    pub committed: u64,
    pub others: u64,
    pub stream_rate: u64,
    pub capacity: Option<u64>,
    pub target_permille: u64,
    /// The stream's subscribers before this one.
    pub stream_children: u64,
    /// An adaptive seed's holders and copies, the streams the peer holds,
    /// and the streams a holder may hold.
    pub holders: u64,
    pub copies: u64,
    pub held: u64,
    pub per_holder: u64,
}

/// Why a subscriber was closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shed {
    /// The uplink was busy at least
    /// [`SHED_UTILIZATION`](crate::scheduler_peer::push::SHED_UTILIZATION) of the time (`busy`
    /// in thousandths), and the committed push rate (`committed`, our
    /// children's, plus `others`, the node's other swarms') was over the
    /// interior target of `capacity`: `excess` is this swarm's share of
    /// the difference, and `count` the subscribers it closed for it.
    Saturated {
        busy: u64,
        committed: u64,
        others: u64,
        capacity: u64,
        excess: u64,
        count: u64,
    },
    /// An adaptive seed's budget pays for fewer than one copy: each holder
    /// keeps `room` streams.
    SeedBudget { room: u64 },
}

/// The rule that pruned a parent. The numbers are the subscription's when
/// it was pruned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prune {
    /// [`PRUNE_SILENT_MISSES`](crate::scheduler_peer::push::PRUNE_SILENT_MISSES) misses since
    /// the last hit, and [`PRUNE_SILENCE`](crate::scheduler_peer::push::PRUNE_SILENCE) since it.
    pub silent: bool,
    /// A miss rate over [`PRUNE_MISS_RATE`](crate::scheduler_peer::push::PRUNE_MISS_RATE).
    pub lossy: bool,
    pub silent_misses: u64,
    /// Since the last hit (or the open); `None` before the parent opened.
    pub since_hit: Option<Duration>,
    /// The miss rate's moving average, in millionths.
    pub miss_rate_ppm: u64,
    /// Chunks expected from the parent, and its pushes measured.
    pub expected: u64,
    pub pushes: u64,
    /// The parent's age; `None` before it opened.
    pub age: Option<Duration>,
    pub lag: LagEstimate,
}

/// Why an adaptive seed took a holder's copy back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HolderLeft {
    /// It didn't announce what it was pushed.
    Silent,
    /// Most reporters saw it push nothing.
    Unserved,
    /// The copies fell under the holders, and it was the farthest.
    PastCopies,
    /// It held a copy without a subscription past its grace.
    Idle,
    /// Its channel closed.
    Closed,
}

/// What a parent check did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParentAction {
    /// With no parent: asked the best candidate.
    Graft,
    /// With no parent: no candidate.
    None,
    /// A candidate beats the parent, one vote short of a switch.
    Vote,
    /// A candidate beat the parent at enough checks in a row: asked it.
    Switch,
    /// No candidate beats the parent any more: the votes start over.
    Reset,
}

/// A parent check's weighing of the candidates for a stream: those that
/// announced it within
/// [`CANDIDATE_FRESHNESS`](crate::scheduler_peer::push::CANDIDATE_FRESHNESS), by their mean
/// announcement lag, the lowest first; and how many neighbours it passed
/// over and why. The parent itself isn't a candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParentChoice {
    pub action: ParentAction,
    pub best: Option<(PeerKey, Duration)>,
    pub runner_up: Option<(PeerKey, Duration)>,
    /// No handshake yet, or it chokes us.
    pub unready: u64,
    /// It takes the stream from us.
    pub subscriber: u64,
    /// Backed off after a prune, a decline or a timeout.
    pub backed_off: u64,
    /// It didn't announce the stream recently.
    pub stale: u64,
    /// Checks in a row at which a candidate beat the parent.
    pub votes: u64,
    /// The parent's mean announcement lag and push delay, when measured.
    pub parent_have: Option<Duration>,
    pub parent_push: Option<Duration>,
}

/// A write cycle's requests to a peer: from the `floor` bin on, at most
/// what the request window has room for (`cap` less `in_flight`, bytes),
/// `room` under the outstanding requests' bound, and `fit` in the frame.
/// `candidates` is how many chunks it found to choose from, `requested`
/// how many it asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestRound {
    pub floor: Bin,
    pub cap: u64,
    pub in_flight: u64,
    pub room: u64,
    pub fit: u64,
    pub candidates: u64,
    pub requested: u64,
}

/// The upload measurement as a swarm takes it: the rate written, the busy
/// fraction in thousandths, the capacity estimate, the driver's hint, the
/// node's other swarms' push rate and this swarm's, and the queueing delay
/// past the runner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Upload {
    pub rate: u64,
    pub busy: u64,
    pub capacity: Option<u64>,
    pub hint: Option<u64>,
    pub others: u64,
    pub committed: u64,
    pub queue_delay: Option<Duration>,
}

/// A buffer of events.
#[derive(Debug, Default)]
pub(crate) struct Tap {
    on: bool,
    /// Bound: [`TAP_CAP`].
    events: Vec<TapEvent>,
    lost: u64,
}

impl Tap {
    /// Whether events are recorded.
    #[inline]
    pub(crate) fn is_on(&self) -> bool {
        self.on
    }

    pub(crate) fn set(&mut self, on: bool) {
        if !on {
            *self = Tap::default();
        }
        self.on = on;
    }

    /// Records the event `f` makes, if on.
    #[inline]
    pub(crate) fn emit(
        &mut self,
        swarm: SwarmKey,
        peer: Option<PeerKey>,
        f: impl FnOnce() -> TapKind,
    ) {
        if self.on {
            self.push(TapEvent {
                swarm: Some(swarm),
                peer,
                kind: f(),
            });
        }
    }

    #[inline]
    pub(crate) fn push(&mut self, e: TapEvent) {
        if !self.on {
            return;
        }
        if self.events.len() < TAP_CAP {
            self.events.push(e);
        } else {
            self.lost = self.lost.saturating_add(1);
        }
    }

    /// Moves `other`'s events here, before it goes.
    pub(crate) fn absorb(&mut self, other: &mut Tap) {
        for e in other.events.drain(..) {
            self.push(e);
        }
        self.lost = self.lost.saturating_add(std::mem::take(&mut other.lost));
    }

    /// Hands every event to `f`, and returns the count lost.
    pub(crate) fn drain(&mut self, f: &mut impl FnMut(TapEvent)) -> u64 {
        for e in self.events.drain(..) {
            f(e);
        }
        std::mem::take(&mut self.lost)
    }
}
