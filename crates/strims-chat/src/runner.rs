// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A chat server's runner on this node: its mode task, which serves the
//! server, reads it for client streams, or waits, and the listing keeper
//! of a served server.
//!
//! # Modes
//!
//! A runner's mode task runs one mode at a time, so that one mode's swarms,
//! transfers and address are gone before the next one's start:
//!
//! - **Serving**, while the profile owns the server's record, the node has
//!   joined the record's network, the profile's membership is trusted, and
//!   this process holds the lease [`lease_key`]. It creates both swarms
//!   signed with the server's key, publishes their transfers, serves
//!   `chat.v1.Chat` on a vpn port and publishes the port's address under
//!   (server key, [`ADDRESS_SALT`]). It then loads the server's assets and
//!   active mutes, writes a unified asset bundle, and has the listing
//!   keeper publish the server's directory listing. While it serves, it
//!   writes the server's events, syncs the assets a debounce after they
//!   change in the store, takes record changes, and refreshes the lease
//!   every [`lease::REFRESH`]. It ends when a condition fails, the lease
//!   is lost or the connection ends, and tears down in reverse: the
//!   listing (best effort), the address, the connection, the transfers,
//!   the swarms and the lease.
//! - **Reading**, while client streams are open or linger, when it
//!   doesn't serve: it adds both swarms by the server key and publishes
//!   their transfers on the network. It tries the lease after every
//!   [`lease::recheck_delay`] while serving is otherwise possible.
//! - **Idle** otherwise, trying the lease on the same schedule.
//!
//! Client streams read whichever swarms the mode holds, and reopen their
//! readers on the next mode's.
//!
//! # Listing keeper
//!
//! While the server is served, the keeper publishes its listing
//! `Chat{key, name: room name}`, at most once per
//! [`Config::min_republish`](crate::Config::min_republish): at the start,
//! when the control asks (the directory restarted, unpublished the
//! listing, or lagged without it), after a failed publish, and when a
//! check every [`Config::listing_check`](crate::Config::listing_check)
//! finds the view without it. Publishing is idempotent and answers the
//! listing's id; a new id replaces the server's nicks with the listing's
//! users.
//!
//! The directory names a chat listing by its key alone and keeps its
//! content while it lists it, so when the room's name changes the keeper
//! unpublishes the listing, waits until the view drops it (or a check
//! interval passes), and publishes the renamed one.

use std::future::{Future, poll_fn};
use std::pin::pin;
use std::rc::Rc;
use std::task::Poll;

use futures_util::FutureExt;
use futures_util::future::{AbortHandle, Aborted, Either, abortable, select};
use strims_base::rng::fork;
use strims_base::time::{Duration, Time};
use strims_crypto::membership_trusted;
use strims_dao::key::Part;
use strims_dao::lease::{self, Token};
use strims_dao::schema::chat::{
    ChatEmotes, ChatModifiers, ChatProfiles, ChatServerIcons, ChatServers, ChatTags,
};
use strims_dao::{ReplicatedStore, Table, ref_key};
use strims_directory::{Host, Net};
use strims_kv::Kv;
use strims_ppspp::{SwarmId, WriterOptions};
use strims_proto::chat::v1::{Profile, register_chat_service};
use strims_proto::network::v1::directory::listing::{Chat, Content};
use strims_proto::network::v1::directory::listing_query::Query;
use strims_proto::network::v1::directory::{Listing, ListingQuery};
use strims_rpc::vpn;
use strims_rpc::{Router, Timer};
use strims_transfer::{RecordWriter, SwarmHost, TransferId};

use crate::assets::{AssetPublisher, Assets};
use crate::control::{Runner, ServerMode, ServerState, Shared, Swarm};
use crate::events::ChatEvent;
use crate::reader::MAX_EVENT_RECORD;
use crate::services::ChatServer;
use crate::{
    ADDRESS_SALT, ASSET_FRAME, ASSET_SALT, ASSET_WINDOW_BYTES, EVENT_FRAME, EVENT_SALT, Server,
    asset_options, event_options,
};

