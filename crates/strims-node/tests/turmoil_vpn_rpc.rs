// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! RPC over vpn between nodes under turmoil, with `strims_rpc::vpn` on
//! ports from `Node::open_vpn_port`.
//!
//! - n0 serves a test Debug service on an ephemeral port and publishes the
//!   address in the hash table under the network key.
//! - n1 dials n0; n2 dials n1, so n2's calls to n0 are relayed.
//! - n1 and n2 resolve the address and call: a unary call whose answer
//!   names the caller's certificate as the server saw it, a stream, and a
//!   stream dropped early whose cancel reaches the server.
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
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use futures_util::{StreamExt, stream};
use sha2::{Digest, Sha256};
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::{
    CertificateChain, DEFAULT_CERT_TTL, Issuer, host_certificate, key_from_seed,
    new_certificate_request, new_self_signed_certificate, sign_certificate_request,
};
use strims_node::{
    Clock, HashTableGetOptions, HostAddr, HostId, NetworkId, NetworkOptions, Node, NodeConfig,
    SimClock, TurmoilNet,
};
use strims_proto::debug::v1::{
    DebugClient, DebugService, PProfRequest, PProfResponse, StartMockStreamRequest,
    StartMockStreamResponse, WatchMetricsRequest, WatchMetricsResponse, register_debug_service,
};
use strims_proto::service::{Context as CallContext, Error, ServerStream};
use strims_proto::types::{Certificate, Key, KeyUsage};
use strims_rpc::vpn::{self, VpnConfig};
use strims_rpc::{Client, Router};
use tokio::task::spawn_local;

const TCP_PORT: u16 = 7000;
const VPN_PORT: u16 = 10_000;
const SALT: &[u8] = b"debug";
const STEP: StdDuration = StdDuration::from_secs(30);

fn epoch() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

type Trace = Rc<RefCell<Vec<String>>>;

#[derive(Clone)]
struct World {
    seed: u64,
    trace: Trace,
    /// n1's host id, once it is up.
    relay: Rc<Cell<Option<HostId>>>,
    /// Set when n1's calls are done.
    relay_done: Rc<Cell<bool>>,
}

impl World {
    fn profile_key(&self, i: u8) -> Key {
        let mut s = [i; 32];
        s[..8].copy_from_slice(&self.seed.to_le_bytes());
        key_from_seed(&s)
    }

    fn network_key(&self) -> Key {
        key_from_seed(&[0xee; 32])
    }

    fn uri(&self, i: u8) -> String {
        let ip = turmoil::lookup(format!("n{i}"));
        format!(
            "tcp://{ip}:{TCP_PORT}/{}",
            hex::encode(&self.profile_key(i).public)
        )
    }

    /// Host `i`'s membership certificate, issued by the network's key.
    fn membership(&self, i: u8) -> Certificate {
        let rng = &mut det_rng(self.seed.wrapping_add(u64::from(i)));
        let network = self.network_key();
        let root = new_self_signed_certificate(
            &network,
            KeyUsage::Sign as i32,
            DEFAULT_CERT_TTL,
            "network",
            epoch(),
            rng,
        )
        .unwrap();
        let usage = KeyUsage::Peer as i32 | KeyUsage::Sign as i32;
        let csr = new_certificate_request(&self.profile_key(i), usage, &format!("n{i}")).unwrap();
        let issuer = Issuer::Certificate {
            key: &network,
            certificate: &root,
        };
        sign_certificate_request(&csr, DEFAULT_CERT_TTL, issuer, epoch(), rng).unwrap()
    }
}

/// PProf answers with the caller's host and profile keys, as its
/// certificate names them; WatchMetrics streams IntervalMs items, or
/// endlessly for 0; StartMockStream answers with the streams cancelled so
/// far.
#[derive(Default)]
struct TestService {
    cancelled: Rc<Cell<u64>>,
}

struct CancelGuard(Rc<Cell<u64>>);

