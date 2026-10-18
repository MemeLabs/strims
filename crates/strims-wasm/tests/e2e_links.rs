// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! WebRTC links between browser nodes and native nodes, made by peer
//! exchange, in headless Firefox.
//!
//! b is a native relay with a WebSocket listener on 127.0.0.1; a is a
//! native node with only WebRTC (and TCP to reach b); the page's nodes
//! (strims-wasm's `e2e` node) dial b over `ws://`. Everyone joins one
//! network with `auto_bind_port`. Then
//!
//! - a peer exchange through b links two of them over WebRTC: a offers
//!   and the browser answers, the browser offers to a, or one page node
//!   offers to another in the same browser. Both sides see `PeerAdded`
//!   with kind WebRtc, and `dialed` set on the offering side only;
//! - b shuts down, so only the new link is left;
//! - 256 vpn messages of 16 KiB, numbered, go to the far side, which
//!   echoes each back; all return in order, byte-exact, over the one
//!   link. The browser nodes' data channels are capped at
//!   [`MAX_BUFFERED`], below one message, so their writers wait for
//!   `bufferedamountlow`, and their 64 KiB read buffers join some data
//!   channel messages and split others.
//!
//! Firefox neither gathers nor checks loopback candidates, so a binds the
//! outbound interface's address, and without one the tests skip. The
//! harness's Firefox prefs turn mDNS obfuscation off, so two pages of one
//! browser link without resolving each other's mDNS names.
//!
//! Ignored by default (they launch a browser):
//!
//! ```sh
//! cargo test -p strims-wasm --test e2e_links -- --ignored --test-threads=1
//! ```

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    // the wall clock, for timings
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

mod support;

use std::net::IpAddr;
use std::time::{Duration, Instant};

use strims_crypto::key_from_seed;
use strims_node::{
    Delivery, ExchangeOutcome, HostId, LinkKind, NetworkId, NetworkOptions, Node, NodeConfig,
    NodeEvent, NodeEvents, PeerInfo, SystemClock, TokioNet, WebRtcConfig,
};
use support::browser::{Browser, Control, PagePeer, STEP, StartOptions, outbound_ip};

const BIND_PORT: u16 = 10_000;
/// Messages to a node's echo port come back to their sender's port.
const ECHO_PORT: u16 = 20_000;
const REPLY_PORT: u16 = 20_001;
const MESSAGES: u32 = 256;
const MESSAGE_LEN: usize = 16 * 1024;
/// Messages sent and not yet echoed. vpn drops messages a link's queue
/// (`link_queue_bytes`, 256 KiB) can't hold, so the sender waits for
/// echoes once this many are out.
const WINDOW: u32 = 8;
/// The browser nodes' `max_buffered_amount`: under one message's frame,
/// so a channel write right after another waits for `bufferedamountlow`.
const MAX_BUFFERED: usize = 16 * 1024;
/// How long the echoes may take in all.
const ECHO_TIMEOUT: Duration = Duration::from_secs(120);

fn network_key() -> Vec<u8> {
    key_from_seed(&[0xee; 32]).public
}

fn hex_id(id: HostId) -> String {
    hex::encode(id.to_bytes())
}

fn host_id(hex: &str) -> HostId {
    HostId::from_bytes(&hex::decode(hex).unwrap().try_into().unwrap())
}

/// Message `seq`: its number, then bytes that depend on it.
fn body(seq: u32) -> Vec<u8> {
    let mut b = Vec::with_capacity(MESSAGE_LEN + 8);
    b.extend_from_slice(&seq.to_le_bytes());
    let mut state = u64::from(seq);
    while b.len() < MESSAGE_LEN {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        b.extend_from_slice(&(z ^ (z >> 31)).to_le_bytes());
    }
    b.truncate(MESSAGE_LEN);
    b
}

/// The echo of message `want`: next in order and intact.
fn check_echo(want: u32, got: &[u8]) {
    assert_eq!(got.len(), MESSAGE_LEN, "echo {want}: length");
    let seq = u32::from_le_bytes(got[..4].try_into().unwrap());
    assert_eq!(seq, want, "echoes out of order");
    assert!(got == body(seq), "echo {seq}: bytes differ");
}

