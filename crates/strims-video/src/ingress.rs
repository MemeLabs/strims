// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The RTMP ingress: a broadcaster's stream to one of the profile's
//! channels, written into the channel's live video swarm in the unit
//! format.
//!
//! # Listener
//!
//! While the device's ingress config is enabled, its listen address
//! passes [`check_ingress_config`] and the [`VideoHost`] runs listeners,
//! the control listens on that address. A changed address drops the
//! listener and binds the new one; the connections it took go on.
//! Disabling the config drops the listener and ends every connection and
//! session. A bind that fails, and a listener that fails or ends, are
//! reported ([`VideoEvent::IngressListenFailed`]) and bound again after
//! [`IngressSettings::bind_retry`](crate::IngressSettings::bind_retry).
//!
//! The control holds at most
//! [`IngressSettings::max_connections`](crate::IngressSettings::max_connections)
//! connections, and at most
//! [`IngressSettings::max_unpublished_per_address`](crate::IngressSettings::max_unpublished_per_address)
//! from one remote host that haven't published. It closes a connection
//! over either cap as it comes.
//!
//! # Connections
//!
//! Each connection runs an [`RtmpServer`] in a task of its own. The
//! server's replies go out as fast as the connection takes them; a peer
//! that doesn't read lets them queue up to
//! [`RtmpSettings::max_queued_output`](crate::RtmpSettings::max_queued_output),
//! and the connection fails past it. From its accept, a connection has
//! [`IngressSettings::handshake_deadline`](crate::IngressSettings::handshake_deadline)
//! to finish the handshake and
//! [`IngressSettings::publish_deadline`](crate::IngressSettings::publish_deadline)
//! to have a publish accepted; a publishing connection ends after
//! [`IngressSettings::media_idle`](crate::IngressSettings::media_idle)
//! without audio or video. A connection carries one publish, and closes
//! when it ends. A closing connection has
//! [`IngressSettings::close_timeout`](crate::IngressSettings::close_timeout)
//! to take its last output: the status telling a publisher why the
//! server ended its stream. It then closes its sending side and takes
//! the publisher's input until the publisher closes too, so the status
//! isn't lost to a reset.
//!
//! # Publishing
//!
//! A publish is refused with `NetStream.Publish.BadName` unless its app
//! is `live` and its name is the stream key of one of the profile's
//! channels ([`channel_by_stream_key`]), and with
//! `NetStream.Publish.Failed` unless the node has joined the channel's
//! network and, but for a session of the same channel, runs fewer than
//! [`IngressSettings::max_sessions`](crate::IngressSettings::max_sessions)
//! sessions.
//!
//! An accepted publish starts the channel's session:
//!
//! - **The swarm.** The channel's swarm is created under the channel key,
//!   so its id is the channel's public key, with [`unit_ingress_options`]
//!   of its signing once the session's signing settles, after the accept
//!   (see [Media](#media)). Its transfer, under the empty salt, is
//!   published on the channel's network. A swarm that can't be created
//!   ends its session, the publisher hearing why.
//! - **The snippet.** The channel's snippet, with `start_time` the
//!   publish's unix time and `live` set, signed with the channel key, is
//!   served for the swarm. A changed channel record is signed and served
//!   again, with the same start time.
//! - **The listing.** Once the first record is written, a media listing
//!   of the stream's MIME type ([`units_mime`] of its decoder configs)
//!   and the swarm URI is kept on the directory of the
//!   channel's network. A record of other codecs lists the new MIME type
//!   in its place.
//!
//! A publish to a channel that streams replaces its session: the old
//! connection closes, and the new session takes over the old one's
//! listing, if on the same network, and its slot among the node's video
//! swarms. The slot's swarm goes with the publish, and the slot takes the
//! new session's swarm, with that session's options, once it is created,
//! so the node's streams of the channel go on after a discontinuity. A
//! session's signing is its own stream's: when the session it replaced
//! signed otherwise, its swarm URI, and so its listing's, is another. A
//! deleted channel, or one moved to another network, ends its session.
//!
//! # Media
//!
//! Audio and video go through an [`FlvSource`] into a [`UnitWriter`] in
//! frames of the swarm's chunk size, whose records go into the swarm as
//! each unit arrives. A unit's publish time is when the connection took
//! its message from the RTMP server, on the node's clock. A codec or tag
//! the source refuses, video other than H.264, which needs a node that
//! transcodes it, or a unit or config the writer refuses, ends the
//! session; the publisher gets the reason in an error status. Metadata is
//! counted, and its bitrates and tracks read.
//!
//! Subtitles, `onTextData` messages, are cues of the source's text track
//! ([`text`](crate::text)), declared once: when the metadata declares a
//! data stream, or else at the first cue. The channel's swarm and every
//! rendition carry the track; the transcode doesn't take it. A cue the
//! writer refuses is dropped, and cues don't keep a session from idling.
//!
//! The swarm's signing, its unit size and interval ([`UnitSigning`]), is
//! the stream's, so the session creates the swarm once it knows enough of
//! the stream:
//!
//! - **What it reads.** The bitrates `onMetaData` states,
//!   `videodatarate` and `audiodatarate` in kbit/s, each counted when
//!   finite and positive, and the tracks it names: video by any of
//!   `width`, `height`, `framerate`, `videocodecid` and `videodatarate`,
//!   audio by any of `audiocodecid`, `audiosamplerate` and `audiodatarate`.
//!   The decoder configs name tracks too. The rates are the stream's
//!   nominal bitrate when they reach
//!   [`UnitSigning::SMALL_BELOW_BPS`] or every track named has one;
//!   otherwise the stream has none. [`UnitSigning::for_stream`] picks
//!   from that, and from whether the stream is audio alone.
//! - **When.** At the first of: metadata that names a track; a video
//!   config; [`IngressSettings::signing_wait`](crate::IngressSettings::signing_wait)
//!   after the first access unit; and 256 KiB of output held.
//!   The last two settle on what is known: a stream of no named track
//!   takes [`UnitSigning::LARGE`]. Encoders send metadata and decoder
//!   configs ahead of their media, so a stream with either has its swarm
//!   before its first unit; audio alone without metadata waits at its
//!   start. Metadata after the settling changes nothing.
//! - **Meanwhile** the writer holds its output, which goes into the swarm
//!   once it exists, so no unit is lost.
//!
//! A partial integrity unit is signed the signing's interval after its
//! first byte ([`SignTimer`]): the writer ends its frame, the swarm writer
//! is flushed, which pads the unit with zeros that read as padding frames,
//! and the writer goes on where the next unit starts.
//!
//! # Renditions
//!
//! Where the node has a codec stage ([`VideoHost::codec`]), a session
//! plans its renditions at its stream's first video config, as
//! [`TranscodeSettings`](crate::TranscodeSettings) have them:
//!
//! - **H.264.** The channel's swarm carries the stream as it came. Each
//!   rung of the ladder whose shorter side is below the stream's is a
//!   rendition.
//! - **HEVC and AV1.** The channel's swarm carries the H.264 the
//!   transcode makes of the stream: at its shorter side, scaled down to
//!   the bitrate table's last, at the table's bitrate, and at most
//!   `h264_max_frame_rate` frames a second. The stream as it came is the
//!   `source` rendition, and the ladder's rungs below the H.264 follow it.
//!   Without a codec stage, or past
//!   [`max_sessions`](crate::TranscodeSettings::max_sessions) transcodes,
//!   such a publish ends with the reason. A stream whose video codec
//!   changes ends.
//! - **Swarms.** Each rendition is a swarm of its own under a key the
//!   channel key derives for its id (the HMAC-SHA256 of
//!   `strims-video-rendition:` and the id), so its swarm id is the same in
//!   every session of the channel: `source`, or a rung's shorter side and
//!   frame rate, such as `480p30`. A
//!   [`UnitWriter`] writes it, and it is signed early as its bitrate has
//!   it. It is created once its first config comes, and published on the
//!   channel's network. A session that replaces another takes the slots
//!   of the renditions it makes, so their streams go on; the others go
//!   with it.
//! - **Captions.** The stream's A/53 closed captions
//!   ([`captions`](crate::captions)) reach every rendition the transcode
//!   makes, whose encoder writes none: each of its video frames carries,
//!   in an SEI before its first slice, the caption tuples of the stream's
//!   frames since its last, up to its time, in presentation order. A
//!   transcode that starts again carries those of the frames it took.
//! - **The transcode** starts at the stream's first video keyframe. Input
//!   past its queue's bound is dropped up to the next keyframe. A
//!   transcode that holds input without output for
//!   [`stall_timeout`](crate::TranscodeSettings::stall_timeout), fails or
//!   ends starts again at the next keyframe, at most
//!   [`max_restarts`](crate::TranscodeSettings::max_restarts) times in
//!   its restart window; past that, an H.264 session's renditions write
//!   their END and leave the snippet, and an HEVC or AV1 session ends.
//! - **Listing.** The snippet lists every rendition once it wrote a
//!   record (`ListingSnippet::renditions`): the channel's swarm first,
//!   then the others, each with its MIME type, swarm URI, nominal
//!   bitrate, picture size and frame rate, and its video size is the
//!   channel swarm's. The snippet is signed with the channel key, which
//!   binds the renditions to the stream.
//! - **End.** A session's end ends the transcode's input and waits up to
//!   [`finish_timeout`](crate::TranscodeSettings::finish_timeout) for its
//!   last output. Then each rendition writes its END on a deliberate end
//!   and has its last partial unit signed, and the renditions' swarms go
//!   with the channel's.
//!
//! [`TranscodeMetrics`](crate::TranscodeMetrics) count what the
//! transcodes did.
//!
//! # Thumbnails
//!
//! Where the node has a codec stage, a live session's snippet shows a
//! thumbnail of its stream in place of its channel's, as
//! [`ThumbnailSettings`](crate::ThumbnailSettings) have it:
//!
//! - **When.** The first is made of the first video keyframe to arrive
//!   [`first_after`](crate::ThumbnailSettings::first_after) or more after
//!   the stream's first video unit, and each later one of the first
//!   keyframe [`interval`](crate::ThumbnailSettings::interval) or more
//!   after the last began. One that fails, or that the codec stage doesn't
//!   make within [`timeout`](crate::ThumbnailSettings::timeout), is tried
//!   again [`retry`](crate::ThumbnailSettings::retry) later at half the
//!   width. One is made at a time.
//! - **What.** A JPEG of the keyframe in the stream's own encoding, at
//!   its aspect ratio within the settings' box, of at most the snippet's
//!   image bound ([`MAX_IMAGE_BYTES`](strims_directory::snippet::MAX_IMAGE_BYTES)).
//! - **The snippet** with it is signed and served again, and keeps it
//!   through the channel's changes. A session that replaces another shows
//!   the other's until it makes its own; any other starts with its
//!   channel's.
//!
//! # Teardown
//!
//! A session ends when its publisher unpublishes or disconnects, at a
//! deadline, on an error, when a new publish replaces it, when the
//! ingress is disabled, when its channel is deleted or moved, and when
//! the control stops. An unpublish (`deleteStream`, `FCUnpublish` or
//! `closeStream`) and the control's stops are deliberate ends; after any
//! other end the encoder may connect again and resume. Its end:
//!
//! 1. writes what the writer holds, and has the stream's last partial
//!    integrity unit signed. On a deliberate end, the session writes its
//!    END record ([`units`](crate::units#records)) after its last unit,
//!    before that signing. A session without its swarm yet, never listed,
//!    writes nothing.
//!
//!    After any other end, a session that wrote records waits
//!    up to
//!    [`IngressSettings::resume_grace`](crate::IngressSettings::resume_grace)
//!    for a publish to its channel, keeping its listing and swarm. Such a
//!    publish replaces the session at once, taking over its listing keeper
//!    and its swarm's slot. A session still its channel's when the wait
//!    ends, or when the control stops it meanwhile, writes and signs its
//!    END then.
//! 2. stops the listing keeper, which unpublishes the listing, waiting at
//!    most the listing settings'
//!    [`unpublish_timeout`](crate::ListingSettings::unpublish_timeout),
//!    and keeps the swarm until [`END_LINGER`] after its END was signed,
//!    for its peers to fetch the END, unless a publish to the channel
//!    replaces the session first;
//! 3. stops serving the snippet;
//! 4. removes the transfer;
//! 5. removes the swarm's slot, ending the node's streams of it;
//! 6. removes the swarm.
//!
//! The connection takes its last output and closes after step 1's writes,
//! and stops counting against the connection caps. A replaced session
//! takes none of these steps: what they take down is the replacing
//! session's. A control that stops takes steps 2 to 5 at once,
//! without waiting for the unpublish, and leaves the swarm to the node.
//! [`VideoEvent::IngressEnded`] reports the end of a session that
//! started, with its reason.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use futures_sink::Sink;
use futures_util::future::select;
use futures_util::{FutureExt, StreamExt};
use strims_base::time::{Duration, Time};
use strims_crypto::Signer;
use strims_dao::schema::video::{channel_by_stream_key, channel_network_key, parse_stream_key};
use strims_directory::{Host, ListingKeeper, SignedSnippet};
use strims_kv::Kv;
use strims_ppspp::integrity::{
    LiveSignatureAlgorithm, MerkleHashTreeFunction, ProtectionMethod, VerifierOptions,
};
use strims_ppspp::{SwarmId, SwarmOptions, Uri, WriterOptions};
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::network::v1::directory::{
    Listing, ListingRendition, ListingSnippet, ListingSnippetImage,
};
use strims_proto::types::Key;
use strims_proto::video::v1::{VideoChannel, VideoIngressConfig, VideoIngressStream};
use strims_rpc::Timer;
use strims_transfer::{SwarmHost, SwarmWrite, TransferId};

