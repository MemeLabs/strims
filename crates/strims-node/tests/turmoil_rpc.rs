// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Peer RPC between nodes under turmoil, over the vnic channel pair every
//! peer gets, with `strims-rpc` running on it.
//!
//! - n0 serves a test fixture service on every peer's pair, and calls n1
//!   back over its pair.
//! - n1 dials n0 over TCP and calls both ways: a response and a request
//!   that each span several frames, a stream, a cancel seen by the server,
//!   an error code, and a stream cut by `close_peer`.
//! - n2 dials n0 over WebSocket and never takes its pair, so n0's calls
//!   overflow the channel's input buffer and n2 closes the pair.
//!
//! In a second world, n1 calls n0 through a `RetryClient` for an answer
//! that takes several attempt timeouts to cross n0's capped upload, and
//! n0 serves it once.
//!
//! Every host appends what it sees, stamped with simulated time, to one
//! trace. The same seed must give the same trace hash, and another seed a
//! different one.

#![cfg(feature = "turmoil")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use futures_util::{StreamExt, stream};
use sha2::{Digest, Sha256};
use strims_base::time::Time;
use strims_crypto::key_from_seed;
use strims_node::app::NodeTimer;
use strims_node::{Clock, HostId, Node, NodeConfig, NodeEvent, NodeEvents, SimClock, TurmoilNet};
use strims_proto::rpc::fixture::{
    EchoRequest, EchoResponse, FixtureClient, FixtureService, GetRequest, GetResponse,
    StartRequest, StartResponse, WatchRequest, WatchResponse, register_fixture_service,
};
use strims_proto::service::{CallError, Context as CallContext, Error, ServerStream};
use strims_proto::types::Key;
use strims_rpc::{Client, Config, RetryClient, RetryPolicy, Router, peer_connection};
use tokio::task::spawn_local;

const TCP_PORT: u16 = 7000;
const WS_PORT: u16 = 7001;
const UPLOAD_CAP: u64 = 1024 * 1024;
const STEP: StdDuration = StdDuration::from_secs(30);

fn epoch() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

type Trace = Rc<RefCell<Vec<String>>>;
type Clients = Rc<RefCell<BTreeMap<HostId, Client>>>;

#[derive(Clone)]
struct World {
    seed: u64,
    /// Every node's upload rate limit, in bytes per second.
    upload_cap: u64,
    trace: Trace,
    ids: Rc<RefCell<BTreeMap<u8, HostId>>>,
    /// Echo requests served, by every host.
    echoes: Rc<Cell<u64>>,
}

impl World {
    fn new(seed: u64, upload_cap: u64) -> World {
        World {
            seed,
            upload_cap,
            trace: Rc::default(),
            ids: Rc::default(),
            echoes: Rc::default(),
        }
    }
}

impl World {
    fn profile_key(&self, i: u8) -> Key {
        let mut s = [i; 32];
        s[..8].copy_from_slice(&self.seed.to_le_bytes());
        key_from_seed(&s)
    }

    fn uri(&self, scheme: &str, port: u16, i: u8) -> String {
        let ip = turmoil::lookup(format!("n{i}"));
        format!(
            "{scheme}://{ip}:{port}/{}",
            hex::encode(&self.profile_key(i).public)
        )
    }

    async fn id(&self, i: u8) -> HostId {
        loop {
            if let Some(id) = self.ids.borrow().get(&i) {
                return *id;
            }
            tokio::time::sleep(StdDuration::from_millis(1)).await;
        }
    }
}

/// The test service. Echo answers with the length of a long name, and
/// with as many bytes of data as a numeric name spells; Watch streams
/// IntervalMs items (endlessly for 0, one per millisecond); Get fails with
/// code 7; Start answers with the number of calls cancelled so far.
#[derive(Default)]
struct TestService {
    cancelled: Rc<Cell<u64>>,
    echoes: Rc<Cell<u64>>,
}

struct CancelGuard(Rc<Cell<u64>>);

impl Drop for CancelGuard {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

fn data(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i * 7) as u8).collect()
}

