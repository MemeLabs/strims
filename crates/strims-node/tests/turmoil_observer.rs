// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Sampling a node must not change what it does.
//!
//! A seed and four viewers share a 1 MiB stream on a mesh under turmoil.
//! Every uplink is capped well below the demand on it, so QoS always has a
//! backlog: that is where any change in when the node task runs changes
//! which session QoS serves next and how frames are packed.
//!
//! Each seed runs three times:
//!
//! - quiet: nothing calls the nodes but the scenario;
//! - sampled: every host calls `Node::metrics` and `Node::peers` every
//!   100 ms;
//! - poked: every host makes a no-op call that isn't a query
//!   (`close_peer` of an unknown host) every 100 ms.
//!
//! The sampled trace (events, read progress with simulated times, stream
//! digests, and every host's final metrics apart from the query counter)
//! must equal the quiet one, and no sample may have had to wake a node.
//! The poked trace must differ: the scenario is sensitive to an extra
//! wakeup, so the first check has teeth.

#![cfg(feature = "turmoil")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use sha2::{Digest, Sha256};
use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_crypto::key_from_seed;
use strims_node::{
    Clock, DeliveryMode, HostId, Node, NodeConfig, NodeEvent, NodeEvents, NodeMetrics, SimClock,
    SwarmId, SwarmKey, SwarmOptions, TurmoilNet, WriterOptions,
};
use strims_proto::types::Key;

const TCP_PORT: u16 = 7000;
const WS_PORT: u16 = 7001;
/// Every host's upload cap. The seed alone would need 4 × the stream
/// rate; viewers relay, but every uplink stays backlogged.
const UPLOAD_CAP: u64 = 192 * 1024;
const STREAM_LEN: usize = 1024 * 1024;
/// 32 KiB every 100 ms: 320 KiB/s.
const WRITE: usize = 32 * 1024;
const WRITE_EVERY: StdDuration = StdDuration::from_millis(100);
const SAMPLE_EVERY: StdDuration = StdDuration::from_millis(100);
const VIEWERS: u8 = 4;
/// Who dials whom (`(dialer, listener, ws)`), host 0 being the seed.
const LINKS: [(u8, u8, bool); 6] = [
    (1, 0, false),
    (2, 0, true),
    (3, 1, false),
    (3, 2, true),
    (4, 2, false),
    (4, 1, true),
];

fn epoch() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Observer {
    /// Nothing calls the nodes but the scenario.
    None,
    /// `metrics` and `peers` every 100 ms on every host.
    Sample,
    /// A no-op non-query call every 100 ms on every host.
    Poke,
}

type Trace = Rc<RefCell<Vec<String>>>;

#[derive(Clone)]
struct World {
    seed: u64,
    observer: Observer,
    trace: Trace,
    ids: Rc<RefCell<BTreeMap<u8, HostId>>>,
    nodes: Rc<RefCell<BTreeMap<u8, Node>>>,
    done: Rc<RefCell<u8>>,
    samples: Rc<RefCell<u64>>,
}

impl World {
    fn profile_key(&self, i: u8) -> Key {
        let mut s = [i; 32];
        s[..8].copy_from_slice(&self.seed.to_le_bytes());
        key_from_seed(&s)
    }

    fn swarm_key(&self) -> Key {
        let mut s = [0x55; 32];
        s[..8].copy_from_slice(&self.seed.to_le_bytes());
        key_from_seed(&s)
    }

    fn stream(&self) -> Vec<u8> {
        let mut rng = det_rng(self.seed ^ 0x5eed);
        let mut b = vec![0u8; STREAM_LEN];
        rand_core::Rng::fill_bytes(&mut rng, &mut b);
        b
    }

    fn uri(&self, i: u8, ws: bool) -> String {
        let ip = turmoil::lookup(format!("n{i}"));
        let (scheme, port) = if ws {
            ("ws", WS_PORT)
        } else {
            ("tcp", TCP_PORT)
        };
        format!(
            "{scheme}://{ip}:{port}/{}",
            hex::encode(&self.profile_key(i).public)
        )
    }

    fn log(&self, clock: &SimClock, i: u8, line: String) {
        let t = (clock.now() - epoch()).as_millis();
        self.trace.borrow_mut().push(format!("{t} n{i} {line}"));
    }
}

fn swarm_options() -> SwarmOptions {
    SwarmOptions {
        live_window: 1 << 11,
        delivery_mode: DeliveryMode::MANDATORY,
        ..Default::default()
    }
}

