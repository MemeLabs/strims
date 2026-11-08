// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A run's tables (`STRIMS_SIM_OUT=<dir>`, [`Report::write_tables`]): its
//! first swarm second by second, every read, and finer bins, as CSV files
//! each with a `<table>.schema.json` of its columns' names and types, for
//! `hack/sim-db` to load without guessing them. Times are true time, in ms
//! from the swarm's stream start.
//!
//! | table | one row per | columns |
//! |---|---|---|
//! | `seconds` | second of [`Report::series`] | `sec`, then the series' per-second fields (`present`, `hits`, `misses`, `pruned`, `parents`, `capacity`, `busy`, `avail`, `copies`, `sent`, `pushed`, `seed_ovh`, `viewer_ovh`, and `written` for a media swarm) |
//! | `viewer_seconds` | viewer and second | `viewer`, `sec`, `lag_ms` (the series' `lag`: the p99 of the frames read in the second, -1 without any), `frames` |
//! | `reads` | frame a viewer read (duplicates too) | `viewer`, `t_ms` (when it was read: its write time plus its lag), `seq`, `lag_ms` |
//! | `bins` | bin of [`Scenario::wire_bins`] (WAN mode) | `bin`, `t_ms` (its start), then what the seed's uplink carried in it (`seed_up_bytes`, `seed_up_packets`, `seed_small_packets`) and the honest viewers' summed (`viewer_up_*`) |
//! | `viewers` | viewer, in `viewer_seconds`' and `reads`' numbering | `viewer`, `host`, `name`, `hostile`, `up` (its uplink in bytes per second, -1 without a limit), `clock_ms` (its clock's offset) |
//!
//! `series_meta.json` holds the stream's `rate`, `chunk`, `frame` and
//! `secs`, the seed's `seed_up` steps and the viewers' `viewer_up` as the
//! series has them, the number of `seconds`, the bins' `bin_ms`, and the
//! hostile viewers' numbers (`hostile`).
//!
//! Only the network's counters go in `bins`: the runners' (pushes, prunes,
//! parents, the seed's meter and copies) are read through a node's
//! metrics call, an answer the node gives at its next pass, so reading
//! them more often than [`Scenario::stats_every`] would change the run.
//!
//! [`Scenario::wire_bins`]: crate::Scenario::wire_bins
//! [`Scenario::stats_every`]: crate::Scenario::stats_every

use std::fmt::Write as _;
use std::io;
use std::path::Path;
use std::time::Duration;

use crate::report::Report;
use crate::spec::HostClass;

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

/// One table being written: its columns, then its rows as CSV.
pub(crate) struct Table {
    name: &'static str,
    cols: Vec<(String, Col)>,
    csv: String,
}

impl Table {
    pub fn new(name: &'static str, cols: &[(&str, Col)]) -> Table {
        let cols: Vec<(String, Col)> = cols.iter().map(|&(n, c)| (n.to_owned(), c)).collect();
        let mut csv = String::new();
        let header: Vec<&str> = cols.iter().map(|(n, _)| n.as_str()).collect();
        csv.push_str(&header.join(","));
        csv.push('\n');
        Table { name, cols, csv }
    }

    /// Appends a row of integers.
    pub fn ints(&mut self, row: &[i64]) {
        for (i, x) in row.iter().enumerate() {
            if i > 0 {
                self.csv.push(',');
            }
            let _ = write!(self.csv, "{x}");
        }
        self.csv.push('\n');
    }

    /// Appends a row already formatted as CSV fields.
    pub fn fields(&mut self, row: &[String]) {
        self.csv.push_str(&row.join(","));
        self.csv.push('\n');
    }

    /// Writes `<name>.csv` and `<name>.schema.json` into `dir`.
    pub fn write(&self, dir: &Path) -> io::Result<()> {
        std::fs::write(dir.join(format!("{}.csv", self.name)), &self.csv)?;
        let cols: Vec<String> = self
            .cols
            .iter()
            .map(|(n, c)| format!("{{\"name\":\"{n}\",\"type\":\"{}\"}}", c.name()))
            .collect();
        std::fs::write(
            dir.join(format!("{}.schema.json", self.name)),
            format!("{{\"columns\":[{}]}}\n", cols.join(",")),
        )
    }
}