use crate::amf0::Metadata;
use crate::config::{RtmpSettings, check_ingress_config};
use crate::control::{Shared, Slot, Swarm};
use crate::flv::{FlvSource, SourceEvent};
use crate::host::{Accepted, VideoHost};
use crate::media::{CodecParams, DecoderConfig, Muxer, TrackKind, units_mime};
use crate::registry::{Added, SwarmKind};
use crate::rtmp::{Phase, RtmpConfig, RtmpError, RtmpEvent, RtmpServer};
use crate::units::{END_LINGER, SignTimer, UnitSigning, UnitWriter, UnitWriterMetrics};
use crate::{VideoError, VideoEvent};

#[path = "renditions.rs"]
mod renditions;
#[path = "thumbnails.rs"]
mod thumbnails;

/// The app a publish names.
const LIVE_APP: &str = "live";
/// The status codes of a refused or ended publish.
const BAD_NAME: &str = "NetStream.Publish.BadName";
const FAILED: &str = "NetStream.Publish.Failed";
/// The bytes of server output handed to the connection at once.
const WRITE_CHUNK: usize = 16 * 1024;
/// The input reads a closing connection takes per poll before it yields.
const CLOSE_DRAIN_READS: usize = 16;

/// Why the control ends a session or connection.
const REPLACED: &str = "replaced by a new publish to the channel";
const DISABLED: &str = "the ingress was disabled";
const CHANNEL_DELETED: &str = "the channel was deleted";
const CHANNEL_MOVED: &str = "the channel moved to another network";

/// The options of a session's swarm signing [`UnitSigning::LARGE`], and
/// of every session's but its chunks per signature
/// ([`unit_ingress_options`]); its URI carries every one but the window.
/// 4 KiB chunks, 8 per signature (32 KiB integrity units), 16 streams, a
/// 4 Ki-chunk (16 MiB) window, Merkle/BLAKE3/Ed25519. Below
/// 4 KiB, HAVEs, requests and hashes take a large share of the wire, and a
/// larger integrity unit holds a chunk back until the unit is signed.
pub fn ingress_options() -> SwarmOptions {
    SwarmOptions {
        chunk_size: 4096,
        chunks_per_signature: 8,
        stream_count: 16,
        live_window: 4 * 1024,
        integrity: VerifierOptions {
            protection_method: ProtectionMethod::MERKLE_TREE,
            merkle_hash_tree_function: MerkleHashTreeFunction::BLAKE3,
            live_signature_algorithm: LiveSignatureAlgorithm::ED25519,
        },
        ..SwarmOptions::default()
    }
    .assign_defaults(&SwarmOptions::default_options())
}

/// The options of a session's swarm signing with `signing`:
/// [`ingress_options`] with its chunks per signature.
pub fn unit_ingress_options(signing: &UnitSigning) -> SwarmOptions {
    SwarmOptions {
        chunks_per_signature: signing.chunks_per_signature,
        ..ingress_options()
    }
}

/// The output a session holds before its swarm exists, past which its
/// signing settles on what it knows. Units held are audio: a video config
/// settles the signing before any video unit. 256 KiB is about two
/// seconds of 1 Mbit/s audio.
const MAX_HELD: usize = 256 * 1024;

/// What a stream's `onMetaData` says of it: the tracks it names, and
/// their bitrates in bits a second, each set when finite and positive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Declared {
    video: bool,
    audio: bool,
    video_bps: Option<u64>,
    audio_bps: Option<u64>,
}

impl Declared {
    fn of(m: &Metadata) -> Declared {
        // A float to integer cast saturates.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bps = |kbps: Option<f64>| {
            kbps.filter(|r| r.is_finite() && *r > 0.0)
                .map(|r| (r * 1000.0) as u64)
                .filter(|&b| b > 0)
        };
        Declared {
            video: [
                m.width,
                m.height,
                m.framerate,
                m.videocodecid,
                m.videodatarate,
            ]
            .iter()
            .any(Option::is_some),
            audio: [m.audiocodecid, m.audiosamplerate, m.audiodatarate]
                .iter()
                .any(Option::is_some),
            video_bps: bps(m.videodatarate),
            audio_bps: bps(m.audiodatarate),
        }
    }
}

/// What a session knows of its stream before its swarm exists, and when
/// that settles the swarm's signing; see the [module docs](self#media).
#[derive(Clone, Debug)]
struct SignChoice {
    /// [`IngressSettings::signing_wait`](crate::IngressSettings::signing_wait).
    wait: Duration,
    /// The latest metadata.
    metadata: Option<Declared>,
    /// A decoder config of the kind came.
    video: bool,
    audio: bool,
    /// When the first access unit came.
    first_unit: Option<Time>,
}

impl SignChoice {
    fn new(wait: Duration) -> SignChoice {
        SignChoice {
            wait,
            metadata: None,
            video: false,
            audio: false,
            first_unit: None,
        }
    }

    /// The stream's `onMetaData`, in place of any before.
    fn metadata(&mut self, m: &Metadata) {
        self.metadata = Some(Declared::of(m));
    }

    /// A decoder config of a track of `kind`; a text track's says nothing
    /// of the stream's bitrate.
    fn config(&mut self, kind: TrackKind) {
        match kind {
            TrackKind::Video => self.video = true,
            TrackKind::Audio => self.audio = true,
            TrackKind::Text => {}
        }
    }

    /// An access unit at `now`.
    fn unit(&mut self, now: Time) {
        self.first_unit.get_or_insert(now);
    }

    /// When the wait ends: `wait` after the first access unit.
    fn deadline(&self) -> Option<Time> {
        self.first_unit.map(|t| t + self.wait)
    }

