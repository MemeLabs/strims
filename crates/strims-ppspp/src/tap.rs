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
    /// still is one) missed its deadline and went to pull.
    PushMiss {
        bin: Bin,
    },
    /// A request to the peer went unanswered past its timeout.
    RequestTimeout {
        bin: Bin,
    },

    /// We cancelled the parent of `stream` for its misses.
    Pruned {
        stream: Stream,
    },
    /// The peer opened `stream` and became its parent, replacing `from`.
    Switched {
        stream: Stream,
        from: PeerKey,
    },
    /// The peer didn't open `stream` in time.
    OpenTimeout {
        stream: Stream,
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
    },
    /// We refused the child's request for `stream`.
    Rejected {
        stream: Stream,
    },
    /// We closed the child's subscription to `stream`: our uplink is
    /// saturated, or a seed's budget no longer covers it.
    Shed {
        stream: Stream,
    },

    /// The seed gave the peer a copy to hold.
    HolderJoined,
    /// The seed took the peer's copy back.
    HolderLeft,
    /// The seed's budget changed: subscriptions its measured upload pays
    /// for, copies and streams per holder.
    SeedBudget {
        budget: u64,
        copies: u64,
        streams: u64,
    },

    /// Events dropped because a buffer filled between drains.
    Lost {
        count: u64,
    },
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
