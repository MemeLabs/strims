// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The DHT on port 10.
//!
//! - **Publish** ([`Host::hash_table_set`]): a record
//!   `{key, salt, value, timestamp}` signed by an app key is broadcast
//!   towards `sha256(key || salt)` every 10 minutes and stored by every
//!   host it reaches. [`Host::hash_table_update`] takes effect at the next
//!   publish. Closing the publisher broadcasts an unpublish.
//! - **Get** ([`Host::hash_table_get`]): the local copy, if any, comes
//!   first. If it is fresh (younger than the publish interval) the get
//!   closes at once; otherwise a get request is broadcast towards the hash
//!   and hosts holding a newer record answer the requester directly. Every
//!   accepted answer comes out as [`Event::HashTableValue`] until the get
//!   times out ([`Event::HashTableGetClosed`]).
//! - **Store** (`HashTableStore`): host-wide, keyed by
//!   `(host id XOR hash) || network`, so that when it is full the record
//!   farthest from this host goes first. Records expire 30 minutes after
//!   their timestamp.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use slotmap::SlotMap;
use strims_base::time::{Duration, Time};
use strims_crypto::SignableMessage;
use strims_crypto::ed25519_dalek::SigningKey;
use strims_kademlia::{ID, ID_LENGTH};
use strims_proto::types::Key;
use strims_proto::vpn::v1::HashTableMessage;
use strims_proto::vpn::v1::hash_table_message::{
    Body, GetRequest, GetResponse, Publish, Record, Unpublish,
};
use strims_vnic::ports::HASH_TABLE_PORT;

use crate::host::{Event, Host, Inbound, random_u64, sign_record};
use crate::message::M_BROADCAST;
use crate::timeout_queue::TimeoutQueue;
use crate::{Error, HashTableGetId, HashTablePublisherId, NetworkId};

/// How often a publisher republishes its record.
pub const HASH_TABLE_SET_INTERVAL: Duration = Duration::from_secs(10 * 60);
/// The least time between two expiry passes over the store.
pub const HASH_TABLE_DISCARD_INTERVAL: Duration = Duration::MINUTE;
/// How long a record lives after its timestamp.
pub const HASH_TABLE_MAX_RECORD_AGE: Duration = Duration::from_secs(30 * 60);
/// Default bound on stored records.
pub const HASH_TABLE_MAX_SIZE: usize = 5120;
/// Default get timeout.
pub const HASH_TABLE_GET_TIMEOUT: Duration = Duration::MINUTE;

/// Bytes counted per stored record on top of its fields.
const RECORD_OVERHEAD: usize = 64;

/// `sha256(key || salt)`: where a record is stored and looked up.
pub fn hash_table_record_hash(key: &[u8], salt: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(key);
    h.update(salt);
    h.finalize().into()
}

/// Options for [`Host::hash_table_get`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HashTableGetOptions {
    /// Skip the local copy and always ask the network.
    pub disable_cache: bool,
    pub timeout: Duration,
}

impl Default for HashTableGetOptions {
    fn default() -> Self {
        HashTableGetOptions {
            disable_cache: false,
            timeout: HASH_TABLE_GET_TIMEOUT,
        }
    }
}

/// `(host id XOR hash) || store id`.
type ItemKey = [u8; ID_LENGTH + 4];

fn item_key(store_id: u32, hash: &[u8; 32], local: &ID) -> ItemKey {
    let mut k = [0u8; ID_LENGTH + 4];
    let id = local.to_bytes();
    for (i, slot) in k.iter_mut().take(ID_LENGTH).enumerate() {
        *slot = id.get(i).copied().unwrap_or(0) ^ hash.get(i).copied().unwrap_or(0);
    }
    if let Some(tail) = k.get_mut(ID_LENGTH..) {
        tail.copy_from_slice(&store_id.to_be_bytes());
    }
    k
}

fn record_size(r: &Record) -> usize {
    r.key.len() + r.salt.len() + r.value.len() + r.signature.len() + RECORD_OVERHEAD
}

#[derive(Debug)]
struct Stored {
    record: Record,
    deadline: Time,
    size: usize,
}

