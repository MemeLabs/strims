// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What each packet a testbed run's Chromium nodes sent took of their
//! network thread ([`SctpProfile::packet_cost`]), for their twin
//! (`STRIMS_SIM_CAL_PACKET_COST_US`) and the bench sim.
//!
//! A Chromium node's send opportunity leaves its thread a packet at a time,
//! so its packets go out in trains, each the thread's cost after the one
//! before. That cost is its host's: set by its CPU model, it barely moves
//! with the host's cores, its load, the node's peers or the packets' sizes
//! (the farm's Chromium seeds: EPYC 9R45's 5.4 µs, 9R14's 12.3, 7R13's
//! 18.1, the dev VM's Threadripper PRO 5955WX 15.4; within a run ±1 µs and
//! between runs of one type ±0.5-1 µs). Every node of a run shares its
//! host, so a run has one cost:
//!
//! - **From a capture.** The first Chromium node, in node order, whose
//!   offered capture (`pcap-<i>-offered.pcap`, ahead of its link's queue)
//!   holds [`MIN_TRAINS`] trains: its UDP datagrams from [`FROM_US`] into
//!   the stream on, cut into trains at gaps of [`TRAIN_GAP_US`] or more, each
//!   train of [`TRAIN_MIN`] packets or more giving its span over its gaps,
//!   the cost the median of those. A train's first gap runs about half as
//!   long again as the rest, and short trains are mostly the first gap and
//!   a stray pause, so the long trains alone read the thread.
//! - **From the host's CPU.** Without one, or in a run whose dcSCTP logged
//!   its packets (TRACE_SCTP, whose observer formats each packet on the
//!   network thread, so its trains read the observer's cost), the CPU model
//!   run.sh recorded ([`cpu_model`]) where [`CPU_COSTS`] knows it.
//! - Else none, and the twin keeps its profile's.
//!
//! [`SctpProfile::packet_cost`]: crate::wan::SctpProfile::packet_cost

use std::path::Path;

use super::packets;

/// A gap this long or more ends a train (µs): the thread's cost is 3-20 µs
/// on every host measured, the gaps between send opportunities 0.1 ms and
/// more.
pub const TRAIN_GAP_US: u64 = 50;
/// The fewest packets of a train the cost is read from.
pub const TRAIN_MIN: usize = 17;
/// The fewest such trains a node's reading takes.
pub const MIN_TRAINS: usize = 5;
/// How far into the stream trains are read from (µs): past the first
/// viewers' joins, the seed's sends at their steady pace.
pub const FROM_US: u64 = 10_000_000;

/// The cost of a host's CPU model, by a part of its `/proc/cpuinfo` model
/// name (µs): the farm's Chromium seeds' and the dev VM's, measured by
/// [`spacing_us`] over their runs' captures.
pub const CPU_COSTS: &[(&str, f64)] = &[
    ("EPYC 9R45", 5.4),
    ("EPYC 9R14", 12.3),
    ("EPYC 7R13", 18.1),
    ("Threadripper PRO 5955WX", 15.4),
];

/// The median spacing of the trains in `us` (a node's packets' times in
/// µs, in order), in µs: each train of [`TRAIN_MIN`] packets or more, cut
/// at gaps of [`TRAIN_GAP_US`] or more, its span over its gaps; `None`
/// with fewer than [`MIN_TRAINS`] such trains.
pub fn spacing_us(us: &[u64]) -> Option<f64> {
    let mut spans: Vec<f64> = us
        .chunk_by(|a, b| b.saturating_sub(*a) < TRAIN_GAP_US)
        .filter(|train| train.len() >= TRAIN_MIN)
        .filter_map(|train| {
            let span = train.last()?.saturating_sub(*train.first()?);
            Some(span as f64 / (train.len() - 1) as f64)
        })
        .collect();
    if spans.len() < MIN_TRAINS {
        return None;
    }
    spans.sort_by(f64::total_cmp);
    let mid = spans.len() / 2;
    let upper = *spans.get(mid)?;
    Some(if spans.len() % 2 == 1 {
        upper
    } else {
        f64::midpoint(*spans.get(mid - 1)?, upper)
    })
}

/// The cost [`CPU_COSTS`] gives CPU model `model`, if it knows it.
pub fn cpu_cost_us(model: &str) -> Option<f64> {
    CPU_COSTS
        .iter()
        .find_map(|&(part, us)| model.contains(part).then_some(us))
}

/// The CPU model run.sh recorded for out dir `dir`'s host (its `cpu`:
/// `/proc/cpuinfo`'s model name), if it did.
pub fn cpu_model(dir: &Path) -> Option<String> {
    let model = std::fs::read_to_string(dir.join("cpu")).ok()?;
    Some(model.trim().to_owned()).filter(|m| !m.is_empty())
}

