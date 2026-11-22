// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The calibration scenario: `hack/testbed/run.sh`'s shaped links in the
//! WAN model ([`calibration_star`]), from the testbed's settings under the
//! knobs' names ([`Calibration`]). strims-sim's `calibration_star` test
//! runs it as the testbed's twin, and [`crate::testbed`] reads a testbed
//! run against it.
//!
//! A mistyped setting fails its run, and a topology's links name the
//! scenario's own hosts.
#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]

use std::time::Duration;

use strims_ppspp::{PushMode, SeedPolicy};
use strims_wan::{Access, Bucket, City, Site};

use crate::spec::{
    Competing, Fault, HostIx, Invariants, PageTiming, Scenario, SendBuffer, SwarmSpec, Transport,
};

const KIB: u64 = 1024;
const MIB: i64 = 1 << 20;

/// The stream rate of the WAN scenarios: about 2 Mbit/s.
pub const WAN_RATE: u64 = 256 * KIB;
/// Their live window: 16 s of stream.
pub const WAN_WINDOW: usize = 4096;
/// The uplink queue of the uncapped WAN cases.
pub const UPLINK_QUEUE: Duration = Duration::from_millis(100);
/// The stream count of the striped WAN cases.
pub const STRIPES: usize = 16;
/// An adaptive seed with uplink shares by allocation.
pub const ADAPTIVE: SeedPolicy = SeedPolicy::ADAPTIVE;

fn secs(s: u64) -> Duration {
    Duration::from_secs(s)
}

/// `s` with every swarm in `streams` streams and its seed on `policy`.
pub fn striped(mut s: Scenario, streams: usize, policy: SeedPolicy) -> Scenario {
    for sw in &mut s.swarms {
        sw.options.stream_count = streams;
        sw.options.seed = policy;
    }
    let adaptive = if policy == SeedPolicy::ADMISSION {
        ""
    } else {
        " adaptive"
    };
    s.name = format!("{} x{streams}{adaptive}", s.name);
    s
}

/// Knob `name`'s value as `get` reads it, parsed; `None` unset or empty.
fn value<T: std::str::FromStr>(get: &impl Fn(&str) -> Option<String>, name: &str) -> Option<T> {
    let v = get(name)?;
    let v = v.trim();
    if v.is_empty() {
        return None;
    }
    match v.parse() {
        Ok(x) => Some(x),
        Err(_) => panic!("{name}={v} doesn't parse"),
    }
}

/// Env var `name`'s comma list of numbers (`<x>,<x>...`), empty where
/// unset or empty; panics on one that doesn't parse.
fn values(get: &impl Fn(&str) -> Option<String>, name: &str) -> Vec<f64> {
    let v = get(name).unwrap_or_default();
    let v = v.trim();
    if v.is_empty() {
        return Vec::new();
    }
    v.split(',')
        .map(|x| match x.trim().parse() {
            Ok(x) => x,
            Err(_) => panic!("{name}={v} doesn't parse"),
        })
        .collect()
}

