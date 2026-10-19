// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The node session factory on real localhost sockets: a signed-in
//! profile's node starts with its listeners, a second profile is refused
//! while it runs, binding a session serves every frontend, and the node
//! runs under the profile's vnic config within the factory's ceilings.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::future::Future;
use std::rc::Rc;

use strims_base::rng::det_rng;
use strims_dao::auth::{self, ProfileKey};
use strims_dao::schema::vnic::{DEFAULT_MAX_PEERS, VnicConfig};
use strims_dao::{ProfileStore, Singleton};
use strims_kv::MemoryKv;
use strims_node::app::{AppConfig, NodeFactory, NodeTimer, StartError};
use strims_node::{Clock, Node, SystemClock, TokioNet};
use strims_proto::vnic::v1::Config as VnicRecord;
use strims_qos::DEFAULT_RATE_LIMIT;
use strims_rpc::Router;
use strims_session::{Manager, SessionFactory};
use tokio::task::LocalSet;

#[test]
fn one_profile_runs_per_process_with_every_frontend_bound() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(LocalSet::new().run_until(async {
        let kv = MemoryKv::new();
        let mut rng = det_rng(1);
        let now = SystemClock.now();
        let (alice, _) = auth::sign_up(&kv, "alice", "pw", "test", &mut rng, now)
            .await
            .unwrap();
        let (bob, _) = auth::sign_up(&kv, "bob", "pw", "test", &mut rng, now)
            .await
            .unwrap();
        let config = AppConfig {
            tcp_listen: Some("127.0.0.1:0".into()),
            ws_listen: Some("127.0.0.1:0".into()),
            allow_insecure_ws: true,
            ..AppConfig::default()
        };
        let factory = NodeFactory::new(TokioNet, SystemClock, config);
        let manager = Rc::new(Manager::new(
            kv,
            factory,
            NodeTimer(SystemClock),
            "test",
            det_rng(2),
        ));

        let (session, runtime) = manager.get_or_create(&alice).await.unwrap();
        let info = manager.factory().running().unwrap();
        let ws_uri = info.ws_uri.unwrap();
        assert!(ws_uri.starts_with("ws://127.0.0.1:"), "{ws_uri}");
        assert_eq!(runtime.node().ws_uri(), Some(ws_uri.as_str()));
        let (_, again) = manager.get_or_create(&alice).await.unwrap();
        assert!(Rc::ptr_eq(&runtime, &again));

        assert!(matches!(
            manager.get_or_create(&bob).await,
            Err(StartError::ProfileRunning)
        ));

        let router = Router::new();
        manager.factory().bind(&runtime, &session, &router);
        for method in [
            "strims.profile.v1.ProfileFrontend.Get",
            "strims.vnic.v1.VNICFrontend.GetConfig",
            "strims.notification.v1.NotificationFrontend.Watch",
            "strims.network.v1.NetworkFrontend.CreateServer",
            "strims.network.v1.bootstrap.BootstrapFrontend.GetConfig",
            "strims.network.v1.directory.DirectoryFrontend.WatchListings",
            "strims.chat.v1.ChatFrontend.OpenClient",
            "strims.chat.v1.ChatServerFrontend.CreateServer",
            "strims.video.v1.VideoChannelFrontend.Create",
            "strims.video.v1.VideoIngress.GetChannelURL",
            "strims.video.v1.HLSEgress.OpenStream",
            "strims.video.v1.Egress.OpenStream",
            "strims.video.v1.Capture.Open",
            "strims.autoseed.v1.AutoseedFrontend.CreateRule",
            "strims.debug.v1.Debug.WatchMetrics",
            "strims.debug.v1.Debug.StartMockStream",
        ] {
            assert!(router.contains(method), "{method} isn't bound");
        }
    }));
}

type TestManager = Manager<MemoryKv, NodeFactory<TokioNet, SystemClock>, NodeTimer<SystemClock>>;

