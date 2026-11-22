// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The shaper bench: a testbed run's offered packets replayed through each
//! node's access link in the model ([`strims_wan::replay`]), written as a
//! job's tables, its `packets` and `hosts`, with a `job.json` for
//! `hack/sim-db`'s fold.
//!
//! ```text
//! shaper_replay <out dir> <tables dir> [--offers offered|carried] [--key K] [--node I ...] [--seed S] [--knob KEY=VALUE ...]
//! shaper_replay --bench <out dir> <tables dir> [--node I ...] [--seed S]
//! ```
//!
//! The source run is read as testbed_tables reads it
//! ([`strims_sim::testbed::Run`], its `--knob`s with it), and each node's
//! packets as its `packets` table has them: what the node offered its
//! link, each at its offer, offered to the model's queue at its entry into
//! the testbed's, its egress delay later
//! (`strims_sim::report::WanHostReport::entry_delay`). `--offers offered` (the
//! default) replays them all, the drops' stimulus; `--offers carried`
//! replays those alone that the testbed's link carried, through a queue
//! without a limit, the departures': a packet the model and the tbf
//! disagree on dropping moves every later departure of its busy period by
//! its frame, so the shaper's timing is read where both queues held the
//! same packets. Each node's uplink is its
//! `links` line's spec, rate and queue (its QUEUE_MS at its rate), the
//! run's bucket and peak for it (`SEED_BURST`, `SEED_PEAK_MBIT`; the
//! viewers'), and the seed's steps as `steps.log` applied them. A row keeps
//! its source's identity (node, n, t_us, peer, link, proto, ip_bytes,
//! kind, offsets, resent, estimated) and takes the model's fate: sent_us,
//! lost, charged_bytes and backlog_bytes; arrived_us is empty. Every node
//! with packets is replayed, or the `--node`s; slots draw from `--seed`
//! (1). A link the twins refuse (`strims_sim::link::TestbedLink`: a
//! router's AQM, DOCSIS-PIE, a delay trace, a byte limit) is refused, and
//! a trace link, whose schedule the replay doesn't read, too: the shaper
//! bench has no replay of a trace link yet.
//!
//! `job.json` holds the job's key (`--key`, else `rp-` and its source's
//! digest, `-carried` after it for the carried), its sim (`strims-sim`),
//! its test (`shaper_replay`), its env (the source's knobs, `SOURCE_K` its
//! key, `REPLAY_NODES` the nodes replayed and `REPLAY_OFFERS` what of their
//! packets), and an empty row.
//!
//! `--bench` reads a TCP bench's out dir (`hack/testbed/bench.sh`'s, which
//! has no node records) from its captures alone
//! ([`strims_sim::testbed::bench_captures`]), times from its schedule's
//! start, its buckets and peaks its settings', and writes three `packets`
//! tables: the bench's own in `<tables dir>/source`, the model's of every
//! offer in `<tables dir>/replay` and of the carried in
//! `<tables dir>/carried`, with no job.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_sim::link::{Class, TestbedLink};
use strims_sim::report::{PacketLoss, PacketRecord};
use strims_sim::tables::{write_hosts, write_numbered_packets};
use strims_sim::testbed::{Run, bench_captures, node_link, settings_of};
use strims_wan::Bucket;
use strims_wan::replay::{Offer, Uplink, replay};
use strims_wan::testbed::Spec;

/// The test a replay's job is of.
const TEST: &str = "shaper_replay";

fn main() {
    if let Err(e) = run() {
        eprintln!("shaper_replay: {e}");
        std::process::exit(1);
    }
}

/// Node `h`'s uplink in out dir `dir`, its bucket `burst` bytes sent at
/// `peak` bytes per second (none: at once), its rate from each of `steps`
/// on.
fn uplink_of(
    dir: &Path,
    h: usize,
    burst: u64,
    peak: Option<u64>,
    steps: Vec<(Time, u64)>,
) -> Result<Uplink, String> {
    let link = node_link(dir, h).ok_or(format!("links: no node {h}"))?;
    // A link the model has no form of (a router's AQM, DOCSIS-PIE, a
    // delay trace) has no replay.
    let class = TestbedLink::parse(&link.spec)
        .map_err(|e| format!("node {h}: {e}"))?
        .class();
    if class == Class::Trace {
        return Err(format!(
            "node {h}: {}: the replay reads no trace's schedule",
            link.spec
        ));
    }
    let up = (link.up_mbit * 125_000.0) as u64;
    let top = steps.iter().map(|&(_, r)| r).fold(up, u64::max);
    let queue = Duration::from_micros((link.queue_ms * 1000.0).round() as i64);
    let bucket = Bucket { bytes: burst, peak };
    let mut uplink = Uplink::of(&Spec::parse(&link.spec)?, up, top, Some(bucket), queue)
        .map_err(|e| format!("node {h}: {e}"))?;
    uplink.steps = steps;
    Ok(uplink)
}