/// The testbed's settings, under the names `run.sh` takes with a
/// `STRIMS_SIM_CAL_` prefix (and the uplink queue from
/// `STRIMS_SIM_SEND_BUFFER`). `STRIMS_SIM_CAL_JITTER=1` adds the cable
/// profile's jitter, so seeds differ the way a real link's timing does.
///
/// Three more serve sweeps on the sim farm, whose jobs take no files:
///
/// - `STRIMS_SIM_CAL_MESH=full` links every pair of nodes, and
///   `STRIMS_SIM_CAL_MESH=mesh6` takes the farm's testbed meshes
///   (`hack/sim-farm/matrices/topologies/mesh6-v<viewers>`: the seed linked
///   to up to 32 viewers, each viewer to 6 others), built in, for 3, 7, 15,
///   31 or 63 viewers. Either replaces `STRIMS_SIM_CAL_TOPOLOGY`.
/// - `STRIMS_SIM_CAL_SEED_UP_SCALE=<s>:<factor> ...` steps the seed's
///   uplink to `factor` times `STRIMS_SIM_CAL_SEED_UP_MBIT`, so one value
///   fits every seed uplink (`60:0.5 180:1` halves it from 60 s to 180 s).
///   It replaces `STRIMS_SIM_CAL_SEED_UP_STEPS`.
/// - `STRIMS_SIM_CAL_SEED_UP_STEPS_MS="<ms> ..."`, a testbed run's times
///   of those steps (`testbed::Run::read`: each one's stamp less the seed's
///   egress delay, ms of stream), one a step in their order, takes the
///   place of their scheduled times; their rates stay the schedule's.
/// - Each viewer makes its links as the testbed's harness does
///   ([`HostSpec::links_at`](crate::spec::HostSpec::links_at)): its node
///   boots with the run, links to the hosts no swarm has (the relay) at
///   once, and to its swarms' hosts all at once at its join, the stream's
///   start or its arrival; `STRIMS_SIM_CAL_HARNESS_LINKS=0` boots each at
///   its arrival (with the run, for one there from the start) and dials
///   every link then.
/// - `STRIMS_SIM_CAL_JOIN_AT_MS=<ms>` boots the viewers that long after the
///   stream starts (before it if negative); `STRIMS_SIM_CAL_JOIN_AT=<s>`,
///   the testbed's `JOIN_AT`, in seconds. Viewers that join after the
///   stream starts arrive then
///   ([`HostSpec::arrive_at`](crate::spec::HostSpec::arrive_at)) and read
///   from the live edge, as the testbed's late viewers do; at or before
///   it, each boots with the run and makes its links then (with
///   `STRIMS_SIM_CAL_HARNESS_LINKS=0`, it boots by a restart of a host
///   down from the run's start), and reads as the swarm's options say, as
///   the testbed's viewers at the start do.
/// - `STRIMS_SIM_CAL_JOINS="<i>:<s> ..."`, the testbed's `JOINS`: viewer
///   `i` (the topology's node `i`) joins `s` seconds after the stream
///   starts, to the ms, so a join falls anywhere within its second, and
///   arrives then whatever the time (0 too), as run.sh's `--arrive` has
///   it; a viewer it doesn't name joins at JOIN_AT's time.
/// - `STRIMS_SIM_CAL_EPOCH_MS=<ms>`, a testbed run's stream start's ms in
///   its second (`testbed::Run::read`), is the run's epoch phase
///   ([`Scenario::epoch_phase`]), 0 to 999: the nodes' clocks then stand
///   to the stream's writes as the testbed's did, their upload meters'
///   100 ms intervals at the run's phase. Unset, each seed draws its own.
/// - `STRIMS_SIM_CAL_PACKET_COST_US=<µs>`, a testbed run's Chromium nodes'
///   cost of a packet on their network thread (`testbed::thread_cost`,
///   their host's CPU's), is each Chromium host's
///   ([`Scenario::packet_cost`]); unset, its stack's 12 µs
///   ([`SctpProfile::CHROMIUM`](crate::SctpProfile::CHROMIUM)).
/// - `STRIMS_SIM_CAL_FIREFOX_GATHER_MS=<ms>,...`,
///   `STRIMS_SIM_CAL_FIREFOX_POST_MS=<ms>,...` and
///   `STRIMS_SIM_CAL_CHROMIUM_SET_MS=<ms>,...`, a testbed run's browser
///   pages' offer times (`testbed::offer_timing`, their host's): a Firefox
///   page's gathering end after its node's connect and its offer's post
///   after that, and a Chromium page's lone setting after the connect, are
///   their stacks' ([`Scenario::page_timing`]), an entry a host of its
///   kind in host order, or one for every such host; unset, each stack's.
/// - `STRIMS_SIM_CAL_SEED_BURST=<bytes>` and `STRIMS_SIM_CAL_VIEWER_BURST=<bytes>`
///   give the seed's and each viewer's uplink a shaper bucket that size
///   (`strims_wan::Bucket`, leaving at once), as `run.sh`'s `SEED_BURST`
///   and `VIEWER_BURST` set its tbf's; unset, none (the testbed's
///   defaults, 3028 each, are a testbed run's: `testbed::knobs_of`).
///   `STRIMS_SIM_CAL_SEED_PEAK_MBIT=<m>` and `STRIMS_SIM_CAL_VIEWER_PEAK_MBIT=<m>`
///   send that bucket at that rate (`Bucket::peak`), as `run.sh`'s
///   `SEED_PEAK_MBIT` and `VIEWER_PEAK_MBIT` set its tbf's peakrate; a
///   peak needs its bucket's knob, and is over every rate its uplink runs
///   at, the seed's steps' too (`strims_wan::testbed::check_peak`).
/// - `STRIMS_SIM_CAL_RATE_LIMIT_PCT=<pct>` gives every node a rate limit
///   (`NodeConfig::rate_limit`) of `pct` percent of its starting uplink,
///   so the node's QoS, not the access link, is where its peers' data
///   waits and the seed's holders' weights apply. A rate limit is also the
///   capacity the upload meter knows, and it doesn't follow the uplink's
///   steps, as a user's setting wouldn't.
/// - `STRIMS_SIM_CAL_SEED_LINK=<spec>`, `STRIMS_SIM_CAL_VIEWER_LINK=<spec>`
///   and `STRIMS_SIM_CAL_LINKS="<i>:<spec> ..."` give the nodes `run.sh`'s
///   access link classes (`crate::link`): their framing, slots,
///   buckets (over `STRIMS_SIM_CAL_SEED_BURST`'s, which `tbf` and `fiber`
///   take) and traces, a trace's time 0 at the stream's start. Unset, the
///   uplinks charge IP bytes alone, as before.
pub struct Calibration {
    pub viewers: usize,
    pub seed_up_mbit: u64,
    pub viewer_up_mbit: u64,
    pub seed_delay_ms: u64,
    pub viewer_delay_ms: u64,
    pub secs: u64,
    /// When the viewers boot, in ms after the stream starts (before it if
    /// negative), arriving if after it; `None` with the seed, at the start
    /// of the run.
    pub join_at_ms: Option<i64>,
    /// Each viewer's own join, in place of `join_at_ms`'s: (node `i`, ms
    /// after the stream starts), arriving then.
    pub joins: Vec<(usize, u64)>,
    /// The run's epoch phase in ms, under 1000; `None` draws it.
    pub epoch_ms: Option<u64>,
    /// The Chromium hosts' packet cost on their network thread, in µs;
    /// `None` keeps their stack's.
    pub packet_cost_us: Option<f64>,
    /// The browsers' pages' offer times, ms: Firefox's gathering end and
    /// its post after it, Chromium's lone setting, an entry a host of its
    /// kind in host order, or one for all; empty keeps the stack's.
    pub firefox_gather_ms: Vec<f64>,
    pub firefox_post_ms: Vec<f64>,
    pub chromium_set_ms: Vec<f64>,
    /// The viewers make their links as the testbed's harness does
    /// ([`HostSpec::links_at`](crate::spec::HostSpec::links_at)).
    pub harness_links: bool,
    pub slow_down_kbit: Option<u64>,
    /// `link <a> <b>` lines, node 0 the seed and node `i` viewer `i - 1`;
    /// a star without.
    pub topology: Option<String>,
    /// The seed's uplink from given seconds of stream on, in Mbit/s
    /// (`<s>:<mbit> ...`).
    pub seed_up_steps: Vec<(f64, f64)>,
    /// Every node's rate limit, in percent of its starting uplink.
    pub rate_limit_pct: Option<u64>,
    /// The send buffers' congestion windows ([`SendBuffer::window`]), which
    /// `STRIMS_SIM_TCP=1` also turns on.
    pub window: bool,
    /// The swarm's chunk and munro sizes ([`SwarmSpec::with_chunks`]);
    /// `None` keeps the testbed's.
    pub chunks: Option<(usize, usize)>,
    /// Bulk uploads from the seed's host, from given seconds of stream on
    /// for given seconds (`<s>:<d> ...`), to a sink 30 ms away (the seed's
    /// delay and the testbed sink's). They need the window model
    /// (`STRIMS_SIM_TCP=1`). `STRIMS_SIM_CAL_COMPETE_MS=<ms>:<d> ...`, a
    /// testbed run's recorded starts of the uploads that ran (their iperf3
    /// reports' timestamps, ms of stream), takes the place of
    /// `STRIMS_SIM_CAL_COMPETE`'s wherever it is set: `none` where none ran
    /// (each one the run asked for failed).
    pub compete: Vec<(f64, f64)>,
    /// The seed's and the viewers' uplink buckets, in bytes; `None`, none.
    pub seed_burst: Option<u64>,
    pub viewer_burst: Option<u64>,
    /// The rate those buckets send at, in Mbit/s; `None`, at once.
    pub seed_peak_mbit: Option<f64>,
    pub viewer_peak_mbit: Option<f64>,
    /// The cable profile's jitter on every path (`STRIMS_SIM_CAL_JITTER=1`).
    pub jitter: bool,
    /// Each node's testbed link spec (node 0 the seed), if any.
    pub links: Vec<Option<String>>,
}

