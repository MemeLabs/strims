// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The bootstrap control: the async shell around [`Schedule`]. It follows
//! the store's bootstrap clients, makes the schedule's dials, serves and
//! asks each peer's bootstrap service, and publishes networks to peers.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Poll, Waker};

use data_encoding::HEXLOWER;
use futures_util::future::{Either, LocalBoxFuture, select};
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, StreamExt};
use strims_base::rng::{DetRng, fork};
use strims_base::time::Time;
use strims_crypto::{DEFAULT_CERT_TTL, sign_certificate_request_with_network};
use strims_dao::events::Record;
use strims_dao::schema::network::{BootstrapClients, BootstrapConfig, Networks};
use strims_dao::{Recv, ReplicatedStore, Singleton, StoreEvent, Table};
use strims_kademlia::ID;
use strims_kv::Kv;
use strims_proto::network::v1::bootstrap::{
    BootstrapPeer, BootstrapPeerGetPublishEnabledRequest, BootstrapPeerPublishRequest,
    PeerServiceClient, bootstrap_client, register_peer_service_service,
};
use strims_proto::profile::v1::Profile;
use strims_proto::service::Error as RpcError;
use strims_proto::types::{CertificateRequestBody, KeyType, KeyUsage};
use strims_rpc::{Client, RetryClient, Router, Timer};

use super::schedule::{Dial, Schedule};
use super::service::PeerService;
use super::{Config, Failure, Host};

fn rpc_error(e: impl std::fmt::Display) -> RpcError {
    RpcError::new(0, e.to_string())
}

/// A linked peer.
struct Peer<T> {
    /// The id the UI names it by: one per link.
    id: u64,
    profile_key: Vec<u8>,
    client: PeerServiceClient<RetryClient<T>>,
    /// Whether it accepts published networks, once it answered.
    publishing: Option<bool>,
}

pub(crate) struct Shared<K: Kv, H: Host, T: Timer> {
    pub(crate) host: H,
    pub(crate) store: ReplicatedStore<K>,
    pub(crate) profile: Profile,
    pub(crate) timer: T,
    pub(crate) config: Config,
    /// Draws published certificates' serial numbers.
    rng: RefCell<DetRng>,
    schedule: RefCell<Schedule>,
    /// Bound: the node's peers.
    peers: RefCell<BTreeMap<ID, Peer<T>>>,
    next_peer_id: Cell<u64>,
    /// `GetPublishEnabled` calls for `run` to make, by host and peer id.
    /// Bound: the node's peers.
    queries: RefCell<VecDeque<(ID, u64)>>,
    /// The peers or the schedule changed outside `run`.
    dirty: Cell<bool>,
    /// Wakes `run`.
    waker: RefCell<Option<Waker>>,
}

impl<K: Kv, H: Host, T: Timer> Shared<K, H, T> {
    fn wake(&self) {
        self.dirty.set(true);
        if let Some(w) = self.waker.borrow_mut().take() {
            w.wake();
        }
    }

    /// Whether this device accepts published networks.
    pub(crate) async fn publishing_enabled(&self) -> Result<bool, strims_dao::Error> {
        let mut tx = self.store.read().await?;
        Ok(BootstrapConfig::get(&mut tx)
            .await?
            .unwrap_or_default()
            .enable_publishing)
    }

    /// Reads the store's clients into the schedule.
    async fn reload(&self) {
        let clients = async {
            let mut tx = self.store.read().await?;
            BootstrapClients::get_all(&mut tx).await
        }
        .await;
        let clients = match clients {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(error = %e, "reading bootstrap clients failed");
                return;
            }
        };
        let clients: Vec<(u64, String)> = clients
            .into_iter()
            .filter_map(|c| match c.client_options {
                Some(bootstrap_client::ClientOptions::WebsocketOptions(o)) => {
                    let mut url = o.url;
                    if o.insecure_skip_verify_tls {
                        url.push_str("#insecure");
                    }
                    Some((c.id, url))
                }
                None => None,
            })
            .collect();
        let own = self.host.own_uris();
        let skipped = self.schedule.borrow_mut().set_clients(clients, &own);
        if skipped > 0 {
            tracing::warn!(skipped, "bootstrap clients beyond the limit aren't dialed");
        }
    }
}

/// What a task of `run` finished.
enum Done {
    Dial {
        dial: Dial,
        result: Result<ID, Failure>,
    },
    Query {
        host: ID,
        id: u64,
        enabled: bool,
    },
}

enum Wake {
    Task(Done),
    Store(Option<Recv<StoreEvent>>),
    Timer,
    Work,
}

/// Bootstrap for a profile's node: dials the store's bootstrap clients,
/// serves every peer the bootstrap peer service, and publishes networks
/// to peers. Clones share the control.
pub struct BootstrapControl<K: Kv, H: Host, T: Timer> {
    pub(crate) shared: Rc<Shared<K, H, T>>,
}

