// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Replication between two devices of one profile on whole nodes, under
//! turmoil.
//!
//! - b is a bootstrap node of another profile; every node joins the same
//!   network through it.
//! - d0 signs up, records the network and makes a pairing token.
//! - d1 imports the token as a new device of the profile. Neither dials
//!   the other: d1 finds d0's host id in the network's hash table and asks
//!   its node to link through peer exchange.
//! - Both write, then both edit one record at the same moment; the stores
//!   converge. d0 then deletes d1's device, and d1's replication stops.
//!
//! Every host appends what it sees, stamped with simulated time, to one
//! trace, which ends with both stores' replicated records. The same seed
//! must give the same trace, and another seed a different one.

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

use futures_util::FutureExt;
use sha2::{Digest, Sha256};
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::key_from_seed;
use strims_dao::auth::{self, ProfileKey};
use strims_dao::replication::dump;
use strims_dao::schema::chat::ChatUiConfigHighlights;
use strims_dao::schema::network::Networks;
use strims_dao::schema::profile::{Devices, Profile};
use strims_dao::{ReplicatedStore, Singleton, Table};
use strims_kv::MemoryKv;
use strims_node::app::{NodeHost, NodeTimer, PeerServices};
use strims_node::{
    Clock, HostId, NetworkId, NetworkOptions, Node, NodeConfig, NodeEvent, SimClock, TurmoilNet,
};
use strims_proto::auth::v1::PairingToken;
use strims_proto::chat::v1::UiConfigHighlight;
use strims_proto::network::v1::Network;
use strims_proto::replication::v1::Event;
use strims_proto::types::{Certificate, Key};
use strims_replication::{Config, ReplicationControl, ReplicationFrontend};
use tokio::task::spawn_local;

const TCP_PORT: u16 = 7000;
const VPN_PORT: u16 = 60;

fn epoch() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

fn network_key() -> Key {
    key_from_seed(&[0xee; 32])
}

type Trace = Rc<RefCell<Vec<String>>>;
type Control = ReplicationControl<MemoryKv, NodeHost, NodeTimer<SimClock>>;

#[derive(Clone)]
struct World {
    seed: u64,
    trace: Trace,
    token: Rc<RefCell<Option<(PairingToken, ProfileKey)>>>,
    stores: Rc<RefCell<BTreeMap<String, ReplicatedStore<MemoryKv>>>>,
    /// Ids of shared records, by name.
    shared: Rc<RefCell<BTreeMap<String, u64>>>,
    ids: Rc<RefCell<BTreeMap<String, HostId>>>,
}

impl World {
    fn new(seed: u64) -> World {
        World {
            seed,
            trace: Rc::default(),
            token: Rc::default(),
            stores: Rc::default(),
            shared: Rc::default(),
            ids: Rc::default(),
        }
    }

    fn log(&self, clock: &SimClock, name: &str, line: impl Into<String>) {
        let t = (clock.now() - epoch()).as_millis();
        self.trace
            .borrow_mut()
            .push(format!("{t} {name} {}", line.into()));
    }

    fn uri(&self, i: &str) -> String {
        let ip = turmoil::lookup(i);
        let key = if i == "b" {
            key_from_seed(&[0xbb; 32])
        } else {
            self.profile_key()
        };
        format!("tcp://{ip}:{TCP_PORT}/{}", hex::encode(&key.public))
    }

    fn profile_key(&self) -> Key {
        let token = self.token.borrow();
        let (t, _) = token.as_ref().unwrap();
        t.profile.as_ref().unwrap().key.clone().unwrap()
    }
}

/// A running node on the shared network.
struct Started {
    node: Node,
    net: NetworkId,
    events: strims_node::NodeEvents,
    clock: SimClock,
}

/// A node with `key` on the shared network, linked to b.
async fn node(w: &World, name: &str, key: Key, dial_b: bool) -> Started {
    let ip: IpAddr = turmoil::lookup(name);
    let index = match name {
        "b" => 0,
        "d0" => 1,
        _ => 2,
    };
    let mut config = NodeConfig::new(key, w.seed.wrapping_add(index));
    config.label = name.to_owned();
    config.tcp_listen = Some(format!("0.0.0.0:{TCP_PORT}"));
    config.public_ip = Some(ip);
    config.max_peers = 16;
    let clock = SimClock::turmoil(epoch());
    let (node, mut events) = Node::spawn(config, TurmoilNet, clock).await.unwrap();
    w.ids.borrow_mut().insert(name.to_owned(), node.host_id());
    let net = node
        .add_network(
            &network_key().public,
            NetworkOptions {
                auto_bind_port: Some(VPN_PORT),
            },
        )
        .await
        .unwrap();
    if dial_b {
        let b = node.dial(&w.uri("b")).await.unwrap();
        loop {
            if let Some(NodeEvent::PeerAdded(p)) = events.recv().await
                && p.host_id == b
            {
                break;
            }
        }
    }
    Started {
        node,
        net,
        events,
        clock,
    }
}

