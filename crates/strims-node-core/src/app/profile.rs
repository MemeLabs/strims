// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A signed-in profile's node and controls, one profile at a time.

use std::cell::RefCell;
use std::rc::Rc;

use futures_util::future::LocalBoxFuture;
use rand_core::SeedableRng;
use strims_base::rng::{DetRng, fork};
use strims_chat::{ChatControl, WhisperControl};
use strims_debug::DebugControl;
use strims_directory::DirectoryControl;
use strims_kv::Kv;
use strims_network::NetworkControl;
use strims_network::bootstrap::BootstrapControl;
use strims_replication::ReplicationControl;
use strims_rpc::{Router, Timer};
use strims_session::{Session, read_vnic_limits, run_vnic};
use strims_transfer::TransferControl;
use strims_video::{VideoControl, VideoHost};

use super::host::node_limits;
use super::metrics::ProfileMetrics;
use super::{AppPlatform, MemoryMetrics, NodeHost, PeerServices, Spawn, StartError, TaskHandle};
use crate::{Node, NodeInfo};

/// The profile whose node runs on a platform. A platform runs one
/// profile's node at a time, since the node listens on the platform's
/// addresses: starting another profile fails with
/// [`StartError::ProfileRunning`].
#[derive(Debug, Default)]
pub struct ProfileSlot {
    /// The profile whose node runs or starts, and the node once it runs.
    running: RefCell<Option<(u64, Option<NodeInfo>)>>,
}

impl ProfileSlot {
    /// A slot with no profile.
    pub fn new() -> ProfileSlot {
        ProfileSlot::default()
    }

    /// The running node, if a session started one.
    pub fn running(&self) -> Option<NodeInfo> {
        self.running.borrow().as_ref().and_then(|(_, n)| n.clone())
    }

    /// Starts `session`'s node on `platform`, under the profile's vnic
    /// config within [`AppPlatform::ceilings`], and runs replication,
    /// transfers, network membership, bootstrap, the directory, chat,
    /// whispers, video and debug, each control as a local task, serving every
    /// peer through [`PeerServices`]. The node follows the vnic config's
    /// changes.
    ///
    /// Fails with [`StartError::ProfileRunning`] while another profile's
    /// node runs or starts. A start that fails, or is dropped unfinished,
    /// frees the slot.
    pub async fn start<K: Kv, P: AppPlatform>(
        &self,
        platform: &P,
        session: &Rc<Session<K>>,
    ) -> Result<ProfileRuntime<K, P::Timer, P::VideoHost, P::Spawn>, StartError> {
        let id = session.profile.id;
        {
            let mut running = self.running.borrow_mut();
            match &*running {
                Some((other, _)) if *other != id => return Err(StartError::ProfileRunning),
                _ => *running = Some((id, None)),
            }
        }
        // Dropped unstarted, a start gives the platform's one node back.
        let mut mark = StartMark {
            running: &self.running,
            started: false,
        };
        let runtime = start_profile(platform, session).await?;
        *self.running.borrow_mut() = Some((id, Some(runtime.node.info().clone())));
        mark.started = true;
        Ok(runtime)
    }
}

/// Clears the slot's running profile unless its start finished.
struct StartMark<'a> {
    running: &'a RefCell<Option<(u64, Option<NodeInfo>)>>,
    started: bool,
}

impl Drop for StartMark<'_> {
    fn drop(&mut self) {
        if !self.started {
            *self.running.borrow_mut() = None;
        }
    }
}

