// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The upload meter bench: no ppspp machinery, transports and links only.
//!
//! One node under test ([`node`]'s node 0) writes traffic shaped like
//! ppspp's to 1-30 neighbours whose links share its access uplink: as a
//! seed, each signed unit as a burst of chunk frames to each push child at
//! its signing time; as a relay, each frame its feeder pushes it forwarded
//! on arrival; and STATEs, HAVEs (with main's mix), requests, NOT_YETs,
//! pings and answers to its neighbours' pulls. Its meters ([`meters`]) read
//! only what every transport offers ([`meters::Obs`]): the bytes it queued
//! and the link took, the transport's unsent bytes, probes and their
//! echoes, and the neighbours' reports. The first meter drives the copies
//! (children) through main's budget rule ([`meters::Budget`]), the rest
//! read the same observations alongside, and with `record` every
//! observation is logged for replays.
//!
//! A run's condition ([`plan::Condition`]) is drawn from its class and seed,
//! and so is its traffic ([`plan::Traffic`]), so one seed is the same run
//! for every transport and meter. [`simulate`] runs it on the sim's sockets
//! under the WAN model's window model: TCP, or its data channels sent as
//! Chromium's, Firefox's or str0m's SCTP sends.
//!
//! # Logs
//!
//! Each node logs JSON lines, times in µs from the bench's start:
//!
//! - node 0: `{"ev":"m","t","meter","cap",..}` each meter's reading every
//!   [`node::TICK`] (`cap` bytes per second, then its diagnostics);
//!   `{"ev":"k","t","copies","budget","children","rate"}` at each budget
//!   check; `{"ev":"b","t","push","answer","control","probe","pad"}` the
//!   bytes its links took each second by class; `{"ev":"o",..}` each
//!   observation with `record`;
//! - neighbours: `{"ev":"u","unit","at","first","last","bytes"}` a pushed
//!   unit complete (`at` its signing), `{"ev":"a","unit","frame","at","got"}`
//!   an answer;
//! - [`simulate`] adds to node 0's the truth: `{"ev":"q","t","wait"}` its
//!   uplink queue (µs) every 7 ms, `{"ev":"x","t","bytes","cwnd"}` the
//!   competing uploads' wire bytes, and a `{"ev":"drops",..}` line.

// The bench's nodes share their state between their tasks on one thread.
#![allow(clippy::disallowed_types)]

pub mod candidates;
pub mod meters;
pub mod node;
pub mod plan;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use strims_base::time::Duration;
use strims_wan::access::Bursts;
use strims_wan::{Access, City, Options, Site};

use crate::bench::{BenchClock, LINGER};
use crate::net::{SendBuffers, SimNet};
use crate::spec::{Competing, Scenario, SendBuffer};
use crate::wan::{Scheduler, SharedWan, sim_time};

use node::{NodeSetup, bench_links, bench_nodes};
use plan::{Condition, Traffic};

/// When the bench starts in the sim, after its links connect.
const SIM_START: StdDuration = StdDuration::from_secs(2);
/// How often the sim samples the node's uplink queue.
const QUEUE_EVERY: StdDuration = StdDuration::from_millis(7);
/// A relay's feeder's uplink.
const FEEDER_UP: u64 = 100 * 125_000;

#[derive(Clone, Copy, Debug)]
struct Clock;

impl BenchClock for Clock {
    fn now_us(&self) -> i64 {
        let t = turmoil::sim_elapsed().unwrap_or_default();
        i64::try_from(t.as_micros()).unwrap_or(i64::MAX)
            - i64::try_from(SIM_START.as_micros()).unwrap_or(0)
    }

    fn time(&self) -> strims_base::time::Time {
        sim_time(turmoil::sim_elapsed().unwrap_or_default())
    }
}

fn std(d: Duration) -> StdDuration {
    StdDuration::from_micros(d.as_micros().max(0).unsigned_abs())
}

/// A mid-run change to the WAN.
enum Event {
    Latency(usize, Duration),
    Freeze(usize),
    Thaw(usize),
}

