// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! How the video control runs and what bounds it, what it counts, and the
//! checks on the device's stored ingress and HLS configs.

use strims_base::time::Duration;
use strims_directory::KeeperConfig;
use strims_proto::video::v1::{HlsEgressConfig, VideoIngressConfig};

use crate::codec::RenditionPlan;
use crate::ingress::IngressMetrics;

/// The longest ingress listen address, ingress public address or HLS
/// public address, in bytes.
pub const MAX_ADDR_BYTES: usize = 256;
/// The most service network keys the ingress config holds.
pub const MAX_SERVICE_NETWORKS: usize = 64;
/// The longest network key taken, in bytes. A network's key is its root
/// certificate's ed25519 key, so a longer one names no network.
pub const MAX_NETWORK_KEY_BYTES: usize = 64;

/// How the video control runs on a node, with the bounds on what peers,
/// broadcasters and viewers can make it and its driver hold or do.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub ingress: IngressSettings,
    pub capture: CaptureSettings,
    pub egress: EgressSettings,
    pub hls: HlsSettings,
    pub listing: ListingSettings,
    pub autoseed: AutoseedSettings,
}

/// The RTMP ingress: its connections and the sessions publishing on them.
/// A session's records are bounded by
/// [`units::MAX_RECORD_LEN`](crate::units::MAX_RECORD_LEN).
#[derive(Clone, Debug, PartialEq)]
pub struct IngressSettings {
    /// How long a connection has from its accept to finish the RTMP
    /// handshake.
    pub handshake_deadline: Duration,
    /// How long a connection has from its accept to publish.
    pub publish_deadline: Duration,
    /// How long a publishing session may go without media.
    pub media_idle: Duration,
    /// Connections open at once, publishing or not.
    pub max_connections: usize,
    /// Connections open at once from one remote address that haven't
    /// published.
    pub max_unpublished_per_address: usize,
    /// Sessions publishing at once.
    pub max_sessions: usize,
    /// The wait before binding the listener again after a bind failed.
    pub bind_retry: Duration,
    /// How long a session's access units wait, from its
    /// first, for what settles its swarm's signing
    /// ([`UnitSigning`](crate::units::UnitSigning)): the stream's
    /// `onMetaData` or a video config. Past it, the session creates its
    /// swarm with the signing of what it knows (see
    /// [`ingress`](crate::ingress#media)). Audio alone without metadata
    /// waits it out at the start; a stream whose video config comes after
    /// its first audio frames waits for that config within it.
    pub signing_wait: Duration,
    pub rtmp: RtmpSettings,
    /// How long a closing connection has to take its last output, such as
    /// the status that tells a publisher why.
    pub close_timeout: Duration,
    /// How long a session whose connection ended without an
    /// unpublish (it dropped, failed or timed out) waits for its encoder to
    /// publish to the channel again, keeping its listing and swarm, before
    /// it ends with its END record (see
    /// [`ingress`](crate::ingress#teardown)). Encoders reconnect after a
    /// drop with backoff: OBS by default retries after 2 s, and about 1.5
    /// times as long again before each retry after that, so its first five
    /// retries come within 30 s.
    pub resume_grace: Duration,
    /// How sessions use the node's codec stage
    /// ([`VideoHost::codec`](crate::VideoHost::codec)).
    pub transcode: TranscodeSettings,
    /// When sessions make their snippets' thumbnails with the node's
    /// codec stage.
    pub thumbnails: ThumbnailSettings,
}

impl Default for IngressSettings {
    fn default() -> IngressSettings {
        IngressSettings {
            handshake_deadline: Duration::from_secs(10),
            publish_deadline: Duration::from_secs(20),
            media_idle: Duration::from_secs(30),
            max_connections: 8,
            max_unpublished_per_address: 2,
            max_sessions: 4,
            bind_retry: Duration::from_secs(30),
            signing_wait: Duration::from_secs(1),
            rtmp: RtmpSettings::default(),
            close_timeout: Duration::from_secs(5),
            resume_grace: Duration::from_secs(30),
            transcode: TranscodeSettings::default(),
            thumbnails: ThumbnailSettings::default(),
        }
    }
}

