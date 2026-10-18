// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Native HTTPS: [`NetFetch`] makes the app layer's
//! [`HttpFetch`] requests over the node's [`Net`], with rustls on the ring
//! provider.
//!
//! Each request takes a new connection: TCP through [`Net::tcp_connect`],
//! then TLS to the URL's host, which must be a DNS name, verified against
//! the roots the [`ClientConfig`] holds ([`tls_client_config`]: the
//! Mozilla roots of `webpki-roots`). Then one HTTP/1.1 exchange. The
//! client sets `Host`, `Connection: close`, `Content-Length` for a POST or
//! a body, and a `User-Agent` unless the request has one; it sends no
//! `Accept-Encoding`, follows no redirect and keeps no cookie.
//!
//! The response head is at most [`MAX_HEAD_BYTES`] with at most
//! [`MAX_HEADERS`] headers, after at most [`MAX_INTERIM`] interim (1xx)
//! responses, which are skipped. The body is delimited by
//! `Content-Length`, by chunked transfer coding or by the connection's
//! end, and holds at most the request's `max_response_bytes`. The
//! deadline is the caller's: dropping the fetch closes the connection.

use std::io;
use std::sync::Arc;

use futures_util::future::LocalBoxFuture;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use strims_directory::{FetchError, HttpFetch, HttpRequest, HttpResponse};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_rustls::TlsConnector;

use crate::Net;

/// The longest response head.
pub const MAX_HEAD_BYTES: usize = 16 * 1024;
/// The most headers a response head has.
pub const MAX_HEADERS: usize = 64;
/// The most interim responses before the final one.
pub const MAX_INTERIM: usize = 8;
/// The longest chunk-size line of a chunked body.
const MAX_CHUNK_LINE: usize = 1024;
/// The most bytes one read takes.
const READ_CHUNK: usize = 16 * 1024;

/// A TLS client config on the ring provider with the `webpki-roots`
/// roots, offering HTTP/1.1 by ALPN.
pub fn tls_client_config() -> Arc<ClientConfig> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    #[allow(clippy::expect_used)]
    // invariant: ring's provider supports rustls's safe default versions
    let builder = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("ring supports the default protocol versions");
    let mut config = builder.with_root_certificates(roots).with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Arc::new(config)
}

/// HTTPS requests over `N`'s sockets; see the [module docs](self).
#[derive(Clone)]
pub struct NetFetch<N: Net> {
    net: N,
    tls: Arc<ClientConfig>,
}

impl<N: Net> NetFetch<N> {
    /// A fetcher connecting through `net`, with TLS from `tls`.
    pub fn new(net: N, tls: Arc<ClientConfig>) -> Self {
        NetFetch { net, tls }
    }
}

impl<N: Net> HttpFetch for NetFetch<N> {
    fn fetch(&self, req: HttpRequest) -> LocalBoxFuture<'static, Result<HttpResponse, FetchError>> {
        let (net, tls) = (self.net.clone(), self.tls.clone());
        Box::pin(async move {
            req.check()?;
            let target = Target::parse(&req.url)?;
            let head = request_head(&req, &target)?;
            let tcp = net
                .tcp_connect(format!("{}:{}", target.host, target.port))
                .await
                .map_err(connection)?;
            let stream = TlsConnector::from(tls)
                .connect(target.name, tcp)
                .await
                .map_err(|e| FetchError::Tls(e.to_string()))?;
            exchange(stream, &head, &req.body, req.max_response_bytes).await
        })
    }
}

fn invalid(what: &str) -> FetchError {
    FetchError::InvalidRequest(what.to_owned())
}

fn protocol(what: impl ToString) -> FetchError {
    FetchError::Protocol(what.to_string())
}

fn connection(e: io::Error) -> FetchError {
    FetchError::Connect(e.to_string())
}

/// Where a URL's request goes.
#[derive(Debug)]
struct Target {
    host: String,
    port: u16,
    /// The path and query, `/` if empty.
    path: String,
    name: ServerName<'static>,
}

