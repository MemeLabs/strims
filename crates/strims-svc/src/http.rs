// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HLS over HTTP: the running profile's HLS streams, answered from the
//! node factory's [`HlsRoutes`] for external players.
//!
//! The server speaks plain HTTP/1.1 and answers one request per
//! connection:
//!
//! - From its accept, a connection has
//!   [`HlsSettings::request_head_timeout`] to send a request head of at
//!   most [`HlsSettings::max_request_head_bytes`]. It closes at the
//!   deadline; a longer head answers 431, and one that doesn't parse 400.
//! - `GET` and `HEAD` are answered from the routes, at the node's clock;
//!   any other method answers 405. A path over
//!   [`HlsSettings::max_path_bytes`] answers 414.
//! - Every response carries `Content-Type`, `Cache-Control`,
//!   `Content-Length`, `Access-Control-Allow-Origin: *` and
//!   `Connection: close`. A HEAD's has no body.
//! - A response has [`RESPONSE_TIMEOUT`] to be sent. The connection then
//!   ends its sending side and takes what the client still sends, for at
//!   most [`LINGER`], before it closes: closing a socket with input unread
//!   resets the connection, which can drop the response before the client
//!   reads it.
//!
//! At most [`HlsSettings::max_connections`] connections are served at
//! once; the server closes the others as they come.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::net::SocketAddr;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use strims_rpc::Timer;
use strims_video::HlsSettings;
use strims_video::hls::{HlsResponse, HlsRoutes};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::{AbortHandle, spawn_local};
use tokio::time::{Instant, timeout, timeout_at};

use crate::SvcTimer;

/// How long a response may take to send.
const RESPONSE_TIMEOUT: StdDuration = StdDuration::from_secs(60);
/// How long a connection that answered takes the client's input before it
/// closes.
const LINGER: StdDuration = StdDuration::from_secs(1);
/// A short pause after a failed accept, so a listener error doesn't spin.
const ACCEPT_BACKOFF: StdDuration = StdDuration::from_millis(50);
/// The most header fields a request head holds.
const MAX_HEADERS: usize = 64;
/// The bytes a closing connection reads its client's input in.
const DRAIN_BUFFER: usize = 1024;

const ERROR_TYPE: &str = "text/plain; charset=utf-8";
const NO_STORE: &str = "no-store";

/// The server's bounds, from [`HlsSettings`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct Limits {
    pub(crate) max_path_bytes: usize,
    pub(crate) max_head_bytes: usize,
    pub(crate) head_timeout: StdDuration,
    pub(crate) max_connections: usize,
}

impl Limits {
    pub(crate) fn new(s: &HlsSettings) -> Limits {
        Limits {
            max_path_bytes: s.max_path_bytes,
            max_head_bytes: s.max_request_head_bytes,
            head_timeout: s.request_head_timeout.to_std().unwrap_or_default(),
            max_connections: s.max_connections,
        }
    }
}

/// The server's shared state.
pub(crate) struct Http {
    routes: HlsRoutes,
    timer: SvcTimer,
    limits: Limits,
    /// The connections being served. Bound: `limits.max_connections`.
    connections: RefCell<BTreeMap<u64, AbortHandle>>,
    next_id: Cell<u64>,
}

/// A response, and whether its body is sent.
#[derive(Debug)]
struct Answer {
    response: HlsResponse,
    /// False for a HEAD.
    body: bool,
}

impl Answer {
    /// An error the server answers itself, with a body naming it.
    fn error(status: u16) -> Answer {
        let text: &[u8] = match status {
            400 => b"bad request\n",
            405 => b"method not allowed\n",
            414 => b"path too long\n",
            431 => b"request head too large\n",
            _ => b"error\n",
        };
        Answer {
            response: HlsResponse {
                status,
                content_type: ERROR_TYPE,
                cache_control: NO_STORE,
                body: Rc::from(text),
            },
            body: true,
        }
    }

    /// The status line and headers.
    fn head(&self) -> String {
        let r = &self.response;
        let mut head = String::with_capacity(192);
        // Writing to a String can't fail.
        let _ = write!(
            head,
            "HTTP/1.1 {} {}\r\n\
             Content-Type: {}\r\n\
             Cache-Control: {}\r\n\
             Content-Length: {}\r\n\
             Access-Control-Allow-Origin: *\r\n",
            r.status,
            reason(r.status),
            r.content_type,
            r.cache_control,
            r.body.len(),
        );
        if r.status == 405 {
            head.push_str("Allow: GET, HEAD\r\n");
        }
        head.push_str("Connection: close\r\n\r\n");
        head
    }
}

/// The reason phrase of a status the server answers.
fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        414 => "URI Too Long",
        431 => "Request Header Fields Too Large",
        _ => "",
    }
}

