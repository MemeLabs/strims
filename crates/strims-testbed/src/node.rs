// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The testbed node's swarm, on a started node of either driver.
//!
//! [`run`] streams (node 0) or reads the testbed's swarm
//! ([`LiveShape::testbed`]). It runs the swarm with every peer that has a
//! testbed index, whoever made the link; the relay has none. The seed
//! writes from [`Settings::start_ms`] for [`Settings::secs`], stamping each
//! frame with the wall clock, and a viewer measures each frame's lag
//! against the wall clock when it reads it: the testbed's containers share
//! the host's clock. It ends 5 s after the stream does.
//!
//! Every second from 5 s before the start, the node emits one JSON line
//! ([`Runtime::emit`]):
//!
//! | field | |
//! |---|---|
//! | `t_ms`, `rel_ms` | the wall clock, and ms since the start |
//! | `index`, `seed` | the node, and whether it seeds |
//! | `bytes_written` | bytes the driver wrote to its links |
//! | `upload_rate`, `busy_permille`, `capacity`, `queue_delay_ms` | the upload meter |
//! | `children`, `seed_budget`, `seed_window`, `seed_copies`, `seed_first_wave` | the push scheduler |
//! | `frames`, `lag_p50_ms`, `lag_p99_ms`, `lag_max_ms` | at a viewer, the frames read in that second and their lag |
//! | `rtt_mean_ms` | the swarm's channels' round trips |
//! | `cpu_ms` | the driver's CPU time so far, `null` when it can't tell |
//! | `transfers` | [`TransferSample`], peers by index |
//! | `links` | every link: `peer` (its index, `null` for the relay), `kind`, `dialed` |
//! | `tick_late_ms` | how late the timer of this line fired |
//! | `write_late_ms` | at the seed, the latest of that second's writes past its time; `null` without writes |
//! | `wakeups` | passes of the node's loop since the last line (`null` on the first): it follows traffic, so it shows load only next to a native node's |
//!
//! `tick_late_ms` and `write_late_ms` show a throttled timer or an
//! overloaded host.

use std::cell::{Cell, RefCell};
use std::future::Future;

use strims_node_core::{
    HostId, LinkKind, Node, NodeMetrics, PeerInfo, ReadError, SwarmId, SwarmKey, WriterOptions,
};
use tokio::sync::mpsc;

use crate::shape::LiveShape;
use crate::stream::{FRAME_LEN, FrameParser, Parsed, encode_frame};
use crate::topology::{index_of, swarm_key};
use crate::transfer::TransferSample;

/// The frame payload seed shared by every node.
const RUN_SEED: u64 = 1;

/// What [`run`] needs from its driver.
pub trait Runtime {
    /// The wall clock in Unix milliseconds.
    fn now_ms(&self) -> u64;
    /// Resolves once the wall clock reaches `at_ms`.
    fn sleep_until_ms(&self, at_ms: u64) -> impl Future<Output = ()>;
    /// The driver's CPU time so far in milliseconds, if it can tell.
    fn cpu_ms(&self) -> Option<u64>;
    /// Writes one line of output.
    fn emit(&self, line: String);
    /// Reports something that went wrong; the node runs on.
    fn error(&self, message: String);
}

/// What one node does in the run.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub index: usize,
    /// The testbed's nodes, which peers' profile keys are matched against.
    pub nodes: usize,
    /// When the seed starts writing, in Unix milliseconds.
    pub start_ms: u64,
    pub secs: u64,
    /// The stream rate in bytes per second.
    pub rate: u64,
}

/// Runs the node's part (see the [module docs](self)), taking the node's
/// peers from `peers` as they're added.
pub async fn run(
    node: &Node,
    mut peers: mpsc::UnboundedReceiver<PeerInfo>,
    s: Settings,
    rt: &impl Runtime,
) -> Result<(), String> {
    let shape = LiveShape::testbed(s.rate);
    let key = swarm_key();
    let seed = s.index == 0;
    let (swarm, writer) = if seed {
        let (swarm, _, writer) = node
            .create_swarm(WriterOptions {
                swarm_options: shape.options.clone(),
                key: key.clone(),
            })
            .await
            .map_err(|e| format!("create swarm: {e}"))?;
        (swarm, Some(writer))
    } else {
        let swarm = node
            .add_swarm(SwarmId::new(&key.public), shape.options.clone())
            .await
            .map_err(|e| format!("add swarm: {e}"))?;
        (swarm, None)
    };

    let ids = RefCell::new(Vec::<(HostId, usize)>::new());
    let accept = async {
        while let Some(p) = peers.recv().await {
            let Some(i) = index_of(&p.profile_key, s.nodes) else {
                continue;
            };
            ids.borrow_mut().push((p.host_id, i));
            if let Err(e) = node.run_swarm(p.host_id, swarm, 1, 1).await {
                rt.error(format!("run swarm: {e}"));
            }
        }
    };

    let end_ms = s.start_ms + s.secs * 1000;
    let lags = RefCell::new(Vec::<u64>::new());
    let write_late = Cell::new(None::<u64>);
    let write = async {
        let Some(mut writer) = writer else {
            return;
        };
        let fpw = shape.frames_per_write as u64;
        let step = shape.write_interval.as_millis() as u64;
        let mut buf = vec![0u8; shape.frames_per_write * FRAME_LEN];
        for k in 0.. {
            let at = s.start_ms + k * step;
            if at >= end_ms {
                break;
            }
            rt.sleep_until_ms(at).await;
            let t = rt.now_ms();
            write_late.set(Some(
                write_late.get().unwrap_or(0).max(t.saturating_sub(at)),
            ));
            for (j, f) in buf.chunks_mut(FRAME_LEN).enumerate() {
                encode_frame(RUN_SEED, 0, k * fpw + j as u64, t, f);
            }
            if let Err(e) = writer.write(&buf).await {
                rt.error(format!("write: {e}"));
                return;
            }
        }
        // Dropping the writer would close the swarm.
        std::future::pending::<()>().await;
    };
    let read = async {
        if seed {
            return;
        }
        let mut reader = match node.open_reader(swarm).await {
            Ok(r) => r,
            Err(e) => {
                rt.error(format!("open reader: {e}"));
                return;
            }
        };
        let mut buf = vec![0u8; 64 * 1024];
        let mut parser = FrameParser::default();
        loop {
            match reader.read(&mut buf).await {
                Ok(n) => {
                    let now = rt.now_ms();
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
                    rt.error(format!("read: {e}"));
                    return;
                }
            }
        }
    };
    let sample = async {
        let mut next = s.start_ms.saturating_sub(5000);
        let mut loops = None;
        loop {
            rt.sleep_until_ms(next).await;
            let t = rt.now_ms();
            let tick_late = t.saturating_sub(next);
            next += 1000;
            let Ok(m) = node.metrics().await else {
                continue;
            };
            let mut lag = std::mem::take(&mut *lags.borrow_mut());
            lag.sort_unstable();
            let wakeups = loops.map(|l| m.driver.loop_iterations.saturating_sub(l));
            loops = Some(m.driver.loop_iterations);
            let line = Line {
                t_ms: t,
                s: &s,
                seed,
                swarm,
                m: &m,
                lag: &lag,
                cpu_ms: rt.cpu_ms(),
                ids: &ids.borrow(),
                tick_late,
                write_late: write_late.take(),
                wakeups,
            };
            rt.emit(line.json());
            if t >= end_ms + 5000 {
                return;
            }
        }
    };
    tokio::select! {
        biased;
        () = sample => {}
        () = async { tokio::join!(accept, write, read); } => {}
    }
    Ok(())
}

