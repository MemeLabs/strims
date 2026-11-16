// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A testbed run (`hack/testbed/run.sh`'s out dir) as a run's report, so
//! it writes the tables a sim run writes ([`Report::write_tables`]), by
//! the same code: one definition of every column on both sides.
//!
//! - **The cell.** A run's settings map to the knobs of its twin, strims-sim's
//!   `calibration_star` ([`knobs_of`]), and the report's scenario is the
//!   twin's ([`calibration_star`]), with what the testbed measured in place
//!   of what the twin schedules: the seed's uplink steps (steps.log), the
//!   competing uploads' starts (compete.log) and each viewer's join
//!   (`JOINS`). Time is the twin's: the stream starts 5 s into the run.
//! - **The nodes.** Each node's lines (node-<i>.jsonl) carry its records
//!   (`strims_testbed::node`'s `rec`, made by the functions strims-sim's
//!   own nodes' samples go through, [`strims_testbed::record`]): its meter
//!   samples and its adaptive seed's changes, its transfers, its stats bins,
//!   its parent changes, and every frame it read, which each viewer's
//!   [`ViewerStats`] takes as a simulated viewer's does
//!   ([`ViewerStats::record`]).
//! - **The network.** The sim's network model counts each uplink's wire
//!   bytes as each packet's IP length (`strims_wan::wire`: no link
//!   framing), its pure acknowledgements apart (the WAN columns,
//!   [`crate::tables::UP_COUNTERS`]). A testbed run counts the same from
//!   each node's capture of its egress (`PCAP=1`: pcap-<i>.pcap, after the
//!   queue), and its queue drops from its tbf's counter (tc-<i>.log). What
//!   a capture can't tell (retransmits, the window's cuts and timeouts, a
//!   data channel's SACKs inside DTLS) is left empty. The uplink's rate is
//!   its tbf's, as configured: tbf charges each packet its IP length and its
//!   14-byte Ethernet header, which the sim's bytes leave out. The
//!   competing upload's wire bytes are its packets' to the sink, from the
//!   seed's capture.
//!
//! A run's lines name the seed's stream start (`t_ms - rel_ms`), which every
//! container's clock shares.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;
use sha2::{Digest, Sha256};
use strims_base::stats::WaitHistogram;
use strims_ppspp::ParentWhy;
use strims_testbed::record::{BIN_MS, MeterSample, NODE_BIN_COUNTERS, SeedSample};
use strims_testbed::stream::{FrameInfo, Parsed};
use strims_testbed::transfer::{PeerTransfer, TransferSample};

use crate::calibration::{self, Calibration, WAN_RATE, calibration_star};
use crate::report::{
    ACK_WHYS, AckWaits, HostReport, NodeBinRecord, ParentRecord, Report, ViewerReport,
    WanHostReport, WanReport,
};
use crate::spec::{Competing, Fault, HostIx, Scenario};
use crate::stream::{SeqSet, ViewerStats};
use crate::wan::{SctpProfile, UP_COUNTERS};

/// The twin's stream starts this far into its run.
const LEAD: Duration = Duration::from_secs(5);
/// The testbed's network, node `i` at `.10 + i`, the competing upload's
/// sink at `.250`.
const SUBNET: [u8; 3] = [10, 77, 0];
const SINK: u8 = 250;
/// The test whose scenario is a testbed run's twin.
pub const TEST: &str = "calibration_star";
/// The sim name a testbed run's records go under.
pub const SIM: &str = "testbed";
/// The settings that only record or capture: no knob of the twin's.
const RECORDING: [&str; 10] = [
    "CAPTURE",
    "PCAP",
    "PCAP_SNAPLEN",
    "SS_EVERY",
    "CPU_PROFILE",
    "RECORD",
    "NODE_BIN",
    "BROWSER_BIN",
    "WEB_BUNDLE",
    "BROWSER_ARGS",
];

/// The settings [`knobs_of`] maps to the twin's knobs.
const TWIN_SETTINGS: [&str; 23] = [
    "VIEWERS",
    "SEED_UP_MBIT",
    "VIEWER_UP_MBIT",
    "SEED_BURST",
    "VIEWER_BURST",
    "SEED_DELAY_MS",
    "VIEWER_DELAY_MS",
    "SECS",
    "JOIN_AT",
    "JOINS",
    "SEED_UP_STEPS",
    "COMPETE",
    "SLOW_DOWN_KBIT",
    "TOPOLOGY",
    "RATE",
    "NODE_ARGS",
    "KINDS",
    "BROWSER_LINKS",
    "RELAY",
    "QDISC",
    "SEED_LINK",
    "VIEWER_LINK",
    "LINKS",
];

/// The qdisc tree of the runs from before run.sh named its tree (`QDISC`):
/// a netem root delaying what a tbf child then shaped.
const OLD_QDISC: &str = "netem>tbf";

/// A testbed run read from its out dir.
pub struct Run {
    pub dir: PathBuf,
    /// run.sh's settings (its `settings` file).
    pub settings: BTreeMap<String, String>,
    /// The twin's knobs ([`knobs_of`]) and any the caller adds.
    pub knobs: BTreeMap<String, String>,
    /// The stream's start, in Unix ms.
    pub start_ms: u64,
    /// Each node's lines, node 0 the seed.
    pub nodes: Vec<Vec<Value>>,
}

/// `run.sh`'s `settings` file: `KEY=VALUE` lines.
pub fn settings_of(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect()
}

/// The `link a b` lines of a topology, each pair low first, sorted.
fn links_of(text: &str) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = text
        .lines()
        .filter_map(|l| match l.split_whitespace().collect::<Vec<_>>()[..] {
            ["link", a, b] => Some((a.parse().ok()?, b.parse().ok()?)),
            _ => None,
        })
        .map(|(a, b): (usize, usize)| (a.min(b), a.max(b)))
        .collect();
    out.sort_unstable();
    out
}

/// The `--flag value` pairs of a node's extra arguments.
fn flags_of(args: &str) -> Result<BTreeMap<String, String>, String> {
    let words: Vec<&str> = args.split_whitespace().collect();
    let mut out = BTreeMap::new();
    for pair in words.chunks(2) {
        match pair {
            [f, v] if f.starts_with("--") => {
                out.insert((*f).to_owned(), (*v).to_owned());
            }
            _ => return Err(format!("node arguments {args:?}: --flag value pairs")),
        }
    }
    Ok(out)
}

/// A number as a knob writes it: `0.5`, `30`.
fn num(x: f64) -> String {
    format!("{x}")
}

