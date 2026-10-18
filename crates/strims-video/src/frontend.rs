// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's video calls, served by [`VideoFrontend`]:
//! - `strims.video.v1.VideoChannelFrontend`: the profile's channels
//!   ([`VideoControl::create_channel`] and the rest);
//! - `strims.video.v1.VideoIngress`: whether the node runs the RTMP
//!   ingress, the device's ingress config, the streams it takes, and a
//!   channel's RTMP URL;
//! - `strims.video.v1.HLSEgress`: whether the driver serves HLS, the
//!   device's HLS config, and the node's HLS streams
//!   ([`VideoControl::open_hls`], [`VideoControl::close_hls`]);
//! - `strims.video.v1.Egress`: a stream of a swarm's records
//!   ([`VideoControl::open_stream`]);
//! - `strims.video.v1.Capture`: the connection's captures
//!   ([`CaptureFrontend`]);
//! - `strims.autoseed.v1.AutoseedFrontend`: the device's autoseed config
//!   and rules ([`autoseed`](crate::autoseed)).
//!
//! A refused call answers with its [`VideoError`]'s message
//! ([`VideoError::to_rpc`]). A `SetConfig` without a config stores the
//! default one, which it checks like any other.
//!
//! # Autoseed
//!
//! `CreateRule` stores a rule under a new id, whatever id the request's
//! rule carries, and `UpdateRule` replaces the network key, swarm id,
//! salt and label of rule `id`; a request without a rule carries an empty
//! one, which the bounds refuse. `GetRule` and `UpdateRule` answer
//! "autoseed rule not found" for an id the device has no rule of;
//! `DeleteRule` of such an id succeeds.
//!
//! # Egress
//!
//! `OpenStream` answers an error when the stream doesn't open: its URI
//! or network keys are refused, the node has its most swarms or streams,
//! or the swarm can't be added. An open stream answers `Open` with its
//! transfer id, then `Data` messages of at most
//! [`EgressSettings::max_data_bytes`](crate::EgressSettings::max_data_bytes)
//! bytes of the records:
//!
//! - the message that completes a record has `segment_end`, and its
//!   record's bytes are those of the messages since the last one with
//!   `segment_end`, or with `discontinuity`;
//! - after a discontinuity, the first message has `discontinuity`, and its
//!   bytes begin the next whole record: the bytes of the record in
//!   progress were dropped.
//!
//! A message waits until it is full or completes a record. A stream that
//! fails sends `Error` with the failure's message, and ends. The call's
//! end, or its connection's, closes the stream.
//!
//! # HLS egress
//!
//! `OpenStream` answers the playlist URL of the swarm's HLS stream, the
//! one the node serves or a new one, or an error when it doesn't open.
//! The stream isn't the connection's: it lasts past the call and the
//! connection, until `CloseStream` with its transfer id closes it, it
//! goes idle or HLS is disabled ([`hls_egress`](crate::hls_egress)).
//! `CloseStream` answers "HLS stream not found" when the node has no HLS
//! stream of the transfer.
//!
//! # Capture
//!
//! `Open` opens a capture ([`VideoControl::open_capture`]) that belongs
//! to the connection: `Update`, `Append` and `Close` take only the
//! connection's own captures, answering "capture not found" for any other
//! id, and the connection's end closes them. `Append` answers once its
//! bytes are written.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use futures_util::StreamExt;
use futures_util::stream::unfold;
use strims_directory::Host;
use strims_kv::Kv;
use strims_proto::autoseed::v1::{
    AutoseedFrontendService, CreateRuleRequest, CreateRuleResponse, DeleteRuleRequest,
    DeleteRuleResponse, GetConfigRequest, GetConfigResponse, GetRuleRequest, GetRuleResponse,
    ListRulesRequest, ListRulesResponse, SetConfigRequest, SetConfigResponse, UpdateRuleRequest,
    UpdateRuleResponse, register_autoseed_frontend_service,
};
use strims_proto::service::{Context, Error as RpcError, ServerStream};
use strims_proto::video::v1::egress_open_stream_response::{self, Body};
use strims_proto::video::v1::{
    CaptureAppendRequest, CaptureAppendResponse, CaptureCloseRequest, CaptureCloseResponse,
    CaptureOpenRequest, CaptureOpenResponse, CaptureService, CaptureUpdateRequest,
    CaptureUpdateResponse, EgressOpenStreamRequest, EgressOpenStreamResponse, EgressService,
    HlsEgressCloseStreamRequest, HlsEgressCloseStreamResponse, HlsEgressGetConfigRequest,
    HlsEgressGetConfigResponse, HlsEgressIsSupportedRequest, HlsEgressIsSupportedResponse,
    HlsEgressOpenStreamRequest, HlsEgressOpenStreamResponse, HlsEgressService,
    HlsEgressSetConfigRequest, HlsEgressSetConfigResponse, VideoChannelCreateRequest,
    VideoChannelCreateResponse, VideoChannelDeleteRequest, VideoChannelDeleteResponse,
    VideoChannelFrontendService, VideoChannelGetRequest, VideoChannelGetResponse,
    VideoChannelListRequest, VideoChannelListResponse, VideoChannelUpdateRequest,
    VideoChannelUpdateResponse, VideoIngressGetChannelUrlRequest,
    VideoIngressGetChannelUrlResponse, VideoIngressGetConfigRequest, VideoIngressGetConfigResponse,
    VideoIngressIsSupportedRequest, VideoIngressIsSupportedResponse,
    VideoIngressListStreamsRequest, VideoIngressListStreamsResponse, VideoIngressService,
    VideoIngressSetConfigRequest, VideoIngressSetConfigResponse, register_capture_service,
    register_egress_service, register_hls_egress_service, register_video_channel_frontend_service,
    register_video_ingress_service,
};
use strims_rpc::{Router, Timer};
use strims_transfer::TransferId;

