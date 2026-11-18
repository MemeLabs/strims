// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Pages for headless browsers: a server for a built bundle and for the
//! pages' WebSockets, and the browsers' command lines. strims-wasm's
//! browser tests and the testbed's `testbed_browser` share them.
//!
//! - **Server** ([`Site`]). One listener on 127.0.0.1 serves a [`Bundle`]
//!   at its root. Every WebSocket upgrade goes to the [`Site::sockets`]
//!   receiver, with its path and query: e2e.html's `/control?i=<n>`, or
//!   whatever a page was told to connect to. [`Site::serve_isolated`]
//!   serves it cross-origin isolated, for a measurement of timers: the
//!   browsers then read `performance.now()` at their isolated pages'
//!   finer clamp. Firefox's `Date.now()`, a browser node's own clock
//!   (strims-wasm's `clock::now`), returns whole ms on either page, but
//!   isolation likely changes whether that ms is jittered (its 1-ms clamp
//!   jitters a plain page's), so a Firefox node's deadlines may fall
//!   otherwise than on a plain page; Chromium's `Date.now()` is the same
//!   on either. The farm's testbed-timers-control.toml compares Firefox's
//!   runs with the timer log off and on.
//! - **Firefox** runs `--headless --no-remote` on a profile with
//!   [`FIREFOX_PREFS`] ([`firefox_profile`]), which leaves its timer
//!   precision at its defaults.
//! - **Chromium** runs headless with [`CHROMIUM_FLAGS`].
//!
//! Both browsers keep ICE host candidates as addresses, so pages link over
//! WebRTC without resolving each other's mDNS names, put the pages'
//! console on their output, and let pages play media with sound on their
//! own.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::Role;

use crate::topology::Kind;

/// A WebSocket a page opened to the server.
pub type Socket = WebSocketStream<TcpStream>;

/// Built pages, by path below the server's root.
pub struct Bundle(pub BTreeMap<String, Vec<u8>>);

impl Bundle {
    /// The files below `dir`, read into memory, so a later build can't
    /// change what a running server serves.
    pub fn read(dir: &Path) -> std::io::Result<Bundle> {
        let mut files = BTreeMap::new();
        read_tree(dir, "", &mut files)?;
        Ok(Bundle(files))
    }
}

fn read_tree(
    dir: &Path,
    prefix: &str,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if entry.file_type()?.is_dir() {
            read_tree(&entry.path(), &format!("{name}/"), files)?;
        } else {
            files.insert(name, std::fs::read(entry.path())?);
        }
    }
    Ok(())
}

/// The headers that make a page cross-origin isolated
/// ([`Site::serve_isolated`]): every response of the bundle's is
/// same-origin, so each takes them.
const ISOLATED: &str =
    "Cross-Origin-Opener-Policy: same-origin\r\nCross-Origin-Embedder-Policy: require-corp\r\n";

/// Serves one HTTP request: a file of the bundle, or a WebSocket upgrade
/// handed to `sockets` with its path and query; each response with
/// `extra`'s headers.
async fn serve(
    mut conn: TcpStream,
    bundle: &Bundle,
    sockets: &mpsc::UnboundedSender<(String, Socket)>,
    extra: &str,
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
        head.extend_from_slice(buf.get(..n).unwrap_or_default());
    }
    let (method, target, accept) = {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        if !matches!(req.parse(&head), Ok(httparse::Status::Complete(_))) {
            return respond(conn, "400 Bad Request", "text/plain", b"", extra).await;
        }
        let header = |name: &str| {
            req.headers
                .iter()
                .find(|h| h.name.eq_ignore_ascii_case(name))
                .map(|h| h.value)
        };
        let upgrade = header("upgrade").is_some_and(|v| v.eq_ignore_ascii_case(b"websocket"));
        let accept = header("sec-websocket-key")
            .filter(|_| upgrade)
            .map(derive_accept_key);
        (
            req.method.unwrap_or("").to_owned(),
            req.path.unwrap_or("/").to_owned(),
            accept,
        )
    };
    if let Some(accept) = accept {
        let reply = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        );
        conn.write_all(reply.as_bytes()).await?;
        let ws = WebSocketStream::from_raw_socket(conn, Role::Server, None).await;
        let _ = sockets.send((target, ws));
        return Ok(());
    }
    let mut path = target
        .split('?')
        .next()
        .unwrap_or("")
        .trim_start_matches('/');
    // A route of a single-page app, whose router reads the path.
    let is_route = !path.rsplit('/').next().unwrap_or("").contains('.');
    if !bundle.0.contains_key(path) && is_route && bundle.0.contains_key("index.html") {
        path = "index.html";
    }
    match bundle.0.get(path) {
        Some(body) if method == "GET" => {
            respond(conn, "200 OK", content_type(path), body, extra).await
        }
        _ => respond(conn, "404 Not Found", "text/plain", b"", extra).await,
    }
}

