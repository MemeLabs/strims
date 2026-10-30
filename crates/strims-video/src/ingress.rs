// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The RTMP ingress: a broadcaster's stream to one of the profile's
//! channels, written into the channel's live video swarm as fMP4 records
//! or in the unit format.
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
//!   so its id is the channel's public key: with [`ingress_options`]
//!   before an fMP4 publish is accepted, and with [`unit_ingress_options`]
//!   of its signing once a unit-format session's signing settles, after
//!   the accept (see [Media](#media)). Its transfer, under the empty salt,
//!   is published on the channel's network. A unit-format swarm that
//!   can't be created ends its session, the publisher hearing why.
//! - **The snippet.** The channel's snippet, with `start_time` the
//!   publish's unix time and `live` set, signed with the channel key, is
//!   served for the swarm. A changed channel record is signed and served
//!   again, with the same start time.
//! - **The listing.** Once the first record is written, a media listing
//!   of the stream's MIME type ([`fmp4_mime`] or [`units_mime`] of its
//!   decoder configs) and the swarm URI is kept on the directory of the
//!   channel's network. A record of other codecs lists the new MIME type
//!   in its place.
//!
//! A publish to a channel that streams replaces its session: the old
//! connection closes, and the new session takes over the old one's
//! listing, if on the same network, and its slot among the node's video
//! swarms. The slot's swarm goes with the publish, and the slot takes the
//! new session's swarm, with that session's options, once it is created,
//! so the node's streams of the channel go on after a discontinuity. A
//! unit-format session's signing is its own stream's: when the session it
//! replaced signed otherwise, its swarm URI, and so its listing's, is
//! another. A deleted channel, or one moved to another network, ends its
//! session.
//!
//! # Media
//!
//! Audio and video go through an [`FlvSource`] into the muxer of the
//! format [`IngressSettings::format`](crate::IngressSettings::format)
//! names. A unit's publish time is when the connection took its message
//! from the RTMP server, on the node's clock. A codec or tag the source
//! refuses, or a record over its bound, ends the session; the publisher
//! gets the reason in an error status. Metadata is counted, and a
//! unit-format session reads its bitrates and tracks.
//!
//! **fMP4**, the default: a [`Segmenter`] over a [`PackedSink`], cut under
//! the rule of
//! [`IngressSettings::min_segment_media_bytes`](crate::IngressSettings::min_segment_media_bytes)
//! and
//! [`IngressSettings::max_record_bytes`](crate::IngressSettings::max_record_bytes),
//! with a fragment per
//! [`IngressSettings::fragment_duration`](crate::IngressSettings::fragment_duration)
//! of media within each record, each with its first unit's publish time
//! in its `prft`. The packed stream's bytes go into the swarm as they
//! become final: a frame at a time, or up to a record's end. The swarm
//! writer is never flushed, which would pad a partial integrity unit with
//! zeros a reader takes for record bytes, so a stream's last partial unit
//! is never published.
//!
//! **Units**: a [`UnitWriter`] in frames of the swarm's chunk size, whose
//! records go into the swarm as each unit arrives. The swarm's signing, its
//! unit size and interval ([`UnitSigning`]), is the stream's, so the
//! session creates the swarm once it knows enough of the stream:
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
//! # Teardown
//!
//! A session ends when its publisher unpublishes or disconnects, at a
//! deadline, on an error, when a new publish replaces it, when the
//! ingress is disabled, when its channel is deleted or moved, and when
//! the control stops. Its end:
//!
//! 1. ends the record in progress and writes it, and has a unit-format
//!    stream's last partial integrity unit signed; a unit-format session
//!    without its swarm yet, never listed, writes nothing;
//! 2. stops the listing keeper, which unpublishes the listing, waiting at
//!    most the listing settings'
//!    [`unpublish_timeout`](crate::ListingSettings::unpublish_timeout);
//! 3. stops serving the snippet;
//! 4. removes the transfer;
//! 5. removes the swarm's slot, ending the node's streams of it;
//! 6. removes the swarm.
//!
//! The connection then takes its last output and closes. A replaced
//! session takes none of these steps: what they take down is the
//! replacing session's. A control that stops takes steps 2 to 5 at once,
//! without waiting for the unpublish, and leaves the swarm to the node.
//! [`VideoEvent::IngressEnded`] reports the end of a session that
//! started, with its reason.

