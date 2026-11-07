// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Per-node clock skew ([`Scenario::clock_skew`], `STRIMS_SIM_CLOCK_SKEW`).
//!
//! A production node reads the time off its machine's wall clock, and a
//! machine's clock can be off: everything the node stamps (live
//! signatures, DATA and REQUEST timestamps, certificates, records) is off
//! with it. With skew on, each host's `SimClock` starts at the run's epoch
//! plus an offset drawn once per run, at most [`MAX_SKEW`], so its `now`
//! and every time it makes are off by that offset in every incarnation.
//! Its timers stay right: the clock maps a deadline to the runtime's
//! instant through its own epoch.
//!
//! The harness measures in true time, the simulated time since the start,
//! never a node's: frames carry the writer's true time, a stats sample
//! reads a node's path readings against that node's clock, the WAN model's
//! capture samples read the model's time, and a capture file's origin has
//! its node's offset taken off (`OffsetCapture`).
//!
//! The offsets come from the run's rng, after the media traces and only
//! with skew on, in host order: a run without skew draws nothing and
//! traces as it did without the setting.
//!
//! [`Scenario::clock_skew`]: crate::Scenario::clock_skew

use std::fmt;
use std::io::{self, Write};
use std::str::FromStr;

use rand_core::Rng;
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_wan::sample::unit;

/// How a run skews its hosts' clocks. No host's clock is ever more than
/// [`MAX_SKEW`] off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClockSkew {
    /// Every clock reads true time, and nothing is drawn.
    #[default]
    Off,
    /// Each host's offset drawn from the [`SkewClass`]es by their
    /// [`CLOCK_SHARES`].
    Real,
    /// Every host's clock off by this much, ahead or behind with equal
    /// odds; past [`MAX_SKEW`], by that. Zero leaves every clock right
    /// but draws the signs: the rng's share of a skewed run's changes.
    Fixed(Duration),
}

impl ClockSkew {
    /// Whether it leaves every clock right, drawing nothing.
    pub fn is_off(self) -> bool {
        self == ClockSkew::Off
    }
}

/// `off`, `real`, or a magnitude in milliseconds up to [`MAX_SKEW`]'s. The
/// error is the knob's message after `<knob>=<value>: `.
impl FromStr for ClockSkew {
    type Err = String;

    fn from_str(s: &str) -> Result<ClockSkew, String> {
        match s.trim() {
            "off" => Ok(ClockSkew::Off),
            "real" => Ok(ClockSkew::Real),
            ms => {
                let d = ms
                    .parse::<u32>()
                    .map(|ms| Duration::from_millis(i64::from(ms)))
                    .map_err(|_| "off, real or <ms>".to_owned())?;
                if d > MAX_SKEW {
                    return Err(format!(
                        "past the cap, MAX_SKEW ({} ms)",
                        MAX_SKEW.as_millis()
                    ));
                }
                Ok(ClockSkew::Fixed(d))
            }
        }
    }
}

/// The knob's form ([`ClockSkew::from_str`]).
impl fmt::Display for ClockSkew {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClockSkew::Off => f.write_str("off"),
            ClockSkew::Real => f.write_str("real"),
            ClockSkew::Fixed(d) => write!(f, "{}", d.as_millis()),
        }
    }
}

/// The most any host's clock is off: ten minutes. A clock further off is
/// user error, for the UI to surface; the service layer needn't handle
/// it. Any two clocks are then at most 20 minutes apart, inside the
/// stack's tightest tolerances of another node's clock: a hash table
/// record lives [`HASH_TABLE_MAX_RECORD_AGE`] (30 min) from its
/// publisher's timestamp, so one from a publisher 20 minutes behind still
/// outlives its next republish ([`HASH_TABLE_SET_INTERVAL`], 10 min); and
/// a link's certificate is valid from [`CERT_PREDATE_DURATION`] before its
/// maker's clock to `strims_vnic::host::HOST_CERT_VALID_DURATION` after it
/// (an hour each).
///
/// [`HASH_TABLE_MAX_RECORD_AGE`]: strims_vpn::hash_table::HASH_TABLE_MAX_RECORD_AGE
/// [`HASH_TABLE_SET_INTERVAL`]: strims_vpn::hash_table::HASH_TABLE_SET_INTERVAL
/// [`CERT_PREDATE_DURATION`]: strims_crypto::CERT_PREDATE_DURATION
pub const MAX_SKEW: Duration = Duration::from_secs(10 * 60);

