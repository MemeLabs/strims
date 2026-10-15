// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Who holds what, on port 11.
//!
//! - **Publish** ([`Host::peer_index_publish`]): a record `{hash, host_id,
//!   port, timestamp}` signed by the host key is broadcast one hop
//!   ([`M_NORELAY`]) every 10 minutes, and pushed to links that come up in
//!   between. Neighbours store it.
//! - **Search** ([`Host::peer_index_search`]): a search request is broadcast
//!   towards the hash; every host it reaches answers the requester with up
//!   to five stored records for the hash whose holders are its own routed
//!   links, nearest to the requester first. Answers are checked against the
//!   requested hash, and each host comes out once as
//!   [`Event::PeerIndexHost`] until the search times out.
//! - **Store** (`PeerIndexStore`): per network, records live 30 minutes
//!   after they were last received. A record must be signed by the host it
//!   names; a newer one replaces it, and an older unpublish is ignored.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};
use slotmap::SlotMap;
use strims_base::time::{Duration, Time};
use strims_crypto::SignableMessage;
use strims_kademlia::{Filter, ID, Interface};
use strims_proto::vpn::v1::PeerIndexMessage;
use strims_proto::vpn::v1::peer_index_message::{
    Body, Publish, Record, SearchRequest, SearchResponse, Unpublish,
};
use strims_vnic::PeerKey;
use strims_vnic::ports::PEER_INDEX_PORT;

use crate::host::{Event, Host, Inbound, random_u64, sign_record};
use crate::message::{M_NORELAY, M_STD_FLAGS};
use crate::{Error, NetworkId, PeerIndexPublisherId, PeerIndexSearchId};

/// Records per search answer.
pub const PEER_INDEX_SEARCH_RESPONSE_SIZE: usize = 5;
/// How often a publisher republishes its record.
pub const PEER_INDEX_PUBLISH_INTERVAL: Duration = Duration::from_secs(10 * 60);
/// The least time between two expiry passes over a store.
pub const PEER_INDEX_GC_INTERVAL: Duration = Duration::MINUTE;
/// How long a record lives after it was last received.
pub const PEER_INDEX_MAX_RECORD_AGE: Duration = Duration::from_secs(30 * 60);
/// Default bound on stored records per network.
pub const PEER_INDEX_MAX_SIZE: usize = 5120;
/// How long a search collects answers.
pub const PEER_INDEX_SEARCH_TIMEOUT: Duration = Duration::MINUTE;
/// Distinct hosts reported per search.
const SEARCH_SEEN_MAX: usize = 256;

/// `sha256(key || salt)`: what a record advertises.
pub fn peer_index_record_hash(key: &[u8], salt: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(key);
    h.update(salt);
    h.finalize().into()
}

/// A search result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerIndexHost {
    /// The record's timestamp (whole seconds).
    pub timestamp: Time,
    pub host_id: ID,
    pub port: u16,
}

/// `(hash, host id)`.
type ItemKey = ([u8; 32], [u8; 32]);

#[derive(Debug)]
struct Item {
    record: Record,
    host_id: ID,
    /// When the record was last received.
    time: Time,
}

/// Stored records, one store per network.
///
/// Bounded at `max` records; past it the least recently received goes.
#[derive(Debug)]
pub(crate) struct PeerIndexStore {
    items: BTreeMap<ItemKey, Item>,
    /// `(time, key)`, oldest first; the same keys as `items`.
    by_time: BTreeSet<(Time, ItemKey)>,
    max: usize,
    last_gc: Time,
}

/// Records must carry a 32-byte hash and host id and be signed by that
/// host's own key, so that no key can advertise another host.
fn validate(r: &Record) -> Result<ItemKey, Error> {
    r.verify_message()
        .map_err(|_| Error::InvalidRecord("peer index record signature"))?;
    let hash = <[u8; 32]>::try_from(r.hash.as_slice())
        .map_err(|_| Error::InvalidRecord("peer index hash length"))?;
    let host = <[u8; 32]>::try_from(r.host_id.as_slice())
        .map_err(|_| Error::InvalidRecord("peer index host id length"))?;
    if r.key != r.host_id {
        return Err(Error::InvalidRecord(
            "peer index record not signed by its host",
        ));
    }
    Ok((hash, host))
}