/// The wait after the swarms of a room failed to be added before trying
/// again.
const READ_RETRY: Duration = Duration::from_secs(10);
/// The wait after a failed asset write before the assets are synced
/// again, unified.
const ASSET_RETRY: Duration = Duration::from_secs(5);

type Publication<H> = <H as strims_network::Host>::Publication;
type Writer<H> = <H as SwarmHost>::Writer;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// The key of the lease whose holder serves the profile's chat server
/// `server_id`.
pub fn lease_key(server_id: u64) -> String {
    lease::key_parts(&[Part::Str("chat"), Part::Id(server_id)])
}

/// What the runner's state allows now.
struct Want<N> {
    /// The network's handle, once joined.
    net: Option<N>,
    /// The record's id, when serving is possible but for the lease.
    serve: Option<u64>,
    /// Client streams read the server.
    read: bool,
}

fn want<K: Kv, H: Host, T: Timer>(s: &Shared<K, H, T>, r: &Runner<H>) -> Want<Net<H>> {
    let now = s.timer.now();
    let info = s.network.network(&r.network_key);
    let net = info.as_ref().and_then(|i| i.net);
    let trusted = info
        .as_ref()
        .and_then(|i| i.network.certificate.as_ref())
        .is_some_and(|c| membership_trusted(c, now));
    let serve = if net.is_some() && trusted {
        r.server_id()
    } else {
        None
    };
    Want {
        net,
        serve,
        read: net.is_some() && r.has_readers(),
    }
}

/// What woke a mode task.
enum Woke {
    Stop,
    /// The runner's state changed.
    Changed,
    /// The server's connection ended.
    Closed,
    /// The server has events to write.
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
            if !st.events.is_empty() {
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

fn recheck<K: Kv, H: Host, T: Timer>(s: &Shared<K, H, T>) -> Time {
    s.timer.now() + lease::recheck_delay(&mut s.rng.borrow_mut())
}

fn due<K: Kv, H: Host, T: Timer>(s: &Shared<K, H, T>, at: Option<Time>) -> bool {
    at.is_none_or(|t| t <= s.timer.now())
}

async fn try_lease<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    server_id: u64,
    token: &Token,
) -> bool {
    let key = lease_key(server_id);
    match lease::try_lock(s.store.store(), &key, token, s.timer.now()).await {
        Ok(()) => true,
        Err(strims_dao::Error::LeaseBusy) => false,
        Err(e) => {
            s.metrics.borrow_mut().store_errors += 1;
            tracing::warn!(error = %e, "taking the chat server lease failed");
            false
        }
    }
}

async fn release<K: Kv, H: Host, T: Timer>(s: &Shared<K, H, T>, server_id: u64, token: &Token) {
    let key = lease_key(server_id);
    if let Err(e) = lease::release(s.store.store(), &key, token, s.timer.now()).await {
        s.metrics.borrow_mut().store_errors += 1;
        tracing::warn!(error = %e, "releasing the chat server lease failed");
    }
}

/// Runs `r` until it stops; see the [module docs](self).
pub(crate) async fn run<K: Kv, H: Host, T: Timer>(s: Rc<Shared<K, H, T>>, r: Rc<Runner<H>>) {
    let previous = r.previous.borrow_mut().take();
    if let Some(p) = previous {
        wait_done(&p).await;
    }
    let token = lease::new_token(&mut s.rng.borrow_mut());
    let mut lease_at: Option<Time> = None;
    while !r.stopped.get() {
        let w = want(&s, &r);
        let mut leased = None;
        if let (Some(net), Some(id)) = (w.net, w.serve)
            && due(&s, lease_at)
        {
            if try_lease(&s, id, &token).await {
                leased = Some((net, id));
            } else {
                lease_at = Some(recheck(&s));
            }
        }
        if leased.is_none() {
            match w.net.filter(|_| w.read) {
                Some(net) => leased = read(&s, &r, net, &token, &mut lease_at).await,
                None => {
                    let at = w.serve.and(lease_at);
                    wait(&s.timer, &r, at).await;
                }
            }
        }
        if let Some((net, id)) = leased {
            if !r.stopped.get() {
                serve(&s, &r, net, id, &token).await;
            }
            release(&s, id, &token).await;
            lease_at = Some(recheck(&s));
        }
    }
    s.finished(&r);
}

/// What a mode added to the node, for its teardown.
struct Held<H: Host> {
    swarms: Vec<Swarm<H>>,
    transfers: Vec<TransferId>,
    conn: Option<AbortHandle>,
    publication: Option<Publication<H>>,
    keeper: Option<AbortHandle>,
    /// The server id, once serving was reported.
    serving: Option<u64>,
}

impl<H: Host> Default for Held<H> {
    fn default() -> Self {
        Held {
            swarms: Vec::new(),
            transfers: Vec::new(),
            conn: None,
            publication: None,
            keeper: None,
            serving: None,
        }
    }
}

impl<H: Host> Held<H> {
    /// Adds and publishes the transfer of `swarm` under `salt`.
    fn publish<K: Kv, T: Timer>(
        &mut self,
        s: &Shared<K, H, T>,
        swarm: Swarm<H>,
        id: &SwarmId,
        salt: &[u8],
        net: Net<H>,
    ) {
        let t = s.transfers.add(swarm, id, salt.to_vec());
        s.transfers.publish(t, net);
        self.transfers.push(t);
    }

