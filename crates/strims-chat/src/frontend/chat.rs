// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `ChatFrontend`: rooms, whispers and the chat UI's settings.

use prost::Message;
use strims_base::time::Duration;
use strims_dao::schema::chat::{
    ChatUiConfig, ChatUiConfigHighlights, ChatUiConfigIgnores, ChatUiConfigTags, WhisperRecords,
    WhisperThreads,
};
use strims_dao::{Index, Singleton, Table};
use strims_directory::Host;
use strims_kv::Kv;
use strims_proto::chat::v1::{
    ChatFrontendService, ClientGetMuteRequest, ClientGetMuteResponse, ClientMuteRequest,
    ClientMuteResponse, ClientSendMessageRequest, ClientSendMessageResponse, ClientUnmuteRequest,
    ClientUnmuteResponse, DeleteWhisperThreadRequest, DeleteWhisperThreadResponse,
    HighlightRequest, HighlightResponse, IgnoreRequest, IgnoreResponse, ListWhispersRequest,
    ListWhispersResponse, MarkWhispersReadRequest, MarkWhispersReadResponse, OpenClientRequest,
    OpenClientResponse, SetUiConfigRequest, SetUiConfigResponse, TagRequest, TagResponse,
    UiConfigHighlight, UiConfigIgnore, UiConfigTag, UnhighlightRequest, UnhighlightResponse,
    UnignoreRequest, UnignoreResponse, UntagRequest, UntagResponse, WatchUiConfigRequest,
    WatchUiConfigResponse, WatchWhispersRequest, WatchWhispersResponse, WhisperRequest,
    WhisperResponse,
};
use strims_proto::network::v1::ca::ca_find_request::Query;
use strims_proto::service::{Context, Error as RpcError, ServerStream};
use strims_rpc::Timer;

use super::open::OpenClient;
use super::{
    ChatFrontend, ITEM_OVERHEAD, MAX_NAME_BYTES, MAX_RECORD_BYTES, delete_indexed, not_found,
    rpc_error, watch,
};
use crate::duration;

/// The duration a mute or ignore is given as.
fn parse_duration(s: &str) -> Result<Duration, RpcError> {
    let d = duration::parse(s).map_err(|e| rpc_error(format!("parsing duration failed: {e}")))?;
    if d.is_negative() {
        return Err(rpc_error("duration must not be negative"));
    }
    Ok(d)
}

impl<K: Kv, H: Host, T: Timer> ChatFrontend<K, H, T> {
    /// Stores the record `make` makes with the id of the record of `R`
    /// that `ix` maps `peer_key` to, or with a new id when there is none.
    async fn put_for_peer<R: Table>(
        &self,
        ix: &Index<R::Record>,
        peer_key: &[u8],
        make: impl FnOnce(u64) -> R::Record,
    ) -> Result<(), RpcError> {
        let mut tx = self.write().await?;
        let existing = R::index_ids(&mut tx, ix, peer_key)
            .await
            .map_err(rpc_error)?
            .first()
            .copied();
        let id = match existing {
            Some(id) => id,
            None => tx.generate_id().await.map_err(rpc_error)?,
        };
        R::upsert(&mut tx, make(id)).await.map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)
    }

    /// Deletes the record of `R` that `ix` maps `peer_key` to.
    async fn delete_for_peer<R: Table>(
        &self,
        ix: &Index<R::Record>,
        peer_key: &[u8],
        what: &str,
    ) -> Result<(), RpcError> {
        let mut tx = self.write().await?;
        delete_indexed::<R, K>(&mut tx, ix, peer_key, what).await?;
        tx.commit().await.map_err(rpc_error)
    }
}

impl<K: Kv, H: Host, T: Timer> ChatFrontendService for ChatFrontend<K, H, T> {
    /// A client stream of the room; see the [module docs](super).
    async fn open_client(
        &self,
        _: &Context,
        req: OpenClientRequest,
    ) -> Result<ServerStream<OpenClientResponse>, RpcError> {
        let stream = self
            .chat
            .open_client(&req.network_key, &req.server_key)
            .map_err(|e| e.to_rpc())?;
        let directory = self.chat.directory().clone();
        Ok(Box::pin(OpenClient::new(
            stream,
            directory,
            self.max_response_bytes,
        )))
    }

    async fn client_send_message(
        &self,
        _: &Context,
        req: ClientSendMessageRequest,
    ) -> Result<ClientSendMessageResponse, RpcError> {
        self.chat
            .send_message(&req.network_key, &req.server_key, &req.body)
            .await
            .map_err(|e| e.to_rpc())?;
        Ok(ClientSendMessageResponse {})
    }