/// When a live session makes the thumbnail its snippet shows in place of
/// its channel's, a JPEG of one of its stream's keyframes, where the node
/// has a codec stage; see [`ingress`](crate::ingress#thumbnails).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThumbnailSettings {
    /// How long after the stream's first video unit the first thumbnail
    /// is made, at the next keyframe: a stream's first pictures are often
    /// black or a fade.
    pub first_after: Duration,
    /// How long after a thumbnail began the next is made, at the next
    /// keyframe.
    pub interval: Duration,
    /// How long after a thumbnail failed the next is tried, at half the
    /// width, down to [`MIN_THUMBNAIL_WIDTH`].
    pub retry: Duration,
    /// The box a thumbnail fits, at its stream's aspect ratio.
    pub max_width: u16,
    pub max_height: u16,
    /// How long the codec stage has to make one.
    pub timeout: Duration,
}

/// The narrowest thumbnail a retry makes.
pub const MIN_THUMBNAIL_WIDTH: u16 = 160;

impl Default for ThumbnailSettings {
    /// The first 5 s in, then every 5 minutes, within 640x360, the box
    /// of the thumbnails the UI's browser capture makes.
    fn default() -> ThumbnailSettings {
        ThumbnailSettings {
            first_after: Duration::from_secs(5),
            interval: Duration::from_secs(300),
            retry: Duration::from_secs(30),
            max_width: 640,
            max_height: 360,
            timeout: Duration::from_secs(10),
        }
    }
}

/// How the RTMP ingress transcodes its sessions' streams, where the node
/// has a codec stage; see [`ingress`](crate::ingress#renditions).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranscodeSettings {
    /// The renditions made of every stream besides its own, each where its
    /// shorter side is below the stream's; at most
    /// [`MAX_RENDITIONS`](crate::codec::MAX_RENDITIONS) less one. None by
    /// default: a node transcodes only HEVC and AV1, into the H.264 every
    /// viewer can play.
    ///
    /// Each rendition is another swarm the ingest node seeds, which splits
    /// the stream's viewers between swarms. For example, 720p of up to 60
    /// frames a second at 3 Mbit/s and 480p30 at 1.2 Mbit/s:
    ///
    /// ```
    /// # use strims_video::codec::RenditionPlan;
    /// let ladder = vec![
    ///     RenditionPlan { short_side: 720, max_frame_rate: 60, bitrate: 3_000_000 },
    ///     RenditionPlan { short_side: 480, max_frame_rate: 30, bitrate: 1_200_000 },
    /// ];
    /// ```
    ///
    /// takes the seed upload of a 6 Mbit/s 1080p stream to about 10.2
    /// Mbit/s, before any copies.
    pub ladder: Vec<RenditionPlan>,
    /// The bitrates of the H.264 made of an HEVC or AV1 stream, by its
    /// shorter side, ascending: the first entry at least the stream's
    /// side. A side past the last is scaled down to it.
    pub h264_bitrates: Vec<(u16, u64)>,
    /// The most frames a second the H.264 made of an HEVC or AV1 stream
    /// keeps.
    pub h264_max_frame_rate: u32,
    /// Sessions transcoded at once. A session past it makes no renditions,
    /// and an HEVC or AV1 publish past it is refused.
    pub max_sessions: usize,
    /// How long a transcode may hold input without output before it is
    /// taken as stalled and started again.
    pub stall_timeout: Duration,
    /// Starts of a session's transcode after its first within
    /// `restart_window`, past which the session drops its renditions.
    pub max_restarts: usize,
    pub restart_window: Duration,
    /// How long a session's end waits for its transcode's last output.
    pub finish_timeout: Duration,
}

