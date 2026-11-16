// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Access links and sites.
//!
//! A [`Profile`] gives typical figures for one kind of consumer or server
//! connection. Loss is the residual IP packet loss once the link layer
//! has recovered what it can. A radio link loses most of it in bursts
//! ([`Bursts`]: a fade, a handover, a satellite's reassignment) rather than
//! packet by packet, and a run of losses costs a TCP sender one cut of its
//! window where as many spread out cost one each. The rates, delays and
//! loss are typical figures, not measured ones; the comments beside LTE's
//! and Starlink's bursts give what was measured.

use core::fmt;

use strims_base::time::Duration;

use crate::city::City;
use crate::framing::Framing;
use crate::slots::Slots;

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
    /// Probability that a packet crossing the link is lost (between
    /// bursts, with them).
    pub loss: f64,
    /// Loss that comes in bursts, if the link has any.
    pub bursts: Option<Bursts>,
    /// The uplink's shaper bucket, if it has one ([`Bucket`]). `None` in
    /// every [`Access::profile`]: the uplink drains at its rate alone.
    /// [`Access::bucket`] gives a kind's typical one.
    pub bucket: Option<Bucket>,
    /// What the uplink charges a packet past its IP bytes.
    /// [`Framing::NONE`] in every [`Access::profile`]; [`Access::framing`]
    /// gives a kind's.
    pub framing: Framing,
    /// The uplink's medium access in slots, ahead of its shaper, if it has
    /// any: `None` in every [`Access::profile`]; [`Access::slots`] gives a
    /// kind's typical ones.
    pub slots: Option<Slots>,
}

/// An uplink shaper's token bucket. Tokens fill at the uplink's rate (its
/// steps' from their times) up to `bytes`; the bytes of a message that the
/// tokens cover leave at once, or at `peak` bytes per second, and the rest
/// at the uplink's rate, so a burst after idle reaches the path faster
/// than the rate alone lets it (see [`crate::network`]).
///
/// Linux's tbf without a peakrate (`tbf rate <r> burst <bytes>`, as
/// `hack/testbed/run.sh` shapes its containers' egress) is a bucket of
/// `bytes` with no `peak`: it keeps its tokens as transmission time at the
/// rate, at most its buffer's, and dequeues a packet once they cover it
/// whole, so after idle the first `bytes` of a burst go at once, the next
/// packet when the rate has refilled what it lacks, and so on. A line's own
/// rate behind the shaper (a cable modem's upstream channels, a fibre
/// ONT's) is `peak`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bucket {
    pub bytes: u64,
    pub peak: Option<u64>,
}

impl Bucket {
    /// A bucket of `bytes` whose bytes leave at once, as tbf's do over a
    /// veth.
    pub const fn of(bytes: u64) -> Bucket {
        Bucket { bytes, peak: None }
    }
}

/// Loss in bursts on an access link, both ways: a burst starts every
/// `every` on average (at random, or exactly every `every` at `phase` past
/// a multiple of it in unix time, at every such link at once), lasts
/// `length` on average (exponentially, at most [`BURST_MAX`]), and loses
/// each packet with probability `loss`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bursts {
    pub every: Duration,
    pub phase: Option<Duration>,
    pub length: Duration,
    pub loss: f64,
}

/// The longest burst.
pub const BURST_MAX: Duration = Duration::SECOND;

