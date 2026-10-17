// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A network's directory services on this node: the mode task, which
//! serves the directory or reads it, the `Directory` service of the
//! server, and the member port with its `DirectorySnippet` service.
//!
//! # Modes
//!
//! A network's mode task runs one mode at a time, so that one mode's
//! swarms and transfers are gone before the other's start:
//!
//! - **Reader**: adds both swarms by the network key and publishes their
//!   transfers. One task reads the event stream into the view; another
//!   reads the asset stream into the asset bundle. While the network's
//!   record has a server config, it tries the lease at the record's
//!   change and then after every [`lease::recheck_delay`].
//! - **Server**: creates both swarms, signed with the root key, publishes
//!   their transfers, serves `Directory` on a vpn port and publishes the
//!   port's address. It writes the asset bundle whenever it changes and
//!   every broadcast, which it also feeds its own view, and refreshes the
//!   lease every [`lease::REFRESH`]. It ends when the lease or the server
//!   config is lost, or its connection ends; the node then reads, and
//!   tries the lease again at the next check.
//!
//! Every mode switch, and every discontinuity of the event stream, starts
//! the view over.
//!
//! # Snippets
//!
//! The member port serves the snippets [`SnippetHub`] holds. A
//! subscription is admitted while the swarm's transfer under the empty
//! salt is published on the network, the hub holds its snippet, and the
//! subscribers number fewer than [`MAX_SNIPPET_SUBSCRIBERS`] in all and
//! [`MAX_SUBSCRIBERS_PER_SWARM`] for the swarm. Its stream sends the
//! snippet's deltas from the last snippet it sent, starting from the empty
//! snippet, and ends when the snippet is deleted.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context as TaskContext, Poll, Waker};

use futures_core::Stream;
use futures_util::future::{AbortHandle, Aborted, abortable};
use futures_util::{FutureExt, StreamExt};
use strims_base::rng::fork;
use strims_base::time::{Duration, Time};
use strims_dao::Table;
use strims_dao::lease::{self, Token};
use strims_dao::schema::directory::{DirectoryListingRecords, DirectoryUserRecords, listing_key};
use strims_dao::schema::network::network_scoped_key;
use strims_kademlia::ID;
use strims_kv::Kv;
use strims_ppspp::{SwarmId, WriterOptions};
use strims_proto::network::v1::directory::{
    AssetBundle, DirectoryClient, DirectoryService, DirectorySnippetClient,
    DirectorySnippetService, JoinRequest, JoinResponse, Listing, ListingModeration, ListingQuery,
    ListingRecord, ListingSnippet, ListingSnippetDelta, ModerateListingRequest,
    ModerateListingResponse, ModerateUserRequest, ModerateUserResponse, PartRequest, PartResponse,
    PingRequest, PingResponse, PublishRequest, PublishResponse, SnippetSubscribeRequest,
    SnippetSubscribeResponse, UnpublishRequest, UnpublishResponse, UserModeration, UserRecord,
    register_directory_service, register_directory_snippet_service,
};
use strims_proto::service::{Context, Error as RpcError, ServerStream};
use strims_rpc::vpn::{self, VpnClient, VpnConnection};
use strims_rpc::{RetryClient, Router, Timer};
use strims_transfer::{ReadFailure, RecordReader, RecordWriter, TransferId};
use strims_vpn::HostAddr;

use crate::control::{CallFailure, MEMBER_RETRY, Runner, Shared};
use crate::host::Host;
use crate::moderation::{is_admin, is_moderator, merge_listing, merge_user};
use crate::server::{Caller, Server, ServerAction, SubId};
use crate::snippet::{
    LocalSnippets, MAX_SNIPPET_SUBSCRIBERS, MAX_SUBSCRIBERS_PER_SWARM, diff, is_empty, split,
};
use crate::{
    ADDRESS_SALT, ASSET_FRAME, ASSET_SALT, DirectoryError, EVENT_FRAME, EVENT_SALT,
    MAX_PEER_KEY_BYTES, SNIPPET_PORT, SnippetError, asset_options, event_options, lease_key,
    listing,
};

/// The wait after a failed swarm read before reading again.
const READ_RETRY: Duration = Duration::SECOND;

type Swarm<H> = <H as strims_transfer::Host>::Swarm;
type Publication<H> = <H as strims_network::Host>::Publication;
type VpnReader<H> = <H as strims_network::Host>::VpnReader;
type VpnWriter<H> = <H as strims_network::Host>::VpnWriter;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// --- calls ------------------------------------------------------------------

/// A call to a directory server.
pub(crate) enum Request {
    Publish(Listing),
    Unpublish(u64),
    Join(ListingQuery),
    Part(u64),
    Ping,
    ModerateListing(u64, Option<ListingModeration>),
    ModerateUser(Vec<u8>, Option<UserModeration>),
}

/// A call's answer.
pub(crate) enum Reply {
    /// The listing's id.
    Id(u64),
    Done,
}

impl Reply {
    pub(crate) fn id(self) -> u64 {
        match self {
            Reply::Id(id) => id,
            Reply::Done => 0,
        }
    }
}