impl<K: Kv, H: Host, T: Timer> Clone for BootstrapControl<K, H, T> {
    fn clone(&self) -> Self {
        BootstrapControl {
            shared: self.shared.clone(),
        }
    }
}

impl<K: Kv, H: Host, T: Timer> BootstrapControl<K, H, T> {
    /// A control for the node behind `host`, dialing the bootstrap clients
    /// of `profile` that `store` holds, timed on `timer`.
    pub fn new(
        host: H,
        store: ReplicatedStore<K>,
        profile: Profile,
        timer: T,
        rng: DetRng,
        config: Config,
    ) -> Self {
        let mut rng = rng;
        let schedule = Schedule::new(&config, fork(&mut rng));
        BootstrapControl {
            shared: Rc::new(Shared {
                host,
                store,
                profile,
                timer,
                config,
                rng: RefCell::new(rng),
                schedule: RefCell::new(schedule),
                peers: RefCell::new(BTreeMap::new()),
                next_peer_id: Cell::new(0),
                queries: RefCell::new(VecDeque::new()),
                dirty: Cell::new(false),
                waker: RefCell::new(None),
            }),
        }
    }

    // --- peers --------------------------------------------------------------

    /// Takes a newly linked peer, replacing an earlier link to it: serves
    /// it the bootstrap peer service on `router`, and asks it through
    /// `client` whether it accepts published networks. `profile_key` is
    /// the key of the profile certificate it linked with.
    pub fn add_peer(&self, host: ID, profile_key: &[u8], router: &Router, client: Client) {
        let s = &self.shared;
        let id = s.next_peer_id.get() + 1;
        s.next_peer_id.set(id);
        let client = RetryClient::new(client, s.timer.clone(), s.config.call_policy);
        s.peers.borrow_mut().insert(
            host,
            Peer {
                id,
                profile_key: profile_key.to_vec(),
                client: PeerServiceClient::new(client),
                publishing: None,
            },
        );
        let mut router = router.clone();
        register_peer_service_service(&mut router, Rc::new(PeerService::new(Rc::downgrade(s))));
        s.queries.borrow_mut().push_back((host, id));
        s.schedule.borrow_mut().peer_added(s.timer.now(), host);
        s.wake();
    }

    /// Forgets a peer that went away.
    pub fn remove_peer(&self, host: ID) {
        let s = &self.shared;
        s.peers.borrow_mut().remove(&host);
        s.schedule.borrow_mut().peer_removed(s.timer.now(), host);
        s.wake();
    }

    /// The linked peers that accept published networks, by id, labelled
    /// with their profile key.
    pub fn list_peers(&self) -> Vec<BootstrapPeer> {
        let mut peers: Vec<BootstrapPeer> = self
            .shared
            .peers
            .borrow()
            .values()
            .filter(|p| p.publishing == Some(true))
            .map(|p| BootstrapPeer {
                peer_id: p.id,
                label: HEXLOWER.encode(&p.profile_key),
            })
            .collect();
        peers.sort_by_key(|p| p.peer_id);
        peers
    }

    /// Publishes the network `network_id`, whose CA this device runs, to
    /// the peer `peer_id`: a membership for the peer's profile key, signed
    /// by the network's key, which the peer adds to its store.
    ///
    /// # Errors
    ///
    /// A peer that isn't linked or doesn't accept published networks, a
    /// network this device doesn't run the CA of, or the peer's refusal.
    pub async fn publish(&self, peer_id: u64, network_id: u64) -> Result<(), RpcError> {
        let s = &self.shared;
        let (client, profile_key) = {
            let peers = s.peers.borrow();
            let p = peers
                .values()
                .find(|p| p.id == peer_id)
                .ok_or_else(|| rpc_error("peer id not found"))?;
            if p.publishing != Some(true) {
                return Err(rpc_error("peer does not support network bootstrapping"));
            }
            (p.client.clone(), p.profile_key.clone())
        };
        let network = {
            let mut tx = s.store.read().await.map_err(rpc_error)?;
            Networks::get(&mut tx, network_id)
                .await
                .map_err(rpc_error)?
                .ok_or_else(|| rpc_error("network not found"))?
        };
        let config = network
            .server_config
            .as_ref()
            .ok_or_else(|| rpc_error("only managed networks can be published"))?;
        let key = config
            .key
            .as_ref()
            .ok_or_else(|| rpc_error("network has no key"))?;
        let csr = CertificateRequestBody {
            key: profile_key,
            key_type: KeyType::Ed25519 as i32,
            key_usage: KeyUsage::Broker as i32 | KeyUsage::Sign as i32,
            subject: String::new(),
        };
        let mut rng = fork(&mut s.rng.borrow_mut());
        let certificate = sign_certificate_request_with_network(
            &csr,
            key,
            &config.name,
            DEFAULT_CERT_TTL,
            s.timer.now(),
            &mut rng,
        )
        .map_err(rpc_error)?;
        client
            .publish(&BootstrapPeerPublishRequest {
                certificate: Some(certificate),
            })
            .await
            .map_err(|e| rpc_error(format!("publishing failed: {e}")))?;
        Ok(())
    }

