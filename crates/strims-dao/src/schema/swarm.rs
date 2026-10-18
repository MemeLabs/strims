// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Swarm caches: a swarm's data and signatures under (swarm id, salt), so
//! a reopened swarm can start from them.
//!
//! A cache is two rows under one id: its meta ([`SwarmCacheMetas`]),
//! found by [`cache_key`] and holding the cache's [`cache_checksum`], and
//! the cache itself ([`SwarmCaches`]). A write that changes a cache moves
//! both rows to a new id, and a device keeps at most [`MAX_SWARM_CACHES`]
//! caches: storing one more evicts the caches with the lowest ids
//! ([`set_cache`]). Ids grow within each of the profile's id ranges, so
//! the lowest id is the least recently written cache, unless the device
//! has since moved on to a lower range.
//!
//! Keys concatenate the swarm id and salt without a separator, so pairs
//! that concatenate to the same bytes share one place: storing either
//! replaces the other, and each reads back only its own cache.
//!
//! Caches stay on the device and aren't observable.
//!
//! Index keys are stored, so [`cache_key`] must keep deriving the same
//! bytes.

use futures_util::future::LocalBoxFuture;
use strims_crypto::crc32_message;
use strims_kv::Kv;
use strims_proto::types::{Cache, CacheMeta};

use crate::error::Error;
use crate::key::Namespace;
use crate::store::{Read, WriteTx};
use crate::table::{Index, Table};

/// The most caches a device keeps.
pub const MAX_SWARM_CACHES: usize = 16;

/// The metas of this device's swarm caches.
pub struct SwarmCacheMetas;

impl SwarmCacheMetas {
    /// Metas by [`cache_key`]; at most one per (swarm id, salt).
    pub const BY_KEY: Index<CacheMeta> =
        Index::unique(Namespace(11002), |m| cache_key(&m.swarm_id, &m.swarm_salt));
}

impl Table for SwarmCacheMetas {
    type Record = CacheMeta;
    const NS: Namespace = Namespace(11001);
    const INDEXES: &'static [Index<CacheMeta>] = &[SwarmCacheMetas::BY_KEY];

    fn id(r: &CacheMeta) -> u64 {
        r.id
    }

    /// Deleting a meta deletes its cache.
    fn on_delete<'a, K: Kv>(
        tx: &'a mut WriteTx<K>,
        p: &'a CacheMeta,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        // The row goes by key: reading it first would decode a whole cache.
        Box::pin(async move { tx.delete_key(&SwarmCaches::NS.record_key(p.id)).await })
    }
}

/// This device's swarm caches, each under its meta's id.
///
/// A row holds a whole swarm's data, so the table is read by id only,
/// never with [`Table::get_all`]. [`set_cache`] writes it, and deleting a
/// meta deletes its row.
pub struct SwarmCaches;

impl Table for SwarmCaches {
    type Record = Cache;
    const NS: Namespace = Namespace(11003);

    fn id(r: &Cache) -> u64 {
        r.id
    }
}

/// The key the cache of a swarm `swarm_id` under `salt` is indexed under:
/// `swarm_id`, then `salt`.
pub fn cache_key(swarm_id: &[u8], salt: &[u8]) -> Vec<u8> {
    let mut k = Vec::with_capacity(swarm_id.len() + salt.len());
    k.extend_from_slice(swarm_id);
    k.extend_from_slice(salt);
    k
}

/// The checksum a meta holds of its cache: the CRC-32 of the cache's
/// stable encoding with id 0, so it doesn't depend on where the cache is
/// stored. A cache with a non-zero id is copied.
pub fn cache_checksum(c: &Cache) -> u32 {
    if c.id == 0 {
        crc32_message(c)
    } else {
        crc32_message(&Cache { id: 0, ..c.clone() })
    }
}

/// The meta under the key of (`swarm_id`, `salt`), which may belong to
/// another pair with the same key.
async fn keyed_meta<R: Read>(
    tx: &mut R,
    swarm_id: &[u8],
    salt: &[u8],
) -> Result<Option<CacheMeta>, Error> {
    let key = cache_key(swarm_id, salt);
    SwarmCacheMetas::index_get(tx, &SwarmCacheMetas::BY_KEY, &key).await
}

