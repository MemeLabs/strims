// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! One native node of the calibration testbed (`hack/testbed/`): a real
//! strims node on real sockets and the wall clock, streaming or reading
//! the testbed's swarm ([`strims_testbed::node`]), so a shaped real link
//! can be compared with the simulated one. It sets no upload limit and no
//! known capacity: the upload meter measures on its own.
//!
//! ```text
//! testbed_node --index <i> --topology <file> [--secs <s>] [--start <unix ms>]
//!              [--rate <bytes/s>] [--join-at <s>] [--hosts <dir>]
//!              [--capture <dir>]
//! testbed_node --relay-only --topology <file> [--secs <s>] [--start <unix ms>]
//! ```
//!
//! The topology file ([`strims_testbed::topology`]) names the nodes, their
//! links and the relay; node 0 seeds. A node listens on its own address
//! and dials the native nodes it links to with a lower index. With a
//! relay in the topology, it also listens for browsers (a WebSocket
//! listener, or WebRTC with `browser-links webrtc`), joins the testbed
//! network, dials the relay, and then, with `--hosts`, writes its host id
//! there ([`strims_testbed::native`]) for the browsers that exchange with
//! it. The seed starts writing at `--start` (all nodes share the clock)
//! and writes for `--secs`; a viewer makes its links `--join-at` seconds
//! after the start. Each node prints its line per second on stdout, with
//! the process's CPU time as `cpu_ms`.
//!
//! `--relay-only` runs the relay: it listens where the topology's `relay`
//! line says, joins the testbed network, runs no swarm, and stops 5 s
//! after the stream does.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::PathBuf;
use std::time::Duration;

use strims_node::{
    CaptureConfig, NetworkOptions, Node, NodeConfig, NodeEvent, SystemClock, TokioNet, WebRtcConfig,
};
use strims_testbed::native::{now_ms, self_cpu_ms, write_host};
use strims_testbed::node::{Runtime, Settings};
use strims_testbed::shape::RATE;
use strims_testbed::topology::{
    BIND_PORT, BrowserLinks, Link, Topology, key_of, network_key, relay_key,
};

struct Args {
    index: usize,
    relay_only: bool,
    topology: String,
    secs: u64,
    start_ms: Option<u64>,
    rate: u64,
    join_at: u64,
    hosts: Option<PathBuf>,
    capture: Option<String>,
}

fn args() -> Result<Args, String> {
    // The testbed's settings come from its command line.
    #[allow(clippy::disallowed_methods)]
    let mut it = std::env::args().skip(1);
    let mut a = Args {
        index: usize::MAX,
        relay_only: false,
        topology: String::new(),
        secs: 180,
        start_ms: None,
        rate: RATE,
        join_at: 0,
        hosts: None,
        capture: None,
    };
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        let num = |v: String| v.parse::<u64>().map_err(|e| format!("{flag}: {e}"));
        match flag.as_str() {
            "--index" => a.index = num(value()?)? as usize,
            "--relay-only" => a.relay_only = true,
            "--topology" => a.topology = value()?,
            "--secs" => a.secs = num(value()?)?,
            "--start" => a.start_ms = Some(num(value()?)?),
            "--rate" => a.rate = num(value()?)?,
            "--join-at" => a.join_at = num(value()?)?,
            "--hosts" => a.hosts = Some(value()?.into()),
            "--capture" => a.capture = Some(value()?),
            _ => return Err(format!("unknown flag {flag}")),
        }
    }
    if (a.index == usize::MAX && !a.relay_only) || a.topology.is_empty() {
        return Err("--index (or --relay-only) and --topology are required".to_owned());
    }
    Ok(a)
}

/// The tokio driver's side of the node.
struct Native;

impl Runtime for Native {
    fn now_ms(&self) -> u64 {
        now_ms()
    }

    async fn sleep_until_ms(&self, at_ms: u64) {
        tokio::time::sleep(Duration::from_millis(at_ms.saturating_sub(now_ms()))).await;
    }

    fn cpu_ms(&self) -> Option<u64> {
        self_cpu_ms()
    }

    fn emit(&self, line: String) {
        println!("{line}");
    }

    fn error(&self, message: String) {
        eprintln!("{message}");
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("testbed_node: {e}");
        std::process::exit(1);
    }
}

