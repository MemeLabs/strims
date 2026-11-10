// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The command line's frontend connection: to a running service's API, or
//! to one it starts on a store and stops when done.

use std::future::ready;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, RootCertStore};
use strims_proto::auth::v1::sign_in_request::{Credentials, Password};
use strims_proto::auth::v1::{AuthFrontendClient, SignInRequest};
use strims_rpc::wire::MAX_VARINT_LEN;
use strims_rpc::{Client, Config as RpcConfig, connect};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::task::spawn_local;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::{self, Message};

use super::ProfileLogin;
use crate::{Config, Db, Svc};

/// The API a running service serves by default.
pub const DEFAULT_API: &str = "ws://127.0.0.1:8083/api";

/// Where the command line's calls go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A running service's API: a `ws://` or `wss://` URL, and a PEM file
    /// of roots a `wss://` certificate may chain to besides the public
    /// ones.
    Api { url: String, ca: Option<PathBuf> },
    /// A service the command line runs on this store, which no running
    /// service holds.
    Db(PathBuf),
}

/// A frontend connection, and the service the command line started for it.
pub(crate) struct Connection {
    pub(crate) client: Client,
    svc: Option<Svc>,
}

impl Connection {
    pub(crate) async fn open(target: &Target) -> Result<Connection, String> {
        match target {
            Target::Api { url, ca } => Ok(Connection {
                client: dial(url, ca.as_deref()).await?,
                svc: None,
            }),
            Target::Db(path) => {
                if !path.exists() {
                    return Err(missing_store(path));
                }
                let config = Config {
                    api_listen: "127.0.0.1:0".into(),
                    db: Db::File(path.clone()),
                    ..Config::default()
                };
                let api_path = config.api_path.clone();
                let svc = Svc::start(config).await.map_err(|e| {
                    format!(
                        "{}: {e} (while a service runs on the store, call it with --api)",
                        path.display()
                    )
                })?;
                let client = dial(&format!("ws://{}{api_path}", svc.api_addr()), None).await?;
                Ok(Connection {
                    client,
                    svc: Some(svc),
                })
            }
        }
    }

    /// Signs in as `login`'s profile, which starts its node if it isn't
    /// running.
    pub(crate) async fn sign_in(&self, login: &ProfileLogin) -> Result<(), String> {
        AuthFrontendClient::new(self.client.clone())
            .sign_in(&SignInRequest {
                credentials: Some(Credentials::Password(Password {
                    name: login.name.clone(),
                    password: login.password.clone(),
                    ..Default::default()
                })),
            })
            .await
            .map(drop)
            .map_err(|e| format!("signing in as {}: {e}", login.name))
    }

    /// Closes the connection, and stops the service it started.
    pub(crate) async fn close(self) {
        drop(self.client);
        if let Some(svc) = self.svc {
            svc.stop().await;
        }
    }
}

/// What a path with no store at it says.
pub(crate) fn missing_store(path: &Path) -> String {
    format!(
        "no store at {} (strims-svc profiles add makes one)",
        path.display()
    )
}

/// A frontend RPC client over a WebSocket connection to `url`.
async fn dial(url: &str, ca: Option<&Path>) -> Result<Client, String> {
    let fail = |e: &dyn std::fmt::Display| format!("connecting to {url}: {e}");
    let req = url.into_client_request().map_err(|e| fail(&e))?;
    let uri = req.uri().clone();
    let secure = match uri.scheme_str() {
        Some("ws") => false,
        Some("wss") => true,
        _ => return Err(format!("--api takes a ws:// or wss:// URL: {url}")),
    };
    let host = uri.host().ok_or_else(|| fail(&"no host"))?;
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_owned();
    let port = uri.port_u16().unwrap_or(if secure { 443 } else { 80 });
    let tcp = TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|e| fail(&e))?;
    let config = Some(ws_config());
    if secure {
        let name = ServerName::try_from(host).map_err(|e| fail(&e))?;
        let tls = TlsConnector::from(tls_config(ca)?)
            .connect(name, tcp)
            .await
            .map_err(|e| fail(&e))?;
        let (ws, _) = tokio_tungstenite::client_async_with_config(req, tls, config)
            .await
            .map_err(|e| fail(&e))?;
        Ok(rpc_client(ws))
    } else {
        let (ws, _) = tokio_tungstenite::client_async_with_config(req, tcp, config)
            .await
            .map_err(|e| fail(&e))?;
        Ok(rpc_client(ws))
    }
}

/// The API's frames: one RPC message each, at most a frontend message
/// and its length, as the API takes them.
fn ws_config() -> WebSocketConfig {
    let max = RpcConfig::frontend().max_message_bytes + MAX_VARINT_LEN;
    WebSocketConfig::default()
        .max_message_size(Some(max))
        .max_frame_size(Some(max))
}

/// A TLS client on the public roots and those in the PEM file `ca`.
fn tls_config(ca: Option<&Path>) -> Result<Arc<ClientConfig>, String> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    if let Some(path) = ca {
        let read = |e: &dyn std::fmt::Display| format!("--ca {}: {e}", path.display());
        for cert in CertificateDer::pem_file_iter(path).map_err(|e| read(&e))? {
            roots
                .add(cert.map_err(|e| read(&e))?)
                .map_err(|e| read(&e))?;
        }
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// An RPC client over `ws`, its connection driven on a local task.
fn rpc_client<S>(ws: WebSocketStream<S>) -> Client
where
    S: AsyncRead + AsyncWrite + Unpin + 'static,
{
    let (sink, stream) = ws.split();
    let reader = stream.filter_map(|m| {
        ready(match m {
            Ok(Message::Binary(b)) => Some(Ok(b)),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    });
    let writer =
        sink.with(|b: Vec<u8>| ready(Ok::<_, tungstenite::Error>(Message::Binary(b.into()))));
    let (client, conn) = connect(
        RpcConfig::frontend(),
        Box::pin(reader),
        Box::pin(writer),
        None,
    );
    spawn_local(async move {
        let _ = conn.await;
    });
    client
}
