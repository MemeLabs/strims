// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's chat calls: `strims.chat.v1.ChatServerFrontend` and
//! `strims.chat.v1.ChatFrontend`, served by [`ChatFrontend`].
//!
//! # Chat servers
//!
//! The profile's chat servers, with their icons, emotes, modifiers and
//! tags, are records in its store, which the [`ChatControl`] follows: a
//! change to a server's assets reaches its readers a debounce later. A
//! server is created on one of the profile's networks with a new ed25519
//! key. Deleting a server deletes its records.
//!
//! # Rooms and whispers
//!
//! `OpenClient` opens a client stream of a room ([`ChatControl::open_client`])
//! and answers `Open`, then the stream's asset bundles and events as it
//! yields them, and ends when the stream does. A bundle larger than a
//! response is cut with [`split`](crate::assets::split): its first part
//! keeps the bundle's `is_delta`, and the rest are deltas. Each message
//! and each of its nick mentions carries the listing its sender, or the
//! user mentioned, views: the first media or embed listing a network's
//! directory has them in, from
//! [`DirectoryControl::listings_by_peer_key`](strims_directory::DirectoryControl::listings_by_peer_key).
//! What the room's server sent in its place is dropped.
//!
//! Users are named by peer key, or by alias through their network's CA.
//! Sent whispers go through the [`WhisperControl`].
//!
//! # Watches
//!
//! `WatchWhispers` and `WatchUIConfig` subscribe to the store's events
//! before they read the store, so what they send converges on it. They
//! send every record they follow first: the whisper threads with their
//! unread counts, or the UI config, highlights, tags and ignores. Then
//! they send the records' changes and deletions. A watch that falls behind
//! the store's events reads those records again and sends them, with a
//! deletion for each record it sent that is gone. `WatchWhispers` sends a
//! whisper only as it changes, so one that changed while the watch was
//! behind is read with `ListWhispers`. A watch runs until the UI cancels
//! the call, or ends with an error when the store fails.
//!
//! # Errors
//!
//! A refused call answers with a readable message: a chat server's or the
//! control's refusal ([`ChatFailure::to_rpc`](crate::ChatFailure::to_rpc)),
//! the store's error, "*what* not found" for a missing record, or the
//! bound a request passes.
//!
//! # Bounds
//!
//! - A stored name, label, tag color or room name is at most
//!   [`MAX_NAME_BYTES`], an emote has at most [`MAX_LABELS`] labels, and
//!   a stored record encodes to at most [`MAX_RECORD_BYTES`].
//! - A server holds at most as many emotes, modifiers and tags as its
//!   readers take ([`RoomLimits`](crate::reader::RoomLimits)).
//! - A response encodes to at most the frontend's response size
//!   ([`MAX_RESPONSE_BYTES`] by default). Past it, `ListWhispers` leaves
//!   out a thread's oldest whispers, and a client stream leaves out the
//!   viewed listings of an event, or an event or bundle entry that doesn't
//!   fit a response alone.

mod chat;
mod open;
mod server;
mod watch;

use std::rc::Rc;

use strims_base::time::Time;
use strims_crypto::certificate_body;
use strims_dao::{Index, ReplicatedStore, Table, WriteTx};
use strims_directory::{Host, MAX_ALIAS_BYTES};
use strims_kv::Kv;
use strims_proto::chat::v1::{
    register_chat_frontend_service, register_chat_server_frontend_service,
};
use strims_proto::network::v1::ca::ca_find_request::Query;
use strims_proto::service::Error as RpcError;
use strims_proto::types::CertificateBody;
use strims_rpc::{Router, Timer};

use crate::{ChatControl, WhisperControl};

/// The largest response the frontend sends by default: what
/// [`strims_rpc::Config::frontend`] sends, less a call's envelope.
pub const MAX_RESPONSE_BYTES: usize = 10 * 1024 * 1024 - ENVELOPE_BYTES;

/// What a response costs in its call beyond its encoding: the call's ids,
/// method, kind and the argument's tag and length, rounded up.
const ENVELOPE_BYTES: usize = 1024;

/// What an item costs in a message beyond its encoding: its tag and length
/// prefix, rounded up.
const ITEM_OVERHEAD: usize = 16;

/// The longest name of an emote, modifier or tag, emote label, tag color,
/// or room name, in bytes.
pub const MAX_NAME_BYTES: usize = 256;

/// The labels of one emote.
pub const MAX_LABELS: usize = 64;

