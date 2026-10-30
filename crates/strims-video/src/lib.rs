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
//!   trait and the MIME types of the fMP4 and unit formats.
//! - [`avc`]: H.264 avcC records and the picture size from their SPS.
//! - [`aac`]: AAC AudioSpecificConfig and the `esds` box.
//! - [`opus`]: the Opus identification header, OpusHead, and the `dOps`
//!   box.
//! - [`flv`]: the source over FLV tags, legacy and Enhanced RTMP. It takes
//!   H.264 with AAC and refuses other codecs.
//! - [`amf0`]: the AMF0 values RTMP commands and metadata carry.
//! - [`rtmp`]: the RTMP server a publisher streams FLV tags to.
//! - [`fmp4`]: fragmented MP4 init segments and fragments, each with its
//!   `prft`, the [`fmp4::FragmentWriter`] that turns access units into
//!   fragments, and the reading of records.
//! - [`segment`]: the muxer cutting access units into fMP4 segments, one
//!   record each written fragment by fragment, under a [`CutRule`].
//! - [`record`]: where records go: packed into a video swarm's frames
//!   ([`PackedSink`]) or collected whole ([`VecSink`]).
//! - `synth` (feature `test-support`): deterministic FLV tag scripts, the
//!   records they produce, and an RTMP publisher client.
//! - [`hls`]: HLS playlists of fMP4 segments, the muxer that cuts access
//!   units into them, and the routes a driver serves them on.
//! - [`units`]: the unit format, a record per access unit in the chunked
//!   chunkstream layout: its records, the [`UnitWriter`] muxer and the
//!   [`UnitReader`], which builds for wasm32.
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
//! [`DirectoryControl`](strims_directory::DirectoryControl), with the
//! [`hls::HlsRoutes`] the driver's HTTP server answers HLS requests from.
//! The driver runs [`VideoControl::run`] as a local task.
//!
//! - The profile's channels ([`channels`]) are records in its store.
//!   Each has its own key and token; its stream key is the token signed
//!   by its owner ([`VideoControl::channel_url`]). Deleting a network
//!   deletes the channels on it.
//! - The device's ingress and HLS configs are records in the store,
//!   checked before they are stored ([`check_ingress_config`],
//!   [`check_hls_config`]). Ingress is supported where the [`VideoHost`]
//!   listens, HLS where the driver serves it ([`HlsSettings::serve`]).
//! - The RTMP ingress ([`ingress`]) listens on the ingress config's
//!   address while it is enabled, binding again when the address changes
//!   and after a failed bind. A publish to `live/<stream key>` of a
//!   channel whose network the node has joined starts the channel's
//!   session, and a second publish to the channel replaces it. Its
//!   connections, deadlines and sessions are bounded by
//!   [`IngressSettings`]; [`VideoControl::ingress_streams`] lists the
//!   live sessions.
//! - The device's autoseed config and rules ([`autoseed`]) are records in
//!   the store. While the config is enabled, the node seeds the swarms of
//!   the directory listings the rules name. Deleting a network deletes
//!   the rules on it.
//! - What changes is reported as [`VideoEvent`]s, with the ingress's
//!   sessions and listener failures.
//!
//! [`bind_frontend`] serves the UI's video and autoseed calls on the
//! control ([`frontend`]). [`Config`] holds every bound the control and its
//! driver apply, each documented where it is declared.
//!
//! # Video swarms
//!
//! Every video swarm on the node is the control's: an ingest session or a
//! capture creates one to write, a stream adds one to read, and a seed
//! one to seed. Each is the transfer of its swarm id under the empty
//! salt, so a transfer id names one video swarm, and a stream or a seed
//! of a swarm the node already runs takes it where it is.
//! A video swarm carries records in the layout of its format, which its
//! listing's MIME type names: opaque records in the packed chunkstream
//! layout ([`strims_chunkstream::packed`]), with at most one record end per
//! frame, or the [unit format](units)'s records in the chunked layout
//! ([`strims_chunkstream::chunked`]).
//!
//! - **Ingest** ([`ingress`]): a broadcaster's RTMP stream to a channel.
//!   Its session's swarm is under the channel key, so the swarm id is
//!   the channel's public key. Its records are the unit format's,
//!   written by a [`UnitWriter`] and signed early, in a swarm whose unit
//!   size and signing interval the stream's nominal bitrate or tracks
//!   pick ([`units::UnitSigning`]) once its metadata or decoder configs
//!   come, after the publish's accept; or with [`IngestFormat::Fmp4`]
//!   fMP4 segments, each starting at a keyframe and cut by a
//!   [`Segmenter`] into a [`PackedSink`], in a swarm of
//!   [`ingress_options`]; a record over its bound, or a codec the
//!   [`FlvSource`] refuses, ends the session with the reason. The session serves the channel's snippet, live from the
//!   publish's start time and signed with the channel key, and keeps a
//!   listing of the records' MIME type and swarm URI on the channel's
//!   network from its first record. A changed channel serves the snippet
//!   again; a deleted or moved one ends the session. Its end writes the
//!   record in progress (and has a unit-format stream's last partial
//!   integrity unit signed), then takes the listing, snippet, transfer, slot
//!   and swarm down; a session that replaces it keeps the slot, so the
//!   node's streams of the channel go on after a discontinuity.
//! - **Captures** ([`capture`]): the UI's encoded media. A capture has a
//!   fresh key, a swarm of [`capture_options_for`] its nominal bitrate,
//!   its snippet signed with its key, and, from its first record, a
//!   listing of its unit-format MIME type and swarm URI on each of its
//!   networks. Its decoder configs and access units are checked and
//!   written in order through a [`UnitWriter`]; an
//!   append that breaks its rules is refused whole, and the capture goes
//!   on. A capture closes when asked or with the frontend connection that
//!   opened it, signing its last partial unit and taking its listings,
//!   snippet, transfer and swarm with it. Bounds: [`CaptureSettings`].
//! - **Streams** ([`egress`]): a viewer's read of the swarm a magnet URI
//!   names, in the layout of its [`StreamFormat`], as [`EgressPiece`]s:
//!   data, record ends, and a discontinuity before the next whole record
//!   once the stream skipped. A unit-format stream starts at the GOP it
//!   joins. A swarm added for streams gets a live window of
//!   [`EgressSettings::window_bytes`], and goes with the last of them; a
//!   capture's or an ingest session's stays with it. A stream whose swarm
//!   closes reads the swarm that replaced it under its transfer id, if
//!   any. Bounds: [`EgressSettings`].
//! - **Seeds** ([`autoseed`]): an autoseed rule's hold on the swarm of a
//!   listing it names: a swarm streams read, or one added as a stream's
//!   is. The swarm goes with the last of its seeds and streams; a
//!   capture's or an ingest session's takes no seed. A rule under a
//!   non-empty salt seeds a transfer of that salt outside the video
//!   swarms. Bounds: [`AutoseedSettings`].
//!
//! Each video swarm's transfer is published on its networks the node has
//! joined, and on the others as it joins them.
//!
//! # HLS egress
//!
//! Where the driver serves HLS over HTTP ([`HlsSettings::serve`]), the
//! node serves video swarms to external players as HLS streams
//! ([`hls_egress`]). [`VideoControl::open_hls`] reads a swarm as a
//! stream in the format the listing's MIME type names, counted among the
//! [`EgressSettings`] bounds, and answers its playlist URL.
//!
//! - **Contract.** A playlist's target duration is
//!   [`HlsSettings::target_duration`] for its life, and every segment
//!   rounds to at most it. A whole fMP4 record ([`fmp4::split_record`]) is
//!   a segment lasting its media time ([`fmp4::media_duration`]), or runs
//!   of its fragments when it lasts longer; other records are skipped. A
//!   unit-format stream's access units are cut into
//!   segments at its keyframes, and mid-GOP when a GOP outlasts the target
//!   ([`hls::HlsMuxer`]). Segments are served with their inits on the
//!   driver's [`hls::HlsRoutes`]. A skipped record, a discontinuity of an
//!   fMP4 stream, or units lost from a unit-format one mark the next
//!   segment discontinuous.
//! - **URL.** The HLS config's public server address, or else the
//!   driver's [`HlsSettings::base_url`], followed by
//!   [`hls::playlist_path`] of the profile's key and the swarm's transfer
//!   id. With neither address, no stream opens.
//! - **Lifetime.** A stream is the node's, whoever opened it, and a swarm
//!   has one: opening it again answers the same stream. It lasts until
//!   [`VideoControl::close_hls`] (the UI's `CloseStream`) closes it, no
//!   request reaches it for [`HlsSettings::idle_timeout`], the HLS config
//!   is disabled, or its stream fails. Then its routes answer 404 and its
//!   swarm is released.
//! - **Bounds.** [`HlsSettings`]: the streams the node serves, the record
//!   a stream holds while it arrives, and the segments and bytes each
//!   playlist keeps.