/// The classes of clock offsets, by magnitude ([`SkewClass::bounds`]).
/// [`ClockSkew::Real`] draws a class by its [`CLOCK_SHARES`], then a
/// magnitude in its bounds, floored to whole milliseconds, then a sign; a
/// report counts any offset into the class its magnitude falls in
/// ([`SkewClass::of`]). The ppspp sim (`strims-ppspp`'s `tests/sim.rs`)
/// draws its nodes' clocks from the same classes, and its rows name them
/// alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkewClass {
    /// Under [`SYNCED_MAX`], uniform: clocks NTP keeps.
    Synced,
    /// Up to [`JITTER_MAX`], log-uniform: jitter, a sync at boot only,
    /// mobile handoffs.
    Jitter,
    /// Up to [`DRIFT_MAX`], log-uniform: failing CMOS batteries,
    /// throttling, VMs waking without NTP.
    Drift,
    /// Up to [`MAX_SKEW`], log-uniform: the long tail past 15 s, cut at
    /// the cap.
    Tail,
}

impl SkewClass {
    /// Every class, in order of magnitude.
    pub const ALL: [SkewClass; 4] = [
        SkewClass::Synced,
        SkewClass::Jitter,
        SkewClass::Drift,
        SkewClass::Tail,
    ];

    /// Its bounds on an offset's magnitude, `[lo, hi)`.
    pub fn bounds(self) -> (Duration, Duration) {
        match self {
            SkewClass::Synced => (Duration::ZERO, SYNCED_MAX),
            SkewClass::Jitter => (SYNCED_MAX, JITTER_MAX),
            SkewClass::Drift => (JITTER_MAX, DRIFT_MAX),
            SkewClass::Tail => (DRIFT_MAX, MAX_SKEW),
        }
    }

    /// How a row counts it, as `clock_<name>`; the tail's count takes
    /// offsets of [`MAX_SKEW`] itself, which `<ms>` allows.
    pub fn name(self) -> &'static str {
        match self {
            SkewClass::Synced => "lt_100ms",
            SkewClass::Jitter => "lt_2s",
            SkewClass::Drift => "lt_15s",
            SkewClass::Tail => "le_10min",
        }
    }

    /// The class `offset`'s magnitude falls in.
    pub fn of(offset: Duration) -> SkewClass {
        let m = offset.abs();
        SkewClass::ALL
            .into_iter()
            .find(|c| m < c.bounds().1)
            .unwrap_or(SkewClass::Tail)
    }
}

/// The share of nodes in each [`SkewClass`], in [`SkewClass::ALL`]'s
/// order: from web telemetry of visitors' clocks against the server's,
/// whose 0.1 % past an hour (dead RTCs, clocks set off by hand) goes to
/// the tail, under [`MAX_SKEW`].
pub const CLOCK_SHARES: [f64; 4] = [0.90, 0.05, 0.04, 0.01];

/// The bound of [`SkewClass::Synced`] (from the same telemetry as
/// [`CLOCK_SHARES`], as are the bounds below).
pub const SYNCED_MAX: Duration = Duration::from_millis(100);
/// The bound of [`SkewClass::Jitter`].
pub const JITTER_MAX: Duration = Duration::from_secs(2);
/// The bound of [`SkewClass::Drift`].
pub const DRIFT_MAX: Duration = Duration::from_secs(15);

/// Each host's offset under `skew`, drawn from `rng` in host order; empty,
/// and nothing drawn, when it's off.
pub(crate) fn offsets(skew: ClockSkew, hosts: usize, rng: &mut DetRng) -> Vec<Duration> {
    match skew {
        ClockSkew::Off => Vec::new(),
        ClockSkew::Real => (0..hosts).map(|_| real(rng)).collect(),
        ClockSkew::Fixed(d) => (0..hosts)
            .map(|_| signed(rng, d.abs().min(MAX_SKEW)))
            .collect(),
    }
}

