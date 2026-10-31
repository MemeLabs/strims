// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The app layer in the worker: [`strims_node_core::app`] composed on the
//! browser driver.
//!
//! - [`WasmPlatform`] is the core's [`AppPlatform`]: a browser node
//!   ([`crate::node::browser_config`]) with the window's options, its
//!   WebRTC signalled over the window's signal port, seeded from
//!   `crypto.getRandomValues`, timed by [`WasmClock`].
//! - Its PSI sessions compute in the PSI worker ([`WorkerPsi`]) when the
//!   window started one, and inline otherwise.
//! - The Debug service's snapshots count the worker's heap and its linear
//!   memory ([`crate::memory`]).
//! - The node hosts no RTMP ingress ([`NoVideoHost`]), and its HLS routes
//!   aren't served. The directory makes its HTTPS requests with the
//!   browser's `fetch` ([`BrowserFetch`]). The vnic config takes no
//!   ceilings, and the services run their default configs.
//! - [`WasmSpawn`] runs the app's tasks on the worker's executor
//!   ([`crate::runtime`]).
//! - [`WasmFactory`] is the [`SessionFactory`] that runs a signed-in
//!   profile on a `WasmPlatform`, one profile per worker. While the
//!   profile runs, the window's feeds port, if it passed one, serves
//!   players its streams ([`crate::feed`]).

use std::future::{Future, ready};
use std::rc::Rc;

use futures_util::future::LocalBoxFuture;
use strims_directory::Fetch;
use strims_kv::Kv;
use strims_network::psi::{InlinePsi, PsiEngine};
use strims_node_core::app::{
    AppPlatform, MemoryMetrics, ProfileRuntime, ProfileSlot, ServicesConfig, Spawn, StartError,
    TaskHandle, bind_profile,
};
use strims_node_core::{Node, NodeConfig, NodeError, NodeEvents, NodeInfo};
use strims_proto::types::Key;
use strims_rpc::Router;
use strims_session::{Session, SessionFactory, VnicCeilings};
use strims_video::NoVideoHost;
use strims_video::hls::HlsRoutes;
use web_sys::MessagePort;

use crate::clock::WasmClock;
use crate::feed::{self, FeedServer};
use crate::fetch::BrowserFetch;
use crate::options::Options;
use crate::psi::WorkerPsi;
use crate::runtime;

/// A running profile on a [`WasmPlatform`]: its node and the controls its
/// frontends serve. Dropping it stops the controls' tasks and the node.
pub type WasmRuntime<K> = ProfileRuntime<K, WasmClock, NoVideoHost, WasmSpawn>;

/// A running profile and the feeds server on the window's feeds port.
/// Dropping it closes the feeds, then stops the profile.
pub struct WasmProfile<K: Kv> {
    _feeds: Option<FeedServer>,
    runtime: WasmRuntime<K>,
}

impl<K: Kv> WasmProfile<K> {
    /// The profile's node and controls.
    pub fn runtime(&self) -> &WasmRuntime<K> {
        &self.runtime
    }
}

/// Runs the app's tasks as local tasks on the worker ([`runtime::spawn`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct WasmSpawn;

impl Spawn for WasmSpawn {
    fn spawn_local(&self, task: LocalBoxFuture<'static, ()>) -> TaskHandle {
        let task = runtime::spawn(task);
        TaskHandle::new(move || task.abort())
    }

    fn shutdown(&self, node: Node) {
        drop(runtime::spawn(async move {
            if let Err(e) = node.shutdown().await {
                tracing::debug!(error = %e, "stopping the node");
            }
        }));
    }
}

/// The browser's [`AppPlatform`]; see the [module docs](self).
pub struct WasmPlatform {
    allow_insecure_ws: bool,
    ice_servers: Option<Vec<String>>,
    /// The window's signal port, which every node of the platform takes.
    signal: MessagePort,
    hls: HlsRoutes,
    /// The engine on the port to the PSI worker, if the window started
    /// one. Every profile's sessions share it.
    psi: Option<WorkerPsi>,
    /// The window's feeds port, which the running profile serves.
    feeds: Option<MessagePort>,
}

impl WasmPlatform {
    /// A platform of nodes configured by `options`, signalling WebRTC
    /// with the window over `signal`, computing PSI in the PSI worker at
    /// the other end of `psi`, if there is one, and feeding players on
    /// `feeds`, if the window passed it.
    pub fn new(
        options: &Options,
        signal: MessagePort,
        psi: Option<MessagePort>,
        feeds: Option<MessagePort>,
    ) -> WasmPlatform {
        WasmPlatform {
            allow_insecure_ws: options.allow_insecure_ws,
            ice_servers: options.ice_servers.clone(),
            signal,
            hls: HlsRoutes::new(),
            psi: psi.map(WorkerPsi::new),
            feeds,
        }
    }
}

