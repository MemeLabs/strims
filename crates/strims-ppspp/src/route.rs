// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Requests routed by expected delivery
//! ([`PushMode::Hybrid`](crate::PushMode::Hybrid)).
//!
//! The first peer to announce a chunk is only a proxy for the earliest
//! delivery. A peer that announces early and answers late (its uplink is
//! busy, or we earned no reciprocity there) is worse than one that
//! announces a little later and answers quickly; the seed, which announces
//! everything first, is the usual example. Each channel measures how long
//! its peer takes to answer our requests ([`AnswerTime`]): a request's
//! send to its answer's arrival, queueing at the sender included, which a
//! ping round trip leaves out.
//!
//! A chunk goes to the channel expected to deliver it first, including
//! channels whose peers haven't announced it yet. A channel expects to
//! deliver a chunk its peer announced after its mean answer time (an
//! unmeasured channel counts its request timeout). Another channel's peer
//! is expected to announce it its mean announcement lag for the stream
//! after the chunk was first seen, and to deliver it its own answer time
//! later. A channel leaves the chunk to one expected [`ROUTE_MARGIN`]
//! sooner that had window room at its last request round, within
//! [`ROOM_FRESH`], while that peer's announcement still fits its lag: until
//! its horizon (the lag's decaying peak, or the mean plus two deviations,
//! at most [`DEADLINE_MAX`]) is [`PREDICT_SLACK`] past.
//!
//! A chunk a push deadline released is routed the same way: relays as late
//! as the tree still announce it within their horizons. So a seed whose
//! answers queue behind its pushes isn't asked for a chunk the tree will
//! bring sooner.
//!
//! Routing ranks peers on what this node measures: when their
//! announcements arrive, and how long they took to answer its requests.
//! A peer expected to announce a chunk keeps it from the other channels
//! for at most [`DEADLINE_MAX`] plus [`PREDICT_SLACK`] after the chunk was
//! first seen, as a parent may; one that answers late routes requests away
//! from itself.
//!
//! [`DEADLINE_MAX`]: crate::scheduler_peer::push::DEADLINE_MAX

use strims_base::time::{Duration, Time};

/// A channel leaves a chunk to another expected to deliver it this much
/// sooner, and a late push goes to pull only if a holder is expected to
/// answer this much before the parent's last chance.
pub const ROUTE_MARGIN: Duration = Duration::from_millis(20);
/// A channel had window room if its last request round, this recent, left
/// some.
pub const ROOM_FRESH: Duration = Duration::from_millis(100);
/// A peer's announcement this far past its horizon is no longer expected.
pub const PREDICT_SLACK: Duration = Duration::from_millis(100);

/// What routing predicts a channel's deliveries from: its mean answer
/// time, its fresh announcement lag per stream (the mean, and how late it
/// still fits), and its last request round that left window room.
#[derive(Clone, Debug, Default)]
pub(crate) struct Route {
    pub(crate) answer: Duration,
    /// Bound: one per stream.
    pub(crate) lags: Vec<Option<(Duration, Duration)>>,
    pub(crate) room: Option<Time>,
}

/// How long a peer takes to answer our requests: an exponentially weighted
/// mean (weight 1/8) of request-to-answer times. Only basic float
/// operations, so it is deterministic on every target.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AnswerTime {
    /// Nanoseconds.
    mean: f64,
    n: u32,
}

impl AnswerTime {
    pub(crate) fn sample(&mut self, d: Duration) {
        let x = d.as_nanos().max(0) as f64;
        self.mean = if self.n == 0 {
            x
        } else {
            self.mean + (x - self.mean) / 8.0
        };
        self.n = self.n.saturating_add(1);
    }

    /// The mean answer time, if measured.
    pub(crate) fn mean(&self) -> Option<Duration> {
        (self.n > 0).then(|| Duration::from_nanos(self.mean as i64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_answer_time_follows_its_samples() {
        let mut a = AnswerTime::default();
        assert_eq!(a.mean(), None);
        a.sample(Duration::from_millis(80));
        assert_eq!(a.mean(), Some(Duration::from_millis(80)));
        a.sample(Duration::from_millis(160));
        assert_eq!(a.mean(), Some(Duration::from_millis(90)));
    }
}
