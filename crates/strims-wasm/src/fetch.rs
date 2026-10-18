// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTPS in the browser: [`BrowserFetch`] makes the directory's
//! [`HttpFetch`] requests with the global `fetch`.
//!
//! # Requests
//!
//! A request runs [`HttpRequest::check`] first, then the URL rules a
//! native fetcher applies: no userinfo, a valid port, and a host that is a
//! DNS name, not an IP address. A GET has no body, since `fetch` refuses
//! one. Each request is sent with `mode: "cors"`, `credentials: "omit"`,
//! `cache: "no-store"`, `referrerPolicy: "no-referrer"` and
//! `redirect: "manual"`, and is aborted through its `AbortController`
//! when the future is dropped: the deadline is the caller's.
//!
//! The request's headers go as they are, except:
//!
//! - `Host`, `Connection`, `Content-Length` and `Transfer-Encoding` are
//!   the client's own: a request that sets one fails with
//!   [`FetchError::InvalidRequest`].
//! - Headers the browser sets itself or refuses from scripts are dropped
//!   silently ([`DROPPED_HEADERS`], and every `Proxy-*` and `Sec-*` name):
//!   among them `User-Agent`, which the browser sends its own of, and
//!   `Accept-Encoding`. The browser negotiates the encoding and decodes
//!   the body, and the cap holds for the decoded body.
//!
//! The browser also adds `Origin`, and a request that isn't a CORS
//! "simple request" (a header such as `Authorization`, or a POST body of
//! another type) is preflighted.
//!
//! # Responses
//!
//! A response resolves with its status and the headers CORS exposes to
//! scripts: the safelisted ones (`Content-Type`, `Content-Length`,
//! `Cache-Control`, …) and those the server names in
//! `Access-Control-Expose-Headers`, at most [`MAX_HEADERS`] of them in at
//! most [`MAX_HEAD_BYTES`]. Names are lower case, and repeated headers
//! come joined with `", "`.
//!
//! The body is read chunk by chunk from its `ReadableStream`, and holds at
//! most the request's `max_response_bytes`: a `Content-Length` over the
//! cap fails at once, and a body that grows past it fails as soon as it
//! does, both with [`FetchError::TooLarge`]. A redirect isn't followed,
//! and the browser hides the redirect response, so it fails with
//! [`FetchError::Protocol`].
//!
//! # CORS: no proxy
//!
//! `fetch` rejects a request the server's CORS headers refuse with the
//! same `TypeError` as an unreachable host, so both fail with
//! [`FetchError::Connect`] "refused or unreachable (CORS)". Nothing relays
//! a browser's requests: a directory server hosted in a browser loads
//! snippets only from APIs that allow cross-origin requests, serves the
//! other listings without snippets, and tries again at its next load.
//! Directory servers that need every embed's snippet run on native nodes.

use futures_util::future::LocalBoxFuture;
use js_sys::{Array, Promise, Uint8Array};
use strims_directory::fetch::{FetchError, HttpFetch, HttpRequest, HttpResponse, Method};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AbortController, Headers, ReadableStreamDefaultReader, ReadableStreamReadResult,
    ReferrerPolicy, Request, RequestCache, RequestCredentials, RequestInit, RequestMode,
    RequestRedirect, Response, ResponseType,
};

use crate::js;

/// The most response headers taken.
pub const MAX_HEADERS: usize = 64;
/// The most bytes of response header names and values taken, counting
/// four more per header for its separators.
pub const MAX_HEAD_BYTES: usize = 16 * 1024;

/// Headers the client sets itself; a request may not.
const OWN_HEADERS: [&str; 4] = ["host", "connection", "content-length", "transfer-encoding"];

/// Request headers dropped because the browser sets them itself or
/// refuses them from scripts, besides every `proxy-*` and `sec-*` name.
pub const DROPPED_HEADERS: [&str; 19] = [
    "accept-charset",
    "accept-encoding",
    "access-control-request-headers",
    "access-control-request-method",
    "access-control-request-private-network",
    "cookie",
    "cookie2",
    "date",
    "dnt",
    "expect",
    "keep-alive",
    "origin",
    "referer",
    "set-cookie",
    "te",
    "trailer",
    "upgrade",
    "user-agent",
    "via",
];

