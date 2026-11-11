// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The scheduler's measurement logs, for the drivers that turn them on
//! (the sims); none of them changes a decision.
//!
//! - The held log ([`Held`], [`crate::Runner::take_held`]): every chunk
//!   the swarm stored, when, and its timeline here: from whom and how it
//!   came, the stream's open parent then, when push first expected it,
//!   when it was released to pull, and the requests we sent for it. On
//!   with the forecast log or the request log.
//! - The request log ([`crate::Runner::set_request_log`]): when each HAVE
//!   first told us a neighbour holds a chunk ([`HaveRecord`],
//!   [`crate::Runner::take_haves`]), and every request the request round
//!   makes, with what it knew of the chunk and the rivals it weighed, once
//!   it ended ([`RequestRecord`], [`crate::Runner::take_requests`]).
//!
//! Each is bound per swarm between the driver's takes
//! ([`super::FORECAST_CAP`] records, a request counting with its rivals),
//! the rest counted lost.

use std::collections::{BTreeMap, VecDeque};

use strims_base::time::{Duration, Time};
use strims_binmap::{Bin, Map};

use super::FORECAST_CAP;
use crate::PeerKey;

/// How a chunk came ([`Held`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeldHow {
    /// Our own publishing, or no data message named it.
    #[default]
    Own,
    /// Pushed by the stream's open parent.
    Push,
    /// An answer to a request of ours.
    Answer,
    /// Any other peer's data.
    Other,
}

impl HeldHow {
    pub fn name(self) -> &'static str {
        match self {
            HeldHow::Own => "own",
            HeldHow::Push => "push",
            HeldHow::Answer => "answer",
            HeldHow::Other => "other",
        }
    }
}

/// A chunk the scheduler stored, with the forecast or the request log on
/// ([`crate::Runner::take_held`]): when it first held it, the truth the
/// logs are judged by, and its timeline here: from whom and how it came,
/// the stream's open parent then, when push first expected it, when it
/// was released to pull, and the requests we sent for it. It changes no
/// decision.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held {
    pub bin: Bin,
    pub at: Time,
    pub peer: Option<PeerKey>,
    pub how: HeldHow,
    pub parent: Option<PeerKey>,
    pub expected: Option<Time>,
    pub released: Option<Time>,
    /// Our first request for it, the requests, and the last one with its
    /// peer.
    pub asked: Option<Time>,
    pub asks: u32,
    pub last_ask: Option<(PeerKey, Time)>,
    /// The source channel's smoothed round trip when it came.
    pub rtt: Duration,
}

/// What a scheduler with the held log on noted of a chunk before it came
/// ([`Held`]). Bound: the window, kept per chunk not yet stored.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct HeldMarks {
    pub(crate) expected: Option<Time>,
    pub(crate) released: Option<Time>,
    pub(crate) asked: Option<Time>,
    pub(crate) asks: u32,
    pub(crate) last_ask: Option<(PeerKey, Time)>,
}

impl HeldMarks {
    pub(crate) fn ask(&mut self, peer: PeerKey, now: Time) {
        self.asked.get_or_insert(now);
        self.asks = self.asks.saturating_add(1);
        self.last_ask = Some((peer, now));
    }
}

/// A chunk a HAVE first announced on a channel, with the request log on
/// ([`crate::Runner::take_haves`]): when the HAVE arrived, from whom, and
/// when the channel opened (a chunk the peer held before then comes in the
/// channel's first HAVEs, a backlog rather than an arrival). Once per
/// chunk and channel: a later HAVE covering it again, or a chunk past the
/// tracked history, adds none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HaveRecord {
    pub at: Time,
    pub peer: PeerKey,
    pub stream: u16,
    pub bin: Bin,
    pub channel_open: Time,
}

/// How a request ended ([`RequestRecord`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestEnd {
    /// The peer's data answered it.
    Answered,
    /// Its timeout came with the chunk still lacking.
    Expired,
    /// The chunk came from another peer first, and the request settled
    /// unanswered (at its timeout, or as the store's read position passed
    /// it).
    Elsewhere,
    /// Its channel closed or restarted, or the scheduler went, first.
    Closed,
}

impl RequestEnd {
    /// Its name for a table: `answered`, `expired`, `elsewhere` or
    /// `closed`.
    pub fn name(self) -> &'static str {
        match self {
            RequestEnd::Answered => "answered",
            RequestEnd::Expired => "expired",
            RequestEnd::Elsewhere => "elsewhere",
            RequestEnd::Closed => "closed",
        }
    }

    /// An expiry's `timeout`; empty otherwise.
    pub fn reason(self) -> &'static str {
        match self {
            RequestEnd::Expired => "timeout",
            _ => "",
        }
    }
}

