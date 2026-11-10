// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The one-click watch page in headless Firefox against a native service
//! whose headless profile owns a network with a live stream.
//!
//! A profile signs up on a service, creates a network and a video channel
//! on it, and turns its RTMP ingress on; the service restarts with the
//! profile headless and the network on its bootstrap API, whose
//! invitations dial a `wss://` URL: a TLS-terminating proxy, with a
//! self-signed certificate the browser accepts, in front of the node's
//! WebSocket listener. ffmpeg publishes a test pattern (H.264 and AAC) to
//! the channel. One origin serves the production bundle (`npx webpack
//! --mode production`, as `ui_smoke` builds it), the watch page at
//! `/watch`, and pipes `/api/v1` to the service's HLS listener, as a
//! reverse proxy in front of both would.
//!
//! A fresh Firefox profile opens `/watch`: the channel's tile shows, its
//! thumbnail from the API. One click on it opens the stream: the page's
//! node, signed up on its own, joins the network with the API's
//! invitation over the proxy, and the video plays (its time advances).
//! Then the page's store holds one linked profile, the one the page
//! remembers, and the network's CA one more peer, the owner's invitee. A
//! reload signs in with the same profile and plays again: still one
//! linked profile, and no new peer.
//!
//! Ignored by default (it launches a browser); skipped without ffmpeg,
//! geckodriver, or what the bundle's build needs:
//!
//! ```sh
//! cargo test -p strims-wasm --test ui_watch -- --ignored --nocapture
//! ```
//!
//! Each step prints how long it took.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    // ffmpeg, an HTTP server and the wall clock
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

mod support;

use std::future::{Future, ready};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use rustls::ServerConfig;
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use serde_json::{Value, json};
use strims_crypto::CertificateChain;
use strims_node::app::AppConfig;
use strims_proto::auth::v1::sign_in_request::{Credentials, Password};
use strims_proto::auth::v1::{AuthFrontendClient, SignInRequest, SignUpRequest};
use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;
use strims_proto::network::v1::directory::{ListingSnippet, ListingSnippetImage};
use strims_proto::network::v1::{
    CreateServerRequest, ListPeersRequest, NetworkFrontendClient, Peer,
};
use strims_proto::types::{Certificate, Image, ImageType};
use strims_proto::video::v1::{
    VideoChannelCreateRequest, VideoChannelFrontendClient, VideoIngressClient, VideoIngressConfig,
    VideoIngressGetChannelUrlRequest, VideoIngressSetConfigRequest,
};
use strims_rpc::{Client, Config as RpcConfig, connect};
use strims_svc::{BootstrapConfig, Config, Db, ProfileLogin, Svc};
use support::browser::{Bundle, Unavailable, build};
use support::webdriver::Session;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::{JoinHandle, LocalSet, spawn_local};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::{self, Message};

const OWNER: &str = "owner";
const PASSWORD: &str = "pw";
/// The channel's title, which its tile shows.
const TITLE: &str = "one click";

/// A 1x1 PNG, the channel's thumbnail.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae,
    0x42, 0x60, 0x82,
];

/// The limit of one wait or call.
const STEP: Duration = Duration::from_secs(60);
/// How long a click has to play: the page's sign-up, the join, the
/// renewal at the CA and the stream's first segments.
const PLAY_TIMEOUT: Duration = Duration::from_secs(120);
/// How far the video's time must advance once it plays.
const PLAYED_SECS: f64 = 2.0;
/// The limit of the whole scenario, the build aside.
const TEST_LIMIT: Duration = Duration::from_secs(600);
/// How long ffmpeg publishes at most.
const PUBLISH_SECS: u32 = 900;

