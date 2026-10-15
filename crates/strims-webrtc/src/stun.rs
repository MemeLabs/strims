// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The STUN client half of ICE gathering: binding requests to STUN servers
//! for server-reflexive candidates (RFC 8489), and `stun:` ICE server URLs
//! (RFC 7064).
//!
//! str0m discovers no addresses itself, so server-reflexive gathering lives
//! here. Only the binding method is implemented: no authentication, no
//! TURN.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// RFC 8489 magic cookie.
pub const MAGIC_COOKIE: u32 = 0x2112_A442;
/// A STUN header.
pub const HEADER_LEN: usize = 20;
/// A binding request without attributes.
pub const BINDING_REQUEST_LEN: usize = HEADER_LEN;

const BINDING_REQUEST: u16 = 0x0001;
const BINDING_SUCCESS: u16 = 0x0101;
const BINDING_ERROR: u16 = 0x0111;
const ATTR_MAPPED_ADDRESS: u16 = 0x0001;
const ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;

/// The default port of `stun:` URLs. RFC 7064.
pub const DEFAULT_STUN_PORT: u16 = 3478;

/// A STUN transaction id.
pub type TransactionId = [u8; 12];

/// Writes a binding request with no attributes.
pub fn encode_binding_request(txn: &TransactionId) -> [u8; BINDING_REQUEST_LEN] {
    let mut b = [0u8; BINDING_REQUEST_LEN];
    let (msg_type, rest) = b.split_at_mut(2);
    msg_type.copy_from_slice(&BINDING_REQUEST.to_be_bytes());
    // The length stays 0.
    let (cookie, id) = rest.split_at_mut(2).1.split_at_mut(4);
    cookie.copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    id.copy_from_slice(txn);
    b
}

/// A binding response's outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingResponse {
    /// The server saw the request from this address.
    Mapped(SocketAddr),
    /// An error response, or a success without a usable address.
    Failed,
}

/// Recognizes a binding response (RFC 8489 §6.3.1 checks: top bits zero,
/// magic cookie, length consistent with the datagram, 4-byte aligned).
/// Returns `None` for anything else, including binding requests and other
/// STUN methods.
pub fn decode_binding_response(data: &[u8]) -> Option<(TransactionId, BindingResponse)> {
    let header = data.get(..HEADER_LEN)?;
    let msg_type = u16::from_be_bytes([*header.first()?, *header.get(1)?]);
    let len = usize::from(u16::from_be_bytes([*header.get(2)?, *header.get(3)?]));
    let cookie = u32::from_be_bytes(header.get(4..8)?.try_into().ok()?);
    if cookie != MAGIC_COOKIE || len % 4 != 0 || data.len() != HEADER_LEN + len {
        return None;
    }
    let txn: TransactionId = header.get(8..20)?.try_into().ok()?;
    let outcome = match msg_type {
        BINDING_SUCCESS => {
            let attrs = data.get(HEADER_LEN..)?;
            match mapped_address(attrs, &txn) {
                Some(a) => BindingResponse::Mapped(a),
                None => BindingResponse::Failed,
            }
        }
        BINDING_ERROR => BindingResponse::Failed,
        _ => return None,
    };
    Some((txn, outcome))
}

/// XOR-MAPPED-ADDRESS, else the RFC 3489 MAPPED-ADDRESS.
fn mapped_address(mut attrs: &[u8], txn: &TransactionId) -> Option<SocketAddr> {
    let mut plain = None;
    while attrs.len() >= 4 {
        let t = u16::from_be_bytes([*attrs.first()?, *attrs.get(1)?]);
        let l = usize::from(u16::from_be_bytes([*attrs.get(2)?, *attrs.get(3)?]));
        let value = attrs.get(4..4 + l)?;
        match t {
            ATTR_XOR_MAPPED_ADDRESS => return decode_address(value, Some(txn)),
            ATTR_MAPPED_ADDRESS => plain = decode_address(value, None),
            _ => {}
        }
        let padded = (4 + l + 3) & !3;
        attrs = attrs.get(padded..).unwrap_or(&[]);
    }
    plain
}

