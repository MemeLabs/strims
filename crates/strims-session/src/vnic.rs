// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! This device's vnic config as its node's limits.
//!
//! The stored config ([`VnicConfig`]) holds the upload cap and the peer
//! budget the UI sets, and a stored 0 is unlimited. The driver may cap
//! each with [`VnicCeilings`]. [`vnic_limits`] combines the two into the
//! [`VnicLimits`] the node runs under: the driver starts its node at
//! [`read_vnic_limits`], then [`run_vnic`] keeps the node at the limits as
//! the config changes. [`VnicFrontend`] serves the stored config to the UI.

use std::fmt::Display;
use std::future::Future;

use futures_util::StreamExt;
use strims_dao::events::Record;
use strims_dao::schema::vnic::VnicConfig;
use strims_dao::{Recv, ReplicatedStore, Singleton, StoreEvent};
use strims_kv::Kv;
use strims_proto::service::{Context, Error as RpcError};
use strims_proto::vnic::v1::{
    Config, GetConfigRequest, GetConfigResponse, SetConfigRequest, SetConfigResponse,
    VnicFrontendService,
};
use strims_rpc::Timer;

use crate::rpc_error;

/// The driver's caps on the stored vnic config. `None`, or 0, leaves a
/// field as stored.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VnicCeilings {
    /// The largest upload cap, in bytes per second.
    pub max_upload_bytes_per_second: Option<u64>,
    /// The largest peer budget.
    pub max_peers: Option<u32>,
}

/// The limits a node runs under. `None` is unlimited, and
/// [`vnic_limits`] never makes `Some(0)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VnicLimits {
    /// The upload cap, in bytes per second.
    pub upload: Option<u64>,
    /// The peer budget, which gates peer exchange.
    pub max_peers: Option<u32>,
}

/// The limits of the stored `config` under `ceilings`: each field is the
/// smaller of its stored value and its ceiling, where 0 in either is no
/// limit.
pub fn vnic_limits(config: &Config, ceilings: &VnicCeilings) -> VnicLimits {
    VnicLimits {
        upload: capped(
            config.max_upload_bytes_per_second,
            ceilings.max_upload_bytes_per_second,
        ),
        max_peers: capped(config.max_peers, ceilings.max_peers),
    }
}

/// The smaller of `stored` and `ceiling`, where 0 in either is no limit.
fn capped<T: Copy + Ord + Default>(stored: T, ceiling: Option<T>) -> Option<T> {
    std::iter::once(stored)
        .chain(ceiling)
        .filter(|&v| v != T::default())
        .min()
}

/// The node a profile's vnic config applies to.
pub trait VnicHost {
    type Error: Display;

    /// Puts the node under `limits`.
    fn set_vnic_limits(&self, limits: VnicLimits) -> impl Future<Output = Result<(), Self::Error>>;
}

/// The limits of the vnic config in `store`, under `ceilings`.
pub async fn read_vnic_limits<K: Kv>(
    store: &ReplicatedStore<K>,
    ceilings: &VnicCeilings,
) -> Result<VnicLimits, strims_dao::Error> {
    let mut tx = store.read().await?;
    let config = VnicConfig::get(&mut tx).await?.unwrap_or_default();
    Ok(vnic_limits(&config, ceilings))
}

/// Keeps `host` at the limits of the vnic config in `store`, under
/// `ceilings`, until dropped. The limits apply at start and again whenever
/// they change. A failed read or apply is logged, and the next change of
/// the config applies it again.
pub async fn run_vnic<K: Kv, H: VnicHost>(
    store: ReplicatedStore<K>,
    host: H,
    ceilings: VnicCeilings,
) {
    let mut events = store.events().subscribe();
    let mut applied = None;
    let mut reread = true;
    loop {
        if reread {
            reread = false;
            match read_vnic_limits(&store, &ceilings).await {
                Ok(limits) => apply(&host, limits, &mut applied).await,
                Err(e) => tracing::warn!(error = %e, "reading the vnic config"),
            }
        }
        match events.next().await {
            Some(Recv::Event(e)) => {
                if let StoreEvent::Changed(Record::VnicConfig(config)) = &*e {
                    apply(&host, vnic_limits(config, &ceilings), &mut applied).await;
                }
            }
            Some(Recv::Lagged) => reread = true,
            None => return,
        }
    }
}

