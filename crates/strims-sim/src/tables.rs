// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A run's tables (`STRIMS_SIM_OUT=<dir>`, [`Report::write_tables`]): its
//! first swarm second by second, every read and its viewers' stalls, its
//! push trees' changes, finer bins, its timed events and its forecast
//! logs, as CSV files each with a `<table>.schema.json` of its columns'
//! names and types, for `hack/sim-db` to load without guessing them. Times
//! are true time, in ms (µs for `events`) from the swarm's stream start.
//!
//! | table | one row per | columns |
//! |---|---|---|
//! | `seconds` | second of [`Report::series`] | `sec`, then the series' per-second fields: `present`, `hits`, `misses`, `pruned`, `parents`, `capacity` (the seed's relay busy-time reading, `PushStats::upload_capacity`, at the stats tick that opens the second: one tick before the one the second's counters come from), `busy` (the same tick's, in thousandths), `avail`, `copies`, `sent`, `pushed`, `seed_ovh`, `viewer_ovh`, `budget_capacity` (the capacity reading the seed's budget was measured from, `PushStats::seed_budget_capacity`, at `capacity`'s tick: 0 until measured, -1 without a reading), and `written` for a media swarm |
//! | `viewer_seconds` | viewer and second it was there ([`Report::presence`]) | `viewer`, `sec`, `lag_ms` (the series' `lag`: the p99 of the frames read in the second, -1 without any), `frames` |
//! | `reads` | frame a viewer read (duplicates too) | `viewer`, `t_ms` (when it was read: its write time plus its lag), `seq`, `lag_ms` |
//! | `stalls` | stall of a viewer's playout at each delay of [`DELAYS_MS`] (`hack/capture/analyze.py`'s `DELIVERY_TARGET_MS` and `DELIVERY_MAX_MS`) | `viewer`, `delay_ms`, `t_ms` (when it began, on the `reads`' clock), `stall_ms`: [`playout::play`] over the viewer's `reads`, as `hack/sim-db`'s `playback(delay_ms)` plays them |
//! | `parents` | change of a node's push parent of a stream ([`Report::parent_changes`]) | `t_ms`, `node` (its host, as in `events`), `stream`, `parent` (the host from then on, -1 for none), `reason`: `open` and `switch` (a STREAM_OPEN gave the stream a parent, or replaced it), `prune` (the node cancelled it), `shed` (the parent closed the subscription), `lost` (their channel closed, the stream started anew, or the node stopped without closing its links: a crash, an abrupt departure) |
//! | `bins` | bin of [`Scenario::wire_bins`] | [`bin_cols`]: `bin`, `t_ms` (its start); the honest viewers present at both its edges and what they did in it (the series' counters, summed, and their median open parents at its end); the seed's meter, `capacity` (the relay busy-time reading, `PushStats::upload_capacity`) and `busy`, the uplink it had and its copies at its end and what it sent; then [`NODE_COLS`] (timeouts, wasted chunks, answers refused and dropped, parent switches, sheds taken and sent, message bytes, payloads, retransmits, drops) and [`WAIT_COLS`] summed over those viewers (`viewer_`) and the seed's (`seed_`), and the window model's window falls and timeouts (`*_window_cuts`, `*_rtos`), then [`ACK_COLS`], the acknowledgements alone their uplinks carried, packets and wire bytes, summed over those viewers and the seed's: TCP's pure ACKs (`*_up_ack_packets`, `*_up_ack_bytes`) and the data channels' SACKs (`*_up_sack_packets`, `*_up_sack_bytes`), none of them in `*_up_bytes`, `*_up_packets` or `*_small_packets`; and `budget_capacity` (the capacity reading the seed's budget was measured from, at the bin's end: `PushStats::seed_budget_capacity`, 0 until measured) |
//! | `node_bins` | node and bin, with `STRIMS_SIM_BIN_NODES=1` | [`node_bin_cols`]: `bin`, `t_ms`, `node` (its host), `viewer` (its number in `viewers`, -1 for the seed and hostile hosts), what it did in the bin |
//! | `viewers` | viewer, in `viewer_seconds`' and `reads`' numbering | `viewer`, `host`, `name`, `up` (its uplink at the start in bytes per second, -1 without a limit; its steps are `events`' `up_step`), `hostile`, `churned` (it restarted), `clock_ms` (its clock's offset), `arrive_ms` (when it booted, [`HostSpec::arrive_at`]: the run's start, negative, for one that came with it) and `depart_ms` (when it left for good, [`Fault::Depart`]; empty if it stayed) |
//! | `events` | timed event | `t_us`, `kind`, `node` (the host it happened to, -1 for none), `peer` (another host, -1 for none), `value`, `detail`: every fault of the scenario (`partition`, `repair`, `hold`, `release` with `node` and `peer` the groups' first hosts and `value` the groups' sizes' product, `crash`, `restart`, `latency` in ms, `pause_reading`, `resume_reading`, `reset_writer` with `node` the swarm, `uplink` in bytes per second, `connect`), `depart` with `value` 1 for an abrupt departure, 0 for a graceful one, every host's uplink steps (`up_step`), competing uploads (`competing_start` (`value` the competing flow's round trip in ms), `competing_stop`), each arriving viewer's `arrive` ([`HostSpec::arrive_at`]), each viewer incarnation's `join`, `first_frame` (`value` its ms from the join) and `caught_up` (the same), each `parent_lost` ([`Report::parent_losses`]: `node` the viewer, `peer` the parent that departed, `value` the ms from its departure until the viewer stopped counting on it, -1 if it never did, at its departure then) and the seed's `copies` (`value` its copies, at each change of its seed sample: its budget, window, copies or first wave, so `value` can repeat) |
//! | `forecast` | sample of a node's forecast log ([`strims_ppspp::LagForecast`]): a HAVE's announcement lag, with `STRIMS_SIM_FORECAST=1` | `t_ms` (the HAVE's arrival), `node`, `viewer`, `peer` (the hosts), `peer_honest` (not a hostile host), `stream`, `seq` (the chunk), `sample_ms` (the lag: from our first sight of the chunk's unit to the HAVE), `mean_ms` (the mean of the peer's lag estimate before the sample, -1 before its first sample), `horizon_ms` (how late a sample still fits that estimate: the later of its decaying peak and its mean plus two deviations, -1 before its first sample), `samples` (the estimate's so far), `fresh` (its latest sample was recent enough for a candidate), `lazy` (we asked the peer for lazy HAVEs), `first_sight` (this HAVE set the unit's first sight: the sample is 0), `run` (the sample's place in the HAVE's run of its stream's bins: a whole unit's HAVE gives one per chunk, the same lag each), `seen_ms` (our first sight of the unit, `t_ms - sample_ms`), `held_ms` (when the peer first stored the chunk, in the incarnation it had at the sample: its runner's held log, `Runner::take_held`, a seed's own writes included; empty if that incarnation never did), `channel_open_ms` (when our channel with the peer opened) and `live` (it was held after both hosts' incarnations started and after the channel opened: the HAVE tracked its arrival, not a backlog the channel's first HAVEs announce) |
//!
//! `series_meta.json` holds the stream's `rate`, `chunk`, `frame` and
//! `secs`, the seed's `seed_up` steps and the viewers' `viewer_up` as the
//! series has them, the number of `seconds`, the bins' `bin_ms`, the
//! hostile viewers' numbers (`hostile`), and the forecast samples the
//! nodes' caps left out (`forecasts_lost`).
//!
//! The runners' counters reach the bins from the nodes themselves
//! (`NodeConfig::stats_bins`), read at each edge before the node's first
//! pass past it and handed over with each sample, so the bins see them
//! without the run sampling more often than [`Scenario::stats_every`]. The
//! network's come from the run loop. Fields a run doesn't have (the
//! network's outside WAN mode, a node's in bins it has no reading at both
//! edges of, a quantile of no waits) are empty: `NULL` to `hack/sim-db`.
//!
//! [`Scenario::wire_bins`]: crate::Scenario::wire_bins
//! [`Scenario::stats_every`]: crate::Scenario::stats_every
//! [`HostSpec::arrive_at`]: crate::HostSpec::arrive_at

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use strims_base::stats::WaitHistogram;
use strims_wan::playout::{self, DELAYS_MS};

