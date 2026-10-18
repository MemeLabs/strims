// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's connections: WebSocket upgrades at the API path, each served
//! as a frontend RPC connection with its own router.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::future::ready;
use std::net::SocketAddr;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use data_encoding::BASE64URL;
use futures_util::{SinkExt, StreamExt};
use strims_dao::seal::StorageKey;
use strims_proto::auth::v1::register_auth_frontend_service;
use strims_rpc::{Config as RpcConfig, Router, connect, wire::MAX_VARINT_LEN};
use strims_session::AuthService;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::{AbortHandle, spawn_local};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::header::{COOKIE, SET_COOKIE};
use tokio_tungstenite::tungstenite::http::{HeaderMap, HeaderValue, StatusCode};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::{self, Message};

use crate::SvcManager;

/// How long an upgrade may take.
const HANDSHAKE_TIMEOUT: StdDuration = StdDuration::from_secs(10);
/// A short pause after a failed accept, so a listener error doesn't spin.
const ACCEPT_BACKOFF: StdDuration = StdDuration::from_millis(50);
const SESSION_COOKIE: &str = "session_key";
/// How long the browser keeps the session cookie: 30 days.
const SESSION_COOKIE_MAX_AGE: u32 = 30 * 24 * 60 * 60;

/// Why a connection's transport failed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum WsError {
    #[error(transparent)]
    Ws(#[from] tungstenite::Error),
    /// The UI sends only binary messages.
    #[error("unexpected text message")]
    Text,
}

/// The API's shared state.
pub(crate) struct Api {
    manager: Rc<SvcManager>,
    path: String,
    max_connections: usize,
    /// The connections being served. Bound: `max_connections`.
    connections: RefCell<BTreeMap<u64, AbortHandle>>,
    next_id: Cell<u64>,
}

impl Api {
    pub(crate) fn new(manager: Rc<SvcManager>, path: String, max_connections: usize) -> Api {
        Api {
            manager,
            path,
            max_connections,
            connections: RefCell::new(BTreeMap::new()),
            next_id: Cell::new(0),
        }
    }

    /// Stops serving every connection.
    pub(crate) fn close(&self) {
        let connections = std::mem::take(&mut *self.connections.borrow_mut());
        for c in connections.values() {
            c.abort();
        }
    }
}

/// Removes a connection from the API's set when its task ends.
struct Served {
    api: Rc<Api>,
    id: u64,
}

impl Drop for Served {
    fn drop(&mut self) {
        self.api.connections.borrow_mut().remove(&self.id);
    }
}

/// Accepts the UI's connections until aborted.
pub(crate) async fn accept(listener: TcpListener, api: Rc<Api>) {
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                if api.connections.borrow().len() >= api.max_connections {
                    tracing::debug!(%addr, "refusing a connection beyond the limit");
                    continue;
                }
                let id = api.next_id.get() + 1;
                api.next_id.set(id);
                let served = Served {
                    api: api.clone(),
                    id,
                };
                let task = spawn_local(serve(served, stream, addr));
                api.connections.borrow_mut().insert(id, task.abort_handle());
            }
            Err(e) => {
                tracing::debug!(error = %e, "accepting a connection failed");
                tokio::time::sleep(ACCEPT_BACKOFF).await;
            }
        }
    }
}

/// The WebSocket limits of the UI's connections: a message holds one
/// frontend RPC message and its length.
fn ws_config() -> WebSocketConfig {
    let max = RpcConfig::frontend().max_message_bytes + MAX_VARINT_LEN;
    WebSocketConfig::default()
        .max_message_size(Some(max))
        .max_frame_size(Some(max))
}

/// The session key in the request's `session_key` cookie, if it holds 32
/// bytes in padded URL-safe base64.
fn cookie_key(headers: &HeaderMap) -> Option<[u8; 32]> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|c| c.trim().split_once('='))
        .filter(|(name, _)| *name == SESSION_COOKIE)
        .find_map(|(_, value)| BASE64URL.decode(value.as_bytes()).ok()?.try_into().ok())
}

/// The `Set-Cookie` value that keeps `key` in the browser.
fn session_cookie(key: &[u8; 32]) -> String {
    format!(
        "{SESSION_COOKIE}={}; Max-Age={SESSION_COOKIE_MAX_AGE}; HttpOnly; Secure; SameSite=None",
        BASE64URL.encode(key)
    )
}

