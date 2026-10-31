// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The headless service: runs a profile's node, serves the UI's frontend
//! RPC over WebSocket, and serves the node's HLS streams over HTTP.
//!
//! [`Svc`] opens the profiles' stores in one redb file, runs their
//! sessions through a [`Manager`] with the node session factory
//! ([`NodeFactory`]), and accepts the UI's WebSocket connections on its
//! API address at its API path. A connection starts with only the auth
//! service bound; signing in or up starts the profile's node and binds its
//! frontends on the connection. A process runs one profile's node at a
//! time.
//!
//! # Connections
//!
//! Each WebSocket message carries whole varint-delimited RPC calls, as the
//! UI sends them. The upgrade reads the `session_key` cookie, 32 bytes in
//! padded URL-safe base64, and sets it on the response; a missing or
//! malformed one is replaced by a new random key. The key seals the
//! session tokens of sign-ins that persist, so a browser that keeps the
//! cookie can sign in with its token again. At most
//! [`Config::max_connections`] connections are served at once.
//!
//! # HLS
//!
//! With [`Config::hls_listen`], the service answers players' plain HTTP
//! requests for the running profile's HLS streams there, from the
//! factory's [`HlsRoutes`](strims_video::hls::HlsRoutes), within the
//! HTTP bounds of [`HlsSettings`](strims_video::HlsSettings). The node's
//! video control then supports HLS egress
//! ([`HlsSettings::serve`](strims_video::HlsSettings::serve)), and its
//! playlist URLs start with
//! [`HlsSettings::base_url`](strims_video::HlsSettings::base_url) when
//! the device's HLS config names no public server address. An empty base
//! URL becomes `http://` and the listener's address; for an unspecified
//! address, its port at [`AppConfig::public_ip`], else at
//! [`AppConfig::ws_public_hostname`], else none, and the HLS config must
//! name the address. Without the listener, HLS egress is unsupported.
//!
//! # Threads
//!
//! [`Svc`] runs inside a tokio `LocalSet`, on one thread. The running
//! profile's PSI sessions compute on a thread of their own by default
//! ([`PsiMode::Thread`] in [`Config::app`]), so the node's I/O goes on
//! while a session computes.

mod api;
mod http;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;

use rand_core::SeedableRng;
use strims_base::rng::DetRng;
use strims_kv_redb::RedbKv;
use strims_node::app::{AppConfig, NodeFactory, NodeTimer, PsiMode};
use strims_node::{NodeInfo, SystemClock, TokioNet};
use strims_session::Manager;
use tokio::net::TcpListener;
use tokio::task::{AbortHandle, spawn_local};

type Factory = NodeFactory<TokioNet, SystemClock>;
type SvcTimer = NodeTimer<SystemClock>;
type SvcManager = Manager<RedbKv, Factory, SvcTimer>;

/// Where the profiles' stores live.
#[derive(Clone, Debug)]
pub enum Db {
    /// A redb file, created if it doesn't exist.
    File(PathBuf),
    /// Memory, gone when the service stops.
    Memory,
}

/// How the service runs.
#[derive(Clone)]
pub struct Config {
    /// `host:port` the API accepts the UI's connections on; port 0 picks
    /// one.
    pub api_listen: String,
    /// The path the API upgrades WebSocket connections at.
    pub api_path: String,
    pub db: Db,
    /// How a signed-in profile's node runs; by default its PSI sessions
    /// compute on their own thread ([`PsiMode::Thread`]). The service sets
    /// [`HlsSettings::serve`](strims_video::HlsSettings::serve) in its
    /// video config, and fills an empty
    /// [`HlsSettings::base_url`](strims_video::HlsSettings::base_url).
    pub app: AppConfig,
    /// The most UI connections served at once.
    pub max_connections: usize,
    /// `host:port` HLS is served to players on, over plain HTTP; port 0
    /// picks one. `None` serves no HLS.
    pub hls_listen: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_listen: "127.0.0.1:8083".to_owned(),
            api_path: "/api".to_owned(),
            db: Db::Memory,
            app: AppConfig {
                psi: PsiMode::Thread,
                ..AppConfig::default()
            },
            max_connections: 64,
            hls_listen: None,
        }
    }
}

