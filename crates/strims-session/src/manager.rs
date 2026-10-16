// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::future::{Future, poll_fn};
use std::rc::Rc;
use std::task::{Poll, Waker};

use strims_base::rng::{DetRng, fork};
use strims_base::time::Time;
use strims_dao::auth::ProfileKey;
use strims_dao::schema::profile::{Devices, Profile};
use strims_dao::{Error, ProfileStore, ReplicatedStore, Singleton, Table};
use strims_kv::Kv;
use strims_proto::profile::v1 as profile;
use strims_rpc::{Router, Timer};

/// A signed-in profile on this device.
pub struct Session<K: Kv> {
    /// The profile, with its key.
    pub profile: profile::Profile,
    /// The profile's store, writing as this device.
    pub store: ReplicatedStore<K>,
    key: ProfileKey,
}

/// How the driver runs a session.
pub trait SessionFactory<K: Kv>: 'static {
    /// What the driver runs for a session: its node, replication and
    /// services.
    type Runtime: 'static;

    /// Starts a session's runtime.
    fn start(&self, session: &Rc<Session<K>>)
    -> impl Future<Output = Result<Self::Runtime, Error>>;

    /// Serves a session's frontend services on a frontend connection's
    /// router.
    fn bind(&self, runtime: &Self::Runtime, session: &Rc<Session<K>>, router: &Router);
}

enum Slot<K: Kv, R> {
    Starting(Vec<Waker>),
    Ready(Rc<Session<K>>, Rc<R>),
}

/// A session start in progress. Dropping it, done or cancelled, settles
/// the profile's slot and wakes the callers waiting on it.
struct Starting<'a, K: Kv, R> {
    sessions: &'a RefCell<BTreeMap<u64, Slot<K, R>>>,
    id: u64,
    ready: Option<(Rc<Session<K>>, Rc<R>)>,
}

impl<K: Kv, R> Drop for Starting<'_, K, R> {
    fn drop(&mut self) {
        let waiters = {
            let mut sessions = self.sessions.borrow_mut();
            let prev = match self.ready.take() {
                Some((s, r)) => sessions.insert(self.id, Slot::Ready(s, r)),
                None => sessions.remove(&self.id),
            };
            match prev {
                Some(Slot::Starting(w)) => w,
                _ => Vec::new(),
            }
        };
        for w in waiters {
            w.wake();
        }
    }
}

/// Opens profiles' sessions, one per profile.
pub struct Manager<K: Kv + Clone, F: SessionFactory<K>, T: Timer> {
    kv: K,
    factory: F,
    timer: T,
    os: String,
    rng: RefCell<DetRng>,
    sessions: RefCell<BTreeMap<u64, Slot<K, F::Runtime>>>,
}

impl<K: Kv + Clone, F: SessionFactory<K>, T: Timer> Manager<K, F, T> {
    /// A manager of the profiles in `kv`, on a device running `os`. `rng`
    /// draws the nonces stores seal values under, so outside simulations
    /// it must be seeded from OS entropy.
    pub fn new(kv: K, factory: F, timer: T, os: &str, rng: DetRng) -> Self {
        Manager {
            kv,
            factory,
            timer,
            os: os.to_owned(),
            rng: RefCell::new(rng),
            sessions: RefCell::new(BTreeMap::new()),
        }
    }

    /// The kv store holding the profiles.
    pub fn kv(&self) -> &K {
        &self.kv
    }

    /// The driver's session factory.
    pub fn factory(&self) -> &F {
        &self.factory
    }

    /// The driver's clock.
    pub fn timer(&self) -> &T {
        &self.timer
    }

    /// The operating system this device reports.
    pub fn os(&self) -> &str {
        &self.os
    }

    /// Draws a child rng.
    pub fn fork_rng(&self) -> DetRng {
        fork(&mut self.rng.borrow_mut())
    }

    /// The running session of profile `pk`, started if needed. Callers for
    /// one profile share one start. Fails with [`Error::Credentials`] if
    /// the profile's session runs under another key.
    pub async fn get_or_create(
        &self,
        pk: &ProfileKey,
    ) -> Result<(Rc<Session<K>>, Rc<F::Runtime>), Error> {
        // A waiter woken by a failed start finds the slot empty and starts
        // the session itself.
        let running = poll_fn(|cx| {
            let mut sessions = self.sessions.borrow_mut();
            match sessions.get_mut(&pk.id) {
                Some(Slot::Ready(s, r)) => Poll::Ready(Some((s.clone(), r.clone()))),
                Some(Slot::Starting(waiters)) => {
                    waiters.push(cx.waker().clone());
                    Poll::Pending
                }
                None => {
                    sessions.insert(pk.id, Slot::Starting(Vec::new()));
                    Poll::Ready(None)
                }
            }
        })
        .await;
        if let Some((session, runtime)) = running {
            if session.key != *pk {
                return Err(Error::Credentials);
            }
            return Ok((session, runtime));
        }

        let mut starting = Starting {
            sessions: &self.sessions,
            id: pk.id,
            ready: None,
        };
        let result = self.start(pk).await;
        if let Ok((s, r)) = &result {
            starting.ready = Some((s.clone(), r.clone()));
        }
        result
    }

    async fn start(&self, pk: &ProfileKey) -> Result<(Rc<Session<K>>, Rc<F::Runtime>), Error> {
        let rng = self.fork_rng();
        let store = ProfileStore::open(self.kv.clone(), pk.id, pk.storage_key(), rng).await?;
        let store = ReplicatedStore::new(store).await?;
        let now: Time = self.timer.now();
        let device = store.replica_id();
        let mut tx = store.write(now).await?;
        let os = self.os.clone();
        Devices::transform(&mut tx, device, |d| {
            d.id = device;
            d.os = os;
            d.last_login = now.unix();
            Ok(())
        })
        .await?;
        let profile = Profile::get(&mut tx).await?.ok_or(Error::NotFound)?;
        tx.commit().await?;

        let session = Rc::new(Session {
            profile,
            store,
            key: pk.clone(),
        });
        let runtime = self.factory.start(&session).await?;
        Ok((session, Rc::new(runtime)))
    }
}