/// Makes `req` through `client`.
pub(crate) async fn call_remote<T: Timer>(
    client: &DirectoryClient<RetryClient<T>>,
    req: Request,
) -> Result<Reply, CallFailure> {
    let f = CallFailure::from_call;
    Ok(match req {
        Request::Publish(listing) => {
            let req = PublishRequest {
                listing: Some(listing),
            };
            Reply::Id(client.publish(&req).await.map_err(f)?.id)
        }
        Request::Unpublish(id) => {
            client
                .unpublish(&UnpublishRequest { id })
                .await
                .map_err(f)?;
            Reply::Done
        }
        Request::Join(query) => {
            let req = JoinRequest { query: Some(query) };
            Reply::Id(client.join(&req).await.map_err(f)?.id)
        }
        Request::Part(id) => {
            client.part(&PartRequest { id }).await.map_err(f)?;
            Reply::Done
        }
        Request::Ping => {
            client.ping(&PingRequest {}).await.map_err(f)?;
            Reply::Done
        }
        Request::ModerateListing(id, moderation) => {
            let req = ModerateListingRequest { id, moderation };
            client.moderate_listing(&req).await.map_err(f)?;
            Reply::Done
        }
        Request::ModerateUser(peer_key, moderation) => {
            let req = ModerateUserRequest {
                peer_key,
                moderation,
            };
            client.moderate_user(&req).await.map_err(f)?;
            Reply::Done
        }
    })
}

/// Serves `req` from `caller` with the server `r` runs, reading and
/// writing the moderation records it needs.
pub(crate) async fn serve<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    r: &Runner<H>,
    caller: &Caller,
    req: Request,
) -> Result<Reply, CallFailure> {
    let res = serve_request(s, r, caller, req).await;
    if let Err(CallFailure::Store(e)) = &res {
        r.metrics.borrow_mut().store_errors += 1;
        tracing::warn!(network = r.network_id, error = %e, "reading directory records failed");
    }
    res
}

async fn serve_request<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    r: &Runner<H>,
    c: &Caller,
    req: Request,
) -> Result<Reply, CallFailure> {
    let network_id = r.network_id;
    let server = |res: Option<Result<u64, DirectoryError>>| {
        res.ok_or(CallFailure::NoServer)?
            .map_err(CallFailure::Refused)
    };
    let done = |res: Option<Result<(), DirectoryError>>| {
        res.ok_or(CallFailure::NoServer)?
            .map(|()| Reply::Done)
            .map_err(CallFailure::Refused)
    };
    match req {
        Request::Publish(l) => {
            listing::validate(&l).map_err(CallFailure::Refused)?;
            let user = moderation_of_user(user_record(s, network_id, &c.peer_key).await?);
            let record = listing_record(s, network_id, &l)
                .await?
                .and_then(|r| r.moderation);
            let res = s.with_server(r, |srv, now| {
                srv.publish(now, c, l, user.as_ref(), record.as_ref())
            });
            server(res).map(Reply::Id)
        }
        Request::Unpublish(id) => done(s.with_server(r, |srv, now| srv.unpublish(now, c, id))),
        Request::Join(q) => {
            let user = moderation_of_user(user_record(s, network_id, &c.peer_key).await?);
            let res = s.with_server(r, |srv, now| srv.join(now, c, &q, user.as_ref()));
            server(res).map(Reply::Id)
        }
        Request::Part(id) => done(s.with_server(r, |srv, now| srv.part(now, c, id))),
        Request::Ping => done(s.with_server(r, |srv, now| srv.ping(now, c))),
        Request::ModerateListing(id, m) => moderate_listing(s, r, c, id, m).await,
        Request::ModerateUser(peer_key, m) => moderate_user(s, r, c, &peer_key, m).await,
    }
}

fn moderation_of_user(r: Option<UserRecord>) -> Option<UserModeration> {
    r.and_then(|r| r.moderation)
}

async fn user_record<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    network_id: u64,
    peer_key: &[u8],
) -> Result<Option<UserRecord>, CallFailure> {
    let key = network_scoped_key(network_id, peer_key);
    async {
        let mut tx = s.store.read().await?;
        DirectoryUserRecords::index_get(&mut tx, &DirectoryUserRecords::BY_PEER_KEY, &key).await
    }
    .await
    .map_err(CallFailure::Store)
}

async fn listing_record<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    network_id: u64,
    l: &Listing,
) -> Result<Option<ListingRecord>, CallFailure> {
    let key = listing_key(network_id, Some(l));
    async {
        let mut tx = s.store.read().await?;
        DirectoryListingRecords::index_get(&mut tx, &DirectoryListingRecords::BY_LISTING, &key)
            .await
    }
    .await
    .map_err(CallFailure::Store)
}

/// `ModerateListing`: a moderator's change to listing `id`'s record, which
/// the server then takes.
async fn moderate_listing<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    r: &Runner<H>,
    c: &Caller,
    id: u64,
    m: Option<ListingModeration>,
) -> Result<Reply, CallFailure> {
    let network_id = r.network_id;
    let caller = user_record(s, network_id, &c.peer_key).await?;
    if !is_moderator(caller.as_ref()) {
        return Err(CallFailure::Refused(DirectoryError::PermissionDenied));
    }
    if let Some(m) = &m {
        listing::validate_moderation(m).map_err(CallFailure::Refused)?;
    }
    let l = s
        .with_server(r, |srv, _| srv.listing(id).cloned())
        .ok_or(CallFailure::NoServer)?
        .ok_or(CallFailure::Refused(DirectoryError::ListingNotFound))?;
    let key = listing_key(network_id, Some(&l));
    let rec = async {
        let mut tx = s.store.write(s.timer.now()).await?;
        let ix = &DirectoryListingRecords::BY_LISTING;
        let existing = DirectoryListingRecords::index_get(&mut tx, ix, &key).await?;
        let new_id = match existing {
            Some(_) => 0,
            None => tx.generate_id().await?,
        };
        let rec = merge_listing(existing, m.as_ref(), new_id, network_id, &l);
        let rec = DirectoryListingRecords::upsert(&mut tx, rec).await?;
        tx.commit().await?;
        Ok(rec)
    }
    .await
    .map_err(CallFailure::Store)?;
    s.with_server(r, |srv, now| srv.listing_moderated(now, id, rec.moderation))
        .ok_or(CallFailure::NoServer)?;
    Ok(Reply::Done)
}

