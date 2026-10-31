// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The headless service: runs a profile's node, serves the UI's frontend
//! RPC over WebSocket, and serves the node's HLS streams over HTTP, both
//! with TLS when given a certificate.
//!
//! [`Svc`] opens the profiles' stores in one redb file, runs their
//! sessions through a [`Manager`] with the node session factory
//! ([`NodeFactory`]), and accepts the UI's WebSocket connections on its
//! API address at its API path. A connection starts with only the auth
//! service bound; signing in or up starts the profile's node and binds its
//! frontends on the connection. A process runs one profile's node at a
//! time.
//!
//! # Connections
//!
//! Each WebSocket message carries whole varint-delimited RPC calls, as the
//! UI sends them. The upgrade reads the `session_key` cookie, 32 bytes in
//! padded URL-safe base64, and sets it on the response; a missing or
//! malformed one is replaced by a new random key. The key seals the
//! session tokens of sign-ins that persist, so a browser that keeps the
//! cookie can sign in with its token again. At most
//! [`Config::max_connections`] connections are served at once.
//!
//! # HLS
//!
//! With [`Config::hls_listen`], the service answers players' HTTP
//! requests for the running profile's HLS streams there, over TLS with
//! [`Config::tls`], from the factory's
//! [`HlsRoutes`](strims_video::hls::HlsRoutes), within the HTTP bounds of
//! [`HlsSettings`](strims_video::HlsSettings). The node's video control
//! then supports HLS egress
//! ([`HlsSettings::serve`](strims_video::HlsSettings::serve)), and its
//! playlist URLs start with
//! [`HlsSettings::base_url`](strims_video::HlsSettings::base_url) when
//! the device's HLS config names no public server address. Without the
//! listener, HLS egress is unsupported.
//!
//! The playlists are low-latency HLS with parts of
//! [`HlsSettings::part_target`](strims_video::HlsSettings::part_target)
//! (1 s by default, `--hls-part-target`): a player that loads parts holds
//! a blocking reload and the hinted part open, each answered as its part
//! comes, over HTTP/2 on one connection. Safari plays parts at low
//! latency only over HTTP/2, so only with [`Config::tls`].
//!
//! An empty base URL becomes one at the listener's port. Without TLS, it
//! is `http://` and the listener's IP, or for an unspecified address
//! [`AppConfig::public_ip`], else [`AppConfig::ws_public_hostname`]. Over
//! TLS, it is `https://` and the first of those three the certificate
//! names, else the first DNS name the certificate names that isn't a
//! wildcard, else the first of the three, with a warning. With none of
//! them, the base URL stays empty and the HLS config must name the
//! address. The base URL is set when the service starts.
//!
//! # TLS
//!
//! With [`Config::tls`], the HLS listener serves HTTPS, HTTP/2 to the
//! players whose handshake offers it and HTTP/1.1 to the others, and with
//! [`TlsConfig::api`] the API takes `wss://` connections, with the
//! operator's certificate: a PEM file of its chain, the service's own
//! certificate first, and one of its private key. The service doesn't
//! start unless both load and match. A connection's handshake counts
//! toward the time it has for its request head
//! ([`HlsSettings::request_head_timeout`](strims_video::HlsSettings::request_head_timeout))
//! or its upgrade. Every [`TlsConfig::reload_interval`] the files are
//! checked, and a renewed pair serves the connections accepted after it
//! loads.
//!
//! An HTTP/2 player makes one TLS handshake for all its requests. One that
//! speaks only HTTP/1.1 makes one for each, since an HTTP/1.1 connection
//! carries one answer: two each target duration, a playlist's and a
//! segment's, a known cost to a phone's battery.
//!
//! A certificate players accept chains to a root their device trusts and
//! names the playlist URL's host: one that is self-signed plays nowhere,
//! and AirPlay and Cast receivers, which fetch the URL themselves, trust
//! only public roots. So the certificate is a DNS name's:
//!
//! - from an ACME CA such as Let's Encrypt through the DNS-01 challenge,
//!   which reaches no port of the host, so it certifies a name for a LAN
//!   address: a public DNS record maps the name to the address, and an
//!   ACME client (certbot, lego, acme.sh) proves the name through the DNS
//!   provider's API and renews the files, which the service reloads. A
//!   resolver that drops private addresses from public answers, as
//!   routers' rebinding protection does, needs the name allowed. Public
//!   CAs certify IP addresses only when they are public;
//! - from `tailscale cert`, for players on the tailnet;
//! - from a private CA whose root each device installs and trusts, for a
//!   lab: no receiver takes one.
//!
//! A TLS proxy in front of plain HTTP, its URL the HLS base URL, serves as
//! well.
//!
//! # Threads
//!
//! [`Svc`] runs inside a tokio `LocalSet`, on one thread. The running
//! profile's PSI sessions compute on a thread of their own by default
//! ([`PsiMode::Thread`] in [`Config::app`]), so the node's I/O goes on
//! while a session computes.

