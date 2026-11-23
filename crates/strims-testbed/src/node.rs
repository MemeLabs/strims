// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The testbed node's swarm, on a started node of either driver.
//!
//! [`run`] streams (node 0) or reads the testbed's swarm, which it runs
//! with its peers through the transfers' announce and answer as a
//! deployed node does, but without the network membership run that
//! precedes them on a deployed node ([`crate::peers`]). Its lines name
//! each peer by its testbed index; the
//! relay has none. The seed writes from [`Settings::start_ms`] for
//! [`Settings::secs`], and a viewer measures the lag of what it reads
//! against the wall clock when it reads it: the testbed's containers share
//! the host's clock. It ends 5 s after the stream does.
//!
//! The stream ([`Stream`]) is the self-checking frames of [`LiveShape::testbed`],
//! each stamped with its due time (the start plus its place times the
//! write interval: a late write lags as much), or an FLV file's
//! media in the unit format ([`media`]), each access unit
//! with its publish time.
//!
//! A viewer joins at the live edge, as the app's player joins a running
//! stream: egress adds its swarm in low latency delivery, the default a
//! URI's options leave, and rewinds to its GOP's start. A media viewer
//! takes egress's options and rewinds as egress does ([`media::read`]). A
//! viewer of the frames that arrives into the running stream
//! ([`Settings::arrives`]) reads in low latency delivery, from the newest
//! chunk its peers have, as strims-sim's arriving viewers do. One there
//! from the start keeps the frames' mandatory delivery, which reads from
//! the stream's first chunk its peers still hold: a late viewer would read
//! that backlog, up to a window, in its lag.
//!
//! Every second from 5 s before the start, the node writes one JSON line
//! ([`Runtime::line`]); a second it missed has none:
//!
//! | field | |
//! |---|---|
//! | `t_ms`, `rel_ms` | the wall clock, and ms since the start |
//! | `index`, `seed` | the node, and whether it seeds |
//! | `frames`, `lag_p50_ms`, `lag_p99_ms`, `lag_max_ms` | at a viewer, the frames read in that second and their lag; of a media stream, the unit records read whole and their lag from their publish times, which no record holds |
//! | `cpu_ms` | the driver's CPU time so far, `null` when it can't tell |
//! | `read_ms` | when the node's metrics were read, which `transfers`, `links` and the records are of; the meter sample's `t_ms` is its own |
//! | `transfers` | [`TransferSample`], peers by index |
//! | `links` | every link: `peer` (its index, `null` for the relay), `kind`, `dialed`, and its path measure as of the latest echo (`null` before one): `rtt_ms`, `base_ms`, `recent_ms`, `in_flight`, `rate`, `queued`, `written_since`, `age_ms` |
//! | `tick_late_ms` | how late the timer of this line fired |
//! | `write_late_ms` | at the seed, the latest of that second's writes past its time; `null` without writes |
//! | `wakeups` | passes of the node's loop since the last line (`null` on the first): it follows traffic, so it shows load only next to a native node's |
//! | `rec` | the line's records as strims-sim's tables take them (`Records`): the meter sample, the seed's state, the stats bins and parent changes since the last line, and every frame read since; with the node's scheduler logs on (its driver's `NodeConfig::forecast_log` and `request_log`: testbed_node's and testbed_browser's `--forecast` and `--spec`), what they logged since ([`record::logs`]); with its timer log (`NodeConfig::timer_log`, `--timer-log`), the timers that fired since: its node task's, the line's own, the frame writer's and a browser's host pacer's ([`record::TimerLog`]); with its link log (`NodeConfig::link_log`, `--link-log`), its links' writes, reads and transport readings since ([`record::links`]) |
//! | `media` | of a media stream, `null` otherwise: at the seed, [`SeedStats`]: the unit writer's `records`, `units`, `gops`, `bytes` and `padding`, `early_signs` and `dropped` units; at a viewer, [`ViewerStats`]: `units` and `keyframes` read whole, the `messages` that took records to the player (Data messages, or the batches of a browser player's feed) and their records' `bytes`, `first_message_ms`, the egress reader's `discontinuities`, `corrupt_frames`, `losses`, `records` and `rewinds`/`refused_rewinds`/`overtaken_rewinds`, and the join: `first_read_ms`, the first rewind's `rewind_start_ms`, `rewind_end_ms`, outcome (`granted`, `refused`, `timed_out`, `overtaken`) and bytes back from the reader's position, `rewind_back`. Times are Unix ms. |
//!
//! `tick_late_ms` and `write_late_ms` show a throttled timer or an
//! overloaded host.
//!
//! A line holds what its node had about its second. Its meter's readings
//! are asked for at the second, as strims-sim's controller asks a host
//! for its metrics, and answered after the node's next pass
//! ([`Node::meter_metrics`], cheap enough to answer in the pass that takes
//! the second's push period's writes); its meter sample's `t_ms` is when
//! they were. The rest of its metrics the node takes in the pass after,
//! and its logs' records up to then. The node's thread only takes them
//! ([`LineParts`]), once what that pass handed the links has gone, and
//! hands them to its driver ([`Runtime::line`]), whose [`LineWriter`]
//! writes the line elsewhere: a native node's on a thread of its own, a
//! browser's in a worker of its own. A deployed node writes no lines, so
//! the line keeps out of the node's way: the whole metrics taken in the
//! period's own pass would hold its links' first writes behind them, and a
//! line of the scheduler's and link logs written on the node's thread
//! would hold it for a millisecond or more in the period after them.

use std::cell::{Cell, RefCell};
use std::fmt::Write as _;
use std::future::Future;

use strims_base::stats::WaitHistogram;
use strims_base::time::{Duration, Time};
use strims_node_core::app::Spawn;
use strims_node_core::{
    DecisionInfo, HostId, LinkKind, Node, NodeMetrics, ParentLogEntry, ReadError, SwarmId,
    SwarmKey, SwarmReader, SwarmWriter, TimerRecord, TimerSource, WriterOptions,
};
use strims_ppspp::{DeliveryMode, SwarmOptions, TapKind};
use strims_rpc::Timer;
use strims_transfer::SwarmRead;

