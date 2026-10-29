// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The per-channel request window: how many bytes a channel may have
//! requested from its peer and not yet received, and when an unanswered
//! request counts as lost.
//!
//! The peer scheduler drives a [`RequestController`] with what it sees of
//! the channel (requests sent, data delivered, answers with their round
//! trips, timeouts, pings) and reads back an in-flight cap, a request
//! timeout and the expected answer delay. Channels run [`Ledbat`]
//! (RFC 6817) with a window floor that carries the stream
//! ([`STREAM_WINDOW_FLOOR`]); the window is local to the node and
//! invisible on the wire.

use strims_base::time::{Duration, Time};

use crate::ledbat::Ledbat;

/// The request window's floor under LEDBAT, in bandwidth-delay products of
/// the stream: whatever LEDBAT reads into the path's delay, a channel can
/// carry twice the stream rate.
pub const STREAM_WINDOW_FLOOR: i64 = 2;
/// The longest round trip the LEDBAT window floor covers.
pub const STREAM_WINDOW_FLOOR_RTT: Duration = Duration::SECOND;

/// What the peer scheduler asks of a channel's request window. Every time
/// is the local clock, passed in; the only peer-supplied value is
/// `peer_time` in [`RequestController::on_answered`], which a controller
/// may ignore.
pub trait RequestController {
    /// Before a request round. `stream_rate` is the swarm's rate as its
    /// peers' HAVEs advance, in bytes per second (0 while unknown), and
    /// `chunk` its chunk size in bytes.
    fn on_round(&mut self, now: Time, stream_rate: u64, chunk: i64);
    /// A request for `bytes` was sent.
    fn on_sent(&mut self, now: Time, bytes: i64);
    /// `bytes` of verified data arrived from the peer, pushed or pulled,
    /// new or not.
    fn on_delivered(&mut self, now: Time, bytes: i64);
    /// `bytes` of data arrived answering a request we sent at `sent`.
    /// `peer_time` is the data's timestamp, on the peer's clock.
    fn on_answered(&mut self, now: Time, bytes: i64, sent: Time, peer_time: Time);
    /// Requests for `bytes` timed out unanswered: a loss.
    fn on_timed_out(&mut self, now: Time, bytes: i64);
    /// Requests for `bytes` that the peer never answered timed out after
    /// their chunks arrived from elsewhere: out of flight, not lost.
    fn on_settled(&mut self, now: Time, bytes: i64);
    /// A ping's round trip.
    fn on_ping_rtt(&mut self, now: Time, rtt: Duration);
    /// The end of a frame from the peer.
    fn on_frame_end(&mut self, now: Time);

    /// The most bytes the channel may have requested and not received.
    fn in_flight_cap(&self) -> i64;
    /// Bytes requested and not yet answered, lost or settled.
    fn in_flight(&self) -> i64;
    /// How long a request may go unanswered before it counts as lost.
    fn request_timeout(&self) -> Duration;
    /// How long an answer to a request sent now is expected to take. The
    /// scheduler starts requests this far (in stream time) ahead of the
    /// start of the peer's window, so the chunks are still there when the
    /// request arrives. At most [`RequestController::request_timeout`].
    fn answer_delay(&self) -> Duration;
    /// The controller's smoothed round trip (diagnostics).
    fn rtt(&self) -> Duration;
    /// The smallest round trip it has seen recently, or zero
    /// (diagnostics).
    fn min_rtt(&self) -> Duration;
}

impl RequestController for Ledbat {
    /// Sets the window floor: [`STREAM_WINDOW_FLOOR`] times the
    /// stream's bandwidth-delay product, and at least one chunk, over the
    /// ping round trip
    /// ([`Ledbat::floor_rtt`], at most [`STREAM_WINDOW_FLOOR_RTT`]): in
    /// [`PushMode::Hybrid`](crate::PushMode::Hybrid) the smallest of the
    /// last [`MIN_RTT_WINDOW`](crate::ledbat::MIN_RTT_WINDOW), otherwise
    /// the smoothed one.
    ///
    /// LEDBAT's minimum of 2 KiB per round trip is a fifth of a 32 KiB/s
    /// stream at 300 ms. A rise in the path's base
    /// delay reads as queueing until the old base ages out of LEDBAT's
    /// history (4 minutes here, RFC 6817's `BASE_HISTORY` is 10), and the
    /// window sits at that minimum all the while. At the floor one channel
    /// still carries the stream and can catch up. The floor overrides only
    /// LEDBAT's delay signal; after a timed out request LEDBAT governs
    /// alone for a while ([`crate::ledbat::FLOOR_LOSS_HOLD`]).
    ///
    /// A window under one chunk asks for nothing: with nothing in flight
    /// no answer comes to grow it, and the chunk the reader waits for goes
    /// unasked until the floor happens to pass a chunk. Twice a 1 Mbit/s
    /// stream over a 10 ms path is 2.5 KiB, and LEDBAT's minimum is 2 KiB,
    /// both under a 4 KiB chunk, so the floor is at least one. The minimum
    /// after a loss stays LEDBAT's: a peer that timed out is left alone
    /// for the hold.
    ///
    /// When the peer's uplink is full, a ping waits behind every byte the
    /// peer has already written to us, in its frames, its socket and the
    /// path, so the mean grows with what we asked for, and a floor on it
    /// lets the channel keep more requested from a congested peer the
    /// more congested it gets. In Hybrid those answers share the peer's
    /// uplink with its pushes, to us and to others, and hold them back
    /// until the children give up on them and pull the streams from the
    /// peer instead: a loop that keeps its uplink full. The floor there
    /// follows the minimum: a ping that finds the pipe briefly drained
    /// reads close to the path's delay, and one in the window is enough;
    /// a real rise in the path's delay reaches the floor once the older
    /// pings leave the window. Pull swarms keep the mean: with no pushes
    /// to hold back, a congested seed's uplink is best kept full of
    /// distinct requests, and a floor on the minimum costs them lag on
    /// long paths.
    fn on_round(&mut self, now: Time, stream_rate: u64, chunk: i64) {
        let rtt = self
            .floor_rtt()
            .min(STREAM_WINDOW_FLOOR_RTT)
            .as_nanos()
            .max(0) as i128;
        let bytes = i128::from(stream_rate) * rtt * i128::from(STREAM_WINDOW_FLOOR)
            / i128::from(Duration::SECOND.as_nanos());
        let bytes = i64::try_from(bytes).unwrap_or(i64::MAX).max(chunk);
        self.set_min_cwnd(bytes, now);
    }