mod api;
mod http;
mod http2;
mod tls;

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use rand_core::SeedableRng;
use strims_base::rng::DetRng;
use strims_kv_redb::RedbKv;
use strims_node::app::{AppConfig, NodeFactory, NodeTimer, PsiMode};
use strims_node::{NodeInfo, SystemClock, TokioNet};
use strims_session::Manager;
use tokio::net::TcpListener;
use tokio::task::{AbortHandle, spawn_local};

pub use crate::tls::TlsError;
use crate::tls::{Certificate, Tls};

type Factory = NodeFactory<TokioNet, SystemClock>;
type SvcTimer = NodeTimer<SystemClock>;
type SvcManager = Manager<RedbKv, Factory, SvcTimer>;

/// Where the profiles' stores live.
#[derive(Clone, Debug)]
pub enum Db {
    /// A redb file, created if it doesn't exist.
    File(PathBuf),
    /// Memory, gone when the service stops.
    Memory,
}

/// How the service runs.
#[derive(Clone)]
pub struct Config {
    /// `host:port` the API accepts the UI's connections on; port 0 picks
    /// one.
    pub api_listen: String,
    /// The path the API upgrades WebSocket connections at.
    pub api_path: String,
    pub db: Db,
    /// How a signed-in profile's node runs; by default its PSI sessions
    /// compute on their own thread ([`PsiMode::Thread`]). The service sets
    /// [`HlsSettings::serve`](strims_video::HlsSettings::serve) in its
    /// video config, and fills an empty
    /// [`HlsSettings::base_url`](strims_video::HlsSettings::base_url).
    pub app: AppConfig,
    /// The most UI connections served at once.
    pub max_connections: usize,
    /// `host:port` HLS is served to players on, over HTTP, or HTTPS with
    /// [`tls`](Config::tls); port 0 picks one. `None` serves no HLS.
    pub hls_listen: Option<String>,
    /// The certificate HLS is served with over TLS, and the API with
    /// [`TlsConfig::api`]. `None` serves both without TLS.
    pub tls: Option<TlsConfig>,
}

/// How often a [`TlsConfig`] checks its files by default.
pub const DEFAULT_TLS_RELOAD_INTERVAL: StdDuration = StdDuration::from_secs(60);

/// The operator's certificate, which the service's listeners serve TLS
/// with (see [TLS](crate#tls)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TlsConfig {
    /// A PEM file of the certificate chain, the service's own certificate
    /// first.
    pub cert_path: PathBuf,
    /// A PEM file of its private key, in PKCS #8, PKCS #1 (RSA) or SEC1
    /// (EC).
    pub key_path: PathBuf,
    /// The API takes its connections over TLS too, as `wss://`.
    pub api: bool,
    /// How often the files are checked for a renewed certificate; zero
    /// never checks.
    pub reload_interval: StdDuration,
}

