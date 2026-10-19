// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Access links and sites.
//!
//! A [`Profile`] gives typical figures for one kind of consumer or server
//! connection. Loss is the residual IP packet loss once the link layer
//! has recovered what it can; it stays low because the simulations don't
//! model a TCP sender's congestion response to it.

use core::fmt;

use strims_base::time::Duration;

use crate::city::City;

/// Bytes per second in one Mbit/s.
pub const MBIT: u64 = 125_000;

/// A kind of access link.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Access {
    Datacenter,
    Fiber,
    Cable,
    Dsl,
    Lte,
    Starlink,
}

/// An access link's figures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Profile {
    /// Downlink rate in bytes per second.
    pub down: u64,
    /// Uplink rate in bytes per second.
    pub up: u64,
    /// One-way delay between the host and its city's backbone.
    pub last_mile: Duration,
    /// Mean extra delay in each direction (see [`crate::network`]).
    pub jitter: Duration,
    /// Probability that a packet crossing the link is lost.
    pub loss: f64,
}

impl Access {
    pub const ALL: [Access; 6] = [
        Access::Datacenter,
        Access::Fiber,
        Access::Cable,
        Access::Dsl,
        Access::Lte,
        Access::Starlink,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Access::Datacenter => "datacenter",
            Access::Fiber => "fiber",
            Access::Cable => "cable",
            Access::Dsl => "dsl",
            Access::Lte => "lte",
            Access::Starlink => "starlink",
        }
    }

    pub const fn profile(self) -> Profile {
        const fn p(down: u64, up: u64, last_mile_us: i64, jitter_us: i64, loss: f64) -> Profile {
            Profile {
                down: down * MBIT,
                up: up * MBIT,
                last_mile: Duration::from_micros(last_mile_us),
                jitter: Duration::from_micros(jitter_us),
                loss,
            }
        }
        match self {
            Access::Datacenter => p(1000, 1000, 500, 100, 0.0),
            Access::Fiber => p(500, 500, 2_000, 500, 0.000_05),
            Access::Cable => p(300, 20, 7_000, 3_000, 0.000_2),
            Access::Dsl => p(50, 10, 10_000, 2_000, 0.000_5),
            Access::Lte => p(30, 8, 25_000, 12_000, 0.002),
            Access::Starlink => p(100, 12, 15_000, 8_000, 0.003),
        }
    }
}

impl fmt::Display for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Where a host sits: its city and its access link, whose profile a site
/// may adjust.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Site {
    pub city: City,
    pub access: Access,
    pub profile: Profile,
}

impl Site {
    pub const fn new(city: City, access: Access) -> Site {
        Site {
            city,
            access,
            profile: access.profile(),
        }
    }

    /// The same site with an uplink of `up` bytes per second.
    pub const fn with_up(mut self, up: u64) -> Site {
        self.profile.up = up;
        self
    }

    /// The same site with a downlink of `down` bytes per second.
    pub const fn with_down(mut self, down: u64) -> Site {
        self.profile.down = down;
        self
    }
}

impl fmt::Display for Site {
    /// `<city> <access> <down>/<up>` with rates in kbit/s.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {}/{}",
            self.city,
            self.access,
            self.profile.down * 8 / 1000,
            self.profile.up * 8 / 1000
        )
    }
}

/// How residential viewers connect, by weight.
pub const RESIDENTIAL: &[(Access, u32)] = &[
    (Access::Cable, 40),
    (Access::Fiber, 30),
    (Access::Dsl, 15),
    (Access::Lte, 10),
    (Access::Starlink, 5),
];

/// Where a worldwide audience lives, by weight.
pub const GLOBAL_AUDIENCE: &[(City, u32)] = &[
    (City::NewYork, 10),
    (City::Toronto, 5),
    (City::Chicago, 8),
    (City::Dallas, 7),
    (City::LosAngeles, 9),
    (City::Seattle, 5),
    (City::SaoPaulo, 6),
    (City::London, 10),
    (City::Amsterdam, 5),
    (City::Frankfurt, 7),
    (City::Stockholm, 4),
    (City::Warsaw, 4),
    (City::Johannesburg, 2),
    (City::Mumbai, 4),
    (City::Singapore, 4),
    (City::Tokyo, 4),
    (City::Sydney, 4),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_are_sane() {
        for a in Access::ALL {
            let p = a.profile();
            assert!(p.up > 0 && p.down >= p.up, "{a}");
            assert!((0.0..0.01).contains(&p.loss), "{a}");
            assert!(p.last_mile > Duration::ZERO, "{a}");
        }
        assert_eq!(Access::Cable.profile().up, 2_500_000);
    }

    #[test]
    fn sites_adjust_their_profile() {
        let s = Site::new(City::Chicago, Access::Cable).with_up(MBIT);
        assert_eq!(s.profile.up, MBIT);
        assert_eq!(s.profile.down, 300 * MBIT);
        assert_eq!(s.to_string(), "chicago cable 300000/1000");
    }

    #[test]
    fn mixes_cover_every_city_and_residential_access() {
        for c in City::ALL {
            assert!(GLOBAL_AUDIENCE.iter().any(|&(x, w)| x == c && w > 0), "{c}");
        }
        assert!(RESIDENTIAL.iter().all(|&(a, _)| a != Access::Datacenter));
    }
}