fn refuse(status: StatusCode, why: &str) -> ErrorResponse {
    let mut r = ErrorResponse::new(Some(why.to_owned()));
    *r.status_mut() = status;
    r
}

/// Upgrades a connection and serves the frontend on it until it ends.
async fn serve(served: Served, stream: TcpStream, addr: SocketAddr) {
    let api = served.api.clone();
    if let Err(e) = stream.set_nodelay(true) {
        tracing::debug!(%addr, error = %e, "setting TCP_NODELAY failed");
    }
    let mut session_key = None;
    // tungstenite's `Callback` fixes the error type.
    #[allow(clippy::result_large_err)]
    let check = |req: &Request, mut resp: Response| -> Result<Response, ErrorResponse> {
        if req.uri().path() != api.path {
            return Err(refuse(StatusCode::NOT_FOUND, "not found"));
        }
        let key = match cookie_key(req.headers()) {
            Some(key) => key,
            None => {
                let mut key = [0u8; 32];
                getrandom::fill(&mut key)
                    .map_err(|_| refuse(StatusCode::INTERNAL_SERVER_ERROR, "no entropy"))?;
                key
            }
        };
        let cookie = HeaderValue::from_str(&session_cookie(&key))
            .map_err(|_| refuse(StatusCode::INTERNAL_SERVER_ERROR, "bad cookie"))?;
        resp.headers_mut().append(SET_COOKIE, cookie);
        session_key = Some(key);
        Ok(resp)
    };
    let upgrade = tokio_tungstenite::accept_hdr_async_with_config(stream, check, Some(ws_config()));
    let ws = match tokio::time::timeout(HANDSHAKE_TIMEOUT, upgrade).await {
        Ok(Ok(ws)) => ws,
        Ok(Err(e)) => {
            tracing::debug!(%addr, error = %e, "websocket upgrade failed");
            return;
        }
        Err(_) => {
            tracing::debug!(%addr, "websocket upgrade timed out");
            return;
        }
    };
    let Some(session_key) = session_key else {
        return;
    };

    let (sink, stream) = ws.split();
    let reader = stream
        .take_while(|m| ready(!matches!(m, Ok(Message::Close(_)))))
        .filter_map(|m| {
            ready(match m {
                Ok(Message::Binary(b)) => Some(Ok(b)),
                Ok(Message::Text(_)) => Some(Err(WsError::Text)),
                Ok(_) => None,
                Err(e) => Some(Err(WsError::Ws(e))),
            })
        });
    let writer = sink
        .sink_map_err(WsError::Ws)
        .with(|b: Vec<u8>| ready(Ok::<_, WsError>(Message::Binary(b.into()))));

    let mut router = Router::new();
    let auth = AuthService::new(
        api.manager.clone(),
        Some(StorageKey::from_bytes(session_key)),
        router.clone(),
    );
    register_auth_frontend_service(&mut router, Rc::new(auth));
    tracing::debug!(%addr, "frontend connected");
    let (_, conn) = connect(
        RpcConfig::frontend(),
        Box::pin(reader),
        Box::pin(writer),
        Some(router),
    );
    let res = conn.await;
    tracing::debug!(%addr, ?res, "frontend disconnected");
    drop(served);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn session_cookies_round_trip() {
        let key = [7u8; 32];
        let set = session_cookie(&key);
        assert!(set.contains("Max-Age=2592000; HttpOnly; Secure; SameSite=None"));
        let value = set.split(';').next().unwrap();
        let mut headers = HeaderMap::new();
        headers.append(COOKIE, HeaderValue::from_str("a=b; c").unwrap());
        headers.append(
            COOKIE,
            HeaderValue::from_str(&format!("x=1; {value}")).unwrap(),
        );
        assert_eq!(cookie_key(&headers), Some(key));
    }

    #[test]
    fn malformed_session_cookies_are_ignored() {
        for v in [
            "session_key=",
            "session_key=AAAA",
            "session_key=not base64!",
            &format!("session_key={}", BASE64URL.encode(&[1u8; 31])),
            &format!("other={}", BASE64URL.encode(&[1u8; 32])),
        ] {
            let mut headers = HeaderMap::new();
            headers.append(COOKIE, HeaderValue::from_str(v).unwrap());
            assert_eq!(cookie_key(&headers), None, "{v}");
        }
    }
}