/// Logs events and runs the swarm with every new peer.
async fn auto(w: World, i: u8, node: Node, mut events: NodeEvents, swarm: SwarmKey) {
    let clock = SimClock::turmoil(epoch());
    while let Some(e) = events.recv().await {
        let line = match &e {
            NodeEvent::PeerAdded(p) => {
                node.run_swarm(p.host_id, swarm, 1, 1).await.unwrap();
                format!("peer+ {} {:?} dialed={}", p.host_id, p.kind, p.dialed)
            }
            NodeEvent::PeerRemoved { host_id } => format!("peer- {host_id}"),
            other => format!("{other:?}"),
        };
        w.log(&clock, i, line);
    }
}

/// The observer: sampling queries or no-op calls, never logged.
async fn observe(w: World, node: Node) {
    if w.observer == Observer::None {
        return;
    }
    let nobody = HostId::default();
    let mut every = tokio::time::interval(SAMPLE_EVERY);
    every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    every.tick().await;
    loop {
        every.tick().await;
        match w.observer {
            Observer::None => return,
            Observer::Sample => {
                let Ok(m) = node.metrics().await else { return };
                let Ok(p) = node.peers().await else { return };
                std::hint::black_box((m, p));
            }
            Observer::Poke => {
                let Ok(found) = node.close_peer(nobody).await else {
                    return;
                };
                assert!(!found);
            }
        }
        *w.samples.borrow_mut() += 1;
    }
}

async fn host(w: World, i: u8) -> turmoil::Result {
    let ip: IpAddr = turmoil::lookup(format!("n{i}"));
    let mut config = NodeConfig::new(w.profile_key(i), w.seed.wrapping_add(u64::from(i)));
    config.label = format!("n{i}");
    config.tcp_listen = Some(format!("0.0.0.0:{TCP_PORT}"));
    config.ws_listen = Some(format!("0.0.0.0:{WS_PORT}"));
    config.public_ip = Some(ip);
    config.allow_insecure_ws = true;
    config.max_peers = 16;
    config.rate_limit = Some(UPLOAD_CAP);
    let clock = SimClock::turmoil(epoch());
    let (node, events) = Node::spawn(config, TurmoilNet, clock).await.unwrap();
    w.ids.borrow_mut().insert(i, node.host_id());
    w.nodes.borrow_mut().insert(i, node.clone());

    let id = SwarmId::new(&w.swarm_key().public);
    let (swarm, writer) = if i == 0 {
        let (s, _, writer) = node
            .create_swarm(WriterOptions {
                swarm_options: swarm_options(),
                key: w.swarm_key(),
            })
            .await
            .unwrap();
        (s, Some(writer))
    } else {
        (node.add_swarm(id, swarm_options()).await.unwrap(), None)
    };
    tokio::task::spawn_local(auto(w.clone(), i, node.clone(), events, swarm));
    tokio::task::spawn_local(observe(w.clone(), node.clone()));

    // Dial in a fixed order, once the listener's host is up.
    for &(from, to, ws) in &LINKS {
        if from != i {
            continue;
        }
        while !w.ids.borrow().contains_key(&to) {
            tokio::time::sleep(StdDuration::from_millis(1)).await;
        }
        let peer = node.dial(&w.uri(to, ws)).await.unwrap();
        assert_eq!(Some(peer), w.ids.borrow().get(&to).copied());
    }

    if let Some(mut writer) = writer {
        // Start once every link is up.
        tokio::time::sleep(StdDuration::from_millis(500)).await;
        let data = w.stream();
        for chunk in data.chunks(WRITE) {
            writer.write(chunk).await.unwrap();
            tokio::time::sleep(WRITE_EVERY).await;
        }
        writer.flush().await.unwrap();
        w.log(&clock, i, "published".to_owned());
        std::future::pending::<()>().await;
    }

    let mut reader = node.open_reader(swarm).await.unwrap();
    let mut out = vec![0u8; STREAM_LEN];
    let mut off = 0;
    let mut mark = 0;
    while off < STREAM_LEN {
        off += reader.read(&mut out[off..]).await.unwrap();
        if off / (32 * 1024) > mark {
            mark = off / (32 * 1024);
            w.log(&clock, i, format!("read {off}"));
        }
    }
    assert!(out == w.stream(), "n{i}: stream bytes differ");
    w.log(
        &clock,
        i,
        format!("stream {}", hex::encode(Sha256::digest(&out))),
    );
    *w.done.borrow_mut() += 1;
    std::future::pending::<()>().await;
    Ok(())
}

/// Clears what an observer is allowed to change: the query counter.
fn scrub(mut m: NodeMetrics) -> NodeMetrics {
    m.driver.queries = 0;
    m
}