/// The largest encoded server, server icon, emote, modifier, tag or UI
/// config the frontend stores. At the default response size, a record
/// this size fits a part of a split bundle with room to spare.
pub const MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;

const _: () = assert!(MAX_RECORD_BYTES + ITEM_OVERHEAD < MAX_RESPONSE_BYTES / 2);

fn rpc_error(e: impl std::fmt::Display) -> RpcError {
    RpcError::new(0, e.to_string())
}

fn not_found(what: &str) -> RpcError {
    RpcError::new(0, format!("{what} not found"))
}

/// Serves the UI's chat calls for a node's chat and whisper controls; see
/// the [module docs](self).
pub struct ChatFrontend<K: Kv, H: Host, T: Timer> {
    chat: ChatControl<K, H, T>,
    whispers: WhisperControl<K, H, T>,
    max_response_bytes: usize,
}

impl<K: Kv, H: Host, T: Timer> ChatFrontend<K, H, T> {
    /// The frontend of `chat` and `whispers`, whose responses encode to at
    /// most [`MAX_RESPONSE_BYTES`].
    pub fn new(chat: ChatControl<K, H, T>, whispers: WhisperControl<K, H, T>) -> Self {
        ChatFrontend {
            chat,
            whispers,
            max_response_bytes: MAX_RESPONSE_BYTES,
        }
    }

    /// The frontend with responses of at most `max` encoded bytes, for a
    /// connection that sends at most `max` plus a call's envelope (1 KiB).
    pub fn with_max_response_bytes(mut self, max: usize) -> Self {
        self.max_response_bytes = max;
        self
    }

    /// Serves both services on `router`.
    pub fn bind(self, router: &Router) {
        let mut router = router.clone();
        let frontend = Rc::new(self);
        register_chat_server_frontend_service(&mut router, frontend.clone());
        register_chat_frontend_service(&mut router, frontend);
    }

    fn now(&self) -> Time {
        self.chat.shared.timer.now()
    }

    fn store(&self) -> &ReplicatedStore<K> {
        self.chat.store()
    }

    async fn write(&self) -> Result<WriteTx<K>, RpcError> {
        self.store().write(self.now()).await.map_err(rpc_error)
    }

    /// The body of the latest certificate the CA of the network with
    /// `network_key` issued for `query`.
    async fn peer_certificate(
        &self,
        network_key: &[u8],
        query: Query,
    ) -> Result<CertificateBody, RpcError> {
        if let Query::Subject(alias) = &query
            && alias.len() > MAX_ALIAS_BYTES
        {
            return Err(rpc_error("alias too long"));
        }
        let cert = self
            .chat
            .network()
            .find_certificate(network_key, query, false)
            .await
            .map_err(|e| RpcError::new(e.code, format!("finding peer cert failed: {e}")))?;
        certificate_body(&cert).map_err(rpc_error)
    }

    /// `peer_key`, or when it's empty the key of the holder of `alias` on
    /// the network with `network_key`.
    async fn peer_key(
        &self,
        network_key: &[u8],
        peer_key: Vec<u8>,
        alias: String,
    ) -> Result<Vec<u8>, RpcError> {
        if !peer_key.is_empty() {
            return Ok(peer_key);
        }
        let cert = self
            .peer_certificate(network_key, Query::Subject(alias))
            .await?;
        Ok(cert.key)
    }
}

/// Serves the chat frontends of `chat` and `whispers` on `router`; see
/// [`ChatFrontend`].
pub fn bind_frontend<K: Kv, H: Host, T: Timer>(
    router: &Router,
    chat: &ChatControl<K, H, T>,
    whispers: &WhisperControl<K, H, T>,
) {
    ChatFrontend::new(chat.clone(), whispers.clone()).bind(router);
}

/// Deletes the records of `T` that `ix` maps `key` to, in `tx`. Fails
/// with "`what` not found" when there are none.
async fn delete_indexed<T: Table, K: Kv>(
    tx: &mut WriteTx<K>,
    ix: &Index<T::Record>,
    key: &[u8],
    what: &str,
) -> Result<(), RpcError> {
    let ids = T::index_ids(tx, ix, key).await.map_err(rpc_error)?;
    if ids.is_empty() {
        return Err(not_found(what));
    }
    for id in ids {
        T::delete(tx, id).await.map_err(rpc_error)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_response_and_its_envelope_fit_the_frontend_connection() {
        let max = strims_rpc::Config::frontend().max_send_bytes;
        assert_eq!(MAX_RESPONSE_BYTES + ENVELOPE_BYTES, max);
    }
}