impl Default for TranscodeSettings {
    fn default() -> TranscodeSettings {
        TranscodeSettings {
            ladder: Vec::new(),
            h264_bitrates: vec![(480, 1_500_000), (720, 4_000_000), (1080, 6_000_000)],
            h264_max_frame_rate: 60,
            max_sessions: 2,
            stall_timeout: Duration::from_secs(5),
            max_restarts: 3,
            restart_window: Duration::from_secs(600),
            finish_timeout: Duration::from_secs(2),
        }
    }
}

/// What the RTMP ingress's transcodes have done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TranscodeMetrics {
    /// Transcodes started, restarts among them.
    pub starts: u64,
    pub restarts: u64,
    /// Transcodes that stalled, failed, or ended before their input.
    pub stalls: u64,
    pub failures: u64,
    /// Sessions that dropped their renditions after too many restarts.
    pub abandoned: u64,
    /// Runs of input dropped to the stream's next keyframe while a
    /// transcode was behind.
    pub dropped_gops: u64,
    /// Sessions refused a transcode by [`TranscodeSettings::max_sessions`].
    pub over_cap: u64,
    /// Rendition records written, and their stream bytes.
    pub records: u64,
    pub bytes: u64,
}

/// What one RTMP connection takes and holds. Decoder configs (avcC,
/// AudioSpecificConfig) are bounded by [`MAX_CONFIG_LEN`](crate::MAX_CONFIG_LEN),
/// and AMF0 values by the [`amf0`](crate::amf0) decoder's limits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RtmpSettings {
    /// The chunk size the server sends with.
    pub chunk_size: u32,
    /// The largest audio or video message.
    pub max_media_message: usize,
    /// The largest command or data message.
    pub max_command_message: usize,
    /// The bytes of partial messages held across all chunk streams. They
    /// grow as chunks arrive, never from a message's declared length.
    pub max_partial_bytes: usize,
    /// Chunk streams in use at once.
    pub max_chunk_streams: usize,
    /// Output queued for the connection; a connection past it closes.
    pub max_queued_output: usize,
    /// The window acknowledgement size and peer bandwidth the server
    /// sets.
    pub window_ack_size: u32,
    pub peer_bandwidth: u32,
}

impl Default for RtmpSettings {
    fn default() -> RtmpSettings {
        RtmpSettings {
            chunk_size: 4096,
            max_media_message: 4 * 1024 * 1024,
            max_command_message: 64 * 1024,
            max_partial_bytes: 8 * 1024 * 1024,
            max_chunk_streams: 64,
            max_queued_output: 64 * 1024,
            window_ack_size: 2_500_000,
            peer_bandwidth: 2_500_000,
        }
    }
}

/// The UI's captures. An append's bytes are bounded by the frontend
/// connection's message size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureSettings {
    /// Captures open at once.
    pub max_captures: usize,
    /// Networks one capture is listed on.
    pub max_network_keys: usize,
    /// Access units one append carries.
    pub max_units_per_append: usize,
    /// The bytes of one capture's appends waiting for its writer.
    pub max_queued_bytes: usize,
    /// The bytes a second a capture takes over time, its configs' and
    /// units' with [`ITEM_COST`](crate::capture::ITEM_COST) for
    /// each: about 34 Mbit/s, several times a capture's encoders' rate.
    pub max_rate: u64,
    /// The bytes a capture takes at once beyond its rate: two of the
    /// largest keyframes.
    pub max_burst: u64,
}

impl Default for CaptureSettings {
    fn default() -> CaptureSettings {
        CaptureSettings {
            max_captures: 2,
            max_network_keys: 16,
            max_units_per_append: 512,
            max_queued_bytes: 16 * 1024 * 1024,
            max_rate: 4 * 1024 * 1024,
            max_burst: 2 * crate::units::MAX_RECORD_LEN as u64,
        }
    }
}