/// The knobs of a testbed run's twin, from its `settings` and its
/// `topology` file: strims-sim's `STRIMS_SIM_*` settings that make
/// `calibration_star` the run's configuration.
///
/// The window model (`STRIMS_SIM_TCP=1`), honest viewers
/// (`STRIMS_SIM_HOSTILE=off`) and one clock (`STRIMS_SIM_CLOCK_SKEW=off`)
/// always; the viewers, the seed's uplink and the stream's length always;
/// the other links' settings where they aren't the twin's defaults; the
/// seed's steps as factors of its uplink (`STRIMS_SIM_CAL_SEED_UP_SCALE`);
/// both tbf buckets (run.sh's 3028 bytes where unset); a full mesh or the
/// farm's 6-link mesh by name, another topology by its file; each node's
/// access link spec (its `links` file's, `STRIMS_SIM_CAL_SEED_LINK`,
/// `STRIMS_SIM_CAL_VIEWER_LINK` where the viewers' are one, else
/// `STRIMS_SIM_CAL_LINKS`), so a run pairs with the twin of its own links; the chunks
/// and munros the nodes take; and with browsers, each node's stack
/// (`STRIMS_SIM_RTC`, native nodes str0m), its shortest repeating cycle.
/// A setting the twin has no knob for (a seed's loss, a bucket's peak
/// rate, another queue) goes in as `TESTBED_<SETTING>`, so the run pairs
/// with no cell, and so does the qdisc tree that shaped it (`QDISC`,
/// `TESTBED_QDISC`; runs from before run.sh named it, its netem root over
/// a tbf). A stream the twin can't stand for (media, another rate),
/// browsers on WebSockets to native nodes, or no relay is an error.
pub fn knobs_of(
    settings: &BTreeMap<String, String>,
    topology: &str,
    links: Option<&str>,
    dir: &Path,
) -> Result<BTreeMap<String, String>, String> {
    let get = |k: &str| settings.get(k).map(String::as_str);
    let mut out = BTreeMap::new();
    let mut put = |k: &str, v: String| {
        out.insert(k.to_owned(), v);
    };
    for k in ["MEDIA", "UNITS_CPS"] {
        if get(k).is_some_and(|v| !v.is_empty()) {
            return Err(format!("{k}: the twin streams frames"));
        }
    }
    if get("RELAY") == Some("0") {
        return Err("RELAY=0: a bench run, which no twin has".to_owned());
    }
    if let Some(r) = get("RATE")
        && r.parse::<u64>().ok() != Some(WAN_RATE)
    {
        return Err(format!("RATE={r}: the twin streams {WAN_RATE}"));
    }
    put("STRIMS_SIM_TCP", "1".to_owned());
    put("STRIMS_SIM_HOSTILE", "off".to_owned());
    put("STRIMS_SIM_CLOCK_SKEW", "off".to_owned());
    let viewers: usize = get("VIEWERS")
        .unwrap_or("3")
        .parse()
        .map_err(|e| format!("VIEWERS: {e}"))?;
    let seed_up: f64 = get("SEED_UP_MBIT")
        .unwrap_or("10")
        .parse()
        .map_err(|e| format!("SEED_UP_MBIT: {e}"))?;
    put("STRIMS_SIM_CAL_VIEWERS", viewers.to_string());
    put("STRIMS_SIM_CAL_SEED_UP_MBIT", num(seed_up));
    put(
        "STRIMS_SIM_CAL_SECS",
        get("SECS").unwrap_or("180").to_owned(),
    );
    for (k, default) in [
        ("VIEWER_UP_MBIT", "20"),
        ("SEED_DELAY_MS", "20"),
        ("VIEWER_DELAY_MS", "10"),
        ("JOIN_AT", "0"),
        ("SLOW_DOWN_KBIT", ""),
    ] {
        if let Some(v) = get(k).filter(|&v| v != default) {
            put(&format!("STRIMS_SIM_CAL_{k}"), v.to_owned());
        }
    }
    if let Some(steps) = get("SEED_UP_STEPS").filter(|s| !s.trim().is_empty()) {
        let mut scale = Vec::new();
        for step in steps.split_whitespace() {
            let (at, mbit) = step
                .split_once(':')
                .ok_or(format!("SEED_UP_STEPS: {step} isn't <s>:<mbit>"))?;
            let at: f64 = at
                .parse()
                .map_err(|e| format!("SEED_UP_STEPS {step}: {e}"))?;
            let mbit: f64 = mbit
                .parse()
                .map_err(|e| format!("SEED_UP_STEPS {step}: {e}"))?;
            scale.push(format!("{}:{}", num(at), num(mbit / seed_up)));
        }
        put("STRIMS_SIM_CAL_SEED_UP_SCALE", scale.join(" "));
    }
    if let Some(c) = get("COMPETE").filter(|s| !s.trim().is_empty()) {
        put("STRIMS_SIM_CAL_COMPETE", c.to_owned());
    }
    // The access links' specs, each node's as run.sh's `links` file names
    // it (`node <i> <spec> <up_mbit> <delay_ms> <queue_ms>`), else as
    // SEED_LINK, VIEWER_LINK and LINKS give them.
    match links {
        Some(text) => {
            let mut specs: BTreeMap<usize, &str> = text
                .lines()
                .filter_map(|l| match l.split_whitespace().collect::<Vec<_>>()[..] {
                    ["node", i, spec, ..] => Some((i.parse().ok()?, spec)),
                    _ => None,
                })
                .collect();
            // A link the twin's model refuses (crate::link) has no twin:
            // the run keeps it as TESTBED_LINKS, its twin knobs none.
            let refused: Vec<String> = specs
                .iter()
                .filter(|(_, spec)| crate::link::TestbedLink::parse(spec).is_err())
                .map(|(i, spec)| format!("{i}:{spec}"))
                .collect();
            if !refused.is_empty() {
                put("TESTBED_LINKS", refused.join(" "));
                specs.retain(|_, spec| crate::link::TestbedLink::parse(spec).is_ok());
            }
            if let Some(seed) = specs.get(&0) {
                put("STRIMS_SIM_CAL_SEED_LINK", (*seed).to_owned());
            }
            let each: Vec<Option<&&str>> = (1..=viewers).map(|i| specs.get(&i)).collect();
            match each.first() {
                Some(Some(v)) if each.iter().all(|x| x == &Some(*v)) => {
                    put("STRIMS_SIM_CAL_VIEWER_LINK", (**v).to_owned());
                }
                _ => {
                    let list: Vec<String> = (1..=viewers)
                        .filter_map(|i| Some(format!("{i}:{}", specs.get(&i)?)))
                        .collect();
                    if !list.is_empty() {
                        put("STRIMS_SIM_CAL_LINKS", list.join(" "));
                    }
                }
            }
        }
        None => {
            for k in ["SEED_LINK", "VIEWER_LINK", "LINKS"] {
                if let Some(v) = get(k).filter(|s| !s.trim().is_empty()) {
                    put(&format!("STRIMS_SIM_CAL_{k}"), v.to_owned());
                }
            }
        }
    }
    for k in ["SEED_BURST", "VIEWER_BURST"] {
        put(
            &format!("STRIMS_SIM_CAL_{k}"),
            get(k).unwrap_or("3028").to_owned(),
        );
    }
    let links = links_of(topology);
    let star: Vec<(usize, usize)> = (1..=viewers).map(|v| (0, v)).collect();
    if links == links_of(&calibration::full_mesh(viewers)) && viewers > 1 {
        put("STRIMS_SIM_CAL_MESH", "full".to_owned());
    } else if [3, 7, 15, 31, 63].contains(&viewers)
        && links == links_of(calibration::mesh6(viewers))
    {
        put("STRIMS_SIM_CAL_MESH", "mesh6".to_owned());
    } else if links != star {
        put(
            "STRIMS_SIM_CAL_TOPOLOGY",
            dir.join("topology").to_string_lossy().into_owned(),
        );
    }
    let node = flags_of(get("NODE_ARGS").unwrap_or(""))?;
    let browser = flags_of(get("BROWSER_ARGS").unwrap_or(""))?;
    let kinds: Vec<&str> = get("KINDS").unwrap_or("").split_whitespace().collect();
    let kind = |i: usize| kinds.get(i).copied().unwrap_or("native");
    let browsers = (0..=viewers).any(|i| kind(i) != "native");
    for (f, args) in [("--chunk", &node), ("--munro", &node)].into_iter().chain(
        browsers
            .then_some([("--chunk", &browser), ("--munro", &browser)])
            .into_iter()
            .flatten(),
    ) {
        let ours = node.get(f);
        if args.get(f) != ours {
            return Err(format!("NODE_ARGS and BROWSER_ARGS differ on {f}"));
        }
    }
    if let Some(extra) = node
        .keys()
        .find(|f| !matches!(f.as_str(), "--chunk" | "--munro"))
    {
        return Err(format!("NODE_ARGS {extra}: the twin has no knob for it"));
    }
    if let (Some(c), Some(m)) = (node.get("--chunk"), node.get("--munro")) {
        put("STRIMS_SIM_CHUNK", c.clone());
        put("STRIMS_SIM_MUNRO", m.clone());
    }
    if browsers {
        if (0..=viewers).any(|i| kind(i) == "native") && get("BROWSER_LINKS") != Some("webrtc") {
            return Err(
                "browsers link to native nodes over WebSockets, which the twin doesn't".to_owned(),
            );
        }
        let stacks: Vec<&str> = (0..=viewers)
            .map(|i| match kind(i) {
                "native" => Ok("str0m"),
                k @ ("chromium" | "firefox") => Ok(k),
                k => Err(format!("KINDS: {k} is no kind")),
            })
            .collect::<Result<_, _>>()?;
        let period = (1..=stacks.len())
            .find(|&p| {
                stacks
                    .iter()
                    .enumerate()
                    .all(|(i, s)| stacks.get(i % p) == Some(s))
            })
            .unwrap_or(stacks.len());
        put(
            "STRIMS_SIM_RTC",
            stacks.get(..period).unwrap_or_default().join(","),
        );
    }
    put(
        "TESTBED_QDISC",
        get("QDISC").unwrap_or(OLD_QDISC).to_owned(),
    );
    for (k, v) in settings {
        let twin = RECORDING.contains(&k.as_str())
            || TWIN_SETTINGS.contains(&k.as_str())
            || (k == "QUEUE_MS" && v == "100");
        if !twin && !v.is_empty() {
            put(&format!("TESTBED_{k}"), v.clone());
        }
    }
    Ok(out)
}