async fn respond(
    mut conn: TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
    extra: &str,
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\n{extra}Connection: close\r\n\r\n",
        body.len()
    );
    conn.write_all(head.as_bytes()).await?;
    conn.write_all(body).await?;
    conn.shutdown().await
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("wasm") => "application/wasm",
        Some("json" | "map") => "application/json",
        Some("webmanifest") => "application/manifest+json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// A server of a bundle on 127.0.0.1, until it drops. A route of the
/// bundle's single-page app, a path whose last segment has no `.`, gets
/// its `index.html`.
pub struct Site {
    pub addr: SocketAddr,
    /// The WebSockets pages opened, with their paths and queries.
    pub sockets: mpsc::UnboundedReceiver<(String, Socket)>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Site {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Site {
    /// Serves `bundle` on a port of 127.0.0.1 the system picks.
    pub async fn serve(bundle: Arc<Bundle>) -> std::io::Result<Site> {
        Site::serve_with(bundle, "").await
    }

    /// [`Site::serve`], each response with the headers that make its pages
    /// cross-origin isolated (`Cross-Origin-Opener-Policy: same-origin`,
    /// `Cross-Origin-Embedder-Policy: require-corp`).
    pub async fn serve_isolated(bundle: Arc<Bundle>) -> std::io::Result<Site> {
        Site::serve_with(bundle, ISOLATED).await
    }

    async fn serve_with(bundle: Arc<Bundle>, extra: &'static str) -> std::io::Result<Site> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let (tx, sockets) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            while let Ok((conn, _)) = listener.accept().await {
                let (bundle, tx) = (bundle.clone(), tx.clone());
                tokio::spawn(async move {
                    let _ = serve(conn, &bundle, &tx, extra).await;
                });
            }
        });
        Ok(Site {
            addr,
            sockets,
            task,
        })
    }
}