/// `ModerateUser`: a moderator's change to the record of the user with
/// `peer_key`, which the server then takes.
async fn moderate_user<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    r: &Runner<H>,
    c: &Caller,
    peer_key: &[u8],
    m: Option<UserModeration>,
) -> Result<Reply, CallFailure> {
    if peer_key.is_empty() || peer_key.len() > MAX_PEER_KEY_BYTES {
        return Err(CallFailure::Refused(DirectoryError::TooLarge));
    }
    let network_id = r.network_id;
    let caller = user_record(s, network_id, &c.peer_key).await?;
    if !is_moderator(caller.as_ref()) {
        return Err(CallFailure::Refused(DirectoryError::PermissionDenied));
    }
    let admin = is_admin(caller.as_ref());
    let key = network_scoped_key(network_id, peer_key);
    let rec = async {
        let mut tx = s.store.write(s.timer.now()).await?;
        let ix = &DirectoryUserRecords::BY_PEER_KEY;
        let existing = DirectoryUserRecords::index_get(&mut tx, ix, &key).await?;
        let new_id = match existing {
            Some(_) => 0,
            None => tx.generate_id().await?,
        };
        let rec = merge_user(existing, m.as_ref(), admin, new_id, network_id, peer_key);
        let rec = DirectoryUserRecords::upsert(&mut tx, rec).await?;
        tx.commit().await?;
        Ok(rec)
    }
    .await
    .map_err(CallFailure::Store)?;
    let m = rec.moderation.unwrap_or_default();
    s.with_server(r, |srv, now| srv.user_moderated(now, peer_key, &m))
        .ok_or(CallFailure::NoServer)?;
    Ok(Reply::Done)
}

/// `strims.network.v1.directory.Directory` for one network, on its
/// server's port. The caller is the vpn certificate's host.
struct DirectoryServer<K: Kv, H: Host, T: Timer> {
    shared: Weak<Shared<K, H, T>>,
    runner: Weak<Runner<H>>,
}

impl<K: Kv, H: Host, T: Timer> DirectoryServer<K, H, T> {
    async fn serve(&self, ctx: &Context, req: Request) -> Result<Reply, RpcError> {
        let (Some(s), Some(r)) = (self.shared.upgrade(), self.runner.upgrade()) else {
            return Err(CallFailure::NoServer.to_rpc());
        };
        let caller = ctx
            .certificate()
            .and_then(Caller::from_certificate)
            .ok_or_else(|| RpcError::new(0, "no caller certificate"))?;
        serve(&s, &r, &caller, req).await.map_err(|e| e.to_rpc())
    }
}

impl<K: Kv, H: Host, T: Timer> DirectoryService for DirectoryServer<K, H, T> {
    async fn publish(
        &self,
        ctx: &Context,
        req: PublishRequest,
    ) -> Result<PublishResponse, RpcError> {
        let listing = req.listing.unwrap_or_default();
        let id = self.serve(ctx, Request::Publish(listing)).await?.id();
        Ok(PublishResponse { id })
    }

    async fn unpublish(
        &self,
        ctx: &Context,
        req: UnpublishRequest,
    ) -> Result<UnpublishResponse, RpcError> {
        self.serve(ctx, Request::Unpublish(req.id)).await?;
        Ok(UnpublishResponse {})
    }

    async fn join(&self, ctx: &Context, req: JoinRequest) -> Result<JoinResponse, RpcError> {
        let query = req.query.unwrap_or_default();
        let id = self.serve(ctx, Request::Join(query)).await?.id();
        Ok(JoinResponse { id })
    }

    async fn part(&self, ctx: &Context, req: PartRequest) -> Result<PartResponse, RpcError> {
        self.serve(ctx, Request::Part(req.id)).await?;
        Ok(PartResponse {})
    }

    async fn ping(&self, ctx: &Context, _: PingRequest) -> Result<PingResponse, RpcError> {
        self.serve(ctx, Request::Ping).await?;
        Ok(PingResponse {})
    }

    async fn moderate_listing(
        &self,
        ctx: &Context,
        req: ModerateListingRequest,
    ) -> Result<ModerateListingResponse, RpcError> {
        let req = Request::ModerateListing(req.id, req.moderation);
        self.serve(ctx, req).await?;
        Ok(ModerateListingResponse {})
    }

    async fn moderate_user(
        &self,
        ctx: &Context,
        req: ModerateUserRequest,
    ) -> Result<ModerateUserResponse, RpcError> {
        let req = Request::ModerateUser(req.peer_key, req.moderation);
        self.serve(ctx, req).await?;
        Ok(ModerateUserResponse {})
    }
}

// --- modes ------------------------------------------------------------------

