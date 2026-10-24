// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! One node of the calibration testbed (`hack/testbed/`): a real strims
//! node on real sockets and the wall clock, streaming or reading one swarm
//! shaped like strims-sim's WAN cases (16 streams, Hybrid, the adaptive
//! seed, 1 KiB frames written every 250 ms), so a shaped real link can be
//! compared with the simulated one. It sets no upload limit and no known
//! capacity: the upload meter measures on its own.
//!
//! ```text
//! testbed_node --index <i> --topology <file> [--secs <s>] [--start <unix ms>]
//!              [--rate <bytes/s>] [--join-at <s>] [--capture <dir>]
//! ```
//!
//! The topology file has a line `node <i> <ip:port>` per node and a line
//! `link <a> <b>` per link; node 0 seeds. Each node listens on its own
//! address and dials the linked nodes with a lower index. Every peer that
//! connects runs the swarm. The seed starts writing at `--start` (all
//! nodes share the clock) and writes for `--secs`; a viewer dials its
//! links `--join-at` seconds after the start. Each node prints one JSON line
//! per second on stdout: its upload meter, its channels' round trips, its
//! transfers per peer (`strims_sim::TransferSample`, peers by index) and,
//! at a viewer, the lag of the frames it read in that second.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::time::Duration;

use strims_crypto::key_from_seed;
use strims_node::{
    CaptureConfig, HostId, Node, NodeConfig, NodeEvent, ReadError, SwarmId, SystemClock, TokioNet,
    WriterOptions,
};
use strims_ppspp::{PushMode, SeedPolicy};
use strims_sim::stream::{FRAME_LEN, FrameParser, Parsed, encode_frame};
use strims_sim::{SwarmSpec, TransferSample};

/// The stream rate of strims-sim's WAN cases.
const WAN_RATE: u64 = 256 * 1024;
/// Their live window, in chunks.
const WAN_WINDOW: usize = 4096;
const STREAMS: usize = 16;
/// The frame payload seed shared by every node.
const RUN_SEED: u64 = 1;

struct Args {
    index: usize,
    topology: String,
    secs: u64,
    start_ms: Option<u64>,
    rate: u64,
    join_at: u64,
    capture: Option<String>,
}

fn args() -> Result<Args, String> {
    // The testbed's settings come from its command line.
    #[allow(clippy::disallowed_methods)]
    let mut it = std::env::args().skip(1);
    let mut a = Args {
        index: usize::MAX,
        topology: String::new(),
        secs: 180,
        start_ms: None,
        rate: WAN_RATE,
        join_at: 0,
        capture: None,
    };
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        let num = |v: String| v.parse::<u64>().map_err(|e| format!("{flag}: {e}"));
        match flag.as_str() {
            "--index" => a.index = num(value()?)? as usize,
            "--topology" => a.topology = value()?,
            "--secs" => a.secs = num(value()?)?,
            "--start" => a.start_ms = Some(num(value()?)?),
            "--rate" => a.rate = num(value()?)?,
            "--join-at" => a.join_at = num(value()?)?,
            "--capture" => a.capture = Some(value()?),
            _ => return Err(format!("unknown flag {flag}")),
        }
    }
    if a.index == usize::MAX || a.topology.is_empty() {
        return Err("--index and --topology are required".to_owned());
    }
    Ok(a)
}

struct Topology {
    nodes: Vec<(usize, String)>,
    links: Vec<(usize, usize)>,
}

fn topology(path: &str) -> Result<Topology, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut t = Topology {
        nodes: Vec::new(),
        links: Vec::new(),
    };
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |s: &str| s.parse::<usize>().map_err(|e| format!("{line:?}: {e}"));
        match w.as_slice() {
            ["node", i, addr] => t.nodes.push((n(i)?, (*addr).to_owned())),
            ["link", a, b] => t.links.push((n(a)?, n(b)?)),
            [] => {}
            _ => return Err(format!("bad topology line {line:?}")),
        }
    }
    Ok(t)
}