/// One offset of [`ClockSkew::Real`].
fn real(rng: &mut DetRng) -> Duration {
    let u = unit(rng);
    let mut below = 0.0;
    let class = SkewClass::ALL
        .into_iter()
        .zip(CLOCK_SHARES)
        .find(|&(_, share)| {
            below += share;
            u < below
        })
        .map_or(SkewClass::Tail, |(c, _)| c);
    let (lo, hi) = class.bounds();
    let (lo, hi) = (lo.as_millis() as f64, hi.as_millis() as f64);
    let x = unit(rng);
    let ms = match class {
        SkewClass::Synced => lo + (hi - lo) * x,
        _ => lo * (hi / lo).powf(x),
    };
    signed(rng, Duration::from_millis(ms.min(hi - 1.0) as i64))
}

/// `m` to whole milliseconds (the clock's precision), ahead or behind
/// with equal odds.
fn signed(rng: &mut DetRng, m: Duration) -> Duration {
    let m = m.truncate(Time::PRECISION);
    if rng.next_u64() & 1 == 0 { m } else { -m }
}

/// The capture file magic and version whose header [`OffsetCapture`]
/// knows: `strims_ppspp_capture`'s format 2.
const CAPTURE_MAGIC: &[u8; 10] = b"STRIMSPC\x00\x02";

/// A node's capture file with its node's clock offset taken off the
/// header's origin. Every record's time is the origin plus the deltas
/// before it, so the records then carry true time, and dumps of nodes
/// with different offsets line up (a swarm view's transit times are
/// true). What the records hold is as the node made it, on its clock: a
/// DATA message's timestamp, a tap's time fields. So a span from one of
/// those to its record's time (an export's `wait`, from a tap's queueing
/// time) reads off by the offset.
///
/// The header follows the magic and version: its record's length, its
/// kind, its zero time delta, the clock, then the origin as eight
/// big-endian bytes of nanoseconds. A file of another format fails.
pub(crate) struct OffsetCapture<W> {
    file: W,
    offset: Duration,
    /// The bytes before the origin's end, until they're all written.
    head: Option<Vec<u8>>,
}

impl<W: Write> OffsetCapture<W> {
    pub fn new(file: W, offset: Duration) -> OffsetCapture<W> {
        OffsetCapture {
            file,
            offset,
            head: Some(Vec::new()),
        }
    }

    /// Where the origin starts in `head`, once its record's length is in.
    fn origin_at(head: &[u8]) -> Option<usize> {
        let len = head.get(CAPTURE_MAGIC.len()..)?;
        let n = len.iter().position(|b| b & 0x80 == 0)? + 1;
        // the length, then the kind, the zero delta and the clock
        Some(CAPTURE_MAGIC.len() + n + 3)
    }
}