/// What of a node's packets a replay offers its uplink (`--offers`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Offers {
    /// Every packet the node offered its link.
    Offered,
    /// Those its link carried: the testbed's drops left out.
    Carried,
}

impl Offers {
    fn name(self) -> &'static str {
        match self {
            Offers::Offered => "offered",
            Offers::Carried => "carried",
        }
    }
}

/// `packets` (a node's, offered at their times since `origin`, entering
/// its queue `entry_delay` later), those `offers` takes, with their fates
/// on `uplink` in place of theirs, each with its number among `packets`
/// (its row's `n`). What the link carried goes through a queue without a
/// limit, so the model's holds what the tbf's held.
fn replayed(
    uplink: &Uplink,
    packets: &[PacketRecord],
    entry_delay: Duration,
    offers: Offers,
    seed: u64,
) -> Vec<(u64, PacketRecord)> {
    let origin = Time::EPOCH;
    let unbounded = Uplink {
        queue: Duration::MAX,
        ..uplink.clone()
    };
    let uplink = if offers == Offers::Carried {
        &unbounded
    } else {
        uplink
    };
    let packets: Vec<(u64, &PacketRecord)> = packets
        .iter()
        .enumerate()
        .filter(|(_, p)| offers == Offers::Offered || p.sent.is_some())
        .map(|(n, p)| (u64::try_from(n).unwrap_or(u64::MAX), p))
        .collect();
    let offers: Vec<Offer> = packets
        .iter()
        .map(|&(_, p)| Offer {
            at: origin + p.at + entry_delay,
            ip: p.ip_bytes as usize,
        })
        .collect();
    let fates = replay(uplink, &offers, &mut det_rng(seed));
    packets
        .iter()
        .zip(fates)
        .map(|(&(n, p), f)| {
            let row = PacketRecord {
                sent: f.sent.map(|t| t - origin),
                arrived: None,
                lost: f.sent.is_none().then_some(PacketLoss::Queue),
                charged_bytes: Some(u32::try_from(f.charged).unwrap_or(u32::MAX)),
                backlog_bytes: Some(f.backlog as u64),
                ..p.clone()
            };
            (n, row)
        })
        .collect()
}

/// A node's `packets`, each with its number (its row's `n`).
fn numbered(packets: &[PacketRecord]) -> Vec<(u64, &PacketRecord)> {
    let n = |i: usize| u64::try_from(i).unwrap_or(u64::MAX);
    packets.iter().enumerate().map(|(i, p)| (n(i), p)).collect()
}

/// Nodes' numbered rows as `write_numbered_packets` takes them.
fn refs(rows: &[(usize, Vec<(u64, PacketRecord)>)]) -> Vec<(usize, Vec<(u64, &PacketRecord)>)> {
    rows.iter()
        .map(|(h, r)| (*h, r.iter().map(|(n, p)| (*n, p)).collect()))
        .collect()
}

/// A bucket's bytes and peak from settings `get` gives, for `role` (`SEED`
/// or `VIEWER`): run.sh's 3028 bytes unset, its peak in Mbit/s.
fn bucket_of(get: impl Fn(&str) -> Option<String>, role: &str) -> (u64, Option<u64>) {
    let bytes = get(&format!("{role}_BURST"))
        .and_then(|b| b.parse().ok())
        .unwrap_or(3028);
    let peak = get(&format!("{role}_PEAK_MBIT"))
        .and_then(|p| p.parse::<f64>().ok())
        .map(|m| (m * 125_000.0) as u64);
    (bytes, peak)
}

