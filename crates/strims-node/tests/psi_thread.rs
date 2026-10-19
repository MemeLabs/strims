// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Profiles whose PSI sessions compute on threads, on real localhost
//! sockets: two node session factories under `PsiMode::Thread` start a
//! PSI thread each, their profiles join a network through an invitation
//! and bind, and the threads end once the profiles are gone.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::rc::Rc;
use std::time::Duration as StdDuration;

use futures_util::{Sink, Stream};
use prost::Message;
use strims_base::rng::det_rng;
use strims_dao::auth;
use strims_kv::MemoryKv;
use strims_node::app::{AppConfig, AppRuntime, NodeFactory, NodeTimer, PsiMode};
use strims_node::psi::THREAD_NAME;
use strims_node::{Clock, Node, SystemClock, TokioNet};
use strims_proto::network::v1::bootstrap::{
    BootstrapClientWebSocketOptions, BootstrapFrontendClient, CreateBootstrapClientRequest,
    create_bootstrap_client_request::ClientOptions,
};
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateNetworkFromInvitationRequest, CreateServerRequest,
    NetworkFrontendClient,
};
use strims_rpc::{Client, Config as RpcConfig, Router, connect};
use strims_session::{Manager, SessionFactory};
use tokio::sync::mpsc;
use tokio::task::{AbortHandle, LocalSet, spawn_local};

const STEP: StdDuration = StdDuration::from_secs(60);

type TestManager = Manager<MemoryKv, NodeFactory<TokioNet, SystemClock>, NodeTimer<SystemClock>>;

/// The threads of this process named as a `ThreadPsi`'s, where the OS
/// lists them.
fn psi_threads() -> Option<usize> {
    let tasks = std::fs::read_dir("/proc/self/task").ok()?;
    let named = tasks
        .filter_map(|t| std::fs::read_to_string(t.ok()?.path().join("comm")).ok())
        .filter(|name| name.trim_end() == THREAD_NAME)
        .count();
    Some(named)
}

/// Waits until `cond` holds, polling every 50 ms.
async fn eventually(what: &str, mut cond: impl AsyncFnMut() -> bool) {
    let deadline = tokio::time::Instant::now() + STEP;
    while !cond().await {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for {what}"
        );
        tokio::time::sleep(StdDuration::from_millis(50)).await;
    }
}

#[derive(Debug)]
struct Closed;

fn reader(rx: mpsc::UnboundedReceiver<Vec<u8>>) -> impl Stream<Item = Result<Vec<u8>, Closed>> {
    Box::pin(futures_util::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|m| (Ok(m), rx))
    }))
}

fn writer(tx: mpsc::UnboundedSender<Vec<u8>>) -> impl Sink<Vec<u8>, Error = Closed> {
    Box::pin(futures_util::sink::unfold(
        tx,
        |tx, m: Vec<u8>| async move {
            tx.send(m).map_err(|_| Closed)?;
            Ok(tx)
        },
    ))
}

/// A client of `router`'s methods over an in-process connection, and the
/// connection's two tasks.
fn frontend(router: Router) -> (Client, [AbortHandle; 2]) {
    let (to_server, from_client) = mpsc::unbounded_channel();
    let (to_client, from_server) = mpsc::unbounded_channel();
    let (client, conn) = connect(
        RpcConfig::frontend(),
        reader(from_server),
        writer(to_server),
        None,
    );
    let (_, server) = connect(
        RpcConfig::frontend(),
        reader(from_client),
        writer(to_client),
        Some(router),
    );
    let tasks = [
        spawn_local(async move {
            let _ = conn.await;
        })
        .abort_handle(),
        spawn_local(async move {
            let _ = server.await;
        })
        .abort_handle(),
    ];
    (client, tasks)
}

/// A signed-in profile on its own factory, with its frontends served.
struct Profile {
    manager: TestManager,
    runtime: Rc<AppRuntime<MemoryKv, TokioNet, SystemClock>>,
    client: Client,
    tasks: [AbortHandle; 2],
}

impl Profile {
    async fn start(name: &str, seed: u64) -> Profile {
        let kv = MemoryKv::new();
        let (key, _) = auth::sign_up(
            &kv,
            name,
            "pw",
            "test",
            &mut det_rng(seed),
            SystemClock.now(),
        )
        .await
        .unwrap();
        let config = AppConfig {
            tcp_listen: Some("127.0.0.1:0".into()),
            ws_listen: Some("127.0.0.1:0".into()),
            allow_insecure_ws: true,
            label: name.into(),
            psi: PsiMode::Thread,
            ..AppConfig::default()
        };
        let factory = NodeFactory::new(TokioNet, SystemClock, config);
        let manager = Manager::new(
            kv,
            factory,
            NodeTimer(SystemClock),
            "test",
            det_rng(seed + 1),
        );
        let (session, runtime) = manager.get_or_create(&key).await.unwrap();
        let router = Router::new();
        manager.factory().bind(&runtime, &session, &router);
        let (client, tasks) = frontend(router);
        Profile {
            manager,
            runtime,
            client,
            tasks,
        }
    }

    fn node(&self) -> &Node {
        self.runtime.node()
    }

    fn networks(&self) -> NetworkFrontendClient<Client> {
        NetworkFrontendClient::new(self.client.clone())
    }

    /// Whether the node has a network with a bound peer.
    async fn bound(&self) -> bool {
        let metrics = self.node().metrics().await.unwrap();
        metrics.networks.iter().any(|n| n.links > 0)
    }

    /// Stops serving the frontends, and drops the profile's runtime and
    /// factory.
    fn stop(self) {
        for t in &self.tasks {
            t.abort();
        }
    }
}

#[test]
fn profiles_computing_psi_on_threads_join_a_network_and_bind() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(LocalSet::new().run_until(async {
        assert_eq!(psi_threads().unwrap_or(0), 0);
        let alice = Profile::start("alice", 1).await;
        let bob = Profile::start("bob", 10).await;
        // A thread takes its name once it runs, which can be after its
        // factory has started.
        if psi_threads().is_some() {
            eventually("one psi thread per factory", async || {
                psi_threads() == Some(2)
            })
            .await;
        }

        let network = alice
            .networks()
            .create_server(&CreateServerRequest {
                name: "net".into(),
                ..Default::default()
            })
            .await
            .unwrap()
            .network
            .unwrap();
        let ws_uri = alice.manager.factory().running().unwrap().ws_uri.unwrap();
        let bootstrap = BootstrapFrontendClient::new(alice.client.clone())
            .create_client(&CreateBootstrapClientRequest {
                client_options: Some(ClientOptions::WebsocketOptions(
                    BootstrapClientWebSocketOptions {
                        url: ws_uri,
                        insecure_skip_verify_tls: false,
                    },
                )),
            })
            .await
            .unwrap()
            .bootstrap_client
            .unwrap();
        let invitation = alice
            .networks()
            .create_invitation(&CreateInvitationRequest {
                network_id: network.id,
                bootstrap_client_id: bootstrap.id,
            })
            .await
            .unwrap()
            .invitation
            .unwrap();
        bob.networks()
            .create_network_from_invitation(&CreateNetworkFromInvitationRequest {
                alias: String::new(),
                invitation: Some(InvitationField::InvitationBytes(invitation.encode_to_vec())),
            })
            .await
            .unwrap();

        eventually("alice and bob to bind", async || {
            alice.bound().await && bob.bound().await
        })
        .await;

        alice.stop();
        bob.stop();
        if psi_threads().is_some() {
            eventually("the psi threads to end", async || psi_threads() == Some(0)).await;
        }
    }));
}