impl Drop for CancelGuard {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

impl DebugService for TestService {
    async fn p_prof(&self, ctx: &CallContext, _: PProfRequest) -> Result<PProfResponse, Error> {
        let cert = ctx.certificate().ok_or(Error::new(1, "no certificate"))?;
        let cert = CertificateChain::decode(cert).map_err(|_| Error::new(3, "malformed"))?;
        let profile = cert.parent().ok_or(Error::new(2, "no membership"))?;
        Ok(PProfResponse {
            name: hex::encode(&cert.key),
            data: profile.key.clone(),
        })
    }

    async fn watch_metrics(
        &self,
        _: &CallContext,
        req: WatchMetricsRequest,
    ) -> Result<ServerStream<WatchMetricsResponse>, Error> {
        let item = |i: i32| WatchMetricsResponse {
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
                tokio::time::sleep(StdDuration::from_millis(5)).await;
                Some((Ok(item(i)), (g, i + 1)))
            },
        )))
    }

    async fn start_mock_stream(
        &self,
        _: &CallContext,
        _: StartMockStreamRequest,
    ) -> Result<StartMockStreamResponse, Error> {
        Ok(StartMockStreamResponse {
            id: self.cancelled.get(),
        })
    }
}

struct Host {
    name: String,
    node: Node,
    net: NetworkId,
    clock: SimClock,
    trace: Trace,
    certificate: Certificate,
}

impl Host {
    async fn start(w: &World, i: u8) -> Host {
        let name = format!("n{i}");
        let ip: IpAddr = turmoil::lookup(name.as_str());
        let mut config = NodeConfig::new(w.profile_key(i), w.seed.wrapping_add(u64::from(i)));
        config.label = name.clone();
        config.tcp_listen = Some(format!("0.0.0.0:{TCP_PORT}"));
        config.public_ip = Some(ip);
        config.max_peers = 16;
        let clock = SimClock::turmoil(epoch());
        let (node, _events) = Node::spawn(config, TurmoilNet, clock).await.unwrap();
        let membership = w.membership(i);
        let net = node
            .add_network_certificate(
                &membership,
                NetworkOptions {
                    auto_bind_port: Some(VPN_PORT),
                },
            )
            .await
            .unwrap();
        let host_key = node.host_id().to_bytes();
        let certificate = host_certificate(&host_key, &w.profile_key(i), &membership).unwrap();
        Host {
            name,
            node,
            net,
            clock,
            trace: w.trace.clone(),
            certificate,
        }
    }

    fn log(&self, line: String) {
        let t = (self.clock.now() - epoch()).as_millis();
        self.trace
            .borrow_mut()
            .push(format!("{t} {} {line}", self.name));
    }

    /// Runs a vpn connection on a new port, serving `router` if given.
    async fn connect(&self, router: Option<Router>) -> (vpn::VpnClient, u16) {
        let port = self.node.open_vpn_port(self.net, None).await.unwrap();
        let (client, conn) = vpn::connect(
            VpnConfig::default(),
            &self.certificate,
            port.reader,
            port.writer,
            router,
        )
        .unwrap();
        spawn_local(conn);
        (client, port.port)
    }

    /// The service's address, once the hash table has it.
    async fn resolve(&self, w: &World) -> HostAddr {
        tokio::time::timeout(STEP, async {
            loop {
                let options = HashTableGetOptions {
                    disable_cache: true,
                    timeout: Duration::from_secs(1),
                };
                let addr = self
                    .node
                    .resolve_vpn_address(self.net, w.network_key().public, SALT.to_vec(), options)
                    .await
                    .unwrap();
                if let Some(addr) = addr {
                    return addr;
                }
            }
        })
        .await
        .expect("resolving the service")
    }
}

async fn server_host(w: World) -> turmoil::Result {
    let h = Host::start(&w, 0).await;
    let service = Rc::new(TestService::default());
    let mut router = Router::new();
    register_debug_service(&mut router, service);
    let (_, port) = h.connect(Some(router)).await;
    h.node
        .publish_vpn_address(h.net, w.network_key(), SALT.to_vec(), port)
        .await
        .unwrap();
    h.log("serving".to_owned());
    std::future::pending::<()>().await;
    Ok(())
}