/// A record's holder, for nearest-first ordering.
struct Holder<'a> {
    id: ID,
    record: &'a Record,
}

impl Interface for Holder<'_> {
    fn id(&self) -> ID {
        self.id
    }
}

impl PeerIndexStore {
    pub(crate) fn new(max: usize) -> PeerIndexStore {
        PeerIndexStore {
            items: BTreeMap::new(),
            by_time: BTreeSet::new(),
            max: max.max(1),
            last_gc: Time::NIL,
        }
    }

    /// Stores a record and refreshes its time. A record at least as new as
    /// the stored one replaces it.
    fn upsert(&mut self, key: ItemKey, r: Record, host_id: ID, now: Time) {
        if let Some(item) = self.items.get_mut(&key) {
            self.by_time.remove(&(item.time, key));
            item.time = now;
            if r.timestamp >= item.record.timestamp {
                item.record = r;
            }
        } else {
            self.items.insert(
                key,
                Item {
                    record: r,
                    host_id,
                    time: now,
                },
            );
        }
        self.by_time.insert((now, key));
        while self.items.len() > self.max {
            let Some((_, k)) = self.by_time.pop_first() else {
                break;
            };
            self.items.remove(&k);
        }
    }

    /// Removes the record. An unpublish older than the stored record (a
    /// replay) is ignored.
    fn remove(&mut self, key: ItemKey, r: &Record) {
        if let Some(item) = self.items.get(&key)
            && r.timestamp >= item.record.timestamp
        {
            self.by_time.remove(&(item.time, key));
            self.items.remove(&key);
        }
    }

    /// Up to five records for `hash`, nearest to `requester` first,
    /// excluding the requester and holders `has_peer` rejects.
    fn get(&self, hash: &[u8; 32], requester: &ID, has_peer: impl Fn(&ID) -> bool) -> Vec<Record> {
        let mut f = Filter::new(*requester);
        for (_, item) in self.items.range((*hash, [0; 32])..=(*hash, [0xff; 32])) {
            f.push(Holder {
                id: item.host_id,
                record: &item.record,
            });
        }
        f.iter()
            .filter(|h| h.id != *requester && has_peer(&h.id))
            .take(PEER_INDEX_SEARCH_RESPONSE_SIZE)
            .map(|h| h.record.clone())
            .collect()
    }

    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn poll_timeout(&self) -> Option<Time> {
        let (oldest, _) = self.by_time.first()?;
        let due = *oldest + PEER_INDEX_MAX_RECORD_AGE + Duration::NANOSECOND;
        if self.last_gc.is_nil() {
            return Some(due);
        }
        Some(due.max(self.last_gc + PEER_INDEX_GC_INTERVAL))
    }

    /// Drops records not received for `PEER_INDEX_MAX_RECORD_AGE`, at most
    /// once per GC interval.
    pub(crate) fn prune(&mut self, now: Time) {
        if self.poll_timeout().is_none_or(|t| now.before(t)) {
            return;
        }
        self.last_gc = now;
        let eol = now - PEER_INDEX_MAX_RECORD_AGE;
        while let Some(&(t, k)) = self.by_time.first() {
            if !t.before(eol) {
                break;
            }
            self.by_time.pop_first();
            self.items.remove(&k);
        }
    }
}

#[derive(Debug)]
pub(crate) struct Publisher {
    network: NetworkId,
    record: Record,
    next: Time,
}

#[derive(Debug)]
pub(crate) struct Search {
    network: NetworkId,
    rid: u64,
    hash: [u8; 32],
    deadline: Time,
    /// Hosts already reported; bounded by `SEARCH_SEEN_MAX`.
    seen: BTreeSet<(ID, u16)>,
}

/// The peer index's per-host state; the stores are per network.
#[derive(Debug, Default)]
pub(crate) struct PeerIndexService {
    /// Bounded by the owner's calls.
    publishers: SlotMap<PeerIndexPublisherId, Publisher>,
    /// Bounded by the owner's calls.
    searches: SlotMap<PeerIndexSearchId, Search>,
    rids: BTreeMap<(NetworkId, u64), PeerIndexSearchId>,
}