/// The swarms viewers read, for the UI's streams and HLS together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EgressSettings {
    /// Swarms added for streams at once. A stream of a swarm the node
    /// already runs adds none.
    pub max_swarms: usize,
    /// Each added swarm's live window, in bytes: the largest power of two
    /// of chunks that fits.
    pub window_bytes: usize,
    /// Streams open at once, of any swarm.
    pub max_streams: usize,
    /// The most bytes one Data message carries.
    pub max_data_bytes: usize,
    /// The longest swarm URI.
    pub max_uri_bytes: usize,
    /// Networks a swarm is read on, and the longest network key.
    pub max_network_keys: usize,
    pub max_network_key_bytes: usize,
    /// The swarm URI options taken: chunk sizes, chunks per signature and
    /// streams.
    pub min_chunk_size: u32,
    pub max_chunk_size: u32,
    pub max_chunks_per_signature: u32,
    pub max_stream_count: u32,
    /// The bytes one read takes from a swarm.
    pub read_buffer_bytes: usize,
    /// How long a unit-format stream's join waits for the rewind to the
    /// GOP it joins while no later sync record has come (see
    /// [`ChunkedReader`](crate::ChunkedReader)). A rewind fetches the GOP's
    /// bytes so far, and holders that also push the live stream answer it
    /// with what their uplinks have left: two viewers joining in the same
    /// second behind a 10 Mbit/s seed wait seconds for theirs. Until the
    /// next GOP starts, the rewind is the join's quickest way to a
    /// picture, so it covers GOPs of up to about 10 s.
    pub rewind_timeout: Duration,
    /// How long after the next GOP's sync record arrives a join still takes
    /// its rewind: a player starting at that record plays once it holds its
    /// initial target lag of media past it, and a rewound GOP plays as soon
    /// as the player's join has settled, so the rewind still plays first
    /// while it lands within the target lag less the join's settle.
    pub rewind_grace: Duration,
    /// How often a swarm no listing names takes the other options of a
    /// stream's URI at most ([`egress`](crate::egress#options)). Its writer
    /// changes its options only as it starts a new session, which a
    /// stream to follow doesn't do twice a minute, while every change
    /// moves the swarm's other streams to a swarm of the new options: a
    /// caller naming other options shifts them at most this often.
    pub unlisted_reopen_interval: Duration,
}

impl Default for EgressSettings {
    fn default() -> EgressSettings {
        EgressSettings {
            max_swarms: 4,
            window_bytes: 32 * 1024 * 1024,
            max_streams: 32,
            max_data_bytes: 32 * 1024,
            max_uri_bytes: 2 * 1024,
            max_network_keys: 16,
            max_network_key_bytes: 64,
            min_chunk_size: 256,
            max_chunk_size: 65536,
            max_chunks_per_signature: 4096,
            max_stream_count: 64,
            read_buffer_bytes: 64 * 1024,
            rewind_timeout: Duration::from_secs(10),
            rewind_grace: Duration::from_millis(700),
            unlisted_reopen_interval: Duration::from_secs(60),
        }
    }
}

