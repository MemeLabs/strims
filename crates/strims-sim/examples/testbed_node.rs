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
//!              [--rate <bytes/s>] [--chunk <bytes> --munro <bytes>]
//!              [--units <chunks per signature> [--media <flv file>]]
//!              [--record <file>] [--join-at <s>] [--arrive] [--hosts <dir>]
//!              [--capture <dir>] [--forecast] [--spec] [--parent-checks] [--timer-log]
//!              [--timer-probe] [--link-log]
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
//! after the start, all at once, each dialed until it links (the
//! topology module's docs say why), and with `--arrive` it arrives into
//! the running stream, reading from the live edge
//! ([`Settings::arrives`]). `--chunk` and
//! `--munro` size the swarm's chunks and signatures
//! ([`with_chunks`](strims_testbed::shape::LiveShape::with_chunks)); every
//! node must take the same. With `--units`, the stream is media in the
//! unit format in a swarm of that many chunks per signature
//! ([`strims_testbed::media`]), every node taking the same, and the seed
//! replays the FLV file `--media` names. A viewer of it with `--record`
//! writes the Data messages it reads to that file, for players to replay
//! at the times they came: each is its arrival time in Unix milliseconds
//! (u64, big-endian), a flags byte (bit 0 the message completes a record,
//! bit 1 a discontinuity came before it), its length (u32, big-endian) and
//! its bytes. Each node prints its line per second on stdout, with the
//! process's CPU time as `cpu_ms` and the records strims-sim's tables take
//! (`rec`), for which it keeps stats bins of
//! [`BIN_MS`](strims_testbed::record::BIN_MS) from the stream's start and
//! its parent log, as a simulated node does. `--forecast` keeps its
//! scheduler's forecast log and `--spec` its request and seed logs
//! (`NodeConfig::forecast_log`, `request_log`), as strims-sim's
//! `STRIMS_SIM_FORECAST=1` and `STRIMS_SIM_SPEC=1` keep a simulated
//! node's, and its lines' records carry them
//! ([`strims_testbed::record::logs`]). `--parent-checks` keeps its
//! parent checks (`NodeConfig::decision_log`'s `ParentChoice`s, a
//! [`DecisionLog`]), as `STRIMS_SIM_PARENT_CHECKS=1` keeps a simulated
//! node's; it takes the tap a capture would, so it fails with
//! `--capture`.
//! ([`strims_testbed::record::logs`]). `--timer-log` keeps its timers as
//! they fire (`NodeConfig::timer_log`, [`TimerLog`]): its node task's, its
//! line's and its frame writer's, each read by the wall clock to the
//! nanosecond, which its lines' records carry, as strims-sim's
//! `STRIMS_SIM_TIMER_LOG` keeps a simulated node's; with `--timer-probe`
//! the node runs the timer probe through the stream
//! ([`strims_testbed::node::probe`]), as `STRIMS_SIM_TIMER_PROBE=1` has a
//! simulated node run it. `--link-log` keeps its links' I/O as it happens
//! (`NodeConfig::link_log`, [`LinkLog`]): its node task's hand-offs and
//! takes, its link tasks' writes and reads and its sockets' readings, by
//! the wall clock to the nanosecond, which its lines' records carry, as
//! strims-sim's `STRIMS_SIM_LINK_LOG` keeps a simulated node's.
//!
//! `--relay-only` runs the relay: it listens where the topology's `relay`
//! line says, joins the testbed network, serves its peers' transfers and
//! runs no swarm ([`strims_testbed::peers`]), and stops 5 s after the
//! stream does.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::cell::RefCell;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use strims_base::time::{Duration as Span, Time};
use strims_node::app::{NodeTimer, TokioSpawn};
use strims_node::config::StatsBins;
use strims_node::{
    CaptureConfig, Clock, NetworkOptions, Node, NodeConfig, NodeEvent, NodeEvents, SystemClock,
    TokioNet, WebRtcConfig,
};
use strims_testbed::media::FlvFile;
use strims_testbed::native::{now_ms, self_cpu_ms, write_host};
use strims_testbed::node::{Runtime, Settings, Stream};
use strims_testbed::peers::{self, Peers};
use strims_testbed::record::links::LinkLog;
use strims_testbed::record::{BIN_MS, DecisionLog, ParentLog, TimerContext, TimerLog};
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
    chunk: Option<usize>,
    munro: Option<usize>,
    units: Option<usize>,
    media: Option<String>,
    record: Option<String>,
    /// When a viewer joins, in ms after the start (`--join-at`, seconds).
    join_at_ms: u64,
    arrive: bool,
    hosts: Option<PathBuf>,
    capture: Option<String>,
    forecast: bool,
    spec: bool,
    parent_checks: bool,
    timer_log: bool,
    timer_probe: bool,
    link_log: bool,
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
        chunk: None,
        munro: None,
        units: None,
        media: None,
        record: None,
        join_at_ms: 0,
        arrive: false,
        hosts: None,
        capture: None,
        forecast: false,
        spec: false,
        parent_checks: false,
        timer_log: false,
        timer_probe: false,
        link_log: false,
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
            "--chunk" => a.chunk = Some(num(value()?)? as usize),
            "--munro" => a.munro = Some(num(value()?)? as usize),
            "--units" => a.units = Some(num(value()?)? as usize),
            "--media" => a.media = Some(value()?),
            "--record" => a.record = Some(value()?),
            "--join-at" => a.join_at_ms = secs_ms(&value()?)?,
            "--arrive" => a.arrive = true,
            "--hosts" => a.hosts = Some(value()?.into()),
            "--capture" => a.capture = Some(value()?),
            "--forecast" => a.forecast = true,
            "--spec" => a.spec = true,
            "--parent-checks" => a.parent_checks = true,
            "--timer-log" => a.timer_log = true,
            "--timer-probe" => a.timer_probe = true,
            "--link-log" => a.link_log = true,
            _ => return Err(format!("unknown flag {flag}")),
        }
    }
    if (a.index == usize::MAX && !a.relay_only) || a.topology.is_empty() {
        return Err("--index (or --relay-only) and --topology are required".to_owned());
    }
    if a.chunk.is_some() != a.munro.is_some() {
        return Err("--chunk and --munro go together".to_owned());
    }
    if a.timer_probe && !a.timer_log {
        return Err("--timer-probe needs --timer-log".to_owned());
    }
    if a.media.is_some() && a.units.is_none() {
        return Err("--media needs --units".to_owned());
    }
    if a.parent_checks && a.capture.is_some() {
        return Err("--parent-checks takes the tap --capture would".to_owned());
    }
    Ok(a)
}