impl<W: Write> Write for OffsetCapture<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let Some(head) = self.head.as_mut() else {
            return self.file.write(buf);
        };
        head.extend_from_slice(buf);
        let n = CAPTURE_MAGIC.len().min(head.len());
        if head.get(..n) != CAPTURE_MAGIC.get(..n) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "no capture format the clock offset knows",
            ));
        }
        let Some(at) = Self::origin_at(head) else {
            return Ok(buf.len());
        };
        let Some(origin) = head.get_mut(at..at + Time::BYTE_LEN) else {
            return Ok(buf.len());
        };
        let mut nanos = [0; Time::BYTE_LEN];
        nanos.copy_from_slice(origin);
        let t = Time::from_unix_nanos(i64::from_be_bytes(nanos)) - self.offset;
        origin.copy_from_slice(&t.unix_nanos().to_be_bytes());
        self.file.write_all(head)?;
        self.head = None;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use strims_base::rng::det_rng;

    #[test]
    fn real_offsets_follow_the_shares() {
        let mut rng = det_rng(7);
        let n = 200_000;
        let offsets = offsets(ClockSkew::Real, n, &mut rng);
        let mut per = [0u64; SkewClass::ALL.len()];
        let mut ahead = 0u64;
        for &o in &offsets {
            per[SkewClass::of(o) as usize] += 1;
            ahead += u64::from(o > Duration::ZERO);
            assert_eq!(o, o.truncate(Time::PRECISION));
            assert!(o.abs() < MAX_SKEW, "{o}");
        }
        for (got, share) in per.iter().zip(CLOCK_SHARES) {
            let want = n as f64 * share;
            let sd = want.sqrt();
            assert!((*got as f64 - want).abs() < 5.0 * sd + 2.0, "{per:?}");
        }
        let half = n as f64 / 2.0;
        assert!((ahead as f64 - half).abs() < 5.0 * half.sqrt(), "{ahead}");
    }

    #[test]
    fn fixed_offsets_take_either_sign_up_to_the_cap() {
        let mut rng = det_rng(1);
        let fixed = ClockSkew::Fixed(Duration::from_secs(15));
        let o = offsets(fixed, 64, &mut rng);
        assert!(o.iter().all(|d| d.abs() == Duration::from_secs(15)));
        assert!(o.iter().any(|d| d.is_negative()) && o.iter().any(|d| !d.is_negative()));
        let past = offsets(ClockSkew::Fixed(Duration::HOUR), 8, &mut rng);
        assert!(past.iter().all(|d| d.abs() == MAX_SKEW));
    }

    #[test]
    fn off_draws_nothing_and_zero_draws_the_signs() {
        let (mut a, b) = (det_rng(3), det_rng(3));
        assert!(offsets(ClockSkew::Off, 8, &mut a).is_empty());
        assert_eq!(a, b);
        let zero = offsets(ClockSkew::Fixed(Duration::ZERO), 8, &mut a);
        assert_eq!(zero, [Duration::ZERO; 8]);
        assert_ne!(a, b);
    }

    #[test]
    fn a_skewed_capture_reads_in_true_time() {
        use strims_ppspp_capture::{Clock, Header, MAGIC, Reader, Record, VERSION, Writer};
        let mut magic = MAGIC.to_vec();
        magic.extend_from_slice(&VERSION.to_be_bytes());
        assert_eq!(&magic[..], &CAPTURE_MAGIC[..]);
        let offset = -Duration::from_secs(90);
        let origin = crate::world::epoch() + Duration::from_secs(5);
        // a label past 127 bytes takes a second byte of record length
        for label in ["v0".to_owned(), "x".repeat(200)] {
            let header = Header {
                clock: Clock::Node,
                origin: origin + offset,
                file: 0,
                node: b"v0",
                label: &label,
            };
            let mut w = Writer::new(OffsetCapture::new(Vec::new(), offset), &header).unwrap();
            let peer = Record::Peer {
                peer: 1,
                name: b"seed",
            };
            w.write(origin + offset + Duration::from_millis(250), &peer)
                .unwrap();
            let bytes = w.into_inner().file;
            let mut r = Reader::new(&bytes[..]).unwrap();
            assert_eq!(r.header().origin, origin);
            assert_eq!(r.header().label, label);
            let e = r.next_record().unwrap().unwrap();
            assert_eq!(e.time, origin + Duration::from_millis(250));
        }
        let mut other = OffsetCapture::new(Vec::new(), offset);
        assert!(other.write(b"PCAPNG").is_err());
    }

    #[test]
    fn the_knob_parses() {
        for (s, want) in [
            ("off", ClockSkew::Off),
            (" real", ClockSkew::Real),
            ("0", ClockSkew::Fixed(Duration::ZERO)),
            ("15000", ClockSkew::Fixed(Duration::from_secs(15))),
            ("600000", ClockSkew::Fixed(MAX_SKEW)),
        ] {
            let got: ClockSkew = s.parse().unwrap();
            assert_eq!(got, want);
            assert_eq!(got.to_string().parse::<ClockSkew>().unwrap(), want);
        }
        let past = "600001".parse::<ClockSkew>().unwrap_err();
        assert_eq!(past, "past the cap, MAX_SKEW (600000 ms)");
        assert_eq!(SkewClass::of(MAX_SKEW), SkewClass::Tail);
        assert_eq!(SkewClass::of(-SYNCED_MAX), SkewClass::Jitter);
        for bad in ["dead", "-5", "soon"] {
            assert!(bad.parse::<ClockSkew>().is_err(), "{bad}");
        }
    }
}
