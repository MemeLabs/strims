// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A profile as one file: its sign-in record and its store's table, entry
//! for entry ([`export_profile`], [`import_profile`]), for a backup or a
//! move between stores.
//!
//! The store's values stay sealed under the profile's key and the record
//! under its password, so a file is as safe as the store it came from, no
//! safer. An import is the same device as its original: running both
//! makes one device's replicas diverge. A second device pairs instead
//! ([`import_pairing_token`](crate::auth::import_pairing_token)).

use prost::Message;
use strims_base::rng::DetRng;
use strims_kv::{Kv, ReadTx as _, WriteTx as _};
use strims_proto::auth::v1::ServerUserThing;

use crate::auth::{PROFILES, SESSIONS, get_user, open_user};
use crate::error::Error;
use crate::key::profile_table;
use crate::store::{ProfileStore, STORE_VERSION};

/// [`ProfileExport::format`]: what the file is.
pub const EXPORT_FORMAT: &str = "strims.profile-export.v1";

/// An exported profile.
#[derive(Clone, PartialEq, Message)]
pub struct ProfileExport {
    /// [`EXPORT_FORMAT`].
    #[prost(string, tag = "1")]
    pub format: String,
    /// The [`STORE_VERSION`] the store's entries are at.
    #[prost(uint32, tag = "2")]
    pub store_version: u32,
    /// The profile's id, which names its store's table.
    #[prost(uint64, tag = "3")]
    pub profile_id: u64,
    /// The profile's sign-in record.
    #[prost(message, optional, tag = "4")]
    pub user: Option<ServerUserThing>,
    /// Every entry of the store's table, in key order, as stored.
    #[prost(message, repeated, tag = "5")]
    pub entries: Vec<ExportEntry>,
}

/// One entry of an exported store's table.
#[derive(Clone, PartialEq, Message)]
pub struct ExportEntry {
    /// The entry's key.
    #[prost(string, tag = "1")]
    pub key: String,
    /// The entry's value, as stored.
    #[prost(bytes = "vec", tag = "2")]
    pub value: Vec<u8>,
}

/// The encoded [`ProfileExport`] of the profile `name` signs in to with
/// `password`. Fails with [`Error::Credentials`] as
/// [`sign_in`](crate::auth::sign_in) does, and with
/// [`Error::UnsupportedStoreVersion`] for a store this build doesn't read.
///
/// `rng` is the store's sealing rng ([`ProfileStore::open`]), which an
/// export, reading alone, never draws from.
pub async fn export_profile<K: Kv + Clone>(
    kv: &K,
    name: &str,
    password: &str,
    rng: DetRng,
) -> Result<Vec<u8>, Error> {
    let user = get_user(kv, name).await?.ok_or(Error::Credentials)?;
    let pk = open_user(&user, password)?;
    // the store must open at this build's version
    ProfileStore::open(kv.clone(), pk.id, pk.storage_key(), rng).await?;
    let mut tx = kv.begin_read(&profile_table(pk.id)).await?;
    let entries = tx
        .scan("", None, usize::MAX)
        .await?
        .into_iter()
        .map(|(key, value)| ExportEntry { key, value })
        .collect();
    Ok(ProfileExport {
        format: EXPORT_FORMAT.to_owned(),
        store_version: STORE_VERSION,
        profile_id: pk.id,
        user: Some(user),
        entries,
    }
    .encode_to_vec())
}

