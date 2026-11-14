// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `wss://` links on localhost sockets: a node whose WebSocket listener
//! has a certificate advertises `wss://` and takes TLS links, which a
//! node trusting the certificate dials, one trusting only the public roots
//! fails to verify unless the URL says `#insecure`, and a `ws://` dial
//! can't reach.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration as StdDuration;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use strims_crypto::key_from_seed;
use strims_node::{
    LinkKind, LinkTls, Node, NodeConfig, NodeError, NodeEvents, SystemClock, TokioNet,
    spawn_with_tls,
};

const STEP: StdDuration = StdDuration::from_secs(20);

fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// A self-signed certificate for `localhost` and 127.0.0.1, and a server
/// config serving it.
fn certificate() -> (CertificateDer<'static>, Arc<ServerConfig>) {
    let names = vec!["localhost".to_owned(), "127.0.0.1".to_owned()];
    let key = rcgen::generate_simple_self_signed(names).unwrap();
    let cert = key.cert.der().clone();
    let private = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.signing_key.serialize_der()));
    let config = ServerConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert.clone()], private)
        .unwrap();
    (cert, Arc::new(config))
}

fn trusting(cert: CertificateDer<'static>) -> Arc<ClientConfig> {
    let mut roots = RootCertStore::empty();
    roots.add(cert).unwrap();
    let config = ClientConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Arc::new(config)
}

async fn spawn(i: u8, listen: bool, tls: LinkTls) -> (Node, NodeEvents) {
    let mut config = NodeConfig::new(key_from_seed(&[i; 32]), u64::from(i));
    config.label = format!("node-{i}");
    if listen {
        config.ws_listen = Some("127.0.0.1:0".to_owned());
    }
    config.allow_insecure_ws = true;
    spawn_with_tls(config, TokioNet, SystemClock, tls)
        .await
        .unwrap()
}

async fn dial(node: &Node, uri: &str) -> Result<strims_node::HostId, NodeError> {
    tokio::time::timeout(STEP, node.dial(uri))
        .await
        .expect("dial timed out")
}

#[tokio::test]
async fn a_listener_with_a_certificate_takes_wss_links() {
    let (cert, server) = certificate();
    let (a, _a_events) = spawn(
        1,
        true,
        LinkTls {
            server: Some(server),
            ..LinkTls::default()
        },
    )
    .await;
    let uri = a.ws_uri().expect("listener uri").to_owned();
    let port = a.info().ws_addr.unwrap().port();
    let key = hex::encode(key_from_seed(&[1; 32]).public);
    assert_eq!(uri, format!("wss://127.0.0.1:{port}/{key}"));

    // Trusting the certificate, the dial links.
    let trusted = LinkTls {
        client: trusting(cert),
        server: None,
    };
    let (b, _b_events) = spawn(2, false, trusted).await;
    assert_eq!(dial(&b, &uri).await.unwrap(), a.host_id());
    let links = b.metrics().await.unwrap().links;
    assert_eq!(links.len(), 1);
    assert_eq!(links.first().unwrap().kind, LinkKind::Ws);

    // The public roots don't vouch for it, unless the URL is `#insecure`.
    let (c, _c_events) = spawn(3, false, LinkTls::default()).await;
    let err = dial(&c, &uri).await.unwrap_err();
    assert!(
        matches!(&err, NodeError::Dial(e) if e.contains("tls")),
        "{err:?}"
    );
    assert_eq!(
        dial(&c, &format!("{uri}#insecure")).await.unwrap(),
        a.host_id()
    );

    // A `ws://` dial gets no link from a TLS listener.
    let (d, _d_events) = spawn(4, false, LinkTls::default()).await;
    let plain = uri.replacen("wss://", "ws://", 1);
    assert!(matches!(dial(&d, &plain).await, Err(NodeError::Dial(_))));

    let m = a.metrics().await.unwrap();
    assert_eq!(m.links.len(), 2, "b's and c's links");
    assert!(
        m.driver.accept_errors >= 2,
        "c's failed handshake and d's ws://"
    );
}

#[tokio::test]
async fn a_listener_without_a_certificate_advertises_ws() {
    let (a, _events) = spawn(1, true, LinkTls::default()).await;
    assert!(a.ws_uri().unwrap().starts_with("ws://127.0.0.1:"));
}