/// Puts `host` under `limits` unless `applied`, the limits it last took,
/// equal them.
async fn apply<H: VnicHost>(host: &H, limits: VnicLimits, applied: &mut Option<VnicLimits>) {
    if *applied == Some(limits) {
        return;
    }
    match host.set_vnic_limits(limits).await {
        Ok(()) => *applied = Some(limits),
        Err(e) => {
            *applied = None;
            tracing::warn!(error = %e, "applying the vnic config");
        }
    }
}

/// Serves `strims.vnic.v1.VNICFrontend` for a profile's store: the stored
/// vnic config, which the node runs under within the driver's ceilings.
pub struct VnicFrontend<K: Kv, T: Timer> {
    store: ReplicatedStore<K>,
    timer: T,
}

impl<K: Kv, T: Timer> VnicFrontend<K, T> {
    /// The service of `store`.
    pub fn new(store: ReplicatedStore<K>, timer: T) -> Self {
        VnicFrontend { store, timer }
    }
}

impl<K: Kv, T: Timer> VnicFrontendService for VnicFrontend<K, T> {
    async fn get_config(
        &self,
        _: &Context,
        _: GetConfigRequest,
    ) -> Result<GetConfigResponse, RpcError> {
        let mut tx = self.store.read().await.map_err(rpc_error)?;
        let config = VnicConfig::get(&mut tx).await.map_err(rpc_error)?;
        Ok(GetConfigResponse {
            config: Some(config.unwrap_or_default()),
        })
    }