/// The HLS egress, and the HTTP server a driver runs for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HlsSettings {
    /// The driver serves HLS over HTTP, so HLS egress is supported.
    pub serve: bool,
    /// The address the driver serves HLS at, such as
    /// `http://127.0.0.1:8082`: playlist URLs start with it when the
    /// stored HLS config has no public server address.
    pub base_url: String,
    /// Streams open at once.
    pub max_streams: usize,
    /// How long the open of a new stream waits for its first segment
    /// before it answers: a player given the URL of an empty playlist may
    /// give up on it.
    pub open_wait: Duration,
    /// How long a stream lasts without a request, and how often idle
    /// streams are looked for.
    pub idle_timeout: Duration,
    pub sweep_interval: Duration,
    /// How long a stream whose read failed or whose swarm is gone waits
    /// before it reads on or opens its swarm again, doubling up to the
    /// most while it fails.
    pub reopen_interval: Duration,
    pub max_reopen_interval: Duration,
    /// A playlist's `EXT-X-TARGETDURATION`, fixed for its life, in whole
    /// seconds (a fraction rounds up): every segment rounds to at most it.
    /// GOPs that round to at most it stay whole: those of keyframes every
    /// 1 to 4 seconds, as encoders are usually set to, and those of
    /// 250-frame GOPs at 60 frames a second. Longer GOPs, from scene-cut
    /// keyframes of CRF or VBR streams, are split into segments that start
    /// mid-GOP. Players that don't load parts start about three targets
    /// behind the live edge.
    pub target_duration: Duration,
    /// The least media time a playlist lists, three targets at least, and
    /// that a stream keeps for fetching. A segment that left the listing
    /// stays fetchable for its own time and a playlist's, the longest a
    /// player may still read a playlist that listed it: the default keeps
    /// the 16 s listed and 24 s more, a playlist's 20 s with the segment
    /// past its listing, and a segment's 4.
    pub listed_duration: Duration,
    pub retained_duration: Duration,
    /// The bytes of segments a stream keeps.
    pub max_retained_bytes: usize,
    /// The largest segment of a stream, its held samples included
    /// ([`HlsMuxerConfig::max_segment_bytes`](crate::hls::HlsMuxerConfig::max_segment_bytes)).
    pub max_segment_bytes: usize,
    /// The media a fragment of a stream's segment holds
    /// ([`HlsMuxerConfig::fragment_duration`](crate::hls::HlsMuxerConfig::fragment_duration)).
    pub fragment_duration: Duration,
    /// The most media a part of a stream's segments lasts, for
    /// low-latency HLS ([`hls`](crate::hls#low-latency)): from 0.1 s to
    /// the target duration, fixed for a playlist's life; zero serves no
    /// parts. Players that load parts start three part targets behind the
    /// live edge.
    pub part_target: Duration,
    /// The longest request path.
    pub max_path_bytes: usize,
    /// The largest HTTP request head, and how long a connection has to
    /// send it.
    pub max_request_head_bytes: usize,
    pub request_head_timeout: Duration,
    /// HTTP connections open at once.
    pub max_connections: usize,
}

impl Default for HlsSettings {
    fn default() -> HlsSettings {
        HlsSettings {
            serve: false,
            base_url: String::new(),
            max_streams: 4,
            open_wait: Duration::from_secs(6),
            idle_timeout: Duration::from_secs(60),
            sweep_interval: Duration::from_secs(10),
            reopen_interval: Duration::from_secs(1),
            max_reopen_interval: Duration::from_secs(30),
            target_duration: Duration::from_secs(4),
            listed_duration: Duration::from_secs(16),
            retained_duration: Duration::from_secs(40),
            max_retained_bytes: 64 * 1024 * 1024,
            max_segment_bytes: 16 * 1024 * 1024,
            fragment_duration: crate::hls::DEFAULT_FRAGMENT_DURATION,
            part_target: Duration::from_secs(1),
            max_path_bytes: 256,
            max_request_head_bytes: 8 * 1024,
            request_head_timeout: Duration::from_secs(10),
            max_connections: 32,
        }
    }
}

/// How the directory listings of streams are kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListingSettings {
    /// The shortest wait between two publishes of a listing.
    pub min_republish: Duration,
    /// How often a listing is checked to be in its directory.
    pub listing_check: Duration,
    /// The longest a stream's end waits for its listing's unpublish.
    pub unpublish_timeout: Duration,
}

impl Default for ListingSettings {
    fn default() -> ListingSettings {
        ListingSettings {
            min_republish: Duration::from_secs(5),
            listing_check: Duration::from_secs(60),
            unpublish_timeout: Duration::from_secs(5),
        }
    }
}

impl ListingSettings {
    /// The listing keepers' config.
    pub fn keeper_config(&self) -> KeeperConfig {
        KeeperConfig {
            min_republish: self.min_republish,
            listing_check: self.listing_check,
            unpublish_timeout: self.unpublish_timeout,
        }
    }
}