    /// The swarm's signing once it is settled at `now`, with `held` bytes
    /// of output waiting for the swarm.
    fn settled(&self, now: Time, held: usize) -> Option<UnitSigning> {
        let named = self.metadata.is_some_and(|m| m.video || m.audio);
        let waited = self.deadline().is_some_and(|d| now >= d) || held >= MAX_HELD;
        (named || self.video || waited).then(|| self.choose())
    }

    /// The signing of what is known now.
    fn choose(&self) -> UnitSigning {
        let m = self.metadata.unwrap_or_default();
        let video = self.video || m.video;
        let audio = self.audio || m.audio;
        UnitSigning::for_stream(self.nominal(), audio && !video)
    }

    /// The stream's nominal bitrate as known now, if it has one; see the
    /// [module docs](self#media).
    fn nominal(&self) -> Option<u64> {
        let m = self.metadata.unwrap_or_default();
        let video = self.video || m.video;
        let audio = self.audio || m.audio;
        let sum = m
            .video_bps
            .unwrap_or(0)
            .saturating_add(m.audio_bps.unwrap_or(0));
        // Missing rates only add to the sum.
        let whole = (!video || m.video_bps.is_some()) && (!audio || m.audio_bps.is_some());
        (whole || sum >= UnitSigning::SMALL_BELOW_BPS)
            .then_some(sum)
            .filter(|&n| n > 0)
    }
}

/// The RTMP server's config under `s`.
pub(crate) fn rtmp_config(s: &RtmpSettings) -> RtmpConfig {
    RtmpConfig {
        max_media_message: s.max_media_message,
        max_command_message: s.max_command_message,
        max_partial_bytes: s.max_partial_bytes,
        max_chunk_streams: s.max_chunk_streams,
        max_output: s.max_queued_output,
        chunk_size: s.chunk_size,
        window_ack_size: s.window_ack_size,
        peer_bandwidth: s.peer_bandwidth,
    }
}

/// The host of a `host:port` peer address, an IPv6 address without its
/// brackets: the remote address the per-address cap counts. An address
/// without a port is its own host.
pub(crate) fn peer_host(peer: &str) -> &str {
    let host = match peer.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => host,
        _ => return peer,
    };
    match host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        Some(h) => h,
        // A bare IPv6 address has no port.
        None if host.contains(':') => peer,
        None => host,
    }
}

/// What the RTMP ingress has done.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IngressMetrics {
    /// Listener binds that succeeded.
    pub listens: u64,
    /// Binds that failed, and bound listeners that failed or ended.
    pub listen_errors: u64,
    /// Connections taken.
    pub connections: u64,
    /// Connections closed as they came, over a cap.
    pub rejected_connections: u64,
    /// Publishes refused.
    pub refused: u64,
    /// Sessions started.
    pub sessions: u64,
    /// Sessions a later publish to their channel replaced.
    pub replaced: u64,
    /// Records written, and the stream bytes written: the records, the
    /// frame headers and padding, and the zeros that pad an integrity unit
    /// signed early.
    pub segments: u64,
    pub bytes: u64,
    /// Integrity units signed before they filled:
    /// at the interval of the stream's [`UnitSigning`], and at a session's
    /// end.
    pub early_signs: u64,
    /// Units dropped: before their track's config, or before the first
    /// keyframe.
    pub dropped_units: u64,
    /// Metadata messages.
    pub metadata: u64,
    /// Connections a deadline ended.
    pub deadlines: u64,
    /// Sessions ended by media the ingress doesn't take: a refused codec,
    /// tag, config or unit.
    pub media_errors: u64,
    /// Swarm writes that failed.
    pub write_errors: u64,
    /// Session swarms created signing [`UnitSigning::SMALL`]'s 1-chunk
    /// units and [`UnitSigning::LARGE`]'s 8-chunk ones, and of those, the
    /// swarms of streams that stated no nominal bitrate, whose signing
    /// their tracks chose ([`UnitSigning::for_stream`]).
    pub small_unit_swarms: u64,
    pub large_unit_swarms: u64,
    pub unrated_swarms: u64,
    /// Thumbnails made, and those that failed or took too long.
    pub thumbnails: u64,
    pub thumbnail_failures: u64,
}

/// The ingress state of the control.
#[derive(Default)]
pub(crate) struct Ingress {
    /// The device's ingress config, once read.
    config: Option<VideoIngressConfig>,
    /// Wakes the listener task.
    listener: Option<Waker>,
    /// Connection and session ids.
    next_id: u64,
    /// The connections, by id. Bound:
    /// [`IngressSettings::max_connections`](crate::IngressSettings::max_connections).
    conns: BTreeMap<u64, Conn>,
    /// The sessions, by channel id. Bound:
    /// [`IngressSettings::max_sessions`](crate::IngressSettings::max_sessions).
    sessions: BTreeMap<u64, Session>,
    /// The transcodes running. Bound:
    /// [`TranscodeSettings::max_sessions`](crate::TranscodeSettings::max_sessions).
    transcodes: Rc<Cell<usize>>,
}

impl Ingress {
    fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
}

/// A connection the control holds.
struct Conn {
    /// The remote host.
    host: String,
    /// Its publish was accepted.
    published: bool,
    stop: Rc<Stop>,
}

/// A channel's session: what its connection's task and the control's
/// hooks share. Its listing, snippet, transfer and swarm slot are the
/// current session's: whoever holds the channel's entry tears them down.
struct Session {
    id: u64,
    /// The channel as last stored.
    channel: VideoChannel,
    key: Key,
    network_key: Vec<u8>,
    swarm_id: SwarmId,
    /// The snippet's start time, in unix seconds.
    start_time: i64,
    /// The publish's accept, and the latest record written, in unix
    /// seconds.
    created_at: i64,
    updated_at: i64,
    /// The publish was accepted.
    live: bool,
    /// The session's teardown started.
    ending: bool,
    keeper: Option<Keeper>,
    /// Ends the session's connection.
    stop: Rc<Stop>,
    /// The slots of the channel's rendition swarms, by transfer: the
    /// session's, and those of a session it replaced that it hasn't made
    /// again. Bound: [`MAX_RENDITIONS`](crate::codec::MAX_RENDITIONS)
    /// per session that held the channel.
    renditions: BTreeMap<TransferId, u64>,
    /// The renditions the snippet lists.
    listed: Vec<ListingRendition>,
    /// The thumbnail the snippet shows in place of the channel's: the
    /// session's latest, or that of the session it replaced.
    thumbnail: Option<ListingSnippetImage>,
}

/// A session's listing keeper, and the end of its run.
struct Keeper {
    keeper: ListingKeeper,
    done: Rc<Slot<()>>,
}

/// Ends a connection from outside its task.
#[derive(Default)]
struct Stop {
    reason: RefCell<Option<String>>,
    waker: RefCell<Option<Waker>>,
}

impl Stop {
    /// Has the connection end for `reason`, unless it was asked before.
    fn stop(&self, reason: &str) {
        {
            let mut r = self.reason.borrow_mut();
            if r.is_none() {
                *r = Some(reason.to_owned());
            }
        }
        let w = self.waker.take();
        if let Some(w) = w {
            w.wake();
        }
    }

    /// The reason, once asked.
    fn reason(&self) -> Option<String> {
        self.reason.borrow().clone()
    }

    /// Has `cx` woken at the next stop, asked before or not.
    fn register(&self, cx: &Context<'_>) {
        let prev = self.waker.replace(Some(cx.waker().clone()));
        drop(prev);
    }

    /// The reason, once asked; registers `cx` until then.
    fn poll(&self, cx: &Context<'_>) -> Option<String> {
        if let Some(r) = self.reason.borrow().clone() {
            return Some(r);
        }
        let prev = self.waker.replace(Some(cx.waker().clone()));
        drop(prev);
        None
    }
}

/// Why a publish was refused: its status code and description.
#[derive(Debug)]
struct Refusal {
    code: &'static str,
    description: String,
}

fn bad_name(description: &str) -> Refusal {
    Refusal {
        code: BAD_NAME,
        description: description.to_owned(),
    }
}

fn failed(description: &str) -> Refusal {
    Refusal {
        code: FAILED,
        description: description.to_owned(),
    }
}

/// The deadline a connection is under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Deadline {
    Handshake,
    Publish,
    Idle(Duration),
}

/// Why a connection ended.
#[derive(Debug)]
enum End {
    /// The publisher ended its publish.
    Unpublished,
    /// The server closed: a refused publish, or output past its bound.
    ServerClosed,
    /// The peer closed the connection.
    PeerClosed,
    /// Reading or writing the connection failed.
    Io(String),
    /// The peer broke the protocol or one of its bounds.
    Rtmp(RtmpError),
    Deadline(Deadline),
    /// The control ended it.
    Stopped(String),
    /// Media the ingress doesn't take.
    Media(String),
    /// Writing the swarm failed.
    Write(String),
}

impl End {
    /// Whether the publisher gets an error status with the reason.
    fn tells_peer(&self) -> bool {
        matches!(
            self,
            End::Deadline(_) | End::Stopped(_) | End::Media(_) | End::Write(_)
        )
    }

    /// Whether the connection can still take its last output.
    fn flushes(&self) -> bool {
        !matches!(self, End::PeerClosed | End::Io(_))
    }

    /// Whether the session ended on purpose: its publisher unpublished,
    /// or the control ended it. After any other end the encoder may
    /// connect again and resume the stream.
    fn deliberate(&self) -> bool {
        matches!(self, End::Unpublished | End::Stopped(_))
    }
}