/// The packet cost of out dir `dir`'s run, whose stream started at Unix ms
/// `start_ms` and whose Chromium nodes are `chromium` (node indices), in
/// µs to a tenth (see the module docs): from their captures unless
/// `traced` (its dcSCTP logged its packets, TRACE_SCTP), else from the
/// host's CPU; `None` without Chromium nodes, or either.
pub fn packet_cost_us(dir: &Path, chromium: &[usize], start_ms: u64, traced: bool) -> Option<f64> {
    if chromium.is_empty() {
        return None;
    }
    let from = start_ms.saturating_mul(1000).saturating_add(FROM_US);
    let captured = if traced { &[][..] } else { chromium };
    let measured = captured.iter().find_map(|&h| {
        let me = u8::try_from(10 + h).ok()?;
        let bytes = std::fs::read(dir.join(format!("pcap-{h}-offered.pcap"))).ok()?;
        let packets = packets(&bytes)?;
        let mut us: Vec<u64> = packets
            .iter()
            .filter(|p| p.src == Some(me) && p.udp.is_some() && p.us >= from)
            .map(|p| p.us)
            .collect();
        us.sort_unstable();
        spacing_us(&us)
    });
    let us = measured.or_else(|| cpu_cost_us(&cpu_model(dir)?))?;
    Some((us * 10.0).round() / 10.0)
}

