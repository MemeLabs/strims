// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! RPC over vpn: the network services' calls between hosts of a network.
//!
//! A connection runs on one vpn port and serves and calls any number of
//! remotes, each a [`HostAddr`]. Every Call is one encrypted vpn message,
//! unprefixed ([`Framing::Message`]), and carries the sender's host
//! certificate in the [`CERTIFICATE_HEADER`] header: requests, answers and
//! cancels alike. Calls are keyed by (host, port, id), and an answer is
//! accepted only from the address its call went to.
//!
//! # Callers
//!
//! A host certificate is exactly three levels: the host's key, signed by
//! the profile's membership certificate, signed by the network's root
//! ([`strims_crypto::host_certificate`] makes one). A delivery reaches the
//! endpoint only if:
//!
//! 1. its trailer signatures didn't fail verification;
//! 2. it was encrypted to this host and isn't a broadcast, so decrypting
//!    it proved the sender holds the [`Delivery::src`] host key;
//! 3. it fits [`Config::max_message_bytes`] and decodes as a Call;
//! 4. its certificate header is at most
//!    [`VpnConfig::max_certificate_bytes`] and decodes;
//! 5. the certificate is for [`Delivery::src`], has the three levels, and
//!    its root is this connection's network root;
//! 6. its chain verifies at [`Delivery::received`].
//!
//! Handlers get the caller's certificate from
//! [`Context::certificate`](strims_proto::service::Context::certificate);
//! its parent is the caller's membership certificate. Signatures are
//! checked again only when a host's certificate bytes change; validity is
//! checked on every Call.
//!
//! # Hostile remotes
//!
//! Remotes share the port's endpoint, so nothing a remote sends closes it:
//! input that fails a check above is dropped and counted in [`Metrics`].
//! Each remote holds at most
//! [`Config::max_inbound_calls_per_remote`] requests in service, and
//! addresses are bounded per port ([`VpnConfig::max_remotes`]) and per
//! host ([`VpnConfig::max_remotes_per_host`]). An address that holds no
//! call or queued Call can be forgotten to make room; the addresses of
//! live [`Client`]s are kept, outside both bounds.
//!
//! # Driver contract
//!
//! The reader yields the port's [`Delivery`]s; the writer takes each Call
//! with its destination and sends it encrypted (`M_STD_FLAGS`) from the
//! port. The transport is lossy: a lost request leaves its call waiting
//! until the caller's deadline drops it.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll};

use futures_core::Stream;
use futures_sink::Sink;
use prost::Message;
use strims_base::time::Time;
use strims_crypto::{certificate_parent, chain_validity, verify_certificate};
use strims_kademlia::ID;
use strims_proto::rpc::Call;
use strims_proto::types::Certificate;
use strims_vpn::{
    DEFAULT_MAX_MESSAGE_LEN, Delivery, HostAddr, M_BROADCAST, M_ENCRYPT, MAX_RELAYED_BODY_LEN,
};

use crate::client::Client;
use crate::connection::{ConnectionError, Core, Io, poll_flush};
use crate::endpoint::{Config, Endpoint, RemoteId};
use crate::router::Router;
use crate::shared::{Shared, SharedRef};
use crate::wire::Framing;

/// The header that carries the sender's host certificate.
pub const CERTIFICATE_HEADER: &str = "certificate";

impl Config {
    /// The endpoint defaults for a vpn port: one Call per message, Calls
    /// that every relay can forward, and a share of the request slots per
    /// remote.
    pub fn vpn() -> Config {
        Config {
            framing: Framing::Message,
            max_message_bytes: DEFAULT_MAX_MESSAGE_LEN,
            max_send_bytes: MAX_RELAYED_BODY_LEN,
            max_inbound_calls_per_remote: 16,
            ..Config::default()
        }
    }
}