/// One second's line.
struct Line<'a> {
    t_ms: u64,
    s: &'a Settings,
    seed: bool,
    swarm: SwarmKey,
    m: &'a NodeMetrics,
    /// The lags read in the second, sorted.
    lag: &'a [u64],
    cpu_ms: Option<u64>,
    ids: &'a [(HostId, usize)],
    tick_late: u64,
    write_late: Option<u64>,
    wakeups: Option<u64>,
}

impl Line<'_> {
    fn json(&self) -> String {
        let m = self.m;
        let index = |id: &HostId| self.ids.iter().find(|(x, _)| x == id).map(|&(_, i)| i);
        let p = m
            .runner
            .swarms
            .iter()
            .find(|s| s.swarm == self.swarm)
            .and_then(|s| s.push.clone())
            .unwrap_or_default();
        let rtts: Vec<String> = m
            .runner
            .peers
            .iter()
            .flat_map(|p| &p.channels)
            .filter(|c| c.swarm == self.swarm)
            .filter_map(|c| {
                c.stats
                    .map(|s| s.scheduler.rtt_mean.as_millis().to_string())
            })
            .collect();
        let transfers =
            TransferSample::of(m, self.swarm, index).map_or("null".to_owned(), |x| x.json());
        let links: Vec<String> = m
            .links
            .iter()
            .filter_map(|l| {
                let peer = index(&l.host_id?);
                Some(format!(
                    "{{\"peer\":{},\"kind\":\"{}\",\"dialed\":{}}}",
                    opt(peer),
                    kind_name(l.kind),
                    l.dialed
                ))
            })
            .collect();
        let lag = self.lag;
        format!(
            "{{\"t_ms\":{},\"rel_ms\":{},\"index\":{},\"seed\":{},\"bytes_written\":{},\"upload_rate\":{},\"busy_permille\":{},\"capacity\":{},\"queue_delay_ms\":{},\"children\":{},\"seed_budget\":{},\"seed_window\":{},\"seed_copies\":{},\"seed_first_wave\":{},\"frames\":{},\"lag_p50_ms\":{},\"lag_p99_ms\":{},\"lag_max_ms\":{},\"rtt_mean_ms\":[{}],\"cpu_ms\":{},\"transfers\":{transfers},\"links\":[{}],\"tick_late_ms\":{},\"write_late_ms\":{},\"wakeups\":{}}}",
            self.t_ms,
            self.t_ms as i64 - self.s.start_ms as i64,
            self.s.index,
            self.seed,
            m.driver.bytes_written,
            p.upload_rate,
            p.upload_busy_permille,
            p.upload_capacity,
            opt(p.upload_queue_delay.map(|d| d.as_millis())),
            p.children,
            p.seed_budget,
            p.seed_window,
            p.seed_copies,
            p.seed_first_wave,
            lag.len(),
            pct(lag, 0.5),
            pct(lag, 0.99),
            lag.last().copied().unwrap_or(0),
            rtts.join(","),
            opt(self.cpu_ms),
            links.join(","),
            self.tick_late,
            opt(self.write_late),
            opt(self.wakeups),
        )
    }
}

/// `v`, or `null`.
fn opt(v: Option<impl ToString>) -> String {
    v.map_or("null".to_owned(), |v| v.to_string())
}

/// Percentile `q` of the sorted `v`, 0 when empty.
fn pct(v: &[u64], q: f64) -> u64 {
    v.get(((v.len().max(1) - 1) as f64 * q).round() as usize)
        .copied()
        .unwrap_or(0)
}

/// The control protocol's and the line's name for a link kind.
pub fn kind_name(kind: LinkKind) -> &'static str {
    match kind {
        LinkKind::Tcp => "tcp",
        LinkKind::Ws => "ws",
        LinkKind::WebRtc => "webrtc",
        LinkKind::Other => "other",
    }
}
