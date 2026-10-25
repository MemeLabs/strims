// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Pages for headless browsers: a server for a built bundle and for the
//! pages' WebSockets, and the browsers' command lines. strims-wasm's
//! browser tests and the testbed's `testbed_browser` share them.
//!
//! - **Server** ([`Site`]). One listener on 127.0.0.1 serves a [`Bundle`]
//!   at its root. Every WebSocket upgrade goes to the [`Site::sockets`]
//!   receiver, with its path and query: e2e.html's `/control?i=<n>`, or
//!   whatever a page was told to connect to.
//! - **Firefox** runs `--headless --no-remote` on a profile with
//!   [`FIREFOX_PREFS`] ([`firefox_profile`]).
//! - **Chromium** runs headless with [`CHROMIUM_FLAGS`].
//!
//! Both browsers keep ICE host candidates as addresses, so pages link over
//! WebRTC without resolving each other's mDNS names, and put the pages'
//! console on their output.

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

/// Serves one HTTP request: a file of the bundle, or a WebSocket upgrade
/// handed to `sockets` with its path and query.
async fn serve(
    mut conn: TcpStream,
    bundle: &Bundle,
    sockets: &mpsc::UnboundedSender<(String, Socket)>,
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
            return respond(conn, "400 Bad Request", "text/plain", b"").await;
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
        Some(body) if method == "GET" => respond(conn, "200 OK", content_type(path), body).await,
        _ => respond(conn, "404 Not Found", "text/plain", b"").await,
    }
}

async fn respond(
    mut conn: TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n",
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
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let (tx, sockets) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            while let Ok((conn, _)) = listener.accept().await {
                let (bundle, tx) = (bundle.clone(), tx.clone());
                tokio::spawn(async move {
                    let _ = serve(conn, &bundle, &tx).await;
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
    // The pages' console, on stderr.
    "--enable-logging=stderr",
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