impl fmt::Display for End {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            End::Unpublished => f.write_str("the publisher ended the stream"),
            End::ServerClosed => f.write_str("the RTMP session closed"),
            End::PeerClosed => f.write_str("the connection closed"),
            End::Io(e) => write!(f, "the connection failed: {e}"),
            End::Rtmp(e) => write!(f, "RTMP error: {e}"),
            End::Deadline(Deadline::Handshake) => f.write_str("the RTMP handshake took too long"),
            End::Deadline(Deadline::Publish) => f.write_str("no publish in time"),
            End::Deadline(Deadline::Idle(d)) => write!(f, "no media for {} s", d.as_secs()),
            End::Stopped(r) | End::Media(r) => f.write_str(r),
            End::Write(e) => write!(f, "writing the stream failed: {e}"),
        }
    }
}

/// What woke a connection's task.
enum Woke {
    Input(Vec<u8>),
    /// The session's transcode's next event, or its end.
    Transcoded(Option<Result<crate::codec::CodecEvent, String>>),
    Deadline,
    Stop(String),
    Closed,
    Failed(String),
}

/// Why the session can't take video of `config`'s codec: video other than
/// H.264 needs a node that transcodes it to H.264, the rendition every
/// viewer can play.
fn untranscoded(config: &DecoderConfig) -> Option<String> {
    let name = match config.params {
        CodecParams::Hevc { .. } => "HEVC (hvc1)",
        CodecParams::Av1 { .. } => "AV1 (av01)",
        _ => return None,
    };
    Some(format!(
        "{name} needs a node that transcodes it to H.264, and this one doesn't; send H.264 with AAC"
    ))
}

/// The live snippet of session `s`: its channel's, with its renditions,
/// the size of the channel swarm's picture and its thumbnail, if it has
/// one.
fn session_snippet(s: &Session) -> ListingSnippet {
    let mut snippet = live_snippet(&s.channel, s.start_time);
    if let Some(main) = s.listed.first() {
        snippet.video_width = main.width;
        snippet.video_height = main.height;
    }
    snippet.renditions = s.listed.clone();
    if let Some(t) = &s.thumbnail {
        snippet.thumbnail = Some(t.clone());
    }
    snippet
}