/// The limits of a vpn connection.
#[derive(Clone, Debug)]
pub struct VpnConfig {
    /// The endpoint's limits.
    pub rpc: Config,
    /// Addresses kept for remotes, besides those of live clients.
    pub max_remotes: usize,
    /// Of those, addresses of one host.
    pub max_remotes_per_host: usize,
    /// Hosts whose verified certificate is kept.
    pub max_certificates: usize,
    /// The longest certificate header accepted.
    pub max_certificate_bytes: usize,
}

impl Default for VpnConfig {
    fn default() -> VpnConfig {
        VpnConfig {
            rpc: Config::vpn(),
            max_remotes: 256,
            max_remotes_per_host: 4,
            max_certificates: 256,
            max_certificate_bytes: 4096,
        }
    }
}

/// Deliveries by outcome; see the module docs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Deliveries read.
    pub received: u64,
    /// Calls handed to the endpoint.
    pub admitted: u64,
    /// Deliveries whose trailer signatures failed.
    pub unverified: u64,
    /// Deliveries not encrypted to this host, or broadcast.
    pub unencrypted: u64,
    /// Deliveries over the endpoint's message limit.
    pub oversized: u64,
    /// Bodies that didn't decode as a Call.
    pub malformed: u64,
    /// Calls without a certificate header, or with one over the limit.
    pub missing_certificate: u64,
    /// Certificates that didn't decode, aren't the sender's, or aren't a
    /// host certificate on this network.
    pub rejected_certificate: u64,
    /// Certificates whose chain didn't verify when the Call arrived.
    pub invalid_certificate: u64,
    /// Calls from a new address while the address table was full of
    /// remotes in use.
    pub no_room: u64,
}

/// Why a certificate can't be this connection's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum VpnError {
    /// The certificate isn't host → membership → network root.
    #[error("not a host certificate")]
    NotHostCertificate,
    /// A renewed certificate is for another network.
    #[error("certificate for another network")]
    NetworkMismatch,
}

/// The root of a host certificate: the certificate's grandparent, which
/// has no parent of its own.
fn host_certificate_root(cert: &Certificate) -> Option<&Certificate> {
    let root = certificate_parent(certificate_parent(cert)?)?;
    root.parent_oneof.is_none().then_some(root)
}

fn certificate_headers(cert: &Certificate) -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([(CERTIFICATE_HEADER.to_owned(), cert.encode_to_vec())])
}

#[derive(Debug)]
struct Remote {
    id: RemoteId,
    /// Live while a client calls the address.
    pin: Weak<()>,
}

impl Remote {
    fn is_pinned(&self) -> bool {
        self.pin.strong_count() > 0
    }
}

#[derive(Debug)]
struct CachedCertificate {
    bytes: Vec<u8>,
    certificate: Rc<Certificate>,
    /// [`chain_validity`] of the certificate.
    valid: (Time, Time),
    /// Insertion order, for eviction.
    stamp: u64,
}

/// The sans-IO state of a vpn port: remote addresses and caller checks.
/// It lives in the connection's [`Shared`] state next to the endpoint.
#[derive(Debug)]
pub(crate) struct Port {
    config: VpnConfig,
    network_key: Vec<u8>,
    /// Bounded by `max_remotes` unpinned entries plus the pinned ones.
    remotes: BTreeMap<HostAddr, Remote>,
    addrs: BTreeMap<RemoteId, HostAddr>,
    next_remote: u64,
    /// Bounded by `max_certificates`.
    certificates: BTreeMap<ID, CachedCertificate>,
    next_stamp: u64,
    metrics: Metrics,
}

impl Port {
    fn new(config: VpnConfig, network_key: Vec<u8>) -> Port {
        Port {
            config,
            network_key,
            remotes: BTreeMap::new(),
            addrs: BTreeMap::new(),
            next_remote: 1,
            certificates: BTreeMap::new(),
            next_stamp: 0,
            metrics: Metrics::default(),
        }
    }

