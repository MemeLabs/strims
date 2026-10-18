// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What a driver supplies to run a profile: its platform and its spawner.

use std::fmt;
use std::future::Future;

use futures_util::future::LocalBoxFuture;
use strims_chat::WhisperConfig;
use strims_directory::Fetch;
use strims_proto::types::Key;
use strims_rpc::Timer;
use strims_session::VnicCeilings;
use strims_video::VideoHost;
use strims_video::hls::HlsRoutes;

use crate::{Node, NodeConfig, NodeError, NodeEvents};

/// A driver's executor for the app's local tasks.
///
/// A spawner carries no state: [`Default`] builds one as good as any
/// other.
pub trait Spawn: Clone + Default + 'static {
    /// Runs `task` as a local task on the current thread's executor.
    fn spawn_local(&self, task: LocalBoxFuture<'static, ()>) -> TaskHandle;

    /// Stops `node`, whose profile's tasks are gone: runs
    /// [`Node::shutdown`] where the executor still runs.
    fn shutdown(&self, node: Node);
}

/// A spawned local task. [`TaskHandle::abort`] cancels it; dropping the
/// handle detaches the task.
pub struct TaskHandle(Box<dyn Fn()>);

impl TaskHandle {
    /// A handle whose [`TaskHandle::abort`] calls `abort`, which may run
    /// more than once.
    pub fn new(abort: impl Fn() + 'static) -> TaskHandle {
        TaskHandle(Box::new(abort))
    }

    /// Cancels the task.
    pub fn abort(&self) {
        (self.0)();
    }
}

impl From<futures_util::future::AbortHandle> for TaskHandle {
    fn from(h: futures_util::future::AbortHandle) -> TaskHandle {
        TaskHandle::new(move || h.abort())
    }
}

impl fmt::Debug for TaskHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TaskHandle").finish_non_exhaustive()
    }
}

/// The configs of the services a profile runs.
#[derive(Clone, Default)]
pub struct ServicesConfig {
    pub network: strims_network::Config,
    pub bootstrap: strims_network::bootstrap::Config,
    pub transfer: strims_transfer::Config,
    pub replication: strims_replication::Config,
    pub directory: strims_directory::Config,
    pub chat: strims_chat::Config,
    pub whispers: WhisperConfig,
    pub video: strims_video::Config,
}

/// What a driver supplies to run a profile's node and services
/// ([`super::ProfileSlot::start`]).
pub trait AppPlatform: 'static {
    /// The services' clock.
    type Timer: Timer;
    /// The video control's RTMP listeners.
    type VideoHost: VideoHost;
    /// The executor of the profile's tasks.
    type Spawn: Spawn;

    /// Fresh entropy for one profile run. It seeds the node and every
    /// control, so a deterministic driver returns a seed it controls.
    fn seed(&self) -> Result<[u8; 32], StartError>;

    /// The node's config for `key`, seeded with `seed`. The start then
    /// sets [`NodeConfig::rate_limit`] and [`NodeConfig::max_peers`] from
    /// the profile's vnic config.
    fn node_config(&self, key: Key, seed: u64) -> NodeConfig;

    /// The driver's caps on the profile's vnic config.
    fn ceilings(&self) -> VnicCeilings;

    /// The services' configs.
    fn services(&self) -> ServicesConfig;

    /// Builds the node from `config` and spawns its task.
    fn spawn_node(
        &self,
        config: NodeConfig,
    ) -> impl Future<Output = Result<(Node, NodeEvents), NodeError>>;

    /// The services' timer.
    fn timer(&self) -> Self::Timer;

    /// The directory servers' HTTP fetcher, for embed snippets.
    fn fetch(&self) -> Fetch;

    /// The video control's RTMP host.
    fn video_host(&self) -> Self::VideoHost;

    /// The routes the profile's HLS streams are served on.
    fn hls_routes(&self) -> HlsRoutes;

    /// The executor of the profile's tasks.
    fn spawner(&self) -> Self::Spawn;
}

/// Why a profile's node didn't start.
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error(transparent)]
    Store(#[from] strims_dao::Error),
    #[error("starting the node: {0}")]
    Node(#[from] NodeError),
    #[error("drawing a seed from entropy: {0}")]
    Entropy(String),
    #[error("profile has no key")]
    NoProfileKey,
    /// The listeners belong to another profile's node.
    #[error("this process runs another profile's node, and it runs one at a time")]
    ProfileRunning,
}
