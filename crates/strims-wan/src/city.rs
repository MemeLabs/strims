// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Major cities and the round-trip times between them.
//!
//! The table holds approximate median RTTs between well-connected servers
//! in each pair of cities, in whole milliseconds, over the route traffic
//! usually takes (São Paulo to Johannesburg, for one, goes through Europe).
//! Routing makes some triangles break the triangle inequality, as on the
//! internet. Within a city the RTT is 2 ms. Access links add their own last
//! miles on top ([`crate::Access`]).

use core::fmt;

use strims_base::time::Duration;

/// A continent-scale region, for summaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Region {
    NorthAmerica,
    SouthAmerica,
    Europe,
    Africa,
    Asia,
    Oceania,
}

impl Region {
    pub const fn name(self) -> &'static str {
        match self {
            Region::NorthAmerica => "north-america",
            Region::SouthAmerica => "south-america",
            Region::Europe => "europe",
            Region::Africa => "africa",
            Region::Asia => "asia",
            Region::Oceania => "oceania",
        }
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A city in the RTT table. The discriminant is the city's row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum City {
    NewYork,
    Toronto,
    Chicago,
    Dallas,
    LosAngeles,
    Seattle,
    SaoPaulo,
    London,
    Amsterdam,
    Frankfurt,
    Stockholm,
    Warsaw,
    Johannesburg,
    Mumbai,
    Singapore,
    Tokyo,
    Sydney,
}

/// RTTs in milliseconds, rows and columns in [`City::ALL`] order. The
/// matrix is symmetric.
#[rustfmt::skip]
const RTT_MS: [[u16; 17]; 17] = [
    //NYC TOR CHI DAL LAX SEA SAO LON AMS FRA STO WAW JNB BOM SIN TYO SYD
    [   2, 12, 19, 38, 66, 70,118, 72, 78, 86,100,104,232,190,235,165,205], // NYC
    [  12,  2, 15, 40, 66, 66,130, 84, 90, 96,110,115,245,205,225,160,210], // TOR
    [  19, 15,  2, 26, 52, 52,135, 88, 95,100,115,120,250,210,220,145,190], // CHI
    [  38, 40, 26,  2, 36, 58,140,108,115,120,135,140,270,230,205,135,176], // DAL
    [  66, 66, 52, 36,  2, 27,170,138,142,148,160,165,290,235,172,102,140], // LAX
    [  70, 66, 52, 58, 27,  2,185,138,145,150,155,165,290,230,165, 92,165], // SEA
    [ 118,130,135,140,170,185,  2,185,195,200,215,220,330,305,330,260,310], // SAO
    [  72, 84, 88,108,138,138,185,  2,  7, 14, 32, 37,160,115,165,225,265], // LON
    [  78, 90, 95,115,142,145,195,  7,  2,  8, 24, 27,165,120,160,230,270], // AMS
    [  86, 96,100,120,148,150,200, 14,  8,  2, 22, 20,170,112,158,230,265], // FRA
    [ 100,110,115,135,160,155,215, 32, 24, 22,  2, 22,190,140,180,245,290], // STO
    [ 104,115,120,140,165,165,220, 37, 27, 20, 22,  2,195,135,180,245,290], // WAW
    [ 232,245,250,270,290,290,330,160,165,170,190,195,  2,125,235,300,300], // JNB
    [ 190,205,210,230,235,230,305,115,120,112,140,135,125,  2, 60,125,155], // BOM
    [ 235,225,220,205,172,165,330,165,160,158,180,180,235, 60,  2, 70, 92], // SIN
    [ 165,160,145,135,102, 92,260,225,230,230,245,245,300,125, 70,  2,110], // TYO
    [ 205,210,190,176,140,165,310,265,270,265,290,290,300,155, 92,110,  2], // SYD
];

