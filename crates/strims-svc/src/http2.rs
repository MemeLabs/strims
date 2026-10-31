// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HLS over HTTP/2, on the connections whose TLS handshake picked `h2`: a
//! player's playlist reloads and segment fetches take one connection, and
//! one handshake.
//!
//! Requests are answered as the HTTP/1.1 ones are ([`http`](crate::http)),
//! several at once. Bounds:
//!
//! - The connection's preface and settings come within the time its
//!   accept gave its TLS handshake and request head.
//! - At most [`MAX_STREAMS`] requests are open at once; h2 refuses the
//!   others, and its own bounds on reset streams hold.
//! - A request's header list holds at most
//!   [`HlsSettings::max_request_head_bytes`](strims_video::HlsSettings::max_request_head_bytes).
//! - A body is sent as the client's flow-control windows open, at most
//!   [`SEND_BUFFER`] of it queued per request, so a connection holds at
//!   most `MAX_STREAMS * SEND_BUFFER` of answers unsent.
//! - A response has the HTTP/1.1 one's time to be sent; then its stream is
//!   reset.
//! - A connection with no request open for the idle timeout is sent a
//!   GOAWAY, and closes.

use std::future::poll_fn;
use std::net::SocketAddr;

use bytes::Bytes;
use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use h2::server::SendResponse;
use h2::{Reason, RecvStream, SendStream};
use http::header::{self, HeaderValue};
use http::{Request, Response, StatusCode};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::{Instant, sleep_until, timeout, timeout_at};

use crate::http::{Answer, Http};

/// The most requests a connection has open at once.
pub(crate) const MAX_STREAMS: u32 = 16;
/// The most bytes of a response queued for sending.
pub(crate) const SEND_BUFFER: usize = 64 * 1024;

/// Serves HTTP/2 on `io` until the connection closes, fails or idles out;
/// its preface is due by `deadline`.
pub(crate) async fn serve<S>(http: &Http, io: S, addr: SocketAddr, deadline: Instant)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let limits = &http.limits;
    let handshake = h2::server::Builder::new()
        .max_concurrent_streams(MAX_STREAMS)
        .max_header_list_size(u32::try_from(limits.max_head_bytes).unwrap_or(u32::MAX))
        .max_send_buffer_size(SEND_BUFFER)
        .handshake::<_, Bytes>(io);
    let mut conn = match timeout_at(deadline, handshake).await {
        Ok(Ok(conn)) => conn,
        Ok(Err(e)) => {
            tracing::debug!(%addr, error = %e, "an HLS HTTP/2 handshake failed");
            return;
        }
        Err(_) => {
            tracing::debug!(%addr, "an HLS HTTP/2 handshake timed out");
            return;
        }
    };
    let mut answers = FuturesUnordered::new();
    let mut idle_from = Instant::now();
    let mut closing = false;
    loop {
        let idle = sleep_until(idle_from + limits.idle_timeout);
        tokio::select! {
            biased;
            accepted = conn.accept() => match accepted {
                Some(Ok((req, respond))) => answers.push(answer(http, req, respond, addr)),
                Some(Err(e)) => {
                    tracing::debug!(%addr, error = %e, "an HLS HTTP/2 connection failed");
                    return;
                }
                None => return,
            },
            Some(()) = answers.next(), if !answers.is_empty() => {
                if answers.is_empty() {
                    idle_from = Instant::now();
                }
            }
            () = idle, if answers.is_empty() && !closing => {
                closing = true;
                conn.graceful_shutdown();
            }
        }
    }
}

/// Answers `req` on its stream.
async fn answer(
    http: &Http,
    req: Request<RecvStream>,
    mut respond: SendResponse<Bytes>,
    addr: SocketAddr,
) {
    let path = req.uri().path_and_query().map_or("", |p| p.as_str());
    let answer = http.route(req.method().as_str(), path);
    // A request's body is never read.
    drop(req);
    let body = &answer.response.body;
    let end = !answer.body || body.is_empty();
    let mut send = match respond.send_response(head(&answer), end) {
        Ok(send) => send,
        Err(e) => {
            tracing::debug!(%addr, error = %e, "sending an HLS HTTP/2 response failed");
            return;
        }
    };
    if end {
        return;
    }
    match timeout(http.limits.response_timeout, send_body(&mut send, body)).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::debug!(%addr, error = %e, "sending an HLS HTTP/2 body failed"),
        Err(_) => {
            tracing::debug!(%addr, "sending an HLS HTTP/2 body timed out");
            send.send_reset(Reason::CANCEL);
        }
    }
}