    /// Removes the transfers, then the swarms.
    async fn remove_swarms<K: Kv, T: Timer>(&mut self, s: &Shared<K, H, T>) {
        for t in self.transfers.drain(..) {
            s.transfers.remove(t);
        }
        for swarm in std::mem::take(&mut self.swarms) {
            if let Err(e) = s.host.remove_swarm(swarm).await {
                s.metrics.borrow_mut().host_errors += 1;
                tracing::warn!(error = %e, "removing a chat swarm failed");
            }
        }
    }
}

// --- reading ----------------------------------------------------------------

/// Reads the server's swarms for client streams until the state no longer
/// asks for it (`None`), or until this process takes the lease.
async fn read<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    r: &Rc<Runner<H>>,
    net: Net<H>,
    token: &Token,
    lease_at: &mut Option<Time>,
) -> Option<(Net<H>, u64)> {
    let mut held = Held::default();
    let swarms = match add_swarms(s, r, net, &mut held).await {
        Ok(swarms) => swarms,
        Err(e) => {
            s.metrics.borrow_mut().host_errors += 1;
            tracing::warn!(error = %e, "reading a chat server failed");
            held.remove_swarms(s).await;
            wait(&s.timer, r, Some(s.timer.now() + READ_RETRY)).await;
            return None;
        }
    };
    r.set_swarms(Some(swarms));
    r.mode.set(ServerMode::Reading);
    s.metrics.borrow_mut().reads += 1;
    s.emit(ChatEvent::ReadStarted {
        server_key: r.key.clone(),
        network_key: r.network_key.clone(),
    });
    let mut leased = None;
    while !r.stopped.get() {
        let w = want(s, r);
        if w.net != Some(net) || !w.read {
            break;
        }
        if let Some(id) = w.serve
            && due(s, *lease_at)
        {
            if try_lease(s, id, token).await {
                leased = Some((net, id));
                break;
            }
            *lease_at = Some(recheck(s));
        }
        wait(&s.timer, r, w.serve.and(*lease_at)).await;
    }
    r.set_swarms(None);
    r.mode.set(ServerMode::Idle);
    held.remove_swarms(s).await;
    s.emit(ChatEvent::ReadStopped {
        server_key: r.key.clone(),
        network_key: r.network_key.clone(),
    });
    leased
}

/// Adds both swarms by the server key and publishes their transfers.
async fn add_swarms<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    r: &Runner<H>,
    net: Net<H>,
    held: &mut Held<H>,
) -> Result<(Swarm<H>, Swarm<H>), String> {
    let id = SwarmId::new(&r.key);
    let events = s
        .host
        .add_swarm(id.clone(), event_options(&r.key))
        .await
        .map_err(err)?;
    held.swarms.push(events);
    let assets = s
        .host
        .add_swarm(id.clone(), asset_options(&r.key))
        .await
        .map_err(err)?;
    held.swarms.push(assets);
    held.publish(s, events, &id, EVENT_SALT, net);
    held.publish(s, assets, &id, ASSET_SALT, net);
    Ok((events, assets))
}

