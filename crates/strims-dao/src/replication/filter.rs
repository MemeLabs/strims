// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Which received events to apply.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use strims_proto::dao::v1::VersionVector;
use strims_proto::replication::v1::{Event, EventLog};

use crate::vv;

/// A record's identity across replicas: its namespace and id.
pub type RecordKey = (i64, u64);

fn key(e: &Event) -> RecordKey {
    (e.namespace, e.id)
}

fn version(e: &Event) -> VersionVector {
    e.version.clone().unwrap_or_default()
}

/// The events of `incoming` to apply, given the records this replica
/// has already `deleted`:
///
/// - a delete wins over every write to its record, received or stored,
///   since no write follows a record's deletion;
/// - of a record's writes, those another write supersedes are dropped,
///   and concurrent ones are all kept, to be resolved against the stored
///   record in order.
///
/// Events come out grouped by record, records in order of first
/// appearance.
pub fn filter_incoming(deleted: &BTreeSet<RecordKey>, incoming: &[EventLog]) -> Vec<Event> {
    let deleting: BTreeSet<RecordKey> = incoming
        .iter()
        .flat_map(|l| &l.events)
        .filter(|e| e.delete)
        .map(key)
        .collect();

    let mut order: Vec<RecordKey> = Vec::new();
    let mut frontier: BTreeMap<RecordKey, Vec<Event>> = BTreeMap::new();
    for e in incoming.iter().flat_map(|l| &l.events) {
        let k = key(e);
        if deleted.contains(&k) || deleting.contains(&k) != e.delete {
            continue;
        }
        let events = frontier.entry(k).or_insert_with(|| {
            order.push(k);
            Vec::new()
        });
        if e.delete {
            if events.is_empty() {
                events.push(e.clone());
            }
            continue;
        }
        let v = version(e);
        if events.iter().any(|w| {
            matches!(
                vv::compare(&version(w), &v),
                Some(Ordering::Greater | Ordering::Equal)
            )
        }) {
            continue;
        }
        events.retain(|w| vv::compare(&version(w), &v) != Some(Ordering::Less));
        events.push(e.clone());
    }

    order
        .into_iter()
        .flat_map(|k| frontier.remove(&k).unwrap_or_default())
        .collect()
}

/// The records `logs` write or delete.
pub fn keys(logs: &[EventLog]) -> BTreeSet<RecordKey> {
    logs.iter().flat_map(|l| &l.events).map(key).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    fn ev(id: u64, delete: bool, entries: &[(u64, u64)]) -> Event {
        Event {
            namespace: 1,
            id,
            version: Some(VersionVector {
                value: entries.iter().copied().collect(),
                updated_at: 0,
            }),
            delete,
            record: vec![id as u8],
        }
    }

    fn log(events: Vec<Event>) -> EventLog {
        EventLog {
            events,
            ..Default::default()
        }
    }

    fn summary(es: &[Event]) -> Vec<(u64, bool, u64, u64)> {
        es.iter()
            .map(|e| {
                let v = version(e);
                (e.id, e.delete, vv::get(&v, 1), vv::get(&v, 2))
            })
            .collect()
    }

    #[test]
    fn newer_writes_supersede_older_ones() {
        let out = filter_incoming(
            &BTreeSet::new(),
            &[
                log(vec![ev(1, false, &[(1, 1)]), ev(2, false, &[(1, 1)])]),
                log(vec![ev(1, false, &[(1, 2)])]),
                log(vec![ev(1, false, &[(1, 1)])]),
            ],
        );
        assert_eq!(summary(&out), [(1, false, 2, 0), (2, false, 1, 0)]);
    }

    #[test]
    fn concurrent_writes_are_all_kept() {
        let out = filter_incoming(
            &BTreeSet::new(),
            &[log(vec![
                ev(1, false, &[(1, 1)]),
                ev(1, false, &[(2, 1)]),
                ev(1, false, &[(1, 1), (2, 1)]),
                ev(1, false, &[(1, 2)]),
            ])],
        );
        assert_eq!(summary(&out), [(1, false, 1, 1), (1, false, 2, 0)]);
    }

    #[test]
    fn deletes_win() {
        let deleted = BTreeSet::from([(1, 3)]);
        let out = filter_incoming(
            &deleted,
            &[
                log(vec![ev(1, false, &[(2, 1)]), ev(3, false, &[(2, 9)])]),
                log(vec![ev(1, true, &[(2, 2)]), ev(3, true, &[(2, 10)])]),
                log(vec![ev(1, false, &[(2, 3)])]),
            ],
        );
        assert_eq!(summary(&out), [(1, true, 0, 2)]);
    }
}
