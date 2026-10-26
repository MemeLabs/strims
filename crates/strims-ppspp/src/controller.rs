// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The per-channel request window: how many bytes a channel may have
//! requested from its peer and not yet received, and when an unanswered
//! request counts as lost.
//!
//! The peer scheduler drives a [`RequestController`] with what it sees of
//! the channel (requests sent, data delivered, answers with their round
//! trips, timeouts, pings) and reads back an in-flight cap, a request
//! timeout and the expected answer delay. Each swarm picks the model with
//! [`SwarmOptions::request_controller`]; the choice is local to the node
//! and invisible on the wire:
//!
//! - [`RequestControllerKind::Ledbat`]: [`Ledbat`] (RFC 6817) with a
//!   window floor that carries the stream ([`STREAM_WINDOW_FLOOR`]);
//! - [`RequestControllerKind::BbrLike`]: [`BbrLike`], a model of the
//!   peer's delivery rate and minimum round trip, measured on the local
//!   clock only.
//!
//! [`SwarmOptions::request_controller`]: crate::SwarmOptions::request_controller

use std::str::FromStr;

use strims_base::time::{Duration, Time};

pub use crate::bbr::BbrLike;
use crate::ledbat::Ledbat;

/// The request window's floor under LEDBAT, in bandwidth-delay products of
/// the stream: whatever LEDBAT reads into the path's delay, a channel can
/// carry twice the stream rate.
pub const STREAM_WINDOW_FLOOR: i64 = 2;
/// The longest round trip the LEDBAT window floor covers.
pub const STREAM_WINDOW_FLOOR_RTT: Duration = Duration::SECOND;

/// Which [`RequestController`] a swarm's channels run
/// (`SwarmOptions::request_controller`). Local to this node: peers can't
/// tell which one runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RequestControllerKind {
    /// The crate's default, resolved by `SwarmOptions::default_options`
    /// (zero means default, as for every other option).
    #[default]
    Default,
    /// [`Ledbat`] with the stream-rate window floor.
    Ledbat,
    /// A BBR-style delivery-rate and min-RTT model ([`BbrLike`]).
    BbrLike,
}

impl RequestControllerKind {
    /// The kind `Default` resolves to.
    pub const DEFAULT: RequestControllerKind = RequestControllerKind::Ledbat;

    /// `Default` resolved.
    pub fn resolve(self) -> RequestControllerKind {
        match self {
            RequestControllerKind::Default => RequestControllerKind::DEFAULT,
            k => k,
        }
    }
}

/// Where the request controllers take their round trips
/// ([`RequestController::on_ping_rtt`]): a trial's option, set with
/// [`Runner::set_rtt_source`](crate::Runner::set_rtt_source). Channels ping
/// once a second only on [`RttSource::Ping`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RttSource {
    /// The channels' pings.
    #[default]
    Ping,
    /// The driver's measure of each peer's link
    /// ([`Runner::on_path_sample`](crate::Runner::on_path_sample): vnic's
    /// probes, one link for every swarm the peer shares).
    Link,
}

/// `ping` (or nothing) or `link`.
impl FromStr for RttSource {
    type Err = String;

    fn from_str(s: &str) -> Result<RttSource, String> {
        match s.trim() {
            "" | "ping" => Ok(RttSource::Ping),
            "link" => Ok(RttSource::Link),
            _ => Err(format!("unknown round trip source {s:?}")),
        }
    }
}

/// What the peer scheduler asks of a channel's request window. Every time
/// is the local clock, passed in; the only peer-supplied value is
/// `peer_time` in [`RequestController::on_answered`], which a controller
/// may ignore (LEDBAT uses it; [`BbrLike`] doesn't, so clock skew can't
/// affect it).
pub trait RequestController {
    /// Before a request round. `stream_rate` is the swarm's rate as its
    /// peers' HAVEs advance, in bytes per second (0 while unknown).
    fn on_round(&mut self, now: Time, stream_rate: u64);
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
    /// A round trip to the peer: a ping's, or, by the swarm's
    /// [`RttSource`], the link's.
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
    /// Its estimate of the peer's delivery rate in bytes per second, or 0
    /// (diagnostics).
    fn delivery_rate(&self) -> u64;
}