/// Runs replication for a device's store on its node.
fn replicate(
    w: &World,
    name: &str,
    started: Started,
    store: &ReplicatedStore<MemoryKv>,
    profile: strims_proto::profile::v1::Profile,
) -> (Control, tokio::task::JoinHandle<()>) {
    let Started {
        node,
        net,
        mut events,
        clock,
    } = started;
    let host = NodeHost::new(node.clone());
    host.add_network(net);
    let config = Config {
        scan_interval: Duration::from_secs(2),
        ..Config::default()
    };
    let control = ReplicationControl::new(
        store.clone(),
        profile,
        host,
        NodeTimer(clock),
        det_rng(w.seed ^ u64::from(name.as_bytes()[1])),
        config,
    );
    let run = spawn_local({
        let control = control.clone();
        let (w, name) = (w.clone(), name.to_owned());
        async move {
            control.run().await;
            w.log(&clock, &name, "replication stopped");
        }
    });
    let services = PeerServices::new(node.clone()).with(Rc::new(control.clone()));
    let (w, name) = (w.clone(), name.to_owned());
    spawn_local(async move {
        while let Some(e) = events.recv().await {
            if let NodeEvent::PeerAdded(p) = &e {
                let who = w
                    .ids
                    .borrow()
                    .iter()
                    .find(|(_, id)| **id == p.host_id)
                    .map(|(n, _)| n.clone());
                w.log(&clock, &name, format!("peer+ {}", who.unwrap_or_default()));
            }
            services.handle_event(&e).await;
        }
    });
    (control, run)
}

async fn records(store: &ReplicatedStore<MemoryKv>) -> Vec<Event> {
    let mut events = dump(store.store()).await.unwrap().events;
    events.sort_by_key(|e| (e.namespace, e.id));
    events
}

