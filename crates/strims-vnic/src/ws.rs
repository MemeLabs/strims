// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! WebSocket link formats and options. The driver owns the sockets.
//!
//! - A listener serves the vnic link at the path `/<hex profile key>`
//!   ([`ws_path`]) and advertises `ws[s]://<host>:<port>/<hex profile key>`
//!   ([`format_ws_uri`]) as a `LinkDescription` with interface `"ws"`.
//! - A dialer accepts `wss`, and `ws` only when insecure links are allowed
//!   ([`ws_valid_scheme`]). A `#insecure` fragment turns off TLS
//!   certificate verification ([`WsDialTarget`]).
//! - The link is a byte stream over binary messages: each write is one
//!   binary message of at most [`WS_MTU`] bytes, and the reader concatenates
//!   message payloads, ignoring message boundaries. A text message is an
//!   error. No handshake runs before `PeerInit`; links use
//!   [`crate::LinkConfig::ws`].

use strims_base::time::Duration;

use crate::tcp::{TcpError, Url, hex};

/// The largest message, and so the link MTU.
pub const WS_MTU: usize = 64 * 1024;

/// The fragment that disables TLS verification when dialing.
pub const INSECURE_FRAGMENT: &str = "insecure";

/// Timeouts.
///
/// The driver must fail the link when a read waits longer than
/// `read_timeout` or a write takes longer than `write_timeout`, and send a
/// ping every `ping_interval`; a pong counts as a read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WsOptions {
    pub write_timeout: Duration,
    pub read_timeout: Duration,
    pub ping_interval: Duration,
}

impl Default for WsOptions {
    fn default() -> WsOptions {
        WsOptions {
            write_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(25),
            ping_interval: Duration::from_secs(20),
        }
    }
}

/// The HTTP path of a host's WebSocket listener.
pub fn ws_path(profile_key: &[u8]) -> String {
    format!("/{}", hex(profile_key))
}

/// Where a WebSocket listener is reachable.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WsListenAddress {
    /// The listen address, `host:port`.
    pub address: String,
    /// Replaces the host in the advertised URI.
    pub public_hostname: String,
    /// Replaces the port in the advertised URI when non-zero.
    pub public_port: u16,
    /// Advertise `wss`.
    pub secure: bool,
}

/// Formats the URI a WebSocket listener advertises. Without a public
/// hostname the listen host must be a hostname or a specified IP.
pub fn format_ws_uri(addr: &WsListenAddress, profile_key: &[u8]) -> Result<String, TcpError> {
    let (mut host, mut port) = split_host_port(&addr.address)
        .ok_or_else(|| TcpError::InvalidAddress(addr.address.clone()))?;
    let public_port;
    if addr.public_port != 0 {
        public_port = addr.public_port.to_string();
        port = &public_port;
    }
    if !addr.public_hostname.is_empty() {
        host = &addr.public_hostname;
    } else {
        let ip = host.parse::<std::net::IpAddr>();
        let invalid = match ip {
            Ok(ip) => ip.is_unspecified(),
            Err(_) => host.is_empty(),
        };
        if invalid {
            return Err(TcpError::InvalidIp(format!("invalid hostname: {host}")));
        }
    }
    let scheme = if addr.secure { "wss" } else { "ws" };
    let host = join_host_port(host, port).replace('%', "%25");
    Ok(format!("{scheme}://{host}{}", ws_path(profile_key)))
}

/// Whether a dialer accepts `scheme`: `wss`, and `ws` with
/// `allow_insecure`. The browser build passes `false`.
pub fn ws_valid_scheme(scheme: &str, allow_insecure: bool) -> bool {
    scheme == "wss" || (allow_insecure && scheme == "ws")
}

/// What a WebSocket dialer connects to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WsDialTarget {
    /// The URI without its fragment, as sent in the HTTP request.
    pub url: String,
    /// `wss`.
    pub secure: bool,
    /// The fragment is `insecure`: skip TLS certificate verification.
    pub insecure_skip_verify: bool,
}

impl WsDialTarget {
    pub fn parse(uri: &str) -> Result<WsDialTarget, TcpError> {
        let u = Url::parse(uri)?;
        let secure = match u.scheme {
            "wss" => true,
            "ws" => false,
            s => return Err(TcpError::InvalidUri(format!("unsupported scheme {s}"))),
        };
        Ok(WsDialTarget {
            url: uri.split_once('#').map_or(uri, |(u, _)| u).to_owned(),
            secure,
            insecure_skip_verify: u.fragment == Some(INSECURE_FRAGMENT),
        })
    }
}

/// Splits `host:port`, with an IPv6 host in brackets.
fn split_host_port(s: &str) -> Option<(&str, &str)> {
    if let Some(rest) = s.strip_prefix('[') {
        let (host, rest) = rest.split_once(']')?;
        let port = rest.strip_prefix(':')?;
        return Some((host, port));
    }
    let (host, port) = s.rsplit_once(':')?;
    if host.contains(':') {
        return None;
    }
    Some((host, port))
}

/// Joins a host and port, bracketing an IPv6 host.
fn join_host_port(host: &str, port: &str) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    const KEY: [u8; 2] = [0x01, 0xfe];

    fn addr(address: &str, host: &str, port: u16, secure: bool) -> WsListenAddress {
        WsListenAddress {
            address: address.to_owned(),
            public_hostname: host.to_owned(),
            public_port: port,
            secure,
        }
    }

    #[test]
    fn formats_listener_uris() {
        assert_eq!(ws_path(&KEY), "/01fe");
        let f = |a: WsListenAddress| format_ws_uri(&a, &KEY);
        assert_eq!(
            f(addr("1.2.3.4:80", "", 0, false)).unwrap(),
            "ws://1.2.3.4:80/01fe"
        );
        assert_eq!(
            f(addr("example.com:80", "", 443, true)).unwrap(),
            "wss://example.com:443/01fe"
        );
        assert_eq!(
            f(addr("0.0.0.0:80", "strims.gg", 0, true)).unwrap(),
            "wss://strims.gg:80/01fe"
        );
        assert_eq!(
            f(addr("[::1]:80", "", 0, false)).unwrap(),
            "ws://[::1]:80/01fe"
        );
        assert!(f(addr("0.0.0.0:80", "", 0, false)).is_err());
        assert!(f(addr(":80", "", 0, false)).is_err());
        assert!(f(addr("[::]:80", "", 0, false)).is_err());
        assert!(f(addr("nope", "", 0, false)).is_err());
    }

    #[test]
    fn dial_targets() {
        let t = WsDialTarget::parse("wss://h:1/01fe#insecure").unwrap();
        assert_eq!(t.url, "wss://h:1/01fe");
        assert!(t.secure && t.insecure_skip_verify);
        let t = WsDialTarget::parse("ws://h:1/01fe").unwrap();
        assert!(!t.secure && !t.insecure_skip_verify);
        assert!(WsDialTarget::parse("tcp://h:1/01fe").is_err());
        assert!(ws_valid_scheme("wss", false));
        assert!(!ws_valid_scheme("ws", false));
        assert!(ws_valid_scheme("ws", true));
    }
}
