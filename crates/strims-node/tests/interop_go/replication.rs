// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Profile replication between a Go and a Rust device of one profile over
//! a direct link: Go's internal/replication control on the helper (the
//! `repl-*` verbs), and `strims_replication` through
//! `strims_node::app` on a Rust node.
//!
//! - `go_pairs_rust`: Go is the first device and makes a pairing token;
//!   the Rust device imports it. Both write, both edit one record, and
//!   the stores converge.
//! - `rust_pairs_go_with_a_large_store`: Rust is the first device, with
//!   whisper messages its bootstrap to Go sends in parts. Rust then
//!   deletes Go's device, and Go applies it.
//!
//! Go writes before the link: its replication only pushes writes made
//! after it has opened the peer.

use std::rc::Rc;
use std::time::Duration as StdDuration;

use prost::Message;
use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_crypto::key_from_seed;
use strims_dao::auth::{self, ProfileKey};
use strims_dao::schema::chat::{ChatUiConfigHighlights, WhisperRecords};
use strims_dao::schema::network::Networks;
use strims_dao::schema::profile::{Devices, Profile};
use strims_dao::{ReplicatedStore, Singleton, Table};
use strims_kv::MemoryKv;
use strims_node::app::{NodeHost, NodeTimer, PeerServices};
use strims_node::{Clock, NetworkOptions, Node, NodeConfig, SystemClock, TokioNet};
use strims_proto::auth::v1::{PairingToken, ServerUserThing, server_user_thing};
use strims_proto::chat::v1::{UiConfigHighlight, WhisperRecord};
use strims_proto::network::v1::Network;
use strims_proto::profile::v1 as profile;
use strims_proto::types::Certificate;
use strims_replication::{Config, ReplicationControl, ReplicationFrontend};
use strims_vpn::VerifyMode;
use tokio::task::{JoinHandle, LocalSet, spawn_local};

use crate::harness::{GoNode, GoOptions, VPN_PORT, helper, network_key};

type Control = ReplicationControl<MemoryKv, NodeHost, NodeTimer<SystemClock>>;

/// A Rust device: its node and replication.
struct RustDevice {
    node: Node,
    run: JoinHandle<()>,
    control: Control,
}

fn now() -> Time {
    SystemClock.now()
}

/// Runs `store`'s profile as a device on a new Rust node.
async fn device(store: ReplicatedStore<MemoryKv>, seed: u8, replication: Config) -> RustDevice {
    let mut tx = store.read().await.unwrap();
    let profile = Profile::get(&mut tx).await.unwrap().unwrap();
    drop(tx);
    let mut config = NodeConfig::new(profile.key.clone().unwrap(), u64::from(seed));
    config.host_key_seed = Some([seed.wrapping_add(0x80); 32]);
    config.label = format!("rust-{seed}");
    config.tcp_listen = Some("127.0.0.1:0".to_owned());
    config.max_peers = 16;
    config.vpn.verify = VerifyMode::Audit;
    let (node, mut events) = Node::spawn(config, TokioNet, SystemClock).await.unwrap();
    let net = node
        .add_network(
            &network_key(),
            NetworkOptions {
                auto_bind_port: Some(VPN_PORT),
            },
        )
        .await
        .unwrap();
    let host = NodeHost::new(node.clone());
    host.add_network(net);
    let control = ReplicationControl::new(
        store.clone(),
        profile,
        host,
        NodeTimer(SystemClock),
        det_rng(u64::from(seed)),
        replication,
    );
    let run = spawn_local({
        let control = control.clone();
        async move { control.run().await }
    });
    let services = PeerServices::new(node.clone()).with(Rc::new(control.clone()));
    spawn_local(async move {
        while let Some(e) = events.recv().await {
            services.handle_event(&e).await;
        }
    });
    RustDevice { node, run, control }
}