async fn highlight(store: &ReplicatedStore<MemoryKv>, clock: &SimClock, alias: &str) -> u64 {
    let mut tx = store.write(clock.now()).await.unwrap();
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

async fn has_alias(store: &ReplicatedStore<MemoryKv>, alias: &str) -> bool {
    let mut tx = store.read().await.unwrap();
    let all = ChatUiConfigHighlights::get_all(&mut tx).await.unwrap();
    all.iter().any(|h| h.alias == alias)
}

/// Waits until `cond` holds, checking every 100 ms of simulated time.
async fn until<F: AsyncFnMut() -> bool>(what: &str, limit: StdDuration, mut cond: F) {
    let wait = async {
        while !cond().await {
            tokio::time::sleep(StdDuration::from_millis(100)).await;
        }
    };
    tokio::time::timeout(limit, wait)
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
}

async fn bootstrap(w: World) -> turmoil::Result {
    let mut b = node(&w, "b", key_from_seed(&[0xbb; 32]), false).await;
    while b.events.recv().await.is_some() {}
    Ok(())
}

async fn first_device(w: World) -> turmoil::Result {
    let clock = SimClock::turmoil(epoch());
    let kv = MemoryKv::new();
    let mut rng = det_rng(w.seed);
    let (pk, store) = auth::sign_up(&kv, "alice", "pw", "d0", &mut rng, clock.now())
        .await
        .unwrap();
    let store = ReplicatedStore::new(store).await.unwrap();
    let mut tx = store.write(clock.now()).await.unwrap();
    let id = tx.generate_id().await.unwrap();
    Networks::insert(
        &mut tx,
        Network {
            id,
            certificate: Some(Certificate {
                key: network_key().public,
                not_after: u64::MAX,
                ..Default::default()
            }),
            alias: "net".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let mut tx = store.read().await.unwrap();
    let profile = Profile::get(&mut tx).await.unwrap().unwrap();
    drop(tx);

    let token = ReplicationFrontend::new(store.clone(), NodeTimer(clock))
        .pairing_token(0, 0)
        .await
        .unwrap();
    w.token.replace(Some((token, pk)));
    w.stores.borrow_mut().insert("d0".into(), store.clone());

    let started = node(&w, "d0", profile.key.clone().unwrap(), true).await;
    let clock = started.clock;
    let (_control, _run) = replicate(&w, "d0", started, &store, profile);
    let shared = highlight(&store, &clock, "shared").await;
    w.shared.borrow_mut().insert("shared".into(), shared);
    highlight(&store, &clock, "from d0").await;

    until("d1's write", StdDuration::from_secs(120), async || {
        has_alias(&store, "from d1").await
    })
    .await;
    w.log(&clock, "d0", "has d1's write");

    // Both edit the shared record, each as soon as it has the other's write.
    let mut tx = store.write(clock.now()).await.unwrap();
    ChatUiConfigHighlights::transform(&mut tx, shared, |h| {
        h.alias = "edited by d0".into();
        Ok(())
    })
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let d1 = w.stores.borrow().get("d1").cloned().unwrap();
    until("convergence", StdDuration::from_secs(60), async || {
        records(&store).await == records(&d1).await
    })
    .await;
    w.log(&clock, "d0", "converged");

    // d1 leaves the profile.
    let d1_device = d1.replica_id();
    let mut tx = store.write(clock.now()).await.unwrap();
    assert!(Devices::delete(&mut tx, d1_device).await.unwrap());
    tx.commit().await.unwrap();
    w.log(&clock, "d0", "deleted d1");
    tokio::time::sleep(StdDuration::from_secs(30)).await;
    Ok(())
}

async fn second_device(w: World) -> turmoil::Result {
    let clock = SimClock::turmoil(epoch());
    until(
        "the pairing token",
        StdDuration::from_secs(10),
        async || w.token.borrow().is_some(),
    )
    .await;
    let (token, pk) = w.token.borrow().clone().unwrap();
    let kv = MemoryKv::new();
    let mut rng = det_rng(w.seed.wrapping_add(1));
    let store = auth::import_pairing_token(&kv, &token, &pk, &mut rng, clock.now())
        .await
        .unwrap();
    let store = ReplicatedStore::new(store).await.unwrap();
    let device = store.replica_id();
    let mut tx = store.write(clock.now()).await.unwrap();
    Devices::transform(&mut tx, device, |d| {
        d.id = device;
        d.os = "d1".into();
        Ok(())
    })
    .await
    .unwrap();
    let profile = Profile::get(&mut tx).await.unwrap().unwrap();
    tx.commit().await.unwrap();
    w.stores.borrow_mut().insert("d1".into(), store.clone());

    let started = node(&w, "d1", profile.key.clone().unwrap(), true).await;
    let clock = started.clock;
    let (control, run) = replicate(&w, "d1", started, &store, profile);
    highlight(&store, &clock, "from d1").await;

    until("d0's write", StdDuration::from_secs(120), async || {
        has_alias(&store, "from d0").await
    })
    .await;
    w.log(&clock, "d1", "has d0's write");
    let shared = *w.shared.borrow().get("shared").unwrap();
    let mut tx = store.write(clock.now()).await.unwrap();
    ChatUiConfigHighlights::transform(&mut tx, shared, |h| {
        h.alias = "edited by d1".into();
        Ok(())
    })
    .await
    .unwrap();
    tx.commit().await.unwrap();

    tokio::time::timeout(StdDuration::from_secs(120), run)
        .await
        .expect("d1's replication stops once d1 is deleted")
        .unwrap();
    let m = control.metrics();
    w.log(
        &clock,
        "d1",
        format!(
            "metrics opens={} syncs={} bootstraps={}",
            m.opens, m.syncs_sent, m.bootstraps_sent
        ),
    );
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
    let wb = w.clone();
    sim.host("b", move || bootstrap(wb.clone()));
    sim.client("d0", first_device(w.clone()));
    sim.client("d1", second_device(w.clone()));
    sim.run().unwrap();

    let mut trace = w.trace.borrow().clone();
    let stores = w.stores.borrow();
    for (name, store) in stores.iter() {
        let records = records(store).now_or_never().unwrap();
        trace.push(format!(
            "{name} records {}",
            hex::encode(Sha256::digest(format!("{records:?}")))
        ));
        assert_eq!(store.store().kv().open_transactions(), 0);
    }
    let r0 = records(stores.get("d0").unwrap()).now_or_never().unwrap();
    let r1 = records(stores.get("d1").unwrap()).now_or_never().unwrap();
    assert_eq!(r0, r1, "the stores converged");
    let highlights = r0
        .iter()
        .filter(|e| e.namespace == ChatUiConfigHighlights::NS.0)
        .count();
    assert_eq!(highlights, 3, "both writes and the shared record");
    trace
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
fn paired_devices_replicate_through_the_network() {
    let a = run(7);
    for l in &a {
        eprintln!("{l}");
    }
    assert_eq!(digest(&a), digest(&run(7)), "same seed, different traces");
    assert_ne!(digest(&a), digest(&run(8)), "different seeds, same trace");
}