    fn on_sent(&mut self, _now: Time, bytes: i64) {
        self.add_sent(bytes);
    }

    fn on_delivered(&mut self, _now: Time, _bytes: i64) {}

    fn on_answered(&mut self, now: Time, bytes: i64, _sent: Time, peer_time: Time) {
        self.add_delay_sample(now - peer_time, bytes, now);
    }

    fn on_timed_out(&mut self, now: Time, bytes: i64) {
        self.add_data_loss(bytes, false, now);
    }

    fn on_settled(&mut self, _now: Time, bytes: i64) {
        self.forget_sent(bytes);
    }

    fn on_ping_rtt(&mut self, now: Time, rtt: Duration) {
        self.add_rtt_sample(rtt, now);
    }

    fn on_frame_end(&mut self, now: Time) {
        self.digest_delay_samples(now);
    }

    fn in_flight_cap(&self) -> i64 {
        self.cwnd()
    }

    fn in_flight(&self) -> i64 {
        self.flight_size()
    }

    fn request_timeout(&self) -> Duration {
        self.cto()
    }

    fn answer_delay(&self) -> Duration {
        self.cto()
    }

    fn rtt(&self) -> Duration {
        self.rtt_mean()
    }

    fn min_rtt(&self) -> Duration {
        self.rtt_min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledbat::{MIN_RTT_WINDOW, MSS};

    fn t(ms: i64) -> Time {
        Time::from_unix_millis(1_700_000_000_000 + ms)
    }

    #[test]
    fn ledbat_floor_follows_the_stream() {
        // The floor is twice the stream's bytes per ping round trip.
        let mut c = Ledbat::new();
        c.on_ping_rtt(t(0), Duration::from_millis(300));
        c.on_round(t(0), 32 * 1024, 1024);
        let floor = 2 * 32 * 1024 * 3 / 10;
        assert_eq!(c.min_cwnd(), floor);
        assert!(c.in_flight_cap() >= floor);
        // An unknown rate leaves LEDBAT's own minimum ...
        c.on_round(t(1), 0, 1024);
        assert_eq!(c.min_cwnd(), 2 * MSS);
        // ... or a chunk, if that is more.
        c.on_round(t(2), 0, 4096);
        assert_eq!(c.min_cwnd(), 4096);
    }

    #[test]
    fn ledbat_floor_follows_the_minimum_round_trip() {
        // Pings that wait behind the peer's full uplink inflate the mean;
        // the floor follows the quickest of the window.
        let mut c = Ledbat::new();
        c.set_floor_on_min(true);
        let step = MIN_RTT_WINDOW / 10;
        c.on_ping_rtt(t(0), Duration::from_millis(50));
        for i in 1..10 {
            c.on_ping_rtt(t(0) + step * i, Duration::from_millis(700));
        }
        assert!(c.rtt() > Duration::from_millis(400), "{:?}", c.rtt());
        assert_eq!(c.min_rtt(), Duration::from_millis(50));
        c.on_round(t(0) + step * 9, 32 * 1024, 1024);
        let floor = 2 * 32 * 1024 * 50 / 1000;
        assert_eq!(c.min_cwnd(), floor);
        // The path's delay rose: the floor follows within the window.
        let later = t(0) + MIN_RTT_WINDOW;
        c.on_ping_rtt(later, Duration::from_millis(700));
        c.on_round(later, 32 * 1024, 1024);
        let floor = 2 * 32 * 1024 * 700 / 1000;
        assert_eq!(c.min_cwnd(), floor);
    }

    #[test]
    fn ledbat_floor_follows_the_mean_by_default() {
        let mut c = Ledbat::new();
        c.on_ping_rtt(t(0), Duration::from_millis(50));
        c.on_ping_rtt(t(1_000), Duration::from_millis(700));
        let rtt = c.rtt();
        assert!(rtt > Duration::from_millis(50), "{rtt:?}");
        c.on_round(t(1_000), 32 * 1024, 1024);
        let floor = 2 * 32 * 1024 * rtt.as_nanos() / Duration::SECOND.as_nanos();
        assert_eq!(c.min_cwnd(), floor);
    }

    #[test]
    fn ledbat_flight_accounting_through_the_trait() {
        let mut c = Ledbat::new();
        c.on_sent(t(0), 4096);
        assert_eq!(c.in_flight(), 4096);
        c.on_answered(t(20), 1024, t(0), t(10));
        c.on_frame_end(t(20));
        assert_eq!(c.in_flight(), 3072);
        c.on_settled(t(30), 1024);
        assert_eq!(c.in_flight(), 2048);
        c.on_timed_out(t(40), 2048);
        assert_eq!(c.in_flight(), 0);
        assert_eq!(c.request_timeout(), Duration::SECOND);
    }
}