use crate::media::{self, FlvFile, SeedStats, TagKind, ViewerStats};
use crate::peers::Peers;
use crate::record::links::{LinkLog, LinkOpen, LinkRowsMaker, LinkTake};
use crate::record::logs::{Field, Logs};
use crate::record::wire::Wire;
use crate::record::{self, DecisionLog, NODE_BIN_COUNTERS, ParentLog, TimerContext, TimerLog};
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
    /// The wall clock at its finest precision, for the timer log alone
    /// ([`TimerLog`]). By default [`Runtime::now_ms`].
    fn precise_now(&self) -> Time {
        Time::from_unix_millis(i64::try_from(self.now_ms()).unwrap_or(i64::MAX))
    }
    /// Resolves once the wall clock reaches `at_ms`.
    fn sleep_until_ms(&self, at_ms: u64) -> impl Future<Output = ()>;
    /// Resolves once the work the driver queued meanwhile has run, between
    /// the steps of a line's ask for the node's metrics, its take and its
    /// hand-over (see the [module docs](self)). By default at once.
    fn yield_now(&self) -> impl Future<Output = ()> {
        std::future::ready(())
    }
    /// The driver's CPU time so far in milliseconds, if it can tell.
    fn cpu_ms(&self) -> Option<u64>;
    /// Takes one line's parts, which a [`LineWriter`] the driver keeps
    /// makes its line of output, in the order they come, off the node's
    /// thread where the driver has another; what hands them over may
    /// yield ([`Runtime::yield_now`]) between its steps.
    fn line(&self, parts: LineParts) -> impl Future<Output = ()>;
    /// Reports something that went wrong; the node runs on.
    fn error(&self, message: String);
    /// Takes a viewer's Data message of a media stream (see
    /// [`media::DataMessages`]); a player plays them.
    fn media(&self, data: &[u8], segment_end: bool, discontinuity: bool) {
        let _ = (data, segment_end, discontinuity);
    }
    /// Plays a viewer's media stream until it closes. By default its
    /// pieces go as the Egress RPC's Data messages to
    /// [`media`](Self::media) ([`media::send_messages`]).
    fn play<S: SwarmRead>(
        &self,
        viewer: media::Viewer<'_, S, Self>,
    ) -> impl Future<Output = Result<(), String>>
    where
        Self: Clone + Sized + 'static,
    {
        media::send_messages(viewer, self)
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
    /// Whether a viewer arrives into the running stream, and so reads from
    /// the live edge (see the [module docs](self)): `--arrive`, which
    /// run.sh gives each viewer of `JOINS`, whatever its time, and every
    /// viewer of a `JOIN_AT` after the start, as strims-sim's twin of
    /// those joins has its viewers arrive.
    pub arrives: bool,
    /// The stream rate in bytes per second.
    pub rate: u64,
    /// The chunk size and munro in bytes ([`LiveShape::with_chunks`]);
    /// `None` keeps [`LiveShape::testbed`]'s. Frames only.
    pub chunks: Option<(usize, usize)>,
    pub stream: Stream,
    /// Whether the node runs the timer probe ([`probe`]) while the stream
    /// runs, its timers in the node's timer log: testbed_node's and
    /// testbed_browser's `--timer-probe`, which needs `--timer-log`.
    pub probe: bool,
}

/// The logs a node's config hands its records to, which its lines take,
/// each `None` while off.
#[derive(Clone, Copy, Debug, Default)]
pub struct NodeLogs<'a> {
    /// Its parent changes (`NodeConfig::parent_log`).
    pub parents: Option<&'a ParentLog>,
    /// Its tap's events (`NodeConfig::decision_log`).
    pub checks: Option<&'a DecisionLog>,
    /// Its timers (`NodeConfig::timer_log`), which the line's timer and
    /// the frame writer add theirs to.
    pub timers: Option<&'a TimerLog>,
    /// Its links' I/O (`NodeConfig::link_log`).
    pub links: Option<&'a LinkLog>,
}