    /// Mutes the holder of the alias for the duration, in whole seconds of
    /// at most `u32::MAX`.
    async fn client_mute(
        &self,
        _: &Context,
        req: ClientMuteRequest,
    ) -> Result<ClientMuteResponse, RpcError> {
        let d = parse_duration(&req.duration)?;
        let secs = u32::try_from(d.as_secs()).unwrap_or(u32::MAX);
        let cert = self
            .peer_certificate(&req.network_key, Query::Subject(req.alias))
            .await?;
        self.chat
            .mute(
                &req.network_key,
                &req.server_key,
                &cert.key,
                secs,
                &req.message,
            )
            .await
            .map_err(|e| e.to_rpc())?;
        Ok(ClientMuteResponse {})
    }

    async fn client_unmute(
        &self,
        _: &Context,
        req: ClientUnmuteRequest,
    ) -> Result<ClientUnmuteResponse, RpcError> {
        let cert = self
            .peer_certificate(&req.network_key, Query::Subject(req.alias))
            .await?;
        self.chat
            .unmute(&req.network_key, &req.server_key, &cert.key)
            .await
            .map_err(|e| e.to_rpc())?;
        Ok(ClientUnmuteResponse {})
    }

    /// The profile's own mute on the server.
    async fn client_get_mute(
        &self,
        _: &Context,
        req: ClientGetMuteRequest,
    ) -> Result<ClientGetMuteResponse, RpcError> {
        let m = self
            .chat
            .get_mute(&req.network_key, &req.server_key)
            .await
            .map_err(|e| e.to_rpc())?;
        Ok(ClientGetMuteResponse {
            end_time: m.end_time,
            message: m.message,
        })
    }

    /// Sends a whisper on the network to the holder of the peer key, or of
    /// the alias when the request has no key.
    async fn whisper(&self, _: &Context, req: WhisperRequest) -> Result<WhisperResponse, RpcError> {
        let query = if !req.peer_key.is_empty() {
            Query::Key(req.peer_key)
        } else if !req.alias.is_empty() {
            Query::Subject(req.alias)
        } else {
            return Err(rpc_error("whisper subject undefined"));
        };
        let cert = self.peer_certificate(&req.network_key, query).await?;
        self.whispers
            .enqueue(
                &req.network_key,
                &req.server_key,
                &cert.key,
                &cert.subject,
                &req.body,
            )
            .await
            .map_err(rpc_error)?;
        Ok(WhisperResponse {})
    }

    /// The thread with the peer and its whispers, oldest first. The oldest
    /// are left out when they don't all fit a response.
    async fn list_whispers(
        &self,
        _: &Context,
        req: ListWhispersRequest,
    ) -> Result<ListWhispersResponse, RpcError> {
        let mut tx = self.whispers.shared.store.read().await.map_err(rpc_error)?;
        let thread =
            WhisperThreads::index_get(&mut tx, &WhisperThreads::BY_PEER_KEY, &req.peer_key)
                .await
                .map_err(rpc_error)?
                .ok_or_else(|| not_found("whisper thread"))?;
        let ix = &WhisperRecords::BY_PEER_KEY;
        let mut whispers = WhisperRecords::index_get_all(&mut tx, ix, &req.peer_key)
            .await
            .map_err(rpc_error)?;
        whispers.sort_by_key(|r| (r.message.as_ref().map_or(0, |m| m.server_time), r.id));
        let mut budget = self
            .max_response_bytes
            .saturating_sub(thread.encoded_len().saturating_add(ITEM_OVERHEAD));
        let mut kept = 0;
        for r in whispers.iter().rev() {
            let len = r.encoded_len().saturating_add(ITEM_OVERHEAD);
            if len > budget {
                break;
            }
            budget -= len;
            kept += 1;
        }
        let whispers = whispers.split_off(whispers.len() - kept);
        Ok(ListWhispersResponse {
            thread: Some(thread),
            whispers,
        })
    }

    /// The whisper threads with their unread counts, then the changes and
    /// deletions of threads and whispers; see the [module docs](super).
    async fn watch_whispers(
        &self,
        _: &Context,
        _: WatchWhispersRequest,
    ) -> Result<ServerStream<WatchWhispersResponse>, RpcError> {
        let store = self.whispers.shared.store.clone();
        let events = store.events().subscribe();
        Ok(watch::whispers(store, events))
    }

    /// Marks the peer's whispers read.
    async fn mark_whispers_read(
        &self,
        _: &Context,
        req: MarkWhispersReadRequest,
    ) -> Result<MarkWhispersReadResponse, RpcError> {
        self.whispers
            .mark_read(&req.peer_key)
            .await
            .map_err(rpc_error)?;
        Ok(MarkWhispersReadResponse {})
    }

    /// Deletes the thread with the peer and its whispers.
    async fn delete_whisper_thread(
        &self,
        _: &Context,
        req: DeleteWhisperThreadRequest,
    ) -> Result<DeleteWhisperThreadResponse, RpcError> {
        let s = &self.whispers.shared;
        let mut tx = s.store.write(s.timer.now()).await.map_err(rpc_error)?;
        let ix = &WhisperThreads::BY_PEER_KEY;
        delete_indexed::<WhisperThreads, K>(&mut tx, ix, &req.peer_key, "whisper thread").await?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(DeleteWhisperThreadResponse {})
    }

