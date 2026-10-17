// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The services a network's members reach over vpn RPC: its CA, served by
//! the device that holds the network's server config, and the client port
//! this side calls them from. Certificate renewal, certificate searches
//! and the forwarding of peers' renewals run on them.
//!
//! # The CA server
//!
//! The device holding a network's server config serves `strims.network.
//! v1.ca.CA` on a vpn port while it holds the store's lease for it (one
//! process at a time), and publishes the port in the network's hash table
//! under the network's key and [`CA_SALT`]. Callers are members: vpn RPC
//! admits only calls with a host certificate under the network's root.
//!
//! # Renewal
//!
//! This side renews its certificate on a network a week before it
//! expires, and at once when its alias differs from the certificate's
//! subject. It signs with the server config when it holds it, calls the
//! CA over vpn when its certificate is trusted, and otherwise asks a peer
//! whose certificate is: that peer's `CAPeer.Renew` forwards the request.
//! The outcome, and its error code, go into the profile's record of the
//! network.
//!
//! A peer may forward only for its own certificate on a network the last
//! run found shared, one at a time, at most one per
//! [`FORWARD_INTERVAL`], with [`MAX_FORWARDS`] in flight across peers.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::future::{Future, poll_fn};
use std::pin::pin;
use std::rc::{Rc, Weak};
use std::task::{Poll, Waker};

use futures_util::FutureExt;
use futures_util::future::{AbortHandle, abortable};
use strims_base::rng::fork;
use strims_base::time::{Duration, Time};
use strims_crypto::{
    certificate_network_key, host_certificate, membership_trusted, new_certificate_request,
    verify_certificate,
};
use strims_dao::key::Part;
use strims_dao::schema::network::Networks;
use strims_dao::{Table, lease};
use strims_kademlia::ID;
use strims_kv::Kv;
use strims_proto::network::v1::Network;
use strims_proto::network::v1::ca::ca_find_request::Query;
use strims_proto::network::v1::ca::{
    CaClient, CaFindRequest, CaFindResponse, CaPeerRenewRequest, CaPeerRenewResponse,
    CaPeerService, CaRenewRequest, CaRenewResponse, CaService, register_ca_service,
};
use strims_proto::network::v1::errors::ErrorCode;
use strims_proto::service::{CallError, Context, Error as RpcError};
use strims_proto::types::{Certificate, CertificateRequest};
use strims_rpc::vpn::{self, VpnClient, VpnConfig};
use strims_rpc::{RetryClient, RetryPolicy, Router, Timer};

use crate::ca::{self, CA_SALT, CaError, MEMBER_USAGE};
use crate::control::{Done, Shared};
use crate::host::Host;
use crate::peer::PeerService;
use crate::state::NetKey;

/// How long before expiry a certificate is renewed.
pub const RENEW_AHEAD: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// The wait before trying a failed renewal again.
pub const RENEW_RETRY: Duration = Duration::from_secs(5 * 60);

/// The least time between renewals a peer forwards.
pub const FORWARD_INTERVAL: Duration = Duration::from_secs(10);

/// Renewals forwarded at once, across peers.
pub const MAX_FORWARDS: usize = 8;

/// The CA renews a key up to [`CA_RENEW_BURST`] times at once, and
/// then once per this interval. Every renewal adds a certificate log.
pub const CA_RENEW_INTERVAL: Duration = Duration::HOUR;

/// See [`CA_RENEW_INTERVAL`].
pub const CA_RENEW_BURST: i64 = 4;

/// Keys the CA tracks renewals of.
const MAX_CA_RENEWALS: usize = 4096;

/// Calls to a peer's `CAPeer`: the peer forwards over vpn, so one attempt
/// may take as long as a vpn call.
pub(crate) fn forward_policy() -> RetryPolicy {
    RetryPolicy {
        attempt_timeout: Duration::from_secs(75),
        retries: 0,
        deadline: Duration::from_secs(75),
        ..RetryPolicy::vpn()
    }
}

/// A network's CA server, while it runs.
#[derive(Default)]
pub(crate) struct CaServerCtl {
    stop: Cell<bool>,
    waker: RefCell<Option<Waker>>,
    /// Its connection, for certificate updates.
    client: RefCell<Option<VpnClient>>,
}

impl CaServerCtl {
    pub(crate) fn stop(&self) {
        self.stop.set(true);
        if let Some(w) = self.waker.borrow_mut().take() {
            w.wake();
        }
    }
}