pub mod aac;
pub mod amf0;
pub mod autoseed;
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
pub mod hls_egress;
pub mod host;
pub mod ingress;
pub mod media;
pub mod opus;
pub mod record;
mod registry;
pub mod rtmp;
pub mod segment;
#[cfg(any(test, feature = "test-support"))]
pub mod synth;
#[cfg(any(test, feature = "test-support"))]
#[cfg_attr(not(test), allow(dead_code))]
mod test_util;
pub mod units;

pub use capture::{CaptureInfo, capture_options, capture_options_for};
pub use channels::{ChannelUrl, MAX_CHANNELS};
pub use config::{
    AutoseedSettings, CaptureSettings, Config, ConfigError, EgressSettings, HlsSettings,
    IngestFormat, IngressSettings, ListingSettings, MAX_ADDR_BYTES, MAX_NETWORK_KEY_BYTES,
    MAX_SERVICE_NETWORKS, Metrics, RtmpSettings, check_hls_config, check_ingress_config,
};
pub use control::VideoControl;
pub use egress::{
    ChunkedReader, EgressPiece, EgressStream, PackedReader, ReaderMetrics, Rewind, StreamFormat,
};
pub use error::VideoError;
pub use events::VideoEvent;
pub use frontend::{CaptureFrontend, VideoFrontend, bind_frontend};
pub use hls_egress::{HlsCloseReason, HlsStreamInfo};
pub use host::{Accepted, NoVideoHost, VideoHost};
pub use ingress::{IngressMetrics, ingress_options, unit_ingress_options};
pub use registry::{SwarmKind, VideoSwarm};

pub use flv::{FlvError, FlvMetrics, FlvSource, SourceEvent, Unsupported};
pub use media::{
    AccessUnit, CodecParams, DecoderConfig, MAX_CONFIG_LEN, Muxer, TrackId, TrackKind, fmp4_mime,
    mime_essence, units_mime,
};
pub use record::{PackedSink, RecordSink, VecSink};
pub use segment::{CutRule, SegmentError, SegmentInfo, Segmenter, SegmenterMetrics};
pub use units::{UnitReader, UnitWriter};
