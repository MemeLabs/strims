// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! vpn connections over an in-memory network of ports, driven by a real
//! executor (tokio's current-thread runtime with paused time).

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration as StdDuration;

use futures_core::Stream;
use futures_sink::Sink;
use futures_util::StreamExt;
use prost::Message;
use strims_base::rng::{DetRng, det_rng};
use strims_base::time::{Duration, Time};
use strims_crypto::{
    DEFAULT_CERT_TTL, generate_key, host_certificate, new_certificate_request,
    new_self_signed_certificate, sign_certificate_request,
};
use strims_kademlia::ID;
use strims_proto::debug::v1::{
    DebugClient, DebugService, PProfRequest, PProfResponse, WatchMetricsRequest,
    WatchMetricsResponse, register_debug_service,
};
use strims_proto::rpc::Call;
use strims_proto::rpc::call::Kind;
use strims_proto::service::{CallError, Context as CallContext, Error, ServerStream};
use strims_proto::types::certificate::ParentOneof;
use strims_proto::types::{Certificate, Key, KeyUsage};
use strims_rpc::vpn::{self, CERTIFICATE_HEADER, VpnClient, VpnConfig, VpnError};
use strims_rpc::wire::CALLBACK;
use strims_rpc::{Client, RetryClient, RetryPolicy, Router};
use strims_vpn::{Delivery, HostAddr, M_BROADCAST, M_ENCRYPT, NetworkId};
use tokio::task::{LocalSet, spawn_local};
use tokio::time::{Instant, timeout};

mod common;
use common::TokioTimer;

const SIGN: i32 = KeyUsage::Sign as i32;
const PEER: i32 = KeyUsage::Peer as i32;

fn epoch() -> Time {
    Time::from_unix(1_700_000_000, 0)
}

// --- certificates ---------------------------------------------------------------

/// A network: its root key and certificate.
struct Network {
    key: Key,
    root: Certificate,
}

impl Network {
    fn new(rng: &mut DetRng) -> Network {
        let key = generate_key(rng);
        let root =
            new_self_signed_certificate(&key, SIGN, DEFAULT_CERT_TTL, "network", epoch(), rng)
                .unwrap();
        Network { key, root }
    }

    /// A profile's membership certificate, valid for `ttl`.
    fn member(&self, profile: &Key, ttl: Duration, rng: &mut DetRng) -> Certificate {
        let csr = new_certificate_request(profile, PEER | SIGN, "member").unwrap();
        let mut cert = sign_certificate_request(&csr, ttl, &self.key, epoch(), rng).unwrap();
        cert.parent_oneof = Some(ParentOneof::Parent(Box::new(self.root.clone())));
        cert
    }
}

/// A host with a profile on a network.
struct Host {
    id: ID,
    key: Key,
    profile: Key,
    cert: Certificate,
}

impl Host {
    fn new(net: &Network, rng: &mut DetRng) -> Host {
        Host::with_ttl(net, DEFAULT_CERT_TTL, rng)
    }

    fn with_ttl(net: &Network, ttl: Duration, rng: &mut DetRng) -> Host {
        let key = generate_key(rng);
        let profile = generate_key(rng);
        let member = net.member(&profile, ttl, rng);
        let cert = host_certificate(&key.public, &profile, &member).unwrap();
        Host {
            id: ID::try_from(key.public.as_slice()).unwrap(),
            key,
            profile,
            cert,
        }
    }

    fn addr(&self, port: u16) -> HostAddr {
        HostAddr::new(self.id, port)
    }
}

// --- the network ----------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct Closed;

#[derive(Default)]
struct Inbox {
    queue: VecDeque<Delivery<u16>>,
    waker: Option<Waker>,
}

#[derive(Default)]
struct NetState {
    ports: BTreeMap<HostAddr, Inbox>,
    /// Every Call sent: from, to, the Call.
    log: Vec<(HostAddr, HostAddr, Call)>,
    now: Time,
    /// The next this many Calls sent are lost.
    lose: usize,
}

/// Delivers each sent Call to the port it names, as vpn would: encrypted,
/// from the sender's host and port, at the network's current time.
#[derive(Clone, Default)]
struct Net(Rc<RefCell<NetState>>);

