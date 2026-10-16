// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A profile store the Go dao wrote (`golden/store.hex`, from `go run
//! -tags golden ./crates/strims-dao/golden`): its sign-in and session
//! records open, its records read back, every key is the one this crate
//! derives, and the compressed delta matches.

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;

use futures_util::FutureExt;
use prost::Message;
use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_dao::auth::{self, ProfileKey, SessionToken};
use strims_dao::key::{Namespace, index_entry_key, parse_base36};
use strims_dao::replication::compressed_delta;
use strims_dao::schema::chat::{
    ChatUiConfig, ChatUiConfigHighlights, ChatUiConfigIgnores, ChatUiConfigTags, WhisperRecords,
    WhisperThreads,
};
use strims_dao::schema::network::{BootstrapClients, Networks};
use strims_dao::schema::profile::{Devices, Profile, ProfileIds};
use strims_dao::schema::system::{Checkpoints, EventLogs, StoreVersions};
use strims_dao::seal::StorageKey;
use strims_dao::{ProfileStore, ReadTx, Singleton, Table, lease};
use strims_kv::{Dump, MemoryKv};
use strims_proto::dao::v1::{Mutex, SecondaryIndexKey, VersionVector};
use strims_proto::replication::v1::{Event, EventLog};

const STORE: &str = include_str!("../golden/store.hex");

fn block<T>(f: impl Future<Output = T>) -> T {
    f.now_or_never()
        .expect("memory store operations are ready immediately")
}

struct Golden {
    now: Time,
    name: String,
    password: String,
    profile: ProfileKey,
    session_key: [u8; 32],
    session: SessionToken,
    dump: Dump,
    records: Vec<(i64, u64, Vec<u8>)>,
    delta_remote: VersionVector,
    delta: Vec<EventLog>,
}

fn u64_of(b: &[u8]) -> u64 {
    u64::from_be_bytes(b.try_into().unwrap())
}

fn load() -> Golden {
    let mut g = Golden {
        now: Time::EPOCH,
        name: String::new(),
        password: String::new(),
        profile: ProfileKey {
            id: 0,
            key: [0; 32],
        },
        session_key: [0; 32],
        session: SessionToken {
            eol: 0,
            token: vec![],
        },
        dump: Dump::new(),
        records: vec![],
        delta_remote: VersionVector::default(),
        delta: vec![],
    };
    for line in STORE.lines().filter(|l| !l.trim().is_empty()) {
        let mut parts = line.split(' ');
        let name = parts.next().unwrap();
        let v: Vec<Vec<u8>> = parts
            .map(|p| {
                if p == "=" {
                    vec![]
                } else {
                    hex::decode(p).unwrap()
                }
            })
            .collect();
        match name {
            "now" => g.now = Time::from_unix_nanos(u64_of(&v[0]) as i64),
            "auth" => {
                g.name = String::from_utf8(v[0].clone()).unwrap();
                g.password = String::from_utf8(v[1].clone()).unwrap();
                g.profile = ProfileKey::from_parts(u64_of(&v[2]), &v[3]).unwrap();
            }
            "session" => {
                g.session_key = v[0].as_slice().try_into().unwrap();
                g.session = SessionToken {
                    eol: u64_of(&v[1]),
                    token: v[2].clone(),
                };
            }
            "kv" => {
                g.dump
                    .entry(String::from_utf8(v[0].clone()).unwrap())
                    .or_default()
                    .insert(String::from_utf8(v[1].clone()).unwrap(), v[2].clone());
            }
            "record" => g
                .records
                .push((u64_of(&v[0]) as i64, u64_of(&v[1]), v[2].clone())),
            "delta_remote" => g.delta_remote = VersionVector::decode(v[0].as_slice()).unwrap(),
            "delta" => g.delta.push(EventLog::decode(v[0].as_slice()).unwrap()),
            other => panic!("unknown line {other}"),
        }
    }
    g
}

fn open(g: &Golden) -> (MemoryKv, ProfileStore<MemoryKv>) {
    let kv = MemoryKv::new();
    kv.load(g.dump.clone()).unwrap();
    let store = block(ProfileStore::open(
        kv.clone(),
        g.profile.id,
        g.profile.storage_key(),
        det_rng(1),
    ))
    .unwrap();
    (kv, store)
}

