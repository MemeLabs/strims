// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Profiles' sign-in records, outside the profiles' own stores.
//!
//! The kv table [`PROFILES`] maps each user name to a `ServerUserThing`:
//! the profile's id and key sealed under a key derived from the user's
//! password. The table [`SESSIONS`] maps `<eol>:<token>` to the profile's
//! id and key sealed under a session key, where `eol` is the session's
//! expiry in unix seconds and `token` its random token in padded url-safe
//! base64. A profile's key opens its store.

use data_encoding::BASE64URL;
use prost::Message;
use rand_core::Rng;
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_kv::{Kv, ReadTx as _, WriteTx as _};
use strims_proto::auth::v1::server_user_thing::{self, password};
use strims_proto::auth::v1::{PairingToken, ServerUserThing, SessionThing};
use strims_proto::dao::v1::VersionVector;
use strims_proto::profile::v1::{self as profile, Device};

use crate::error::Error;
use crate::ids;
use crate::schema::network::{BootstrapClients, Networks};
use crate::schema::profile::{Devices, Profile, ProfileIds};
use crate::seal::{KEY_SIZE, StorageKey};
use crate::store::ProfileStore;
use crate::table::{Singleton, Table};

/// The table of sign-in records by user name.
pub const PROFILES: &str = "profiles";
/// The table of persisted sessions.
pub const SESSIONS: &str = "sessions";
/// How long a persisted session lasts.
pub const SESSION_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// Size of a session token.
pub const SESSION_TOKEN_SIZE: usize = 32;
/// The most attempts at a profile id no other profile's store has.
const PROFILE_ID_ATTEMPTS: usize = 8;

/// A profile's id and the key that opens its store.
#[derive(Clone, PartialEq, Eq)]
pub struct ProfileKey {
    /// The profile's id.
    pub id: u64,
    /// The key of the profile's store.
    pub key: [u8; KEY_SIZE],
}

impl std::fmt::Debug for ProfileKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfileKey")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl ProfileKey {
    /// The profile key in `key`, which must be [`KEY_SIZE`] bytes.
    pub fn from_parts(id: u64, key: &[u8]) -> Result<ProfileKey, Error> {
        let key = key.try_into().map_err(|_| Error::Invalid("profile key"))?;
        Ok(ProfileKey { id, key })
    }

    /// The key that opens the profile's store.
    pub fn storage_key(&self) -> StorageKey {
        StorageKey::from_bytes(self.key)
    }
}

/// A persisted session's token.
#[derive(Clone, PartialEq, Eq)]
pub struct SessionToken {
    /// When the session expires, in unix seconds.
    pub eol: u64,
    /// The session's random token.
    pub token: Vec<u8>,
}

impl std::fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionToken")
            .field("eol", &self.eol)
            .finish_non_exhaustive()
    }
}

impl SessionToken {
    fn key(&self) -> String {
        format!("{}:{}", self.eol, BASE64URL.encode(&self.token))
    }
}

/// A new profile named `name` with a new signing key.
pub fn new_profile(name: &str, rng: &mut DetRng, now: Time) -> profile::Profile {
    profile::Profile {
        id: ids::new_profile_id(now, rng),
        name: name.to_owned(),
        key: Some(strims_crypto::generate_key(rng)),
        ..Default::default()
    }
}

