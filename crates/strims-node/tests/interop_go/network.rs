// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Network membership between a Go and a Rust node over a direct link:
//! Go's internal/network on the helper (`-network-control`, the `net-*`
//! verbs, a profile store of its own), and `strims_network` through
//! `strims_node::app` on a Rust node with a profile store. Neither binds
//! peers on a fixed port pair.
//!
//! - `go_owner_rust_invitee`: Go creates three networks, Rust one, and
//!   Rust joins one of Go's from an invitation: Go holds more keys and
//!   sends the PSI. Rust's certificate is provisional until it renews
//!   through Go's `CAPeer`; then the shared network binds, and only it.
//!   Messages cross it both ways. Rust then renames, renewing at Go's CA
//!   over vpn.
//! - `rust_owner_go_invitee`: the reverse, with Rust sending the PSI and
//!   Go renewing through Rust's `CAPeer`, then at Rust's CA over vpn.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use prost::Message;
use strims_base::rng::det_rng;
use strims_crypto::{key_from_seed, membership_trusted};
use strims_dao::schema::profile::Profile as ProfileRecord;
use strims_dao::seal::StorageKey;
use strims_dao::{ProfileStore, ReplicatedStore, Singleton};
use strims_kv::MemoryKv;
use strims_network::{Config, NetworkControl, NetworkFrontend};
use strims_node::app::{NodeHost, NodeTimer, PeerServices};
use strims_node::{Clock, HostId, Node, NodeConfig, NodeEvent, SystemClock, TokioNet};
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateNetworkFromInvitationRequest, CreateServerRequest,
    GetNetworkRequest, Invitation, Network, NetworkFrontendService, UpdateAliasRequest,
};
use strims_proto::profile::v1::Profile;
use strims_proto::service::Context;
use strims_vpn::VerifyMode;
use tokio::task::{LocalSet, spawn_local};
use tokio::time::Instant;

use crate::harness::{GoEvent, GoNode, GoOptions, STEP, helper};

const APP_PORT: u16 = 3000;

type Control = NetworkControl<MemoryKv, NodeHost, NodeTimer<SystemClock>>;
type Frontend = NetworkFrontend<MemoryKv, NodeHost, NodeTimer<SystemClock>>;
type Bound = Rc<RefCell<BTreeMap<(HostId, u64), bool>>>;
type Messages = Rc<RefCell<Vec<(HostId, Vec<u8>)>>>;

/// A Rust node running its profile's networks.
struct RustMember {
    node: Node,
    control: Control,
    frontend: Frontend,
    /// Bound peers by network.
    bound: Bound,
    /// (sender, body) of vpn messages to `APP_PORT`.
    messages: Messages,
}

async fn rust_member(seed: u8) -> RustMember {
    let key = key_from_seed(&[seed; 32]);
    let profile = Profile {
        id: u64::from(seed),
        name: format!("rust-{seed}"),
        key: Some(key.clone()),
        device_id: 1,
        ..Default::default()
    };
    let mut config = NodeConfig::new(key, u64::from(seed));
    config.host_key_seed = Some([seed.wrapping_add(0x80); 32]);
    config.label = format!("rust-{seed}");
    config.tcp_listen = Some("127.0.0.1:0".to_owned());
    config.max_peers = 16;
    config.vpn.verify = VerifyMode::Audit;
    let (node, mut events) = Node::spawn(config, TokioNet, SystemClock).await.unwrap();

    let now = SystemClock.now();
    let store = ProfileStore::create(
        MemoryKv::new(),
        profile.id,
        StorageKey::from_bytes([seed; 32]),
        det_rng(u64::from(seed)),
        now,
    )
    .await
    .unwrap();
    let mut tx = store.write(now).await.unwrap();
    ProfileRecord::set(&mut tx, profile.clone()).await.unwrap();
    tx.commit().await.unwrap();
    let store = ReplicatedStore::new(store).await.unwrap();

    let control = NetworkControl::new(
        NodeHost::new(node.clone()),
        store,
        profile,
        NodeTimer(SystemClock),
        det_rng(u64::from(seed) + 1),
        Config::default(),
    );
    spawn_local({
        let control = control.clone();
        async move { control.run().await }
    });
    let services = PeerServices::new(node.clone()).with(Rc::new(control.clone()));
    let bound: Bound = Rc::default();
    let messages: Messages = Rc::default();
    let (b2, m2, c2) = (bound.clone(), messages.clone(), control.clone());
    spawn_local(async move {
        while let Some(e) = events.recv().await {
            let id_of = |net| {
                c2.networks()
                    .iter()
                    .find(|n| n.net == Some(net))
                    .map(|n| n.network.id)
            };
            match &e {
                NodeEvent::NetworkPeerAdded { network, host_id } => {
                    if let Some(id) = id_of(*network) {
                        b2.borrow_mut().insert((*host_id, id), true);
                    }
                }
                NodeEvent::NetworkPeerRemoved { network, host_id } => {
                    if let Some(id) = id_of(*network) {
                        b2.borrow_mut().insert((*host_id, id), false);
                    }
                }
                NodeEvent::Message(d) if d.dst_port == APP_PORT => {
                    m2.borrow_mut().push((d.src, d.body.clone()));
                }
                _ => {}
            }
            services.handle_event(&e).await;
        }
    });
    let frontend = NetworkFrontend::new(control.clone());
    RustMember {
        node,
        control,
        frontend,
        bound,
        messages,
    }
}