impl TlsConfig {
    /// The certificate in these files, for HLS alone, checked every
    /// [`DEFAULT_TLS_RELOAD_INTERVAL`].
    pub fn new(cert_path: impl Into<PathBuf>, key_path: impl Into<PathBuf>) -> TlsConfig {
        TlsConfig {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            api: false,
            reload_interval: DEFAULT_TLS_RELOAD_INTERVAL,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_listen: "127.0.0.1:8083".to_owned(),
            api_path: "/api".to_owned(),
            db: Db::Memory,
            app: AppConfig {
                psi: PsiMode::Thread,
                ..AppConfig::default()
            },
            max_connections: 64,
            hls_listen: None,
            tls: None,
        }
    }
}

/// Why the service didn't start.
#[derive(Debug, thiserror::Error)]
pub enum SvcError {
    #[error("opening the store: {0}")]
    Store(#[from] strims_kv::Error),
    #[error("listening for the API: {0}")]
    Listen(#[from] std::io::Error),
    #[error("listening for HLS: {0}")]
    HlsListen(std::io::Error),
    #[error("drawing a seed from OS entropy: {0}")]
    Entropy(#[from] getrandom::Error),
    #[error("checking the codec stage's ffmpeg: {0}")]
    Ffmpeg(strims_node::codec::ProbeError),
    #[error("loading the TLS certificate: {0}")]
    Tls(#[from] TlsError),
}

/// A running service. Dropping it stops serving the UI and HLS.
pub struct Svc {
    manager: Rc<SvcManager>,
    api: Rc<api::Api>,
    api_addr: SocketAddr,
    accept: AbortHandle,
    hls: Option<Hls>,
    /// Checks the certificate's files for a renewal.
    watch: Option<AbortHandle>,
}

/// The HLS server.
struct Hls {
    http: Rc<http::Http>,
    addr: SocketAddr,
    base_url: String,
    accept: AbortHandle,
}

impl Svc {
    /// Opens the store and starts serving the API, and HLS if configured.
    /// A config naming an ffmpeg for the codec stage starts only if
    /// [`probe_ffmpeg`](strims_node::codec::probe_ffmpeg) takes it, and
    /// one with a certificate only if it loads. Runs inside a tokio
    /// `LocalSet`.
    pub async fn start(config: Config) -> Result<Svc, SvcError> {
        let (tls, cert) = match &config.tls {
            Some(c) => {
                let (tls, cert) = Tls::load(c)?;
                (Some(Rc::new(tls)), Some(cert))
            }
            None => (None, None),
        };
        // The HLS address goes into the video config the factory takes.
        let hls_listener = match &config.hls_listen {
            Some(addr) => {
                let listener = TcpListener::bind(addr).await.map_err(SvcError::HlsListen)?;
                let addr = listener.local_addr().map_err(SvcError::HlsListen)?;
                Some((listener, addr))
            }
            None => None,
        };
        let mut app = config.app;
        app.video.hls.serve = hls_listener.is_some();
        // The host a certificate renewal should name too.
        let mut tls_host = None;
        if let Some((_, addr)) = &hls_listener {
            if app.video.hls.base_url.is_empty() {
                if let Some(base) = hls_base_url(*addr, &app, cert.as_ref()) {
                    if let Some(cert) = &cert {
                        if !cert.valid_for(&base.host) {
                            tracing::warn!(
                                host = base.host,
                                "the TLS certificate doesn't name the HLS URLs' host"
                            );
                        }
                        tls_host = Some(base.host);
                    }
                    app.video.hls.base_url = base.url;
                }
            } else if cert.is_some() && has_scheme(&app.video.hls.base_url, "http://") {
                tracing::warn!(
                    base_url = app.video.hls.base_url,
                    "HLS is served over TLS, but its base URL is http://"
                );
            }
        }
        if let Some(ffmpeg) = &app.ffmpeg {
            let version = strims_node::codec::probe_ffmpeg(ffmpeg)
                .await
                .map_err(SvcError::Ffmpeg)?;
            tracing::info!(program = %ffmpeg.program.display(), version, "transcoding with ffmpeg");
        }
        let hls_limits = http::Limits::new(&app.video.hls);
        let hls_base_url = app.video.hls.base_url.clone();

        let kv = match &config.db {
            Db::File(path) => RedbKv::open(path)?,
            Db::Memory => RedbKv::in_memory()?,
        };
        // Profile keys and sealing nonces come from this rng.
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed)?;
        let manager = Rc::new(Manager::new(
            kv,
            NodeFactory::new(TokioNet, SystemClock, app),
            NodeTimer(SystemClock),
            std::env::consts::OS,
            DetRng::from_seed(seed),
        ));
        let listener = TcpListener::bind(&config.api_listen).await?;
        let api_addr = listener.local_addr()?;
        let api_tls = tls
            .clone()
            .filter(|_| config.tls.as_ref().is_some_and(|t| t.api));
        let api = Rc::new(api::Api::new(
            manager.clone(),
            config.api_path,
            config.max_connections,
            api_tls,
        ));
        let accept = spawn_local(api::accept(listener, api.clone())).abort_handle();
        let hls = hls_listener.map(|(listener, addr)| {
            let http = Rc::new(http::Http::new(
                manager.factory().hls_routes().clone(),
                manager.timer().clone(),
                hls_limits,
                tls.clone(),
            ));
            let accept = spawn_local(http::accept(listener, http.clone())).abort_handle();
            Hls {
                http,
                addr,
                base_url: hls_base_url,
                accept,
            }
        });
        let watch = tls
            .filter(|t| !t.reload_interval().is_zero())
            .map(|t| spawn_local(tls::watch(t, tls_host)).abort_handle());
        Ok(Svc {
            manager,
            api,
            api_addr,
            accept,
            hls,
            watch,
        })
    }

    /// The address the API accepts connections on.
    pub fn api_addr(&self) -> SocketAddr {
        self.api_addr
    }

    /// The address HLS is served on, if it is.
    pub fn hls_addr(&self) -> Option<SocketAddr> {
        self.hls.as_ref().map(|h| h.addr)
    }

    /// The start of HLS playlist URLs when the HLS config names no public
    /// server address; empty for none, and when HLS isn't served.
    pub fn hls_base_url(&self) -> &str {
        self.hls.as_ref().map_or("", |h| h.base_url.as_str())
    }

    /// The running profile's node, once a sign-in started one.
    pub fn node_info(&self) -> Option<NodeInfo> {
        self.manager.factory().running()
    }

    /// Stops serving the UI and HLS, and stops the running node.
    pub async fn stop(self) {
        drop(self);
        tokio::task::yield_now().await;
    }
}

impl Drop for Svc {
    fn drop(&mut self) {
        self.accept.abort();
        self.api.close();
        if let Some(hls) = &self.hls {
            hls.accept.abort();
            hls.http.close();
        }
        if let Some(watch) = &self.watch {
            watch.abort();
        }
    }
}

/// A base URL of HLS playlists, and its host.
#[derive(Debug, PartialEq, Eq)]
struct BaseUrl {
    url: String,
    /// An IP address without brackets, or a DNS name.
    host: String,
}

/// A host a player may reach the HLS listener at.
#[derive(Clone)]
enum Host {
    Ip(IpAddr),
    Name(String),
}

impl Host {
    fn base_url(self, scheme: &str, port: u16) -> BaseUrl {
        match self {
            Host::Ip(ip) => BaseUrl {
                url: format!("{scheme}://{}", SocketAddr::new(ip, port)),
                host: ip.to_string(),
            },
            Host::Name(name) => BaseUrl {
                url: format!("{scheme}://{name}:{port}"),
                host: name,
            },
        }
    }

    fn valid_for(&self, cert: &Certificate) -> bool {
        match self {
            Host::Ip(ip) => cert.valid_for(&ip.to_string()),
            Host::Name(name) => cert.valid_for(name),
        }
    }
}

/// The base of playlist URLs for HLS served at `addr`, over TLS with
/// `cert` if given, by the rule of [HLS](crate#hls); `None` for no host.
fn hls_base_url(addr: SocketAddr, app: &AppConfig, cert: Option<&Certificate>) -> Option<BaseUrl> {
    let mut hosts = Vec::with_capacity(3);
    if !addr.ip().is_unspecified() {
        hosts.push(Host::Ip(addr.ip()));
    }
    hosts.extend(app.public_ip.map(Host::Ip));
    if !app.ws_public_hostname.is_empty() {
        hosts.push(Host::Name(app.ws_public_hostname.clone()));
    }
    let Some(cert) = cert else {
        return hosts
            .into_iter()
            .next()
            .map(|h| h.base_url("http", addr.port()));
    };
    let named = hosts.iter().find(|h| h.valid_for(cert)).cloned();
    named
        .or_else(|| cert.dns_names().into_iter().next().map(Host::Name))
        .or_else(|| hosts.into_iter().next())
        .map(|h| h.base_url("https", addr.port()))
}

/// Whether `url` starts with `scheme`, in any case.
fn has_scheme(url: &str, scheme: &str) -> bool {
    url.get(..scheme.len())
        .is_some_and(|p| p.eq_ignore_ascii_case(scheme))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::tls::testing::{dir, self_signed, write};

    fn at(a: &str) -> SocketAddr {
        a.parse().unwrap()
    }

    fn url(addr: &str, app: &AppConfig, cert: Option<&Certificate>) -> String {
        hls_base_url(at(addr), app, cert).map_or(String::new(), |b| b.url)
    }

    #[test]
    fn hls_base_urls_name_a_reachable_address() {
        let mut app = AppConfig::default();
        assert_eq!(url("127.0.0.1:8082", &app, None), "http://127.0.0.1:8082");
        assert_eq!(url("[::1]:8082", &app, None), "http://[::1]:8082");
        assert_eq!(url("0.0.0.0:8082", &app, None), "");
        app.ws_public_hostname = "node.example".into();
        assert_eq!(url("0.0.0.0:8082", &app, None), "http://node.example:8082");
        app.public_ip = Some("2001:db8::1".parse().unwrap());
        assert_eq!(url("[::]:8082", &app, None), "http://[2001:db8::1]:8082");
        let base = hls_base_url(at("[::]:8082"), &app, None).unwrap();
        assert_eq!(base.host, "2001:db8::1");
    }

    #[test]
    fn hls_base_urls_over_tls_name_a_host_the_certificate_names() {
        let dir = dir("base-url");
        let load = |names: &[&str]| {
            let (cert, key, _) = self_signed(names);
            let (c, k) = write(&dir, &cert, &key);
            Certificate::load(&c, &k).unwrap()
        };
        let mut app = AppConfig::default();

        // The listener's IP, then the public IP, then the public hostname.
        let ips = load(&["10.0.0.1", "2001:db8::1"]);
        assert_eq!(
            url("10.0.0.1:8443", &app, Some(&ips)),
            "https://10.0.0.1:8443"
        );
        app.public_ip = Some("2001:db8::1".parse().unwrap());
        app.ws_public_hostname = "node.example".into();
        assert_eq!(
            url("10.0.0.2:8443", &app, Some(&ips)),
            "https://[2001:db8::1]:8443"
        );
        let names = load(&["*.example", "media.example", "node.example"]);
        assert_eq!(
            url("10.0.0.1:8443", &app, Some(&names)),
            "https://node.example:8443"
        );

        // Else the certificate's first DNS name that isn't a wildcard.
        app.ws_public_hostname = String::new();
        let base = hls_base_url(at("0.0.0.0:8443"), &app, Some(&names)).unwrap();
        assert_eq!(base.url, "https://media.example:8443");
        assert_eq!(base.host, "media.example");
        app.public_ip = None;
        assert_eq!(
            url("0.0.0.0:8443", &app, Some(&names)),
            "https://media.example:8443"
        );

        // Else the first host, which the certificate doesn't name.
        let wildcard = load(&["*.example"]);
        let base = hls_base_url(at("10.0.0.1:8443"), &app, Some(&wildcard)).unwrap();
        assert_eq!(base.url, "https://10.0.0.1:8443");
        assert!(!wildcard.valid_for(&base.host));
        assert_eq!(
            hls_base_url(at("0.0.0.0:8443"), &app, Some(&wildcard)),
            None
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
