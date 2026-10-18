// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTPS requests the directory makes, through a fetcher the driver
//! supplies: a native client, or the browser's fetch.
//!
//! A request is a GET or POST of an `https://` URL whose query is already
//! percent-encoded ([`encode_component`], [`query`]), with headers, a body
//! and a cap on the response body. [`HttpRequest::check`] refuses what
//! would let a value smuggle in another header or request: control
//! characters in the URL, a header name that isn't a token, a header value
//! with CR, LF or NUL. Fetchers run it before sending.
//!
//! A fetch is a future the caller drops to cancel; the deadline is the
//! caller's. [`Fetch`] holds the driver's fetcher, [`NoFetch`] by default,
//! which refuses every request.

use std::fmt;
use std::rc::Rc;

use futures_util::future::{LocalBoxFuture, ready};

/// The longest URL a request may have.
pub const MAX_URL_BYTES: usize = 8 * 1024;

/// A request's method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
        }
    }
}

/// An HTTPS request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: Method,
    /// An `https://` URL, percent-encoded.
    pub url: String,
    /// Headers besides those the fetcher sets itself (`Host`,
    /// `Connection`, `Content-Length`).
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// The longest response body the caller takes. A longer one fails the
    /// fetch with [`FetchError::TooLarge`].
    pub max_response_bytes: usize,
}

impl HttpRequest {
    /// A request of `url` with no headers or body.
    pub fn new(method: Method, url: impl Into<String>, max_response_bytes: usize) -> HttpRequest {
        HttpRequest {
            method,
            url: url.into(),
            headers: Vec::new(),
            body: Vec::new(),
            max_response_bytes,
        }
    }

    /// The request with header `name: value` added.
    pub fn header(mut self, name: &str, value: &str) -> HttpRequest {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    /// The request with `body` as its body, of type `content_type`.
    pub fn body(self, content_type: &str, body: Vec<u8>) -> HttpRequest {
        let mut r = self.header("Content-Type", content_type);
        r.body = body;
        r
    }

    /// Checks the request against the rules in the module docs.
    pub fn check(&self) -> Result<(), FetchError> {
        let invalid = |what: &str| Err(FetchError::InvalidRequest(what.to_owned()));
        if !self.url.starts_with("https://") {
            return invalid("not an https url");
        }
        if self.url.len() > MAX_URL_BYTES {
            return invalid("url too long");
        }
        if !self.url.bytes().all(|b| (0x21..0x7f).contains(&b)) {
            return invalid("url not percent-encoded");
        }
        for (name, value) in &self.headers {
            if name.is_empty() || !name.bytes().all(is_token) {
                return invalid("invalid header name");
            }
            if value.bytes().any(|b| matches!(b, b'\r' | b'\n' | 0)) {
                return invalid("invalid header value");
            }
        }
        if self.max_response_bytes == 0 {
            return invalid("no response cap");
        }
        Ok(())
    }
}

/// Whether `b` may appear in a header name (an RFC 9110 token).
fn is_token(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

/// An HTTP response.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// The value of the first header named `name`, in any case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Why a fetch failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FetchError {
    /// The node has no fetcher.
    #[error("http fetch unsupported")]
    Unsupported,
    /// The request breaks a rule of [`HttpRequest::check`], or names a
    /// host the fetcher won't connect to.
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// No connection, or it failed.
    #[error("connection: {0}")]
    Connect(String),
    /// The TLS handshake failed.
    #[error("tls: {0}")]
    Tls(String),
    /// The response doesn't parse.
    #[error("http: {0}")]
    Protocol(String),
    /// The response body is over the request's cap.
    #[error("response too large")]
    TooLarge,
}

/// Makes HTTPS requests. Its futures aren't `Send`.
pub trait HttpFetch {
    /// Sends `req`, and resolves to the response, whatever its status.
    fn fetch(&self, req: HttpRequest) -> LocalBoxFuture<'static, Result<HttpResponse, FetchError>>;
}

/// The fetcher of a node that makes no requests.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoFetch;

impl HttpFetch for NoFetch {
    fn fetch(&self, _: HttpRequest) -> LocalBoxFuture<'static, Result<HttpResponse, FetchError>> {
        Box::pin(ready(Err(FetchError::Unsupported)))
    }
}

