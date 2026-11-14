// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The service's TLS: the operator's certificate from PEM files, the
//! server configs its listeners hand their handshakes, and their reload
//! when the files change.
//!
//! [`Tls::load`] reads the chain and its key, which must match. The
//! listeners take their configs from [`Tls::hls`] and [`Tls::api`] at each
//! accept, so a reload reaches the connections accepted after it, and the
//! connections open before keep the certificate their handshake took. The
//! node's WebSocket listener runs on tasks of its own and takes
//! [`Tls::links`] once: its certificate resolver hands each handshake the
//! certificate loaded last, so a reload reaches its links the same way.
//! [`watch`] checks the files every
//! [`TlsConfig::reload_interval`](crate::TlsConfig::reload_interval): a
//! change of either file's modification time or length loads both again.
//! A pair that fails to load is reported once and the certificate before
//! it stays; the next change of either file tries again. Every load's
//! configs share one session cache.

use std::cell::RefCell;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration as StdDuration, SystemTime};

use rustls::ServerConfig;
use rustls::crypto::CryptoProvider;
use rustls::pki_types::pem::{self, PemObject};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::server::{ServerSessionMemoryCache, StoresServerSessions};
use rustls::sign::CertifiedKey;
use tokio_rustls::TlsAcceptor;

use crate::TlsConfig;

/// HTTP/2's ALPN protocol id.
pub(crate) const H2: &[u8] = b"h2";
/// The ALPN protocols the HLS listener speaks, by preference. A client
/// that offers none takes HTTP/1.1.
const HLS_ALPN: &[&[u8]] = &[H2, b"http/1.1"];
/// The sessions kept for resumption, across the listeners.
const SESSION_CACHE: usize = 256;

/// Why the certificate didn't load.
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("reading the certificates in {path}: {source}")]
    Certificates { path: PathBuf, source: pem::Error },
    #[error("{0} holds no certificate")]
    NoCertificate(PathBuf),
    #[error("reading the private key in {path}: {source}")]
    Key { path: PathBuf, source: pem::Error },
    /// The key doesn't match the chain's first certificate, or isn't a
    /// kind TLS signs with.
    #[error("the certificate and its key: {0}")]
    Rejected(rustls::Error),
}

/// A certificate chain and its private key, as loaded.
pub(crate) struct Certificate {
    /// Never empty.
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
}

impl Certificate {
    /// Reads the chain in `cert` and the first private key in `key`.
    pub(crate) fn load(cert: &Path, key: &Path) -> Result<Certificate, TlsError> {
        let chain = CertificateDer::pem_file_iter(cert)
            .and_then(|certs| certs.collect::<Result<Vec<_>, _>>())
            .map_err(|source| TlsError::Certificates {
                path: cert.to_owned(),
                source,
            })?;
        if chain.is_empty() {
            return Err(TlsError::NoCertificate(cert.to_owned()));
        }
        let key = PrivateKeyDer::from_pem_file(key).map_err(|source| TlsError::Key {
            path: key.to_owned(),
            source,
        })?;
        Ok(Certificate { chain, key })
    }

    fn leaf(&self) -> Option<webpki::EndEntityCert<'_>> {
        webpki::EndEntityCert::try_from(self.chain.first()?).ok()
    }

    /// Whether the certificate is valid for `host`: a DNS name, or an IP
    /// address, an IPv6 one with or without its URL brackets.
    pub(crate) fn valid_for(&self, host: &str) -> bool {
        let host = host
            .strip_prefix('[')
            .and_then(|h| h.strip_suffix(']'))
            .unwrap_or(host);
        let (Some(leaf), Ok(name)) = (self.leaf(), ServerName::try_from(host)) else {
            return false;
        };
        leaf.verify_is_valid_for_subject_name(&name).is_ok()
    }

    /// The DNS names the certificate is valid for, in its order, but for
    /// its wildcard names.
    pub(crate) fn dns_names(&self) -> Vec<String> {
        let Some(leaf) = self.leaf() else {
            return Vec::new();
        };
        leaf.valid_dns_names()
            .filter(|n| !n.starts_with('*'))
            .map(str::to_owned)
            .collect()
    }

    /// The hosts the certificate names: its [DNS names](Self::dns_names),
    /// then its IP addresses, each in its order.
    pub(crate) fn names(&self) -> Vec<String> {
        let ips = self
            .chain
            .first()
            .map_or_else(Vec::new, |leaf| san_ips(leaf));
        let ips = ips
            .into_iter()
            .map(|ip| ip.to_string())
            .filter(|ip| self.valid_for(ip));
        self.dns_names().into_iter().chain(ips).collect()
    }
}