/// Stored records, shared by every network of the host.
///
/// Bounded at `max_records` records and `max_bytes` bytes; past either,
/// the record farthest from this host is dropped.
#[derive(Debug)]
pub(crate) struct HashTableStore {
    host_id: ID,
    records: BTreeMap<ItemKey, Stored>,
    /// Expiry deadlines; the same keys as `records`.
    discard: TimeoutQueue<ItemKey>,
    bytes: usize,
    max_records: usize,
    max_bytes: usize,
    /// Expiry runs at most once per discard interval.
    last_discard: Time,
}

impl HashTableStore {
    pub(crate) fn new(host_id: ID, max_records: usize, max_bytes: usize) -> HashTableStore {
        HashTableStore {
            host_id,
            records: BTreeMap::new(),
            discard: TimeoutQueue::default(),
            bytes: 0,
            max_records: max_records.max(1),
            max_bytes,
            last_discard: Time::NIL,
        }
    }

    /// Records expire `HASH_TABLE_MAX_RECORD_AGE` after their timestamp.
    /// A timestamp in the future counts as now, so no record outlives the
    /// age limit.
    fn deadline(r: &Record, now: Time) -> Time {
        Time::from_unix(r.timestamp.min(now.unix()), 0) + HASH_TABLE_MAX_RECORD_AGE
    }

    /// Stores a verified record. Returns whether it is new or its value
    /// changed.
    pub(crate) fn upsert(&mut self, store_id: u32, r: Record, now: Time) -> bool {
        let hash = hash_table_record_hash(&r.key, &r.salt);
        let k = item_key(store_id, &hash, &self.host_id);
        let deadline = Self::deadline(&r, now);
        let size = record_size(&r);
        let modified = match self.records.get_mut(&k) {
            Some(prev) => {
                if prev.record.timestamp > r.timestamp {
                    return false;
                }
                let modified = prev.record.value != r.value;
                self.discard.remove(prev.deadline, k);
                self.bytes = self.bytes.saturating_sub(prev.size);
                *prev = Stored {
                    record: r,
                    deadline,
                    size,
                };
                modified
            }
            None => {
                self.records.insert(
                    k,
                    Stored {
                        record: r,
                        deadline,
                        size,
                    },
                );
                true
            }
        };
        self.discard.push(deadline, k);
        self.bytes += size;
        while self.records.len() > self.max_records || self.bytes > self.max_bytes {
            let Some((k, s)) = self.records.pop_last() else {
                break;
            };
            self.discard.remove(s.deadline, k);
            self.bytes = self.bytes.saturating_sub(s.size);
        }
        modified
    }

    /// Removes the record if `r` is newer.
    pub(crate) fn remove(&mut self, store_id: u32, r: &Record) {
        let hash = hash_table_record_hash(&r.key, &r.salt);
        let k = item_key(store_id, &hash, &self.host_id);
        if self
            .records
            .get(&k)
            .is_some_and(|prev| prev.record.timestamp < r.timestamp)
            && let Some(s) = self.records.remove(&k)
        {
            self.discard.remove(s.deadline, k);
            self.bytes = self.bytes.saturating_sub(s.size);
        }
    }

    /// The record stored under `hash`.
    pub(crate) fn get(&self, store_id: u32, hash: &[u8; 32]) -> Option<&Record> {
        self.records
            .get(&item_key(store_id, hash, &self.host_id))
            .map(|s| &s.record)
    }