impl Http {
    pub(crate) fn new(routes: HlsRoutes, timer: SvcTimer, limits: Limits) -> Http {
        Http {
            routes,
            timer,
            limits,
            connections: RefCell::new(BTreeMap::new()),
            next_id: Cell::new(0),
        }
    }

    /// Stops serving every connection.
    pub(crate) fn close(&self) {
        let connections = std::mem::take(&mut *self.connections.borrow_mut());
        for c in connections.values() {
            c.abort();
        }
    }

    /// The answer to the request head `head` begins, or `None` while more
    /// bytes may complete it.
    fn answer(&self, head: &[u8]) -> Option<Answer> {
        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut req = httparse::Request::new(&mut headers);
        match req.parse(head) {
            Ok(httparse::Status::Complete(_)) => {}
            Ok(httparse::Status::Partial) => return None,
            Err(httparse::Error::TooManyHeaders) => return Some(Answer::error(431)),
            Err(_) => return Some(Answer::error(400)),
        }
        let (Some(method), Some(path)) = (req.method, req.path) else {
            return Some(Answer::error(400));
        };
        let body = match method {
            "GET" => true,
            "HEAD" => false,
            _ => return Some(Answer::error(405)),
        };
        if path.len() > self.limits.max_path_bytes {
            return Some(Answer::error(414));
        }
        Some(Answer {
            response: self.routes.get(self.timer.now(), path),
            body,
        })
    }

    /// Reads a request head and answers it; `None` when the connection
    /// closes, fails or runs out of time first.
    async fn read_request(&self, stream: &mut TcpStream, addr: SocketAddr) -> Option<Answer> {
        let deadline = Instant::now() + self.limits.head_timeout;
        let mut head = vec![0u8; self.limits.max_head_bytes];
        let mut len = 0;
        loop {
            let Some(free) = head.get_mut(len..).filter(|f| !f.is_empty()) else {
                return Some(Answer::error(431));
            };
            let n = match timeout_at(deadline, stream.read(free)).await {
                Ok(Ok(0)) => return None,
                Ok(Ok(n)) => n,
                Ok(Err(e)) => {
                    tracing::debug!(%addr, error = %e, "reading an HLS request failed");
                    return None;
                }
                Err(_) => {
                    tracing::debug!(%addr, "an HLS request head timed out");
                    return None;
                }
            };
            // `n` is at most `free.len()`, so `len` stays within `head`.
            len += n;
            if let Some(answer) = self.answer(head.get(..len).unwrap_or_default()) {
                return Some(answer);
            }
        }
    }
}

/// Removes a connection from the server's set when its task ends.
struct Served {
    http: Rc<Http>,
    id: u64,
}

impl Drop for Served {
    fn drop(&mut self) {
        self.http.connections.borrow_mut().remove(&self.id);
    }
}

/// Accepts players' connections until aborted.
pub(crate) async fn accept(listener: TcpListener, http: Rc<Http>) {
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                if http.connections.borrow().len() >= http.limits.max_connections {
                    tracing::debug!(%addr, "refusing an HLS connection beyond the limit");
                    continue;
                }
                let id = http.next_id.get() + 1;
                http.next_id.set(id);
                let served = Served {
                    http: http.clone(),
                    id,
                };
                let task = spawn_local(serve(served, stream, addr));
                http.connections
                    .borrow_mut()
                    .insert(id, task.abort_handle());
            }
            Err(e) => {
                tracing::debug!(error = %e, "accepting an HLS connection failed");
                tokio::time::sleep(ACCEPT_BACKOFF).await;
            }
        }
    }
}

/// Answers a connection's request, then closes it.
async fn serve(served: Served, mut stream: TcpStream, addr: SocketAddr) {
    let http = served.http.clone();
    if let Err(e) = stream.set_nodelay(true) {
        tracing::debug!(%addr, error = %e, "setting TCP_NODELAY failed");
    }
    let Some(answer) = http.read_request(&mut stream, addr).await else {
        return;
    };
    match timeout(RESPONSE_TIMEOUT, respond(&mut stream, &answer)).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            tracing::debug!(%addr, error = %e, "sending an HLS response failed");
            return;
        }
        Err(_) => {
            tracing::debug!(%addr, "sending an HLS response timed out");
            return;
        }
    }
    let _ = timeout(LINGER, drain(&mut stream)).await;
    drop(served);
}

/// Sends `answer` and ends the sending side.
async fn respond(stream: &mut TcpStream, answer: &Answer) -> std::io::Result<()> {
    stream.write_all(answer.head().as_bytes()).await?;
    if answer.body {
        stream.write_all(&answer.response.body).await?;
    }
    stream.shutdown().await
}