fn run_local(f: impl Future<Output = ()>) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(LocalSet::new().run_until(f));
}

fn vnic(upload: u64, peers: u32) -> VnicRecord {
    VnicRecord {
        max_upload_bytes_per_second: upload,
        max_peers: peers,
    }
}

/// A manager on a factory with `config`, and a profile whose store holds
/// `stored`.
async fn manager(config: AppConfig, stored: Option<VnicRecord>) -> (TestManager, ProfileKey) {
    let kv = MemoryKv::new();
    let now = SystemClock.now();
    let (alice, _) = auth::sign_up(&kv, "alice", "pw", "test", &mut det_rng(1), now)
        .await
        .unwrap();
    if let Some(stored) = stored {
        let store = ProfileStore::open(kv.clone(), alice.id, alice.storage_key(), det_rng(3))
            .await
            .unwrap();
        let mut tx = store.write(now).await.unwrap();
        VnicConfig::set(&mut tx, stored).await.unwrap();
        tx.commit().await.unwrap();
    }
    let factory = NodeFactory::new(TokioNet, SystemClock, config);
    let manager = Manager::new(kv, factory, NodeTimer(SystemClock), "test", det_rng(2));
    (manager, alice)
}

/// The vnic limits of the profile's node once its session starts.
async fn limits_at_start(config: AppConfig, stored: Option<VnicRecord>) -> (u64, usize) {
    let (manager, alice) = manager(config, stored).await;
    let (_, runtime) = manager.get_or_create(&alice).await.unwrap();
    runtime.node().vnic_limits().await.unwrap()
}

/// Waits for `node` to run under `want`.
async fn wait_for_limits(node: &Node, want: (u64, usize)) {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let got = node.vnic_limits().await.unwrap();
        if got == want {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "the node runs under {got:?}, not {want:?}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

#[test]
fn a_vnic_config_stored_before_start_is_in_effect_at_start() {
    run_local(async {
        let default = (DEFAULT_RATE_LIMIT, DEFAULT_MAX_PEERS as usize);
        assert_eq!(limits_at_start(AppConfig::default(), None).await, default);
        let stored = Some(vnic(1_000_000, 7));
        assert_eq!(
            limits_at_start(AppConfig::default(), stored).await,
            (1_000_000, 7)
        );
        // a stored 0 is unlimited, and so is any cap past QoS's largest
        let unlimited = (DEFAULT_RATE_LIMIT, i32::MAX as usize);
        let stored = Some(vnic(0, 0));
        assert_eq!(
            limits_at_start(AppConfig::default(), stored).await,
            unlimited
        );
        let stored = Some(vnic(u64::MAX, 0));
        assert_eq!(
            limits_at_start(AppConfig::default(), stored).await,
            unlimited
        );
    });
}

#[test]
fn a_ceiling_caps_a_larger_stored_vnic_config() {
    run_local(async {
        let config = AppConfig {
            max_upload: Some(500_000),
            max_peers: Some(3),
            ..AppConfig::default()
        };
        let (manager, alice) = manager(config, Some(vnic(1_000_000, 7))).await;
        let (session, runtime) = manager.get_or_create(&alice).await.unwrap();
        let node = runtime.node();
        assert_eq!(node.vnic_limits().await.unwrap(), (500_000, 3));

        // the running node follows the config, still under the ceilings
        let mut tx = session.store.write(SystemClock.now()).await.unwrap();
        VnicConfig::set(&mut tx, vnic(100_000, 50)).await.unwrap();
        tx.commit().await.unwrap();
        wait_for_limits(node, (100_000, 3)).await;
        let mut tx = session.store.write(SystemClock.now()).await.unwrap();
        VnicConfig::set(&mut tx, vnic(0, 0)).await.unwrap();
        tx.commit().await.unwrap();
        wait_for_limits(node, (500_000, 3)).await;
    });
}
