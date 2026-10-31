// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Certificates for a service's TLS, and clients that trust them: HTTPS
//! requests over HTTP/1.1 and HTTP/2, and the UI's `wss://` connections.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use data_encoding::BASE64;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, RootCertStore};
use strims_svc::Svc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tokio_tungstenite::tungstenite;

use super::video::{Reply, STEP};
use super::{Ui, ui, ws_config};

/// A self-signed certificate for `names`, DNS names or IP addresses.
pub struct TestCert {
    pub cert_pem: String,
    pub key_pem: String,
    pub der: CertificateDer<'static>,
}

impl TestCert {
    pub fn new(names: &[&str]) -> TestCert {
        let names: Vec<String> = names.iter().map(|n| (*n).to_owned()).collect();
        let key = rcgen::generate_simple_self_signed(names).unwrap();
        TestCert {
            cert_pem: pem("CERTIFICATE", key.cert.der()),
            key_pem: pem("PRIVATE KEY", &key.signing_key.serialize_der()),
            der: key.cert.der().clone(),
        }
    }

    /// Writes the certificate and its key to `cert.pem` and `key.pem` in
    /// `dir`. The key's file ends with `pad` blank lines, so files written
    /// in one clock tick differ in length.
    pub fn write(&self, dir: &Path, pad: usize) -> (PathBuf, PathBuf) {
        let (cert, key) = (dir.join("cert.pem"), dir.join("key.pem"));
        std::fs::write(&cert, &self.cert_pem).unwrap();
        std::fs::write(&key, format!("{}{}", self.key_pem, "\n".repeat(pad))).unwrap();
        (cert, key)
    }

    /// A client trusting this certificate, offering `alpn`.
    pub fn client(&self, alpn: &[&[u8]]) -> TlsConnector {
        let mut roots = RootCertStore::empty();
        roots.add(self.der.clone()).unwrap();
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth();
        config.alpn_protocols = alpn.iter().map(|p| p.to_vec()).collect();
        TlsConnector::from(Arc::new(config))
    }
}

fn pem(label: &str, der: &[u8]) -> String {
    let b64 = BASE64.encode(der);
    let mut out = format!("-----BEGIN {label}-----\n");
    for line in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(line).unwrap());
        out.push('\n');
    }
    out + &format!("-----END {label}-----\n")
}

/// A directory of its own for test `name`'s files.
pub fn dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("strims-svc-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Connects to `addr` over TLS as `name`.
pub async fn connect(
    client: &TlsConnector,
    addr: SocketAddr,
    name: &str,
) -> std::io::Result<TlsStream<TcpStream>> {
    let tcp = TcpStream::connect(addr).await?;
    client
        .connect(ServerName::try_from(name.to_owned()).unwrap(), tcp)
        .await
}

/// Sends an HTTP/1.1 request of `path` over TLS to `addr`, as `name`, and
/// reads the answer until the server ends the connection.
pub async fn request(
    client: &TlsConnector,
    addr: SocketAddr,
    name: &str,
    method: &str,
    path: &str,
) -> Reply {
    let mut s = connect(client, addr, name).await.unwrap();
    let req = format!("{method} {path} HTTP/1.1\r\nHost: {name}\r\nAccept: */*\r\n\r\n");
    s.write_all(req.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    tokio::time::timeout(STEP, s.read_to_end(&mut raw))
        .await
        .expect("the server ends the connection")
        .expect("the answer ends with close_notify");
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut head = httparse::Response::new(&mut headers);
    let httparse::Status::Complete(len) = head.parse(&raw).unwrap() else {
        panic!("a partial response: {}", String::from_utf8_lossy(&raw));
    };
    Reply {
        status: head.code.unwrap(),
        headers: head
            .headers
            .iter()
            .map(|h| {
                let value = String::from_utf8(h.value.to_vec()).unwrap();
                (h.name.to_ascii_lowercase(), value)
            })
            .collect(),
        body: raw[len..].to_vec(),
    }
}

/// Sends requests of `paths` over one HTTP/2 connection to `addr`, as
/// `name`, all at once: their answers, in order.
pub async fn request_h2(
    client: &TlsConnector,
    addr: SocketAddr,
    name: &str,
    paths: &[&str],
) -> Vec<Reply> {
    let tls = connect(client, addr, name).await.unwrap();
    assert_eq!(tls.get_ref().1.alpn_protocol(), Some(&b"h2"[..]));
    let (send, conn) = h2::client::handshake(tls).await.unwrap();
    let driving = tokio::task::spawn_local(conn);
    let fetches = paths.iter().map(|path| {
        let send = send.clone();
        async move {
            let req = http::Request::get(format!("https://{name}{path}"))
                .body(())
                .unwrap();
            let mut send = send.ready().await.unwrap();
            let (response, _) = send.send_request(req, true).unwrap();
            let (head, mut body) = response.await.unwrap().into_parts();
            let mut out = Vec::new();
            while let Some(chunk) = body.data().await {
                let chunk = chunk.unwrap();
                body.flow_control().release_capacity(chunk.len()).unwrap();
                out.extend_from_slice(&chunk);
            }
            Reply {
                status: head.status.as_u16(),
                headers: head
                    .headers
                    .iter()
                    .map(|(n, v)| (n.as_str().to_owned(), v.to_str().unwrap().to_owned()))
                    .collect(),
                body: out,
            }
        }
    });
    let replies = tokio::time::timeout(STEP, futures_util::future::join_all(fetches))
        .await
        .expect("the HTTP/2 answers");
    driving.abort();
    replies
}

/// Connects to `svc`'s API over `wss://` as `name`.
pub async fn connect_wss(
    svc: &Svc,
    client: &TlsConnector,
    name: &str,
) -> Result<Ui, tungstenite::Error> {
    let addr = svc.api_addr();
    let stream = connect(client, addr, name).await?;
    let url = format!("wss://{name}:{}/api", addr.port());
    let (ws, resp) =
        tokio_tungstenite::client_async_with_config(url, stream, Some(ws_config())).await?;
    Ok(ui(ws, &resp))
}
