// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Services on localhost driven only through their frontend API, over real
//! WebSockets, as the UI drives them: profiles sign up, create and join a
//! network through invitations whose bootstrap clients link the nodes,
//! renew their certificates, find a chat room in the directory and chat;
//! a network's CA publishes it to a peer that accepts it; and a member
//! reads another's mock stream.
//!
//! Each step prints how long it took.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::time::Duration as StdDuration;

use futures_util::StreamExt;
use prost::Message as _;
use strims_crypto::{CertificateChain, certificate_body, membership_trusted};
use strims_node::{Clock, SystemClock};
use strims_proto::auth::v1::{AuthFrontendClient, SignUpRequest};
use strims_proto::chat::v1::open_client_response::Body as OpenBody;
use strims_proto::chat::v1::{
    ChatFrontendClient, ChatServerFrontendClient, ClientSendMessageRequest,
    CreateServerRequest as CreateChatServerRequest, Message, OpenClientRequest, OpenClientResponse,
    Room, server_event,
};
use strims_proto::debug::v1 as debug;
use strims_proto::network::v1::bootstrap::{
    BootstrapClientWebSocketOptions, BootstrapFrontendClient, Config as BootstrapConfig,
    CreateBootstrapClientRequest, ListBootstrapPeersRequest, PublishNetworkToBootstrapPeerRequest,
    SetConfigRequest, create_bootstrap_client_request,
};
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::directory::listing::Content;
use strims_proto::network::v1::directory::{
    DirectoryFrontendClient, FrontendGetListingsRequest, ListingContentType,
};
use strims_proto::network::v1::network_event::Body as NetworkBody;
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateNetworkFromInvitationRequest, CreateServerRequest,
    ListNetworksRequest, Network, NetworkFrontendClient, WatchNetworksRequest,
};
use strims_proto::service::CallError;
use strims_rpc::{Client, ClientStream};
use strims_svc::Svc;
use support::*;

const STEP: StdDuration = StdDuration::from_secs(60);
/// A step that goes through a relaying member.
const RELAYED_STEP: StdDuration = StdDuration::from_secs(90);

/// A profile signed up on its own service.
struct Profile {
    name: &'static str,
    svc: Svc,
    ui: Ui,
    key: Vec<u8>,
}