impl Calibration {
    /// The settings the knobs `get` reads give (see the [type docs](Calibration)):
    /// the environment for a sim run, a testbed run's cell for its report.
    pub fn from_knobs(get: impl Fn(&str) -> Option<String>) -> Calibration {
        let viewers = value(&get, "STRIMS_SIM_CAL_VIEWERS").unwrap_or(3);
        let seed_up_mbit = value(&get, "STRIMS_SIM_CAL_SEED_UP_MBIT").unwrap_or(10);
        let file = get("STRIMS_SIM_CAL_TOPOLOGY");
        let mesh = get("STRIMS_SIM_CAL_MESH");
        assert!(
            file.is_none() || mesh.is_none(),
            "STRIMS_SIM_CAL_TOPOLOGY or STRIMS_SIM_CAL_MESH, not both"
        );
        let topology = match mesh.as_deref() {
            None => file.map(|path| std::fs::read_to_string(&path).unwrap()),
            Some("full") => Some(full_mesh(viewers)),
            Some("mesh6") => Some(mesh6(viewers).to_owned()),
            Some(m) => panic!("STRIMS_SIM_CAL_MESH={m}: full or mesh6"),
        };
        let steps = pairs(&get, "STRIMS_SIM_CAL_SEED_UP_STEPS");
        let scale = pairs(&get, "STRIMS_SIM_CAL_SEED_UP_SCALE");
        assert!(
            steps.is_empty() || scale.is_empty(),
            "STRIMS_SIM_CAL_SEED_UP_STEPS or STRIMS_SIM_CAL_SEED_UP_SCALE, not both"
        );
        let seed_up_steps = if scale.is_empty() {
            steps
        } else {
            scale
                .into_iter()
                .map(|(at, f)| (at, f * seed_up_mbit as f64))
                .collect()
        };
        let seed_up_steps =
            match get("STRIMS_SIM_CAL_SEED_UP_STEPS_MS").filter(|v| !v.trim().is_empty()) {
                None => seed_up_steps,
                Some(v) => {
                    let ms: Vec<f64> = v
                        .split_whitespace()
                        .map(|m| {
                            m.parse()
                                .ok()
                                .filter(|m: &f64| m.is_finite())
                                .unwrap_or_else(|| {
                                    panic!("STRIMS_SIM_CAL_SEED_UP_STEPS_MS={v}: {m} isn't ms")
                                })
                        })
                        .collect();
                    assert_eq!(
                        ms.len(),
                        seed_up_steps.len(),
                        "STRIMS_SIM_CAL_SEED_UP_STEPS_MS={v}: a time for each of the seed's steps"
                    );
                    seed_up_steps
                        .into_iter()
                        .zip(ms)
                        .map(|((_, mbit), ms)| (ms / 1000.0, mbit))
                        .collect()
                }
            };
        let c = Calibration {
            viewers,
            seed_up_mbit,
            viewer_up_mbit: value(&get, "STRIMS_SIM_CAL_VIEWER_UP_MBIT").unwrap_or(20),
            seed_delay_ms: value(&get, "STRIMS_SIM_CAL_SEED_DELAY_MS").unwrap_or(20),
            viewer_delay_ms: value(&get, "STRIMS_SIM_CAL_VIEWER_DELAY_MS").unwrap_or(10),
            secs: value(&get, "STRIMS_SIM_CAL_SECS").unwrap_or(180),
            join_at_ms: value::<i64>(&get, "STRIMS_SIM_CAL_JOIN_AT_MS").or_else(|| {
                value::<i64>(&get, "STRIMS_SIM_CAL_JOIN_AT")
                    .filter(|&s| s > 0)
                    .map(|s| s * 1000)
            }),
            joins: joins(&get, viewers),
            epoch_ms: value(&get, "STRIMS_SIM_CAL_EPOCH_MS"),
            packet_cost_us: value(&get, "STRIMS_SIM_CAL_PACKET_COST_US"),
            firefox_gather_ms: values(&get, "STRIMS_SIM_CAL_FIREFOX_GATHER_MS"),
            firefox_post_ms: values(&get, "STRIMS_SIM_CAL_FIREFOX_POST_MS"),
            chromium_set_ms: values(&get, "STRIMS_SIM_CAL_CHROMIUM_SET_MS"),
            harness_links: value::<u64>(&get, "STRIMS_SIM_CAL_HARNESS_LINKS")
                .is_none_or(|v| v != 0),
            slow_down_kbit: value(&get, "STRIMS_SIM_CAL_SLOW_DOWN_KBIT"),
            topology,
            seed_up_steps,
            rate_limit_pct: value(&get, "STRIMS_SIM_CAL_RATE_LIMIT_PCT"),
            window: false,
            chunks: None,
            compete: match get("STRIMS_SIM_CAL_COMPETE_MS").as_deref().map(str::trim) {
                Some("none") => Vec::new(),
                Some(_) => pairs(&get, "STRIMS_SIM_CAL_COMPETE_MS")
                    .into_iter()
                    .map(|(at, d)| (at / 1000.0, d))
                    .collect(),
                None => pairs(&get, "STRIMS_SIM_CAL_COMPETE"),
            },
            seed_burst: value(&get, "STRIMS_SIM_CAL_SEED_BURST"),
            viewer_burst: value(&get, "STRIMS_SIM_CAL_VIEWER_BURST"),
            seed_peak_mbit: value(&get, "STRIMS_SIM_CAL_SEED_PEAK_MBIT"),
            viewer_peak_mbit: value(&get, "STRIMS_SIM_CAL_VIEWER_PEAK_MBIT"),
            jitter: value::<u64>(&get, "STRIMS_SIM_CAL_JITTER").unwrap_or(0) == 1,
            links: {
                let each = get("STRIMS_SIM_CAL_LINKS").unwrap_or_default();
                (0..=viewers)
                    .map(|i| {
                        each.split_whitespace()
                            .find_map(|l| {
                                let (n, spec) = l.split_once(':')?;
                                (n.parse::<usize>().ok()? == i).then(|| spec.to_owned())
                            })
                            .or_else(|| {
                                get(if i == 0 {
                                    "STRIMS_SIM_CAL_SEED_LINK"
                                } else {
                                    "STRIMS_SIM_CAL_VIEWER_LINK"
                                })
                            })
                    })
                    .collect()
            },
        };
        if let Some(ms) = c.epoch_ms {
            assert!(
                ms < 1000,
                "STRIMS_SIM_CAL_EPOCH_MS={ms}: a ms of the second, 0 to 999"
            );
        }
        if let Some(us) = c.packet_cost_us {
            assert!(
                us.is_finite() && us > 0.0 && us < 1000.0,
                "STRIMS_SIM_CAL_PACKET_COST_US={us}: µs a packet, over 0 and under 1000"
            );
        }
        for (knob, ms) in [
            ("FIREFOX_GATHER", &c.firefox_gather_ms),
            ("FIREFOX_POST", &c.firefox_post_ms),
            ("CHROMIUM_SET", &c.chromium_set_ms),
        ] {
            for &ms in ms {
                assert!(
                    ms.is_finite() && (0.0..10_000.0).contains(&ms),
                    "STRIMS_SIM_CAL_{knob}_MS={ms}: ms, at least 0 and under 10000"
                );
            }
        }
        // A peak is the rate a bucket sends at: without the bucket it would
        // shape nothing.
        for (who, peak, burst) in [
            ("SEED", c.seed_peak_mbit, c.seed_burst),
            ("VIEWER", c.viewer_peak_mbit, c.viewer_burst),
        ] {
            assert!(
                peak.is_none() || burst.is_some(),
                "STRIMS_SIM_CAL_{who}_PEAK_MBIT needs STRIMS_SIM_CAL_{who}_BURST, the bucket it sends"
            );
        }
        c
    }
}

