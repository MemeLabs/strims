// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! RPC over vpn between a Go and a Rust node: Go's `internal/network/dialer`
//! (`VPNServerDialer`, `VPNDialer`, the hash table address publisher and
//! resolver) and `strims_rpc::vpn` on `Node::open_vpn_port`. Both serve the
//! test Debug service of `crates/strims-rpc/golden/debugsvc`, and each
//! side's certificate chains to the same network key.
//!
//! The Rust node doesn't check that cancels reach the Go server: Go's vpn
//! transport looks up a cancelled call among its outgoing calls, so its
//! servers never act on a cancel.

use std::rc::Rc;

use futures_util::StreamExt;
use strims_base::rng::det_rng;
use strims_crypto::{
    DEFAULT_CERT_TTL, host_certificate, key_from_seed, new_certificate_request,
    new_self_signed_certificate, sign_certificate_request,
};
use strims_node::{Clock, HashTableGetOptions, HostAddr, SystemClock};
use strims_proto::debug::v1::{DebugClient, GetConfigRequest, register_debug_service};
use strims_proto::service::{CallError, Caller, Error, Method};
use strims_proto::types::certificate::ParentOneof;
use strims_proto::types::{Certificate, KeyUsage};
use strims_rpc::vpn::{self, VpnConfig};
use strims_rpc::{Client, Router};
use tokio::task::{LocalSet, spawn_local};

use crate::harness::{
    GoNode, GoOptions, NETWORK_SEED, RustNode, RustOptions, STEP, TestNode, Transport, helper,
};
use crate::rpc::{Nope, TestService, data, pprof, wait_count, watch};

/// The Go node's service port.
const GO_PORT: u16 = 3000;
const SALT: &[u8] = b"debug";

/// A Rust node's host certificate, its membership issued by the network
/// key as Go's `dao.NewNetwork` issues it.
fn rust_certificate(r: &RustNode, seed: u8) -> Certificate {
    let rng = &mut det_rng(u64::from(seed));
    let now = SystemClock.now();
    let network = key_from_seed(&NETWORK_SEED);
    let profile = key_from_seed(&[seed; 32]);
    let root = new_self_signed_certificate(
        &network,
        KeyUsage::Sign as i32,
        DEFAULT_CERT_TTL,
        "network",
        now,
        rng,
    )
    .unwrap();
    let usage = KeyUsage::Peer as i32 | KeyUsage::Sign as i32;
    let csr = new_certificate_request(&profile, usage, &r.name).unwrap();
    let mut member = sign_certificate_request(&csr, DEFAULT_CERT_TTL, &network, now, rng).unwrap();
    member.parent_oneof = Some(ParentOneof::Parent(Box::new(root)));
    host_certificate(&r.node.host_id().to_bytes(), &profile, &member).unwrap()
}

/// The address published under (`key`, [`SALT`]), once the hash table has
/// it.
async fn resolve(r: &RustNode, key: Vec<u8>) -> HostAddr {
    tokio::time::timeout(STEP, async {
        loop {
            let options = HashTableGetOptions {
                disable_cache: true,
                timeout: strims_base::time::Duration::from_secs(1),
            };
            let addr = r
                .node
                .resolve_vpn_address(r.net, key.clone(), SALT.to_vec(), options)
                .await
                .unwrap();
            if let Some(addr) = addr {
                return addr;
            }
        }
    })
    .await
    .expect("resolving the Go service")
}

/// The Rust node calls the Go node's service.
async fn rust_calls_go(client: Client) {
    let debug = DebugClient::new(client.clone());

    let res = debug.p_prof(&pprof("30000")).await.unwrap();
    assert!(res.data == data(30_000));
    let res = debug.p_prof(&pprof("y".repeat(30_000))).await.unwrap();
    assert_eq!(res.name, "30000");

    let items: Vec<_> = debug
        .watch_metrics(&watch(5))
        .await
        .unwrap()
        .collect()
        .await;
    let items: Vec<Vec<u8>> = items.into_iter().map(|i| i.unwrap().data).collect();
    assert_eq!(items, [[0], [1], [2], [3], [4]]);

    assert_eq!(
        debug.get_config(&GetConfigRequest::default()).await,
        Err(CallError::Remote(Error::new(7, "no config")))
    );
    let mut negative = debug.watch_metrics(&watch(-1)).await.unwrap();
    assert_eq!(
        negative.next().await,
        Some(Err(CallError::Remote(Error::new(3, "negative interval"))))
    );
    assert_eq!(
        client.call_unary::<Nope>(&pprof("")).await,
        Err(CallError::Remote(Error::method_not_found(Nope::NAME)))
    );
}

