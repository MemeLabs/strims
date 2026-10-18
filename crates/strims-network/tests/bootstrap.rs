// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Bootstrap controls on fake nodes (see `support`), with fake dialers:
//! which bootstrap clients are dialed and when, the bootstrap peer service
//! between linked controls, publishing networks to peers, and the UI's
//! bootstrap frontend.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use data_encoding::HEXLOWER;
use strims_base::rng::det_rng;
use strims_crypto::{
    DEFAULT_CERT_TTL, certificate_network_key, generate_key, membership_trusted,
    new_certificate_request, sign_certificate_request_with_network,
};
use strims_dao::Table;
use strims_dao::schema::network::Networks;
use strims_kademlia::ID;
use strims_kv::MemoryKv;
use strims_network::NetworkFrontend;
use strims_network::bootstrap::{BootstrapControl, BootstrapFrontend, Config, Failure, Host};
use strims_proto::network::v1::bootstrap::{
    BootstrapClientWebSocketOptions, BootstrapFrontendService, BootstrapPeerPublishRequest,
    Config as BootstrapConfig, CreateBootstrapClientRequest, DeleteBootstrapClientRequest,
    GetConfigRequest, ListBootstrapClientsRequest, ListBootstrapPeersRequest, PeerServiceClient,
    PublishNetworkToBootstrapPeerRequest, SetConfigRequest, UpdateBootstrapClientRequest,
    create_bootstrap_client_request, update_bootstrap_client_request,
};
use strims_proto::network::v1::{CreateServerRequest, Network, NetworkFrontendService};
use strims_proto::service::Context;
use strims_proto::types::KeyUsage;
use strims_rpc::{Client, Router, peer_connection};
use support::*;
use tokio::task::spawn_local;
use tokio::time::{Instant, sleep};

#[derive(Default)]
struct DialerState {
    /// Every dial, with when it was made.
    dials: Vec<(String, Instant)>,
    /// What dialing each URL gives; others fail as `Retry`.
    results: BTreeMap<String, Result<ID, Failure>>,
    own: Vec<String>,
}

#[derive(Clone, Default)]
struct Dialer(Rc<RefCell<DialerState>>);

impl Dialer {
    fn dials(&self) -> Vec<String> {
        self.0
            .borrow()
            .dials
            .iter()
            .map(|(u, _)| u.clone())
            .collect()
    }

    fn times(&self) -> Vec<Instant> {
        self.0.borrow().dials.iter().map(|(_, t)| *t).collect()
    }

    fn answer(&self, url: &str, r: Result<ID, Failure>) {
        self.0.borrow_mut().results.insert(url.to_owned(), r);
    }
}

impl Host for Dialer {
    async fn dial(&self, uri: &str) -> Result<ID, Failure> {
        let mut s = self.0.borrow_mut();
        s.dials.push((uri.to_owned(), Instant::now()));
        s.results
            .get(uri)
            .cloned()
            .unwrap_or_else(|| Err(Failure::Retry("refused".into())))
    }

    fn own_uris(&self) -> Vec<String> {
        self.0.borrow().own.clone()
    }
}

type Control = BootstrapControl<MemoryKv, Dialer, TokioTimer>;
type Frontend = BootstrapFrontend<MemoryKv, Dialer, TokioTimer>;

struct Boot {
    node: Node,
    dialer: Dialer,
    control: Control,
    frontend: Frontend,
}

impl Boot {
    fn start(&self) {
        let c = self.control.clone();
        spawn_local(async move { c.run().await });
    }

    async fn add_client(&self, url: &str, insecure: bool) -> u64 {
        self.frontend
            .create_client(
                &ctx(),
                CreateBootstrapClientRequest {
                    client_options: Some(
                        create_bootstrap_client_request::ClientOptions::WebsocketOptions(
                            BootstrapClientWebSocketOptions {
                                url: url.into(),
                                insecure_skip_verify_tls: insecure,
                            },
                        ),
                    ),
                },
            )
            .await
            .unwrap()
            .bootstrap_client
            .unwrap()
            .id
    }

    async fn set_publishing(&self, on: bool) {
        self.frontend
            .set_config(
                &ctx(),
                SetConfigRequest {
                    config: Some(BootstrapConfig {
                        enable_publishing: on,
                    }),
                },
            )
            .await
            .unwrap();
    }

