// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! TCP links: the connection preamble, the listener's mux and the URIs. The
//! driver owns the sockets.
//!
//! A TCP connection carries, in order:
//!
//! 1. from the dialer, a `TCPMuxInit` (protocol version 1, the listener's
//!    32-byte ed25519 profile key) in [`crate::stream`] framing, which the
//!    listener's [`TcpMux`] uses to pick the host the connection is for;
//! 2. the AES link handshake both ways ([`crate::aes_link`]), using the
//!    hosts' profile keys converted to X25519; the dialer checks that the
//!    listener's key is the X25519 form of the profile key it dialed;
//! 3. the encrypted vnic link: `PeerInit` both ways, then frames.
//!
//! [`crate::LinkConfig::tcp_dial`] and [`crate::LinkConfig::tcp_accept`]
//! make a [`crate::Host`] run steps 1 (dialer side) to 3. The listener runs
//! step 1 with a [`TcpMuxAcceptor`] before it knows which host to add the
//! link to.
//!
//! The driver:
//!
//! - dials a [`TcpUri`]: it resolves [`TcpUri::host`] and adds the link with
//!   [`crate::LinkConfig::tcp_dial`]`(&uri.peer_key)`;
//! - on accept, feeds bytes to a [`TcpMuxAcceptor`] until it yields a
//!   profile key, finds the host with [`TcpMux::route`] (closing the
//!   connection for an unknown key), adds the link with
//!   [`crate::LinkConfig::tcp_accept`], and passes it the bytes that followed
//!   the init;
//! - advertises [`format_tcp_uri`] as a `LinkDescription` with interface
//!   `"tcp"`, and nothing when it fails;
//! - applies [`TcpOptions`] to every socket.

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};

use strims_base::time::Duration;
use strims_proto::vnic::v1::TcpMuxInit;

use crate::PROTOCOL_VERSION;
use crate::stream::{MessageReader, StreamError, decode_stream_message, encode_stream_message};

/// The TCP link MTU.
pub const TCP_MTU: usize = u16::MAX as usize;

/// The scheme of TCP link URIs.
pub const TCP_SCHEME: &str = "tcp";

/// Socket options and deadlines.
///
/// The driver must:
/// - enable keepalive with `keep_alive_period` and set the socket buffer
///   sizes on every connection, dialed or accepted;
/// - fail the link when a single read waits longer than `read_timeout` (the
///   deadline restarts before every read, so this is an idle timeout on
///   input), and when a single write takes longer than `write_timeout`;
/// - apply `read_timeout` to the [`TcpMuxAcceptor`] stage too, which comes
///   before [`crate::Host::add_link`] starts the handshake deadline.
///
/// Peers that send keepalives keep an idle link inside `read_timeout`; see
/// [`crate::Peer::sends_keepalives`] for peers that don't.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TcpOptions {
    pub keep_alive_period: Duration,
    pub read_buffer_size: usize,
    pub write_buffer_size: usize,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
}

impl Default for TcpOptions {
    fn default() -> TcpOptions {
        TcpOptions {
            keep_alive_period: Duration::from_secs(20),
            read_buffer_size: 2 * 1024 * 1024,
            write_buffer_size: 2 * 1024 * 1024,
            read_timeout: Duration::from_secs(25),
            write_timeout: Duration::from_secs(5),
        }
    }
}

/// Errors from the TCP mux and URIs.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TcpError {
    #[error("reading peer init failed: {0}")]
    Read(StreamError),
    #[error("writing tcp mux init failed: {0}")]
    Write(StreamError),
    #[error("invalid peer key size")]
    InvalidPeerKeySize,
    /// No host is registered for the key; the driver closes the connection.
    #[error("peer key not found")]
    PeerKeyNotFound,
    #[error("invalid ip: {0}")]
    InvalidIp(String),
    #[error("invalid address: {0}")]
    InvalidAddress(String),
    #[error("invalid uri: {0}")]
    InvalidUri(String),
}

/// Appends the dialer's `TCPMuxInit`.
pub fn encode_tcp_mux_init(peer_key: &[u8; 32], out: &mut Vec<u8>) -> Result<(), TcpError> {
    let init = TcpMuxInit {
        protocol_version: PROTOCOL_VERSION,
        peer_key: peer_key.to_vec(),
    };
    encode_stream_message(&init, out).map_err(TcpError::Write)
}

/// Reads the `TCPMuxInit` at the start of an accepted connection. Buffers
/// at most `2 + 65535` bytes.
#[derive(Debug, Default)]
pub struct TcpMuxAcceptor {
    reader: MessageReader,
}

impl TcpMuxAcceptor {
    pub fn new() -> TcpMuxAcceptor {
        TcpMuxAcceptor::default()
    }

