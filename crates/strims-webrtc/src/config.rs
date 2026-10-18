// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Endpoint settings. Plain data on every target, so a node's WebRTC
//! configuration has one type whichever implementation runs it.

use std::net::IpAddr;

use strims_base::time::Duration;
use strims_vnic::webrtc::DEFAULT_ICE_SERVERS;

use crate::stun;

/// Endpoint settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndpointConfig {
    /// ICE server URLs. `None` means [`DEFAULT_ICE_SERVERS`];
    /// `Some(vec![])` means none. Only `stun:` URLs are used
    /// (see [`crate::stun::parse_stun_url`]); the driver resolves them and
    /// passes the addresses to `WebRtcEndpoint::set_stun_servers`.
    pub ice_servers: Option<Vec<String>>,
    /// The IP that host candidates on sockets of its address family
    /// advertise (a 1:1 NAT). Datagrams arriving on such sockets are
    /// matched against it, so a socket bound to the wildcard address needs
    /// it.
    pub host_ip: Option<IpAddr>,
    /// How long gathering waits for STUN servers.
    pub stun_gather_timeout: Duration,
    /// A connection whose data channel hasn't opened this long after it was
    /// created is closed with `CloseReason::ConnectTimeout`. `None` leaves
    /// it to the owner.
    pub connect_timeout: Option<Duration>,
    /// How long ICE may stay disconnected before the connection closes with
    /// `CloseReason::IceFailed`.
    pub failed_timeout: Duration,
    /// Bytes the data channel may hold unacknowledged (str0m's
    /// `sctp_max_buffered_amount`). It caps throughput at about this much
    /// per round trip; the receiver's SCTP window (1 MiB in sctp-proto)
    /// caps it too. At least `2 * DATA_CHANNEL_MTU`.
    pub send_buffer: usize,
    /// The most connections at once. Each vpn peer exchange creates one,
    /// so this bounds what remote peers can make a node hold.
    pub max_connections: usize,
}

impl Default for EndpointConfig {
    fn default() -> EndpointConfig {
        EndpointConfig {
            ice_servers: None,
            host_ip: None,
            stun_gather_timeout: Duration::from_secs(5),
            connect_timeout: Some(Duration::from_secs(20)),
            failed_timeout: Duration::from_secs(25),
            send_buffer: 1024 * 1024,
            max_connections: 1024,
        }
    }
}

impl EndpointConfig {
    /// The ICE server URLs in effect.
    pub fn ice_server_urls(&self) -> Vec<String> {
        match &self.ice_servers {
            Some(v) => v.clone(),
            None => DEFAULT_ICE_SERVERS
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
        }
    }

    /// The STUN servers to resolve, skipping URLs str0m can't use (TURN,
    /// STUN over TLS) and malformed ones.
    pub fn stun_servers(&self) -> Vec<stun::StunServer> {
        self.ice_server_urls()
            .iter()
            .filter_map(|u| stun::parse_stun_url(u).ok())
            .collect()
    }
}
