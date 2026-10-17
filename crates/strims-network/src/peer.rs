// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `strims.network.v1.NetworkPeer`, served to one linked peer.

use std::rc::Weak;

use strims_kademlia::ID;
use strims_kv::Kv;
use strims_proto::network::v1::{
    NetworkPeerCloseRequest, NetworkPeerCloseResponse, NetworkPeerNegotiateRequest,
    NetworkPeerNegotiateResponse, NetworkPeerOpenRequest, NetworkPeerOpenResponse,
    NetworkPeerService, NetworkPeerUpdateCertificateRequest, NetworkPeerUpdateCertificateResponse,
};
use strims_proto::service::{Context, Error as RpcError};
use strims_rpc::Timer;

use crate::control::Shared;
use crate::host::Host;
use crate::state::Refusal;

/// Serves one link's requests. Holds the control weakly: the control
/// holds the link's client, whose connection holds this.
pub(crate) struct PeerService<K: Kv, H: Host, T: Timer> {
    shared: Weak<Shared<K, H, T>>,
    pub(crate) host: ID,
    pub(crate) link: u64,
}

impl<K: Kv, H: Host, T: Timer> PeerService<K, H, T> {
    pub(crate) fn new(shared: Weak<Shared<K, H, T>>, host: ID, link: u64) -> Self {
        PeerService { shared, host, link }
    }

    pub(crate) fn shared(&self) -> Result<std::rc::Rc<Shared<K, H, T>>, RpcError> {
        self.shared
            .upgrade()
            .ok_or_else(|| RpcError::new(0, "network control stopped"))
    }
}

pub(crate) fn refused(r: Refusal) -> RpcError {
    RpcError::new(0, r.to_string())
}

impl<K: Kv, H: Host, T: Timer> NetworkPeerService for PeerService<K, H, T> {
    async fn negotiate(
        &self,
        _: &Context,
        req: NetworkPeerNegotiateRequest,
    ) -> Result<NetworkPeerNegotiateResponse, RpcError> {
        let (host, link) = (self.host, self.link);
        self.shared()?
            .update(|st, now| st.handle_negotiate(host, link, req.key_count, now))
            .map_err(refused)?;
        Ok(NetworkPeerNegotiateResponse::default())
    }

    async fn open(
        &self,
        _: &Context,
        req: NetworkPeerOpenRequest,
    ) -> Result<NetworkPeerOpenResponse, RpcError> {
        let (host, link) = (self.host, self.link);
        self.shared()?
            .update(|st, now| st.handle_open(host, link, req.bindings, now))
            .map_err(refused)?;
        Ok(NetworkPeerOpenResponse::default())
    }

    async fn close(
        &self,
        _: &Context,
        req: NetworkPeerCloseRequest,
    ) -> Result<NetworkPeerCloseResponse, RpcError> {
        let (host, link) = (self.host, self.link);
        self.shared()?
            .update(|st, _| st.handle_close(host, link, &req.key))
            .map_err(refused)?;
        Ok(NetworkPeerCloseResponse {})
    }

    async fn update_certificate(
        &self,
        _: &Context,
        req: NetworkPeerUpdateCertificateRequest,
    ) -> Result<NetworkPeerUpdateCertificateResponse, RpcError> {
        let (host, link) = (self.host, self.link);
        self.shared()?
            .update(|st, now| st.handle_update_certificate(host, link, req.certificate, now))
            .map_err(refused)?;
        Ok(NetworkPeerUpdateCertificateResponse {})
    }
}