async fn rust_highlight(store: &ReplicatedStore<MemoryKv>, alias: &str) -> u64 {
    let mut tx = store.write(now()).await.unwrap();
    let id = tx.generate_id().await.unwrap();
    ChatUiConfigHighlights::insert(
        &mut tx,
        UiConfigHighlight {
            id,
            alias: alias.into(),
            peer_key: alias.as_bytes().to_vec(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    id
}

async fn rust_highlights(store: &ReplicatedStore<MemoryKv>) -> String {
    let mut tx = store.read().await.unwrap();
    let mut all = ChatUiConfigHighlights::get_all(&mut tx).await.unwrap();
    all.sort_by_key(|h| h.id);
    all.iter()
        .map(|h| format!("{}:{}", h.id, h.alias))
        .collect::<Vec<_>>()
        .join("\n")
}

async fn go_text(go: &mut GoNode, verb: &str) -> String {
    let v = go.cmd(verb).await.unwrap();
    if v == "-" {
        return String::new();
    }
    String::from_utf8(hex::decode(v).unwrap()).unwrap()
}

/// Waits until the two stores hold the same highlights, `n` of them.
async fn converge(go: &mut GoNode, store: &ReplicatedStore<MemoryKv>, n: usize) -> String {
    for _ in 0..600 {
        let (g, r) = (
            go_text(go, "repl-highlights").await,
            rust_highlights(store).await,
        );
        if g == r && g.lines().count() == n {
            return g;
        }
        tokio::time::sleep(StdDuration::from_millis(100)).await;
    }
    let (g, r) = (
        go_text(go, "repl-highlights").await,
        rust_highlights(store).await,
    );
    panic!("stores never converged:\ngo:\n{g}\nrust:\n{r}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn go_pairs_rust() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        180,
        LocalSet::new().run_until(async {
            let mut o = GoOptions::new(0x90);
            o.peer_rpc = true;
            o.max_peers = 16;
            let mut go = GoNode::spawn(&bin, "g90", o).await;
            go.cmd(&format!("repl-new {}", hex::encode("go-os")))
                .await
                .unwrap();
            let token = hex::decode(go.cmd("repl-token").await.unwrap()).unwrap();
            let token = PairingToken::decode(token.as_slice()).unwrap();

            let pk = auth::open_user(token.auth.as_ref().unwrap(), "").unwrap();
            let kv = MemoryKv::new();
            let mut rng = det_rng(0x91);
            let store = auth::import_pairing_token(&kv, &token, &pk, &mut rng, now())
                .await
                .unwrap();
            let store = ReplicatedStore::new(store).await.unwrap();
            // The new device's record, as a session writes it.
            let device_id = store.replica_id();
            let mut tx = store.write(now()).await.unwrap();
            Devices::transform(&mut tx, device_id, |d| {
                d.id = device_id;
                d.os = "rust-os".into();
                Ok(())
            })
            .await
            .unwrap();
            tx.commit().await.unwrap();
            go.cmd(&format!("repl-highlight {}", hex::encode("from go")))
                .await
                .unwrap();
            let rust = device(store.clone(), 0x91, Config::default()).await;
            assert_eq!(rust.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);
            rust_highlight(&store, "from rust").await;
            tokio::time::sleep(StdDuration::from_secs(5)).await;
            eprintln!("rust metrics {:?}", rust.control.metrics());
            let both = converge(&mut go, &store, 2).await;
            eprintln!("converged:\n{both}");

            // Go's record, renamed by both.
            let go_id: u64 = both
                .lines()
                .find(|l| l.ends_with(":from go"))
                .unwrap()
                .split(':')
                .next()
                .unwrap()
                .parse()
                .unwrap();
            go.cmd(&format!(
                "repl-edit {go_id} {}",
                hex::encode("renamed by go")
            ))
            .await
            .unwrap();
            let mut tx = store.write(now()).await.unwrap();
            ChatUiConfigHighlights::transform(&mut tx, go_id, |h| {
                h.alias = "renamed by rust".into();
                Ok(())
            })
            .await
            .unwrap();
            tx.commit().await.unwrap();
            let after = converge(&mut go, &store, 2).await;
            assert!(after.contains("renamed by"), "{after}");

            // The Rust device's record reached Go.
            let go_devices = go_text(&mut go, "repl-devices").await;
            assert!(
                go_devices
                    .split(',')
                    .any(|d| d == store.replica_id().to_string()),
                "{go_devices}"
            );
            rust.run.abort();
        }),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rust_pairs_go_with_a_large_store() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        180,
        LocalSet::new().run_until(async {
            let seed: u8 = 0x92;
            let kv = MemoryKv::new();
            let mut rng = det_rng(u64::from(seed));
            // The Go helper's node runs with this profile key.
            let key = key_from_seed(&[seed; 32]);
            let profile = profile::Profile {
                id: 4242,
                name: "rusty".into(),
                key: Some(key),
                ..Default::default()
            };
            let pk = ProfileKey {
                id: profile.id,
                key: [7; 32],
            };
            let user = ServerUserThing {
                name: "rusty".into(),
                credentials: Some(server_user_thing::Credentials::Unencrypted(
                    server_user_thing::Unencrypted {
                        profile_id: pk.id,
                        profile_key: pk.key.to_vec(),
                    },
                )),
                ..Default::default()
            };
            let store = auth::store_profile(&kv, &user, &profile, &pk, &mut rng, now())
                .await
                .unwrap();
            auth::init_device(&store, "rust-os", now()).await.unwrap();
            let store = ReplicatedStore::new(store).await.unwrap();

            let mut tx = store.write(now()).await.unwrap();
            let id = tx.generate_id().await.unwrap();
            Networks::insert(
                &mut tx,
                Network {
                    id,
                    certificate: Some(Certificate {
                        key: network_key(),
                        not_after: 1 << 62,
                        ..Default::default()
                    }),
                    alias: "net".into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            for i in 0..100u64 {
                let id = tx.generate_id().await.unwrap();
                WhisperRecords::insert(
                    &mut tx,
                    WhisperRecord {
                        id,
                        peer_key: vec![1; 32],
                        peer_message_id: i + 1,
                        message: Some(strims_proto::chat::v1::Message {
                            body: "x".repeat(2000),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
            }
            tx.commit().await.unwrap();
            rust_highlight(&store, "from rust").await;

            let token = ReplicationFrontend::new(store.clone(), NodeTimer(SystemClock))
                .pairing_token(0, 0)
                .await
                .unwrap();
            let go_device = token.profile.as_ref().unwrap().device_id;
            let mut o = GoOptions::new(seed);
            o.peer_rpc = true;
            o.max_peers = 16;
            let mut go = GoNode::spawn(&bin, "g92", o).await;
            go.cmd(&format!(
                "repl-import {}",
                hex::encode(token.encode_to_vec())
            ))
            .await
            .unwrap();

            go.cmd(&format!("repl-highlight {}", hex::encode("from go")))
                .await
                .unwrap();
            // Parts of 64 KiB: Rust's bootstrap to Go comes in several.
            let config = Config {
                message_budget: 64 * 1024,
                ..Config::default()
            };
            let rust = device(store.clone(), seed.wrapping_add(1), config).await;
            assert_eq!(rust.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);
            converge(&mut go, &store, 2).await;
            assert!(
                rust.control.metrics().bootstraps_sent >= 3,
                "{:?}",
                rust.control.metrics()
            );

            // Go's device leaves the profile once its record is here; Go
            // applies it.
            for _ in 0..300 {
                let mut tx = store.read().await.unwrap();
                if Devices::get(&mut tx, go_device).await.unwrap().is_some() {
                    break;
                }
                drop(tx);
                tokio::time::sleep(StdDuration::from_millis(100)).await;
            }
            let mut tx = store.write(now()).await.unwrap();
            assert!(Devices::delete(&mut tx, go_device).await.unwrap());
            tx.commit().await.unwrap();
            for _ in 0..300 {
                let devices = go_text(&mut go, "repl-devices").await;
                if !devices.split(',').any(|d| d == go_device.to_string()) {
                    return;
                }
                tokio::time::sleep(StdDuration::from_millis(100)).await;
            }
            panic!("go never applied its device's deletion");
        }),
    )
    .await;
}