/// The address a native WebRTC node binds, or `None` after a skip
/// message.
fn lan_ip() -> Option<IpAddr> {
    let ip = outbound_ip();
    if ip.is_none() {
        eprintln!("skipping: no interface other than loopback for WebRTC host candidates");
    }
    ip
}

struct Native {
    node: Node,
    net: NetworkId,
    events: NodeEvents,
    seen: Vec<NodeEvent>,
}

impl Native {
    /// b: TCP and WebSocket listeners on 127.0.0.1, no WebRTC.
    async fn relay(seed: u8) -> Native {
        Native::spawn(seed, |c| {
            c.tcp_listen = Some("127.0.0.1:0".to_owned());
            c.ws_listen = Some("127.0.0.1:0".to_owned());
        })
        .await
    }

    /// a: WebRTC only, on `ip`.
    async fn webrtc(seed: u8, ip: IpAddr) -> Native {
        let n = Native::spawn(seed, |c| {
            c.webrtc = WebRtcConfig::local([format!("{ip}:0")])
        })
        .await;
        assert_eq!(n.node.info().webrtc_addrs.len(), 1);
        n
    }

    async fn spawn(seed: u8, f: impl FnOnce(&mut NodeConfig)) -> Native {
        let mut config = NodeConfig::new(key_from_seed(&[seed; 32]), u64::from(seed));
        config.label = format!("node-{seed}");
        config.allow_insecure_ws = true;
        config.max_peers = 16;
        f(&mut config);
        let (node, events) = Node::spawn(config, TokioNet, SystemClock).await.unwrap();
        let options = NetworkOptions {
            auto_bind_port: Some(BIND_PORT),
        };
        let net = node.add_network(&network_key(), options).await.unwrap();
        node.set_vpn_handler(net, REPLY_PORT).await.unwrap();
        Native {
            node,
            net,
            events,
            seen: Vec::new(),
        }
    }

    fn id(&self) -> HostId {
        self.node.host_id()
    }

    /// The first event `f` accepts within `timeout`, keeping the others.
    async fn try_wait(
        &mut self,
        timeout: Duration,
        mut f: impl FnMut(&NodeEvent) -> bool,
    ) -> Option<NodeEvent> {
        if let Some(i) = self.seen.iter().position(&mut f) {
            return Some(self.seen.remove(i));
        }
        tokio::time::timeout(timeout, async {
            loop {
                let e = self.events.recv().await.expect("node stopped");
                if f(&e) {
                    return e;
                }
                self.seen.push(e);
            }
        })
        .await
        .ok()
    }

    async fn wait(&mut self, what: &str, f: impl FnMut(&NodeEvent) -> bool) -> NodeEvent {
        match self.try_wait(STEP, f).await {
            Some(e) => e,
            None => panic!("timed out waiting for {what}; seen {:?}", self.seen),
        }
    }

    async fn peer_added(&mut self, peer: HostId) -> PeerInfo {
        match self
            .wait(
                &format!("PeerAdded {peer}"),
                |e| matches!(e, NodeEvent::PeerAdded(p) if p.host_id == peer),
            )
            .await
        {
            NodeEvent::PeerAdded(p) => p,
            _ => unreachable!(),
        }
    }

    async fn peer_removed(&mut self, peer: HostId) {
        self.wait(
            &format!("PeerRemoved {peer}"),
            |e| matches!(e, NodeEvent::PeerRemoved { host_id } if *host_id == peer),
        )
        .await;
    }

    async fn exchange_closed(&mut self, peer: HostId) -> ExchangeOutcome {
        match self
            .wait(
                &format!("ExchangeClosed {peer}"),
                |e| matches!(e, NodeEvent::ExchangeClosed { host_id, .. } if *host_id == peer),
            )
            .await
        {
            NodeEvent::ExchangeClosed { outcome, .. } => outcome,
            _ => unreachable!(),
        }
    }

    async fn try_reply(&mut self, from: HostId, timeout: Duration) -> Option<Delivery<u16>> {
        let e = self
            .try_wait(
                timeout,
                |e| matches!(e, NodeEvent::Message(d) if d.src == from && d.dst_port == REPLY_PORT),
            )
            .await?;
        match e {
            NodeEvent::Message(d) => Some(d),
            _ => unreachable!(),
        }
    }

