// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The checkpoints of a profile's live replicas.

use std::collections::BTreeMap;

use strims_proto::dao::v1::VersionVector;
use strims_proto::replication::v1::Checkpoint;

use crate::vv;

/// The latest checkpoint of each live replica. Deleted replicas drop
/// out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CheckpointMap(BTreeMap<u64, Checkpoint>);

impl CheckpointMap {
    /// The map of `checkpoints`.
    pub fn new(checkpoints: impl IntoIterator<Item = Checkpoint>) -> CheckpointMap {
        let mut m = CheckpointMap::default();
        for c in checkpoints {
            m.set(c);
        }
        m
    }

    /// Records `c` as its replica's checkpoint.
    pub fn set(&mut self, c: Checkpoint) {
        if c.deleted {
            self.0.remove(&c.id);
        } else {
            self.0.insert(c.id, c);
        }
    }

    /// The live replicas' checkpoints.
    pub fn iter(&self) -> impl Iterator<Item = &Checkpoint> {
        self.0.values()
    }

    /// What every live replica has seen: the pointwise minimum of their
    /// checkpoints over the replicas all of them know.
    pub fn min_version(&self) -> VersionVector {
        let mut versions = self
            .0
            .values()
            .map(|c| c.version.clone().unwrap_or_default());
        let Some(first) = versions.next() else {
            return VersionVector::default();
        };
        let mut min = vv::without_time(&first);
        for v in versions {
            vv::downgrade(&mut min, &v);
        }
        min
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    fn cp(id: u64, entries: &[(u64, u64)], deleted: bool) -> Checkpoint {
        Checkpoint {
            id,
            version: Some(VersionVector {
                value: entries.iter().copied().collect(),
                updated_at: 9,
            }),
            deleted,
        }
    }

    #[test]
    fn min_version_covers_live_replicas() {
        let mut m = CheckpointMap::new([
            cp(1, &[(1, 5), (2, 3)], false),
            cp(2, &[(1, 4), (2, 6), (3, 1)], false),
        ]);
        assert_eq!(m.min_version().value, [(1, 4), (2, 3)].into());
        m.set(cp(3, &[(3, 9)], false));
        assert!(m.min_version().value.is_empty());
        m.set(cp(3, &[], true));
        assert_eq!(m.min_version().value, [(1, 4), (2, 3)].into());
        assert_eq!(
            CheckpointMap::default().min_version(),
            VersionVector::default()
        );
    }
}
