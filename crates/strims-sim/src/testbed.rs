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
//!   (`JOINS`, which is the twin's own knob unless the joins draw an
//!   arrivals profile). Time is the twin's: the stream starts 5 s into the
//!   run.
//! - **The nodes.** Each node's lines (`node-<i>.jsonl`) carry its records
//!   (`strims_testbed::node`'s `rec`, made by the functions strims-sim's
//!   own nodes' samples go through, [`strims_testbed::record`]): its meter
//!   samples and its adaptive seed's changes, its transfers, its stats bins,
//!   its parent changes, and every frame it read, which each viewer's
//!   [`ViewerStats`] takes as a simulated viewer's does
//!   ([`ViewerStats::record`]); with run.sh's `FORECAST=1` or `SPEC=1`,
//!   its scheduler's logs too (`rec.logs`, [`strims_testbed::record::logs`]),
//!   which its report takes as a simulated node's samples hand theirs
//!   (`add_logs`), and with `PARENT_CHECKS=1` its parent checks
//!   (`rec.checks`, its decision log's). Reading a run holds every node's
//!   lines at once, so its memory grows with them: lines with the logs
//!   take about twelve times
//!   their bytes.
//! - **The network.** The WAN columns ([`crate::tables::UP_COUNTERS`])
//!   count each uplink's packets and bytes, its pure acknowledgements and
//!   a data channel's SACKs apart. The twin counts its bytes as its access
//!   link charges them (`strims_wan::framing`: under the testbed's tbf
//!   class, each packet's IP length and an Ethernet header). A testbed run
//!   counts each packet's IP length from its node's capture of its egress
//!   to the swarm's nodes (`PCAP=1`: `pcap-<i>.pcap`, after the queue), and
//!   its queue drops and backlog from its tbf (`tc-<i>.log`), whose rate,
//!   as each sample shows it, is the uplink's; the queries convert between
//!   the two by the link class (scoring.sql's link_overhead). What a
//!   capture can't tell (a data channel's SACKs inside DTLS, which it
//!   counts with the data) is left empty, and a sampler that stopped before
//!   the run's end leaves its columns empty past it (`Report::wire_ends`,
//!   `WanHostReport::up_queue_until`): a tc sampler past its last sample, a
//!   capture past the stamp run.sh writes when its tcpdump exits while its
//!   node runs (`pcap-<i>.end`). The competing upload's bytes are its
//!   packets' to the sink, from the seed's capture.
//! - **TCP's own counts.** A native node's retransmits are its capture's
//!   segments to the swarm that resent bytes its link had carried
//!   (`carried_again`): a loss past its queue, as the window model counts
//!   its path's, a spurious resend too. One its own queue dropped counts in
//!   its drops, and so does one a receiver's downlink queue dropped
//!   (SLOW_DOWN_KBIT's viewer 1, the testbed's one loss past a queue but
//!   SEED_LOSS_PCT's): its resend of carried bytes to that receiver, as
//!   the window model counts a full downlink queue's loss as the sender's
//!   drop. Its window cuts and timeouts are its network namespace's TCP
//!   counters' (`netstat-<i>.log`, `tcp_counts`: the recoveries Linux
//!   entered and the losses a tail-loss probe repaired, and its RTOs, a
//!   spurious one's too), as of each bin's edge, over every TCP flow of
//!   the namespace; in a run with browsers the twin's swarm links are all
//!   data channels, so a native node's TCP links to native nodes have none
//!   there. A browser's links are data channels, whose resends DTLS hides:
//!   its are empty, as are the retransmits of a native node with a data
//!   channel to one, the drops of one sending a slow receiver over a data
//!   channel, the seed's drops and retransmits where SEED_LOSS_PCT's loss
//!   and a slow receiver's both lose its packets, and the seed's cuts and
//!   timeouts in a run with competing uploads, whose flows its namespace's
//!   counters count too.
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
use strims_base::time::Time;
use strims_ppspp::{ParentAction, ParentWhy};
use strims_testbed::record::logs::Logs;
use strims_testbed::record::{
    BIN_MS, Gated, MeterSample, NODE_BIN_COUNTERS, SeedSample, timers_of,
};
use strims_testbed::stream::{FrameInfo, Parsed};
use strims_testbed::transfer::{PeerTransfer, TransferSample};

use crate::calibration::{self, Calibration, WAN_RATE, calibration_star};
use crate::report::{
    ACK_WHYS, AckWaits, ArrivalHostRecord, ForecastRecord, HaveHostRecord, HostReport,
    NodeBinRecord, ParentCheckRecord, ParentRecord, Report, RequestHostRecord,
    SeedDecisionHostRecord, SeedSendHostRecord, SocketSample, TimerHostRecord, ViewerReport,
    WanHostReport, WanReport,
};
use crate::spec::{Competing, Fault, HostIx, HostSet, Scenario};
use crate::stream::{SeqSet, ViewerStats};
use crate::wan::{SctpProfile, UP_COUNTERS};

mod packets;

pub use packets::{NodeCapture, NodeLink, bench_captures, captures, node_link};

/// The twin's stream starts this far into its run.
const LEAD: Duration = Duration::from_secs(5);
/// The testbed's network, node `i` at `.10 + i`, the competing upload's
/// sink at `.250`.
const SUBNET: [u8; 3] = [10, 77, 0];
const SINK: u8 = 250;
/// How long after a datagram's offer its link may carry it: past that, the
/// queue dropped it (`dropped`). The delay and the queue hold a packet some
/// 20 ms and 100 ms at most.
const DROP_WAIT_US: u64 = 1_000_000;
/// The test whose scenario is a testbed run's twin.
pub const TEST: &str = "calibration_star";
/// The sim name a testbed run's records go under.
pub const SIM: &str = "testbed";
/// The settings that only record or capture, or say how run.sh ran
/// (`STAMPS`, `JOIN_LINKS`, which [`knobs_of`] reads apart): no knob of
/// the twin's.
const RECORDING: [&str; 11] = [
    "STAMPS",
    "CAPTURE",
    "PCAP",
    "PCAP_SNAPLEN",
    "SS_EVERY",
    "CPU_PROFILE",
    "RECORD",
    "NODE_BIN",
    "BROWSER_BIN",
    "WEB_BUNDLE",
    "JOIN_LINKS",
];

/// The settings [`knobs_of`] maps to the twin's knobs.
const TWIN_SETTINGS: [&str; 30] = [
    "FORECAST",
    "SPEC",
    "PARENT_CHECKS",
    "TIMER_LOG",
    "VIEWERS",
    "SEED_UP_MBIT",
    "VIEWER_UP_MBIT",
    "SEED_BURST",
    "VIEWER_BURST",
    "SEED_PEAK_MBIT",
    "VIEWER_PEAK_MBIT",
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
    "BROWSER_ARGS",
    "KINDS",
    "BROWSER_LINKS",
    "RELAY",
    "QDISC",
    "SEED_LINK",
    "VIEWER_LINK",
    "LINKS",
];

/// How a run's settings say its nodes made their links at their joins
/// (`JOIN_LINKS`): all at once, as strims-testbed's topology module has a
/// deployed viewer make them, and the twin's viewers make theirs.
const JOIN_LINKS_TOGETHER: &str = "together";

/// The clock a run's settings name (`STAMPS`) where run.sh stamps its logs
/// and reckons its waits from bash's clock, which nothing makes late.
const STAMP_CLOCK: &str = "clock";

/// How far from its schedule a step or a competing upload may land: a
/// `tc qdisc change` or an iperf3's start through `docker exec`, not a
/// wait fired early.
const SCHEDULE_SLACK_MS: u64 = 2000;

/// The qdisc trees of the runs from before run.sh named its tree
/// (`QDISC`): a netem root delaying what a tbf child then shaped; then an
/// ifb's netem ahead of a root tbf; then (with a `links` file) the access
/// link classes' root.
const OLD_QDISC: &str = "netem>tbf";
const IFB_TBF_QDISC: &str = "clsact>ifb-netem>root-tbf";
const CLASS_QDISC: &str = "clsact>ifb-netem>root-class";

