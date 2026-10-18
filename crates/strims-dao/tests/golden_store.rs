// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A recorded profile store (`golden/store.hex`) that pins the store layout:
//! its sign-in and session records open, its records read back, every key is the one this crate
//! derives, the compressed delta matches, its stream keys name their
//! channels, and its swarm cache reads back by swarm id and salt.

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
use strims_dao::schema::autoseed::{AutoseedConfig, AutoseedRules};
use strims_dao::schema::chat::{
    ChatEmotes, ChatModifiers, ChatProfiles, ChatServerIcons, ChatServers, ChatTags, ChatUiConfig,
    ChatUiConfigHighlights, ChatUiConfigIgnores, ChatUiConfigTags, WhisperRecords, WhisperThreads,
    profile_peer_key,
};
use strims_dao::schema::directory::{DirectoryListingRecords, DirectoryUserRecords, listing_key};
use strims_dao::schema::network::{
    AliasReservations, BootstrapClients, BootstrapConfig, CertificateLogs, NetworkPeers,
    NetworkUiConfig, Networks, RESERVED_FOREVER, network_key, network_scoped_key,
};
use strims_dao::schema::notification::Notifications;
use strims_dao::schema::profile::{Devices, Profile, ProfileIds};
use strims_dao::schema::swarm::{
    SwarmCacheMetas, SwarmCaches, cache_checksum, cache_key, get_cache, set_cache,
};
use strims_dao::schema::system::{Checkpoints, EventLogs, StoreVersions};
use strims_dao::schema::video::{
    HlsConfig, IngressConfig, VideoChannels, channel_by_stream_key, channel_network_key, owner_key,
};
use strims_dao::schema::vnic::VnicConfig;
use strims_dao::seal::StorageKey;
use strims_dao::{ProfileStore, ReadTx, Singleton, Table, lease, ref_key};
use strims_kv::{Dump, MemoryKv};
use strims_proto::dao::v1::{Mutex, SecondaryIndexKey, VersionVector};
use strims_proto::replication::v1::{Event, EventLog};
use strims_proto::types::Cache;

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
    /// Recorded stream keys, with the channel each names.
    stream_keys: Vec<(u64, String)>,
    /// Recorded swarm caches, as (swarm id, salt, cache).
    swarm_caches: Vec<(Vec<u8>, Vec<u8>, Cache)>,
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
        stream_keys: vec![],
        swarm_caches: vec![],
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
            "video_stream_key" => g
                .stream_keys
                .push((u64_of(&v[0]), String::from_utf8(v[1].clone()).unwrap())),
            "swarm_cache" => g.swarm_caches.push((
                v[0].clone(),
                v[1].clone(),
                Cache::decode(v[2].as_slice()).unwrap(),
            )),
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
        NetworkUiConfig::NS => re::<<NetworkUiConfig as Singleton>::Record>(bytes),
        BootstrapConfig::NS => re::<<BootstrapConfig as Singleton>::Record>(bytes),
        CertificateLogs::NS => re::<<CertificateLogs as Table>::Record>(bytes),
        NetworkPeers::NS => re::<<NetworkPeers as Table>::Record>(bytes),
        AliasReservations::NS => re::<<AliasReservations as Table>::Record>(bytes),
        ChatUiConfigHighlights::NS => re::<<ChatUiConfigHighlights as Table>::Record>(bytes),
        ChatUiConfigTags::NS => re::<<ChatUiConfigTags as Table>::Record>(bytes),
        ChatUiConfigIgnores::NS => re::<<ChatUiConfigIgnores as Table>::Record>(bytes),
        WhisperThreads::NS => re::<<WhisperThreads as Table>::Record>(bytes),
        WhisperRecords::NS => re::<<WhisperRecords as Table>::Record>(bytes),
        EventLogs::NS => re::<<EventLogs as Table>::Record>(bytes),
        Checkpoints::NS => re::<<Checkpoints as Table>::Record>(bytes),
        DirectoryListingRecords::NS => re::<<DirectoryListingRecords as Table>::Record>(bytes),
        DirectoryUserRecords::NS => re::<<DirectoryUserRecords as Table>::Record>(bytes),
        ChatServers::NS => re::<<ChatServers as Table>::Record>(bytes),
        ChatServerIcons::NS => re::<<ChatServerIcons as Table>::Record>(bytes),
        ChatEmotes::NS => re::<<ChatEmotes as Table>::Record>(bytes),
        ChatModifiers::NS => re::<<ChatModifiers as Table>::Record>(bytes),
        ChatTags::NS => re::<<ChatTags as Table>::Record>(bytes),
        ChatProfiles::NS => re::<<ChatProfiles as Table>::Record>(bytes),
        Notifications::NS => re::<<Notifications as Table>::Record>(bytes),
        VideoChannels::NS => re::<<VideoChannels as Table>::Record>(bytes),
        IngressConfig::NS => re::<<IngressConfig as Singleton>::Record>(bytes),
        HlsConfig::NS => re::<<HlsConfig as Singleton>::Record>(bytes),
        VnicConfig::NS => re::<<VnicConfig as Singleton>::Record>(bytes),
        AutoseedConfig::NS => re::<<AutoseedConfig as Singleton>::Record>(bytes),
        AutoseedRules::NS => re::<<AutoseedRules as Table>::Record>(bytes),
        SwarmCacheMetas::NS => re::<<SwarmCacheMetas as Table>::Record>(bytes),
        SwarmCaches::NS => re::<<SwarmCaches as Table>::Record>(bytes),
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
        NetworkUiConfig::NS => s::<NetworkUiConfig>(tx),
        BootstrapConfig::NS => s::<BootstrapConfig>(tx),
        CertificateLogs::NS => t::<CertificateLogs>(tx, id),
        NetworkPeers::NS => t::<NetworkPeers>(tx, id),
        AliasReservations::NS => t::<AliasReservations>(tx, id),
        ChatUiConfigHighlights::NS => t::<ChatUiConfigHighlights>(tx, id),
        ChatUiConfigTags::NS => t::<ChatUiConfigTags>(tx, id),
        ChatUiConfigIgnores::NS => t::<ChatUiConfigIgnores>(tx, id),
        WhisperThreads::NS => t::<WhisperThreads>(tx, id),
        WhisperRecords::NS => t::<WhisperRecords>(tx, id),
        EventLogs::NS => t::<EventLogs>(tx, id),
        Checkpoints::NS => t::<Checkpoints>(tx, id),
        DirectoryListingRecords::NS => t::<DirectoryListingRecords>(tx, id),
        DirectoryUserRecords::NS => t::<DirectoryUserRecords>(tx, id),
        ChatServers::NS => t::<ChatServers>(tx, id),
        ChatServerIcons::NS => t::<ChatServerIcons>(tx, id),
        ChatEmotes::NS => t::<ChatEmotes>(tx, id),
        ChatModifiers::NS => t::<ChatModifiers>(tx, id),
        ChatTags::NS => t::<ChatTags>(tx, id),
        ChatProfiles::NS => t::<ChatProfiles>(tx, id),
        Notifications::NS => t::<Notifications>(tx, id),
        VideoChannels::NS => t::<VideoChannels>(tx, id),
        IngressConfig::NS => s::<IngressConfig>(tx),
        HlsConfig::NS => s::<HlsConfig>(tx),
        VnicConfig::NS => s::<VnicConfig>(tx),
        AutoseedConfig::NS => s::<AutoseedConfig>(tx),
        AutoseedRules::NS => t::<AutoseedRules>(tx, id),
        SwarmCacheMetas::NS => t::<SwarmCacheMetas>(tx, id),
        SwarmCaches::NS => t::<SwarmCaches>(tx, id),
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
    let notifications = block(Notifications::get_all(&mut tx)).unwrap();
    assert_eq!(notifications.len(), 1);
    assert!(notifications[0].subject.is_some());
}