impl City {
    pub const ALL: [City; 17] = [
        City::NewYork,
        City::Toronto,
        City::Chicago,
        City::Dallas,
        City::LosAngeles,
        City::Seattle,
        City::SaoPaulo,
        City::London,
        City::Amsterdam,
        City::Frankfurt,
        City::Stockholm,
        City::Warsaw,
        City::Johannesburg,
        City::Mumbai,
        City::Singapore,
        City::Tokyo,
        City::Sydney,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            City::NewYork => "new-york",
            City::Toronto => "toronto",
            City::Chicago => "chicago",
            City::Dallas => "dallas",
            City::LosAngeles => "los-angeles",
            City::Seattle => "seattle",
            City::SaoPaulo => "sao-paulo",
            City::London => "london",
            City::Amsterdam => "amsterdam",
            City::Frankfurt => "frankfurt",
            City::Stockholm => "stockholm",
            City::Warsaw => "warsaw",
            City::Johannesburg => "johannesburg",
            City::Mumbai => "mumbai",
            City::Singapore => "singapore",
            City::Tokyo => "tokyo",
            City::Sydney => "sydney",
        }
    }

    pub const fn region(self) -> Region {
        match self {
            City::NewYork
            | City::Toronto
            | City::Chicago
            | City::Dallas
            | City::LosAngeles
            | City::Seattle => Region::NorthAmerica,
            City::SaoPaulo => Region::SouthAmerica,
            City::London | City::Amsterdam | City::Frankfurt | City::Stockholm | City::Warsaw => {
                Region::Europe
            }
            City::Johannesburg => Region::Africa,
            City::Mumbai | City::Singapore | City::Tokyo => Region::Asia,
            City::Sydney => Region::Oceania,
        }
    }

    /// Latitude and longitude in degrees.
    pub const fn coords(self) -> (f64, f64) {
        match self {
            City::NewYork => (40.71, -74.01),
            City::Toronto => (43.65, -79.38),
            City::Chicago => (41.88, -87.63),
            City::Dallas => (32.78, -96.80),
            City::LosAngeles => (34.05, -118.24),
            City::Seattle => (47.61, -122.33),
            City::SaoPaulo => (-23.55, -46.63),
            City::London => (51.51, -0.13),
            City::Amsterdam => (52.37, 4.90),
            City::Frankfurt => (50.11, 8.68),
            City::Stockholm => (59.33, 18.07),
            City::Warsaw => (52.23, 21.01),
            City::Johannesburg => (-26.20, 28.05),
            City::Mumbai => (19.08, 72.88),
            City::Singapore => (1.35, 103.82),
            City::Tokyo => (35.68, 139.69),
            City::Sydney => (-33.87, 151.21),
        }
    }

    /// The great-circle distance in kilometres (haversine, mean Earth
    /// radius).
    pub fn great_circle_km(self, o: City) -> f64 {
        const EARTH_KM: f64 = 6371.0;
        let (lat1, lng1) = self.coords();
        let (lat2, lng2) = o.coords();
        let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
        let dp = (lat2 - lat1).to_radians();
        let dl = (lng2 - lng1).to_radians();
        let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
        2.0 * EARTH_KM * h.sqrt().min(1.0).asin()
    }

    /// The round-trip time between servers in the two cities.
    pub fn rtt(self, o: City) -> Duration {
        let ms = RTT_MS
            .get(self as usize)
            .and_then(|row| row.get(o as usize))
            .copied()
            .unwrap_or(0);
        Duration::from_millis(i64::from(ms))
    }
}

impl fmt::Display for City {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Light in fiber covers about 200 km per millisecond.
    const FIBER_KM_PER_MS: f64 = 200.0;

    #[test]
    fn all_lists_every_row_in_order() {
        for (i, c) in City::ALL.iter().enumerate() {
            assert_eq!(*c as usize, i, "{c}");
        }
        assert_eq!(City::ALL.len(), RTT_MS.len());
    }

    #[test]
    fn rtts_are_symmetric_and_physical() {
        for a in City::ALL {
            assert_eq!(a.rtt(a), Duration::from_millis(2), "{a}");
            for b in City::ALL {
                assert_eq!(a.rtt(b), b.rtt(a), "{a} {b}");
                if a == b {
                    continue;
                }
                let floor_ms = 2.0 * a.great_circle_km(b) / FIBER_KM_PER_MS;
                let ms = a.rtt(b).as_millis() as f64;
                assert!(
                    ms >= floor_ms,
                    "{a}–{b}: {ms} ms is below the fiber floor of {floor_ms:.1} ms"
                );
                assert!(ms <= 400.0, "{a}–{b}: {ms} ms");
            }
        }
    }

    #[test]
    fn great_circle_distances_are_plausible() {
        let km = City::NewYork.great_circle_km(City::London);
        assert!((5500.0..5650.0).contains(&km), "{km}");
        let km = City::Sydney.great_circle_km(City::Tokyo);
        assert!((7700.0..7900.0).contains(&km), "{km}");
        assert!(City::Tokyo.great_circle_km(City::Tokyo) < 1e-6);
    }
}