use crate::report::{NODE_BIN_COUNTERS, NodeBinRecord, Report, ViewerReport};
use crate::spec::{Departure, Fault, HostClass, HostIx};
pub use crate::wan::UP_COUNTERS;

/// A column's type, as `hack/sim-db` names it.
#[derive(Clone, Copy)]
pub(crate) enum Col {
    Int,
    Text,
    Bool,
}

impl Col {
    fn name(self) -> &'static str {
        match self {
            Col::Int => "BIGINT",
            Col::Text => "VARCHAR",
            Col::Bool => "BOOLEAN",
        }
    }
}

/// How much of a [`Table`]'s CSV it holds before writing it to its file.
const SPILL: usize = 1 << 20;

/// The tables that grow with a run's nodes and its length, written
/// zstd-compressed, `<name>.csv.zst` (`hack/sim-db` reads either): an
/// overloaded run's come to gigabytes of CSV, which a job's dir holds until
/// it is folded. The others are a run's summaries, `<name>.csv`.
pub const ZSTD: [&str; 4] = ["reads", "node_bins", "events", "forecast"];

/// Where a [`Table`]'s rows go: its file, or a zstd stream onto it.
enum Sink {
    Plain(File),
    Zstd(zstd::stream::write::Encoder<'static, File>),
}

impl Sink {
    fn write_all(&mut self, b: &[u8]) -> io::Result<()> {
        match self {
            Sink::Plain(f) => io::Write::write_all(f, b),
            Sink::Zstd(z) => io::Write::write_all(z, b),
        }
    }

    /// Ends the stream: a zstd frame's last block and its checksum.
    fn finish(self) -> io::Result<()> {
        match self {
            Sink::Plain(_) => Ok(()),
            Sink::Zstd(z) => z.finish().map(drop),
        }
    }
}

/// `<name>.csv` in `dir`, or `<name>.csv.zst` for a table of [`ZSTD`].
fn sink(dir: &Path, name: &str) -> io::Result<Sink> {
    if ZSTD.contains(&name) {
        let file = File::create(dir.join(format!("{name}.csv.zst")))?;
        Ok(Sink::Zstd(zstd::stream::write::Encoder::new(file, 0)?))
    } else {
        Ok(Sink::Plain(File::create(dir.join(format!("{name}.csv")))?))
    }
}

/// A table's CSV as a run wrote it: `<name>.csv`, or `<name>.csv.zst`
/// decompressed.
pub fn read_table(dir: &Path, name: &str) -> io::Result<String> {
    let z = dir.join(format!("{name}.csv.zst"));
    if z.exists() {
        let csv = zstd::stream::decode_all(File::open(z)?)?;
        return String::from_utf8(csv).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
    }
    std::fs::read_to_string(dir.join(format!("{name}.csv")))
}

/// One table being written: its columns, and its rows as CSV, which go to
/// its file as the table fills, a MiB at a time ([`SPILL`]): a long or
/// overloaded run's logs come to gigabytes of CSV, more than their records
/// take. [`Table::finish`] writes the rest and `<name>.schema.json`.
pub(crate) struct Table {
    name: &'static str,
    cols: Vec<(String, Col)>,
    dir: PathBuf,
    file: Sink,
    /// The rows not written yet.
    csv: String,
    /// The first write that failed: [`Table::finish`] returns it, and
    /// nothing is written after it.
    failed: Option<io::Error>,
}

impl Table {
    /// Creates the table's file in `dir` ([`ZSTD`]), its header the first
    /// line.
    pub fn new(dir: &Path, name: &'static str, cols: &[(&str, Col)]) -> io::Result<Table> {
        let cols: Vec<(String, Col)> = cols.iter().map(|&(n, c)| (n.to_owned(), c)).collect();
        let file = sink(dir, name)?;
        let mut csv = String::new();
        let header: Vec<&str> = cols.iter().map(|(n, _)| n.as_str()).collect();
        csv.push_str(&header.join(","));
        csv.push('\n');
        Ok(Table {
            name,
            cols,
            dir: dir.to_owned(),
            file,
            csv,
            failed: None,
        })
    }