/// The keys of `T`'s rows and index entries. A record whose unique key
/// another record took over has no entry under it, so a unique entry is
/// expected only for the record the index finds.
fn keys_of<T: Table>(tx: &mut ReadTx<MemoryKv>, salt: &[u8], out: &mut BTreeSet<String>) {
    for r in block(T::get_all(tx)).unwrap() {
        let id = T::id(&r);
        out.insert(T::NS.record_key(id));
        for ix in T::INDEXES {
            let key = (ix.key)(&r);
            if !(ix.condition)(&r)
                || ix.unique && !block(T::index_ids(tx, ix, &key)).unwrap().contains(&id)
            {
                continue;
            }
            out.insert(index_entry_key(ix.ns, salt, &key, id));
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
    keys_of::<CertificateLogs>(&mut tx, &salt, &mut derived);
    keys_of::<NetworkPeers>(&mut tx, &salt, &mut derived);
    keys_of::<AliasReservations>(&mut tx, &salt, &mut derived);
    keys_of::<ChatUiConfigHighlights>(&mut tx, &salt, &mut derived);
    keys_of::<ChatUiConfigTags>(&mut tx, &salt, &mut derived);
    keys_of::<ChatUiConfigIgnores>(&mut tx, &salt, &mut derived);
    keys_of::<WhisperThreads>(&mut tx, &salt, &mut derived);
    keys_of::<WhisperRecords>(&mut tx, &salt, &mut derived);
    keys_of::<EventLogs>(&mut tx, &salt, &mut derived);
    keys_of::<Checkpoints>(&mut tx, &salt, &mut derived);
    keys_of::<DirectoryListingRecords>(&mut tx, &salt, &mut derived);
    keys_of::<DirectoryUserRecords>(&mut tx, &salt, &mut derived);
    keys_of::<ChatServers>(&mut tx, &salt, &mut derived);
    keys_of::<ChatServerIcons>(&mut tx, &salt, &mut derived);
    keys_of::<ChatEmotes>(&mut tx, &salt, &mut derived);
    keys_of::<ChatModifiers>(&mut tx, &salt, &mut derived);
    keys_of::<ChatTags>(&mut tx, &salt, &mut derived);
    keys_of::<ChatProfiles>(&mut tx, &salt, &mut derived);
    keys_of::<Notifications>(&mut tx, &salt, &mut derived);
    keys_of::<VideoChannels>(&mut tx, &salt, &mut derived);
    keys_of::<AutoseedRules>(&mut tx, &salt, &mut derived);
    keys_of::<SwarmCacheMetas>(&mut tx, &salt, &mut derived);
    // A cache row is under its meta's id.
    for m in block(SwarmCacheMetas::get_all(&mut tx)).unwrap() {
        derived.insert(SwarmCaches::NS.record_key(m.id));
    }
    for ns in [
        StoreVersions::NS,
        Profile::NS,
        ProfileIds::NS,
        ChatUiConfig::NS,
        NetworkUiConfig::NS,
        BootstrapConfig::NS,
        IngressConfig::NS,
        HlsConfig::NS,
        VnicConfig::NS,
        AutoseedConfig::NS,
    ] {
        derived.insert(ns.key());
    }
    let lease_key = lease::key(&["replication"]);
    derived.insert(lease_key.clone());

    // Every namespace this crate defines, and the lease namespace.
    let mut known: BTreeSet<i64> = [1001, 1002, 1003, 1004, 2001, 2002, 2003].into();
    known.extend(4001..=4018);
    known.insert(5001);
    known.extend(6001..=6025);
    known.extend(7001..=7004);
    known.insert(8001);
    known.extend(9001..=9006);
    known.extend(10001..=10002);
    known.extend(11001..=11003);

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
        let index_ns = [
            4002, 4004, 4005, 4006, 4010, 4011, 4012, 4013, 4014, 4017, 4018,
        ];
        if index_ns.contains(&ns)
            || [6003, 6005, 6007, 6010, 6011].contains(&ns)
            || [6013, 6014, 6016, 6017, 6019, 6021, 6023, 6025].contains(&ns)
            || [7002, 9002, 9003, 9005, 9006, 11002].contains(&ns)
        {
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

/// An event as (namespace, id, delete, encoded version, canonical record).
type EventKey = (i64, u64, bool, Vec<u8>, Vec<u8>);

/// Logs by id: the encoded checkpoint and the sorted events.
fn normalize(logs: &[EventLog]) -> BTreeMap<u64, (Vec<u8>, Vec<EventKey>)> {
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

/// The owned network's CA records: the owner's chain and its renewal, an
/// invitee under the owner, and an alias given up.
#[test]
fn ca_records_read_back_by_index() {
    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    let owned = block(Networks::get_all(&mut tx))
        .unwrap()
        .into_iter()
        .find(|n| n.server_config.is_some())
        .unwrap();
    let owner_cert = owned.certificate.clone().unwrap();
    let scoped = |b: &[u8]| network_scoped_key(owned.id, b);

    let logs = block(CertificateLogs::index_get_all(
        &mut tx,
        &CertificateLogs::BY_NETWORK,
        &ref_key(owned.id),
    ))
    .unwrap();
    // the owner's member and root certificates, the invitee's, the renewal
    assert_eq!(logs.len(), 4);
    let by_subject = |tx: &mut ReadTx<MemoryKv>, subject: &str| {
        block(CertificateLogs::index_get(
            tx,
            &CertificateLogs::BY_SUBJECT,
            &scoped(subject.as_bytes()),
        ))
        .unwrap()
        .unwrap()
    };
    let latest = by_subject(&mut tx, &g.name);
    assert_ne!(latest.certificate.as_ref().unwrap(), &owner_cert);
    assert_eq!(latest.certificate.as_ref().unwrap().key, owner_cert.key);
    assert_eq!(
        block(CertificateLogs::index_get(
            &mut tx,
            &CertificateLogs::BY_KEY,
            &scoped(&owner_cert.key)
        ))
        .unwrap()
        .unwrap()
        .id,
        latest.id
    );
    let invitee_log = by_subject(&mut tx, "invitee");

    let owner = block(NetworkPeers::index_get(
        &mut tx,
        &NetworkPeers::BY_PUBLIC_KEY,
        &scoped(&owner_cert.key),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(owner.inviter_peer_id, 0);
    let invited = block(NetworkPeers::index_get_all(
        &mut tx,
        &NetworkPeers::BY_INVITER,
        &ref_key(owner.id),
    ))
    .unwrap();
    assert_eq!(invited.len(), 1);
    assert_eq!(invited[0].public_key, invitee_log.certificate.unwrap().key);

    let held = block(AliasReservations::index_get(
        &mut tx,
        &AliasReservations::BY_ALIAS,
        &scoped(b"invitee"),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(held.reserved_until, RESERVED_FOREVER);
    let spare = block(AliasReservations::index_get(
        &mut tx,
        &AliasReservations::BY_ALIAS,
        &scoped(b"spare"),
    ))
    .unwrap()
    .unwrap();
    assert!(spare.peer_key.is_empty());
    assert!(spare.reserved_until > g.now.unix() && spare.reserved_until < RESERVED_FOREVER);

    let ui = block(NetworkUiConfig::get(&mut tx)).unwrap().unwrap();
    assert_eq!(ui.network_display_order.len(), 2);
    assert_eq!(ui.network_display_order[1], owned.id);
    let bootstrap = block(BootstrapConfig::get(&mut tx)).unwrap().unwrap();
    assert!(bootstrap.enable_publishing);
}

/// The owned network's directory: the owner as its admin, an invitee who
/// may not join, and a listing of each kind, each found by its key.
#[test]
fn directory_records_read_back_by_index() {
    use strims_proto::network::v1::directory::listing::Content;

    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    let owned = block(Networks::get_all(&mut tx))
        .unwrap()
        .into_iter()
        .find(|n| n.server_config.is_some())
        .unwrap();
    let owner_key = owned.certificate.clone().unwrap().key;

    let users = block(DirectoryUserRecords::index_get_all(
        &mut tx,
        &DirectoryUserRecords::BY_NETWORK,
        &ref_key(owned.id),
    ))
    .unwrap();
    assert_eq!(users.len(), 2);
    let owner = block(DirectoryUserRecords::index_get(
        &mut tx,
        &DirectoryUserRecords::BY_PEER_KEY,
        &network_scoped_key(owned.id, &owner_key),
    ))
    .unwrap()
    .unwrap();
    let m = owner.moderation.unwrap();
    assert_eq!((m.is_admin, m.is_moderator), (Some(true), Some(true)));
    assert_eq!((m.disable_join, m.disable_publish), (None, None));
    let invitee = users.into_iter().find(|u| u.id != owner.id).unwrap();
    assert_eq!(invitee.moderation.unwrap().disable_join, Some(true));

    let listings = block(DirectoryListingRecords::index_get_all(
        &mut tx,
        &DirectoryListingRecords::BY_NETWORK,
        &ref_key(owned.id),
    ))
    .unwrap();
    assert_eq!(listings.len(), 4);
    for r in &listings {
        let key = listing_key(owned.id, r.listing.as_ref());
        let found = block(DirectoryListingRecords::index_get(
            &mut tx,
            &DirectoryListingRecords::BY_LISTING,
            &key,
        ))
        .unwrap()
        .unwrap();
        assert_eq!(found.id, r.id);
        if let Some(Content::Media(media)) = r.listing.as_ref().and_then(|l| l.content.as_ref()) {
            let id = strims_ppspp::Uri::parse(&media.swarm_uri).unwrap().id;
            assert_eq!(key.get(9..).unwrap(), id.as_bytes());
            let m = found.moderation.unwrap();
            assert_eq!(
                (m.is_mature, m.category.as_deref()),
                (Some(true), Some("games"))
            );
            assert_eq!(found.notes, "checked");
        }
    }
}

/// The owned network's chat server, with its icon, emotes, modifier, tag
/// and profiles, each found by index; the deleted server left nothing.
#[test]
fn chat_records_read_back_by_index() {
    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    let owned = block(Networks::get_all(&mut tx))
        .unwrap()
        .into_iter()
        .find(|n| n.server_config.is_some())
        .unwrap();
    let owner_key = owned.certificate.clone().unwrap().key;

    let servers = block(ChatServers::get_all(&mut tx)).unwrap();
    assert_eq!(servers.len(), 1);
    let server = &servers[0];
    assert_eq!(server.network_key, network_key(&owned));
    assert_eq!(server.room.as_ref().unwrap().name, "kept");
    let icon = block(ChatServerIcons::get(&mut tx, server.id))
        .unwrap()
        .unwrap();
    assert_eq!(icon.image.unwrap().data, b"kept");
    assert_eq!(block(ChatServerIcons::get_all(&mut tx)).unwrap().len(), 1);

    let k = ref_key(server.id);
    let emotes = block(ChatEmotes::index_get_all(
        &mut tx,
        &ChatEmotes::BY_SERVER,
        &k,
    ))
    .unwrap();
    let mut names: Vec<_> = emotes.iter().map(|e| e.name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["FeelsBadMan", "PeepoHappy"]);
    assert_eq!(block(ChatEmotes::get_all(&mut tx)).unwrap().len(), 2);
    let modifiers = block(ChatModifiers::index_get_all(
        &mut tx,
        &ChatModifiers::BY_SERVER,
        &k,
    ))
    .unwrap();
    assert_eq!(modifiers.len(), 1);
    assert_eq!(modifiers[0].proc_chance, 0.25);
    assert_eq!(block(ChatModifiers::get_all(&mut tx)).unwrap().len(), 1);
    let tags = block(ChatTags::index_get_all(&mut tx, &ChatTags::BY_SERVER, &k)).unwrap();
    assert_eq!(tags.len(), 1);
    assert!(tags[0].sensitive);
    assert_eq!(block(ChatTags::get_all(&mut tx)).unwrap().len(), 1);

    let profiles = block(ChatProfiles::index_get_all(
        &mut tx,
        &ChatProfiles::BY_SERVER,
        &k,
    ))
    .unwrap();
    assert_eq!(profiles.len(), 2);
    assert_eq!(block(ChatProfiles::get_all(&mut tx)).unwrap().len(), 2);
    for p in &profiles {
        let found = block(ChatProfiles::index_get(
            &mut tx,
            &ChatProfiles::BY_PEER_KEY,
            &profile_peer_key(server.id, &p.peer_key),
        ))
        .unwrap()
        .unwrap();
        assert_eq!(found.id, p.id);
    }
    let owner = block(ChatProfiles::index_get(
        &mut tx,
        &ChatProfiles::BY_PEER_KEY,
        &profile_peer_key(server.id, &owner_key),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(owner.alias, g.name);
    let muted = profiles.iter().find(|p| p.id != owner.id).unwrap();
    assert_eq!(muted.mute_deadline, 160);
    assert_eq!(muted.mutes[0].moderator_peer_key, owner_key);
}

/// The video channels, one of each owner kind, each found by its owner
/// key; the deleted one left nothing; both configs; and the recorded stream
/// keys name their channels.
#[test]
fn video_records_read_back() {
    use strims_proto::video::v1::video_channel::Owner;

    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    let networks = block(Networks::get_all(&mut tx)).unwrap();
    let owned = networks.iter().find(|n| n.server_config.is_some()).unwrap();
    let joined = networks.iter().find(|n| n.server_config.is_none()).unwrap();
    let (owned_key, joined_key) = (network_key(owned), network_key(joined));

    let channels = block(VideoChannels::get_all(&mut tx)).unwrap();
    assert_eq!(channels.len(), 3);
    for c in &channels {
        let found = block(VideoChannels::index_get(
            &mut tx,
            &VideoChannels::BY_OWNER,
            &owner_key(c),
        ))
        .unwrap()
        .unwrap();
        assert_eq!(found.id, c.id);
        assert_eq!(c.token.len(), 32);
        assert_eq!(c.key.as_ref().unwrap().public.len(), 32);
        let title = &c.directory_listing_snippet.as_ref().unwrap().title;
        let expected = match c.owner.as_ref().unwrap() {
            Owner::Local(_) => ("local", &owned_key),
            Owner::LocalShare(_) => ("share", &owned_key),
            Owner::RemoteShare(r) => {
                assert_eq!(r.server_addr, "rtmp.example:1935");
                ("remote", &joined_key)
            }
        };
        assert_eq!(
            (title.as_str(), channel_network_key(c)),
            (expected.0, Some(expected.1.as_slice()))
        );
    }

    assert_eq!(g.stream_keys.len(), 2);
    for (id, key) in &g.stream_keys {
        let found = block(channel_by_stream_key(&mut tx, key)).unwrap().unwrap();
        assert_eq!(found.id, *id);
    }

    let ingress = block(IngressConfig::get(&mut tx)).unwrap().unwrap();
    assert!(ingress.enabled);
    assert_eq!(ingress.server_addr, "0.0.0.0:1935");
    assert_eq!(ingress.service_network_keys, [owned_key, joined_key]);
    let hls = block(HlsConfig::get(&mut tx)).unwrap().unwrap();
    assert!(hls.enabled);
    assert_eq!(hls.public_server_addr, "https://hls.example");
}

/// The vnic config, the enabled autoseed config with a rule on each
/// network, and a swarm cache found by its swarm id and salt, whose
/// checksum is the one this crate derives.
#[test]
fn vnic_autoseed_and_swarm_cache_records_read_back() {
    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    let vnic = block(VnicConfig::get(&mut tx)).unwrap().unwrap();
    assert_eq!(
        (vnic.max_upload_bytes_per_second, vnic.max_peers),
        (1 << 20, 12)
    );
    assert!(block(AutoseedConfig::get(&mut tx)).unwrap().unwrap().enable);
    let networks = block(Networks::get_all(&mut tx)).unwrap();
    let owned = networks.iter().find(|n| n.server_config.is_some()).unwrap();
    let joined = networks.iter().find(|n| n.server_config.is_none()).unwrap();
    let mut rules = block(AutoseedRules::get_all(&mut tx)).unwrap();
    rules.sort_by(|a, b| a.label.cmp(&b.label));
    assert_eq!(rules.len(), 2);
    for (r, (label, network)) in rules.iter().zip([("rule 0", owned), ("rule 1", joined)]) {
        assert_eq!(r.label, label);
        assert_eq!(r.salt, label.as_bytes());
        assert_eq!(r.network_key, network_key(network));
        assert_eq!(r.swarm_id.len(), 32);
    }

    assert_eq!(g.swarm_caches.len(), 1);
    let (swarm_id, salt, cache) = &g.swarm_caches[0];
    assert_eq!(salt, b"chat:assets");
    let got = block(get_cache(&mut tx, swarm_id, salt)).unwrap().unwrap();
    assert_eq!(
        got,
        Cache {
            id: got.id,
            ..cache.clone()
        }
    );
    let meta = block(SwarmCacheMetas::index_get(
        &mut tx,
        &SwarmCacheMetas::BY_KEY,
        &cache_key(swarm_id, salt),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(meta.id, got.id);
    assert_eq!(meta.checksum, cache_checksum(cache));
    drop(tx);

    let mut tx = block(store.write(g.now)).unwrap();
    assert!(!block(set_cache(&mut tx, swarm_id, salt, cache.clone())).unwrap());
    // deleting a network deletes its rule
    block(Networks::delete(&mut tx, owned.id)).unwrap();
    let left = block(AutoseedRules::get_all(&mut tx)).unwrap();
    assert_eq!(left, [rules[1].clone()]);
}