/// The server of a network this node serves.
pub(crate) struct ServerState {
    pub(crate) server: Server,
    /// The server's connection, which also makes its snippet
    /// subscriptions.
    pub(crate) client: VpnClient,
    /// The server's actions the server task hasn't carried out. Bound:
    /// those of the calls and timeouts since the task last ran.
    pub(crate) actions: VecDeque<ServerAction>,
    /// The snippet subscriptions in flight. Bound: the server's, at most
    /// one per listing.
    pub(crate) subs: BTreeMap<SubId, AbortHandle>,
    /// The connection ended.
    pub(crate) closed: bool,
}

/// A network's member port.
pub(crate) enum MemberPort {
    Closed,
    Opening(AbortHandle),
    Open {
        client: VpnClient,
        abort: AbortHandle,
    },
}

impl MemberPort {
    pub(crate) fn abort(&self) {
        match self {
            MemberPort::Closed => {}
            MemberPort::Opening(abort) | MemberPort::Open { abort, .. } => abort.abort(),
        }
    }
}

/// Swarms a mode added, and their transfers.
struct Swarms<H: Host> {
    swarms: Vec<Swarm<H>>,
    transfers: Vec<TransferId>,
}

impl<H: Host> Default for Swarms<H> {
    fn default() -> Self {
        Swarms {
            swarms: Vec::new(),
            transfers: Vec::new(),
        }
    }
}

impl<H: Host> Swarms<H> {
    /// Adds and publishes the transfer of `swarm` under `salt`.
    fn publish<K: Kv, T: Timer>(
        &mut self,
        s: &Shared<K, H, T>,
        r: &Runner<H>,
        swarm: Swarm<H>,
        id: &SwarmId,
        salt: &[u8],
    ) {
        let t = s.transfers.add(swarm, id, salt.to_vec());
        s.transfers.publish(t, r.net);
        self.transfers.push(t);
    }

    /// Removes the transfers, then the swarms.
    async fn remove<K: Kv, T: Timer>(self, s: &Shared<K, H, T>, r: &Runner<H>) {
        for t in self.transfers {
            s.transfers.remove(t);
        }
        for swarm in self.swarms {
            if let Err(e) = s.host.remove_swarm(swarm).await {
                r.metrics.borrow_mut().host_errors += 1;
                tracing::warn!(error = %e, "removing a directory swarm failed");
            }
        }
    }
}

/// What woke a mode task.
enum Woke {
    Stop,
    /// The record's server config changed.
    Changed,
    /// The server's connection ended.
    Closed,
    /// The server has actions.
    Work,
    Timer,
}

/// Waits for `r`'s mode task's next thing to do, or until `at`.
async fn wait<H: Host, T: Timer>(timer: &T, r: &Runner<H>, at: Option<Time>) -> Woke {
    let mut sleep = at.map(|at| Box::pin(timer.sleep_until(at)));
    poll_fn(|cx| {
        if r.stopped.get() {
            return Poll::Ready(Woke::Stop);
        }
        if let Some(st) = &*r.server.borrow() {
            if st.closed {
                return Poll::Ready(Woke::Closed);
            }
            if !st.actions.is_empty() {
                return Poll::Ready(Woke::Work);
            }
        }
        if r.changed.replace(false) {
            return Poll::Ready(Woke::Changed);
        }
        let prev = r.waker.replace(Some(cx.waker().clone()));
        drop(prev);
        if let Some(sl) = &mut sleep
            && sl.as_mut().poll(cx).is_ready()
        {
            return Poll::Ready(Woke::Timer);
        }
        Poll::Pending
    })
    .await
}

async fn wait_done<H: Host>(p: &Runner<H>) {
    poll_fn(|cx| {
        if p.done.get() {
            return Poll::Ready(());
        }
        let prev = p.done_waker.replace(Some(cx.waker().clone()));
        drop(prev);
        Poll::Pending
    })
    .await;
}

async fn try_lease<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    r: &Runner<H>,
    key: &str,
    token: &Token,
) -> bool {
    match lease::try_lock(s.store.store(), key, token, s.timer.now()).await {
        Ok(()) => true,
        Err(strims_dao::Error::LeaseBusy) => false,
        Err(e) => {
            r.metrics.borrow_mut().store_errors += 1;
            tracing::warn!(error = %e, "taking the directory lease failed");
            false
        }
    }
}

/// Runs network `r`'s directory on this node until `r` stops; see the
/// [module docs](self).
pub(crate) async fn run_mode<K: Kv, H: Host, T: Timer>(s: Rc<Shared<K, H, T>>, r: Rc<Runner<H>>) {
    let previous = r.previous.borrow_mut().take();
    if let Some(p) = previous {
        wait_done(&p).await;
    }
    let lease_key = lease_key(r.network_id);
    let token = lease::new_token(&mut s.rng.borrow_mut());
    let mut leased =
        !r.stopped.get() && r.wants_server() && try_lease(&s, &r, &lease_key, &token).await;
    while !r.stopped.get() {
        if !leased {
            leased = read(&s, &r, &lease_key, &token).await;
            s.reset_view(&r);
            if !leased {
                break;
            }
        }
        r.metrics.borrow_mut().serves += 1;
        if let Err(e) = serve_directory(&s, &r, &lease_key, &token).await {
            tracing::warn!(network = r.network_id, error = %e, "the directory server stopped");
        }
        if let Err(e) = lease::release(s.store.store(), &lease_key, &token, s.timer.now()).await {
            tracing::warn!(error = %e, "releasing the directory lease failed");
        }
        s.reset_view(&r);
        leased = false;
    }
    s.finished(&r);
}