/// The Go node calls the Rust node's service at `port`, and through the
/// hash table under `key`.
async fn go_calls_rust(g: &mut GoNode, r: &str, port: u16, key: &[u8], service: &TestService) {
    let mut cmd = async |line: String| g.cmd(&line).await.unwrap();
    let to = format!("vpn-rpc {r} {port}");
    assert_eq!(cmd(format!("{to} pprof 30000 0")).await, "30000 30000 ok");
    assert_eq!(cmd(format!("{to} pprof 0 30000")).await, "30000 0 ok");
    assert_eq!(cmd(format!("{to} watch 5")).await, "5 0001020304");
    assert_eq!(cmd(format!("{to} watch-cancel 3")).await, "canceled");
    wait_count("rust", 1, async || service.cancelled.get()).await;
    assert_eq!(cmd(format!("{to} read-deadline 200")).await, "deadline");
    wait_count("rust", 2, async || service.cancelled.get()).await;
    assert_eq!(cmd(format!("{to} cancels")).await, "2");

    let err = |code: i32, message: &str| format!("{code} {}", hex::encode(message));
    assert_eq!(cmd(format!("{to} config")).await, err(7, "no config"));
    assert_eq!(
        cmd(format!("{to} negative")).await,
        err(3, "negative interval")
    );
    assert_eq!(
        cmd(format!("{to} raw {}", Nope::NAME)).await,
        err(0, &format!("method not found: {}", Nope::NAME))
    );

    let dht = format!("vpn-rpc-dht {} {}", hex::encode(key), hex::encode(SALT));
    assert_eq!(cmd(format!("{dht} pprof 10 0")).await, "10 10 ok");
}

async fn vpn_rpc(t: Transport, seed: u8) {
    let Some(bin) = helper() else {
        return;
    };
    let mut o = GoOptions::new(seed);
    o.vpn_rpc = true;
    let mut g = TestNode::Go(GoNode::spawn(&bin, &format!("g{seed}"), o).await);
    let rust_seed = seed + 1;
    let mut r =
        TestNode::Rust(RustNode::spawn(&format!("r{seed}"), RustOptions::new(rust_seed)).await);
    let (g_id, r_id) = (g.host_id(), r.host_id());
    assert_eq!(r.dial(&g.uri(t)).await, g_id);
    r.wait_peer(g_id).await;
    g.wait_peer(r_id).await;

    // each side's service, published under a key of its own
    let (go_key, rust_key) = ([seed; 32], [rust_seed.wrapping_add(0x40); 32]);
    g.go()
        .cmd(&format!(
            "vpn-serve {GO_PORT} {} {}",
            hex::encode(go_key),
            hex::encode(SALT)
        ))
        .await
        .unwrap();
    let go_profile = g.go().profile_key.clone();

    let certificate = rust_certificate(r.rust(), rust_seed);
    let r = r.rust();
    let port = r.node.open_vpn_port(r.net, None).await.unwrap();
    let rust_port = port.port;
    r.node
        .publish_vpn_address(r.net, key_from_seed(&rust_key), SALT.to_vec(), rust_port)
        .await
        .unwrap();
    let go_addr = resolve(r, key_from_seed(&go_key).public).await;
    assert_eq!(go_addr, HostAddr::new(g_id, GO_PORT));

    let service = Rc::new(TestService::default());
    LocalSet::new()
        .run_until(async move {
            let mut router = Router::new();
            register_debug_service(&mut router, service.clone());
            let (client, conn) = vpn::connect(
                VpnConfig::default(),
                &certificate,
                port.reader,
                port.writer,
                Some(router),
            )
            .unwrap();
            let conn = spawn_local(conn);
            rust_calls_go(client.to(go_addr)).await;
            let rust_key = key_from_seed(&rust_key).public;
            let r_hex = hex::encode(r_id.to_bytes());
            go_calls_rust(g.go(), &r_hex, rust_port, &rust_key, &service).await;
            // every call the Rust service answered carried the Go profile's
            // membership
            let callers = service.callers.borrow().clone();
            assert!(!callers.is_empty());
            assert!(callers.iter().all(|c| *c == go_profile));
            conn.abort();
        })
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn vpn_rpc_both_ways() {
    crate::bounded(180, async {
        vpn_rpc(Transport::Tcp, 0x74).await;
        vpn_rpc(Transport::Ws, 0x76).await;
    })
    .await;
}
