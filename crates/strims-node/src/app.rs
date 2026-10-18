// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The app layer on a native node: [`strims_node_core::app`] composed on
//! tokio, natively or under turmoil.
//!
//! - [`NodeTimer`] is the node's [`Clock`] as the app's [`Timer`].
//! - [`NativePlatform`] is the core's [`AppPlatform`] for an
//!   [`AppConfig`]: the node spawns on the current runtime over a [`Net`],
//!   with the config's listeners and advertised addresses, and its seed
//!   comes from OS entropy. The directory's servers make their HTTPS
//!   requests over the `Net` ([`NetFetch`]), and the video control's RTMP
//!   ingress listens on it ([`NetVideoHost`]).
//! - [`TokioSpawn`] runs the app's tasks with `tokio::task::spawn_local`,
//!   so they run inside a tokio `LocalSet`.
//! - [`NodeFactory`] is the [`SessionFactory`] that runs a signed-in
//!   profile on a `NativePlatform`, one profile per process. It holds the
//!   [`HlsRoutes`] the running profile's HLS streams are served on, for
//!   the driver's HTTP server.
//!
//! The core's hosts and peer services are re-exported for tests and
//! simulations that compose services themselves: [`NodeHost`],
//! [`PeerHandler`], and [`PeerServices`] on `TokioSpawn`.

use std::future::Future;
use std::net::IpAddr;
use std::rc::Rc;
use std::sync::Arc;

use futures_util::future::LocalBoxFuture;
use strims_base::time::Time;
use strims_chat::WhisperConfig;
use strims_directory::Fetch;
use strims_kv::Kv;
use strims_proto::types::Key;
use strims_rpc::{Router, Timer};
use strims_session::{Session, SessionFactory, VnicCeilings};
use strims_video::hls::HlsRoutes;
use tokio::time::Sleep;

pub use strims_node_core::app::{
    AppPlatform, IndexHosts, NodeHost, PeerHandler, ProfileRuntime, ProfileSlot, ServicesConfig,
    Spawn, StartError, TaskHandle, bind_profile,
};

use crate::fetch::{NetFetch, tls_client_config};
use crate::video_host::NetVideoHost;
use crate::{Clock, Net, Node, NodeConfig, NodeError, NodeEvents, NodeInfo};

/// The core's [`strims_node_core::app::PeerServices`], spawning on
/// [`TokioSpawn`].
pub type PeerServices = strims_node_core::app::PeerServices<TokioSpawn>;

/// A running profile on a [`NativePlatform`]: its node and the controls
/// its frontends serve. Dropping it stops the controls' tasks and the
/// node.
pub type AppRuntime<K, N, C> = ProfileRuntime<K, NodeTimer<C>, NetVideoHost<N>, TokioSpawn>;

/// A node's clock as the app layer's [`Timer`].
#[derive(Clone)]
pub struct NodeTimer<C: Clock>(pub C);

impl<C: Clock> Timer for NodeTimer<C> {
    type Sleep = Sleep;

    fn now(&self) -> Time {
        self.0.now()
    }

    fn sleep_until(&self, deadline: Time) -> Sleep {
        tokio::time::sleep_until(self.0.deadline(deadline))
    }
}

/// Runs the app's tasks on the current tokio `LocalSet`.
#[derive(Clone, Copy, Debug, Default)]
pub struct TokioSpawn;

impl Spawn for TokioSpawn {
    fn spawn_local(&self, task: LocalBoxFuture<'static, ()>) -> TaskHandle {
        let task = tokio::task::spawn_local(task).abort_handle();
        TaskHandle::new(move || task.abort())
    }

    /// Spawns the shutdown on the current runtime. Outside one there is
    /// nothing to run it on, and the node task stops with its runtime.
    fn shutdown(&self, node: Node) {
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::spawn(async move {
                let _ = node.shutdown().await;
            });
        }
    }
}

// --- sessions -----------------------------------------------------------------

/// How a profile's node and services run.
#[derive(Clone, Default)]
pub struct AppConfig {
    /// `host:port` the node accepts TCP links on; port 0 picks one.
    pub tcp_listen: Option<String>,
    /// `host:port` the node accepts WebSocket links on, at
    /// `/<hex profile key>`.
    pub ws_listen: Option<String>,
    /// The address link URIs advertise for a listener on an unspecified
    /// address.
    pub public_ip: Option<IpAddr>,
    /// The host WebSocket link URIs advertise; empty for `public_ip`.
    pub ws_public_hostname: String,
    /// The port WebSocket link URIs advertise; 0 for the listener's.
    pub ws_public_port: u16,
    /// Whether the node dials `ws://` links, which aren't encrypted by TLS.
    pub allow_insecure_ws: bool,
    /// The largest peer budget the node takes from the profile's vnic
    /// config; `None` takes it as stored.
    pub max_peers: Option<u32>,
    /// The largest upload cap, in bytes per second, the node takes from the
    /// profile's vnic config; `None` takes it as stored.
    pub max_upload: Option<u64>,
    /// The host certificate's subject.
    pub label: String,
    pub network: strims_network::Config,
    pub bootstrap: strims_network::bootstrap::Config,
    pub transfer: strims_transfer::Config,
    pub replication: strims_replication::Config,
    pub directory: strims_directory::Config,
    pub chat: strims_chat::Config,
    pub whispers: WhisperConfig,
    pub video: strims_video::Config,
}