/// What a `TypeError` from `fetch` means.
const REFUSED: &str = "refused or unreachable (CORS)";

#[wasm_bindgen]
extern "C" {
    /// The global `fetch`, of a worker or a window.
    #[wasm_bindgen(js_name = fetch)]
    fn global_fetch(request: &Request) -> Promise;
}

/// HTTPS requests with the browser's `fetch`; see the
/// [module docs](self).
#[derive(Clone, Copy, Debug, Default)]
pub struct BrowserFetch;

impl BrowserFetch {
    pub fn new() -> BrowserFetch {
        BrowserFetch
    }
}

impl HttpFetch for BrowserFetch {
    fn fetch(&self, req: HttpRequest) -> LocalBoxFuture<'static, Result<HttpResponse, FetchError>> {
        Box::pin(async move {
            req.check()?;
            check_url(&req.url)?;
            send(req).await
        })
    }
}

fn invalid(what: &str) -> FetchError {
    FetchError::InvalidRequest(what.to_owned())
}

fn protocol(what: impl ToString) -> FetchError {
    FetchError::Protocol(what.to_string())
}

/// Checks the authority of an `https://` URL: no userinfo, a port in
/// range, and a DNS name for the host.
fn check_url(url: &str) -> Result<(), FetchError> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| invalid("not an https url"))?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = rest.get(..end).unwrap_or_default();
    if authority.contains('@') {
        return Err(invalid("userinfo in the url"));
    }
    if authority.starts_with('[') {
        return Err(invalid("the host is not a DNS name"));
    }
    let host = match authority.rsplit_once(':') {
        Some((host, port)) => match port.parse::<u16>() {
            Ok(0) => return Err(invalid("no host or port")),
            Ok(_) => host,
            Err(_) => return Err(invalid("invalid port")),
        },
        None => authority,
    };
    if host.is_empty() {
        return Err(invalid("no host or port"));
    }
    let name = host.strip_suffix('.').unwrap_or(host);
    let label_ok = |l: &str| {
        !l.is_empty()
            && l.len() <= 63
            && l.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    };
    if name.len() > 253 || !name.split('.').all(label_ok) {
        return Err(invalid("invalid host"));
    }
    // A URL host whose last label is a number is an IPv4 address.
    let last = name.rsplit('.').next().unwrap_or_default();
    let hex = last
        .strip_prefix("0x")
        .or_else(|| last.strip_prefix("0X"))
        .is_some_and(|h| h.bytes().all(|b| b.is_ascii_hexdigit()));
    if hex || last.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid("the host is not a DNS name"));
    }
    Ok(())
}

/// The headers of `req` that go to `fetch`.
fn request_headers(req: &HttpRequest) -> Result<Vec<(&str, &str)>, FetchError> {
    let mut out = Vec::with_capacity(req.headers.len());
    for (name, value) in &req.headers {
        let lower = name.to_ascii_lowercase();
        if OWN_HEADERS.contains(&lower.as_str()) {
            return Err(invalid("the client sets this header"));
        }
        let dropped = DROPPED_HEADERS.contains(&lower.as_str())
            || lower.starts_with("proxy-")
            || lower.starts_with("sec-");
        if !dropped {
            out.push((name.as_str(), value.as_str()));
        }
    }
    Ok(out)
}

