// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's network calls: `strims.network.v1.NetworkFrontend`.
//!
//! Networks are records in the profile's store, so creating, joining,
//! renaming and deleting one is a store write that the control follows.
//! The CA's records (peers and alias reservations) are those of the
//! networks whose server config this device holds.

use std::collections::VecDeque;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context as TaskContext, Poll};

use data_encoding::{BASE64, BASE64_NOPAD};
use futures_core::Stream;
use prost::Message;
use strims_base::rng::fork;
use strims_dao::events::Record;
use strims_dao::schema::network::{
    AliasReservations, BootstrapClients, NetworkPeers, NetworkUiConfig, Networks,
};
use strims_dao::{Recv, Singleton, StoreEvent, Subscription, Table, ref_key};
use strims_kv::Kv;
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::network_event::{
    Body, NetworkPeerCountUpdate, NetworkStart, NetworkStop,
};
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateInvitationResponse, CreateNetworkFromInvitationRequest,
    CreateNetworkFromInvitationResponse, CreateServerRequest, CreateServerResponse,
    DeleteNetworkRequest, DeleteNetworkResponse, DeletePeerRequest, DeletePeerResponse,
    GetNetworkRequest, GetNetworkResponse, GetUiConfigRequest, GetUiConfigResponse,
    GrantPeerInvitationRequest, GrantPeerInvitationResponse, Invitation, InvitationV0,
    ListAliasReservationsRequest, ListAliasReservationsResponse, ListNetworksRequest,
    ListNetworksResponse, ListPeersRequest, ListPeersResponse, Network, NetworkEvent as Event,
    NetworkFrontendService, Peer, ResetAliasReservationCooldownRequest,
    ResetAliasReservationCooldownResponse, ResetPeerRenameCooldownRequest,
    ResetPeerRenameCooldownResponse, TogglePeerBanRequest, TogglePeerBanResponse, UiConfig,
    UpdateAliasRequest, UpdateAliasResponse, UpdateDisplayOrderRequest, UpdateDisplayOrderResponse,
    UpdateServerConfigRequest, UpdateServerConfigResponse, WatchNetworksRequest,
    WatchNetworksResponse, register_network_frontend_service,
};
use strims_proto::service::{Context, Error as RpcError, ServerStream};
use strims_rpc::{Router, Timer};

use crate::NetworkEvent;
use crate::ca::{self, CaError};
use crate::control::NetworkControl;
use crate::host::Host;
use crate::membership::{network_from_invitation_v0, new_invitation_v0, new_network};

/// The longest invitation accepted.
const MAX_INVITATION_BYTES: usize = 64 * 1024;

fn rpc_error(e: impl std::fmt::Display) -> RpcError {
    RpcError::new(0, e.to_string())
}

fn not_found(what: &str) -> RpcError {
    RpcError::new(0, format!("{what} not found"))
}

/// Serves `strims.network.v1.NetworkFrontend` for a profile's network
/// control.
pub struct NetworkFrontend<K: Kv, H: Host, T: Timer> {
    control: NetworkControl<K, H, T>,
}

impl<K: Kv, H: Host, T: Timer> NetworkFrontend<K, H, T> {
    /// The frontend of `control`.
    pub fn new(control: NetworkControl<K, H, T>) -> Self {
        NetworkFrontend { control }
    }

    /// Changes the peer `id` of a network this device runs the CA of.
    async fn change_peer(&self, id: u64, f: impl FnOnce(&mut Peer)) -> Result<Peer, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.write(s.timer.now()).await.map_err(rpc_error)?;
        let mut peer = NetworkPeers::get(&mut tx, id)
            .await
            .map_err(rpc_error)?
            .ok_or_else(|| not_found("peer"))?;
        f(&mut peer);
        let peer = NetworkPeers::update(&mut tx, peer)
            .await
            .map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(peer)
    }

    async fn get_network(&self, id: u64) -> Result<Network, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.read().await.map_err(rpc_error)?;
        Networks::get(&mut tx, id)
            .await
            .map_err(rpc_error)?
            .ok_or_else(|| not_found("network"))
    }
}

/// Serves the network frontend of `control` on `router`.
pub fn bind_frontend<K: Kv, H: Host, T: Timer>(router: &Router, control: &NetworkControl<K, H, T>) {
    let mut router = router.clone();
    register_network_frontend_service(&mut router, Rc::new(NetworkFrontend::new(control.clone())));
}

/// The bytes of an invitation, as the UI passes it: base64 with or
/// without padding, or raw.
fn invitation_bytes(field: Option<InvitationField>) -> Result<Vec<u8>, RpcError> {
    let b = match field.ok_or_else(|| rpc_error("invitation has no content"))? {
        InvitationField::InvitationB64(s) => {
            let s = s.trim();
            BASE64_NOPAD
                .decode(s.as_bytes())
                .or_else(|_| BASE64.decode(s.as_bytes()))
                .map_err(|_| rpc_error("invitation isn't base64"))?
        }
        InvitationField::InvitationBytes(b) => b,
    };
    if b.len() > MAX_INVITATION_BYTES {
        return Err(rpc_error("invitation too large"));
    }
    Ok(b)
}