impl AppConfig {
    /// The configs of the services a profile runs.
    pub fn services(&self) -> ServicesConfig {
        ServicesConfig {
            network: self.network,
            bootstrap: self.bootstrap,
            transfer: self.transfer,
            replication: self.replication.clone(),
            directory: self.directory.clone(),
            chat: self.chat.clone(),
            whispers: self.whispers.clone(),
            video: self.video.clone(),
        }
    }
}

/// The native [`AppPlatform`]: a profile's node on `N`'s sockets, timed
/// by `C` and configured by an [`AppConfig`], under the profile's vnic
/// config within [`AppConfig::max_upload`] and [`AppConfig::max_peers`].
/// See the [module docs](self).
pub struct NativePlatform<N: Net, C: Clock> {
    net: N,
    clock: C,
    config: AppConfig,
    /// The TLS client config of every node's HTTPS requests.
    tls: Arc<rustls::ClientConfig>,
    hls: HlsRoutes,
}

impl<N: Net, C: Clock> NativePlatform<N, C> {
    /// A platform of nodes on `net`, timed by `clock`.
    pub fn new(net: N, clock: C, config: AppConfig) -> Self {
        NativePlatform {
            net,
            clock,
            config,
            tls: tls_client_config(),
            hls: HlsRoutes::new(),
        }
    }
}

impl<N: Net, C: Clock> AppPlatform for NativePlatform<N, C> {
    type Timer = NodeTimer<C>;
    type VideoHost = NetVideoHost<N>;
    type Spawn = TokioSpawn;

    fn seed(&self) -> Result<[u8; 32], StartError> {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).map_err(|e| StartError::Entropy(e.to_string()))?;
        Ok(seed)
    }

    fn node_config(&self, key: Key, seed: u64) -> NodeConfig {
        let c = &self.config;
        let mut config = NodeConfig::new(key, seed);
        config.label.clone_from(&c.label);
        config.tcp_listen.clone_from(&c.tcp_listen);
        config.ws_listen.clone_from(&c.ws_listen);
        config.public_ip = c.public_ip;
        config.ws_public_hostname.clone_from(&c.ws_public_hostname);
        config.ws_public_port = c.ws_public_port;
        config.allow_insecure_ws = c.allow_insecure_ws;
        config.node_platform = format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH);
        config.node_version = env!("CARGO_PKG_VERSION").to_owned();
        config
    }

    fn ceilings(&self) -> VnicCeilings {
        VnicCeilings {
            max_upload_bytes_per_second: self.config.max_upload,
            max_peers: self.config.max_peers,
        }
    }

    fn services(&self) -> ServicesConfig {
        self.config.services()
    }

    fn spawn_node(
        &self,
        config: NodeConfig,
    ) -> impl Future<Output = Result<(Node, NodeEvents), NodeError>> {
        crate::native::spawn(config, self.net.clone(), self.clock.clone())
    }

    fn timer(&self) -> NodeTimer<C> {
        NodeTimer(self.clock.clone())
    }

    fn fetch(&self) -> Fetch {
        Fetch::new(NetFetch::new(self.net.clone(), self.tls.clone()))
    }

    fn video_host(&self) -> NetVideoHost<N> {
        NetVideoHost(self.net.clone())
    }

    fn hls_routes(&self) -> HlsRoutes {
        self.hls.clone()
    }

    fn spawner(&self) -> TokioSpawn {
        TokioSpawn
    }
}

/// The [`SessionFactory`] that runs a signed-in profile's node and
/// services on a [`NativePlatform`] ([`ProfileSlot::start`]). A process
/// runs one profile's node at a time, since the node listens on the
/// configured addresses: starting another profile's session fails with
/// [`StartError::ProfileRunning`].
pub struct NodeFactory<N: Net, C: Clock> {
    platform: NativePlatform<N, C>,
    slot: ProfileSlot,
}

impl<N: Net, C: Clock> NodeFactory<N, C> {
    /// A factory of nodes on `net`, timed by `clock`.
    pub fn new(net: N, clock: C, config: AppConfig) -> Self {
        NodeFactory {
            platform: NativePlatform::new(net, clock, config),
            slot: ProfileSlot::new(),
        }
    }

    /// The running node, if a session started one.
    pub fn running(&self) -> Option<NodeInfo> {
        self.slot.running()
    }

    /// The routes of the running profile's HLS streams, for the driver's
    /// HTTP server to answer requests from. The video control serves its
    /// streams there only where [`strims_video::HlsSettings::serve`] is
    /// set in [`AppConfig::video`].
    pub fn hls_routes(&self) -> &HlsRoutes {
        &self.platform.hls
    }
}

impl<K: Kv, N: Net, C: Clock> SessionFactory<K> for NodeFactory<N, C> {
    type Runtime = AppRuntime<K, N, C>;
    type Error = StartError;

    async fn start(&self, session: &Rc<Session<K>>) -> Result<AppRuntime<K, N, C>, StartError> {
        self.slot.start(&self.platform, session).await
    }

    fn bind(&self, runtime: &AppRuntime<K, N, C>, session: &Rc<Session<K>>, router: &Router) {
        bind_profile(runtime, session, router);
    }
}
