// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `NetFetch` over real sockets: a TLS round trip against a local server
//! with a self-signed certificate, and a live request (ignored).

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use strims_directory::{FetchError, HttpFetch, HttpRequest, Method};
use strims_node::TokioNet;
use strims_node::fetch::{NetFetch, tls_client_config};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;

fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// A TLS server for `localhost` on a loopback port that answers one
/// request with `reply`: its port, its certificate, and the task that
/// returns the request it read.
async fn serve_once(reply: &'static [u8]) -> (u16, CertificateDer<'static>, JoinHandle<Vec<u8>>) {
    let key = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let cert = key.cert.der().clone();
    let private = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.signing_key.serialize_der()));
    let config = ServerConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert.clone()], private)
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let task = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let Ok(mut tls) = acceptor.accept(tcp).await else {
            return Vec::new();
        };
        let mut got = Vec::new();
        let mut buf = [0u8; 1024];
        while !got.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = tls.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            got.extend_from_slice(&buf[..n]);
        }
        tls.write_all(reply).await.unwrap();
        tls.shutdown().await.unwrap();
        got
    });
    (port, cert, task)
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

#[tokio::test]
async fn a_request_round_trips_over_tls_to_a_trusted_host() {
    let (port, cert, server) =
        serve_once(b"HTTP/1.1 200 OK\r\nContent-Length: 17\r\n\r\n{\"data\":[\"tls\"]}\n").await;
    let fetch = NetFetch::new(TokioNet, trusting(cert));
    let url = format!("https://localhost:{port}/helix/users?login=a");
    let res = fetch
        .fetch(HttpRequest::new(Method::Get, url, 1024))
        .await
        .unwrap();
    assert_eq!(res.status, 200);
    assert_eq!(res.body, b"{\"data\":[\"tls\"]}\n");
    let sent = String::from_utf8(server.await.unwrap()).unwrap();
    assert!(
        sent.starts_with(&format!(
            "GET /helix/users?login=a HTTP/1.1\r\nHost: localhost:{port}\r\nConnection: close\r\n"
        )),
        "{sent}"
    );
}

#[tokio::test]
async fn an_untrusted_certificate_fails_the_handshake() {
    let (port, _, server) = serve_once(b"HTTP/1.1 200 OK\r\n\r\n").await;
    let fetch = NetFetch::new(TokioNet, tls_client_config());
    let url = format!("https://localhost:{port}/");
    let res = fetch.fetch(HttpRequest::new(Method::Get, url, 1024)).await;
    assert!(matches!(res, Err(FetchError::Tls(_))), "{res:?}");
    server.await.unwrap();
}

#[tokio::test]
async fn an_ip_host_is_refused_before_connecting() {
    let fetch = NetFetch::new(TokioNet, tls_client_config());
    let res = fetch
        .fetch(HttpRequest::new(Method::Get, "https://127.0.0.1:1/", 1024))
        .await;
    assert!(matches!(res, Err(FetchError::InvalidRequest(_))), "{res:?}");
}

#[tokio::test]
#[ignore = "makes requests to the internet"]
async fn public_apis_answer_through_the_webpki_roots() {
    let fetch = NetFetch::new(TokioNet, tls_client_config());
    for url in [
        "https://www.googleapis.com/discovery/v1/apis?name=youtube&preferred=true",
        "https://api.angelthump.com/v3/streams/?username=psrngafk",
    ] {
        let res = fetch
            .fetch(HttpRequest::new(Method::Get, url, 1 << 20))
            .await
            .unwrap();
        let body = String::from_utf8_lossy(&res.body);
        assert_eq!(res.status, 200, "{url}: {:?} {body}", res.headers);
        let json = body.trim();
        assert!(
            json.starts_with('{') || json.starts_with('['),
            "{url}: {body}"
        );
    }
}
