// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The bootstrap API of a service whose headless profile owns a network,
//! driven as a browser drives it, over TLS.
//!
//! A profile signs up, creates a network it serves the API for and one it
//! doesn't, and a video channel with a thumbnail on the first; then the
//! service restarts with the profile headless. Its node runs before any UI
//! signs in, and a UI's sign-in joins the same session. A synthetic
//! publisher streams to the channel. The event stream, over HTTP/2 and
//! over HTTP/1.1, carries the channel's media listing with its thumbnail
//! as the API's path, never the other network's listing, and pings while
//! nothing changes; the path answers the thumbnail's bytes. The channel's
//! SVG logo, which the owner's own directory holds, is neither in the
//! stream nor served. Every answer carries `nosniff` and the API's
//! sandboxing content security policy, over both protocols. The API's
//! invitation dials the node's WebSocket listener, and more profiles than
//! an inviter's quota join through it and are renewed, each recorded as
//! the owner's invitee, the owner's quota unspent.
//!
//! Event streams take at most half the listener's connections: past them
//! a stream answers 503, and other requests are still answered.
//!
//! A service whose bootstrap network the profile doesn't own (one it
//! joined, or none it has), whose profile isn't in the store or can't sign
//! in, or whose API has no profile doesn't start.
//!
//! Each step prints how long it took.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration as StdDuration;

use bytes::Bytes;
use data_encoding::{BASE64URL_NOPAD, HEXLOWER};
use futures_util::future::join_all;
use prost::Message as _;
use strims_crypto::{CertificateChain, membership_trusted};
use strims_dao::schema::network::DEFAULT_INVITE_QUOTA;
use strims_node::app::AppConfig;
use strims_node::{Clock, SystemClock};
use strims_ppspp::{SwarmId, Uri};
use strims_proto::auth::v1::sign_in_request::{Credentials, Password};
use strims_proto::auth::v1::{AuthFrontendClient, SignInRequest, SignUpRequest};
use strims_proto::network::v1::bootstrap::bootstrap_client::ClientOptions;
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::directory::frontend_watch_listings_response::event::Event as WatchEvent;
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;
use strims_proto::network::v1::directory::{
    DirectoryFrontendClient, FrontendGetListingsRequest, FrontendPublishRequest,
    FrontendWatchListingsResponse, Listing, ListingContentType, ListingSnippet,
    ListingSnippetImage,
};
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateNetworkFromInvitationRequest, CreateServerRequest, Invitation,
    InvitationV0, ListNetworksRequest, ListPeersRequest, Network, NetworkFrontendClient,
};
use strims_proto::types::{Image, ImageType};
use strims_proto::video::v1::{
    VideoChannelCreateRequest, VideoChannelFrontendClient, VideoIngressClient, VideoIngressConfig,
    VideoIngressGetChannelUrlRequest, VideoIngressSetConfigRequest,
};
use strims_rpc::Client;
use strims_svc::{BootstrapConfig, Config, Db, ProfileLogin, Svc, SvcError, TlsConfig};
use strims_video::synth::script::{Script, ScriptConfig};
use strims_video::{ingress_options, units};
use support::rtmp::Publish;
use support::tls::{TestCert, connect, dir};
use support::video::*;
use support::*;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::task::spawn_local;
use tokio::time::Instant;

/// A 1x1 PNG.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae,
    0x42, 0x60, 0x82,
];

/// An SVG that runs script.
const SVG: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>";

/// The content security policy of the API's answers.
const POLICY: &str = "default-src 'none'; sandbox";

/// How many profiles join through the API's invitation: more than an
/// inviter's quota.
const VIEWERS: usize = DEFAULT_INVITE_QUOTA as usize + 1;

const OWNER: &str = "owner";
const PASSWORD: &str = "pw";

/// A port the OS picks, free when this returns.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// A store file of its own for test `name`.
fn db_file(name: &str) -> (PathBuf, PathBuf) {
    let dir = dir(name);
    let db = dir.join("strims.redb");
    (dir, db)
}

