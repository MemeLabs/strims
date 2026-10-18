// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The node session factory on real localhost sockets: a signed-in
//! profile's node starts with its listeners, a second profile is refused
//! while it runs, and binding a session serves every frontend.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::rc::Rc;

use strims_base::rng::det_rng;
use strims_dao::auth;
use strims_kv::MemoryKv;
use strims_node::app::{AppConfig, NodeFactory, NodeTimer, StartError};
use strims_node::{Clock, SystemClock, TokioNet};
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
        ] {
            assert!(router.contains(method), "{method} isn't bound");
        }
    }));
}