impl Bursts {
    /// The mean loss on a link that loses `between` of its packets between
    /// bursts.
    pub fn mean_loss(&self, between: f64) -> f64 {
        let duty = (self.length.as_secs_f64() / self.every.as_secs_f64()).min(1.0);
        between * (1.0 - duty) + self.loss.max(between) * duty
    }
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
                bursts: None,
                bucket: None,
                framing: Framing::NONE,
                slots: None,
            }
        }
        const fn bursty(mut p: Profile, bursts: Bursts) -> Profile {
            p.bursts = Some(bursts);
            p
        }
        match self {
            Access::Datacenter => p(1000, 1000, 500, 100, 0.0),
            Access::Fiber => p(500, 500, 2_000, 500, 0.000_05),
            Access::Cable => p(300, 20, 7_000, 3_000, 0.000_2),
            Access::Dsl => p(50, 10, 10_000, 2_000, 0.000_5),
            // 0.2 % on the whole: fades and handovers of 50 ms every 4 s
            // that lose 12 % of their packets. A handover interrupts
            // 40-43 ms at the median, about 90 ms at p95 (Lauridsen et al.,
            // IEEE Comm. Mag. 2017; Kalntis et al., IMC 2024); how often,
            // and what an uplink loses past HARQ and RLC, no measurement
            // found says.
            Access::Lte => bursty(
                p(30, 8, 25_000, 12_000, 0.000_5),
                Bursts {
                    every: Duration::from_secs(4),
                    phase: None,
                    length: Duration::from_millis(50),
                    loss: 0.12,
                },
            ),
            // 0.3 % on the whole: the satellites' reassignment every 15 s
            // (12, 27, 42 and 57 s past each minute: Tanveer et al., CoNEXT
            // 2023 Companion; Mohan et al., WWW 2024) loses 30 % for 120 ms.
            // A boundary's gap is 45-120 ms with its loss there (Lai et al.,
            // SIGCOMM 2025; StarQUIC, LEO-NET 2024); the uplink loses 0.45 %
            // lightly loaded and 1.96 % under a bulk upload, 85 % of its
            // runs of loss a single packet (Michel et al., IMC 2022).
            Access::Starlink => bursty(
                p(100, 12, 15_000, 8_000, 0.000_5),
                Bursts {
                    every: Duration::from_secs(15),
                    phase: Some(Duration::from_secs(12)),
                    length: Duration::from_millis(120),
                    loss: 0.3,
                },
            ),
        }
    }
}

/// How long a cable uplink's bucket lets it send at its peak, at its rate:
/// a CMTS's Maximum Traffic Burst of at least the sustained rate over 64
/// (Cisco's uBR guidance, "Upstream Scheduler Mode Configuration for the
/// Cisco uBR CMTS", which in bytes is 1/8 s of the rate). DOCSIS 3.0 MULPI
/// (ETSI EN 302 878-4, C.2.2.5.3) defaults it to 3044 bytes when the
/// operator leaves it out, at least 1522; PowerBoost-era Comcast buckets
/// were 5-10 MB (Bauer, Clark and Lehr, HomeNets 2011, table 1;
/// Kanuparthy and Dovrolis, IMC 2011, table 2). This is the middle of that
/// range, and the guidance a CMTS's own vendor gives.
pub const CABLE_BURST: Duration = Duration::from_millis(125);

/// The rate a cable uplink's bucket sends at: four bonded DOCSIS 3.0
/// upstream channels of about 27 Mbit/s each (A-TDMA at 5.12 Msym/s, 64-QAM;
/// Cisco, "Data Throughput in a DOCSIS World"), where a burst goes out in
/// the minislots the CMTS grants.
pub const CABLE_PEAK: u64 = 100 * MBIT;

impl Access {
    /// The shaper bucket a link of this kind's uplink typically has at a
    /// rate of `up` bytes per second, `None` where a token bucket doesn't
    /// model how its uplink sends:
    ///
    /// - **Cable**: DOCSIS shapes the upstream service flow with a token
    ///   bucket, `T·R/8 + B` bytes over any `T` (MULPI C.2.2.5.2), which
    ///   the modem shapes to and the CMTS enforces by its grants:
    ///   [`CABLE_BURST`] of the rate, leaving at [`CABLE_PEAK`].
    /// - **DSL**: the modem serializes its upstream at the line's sync
    ///   rate, the cap itself, so no burst passes it; a BNG policer behind
    ///   it, where there is one, only sees what the line has paced
    ///   (ShaperProbe, IMC 2011: no shaping on the DSL lines it measured).
    /// - **Fibre**: a GPON ONU's upstream goes in the grants the OLT's
    ///   dynamic bandwidth assignment sizes, at most its T-CONT's maximum
    ///   bandwidth in every 125 µs frame (ITU-T G.984.3, 7.4 and table
    ///   7-1), whose traffic descriptor has no burst size: a frame's
    ///   share, 7.8 KB at 500 Mbit/s, passes in 125 µs, under the sims'
    ///   millisecond.
    /// - **LTE** and **Starlink**: the base station or satellite schedules
    ///   uplink grants on request (a scheduling request, a buffer status
    ///   report, grants each TTI: 3GPP TR 36.912, B.1.2; Starlink's
    ///   on-demand allocation, Mohan et al., WWW 2024), so an idle uplink
    ///   sends later, not faster. A bucket models none of that.
    /// - **Datacenter**: at its line rate, 1 Gbit/s, a bucket's bytes
    ///   drain in microseconds; a cloud's burst credits act over minutes
    ///   (AWS EC2's network I/O credits).
    ///
    /// Home routers' shapers pass no more: cake paces from the first
    /// packet after idle, and sqm-scripts' HTB takes a millisecond of its
    /// rate, at least 1749 bytes (sqm-scripts' `SHAPER_BURST_DUR_US`).
    pub const fn bucket(self, up: u64) -> Option<Bucket> {
        match self {
            Access::Cable => Some(Bucket {
                bytes: up * CABLE_BURST.as_millis() as u64 / 1000,
                peak: Some(if up > CABLE_PEAK { up } else { CABLE_PEAK }),
            }),
            Access::Datacenter | Access::Fiber | Access::Dsl | Access::Lte | Access::Starlink => {
                None
            }
        }
    }
}