/// Reads and drops the client's input until it ends.
async fn drain(stream: &mut TcpStream) {
    let mut buf = [0u8; DRAIN_BUFFER];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )]

    use rand_core::Rng;
    use strims_base::rng::det_rng;
    use strims_base::time::Duration;
    use strims_node::SystemClock;
    use strims_node::app::NodeTimer;
    use strims_video::hls::{HlsProfile, HlsStream, PlaylistConfig};
    use tokio::task::LocalSet;

    use super::*;

    /// Routes holding one stream with one segment.
    struct Fixture {
        routes: HlsRoutes,
        _profile: HlsProfile,
        stream: HlsStream,
    }

    fn fixture() -> Fixture {
        let routes = HlsRoutes::new();
        let profile = routes.profile(&[1; 32]).unwrap();
        let now = NodeTimer(SystemClock).now();
        let stream = profile
            .open_stream(now, [2; 32], PlaylistConfig::default())
            .unwrap();
        stream.push_segment(b"init", b"media", Duration::from_millis(1500));
        Fixture {
            routes,
            _profile: profile,
            stream,
        }
    }

    fn limits() -> Limits {
        Limits::new(&HlsSettings::default())
    }

    fn http(routes: &HlsRoutes, limits: Limits) -> Http {
        Http::new(routes.clone(), NodeTimer(SystemClock), limits)
    }

    fn get(path: &str) -> Vec<u8> {
        format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").into_bytes()
    }

    #[test]
    fn gets_and_heads_answer_from_the_routes() {
        let f = fixture();
        let h = http(&f.routes, limits());
        let a = h.answer(&get(f.stream.path())).unwrap();
        assert_eq!(a.response.status, 200);
        assert!(a.body);
        assert_eq!(a.response.content_type, "application/vnd.apple.mpegurl");
        let playlist = String::from_utf8(a.response.body.to_vec()).unwrap();
        assert!(playlist.contains("#EXTINF:1.500,"), "{playlist}");
        let head = a.head();
        assert!(head.starts_with("HTTP/1.1 200 OK\r\n"), "{head}");
        for line in [
            "Content-Type: application/vnd.apple.mpegurl\r\n",
            "Cache-Control: no-cache\r\n",
            &format!("Content-Length: {}\r\n", playlist.len()),
            "Access-Control-Allow-Origin: *\r\n",
        ] {
            assert!(head.contains(line), "{head}");
        }
        assert!(head.ends_with("Connection: close\r\n\r\n"), "{head}");

        let req = format!("HEAD {} HTTP/1.0\r\n\r\n", f.stream.path());
        let a = h.answer(req.as_bytes()).unwrap();
        assert_eq!(a.response.status, 200);
        assert!(!a.body);
        let other = f.stream.path().replace(&"02".repeat(32), &"03".repeat(32));
        for (path, status) in [("/nope", 404), (other.as_str(), 404), ("/hls/x", 400)] {
            let a = h.answer(&get(path)).unwrap();
            assert_eq!(a.response.status, status, "{path}");
            assert!(a.head().contains("Access-Control-Allow-Origin: *\r\n"));
        }
    }

    #[test]
    fn requests_other_than_gets_and_heads_are_refused() {
        let f = fixture();
        let h = http(&f.routes, limits());
        let status = |req: &[u8]| h.answer(req).map(|a| a.response.status);
        let post = format!("POST {} HTTP/1.1\r\n\r\n", f.stream.path());
        assert_eq!(status(post.as_bytes()), Some(405));
        assert!(Answer::error(405).head().contains("Allow: GET, HEAD\r\n"));
        assert_eq!(status(b"get / HTTP/1.1\r\n\r\n"), Some(405));
        let long = format!("/hls/{}", "a".repeat(256));
        assert_eq!(status(&get(&long)), Some(414));
        assert_eq!(status(b"GET /\0 HTTP/1.1\r\n\r\n"), Some(400));
        assert_eq!(status(b"\x16\x03\x01\x02\x00\x01\x00"), Some(400));
        let many = format!(
            "GET / HTTP/1.1\r\n{}\r\n",
            "A: b\r\n".repeat(MAX_HEADERS + 1)
        );
        assert_eq!(status(many.as_bytes()), Some(431));
        assert_eq!(status(b"GET /hls/ HTTP/1.1\r\nHost:"), None);
        assert_eq!(status(b""), None);
    }

    #[test]
    fn arbitrary_request_heads_never_panic() {
        let f = fixture();
        let h = http(&f.routes, limits());
        let mut rng = det_rng(5);
        let valid = get(f.stream.path());
        for i in 0..20_000 {
            let mut input = if i % 2 == 0 {
                valid.clone()
            } else {
                let mut b = vec![0u8; (rng.next_u32() % 600) as usize];
                rng.fill_bytes(&mut b);
                b
            };
            if !input.is_empty() {
                for _ in 0..1 + rng.next_u32() % 4 {
                    let at = (rng.next_u32() as usize) % input.len();
                    input[at] = rng.next_u32() as u8;
                }
                input.truncate((rng.next_u32() as usize) % (input.len() + 1));
            }
            let _ = h.answer(&input);
        }
    }

    /// A server on localhost under `limits`.
    async fn serving(routes: &HlsRoutes, limits: Limits) -> (Rc<Http>, SocketAddr) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let h = Rc::new(http(routes, limits));
        spawn_local(accept(listener, h.clone()));
        (h, addr)
    }

    /// Sends `req` and reads until the server closes or resets the
    /// connection.
    async fn exchange(addr: SocketAddr, req: &[u8]) -> Vec<u8> {
        let mut s = TcpStream::connect(addr).await.unwrap();
        let mut out = Vec::new();
        if s.write_all(req).await.is_ok() {
            let read = timeout(StdDuration::from_secs(10), s.read_to_end(&mut out)).await;
            assert!(read.is_ok(), "the server didn't close the connection");
        }
        out
    }

    fn run(f: impl Future<Output = ()>) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(LocalSet::new().run_until(f));
    }

    #[test]
    fn a_connection_carries_one_answer() {
        run(async {
            let f = fixture();
            let (h, addr) = serving(&f.routes, limits()).await;
            let segment = f.stream.path().replace("index.m3u8", "");
            let playlist =
                String::from_utf8(f.routes.get(h.timer.now(), f.stream.path()).body.to_vec())
                    .unwrap();
            let seq = playlist.lines().find(|l| l.ends_with(".m4s")).unwrap();
            let out = exchange(addr, &get(&format!("{segment}{seq}"))).await;
            let text = String::from_utf8_lossy(&out);
            assert!(text.starts_with("HTTP/1.1 200 OK\r\n"), "{text}");
            assert!(
                text.contains("Content-Type: video/iso.segment\r\n"),
                "{text}"
            );
            assert!(text.ends_with("\r\n\r\nmedia"), "{text}");

            // A pipelined request is read and dropped.
            let mut two = get(f.stream.path());
            two.extend_from_slice(&get(f.stream.path()));
            let out = exchange(addr, &two).await;
            let text = String::from_utf8_lossy(&out);
            assert_eq!(text.matches("HTTP/1.1").count(), 1, "{text}");
            assert!(text.ends_with(&playlist), "{text}");

            let head = format!("HEAD {} HTTP/1.1\r\n\r\n", f.stream.path());
            let out = exchange(addr, head.as_bytes()).await;
            let text = String::from_utf8_lossy(&out);
            assert!(
                text.contains(&format!("Content-Length: {}\r\n", playlist.len())),
                "{text}"
            );
            assert!(text.ends_with("Connection: close\r\n\r\n"), "{text}");
            assert!(h.connections.borrow().len() <= 1);
        });
    }

    #[test]
    fn slow_and_oversized_heads_are_cut_off() {
        run(async {
            let f = fixture();
            let limits = Limits {
                max_head_bytes: 64,
                head_timeout: StdDuration::from_millis(200),
                ..limits()
            };
            let (_h, addr) = serving(&f.routes, limits).await;
            let out = exchange(addr, b"GET / HTTP/1.1\r\nHost:").await;
            assert!(out.is_empty(), "{}", String::from_utf8_lossy(&out));
            let big = format!("GET / HTTP/1.1\r\nA: {}\r\n\r\n", "b".repeat(64));
            let out = exchange(addr, big.as_bytes()).await;
            let text = String::from_utf8_lossy(&out);
            assert!(
                text.starts_with("HTTP/1.1 431 Request Header Fields Too Large\r\n"),
                "{text}"
            );
        });
    }

    #[test]
    fn connections_past_the_limit_are_closed() {
        run(async {
            let f = fixture();
            let limits = Limits {
                max_connections: 1,
                ..limits()
            };
            let (h, addr) = serving(&f.routes, limits).await;
            let mut first = TcpStream::connect(addr).await.unwrap();
            first.write_all(b"GET / HTTP/1.1\r\n").await.unwrap();
            while h.connections.borrow().is_empty() {
                tokio::time::sleep(StdDuration::from_millis(5)).await;
            }
            let out = exchange(addr, &get(f.stream.path())).await;
            assert!(out.is_empty(), "{}", String::from_utf8_lossy(&out));

            first.write_all(b"\r\n").await.unwrap();
            let mut out = Vec::new();
            first.read_to_end(&mut out).await.unwrap();
            assert!(out.starts_with(b"HTTP/1.1 404 Not Found\r\n"));
            drop(first);
            let out = exchange(addr, &get(f.stream.path())).await;
            assert!(out.starts_with(b"HTTP/1.1 200 OK\r\n"));
        });
    }
}
