// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Services on localhost and the UI's side of their API: real WebSocket
//! connections carrying frontend RPC, as the UI makes them.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

use std::future::{Future, ready};
use std::time::Duration as StdDuration;

use futures_util::{SinkExt, StreamExt};
use strims_crypto::CertificateChain;
use strims_node::app::AppConfig;
use strims_proto::types::Certificate;
use strims_rpc::{Client, Config as RpcConfig, connect};
use strims_svc::{Config, Db, Svc};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::task::{LocalSet, spawn_local};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::{COOKIE, SET_COOKIE};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::{self, Message};

pub mod tls;
pub mod video;

/// The config of a service on localhost: its API, and a node on TCP and
/// WebSocket listeners, all on ports the OS picks, under the service's
/// defaults otherwise.
pub fn local_config() -> Config {
    let defaults = Config::default();
    Config {
        api_listen: "127.0.0.1:0".into(),
        db: Db::Memory,
        app: AppConfig {
            tcp_listen: Some("127.0.0.1:0".into()),
            ws_listen: Some("127.0.0.1:0".into()),
            allow_insecure_ws: true,
            ..defaults.app.clone()
        },
        ..defaults
    }
}

/// A service of [`local_config`].
pub async fn service() -> Svc {
    Svc::start(local_config()).await.unwrap()
}

/// The UI's connection to a service.
pub struct Ui {
    pub client: Client,
    /// The upgrade response's `Set-Cookie`.
    pub set_cookie: Option<String>,
}

impl Ui {
    /// The `name=value` part of the session cookie the service set.
    pub fn cookie(&self) -> String {
        let set = self.set_cookie.as_deref().expect("a session cookie");
        set.split(';').next().unwrap().to_owned()
    }
}

/// Connects to `svc`'s API at `path`, sending `cookie` if given.
pub async fn connect_at(
    svc: &Svc,
    path: &str,
    cookie: Option<&str>,
) -> Result<Ui, tungstenite::Error> {
    let mut req = format!("ws://{}{path}", svc.api_addr()).into_client_request()?;
    if let Some(c) = cookie {
        req.headers_mut()
            .insert(COOKIE, HeaderValue::from_str(c).unwrap());
    }
    let (ws, resp) =
        tokio_tungstenite::connect_async_with_config(req, Some(ws_config()), true).await?;
    Ok(ui(ws, &resp))
}

/// The WebSocket limits of the UI's connections.
pub fn ws_config() -> WebSocketConfig {
    let max = RpcConfig::frontend().max_message_bytes + 16;
    WebSocketConfig::default()
        .max_message_size(Some(max))
        .max_frame_size(Some(max))
}

/// The UI's connection over `ws`, whose upgrade `resp` answered.
pub fn ui<S>(ws: WebSocketStream<S>, resp: &Response) -> Ui
where
    S: AsyncRead + AsyncWrite + Unpin + 'static,
{
    let set_cookie = resp
        .headers()
        .get(SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_owned());
    let (sink, stream) = ws.split();
    let reader = stream.filter_map(|m| {
        ready(match m {
            Ok(Message::Binary(b)) => Some(Ok(b)),
            Ok(Message::Close(_)) => None,
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
    Ui { client, set_cookie }
}

/// Connects to `svc`'s API.
pub async fn connect_ui(svc: &Svc) -> Ui {
    connect_at(svc, "/api", None).await.unwrap()
}

/// The key of the network whose root `cert` chains to.
pub fn network_key(cert: &Certificate) -> Vec<u8> {
    CertificateChain::decode(cert)
        .unwrap()
        .network_key()
        .to_vec()
}

/// Runs `f` on a current-thread runtime inside a `LocalSet`, on the real
/// clock. Logs as `RUST_LOG` says, if it's set.
pub fn run<F: Future<Output = ()>>(f: F) {
    if let Ok(filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .try_init();
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(LocalSet::new().run_until(f));
}

/// Polls `f` every 250 ms until it returns `Some`, for at most `limit`.
pub async fn eventually<T, Fut: Future<Output = Option<T>>>(
    what: &str,
    limit: StdDuration,
    mut f: impl FnMut() -> Fut,
) -> T {
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        if let Some(v) = f().await {
            return v;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("timed out after {limit:?} waiting for {what}");
        }
        tokio::time::sleep(StdDuration::from_millis(250)).await;
    }
}
