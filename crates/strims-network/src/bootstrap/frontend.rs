// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's bootstrap calls: `strims.network.v1.bootstrap.BootstrapFrontend`.
//!
//! Bootstrap clients and the bootstrap config are records in the profile's
//! store, so changing them is a store write that the control follows.

use std::rc::Rc;

use strims_dao::schema::network::{BootstrapClients, BootstrapConfig, client_options_key};
use strims_dao::{Singleton, Table};
use strims_kv::Kv;
use strims_proto::network::v1::bootstrap::{
    BootstrapClient, BootstrapClientWebSocketOptions, BootstrapFrontendService,
    CreateBootstrapClientRequest, CreateBootstrapClientResponse, DeleteBootstrapClientRequest,
    DeleteBootstrapClientResponse, GetBootstrapClientRequest, GetBootstrapClientResponse,
    GetConfigRequest, GetConfigResponse, ListBootstrapClientsRequest, ListBootstrapClientsResponse,
    ListBootstrapPeersRequest, ListBootstrapPeersResponse, PublishNetworkToBootstrapPeerRequest,
    PublishNetworkToBootstrapPeerResponse, SetConfigRequest, SetConfigResponse,
    UpdateBootstrapClientRequest, UpdateBootstrapClientResponse, bootstrap_client,
    create_bootstrap_client_request, register_bootstrap_frontend_service,
    update_bootstrap_client_request,
};
use strims_proto::service::{Context, Error as RpcError};
use strims_rpc::{Router, Timer};

use super::Host;
use super::control::BootstrapControl;

fn rpc_error(e: impl std::fmt::Display) -> RpcError {
    RpcError::new(0, e.to_string())
}

fn not_found() -> RpcError {
    rpc_error("bootstrap client not found")
}

/// Serves `strims.network.v1.bootstrap.BootstrapFrontend` for a profile's
/// bootstrap control.
pub struct BootstrapFrontend<K: Kv, H: Host, T: Timer> {
    control: BootstrapControl<K, H, T>,
}

impl<K: Kv, H: Host, T: Timer> BootstrapFrontend<K, H, T> {
    /// The frontend of `control`.
    pub fn new(control: BootstrapControl<K, H, T>) -> Self {
        BootstrapFrontend { control }
    }

    /// WebSocket options with a URL a client may hold.
    fn checked(
        &self,
        o: BootstrapClientWebSocketOptions,
    ) -> Result<BootstrapClientWebSocketOptions, RpcError> {
        if !(o.url.starts_with("ws://") || o.url.starts_with("wss://")) {
            return Err(rpc_error("bootstrap client url must be ws:// or wss://"));
        }
        if o.url.len() > self.control.shared.config.max_url_bytes {
            return Err(rpc_error("bootstrap client url too long"));
        }
        Ok(o)
    }
}

/// Serves the bootstrap frontend of `control` on `router`.
pub fn bind_frontend<K: Kv, H: Host, T: Timer>(
    router: &Router,
    control: &BootstrapControl<K, H, T>,
) {
    let mut router = router.clone();
    register_bootstrap_frontend_service(
        &mut router,
        Rc::new(BootstrapFrontend::new(control.clone())),
    );
}

