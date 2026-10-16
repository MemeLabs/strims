// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Which logs a replica sends another, and in what order.

use std::collections::{BTreeMap, BTreeSet};

use prost::Message;
use strims_proto::dao::v1::VersionVector;
use strims_proto::replication::v1::{Checkpoint, Event, EventLog};

use super::filter::RecordKey;
use crate::vv;

/// The replica that wrote a log.
pub fn origin(l: &EventLog) -> u64 {
    l.checkpoint.as_ref().map_or(0, |c| c.id)
}

/// A log's position in its origin's sequence of logs.
pub fn local_version(l: &EventLog) -> u64 {
    let c = l.checkpoint.as_ref();
    c.and_then(|c| c.version.as_ref())
        .map_or(0, |v| vv::get(v, origin(l)))
}

/// Whether a replica at `checkpoint` may lack log `l`. The log at the
/// checkpoint's own position counts, so a log split across messages is
/// sent again whole.
pub fn after(l: &EventLog, checkpoint: &VersionVector) -> bool {
    match checkpoint.value.get(&origin(l)) {
        None => true,
        Some(&v) => local_version(l) >= v,
    }
}

/// Of each origin's logs, keeps only the newest event for each record,
/// and drops the logs left empty.
pub fn compress(logs: Vec<EventLog>) -> Vec<EventLog> {
    let mut by_origin: BTreeMap<u64, Vec<EventLog>> = BTreeMap::new();
    for l in logs {
        by_origin.entry(origin(&l)).or_default().push(l);
    }
    let mut out = Vec::new();
    for (_, mut logs) in by_origin {
        logs.sort_by_key(|l| std::cmp::Reverse(local_version(l)));
        let mut seen: BTreeSet<RecordKey> = BTreeSet::new();
        for mut l in logs {
            let mut kept: Vec<Event> = Vec::new();
            for e in l.events.into_iter().rev() {
                if seen.insert((e.namespace, e.id)) {
                    kept.push(e);
                }
            }
            kept.reverse();
            l.events = kept;
            if !l.events.is_empty() {
                out.push(l);
            }
        }
    }
    out
}

/// Orders logs so that each follows every log its checkpoint shows its
/// origin had seen: by the sum of the checkpoint's counts, then origin and
/// position. A receiver that stops partway then holds no log without its
/// predecessors.
pub fn causal_order(logs: &mut [EventLog]) {
    logs.sort_by_key(|l| {
        let sum = l
            .checkpoint
            .as_ref()
            .and_then(|c| c.version.as_ref())
            .map_or(0u64, |v| {
                v.value.values().fold(0u64, |s, &n| s.saturating_add(n))
            });
        (sum, origin(l), local_version(l))
    });
}

/// The bytes `m` adds to a message as a repeated field entry.
pub fn entry_len(m: &impl Message) -> usize {
    let n = m.encoded_len();
    1 + prost::length_delimiter_len(n) + n
}

/// Splits ordered logs into consecutive groups of at most `budget` encoded
/// bytes. A log larger than `budget` is split into parts that share its id
/// and checkpoint.
pub fn chunk_logs(logs: Vec<EventLog>, budget: usize) -> Vec<Vec<EventLog>> {
    let mut out: Vec<Vec<EventLog>> = Vec::new();
    let mut size = 0;
    for l in logs {
        for part in split_log(l, budget) {
            let len = entry_len(&part);
            match out.last_mut() {
                Some(last) if size + len <= budget => last.push(part),
                _ => {
                    size = 0;
                    out.push(vec![part]);
                }
            }
            size += len;
        }
    }
    out
}

fn split_log(l: EventLog, budget: usize) -> Vec<EventLog> {
    if entry_len(&l) <= budget || l.events.len() <= 1 {
        return vec![l];
    }
    let header = EventLog {
        id: l.id,
        checkpoint: l.checkpoint.clone(),
        events: Vec::new(),
    };
    // The part's own length prefix grows by at most 2 bytes up to 2 MiB.
    let base = entry_len(&header) + 2;
    let mut parts: Vec<EventLog> = Vec::new();
    let mut size = base;
    for e in l.events {
        let len = entry_len(&e);
        match parts.last_mut() {
            Some(p) if size + len <= budget => p.events.push(e),
            _ => {
                let mut p = header.clone();
                p.events.push(e);
                parts.push(p);
                size = base;
            }
        }
        size += len;
    }
    parts
}

/// A bootstrap: every record, every stored log and every checkpoint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bootstrap {
    /// The records, as events.
    pub events: Vec<Event>,
    /// The stored logs, without records.
    pub logs: Vec<EventLog>,
    /// The checkpoints.
    pub checkpoints: Vec<Checkpoint>,
}

