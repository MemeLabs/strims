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
//! Every second from 5 s before the start, the node emits one JSON line
//! ([`Runtime::emit`]); a second it missed has none:
//!
//! | field | |
//! |---|---|
//! | `t_ms`, `rel_ms` | the wall clock, and ms since the start |
//! | `index`, `seed` | the node, and whether it seeds |
//! | `frames`, `lag_p50_ms`, `lag_p99_ms`, `lag_max_ms` | at a viewer, the frames read in that second and their lag; of a media stream, the unit records read whole and their lag from their publish times, which no record holds |
//! | `cpu_ms` | the driver's CPU time so far, `null` when it can't tell |
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

use std::cell::{Cell, RefCell};
use std::future::Future;

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
use crate::record::links::{LinkLog, LinkOpen, LinkRows};
use crate::record::logs::Logs;
use crate::record::{self, DecisionLog, ParentLog, TimerContext, TimerLog};
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
        // Links the log named before the node's peer events named their
        // peers, and the lines they have waited.
        let mut unnamed: Vec<(LinkOpen, u32)> = Vec::new();
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
            let now = Time::from_unix_millis(i64::try_from(t).unwrap_or(i64::MAX));
            let link_log = links.map(|log| {
                let mut rows = log.take();
                let ids = ids.borrow();
                let known = |l: &LinkOpen| ids.iter().any(|(id, _)| *id == l.peer);
                let waiting: Vec<_> = unnamed
                    .drain(..)
                    .chain(rows.links.drain(..).map(|l| (l, 0)))
                    .collect();
                let (named, later): (Vec<_>, Vec<_>) =
                    waiting.into_iter().partition(|(l, _)| known(l));
                rows.links = named.into_iter().map(|(l, _)| l).collect();
                // A peer the swarm never names (the relay) waits a minute.
                unnamed = later
                    .into_iter()
                    .filter(|&(_, w)| w < 60)
                    .map(|(l, w)| (l, w + 1))
                    .collect();
                rows
            });
            let rec = Records {
                t_ms: t,
                swarm,
                m: &m,
                ids: &ids.borrow(),
                parents: parents.map(ParentLog::take).unwrap_or_default(),
                checks: checks.map(DecisionLog::take),
                reads: std::mem::take(&mut *reads.borrow_mut()),
                timers: timers.map(|log| {
                    let (rows, lost) = log.take();
                    (rows, lost, log.context())
                }),
                link_log,
                now,
                origin: Time::from_unix_millis(i64::try_from(s.start_ms).unwrap_or(i64::MAX)),
            }
            .json();
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
                rec,
                now,
            };
            rt.emit(line.json());
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
/// [`record::waits_json`]. With the node's decision log
/// ([`DecisionLog`]), its parent checks since the last line too,
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
struct Records<'a> {
    t_ms: u64,
    swarm: SwarmKey,
    m: &'a NodeMetrics,
    ids: &'a [(HostId, usize)],
    parents: (Vec<ParentLogEntry>, u64),
    checks: Option<(Vec<DecisionInfo>, u64)>,
    reads: Vec<[u64; 3]>,
    /// The timer log's take, and where its timers run.
    timers: Option<(Vec<TimerRecord>, u64, TimerContext)>,
    /// The link log's take.
    link_log: Option<LinkRows>,
    now: Time,
    origin: Time,
}