impl PeerIndexService {
    /// Whether this host publishes records in `net`.
    pub(crate) fn publishes_in(&self, net: NetworkId) -> bool {
        self.publishers.values().any(|p| p.network == net)
    }
}

impl<H: Copy> Host<H> {
    /// Advertises that this host serves (`key`, `salt`) on `port`, now and
    /// every 10 minutes until [`Host::peer_index_unpublish`].
    pub fn peer_index_publish(
        &mut self,
        now: Time,
        net: NetworkId,
        key: &[u8],
        salt: &[u8],
        port: u16,
    ) -> Result<PeerIndexPublisherId, Error> {
        if !self.networks.contains_key(net) {
            return Err(Error::NetworkNotFound);
        }
        let p = self.peer_index.publishers.insert(Publisher {
            network: net,
            record: Record {
                hash: peer_index_record_hash(key, salt).to_vec(),
                host_id: self.id.to_bytes().to_vec(),
                port: u32::from(port),
                ..Default::default()
            },
            next: now,
        });
        if let Err(e) = self.peer_index_send(now, p, false) {
            self.peer_index.publishers.remove(p);
            return Err(e);
        }
        Ok(p)
    }

    /// Stops publishing and broadcasts an unpublish.
    pub fn peer_index_unpublish(
        &mut self,
        now: Time,
        publisher: PeerIndexPublisherId,
    ) -> Result<(), Error> {
        let r = self.peer_index_send(now, publisher, true);
        self.peer_index.publishers.remove(publisher);
        r
    }

    /// Signs and broadcasts the publisher's record to the neighbours, or
    /// its unpublish.
    fn peer_index_send(
        &mut self,
        now: Time,
        id: PeerIndexPublisherId,
        unpublish: bool,
    ) -> Result<(), Error> {
        let p = self
            .peer_index
            .publishers
            .get_mut(id)
            .ok_or(Error::UnknownRequest)?;
        p.record.timestamp = now.unix();
        sign_record(&mut p.record, &self.signer);
        p.next = now + PEER_INDEX_PUBLISH_INTERVAL;
        let (net, record) = (p.network, p.record.clone());
        let body = if unpublish {
            Body::Unpublish(Unpublish {
                record: Some(record),
            })
        } else {
            Body::Publish(Publish {
                record: Some(record),
            })
        };
        let msg = PeerIndexMessage { body: Some(body) };
        self.broadcast_proto_with_flags(now, net, PEER_INDEX_PORT, PEER_INDEX_PORT, &msg, M_NORELAY)
    }

    /// Sends the records this host publishes in `net` to the links
    /// `peers`, as last published. See `Host::push_records`.
    pub(crate) fn peer_index_push(&mut self, now: Time, net: NetworkId, peers: &[PeerKey]) {
        if peers.is_empty() {
            return;
        }
        let records: Vec<Record> = self
            .peer_index
            .publishers
            .values()
            .filter(|p| p.network == net && !p.record.signature.is_empty())
            .map(|p| p.record.clone())
            .collect();
        for record in records {
            let msg = PeerIndexMessage {
                body: Some(Body::Publish(Publish {
                    record: Some(record),
                })),
            };
            let mut b = self.pool.take();
            // invariant: encoding into a Vec can't run out of space.
            let _ = prost::Message::encode(&msg, &mut b);
            if self
                .send_to_links(now, net, PEER_INDEX_PORT, &b, peers)
                .is_ok()
            {
                self.metrics.records_pushed += 1;
            }
            self.pool.put(b);
        }
    }