// --- serving ----------------------------------------------------------------

/// Serves the server while this process holds the lease; see the
/// [module docs](self).
async fn serve<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    r: &Rc<Runner<H>>,
    net: Net<H>,
    server_id: u64,
    token: &Token,
) {
    let mut held = Held::default();
    if let Err(e) = run_server(s, r, net, server_id, token, &mut held).await {
        tracing::warn!(server = server_id, error = %e, "the chat server stopped");
    }
    teardown(s, r, held).await;
}

async fn run_server<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    r: &Rc<Runner<H>>,
    net: Net<H>,
    server_id: u64,
    token: &Token,
    held: &mut Held<H>,
) -> Result<(), String> {
    let record = r
        .record
        .borrow()
        .clone()
        .ok_or("the server record is gone")?;
    let key = record.key.clone().ok_or("the server record has no key")?;
    let cert = s
        .network
        .host_certificate(&r.network_key)
        .ok_or("no host certificate on the network")?;
    let network_id = s
        .network
        .network(&r.network_key)
        .map(|i| i.network.id)
        .ok_or("the network is gone")?;

    let (event_swarm, event_id, event_writer) = s
        .host
        .create_swarm(WriterOptions {
            swarm_options: event_options(&key.public),
            key: key.clone(),
        })
        .await
        .map_err(err)?;
    held.swarms.push(event_swarm);
    let (asset_swarm, asset_id, asset_writer) = s
        .host
        .create_swarm(WriterOptions {
            swarm_options: asset_options(&key.public),
            key: key.clone(),
        })
        .await
        .map_err(err)?;
    held.swarms.push(asset_swarm);
    held.publish(s, event_swarm, &event_id, EVENT_SALT, net);
    held.publish(s, asset_swarm, &asset_id, ASSET_SALT, net);

    let (port, reader, writer) = s.host.open_vpn_port(net).await.map_err(err)?;
    let mut router = Router::new();
    register_chat_service(&mut router, Rc::new(ChatServer::new(s, r)));
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
                tracing::debug!(error = %e, "the chat server's connection failed");
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
        &record,
        &s.owner_peer_key(),
        s.config.server,
        fork(&mut s.rng.borrow_mut()),
    );
    let prev = r.server.replace(Some(ServerState {
        server,
        server_id,
        network_id,
        client,
        events: Default::default(),
        profiles: Vec::new(),
        closed: false,
    }));
    drop(prev);
    let publication = s
        .host
        .publish_address(net, key, ADDRESS_SALT.to_vec(), port)
        .await
        .map_err(err)?;
    held.publication = Some(publication);

    let profiles = load_profiles(&s.store, server_id).await.map_err(|e| {
        s.metrics.borrow_mut().store_errors += 1;
        err(e)
    })?;
    s.with_server(r, |srv, now| srv.set_mutes(now, &profiles));
    drop(profiles);
    let mut events = RecordWriter::new(event_writer, EVENT_FRAME, MAX_EVENT_RECORD).map_err(err)?;
    let mut assets =
        RecordWriter::new(asset_writer, ASSET_FRAME, ASSET_WINDOW_BYTES).map_err(err)?;
    let mut publisher = AssetPublisher::new();
    r.assets_due.set(None);
    r.unified_due.set(false);
    sync_assets(s, r, server_id, &mut publisher, &mut assets, true).await?;

    r.set_swarms(Some((event_swarm, asset_swarm)));
    r.mode.set(ServerMode::Serving);
    s.metrics.borrow_mut().serves += 1;
    held.serving = Some(server_id);
    s.emit(ChatEvent::ServeStarted {
        server_id,
        server_key: r.key.clone(),
        network_key: r.network_key.clone(),
    });
    r.republish.set(true);
    let (keeper, abort) = abortable(keep_listing(s.clone(), r.clone(), server_id, network_id));
    held.keeper = Some(abort);
    s.spawn(keeper.map(|_| ()).boxed_local());

    let mut refresh_at = s.timer.now() + lease::REFRESH;
    loop {
        loop {
            let e = r
                .server
                .borrow_mut()
                .as_mut()
                .and_then(|st| st.events.pop_front());
            let Some(e) = e else {
                break;
            };
            match events.write(&e).await {
                Ok(()) => s.metrics.borrow_mut().events_written += 1,
                Err(e) => {
                    s.metrics.borrow_mut().write_errors += 1;
                    tracing::warn!(error = %e, "writing a chat event failed");
                }
            }
        }

        let now = s.timer.now();
        if r.assets_due.get().is_some_and(|t| t <= now) {
            r.assets_due.set(None);
            let unified = r.unified_due.replace(false);
            if let Err(e) = sync_assets(s, r, server_id, &mut publisher, &mut assets, unified).await
            {
                tracing::warn!(server = server_id, error = %e, "syncing chat assets failed");
            }
            continue;
        }

        let at = r.assets_due.get().map_or(refresh_at, |t| t.min(refresh_at));
        match wait(&s.timer, r, Some(at)).await {
            Woke::Stop => return Ok(()),
            Woke::Closed => return Err("the server's connection ended".into()),
            Woke::Changed => {
                let w = want(s, r);
                if w.net != Some(net) || w.serve != Some(server_id) {
                    return Ok(());
                }
                let record = r.record.borrow().clone();
                if let Some(record) = record {
                    s.with_server(r, |srv, _| srv.set_record(&record));
                }
                r.poke_keeper();
            }
            Woke::Work => {}
            Woke::Timer => {
                let now = s.timer.now();
                if now >= refresh_at {
                    let key = lease_key(server_id);
                    if let Err(e) = lease::try_lock(s.store.store(), &key, token, now).await {
                        return Err(format!("the chat server lease was lost: {e}"));
                    }
                    refresh_at = now + lease::REFRESH;
                }
            }
        }
    }
}