/// The client port for a network's services.
pub(crate) struct Dialer {
    client: VpnClient,
    abort: AbortHandle,
}

/// A network's vpn services on this side.
#[derive(Default)]
pub(crate) struct Services {
    ca: Option<Rc<CaServerCtl>>,
    dialer: Option<Dialer>,
}

/// Renewals peers forward through this side.
#[derive(Default)]
pub(crate) struct Forwards {
    /// When each peer last forwarded. Bound: the node's peers.
    last: BTreeMap<ID, Time>,
    active: BTreeSet<ID>,
}

impl Forwards {
    pub(crate) fn forget(&mut self, host: ID) {
        self.last.remove(&host);
    }
}

/// A forward in flight; dropping it ends it.
struct ForwardSlot<K: Kv, H: Host, T: Timer> {
    shared: Rc<Shared<K, H, T>>,
    host: ID,
}

impl<K: Kv, H: Host, T: Timer> Drop for ForwardSlot<K, H, T> {
    fn drop(&mut self) {
        self.shared.forwards.borrow_mut().active.remove(&self.host);
    }
}

/// When this side renews its certificate on `n`: `None` while it can only
/// renew through a peer.
pub(crate) fn renew_time(n: &Network, now: Time) -> Option<Time> {
    let cert = n.certificate.as_ref()?;
    if n.server_config.is_none() && !membership_trusted(cert, now) {
        return None;
    }
    if !n.alias.is_empty() && n.alias != cert.subject {
        return Some(now);
    }
    let not_after = Time::from_unix(i64::try_from(cert.not_after).unwrap_or(i64::MAX), 0);
    Some((not_after - RENEW_AHEAD).max(now))
}

fn remote_error(e: CallError) -> (ErrorCode, String) {
    match e {
        CallError::Remote(e) => {
            let code = ErrorCode::try_from(e.code)
                .ok()
                .filter(|c| *c != ErrorCode::Undefined)
                .unwrap_or(ErrorCode::Unknown);
            (code, e.message)
        }
        e => (ErrorCode::Unknown, e.to_string()),
    }
}

fn to_rpc((code, message): (ErrorCode, String)) -> RpcError {
    RpcError::new(code, message)
}

impl<K: Kv, H: Host, T: Timer> Shared<K, H, T> {
    /// This host's certificate on the network, under this side's
    /// membership.
    pub(crate) fn host_certificate(&self, key: &[u8]) -> Option<Certificate> {
        let st = self.state.borrow();
        let membership = st.certificate(key)?;
        let profile_key = self.profile.key.as_ref()?;
        host_certificate(&self.host.host_id().to_bytes(), profile_key, membership).ok()
    }

    /// Starts what a network needs once the node has joined it: its CA
    /// server, if this side holds the server config.
    pub(crate) fn start_services(self: &Rc<Self>, key: &[u8]) {
        let wants_ca = self
            .records
            .borrow()
            .get(key)
            .is_some_and(|n| n.server_config.is_some());
        let joined = self.state.borrow().network(key).is_some();
        let mut services = self.services.borrow_mut();
        let svc = services.entry(key.to_vec()).or_default();
        match (&svc.ca, wants_ca && joined) {
            (None, true) => {
                let ctl = Rc::new(CaServerCtl::default());
                svc.ca = Some(ctl.clone());
                self.spawn(ca_server(self.clone(), key.to_vec(), ctl).boxed_local());
            }
            (Some(ctl), false) => {
                ctl.stop();
                svc.ca = None;
            }
            _ => {}
        }
    }

    /// Stops a network's services.
    pub(crate) fn stop_services(&self, key: &[u8]) {
        let Some(svc) = self.services.borrow_mut().remove(key) else {
            return;
        };
        if let Some(ctl) = svc.ca {
            ctl.stop();
        }
        if let Some(d) = svc.dialer {
            d.abort.abort();
        }
    }

    /// Passes a new certificate on to a network's vpn connections.
    pub(crate) fn update_services(&self, key: &[u8]) {
        let Some(cert) = self.host_certificate(key) else {
            return;
        };
        let services = self.services.borrow();
        let Some(svc) = services.get(key) else {
            return;
        };
        let clients = svc
            .ca
            .iter()
            .filter_map(|c| c.client.borrow().clone())
            .chain(svc.dialer.as_ref().map(|d| d.client.clone()));
        for c in clients {
            if let Err(e) = c.set_certificate(&cert) {
                tracing::debug!(error = %e, "updating a vpn certificate failed");
            }
        }
    }