/// Runs the node's part (see the [module docs](self)), serving the node's
/// peers through `peers`. The seed of a media stream writes `file`. `logs`
/// are the logs the node's config hands its records to.
pub async fn run<R: Runtime + Clone + 'static, T: Timer, S: Spawn>(
    node: &Node,
    mut peers: Peers<T, S>,
    s: Settings,
    file: Option<&FlvFile>,
    logs: NodeLogs<'_>,
    rt: &R,
) -> Result<(), String> {
    let NodeLogs {
        parents,
        checks,
        timers,
        links,
    } = logs;
    let seed = s.index == 0;
    let mut shape = LiveShape::testbed(s.rate);
    let (options, signing) = match s.stream {
        Stream::Frames => {
            if let Some((chunk, munro)) = s.chunks {
                shape = shape.with_chunks(chunk, munro)?;
            }
            (frames_options(&shape, seed, s.arrives), None)
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
    peers.publish(swarm, &SwarmId::new(&key.public));

    let ids = RefCell::new(Vec::<(HostId, usize)>::new());
    let accept = peers.serve(|p| {
        if let Some(i) = index_of(&p.profile_key, s.nodes) {
            ids.borrow_mut().push((p.host_id, i));
        }
    });

    let end_ms = s.start_ms + s.secs * 1000;
    let lags = RefCell::new(Vec::<u64>::new());
    let reads = RefCell::new(Vec::<[u64; 3]>::new());
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
            _ => {
                write_frames(
                    &mut writer,
                    &shape,
                    s.start_ms,
                    end_ms,
                    rt,
                    &write_late,
                    timers,
                )
                .await
            }
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
            Stream::Frames => read_frames(reader, rt, &lags, &reads).await,
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
        let origin = Time::from_unix_millis(i64::try_from(s.start_ms).unwrap_or(i64::MAX));
        loop {
            let armed = timers.map(|_| (rt.precise_now(), rt.now_ms()));
            rt.sleep_until_ms(next).await;
            let t = rt.now_ms();
            let tick_late = t.saturating_sub(next);
            if let (Some(log), Some(armed)) = (timers, armed) {
                log.push(fired(rt, TimerSource::Record, armed, next));
            }
            // A node that started late, or stalled, skips the seconds it
            // missed.
            while next <= t {
                next += 1000;
            }
            // The meter's readings, as strims-sim's controller takes a
            // host's: asked at the second, answered after the node's next
            // pass (a seed's takes the push period's writes; a viewer's
            // is its next input or timer). A cheap query: the full one's
            // collection would hold a seed's writes behind it.
            let Ok(meter) = node.meter_metrics().await else {
                continue;
            };
            let meter_ms = rt.now_ms();
            // The rest once the period's writes have gone: the node answers
            // after its next pass.
            let Ok(m) = node.metrics().await else {
                continue;
            };
            // The line's moment: its metrics' readings are of now, and it
            // takes its logs' records up to now.
            let read_ms = rt.now_ms();
            let mut lag = std::mem::take(&mut *lags.borrow_mut());
            lag.sort_unstable();
            let wakeups = loops.map(|l| m.driver.loop_iterations.saturating_sub(l));
            loops = Some(m.driver.loop_iterations);
            let media = match s.stream {
                Stream::Frames => None,
                Stream::Units { .. } if seed => Some(seed_stats.get().json()),
                Stream::Units { .. } => Some(viewer_stats.borrow().json()),
            };
            let link_take = links.map(LinkLog::take_raw);
            let taken_parents = parents.map(ParentLog::take).unwrap_or_default();
            let taken_checks = checks.map(DecisionLog::take);
            let taken_reads = std::mem::take(&mut *reads.borrow_mut());
            let taken_timers = timers.map(|log| {
                let (rows, lost) = log.take();
                (rows, lost, log.context())
            });
            let cpu_ms = rt.cpu_ms();
            let late = write_late.take();
            // The work the node queued by now goes first: what the pass
            // that answered the query handed its links.
            rt.yield_now().await;
            let moment = Moment {
                t_ms: t,
                meter_ms,
                meter: &meter,
                read_ms,
                s: &s,
                seed,
                swarm,
                m: &m,
                lag: &lag,
                cpu_ms,
                ids: ids.borrow().clone(),
                tick_late,
                write_late: late,
                wakeups,
                media,
                parents: taken_parents,
                checks: taken_checks,
                reads: taken_reads,
                timers: taken_timers,
                links: link_take,
                origin,
            };
            let parts = moment.parts();
            rt.yield_now().await;
            rt.line(parts).await;
            if t >= end_ms + 5000 {
                return;
            }
        }
    };
    let probing = async {
        if let (true, Some(log)) = (s.probe, timers) {
            probe(rt, log, s.start_ms, end_ms).await;
        }
    };
    tokio::select! {
        biased;
        () = sample => {}
        () = async { tokio::join!(accept, write, read, probing); } => {}
    }
    Ok(())
}

/// The probe's timers' delays in ms, which it arms in turn, one a
/// [`PROBE_PERIOD_MS`].
pub const PROBE_DELAYS_MS: [u64; 6] = [0, 1, 4, 10, 16, 50];

/// How often the probe arms a timer, in ms: past its longest delay, so
/// one fires before the next is armed.
pub const PROBE_PERIOD_MS: u64 = 60;

/// Arms the probe's timers from `from_ms` until `until_ms` (Unix ms, the
/// wall clock's): at the start of each [`PROBE_PERIOD_MS`] by that clock,
/// a timer of the next of [`PROBE_DELAYS_MS`] from the time then, each
/// kept in `log` as it fires ([`TimerSource::Probe`]). One that starts
/// late, or restarts, begins at the next period. Every side runs this
/// schedule: a testbed node on its runtime's timers, strims-sim's hosts on
/// the model's.
pub async fn probe(rt: &impl Runtime, log: &TimerLog, from_ms: u64, until_ms: u64) {
    let mut k = rt
        .now_ms()
        .saturating_sub(from_ms)
        .div_ceil(PROBE_PERIOD_MS);
    loop {
        let start = from_ms + k * PROBE_PERIOD_MS;
        if start >= until_ms {
            return;
        }
        rt.sleep_until_ms(start).await;
        let delay = PROBE_DELAYS_MS
            .get(usize::try_from(k).unwrap_or(0) % PROBE_DELAYS_MS.len())
            .copied()
            .unwrap_or(0);
        let now = rt.now_ms();
        let armed = (rt.precise_now(), now);
        rt.sleep_until_ms(now + delay).await;
        log.push(fired(rt, TimerSource::Probe, armed, now + delay));
        k += 1;
    }
}

/// The swarm options of the frames of `shape` at a node: the shape's, in
/// low latency delivery at a viewer that `arrives` (see the
/// [module docs](self)).
fn frames_options(shape: &LiveShape, seed: bool, arrives: bool) -> SwarmOptions {
    let mut options = shape.options.clone();
    if !seed && arrives {
        options.delivery_mode = DeliveryMode::LOW_LATENCY;
    }
    options
}

/// A timer of `source` that was armed for `deadline_ms` when the precise
/// clock and the ms clock read `armed`, and fired now.
fn fired(
    rt: &impl Runtime,
    source: TimerSource,
    armed: (Time, u64),
    deadline_ms: u64,
) -> TimerRecord {
    let ms = |t: u64| i64::try_from(t).unwrap_or(i64::MAX);
    TimerRecord {
        source,
        armed: armed.0,
        deadline: Time::from_unix_millis(ms(deadline_ms)),
        fired: rt.precise_now(),
        requested: Duration::from_millis(ms(deadline_ms) - ms(armed.1)),
    }
}

/// Writes the self-checking frames of `shape` from `start_ms` until
/// `end_ms`, each stamped with the time its write was due (strims-sim's
/// `Report::written_at`), so a viewer's lag counts a late write; the
/// latest write past its time in `late`, and with `timers` each write's
/// wait for its time.
async fn write_frames(
    writer: &mut SwarmWriter,
    shape: &LiveShape,
    start_ms: u64,
    end_ms: u64,
    rt: &impl Runtime,
    late: &Cell<Option<u64>>,
    timers: Option<&TimerLog>,
) -> Result<(), String> {
    let fpw = shape.frames_per_write as u64;
    let step = shape.write_interval.as_millis() as u64;
    let mut buf = vec![0u8; shape.frames_per_write * FRAME_LEN];
    for k in 0.. {
        let at = start_ms + k * step;
        if at >= end_ms {
            break;
        }
        let armed = timers.map(|_| (rt.precise_now(), rt.now_ms()));
        rt.sleep_until_ms(at).await;
        let t = rt.now_ms();
        late.set(Some(late.get().unwrap_or(0).max(t.saturating_sub(at))));
        if let (Some(log), Some(armed)) = (timers, armed) {
            log.push(fired(rt, TimerSource::Writer, armed, at));
        }
        for (j, f) in buf.chunks_mut(FRAME_LEN).enumerate() {
            encode_frame(RUN_SEED, 0, k * fpw + j as u64, at, f);
        }
        writer
            .write(&buf)
            .await
            .map_err(|e| format!("write: {e}"))?;
    }
    Ok(())
}

/// Reads the self-checking frames, each one's lag into `lags`, and its
/// sequence number, stamp and read time into `reads`.
async fn read_frames(
    mut reader: SwarmReader,
    rt: &impl Runtime,
    lags: &RefCell<Vec<u64>>,
    reads: &RefCell<Vec<[u64; 3]>>,
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
                        reads.borrow_mut().push([f.seq, f.time_ms, now]);
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

/// A line's records, as strims-sim's tables take them ([`record`]): the
/// meter sample of the line's metrics, its adaptive seed's state, the
/// stats bins and parent changes since the last line, and every frame read
/// since: `{"meter": MeterSample | null, "seed": SeedSample | null,
/// "bins": [[edge_ms, [counters], [push waits], [answer waits]], ...],
/// "parents": [[t_ms, stream, peer, why], ...], "parents_lost": n,
/// "reads": [[seq, stamp_ms, read_ms], ...]}`, times Unix ms, peers by
/// index (`null`: none, or one without), waits as
/// [`record::waits_json`]. The meter sample's `t_ms` is when the node's
/// meter was read, which its readings are of. With the node's decision
/// log ([`DecisionLog`]), its parent checks since the last line too,
/// `"checks": [[t_us, stream, action, best, best_lag_us, runner_up,
/// runner_up_lag_us, parent, parent_have_us, parent_push_us, unready,
/// subscriber, backed_off, stale, behind, votes], ...], "checks_lost": n`:
/// `t_us` in Unix µs, the action by its name (`ParentAction::name`), the
/// lags in µs (`null`: none). With the node's scheduler logs on
/// (`NodeConfig::forecast_log`, `request_log`), what they logged since
/// the last line too, `"logs": Logs` ([`record::logs`], times from
/// `origin`, the stream's start); a node with them off writes none. With
/// its timer log, the timers that fired since, `"timers": {...}`
/// ([`record::timers_json`], times in µs from `origin`); a node without
/// one writes none. With its link log, its links' I/O since, `"link_log":
/// {...}` ([`record::links::link_log_json`], times in µs from `origin`); a
/// node without one writes none.
///
/// A line's moment holds what the node had then, borrowed where its
/// metrics hold it; [`Moment::parts`] takes it, and a [`LineWriter`]
/// writes the line.
struct Moment<'a> {
    /// The line's second, as its timer fired.
    t_ms: u64,
    /// When the node's meter was read ([`Node::meter_metrics`]), and what
    /// was.
    meter_ms: u64,
    meter: &'a NodeMetrics,
    /// When the node's metrics were read.
    read_ms: u64,
    s: &'a Settings,
    seed: bool,
    swarm: SwarmKey,
    m: &'a NodeMetrics,
    /// The lags read in the second, sorted.
    lag: &'a [u64],
    cpu_ms: Option<u64>,
    ids: Vec<(HostId, usize)>,
    tick_late: u64,
    write_late: Option<u64>,
    wakeups: Option<u64>,
    /// The media stream's stats, as JSON.
    media: Option<String>,
    parents: (Vec<ParentLogEntry>, u64),
    checks: Option<(Vec<DecisionInfo>, u64)>,
    reads: Vec<[u64; 3]>,
    /// The timer log's take, and where its timers run.
    timers: Option<(Vec<TimerRecord>, u64, TimerContext)>,
    /// The link log's raw take.
    links: Option<LinkTake>,
    origin: Time,
}

/// A line's parts ([`Moment::parts`]): what the node had at the line's
/// moment, the line's head, its meter and seed samples written (a few
/// hundred bytes), and the rest as records, which a [`LineWriter`] writes
/// into the line. A browser's parts cross to the worker that writes its
/// lines as bytes ([`Wire`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineParts {
    /// The line up to its records: `{"t_ms":...,"rec":`.
    head: String,
    meter: String,
    seed: String,
    /// Each bin edge of the line's swarm: its time (Unix ms), counters
    /// ([`record::NODE_BIN_COUNTERS`]) and waits ([`record::bin_waits`]).
    bins: Vec<(i64, [u64; NODE_BIN_COUNTERS.len()], [WaitHistogram; 2])>,
    /// Each parent change of the line's swarm: when (Unix ms), the stream,
    /// its parent by index, and why.
    parents: Vec<(i64, u64, Option<usize>, String)>,
    parents_lost: u64,
    reads: Vec<[u64; 3]>,
    checks: Option<(Vec<Check>, u64)>,
    logs: Logs,
    timers: Option<(Vec<TimerRecord>, u64, TimerContext)>,
    links: Option<LinkTake>,
    /// The nodes the line names, by host.
    ids: Vec<(HostId, usize)>,
    origin: Time,
}