use std::cell::RefCell;
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
use strims_proto::network::v1::directory::{Listing, ListingSnippet};
use strims_proto::types::Key;
use strims_proto::video::v1::{VideoChannel, VideoIngressConfig, VideoIngressStream};
use strims_rpc::Timer;
use strims_transfer::{SwarmHost, SwarmWrite, TransferId};

use crate::amf0::Metadata;
use crate::config::{IngestFormat, RtmpSettings, check_ingress_config};
use crate::control::{Shared, Slot, Swarm};
use crate::flv::{FlvSource, SourceEvent};
use crate::host::{Accepted, VideoHost};
use crate::media::{Muxer, TrackKind, fmp4_mime, units_mime};
use crate::record::PackedSink;
use crate::registry::{Added, SwarmKind};
use crate::rtmp::{Phase, RtmpConfig, RtmpError, RtmpEvent, RtmpServer};
use crate::segment::{CutRule, Segmenter};
use crate::units::{SignTimer, UnitSigning, UnitWriter, UnitWriterMetrics};
use crate::{VideoError, VideoEvent};

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

/// The options of an fMP4 session's swarm, and of a unit-format session's
/// signing [`UnitSigning::LARGE`]; its URI carries every one but the
/// window. 4 KiB chunks, 8 per signature (32 KiB integrity units), 16
/// streams, a 4 Ki-chunk (16 MiB) window, Merkle/BLAKE3/Ed25519. Below
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

/// The options of a unit-format session's swarm signing with `signing`:
/// [`ingress_options`] with its chunks per signature.
pub fn unit_ingress_options(signing: &UnitSigning) -> SwarmOptions {
    SwarmOptions {
        chunks_per_signature: signing.chunks_per_signature,
        ..ingress_options()
    }
}

/// The output a unit-format session holds before its swarm exists, past
/// which its signing settles on what it knows. Units held are audio: a
/// video config settles the signing before any video unit. 256 KiB is
/// about two seconds of 1 Mbit/s audio.
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