/// Splits a bootstrap into parts of at most `budget` encoded bytes. Only
/// the last part carries the checkpoints, so a receiver that stops partway
/// records none of the bootstrap's progress.
pub fn chunk_bootstrap(b: Bootstrap, budget: usize) -> Vec<Bootstrap> {
    enum Item {
        Event(Event),
        Log(EventLog),
    }
    let room = budget.saturating_sub(b.checkpoints.iter().map(entry_len).sum());
    let mut out = vec![Bootstrap::default()];
    let mut size = 0;
    let items = b
        .events
        .into_iter()
        .map(Item::Event)
        .chain(b.logs.into_iter().map(Item::Log));
    for item in items {
        let len = match &item {
            Item::Event(e) => entry_len(e),
            Item::Log(l) => entry_len(l),
        };
        if size > 0 && size + len > room {
            out.push(Bootstrap::default());
            size = 0;
        }
        size += len;
        if let Some(last) = out.last_mut() {
            match item {
                Item::Event(e) => last.events.push(e),
                Item::Log(l) => last.logs.push(l),
            }
        }
    }
    if let Some(last) = out.last_mut() {
        last.checkpoints = b.checkpoints;
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    fn log(origin: u64, entries: &[(u64, u64)], events: &[u64]) -> EventLog {
        EventLog {
            id: origin * 100 + entries.iter().find(|e| e.0 == origin).map_or(0, |e| e.1),
            checkpoint: Some(Checkpoint {
                id: origin,
                version: Some(VersionVector {
                    value: entries.iter().copied().collect(),
                    updated_at: 0,
                }),
                deleted: false,
            }),
            events: events
                .iter()
                .map(|&id| Event {
                    namespace: 1,
                    id,
                    record: vec![0; 10],
                    ..Default::default()
                })
                .collect(),
        }
    }

    #[test]
    fn after_includes_the_boundary() {
        let cp = VersionVector {
            value: [(1, 5)].into(),
            updated_at: 0,
        };
        assert!(!after(&log(1, &[(1, 4)], &[]), &cp));
        assert!(after(&log(1, &[(1, 5)], &[]), &cp));
        assert!(after(&log(2, &[(2, 1)], &[]), &cp));
    }

    #[test]
    fn compress_keeps_each_records_newest_event_per_origin() {
        let out = compress(vec![
            log(1, &[(1, 1)], &[1, 2]),
            log(1, &[(1, 2)], &[1, 3, 1]),
            log(1, &[(1, 3)], &[3]),
            log(2, &[(2, 1)], &[1]),
        ]);
        let shape: Vec<(u64, Vec<u64>)> = out
            .iter()
            .map(|l| (l.id, l.events.iter().map(|e| e.id).collect()))
            .collect();
        assert_eq!(
            shape,
            [
                (103, vec![3]),
                (102, vec![1]),
                (101, vec![2]),
                (201, vec![1])
            ]
        );
    }

    #[test]
    fn causal_order_puts_predecessors_first() {
        let mut logs = vec![
            log(2, &[(1, 2), (2, 1)], &[]),
            log(1, &[(1, 2)], &[]),
            log(1, &[(1, 1)], &[]),
            log(3, &[(3, 1)], &[]),
        ];
        causal_order(&mut logs);
        let ids: Vec<u64> = logs.iter().map(|l| l.id).collect();
        assert_eq!(ids, [101, 301, 102, 201]);
    }

    #[test]
    fn chunks_respect_the_budget() {
        let logs: Vec<EventLog> = (1..=20).map(|i| log(1, &[(1, i)], &[i, i + 100])).collect();
        let budget = 200;
        let chunks = chunk_logs(logs.clone(), budget);
        assert!(chunks.len() > 1);
        assert!(
            chunks
                .iter()
                .all(|c| c.iter().map(entry_len).sum::<usize>() <= budget)
        );
        assert_eq!(chunks.concat(), logs);

        let big = log(1, &[(1, 1)], &(1..=40).collect::<Vec<_>>());
        let parts = chunk_logs(vec![big.clone()], budget);
        assert!(
            parts
                .iter()
                .all(|c| c.iter().map(entry_len).sum::<usize>() <= budget)
        );
        let flat: Vec<EventLog> = parts.concat();
        assert!(flat.len() > 1);
        assert!(
            flat.iter()
                .all(|p| p.id == big.id && p.checkpoint == big.checkpoint)
        );
        let events: Vec<Event> = flat.into_iter().flat_map(|p| p.events).collect();
        assert_eq!(events, big.events);
    }

    #[test]
    fn bootstrap_checkpoints_ride_the_last_part() {
        let b = Bootstrap {
            events: (0..50)
                .map(|id| Event {
                    namespace: 1,
                    id,
                    record: vec![0; 20],
                    ..Default::default()
                })
                .collect(),
            logs: (1..=10).map(|i| log(1, &[(1, i)], &[i])).collect(),
            checkpoints: vec![Checkpoint {
                id: 1,
                ..Default::default()
            }],
        };
        let parts = chunk_bootstrap(b.clone(), 300);
        assert!(parts.len() > 2);
        assert!(parts.iter().rev().skip(1).all(|p| p.checkpoints.is_empty()));
        assert_eq!(parts.last().unwrap().checkpoints, b.checkpoints);
        assert!(parts.iter().all(|p| {
            let len: usize = p.events.iter().map(entry_len).sum::<usize>()
                + p.logs.iter().map(entry_len).sum::<usize>()
                + p.checkpoints.iter().map(entry_len).sum::<usize>();
            len <= 300
        }));
        let events: Vec<Event> = parts.iter().flat_map(|p| p.events.clone()).collect();
        assert_eq!(events, b.events);
        let logs: Vec<EventLog> = parts.into_iter().flat_map(|p| p.logs).collect();
        assert_eq!(logs, b.logs);
    }
}