/// `bytes`, a record of namespace `ns`, re-encoded by this crate's types,
/// so encodings compare by content.
fn canonical(ns: i64, bytes: &[u8]) -> Vec<u8> {
    fn re<M: Message + Default>(b: &[u8]) -> Vec<u8> {
        M::decode(b).unwrap().encode_to_vec()
    }
    match Namespace(ns) {
        Profile::NS => re::<<Profile as Singleton>::Record>(bytes),
        ProfileIds::NS => re::<<ProfileIds as Singleton>::Record>(bytes),
        ChatUiConfig::NS => re::<<ChatUiConfig as Singleton>::Record>(bytes),
        Devices::NS => re::<<Devices as Table>::Record>(bytes),
        Networks::NS => re::<<Networks as Table>::Record>(bytes),
        BootstrapClients::NS => re::<<BootstrapClients as Table>::Record>(bytes),
        ChatUiConfigHighlights::NS => re::<<ChatUiConfigHighlights as Table>::Record>(bytes),
        ChatUiConfigTags::NS => re::<<ChatUiConfigTags as Table>::Record>(bytes),
        ChatUiConfigIgnores::NS => re::<<ChatUiConfigIgnores as Table>::Record>(bytes),
        WhisperThreads::NS => re::<<WhisperThreads as Table>::Record>(bytes),
        WhisperRecords::NS => re::<<WhisperRecords as Table>::Record>(bytes),
        EventLogs::NS => re::<<EventLogs as Table>::Record>(bytes),
        Checkpoints::NS => re::<<Checkpoints as Table>::Record>(bytes),
        other => panic!("no record type for {other}"),
    }
}

/// Record `id` of namespace `ns`, as this crate reads it.
fn read(tx: &mut ReadTx<MemoryKv>, ns: i64, id: u64) -> Option<Vec<u8>> {
    fn t<T: Table>(tx: &mut ReadTx<MemoryKv>, id: u64) -> Option<Vec<u8>> {
        block(T::get(tx, id)).unwrap().map(|r| r.encode_to_vec())
    }
    fn s<S: Singleton>(tx: &mut ReadTx<MemoryKv>) -> Option<Vec<u8>> {
        block(S::get(tx)).unwrap().map(|r| r.encode_to_vec())
    }
    match Namespace(ns) {
        Profile::NS => s::<Profile>(tx),
        ProfileIds::NS => s::<ProfileIds>(tx),
        ChatUiConfig::NS => s::<ChatUiConfig>(tx),
        Devices::NS => t::<Devices>(tx, id),
        Networks::NS => t::<Networks>(tx, id),
        BootstrapClients::NS => t::<BootstrapClients>(tx, id),
        ChatUiConfigHighlights::NS => t::<ChatUiConfigHighlights>(tx, id),
        ChatUiConfigTags::NS => t::<ChatUiConfigTags>(tx, id),
        ChatUiConfigIgnores::NS => t::<ChatUiConfigIgnores>(tx, id),
        WhisperThreads::NS => t::<WhisperThreads>(tx, id),
        WhisperRecords::NS => t::<WhisperRecords>(tx, id),
        EventLogs::NS => t::<EventLogs>(tx, id),
        Checkpoints::NS => t::<Checkpoints>(tx, id),
        other => panic!("no record type for {other}"),
    }
}

#[test]
fn sign_in_and_session_records_open() {
    let g = load();
    let kv = MemoryKv::new();
    kv.load(g.dump.clone()).unwrap();
    assert_eq!(
        block(auth::sign_in(&kv, &g.name, &g.password)).unwrap(),
        g.profile
    );
    assert!(block(auth::sign_in(&kv, &g.name, "wrong")).is_err());
    let sk = StorageKey::from_bytes(g.session_key);
    assert_eq!(
        block(auth::load_session(&kv, &sk, &g.session, g.now)).unwrap(),
        g.profile
    );
}

#[test]
fn records_read_back() {
    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    for (ns, id, bytes) in &g.records {
        let got = read(&mut tx, *ns, *id).unwrap_or_else(|| panic!("missing {ns}/{id}"));
        assert_eq!(got, canonical(*ns, bytes), "{ns}/{id}");
    }
    let profile = block(Profile::get(&mut tx)).unwrap().unwrap();
    assert_eq!(profile.name, g.name);
    assert_ne!(profile.device_id, 0);
}

