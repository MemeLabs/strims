// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's video calls, served by [`VideoFrontend`]:
//! - `strims.video.v1.VideoChannelFrontend`: the profile's channels
//!   ([`VideoControl::create_channel`] and the rest);
//! - `strims.video.v1.VideoIngress`: whether the node runs the RTMP
//!   ingress, the device's ingress config, the streams it takes, and a
//!   channel's RTMP URL;
//! - `strims.video.v1.HLSEgress`: whether the driver serves HLS and the
//!   device's HLS config. `OpenStream` and `CloseStream` answer "not
//!   implemented".
//!
//! A refused call answers with its [`VideoError`]'s message
//! ([`VideoError::to_rpc`]). A `SetConfig` without a config stores the
//! default one, which it checks like any other.

use std::rc::Rc;

use strims_directory::Host;
use strims_kv::Kv;
use strims_proto::service::{Context, Error as RpcError};
use strims_proto::video::v1::{
    HlsEgressGetConfigRequest, HlsEgressGetConfigResponse, HlsEgressIsSupportedRequest,
    HlsEgressIsSupportedResponse, HlsEgressService, HlsEgressSetConfigRequest,
    HlsEgressSetConfigResponse, VideoChannelCreateRequest, VideoChannelCreateResponse,
    VideoChannelDeleteRequest, VideoChannelDeleteResponse, VideoChannelFrontendService,
    VideoChannelGetRequest, VideoChannelGetResponse, VideoChannelListRequest,
    VideoChannelListResponse, VideoChannelUpdateRequest, VideoChannelUpdateResponse,
    VideoIngressGetChannelUrlRequest, VideoIngressGetChannelUrlResponse,
    VideoIngressGetConfigRequest, VideoIngressGetConfigResponse, VideoIngressIsSupportedRequest,
    VideoIngressIsSupportedResponse, VideoIngressListStreamsRequest,
    VideoIngressListStreamsResponse, VideoIngressService, VideoIngressSetConfigRequest,
    VideoIngressSetConfigResponse, register_hls_egress_service,
    register_video_channel_frontend_service, register_video_ingress_service,
};
use strims_rpc::{Router, Timer};

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

    /// Serves its services on `router`.
    pub fn bind(self, router: &Router) {
        let mut router = router.clone();
        let frontend = Rc::new(self);
        register_video_channel_frontend_service(&mut router, frontend.clone());
        register_video_ingress_service(&mut router, frontend.clone());
        register_hls_egress_service(&mut router, frontend);
    }
}

/// Serves the video frontend of `control` on `router`; see
/// [`VideoFrontend`].
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
}