/// Firefox's `user.js` for the pages.
pub const FIREFOX_PREFS: &str = concat!(
    "user_pref(\"browser.shell.checkDefaultBrowser\", false);\n",
    "user_pref(\"datareporting.policy.dataSubmissionEnabled\", false);\n",
    // Host candidates as addresses, so pages link without resolving each
    // other's mDNS names.
    "user_pref(\"media.peerconnection.ice.obfuscate_host_addresses\", false);\n",
    // The pages' and workers' console, on stdout.
    "user_pref(\"devtools.console.stdout.content\", true);\n",
    // An `open` after a `close` on the same profile loads only the page it
    // names. `close` kills Firefox, so its session isn't restored; and
    // thumbnail capture would load the profile's top sites, the pages
    // opened before, in a hidden browser.
    "user_pref(\"browser.sessionstore.resume_from_crash\", false);\n",
    "user_pref(\"browser.pagethumbnails.capturing_disabled\", true);\n",
    // Media plays, with sound, without a user's gesture.
    "user_pref(\"media.autoplay.default\", 0);\n",
    // No fetches of its own, which would cross the node's shaped link (the
    // testbed's network has no route out, so they only fail): updates of
    // the browser, its add-ons, media plugins (OpenH264) and remote
    // settings, safe browsing's lists, telemetry and studies, captive
    // portal and connectivity checks, the new tab page's feeds, DNS over
    // HTTPS and prefetches.
    // With no route out it would take itself to be offline, and offline it
    // lets no WebRTC connection through.
    "user_pref(\"network.manage-offline-status\", false);\n",
    "user_pref(\"app.update.enabled\", false);\n",
    "user_pref(\"app.update.auto\", false);\n",
    "user_pref(\"extensions.update.enabled\", false);\n",
    "user_pref(\"extensions.blocklist.enabled\", false);\n",
    "user_pref(\"extensions.getAddons.cache.enabled\", false);\n",
    "user_pref(\"media.gmp-manager.updateEnabled\", false);\n",
    "user_pref(\"services.settings.server\", \"\");\n",
    "user_pref(\"browser.safebrowsing.malware.enabled\", false);\n",
    "user_pref(\"browser.safebrowsing.phishing.enabled\", false);\n",
    "user_pref(\"browser.safebrowsing.downloads.enabled\", false);\n",
    "user_pref(\"browser.safebrowsing.blockedURIs.enabled\", false);\n",
    "user_pref(\"datareporting.healthreport.uploadEnabled\", false);\n",
    "user_pref(\"toolkit.telemetry.enabled\", false);\n",
    "user_pref(\"toolkit.telemetry.unified\", false);\n",
    "user_pref(\"app.normandy.enabled\", false);\n",
    "user_pref(\"app.shield.optoutstudies.enabled\", false);\n",
    "user_pref(\"network.captive-portal-service.enabled\", false);\n",
    "user_pref(\"network.connectivity-service.enabled\", false);\n",
    "user_pref(\"browser.newtabpage.enabled\", false);\n",
    "user_pref(\"browser.newtabpage.activity-stream.feeds.topsites\", false);\n",
    "user_pref(\"browser.newtabpage.activity-stream.feeds.section.topstories\", false);\n",
    "user_pref(\"browser.topsites.contile.enabled\", false);\n",
    "user_pref(\"browser.startup.homepage_override.mstone\", \"ignore\");\n",
    "user_pref(\"network.trr.mode\", 5);\n",
    "user_pref(\"network.dns.disablePrefetch\", true);\n",
    "user_pref(\"network.prefetch-next\", false);\n",
);

/// Chromium's flags for the pages, besides `--user-data-dir`.
pub const CHROMIUM_FLAGS: &[&str] = &[
    "--headless",
    // A container has no user namespaces for the sandbox.
    "--no-sandbox",
    // Host candidates as addresses.
    "--disable-features=WebRtcHideLocalIpsWithMdns",
    // A container's /dev/shm is small.
    "--disable-dev-shm-usage",
    "--no-first-run",
    "--no-default-browser-check",
    // No fetches of its own, which would cross the node's shaped link (the
    // testbed's network has no route out, so they only fail): component
    // updates, safe browsing, variations and the rest of its background
    // networking.
    "--disable-background-networking",
    "--disable-component-update",
    // The pages' console, on stderr.
    "--enable-logging=stderr",
    // Media plays, with sound, without a user's gesture.
    "--autoplay-policy=no-user-gesture-required",
];

/// Writes [`FIREFOX_PREFS`] into the Firefox profile `dir`.
pub fn firefox_profile(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("user.js"), FIREFOX_PREFS)
}

/// The arguments that open `url` in a headless browser of `kind` on the
/// profile `dir`; a Firefox profile needs [`firefox_profile`] first.
pub fn args(kind: Kind, dir: &Path, url: &str) -> Vec<OsString> {
    let mut args: Vec<OsString> = Vec::new();
    match kind {
        Kind::Chromium => {
            args.extend(CHROMIUM_FLAGS.iter().map(OsString::from));
            let mut d = OsString::from("--user-data-dir=");
            d.push(dir);
            args.push(d);
        }
        Kind::Firefox | Kind::Native => {
            args.extend(["--headless", "--no-remote", "--profile"].map(OsString::from));
            args.push(dir.into());
        }
    }
    args.push(url.into());
    args
}