/// Dials `uri` until it links.
async fn dial(node: &Node, uri: &str) {
    loop {
        match node.dial(uri).await {
            Ok(_) => return,
            Err(e) => {
                eprintln!("dial {uri}: {e}");
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

async fn join_network(node: &Node) -> Result<(), String> {
    let options = NetworkOptions {
        auto_bind_port: Some(BIND_PORT),
    };
    node.add_network(&network_key(), options)
        .await
        .map(drop)
        .map_err(|e| format!("add network: {e}"))
}

async fn run() -> Result<(), String> {
    let a = args()?;
    let text = std::fs::read_to_string(&a.topology).map_err(|e| format!("{}: {e}", a.topology))?;
    let topo = Topology::parse(&text)?;
    let start_ms = a.start_ms.unwrap_or_else(|| now_ms() + 2000);
    if a.relay_only {
        return relay(&topo, start_ms + (a.secs + 5) * 1000).await;
    }
    let me = topo
        .node(a.index)
        .ok_or("this node isn't in the topology")?
        .clone();
    let mut config = NodeConfig::new(key_of(a.index), a.index as u64 + 1);
    config.label = format!("testbed-{}", a.index);
    config.tcp_listen = Some(me.addr.to_string());
    config.max_peers = 64;
    config.rate_limit = None;
    config.upload_capacity = None;
    if topo.relay.is_some() {
        let at = me.browser_addr().to_string();
        match topo.browser_links {
            BrowserLinks::Ws => config.ws_listen = Some(at),
            BrowserLinks::WebRtc => config.webrtc = WebRtcConfig::local([at]),
        }
    }
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

    let (peers_tx, peers) = tokio::sync::mpsc::unbounded_channel();
    let forward = async move {
        while let Some(e) = events.recv().await {
            if let NodeEvent::PeerAdded(p) = e {
                let _ = peers_tx.send(p);
            }
        }
    };
    let links = async {
        if let Some(uri) = topo.relay_uri(a.index) {
            if let Err(e) = join_network(&node).await {
                eprintln!("{e}");
            }
            dial(&node, &uri).await;
            // Peer exchanges route through the relay once the network has
            // heard of the link.
            tokio::time::sleep(Duration::from_secs(1)).await;
            if let Some(dir) = &a.hosts
                && let Err(e) = write_host(dir, a.index, node.host_id())
            {
                eprintln!("host file: {e}");
            }
        }
        if a.start_ms.is_some() {
            Native.sleep_until_ms(start_ms + a.join_at * 1000).await;
        }
        for link in topo.plan(a.index) {
            match link {
                Link::Dial(uri) => dial(&node, &uri).await,
                Link::Exchange(j) => eprintln!("native nodes don't exchange (with node {j})"),
            }
        }
        std::future::pending::<()>().await;
    };
    let settings = Settings {
        index: a.index,
        nodes: topo.nodes.len(),
        start_ms,
        secs: a.secs,
        rate: a.rate,
    };
    let ran = tokio::select! {
        biased;
        r = strims_testbed::node::run(&node, peers, settings, &Native) => r,
        () = async { tokio::join!(forward, links); } => Ok(()),
    };
    let _ = node.shutdown().await;
    ran
}

/// The relay, until `end_ms`.
async fn relay(topo: &Topology, end_ms: u64) -> Result<(), String> {
    let r = topo.relay.ok_or("no relay in the topology")?;
    let mut config = NodeConfig::new(relay_key(), 0xee);
    config.label = "testbed-relay".to_owned();
    config.tcp_listen = Some(r.tcp.to_string());
    config.ws_listen = Some(r.ws.to_string());
    config.max_peers = 64;
    let (node, mut events) = Node::spawn(config, TokioNet, SystemClock)
        .await
        .map_err(|e| format!("spawn: {e}"))?;
    join_network(&node).await?;
    eprintln!("relay {} on {} and {}", node.host_id(), r.tcp, r.ws);
    let log = async {
        while let Some(e) = events.recv().await {
            if let NodeEvent::PeerAdded(p) = e {
                eprintln!("{} peer {} ({:?})", now_ms(), p.host_id, p.kind);
            }
        }
    };
    tokio::select! {
        () = Native.sleep_until_ms(end_ms) => {}
        () = log => {}
    }
    let _ = node.shutdown().await;
    Ok(())
}