/// The IP addresses in a certificate's subject alternative names, in their
/// order; none when the DER doesn't read as a certificate.
fn san_ips(der: &[u8]) -> Vec<IpAddr> {
    const SEQUENCE: u8 = 0x30;
    const OCTET_STRING: u8 = 0x04;
    const OID: u8 = 0x06;
    /// `[3] EXPLICIT Extensions` in a `TBSCertificate`.
    const EXTENSIONS: u8 = 0xa3;
    /// `[7] IMPLICIT OCTET STRING`, a `GeneralName`'s `iPAddress`.
    const IP_ADDRESS: u8 = 0x87;
    /// id-ce-subjectAltName, 2.5.29.17.
    const SUBJECT_ALT_NAME: &[u8] = &[0x55, 0x1d, 0x11];

    let names = (|| {
        let cert = expect(der, SEQUENCE)?;
        let tbs = expect(cert, SEQUENCE)?;
        let (_, extensions) = elements(tbs).find(|&(tag, _)| tag == EXTENSIONS)?;
        let extensions = expect(extensions, SEQUENCE)?;
        let san = elements(extensions)
            .filter(|&(tag, _)| tag == SEQUENCE)
            .find_map(|(_, ext)| {
                let mut parts = elements(ext);
                let (tag, id) = parts.next()?;
                if tag != OID || id != SUBJECT_ALT_NAME {
                    return None;
                }
                // After the optional `critical` flag, the value.
                parts.find(|&(t, _)| t == OCTET_STRING).map(|(_, v)| v)
            })?;
        expect(san, SEQUENCE)
    })();
    let Some(names) = names else {
        return Vec::new();
    };
    elements(names)
        .filter(|&(tag, _)| tag == IP_ADDRESS)
        .filter_map(|(_, ip)| {
            if let Ok(b) = <[u8; 4]>::try_from(ip) {
                Some(IpAddr::V4(Ipv4Addr::from(b)))
            } else {
                <[u8; 16]>::try_from(ip)
                    .ok()
                    .map(|b| IpAddr::V6(Ipv6Addr::from(b)))
            }
        })
        .collect()
}

/// The contents of `der`'s first element, if its tag is `tag`.
fn expect(der: &[u8], tag: u8) -> Option<&[u8]> {
    der_element(der).and_then(|(t, contents, _)| (t == tag).then_some(contents))
}

/// The elements of a DER sequence's contents: each one's tag and contents,
/// up to the first that doesn't read.
fn elements(mut der: &[u8]) -> impl Iterator<Item = (u8, &[u8])> {
    std::iter::from_fn(move || {
        let (tag, contents, rest) = der_element(der)?;
        der = rest;
        Some((tag, contents))
    })
}

/// The first DER element in `der`: its tag, its contents and the bytes
/// after it. Tags of one byte and definite lengths of up to 4 bytes.
fn der_element(der: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, rest) = der.split_first()?;
    let (&first, rest) = rest.split_first()?;
    let (len, rest) = if first < 0x80 {
        (usize::from(first), rest)
    } else {
        let (bytes, rest) = rest.split_at_checked(usize::from(first & 0x7f))?;
        if bytes.is_empty() || bytes.len() > 4 {
            return None;
        }
        let len = bytes.iter().fold(0u32, |len, &b| (len << 8) | u32::from(b));
        (usize::try_from(len).ok()?, rest)
    };
    let (contents, rest) = rest.split_at_checked(len)?;
    Some((tag, contents, rest))
}