/// Why the service didn't start.
#[derive(Debug, thiserror::Error)]
pub enum SvcError {
    #[error("opening the store: {0}")]
    Store(#[from] strims_kv::Error),
    #[error("listening for the API: {0}")]
    Listen(#[from] std::io::Error),
    #[error("listening for HLS: {0}")]
    HlsListen(std::io::Error),
    #[error("drawing a seed from OS entropy: {0}")]
    Entropy(#[from] getrandom::Error),
    #[error("checking the codec stage's ffmpeg: {0}")]
    Ffmpeg(strims_node::codec::ProbeError),
}

/// A running service. Dropping it stops serving the UI and HLS.
pub struct Svc {
    manager: Rc<SvcManager>,
    api: Rc<api::Api>,
    api_addr: SocketAddr,
    accept: AbortHandle,
    hls: Option<Hls>,
}

/// The HLS server.
struct Hls {
    http: Rc<http::Http>,
    addr: SocketAddr,
    base_url: String,
    accept: AbortHandle,
}

impl Svc {
    /// Opens the store and starts serving the API, and HLS if configured.
    /// A config naming an ffmpeg for the codec stage starts only if
    /// [`probe_ffmpeg`](strims_node::codec::probe_ffmpeg) takes it. Runs
    /// inside a tokio `LocalSet`.
    pub async fn start(config: Config) -> Result<Svc, SvcError> {
        // The HLS address goes into the video config the factory takes.
        let hls_listener = match &config.hls_listen {
            Some(addr) => {
                let listener = TcpListener::bind(addr).await.map_err(SvcError::HlsListen)?;
                let addr = listener.local_addr().map_err(SvcError::HlsListen)?;
                Some((listener, addr))
            }
            None => None,
        };
        let mut app = config.app;
        app.video.hls.serve = hls_listener.is_some();
        if let Some((_, addr)) = &hls_listener
            && app.video.hls.base_url.is_empty()
        {
            app.video.hls.base_url = hls_base_url(*addr, &app);
        }
        if let Some(ffmpeg) = &app.ffmpeg {
            let version = strims_node::codec::probe_ffmpeg(ffmpeg)
                .await
                .map_err(SvcError::Ffmpeg)?;
            tracing::info!(program = %ffmpeg.program.display(), version, "transcoding with ffmpeg");
        }
        let hls_limits = http::Limits::new(&app.video.hls);
        let hls_base_url = app.video.hls.base_url.clone();

        let kv = match &config.db {
            Db::File(path) => RedbKv::open(path)?,
            Db::Memory => RedbKv::in_memory()?,
        };
        // Profile keys and sealing nonces come from this rng.
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed)?;
        let manager = Rc::new(Manager::new(
            kv,
            NodeFactory::new(TokioNet, SystemClock, app),
            NodeTimer(SystemClock),
            std::env::consts::OS,
            DetRng::from_seed(seed),
        ));
        let listener = TcpListener::bind(&config.api_listen).await?;
        let api_addr = listener.local_addr()?;
        let api = Rc::new(api::Api::new(
            manager.clone(),
            config.api_path,
            config.max_connections,
        ));
        let accept = spawn_local(api::accept(listener, api.clone())).abort_handle();
        let hls = hls_listener.map(|(listener, addr)| {
            let http = Rc::new(http::Http::new(
                manager.factory().hls_routes().clone(),
                manager.timer().clone(),
                hls_limits,
            ));
            let accept = spawn_local(http::accept(listener, http.clone())).abort_handle();
            Hls {
                http,
                addr,
                base_url: hls_base_url,
                accept,
            }
        });
        Ok(Svc {
            manager,
            api,
            api_addr,
            accept,
            hls,
        })
    }

    /// The address the API accepts connections on.
    pub fn api_addr(&self) -> SocketAddr {
        self.api_addr
    }

    /// The address HLS is served on, if it is.
    pub fn hls_addr(&self) -> Option<SocketAddr> {
        self.hls.as_ref().map(|h| h.addr)
    }

    /// The start of HLS playlist URLs when the HLS config names no public
    /// server address; empty for none, and when HLS isn't served.
    pub fn hls_base_url(&self) -> &str {
        self.hls.as_ref().map_or("", |h| h.base_url.as_str())
    }

    /// The running profile's node, once a sign-in started one.
    pub fn node_info(&self) -> Option<NodeInfo> {
        self.manager.factory().running()
    }

    /// Stops serving the UI and HLS, and stops the running node.
    pub async fn stop(self) {
        drop(self);
        tokio::task::yield_now().await;
    }
}

impl Drop for Svc {
    fn drop(&mut self) {
        self.accept.abort();
        self.api.close();
        if let Some(hls) = &self.hls {
            hls.accept.abort();
            hls.http.close();
        }
    }
}

/// The base of playlist URLs for HLS served at `addr`: `http://` and the
/// address; for an unspecified address, its port at the public IP, else at
/// the WebSocket public hostname, else empty.
fn hls_base_url(addr: SocketAddr, app: &AppConfig) -> String {
    if !addr.ip().is_unspecified() {
        return format!("http://{addr}");
    }
    match (app.public_ip, app.ws_public_hostname.as_str()) {
        (Some(ip), _) => format!("http://{}", SocketAddr::new(ip, addr.port())),
        (None, "") => String::new(),
        (None, host) => format!("http://{host}:{}", addr.port()),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn hls_base_urls_name_a_reachable_address() {
        let mut app = AppConfig::default();
        let at = |a: &str| a.parse::<SocketAddr>().unwrap();
        assert_eq!(
            hls_base_url(at("127.0.0.1:8082"), &app),
            "http://127.0.0.1:8082"
        );
        assert_eq!(hls_base_url(at("[::1]:8082"), &app), "http://[::1]:8082");
        assert_eq!(hls_base_url(at("0.0.0.0:8082"), &app), "");
        app.ws_public_hostname = "node.example".into();
        assert_eq!(
            hls_base_url(at("0.0.0.0:8082"), &app),
            "http://node.example:8082"
        );
        app.public_ip = Some("2001:db8::1".parse().unwrap());
        assert_eq!(
            hls_base_url(at("[::]:8082"), &app),
            "http://[2001:db8::1]:8082"
        );
    }
}
