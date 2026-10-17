// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Network membership on whole nodes, under turmoil. Every node runs a
//! profile store, a network control and its frontend; nobody calls
//! `Node::bind_peer_network` or sets `auto_bind_port`.
//!
//! - o creates a network through its frontend and invites m.
//! - m joins from the invitation and links to o. Its certificate is
//!   provisional, so the two pair without binding; m renews through o's
//!   `CAPeer`, o's CA signs, and the renewed certificate binds them.
//! - m invites i. i joins and links to m only: its renewal goes through
//!   m's `CAPeer`, which forwards it over vpn to o's CA. Then i and m
//!   bind, and a vpn message from i reaches o through m.
//! - i deletes the network, which closes its link with m.
//!
//! Every host appends what it sees, stamped with simulated time, to one
//! trace. The same seed must give the same trace, and another seed a
//! different one.

#![cfg(feature = "turmoil")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use prost::Message;
use sha2::{Digest, Sha256};
use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_crypto::{key_from_seed, membership_trusted};
use strims_dao::schema::profile::Profile as ProfileRecord;
use strims_dao::seal::StorageKey;
use strims_dao::{ProfileStore, ReplicatedStore, Singleton};
use strims_kv::MemoryKv;
use strims_network::{Config, NetworkControl, NetworkFrontend};
use strims_node::app::{NodeHost, NodeTimer, PeerServices};
use strims_node::{Clock, HostId, Node, NodeConfig, NodeEvent, SimClock, TurmoilNet};
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateNetworkFromInvitationRequest, CreateServerRequest,
    DeleteNetworkRequest, Invitation, NetworkFrontendService,
};
use strims_proto::profile::v1::Profile;
use strims_proto::service::Context;
use tokio::task::spawn_local;

const TCP_PORT: u16 = 7000;
const APP_PORT: u16 = 2000;

fn epoch() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

type Trace = Rc<RefCell<Vec<String>>>;
/// vpn messages received: (sender, body).
type Messages = Rc<RefCell<Vec<(String, Vec<u8>)>>>;
type Control = NetworkControl<MemoryKv, NodeHost, NodeTimer<SimClock>>;

#[derive(Clone)]
struct World {
    seed: u64,
    trace: Trace,
    ids: Rc<RefCell<BTreeMap<String, HostId>>>,
    /// Invitations by invitee.
    invitations: Rc<RefCell<BTreeMap<String, Invitation>>>,
    /// Steps done, by name.
    done: Rc<RefCell<BTreeMap<String, bool>>>,
}

impl World {
    fn new(seed: u64) -> World {
        World {
            seed,
            trace: Rc::default(),
            ids: Rc::default(),
            invitations: Rc::default(),
            done: Rc::default(),
        }
    }

    fn log(&self, clock: &SimClock, name: &str, line: impl Into<String>) {
        let t = (clock.now() - epoch()).as_millis();
        self.trace
            .borrow_mut()
            .push(format!("{t} {name} {}", line.into()));
    }

    fn profile(&self, name: &str) -> Profile {
        let mut s = [name.as_bytes()[0]; 32];
        s[..8].copy_from_slice(&self.seed.to_le_bytes());
        Profile {
            id: u64::from(name.as_bytes()[0]),
            name: name.to_owned(),
            key: Some(key_from_seed(&s)),
            device_id: 1,
            ..Default::default()
        }
    }

    fn uri(&self, name: &str) -> String {
        let ip = turmoil::lookup(name);
        let key = self.profile(name).key.unwrap();
        format!("tcp://{ip}:{TCP_PORT}/{}", hex::encode(&key.public))
    }

    fn name(&self, id: &HostId) -> String {
        self.ids
            .borrow()
            .iter()
            .find(|(_, i)| *i == id)
            .map(|(n, _)| n.clone())
            .unwrap_or_default()
    }

    async fn wait<T>(&self, what: &str, mut f: impl FnMut(&World) -> Option<T>) -> T {
        let wait = async {
            loop {
                if let Some(v) = f(self) {
                    return v;
                }
                tokio::time::sleep(StdDuration::from_millis(100)).await;
            }
        };
        tokio::time::timeout(StdDuration::from_secs(240), wait)
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
    }

    fn set(&self, step: &str) {
        self.done.borrow_mut().insert(step.to_owned(), true);
    }