    /// Drops a removed network's records.
    pub(crate) fn remove_store(&mut self, store_id: u32) {
        let id = store_id.to_be_bytes();
        let dead: Vec<(ItemKey, Time)> = self
            .records
            .iter()
            .filter(|(k, _)| k.get(ID_LENGTH..) == Some(&id[..]))
            .map(|(k, s)| (*k, s.deadline))
            .collect();
        for (k, d) in dead {
            if let Some(s) = self.records.remove(&k) {
                self.bytes = self.bytes.saturating_sub(s.size);
            }
            self.discard.remove(d, k);
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn poll_timeout(&self) -> Option<Time> {
        let next = self.discard.next()? + Duration::NANOSECOND;
        if self.last_discard.is_nil() {
            return Some(next);
        }
        Some(next.max(self.last_discard + HASH_TABLE_DISCARD_INTERVAL))
    }

    /// Drops expired records, at most once per discard interval.
    pub(crate) fn handle_timeout(&mut self, now: Time) {
        if self.poll_timeout().is_none_or(|t| now.before(t)) {
            return;
        }
        self.last_discard = now;
        while let Some(k) = self.discard.pop_expired(now) {
            if let Some(s) = self.records.remove(&k) {
                self.bytes = self.bytes.saturating_sub(s.size);
            }
        }
    }
}

/// A record this host publishes.
#[derive(Debug)]
pub(crate) struct Publisher {
    network: NetworkId,
    signer: SigningKey,
    record: Record,
    target: ID,
    next: Time,
    /// The last publish reached no link: publish again when links come
    /// up (`Host::push_records`).
    unsent: bool,
}

#[derive(Debug)]
pub(crate) struct Get {
    network: NetworkId,
    rid: u64,
    hash: [u8; 32],
    deadline: Time,
    /// Whether a value was delivered; the first response always is.
    delivered: bool,
}

/// The hash table's per-host state.
#[derive(Debug)]
pub(crate) struct HashTableService {
    pub(crate) store: HashTableStore,
    /// Bounded by the owner's calls.
    publishers: SlotMap<HashTablePublisherId, Publisher>,
    /// Bounded by the owner's calls.
    gets: SlotMap<HashTableGetId, Get>,
    rids: BTreeMap<(NetworkId, u64), HashTableGetId>,
}

impl HashTableService {
    /// Whether a record this host publishes in `net` reached no link.
    pub(crate) fn unsent_in(&self, net: NetworkId) -> bool {
        self.publishers
            .values()
            .any(|p| p.network == net && p.unsent)
    }

    pub(crate) fn new(store: HashTableStore) -> HashTableService {
        HashTableService {
            store,
            publishers: SlotMap::with_key(),
            gets: SlotMap::with_key(),
            rids: BTreeMap::new(),
        }
    }
}

// TODO: cache verifying keys for hash table and peer index records, as the
// trailer path does, if profiles show the cost.
fn validate(r: &Record) -> Result<(), Error> {
    r.verify_message()
        .map_err(|_| Error::InvalidRecord("hash table record signature"))
}

impl<H: Copy> Host<H> {
    /// Publishes `value` under (`key.public`, `salt`) now and every 10
    /// minutes until [`Host::hash_table_close`].
    pub fn hash_table_set(
        &mut self,
        now: Time,
        net: NetworkId,
        key: &Key,
        salt: &[u8],
        value: &[u8],
    ) -> Result<HashTablePublisherId, Error> {
        if !self.networks.contains_key(net) {
            return Err(Error::NetworkNotFound);
        }
        let signer = strims_crypto::signing_key(key)?;
        let target = ID::from_bytes(&hash_table_record_hash(&key.public, salt));
        let p = self.hash_table.publishers.insert(Publisher {
            network: net,
            signer,
            record: Record {
                key: key.public.clone(),
                salt: salt.to_vec(),
                value: value.to_vec(),
                ..Default::default()
            },
            target,
            next: now,
            unsent: false,
        });
        if let Err(e) = self.hash_table_publish(now, p, false) {
            self.hash_table.publishers.remove(p);
            return Err(e);
        }
        Ok(p)
    }

    /// Changes the published value from the next publish on.
    pub fn hash_table_update(
        &mut self,
        publisher: HashTablePublisherId,
        value: &[u8],
    ) -> Result<(), Error> {
        let p = self
            .hash_table
            .publishers
            .get_mut(publisher)
            .ok_or(Error::UnknownRequest)?;
        p.record.value = value.to_vec();
        Ok(())
    }

    /// Stops publishing and broadcasts an unpublish.
    pub fn hash_table_close(
        &mut self,
        now: Time,
        publisher: HashTablePublisherId,
    ) -> Result<(), Error> {
        let r = self.hash_table_publish(now, publisher, true);
        self.hash_table.publishers.remove(publisher);
        r
    }

    /// Signs and broadcasts the publisher's record, or its unpublish.
    fn hash_table_publish(
        &mut self,
        now: Time,
        id: HashTablePublisherId,
        unpublish: bool,
    ) -> Result<(), Error> {
        let p = self
            .hash_table
            .publishers
            .get_mut(id)
            .ok_or(Error::UnknownRequest)?;
        p.record.timestamp = now.unix();
        sign_record(&mut p.record, &p.signer);
        p.next = now + HASH_TABLE_SET_INTERVAL;
        let (net, target, record) = (p.network, p.target, p.record.clone());
        let body = if unpublish {
            Body::Unpublish(Unpublish {
                record: Some(record),
            })
        } else {
            if let Some(n) = self.networks.get(net) {
                self.hash_table
                    .store
                    .upsert(n.store_id, record.clone(), now);
            }
            Body::Publish(Publish {
                record: Some(record),
            })
        };
        let msg = HashTableMessage { body: Some(body) };
        let copies = self.send_proto_message(
            now,
            net,
            target,
            HASH_TABLE_PORT,
            HASH_TABLE_PORT,
            &msg,
            M_BROADCAST,
        );
        if !unpublish && let Some(p) = self.hash_table.publishers.get_mut(id) {
            p.unsent = matches!(copies, Ok(0));
        }
        copies.map(|_| ())
    }

    /// Publishes again the records of `net` whose last publish reached no
    /// link. See `Host::push_records`.
    pub(crate) fn hash_table_publish_unsent(&mut self, now: Time, net: NetworkId) {
        let unsent: Vec<HashTablePublisherId> = self
            .hash_table
            .publishers
            .iter()
            .filter(|(_, p)| p.network == net && p.unsent)
            .map(|(k, _)| k)
            .collect();
        for p in unsent {
            if self.hash_table_publish(now, p, false).is_ok() {
                self.metrics.records_pushed += 1;
            }
        }
    }

    /// Looks up the value under (`key`, `salt`).
    ///
    /// Values arrive as [`Event::HashTableValue`]. When the local copy is
    /// fresh no request is sent and the get closes at once.
    pub fn hash_table_get(
        &mut self,
        now: Time,
        net: NetworkId,
        key: &[u8],
        salt: &[u8],
        options: HashTableGetOptions,
    ) -> Result<HashTableGetId, Error> {
        let store_id = self
            .networks
            .get(net)
            .ok_or(Error::NetworkNotFound)?
            .store_id;
        let hash = hash_table_record_hash(key, salt);
        let target = ID::from_bytes(&hash);
        let rid = random_u64(&mut self.rng);

        let cached = if options.disable_cache {
            None
        } else {
            self.hash_table
                .store
                .get(store_id, &hash)
                .map(|r| (r.timestamp, r.value.clone()))
        };
        let timestamp = cached.as_ref().map_or(0, |c| c.0);
        let request = options.disable_cache
            || now.after(Time::from_unix(timestamp, 0) + HASH_TABLE_SET_INTERVAL);
        if request {
            let msg = HashTableMessage {
                body: Some(Body::GetRequest(GetRequest {
                    request_id: rid,
                    hash: hash.to_vec(),
                    if_modified_since: timestamp,
                })),
            };
            self.send_proto_with_flags(
                now,
                net,
                target,
                HASH_TABLE_PORT,
                HASH_TABLE_PORT,
                &msg,
                M_BROADCAST,
            )?;
        }
        let get = self.hash_table.gets.insert(Get {
            network: net,
            rid,
            hash,
            deadline: now + options.timeout,
            delivered: cached.is_some(),
        });
        if let Some((_, value)) = cached {
            self.push_event(Event::HashTableValue {
                network: net,
                get,
                value,
            });
        }
        if request {
            self.hash_table.rids.insert((net, rid), get);
        } else {
            self.hash_table.gets.remove(get);
            self.push_event(Event::HashTableGetClosed { network: net, get });
        }
        Ok(get)
    }

    /// Ends a get without a closed event. Returns whether it was open.
    pub fn hash_table_cancel_get(&mut self, get: HashTableGetId) -> bool {
        let Some(g) = self.hash_table.gets.remove(get) else {
            return false;
        };
        self.hash_table.rids.remove(&(g.network, g.rid));
        true
    }

    /// Stored hash table records, host-wide.
    pub fn hash_table_len(&self) -> usize {
        self.hash_table.store.len()
    }

    /// Port 10.
    pub(crate) fn hash_table_handle(
        &mut self,
        now: Time,
        net: NetworkId,
        msg: &Inbound<'_>,
    ) -> Result<(), Error> {
        if msg.hops == 0 {
            return Ok(());
        }
        let m = <HashTableMessage as prost::Message>::decode(msg.body)
            .map_err(|e| Error::Decode(e.to_string()))?;
        let store_id = self
            .networks
            .get(net)
            .ok_or(Error::NetworkNotFound)?
            .store_id;
        match m.body {
            Some(Body::Publish(p)) => {
                let r = p.record.ok_or(Error::InvalidRecord("missing record"))?;
                validate(&r)?;
                self.hash_table.store.upsert(store_id, r, now);
                Ok(())
            }
            Some(Body::Unpublish(p)) => {
                let r = p.record.ok_or(Error::InvalidRecord("missing record"))?;
                validate(&r)?;
                self.hash_table.store.remove(store_id, &r);
                Ok(())
            }
            Some(Body::GetRequest(r)) => {
                let Ok(hash) = <[u8; 32]>::try_from(r.hash.as_slice()) else {
                    return Ok(());
                };
                let Some(record) = self.hash_table.store.get(store_id, &hash) else {
                    return Ok(());
                };
                if record.timestamp <= r.if_modified_since {
                    return Ok(());
                }
                let res = HashTableMessage {
                    body: Some(Body::GetResponse(GetResponse {
                        request_id: r.request_id,
                        record: Some(record.clone()),
                    })),
                };
                self.send_proto_with_flags(
                    now,
                    net,
                    msg.src,
                    HASH_TABLE_PORT,
                    HASH_TABLE_PORT,
                    &res,
                    0,
                )
            }
            Some(Body::GetResponse(r)) => {
                let Some(record) = r.record else {
                    return Ok(());
                };
                if validate(&record).is_err() {
                    return Ok(());
                }
                let modified = self.hash_table.store.upsert(store_id, record.clone(), now);
                if msg.header.dst_id != self.id {
                    return Ok(());
                }
                let Some(&get) = self.hash_table.rids.get(&(net, r.request_id)) else {
                    return Ok(());
                };
                let Some(g) = self.hash_table.gets.get_mut(get) else {
                    return Ok(());
                };
                // A record for another key isn't an answer.
                if hash_table_record_hash(&record.key, &record.salt) != g.hash {
                    return Ok(());
                }
                if !modified && g.delivered {
                    return Ok(());
                }
                g.delivered = true;
                self.push_event(Event::HashTableValue {
                    network: net,
                    get,
                    value: record.value,
                });
                Ok(())
            }
            None => Err(Error::UnexpectedMessageType),
        }
    }

    pub(crate) fn hash_table_poll_timeout(&self) -> Option<Time> {
        let s = &self.hash_table;
        let publish = s.publishers.values().map(|p| p.next);
        let gets = s.gets.values().map(|g| g.deadline);
        publish.chain(gets).chain(s.store.poll_timeout()).min()
    }

    pub(crate) fn hash_table_timeout(&mut self, now: Time) {
        self.hash_table.store.handle_timeout(now);
        let due: Vec<HashTablePublisherId> = self
            .hash_table
            .publishers
            .iter()
            .filter(|(_, p)| !now.before(p.next))
            .map(|(k, _)| k)
            .collect();
        for p in due {
            // A failed publish (no route, or a removed network) is retried
            // at the next interval.
            let _ = self.hash_table_publish(now, p, false);
        }
        let expired: Vec<HashTableGetId> = self
            .hash_table
            .gets
            .iter()
            .filter(|(_, g)| !now.before(g.deadline))
            .map(|(k, _)| k)
            .collect();
        for get in expired {
            if let Some(g) = self.hash_table.gets.remove(get) {
                self.hash_table.rids.remove(&(g.network, g.rid));
                self.push_event(Event::HashTableGetClosed {
                    network: g.network,
                    get,
                });
            }
        }
    }

    pub(crate) fn hash_table_network_removed(&mut self, net: NetworkId) {
        self.hash_table.publishers.retain(|_, p| p.network != net);
        let gets: Vec<HashTableGetId> = self
            .hash_table
            .gets
            .iter()
            .filter(|(_, g)| g.network == net)
            .map(|(k, _)| k)
            .collect();
        for get in gets {
            if self.hash_table_cancel_get(get) {
                self.push_event(Event::HashTableGetClosed { network: net, get });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use strims_crypto::key_from_seed;

    fn record(seed: u8, salt: &[u8], value: &[u8], ts: i64) -> Record {
        let mut r = Record {
            salt: salt.to_vec(),
            value: value.to_vec(),
            timestamp: ts,
            ..Default::default()
        };
        r.sign_message(&key_from_seed(&[seed; 32])).unwrap();
        r
    }

    fn now() -> Time {
        Time::from_unix(1_700_000_000, 0)
    }

    fn host() -> (crate::VnicHost<NetworkId>, Host<u16>, NetworkId) {
        let mut vnic = crate::VnicHost::new(
            key_from_seed(&[1; 32]),
            strims_vnic::HostConfig::default(),
            strims_base::rng::det_rng(1),
        )
        .unwrap();
        let mut host = Host::new(
            &mut vnic,
            crate::Config::default(),
            strims_base::rng::det_rng(2),
        )
        .unwrap();
        let net = host.add_network(&mut vnic, b"net").unwrap();
        (vnic, host, net)
    }

    fn respond(host: &mut Host<u16>, net: NetworkId, rid: u64, r: &Record) {
        let body = prost::Message::encode_to_vec(&HashTableMessage {
            body: Some(Body::GetResponse(GetResponse {
                request_id: rid,
                record: Some(r.clone()),
            })),
        });
        let header = crate::MessageHeader {
            dst_id: host.id(),
            ..Default::default()
        };
        let msg = Inbound {
            header,
            src: ID([5; 4]),
            hops: 2,
            lead: 1,
            verified: None,
            body: &body,
        };
        host.hash_table_handle(now(), net, &msg).unwrap();
    }

    fn values(host: &mut Host<u16>) -> Vec<Vec<u8>> {
        std::iter::from_fn(|| host.poll_event())
            .filter_map(|e| match e {
                Event::HashTableValue { value, .. } => Some(value),
                _ => None,
            })
            .collect()
    }

    /// Only a record for the requested key answers a get. With
    /// `disable_cache`, the first answer comes out even when it equals the
    /// local copy.
    #[test]
    fn get_responses_must_answer_the_request() {
        let (_vnic, mut host, net) = host();
        let k = key_from_seed(&[9; 32]);
        let t = now().unix();
        // The local copy is already current.
        let mine = record(9, b"s", b"v", t);
        let store_id = host.networks[net].store_id;
        host.hash_table.store.upsert(store_id, mine.clone(), now());

        let opts = HashTableGetOptions {
            disable_cache: true,
            ..Default::default()
        };
        let get = host
            .hash_table_get(now(), net, &k.public, b"s", opts)
            .unwrap();
        let rid = host.hash_table.gets[get].rid;

        // Another key's record under our request id: not an answer.
        respond(&mut host, net, rid, &record(8, b"s", b"x", t));
        // An unknown request id: stored, not delivered.
        respond(&mut host, net, rid ^ 1, &record(7, b"s", b"y", t));
        assert!(values(&mut host).is_empty());
        // The answer, identical to the local copy, comes out once.
        respond(&mut host, net, rid, &mine);
        respond(&mut host, net, rid, &mine);
        assert_eq!(values(&mut host), vec![b"v".to_vec()]);
        // A changed value comes out too.
        respond(&mut host, net, rid, &record(9, b"s", b"w", t + 1));
        assert_eq!(values(&mut host), vec![b"w".to_vec()]);
        // A forged signature is ignored.
        let mut bad = record(9, b"s", b"z", t + 2);
        bad.value = b"zz".to_vec();
        respond(&mut host, net, rid, &bad);
        assert!(values(&mut host).is_empty());
    }

    #[test]
    fn record_hash_is_sha256_of_key_and_salt() {
        let h = hash_table_record_hash(b"ab", b"c");
        assert_eq!(h, <[u8; 32]>::from(Sha256::digest(b"abc")));
    }

    #[test]
    fn upsert_reports_changes() {
        let mut s = HashTableStore::new(ID([9; 4]), 100, 1 << 20);
        let t = now().unix();
        assert!(s.upsert(1, record(1, b"s", b"v1", t), now()));
        // Same value, newer: stored but not a change.
        assert!(!s.upsert(1, record(1, b"s", b"v1", t + 1), now()));
        assert_eq!(
            s.get(
                1,
                &hash_table_record_hash(&key_from_seed(&[1; 32]).public, b"s")
            )
            .unwrap()
            .timestamp,
            t + 1
        );
        // Changed value: a change.
        assert!(s.upsert(1, record(1, b"s", b"v2", t + 2), now()));
        // Older: ignored.
        assert!(!s.upsert(1, record(1, b"s", b"v3", t), now()));
        // Another network's store is separate.
        assert!(s.upsert(2, record(1, b"s", b"v1", t), now()));
        assert_eq!(s.len(), 2);
        s.remove_store(2);
        assert_eq!(s.len(), 1);
        assert_eq!(s.discard.len(), 1);
    }

    /// A record stored again under the key of an evicted or removed one
    /// expires at its own deadline.
    #[test]
    fn stale_deadlines_do_not_delete_new_records() {
        let mut s = HashTableStore::new(ID([9; 4]), 100, 1 << 20);
        let t0 = now();
        s.upsert(1, record(1, b"s", b"old", t0.unix()), t0);
        // Unpublished, then published again ten minutes later.
        s.remove(1, &record(1, b"s", b"old", t0.unix() + 1));
        assert_eq!(s.len(), 0);
        let t1 = t0 + Duration::from_secs(600);
        s.upsert(1, record(1, b"s", b"new", t1.unix()), t1);
        // Past the old record's deadline, the new one stays.
        let t2 = t0 + HASH_TABLE_MAX_RECORD_AGE + Duration::from_secs(60);
        s.handle_timeout(t2);
        assert_eq!(s.len(), 1);
        // And it expires at its own.
        let t3 = t1 + HASH_TABLE_MAX_RECORD_AGE + Duration::from_secs(60);
        s.handle_timeout(t3);
        assert_eq!(s.len(), 0);
        assert_eq!(s.poll_timeout(), None);
    }

    /// A record stamped in the future expires within the age limit of its
    /// arrival.
    #[test]
    fn future_timestamps_expire_from_now() {
        let mut s = HashTableStore::new(ID([9; 4]), 100, 1 << 20);
        let t0 = now();
        s.upsert(1, record(1, b"s", b"v", t0.unix() + 86_400), t0);
        let t = t0 + HASH_TABLE_MAX_RECORD_AGE + Duration::from_secs(60);
        assert!(s.poll_timeout().unwrap() <= t);
        s.handle_timeout(t);
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn full_store_drops_the_farthest_record() {
        let local = ID::MIN;
        let mut s = HashTableStore::new(local, 3, 1 << 20);
        let t = now();
        let mut keys = Vec::new();
        for seed in 1..=4u8 {
            let r = record(seed, b"", b"v", t.unix());
            keys.push(hash_table_record_hash(&r.key, &r.salt));
            s.upsert(1, r, t);
        }
        assert_eq!(s.len(), 3);
        // The kept hashes are the three smallest (closest to ID::MIN).
        let mut sorted = keys.clone();
        sorted.sort();
        for h in &sorted[..3] {
            assert!(s.get(1, h).is_some());
        }
        assert!(s.get(1, &sorted[3]).is_none());
        // A byte budget also bounds it.
        let mut s = HashTableStore::new(local, 100, 2 * (RECORD_OVERHEAD + 32 + 64 + 1000));
        for seed in 1..=4u8 {
            s.upsert(1, record(seed, b"", &[0; 1000], t.unix()), t);
        }
        assert_eq!(s.len(), 2);
        assert!(s.bytes <= s.max_bytes);
    }

    #[test]
    fn expiry_is_batched_per_discard_interval() {
        let mut s = HashTableStore::new(ID::MIN, 100, 1 << 20);
        let t0 = now();
        s.upsert(1, record(1, b"a", b"v", t0.unix()), t0);
        let t1 = t0 + Duration::SECOND;
        s.upsert(1, record(2, b"b", b"v", t1.unix()), t1);
        let first = s.poll_timeout().unwrap();
        assert_eq!(first, t0 + HASH_TABLE_MAX_RECORD_AGE + Duration::NANOSECOND);
        s.handle_timeout(first);
        assert_eq!(s.len(), 1);
        // The next one waits for the discard interval.
        assert_eq!(s.poll_timeout(), Some(first + HASH_TABLE_DISCARD_INTERVAL));
    }
}