/// The channel's logo: [`SVG`].
fn svg_logo() -> ListingSnippetImage {
    ListingSnippetImage {
        source_oneof: Some(SourceOneof::Image(Image {
            r#type: ImageType::Svg as i32,
            height: 1,
            width: 1,
            data: SVG.to_vec(),
        })),
    }
}

/// What the owner's first run left in the store.
struct Owned {
    net: Vec<u8>,
    other: Vec<u8>,
    network_id: u64,
    channel_key: Vec<u8>,
    ingress_port: u16,
}

async fn sign_up(svc: &Svc, name: &str) -> Client {
    let ui = connect_ui(svc).await;
    AuthFrontendClient::new(ui.client.clone())
        .sign_up(&SignUpRequest {
            name: name.into(),
            password: PASSWORD.into(),
            ..Default::default()
        })
        .await
        .unwrap();
    ui.client
}

async fn create_network(client: &Client, name: &str) -> Network {
    NetworkFrontendClient::new(client.clone())
        .create_server(&CreateServerRequest {
            name: name.into(),
            ..Default::default()
        })
        .await
        .unwrap()
        .network
        .unwrap()
}

/// Signs the owner up on a service over `db`, with two networks and a
/// video channel on the first whose snippet has [`PNG`] as its thumbnail,
/// and the RTMP ingress on.
async fn first_run(db: &Path) -> Owned {
    let svc = Svc::start(Config {
        db: Db::File(db.to_owned()),
        ..local_config()
    })
    .await
    .unwrap();
    let client = sign_up(&svc, OWNER).await;
    let network = create_network(&client, "streams").await;
    let other = create_network(&client, "private").await;
    let net = network_key(network.certificate.as_ref().unwrap());
    let channel = VideoChannelFrontendClient::new(client.clone())
        .create(&VideoChannelCreateRequest {
            directory_listing_snippet: Some(ListingSnippet {
                title: "synthetic bars".into(),
                channel_name: "the owner's channel".into(),
                thumbnail: Some(ListingSnippetImage {
                    source_oneof: Some(SourceOneof::Image(Image {
                        r#type: ImageType::Png as i32,
                        height: 1,
                        width: 1,
                        data: PNG.to_vec(),
                    })),
                }),
                channel_logo: Some(svg_logo()),
                ..Default::default()
            }),
            network_key: net.clone(),
        })
        .await
        .unwrap()
        .channel
        .unwrap();
    let ingress_port = free_port();
    VideoIngressClient::new(client.clone())
        .set_config(&VideoIngressSetConfigRequest {
            config: Some(VideoIngressConfig {
                enabled: true,
                server_addr: format!("127.0.0.1:{ingress_port}"),
                ..Default::default()
            }),
        })
        .await
        .unwrap();
    svc.stop().await;
    Owned {
        net,
        other: network_key(other.certificate.as_ref().unwrap()),
        network_id: network.id,
        channel_key: channel.key.unwrap().public,
        ingress_port,
    }
}

/// The messages of an event stream, from its bytes as they come.
#[derive(Default)]
struct Frames {
    buf: Vec<u8>,
}

impl Frames {
    fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// The next whole message, if its bytes came.
    fn pop(&mut self) -> Option<FrontendWatchListingsResponse> {
        let len = prost::decode_length_delimiter(self.buf.as_slice()).ok()?;
        let at = prost::length_delimiter_len(len);
        if self.buf.len() < at + len {
            return None;
        }
        let msg = FrontendWatchListingsResponse::decode(&self.buf[at..at + len]).unwrap();
        self.buf.drain(..at + len);
        Some(msg)
    }
}

/// Where an event stream's bytes come from.
enum Source {
    H2(h2::RecvStream),
    H1(Box<dyn AsyncRead + Unpin>),
}

impl Source {
    async fn read(&mut self) -> Option<Bytes> {
        match self {
            Source::H2(body) => {
                let chunk = body.data().await?.unwrap();
                body.flow_control().release_capacity(chunk.len()).unwrap();
                Some(chunk)
            }
            Source::H1(r) => {
                let mut buf = vec![0u8; 16 * 1024];
                let n = r.read(&mut buf).await.unwrap();
                (n > 0).then(|| Bytes::copy_from_slice(&buf[..n]))
            }
        }
    }
}