impl RustMember {
    async fn create(&self, name: &str) -> Network {
        self.frontend
            .create_server(
                &Context::default(),
                CreateServerRequest {
                    name: name.into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .network
            .unwrap()
    }

    async fn invite(&self, id: u64) -> Invitation {
        self.frontend
            .create_invitation(
                &Context::default(),
                CreateInvitationRequest {
                    network_id: id,
                    bootstrap_client_id: 0,
                },
            )
            .await
            .unwrap()
            .invitation
            .unwrap()
    }

    async fn join(&self, invitation: Vec<u8>) -> Network {
        self.frontend
            .create_network_from_invitation(
                &Context::default(),
                CreateNetworkFromInvitationRequest {
                    alias: String::new(),
                    invitation: Some(InvitationField::InvitationBytes(invitation)),
                },
            )
            .await
            .unwrap()
            .network
            .unwrap()
    }

    async fn get(&self, id: u64) -> Network {
        self.frontend
            .get(&Context::default(), GetNetworkRequest { id })
            .await
            .unwrap()
            .network
            .unwrap()
    }

    fn is_bound(&self, peer: HostId, id: u64) -> bool {
        self.bound
            .borrow()
            .get(&(peer, id))
            .copied()
            .unwrap_or(false)
    }

    /// Waits until the network runs on the node and has a vpn handler on
    /// `APP_PORT`.
    async fn listen(&self, id: u64) -> strims_node::NetworkId {
        let net = until("the network to start", || {
            self.control
                .networks()
                .iter()
                .find(|n| n.network.id == id)
                .and_then(|n| n.net)
        })
        .await;
        self.node.set_vpn_handler(net, APP_PORT).await.unwrap();
        net
    }
}

/// Polls `f` every 50 ms until it has a value.
async fn until<T>(what: &str, mut f: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + STEP * 3;
    loop {
        if let Some(v) = f() {
            return v;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        tokio::time::sleep(StdDuration::from_millis(50)).await;
    }
}

fn go_options(seed: u8) -> GoOptions {
    let mut o = GoOptions::new(seed);
    o.peer_rpc = true;
    o.network_control = true;
    o.max_peers = 16;
    o
}

/// `<id> <key hex>` from a net-create or net-join reply.
fn id_key(reply: &str) -> (u64, String) {
    let mut f = reply.split(' ');
    let id = f.next().unwrap().parse().unwrap();
    (id, f.next().unwrap().to_owned())
}

/// Go's `<trusted> <subject> <renewal error>` for a network.
async fn go_cert(go: &mut GoNode, id: u64) -> (bool, String, i32) {
    let reply = go.cmd(&format!("net-cert {id}")).await.unwrap();
    let f: Vec<&str> = reply.split(' ').collect();
    let subject = if f[1] == "-" {
        String::new()
    } else {
        String::from_utf8(hex::decode(f[1]).unwrap()).unwrap()
    };
    (f[0] == "1", subject, f[2].parse().unwrap())
}

/// Waits for Go to bind `peer` to the network whose key is `key`.
async fn go_bound(go: &mut GoNode, peer: HostId, key: &str) {
    let key = hex::decode(key).unwrap();
    let deadline = Instant::now() + STEP * 3;
    go.wait_until(
        "go to bind the peer",
        deadline,
        |e| matches!(e, GoEvent::NetOpen(h, k) if *h == peer && *k == key),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn go_owner_rust_invitee() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        240,
        LocalSet::new().run_until(async {
            let mut go = GoNode::spawn(&bin, "gn0", go_options(0xc0)).await;
            let mut go_nets = Vec::new();
            for name in ["shared", "go-only", "go-other"] {
                let reply = go
                    .cmd(&format!("net-create {}", hex::encode(name)))
                    .await
                    .unwrap();
                go_nets.push(id_key(&reply));
            }
            let invitation = go
                .cmd(&format!("net-invite {}", go_nets[0].0))
                .await
                .unwrap();

            let r = rust_member(0xc1).await;
            let own = r.create("rust-only").await;
            let joined = r.join(hex::decode(&invitation).unwrap()).await;
            assert!(!membership_trusted(
                joined.certificate.as_ref().unwrap(),
                SystemClock.now()
            ));
            let net = r.listen(joined.id).await;
            assert_eq!(r.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);

            go_bound(&mut go, r.node.host_id(), &go_nets[0].1).await;
            until("rust to bind go", || {
                r.is_bound(go.host_id, joined.id).then_some(())
            })
            .await;
            let renewed = r.get(joined.id).await;
            assert!(membership_trusted(
                renewed.certificate.as_ref().unwrap(),
                SystemClock.now()
            ));
            assert!(!r.is_bound(go.host_id, own.id));
            let m = r.control.metrics();
            assert_eq!(m.runs_failed, 0, "{m:?}");
            exchange(&mut go, &r, &go_nets[0].1, net).await;

            // a trusted member renews at the CA over vpn
            r.frontend
                .update_alias(
                    &Context::default(),
                    UpdateAliasRequest {
                        id: joined.id,
                        alias: "renamed".into(),
                    },
                )
                .await
                .unwrap();
            until("rust's renewal at go's CA", || {
                let n = r.control.networks();
                let n = n.iter().find(|n| n.network.id == joined.id)?;
                let c = n.network.certificate.as_ref()?;
                (c.subject == "renamed").then_some(())
            })
            .await;
            assert_eq!(r.get(joined.id).await.certificate_renewal_error, 0);
            let others = go
                .events()
                .iter()
                .any(|e| matches!(e, GoEvent::NetOpen(_, k) if hex::encode(k) != go_nets[0].1));
            assert!(!others, "only the shared network binds");
        }),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rust_owner_go_invitee() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        240,
        LocalSet::new().run_until(async {
            let r = rust_member(0xd1).await;
            let shared = r.create("shared").await;
            r.create("rust-only").await;
            r.create("rust-other").await;
            let invitation = r.invite(shared.id).await;
            let net = r.listen(shared.id).await;

            let mut go = GoNode::spawn(&bin, "gn1", go_options(0xd0)).await;
            go.cmd(&format!("net-create {}", hex::encode("go-only")))
                .await
                .unwrap();
            let (go_id, key) = id_key(
                &go.cmd(&format!(
                    "net-join {}",
                    hex::encode(invitation.encode_to_vec())
                ))
                .await
                .unwrap(),
            );
            assert!(!go_cert(&mut go, go_id).await.0);
            assert_eq!(r.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);

            go_bound(&mut go, r.node.host_id(), &key).await;
            until("rust to bind go", || {
                r.is_bound(go.host_id, shared.id).then_some(())
            })
            .await;
            let (trusted, _, err) = go_cert(&mut go, go_id).await;
            assert!(trusted && err == 0);
            let m = r.control.metrics();
            assert_eq!(m.runs_failed, 0, "{m:?}");
            exchange(&mut go, &r, &key, net).await;

            // a trusted member renews at the CA over vpn
            go.cmd(&format!("net-alias {go_id} {}", hex::encode("renamed")))
                .await
                .unwrap();
            let deadline = Instant::now() + STEP * 3;
            loop {
                let (_, subject, err) = go_cert(&mut go, go_id).await;
                if subject == "renamed" {
                    assert_eq!(err, 0);
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "go's renewal at rust's CA: error {err}"
                );
                tokio::time::sleep(StdDuration::from_millis(100)).await;
            }
        }),
    )
    .await;
}

/// Messages cross the bound network both ways.
async fn exchange(go: &mut GoNode, r: &RustMember, key: &str, net: strims_node::NetworkId) {
    go.cmd(&format!("net-handler {key} {APP_PORT}"))
        .await
        .unwrap();
    go.cmd(&format!(
        "net-send {key} {} {APP_PORT} {APP_PORT} {}",
        hex::encode(r.node.host_id().to_bytes()),
        hex::encode(b"from go")
    ))
    .await
    .unwrap();
    let go_id = go.host_id;
    until("go's message", || {
        r.messages
            .borrow()
            .iter()
            .any(|(src, body)| *src == go_id && body == b"from go")
            .then_some(())
    })
    .await;
    let rust_id = r.node.host_id();
    let deadline = Instant::now() + STEP;
    loop {
        r.node
            .send(net, go_id, APP_PORT, APP_PORT, b"from rust".to_vec())
            .await
            .unwrap();
        if go
            .take_msgs()
            .iter()
            .any(|m| m.src == rust_id && m.body == b"from rust")
        {
            break;
        }
        assert!(Instant::now() < deadline, "rust's message never arrived");
        tokio::time::sleep(StdDuration::from_millis(200)).await;
    }
}