/// The tokio driver's side of the node, and the file a viewer records its
/// Data messages in.
#[derive(Clone, Default)]
struct Native {
    record: Option<Rc<RefCell<BufWriter<File>>>>,
}

impl Runtime for Native {
    fn now_ms(&self) -> u64 {
        now_ms()
    }

    fn precise_now(&self) -> Time {
        SystemClock.precise_now()
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

    fn media(&self, data: &[u8], segment_end: bool, discontinuity: bool) {
        let Some(f) = &self.record else {
            return;
        };
        let flags = u8::from(segment_end) | u8::from(discontinuity) << 1;
        let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
        let mut f = f.borrow_mut();
        let written = f
            .write_all(&now_ms().to_be_bytes())
            .and_then(|()| f.write_all(&[flags]))
            .and_then(|()| f.write_all(&len.to_be_bytes()))
            .and_then(|()| f.write_all(data));
        if let Err(e) = written {
            eprintln!("record: {e}");
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // The peers' RPC connections run as local tasks (TokioSpawn).
    let ran = tokio::task::LocalSet::new().run_until(run()).await;
    if let Err(e) = ran {
        eprintln!("testbed_node: {e}");
        std::process::exit(1);
    }
}

/// Sleeps until the wall clock reads `at_ms`, a viewer's join, which its
/// tables stamp at that time: a deployed viewer's dials go as it joins, its
/// node reaching connect() in microseconds. tokio's timer wakes up to a ms
/// past its deadline, and [`Runtime::sleep_until_ms`] reads the clock to
/// the ms, which would put the dials up to 2 ms past the stamp, so the
/// last 2 ms are yielded away.
async fn join_at(at_ms: u64) {
    let at = Time::from_unix_millis(i64::try_from(at_ms).unwrap_or(i64::MAX));
    loop {
        let left = at - SystemClock.precise_now();
        if left <= Span::ZERO {
            return;
        }
        match (left - Span::from_millis(2)).to_std() {
            Some(d) if !d.is_zero() => tokio::time::sleep(d).await,
            _ => tokio::task::yield_now().await,
        }
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
    let file = match &a.media {
        Some(path) => {
            let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
            Some(FlvFile::parse(bytes).map_err(|e| format!("{path}: {e}"))?)
        }
        None => None,
    };
    let mut config = NodeConfig::new(key_of(a.index), a.index as u64 + 1);
    config.label = format!("testbed-{}", a.index);
    config.tcp_listen = Some(me.addr.to_string());
    config.max_peers = 64;
    config.rate_limit = None;
    config.upload_capacity = None;
    // The records strims-sim's tables take: stats bins from the stream's
    // start, and the parent log.
    config.stats_bins = Some(StatsBins {
        origin: Time::from_unix_millis(i64::try_from(start_ms).unwrap_or(i64::MAX)),
        width: Span::from_millis(BIN_MS),
    });
    let parents = ParentLog::default();
    config.parent_log = Some(parents.sink());
    config.forecast_log = a.forecast;
    config.request_log = a.spec;
    let checks = a.parent_checks.then(DecisionLog::default);
    config.decision_log = checks.as_ref().map(DecisionLog::sink);
    let timers = a.timer_log.then(|| TimerLog::new(TimerContext::Task));
    config.timer_log = timers.as_ref().map(TimerLog::sink);
    let link_log = a.link_log.then(|| {
        let clock = SystemClock;
        LinkLog::new(Arc::new(move || clock.precise_now()))
    });
    config.link_log = link_log.as_ref().map(LinkLog::sink);
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
    let (node, events) = Node::spawn(config, TokioNet, SystemClock)
        .await
        .map_err(|e| format!("spawn: {e}"))?;

    let (peers, forward) = serve(&node, events, |_| {});
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
            join_at(start_ms + a.join_at_ms).await;
        }
        // Every link of the plan at once (strims_testbed::topology).
        let dials = topo
            .plan(a.index)
            .into_iter()
            .filter_map(|link| match link {
                Link::Dial(uri) => Some(uri),
                Link::Exchange(j) => {
                    eprintln!("native nodes don't exchange (with node {j})");
                    None
                }
            });
        let node = &node;
        futures_util::future::join_all(dials.map(|uri| async move { dial(node, &uri).await }))
            .await;
        std::future::pending::<()>().await;
    };
    let settings = Settings {
        index: a.index,
        nodes: topo.nodes.len(),
        start_ms,
        secs: a.secs,
        arrives: a.arrive,
        rate: a.rate,
        chunks: a.chunk.zip(a.munro),
        stream: a.units.map_or(Stream::Frames, |cps| Stream::Units {
            chunks_per_signature: cps,
        }),
        probe: a.timer_probe,
    };
    let rt = Native {
        record: match &a.record {
            Some(path) => Some(Rc::new(RefCell::new(BufWriter::new(
                File::create(path).map_err(|e| format!("{path}: {e}"))?,
            )))),
            None => None,
        },
    };
    let ran = tokio::select! {
        biased;
        r = strims_testbed::node::run(
            &node,
            peers,
            settings,
            file.as_ref(),
            strims_testbed::node::NodeLogs {
                parents: Some(&parents),
                checks: checks.as_ref(),
                timers: timers.as_ref(),
                links: link_log.as_ref(),
            },
            &rt,
        ) => r,
        () = async { tokio::join!(forward, links); } => Ok(()),
    };
    let _ = node.shutdown().await;
    if let Some(f) = &rt.record
        && let Err(e) = f.borrow_mut().flush()
    {
        eprintln!("record: {e}");
    }
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
    let (node, events) = Node::spawn(config, TokioNet, SystemClock)
        .await
        .map_err(|e| format!("spawn: {e}"))?;
    join_network(&node).await?;
    eprintln!("relay {} on {} and {}", node.host_id(), r.tcp, r.ws);
    let (mut peers, forward) = serve(&node, events, |e| {
        if let NodeEvent::PeerAdded(p) = e {
            eprintln!("{} peer {} ({:?})", now_ms(), p.host_id, p.kind);
        }
    });
    let rt = Native::default();
    tokio::select! {
        biased;
        () = rt.sleep_until_ms(end_ms) => {}
        () = forward => {}
        () = peers.serve(|_| {}) => {}
    }
    let _ = node.shutdown().await;
    Ok(())
}

/// The node's peers ([`Peers`]), and the task that hands them the node's
/// events, each seen by `each` first.
fn serve(
    node: &Node,
    mut events: NodeEvents,
    mut each: impl FnMut(&NodeEvent),
) -> (
    Peers<NodeTimer<SystemClock>, TokioSpawn>,
    impl std::future::Future<Output = ()>,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let forward = async move {
        while let Some(e) = events.recv().await {
            each(&e);
            if peers::serves(&e) {
                let _ = tx.send(e);
            }
        }
    };
    (
        Peers::new(node, rx, NodeTimer(SystemClock), TokioSpawn),
        forward,
    )
}

/// A time in seconds (`2`, `0.5`) as whole ms.
fn secs_ms(v: &str) -> Result<u64, String> {
    let s: f64 = v.parse().map_err(|e| format!("{v}: {e}"))?;
    if !s.is_finite() || s < 0.0 {
        return Err(format!("{v}: seconds, 0 or more"));
    }
    Ok((s * 1000.0).round() as u64)
}
