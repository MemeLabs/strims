// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! One peer device: the requests it sends, and what this device sends it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use strims_dao::replication::delta::chunk_bootstrap;
use strims_dao::replication::{
    self, Bootstrap, apply_bootstrap, apply_event_logs, checkpoint, compressed_delta, dump,
    free_ids, merge_checkpoints, push_ids,
};
use strims_dao::store::STORE_VERSION;
use strims_dao::{ReplicatedStore, vv};
use strims_kv::Kv;
use strims_proto::dao::v1::VersionVector;
use strims_proto::replication::v1::{
    Checkpoint, EventLog, PeerAllocateProfileIDsRequest, PeerAllocateProfileIDsResponse,
    PeerBootstrapRequest, PeerBootstrapResponse, PeerOpenRequest, PeerOpenResponse,
    PeerSyncRequest, PeerSyncResponse, ReplicationPeerClient, ReplicationPeerService,
};
use strims_proto::service::{CallError, Context, Error as RpcError};
use strims_rpc::{Client, RetryClient, Timer};

use crate::limits::{check_bootstrap, check_sync};
use crate::queue::{Push, PushQueue};
use crate::{Config, Metrics};

/// Why replication to a peer stopped.
#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    /// A call failed.
    #[error("call failed: {0}")]
    Call(#[from] CallError),
    /// The store failed.
    #[error(transparent)]
    Store(#[from] strims_dao::Error),
    /// The peer's store has another format version.
    #[error("peer store version {0} is not supported")]
    Incompatible(u32),
    /// The peer is a deleted device.
    #[error("peer replica {0} is deleted")]
    Deleted(u64),
    /// The peer answered something malformed.
    #[error("invalid answer from peer")]
    Invalid,
}

/// The most ids one peer is given at a time.
pub const MAX_ALLOCATION: u64 = 10_000;

fn rpc_error(e: impl std::fmt::Display) -> RpcError {
    RpcError::new(0, e.to_string())
}

/// Serves one peer's requests.
pub(crate) struct PeerService<K: Kv, T: Timer> {
    pub(crate) store: ReplicatedStore<K>,
    pub(crate) timer: T,
    pub(crate) metrics: Rc<RefCell<Metrics>>,
    /// The peer's replica, once it has opened.
    pub(crate) opened: Cell<Option<u64>>,
}

impl<K: Kv, T: Timer> PeerService<K, T> {
    /// The peer's replica, if it has opened and isn't deleted.
    async fn opened(&self) -> Result<u64, RpcError> {
        let replica = self
            .opened
            .get()
            .ok_or_else(|| rpc_error("replication not open"))?;
        let c = checkpoint(self.store.store(), replica)
            .await
            .map_err(rpc_error)?;
        if c.is_some_and(|c| c.deleted) {
            return Err(rpc_error("replica is deleted"));
        }
        Ok(replica)
    }

    fn refuse(&self, e: impl std::fmt::Display) -> RpcError {
        self.metrics.borrow_mut().refused += 1;
        rpc_error(e)
    }
}

impl<K: Kv, T: Timer> ReplicationPeerService for PeerService<K, T> {
    async fn open(&self, _: &Context, req: PeerOpenRequest) -> Result<PeerOpenResponse, RpcError> {
        let own = self.store.replica_id();
        if req.store_version != STORE_VERSION {
            return Err(self.refuse(format!(
                "store version {} is not supported",
                req.store_version
            )));
        }
        if req.replica_id == 0 || req.replica_id == own {
            return Err(self.refuse("invalid replica id"));
        }
        let c = checkpoint(self.store.store(), req.replica_id)
            .await
            .map_err(rpc_error)?;
        if c.is_some_and(|c| c.deleted) {
            return Err(self.refuse("replica is deleted"));
        }
        self.opened.set(Some(req.replica_id));
        let own_checkpoint = checkpoint(self.store.store(), own)
            .await
            .map_err(rpc_error)?;
        Ok(PeerOpenResponse {
            store_version: STORE_VERSION,
            replica_id: own,
            checkpoint: Some(own_checkpoint.unwrap_or(Checkpoint {
                id: own,
                version: Some(VersionVector::default()),
                deleted: false,
            })),
        })
    }

    async fn bootstrap(
        &self,
        _: &Context,
        req: PeerBootstrapRequest,
    ) -> Result<PeerBootstrapResponse, RpcError> {
        self.opened().await?;
        check_bootstrap(&req).map_err(|e| self.refuse(e))?;
        let events = req.events.len() as u64;
        let b = Bootstrap {
            events: req.events,
            logs: req.logs,
            checkpoints: req.checkpoints,
        };
        let c = apply_bootstrap(&self.store, b, self.timer.now())
            .await
            .map_err(|e| self.refuse(e))?;
        let mut m = self.metrics.borrow_mut();
        m.bootstraps_received += 1;
        m.events_received += events;
        Ok(PeerBootstrapResponse {
            checkpoint: Some(c),
        })
    }

    async fn sync(&self, _: &Context, req: PeerSyncRequest) -> Result<PeerSyncResponse, RpcError> {
        let from = self.opened().await?;
        check_sync(&req).map_err(|e| self.refuse(e))?;
        let logs = req.logs.len() as u64;
        let c = apply_event_logs(&self.store, from, req.logs, self.timer.now())
            .await
            .map_err(|e| self.refuse(e))?;
        let mut m = self.metrics.borrow_mut();
        m.syncs_received += 1;
        m.logs_received += logs;
        Ok(PeerSyncResponse {
            checkpoint: Some(c),
        })
    }

    async fn allocate_profile_i_ds(
        &self,
        _: &Context,
        _: PeerAllocateProfileIDsRequest,
    ) -> Result<PeerAllocateProfileIDsResponse, RpcError> {
        self.opened().await?;
        let ids = replication::allocate_ids(self.store.store(), MAX_ALLOCATION, self.timer.now())
            .await
            .map_err(rpc_error)?;
        Ok(PeerAllocateProfileIDsResponse { profile_id: ids })
    }
}

/// What this device has sent a peer, and the peer's checkpoint after it.
struct Outbound<'a, K: Kv, T: Timer> {
    store: &'a ReplicatedStore<K>,
    timer: &'a T,
    config: &'a Config,
    metrics: &'a RefCell<Metrics>,
    client: ReplicationPeerClient<RetryClient<T>>,
    remote: u64,
    remote_version: VersionVector,
}