/// The meta of the cache of the swarm `swarm_id` under `salt`.
async fn meta<R: Read>(
    tx: &mut R,
    swarm_id: &[u8],
    salt: &[u8],
) -> Result<Option<CacheMeta>, Error> {
    let m = keyed_meta(tx, swarm_id, salt).await?;
    Ok(m.filter(|m| m.swarm_id == swarm_id && m.swarm_salt == salt))
}

/// The cache of the swarm `swarm_id` under `salt`. Its id is its meta's.
pub async fn get_cache<R: Read>(
    tx: &mut R,
    swarm_id: &[u8],
    salt: &[u8],
) -> Result<Option<Cache>, Error> {
    let Some(m) = meta(tx, swarm_id, salt).await? else {
        return Ok(None);
    };
    SwarmCaches::get(tx, m.id).await
}

/// Stores `cache` as the cache of the swarm `swarm_id` under `salt`, with
/// a new id, in place of any cache under the same [`cache_key`], and
/// resolves to true. When the pair's stored cache has the same
/// [`cache_checksum`], writes nothing and resolves to false. With
/// [`MAX_SWARM_CACHES`] other caches stored, the ones with the lowest ids
/// are deleted to make room.
pub async fn set_cache<K: Kv>(
    tx: &mut WriteTx<K>,
    swarm_id: &[u8],
    salt: &[u8],
    mut cache: Cache,
) -> Result<bool, Error> {
    cache.id = 0;
    let checksum = crc32_message(&cache);
    if let Some(m) = keyed_meta(tx, swarm_id, salt).await? {
        if m.swarm_id == swarm_id && m.swarm_salt == salt && m.checksum == checksum {
            return Ok(false);
        }
        SwarmCacheMetas::delete(tx, m.id).await?;
    }
    let mut ids: Vec<u64> = SwarmCacheMetas::get_all(tx)
        .await?
        .iter()
        .map(|m| m.id)
        .collect();
    ids.sort_unstable();
    let excess = (ids.len() + 1).saturating_sub(MAX_SWARM_CACHES);
    for id in ids.into_iter().take(excess) {
        SwarmCacheMetas::delete(tx, id).await?;
    }
    let id = tx.generate_id().await?;
    let m = CacheMeta {
        id,
        swarm_id: swarm_id.to_vec(),
        swarm_salt: salt.to_vec(),
        checksum,
    };
    SwarmCacheMetas::insert(tx, m).await?;
    cache.id = id;
    SwarmCaches::insert(tx, cache).await?;
    Ok(true)
}

/// Deletes the cache of the swarm `swarm_id` under `salt`. Resolves to
/// whether there was one.
pub async fn delete_cache<K: Kv>(
    tx: &mut WriteTx<K>,
    swarm_id: &[u8],
    salt: &[u8],
) -> Result<bool, Error> {
    let Some(m) = meta(tx, swarm_id, salt).await? else {
        return Ok(false);
    };
    SwarmCacheMetas::delete(tx, m.id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_keys_concatenate_id_and_salt() {
        assert_eq!(cache_key(b"id", b"salt"), b"idsalt");
        assert_eq!(cache_key(b"id", b""), b"id");
    }

    #[test]
    fn checksums_ignore_the_cache_id() {
        let c = Cache {
            uri: "magnet:?xt=urn:ppspp:x".into(),
            data: vec![1, 2, 3],
            ..Default::default()
        };
        let stored = Cache { id: 7, ..c.clone() };
        assert_eq!(cache_checksum(&stored), cache_checksum(&c));
        assert_eq!(cache_checksum(&c), crc32_message(&c));
    }

    #[test]
    fn swarm_caches_stay_local() {
        assert!(SwarmCacheMetas::REPLICATION.is_none());
        assert!(SwarmCaches::REPLICATION.is_none());
    }
}