/// What the reader mode runs.
struct Reading<H: Host> {
    swarms: Swarms<H>,
    readers: Vec<AbortHandle>,
}

/// Reads the server's swarms until `r` stops (`false`), or until this node
/// takes the lease (`true`).
async fn read<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    r: &Rc<Runner<H>>,
    lease_key: &str,
    token: &Token,
) -> bool {
    let recheck = || s.timer.now() + lease::recheck_delay(&mut s.rng.borrow_mut());
    let mut reading = start_reading(s, r).await;
    let mut next_check = recheck();
    let leased = loop {
        let at = (r.wants_server() || reading.is_none()).then_some(next_check);
        match wait(&s.timer, r, at).await {
            Woke::Stop => break false,
            Woke::Changed => {
                if r.wants_server() && try_lease(s, r, lease_key, token).await {
                    break true;
                }
            }
            Woke::Timer => {
                next_check = recheck();
                if reading.is_none() {
                    reading = start_reading(s, r).await;
                }
                if r.wants_server() && try_lease(s, r, lease_key, token).await {
                    break true;
                }
            }
            Woke::Closed | Woke::Work => {}
        }
    };
    if let Some(reading) = reading {
        for a in reading.readers {
            a.abort();
        }
        reading.swarms.remove(s, r).await;
    }
    leased
}

async fn start_reading<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    r: &Rc<Runner<H>>,
) -> Option<Reading<H>> {
    let mut swarms = Swarms::default();
    match add_readers(s, r, &mut swarms).await {
        Ok(readers) => Some(Reading { swarms, readers }),
        Err(e) => {
            r.metrics.borrow_mut().host_errors += 1;
            tracing::warn!(network = r.network_id, error = %e, "reading the directory failed");
            swarms.remove(s, r).await;
            None
        }
    }
}

/// Adds both swarms, publishes their transfers and starts their readers.
async fn add_readers<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    r: &Rc<Runner<H>>,
    swarms: &mut Swarms<H>,
) -> Result<Vec<AbortHandle>, String> {
    let id = SwarmId::new(&r.key);
    let events = s
        .host
        .add_swarm(id.clone(), event_options(&r.key))
        .await
        .map_err(err)?;
    swarms.swarms.push(events);
    let assets = s
        .host
        .add_swarm(id.clone(), asset_options(&r.key))
        .await
        .map_err(err)?;
    swarms.swarms.push(assets);
    swarms.publish(s, r, events, &id, EVENT_SALT);
    swarms.publish(s, r, assets, &id, ASSET_SALT);
    let events = s.host.open_reader(events).await.map_err(err)?;
    let events = RecordReader::new(events, EVENT_FRAME, s.config.max_event_record).map_err(err)?;
    let assets = s.host.open_reader(assets).await.map_err(err)?;
    let assets = RecordReader::new(assets, ASSET_FRAME, s.config.max_asset_record).map_err(err)?;
    let (events, a) = abortable(read_events(s.clone(), r.clone(), events));
    let (assets, b) = abortable(read_assets(s.clone(), r.clone(), assets));
    s.spawn(events.map(|_| ()).boxed_local());
    s.spawn(assets.map(|_| ()).boxed_local());
    Ok(vec![a, b])
}

/// Reads the event stream into `r`'s view. A reader is dropped with its
/// task, never between a read and its answer.
async fn read_events<K: Kv, H: Host, T: Timer>(
    s: Rc<Shared<K, H, T>>,
    r: Rc<Runner<H>>,
    mut reader: RecordReader<H::Reader>,
) {
    loop {
        match reader.next().await {
            Ok(record) => {
                r.view.borrow_mut().handle_record(record);
                s.drain_view(&r);
            }
            Err(ReadFailure::Discontinuity) => {
                r.metrics.borrow_mut().discontinuities += 1;
                s.reset_view(&r);
            }
            Err(ReadFailure::Closed) => {
                s.reset_view(&r);
                return;
            }
            Err(ReadFailure::Failed(e)) => {
                r.metrics.borrow_mut().host_errors += 1;
                tracing::debug!(error = %e, "reading directory events failed");
                s.timer.sleep_until(s.timer.now() + READ_RETRY).await;
            }
        }
    }
}

/// Reads the asset stream into `r`'s asset bundle.
async fn read_assets<K: Kv, H: Host, T: Timer>(
    s: Rc<Shared<K, H, T>>,
    r: Rc<Runner<H>>,
    mut reader: RecordReader<H::Reader>,
) {
    loop {
        match reader.next_message::<AssetBundle>().await {
            Ok(b) => s.set_bundle(&r, b),
            Err(ReadFailure::Discontinuity) => {}
            Err(ReadFailure::Closed) => return,
            Err(ReadFailure::Failed(e)) => {
                r.metrics.borrow_mut().host_errors += 1;
                tracing::debug!(error = %e, "reading directory assets failed");
                s.timer.sleep_until(s.timer.now() + READ_RETRY).await;
            }
        }
    }
}

/// What the server mode holds, for its teardown.
struct Held<H: Host> {
    swarms: Swarms<H>,
    conn: Option<AbortHandle>,
    publication: Option<Publication<H>>,
}