fn decode_address(v: &[u8], xor: Option<&TransactionId>) -> Option<SocketAddr> {
    let family = *v.get(1)?;
    let mut port = u16::from_be_bytes([*v.get(2)?, *v.get(3)?]);
    let cookie = MAGIC_COOKIE.to_be_bytes();
    if xor.is_some() {
        port ^= (MAGIC_COOKIE >> 16) as u16;
    }
    let ip = match family {
        0x01 => {
            let mut a: [u8; 4] = v.get(4..8)?.try_into().ok()?;
            if xor.is_some() {
                for (b, k) in a.iter_mut().zip(cookie) {
                    *b ^= k;
                }
            }
            IpAddr::V4(Ipv4Addr::from(a))
        }
        0x02 => {
            let mut a: [u8; 16] = v.get(4..20)?.try_into().ok()?;
            if let Some(txn) = xor {
                for (b, k) in a.iter_mut().zip(cookie.iter().chain(txn.iter())) {
                    *b ^= k;
                }
            }
            IpAddr::V6(Ipv6Addr::from(a))
        }
        _ => return None,
    };
    Some(SocketAddr::new(ip, port))
}

/// A STUN server named by an ICE server URL, to be resolved by the driver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StunServer {
    /// A hostname or IP literal (IPv6 without brackets).
    pub host: String,
    pub port: u16,
}

/// Why an ICE server URL is unusable.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum IceUrlError {
    #[error("malformed ice server url: {0}")]
    Malformed(String),
    /// `turn:`, `turns:` and `stuns:`: str0m has no TURN or STUN-over-TLS
    /// client.
    #[error("unsupported ice server scheme: {0}")]
    Unsupported(String),
}