/// The wall clock in Unix milliseconds.
fn now_ms() -> u64 {
    // The testbed measures lag against the wall clock its containers share.
    #[allow(clippy::disallowed_methods)]
    let now = std::time::SystemTime::now();
    now.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

fn key_of(i: usize) -> strims_proto::types::Key {
    key_from_seed(&[u8::try_from(i + 1).unwrap_or(u8::MAX); 32])
}

fn swarm_spec(rate: u64) -> SwarmSpec {
    SwarmSpec::live(0, Vec::new(), rate, Duration::ZERO, Duration::ZERO)
        .with_push_mode(PushMode::Hybrid)
        .with_live_window(WAN_WINDOW)
        .with_stream_count(STREAMS)
        .with_seed_policy(SeedPolicy::ADAPTIVE)
}

/// Percentile `q` of the sorted `v`, 0 when empty.
fn pct(v: &[u64], q: f64) -> u64 {
    v.get(((v.len().max(1) - 1) as f64 * q).round() as usize)
        .copied()
        .unwrap_or(0)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("testbed_node: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let a = args()?;
    let topo = topology(&a.topology)?;
    let listen = topo
        .nodes
        .iter()
        .find(|(i, _)| *i == a.index)
        .map(|(_, addr)| addr.clone())
        .ok_or("this node isn't in the topology")?;
    let mut config = NodeConfig::new(key_of(a.index), a.index as u64 + 1);
    config.label = format!("testbed-{}", a.index);
    config.tcp_listen = Some(listen);
    config.max_peers = 64;
    config.rate_limit = None;
    config.upload_capacity = None;
    if let Some(dir) = &a.capture {
        std::fs::create_dir_all(dir).map_err(|e| format!("{dir}: {e}"))?;
        let capture = CaptureConfig {
            dir: dir.into(),
            max_bytes: 4 << 30,
            tap: true,
        };
        config.ppspp_capture = Some(capture.node_capture());
    }
    let (node, mut events) = Node::spawn(config, TokioNet, SystemClock)
        .await
        .map_err(|e| format!("spawn: {e}"))?;

    let spec = swarm_spec(a.rate);
    let swarm_key = key_from_seed(&[0x55; 32]);
    let seed = a.index == 0;
    let (swarm, writer) = if seed {
        let (swarm, _, writer) = node
            .create_swarm(WriterOptions {
                swarm_options: spec.options.clone(),
                key: swarm_key.clone(),
            })
            .await
            .map_err(|e| format!("create swarm: {e}"))?;
        (swarm, Some(writer))
    } else {
        let swarm = node
            .add_swarm(SwarmId::new(&swarm_key.public), spec.options.clone())
            .await
            .map_err(|e| format!("add swarm: {e}"))?;
        (swarm, None)
    };

    // Every peer that connects runs the swarm, whoever dialed. Its index
    // comes from its profile key; its host key is fresh.
    let ids = std::rc::Rc::new(std::cell::RefCell::new(Vec::<(HostId, usize)>::new()));
    let peers = {
        let node = node.clone();
        let ids = ids.clone();
        let n = topo.nodes.len();
        async move {
            while let Some(e) = events.recv().await {
                let NodeEvent::PeerAdded(p) = e else {
                    continue;
                };
                if let Some(i) = (0..n).find(|&i| key_of(i).public == p.profile_key) {
                    ids.borrow_mut().push((p.host_id, i));
                }
                if let Err(e) = node.run_swarm(p.host_id, swarm, 1, 1).await {
                    eprintln!("run swarm: {e}");
                }
            }
        }
    };
    let dials = {
        let node = node.clone();
        let targets: Vec<String> = topo
            .links
            .iter()
            .filter_map(|&(x, y)| match (x == a.index, y == a.index) {
                (true, false) if y < a.index => Some(y),
                (false, true) if x < a.index => Some(x),
                _ => None,
            })
            .filter_map(|j| topo.nodes.iter().find(|(i, _)| *i == j))
            .map(|(j, addr)| {
                let id = hex::encode(key_of(*j).public);
                format!("tcp://{addr}/{id}")
            })
            .collect();
        let join_ms = a.start_ms.map(|s| s + a.join_at * 1000);
        async move {
            if let Some(at) = join_ms {
                tokio::time::sleep(Duration::from_millis(at.saturating_sub(now_ms()))).await;
            }
            for uri in targets {
                loop {
                    match node.dial(&uri).await {
                        Ok(_) => break,
                        Err(e) => {
                            eprintln!("dial {uri}: {e}");
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                    }
                }
            }
        }
    };

    let start_ms = a.start_ms.unwrap_or_else(|| now_ms() + 2000);
    let end_ms = start_ms + a.secs * 1000;
    let lags = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u64>::new()));
    let write = async {
        let Some(mut writer) = writer else {
            return;
        };
        let fpw = spec.frames_per_write as u64;
        let step = spec.write_interval.as_millis() as u64;
        let mut buf = vec![0u8; spec.frames_per_write * FRAME_LEN];
        for k in 0.. {
            let at = start_ms + k * step;
            if at >= end_ms {
                break;
            }
            tokio::time::sleep(Duration::from_millis(at.saturating_sub(now_ms()))).await;
            let t = now_ms();
            for (j, f) in buf.chunks_mut(FRAME_LEN).enumerate() {
                encode_frame(RUN_SEED, 0, k * fpw + j as u64, t, f);
            }
            if let Err(e) = writer.write(&buf).await {
                eprintln!("write: {e}");
                return;
            }
        }
        // Dropping the writer would close the swarm.
        std::future::pending::<()>().await;
    };
    let read = {
        let lags = lags.clone();
        let node = node.clone();
        async move {
            if seed {
                return;
            }
            let mut reader = match node.open_reader(swarm).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("open reader: {e}");
                    return;
                }
            };
            let mut buf = vec![0u8; 64 * 1024];
            let mut parser = FrameParser::default();
            loop {
                match reader.read(&mut buf).await {
                    Ok(n) => {
                        let now = now_ms();
                        parser.push(RUN_SEED, 0, buf.get(..n).unwrap_or_default(), |p| {
                            if let Parsed::Frame(f) = p {
                                lags.borrow_mut().push(now.saturating_sub(f.time_ms));
                            }
                        });
                    }
                    Err(ReadError::Underrun { .. } | ReadError::Reset) => {
                        parser.resync();
                    }
                    Err(e) => {
                        eprintln!("read: {e}");
                        return;
                    }
                }
            }
        }
    };
    let sample = async {
        let mut next = start_ms.saturating_sub(5000);
        loop {
            tokio::time::sleep(Duration::from_millis(next.saturating_sub(now_ms()))).await;
            next += 1000;
            let t = now_ms();
            let Ok(m) = node.metrics().await else {
                continue;
            };
            let push = m
                .runner
                .swarms
                .iter()
                .find(|s| s.swarm == swarm)
                .and_then(|s| s.push.clone());
            let mut lag = std::mem::take(&mut *lags.borrow_mut());
            lag.sort_unstable();
            let rtts: Vec<String> = m
                .runner
                .peers
                .iter()
                .flat_map(|p| &p.channels)
                .filter(|c| c.swarm == swarm)
                .filter_map(|c| {
                    c.stats
                        .map(|s| s.scheduler.rtt_mean.as_millis().to_string())
                })
                .collect();
            let p = push.unwrap_or_default();
            let ids = ids.borrow();
            let index = |id: &HostId| ids.iter().find(|(x, _)| x == id).map(|&(_, i)| i);
            let transfers =
                TransferSample::of(&m, swarm, index).map_or("null".to_owned(), |x| x.json());
            println!(
                "{{\"t_ms\":{t},\"rel_ms\":{},\"index\":{},\"seed\":{seed},\"bytes_written\":{},\"upload_rate\":{},\"busy_permille\":{},\"capacity\":{},\"queue_delay_ms\":{},\"children\":{},\"seed_budget\":{},\"seed_window\":{},\"seed_copies\":{},\"seed_first_wave\":{},\"frames\":{},\"lag_p50_ms\":{},\"lag_p99_ms\":{},\"lag_max_ms\":{},\"rtt_mean_ms\":[{}],\"transfers\":{transfers}}}",
                t as i64 - start_ms as i64,
                a.index,
                m.driver.bytes_written,
                p.upload_rate,
                p.upload_busy_permille,
                p.upload_capacity,
                p.upload_queue_delay
                    .map_or("null".to_owned(), |d| d.as_millis().to_string()),
                p.children,
                p.seed_budget,
                p.seed_window,
                p.seed_copies,
                p.seed_first_wave,
                lag.len(),
                pct(&lag, 0.5),
                pct(&lag, 0.99),
                lag.last().copied().unwrap_or(0),
                rtts.join(","),
            );
            if t >= end_ms + 5000 {
                return;
            }
        }
    };
    tokio::select! {
        biased;
        () = sample => {}
        () = async { tokio::join!(peers, dials, write, read); } => {}
    }
    let _ = node.shutdown().await;
    Ok(())
}