/// Takes down what serving added, in reverse; see the [module docs](self).
async fn teardown<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    r: &Rc<Runner<H>>,
    mut held: Held<H>,
) {
    if let Some(a) = held.keeper.take() {
        a.abort();
    }
    if let Some(id) = r.listing_id.take() {
        let deadline = s.timer.now() + s.config.unpublish_timeout;
        let unpublish = pin!(s.directory.unpublish(&r.network_key, id));
        let timeout = pin!(s.timer.sleep_until(deadline));
        match select(unpublish, timeout).await {
            Either::Left((Err(e), _)) => {
                tracing::debug!(error = %e, "unpublishing a chat listing failed");
            }
            Either::Right(_) => tracing::debug!("unpublishing a chat listing timed out"),
            Either::Left((Ok(()), _)) => {}
        }
    }
    if let Some(p) = held.publication.take()
        && let Err(e) = s.host.unpublish_address(p).await
    {
        s.metrics.borrow_mut().host_errors += 1;
        tracing::warn!(error = %e, "withdrawing a chat server address failed");
    }
    let state = r.server.borrow_mut().take();
    drop(state);
    if let Some(a) = held.conn.take() {
        a.abort();
    }
    r.set_swarms(None);
    r.mode.set(ServerMode::Idle);
    held.remove_swarms(s).await;
    if let Some(server_id) = held.serving {
        s.emit(ChatEvent::ServeStopped {
            server_id,
            server_key: r.key.clone(),
            network_key: r.network_key.clone(),
        });
    }
}

async fn load_profiles<K: Kv>(
    store: &ReplicatedStore<K>,
    server_id: u64,
) -> Result<Vec<Profile>, strims_dao::Error> {
    let mut tx = store.read().await?;
    ChatProfiles::index_get_all(&mut tx, &ChatProfiles::BY_SERVER, &ref_key(server_id)).await
}