/// A port the OS picks, free when this returns.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

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
        assert!(
            Instant::now() < deadline,
            "timed out after {limit:?} waiting for {what}"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// The scenario's steps and how long each took.
struct Steps(Vec<(&'static str, Duration)>, Instant);

impl Steps {
    fn done(&mut self, step: &'static str) {
        let took = self.1.elapsed();
        eprintln!("{step}: {took:.2?}");
        self.0.push((step, took));
        self.1 = Instant::now();
    }

    fn summary(&self) {
        let total: Duration = self.0.iter().map(|(_, d)| *d).sum();
        eprintln!("--- steps ({total:.2?} in all) ---");
        for (step, took) in &self.0 {
            eprintln!("{took:>10.2?}  {step}");
        }
    }
}

/// A frontend RPC client over `ws`, one Call message per binary frame.
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

/// A UI's connection to `svc`'s API.
async fn connect_ui(svc: &Svc) -> Client {
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

fn network_key(cert: &Certificate) -> Vec<u8> {
    CertificateChain::decode(cert)
        .unwrap()
        .network_key()
        .to_vec()
}

/// The service's config over the store `db`: its API and node on
/// 127.0.0.1.
fn config(db: &Path) -> Config {
    Config {
        api_listen: "127.0.0.1:0".into(),
        db: Db::File(db.to_owned()),
        app: AppConfig {
            ws_listen: Some("127.0.0.1:0".into()),
            ..AppConfig::default()
        },
        ..Config::default()
    }
}

/// What the owner's first run left in the store.
struct Owned {
    net: Vec<u8>,
    network_id: u64,
    ingress_port: u16,
}

/// Signs the owner up on a service over `db`, with a network, a video
/// channel on it whose snippet is [`TITLE`] with [`PNG`], and the RTMP
/// ingress on.
async fn first_run(db: &Path) -> Owned {
    let svc = Svc::start(config(db)).await.unwrap();
    let client = connect_ui(&svc).await;
    AuthFrontendClient::new(client.clone())
        .sign_up(&SignUpRequest {
            name: OWNER.into(),
            password: PASSWORD.into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let network = NetworkFrontendClient::new(client.clone())
        .create_server(&CreateServerRequest {
            name: "streams".into(),
            ..Default::default()
        })
        .await
        .unwrap()
        .network
        .unwrap();
    let net = network_key(network.certificate.as_ref().unwrap());
    VideoChannelFrontendClient::new(client.clone())
        .create(&VideoChannelCreateRequest {
            directory_listing_snippet: Some(ListingSnippet {
                title: TITLE.into(),
                channel_name: "the owner's channel".into(),
                thumbnail: Some(ListingSnippetImage {
                    source_oneof: Some(SourceOneof::Image(Image {
                        r#type: ImageType::Png as i32,
                        height: 1,
                        width: 1,
                        data: PNG.to_vec(),
                    })),
                }),
                ..Default::default()
            }),
            network_key: net.clone(),
        })
        .await
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
        network_id: network.id,
        ingress_port,
    }
}

/// Whether ffmpeg runs here.
fn ffmpeg_runs() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// ffmpeg publishing a test pattern and a tone to `url` in real time, as
/// H.264 (1-second GOPs) and AAC. Dropping it stops ffmpeg.
struct Publisher(Child);

impl Publisher {
    fn start(url: &str) -> Publisher {
        let secs = PUBLISH_SECS.to_string();
        let child = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-re"])
            .args(["-f", "lavfi", "-i", "testsrc2=size=640x360:rate=30"])
            .args(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000"])
            .args([
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-profile:v",
                "high",
            ])
            .args(["-pix_fmt", "yuv420p", "-g", "30", "-keyint_min", "30"])
            .args(["-sc_threshold", "0", "-b:v", "1M"])
            .args(["-c:a", "aac", "-b:a", "128k", "-ar", "48000", "-ac", "2"])
            .args(["-t", &secs, "-f", "flv", url])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        Publisher(child)
    }

    /// Whether ffmpeg still runs.
    fn running(&mut self) -> bool {
        matches!(self.0.try_wait(), Ok(None))
    }
}

impl Drop for Publisher {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A TLS-terminating proxy on `listener` in front of the node's WebSocket
/// listener at `upstream`, with a self-signed certificate: the `wss://`
/// URL a browser dials. Counts the TLS connections it took in `accepted`.
fn wss_proxy(
    listener: TcpListener,
    upstream: SocketAddr,
    accepted: Arc<AtomicUsize>,
) -> JoinHandle<()> {
    let names = vec!["127.0.0.1".to_owned(), "localhost".to_owned()];
    let key = rcgen::generate_simple_self_signed(names).unwrap();
    let cert = key.cert.der().clone();
    let private = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.signing_key.serialize_der()));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert], private)
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    tokio::spawn(async move {
        while let Ok((conn, _)) = listener.accept().await {
            let (acceptor, accepted) = (acceptor.clone(), accepted.clone());
            tokio::spawn(async move {
                let Ok(mut tls) = acceptor.accept(conn).await else {
                    return;
                };
                accepted.fetch_add(1, Ordering::Relaxed);
                let Ok(mut up) = TcpStream::connect(upstream).await else {
                    return;
                };
                let _ = tokio::io::copy_bidirectional(&mut tls, &mut up).await;
            });
        }
    })
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("wasm") => "application/wasm",
        Some("json" | "map") => "application/json",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// Serves one request of the page's origin: the bootstrap API's, piped to
/// `api` as it came, else a file of `bundle`, `/watch` and the paths under
/// it being the watch page.
async fn front_request(
    mut conn: TcpStream,
    bundle: &Bundle,
    api: SocketAddr,
) -> std::io::Result<()> {
    let mut head = Vec::new();
    let mut buf = [0u8; 4096];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        if head.len() > 64 * 1024 {
            return Ok(());
        }
        let n = conn.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }
        head.extend_from_slice(&buf[..n]);
    }
    let target = {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let _ = req.parse(&head);
        req.path.unwrap_or("/").to_owned()
    };
    if target.starts_with("/api/v1/") {
        let mut upstream = TcpStream::connect(api).await?;
        upstream.write_all(&head).await?;
        tokio::io::copy_bidirectional(&mut conn, &mut upstream).await?;
        return Ok(());
    }
    let path = target.split('?').next().unwrap_or("");
    let file = if path == "/watch" || path.starts_with("/watch/") {
        "watch.html"
    } else {
        path.trim_start_matches('/')
    };
    let (status, body) = match bundle.0.get(file) {
        Some(b) => ("200 OK", b.as_slice()),
        None => ("404 Not Found", &[][..]),
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n",
        content_type(file),
        body.len()
    );
    conn.write_all(head.as_bytes()).await?;
    conn.write_all(body).await?;
    conn.shutdown().await
}

/// The page's origin on 127.0.0.1 ([`front_request`]).
async fn front(bundle: Arc<Bundle>, api: SocketAddr) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        while let Ok((conn, _)) = listener.accept().await {
            let bundle = bundle.clone();
            tokio::spawn(async move {
                let _ = front_request(conn, &bundle, api).await;
            });
        }
    });
    (addr, task)
}