/// A parent check's row ([`Moment`]'s doc gives its columns), its peers by
/// index, its times in µs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Check {
    t_us: i64,
    stream: u64,
    action: String,
    best: (Option<usize>, Option<i64>),
    runner_up: (Option<usize>, Option<i64>),
    parent: Option<usize>,
    parent_lag: (Option<i64>, Option<i64>),
    /// unready, subscriber, backed_off, stale, behind, votes.
    counts: [u64; 6],
}

impl Moment<'_> {
    fn index(&self, id: &HostId) -> Option<usize> {
        self.ids.iter().find(|(x, _)| x == id).map(|&(_, i)| i)
    }

    /// The moment's parts: the head and the samples written, the rest
    /// taken as records.
    fn parts(self) -> LineParts {
        let index = |id: &HostId| self.index(id);
        let swarm = self.swarm;
        let m = self.m;
        let at = |ms: u64| Time::from_unix_millis(i64::try_from(ms).unwrap_or(i64::MAX));
        let (meter, seeds) = record::meter_sample(
            self.meter,
            |k| (k == swarm).then_some(0),
            std::time::Duration::from_millis(self.meter_ms),
            at(self.meter_ms),
        );
        let read = at(self.read_ms);
        let seed = seeds
            .iter()
            .find(|(s, _)| *s == 0)
            .map_or("null".to_owned(), |(_, x)| x.json());
        let bins = m
            .bins
            .iter()
            .filter_map(|b| {
                let sw = b.swarms.iter().find(|s| s.swarm == swarm)?;
                Some((
                    b.at.unix_millis(),
                    record::bin_counters(b, sw),
                    record::bin_waits(sw),
                ))
            })
            .collect();
        let (changes, parents_lost) = &self.parents;
        let parents = changes
            .iter()
            .filter_map(|e| match e {
                ParentLogEntry::Change(p) if p.swarm == swarm => Some((
                    p.at.unix_millis(),
                    u64::from(p.stream),
                    p.parent.as_ref().and_then(index),
                    p.why.name().to_owned(),
                )),
                _ => None,
            })
            .collect();
        let checks = self.checks.as_ref().map(|(checks, lost)| {
            let us = |d: Option<Duration>| d.map(|d| d.as_micros());
            let rows = checks
                .iter()
                .filter(|d| d.swarm == Some(swarm))
                .filter_map(|d| {
                    let TapKind::ParentChoice { stream, choice: c } = d.kind else {
                        return None;
                    };
                    Some(Check {
                        t_us: d.at.unix_nanos() / 1000,
                        stream: u64::from(stream),
                        action: c.action.name().to_owned(),
                        best: (d.peer.as_ref().and_then(index), us(c.best.map(|b| b.1))),
                        runner_up: (
                            d.runner_up.as_ref().and_then(index),
                            us(c.runner_up.map(|r| r.1)),
                        ),
                        parent: d.parent.as_ref().and_then(index),
                        parent_lag: (us(c.parent_have), us(c.parent_push)),
                        counts: [
                            c.unready,
                            c.subscriber,
                            c.backed_off,
                            c.stale,
                            c.behind,
                            c.votes,
                        ],
                    })
                })
                .collect();
            (rows, *lost)
        });
        let parents_lost = *parents_lost;
        let logs = Logs::of(m, swarm, index);
        let head = self.head(read);
        LineParts {
            head,
            meter: meter.map_or("null".to_owned(), |m| m.json()),
            seed,
            bins,
            parents,
            parents_lost,
            reads: self.reads,
            checks,
            logs,
            timers: self.timers,
            links: self.links,
            ids: self.ids,
            origin: self.origin,
        }
    }

    /// The line up to its records, its links' readings' ages at `read`.
    fn head(&self, read: Time) -> String {
        let m = self.m;
        let index = |id: &HostId| self.index(id);
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
                    opt(r.map(|r| ms(r.age(read)))),
                ))
            })
            .collect();
        let lag = self.lag;
        format!(
            "{{\"t_ms\":{},\"rel_ms\":{},\"index\":{},\"seed\":{},\"frames\":{},\"lag_p50_ms\":{},\"lag_p99_ms\":{},\"lag_max_ms\":{},\"cpu_ms\":{},\"read_ms\":{},\"transfers\":{transfers},\"links\":[{}],\"tick_late_ms\":{},\"write_late_ms\":{},\"wakeups\":{},\"media\":{},\"rec\":",
            self.t_ms,
            self.t_ms as i64 - self.s.start_ms as i64,
            self.s.index,
            self.seed,
            lag.len(),
            pct(lag, 0.5),
            pct(lag, 0.99),
            lag.last().copied().unwrap_or(0),
            opt(self.cpu_ms),
            self.read_ms,
            links.join(","),
            self.tick_late,
            opt(self.write_late),
            opt(self.wakeups),
            self.media.as_deref().unwrap_or("null"),
        )
    }
}