    /// A table of integer columns.
    pub fn ints_named(dir: &Path, name: &'static str, cols: &[String]) -> io::Result<Table> {
        let cols: Vec<(&str, Col)> = cols.iter().map(|n| (n.as_str(), Col::Int)).collect();
        Table::new(dir, name, &cols)
    }

    /// Appends a row of integers, `None` an empty field (`NULL` to
    /// `hack/sim-db`).
    pub fn opt_ints(&mut self, row: &[Option<i64>]) {
        for (i, x) in row.iter().enumerate() {
            if i > 0 {
                self.csv.push(',');
            }
            if let Some(x) = x {
                let _ = write!(self.csv, "{x}");
            }
        }
        self.end_row();
    }

    /// Appends a row of integers.
    pub fn ints(&mut self, row: &[i64]) {
        for (i, x) in row.iter().enumerate() {
            if i > 0 {
                self.csv.push(',');
            }
            let _ = write!(self.csv, "{x}");
        }
        self.end_row();
    }

    /// Appends a row already formatted as CSV fields.
    pub fn fields(&mut self, row: &[String]) {
        self.csv.push_str(&row.join(","));
        self.end_row();
    }

    /// Ends a row, and writes the rows held once they come to [`SPILL`].
    fn end_row(&mut self) {
        self.csv.push('\n');
        if self.csv.len() >= SPILL {
            self.flush();
        }
    }

    /// Writes the rows held to the file, unless a write failed before.
    fn flush(&mut self) {
        if self.failed.is_none()
            && let Err(e) = self.file.write_all(self.csv.as_bytes())
        {
            self.failed = Some(e);
        }
        self.csv.clear();
    }

    /// Writes the rows not written yet, then `<name>.schema.json`; the
    /// first write that failed, if one did.
    pub fn finish(mut self) -> io::Result<()> {
        self.flush();
        if let Some(e) = self.failed.take() {
            return Err(e);
        }
        self.file.finish()?;
        let cols: Vec<String> = self
            .cols
            .iter()
            .map(|(n, c)| format!("{{\"name\":\"{n}\",\"type\":\"{}\"}}", c.name()))
            .collect();
        std::fs::write(
            self.dir.join(format!("{}.schema.json", self.name)),
            format!("{{\"columns\":[{}]}}\n", cols.join(",")),
        )
    }
}

/// `at` in ms from `start`, negative before it.
pub(crate) fn ms_since(at: Duration, start: Duration) -> i64 {
    match at.checked_sub(start) {
        Some(d) => d.as_millis() as i64,
        None => -(start.saturating_sub(at).as_millis() as i64),
    }
}

/// Text for a CSV field: names here carry no commas, quotes or newlines.
fn text(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, ',' | '"' | '\n' | '\r'))
        .collect()
}

/// A node's counters in the bins beyond the series', as the ppspp sim's
/// tables name them too: requests timed out and chunks wasted, answers a
/// full queue refused (`answers_refused`) and answers dropped at write time
/// (`answers_dropped`, ppspp's `DataDropped`), parents replaced
/// (`switched`), parents lost to their closing the subscription (`shed`)
/// and subscribers closed for a saturated uplink (`sheds_sent`), outbound
/// message bytes by kind, payloads and those of HAVEs alone (from the
/// node's [`NODE_BIN_COUNTERS`]), then stream messages that lost a packet
/// and packets lost (the network's [`UP_COUNTERS`]).
pub const NODE_COLS: [&str; 21] = [
    "timeouts",
    "wasted",
    "answers_refused",
    "answers_dropped",
    "switched",
    "shed",
    "sheds_sent",
    "have_bytes",
    "ack_bytes",
    "request_bytes",
    "announce_bytes",
    "stream_bytes",
    "session_bytes",
    "ping_bytes",
    "header_bytes",
    "integrity_bytes",
    "data_bytes",
    "payloads",
    "have_only_payloads",
    "retransmits",
    "drops",
];

/// What a bin's waits give: pushes written and answers sent in it, and the
/// p50 and p99 of their waits (empty without any).
pub const WAIT_COLS: [&str; 6] = [
    "push_waits",
    "push_wait_p50_ms",
    "push_wait_p99_ms",
    "answer_waits",
    "answer_wait_p50_ms",
    "answer_wait_p99_ms",
];

/// The window model's acknowledgements alone through a host's uplink, as
/// [`UP_COUNTERS`] names them: TCP's pure ACKs and the data channels'
/// SACKs, packets and wire bytes of each. The row's `packets_ack` and
/// `up_acks` count both kinds, over the whole run and every host.
pub const ACK_COLS: [&str; 4] = [
    "up_ack_packets",
    "up_ack_bytes",
    "up_sack_packets",
    "up_sack_bytes",
];