impl Run {
    /// Reads the out dir `dir`, its knobs [`knobs_of`] and `extra`'s.
    pub fn read(dir: &Path, extra: &BTreeMap<String, String>) -> Result<Run, String> {
        let read = |name: &str| {
            std::fs::read_to_string(dir.join(name)).map_err(|e| format!("{name}: {e}"))
        };
        let settings = settings_of(&read("settings")?);
        let links = std::fs::read_to_string(dir.join("links")).ok();
        let mut knobs = knobs_of(&settings, &read("topology")?, links.as_deref(), dir)?;
        knobs.extend(extra.iter().map(|(k, v)| (k.clone(), v.clone())));
        let viewers: usize = knobs
            .get("STRIMS_SIM_CAL_VIEWERS")
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
        let mut nodes = Vec::new();
        for i in 0..=viewers {
            let text = read(&format!("node-{i}.jsonl"))?;
            nodes.push(
                text.lines()
                    .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                    .collect::<Vec<_>>(),
            );
        }
        let start_ms = nodes
            .iter()
            .flatten()
            .find_map(|l| Some(l.get("t_ms")?.as_i64()? - l.get("rel_ms")?.as_i64()?))
            .and_then(|s| u64::try_from(s).ok())
            .ok_or("no node line names the stream's start")?;
        Ok(Run {
            dir: dir.to_owned(),
            settings,
            knobs,
            start_ms,
            nodes,
        })
    }

    /// The run's key: a digest of its nodes' lines.
    pub fn key(&self) -> String {
        let mut h = Sha256::new();
        for (i, lines) in self.nodes.iter().enumerate() {
            h.update(format!("node {i} {}\n", lines.len()).as_bytes());
            for l in lines {
                h.update(l.to_string().as_bytes());
            }
        }
        let mut s = String::from("tb-");
        for b in h.finalize().iter().take(8) {
            let _ = write!(s, "{b:02x}");
        }
        s
    }

    /// The run time of Unix ms `ms`: from 5 s before the stream's start,
    /// `None` before it.
    fn at(&self, ms: u64) -> Option<Duration> {
        let run0 = self.start_ms.checked_sub(LEAD.as_millis() as u64)?;
        ms.checked_sub(run0).map(Duration::from_millis)
    }

