// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Live video.
//!
//! Sources produce access units ([`AccessUnit`]): one coded frame of a
//! track, with its decode and presentation timestamps, keyframe flag and
//! publish time. Each track has a decoder config ([`DecoderConfig`]): an
//! RFC 6381 codec string, the timescale of the track's timestamps and the
//! codec's parameters. A source emits a track's config before the track's
//! first access unit and again whenever it changes. Muxers ([`Muxer`])
//! consume both.
//!
//! - [`media`]: tracks, decoder configs, access units, the [`Muxer`]
//!   trait and the fMP4 MIME type.
//! - [`avc`]: H.264 avcC records and the picture size from their SPS.
//! - [`aac`]: AAC AudioSpecificConfig and the `esds` box.
//! - [`flv`]: the source over FLV tags, legacy and Enhanced RTMP. It takes
//!   H.264 with AAC and refuses other codecs.
//! - [`hls`]: HLS playlists of fMP4 segments, and the routes a driver
//!   serves them on.
//!
//! Every parser takes hostile input: none panics, and decoder configs are
//! bounded by [`MAX_CONFIG_LEN`].
//!
//! # Running
//!
//! [`VideoControl`] runs on the node the driver supplies through
//! [`strims_directory::Host`] and a [`VideoHost`] (its RTMP listener;
//! [`NoVideoHost`] where there is none), beside the node's
//! [`NetworkControl`](strims_network::NetworkControl),
//! [`TransferControl`](strims_transfer::TransferControl) and
//! [`DirectoryControl`](strims_directory::DirectoryControl). The driver
//! runs [`VideoControl::run`] as a local task.
//!
//! - The profile's channels ([`channels`]) are records in its store.
//!   Each has its own key and token; its stream key is the token signed
//!   by its owner ([`VideoControl::channel_url`]). Deleting a network
//!   deletes the channels on it.
//! - The device's ingress and HLS configs are records in the store,
//!   checked before they are stored ([`check_ingress_config`],
//!   [`check_hls_config`]). Ingress is supported where the [`VideoHost`]
//!   listens, HLS where the driver serves it ([`HlsSettings::serve`]).
//! - What changes is reported as [`VideoEvent`]s.
//!
//! [`bind_frontend`] serves the UI's video calls on the control
//! ([`frontend`]). [`Config`] holds every bound the control and its
//! driver apply, each documented where it is declared.

pub mod aac;
pub mod avc;
mod bits;
pub mod channels;
mod config;
mod control;
mod error;
mod events;
pub mod flv;
pub mod frontend;
pub mod hls;
pub mod host;
pub mod media;
#[cfg(test)]
mod test_util;

pub use channels::{ChannelUrl, MAX_CHANNELS};
pub use config::{
    CaptureSettings, Config, ConfigError, EgressSettings, HlsSettings, IngressSettings,
    ListingSettings, MAX_ADDR_BYTES, MAX_NETWORK_KEY_BYTES, MAX_SERVICE_NETWORKS, Metrics,
    RtmpSettings, check_hls_config, check_ingress_config,
};
pub use control::VideoControl;
pub use error::VideoError;
pub use events::VideoEvent;
pub use frontend::{VideoFrontend, bind_frontend};
pub use host::{Accepted, NoVideoHost, VideoHost};

pub use flv::{FlvError, FlvMetrics, FlvSource, SourceEvent, Unsupported};
pub use media::{
    AccessUnit, CodecParams, DecoderConfig, MAX_CONFIG_LEN, Muxer, TrackId, TrackKind, fmp4_mime,
};