    /// Looks for hosts that published (`key`, `salt`).
    pub fn peer_index_search(
        &mut self,
        now: Time,
        net: NetworkId,
        key: &[u8],
        salt: &[u8],
    ) -> Result<PeerIndexSearchId, Error> {
        if !self.networks.contains_key(net) {
            return Err(Error::NetworkNotFound);
        }
        let hash = peer_index_record_hash(key, salt);
        let rid = random_u64(&mut self.rng);
        // Registered first: this host answers its own request.
        let search = self.peer_index.searches.insert(Search {
            network: net,
            rid,
            hash,
            deadline: now + PEER_INDEX_SEARCH_TIMEOUT,
            seen: BTreeSet::new(),
        });
        self.peer_index.rids.insert((net, rid), search);
        let msg = PeerIndexMessage {
            body: Some(Body::SearchRequest(SearchRequest {
                request_id: rid,
                hash: hash.to_vec(),
            })),
        };
        let target = ID::from_bytes(&hash);
        let res = self.send_proto_with_flags(
            now,
            net,
            target,
            PEER_INDEX_PORT,
            PEER_INDEX_PORT,
            &msg,
            crate::message::M_BROADCAST,
        );
        if let Err(e) = res {
            self.peer_index_cancel_search(search);
            return Err(e);
        }
        Ok(search)
    }

    /// Ends a search without a closed event. Returns whether it was open.
    pub fn peer_index_cancel_search(&mut self, search: PeerIndexSearchId) -> bool {
        let Some(s) = self.peer_index.searches.remove(search) else {
            return false;
        };
        self.peer_index.rids.remove(&(s.network, s.rid));
        true
    }

    /// Records in the network's peer index.
    pub fn peer_index_len(&self, net: NetworkId) -> usize {
        self.networks.get(net).map_or(0, |n| n.peer_index.len())
    }

    /// Port 11.
    pub(crate) fn peer_index_handle(
        &mut self,
        now: Time,
        net: NetworkId,
        msg: &Inbound<'_>,
    ) -> Result<(), Error> {
        let m = <PeerIndexMessage as prost::Message>::decode(msg.body)
            .map_err(|e| Error::Decode(e.to_string()))?;
        let src_bytes = msg.src.to_bytes();
        match m.body {
            Some(Body::Publish(p)) => {
                let r = p.record.ok_or(Error::InvalidRecord("missing record"))?;
                let key = validate(&r)?;
                if r.host_id != src_bytes {
                    return Err(Error::HostIdMismatch);
                }
                let n = self.networks.get_mut(net).ok_or(Error::NetworkNotFound)?;
                n.peer_index.upsert(key, r, msg.src, now);
                Ok(())
            }
            Some(Body::Unpublish(p)) => {
                let r = p.record.ok_or(Error::InvalidRecord("missing record"))?;
                let key = validate(&r)?;
                if r.host_id != src_bytes {
                    return Err(Error::HostIdMismatch);
                }
                let n = self.networks.get_mut(net).ok_or(Error::NetworkNotFound)?;
                n.peer_index.remove(key, &r);
                Ok(())
            }
            Some(Body::SearchRequest(r)) => {
                let Ok(hash) = <[u8; 32]>::try_from(r.hash.as_slice()) else {
                    return Ok(());
                };
                let n = self.networks.get(net).ok_or(Error::NetworkNotFound)?;
                let records = n.peer_index.get(&hash, &msg.src, |id| n.has_peer(id));
                if records.is_empty() {
                    return Ok(());
                }
                let res = PeerIndexMessage {
                    body: Some(Body::SearchResponse(SearchResponse {
                        request_id: r.request_id,
                        records,
                    })),
                };
                self.send_proto_with_flags(
                    now,
                    net,
                    msg.src,
                    PEER_INDEX_PORT,
                    PEER_INDEX_PORT,
                    &res,
                    M_STD_FLAGS,
                )
            }
            Some(Body::SearchResponse(r)) => {
                let Some(&search) = self.peer_index.rids.get(&(net, r.request_id)) else {
                    return Ok(());
                };
                let mut found = Vec::new();
                let mut hinted = Vec::new();
                if let Some(s) = self.peer_index.searches.get_mut(search) {
                    for rec in r.records {
                        let Ok((hash, host)) = validate(&rec) else {
                            continue;
                        };
                        if hash != s.hash {
                            continue;
                        }
                        let h = PeerIndexHost {
                            timestamp: Time::from_unix(rec.timestamp, 0),
                            host_id: ID::from_bytes(&host),
                            // Ports are 16 bits; higher bits are ignored.
                            port: rec.port as u16,
                        };
                        hinted.push(h.host_id);
                        if s.seen.len() < SEARCH_SEEN_MAX && s.seen.insert((h.host_id, h.port)) {
                            found.push(h);
                        }
                    }
                }
                // Responders only list holders that are their own links, so
                // each is a learned link (`Routing::Learned`): the app's
                // next message is usually to that holder.
                if self.config.routing == crate::Routing::Learned
                    && let Some(n) = self.networks.get_mut(net)
                {
                    for holder in hinted {
                        n.topology.learn_edge(now, msg.src, holder);
                    }
                }
                for host in found {
                    self.push_event(Event::PeerIndexHost {
                        network: net,
                        search,
                        host,
                    });
                }
                Ok(())
            }
            None => Err(Error::UnexpectedMessageType),
        }
    }