    // --- running ------------------------------------------------------------

    /// Dials the store's bootstrap clients as they come and as their links
    /// go, and asks new peers whether they accept published networks.
    /// Peers' requests are answered whether or not it runs. Never
    /// resolves; drop it to stop.
    pub async fn run(&self) {
        let s = &self.shared;
        let mut store_events = Some(s.store.events().subscribe());
        s.reload().await;
        let mut tasks: FuturesUnordered<LocalBoxFuture<'static, Done>> = FuturesUnordered::new();
        let mut sleep: Option<(Time, Pin<Box<T::Sleep>>)> = None;
        loop {
            s.dirty.set(false);
            loop {
                let next = s.schedule.borrow_mut().poll_dial(s.timer.now());
                let Some(d) = next else {
                    break;
                };
                tasks.push(dial(s.clone(), d).boxed_local());
            }
            let queries: Vec<(ID, u64)> = s.queries.borrow_mut().drain(..).collect();
            for (host, id) in queries {
                if let Some(t) = query(s, host, id) {
                    tasks.push(t);
                }
            }
            let timeout = s.schedule.borrow().poll_timeout();
            match timeout {
                Some(at) if sleep.as_ref().is_none_or(|(d, _)| *d != at) => {
                    sleep = Some((at, Box::pin(s.timer.sleep_until(at))));
                }
                Some(_) => {}
                None => sleep = None,
            }
            let wake = poll_fn(|cx| {
                *s.waker.borrow_mut() = Some(cx.waker().clone());
                if let Poll::Ready(Some(done)) = tasks.poll_next_unpin(cx) {
                    return Poll::Ready(Wake::Task(done));
                }
                if let Some(events) = &mut store_events
                    && let Poll::Ready(r) = events.poll_next_unpin(cx)
                {
                    return Poll::Ready(Wake::Store(r));
                }
                if let Some((_, sl)) = &mut sleep
                    && sl.as_mut().poll(cx).is_ready()
                {
                    sleep = None;
                    return Poll::Ready(Wake::Timer);
                }
                if s.dirty.get() {
                    return Poll::Ready(Wake::Work);
                }
                Poll::Pending
            })
            .await;
            match wake {
                Wake::Task(Done::Dial { dial, result }) => {
                    match &result {
                        Ok(host) => {
                            tracing::info!(url = %dial.url, %host, "linked to a bootstrap client");
                        }
                        Err(e) => {
                            tracing::debug!(url = %dial.url, error = ?e, "dialing a bootstrap client failed");
                        }
                    }
                    s.schedule.borrow_mut().dial_done(
                        s.timer.now(),
                        dial.client,
                        dial.generation,
                        result,
                    );
                }
                Wake::Task(Done::Query { host, id, enabled }) => {
                    if let Some(p) = s.peers.borrow_mut().get_mut(&host)
                        && p.id == id
                    {
                        p.publishing = Some(enabled);
                    }
                }
                Wake::Store(Some(Recv::Event(e))) => {
                    if let StoreEvent::Changed(Record::BootstrapClient(_))
                    | StoreEvent::Deleted(Record::BootstrapClient(_)) = &*e
                    {
                        s.reload().await;
                    }
                }
                Wake::Store(Some(Recv::Lagged)) => s.reload().await,
                Wake::Store(None) => store_events = None,
                Wake::Timer => s.schedule.borrow_mut().handle_timeout(s.timer.now()),
                Wake::Work => {}
            }
        }
    }
}

/// Dials a client, within `dial_timeout`.
async fn dial<K: Kv, H: Host, T: Timer>(s: Rc<Shared<K, H, T>>, dial: Dial) -> Done {
    let deadline = s.timer.now() + s.config.dial_timeout;
    let call = Box::pin(s.host.dial(&dial.url));
    let timeout = Box::pin(s.timer.sleep_until(deadline));
    let result = match select(call, timeout).await {
        Either::Left((r, _)) => r,
        Either::Right(_) => Err(Failure::Retry("dial timed out".to_owned())),
    };
    Done::Dial { dial, result }
}

/// Asks the peer `host`'s link `id` whether it accepts published networks.
/// A failed call counts as no.
fn query<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    host: ID,
    id: u64,
) -> Option<LocalBoxFuture<'static, Done>> {
    let call = {
        let peers = s.peers.borrow();
        let p = peers.get(&host).filter(|p| p.id == id)?;
        p.client
            .get_publish_enabled(&BootstrapPeerGetPublishEnabledRequest {})
    };
    Some(
        async move {
            let enabled = call.await.is_ok_and(|r| r.enabled);
            Done::Query { host, id, enabled }
        }
        .boxed_local(),
    )
}