    /// Checks a delivery and hands its Call to `endpoint`.
    pub(crate) fn admit<H>(&mut self, endpoint: &mut Endpoint, d: &Delivery<H>) {
        self.metrics.received += 1;
        if d.verified == Some(false) {
            self.metrics.unverified += 1;
            return;
        }
        if d.flags & M_ENCRYPT == 0 || d.flags & M_BROADCAST != 0 {
            self.metrics.unencrypted += 1;
            return;
        }
        if d.body.len() > endpoint.config().max_message_bytes {
            self.metrics.oversized += 1;
            return;
        }
        let Ok(call) = Call::decode(d.body.as_slice()) else {
            self.metrics.malformed += 1;
            return;
        };
        let Some(certificate) = self.check_certificate(&call, d.src, d.received) else {
            return;
        };
        let Some(remote) = self.inbound_remote(endpoint, HostAddr::new(d.src, d.src_port)) else {
            self.metrics.no_room += 1;
            return;
        };
        self.metrics.admitted += 1;
        endpoint.handle_call(remote, call, Some(certificate));
    }

    fn check_certificate(&mut self, call: &Call, src: ID, at: Time) -> Option<Rc<Certificate>> {
        let Some(bytes) = call
            .headers
            .get(CERTIFICATE_HEADER)
            .filter(|b| b.len() <= self.config.max_certificate_bytes)
        else {
            self.metrics.missing_certificate += 1;
            return None;
        };
        let valid_at = |(from, until): (Time, Time)| !at.before(from) && !at.after(until);
        if let Some(c) = self.certificates.get(&src)
            && c.bytes == *bytes
        {
            if valid_at(c.valid) {
                return Some(c.certificate.clone());
            }
            self.metrics.invalid_certificate += 1;
            return None;
        }
        let Ok(cert) = Certificate::decode(bytes.as_slice()) else {
            self.metrics.rejected_certificate += 1;
            return None;
        };
        let on_network =
            host_certificate_root(&cert).is_some_and(|root| root.key == self.network_key);
        if cert.key != src.to_bytes() || !on_network {
            self.metrics.rejected_certificate += 1;
            return None;
        }
        // the structure check bounds this to three signatures
        if verify_certificate(&cert, at).is_err() {
            self.metrics.invalid_certificate += 1;
            return None;
        }
        let certificate = Rc::new(cert);
        if !self.certificates.contains_key(&src)
            && self.certificates.len() >= self.config.max_certificates
        {
            let oldest = self
                .certificates
                .iter()
                .min_by_key(|(_, c)| c.stamp)
                .map(|(id, _)| *id);
            if let Some(id) = oldest {
                self.certificates.remove(&id);
            }
        }
        self.certificates.insert(
            src,
            CachedCertificate {
                bytes: bytes.clone(),
                valid: chain_validity(&certificate),
                certificate: certificate.clone(),
                stamp: self.next_stamp,
            },
        );
        self.next_stamp += 1;
        Some(certificate)
    }

    /// The remote of a Call from `addr`, if there's room for it.
    fn inbound_remote(&mut self, endpoint: &Endpoint, addr: HostAddr) -> Option<RemoteId> {
        if let Some(r) = self.remotes.get(&addr) {
            return Some(r.id);
        }
        let host = HostAddr::new(addr.host, 0)..=HostAddr::new(addr.host, u16::MAX);
        let per_host = self.config.max_remotes_per_host;
        if !self.make_room(endpoint, Some(host), per_host)
            || !self.make_room(endpoint, None, self.config.max_remotes)
        {
            return None;
        }
        Some(self.insert(addr, Weak::new()))
    }

    /// The remote of a client's calls to `addr`, and the pin that keeps its
    /// address while the client exists.
    pub(crate) fn pin(&mut self, endpoint: &Endpoint, addr: HostAddr) -> (RemoteId, Rc<()>) {
        if let Some(r) = self.remotes.get_mut(&addr) {
            let pin = r.pin.upgrade().unwrap_or_else(|| {
                let pin = Rc::new(());
                r.pin = Rc::downgrade(&pin);
                pin
            });
            return (r.id, pin);
        }
        // pinned addresses are outside the bound, but take the chance to
        // forget idle ones that clients no longer use
        self.make_room(endpoint, None, self.config.max_remotes);
        let pin = Rc::new(());
        (self.insert(addr, Rc::downgrade(&pin)), pin)
    }