impl Records<'_> {
    fn json(&self) -> String {
        let index = |id: &HostId| self.ids.iter().find(|(x, _)| x == id).map(|&(_, i)| i);
        let swarm = self.swarm;
        let (meter, seeds) = record::meter_sample(
            self.m,
            |k| (k == swarm).then_some(0),
            std::time::Duration::from_millis(self.t_ms),
            self.now,
        );
        let seed = seeds
            .iter()
            .find(|(s, _)| *s == 0)
            .map_or("null".to_owned(), |(_, x)| x.json());
        let bins: Vec<String> = self
            .m
            .bins
            .iter()
            .filter_map(|b| {
                let sw = b.swarms.iter().find(|s| s.swarm == swarm)?;
                let c: Vec<String> = record::bin_counters(b, sw)
                    .iter()
                    .map(u64::to_string)
                    .collect();
                let [p, a] = record::bin_waits(sw);
                Some(format!(
                    "[{},[{}],{},{}]",
                    b.at.unix_millis(),
                    c.join(","),
                    record::waits_json(&p),
                    record::waits_json(&a)
                ))
            })
            .collect();
        let (changes, lost) = &self.parents;
        let parents: Vec<String> = changes
            .iter()
            .filter_map(|e| match e {
                ParentLogEntry::Change(p) if p.swarm == swarm => Some(format!(
                    "[{},{},{},\"{}\"]",
                    p.at.unix_millis(),
                    p.stream,
                    opt(p.parent.as_ref().and_then(index)),
                    p.why.name()
                )),
                _ => None,
            })
            .collect();
        let reads: Vec<String> = self
            .reads
            .iter()
            .map(|[q, s, r]| format!("[{q},{s},{r}]"))
            .collect();
        let checks = self
            .checks
            .as_ref()
            .map_or(String::new(), |(checks, lost)| {
                let us = |d: Option<Duration>| opt(d.map(|d| d.as_micros()));
                let rows: Vec<String> = checks
                    .iter()
                    .filter(|d| d.swarm == Some(swarm))
                    .filter_map(|d| {
                        let TapKind::ParentChoice { stream, choice: c } = d.kind else {
                            return None;
                        };
                        Some(format!(
                            "[{},{stream},\"{}\",{},{},{},{},{},{},{},{},{},{},{},{},{}]",
                            d.at.unix_nanos() / 1000,
                            c.action.name(),
                            opt(d.peer.as_ref().and_then(index)),
                            us(c.best.map(|b| b.1)),
                            opt(d.runner_up.as_ref().and_then(index)),
                            us(c.runner_up.map(|r| r.1)),
                            opt(d.parent.as_ref().and_then(index)),
                            us(c.parent_have),
                            us(c.parent_push),
                            c.unready,
                            c.subscriber,
                            c.backed_off,
                            c.stale,
                            c.behind,
                            c.votes,
                        ))
                    })
                    .collect();
                format!(",\"checks\":[{}],\"checks_lost\":{lost}", rows.join(","))
            });
        let logs = Logs::of(self.m, swarm, index);
        let logs = if logs.is_empty() {
            String::new()
        } else {
            format!(",\"logs\":{}", logs.json(self.origin))
        };
        let timers = self
            .timers
            .as_ref()
            .map_or(String::new(), |(rows, lost, context)| {
                format!(
                    ",\"timers\":{}",
                    record::timers_json(rows, *lost, *context, self.origin)
                )
            });
        let link_log = self.link_log.as_ref().map_or(String::new(), |rows| {
            format!(
                ",\"link_log\":{}",
                record::links::link_log_json(rows, index, self.origin)
            )
        });
        format!(
            "{{\"meter\":{},\"seed\":{seed},\"bins\":[{}],\"parents\":[{}],\"parents_lost\":{lost},\"reads\":[{}]{checks}{logs}{timers}{link_log}}}",
            meter.map_or("null".to_owned(), |m| m.json()),
            bins.join(","),
            parents.join(","),
            reads.join(","),
        )
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
    /// The line's records ([`Records`]), as JSON.
    rec: String,
    /// When the metrics were read.
    now: Time,
}

impl Line<'_> {
    fn json(&self) -> String {
        let m = self.m;
        let index = |id: &HostId| self.ids.iter().find(|(x, _)| x == id).map(|&(_, i)| i);
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
        let lag = self.lag;
        format!(
            "{{\"t_ms\":{},\"rel_ms\":{},\"index\":{},\"seed\":{},\"frames\":{},\"lag_p50_ms\":{},\"lag_p99_ms\":{},\"lag_max_ms\":{},\"cpu_ms\":{},\"transfers\":{transfers},\"links\":[{}],\"tick_late_ms\":{},\"write_late_ms\":{},\"wakeups\":{},\"media\":{},\"rec\":{}}}",
            self.t_ms,
            self.t_ms as i64 - self.s.start_ms as i64,
            self.s.index,
            self.seed,
            lag.len(),
            pct(lag, 0.5),
            pct(lag, 0.99),
            lag.last().copied().unwrap_or(0),
            opt(self.cpu_ms),
            links.join(","),
            self.tick_late,
            opt(self.write_late),
            opt(self.wakeups),
            self.media.as_deref().unwrap_or("null"),
            self.rec,
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
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
}