/// The stored assets of server `server_id`; `None` once its record is
/// gone.
async fn load_assets<K: Kv>(
    store: &ReplicatedStore<K>,
    server_id: u64,
) -> Result<Option<Assets>, strims_dao::Error> {
    let mut tx = store.read().await?;
    let Some(server) = ChatServers::get(&mut tx, server_id).await? else {
        return Ok(None);
    };
    let icon = ChatServerIcons::get(&mut tx, server_id).await?;
    let by = ref_key(server_id);
    let emotes = ChatEmotes::index_get_all(&mut tx, &ChatEmotes::BY_SERVER, &by).await?;
    let modifiers = ChatModifiers::index_get_all(&mut tx, &ChatModifiers::BY_SERVER, &by).await?;
    let tags = ChatTags::index_get_all(&mut tx, &ChatTags::BY_SERVER, &by).await?;
    Ok(Some(Assets {
        server,
        icon,
        emotes,
        modifiers,
        tags,
    }))
}

/// Loads the server's assets into the server, and writes the bundle that
/// brings the asset stream up to them. A failed write makes the next sync,
/// [`ASSET_RETRY`] later, unified. A record deleted since the sync was
/// scheduled leaves everything as it is: the serving ends with it.
async fn sync_assets<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    r: &Runner<H>,
    server_id: u64,
    publisher: &mut AssetPublisher,
    writer: &mut RecordWriter<Writer<H>>,
    unified: bool,
) -> Result<(), String> {
    let loaded = load_assets(&s.store, server_id).await.map_err(|e| {
        s.metrics.borrow_mut().store_errors += 1;
        err(e)
    })?;
    let Some(assets) = loaded else {
        return Ok(());
    };
    s.with_server(r, |srv, _| {
        srv.set_record(&assets.server);
        srv.sync_assets(&assets.emotes, &assets.modifiers, &assets.tags);
    });
    let write = match publisher.sync(&assets, unified) {
        Ok(Some(w)) => w,
        Ok(None) => return Ok(()),
        Err(e) => {
            s.metrics.borrow_mut().assets_too_large += 1;
            tracing::warn!(server = server_id, error = %e, "a chat server's assets are too large");
            return Ok(());
        }
    };
    drop(assets);
    let res = async {
        if write.reset {
            writer.reset().await.map_err(err)?;
        }
        writer.write(&write.bundle).await.map_err(err)
    }
    .await;
    match res {
        Ok(()) => {
            let mut m = s.metrics.borrow_mut();
            m.asset_bundles += 1;
            m.asset_resets += u64::from(write.reset);
        }
        Err(e) => {
            s.metrics.borrow_mut().write_errors += 1;
            tracing::warn!(server = server_id, error = %e, "writing a chat asset bundle failed");
            publisher.reset();
            r.assets_due.set(Some(s.timer.now() + ASSET_RETRY));
        }
    }
    Ok(())
}

// --- the listing keeper -----------------------------------------------------

/// The listing of `r`'s server, while the profile owns it.
fn chat_listing<H: Host>(r: &Runner<H>) -> Option<Listing> {
    let record = r.record.borrow();
    let record = record.as_ref()?;
    Some(Listing {
        content: Some(Content::Chat(Chat {
            key: r.key.clone(),
            name: record
                .room
                .as_ref()
                .map(|room| room.name.clone())
                .unwrap_or_default(),
        })),
    })
}

/// Whether network `network_id`'s view holds listing `id` as `r`'s
/// server's listing.
pub(crate) fn listed<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    r: &Runner<H>,
    network_id: u64,
    id: u64,
) -> bool {
    let Some(l) = s.directory.listing(network_id, &by_id(id)) else {
        return false;
    };
    chat_listing(r).is_some_and(|want| l.listing == want)
}

/// Waits for the keeper's next poke, or until `at`.
async fn wait_keeper<H: Host, T: Timer>(timer: &T, r: &Runner<H>, at: Time) {
    let mut sleep = Box::pin(timer.sleep_until(at));
    poll_fn(|cx| {
        if r.keeper_poked.replace(false) || r.stopped.get() {
            return Poll::Ready(());
        }
        let prev = r.keeper_waker.replace(Some(cx.waker().clone()));
        drop(prev);
        if sleep.as_mut().poll(cx).is_ready() {
            return Poll::Ready(());
        }
        Poll::Pending
    })
    .await;
}