    /// Feeds bytes read from the connection. Returns the bytes consumed and,
    /// once the init is complete, the profile key it names. Bytes after the
    /// init are not consumed: pass them to the host's
    /// [`crate::Host::handle_link_input`] after
    /// [`crate::Host::add_link`]`(now, LinkConfig::tcp_accept())`.
    pub fn handle_input(&mut self, data: &[u8]) -> Result<(usize, Option<[u8; 32]>), TcpError> {
        let pushed = self.reader.push(data);
        let Some(body) = pushed.message else {
            return Ok((pushed.consumed, None));
        };
        let consumed = pushed.consumed;
        let init: TcpMuxInit = decode_stream_message(body).map_err(TcpError::Read)?;
        let key: [u8; 32] = init
            .peer_key
            .as_slice()
            .try_into()
            .map_err(|_| TcpError::InvalidPeerKeySize)?;
        Ok((consumed, Some(key)))
    }
}

/// Routes accepted connections to hosts by profile key, so several hosts
/// can share a listener. `H` is whatever the driver uses to find a host.
#[derive(Debug)]
pub struct TcpMux<H> {
    /// Bounded by the hosts the driver registers.
    handlers: BTreeMap<[u8; 32], H>,
}

impl<H> Default for TcpMux<H> {
    fn default() -> TcpMux<H> {
        TcpMux {
            handlers: BTreeMap::new(),
        }
    }
}

impl<H> TcpMux<H> {
    pub fn new() -> TcpMux<H> {
        TcpMux::default()
    }

    /// Routes connections naming `profile_key` to `h`.
    pub fn handle(&mut self, profile_key: [u8; 32], h: H) {
        self.handlers.insert(profile_key, h);
    }

    /// Stops routing `profile_key` and returns its handler.
    pub fn stop_handling(&mut self, profile_key: &[u8; 32]) -> Option<H> {
        self.handlers.remove(profile_key)
    }

    /// The handler for a connection's init key.
    pub fn route(&self, peer_key: &[u8; 32]) -> Result<&H, TcpError> {
        self.handlers.get(peer_key).ok_or(TcpError::PeerKeyNotFound)
    }
}

/// Formats the URI a TCP listener advertises in its `LinkDescription`:
/// `tcp://<ip>:<port>/<hex profile key>`. `address` is the listen address
/// (an IP literal and port); a non-empty `host_ip` replaces its IP. An
/// unspecified IP is an error. An IPv6 zone is a numeric scope id, written
/// `%25<id>`.
pub fn format_tcp_uri(
    address: &str,
    host_ip: &str,
    profile_key: &[u8],
) -> Result<String, TcpError> {
    let mut ap: SocketAddr = address
        .parse()
        .map_err(|_| TcpError::InvalidAddress(address.to_owned()))?;
    if !host_ip.is_empty() {
        let ip: IpAddr = host_ip
            .parse()
            .map_err(|_| TcpError::InvalidIp(host_ip.to_owned()))?;
        ap.set_ip(ip);
    }
    if ap.ip().is_unspecified() {
        return Err(TcpError::InvalidIp(ap.ip().to_string()));
    }
    let host = ap.to_string().replace('%', "%25");
    Ok(format!("{TCP_SCHEME}://{host}/{}", hex(profile_key)))
}

/// A parsed TCP link URI, `tcp://<host:port>/<hex profile key>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TcpUri {
    /// `host:port`, for the driver to resolve.
    pub host: String,
    /// The listener's ed25519 profile key: the mux key, and (as X25519) the
    /// key its AES handshake must present.
    pub peer_key: [u8; 32],
}

impl TcpUri {
    /// Parses a URI. Every leading `/` of the path is trimmed, and the key's
    /// hex may be either case.
    pub fn parse(uri: &str) -> Result<TcpUri, TcpError> {
        let u = Url::parse(uri)?;
        if !u.scheme.eq_ignore_ascii_case(TCP_SCHEME) {
            return Err(TcpError::InvalidUri(format!(
                "unsupported scheme {}",
                u.scheme
            )));
        }
        let key = unhex(u.path.trim_start_matches('/'))
            .ok_or_else(|| TcpError::InvalidUri("invalid peer key".to_owned()))?;
        let peer_key: [u8; 32] = key
            .as_slice()
            .try_into()
            .map_err(|_| TcpError::InvalidPeerKeySize)?;
        Ok(TcpUri {
            host: u.host.to_owned(),
            peer_key,
        })
    }

    /// The URI form.
    pub fn to_uri(&self) -> String {
        format!("{TCP_SCHEME}://{}/{}", self.host, hex(&self.peer_key))
    }
}

/// The parts of a URL the link URIs use.
#[derive(Debug)]
pub(crate) struct Url<'a> {
    pub(crate) scheme: &'a str,
    /// Authority, still escaped: the driver's resolver unescapes an IPv6
    /// zone's `%25`.
    pub(crate) host: &'a str,
    /// Percent-decoded path.
    pub(crate) path: String,
    pub(crate) fragment: Option<&'a str>,
}