    async fn create_network(&self) -> Network {
        NetworkFrontend::new(self.node.control.clone())
            .create_server(
                &ctx(),
                CreateServerRequest {
                    name: "net".into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .network
            .unwrap()
    }

    async fn peer_ids(&self) -> Vec<(u64, String)> {
        self.frontend
            .list_peers(&ctx(), ListBootstrapPeersRequest {})
            .await
            .unwrap()
            .peers
            .into_iter()
            .map(|p| (p.peer_id, p.label))
            .collect()
    }
}

async fn boot(world: &World, n: u8) -> Boot {
    let node = node_in(world, n).await;
    let dialer = Dialer::default();
    let control = BootstrapControl::new(
        dialer.clone(),
        node.store.clone(),
        node.profile.clone(),
        TokioTimer {
            base: Instant::now(),
        },
        det_rng(u64::from(n) + 10),
        Config::default(),
    );
    Boot {
        frontend: BootstrapFrontend::new(control.clone()),
        node,
        dialer,
        control,
    }
}

fn ctx() -> Context {
    Context::default()
}

/// Links `a` and `b`'s controls as the node would, over an RPC channel
/// pair each way. Returns each side's client of the other.
fn link(a: &Boot, b: &Boot) -> (Client, Client) {
    let pipes: Vec<Pipe> = (0..4).map(|_| Pipe::default()).collect();
    let [ab, ba, ab2, ba2] = <[Pipe; 4]>::try_from(pipes).ok().unwrap();
    let (a_router, b_router) = (Router::new(), Router::new());
    let (a_client, a_conn) = peer_connection(
        strims_rpc::Config::peer(),
        (Reader(ba.clone()), Writer(ab.clone())),
        (Reader(ba2.clone()), Writer(ab2.clone())),
        a_router.clone(),
    );
    let (b_client, b_conn) = peer_connection(
        strims_rpc::Config::peer(),
        (Reader(ab2), Writer(ba2)),
        (Reader(ab), Writer(ba)),
        b_router.clone(),
    );
    a.control.add_peer(
        b.node.id,
        &b.node.profile_key().public,
        &a_router,
        a_client.clone(),
    );
    b.control.add_peer(
        a.node.id,
        &a.node.profile_key().public,
        &b_router,
        b_client.clone(),
    );
    spawn_local(async move {
        let _ = a_conn.await;
    });
    spawn_local(async move {
        let _ = b_conn.await;
    });
    (a_client, b_client)
}

#[test]
fn clients_are_dialed_at_start_and_on_change() {
    run(async {
        let a = boot(&World::default(), 1).await;
        a.dialer.answer("ws://one/x", Ok(ID::from_bytes(&[9; 32])));
        a.dialer.answer("ws://two/x", Ok(ID::from_bytes(&[8; 32])));
        a.add_client("ws://one/x", false).await;
        a.start();
        sleep(StdDuration::from_millis(100)).await;
        assert_eq!(a.dialer.dials(), ["ws://one/x"]);
        a.add_client("ws://two/x", true).await;
        sleep(StdDuration::from_millis(100)).await;
        assert_eq!(a.dialer.dials(), ["ws://one/x", "ws://two/x#insecure"]);
    });
}

#[test]
fn this_nodes_own_uri_is_never_dialed() {
    run(async {
        let a = boot(&World::default(), 1).await;
        a.dialer.0.borrow_mut().own = vec!["ws://me/x".into()];
        a.start();
        a.add_client("ws://me/x", true).await;
        a.add_client("ws://other/x", false).await;
        sleep(StdDuration::from_secs(60)).await;
        let dials = a.dialer.dials();
        assert!(!dials.is_empty());
        assert!(dials.iter().all(|u| u == "ws://other/x"), "{dials:?}");
    });
}

#[test]
fn a_replaced_link_is_not_redialed() {
    run(async {
        let world = World::default();
        let (a, b) = (boot(&world, 1).await, boot(&world, 2).await);
        a.dialer.answer("ws://b/x", Ok(b.node.id));
        a.start();
        a.add_client("ws://b/x", false).await;
        sleep(StdDuration::from_millis(100)).await;
        link(&a, &b);
        sleep(StdDuration::from_secs(5)).await;
        a.control.remove_peer(b.node.id);
        link(&a, &b);
        sleep(StdDuration::from_secs(600)).await;
        assert_eq!(a.dialer.dials(), ["ws://b/x"]);
    });
}

#[test]
fn a_lost_link_is_redialed_with_backoff() {
    run(async {
        let world = World::default();
        let (a, b) = (boot(&world, 1).await, boot(&world, 2).await);
        a.dialer.answer("ws://b/x", Ok(b.node.id));
        a.start();
        a.add_client("ws://b/x", false).await;
        sleep(StdDuration::from_millis(100)).await;
        link(&a, &b);
        sleep(StdDuration::from_secs(1)).await;
        a.dialer
            .answer("ws://b/x", Err(Failure::Retry("gone".into())));
        let lost = Instant::now();
        a.control.remove_peer(b.node.id);
        sleep(StdDuration::from_millis(1500)).await;
        assert_eq!(a.dialer.dials().len(), 1, "the relink grace holds");
        sleep(StdDuration::from_secs(60)).await;
        let times = a.dialer.times();
        assert!(times.len() >= 6, "{times:?}");
        assert!(times[1] - lost >= StdDuration::from_secs(2));
        let gaps: Vec<StdDuration> = times[1..].windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps[3] >= StdDuration::from_secs(8), "{gaps:?}");
        assert!(gaps[3] > gaps[0], "{gaps:?}");
    });
}

#[test]
fn deleting_a_client_stops_its_redials() {
    run(async {
        let a = boot(&World::default(), 1).await;
        a.start();
        let id = a.add_client("ws://down/x", false).await;
        sleep(StdDuration::from_secs(10)).await;
        assert!(a.dialer.dials().len() >= 2);
        a.frontend
            .delete_client(&ctx(), DeleteBootstrapClientRequest { id })
            .await
            .unwrap();
        sleep(StdDuration::from_millis(1)).await;
        let n = a.dialer.dials().len();
        sleep(StdDuration::from_secs(600)).await;
        assert_eq!(a.dialer.dials().len(), n);
        assert!(
            a.frontend
                .delete_client(&ctx(), DeleteBootstrapClientRequest { id })
                .await
                .is_err()
        );
    });
}

#[test]
fn only_peers_that_publish_are_listed() {
    run(async {
        let world = World::default();
        let (a, b, c) = (
            boot(&world, 1).await,
            boot(&world, 2).await,
            boot(&world, 3).await,
        );
        b.set_publishing(true).await;
        for n in [&a, &b, &c] {
            n.start();
        }
        link(&a, &b);
        link(&a, &c);
        sleep(StdDuration::from_secs(1)).await;
        let peers = a.peer_ids().await;
        assert_eq!(peers.len(), 1, "{peers:?}");
        assert_eq!(peers[0].1, HEXLOWER.encode(&b.node.profile_key().public));
        a.control.remove_peer(b.node.id);
        assert!(a.peer_ids().await.is_empty());
    });
}

#[test]
fn publishing_to_a_peer_that_refuses_fails() {
    run(async {
        let world = World::default();
        let (a, b) = (boot(&world, 1).await, boot(&world, 2).await);
        b.set_publishing(true).await;
        a.start();
        b.start();
        link(&a, &b);
        sleep(StdDuration::from_secs(1)).await;
        let network = a.create_network().await;
        let peer = a.peer_ids().await[0].0;
        let c = ctx();
        let publish = |peer_id| {
            a.frontend.publish_network_to_peer(
                &c,
                PublishNetworkToBootstrapPeerRequest {
                    peer_id,
                    network_id: network.id,
                },
            )
        };
        let e = publish(peer + 100).await.unwrap_err();
        assert!(e.message.contains("peer id not found"), "{e:?}");
        // The answer was given at link time; the peer refuses now.
        b.set_publishing(false).await;
        let e = publish(peer).await.unwrap_err();
        assert!(e.message.contains("not supported"), "{e:?}");
        let mut tx = b.node.store.read().await.unwrap();
        assert!(Networks::get_all(&mut tx).await.unwrap().is_empty());
    });
}

#[test]
fn a_published_network_joins_the_receivers_store() {
    run(async {
        let world = World::default();
        let (a, b) = (boot(&world, 1).await, boot(&world, 2).await);
        b.set_publishing(true).await;
        a.start();
        b.start();
        link(&a, &b);
        sleep(StdDuration::from_secs(1)).await;
        let network = a.create_network().await;
        let peer = a.peer_ids().await[0].0;
        let req = PublishNetworkToBootstrapPeerRequest {
            peer_id: peer,
            network_id: network.id,
        };
        a.frontend
            .publish_network_to_peer(&ctx(), req)
            .await
            .unwrap();

        let mut tx = b.node.store.read().await.unwrap();
        let networks = Networks::get_all(&mut tx).await.unwrap();
        drop(tx);
        assert_eq!(networks.len(), 1);
        let cert = networks[0].certificate.as_ref().unwrap();
        assert_eq!(cert.key, b.node.profile_key().public);
        assert_eq!(
            certificate_network_key(cert),
            certificate_network_key(network.certificate.as_ref().unwrap())
        );
        assert_eq!(
            cert.key_usage,
            KeyUsage::Broker as i32 | KeyUsage::Sign as i32
        );
        assert!(membership_trusted(cert, T0));
        assert!(networks[0].alias.is_empty());
        assert!(networks[0].server_config.is_none());

        let e = a
            .frontend
            .publish_network_to_peer(&ctx(), req)
            .await
            .unwrap_err();
        assert!(e.message.contains("already exists"), "{e:?}");
    });
}

#[test]
fn published_certificates_for_another_key_are_refused() {
    run(async {
        let world = World::default();
        let (a, b) = (boot(&world, 1).await, boot(&world, 2).await);
        b.set_publishing(true).await;
        a.start();
        b.start();
        let (to_b, _) = link(&a, &b);
        let mut rng = det_rng(77);
        let network_key = generate_key(&mut rng);
        let stranger = generate_key(&mut rng);
        let usage = KeyUsage::Broker as i32 | KeyUsage::Sign as i32;
        let sign = |key| {
            let csr = new_certificate_request(key, usage, "").unwrap();
            sign_certificate_request_with_network(
                &csr,
                &network_key,
                "net",
                DEFAULT_CERT_TTL,
                T0,
                &mut det_rng(5),
            )
            .unwrap()
        };
        let client = PeerServiceClient::new(to_b);
        let e = client
            .publish(&BootstrapPeerPublishRequest {
                certificate: Some(sign(&stranger)),
            })
            .await
            .unwrap_err();
        assert!(format!("{e:?}").contains("isn't for this profile"), "{e:?}");
        assert!(
            client
                .publish(&BootstrapPeerPublishRequest { certificate: None })
                .await
                .is_err()
        );
        client
            .publish(&BootstrapPeerPublishRequest {
                certificate: Some(sign(b.node.profile_key())),
            })
            .await
            .unwrap();
        let mut tx = b.node.store.read().await.unwrap();
        assert_eq!(Networks::get_all(&mut tx).await.unwrap().len(), 1);
    });
}

#[test]
fn get_config_always_has_a_config() {
    run(async {
        let a = boot(&World::default(), 1).await;
        let config = a
            .frontend
            .get_config(&ctx(), GetConfigRequest {})
            .await
            .unwrap()
            .config;
        assert_eq!(
            config,
            Some(BootstrapConfig {
                enable_publishing: false
            })
        );
        a.set_publishing(true).await;
        let config = a
            .frontend
            .get_config(&ctx(), GetConfigRequest {})
            .await
            .unwrap()
            .config;
        assert_eq!(config.map(|c| c.enable_publishing), Some(true));
    });
}

#[test]
fn clients_are_validated_and_updated_in_place() {
    run(async {
        let a = boot(&World::default(), 1).await;
        a.start();
        let options = |url: &str| BootstrapClientWebSocketOptions {
            url: url.into(),
            insecure_skip_verify_tls: false,
        };
        let c = ctx();
        let update = |id, url: &str| {
            a.frontend.update_client(
                &c,
                UpdateBootstrapClientRequest {
                    id,
                    client_options: Some(
                        update_bootstrap_client_request::ClientOptions::WebsocketOptions(options(
                            url,
                        )),
                    ),
                },
            )
        };
        assert!(
            update(999, "ws://a/x").await.is_err(),
            "missing clients fail"
        );
        assert!(
            a.frontend
                .create_client(
                    &ctx(),
                    CreateBootstrapClientRequest {
                        client_options: Some(
                            create_bootstrap_client_request::ClientOptions::WebsocketOptions(
                                options("http://a/x")
                            )
                        ),
                    },
                )
                .await
                .is_err()
        );
        let first = a.add_client("ws://a/x", false).await;
        let id = a.add_client("ws://a/x", false).await;
        assert_ne!(first, id);
        let list = || a.frontend.list_clients(&c, ListBootstrapClientsRequest {});
        let all = list().await.unwrap().bootstrap_clients;
        assert_eq!(all.len(), 1, "a url's client is replaced");
        assert_eq!(all[0].id, id);
        let updated = update(id, "ws://b/x")
            .await
            .unwrap()
            .bootstrap_client
            .unwrap();
        assert_eq!(updated.id, id);
        let all = list().await.unwrap().bootstrap_clients;
        assert_eq!(all, [updated]);
        sleep(StdDuration::from_millis(100)).await;
        assert!(a.dialer.dials().contains(&"ws://b/x".to_owned()));
    });
}