impl Profile {
    async fn sign_up(name: &'static str) -> Profile {
        let svc = service().await;
        let ui = connect_ui(&svc).await;
        let res = AuthFrontendClient::new(ui.client.clone())
            .sign_up(&SignUpRequest {
                name: name.into(),
                password: "pw".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        let key = res.profile.unwrap().key.unwrap().public;
        Profile { name, svc, ui, key }
    }

    fn client(&self) -> Client {
        self.ui.client.clone()
    }

    fn networks(&self) -> NetworkFrontendClient<Client> {
        NetworkFrontendClient::new(self.client())
    }

    fn bootstrap(&self) -> BootstrapFrontendClient<Client> {
        BootstrapFrontendClient::new(self.client())
    }

    fn debug(&self) -> debug::DebugClient<Client> {
        debug::DebugClient::new(self.client())
    }

    async fn debug_metrics(&self) -> Option<debug::DebugMetrics> {
        let res = self
            .debug()
            .read_metrics(&debug::ReadMetricsRequest {})
            .await
            .ok()?;
        res.metrics?.debug
    }

    /// The URI other nodes link to this profile's node at.
    fn ws_uri(&self) -> String {
        let uri = self.svc.node_info().unwrap().ws_uri.unwrap();
        let suffix = format!("/{}", data_encoding::HEXLOWER.encode(&self.key));
        assert!(
            uri.starts_with("ws://127.0.0.1:") && uri.ends_with(&suffix),
            "{uri}"
        );
        uri
    }

    /// Adds a bootstrap client for `uri`; returns its id.
    async fn add_bootstrap_client(&self, uri: &str) -> u64 {
        self.bootstrap()
            .create_client(&CreateBootstrapClientRequest {
                client_options: Some(
                    create_bootstrap_client_request::ClientOptions::WebsocketOptions(
                        BootstrapClientWebSocketOptions {
                            url: uri.into(),
                            insecure_skip_verify_tls: false,
                        },
                    ),
                ),
            })
            .await
            .unwrap()
            .bootstrap_client
            .unwrap()
            .id
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

    /// An invitation to the network `id` that links its holder to this
    /// profile's node.
    async fn invite(&self, id: u64) -> Vec<u8> {
        let client = self.add_bootstrap_client(&self.ws_uri()).await;
        self.networks()
            .create_invitation(&CreateInvitationRequest {
                network_id: id,
                bootstrap_client_id: client,
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
                .and_then(|c| CertificateChain::decode(c).ok())
                .is_some_and(|c| c.network_key() == key)
        })
    }

    /// Waits until the profile's membership of the network `key` is signed
    /// by the network's root.
    async fn wait_trusted(&self, key: &[u8], limit: StdDuration) -> Network {
        eventually(
            &format!("{}'s renewed membership", self.name),
            limit,
            || async {
                let n = self.network(key).await?;
                let cert = n.certificate.as_ref()?;
                membership_trusted(cert, SystemClock.now()).then_some(n.clone())
            },
        )
        .await
    }

    /// Waits for a chat room listed in the network `key`'s directory;
    /// returns its server key.
    async fn find_room(&self, key: &[u8], name: &str, limit: StdDuration) -> Vec<u8> {
        let directory = DirectoryFrontendClient::new(self.client());
        eventually(
            &format!("{}: the room in the directory", self.name),
            limit,
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
        let chat = ChatFrontendClient::new(self.client());
        let mut stream = chat
            .open_client(&OpenClientRequest {
                network_key: net.to_vec(),
                server_key: room.to_vec(),
            })
            .await
            .unwrap();
        let first = tokio::time::timeout(STEP, stream.next()).await;
        match first {
            Ok(Some(Ok(r))) if r.body == Some(OpenBody::Open(Default::default())) => {}
            r => panic!("{}: the room didn't open: {r:?}", self.name),
        }
        stream
    }

    /// Sends `body` to the room, retrying while the room's server isn't
    /// reachable yet.
    async fn say(&self, net: &[u8], room: &[u8], body: &str, limit: StdDuration) {
        let chat = ChatFrontendClient::new(self.client());
        eventually(&format!("{} to send {body:?}", self.name), limit, || {
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

fn messages(r: &OpenClientResponse) -> Vec<Message> {
    match &r.body {
        Some(OpenBody::ServerEvents(e)) => e
            .events
            .iter()
            .filter_map(|e| {
                e.body
                    .as_ref()
                    .map(|server_event::Body::Message(m)| m.clone())
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Reads `stream` until every `(nick, body)` in `want` has arrived.
async fn expect_messages(
    who: &str,
    stream: &mut ClientStream<OpenClientResponse>,
    want: &[(&str, &str)],
    limit: StdDuration,
) {
    let deadline = tokio::time::Instant::now() + limit;
    let mut missing: Vec<(&str, &str)> = want.to_vec();
    let mut seen = Vec::new();
    while !missing.is_empty() {
        match tokio::time::timeout_at(deadline, stream.next()).await {
            Ok(Some(Ok(r))) => {
                for m in messages(&r) {
                    missing.retain(|(nick, body)| !(m.nick == *nick && m.body == *body));
                    seen.push((m.nick, m.body));
                }
            }
            r => panic!("{who}: missing {missing:?}; saw {seen:?}; then {r:?}"),
        }
    }
}

/// Times the steps of a scenario.
struct Steps(tokio::time::Instant);

impl Steps {
    fn done(&mut self, step: &str) {
        let now = tokio::time::Instant::now();
        eprintln!("{step}: {:?}", now - self.0);
        self.0 = now;
    }
}

#[test]
fn three_profiles_chat_across_local_nodes() {
    run(async {
        tokio::time::timeout(StdDuration::from_secs(300), async {
            let mut steps = Steps(tokio::time::Instant::now());
            let alice = Profile::sign_up("alice").await;
            let bob = Profile::sign_up("bob").await;
            steps.done("alice and bob sign up");

            // Alice's invitation links bob to her node, and her own client
            // for it isn't dialed.
            let network = alice.create_network("lobby-net").await;
            let key = network_key(network.certificate.as_ref().unwrap());
            let invitation = alice.invite(network.id).await;
            let joined = bob.join(invitation).await;
            assert!(!membership_trusted(
                joined.certificate.as_ref().unwrap(),
                SystemClock.now()
            ));
            let renewed = bob.wait_trusted(&key, STEP).await;
            let membership = certificate_body(renewed.certificate.as_ref().unwrap()).unwrap();
            assert_eq!(membership.subject, "bob");
            steps.done("bob joins from alice's invitation and renews");

            let room = ChatServerFrontendClient::new(alice.client())
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
            assert_eq!(bob.find_room(&key, "lobby", STEP).await, room_key);
            steps.done("bob finds alice's room");

            let mut at_alice = alice.open_room(&key, &room_key).await;
            let mut at_bob = bob.open_room(&key, &room_key).await;
            alice.say(&key, &room_key, "hi bob", STEP).await;
            bob.say(&key, &room_key, "hey alice", STEP).await;
            let both = [("alice", "hi bob"), ("bob", "hey alice")];
            expect_messages("alice", &mut at_alice, &both, STEP).await;
            expect_messages("bob", &mut at_bob, &both, STEP).await;
            steps.done("alice and bob chat");

            // Bob invites carol, who reaches the network through him.
            let carol = Profile::sign_up("carol").await;
            let invitation = bob.invite(renewed.id).await;
            carol.join(invitation).await;
            carol.wait_trusted(&key, RELAYED_STEP).await;
            steps.done("carol joins from bob's invitation and renews");
            assert_eq!(carol.find_room(&key, "lobby", RELAYED_STEP).await, room_key);
            steps.done("carol finds the room");

            let mut at_carol = carol.open_room(&key, &room_key).await;
            carol.say(&key, &room_key, "hello both", RELAYED_STEP).await;
            alice.say(&key, &room_key, "welcome carol", STEP).await;
            let all = [("carol", "hello both"), ("alice", "welcome carol")];
            expect_messages("alice", &mut at_alice, &all, STEP).await;
            expect_messages("bob", &mut at_bob, &all, STEP).await;
            expect_messages("carol", &mut at_carol, &all, STEP).await;
            steps.done("all three chat");

            for p in [alice, bob, carol] {
                p.svc.stop().await;
            }
        })
        .await
        .expect("the scenario took over 300 s");
    });
}

#[test]
fn publishing_a_network_to_a_bootstrap_peer() {
    run(async {
        tokio::time::timeout(StdDuration::from_secs(120), async {
            let mut steps = Steps(tokio::time::Instant::now());
            let publisher = Profile::sign_up("publisher").await;
            let receiver = Profile::sign_up("receiver").await;
            receiver
                .bootstrap()
                .set_config(&SetConfigRequest {
                    config: Some(BootstrapConfig {
                        enable_publishing: true,
                    }),
                })
                .await
                .unwrap();
            receiver.add_bootstrap_client(&publisher.ws_uri()).await;
            let network = publisher.create_network("published").await;
            let key = network_key(network.certificate.as_ref().unwrap());

            let label = data_encoding::HEXLOWER.encode(&receiver.key);
            let peer = eventually("the receiver as a bootstrap peer", STEP, || async {
                let peers = publisher
                    .bootstrap()
                    .list_peers(&ListBootstrapPeersRequest {})
                    .await
                    .ok()?
                    .peers;
                peers.into_iter().find(|p| p.label == label)
            })
            .await;
            steps.done("the receiver links and accepts publishing");

            let mut watch = receiver
                .networks()
                .watch(&WatchNetworksRequest {})
                .await
                .unwrap();
            publisher
                .bootstrap()
                .publish_network_to_peer(&PublishNetworkToBootstrapPeerRequest {
                    peer_id: peer.peer_id,
                    network_id: network.id,
                })
                .await
                .unwrap();
            let published = receiver.network(&key).await.unwrap();
            let cert = published.certificate.as_ref().unwrap();
            assert_eq!(certificate_body(cert).unwrap().key, receiver.key);
            assert!(membership_trusted(cert, SystemClock.now()));
            assert!(published.server_config.is_none());
            steps.done("the publisher publishes the network");

            let deadline = tokio::time::Instant::now() + STEP;
            loop {
                let r: Result<_, CallError> =
                    match tokio::time::timeout_at(deadline, watch.next()).await {
                        Ok(Some(r)) => r,
                        r => panic!("the published network bound no peer: {r:?}"),
                    };
                let body = r.unwrap().event.and_then(|e| e.body);
                if let Some(NetworkBody::NetworkPeerCountUpdate(u)) = body
                    && u.network_id == published.id
                    && u.peer_count >= 1
                {
                    break;
                }
            }
            steps.done("the receiver binds the publisher on the network");
            publisher.svc.stop().await;
            receiver.svc.stop().await;
        })
        .await
        .expect("the scenario took over 120 s");
    });
}

#[test]
fn a_mock_stream_crosses_local_services() {
    run(async {
        tokio::time::timeout(StdDuration::from_secs(180), async {
            let mut steps = Steps(tokio::time::Instant::now());
            let alice = Profile::sign_up("alice").await;
            let bob = Profile::sign_up("bob").await;
            let network = alice.create_network("mock-net").await;
            let key = network_key(network.certificate.as_ref().unwrap());
            let invitation = alice.invite(network.id).await;
            bob.join(invitation).await;
            bob.wait_trusted(&key, STEP).await;
            steps.done("bob joins alice's network");

            bob.debug()
                .set_config(&debug::SetConfigRequest {
                    config: Some(debug::Config {
                        enable_mock_streams: true,
                        mock_stream_network_key: key.clone(),
                    }),
                })
                .await
                .unwrap();
            let id = alice
                .debug()
                .start_mock_stream(&debug::StartMockStreamRequest {
                    bitrate_kbps: 1000,
                    segment_interval_ms: 500,
                    timeout_ms: 0,
                    network_key: key.clone(),
                })
                .await
                .unwrap()
                .id;
            let listed = eventually("alice's mock stream listed", STEP, || async {
                let streams = alice
                    .debug()
                    .list_mock_streams(&debug::ListMockStreamsRequest {})
                    .await
                    .ok()?
                    .mock_streams;
                streams
                    .into_iter()
                    .find(|s| s.id == id && s.listing_id.is_some())
            })
            .await;
            assert_eq!(listed.network_key, key);
            steps.done("alice lists a mock stream");

            let reader = eventually("bob reading alice's mock stream", STEP, || async {
                let readers = bob.debug_metrics().await?.readers;
                readers
                    .into_iter()
                    .find(|r| r.id == Some(id) && r.segments >= 3 && r.latency.is_some())
            })
            .await;
            assert_eq!(reader.network_key, key);
            assert_eq!(reader.malformed, 0);
            steps.done("bob reads it with its latency");

            alice
                .debug()
                .stop_mock_stream(&debug::StopMockStreamRequest { id })
                .await
                .unwrap();
            eventually("bob's reader to end", STEP, || async {
                bob.debug_metrics().await?.readers.is_empty().then_some(())
            })
            .await;
            assert!(alice.debug_metrics().await.unwrap().mock_streams.is_empty());
            steps.done("alice stops it and bob's reader ends");

            alice.svc.stop().await;
            bob.svc.stop().await;
        })
        .await
        .expect("the scenario took over 180 s");
    });
}