/// The qdisc tree that shaped a run: settings' `QDISC`, else the tree its
/// out dir `dir` shows (a `links` file: the access link classes'; the
/// seed's tc-0.log dump with a tbf 10: at its root: an ifb ahead of it;
/// one with a netem root: the old tree), else none known.
fn qdisc_of(settings: &BTreeMap<String, String>, dir: &Path) -> Result<String, String> {
    if let Some(q) = settings.get("QDISC").filter(|q| !q.is_empty()) {
        return Ok(q.clone());
    }
    if dir.join("links").exists() {
        return Ok(CLASS_QDISC.to_owned());
    }
    // The root qdisc tells the two trees apart: an old netem-root run with
    // PCAP=1 has a clsact too (its capture's mirror).
    match std::fs::read_to_string(dir.join("tc-0.log")) {
        Ok(t) if t.contains("qdisc tbf 10: root") => Ok(IFB_TBF_QDISC.to_owned()),
        Ok(t) if t.contains("qdisc netem") => Ok(OLD_QDISC.to_owned()),
        _ => Err(
            "no QDISC, links file or tc-0.log dump: the qdisc tree that shaped it is unknown"
                .to_owned(),
        ),
    }
}

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
/// always; the scheduler's logs the nodes kept (run.sh's `FORECAST=1`,
/// `SPEC=1`, `PARENT_CHECKS=1`: `STRIMS_SIM_FORECAST=1`,
/// `STRIMS_SIM_SPEC=1`, `STRIMS_SIM_PARENT_CHECKS=1`) and the nodes that
/// kept their timers (`TIMER_LOG`: `STRIMS_SIM_TIMER_LOG`, their numbers),
/// so the twin writes the log tables the run does; the viewers, the seed's
/// uplink and the stream's length always; the viewers' joins (`JOIN_AT`,
/// and each of `JOINS`'s to the ms, `STRIMS_SIM_CAL_JOINS`: [`Run::read`]
/// leaves the latter out where its caller names the arrivals profile they
/// drew);
/// the other links' settings where they aren't the twin's defaults; the
/// seed's steps as factors of its uplink (`STRIMS_SIM_CAL_SEED_UP_SCALE`);
/// both tbf buckets (run.sh's 3028 bytes where unset) and the peak rates
/// they send at (`SEED_PEAK_MBIT`, `VIEWER_PEAK_MBIT`, where set:
/// `STRIMS_SIM_CAL_SEED_PEAK_MBIT`, `STRIMS_SIM_CAL_VIEWER_PEAK_MBIT`); a
/// full mesh or the farm's 6-link mesh by name, another topology by its
/// file; each node's
/// access link spec (its `links` file's, `STRIMS_SIM_CAL_SEED_LINK`,
/// `STRIMS_SIM_CAL_VIEWER_LINK` where the viewers' are one, else
/// `STRIMS_SIM_CAL_LINKS`), so a run pairs with the twin of its own links; the chunks
/// and munros the nodes take; and with browsers, each node's stack
/// (`STRIMS_SIM_RTC`, native nodes str0m), its shortest repeating cycle.
/// A setting the twin has no knob for (a seed's loss, another queue) goes
/// in as `TESTBED_<SETTING>`, so the run pairs
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
    // The scheduler's logs the nodes kept, which the twin keeps too: its
    // knobs are the sim's.
    for k in ["FORECAST", "SPEC", "PARENT_CHECKS"] {
        match get(k).map(str::trim) {
            None | Some("" | "0") => {}
            Some("1") => put(&format!("STRIMS_SIM_{k}"), "1".to_owned()),
            Some(v) => return Err(format!("{k}={v}: 1 or 0")),
        }
    }
    let viewers: usize = get("VIEWERS")
        .unwrap_or("3")
        .parse()
        .map_err(|e| format!("VIEWERS: {e}"))?;
    let seed_up: f64 = get("SEED_UP_MBIT")
        .unwrap_or("10")
        .parse()
        .map_err(|e| format!("SEED_UP_MBIT: {e}"))?;
    // The twin's uplinks are whole Mbit/s (Calibration's).
    for k in ["SEED_UP_MBIT", "VIEWER_UP_MBIT"] {
        if let Some(v) = get(k)
            && v.parse::<u64>().is_err()
        {
            return Err(format!("{k}={v}: the twin's uplinks are whole Mbit/s"));
        }
    }
    // The nodes that kept their timers (TIMER_LOG: `all`, or node
    // indices of the run's), whose hosts the twin's log keeps too, by
    // number (`hosts:...`, which no bare number reads as): the twin's
    // other hosts (a relay) keep none, as the testbed's don't.
    if let Some(v) = get("TIMER_LOG").map(str::trim).filter(|v| !v.is_empty()) {
        let mut hosts = std::collections::BTreeSet::new();
        for w in v.split_whitespace() {
            if w == "all" {
                hosts.extend(0..=viewers);
                continue;
            }
            let i = w
                .parse::<usize>()
                .ok()
                .filter(|i| i.to_string() == w && *i <= viewers)
                .ok_or_else(|| {
                    format!("TIMER_LOG={v}: all, or node indices of the run's 0 to {viewers}")
                })?;
            hosts.insert(i);
        }
        put("STRIMS_SIM_TIMER_LOG", HostSet::Only(hosts).knob());
    }
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
        ("SLOW_DOWN_KBIT", ""),
    ] {
        if let Some(v) = get(k).filter(|&v| v != default) {
            put(&format!("STRIMS_SIM_CAL_{k}"), v.to_owned());
        }
    }
    // When the viewers join, in whole seconds as the tiers' cells give it,
    // else in ms (the twin takes both), to the ms run.sh's --join-at takes.
    // 0 however it's written (0.0, 00, or under half a ms) is run.sh's
    // default, as unset is: no knob.
    if let Some(v) = get("JOIN_AT").filter(|&v| !v.is_empty()) {
        let s: f64 = v.parse().map_err(|e| format!("JOIN_AT={v}: {e}"))?;
        if !s.is_finite() || s < 0.0 {
            return Err(format!("JOIN_AT={v}: seconds, 0 or more"));
        }
        let ms = (s * 1000.0).round() as u64;
        if ms > 0 && ms.is_multiple_of(1000) {
            put("STRIMS_SIM_CAL_JOIN_AT", (ms / 1000).to_string());
        } else if ms > 0 {
            put("STRIMS_SIM_CAL_JOIN_AT_MS", ms.to_string());
        }
    }
    // Each viewer's own join (JOINS), to the ms --join-at takes, as the
    // twin's knob takes it: whole seconds as `2`, the rest as `2.25`.
    if let Some(j) = get("JOINS").filter(|j| !j.trim().is_empty()) {
        let (mut joins, mut named) = (Vec::new(), Vec::new());
        for pair in j.split_whitespace() {
            let parsed = pair.split_once(':').and_then(|(i, s)| {
                let (i, s): (usize, f64) = (i.parse().ok()?, s.parse().ok()?);
                (s.is_finite() && s >= 0.0).then(|| (i, (s * 1000.0).round() as u64))
            });
            match parsed {
                Some((i, ms)) if (1..=viewers).contains(&i) && !named.contains(&i) => {
                    named.push(i);
                    joins.push(format!("{i}:{}", num(ms as f64 / 1000.0)));
                }
                _ => {
                    return Err(format!(
                        "JOINS: {pair} isn't <viewer 1..={viewers}>:<seconds>, once a viewer"
                    ));
                }
            }
        }
        put("STRIMS_SIM_CAL_JOINS", joins.join(" "));
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
    // A bucket's peak rate (tbf's peakrate), which the twin's bucket sends
    // at as run.sh's does; run.sh refused one under its uplink's rates.
    for k in ["SEED_PEAK_MBIT", "VIEWER_PEAK_MBIT"] {
        if let Some(v) = get(k).filter(|v| !v.trim().is_empty()) {
            let mbit: f64 = v.parse().map_err(|e| format!("{k}={v}: {e}"))?;
            put(&format!("STRIMS_SIM_CAL_{k}"), num(mbit));
        }
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
        // Another topology keys by its links, not its file's path, and
        // has no twin: the twin reads a topology from a file of its own
        // host (the run's report reads the run's, Run::scenario).
        let mut h = Sha256::new();
        for (a, b) in &links {
            h.update(format!("{a} {b}\n"));
        }
        let mut digest = String::new();
        for b in h.finalize().iter().take(8) {
            let _ = write!(digest, "{b:02x}");
        }
        put("TESTBED_TOPOLOGY", digest);
    }
    let node = flags_of(get("NODE_ARGS").unwrap_or(""))?;
    let browser = flags_of(get("BROWSER_ARGS").unwrap_or(""))?;
    let kinds: Vec<&str> = get("KINDS").unwrap_or("").split_whitespace().collect();
    let kind = |i: usize| kinds.get(i).copied().unwrap_or("native");
    let browsers = (0..=viewers).any(|i| kind(i) != "native");
    let natives = (0..=viewers).any(|i| kind(i) == "native");
    // The chunks and munros: the native nodes' (NODE_ARGS) and the
    // browsers' (BROWSER_ARGS), one where the run has both kinds.
    if natives && browsers {
        for f in ["--chunk", "--munro"] {
            if browser.get(f) != node.get(f) {
                return Err(format!("NODE_ARGS and BROWSER_ARGS differ on {f}"));
            }
        }
    }
    let shape = if natives { &node } else { &browser };
    for (name, args) in [("NODE_ARGS", &node), ("BROWSER_ARGS", &browser)] {
        if let Some(extra) = args
            .keys()
            .find(|f| !matches!(f.as_str(), "--chunk" | "--munro"))
        {
            return Err(format!("{name} {extra}: the twin has no knob for it"));
        }
    }
    if let (Some(c), Some(m)) = (shape.get("--chunk"), shape.get("--munro")) {
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
    put("TESTBED_QDISC", qdisc_of(settings, dir)?);
    // run.sh stamps its own logs (tc-<i>.log, ss-<i>.log, steps.log,
    // compete.log) and reckons its steps' and competing uploads' waits from
    // a clock its settings name (STAMPS). Before it named one it read
    // uutils' `date +%s%3N`, which drops the nanoseconds' leading zeros: in
    // a second's first 100 ms a stamp reads 90-900 ms late, and a wait
    // reckoned from one fires as early. No reading repairs them, so a run
    // that doesn't name the clock (STAMPS=clock), or whose stamps go back
    // anyway, keys apart, its stamped columns as read: TESTBED_STAMPS
    // pairs it with no cell.
    match get("STAMPS") {
        Some(STAMP_CLOCK) if late_stamps(dir) => put("TESTBED_STAMPS", "back".to_owned()),
        Some(STAMP_CLOCK) => {}
        other => put("TESTBED_STAMPS", other.unwrap_or("date").to_owned()),
    }
    // How the nodes made their links at their joins (JOIN_LINKS). A run
    // whose settings don't say `together` made them one at a time, each
    // awaited, in the topology's order, the seed first, where its twin's
    // viewers make them all at once: it keys apart, TESTBED_JOIN_LINKS
    // pairing it with no cell.
    match get("JOIN_LINKS") {
        Some(JOIN_LINKS_TOGETHER) => {}
        other => put("TESTBED_JOIN_LINKS", other.unwrap_or("serial").to_owned()),
    }
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
        // Each viewer's join (JOINS) is a draw of an arrivals profile where
        // the caller names it (STRIMS_SIM_ARRIVALS, as a tier matrix's
        // knobs do), which the twin draws its own of; else the joins
        // themselves, the twin's STRIMS_SIM_CAL_JOINS.
        if knobs.contains_key("STRIMS_SIM_ARRIVALS") {
            knobs.remove("STRIMS_SIM_CAL_JOINS");
        }
        let viewers: usize = knobs
            .get("STRIMS_SIM_CAL_VIEWERS")
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
        let mut nodes = Vec::new();
        for i in 0..=viewers {
            let text = read(&format!("node-{i}.jsonl"))?;
            let lines = text
                .lines()
                .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                .collect::<Vec<_>>();
            // A run with a node that never ran has no tables: it would read
            // as a swarm without that node. Such a node leaves no lines (its
            // arguments refused, a crash at start), none of the records
            // (`rec`: a node from before them), or its harness's error
            // (testbed_node's or testbed_browser's last words). A viewer that
            // ran and read nothing is a result, not one of these.
            let err = read(&format!("node-{i}.err")).unwrap_or_default();
            let fatal = err
                .lines()
                .find(|l| l.starts_with("testbed_node: ") || l.starts_with("testbed_browser: "));
            if let Some(why) = fatal {
                return Err(format!("node {i}'s harness failed: {why}"));
            }
            if lines.is_empty() {
                let why = err.lines().find(|l| !l.trim().is_empty());
                return Err(format!(
                    "node-{i}.jsonl has no lines: {}",
                    why.map_or_else(|| format!("node-{i}.err says nothing"), str::to_owned)
                ));
            }
            if !lines.iter().any(|l| l.get("rec").is_some()) {
                return Err(format!(
                    "node-{i}.jsonl has no records (`rec`): a node from before them"
                ));
            }
            nodes.push(lines);
        }
        let start_ms = nodes
            .iter()
            .flatten()
            .find_map(|l| Some(l.get("t_ms")?.as_i64()? - l.get("rel_ms")?.as_i64()?))
            .and_then(|s| u64::try_from(s).ok())
            .ok_or("no node line names the stream's start")?;
        check_samplers(dir, &settings, &nodes)?;
        let starts = competing_starts(dir);
        if let Some(n) = starts.iter().position(|c| c.failed) {
            knobs.insert(
                "TESTBED_SCHEDULE".to_owned(),
                format!("competing upload {} failed", n + 1),
            );
        } else if let Some(why) = off_schedule(dir, &settings, start_ms) {
            knobs.insert("TESTBED_SCHEDULE".to_owned(), why);
        }
        // The competing uploads' starts as recorded, for the twin: each
        // client's (its report's timestamp), ms of stream; the twin adds
        // the setup's round trips. The keying sets them aside (simlab's
        // testbed MEASURED). A failed one is no upload, and a run whose
        // every upload failed says so (`none`), so its knobs' schedule
        // (STRIMS_SIM_CAL_COMPETE) starts none in a twin.
        let competing: Vec<String> = starts
            .iter()
            .filter(|c| !c.failed)
            .map(|c| format!("{}:{}", c.client().saturating_sub(start_ms), c.secs))
            .collect();
        if !starts.is_empty() {
            let ms = if competing.is_empty() {
                "none".to_owned()
            } else {
                competing.join(" ")
            };
            knobs.insert("STRIMS_SIM_CAL_COMPETE_MS".to_owned(), ms);
        }
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

    /// The run time of Unix µs `us`, as [`Run::at`].
    fn at_us(&self, us: u64) -> Option<Duration> {
        let run0 = self.start_ms.checked_sub(LEAD.as_millis() as u64)?;
        us.checked_sub(run0 * 1000).map(Duration::from_micros)
    }

    fn file(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.dir.join(name)).ok()
    }

    /// The twin's scenario, with the times the testbed measured.
    fn scenario(&self) -> Scenario {
        // A topology the knobs key by its links alone is the run's file.
        let topology = self
            .knobs
            .contains_key("TESTBED_TOPOLOGY")
            .then(|| self.dir.join("topology").to_string_lossy().into_owned());
        let cal = Calibration::from_knobs(|k| {
            if k == "STRIMS_SIM_CAL_TOPOLOGY" && topology.is_some() {
                return topology.clone();
            }
            self.knobs.get(k).cloned()
        });
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
        // Each competing upload from its first data (Started::data), for
        // as long as it was asked: what the seed's link carried of it.
        let starts = competing_starts(&self.dir);
        if !starts.is_empty()
            && let Some(h) = s.hosts.get_mut(seed)
        {
            let rtt = h.competing.first().map(|c| c.rtt).unwrap_or_default();
            h.competing = starts
                .iter()
                .filter(|c| !c.failed)
                .filter_map(|c| {
                    let start = self.at(c.data(rtt))?;
                    Some(Competing {
                        start,
                        stop: start + Duration::from_secs_f64(c.secs),
                        rtt,
                    })
                })
                .collect();
        }
        // The logs the nodes kept, which the tables write as the twin's.
        let on = |k: &str| self.knobs.get(k).is_some_and(|v| v == "1");
        s.forecast_log = on("STRIMS_SIM_FORECAST");
        s.request_logs = on("STRIMS_SIM_SPEC");
        s.parent_checks = on("STRIMS_SIM_PARENT_CHECKS");
        s.timer_log = self
            .knobs
            .get("STRIMS_SIM_TIMER_LOG")
            .and_then(|v| HostSet::parse(v).ok().flatten());
        // The arrivals profile JOINS drew (a tier matrix's knob), as the
        // twin's row names it; the times are the run's.
        if let Some(a) = self
            .knobs
            .get("STRIMS_SIM_ARRIVALS")
            .and_then(|a| a.parse().ok())
        {
            s.arrivals = a;
        }
        if let Some(joins) = self.settings.get("JOINS") {
            for j in joins.split_whitespace() {
                if let Some((i, at)) = j.split_once(':')
                    && let (Ok(i), Ok(at)) = (i.parse::<usize>(), at.parse::<f64>())
                    && i > 0
                    && let Some(h) = s.hosts.get_mut(i)
                {
                    // To the ms, as --join-at takes it.
                    h.arrive_at =
                        Some(LEAD + Duration::from_millis((at.max(0.0) * 1000.0).round() as u64));
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

    /// When host `h` started reading: its arrival, its last applied restart
    /// (of `s`'s time-ordered faults less `skipped`), or the run's start.
    fn joined(s: &Scenario, skipped: &[usize], h: HostIx) -> Duration {
        s.hosts.get(h).and_then(|x| x.arrive_at).unwrap_or_else(|| {
            s.faults
                .iter()
                .enumerate()
                .filter(|(i, f)| {
                    !skipped.contains(i) && matches!(f.fault, Fault::Restart(x) if x == h)
                })
                .map(|(_, f)| f.at)
                .max()
                .unwrap_or_default()
        })
    }

    /// The run's report (see the [module docs](self)).
    pub fn report(&self) -> Report {
        let mut scenario = self.scenario();
        // As a run orders them, and with the run's skips, so the tables
        // read the applied faults (Report::applied_faults, departure) as
        // the twin's do.
        scenario.faults.sort_by_key(|f| f.at);
        let skipped_faults = skipped_faults(&scenario);
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
        let mut parent_checks = Vec::new();
        let swarm = scenario.swarms.first();
        let seed = swarm.map_or(0, |s| s.seed);
        // The logs' times are from the stream's start (record::logs), and
        // the report's from the run's, which every node's clock shares.
        let unix = |ms: u64| Time::from_unix_millis(i64::try_from(ms).unwrap_or(i64::MAX));
        let origin = unix(self.start_ms);
        let epoch = unix(self.start_ms.saturating_sub(LEAD.as_millis() as u64));
        for (i, lines) in self.nodes.iter().enumerate() {
            let Some(host) = hosts.get_mut(i) else {
                continue;
            };
            let mut stats = ViewerStats::default();
            let is_viewer = swarm.is_some_and(|s| s.viewers.contains(&i));
            if is_viewer {
                stats.start(Self::joined(&scenario, &skipped_faults, i).as_millis() as u64);
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
                for c in rec
                    .get("checks")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if let Some(r) = self.check_of(i, c) {
                        parent_checks.push(r);
                    }
                }
                host.parent_checks_lost +=
                    rec.get("checks_lost").and_then(Value::as_u64).unwrap_or(0);
                if let Some(logs) = rec.get("logs").and_then(|v| Logs::parse(v, origin)) {
                    add_logs(host, logs, epoch);
                }
                if let Some((rows, lost, context)) =
                    rec.get("timers").and_then(|v| timers_of(v, origin))
                {
                    host.timers
                        .extend(rows.into_iter().map(|r| TimerHostRecord::of(r, epoch)));
                    host.timers_lost += lost;
                    host.timer_context = Some(context);
                }
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
        parent_checks.sort_by_key(|c: &ParentCheckRecord| (c.at, c.node, c.stream));
        let mut written = SeqSet::default();
        if let Some(s) = swarm {
            for seq in s.frames_between(s.start, s.stop) {
                written.insert(seq);
            }
        }
        let mut net = Network::read(self, &scenario, seed);
        packets::fill(self, &scenario, &mut net.wan, &mut hosts);
        // The meter's truth's share at each sample, from the captures.
        let run0_us = self.start_ms.saturating_sub(LEAD.as_millis() as u64) * 1000;
        for (h, host) in hosts.iter_mut().enumerate() {
            let (Some(spec), Some(sends)) = (scenario.hosts.get(h), net.sends.get(h)) else {
                continue;
            };
            let log = net
                .wan
                .hosts
                .iter()
                .find(|w| w.name == spec.name)
                .map_or(&[][..], |w| &w.competing_log[..]);
            gate(&mut host.runner.meter, sends, spec, log, |d| {
                run0_us + d.as_micros() as u64
            });
        }
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
            wire_ends: net.ends,
            sim: SIM,
            sockets: (0..self.nodes.len())
                .flat_map(|i| {
                    self.file(&format!("ss-{i}.log"))
                        .map(|text| sockets_of(self, i, &text))
                        .unwrap_or_default()
                })
                .collect(),
            parent_losses: Vec::new(),
            parent_changes,
            parent_checks,
            loops: None,
            skipped_faults,
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
            // A node runs one incarnation: its channel and vnic bytes are
            // the run's.
            channel_bytes: u("channel_bytes"),
            vnic_bytes: u("bytes_written").unwrap_or(0),
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

    /// A line's parent check (`strims_testbed::node`'s `rec.checks`).
    fn check_of(&self, node: HostIx, c: &Value) -> Option<ParentCheckRecord> {
        let host = |k: usize| c.get(k).and_then(Value::as_u64).map(|x| x as usize);
        let lag = |k: usize| {
            c.get(k)
                .and_then(Value::as_i64)
                .map(strims_base::time::Duration::from_micros)
        };
        let n = |k: usize| c.get(k).and_then(Value::as_u64);
        let action = c.get(2)?.as_str()?;
        Some(ParentCheckRecord {
            at: self.at_us(c.get(0)?.as_u64()?)?,
            node,
            stream: u16::try_from(c.get(1)?.as_u64()?).ok()?,
            action: ParentAction::ALL.into_iter().find(|a| a.name() == action)?,
            best: host(3),
            best_lag: lag(4),
            runner_up: host(5),
            runner_up_lag: lag(6),
            parent: host(7),
            parent_have: lag(8),
            parent_push: lag(9),
            unready: n(10)?,
            subscriber: n(11)?,
            backed_off: n(12)?,
            stale: n(13)?,
            behind: n(14)?,
            votes: n(15)?,
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

/// A line's logs ([`Logs`]) into its node's report, as a simulated node's
/// sample hands its own (the world's `observe_bins`), by the same
/// records: each chunk it first held (the forecast table's truth and the
/// arrivals), its forecast samples, HAVEs, requests and seed log, their
/// times true time from the run's start (the node's clock read `epoch`
/// then: every container's clock is the host's), and the records the caps
/// left out.
fn add_logs(host: &mut HostReport, logs: Logs, epoch: Time) {
    for (peer, f) in logs.forecasts {
        host.forecasts.push(ForecastRecord::of(f, peer, epoch));
    }
    for (peer, parent, asked, held) in logs.held {
        let a = ArrivalHostRecord::of(held, peer, parent, asked, epoch);
        host.held.push((held.bin.base_offset(), a.at));
        host.arrivals.push(a);
    }
    for (peer, have) in logs.haves {
        host.haves.push(HaveHostRecord::of(have, peer, epoch));
    }
    for (peer, rivals, record) in logs.asks {
        host.requests.push(RequestHostRecord {
            epoch,
            peer,
            rivals,
            record,
        });
    }
    for (peer, d) in logs.seed_decisions {
        host.seed_decisions
            .push(SeedDecisionHostRecord::of(d, peer, epoch));
    }
    for (peer, s) in logs.seed_sends {
        host.seed_sends.push(SeedSendHostRecord::of(s, peer, epoch));
    }
    host.forecasts_lost += logs.lost.forecasts;
    host.held_lost += logs.lost.held;
    host.haves_lost += logs.lost.haves;
    host.requests_lost += logs.lost.requests;
    host.seed_log_lost += logs.lost.seed_log;
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

/// A rate as ss prints it (`93.9Mbps`, `3728280bps`), in bits per second.
fn ss_rate(v: &str) -> Option<u64> {
    let v = v.strip_suffix("bps")?;
    let (n, mult) = match v.chars().last()? {
        'K' => (v.get(..v.len() - 1)?, 1e3),
        'M' => (v.get(..v.len() - 1)?, 1e6),
        'G' => (v.get(..v.len() - 1)?, 1e9),
        _ => (v, 1.0),
    };
    Some((n.parse::<f64>().ok()? * mult) as u64)
}

/// Why a run's steps or competing uploads came off their schedule, if they
/// did: each one's logged time (steps.log's, compete.log's) against run.sh's
/// stream start plus its seconds (and the seed's delay, a step's), early
/// at all (a wait fired from a late clock) or late by more than
/// SCHEDULE_SLACK_MS, or not logged. Such a run keys apart
/// (TESTBED_SCHEDULE), as its twin's schedule isn't the run's.
fn off_schedule(dir: &Path, settings: &BTreeMap<String, String>, start_ms: u64) -> Option<String> {
    let delay: u64 = settings
        .get("SEED_DELAY_MS")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let ats = |key: &str| -> Vec<f64> {
        settings.get(key).map_or_else(Vec::new, |v| {
            v.split_whitespace()
                .filter_map(|p| p.split_once(':')?.0.parse().ok())
                .collect()
        })
    };
    let logged = |name: &str| -> Vec<u64> {
        std::fs::read_to_string(dir.join(name)).map_or_else(
            |_| Vec::new(),
            |t| {
                t.lines()
                    .filter_map(|l| l.split_whitespace().next()?.parse().ok())
                    .collect()
            },
        )
    };
    // A competing upload is on time by its client's start (its report's
    // timestamp), which its twin starts at.
    let competing: Vec<u64> = competing_starts(dir).iter().map(Started::client).collect();
    for (what, key, got, extra) in [
        ("step", "SEED_UP_STEPS", logged("steps.log"), delay),
        ("competing upload", "COMPETE", competing, 0),
    ] {
        let due = ats(key);
        for (i, at) in due.iter().enumerate() {
            let due = start_ms + (at * 1000.0) as u64 + extra;
            let Some(&t) = got.get(i) else {
                return Some(format!("{what} {} not logged", i + 1));
            };
            if t < due {
                return Some(format!("{what} {} {} ms early", i + 1, due - t));
            }
            if t > due + SCHEDULE_SLACK_MS {
                return Some(format!("{what} {} {} ms late", i + 1, t - due));
            }
        }
    }
    None
}

/// The smallest TCP payload a competing upload's data segment carries:
/// iperf3's control messages (its cookie, its parameters' JSON) are under
/// it, its data's full segments over it.
const COMPETE_DATA_PAYLOAD: u64 = 512;

/// The faults of `s` (time-ordered, as a run sorts them) a run of it skips
/// (Report::skipped_faults), by the run's own rules (run.rs's `apply`): a
/// crash of a host that isn't there (before its arrival, after its
/// departure) or is down, a restart of a host that isn't there or isn't
/// down, a host's second departure, and every fault at or past the run's
/// end, which it never reaches. A testbed run has the WAN model, so a
/// one-way latency applies. The testbed's nodes did what the scenario says
/// (their joins and links are run.sh's), so its skips are its twin's.
fn skipped_faults(s: &Scenario) -> Vec<usize> {
    let n = s.hosts.len();
    let (mut down, mut departed) = (vec![false; n], vec![false; n]);
    let absent = |h: usize, at: Duration, departed: &[bool]| {
        s.hosts
            .get(h)
            .and_then(|x| x.arrive_at)
            .is_some_and(|a| at < a)
            || departed.get(h).copied().unwrap_or(false)
    };
    let mut out = Vec::new();
    for (i, f) in s.faults.iter().enumerate() {
        let skip = !s.reaches(f.at)
            || match f.fault {
                Fault::Crash(h) => {
                    let skip = down.get(h).copied().unwrap_or(false) || absent(h, f.at, &departed);
                    if !skip && let Some(d) = down.get_mut(h) {
                        *d = true;
                    }
                    skip
                }
                Fault::Restart(h) => {
                    let skip = absent(h, f.at, &departed) || !down.get(h).copied().unwrap_or(false);
                    if !skip && let Some(d) = down.get_mut(h) {
                        *d = false;
                    }
                    skip
                }
                Fault::Depart(h, _) => {
                    let skip = departed.get(h).copied().unwrap_or(false);
                    if let Some(d) = departed.get_mut(h) {
                        *d = true;
                    }
                    skip
                }
                _ => false,
            };
        if skip {
            out.push(i);
        }
    }
    out
}

/// iperf3's setup ahead of its data, in its path's round trips: its data
/// stream's handshake and the server's TEST_START and TEST_RUNNING (the
/// bench measured 93-178 ms, 2.4-3.1 of them).
const COMPETE_SETUP_TRIPS: u32 = 3;

/// One competing upload of a testbed run, as it was recorded: when run.sh
/// launched it, when iperf3's report says it started, when its first data
/// left (Unix ms), and the seconds it was asked to send.
struct Started {
    launch: u64,
    report: Option<u64>,
    data: Option<u64>,
    secs: f64,
    /// It failed: its report is missing, unreadable or says iperf3's
    /// `error` (which iperf3 may report after it sent some data), or the
    /// seed's capture, running to the upload's end or past it, holds no
    /// data of it.
    failed: bool,
}

impl Started {
    /// When the upload's client started: its report's timestamp, else its
    /// launch. A twin starts its competitor there and adds the setup's
    /// round trips itself (`STRIMS_SIM_CAL_COMPETE_MS`), and the schedule
    /// is judged by it.
    fn client(&self) -> u64 {
        self.report.unwrap_or(self.launch)
    }

    /// When its data started: its first data segment, else its client's
    /// start and the setup's round trips of `rtt`.
    fn data(&self, rtt: Duration) -> u64 {
        self.data
            .unwrap_or_else(|| self.client() + (rtt * COMPETE_SETUP_TRIPS).as_millis() as u64)
    }
}

/// Each competing upload in out dir `dir` (compete.log's lines, run.sh's
/// launch of each, `<unix ms> <at> <secs>`), as recorded (and whether it
/// failed, [`Started::failed`]: its report missing, unreadable or iperf3's
/// `error`, or none of its data in a capture that ran to its end): iperf3's
/// report's timestamp (compete-<at>.json's start.timestamp.timemillisecs:
/// the client's start, 150-215 ms after the launch, before its setup's
/// round trips) and, with PCAP, its first data, the seed's capture's
/// first segment to the sink of at least COMPETE_DATA_PAYLOAD bytes after
/// the launch (pcap-0.pcap). A capture that stopped while the seed ran
/// (run.sh's pcap-0.end) before an upload's end can't say its data never
/// came.
fn competing_starts(dir: &Path) -> Vec<Started> {
    let Ok(log) = std::fs::read_to_string(dir.join("compete.log")) else {
        return Vec::new();
    };
    let pcap = std::fs::read(dir.join("pcap-0.pcap"))
        .ok()
        .and_then(|b| packets(&b));
    let pcap_end: Option<u64> = std::fs::read_to_string(dir.join("pcap-0.end"))
        .ok()
        .and_then(|t| t.split_whitespace().next()?.parse().ok());
    let seed = u8::try_from(10).ok();
    log.lines()
        .filter_map(|l| {
            let mut w = l.split_whitespace();
            let launch: u64 = w.next()?.parse().ok()?;
            let at = w.next()?;
            let secs: f64 = w.next()?.parse().ok()?;
            let data = pcap
                .iter()
                .flatten()
                .find(|p| {
                    p.us >= launch * 1000
                        && p.src == seed
                        && p.dst == Some(SINK)
                        && p.payload >= COMPETE_DATA_PAYLOAD
                })
                .map(|p| p.us / 1000);
            let json = std::fs::read_to_string(dir.join(format!("compete-{at}.json")))
                .ok()
                .and_then(|text| serde_json::from_str::<Value>(&text).ok());
            let report = json
                .as_ref()
                .and_then(|v| v.pointer("/start/timestamp/timemillisecs")?.as_u64());
            let end = launch + (secs * 1000.0) as u64;
            let watched = pcap.is_some() && pcap_end.is_none_or(|ms| ms >= end);
            let failed = json.as_ref().is_none_or(|v| v.get("error").is_some())
                || (watched && data.is_none());
            Some(Started {
                launch,
                report,
                data,
                secs,
                failed,
            })
        })
        .collect()
}

/// Whether a sampler's stamps in out dir `dir` go back (a tc-<i>.log
/// line's first word, an ss-<i>.log `@` line's), as run.sh's uutils
/// `date +%s%3N` stamps do in about a tenth of a second's samples.
fn late_stamps(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|e| {
        let name = e.file_name().to_string_lossy().into_owned();
        let ss = name.starts_with("ss-");
        if !(name.ends_with(".log") && (ss || name.starts_with("tc-"))) {
            return false;
        }
        let Ok(text) = std::fs::read_to_string(e.path()) else {
            return false;
        };
        let stamps: Vec<u64> = text
            .lines()
            .filter_map(|l| {
                let l = if ss { l.strip_prefix("@ ")? } else { l };
                l.split_whitespace().next()?.parse().ok()
            })
            .collect();
        stamps.windows(2).any(|w| w.get(1) < w.first())
    })
}

/// A node's TCP sockets as its ss log has them (`ss -tin` once a
/// second, each sample after a `@ <Unix ms>` line), peers by their index in
/// the testbed's subnet, the sink's and the relay's none.
fn sockets_of(run: &Run, node: HostIx, text: &str) -> Vec<SocketSample> {
    let mut out = Vec::new();
    let mut at = None;
    let mut peer = None;
    for line in text.lines() {
        if let Some(ms) = line.strip_prefix("@ ") {
            at = ms.trim().parse::<u64>().ok().and_then(|ms| run.at(ms));
            continue;
        }
        if !line.starts_with(char::is_whitespace) {
            // A socket's line (`state established` leaves out the
            // state): its queues, local and peer addresses.
            let w: Vec<&str> = line.split_whitespace().collect();
            let host = |a: &str| {
                let ip = a.rsplit_once(':')?.0;
                let o: Vec<u8> = ip.split('.').filter_map(|x| x.parse().ok()).collect();
                (o.get(..3) == Some(&SUBNET[..]))
                    .then(|| o.get(3).copied())
                    .flatten()
            };
            peer = match (w.first().and_then(|x| x.parse::<u64>().ok()), w.get(3)) {
                (Some(_), Some(p)) => {
                    Some(host(p).and_then(|o| o.checked_sub(10)).map(usize::from))
                }
                _ => None,
            };
            continue;
        }
        // A socket to no swarm host (the competing upload's two to the
        // sink, the relay's) has no peer, the table's key: left out.
        let (Some(t), Some(Some(peer))) = (at, peer.take()) else {
            continue;
        };
        if peer >= run.nodes.len() {
            continue;
        }
        let mut kv: BTreeMap<&str, &str> = BTreeMap::new();
        let words: Vec<&str> = line.split_whitespace().collect();
        for (i, w) in words.iter().enumerate() {
            match w.split_once(':') {
                Some((k, v)) => {
                    kv.insert(k, v);
                }
                None => {
                    if let Some(v) = words.get(i + 1) {
                        kv.insert(w, v);
                    }
                }
            }
        }
        let n = |k: &str| kv.get(k).and_then(|v| v.parse::<u64>().ok());
        let us = |v: &str| v.parse::<f64>().ok().map(|ms| (ms * 1000.0) as u64);
        let (rtt, rttvar) = kv
            .get("rtt")
            .and_then(|v| v.split_once('/'))
            .map_or((None, None), |(a, b)| (us(a), us(b)));
        out.push(SocketSample {
            t,
            node,
            peer: Some(peer),
            rtt_us: rtt,
            rttvar_us: rttvar,
            min_rtt_us: kv.get("minrtt").and_then(|v| us(v)),
            cwnd_segments: n("cwnd"),
            ssthresh_segments: n("ssthresh"),
            bytes_sent: n("bytes_sent"),
            bytes_acked: n("bytes_acked"),
            bytes_received: n("bytes_received"),
            notsent_bytes: n("notsent").or(Some(0)),
            segs_out: n("segs_out"),
            segs_in: n("segs_in"),
            data_segs_out: n("data_segs_out"),
            data_segs_in: n("data_segs_in"),
            retrans_segments: kv
                .get("retrans")
                .and_then(|v| v.rsplit('/').next())
                .and_then(|v| v.parse().ok())
                .or(Some(0)),
            delivery_rate_bps: kv
                .get("delivery_rate")
                .and_then(|v| ss_rate(v))
                .map(|b| b / 8),
            pacing_rate_bps: kv
                .get("pacing_rate")
                .and_then(|v| ss_rate(v))
                .map(|b| b / 8),
        });
    }
    out
}

/// One packet a node's capture holds: when (Unix µs), its source and
/// destination's last octets in the testbed's subnet, its IP length,
/// whether it is a pure TCP acknowledgement (no payload, no SYN, FIN or
/// RST), its transport's payload bytes (TCP's or UDP's), a TCP segment's
/// ports and sequence number, its IP id, and a UDP datagram's ports.
struct Packet {
    us: u64,
    src: Option<u8>,
    dst: Option<u8>,
    ip_len: u64,
    pure_ack: bool,
    payload: u64,
    tcp: Option<(u16, u16, u32)>,
    ip_id: u16,
    udp: Option<(u16, u16)>,
}

/// A libpcap file's frames over Ethernet, each its time (Unix µs), its
/// length on the wire and the bytes captured; `None` for another format.
fn frames(bytes: &[u8]) -> Option<Vec<(u64, u32, &[u8])>> {
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
    while let (Some(sec), Some(frac), Some(len), Some(wire)) = (
        word(bytes, at, le),
        word(bytes, at + 4, le),
        word(bytes, at + 8, le),
        word(bytes, at + 12, le),
    ) {
        let data = bytes.get(at + 16..at + 16 + len as usize)?;
        at += 16 + len as usize;
        let us = u64::from(sec) * 1_000_000 + u64::from(if nanos { frac / 1000 } else { frac });
        out.push((us, wire, data));
    }
    Some(out)
}

/// A libpcap file's IPv4 packets over Ethernet; `None` for another
/// format.
fn packets(bytes: &[u8]) -> Option<Vec<Packet>> {
    let mut out = Vec::new();
    for (us, _, data) in frames(bytes)? {
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
        let be16 = |at: usize| {
            ip.get(at..at + 2)
                .and_then(|b| <[u8; 2]>::try_from(b).ok())
                .map(u16::from_be_bytes)
        };
        let (payload, tcp) = match proto {
            6 => {
                let thl = ip.get(ihl + 12).map_or(0, |&off| u64::from(off >> 4) * 4);
                let seq = ip
                    .get(ihl + 4..ihl + 8)
                    .and_then(|b| <[u8; 4]>::try_from(b).ok())
                    .map(u32::from_be_bytes);
                let ports = be16(ihl).zip(be16(ihl + 2));
                (
                    ip_len.saturating_sub(ihl as u64 + thl),
                    ports.zip(seq).map(|((a, b), q)| (a, b, q)),
                )
            }
            17 => (ip_len.saturating_sub(ihl as u64 + 8), None),
            _ => (0, None),
        };
        out.push(Packet {
            us,
            src: host(12),
            dst: host(16),
            ip_len,
            pure_ack,
            payload,
            tcp,
            ip_id: be16(4).unwrap_or(0),
            udp: (proto == 17)
                .then(|| be16(ihl).zip(be16(ihl + 2)))
                .flatten(),
        });
    }
    Some(out)
}

/// A TCP flow: its destination and ports.
type Flow = (Option<u8>, u16, u16);

/// The most a segment the node offered its link carries: a GSO buffer
/// (64 KiB), which the queue's device splits into the link's segments.
const OFFER_MAX: u64 = 65536;

/// A node's offers to its link (its offered capture's data segments, in
/// capture order), by flow and then by the offset of their first byte in
/// it from the flow's first offer's (sequence numbers wrap): each offer's
/// length, when it was offered, and whether it resent bytes the flow had
/// offered before.
struct Offers {
    first: BTreeMap<Flow, u32>,
    by: BTreeMap<Flow, FlowOffers>,
}

/// A flow's offers by their first byte's offset: (length, µs, resent).
type FlowOffers = BTreeMap<u32, Vec<(u64, u64, bool)>>;

impl Offers {
    fn of(ps: &[&Packet]) -> Offers {
        let mut o = Offers {
            first: BTreeMap::new(),
            by: BTreeMap::new(),
        };
        for (p, again) in ps.iter().zip(resends(ps.iter().copied())) {
            let Some((sp, dp, seq)) = p.tcp else {
                continue;
            };
            let flow = (p.dst, sp, dp);
            let first = *o.first.entry(flow).or_insert(seq);
            o.by.entry(flow)
                .or_default()
                .entry(seq.wrapping_sub(first))
                .or_default()
                .push((p.payload, p.us, again));
        }
        o
    }

    /// Whether the latest offer at or before the link carried segment `p`
    /// whose bytes hold all of `p`'s resent them (one GSO buffer offered
    /// holds several carried segments); `None` where no offer holds them.
    fn resent(&self, p: &Packet) -> Option<bool> {
        let (sp, dp, seq) = p.tcp?;
        let flow = (p.dst, sp, dp);
        let at = seq.wrapping_sub(*self.first.get(&flow)?);
        let by = self.by.get(&flow)?;
        by.range(..=at)
            .rev()
            .take_while(|&(&start, _)| u64::from(at - start) < OFFER_MAX)
            .flat_map(|(&start, offers)| {
                offers.iter().filter(move |&&(len, us, _)| {
                    us <= p.us && u64::from(at - start) + p.payload <= len
                })
            })
            .max_by_key(|&&(_, us, _)| us)
            .map(|&(_, _, again)| again)
    }
}

/// Whether each of a node's data segments `ps` (in capture order) resent
/// bytes its flow had sent: its sequence number under the highest its flow
/// had sent, even where no gap showed (a tail-loss probe resends the last
/// segment).
fn resends<'a>(ps: impl Iterator<Item = &'a Packet>) -> Vec<bool> {
    let mut sent: BTreeMap<(Option<u8>, u16, u16), u32> = BTreeMap::new();
    ps.map(|p| {
        p.tcp.is_some_and(|(sp, dp, seq)| {
            let end = seq.wrapping_add(p.payload as u32);
            let max = sent.entry((p.dst, sp, dp)).or_insert(seq);
            let again = (seq.wrapping_sub(*max) as i32) < 0;
            if (end.wrapping_sub(*max) as i32) > 0 {
                *max = end;
            }
            again
        })
    })
    .collect()
}

/// Whether each of a node's TCP segments `ps` (its link's, in capture
/// order) resent bytes its flow's link had carried before: any of its
/// bytes among what an earlier segment of its flow carried. A segment that
/// fills a hole (its first sending lost before the link, at the node's own
/// queue) isn't one, though its sequence number is under the flow's
/// highest; a tail-loss probe's or a timeout's resend of a segment the
/// link had carried is. Offsets are from each flow's first segment
/// (sequence numbers wrap); a datagram is none.
fn carried_again<'a>(ps: impl Iterator<Item = &'a Packet>) -> Vec<bool> {
    // Each flow's first sequence number and its carried byte ranges,
    // merged: start -> end.
    let mut flows: BTreeMap<Flow, (u32, BTreeMap<u64, u64>)> = BTreeMap::new();
    ps.map(|p| {
        let Some((sp, dp, seq)) = p.tcp else {
            return false;
        };
        let (first, carried) = flows
            .entry((p.dst, sp, dp))
            .or_insert((seq, BTreeMap::new()));
        let start = u64::from(seq.wrapping_sub(*first));
        let end = start + p.payload;
        if end == start {
            return false;
        }
        // The last range starting before the segment's end reaches into it.
        let again = carried
            .range(..end)
            .next_back()
            .is_some_and(|(_, &e)| e > start);
        // Merge it with every range it touches.
        let (mut lo, mut hi) = (start, end);
        let touching: Vec<u64> = carried
            .range(..=end)
            .rev()
            .take_while(|&(_, &e)| e >= start)
            .map(|(&s, _)| s)
            .collect();
        for s in touching {
            if let Some(e) = carried.remove(&s) {
                lo = lo.min(s);
                hi = hi.max(e);
            }
        }
        carried.insert(lo, hi);
        again
    })
    .collect()
}

/// Node `h`'s access link spec and the device whose root is its modem, from
/// out dir `dir`'s `links` file (`node <i> <spec> <up_mbit> <delay_ms>
/// <queue_ms> [<modem_dev>]`, run.sh's): a run from before it wrote the
/// file had a tbf on eth0 alone, so tbf and no device.
fn link_line(dir: &Path, h: usize) -> Result<(String, Option<String>), String> {
    let Ok(text) = std::fs::read_to_string(dir.join("links")) else {
        return Ok(("tbf".to_owned(), None));
    };
    text.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>())
        .find(|w| w.first() == Some(&"node") && w.get(1) == Some(&h.to_string().as_str()))
        .and_then(|w| Some(((*w.get(2)?).to_owned(), w.get(6).map(|d| (*d).to_owned()))))
        .ok_or_else(|| format!("links: no node {h}"))
}

/// How far a sampler's last line may fall short of its node's last one
/// (its interval aside): the container ends both, and a node's last line
/// is a second's.
const SAMPLER_SLACK_MS: u64 = 2000;

/// The run's samplers' logs, checked as [`uplink`] reads them:
/// each node's tc-<i>.log and the seed's ss-0.log, where they exist, must
/// hold samples up to their node's last line (`SAMPLER_SLACK_MS` and an
/// interval short of it), unless the sampler's `<sampler>-<i>.end`
/// (run.sh's: when it exited while its node ran) says why it stopped
/// sooner, and a tc log with samples the node's modem (its link class's
/// qdisc 10: on the device `links` names), whenever its sampler stopped.
/// A run whose sampler failed unseen would read as unmeasured columns.
fn check_samplers(
    dir: &Path,
    settings: &BTreeMap<String, String>,
    nodes: &[Vec<Value>],
) -> Result<(), String> {
    let ss_ms = settings
        .get("SS_EVERY")
        .and_then(|v| v.parse::<f64>().ok())
        .map_or(1000, |s| (s * 1000.0) as u64);
    for (i, lines) in nodes.iter().enumerate() {
        let last = lines
            .iter()
            .filter_map(|l| l.get("t_ms")?.as_u64())
            .max()
            .unwrap_or(0);
        let logs = [("tc", "tcsample", 100), ("ss", "sssample", ss_ms)];
        for (log, sampler, every) in logs {
            let Ok(text) = std::fs::read_to_string(dir.join(format!("{log}-{i}.log"))) else {
                continue;
            };
            let ended = dir.join(format!("{sampler}-{i}.end")).exists();
            let stamps: Vec<u64> = text
                .lines()
                .filter_map(|l| {
                    let l = if log == "ss" {
                        l.strip_prefix("@ ")?
                    } else {
                        l
                    };
                    l.split_whitespace().next()?.parse().ok()
                })
                .collect();
            match stamps.iter().max() {
                // It exited before its first sample.
                None if ended => continue,
                None => {
                    return Err(format!(
                        "{log}-{i}.log is empty: its sampler wrote nothing ({sampler}-{i}.err)"
                    ));
                }
                Some(&end) if !ended && end + every + SAMPLER_SLACK_MS < last => {
                    return Err(format!(
                        "{log}-{i}.log stops {} ms before node {i}'s last line, with no {sampler}-{i}.end",
                        last - end
                    ));
                }
                Some(_) => {}
            }
            if log == "tc" {
                uplink(dir, i)?;
            }
        }
    }
    Ok(())
}

/// A node's access link queue at one sample of its `tc-<i>.log`
/// ([`uplink`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModemSample {
    /// When it was taken, Unix ms.
    pub ms: u64,
    /// The packets the queue dropped so far.
    pub drops: u64,
    /// What it holds, as its shaper charges it: tc's backlog, or under
    /// cake each packet's network bytes and overhead (`cake_held`).
    pub held: u64,
    /// The rate it drains at, bytes a second, where its qdisc has one (a
    /// trace's prio has none).
    pub rate: Option<u64>,
}

/// A node's access link queue over a run ([`uplink`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Uplink {
    /// Its link spec, as the run's `links` file names it.
    pub spec: String,
    /// The kind of the qdisc 10: that holds its queue (`queue_kind`).
    pub kind: &'static str,
    /// The device whose root that qdisc is, as `links` names it: none for a
    /// run from before the file named it.
    pub dev: Option<String>,
    /// Its samples in file order, which is time order (an old run's
    /// `date` stamps may run backwards); none without a log.
    pub samples: Vec<ModemSample>,
}

/// Node `node`'s access link queue in out dir `dir`: its link spec (the
/// `links` file's line, tbf on eth0 for a run from before it), the qdisc
/// that holds its queue (its class's 10:, `queue_kind`, on the device
/// the line names: eth0, or ifbm for a class with slots) and each sample
/// of it in `tc-<node>.log`. tcsample writes every device's qdiscs as tc's
/// JSON (`tc -s -j qdisc show`), each with its dev; an older run's log
/// holds eth0's alone, as JSON with no dev or, older still, tc's text
/// (`tc -s qdisc show dev eth0`, its lines joined: a tbf's), and a sample
/// that names no device counts as the modem's. A line cut short (the
/// run's end) is no sample. Refused: a node `links` lacks, and a log with
/// samples none of which holds the queue.
pub fn uplink(dir: &Path, node: usize) -> Result<Uplink, String> {
    let (spec, dev) = link_line(dir, node)?;
    let kind = queue_kind(&spec);
    let text = std::fs::read_to_string(dir.join(format!("tc-{node}.log"))).unwrap_or_default();
    let samples: Vec<ModemSample> = text
        .lines()
        .filter_map(|l| modem_of(l, kind, dev.as_deref()))
        .map(|(ms, drops, held, rate)| ModemSample {
            ms,
            drops,
            held,
            rate,
        })
        .collect();
    let sampled = text.lines().any(|l| {
        l.split_once(' ')
            .is_some_and(|(t, _)| t.parse::<u64>().is_ok())
    });
    if sampled && samples.is_empty() {
        let on = dev.map(|d| format!(" on {d}")).unwrap_or_default();
        return Err(format!(
            "tc-{node}.log: node {node}'s {spec} link has no {kind} 10:{on} in its samples"
        ));
    }
    Ok(Uplink {
        spec,
        kind,
        dev,
        samples,
    })
}

/// The qdisc tree that shaped the run in out dir `dir`, as its knobs key
/// it (`TESTBED_QDISC`): its settings' `QDISC`, else what its files show.
pub fn qdisc_tree(dir: &Path) -> Result<String, String> {
    let settings = std::fs::read_to_string(dir.join("settings"))
        .map(|t| settings_of(&t))
        .unwrap_or_default();
    qdisc_of(&settings, dir)
}

/// The kind of a node's modem qdisc 10:, the root of its link class's tree
/// that holds its queue, for link `spec`: a cake router's cake; a trace's
/// prio, over the fq tracelink's program holds packets in (its drops count
/// in the prio); else the class's tbf, which counts what the qdisc under
/// it holds and drops (cable's PIE, a router's fq_codel).
fn queue_kind(spec: &str) -> &'static str {
    let mut parts = spec.split(',');
    let class = parts.next().unwrap_or("");
    if parts.any(|kv| kv == "router=cake") {
        "cake"
    } else if class == "trace" {
        "prio"
    } else {
        "tbf"
    }
}

/// The bytes of a packet's link header ahead of its network header, which
/// cake's overhead takes the place of (its `avg_hdr_offset`: Ethernet's).
const HDR_OFFSET: f64 = 14.0;

/// What cake charges for `backlog` skb bytes in `qlen` packets, at their
/// mean size: it counts its backlog as the skbs are, but shapes on each
/// packet's network bytes and its overhead, at least its mpu, in ATM's
/// cells or PTM's 65 bytes for 64 (tc-cake(8); sch_cake's cake_overhead),
/// so a dsl router's queue drains at 0.88 of its bandwidth in skb bytes.
fn cake_held(backlog: u64, qlen: u64, overhead: f64, mpu: f64, line: Option<&str>) -> u64 {
    if qlen == 0 {
        return backlog;
    }
    let mut n = (backlog as f64 / qlen as f64 - HDR_OFFSET + overhead).max(mpu);
    match line {
        Some("atm") => n = (n / 48.0).ceil() * 53.0,
        Some("ptm") => n += (n / 64.0).ceil(),
        _ => {}
    }
    (qlen as f64 * n).round() as u64
}

/// A `tc-<i>.log` line's modem queue: (Unix ms, packets dropped, what it
/// holds as its shaper charges it, its rate in bytes a second where it
/// has one), its qdisc 10: of `kind` on `dev` (any device where `dev` is
/// None or the sample names none), from tc's JSON (`tc -s -j qdisc show`,
/// tcsample's: bytes, and rates in bytes a second), or from an older
/// run's text (a tbf's: tc's text rounds a size within 16 bytes of a KiB
/// multiple to `<n>Kb` and one within a KiB of a MiB multiple to `<n>Mb`,
/// iproute2's sprint_size, and prints rates in SI units).
fn modem_of(line: &str, kind: &str, dev: Option<&str>) -> Option<(u64, u64, u64, Option<u64>)> {
    let (ms, rest) = line.split_once(' ')?;
    let ms: u64 = ms.parse().ok()?;
    let rest = rest.trim();
    if rest.starts_with('[') {
        let qs: Vec<Value> = serde_json::from_str(rest).ok()?;
        let q = qs.iter().find(|q| {
            q.get("handle").and_then(Value::as_str) == Some("10:")
                && q.get("kind").and_then(Value::as_str) == Some(kind)
                && dev.is_none_or(|d| q.get("dev").and_then(Value::as_str).is_none_or(|x| x == d))
        })?;
        let num = |v: Option<&Value>| v.and_then(Value::as_f64);
        let int = |k: &str| q.get(k).and_then(Value::as_u64).unwrap_or(0);
        let opts = q.get("options");
        let opt = |k: &str| opts.and_then(|o| o.get(k));
        let rate = num(opt("rate")).or_else(|| num(opt("bandwidth")));
        let (backlog, qlen) = (int("backlog"), int("qlen"));
        let raw = opt("raw").and_then(Value::as_bool).unwrap_or(false);
        let held = if kind == "cake" && !raw {
            cake_held(
                backlog,
                qlen,
                num(opt("overhead")).unwrap_or(0.0),
                num(opt("mpu")).unwrap_or(0.0),
                opt("atm").and_then(Value::as_str),
            )
        } else {
            backlog
        };
        return Some((ms, int("drops"), held, rate.map(|r| r as u64)));
    }
    if kind != "tbf" {
        return None;
    }
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
    Some((
        ms,
        drops,
        n.parse::<u64>().ok()? * mult,
        after("rate").and_then(|r| tc_rate(&r)),
    ))
}

/// A rate as tc prints it (`10Mbit`, `1500Kbit`, `800bit`), in bytes per
/// second.
fn tc_rate(v: &str) -> Option<u64> {
    let unit = v.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
    let (n, unit) = v.split_at(unit);
    let mult = match unit {
        "bit" => 1.0,
        "Kbit" => 1e3,
        "Mbit" => 1e6,
        "Gbit" => 1e9,
        _ => return None,
    };
    Some((n.parse::<f64>().ok()? * mult / 8.0) as u64)
}

/// The wire counters a node's capture gives.
const PCAP_COUNTERS: [&str; 6] = [
    "up_bytes",
    "up_packets",
    "small_packets",
    "retransmits",
    "up_ack_packets",
    "up_ack_bytes",
];

/// The network counters a native node's TCP counters give
/// (`netstat-<i>.log`, [`tcp_counts`]).
const NETSTAT_COUNTERS: [&str; 2] = ["window_cuts", "rtos"];

/// The TcpExt counters (`/proc/net/netstat`) that count a window cut:
/// recoveries entered (SACK's, and Reno's without SACK: each sets the
/// threshold and starts the window's fall, `tcp_enter_recovery`), and
/// losses a tail-loss probe repaired (`tcp_process_tlp_ack`: a cwnd
/// reduction outside a recovery).
const CUT_COUNTERS: [&str; 3] = ["TCPSackRecovery", "TCPRenoRecovery", "TCPLossProbeRecovery"];

/// The TcpExt counter of retransmission timeouts: every RTO timer that
/// fired and resent (`tcp_retransmit_timer`), a backed-off one's again.
const RTO_COUNTER: &str = "TCPTimeouts";

/// A native node's TCP counters over a run, from its `netstat-<i>.log`
/// (run.sh's: its network namespace's TcpExt line every 100 ms, the
/// names first): each sample's Unix ms, its window cuts so far
/// ([`CUT_COUNTERS`]) and its timeouts so far ([`RTO_COUNTER`]), from the
/// first sample's; none without a log, or one that names none of them.
fn tcp_counts(text: &str) -> Vec<(u64, u64, u64)> {
    let mut names: Vec<&str> = Vec::new();
    let mut first: Option<(u64, u64)> = None;
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(h) = line.strip_prefix("# TcpExt:") {
            names = h.split_whitespace().collect();
            continue;
        }
        let Some((ms, rest)) = line.split_once(' ') else {
            continue;
        };
        let (Ok(ms), Some(values)) = (ms.parse::<u64>(), rest.strip_prefix("TcpExt:")) else {
            continue;
        };
        let values: Vec<u64> = values
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        // A line cut short (the run's end) is no sample.
        if values.len() != names.len() || names.is_empty() {
            continue;
        }
        let get = |n: &str| {
            names
                .iter()
                .position(|&x| x == n)
                .and_then(|i| values.get(i))
        };
        let cuts: u64 = CUT_COUNTERS.iter().filter_map(|n| get(n)).sum();
        let Some(&rtos) = get(RTO_COUNTER) else {
            continue;
        };
        let (c0, r0) = *first.get_or_insert((cuts, rtos));
        out.push((ms, cuts.saturating_sub(c0), rtos.saturating_sub(r0)));
    }
    out
}

/// The network side of a testbed run's report.
struct Network {
    wan: WanReport,
    bins: Vec<Vec<[u64; UP_COUNTERS.len()]>>,
    unmeasured: Vec<usize>,
    ends: BTreeMap<(HostIx, usize), usize>,
    /// Each host's own sends, from its capture (none without one): what
    /// its meter's truth takes its share from (`gate`).
    sends: Vec<Sent>,
}

/// What a node's capture says it sent: its segments and datagrams to the
/// swarm, in time order, a SACK alone apart where its stack tells one
/// (`sack_alone`); and its own datagrams with a payload that its link's
/// queue dropped (`dropped`, from its offered capture: none without one).
#[derive(Default)]
struct Sent {
    sends: Vec<Send>,
    dropped: Vec<Dropped>,
}

/// One segment a node sent the swarm, as its capture saw it: when (Unix
/// µs), what its link charged it (its class's framing over its IP length,
/// `Framing::packet`), its transport's payload (TCP's: the vnic bytes its
/// transport took; `None` for a datagram, a data channel's, whose payload
/// DTLS hides), a datagram's payload as its length gives it
/// (`SctpProfile::datagram_payload` of its host's stack; 0 for a segment
/// and without a stack), and whether it resent bytes its flow had sent.
struct Send {
    us: u64,
    charge: u64,
    app: Option<u64>,
    est: u64,
    resent: bool,
}

/// One of a node's datagrams its link's queue dropped: when it was offered
/// (Unix µs), the charge its link would have taken, and its payload as its
/// length gives it (`Send::est`).
struct Dropped {
    us: u64,
    charge: u64,
    est: u64,
}

/// The node's own datagrams that it offered its link (`offered`, ahead of
/// its queue) and its link didn't carry within DROP_WAIT_US (`carried`),
/// told apart by destination, IP id, IP length and ports, each charged at
/// `framing` and its payload read as `stack` reads a datagram's length.
fn dropped(
    offered: &[&Packet],
    carried: &[&Packet],
    framing: strims_wan::framing::Framing,
    stack: Option<SctpProfile>,
) -> Vec<Dropped> {
    type Key = (Option<u8>, u16, u64, (u16, u16));
    let key = |p: &Packet| p.udp.map(|ports| (p.dst, p.ip_id, p.ip_len, ports));
    let mut at: BTreeMap<Key, Vec<u64>> = BTreeMap::new();
    for p in carried {
        if let Some(k) = key(p) {
            at.entry(k).or_default().push(p.us);
        }
    }
    for v in at.values_mut() {
        v.sort_unstable();
    }
    offered
        .iter()
        .filter_map(|p| {
            let k = key(p)?;
            let ts = at.get(&k).map_or(&[][..], |v| &v[..]);
            let i = ts.partition_point(|&t| t < p.us.saturating_sub(1000));
            let carried = ts.get(i).is_some_and(|&t| t <= p.us + DROP_WAIT_US);
            (!carried).then(|| Dropped {
                us: p.us,
                charge: framing.packet(p.ip_len as usize) as u64,
                est: stack.map_or(0, |x| x.datagram_payload(p.ip_len)),
            })
        })
        .collect()
}

/// The truth's log of a competing upload from the node at `me` (its
/// packets to the sink in its capture `ps`, in capture order): after each
/// packet, its IP bytes so far, stamped with the packet's capture time
/// since the run's start (`LEAD` before the stream's, `start_ms`), as
/// `report::competing_before` reads a log: what a window counts is what
/// the link carried in it.
fn competing_log(
    ps: &[Packet],
    me: Option<u8>,
    start_ms: u64,
) -> Vec<(strims_base::time::Duration, u64)> {
    let run0_us = start_ms.saturating_sub(LEAD.as_millis() as u64) * 1000;
    let mut sent = 0u64;
    ps.iter()
        .filter(|p| p.src.is_some() && p.src == me && p.dst == Some(SINK))
        .map(|p| {
            sent += p.ip_len;
            let us = i64::try_from(p.us.saturating_sub(run0_us)).unwrap_or(i64::MAX);
            (strims_base::time::Duration::from_micros(us), sent)
        })
        .collect()
}

/// The meter's truth's share at each of a testbed node's meter samples,
/// as strims-sim's HostStats::gate takes it from the sim's counters, from
/// the node's capture (`sent`) and its records: its own charge, its first
/// transmissions' and its transports' bytes so far (link_bytes, app_bytes,
/// resent_bytes), and over its near-saturated windows (`Gated`: a window
/// between two samples whose own charge was at least NEAR_SATURATED of
/// what its link could carry for it, `report::available` over `spec` and
/// its competitor's log) its transports' bytes over its link's charge of
/// its first transmissions, and its runner's channel bytes over what it
/// handed vnic (its records' channel_bytes and bytes_written). A window
/// whose samples lack channel_bytes adds none of them: a node whose lines
/// lack the field keeps S_c = 1 (`report::channel_share` of no data).
/// `us_of` takes a sample's ask to Unix µs.
///
/// A TCP segment's transports' bytes are its payload, and its flow's
/// resends are out of the charge. A data channel's datagram hides its
/// payload inside DTLS, so where its spec names its stack a node takes a
/// datagram's from its length (`Send::est`: one DATA chunk under its
/// stack's fixed headers), leaves a SACK alone out of both sides, as the
/// sim leaves pure acknowledgements out, and takes its datagrams its queue
/// dropped (`Sent::dropped`, offered against carried) as resent once: each
/// one's payload off its transports' bytes and its charge off its link's,
/// in the window it was offered, as the sim leaves a dropped chunk out of
/// both. What stays in, which DTLS hides:
///
/// - a packet of two chunks (a message's tail and the next's head, which
///   dcSCTP bundles), a chunk's padding and a SACK on a DATA packet read
///   as payload, so the share reads high;
/// - a chunk SCTP resends that had arrived counts on both sides, about as
///   a first transmission does;
/// - a dropped SACK alone, a chunk resent twice or given up on, and a loss
///   past the node's queue are each taken wrong, and without an offered
///   capture no drop is taken out: the share reads low by its resends'
///   charge;
/// - a SACK with gap blocks reads as a few bytes of payload.
///
/// A node without a stack adds no datagram's bytes, and keeps its link's
/// fallback (`report::full_share`) unless its segments carry some.
fn gate(
    meter: &mut [MeterSample],
    sent: &Sent,
    spec: &crate::spec::HostSpec,
    competing: &[(strims_base::time::Duration, u64)],
    us_of: impl Fn(Duration) -> u64,
) {
    let mut g = Gated::default();
    let (mut link, mut app, mut resent) = (0u64, 0u64, 0u64);
    let mut next = sent.sends.iter().peekable();
    let mut drops = sent.dropped.iter().peekable();
    let mut prev: Option<(Duration, Option<u64>, u64)> = None;
    for m in meter.iter_mut() {
        let to = us_of(m.asked);
        let (mut charged, mut took, mut again) = (0u64, 0u64, 0u64);
        while let Some(x) = next.next_if(|x| x.us <= to) {
            charged += x.charge;
            if x.resent {
                again += x.charge;
            } else {
                took += x.app.unwrap_or(x.est);
            }
        }
        while let Some(d) = drops.next_if(|d| d.us <= to) {
            again += d.charge;
            took = took.saturating_sub(d.est);
        }
        link += charged;
        app += took;
        resent += again;
        (m.link_bytes, m.app_bytes, m.resent_bytes) = (link, app, resent);
        g.window = prev.is_some_and(|(at, ..)| {
            at < m.asked
                && charged > 0
                && charged as f64
                    >= crate::report::NEAR_SATURATED
                        * crate::report::available(spec, competing, at, m.asked) as f64
        });
        if g.window {
            g.app += took;
            g.link += charged.saturating_sub(again);
            if let (Some((_, Some(data), vnic)), Some(now)) = (prev, m.channel_bytes) {
                g.data += now.saturating_sub(data);
                g.vnic += m.vnic_bytes.saturating_sub(vnic);
            }
        }
        m.gated = g;
        prev = Some((m.asked, m.channel_bytes, m.vnic_bytes));
    }
}

/// Whether `p` is a datagram that carries a SACK alone as `stack` reads
/// its length: no DATA chunk's payload under its fixed headers. None is
/// without a stack.
fn sack_alone(p: &Packet, stack: Option<SctpProfile>) -> bool {
    p.udp.is_some() && stack.is_some_and(|x| x.datagram_payload(p.ip_len) == 0)
}

impl Network {
    fn read(run: &Run, s: &Scenario, seed: HostIx) -> Network {
        let bin = BIN_MS.unsigned_abs().max(1);
        let edges = s.duration.saturating_sub(LEAD).as_millis() as u64 / bin + 1;
        let edge_us = |k: u64| (run.start_ms + k * bin) * 1000;
        let mut bins = vec![vec![[0u64; UP_COUNTERS.len()]; s.hosts.len()]; edges as usize];
        let mut unmeasured: Vec<usize> = ["up_sack_packets", "up_sack_bytes"].map(up).to_vec();
        // The nodes without a capture, a tc log or TCP counters: their
        // columns are unmeasured alone, where another node's are measured.
        let (mut no_pcap, mut no_tc, mut no_tcp) = (Vec::new(), Vec::new(), Vec::new());
        let kinds: Vec<&str> = run
            .settings
            .get("KINDS")
            .map_or_else(Vec::new, |k| k.split_whitespace().collect());
        let competes = run
            .settings
            .get("COMPETE")
            .is_some_and(|c| !c.trim().is_empty());
        // The nodes with a data channel: a link to a browser, which is
        // WebRTC but for a native node's under BROWSER_LINKS=ws.
        let browser = |i: usize| kinds.get(i).is_some_and(|&k| k != "native");
        let webrtc = run.settings.get("BROWSER_LINKS").map(String::as_str) == Some("webrtc");
        let topology = run
            .file("topology")
            .map(|t| links_of(&t))
            .unwrap_or_default();
        let is_rtc = |(a, b): (usize, usize)| {
            (browser(a) && browser(b)) || (webrtc && (browser(a) || browser(b)))
        };
        let rtc: Vec<HostIx> = topology
            .iter()
            .copied()
            .filter(|&l| is_rtc(l))
            .flat_map(|(a, b)| [a, b])
            .collect();
        // The receivers whose downlink has a queue of its own
        // (SLOW_DOWN_KBIT's viewer 1, its ifb's tbf): what it drops of a
        // sender's is the sender's drop, as the window model counts a full
        // downlink queue's, and the sender's resend of it no retransmit.
        // Each sender's capture tells them, a resend of carried bytes to
        // that receiver: the testbed loses nothing else past a queue but
        // SEED_LOSS_PCT's filters.
        let slow: Vec<HostIx> = if run
            .settings
            .get("SLOW_DOWN_KBIT")
            .is_some_and(|v| !v.trim().is_empty())
        {
            vec![1]
        } else {
            Vec::new()
        };
        let seed_loss = run
            .settings
            .get("SEED_LOSS_PCT")
            .is_some_and(|v| !v.trim().is_empty());
        // The slow receivers a node sends to, and whether over a data
        // channel, whose resends DTLS hides.
        let feeds = |h: HostIx, rtc_only: bool| {
            topology.iter().any(|&(a, b)| {
                let peer = if a == h {
                    b
                } else if b == h {
                    a
                } else {
                    return false;
                };
                slow.contains(&peer) && (!rtc_only || is_rtc((a, b)))
            })
        };
        let mut wan = WanReport::default();
        let mut ends = BTreeMap::new();
        let mut sends: Vec<Sent> = (0..s.hosts.len()).map(|_| Sent::default()).collect();
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
                up_queue_until: None,
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
                competing_drop_packets: 0,
                competing_cwnd: Vec::new(),
                packets: Vec::new(),
                offers: None,
            };
            if h >= run.nodes.len() {
                wan.hosts.push(w);
                continue;
            }
            let me = u8::try_from(10 + h).ok();
            let pcap = std::fs::read(run.dir.join(format!("pcap-{h}.pcap")))
                .ok()
                .and_then(|b| packets(&b));
            // A capture that stopped while its node ran (tcpdump exited:
            // run.sh's pcap-<i>.end, its unix ms) leaves the node's wire
            // columns unmeasured past it, not 0: they end at the first
            // edge after it, as a stopped tc sampler's drops do. A
            // capture's last packet can't tell: a node's own traffic
            // falls silent for seconds while it runs.
            let pcap_end = run
                .file(&format!("pcap-{h}.end"))
                .and_then(|t| t.split_whitespace().next()?.parse::<u64>().ok())
                .and_then(|ms| (0..bins.len()).find(|&k| edge_us(k as u64) > ms * 1000));
            if let (Some(end), true) = (pcap_end, pcap.is_some()) {
                for c in PCAP_COUNTERS {
                    ends.insert((h, up(c)), end);
                }
            }
            // What slow receivers' downlinks dropped of its, when.
            let mut down: Vec<u64> = Vec::new();
            let has_pcap = pcap.is_some();
            match pcap {
                Some(ps) => {
                    let mut mine = ps
                        .iter()
                        .filter(|p| p.src.is_some() && p.src == me)
                        .peekable();
                    let mut cum = [0u64; UP_COUNTERS.len()];
                    let mut cross = 0u64;
                    // The swarm's traffic is between its nodes (10 + index);
                    // a browser's own downloads and their ACKs (it reaches the
                    // internet) and the relay's (.5) are no sim host's.
                    let node = |o: Option<u8>| {
                        o.and_then(|o| o.checked_sub(10))
                            .is_some_and(|i| usize::from(i) < run.nodes.len())
                    };
                    // Its own data to the swarm (`gate` takes it): each
                    // segment the link carried, a resend where its flow had
                    // sent its bytes before. That is told from what the
                    // node offered its link (pcap-<i>-offered.pcap, ahead of
                    // the queue and of GSO's split: a segment the queue
                    // dropped is there, and its resend, a tail-loss probe's
                    // too, falls under what its flow had offered), by the
                    // latest offer holding the carried segment's bytes, else
                    // from what the link carried.
                    let framing = site.profile.framing;
                    let own_data = |p: &&Packet| {
                        p.src.is_some()
                            && p.src == me
                            && node(p.dst)
                            && !p.pure_ack
                            && p.payload > 0
                    };
                    let offered_ps: Option<Vec<Packet>> =
                        std::fs::read(run.dir.join(format!("pcap-{h}-offered.pcap")))
                            .ok()
                            .and_then(|b| packets(&b));
                    let offered_own: Option<Vec<&Packet>> = offered_ps
                        .as_ref()
                        .map(|o| o.iter().filter(own_data).collect());
                    let offered = offered_own.as_deref().map(Offers::of);
                    let carried: Vec<&Packet> = ps.iter().filter(own_data).collect();
                    let again = resends(carried.iter().copied());
                    // Its retransmits: the TCP segments that resent bytes
                    // its link had carried (past its queue) before, as the
                    // window model counts a segment the path lost; a resend
                    // of what its own queue dropped is a drop there (the tc
                    // log's), not one of these, and one to a slow receiver
                    // a drop at that receiver's downlink queue (`down`).
                    let to_slow = |p: &Packet| {
                        p.dst
                            .and_then(|o| o.checked_sub(10))
                            .is_some_and(|i| slow.contains(&usize::from(i)))
                    };
                    let (mut resent, mut down_at) = (Vec::new(), Vec::new());
                    for (p, a) in carried.iter().zip(carried_again(carried.iter().copied())) {
                        if a {
                            if to_slow(p) {
                                &mut down_at
                            } else {
                                &mut resent
                            }
                            .push(p.us);
                        }
                    }
                    let mut resent = resent.into_iter().peekable();
                    down = down_at;
                    let stack = spec.rtc_stack;
                    let own: Vec<Send> = carried
                        .iter()
                        .zip(again)
                        .filter(|(p, _)| !sack_alone(p, stack))
                        .map(|(p, carried_again)| {
                            let offer = offered.as_ref().and_then(|o| o.resent(p));
                            Send {
                                us: p.us,
                                charge: framing.packet(p.ip_len as usize) as u64,
                                app: p.tcp.is_some().then_some(p.payload),
                                est: match (p.udp, stack) {
                                    (Some(_), Some(x)) => x.datagram_payload(p.ip_len),
                                    _ => 0,
                                },
                                resent: offer.unwrap_or(carried_again),
                            }
                        })
                        .collect();
                    // A SACK alone dropped isn't resent: it is no drop of
                    // the share's.
                    let drops = offered_own.as_deref().map_or_else(Vec::new, |o| {
                        let mut d = dropped(o, &carried, framing, stack);
                        d.retain(|x| x.est > 0);
                        d
                    });
                    if let Some(x) = sends.get_mut(h) {
                        *x = Sent {
                            sends: own,
                            dropped: drops,
                        };
                    }
                    for (k, edge) in bins.iter_mut().enumerate() {
                        while resent.next_if(|&us| us < edge_us(k as u64)).is_some() {
                            add(&mut cum, "retransmits", 1);
                        }
                        while let Some(p) = mine.next_if(|p| p.us < edge_us(k as u64)) {
                            if p.dst == Some(SINK) {
                                cross += p.ip_len;
                            } else if !node(p.dst) {
                                continue;
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
                        if h == seed && pcap_end.is_none_or(|end| k < end) {
                            let t = LEAD + Duration::from_millis(k as u64 * bin);
                            w.competing.push((t.into(), cross));
                        }
                    }
                    w.acks = cum.get(up("up_ack_packets")).copied().unwrap_or(0);
                    w.ack_bytes = cum.get(up("up_ack_bytes")).copied().unwrap_or(0);
                    // The truth's log of the competitor's bytes: its IP bytes
                    // so far as each of its packets leaves, stamped with the
                    // packet's capture time (report::available charges them).
                    if h == seed {
                        w.competing_log = competing_log(&ps, me, run.start_ms);
                    }
                }
                None => no_pcap.push(h),
            }
            // The node's modem queue (uplink); a node with none has no
            // counters here. Run::read checked it (check_samplers).
            let tbf = uplink(&run.dir, h)
                .ok()
                .map(|u| u.samples)
                .filter(|s| !s.is_empty());
            match tbf {
                Some(samples) => {
                    let mut at = samples.iter().peekable();
                    let mut last = samples.first().map_or(0, |x| x.drops);
                    // With what slow receivers' downlinks dropped of its.
                    let mut downs = down.iter().peekable();
                    let mut lost = 0;
                    for (k, edge) in bins.iter_mut().enumerate() {
                        while let Some(s) = at.next_if(|x| x.ms * 1000 < edge_us(k as u64)) {
                            last = s.drops;
                        }
                        while downs.next_if(|&&us| us < edge_us(k as u64)).is_some() {
                            lost += 1;
                        }
                        if let Some(x) = edge.get_mut(h).and_then(|c| c.get_mut(up("drops"))) {
                            *x = last + lost;
                        }
                    }
                    // A sampler that stopped before the run's end leaves its
                    // columns unmeasured past its last sample, not flat: the
                    // drops past the first edge after it, the queue past it
                    // and an interval (the median gap) more.
                    if let Some(t_last) = samples.last().map(|s| s.ms) {
                        if let Some(end) =
                            (0..bins.len()).find(|&k| edge_us(k as u64) > t_last * 1000)
                        {
                            ends.insert((h, up("drops")), end);
                        }
                        let mut gaps: Vec<u64> = samples
                            .iter()
                            .zip(samples.iter().skip(1))
                            .map(|(a, b)| b.ms.saturating_sub(a.ms))
                            .collect();
                        gaps.sort_unstable();
                        let gap = gaps.get(gaps.len() / 2).copied().unwrap_or(0);
                        w.up_queue_until = run.at(t_last + gap).map(Into::into);
                    }
                    w.queue_drops = samples
                        .last()
                        .zip(samples.first())
                        .map_or(0, |(b, a)| b.drops.saturating_sub(a.drops))
                        + down.len() as u64;
                    // tbf drops frames: GSO is off, a segment is one.
                    w.queue_drop_packets = w.queue_drops;
                    // The wait is the backlog over the tbf's own rate as the
                    // sample shows it (its steps as applied; a class's
                    // line code: VDSL's tbf runs at 64/65 of the line's),
                    // else the scenario's.
                    for s in &samples {
                        let Some(t) = run.at(s.ms) else {
                            continue;
                        };
                        let rate = s
                            .rate
                            .unwrap_or_else(|| {
                                spec.up_steps
                                    .iter()
                                    .rfind(|&&(at, _)| at <= t)
                                    .map_or(site.profile.up, |&(_, r)| r)
                            })
                            .max(1);
                        let wait = i64::try_from(s.held * 1_000_000 / rate).unwrap_or(i64::MAX);
                        w.up_queue
                            .push((t.into(), strims_base::time::Duration::from_micros(wait)));
                        w.up_backlog.push((t.into(), s.held));
                    }
                }
                None => no_tc.push(h),
            }
            // A browser's links are data channels, whose resends DTLS
            // hides and whose windows no TCP counter counts; so is a
            // native node's link to one over WebRTC: its retransmits miss
            // that link's.
            let native = kinds.get(h).is_none_or(|&k| k == "native");
            if !native || rtc.contains(&h) {
                ends.insert((h, up("retransmits")), 0);
            }
            // A sender to a slow receiver: its drops there are its
            // capture's to tell, none without one or past its end, nor a
            // data channel's; with SEED_LOSS_PCT the seed's resends to it
            // are its site loss's too, its drops and retransmits not to be
            // told apart.
            if feeds(h, false) {
                let lossy = h == seed && seed_loss;
                let end = if !has_pcap || feeds(h, true) || lossy {
                    Some(0)
                } else {
                    pcap_end
                };
                if let Some(end) = end {
                    let e = ends.entry((h, up("drops"))).or_insert(end);
                    *e = (*e).min(end);
                }
                if lossy {
                    ends.insert((h, up("retransmits")), 0);
                }
            }
            // Its window cuts and timeouts, from its namespace's TCP
            // counters (tcp_counts), as of each bin's edge; not the seed's
            // where it ran competing uploads, whose flows they count too.
            let tcp = run
                .file(&format!("netstat-{h}.log"))
                .map(|t| tcp_counts(&t))
                .filter(|x| !x.is_empty());
            match tcp {
                Some(samples) if native && !(h == seed && competes) => {
                    let mut at = samples.iter().peekable();
                    let (mut cuts, mut rtos) = (0, 0);
                    for (k, edge) in bins.iter_mut().enumerate() {
                        while let Some(&(_, c, r)) = at.next_if(|x| x.0 * 1000 < edge_us(k as u64))
                        {
                            (cuts, rtos) = (c, r);
                        }
                        if let Some(x) = edge.get_mut(h) {
                            for (name, v) in [("window_cuts", cuts), ("rtos", rtos)] {
                                if let Some(y) = x.get_mut(up(name)) {
                                    *y = v;
                                }
                            }
                        }
                    }
                    // A log that stopped before the run's end leaves the
                    // columns unmeasured past the first edge after it.
                    if let Some(&(t_last, ..)) = samples.last()
                        && let Some(end) =
                            (0..bins.len()).find(|&k| edge_us(k as u64) > t_last * 1000)
                    {
                        for c in NETSTAT_COUNTERS {
                            ends.insert((h, up(c)), end);
                        }
                    }
                }
                _ => no_tcp.push(h),
            }
            wan.hosts.push(w);
        }
        // Unmeasured where no node measured it; else past nothing (end 0)
        // for the nodes that didn't.
        let nodes = run.nodes.len();
        if no_pcap.len() == nodes {
            unmeasured.extend(PCAP_COUNTERS.map(up));
        } else {
            for &h in &no_pcap {
                for c in PCAP_COUNTERS {
                    ends.insert((h, up(c)), 0);
                }
            }
        }
        if no_tc.len() == nodes {
            unmeasured.push(up("drops"));
        } else {
            for &h in &no_tc {
                ends.insert((h, up("drops")), 0);
            }
        }
        if no_tcp.len() == nodes {
            unmeasured.extend(NETSTAT_COUNTERS.map(up));
        } else {
            for &h in &no_tcp {
                for c in NETSTAT_COUNTERS {
                    ends.insert((h, up(c)), 0);
                }
            }
        }
        // No node measured its retransmits: every node a browser.
        if (0..nodes).all(|h| ends.get(&(h, up("retransmits"))) == Some(&0)) {
            unmeasured.push(up("retransmits"));
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
            ends,
            sends,
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
/// which a testbed run's report lacks. The joins are a run's whose viewers
/// arrive into the running stream (`JOINS`, a `JOIN_AT` after the start:
/// `Scenario::profiled`); a testbed viewer starts once, so a run has no
/// rejoins.
pub const ROW: &[&str] = &[
    "test",
    "scenario",
    "trace",
    "reads_hash",
    "passed",
    "failure",
    "delivery",
    "delivery_honest",
    "arrivals",
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
    "arrived_honest",
    "join_p50_ms",
    "join_p90_ms",
    "caught_up_p50_ms",
    "caught_up_p90_ms",
    "join_unread",
    "caught_up_never",
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
/// its rate (`tc-<i>.log`) as it is a sim's model queue.
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

    /// A TCP segment of `payload` bytes from `src` to `dst` at sequence
    /// number `seq`, ports 4000 to 5000.
    fn tcp_seq(src: u8, dst: u8, payload: usize, seq: u32) -> Vec<u8> {
        let mut p = tcp(src, dst, payload, 0x18);
        p[20..22].copy_from_slice(&4000u16.to_be_bytes());
        p[22..24].copy_from_slice(&5000u16.to_be_bytes());
        p[24..28].copy_from_slice(&seq.to_be_bytes());
        p
    }

    /// A seed and one viewer: the seed's two lines and the viewer's one,
    /// which read two frames; the seed's capture a data packet, a pure ACK,
    /// a packet to the competing upload's sink and one it received; both
    /// tbfs' counters.
    fn fixture(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("strims-testbed-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let w = |name: &str, text: &str| std::fs::write(dir.join(name), text).unwrap();
        w(
            "settings",
            "VIEWERS=1\nSECS=10\nQDISC=clsact>ifb-netem>root-class\nSTAMPS=clock\nJOIN_LINKS=together\n",
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
        // A browser's download's ACK to the internet, and the relay's:
        // no swarm host's.
        let mut internet = tcp(10, 11, 0, 0x10);
        internet[16..20].copy_from_slice(&[93, 184, 216, 34]);
        let packets = [
            (us(50), tcp(10, 11, 1448, 0x18)),
            (us(55), internet),
            (us(58), tcp(10, 5, 300, 0x18)),
            (us(60), tcp(10, 11, 0, 0x10)),
            (us(70), tcp(10, 250, 1448, 0x10)),
            (us(80), tcp(11, 10, 100, 0x18)),
        ];
        std::fs::write(dir.join("pcap-0.pcap"), pcap(&packets)).unwrap();
        std::fs::write(dir.join("pcap-1.pcap"), pcap(&[])).unwrap();
        let tc_at = |ms: u64, rate: &str, drops: u64| {
            format!(
                "{} qdisc tbf 10: root refcnt 2 rate {rate} burst 3028b lat 99ms  Sent 3000 bytes 2 pkt (dropped {drops}, overlimits 0 requeues 0)  backlog 1514b 1p requeues 0  qdisc clsact ffff: parent ffff:fff1 Sent 0 bytes 0 pkt (dropped 9, overlimits 0 requeues 0)\n",
                START + ms
            )
        };
        let tc = |drops: u64| tc_at(150, "10Mbit", drops);
        w(
            "ss-0.log",
            &format!(
                "@ {}\nRecv-Q Send-Q Local Address:Port Peer Address:Port\n0      8607      10.77.0.10:4000   10.77.0.11:51216\n\t cubic rto:300 rtt:99.967/3.159 cwnd:33 ssthresh:29 bytes_sent:493341 bytes_acked:472403 segs_out:419 data_segs_out:375 pacing_rate 4588736bps delivery_rate 3728280bps retrans:0/13 notsent:1448 minrtt:20.5\n0      0      10.77.0.10:5201   10.77.0.250:40000\n\t cubic rtt:30/1 cwnd:10\n",
                START + 1000
            ),
        );
        // The seed's tbf: 4 drops by 150 ms, 3 more by 450 ms, when a
        // step has halved its rate.
        w(
            "tc-0.log",
            &[tc(4), tc_at(250, "10Mbit", 4), tc_at(450, "5Mbit", 7)].concat(),
        );
        w("tc-1.log", &tc(0));
        dir
    }

    /// What changes a run keys it: settings without a twin knob go in as
    /// TESTBED_*, and what the twin can't take is refused.
    #[test]
    fn a_runs_knobs_key_what_changed_it() {
        let dir = std::env::temp_dir().join(format!("strims-testbed-knobs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let star = "node 0 10.77.0.10:4000\nnode 1 10.77.0.11:4000\nnode 2 10.77.0.12:4000\nlink 0 1\nlink 0 2\n";
        let of = |text: &str, links: Option<&str>| knobs_of(&settings_of(text), star, links, &dir);
        let base = "VIEWERS=2\nSECS=30\nQDISC=clsact>ifb-netem>root-class\nSTAMPS=clock\nJOIN_LINKS=together\n";
        let k = of(base, None).unwrap();
        assert_eq!(k["TESTBED_QDISC"], "clsact>ifb-netem>root-class");
        assert!(
            !k.keys()
                .any(|k| k.starts_with("TESTBED_") && k != "TESTBED_QDISC")
        );
        // The trees of runs from before QDISC: a links file's, a tc dump's.
        let none = "VIEWERS=2\nSECS=30\n";
        assert!(of(none, None).unwrap_err().contains("qdisc tree"));
        std::fs::write(
            dir.join("tc-0.log"),
            "1 qdisc clsact ffff: parent ffff:fff1 qdisc tbf 10: root\n",
        )
        .unwrap();
        assert_eq!(
            of(none, None).unwrap()["TESTBED_QDISC"],
            "clsact>ifb-netem>root-tbf"
        );
        std::fs::write(
            dir.join("tc-0.log"),
            "1 qdisc netem 1: root qdisc tbf 10: parent 1:1\n",
        )
        .unwrap();
        assert_eq!(of(none, None).unwrap()["TESTBED_QDISC"], "netem>tbf");
        // An old netem-root run with PCAP=1 has a clsact for its mirror.
        std::fs::write(
            dir.join("tc-0.log"),
            "1 qdisc netem 1: root qdisc tbf 10: parent 1:1 qdisc clsact ffff: parent ffff:fff1\n",
        )
        .unwrap();
        assert_eq!(of(none, None).unwrap()["TESTBED_QDISC"], "netem>tbf");
        std::fs::write(dir.join("links"), "node 0 tbf 10 20 100\n").unwrap();
        assert_eq!(
            of(none, None).unwrap()["TESTBED_QDISC"],
            "clsact>ifb-netem>root-class"
        );
        // Uplinks the twin can't take, and node arguments it has no knob for.
        assert!(
            of(&format!("{base}SEED_UP_MBIT=2.5\n"), None)
                .unwrap_err()
                .contains("whole Mbit/s")
        );
        assert!(
            of(&format!("{base}VIEWER_UP_MBIT=0.5\n"), None)
                .unwrap_err()
                .contains("whole Mbit/s")
        );
        let browsers = format!("{base}KINDS=native chromium firefox\nBROWSER_LINKS=webrtc\n");
        assert!(
            of(&format!("{browsers}BROWSER_ARGS=--rate 3000\n"), None)
                .unwrap_err()
                .contains("BROWSER_ARGS --rate")
        );
        let k = of(
            &format!("{browsers}NODE_ARGS=--chunk 4096 --munro 32768\nBROWSER_ARGS=--chunk 4096 --munro 32768\n"),
            None,
        )
        .unwrap();
        assert_eq!(
            (k["STRIMS_SIM_CHUNK"].as_str(), k["STRIMS_SIM_RTC"].as_str()),
            ("4096", "str0m,chromium,firefox")
        );
        // An all-browser run's chunks are its BROWSER_ARGS'.
        let all = format!(
            "{base}KINDS=chromium firefox chromium\nBROWSER_ARGS=--chunk 1024 --munro 8192\n"
        );
        assert_eq!(of(&all, None).unwrap()["STRIMS_SIM_CHUNK"], "1024");
        // A join in whole seconds, as the tiers' cells give it, else in ms.
        assert_eq!(
            of(&format!("{base}JOIN_AT=3\n"), None).unwrap()["STRIMS_SIM_CAL_JOIN_AT"],
            "3"
        );
        let k = of(&format!("{base}JOIN_AT=2.5\n"), None).unwrap();
        assert_eq!(k["STRIMS_SIM_CAL_JOIN_AT_MS"], "2500");
        assert!(!k.contains_key("STRIMS_SIM_CAL_JOIN_AT"));
        assert_eq!(
            of(&format!("{base}JOIN_AT=3.0\n"), None).unwrap()["STRIMS_SIM_CAL_JOIN_AT"],
            "3"
        );
        // 0 in any spelling keys as unset does.
        let unset = of(base, None).unwrap();
        for zero in ["0", "0.0", "00", "0.0001"] {
            assert_eq!(
                of(&format!("{base}JOIN_AT={zero}\n"), None).unwrap(),
                unset,
                "JOIN_AT={zero}"
            );
        }
        let cal = Calibration::from_knobs(|n| k.get(n).cloned());
        assert_eq!(cal.join_at_ms, Some(2500));
        // The buckets' peaks are the twin's, its buckets sending at them.
        let k = of(
            &format!("{base}SEED_PEAK_MBIT=100\nVIEWER_PEAK_MBIT=40.5\n"),
            None,
        )
        .unwrap();
        assert_eq!(
            (
                k["STRIMS_SIM_CAL_SEED_PEAK_MBIT"].as_str(),
                k["STRIMS_SIM_CAL_VIEWER_PEAK_MBIT"].as_str()
            ),
            ("100", "40.5")
        );
        assert!(
            !k.keys()
                .any(|k| k.starts_with("TESTBED_") && k != "TESTBED_QDISC"),
            "{k:?}"
        );
        let cal = Calibration::from_knobs(|n| k.get(n).cloned());
        assert_eq!(
            (cal.seed_peak_mbit, cal.viewer_peak_mbit),
            (Some(100.0), Some(40.5))
        );
        let peaks: Vec<Option<u64>> = calibration_star(&cal)
            .hosts
            .iter()
            .map(|h| h.site.and_then(|s| s.profile.bucket).and_then(|b| b.peak))
            .collect();
        assert_eq!(peaks, [Some(12_500_000), Some(5_062_500), Some(5_062_500)]);
        assert!(
            of(&format!("{base}SEED_PEAK_MBIT=fast\n"), None)
                .unwrap_err()
                .contains("SEED_PEAK_MBIT=fast")
        );
        assert!(
            of(&format!("{base}JOIN_AT=soon\n"), None)
                .unwrap_err()
                .contains("JOIN_AT=soon")
        );
        // Another topology keys by its links, not by its file.
        let line = "node 0 10.77.0.10:4000\nnode 1 10.77.0.11:4000\nnode 2 10.77.0.12:4000\nlink 0 1\nlink 1 2\n";
        let k = knobs_of(&settings_of(base), line, None, &dir).unwrap();
        assert_eq!(k["TESTBED_TOPOLOGY"].len(), 16);
        assert!(!k.contains_key("STRIMS_SIM_CAL_TOPOLOGY"));
        let other =
            std::env::temp_dir().join(format!("strims-testbed-knobs2-{}", std::process::id()));
        std::fs::create_dir_all(&other).unwrap();
        assert_eq!(
            knobs_of(&settings_of(base), line, None, &other).unwrap()["TESTBED_TOPOLOGY"],
            k["TESTBED_TOPOLOGY"]
        );
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&other).unwrap();
    }

    /// A run's joins are its twin's knob, to the ms its nodes took them,
    /// unless they draw an arrivals profile its caller names; a join that
    /// names no viewer of the run's is an error.
    #[test]
    fn a_runs_joins_are_its_twins_knob_unless_they_draw_its_arrivals() {
        let dir = fixture("joins");
        let settings = std::fs::read_to_string(dir.join("settings")).unwrap();
        let joins = |j: &str| {
            std::fs::write(dir.join("settings"), format!("{settings}JOINS={j}\n")).unwrap();
        };
        let ms = |m: u64| Some(Duration::from_millis(m));
        for (j, knob, at) in [
            ("1:4.0", "1:4", 9000),
            ("1:10.2504", "1:10.25", 15_250),
            ("1:0", "1:0", 5000),
        ] {
            joins(j);
            let run = Run::read(&dir, &BTreeMap::new()).unwrap();
            assert_eq!(run.knobs["STRIMS_SIM_CAL_JOINS"], knob);
            assert!(!run.knobs.keys().any(|k| k.starts_with("TESTBED_J")));
            let arrivals: Vec<Option<Duration>> =
                run.scenario().hosts.iter().map(|h| h.arrive_at).collect();
            assert_eq!(&arrivals[..2], [None, ms(at)], "JOINS={j}");
        }
        let drawn = BTreeMap::from([("STRIMS_SIM_ARRIVALS".to_owned(), "trickle".to_owned())]);
        let run = Run::read(&dir, &drawn).unwrap();
        assert_eq!(run.knobs.get("STRIMS_SIM_CAL_JOINS"), None);
        for bad in ["2:4", "1:4 1:5"] {
            joins(bad);
            assert!(Run::read(&dir, &BTreeMap::new()).is_err(), "JOINS={bad}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A run whose viewers arrive into the running stream (`JOINS`, or a
    /// `JOIN_AT` after the start) writes their joins, as its twin does,
    /// and one whose viewers are there from the start writes none; a
    /// testbed viewer starts once, so no run writes rejoins.
    #[test]
    fn a_run_with_arrivals_writes_its_joins_and_no_rejoins() {
        let dir = fixture("arrivals");
        let settings = std::fs::read_to_string(dir.join("settings")).unwrap();
        let row_of = |knobs: &BTreeMap<String, String>| -> serde_json::Map<String, Value> {
            let run = Run::read(&dir, knobs).unwrap();
            serde_json::from_str(&row(&run.report())).unwrap()
        };
        let none = BTreeMap::new();
        let r = row_of(&none);
        assert!(
            !r.keys()
                .any(|k| k.starts_with("join_") || k.starts_with("caught_up_")),
            "{r:?}"
        );
        std::fs::write(dir.join("settings"), format!("{settings}JOINS=1:0\n")).unwrap();
        let drawn = BTreeMap::from([("STRIMS_SIM_ARRIVALS".to_owned(), "trickle".to_owned())]);
        for knobs in [&none, &drawn] {
            let r = row_of(knobs);
            // The viewer arrived at the stream's start and read its first
            // frame 40 ms later, 40 ms behind its stamp.
            for k in [
                "join_p50_ms",
                "join_p90_ms",
                "caught_up_p50_ms",
                "caught_up_p90_ms",
            ] {
                assert_eq!(r[k], 40, "{k}: {r:?}");
            }
            assert_eq!(
                (
                    &r["arrived_honest"],
                    &r["join_unread"],
                    &r["caught_up_never"]
                ),
                (&Value::from(1), &Value::from(0), &Value::from(0))
            );
            assert!(!r.keys().any(|k| k.starts_with("rejoin_")), "{r:?}");
            assert_eq!(
                r.get("arrivals").and_then(Value::as_str),
                knobs.get("STRIMS_SIM_ARRIVALS").map(String::as_str)
            );
        }
        // A JOIN_AT after the start has its viewers arrive, as the twin's.
        std::fs::write(dir.join("settings"), format!("{settings}JOIN_AT=1\n")).unwrap();
        let run = Run::read(&dir, &none).unwrap();
        assert!(run.scenario().profiled());
        std::fs::write(dir.join("settings"), format!("{settings}JOIN_AT=0\n")).unwrap();
        assert!(!Run::read(&dir, &none).unwrap().scenario().profiled());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_sampler_that_stopped_leaves_its_columns_unmeasured_past_it() {
        let dir = fixture("stopped");
        let r = Run::read(&dir, &BTreeMap::new()).unwrap().report();
        // tc-0.log's last sample is 450 ms in: the drops end at the first
        // edge after it, the queue an interval (the median gap) on.
        let end = r.wire_ends[&(0, up("drops"))];
        assert!(end < r.wire_bins.len() - 1, "{end}");
        let w = &r.wan.as_ref().unwrap().hosts[0];
        assert_eq!(
            w.up_queue_until.unwrap().as_millis(),
            (LEAD + Duration::from_millis(450 + 200)).as_millis() as i64
        );
        let out = dir.join("tables");
        r.write_tables(&out, Duration::from_millis(100), true)
            .unwrap();
        let bins = crate::tables::read_table(&out, "bins").unwrap();
        let head: Vec<&str> = bins.lines().next().unwrap().split(',').collect();
        let col = |name: &str| head.iter().position(|c| *c == name).unwrap();
        let (t, drops, queue) = (col("t_ms"), col("seed_drops"), col("seed_up_queue_ms"));
        let (mut before, mut after) = (0, 0);
        for l in bins.lines().skip(1) {
            let f: Vec<&str> = l.split(',').collect();
            let ms: i64 = f[t].parse().unwrap();
            if ms > 1000 {
                assert_eq!((f[drops], f[queue]), ("", ""), "{l}");
                after += 1;
            } else if ms < 400 && !f[drops].is_empty() {
                before += 1;
            }
        }
        assert!(before > 0 && after > 0, "{before} {after}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// hack/testbed/testdata/tc's captures (its capture.sh: each link
    /// class's tree as run.sh builds a node's, under a flood so its queue
    /// stands and drops, as iproute2 6.19 prints it), each file's text.
    fn tc_fixture(name: &str) -> String {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../hack/testbed/testdata/tc");
        std::fs::read_to_string(dir.join(name)).unwrap()
    }

    /// An out dir of node 0's `links` line(s), if any, and its tc-0.log's
    /// samples: (Unix ms, tc's output, its lines joined).
    fn tc_run(name: &str, links: Option<&str>, samples: &[(u64, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("strims-tc-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        if let Some(l) = links {
            std::fs::write(dir.join("links"), l).unwrap();
        }
        let log: String = samples
            .iter()
            .map(|(ms, text)| {
                format!(
                    "{ms} {}\n",
                    text.split_whitespace().collect::<Vec<_>>().join(" ")
                )
            })
            .collect();
        std::fs::write(dir.join("tc-0.log"), log).unwrap();
        dir
    }

    /// Each class's queue is its modem's, the class's qdisc 10: on the
    /// device the run's `links` line names, in every device's JSON
    /// (tc-<i>.log's, tcsample's) and in an older run's of the modem's
    /// device alone: its drops, what it holds as its shaper charges it,
    /// and its rate (a trace's prio has none).
    #[test]
    fn each_classs_queue_is_its_modems_on_the_device_links_names() {
        // (capture, spec, modem's device, drops, held, rate) of each
        // class's every-device capture.
        let cases = [
            ("tbf", "tbf", "eth0", 22439, 29808, Some(12500)),
            ("cable", "cable", "ifbm", 29738, 12460, Some(12500)),
            (
                "cable_aqm_fifo",
                "cable,aqm=fifo",
                "ifbm",
                30383,
                29904,
                Some(12500),
            ),
            ("dsl", "dsl", "eth0", 30856, 28620, Some(12500)),
            (
                "dsl_line_vdsl",
                "dsl,line=vdsl",
                "eth0",
                32510,
                28934,
                Some(12250),
            ),
            ("fiber", "fiber", "eth0", 32603, 29904, Some(12500)),
            (
                "cable_router_fq_codel",
                "cable,router=fq_codel",
                "ifbm",
                20607,
                12751564,
                Some(11875),
            ),
            (
                "dsl_router_cake",
                "dsl,router=cake",
                "eth0",
                26720,
                2604420,
                Some(11875),
            ),
            (
                "trace",
                "trace,file=slow.us,format=us",
                "eth0",
                27434,
                29808,
                None,
            ),
        ];
        for (name, spec, dev, drops, held, rate) in cases {
            let links = format!("node 0 {spec} 10 20 100 {dev}\nnode 1 tbf 20 10 100 eth0\n");
            let every = tc_fixture(&format!("{name}.every.json"));
            let d = tc_run(name, Some(&links), &[(1000, &every)]);
            let u = uplink(&d, 0).unwrap();
            assert_eq!(
                (u.spec.as_str(), u.kind, u.dev.as_deref()),
                (spec, queue_kind(spec), Some(dev)),
                "{name}"
            );
            assert_eq!(
                u.samples,
                [ModemSample {
                    ms: 1000,
                    drops,
                    held,
                    rate
                }],
                "{name}"
            );
            assert_eq!(uplink(&d, 1).unwrap().dev.as_deref(), Some("eth0"));
            // An older run's log of the modem's device alone, no dev in it.
            let one = tc_fixture(&format!("{name}.json"));
            let d = tc_run(name, Some(&links), &[(1000, &one)]);
            let u = uplink(&d, 0).unwrap();
            assert_eq!(u.samples.len(), 1, "{name}");
            assert!(u.samples[0].drops > 0 && u.samples[0].held > 0, "{name}");
            assert_eq!(u.samples[0].rate, rate, "{name}");
            std::fs::remove_dir_all(&d).unwrap();
        }
        assert_eq!(
            [
                "tbf",
                "cable,aqm=fifo,slot=4-4",
                "cable,router=fq_codel",
                "dsl,line=vdsl,router=cake",
                "trace,file=up.mm,limit=30000",
            ]
            .map(queue_kind),
            ["tbf", "tbf", "tbf", "cake", "prio"]
        );
    }

    /// A 10: on a device that isn't the modem's isn't its queue, nor one of
    /// another kind than its class's: a log with samples and no queue is
    /// refused. A `links` line that names no device (a run from before it
    /// did) takes the 10: wherever it is, and a node `links` lacks is
    /// refused.
    #[test]
    fn a_queue_on_another_device_or_of_another_kind_is_refused() {
        let elsewhere =
            tc_fixture("tbf.every.json").replace("\"dev\":\"eth0\"", "\"dev\":\"ifbx\"");
        let d = tc_run(
            "dev",
            Some("node 0 tbf 10 20 100 eth0\n"),
            &[(1000, &elsewhere)],
        );
        let err = uplink(&d, 0).unwrap_err();
        assert!(err.contains("tbf link has no tbf 10: on eth0"), "{err}");
        let tbf = tc_fixture("tbf.json");
        let d = tc_run(
            "kind",
            Some("node 0 cable,router=cake 10 20 100\n"),
            &[(1000, &tbf)],
        );
        let err = uplink(&d, 0).unwrap_err();
        assert!(
            err.contains("cable,router=cake link has no cake 10: in its samples"),
            "{err}"
        );
        let cable = tc_fixture("cable.every.json");
        let d = tc_run("nodev", Some("node 0 cable 10 20 100\n"), &[(1000, &cable)]);
        let u = uplink(&d, 0).unwrap();
        assert_eq!((u.dev, u.samples.len()), (None, 1));
        assert_eq!(uplink(&d, 1), Err("links: no node 1".to_owned()));
        // A run without a log has no samples, and is no error.
        std::fs::remove_file(d.join("tc-0.log")).unwrap();
        assert_eq!(uplink(&d, 0).unwrap().samples, []);
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// A run from before the `links` file had a tbf on eth0, its log eth0's
    /// alone: as JSON, or older still as tc's text, whose sizes in KiB
    /// (`3Kb`) read as their bytes and whose rates in SI units as bytes a
    /// second. The log's samples keep its order, which is time order
    /// though `date`'s stamps ran backwards, and a line cut short is none.
    #[test]
    fn a_run_before_the_links_file_had_a_tbf_on_eth0() {
        let (json, kib, text) = (
            tc_fixture("tbf.json"),
            tc_fixture("tbf-kib.json"),
            tc_fixture("tbf-kib.txt"),
        );
        assert!(text.contains("backlog 3Kb 3p"));
        // The second's stamp is late (date's lost leading zero), the
        // third's line text, the fourth's torn.
        let d = tc_run(
            "old",
            None,
            &[
                (1_794_840_471_700, &json),
                (1_794_840_471_604, &kib),
                (1_794_840_471_800, &text),
                (1_794_840_471_900, "[{\"kind\":\"tbf\",\"handle\""),
            ],
        );
        let u = uplink(&d, 0).unwrap();
        assert_eq!((u.spec.as_str(), u.kind, u.dev), ("tbf", "tbf", None));
        let got: Vec<(u64, u64)> = u.samples.iter().map(|s| (s.ms, s.held)).collect();
        assert_eq!(
            got,
            [
                (1_794_840_471_700, 29808),
                (1_794_840_471_604, 3072),
                (1_794_840_471_800, 3072)
            ]
        );
        // The KiB tbf's JSON and its text read alike, at 1 Mbit/s.
        let (a, b) = (u.samples[1], u.samples[2]);
        assert_eq!((a.drops, a.held, a.rate), (b.drops, b.held, Some(125_000)));
        assert_eq!(a.rate, b.rate);
        // The text of another, and none of another kind in it.
        let line = "2000 qdisc tbf 10: root refcnt 2 rate 10Mbit burst 3028b lat 99ms  Sent 3000 bytes 2 pkt (dropped 4, overlimits 0 requeues 0)  backlog 1514b 1p requeues 0";
        assert_eq!(
            modem_of(line, "tbf", None),
            Some((2000, 4, 1514, Some(1_250_000)))
        );
        assert_eq!(modem_of(line, "cake", None), None);
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// A dsl router's cake under a flood of 1442-byte skbs (1428 IP bytes)
    /// holds what its shaper charges: each costs 1428 + 40 in ATM cells, 31
    /// of 53 bytes, so its queue drains at 1442 / 1643 of its bandwidth in
    /// skb bytes (0.879 measured over 10 s).
    #[test]
    fn cake_holds_what_its_shaper_charges() {
        let flood = tc_fixture("dsl-cake-flood.json");
        let qs: Vec<Value> = serde_json::from_str(&flood).unwrap();
        let cake = qs.iter().find(|q| q["kind"] == "cake").unwrap();
        let qlen = cake["qlen"].as_u64().unwrap();
        assert_eq!(cake["backlog"].as_u64(), Some(qlen * 1442));
        let d = tc_run(
            "cake",
            Some("node 0 dsl,router=cake 10 20 100 eth0\n"),
            &[(1000, &flood)],
        );
        assert_eq!(uplink(&d, 0).unwrap().samples[0].held, qlen * 1643);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn the_truth_takes_its_share_from_the_captures_near_saturated_windows() {
        let s = calibration_star(&Calibration::from_knobs(|_| None));
        let spec = &s.hosts[0];
        let link = crate::report::available(spec, &[], Duration::ZERO, Duration::from_secs(1));
        let sample = |secs: u64| MeterSample {
            t: Duration::from_secs(secs),
            asked: Duration::from_secs(secs),
            ..MeterSample::default()
        };
        // A second at 0.9 of its link, 10 of its 1000 segments resent; then
        // one at a tenth of it.
        let full = link * 9 / 10 / 1000;
        let mut sends: Vec<Send> = (0..1000u64)
            .map(|i| Send {
                us: 1_000_000 + 1 + i * 999,
                charge: full,
                app: Some(full - 60),
                est: 0,
                resent: i % 100 == 0,
            })
            .collect();
        sends.extend((0..100u64).map(|i| Send {
            us: 2_000_000 + 1 + i * 9_999,
            charge: full,
            app: Some(full - 60),
            est: 0,
            resent: false,
        }));
        let sends = Sent {
            sends,
            ..Sent::default()
        };
        // Lines without the runner's channel bytes (channel_bytes).
        let mut meter = vec![sample(1), sample(2), sample(3)];
        gate(&mut meter, &sends, spec, &[], |d| d.as_micros() as u64);
        assert!(!meter[0].gated.window);
        let g = meter[1].gated;
        assert!(g.window);
        assert_eq!((g.app, g.link), (990 * (full - 60), 990 * full));
        // The light second isn't one: the share stays the saturated one's.
        assert!(!meter[2].gated.window);
        assert_eq!(meter[2].gated.link, g.link);
        assert_eq!(meter[2].link_bytes, 1100 * full);
        assert_eq!(meter[2].resent_bytes, 10 * full);
        // The truth there is the link's second times that share, its
        // channel share 1 without the channel bytes.
        assert_eq!((g.data, g.vnic), (0, 0));
        let truth = |m: &MeterSample| {
            crate::report::upload_truth(spec, Some(m), &[], Duration::from_secs(4)).unwrap()
        };
        let share = g.app as f64 / g.link as f64;
        assert_eq!(truth(&meter[2]), (link as f64 * share) as u64);
        // With them, the saturated second's channel bytes over its vnic
        // bytes: 0.98 of what it handed vnic was its runner's.
        let counted = |secs: u64, data: u64, vnic: u64| MeterSample {
            channel_bytes: Some(data),
            vnic_bytes: vnic,
            ..sample(secs)
        };
        let mut meter = vec![
            counted(1, 1_000, 2_000),
            counted(2, 491_000, 502_000),
            counted(3, 500_000, 600_000),
        ];
        gate(&mut meter, &sends, spec, &[], |d| d.as_micros() as u64);
        assert_eq!(
            (meter[1].gated.data, meter[1].gated.vnic),
            (490_000, 500_000)
        );
        assert_eq!(
            meter[2].gated,
            Gated {
                window: false,
                ..meter[1].gated
            }
        );
        assert_eq!(truth(&meter[2]), (link as f64 * 0.98 * share) as u64);
        // A window whose earlier sample lacks them adds none.
        meter[0].channel_bytes = None;
        gate(&mut meter, &sends, spec, &[], |d| d.as_micros() as u64);
        assert_eq!((meter[2].gated.data, meter[2].gated.vnic), (0, 0));
    }

    /// A node whose capture holds datagrams takes its transports' bytes as
    /// its datagrams' payloads from their lengths, its TCP segments' as
    /// their payloads: over its near-saturated windows, its dropped
    /// datagrams' payloads off its bytes and their charges off its link's,
    /// as its TCP resends' are, in the window each was offered; and the
    /// truth takes the share they give. Without a stack its datagrams have
    /// no payload, and the share is its link's fallback.
    #[test]
    fn a_data_channels_share_is_its_datagrams_lengths_over_their_charge() {
        let s = calibration_star(&Calibration::from_knobs(|_| None));
        let spec = &s.hosts[0];
        let link = crate::report::available(spec, &[], Duration::ZERO, Duration::from_secs(1));
        // Seconds 1 and 2 of datagrams at 0.9 of its link, the first a TCP
        // segment resent, then one at a tenth of it.
        let full = link * 9 / 10 / 1000;
        let est = full - 100;
        let second = |at: u64, n: u64, est: u64| {
            (0..n).map(move |i| Send {
                us: at * 1_000_000 + 1 + i * (999_000 / n),
                charge: full,
                app: None,
                est,
                resent: false,
            })
        };
        let gated = |est: u64| {
            let mut sends: Vec<Send> = [(1, 1000), (2, 1000), (3, 100)]
                .into_iter()
                .flat_map(|(at, n)| second(at, n, est))
                .collect();
            if let Some(x) = sends.first_mut() {
                (x.app, x.est, x.resent) = (Some(full - 60), 0, true);
            }
            let sent = Sent {
                sends,
                dropped: vec![Dropped {
                    us: 2_500_000,
                    charge: full,
                    est,
                }],
            };
            let mut meter: Vec<MeterSample> = (1..=4u64)
                .map(|secs| MeterSample {
                    t: Duration::from_secs(secs),
                    asked: Duration::from_secs(secs),
                    ..MeterSample::default()
                })
                .collect();
            gate(&mut meter, &sent, spec, &[], |d| d.as_micros() as u64);
            meter
        };
        let meter = gated(est);
        assert!(meter[1].gated.window && meter[2].gated.window && !meter[3].gated.window);
        let g = meter[3].gated;
        assert_eq!((g.app, g.link), (1998 * est, 1998 * full));
        assert_eq!(meter[2].resent_bytes, 2 * full);
        assert_eq!(meter[2].app_bytes, 1998 * est);
        let truth = |m: &MeterSample| {
            crate::report::upload_truth(spec, Some(m), &[], Duration::from_secs(5))
        };
        let share = (1998 * est) as f64 / (1998 * full) as f64;
        assert_eq!(truth(&meter[3]), Some((link as f64 * share) as u64));
        // Without a stack, no payload: the fallback.
        let meter = gated(0);
        assert_eq!(meter[3].gated.app, 0);
        let fallback = crate::report::full_share(spec);
        assert_eq!(truth(&meter[3]), Some((link as f64 * fallback) as u64));
    }

    /// A data channel's lengths over its charge read at least what its node
    /// handed vnic over that charge, and as much without duplicates: a
    /// chunk SCTP resends that had arrived is a datagram the lengths count
    /// on both sides and vnic's bytes count once.
    #[test]
    fn a_data_channels_lengths_read_at_least_its_vnic_bytes() {
        let s = calibration_star(&Calibration::from_knobs(|_| None));
        let spec = &s.hosts[0];
        let link = crate::report::available(spec, &[], Duration::ZERO, Duration::from_secs(1));
        let full = link * 9 / 10 / 1000;
        let est = full - 100;
        for duplicates in [0, 100] {
            let sends = (0..1000u64)
                .map(|i| Send {
                    us: 1_000_001 + i * 999,
                    charge: full,
                    app: None,
                    est,
                    resent: false,
                })
                .collect();
            let sent = Sent {
                sends,
                dropped: Vec::new(),
            };
            let mut meter: Vec<MeterSample> = (1..=2u64)
                .map(|secs| MeterSample {
                    t: Duration::from_secs(secs),
                    asked: Duration::from_secs(secs),
                    ..MeterSample::default()
                })
                .collect();
            gate(&mut meter, &sent, spec, &[], |d| d.as_micros() as u64);
            let g = meter[1].gated;
            let lengths = g.app as f64 / g.link as f64;
            // What it handed vnic: its first transmissions' payloads.
            let vnic = ((1000 - duplicates) * est) as f64 / g.link as f64;
            assert!(vnic <= lengths, "{duplicates}: {vnic} > {lengths}");
            assert_eq!(vnic == lengths, duplicates == 0);
        }
    }

    /// A datagram whose length holds no DATA chunk's payload under its
    /// stack's headers is a SACK alone, out of the share; one of a node
    /// without a stack, and a TCP segment, aren't.
    #[test]
    fn a_sack_alone_is_out_of_the_share() {
        let datagram = |ip_len: u64| Packet {
            us: 0,
            src: Some(10),
            dst: Some(11),
            ip_len,
            pure_ack: false,
            payload: ip_len - 28,
            tcp: None,
            ip_id: 0,
            udp: Some((5000, 6000)),
        };
        let chromium = Some(SctpProfile::CHROMIUM);
        assert!(sack_alone(&datagram(78), chromium));
        assert!(!sack_alone(&datagram(1238), chromium));
        assert!(!sack_alone(&datagram(78), None));
        let segment = Packet {
            udp: None,
            tcp: Some((4000, 5000, 0)),
            ..datagram(52)
        };
        assert!(!sack_alone(&segment, chromium));
    }

    /// A datagram the node offered and its link didn't carry within
    /// DROP_WAIT_US is dropped, told apart by destination, IP id, length
    /// and ports; one carried later, or a TCP segment, isn't.
    #[test]
    fn a_dropped_datagram_is_one_offered_and_never_carried() {
        let datagram = |us: u64, ip_id: u16| Packet {
            us,
            src: Some(10),
            dst: Some(11),
            ip_len: 1238,
            pure_ack: false,
            payload: 1210,
            tcp: None,
            ip_id,
            udp: Some((5000, 6000)),
        };
        let offered = [
            datagram(1_000_000, 1),
            datagram(1_000_100, 2),
            datagram(1_000_200, 3),
            Packet {
                udp: None,
                tcp: Some((4000, 5000, 7)),
                ..datagram(1_000_300, 4)
            },
        ];
        // 1 carried 30 ms on, 3 past DROP_WAIT_US, 2 never.
        let carried = [
            datagram(1_030_000, 1),
            datagram(1_000_200 + DROP_WAIT_US + 1, 3),
        ];
        fn refs(ps: &[Packet]) -> Vec<&Packet> {
            ps.iter().collect()
        }
        let framing = strims_wan::framing::Framing::NONE;
        let d = dropped(
            &refs(&offered),
            &refs(&carried),
            framing,
            Some(SctpProfile::CHROMIUM),
        );
        let got: Vec<(u64, u64, u64)> = d.iter().map(|x| (x.us, x.charge, x.est)).collect();
        let charge = framing.packet(1238) as u64;
        assert_eq!(got, [(1_000_100, charge, 1160), (1_000_200, charge, 1160)]);
        assert!(dropped(&refs(&offered), &refs(&offered), framing, None).is_empty());
    }

    /// A Chromium datagram's payload as its length gives it: a full one's
    /// is the most a chunk carries, a SACK alone's none.
    #[test]
    fn a_datagrams_length_gives_its_chunks_payload() {
        let x = SctpProfile::CHROMIUM;
        assert_eq!(x.datagram_payload(1238), x.max_payload as u64);
        assert_eq!(x.datagram_payload(78), 0);
        assert_eq!(x.datagram_payload(40), 0);
    }

    #[test]
    fn a_lines_meter_record_reads_back_its_channel_and_vnic_bytes() {
        let dir = fixture("meter-bytes");
        let r = Run::read(&dir, &BTreeMap::new()).unwrap();
        // As a node's line writes it (MeterSample::json, native and
        // browser nodes alike), its clock in Unix ms.
        let t = Duration::from_millis(START + 1000);
        let m = MeterSample {
            t,
            asked: t,
            rate: 1000,
            capacity: 5000,
            bytes_written: 700,
            channel_bytes: Some(650),
            ..MeterSample::default()
        };
        let read = |m: &MeterSample| {
            let v: Value = serde_json::from_str(&m.json()).unwrap();
            r.meter_of(&v).unwrap()
        };
        let got = read(&m);
        assert_eq!(
            (got.channel_bytes, got.vnic_bytes, got.bytes_written),
            (Some(650), 700, 700)
        );
        // A line without them.
        let got = read(&MeterSample {
            channel_bytes: None,
            ..m
        });
        assert_eq!((got.channel_bytes, got.vnic_bytes), (None, 700));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_capture_that_stopped_leaves_its_columns_unmeasured_past_it() {
        let dir = fixture("pcap-end");
        // tcpdump on the seed exited 450 ms in, its node running on.
        std::fs::write(dir.join("pcap-0.end"), format!("{} 1\n", START + 450)).unwrap();
        let r = Run::read(&dir, &BTreeMap::new()).unwrap().report();
        let end = r.wire_ends[&(0, up("up_bytes"))];
        assert!(end < r.wire_bins.len() - 1, "{end}");
        for c in PCAP_COUNTERS {
            assert_eq!(r.wire_ends.get(&(0, up(c))), Some(&end), "{c}");
        }
        // The viewer's capture ran to the end.
        assert!(!r.wire_ends.contains_key(&(1, up("up_bytes"))));
        let out = dir.join("tables");
        r.write_tables(&out, Duration::from_millis(100), true)
            .unwrap();
        let bins = crate::tables::read_table(&out, "bins").unwrap();
        let head: Vec<&str> = bins.lines().next().unwrap().split(',').collect();
        let col = |name: &str| head.iter().position(|c| *c == name).unwrap();
        let (t, bytes, acks) = (col("t_ms"), col("seed_up_bytes"), col("seed_up_ack_bytes"));
        let (mut before, mut after) = (0, 0);
        for l in bins.lines().skip(1) {
            let f: Vec<&str> = l.split(',').collect();
            let ms: i64 = f[t].parse().unwrap();
            if ms > 1000 {
                assert_eq!((f[bytes], f[acks]), ("", ""), "{l}");
                after += 1;
            } else if ms < 400 {
                assert!(!f[bytes].is_empty(), "{l}");
                before += 1;
            }
        }
        assert!(before > 0 && after > 0, "{before} {after}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_run_without_captures_leaves_them_unmeasured() {
        let dir = fixture("unmeasured");
        for i in 0..2 {
            std::fs::remove_file(dir.join(format!("pcap-{i}.pcap"))).unwrap();
            std::fs::remove_file(dir.join(format!("tc-{i}.log"))).unwrap();
        }
        let r = Run::read(&dir, &BTreeMap::new()).unwrap().report();
        for c in [
            "up_bytes",
            "up_packets",
            "small_packets",
            "up_ack_bytes",
            "drops",
            "retransmits",
        ] {
            assert!(r.wire_unmeasured.contains(&up(c)), "{c}");
        }
        assert!(r.wan.as_ref().unwrap().hosts[0].up_queue.is_empty());
        let out = dir.join("tables");
        r.write_tables(&out, Duration::from_millis(100), true)
            .unwrap();
        // The tables leave them empty, not 0.
        let bins = crate::tables::read_table(&out, "bins").unwrap();
        let head: Vec<&str> = bins.lines().next().unwrap().split(',').collect();
        let col = head.iter().position(|c| *c == "seed_up_bytes").unwrap();
        assert!(bins.lines().count() >= 2);
        assert!(
            bins.lines()
                .skip(1)
                .all(|l| l.split(',').nth(col) == Some(""))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A run whose settings don't say its nodes made their links at their
    /// joins all at once (JOIN_LINKS=together) made them one at a time, and
    /// keys apart; one that says so doesn't.
    #[test]
    fn a_run_whose_nodes_linked_one_at_a_time_keys_apart() {
        let dir = fixture("join-links");
        let run = Run::read(&dir, &BTreeMap::new()).unwrap();
        assert_eq!(run.knobs.get("TESTBED_JOIN_LINKS"), None);
        let settings = std::fs::read_to_string(dir.join("settings")).unwrap();
        for (setting, key) in [("", "serial"), ("JOIN_LINKS=awaited\n", "awaited")] {
            std::fs::write(
                dir.join("settings"),
                settings.replace("JOIN_LINKS=together\n", setting),
            )
            .unwrap();
            assert_eq!(
                Run::read(&dir, &BTreeMap::new()).unwrap().knobs["TESTBED_JOIN_LINKS"],
                key
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_run_not_stamped_by_the_clock_keys_apart() {
        let dir = fixture("stamps");
        let run = Run::read(&dir, &BTreeMap::new()).unwrap();
        assert_eq!(run.knobs.get("TESTBED_STAMPS"), None);
        // Settings that name no clock, or another: run.sh's date stamps.
        let settings = std::fs::read_to_string(dir.join("settings")).unwrap();
        std::fs::write(dir.join("settings"), settings.replace("STAMPS=clock\n", "")).unwrap();
        assert_eq!(
            Run::read(&dir, &BTreeMap::new()).unwrap().knobs["TESTBED_STAMPS"],
            "date"
        );
        std::fs::write(
            dir.join("settings"),
            settings.replace("STAMPS=clock", "STAMPS=uutils"),
        )
        .unwrap();
        assert_eq!(
            Run::read(&dir, &BTreeMap::new()).unwrap().knobs["TESTBED_STAMPS"],
            "uutils"
        );
        // The clock's, but a sampler's stamps go back all the same.
        std::fs::write(dir.join("settings"), &settings).unwrap();
        let tc = std::fs::read_to_string(dir.join("tc-1.log")).unwrap();
        let first = tc.lines().next().unwrap();
        let (ms, rest) = first.split_once(' ').unwrap();
        let ms: u64 = ms.parse().unwrap();
        std::fs::write(dir.join("tc-1.log"), format!("{} {rest}\n{tc}", ms + 433)).unwrap();
        assert_eq!(
            Run::read(&dir, &BTreeMap::new()).unwrap().knobs["TESTBED_STAMPS"],
            "back"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_testbed_run_skips_the_faults_a_run_of_its_scenario_would() {
        let mut s = calibration_star(&Calibration::from_knobs(|_| None));
        let (v0, v1, v2) = (1, 2, 3);
        for v in [v0, v1, v2] {
            s.at(Duration::ZERO, Fault::Crash(v));
            s.at(Duration::from_secs(7), Fault::Restart(v));
        }
        s.hosts[v2].arrive_at = Some(Duration::from_secs(9));
        let end = s.duration;
        s.at(Duration::from_secs(8), Fault::Crash(v2)); // not there yet
        s.at(Duration::from_secs(10), Fault::Restart(v1)); // up
        s.at(
            Duration::from_secs(11),
            Fault::Depart(v0, crate::Departure::Graceful),
        );
        s.at(
            Duration::from_secs(12),
            Fault::Depart(v0, crate::Departure::Abrupt),
        ); // again
        s.at(Duration::from_secs(13), Fault::Crash(v0)); // departed
        s.at(end, Fault::Crash(v1)); // unreached
        s.faults.sort_by_key(|f| f.at);
        let skipped: Vec<String> = skipped_faults(&s)
            .into_iter()
            .map(|i| format!("{:?}@{}", s.faults[i].fault, s.faults[i].at.as_secs()))
            .collect();
        assert_eq!(
            skipped,
            [
                // v2's crash and restart, before it arrives.
                format!("Crash({v2})@0"),
                format!("Restart({v2})@7"),
                format!("Crash({v2})@8"),
                format!("Restart({v1})@10"),
                format!("Depart({v0}, Abrupt)@12"),
                format!("Crash({v0})@13"),
                format!("Crash({v1})@{}", end.as_secs()),
            ]
        );
    }

    #[test]
    fn a_tail_loss_probes_resend_is_one_though_its_original_never_reached_the_link() {
        let dir = fixture("tlp");
        let us = |ms: u64| (START + ms) * 1000;
        // The seed offers three segments, the queue drops the third, and a
        // tail-loss probe resends it 200 ms on: the link carries the first
        // two and the probe, never the third's original.
        let seg = |seq: u32| tcp_seq(10, 11, 1000, seq);
        let offered = [
            (us(1100), seg(0)),
            (us(1101), seg(1000)),
            (us(1102), seg(2000)),
            (us(1302), seg(2000)),
        ];
        let carried = [
            (us(1110), seg(0)),
            (us(1111), seg(1000)),
            (us(1312), seg(2000)),
        ];
        std::fs::write(dir.join("pcap-0.pcap"), pcap(&carried)).unwrap();
        let resent = || {
            let r = Run::read(&dir, &BTreeMap::new()).unwrap().report();
            let framing = r.scenario.hosts[0].site.unwrap().profile.framing;
            let m = &r.hosts[0].runner.meter;
            (
                m.last().unwrap().resent_bytes,
                framing.packet(20 + 32 + 1000) as u64,
            )
        };
        // Without the offered capture the probe reads as a first
        // transmission: the link never carried the bytes before it.
        assert_eq!(resent().0, 0);
        std::fs::write(dir.join("pcap-0-offered.pcap"), pcap(&offered)).unwrap();
        let (got, one) = resent();
        assert_eq!(got, one);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_gso_buffers_resend_counts_in_each_segment_the_link_carried() {
        let dir = fixture("gso");
        let us = |ms: u64| (START + ms) * 1000;
        // The seed offers a 2000-byte GSO buffer, which the queue drops
        // whole, and offers it again 200 ms on; the queue's device splits
        // the resend into the two 1000-byte segments the link carries. The
        // link never carried those bytes before, yet both are resends.
        let carried = [
            (us(1310), tcp_seq(10, 11, 1000, 0)),
            (us(1311), tcp_seq(10, 11, 1000, 1000)),
        ];
        let offered = [
            (us(1100), tcp_seq(10, 11, 2000, 0)),
            (us(1300), tcp_seq(10, 11, 2000, 0)),
        ];
        std::fs::write(dir.join("pcap-0.pcap"), pcap(&carried)).unwrap();
        let resent = || {
            let r = Run::read(&dir, &BTreeMap::new()).unwrap().report();
            let framing = r.scenario.hosts[0].site.unwrap().profile.framing;
            let m = &r.hosts[0].runner.meter;
            (
                m.last().unwrap().resent_bytes,
                framing.packet(20 + 32 + 1000) as u64,
            )
        };
        assert_eq!(resent().0, 0);
        std::fs::write(dir.join("pcap-0-offered.pcap"), pcap(&offered)).unwrap();
        let (got, one) = resent();
        assert_eq!(got, 2 * one);
        // Offered once, the same split is two first transmissions.
        std::fs::write(dir.join("pcap-0-offered.pcap"), pcap(&offered[..1])).unwrap();
        assert_eq!(resent().0, 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_sampler_that_failed_unseen_refuses_the_run() {
        let dir = fixture("samplers");
        let read = || Run::read(&dir, &BTreeMap::new()).map(|_| ());
        let tc1 = std::fs::read_to_string(dir.join("tc-1.log")).unwrap();
        // Empty: its sampler wrote nothing.
        std::fs::write(dir.join("tc-1.log"), "").unwrap();
        assert!(read().unwrap_err().contains("tc-1.log is empty"));
        // Unless run.sh's marker says it exited (its columns end there).
        std::fs::write(dir.join("tcsample-1.end"), format!("{} 1\n", START)).unwrap();
        assert!(read().is_ok());
        std::fs::remove_file(dir.join("tcsample-1.end")).unwrap();
        // Stopped seconds before its node's last line.
        let (_, rest) = tc1.lines().next().unwrap().split_once(' ').unwrap();
        std::fs::write(dir.join("tc-1.log"), format!("{} {rest}\n", START - 3000)).unwrap();
        assert!(
            read()
                .unwrap_err()
                .contains("tc-1.log stops 4000 ms before node 1's last line")
        );
        std::fs::write(dir.join("tc-1.log"), &tc1).unwrap();
        // Samples (tc's JSON, each qdisc with its device), but none of the
        // modem on the device `links` names.
        let json = |ms: u64| {
            format!(
                "{} [{{\"kind\":\"tbf\",\"handle\":\"10:\",\"dev\":\"eth0\",\"root\":true,\"options\":{{\"rate\":1250000}},\"bytes\":3000,\"packets\":2,\"drops\":4,\"overlimits\":0,\"requeues\":0,\"backlog\":1514,\"qlen\":1}}]\n",
                START + ms
            )
        };
        let tc0 = std::fs::read_to_string(dir.join("tc-0.log")).unwrap();
        std::fs::write(dir.join("tc-0.log"), json(150) + &json(1500)).unwrap();
        let links = std::fs::read_to_string(dir.join("links")).unwrap();
        std::fs::write(
            dir.join("links"),
            links.replace("node 0 tbf 10 20 100", "node 0 tbf 10 20 100 eth9"),
        )
        .unwrap();
        assert_eq!(
            read().unwrap_err(),
            "tc-0.log: node 0's tbf link has no tbf 10: on eth9 in its samples"
        );
        // Nor does a sampler that stopped early: its samples still show
        // where the modem is.
        std::fs::write(dir.join("tcsample-0.end"), format!("{} 1\n", START + 1600)).unwrap();
        assert_eq!(
            read().unwrap_err(),
            "tc-0.log: node 0's tbf link has no tbf 10: on eth9 in its samples"
        );
        std::fs::remove_file(dir.join("tcsample-0.end")).unwrap();
        // A links file without a node's line.
        std::fs::write(dir.join("links"), "node 0 tbf 10 20 100\n").unwrap();
        assert_eq!(read().unwrap_err(), "links: no node 1");
        std::fs::write(dir.join("links"), &links).unwrap();
        std::fs::write(dir.join("tc-0.log"), &tc0).unwrap();
        // A node without a tc log is unmeasured alone: the other's drops
        // stay.
        std::fs::remove_file(dir.join("tc-1.log")).unwrap();
        let r = Run::read(&dir, &BTreeMap::new()).unwrap().report();
        assert_eq!(r.wire_ends.get(&(1, up("drops"))), Some(&0));
        assert_ne!(r.wire_ends.get(&(0, up("drops"))), Some(&0));
        assert!(!r.wire_unmeasured.contains(&up("drops")));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_competing_upload_starts_at_its_recorded_times() {
        let dir = fixture("compete-start");
        let settings = std::fs::read_to_string(dir.join("settings")).unwrap();
        std::fs::write(dir.join("settings"), format!("{settings}COMPETE=3:2\n")).unwrap();
        // Launched 3 s in; iperf3's report 180 ms later; its first data in
        // the seed's capture (a control message's 100 bytes first) 120 ms
        // after that.
        std::fs::write(dir.join("compete.log"), format!("{} 3 2\n", START + 3000)).unwrap();
        std::fs::write(
            dir.join("compete-3.json"),
            format!(
                "{{\"start\":{{\"timestamp\":{{\"timemillisecs\":{}}}}}}}",
                START + 3180
            ),
        )
        .unwrap();
        let us = |ms: u64| (START + ms) * 1000;
        let capture = pcap(&[
            (us(3200), tcp(10, 250, 100, 0x18)),
            (us(3300), tcp(10, 250, 1448, 0x10)),
        ]);
        std::fs::write(dir.join("pcap-0.pcap"), &capture).unwrap();
        let c = competing_starts(&dir);
        assert_eq!(
            (c[0].launch, c[0].report, c[0].data, c[0].secs),
            (START + 3000, Some(START + 3180), Some(START + 3300), 2.0)
        );
        let r = Run::read(&dir, &BTreeMap::new()).unwrap();
        // The twin's: the client's start, in ms of stream (it adds the
        // setup's round trips).
        assert_eq!(r.knobs["STRIMS_SIM_CAL_COMPETE_MS"], "3180:2");
        assert!(!r.knobs.contains_key("TESTBED_SCHEDULE"));
        // The run's own: its first data, else its client's start and three
        // of its round trips.
        let start = |r: &Run| r.scenario().hosts[0].competing[0].start;
        assert_eq!(start(&r), r.at(START + 3300).unwrap());
        std::fs::remove_file(dir.join("pcap-0.pcap")).unwrap();
        let r = Run::read(&dir, &BTreeMap::new()).unwrap();
        let rtt = r.scenario().hosts[0].competing[0].rtt;
        assert_eq!(start(&r), r.at(START + 3180).unwrap() + rtt * 3);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_capture_that_stopped_before_an_uploads_end_cant_fail_it() {
        let dir = fixture("compete-cut");
        let settings = std::fs::read_to_string(dir.join("settings")).unwrap();
        std::fs::write(dir.join("settings"), format!("{settings}COMPETE=3:2\n")).unwrap();
        std::fs::write(dir.join("compete.log"), format!("{} 3 2\n", START + 3000)).unwrap();
        std::fs::write(
            dir.join("compete-3.json"),
            format!(
                "{{\"start\":{{\"timestamp\":{{\"timemillisecs\":{}}}}}}}",
                START + 3180
            ),
        )
        .unwrap();
        // The seed's capture holds none of the upload's data.
        let us = |ms: u64| (START + ms) * 1000;
        std::fs::write(
            dir.join("pcap-0.pcap"),
            pcap(&[(us(1000), tcp(10, 11, 1448, 0x10))]),
        )
        .unwrap();
        let failed = || competing_starts(&dir)[0].failed;
        assert!(failed());
        // Its tcpdump exited 3.1 s in, while the seed ran: before the
        // upload's data could show.
        std::fs::write(dir.join("pcap-0.end"), format!("{} 1\n", START + 3100)).unwrap();
        assert!(!failed());
        // One that ran to the upload's end saw none of it.
        std::fs::write(dir.join("pcap-0.end"), format!("{} 1\n", START + 5000)).unwrap();
        assert!(failed());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_step_or_upload_off_its_schedule_keys_the_run_apart() {
        let dir = fixture("schedule");
        let settings = std::fs::read_to_string(dir.join("settings")).unwrap();
        std::fs::write(
            dir.join("settings"),
            format!("{settings}SEED_UP_MBIT=10\nSEED_UP_STEPS=2:5\nCOMPETE=3:2\n"),
        )
        .unwrap();
        // Due at start + 2 s + the seed's 20 ms, and at start + 3 s; the
        // upload judged by its report's time (no capture: no data to find).
        std::fs::remove_file(dir.join("pcap-0.pcap")).unwrap();
        let step = |ms: u64| format!("{} 5\n", START + ms);
        let compete = |ms: u64| {
            let report = format!(
                "{{\"start\":{{\"timestamp\":{{\"timemillisecs\":{}}}}}}}",
                START + ms
            );
            std::fs::write(dir.join("compete-3.json"), report).unwrap();
            format!("{} 3 2\n", START + ms - 150)
        };
        let read = |s: &str, c: &str| {
            std::fs::write(dir.join("steps.log"), s).unwrap();
            std::fs::write(dir.join("compete.log"), c).unwrap();
            Run::read(&dir, &BTreeMap::new())
                .unwrap()
                .knobs
                .get("TESTBED_SCHEDULE")
                .cloned()
        };
        assert_eq!(read(&step(2100), &compete(3050)), None);
        assert_eq!(
            read(&step(1700), &compete(3050)).unwrap(),
            "step 1 320 ms early"
        );
        assert_eq!(
            read(&step(2100), &compete(6000)).unwrap(),
            "competing upload 1 3000 ms late"
        );
        assert_eq!(read("", &compete(3050)).unwrap(), "step 1 not logged");
        // An upload iperf3 failed (its report's `error`) keys the run apart,
        // and its twin has none of it.
        let c = compete(3050);
        std::fs::write(
            dir.join("compete-3.json"),
            r#"{"start":{},"error":"unable to connect"}"#,
        )
        .unwrap();
        std::fs::write(dir.join("steps.log"), step(2100)).unwrap();
        std::fs::write(dir.join("compete.log"), c).unwrap();
        let r = Run::read(&dir, &BTreeMap::new()).unwrap();
        assert_eq!(r.knobs["TESTBED_SCHEDULE"], "competing upload 1 failed");
        assert_eq!(r.knobs["STRIMS_SIM_CAL_COMPETE_MS"], "none");
        assert!(r.scenario().hosts[0].competing.is_empty());
        // A twin of its knobs has none either, though they keep its
        // schedule.
        assert_eq!(r.knobs["STRIMS_SIM_CAL_COMPETE"], "3:2");
        let twin = Calibration::from_knobs(|k| r.knobs.get(k).cloned());
        assert!(twin.compete.is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The truth's log of a competing upload holds each of its packets at
    /// its capture time: a window counts what the link carried in it,
    /// however it falls on the bins' edges, and the node's other packets
    /// and the sink's are none of it.
    #[test]
    fn the_truths_competitor_log_holds_each_packet_at_its_capture_time() {
        let packet = |us: u64, src: u8, dst: u8| Packet {
            us: START * 1000 + us,
            src: Some(src),
            dst: Some(dst),
            ip_len: 1500,
            pure_ack: false,
            payload: 1448,
            tcp: Some((4000, 5201, 0)),
            ip_id: 0,
            udp: None,
        };
        // From 20.005 s, a packet to the sink every 10 ms, one to a viewer
        // and one from the sink between each.
        let mut ps = Vec::new();
        for i in 0..500u64 {
            let us = 20_005_000 + i * 10_000;
            ps.extend([
                packet(us, 10, SINK),
                packet(us + 1, 10, 11),
                packet(us + 2, SINK, 10),
            ]);
        }
        let log = competing_log(&ps, Some(10), START);
        assert_eq!(log.len(), 500);
        let at = |ms: u64| strims_base::time::Duration::from(LEAD + Duration::from_millis(ms));
        assert_eq!(log.first(), Some(&(at(20_005), 1500)));
        assert_eq!(log.last().map(|x| x.1), Some(500 * 1500));
        let between = |a: u64, b: u64| {
            crate::report::competing_before(&log, at(b))
                - crate::report::competing_before(&log, at(a))
        };
        assert_eq!(between(20_000, 21_000), 100 * 1500);
        // A window ending on an edge takes the packets up to it.
        assert_eq!(between(19_100, 20_100), 10 * 1500);
        assert_eq!(between(20_100, 21_100), 100 * 1500);
        assert_eq!(between(24_900, 25_900), 10 * 1500);
    }

    /// A table's rows, by its header's column names.
    fn table_rows(dir: &Path, name: &str) -> Vec<BTreeMap<String, String>> {
        let csv = crate::tables::read_table(dir, name).unwrap();
        let mut lines = csv.lines();
        let head: Vec<String> = lines
            .next()
            .unwrap()
            .split(',')
            .map(str::to_owned)
            .collect();
        lines
            .map(|l| {
                head.iter()
                    .cloned()
                    .zip(l.split(',').map(str::to_owned))
                    .collect()
            })
            .collect()
    }

    /// A run whose nodes kept their scheduler's logs (FORECAST=1, SPEC=1)
    /// keys its twin with the sim's knobs, and its lines' logs fill the log
    /// tables as a sim run's records fill them: each record's time from
    /// the stream's start, its hosts by index, the caps' losses in
    /// series_meta. A run whose knobs are off writes none of the tables,
    /// and its nodes' lines hold no logs.
    #[test]
    fn a_runs_logs_fill_the_log_tables_its_twin_writes() {
        use strims_binmap::Bin;
        use strims_ppspp::{
            AdmitRule, HaveRecord, Held, HeldHow, LagForecast, PeerKey, RequestChannel, RequestEnd,
            RequestRecord, SeedDecision, SeedDecisionKind, SeedRequest, SeedSend, SendFate,
            SendHow,
        };
        use strims_testbed::record::logs::Lost;

        let dir = fixture("logs");
        let origin = Time::from_unix_millis(START as i64);
        let at = |ms: i64| origin + strims_base::time::Duration::from_millis(ms);
        let peer = PeerKey::default();
        let channel = |announced| RequestChannel {
            peer,
            announced,
            predicted: None,
            rtt: strims_base::time::Duration::from_millis(20),
            channel_open: at(-100),
        };
        let viewer = Logs {
            forecasts: vec![(
                Some(0),
                LagForecast {
                    at: at(310),
                    peer,
                    stream: 0,
                    bin: Bin(2),
                    sample: strims_base::time::Duration::from_millis(30),
                    estimate: None,
                    lazy: false,
                    first_sight: false,
                    channel_open: at(-100),
                    run: 0,
                },
            )],
            held: vec![(
                Some(0),
                Some(0),
                None,
                Held {
                    bin: Bin(0),
                    at: at(300),
                    peer: Some(peer),
                    how: HeldHow::Push,
                    parent: Some(peer),
                    expected: Some(at(290)),
                    ..Held::default()
                },
            )],
            haves: vec![(
                Some(0),
                HaveRecord {
                    at: at(305),
                    peer,
                    stream: 0,
                    bin: Bin(0),
                    channel_open: at(-100),
                },
            )],
            asks: vec![(
                Some(0),
                vec![None],
                RequestRecord {
                    at: at(200),
                    stream: 0,
                    bin: Bin(4),
                    timeout: strims_base::time::Duration::from_millis(250),
                    released: Some(at(199)),
                    asked: channel(Some(at(150))),
                    rivals: vec![channel(None)],
                    end: RequestEnd::Answered,
                    ended: at(240),
                },
            )],
            lost: Lost {
                forecasts: 2,
                ..Lost::default()
            },
            ..Logs::default()
        };
        let seed = Logs {
            seed_decisions: vec![(
                Some(1),
                SeedDecision {
                    at: at(100),
                    peer: Some(peer),
                    kind: SeedDecisionKind::Request(SeedRequest {
                        stream: 0,
                        granted: true,
                        rule: AdmitRule::Unmeasured,
                        subscribers: 0,
                        holders: None,
                        copies: None,
                        held: None,
                        per_holder: None,
                    }),
                },
            )],
            seed_sends: vec![(
                Some(1),
                SeedSend {
                    peer,
                    stream: 0,
                    bin: Bin(0),
                    how: SendHow::Push,
                    queued: at(150),
                    ended: at(160),
                    fate: SendFate::Written,
                    bytes: 1024,
                    cancel: None,
                    declared: false,
                    subscribed: true,
                },
            )],
            ..Logs::default()
        };
        // Each node's first line carries its logs.
        for (i, logs) in [(0, &seed), (1, &viewer)] {
            let path = dir.join(format!("node-{i}.jsonl"));
            let text = std::fs::read_to_string(&path).unwrap();
            let mut lines: Vec<Value> = text
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect();
            let v: Value = serde_json::from_str(&logs.json(origin)).unwrap();
            lines[0]["rec"]["logs"] = v;
            let out: String = lines.iter().map(|l| format!("{l}\n")).collect();
            std::fs::write(&path, out).unwrap();
        }
        let settings = std::fs::read_to_string(dir.join("settings")).unwrap();
        let bin = Duration::from_millis(BIN_MS.unsigned_abs());
        // Off: no log table, though the lines held logs.
        let off = dir.join("tables-off");
        Run::read(&dir, &BTreeMap::new())
            .unwrap()
            .report()
            .write_tables(&off, bin, true)
            .unwrap();
        for t in [
            "forecast",
            "arrivals",
            "haves",
            "asks",
            "seed_decisions",
            "seed_sends",
        ] {
            assert!(crate::tables::read_table(&off, t).is_err(), "{t}");
        }
        std::fs::write(
            dir.join("settings"),
            format!("{settings}FORECAST=1\nSPEC=1\n"),
        )
        .unwrap();
        let run = Run::read(&dir, &BTreeMap::new()).unwrap();
        assert_eq!(
            (
                run.knobs["STRIMS_SIM_FORECAST"].as_str(),
                run.knobs["STRIMS_SIM_SPEC"].as_str()
            ),
            ("1", "1")
        );
        assert!(
            !run.knobs
                .keys()
                .any(|k| k == "TESTBED_SPEC" || k == "TESTBED_FORECAST")
        );
        let r = run.report();
        assert_eq!(r.hosts[1].arrivals[0].at, LEAD + Duration::from_millis(300));
        assert_eq!(r.hosts[1].held, [(0, LEAD + Duration::from_millis(300))]);
        assert_eq!(r.hosts[1].forecasts[0].peer, Some(0));
        assert_eq!(r.hosts[1].requests[0].rivals, [None]);
        assert_eq!(
            r.hosts[0].seed_sends[0].ended,
            LEAD + Duration::from_millis(160)
        );
        let on = dir.join("tables-on");
        r.write_tables(&on, bin, true).unwrap();
        let col = |t: &str, c: &str| -> Vec<String> {
            table_rows(&on, t).iter().map(|r| r[c].clone()).collect()
        };
        assert_eq!(col("arrivals", "t_ms"), ["300"]);
        assert_eq!(
            (col("arrivals", "how"), col("arrivals", "expected_ms")),
            (vec!["push".to_owned()], vec!["290".to_owned()])
        );
        assert_eq!(
            (col("forecast", "t_ms"), col("forecast", "seen_ms")),
            (vec!["310".to_owned()], vec!["280".to_owned()])
        );
        assert_eq!(col("haves", "t_ms"), ["305"]);
        assert_eq!(col("asks", "picked"), ["true", "false"]);
        assert_eq!(col("asks", "have_ms"), ["150", ""]);
        assert_eq!(col("seed_decisions", "kind"), ["request"]);
        assert_eq!(col("seed_decisions", "peer"), ["1"]);
        assert_eq!(col("seed_sends", "t_ms"), ["160"]);
        assert_eq!(col("seed_sends", "queued_ms"), ["150"]);
        let meta: Value =
            serde_json::from_str(&std::fs::read_to_string(on.join("series_meta.json")).unwrap())
                .unwrap();
        assert_eq!(meta["forecasts_lost"], 2);
        assert_eq!(meta["held_lost"], 0);
        // A knob the sim has no value for is refused.
        std::fs::write(dir.join("settings"), format!("{settings}SPEC=2\n")).unwrap();
        assert!(
            Run::read(&dir, &BTreeMap::new())
                .err()
                .unwrap()
                .contains("SPEC=2: 1 or 0")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A run whose nodes kept their parent checks (PARENT_CHECKS=1) keys
    /// its twin with the sim's knob, and its lines' checks fill the
    /// `parent_checks` table as a sim run's decision logs fill it: each
    /// check's time in µs from the stream's start, its hosts by index, a
    /// graft's chosen peer its best, its losses in series_meta. A run
    /// whose knob is off writes no table.
    #[test]
    fn a_runs_parent_checks_fill_the_table_its_twin_writes() {
        let dir = fixture("checks");
        let path = dir.join("node-1.jsonl");
        let text = std::fs::read_to_string(&path).unwrap();
        let mut lines: Vec<Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let us = (START + 310) * 1000 + 250;
        lines[0]["rec"]["checks"] = serde_json::from_str(&format!(
            "[[{us},3,\"graft\",0,12000,null,null,null,null,null,0,0,0,1,0,0]]"
        ))
        .unwrap();
        lines[0]["rec"]["checks_lost"] = Value::from(1);
        let out: String = lines.iter().map(|l| format!("{l}\n")).collect();
        std::fs::write(&path, out).unwrap();
        let bin = Duration::from_millis(BIN_MS.unsigned_abs());
        let off = dir.join("tables-off");
        Run::read(&dir, &BTreeMap::new())
            .unwrap()
            .report()
            .write_tables(&off, bin, true)
            .unwrap();
        assert!(crate::tables::read_table(&off, "parent_checks").is_err());
        let settings = std::fs::read_to_string(dir.join("settings")).unwrap();
        std::fs::write(dir.join("settings"), format!("{settings}PARENT_CHECKS=1\n")).unwrap();
        let run = Run::read(&dir, &BTreeMap::new()).unwrap();
        assert_eq!(run.knobs["STRIMS_SIM_PARENT_CHECKS"], "1");
        let r = run.report();
        assert_eq!(r.parent_checks.len(), 1);
        assert_eq!(r.parent_checks[0].at, LEAD + Duration::from_micros(310_250));
        let on = dir.join("tables-on");
        r.write_tables(&on, bin, true).unwrap();
        let rows = table_rows(&on, "parent_checks");
        let row: Vec<&str> = [
            "t_us",
            "node",
            "stream",
            "action",
            "chosen",
            "best_peer",
            "best_lag_us",
            "runner_up_peer",
            "parent",
            "stale",
            "votes",
        ]
        .iter()
        .map(|c| rows[0][*c].as_str())
        .collect();
        assert_eq!(
            row,
            [
                "310250", "1", "3", "graft", "0", "0", "12000", "", "", "1", "0"
            ]
        );
        let meta: Value =
            serde_json::from_str(&std::fs::read_to_string(on.join("series_meta.json")).unwrap())
                .unwrap();
        assert_eq!(meta["parent_checks_lost"], 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A run whose nodes kept their timers (TIMER_LOG) writes them as the
    /// twin's table has them, the twin keeping the same nodes' (the relay
    /// none): times from the stream's start, each node's context, and the
    /// lost count.
    #[test]
    fn a_runs_timer_logs_are_its_twins_table() {
        use strims_node::event::{TimerRecord, TimerSource};
        use strims_testbed::record::{TimerContext, timers_json};
        let dir = fixture("timers");
        let origin = Time::from_unix_millis(START as i64);
        let at = |us: i64| origin + strims_base::time::Duration::from_micros(us);
        let row = |source, armed, deadline, fired, requested| TimerRecord {
            source,
            armed: at(armed),
            deadline: at(deadline),
            fired: at(fired),
            requested: strims_base::time::Duration::from_micros(requested),
        };
        let rows = [
            (
                0,
                vec![
                    row(TimerSource::NodeTask, 1_200, 4_000, 5_130, 3_000),
                    row(TimerSource::Writer, -400, 0, 1_020, 1_000),
                ],
                TimerContext::Task,
                0,
            ),
            (
                1,
                vec![row(TimerSource::Pacer, 9_000, 14_000, 13_900, 5_000)],
                TimerContext::Worker,
                3,
            ),
        ];
        for (i, rows, context, lost) in &rows {
            let path = dir.join(format!("node-{i}.jsonl"));
            let text = std::fs::read_to_string(&path).unwrap();
            let mut lines: Vec<Value> = text
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect();
            let v: Value =
                serde_json::from_str(&timers_json(rows, *lost, *context, origin)).unwrap();
            lines[0]["rec"]["timers"] = v;
            let out: String = lines.iter().map(|l| format!("{l}\n")).collect();
            std::fs::write(&path, out).unwrap();
        }
        let settings = std::fs::read_to_string(dir.join("settings")).unwrap();
        let bin = Duration::from_millis(BIN_MS.unsigned_abs());
        // Off: no table, though the lines held timers.
        let off = dir.join("tables-off");
        Run::read(&dir, &BTreeMap::new())
            .unwrap()
            .report()
            .write_tables(&off, bin, true)
            .unwrap();
        assert!(crate::tables::read_table(&off, "timers").is_err());
        std::fs::write(dir.join("settings"), format!("{settings}TIMER_LOG=all\n")).unwrap();
        let run = Run::read(&dir, &BTreeMap::new()).unwrap();
        // The fixture's seed and viewer, by number.
        assert_eq!(run.knobs["STRIMS_SIM_TIMER_LOG"], "hosts:0,1");
        assert!(!run.knobs.contains_key("TESTBED_TIMER_LOG"));
        let on = dir.join("tables-on");
        run.report().write_tables(&on, bin, true).unwrap();
        let rows = table_rows(&on, "timers");
        let got: Vec<Vec<&str>> = rows
            .iter()
            .map(|r| {
                [
                    "t_us",
                    "node",
                    "n",
                    "source",
                    "armed_us",
                    "deadline_us",
                    "late_us",
                    "requested_us",
                    "context",
                ]
                .iter()
                .map(|c| r[*c].as_str())
                .collect()
            })
            .collect();
        assert_eq!(
            got,
            [
                [
                    "5130",
                    "0",
                    "0",
                    "node_task",
                    "1200",
                    "4000",
                    "1130",
                    "3000",
                    "task"
                ],
                [
                    "1020", "0", "1", "writer", "-400", "0", "1020", "1000", "task"
                ],
                [
                    "13900", "1", "0", "pacer", "9000", "14000", "-100", "5000", "worker"
                ],
            ]
        );
        let meta: Value =
            serde_json::from_str(&std::fs::read_to_string(on.join("series_meta.json")).unwrap())
                .unwrap();
        assert_eq!(meta["timers_lost"], 3);
        // One node: its number, which no bare number reads as: the seed
        // alone (0), or the viewer alone (1), the twin's other hosts none.
        for (v, want, nodes) in [("0", "hosts:0", ["0"]), ("1", "hosts:1", ["1"])] {
            std::fs::write(dir.join("settings"), format!("{settings}TIMER_LOG={v}\n")).unwrap();
            let run = Run::read(&dir, &BTreeMap::new()).unwrap();
            assert_eq!(run.knobs["STRIMS_SIM_TIMER_LOG"], want);
            let r = run.report();
            let set = r.scenario.timer_log.clone().unwrap();
            assert_eq!(
                (0..r.scenario.hosts.len())
                    .filter(|&h| set.contains(&r.scenario, h))
                    .map(|h| h.to_string())
                    .collect::<Vec<_>>(),
                nodes
            );
        }
        std::fs::write(dir.join("settings"), format!("{settings}TIMER_LOG=1 0\n")).unwrap();
        let run = Run::read(&dir, &BTreeMap::new()).unwrap();
        assert_eq!(run.knobs["STRIMS_SIM_TIMER_LOG"], "hosts:0,1");
        std::fs::write(dir.join("settings"), format!("{settings}TIMER_LOG=seed\n")).unwrap();
        assert!(
            Run::read(&dir, &BTreeMap::new())
                .err()
                .unwrap()
                .contains("TIMER_LOG=seed: all, or node indices of the run's 0 to 1")
        );
        // A node the run lacks, a number written otherwise, and a word.
        for v in ["2", "01", "seed"] {
            std::fs::write(dir.join("settings"), format!("{settings}TIMER_LOG={v}\n")).unwrap();
            let err = Run::read(&dir, &BTreeMap::new()).err().unwrap();
            assert!(
                err.contains(&format!(
                    "TIMER_LOG={v}: all, or node indices of the run's 0 to 1"
                )),
                "{err}"
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_node_that_never_ran_refuses_the_run() {
        let dir = fixture("silent");
        std::fs::write(dir.join("node-1.jsonl"), "").unwrap();
        std::fs::write(
            dir.join("node-1.err"),
            "testbed_node: --join-at: invalid digit found in string\n",
        )
        .unwrap();
        let err = Run::read(&dir, &BTreeMap::new()).err().unwrap();
        assert_eq!(
            err,
            "node 1's harness failed: testbed_node: --join-at: invalid digit found in string"
        );
        std::fs::write(dir.join("node-1.err"), "").unwrap();
        let err = Run::read(&dir, &BTreeMap::new()).err().unwrap();
        assert_eq!(err, "node-1.jsonl has no lines: node-1.err says nothing");
        // A viewer that ran but read nothing is a result; a node from
        // before the records never ran as one.
        let line = |rec: &str| {
            format!(
                "{{\"t_ms\":{},\"rel_ms\":1000,\"transfers\":{{}}{rec}}}\n",
                START + 1000
            )
        };
        std::fs::write(
            dir.join("node-1.jsonl"),
            line(",\"rec\":{\"meter\":null,\"seed\":null,\"bins\":[],\"parents\":[],\"parents_lost\":0,\"reads\":[]}"),
        )
        .unwrap();
        assert!(Run::read(&dir, &BTreeMap::new()).is_ok());
        std::fs::write(dir.join("node-1.jsonl"), line("")).unwrap();
        let err = Run::read(&dir, &BTreeMap::new()).err().unwrap();
        assert_eq!(
            err,
            "node-1.jsonl has no records (`rec`): a node from before them"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A native node's window cuts and timeouts are its TCP counters'
    /// (netstat-<i>.log: recoveries entered and losses a tail-loss probe
    /// repaired, and RTOs), its retransmits its capture's segments that
    /// resent bytes its link had carried; each counted at the bin edge its
    /// sample or packet came before, unmeasured past a log's last sample.
    /// A browser's are unmeasured (data channels), and so are the seed's
    /// cuts and timeouts in a run with competing uploads, whose flows its
    /// counters count too.
    #[test]
    fn a_runs_tcp_counters_and_captures_give_its_network_columns() {
        let dir = fixture("netcols");
        let names =
            "# TcpExt: TCPSackRecovery TCPRenoRecovery TCPLossProbeRecovery TCPTimeouts Other\n";
        let line = |ms: u64, v: [u64; 5]| {
            let v: Vec<String> = v.iter().map(u64::to_string).collect();
            format!("{} TcpExt: {}\n", START + ms, v.join(" "))
        };
        std::fs::write(
            dir.join("netstat-0.log"),
            [
                names.to_owned(),
                line(50, [10, 0, 1, 5, 7]),
                line(150, [12, 0, 1, 6, 7]),
                line(350, [12, 0, 3, 9, 7]),
                format!("{} TcpExt: 12 0", START + 450),
            ]
            .concat(),
        )
        .unwrap();
        std::fs::write(
            dir.join("netstat-1.log"),
            [
                names.to_owned(),
                line(50, [0; 5]),
                line(250, [1, 0, 0, 0, 0]),
                line(950, [1, 0, 0, 0, 0]),
            ]
            .concat(),
        )
        .unwrap();
        let us = |ms: u64| (START + ms) * 1000;
        // Two segments with a hole between them (the one its queue
        // dropped), the hole's resend, then the first again: a resend of
        // carried bytes. The hole's isn't one, though under the highest.
        std::fs::write(
            dir.join("pcap-0.pcap"),
            pcap(&[
                (us(50), tcp_seq(10, 11, 1448, 1000)),
                (us(60), tcp_seq(10, 11, 1448, 3896)),
                (us(150), tcp_seq(10, 11, 1448, 2448)),
                (us(250), tcp_seq(10, 11, 1448, 1000)),
            ]),
        )
        .unwrap();
        let r = Run::read(&dir, &BTreeMap::new()).unwrap().report();
        let at = |k: usize, h: usize, c: &str| r.wire_bins[k][h][up(c)];
        let delta = |h, k, c| r.wire_delta(h, k, up(c));
        for c in ["retransmits", "window_cuts", "rtos"] {
            assert!(!r.wire_unmeasured.contains(&up(c)), "{c}");
        }
        assert_eq!(
            (1..5).map(|k| at(k, 0, "window_cuts")).collect::<Vec<_>>(),
            [0, 2, 2, 4]
        );
        assert_eq!(
            (1..5).map(|k| at(k, 0, "rtos")).collect::<Vec<_>>(),
            [0, 1, 1, 4]
        );
        assert_eq!(delta(0, 3, "rtos"), Some(3));
        // The seed's log stops at 350 ms (the torn line is none): past the
        // edge after it, nothing.
        assert_eq!(delta(0, 4, "rtos"), None);
        assert_eq!(delta(1, 2, "window_cuts"), Some(1));
        assert_eq!(delta(1, 4, "window_cuts"), Some(0));
        assert_eq!(delta(0, 2, "retransmits"), Some(1));
        assert_eq!(delta(0, 1, "retransmits"), Some(0));
        // The tables write them.
        let out = dir.join("tables");
        r.write_tables(&out, Duration::from_millis(100), true)
            .unwrap();
        let seed: Vec<String> = table_rows(&out, "bins")
            .iter()
            .take(5)
            .map(|b| b["seed_rtos"].clone())
            .collect();
        assert_eq!(seed, ["0", "1", "0", "3", ""]);
        let settings = std::fs::read_to_string(dir.join("settings")).unwrap();
        // Competing uploads: the seed's counters count their flows.
        std::fs::write(dir.join("settings"), format!("{settings}COMPETE=5:2\n")).unwrap();
        let r = Run::read(&dir, &BTreeMap::new()).unwrap().report();
        assert_eq!(r.wire_delta(0, 2, up("rtos")), None);
        assert_eq!(r.wire_delta(0, 2, up("retransmits")), Some(1));
        assert_eq!(r.wire_delta(1, 2, up("window_cuts")), Some(1));
        // A browser seed: its links are data channels.
        std::fs::write(
            dir.join("settings"),
            format!("{settings}KINDS=chromium native\nBROWSER_LINKS=webrtc\n"),
        )
        .unwrap();
        let r = Run::read(&dir, &BTreeMap::new()).unwrap().report();
        for c in ["retransmits", "window_cuts", "rtos"] {
            assert_eq!(r.wire_delta(0, 2, up(c)), None, "{c}");
        }
        // Its native viewer's TCP counters stay; its retransmits miss its
        // data channel to the seed's.
        assert_eq!(r.wire_delta(1, 2, up("window_cuts")), Some(1));
        assert_eq!(r.wire_delta(1, 2, up("retransmits")), None);
        // A slow receiver (SLOW_DOWN_KBIT's viewer 1): the seed's resend of
        // carried bytes to it is its drop at that downlink, as the twin
        // counts a full downlink queue's, and no retransmit.
        std::fs::write(
            dir.join("settings"),
            format!("{settings}SLOW_DOWN_KBIT=2000\n"),
        )
        .unwrap();
        let r = Run::read(&dir, &BTreeMap::new()).unwrap().report();
        assert_eq!(r.wire_delta(0, 2, up("retransmits")), Some(0));
        assert_eq!(r.wire_delta(0, 2, up("drops")), Some(1));
        assert_eq!(r.wire_delta(0, 3, up("drops")), Some(0));
        // With the seed's site loss too, its resends to it are that loss's
        // as well: neither column can be told.
        std::fs::write(
            dir.join("settings"),
            format!("{settings}SLOW_DOWN_KBIT=2000\nSEED_LOSS_PCT=1\n"),
        )
        .unwrap();
        let r = Run::read(&dir, &BTreeMap::new()).unwrap().report();
        assert_eq!(r.wire_delta(0, 2, up("retransmits")), None);
        assert_eq!(r.wire_delta(0, 2, up("drops")), None);
        // Without a node's TCP counters, they are unmeasured.
        for i in 0..2 {
            std::fs::remove_file(dir.join(format!("netstat-{i}.log"))).unwrap();
        }
        std::fs::write(dir.join("settings"), &settings).unwrap();
        let r = Run::read(&dir, &BTreeMap::new()).unwrap().report();
        assert!(r.wire_unmeasured.contains(&up("rtos")));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A testbed run's report reads its records as the sim's: each read a
    /// lag from its frame's due time, each meter sample and seed change,
    /// the bins, the parents; and its captures in the sim's wire bytes:
    /// IP lengths, the pure ACK apart, the competing upload's apart, the
    /// tbf's drops and backlog.
    #[test]
    fn a_testbed_runs_records_and_captures_read_as_a_sim_runs() {
        let dir = fixture("records");
        let run = Run::read(&dir, &BTreeMap::new()).unwrap();
        assert_eq!(run.knobs["STRIMS_SIM_CAL_VIEWERS"], "1");
        assert_eq!(run.knobs["TESTBED_QDISC"], "clsact>ifb-netem>root-class");
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
        assert_eq!(last[0][up("drops")], 7);
        assert_eq!(last[1][up("up_bytes")], 0);
        // Its capture's segments resent none; its nodes logged no TCP
        // counters.
        assert_eq!(last[0][up("retransmits")], 0);
        assert!(!r.wire_unmeasured.contains(&up("retransmits")));
        assert!(r.wire_unmeasured.contains(&up("rtos")));
        let w = &r.wan.as_ref().unwrap().hosts[0];
        assert_eq!(w.competing.last().unwrap().1, 1500);
        assert_eq!(w.up_queue[0].1.as_micros(), 1514 * 1_000_000 / 1_250_000);
        // The wait at the tbf's own rate as the sample shows it.
        assert_eq!(w.up_queue[2].1.as_micros(), 1514 * 1_000_000 / 625_000);
        // The drops per bin: the counter's steps, in the bins they fell in.
        let drops: Vec<u64> = r
            .wire_bins
            .iter()
            .map(|b| b[0][up("drops")])
            .collect::<Vec<_>>()
            .windows(2)
            .map(|w| w[1] - w[0])
            .collect();
        assert_eq!(drops.iter().sum::<u64>(), 3);
        assert_eq!(drops.iter().position(|&d| d == 3), Some(4));
        assert_eq!(r.sockets.len(), 1);
        let k = &r.sockets[0];
        assert_eq!((k.node, k.peer), (0, Some(1)));
        assert_eq!(k.rtt_us, Some(99_967));
        assert_eq!(k.min_rtt_us, Some(20_500));
        assert_eq!((k.cwnd_segments, k.retrans_segments), (Some(33), Some(13)));
        assert_eq!(k.delivery_rate_bps, Some(3_728_280 / 8));
        assert_eq!(k.notsent_bytes, Some(1448));
        // The competing upload's socket to the sink, no swarm host's, is left out.
        assert_eq!(w.up_backlog[0].1, 1514);
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
