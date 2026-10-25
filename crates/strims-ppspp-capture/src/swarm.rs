// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Several nodes' dumps as one view: every node's [export](crate::export)
//! rows in time order, each naming its node, with what ties a row to
//! another node's added to it. The dumps' clocks must agree: a
//! simulation's nodes share one, and so do a host's containers.
//!
//! A node's files are those whose header names it, read in the order of
//! their `file` index. Rows at the same time come in the order of the
//! nodes' names, a sender's before a receiver's.
//!
//! Two kinds of rows gain fields:
//!
//! | row | fields | from |
//! |---|---|---|
//! | DATA in | `sent`, the time the sender wrote it; `transit`, ns since; `sent_ann`, `push` or `answer` | the sender's DATA out row of the same swarm and bin to this node |
//! | `Declined`, `ShedBy` | `by`, the parent's row | the parent's latest `Rejected` (for `Declined`) or `Shed` (for `ShedBy`) of the same swarm and stream about this node |
//!
//! Each sender's or parent's row ties to one row at most, the earliest
//! after it, within [`JOIN_WINDOW`]. A child's `Declined` for a start too
//! far ahead is its own, and finds no row.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::PathBuf;

use crate::export::{Dump, ExportStats, Join, RowSink};
use crate::reader::{Error, Reader};

/// How long a sender's or a parent's row waits for its other side.
pub const JOIN_WINDOW: i64 = 30_000_000_000;
/// Inserts between sweeps of the rows that waited past [`JOIN_WINDOW`].
const SWEEP_EVERY: u64 = 1 << 14;

/// What a swarm view did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SwarmStats {
    pub nodes: u64,
    pub rows: u64,
    pub frames: u64,
    /// Records skipped, and errors that ended a file.
    pub errors: u64,
    /// DATA in rows tied to their DATA out.
    pub data_joined: u64,
    /// `Declined` and `ShedBy` rows tied to their parent's decision.
    pub decisions_joined: u64,
}

struct Pending {
    t: i64,
    json: String,
    join: Option<Join>,
}

#[derive(Default)]
struct Queue(VecDeque<Pending>);

impl RowSink for Queue {
    fn wants_joins(&self) -> bool {
        true
    }

    fn row(&mut self, t: i64, json: &str, join: Option<Join>) -> io::Result<()> {
        self.0.push_back(Pending {
            t,
            json: json.to_owned(),
            join,
        });
        Ok(())
    }
}

/// One node's files, read in order.
struct Source {
    files: VecDeque<PathBuf>,
    reader: Option<Reader<BufReader<File>>>,
    dump: Dump<Queue>,
}

impl Source {
    /// The node's next row; `None` once its files are read.
    fn next(&mut self, on_error: &mut dyn FnMut(&Error)) -> io::Result<Option<Pending>> {
        loop {
            if let Some(p) = self.dump.sink.0.pop_front() {
                return Ok(Some(p));
            }
            let Some(r) = self.reader.as_mut() else {
                let Some(path) = self.files.pop_front() else {
                    return Ok(None);
                };
                match Reader::new(BufReader::new(File::open(&path)?)) {
                    Ok(r) => {
                        self.dump.set_version(r.header().version);
                        self.reader = Some(r);
                    }
                    Err(e) => {
                        self.dump.stats.errors += 1;
                        on_error(&e);
                    }
                }
                continue;
            };
            match r.next_record() {
                Some(Ok(e)) => self.dump.record(e.time.unix_nanos(), &e.record)?,
                Some(Err(e)) => {
                    self.dump.stats.errors += 1;
                    on_error(&e);
                }
                None => {
                    self.reader = None;
                    self.dump.flush()?;
                }
            }
        }
    }
}