/// Serves network `r`'s directory while this node holds the lease; see
/// the [module docs](self).
async fn serve_directory<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    r: &Rc<Runner<H>>,
    lease_key: &str,
    token: &Token,
) -> Result<(), String> {
    let mut held = Held {
        swarms: Swarms::default(),
        conn: None,
        publication: None,
    };
    let res = run_server(s, r, lease_key, token, &mut held).await;
    if let Some(p) = held.publication.take()
        && let Err(e) = s.host.unpublish_address(p).await
    {
        r.metrics.borrow_mut().host_errors += 1;
        tracing::warn!(error = %e, "withdrawing the directory address failed");
    }
    let state = r.server.borrow_mut().take();
    if let Some(st) = &state {
        for a in st.subs.values() {
            a.abort();
        }
    }
    drop(state);
    if let Some(a) = held.conn.take() {
        a.abort();
    }
    held.swarms.remove(s, r).await;
    res
}

async fn run_server<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    r: &Rc<Runner<H>>,
    lease_key: &str,
    token: &Token,
    held: &mut Held<H>,
) -> Result<(), String> {
    let config = r
        .record
        .borrow()
        .server_config
        .clone()
        .ok_or("no server config")?;
    let root = config.key.clone().ok_or("no root key")?;
    let cert = s
        .network
        .host_certificate(&r.key)
        .ok_or("no host certificate on the network")?;

    let (events, events_id, events_writer) = s
        .host
        .create_swarm(WriterOptions {
            swarm_options: event_options(&r.key),
            key: root.clone(),
        })
        .await
        .map_err(err)?;
    held.swarms.swarms.push(events);
    let (assets, assets_id, assets_writer) = s
        .host
        .create_swarm(WriterOptions {
            swarm_options: asset_options(&r.key),
            key: root.clone(),
        })
        .await
        .map_err(err)?;
    held.swarms.swarms.push(assets);
    held.swarms.publish(s, r, events, &events_id, EVENT_SALT);
    held.swarms.publish(s, r, assets, &assets_id, ASSET_SALT);

    let (port, reader, writer) = s.host.open_vpn_port(r.net).await.map_err(err)?;
    let mut router = Router::new();
    let service = DirectoryServer {
        shared: Rc::downgrade(s),
        runner: Rc::downgrade(r),
    };
    register_directory_service(&mut router, Rc::new(service));
    let (client, conn) = vpn::connect(
        s.config.server_vpn.clone(),
        &cert,
        reader,
        writer,
        Some(router),
    )
    .map_err(err)?;
    let (conn, abort) = abortable(conn);
    held.conn = Some(abort);
    let runner = Rc::downgrade(r);
    s.spawn(
        async move {
            let res: Result<_, Aborted> = conn.await;
            let Ok(res) = res else {
                return;
            };
            if let Err(e) = res {
                tracing::debug!(error = %e, "the directory server's connection failed");
            }
            if let Some(r) = runner.upgrade() {
                if let Some(st) = r.server.borrow_mut().as_mut() {
                    st.closed = true;
                }
                r.wake();
            }
        }
        .boxed_local(),
    );

    let server = Server::new(
        s.config.limits,
        config.directory.as_ref(),
        fork(&mut s.rng.borrow_mut()),
        s.timer.now(),
    );
    let prev = r.server.replace(Some(ServerState {
        server,
        client,
        actions: VecDeque::new(),
        subs: BTreeMap::new(),
        closed: false,
    }));
    drop(prev);
    let publication = s
        .host
        .publish_address(r.net, root, ADDRESS_SALT.to_vec(), port)
        .await
        .map_err(err)?;
    held.publication = Some(publication);

    let mut events =
        RecordWriter::new(events_writer, EVENT_FRAME, s.config.max_event_record).map_err(err)?;
    let mut assets =
        RecordWriter::new(assets_writer, ASSET_FRAME, s.config.max_asset_record).map_err(err)?;
    let mut written: Option<AssetBundle> = None;
    let mut assets_due = true;
    let mut refresh_at = s.timer.now() + lease::REFRESH;
    loop {
        if assets_due {
            assets_due = false;
            let icon = r
                .record
                .borrow()
                .server_config
                .as_ref()
                .and_then(|c| c.icon.clone());
            let bundle = s
                .with_server(r, |srv, _| srv.asset_bundle(icon))
                .ok_or("the server stopped")?;
            if written.as_ref() != Some(&bundle) {
                let res = match assets.reset().await {
                    Ok(()) => assets.write(&bundle).await.map_err(err),
                    Err(e) => Err(err(e)),
                };
                match res {
                    Ok(()) => {
                        written = Some(bundle.clone());
                        s.set_bundle(r, bundle);
                    }
                    Err(e) => {
                        assets_due = true;
                        r.metrics.borrow_mut().host_errors += 1;
                        tracing::warn!(error = %e, "writing the directory's asset bundle failed");
                    }
                }
            }
        }

        loop {
            let action = r
                .server
                .borrow_mut()
                .as_mut()
                .and_then(|st| st.actions.pop_front());
            let Some(action) = action else {
                break;
            };
            match action {
                ServerAction::Broadcast(records) => {
                    for b in records {
                        if let Err(e) = events.write(&b).await {
                            r.metrics.borrow_mut().host_errors += 1;
                            tracing::warn!(error = %e, "writing directory events failed");
                        }
                        r.view.borrow_mut().handle_broadcast(&b);
                        s.drain_view(r);
                    }
                }
                ServerAction::Subscribe {
                    sub,
                    host,
                    swarm_id,
                    ..
                } => subscribe(s, r, sub, host, swarm_id),
                ServerAction::Unsubscribe { sub } => {
                    let a = r
                        .server
                        .borrow_mut()
                        .as_mut()
                        .and_then(|st| st.subs.remove(&sub));
                    if let Some(a) = a {
                        a.abort();
                    }
                }
            }
        }

        let timeout = r
            .server
            .borrow()
            .as_ref()
            .and_then(|st| st.server.poll_timeout());
        let at = timeout.map_or(refresh_at, |t| t.min(refresh_at));
        match wait(&s.timer, r, Some(at)).await {
            Woke::Stop => return Ok(()),
            Woke::Closed => return Err("the server's connection ended".into()),
            Woke::Changed => {
                let config = r.record.borrow().server_config.clone();
                let Some(config) = config else {
                    return Ok(());
                };
                s.with_server(r, |srv, _| srv.set_config(config.directory.as_ref()));
                assets_due = true;
            }
            Woke::Work => {}
            Woke::Timer => {
                let now = s.timer.now();
                if now >= refresh_at {
                    if let Err(e) = lease::try_lock(s.store.store(), lease_key, token, now).await {
                        return Err(format!("the directory lease was lost: {e}"));
                    }
                    refresh_at = now + lease::REFRESH;
                }
                s.with_server(r, |srv, now| srv.handle_timeout(now));
                if written.is_none() {
                    assets_due = true;
                }
            }
        }
    }
}

