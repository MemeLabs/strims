// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A browser profile and a native profile on one network, driven only
//! through the UI's frontend services, as the UI drives them.
//!
//! alice signs up on an in-process strims-svc, whose node listens for
//! WebSocket links on 127.0.0.1; bob signs up in the browser, in
//! test.html's worker, whose frontend connection the page relays to the
//! harness's `/manage` socket. Then
//!
//! - bob adds a bootstrap client for alice's node, so his node dials it,
//!   and both list the other as a bootstrap peer;
//! - bob joins alice's network from her invitation, renews his membership
//!   at her CA, and both count a peer on the network;
//! - alice opens a chat room, bob finds it in the network's directory,
//!   and a message from each reaches both;
//! - Firefox is killed and started again on the same profile: bob signs
//!   in with the profile key his sign-up returned, finds his profile, his
//!   network and his bootstrap client in IndexedDB, links to alice again
//!   and chats.
//!
//! Each step has a time limit and prints how long it took. A failure
//! prints Firefox's output, the page's console included.
//!
//! Ignored by default (it launches a browser):
//!
//! ```sh
//! cargo test -p strims-wasm --test e2e_app -- --ignored
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

use std::future::{Future, ready};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use strims_crypto::{certificate_network_key, membership_trusted};
use strims_node::app::AppConfig;
use strims_node::{Clock, SystemClock};
use strims_proto::auth::v1::{
    AuthFrontendClient, SignInRequest, SignUpRequest, linked_profile, sign_in_request,
};
use strims_proto::chat::v1::open_client_response::Body as OpenBody;
use strims_proto::chat::v1::{
    ChatFrontendClient, ChatServerFrontendClient, ClientSendMessageRequest,
    CreateServerRequest as CreateChatServerRequest, OpenClientRequest, OpenClientResponse, Room,
    server_event,
};
use strims_proto::network::v1::bootstrap::{
    BootstrapClientWebSocketOptions, BootstrapFrontendClient, Config as BootstrapConfig,
    CreateBootstrapClientRequest, ListBootstrapClientsRequest, ListBootstrapPeersRequest,
    SetConfigRequest, bootstrap_client, create_bootstrap_client_request,
};
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::directory::listing::Content;
use strims_proto::network::v1::directory::{
    DirectoryFrontendClient, FrontendGetListingsRequest, ListingContentType,
};
use strims_proto::network::v1::network_event::Body as NetworkBody;
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateNetworkFromInvitationRequest, CreateServerRequest,
    ListNetworksRequest, ListPeersRequest, Network, NetworkFrontendClient, WatchNetworksRequest,
};
use strims_rpc::{Client, ClientStream, Config as RpcConfig, connect};
use strims_svc::{Config, Db, Svc};
use support::browser::{Browser, PAGE_TIMEOUT, Socket};
use tokio::task::{LocalSet, spawn_local};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::{self, Message};

/// The limit of one wait or call.
const STEP: Duration = Duration::from_secs(60);
/// The limit of one step of the scenario, which waits several times.
const STEP_LIMIT: Duration = Duration::from_secs(180);

/// `f`, or a panic naming `what` after `limit`.
async fn within<F: Future>(what: &str, limit: Duration, f: F) -> F::Output {
    match tokio::time::timeout(limit, f).await {
        Ok(v) => v,
        Err(_) => panic!("timed out after {limit:?} waiting for {what}"),
    }
}