use link_certificate::LinkCertificate;

mod link_certificate {
    //! The node's WebSocket listener's certificate, which its tasks on the
    //! runtime's threads read at each handshake and a reload replaces:
    //! rustls's resolvers are `Send + Sync`, so the certificate sits
    //! behind a lock, written once a reload and otherwise only read.

    #![allow(clippy::disallowed_types)]

    use std::sync::{Arc, PoisonError, RwLock};

    use rustls::server::{ClientHello, ResolvesServerCert};
    use rustls::sign::CertifiedKey;

    /// The certificate loaded last.
    #[derive(Debug)]
    pub(super) struct LinkCertificate(RwLock<Arc<CertifiedKey>>);

    impl LinkCertificate {
        pub(super) fn new(key: Arc<CertifiedKey>) -> Self {
            LinkCertificate(RwLock::new(key))
        }

        /// Serves `key` from the next handshake on.
        pub(super) fn set(&self, key: Arc<CertifiedKey>) {
            *self.0.write().unwrap_or_else(PoisonError::into_inner) = key;
        }
    }

    impl ResolvesServerCert for LinkCertificate {
        fn resolve(&self, _: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
            Some(
                self.0
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone(),
            )
        }
    }
}

/// A file's modification time and length, `None` when it can't be read.
type Stamp = Option<(SystemTime, u64)>;