/// The device's autoseed rules, and the seeds they start; see
/// [`autoseed`](crate::autoseed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoseedSettings {
    /// Rules the frontend creates. Each names one swarm.
    pub max_rules: usize,
    /// Rules seeding at once, from the lowest rule id up.
    pub max_seeds: usize,
    /// The longest rule label and salt, in bytes.
    pub max_label_bytes: usize,
    pub max_salt_bytes: usize,
}

impl Default for AutoseedSettings {
    fn default() -> AutoseedSettings {
        AutoseedSettings {
            max_rules: 64,
            max_seeds: 8,
            max_label_bytes: 256,
            max_salt_bytes: 64,
        }
    }
}

/// What the video control has done.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Times the store's events lagged and the records were read again.
    pub store_lags: u64,
    /// Store reads that failed while following the records.
    pub store_errors: u64,
    /// Node calls that failed: creating, adding or removing a swarm, or
    /// opening its reader.
    pub host_errors: u64,
    /// Capture units the writer dropped: those before a capture's first
    /// keyframe.
    pub dropped_records: u64,
    /// Capture writes into a swarm that failed.
    pub write_errors: u64,
    /// Stream readers opened again on a swarm that replaced theirs.
    pub reopens: u64,
    /// HLS streams opened.
    pub hls_streams: u64,
    /// Segments HLS streams served.
    pub hls_segments: u64,
    /// Records HLS streams skipped: those over
    /// [`units::MAX_RECORD_LEN`](crate::units::MAX_RECORD_LEN), and those
    /// whose unit the muxer refused.
    pub hls_skipped_records: u64,
    /// Times an autoseed rule found another component's transfer of its
    /// swarm under the rule's salt. The rule seeds nothing while it is so.
    pub autoseed_skipped: u64,
    /// The RTMP ingress's counters.
    pub ingress: IngressMetrics,
    /// The RTMP ingress's transcodes.
    pub transcode: TranscodeMetrics,
}

/// Why a config was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The listen address is over [`MAX_ADDR_BYTES`].
    #[error("server address too long")]
    ServerAddrTooLong,
    /// The listen address isn't a host and a port.
    #[error("server address must be host:port")]
    ServerAddr,
    /// The public address is over [`MAX_ADDR_BYTES`].
    #[error("public server address too long")]
    PublicAddrTooLong,
    /// The HLS public address isn't an HTTP URL.
    #[error("public server address must start with http:// or https://")]
    PublicUrl,
    /// The ingress config holds over [`MAX_SERVICE_NETWORKS`] keys.
    #[error("too many service networks")]
    TooManyServiceNetworks,
    /// A service network key is over [`MAX_NETWORK_KEY_BYTES`].
    #[error("service network key too long")]
    ServiceNetworkKeyTooLong,
}

/// Checks an ingress config: its listen address is a `host:port` of at
/// most [`MAX_ADDR_BYTES`], its public address at most that long, and
/// its service network keys within their bounds.
pub fn check_ingress_config(c: &VideoIngressConfig) -> Result<(), ConfigError> {
    if c.server_addr.len() > MAX_ADDR_BYTES {
        return Err(ConfigError::ServerAddrTooLong);
    }
    if !is_host_port(&c.server_addr) {
        return Err(ConfigError::ServerAddr);
    }
    if c.public_server_addr.len() > MAX_ADDR_BYTES {
        return Err(ConfigError::PublicAddrTooLong);
    }
    if c.service_network_keys.len() > MAX_SERVICE_NETWORKS {
        return Err(ConfigError::TooManyServiceNetworks);
    }
    if c.service_network_keys
        .iter()
        .any(|k| k.len() > MAX_NETWORK_KEY_BYTES)
    {
        return Err(ConfigError::ServiceNetworkKeyTooLong);
    }
    Ok(())
}

