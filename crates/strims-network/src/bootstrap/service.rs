// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `strims.network.v1.bootstrap.PeerService`, served to one linked peer.

use std::rc::{Rc, Weak};

use strims_crypto::{
    certificate_chain, certificate_network_key, membership_trusted, verify_certificate,
};
use strims_dao::Table;
use strims_dao::schema::network::Networks;
use strims_kv::Kv;
use strims_proto::network::v1::Network;
use strims_proto::network::v1::bootstrap::{
    BootstrapPeerGetPublishEnabledRequest, BootstrapPeerGetPublishEnabledResponse,
    BootstrapPeerPublishRequest, BootstrapPeerPublishResponse, PeerServiceService,
};
use strims_proto::service::{Context, Error as RpcError};
use strims_proto::types::Certificate;
use strims_rpc::Timer;

use super::Host;
use super::control::Shared;

fn rpc_error(e: impl std::fmt::Display) -> RpcError {
    RpcError::new(0, e.to_string())
}

/// Serves one link's requests. Holds the control weakly: the control
/// holds the link's client, whose connection holds this.
pub(crate) struct PeerService<K: Kv, H: Host, T: Timer> {
    shared: Weak<Shared<K, H, T>>,
}

impl<K: Kv, H: Host, T: Timer> PeerService<K, H, T> {
    pub(crate) fn new(shared: Weak<Shared<K, H, T>>) -> Self {
        PeerService { shared }
    }

    fn shared(&self) -> Result<Rc<Shared<K, H, T>>, RpcError> {
        self.shared
            .upgrade()
            .ok_or_else(|| rpc_error("bootstrap control stopped"))
    }
}

impl<K: Kv, H: Host, T: Timer> PeerServiceService for PeerService<K, H, T> {
    async fn get_publish_enabled(
        &self,
        _: &Context,
        _: BootstrapPeerGetPublishEnabledRequest,
    ) -> Result<BootstrapPeerGetPublishEnabledResponse, RpcError> {
        let enabled = self
            .shared()?
            .publishing_enabled()
            .await
            .map_err(rpc_error)?;
        Ok(BootstrapPeerGetPublishEnabledResponse { enabled })
    }

    async fn publish(
        &self,
        _: &Context,
        req: BootstrapPeerPublishRequest,
    ) -> Result<BootstrapPeerPublishResponse, RpcError> {
        let s = self.shared()?;
        accept_published(&s, req.certificate).await?;
        Ok(BootstrapPeerPublishResponse {})
    }
}

/// Adds the network of a membership a peer published to this profile.
async fn accept_published<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    certificate: Option<Certificate>,
) -> Result<(), RpcError> {
    if !s.publishing_enabled().await.map_err(rpc_error)? {
        return Err(rpc_error("not supported"));
    }
    let cert = certificate.ok_or_else(|| rpc_error("certificate required"))?;
    if certificate_chain(&cert).len() > s.config.max_chain_depth {
        return Err(rpc_error("certificate chain too long"));
    }
    let now = s.timer.now();
    verify_certificate(&cert, now).map_err(|e| rpc_error(format!("invalid certificate: {e}")))?;
    let profile_key = s.profile.key.as_ref().map(|k| k.public.as_slice());
    if profile_key != Some(cert.key.as_slice()) {
        return Err(rpc_error("certificate isn't for this profile"));
    }
    if !membership_trusted(&cert, now) {
        return Err(rpc_error("certificate isn't signed by its network"));
    }
    let network_key = certificate_network_key(&cert).to_vec();
    let mut tx = s.store.write(now).await.map_err(rpc_error)?;
    if Networks::index_get(&mut tx, &Networks::BY_KEY, &network_key)
        .await
        .map_err(rpc_error)?
        .is_some()
    {
        return Err(rpc_error("network already exists"));
    }
    if Networks::get_all(&mut tx).await.map_err(rpc_error)?.len() >= s.config.max_networks {
        return Err(rpc_error("too many networks"));
    }
    let id = tx.generate_id().await.map_err(rpc_error)?;
    let network = Network {
        id,
        certificate: Some(cert),
        ..Default::default()
    };
    Networks::insert(&mut tx, network)
        .await
        .map_err(rpc_error)?;
    tx.commit().await.map_err(rpc_error)?;
    Ok(())
}