/// An event stream: its messages as they come.
struct Events {
    source: Source,
    frames: Frames,
}

impl Events {
    async fn next(&mut self) -> FrontendWatchListingsResponse {
        loop {
            if let Some(msg) = self.frames.pop() {
                return msg;
            }
            let chunk = tokio::time::timeout(STEP, self.source.read())
                .await
                .expect("an event stream message")
                .expect("the event stream ended");
            self.frames.push(&chunk);
        }
    }

    /// Reads until a change lists the media listing of swarm `uri` with a
    /// thumbnail that is a URL: its id and the URL. Every change is of
    /// network `net`, no listing is `hidden`'s, and none has a logo.
    async fn media_thumbnail(&mut self, net: &[u8], uri: &str, hidden: &str) -> (u64, String) {
        loop {
            let msg = self.next().await;
            for e in msg.events {
                let Some(WatchEvent::Change(change)) = e.event else {
                    continue;
                };
                let listings = change.listings.unwrap();
                assert_eq!(listings.network.as_ref().unwrap().key, net);
                for item in listings.listings {
                    let Some(Content::Media(media)) = item.listing.unwrap().content else {
                        panic!("a listing other than media");
                    };
                    assert_ne!(media.swarm_uri, hidden, "the other network's listing");
                    if media.swarm_uri != uri {
                        continue;
                    }
                    let snippet = item.snippet.unwrap_or_default();
                    assert_eq!(snippet.channel_logo, None, "the SVG logo in the stream");
                    if let Some(SourceOneof::Url(url)) =
                        snippet.thumbnail.and_then(|t| t.source_oneof)
                    {
                        return (item.id, url);
                    }
                }
            }
        }
    }

    /// Reads until a ping, an empty message.
    async fn ping(&mut self) {
        while !self.next().await.events.is_empty() {}
    }
}

/// An event stream over HTTP/2, after checking its head.
async fn events_h2(cert: &TestCert, addr: std::net::SocketAddr) -> Events {
    let tls = connect(&cert.client(&[b"h2"]), addr, "localhost")
        .await
        .unwrap();
    let (send, conn) = h2::client::handshake(tls).await.unwrap();
    spawn_local(conn);
    let req = http::Request::get("https://localhost/api/v1/directory/events")
        .body(())
        .unwrap();
    let mut send = send.ready().await.unwrap();
    let (response, _) = send.send_request(req, true).unwrap();
    let (head, body) = response.await.unwrap().into_parts();
    assert_eq!(head.status, 200);
    let h = |n: &str| head.headers.get(n).map(|v| v.to_str().unwrap().to_owned());
    assert_eq!(
        h("content-type").as_deref(),
        Some("application/octet-stream")
    );
    assert_eq!(h("cache-control").as_deref(), Some("no-store"));
    assert_eq!(h("access-control-allow-origin").as_deref(), Some("*"));
    assert_eq!(h("x-content-type-options").as_deref(), Some("nosniff"));
    assert_eq!(h("content-security-policy").as_deref(), Some(POLICY));
    assert_eq!(h("content-length"), None);
    Events {
        source: Source::H2(body),
        frames: Frames::default(),
    }
}