impl Target {
    /// The target of `https://host[:port][/path][?query]`; a fragment is
    /// dropped.
    fn parse(url: &str) -> Result<Target, FetchError> {
        let rest = url
            .strip_prefix("https://")
            .ok_or_else(|| invalid("not an https url"))?;
        let rest = rest.split('#').next().unwrap_or_default();
        let end = rest.find(['/', '?']).unwrap_or(rest.len());
        let (authority, path) = rest.split_at(end);
        if authority.contains('@') {
            return Err(invalid("userinfo in the url"));
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => {
                let port: u16 = port.parse().map_err(|_| invalid("invalid port"))?;
                (host, port)
            }
            None => (authority, 443),
        };
        if host.is_empty() || port == 0 {
            return Err(invalid("no host or port"));
        }
        let name = ServerName::try_from(host.to_owned()).map_err(|_| invalid("invalid host"))?;
        if !matches!(name, ServerName::DnsName(_)) {
            return Err(invalid("the host is not a DNS name"));
        }
        let path = match path.chars().next() {
            None => "/".to_owned(),
            Some('?') => format!("/{path}"),
            Some(_) => path.to_owned(),
        };
        Ok(Target {
            host: host.to_owned(),
            port,
            path,
            name,
        })
    }
}

/// The values of the headers named `name`, in any case.
fn values<'a>(headers: &'a [(String, String)], name: &'a str) -> impl Iterator<Item = &'a str> {
    headers
        .iter()
        .filter(move |(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// Headers the client sets itself.
const OWN_HEADERS: [&str; 4] = ["host", "connection", "content-length", "transfer-encoding"];

/// `req`'s head to `target`.
fn request_head(req: &HttpRequest, target: &Target) -> Result<Vec<u8>, FetchError> {
    let mut h = format!("{} {} HTTP/1.1\r\n", req.method.as_str(), target.path);
    if target.port == 443 {
        h.push_str(&format!("Host: {}\r\n", target.host));
    } else {
        h.push_str(&format!("Host: {}:{}\r\n", target.host, target.port));
    }
    h.push_str("Connection: close\r\n");
    let mut agent = false;
    for (name, value) in &req.headers {
        if OWN_HEADERS.iter().any(|o| name.eq_ignore_ascii_case(o)) {
            return Err(invalid("the client sets this header"));
        }
        agent |= name.eq_ignore_ascii_case("user-agent");
        h.push_str(&format!("{name}: {value}\r\n"));
    }
    if !agent {
        h.push_str(concat!(
            "User-Agent: strims/",
            env!("CARGO_PKG_VERSION"),
            "\r\n"
        ));
    }
    if !req.body.is_empty() || req.method == strims_directory::Method::Post {
        h.push_str(&format!("Content-Length: {}\r\n", req.body.len()));
    }
    h.push_str("\r\n");
    Ok(h.into_bytes())
}

/// Sends `head` and `body` on `stream`, and reads the response, its body
/// at most `max` bytes.
async fn exchange<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    head: &[u8],
    body: &[u8],
    max: usize,
) -> Result<HttpResponse, FetchError> {
    stream.write_all(head).await.map_err(connection)?;
    stream.write_all(body).await.map_err(connection)?;
    stream.flush().await.map_err(connection)?;
    let mut input = Input {
        stream,
        buf: Vec::new(),
        pos: 0,
    };
    for _ in 0..=MAX_INTERIM {
        let (status, headers) = input.head().await?;
        match status {
            101 => return Err(protocol("unexpected protocol switch")),
            100..=199 => continue,
            _ => {
                let body = input.body(status, &headers, max).await?;
                return Ok(HttpResponse {
                    status,
                    headers,
                    body,
                });
            }
        }
    }
    Err(protocol("too many interim responses"))
}

/// A response being read: the bytes read and not yet taken are
/// `buf[pos..]`.
struct Input<S> {
    stream: S,
    buf: Vec<u8>,
    pos: usize,
}

impl<S: AsyncRead + Unpin> Input<S> {
    fn available(&self) -> &[u8] {
        self.buf.get(self.pos..).unwrap_or_default()
    }

    fn consume(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.buf.len());
    }

    /// Reads up to [`READ_CHUNK`] more bytes: how many, 0 at the end. A
    /// TLS connection closed without `close_notify` ends like any other;
    /// the framing tells a truncated body.
    async fn fill(&mut self) -> Result<usize, FetchError> {
        if self.pos > 0 {
            self.buf.drain(..self.pos);
            self.pos = 0;
        }
        let start = self.buf.len();
        self.buf.resize(start + READ_CHUNK, 0);
        let res = match self.buf.get_mut(start..) {
            Some(free) => self.stream.read(free).await,
            None => Ok(0),
        };
        let n = match res {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => 0,
            Err(e) => {
                self.buf.truncate(start);
                return Err(connection(e));
            }
        };
        self.buf.truncate(start + n);
        Ok(n)
    }

    /// The next response head: its status and headers.
    async fn head(&mut self) -> Result<(u16, Vec<(String, String)>), FetchError> {
        loop {
            let avail = self.available();
            let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
            let mut resp = httparse::Response::new(&mut headers);
            match resp.parse(avail).map_err(protocol)? {
                httparse::Status::Complete(n) => {
                    if n > MAX_HEAD_BYTES {
                        return Err(protocol("response head too large"));
                    }
                    let status = resp.code.ok_or_else(|| protocol("no status"))?;
                    let headers = resp
                        .headers
                        .iter()
                        .map(|h| {
                            let value = String::from_utf8_lossy(h.value).into_owned();
                            (h.name.to_owned(), value)
                        })
                        .collect();
                    self.consume(n);
                    return Ok((status, headers));
                }
                httparse::Status::Partial => {
                    if avail.len() >= MAX_HEAD_BYTES {
                        return Err(protocol("response head too large"));
                    }
                }
            }
            if self.fill().await? == 0 {
                return Err(protocol("the connection closed before the response head"));
            }
        }
    }

    /// The body of a response with `status` and `headers`, at most `max`
    /// bytes.
    async fn body(
        &mut self,
        status: u16,
        headers: &[(String, String)],
        max: usize,
    ) -> Result<Vec<u8>, FetchError> {
        if status == 204 || status == 304 {
            return Ok(Vec::new());
        }
        let codings: Vec<String> = values(headers, "transfer-encoding")
            .flat_map(|v| v.split(','))
            .map(|c| c.trim().to_ascii_lowercase())
            .filter(|c| !c.is_empty())
            .collect();
        if !codings.is_empty() {
            if codings != ["chunked"] {
                return Err(protocol("unsupported transfer coding"));
            }
            return self.chunked(max).await;
        }
        let mut length: Option<u64> = None;
        for v in values(headers, "content-length") {
            let l: u64 = v
                .trim()
                .parse()
                .map_err(|_| protocol("invalid content length"))?;
            if length.is_some_and(|prev| prev != l) {
                return Err(protocol("conflicting content lengths"));
            }
            length = Some(l);
        }
        let mut out = Vec::new();
        match length {
            Some(l) => {
                let l = usize::try_from(l)
                    .ok()
                    .filter(|l| *l <= max)
                    .ok_or(FetchError::TooLarge)?;
                self.take(l, &mut out).await?;
            }
            None => self.rest(max, &mut out).await?,
        }
        Ok(out)
    }

    /// Appends the next `n` bytes to `out`.
    async fn take(&mut self, n: usize, out: &mut Vec<u8>) -> Result<(), FetchError> {
        let mut left = n;
        while left > 0 {
            let avail = self.available();
            if avail.is_empty() {
                if self.fill().await? == 0 {
                    return Err(protocol("truncated body"));
                }
                continue;
            }
            let k = avail.len().min(left);
            out.extend_from_slice(avail.get(..k).unwrap_or_default());
            self.consume(k);
            left -= k;
        }
        Ok(())
    }

    /// Appends everything up to the connection's end to `out`, refusing
    /// more than `max` bytes in all.
    async fn rest(&mut self, max: usize, out: &mut Vec<u8>) -> Result<(), FetchError> {
        loop {
            let avail = self.available();
            if out.len() + avail.len() > max {
                return Err(FetchError::TooLarge);
            }
            out.extend_from_slice(avail);
            let n = avail.len();
            self.consume(n);
            if self.fill().await? == 0 {
                return Ok(());
            }
        }
    }

    /// The next line, without its CRLF or LF, at most `max` bytes. `None`
    /// at the connection's end.
    async fn line(&mut self, max: usize) -> Result<Option<Vec<u8>>, FetchError> {
        loop {
            let avail = self.available();
            if let Some(i) = avail.iter().position(|b| *b == b'\n') {
                let mut line = avail.get(..i).unwrap_or_default().to_vec();
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                self.consume(i + 1);
                if line.len() > max {
                    return Err(protocol("line too long"));
                }
                return Ok(Some(line));
            }
            if avail.len() > max + 1 {
                return Err(protocol("line too long"));
            }
            if self.fill().await? == 0 {
                return Ok(None);
            }
        }
    }

    /// A chunked body of at most `max` bytes, and its trailers, which are
    /// dropped.
    async fn chunked(&mut self, max: usize) -> Result<Vec<u8>, FetchError> {
        let truncated = || protocol("truncated chunked body");
        let mut out = Vec::new();
        loop {
            let line = self.line(MAX_CHUNK_LINE).await?.ok_or_else(truncated)?;
            let size = line.split(|b| *b == b';').next().unwrap_or_default();
            let size = std::str::from_utf8(size)
                .ok()
                .map(str::trim)
                .filter(|s| !s.is_empty() && s.len() <= 16)
                .and_then(|s| u64::from_str_radix(s, 16).ok())
                .ok_or_else(|| protocol("invalid chunk size"))?;
            if size == 0 {
                break;
            }
            let size = usize::try_from(size)
                .ok()
                .filter(|s| out.len().saturating_add(*s) <= max)
                .ok_or(FetchError::TooLarge)?;
            self.take(size, &mut out).await?;
            match self.line(0).await? {
                Some(l) if l.is_empty() => {}
                Some(_) => return Err(protocol("missing chunk end")),
                None => return Err(truncated()),
            }
        }
        let mut trailers = 0usize;
        while let Some(line) = self.line(MAX_HEAD_BYTES).await? {
            if line.is_empty() {
                break;
            }
            trailers += line.len() + 2;
            if trailers > MAX_HEAD_BYTES {
                return Err(protocol("trailers too large"));
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

    use strims_directory::Method;
    use tokio::io::{DuplexStream, duplex};

    use super::*;

    /// Runs one exchange of `req` against a server that answers `reply`
    /// and then closes: the response, and the request bytes the server
    /// read.
    async fn round_trip(
        req: &HttpRequest,
        reply: Vec<u8>,
    ) -> (Result<HttpResponse, FetchError>, Vec<u8>) {
        let target = Target::parse(&req.url).unwrap();
        let head = request_head(req, &target).unwrap();
        let (client, mut server) = duplex(64 * 1024);
        let server = tokio::spawn(async move {
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            while !got.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = server.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                got.extend_from_slice(&buf[..n]);
            }
            // a body the head announces
            let len = String::from_utf8_lossy(&got)
                .lines()
                .find_map(|l| {
                    l.strip_prefix("Content-Length: ")
                        .map(|v| v.parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let head_end = got.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
            while got.len() < head_end + len {
                let n = server.read(&mut buf).await.unwrap();
                got.extend_from_slice(&buf[..n]);
            }
            // the client may have stopped reading
            let _ = server.write_all(&reply).await;
            drop::<DuplexStream>(server);
            got
        });
        let res = exchange(client, &head, &req.body, req.max_response_bytes).await;
        (res, server.await.unwrap())
    }

    fn get(url: &str) -> HttpRequest {
        HttpRequest::new(Method::Get, url, 64)
    }

    #[tokio::test]
    async fn a_get_sends_its_head_and_reads_a_sized_body() {
        let req = get("https://api.example.com/v3/streams/?username=a%26b").header("Accept", "x");
        let reply =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\nhello"
                .to_vec();
        let (res, sent) = round_trip(&req, reply).await;
        let res = res.unwrap();
        assert_eq!((res.status, res.body.as_slice()), (200, &b"hello"[..]));
        assert_eq!(res.header("content-type"), Some("text/plain"));
        assert_eq!(
            String::from_utf8(sent).unwrap(),
            format!(
                "GET /v3/streams/?username=a%26b HTTP/1.1\r\nHost: api.example.com\r\nConnection: close\r\nAccept: x\r\nUser-Agent: strims/{}\r\n\r\n",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    #[tokio::test]
    async fn a_post_sends_its_body_with_its_length() {
        let req = HttpRequest::new(Method::Post, "https://id.example.com:8443/token", 64)
            .body("application/x-www-form-urlencoded", b"a=b".to_vec());
        let reply = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}".to_vec();
        let (res, sent) = round_trip(&req, reply).await;
        assert_eq!(res.unwrap().body, b"{}");
        let sent = String::from_utf8(sent).unwrap();
        assert!(
            sent.starts_with("POST /token HTTP/1.1\r\nHost: id.example.com:8443\r\n"),
            "{sent}"
        );
        assert!(sent.ends_with("Content-Length: 3\r\n\r\na=b"), "{sent}");
        assert!(!sent.to_ascii_lowercase().contains("accept-encoding"));
    }

    #[tokio::test]
    async fn chunked_bodies_are_joined_and_their_trailers_dropped() {
        let reply = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5;ext=1\r\nhello\r\n1\r\n \r\n5\r\nworld\r\n0\r\nX-Trailer: y\r\n\r\n".to_vec();
        let (res, _) = round_trip(&get("https://x.example/"), reply).await;
        assert_eq!(res.unwrap().body, b"hello world");
    }

    #[tokio::test]
    async fn a_body_without_a_length_ends_with_the_connection() {
        let reply = b"HTTP/1.1 200 OK\r\n\r\nall of it".to_vec();
        let (res, _) = round_trip(&get("https://x.example/"), reply).await;
        assert_eq!(res.unwrap().body, b"all of it");
        let reply = b"HTTP/1.1 204 No Content\r\n\r\n".to_vec();
        let (res, _) = round_trip(&get("https://x.example/"), reply).await;
        assert_eq!(res.unwrap().body, b"");
    }

    #[tokio::test]
    async fn interim_responses_are_skipped() {
        let reply = b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 103 Early Hints\r\nLink: x\r\n\r\nHTTP/1.1 404 Not Found\r\nContent-Length: 1\r\n\r\nx".to_vec();
        let (res, _) = round_trip(&get("https://x.example/"), reply).await;
        let res = res.unwrap();
        assert_eq!((res.status, res.body.as_slice()), (404, &b"x"[..]));
        let mut many = b"HTTP/1.1 100 Continue\r\n\r\n".repeat(MAX_INTERIM + 1);
        many.extend_from_slice(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
        let (res, _) = round_trip(&get("https://x.example/"), many).await;
        assert!(matches!(res, Err(FetchError::Protocol(_))), "{res:?}");
    }

    #[tokio::test]
    async fn bodies_over_the_cap_are_refused() {
        let big = "x".repeat(65);
        for reply in [
            format!("HTTP/1.1 200 OK\r\nContent-Length: 65\r\n\r\n{big}"),
            "HTTP/1.1 200 OK\r\nContent-Length: 1000000\r\n\r\n".to_owned(),
            format!(
                "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n20\r\n{}\r\n21\r\n{}\r\n0\r\n\r\n",
                &big[..32],
                &big[..33]
            ),
            format!("HTTP/1.1 200 OK\r\n\r\n{big}"),
        ] {
            let (res, _) = round_trip(&get("https://x.example/"), reply.clone().into_bytes()).await;
            assert_eq!(res, Err(FetchError::TooLarge), "{reply}");
        }
        let exact = format!("HTTP/1.1 200 OK\r\n\r\n{}", &big[..64]);
        let (res, _) = round_trip(&get("https://x.example/"), exact.into_bytes()).await;
        assert_eq!(res.unwrap().body.len(), 64);
    }

    #[tokio::test]
    async fn malformed_responses_are_refused() {
        let long_head = format!(
            "HTTP/1.1 200 OK\r\nX: {}\r\n\r\n",
            "a".repeat(MAX_HEAD_BYTES)
        );
        let mut many_headers = "HTTP/1.1 200 OK\r\n".to_owned();
        for i in 0..=MAX_HEADERS {
            many_headers.push_str(&format!("X-{i}: v\r\n"));
        }
        many_headers.push_str("\r\n");
        for reply in [
            long_head,
            many_headers,
            "SSH-2.0-OpenSSH\r\n\r\n".to_owned(),
            "HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nshort".to_owned(),
            "HTTP/1.1 200 OK\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\nab".to_owned(),
            "HTTP/1.1 200 OK\r\nContent-Length: -1\r\n\r\n".to_owned(),
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip, chunked\r\n\r\n0\r\n\r\n".to_owned(),
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n".to_owned(),
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhelloXX\r\n0\r\n\r\n"
                .to_owned(),
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhel".to_owned(),
            "HTTP/1.1 101 Switching Protocols\r\n\r\n".to_owned(),
            "HTTP/1.1 200".to_owned(),
        ] {
            let (res, _) = round_trip(&get("https://x.example/"), reply.clone().into_bytes()).await;
            assert!(
                matches!(res, Err(FetchError::Protocol(_))),
                "{reply:?}: {res:?}"
            );
        }
    }

    #[test]
    fn targets_need_a_dns_name_over_https() {
        let t = Target::parse("https://api.twitch.tv/helix/users?login=a").unwrap();
        assert_eq!(
            (t.host.as_str(), t.port, t.path.as_str()),
            ("api.twitch.tv", 443, "/helix/users?login=a")
        );
        let t = Target::parse("https://localhost:8443?x=1#frag").unwrap();
        assert_eq!(
            (t.host.as_str(), t.port, t.path.as_str()),
            ("localhost", 8443, "/?x=1")
        );
        assert_eq!(Target::parse("https://example.com").unwrap().path, "/");
        for bad in [
            "http://example.com/",
            "https://1.2.3.4/",
            "https://[::1]/",
            "https://user@example.com/",
            "https://example.com:0/",
            "https://example.com:99999/",
            "https://example.com:x/",
            "https:///path",
            "https://exa_mple .com/",
        ] {
            assert!(
                matches!(Target::parse(bad), Err(FetchError::InvalidRequest(_))),
                "{bad}"
            );
        }
    }

    #[test]
    fn the_client_sets_its_own_framing_headers() {
        let t = Target::parse("https://x.example/").unwrap();
        for name in ["Host", "connection", "Content-Length", "Transfer-Encoding"] {
            let req = get("https://x.example/").header(name, "v");
            assert!(
                matches!(request_head(&req, &t), Err(FetchError::InvalidRequest(_))),
                "{name}"
            );
        }
        let req = get("https://x.example/").header("User-Agent", "mine");
        let head = String::from_utf8(request_head(&req, &t).unwrap()).unwrap();
        assert_eq!(head.matches("User-Agent").count(), 1);
    }

    #[test]
    fn the_client_config_offers_http_1_1_on_ring() {
        let c = tls_client_config();
        assert_eq!(c.alpn_protocols, [b"http/1.1".to_vec()]);
    }
}