    fn file(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.dir.join(name)).ok()
    }

    /// The twin's scenario, with the times the testbed measured.
    fn scenario(&self) -> Scenario {
        let cal = Calibration::from_knobs(|k| self.knobs.get(k).cloned());
        let mut s = calibration_star(&cal);
        let get = |k: &str| self.knobs.get(k).and_then(|v| v.parse::<usize>().ok());
        if let (Some(c), Some(m), Some(sw)) = (
            get("STRIMS_SIM_CHUNK"),
            get("STRIMS_SIM_MUNRO"),
            s.swarms.first_mut(),
        ) && let Ok(x) = sw.clone().with_chunks(c, m)
        {
            *sw = x;
        }
        let seed = s.swarms.first().map_or(0, |sw| sw.seed);
        if let Some(text) = self.file("steps.log")
            && let Some(h) = s.hosts.get_mut(seed)
        {
            h.up_steps = text
                .lines()
                .filter_map(|l| {
                    let mut w = l.split_whitespace();
                    let ms: u64 = w.next()?.parse().ok()?;
                    let mbit: f64 = w.next()?.parse().ok()?;
                    Some((self.at(ms)?, (mbit * 125_000.0) as u64))
                })
                .collect();
        }
        if let Some(text) = self.file("compete.log")
            && let Some(h) = s.hosts.get_mut(seed)
        {
            let rtt = h.competing.first().map(|c| c.rtt).unwrap_or_default();
            h.competing = text
                .lines()
                .filter_map(|l| {
                    let mut w = l.split_whitespace();
                    let ms: u64 = w.next()?.parse().ok()?;
                    let _at = w.next()?;
                    let secs: f64 = w.next()?.parse().ok()?;
                    let start = self.at(ms)?;
                    Some(Competing {
                        start,
                        stop: start + Duration::from_secs_f64(secs),
                        rtt,
                    })
                })
                .collect();
        }
        if let Some(joins) = self.settings.get("JOINS") {
            for j in joins.split_whitespace() {
                if let Some((i, at)) = j.split_once(':')
                    && let (Ok(i), Ok(at)) = (i.parse::<usize>(), at.parse::<f64>())
                    && i > 0
                    && let Some(h) = s.hosts.get_mut(i)
                {
                    h.arrive_at = Some(LEAD + Duration::from_secs_f64(at.max(0.0)));
                }
            }
        }
        if let Some(rtc) = self.knobs.get("STRIMS_SIM_RTC") {
            let stacks: Vec<Option<SctpProfile>> = rtc
                .split(',')
                .map(|x| match x.trim() {
                    "chromium" => Some(SctpProfile::CHROMIUM),
                    "firefox" => Some(SctpProfile::FIREFOX),
                    "str0m" => Some(SctpProfile::STR0M),
                    _ => None,
                })
                .collect();
            s.over_webrtc(&stacks);
        }
        s
    }

    /// When host `h` started reading: its arrival, its last restart, or the
    /// run's start.
    fn joined(s: &Scenario, h: HostIx) -> Duration {
        s.hosts.get(h).and_then(|x| x.arrive_at).unwrap_or_else(|| {
            s.faults
                .iter()
                .filter(|f| matches!(f.fault, Fault::Restart(x) if x == h))
                .map(|f| f.at)
                .max()
                .unwrap_or_default()
        })
    }

    /// The run's report (see the [module docs](self)).
    pub fn report(&self) -> Report {
        let scenario = self.scenario();
        let mut hosts: Vec<HostReport> = scenario
            .hosts
            .iter()
            .map(|h| HostReport {
                name: h.name.clone(),
                ..HostReport::default()
            })
            .collect();
        let mut viewers = Vec::new();
        let mut parent_changes = Vec::new();
        let swarm = scenario.swarms.first();
        let seed = swarm.map_or(0, |s| s.seed);
        for (i, lines) in self.nodes.iter().enumerate() {
            let Some(host) = hosts.get_mut(i) else {
                continue;
            };
            let mut stats = ViewerStats::default();
            let is_viewer = swarm.is_some_and(|s| s.viewers.contains(&i));
            if is_viewer {
                stats.start(Self::joined(&scenario, i).as_millis() as u64);
            }
            let mut last_seed: Option<SeedSample> = None;
            for l in lines {
                let t = l
                    .get("t_ms")
                    .and_then(Value::as_u64)
                    .and_then(|t| self.at(t));
                if let (Some(t), Some(x)) = (t, l.get("transfers").and_then(transfer_of)) {
                    host.transfers.push((t, 0, x));
                }
                let Some(rec) = l.get("rec") else {
                    continue;
                };
                if let Some(m) = rec.get("meter").and_then(|m| self.meter_of(m)) {
                    if let Some(x) = rec.get("seed").and_then(seed_of) {
                        host.runner.seeds.insert(0, x);
                        if last_seed != Some(x) {
                            host.runner.seed_changes.push((m.t, 0, x));
                        }
                        last_seed = Some(x);
                    }
                    host.runner.meter.push(m);
                }
                for b in rec
                    .get("bins")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if let Some(r) = self.bin_of(b) {
                        host.node_bins.push(r);
                    }
                }
                for p in rec
                    .get("parents")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if let Some(r) = self.parent_of(i, p) {
                        parent_changes.push(r);
                    }
                }
                host.parents_lost += rec.get("parents_lost").and_then(Value::as_u64).unwrap_or(0);
                if is_viewer {
                    for r in rec
                        .get("reads")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        let x = |k: usize| r.get(k).and_then(Value::as_u64);
                        if let (Some(seq), Some(stamp), Some(read)) = (x(0), x(1), x(2))
                            && let (Some(stamp), Some(read)) = (self.at(stamp), self.at(read))
                        {
                            let f = FrameInfo {
                                seq,
                                time_ms: stamp.as_millis() as u64,
                            };
                            stats.record(Parsed::Frame(f), read.as_millis() as u64);
                        }
                    }
                }
            }
            if is_viewer {
                viewers.push(ViewerReport {
                    host: i,
                    name: host.name.clone(),
                    swarm: 0,
                    stats,
                });
            }
        }
        parent_changes.sort_by_key(|p: &ParentRecord| (p.at, p.node));
        let mut written = SeqSet::default();
        if let Some(s) = swarm {
            for seq in s.frames_between(s.start, s.stop) {
                written.insert(seq);
            }
        }
        let net = Network::read(self, &scenario, seed);
        Report {
            seed: 0,
            trace_hash: self.key(),
            trace: Vec::new(),
            sim_secs: scenario.duration.as_secs_f64(),
            wall_secs: 0.0,
            viewers,
            hosts,
            written: vec![written],
            media: vec![None],
            vpn: None,
            memory: None,
            violations: Vec::new(),
            wan: Some(net.wan),
            wire_bins: net.bins,
            wire_unmeasured: net.unmeasured,
            sockets: Vec::new(),
            parent_losses: Vec::new(),
            parent_changes,
            loops: None,
            scenario,
        }
    }

    fn meter_of(&self, m: &Value) -> Option<MeterSample> {
        let u = |k: &str| m.get(k).and_then(Value::as_u64);
        let o = |k: &str| u(k).map(|x| u32::try_from(x).unwrap_or(u32::MAX));
        let n = |k: &str| o(k).unwrap_or(0);
        let by_swarm = |k: &str| -> BTreeMap<usize, u64> {
            m.get(k)
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .filter_map(|(s, v)| Some((s.parse().ok()?, v.as_u64()?)))
                .collect()
        };
        let t = self.at(u("t_ms")?)?;
        // A node is sampled at its line: its truth is read there.
        Some(MeterSample {
            t,
            asked: t,
            rate: u("rate")?,
            busy_permille: n("busy_permille"),
            queue_delay_ms: o("queue_delay_ms"),
            path_delay_ms: o("path_delay_ms"),
            path_queued_ms: o("path_queued_ms"),
            path_latest_ms: o("path_latest_ms"),
            capacity: u("capacity")?,
            budget_capacity: by_swarm("budget_capacity"),
            latest_floor_ms: by_swarm("latest_floor_ms")
                .into_iter()
                .map(|(s, f)| (s, u32::try_from(f).unwrap_or(u32::MAX)))
                .collect(),
            children: by_swarm("children")
                .into_iter()
                .map(|(s, c)| (s, c as usize))
                .collect(),
            requests_in: n("requests_in"),
            rejected: n("rejected"),
            pace_channels: n("pace_channels"),
            lazy_out: n("lazy_out"),
            lazy_in: n("lazy_in"),
            bytes_written: u("bytes_written").unwrap_or(0),
            ..MeterSample::default()
        })
    }

    fn bin_of(&self, b: &Value) -> Option<NodeBinRecord> {
        let edge = b.get(0)?.as_u64()?;
        let since = edge.checked_sub(self.start_ms)?;
        let mut counters = [0u64; NODE_BIN_COUNTERS.len()];
        let c = b.get(1)?.as_array()?;
        for (x, v) in counters.iter_mut().zip(c) {
            *x = v.as_u64()?;
        }
        let waits = |k: usize| {
            let mut h = WaitHistogram::default();
            for p in b.get(k).and_then(Value::as_array).into_iter().flatten() {
                if let (Some(i), Some(n)) = (
                    p.get(0).and_then(Value::as_u64),
                    p.get(1).and_then(Value::as_u64),
                ) && let Some(x) = h.buckets.get_mut(i as usize)
                {
                    *x = n;
                }
            }
            h
        };
        Some(NodeBinRecord {
            bin: since / BIN_MS.unsigned_abs(),
            incarnation: 1,
            counters,
            waits: [waits(2), waits(3)],
        })
    }

    fn parent_of(&self, node: HostIx, p: &Value) -> Option<ParentRecord> {
        let why = p.get(3)?.as_str()?;
        Some(ParentRecord {
            at: self.at(p.get(0)?.as_u64()?)?,
            node,
            stream: u16::try_from(p.get(1)?.as_u64()?).ok()?,
            parent: p.get(2).and_then(Value::as_u64).map(|x| x as usize),
            why: ParentWhy::ALL.into_iter().find(|w| w.name() == why)?,
        })
    }
}