/// Runs `cond` with `meters` (the driving one first) on the sim. Returns
/// each node's log lines.
pub fn simulate(
    cond: &Condition,
    meters: &[String],
    record: bool,
) -> Result<Vec<Vec<String>>, String> {
    let traffic = Rc::new(Traffic::new(cond));
    let cond = Rc::new(cond.clone());
    let nodes = bench_nodes(&cond);
    let n = cond.neighbours.len();
    let mut spec = Scenario::new("meter-bench");
    let quiet = |site: Site| {
        let mut s = site;
        s.profile.jitter = Duration::ZERO;
        s.profile.loss = 0.0;
        s.profile.bursts = None;
        s
    };
    for i in 0..nodes {
        let site = if i == 0 {
            let mut s = quiet(Site::new(City::Chicago, Access::Cable).with_up(cond.up));
            s.profile.loss = cond.loss;
            s.profile.jitter = cond.jitter;
            s.profile.bursts = cond.bursts.map(|b| Bursts {
                every: b.every,
                phase: b.phase,
                length: b.length,
                loss: b.loss,
            });
            s
        } else if i <= n {
            let mut s = quiet(Site::new(City::Chicago, Access::Fiber));
            if let Some(d) = cond.neighbours.get(i - 1).and_then(|nb| nb.down) {
                s = s.with_down(d);
            }
            s
        } else {
            quiet(Site::new(City::Chicago, Access::Datacenter).with_up(FEEDER_UP))
        };
        spec.host_at(&format!("n{i}"), site);
    }
    if let Some(h) = spec.hosts.first_mut() {
        h.up_steps = cond
            .up_steps
            .iter()
            .map(|&(at, r)| (SIM_START + std(at), r))
            .collect();
        h.competing = cond
            .compete
            .iter()
            .map(|&(at, len)| Competing {
                start: SIM_START + std(at),
                stop: SIM_START + std(at + len),
                rtt: std(cond.compete_rtt),
            })
            .collect();
    }
    let window = SendBuffer {
        uplink_queue: std(cond.queue),
        window: true,
    };
    spec.send_buffer = Some(window);
    let options = Options {
        jitter: cond.jitter > Duration::ZERO,
        loss: cond.loss > 0.0 || cond.bursts.is_some(),
    };
    spec.wan = Some(options);
    let segment = spec.segment();
    let end_us = i64::try_from(cond.secs).unwrap_or(0) * 1_000_000;
    strims_node::seed_str0m_thread_rng(cond.seed);
    let mut builder = turmoil::Builder::new();
    builder
        .rng_seed(cond.seed)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_millis(crate::EPOCH_MS.unsigned_abs()))
        .simulation_duration(SIM_START + StdDuration::from_secs(cond.secs) + LINGER * 2)
        .tick_duration(spec.tick);
    if let Some(seg) = segment {
        builder.tcp_capacity(64 * (64 * 1024usize).div_ceil(seg.max(1)));
    }
    if let Some(p) = cond.transport.profile() {
        builder.tcp_capacity(64 * (64 * 1024usize).div_ceil(p.max_payload));
    }
    let mut sim = builder.build();
    let buffers = SendBuffers::new(window);
    let wan = SharedWan::default();
    let logs: Rc<RefCell<Vec<Vec<String>>>> = Rc::new(RefCell::new(vec![Vec::new(); nodes]));
    for me in 0..nodes {
        let net = SimNet::new(Some(buffers.clone()))
            .with_wan(wan.clone())
            .with_segment(segment);
        let setup = NodeSetup {
            me,
            listen: "0.0.0.0:9000".to_owned(),
            addr: Rc::new(|i| format!("n{i}:9000")),
            cond: cond.clone(),
            traffic: traffic.clone(),
            meters: meters.to_vec(),
            record,
            end_us,
        };
        let logs = logs.clone();
        let log: Rc<dyn Fn(String)> = Rc::new(move |line: String| {
            if let Some(l) = logs.borrow_mut().get_mut(me) {
                l.push(line);
            }
        });
        sim.client(format!("n{me}").as_str(), async move {
            node::node(net, Clock, setup, log).await?;
            Ok(())
        });
    }
    let mut sched = Scheduler::new(&sim, &spec, options, cond.seed, Some(buffers))?;
    sched.sample_queues_every(Duration::from(QUEUE_EVERY));
    for (a, b) in bench_links(&cond) {
        let d = if b <= n {
            cond.neighbours
                .get(b - 1)
                .map_or(Duration::ZERO, |nb| nb.delay)
        } else {
            cond.feeder_delay
        };
        sched.set_latency(a, b, d);
    }
    *wan.lock() = Some(sched);
    let mut events: Vec<(StdDuration, Event)> = Vec::new();
    for s in &cond.latency_steps {
        let at = SIM_START + std(s.at);
        for (&l, &d) in s.links.iter().zip(&s.delays) {
            if s.freeze > Duration::ZERO {
                events.push((at, Event::Freeze(l)));
                events.push((at + std(s.freeze), Event::Latency(l, d)));
                events.push((at + std(s.freeze), Event::Thaw(l)));
            } else {
                events.push((at, Event::Latency(l, d)));
            }
        }
    }
    events.sort_by_key(|&(t, _)| t);
    let mut next = 0usize;
    loop {
        let elapsed = sim.elapsed();
        while let Some((t, e)) = events.get(next) {
            if *t > elapsed {
                break;
            }
            if let Some(s) = wan.lock().as_mut() {
                match *e {
                    Event::Latency(l, d) => s.set_latency(0, l + 1, d),
                    Event::Freeze(l) => s.freeze(0, l + 1),
                    Event::Thaw(l) => s.thaw(0, l + 1),
                }
            }
            next += 1;
        }
        if let Some(s) = wan.lock().as_mut() {
            s.walk(&sim)?;
        }
        match sim.step() {
            Ok(true) => break,
            Ok(false) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    drop(sim);
    let report = wan.lock().take().map(|s| s.report(&spec));
    let mut logs = logs.take();
    let start = Duration::from(SIM_START);
    if let (Some(h), Some(l)) = (
        report.as_ref().and_then(|r| r.hosts.first()),
        logs.first_mut(),
    ) {
        l.push(format!(
            r#"{{"ev":"drops","up_packets":{},"queue_drops":{},"competing_drops":{},"acks":{}}}"#,
            h.stats.up_bytes / 1500,
            h.queue_drops,
            h.competing_drops,
            h.acks,
        ));
        for &(t, bytes) in &h.competing {
            let cwnd = h
                .competing_cwnd
                .iter()
                .find(|&&(at, _)| at == t)
                .map_or(0, |&(_, w)| w);
            l.push(format!(
                r#"{{"ev":"x","t":{},"bytes":{bytes},"cwnd":{cwnd}}}"#,
                (t - start).as_micros()
            ));
        }
        for &(t, wait) in &h.up_queue {
            l.push(format!(
                r#"{{"ev":"q","t":{},"wait":{}}}"#,
                (t - start).as_micros(),
                wait.as_micros()
            ));
        }
    }
    Ok(logs)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::plan::{Class, Transport};
    use super::*;

    /// `key`'s number in a log line.
    fn field(line: &str, key: &str) -> Option<f64> {
        let at = line.find(&format!("\"{key}\":"))? + key.len() + 3;
        let rest = line.get(at..)?;
        let end = rest
            .find(|c: char| c != '-' && c != '.' && !c.is_ascii_digit())
            .unwrap_or(rest.len());
        rest.get(..end)?.parse().ok()
    }

    #[test]
    fn a_seed_pushes_its_units_and_its_trains_read_its_uplink() {
        let mut c = Condition::draw(Class::Steady, 2, Transport::Tcp, false);
        c.secs = 20;
        let meters = ["oracle".to_owned(), "train".to_owned()];
        let logs = simulate(&c, &meters, false).unwrap();
        let units = logs
            .iter()
            .skip(1)
            .flatten()
            .filter(|l| l.contains(r#""ev":"u""#))
            .count();
        assert!(units > 100, "{units}");
        let reads: Vec<f64> = logs[0]
            .iter()
            .filter(|l| l.contains(r#""meter":"train""#) && field(l, "t").is_some_and(|t| t > 1e7))
            .filter_map(|l| field(l, "cap"))
            .collect();
        assert!(!reads.is_empty());
        let mid = reads[reads.len() / 2] / c.up as f64;
        assert!((0.8..1.1).contains(&mid), "{mid}");
        // The same run again is the same run.
        assert_eq!(simulate(&c, &meters, false).unwrap(), logs);
    }
}