impl RequestController for Ledbat {
    /// Sets the window floor: [`STREAM_WINDOW_FLOOR`] times the
    /// stream's bandwidth-delay product over the ping round trip
    /// ([`Ledbat::floor_rtt`], at most [`STREAM_WINDOW_FLOOR_RTT`]): in
    /// [`PushMode::Hybrid`](crate::PushMode::Hybrid) the smallest of the
    /// last [`MIN_RTT_WINDOW`](crate::ledbat::MIN_RTT_WINDOW), otherwise
    /// the smoothed one.
    ///
    /// LEDBAT's minimum of 2 chunks carries 2 chunks per round trip, a
    /// fifth of a 32 KiB/s stream at 300 ms. A rise in the path's base
    /// delay reads as queueing until the old base ages out of LEDBAT's
    /// history (4 minutes here, RFC 6817's `BASE_HISTORY` is 10), and the
    /// window sits at that minimum all the while. At the floor one channel
    /// still carries the stream and can catch up. The floor overrides only
    /// LEDBAT's delay signal; after a timed out request LEDBAT governs
    /// alone for a while ([`crate::ledbat::FLOOR_LOSS_HOLD`]).
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
    fn on_round(&mut self, now: Time, stream_rate: u64) {
        let rtt = self
            .floor_rtt()
            .min(STREAM_WINDOW_FLOOR_RTT)
            .as_nanos()
            .max(0) as i128;
        let bytes = i128::from(stream_rate) * rtt * i128::from(STREAM_WINDOW_FLOOR)
            / i128::from(Duration::SECOND.as_nanos());
        self.set_min_cwnd(i64::try_from(bytes).unwrap_or(i64::MAX), now);
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

    fn delivery_rate(&self) -> u64 {
        0
    }
}

/// A channel's controller: one of the [`RequestControllerKind`]s, behind
/// [`RequestController`].
#[derive(Clone, Debug)]
// The LEDBAT variant is a few hundred bytes, small next to the channel
// state around it.
#[allow(clippy::large_enum_variant)]
pub enum ChannelController {
    Ledbat(Ledbat),
    /// Boxed: its rings make it several times LEDBAT's size, and a channel
    /// allocates it once, when it starts.
    BbrLike(Box<BbrLike>),
}

impl ChannelController {
    /// A fresh controller of `kind` (`Default` resolved) for a channel of
    /// a swarm with `chunk_size` byte chunks. `phase_seed` spreads the
    /// channels' probing phases ([`BbrLike`]).
    pub fn new(
        kind: RequestControllerKind,
        chunk_size: usize,
        phase_seed: u64,
        now: Time,
    ) -> ChannelController {
        match kind.resolve() {
            RequestControllerKind::BbrLike => {
                ChannelController::BbrLike(Box::new(BbrLike::new(chunk_size, phase_seed, now)))
            }
            _ => ChannelController::Ledbat(Ledbat::new()),
        }
    }

    /// The kind it runs.
    pub fn kind(&self) -> RequestControllerKind {
        match self {
            ChannelController::Ledbat(_) => RequestControllerKind::Ledbat,
            ChannelController::BbrLike(_) => RequestControllerKind::BbrLike,
        }
    }

    /// The LEDBAT controller, if it runs one (tests, diagnostics).
    pub fn ledbat(&self) -> Option<&Ledbat> {
        match self {
            ChannelController::Ledbat(l) => Some(l),
            ChannelController::BbrLike(_) => None,
        }
    }

    /// The LEDBAT controller, if it runs one (the scheduler's setup,
    /// tests).
    pub fn ledbat_mut(&mut self) -> Option<&mut Ledbat> {
        match self {
            ChannelController::Ledbat(l) => Some(l),
            ChannelController::BbrLike(_) => None,
        }
    }

    /// The BBR-style controller, if it runs one (tests, diagnostics).
    pub fn bbr(&self) -> Option<&BbrLike> {
        match self {
            ChannelController::BbrLike(b) => Some(b.as_ref()),
            ChannelController::Ledbat(_) => None,
        }
    }

    fn get(&self) -> &dyn RequestController {
        match self {
            ChannelController::Ledbat(l) => l,
            ChannelController::BbrLike(b) => b.as_ref(),
        }
    }