/// A channel as a request decision saw it ([`RequestRecord`]): when a
/// HAVE from its peer first covered the chunk (`announced`; `None` if
/// none had); for a rival, when the request round's route expected it to
/// hold the chunk, by its announcement lag's mean and horizon from the
/// chunk's first sight ([`crate::route`]); and its smoothed round trip
/// and opening.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestChannel {
    pub peer: PeerKey,
    pub announced: Option<Time>,
    pub predicted: Option<(Time, Time)>,
    pub rtt: Duration,
    pub channel_open: Time,
}

/// A request the request round sent ([`crate::Runner::take_requests`]):
/// when, for which chunk, its timeout, when push's deadline released the
/// chunk to pull (`None` if push never expected it), the asked channel,
/// the rivals the round weighed for it (the other channels whose route
/// had window room and an announcement lag for the stream: any of them
/// expected to deliver it sooner would have taken it), and how and when
/// it ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestRecord {
    pub at: Time,
    pub stream: u16,
    pub bin: Bin,
    pub timeout: Duration,
    pub released: Option<Time>,
    pub asked: RequestChannel,
    pub rivals: Vec<RequestChannel>,
    pub end: RequestEnd,
    pub ended: Time,
}

/// The request log's state: the requests out, by peer and chunk in the
/// order they went, and those ended since the driver last took them. A
/// chunk asked of a peer again before the earlier request's timeout came
/// (its request time pruned behind the store's window) has both out, and
/// an end goes to the earlier, as the runner's counters count them. Bound:
/// the requests in flight and those awaiting their timeouts, and
/// [`FORECAST_CAP`] records' rows (each with its rivals) ended, the rest
/// counted in `lost`.
#[derive(Debug, Default)]
pub(crate) struct RequestLog {
    open: BTreeMap<(PeerKey, Bin), VecDeque<RequestRecord>>,
    ended: Vec<RequestRecord>,
    rows: usize,
    lost: u64,
}

impl RequestLog {
    /// A request went.
    pub(crate) fn sent(&mut self, r: RequestRecord) {
        self.open
            .entry((r.asked.peer, r.bin))
            .or_default()
            .push_back(r);
    }

    /// Ends `peer`'s earliest open request for `bin`, if one is.
    pub(crate) fn ended(&mut self, peer: PeerKey, bin: Bin, end: RequestEnd, now: Time) {
        let Some(q) = self.open.get_mut(&(peer, bin)) else {
            return;
        };
        let r = q.pop_front();
        if q.is_empty() {
            self.open.remove(&(peer, bin));
        }
        if let Some(r) = r {
            self.push(r, end, now);
        }
    }

    /// Ends every open request of `peer` for `bin`.
    fn ended_all(&mut self, peer: PeerKey, bin: Bin, end: RequestEnd, now: Time) {
        for r in self.open.remove(&(peer, bin)).into_iter().flatten() {
            self.push(r, end, now);
        }
    }

    /// Ends `peer`'s requests for chunks before `before`, which the store's
    /// read position passed, and that the store holds (`held`), as
    /// [`RequestEnd::Elsewhere`]; one for a chunk the reader passed without
    /// it ends at its timeout, as the runner counts it.
    pub(crate) fn settled(&mut self, peer: PeerKey, before: Bin, held: &Map, now: Time) {
        let bins: Vec<Bin> = self
            .open
            .range((peer, Bin(0))..(peer, before))
            .map(|(&(_, b), _)| b)
            .filter(|&b| held.filled_at(b))
            .collect();
        for b in bins {
            self.ended_all(peer, b, RequestEnd::Elsewhere, now);
        }
    }

    /// Ends the requests open to `peer` (every peer's with `None`) as
    /// [`RequestEnd::Closed`].
    pub(crate) fn close(&mut self, peer: Option<PeerKey>, now: Time) {
        let keys: Vec<(PeerKey, Bin)> = self
            .open
            .keys()
            .filter(|(p, _)| peer.is_none_or(|q| q == *p))
            .copied()
            .collect();
        for (p, b) in keys {
            self.ended_all(p, b, RequestEnd::Closed, now);
        }
    }

    fn push(&mut self, mut r: RequestRecord, end: RequestEnd, now: Time) {
        let rows = 1 + r.rivals.len();
        if self.rows + rows > FORECAST_CAP {
            self.lost += 1;
            return;
        }
        self.rows += rows;
        r.end = end;
        r.ended = now;
        self.ended.push(r);
    }

    /// The requests ended since the last call, and how many the cap left
    /// out.
    pub(crate) fn take(&mut self) -> (Vec<RequestRecord>, u64) {
        self.rows = 0;
        (
            std::mem::take(&mut self.ended),
            std::mem::take(&mut self.lost),
        )
    }
}