/// Creates a profile `name` protected by `password`, with a store and a
/// first device running `os`.
pub async fn sign_up<K: Kv + Clone>(
    kv: &K,
    name: &str,
    password: &str,
    os: &str,
    rng: &mut DetRng,
    now: Time,
) -> Result<(ProfileKey, ProfileStore<K>), Error> {
    let mut key = [0u8; KEY_SIZE];
    rng.fill_bytes(&mut key);
    let (auth_key, record) = StorageKey::new_password_key(password, rng);

    let mut last = Error::StoreExists;
    for _ in 0..PROFILE_ID_ATTEMPTS {
        let profile = new_profile(name, rng, now);
        let pk = ProfileKey {
            id: profile.id,
            key,
        };
        let secret = password::Secret {
            profile_id: pk.id,
            profile_key: key.to_vec(),
            totp: None,
        };
        let user = ServerUserThing {
            id: 0,
            name: name.to_owned(),
            credentials: Some(server_user_thing::Credentials::Password(
                server_user_thing::Password {
                    auth_key: Some(record.clone()),
                    totp_required: false,
                    secret: auth_key.seal(rng, &secret.encode_to_vec())?,
                },
            )),
        };
        match store_profile(kv, &user, &profile, &pk, rng, now).await {
            Ok(store) => {
                init_device(&store, os, now).await?;
                return Ok((pk, store));
            }
            Err(Error::StoreExists) => last = Error::StoreExists,
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

/// Writes `user`'s sign-in record and creates the profile's store with
/// `profile` in it. Fails with [`Error::NameTaken`] if the name has a
/// record, and with [`Error::StoreExists`] if the profile has a store.
pub async fn store_profile<K: Kv + Clone>(
    kv: &K,
    user: &ServerUserThing,
    profile: &profile::Profile,
    pk: &ProfileKey,
    rng: &mut DetRng,
    now: Time,
) -> Result<ProfileStore<K>, Error> {
    kv.create_table(PROFILES).await?;
    kv.create_table(SESSIONS).await?;
    if kv
        .begin_read(PROFILES)
        .await?
        .get(&user.name)
        .await?
        .is_some()
    {
        return Err(Error::NameTaken);
    }

    let store = ProfileStore::create(
        kv.clone(),
        pk.id,
        pk.storage_key(),
        strims_base::rng::fork(rng),
        now,
    )
    .await?;
    let mut tx = store.write(now).await?;
    Profile::set(&mut tx, profile.clone()).await?;
    tx.commit().await?;

    let mut tx = kv.begin_write(PROFILES).await?;
    if tx.get(&user.name).await?.is_some() {
        drop(tx);
        store.delete().await?;
        return Err(Error::NameTaken);
    }
    tx.put(&user.name, &user.encode_to_vec()).await?;
    tx.commit().await?;
    Ok(store)
}

/// Gives the profile in `store` a device running `os` if it has none.
pub async fn init_device<K: Kv>(store: &ProfileStore<K>, os: &str, now: Time) -> Result<(), Error> {
    let mut tx = store.write(now).await?;
    let mut profile = Profile::get(&mut tx).await?.ok_or(Error::NotFound)?;
    if profile.device_id != 0 {
        return Ok(());
    }
    let id = tx.generate_id().await?;
    Devices::insert(
        &mut tx,
        Device {
            id,
            version: Some(VersionVector::default()),
            device: String::new(),
            os: os.to_owned(),
            last_login: 0,
        },
    )
    .await?;
    profile.device_id = id;
    Profile::set(&mut tx, profile).await?;
    tx.commit().await
}

/// The sign-in record of `name`.
pub async fn get_user<K: Kv>(kv: &K, name: &str) -> Result<Option<ServerUserThing>, Error> {
    let mut tx = match kv.begin_read(PROFILES).await {
        Ok(tx) => tx,
        Err(strims_kv::Error::NoTable) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match tx.get(name).await? {
        Some(b) => Ok(Some(ServerUserThing::decode(b.as_slice())?)),
        None => Ok(None),
    }
}

/// The profile `user` signs in to with `password`. Fails with
/// [`Error::Credentials`] if the password is wrong.
pub fn open_user(user: &ServerUserThing, password: &str) -> Result<ProfileKey, Error> {
    match &user.credentials {
        Some(server_user_thing::Credentials::Password(p)) => {
            let record = p
                .auth_key
                .as_ref()
                .ok_or(Error::Invalid("sign-in record"))?;
            let key = StorageKey::from_password(password, record)?;
            let secret = key.open(&p.secret).map_err(|_| Error::Credentials)?;
            let secret = password::Secret::decode(secret.as_slice())?;
            ProfileKey::from_parts(secret.profile_id, &secret.profile_key)
        }
        Some(server_user_thing::Credentials::Unencrypted(u)) => {
            ProfileKey::from_parts(u.profile_id, &u.profile_key)
        }
        None => Err(Error::Invalid("sign-in record")),
    }
}

/// The profile `name` signs in to with `password`. Fails with
/// [`Error::Credentials`] if there is no such user or the password is
/// wrong.
pub async fn sign_in<K: Kv>(kv: &K, name: &str, password: &str) -> Result<ProfileKey, Error> {
    let user = get_user(kv, name).await?.ok_or(Error::Credentials)?;
    open_user(&user, password)
}

/// Adds the profile a pairing token carries, as a new device of it: its
/// sign-in record, and a store holding the profile, the token's id ranges,
/// and its devices, networks and bootstrap clients as they were written.
pub async fn import_pairing_token<K: Kv + Clone>(
    kv: &K,
    token: &PairingToken,
    pk: &ProfileKey,
    rng: &mut DetRng,
    now: Time,
) -> Result<ProfileStore<K>, Error> {
    let user = token.auth.as_ref().ok_or(Error::Invalid("pairing token"))?;
    let profile = token
        .profile
        .as_ref()
        .ok_or(Error::Invalid("pairing token"))?;
    let ranges = token
        .profile_id
        .clone()
        .ok_or(Error::Invalid("pairing token"))?;
    ids::validate(&ranges)?;
    if profile.id != pk.id || profile.device_id == 0 {
        return Err(Error::Invalid("pairing token"));
    }

    let store = store_profile(kv, user, profile, pk, rng, now).await?;
    let mut tx = store.write(now).await?;
    ProfileIds::set(&mut tx, ranges).await?;
    for d in &token.devices {
        Devices::insert(&mut tx, d.clone()).await?;
    }
    for n in &token.networks {
        Networks::insert(&mut tx, n.clone()).await?;
    }
    for b in &token.bootstraps {
        BootstrapClients::insert(&mut tx, b.clone()).await?;
    }
    tx.commit().await?;
    Ok(store)
}

/// Persists a session for profile `pk` under `session_key`. Also removes
/// expired sessions.
pub async fn create_session<K: Kv>(
    kv: &K,
    session_key: &StorageKey,
    pk: &ProfileKey,
    rng: &mut DetRng,
    now: Time,
) -> Result<SessionToken, Error> {
    kv.create_table(SESSIONS).await?;
    purge_expired_sessions(kv, now).await?;

    let mut token = vec![0u8; SESSION_TOKEN_SIZE];
    rng.fill_bytes(&mut token);
    let t = SessionToken {
        eol: u64::try_from((now + SESSION_TTL).unix()).unwrap_or(0),
        token,
    };
    let thing = SessionThing {
        profile_id: pk.id,
        profile_key: pk.key.to_vec(),
    };
    let sealed = session_key.seal(rng, &thing.encode_to_vec())?;
    let mut tx = kv.begin_write(SESSIONS).await?;
    tx.put(&t.key(), &sealed).await?;
    tx.commit().await?;
    Ok(t)
}

/// The profile session `t` signs in to. Fails with [`Error::Credentials`]
/// if the session doesn't exist, has expired, or wasn't persisted under
/// `session_key`.
pub async fn load_session<K: Kv>(
    kv: &K,
    session_key: &StorageKey,
    t: &SessionToken,
    now: Time,
) -> Result<ProfileKey, Error> {
    if i64::try_from(t.eol).unwrap_or(i64::MAX) <= now.unix() {
        return Err(Error::Credentials);
    }
    let mut tx = match kv.begin_read(SESSIONS).await {
        Ok(tx) => tx,
        Err(strims_kv::Error::NoTable) => return Err(Error::Credentials),
        Err(e) => return Err(e.into()),
    };
    let sealed = tx.get(&t.key()).await?.ok_or(Error::Credentials)?;
    drop(tx);
    let thing = session_key.open(&sealed).map_err(|_| Error::Credentials)?;
    let thing = SessionThing::decode(thing.as_slice())?;
    ProfileKey::from_parts(thing.profile_id, &thing.profile_key)
}

/// Removes sessions that expired at or before `now`. Resolves to how many
/// it removed.
pub async fn purge_expired_sessions<K: Kv>(kv: &K, now: Time) -> Result<usize, Error> {
    let mut tx = match kv.begin_write(SESSIONS).await {
        Ok(tx) => tx,
        Err(strims_kv::Error::NoTable) => return Ok(0),
        Err(e) => return Err(e.into()),
    };
    let mut n = 0;
    for (key, _) in tx.scan("", None, usize::MAX).await? {
        let eol = key.split(':').next().and_then(|e| e.parse::<i64>().ok());
        if eol.is_none_or(|eol| eol <= now.unix()) {
            tx.delete(&key).await?;
            n += 1;
        }
    }
    tx.commit().await?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use futures_util::FutureExt;
    use strims_base::rng::det_rng;
    use strims_kv::MemoryKv;

    use super::*;
    use crate::schema::network::Networks;

    const T0: Time = Time::from_unix_millis(1_700_000_000_000);

    fn block<T>(f: impl std::future::Future<Output = T>) -> T {
        f.now_or_never().unwrap()
    }

    #[test]
    fn sign_up_then_sign_in() {
        let kv = MemoryKv::new();
        let mut rng = det_rng(1);
        let (pk, store) = block(sign_up(&kv, "alice", "pw", "linux", &mut rng, T0)).unwrap();
        assert_eq!(store.profile_id(), pk.id);
        assert_eq!(block(sign_in(&kv, "alice", "pw")).unwrap(), pk);
        assert!(matches!(
            block(sign_in(&kv, "alice", "nope")),
            Err(Error::Credentials)
        ));
        assert!(matches!(
            block(sign_in(&kv, "bob", "pw")),
            Err(Error::Credentials)
        ));
        assert!(matches!(
            block(sign_up(&kv, "alice", "pw2", "linux", &mut rng, T0)),
            Err(Error::NameTaken)
        ));
        assert_eq!(
            kv.dump().len(),
            3,
            "the losing sign-up leaves no store behind"
        );

        let store = block(ProfileStore::open(
            kv.clone(),
            pk.id,
            pk.storage_key(),
            det_rng(2),
        ))
        .unwrap();
        block(async {
            let mut tx = store.read().await.unwrap();
            let p = Profile::get(&mut tx).await.unwrap().unwrap();
            assert_eq!(p.name, "alice");
            assert_ne!(p.device_id, 0);
            assert_eq!(
                Devices::get(&mut tx, p.device_id)
                    .await
                    .unwrap()
                    .unwrap()
                    .os,
                "linux"
            );
        });
    }

    #[test]
    fn sessions_open_until_they_expire() {
        let kv = MemoryKv::new();
        let mut rng = det_rng(1);
        let sk = StorageKey::from_bytes([3; 32]);
        let pk = ProfileKey {
            id: 9,
            key: [4; 32],
        };
        let t = block(create_session(&kv, &sk, &pk, &mut rng, T0)).unwrap();
        assert_eq!(t.eol, (T0 + SESSION_TTL).unix() as u64);
        assert_eq!(block(load_session(&kv, &sk, &t, T0)).unwrap(), pk);

        let other = StorageKey::from_bytes([5; 32]);
        assert!(matches!(
            block(load_session(&kv, &other, &t, T0)),
            Err(Error::Credentials)
        ));
        let forged = SessionToken {
            eol: t.eol + 1,
            token: t.token.clone(),
        };
        assert!(matches!(
            block(load_session(&kv, &sk, &forged, T0)),
            Err(Error::Credentials)
        ));
        let later = T0 + SESSION_TTL;
        assert!(matches!(
            block(load_session(&kv, &sk, &t, later)),
            Err(Error::Credentials)
        ));

        block(create_session(&kv, &sk, &pk, &mut rng, later)).unwrap();
        assert_eq!(
            kv.dump()[SESSIONS].len(),
            1,
            "creating a session purges expired ones"
        );
    }

    #[test]
    fn pairing_tokens_import_a_profile() {
        let kv = MemoryKv::new();
        let mut rng = det_rng(1);
        let profile = profile::Profile {
            device_id: 50,
            ..new_profile("carol", &mut rng, T0)
        };
        let pk = ProfileKey {
            id: profile.id,
            key: [6; 32],
        };
        let token = PairingToken {
            auth: Some(ServerUserThing {
                name: "carol".into(),
                credentials: Some(server_user_thing::Credentials::Unencrypted(
                    server_user_thing::Unencrypted {
                        profile_id: pk.id,
                        profile_key: pk.key.to_vec(),
                    },
                )),
                ..Default::default()
            }),
            profile: Some(profile.clone()),
            networks: vec![strims_proto::network::v1::Network {
                id: 7,
                ..Default::default()
            }],
            bootstraps: vec![],
            devices: vec![Device {
                id: 3,
                os: "android".into(),
                ..Default::default()
            }],
            profile_id: ids::from_ranges(&[(1000, 2000)]),
        };
        let store = block(import_pairing_token(&kv, &token, &pk, &mut rng, T0)).unwrap();
        assert_eq!(block(sign_in(&kv, "carol", "")).unwrap(), pk);
        block(async {
            let mut tx = store.read().await.unwrap();
            assert_eq!(Profile::get(&mut tx).await.unwrap().unwrap(), profile);
            assert_eq!(
                Devices::get(&mut tx, 3).await.unwrap().unwrap().os,
                "android"
            );
            assert!(Networks::get(&mut tx, 7).await.unwrap().is_some());
            assert_eq!(
                ProfileIds::get(&mut tx).await.unwrap().unwrap().next_id,
                1000
            );
        });

        let mut bad = token.clone();
        bad.profile_id = ids::from_ranges(&[(5, 1)]);
        assert!(
            block(import_pairing_token(
                &MemoryKv::new(),
                &bad,
                &pk,
                &mut rng,
                T0
            ))
            .is_err()
        );
    }
}
