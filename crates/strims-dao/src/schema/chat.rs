// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Chat settings and whispers that follow the profile across devices.

use futures_util::future::LocalBoxFuture;
use strims_kv::Kv;
use strims_proto::chat::v1::{
    MessageState, UiConfig, UiConfigHighlight, UiConfigIgnore, UiConfigTag, WhisperRecord,
    WhisperThread,
};

use crate::error::Error;
use crate::events::Record;
use crate::key::Namespace;
use crate::store::WriteTx;
use crate::table::{
    Index, Replication, Singleton, Table, UniqueConflict, resolve_last_written, upgrade_version,
};

/// The chat UI settings. A device's settings replicate only while both
/// it and the receiving device have replication enabled.
pub struct ChatUiConfig;

impl Singleton for ChatUiConfig {
    type Record = UiConfig;
    const NS: Namespace = Namespace(6008);
    const REPLICATION: Option<Replication<UiConfig>> = Some(Replication {
        merge: |next, local| {
            keep_local_unless_replicating(next, local, |mut n, l| {
                upgrade_version(&mut n, l);
                n
            })
        },
        resolve: |next, local| keep_local_unless_replicating(next, local, resolve_last_written),
        ..Replication::DEFAULT
    });

    fn observe(r: &UiConfig) -> Option<Record> {
        Some(Record::ChatUiConfig(r.clone()))
    }
}

fn keep_local_unless_replicating(
    next: UiConfig,
    local: &UiConfig,
    f: fn(UiConfig, &UiConfig) -> UiConfig,
) -> UiConfig {
    if next.replicate && local.replicate {
        return f(next, local);
    }
    let mut kept = local.clone();
    upgrade_version(&mut kept, &next);
    kept
}

macro_rules! peer_key_table {
    ($(#[$doc:meta])* $name:ident, $record:ty, $variant:ident, $ns:expr, $key_ns:expr) => {
        $(#[$doc])*
        pub struct $name;

        impl $name {
            /// Records by peer key; at most one per key.
            pub const BY_PEER_KEY: Index<$record> = Index::unique(Namespace($key_ns), |r: &$record| r.peer_key.clone());
        }

        impl Table for $name {
            type Record = $record;
            const NS: Namespace = Namespace($ns);
            const INDEXES: &'static [Index<$record>] = &[$name::BY_PEER_KEY];
            const REPLICATION: Option<Replication<$record>> = Some(Replication::DEFAULT);

            fn id(r: &$record) -> u64 {
                r.id
            }

            fn observe(r: &$record) -> Option<Record> {
                Some(Record::$variant(r.clone()))
            }
        }
    };
}

peer_key_table!(
    /// Highlighted chat users.
    ChatUiConfigHighlights,
    UiConfigHighlight,
    ChatUiConfigHighlight,
    6018,
    6019
);
peer_key_table!(
    /// Tagged chat users.
    ChatUiConfigTags,
    UiConfigTag,
    ChatUiConfigTag,
    6020,
    6021
);
peer_key_table!(
    /// Ignored chat users.
    ChatUiConfigIgnores,
    UiConfigIgnore,
    ChatUiConfigIgnore,
    6022,
    6023
);

/// Whisper conversations, one per peer. Deleting a conversation locally
/// deletes its messages. Concurrent writes to a conversation resolve to
/// the later one.
pub struct WhisperThreads;

impl WhisperThreads {
    /// Conversations by peer key. A new conversation with a peer absorbs
    /// the existing one.
    pub const BY_PEER_KEY: Index<WhisperThread> =
        Index::unique(Namespace(6016), |r| r.peer_key.clone());
}

impl Table for WhisperThreads {
    type Record = WhisperThread;
    const NS: Namespace = Namespace(6015);
    const INDEXES: &'static [Index<WhisperThread>] = &[WhisperThreads::BY_PEER_KEY];
    const REPLICATION: Option<Replication<WhisperThread>> = Some(Replication::DEFAULT);

    fn id(r: &WhisperThread) -> u64 {
        r.id
    }

    fn observe(r: &WhisperThread) -> Option<Record> {
        Some(Record::WhisperThread(r.clone()))
    }

    fn on_delete<'a, K: Kv>(
        tx: &'a mut WriteTx<K>,
        p: &'a WhisperThread,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            if tx.replica_id().is_none() {
                return Ok(());
            }
            for id in
                WhisperRecords::index_ids(tx, &WhisperRecords::BY_PEER_KEY, &p.peer_key).await?
            {
                WhisperRecords::delete(tx, id).await?;
            }
            Ok(())
        })
    }

    fn on_unique_conflict(
        _: &Index<WhisperThread>,
        m: &mut WhisperThread,
        existing: &WhisperThread,
    ) -> UniqueConflict {
        fold_thread(m, existing);
        UniqueConflict::Replace
    }
}