fn keys_of<T: Table>(tx: &mut ReadTx<MemoryKv>, salt: &[u8], out: &mut BTreeSet<String>) {
    for r in block(T::get_all(tx)).unwrap() {
        let id = T::id(&r);
        out.insert(T::NS.record_key(id));
        for ix in T::INDEXES {
            if (ix.condition)(&r) {
                out.insert(index_entry_key(ix.ns, salt, &(ix.key)(&r), id));
            }
        }
    }
}

#[test]
fn every_key_is_the_one_this_crate_derives() {
    let g = load();
    let (kv, store) = open(&g);
    let salt = g.profile.key;
    let mut tx = block(store.read()).unwrap();

    let mut derived = BTreeSet::new();
    keys_of::<Devices>(&mut tx, &salt, &mut derived);
    keys_of::<Networks>(&mut tx, &salt, &mut derived);
    keys_of::<BootstrapClients>(&mut tx, &salt, &mut derived);
    keys_of::<ChatUiConfigHighlights>(&mut tx, &salt, &mut derived);
    keys_of::<ChatUiConfigTags>(&mut tx, &salt, &mut derived);
    keys_of::<ChatUiConfigIgnores>(&mut tx, &salt, &mut derived);
    keys_of::<WhisperThreads>(&mut tx, &salt, &mut derived);
    keys_of::<WhisperRecords>(&mut tx, &salt, &mut derived);
    keys_of::<EventLogs>(&mut tx, &salt, &mut derived);
    keys_of::<Checkpoints>(&mut tx, &salt, &mut derived);
    for ns in [
        StoreVersions::NS,
        Profile::NS,
        ProfileIds::NS,
        ChatUiConfig::NS,
    ] {
        derived.insert(ns.key());
    }
    let lease_key = lease::key(&["replication"]);
    derived.insert(lease_key.clone());

    // Every namespace this crate defines, and the lease namespace.
    let mut known: BTreeSet<i64> = [1001, 1002, 1003, 1004, 2001, 2002, 2003].into();
    known.extend([4001, 4002, 4007, 4014, 6008]);
    known.extend((6012..=6023).chain([6025]));

    let table = &kv.dump()[&strims_dao::key::profile_table(g.profile.id)];
    let stored: BTreeSet<String> = table
        .keys()
        .filter(|k| {
            let ns = k.split(':').next().and_then(parse_base36).unwrap();
            known.contains(&(ns as i64))
        })
        .cloned()
        .collect();
    let missing: Vec<_> = stored.difference(&derived).collect();
    let extra: Vec<_> = derived.difference(&stored).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "missing {missing:?}, extra {extra:?}"
    );

    // Index entries name their key and record.
    let sk = g.profile.storage_key();
    for (k, v) in table {
        let ns = k.split(':').next().and_then(parse_base36).unwrap() as i64;
        if [4002, 4014, 6013, 6014, 6016, 6017, 6019, 6021, 6023, 6025].contains(&ns) {
            let entry = SecondaryIndexKey::decode(sk.open(v).unwrap().as_slice()).unwrap();
            assert_eq!(
                index_entry_key(Namespace(ns), &salt, &entry.key, entry.id),
                *k
            );
        }
    }
    let lease = Mutex::decode(sk.open(&table[&lease_key]).unwrap().as_slice()).unwrap();
    assert_eq!(lease.token.len(), 16);
    assert!(lease.eol > g.now.unix_nanos());
}

fn normalize(
    logs: &[EventLog],
) -> BTreeMap<u64, (Vec<u8>, Vec<(i64, u64, bool, Vec<u8>, Vec<u8>)>)> {
    logs.iter()
        .map(|l| {
            let mut events: Vec<_> = l
                .events
                .iter()
                .map(|e: &Event| {
                    let record = if e.delete {
                        vec![]
                    } else {
                        canonical(e.namespace, &e.record)
                    };
                    (
                        e.namespace,
                        e.id,
                        e.delete,
                        e.version.clone().unwrap_or_default().encode_to_vec(),
                        record,
                    )
                })
                .collect();
            events.sort();
            (
                l.id,
                (
                    l.checkpoint.clone().unwrap_or_default().encode_to_vec(),
                    events,
                ),
            )
        })
        .collect()
}

#[test]
fn compressed_delta_matches() {
    let g = load();
    let (_, store) = open(&g);
    let delta = block(compressed_delta(&store, &g.delta_remote)).unwrap();
    assert!(!delta.is_empty());
    assert_eq!(normalize(&delta), normalize(&g.delta));
}