/// Aborts its request when dropped. A request that finished ignores it.
struct AbortOnDrop(AbortController);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Sends `req`, whose checks have passed, and reads the response.
async fn send(req: HttpRequest) -> Result<HttpResponse, FetchError> {
    if req.method == Method::Get && !req.body.is_empty() {
        return Err(invalid("a GET has no body"));
    }
    let controller = AbortController::new().map_err(|e| FetchError::Connect(js::describe(&e)))?;
    let abort = AbortOnDrop(controller);
    let headers = Headers::new().map_err(|e| invalid(&js::describe(&e)))?;
    for (name, value) in request_headers(&req)? {
        // Refuses values with characters past U+00FF.
        headers
            .append(name, value)
            .map_err(|_| invalid("invalid header value"))?;
    }
    let init = RequestInit::new();
    init.set_method(req.method.as_str());
    init.set_mode(RequestMode::Cors);
    init.set_credentials(RequestCredentials::Omit);
    init.set_cache(RequestCache::NoStore);
    init.set_referrer_policy(ReferrerPolicy::NoReferrer);
    init.set_redirect(RequestRedirect::Manual);
    init.set_signal(Some(&abort.0.signal()));
    init.set_headers(&headers);
    if !req.body.is_empty() {
        init.set_body(&Uint8Array::from(req.body.as_slice()));
    }
    let request = Request::new_with_str_and_init(&req.url, &init)
        .map_err(|e| FetchError::InvalidRequest(js::describe(&e)))?;
    let response: Response = JsFuture::from(global_fetch(&request))
        .await
        .map_err(|e| fetch_error(&e))?
        .unchecked_into();
    if response.type_() == ResponseType::Opaqueredirect {
        return Err(protocol("redirect not followed"));
    }
    let headers = response_headers(&response.headers())?;
    let length = response
        .headers()
        .get("content-length")
        .ok()
        .flatten()
        .and_then(|l| l.trim().parse::<u64>().ok());
    let max = u64::try_from(req.max_response_bytes).unwrap_or(u64::MAX);
    if length.is_some_and(|l| l > max) {
        return Err(FetchError::TooLarge);
    }
    let body = read_body(&response, req.max_response_bytes).await?;
    drop(abort);
    Ok(HttpResponse {
        status: response.status(),
        headers,
        body,
    })
}

/// A rejection from `fetch` as a fetch error.
fn fetch_error(e: &JsValue) -> FetchError {
    match js::error_name(e).as_deref() {
        Some("TypeError") => FetchError::Connect(REFUSED.to_owned()),
        _ => FetchError::Connect(js::describe(e)),
    }
}

/// The response's headers, within [`MAX_HEADERS`] and [`MAX_HEAD_BYTES`].
fn response_headers(headers: &Headers) -> Result<Vec<(String, String)>, FetchError> {
    let too_large = || protocol("response head too large");
    let mut out = Vec::new();
    let mut bytes = 0usize;
    let entries = headers.entries();
    loop {
        let next = entries.next().map_err(|e| protocol(js::describe(&e)))?;
        if next.done() {
            return Ok(out);
        }
        let entry: Array = next.value().unchecked_into();
        let name = entry.get(0).as_string().unwrap_or_default();
        let value = entry.get(1).as_string().unwrap_or_default();
        bytes = bytes.saturating_add(name.len() + value.len() + 4);
        if out.len() == MAX_HEADERS || bytes > MAX_HEAD_BYTES {
            return Err(too_large());
        }
        out.push((name, value));
    }
}