/// The `bins` table's columns ([`Report::write_tables`]).
pub fn bin_cols() -> Vec<String> {
    let mut cols: Vec<String> = [
        "bin",
        "t_ms",
        "present",
        "hits",
        "misses",
        "pruned",
        "parents",
        "viewer_ovh",
        "viewer_up_bytes",
        "viewer_up_packets",
        "viewer_small_packets",
        "capacity",
        "busy",
        "avail",
        "copies",
        "sent",
        "pushed",
        "seed_ovh",
        "seed_up_bytes",
        "seed_up_packets",
        "seed_small_packets",
    ]
    .map(String::from)
    .to_vec();
    for who in ["viewer", "seed"] {
        cols.extend(
            NODE_COLS
                .iter()
                .chain(&WAIT_COLS)
                .map(|c| format!("{who}_{c}")),
        );
    }
    for who in ["viewer", "seed"] {
        cols.push(format!("{who}_window_cuts"));
        cols.push(format!("{who}_rtos"));
    }
    for who in ["viewer", "seed"] {
        cols.extend(ACK_COLS.iter().map(|c| format!("{who}_{c}")));
    }
    cols.push("budget_capacity".into());
    cols
}

/// The `node_bins` table's columns: a node's in one bin.
pub fn node_bin_cols() -> Vec<String> {
    let mut cols: Vec<String> = [
        "bin",
        "t_ms",
        "node",
        "viewer",
        "hits",
        "misses",
        "pruned",
        "parents",
        "ovh",
        "up_bytes",
        "up_packets",
        "small_packets",
    ]
    .map(String::from)
    .to_vec();
    cols.extend(NODE_COLS.iter().chain(&WAIT_COLS).map(|c| c.to_string()));
    cols.push("window_cuts".into());
    cols.push("rtos".into());
    cols.extend(ACK_COLS.iter().map(|c| c.to_string()));
    cols
}

/// [`NODE_COLS`] the nodes count: all but the network's last two.
fn node_cols() -> &'static [&'static str] {
    NODE_COLS.get(..NODE_COLS.len() - 2).unwrap_or_default()
}

/// The index of a [`NODE_BIN_COUNTERS`] counter.
fn nc(name: &str) -> usize {
    NODE_BIN_COUNTERS
        .iter()
        .position(|&n| n == name)
        .unwrap_or(0)
}

/// The index of an [`UP_COUNTERS`] counter.
fn uc(name: &str) -> usize {
    UP_COUNTERS.iter().position(|&n| n == name).unwrap_or(0)
}

/// [`WAIT_COLS`]' values for the waits `w`.
fn wait_cols(w: &[WaitHistogram; 2]) -> [Option<i64>; WAIT_COLS.len()] {
    let p = |h: &WaitHistogram, q: f64| h.percentile(q).map(|d| d.as_millis());
    [
        Some(w[0].count() as i64),
        p(&w[0], 0.5),
        p(&w[0], 0.99),
        Some(w[1].count() as i64),
        p(&w[1], 0.5),
        p(&w[1], 0.99),
    ]
}

/// A node's readings at the two edges of a bin, from one incarnation.
struct Span<'a> {
    a: &'a NodeBinRecord,
    b: &'a NodeBinRecord,
}

impl Span<'_> {
    fn delta(&self, f: usize) -> i64 {
        let at = |r: &NodeBinRecord| r.counters.get(f).copied().unwrap_or(0);
        at(self.b).saturating_sub(at(self.a)) as i64
    }

    fn gauge(&self, f: usize) -> i64 {
        self.b.counters.get(f).copied().unwrap_or(0) as i64
    }

    /// Its waits' histograms in the bin.
    fn waits(&self) -> [WaitHistogram; 2] {
        let mut w = [WaitHistogram::default(); 2];
        for (x, (p, q)) in w.iter_mut().zip(self.b.waits.iter().zip(&self.a.waits)) {
            for (y, (m, n)) in x.buckets.iter_mut().zip(p.buckets.iter().zip(&q.buckets)) {
                *y = m.saturating_sub(*n);
            }
        }
        w
    }
}

/// Every host's node bins by edge.
struct NodeBins<'a>(Vec<BTreeMap<u64, &'a NodeBinRecord>>);

impl NodeBins<'_> {
    /// Host `h`'s readings at bin `k`'s edges, if one incarnation took both.
    fn span(&self, h: HostIx, k: u64) -> Option<Span<'_>> {
        let m = self.0.get(h)?;
        let (a, b) = (*m.get(&k)?, *m.get(&(k + 1))?);
        (a.incarnation == b.incarnation).then_some(Span { a, b })
    }

    fn edges(&self) -> u64 {
        self.0
            .iter()
            .filter_map(|m| m.keys().next_back())
            .max()
            .map_or(0, |&k| k + 1)
    }
}

impl Report {
    /// Host `h`'s network counter `f` added in bin `k`; `None` outside WAN
    /// mode.
    fn wire_delta(&self, h: HostIx, k: usize, f: usize) -> Option<i64> {
        let (a, b) = (
            self.wire_bins.get(k)?.get(h)?,
            self.wire_bins.get(k + 1)?.get(h)?,
        );
        Some(b.get(f)?.saturating_sub(*a.get(f)?) as i64)
    }