    /// Dials the relay over TCP and waits for both ends.
    async fn join(&mut self, relay: &mut Native) {
        let uri = relay.node.tcp_uri().unwrap().to_owned();
        assert_eq!(self.node.dial(&uri).await.unwrap(), relay.id());
        self.peer_added(relay.id()).await;
        relay.peer_added(self.id()).await;
    }
}

/// Starts page node `i`, has it dial the relay over `ws://` and waits for
/// both ends. Returns its host id.
async fn join_page(page: &mut Control, i: u8, relay: &mut Native) -> HostId {
    let id = page
        .start(&StartOptions {
            profile_seed: [0x10 + i; 32],
            rng_seed: 0x10 + u64::from(i),
            network_key: network_key(),
            bind_port: BIND_PORT,
            echo_port: ECHO_PORT,
            reply_port: REPLY_PORT,
            max_buffered_amount: Some(MAX_BUFFERED),
        })
        .await;
    let uri = relay.node.ws_uri().unwrap().to_owned();
    assert_eq!(page.dial(&uri).await, hex_id(relay.id()));
    let p = page.peer_added(&hex_id(relay.id())).await;
    assert_eq!(
        p,
        PagePeer {
            kind: "ws".to_owned(),
            dialed: true
        }
    );
    let id = host_id(&id);
    relay.peer_added(id).await;
    id
}

/// Shuts the relay down and waits for its peers to see it go.
async fn drop_relay(relay: Native, natives: &mut [&mut Native], pages: &mut [&mut Control]) {
    let id = relay.id();
    relay.node.shutdown().await.unwrap();
    for n in natives {
        n.peer_removed(id).await;
    }
    for p in pages {
        p.peer_removed(&hex_id(id)).await;
    }
}

fn webrtc_peer(dialed: bool) -> PagePeer {
    PagePeer {
        kind: "webrtc".to_owned(),
        dialed,
    }
}

/// The page's node echoed every message over its one link, a WebRTC link.
async fn assert_page_echoed(page: &mut Control) {
    let stats = page.stats().await;
    eprintln!("echoing page node: {stats}");
    assert_eq!(stats["echoed"], u64::from(MESSAGES), "{stats}");
    assert_eq!(stats["echoErrors"], 0, "{stats}");
    let links = stats["links"].as_array().unwrap();
    assert_eq!(links.len(), 1, "{stats}");
    assert_eq!(links[0]["kind"], "webrtc", "{stats}");
}

/// a sends the numbered messages to the page's echo port, at most
/// [`WINDOW`] unanswered, and checks every echo.
async fn echo_from_native(a: &mut Native, peer: HostId, page: &mut Control) {
    let started = Instant::now();
    let (mut sent, mut got) = (0, 0);
    while got < MESSAGES {
        while sent < MESSAGES && sent - got < WINDOW {
            a.node
                .send(a.net, peer, ECHO_PORT, REPLY_PORT, body(sent))
                .await
                .unwrap();
            sent += 1;
        }
        let left = ECHO_TIMEOUT.saturating_sub(started.elapsed());
        let Some(d) = a.try_reply(peer, left.min(STEP)).await else {
            let stats = page.stats().await;
            let m = a.node.metrics().await.unwrap();
            panic!(
                "timed out waiting for echo {got} ({sent} sent); page {stats}; \
                 native vpn {:?}; native driver {:?}; native links {:?}",
                m.vpn, m.driver, m.links
            );
        };
        assert_eq!(d.hops, 1, "the echo came over the direct link");
        check_echo(got, &d.body);
        got += 1;
    }
    let elapsed = started.elapsed();
    eprintln!(
        "{MESSAGES} × {} KiB echoed over webrtc in {elapsed:.2?}",
        MESSAGE_LEN / 1024
    );
    let m = a.node.metrics().await.unwrap();
    assert_eq!(m.links.len(), 1);
    assert_eq!(m.links[0].kind, LinkKind::WebRtc);
    assert_eq!(m.vpn.link_queue_full, 0);
    assert_page_echoed(page).await;
}