/// A seed's state from its record (`SeedSample::json`).
fn seed_of(v: &Value) -> Option<SeedSample> {
    let x = |i: usize| v.get(i).and_then(Value::as_u64);
    Some(SeedSample {
        budget: x(0)? as usize,
        window: x(1)? as usize,
        copies: x(2)? as usize,
        first_wave: x(3)? as usize,
        first_wave_capacity: x(4)?,
    })
}

/// A node's transfers from its line (`TransferSample::json`).
fn transfer_of(v: &Value) -> Option<TransferSample> {
    let o = v.as_object()?;
    let u = |k: &str| o.get(k).and_then(Value::as_u64).unwrap_or(0);
    let peers = o
        .get("peers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|p| {
            let u = |k: &str| p.get(k).and_then(Value::as_u64).unwrap_or(0);
            let i = |k: &str| p.get(k).and_then(Value::as_i64).unwrap_or(0);
            let q = |k: usize| {
                p.get("queued")
                    .and_then(|q| q.get(k))
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize
            };
            PeerTransfer {
                peer: p.get("peer").and_then(Value::as_u64).map(|x| x as usize),
                sent: u("sent"),
                received: u("received"),
                requests_in: u("requests_in"),
                requests_out: u("requests_out"),
                queued: [q(0), q(1)],
                cwnd: i("cwnd"),
                flight: i("flight"),
                rtt_ms: u("rtt_ms"),
                cto_ms: u("cto_ms"),
                pending: u("pending") as usize,
            }
        })
        .collect();
    let parent_lag = o
        .get("parent_lag")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|l| Some((l.get(0)?.as_u64()?, l.get(1)?.as_u64()?)))
        .collect();
    Some(TransferSample {
        chunks_sent: u("chunks_sent"),
        pushed_chunks: u("pushed_chunks"),
        children: u("children") as usize,
        parents: u("parents") as usize,
        parent_peers: o
            .get("parent_peers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|p| p.as_u64().map(|x| x as usize))
            .collect(),
        hits: u("hits"),
        misses: u("misses"),
        late: u("late"),
        opens: u("opens"),
        declined: u("declined"),
        open_timeouts: u("open_timeouts"),
        shed: u("shed"),
        pruned: u("pruned"),
        switched: u("switched"),
        accepted: u("accepted"),
        rejected: u("rejected"),
        sheds_sent: u("sheds_sent"),
        stale_drops: u("stale_drops"),
        parent_lag,
        peers,
        overhead: u("overhead"),
    })
}

/// One packet a node's capture holds: when (Unix µs), its source and
/// destination's last octets in the testbed's subnet, its IP length, and
/// whether it is a pure TCP acknowledgement (no payload, no SYN, FIN or
/// RST).
struct Packet {
    us: u64,
    src: Option<u8>,
    dst: Option<u8>,
    ip_len: u64,
    pure_ack: bool,
}

/// A libpcap file's IPv4 packets over Ethernet; `None` for another
/// format.
fn packets(bytes: &[u8]) -> Option<Vec<Packet>> {
    let word = |b: &[u8], at: usize, le: bool| -> Option<u32> {
        let w: [u8; 4] = b.get(at..at + 4)?.try_into().ok()?;
        Some(if le {
            u32::from_le_bytes(w)
        } else {
            u32::from_be_bytes(w)
        })
    };
    let magic = word(bytes, 0, true)?;
    let (le, nanos) = match magic {
        0xa1b2_c3d4 => (true, false),
        0xa1b2_3c4d => (true, true),
        0xd4c3_b2a1 => (false, false),
        0x4d3c_b2a1 => (false, true),
        _ => return None,
    };
    if word(bytes, 20, le)? != 1 {
        return None;
    }
    let mut out = Vec::new();
    let mut at = 24;
    while let (Some(sec), Some(frac), Some(len)) = (
        word(bytes, at, le),
        word(bytes, at + 4, le),
        word(bytes, at + 8, le),
    ) {
        let data = bytes.get(at + 16..at + 16 + len as usize)?;
        at += 16 + len as usize;
        let us = u64::from(sec) * 1_000_000 + u64::from(if nanos { frac / 1000 } else { frac });
        let (Some(&[0x08, 0x00]), Some(ip)) = (data.get(12..14), data.get(14..)) else {
            continue;
        };
        let (Some(&vihl), Some(len), Some(&proto)) = (ip.first(), ip.get(2..4), ip.get(9)) else {
            continue;
        };
        let ihl = usize::from(vihl & 0x0f) * 4;
        let ip_len = u64::from(u16::from_be_bytes([
            len.first().copied().unwrap_or(0),
            len.get(1).copied().unwrap_or(0),
        ]));
        let host = |at: usize| {
            ip.get(at..at + 4)
                .filter(|a| a.get(..3) == Some(&SUBNET[..]))
                .and_then(|a| a.get(3).copied())
        };
        let pure_ack = proto == 6
            && ip
                .get(ihl + 12)
                .zip(ip.get(ihl + 13))
                .is_some_and(|(&off, &flags)| {
                    let thl = u64::from(off >> 4) * 4;
                    ip_len == ihl as u64 + thl && flags & 0x07 == 0 && flags & 0x10 != 0
                });
        out.push(Packet {
            us,
            src: host(12),
            dst: host(16),
            ip_len,
            pure_ack,
        });
    }
    Some(out)
}