    /// Forgets idle unpinned addresses in `range` (every address if
    /// `None`) until fewer than `max` unpinned ones remain there. Returns
    /// whether that succeeded.
    fn make_room(
        &mut self,
        endpoint: &Endpoint,
        range: Option<std::ops::RangeInclusive<HostAddr>>,
        max: usize,
    ) -> bool {
        let in_range = |a: &HostAddr| range.as_ref().is_none_or(|r| r.contains(a));
        let mut unpinned = self
            .remotes
            .iter()
            .filter(|(a, r)| in_range(a) && !r.is_pinned())
            .count();
        while unpinned >= max {
            let idle = self
                .remotes
                .iter()
                .find(|(a, r)| in_range(a) && !r.is_pinned() && endpoint.is_remote_idle(r.id))
                .map(|(a, r)| (*a, r.id));
            let Some((addr, id)) = idle else {
                return false;
            };
            self.remotes.remove(&addr);
            self.addrs.remove(&id);
            unpinned -= 1;
        }
        true
    }

    fn insert(&mut self, addr: HostAddr, pin: Weak<()>) -> RemoteId {
        let id = RemoteId(self.next_remote);
        self.next_remote += 1;
        self.remotes.insert(addr, Remote { id, pin });
        self.addrs.insert(id, addr);
        id
    }

    /// Where `remote`'s Calls go.
    pub(crate) fn addr(&self, remote: RemoteId) -> Option<HostAddr> {
        self.addrs.get(&remote).copied()
    }
}

/// Runs RPC on a vpn port with this host's `certificate` for the network,
/// serving `router`'s methods (or none, as a client only) and calling
/// through the returned [`VpnClient`]. See the module docs.
///
/// The connection does its work when the returned future is polled, and
/// ends as [`crate::connect`]'s does: at the end of the deliveries or on a
/// transport error.
pub fn connect<R, W, H, E>(
    config: VpnConfig,
    certificate: &Certificate,
    reader: R,
    writer: W,
    router: Option<Router>,
) -> Result<(VpnClient, VpnConnection<R, W>), VpnError>
where
    R: Stream<Item = Result<Delivery<H>, E>> + Unpin,
    W: Sink<(HostAddr, Vec<u8>), Error = E> + Unpin,
{
    let network_key = host_certificate_root(certificate)
        .ok_or(VpnError::NotHostCertificate)?
        .key
        .clone();
    let mut rpc = config.rpc.clone();
    rpc.serve &= router.is_some();
    let mut endpoint = Endpoint::new(rpc);
    endpoint.set_headers(&certificate_headers(certificate));
    let shared = Shared::new(endpoint);
    shared.borrow_mut().vpn = Some(Port::new(config, network_key));
    let client = VpnClient {
        shared: shared.clone(),
    };
    let conn = VpnConnection {
        core: Core::new(shared, router),
        io: VpnIo {
            reader,
            writer,
            flush: false,
        },
    };
    Ok((client, conn))
}

/// Makes calls through a vpn connection; see [`connect`]. Clones share the
/// connection.
#[derive(Clone)]
pub struct VpnClient {
    shared: SharedRef,
}

impl fmt::Debug for VpnClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VpnClient").finish_non_exhaustive()
    }
}

impl VpnClient {
    /// A client that calls the server at `addr`. The connection keeps the
    /// address while the client or a clone of it exists.
    pub fn to(&self, addr: HostAddr) -> Client {
        let pinned = {
            let mut s = self.shared.borrow_mut();
            let s = &mut *s;
            s.vpn.as_mut().map(|port| port.pin(&s.endpoint, addr))
        };
        match pinned {
            Some((remote, pin)) => Client::to_remote(self.shared.clone(), remote, Some(pin)),
            // invariant: `connect` gives every vpn client a port
            None => Client::new(self.shared.clone()),
        }
    }

