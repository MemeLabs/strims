// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `strims.transfer.v1.TransferPeer`, served to one linked peer.

use std::rc::Weak;

use strims_kademlia::ID;
use strims_proto::service::{Context, Error as RpcError};
use strims_proto::transfer::v1::transfer_peer_announce_response::Body;
use strims_proto::transfer::v1::{
    TransferPeerAnnounceRequest, TransferPeerAnnounceResponse, TransferPeerCloseRequest,
    TransferPeerCloseResponse, TransferPeerService,
};
use strims_rpc::Timer;

use crate::TransferId;
use crate::control::Shared;
use crate::host::Host;

/// Serves one link's requests. Holds the control weakly: the control
/// holds the link's client, whose connection holds this.
pub(crate) struct PeerService<H: Host, T: Timer> {
    shared: Weak<Shared<H, T>>,
    host: ID,
    link: u64,
}

impl<H: Host, T: Timer> PeerService<H, T> {
    pub(crate) fn new(shared: Weak<Shared<H, T>>, host: ID, link: u64) -> Self {
        PeerService { shared, host, link }
    }
}

fn parse_id(b: &[u8]) -> Result<TransferId, RpcError> {
    TransferId::from_bytes(b).ok_or_else(|| RpcError::new(0, "transfer id length mismatch"))
}

fn stopped() -> RpcError {
    RpcError::new(0, "transfers stopped")
}

impl<H: Host, T: Timer> TransferPeerService for PeerService<H, T> {
    async fn announce(
        &self,
        _: &Context,
        req: TransferPeerAnnounceRequest,
    ) -> Result<TransferPeerAnnounceResponse, RpcError> {
        let id = parse_id(&req.id)?;
        let s = self.shared.upgrade().ok_or_else(stopped)?;
        let (mut channel, wait) = s.announce(self.host, self.link, id, req.channel);
        if let Some(wait) = wait
            && !wait.await
        {
            channel = None;
        }
        if channel.is_some() {
            s.metrics_mut().announces_answered += 1;
        }
        Ok(TransferPeerAnnounceResponse {
            body: channel.map(Body::Channel),
        })
    }

    async fn close(
        &self,
        _: &Context,
        req: TransferPeerCloseRequest,
    ) -> Result<TransferPeerCloseResponse, RpcError> {
        let id = parse_id(&req.id)?;
        let s = self.shared.upgrade().ok_or_else(stopped)?;
        if let Some(wait) = s.close(self.host, self.link, id) {
            wait.await;
        }
        Ok(TransferPeerCloseResponse {})
    }
}
