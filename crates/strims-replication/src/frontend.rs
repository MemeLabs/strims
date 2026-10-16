// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's replication calls.

use strims_dao::auth;
use strims_dao::replication::checkpoints;
use strims_dao::schema::network::{BootstrapClients, Networks};
use strims_dao::schema::profile::{Devices, Profile, ProfileIds};
use strims_dao::{ReplicatedStore, Singleton, Table, ids};
use strims_kv::Kv;
use strims_proto::auth::v1::PairingToken;
use strims_proto::network::v1::Network;
use strims_proto::replication::v1::{
    CreatePairingTokenRequest, CreatePairingTokenResponse, ListCheckpointsRequest,
    ListCheckpointsResponse, ReplicationFrontendService,
};
use strims_proto::service::{Context, Error as RpcError};
use strims_rpc::Timer;

/// The ids a pairing token gives the new device.
pub const PAIRING_IDS: u64 = 1000;

fn rpc_error(e: impl std::fmt::Display) -> RpcError {
    RpcError::new(0, e.to_string())
}

/// Serves `strims.replication.v1.ReplicationFrontend` for a profile.
pub struct ReplicationFrontend<K: Kv, T: Timer> {
    store: ReplicatedStore<K>,
    timer: T,
}

impl<K: Kv, T: Timer> ReplicationFrontend<K, T> {
    /// The frontend of `store`.
    pub fn new(store: ReplicatedStore<K>, timer: T) -> Self {
        ReplicationFrontend { store, timer }
    }

    /// A token that makes a new device of the profile: its sign-in record,
    /// the profile with a new device id, the profile's networks (the one
    /// named, or all) without their server configs, bootstrap clients (the
    /// one named, or all), its devices, and [`PAIRING_IDS`] of this
    /// device's ids.
    pub async fn pairing_token(
        &self,
        network_id: u64,
        bootstrap_id: u64,
    ) -> Result<PairingToken, strims_dao::Error> {
        let store = self.store.store();
        let mut tx = store.write(self.timer.now()).await?;
        let mut profile = Profile::get(&mut tx)
            .await?
            .ok_or(strims_dao::Error::NotFound)?;
        let networks: Vec<Network> = Networks::get_all(&mut tx)
            .await?
            .into_iter()
            .filter(|n| network_id == 0 || n.id == network_id)
            .map(|n| Network {
                server_config: None,
                ..n
            })
            .collect();
        if networks.is_empty() {
            return Err(strims_dao::Error::Invalid("pairing without a network"));
        }
        let bootstraps = BootstrapClients::get_all(&mut tx)
            .await?
            .into_iter()
            .filter(|b| bootstrap_id == 0 || b.id == bootstrap_id)
            .collect();
        let devices = Devices::get_all(&mut tx).await?;
        profile.device_id = tx.generate_id().await?;
        let mut ranges = ProfileIds::get(&mut tx)
            .await?
            .unwrap_or_else(ids::initial_ranges);
        let given = ids::pop(&mut ranges, PAIRING_IDS)?;
        ProfileIds::set(&mut tx, ranges).await?;
        tx.commit().await?;

        let auth = auth::get_user(store.kv(), &profile.name).await?;
        Ok(PairingToken {
            auth,
            profile: Some(profile),
            networks,
            bootstraps,
            devices,
            profile_id: given,
        })
    }
}

impl<K: Kv, T: Timer> ReplicationFrontendService for ReplicationFrontend<K, T> {
    async fn create_pairing_token(
        &self,
        _: &Context,
        req: CreatePairingTokenRequest,
    ) -> Result<CreatePairingTokenResponse, RpcError> {
        let token = self
            .pairing_token(req.network_id, req.bootstrap_id)
            .await
            .map_err(rpc_error)?;
        Ok(CreatePairingTokenResponse { token: Some(token) })
    }

    async fn list_checkpoints(
        &self,
        _: &Context,
        _: ListCheckpointsRequest,
    ) -> Result<ListCheckpointsResponse, RpcError> {
        let checkpoints = checkpoints(self.store.store()).await.map_err(rpc_error)?;
        Ok(ListCheckpointsResponse { checkpoints })
    }
}
