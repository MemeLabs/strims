// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The headless service: runs a profile's node and serves the UI's
//! frontend RPC over WebSocket.
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
//! Everything runs on one thread: [`Svc`] runs inside a tokio `LocalSet`.

mod api;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;

use rand_core::SeedableRng;
use strims_base::rng::DetRng;
use strims_kv_redb::RedbKv;
use strims_node::app::{AppConfig, NodeFactory, NodeTimer};
use strims_node::{NodeInfo, SystemClock, TokioNet};
use strims_session::Manager;
use tokio::net::TcpListener;
use tokio::task::{AbortHandle, spawn_local};

type Factory = NodeFactory<TokioNet, SystemClock>;
type SvcManager = Manager<RedbKv, Factory, NodeTimer<SystemClock>>;

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
    /// How a signed-in profile's node runs.
    pub app: AppConfig,
    /// The most UI connections served at once.
    pub max_connections: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_listen: "127.0.0.1:8083".to_owned(),
            api_path: "/api".to_owned(),
            db: Db::Memory,
            app: AppConfig::default(),
            max_connections: 64,
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
    #[error("drawing a seed from OS entropy: {0}")]
    Entropy(#[from] getrandom::Error),
}

/// A running service. Dropping it stops serving the UI.
pub struct Svc {
    manager: Rc<SvcManager>,
    api: Rc<api::Api>,
    api_addr: SocketAddr,
    accept: AbortHandle,
}

impl Svc {
    /// Opens the store and starts serving the API. Runs inside a tokio
    /// `LocalSet`.
    pub async fn start(config: Config) -> Result<Svc, SvcError> {
        let kv = match &config.db {
            Db::File(path) => RedbKv::open(path)?,
            Db::Memory => RedbKv::in_memory()?,
        };
        // Profile keys and sealing nonces come from this rng.
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed)?;
        let manager = Rc::new(Manager::new(
            kv,
            NodeFactory::new(TokioNet, SystemClock, config.app),
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
        Ok(Svc {
            manager,
            api,
            api_addr,
            accept,
        })
    }

    /// The address the API accepts connections on.
    pub fn api_addr(&self) -> SocketAddr {
        self.api_addr
    }

    /// The running profile's node, once a sign-in started one.
    pub fn node_info(&self) -> Option<NodeInfo> {
        self.manager.factory().running()
    }

    /// Stops serving the UI and stops the running node.
    pub async fn stop(self) {
        drop(self);
        tokio::task::yield_now().await;
    }
}

impl Drop for Svc {
    fn drop(&mut self) {
        self.accept.abort();
        self.api.close();
    }
}