impl<K: Kv, H: Host, T: Timer> BootstrapFrontendService for BootstrapFrontend<K, H, T> {
    async fn get_config(
        &self,
        _: &Context,
        _: GetConfigRequest,
    ) -> Result<GetConfigResponse, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.read().await.map_err(rpc_error)?;
        let config = BootstrapConfig::get(&mut tx).await.map_err(rpc_error)?;
        Ok(GetConfigResponse {
            config: Some(config.unwrap_or_default()),
        })
    }

    async fn set_config(
        &self,
        _: &Context,
        req: SetConfigRequest,
    ) -> Result<SetConfigResponse, RpcError> {
        let s = &self.control.shared;
        let config = req.config.ok_or_else(|| rpc_error("config required"))?;
        let mut tx = s.store.write(s.timer.now()).await.map_err(rpc_error)?;
        let config = BootstrapConfig::set(&mut tx, config)
            .await
            .map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(SetConfigResponse {
            config: Some(config),
        })
    }

    async fn create_client(
        &self,
        _: &Context,
        req: CreateBootstrapClientRequest,
    ) -> Result<CreateBootstrapClientResponse, RpcError> {
        let s = &self.control.shared;
        let Some(create_bootstrap_client_request::ClientOptions::WebsocketOptions(o)) =
            req.client_options
        else {
            return Err(rpc_error("unexpected client options type"));
        };
        let options = bootstrap_client::ClientOptions::WebsocketOptions(self.checked(o)?);
        let mut tx = s.store.write(s.timer.now()).await.map_err(rpc_error)?;
        let replaces = BootstrapClients::index_get(
            &mut tx,
            &BootstrapClients::BY_CLIENT_OPTIONS,
            &client_options_key(&BootstrapClient {
                client_options: Some(options.clone()),
                ..Default::default()
            }),
        )
        .await
        .map_err(rpc_error)?
        .is_some();
        if !replaces
            && BootstrapClients::get_all(&mut tx)
                .await
                .map_err(rpc_error)?
                .len()
                >= s.config.max_clients
        {
            return Err(rpc_error("too many bootstrap clients"));
        }
        let id = tx.generate_id().await.map_err(rpc_error)?;
        let client = BootstrapClients::insert(
            &mut tx,
            BootstrapClient {
                id,
                version: None,
                client_options: Some(options),
            },
        )
        .await
        .map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(CreateBootstrapClientResponse {
            bootstrap_client: Some(client),
        })
    }

    async fn update_client(
        &self,
        _: &Context,
        req: UpdateBootstrapClientRequest,
    ) -> Result<UpdateBootstrapClientResponse, RpcError> {
        let s = &self.control.shared;
        let Some(update_bootstrap_client_request::ClientOptions::WebsocketOptions(o)) =
            req.client_options
        else {
            return Err(rpc_error("unexpected client options type"));
        };
        let options = bootstrap_client::ClientOptions::WebsocketOptions(self.checked(o)?);
        let mut tx = s.store.write(s.timer.now()).await.map_err(rpc_error)?;
        let mut client = BootstrapClients::get(&mut tx, req.id)
            .await
            .map_err(rpc_error)?
            .ok_or_else(not_found)?;
        client.client_options = Some(options);
        BootstrapClients::update(&mut tx, client.clone())
            .await
            .map_err(rpc_error)?;
        let client = BootstrapClients::get(&mut tx, req.id)
            .await
            .map_err(rpc_error)?
            .unwrap_or(client);
        tx.commit().await.map_err(rpc_error)?;
        Ok(UpdateBootstrapClientResponse {
            bootstrap_client: Some(client),
        })
    }

    async fn delete_client(
        &self,
        _: &Context,
        req: DeleteBootstrapClientRequest,
    ) -> Result<DeleteBootstrapClientResponse, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.write(s.timer.now()).await.map_err(rpc_error)?;
        if !BootstrapClients::delete(&mut tx, req.id)
            .await
            .map_err(rpc_error)?
        {
            return Err(not_found());
        }
        tx.commit().await.map_err(rpc_error)?;
        Ok(DeleteBootstrapClientResponse {})
    }

    async fn get_client(
        &self,
        _: &Context,
        req: GetBootstrapClientRequest,
    ) -> Result<GetBootstrapClientResponse, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.read().await.map_err(rpc_error)?;
        let client = BootstrapClients::get(&mut tx, req.id)
            .await
            .map_err(rpc_error)?
            .ok_or_else(not_found)?;
        Ok(GetBootstrapClientResponse {
            bootstrap_client: Some(client),
        })
    }

    async fn list_clients(
        &self,
        _: &Context,
        _: ListBootstrapClientsRequest,
    ) -> Result<ListBootstrapClientsResponse, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.read().await.map_err(rpc_error)?;
        let bootstrap_clients = BootstrapClients::get_all(&mut tx)
            .await
            .map_err(rpc_error)?;
        Ok(ListBootstrapClientsResponse { bootstrap_clients })
    }

    async fn list_peers(
        &self,
        _: &Context,
        _: ListBootstrapPeersRequest,
    ) -> Result<ListBootstrapPeersResponse, RpcError> {
        Ok(ListBootstrapPeersResponse {
            peers: self.control.list_peers(),
        })
    }

    async fn publish_network_to_peer(
        &self,
        _: &Context,
        req: PublishNetworkToBootstrapPeerRequest,
    ) -> Result<PublishNetworkToBootstrapPeerResponse, RpcError> {
        self.control.publish(req.peer_id, req.network_id).await?;
        Ok(PublishNetworkToBootstrapPeerResponse {})
    }
}