/// What writes a node's lines from their parts, given them in the order
/// they were taken: the link log's links between takes
/// ([`LinkRowsMaker`]), and the links the log named before the node's
/// peer events named their peers, with the lines they have waited.
#[derive(Debug, Default)]
pub struct LineWriter {
    links: LinkRowsMaker,
    unnamed: Vec<(LinkOpen, u32)>,
}

impl LineWriter {
    /// The line of `p` (see [`Moment`] for its records' fields).
    pub fn write(&mut self, p: LineParts) -> String {
        let LineParts {
            mut head,
            meter,
            seed,
            bins,
            parents,
            parents_lost,
            reads,
            checks,
            logs,
            timers,
            links,
            ids,
            origin,
        } = p;
        let index = |id: &HostId| ids.iter().find(|(x, _)| x == id).map(|&(_, i)| i);
        let out = &mut head;
        let _ = write!(out, "{{\"meter\":{meter},\"seed\":{seed},\"bins\":");
        record::list(out, &bins, |s, (at, counters, [p, a])| {
            let _ = write!(s, "[{at},[");
            for (i, c) in counters.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                let _ = write!(s, "{c}");
            }
            let _ = write!(s, "],{},{}]", record::waits_json(p), record::waits_json(a));
        });
        out.push_str(",\"parents\":");
        record::list(out, &parents, |s, (at, stream, parent, why)| {
            let _ = write!(s, "[{at},{stream},");
            record::put_opt(s, *parent);
            let _ = write!(s, ",\"{why}\"]");
        });
        let _ = write!(out, ",\"parents_lost\":{parents_lost},\"reads\":");
        record::list(out, &reads, |s, [q, t, r]| {
            let _ = write!(s, "[{q},{t},{r}]");
        });
        if let Some((checks, lost)) = &checks {
            out.push_str(",\"checks\":");
            record::list(out, checks, |s, c| {
                let _ = write!(
                    s,
                    "[{},{},\"{}\",{},{},{},{},{},{},{}",
                    c.t_us,
                    c.stream,
                    c.action,
                    opt(c.best.0),
                    opt(c.best.1),
                    opt(c.runner_up.0),
                    opt(c.runner_up.1),
                    opt(c.parent),
                    opt(c.parent_lag.0),
                    opt(c.parent_lag.1),
                );
                for n in c.counts {
                    let _ = write!(s, ",{n}");
                }
                s.push(']');
            });
            let _ = write!(out, ",\"checks_lost\":{lost}");
        }
        if !logs.is_empty() {
            out.push_str(",\"logs\":");
            Field::put(&logs, out, origin);
        }
        if let Some((rows, lost, context)) = &timers {
            out.push_str(",\"timers\":");
            record::put_timers(out, rows, *lost, *context, origin);
        }
        if let Some(take) = links {
            let mut rows = self.links.rows(take);
            let known = |l: &LinkOpen| ids.iter().any(|(id, _)| *id == l.peer);
            let waiting: Vec<_> = self
                .unnamed
                .drain(..)
                .chain(rows.links.drain(..).map(|l| (l, 0)))
                .collect();
            let (known, later): (Vec<_>, Vec<_>) = waiting.into_iter().partition(|(l, _)| known(l));
            rows.links = known.into_iter().map(|(l, _)| l).collect();
            // A peer the swarm never names (the relay) waits a minute.
            self.unnamed = later
                .into_iter()
                .filter(|&(_, w)| w < 60)
                .map(|(l, w)| (l, w + 1))
                .collect();
            out.push_str(",\"link_log\":");
            record::links::put_link_log(out, &rows, index, origin);
        }
        out.push_str("}}");
        head
    }
}

