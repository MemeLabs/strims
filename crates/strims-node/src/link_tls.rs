// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! TLS for WebSocket links ([`LinkTls`]).
//!
//! - A `wss://` dial verifies the listener's certificate with
//!   [`LinkTls::client`], and names the URL's host to it (SNI and the
//!   certificate check alike): a DNS name or an IP address. A URL with the
//!   `#insecure` fragment accepts any certificate, though the handshake's
//!   signatures are still checked: the link is encrypted, to a listener
//!   nothing vouches for. vnic's `PeerInit` authenticates the peer either
//!   way.
//! - With [`LinkTls::server`], the WebSocket listener takes TLS before the
//!   upgrade, and its link URIs are `wss://`. Without it, `ws://`.

use std::sync::{Arc, OnceLock};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, ServerConfig, SignatureScheme};

use crate::fetch::tls_client_config;

/// The TLS of a node's WebSocket links: the client config its `wss://`
/// dials verify with, and the listener's server config. See the
/// [module docs](self).
#[derive(Clone, Debug)]
pub struct LinkTls {
    /// The roots and protocols `wss://` dials verify listeners with.
    pub client: Arc<ClientConfig>,
    /// The listener's certificate: with it, the listener takes `wss://`
    /// links and advertises `wss://` URIs.
    pub server: Option<Arc<ServerConfig>>,
}

impl Default for LinkTls {
    /// Dials verify with the public roots ([`tls_client_config`]); the
    /// listener takes `ws://`.
    fn default() -> Self {
        static PUBLIC: OnceLock<Arc<ClientConfig>> = OnceLock::new();
        LinkTls {
            client: PUBLIC.get_or_init(tls_client_config).clone(),
            server: None,
        }
    }
}

impl LinkTls {
    /// `client` accepting any certificate, for `#insecure` dials.
    pub(crate) fn insecure_client(&self) -> Arc<ClientConfig> {
        let mut config = (*self.client).clone();
        let algorithms = config.crypto_provider().signature_verification_algorithms;
        config
            .dangerous()
            .set_certificate_verifier(Arc::new(AnyCertificate(algorithms)));
        Arc::new(config)
    }
}

/// The host a `wss://` URL names, as TLS names it: a DNS name or an IP
/// address (IPv6 without its brackets).
pub(crate) fn server_name(url: &str) -> Result<ServerName<'static>, String> {
    let uri: tokio_tungstenite::tungstenite::http::Uri =
        url.parse().map_err(|e| format!("invalid url {url}: {e}"))?;
    let host = uri.host().ok_or_else(|| format!("no host in {url}"))?;
    let host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    ServerName::try_from(host.to_owned()).map_err(|e| format!("invalid host {host}: {e}"))
}

/// Accepts every certificate, and checks the handshake's signatures with
/// the provider's algorithms.
#[derive(Debug)]
struct AnyCertificate(WebPkiSupportedAlgorithms);

impl ServerCertVerifier for AnyCertificate {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.0)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.0)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn server_names_are_the_urls_hosts() {
        let name = |u| server_name(u).unwrap().to_str().into_owned();
        assert_eq!(name("wss://a.b:8082/k"), "a.b");
        assert_eq!(name("wss://10.0.0.9:8082/k"), "10.0.0.9");
        assert_eq!(name("wss://[::1]:8082/k"), "::1");
        assert!(matches!(
            server_name("wss://10.0.0.9/k").unwrap(),
            ServerName::IpAddress(_)
        ));
        assert!(server_name("/k").is_err());
    }
}
