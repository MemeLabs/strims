// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The testbed node's swarm, on a started node of either driver.
//!
//! [`run`] streams (node 0) or reads the testbed's swarm. It runs the swarm
//! with every peer that has a testbed index, whoever made the link; the
//! relay has none. The seed writes from [`Settings::start_ms`] for
//! [`Settings::secs`], and a viewer measures the lag of what it reads
//! against the wall clock when it reads it: the testbed's containers share
//! the host's clock. It ends 5 s after the stream does.
//!
//! The stream ([`Stream`]) is the self-checking frames of [`LiveShape::testbed`],
//! each stamped with the wall clock as it is written, or an FLV file's
//! media in the unit format ([`media`]), each access unit
//! with its publish time.
//!
//! Every second from 5 s before the start, the node emits one JSON line
//! ([`Runtime::emit`]); a second it missed has none:
//!
//! | field | |
//! |---|---|
//! | `t_ms`, `rel_ms` | the wall clock, and ms since the start |
//! | `index`, `seed` | the node, and whether it seeds |
//! | `bytes_written` | bytes the driver wrote to its links |
//! | `upload_rate`, `busy_permille`, `capacity`, `queue_delay_ms` | the upload meter |
//! | `path_delay_ms`, `path_queued_ms`, `path_latest_ms` | the queueing delay from vnic's path measure ([`strims_base::path::node_delays`]): the links' excess round trips, and their queued bytes over their delivery rates; and their latest round trips' excess ([`strims_base::path::latest_excess`]) |
//! | `children`, `seed_budget`, `seed_window`, `seed_copies`, `seed_first_wave` | the push scheduler |
//! | `frames`, `lag_p50_ms`, `lag_p99_ms`, `lag_max_ms` | at a viewer, the frames read in that second and their lag; of a media stream, the unit records read whole and their lag from their publish times |
//! | `rtt_mean_ms` | the swarm's channels' round trips |
//! | `cpu_ms` | the driver's CPU time so far, `null` when it can't tell |
//! | `transfers` | [`TransferSample`], peers by index |
//! | `links` | every link: `peer` (its index, `null` for the relay), `kind`, `dialed`, and its path measure as of the latest echo (`null` before one): `rtt_ms`, `base_ms`, `recent_ms`, `in_flight`, `rate`, `queued`, `written_since`, `age_ms` |
//! | `tick_late_ms` | how late the timer of this line fired |
//! | `write_late_ms` | at the seed, the latest of that second's writes past its time; `null` without writes |
//! | `wakeups` | passes of the node's loop since the last line (`null` on the first): it follows traffic, so it shows load only next to a native node's |
//! | `media` | of a media stream, `null` otherwise: at the seed, [`SeedStats`]: the unit writer's `records`, `units`, `gops`, `bytes` and `padding`, `early_signs` and `dropped` units; at a viewer, [`ViewerStats`]: `units` and `keyframes` read whole, Data `messages` and their `bytes`, `first_message_ms`, the egress reader's `discontinuities`, `corrupt_frames`, `losses`, `records` and `rewinds`/`refused_rewinds`/`overtaken_rewinds`, and the join: `first_read_ms`, the first rewind's `rewind_start_ms`, `rewind_end_ms`, outcome (`granted`, `refused`, `timed_out`, `overtaken`) and bytes back from the reader's position, `rewind_back`. Times are Unix ms. |
//!
//! `tick_late_ms` and `write_late_ms` show a throttled timer or an
//! overloaded host.

use std::cell::{Cell, RefCell};
use std::future::Future;

use strims_base::path;
use strims_base::time::{Duration, Time};
use strims_node_core::{
    HostId, LinkKind, Node, NodeMetrics, PeerInfo, ReadError, SwarmId, SwarmKey, SwarmReader,
    SwarmWriter, WriterOptions,
};
use tokio::sync::mpsc;

use crate::media::{self, FlvFile, SeedStats, TagKind, ViewerStats};
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
    /// Takes a viewer's Data message of a media stream (see
    /// [`media::DataMessages`]); a player plays them.
    fn media(&self, data: &[u8], segment_end: bool, discontinuity: bool) {
        let _ = (data, segment_end, discontinuity);
    }
}

/// What the seed streams.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stream {
    /// The self-checking frames, at [`Settings::rate`].
    #[default]
    Frames,
    /// An FLV file's media in the unit format, in a swarm of
    /// `chunks_per_signature` chunks per integrity unit.
    Units { chunks_per_signature: usize },
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
    /// The chunk size and munro in bytes ([`LiveShape::with_chunks`]);
    /// `None` keeps [`LiveShape::testbed`]'s. Frames only.
    pub chunks: Option<(usize, usize)>,
    pub stream: Stream,
}