impl Wire for Check {
    fn to_wire(&self, out: &mut Vec<u8>) {
        (self.t_us, self.stream, self.action.clone()).to_wire(out);
        (self.best, self.runner_up, self.parent, self.parent_lag).to_wire(out);
        self.counts.to_wire(out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        let (t_us, stream, action) = Wire::from_wire(r)?;
        let (best, runner_up, parent, parent_lag) = Wire::from_wire(r)?;
        Some(Check {
            t_us,
            stream,
            action,
            best,
            runner_up,
            parent,
            parent_lag,
            counts: Wire::from_wire(r)?,
        })
    }
}

impl Wire for LineParts {
    fn to_wire(&self, out: &mut Vec<u8>) {
        let LineParts {
            head,
            meter,
            seed,
            bins,
            parents,
            parents_lost,
            reads,
            checks,
            logs,
            timers,
            links,
            ids,
            origin,
        } = self;
        head.to_wire(out);
        meter.to_wire(out);
        seed.to_wire(out);
        bins.to_wire(out);
        parents.to_wire(out);
        parents_lost.to_wire(out);
        reads.to_wire(out);
        checks.to_wire(out);
        logs.to_wire(out);
        timers.to_wire(out);
        links.to_wire(out);
        ids.to_wire(out);
        origin.to_wire(out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        Some(LineParts {
            head: Wire::from_wire(r)?,
            meter: Wire::from_wire(r)?,
            seed: Wire::from_wire(r)?,
            bins: Wire::from_wire(r)?,
            parents: Wire::from_wire(r)?,
            parents_lost: Wire::from_wire(r)?,
            reads: Wire::from_wire(r)?,
            checks: Wire::from_wire(r)?,
            logs: Wire::from_wire(r)?,
            timers: Wire::from_wire(r)?,
            links: Wire::from_wire(r)?,
            ids: Wire::from_wire(r)?,
            origin: Wire::from_wire(r)?,
        })
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]
    use super::*;
    use crate::shape::RATE;

    /// A viewer that arrives reads the frames from the live edge, one
    /// there from the start and the seed in the shape's mandatory
    /// delivery. Arriving doesn't depend on the time: run.sh's viewer of
    /// a `JOINS` entry of 0 s arrives, as its twin's does.
    #[test]
    fn an_arriving_viewer_reads_the_frames_from_the_live_edge() {
        let shape = LiveShape::testbed(RATE)
            .with_chunks(4096, 32 * 1024)
            .unwrap();
        assert_eq!(shape.options.delivery_mode, DeliveryMode::MANDATORY);
        let mode = |seed, arrives| frames_options(&shape, seed, arrives).delivery_mode;
        assert_eq!(mode(false, true), DeliveryMode::LOW_LATENCY);
        assert_eq!(mode(false, false), DeliveryMode::MANDATORY);
        assert_eq!(mode(true, false), DeliveryMode::MANDATORY);
        assert_eq!(mode(true, true), DeliveryMode::MANDATORY);
        let mut live = shape.options.clone();
        live.delivery_mode = DeliveryMode::LOW_LATENCY;
        assert_eq!(frames_options(&shape, false, true), live);
    }

    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering};

    use slotmap::KeyData;
    use strims_base::stats::WaitHistogram;
    use strims_node_core::{
        LinkEvent, LinkInfo, LinkState, NodeBin, ParentInfo, RunnerMetrics, SampleSource,
        StoreInfo, SwarmBin, SwarmInfo, WriteWait,
    };
    use strims_ppspp::{
        ParentAction, ParentChoice, ParentWhy, PeerKey, PushMode, PushStats, SwarmCounters, SwarmId,
    };

    use crate::record::logs::tests::every;

    /// The nodes a line's fixture names, by index.
    fn host(i: usize) -> HostId {
        HostId([i as u64 + 1, 0, 0, 0])
    }

    /// A swarm the line isn't of.
    fn other_swarm() -> SwarmKey {
        KeyData::from_ffi((1 << 32) | 7).into()
    }

    /// A hybrid swarm's info, an adaptive seed's, its readings offset by
    /// `n`.
    fn swarm_info(swarm: SwarmKey, n: u64) -> SwarmInfo {
        let peer: PeerKey = KeyData::from_ffi((1 << 32) | 3).into();
        SwarmInfo {
            swarm,
            id: SwarmId::new(&[1, 2, n as u8]),
            label: "x".into(),
            channels: 3,
            store: StoreInfo {
                ready: true,
                tail: strims_binmap::Bin(2),
                next: strims_binmap::Bin(6),
                window: 16,
                readable_seq: 2,
            },
            scheduler: None,
            push: Some(PushStats {
                mode: PushMode::Hybrid,
                parents: vec![Some(peer), None],
                upload_rate: 123_456 + n,
                upload_busy_permille: 321,
                upload_capacity: 2_000_000 + n,
                upload_queue_delay: Some(Duration::from_micros(4_700)),
                children: 5,
                accepted: 9,
                rejected: 2,
                pace_channels: 4,
                have_lazy_out: 1,
                have_lazy_in: 2,
                seed_window: 6,
                seed_budget: 4,
                seed_copies: 2,
                seed_first_wave: 3,
                seed_first_wave_capacity: 1_500_000,
                seed_budget_capacity: 1_900_000,
                seed_latest_floor_ms: Some(7),
                ..PushStats::default()
            }),
            parent_hosts: vec![Some(host(2)), None],
            seed_allocation: None,
            readers: 1,
            readers_waiting: 0,
            publishers: 1,
        }
    }