/// Text for a CSV field: names here carry no commas, quotes or newlines.
fn text(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, ',' | '"' | '\n' | '\r'))
        .collect()
}

impl Report {
    /// Writes the run's tables for its first swarm into `dir` (see the
    /// [module docs](self)); `bin` is [`crate::Scenario::wire_bins`]'s
    /// width, for the meta file. Without transfer samples there are no
    /// `seconds` or `viewer_seconds`.
    pub fn write_tables(&self, dir: &Path, bin: Duration) -> io::Result<()> {
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
            let mut t = Table::new("seconds", &cols);
            let mut row = Vec::with_capacity(cols.len());
            for k in 0..s.len() {
                row.clear();
                row.push(k as i64);
                row.extend(s.cols.iter().map(|(_, v)| v.get(k).copied().unwrap_or(-1)));
                t.ints(&row);
            }
            t.write(dir)?;
            let mut t = Table::new(
                "viewer_seconds",
                &[
                    ("viewer", Col::Int),
                    ("sec", Col::Int),
                    ("lag_ms", Col::Int),
                    ("frames", Col::Int),
                ],
            );
            for (i, (lag, frames)) in s.lag.iter().zip(&s.frames).enumerate() {
                for (k, (l, f)) in lag.iter().zip(frames).enumerate() {
                    t.ints(&[i as i64, k as i64, *l, *f]);
                }
            }
            t.write(dir)?;
        }
        let mut t = Table::new(
            "reads",
            &[
                ("viewer", Col::Int),
                ("t_ms", Col::Int),
                ("seq", Col::Int),
                ("lag_ms", Col::Int),
            ],
        );
        for (i, v) in viewers.iter().enumerate() {
            for &(seq, lag) in &v.stats.lags {
                let at = self.written_at(swarm, seq) + Duration::from_millis(u64::from(lag));
                let t_ms = match at.checked_sub(start) {
                    Some(d) => d.as_millis() as i64,
                    None => -(start.saturating_sub(at).as_millis() as i64),
                };
                t.ints(&[i as i64, t_ms, seq as i64, i64::from(lag)]);
            }
        }
        t.write(dir)?;
        let mut t = Table::new(
            "bins",
            &[
                ("bin", Col::Int),
                ("t_ms", Col::Int),
                ("seed_up_bytes", Col::Int),
                ("seed_up_packets", Col::Int),
                ("seed_small_packets", Col::Int),
                ("viewer_up_bytes", Col::Int),
                ("viewer_up_packets", Col::Int),
                ("viewer_small_packets", Col::Int),
            ],
        );
        let width = bin.as_millis() as i64;
        let edges = self.wire_bins.iter();
        for (k, (a, b)) in edges.clone().zip(edges.skip(1)).enumerate() {
            let mut row = vec![k as i64, k as i64 * width];
            row.extend(b.iter().zip(a).map(|(x, y)| x.saturating_sub(*y) as i64));
            t.ints(&row);
        }
        t.write(dir)?;
        let mut t = Table::new(
            "viewers",
            &[
                ("viewer", Col::Int),
                ("host", Col::Int),
                ("name", Col::Text),
                ("hostile", Col::Bool),
                ("up", Col::Int),
                ("clock_ms", Col::Int),
            ],
        );
        for (i, v) in viewers.iter().enumerate() {
            let hostile = matches!(
                self.scenario.hosts.get(v.host).map(|h| h.class),
                Some(HostClass::Hostile(_))
            );
            let clock = self
                .hosts
                .get(v.host)
                .map_or(0, |h| h.clock_offset.as_millis());
            t.fields(&[
                i.to_string(),
                v.host.to_string(),
                text(&v.name),
                hostile.to_string(),
                self.upload_cap(v.host).map_or(-1, |c| c as i64).to_string(),
                clock.to_string(),
            ]);
        }
        t.write(dir)?;
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
        let _ = write!(meta, ",\"viewers\":{},\"bin_ms\":{width}}}", viewers.len());
        meta.push('\n');
        std::fs::write(dir.join("series_meta.json"), meta)
    }
}