/// Waits for every viewer, then logs every host's final metrics.
async fn control(w: World) -> turmoil::Result {
    let clock = SimClock::turmoil(epoch());
    while *w.done.borrow() < VIEWERS {
        tokio::time::sleep(StdDuration::from_millis(1)).await;
    }
    let nodes: Vec<(u8, Node)> = w.nodes.borrow().clone().into_iter().collect();
    for (i, node) in nodes {
        let m = scrub(node.metrics().await.unwrap());
        // Every node has peers, so its passes come at least every ppspp
        // tick: no query, sample or not, had to wake it.
        assert_eq!(m.driver.query_wakes, 0, "n{i}");
        let sent = m.driver.bytes_written;
        let elapsed = (clock.now() - epoch()).as_secs_f64();
        assert!(sent as f64 <= UPLOAD_CAP as f64 * (elapsed + 1.0) + 65_536.0);
        w.log(
            &clock,
            i,
            format!(
                "frames out {} in {} written {sent} loops {} commands {} metrics {}",
                m.host.frame_write_count,
                m.host.frame_read_count,
                m.driver.loop_iterations,
                m.driver.commands,
                hex::encode(Sha256::digest(format!("{m:?}").as_bytes()))
            ),
        );
    }
    Ok(())
}

struct Run {
    trace: Vec<String>,
    samples: u64,
}

fn run(seed: u64, observer: Observer) -> Run {
    let w = World {
        seed,
        observer,
        trace: Rc::new(RefCell::new(Vec::new())),
        ids: Rc::new(RefCell::new(BTreeMap::new())),
        nodes: Rc::new(RefCell::new(BTreeMap::new())),
        done: Rc::new(RefCell::new(0)),
        samples: Rc::new(RefCell::new(0)),
    };
    let mut sim = turmoil::Builder::new()
        .rng_seed(seed)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_secs(1_700_000_000))
        .simulation_duration(StdDuration::from_secs(120))
        .min_message_latency(StdDuration::from_millis(1))
        .max_message_latency(StdDuration::from_millis(15))
        .build();
    for i in 0..=VIEWERS {
        let wh = w.clone();
        sim.host(format!("n{i}"), move || host(wh.clone(), i));
    }
    sim.client("control", control(w.clone()));
    sim.run().unwrap();
    w.nodes.borrow_mut().clear();
    let trace = w.trace.borrow().clone();
    let samples = *w.samples.borrow();
    Run { trace, samples }
}

fn digest(trace: &[String]) -> String {
    let mut h = Sha256::new();
    for l in trace {
        h.update(l.as_bytes());
        h.update(b"\n");
    }
    hex::encode(h.finalize())
}

/// The first line where two traces part.
fn first_difference(a: &[String], b: &[String]) -> String {
    let i = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    format!(
        "line {i}: {:?} vs {:?}",
        a.get(i).map(String::as_str),
        b.get(i).map(String::as_str)
    )
}

fn check(seed: u64) {
    let replay = format!(
        "replay: cargo test -p strims-node --features turmoil --test turmoil_observer \
         sampling_metrics_leaves_a_contended_run_unchanged -- --exact --nocapture  (seed {seed})"
    );
    let quiet = run(seed, Observer::None);
    let sampled = run(seed, Observer::Sample);
    let poked = run(seed, Observer::Poke);
    eprintln!(
        "seed {seed}: quiet {} lines {}; sampled {} lines {} ({} samples); \
         poked {} lines {} ({} calls)",
        quiet.trace.len(),
        digest(&quiet.trace),
        sampled.trace.len(),
        digest(&sampled.trace),
        sampled.samples,
        poked.trace.len(),
        digest(&poked.trace),
        poked.samples,
    );
    assert_eq!(quiet.samples, 0);
    assert!(
        sampled.samples > 200,
        "{} samples\n{replay}",
        sampled.samples
    );
    assert!(
        quiet.trace == sampled.trace,
        "sampling changed the run: {}\n{replay}",
        first_difference(&quiet.trace, &sampled.trace)
    );
    // The scenario must be sensitive to a wakeup, or the check above
    // proves nothing. If this fails, raise the load.
    assert!(
        quiet.trace != poked.trace,
        "a no-op call every 100 ms didn't change the run: the scenario isn't contended\n{replay}"
    );
    eprintln!(
        "seed {seed}: a no-op call every 100 ms parts the run at {}",
        first_difference(&quiet.trace, &poked.trace)
    );
}

#[test]
fn sampling_metrics_leaves_a_contended_run_unchanged() {
    for seed in [2, 3] {
        check(seed);
    }
}
