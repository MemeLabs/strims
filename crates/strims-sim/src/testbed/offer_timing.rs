// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! When a testbed run's browser pages made their first offers, for their
//! twin's stacks (strims-node's `LinkSetup`, calibration_star's
//! `STRIMS_SIM_CAL_FIREFOX_GATHER_MS`, `STRIMS_SIM_CAL_FIREFOX_POST_MS`
//! and `STRIMS_SIM_CAL_CHROMIUM_SET_MS`).
//!
//! A browser node's harness logs to its stderr (node-<i>.err, each line
//! its Unix ms and a JSON object) the node's `connecting` as it starts a
//! link (`atMs`) and each description's `rtcTiming` steps as its page
//! made it (`step` `describe`, `sdpType` `offer`: `asked`, `created`,
//! `set`, `gathered`, Unix ms of the page's `performance.timeOrigin +
//! performance.now()`, src/web/wasm.ts: whole ms on Firefox, which rounds
//! its timers, about 0.1 ms on Chromium). Of a node's
//! first `connecting` and its first offer, this reads, a value a node of
//! its kind in node order (a twin page takes its node's, by its rank
//! among its kind's: two pages of a run part, as the bpace runs' run 1's
//! Firefoxes gathered in 15 and 125 ms):
//!
//! - a Firefox page's gathering end after the connect (`gathered` less
//!   `atMs`), and its offer's post after that end: its first TCP payload
//!   to the signalling relay in its offered capture
//!   (pcap-<i>-offered.pcap) from 1 ms before `gathered` on (its stamp's
//!   whole ms), less `gathered`. That post is its offer's: Firefox offers
//!   at once to every neighbour there before it.
//! - a Chromium page's first setting after the connect (`set` less
//!   `atMs`), its stack's `set_ready`: its gathering then ends on its
//!   metronome's tick, which no measured time stands in for.
//!
//! These move with the host. The farm's h7c8 halve-7-20 runs (8, on
//! c8a's EPYC 9R45) read Firefox's gathering end 12.8 ms after the
//! connect at the median of their run medians (11.0-14.5) and its post
//! 1.7 ms after that (1.1-2.3), Chromium's setting at 10.2 (7.6-11.4);
//! the h7spec halve-7-10 and halve-7-20 runs (16, on c7a's EPYC 9R14,
//! whose packet cost they read) 17.0 (13.0-18.5), 2.6 (1.4-5.1) and 13.9
//! (11.2-15.7); with 16 browsers on one host (halve-15-10) the ends come
//! later still, Firefox's about 22 ms after the connect and Chromium's
//! setting about 22. The packet cost doesn't scale them: 9R14's is 2.3
//! times 9R45's, its Firefox gathering 1.3 times. A node without the log
//! or the capture takes its host's model's ([`by_cpu`]), where its fold
//! knows the model, else its kind's other nodes' median; a run none of
//! whose nodes of a kind gave one takes the model's alone, a value for
//! them all.

use std::path::Path;

use serde_json::Value;

use super::packets;

/// The knob a twin takes its Firefox pages' gathering end from, in ms
/// after the node's connect.
pub const FIREFOX_GATHER: &str = "STRIMS_SIM_CAL_FIREFOX_GATHER_MS";
/// The knob a twin takes its Firefox offers' post from, in ms after their
/// gathering's end.
pub const FIREFOX_POST: &str = "STRIMS_SIM_CAL_FIREFOX_POST_MS";
/// The knob a twin takes its Chromium pages' first setting from, in ms
/// after the node's connect.
pub const CHROMIUM_SET: &str = "STRIMS_SIM_CAL_CHROMIUM_SET_MS";

/// The signalling relay's address octet (run.sh's tb-relay, `$subnet.5`).
const RELAY: u8 = 5;

/// What each CPU model's hosts gave, (a part of its name, Firefox's
/// gathering end, its post after it, Chromium's setting), ms: the farm's
/// runs the module docs name, each its runs' median.
const CPU_TIMES: [(&str, f64, f64, f64); 2] = [
    ("EPYC 9R45", 12.8, 1.7, 10.2),
    ("EPYC 9R14", 17.0, 2.6, 13.9),
];