    /// A client of the network's CA, over the network's client port.
    async fn ca_client(
        self: &Rc<Self>,
        key: &[u8],
    ) -> Result<CaClient<RetryClient<T>>, (ErrorCode, String)> {
        let fail = |m: String| (ErrorCode::Unknown, m);
        let net = self
            .state
            .borrow()
            .network(key)
            .map(|(_, net)| net)
            .ok_or_else(|| fail("network not joined".into()))?;
        let existing = self
            .services
            .borrow()
            .get(key)
            .and_then(|s| s.dialer.as_ref().map(|d| d.client.clone()));
        let client = match existing {
            Some(c) => c,
            None => {
                let cert = self
                    .host_certificate(key)
                    .ok_or_else(|| fail("no host certificate".into()))?;
                let (_, reader, writer) = self
                    .host
                    .open_vpn_port(net)
                    .await
                    .map_err(|e| fail(e.to_string()))?;
                let (client, conn) =
                    vpn::connect(VpnConfig::default(), &cert, reader, writer, None)
                        .map_err(|e| fail(e.to_string()))?;
                let (conn, abort) = abortable(conn);
                self.spawn(
                    async move {
                        let _ = conn.await;
                        Done::Service
                    }
                    .boxed_local(),
                );
                let mut services = self.services.borrow_mut();
                let svc = services.entry(key.to_vec()).or_default();
                match &svc.dialer {
                    Some(d) => {
                        abort.abort();
                        d.client.clone()
                    }
                    None => {
                        svc.dialer = Some(Dialer {
                            client: client.clone(),
                            abort,
                        });
                        client
                    }
                }
            }
        };
        let addr = self
            .host
            .resolve_address(net, key.to_vec(), CA_SALT.to_vec())
            .await
            .map_err(|e| fail(e.to_string()))?
            .ok_or_else(|| fail("the network's CA isn't published".into()))?;
        let retry = RetryClient::new(client.to(addr), self.timer.clone(), RetryPolicy::vpn());
        Ok(CaClient::new(retry))
    }

    /// Renews `cert` for `csr` at the network's CA: this side's, or over
    /// vpn.
    async fn renew_at_ca(
        self: &Rc<Self>,
        key: &[u8],
        cert: &Certificate,
        csr: &CertificateRequest,
    ) -> Result<Certificate, (ErrorCode, String)> {
        let n = self
            .records
            .borrow()
            .get(key)
            .cloned()
            .ok_or((ErrorCode::Unknown, "network not found".to_owned()))?;
        if n.server_config.is_some() {
            return self.renew_locally(&n, cert, csr).await;
        }
        let client = self.ca_client(key).await?;
        let req = CaRenewRequest {
            certificate: Some(cert.clone()),
            certificate_request: Some(csr.clone()),
        };
        let res = client.renew(&req).await.map_err(remote_error)?;
        res.certificate
            .ok_or((ErrorCode::Unknown, "no certificate".to_owned()))
    }

    /// Finds the certificate the network's CA issued that matches
    /// `query`: at this side's CA, or over vpn. See
    /// [`NetworkControl::find_certificate`](crate::NetworkControl::find_certificate).
    pub(crate) async fn find_at_ca(
        self: &Rc<Self>,
        key: &[u8],
        query: Query,
        full_chain: bool,
    ) -> Result<Certificate, RpcError> {
        let n = self
            .records
            .borrow()
            .get(key)
            .cloned()
            .ok_or_else(|| RpcError::new(ErrorCode::Unknown, "network not found"))?;
        if n.server_config.is_some() {
            return ca::find(&self.store, n.id, Some(&query), full_chain)
                .await
                .map_err(|e| e.to_rpc());
        }
        // vpn RPC admits only host certificates under a trusted membership
        let trusted = self
            .state
            .borrow()
            .certificate(key)
            .is_some_and(|c| membership_trusted(c, self.timer.now()));
        if !trusted {
            return Err(RpcError::new(
                ErrorCode::Unknown,
                "membership not trusted by the network's CA",
            ));
        }
        let client = self.ca_client(key).await.map_err(to_rpc)?;
        let req = CaFindRequest {
            query: Some(query),
            full_chain,
        };
        let res = client
            .find(&req)
            .await
            .map_err(|e| to_rpc(remote_error(e)))?;
        res.certificate
            .ok_or_else(|| RpcError::new(ErrorCode::Unknown, "no certificate"))
    }