/// Starts the server's snippet subscription `sub` to `host`'s snippet of
/// `swarm_id`, feeding its deltas and its end to the server.
fn subscribe<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    r: &Rc<Runner<H>>,
    sub: SubId,
    host: ID,
    swarm_id: SwarmId,
) {
    let client = r.server.borrow().as_ref().map(|st| st.client.clone());
    let Some(client) = client else {
        return;
    };
    let snippets = DirectorySnippetClient::new(client.to(HostAddr::new(host, SNIPPET_PORT)));
    let call = snippets.subscribe(&SnippetSubscribeRequest {
        swarm_id: swarm_id.as_bytes().to_vec(),
    });
    let shared = Rc::downgrade(s);
    let runner = Rc::downgrade(r);
    let task = async move {
        if let Ok(mut stream) = call.await {
            while let Some(Ok(res)) = stream.next().await {
                let (Some(s), Some(r)) = (shared.upgrade(), runner.upgrade()) else {
                    return;
                };
                if let Some(d) = res.snippet_delta {
                    s.with_server(&r, |srv, now| srv.snippet_delta(now, sub, &d));
                }
            }
        }
        if let (Some(s), Some(r)) = (shared.upgrade(), runner.upgrade()) {
            let a = r
                .server
                .borrow_mut()
                .as_mut()
                .and_then(|st| st.subs.remove(&sub));
            drop(a);
            s.with_server(&r, |srv, now| srv.snippet_ended(now, sub));
        }
    };
    let (task, abort) = abortable(task);
    if let Some(st) = r.server.borrow_mut().as_mut() {
        st.subs.insert(sub, abort);
    }
    s.spawn(task.map(|_| ()).boxed_local());
}

// --- the member port --------------------------------------------------------

/// Runs network `r`'s member port: serves snippets and carries this node's
/// calls to the server, until its connection ends.
pub(crate) async fn member_port<K: Kv, H: Host, T: Timer>(
    s: Rc<Shared<K, H, T>>,
    r: Rc<Runner<H>>,
) {
    match open_member_port(&s, &r).await {
        Ok(conn) => {
            if let Err(e) = conn.await {
                tracing::debug!(error = %e, "the directory member port failed");
            }
        }
        Err(e) => {
            r.metrics.borrow_mut().host_errors += 1;
            tracing::warn!(network = r.network_id, error = %e, "opening the directory member port failed");
        }
    }
    let prev = r.member.replace(MemberPort::Closed);
    drop(prev);
    if !r.stopped.get() {
        r.member_retry.set(Some(s.timer.now() + MEMBER_RETRY));
    }
}

async fn open_member_port<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    r: &Rc<Runner<H>>,
) -> Result<VpnConnection<VpnReader<H>, VpnWriter<H>>, String> {
    let cert = s
        .network
        .host_certificate(&r.key)
        .ok_or("no host certificate on the network")?;
    let (reader, writer) = s
        .host
        .open_vpn_port_at(r.net, SNIPPET_PORT)
        .await
        .map_err(err)?;
    let mut router = Router::new();
    let service = SnippetService {
        shared: Rc::downgrade(s),
        runner: Rc::downgrade(r),
    };
    register_directory_snippet_service(&mut router, Rc::new(service));
    let (client, conn) = vpn::connect(
        s.config.member_vpn.clone(),
        &cert,
        reader,
        writer,
        Some(router),
    )
    .map_err(err)?;
    let opening = match &*r.member.borrow() {
        MemberPort::Opening(abort) => Some(abort.clone()),
        _ => None,
    };
    if let Some(abort) = opening {
        let prev = r.member.replace(MemberPort::Open { client, abort });
        drop(prev);
    }
    Ok(conn)
}