/// Folds `p` into `m`: the later last message, and unread if either is.
fn fold_thread(m: &mut WhisperThread, p: &WhisperThread) {
    upgrade_version(m, p);
    if (p.last_message_time, p.last_message_id) > (m.last_message_time, m.last_message_id) {
        m.alias = p.alias.clone();
        m.last_message_time = p.last_message_time;
        m.last_message_id = p.last_message_id;
    }
    m.has_unread |= p.has_unread;
}

/// Whisper messages.
pub struct WhisperRecords;

impl WhisperRecords {
    /// Messages by peer key.
    pub const BY_PEER_KEY: Index<WhisperRecord> =
        Index::new(Namespace(6013), |r| r.peer_key.clone());
    /// Messages by state.
    pub const BY_STATE: Index<WhisperRecord> = Index::new(Namespace(6014), |r| state_key(r.state));
    /// Unread messages by peer key.
    pub const UNREAD_BY_PEER_KEY: Index<WhisperRecord> =
        Index::new(Namespace(6017), |r: &WhisperRecord| r.peer_key.clone())
            .when(|r| r.state == MessageState::Unread as i32);
    /// Messages by the sender's message id and peer key; at most one per
    /// key.
    pub const BY_PEER_MESSAGE_ID: Index<WhisperRecord> =
        Index::unique(Namespace(6025), peer_message_id_key).when(|r| r.peer_message_id != 0);
}

/// The key messages in `state` are indexed under.
pub fn state_key(state: i32) -> Vec<u8> {
    (state as u32).to_be_bytes().to_vec()
}

/// The key a message is indexed under by its sender's message id.
pub fn peer_message_id_key(r: &WhisperRecord) -> Vec<u8> {
    let mut k = r.peer_message_id.to_be_bytes().to_vec();
    k.extend_from_slice(&r.peer_key);
    k
}

impl Table for WhisperRecords {
    type Record = WhisperRecord;
    const NS: Namespace = Namespace(6012);
    const INDEXES: &'static [Index<WhisperRecord>] = &[
        WhisperRecords::BY_PEER_KEY,
        WhisperRecords::UNREAD_BY_PEER_KEY,
        WhisperRecords::BY_STATE,
        WhisperRecords::BY_PEER_MESSAGE_ID,
    ];
    const REPLICATION: Option<Replication<WhisperRecord>> = Some(Replication::DEFAULT);

    fn id(r: &WhisperRecord) -> u64 {
        r.id
    }

    fn observe(r: &WhisperRecord) -> Option<Record> {
        Some(Record::WhisperRecord(r.clone()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use strims_proto::dao::v1::VersionVector;

    use super::*;

    fn thread(alias: &str, at: i64, last: i64, unread: bool) -> WhisperThread {
        WhisperThread {
            id: 1,
            version: Some(VersionVector {
                value: [(at as u64, 1)].into(),
                updated_at: at,
            }),
            peer_key: b"peer".to_vec(),
            alias: alias.into(),
            unread_count: 0,
            last_message_time: last,
            last_message_id: last as u64,
            has_unread: unread,
        }
    }

    #[test]
    fn a_new_thread_absorbs_the_old_one_for_its_peer() {
        let mut m = thread("new", 20, 100, false);
        fold_thread(&mut m, &thread("old", 10, 500, true));
        assert_eq!(m.alias, "old");
        assert_eq!(m.last_message_time, 500);
        assert!(m.has_unread);
        assert_eq!(m.version.unwrap().value.len(), 2);
    }

    #[test]
    fn ui_config_replicates_only_between_replicating_devices() {
        let rep = ChatUiConfig::REPLICATION.unwrap();
        let cfg = |replicate: bool, at: i64| UiConfig {
            replicate,
            show_time: at > 10,
            version: Some(VersionVector {
                value: [(at as u64, 1)].into(),
                updated_at: at,
            }),
            ..Default::default()
        };
        let kept = (rep.merge)(cfg(true, 20), &cfg(false, 5));
        assert!(!kept.replicate);
        assert_eq!(kept.version.unwrap().value.len(), 2);
        let taken = (rep.merge)(cfg(true, 20), &cfg(true, 5));
        assert!(taken.show_time);
        assert_eq!(
            (rep.resolve)(cfg(true, 20), &cfg(true, 5)),
            (rep.resolve)(cfg(true, 5), &cfg(true, 20))
        );
    }

    #[test]
    fn whisper_index_keys() {
        assert_eq!(state_key(2), [0, 0, 0, 2]);
        let r = WhisperRecord {
            peer_message_id: 0x0102,
            peer_key: b"k".to_vec(),
            ..Default::default()
        };
        assert_eq!(peer_message_id_key(&r), [0, 0, 0, 0, 0, 0, 1, 2, b'k']);
        assert!(!(WhisperRecords::BY_PEER_MESSAGE_ID.condition)(
            &WhisperRecord::default()
        ));
        assert!((WhisperRecords::UNREAD_BY_PEER_KEY.condition)(
            &WhisperRecord::default()
        ));
    }
}