    fn is(&self, step: &str) -> bool {
        self.done.borrow().get(step).copied().unwrap_or(false)
    }
}

struct Started {
    node: Node,
    clock: SimClock,
    control: Control,
    frontend: NetworkFrontend<MemoryKv, NodeHost, NodeTimer<SimClock>>,
    /// Peers bound to the network, by name.
    bound: Rc<RefCell<BTreeMap<String, bool>>>,
    messages: Messages,
}

async fn start(w: &World, name: &str, index: u64) -> Started {
    let ip: IpAddr = turmoil::lookup(name);
    let profile = w.profile(name);
    let mut config = NodeConfig::new(profile.key.clone().unwrap(), w.seed.wrapping_add(index));
    config.label = name.to_owned();
    config.tcp_listen = Some(format!("0.0.0.0:{TCP_PORT}"));
    config.public_ip = Some(ip);
    config.max_peers = 16;
    let clock = SimClock::turmoil(epoch());
    let (node, mut events) = Node::spawn(config, TurmoilNet, clock).await.unwrap();
    w.ids.borrow_mut().insert(name.to_owned(), node.host_id());

    let store = ProfileStore::create(
        MemoryKv::new(),
        profile.id,
        StorageKey::from_bytes([index as u8; 32]),
        det_rng(w.seed ^ index),
        clock.now(),
    )
    .await
    .unwrap();
    let mut tx = store.write(clock.now()).await.unwrap();
    ProfileRecord::set(&mut tx, profile.clone()).await.unwrap();
    tx.commit().await.unwrap();
    let store = ReplicatedStore::new(store).await.unwrap();

    let control = NetworkControl::new(
        NodeHost::new(node.clone()),
        store,
        profile,
        NodeTimer(clock),
        det_rng(w.seed.wrapping_mul(31) ^ index),
        Config::default(),
    );
    spawn_local({
        let control = control.clone();
        async move { control.run().await }
    });
    let services = PeerServices::new(node.clone()).with(Rc::new(control.clone()));
    let bound: Rc<RefCell<BTreeMap<String, bool>>> = Rc::default();
    let messages: Messages = Rc::default();
    let (w2, me, b2, m2) = (w.clone(), name.to_owned(), bound.clone(), messages.clone());
    spawn_local(async move {
        while let Some(e) = events.recv().await {
            match &e {
                NodeEvent::PeerAdded(p) => {
                    w2.log(&clock, &me, format!("peer+ {}", w2.name(&p.host_id)))
                }
                NodeEvent::NetworkPeerAdded { host_id, .. } => {
                    let peer = w2.name(host_id);
                    w2.log(&clock, &me, format!("net+ {peer}"));
                    b2.borrow_mut().insert(peer, true);
                }
                NodeEvent::NetworkPeerRemoved { host_id, .. } => {
                    let peer = w2.name(host_id);
                    w2.log(&clock, &me, format!("net- {peer}"));
                    b2.borrow_mut().insert(peer, false);
                }
                NodeEvent::Message(d) if d.dst_port == APP_PORT => {
                    let from = w2.name(&d.src);
                    w2.log(&clock, &me, format!("message from {from} hops={}", d.hops));
                    m2.borrow_mut().push((from, d.body.clone()));
                }
                _ => {}
            }
            services.handle_event(&e).await;
        }
    });
    let frontend = NetworkFrontend::new(control.clone());
    Started {
        node,
        clock,
        control,
        frontend,
        bound,
        messages,
    }
}

impl Started {
    fn is_bound(&self, peer: &str) -> bool {
        self.bound.borrow().get(peer).copied().unwrap_or(false)
    }

    /// Whether this side's certificate on its one network is trusted.
    fn trusted(&self) -> bool {
        let now = self.clock.now();
        self.control.networks().first().is_some_and(|n| {
            n.network
                .certificate
                .as_ref()
                .is_some_and(|c| membership_trusted(c, now))
        })
    }

    async fn invite(&self, w: &World, invitee: &str) {
        let id = self.control.networks()[0].network.id;
        let invitation = self
            .frontend
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
            .unwrap();
        w.invitations
            .borrow_mut()
            .insert(invitee.to_owned(), invitation);
    }