/// Checks an HLS config: its public address is empty, or an `http://` or
/// `https://` URL of at most [`MAX_ADDR_BYTES`].
pub fn check_hls_config(c: &HlsEgressConfig) -> Result<(), ConfigError> {
    let addr = &c.public_server_addr;
    if addr.len() > MAX_ADDR_BYTES {
        return Err(ConfigError::PublicAddrTooLong);
    }
    let http = ["http://", "https://"].iter().any(|scheme| {
        addr.get(..scheme.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(scheme))
    });
    if !addr.is_empty() && !http {
        return Err(ConfigError::PublicUrl);
    }
    Ok(())
}

/// Whether `addr` is a host, which a URL can carry, then `:` and a
/// nonzero port number.
fn is_host_port(addr: &str) -> bool {
    let Some((host, port)) = addr.rsplit_once(':') else {
        return false;
    };
    let host_ok = !host.is_empty()
        && host
            .bytes()
            .all(|b| b.is_ascii_graphic() && !matches!(b, b'/' | b'?' | b'#' | b'@'));
    let port_ok = !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit());
    host_ok && port_ok && port.parse::<u16>().is_ok_and(|p| p != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ingress(server_addr: &str) -> VideoIngressConfig {
        VideoIngressConfig {
            server_addr: server_addr.into(),
            ..Default::default()
        }
    }

    #[test]
    fn listen_addresses_are_hosts_and_ports() {
        for ok in ["127.0.0.1:1935", "0.0.0.0:1935", "[::]:1935", "a.b:65535"] {
            assert_eq!(check_ingress_config(&ingress(ok)), Ok(()), "{ok}");
        }
        for bad in [
            "",
            "127.0.0.1",
            ":1935",
            "host:",
            "host:+1935",
            "host:65536",
            "host:0",
            "a b:1935",
            "a/b:1935",
            "user@host:1935",
        ] {
            assert_eq!(
                check_ingress_config(&ingress(bad)),
                Err(ConfigError::ServerAddr),
                "{bad}"
            );
        }
        let long = format!("{}:1935", "h".repeat(MAX_ADDR_BYTES));
        assert_eq!(
            check_ingress_config(&ingress(&long)),
            Err(ConfigError::ServerAddrTooLong)
        );
    }

    #[test]
    fn ingress_configs_are_bounded() {
        let mut c = ingress("127.0.0.1:1935");
        c.public_server_addr = "p".repeat(MAX_ADDR_BYTES);
        c.service_network_keys = vec![vec![1; MAX_NETWORK_KEY_BYTES]; MAX_SERVICE_NETWORKS];
        assert_eq!(check_ingress_config(&c), Ok(()));
        let mut long_addr = c.clone();
        long_addr.public_server_addr.push('p');
        assert_eq!(
            check_ingress_config(&long_addr),
            Err(ConfigError::PublicAddrTooLong)
        );
        let mut many = c.clone();
        many.service_network_keys.push(Vec::new());
        assert_eq!(
            check_ingress_config(&many),
            Err(ConfigError::TooManyServiceNetworks)
        );
        let mut long_key = c;
        long_key
            .service_network_keys
            .push(vec![1; MAX_NETWORK_KEY_BYTES + 1]);
        long_key.service_network_keys.remove(0);
        assert_eq!(
            check_ingress_config(&long_key),
            Err(ConfigError::ServiceNetworkKeyTooLong)
        );
    }

    #[test]
    fn hls_public_addresses_are_empty_or_http_urls() {
        let hls = |addr: &str| HlsEgressConfig {
            enabled: true,
            public_server_addr: addr.into(),
        };
        for ok in ["", "http://a", "https://cdn.strims.gg:8443/x", "HTTPS://A"] {
            assert_eq!(check_hls_config(&hls(ok)), Ok(()), "{ok}");
        }
        for bad in ["cdn.strims.gg", "ftp://a", "http:/a", "é"] {
            assert_eq!(check_hls_config(&hls(bad)), Err(ConfigError::PublicUrl));
        }
        let long = format!("http://{}", "a".repeat(MAX_ADDR_BYTES));
        assert_eq!(
            check_hls_config(&hls(&long)),
            Err(ConfigError::PublicAddrTooLong)
        );
    }
}