/// A tc-<i>.log line's tbf: (Unix ms, packets dropped, backlog bytes).
fn tbf_of(line: &str) -> Option<(u64, u64, u64)> {
    let (ms, rest) = line.split_once(' ')?;
    let tbf = rest.split("qdisc tbf 10:").nth(1)?;
    let tbf = tbf.split(" qdisc ").next()?;
    let after = |key: &str| {
        let s = tbf.split(key).nth(1)?;
        Some(s.trim_start().split([',', ' ']).next()?.to_owned())
    };
    let drops: u64 = after("dropped")?.parse().ok()?;
    let backlog = after("backlog")?;
    let (n, unit) = backlog.split_at(backlog.find(|c: char| !c.is_ascii_digit())?);
    let mult = match unit {
        "b" => 1,
        "Kb" => 1024,
        "Mb" => 1024 * 1024,
        _ => return None,
    };
    Some((ms.parse().ok()?, drops, n.parse::<u64>().ok()? * mult))
}

/// The network side of a testbed run's report.
struct Network {
    wan: WanReport,
    bins: Vec<Vec<[u64; UP_COUNTERS.len()]>>,
    unmeasured: Vec<usize>,
}

impl Network {
    fn read(run: &Run, s: &Scenario, seed: HostIx) -> Network {
        let bin = BIN_MS.unsigned_abs().max(1);
        let edges = s.duration.saturating_sub(LEAD).as_millis() as u64 / bin + 1;
        let edge_us = |k: u64| (run.start_ms + k * bin) * 1000;
        let mut bins = vec![vec![[0u64; UP_COUNTERS.len()]; s.hosts.len()]; edges as usize];
        let mut unmeasured: Vec<usize> = [
            "retransmits",
            "window_cuts",
            "rtos",
            "up_sack_packets",
            "up_sack_bytes",
        ]
        .map(up)
        .to_vec();
        let (mut have_pcap, mut have_tc) = (true, true);
        let mut wan = WanReport::default();
        for (h, spec) in s.hosts.iter().enumerate() {
            let Some(site) = spec.site else {
                continue;
            };
            let mut w = WanHostReport {
                name: spec.name.clone(),
                site,
                stats: Default::default(),
                up_queue: Vec::new(),
                up_backlog: Vec::new(),
                competing: Vec::new(),
                competing_log: Vec::new(),
                queue_drops: 0,
                queue_drop_packets: 0,
                acks: 0,
                ack_bytes: 0,
                ack_whys: [0; ACK_WHYS.len()],
                ack_waits: AckWaits::default(),
                app_bytes: 0,
                competing_drops: 0,
                competing_cwnd: Vec::new(),
            };
            if h >= run.nodes.len() {
                wan.hosts.push(w);
                continue;
            }
            let me = u8::try_from(10 + h).ok();
            let pcap = std::fs::read(run.dir.join(format!("pcap-{h}.pcap")))
                .ok()
                .and_then(|b| packets(&b));
            match pcap {
                Some(ps) => {
                    let mut mine = ps
                        .iter()
                        .filter(|p| p.src.is_some() && p.src == me)
                        .peekable();
                    let mut cum = [0u64; UP_COUNTERS.len()];
                    let mut cross = 0u64;
                    for (k, edge) in bins.iter_mut().enumerate() {
                        while let Some(p) = mine.next_if(|p| p.us < edge_us(k as u64)) {
                            if p.dst == Some(SINK) {
                                cross += p.ip_len;
                            } else if p.pure_ack {
                                add(&mut cum, "up_ack_packets", 1);
                                add(&mut cum, "up_ack_bytes", p.ip_len);
                            } else {
                                add(&mut cum, "up_bytes", p.ip_len);
                                add(&mut cum, "up_packets", 1);
                                let small = p.ip_len <= strims_wan::wire::SMALL_PACKET as u64;
                                add(&mut cum, "small_packets", u64::from(small));
                            }
                        }
                        if let Some(c) = edge.get_mut(h) {
                            for (i, (x, v)) in c.iter_mut().zip(cum).enumerate() {
                                if i != up("drops") {
                                    *x = v;
                                }
                            }
                        }
                        if h == seed {
                            let t = LEAD + Duration::from_millis(k as u64 * bin);
                            w.competing.push((t.into(), cross));
                        }
                    }
                    w.acks = cum.get(up("up_ack_packets")).copied().unwrap_or(0);
                    w.ack_bytes = cum.get(up("up_ack_bytes")).copied().unwrap_or(0);
                }
                None => have_pcap = false,
            }
            // A node shaped by no tbf of handle 10: has no counters here.
            let tbf = run
                .file(&format!("tc-{h}.log"))
                .map(|t| t.lines().filter_map(tbf_of).collect::<Vec<_>>())
                .filter(|s| !s.is_empty());
            match tbf {
                Some(samples) => {
                    let mut at = samples.iter().peekable();
                    let mut last = samples.first().map_or(0, |x| x.1);
                    for (k, edge) in bins.iter_mut().enumerate() {
                        while let Some(&(_, d, _)) = at.next_if(|x| x.0 * 1000 < edge_us(k as u64))
                        {
                            last = d;
                        }
                        if let Some(x) = edge.get_mut(h).and_then(|c| c.get_mut(up("drops"))) {
                            *x = last;
                        }
                    }
                    w.queue_drops = samples
                        .last()
                        .zip(samples.first())
                        .map_or(0, |(b, a)| b.1.saturating_sub(a.1));
                    // tbf drops frames: GSO is off, a segment is one.
                    w.queue_drop_packets = w.queue_drops;
                    for &(ms, _, backlog) in &samples {
                        let Some(t) = run.at(ms) else {
                            continue;
                        };
                        let rate = spec
                            .up_steps
                            .iter()
                            .rfind(|&&(at, _)| at <= t)
                            .map_or(site.profile.up, |&(_, r)| r)
                            .max(1);
                        let wait = i64::try_from(backlog * 1_000_000 / rate).unwrap_or(i64::MAX);
                        w.up_queue
                            .push((t.into(), strims_base::time::Duration::from_micros(wait)));
                    }
                }
                None => have_tc = false,
            }
            wan.hosts.push(w);
        }
        if !have_pcap {
            unmeasured.extend(
                [
                    "up_bytes",
                    "up_packets",
                    "small_packets",
                    "up_ack_packets",
                    "up_ack_bytes",
                ]
                .map(up),
            );
        }
        if !have_tc {
            unmeasured.push(up("drops"));
        }
        unmeasured.sort_unstable();
        unmeasured.dedup();
        if unmeasured.len() == UP_COUNTERS.len() {
            bins.clear();
        }
        Network {
            wan,
            bins,
            unmeasured,
        }
    }
}