    /// Writes the run's tables for its first swarm into `dir` (see the
    /// [module docs](self)); `bin` is [`crate::Scenario::wire_bins`]'s
    /// width, and `nodes` writes the `node_bins` table. Without transfer
    /// samples there are no `seconds` or `viewer_seconds`.
    pub fn write_tables(&self, dir: &Path, bin: Duration, nodes: bool) -> io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let swarm = 0;
        let Some(spec) = self.scenario.swarms.get(swarm) else {
            return Ok(());
        };
        let start = spec.start;
        let viewers: Vec<_> = self.viewers.iter().filter(|v| v.swarm == swarm).collect();
        let series = self.series_data(swarm);
        if let Some(s) = &series {
            let mut cols = vec![("sec", Col::Int)];
            cols.extend(s.cols.iter().map(|(n, _)| (*n, Col::Int)));
            let mut t = Table::new(dir, "seconds", &cols)?;
            let mut row = Vec::with_capacity(cols.len());
            for k in 0..s.len() {
                row.clear();
                row.push(k as i64);
                row.extend(s.cols.iter().map(|(_, v)| v.get(k).copied().unwrap_or(-1)));
                t.ints(&row);
            }
            t.finish()?;
            let mut t = Table::new(
                dir,
                "viewer_seconds",
                &[
                    ("viewer", Col::Int),
                    ("sec", Col::Int),
                    ("lag_ms", Col::Int),
                    ("frames", Col::Int),
                ],
            )?;
            for (i, (lag, frames)) in s.lag.iter().zip(&s.frames).enumerate() {
                for (k, (l, f)) in lag.iter().zip(frames).enumerate() {
                    // A second it wasn't there in (see `Report::series`).
                    if *f < 0 {
                        continue;
                    }
                    t.ints(&[i as i64, k as i64, *l, *f]);
                }
            }
            t.finish()?;
        }
        let ms_from = |at: Duration| ms_since(at, start);
        let mut t = Table::new(
            dir,
            "reads",
            &[
                ("viewer", Col::Int),
                ("t_ms", Col::Int),
                ("seq", Col::Int),
                ("lag_ms", Col::Int),
            ],
        )?;
        let mut stalls = Table::new(
            dir,
            "stalls",
            &[
                ("viewer", Col::Int),
                ("delay_ms", Col::Int),
                ("t_ms", Col::Int),
                ("stall_ms", Col::Int),
            ],
        )?;
        for (i, v) in viewers.iter().enumerate() {
            let reads = self.read_rows(v);
            for &(at, seq, lag) in &reads {
                t.ints(&[i as i64, at, seq as i64, lag]);
            }
            for d in DELAYS_MS {
                for (at, ms) in playout::play(&reads, d).stalls {
                    stalls.ints(&[i as i64, d, at, ms]);
                }
            }
        }
        t.finish()?;
        stalls.finish()?;
        let mut t = Table::new(
            dir,
            "parents",
            &[
                ("t_ms", Col::Int),
                ("node", Col::Int),
                ("stream", Col::Int),
                ("parent", Col::Int),
                ("reason", Col::Text),
            ],
        )?;
        for p in &self.parent_changes {
            t.fields(&[
                ms_from(p.at).to_string(),
                p.node.to_string(),
                p.stream.to_string(),
                p.parent.map_or(-1, |h| h as i64).to_string(),
                p.why.name().to_owned(),
            ]);
        }
        t.finish()?;
        let hostile = |h: HostIx| {
            matches!(
                self.scenario.hosts.get(h).map(|x| x.class),
                Some(HostClass::Hostile(_))
            )
        };
        let honest: Vec<HostIx> = viewers
            .iter()
            .map(|v| v.host)
            .filter(|&h| !hostile(h))
            .collect();
        let width = bin.max(Duration::from_millis(1));
        self.bins_table(dir, spec.seed, &honest, width)?.finish()?;
        let number: BTreeMap<HostIx, usize> = viewers
            .iter()
            .enumerate()
            .filter(|(_, v)| !hostile(v.host))
            .map(|(i, v)| (v.host, i))
            .collect();
        if nodes {
            let mut hosts = vec![spec.seed];
            hosts.extend(viewers.iter().map(|v| v.host));
            self.node_bins_table(dir, &hosts, &number, width)?
                .finish()?;
        }
        let mut t = Table::new(
            dir,
            "viewers",
            &[
                ("viewer", Col::Int),
                ("host", Col::Int),
                ("name", Col::Text),
                ("up", Col::Int),
                ("hostile", Col::Bool),
                ("churned", Col::Bool),
                ("clock_ms", Col::Int),
                ("arrive_ms", Col::Int),
                ("depart_ms", Col::Int),
            ],
        )?;
        for (i, v) in viewers.iter().enumerate() {
            let h = self.hosts.get(v.host);
            let (arrive, _) = self.presence(v.host);
            let depart = self.scenario.departure(v.host);
            t.fields(&[
                i.to_string(),
                v.host.to_string(),
                text(&v.name),
                self.upload_cap(v.host).map_or(-1, |c| c as i64).to_string(),
                hostile(v.host).to_string(),
                h.is_some_and(|h| h.restarts > 0).to_string(),
                h.map_or(0, |h| h.clock_offset.as_millis()).to_string(),
                ms_from(arrive).to_string(),
                depart.map_or(String::new(), |(at, _)| ms_from(at).to_string()),
            ]);
        }
        t.finish()?;
        self.events_table(dir, &viewers, start)?.finish()?;
        if self.scenario.forecast_log {
            self.forecast_table(dir, &number, start)?.finish()?;
        }
        let ints = |v: &[i64]| {
            let v: Vec<String> = v.iter().map(i64::to_string).collect();
            v.join(",")
        };
        let mut meta = String::from("{\"sim\":\"strims-sim\"");
        if let Some(s) = &series {
            let _ = write!(
                meta,
                ",\"rate\":{},\"chunk\":{},\"frame\":{},\"secs\":{},\"seed_up\":[{}],\"viewer_up\":{},\"seconds\":{},\"hostile\":[{}]",
                s.rate,
                s.chunk,
                s.frame,
                s.secs,
                s.seed_up.join(","),
                s.viewer_up,
                s.len(),
                ints(&s.hostile),
            );
        }
        let lost: u64 = self.hosts.iter().map(|h| h.forecasts_lost).sum();
        let _ = write!(
            meta,
            ",\"viewers\":{},\"bin_ms\":{},\"forecasts_lost\":{lost}}}",
            viewers.len(),
            width.as_millis()
        );
        meta.push('\n');
        std::fs::write(dir.join("series_meta.json"), meta)
    }

    /// Viewer `v`'s reads as the `reads` table has them: `(t_ms, seq,
    /// lag_ms)`, `t_ms` from its swarm's stream start.
    pub(crate) fn read_rows(&self, v: &ViewerReport) -> Vec<(i64, u64, i64)> {
        let start = self
            .scenario
            .swarms
            .get(v.swarm)
            .map_or(Duration::ZERO, |s| s.start);
        v.stats
            .lags
            .iter()
            .map(|&(seq, lag)| {
                let at = self.written_at(v.swarm, seq) + Duration::from_millis(u64::from(lag));
                (ms_since(at, start), seq, i64::from(lag))
            })
            .collect()
    }

    /// Every host's node bins, by edge.
    fn node_bins(&self) -> NodeBins<'_> {
        NodeBins(
            self.hosts
                .iter()
                .map(|h| h.node_bins.iter().map(|b| (b.bin, b)).collect())
                .collect(),
        )
    }

    /// The `bins` table ([`bin_cols`]), of `seed` and the `honest` viewers.
    fn bins_table(
        &self,
        dir: &Path,
        seed: HostIx,
        honest: &[HostIx],
        width: Duration,
    ) -> io::Result<Table> {
        let nb = self.node_bins();
        let edges = nb.edges().max(self.wire_bins.len() as u64);
        let mut t = Table::ints_named(dir, "bins", &bin_cols())?;
        let w = width.as_millis() as i64;
        let start = self
            .scenario
            .swarms
            .first()
            .map_or(Duration::ZERO, |s| s.start);
        let wire_sum = |hosts: &[HostIx], k: usize, f: usize| -> Option<i64> {
            hosts.iter().map(|&h| self.wire_delta(h, k, f)).sum()
        };
        for k in 0..edges.saturating_sub(1) {
            let spans: Vec<Span<'_>> = honest.iter().filter_map(|&h| nb.span(h, k)).collect();
            let sum = |f: usize| -> Option<i64> { Some(spans.iter().map(|s| s.delta(f)).sum()) };
            let mut parents: Vec<i64> = spans.iter().map(|s| s.gauge(nc("parents"))).collect();
            parents.sort_unstable();
            let s = nb.span(seed, k);
            let gauge = |f: usize| s.as_ref().map(|s| s.gauge(f));
            let delta = |f: usize| s.as_ref().map(|s| s.delta(f));
            let ku = k as usize;
            let avail = self
                .upload_cap_at(seed, start + width * (k as u32 + 1))
                .map(|c| c as i64);
            let mut row = vec![
                Some(k as i64),
                Some(k as i64 * w),
                Some(spans.len() as i64),
                sum(nc("hits")),
                sum(nc("misses")),
                sum(nc("pruned")),
                parents.get(parents.len() / 2).copied(),
                sum(nc("ovh")),
                wire_sum(honest, ku, uc("up_bytes")),
                wire_sum(honest, ku, uc("up_packets")),
                wire_sum(honest, ku, uc("small_packets")),
                gauge(nc("capacity")),
                gauge(nc("busy")),
                avail,
                gauge(nc("copies")),
                delta(nc("sent")),
                delta(nc("pushed")),
                delta(nc("ovh")),
                self.wire_delta(seed, ku, uc("up_bytes")),
                self.wire_delta(seed, ku, uc("up_packets")),
                self.wire_delta(seed, ku, uc("small_packets")),
            ];
            // The node's counters of NODE_COLS, then the network's.
            let node_cols = node_cols();
            row.extend(node_cols.iter().map(|c| sum(nc(c))));
            row.push(wire_sum(honest, ku, uc("retransmits")));
            row.push(wire_sum(honest, ku, uc("drops")));
            let mut waits = [WaitHistogram::default(); 2];
            for sp in &spans {
                for (x, y) in waits.iter_mut().zip(&sp.waits()) {
                    x.merge(y);
                }
            }
            row.extend(wait_cols(&waits));
            row.extend(node_cols.iter().map(|c| delta(nc(c))));
            row.push(self.wire_delta(seed, ku, uc("retransmits")));
            row.push(self.wire_delta(seed, ku, uc("drops")));
            match &s {
                Some(s) => row.extend(wait_cols(&s.waits())),
                None => row.extend([None; WAIT_COLS.len()]),
            }
            row.push(wire_sum(honest, ku, uc("window_cuts")));
            row.push(wire_sum(honest, ku, uc("rtos")));
            row.push(self.wire_delta(seed, ku, uc("window_cuts")));
            row.push(self.wire_delta(seed, ku, uc("rtos")));
            row.extend(ACK_COLS.iter().map(|c| wire_sum(honest, ku, uc(c))));
            row.extend(ACK_COLS.iter().map(|c| self.wire_delta(seed, ku, uc(c))));
            row.push(gauge(nc("budget_capacity")));
            t.opt_ints(&row);
        }
        Ok(t)
    }

    /// The `node_bins` table ([`node_bin_cols`]) of `hosts`, each with its
    /// viewer `number`.
    fn node_bins_table(
        &self,
        dir: &Path,
        hosts: &[HostIx],
        number: &BTreeMap<HostIx, usize>,
        width: Duration,
    ) -> io::Result<Table> {
        let nb = self.node_bins();
        let edges = nb.edges().max(self.wire_bins.len() as u64);
        let mut t = Table::ints_named(dir, "node_bins", &node_bin_cols())?;
        let w = width.as_millis() as i64;
        let node_cols = node_cols();
        for k in 0..edges.saturating_sub(1) {
            let ku = k as usize;
            for &h in hosts {
                let Some(s) = nb.span(h, k) else {
                    continue;
                };
                let mut row = vec![
                    Some(k as i64),
                    Some(k as i64 * w),
                    Some(h as i64),
                    Some(number.get(&h).map_or(-1, |&i| i as i64)),
                    Some(s.delta(nc("hits"))),
                    Some(s.delta(nc("misses"))),
                    Some(s.delta(nc("pruned"))),
                    Some(s.gauge(nc("parents"))),
                    Some(s.delta(nc("ovh"))),
                    self.wire_delta(h, ku, uc("up_bytes")),
                    self.wire_delta(h, ku, uc("up_packets")),
                    self.wire_delta(h, ku, uc("small_packets")),
                ];
                row.extend(node_cols.iter().map(|c| Some(s.delta(nc(c)))));
                row.push(self.wire_delta(h, ku, uc("retransmits")));
                row.push(self.wire_delta(h, ku, uc("drops")));
                row.extend(wait_cols(&s.waits()));
                row.push(self.wire_delta(h, ku, uc("window_cuts")));
                row.push(self.wire_delta(h, ku, uc("rtos")));
                row.extend(ACK_COLS.iter().map(|c| self.wire_delta(h, ku, uc(c))));
                t.opt_ints(&row);
            }
        }
        Ok(t)
    }

    /// The `events` table (see the [module docs](self)), times from
    /// `start`.
    fn events_table(
        &self,
        dir: &Path,
        viewers: &[&ViewerReport],
        start: Duration,
    ) -> io::Result<Table> {
        let mut t = Table::new(
            dir,
            "events",
            &[
                ("t_us", Col::Int),
                ("kind", Col::Text),
                ("node", Col::Int),
                ("peer", Col::Int),
                ("value", Col::Int),
                ("detail", Col::Text),
            ],
        )?;
        let us = |at: Duration| -> i64 {
            match at.checked_sub(start) {
                Some(d) => d.as_micros() as i64,
                None => -(start.saturating_sub(at).as_micros() as i64),
            }
        };
        let mut rows: Vec<(i64, String, i64, i64, i64, String)> = Vec::new();
        let mut push =
            |at: Duration, kind: &str, node: i64, peer: i64, value: i64, detail: &str| {
                rows.push((us(at), kind.to_owned(), node, peer, value, text(detail)));
            };
        let first = |g: &[HostIx]| g.first().map_or(-1, |&h| h as i64);
        let groups = |a: &[HostIx], b: &[HostIx]| (a.len() * b.len()) as i64;
        for f in &self.scenario.faults {
            let at = f.at;
            match &f.fault {
                Fault::Partition(a, b) => {
                    push(at, "partition", first(a), first(b), groups(a, b), "")
                }
                Fault::Repair(a, b) => push(at, "repair", first(a), first(b), groups(a, b), ""),
                Fault::Hold(a, b) => push(at, "hold", first(a), first(b), groups(a, b), ""),
                Fault::Release(a, b) => push(at, "release", first(a), first(b), groups(a, b), ""),
                Fault::Crash(h) => push(at, "crash", *h as i64, -1, 0, ""),
                Fault::Restart(h) => push(at, "restart", *h as i64, -1, 0, ""),
                Fault::Latency(a, b, d) => {
                    push(at, "latency", first(a), first(b), d.as_millis() as i64, "")
                }
                Fault::PauseReading(h) => push(at, "pause_reading", *h as i64, -1, 0, ""),
                Fault::ResumeReading(h) => push(at, "resume_reading", *h as i64, -1, 0, ""),
                Fault::ResetWriter(s) => push(at, "reset_writer", *s as i64, -1, 0, ""),
                Fault::Uplink(h, bps) => push(at, "uplink", *h as i64, -1, *bps as i64, ""),
                Fault::Connect { from, to, network } => {
                    push(at, "connect", *from as i64, *to as i64, *network as i64, "")
                }
                Fault::Depart(h, how) => {
                    let (value, detail) = match how {
                        Departure::Graceful => (0, "graceful"),
                        Departure::Abrupt => (1, "abrupt"),
                    };
                    push(at, "depart", *h as i64, -1, value, detail)
                }
            }
        }
        for (h, spec) in self.scenario.hosts.iter().enumerate() {
            if let Some(at) = spec.arrive_at {
                push(at, "arrive", h as i64, -1, 0, "");
            }
        }
        for l in &self.parent_losses {
            let (at, value) = match l.noticed_ms {
                Some(n) => (n, n.saturating_sub(l.departed_ms) as i64),
                None => (l.departed_ms, -1),
            };
            push(
                Duration::from_millis(at),
                "parent_lost",
                l.viewer as i64,
                l.parent as i64,
                value,
                "",
            );
        }
        for (h, spec) in self.scenario.hosts.iter().enumerate() {
            for &(at, bps) in &spec.up_steps {
                push(at, "up_step", h as i64, -1, bps as i64, "");
            }
            for c in &spec.competing {
                push(
                    c.start,
                    "competing_start",
                    h as i64,
                    -1,
                    c.rtt.as_millis() as i64,
                    "",
                );
                push(c.stop, "competing_stop", h as i64, -1, 0, "");
            }
        }
        for v in viewers {
            for j in &v.stats.joins {
                let at = Duration::from_millis(j.start_ms);
                push(at, "join", v.host as i64, -1, 0, "");
                if let Some(ms) = j.first_frame_ms {
                    let d = ms.saturating_sub(j.start_ms) as i64;
                    push(
                        Duration::from_millis(ms),
                        "first_frame",
                        v.host as i64,
                        -1,
                        d,
                        "",
                    );
                }
                if let Some(ms) = j.caught_up_ms {
                    let d = ms.saturating_sub(j.start_ms) as i64;
                    push(
                        Duration::from_millis(ms),
                        "caught_up",
                        v.host as i64,
                        -1,
                        d,
                        "",
                    );
                }
            }
        }
        if let Some(seed) = self.scenario.swarms.first().map(|s| s.seed)
            && let Some(h) = self.hosts.get(seed)
        {
            for &(at, s, sample) in &h.runner.seed_changes {
                if s == 0 {
                    push(at, "copies", seed as i64, -1, sample.copies as i64, "");
                }
            }
        }
        rows.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then(a.2.cmp(&b.2))
        });
        for (at, kind, node, peer, value, detail) in rows {
            t.fields(&[
                at.to_string(),
                kind,
                node.to_string(),
                peer.to_string(),
                value.to_string(),
                detail,
            ]);
        }
        Ok(t)
    }

    /// The `forecast` table (see the [module docs](self)): every host's
    /// forecast log, `number` the viewers' numbers, times from `start`.
    fn forecast_table(
        &self,
        dir: &Path,
        number: &BTreeMap<HostIx, usize>,
        start: Duration,
    ) -> io::Result<Table> {
        let mut t = Table::new(
            dir,
            "forecast",
            &[
                ("t_ms", Col::Int),
                ("node", Col::Int),
                ("viewer", Col::Int),
                ("peer", Col::Int),
                ("peer_honest", Col::Bool),
                ("stream", Col::Int),
                ("seq", Col::Int),
                ("sample_ms", Col::Int),
                ("mean_ms", Col::Int),
                ("horizon_ms", Col::Int),
                ("samples", Col::Int),
                ("fresh", Col::Bool),
                ("lazy", Col::Bool),
                ("first_sight", Col::Bool),
                ("run", Col::Int),
                ("seen_ms", Col::Int),
                ("held_ms", Col::Int),
                ("channel_open_ms", Col::Int),
                ("live", Col::Bool),
            ],
        )?;
        let ms = |at: Duration| -> i64 {
            match at.checked_sub(start) {
                Some(d) => d.as_millis() as i64,
                None => -(start.saturating_sub(at).as_millis() as i64),
            }
        };
        // When each host's incarnations started: a chunk an earlier one
        // held isn't what a later one announced.
        let mut starts: Vec<Vec<Duration>> = vec![vec![Duration::ZERO]; self.hosts.len()];
        for f in &self.scenario.faults {
            if let Fault::Restart(h) = f.fault
                && let Some(s) = starts.get_mut(h)
            {
                s.push(f.at);
            }
        }
        for s in &mut starts {
            s.sort_unstable();
        }
        let place = |h: HostIx, at: Duration| -> Duration {
            starts
                .get(h)
                .and_then(|s| s.iter().rev().find(|&&t| t <= at).copied())
                .unwrap_or(Duration::ZERO)
        };
        // ... and when the one after it did, if any.
        let next = |h: HostIx, at: Duration| -> Duration {
            starts
                .get(h)
                .and_then(|s| s.iter().find(|&&t| t > at).copied())
                .unwrap_or(Duration::MAX)
        };
        // Every host's chunks by index, with when each incarnation first
        // held it, in order.
        let held: Vec<BTreeMap<u64, Vec<Duration>>> = self
            .hosts
            .iter()
            .map(|h| {
                let mut m: BTreeMap<u64, Vec<Duration>> = BTreeMap::new();
                for &(seq, at) in &h.held {
                    m.entry(seq).or_default().push(at);
                }
                for v in m.values_mut() {
                    v.sort_unstable();
                }
                m
            })
            .collect();
        for (h, host) in self.hosts.iter().enumerate() {
            for r in &host.forecasts {
                let f = &r.forecast;
                let seen = r.at.saturating_sub(f.sample.to_std().unwrap_or_default());
                let est = f.estimate;
                // The peer's first hold in the incarnation that announced
                // it, and whether it came after we took our place too.
                let truth = r.peer.and_then(|p| {
                    let (since, until) = (place(p, r.at), next(p, r.at));
                    held.get(p)?
                        .get(&f.bin.base_offset())?
                        .iter()
                        .find(|&&t| t >= since && t < until)
                        .copied()
                });
                let opened = r.opened;
                let live = truth.is_some_and(|t| t >= place(h, r.at) && t >= opened);
                t.fields(&[
                    ms(r.at).to_string(),
                    h.to_string(),
                    number.get(&h).map_or(-1, |&i| i as i64).to_string(),
                    r.peer.map_or(-1, |p| p as i64).to_string(),
                    r.peer
                        .is_some_and(|p| {
                            !matches!(
                                self.scenario.hosts.get(p).map(|x| x.class),
                                Some(HostClass::Hostile(_))
                            )
                        })
                        .to_string(),
                    f.stream.to_string(),
                    f.bin.base_offset().to_string(),
                    f.sample.as_millis().to_string(),
                    est.map_or(-1, |e| e.mean.as_millis()).to_string(),
                    est.map_or(-1, |e| e.horizon.as_millis()).to_string(),
                    est.map_or(0, |e| e.samples).to_string(),
                    est.is_some_and(|e| e.fresh).to_string(),
                    f.lazy.to_string(),
                    f.first_sight.to_string(),
                    f.run.to_string(),
                    ms(seen).to_string(),
                    truth.map_or(String::new(), |t| ms(t).to_string()),
                    ms(opened).to_string(),
                    live.to_string(),
                ]);
            }
        }
        Ok(t)
    }
}