impl<'a> Url<'a> {
    pub(crate) fn parse(uri: &'a str) -> Result<Url<'a>, TcpError> {
        let bad = || TcpError::InvalidUri(uri.to_owned());
        let (rest, fragment) = match uri.split_once('#') {
            Some((r, f)) => (r, Some(f)),
            None => (uri, None),
        };
        let (scheme, rest) = rest.split_once("://").ok_or_else(bad)?;
        if scheme.is_empty()
            || !scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            || !scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        {
            return Err(bad());
        }
        let rest = rest.split_once('?').map_or(rest, |(r, _)| r);
        let (host, path) = match rest.find('/') {
            Some(i) => rest.split_at(i),
            None => (rest, ""),
        };
        Ok(Url {
            scheme,
            host,
            path: percent_decode(path).ok_or_else(bad)?,
            fragment,
        })
    }
}

fn percent_decode(s: &str) -> Option<String> {
    let mut out = Vec::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hi = hex_val(bytes.next()?)?;
            let lo = hex_val(bytes.next()?)?;
            out.push(hi << 4 | lo);
        } else {
            out.push(b);
        }
    }
    String::from_utf8(out).ok()
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Lowercase hex.
pub(crate) fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        for d in [x >> 4, x & 15] {
            s.push(char::from_digit(u32::from(d), 16).unwrap_or('0'));
        }
    }
    s
}

/// Decodes hex of either case.
pub(crate) fn unhex(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    let (pairs, rest) = b.as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    pairs
        .iter()
        .map(|&[hi, lo]| Some(hex_val(hi)? << 4 | hex_val(lo)?))
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    const KEY: [u8; 32] = [0xab; 32];

    #[test]
    fn uri_rejects_unspecified() {
        assert!(matches!(
            format_tcp_uri("0.0.0.0:8080", "", &KEY),
            Err(TcpError::InvalidIp(_))
        ));
        assert!(matches!(
            format_tcp_uri("[::]:8080", "", &KEY),
            Err(TcpError::InvalidIp(_))
        ));
        let u = format_tcp_uri("0.0.0.0:8080", "10.0.0.1", &KEY).unwrap();
        assert_eq!(u, format!("tcp://10.0.0.1:8080/{}", "ab".repeat(32)));
        let u = format_tcp_uri("[::1]:9", "", &KEY).unwrap();
        assert_eq!(u, format!("tcp://[::1]:9/{}", "ab".repeat(32)));
        assert!(format_tcp_uri("localhost:80", "", &KEY).is_err());
    }

    #[test]
    fn uri_round_trip() {
        let u = format_tcp_uri("1.2.3.4:5", "", &KEY).unwrap();
        let p = TcpUri::parse(&u).unwrap();
        assert_eq!(p.host, "1.2.3.4:5");
        assert_eq!(p.peer_key, KEY);
        assert_eq!(p.to_uri(), u);
        // Every leading slash is trimmed, and upper-case hex is accepted.
        let p = TcpUri::parse(&format!("tcp://h:1//{}", "AB".repeat(32))).unwrap();
        assert_eq!(p.peer_key, KEY);
        assert_eq!(
            TcpUri::parse("tcp://h:1/abcd"),
            Err(TcpError::InvalidPeerKeySize)
        );
        assert!(TcpUri::parse("tcp://h:1/zz").is_err());
        assert!(TcpUri::parse("ws://h:1/ab").is_err());
    }

    #[test]
    fn mux_init_and_routing() {
        let mut wire = Vec::new();
        encode_tcp_mux_init(&KEY, &mut wire).unwrap();
        wire.extend_from_slice(b"aes");
        let mut a = TcpMuxAcceptor::new();
        let (n, k) = a.handle_input(&wire[..10]).unwrap();
        assert_eq!((n, k), (10, None));
        let (m, k) = a.handle_input(&wire[10..]).unwrap();
        assert_eq!(k, Some(KEY));
        assert_eq!(&wire[10 + m..], b"aes");

        let mut mux = TcpMux::new();
        mux.handle(KEY, 7);
        assert_eq!(mux.route(&KEY), Ok(&7));
        assert_eq!(mux.route(&[0; 32]), Err(TcpError::PeerKeyNotFound));
        mux.stop_handling(&KEY);
        assert!(mux.route(&KEY).is_err());

        let bad = TcpMuxInit {
            protocol_version: 1,
            peer_key: vec![1; 31],
        };
        let mut wire = Vec::new();
        encode_stream_message(&bad, &mut wire).unwrap();
        assert_eq!(
            TcpMuxAcceptor::new().handle_input(&wire),
            Err(TcpError::InvalidPeerKeySize)
        );
    }
}