    pub(crate) fn peer_index_poll_timeout(&self) -> Option<Time> {
        let s = &self.peer_index;
        let publish = s.publishers.values().map(|p| p.next);
        let searches = s.searches.values().map(|x| x.deadline);
        let stores = self
            .networks
            .values()
            .filter_map(|n| n.peer_index.poll_timeout());
        publish.chain(searches).chain(stores).min()
    }

    pub(crate) fn peer_index_timeout(&mut self, now: Time) {
        for (_, n) in self.networks.iter_mut() {
            n.peer_index.prune(now);
        }
        let due: Vec<PeerIndexPublisherId> = self
            .peer_index
            .publishers
            .iter()
            .filter(|(_, p)| !now.before(p.next))
            .map(|(k, _)| k)
            .collect();
        for p in due {
            let _ = self.peer_index_send(now, p, false);
        }
        let expired: Vec<PeerIndexSearchId> = self
            .peer_index
            .searches
            .iter()
            .filter(|(_, s)| !now.before(s.deadline))
            .map(|(k, _)| k)
            .collect();
        for search in expired {
            if let Some(s) = self.peer_index.searches.remove(search) {
                self.peer_index.rids.remove(&(s.network, s.rid));
                self.push_event(Event::PeerIndexSearchClosed {
                    network: s.network,
                    search,
                });
            }
        }
    }