    async fn join(&self, w: &World, me: &str) {
        let invitation = w
            .wait("an invitation", |w| w.invitations.borrow().get(me).cloned())
            .await;
        self.frontend
            .create_network_from_invitation(
                &Context::default(),
                CreateNetworkFromInvitationRequest {
                    alias: String::new(),
                    invitation: Some(InvitationField::InvitationBytes(invitation.encode_to_vec())),
                },
            )
            .await
            .unwrap();
        w.log(&self.clock, me, "joined");
    }
}

async fn owner(w: World) -> turmoil::Result {
    let h = start(&w, "o", 1).await;
    h.frontend
        .create_server(
            &Context::default(),
            CreateServerRequest {
                name: "net".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    w.wait("o's network to start", |_| {
        h.control.networks().first().and_then(|n| n.net)
    })
    .await;
    let net = h.control.networks()[0].net.unwrap();
    h.node.set_vpn_handler(net, APP_PORT).await.unwrap();
    w.log(&h.clock, "o", "created");
    h.invite(&w, "m").await;

    let msgs = h.messages.clone();
    w.wait("i's message", |_| {
        msgs.borrow()
            .iter()
            .any(|(from, _)| from == "i")
            .then_some(())
    })
    .await;
    w.set("o got i's message");
    let m = h.control.metrics();
    w.log(
        &h.clock,
        "o",
        format!("metrics runs={} completed={}", m.runs, m.runs_completed),
    );
    w.wait("the end", |w| w.is("i left").then_some(())).await;
    Ok(())
}

async fn member(w: World) -> turmoil::Result {
    let h = start(&w, "m", 2).await;
    h.join(&w, "m").await;
    h.node.dial(&w.uri("o")).await.unwrap();
    w.wait("m to bind with o", |_| {
        (h.is_bound("o") && h.trusted()).then_some(())
    })
    .await;
    w.log(&h.clock, "m", "renewed and bound with o");
    h.invite(&w, "i").await;
    w.wait("m to bind with i", |_| h.is_bound("i").then_some(()))
        .await;
    w.wait("i to leave", |_| (!h.is_bound("i")).then_some(()))
        .await;
    w.log(&h.clock, "m", "i left");
    w.set("i left");
    Ok(())
}

async fn invitee(w: World) -> turmoil::Result {
    let h = start(&w, "i", 3).await;
    h.join(&w, "i").await;
    h.node.dial(&w.uri("m")).await.unwrap();
    w.wait("i to bind with m", |_| {
        (h.is_bound("m") && h.trusted()).then_some(())
    })
    .await;
    w.log(&h.clock, "i", "renewed through m and bound");
    let peers = h.node.peers().await.unwrap();
    assert_eq!(peers.len(), 1, "i links to m only");

    let net = h.control.networks()[0].net.unwrap();
    let o = w.ids.borrow()["o"];
    w.wait("o to get i's message", |w| {
        if w.is("o got i's message") {
            return Some(());
        }
        let node = h.node.clone();
        spawn_local(async move {
            let _ = node
                .send(net, o, APP_PORT, APP_PORT, b"hello".to_vec())
                .await;
        });
        None
    })
    .await;

    let id = h.control.networks()[0].network.id;
    h.frontend
        .delete(&Context::default(), DeleteNetworkRequest { id })
        .await
        .unwrap();
    w.log(&h.clock, "i", "deleted the network");
    w.wait("the end", |w| w.is("i left").then_some(())).await;
    Ok(())
}

fn run(seed: u64) -> Vec<String> {
    let w = World::new(seed);
    let mut sim = turmoil::Builder::new()
        .rng_seed(seed)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_secs(1_700_000_000))
        .simulation_duration(StdDuration::from_secs(600))
        .min_message_latency(StdDuration::from_millis(1))
        .max_message_latency(StdDuration::from_millis(15))
        .build();
    sim.client("o", owner(w.clone()));
    sim.client("m", member(w.clone()));
    sim.client("i", invitee(w.clone()));
    sim.run().unwrap();
    w.trace.borrow().clone()
}

fn digest(trace: &[String]) -> String {
    let mut h = Sha256::new();
    for l in trace {
        h.update(l.as_bytes());
        h.update(b"\n");
    }
    hex::encode(h.finalize())
}

#[test]
fn members_join_renew_and_bind_through_each_other() {
    let a = run(7);
    for l in &a {
        eprintln!("{l}");
    }
    assert_eq!(digest(&a), digest(&run(7)), "same seed, different traces");
    assert_ne!(digest(&a), digest(&run(8)), "different seeds, same trace");
}