fn stamp(path: &Path) -> Stamp {
    let m = std::fs::metadata(path).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// The listeners' TLS configs, and the files they were loaded from.
pub(crate) struct Tls {
    cert_path: PathBuf,
    key_path: PathBuf,
    reload_interval: StdDuration,
    provider: Arc<CryptoProvider>,
    sessions: Arc<dyn StoresServerSessions>,
    state: RefCell<State>,
    /// The node's WebSocket listener's config, over `link_cert`.
    links: Arc<ServerConfig>,
    link_cert: Arc<LinkCertificate>,
}

struct State {
    hls: TlsAcceptor,
    api: TlsAcceptor,
    /// The files' stamps at the last load or attempt.
    stamps: [Stamp; 2],
}

impl Tls {
    /// Loads `config`'s certificate: the listeners' TLS, and the
    /// certificate.
    pub(crate) fn load(config: &TlsConfig) -> Result<(Tls, Certificate), TlsError> {
        let stamps = [stamp(&config.cert_path), stamp(&config.key_path)];
        let cert = Certificate::load(&config.cert_path, &config.key_path)?;
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let sessions: Arc<dyn StoresServerSessions> = ServerSessionMemoryCache::new(SESSION_CACHE);
        let (hls, api) = acceptors(&cert, &provider, &sessions)?;
        let link_cert = Arc::new(LinkCertificate::new(certified(&cert, &provider)?));
        let mut links = ServerConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()
            .map_err(TlsError::Rejected)?
            .with_no_client_auth()
            .with_cert_resolver(link_cert.clone());
        // Like the API's, links negotiate no protocol.
        links.session_storage = sessions.clone();
        let tls = Tls {
            cert_path: config.cert_path.clone(),
            key_path: config.key_path.clone(),
            reload_interval: config.reload_interval,
            provider,
            sessions,
            state: RefCell::new(State { hls, api, stamps }),
            links: Arc::new(links),
            link_cert,
        };
        Ok((tls, cert))
    }

    /// How often [`watch`] checks the files.
    pub(crate) fn reload_interval(&self) -> StdDuration {
        self.reload_interval
    }

    /// The HLS listener's TLS, as now loaded.
    pub(crate) fn hls(&self) -> TlsAcceptor {
        self.state.borrow().hls.clone()
    }

    /// The API listener's TLS, as now loaded.
    pub(crate) fn api(&self) -> TlsAcceptor {
        self.state.borrow().api.clone()
    }

    /// The node's WebSocket listener's TLS, which serves every reload.
    pub(crate) fn links(&self) -> Arc<ServerConfig> {
        self.links.clone()
    }

    /// Loads the files again if either changed since the last load or
    /// attempt: the certificate that now serves, or why the files didn't
    /// load; `None` when neither changed.
    pub(crate) fn reload(&self) -> Option<Result<Certificate, TlsError>> {
        // Stamped before the reads: a write during them changes the stamp
        // again, so the next check reads the files again.
        let stamps = [stamp(&self.cert_path), stamp(&self.key_path)];
        if self.state.borrow().stamps == stamps {
            return None;
        }
        self.state.borrow_mut().stamps = stamps;
        let loaded = Certificate::load(&self.cert_path, &self.key_path).and_then(|cert| {
            let (hls, api) = acceptors(&cert, &self.provider, &self.sessions)?;
            let link = certified(&cert, &self.provider)?;
            let mut state = self.state.borrow_mut();
            state.hls = hls;
            state.api = api;
            self.link_cert.set(link);
            Ok(cert)
        });
        Some(loaded)
    }
}

/// `cert` as a resolver serves it, its key checked against the chain.
fn certified(cert: &Certificate, provider: &CryptoProvider) -> Result<Arc<CertifiedKey>, TlsError> {
    CertifiedKey::from_der(cert.chain.clone(), cert.key.clone_key(), provider)
        .map(Arc::new)
        .map_err(TlsError::Rejected)
}

/// The HLS and API listeners' TLS with `cert`.
fn acceptors(
    cert: &Certificate,
    provider: &Arc<CryptoProvider>,
    sessions: &Arc<dyn StoresServerSessions>,
) -> Result<(TlsAcceptor, TlsAcceptor), TlsError> {
    let config = |alpn: &[&[u8]]| -> Result<TlsAcceptor, TlsError> {
        let mut c = ServerConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()
            .map_err(TlsError::Rejected)?
            .with_no_client_auth()
            .with_single_cert(cert.chain.clone(), cert.key.clone_key())
            .map_err(TlsError::Rejected)?;
        c.alpn_protocols = alpn.iter().map(|p| p.to_vec()).collect();
        c.session_storage = sessions.clone();
        Ok(TlsAcceptor::from(Arc::new(c)))
    };
    // The API negotiates no protocol: WebSocket upgrades take HTTP/1.1.
    Ok((config(HLS_ALPN)?, config(&[])?))
}

/// Checks `tls`'s files every reload interval and serves a renewed
/// certificate, until aborted. `hosts` are the hosts the service
/// advertised over TLS when it started (the HLS URLs', the node's link
/// URIs'), which a renewal that doesn't name them leaves as they are.
pub(crate) async fn watch(tls: Rc<Tls>, hosts: Vec<String>) {
    loop {
        tokio::time::sleep(tls.reload_interval).await;
        match tls.reload() {
            None => {}
            Some(Ok(cert)) => {
                tracing::info!(path = %tls.cert_path.display(), "serving a renewed TLS certificate");
                for host in hosts.iter().filter(|h| !cert.valid_for(h)) {
                    tracing::warn!(
                        host,
                        "the renewed TLS certificate doesn't name a host the service advertises"
                    );
                }
            }
            Some(Err(e)) => tracing::warn!(
                error = %e,
                "the TLS certificate's files changed but don't load; serving the certificate before"
            ),
        }
    }
}

#[cfg(test)]
pub(crate) mod testing {
    //! Certificates for tests, and clients that trust them.

    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use data_encoding::BASE64;
    use rustls::pki_types::{CertificateDer, ServerName};
    use rustls::{ClientConfig, RootCertStore};
    use tokio::net::TcpStream;
    use tokio_rustls::TlsConnector;
    use tokio_rustls::client::TlsStream;

    /// A self-signed certificate for `names` (DNS names or IP addresses):
    /// its PEM, its key's PEM, and its DER.
    pub(crate) fn self_signed(names: &[&str]) -> (String, String, CertificateDer<'static>) {
        let names: Vec<String> = names.iter().map(|n| (*n).to_owned()).collect();
        let key = rcgen::generate_simple_self_signed(names).unwrap();
        let der = key.cert.der().clone();
        let cert = pem("CERTIFICATE", &der);
        let key = pem("PRIVATE KEY", &key.signing_key.serialize_der());
        (cert, key, der)
    }

    pub(crate) fn pem(label: &str, der: &[u8]) -> String {
        let b64 = BASE64.encode(der);
        let mut out = format!("-----BEGIN {label}-----\n");
        for line in b64.as_bytes().chunks(64) {
            out.push_str(std::str::from_utf8(line).unwrap());
            out.push('\n');
        }
        out.push_str(&format!("-----END {label}-----\n"));
        out
    }

    /// A directory of its own for test `name`'s files.
    pub(crate) fn dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("strims-svc-tls-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Writes `cert` and `key` to `cert.pem` and `key.pem` in `dir`.
    pub(crate) fn write(dir: &Path, cert: &str, key: &str) -> (PathBuf, PathBuf) {
        let (c, k) = (dir.join("cert.pem"), dir.join("key.pem"));
        std::fs::write(&c, cert).unwrap();
        std::fs::write(&k, key).unwrap();
        (c, k)
    }

    /// A client trusting `root`, offering `alpn`.
    pub(crate) fn client(root: &CertificateDer<'static>, alpn: &[&[u8]]) -> TlsConnector {
        let mut roots = RootCertStore::empty();
        roots.add(root.clone()).unwrap();
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth();
        config.alpn_protocols = alpn.iter().map(|p| p.to_vec()).collect();
        TlsConnector::from(Arc::new(config))
    }

    /// Connects to `addr` over TLS as `name`.
    pub(crate) async fn connect(
        connector: &TlsConnector,
        addr: std::net::SocketAddr,
        name: &str,
    ) -> std::io::Result<TlsStream<TcpStream>> {
        let tcp = TcpStream::connect(addr).await?;
        let name = ServerName::try_from(name.to_owned()).unwrap();
        connector.connect(name, tcp).await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::testing::*;
    use super::*;

    #[test]
    fn certificates_load_with_their_keys() {
        let dir = dir("load");
        let (cert, key, _) = self_signed(&["node.example", "10.0.0.1", "*.lan.example"]);
        let (cert_path, key_path) = write(&dir, &cert, &key);
        let c = Certificate::load(&cert_path, &key_path).unwrap();
        assert_eq!(c.dns_names(), ["node.example"]);
        for host in ["node.example", "NODE.example", "10.0.0.1", "a.lan.example"] {
            assert!(c.valid_for(host), "{host}");
        }
        for host in ["other.example", "10.0.0.2", "[::1]", "", "a b"] {
            assert!(!c.valid_for(host), "{host}");
        }
        let (tls, _) = Tls::load(&TlsConfig::new(&cert_path, &key_path)).unwrap();
        assert!(tls.reload().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ipv6_hosts_are_named_with_or_without_brackets() {
        let dir = dir("ipv6");
        let (cert, key, _) = self_signed(&["::1"]);
        let (cert_path, key_path) = write(&dir, &cert, &key);
        let c = Certificate::load(&cert_path, &key_path).unwrap();
        assert!(c.valid_for("::1"));
        assert!(c.valid_for("[::1]"));
        assert!(c.dns_names().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn files_that_hold_no_matching_pair_are_refused() {
        let dir = dir("refused");
        let (cert, key, _) = self_signed(&["a.example"]);
        let (_, other_key, _) = self_signed(&["b.example"]);
        let (cert_path, key_path) = write(&dir, &cert, &key);
        let load = |c: &Path, k: &Path| Tls::load(&TlsConfig::new(c, k)).err().unwrap();

        let missing = dir.join("missing.pem");
        assert!(matches!(
            load(&missing, &key_path),
            TlsError::Certificates { .. }
        ));
        assert!(matches!(load(&cert_path, &missing), TlsError::Key { .. }));
        let empty = dir.join("empty.pem");
        std::fs::write(&empty, "").unwrap();
        assert!(matches!(
            load(&empty, &key_path),
            TlsError::NoCertificate(_)
        ));
        // A key file holding only a certificate holds no key.
        assert!(matches!(load(&cert_path, &cert_path), TlsError::Key { .. }));
        let garbled = dir.join("garbled.pem");
        std::fs::write(&garbled, cert.replacen("\nMII", "\n!II", 1)).unwrap();
        assert!(matches!(
            load(&garbled, &key_path),
            TlsError::Certificates { .. }
        ));

        std::fs::write(&key_path, &other_key).unwrap();
        let e = load(&cert_path, &key_path);
        assert!(matches!(e, TlsError::Rejected(_)), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_change_of_either_file_loads_the_pair_again() {
        let dir = dir("reload");
        let (cert, key, _) = self_signed(&["a.example"]);
        let (cert_path, key_path) = write(&dir, &cert, &key);
        let (tls, _) = Tls::load(&TlsConfig::new(&cert_path, &key_path)).unwrap();
        let before = tls.hls();

        // A renewal written one file at a time: the new certificate with
        // the old key fails once, then loads with its key.
        let (renewed, renewed_key, _) = self_signed(&["b.example", "a.example"]);
        std::fs::write(&cert_path, &renewed).unwrap();
        assert!(matches!(tls.reload(), Some(Err(TlsError::Rejected(_)))));
        assert!(tls.reload().is_none(), "a failed pair is tried once");
        assert!(Arc::ptr_eq(before.config(), tls.hls().config()));
        // File times may not move between writes this close together, so
        // the renewed key differs in length too.
        std::fs::write(&key_path, format!("{renewed_key}\n")).unwrap();
        let cert = tls.reload().unwrap().unwrap();
        assert_eq!(cert.dns_names(), ["b.example", "a.example"]);
        assert!(!Arc::ptr_eq(before.config(), tls.hls().config()));
        assert!(Arc::ptr_eq(
            &before.config().session_storage,
            &tls.api().config().session_storage
        ));
        assert!(tls.reload().is_none());

        // A file that goes missing keeps the certificate.
        std::fs::remove_file(&key_path).unwrap();
        assert!(matches!(tls.reload(), Some(Err(TlsError::Key { .. }))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A Kubernetes secret volume: the files are symlinks through `..data`,
    /// which kubelet points at a new directory of the renewed pair with one
    /// rename. The paths never change; what they resolve to does.
    #[cfg(unix)]
    #[test]
    fn a_secret_volume_whose_data_link_moves_loads_the_renewed_pair() {
        use std::os::unix::fs::symlink;

        let dir = dir("secret-volume");
        let version = |name: &str, cert: &str, key: &str| {
            let v = dir.join(name);
            std::fs::create_dir(&v).unwrap();
            std::fs::write(v.join("tls.crt"), cert).unwrap();
            std::fs::write(v.join("tls.key"), key).unwrap();
        };
        let (cert, key, _) = self_signed(&["a.example"]);
        version("..2026_10_31_00_00_00.1", &cert, &key);
        symlink("..2026_10_31_00_00_00.1", dir.join("..data")).unwrap();
        symlink("..data/tls.crt", dir.join("tls.crt")).unwrap();
        symlink("..data/tls.key", dir.join("tls.key")).unwrap();
        let (cert_path, key_path) = (dir.join("tls.crt"), dir.join("tls.key"));
        let (tls, _) = Tls::load(&TlsConfig::new(&cert_path, &key_path)).unwrap();
        assert!(tls.reload().is_none());

        // The renewed certificate names one more host, so its length
        // differs even where file times don't move this close together.
        let (renewed, renewed_key, _) = self_signed(&["b.example", "a.example"]);
        version("..2026_12_01_00_00_00.2", &renewed, &renewed_key);
        symlink("..2026_12_01_00_00_00.2", dir.join("..data_tmp")).unwrap();
        std::fs::rename(dir.join("..data_tmp"), dir.join("..data")).unwrap();
        std::fs::remove_dir_all(dir.join("..2026_10_31_00_00_00.1")).unwrap();

        let cert = tls.reload().unwrap().unwrap();
        assert_eq!(cert.dns_names(), ["b.example", "a.example"]);
        assert!(tls.reload().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