    /// Renews at this side's CA, under its rate limit.
    async fn renew_locally(
        &self,
        n: &Network,
        cert: &Certificate,
        csr: &CertificateRequest,
    ) -> Result<Certificate, (ErrorCode, String)> {
        let err = |e: CaError| (e.code(), e.to_string());
        let now = self.timer.now();
        self.ca_rate_limit(&csr.key, now).map_err(err)?;
        let mut rng = fork(&mut self.rng.borrow_mut());
        ca::renew(&self.store, n, cert, csr, now, &mut rng)
            .await
            .map_err(err)
    }

    /// Admits a renewal for `key` under the CA's limit: each key's
    /// theoretical arrival time advances by [`CA_RENEW_INTERVAL`] per
    /// renewal and may run [`CA_RENEW_BURST`] intervals ahead.
    fn ca_rate_limit(&self, key: &[u8], now: Time) -> Result<(), CaError> {
        let mut renewals = self.ca_renewals.borrow_mut();
        let tat = renewals.get(key).copied().unwrap_or(now).max(now);
        if tat.after(now + CA_RENEW_INTERVAL * (CA_RENEW_BURST - 1)) {
            return Err(CaError::RateLimited);
        }
        if renewals.len() >= MAX_CA_RENEWALS && !renewals.contains_key(key) {
            renewals.retain(|_, t| t.after(now));
            let oldest = renewals
                .iter()
                .min_by_key(|(_, t)| **t)
                .map(|(k, _)| k.clone());
            if renewals.len() >= MAX_CA_RENEWALS
                && let Some(k) = oldest
            {
                renewals.remove(&k);
            }
        }
        renewals.insert(key.to_vec(), tat + CA_RENEW_INTERVAL);
        Ok(())
    }

    fn forward_slot(self: &Rc<Self>, host: ID) -> Result<ForwardSlot<K, H, T>, RpcError> {
        let now = self.timer.now();
        let mut f = self.forwards.borrow_mut();
        let recent = f
            .last
            .get(&host)
            .is_some_and(|t| now.before(*t + FORWARD_INTERVAL));
        if recent || f.active.contains(&host) || f.active.len() >= MAX_FORWARDS {
            return Err(CaError::RateLimited.to_rpc());
        }
        f.active.insert(host);
        f.last.insert(host, now);
        Ok(ForwardSlot {
            shared: self.clone(),
            host,
        })
    }
}

/// Renews this side's certificate on the network whose key is `key`, if
/// no renewal of it is in flight: through the peer `via` when given, and
/// otherwise as the [module docs](self) say.
pub(crate) async fn renew<K: Kv, H: Host, T: Timer>(
    s: Rc<Shared<K, H, T>>,
    key: NetKey,
    via: Option<(ID, u64)>,
) -> Done {
    if !s.renewing.borrow_mut().insert(key.clone()) {
        return Done::Service;
    }
    let outcome = renew_once(&s, &key, via).await;
    s.renewing.borrow_mut().remove(&key);
    let Some((id, outcome)) = outcome else {
        return Done::Service;
    };
    let now = s.timer.now();
    let (certificate, code) = match outcome {
        Ok(c) => (Some(c), ErrorCode::Undefined),
        Err((code, message)) => {
            tracing::info!(network = id, error = %message, "certificate renewal failed");
            s.update(|st, now| st.schedule_renewal(&key, Some(now + RENEW_RETRY)));
            (None, code)
        }
    };
    let written = async {
        let mut tx = s.store.write(now).await?;
        let Some(mut n) = Networks::get(&mut tx, id).await? else {
            return Ok(());
        };
        n.certificate_renewal_error = code.into();
        if let Some(c) = certificate {
            n.certificate = Some(c);
        }
        Networks::update(&mut tx, n).await?;
        tx.commit().await
    }
    .await;
    if let Err(e) = written {
        tracing::warn!(error = %e, "storing a renewed certificate failed");
    }
    Done::Service
}

