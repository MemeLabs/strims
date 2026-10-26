// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Queueing delay on the node's uplink, from ping round trips.
//!
//! A channel pings once a second for its request controller, and in a
//! hybrid swarm every [`METER_PING`] while it sends data, for the meter
//! alone: with a sample
//! every few hundred milliseconds the least of the recent ones finds the
//! standing queue between bursts.
//!
//! A ping's nonce is our clock when its frame was built, so its round trip
//! includes whatever waits past the runner: the driver's link buffers, the
//! kernel's unsent bytes and congestion window, the access network's queue.
//! A queue on our uplink delays every peer's round trip by about the same
//! amount; one on a peer's path (its downlink, its own upload of the pong)
//! delays only that peer's. So the node's queueing delay is a low quantile,
//! across peers, of how far each peer's recent round trips stand above its
//! base: a slow neighbour, or a minority that delays its pongs, can't move
//! it.
//!
//! Each peer's round trips go into a [`PathDelay`], which keeps the base
//! (the least round trip over ten minutes) and the recent minimum, and
//! [`aggregate`] takes the low quantile. A channel's first ping goes out
//! with its handshake, before any data, so a queue that builds later reads
//! as excess.

use std::str::FromStr;

pub(crate) use strims_base::path::{PathDelay, aggregate};
use strims_base::time::Duration;

/// While a channel sends data it also pings this often, for the meter.
pub(crate) const METER_PING: Duration = Duration::from_millis(200);

/// Where a reader of round trips takes them (a trial's option; see
/// [`PathSources`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PathSource {
    /// The channels' pings.
    #[default]
    Ping,
    /// The driver's measure of each peer's link
    /// ([`Runner::on_path_sample`](crate::Runner::on_path_sample) and
    /// [`Runner::set_peer_path`](crate::Runner::set_peer_path): vnic's
    /// probes, one link for every swarm the peer shares).
    Link,
}

/// Where each reader of round trips takes them
/// ([`Runner::set_path_sources`](crate::Runner::set_path_sources)).
/// Channels ping once a second only while some reader takes pings, and
/// every [`METER_PING`] only while the meter does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PathSources {
    /// The upload meter's queueing delay: each peer's round trips over its
    /// base.
    pub meter: PathSource,
    /// An adaptive seed's holders' round trips, which rank them.
    pub holder: PathSource,
    /// The request controllers' round trips (`on_ping_rtt`): the request
    /// timeout, and BBR's least round trip.
    pub controller: PathSource,
}

impl PathSources {
    /// Every reader on the link.
    pub const LINK: PathSources = PathSources {
        meter: PathSource::Link,
        holder: PathSource::Link,
        controller: PathSource::Link,
    };

    /// Whether channels ping once a second.
    pub(crate) fn pings(self) -> bool {
        [self.meter, self.holder, self.controller].contains(&PathSource::Ping)
    }
}

/// `ping` (or nothing): every reader on pings; `link`: every reader on the
/// link; or the readers on the link joined by `+`, of `meter`, `holder`
/// and `controller`.
impl FromStr for PathSources {
    type Err = String;

    fn from_str(s: &str) -> Result<PathSources, String> {
        let mut p = PathSources::default();
        match s.trim() {
            "" | "ping" => {}
            "link" => p = PathSources::LINK,
            list => {
                for name in list.split('+').map(str::trim) {
                    *match name {
                        "meter" => &mut p.meter,
                        "holder" => &mut p.holder,
                        "controller" => &mut p.controller,
                        _ => return Err(format!("unknown path reader {name:?} in {s:?}")),
                    } = PathSource::Link;
                }
            }
        }
        Ok(p)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn path_sources_parse() {
        use PathSource::{Link, Ping};
        assert_eq!("".parse::<PathSources>().unwrap(), PathSources::default());
        assert_eq!(
            "ping".parse::<PathSources>().unwrap(),
            PathSources::default()
        );
        assert_eq!("link".parse::<PathSources>().unwrap(), PathSources::LINK);
        let p: PathSources = "meter+controller".parse().unwrap();
        assert_eq!((p.meter, p.holder, p.controller), (Link, Ping, Link));
        assert!(p.pings());
        assert!(!PathSources::LINK.pings());
        assert!("meter+hold".parse::<PathSources>().is_err());
    }
}
