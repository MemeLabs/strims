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
//! - [`amf0`]: the AMF0 values RTMP commands and metadata carry.
//! - [`rtmp`]: the RTMP server a publisher streams FLV tags to.
//! - [`fmp4`]: fragmented MP4 init segments and fragments, and the reading
//!   of records.
//! - [`segment`]: the muxer cutting access units into fMP4 segments, one
//!   record each, under a [`CutRule`].
//! - [`record`]: where records go: packed into a video swarm's frames
//!   ([`PackedSink`]) or collected whole ([`VecSink`]).
//! - `synth` (feature `test-support`): deterministic FLV tag scripts, the
//!   records they produce, and an RTMP publisher client.
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
//!
//! # Video swarms
//!
//! Every video swarm on the node is the control's: a capture creates one
//! to write, and a stream adds one to read. Each is the transfer of its
//! swarm id under the empty salt, so a transfer id names one video swarm,
//! and a stream of a swarm the node already runs reads it where it is.
//! A video swarm carries opaque records in the packed chunkstream layout
//! ([`strims_chunkstream::packed`]), with at most one record end per
//! frame.
//!
//! - **Captures** ([`capture`]): the UI's recording. A capture has a
//!   fresh key, a swarm of [`capture_options`], its snippet signed with
//!   its key, and a listing of its MIME type and swarm URI on each of its
//!   networks. Its records are appended in order; one that would end in
//!   the frame of the record before it is dropped, and the capture goes
//!   on. A capture closes when asked or with the frontend connection that
//!   opened it, taking its listings, snippet, transfer and swarm with it.
//!   Bounds: [`CaptureSettings`].
//! - **Streams** ([`egress`]): a viewer's read of the swarm a magnet URI
//!   names, as [`EgressPiece`]s: data, record ends, and a discontinuity
//!   before the next whole record once the stream skipped. A swarm added
//!   for streams gets a live window of
//!   [`EgressSettings::window_bytes`], and goes with the last of them; a
//!   capture's stays with the capture. A stream whose swarm closes reads
//!   the swarm that replaced it under its transfer id, if any. Bounds:
//!   [`EgressSettings`].
//!
//! Each video swarm's transfer is published on its networks the node has
//! joined, and on the others as it joins them.

pub mod aac;
pub mod amf0;
pub mod avc;
mod bits;
pub mod capture;
pub mod channels;
mod config;
mod control;
pub mod egress;
mod error;
mod events;
pub mod flv;
pub mod fmp4;
pub mod frontend;
pub mod hls;
pub mod host;
pub mod media;
pub mod record;
mod registry;
pub mod rtmp;
pub mod segment;
#[cfg(any(test, feature = "test-support"))]
pub mod synth;
#[cfg(any(test, feature = "test-support"))]
#[cfg_attr(not(test), allow(dead_code))]
mod test_util;

pub use capture::{CaptureInfo, capture_options};
pub use channels::{ChannelUrl, MAX_CHANNELS};
pub use config::{
    CaptureSettings, Config, ConfigError, EgressSettings, HlsSettings, IngressSettings,
    ListingSettings, MAX_ADDR_BYTES, MAX_NETWORK_KEY_BYTES, MAX_SERVICE_NETWORKS, Metrics,
    RtmpSettings, check_hls_config, check_ingress_config,
};
pub use control::VideoControl;
pub use egress::{EgressPiece, EgressStream, PackedReader, ReaderMetrics};
pub use error::VideoError;
pub use events::VideoEvent;
pub use frontend::{CaptureFrontend, VideoFrontend, bind_frontend};
pub use host::{Accepted, NoVideoHost, VideoHost};
pub use registry::{SwarmKind, VideoSwarm};

pub use flv::{FlvError, FlvMetrics, FlvSource, SourceEvent, Unsupported};
pub use media::{
    AccessUnit, CodecParams, DecoderConfig, MAX_CONFIG_LEN, Muxer, TrackId, TrackKind, fmp4_mime,
};
pub use record::{PackedSink, RecordSink, VecSink};
pub use segment::{CutRule, SegmentError, SegmentInfo, Segmenter, SegmenterMetrics};