    async fn set_ui_config(
        &self,
        _: &Context,
        req: SetUiConfigRequest,
    ) -> Result<SetUiConfigResponse, RpcError> {
        let config = req.ui_config.unwrap_or_default();
        if config.encoded_len() > MAX_RECORD_BYTES {
            return Err(rpc_error("ui config too large"));
        }
        let mut tx = self.write().await?;
        ChatUiConfig::set(&mut tx, config)
            .await
            .map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(SetUiConfigResponse {})
    }

    /// The UI config, highlights, tags and ignores, then their changes and
    /// deletions; see the [module docs](super).
    async fn watch_ui_config(
        &self,
        _: &Context,
        _: WatchUiConfigRequest,
    ) -> Result<ServerStream<WatchUiConfigResponse>, RpcError> {
        let store = self.store().clone();
        let events = store.events().subscribe();
        Ok(watch::ui_config(store, events))
    }

    /// Ignores the holder of the alias, for the duration if there is one
    /// and otherwise until unignored. It replaces the peer's ignore.
    async fn ignore(&self, _: &Context, req: IgnoreRequest) -> Result<IgnoreResponse, RpcError> {
        let deadline = if req.duration.is_empty() {
            0
        } else {
            (self.now() + parse_duration(&req.duration)?).unix()
        };
        let cert = self
            .peer_certificate(&req.network_key, Query::Subject(req.alias))
            .await?;
        let ix = &ChatUiConfigIgnores::BY_PEER_KEY;
        self.put_for_peer::<ChatUiConfigIgnores>(ix, &cert.key.clone(), |id| UiConfigIgnore {
            id,
            version: None,
            alias: cert.subject,
            peer_key: cert.key,
            deadline,
        })
        .await?;
        Ok(IgnoreResponse {})
    }

    /// Unignores the holder of the peer key, or of the alias when the
    /// request has no key.
    async fn unignore(
        &self,
        _: &Context,
        req: UnignoreRequest,
    ) -> Result<UnignoreResponse, RpcError> {
        let peer_key = self
            .peer_key(&req.network_key, req.peer_key, req.alias)
            .await?;
        let ix = &ChatUiConfigIgnores::BY_PEER_KEY;
        self.delete_for_peer::<ChatUiConfigIgnores>(ix, &peer_key, "ignore")
            .await?;
        Ok(UnignoreResponse {})
    }

    /// Highlights the holder of the alias.
    async fn highlight(
        &self,
        _: &Context,
        req: HighlightRequest,
    ) -> Result<HighlightResponse, RpcError> {
        let cert = self
            .peer_certificate(&req.network_key, Query::Subject(req.alias))
            .await?;
        let ix = &ChatUiConfigHighlights::BY_PEER_KEY;
        self.put_for_peer::<ChatUiConfigHighlights>(ix, &cert.key.clone(), |id| {
            UiConfigHighlight {
                id,
                version: None,
                alias: cert.subject,
                peer_key: cert.key,
            }
        })
        .await?;
        Ok(HighlightResponse {})
    }

    /// Unhighlights the holder of the peer key, or of the alias when the
    /// request has no key.
    async fn unhighlight(
        &self,
        _: &Context,
        req: UnhighlightRequest,
    ) -> Result<UnhighlightResponse, RpcError> {
        let peer_key = self
            .peer_key(&req.network_key, req.peer_key, req.alias)
            .await?;
        let ix = &ChatUiConfigHighlights::BY_PEER_KEY;
        self.delete_for_peer::<ChatUiConfigHighlights>(ix, &peer_key, "highlight")
            .await?;
        Ok(UnhighlightResponse {})
    }

    /// Tags the holder of the alias with the color.
    async fn tag(&self, _: &Context, req: TagRequest) -> Result<TagResponse, RpcError> {
        if req.color.len() > MAX_NAME_BYTES {
            return Err(rpc_error("tag color too long"));
        }
        let cert = self
            .peer_certificate(&req.network_key, Query::Subject(req.alias))
            .await?;
        let ix = &ChatUiConfigTags::BY_PEER_KEY;
        self.put_for_peer::<ChatUiConfigTags>(ix, &cert.key.clone(), |id| UiConfigTag {
            id,
            version: None,
            alias: cert.subject,
            peer_key: cert.key,
            color: req.color,
        })
        .await?;
        Ok(TagResponse {})
    }

    /// Untags the holder of the peer key, or of the alias when the request
    /// has no key.
    async fn untag(&self, _: &Context, req: UntagRequest) -> Result<UntagResponse, RpcError> {
        let peer_key = self
            .peer_key(&req.network_key, req.peer_key, req.alias)
            .await?;
        let ix = &ChatUiConfigTags::BY_PEER_KEY;
        self.delete_for_peer::<ChatUiConfigTags>(ix, &peer_key, "tag")
            .await?;
        Ok(UntagResponse {})
    }
}