/// The node and controls of `session`. The seed's rng forks and the spawns
/// happen in the orders the [module docs](super#determinism) fix.
async fn start_profile<K: Kv, P: AppPlatform>(
    platform: &P,
    session: &Rc<Session<K>>,
) -> Result<ProfileRuntime<K, P::Timer, P::VideoHost, P::Spawn>, StartError> {
    let key = session
        .profile
        .key
        .clone()
        .ok_or(StartError::NoProfileKey)?;
    let mut rng = DetRng::from_seed(platform.seed()?);
    let node_seed = rand_core::Rng::next_u64(&mut rng);
    let mut config = platform.node_config(key, node_seed);
    let ceilings = platform.ceilings();
    let limits = read_vnic_limits(&session.store, &ceilings).await?;
    let (rate_limit, max_peers) = node_limits(limits);
    config.rate_limit = Some(rate_limit);
    config.max_peers = max_peers;
    let (node, mut events) = platform.spawn_node(config).await?;

    let c = platform.services();
    let timer = platform.timer();
    let (store, profile) = (&session.store, &session.profile);
    let host = NodeHost::new(node.clone());
    let replication = ReplicationControl::new(
        store.clone(),
        profile.clone(),
        host.clone(),
        timer.clone(),
        fork(&mut rng),
        c.replication,
    );
    let network = NetworkControl::new(
        host.clone(),
        store.clone(),
        profile.clone(),
        timer.clone(),
        fork(&mut rng),
        c.network,
    )
    .with_psi(platform.psi());
    let bootstrap = BootstrapControl::new(
        host.clone(),
        store.clone(),
        profile.clone(),
        timer.clone(),
        fork(&mut rng),
        c.bootstrap,
    );
    let transfers = TransferControl::new(host.clone(), timer.clone(), c.transfer);
    let directory = DirectoryControl::new(
        host.clone(),
        store.clone(),
        network.clone(),
        transfers.clone(),
        timer.clone(),
        fork(&mut rng),
        c.directory,
    )
    .with_fetch(platform.fetch());
    let chat = ChatControl::new(
        host.clone(),
        store.clone(),
        profile.clone(),
        network.clone(),
        transfers.clone(),
        directory.clone(),
        timer.clone(),
        fork(&mut rng),
        c.chat,
    );
    let whispers = WhisperControl::new(
        host.clone(),
        store.clone(),
        profile.clone(),
        network.clone(),
        timer.clone(),
        fork(&mut rng),
        c.whispers,
    );
    let video = VideoControl::new(
        host.clone(),
        platform.video_host(),
        store.clone(),
        profile.clone(),
        network.clone(),
        transfers.clone(),
        directory.clone(),
        platform.hls_routes(),
        timer.clone(),
        fork(&mut rng),
        c.video,
    );
    let debug = DebugControl::new(
        host.clone(),
        store.clone(),
        network.clone(),
        transfers.clone(),
        directory.clone(),
        timer.clone(),
        fork(&mut rng),
        c.debug,
    );

    let spawner = platform.spawner();
    let mut tasks = Vec::new();
    let mut spawn = |f: LocalBoxFuture<'static, ()>| {
        tasks.push(spawner.spawn_local(f));
    };
    spawn({
        let replication = replication.clone();
        Box::pin(async move { replication.run().await })
    });
    spawn({
        let network = network.clone();
        Box::pin(async move { network.run().await })
    });
    spawn({
        let bootstrap = bootstrap.clone();
        Box::pin(async move { bootstrap.run().await })
    });
    spawn({
        let transfers = transfers.clone();
        Box::pin(async move { transfers.run().await })
    });
    spawn({
        let directory = directory.clone();
        Box::pin(async move { directory.run().await })
    });
    spawn({
        let chat = chat.clone();
        Box::pin(async move { chat.run().await })
    });
    spawn({
        let whispers = whispers.clone();
        Box::pin(async move { whispers.run().await })
    });
    spawn({
        let video = video.clone();
        Box::pin(async move { video.run().await })
    });
    spawn(Box::pin(run_vnic(store.clone(), host, ceilings)));
    let services = PeerServices::with_spawner(node.clone(), spawner.clone())
        .with(Rc::new(network.clone()))
        .with(Rc::new(transfers.clone()))
        .with(Rc::new(replication.clone()))
        .with(Rc::new(bootstrap.clone()));
    spawn(Box::pin(async move {
        while let Some(e) = events.recv().await {
            services.handle_event(&e).await;
        }
    }));
    spawn({
        let debug = debug.clone();
        Box::pin(async move { debug.run().await })
    });

    tracing::info!(
        profile = %session.profile.name,
        ws_uri = node.ws_uri().unwrap_or_default(),
        tcp_uri = node.tcp_uri().unwrap_or_default(),
        "node started",
    );
    Ok(ProfileRuntime {
        node,
        timer,
        network,
        bootstrap,
        transfers,
        replication,
        directory,
        chat,
        whispers,
        video,
        debug,
        memory: platform.memory(),
        tasks,
        spawn: spawner,
    })
}

/// Serves `session`'s frontends on a frontend connection's `router`: the
/// profile's own, and those of `runtime`'s controls.
pub fn bind_profile<K: Kv, T: Timer, V: VideoHost, S: Spawn>(
    runtime: &ProfileRuntime<K, T, V, S>,
    session: &Rc<Session<K>>,
    router: &Router,
) {
    strims_session::bind_frontends(session, &runtime.timer, router);
    strims_network::bind_frontend(router, &runtime.network);
    strims_network::bootstrap::bind_frontend(router, &runtime.bootstrap);
    strims_directory::bind_frontend(router, &runtime.directory);
    strims_chat::bind_frontend(router, &runtime.chat, &runtime.whispers);
    strims_video::bind_frontend(router, &runtime.video);
    strims_debug::bind_frontend(
        router,
        &runtime.debug,
        ProfileMetrics {
            node: runtime.node.clone(),
            network: runtime.network.clone(),
            transfers: runtime.transfers.clone(),
            replication: runtime.replication.clone(),
            directory: runtime.directory.clone(),
            chat: runtime.chat.clone(),
            whispers: runtime.whispers.clone(),
            video: runtime.video.clone(),
            memory: runtime.memory,
        },
    );
}

/// A running profile's node and the controls its frontends serve
/// ([`bind_profile`]). Dropping it aborts the controls' tasks and hands
/// the node to [`Spawn::shutdown`].
pub struct ProfileRuntime<K: Kv, T: Timer, V: VideoHost, S: Spawn> {
    node: Node,
    timer: T,
    network: NetworkControl<K, NodeHost, T>,
    bootstrap: BootstrapControl<K, NodeHost, T>,
    transfers: TransferControl<NodeHost, T>,
    replication: ReplicationControl<K, NodeHost, T>,
    directory: DirectoryControl<K, NodeHost, T>,
    chat: ChatControl<K, NodeHost, T>,
    whispers: WhisperControl<K, NodeHost, T>,
    video: VideoControl<K, NodeHost, V, T>,
    debug: DebugControl<K, NodeHost, T>,
    /// The platform's heap counters ([`AppPlatform::memory`]).
    memory: Option<fn() -> MemoryMetrics>,
    tasks: Vec<TaskHandle>,
    spawn: S,
}

impl<K: Kv, T: Timer, V: VideoHost, S: Spawn> ProfileRuntime<K, T, V, S> {
    /// The profile's node.
    pub fn node(&self) -> &Node {
        &self.node
    }

    /// The profile's video control, which its frontends serve. A driver
    /// that feeds players on channels of its own reads streams through it.
    pub fn video(&self) -> &VideoControl<K, NodeHost, V, T> {
        &self.video
    }
}

impl<K: Kv, T: Timer, V: VideoHost, S: Spawn> Drop for ProfileRuntime<K, T, V, S> {
    fn drop(&mut self) {
        for t in &self.tasks {
            t.abort();
        }
        self.spawn.shutdown(self.node.clone());
    }
}