/// The live snippet of `c`: its snippet, starting at `start_time`, live.
fn live_snippet(c: &VideoChannel, start_time: i64) -> ListingSnippet {
    let mut s = c.directory_listing_snippet.clone().unwrap_or_default();
    s.start_time = start_time;
    s.live = true;
    s
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Shared<K, H, V, T> {
    fn count(&self, f: impl FnOnce(&mut IngressMetrics)) {
        f(&mut self.metrics.borrow_mut().ingress);
    }

    fn emit(&self, e: VideoEvent) {
        self.events.emit(vec![e]);
    }

    /// Starts the RTMP listener's task.
    pub(crate) fn start_ingress(self: &Rc<Self>) {
        self.spawn(self.clone().run_listener().boxed_local());
    }

    /// Takes the records' changes: the config, and the channels of
    /// sessions; see the [module docs](self).
    pub(crate) fn ingress_records_changed(&self, events: &[VideoEvent]) {
        for e in events {
            match e {
                VideoEvent::IngressConfigChanged(c) => self.ingress_config_changed(c),
                VideoEvent::ChannelChanged(c) => self.channel_changed(c),
                VideoEvent::ChannelDeleted(c) => self.stop_session(c.id, CHANNEL_DELETED),
                _ => {}
            }
        }
    }

    /// The listing keepers of the sessions.
    pub(crate) fn ingest_keepers(&self) -> Vec<ListingKeeper> {
        self.ingress
            .borrow()
            .sessions
            .values()
            .filter_map(|s| s.keeper.as_ref().map(|k| k.keeper.clone()))
            .collect()
    }

    /// The live sessions, by channel id.
    pub(crate) fn ingress_streams(&self) -> Vec<VideoIngressStream> {
        self.ingress
            .borrow()
            .sessions
            .values()
            .filter(|s| s.live && !s.ending)
            .map(|s| VideoIngressStream {
                id: s.id,
                channel_id: s.channel.id,
                created_at: s.created_at,
                updated_at: s.updated_at,
            })
            .collect()
    }

    fn ingress_config_changed(&self, c: &VideoIngressConfig) {
        let waker = {
            let mut ing = self.ingress.borrow_mut();
            ing.config = Some(c.clone());
            ing.listener.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
        if self.listen_addr().is_none() {
            // Sessions waiting for their encoder to resume hold no
            // connection.
            let stops: Vec<Rc<Stop>> = {
                let ing = self.ingress.borrow();
                ing.conns
                    .values()
                    .map(|c| c.stop.clone())
                    .chain(ing.sessions.values().map(|s| s.stop.clone()))
                    .collect()
            };
            for s in stops {
                s.stop(DISABLED);
            }
        }
    }

    fn channel_changed(&self, c: &VideoChannel) {
        let update = {
            let mut ing = self.ingress.borrow_mut();
            let Some(s) = ing.sessions.get_mut(&c.id) else {
                return;
            };
            let moved = channel_network_key(c).as_deref() != Some(s.network_key.as_slice())
                || c.key.as_ref() != Some(&s.key);
            if moved {
                Err(s.stop.clone())
            } else {
                let changed = c.directory_listing_snippet != s.channel.directory_listing_snippet;
                s.channel = c.clone();
                Ok((changed && s.live)
                    .then(|| (session_snippet(s), s.key.clone(), s.swarm_id.clone())))
            }
        };
        match update {
            Err(stop) => stop.stop(CHANNEL_MOVED),
            Ok(Some((snippet, key, swarm_id))) => {
                if let Err(e) = self.push_live_snippet(&swarm_id, &snippet, &key) {
                    tracing::warn!(error = %e, channel = c.id, "serving a changed live snippet failed");
                }
            }
            Ok(None) => {}
        }
    }

    fn stop_session(&self, channel_id: u64, reason: &str) {
        let stop = self
            .ingress
            .borrow()
            .sessions
            .get(&channel_id)
            .map(|s| s.stop.clone());
        if let Some(s) = stop {
            s.stop(reason);
        }
    }

    /// Signs `snippet` now with `key`, the key of swarm `swarm_id`, and
    /// serves it for the swarm.
    fn push_live_snippet(
        &self,
        swarm_id: &SwarmId,
        snippet: &ListingSnippet,
        key: &Key,
    ) -> Result<(), VideoError> {
        let signed = SignedSnippet::sign(&Signer::new(key)?, snippet, self.timer.now());
        self.directory.push_snippet(swarm_id, signed)?;
        Ok(())
    }

    // --- listener -------------------------------------------------------------

    /// The address to listen on; see the [module docs](self).
    fn listen_addr(&self) -> Option<String> {
        if !self.video.rtmp_supported() {
            return None;
        }
        let ing = self.ingress.borrow();
        let c = ing.config.as_ref()?;
        (c.enabled && check_ingress_config(c).is_ok()).then(|| c.server_addr.clone())
    }

    /// Registers the listener task to wake at a config change; whether
    /// the address to listen on is other than `addr`.
    fn listen_addr_changed(&self, addr: &Option<String>, cx: &Context<'_>) -> bool {
        if self.listen_addr() != *addr {
            return true;
        }
        let prev = self
            .ingress
            .borrow_mut()
            .listener
            .replace(cx.waker().clone());
        drop(prev);
        false
    }

    /// Resolves once the address to listen on is other than `addr`.
    async fn addr_changed(&self, addr: &Option<String>) {
        poll_fn(|cx| {
            if self.listen_addr_changed(addr, cx) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
    }

    /// Keeps the listener bound to the address to listen on.
    async fn run_listener(self: Rc<Self>) {
        loop {
            let addr = self.listen_addr();
            let Some(a) = addr.clone() else {
                self.addr_changed(&addr).await;
                continue;
            };
            let listened = {
                let listen = pin!(self.video.listen(&a));
                let changed = pin!(self.addr_changed(&addr));
                match select(listen, changed).await {
                    futures_util::future::Either::Left((r, _)) => r,
                    futures_util::future::Either::Right(_) => continue,
                }
            };
            let error = match listened {
                Ok(listener) => {
                    self.count(|m| m.listens += 1);
                    tracing::info!(addr = %a, "RTMP ingress listening");
                    match self.accept(listener, &addr).await {
                        Some(e) => e,
                        None => continue,
                    }
                }
                Err(e) => e.to_string(),
            };
            self.count(|m| m.listen_errors += 1);
            tracing::warn!(addr = %a, %error, "RTMP ingress listener failed");
            self.emit(VideoEvent::IngressListenFailed {
                addr: a.clone(),
                error,
            });
            let retry = self.timer.now() + self.config.ingress.bind_retry;
            let sleep = pin!(self.timer.sleep_until(retry));
            let changed = pin!(self.addr_changed(&addr));
            select(sleep, changed).await;
        }
    }

    /// Takes the listener's connections until the address to listen on
    /// changes (`None`), or the listener fails or ends (why).
    async fn accept(
        self: &Rc<Self>,
        mut listener: V::Listener,
        addr: &Option<String>,
    ) -> Option<String> {
        loop {
            let next = poll_fn(|cx| {
                if self.listen_addr_changed(addr, cx) {
                    return Poll::Ready(None);
                }
                listener.poll_next_unpin(cx).map(Some)
            })
            .await;
            match next {
                None => return None,
                Some(Some(Ok(a))) => self.admit(a),
                Some(Some(Err(e))) => return Some(e.to_string()),
                Some(None) => return Some("the listener ended".to_owned()),
            }
        }
    }

    /// Serves connection `a`, or closes it over a cap.
    fn admit(self: &Rc<Self>, a: Accepted<V::Reader, V::Writer>) {
        let s = &self.config.ingress;
        let host = peer_host(&a.peer).to_owned();
        let admitted = {
            let mut ing = self.ingress.borrow_mut();
            let unpublished = ing
                .conns
                .values()
                .filter(|c| !c.published && c.host == host)
                .count();
            if ing.conns.len() >= s.max_connections || unpublished >= s.max_unpublished_per_address
            {
                None
            } else {
                let id = ing.next_id();
                let stop = Rc::new(Stop::default());
                ing.conns.insert(
                    id,
                    Conn {
                        host,
                        published: false,
                        stop: stop.clone(),
                    },
                );
                Some((id, stop))
            }
        };
        let Some((id, stop)) = admitted else {
            self.count(|m| m.rejected_connections += 1);
            tracing::debug!(peer = %a.peer, "an RTMP connection over a cap closed");
            return;
        };
        self.count(|m| m.connections += 1);
        let mut rng = self.rng.borrow_mut();
        let server = RtmpServer::new(&rtmp_config(&s.rtmp), &mut rng);
        drop(rng);
        let conn = Connection {
            shared: self.clone(),
            id,
            stop,
            accepted: self.timer.now(),
            server,
            reader: a.reader,
            writer: a.writer,
            input: Vec::new(),
            pos: 0,
            scratch: vec![0; WRITE_CHUNK].into_boxed_slice(),
            unsent: None,
            unflushed: false,
            live: None,
        };
        self.spawn(conn.run().boxed_local());
    }

    // --- sessions -------------------------------------------------------------

    fn is_current(&self, channel_id: u64, session: u64) -> bool {
        self.ingress
            .borrow()
            .sessions
            .get(&channel_id)
            .is_some_and(|s| s.id == session)
    }

    /// Checks a publish of `app` and `name` and starts its session; see
    /// the [module docs](self#publishing).
    async fn start_session(
        self: &Rc<Self>,
        conn: u64,
        stop: &Rc<Stop>,
        app: &str,
        name: &str,
    ) -> Result<Live<K, H, V, T>, Refusal> {
        if app != LIVE_APP {
            return Err(bad_name("the app must be \"live\""));
        }
        if parse_stream_key(name).is_err() {
            return Err(bad_name("invalid stream key"));
        }
        let found = async {
            let mut tx = self.store.read().await?;
            channel_by_stream_key(&mut tx, name).await
        }
        .await;
        let channel = match found {
            Ok(Some(c)) => c,
            Ok(None) => return Err(bad_name("unknown stream key")),
            Err(e) => {
                tracing::warn!(error = %e, "reading a stream key's channel failed");
                return Err(failed("the stream key couldn't be checked"));
            }
        };
        let key = channel
            .key
            .clone()
            .filter(|k| !k.public.is_empty())
            .ok_or_else(|| failed("the channel has no key"))?;
        let network_key =
            channel_network_key(&channel).ok_or_else(|| failed("the channel has no network"))?;
        if self
            .network
            .network(&network_key)
            .and_then(|n| n.net)
            .is_none()
        {
            return Err(failed("the channel's network is not joined"));
        }

        let swarm_id = SwarmId::new(&key.public);
        let session = self.claim_channel(stop, &channel, &key, &network_key, &swarm_id)?;
        let mut publication = Publication {
            shared: self.clone(),
            channel_id: channel.id,
            session,
            t: TransferId::new(&swarm_id, b""),
            swarm_id,
            slot: None,
            armed: true,
        };
        self.take_ingest_slot(&mut publication, &network_key).await;

        let pushed = {
            let ing = self.ingress.borrow();
            ing.sessions
                .get(&channel.id)
                .filter(|s| s.id == session)
                .map(|s| (session_snippet(s), s.key.clone()))
        };
        let Some((snippet, key)) = pushed else {
            return Err(failed(REPLACED));
        };
        if let Err(e) = self.push_live_snippet(&publication.swarm_id, &snippet, &key) {
            tracing::warn!(error = %e, channel = channel.id, "serving a live snippet failed");
            return Err(failed("the stream's snippet couldn't be served"));
        }

        let now = self.timer.now();
        {
            let mut ing = self.ingress.borrow_mut();
            if let Some(s) = ing.sessions.get_mut(&channel.id) {
                s.live = true;
                s.created_at = now.unix();
                s.updated_at = now.unix();
            }
            if let Some(c) = ing.conns.get_mut(&conn) {
                c.published = true;
            }
        }
        self.count(|m| m.sessions += 1);
        tracing::info!(channel = channel.id, session, "RTMP stream live");
        self.emit(VideoEvent::IngressStarted {
            channel_id: channel.id,
        });
        Ok(Live {
            publication,
            source: FlvSource::new(),
            // invariant: the ingest swarm's chunk size is a valid frame size.
            #[allow(clippy::expect_used)]
            writer: UnitWriter::new(ingress_options().chunk_size)
                .expect("the ingest chunk size is a frame size"),
            sign: Sign::Choosing(SignChoice::new(self.config.ingress.signing_wait)),
            counted: UnitWriterMetrics::default(),
            swarm: None,
            mime: None,
            last_media: now,
            dropped: 0,
            video_codec: None,
            main_from_source: true,
            frame_rate: 0.0,
            main_bitrate: None,
            listed_main: None,
            renditions: None,
            thumbnails: self
                .video
                .codec()
                .map(|_| thumbnails::Thumbnails::new(&self.config.ingress.thumbnails)),
            text_config: None,
        })
    }

    /// Makes a new session the channel's, in place of any it has: its
    /// id. The session it replaces ends, leaving its listing keeper to
    /// the new one if they list on one network.
    fn claim_channel(
        &self,
        stop: &Rc<Stop>,
        channel: &VideoChannel,
        key: &Key,
        network_key: &[u8],
        swarm_id: &SwarmId,
    ) -> Result<u64, Refusal> {
        let now = self.timer.now().unix();
        let (id, replaced) = {
            let mut ing = self.ingress.borrow_mut();
            let replacing = ing.sessions.contains_key(&channel.id);
            if !replacing && ing.sessions.len() >= self.config.ingress.max_sessions {
                return Err(failed("too many streams"));
            }
            let id = ing.next_id();
            let mut old = ing.sessions.remove(&channel.id);
            let keeper = old.as_mut().and_then(|o| {
                let k = o.keeper.take()?;
                if o.network_key == network_key {
                    return Some(k);
                }
                k.keeper.stop();
                None
            });
            let renditions = old
                .as_mut()
                .map(|o| std::mem::take(&mut o.renditions))
                .unwrap_or_default();
            let thumbnail = old.as_mut().and_then(|o| o.thumbnail.take());
            ing.sessions.insert(
                channel.id,
                Session {
                    id,
                    channel: channel.clone(),
                    key: key.clone(),
                    network_key: network_key.to_vec(),
                    swarm_id: swarm_id.clone(),
                    start_time: now,
                    created_at: now,
                    updated_at: now,
                    live: false,
                    ending: false,
                    keeper,
                    stop: stop.clone(),
                    renditions,
                    listed: Vec::new(),
                    thumbnail,
                },
            );
            (id, old.map(|o| o.stop))
        };
        if let Some(old) = replaced {
            self.count(|m| m.replaced += 1);
            old.stop(REPLACED);
        }
        Ok(id)
    }

    /// Takes the slot of `p`'s channel among the node's video swarms for
    /// the session's swarm to come, if there is one: the slot of a session
    /// it replaced or of streams of the channel. The slot's swarm and its
    /// transfer go.
    async fn take_ingest_slot(&self, p: &mut Publication<K, H, V, T>, network_key: &[u8]) {
        let t = p.t;
        let taken = self.with_swarms(|r| {
            r.replace(
                &t,
                p.swarm_id.clone(),
                ingress_options(),
                SwarmKind::Ingest,
                BTreeSet::from([network_key.to_vec()]),
            )
        });
        if let Some((slot, old)) = taken {
            p.slot = Some(slot);
            self.transfers.remove(t);
            if let Some(old) = old {
                self.remove_host_swarm(old).await;
            }
        }
    }

    /// Creates the swarm of `p`'s session, of `options`, and makes it the
    /// one of its slot among the node's video swarms: the slot it took
    /// ([`take_ingest_slot`](Self::take_ingest_slot)), which takes
    /// `options`, the slot of streams of the channel that appeared since,
    /// or a new one. Adds and publishes its transfer. Returns its writer.
    async fn create_ingest_swarm(
        &self,
        p: &mut Publication<K, H, V, T>,
        options: SwarmOptions,
    ) -> Result<<H as SwarmHost>::Writer, Refusal> {
        let entry = {
            let ing = self.ingress.borrow();
            ing.sessions
                .get(&p.channel_id)
                .filter(|s| s.id == p.session)
                .map(|s| (s.key.clone(), s.network_key.clone()))
        };
        let Some((key, network_key)) = entry else {
            return Err(failed(REPLACED));
        };
        let networks = BTreeSet::from([network_key]);
        let t = p.t;
        let created = self
            .host
            .create_swarm(WriterOptions {
                swarm_options: options.clone(),
                key,
            })
            .await;
        let (swarm, id, writer) = match created {
            Ok(c) => c,
            Err(e) => {
                self.host_failed("creating an ingest swarm", &e);
                return Err(failed("the stream's swarm couldn't be created"));
            }
        };
        if id != p.swarm_id || !self.is_current(p.channel_id, p.session) {
            self.remove_host_swarm(swarm).await;
            return Err(failed(REPLACED));
        }
        // A slot that appeared while the swarm was created is a stream's.
        let stale = self.with_swarms(|r| {
            if let Some(slot) = p.slot
                && let Some(e) = r
                    .get_mut(&t)
                    .filter(|e| e.slot == slot && e.swarm.is_none())
            {
                e.options = options.clone();
                if r.set_swarm(&t, slot, swarm) == Added::Kept {
                    return None;
                }
            }
            match r.replace(
                &t,
                id.clone(),
                options.clone(),
                SwarmKind::Ingest,
                networks.clone(),
            ) {
                Some((slot, old)) => {
                    p.slot = Some(slot);
                    r.set_swarm(&t, slot, swarm);
                    old
                }
                None => {
                    let claim = r.insert(
                        t,
                        Some(swarm),
                        id.clone(),
                        options,
                        SwarmKind::Ingest,
                        networks,
                        0,
                    );
                    p.slot = Some(claim.slot);
                    None
                }
            }
        });
        if stale.is_some() {
            self.transfers.remove(t);
        }
        self.transfers.add(swarm, &id, Vec::new());
        self.publish_joined(&t);
        if let Some(old) = stale {
            self.remove_host_swarm(old).await;
        }
        Ok(writer)
    }

    /// A record of `p`'s session was written at `now`; `listing`, if
    /// set, is the listing to keep for it.
    fn records_written(
        self: &Rc<Self>,
        p: &Publication<K, H, V, T>,
        now: Time,
        listing: Option<Listing>,
    ) {
        let (keeper, start) = {
            let mut ing = self.ingress.borrow_mut();
            let Some(s) = ing
                .sessions
                .get_mut(&p.channel_id)
                .filter(|s| s.id == p.session)
            else {
                return;
            };
            s.updated_at = now.unix();
            let Some(listing) = listing else {
                return;
            };
            match &s.keeper {
                Some(k) => ((k.keeper.clone(), listing), None),
                None => {
                    let keeper = ListingKeeper::new(s.network_key.clone(), listing.clone());
                    let done = Slot::new();
                    s.keeper = Some(Keeper {
                        keeper: keeper.clone(),
                        done: done.clone(),
                    });
                    ((keeper.clone(), listing), Some((keeper, done)))
                }
            }
        };
        let (keeper, listing) = keeper;
        keeper.set_listing(listing);
        if let Some((keeper, done)) = start {
            let directory = self.directory.clone();
            let config = self.config.listing.keeper_config();
            self.spawn(
                async move {
                    keeper.run(&directory, config, |_| {}).await;
                    done.set(());
                }
                .boxed_local(),
            );
        }
    }

    /// Marks `p`'s session as ending, if it is its channel's: its keeper,
    /// for the caller to stop.
    fn session_ending(&self, p: &Publication<K, H, V, T>) -> Option<Keeper> {
        let mut ing = self.ingress.borrow_mut();
        let s = ing
            .sessions
            .get_mut(&p.channel_id)
            .filter(|s| s.id == p.session)?;
        s.ending = true;
        s.keeper.take()
    }

    /// Takes `p`'s publication out of the node, if its session is its
    /// channel's: the session, its keeper (stopped), snippet, transfer and
    /// slot. Returns the slot's swarm, for the caller to remove, and the
    /// slots of the channel's rendition swarms, for the caller to take
    /// out ([`remove_renditions`](Self::remove_renditions)).
    fn take_publication(
        &self,
        p: &Publication<K, H, V, T>,
    ) -> (Option<Swarm<H>>, Vec<(TransferId, u64)>) {
        let session = {
            let mut ing = self.ingress.borrow_mut();
            if !ing
                .sessions
                .get(&p.channel_id)
                .is_some_and(|s| s.id == p.session)
            {
                return (None, Vec::new());
            }
            ing.sessions.remove(&p.channel_id)
        };
        let Some(session) = session else {
            return (None, Vec::new());
        };
        if let Some(k) = &session.keeper {
            k.keeper.stop();
        }
        let renditions = session.renditions.iter().map(|(t, s)| (*t, *s)).collect();
        drop(session);
        self.directory.delete_snippet(&p.swarm_id);
        let Some(slot) = p.slot else {
            return (None, renditions);
        };
        self.transfers.remove(p.t);
        let swarm = self
            .with_swarms(|r| r.remove_slot(&p.t, slot))
            .and_then(|e| e.swarm);
        (swarm, renditions)
    }
}

/// What a session put on the node: its entry, listing, snippet, transfer
/// and swarm slot. Dropped without [`end`](Self::end), it takes them down
/// at once and removes the swarm in a task.
struct Publication<K: Kv, H: Host, V: VideoHost, T: Timer> {
    shared: Rc<Shared<K, H, V, T>>,
    channel_id: u64,
    session: u64,
    t: TransferId,
    swarm_id: SwarmId,
    /// The swarm slot, once the session took or made it.
    slot: Option<u64>,
    /// The teardown is the drop's to do.
    armed: bool,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Publication<K, H, V, T> {
    fn is_current(&self) -> bool {
        self.shared.is_current(self.channel_id, self.session)
    }

    /// Takes the publication down: steps 2 to 6 of the
    /// [teardown](self#teardown). The swarm stays until `linger`, if set,
    /// unless a new session takes the channel, which `stop`, the
    /// session's, is woken for.
    async fn end(mut self, stop: &Stop, linger: Option<Time>) {
        let s = self.shared.clone();
        if let Some(k) = s.session_ending(&self) {
            k.keeper.stop();
            let deadline = s.timer.now() + s.config.listing.unpublish_timeout;
            let done = pin!(k.done.take());
            let timeout = pin!(s.timer.sleep_until(deadline));
            select(done, timeout).await;
        }
        if let Some(until) = linger {
            let sleep = pin!(s.timer.sleep_until(until));
            let replaced = pin!(poll_fn(|cx| {
                if !self.is_current() {
                    return Poll::Ready(());
                }
                stop.register(cx);
                Poll::Pending
            }));
            select(sleep, replaced).await;
        }
        self.armed = false;
        let (swarm, renditions) = s.take_publication(&self);
        if let Some(swarm) = swarm {
            s.remove_host_swarm(swarm).await;
        }
        s.remove_renditions(renditions).await;
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Drop for Publication<K, H, V, T> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let (swarm, renditions) = self.shared.take_publication(self);
        if swarm.is_some() || !renditions.is_empty() {
            let s = self.shared.clone();
            self.shared.spawn(
                async move {
                    if let Some(swarm) = swarm {
                        s.remove_host_swarm(swarm).await;
                    }
                    s.remove_renditions(renditions).await;
                }
                .boxed_local(),
            );
        }
    }
}

/// A session's early signs.
enum Sign {
    /// The swarm's signing isn't settled, so the swarm doesn't exist yet.
    Choosing(SignChoice),
    /// The swarm signs as its signing has it.
    Timer(SignTimer),
}

/// A session's swarm, once it is created.
struct IngestSwarm<W> {
    writer: W,
    uri: String,
}

/// A publishing session's media path.
struct Live<K: Kv, H: Host, V: VideoHost, T: Timer> {
    publication: Publication<K, H, V, T>,
    source: FlvSource,
    /// The unit-format writer, signed early by `sign`.
    writer: UnitWriter,
    sign: Sign,
    /// What the metrics hold of the writer's.
    counted: UnitWriterMetrics,
    /// Once its signing settles.
    swarm: Option<IngestSwarm<<H as SwarmHost>::Writer>>,
    /// The MIME type listed.
    mime: Option<String>,
    last_media: Time,
    /// Units dropped so far, as counted.
    dropped: u64,
    /// The video codec the stream was planned for, once its first video
    /// config came.
    video_codec: Option<u8>,
    /// The channel's swarm carries the stream as it came; otherwise, the
    /// transcode's H.264.
    main_from_source: bool,
    /// The stream's frames a second, as its metadata states them; 0 when
    /// unknown.
    frame_rate: f64,
    /// The channel swarm's nominal bitrate, for its snippet entry.
    main_bitrate: Option<u64>,
    /// The snippet entry of the channel's swarm, once listed.
    listed_main: Option<ListingRendition>,
    renditions: Option<renditions::Renditions<V::Codec, <H as SwarmHost>::Writer>>,
    /// Where the node has a codec stage.
    thumbnails: Option<thumbnails::Thumbnails>,
    /// The text track's config, once declared.
    text_config: Option<DecoderConfig>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Live<K, H, V, T> {
    /// Takes an audio or video message's body.
    fn media(&mut self, video: bool, ts: u32, body: &[u8], now: Time) -> Result<(), End> {
        self.last_media = now;
        let event = if video {
            self.source.video(ts, body, now)
        } else {
            self.source.audio(ts, body, now)
        };
        let media = |e: &dyn std::fmt::Display| End::Media(e.to_string());
        let res = match event {
            Err(e) => Err(media(&e)),
            Ok(None) => Ok(()),
            Ok(Some(SourceEvent::Config(c))) => {
                self.plan_renditions(&c)?;
                if let Some(r) = &mut self.renditions {
                    r.config(&c);
                }
                if let Some(t) = &mut self.thumbnails {
                    t.config(&c);
                }
                let res = if self.main_from_source {
                    self.writer.set_config(&c).map_err(|e| media(&e))
                } else {
                    Ok(())
                };
                if let Sign::Choosing(choice) = &mut self.sign {
                    choice.config(c.kind);
                }
                res
            }
            Ok(Some(SourceEvent::Unit(u))) => {
                if let Sign::Choosing(choice) = &mut self.sign {
                    choice.unit(now);
                }
                if let Some(r) = &mut self.renditions {
                    let s = self.publication.shared.clone();
                    r.unit(&u, now, &mut s.metrics.borrow_mut().transcode);
                }
                self.thumbnail(&u, now);
                if self.main_from_source {
                    self.writer.push(&u).map_err(|e| media(&e))
                } else {
                    Ok(())
                }
            }
        };
        let dropped =
            self.source.metrics().before_config + self.writer.metrics().dropped_before_start;
        let new = dropped.saturating_sub(self.dropped);
        if new > 0 {
            self.dropped = dropped;
            self.publication.shared.count(|m| m.dropped_units += new);
        }
        res
    }

    /// Takes the stream's `onMetaData`.
    fn metadata(&mut self, m: &Metadata) {
        if let Some(rate) = m.framerate.filter(|r| r.is_finite() && *r > 0.0) {
            self.frame_rate = rate;
        }
        if let Sign::Choosing(choice) = &mut self.sign {
            choice.metadata(m);
        }
        if m.datastream {
            self.declare_text();
        }
    }

    /// Declares the stream's text track, once, to the channel's swarm and
    /// every rendition.
    fn declare_text(&mut self) {
        let Some(c) = self.source.text_config() else {
            return;
        };
        if let Err(e) = self.writer.set_config(&c) {
            tracing::debug!(error = %e, "the channel's writer refused the text track");
        }
        if let Some(r) = &mut self.renditions {
            r.text_config(&c);
        }
        self.text_config = Some(c);
    }

    /// Takes an `onTextData` message's text sent at `ts`, received at
    /// `now`: a cue of the text track, which the channel's swarm and every
    /// rendition carry. Cues aren't media: they don't keep the session
    /// from idling.
    fn text(&mut self, ts: u32, text: &str, now: Time) {
        self.declare_text();
        let u = self.source.text(ts, text, now);
        if let Err(e) = self.writer.push(&u) {
            tracing::debug!(error = %e, "the channel's writer refused a cue");
        }
        if let Some(r) = &mut self.renditions {
            r.text_unit(&u);
        }
    }

    /// Creates the session's swarm once its signing settles at `now`; see
    /// the [module docs](self#media).
    async fn settle(&mut self, now: Time) -> Result<(), End> {
        let Sign::Choosing(choice) = &self.sign else {
            return Ok(());
        };
        let Some(signing) = choice.settled(now, self.writer.output().len()) else {
            return Ok(());
        };
        let unrated = choice.nominal().is_none();
        let options = unit_ingress_options(&signing);
        let uri = Uri::new(self.publication.swarm_id.clone(), options.uri_options()).to_string();
        let s = self.publication.shared.clone();
        let created = s.create_ingest_swarm(&mut self.publication, options).await;
        let writer = created.map_err(|r| End::Stopped(r.description))?;
        self.sign = Sign::Timer(signing.timer(self.writer.frame_size()));
        self.swarm = Some(IngestSwarm { writer, uri });
        s.count(|m| {
            if signing == UnitSigning::SMALL {
                m.small_unit_swarms += 1;
            } else if signing == UnitSigning::LARGE {
                m.large_unit_swarms += 1;
            }
            if unrated {
                m.unrated_swarms += 1;
            }
        });
        tracing::debug!(
            channel = self.publication.channel_id,
            chunks_per_signature = signing.chunks_per_signature,
            interval_ms = signing.interval.as_millis(),
            "the stream's signing settled"
        );
        Ok(())
    }

    /// Writes the stream's final bytes into the swarm at `now`, once it
    /// exists.
    async fn write_output(&mut self, now: Time) -> Result<(), End> {
        let Some(swarm) = &mut self.swarm else {
            return Ok(());
        };
        let out = self.writer.take_output();
        let res = if out.is_empty() {
            Ok(())
        } else {
            swarm.writer.write(&out).await
        };
        self.writer.restore(out);
        if let (Ok(()), Sign::Timer(t)) = (&res, &mut self.sign) {
            t.written(self.writer.offset(), now);
        }
        res.map_err(|e| {
            self.publication.shared.count(|m| m.write_errors += 1);
            End::Write(e.to_string())
        })
    }

    /// When the stream's partial integrity unit is due for signing.
    fn sign_due(&self) -> Option<Time> {
        match &self.sign {
            Sign::Timer(t) => t.due(),
            Sign::Choosing(_) => None,
        }
    }

    /// When the stream's wait for its signing ends.
    fn settle_due(&self) -> Option<Time> {
        match &self.sign {
            Sign::Choosing(c) => c.deadline(),
            Sign::Timer(_) => None,
        }
    }

    /// The stream has a partial integrity unit.
    fn partial(&self) -> bool {
        match &self.sign {
            Sign::Timer(t) => t.is_partial(),
            Sign::Choosing(_) => false,
        }
    }

    /// Has the swarm sign the stream's partial integrity unit: ends the
    /// frame in progress, writes it, flushes the swarm writer and goes on
    /// where the next unit starts.
    async fn sign(&mut self, now: Time) -> Result<(), End> {
        self.writer.end_frame();
        self.write_output(now).await?;
        let Some(swarm) = &mut self.swarm else {
            return Ok(());
        };
        let flushed = swarm.writer.flush().await;
        let next = flushed.map_err(|e| {
            self.publication.shared.count(|m| m.write_errors += 1);
            End::Write(e.to_string())
        })?;
        let Sign::Timer(sign) = &mut self.sign else {
            return Ok(());
        };
        let padding = next.saturating_sub(self.writer.offset());
        self.writer
            .continue_at(next)
            .map_err(|e| End::Write(e.to_string()))?;
        sign.flushed(next);
        self.publication.shared.count(|m| {
            m.early_signs += 1;
            m.bytes += padding;
        });
        Ok(())
    }

    /// Does what is due at `now`: creates the session's swarm once its
    /// wait ends, writing what the session held, and has a partial
    /// integrity unit signed at its interval.
    async fn timer(&mut self, now: Time) -> Result<(), End> {
        self.renditions_timer(now)?;
        self.settle(now).await?;
        self.write_output(now).await?;
        if self.sign_due().is_some_and(|t| now >= t) {
            self.sign(now).await?;
        }
        self.take_records(now, true);
        self.settle_renditions(now).await;
        Ok(())
    }

    /// Counts the records written, and keeps their listing if `list`.
    /// Records wait in the session until its swarm exists.
    fn take_records(&mut self, now: Time, list: bool) {
        let Some(swarm) = &self.swarm else {
            return;
        };
        let m = self.writer.metrics();
        let records = m.records.saturating_sub(self.counted.records);
        let bytes = m.bytes.saturating_sub(self.counted.bytes);
        self.counted = m;
        self.publication.shared.count(|i| {
            i.segments += records;
            i.bytes += bytes;
        });
        if records == 0 {
            return;
        }
        let mime = units_mime(self.writer.configs());
        let listing = (list && self.mime.as_ref() != Some(&mime)).then(|| {
            self.mime = Some(mime.clone());
            Listing {
                content: Some(Content::Media(Media {
                    mime_type: mime,
                    swarm_uri: swarm.uri.clone(),
                })),
            }
        });
        let s = self.publication.shared.clone();
        s.records_written(&self.publication, now, listing);
    }

    /// Writes what the writer holds, and has the swarm sign the stream's
    /// last partial unit: step 1 of the [teardown](self#teardown). With
    /// `end`, the stream's END goes first. When the END was written into
    /// the swarm and signed.
    async fn finish(&mut self, now: Time, end: bool) -> Option<Time> {
        self.finish_renditions(end).await;
        let finished = !end || self.writer.finish().is_ok();
        let written = finished && self.write_output(now).await.is_ok();
        if written {
            self.take_records(now, false);
        }
        if self.partial() {
            if self.sign(now).await.is_err() {
                return None;
            }
            self.take_records(now, false);
        }
        let ended = self.writer.metrics().ends > 0;
        (end && written && ended && self.swarm.is_some()).then_some(now)
    }

    /// Whether the session waits for its encoder to resume after a
    /// connection that ended by itself: a session that wrote records into
    /// its swarm, and no END.
    fn resumable(&self) -> bool {
        let m = self.writer.metrics();
        self.swarm.is_some() && m.records > 0 && m.ends == 0
    }

    /// Waits up to the resume grace, or until `stop`, the session's,
    /// stops it, for a new session to take the channel; then a session
    /// still its channel's writes its END. When the END was signed; see
    /// step 1 of the [teardown](self#teardown).
    async fn await_resume(&mut self, stop: &Stop) -> Option<Time> {
        let s = self.publication.shared.clone();
        let deadline = s.timer.now() + s.config.ingress.resume_grace;
        let sleep = pin!(s.timer.sleep_until(deadline));
        let stopped = pin!(poll_fn(|cx| match stop.poll(cx) {
            Some(_) => Poll::Ready(()),
            None => Poll::Pending,
        }));
        select(sleep, stopped).await;
        if !self.publication.is_current() {
            return None;
        }
        self.finish(s.timer.now(), true).await
    }
}

/// An RTMP connection's task.
struct Connection<K: Kv, H: Host, V: VideoHost, T: Timer> {
    shared: Rc<Shared<K, H, V, T>>,
    id: u64,
    stop: Rc<Stop>,
    accepted: Time,
    server: RtmpServer,
    reader: V::Reader,
    writer: V::Writer,
    /// The bytes read last, from `pos` on not yet taken by the server.
    input: Vec<u8>,
    pos: usize,
    /// Server output moved out for the writer. Bound: [`WRITE_CHUNK`].
    scratch: Box<[u8]>,
    /// Output the writer hasn't taken yet.
    unsent: Option<Vec<u8>>,
    /// Output the writer took and hasn't flushed.
    unflushed: bool,
    live: Option<Live<K, H, V, T>>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Drop for Connection<K, H, V, T> {
    fn drop(&mut self) {
        self.release();
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Connection<K, H, V, T> {
    /// Takes the connection out of the control's connections, so it no
    /// longer counts against their caps.
    fn release(&self) {
        let c = self.shared.ingress.borrow_mut().conns.remove(&self.id);
        drop(c);
    }

    async fn run(mut self) {
        let s = self.shared.clone();
        let mut armed = self.wake_at();
        let mut sleep = Box::pin(s.timer.sleep_until(armed));
        let end = loop {
            if let Err(end) = self.process().await {
                break end;
            }
            let at = self.wake_at();
            if at < armed {
                armed = at;
                sleep = Box::pin(s.timer.sleep_until(at));
            }
            match self.wait(sleep.as_mut()).await {
                Woke::Input(b) => {
                    self.input = b;
                    self.pos = 0;
                }
                Woke::Transcoded(e) => {
                    let now = s.timer.now();
                    if let Some(l) = &mut self.live {
                        if let Err(end) = l.transcoded(e, now) {
                            break end;
                        }
                        l.settle_renditions(now).await;
                        if let Err(end) = l.settle(now).await {
                            break end;
                        }
                        if let Err(end) = l.write_output(now).await {
                            break end;
                        }
                        l.take_records(now, true);
                    }
                }
                Woke::Deadline => {
                    let now = s.timer.now();
                    if let Some(l) = &mut self.live
                        && let Err(end) = l.timer(now).await
                    {
                        break end;
                    }
                    let (due, which) = self.due();
                    if now >= due {
                        break End::Deadline(which);
                    }
                    armed = self.wake_at();
                    sleep = Box::pin(s.timer.sleep_until(armed));
                }
                Woke::Stop(r) => break End::Stopped(r),
                Woke::Closed => break End::PeerClosed,
                Woke::Failed(e) => break End::Io(e),
            }
        };
        match end {
            End::Deadline(_) => s.count(|m| m.deadlines += 1),
            End::Media(_) => s.count(|m| m.media_errors += 1),
            _ => {}
        }
        tracing::debug!(conn = self.id, reason = %end, "RTMP connection ended");
        let mut live = self.live.take();
        let mut ended = None;
        if let Some(l) = &mut live
            && l.publication.is_current()
        {
            ended = l.finish(s.timer.now(), end.deliberate()).await;
        }
        self.close(&end).await;
        self.release();
        if let Some(mut live) = live {
            let channel_id = live.publication.channel_id;
            if !end.deliberate() && live.publication.is_current() && live.resumable() {
                ended = live.await_resume(&self.stop).await;
            }
            let linger = ended.map(|t| t + END_LINGER);
            live.publication.end(&self.stop, linger).await;
            tracing::info!(channel = channel_id, reason = %end, "RTMP stream ended");
            s.emit(VideoEvent::IngressEnded {
                channel_id,
                reason: end.to_string(),
            });
        }
    }

    /// When the connection's task wakes: at its deadline, when a
    /// stream's partial integrity unit is due for signing, or
    /// when its wait for its signing ends.
    fn wake_at(&self) -> Time {
        let (due, _) = self.due();
        let Some(l) = &self.live else {
            return due;
        };
        [l.sign_due(), l.settle_due(), l.renditions_due()]
            .into_iter()
            .flatten()
            .fold(due, Time::min)
    }

    /// The deadline the connection is under, and when it passes.
    fn due(&self) -> (Time, Deadline) {
        let s = &self.shared.config.ingress;
        if let Some(l) = &self.live {
            return (l.last_media + s.media_idle, Deadline::Idle(s.media_idle));
        }
        let publish = self.accepted + s.publish_deadline;
        let handshake = self.accepted + s.handshake_deadline;
        if self.server.phase() == Phase::Handshake && handshake < publish {
            return (handshake, Deadline::Handshake);
        }
        (publish, Deadline::Publish)
    }

    /// Has the server take the input read, acting on its events.
    async fn process(&mut self) -> Result<(), End> {
        while self.pos < self.input.len() {
            let rest = self.input.get(self.pos..).unwrap_or_default();
            let n = self.server.handle_input(rest).map_err(End::Rtmp)?;
            self.pos += n;
            let mut evented = false;
            loop {
                if let Some(r) = self.stop.reason() {
                    return Err(End::Stopped(r));
                }
                let now = self.shared.timer.now();
                let publish = match self.server.poll_event() {
                    None => break,
                    Some(RtmpEvent::Publish { app, name }) => Some((app, name)),
                    Some(RtmpEvent::Video { timestamp, data }) => {
                        if let Some(l) = &mut self.live {
                            l.media(true, timestamp, data, now)?;
                        }
                        None
                    }
                    Some(RtmpEvent::Audio { timestamp, data }) => {
                        if let Some(l) = &mut self.live {
                            l.media(false, timestamp, data, now)?;
                        }
                        None
                    }
                    Some(RtmpEvent::Metadata(m)) => {
                        self.shared.count(|c| c.metadata += 1);
                        if let Some(l) = &mut self.live {
                            l.metadata(&m);
                        }
                        None
                    }
                    Some(RtmpEvent::Text { timestamp, text }) => {
                        if let Some(l) = &mut self.live {
                            l.text(timestamp, &text, now);
                        }
                        None
                    }
                    Some(RtmpEvent::Unpublish) => return Err(End::Unpublished),
                };
                evented = true;
                if let Some((app, name)) = publish {
                    self.publish(&app, &name).await;
                }
                if let Some(l) = &mut self.live {
                    l.settle(now).await?;
                    l.write_output(now).await?;
                    l.take_records(now, true);
                    l.settle_renditions(now).await;
                }
            }
            if self.server.phase() == Phase::Closed {
                return Err(End::ServerClosed);
            }
            if n == 0 && !evented {
                break;
            }
        }
        Ok(())
    }

    /// Answers a publish: starts its session, or refuses it.
    async fn publish(&mut self, app: &str, name: &str) {
        let s = self.shared.clone();
        match s.start_session(self.id, &self.stop, app, name).await {
            Ok(live) => {
                self.server.accept_publish();
                self.live = Some(live);
            }
            Err(r) => {
                s.count(|m| m.refused += 1);
                tracing::info!(
                    conn = self.id,
                    code = r.code,
                    reason = %r.description,
                    "RTMP publish refused"
                );
                self.server.refuse(r.code, &r.description);
            }
        }
    }

    /// Waits for input, a stop or the deadline, writing the server's
    /// output meanwhile.
    async fn wait(&mut self, mut sleep: Pin<&mut T::Sleep>) -> Woke {
        poll_fn(|cx| {
            if let Some(r) = self.stop.poll(cx) {
                return Poll::Ready(Woke::Stop(r));
            }
            if let Poll::Ready(Err(e)) = self.poll_write(cx) {
                return Poll::Ready(Woke::Failed(e));
            }
            if sleep.as_mut().poll(cx).is_ready() {
                return Poll::Ready(Woke::Deadline);
            }
            if let Some(r) = self.live.as_mut().and_then(|l| l.renditions.as_mut())
                && let Poll::Ready(e) = r.poll_event(cx)
            {
                return Poll::Ready(Woke::Transcoded(e));
            }
            match Pin::new(&mut self.reader).poll_next(cx) {
                Poll::Ready(Some(Ok(b))) => Poll::Ready(Woke::Input(b)),
                Poll::Ready(Some(Err(e))) => Poll::Ready(Woke::Failed(e.to_string())),
                Poll::Ready(None) => Poll::Ready(Woke::Closed),
                Poll::Pending => Poll::Pending,
            }
        })
        .await
    }

    /// Moves the server's output into the writer; ready once all of it
    /// is flushed.
    fn poll_write(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), String>> {
        loop {
            if self.unsent.is_none() {
                let n = self.server.poll_transmit(&mut self.scratch);
                if n == 0 {
                    break;
                }
                self.unsent = Some(self.scratch.get(..n).unwrap_or_default().to_vec());
            }
            match Pin::new(&mut self.writer).poll_ready(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e.to_string())),
                Poll::Pending => return Poll::Pending,
            }
            if let Some(b) = self.unsent.take() {
                if let Err(e) = Pin::new(&mut self.writer).start_send(b) {
                    return Poll::Ready(Err(e.to_string()));
                }
                self.unflushed = true;
            }
        }
        if self.unflushed {
            match Pin::new(&mut self.writer).poll_flush(cx) {
                Poll::Ready(Ok(())) => self.unflushed = false,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e.to_string())),
                Poll::Pending => return Poll::Pending,
            }
        }
        Poll::Ready(Ok(()))
    }

    /// Tells the publisher why the connection ends, if it should hear,
    /// then hands over the last output, closes the sending side, and takes
    /// the publisher's input until it closes its side, all within the
    /// close timeout.
    async fn close(&mut self, end: &End) {
        if end.tells_peer() {
            self.server.refuse(FAILED, &end.to_string());
        }
        if !end.flushes() {
            return;
        }
        let s = self.shared.clone();
        let deadline = s.timer.now() + s.config.ingress.close_timeout;
        let mut sleep = pin!(s.timer.sleep_until(deadline));
        let mut shut = false;
        poll_fn(|cx| {
            if sleep.as_mut().poll(cx).is_ready() {
                return Poll::Ready(());
            }
            if !shut {
                match self.poll_write(cx) {
                    Poll::Ready(Ok(())) => {}
                    Poll::Ready(Err(_)) => return Poll::Ready(()),
                    Poll::Pending => return Poll::Pending,
                }
                match Pin::new(&mut self.writer).poll_close(cx) {
                    Poll::Ready(Ok(())) => shut = true,
                    Poll::Ready(Err(_)) => return Poll::Ready(()),
                    Poll::Pending => return Poll::Pending,
                }
            }
            // A connection dropped with input unread is reset, and a reset
            // can discard the last output before the publisher reads it.
            for _ in 0..CLOSE_DRAIN_READS {
                match Pin::new(&mut self.reader).poll_next(cx) {
                    Poll::Ready(Some(Ok(_))) => {}
                    Poll::Ready(Some(Err(_)) | None) => return Poll::Ready(()),
                    Poll::Pending => return Poll::Pending,
                }
            }
            cx.waker().wake_by_ref();
            Poll::Pending
        })
        .await;
    }
}

#[cfg(test)]
#[path = "ingress_test.rs"]
mod tests;