    /// Replaces this host's certificate, e.g. when its membership is
    /// renewed. Calls sent from now on carry the new one.
    pub fn set_certificate(&self, certificate: &Certificate) -> Result<(), VpnError> {
        let root = host_certificate_root(certificate).ok_or(VpnError::NotHostCertificate)?;
        let mut s = self.shared.borrow_mut();
        let s = &mut *s;
        if s.vpn.as_ref().is_some_and(|p| p.network_key != root.key) {
            return Err(VpnError::NetworkMismatch);
        }
        s.endpoint.set_headers(&certificate_headers(certificate));
        Ok(())
    }
}

/// A running vpn connection; see [`connect`].
#[must_use = "a connection does nothing unless polled"]
pub struct VpnConnection<R, W> {
    core: Core,
    io: VpnIo<R, W>,
}

impl<R, W> fmt::Debug for VpnConnection<R, W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.core.fmt("VpnConnection", f)
    }
}

impl<R, W> VpnConnection<R, W> {
    /// The endpoint's counters.
    pub fn metrics(&self) -> crate::Metrics {
        self.core.metrics()
    }

    /// The deliveries' outcomes.
    pub fn vpn_metrics(&self) -> Metrics {
        let s: &RefCell<Shared> = self.core.shared();
        s.borrow()
            .vpn
            .as_ref()
            .map(|p| p.metrics)
            .unwrap_or_default()
    }
}

impl<R, W, H, E> Future for VpnConnection<R, W>
where
    R: Stream<Item = Result<Delivery<H>, E>> + Unpin,
    W: Sink<(HostAddr, Vec<u8>), Error = E> + Unpin,
{
    type Output = Result<(), ConnectionError<E>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.core.poll(cx, &mut this.io)
    }
}

/// A vpn port's deliveries in, addressed Calls out.
struct VpnIo<R, W> {
    reader: R,
    writer: W,
    /// The sink has items that haven't been flushed.
    flush: bool,
}

impl<R, W, H, E> Io for VpnIo<R, W>
where
    R: Stream<Item = Result<Delivery<H>, E>> + Unpin,
    W: Sink<(HostAddr, Vec<u8>), Error = E> + Unpin,
{
    type Error = E;

    fn poll_write(&mut self, cx: &mut Context<'_>, shared: &SharedRef) -> Result<bool, E> {
        let mut wrote = false;
        loop {
            if shared.borrow().endpoint.transmit_buffered() == 0 {
                break;
            }
            match Pin::new(&mut self.writer).poll_ready(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(e)) => return Err(e),
                Poll::Pending => break,
            }
            let next = {
                let mut s = shared.borrow_mut();
                let s = &mut *s;
                s.endpoint.poll_transmit_to().map(|(remote, frame)| {
                    // invariant: a remote with queued Calls keeps its address
                    let addr = s.vpn.as_ref().and_then(|p| p.addr(remote));
                    (addr, frame)
                })
            };
            let Some((addr, frame)) = next else {
                break;
            };
            wrote = true;
            if let Some(addr) = addr {
                Pin::new(&mut self.writer).start_send((addr, frame))?;
                self.flush = true;
            }
        }
        poll_flush(&mut self.writer, &mut self.flush, cx)?;
        Ok(wrote)
    }

    fn poll_read(
        &mut self,
        cx: &mut Context<'_>,
        shared: &SharedRef,
    ) -> Poll<Option<Result<(), ConnectionError<E>>>> {
        match Pin::new(&mut self.reader).poll_next(cx) {
            Poll::Ready(Some(Ok(delivery))) => {
                {
                    let mut s = shared.borrow_mut();
                    let s = &mut *s;
                    if let Some(port) = s.vpn.as_mut() {
                        port.admit(&mut s.endpoint, &delivery);
                    }
                }
                drop(delivery);
                Poll::Ready(Some(Ok(())))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(ConnectionError::Read(e)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