/// A bench run's packet cost ([`packet_cost_us`]), its Chromium nodes
/// `chromium` and its stream's start its schedule's (node 0's line
/// `{"ev": "start", "ms": ...}`); a bench's dcSCTP logs no packets.
pub fn bench_packet_cost_us(dir: &Path, chromium: &[usize]) -> Option<f64> {
    let start = std::fs::read_to_string(dir.join("node-0.jsonl"))
        .ok()?
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .find(|v| v.get("ev").and_then(serde_json::Value::as_str) == Some("start"))
        .and_then(|v| v.get("ms")?.as_u64())?;
    packet_cost_us(dir, chromium, start, false)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    /// Times of `trains` trains of `n` packets `gap` µs apart, each train
    /// `every` µs after the one before, from `t0`.
    fn trains(t0: u64, trains: u64, n: u64, gap: u64, every: u64) -> Vec<u64> {
        (0..trains)
            .flat_map(|k| (0..n).map(move |i| t0 + k * every + i * gap))
            .collect()
    }

    /// A libpcap file of UDP datagrams `(Unix µs, from, to)` (last
    /// octets), each of 1191 bytes, in Ethernet frames.
    fn pcap(datagrams: &[(u64, u8, u8)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0xa1b2_c3d4u32.to_le_bytes());
        out.extend_from_slice(&[2, 0, 4, 0]);
        out.extend_from_slice(&[0; 8]);
        out.extend_from_slice(&65535u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        for &(us, src, dst) in datagrams {
            let len = 20 + 8 + 1191;
            let mut ip = vec![0u8; len];
            ip[0] = 0x45;
            ip[2..4].copy_from_slice(&(len as u16).to_be_bytes());
            ip[9] = 17;
            ip[12..16].copy_from_slice(&[10, 77, 0, src]);
            ip[16..20].copy_from_slice(&[10, 77, 0, dst]);
            ip[20..22].copy_from_slice(&4000u16.to_be_bytes());
            ip[22..24].copy_from_slice(&4000u16.to_be_bytes());
            let mut frame = vec![0u8; 12];
            frame.extend_from_slice(&[0x08, 0x00]);
            frame.extend_from_slice(&ip);
            let n = frame.len() as u64;
            for w in [us / 1_000_000, us % 1_000_000, n, n] {
                out.extend_from_slice(&(w as u32).to_le_bytes());
            }
            out.extend_from_slice(&frame);
        }
        out
    }

    /// A run's cost is its first Chromium node's with trains enough in its
    /// offered capture: its own datagrams from FROM_US into the stream,
    /// the rest (its peers' datagrams, its first seconds) left out; without
    /// one, its host's CPU's (`cpu`); without either, or Chromium nodes,
    /// none.
    #[test]
    fn a_runs_cost_is_its_chromium_nodes_trains_else_its_cpus() {
        let dir = std::env::temp_dir().join(format!("strims-thread-cost-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let start_ms = 1_700_000_000_000u64;
        let at = |s: u64| start_ms * 1000 + s * 1_000_000;
        // Node 1 (.11) sends trains of 40 at 5.4 µs from 10 s on, and at
        // 30 µs before; a peer's datagrams in its capture, 1 µs apart,
        // aren't its own. Node 0 (.10) has too few trains.
        let mut one: Vec<(u64, u8, u8)> = Vec::new();
        for k in 0..20u64 {
            let (t0, gap) = if k < 10 { (at(2), 30.0) } else { (at(10), 5.4) };
            for i in 0..40u64 {
                one.push((t0 + k * 2000 + (i as f64 * gap) as u64, 11, 12));
                one.push((t0 + k * 2000 + 1000 + i, 12, 11));
            }
        }
        one.sort_unstable();
        let zero: Vec<(u64, u8, u8)> = (0..40).map(|i| (at(11) + i * 12, 10, 11)).collect();
        std::fs::write(dir.join("pcap-0-offered.pcap"), pcap(&zero)).unwrap();
        std::fs::write(dir.join("pcap-1-offered.pcap"), pcap(&one)).unwrap();
        // Each train's times truncated to the µs: 5.4 µs at the tenth.
        assert_eq!(packet_cost_us(&dir, &[0, 1], start_ms, false), Some(5.4));
        assert_eq!(packet_cost_us(&dir, &[], start_ms, false), None);
        // Node 0 alone: no reading, nor a CPU's.
        assert_eq!(packet_cost_us(&dir, &[0], start_ms, false), None);
        std::fs::write(dir.join("cpu"), "AMD EPYC 7R13 Processor\n").unwrap();
        assert_eq!(packet_cost_us(&dir, &[0], start_ms, false), Some(18.1));
        assert_eq!(packet_cost_us(&dir, &[1], start_ms, false), Some(5.4));
        // A TRACE_SCTP run's trains are its observer's: its CPU's.
        assert_eq!(packet_cost_us(&dir, &[1], start_ms, true), Some(18.1));
        std::fs::write(dir.join("cpu"), "Intel(R) Xeon(R)\n").unwrap();
        assert_eq!(packet_cost_us(&dir, &[0], start_ms, false), None);
        assert_eq!(packet_cost_us(&dir, &[1], start_ms, true), None);
        // A bench run's stream starts at its schedule's.
        let line = serde_json::json!({"ev": "start", "ms": start_ms});
        std::fs::write(dir.join("node-0.jsonl"), format!("{line}\n")).unwrap();
        assert_eq!(bench_packet_cost_us(&dir, &[1]), Some(5.4));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Trains of a constant spacing read as it; trains shorter than
    /// TRAIN_MIN and stray packets don't count; too few trains read none.
    #[test]
    fn a_runs_spacing_is_its_long_trains_median() {
        assert_eq!(spacing_us(&trains(0, 9, 40, 12, 1000)), Some(12.0));
        // Short trains of a wider spacing between the long ones leave the
        // long trains' reading.
        let mut us = trains(0, 9, 30, 6, 1000);
        us.extend(trains(500, 9, 10, 30, 1000));
        us.sort_unstable();
        assert_eq!(spacing_us(&us), Some(6.0));
        // A train's first gap half as long again: its span over its gaps.
        let skewed: Vec<u64> = (0..7)
            .flat_map(|k| {
                let t = k * 2000;
                std::iter::once(t).chain((0..17).map(move |i| t + 18 + i * 12))
            })
            .collect();
        let got = spacing_us(&skewed).unwrap();
        assert!((got - (18.0 + 16.0 * 12.0) / 17.0).abs() < 1e-9, "{got}");
        // The median of differing trains: odd and even counts.
        let mut mixed = trains(0, 3, 20, 5, 1000);
        mixed.extend(trains(3000, 2, 20, 9, 1000));
        assert_eq!(spacing_us(&mixed), Some(5.0));
        mixed.extend(trains(5000, 1, 20, 9, 1000));
        assert_eq!(spacing_us(&mixed), Some(7.0));
        // A gap of TRAIN_GAP_US cuts a train; one of 16 packets is too
        // short; four trains are too few.
        assert_eq!(spacing_us(&trains(0, 9, 16, 12, 1000)), None);
        assert_eq!(spacing_us(&trains(0, 9, 40, TRAIN_GAP_US, 100_000)), None);
        assert_eq!(spacing_us(&trains(0, 4, 40, 12, 1000)), None);
        assert_eq!(spacing_us(&[]), None);
    }

    #[test]
    fn a_cpu_model_reads_its_hosts_cost() {
        assert_eq!(cpu_cost_us("AMD EPYC 9R45"), Some(5.4));
        assert_eq!(cpu_cost_us("AMD EPYC 9R14"), Some(12.3));
        assert_eq!(cpu_cost_us("AMD EPYC 7R13 Processor"), Some(18.1));
        assert_eq!(
            cpu_cost_us("AMD Ryzen Threadripper PRO 5955WX 16-Cores"),
            Some(15.4)
        );
        assert_eq!(cpu_cost_us("Intel(R) Xeon(R) Platinum 8488C"), None);
    }
}