/// Runs the node's part (see the [module docs](self)), taking the node's
/// peers from `peers` as they're added. The seed of a media stream writes
/// `file`.
pub async fn run<R: Runtime + Clone + 'static>(
    node: &Node,
    mut peers: mpsc::UnboundedReceiver<PeerInfo>,
    s: Settings,
    file: Option<&FlvFile>,
    rt: &R,
) -> Result<(), String> {
    let seed = s.index == 0;
    let mut shape = LiveShape::testbed(s.rate);
    let (options, signing) = match s.stream {
        Stream::Frames => {
            if let Some((chunk, munro)) = s.chunks {
                shape = shape.with_chunks(chunk, munro)?;
            }
            (shape.options.clone(), None)
        }
        Stream::Units { .. } if s.chunks.is_some() => {
            return Err("a media stream takes ingest's chunks".to_owned());
        }
        Stream::Units {
            chunks_per_signature: cps,
        } if seed => {
            let file = file.ok_or("the seed of a media stream needs its file")?;
            let signing = media::signing_of(
                file.metadata().as_ref(),
                file.has(TagKind::Video),
                file.has(TagKind::Audio),
            );
            if signing.chunks_per_signature != cps {
                return Err(format!(
                    "the file's stream signs {} chunks at a time, not {cps}",
                    signing.chunks_per_signature
                ));
            }
            (media::seed_options(cps)?, Some(signing))
        }
        Stream::Units {
            chunks_per_signature: cps,
        } => (media::viewer_options(cps)?, None),
    };
    let key = swarm_key();
    let (swarm, writer) = if seed {
        let (swarm, _, writer) = node
            .create_swarm(WriterOptions {
                swarm_options: options.clone(),
                key: key.clone(),
            })
            .await
            .map_err(|e| format!("create swarm: {e}"))?;
        (swarm, Some(writer))
    } else {
        let swarm = node
            .add_swarm(SwarmId::new(&key.public), options.clone())
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
    let seed_stats = Cell::new(SeedStats::default());
    let viewer_stats = RefCell::new(ViewerStats::default());
    let write = async {
        let Some(mut writer) = writer else {
            return;
        };
        let written = match (signing, file) {
            (Some(signing), Some(file)) => {
                media::write(
                    &mut writer,
                    options.chunk_size,
                    signing,
                    file,
                    s.start_ms,
                    end_ms,
                    rt,
                    &write_late,
                    &seed_stats,
                )
                .await
            }
            _ => write_frames(&mut writer, &shape, s.start_ms, end_ms, rt, &write_late).await,
        };
        if let Err(e) = written {
            rt.error(e);
            return;
        }
        // Dropping the writer would close the swarm.
        std::future::pending::<()>().await;
    };
    let read = async {
        if seed {
            return;
        }
        let reader = match node.open_reader(swarm).await {
            Ok(r) => r,
            Err(e) => {
                rt.error(format!("open reader: {e}"));
                return;
            }
        };
        let read = match s.stream {
            Stream::Frames => read_frames(reader, rt, &lags).await,
            Stream::Units { .. } => {
                // The join's spare, opened with the reader as egress opens it.
                let spare = match node.open_reader(swarm).await {
                    Ok(r) => Some(r),
                    Err(e) => {
                        rt.error(format!("open spare reader: {e}"));
                        None
                    }
                };
                media::read(reader, spare, options.chunk_size, rt, &lags, &viewer_stats).await
            }
        };
        if let Err(e) = read {
            rt.error(e);
        }
    };
    let sample = async {
        let mut next = s.start_ms.saturating_sub(5000);
        let mut loops = None;
        loop {
            rt.sleep_until_ms(next).await;
            let t = rt.now_ms();
            let tick_late = t.saturating_sub(next);
            // A node that started late, or stalled, skips the seconds it
            // missed.
            while next <= t {
                next += 1000;
            }
            let Ok(m) = node.metrics().await else {
                continue;
            };
            let mut lag = std::mem::take(&mut *lags.borrow_mut());
            lag.sort_unstable();
            let wakeups = loops.map(|l| m.driver.loop_iterations.saturating_sub(l));
            loops = Some(m.driver.loop_iterations);
            let media = match s.stream {
                Stream::Frames => None,
                Stream::Units { .. } if seed => Some(seed_stats.get().json()),
                Stream::Units { .. } => Some(viewer_stats.borrow().json()),
            };
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
                media,
                now: Time::from_unix_millis(i64::try_from(t).unwrap_or(i64::MAX)),
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

/// Writes the self-checking frames of `shape` from `start_ms` until
/// `end_ms`, the latest write past its time in `late`.
async fn write_frames(
    writer: &mut SwarmWriter,
    shape: &LiveShape,
    start_ms: u64,
    end_ms: u64,
    rt: &impl Runtime,
    late: &Cell<Option<u64>>,
) -> Result<(), String> {
    let fpw = shape.frames_per_write as u64;
    let step = shape.write_interval.as_millis() as u64;
    let mut buf = vec![0u8; shape.frames_per_write * FRAME_LEN];
    for k in 0.. {
        let at = start_ms + k * step;
        if at >= end_ms {
            break;
        }
        rt.sleep_until_ms(at).await;
        let t = rt.now_ms();
        late.set(Some(late.get().unwrap_or(0).max(t.saturating_sub(at))));
        for (j, f) in buf.chunks_mut(FRAME_LEN).enumerate() {
            encode_frame(RUN_SEED, 0, k * fpw + j as u64, t, f);
        }
        writer
            .write(&buf)
            .await
            .map_err(|e| format!("write: {e}"))?;
    }
    Ok(())
}

/// Reads the self-checking frames, each one's lag into `lags`.
async fn read_frames(
    mut reader: SwarmReader,
    rt: &impl Runtime,
    lags: &RefCell<Vec<u64>>,
) -> Result<(), String> {
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
            Err(e) => return Err(format!("read: {e}")),
        }
    }
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
    /// The media stream's stats, as JSON.
    media: Option<String>,
    /// When the metrics were read.
    now: Time,
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
        let ms = |d: Duration| d.as_millis();
        let links: Vec<String> = m
            .links
            .iter()
            .filter_map(|l| {
                let peer = index(&l.host_id?);
                let r = l.path.and_then(|p| p.reading);
                Some(format!(
                    "{{\"peer\":{},\"kind\":\"{}\",\"dialed\":{},\"rtt_ms\":{},\"base_ms\":{},\"recent_ms\":{},\"in_flight\":{},\"rate\":{},\"queued\":{},\"written_since\":{},\"age_ms\":{}}}",
                    opt(peer),
                    kind_name(l.kind),
                    l.dialed,
                    opt(r.map(|r| ms(r.rtt))),
                    opt(r.and_then(|r| r.base).map(ms)),
                    opt(r.and_then(|r| r.recent_min).map(ms)),
                    opt(r.map(|r| r.in_flight)),
                    opt(r.and_then(|r| r.rate)),
                    opt(r.and_then(|r| r.queued)),
                    opt(r.map(|r| r.written_since)),
                    opt(r.map(|r| ms(r.age(self.now)))),
                ))
            })
            .collect();
        let (path_delay, path_queued) =
            path::node_delays(m.links.iter().filter_map(|l| l.path?.reading));
        let path_latest = path::latest_excess(
            m.links.iter().filter_map(|l| l.path?.reading),
            self.now,
            &mut Vec::new(),
        );
        let lag = self.lag;
        format!(
            "{{\"t_ms\":{},\"rel_ms\":{},\"index\":{},\"seed\":{},\"bytes_written\":{},\"upload_rate\":{},\"busy_permille\":{},\"capacity\":{},\"queue_delay_ms\":{},\"path_delay_ms\":{},\"path_queued_ms\":{},\"path_latest_ms\":{},\"children\":{},\"seed_budget\":{},\"seed_window\":{},\"seed_copies\":{},\"seed_first_wave\":{},\"frames\":{},\"lag_p50_ms\":{},\"lag_p99_ms\":{},\"lag_max_ms\":{},\"rtt_mean_ms\":[{}],\"cpu_ms\":{},\"transfers\":{transfers},\"links\":[{}],\"tick_late_ms\":{},\"write_late_ms\":{},\"wakeups\":{},\"media\":{}}}",
            self.t_ms,
            self.t_ms as i64 - self.s.start_ms as i64,
            self.s.index,
            self.seed,
            m.driver.bytes_written,
            p.upload_rate,
            p.upload_busy_permille,
            p.upload_capacity,
            opt(p.upload_queue_delay.map(|d| d.as_millis())),
            opt(path_delay.map(ms)),
            opt(path_queued.map(ms)),
            opt(path_latest.map(ms)),
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
            self.media.as_deref().unwrap_or("null"),
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