impl Net {
    fn new() -> Net {
        let net = Net::default();
        net.0.borrow_mut().now = epoch();
        net
    }

    fn port(&self, addr: HostAddr) -> (PortReader, PortWriter) {
        self.0.borrow_mut().ports.entry(addr).or_default();
        (
            PortReader {
                net: self.clone(),
                addr,
            },
            PortWriter {
                net: self.clone(),
                addr,
            },
        )
    }

    fn set_now(&self, now: Time) {
        self.0.borrow_mut().now = now;
    }

    /// Loses the next `n` Calls sent.
    fn lose(&self, n: usize) {
        self.0.borrow_mut().lose = n;
    }

    fn delivery(&self, from: HostAddr, to: HostAddr, body: Vec<u8>) -> Delivery<u16> {
        Delivery {
            network: NetworkId::default(),
            handler: to.port,
            src: from.host,
            src_port: from.port,
            dst: to.host,
            dst_port: to.port,
            flags: M_ENCRYPT,
            hops: usize::from(from.host != to.host),
            verified: None,
            body,
            received: self.0.borrow().now,
        }
    }

    /// Puts `d` in its port's inbox.
    fn inject(&self, d: Delivery<u16>) {
        let waker = {
            let mut s = self.0.borrow_mut();
            let Some(inbox) = s.ports.get_mut(&HostAddr::new(d.dst, d.dst_port)) else {
                return;
            };
            inbox.queue.push_back(d);
            inbox.waker.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
    }

    fn log(&self) -> Vec<(HostAddr, HostAddr, Call)> {
        self.0.borrow().log.clone()
    }
}

struct PortReader {
    net: Net,
    addr: HostAddr,
}

impl Stream for PortReader {
    type Item = Result<Delivery<u16>, Closed>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut s = self.net.0.borrow_mut();
        let Some(inbox) = s.ports.get_mut(&self.addr) else {
            return Poll::Ready(None);
        };
        match inbox.queue.pop_front() {
            Some(d) => Poll::Ready(Some(Ok(d))),
            None => {
                inbox.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

struct PortWriter {
    net: Net,
    addr: HostAddr,
}

impl Sink<(HostAddr, Vec<u8>)> for PortWriter {
    type Error = Closed;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, (to, body): (HostAddr, Vec<u8>)) -> Result<(), Closed> {
        let call = Call::decode(body.as_slice()).unwrap();
        let lost = {
            let mut s = self.net.0.borrow_mut();
            s.log.push((self.addr, to, call));
            let lost = s.lose > 0;
            s.lose = s.lose.saturating_sub(1);
            lost
        };
        if lost {
            return Ok(());
        }
        let d = self.net.delivery(self.addr, to, body);
        self.net.inject(d);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Poll::Ready(Ok(()))
    }
}

// --- the service under test -----------------------------------------------------

#[derive(Default)]
struct TestService {
    /// Requests that reached a handler.
    calls: Cell<usize>,
}

impl DebugService for TestService {
    /// Answers with the caller's profile key, from its membership
    /// certificate.
    async fn p_prof(&self, ctx: &CallContext, req: PProfRequest) -> Result<PProfResponse, Error> {
        self.calls.set(self.calls.get() + 1);
        let profile = ctx
            .certificate()
            .and_then(strims_crypto::certificate_parent)
            .map(|c| c.key.clone())
            .unwrap_or_default();
        Ok(PProfResponse {
            name: req.name,
            data: profile,
        })
    }

    /// Streams `interval_ms` items, or never ends if it's negative.
    async fn watch_metrics(
        &self,
        _ctx: &CallContext,
        req: WatchMetricsRequest,
    ) -> Result<ServerStream<WatchMetricsResponse>, Error> {
        self.calls.set(self.calls.get() + 1);
        if req.interval_ms < 0 {
            return Ok(Box::pin(futures_util::stream::pending()));
        }
        let items = (0..req.interval_ms).map(|i| {
            Ok(WatchMetricsResponse {
                data: vec![u8::try_from(i).unwrap()],
            })
        });
        Ok(Box::pin(futures_util::stream::iter(items)))
    }
}

/// Runs `f` on a local task set with a timeout.
async fn local<F: Future>(f: F) -> F::Output {
    LocalSet::new()
        .run_until(timeout(StdDuration::from_secs(60), f))
        .await
        .expect("test timed out")
}

/// Lets the connections run.
async fn settle() {
    tokio::time::sleep(StdDuration::from_millis(10)).await;
}

type Metrics = Rc<dyn Fn() -> vpn::Metrics>;

/// Runs a connection for `host` on `port`, serving the test service if
/// `service` is set. Returns its client and a reader of its counters.
fn start(
    net: &Net,
    host: &Host,
    port: u16,
    service: Option<Rc<TestService>>,
    config: VpnConfig,
) -> (VpnClient, Metrics) {
    let (reader, writer) = net.port(host.addr(port));
    let router = service.map(|s| {
        let mut r = Router::new();
        register_debug_service(&mut r, s);
        r
    });
    let (client, conn) = vpn::connect(config, &host.cert, reader, writer, router).unwrap();
    let conn = Rc::new(RefCell::new(Some(conn)));
    let metrics: Metrics = Rc::new({
        let conn = conn.clone();
        move || {
            conn.borrow()
                .as_ref()
                .map(|c| c.vpn_metrics())
                .unwrap_or_default()
        }
    });
    spawn_local(async move {
        let fut = futures_util::future::poll_fn(move |cx| {
            let mut c = conn.borrow_mut();
            match c.as_mut() {
                Some(conn) => Pin::new(conn).poll(cx),
                None => Poll::Ready(Ok(())),
            }
        });
        let _ = fut.await;
    });
    (client, metrics)
}

fn pprof(name: &str) -> PProfRequest {
    PProfRequest {
        name: name.into(),
        ..Default::default()
    }
}

async fn call(client: &Client, name: &str) -> Result<PProfResponse, CallError> {
    DebugClient::new(client.clone()).p_prof(&pprof(name)).await
}

// --- tests ----------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn calls_reach_servers_and_handlers_see_the_caller() {
    local(async {
        let rng = &mut det_rng(1);
        let network = Network::new(rng);
        let (server, client) = (Host::new(&network, rng), Host::new(&network, rng));
        let net = Net::new();
        let service = Rc::new(TestService::default());
        let (server_client, _) = start(
            &net,
            &server,
            100,
            Some(service.clone()),
            VpnConfig::default(),
        );
        let (vpn_client, _) = start(&net, &client, 200, None, VpnConfig::default());

        let to_server = vpn_client.to(server.addr(100));
        let res = call(&to_server, "a").await.unwrap();
        assert_eq!(res.name, "a");
        assert_eq!(res.data, client.profile.public);

        let mut stream = DebugClient::new(to_server.clone())
            .watch_metrics(&WatchMetricsRequest {
                interval_ms: 3,
                ..Default::default()
            })
            .await
            .unwrap();
        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            items.push(item.unwrap().data);
        }
        assert_eq!(items, [vec![0], vec![1], vec![2]]);

        // a host calls its own port, where it also serves
        let res = call(&server_client.to(server.addr(100)), "self")
            .await
            .unwrap();
        assert_eq!(res.data, server.profile.public);
        assert_eq!(service.calls.get(), 3);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_lost_request_is_sent_again() {
    local(async {
        let rng = &mut det_rng(9);
        let network = Network::new(rng);
        let (server, client) = (Host::new(&network, rng), Host::new(&network, rng));
        let net = Net::new();
        let service = Rc::new(TestService::default());
        start(
            &net,
            &server,
            100,
            Some(service.clone()),
            VpnConfig::default(),
        );
        let (vpn_client, _) = start(&net, &client, 200, None, VpnConfig::default());
        let to_server = RetryClient::new(
            vpn_client.to(server.addr(100)),
            TokioTimer::new(epoch()),
            RetryPolicy::vpn(),
        );

        net.lose(1);
        let begin = Instant::now();
        let res = DebugClient::new(to_server).p_prof(&pprof("a")).await;
        assert_eq!(res.unwrap().name, "a");
        // a 10 s attempt, then a 1 s delay
        assert_eq!(begin.elapsed(), StdDuration::from_secs(11));
        assert_eq!(service.calls.get(), 1);
        let sent: Vec<(HostAddr, String)> = net
            .log()
            .into_iter()
            .map(|(from, _, call)| (from, call.method))
            .collect();
        let pprof = "strims.debug.v1.Debug.PProf".to_owned();
        assert_eq!(
            sent,
            [
                (client.addr(200), pprof.clone()),
                (client.addr(200), "CANCEL".to_owned()),
                (client.addr(200), pprof),
                (server.addr(100), CALLBACK.to_owned()),
            ]
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_call_carries_the_senders_certificate() {
    local(async {
        let rng = &mut det_rng(2);
        let network = Network::new(rng);
        let (server, client) = (Host::new(&network, rng), Host::new(&network, rng));
        let net = Net::new();
        start(
            &net,
            &server,
            100,
            Some(Rc::default()),
            VpnConfig::default(),
        );
        let (vpn_client, _) = start(&net, &client, 200, None, VpnConfig::default());
        let to_server = vpn_client.to(server.addr(100));

        call(&to_server, "a").await.unwrap();
        let stream = DebugClient::new(to_server)
            .watch_metrics(&WatchMetricsRequest {
                interval_ms: -1,
                ..Default::default()
            })
            .await
            .unwrap();
        settle().await;
        drop(stream);
        settle().await;

        let log = net.log();
        let methods: Vec<&str> = log.iter().map(|(_, _, c)| c.method.as_str()).collect();
        assert_eq!(
            methods,
            [
                "strims.debug.v1.Debug.PProf",
                CALLBACK,
                "strims.debug.v1.Debug.WatchMetrics",
                "CANCEL"
            ]
        );
        for (from, _, c) in &log {
            let sender = if from.host == server.id {
                &server
            } else {
                &client
            };
            assert_eq!(
                c.headers.get(CERTIFICATE_HEADER),
                Some(&sender.cert.encode_to_vec())
            );
        }
    })
    .await;
}

/// A request as `from` would send it, with `cert` as its header.
fn raw_request(id: u64, cert: Option<&Certificate>) -> Call {
    Call {
        id,
        method: "strims.debug.v1.Debug.PProf".into(),
        argument: pprof("raw").encode_to_vec(),
        headers: cert
            .map(|c| BTreeMap::from([(CERTIFICATE_HEADER.to_owned(), c.encode_to_vec())]))
            .unwrap_or_default(),
        ..Default::default()
    }
}

#[tokio::test(start_paused = true)]
async fn calls_failing_a_check_are_dropped_and_others_still_served() {
    local(async {
        let rng = &mut det_rng(3);
        let network = Network::new(rng);
        let other_network = Network::new(rng);
        let server = Host::new(&network, rng);
        let mallory = Host::new(&network, rng);
        let honest = Host::new(&network, rng);
        let stranger = Host::new(&other_network, rng);
        let short_lived = Host::with_ttl(&network, Duration::HOUR, rng);
        let net = Net::new();
        let service = Rc::new(TestService::default());
        let config = VpnConfig {
            rpc: strims_rpc::Config {
                max_message_bytes: 4096,
                ..strims_rpc::Config::vpn()
            },
            max_certificate_bytes: 1024,
            ..VpnConfig::default()
        };
        let (_, metrics) = start(&net, &server, 100, Some(service.clone()), config);
        let to = server.addr(100);
        let from = mallory.addr(5);
        let req = |cert: &Certificate| raw_request(1, Some(cert)).encode_to_vec();

        let mut d = net.delivery(from, to, req(&mallory.cert));
        d.verified = Some(false);
        net.inject(d);
        let mut d = net.delivery(from, to, req(&mallory.cert));
        d.flags = 0;
        net.inject(d);
        let mut d = net.delivery(from, to, req(&mallory.cert));
        d.flags = M_ENCRYPT | M_BROADCAST;
        net.inject(d);
        let mut padded = raw_request(1, Some(&mallory.cert));
        padded.argument = vec![0; 4096];
        net.inject(net.delivery(from, to, padded.encode_to_vec()));
        net.inject(net.delivery(from, to, vec![0xff, 0xff]));
        net.inject(net.delivery(from, to, raw_request(1, None).encode_to_vec()));
        let mut big = mallory.cert.clone();
        big.subject = "x".repeat(1024);
        net.inject(net.delivery(from, to, req(&big)));

        // someone else's certificate
        net.inject(net.delivery(from, to, req(&honest.cert)));
        // another network's member
        net.inject(net.delivery(stranger.addr(5), to, req(&stranger.cert)));
        // a host certificate signed by the network root directly
        let shallow = host_certificate(&mallory.key.public, &network.key, &network.root).unwrap();
        net.inject(net.delivery(from, to, req(&shallow)));
        // a tampered certificate
        let mut forged = mallory.cert.clone();
        forged.not_after += 1;
        net.inject(net.delivery(from, to, req(&forged)));
        // an expired one, at the time it arrives
        net.set_now(epoch() + Duration::HOUR * 2);
        net.inject(net.delivery(short_lived.addr(5), to, req(&short_lived.cert)));
        settle().await;

        assert_eq!(
            metrics(),
            vpn::Metrics {
                received: 12,
                admitted: 0,
                unverified: 1,
                unencrypted: 2,
                oversized: 1,
                malformed: 1,
                missing_certificate: 2,
                rejected_certificate: 3,
                invalid_certificate: 2,
                no_room: 0,
            }
        );
        assert_eq!(service.calls.get(), 0);

        let (vpn_client, _) = start(&net, &honest, 200, None, VpnConfig::default());
        let res = call(&vpn_client.to(to), "ok").await.unwrap();
        assert_eq!(res.data, honest.profile.public);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_certificate_is_checked_again_when_it_expires() {
    local(async {
        let rng = &mut det_rng(4);
        let network = Network::new(rng);
        let server = Host::new(&network, rng);
        let client = Host::with_ttl(&network, Duration::HOUR, rng);
        let net = Net::new();
        let (_, metrics) = start(
            &net,
            &server,
            100,
            Some(Rc::default()),
            VpnConfig::default(),
        );
        let (vpn_client, _) = start(&net, &client, 200, None, VpnConfig::default());
        let to_server = vpn_client.to(server.addr(100));
        call(&to_server, "a").await.unwrap();

        net.set_now(epoch() + Duration::HOUR * 2);
        let late = timeout(StdDuration::from_secs(1), call(&to_server, "b")).await;
        assert!(late.is_err(), "an expired caller gets no answer");
        settle().await;
        // the request and the cancel that dropping it sent
        assert_eq!(metrics().invalid_certificate, 2);
        assert_eq!(metrics().admitted, 1);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn answers_count_only_from_the_called_address() {
    local(async {
        let rng = &mut det_rng(5);
        let network = Network::new(rng);
        let (server, client, mallory) = (
            Host::new(&network, rng),
            Host::new(&network, rng),
            Host::new(&network, rng),
        );
        let net = Net::new();
        let service = Rc::new(TestService::default());
        let (vpn_client, _) = start(&net, &client, 200, None, VpnConfig::default());
        // the server isn't up yet, so the request waits
        let to_server = vpn_client.to(server.addr(100));
        let pending = spawn_local(async move { call(&to_server, "a").await });
        settle().await;
        let request = net.log()[0].2.clone();

        let forged = Call {
            id: 99,
            parent_id: request.id,
            method: CALLBACK.into(),
            kind: Kind::CallKindDefault as i32,
            argument: PProfResponse {
                name: "forged".into(),
                data: Vec::new(),
            }
            .encode_to_vec(),
            headers: BTreeMap::from([(
                CERTIFICATE_HEADER.to_owned(),
                mallory.cert.encode_to_vec(),
            )]),
        };
        net.inject(net.delivery(mallory.addr(100), client.addr(200), forged.encode_to_vec()));
        settle().await;
        assert!(!pending.is_finished());

        // the real server answers a resent request
        start(&net, &server, 100, Some(service), VpnConfig::default());
        net.inject(net.delivery(client.addr(200), server.addr(100), request.encode_to_vec()));
        let res = pending.await.unwrap().unwrap();
        assert_eq!(res.name, "a");
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn remote_addresses_are_bounded_per_port_and_per_host() {
    local(async {
        let rng = &mut det_rng(6);
        let network = Network::new(rng);
        let server = Host::new(&network, rng);
        let (h1, h2, h3) = (
            Host::new(&network, rng),
            Host::new(&network, rng),
            Host::new(&network, rng),
        );
        let net = Net::new();
        let service = Rc::new(TestService::default());
        let config = VpnConfig {
            max_remotes: 2,
            max_remotes_per_host: 1,
            ..VpnConfig::default()
        };
        let (_, metrics) = start(&net, &server, 100, Some(service.clone()), config);
        let to = server.addr(100);
        let hang = |h: &Host| Call {
            id: 1,
            method: "strims.debug.v1.Debug.WatchMetrics".into(),
            argument: WatchMetricsRequest {
                interval_ms: -1,
                ..Default::default()
            }
            .encode_to_vec(),
            headers: BTreeMap::from([(CERTIFICATE_HEADER.to_owned(), h.cert.encode_to_vec())]),
            ..Default::default()
        };

        // two open streams fill the table; a second port of h1 and a third
        // host find no room
        for (h, port) in [(&h1, 1), (&h2, 1), (&h1, 2), (&h3, 1)] {
            net.inject(net.delivery(h.addr(port), to, hang(h).encode_to_vec()));
        }
        settle().await;
        assert_eq!(service.calls.get(), 2);
        assert_eq!(metrics().no_room, 2);

        // once h1 cancels and the server has nothing left for it, its
        // address makes room
        let cancel = Call {
            id: 2,
            parent_id: 1,
            method: "CANCEL".into(),
            kind: Kind::CallKindCancel as i32,
            headers: BTreeMap::from([(CERTIFICATE_HEADER.to_owned(), h1.cert.encode_to_vec())]),
            ..Default::default()
        };
        net.inject(net.delivery(h1.addr(1), to, cancel.encode_to_vec()));
        net.inject(net.delivery(h3.addr(1), to, hang(&h3).encode_to_vec()));
        settle().await;
        assert_eq!(service.calls.get(), 3);
        assert_eq!(metrics().no_room, 2);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn clients_keep_their_addresses_outside_the_bound() {
    local(async {
        let rng = &mut det_rng(7);
        let network = Network::new(rng);
        let client = Host::new(&network, rng);
        let servers: Vec<Host> = (0..3).map(|_| Host::new(&network, rng)).collect();
        let net = Net::new();
        for s in &servers {
            start(&net, s, 100, Some(Rc::default()), VpnConfig::default());
        }
        let config = VpnConfig {
            max_remotes: 1,
            ..VpnConfig::default()
        };
        let (vpn_client, _) = start(&net, &client, 200, None, config);
        let clients: Vec<Client> = servers.iter().map(|s| vpn_client.to(s.addr(100))).collect();
        for _ in 0..2 {
            for c in &clients {
                call(c, "x").await.unwrap();
            }
        }
    })
    .await;
}

#[test]
fn own_certificates_must_be_host_certificates_on_one_network() {
    let rng = &mut det_rng(8);
    let network = Network::new(rng);
    let other = Network::new(rng);
    let host = Host::new(&network, rng);
    let net = Net::new();

    let (reader, writer) = net.port(host.addr(1));
    let member = strims_crypto::certificate_parent(&host.cert)
        .unwrap()
        .clone();
    let err = vpn::connect(VpnConfig::default(), &member, reader, writer, None).unwrap_err();
    assert_eq!(err, VpnError::NotHostCertificate);

    let (reader, writer) = net.port(host.addr(2));
    let (client, _conn) =
        vpn::connect(VpnConfig::default(), &host.cert, reader, writer, None).unwrap();
    let elsewhere = Host::new(&other, rng);
    assert_eq!(
        client.set_certificate(&elsewhere.cert),
        Err(VpnError::NetworkMismatch)
    );
    assert_eq!(
        client.set_certificate(&member),
        Err(VpnError::NotHostCertificate)
    );
    let renewed = Host::new(&network, rng);
    client.set_certificate(&renewed.cert).unwrap();
}