/// A node's first connect and its first offer's `rtcTiming`, from its
/// harness's stderr `err`.
fn first_offer(err: &str) -> Option<(f64, Value)> {
    let mut connect = None;
    let mut offer = None;
    for line in err.lines() {
        let Some(at) = line.find('{') else { continue };
        let Ok(v) = serde_json::from_str::<Value>(line.get(at..)?) else {
            continue;
        };
        let ty = v.get("type").and_then(Value::as_str);
        if connect.is_none() && ty == Some("connecting") {
            connect = v.get("atMs").and_then(Value::as_f64);
        }
        if offer.is_none()
            && ty == Some("rtcTiming")
            && v.get("step").and_then(Value::as_str) == Some("describe")
            && v.get("sdpType").and_then(Value::as_str) == Some("offer")
        {
            offer = Some(v);
        }
        if connect.is_some() && offer.is_some() {
            break;
        }
    }
    Some((connect?, offer?))
}

/// The first TCP payload node `h` offered the relay from 1 ms before Unix
/// ms `gathered` on, in Unix ms.
fn first_post_ms(dir: &Path, h: usize, gathered: f64) -> Option<f64> {
    let from_us = ((gathered - 1.0) * 1000.0).max(0.0) as u64;
    let me = u8::try_from(10 + h).ok()?;
    let bytes = std::fs::read(dir.join(format!("pcap-{h}-offered.pcap"))).ok()?;
    let us = packets(&bytes)?
        .iter()
        .filter(|p| p.src == Some(me) && p.dst == Some(RELAY))
        .filter(|p| p.tcp.is_some() && p.payload > 0 && p.us >= from_us)
        .map(|p| p.us)
        .min()?;
    Some(us as f64 / 1000.0)
}

fn median(mut xs: Vec<f64>) -> Option<f64> {
    xs.sort_by(f64::total_cmp);
    let mid = xs.len() / 2;
    match xs.len() {
        0 => None,
        n if n % 2 == 1 => xs.get(mid).copied(),
        _ => Some(f64::midpoint(*xs.get(mid - 1)?, *xs.get(mid)?)),
    }
}

/// Out dir `dir`'s run's offer timing (the module docs), its nodes'
/// kinds `kinds` (node index order): each knob with its values, in ms to
/// a tenth, a node of its kind each in node order where some node gave
/// one (one that gave none its host's CPU model's, [`by_cpu`], else the
/// others' median), else the model's alone.
pub fn offer_timing(dir: &Path, kinds: &[&str]) -> Vec<(&'static str, Vec<f64>)> {
    let (mut gather, mut post, mut set) = (Vec::new(), Vec::new(), Vec::new());
    for (h, &kind) in kinds.iter().enumerate() {
        if kind != "firefox" && kind != "chromium" {
            continue;
        }
        let offer = std::fs::read_to_string(dir.join(format!("node-{h}.err")))
            .ok()
            .and_then(|err| first_offer(&err));
        let at = |k: &str| {
            let (connect, offer) = offer.as_ref()?;
            Some((*connect, offer.get(k).and_then(Value::as_f64)?))
        };
        if kind == "firefox" {
            let gathered = at("gathered");
            gather.push(gathered.map(|(c, g)| g - c));
            post.push(gathered.and_then(|(_, g)| Some(first_post_ms(dir, h, g)? - g)));
        } else {
            set.push(at("set").map(|(c, s)| s - c));
        }
    }
    let model = super::thread_cost::cpu_model(dir);
    let fallback = model.as_deref().and_then(by_cpu);
    let has = |k: &str| kinds.contains(&k);
    let mut out = Vec::new();
    let fb = |f: fn((f64, f64, f64)) -> f64| fallback.map(f);
    let tenth = |ms: f64| (ms.max(0.0) * 10.0).round() / 10.0;
    for (knob, xs, kind, fb) in [
        (FIREFOX_GATHER, gather, "firefox", fb(|f| f.0)),
        (FIREFOX_POST, post, "firefox", fb(|f| f.1)),
        (CHROMIUM_SET, set, "chromium", fb(|f| f.2)),
    ] {
        if !has(kind) {
            continue;
        }
        let Some(others) = median(xs.iter().flatten().copied().collect()) else {
            out.extend(fb.map(|ms| (knob, vec![tenth(ms)])));
            continue;
        };
        let each = xs.iter().map(|x| tenth(x.or(fb).unwrap_or(others)));
        out.push((knob, each.collect()));
    }
    out
}