/// a offers WebRTC through b, and the browser answers.
#[tokio::test]
#[ignore = "launches headless Firefox"]
async fn browser_answers_native_offer() {
    let Some(ip) = lan_ip() else { return };
    let Some((_browser, mut pages)) = Browser::start(1).await else {
        return;
    };
    let mut b = Native::relay(2).await;
    let mut a = Native::webrtc(1, ip).await;
    let page = &mut pages[0];
    let p = join_page(page, 0, &mut b).await;
    a.join(&mut b).await;

    let started = Instant::now();
    assert!(a.node.connect_peer(a.net, p).await.unwrap());
    let pa = a.peer_added(p).await;
    assert_eq!((pa.kind, pa.dialed), (LinkKind::WebRtc, true));
    assert_eq!(page.peer_added(&hex_id(a.id())).await, webrtc_peer(false));
    assert_eq!(a.exchange_closed(p).await, ExchangeOutcome::Connected);
    eprintln!("webrtc link up in {:.2?}", started.elapsed());

    drop_relay(b, &mut [&mut a], &mut [&mut *page]).await;
    echo_from_native(&mut a, p, page).await;
    a.node.shutdown().await.unwrap();
}

/// The browser offers WebRTC to a through b, and a answers.
#[tokio::test]
#[ignore = "launches headless Firefox"]
async fn browser_offers_to_native() {
    let Some(ip) = lan_ip() else { return };
    let Some((_browser, mut pages)) = Browser::start(1).await else {
        return;
    };
    let mut b = Native::relay(2).await;
    let mut a = Native::webrtc(1, ip).await;
    let page = &mut pages[0];
    let p = join_page(page, 0, &mut b).await;
    a.join(&mut b).await;

    let started = Instant::now();
    let a_hex = hex_id(a.id());
    assert!(page.connect_peer(&a_hex).await);
    assert_eq!(page.peer_added(&a_hex).await, webrtc_peer(true));
    let pa = a.peer_added(p).await;
    assert_eq!((pa.kind, pa.dialed), (LinkKind::WebRtc, false));
    assert_eq!(page.exchange_closed(&a_hex).await, "Connected");
    eprintln!("webrtc link up in {:.2?}", started.elapsed());

    drop_relay(b, &mut [&mut a], &mut [&mut *page]).await;
    echo_from_native(&mut a, p, page).await;
    a.node.shutdown().await.unwrap();
}

/// Two page nodes in one browser link over WebRTC: node 0 offers, node 1
/// answers, and node 1 echoes node 0's messages.
#[tokio::test]
#[ignore = "launches headless Firefox"]
async fn browser_to_browser() {
    if lan_ip().is_none() {
        return;
    }
    let Some((_browser, mut pages)) = Browser::start(2).await else {
        return;
    };
    let mut b = Native::relay(2).await;
    let [p0, p1] = &mut pages[..] else {
        unreachable!()
    };
    let id0 = hex_id(join_page(p0, 0, &mut b).await);
    let id1 = hex_id(join_page(p1, 1, &mut b).await);

    let started = Instant::now();
    assert!(p0.connect_peer(&id1).await);
    assert_eq!(p0.peer_added(&id1).await, webrtc_peer(true));
    assert_eq!(p1.peer_added(&id0).await, webrtc_peer(false));
    assert_eq!(p0.exchange_closed(&id1).await, "Connected");
    eprintln!("webrtc link up in {:.2?}", started.elapsed());

    drop_relay(b, &mut [], &mut [&mut *p0, &mut *p1]).await;

    let started = Instant::now();
    let (mut sent, mut got) = (0, 0);
    while got < MESSAGES {
        while sent < MESSAGES && sent - got < WINDOW {
            p0.send_message(&id1, &body(sent)).await;
            sent += 1;
        }
        let left = ECHO_TIMEOUT.saturating_sub(started.elapsed());
        let Some(echo) = p0.try_message(&id1, left.min(STEP)).await else {
            let (s0, s1) = (p0.stats().await, p1.stats().await);
            panic!("timed out waiting for echo {got} ({sent} sent); node 0 {s0}; node 1 {s1}");
        };
        check_echo(got, &echo);
        got += 1;
    }
    eprintln!(
        "{MESSAGES} × {} KiB echoed over webrtc in {:.2?}",
        MESSAGE_LEN / 1024,
        started.elapsed()
    );
    let s0 = p0.stats().await;
    assert_eq!(s0["links"].as_array().unwrap().len(), 1, "{s0}");
    assert_eq!(s0["linkQueueFull"], 0, "{s0}");
    assert_page_echoed(p1).await;
}