/// An event stream over HTTP/1.1 and TLS, after checking its head.
async fn events_h1(cert: &TestCert, addr: std::net::SocketAddr) -> Events {
    let mut s = connect(&cert.client(&[b"http/1.1"]), addr, "localhost")
        .await
        .unwrap();
    s.write_all(b"GET /api/v1/directory/events HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();
    let mut raw = Vec::new();
    let mut byte = [0u8; 1];
    while !raw.ends_with(b"\r\n\r\n") {
        assert_eq!(s.read(&mut byte).await.unwrap(), 1, "the head ended early");
        raw.push(byte[0]);
    }
    let head = String::from_utf8(raw).unwrap();
    assert!(head.starts_with("HTTP/1.1 200 OK\r\n"), "{head}");
    assert!(
        head.contains("Content-Type: application/octet-stream\r\n"),
        "{head}"
    );
    assert!(!head.contains("Content-Length"), "{head}");
    assert!(head.contains("Connection: close\r\n"), "{head}");
    assert!(
        head.contains("X-Content-Type-Options: nosniff\r\n"),
        "{head}"
    );
    let policy = format!("Content-Security-Policy: {POLICY}\r\n");
    assert!(head.contains(&policy), "{head}");
    Events {
        source: Source::H1(Box::new(s)),
        frames: Frames::default(),
    }
}

/// The record of the network `key` of the profile on `client`.
async fn network_of(client: &Client, key: &[u8]) -> Option<Network> {
    let all = NetworkFrontendClient::new(client.clone())
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

/// A profile of its own on its own service joins through `invitation` and
/// waits until the network's root signs its membership.
async fn viewer(i: usize, invitation: Vec<u8>, net: Vec<u8>) -> (Svc, Vec<u8>) {
    let svc = service().await;
    let client = sign_up(&svc, &format!("viewer{i}")).await;
    NetworkFrontendClient::new(client.clone())
        .create_network_from_invitation(&CreateNetworkFromInvitationRequest {
            alias: String::new(),
            invitation: Some(InvitationField::InvitationBytes(invitation)),
        })
        .await
        .unwrap();
    let what = format!("viewer{i}'s renewed membership");
    let renewed = eventually(&what, STEP * 2, || async {
        let n = network_of(&client, &net).await?;
        let cert = n.certificate.as_ref()?;
        membership_trusted(cert, SystemClock.now()).then_some(())
    });
    renewed.await;
    let key = svc.node_info().unwrap().profile_key;
    (svc, key)
}

#[test]
fn the_bootstrap_api_serves_an_owned_networks_streams_and_invitations() {
    run(async {
        tokio::time::timeout(StdDuration::from_secs(420), async {
            let mut steps = Steps(Instant::now());
            let (dir, db) = db_file("bootstrap-api");
            let owned = first_run(&db).await;
            steps.done("the owner signs up with two networks and a video channel");

            let cert = TestCert::new(&["localhost"]);
            let (cert_path, key_path) = cert.write(&dir, 0);
            let ws_port = free_port();
            let defaults = local_config();
            let svc = Svc::start(Config {
                db: Db::File(db.clone()),
                hls_listen: Some("127.0.0.1:0".into()),
                tls: Some(TlsConfig::new(cert_path, key_path)),
                profile: Some(ProfileLogin {
                    name: OWNER.into(),
                    password: PASSWORD.into(),
                }),
                bootstrap: Some(BootstrapConfig {
                    networks: vec![owned.net.clone()],
                    ws_url: format!("ws://127.0.0.1:{ws_port}/"),
                }),
                app: AppConfig {
                    ws_listen: Some(format!("127.0.0.1:{ws_port}")),
                    ..defaults.app.clone()
                },
                ..defaults
            })
            .await
            .unwrap();
            let key = svc.profile_key().unwrap().to_vec();
            let node = svc.node_info().expect("the headless profile's node runs");
            let ws_uri = format!("ws://127.0.0.1:{ws_port}/{}", HEXLOWER.encode(&key));
            assert_eq!(node.ws_uri.as_deref(), Some(ws_uri.as_str()));
            let ui = connect_ui(&svc).await;
            let signed_in = AuthFrontendClient::new(ui.client.clone())
                .sign_in(&SignInRequest {
                    credentials: Some(Credentials::Password(Password {
                        name: OWNER.into(),
                        password: PASSWORD.into(),
                        ..Default::default()
                    })),
                })
                .await
                .unwrap();
            assert_eq!(signed_in.profile.unwrap().key.unwrap().public, key);
            let client = ui.client.clone();
            steps.done("the service restarts with the owner headless, and a UI signs in");

            // A media listing on the network the API doesn't serve.
            let hidden = Uri::new(SwarmId::new(&[9; 32]), ingress_options().uri_options());
            let hidden = hidden.to_string();
            let directory = DirectoryFrontendClient::new(client.clone());
            eventually("a listing on the other network", STEP, || {
                let (directory, other, hidden) =
                    (directory.clone(), owned.other.clone(), hidden.clone());
                async move {
                    directory
                        .publish(&FrontendPublishRequest {
                            network_key: other,
                            listing: Some(Listing {
                                content: Some(Content::Media(Media {
                                    mime_type: units::MIME_TYPE.into(),
                                    swarm_uri: hidden,
                                })),
                            }),
                        })
                        .await
                        .ok()
                }
            })
            .await;
            let channels = VideoChannelFrontendClient::new(client.clone());
            let channel_id = channels.list(&Default::default()).await.unwrap().channels[0].id;
            let url = VideoIngressClient::new(client.clone())
                .get_channel_url(&VideoIngressGetChannelUrlRequest { id: channel_id })
                .await
                .unwrap();
            assert_eq!(url.server_addr, format!("127.0.0.1:{}", owned.ingress_port));
            let script = Rc::new(Script::new(&ScriptConfig {
                gop_frames: 30,
                duration_secs: 300,
                ..ScriptConfig::default()
            }));
            let publish = eventually("the ingress to accept the publish", STEP, || {
                let (addr, key) = (url.server_addr.clone(), url.stream_key.clone());
                async move { Publish::open(&addr, &key, StdDuration::from_secs(10)).await }
            })
            .await;
            let _streaming = spawn_local(publish.stream(script));
            steps.done("a publisher streams to the channel");

            let addr = svc.hls_addr().unwrap();
            let channel = Uri::new(
                SwarmId::new(&owned.channel_key),
                ingress_options().uri_options(),
            );
            let channel = channel.to_string();
            let mut h2_events = events_h2(&cert, addr).await;
            let (id, thumbnail) = h2_events
                .media_thumbnail(&owned.net, &channel, &hidden)
                .await;
            let net = BASE64URL_NOPAD.encode(&owned.net);
            let prefix = format!("/api/v1/directory/thumbnails/{net}/{id}/");
            assert!(thumbnail.starts_with(&prefix), "{thumbnail}");
            let mut h1_events = events_h1(&cert, addr).await;
            assert_eq!(
                h1_events
                    .media_thumbnail(&owned.net, &channel, &hidden)
                    .await,
                (id, thumbnail.clone())
            );
            steps.done("both event streams list the channel with its thumbnail's path");

            let http1 = cert.client(&[b"http/1.1"]);
            let get = |path: String| {
                let http1 = http1.clone();
                async move { support::tls::request(&http1, addr, "localhost", "GET", &path).await }
            };
            let image = get(thumbnail.clone()).await;
            assert_eq!(image.status, 200);
            image.check(
                "the thumbnail",
                "image/png",
                "public, max-age=31536000, immutable",
            );
            assert_eq!(image.body, PNG);
            assert_eq!(image.header("x-content-type-options"), Some("nosniff"));
            assert_eq!(image.header("content-security-policy"), Some(POLICY));
            let digest = thumbnail.rsplit('/').next().unwrap();
            let wrong = thumbnail.replace(digest, &"0".repeat(64));
            assert_eq!(get(wrong).await.status, 404);
            let other = BASE64URL_NOPAD.encode(&owned.other);
            let elsewhere = thumbnail.replace(&net, &other);
            assert_eq!(get(elsewhere).await.status, 404);

            // The owner's directory holds the SVG logo; the API serves none.
            let held = directory
                .get_listings(&FrontendGetListingsRequest {
                    content_types: vec![ListingContentType::Media as i32],
                    network_keys: vec![owned.net.clone()],
                })
                .await
                .unwrap()
                .listings;
            let logo = held
                .iter()
                .flat_map(|n| &n.listings)
                .find(|item| item.id == id)
                .and_then(|item| item.snippet.as_ref()?.channel_logo.clone());
            assert_eq!(logo, Some(svg_logo()));
            let svg = strims_directory::snippet::digest(&svg_logo().encode_to_vec());
            let svg = HEXLOWER.encode(&svg);
            let svg = get(format!("{prefix}{svg}")).await;
            assert_eq!(svg.status, 404);
            assert_eq!(svg.header("x-content-type-options"), Some("nosniff"));
            assert_eq!(svg.header("content-security-policy"), Some(POLICY));
            let hls = get("/hls/x".into()).await;
            assert_eq!(hls.status, 400);
            assert_eq!(hls.header("x-content-type-options"), Some("nosniff"));
            assert_eq!(hls.header("content-security-policy"), None);
            steps.done("the thumbnail's path answers its bytes, and the SVG logo's none");

            let reply = get(format!("/api/v1/invitation/{net}")).await;
            assert_eq!(reply.status, 200);
            reply.check("the invitation", "application/protobuf", "no-store");
            let invitation = Invitation::decode(reply.body.as_slice()).unwrap();
            assert_eq!(invitation.version, 0);
            let v0 = InvitationV0::decode(invitation.data.as_slice()).unwrap();
            let [client_options] = v0
                .bootstrap_clients
                .iter()
                .map(|c| c.client_options.clone())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();
            let Some(ClientOptions::WebsocketOptions(ws)) = client_options else {
                panic!("a bootstrap client without a WebSocket URL");
            };
            assert_eq!(ws.url, ws_uri);
            assert_eq!(get(format!("/api/v1/invitation/{other}")).await.status, 404);
            assert_eq!(get("/api/v1/invitation/x".into()).await.status, 404);
            assert_eq!(get("/api/v1/nope".into()).await.status, 404);
            steps.done("the API vends an invitation that dials the node");

            h1_events.ping().await;
            steps.done("the HTTP/1.1 event stream pings");

            let joined =
                join_all((0..VIEWERS).map(|i| viewer(i, reply.body.clone(), owned.net.clone())));
            let viewers = joined.await;
            steps.done("more profiles than a quota join through the invitation");

            let peers = NetworkFrontendClient::new(client.clone())
                .list_peers(&ListPeersRequest {
                    network_id: owned.network_id,
                })
                .await
                .unwrap()
                .peers;
            let owner = peers.iter().find(|p| p.public_key == key).unwrap();
            assert_eq!(owner.invite_quota, DEFAULT_INVITE_QUOTA);
            for (_, viewer_key) in &viewers {
                let p = peers.iter().find(|p| &p.public_key == viewer_key).unwrap();
                assert_eq!(p.inviter_peer_id, owner.id);
            }
            h2_events.ping().await;
            steps.done("the CA records them as the owner's, its quota unspent");

            drop(viewers);
            svc.stop().await;
            let _ = std::fs::remove_dir_all(&dir);
        })
        .await
        .expect("the scenario finishes in time");
    });
}

/// Opens an event stream over plain HTTP/1.1: its head, and the
/// connection carrying its body.
async fn open_events(addr: std::net::SocketAddr) -> (String, tokio::net::TcpStream) {
    let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
    s.write_all(b"GET /api/v1/directory/events HTTP/1.1\r\nHost: x\r\n\r\n")
        .await
        .unwrap();
    let mut raw = Vec::new();
    let mut byte = [0u8; 1];
    while !raw.ends_with(b"\r\n\r\n") {
        if s.read(&mut byte).await.unwrap() == 0 {
            break;
        }
        raw.push(byte[0]);
    }
    (String::from_utf8(raw).unwrap(), s)
}

#[test]
fn event_streams_leave_the_listener_room_for_other_requests() {
    run(async {
        let (dir, db) = db_file("bootstrap-cap");
        let svc = Svc::start(Config {
            db: Db::File(db.clone()),
            ..local_config()
        })
        .await
        .unwrap();
        let client = sign_up(&svc, OWNER).await;
        let network = create_network(&client, "streams").await;
        let net = network_key(network.certificate.as_ref().unwrap());
        svc.stop().await;

        let mut app = local_config().app;
        app.video.hls.max_connections = 4;
        let svc = Svc::start(Config {
            db: Db::File(db.clone()),
            hls_listen: Some("127.0.0.1:0".into()),
            profile: Some(ProfileLogin {
                name: OWNER.into(),
                password: PASSWORD.into(),
            }),
            bootstrap: Some(BootstrapConfig {
                networks: vec![net.clone()],
                ws_url: "wss://node.example".into(),
            }),
            app,
            ..local_config()
        })
        .await
        .unwrap();
        let addr = svc.hls_addr().unwrap();
        let mut open = Vec::new();
        for _ in 0..2 {
            let (head, s) = open_events(addr).await;
            assert!(head.starts_with("HTTP/1.1 200 OK\r\n"), "{head}");
            open.push(s);
        }
        let past = request(addr, "GET", "/api/v1/directory/events").await;
        assert_eq!(past.status, 503);
        let invitation = format!("/api/v1/invitation/{}", BASE64URL_NOPAD.encode(&net));
        assert_eq!(request(addr, "GET", &invitation).await.status, 200);
        assert_eq!(request(addr, "GET", "/hls/x").await.status, 400);

        // A stream whose client goes makes room for another.
        open.pop();
        eventually("a stream in the freed place", STEP, || async {
            let (head, s) = open_events(addr).await;
            head.starts_with("HTTP/1.1 200 OK\r\n").then_some(s)
        })
        .await;
        svc.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    });
}

#[test]
fn a_bootstrap_api_the_profile_cant_serve_fails_the_start() {
    run(async {
        // Another profile's network, which the owner joins as a member.
        let host = service().await;
        let host_client = sign_up(&host, "host").await;
        let theirs = create_network(&host_client, "theirs").await;
        let theirs_key = network_key(theirs.certificate.as_ref().unwrap());
        let invitation = NetworkFrontendClient::new(host_client.clone())
            .create_invitation(&CreateInvitationRequest {
                network_id: theirs.id,
                bootstrap_client_id: 0,
            })
            .await
            .unwrap()
            .invitation
            .unwrap();

        let (dir, db) = db_file("bootstrap-start");
        let svc = Svc::start(Config {
            db: Db::File(db.clone()),
            ..local_config()
        })
        .await
        .unwrap();
        let client = sign_up(&svc, OWNER).await;
        let network = create_network(&client, "streams").await;
        let net = network_key(network.certificate.as_ref().unwrap());
        let joined = NetworkFrontendClient::new(client.clone())
            .create_network_from_invitation(&CreateNetworkFromInvitationRequest {
                alias: String::new(),
                invitation: Some(InvitationField::InvitationBytes(invitation.encode_to_vec())),
            })
            .await
            .unwrap()
            .network
            .unwrap();
        assert!(joined.server_config.is_none());
        assert_eq!(
            network_key(joined.certificate.as_ref().unwrap()),
            theirs_key
        );
        svc.stop().await;
        host.stop().await;

        let config = |name: &str, password: &str, networks: Vec<Vec<u8>>| Config {
            db: Db::File(db.clone()),
            hls_listen: Some("127.0.0.1:0".into()),
            profile: Some(ProfileLogin {
                name: name.into(),
                password: password.into(),
            }),
            bootstrap: Some(BootstrapConfig {
                networks,
                ws_url: "wss://node.example".into(),
            }),
            ..local_config()
        };
        let err = |c: Config| async { Svc::start(c).await.err().unwrap() };
        for networks in [
            vec![theirs_key.clone()],
            vec![net.clone(), theirs_key.clone()],
            vec![vec![3; 32]],
        ] {
            assert!(matches!(
                err(config(OWNER, PASSWORD, networks)).await,
                SvcError::NotOwner(_)
            ));
        }
        for (name, password) in [(OWNER, "wrong"), ("nobody", PASSWORD)] {
            assert!(matches!(
                err(config(name, password, vec![net.clone()])).await,
                SvcError::SignIn(..)
            ));
        }
        let no_profile = Config {
            profile: None,
            ..config(OWNER, PASSWORD, vec![net.clone()])
        };
        assert!(matches!(err(no_profile).await, SvcError::BootstrapConfig));
        let svc = Svc::start(config(OWNER, PASSWORD, vec![net]))
            .await
            .unwrap();
        assert!(svc.node_info().is_some());
        svc.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    });
}