impl<K: Kv, H: Host, T: Timer> NetworkFrontendService for NetworkFrontend<K, H, T> {
    async fn create_server(
        &self,
        _: &Context,
        req: CreateServerRequest,
    ) -> Result<CreateServerResponse, RpcError> {
        let s = &self.control.shared;
        let now = s.timer.now();
        let mut rng = fork(&mut s.rng.borrow_mut());
        let mut tx = s.store.write(now).await.map_err(rpc_error)?;
        let id = tx.generate_id().await.map_err(rpc_error)?;
        let network = new_network(
            id, &req.name, req.icon, &s.profile, &req.alias, now, &mut rng,
        )
        .map_err(rpc_error)?;
        let network = Networks::insert(&mut tx, network)
            .await
            .map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(CreateServerResponse {
            network: Some(network),
        })
    }

    async fn update_server_config(
        &self,
        _: &Context,
        req: UpdateServerConfigRequest,
    ) -> Result<UpdateServerConfigResponse, RpcError> {
        let s = &self.control.shared;
        let config = req
            .server_config
            .ok_or_else(|| rpc_error("server config missing"))?;
        let mut tx = s.store.write(s.timer.now()).await.map_err(rpc_error)?;
        let mut network = Networks::get(&mut tx, req.network_id)
            .await
            .map_err(rpc_error)?
            .ok_or_else(|| not_found("network"))?;
        let prev = network
            .server_config
            .as_ref()
            .ok_or_else(|| rpc_error("network has no server config"))?;
        if prev.key != config.key {
            return Err(rpc_error("a network's key can't change"));
        }
        network.server_config = Some(config);
        let network = Networks::update(&mut tx, network)
            .await
            .map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(UpdateServerConfigResponse {
            network: Some(network),
        })
    }