/// `link` lines between every pair of a seed and `viewers` viewers.
pub fn full_mesh(viewers: usize) -> String {
    let mut t = String::new();
    for a in 0..=viewers {
        for b in a + 1..=viewers {
            t.push_str(&format!("link {a} {b}\n"));
        }
    }
    t
}

/// The farm's testbed mesh for `viewers` viewers.
pub fn mesh6(viewers: usize) -> &'static str {
    match viewers {
        3 => include_str!("../../../hack/sim-farm/matrices/topologies/mesh6-v3"),
        7 => include_str!("../../../hack/sim-farm/matrices/topologies/mesh6-v7"),
        15 => include_str!("../../../hack/sim-farm/matrices/topologies/mesh6-v15"),
        31 => include_str!("../../../hack/sim-farm/matrices/topologies/mesh6-v31"),
        63 => include_str!("../../../hack/sim-farm/matrices/topologies/mesh6-v63"),
        n => panic!("no mesh6 topology for {n} viewers: 3, 7, 15, 31 or 63"),
    }
}

/// `STRIMS_SIM_CAL_JOINS`'s `<i>:<s> ...`: each viewer `i` of `viewers`
/// (from 1) and its join's ms, `s` seconds rounded as run.sh's
/// `--join-at` rounds them.
fn joins(get: &impl Fn(&str) -> Option<String>, viewers: usize) -> Vec<(usize, u64)> {
    let mut out: Vec<(usize, u64)> = Vec::new();
    for pair in get("STRIMS_SIM_CAL_JOINS")
        .unwrap_or_default()
        .split_whitespace()
    {
        let parsed = pair.split_once(':').and_then(|(i, s)| {
            let (i, s): (usize, f64) = (i.parse().ok()?, s.parse().ok()?);
            (s.is_finite() && s >= 0.0).then(|| (i, (s * 1000.0).round() as u64))
        });
        match parsed {
            Some((i, ms)) if (1..=viewers).contains(&i) && out.iter().all(|&(x, _)| x != i) => {
                out.push((i, ms));
            }
            _ => panic!(
                "STRIMS_SIM_CAL_JOINS: {pair} isn't <viewer 1..={viewers}>:<seconds, 0 or more>, once a viewer"
            ),
        }
    }
    out
}