/// The index of uplink counter `name` ([`UP_COUNTERS`]).
fn up(name: &str) -> usize {
    UP_COUNTERS.iter().position(|&c| c == name).unwrap_or(0)
}

/// Adds `n` to counter `name` of `c`.
fn add(c: &mut [u64; UP_COUNTERS.len()], name: &str, n: u64) {
    if let Some(x) = c.get_mut(up(name)) {
        *x += n;
    }
}

/// The row fields a testbed run's report writes ([`Report::row`]): the
/// run's names, and those made of what a testbed run records alone (its
/// viewers' reads and joins, its nodes' meter samples, its seed's changes,
/// its nodes' parents), not of the network model's counters or truths,
/// which a testbed run's report lacks.
pub const ROW: &[&str] = &[
    "test",
    "scenario",
    "trace",
    "reads_hash",
    "passed",
    "failure",
    "delivery",
    "delivery_honest",
    "lag_p50_ms",
    "lag_p99_ms",
    "lag_p50_ms_honest",
    "lag_p99_ms_honest",
    "stalls_per_min_1250",
    "stalled_share_1250",
    "stalls_per_min_4250",
    "stalled_share_4250",
    "stalls_per_min_1250_honest",
    "stalled_share_1250_honest",
    "stalls_per_min_4250_honest",
    "stalled_share_4250_honest",
    "join_p50_ms",
    "join_p90_ms",
    "caught_up_p50_ms",
    "caught_up_p90_ms",
    "caught_up_never",
    "rejoin_first_p90_ms",
    "rejoin_caught_up_p90_ms",
    "meter_seed_p10",
    "meter_seed_p50",
    "meter_seed_p90",
    "meter_budget_seed_p10",
    "meter_budget_seed_p50",
    "meter_budget_seed_p90",
    "meter_bucket_seeds",
    "meter_accepting_p10",
    "meter_accepting_p50",
    "meter_accepting_p90",
    "meter_delivering_p10",
    "meter_delivering_p50",
    "meter_delivering_p90",
    "meter_others_p10",
    "meter_others_p50",
    "meter_others_p90",
    "meter_bucket_accepting",
    "meter_bucket_delivering",
    "meter_bucket_others",
    "n_accepting",
    "n_delivering",
    "n_others",
    "seed_first_wave",
    "seed_first_wave_capacity",
    "fanout_p50",
    "fanout_p90",
    "depth_none",
    "depth_1",
    "depth_2",
    "depth_3",
    "depth_4",
    "depth_5",
    "depth_6",
    "depth_deeper",
    "depth_cycle",
    "depth_pull_rooted",
    "parent_changes_per_min",
];

/// The families of row fields a testbed run's report writes beside
/// [`ROW`]'s: the queueing delay readings' scores against the uplink's
/// queue (`Report::queue_delay_accuracy`: `{reading}_err_{role}_ms`,
/// `{reading}_fp_{role}`, `{reading}_fn_{role}`) and the standing queue's
/// share (`standing_{pop}`), whose truth is a testbed run's tbf backlog over
/// its rate (tc-<i>.log) as it is a sim's model queue.
pub const ROW_FAMILIES: &[&str] = &["qd_", "qdv_", "qdb_", "qdbi_", "qdl_", "standing_"];

/// The testbed run's row: [`Report::row`]'s fields that [`ROW`] names or
/// [`ROW_FAMILIES`] holds.
pub fn row(r: &Report) -> String {
    let full: serde_json::Map<String, Value> =
        serde_json::from_str(&r.row(TEST, None)).unwrap_or_default();
    let kept: serde_json::Map<String, Value> = full
        .into_iter()
        .filter(|(k, _)| ROW.contains(&k.as_str()) || ROW_FAMILIES.iter().any(|f| k.starts_with(f)))
        .collect();
    Value::Object(kept).to_string()
}