    fn get_mut(&mut self) -> &mut dyn RequestController {
        match self {
            ChannelController::Ledbat(l) => l,
            ChannelController::BbrLike(b) => b.as_mut(),
        }
    }
}

impl RequestController for ChannelController {
    fn on_round(&mut self, now: Time, stream_rate: u64) {
        self.get_mut().on_round(now, stream_rate);
    }
    fn on_sent(&mut self, now: Time, bytes: i64) {
        self.get_mut().on_sent(now, bytes);
    }
    fn on_delivered(&mut self, now: Time, bytes: i64) {
        self.get_mut().on_delivered(now, bytes);
    }
    fn on_answered(&mut self, now: Time, bytes: i64, sent: Time, peer_time: Time) {
        self.get_mut().on_answered(now, bytes, sent, peer_time);
    }
    fn on_timed_out(&mut self, now: Time, bytes: i64) {
        self.get_mut().on_timed_out(now, bytes);
    }
    fn on_settled(&mut self, now: Time, bytes: i64) {
        self.get_mut().on_settled(now, bytes);
    }
    fn on_ping_rtt(&mut self, now: Time, rtt: Duration) {
        self.get_mut().on_ping_rtt(now, rtt);
    }
    fn on_frame_end(&mut self, now: Time) {
        self.get_mut().on_frame_end(now);
    }
    fn in_flight_cap(&self) -> i64 {
        self.get().in_flight_cap()
    }
    fn in_flight(&self) -> i64 {
        self.get().in_flight()
    }
    fn request_timeout(&self) -> Duration {
        self.get().request_timeout()
    }
    fn answer_delay(&self) -> Duration {
        self.get().answer_delay()
    }
    fn rtt(&self) -> Duration {
        self.get().rtt()
    }
    fn min_rtt(&self) -> Duration {
        self.get().min_rtt()
    }
    fn delivery_rate(&self) -> u64 {
        self.get().delivery_rate()
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
    fn rtt_source_parses() {
        assert_eq!("".parse::<RttSource>(), Ok(RttSource::Ping));
        assert_eq!("ping".parse::<RttSource>(), Ok(RttSource::Ping));
        assert_eq!(" link".parse::<RttSource>(), Ok(RttSource::Link));
        assert!("meter".parse::<RttSource>().is_err());
    }

    #[test]
    fn default_resolves_to_the_crate_default() {
        assert_eq!(
            RequestControllerKind::Default.resolve(),
            RequestControllerKind::DEFAULT
        );
        assert_eq!(
            RequestControllerKind::BbrLike.resolve(),
            RequestControllerKind::BbrLike
        );
        let c = ChannelController::new(RequestControllerKind::Default, 1024, 0, t(0));
        assert_eq!(c.kind(), RequestControllerKind::DEFAULT);
        let c = ChannelController::new(RequestControllerKind::Ledbat, 1024, 0, t(0));
        assert!(c.ledbat().is_some() && c.bbr().is_none());
        let c = ChannelController::new(RequestControllerKind::BbrLike, 1024, 0, t(0));
        assert!(c.bbr().is_some() && c.ledbat().is_none());
    }

    #[test]
    fn ledbat_floor_follows_the_stream() {
        // The floor is twice the stream's bytes per ping round trip.
        let mut c = ChannelController::new(RequestControllerKind::Ledbat, 1024, 0, t(0));
        c.on_ping_rtt(t(0), Duration::from_millis(300));
        c.on_round(t(0), 32 * 1024);
        let floor = 2 * 32 * 1024 * 3 / 10;
        assert_eq!(c.ledbat().map(Ledbat::min_cwnd), Some(floor));
        assert!(c.in_flight_cap() >= floor);
        // An unknown rate leaves LEDBAT's own minimum.
        c.on_round(t(1), 0);
        assert_eq!(c.ledbat().map(Ledbat::min_cwnd), Some(2 * MSS));
    }

    #[test]
    fn ledbat_floor_follows_the_minimum_round_trip() {
        // Pings that wait behind the peer's full uplink inflate the mean;
        // the floor follows the quickest of the window.
        let mut c = ChannelController::new(RequestControllerKind::Ledbat, 1024, 0, t(0));
        if let Some(l) = c.ledbat_mut() {
            l.set_floor_on_min(true);
        }
        let step = MIN_RTT_WINDOW / 10;
        c.on_ping_rtt(t(0), Duration::from_millis(50));
        for i in 1..10 {
            c.on_ping_rtt(t(0) + step * i, Duration::from_millis(700));
        }
        assert!(c.rtt() > Duration::from_millis(400), "{:?}", c.rtt());
        assert_eq!(c.min_rtt(), Duration::from_millis(50));
        c.on_round(t(0) + step * 9, 32 * 1024);
        let floor = 2 * 32 * 1024 * 50 / 1000;
        assert_eq!(c.ledbat().map(Ledbat::min_cwnd), Some(floor));
        // The path's delay rose: the floor follows within the window.
        let later = t(0) + MIN_RTT_WINDOW;
        c.on_ping_rtt(later, Duration::from_millis(700));
        c.on_round(later, 32 * 1024);
        let floor = 2 * 32 * 1024 * 700 / 1000;
        assert_eq!(c.ledbat().map(Ledbat::min_cwnd), Some(floor));
    }

    #[test]
    fn ledbat_floor_follows_the_mean_by_default() {
        let mut c = ChannelController::new(RequestControllerKind::Ledbat, 1024, 0, t(0));
        c.on_ping_rtt(t(0), Duration::from_millis(50));
        c.on_ping_rtt(t(1_000), Duration::from_millis(700));
        let rtt = c.rtt();
        assert!(rtt > Duration::from_millis(50), "{rtt:?}");
        c.on_round(t(1_000), 32 * 1024);
        let floor = 2 * 32 * 1024 * rtt.as_nanos() / Duration::SECOND.as_nanos();
        assert_eq!(c.ledbat().map(Ledbat::min_cwnd), Some(floor));
    }

    #[test]
    fn ledbat_flight_accounting_through_the_trait() {
        let mut c = ChannelController::new(RequestControllerKind::Ledbat, 1024, 0, t(0));
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