    async fn delete(
        &self,
        _: &Context,
        req: DeleteNetworkRequest,
    ) -> Result<DeleteNetworkResponse, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.write(s.timer.now()).await.map_err(rpc_error)?;
        if !Networks::delete(&mut tx, req.id).await.map_err(rpc_error)? {
            return Err(not_found("network"));
        }
        tx.commit().await.map_err(rpc_error)?;
        Ok(DeleteNetworkResponse {})
    }

    async fn get(
        &self,
        _: &Context,
        req: GetNetworkRequest,
    ) -> Result<GetNetworkResponse, RpcError> {
        Ok(GetNetworkResponse {
            network: Some(self.get_network(req.id).await?),
        })
    }

    async fn list(
        &self,
        _: &Context,
        _: ListNetworksRequest,
    ) -> Result<ListNetworksResponse, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.read().await.map_err(rpc_error)?;
        let networks = Networks::get_all(&mut tx).await.map_err(rpc_error)?;
        Ok(ListNetworksResponse { networks })
    }

    async fn create_invitation(
        &self,
        _: &Context,
        req: CreateInvitationRequest,
    ) -> Result<CreateInvitationResponse, RpcError> {
        let s = &self.control.shared;
        let network = self.get_network(req.network_id).await?;
        let membership = network
            .certificate
            .ok_or_else(|| rpc_error("network has no certificate"))?;
        let mut bootstrap_clients = Vec::new();
        if req.bootstrap_client_id != 0 {
            let mut tx = s.store.read().await.map_err(rpc_error)?;
            let mut c = BootstrapClients::get(&mut tx, req.bootstrap_client_id)
                .await
                .map_err(rpc_error)?
                .ok_or_else(|| not_found("bootstrap client"))?;
            c.id = 0;
            bootstrap_clients.push(c);
        }
        let mut rng = fork(&mut s.rng.borrow_mut());
        let invitation = InvitationV0 {
            bootstrap_clients,
            ..new_invitation_v0(&s.profile, &membership, s.timer.now(), &mut rng)
                .map_err(rpc_error)?
        };
        Ok(CreateInvitationResponse {
            invitation: Some(Invitation {
                version: 0,
                data: invitation.encode_to_vec(),
            }),
        })
    }

    async fn create_network_from_invitation(
        &self,
        _: &Context,
        req: CreateNetworkFromInvitationRequest,
    ) -> Result<CreateNetworkFromInvitationResponse, RpcError> {
        let s = &self.control.shared;
        let wrapper =
            Invitation::decode(invitation_bytes(req.invitation)?.as_slice()).map_err(rpc_error)?;
        if wrapper.version != 0 {
            return Err(rpc_error("unsupported invitation version"));
        }
        let invitation = InvitationV0::decode(wrapper.data.as_slice()).map_err(rpc_error)?;
        let now = s.timer.now();
        let mut rng = fork(&mut s.rng.borrow_mut());
        let mut tx = s.store.write(now).await.map_err(rpc_error)?;
        let id = tx.generate_id().await.map_err(rpc_error)?;
        let network =
            network_from_invitation_v0(id, &invitation, &s.profile, &req.alias, now, &mut rng)
                .map_err(rpc_error)?;
        let network = Networks::insert(&mut tx, network)
            .await
            .map_err(rpc_error)?;
        for mut c in invitation.bootstrap_clients {
            c.id = tx.generate_id().await.map_err(rpc_error)?;
            BootstrapClients::insert(&mut tx, c)
                .await
                .map_err(rpc_error)?;
        }
        tx.commit().await.map_err(rpc_error)?;
        Ok(CreateNetworkFromInvitationResponse {
            network: Some(network),
        })
    }

    async fn watch(
        &self,
        _: &Context,
        _: WatchNetworksRequest,
    ) -> Result<ServerStream<WatchNetworksResponse>, RpcError> {
        let mut w = Watch {
            control: self.control.clone(),
            events: self.control.subscribe(),
            store: self.control.shared.store.events().subscribe(),
            pending: VecDeque::new(),
        };
        w.snapshot();
        Ok(Box::pin(w))
    }

    async fn update_display_order(
        &self,
        _: &Context,
        req: UpdateDisplayOrderRequest,
    ) -> Result<UpdateDisplayOrderResponse, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.write(s.timer.now()).await.map_err(rpc_error)?;
        NetworkUiConfig::set(
            &mut tx,
            UiConfig {
                network_display_order: req.network_ids,
            },
        )
        .await
        .map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(UpdateDisplayOrderResponse {})
    }

    /// Renaming renews the certificate under the new alias.
    async fn update_alias(
        &self,
        _: &Context,
        req: UpdateAliasRequest,
    ) -> Result<UpdateAliasResponse, RpcError> {
        let s = &self.control.shared;
        if req.alias.len() > ca::MAX_SUBJECT_LEN {
            return Err(rpc_error("alias too long"));
        }
        let mut tx = s.store.write(s.timer.now()).await.map_err(rpc_error)?;
        let mut network = Networks::get(&mut tx, req.id)
            .await
            .map_err(rpc_error)?
            .ok_or_else(|| not_found("network"))?;
        network.alias = req.alias;
        let network = Networks::update(&mut tx, network)
            .await
            .map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(UpdateAliasResponse {
            network: Some(network),
        })
    }

    async fn get_ui_config(
        &self,
        _: &Context,
        _: GetUiConfigRequest,
    ) -> Result<GetUiConfigResponse, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.read().await.map_err(rpc_error)?;
        let config = NetworkUiConfig::get(&mut tx).await.map_err(rpc_error)?;
        Ok(GetUiConfigResponse { config })
    }

    async fn list_peers(
        &self,
        _: &Context,
        req: ListPeersRequest,
    ) -> Result<ListPeersResponse, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.read().await.map_err(rpc_error)?;
        let peers = NetworkPeers::index_get_all(
            &mut tx,
            &NetworkPeers::BY_NETWORK,
            &ref_key(req.network_id),
        )
        .await
        .map_err(rpc_error)?;
        Ok(ListPeersResponse { peers })
    }

    async fn grant_peer_invitation(
        &self,
        _: &Context,
        req: GrantPeerInvitationRequest,
    ) -> Result<GrantPeerInvitationResponse, RpcError> {
        let peer = self
            .change_peer(req.id, |p| {
                p.invite_quota = p.invite_quota.saturating_add(req.count);
            })
            .await?;
        Ok(GrantPeerInvitationResponse { peer: Some(peer) })
    }

    async fn toggle_peer_ban(
        &self,
        _: &Context,
        req: TogglePeerBanRequest,
    ) -> Result<TogglePeerBanResponse, RpcError> {
        let peer = self
            .change_peer(req.id, |p| p.is_banned = req.value)
            .await?;
        Ok(TogglePeerBanResponse { peer: Some(peer) })
    }

    async fn reset_peer_rename_cooldown(
        &self,
        _: &Context,
        req: ResetPeerRenameCooldownRequest,
    ) -> Result<ResetPeerRenameCooldownResponse, RpcError> {
        let peer = self.change_peer(req.id, |p| p.alias_changed_at = 0).await?;
        Ok(ResetPeerRenameCooldownResponse { peer: Some(peer) })
    }

    /// Deleting a peer gives up its alias, which stays reserved for its
    /// cooldown.
    async fn delete_peer(
        &self,
        _: &Context,
        req: DeletePeerRequest,
    ) -> Result<DeletePeerResponse, RpcError> {
        let s = &self.control.shared;
        let now = s.timer.now();
        let mut tx = s.store.write(now).await.map_err(rpc_error)?;
        let peer = NetworkPeers::get(&mut tx, req.id)
            .await
            .map_err(rpc_error)?
            .ok_or_else(|| not_found("peer"))?;
        NetworkPeers::delete(&mut tx, req.id)
            .await
            .map_err(rpc_error)?;
        ca::release_alias(&mut tx, peer.network_id, &peer.alias, now)
            .await
            .map_err(|e: CaError| e.to_rpc())?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(DeletePeerResponse {})
    }

    async fn list_alias_reservations(
        &self,
        _: &Context,
        req: ListAliasReservationsRequest,
    ) -> Result<ListAliasReservationsResponse, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.read().await.map_err(rpc_error)?;
        let alias_reservations = AliasReservations::index_get_all(
            &mut tx,
            &AliasReservations::BY_NETWORK,
            &ref_key(req.network_id),
        )
        .await
        .map_err(rpc_error)?;
        Ok(ListAliasReservationsResponse { alias_reservations })
    }

    async fn reset_alias_reservation_cooldown(
        &self,
        _: &Context,
        req: ResetAliasReservationCooldownRequest,
    ) -> Result<ResetAliasReservationCooldownResponse, RpcError> {
        let s = &self.control.shared;
        let mut tx = s.store.write(s.timer.now()).await.map_err(rpc_error)?;
        let mut r = AliasReservations::get(&mut tx, req.id)
            .await
            .map_err(rpc_error)?
            .ok_or_else(|| not_found("alias reservation"))?;
        if !r.peer_key.is_empty() {
            return Err(rpc_error("cannot reset the cooldown of an alias in use"));
        }
        r.reserved_until = 0;
        AliasReservations::update(&mut tx, r)
            .await
            .map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(ResetAliasReservationCooldownResponse {})
    }
}