    /// A link to `peer`, without a path reading yet.
    fn link(peer: Option<HostId>, kind: LinkKind) -> LinkInfo {
        LinkInfo {
            host_id: peer,
            kind,
            metrics: Default::default(),
            dialed: true,
            mtu: 1200,
            transmit_pending: 0,
            input_buffered: 0,
            write_blocked: false,
            buffered_amount: 64,
            path: None,
        }
    }

    /// What a line's records hold: a snapshot with its swarm and another,
    /// both hybrid and seeded (a meter and a seed sample), links (one to
    /// no testbed node), two bins of the swarm, one holding the other's
    /// too and one the other's alone, every kind of the scheduler's logs'
    /// records, a parent change of each swarm and its cap's losses, frames
    /// read, a
    /// parent check, timers and a link log's raw take; and the nodes it
    /// names.
    struct Fixture {
        m: NodeMetrics,
        ids: Vec<(HostId, usize)>,
        parents: (Vec<ParentLogEntry>, u64),
        checks: (Vec<DecisionInfo>, u64),
        reads: Vec<[u64; 3]>,
        timers: (Vec<TimerRecord>, u64, TimerContext),
        links: LinkTake,
        origin: Time,
    }

    fn fixture() -> Fixture {
        let origin = Time::from_unix_millis(1_800_000_000_000);
        let at = move |ms: i64| origin + Duration::from_millis(ms);
        let swarm = SwarmKey::default();
        let ids: Vec<_> = (0..64).map(|i| (host(i), i)).collect();
        let peer = |i: Option<usize>| i.map(host);
        let logs = every(origin);
        let mut m = NodeMetrics::default();
        for (p, forecast) in &logs.forecasts {
            m.forecasts.push(strims_node_core::ForecastInfo {
                swarm,
                peer: peer(*p),
                forecast: *forecast,
            });
        }
        for (p, parent, asked, held) in &logs.held {
            m.held.push(strims_node_core::HeldInfo {
                swarm,
                bin: held.bin,
                at: held.at,
                held: *held,
                peer: peer(*p),
                parent: peer(*parent),
                asked: peer(*asked),
            });
        }
        for (p, have) in &logs.haves {
            m.haves.push(strims_node_core::HaveInfo {
                swarm,
                peer: peer(*p),
                have: *have,
            });
        }
        for (p, rivals, record) in &logs.asks {
            m.requests.push(strims_node_core::RequestInfo {
                swarm,
                peer: peer(*p),
                rivals: rivals.iter().map(|r| peer(*r)).collect(),
                record: record.clone(),
            });
        }
        for (p, record) in &logs.seed_decisions {
            m.seed_decisions.push(strims_node_core::LogInfo {
                swarm,
                peer: peer(*p),
                record: *record,
            });
        }
        for (p, record) in &logs.seed_sends {
            m.seed_sends.push(strims_node_core::LogInfo {
                swarm,
                peer: peer(*p),
                record: *record,
            });
        }
        // A probe's peer without an index is a host the testbed doesn't name.
        for (p, record) in &logs.path_probes {
            m.path_probes.push(strims_node_core::PathProbeInfo {
                peer: host(p.unwrap_or(99)),
                record: *record,
            });
        }
        m.forecasts_lost = logs.lost.forecasts;
        m.held_lost = logs.lost.held;
        m.haves_lost = logs.lost.haves;
        m.requests_lost = logs.lost.requests;
        m.seed_log_lost = logs.lost.seed_log;
        let mut waits = WaitHistogram::default();
        waits.buckets[1] = 4;
        waits.buckets[3] = 2;
        let other = other_swarm();
        m.driver.bytes_written = 77_777;
        m.runner = RunnerMetrics {
            swarms: vec![swarm_info(other, 9), swarm_info(swarm, 1)],
            ..RunnerMetrics::default()
        };
        m.links = vec![
            link(Some(host(3)), LinkKind::WebRtc),
            link(None, LinkKind::Tcp),
            link(Some(host(99)), LinkKind::Ws),
        ];
        let swarm_bin = |swarm: SwarmKey, hits: u64| SwarmBin {
            swarm,
            hits,
            misses: 2,
            pruned: 0,
            switched: 1,
            shed: 0,
            sheds_sent: 0,
            parents: 3,
            upload_capacity: 250_000,
            upload_busy_permille: 400,
            seed_copies: 2,
            seed_budget_capacity: 0,
            counters: SwarmCounters {
                chunks_sent: 40,
                push_wait: waits,
                ..SwarmCounters::default()
            },
        };
        let bin = |ms: i64, swarms: Vec<SwarmBin>| NodeBin {
            at: at(ms),
            swarms,
            outbound: strims_ppspp::ChannelMetrics::default(),
            bytes_written: 9000 + ms as u64,
        };
        m.bins = vec![
            bin(100, vec![swarm_bin(other, 1), swarm_bin(swarm, 5)]),
            bin(200, vec![swarm_bin(swarm, 7)]),
            bin(300, vec![swarm_bin(other, 2)]),
        ];
        let parents = vec![
            ParentLogEntry::Change(ParentInfo {
                at: at(150),
                swarm,
                stream: 1,
                parent: Some(host(2)),
                why: ParentWhy::Open,
            }),
            ParentLogEntry::Change(ParentInfo {
                at: at(160),
                swarm,
                stream: 2,
                parent: None,
                why: ParentWhy::Lost,
            }),
            ParentLogEntry::Change(ParentInfo {
                at: at(165),
                swarm: other,
                stream: 1,
                parent: Some(host(4)),
                why: ParentWhy::Open,
            }),
        ];
        let choice = ParentChoice {
            action: ParentAction::Graft,
            parent: None,
            best: None,
            runner_up: None,
            unready: 1,
            subscriber: 0,
            backed_off: 2,
            stale: 0,
            behind: 3,
            votes: 4,
            parent_have: Some(Duration::from_micros(2500)),
            parent_push: None,
        };
        let checks = vec![DecisionInfo {
            at: at(170),
            swarm: Some(swarm),
            peer: Some(host(3)),
            runner_up: None,
            parent: Some(host(4)),
            kind: TapKind::ParentChoice { stream: 1, choice },
        }];
        let timer = |source, ms: i64| TimerRecord {
            source,
            armed: at(ms) - Duration::from_micros(2_500),
            deadline: at(ms),
            fired: at(ms) + Duration::from_micros(130),
            requested: Duration::from_micros(2_500),
        };
        let timers = vec![
            timer(TimerSource::Writer, 0),
            timer(TimerSource::Record, 1000),
        ];
        let now = Arc::new(AtomicI64::new(0));
        let clock = now.clone();
        let log = LinkLog::new(Arc::new(move || at(clock.load(Ordering::Relaxed))));
        let sink = log.sink();
        sink.record(1, LinkEvent::Opened(LinkKind::WebRtc));
        sink.record(1, LinkEvent::Peer(host(1)));
        for k in 0..40 {
            now.store(k, Ordering::Relaxed);
            sink.record(1, LinkEvent::Queued(1200));
            sink.record(1, LinkEvent::Began(1200));
            if k % 2 == 0 {
                sink.record(
                    1,
                    LinkEvent::Checked {
                        source: SampleSource::WebRtc,
                        buffered: 9000,
                        low: Some(4096),
                    },
                );
                sink.record(1, LinkEvent::Held(WriteWait::Host));
            }
            sink.record(1, LinkEvent::Done);
            sink.record(1, LinkEvent::Arrived(80));
            sink.record(1, LinkEvent::Taken(80));
            sink.record(
                1,
                LinkEvent::Sample {
                    source: SampleSource::WebRtc,
                    state: LinkState {
                        unsent: Some(k as u64),
                        srtt: Some(Duration::from_micros(1500)),
                        buffered: Some(4),
                        uplink_queue: Some(Duration::from_micros(250)),
                        ..LinkState::default()
                    },
                },
            );
        }
        Fixture {
            m,
            ids,
            parents: (parents, 2),
            checks: (checks, 1),
            reads: (0..50).map(|q| [q, 1000 + q, 1040 + q]).collect(),
            timers: (timers, 0, TimerContext::Worker),
            links: log.take_raw(),
            origin,
        }
    }