/// Calls `f` every 250 ms until it returns `Some`, for at most `limit`.
async fn eventually<T, Fut: Future<Output = Option<T>>>(
    what: &str,
    limit: Duration,
    mut f: impl FnMut() -> Fut,
) -> T {
    let deadline = Instant::now() + limit;
    loop {
        if let Some(v) = f().await {
            return v;
        }
        if Instant::now() >= deadline {
            panic!("timed out after {limit:?} waiting for {what}");
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// A frontend RPC client over `ws`, one Call message per binary frame.
/// The connection runs as a local task until the socket closes.
fn client_over<S>(ws: tokio_tungstenite::WebSocketStream<S>) -> Client
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + 'static,
{
    let (sink, stream) = ws.split();
    let reader = stream.filter_map(|m| {
        ready(match m {
            Ok(Message::Binary(b)) => Some(Ok(b)),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    });
    let writer =
        sink.with(|b: Vec<u8>| ready(Ok::<_, tungstenite::Error>(Message::Binary(b.into()))));
    let (client, conn) = connect(
        RpcConfig::frontend(),
        Box::pin(reader),
        Box::pin(writer),
        None,
    );
    spawn_local(async move {
        let _ = conn.await;
    });
    client
}

/// The UI's connection to the browser's node: the next `/manage` socket
/// the page opens.
async fn connect_page(browser: &mut Browser) -> Client {
    let ws: Socket = browser.socket("/manage", PAGE_TIMEOUT).await;
    client_over(ws)
}

/// The native service: an in-memory store, a node that accepts WebSocket
/// links on 127.0.0.1, and the API on 127.0.0.1.
async fn native_service() -> Svc {
    Svc::start(Config {
        api_listen: "127.0.0.1:0".into(),
        db: Db::Memory,
        app: AppConfig {
            ws_listen: Some("127.0.0.1:0".into()),
            allow_insecure_ws: true,
            ..AppConfig::default()
        },
        ..Config::default()
    })
    .await
    .unwrap()
}

/// The UI's connection to `svc`'s API.
async fn connect_native(svc: &Svc) -> Client {
    let url = format!("ws://{}/api", svc.api_addr());
    let max = RpcConfig::frontend().max_message_bytes + 16;
    let config = WebSocketConfig::default()
        .max_message_size(Some(max))
        .max_frame_size(Some(max));
    let (ws, _) = tokio_tungstenite::connect_async_with_config(url, Some(config), true)
        .await
        .unwrap();
    client_over(ws)
}

/// A profile, as the UI sees it through its frontend connection.
struct Profile {
    name: &'static str,
    client: Client,
    /// The profile's public key.
    key: Vec<u8>,
}

/// What a sign-up returned: the profile, and the credentials it signs in
/// with again.
struct SignedUp {
    profile: Profile,
    credentials: Option<linked_profile::Credentials>,
}

impl Profile {
    async fn sign_up(name: &'static str, client: Client, persist_login: bool) -> SignedUp {
        let res = within(
            &format!("{name}'s sign-up"),
            STEP,
            AuthFrontendClient::new(client.clone()).sign_up(&SignUpRequest {
                name: name.into(),
                password: "pw".into(),
                persist_session: false,
                persist_login,
            }),
        )
        .await
        .unwrap_or_else(|e| panic!("{name}'s sign-up failed: {e:?}"));
        let profile = res.profile.unwrap();
        assert_eq!(profile.name, name);
        SignedUp {
            profile: Profile {
                name,
                client,
                key: profile.key.unwrap().public,
            },
            credentials: res.linked_profile.and_then(|l| l.credentials),
        }
    }

    fn networks(&self) -> NetworkFrontendClient<Client> {
        NetworkFrontendClient::new(self.client.clone())
    }

    fn bootstrap(&self) -> BootstrapFrontendClient<Client> {
        BootstrapFrontendClient::new(self.client.clone())
    }

    fn hex_key(&self) -> String {
        hex::encode(&self.key)
    }

    /// Lets peers publish networks to this profile, so the bootstrap
    /// peers it links to list it.
    async fn accept_publishing(&self) {
        self.bootstrap()
            .set_config(&SetConfigRequest {
                config: Some(BootstrapConfig {
                    enable_publishing: true,
                }),
            })
            .await
            .unwrap();
    }

    /// Adds a bootstrap client for `url`.
    async fn add_bootstrap_client(&self, url: &str) {
        self.bootstrap()
            .create_client(&CreateBootstrapClientRequest {
                client_options: Some(
                    create_bootstrap_client_request::ClientOptions::WebsocketOptions(
                        BootstrapClientWebSocketOptions {
                            url: url.into(),
                            insecure_skip_verify_tls: false,
                        },
                    ),
                ),
            })
            .await
            .unwrap();
    }

    /// The URLs of the profile's bootstrap clients.
    async fn bootstrap_urls(&self) -> Vec<String> {
        let res = self
            .bootstrap()
            .list_clients(&ListBootstrapClientsRequest {})
            .await
            .unwrap();
        res.bootstrap_clients
            .into_iter()
            .filter_map(|c| match c.client_options? {
                bootstrap_client::ClientOptions::WebsocketOptions(o) => Some(o.url),
            })
            .collect()
    }

    /// Waits until `other` is linked to this profile's node as a bootstrap
    /// peer.
    async fn wait_bootstrap_peer(&self, other: &Profile) {
        let label = other.hex_key();
        eventually(
            &format!("{} as {}'s bootstrap peer", other.name, self.name),
            STEP,
            || async {
                let peers = self
                    .bootstrap()
                    .list_peers(&ListBootstrapPeersRequest {})
                    .await
                    .ok()?
                    .peers;
                peers.iter().any(|p| p.label == label).then_some(())
            },
        )
        .await;
    }

    /// Waits until `other` is no longer a bootstrap peer.
    async fn wait_bootstrap_peer_gone(&self, other: &Profile) {
        let label = other.hex_key();
        eventually(
            &format!("{} to leave {}'s bootstrap peers", other.name, self.name),
            STEP,
            || async {
                let peers = self
                    .bootstrap()
                    .list_peers(&ListBootstrapPeersRequest {})
                    .await
                    .ok()?
                    .peers;
                (!peers.iter().any(|p| p.label == label)).then_some(())
            },
        )
        .await;
    }

    async fn create_network(&self, name: &str) -> Network {
        self.networks()
            .create_server(&CreateServerRequest {
                name: name.into(),
                ..Default::default()
            })
            .await
            .unwrap()
            .network
            .unwrap()
    }

    /// An invitation to the network `id`, with no bootstrap client.
    async fn invite(&self, id: u64) -> Vec<u8> {
        self.networks()
            .create_invitation(&CreateInvitationRequest {
                network_id: id,
                bootstrap_client_id: 0,
            })
            .await
            .unwrap()
            .invitation
            .unwrap()
            .encode_to_vec()
    }

    async fn join(&self, invitation: Vec<u8>) -> Network {
        self.networks()
            .create_network_from_invitation(&CreateNetworkFromInvitationRequest {
                alias: String::new(),
                invitation: Some(InvitationField::InvitationBytes(invitation)),
            })
            .await
            .unwrap()
            .network
            .unwrap()
    }

    /// The profile's record of the network whose key is `key`.
    async fn network(&self, key: &[u8]) -> Option<Network> {
        let all = self
            .networks()
            .list(&ListNetworksRequest {})
            .await
            .ok()?
            .networks;
        all.into_iter().find(|n| {
            n.certificate
                .as_ref()
                .is_some_and(|c| certificate_network_key(c) == key)
        })
    }

    /// Waits until the profile's membership of the network `key` is signed
    /// by the network's root.
    async fn wait_trusted(&self, key: &[u8]) -> Network {
        eventually(
            &format!("{}'s membership signed by the network", self.name),
            STEP,
            || async {
                let n = self.network(key).await?;
                let cert = n.certificate.as_ref()?;
                membership_trusted(cert, SystemClock.now()).then_some(n.clone())
            },
        )
        .await
    }

    /// Waits until the CA's records of the network `id` hold `other`.
    async fn wait_ca_peer(&self, id: u64, other: &Profile) {
        eventually(
            &format!("{}'s CA record of {}", self.name, other.name),
            STEP,
            || async {
                let peers = self
                    .networks()
                    .list_peers(&ListPeersRequest { network_id: id })
                    .await
                    .ok()?
                    .peers;
                peers
                    .iter()
                    .any(|p| p.public_key == other.key && p.alias == other.name)
                    .then_some(())
            },
        )
        .await;
    }

    /// Waits until the node counts at least one peer on the network `id`,
    /// as the UI's network watch reports it.
    async fn wait_network_peer(&self, id: u64) {
        let mut watch = self
            .networks()
            .watch(&WatchNetworksRequest {})
            .await
            .unwrap();
        let mut count = None;
        within(&format!("a peer on {}'s network", self.name), STEP, async {
            while count.is_none_or(|c| c == 0) {
                let r = watch.next().await.expect("the watch ended").unwrap();
                match r.event.and_then(|e| e.body) {
                    Some(NetworkBody::NetworkStart(s))
                        if s.network.as_ref().is_some_and(|n| n.id == id) =>
                    {
                        count = Some(s.peer_count);
                    }
                    Some(NetworkBody::NetworkPeerCountUpdate(u)) if u.network_id == id => {
                        count = Some(u.peer_count);
                    }
                    _ => {}
                }
            }
        })
        .await;
    }

    /// Waits for a chat room named `name` in the network `key`'s
    /// directory; returns its server key.
    async fn find_room(&self, key: &[u8], name: &str) -> Vec<u8> {
        let directory = DirectoryFrontendClient::new(self.client.clone());
        eventually(
            &format!("{}: the room in the directory", self.name),
            STEP,
            || {
                let directory = directory.clone();
                async move {
                    let res = directory
                        .get_listings(&FrontendGetListingsRequest {
                            content_types: vec![ListingContentType::Chat as i32],
                            network_keys: vec![key.to_vec()],
                        })
                        .await
                        .ok()?;
                    res.listings
                        .iter()
                        .flat_map(|n| &n.listings)
                        .find_map(|item| match item.listing.as_ref()?.content.as_ref()? {
                            Content::Chat(c) if c.name == name => Some(c.key.clone()),
                            _ => None,
                        })
                }
            },
        )
        .await
    }

    /// Opens the room and waits for it to open.
    async fn open_room(&self, net: &[u8], room: &[u8]) -> ClientStream<OpenClientResponse> {
        let mut stream = ChatFrontendClient::new(self.client.clone())
            .open_client(&OpenClientRequest {
                network_key: net.to_vec(),
                server_key: room.to_vec(),
            })
            .await
            .unwrap();
        match tokio::time::timeout(STEP, stream.next()).await {
            Ok(Some(Ok(r))) if r.body == Some(OpenBody::Open(Default::default())) => {}
            r => panic!("{}: the room didn't open: {r:?}", self.name),
        }
        stream
    }

    /// Sends `body` to the room, retrying while its server isn't reachable
    /// yet.
    async fn say(&self, net: &[u8], room: &[u8], body: &str) {
        let chat = ChatFrontendClient::new(self.client.clone());
        eventually(&format!("{} to send {body:?}", self.name), STEP, || {
            let chat = chat.clone();
            async move {
                chat.client_send_message(&ClientSendMessageRequest {
                    network_key: net.to_vec(),
                    server_key: room.to_vec(),
                    body: body.into(),
                })
                .await
                .ok()
            }
        })
        .await;
    }
}

/// Reads `stream` until every `(nick, body)` in `want` has arrived.
async fn expect_messages(
    who: &str,
    stream: &mut ClientStream<OpenClientResponse>,
    want: &[(&str, &str)],
) {
    let deadline = tokio::time::Instant::now() + STEP;
    let mut missing: Vec<(&str, &str)> = want.to_vec();
    let mut seen = Vec::new();
    while !missing.is_empty() {
        match tokio::time::timeout_at(deadline, stream.next()).await {
            Ok(Some(Ok(r))) => {
                let Some(OpenBody::ServerEvents(e)) = &r.body else {
                    continue;
                };
                for server_event::Body::Message(m) in e.events.iter().filter_map(|e| e.body.clone())
                {
                    missing.retain(|(nick, body)| !(m.nick == *nick && m.body == *body));
                    seen.push((m.nick, m.body));
                }
            }
            r => panic!("{who}: missing {missing:?}; saw {seen:?}; then {r:?}"),
        }
    }
}

/// Runs the scenario's steps, each within [`STEP_LIMIT`], and times them.
struct Steps(Vec<(&'static str, Duration)>);

impl Steps {
    /// `f`, or a panic naming `step` after [`STEP_LIMIT`].
    async fn run<T>(&mut self, step: &'static str, f: impl Future<Output = T>) -> T {
        let started = Instant::now();
        let v = within(&format!("the step {step:?}"), STEP_LIMIT, f).await;
        let took = started.elapsed();
        eprintln!("{step}: {took:.2?}");
        self.0.push((step, took));
        v
    }

    fn summary(&self) {
        let total: Duration = self.0.iter().map(|(_, d)| *d).sum();
        eprintln!("--- steps ({total:.2?} in all) ---");
        for (step, took) in &self.0 {
            eprintln!("{took:>10.2?}  {step}");
        }
    }
}

#[tokio::test]
#[ignore = "launches headless Firefox"]
async fn a_browser_profile_joins_a_native_network_chats_and_persists() {
    LocalSet::new()
        .run_until(async {
            let Some(mut browser) = Browser::serve().await else {
                return;
            };
            // The browser outlives the steps, so a failed step prints its
            // output.
            scenario(&mut browser).await;
        })
        .await;
}

async fn scenario(browser: &mut Browser) {
    let mut steps = Steps(Vec::new());
    let page = format!(
        "/test.html?manage={}/manage&allowInsecureWs=true&logLevel=0",
        browser.ws_origin()
    );

    let svc = native_service().await;
    let (alice, alice_uri) = steps
        .run("alice signs up on the native service", async {
            let alice = Profile::sign_up("alice", connect_native(&svc).await, false).await;
            let uri = svc.node_info().unwrap().ws_uri.unwrap();
            assert!(
                uri.starts_with("ws://127.0.0.1:")
                    && uri.ends_with(&format!("/{}", alice.profile.hex_key())),
                "{uri}"
            );
            (alice.profile, uri)
        })
        .await;

    let client = steps
        .run("the page loads and opens its frontend connection", async {
            browser.open(&page);
            connect_page(browser).await
        })
        .await;
    let (bob, bob_key) = steps
        .run("bob signs up in the browser", async {
            let bob = Profile::sign_up("bob", client, true).await;
            let Some(linked_profile::Credentials::Key(key)) = bob.credentials else {
                panic!("a sign-up in the browser gets the profile key");
            };
            (bob.profile, key)
        })
        .await;

    steps
        .run("bob's bootstrap client links his node to alice's", async {
            alice.accept_publishing().await;
            bob.accept_publishing().await;
            bob.add_bootstrap_client(&alice_uri).await;
            alice.wait_bootstrap_peer(&bob).await;
            bob.wait_bootstrap_peer(&alice).await;
        })
        .await;

    let (network, key, joined) = steps
        .run("bob joins alice's network and renews at her CA", async {
            let network = alice.create_network("lobby-net").await;
            let key = certificate_network_key(network.certificate.as_ref().unwrap()).to_vec();
            let joined = bob.join(alice.invite(network.id).await).await;
            assert!(!membership_trusted(
                joined.certificate.as_ref().unwrap(),
                SystemClock.now()
            ));
            let renewed = bob.wait_trusted(&key).await;
            assert_eq!(renewed.id, joined.id);
            assert_eq!(renewed.certificate.unwrap().subject, "bob");
            alice.wait_ca_peer(network.id, &bob).await;
            (network, key, joined)
        })
        .await;
    steps
        .run("both count a peer on the network", async {
            alice.wait_network_peer(network.id).await;
            bob.wait_network_peer(joined.id).await;
        })
        .await;

    let room_key = steps
        .run("bob finds alice's room in the directory", async {
            let room = ChatServerFrontendClient::new(alice.client.clone())
                .create_server(&CreateChatServerRequest {
                    network_key: key.clone(),
                    room: Some(Room {
                        name: "lobby".into(),
                        css: String::new(),
                    }),
                })
                .await
                .unwrap()
                .server
                .unwrap();
            let room_key = room.key.unwrap().public;
            assert_eq!(bob.find_room(&key, "lobby").await, room_key);
            room_key
        })
        .await;

    let (mut at_alice, at_bob) = steps
        .run("both open the room", async {
            let at_alice = alice.open_room(&key, &room_key).await;
            let at_bob = bob.open_room(&key, &room_key).await;
            (at_alice, at_bob)
        })
        .await;
    steps
        .run("alice and bob chat both ways", async {
            let mut at_bob = at_bob;
            alice.say(&key, &room_key, "hi bob").await;
            bob.say(&key, &room_key, "hey alice").await;
            let both = [("alice", "hi bob"), ("bob", "hey alice")];
            expect_messages("alice", &mut at_alice, &both).await;
            expect_messages("bob", &mut at_bob, &both).await;
        })
        .await;

    // Killing Firefox stops bob's node, and alice sees his link go.
    steps
        .run("the browser closes and alice loses bob's link", async {
            browser.close();
            alice.wait_bootstrap_peer_gone(&bob).await;
        })
        .await;

    let client = steps
        .run("the page loads again on the same Firefox profile", async {
            browser.open(&page);
            connect_page(browser).await
        })
        .await;
    let bob = steps
        .run("bob signs in to his stored profile and network", async {
            let sign_in = SignInRequest {
                credentials: Some(sign_in_request::Credentials::Key(sign_in_request::Key {
                    profile_id: bob_key.profile_id,
                    profile_key: bob_key.profile_key.clone(),
                })),
            };
            let profile = AuthFrontendClient::new(client.clone())
                .sign_in(&sign_in)
                .await
                .unwrap_or_else(|e| panic!("bob's sign-in after the reload failed: {e:?}"))
                .profile
                .unwrap();
            assert_eq!(
                (profile.id, profile.name.as_str()),
                (bob_key.profile_id, "bob")
            );
            let bob = Profile {
                name: "bob",
                client,
                key: profile.key.unwrap().public,
            };
            let kept = bob
                .network(&key)
                .await
                .expect("bob's network survives the reload");
            assert_eq!(kept.id, joined.id);
            assert!(membership_trusted(
                kept.certificate.as_ref().unwrap(),
                SystemClock.now()
            ));
            assert_eq!(bob.bootstrap_urls().await, vec![alice_uri.clone()]);
            bob
        })
        .await;

    steps
        .run(
            "bob's node links to alice's again and binds the network",
            async {
                alice.wait_bootstrap_peer(&bob).await;
                bob.wait_network_peer(joined.id).await;
            },
        )
        .await;
    steps
        .run("bob chats again after the reload", async {
            let mut at_bob = bob.open_room(&key, &room_key).await;
            bob.say(&key, &room_key, "back again").await;
            let back = [("bob", "back again")];
            expect_messages("alice", &mut at_alice, &back).await;
            expect_messages("bob", &mut at_bob, &back).await;
        })
        .await;

    assert_eq!(
        browser.unclaimed(),
        Vec::<String>::new(),
        "sockets of pages the test didn't open"
    );
    drop(at_alice);
    browser.close();
    svc.stop().await;
    steps.summary();
}
