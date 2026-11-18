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
    Competing, Fault, HostIx, Invariants, Scenario, SendBuffer, SwarmSpec, Transport,
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
/// - Each viewer makes its links one at a time from the stream's start, as
///   the testbed's harness does; `STRIMS_SIM_CAL_SERIAL_LINKS=0` links them
///   all at once from the start of the run.
/// - `STRIMS_SIM_CAL_JOIN_AT_MS=<ms>` boots the viewers that long after the
///   stream starts (before it if negative); `STRIMS_SIM_CAL_JOIN_AT=<s>`,
///   the testbed's `JOIN_AT`, in seconds. Viewers that join after the
///   stream starts arrive then
///   ([`HostSpec::arrive_at`](crate::spec::HostSpec::arrive_at)) and read
///   from the live edge, as the testbed's late viewers do; at or before
///   it, each boots by a restart of a host down from the run's start, and
///   reads as the swarm's options say, as the testbed's viewers at the
///   start do.
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
    /// The viewers make their links one at a time from the stream's start,
    /// as the testbed's harness does ([`HostSpec::serial_links`](crate::spec::HostSpec::serial_links)).
    pub serial_links: bool,
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
            serial_links: value::<u64>(&get, "STRIMS_SIM_CAL_SERIAL_LINKS").is_none_or(|v| v != 0),
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
/// `STRIMS_SIM_CAL_RATE_LIMIT_PCT` sets them; send buffers.
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
    if c.serial_links {
        for &v in &vs {
            s.hosts[v].serial_links = Some(secs(5));
        }
    }
    if let Some(ms) = c.join_at_ms {
        let at = Duration::from_millis(u64::try_from(5000 + ms).unwrap_or(0).max(1));
        for &v in &vs {
            if ms > 0 {
                s.hosts[v].arrive_at = Some(at);
            } else {
                s.at(Duration::ZERO, Fault::Crash(v));
                s.at(at, Fault::Restart(v));
            }
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
    striped(s, STRIPES, ADAPTIVE)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Viewers that join after the stream starts (5 s into the run) arrive
    /// then, with no crash or restart; at or before it, each is down from
    /// the run's start and restarts at its join. The seed is there from
    /// the start either way.
    #[test]
    fn viewers_joining_after_the_start_arrive_and_the_rest_restart() {
        let star = |ms: i64| {
            calibration_star(&Calibration::from_knobs(|k| {
                (k == "STRIMS_SIM_CAL_JOIN_AT_MS").then(|| ms.to_string())
            }))
        };
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
    }
}