/// What the directory holds of the server's listing, as the keeper last
/// saw it.
enum ListingState {
    /// Nothing, or nothing known.
    Unlisted,
    /// Listing `id`, as `listing`.
    Listed { id: u64, listing: Listing },
    /// Listing `id`, unpublished at `since` for a rename, until the view
    /// drops it.
    Retiring { id: u64, since: Time },
}

/// Keeps the served server's listing published; see the
/// [module docs](self).
async fn keep_listing<K: Kv, H: Host, T: Timer>(
    s: Rc<Shared<K, H, T>>,
    r: Rc<Runner<H>>,
    server_id: u64,
    network_id: u64,
) {
    let mut state = ListingState::Unlisted;
    let mut last: Option<Time> = None;
    let mut check_at = s.timer.now() + s.config.listing_check;
    while !r.stopped.get() {
        let Some(listing) = chat_listing(&r) else {
            return;
        };
        let now = s.timer.now();
        let publish_at = last.map_or(now, |t| t + s.config.min_republish);
        match &state {
            // The directory keeps a listing's content while it lists it,
            // so a renamed listing is published again once the view no
            // longer holds the old one.
            ListingState::Retiring { id, since } => {
                let gone = s.directory.listing(network_id, &by_id(*id)).is_none();
                let give_up = *since + s.config.listing_check;
                if gone || now >= give_up {
                    state = ListingState::Unlisted;
                    r.republish.set(true);
                } else {
                    wait_keeper(&s.timer, &r, give_up).await;
                }
                continue;
            }
            ListingState::Listed { id, listing: l } if *l != listing => {
                if publish_at > now {
                    wait_keeper(&s.timer, &r, publish_at).await;
                    continue;
                }
                let id = *id;
                last = Some(now);
                match s.directory.unpublish(&r.network_key, id).await {
                    Ok(()) => state = ListingState::Retiring { id, since: now },
                    Err(e) => {
                        s.metrics.borrow_mut().listing_errors += 1;
                        tracing::debug!(server = server_id, error = %e, "unpublishing a chat listing failed");
                    }
                }
                continue;
            }
            _ => {}
        }
        let due = r.republish.get() || matches!(state, ListingState::Unlisted);
        if due && publish_at <= now {
            r.republish.set(false);
            last = Some(now);
            match s.directory.publish(&r.network_key, listing.clone()).await {
                Ok(id) => {
                    s.metrics.borrow_mut().listing_publishes += 1;
                    if r.listing_id.get() != Some(id) {
                        set_listing(&s, &r, server_id, network_id, id);
                    }
                    state = ListingState::Listed { id, listing };
                }
                Err(e) => {
                    s.metrics.borrow_mut().listing_errors += 1;
                    tracing::debug!(server = server_id, error = %e, "publishing a chat listing failed");
                    r.republish.set(true);
                }
            }
            continue;
        }
        if now >= check_at {
            check_at = now + s.config.listing_check;
            if let ListingState::Listed { id, .. } = &state
                && !listed(&s, &r, network_id, *id)
            {
                r.republish.set(true);
            }
            continue;
        }
        let at = if due {
            publish_at.min(check_at)
        } else {
            check_at
        };
        wait_keeper(&s.timer, &r, at).await;
    }
}

fn by_id(id: u64) -> ListingQuery {
    ListingQuery {
        query: Some(Query::Id(id)),
    }
}

/// The server's listing has id `id` now: its nicks are the listing's
/// users.
fn set_listing<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    r: &Runner<H>,
    server_id: u64,
    network_id: u64,
    id: u64,
) {
    r.listing_id.set(Some(id));
    let users = s.directory.listing_users(network_id, id);
    s.with_server(r, |srv, _| srv.reset_nicks(&users));
    s.emit(ChatEvent::ListingChanged {
        server_id,
        server_key: r.key.clone(),
        network_key: r.network_key.clone(),
        listing_id: id,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lease_key_names_chat_and_the_server() {
        assert_eq!(
            lease_key(36),
            lease::key_parts(&[Part::Str("chat"), Part::Id(36)])
        );
        assert!(lease_key(36).ends_with(":chat:10"));
    }
}