impl AppPlatform for WasmPlatform {
    type Timer = WasmClock;
    type VideoHost = NoVideoHost;
    type Spawn = WasmSpawn;

    fn seed(&self) -> Result<[u8; 32], StartError> {
        crate::entropy::seed().map_err(StartError::Entropy)
    }

    /// [`crate::node::browser_config`] with the options' ICE servers and
    /// `ws://` setting, and this crate's version.
    fn node_config(&self, key: Key, seed: u64) -> NodeConfig {
        let mut config = crate::node::browser_config(key, seed);
        if let Some(servers) = &self.ice_servers {
            config.webrtc.endpoint.ice_servers = Some(servers.clone());
        }
        config.allow_insecure_ws = self.allow_insecure_ws;
        config.node_version = env!("CARGO_PKG_VERSION").to_owned();
        config
    }

    fn ceilings(&self) -> VnicCeilings {
        VnicCeilings::default()
    }

    fn services(&self) -> ServicesConfig {
        ServicesConfig::default()
    }

    fn spawn_node(
        &self,
        config: NodeConfig,
    ) -> impl Future<Output = Result<(Node, NodeEvents), NodeError>> {
        ready(crate::node::spawn(config, self.signal.clone()))
    }

    fn timer(&self) -> WasmClock {
        WasmClock
    }

    fn fetch(&self) -> Fetch {
        Fetch::new(BrowserFetch::new())
    }

    fn video_host(&self) -> NoVideoHost {
        NoVideoHost
    }

    fn hls_routes(&self) -> HlsRoutes {
        self.hls.clone()
    }

    fn spawner(&self) -> WasmSpawn {
        WasmSpawn
    }

    /// The PSI worker's [`WorkerPsi`] when the window started one, else
    /// [`InlinePsi`].
    fn psi(&self) -> Rc<dyn PsiEngine> {
        match &self.psi {
            Some(engine) => Rc::new(engine.clone()),
            None => Rc::new(InlinePsi),
        }
    }

    /// The worker's heap and linear memory ([`crate::memory`]).
    fn memory(&self) -> Option<fn() -> MemoryMetrics> {
        Some(crate::memory::metrics)
    }
}

/// The [`SessionFactory`] that runs a signed-in profile's node and
/// services on a [`WasmPlatform`] ([`ProfileSlot::start`]), and serves the
/// window's feeds from its streams ([`feed::serve`]). A worker runs one
/// profile's node at a time, since the nodes share the signal port:
/// starting another profile's session fails with
/// [`StartError::ProfileRunning`].
pub struct WasmFactory {
    platform: WasmPlatform,
    slot: ProfileSlot,
}

impl WasmFactory {
    /// A factory of nodes configured by `options`, signalling WebRTC with
    /// the window over `signal`, computing PSI in the PSI worker at the
    /// other end of `psi`, if there is one, and feeding players on
    /// `feeds`, if the window passed it ([`WasmPlatform::new`]).
    pub fn new(
        options: &Options,
        signal: MessagePort,
        psi: Option<MessagePort>,
        feeds: Option<MessagePort>,
    ) -> WasmFactory {
        WasmFactory {
            platform: WasmPlatform::new(options, signal, psi, feeds),
            slot: ProfileSlot::new(),
        }
    }

    /// The platform the profiles run on.
    pub fn platform(&self) -> &WasmPlatform {
        &self.platform
    }

    /// The running node, if a session started one.
    pub fn running(&self) -> Option<NodeInfo> {
        self.slot.running()
    }
}

impl<K: Kv> SessionFactory<K> for WasmFactory {
    type Runtime = WasmProfile<K>;
    type Error = StartError;

    async fn start(&self, session: &Rc<Session<K>>) -> Result<WasmProfile<K>, StartError> {
        let runtime = self.slot.start(&self.platform, session).await?;
        let feeds = self
            .platform
            .feeds
            .as_ref()
            .map(|port| feed::serve(runtime.video().clone(), port.clone()));
        Ok(WasmProfile {
            _feeds: feeds,
            runtime,
        })
    }

    fn bind(&self, profile: &WasmProfile<K>, session: &Rc<Session<K>>, router: &Router) {
        bind_profile(&profile.runtime, session, router);
    }
}