impl FixtureService for TestService {
    async fn echo(&self, _: &CallContext, req: EchoRequest) -> Result<EchoResponse, Error> {
        self.echoes.set(self.echoes.get() + 1);
        let name = if req.name.len() > 64 {
            req.name.len().to_string()
        } else {
            req.name.clone()
        };
        Ok(EchoResponse {
            name,
            data: data(req.name.parse().unwrap_or(0)),
        })
    }

    async fn watch(
        &self,
        _: &CallContext,
        req: WatchRequest,
    ) -> Result<ServerStream<WatchResponse>, Error> {
        let item = |i: i32| WatchResponse {
            data: vec![i as u8],
        };
        if req.interval_ms > 0 {
            let n = req.interval_ms;
            return Ok(Box::pin(stream::iter((0..n).map(move |i| Ok(item(i))))));
        }
        let guard = CancelGuard(self.cancelled.clone());
        Ok(Box::pin(stream::unfold(
            (guard, 0),
            move |(g, i)| async move {
                tokio::time::sleep(StdDuration::from_millis(1)).await;
                Some((Ok(item(i)), (g, i + 1)))
            },
        )))
    }

    async fn get(&self, _: &CallContext, _: GetRequest) -> Result<GetResponse, Error> {
        Err(Error::new(7, "no config"))
    }

    async fn start(&self, _: &CallContext, _: StartRequest) -> Result<StartResponse, Error> {
        Ok(StartResponse {
            id: self.cancelled.get(),
        })
    }
}

struct Host {
    name: String,
    node: Node,
    clock: SimClock,
    trace: Trace,
    clients: Clients,
}

impl Host {
    /// Starts host `i`'s node. With `serve`, it takes every peer's RPC pair
    /// and serves the test service on it.
    async fn start(w: &World, i: u8, serve: bool) -> Host {
        let name = format!("n{i}");
        let ip: IpAddr = turmoil::lookup(name.as_str());
        let mut config = NodeConfig::new(w.profile_key(i), w.seed.wrapping_add(u64::from(i)));
        config.label = name.clone();
        config.tcp_listen = Some(format!("0.0.0.0:{TCP_PORT}"));
        config.ws_listen = Some(format!("0.0.0.0:{WS_PORT}"));
        config.public_ip = Some(ip);
        config.allow_insecure_ws = true;
        config.rate_limit = Some(w.upload_cap);
        let clock = SimClock::turmoil(epoch());
        let (node, events) = Node::spawn(config, TurmoilNet, clock).await.unwrap();
        w.ids.borrow_mut().insert(i, node.host_id());
        let h = Host {
            name,
            node,
            clock,
            trace: w.trace.clone(),
            clients: Rc::default(),
        };
        let service = Rc::new(TestService {
            echoes: w.echoes.clone(),
            ..TestService::default()
        });
        spawn_local(h.handle_events(events, serve, service));
        h
    }

    fn log(&self, line: String) {
        log(&self.trace, &self.clock, &self.name, line);
    }

    fn handle_events(
        &self,
        mut events: NodeEvents,
        serve: bool,
        service: Rc<TestService>,
    ) -> impl Future<Output = ()> + use<> {
        let (name, node, clock, trace, clients) = (
            self.name.clone(),
            self.node.clone(),
            self.clock,
            self.trace.clone(),
            self.clients.clone(),
        );
        async move {
            while let Some(e) = events.recv().await {
                match e {
                    NodeEvent::PeerAdded(p) => {
                        log(&trace, &clock, &name, format!("peer+ {}", p.host_id));
                        if !serve {
                            continue;
                        }
                        let rpc = node.take_peer_rpc(p.host_id).await.unwrap();
                        assert_eq!(
                            node.take_peer_rpc(p.host_id).await.unwrap_err(),
                            strims_node::NodeError::RpcTaken
                        );
                        let mut router = Router::new();
                        register_fixture_service(&mut router, service.clone());
                        let (client, conn) = peer_connection(
                            Config::peer(),
                            (rpc.client.reader, rpc.client.writer),
                            (rpc.server.reader, rpc.server.writer),
                            router,
                        );
                        clients.borrow_mut().insert(p.host_id, client);
                        let (trace, name, peer) = (trace.clone(), name.clone(), p.host_id);
                        spawn_local(async move {
                            let res = conn.await;
                            log(&trace, &clock, &name, format!("rpc {peer} ended {res:?}"));
                        });
                    }
                    NodeEvent::PeerRemoved { host_id } => {
                        log(&trace, &clock, &name, format!("peer- {host_id}"));
                        clients.borrow_mut().remove(&host_id);
                    }
                    _ => {}
                }
            }
        }
    }