/// The times [`CPU_TIMES`] gives CPU model `model` (Firefox's gathering
/// end, its post after it, Chromium's setting), if it knows it.
pub fn by_cpu(model: &str) -> Option<(f64, f64, f64)> {
    CPU_TIMES
        .iter()
        .find_map(|&(part, g, p, s)| model.contains(part).then_some((g, p, s)))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// A node's first connect and first offer come from its stderr's JSON
    /// lines, past the harness's other lines; a stderr without either gives
    /// none.
    #[test]
    fn the_first_connect_and_offer_come_from_the_harness_log() {
        let err = "browser: up\n\
            1000 {\"atMs\":999,\"started\":true,\"type\":\"connecting\"}\n\
            1001 {\"at\":1001,\"conn\":1,\"step\":\"create\",\"type\":\"rtcTiming\"}\n\
            1012 {\"asked\":1003,\"conn\":1,\"gathered\":1012,\"sdpType\":\"offer\",\"set\":1010,\"step\":\"describe\",\"type\":\"rtcTiming\"}\n\
            1013 {\"atMs\":1013,\"type\":\"connecting\"}\n\
            1020 {\"asked\":1013,\"conn\":2,\"gathered\":1020,\"sdpType\":\"offer\",\"step\":\"describe\",\"type\":\"rtcTiming\"}\n";
        let (connect, offer) = first_offer(err).unwrap();
        assert!((connect - 999.0).abs() < 1e-9);
        assert_eq!(offer.get("conn").and_then(Value::as_u64), Some(1));
        assert!(first_offer("1000 {\"atMs\":999,\"type\":\"connecting\"}\n").is_none());
        assert_eq!(
            by_cpu("AMD EPYC 9R45 96-Core Processor"),
            Some((12.8, 1.7, 10.2))
        );
        assert_eq!(by_cpu("AMD EPYC 7R13 Processor"), None);
        assert_eq!(median(vec![3.0, 1.0, 2.0, 9.0]), Some(2.5));
    }

    /// A run's knobs: a node of a kind with its log gives its own, in
    /// node order; one without (no node-<i>.err) its host's CPU model's
    /// ([`by_cpu`]) or, of a model it doesn't know, its kind's others'
    /// median; a kind none of whose nodes gave one the model's alone, or
    /// no knob; and a kind the run has no node of gives no knob.
    #[test]
    fn a_runs_offer_times_are_its_nodes_else_its_hosts_cpus() {
        let dir = std::env::temp_dir().join(format!("offer-timing-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("cpu"), "AMD EPYC 9R45 96-Core Processor\n").unwrap();
        let chromium = |set: f64| {
            format!(
                "1000 {{\"atMs\":1000,\"type\":\"connecting\"}}\n\
                 1011 {{\"asked\":1001,\"set\":{set},\"sdpType\":\"offer\",\"step\":\"describe\",\"type\":\"rtcTiming\"}}\n"
            )
        };
        std::fs::write(dir.join("node-1.err"), chromium(1011.4)).unwrap();
        std::fs::write(dir.join("node-4.err"), chromium(1020.0)).unwrap();
        // Node 2, a Firefox, and node 3, a Chromium, left no log; node 0 is
        // the native seed.
        let all = ["native", "chromium", "firefox", "chromium", "chromium"];
        let knobs = offer_timing(&dir, &all);
        let want = [
            (FIREFOX_GATHER, vec![12.8]),
            (FIREFOX_POST, vec![1.7]),
            (CHROMIUM_SET, vec![11.4, 10.2, 20.0]),
        ];
        assert_eq!(knobs, want);
        let no_firefox = offer_timing(&dir, &["native", "chromium"]);
        assert_eq!(no_firefox, [(CHROMIUM_SET, vec![11.4])]);
        std::fs::write(dir.join("cpu"), "AMD EPYC 7R13 Processor\n").unwrap();
        assert_eq!(
            offer_timing(&dir, &all),
            [(CHROMIUM_SET, vec![11.4, 15.7, 20.0])]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