/// Calls the service and checks what it saw.
async fn call_service(h: &Host, w: &World, i: u8, server: HostId) {
    let addr = h.resolve(w).await;
    assert_eq!(addr.host, server);
    let (vpn_client, _) = h.connect(None).await;
    let debug = DebugClient::new(vpn_client.to(addr));

    let res = debug.p_prof(&PProfRequest::default()).await.unwrap();
    assert_eq!(res.name, hex::encode(h.node.host_id().to_bytes()));
    assert_eq!(res.data, w.profile_key(i).public);
    h.log("the server saw our certificate".to_owned());

    let items: Vec<_> = debug
        .watch_metrics(&watch(10))
        .await
        .unwrap()
        .collect()
        .await;
    assert_eq!(items.len(), 10);
    assert!(items.iter().all(Result::is_ok));
    h.log("watch ok".to_owned());

    let before = cancelled(&debug).await;
    let mut endless = debug.watch_metrics(&watch(0)).await.unwrap();
    for _ in 0..3 {
        endless.next().await.unwrap().unwrap();
    }
    drop(endless);
    tokio::time::timeout(STEP, async {
        while cancelled(&debug).await == before {
            tokio::time::sleep(StdDuration::from_millis(10)).await;
        }
    })
    .await
    .expect("the cancel reaching the server");
    h.log("cancel reached the server".to_owned());
}

async fn cancelled(debug: &DebugClient<Client>) -> u64 {
    debug
        .start_mock_stream(&StartMockStreamRequest::default())
        .await
        .unwrap()
        .id
}

fn watch(interval_ms: i32) -> WatchMetricsRequest {
    WatchMetricsRequest {
        interval_ms,
        ..Default::default()
    }
}

async fn relay_host(w: World) -> turmoil::Result {
    let h = Host::start(&w, 1).await;
    w.relay.set(Some(h.node.host_id()));
    let n0 = h.node.dial(&w.uri(0)).await.unwrap();
    call_service(&h, &w, 1, n0).await;
    w.relay_done.set(true);
    std::future::pending::<()>().await;
    Ok(())
}

async fn client_host(w: World) -> turmoil::Result {
    let h = Host::start(&w, 2).await;
    // join through n1 once it is up
    let n1 = loop {
        if let Some(id) = w.relay.get() {
            break id;
        }
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    };
    tokio::time::sleep(StdDuration::from_millis(200)).await;
    assert_eq!(h.node.dial(&w.uri(1)).await.unwrap(), n1);
    let n0 = h.resolve(&w).await.host;
    assert!(
        h.node
            .peers()
            .await
            .unwrap()
            .iter()
            .all(|p| p.host_id != n0)
    );
    call_service(&h, &w, 2, n0).await;
    while !w.relay_done.get() {
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    }
    Ok(())
}

fn run(seed: u64) -> Vec<String> {
    let w = World {
        seed,
        trace: Rc::default(),
        relay: Rc::default(),
        relay_done: Rc::default(),
    };
    let mut sim = turmoil::Builder::new()
        .rng_seed(seed)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_secs(1_700_000_000))
        .simulation_duration(StdDuration::from_secs(120))
        .min_message_latency(StdDuration::from_millis(1))
        .max_message_latency(StdDuration::from_millis(15))
        .build();
    let ws = w.clone();
    sim.host("n0", move || server_host(ws.clone()));
    let w1 = w.clone();
    sim.host("n1", move || relay_host(w1.clone()));
    sim.client("n2", client_host(w.clone()));
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
fn vpn_rpc_reaches_a_service_through_a_relay() {
    let a = run(7);
    for l in &a {
        eprintln!("{l}");
    }
    let b = run(7);
    assert_eq!(digest(&a), digest(&b), "same seed, different traces");
    let c = run(8);
    assert_ne!(digest(&a), digest(&c), "different seeds, same trace");
}