    pub(crate) fn peer_index_network_removed(&mut self, net: NetworkId) {
        self.peer_index.publishers.retain(|_, p| p.network != net);
        let searches: Vec<PeerIndexSearchId> = self
            .peer_index
            .searches
            .iter()
            .filter(|(_, s)| s.network == net)
            .map(|(k, _)| k)
            .collect();
        for search in searches {
            if self.peer_index_cancel_search(search) {
                self.push_event(Event::PeerIndexSearchClosed {
                    network: net,
                    search,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use strims_crypto::key_from_seed;

    fn now() -> Time {
        Time::from_unix(1_700_000_000, 0)
    }

    fn record(seed: u8, hash: &[u8; 32], port: u32, ts: i64) -> (Record, ID) {
        let k = key_from_seed(&[seed; 32]);
        let mut r = Record {
            hash: hash.to_vec(),
            host_id: k.public.clone(),
            port,
            timestamp: ts,
            ..Default::default()
        };
        r.sign_message(&k).unwrap();
        (r, ID::unmarshal(&k.public).unwrap().0)
    }

    fn upsert(s: &mut PeerIndexStore, r: Record, id: ID, t: Time) {
        let key = validate(&r).unwrap();
        s.upsert(key, r, id, t);
    }

    /// 20 holders of one hash: five records, nearest to the requester
    /// first.
    #[test]
    fn peer_index_store() {
        let mut s = PeerIndexStore::new(PEER_INDEX_MAX_SIZE);
        let hash = <[u8; 32]>::from(Sha256::digest(b"test"));
        let mut ids = Vec::new();
        for seed in 1..=20u8 {
            let (r, id) = record(seed, &hash, 0, now().unix());
            ids.push(id);
            upsert(&mut s, r, id, now());
        }
        // Another hash is never returned.
        let (r, id) = record(99, &[7; 32], 0, now().unix());
        upsert(&mut s, r, id, now());

        let requester = ID([0x5555_5555_5555_5555; 4]);
        let got = s.get(&hash, &requester, |_| true);
        assert_eq!(got.len(), PEER_INDEX_SEARCH_RESPONSE_SIZE);
        let mut want = ids.clone();
        want.sort_by_key(|id| *id ^ requester);
        let got_ids: Vec<ID> = got
            .iter()
            .map(|r| ID::unmarshal(&r.host_id).unwrap().0)
            .collect();
        assert_eq!(got_ids, want[..5]);
        // The requester itself and non-peers are skipped.
        let got = s.get(&hash, &ids[0], |id| *id != ids[1]);
        assert!(
            got.iter()
                .all(|r| r.host_id != ids[0].to_bytes() && r.host_id != ids[1].to_bytes())
        );
        assert_eq!(got.len(), 5);
    }

    #[test]
    fn upsert_replaces_the_record() {
        let mut s = PeerIndexStore::new(10);
        let hash = [1u8; 32];
        let (r, id) = record(1, &hash, 1000, now().unix());
        upsert(&mut s, r, id, now());
        let (r, id) = record(1, &hash, 2000, now().unix() + 600);
        upsert(&mut s, r, id, now() + Duration::from_secs(600));
        let got = s.get(&hash, &ID::MIN, |_| true);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].port, 2000);
        // An older record refreshes the time but not the record.
        let (r, id) = record(1, &hash, 3000, now().unix());
        upsert(&mut s, r.clone(), id, now() + Duration::from_secs(700));
        assert_eq!(s.get(&hash, &ID::MIN, |_| true)[0].port, 2000);
        // A replayed old unpublish doesn't remove it; a newer one does.
        let key = validate(&r).unwrap();
        s.remove(key, &r);
        assert_eq!(s.len(), 1);
        let (r, _) = record(1, &hash, 0, now().unix() + 900);
        s.remove(key, &r);
        assert_eq!(s.len(), 0);
    }

    /// Stale records go while fresh ones stay, and the size bound evicts
    /// the least recently received.
    #[test]
    fn prune_walks_from_the_oldest_and_size_is_bounded() {
        let mut s = PeerIndexStore::new(3);
        let t0 = now();
        for seed in 1..=3u8 {
            let (r, id) = record(seed, &[seed; 32], 0, t0.unix());
            upsert(
                &mut s,
                r,
                id,
                t0 + Duration::from_secs(i64::from(seed) * 600),
            );
        }
        // 1 is stale at 1800 + 600 + 1s; 2 and 3 are fresh.
        let t = t0 + Duration::from_secs(600) + PEER_INDEX_MAX_RECORD_AGE + Duration::SECOND;
        assert!(s.poll_timeout().unwrap() <= t);
        s.prune(t);
        assert_eq!(s.len(), 2);
        assert!(s.get(&[1; 32], &ID::MIN, |_| true).is_empty());
        assert_eq!(s.get(&[2; 32], &ID::MIN, |_| true).len(), 1);

        for seed in 4..=6u8 {
            let (r, id) = record(seed, &[seed; 32], 0, t0.unix());
            upsert(&mut s, r, id, t);
        }
        assert_eq!(s.len(), 3);
        assert!(s.get(&[2; 32], &ID::MIN, |_| true).is_empty());
        assert_eq!(s.get(&[6; 32], &ID::MIN, |_| true).len(), 1);
    }

    #[test]
    fn records_must_be_signed_by_their_host() {
        let hash = [3u8; 32];
        let (mut r, _) = record(1, &hash, 0, 1);
        assert!(validate(&r).is_ok());
        // Signed by key 2, claiming host 1.
        r.host_id = key_from_seed(&[1; 32]).public;
        r.sign_message(&key_from_seed(&[2; 32])).unwrap();
        assert!(validate(&r).is_err());
        let (mut r, _) = record(1, &hash, 0, 1);
        r.hash = vec![1; 31];
        r.sign_message(&key_from_seed(&[1; 32])).unwrap();
        assert!(validate(&r).is_err());
    }
}