    /// Stores any value: 0 is unlimited, and a cap past what the node can
    /// send caps nothing.
    async fn set_config(
        &self,
        _: &Context,
        req: SetConfigRequest,
    ) -> Result<SetConfigResponse, RpcError> {
        let config = req.config.ok_or_else(|| rpc_error("config required"))?;
        let mut tx = self
            .store
            .write(self.timer.now())
            .await
            .map_err(rpc_error)?;
        let config = VnicConfig::set(&mut tx, config).await.map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(SetConfigResponse {
            config: Some(config),
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use std::cell::RefCell;
    use std::convert::Infallible;
    use std::pin::pin;
    use std::rc::Rc;
    use std::task::{Context as TaskContext, Poll, Waker};

    use futures_util::FutureExt;
    use strims_base::rng::det_rng;
    use strims_base::time::Time;
    use strims_dao::ProfileStore;
    use strims_dao::events::SUBSCRIBER_CAPACITY;
    use strims_dao::schema::profile::Profile as ProfileRecord;
    use strims_dao::schema::vnic::{DEFAULT_MAX_PEERS, DEFAULT_MAX_UPLOAD_BYTES_PER_SECOND};
    use strims_dao::seal::StorageKey;
    use strims_kv::MemoryKv;
    use strims_proto::profile::v1::Profile;

    use super::*;

    const T0: Time = Time::from_unix_millis(1_700_000_000_000);

    fn block<T>(f: impl Future<Output = T>) -> T {
        f.now_or_never().expect("memory store operations are ready")
    }

    fn store() -> ReplicatedStore<MemoryKv> {
        block(async {
            let key = StorageKey::from_bytes([1; 32]);
            let store = ProfileStore::create(MemoryKv::new(), 1, key, det_rng(1), T0)
                .await
                .unwrap();
            let mut tx = store.write(T0).await.unwrap();
            let profile = Profile {
                id: 1,
                device_id: 7,
                ..Default::default()
            };
            ProfileRecord::set(&mut tx, profile).await.unwrap();
            tx.commit().await.unwrap();
            ReplicatedStore::new(store).await.unwrap()
        })
    }

    fn set(store: &ReplicatedStore<MemoryKv>, config: Config) {
        block(async {
            let mut tx = store.write(T0).await.unwrap();
            VnicConfig::set(&mut tx, config).await.unwrap();
            tx.commit().await.unwrap();
        });
    }

    fn config(upload: u64, peers: u32) -> Config {
        Config {
            max_upload_bytes_per_second: upload,
            max_peers: peers,
        }
    }

    fn ceilings(upload: Option<u64>, peers: Option<u32>) -> VnicCeilings {
        VnicCeilings {
            max_upload_bytes_per_second: upload,
            max_peers: peers,
        }
    }

    fn limits(upload: Option<u64>, peers: Option<u32>) -> VnicLimits {
        VnicLimits {
            upload,
            max_peers: peers,
        }
    }

    #[test]
    fn a_stored_zero_is_unlimited_and_a_ceiling_caps_the_stored_value() {
        let cases = [
            (config(0, 0), ceilings(None, None), limits(None, None)),
            (
                config(0, 0),
                ceilings(Some(100), Some(5)),
                limits(Some(100), Some(5)),
            ),
            (
                config(1000, 25),
                ceilings(None, None),
                limits(Some(1000), Some(25)),
            ),
            (
                config(1000, 25),
                ceilings(Some(100), Some(5)),
                limits(Some(100), Some(5)),
            ),
            (
                config(50, 3),
                ceilings(Some(100), Some(5)),
                limits(Some(50), Some(3)),
            ),
            (
                config(50, 3),
                ceilings(Some(0), Some(0)),
                limits(Some(50), Some(3)),
            ),
            (
                config(u64::MAX, u32::MAX),
                ceilings(None, Some(5)),
                limits(Some(u64::MAX), Some(5)),
            ),
        ];
        for (stored, caps, want) in cases {
            assert_eq!(vnic_limits(&stored, &caps), want, "{stored:?} {caps:?}");
        }
    }

    #[derive(Clone, Default)]
    struct FakeHost(Rc<RefCell<Vec<VnicLimits>>>);

    impl FakeHost {
        fn take(&self) -> Vec<VnicLimits> {
            std::mem::take(&mut self.0.borrow_mut())
        }
    }

    impl VnicHost for FakeHost {
        type Error = Infallible;

        async fn set_vnic_limits(&self, limits: VnicLimits) -> Result<(), Infallible> {
            self.0.borrow_mut().push(limits);
            Ok(())
        }
    }

    #[test]
    fn run_vnic_applies_the_limits_at_start_and_on_each_change() {
        let store = store();
        let host = FakeHost::default();
        let mut run = pin!(run_vnic(
            store.clone(),
            host.clone(),
            ceilings(None, Some(30))
        ));
        let mut cx = TaskContext::from_waker(Waker::noop());
        let mut poll = || assert_eq!(run.as_mut().poll(&mut cx), Poll::Pending);

        poll();
        let default = limits(
            Some(DEFAULT_MAX_UPLOAD_BYTES_PER_SECOND),
            Some(DEFAULT_MAX_PEERS),
        );
        assert_eq!(host.take(), [default]);

        set(&store, config(1000, 50));
        poll();
        assert_eq!(host.take(), [limits(Some(1000), Some(30))]);

        // the ceiling makes these the limits already applied
        set(&store, config(1000, 40));
        poll();
        assert!(host.take().is_empty());

        set(&store, config(0, 0));
        poll();
        assert_eq!(host.take(), [limits(None, Some(30))]);

        // a lagged subscription rereads the config once
        for i in 0..=SUBSCRIBER_CAPACITY {
            set(&store, config(2000 + i as u64, 10));
        }
        poll();
        let last = 2000 + SUBSCRIBER_CAPACITY as u64;
        assert_eq!(host.take(), [limits(Some(last), Some(10))]);
    }

    #[derive(Clone)]
    struct FixedTimer;

    impl Timer for FixedTimer {
        type Sleep = std::future::Pending<()>;

        fn now(&self) -> Time {
            T0
        }

        fn sleep_until(&self, _: Time) -> Self::Sleep {
            std::future::pending()
        }
    }

    #[test]
    fn get_config_returns_the_default_before_a_config_is_stored() {
        let svc = VnicFrontend::new(store(), FixedTimer);
        let res = block(svc.get_config(&Context::default(), GetConfigRequest {})).unwrap();
        let want = config(DEFAULT_MAX_UPLOAD_BYTES_PER_SECOND, DEFAULT_MAX_PEERS);
        assert_eq!(res.config, Some(want));
    }

    #[test]
    fn set_config_stores_and_echoes_any_value() {
        let store = store();
        let svc = VnicFrontend::new(store.clone(), FixedTimer);
        let cx = Context::default();
        for c in [config(u64::MAX, 0), config(0, u32::MAX), config(1, 1)] {
            let req = SetConfigRequest { config: Some(c) };
            let res = block(svc.set_config(&cx, req)).unwrap();
            assert_eq!(res.config, Some(c));
            let got = block(svc.get_config(&cx, GetConfigRequest {})).unwrap();
            assert_eq!(got.config, Some(c));
        }
        let missing = SetConfigRequest { config: None };
        assert!(block(svc.set_config(&cx, missing)).is_err());
        let mut tx = block(store.read()).unwrap();
        assert_eq!(block(VnicConfig::get(&mut tx)).unwrap(), Some(config(1, 1)));
    }
}