/// One renewal: the network's id and the outcome, or `None` with no way
/// to renew.
async fn renew_once<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    key: &[u8],
    via: Option<(ID, u64)>,
) -> Option<(u64, Result<Certificate, (ErrorCode, String)>)> {
    let n = s.records.borrow().get(key).cloned()?;
    let cert = n.certificate.clone()?;
    let profile_key = s.profile.key.as_ref()?;
    let subject = if n.alias.is_empty() {
        &s.profile.name
    } else {
        &n.alias
    };
    let csr = new_certificate_request(profile_key, MEMBER_USAGE, subject).ok()?;
    let now = s.timer.now();
    let trusted = membership_trusted(&cert, now);
    let via = via.or_else(|| {
        let peers = s.state.borrow().trusted_peers(key);
        peers.first().copied()
    });
    let res = if n.server_config.is_some() || trusted {
        s.renew_at_ca(key, &cert, &csr).await
    } else if let Some((host, link)) = via {
        let client = s.ca_peer_client(host, link)?;
        let req = CaPeerRenewRequest {
            certificate: Some(cert.clone()),
            certificate_request: Some(csr.clone()),
        };
        match client.renew(&req).await {
            Ok(r) => r
                .certificate
                .ok_or((ErrorCode::Unknown, "no certificate".to_owned())),
            Err(e) => Err(remote_error(e)),
        }
    } else {
        return None;
    };
    let res = res.and_then(|c| {
        let ok = verify_certificate(&c, s.timer.now()).is_ok()
            && c.key == profile_key.public
            && certificate_network_key(&c) == key;
        if ok {
            Ok(c)
        } else {
            Err((ErrorCode::Unknown, "invalid certificate issued".to_owned()))
        }
    });
    Some((n.id, res))
}

/// Serves a network's CA while this side holds its lease, until stopped.
async fn ca_server<K: Kv, H: Host, T: Timer>(
    s: Rc<Shared<K, H, T>>,
    key: NetKey,
    ctl: Rc<CaServerCtl>,
) -> Done {
    let Some((network_id, net)) = s.state.borrow().network(&key) else {
        return Done::Service;
    };
    let server_key = s
        .records
        .borrow()
        .get(&key)
        .and_then(|n| n.server_config.as_ref()?.key.clone());
    let Some(server_key) = server_key else {
        return Done::Service;
    };
    let lease_key = lease::key_parts(&[Part::Str("ca"), Part::Id(network_id)]);
    let token = lease::new_token(&mut s.rng.borrow_mut());
    loop {
        if ctl.stop.get() {
            return Done::Service;
        }
        match lease::try_lock(s.store.store(), &lease_key, &token, s.timer.now()).await {
            Ok(()) => break,
            Err(strims_dao::Error::LeaseBusy) => {}
            Err(e) => tracing::warn!(error = %e, "taking the CA lease failed"),
        }
        let delay = lease::recheck_delay(&mut s.rng.borrow_mut());
        if wait_or_stop(&s, &ctl, s.timer.now() + delay).await {
            return Done::Service;
        }
    }
    if let Err(e) = serve(&s, &key, net, server_key, &ctl, &lease_key, &token).await {
        tracing::warn!(network = network_id, error = %e, "the CA server failed");
    }
    let _ = lease::release(s.store.store(), &lease_key, &token, s.timer.now()).await;
    Done::Service
}

/// Waits until `at`, or until the server is stopped: returns whether it
/// was.
async fn wait_or_stop<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    ctl: &CaServerCtl,
    at: Time,
) -> bool {
    let mut sleep = pin!(s.timer.sleep_until(at));
    poll_fn(|cx| {
        if ctl.stop.get() {
            return Poll::Ready(true);
        }
        *ctl.waker.borrow_mut() = Some(cx.waker().clone());
        sleep.as_mut().poll(cx).map(|()| false)
    })
    .await
}