/// What a unit-format session knows of its stream before its swarm
/// exists, and when that settles the swarm's signing; see the
/// [module docs](self#media).
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

    /// A decoder config of a track of `kind`.
    fn config(&mut self, kind: TrackKind) {
        match kind {
            TrackKind::Video => self.video = true,
            TrackKind::Audio => self.audio = true,
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
        let sum = m
            .video_bps
            .unwrap_or(0)
            .saturating_add(m.audio_bps.unwrap_or(0));
        // Missing rates only add to the sum.
        let whole = (!video || m.video_bps.is_some()) && (!audio || m.audio_bps.is_some());
        let nominal = (whole || sum >= UnitSigning::SMALL_BELOW_BPS).then_some(sum);
        UnitSigning::for_stream(nominal, audio && !video)
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
    /// Records written, fMP4 segments or unit-format records, and the
    /// stream bytes written: an fMP4 record's prefix, init and padding,
    /// and the unit format's frame headers and padding, and the zeros that
    /// pad an integrity unit signed early.
    pub segments: u64,
    pub bytes: u64,
    /// Integrity units of unit-format streams signed before they filled:
    /// at the interval of the stream's [`UnitSigning`], and at a session's
    /// end.
    pub early_signs: u64,
    /// Units dropped: before their track's config, before the first
    /// keyframe, or of a changed config the record in progress doesn't
    /// declare.
    pub dropped_units: u64,
    /// Metadata messages.
    pub metadata: u64,
    /// Connections a deadline ended.
    pub deadlines: u64,
    /// Sessions ended by media the ingress doesn't take: a refused codec
    /// or tag, or a record over its bound.
    pub media_errors: u64,
    /// Swarm writes that failed.
    pub write_errors: u64,
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
    Deadline,
    Stop(String),
    Closed,
    Failed(String),
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
            let stops: Vec<Rc<Stop>> = self
                .ingress
                .borrow()
                .conns
                .values()
                .map(|c| c.stop.clone())
                .collect();
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
                Ok((changed && s.live).then(|| {
                    (
                        live_snippet(c, s.start_time),
                        s.key.clone(),
                        s.swarm_id.clone(),
                    )
                }))
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
        let format = self.config.ingress.format;
        let swarm = match format {
            IngestFormat::Fmp4 => {
                let options = ingress_options();
                let uri = Uri::new(publication.swarm_id.clone(), options.uri_options()).to_string();
                let writer = self.create_ingest_swarm(&mut publication, options).await?;
                Some(IngestSwarm { writer, uri })
            }
            IngestFormat::Units => None,
        };

        let pushed = {
            let ing = self.ingress.borrow();
            ing.sessions
                .get(&channel.id)
                .filter(|s| s.id == session)
                .map(|s| (live_snippet(&s.channel, s.start_time), s.key.clone()))
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
        let rule = CutRule {
            min_media_bytes: self.config.ingress.min_segment_media_bytes,
            max_record_bytes: self.config.ingress.max_record_bytes,
            fragment_duration: self.config.ingress.fragment_duration,
        };
        let media = match format {
            IngestFormat::Fmp4 => Mux::Fmp4(Segmenter::new(rule, PackedSink::new())),
            IngestFormat::Units => Mux::Units {
                // invariant: the ingest swarm's chunk size is a valid frame
                // size.
                #[allow(clippy::expect_used)]
                writer: UnitWriter::new(ingress_options().chunk_size)
                    .expect("the ingest chunk size is a frame size"),
                sign: Sign::Choosing(SignChoice::new(self.config.ingress.signing_wait)),
                counted: UnitWriterMetrics::default(),
            },
        };
        Ok(Live {
            publication,
            source: FlvSource::new(),
            media,
            swarm,
            mime: None,
            last_media: now,
            dropped: 0,
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
    fn segment_written(
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
    /// slot. Returns the slot's swarm, for the caller to remove.
    fn take_publication(&self, p: &Publication<K, H, V, T>) -> Option<Swarm<H>> {
        let session = {
            let mut ing = self.ingress.borrow_mut();
            if !ing
                .sessions
                .get(&p.channel_id)
                .is_some_and(|s| s.id == p.session)
            {
                return None;
            }
            ing.sessions.remove(&p.channel_id)
        }?;
        if let Some(k) = &session.keeper {
            k.keeper.stop();
        }
        drop(session);
        self.directory.delete_snippet(&p.swarm_id);
        let slot = p.slot?;
        self.transfers.remove(p.t);
        let e = self.with_swarms(|r| r.remove_slot(&p.t, slot))?;
        e.swarm
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
    /// [teardown](self#teardown).
    async fn end(mut self) {
        let s = self.shared.clone();
        if let Some(k) = s.session_ending(&self) {
            k.keeper.stop();
            let deadline = s.timer.now() + s.config.listing.unpublish_timeout;
            let done = pin!(k.done.take());
            let timeout = pin!(s.timer.sleep_until(deadline));
            select(done, timeout).await;
        }
        self.armed = false;
        if let Some(swarm) = s.take_publication(&self) {
            s.remove_host_swarm(swarm).await;
        }
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Drop for Publication<K, H, V, T> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Some(swarm) = self.shared.take_publication(self) {
            let s = self.shared.clone();
            self.shared
                .spawn(async move { s.remove_host_swarm(swarm).await }.boxed_local());
        }
    }
}

/// What a session writes its media as.
enum Mux {
    /// fMP4 segments in the packed layout.
    Fmp4(Segmenter<PackedSink>),
    /// The unit format, signed early by `sign`; `counted` is what the
    /// metrics hold of the writer's.
    Units {
        writer: UnitWriter,
        sign: Sign,
        counted: UnitWriterMetrics,
    },
}

/// A unit-format session's early signs.
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
    media: Mux,
    /// From the publish for fMP4, and once its signing settles for the
    /// unit format.
    swarm: Option<IngestSwarm<<H as SwarmHost>::Writer>>,
    /// The MIME type listed.
    mime: Option<String>,
    last_media: Time,
    /// Units dropped so far, as counted.
    dropped: u64,
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
        let res = match (event, &mut self.media) {
            (Err(e), _) => Err(media(&e)),
            (Ok(None), _) => Ok(()),
            (Ok(Some(SourceEvent::Config(c))), Mux::Fmp4(s)) => {
                s.set_config(&c).map_err(|e| media(&e))
            }
            (Ok(Some(SourceEvent::Config(c))), Mux::Units { writer, sign, .. }) => {
                let res = writer.set_config(&c).map_err(|e| media(&e));
                if let Sign::Choosing(choice) = sign {
                    choice.config(c.kind);
                }
                res
            }
            (Ok(Some(SourceEvent::Unit(u))), Mux::Fmp4(s)) => s.push(&u).map_err(|e| media(&e)),
            (Ok(Some(SourceEvent::Unit(u))), Mux::Units { writer, sign, .. }) => {
                if let Sign::Choosing(choice) = sign {
                    choice.unit(now);
                }
                writer.push(&u).map_err(|e| media(&e))
            }
        };
        let muxed = match &self.media {
            Mux::Fmp4(s) => {
                let m = s.metrics();
                m.dropped_before_start + m.dropped_pending_config
            }
            Mux::Units { writer, .. } => writer.metrics().dropped_before_start,
        };
        let dropped = self.source.metrics().before_config + muxed;
        let new = dropped.saturating_sub(self.dropped);
        if new > 0 {
            self.dropped = dropped;
            self.publication.shared.count(|m| m.dropped_units += new);
        }
        res
    }

    /// Takes the stream's `onMetaData`.
    fn metadata(&mut self, m: &Metadata) {
        if let Mux::Units {
            sign: Sign::Choosing(choice),
            ..
        } = &mut self.media
        {
            choice.metadata(m);
        }
    }

    /// Creates a unit-format session's swarm once its signing settles at
    /// `now`; see the [module docs](self#media).
    async fn settle(&mut self, now: Time) -> Result<(), End> {
        let Mux::Units {
            writer,
            sign: Sign::Choosing(choice),
            ..
        } = &self.media
        else {
            return Ok(());
        };
        let Some(signing) = choice.settled(now, writer.output().len()) else {
            return Ok(());
        };
        let options = unit_ingress_options(&signing);
        let uri = Uri::new(self.publication.swarm_id.clone(), options.uri_options()).to_string();
        let s = self.publication.shared.clone();
        let created = s.create_ingest_swarm(&mut self.publication, options).await;
        let writer = created.map_err(|r| End::Stopped(r.description))?;
        if let Mux::Units { writer, sign, .. } = &mut self.media {
            *sign = Sign::Timer(signing.timer(writer.frame_size()));
        }
        self.swarm = Some(IngestSwarm { writer, uri });
        tracing::debug!(
            channel = self.publication.channel_id,
            chunks_per_signature = signing.chunks_per_signature,
            interval_ms = signing.interval.as_millis(),
            "unit-format stream's signing settled"
        );
        Ok(())
    }

    /// Writes the stream's final bytes into the swarm at `now`, once it
    /// exists.
    async fn write_output(&mut self, now: Time) -> Result<(), End> {
        let Some(swarm) = &mut self.swarm else {
            return Ok(());
        };
        let out = match &mut self.media {
            Mux::Fmp4(s) => s.sink_mut().take_output(),
            Mux::Units { writer, .. } => writer.take_output(),
        };
        let res = if out.is_empty() {
            Ok(())
        } else {
            swarm.writer.write(&out).await
        };
        match &mut self.media {
            Mux::Fmp4(s) => s.sink_mut().restore(out),
            Mux::Units { writer, sign, .. } => {
                writer.restore(out);
                if let (Ok(()), Sign::Timer(t)) = (&res, sign) {
                    t.written(writer.offset(), now);
                }
            }
        }
        res.map_err(|e| {
            self.publication.shared.count(|m| m.write_errors += 1);
            End::Write(e.to_string())
        })
    }

    /// When the unit-format stream's partial integrity unit is due for
    /// signing.
    fn sign_due(&self) -> Option<Time> {
        match &self.media {
            Mux::Units {
                sign: Sign::Timer(t),
                ..
            } => t.due(),
            _ => None,
        }
    }

    /// When the unit-format stream's wait for its signing ends.
    fn settle_due(&self) -> Option<Time> {
        match &self.media {
            Mux::Units {
                sign: Sign::Choosing(c),
                ..
            } => c.deadline(),
            _ => None,
        }
    }

    /// The unit-format stream has a partial integrity unit.
    fn partial(&self) -> bool {
        match &self.media {
            Mux::Units {
                sign: Sign::Timer(t),
                ..
            } => t.is_partial(),
            _ => false,
        }
    }

    /// Has the swarm sign a unit-format stream's partial integrity unit:
    /// ends the frame in progress, writes it, flushes the swarm writer and
    /// goes on where the next unit starts. The fMP4 format's packed layout
    /// would read the padding as record bytes, so it is never flushed.
    async fn sign(&mut self, now: Time) -> Result<(), End> {
        let Mux::Units { writer, .. } = &mut self.media else {
            return Ok(());
        };
        writer.end_frame();
        self.write_output(now).await?;
        let Some(swarm) = &mut self.swarm else {
            return Ok(());
        };
        let flushed = swarm.writer.flush().await;
        let next = flushed.map_err(|e| {
            self.publication.shared.count(|m| m.write_errors += 1);
            End::Write(e.to_string())
        })?;
        let Mux::Units {
            writer,
            sign: Sign::Timer(sign),
            ..
        } = &mut self.media
        else {
            return Ok(());
        };
        let padding = next.saturating_sub(writer.offset());
        writer
            .continue_at(next)
            .map_err(|e| End::Write(e.to_string()))?;
        sign.flushed(next);
        self.publication.shared.count(|m| {
            m.early_signs += 1;
            m.bytes += padding;
        });
        Ok(())
    }

    /// Does what is due at `now`: creates a unit-format session's swarm
    /// once its wait ends, writing what the session held, and has a
    /// partial integrity unit signed at its interval.
    async fn timer(&mut self, now: Time) -> Result<(), End> {
        self.settle(now).await?;
        self.write_output(now).await?;
        if self.sign_due().is_some_and(|t| now >= t) {
            self.sign(now).await?;
        }
        self.take_segments(now, true);
        Ok(())
    }

    /// Counts the records written, and keeps their listing if `list`.
    /// Records wait in the session until its swarm exists.
    fn take_segments(&mut self, now: Time, list: bool) {
        let Some(swarm) = &self.swarm else {
            return;
        };
        let (written, mime) = match &mut self.media {
            Mux::Fmp4(s) => {
                let mut written = false;
                while let Some(info) = s.take_segment() {
                    written = true;
                    let bytes = info.bytes as u64;
                    self.publication.shared.count(|m| {
                        m.segments += 1;
                        m.bytes += bytes;
                    });
                }
                (written, fmp4_mime(s.segment_configs()))
            }
            Mux::Units {
                writer, counted, ..
            } => {
                let m = writer.metrics();
                let records = m.records.saturating_sub(counted.records);
                let bytes = m.bytes.saturating_sub(counted.bytes);
                *counted = m;
                self.publication.shared.count(|i| {
                    i.segments += records;
                    i.bytes += bytes;
                });
                (records > 0, units_mime(writer.configs()))
            }
        };
        if !written {
            return;
        }
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
        s.segment_written(&self.publication, now, listing);
    }

    /// Ends and writes the record in progress, and has the swarm sign a
    /// unit-format stream's last partial unit: step 1 of the
    /// [teardown](self#teardown).
    async fn finish(&mut self, now: Time) {
        let finished = match &mut self.media {
            Mux::Fmp4(s) => s.finish().is_ok(),
            Mux::Units { writer, .. } => writer.finish().is_ok(),
        };
        if finished && self.write_output(now).await.is_ok() {
            self.take_segments(now, false);
        }
        if self.partial() && self.sign(now).await.is_ok() {
            self.take_segments(now, false);
        }
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
        let c = self.shared.ingress.borrow_mut().conns.remove(&self.id);
        drop(c);
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Connection<K, H, V, T> {
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
        if let Some(l) = &mut live
            && l.publication.is_current()
        {
            l.finish(s.timer.now()).await;
        }
        self.close(&end).await;
        if let Some(live) = live {
            let channel_id = live.publication.channel_id;
            live.publication.end().await;
            tracing::info!(channel = channel_id, reason = %end, "RTMP stream ended");
            s.emit(VideoEvent::IngressEnded {
                channel_id,
                reason: end.to_string(),
            });
        }
    }

    /// When the connection's task wakes: at its deadline, when a
    /// unit-format stream's partial integrity unit is due for signing, or
    /// when its wait for its signing ends.
    fn wake_at(&self) -> Time {
        let (due, _) = self.due();
        let Some(l) = &self.live else {
            return due;
        };
        [l.sign_due(), l.settle_due()]
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
                    Some(RtmpEvent::Unpublish) => return Err(End::Unpublished),
                };
                evented = true;
                if let Some((app, name)) = publish {
                    self.publish(&app, &name).await;
                }
                if let Some(l) = &mut self.live {
                    l.settle(now).await?;
                    l.write_output(now).await?;
                    l.take_segments(now, true);
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