/// A cable modem's upstream slots: an idle modem's request waits 2-8 ms
/// for its grant (CableLabs, "Low Latency DOCSIS: Technology Overview",
/// 2019, s3 and Fig 1; RFC 8034 s4.2 gives about 4-8 ms), and a loaded
/// one's piggybacked requests cycle in about 5 ms (Cisco, "Upstream
/// Scheduler Mode Configuration for the Cisco uBR CMTS"); half the cable
/// links of Dischinger et al. (IMC 2007, Fig 12) sent 19 or more small
/// packets in one upstream burst, their concatenation jitter about 5 ms on
/// average.
pub const CABLE_GRANTS: Slots = Slots {
    min: Duration::from_millis(2),
    max: Duration::from_millis(8),
};

impl Access {
    /// What a link of this kind charges a packet past its IP bytes
    /// (tc-cake(8)'s overhead keywords, which the Docker testbed's link
    /// classes take, `hack/testbed/links.sh`):
    ///
    /// - **Cable**: a DOCSIS service flow counts the Ethernet frame through
    ///   its CRC, 18 bytes, at least 64 ([`Framing::DOCSIS`]).
    /// - **DSL**: ADSL's ATM cells under PPPoE over LLC/SNAP, 40 bytes padded
    ///   to 48-byte payloads of 53-byte cells ([`Framing::ADSL_PPPOE`]): a
    ///   64-byte IP packet costs 159 bytes, a 1500-byte one 1749.
    /// - **Fibre**: the Ethernet frame with its FCS, 18 bytes; a GPON ONU
    ///   adds a 5-byte GEM header (ITU-T G.984.3), whether its rate counts
    ///   it or not.
    /// - **LTE** and **Starlink**: their capacity traces count 1500-byte IP
    ///   packets, which the testbed charges 1514, Ethernet's 14
    ///   ([`Framing::ETHERNET`]).
    /// - **Datacenter**: Ethernet on the wire, 38 bytes, at least 84
    ///   ([`Framing::ETHERNET_WIRE`]).
    pub const fn framing(self) -> Framing {
        match self {
            Access::Cable => Framing::DOCSIS,
            Access::Dsl => Framing::ADSL_PPPOE,
            Access::Fiber => Framing::bytes(18, 0),
            Access::Lte | Access::Starlink => Framing::ETHERNET,
            Access::Datacenter => Framing::ETHERNET_WIRE,
        }
    }