/// The snippets this node serves, and their subscribers.
#[derive(Default)]
pub(crate) struct SnippetHub {
    snippets: RefCell<LocalSnippets>,
    /// Each subscriber's swarm and waker, by slot. Bound:
    /// [`MAX_SNIPPET_SUBSCRIBERS`].
    subscribers: RefCell<BTreeMap<u64, (SwarmId, Option<Waker>)>>,
    next_slot: Cell<u64>,
}

impl SnippetHub {
    pub(crate) fn push(&self, id: &SwarmId, snippet: ListingSnippet) -> Result<(), SnippetError> {
        let res = self.snippets.borrow_mut().push(id.clone(), snippet);
        res?;
        self.wake(id);
        Ok(())
    }

    pub(crate) fn delete(&self, id: &SwarmId) {
        let deleted = self.snippets.borrow_mut().delete(id);
        if deleted {
            self.wake(id);
        }
    }

    fn wake(&self, id: &SwarmId) {
        let wakers: Vec<Waker> = self
            .subscribers
            .borrow_mut()
            .values_mut()
            .filter(|(s, _)| s == id)
            .filter_map(|(_, w)| w.take())
            .collect();
        for w in wakers {
            w.wake();
        }
    }

    /// A stream of the snippet of `id`, within the subscriber bounds.
    fn subscribe(self: &Rc<Self>, id: SwarmId) -> Result<SnippetStream, RpcError> {
        if self.snippets.borrow().get(&id).is_none() {
            return Err(snippet_not_found());
        }
        let slot = {
            let mut subs = self.subscribers.borrow_mut();
            let of_swarm = subs.values().filter(|(s, _)| *s == id).count();
            if subs.len() >= MAX_SNIPPET_SUBSCRIBERS || of_swarm >= MAX_SUBSCRIBERS_PER_SWARM {
                return Err(RpcError::new(0, "too many snippet subscribers"));
            }
            let slot = self.next_slot.get();
            self.next_slot.set(slot + 1);
            subs.insert(slot, (id.clone(), None));
            slot
        };
        Ok(SnippetStream {
            hub: self.clone(),
            slot,
            id,
            sent: ListingSnippet::default(),
            version: 0,
            pending: VecDeque::new(),
        })
    }
}

fn snippet_not_found() -> RpcError {
    RpcError::new(0, "snippet not found")
}

/// A subscriber's deltas of one snippet; see the [module docs](self).
struct SnippetStream {
    hub: Rc<SnippetHub>,
    slot: u64,
    id: SwarmId,
    /// The snippet the subscriber has, and the version it is of.
    sent: ListingSnippet,
    version: u64,
    /// Deltas to send. Bound: one split delta.
    pending: VecDeque<ListingSnippetDelta>,
}

impl Stream for SnippetStream {
    type Item = Result<SnippetSubscribeResponse, RpcError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if let Some(d) = this.pending.pop_front() {
                return Poll::Ready(Some(Ok(SnippetSubscribeResponse {
                    snippet_delta: Some(d),
                })));
            }
            let next = {
                let snippets = this.hub.snippets.borrow();
                match snippets.get(&this.id) {
                    None => return Poll::Ready(None),
                    Some((s, v)) if v != this.version => Some((s.clone(), v)),
                    Some(_) => None,
                }
            };
            let Some((current, version)) = next else {
                break;
            };
            let d = diff(&this.sent, &current);
            this.sent = current;
            this.version = version;
            if !is_empty(&d) {
                this.pending.extend(split(d));
            }
        }
        let prev = this
            .hub
            .subscribers
            .borrow_mut()
            .get_mut(&this.slot)
            .and_then(|(_, w)| w.replace(cx.waker().clone()));
        drop(prev);
        Poll::Pending
    }
}

impl Drop for SnippetStream {
    fn drop(&mut self) {
        let removed = self.hub.subscribers.borrow_mut().remove(&self.slot);
        drop(removed);
    }
}

/// `strims.network.v1.directory.DirectorySnippet` for one network, on its
/// member port.
struct SnippetService<K: Kv, H: Host, T: Timer> {
    shared: Weak<Shared<K, H, T>>,
    runner: Weak<Runner<H>>,
}

impl<K: Kv, H: Host, T: Timer> DirectorySnippetService for SnippetService<K, H, T> {
    async fn subscribe(
        &self,
        _: &Context,
        req: SnippetSubscribeRequest,
    ) -> Result<ServerStream<SnippetSubscribeResponse>, RpcError> {
        let (Some(s), Some(r)) = (self.shared.upgrade(), self.runner.upgrade()) else {
            return Err(snippet_not_found());
        };
        let res = admit(&s, &r, &req.swarm_id);
        let mut m = r.metrics.borrow_mut();
        match res {
            Ok(stream) => {
                m.snippets_served += 1;
                Ok(Box::pin(stream))
            }
            Err(e) => {
                m.snippets_refused += 1;
                Err(e)
            }
        }
    }
}

/// Admits a subscription to the snippet of swarm `swarm_id`; see the
/// [module docs](self).
fn admit<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    r: &Runner<H>,
    swarm_id: &[u8],
) -> Result<SnippetStream, RpcError> {
    if swarm_id.is_empty() || swarm_id.len() > listing::MAX_SWARM_ID_BYTES {
        return Err(snippet_not_found());
    }
    let id = SwarmId::new(swarm_id);
    if !s.transfers.is_published(TransferId::new(&id, b""), r.net) {
        return Err(snippet_not_found());
    }
    s.snippets.subscribe(id)
}