async fn serve<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    key: &[u8],
    net: H::Network,
    server_key: strims_proto::types::Key,
    ctl: &CaServerCtl,
    lease_key: &str,
    token: &lease::Token,
) -> Result<(), String> {
    let cert = s
        .host_certificate(key)
        .ok_or("no host certificate on the network")?;
    let (port, reader, writer) = s.host.open_vpn_port(net).await.map_err(|e| e.to_string())?;
    let mut router = Router::new();
    register_ca_service(
        &mut router,
        Rc::new(CaServer {
            shared: Rc::downgrade(s),
            key: key.to_vec(),
        }),
    );
    let (client, conn) = vpn::connect(VpnConfig::default(), &cert, reader, writer, Some(router))
        .map_err(|e| e.to_string())?;
    *ctl.client.borrow_mut() = Some(client);
    let publication = s
        .host
        .publish_address(net, server_key, CA_SALT.to_vec(), port)
        .await
        .map_err(|e| e.to_string())?;
    let mut conn = pin!(conn);
    let mut refresh_at = s.timer.now() + lease::REFRESH;
    loop {
        let mut sleep = pin!(s.timer.sleep_until(refresh_at));
        let woke = poll_fn(|cx| {
            if conn.as_mut().poll(cx).is_ready() || ctl.stop.get() {
                return Poll::Ready(false);
            }
            *ctl.waker.borrow_mut() = Some(cx.waker().clone());
            sleep.as_mut().poll(cx).map(|()| true)
        })
        .await;
        if !woke {
            break;
        }
        let now = s.timer.now();
        if let Err(e) = lease::try_lock(s.store.store(), lease_key, token, now).await {
            tracing::warn!(error = %e, "the CA lease was lost");
            break;
        }
        refresh_at = now + lease::REFRESH;
    }
    *ctl.client.borrow_mut() = None;
    s.host
        .unpublish_address(publication)
        .await
        .map_err(|e| e.to_string())
}

/// `strims.network.v1.ca.CA` for one network, on its CA server's port.
struct CaServer<K: Kv, H: Host, T: Timer> {
    shared: Weak<Shared<K, H, T>>,
    key: NetKey,
}

type Served<K, H, T> = (Rc<Shared<K, H, T>>, Network);

impl<K: Kv, H: Host, T: Timer> CaServer<K, H, T> {
    fn network(&self) -> Result<Served<K, H, T>, RpcError> {
        let s = self
            .shared
            .upgrade()
            .ok_or_else(|| RpcError::new(0, "network control stopped"))?;
        let n = s
            .records
            .borrow()
            .get(&self.key)
            .filter(|n| n.server_config.is_some())
            .cloned()
            .ok_or_else(|| CaError::NotCa.to_rpc())?;
        Ok((s, n))
    }
}

impl<K: Kv, H: Host, T: Timer> CaService for CaServer<K, H, T> {
    async fn renew(&self, _: &Context, req: CaRenewRequest) -> Result<CaRenewResponse, RpcError> {
        let (s, n) = self.network()?;
        let cert = req
            .certificate
            .ok_or_else(|| CaError::Invalid("certificate missing").to_rpc())?;
        let csr = req
            .certificate_request
            .ok_or_else(|| CaError::Invalid("certificate request missing").to_rpc())?;
        let issued = s.renew_locally(&n, &cert, &csr).await.map_err(to_rpc)?;
        Ok(CaRenewResponse {
            certificate: Some(issued),
        })
    }

    async fn find(&self, _: &Context, req: CaFindRequest) -> Result<CaFindResponse, RpcError> {
        let (s, n) = self.network()?;
        let cert = ca::find(&s.store, n.id, req.query.as_ref(), req.full_chain)
            .await
            .map_err(|e| e.to_rpc())?;
        Ok(CaFindResponse {
            certificate: Some(cert),
        })
    }
}

/// `CAPeer.Renew`: forwards a peer's renewal to the network's CA.
impl<K: Kv, H: Host, T: Timer> CaPeerService for PeerService<K, H, T> {
    async fn renew(
        &self,
        _: &Context,
        req: CaPeerRenewRequest,
    ) -> Result<CaPeerRenewResponse, RpcError> {
        let s = self.shared()?;
        let (host, link) = (self.host, self.link);
        let cert = req
            .certificate
            .ok_or_else(|| CaError::Invalid("certificate missing").to_rpc())?;
        let csr = req
            .certificate_request
            .ok_or_else(|| CaError::Invalid("certificate request missing").to_rpc())?;
        let key = certificate_network_key(&cert).to_vec();
        let allowed = {
            let st = s.state.borrow();
            st.shares(host, link, &key) && st.profile_key(host, link) == Some(cert.key.as_slice())
        };
        if !allowed {
            return Err(CaError::Invalid("not a member of a shared network").to_rpc());
        }
        let _slot = s.forward_slot(host)?;
        let issued = s.renew_at_ca(&key, &cert, &csr).await.map_err(to_rpc)?;
        Ok(CaPeerRenewResponse {
            certificate: Some(issued),
        })
    }
}