use crate::egress::{EgressPiece, EgressStream};
use crate::{VideoControl, VideoError, VideoHost};

fn rpc(e: VideoError) -> RpcError {
    e.to_rpc()
}

/// Serves the UI's video calls on a node's video control; see the
/// [module docs](self).
pub struct VideoFrontend<K: Kv, H: Host, V: VideoHost, T: Timer> {
    control: VideoControl<K, H, V, T>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> VideoFrontend<K, H, V, T> {
    /// The frontend of `control`.
    pub fn new(control: VideoControl<K, H, V, T>) -> Self {
        VideoFrontend { control }
    }

    /// Serves its services on `router`, a connection's: the captures
    /// opened through it close with it.
    pub fn bind(self, router: &Router) {
        let mut router = router.clone();
        let captures = Rc::new(CaptureFrontend::new(self.control.clone()));
        let frontend = Rc::new(self);
        register_video_channel_frontend_service(&mut router, frontend.clone());
        register_video_ingress_service(&mut router, frontend.clone());
        register_hls_egress_service(&mut router, frontend.clone());
        register_autoseed_frontend_service(&mut router, frontend.clone());
        register_egress_service(&mut router, frontend);
        register_capture_service(&mut router, captures);
    }
}

/// Serves the video frontend of `control` on `router`, a connection's;
/// see [`VideoFrontend`].
pub fn bind_frontend<K: Kv, H: Host, V: VideoHost, T: Timer>(
    router: &Router,
    control: &VideoControl<K, H, V, T>,
) {
    VideoFrontend::new(control.clone()).bind(router);
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> VideoChannelFrontendService
    for VideoFrontend<K, H, V, T>
{
    async fn list(
        &self,
        _: &Context,
        _: VideoChannelListRequest,
    ) -> Result<VideoChannelListResponse, RpcError> {
        let channels = self.control.list_channels().await.map_err(rpc)?;
        Ok(VideoChannelListResponse { channels })
    }

    async fn get(
        &self,
        _: &Context,
        req: VideoChannelGetRequest,
    ) -> Result<VideoChannelGetResponse, RpcError> {
        let channel = self.control.get_channel(req.id).await.map_err(rpc)?;
        Ok(VideoChannelGetResponse {
            channel: Some(channel),
        })
    }

    async fn create(
        &self,
        _: &Context,
        req: VideoChannelCreateRequest,
    ) -> Result<VideoChannelCreateResponse, RpcError> {
        let snippet = req.directory_listing_snippet.unwrap_or_default();
        let channel = self
            .control
            .create_channel(&req.network_key, snippet)
            .await
            .map_err(rpc)?;
        Ok(VideoChannelCreateResponse {
            channel: Some(channel),
        })
    }

    async fn update(
        &self,
        _: &Context,
        req: VideoChannelUpdateRequest,
    ) -> Result<VideoChannelUpdateResponse, RpcError> {
        let snippet = req.directory_listing_snippet.unwrap_or_default();
        let channel = self
            .control
            .update_channel(req.id, &req.network_key, snippet)
            .await
            .map_err(rpc)?;
        Ok(VideoChannelUpdateResponse {
            channel: Some(channel),
        })
    }

    async fn delete(
        &self,
        _: &Context,
        req: VideoChannelDeleteRequest,
    ) -> Result<VideoChannelDeleteResponse, RpcError> {
        self.control.delete_channel(req.id).await.map_err(rpc)?;
        Ok(VideoChannelDeleteResponse {})
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> VideoIngressService for VideoFrontend<K, H, V, T> {
    async fn is_supported(
        &self,
        _: &Context,
        _: VideoIngressIsSupportedRequest,
    ) -> Result<VideoIngressIsSupportedResponse, RpcError> {
        Ok(VideoIngressIsSupportedResponse {
            supported: self.control.ingress_supported(),
        })
    }

    async fn get_config(
        &self,
        _: &Context,
        _: VideoIngressGetConfigRequest,
    ) -> Result<VideoIngressGetConfigResponse, RpcError> {
        let config = self.control.ingress_config().await.map_err(rpc)?;
        Ok(VideoIngressGetConfigResponse {
            config: Some(config),
        })
    }

    async fn set_config(
        &self,
        _: &Context,
        req: VideoIngressSetConfigRequest,
    ) -> Result<VideoIngressSetConfigResponse, RpcError> {
        let config = self
            .control
            .set_ingress_config(req.config.unwrap_or_default())
            .await
            .map_err(rpc)?;
        Ok(VideoIngressSetConfigResponse {
            config: Some(config),
        })
    }

    async fn list_streams(
        &self,
        _: &Context,
        _: VideoIngressListStreamsRequest,
    ) -> Result<VideoIngressListStreamsResponse, RpcError> {
        Ok(VideoIngressListStreamsResponse {
            streams: self.control.ingress_streams(),
        })
    }

    async fn get_channel_url(
        &self,
        _: &Context,
        req: VideoIngressGetChannelUrlRequest,
    ) -> Result<VideoIngressGetChannelUrlResponse, RpcError> {
        let u = self.control.channel_url(req.id).await.map_err(rpc)?;
        Ok(VideoIngressGetChannelUrlResponse {
            url: u.url,
            server_addr: u.server_addr,
            stream_key: u.stream_key,
        })
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> HlsEgressService for VideoFrontend<K, H, V, T> {
    async fn is_supported(
        &self,
        _: &Context,
        _: HlsEgressIsSupportedRequest,
    ) -> Result<HlsEgressIsSupportedResponse, RpcError> {
        Ok(HlsEgressIsSupportedResponse {
            supported: self.control.hls_supported(),
        })
    }

    async fn get_config(
        &self,
        _: &Context,
        _: HlsEgressGetConfigRequest,
    ) -> Result<HlsEgressGetConfigResponse, RpcError> {
        let config = self.control.hls_config().await.map_err(rpc)?;
        Ok(HlsEgressGetConfigResponse {
            config: Some(config),
        })
    }

    async fn set_config(
        &self,
        _: &Context,
        req: HlsEgressSetConfigRequest,
    ) -> Result<HlsEgressSetConfigResponse, RpcError> {
        let config = self
            .control
            .set_hls_config(req.config.unwrap_or_default())
            .await
            .map_err(rpc)?;
        Ok(HlsEgressSetConfigResponse {
            config: Some(config),
        })
    }

    async fn open_stream(
        &self,
        _: &Context,
        req: HlsEgressOpenStreamRequest,
    ) -> Result<HlsEgressOpenStreamResponse, RpcError> {
        let playlist_url = self
            .control
            .open_hls(&req.swarm_uri, &req.network_keys)
            .await
            .map_err(rpc)?;
        Ok(HlsEgressOpenStreamResponse { playlist_url })
    }

    async fn close_stream(
        &self,
        _: &Context,
        req: HlsEgressCloseStreamRequest,
    ) -> Result<HlsEgressCloseStreamResponse, RpcError> {
        let id = TransferId::from_bytes(&req.transfer_id)
            .ok_or_else(|| VideoError::HlsStreamNotFound.to_rpc())?;
        self.control.close_hls(&id).map_err(rpc)?;
        Ok(HlsEgressCloseStreamResponse {})
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> AutoseedFrontendService for VideoFrontend<K, H, V, T> {
    async fn get_config(
        &self,
        _: &Context,
        _: GetConfigRequest,
    ) -> Result<GetConfigResponse, RpcError> {
        let config = self.control.autoseed_config().await.map_err(rpc)?;
        Ok(GetConfigResponse {
            config: Some(config),
        })
    }

    async fn set_config(
        &self,
        _: &Context,
        req: SetConfigRequest,
    ) -> Result<SetConfigResponse, RpcError> {
        let config = self
            .control
            .set_autoseed_config(req.config.unwrap_or_default())
            .await
            .map_err(rpc)?;
        Ok(SetConfigResponse {
            config: Some(config),
        })
    }

    async fn list_rules(
        &self,
        _: &Context,
        _: ListRulesRequest,
    ) -> Result<ListRulesResponse, RpcError> {
        let rules = self.control.list_autoseed_rules().await.map_err(rpc)?;
        Ok(ListRulesResponse { rules })
    }

    async fn get_rule(
        &self,
        _: &Context,
        req: GetRuleRequest,
    ) -> Result<GetRuleResponse, RpcError> {
        let rule = self.control.get_autoseed_rule(req.id).await.map_err(rpc)?;
        Ok(GetRuleResponse { rule: Some(rule) })
    }

    async fn create_rule(
        &self,
        _: &Context,
        req: CreateRuleRequest,
    ) -> Result<CreateRuleResponse, RpcError> {
        let rule = self
            .control
            .create_autoseed_rule(req.rule.unwrap_or_default())
            .await
            .map_err(rpc)?;
        Ok(CreateRuleResponse { rule: Some(rule) })
    }

    async fn update_rule(
        &self,
        _: &Context,
        req: UpdateRuleRequest,
    ) -> Result<UpdateRuleResponse, RpcError> {
        let rule = self
            .control
            .update_autoseed_rule(req.id, req.rule.unwrap_or_default())
            .await
            .map_err(rpc)?;
        Ok(UpdateRuleResponse { rule: Some(rule) })
    }

    async fn delete_rule(
        &self,
        _: &Context,
        req: DeleteRuleRequest,
    ) -> Result<DeleteRuleResponse, RpcError> {
        self.control
            .delete_autoseed_rule(req.id)
            .await
            .map_err(rpc)?;
        Ok(DeleteRuleResponse {})
    }
}

// --- egress -------------------------------------------------------------------

/// An `OpenStream` call's stream, and the message it fills.
struct Feed<K: Kv, H: Host, V: VideoHost, T: Timer> {
    stream: EgressStream<K, H, V, T>,
    /// The next `Data` message's bytes. Bound: `max`.
    data: Vec<u8>,
    max: usize,
    /// The next `Data` message has `discontinuity`.
    discontinuity: bool,
    opened: bool,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Feed<K, H, V, T> {
    /// The next message, and whether the stream goes on.
    async fn next(&mut self) -> (EgressOpenStreamResponse, bool) {
        if !self.opened {
            self.opened = true;
            let open = egress_open_stream_response::Open {
                transfer_id: self.stream.transfer_id().as_bytes().to_vec(),
            };
            return (response(Body::Open(open)), true);
        }
        loop {
            let room = self.max.saturating_sub(self.data.len());
            match self.stream.next(room).await {
                Ok(EgressPiece::Data(d)) => {
                    self.data.extend_from_slice(d);
                    if self.data.len() >= self.max {
                        return (self.data_message(false), true);
                    }
                }
                Ok(EgressPiece::RecordEnd) => return (self.data_message(true), true),
                Ok(EgressPiece::Discontinuity) => {
                    self.data.clear();
                    self.discontinuity = true;
                }
                Err(e) => {
                    let error = egress_open_stream_response::Error {
                        message: e.to_string(),
                    };
                    return (response(Body::Error(error)), false);
                }
            }
        }
    }

    fn data_message(&mut self, segment_end: bool) -> EgressOpenStreamResponse {
        let data = std::mem::replace(&mut self.data, Vec::with_capacity(self.max));
        response(Body::Data(egress_open_stream_response::Data {
            data,
            segment_end,
            discontinuity: std::mem::take(&mut self.discontinuity),
        }))
    }
}

fn response(body: Body) -> EgressOpenStreamResponse {
    EgressOpenStreamResponse { body: Some(body) }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> EgressService for VideoFrontend<K, H, V, T> {
    async fn open_stream(
        &self,
        _: &Context,
        req: EgressOpenStreamRequest,
    ) -> Result<ServerStream<EgressOpenStreamResponse>, RpcError> {
        let stream = self
            .control
            .open_stream(&req.swarm_uri, &req.network_keys)
            .await
            .map_err(rpc)?;
        let max = self.control.shared.config.egress.max_data_bytes.max(1);
        let feed = Feed {
            stream,
            data: Vec::with_capacity(max),
            max,
            discontinuity: false,
            opened: false,
        };
        Ok(unfold(Some(feed), |feed| async move {
            let mut feed = feed?;
            let (message, more) = feed.next().await;
            Some((Ok(message), more.then_some(feed)))
        })
        .boxed_local())
    }
}

// --- capture ------------------------------------------------------------------

/// Serves `strims.video.v1.Capture` for one frontend connection: its
/// `Update`, `Append` and `Close` take only the captures its `Open`
/// opened, and dropping it, as its connection closes, closes them.
pub struct CaptureFrontend<K: Kv, H: Host, V: VideoHost, T: Timer> {
    control: VideoControl<K, H, V, T>,
    /// Bound: the node's captures
    /// ([`CaptureSettings::max_captures`](crate::CaptureSettings::max_captures)).
    owned: RefCell<BTreeSet<TransferId>>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> CaptureFrontend<K, H, V, T> {
    pub fn new(control: VideoControl<K, H, V, T>) -> Self {
        CaptureFrontend {
            control,
            owned: RefCell::new(BTreeSet::new()),
        }
    }

    /// Capture `id` if this connection opened it.
    fn owned(&self, id: &[u8]) -> Result<TransferId, RpcError> {
        TransferId::from_bytes(id)
            .filter(|id| self.owned.borrow().contains(id))
            .ok_or_else(|| VideoError::CaptureNotFound.to_rpc())
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Drop for CaptureFrontend<K, H, V, T> {
    fn drop(&mut self) {
        for id in std::mem::take(self.owned.get_mut()) {
            self.control.shared.queue_close_capture(&id);
        }
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> CaptureService for CaptureFrontend<K, H, V, T> {
    async fn open(
        &self,
        _: &Context,
        req: CaptureOpenRequest,
    ) -> Result<CaptureOpenResponse, RpcError> {
        let id = self
            .control
            .open_capture(
                &req.mime_type,
                req.network_keys,
                req.directory_snippet.unwrap_or_default(),
            )
            .await
            .map_err(rpc)?;
        let open = self.control.shared.captures.borrow();
        let mut owned = self.owned.borrow_mut();
        owned.retain(|id| open.contains_key(id));
        owned.insert(id);
        Ok(CaptureOpenResponse {
            id: id.as_bytes().to_vec(),
        })
    }

    async fn update(
        &self,
        _: &Context,
        req: CaptureUpdateRequest,
    ) -> Result<CaptureUpdateResponse, RpcError> {
        let id = self.owned(&req.id)?;
        let snippet = req.directory_snippet.unwrap_or_default();
        self.control.update_capture(&id, snippet).map_err(rpc)?;
        Ok(CaptureUpdateResponse {})
    }

    async fn append(
        &self,
        _: &Context,
        req: CaptureAppendRequest,
    ) -> Result<CaptureAppendResponse, RpcError> {
        let id = self.owned(&req.id)?;
        self.control
            .append_capture(&id, req.data, req.segment_end)
            .await
            .map_err(rpc)?;
        Ok(CaptureAppendResponse {})
    }

    async fn close(
        &self,
        _: &Context,
        req: CaptureCloseRequest,
    ) -> Result<CaptureCloseResponse, RpcError> {
        let id = self.owned(&req.id)?;
        self.owned.borrow_mut().remove(&id);
        self.control.close_capture(&id).await.map_err(rpc)?;
        Ok(CaptureCloseResponse {})
    }
}