impl<K: Kv, T: Timer> Outbound<'_, K, T> {
    /// Records the peer's checkpoint from an answer.
    async fn accept(&mut self, c: Option<Checkpoint>) -> Result<(), PeerError> {
        let Some(c) = c.filter(|c| c.id == self.remote) else {
            return Ok(());
        };
        if let Some(v) = &c.version {
            vv::upgrade(&mut self.remote_version, v);
        }
        merge_checkpoints(self.store.store(), &[c], self.timer.now()).await?;
        Ok(())
    }

    async fn send_logs(&mut self, logs: Vec<EventLog>) -> Result<(), PeerError> {
        for chunk in replication::delta::chunk_logs(logs, self.config.message_budget) {
            let n = chunk.len() as u64;
            let res = self.client.sync(&PeerSyncRequest { logs: chunk }).await?;
            {
                let mut m = self.metrics.borrow_mut();
                m.syncs_sent += 1;
                m.logs_sent += n;
            }
            self.accept(res.checkpoint).await?;
        }
        Ok(())
    }

    async fn send_delta(&mut self) -> Result<(), PeerError> {
        let logs = compressed_delta(self.store.store(), &self.remote_version).await?;
        self.send_logs(logs).await
    }

    async fn send_bootstrap(&mut self) -> Result<(), PeerError> {
        let b = dump(self.store.store()).await?;
        for part in chunk_bootstrap(b, self.config.message_budget) {
            let req = PeerBootstrapRequest {
                events: part.events,
                logs: part.logs,
                checkpoints: part.checkpoints,
            };
            let n = req.events.len() as u64;
            let res = self.client.bootstrap(&req).await?;
            {
                let mut m = self.metrics.borrow_mut();
                m.bootstraps_sent += 1;
                m.events_sent += n;
            }
            self.accept(res.checkpoint).await?;
        }
        Ok(())
    }
}

/// Replicates this device's store to one peer: opens, tops up this
/// device's ids, catches the peer up, then sends what `queue` holds. Runs
/// until a call or the store fails, or the peer turns out to be deleted.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn replicate_to<K: Kv, T: Timer>(
    store: ReplicatedStore<K>,
    timer: T,
    config: Rc<Config>,
    metrics: Rc<RefCell<Metrics>>,
    client: Client,
    queue: PushQueue,
    replica: Rc<Cell<Option<u64>>>,
) -> Result<(), PeerError> {
    let quick = ReplicationPeerClient::new(RetryClient::new(
        client.clone(),
        timer.clone(),
        config.call_policy,
    ));
    let own = store.replica_id();
    let res = quick
        .open(&PeerOpenRequest {
            store_version: STORE_VERSION,
            replica_id: own,
        })
        .await?;
    metrics.borrow_mut().opens += 1;
    if res.store_version != STORE_VERSION {
        return Err(PeerError::Incompatible(res.store_version));
    }
    let remote = res.replica_id;
    if remote == 0 || remote == own {
        return Err(PeerError::Invalid);
    }
    replica.set(Some(remote));
    if checkpoint(store.store(), remote)
        .await?
        .is_some_and(|c| c.deleted)
    {
        return Err(PeerError::Deleted(remote));
    }

    if free_ids(store.store()).await? <= config.allocate_below {
        let res = quick
            .allocate_profile_i_ds(&PeerAllocateProfileIDsRequest {})
            .await?;
        if let Some(ids) = res.profile_id {
            push_ids(store.store(), ids, timer.now()).await?;
        }
    }

    let mut out = Outbound {
        store: &store,
        timer: &timer,
        config: &config,
        metrics: &metrics,
        client: ReplicationPeerClient::new(RetryClient::new(
            client,
            timer.clone(),
            config.transfer_policy,
        )),
        remote,
        remote_version: VersionVector::default(),
    };
    let first = res.checkpoint.filter(|c| c.id == remote);
    let known = first
        .as_ref()
        .and_then(|c| c.version.as_ref())
        .map_or(0, |v| v.value.len());
    out.accept(first).await?;
    if known > 1 {
        out.send_delta().await?;
    } else {
        out.send_bootstrap().await?;
    }
    loop {
        match queue.next().await {
            Push::Logs(logs) => out.send_logs(logs).await?,
            Push::Resync => out.send_delta().await?,
        }
        // A deleted peer still gets the logs that delete it, then no more.
        if checkpoint(store.store(), remote)
            .await?
            .is_some_and(|c| c.deleted)
        {
            return Err(PeerError::Deleted(remote));
        }
    }
}