/// Adds the profile an encoded [`ProfileExport`] holds, under the user
/// name it was exported with, which it returns. Fails with
/// [`Error::NameTaken`] if the name has a record, [`Error::StoreExists`] if
/// the profile has a store, and [`Error::UnsupportedStoreVersion`] for an
/// export at another store version; a failed import leaves no store.
pub async fn import_profile<K: Kv>(kv: &K, file: &[u8]) -> Result<String, Error> {
    let export = ProfileExport::decode(file)?;
    if export.format != EXPORT_FORMAT {
        return Err(Error::Invalid("profile export"));
    }
    if export.store_version != STORE_VERSION {
        return Err(Error::UnsupportedStoreVersion {
            have: export.store_version,
        });
    }
    let user = export.user.ok_or(Error::Invalid("profile export"))?;
    kv.create_table(PROFILES).await?;
    kv.create_table(SESSIONS).await?;
    if get_user(kv, &user.name).await?.is_some() {
        return Err(Error::NameTaken);
    }
    let table = profile_table(export.profile_id);
    if !kv.create_table(&table).await? {
        return Err(Error::StoreExists);
    }
    let stored = async {
        let mut tx = kv.begin_write(&table).await?;
        for e in &export.entries {
            tx.put(&e.key, &e.value).await?;
        }
        tx.commit().await?;
        let mut tx = kv.begin_write(PROFILES).await?;
        if tx.get(&user.name).await?.is_some() {
            return Err(Error::NameTaken);
        }
        tx.put(&user.name, &user.encode_to_vec()).await?;
        tx.commit().await?;
        Ok(())
    }
    .await;
    if let Err(e) = stored {
        kv.delete_table(&table).await?;
        return Err(e);
    }
    Ok(user.name)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use futures_util::FutureExt;
    use strims_base::rng::det_rng;
    use strims_base::time::Time;
    use strims_kv::MemoryKv;
    use strims_proto::network::v1::Network;

    use super::*;
    use crate::auth::{list_users, remove_profile, sign_in, sign_up};
    use crate::schema::network::Networks;
    use crate::table::Table;

    const T0: Time = Time::from_unix_millis(1_700_000_000_000);

    fn block<T>(f: impl std::future::Future<Output = T>) -> T {
        f.now_or_never().unwrap()
    }

    /// Alice with a network in her store.
    fn alice(kv: &MemoryKv) {
        let (_, store) = block(sign_up(kv, "alice", "pw", "linux", &mut det_rng(1), T0)).unwrap();
        let mut tx = block(store.write(T0)).unwrap();
        let network = Network {
            id: 7,
            alias: "streams".into(),
            ..Default::default()
        };
        block(Networks::insert(&mut tx, network)).unwrap();
        block(tx.commit()).unwrap();
    }

    fn networks(kv: &MemoryKv, name: &str) -> Vec<String> {
        let pk = block(sign_in(kv, name, "pw")).unwrap();
        let store = block(ProfileStore::open(
            kv.clone(),
            pk.id,
            pk.storage_key(),
            det_rng(2),
        ))
        .unwrap();
        let mut tx = block(store.read()).unwrap();
        let all = block(Networks::get_all(&mut tx)).unwrap();
        all.into_iter().map(|n| n.alias).collect()
    }

    #[test]
    fn an_exported_profile_imports_whole_after_its_removal() {
        let kv = MemoryKv::new();
        assert!(block(list_users(&kv)).unwrap().is_empty());
        alice(&kv);
        block(sign_up(&kv, "bob", "pw", "linux", &mut det_rng(3), T0)).unwrap();
        assert_eq!(block(list_users(&kv)).unwrap(), ["alice", "bob"]);

        let file = block(export_profile(&kv, "alice", "pw", det_rng(4))).unwrap();
        block(remove_profile(&kv, "alice", "pw")).unwrap();
        assert_eq!(block(list_users(&kv)).unwrap(), ["bob"]);
        assert!(matches!(
            block(sign_in(&kv, "alice", "pw")),
            Err(Error::Credentials)
        ));

        assert_eq!(block(import_profile(&kv, &file)).unwrap(), "alice");
        assert_eq!(networks(&kv, "alice"), ["streams"]);
        assert_eq!(block(list_users(&kv)).unwrap(), ["alice", "bob"]);

        // into another store, as a move
        let other = MemoryKv::new();
        assert_eq!(block(import_profile(&other, &file)).unwrap(), "alice");
        assert_eq!(networks(&other, "alice"), ["streams"]);
    }

    #[test]
    fn imports_refuse_a_taken_name_a_held_store_and_other_versions() {
        let kv = MemoryKv::new();
        alice(&kv);
        let file = block(export_profile(&kv, "alice", "pw", det_rng(4))).unwrap();
        assert!(matches!(
            block(import_profile(&kv, &file)),
            Err(Error::NameTaken)
        ));

        // the store is still there under another name
        let mut renamed = ProfileExport::decode(file.as_slice()).unwrap();
        renamed.user.as_mut().unwrap().name = "alice2".into();
        assert!(matches!(
            block(import_profile(&kv, &renamed.encode_to_vec())),
            Err(Error::StoreExists)
        ));

        let other = MemoryKv::new();
        let mut old = ProfileExport::decode(file.as_slice()).unwrap();
        old.store_version = STORE_VERSION - 1;
        assert!(matches!(
            block(import_profile(&other, &old.encode_to_vec())),
            Err(Error::UnsupportedStoreVersion { have }) if have == STORE_VERSION - 1
        ));
        let mut foreign = ProfileExport::decode(file.as_slice()).unwrap();
        foreign.format = "something else".into();
        assert!(matches!(
            block(import_profile(&other, &foreign.encode_to_vec())),
            Err(Error::Invalid(_))
        ));
        assert!(block(list_users(&other)).unwrap().is_empty());
    }

    #[test]
    fn exports_and_removals_need_the_password() {
        let kv = MemoryKv::new();
        alice(&kv);
        assert!(matches!(
            block(export_profile(&kv, "alice", "nope", det_rng(4))),
            Err(Error::Credentials)
        ));
        assert!(matches!(
            block(remove_profile(&kv, "alice", "nope")),
            Err(Error::Credentials)
        ));
        assert!(matches!(
            block(remove_profile(&kv, "carol", "pw")),
            Err(Error::Credentials)
        ));
        assert_eq!(networks(&kv, "alice"), ["streams"]);
    }
}