/// The response's body, of at most `max` bytes.
async fn read_body(response: &Response, max: usize) -> Result<Vec<u8>, FetchError> {
    let Some(stream) = response.body() else {
        return Ok(Vec::new());
    };
    let reader =
        ReadableStreamDefaultReader::new(&stream).map_err(|e| protocol(js::describe(&e)))?;
    let mut out = Vec::new();
    loop {
        let read: ReadableStreamReadResult = JsFuture::from(reader.read())
            .await
            .map_err(|e| FetchError::Connect(js::describe(&e)))?
            .unchecked_into();
        if read.get_done().unwrap_or(false) {
            return Ok(out);
        }
        let chunk: Uint8Array = read
            .get_value()
            .dyn_into()
            .map_err(|_| protocol("a body chunk that isn't bytes"))?;
        let start = out.len();
        let end = start.saturating_add(chunk.length() as usize);
        if end > max {
            return Err(FetchError::TooLarge);
        }
        out.resize(end, 0);
        if let Some(dst) = out.get_mut(start..) {
            chunk.copy_to(dst);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use futures_util::FutureExt;
    use js_sys::Reflect;
    use wasm_bindgen_test::wasm_bindgen_test;

    use super::*;

    /// The test server's origin. It serves the crate's files.
    fn origin() -> String {
        let location = Reflect::get(&js_sys::global(), &"location".into()).unwrap();
        Reflect::get(&location, &"origin".into())
            .unwrap()
            .as_string()
            .unwrap()
    }

    fn get(url: &str, max: usize) -> HttpRequest {
        HttpRequest::new(Method::Get, url, max)
    }

    const MANIFEST: &[u8] = include_bytes!("../Cargo.toml");

    #[wasm_bindgen_test]
    async fn a_get_reads_the_status_headers_and_body() {
        let url = format!("{}/Cargo.toml", origin());
        let res = send(get(&url, MANIFEST.len()).header("Accept", "*/*"))
            .await
            .unwrap();
        assert_eq!(res.status, 200);
        assert_eq!(res.body, MANIFEST);
        let length = res.header("content-length").unwrap();
        assert_eq!(length, MANIFEST.len().to_string());
        assert!(
            res.headers
                .iter()
                .all(|(n, _)| *n == n.to_ascii_lowercase())
        );

        let res = send(get(&format!("{}/no-such-file", origin()), 64))
            .await
            .unwrap();
        assert_eq!(res.status, 404);
    }

    #[wasm_bindgen_test]
    async fn bodies_over_the_cap_are_refused() {
        let url = format!("{}/Cargo.toml", origin());
        let res = send(get(&url, MANIFEST.len() - 1)).await;
        assert_eq!(res, Err(FetchError::TooLarge));
    }

    #[wasm_bindgen_test]
    async fn an_unreachable_host_is_a_connection_error() {
        let res = BrowserFetch.fetch(get("https://localhost:1/", 64)).await;
        assert_eq!(res, Err(FetchError::Connect(REFUSED.to_owned())));
    }

    #[wasm_bindgen_test]
    async fn a_dropped_fetch_is_aborted() {
        let url = format!("{}/Cargo.toml", origin());
        let mut pending = Box::pin(send(get(&url, MANIFEST.len())));
        assert!((&mut pending).now_or_never().is_none());
        drop(pending);
        let res = send(get(&url, MANIFEST.len())).await.unwrap();
        assert_eq!(res.body, MANIFEST);
    }

    #[wasm_bindgen_test]
    async fn requests_a_native_client_refuses_are_refused() {
        let post = |url: &str| HttpRequest::new(Method::Post, url, 64);
        for bad in [
            get("http://example.com/", 64),
            get("https://user@example.com/", 64),
            get("https://example.com:0/", 64),
            get("https://example.com:99999/", 64),
            get("https://example.com:x/", 64),
            get("https:///path", 64),
            get("https://1.2.3.4/", 64),
            get("https://127.1/", 64),
            get("https://0x7f.1/", 64),
            get("https://[::1]/", 64),
            get("https://exa%6dple.com/", 64),
            get("https://example.com/", 64).header("Host", "x"),
            get("https://example.com/", 64).header("content-length", "1"),
            get("https://example.com/", 64).header("X-A", "€"),
            post("https://example.com/").header("Transfer-Encoding", "chunked"),
            HttpRequest {
                body: b"x".to_vec(),
                ..get("https://example.com/", 64)
            },
        ] {
            let res = BrowserFetch.fetch(bad.clone()).await;
            assert!(
                matches!(res, Err(FetchError::InvalidRequest(_))),
                "{bad:?}: {res:?}"
            );
        }
    }

    #[wasm_bindgen_test]
    fn hosts_must_be_dns_names() {
        for ok in [
            "https://api.twitch.tv/helix/users?login=a",
            "https://localhost:8443?x=1#frag",
            "https://example.com",
            "https://example.com./",
            "https://a_b.example.com/",
            "https://1.example.com/",
        ] {
            assert_eq!(check_url(ok), Ok(()), "{ok}");
        }
    }

    #[wasm_bindgen_test]
    fn headers_the_browser_sets_are_dropped() {
        let req = get("https://example.com/", 64)
            .header("Accept", "x")
            .header("User-Agent", "strims")
            .header("Accept-Encoding", "gzip")
            .header("Sec-Fetch-Mode", "cors")
            .header("Proxy-Authorization", "x")
            .header("Authorization", "Bearer y");
        assert_eq!(
            request_headers(&req).unwrap(),
            [("Accept", "x"), ("Authorization", "Bearer y")]
        );
        for name in ["Host", "connection", "Content-Length", "Transfer-Encoding"] {
            let req = get("https://example.com/", 64).header(name, "v");
            assert!(matches!(
                request_headers(&req),
                Err(FetchError::InvalidRequest(_))
            ));
        }
    }
}