/// Parses a `stun:host[:port]` URL (RFC 7064).
pub fn parse_stun_url(url: &str) -> Result<StunServer, IceUrlError> {
    let (scheme, rest) = url
        .split_once(':')
        .ok_or_else(|| IceUrlError::Malformed(url.to_owned()))?;
    match scheme.to_ascii_lowercase().as_str() {
        "stun" => {}
        "stuns" | "turn" | "turns" => return Err(IceUrlError::Unsupported(url.to_owned())),
        _ => return Err(IceUrlError::Malformed(url.to_owned())),
    }
    // RFC 7064: no "//" and no query for stun URLs.
    if rest.starts_with("//") || rest.contains('?') {
        return Err(IceUrlError::Malformed(url.to_owned()));
    }
    let (host, port) = if let Some(v6) = rest.strip_prefix('[') {
        let (h, after) = v6
            .split_once(']')
            .ok_or_else(|| IceUrlError::Malformed(url.to_owned()))?;
        let port = match after.strip_prefix(':') {
            Some(p) => Some(p),
            None if after.is_empty() => None,
            None => return Err(IceUrlError::Malformed(url.to_owned())),
        };
        (h, port)
    } else {
        match rest.rsplit_once(':') {
            Some((h, p)) => (h, Some(p)),
            None => (rest, None),
        }
    };
    if host.is_empty() {
        return Err(IceUrlError::Malformed(url.to_owned()));
    }
    let port = match port {
        Some(p) => p
            .parse::<u16>()
            .map_err(|_| IceUrlError::Malformed(url.to_owned()))?,
        None => DEFAULT_STUN_PORT,
    };
    Ok(StunServer {
        host: host.to_owned(),
        port,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    /// A success response carrying `addr` as XOR-MAPPED-ADDRESS, as a
    /// server writes it.
    pub(crate) fn success(txn: &TransactionId, addr: SocketAddr) -> Vec<u8> {
        let mut attr = vec![0u8, 0];
        let port = addr.port() ^ (MAGIC_COOKIE >> 16) as u16;
        let cookie = MAGIC_COOKIE.to_be_bytes();
        match addr.ip() {
            IpAddr::V4(ip) => {
                attr[1] = 1;
                attr.extend_from_slice(&port.to_be_bytes());
                attr.extend(ip.octets().iter().zip(cookie).map(|(b, k)| b ^ k));
            }
            IpAddr::V6(ip) => {
                attr[1] = 2;
                attr.extend_from_slice(&port.to_be_bytes());
                let key: Vec<u8> = cookie.iter().chain(txn.iter()).copied().collect();
                attr.extend(ip.octets().iter().zip(key).map(|(b, k)| b ^ k));
            }
        }
        let mut m = Vec::new();
        m.extend_from_slice(&BINDING_SUCCESS.to_be_bytes());
        m.extend_from_slice(&((4 + attr.len()) as u16).to_be_bytes());
        m.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        m.extend_from_slice(txn);
        m.extend_from_slice(&ATTR_XOR_MAPPED_ADDRESS.to_be_bytes());
        m.extend_from_slice(&(attr.len() as u16).to_be_bytes());
        m.extend_from_slice(&attr);
        m
    }

    #[test]
    fn request_layout() {
        let txn = [7u8; 12];
        let r = encode_binding_request(&txn);
        assert_eq!(hex::encode(&r[..8]), "000100002112a442");
        assert_eq!(&r[8..], &txn);
        // A request is not a response.
        assert_eq!(decode_binding_response(&r), None);
    }

    #[test]
    fn xor_mapped_addresses() {
        let txn = *b"abcdefghijkl";
        for a in ["203.0.113.9:40000", "[2001:db8::1]:3478"] {
            let a: SocketAddr = a.parse().unwrap();
            let m = success(&txn, a);
            assert_eq!(
                decode_binding_response(&m),
                Some((txn, BindingResponse::Mapped(a)))
            );
            // Truncated or padded datagrams are not responses.
            assert_eq!(decode_binding_response(&m[..m.len() - 1]), None);
            let mut long = m.clone();
            long.push(0);
            assert_eq!(decode_binding_response(&long), None);
        }
        // RFC 5769 §2.2 sample response (IPv4), XOR-MAPPED-ADDRESS
        // 192.0.2.1:32853, with SOFTWARE, MESSAGE-INTEGRITY and FINGERPRINT.
        let v = hex::decode(concat!(
            "0101003c2112a442b7e7a701bc34d686fa87dfae",
            "8022000b7465737420766563746f7220",
            "002000080001a147e112a643",
            "000800142b91f599fd9e90c38c7489f92af9ba53f06be7d7",
            "80280004c07d4c96"
        ))
        .unwrap();
        assert_eq!(
            decode_binding_response(&v).map(|(_, r)| r),
            Some(BindingResponse::Mapped("192.0.2.1:32853".parse().unwrap()))
        );
    }

    #[test]
    fn stun_urls() {
        assert_eq!(
            parse_stun_url("stun:stun.l.google.com:19302"),
            Ok(StunServer {
                host: "stun.l.google.com".into(),
                port: 19302
            })
        );
        assert_eq!(parse_stun_url("stun:example.org").unwrap().port, 3478);
        assert_eq!(
            parse_stun_url("stun:[2001:db8::1]:1234"),
            Ok(StunServer {
                host: "2001:db8::1".into(),
                port: 1234
            })
        );
        assert!(matches!(
            parse_stun_url("turn:example.org?transport=udp"),
            Err(IceUrlError::Unsupported(_))
        ));
        for bad in ["stun:", "stun://x", "http:x", "stun:x:y", "stun:[::1"] {
            assert!(
                matches!(parse_stun_url(bad), Err(IceUrlError::Malformed(_))),
                "{bad}"
            );
        }
        for url in crate::DEFAULT_ICE_SERVERS {
            assert_eq!(parse_stun_url(url).unwrap().port, 19302);
        }
    }
}