    async fn client(&self, peer: HostId) -> FixtureClient<Client> {
        FixtureClient::new(self.rpc_client(peer).await)
    }

    async fn rpc_client(&self, peer: HostId) -> Client {
        tokio::time::timeout(STEP, async {
            loop {
                if let Some(c) = self.clients.borrow().get(&peer) {
                    return c.clone();
                }
                tokio::time::sleep(StdDuration::from_millis(1)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{}: no rpc client for {peer}", self.name))
    }
}

fn log(trace: &Trace, clock: &SimClock, name: &str, line: String) {
    let t = (clock.now() - epoch()).as_millis();
    trace.borrow_mut().push(format!("{t} {name} {line}"));
}

/// Waits until the service behind `fixture` has seen `n` cancels.
async fn wait_cancelled(fixture: &FixtureClient<Client>, n: u64) {
    for _ in 0..1000 {
        let res = fixture.start(&StartRequest::default()).await.unwrap();
        if res.id >= n {
            return;
        }
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    }
    panic!("the server saw fewer than {n} cancels");
}

fn echo(name: impl Into<String>) -> EchoRequest {
    EchoRequest {
        name: name.into(),
        ..Default::default()
    }
}

fn watch(interval_ms: i32) -> WatchRequest {
    WatchRequest {
        interval_ms,
        ..Default::default()
    }
}

async fn server_host(w: World) -> turmoil::Result {
    let h = Host::start(&w, 0, true).await;
    // calls go the other way over the same pairs
    let n1 = w.id(1).await;
    let res = h.client(n1).await.echo(&echo("1000")).await.unwrap();
    assert_eq!(res.data, data(1000));
    h.log("called n1".to_owned());

    // n2 never takes its pair: calls to it fill its input buffer
    let n2 = w.id(2).await;
    let fixture = h.client(n2).await;
    for _ in 0..4 {
        let call = fixture.echo(&echo("x".repeat(400 * 1024)));
        let res = tokio::time::timeout(StdDuration::from_secs(2), call).await;
        h.log(format!("call to n2: {}", res.is_ok()));
    }
    std::future::pending::<()>().await;
    Ok(())
}

async fn script_host(w: World) -> turmoil::Result {
    let h = Host::start(&w, 1, true).await;
    let n0 = h.node.dial(&w.uri("tcp", TCP_PORT, 0)).await.unwrap();
    let fixture = h.client(n0).await;

    // a response of several frames
    let res = fixture.echo(&echo("204800")).await.unwrap();
    assert_eq!(res.data, data(204_800));
    // a request of several frames
    let res = fixture.echo(&echo("y".repeat(300 * 1024))).await.unwrap();
    assert_eq!(res.name, (300 * 1024).to_string());
    h.log("echo ok".to_owned());

    let items: Vec<_> = fixture.watch(&watch(20)).await.unwrap().collect().await;
    assert_eq!(items.len(), 20);
    assert!(items.iter().all(Result::is_ok));
    h.log("watch ok".to_owned());

    let mut endless = fixture.watch(&watch(0)).await.unwrap();
    for _ in 0..3 {
        endless.next().await.unwrap().unwrap();
    }
    drop(endless);
    wait_cancelled(&fixture, 1).await;
    h.log("cancel reached n0".to_owned());

    assert_eq!(
        fixture.get(&GetRequest::default()).await,
        Err(CallError::Remote(Error::new(7, "no config")))
    );

    // closing the peer ends the stream
    let mut endless = fixture.watch(&watch(0)).await.unwrap();
    endless.next().await.unwrap().unwrap();
    assert!(h.node.close_peer(n0).await.unwrap());
    let end = tokio::time::timeout(STEP, async {
        loop {
            match endless.next().await {
                Some(Ok(_)) => {}
                other => return other,
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(end, Some(Err(CallError::Disconnected)));
    h.log("closed peer ended the stream".to_owned());
    Ok(())
}

async fn quiet_host(w: World) -> turmoil::Result {
    let h = Host::start(&w, 2, false).await;
    let n0 = h.node.dial(&w.uri("ws", WS_PORT, 0)).await.unwrap();
    assert_eq!(n0, w.id(0).await);
    let overflowed = tokio::time::timeout(STEP, async {
        loop {
            let m = h.node.metrics().await.unwrap();
            if m.driver.rpc_overflows > 0 {
                return m.driver.rpc_overflows;
            }
            tokio::time::sleep(StdDuration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(overflowed, 1);
    // the pair is gone, the peer isn't
    assert_eq!(
        h.node.take_peer_rpc(n0).await.unwrap_err(),
        strims_node::NodeError::Closed
    );
    assert_eq!(h.node.peers().await.unwrap().len(), 1);
    h.log("overflowed".to_owned());
    Ok(())
}

fn sim(seed: u64) -> turmoil::Sim<'static> {
    turmoil::Builder::new()
        .rng_seed(seed)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_secs(1_700_000_000))
        .simulation_duration(StdDuration::from_secs(120))
        .min_message_latency(StdDuration::from_millis(1))
        .max_message_latency(StdDuration::from_millis(15))
        .build()
}

fn run(seed: u64) -> Vec<String> {
    let w = World::new(seed, UPLOAD_CAP);
    let mut sim = sim(seed);
    let ws = w.clone();
    sim.host("n0", move || server_host(ws.clone()));
    sim.client("n1", script_host(w.clone()));
    sim.client("n2", quiet_host(w.clone()));
    sim.run().unwrap();
    w.trace.borrow().clone()
}

/// The answer that crosses n0's capped upload: about four seconds at
/// [`CAPPED_UPLOAD`].
const SLOW_ANSWER: usize = 256 * 1024;
const CAPPED_UPLOAD: u64 = 64 * 1024;

async fn capped_server(w: World) -> turmoil::Result {
    let _h = Host::start(&w, 0, true).await;
    std::future::pending::<()>().await;
    Ok(())
}

async fn retrying_caller(w: World) -> turmoil::Result {
    let h = Host::start(&w, 1, true).await;
    let n0 = h.node.dial(&w.uri("tcp", TCP_PORT, 0)).await.unwrap();
    let policy = RetryPolicy::peer();
    let fixture = FixtureClient::new(RetryClient::new(
        h.rpc_client(n0).await,
        NodeTimer(h.clock),
        policy,
    ));
    let start = h.clock.now();
    let res = fixture.echo(&echo(SLOW_ANSWER.to_string())).await.unwrap();
    let took = h.clock.now() - start;
    assert_eq!(res.data, data(SLOW_ANSWER));
    assert!(took > policy.attempt_timeout * 3, "answered in {took}");
    assert_eq!(w.echoes.get(), 1);
    h.log(format!("answered in {} ms", took.as_millis()));
    Ok(())
}

fn run_capped(seed: u64) -> Vec<String> {
    let w = World::new(seed, CAPPED_UPLOAD);
    let mut sim = sim(seed);
    let ws = w.clone();
    sim.host("n0", move || capped_server(ws.clone()));
    sim.client("n1", retrying_caller(w.clone()));
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
fn peer_rpc_runs_over_the_channel_pair() {
    let a = run(7);
    for l in &a {
        eprintln!("{l}");
    }
    let b = run(7);
    assert_eq!(digest(&a), digest(&b), "same seed, different traces");
    let c = run(8);
    assert_ne!(digest(&a), digest(&c), "different seeds, same trace");
}

#[test]
fn a_large_answer_on_a_capped_link_is_requested_once() {
    let a = run_capped(7);
    for l in &a {
        eprintln!("{l}");
    }
    assert_eq!(
        digest(&a),
        digest(&run_capped(7)),
        "same seed, different traces"
    );
}