/// The run's job for `hack/sim-db`'s fold (`job.json`): its key, sim,
/// test, twin's knobs and row.
pub fn job(run: &Run, report: &Report) -> String {
    let env: serde_json::Map<String, Value> = run
        .knobs
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    let row: Value = serde_json::from_str(&row(report)).unwrap_or(Value::Null);
    serde_json::json!({
        "k": run.key(),
        "sim": SIM,
        "test": TEST,
        "env": env,
        "row": row,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    /// The stream's start in the fixture, Unix ms.
    const START: u64 = 1_800_000_000_000;

    /// A libpcap file of Ethernet frames: (Unix µs, IPv4 packet bytes).
    fn pcap(packets: &[(u64, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0xa1b2_c3d4u32.to_le_bytes());
        out.extend_from_slice(&[2, 0, 4, 0]);
        out.extend_from_slice(&[0; 8]);
        out.extend_from_slice(&65535u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        for (us, ip) in packets {
            let mut frame = vec![0u8; 12];
            frame.extend_from_slice(&[0x08, 0x00]);
            frame.extend_from_slice(ip);
            let len = frame.len() as u64;
            for w in [us / 1_000_000, us % 1_000_000, len, len] {
                out.extend_from_slice(&(w as u32).to_le_bytes());
            }
            out.extend_from_slice(&frame);
        }
        out
    }

    /// An IPv4 TCP packet from `src` to `dst` (last octets) of `payload`
    /// bytes, `flags` its TCP flags, with the timestamp option.
    fn tcp(src: u8, dst: u8, payload: usize, flags: u8) -> Vec<u8> {
        let len = 20 + 32 + payload;
        let mut p = vec![0u8; len];
        p[0] = 0x45;
        p[2..4].copy_from_slice(&(len as u16).to_be_bytes());
        p[9] = 6;
        p[12..16].copy_from_slice(&[10, 77, 0, src]);
        p[16..20].copy_from_slice(&[10, 77, 0, dst]);
        p[20 + 12] = 8 << 4;
        p[20 + 13] = flags;
        p
    }

    /// A seed and one viewer: the seed's two lines and the viewer's one,
    /// which read two frames; the seed's capture a data packet, a pure ACK,
    /// a packet to the competing upload's sink and one it received; both
    /// tbfs' counters.
    fn fixture() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("strims-testbed-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let w = |name: &str, text: &str| std::fs::write(dir.join(name), text).unwrap();
        w(
            "settings",
            "VIEWERS=1\nSECS=10\nQDISC=clsact>ifb-netem>root-tbf\n",
        );
        w(
            "topology",
            "node 0 10.77.0.10:4000\nnode 1 10.77.0.11:4000\nlink 0 1\n",
        );
        w(
            "links",
            "node 0 tbf 10 20 100\nnode 1 cable,slot=2-4,aqm=fifo 20 10 100\n",
        );
        let meter = |t: u64| {
            format!(
                "{{\"t_ms\":{t},\"rate\":1000,\"busy_permille\":5,\"queue_delay_ms\":null,\"path_delay_ms\":null,\"path_queued_ms\":null,\"path_latest_ms\":3,\"capacity\":5000,\"budget_capacity\":{{\"0\":4000}},\"children\":{{\"0\":2}},\"requests_in\":1,\"rejected\":0,\"pace_channels\":1,\"lazy_out\":0,\"lazy_in\":0,\"bytes_written\":99}}"
            )
        };
        let line = |t: u64, rec: &str| {
            format!(
                "{{\"t_ms\":{t},\"rel_ms\":{},\"transfers\":{{\"hits\":1,\"parent_peers\":[0]}},\"rec\":{rec}}}\n",
                t as i64 - START as i64
            )
        };
        let bins = format!("[[{},[1{}],[[2,3]],[]]]", START + 100, ",0".repeat(29));
        let seed_lines: Vec<String> = [1000, 2000]
            .iter()
            .map(|&d| {
                line(
                    START + d,
                    &format!(
                        "{{\"meter\":{},\"seed\":[16,32,{},1,0],\"bins\":{bins},\"parents\":[],\"parents_lost\":0,\"reads\":[]}}",
                        meter(START + d),
                        d / 1000
                    ),
                )
            })
            .collect();
        w("node-0.jsonl", &seed_lines.concat());
        w(
            "node-1.jsonl",
            &line(
                START + 1000,
                &format!(
                    "{{\"meter\":null,\"seed\":null,\"bins\":[],\"parents\":[[{},0,0,\"open\"]],\"parents_lost\":0,\"reads\":[[0,{START},{}],[1,{START},{}]]}}",
                    START + 300,
                    START + 40,
                    START + 45
                ),
            ),
        );
        let us = |ms: u64| (START + ms) * 1000;
        let packets = [
            (us(50), tcp(10, 11, 1448, 0x18)),
            (us(60), tcp(10, 11, 0, 0x10)),
            (us(70), tcp(10, 250, 1448, 0x10)),
            (us(80), tcp(11, 10, 100, 0x18)),
        ];
        std::fs::write(dir.join("pcap-0.pcap"), pcap(&packets)).unwrap();
        std::fs::write(dir.join("pcap-1.pcap"), pcap(&[])).unwrap();
        let tc = |drops: u64| {
            format!(
                "{} qdisc tbf 10: root refcnt 2 rate 10Mbit burst 3028b lat 99ms  Sent 3000 bytes 2 pkt (dropped {drops}, overlimits 0 requeues 0)  backlog 1514b 1p requeues 0  qdisc clsact ffff: parent ffff:fff1 Sent 0 bytes 0 pkt (dropped 9, overlimits 0 requeues 0)\n",
                START + 150
            )
        };
        w("tc-0.log", &tc(4));
        w("tc-1.log", &tc(0));
        dir
    }

    /// A testbed run's report reads its records as the sim's: each read a
    /// lag from its frame's due time, each meter sample and seed change,
    /// the bins, the parents; and its captures in the sim's wire bytes:
    /// IP lengths, the pure ACK apart, the competing upload's apart, the
    /// tbf's drops and backlog.
    #[test]
    fn a_testbed_runs_records_and_captures_read_as_a_sim_runs() {
        let dir = fixture();
        let run = Run::read(&dir, &BTreeMap::new()).unwrap();
        assert_eq!(run.knobs["STRIMS_SIM_CAL_VIEWERS"], "1");
        assert_eq!(run.knobs["TESTBED_QDISC"], "clsact>ifb-netem>root-tbf");
        assert_eq!(run.knobs["STRIMS_SIM_CAL_SEED_LINK"], "tbf");
        assert_eq!(
            run.knobs["STRIMS_SIM_CAL_VIEWER_LINK"],
            "cable,slot=2-4,aqm=fifo"
        );
        let r = run.report();
        assert_eq!(r.viewers[0].stats.lags, [(0, 40), (1, 45)]);
        let seed = &r.hosts[0];
        assert_eq!(seed.runner.meter.len(), 2);
        assert_eq!(seed.runner.meter[0].t, LEAD + Duration::from_secs(1));
        assert_eq!(seed.runner.meter[0].children[&0], 2);
        assert_eq!(seed.runner.seed_changes.len(), 2);
        assert_eq!(seed.node_bins[0].bin, 1);
        assert_eq!(seed.node_bins[0].waits[0].buckets[2], 3);
        assert_eq!(seed.transfers[0].2.parent_peers, [Some(0)]);
        assert_eq!(r.parent_changes.len(), 1);
        assert_eq!(r.parent_changes[0].parent, Some(0));
        let last = r.wire_bins.last().unwrap();
        assert_eq!(last[0][up("up_bytes")], 1500);
        assert_eq!(last[0][up("up_packets")], 1);
        assert_eq!(last[0][up("up_ack_packets")], 1);
        assert_eq!(last[0][up("up_ack_bytes")], 52);
        assert_eq!(last[0][up("drops")], 4);
        assert_eq!(last[1][up("up_bytes")], 0);
        assert!(r.wire_unmeasured.contains(&up("retransmits")));
        let w = &r.wan.as_ref().unwrap().hosts[0];
        assert_eq!(w.competing.last().unwrap().1, 1500);
        assert_eq!(w.up_queue[0].1.as_micros(), 1514 * 1_000_000 / 1_250_000);
        let out = dir.join("tables");
        r.write_tables(&out, Duration::from_millis(100), true)
            .unwrap();
        let meter = crate::tables::read_table(&out, "meter_samples").unwrap();
        assert_eq!(meter.lines().count(), 3);
        let reads = crate::tables::read_table(&out, "reads").unwrap();
        assert_eq!(reads.lines().count(), 3);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