/// A shared fetcher. The default is [`NoFetch`].
#[derive(Clone)]
pub struct Fetch(Rc<dyn HttpFetch>);

impl Fetch {
    pub fn new(fetch: impl HttpFetch + 'static) -> Fetch {
        Fetch(Rc::new(fetch))
    }

    pub fn fetch(
        &self,
        req: HttpRequest,
    ) -> LocalBoxFuture<'static, Result<HttpResponse, FetchError>> {
        self.0.fetch(req)
    }
}

impl Default for Fetch {
    fn default() -> Fetch {
        Fetch::new(NoFetch)
    }
}

impl fmt::Debug for Fetch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Fetch")
    }
}

/// `s` percent-encoded as a URL query value or form field: every byte
/// but the RFC 3986 unreserved characters as `%XX`.
pub fn encode_component(s: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            out.push('%');
            for nibble in [b >> 4, b & 0xf] {
                if let Some(&h) = HEX.get(usize::from(nibble)) {
                    out.push(char::from(h));
                }
            }
        }
    }
    out
}

/// `pairs` as a query string or form body: `k=v` pairs joined by `&`,
/// each key and value [`encode_component`]ed.
pub fn query<K: AsRef<str>, V: AsRef<str>>(pairs: &[(K, V)]) -> String {
    let mut out = String::new();
    for (k, v) in pairs {
        if !out.is_empty() {
            out.push('&');
        }
        out.push_str(&encode_component(k.as_ref()));
        out.push('=');
        out.push_str(&encode_component(v.as_ref()));
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn components_keep_only_unreserved_characters() {
        assert_eq!(encode_component("aZ09-._~"), "aZ09-._~");
        assert_eq!(
            encode_component("a b&c=d?e/f#g+h%"),
            "a%20b%26c%3Dd%3Fe%2Ff%23g%2Bh%25"
        );
        assert_eq!(encode_component("\r\n"), "%0D%0A");
        assert_eq!(encode_component("é"), "%C3%A9");
        assert_eq!(encode_component(""), "");
    }

    #[test]
    fn queries_encode_keys_and_values() {
        assert_eq!(
            query(&[("id", "a,b"), ("key", "x&y=z"), ("n k", "")]),
            "id=a%2Cb&key=x%26y%3Dz&n%20k="
        );
        assert_eq!(query::<&str, &str>(&[]), "");
    }

    fn get(url: &str) -> HttpRequest {
        HttpRequest::new(Method::Get, url, 1024)
    }

    #[test]
    fn requests_that_could_smuggle_headers_are_refused() {
        assert_eq!(get("https://example.com/a?b=c").check(), Ok(()));
        for bad in [
            get("http://example.com/"),
            get("https://example.com/a b"),
            get("https://example.com/\r\nX: y"),
            get("https://example.com/é"),
            get("https://example.com/").header("X-A", "b\r\nX-B: c"),
            get("https://example.com/").header("X-A", "b\nc"),
            get("https://example.com/").header("X-A", "b\0"),
            get("https://example.com/").header("X A", "b"),
            get("https://example.com/").header("", "b"),
            get("https://example.com/").header("X:A", "b"),
            HttpRequest::new(Method::Get, "https://example.com/", 0),
            get(&format!(
                "https://example.com/{}",
                "a".repeat(MAX_URL_BYTES)
            )),
        ] {
            assert!(
                matches!(bad.check(), Err(FetchError::InvalidRequest(_))),
                "{bad:?}"
            );
        }
        let ok = get("https://example.com/")
            .header("Authorization", "Bearer abc\tdef")
            .body("application/x-www-form-urlencoded", b"a=b".to_vec());
        assert_eq!(ok.check(), Ok(()));
        assert_eq!(ok.headers.len(), 2);
    }

    #[test]
    fn the_default_fetch_refuses_every_request() {
        let r = futures_util::FutureExt::now_or_never(
            Fetch::default().fetch(get("https://example.com/")),
        );
        assert_eq!(r, Some(Err(FetchError::Unsupported)));
    }

    #[test]
    fn response_headers_are_found_in_any_case() {
        let r = HttpResponse {
            status: 200,
            headers: vec![("Content-Type".into(), "a".into())],
            body: Vec::new(),
        };
        assert_eq!(r.header("content-type"), Some("a"));
        assert_eq!(r.header("x"), None);
    }
}