/// A `Watch` call's stream: the networks the node runs, then their
/// changes and the UI config's. A subscriber that falls behind gets the
/// networks again.
struct Watch<K: Kv, H: Host, T: Timer> {
    control: NetworkControl<K, H, T>,
    events: Subscription<NetworkEvent>,
    store: Subscription,
    /// Bound: the networks, plus one event.
    pending: VecDeque<WatchNetworksResponse>,
}

fn response(body: Body) -> WatchNetworksResponse {
    WatchNetworksResponse {
        event: Some(Event { body: Some(body) }),
    }
}

impl<K: Kv, H: Host, T: Timer> Watch<K, H, T> {
    fn snapshot(&mut self) {
        for n in self.control.networks() {
            self.pending
                .push_back(response(Body::NetworkStart(NetworkStart {
                    network: Some(n.network),
                    peer_count: u32::try_from(n.peer_count).unwrap_or(u32::MAX),
                })));
        }
    }

    /// Queues the UI's event for `e`, if it has one: the UI's protocol has
    /// none for `Joined`.
    fn push(&mut self, e: &NetworkEvent) {
        let body = match e {
            NetworkEvent::Started { network } => Body::NetworkStart(NetworkStart {
                network: Some(network.clone()),
                peer_count: 0,
            }),
            NetworkEvent::Joined { .. } => return,
            NetworkEvent::Stopped { network } => Body::NetworkStop(NetworkStop {
                network_id: network.id,
            }),
            NetworkEvent::Updated { network } => Body::NetworkUpdate(network.clone()),
            NetworkEvent::PeerCount { network_id, count } => {
                Body::NetworkPeerCountUpdate(NetworkPeerCountUpdate {
                    network_id: *network_id,
                    peer_count: u32::try_from(*count).unwrap_or(u32::MAX),
                })
            }
        };
        self.pending.push_back(response(body));
    }
}

impl<K: Kv, H: Host, T: Timer> Stream for Watch<K, H, T> {
    type Item = Result<WatchNetworksResponse, RpcError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        loop {
            if let Some(r) = this.pending.pop_front() {
                return Poll::Ready(Some(Ok(r)));
            }
            match Pin::new(&mut this.events).poll_next(cx) {
                Poll::Ready(Some(Recv::Event(e))) => {
                    this.push(&e);
                    continue;
                }
                Poll::Ready(Some(Recv::Lagged)) => {
                    this.snapshot();
                    continue;
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => {}
            }
            match Pin::new(&mut this.store).poll_next(cx) {
                Poll::Ready(Some(Recv::Event(e))) => {
                    if let StoreEvent::Changed(Record::NetworkUiConfig(c)) = &*e {
                        this.pending
                            .push_back(response(Body::UiConfigUpdate(c.clone())));
                    }
                    continue;
                }
                Poll::Ready(Some(Recv::Lagged)) => continue,
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