/// The response head of `answer`.
fn head(answer: &Answer) -> Response<()> {
    let r = &answer.response;
    let mut head = Response::new(());
    *head.status_mut() =
        StatusCode::from_u16(r.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let headers = head.headers_mut();
    for (name, value) in [
        (header::CONTENT_TYPE, r.content_type),
        (header::CACHE_CONTROL, r.cache_control),
        (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
    ] {
        if let Ok(value) = HeaderValue::from_str(value) {
            headers.insert(name, value);
        }
    }
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(r.body.len()));
    if r.status == 405 {
        headers.insert(header::ALLOW, HeaderValue::from_static("GET, HEAD"));
    }
    head
}

/// Sends `body` as the stream's capacity allows, then ends the stream.
async fn send_body(send: &mut SendStream<Bytes>, body: &[u8]) -> Result<(), h2::Error> {
    let mut rest = body;
    while !rest.is_empty() {
        send.reserve_capacity(rest.len());
        let granted = match poll_fn(|cx| send.poll_capacity(cx)).await {
            Some(granted) => granted?,
            None => return Err(Reason::STREAM_CLOSED.into()),
        };
        let (chunk, tail) = rest.split_at(granted.min(rest.len()));
        if chunk.is_empty() {
            continue;
        }
        send.send_data(Bytes::copy_from_slice(chunk), tail.is_empty())?;
        rest = tail;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )]

    use std::rc::Rc;
    use std::time::Duration as StdDuration;

    use h2::client::SendRequest;
    use http::HeaderMap;
    use strims_base::time::Duration;
    use strims_node::SystemClock;
    use strims_node::app::NodeTimer;
    use strims_rpc::Timer;
    use strims_video::HlsSettings;
    use strims_video::hls::{HlsProfile, HlsRoutes, HlsStream, PlaylistConfig};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::task::{JoinHandle, LocalSet, spawn_local};
    use tokio_rustls::TlsConnector;

    use super::*;
    use crate::TlsConfig;
    use crate::http::{Limits, accept};
    use crate::tls::Tls;
    use crate::tls::testing::{client, connect, dir, self_signed, write};

    /// Routes holding one stream with one segment of [`MEDIA_LEN`] bytes,
    /// more than a stream's first flow-control window.
    struct Fixture {
        routes: HlsRoutes,
        _profile: HlsProfile,
        stream: HlsStream,
        media: Vec<u8>,
    }

    const MEDIA_LEN: usize = 1 << 20;

    fn fixture() -> Fixture {
        let routes = HlsRoutes::new();
        let profile = routes.profile(&[1; 32]).unwrap();
        let now = NodeTimer(SystemClock).now();
        let stream = profile
            .open_stream(now, [2; 32], PlaylistConfig::default())
            .unwrap();
        let media: Vec<u8> = (0..MEDIA_LEN).map(|i| (i % 251) as u8).collect();
        stream.push_segment(b"init", &media, Duration::from_millis(1500));
        Fixture {
            routes,
            _profile: profile,
            stream,
            media,
        }
    }

    fn limits() -> Limits {
        Limits::new(&HlsSettings::default())
    }

    /// A server over TLS on localhost under `limits`, and a client that
    /// trusts its certificate and offers `alpn`.
    async fn serving(
        f: &Fixture,
        limits: Limits,
        name: &str,
        alpn: &[&[u8]],
    ) -> (Rc<Http>, SocketAddr, TlsConnector) {
        let dir = dir(name);
        let (cert, key, root) = self_signed(&["localhost"]);
        let (cert, key) = write(&dir, &cert, &key);
        let (tls, _) = Tls::load(&TlsConfig::new(cert, key)).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let h = Rc::new(Http::new(
            f.routes.clone(),
            NodeTimer(SystemClock),
            limits,
            Some(Rc::new(tls)),
        ));
        spawn_local(accept(listener, h.clone()));
        (h, addr, client(&root, alpn))
    }

    /// An HTTP/2 connection to `addr`, and the task driving it.
    async fn h2_client(
        addr: SocketAddr,
        c: &TlsConnector,
    ) -> (SendRequest<Bytes>, JoinHandle<Result<(), h2::Error>>) {
        let tls = connect(c, addr, "localhost").await.unwrap();
        assert_eq!(tls.get_ref().1.alpn_protocol(), Some(&b"h2"[..]));
        let (client, conn) = h2::client::handshake(tls).await.unwrap();
        (client, spawn_local(conn))
    }

    /// The answer to a request of `path`: its status, headers and body.
    async fn fetch(
        client: &SendRequest<Bytes>,
        method: &str,
        path: &str,
    ) -> (StatusCode, HeaderMap, Vec<u8>) {
        let req = Request::builder()
            .method(method)
            .uri(format!("https://localhost{path}"))
            .body(())
            .unwrap();
        let mut ready = client.clone().ready().await.unwrap();
        let (response, _) = ready.send_request(req, true).unwrap();
        let (head, mut body) = response.await.unwrap().into_parts();
        let mut out = Vec::new();
        while let Some(chunk) = body.data().await {
            let chunk = chunk.unwrap();
            body.flow_control().release_capacity(chunk.len()).unwrap();
            out.extend_from_slice(&chunk);
        }
        (head.status, head.headers, out)
    }

    fn header<'a>(h: &'a HeaderMap, name: &str) -> &'a str {
        h.get(name).map_or("", |v| v.to_str().unwrap())
    }

    fn run(f: impl Future<Output = ()>) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(LocalSet::new().run_until(f));
    }

    #[test]
    fn http2_answers_carry_http1s_heads_and_bodies() {
        run(async {
            let f = fixture();
            let alpn: &[&[u8]] = &[b"h2", b"http/1.1"];
            let (_h, addr, c) = serving(&f, limits(), "answers", alpn).await;
            let (client, _conn) = h2_client(addr, &c).await;

            let (status, h, body) = fetch(&client, "GET", f.stream.path()).await;
            assert_eq!(status, 200);
            let playlist = String::from_utf8(body).unwrap();
            assert!(playlist.contains("#EXTINF:1.500,"), "{playlist}");
            assert_eq!(header(&h, "content-type"), "application/vnd.apple.mpegurl");
            assert_eq!(header(&h, "cache-control"), "no-cache");
            assert_eq!(header(&h, "content-length"), playlist.len().to_string());
            assert_eq!(header(&h, "access-control-allow-origin"), "*");
            assert!(h.get("connection").is_none());

            // A segment larger than the stream's window, and several at
            // once on the connection.
            let seq = playlist.lines().find(|l| l.ends_with(".m4s")).unwrap();
            let segment = f.stream.path().replace("index.m3u8", seq);
            let fetches = (0..8).map(|_| fetch(&client, "GET", &segment));
            for (status, h, body) in futures_util::future::join_all(fetches).await {
                assert_eq!(status, 200);
                assert_eq!(header(&h, "content-type"), "video/iso.segment");
                assert_eq!(header(&h, "content-length"), MEDIA_LEN.to_string());
                assert!(body == f.media, "the segment's bytes");
            }
            assert_eq!(client.current_max_send_streams(), MAX_STREAMS as usize);

            let (status, h, body) = fetch(&client, "HEAD", &segment).await;
            assert_eq!(status, 200);
            assert_eq!(header(&h, "content-length"), MEDIA_LEN.to_string());
            assert!(body.is_empty());

            let (status, h, _) = fetch(&client, "GET", "/nope").await;
            assert_eq!(status, 404);
            assert_eq!(header(&h, "content-type"), "text/plain; charset=utf-8");
            assert_eq!(header(&h, "cache-control"), "no-store");
            assert_eq!(header(&h, "access-control-allow-origin"), "*");
            let (status, h, _) = fetch(&client, "POST", f.stream.path()).await;
            assert_eq!(status, 405);
            assert_eq!(header(&h, "allow"), "GET, HEAD");
            let long = format!("/hls/{}?_HLS_msn=1", "a".repeat(250));
            assert_eq!(fetch(&client, "GET", &long).await.0, 414);
            let query = format!("{}?_HLS_msn=1", f.stream.path());
            assert_eq!(fetch(&client, "GET", &query).await.0, 200);
        });
    }

    #[test]
    fn alpn_picks_http2_and_else_http1() {
        run(async {
            let f = fixture();
            let offers: [&[&[u8]]; 3] = [&[b"h2", b"http/1.1"], &[b"http/1.1"], &[]];
            let picks: [Option<&[u8]>; 3] = [Some(b"h2"), Some(b"http/1.1"), None];
            for (alpn, picked) in offers.into_iter().zip(picks) {
                let (_h, addr, c) = serving(&f, limits(), "alpn", alpn).await;
                let mut s = connect(&c, addr, "localhost").await.unwrap();
                assert_eq!(s.get_ref().1.alpn_protocol(), picked, "{alpn:?}");
                if picked == Some(b"h2") {
                    continue;
                }
                let req = format!("GET {} HTTP/1.1\r\nHost: x\r\n\r\n", f.stream.path());
                s.write_all(req.as_bytes()).await.unwrap();
                let mut out = Vec::new();
                s.read_to_end(&mut out).await.unwrap();
                assert!(out.starts_with(b"HTTP/1.1 200 OK\r\n"), "{alpn:?}");
            }
        });
    }

    #[test]
    fn idle_http2_connections_close() {
        run(async {
            let f = fixture();
            let limits = Limits {
                idle_timeout: StdDuration::from_millis(200),
                ..limits()
            };
            let (h, addr, c) = serving(&f, limits, "idle", &[b"h2"]).await;
            let (client, conn) = h2_client(addr, &c).await;
            assert_eq!(fetch(&client, "GET", f.stream.path()).await.0, 200);
            let closed = tokio::time::timeout(StdDuration::from_secs(5), conn).await;
            assert!(closed.is_ok(), "an idle connection stays open");
            while !h.connections_empty() {
                tokio::time::sleep(StdDuration::from_millis(5)).await;
            }
        });
    }

    #[test]
    fn a_response_the_client_doesnt_take_is_reset() {
        run(async {
            let f = fixture();
            let limits = Limits {
                response_timeout: StdDuration::from_millis(300),
                ..limits()
            };
            let (_h, addr, c) = serving(&f, limits, "reset", &[b"h2"]).await;
            let (client, _conn) = h2_client(addr, &c).await;
            let seq = String::from_utf8(fetch(&client, "GET", f.stream.path()).await.2).unwrap();
            let seq = seq
                .lines()
                .find(|l| l.ends_with(".m4s"))
                .unwrap()
                .to_owned();
            let segment = f.stream.path().replace("index.m3u8", &seq);

            // The body's bytes are read, but none of the window released.
            let req = Request::get(format!("https://localhost{segment}"))
                .body(())
                .unwrap();
            let mut ready = client.clone().ready().await.unwrap();
            let (response, _) = ready.send_request(req, true).unwrap();
            let mut body = response.await.unwrap().into_body();
            let mut got = 0;
            let reset = loop {
                match body.data().await {
                    Some(Ok(chunk)) => got += chunk.len(),
                    Some(Err(e)) => break e,
                    None => panic!("the whole body came without a window"),
                }
            };
            assert_eq!(reset.reason(), Some(Reason::CANCEL), "{reset}");
            assert!(got < MEDIA_LEN, "{got}");
            // Dropped, the body gives its share of the connection's window
            // back, and the connection serves on.
            drop(body);
            assert_eq!(fetch(&client, "GET", f.stream.path()).await.0, 200);
        });
    }

    #[test]
    fn a_stalled_http2_preface_is_cut_off() {
        run(async {
            let f = fixture();
            let limits = Limits {
                head_timeout: StdDuration::from_millis(200),
                ..limits()
            };
            let (h, addr, c) = serving(&f, limits, "preface", &[b"h2"]).await;
            let mut s = connect(&c, addr, "localhost").await.unwrap();
            let mut out = Vec::new();
            let read = tokio::time::timeout(StdDuration::from_secs(5), s.read_to_end(&mut out));
            assert!(read.await.is_ok(), "a silent HTTP/2 connection stays open");
            while !h.connections_empty() {
                tokio::time::sleep(StdDuration::from_millis(5)).await;
            }
        });
    }
}