    /// The fixture's line's parts.
    fn parts_of(f: &Fixture) -> LineParts {
        let s = Settings {
            index: 0,
            nodes: 8,
            start_ms: 1_800_000_000_000,
            secs: 60,
            arrives: false,
            rate: RATE,
            chunks: None,
            stream: Stream::Frames,
            probe: false,
        };
        Moment {
            t_ms: 1_800_000_001_000,
            meter_ms: 1_800_000_001_000,
            meter: &f.m,
            read_ms: 1_800_000_001_000,
            s: &s,
            seed: true,
            swarm: SwarmKey::default(),
            m: &f.m,
            lag: &[3, 5, 8],
            cpu_ms: Some(120),
            ids: f.ids.clone(),
            tick_late: 1,
            write_late: Some(2),
            wakeups: None,
            media: None,
            parents: f.parents.clone(),
            checks: Some(f.checks.clone()),
            reads: f.reads.clone(),
            timers: Some(f.timers.clone()),
            links: Some(f.links.clone()),
            origin: f.origin,
        }
        .parts()
    }

    /// A line's parts cross as bytes whole, and the writer makes the same
    /// line of them as of the parts it was given; the line holds every
    /// list's rows, the logs and the link log read back as their records.
    #[test]
    fn a_lines_parts_cross_as_bytes_and_make_the_same_line() {
        let f = fixture();
        let parts = parts_of(&f);
        let mut bytes = Vec::new();
        parts.to_wire(&mut bytes);
        let mut r = bytes.as_slice();
        let back = LineParts::from_wire(&mut r).unwrap();
        assert!(r.is_empty());
        assert_eq!(back, parts);
        let line = LineWriter::default().write(parts);
        assert_eq!(LineWriter::default().write(back), line);
        assert_eq!(LineParts::from_wire(&mut &bytes[..bytes.len() - 1]), None);
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["rel_ms"], 1000);
        assert_eq!(v["links"].as_array().unwrap().len(), 2);
        assert_eq!(v["links"][0]["peer"], 3);
        let rec = &v["rec"];
        assert_eq!(rec["meter"]["rate"], 123_457);
        assert_eq!(rec["meter"]["bytes_written"], 77_777);
        assert_eq!(rec["seed"], serde_json::json!([4, 6, 2, 3, 1_500_000]));
        assert_eq!(rec["bins"].as_array().unwrap().len(), 2);
        assert_eq!(rec["parents"].as_array().unwrap().len(), 2);
        assert_eq!(rec["reads"].as_array().unwrap().len(), 50);
        assert_eq!(rec["checks"].as_array().unwrap().len(), 1);
        assert_eq!(Logs::parse(&rec["logs"], f.origin), Some(every(f.origin)));
        assert_eq!(rec["timers"]["rows"].as_array().unwrap().len(), 2);
        let back = record::links::link_log_of(&rec["link_log"], f.origin).unwrap();
        let rows = LinkRowsMaker::default().rows(f.links.clone());
        assert_eq!(
            (back.writes, back.reads, back.samples),
            (rows.writes, rows.reads, rows.samples)
        );
        assert_eq!(back.links.len(), 1);
    }

    /// A link the log named before the node's peer events named its peer
    /// waits for it in the writer, and a line names it once they have.
    #[test]
    fn a_link_waits_in_the_writer_for_its_peers_name() {
        let mut f = fixture();
        let log = LinkLog::new(Arc::new(|| Time::from_unix_millis(1_800_000_000_500)));
        let sink = log.sink();
        sink.record(9, LinkEvent::Opened(LinkKind::Tcp));
        sink.record(9, LinkEvent::Peer(host(70)));
        f.links = log.take_raw();
        let mut w = LineWriter::default();
        let named = |line: &str| {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            v["rec"]["link_log"]["links"].as_array().unwrap().len()
        };
        assert_eq!(named(&w.write(parts_of(&f))), 0);
        f.links = log.take_raw();
        f.ids.push((host(70), 70));
        assert_eq!(named(&w.write(parts_of(&f))), 1);
    }
}
