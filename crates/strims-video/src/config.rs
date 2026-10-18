// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! How the video control runs and what bounds it, what it counts, and the
//! checks on the device's stored ingress and HLS configs.

use strims_base::time::Duration;
use strims_directory::KeeperConfig;
use strims_proto::video::v1::{HlsEgressConfig, VideoIngressConfig};

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
}

/// The RTMP ingress: its connections, the sessions publishing on them,
/// and the records it cuts.
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
    /// The media a record holds before it is cut at the next keyframe.
    pub min_segment_media_bytes: usize,
    /// The largest record. A session whose record would grow past it ends:
    /// half the ingress swarm's 16 MiB window, so a viewer holds a whole
    /// record.
    pub max_record_bytes: usize,
    pub rtmp: RtmpSettings,
    /// How long a closing connection has to take its last output, such as
    /// the status that tells a publisher why.
    pub close_timeout: Duration,
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
            min_segment_media_bytes: 32 * 1024,
            max_record_bytes: 8 * 1024 * 1024,
            rtmp: RtmpSettings::default(),
            close_timeout: Duration::from_secs(5),
        }
    }
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

/// The UI's captures. An append's data is bounded by the frontend
/// connection's message size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureSettings {
    /// Captures open at once.
    pub max_captures: usize,
    /// Networks one capture is listed on.
    pub max_network_keys: usize,
    /// The longest MIME type, in bytes of printable ASCII.
    pub max_mime_bytes: usize,
}

impl Default for CaptureSettings {
    fn default() -> CaptureSettings {
        CaptureSettings {
            max_captures: 2,
            max_network_keys: 16,
            max_mime_bytes: 256,
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
    /// How long a stream lasts without a request, and how often idle
    /// streams are looked for.
    pub idle_timeout: Duration,
    pub sweep_interval: Duration,
    /// Segments a playlist lists, and segments a stream keeps.
    pub listed_segments: usize,
    pub retained_segments: usize,
    /// The bytes of segments a stream keeps.
    pub max_retained_bytes: usize,
    /// The largest record a stream reads.
    pub max_record_bytes: usize,
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
            idle_timeout: Duration::from_secs(60),
            sweep_interval: Duration::from_secs(10),
            listed_segments: 6,
            retained_segments: 8,
            max_retained_bytes: 64 * 1024 * 1024,
            max_record_bytes: 16 * 1024 * 1024,
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
    /// Capture records dropped because they ended in a frame that held
    /// the end of the record before them.
    pub dropped_records: u64,
    /// Capture writes into a swarm that failed.
    pub write_errors: u64,
    /// Stream readers opened again on a swarm that replaced theirs.
    pub reopens: u64,
    /// HLS streams opened.
    pub hls_streams: u64,
    /// Segments HLS streams served.
    pub hls_segments: u64,
    /// Records HLS streams skipped: over
    /// [`HlsSettings::max_record_bytes`], or not fMP4 records with media
    /// time.
    pub hls_skipped_records: u64,
    /// The RTMP ingress's counters.
    pub ingress: IngressMetrics,
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