/// The network's peers at its CA, the owner's.
async fn peers(client: &Client, network_id: u64) -> Vec<Peer> {
    NetworkFrontendClient::new(client.clone())
        .list_peers(&ListPeersRequest { network_id })
        .await
        .unwrap()
        .peers
}

/// What the page shows of its player, its video's time included.
const PLAYER_STATE: &str = r#"
    const v = document.querySelector("video");
    const s = document.querySelector(".watch_status");
    return {
        time: v ? v.currentTime : null,
        readyState: v ? v.readyState : null,
        error: v && v.error ? `${v.error.code}: ${v.error.message}` : null,
        status: s ? s.textContent : null,
        path: location.pathname,
    };
"#;

/// The ids of the page's linked profiles, and the one it remembers.
const STORED_PROFILES: &str = r#"
    const done = arguments[arguments.length - 1];
    const open = indexedDB.open("linkedProfiles");
    open.onsuccess = () => {
        const keys = open.result.transaction("data").objectStore("data").getAllKeys();
        keys.onsuccess = () =>
            done({ ids: keys.result, remembered: localStorage.getItem("watch.profileId") });
        keys.onerror = () => done({ error: String(keys.error) });
    };
    open.onerror = () => done({ error: String(open.error) });
"#;

/// The page's linked profile ids, and the one it remembers.
async fn stored_profiles(session: &Session) -> (Vec<String>, Option<String>) {
    let v = session.execute_async(STORED_PROFILES).await;
    assert!(v["error"].is_null(), "reading the linked profiles: {v}");
    let ids = v["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap().to_owned())
        .collect();
    (ids, v["remembered"].as_str().map(str::to_owned))
}

/// Waits for the channel's tile with its thumbnail from the API, clicks it,
/// and waits until the video's time advances [`PLAYED_SECS`].
async fn click_and_play(
    session: &Session,
    steps: &mut Steps,
    tile_step: &'static str,
    play_step: &'static str,
) {
    let title = session
        .wait_for(".directory_grid__item__channel__title", STEP * 2)
        .await;
    assert_eq!(session.text(&title).await, TITLE);
    let thumbnail = r#"
        const i = document.querySelector(".directory_grid__item__thumbnail");
        return i && i.complete ? [i.src, i.naturalWidth] : null;
    "#;
    eventually("the tile's thumbnail from the API", STEP, || async {
        let v = session.execute(thumbnail).await;
        let src = v[0].as_str()?;
        (src.contains("/api/v1/directory/thumbnails/") && v[1].as_u64()? > 0).then_some(())
    })
    .await;
    steps.done(tile_step);

    let link = session.find(".directory_grid__item__link").await.unwrap();
    session.click(&link).await;
    let mut last = Value::Null;
    let deadline = Instant::now() + PLAY_TIMEOUT;
    let mut started: Option<f64> = None;
    loop {
        let state = session.execute(PLAYER_STATE).await;
        if let Some(t) = state["time"].as_f64() {
            match started {
                None if t > 0.0 => started = Some(t),
                Some(s) if t >= s + PLAYED_SECS => break,
                _ => {}
            }
        }
        last = state;
        assert!(
            Instant::now() < deadline,
            "the video didn't play within {PLAY_TIMEOUT:?}: {last}"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert!(
        last["path"].as_str().unwrap().starts_with("/watch/"),
        "{last}"
    );
    steps.done(play_step);
}

#[tokio::test]
#[ignore = "launches headless Firefox through geckodriver"]
async fn one_click_on_a_watch_tile_joins_the_network_and_plays_the_stream() {
    let bundle = match build("web", &["--mode", "production"], "dist/web") {
        Ok(b) => Arc::new(b),
        Err(Unavailable::Skip(why)) => {
            eprintln!("skipping: {why}");
            return;
        }
        Err(Unavailable::Failed(why)) => panic!("{why}"),
    };
    assert!(
        bundle.0.contains_key("watch.html"),
        "the bundle has no watch page"
    );
    if !ffmpeg_runs() {
        eprintln!("skipping: no ffmpeg");
        return;
    }

    LocalSet::new()
        .run_until(within("the scenario", TEST_LIMIT, async {
            let mut steps = Steps(Vec::new(), Instant::now());
            let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
                .join(format!("ui-watch-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let db = dir.join("strims.redb");
            let owned = first_run(&db).await;
            steps.done("the owner signs up with a network and a video channel");

            let ws_port = free_port();
            let proxy = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let proxy_port = proxy.local_addr().unwrap().port();
            let accepted = Arc::new(AtomicUsize::new(0));
            let _proxy = wss_proxy(
                proxy,
                SocketAddr::from(([127, 0, 0, 1], ws_port)),
                accepted.clone(),
            );
            let defaults = config(&db);
            let svc = Svc::start(Config {
                hls_listen: Some("127.0.0.1:0".into()),
                profile: Some(ProfileLogin {
                    name: OWNER.into(),
                    password: PASSWORD.into(),
                }),
                bootstrap: Some(BootstrapConfig {
                    networks: vec![owned.net.clone()],
                    ws_url: format!("wss://127.0.0.1:{proxy_port}/"),
                }),
                app: AppConfig {
                    ws_listen: Some(format!("127.0.0.1:{ws_port}")),
                    ..defaults.app.clone()
                },
                ..defaults
            })
            .await
            .unwrap();
            let owner = connect_ui(&svc).await;
            AuthFrontendClient::new(owner.clone())
                .sign_in(&SignInRequest {
                    credentials: Some(Credentials::Password(Password {
                        name: OWNER.into(),
                        password: PASSWORD.into(),
                        ..Default::default()
                    })),
                })
                .await
                .unwrap();
            let before = peers(&owner, owned.network_id).await;
            assert_eq!(before.len(), 1, "the owner alone: {before:?}");
            let owner_peer = before[0].id;
            steps.done("the service restarts with the owner headless");

            let channels = VideoChannelFrontendClient::new(owner.clone());
            let channel_id = channels.list(&Default::default()).await.unwrap().channels[0].id;
            let url = VideoIngressClient::new(owner.clone())
                .get_channel_url(&VideoIngressGetChannelUrlRequest { id: channel_id })
                .await
                .unwrap();
            assert_eq!(url.server_addr, format!("127.0.0.1:{}", owned.ingress_port));
            let mut publisher = Publisher::start(&format!(
                "rtmp://{}/live/{}",
                url.server_addr, url.stream_key
            ));

            let (origin, _front) = front(bundle, svc.hls_addr().unwrap()).await;
            let prefs = json!({
                "devtools.console.stdout.content": true,
                "media.autoplay.default": 0,
                "media.autoplay.blocking_policy": 0,
            });
            let Some(session) = Session::start_with(prefs, true).await else {
                return;
            };
            steps.done("ffmpeg publishes, and Firefox starts");

            session.navigate(&format!("http://{origin}/watch")).await;
            click_and_play(
                &session,
                &mut steps,
                "a fresh profile's /watch shows the channel's tile",
                "one click joins and plays",
            )
            .await;
            assert!(publisher.running(), "ffmpeg stopped publishing");
            assert!(
                accepted.load(Ordering::Relaxed) > 0,
                "the page's node never dialed the wss:// proxy"
            );

            let invitee = eventually("the CA's record of the page's profile", STEP, || {
                let owner = owner.clone();
                async move {
                    let all = peers(&owner, owned.network_id).await;
                    (all.len() == 2).then_some(all)
                }
            })
            .await;
            let invitee = invitee.into_iter().find(|p| p.id != owner_peer).unwrap();
            assert_eq!(invitee.inviter_peer_id, owner_peer, "{invitee:?}");
            let (ids, remembered) = stored_profiles(&session).await;
            assert_eq!(ids.len(), 1, "linked profiles: {ids:?}");
            assert_eq!(remembered.as_deref(), Some(ids[0].as_str()));
            steps.done("one linked profile, and the CA records it as the owner's invitee");

            session.navigate(&format!("http://{origin}/watch")).await;
            click_and_play(
                &session,
                &mut steps,
                "a reload shows the tile",
                "the reloaded page plays with the same profile",
            )
            .await;
            assert_eq!(stored_profiles(&session).await, (ids, remembered));
            let after = peers(&owner, owned.network_id).await;
            assert_eq!(after.len(), 2, "a new peer after the reload: {after:?}");
            steps.done("still one linked profile and one invitee");

            session.end().await;
            drop(publisher);
            svc.stop().await;
            let _ = std::fs::remove_dir_all(&dir);
            steps.summary();
        }))
        .await;
}