type DataKey = (String, String, String, u64);
type DecisionKey = (String, String, String, u64, &'static str);

/// Rows waiting for their other side.
#[derive(Default)]
struct Joins {
    /// (sender, receiver, swarm, bin) to when it was sent and as what.
    data: BTreeMap<DataKey, VecDeque<(i64, Option<&'static str>)>>,
    /// (parent, child, swarm, stream, what the child hears) to the
    /// parent's rows.
    decisions: BTreeMap<DecisionKey, VecDeque<(i64, String)>>,
    inserts: u64,
}

impl Joins {
    /// Notes a sender's or a parent's row, or returns the fields a
    /// receiver's or a child's row gains.
    fn apply(&mut self, node: &str, p: &Pending, stats: &mut SwarmStats) -> Option<String> {
        let old = p.t.saturating_sub(JOIN_WINDOW);
        match p.join.as_ref()? {
            Join::Sent {
                to,
                swarm,
                bin,
                ann,
            } => {
                self.inserted(p.t);
                self.data
                    .entry((node.to_owned(), to.clone(), swarm.clone(), *bin))
                    .or_default()
                    .push_back((p.t, *ann));
                None
            }
            Join::Received { from, swarm, bin } => {
                let key = (from.clone(), node.to_owned(), swarm.clone(), *bin);
                let q = self.data.get_mut(&key)?;
                let (sent, ann) = pop_after(q, old)?;
                if q.is_empty() {
                    self.data.remove(&key);
                }
                stats.data_joined += 1;
                let mut s = format!(",\"sent\":{sent},\"transit\":{}", p.t.saturating_sub(sent));
                if let Some(a) = ann {
                    s.push_str(&format!(",\"sent_ann\":\"{a}\""));
                }
                Some(s)
            }
            Join::Decided {
                child,
                swarm,
                stream,
                heard,
            } => {
                self.inserted(p.t);
                self.decisions
                    .entry((
                        node.to_owned(),
                        child.clone(),
                        swarm.clone(),
                        *stream,
                        heard,
                    ))
                    .or_default()
                    .push_back((p.t, p.json.clone()));
                None
            }
            Join::Heard {
                parent,
                swarm,
                stream,
                kind,
            } => {
                let key = (
                    parent.clone(),
                    node.to_owned(),
                    swarm.clone(),
                    *stream,
                    *kind,
                );
                let q = self.decisions.get_mut(&key)?;
                let (_, json) = pop_after(q, old)?;
                if q.is_empty() {
                    self.decisions.remove(&key);
                }
                stats.decisions_joined += 1;
                Some(format!(",\"by\":{json}}}"))
            }
        }
    }

    fn inserted(&mut self, now: i64) {
        self.inserts += 1;
        if self.inserts.is_multiple_of(SWEEP_EVERY) {
            let old = now.saturating_sub(JOIN_WINDOW);
            self.data.retain(|_, q| {
                q.retain(|x| x.0 >= old);
                !q.is_empty()
            });
            self.decisions.retain(|_, q| {
                q.retain(|x| x.0 >= old);
                !q.is_empty()
            });
        }
    }
}

/// The earliest entry at or after `old`, dropping those before it.
fn pop_after<T>(q: &mut VecDeque<(i64, T)>, old: i64) -> Option<(i64, T)> {
    while q.front().is_some_and(|x| x.0 < old) {
        q.pop_front();
    }
    q.pop_front()
}

/// Writes the rows of every node's dumps in `paths` as JSON lines, in
/// time order. Records that don't decode, and files that aren't dumps, go
/// to `on_error` and are skipped. A file that can't be opened or read, and
/// write errors, fail it.
pub fn swarm<W: Write>(
    paths: &[PathBuf],
    out: &mut W,
    on_error: &mut dyn FnMut(&Error),
) -> io::Result<SwarmStats> {
    // node to (file index, path)
    let mut nodes: BTreeMap<Vec<u8>, Vec<(u64, PathBuf)>> = BTreeMap::new();
    let mut stats = SwarmStats::default();
    for path in paths {
        match Reader::new(BufReader::new(File::open(path)?)) {
            Ok(r) => nodes
                .entry(r.header().node.clone())
                .or_default()
                .push((r.header().file, path.clone())),
            Err(e) => {
                stats.errors += 1;
                on_error(&e);
            }
        }
    }
    let mut sources: Vec<Source> = nodes
        .into_iter()
        .map(|(node, mut files)| {
            files.sort();
            Source {
                files: files.into_iter().map(|f| f.1).collect(),
                reader: None,
                dump: Dump::new(Queue::default(), &node, crate::VERSION),
            }
        })
        .collect();
    stats.nodes = sources.len() as u64;

    let mut heads: Vec<Option<Pending>> = Vec::with_capacity(sources.len());
    // (time, a receiver's or child's row after a sender's, node, sequence)
    let mut heap = BinaryHeap::new();
    let mut seq = 0u64;
    let order = |p: &Pending| {
        u8::from(matches!(
            p.join,
            Some(Join::Received { .. } | Join::Heard { .. })
        ))
    };
    for (i, s) in sources.iter_mut().enumerate() {
        let p = s.next(on_error)?;
        if let Some(p) = &p {
            heap.push(Reverse((p.t, order(p), i, seq)));
            seq += 1;
        }
        heads.push(p);
    }
    let mut joins = Joins::default();
    while let Some(Reverse((_, _, i, _))) = heap.pop() {
        let (Some(p), Some(src)) = (heads.get_mut(i).and_then(Option::take), sources.get_mut(i))
        else {
            continue;
        };
        let extra = joins.apply(src.dump.node(), &p, &mut stats);
        out.write_all(p.json.as_bytes())?;
        if let Some(x) = extra {
            out.write_all(x.as_bytes())?;
        }
        out.write_all(b"}\n")?;
        stats.rows += 1;
        let next = src.next(on_error)?;
        if let Some(n) = &next {
            heap.push(Reverse((n.t, order(n), i, seq)));
            seq += 1;
        }
        if let Some(h) = heads.get_mut(i) {
            *h = next;
        }
    }
    for s in &sources {
        let ExportStats { frames, errors, .. } = s.dump.stats;
        stats.frames += frames;
        stats.errors += errors;
    }
    Ok(stats)
}