    /// The slots a link of this kind sends its uplink in, if it has any it
    /// is known to: cable's requests and grants ([`CABLE_GRANTS`]). A GPON
    /// ONU's grants come every DBA cycle, simulated at 2-8 ms at most but
    /// never measured on a residential link; LTE's and Starlink's grants
    /// are in their capacity traces ([`crate::capacity`]).
    pub const fn slots(self) -> Option<Slots> {
        match self {
            Access::Cable => Some(CABLE_GRANTS),
            Access::Datacenter | Access::Fiber | Access::Dsl | Access::Lte | Access::Starlink => {
                None
            }
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

    /// The same site with its uplink shaped by `bucket` (`None`: the rate
    /// alone).
    pub const fn with_bucket(mut self, bucket: Option<Bucket>) -> Site {
        self.profile.bucket = bucket;
        self
    }

    /// The same site with its uplink charging packets as `framing` says.
    pub const fn with_framing(mut self, framing: Framing) -> Site {
        self.profile.framing = framing;
        self
    }

    /// The same site with its uplink's medium access in `slots` (`None`:
    /// none).
    pub const fn with_slots(mut self, slots: Option<Slots>) -> Site {
        self.profile.slots = slots;
        self
    }

    /// The site with each packet lost with probability `loss` on its
    /// access link, and no bursts.
    pub const fn with_loss(mut self, loss: f64) -> Site {
        self.profile.loss = loss;
        self.profile.bursts = None;
        self
    }
}

impl fmt::Display for Site {
    /// `<city> <access> <down>/<up>` with rates in kbit/s, and with a
    /// bucket ` burst <bytes>`, `@<peak>` in kbit/s if it has one; then
    /// ` framing <framing>` unless it is [`Framing::NONE`], and
    /// ` slots <slots>` if it has any.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {}/{}",
            self.city,
            self.access,
            self.profile.down * 8 / 1000,
            self.profile.up * 8 / 1000
        )?;
        if let Some(b) = self.profile.bucket {
            write!(f, " burst {}", b.bytes)?;
            if let Some(peak) = b.peak {
                write!(f, "@{}", peak * 8 / 1000)?;
            }
        }
        if self.profile.framing != Framing::NONE {
            write!(f, " framing {}", self.profile.framing)?;
        }
        if let Some(s) = self.profile.slots {
            write!(f, " slots {s}")?;
        }
        Ok(())
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
            let mean = p.bursts.map_or(p.loss, |b| b.mean_loss(p.loss));
            assert!((0.0..0.01).contains(&mean), "{a}");
            assert!(p.last_mile > Duration::ZERO, "{a}");
        }
        assert_eq!(Access::Cable.profile().up, 2_500_000);
    }

    #[test]
    fn bursty_links_lose_what_they_did_packet_by_packet() {
        let mean = |a: Access| {
            let p = a.profile();
            p.bursts.map_or(p.loss, |b| b.mean_loss(p.loss))
        };
        assert!((mean(Access::Lte) - 0.002).abs() < 0.000_1);
        assert!((mean(Access::Starlink) - 0.003).abs() < 0.000_2);
    }

    #[test]
    fn only_cable_has_a_typical_bucket() {
        let up = Access::Cable.profile().up;
        assert_eq!(
            Access::Cable.bucket(up),
            Some(Bucket {
                bytes: 312_500,
                peak: Some(100 * MBIT)
            })
        );
        assert_eq!(
            Access::Cable.bucket(5 * MBIT).map(|b| b.bytes),
            Some(78_125)
        );
        assert_eq!(
            Access::Cable.bucket(200 * MBIT).and_then(|b| b.peak),
            Some(200 * MBIT)
        );
        for a in Access::ALL.into_iter().filter(|&a| a != Access::Cable) {
            assert_eq!(a.bucket(a.profile().up), None, "{a}");
        }
        for a in Access::ALL {
            assert_eq!(a.profile().bucket, None, "{a}");
            assert_eq!(a.profile().framing, Framing::NONE, "{a}");
            assert_eq!(a.profile().slots, None, "{a}");
        }
        assert_eq!(Access::Cable.slots(), Some(CABLE_GRANTS));
        assert_eq!(Access::Dsl.framing().packet(1500), 1749);
        assert_eq!(Access::Cable.framing().packet(40), 64);
        let s = Site::new(City::Chicago, Access::Cable).with_bucket(Access::Cable.bucket(up));
        assert_eq!(
            s.to_string(),
            "chicago cable 300000/20000 burst 312500@100000"
        );
        let s = s
            .with_framing(Access::Cable.framing())
            .with_slots(Access::Cable.slots());
        assert_eq!(
            s.to_string(),
            "chicago cable 300000/20000 burst 312500@100000 framing 18 mpu 64 slots 2000-8000us"
        );
        let s = Site::new(City::Chicago, Access::Dsl).with_framing(Access::Dsl.framing());
        assert_eq!(s.to_string(), "chicago dsl 50000/10000 framing 40 atm");
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