fn run() -> Result<(), String> {
    // The command line names the run.
    #[allow(clippy::disallowed_methods)]
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: shaper_replay [--bench] <out dir> <tables dir> [--offers offered|carried] [--key K] [--node I ...] [--seed S] [--knob KEY=VALUE ...]";
    let bench = args.first().is_some_and(|a| a == "--bench");
    if bench {
        args.remove(0);
    }
    let mut args = args.into_iter();
    let dir = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().ok_or(usage)?);
    let (mut key, mut nodes, mut seed, mut extra) = (None, Vec::new(), 1u64, BTreeMap::new());
    let mut offers = Offers::Offered;
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(usage);
        match flag.as_str() {
            "--key" => key = Some(value()?),
            "--offers" => {
                offers = match value()?.as_str() {
                    "offered" => Offers::Offered,
                    "carried" => Offers::Carried,
                    _ => return Err(usage.to_owned()),
                }
            }
            "--node" => nodes.push(
                value()?
                    .parse::<usize>()
                    .map_err(|e| format!("--node: {e}"))?,
            ),
            "--seed" => seed = value()?.parse().map_err(|e| format!("--seed: {e}"))?,
            "--knob" => {
                let kv = value()?;
                let (k, v) = kv.split_once('=').ok_or("--knob KEY=VALUE")?;
                extra.insert(k.to_owned(), v.to_owned());
            }
            _ => return Err(usage.to_owned()),
        }
    }
    let wanted = |h: usize| nodes.is_empty() || nodes.contains(&h);
    if bench {
        let settings = settings_of(
            &std::fs::read_to_string(dir.join("settings")).map_err(|e| format!("settings: {e}"))?,
        );
        let (start_ms, captures) = bench_captures(&dir)?;
        // The seed's steps as applied, from the schedule's start.
        let steps: Vec<(Time, u64)> = std::fs::read_to_string(dir.join("steps.log"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| {
                let mut w = l.split_whitespace();
                let ms: i64 = w.next()?.parse().ok()?;
                let mbit: f64 = w.next()?.parse().ok()?;
                let at = Duration::from_millis(ms - i64::try_from(start_ms).ok()?);
                Some((Time::EPOCH + at, (mbit * 125_000.0) as u64))
            })
            .collect();
        let (mut source, mut model, mut carried) = (Vec::new(), Vec::new(), Vec::new());
        for (h, c) in captures.iter().enumerate() {
            if !wanted(h) || c.packets.is_empty() {
                continue;
            }
            let role = if h == 0 { "SEED" } else { "VIEWER" };
            let (burst, peak) = bucket_of(|k| settings.get(k).cloned(), role);
            let st = if h == 0 { steps.clone() } else { Vec::new() };
            let uplink = uplink_of(&dir, h, burst, peak, st)?;
            source.push((h, numbered(&c.packets)));
            model.push((
                h,
                replayed(
                    &uplink,
                    &c.packets,
                    c.entry_delay,
                    Offers::Offered,
                    seed ^ h as u64,
                ),
            ));
            carried.push((
                h,
                replayed(
                    &uplink,
                    &c.packets,
                    c.entry_delay,
                    Offers::Carried,
                    seed ^ h as u64,
                ),
            ));
        }
        let model = refs(&model);
        let carried = refs(&carried);
        for (name, rows) in [
            ("source", &source),
            ("replay", &model),
            ("carried", &carried),
        ] {
            let d = out.join(name);
            write_numbered_packets(&d, Duration::ZERO, rows)
                .map_err(|e| format!("{}: {e}", d.display()))?;
        }
        println!("{}", out.display());
        return Ok(());
    }
    let run = Run::read(&dir, &extra)?;
    let key = key.unwrap_or_else(|| {
        let digest = run.key().trim_start_matches("tb-").to_owned();
        match offers {
            Offers::Offered => format!("rp-{digest}"),
            Offers::Carried => format!("rp-{digest}-carried"),
        }
    });
    let report = run.report();
    let wan = report.wan.as_ref().ok_or("the run has no network")?;
    let start = report
        .scenario
        .swarms
        .first()
        .map_or(Duration::ZERO, |s| Duration::from(s.start));
    let mut done: Vec<(usize, Vec<(u64, PacketRecord)>)> = Vec::new();
    for (h, spec) in report.scenario.hosts.iter().enumerate() {
        if !wanted(h) {
            continue;
        }
        let Some(w) = wan.hosts.iter().find(|w| w.name == spec.name) else {
            continue;
        };
        if w.packets.is_empty() {
            continue;
        }
        let steps: Vec<(Time, u64)> = spec
            .up_steps
            .iter()
            .map(|&(at, rate)| (Time::EPOCH + Duration::from(at), rate))
            .collect();
        let role = if h == 0 { "SEED" } else { "VIEWER" };
        let (burst, peak) = bucket_of(
            |k| run.knobs.get(&format!("STRIMS_SIM_CAL_{k}")).cloned(),
            role,
        );
        let uplink = uplink_of(&run.dir, h, burst, peak, steps)?;
        done.push((
            h,
            replayed(&uplink, &w.packets, w.entry_delay, offers, seed ^ h as u64),
        ));
    }
    write_numbered_packets(&out, start, &refs(&done))
        .map_err(|e| format!("{}: {e}", out.display()))?;
    let rows = report.host_rows();
    let hosts: Vec<(usize, &str, &strims_sim::report::HostRow)> = done
        .iter()
        .filter_map(|(h, _)| {
            let name = report.scenario.hosts.get(*h)?.name.as_str();
            Some((*h, name, rows.get(*h)?))
        })
        .collect();
    write_hosts(&out, &hosts).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut env: serde_json::Map<String, serde_json::Value> = run
        .knobs
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
        .collect();
    env.insert("SOURCE_K".to_owned(), run.key().into());
    let nodes: Vec<String> = done.iter().map(|(h, _)| h.to_string()).collect();
    env.insert("REPLAY_NODES".to_owned(), nodes.join(",").into());
    env.insert("REPLAY_OFFERS".to_owned(), offers.name().into());
    let job = serde_json::json!({
        "k": key,
        "sim": "strims-sim",
        "test": TEST,
        "env": env,
        "row": {},
    });
    std::fs::write(out.join("job.json"), format!("{job}\n"))
        .map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{}", out.display());
    Ok(())
}