/// `<a>:<b> ...` from knob `name`.
fn pairs(get: &impl Fn(&str) -> Option<String>, name: &str) -> Vec<(f64, f64)> {
    get(name)
        .unwrap_or_default()
        .split_whitespace()
        .map(|pair| {
            let (a, b) = pair.split_once(':').unwrap();
            (a.parse().unwrap(), b.parse().unwrap())
        })
        .collect()
}

/// A seed and `viewers` viewers, each linked to the seed alone (a star:
/// whatever the seed doesn't push its viewers pull from it, so its load is
/// the viewers times the stream rate), or as the topology says. No jitter
/// or loss; fixed delays; no upload limits unless
/// `STRIMS_SIM_CAL_RATE_LIMIT_PCT` sets them; send buffers; as browsers,
/// the signalling relay on the hosts' bridge
/// ([`Scenario::signal_on_bridge`]), as the testbed's.
pub fn calibration_star(c: &Calibration) -> Scenario {
    let mut s = Scenario::new("calibration_star");
    let link =
        |i: usize| {
            c.links.get(i).cloned().flatten().map(|spec| {
                crate::link::TestbedLink::parse(&spec).unwrap_or_else(|e| panic!("{e}"))
            })
        };
    // The highest rate a node's uplink runs at, which a peak is over: the
    // seed's steps' too.
    let top = |i: usize, mbit: u64| {
        let start = mbit * 125_000;
        if i > 0 {
            return start;
        }
        c.seed_up_steps
            .iter()
            .map(|&(_, m)| (m * 125_000.0) as u64)
            .fold(start, u64::max)
    };
    let site = |i: usize, mbit: u64, burst: Option<u64>, peak: Option<f64>| {
        let bucket = burst.map(|bytes| Bucket {
            bytes,
            peak: peak.map(|m| (m * 125_000.0) as u64),
        });
        let top = top(i, mbit);
        let st = Site::new(City::Chicago, Access::Cable)
            .with_up(mbit * 125_000)
            .with_bucket(bucket);
        match link(i) {
            Some(l) => l.site(st, bucket, top).unwrap_or_else(|e| panic!("{e}")),
            None => {
                if let Some(p) = bucket.and_then(|b| b.peak) {
                    strims_wan::testbed::check_peak(p, top)
                        .unwrap_or_else(|e| panic!("node {i}: {e}"));
                }
                st
            }
        }
    };
    let seed_host = s.host_at(
        "seed",
        site(0, c.seed_up_mbit, c.seed_burst, c.seed_peak_mbit),
    );
    let vs: Vec<HostIx> = (0..c.viewers)
        .map(|i| {
            let mut st = site(i + 1, c.viewer_up_mbit, c.viewer_burst, c.viewer_peak_mbit);
            if i == 0
                && let Some(k) = c.slow_down_kbit
            {
                st = st.with_down(k * 125);
            }
            s.host_at(&format!("v{i}"), st)
        })
        .collect();
    for h in &mut s.hosts {
        h.uplink = c
            .rate_limit_pct
            .and_then(|pct| Some(h.site?.profile.up * pct / 100));
    }
    for (i, h) in std::iter::once(seed_host)
        .chain(vs.iter().copied())
        .enumerate()
    {
        s.hosts[h].link_spec = c.links.get(i).cloned().flatten();
        if let Some(l) = link(i) {
            s.hosts[h].up_capacity = l
                .capacity(Duration::from_secs(5))
                .unwrap_or_else(|e| panic!("{e}"));
        }
    }
    s.hosts[seed_host].up_steps = c
        .seed_up_steps
        .iter()
        .map(|&(at, mbit)| (Duration::from_secs_f64(5.0 + at), (mbit * 125_000.0) as u64))
        .collect();
    s.hosts[seed_host].competing = c
        .compete
        .iter()
        .map(|&(at, secs)| Competing {
            start: Duration::from_secs_f64(5.0 + at),
            stop: Duration::from_secs_f64(5.0 + at + secs),
            rtt: Duration::from_millis(c.seed_delay_ms + 10),
        })
        .collect();
    s.wan = Some(strims_wan::Options {
        jitter: c.jitter,
        loss: false,
    });
    s.send_buffer = Some(SendBuffer {
        uplink_queue: UPLINK_QUEUE,
        window: c.window,
    });
    let all: Vec<HostIx> = std::iter::once(seed_host)
        .chain(vs.iter().copied())
        .collect();
    match &c.topology {
        Some(t) => {
            for line in t.lines() {
                if let ["link", a, b] = line.split_whitespace().collect::<Vec<_>>()[..] {
                    let (a, b): (usize, usize) = (a.parse().unwrap(), b.parse().unwrap());
                    s.link(all[a.max(b)], all[a.min(b)], Transport::Tcp);
                }
            }
        }
        None => {
            for &v in &vs {
                s.link(v, seed_host, Transport::Tcp);
            }
        }
    }
    if vs.len() > 1 {
        s.at(
            Duration::ZERO,
            Fault::Latency(
                vs.clone(),
                vs.clone(),
                Duration::from_millis(c.viewer_delay_ms),
            ),
        );
    }
    // Each direction takes its sender's delay, as run.sh's netem holds
    // what a node sends.
    s.at(
        Duration::ZERO,
        Fault::LatencyFrom(
            vec![seed_host],
            vs.clone(),
            Duration::from_millis(c.seed_delay_ms),
        ),
    );
    s.at(
        Duration::ZERO,
        Fault::LatencyFrom(
            vs.clone(),
            vec![seed_host],
            Duration::from_millis(c.viewer_delay_ms),
        ),
    );
    // Each viewer's join, where the testbed's harness makes its links: its
    // JOINS time, arriving then, else the stream's start or JOIN_AT after
    // it, arriving if after it; at or before it, the harness's viewer
    // boots with the run, and one without booted by a restart of a host
    // down from the run's start.
    for (i, &v) in vs.iter().enumerate() {
        let own = c
            .joins
            .iter()
            .find(|&&(x, _)| x == i + 1)
            .map(|&(_, ms)| ms);
        let ms = own.map(|ms| ms as i64).or(c.join_at_ms);
        let join = Duration::from_millis(u64::try_from(5000 + ms.unwrap_or(0)).unwrap_or(0).max(1));
        if c.harness_links {
            s.hosts[v].links_at = Some(join);
        }
        match ms {
            Some(ms) if ms > 0 || own.is_some() => s.hosts[v].arrive_at = Some(join),
            Some(_) if !c.harness_links => {
                s.at(Duration::ZERO, Fault::Crash(v));
                s.at(join, Fault::Restart(v));
            }
            _ => {}
        }
    }
    let mut swarm = SwarmSpec::live(seed_host, vs, WAN_RATE, secs(5), secs(5 + c.secs))
        .with_push_mode(PushMode::Hybrid)
        .with_live_window(WAN_WINDOW);
    if let Some((chunk, munro)) = c.chunks {
        swarm = swarm.with_chunks(chunk, munro).unwrap();
    }
    s.swarm(swarm);
    // The heap grows with the swarm: a 32-node full mesh holds about
    // 450 MiB. Past the bound, the trace names the heap's bytes, which
    // vary between processes, and the farm's reruns would disagree.
    s.invariants = Invariants {
        max_heap: Some((256 + 24 * c.viewers as i64) * MIB),
        ..Invariants::for_window(WAN_WINDOW)
    };
    s.duration = secs(5 + c.secs + 10);
    s.transfer_samples = true;
    s.signal_on_bridge = true;
    s.epoch_phase = c.epoch_ms.map(Duration::from_millis);
    s.packet_cost = c
        .packet_cost_us
        .map(|us| Duration::from_nanos((us * 1000.0).round() as u64));
    let ms = |ms: &[f64]| {
        ms.iter()
            .map(|ms| Duration::from_micros((ms * 1000.0).round() as u64))
            .collect()
    };
    s.page_timing = PageTiming {
        firefox_gather: ms(&c.firefox_gather_ms),
        firefox_post: ms(&c.firefox_post_ms),
        chromium_set: ms(&c.chromium_set_ms),
    };
    striped(s, STRIPES, ADAPTIVE)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Viewers that join after the stream starts (5 s into the run) arrive
    /// then, with no crash or restart; at or before it, each boots with the
    /// run, or without the harness's links is down from the run's start and
    /// restarts at its join. The seed is there from the start either way.
    /// Every viewer makes its links at its join, as the testbed's harness
    /// does: the stream's start without a join.
    #[test]
    fn viewers_joining_after_the_start_arrive_and_the_rest_link_at_their_join() {
        let star_of = |ms: i64, harness: bool| {
            calibration_star(&Calibration::from_knobs(|k| match k {
                "STRIMS_SIM_CAL_JOIN_AT_MS" => Some(ms.to_string()),
                "STRIMS_SIM_CAL_HARNESS_LINKS" if !harness => Some("0".to_owned()),
                _ => None,
            }))
        };
        let star = |ms: i64| star_of(ms, true);
        let joins = |s: &Scenario| -> Vec<(Duration, String)> {
            let mut v: Vec<(Duration, String)> = s
                .faults
                .iter()
                .filter(|f| matches!(f.fault, Fault::Crash(_) | Fault::Restart(_)))
                .map(|f| (f.at, format!("{:?}", f.fault)))
                .collect();
            v.sort();
            v
        };
        let ms = Duration::from_millis;
        for (join_ms, up) in [(-2000, ms(3000)), (0, ms(5000))] {
            let s = star(join_ms);
            assert_eq!(s.hosts.len(), 4, "JOIN_AT_MS={join_ms}");
            let links: Vec<Option<Duration>> = s.hosts.iter().map(|h| h.links_at).collect();
            assert_eq!(links, [None, Some(up), Some(up), Some(up)]);
            assert_eq!(joins(&s), [], "JOIN_AT_MS={join_ms}");
            let s = star_of(join_ms, false);
            assert!(s.hosts.iter().all(|h| h.links_at.is_none()));
            assert!(
                s.hosts.iter().all(|h| h.arrive_at.is_none()),
                "JOIN_AT_MS={join_ms}"
            );
            let mut want: Vec<(Duration, String)> = [1, 2, 3]
                .into_iter()
                .flat_map(|v: HostIx| {
                    [
                        (Duration::ZERO, format!("{:?}", Fault::Crash(v))),
                        (up, format!("{:?}", Fault::Restart(v))),
                    ]
                })
                .collect();
            want.sort();
            assert_eq!(joins(&s), want, "JOIN_AT_MS={join_ms}");
            // Its viewers there from the start: the row's measures are the
            // whole run's.
            assert!(!s.profiled(), "JOIN_AT_MS={join_ms}");
        }
        // Arriving viewers: the row's measures follow their presence, and
        // it writes their joins.
        assert!(star(2500).profiled());
        let s = star(2500);
        assert_eq!(joins(&s), []);
        let arrivals: Vec<Option<Duration>> = s.hosts.iter().map(|h| h.arrive_at).collect();
        assert_eq!(
            arrivals,
            [None, Some(ms(7500)), Some(ms(7500)), Some(ms(7500))]
        );
        let links: Vec<Option<Duration>> = s.hosts.iter().map(|h| h.links_at).collect();
        assert_eq!(links, arrivals);
        let at_start = calibration_star(&Calibration::from_knobs(|_| None));
        let links: Vec<Option<Duration>> = at_start.hosts.iter().map(|h| h.links_at).collect();
        assert_eq!(
            links,
            [None, Some(ms(5000)), Some(ms(5000)), Some(ms(5000))]
        );
        let off = calibration_star(&Calibration::from_knobs(|k| {
            (k == "STRIMS_SIM_CAL_HARNESS_LINKS").then(|| "0".to_owned())
        }));
        assert!(off.hosts.iter().all(|h| h.links_at.is_none()));
    }

    /// A viewer of JOINS arrives at its own time, to the ms, whatever the
    /// time (the stream's start too), and makes its links then; the others
    /// join at JOIN_AT's.
    #[test]
    fn a_viewer_of_joins_arrives_at_its_own_time_to_the_ms() {
        let star = |joins: &str, join_at: Option<&str>| {
            calibration_star(&Calibration::from_knobs(|k| match k {
                "STRIMS_SIM_CAL_JOINS" => Some(joins.to_owned()),
                "STRIMS_SIM_CAL_JOIN_AT_MS" => join_at.map(str::to_owned),
                _ => None,
            }))
        };
        let ms = |m: u64| Some(Duration::from_millis(m));
        let s = star("1:2.0004 3:0.25", None);
        let arrivals: Vec<Option<Duration>> = s.hosts.iter().map(|h| h.arrive_at).collect();
        assert_eq!(arrivals, [None, ms(7000), None, ms(5250)]);
        let links: Vec<Option<Duration>> = s.hosts.iter().map(|h| h.links_at).collect();
        assert_eq!(links, [None, ms(7000), ms(5000), ms(5250)]);
        let s = star("2:0", Some("1500"));
        let arrivals: Vec<Option<Duration>> = s.hosts.iter().map(|h| h.arrive_at).collect();
        assert_eq!(arrivals, [None, ms(6500), ms(5000), ms(6500)]);
        for bad in ["0:2", "4:2", "1:-1", "1:2 1:3", "1"] {
            let refused = std::panic::catch_unwind(|| star(bad, None)).is_err();
            assert!(refused, "JOINS={bad}");
        }
    }

    /// EPOCH_MS sets the run's epoch phase, to the ms of its second; unset,
    /// the run draws its own.
    #[test]
    fn epoch_ms_sets_the_runs_epoch_phase() {
        let star = |ms: Option<&str>| {
            calibration_star(&Calibration::from_knobs(|k| {
                (k == "STRIMS_SIM_CAL_EPOCH_MS")
                    .then_some(ms)
                    .flatten()
                    .map(str::to_owned)
            }))
        };
        assert_eq!(star(None).epoch_phase, None);
        for (v, ms) in [("0", 0), ("902", 902), ("999", 999)] {
            assert_eq!(star(Some(v)).epoch_phase, Some(Duration::from_millis(ms)));
        }
        for bad in ["1000", "-1", "0.5"] {
            let refused = std::panic::catch_unwind(|| star(Some(bad))).is_err();
            assert!(refused, "EPOCH_MS={bad}");
        }
    }

    /// SEED_UP_STEPS_MS steps the seed's uplink at a run's measured times,
    /// its schedule's rates; unset or empty, at the schedule's times, and
    /// without steps, none. A time short of a step is refused.
    #[test]
    fn seed_up_steps_ms_moves_the_seeds_steps_to_their_measured_times() {
        let star = |knobs: &[(&str, &str)]| {
            let c = Calibration::from_knobs(|k| {
                knobs
                    .iter()
                    .find(|&&(name, _)| name == k)
                    .map(|&(_, v)| v.to_owned())
            });
            calibration_star(&c).hosts[0].up_steps.clone()
        };
        let scale = ("STRIMS_SIM_CAL_SEED_UP_SCALE", "30:0.5 60:1");
        let ms = |v| ("STRIMS_SIM_CAL_SEED_UP_STEPS_MS", v);
        let at = |ms: u64| Duration::from_millis(5000 + ms);
        let nominal = vec![(at(30_000), 625_000), (at(60_000), 1_250_000)];
        assert_eq!(star(&[scale]), nominal);
        assert_eq!(star(&[scale, ms(" ")]), nominal);
        let measured = star(&[scale, ms("30037 60034.5")]);
        assert_eq!(
            measured.iter().map(|s| s.1).collect::<Vec<_>>(),
            [625_000, 1_250_000]
        );
        for (got, want) in measured
            .iter()
            .zip([at(30_037), at(60_034) + Duration::from_micros(500)])
        {
            assert!(
                got.0.abs_diff(want) < Duration::from_micros(1),
                "{got:?} {want:?}"
            );
        }
        assert!(star(&[]).is_empty());
        assert!(star(&[ms("")]).is_empty());
        for bad in [
            &[scale, ms("30037")][..],
            &[ms("30037")],
            &[scale, ms("a 1")],
        ] {
            let refused = std::panic::catch_unwind(|| star(bad)).is_err();
            assert!(refused, "{bad:?}");
        }
    }

    /// PACKET_COST_US gives the Chromium hosts' stacks that cost a packet
    /// on their network thread, to the ns, once they link as browsers do;
    /// Firefox's, str0m's and a host over UDP keep theirs, and unset every
    /// stack keeps its own.
    #[test]
    fn packet_cost_us_reaches_the_chromium_hosts_stacks_alone() {
        use crate::SctpProfile;
        let star = |us: Option<&str>| {
            calibration_star(&Calibration::from_knobs(|k| {
                (k == "STRIMS_SIM_CAL_PACKET_COST_US")
                    .then_some(us)
                    .flatten()
                    .map(str::to_owned)
            }))
        };
        let stacks = [
            Some(SctpProfile::CHROMIUM),
            Some(SctpProfile::FIREFOX),
            Some(SctpProfile::STR0M),
            None,
        ];
        let linked = |us: Option<&str>| {
            let mut s = star(us);
            let relay = s.over_webrtc(&stacks);
            s.hosts[..relay]
                .iter()
                .map(|h| h.rtc_stack)
                .collect::<Vec<_>>()
        };
        assert_eq!(star(None).packet_cost, None);
        assert_eq!(
            star(Some("5.4")).packet_cost,
            Some(Duration::from_nanos(5400))
        );
        let before = linked(None);
        let after = linked(Some("5.4"));
        assert_eq!(before.len(), after.len());
        for (h, (b, a)) in before.iter().zip(&after).enumerate() {
            match h % stacks.len() {
                0 => {
                    assert_eq!(
                        b.unwrap().packet_cost,
                        Some(strims_base::time::Duration::from_micros(12))
                    );
                    assert_eq!(
                        a.unwrap().packet_cost,
                        Some(strims_base::time::Duration::from_nanos(5400))
                    );
                    let rest = SctpProfile {
                        packet_cost: None,
                        ..a.unwrap()
                    };
                    assert_eq!(
                        rest,
                        SctpProfile {
                            packet_cost: None,
                            ..b.unwrap()
                        }
                    );
                }
                _ => assert_eq!(a, b, "host {h}"),
            }
        }
        for bad in ["0", "-1", "nan", "1000"] {
            let refused = std::panic::catch_unwind(|| star(Some(bad))).is_err();
            assert!(refused, "PACKET_COST_US={bad}");
        }
    }
}
