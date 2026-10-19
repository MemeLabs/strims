// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The whisper ports: `chat.v1.Whisper` for the whispers others send, and
//! the delivery attempts of those the profile sends; see the
//! [module docs](super).

use std::cell::{Cell, RefCell};
use std::future::poll_fn;
use std::rc::{Rc, Weak};
use std::task::{Poll, Waker};

use futures_util::FutureExt;
use futures_util::future::{AbortHandle, Aborted, Either, abortable, select};
use strims_crypto::CertificateChain;
use strims_dao::Table;
use strims_dao::schema::chat::{
    ChatUiConfigIgnores, WhisperRecords, WhisperThreads, peer_message_id_key,
};
use strims_directory::{Caller, Host, MAX_ALIAS_BYTES, MAX_PEER_KEY_BYTES, Net};
use strims_kv::Kv;
use strims_proto::chat::v1::{
    Message, MessageState, WhisperClient, WhisperRecord, WhisperSendMessageRequest,
    WhisperSendMessageResponse, WhisperService, WhisperThread, register_whisper_service,
};
use strims_proto::service::{CallError, Context, Error as RpcError};
use strims_proto::types::Certificate;
use strims_rpc::vpn::{self, VpnClient};
use strims_rpc::{RetryClient, Router, Timer};

use super::control::Shared;
use super::{Delivery, WHISPER_REFUSED, WhisperRefusal, check_receive, thread_received};
use crate::{ChatError, WHISPER_SALT};

type Publication<H> = <H as strims_network::Host>::Publication;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// --- ports ------------------------------------------------------------------

/// A network's whisper port, while its task runs.
#[derive(Default)]
pub(crate) struct Port {
    client: RefCell<Option<VpnClient>>,
    /// The connection ended.
    closed: Cell<bool>,
    /// The node left the network, or joined it again under another handle.
    stopped: Cell<bool>,
    /// The lease changed since the task last looked.
    changed: Cell<bool>,
    /// The port's address is published.
    published: Cell<bool>,
    /// Wakes the port's task.
    waker: RefCell<Option<Waker>>,
}

impl Port {
    /// The port's client, once its connection runs.
    pub(crate) fn client(&self) -> Option<VpnClient> {
        self.client.borrow().clone()
    }

    pub(crate) fn is_published(&self) -> bool {
        self.published.get()
    }

    /// Has the task withdraw the address and close the port.
    pub(crate) fn stop(&self) {
        self.stopped.set(true);
        self.wake();
    }

    /// Has the task publish or withdraw the address as the lease now says.
    pub(crate) fn changed(&self) {
        self.changed.set(true);
        self.wake();
    }

    fn wake(&self) {
        let w = self.waker.borrow_mut().take();
        if let Some(w) = w {
            w.wake();
        }
    }

    async fn wait(&self) {
        poll_fn(|cx| {
            if self.stopped.get() || self.closed.get() || self.changed.replace(false) {
                return Poll::Ready(());
            }
            let prev = self.waker.replace(Some(cx.waker().clone()));
            drop(prev);
            Poll::Pending
        })
        .await;
    }
}

/// Runs network `key`'s whisper port `p` until its connection ends or it
/// is stopped: serves `chat.v1.Whisper`, and publishes the port's address
/// while this process holds the lease.
pub(crate) async fn run_port<K: Kv, H: Host, T: Timer>(
    s: Rc<Shared<K, H, T>>,
    key: Vec<u8>,
    net: Net<H>,
    p: Rc<Port>,
) {
    let mut conn = None;
    let mut publication = None;
    if let Err(e) = serve_port(&s, &key, net, &p, &mut conn, &mut publication).await {
        s.metrics.borrow_mut().host_errors += 1;
        tracing::warn!(error = %e, "the whisper port closed");
    }
    p.published.set(false);
    if let Some(pb) = publication
        && let Err(e) = s.host.unpublish_address(pb).await
    {
        s.metrics.borrow_mut().host_errors += 1;
        tracing::debug!(error = %e, "withdrawing the whisper address failed");
    }
    if let Some(a) = conn {
        a.abort();
    }
    let prev = p.client.replace(None);
    drop(prev);
    s.port_closed(&key, &p);
}

async fn serve_port<K: Kv, H: Host, T: Timer>(
    s: &Rc<Shared<K, H, T>>,
    key: &[u8],
    net: Net<H>,
    p: &Rc<Port>,
    conn: &mut Option<AbortHandle>,
    publication: &mut Option<Publication<H>>,
) -> Result<(), String> {
    let cert = s
        .network
        .host_certificate(key)
        .ok_or("no host certificate on the network")?;
    let (number, reader, writer) = s.host.open_vpn_port(net).await.map_err(err)?;
    let mut router = Router::new();
    let service = WhisperServer {
        shared: Rc::downgrade(s),
    };
    register_whisper_service(&mut router, Rc::new(service));
    let (client, c) =
        vpn::connect(s.config.vpn.clone(), &cert, reader, writer, Some(router)).map_err(err)?;
    let (c, abort) = abortable(c);
    *conn = Some(abort);
    let port = Rc::downgrade(p);
    s.spawn(
        async move {
            let res: Result<_, Aborted> = c.await;
            let Ok(res) = res else {
                return;
            };
            if let Err(e) = res {
                tracing::debug!(error = %e, "a whisper port's connection failed");
            }
            if let Some(p) = port.upgrade() {
                p.closed.set(true);
                p.wake();
            }
        }
        .boxed_local(),
    );
    let prev = p.client.replace(Some(client));
    drop(prev);
    s.metrics.borrow_mut().ports_opened += 1;
    loop {
        if p.stopped.get() {
            return Ok(());
        }
        if p.closed.get() {
            return Err("its connection ended".into());
        }
        let leased = s.holding();
        if leased && publication.is_none() {
            let profile_key = s.profile.key.clone().ok_or("the profile has no key")?;
            let salt = WHISPER_SALT.to_vec();
            let pb = s
                .host
                .publish_address(net, profile_key, salt, number)
                .await
                .map_err(err)?;
            *publication = Some(pb);
            p.published.set(true);
        } else if !leased && let Some(pb) = publication.take() {
            p.published.set(false);
            s.host.unpublish_address(pb).await.map_err(err)?;
        } else {
            p.wait().await;
        }
    }
}

// --- receiving --------------------------------------------------------------

/// Why a whisper wasn't taken.
enum Refusal {
    /// The caller's certificate isn't a host certificate under a
    /// membership, or a key, the alias or the server key is over its
    /// bound.
    Caller,
    Refused(WhisperRefusal),
    /// The sender is over its rate.
    TooFast,
    Store(strims_dao::Error),
}

impl Refusal {
    /// The answer: [`ChatError::TooFast`] for a sender over its rate,
    /// which may try again later, and [`WHISPER_REFUSED`] for the rest.
    fn to_rpc(&self) -> RpcError {
        match self {
            Refusal::TooFast => RpcError::new(0, ChatError::TooFast.to_string()),
            _ => RpcError::new(0, WHISPER_REFUSED),
        }
    }
}

impl From<strims_dao::Error> for Refusal {
    fn from(e: strims_dao::Error) -> Self {
        Refusal::Store(e)
    }
}

impl From<WhisperRefusal> for Refusal {
    fn from(r: WhisperRefusal) -> Self {
        Refusal::Refused(r)
    }
}

/// How a taken whisper was taken.
enum Received {
    Stored,
    /// Its (id, sender) is stored already.
    Duplicate,
}

/// `chat.v1.Whisper` on a whisper port. The sender is the profile of the
/// vpn certificate's membership.
struct WhisperServer<K: Kv, H: Host, T: Timer> {
    shared: Weak<Shared<K, H, T>>,
}

impl<K: Kv, H: Host, T: Timer> WhisperService for WhisperServer<K, H, T> {
    async fn send_message(
        &self,
        ctx: &Context,
        req: WhisperSendMessageRequest,
    ) -> Result<WhisperSendMessageResponse, RpcError> {
        let Some(s) = self.shared.upgrade() else {
            return Err(RpcError::new(0, WHISPER_REFUSED));
        };
        let res = receive(&s, ctx.certificate(), req).await;
        {
            let mut m = s.metrics.borrow_mut();
            match &res {
                Ok(Received::Stored) => m.received += 1,
                Ok(Received::Duplicate) => m.duplicates += 1,
                Err(Refusal::TooFast) => m.rate_limited += 1,
                Err(Refusal::Store(_)) => m.store_errors += 1,
                Err(Refusal::Caller | Refusal::Refused(_)) => m.refused += 1,
            }
        }
        match &res {
            Err(Refusal::Store(e)) => {
                tracing::warn!(error = %e, "storing a received whisper failed");
            }
            Err(Refusal::Refused(r)) => tracing::debug!(reason = ?r, "refused a whisper"),
            _ => {}
        }
        match res {
            Ok(_) => Ok(WhisperSendMessageResponse {}),
            Err(e) => Err(e.to_rpc()),
        }
    }
}

/// Takes a whisper from the caller presenting `cert`; see the
/// [module docs](super).
async fn receive<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    cert: Option<&Certificate>,
    req: WhisperSendMessageRequest,
) -> Result<Received, Refusal> {
    let cert = cert.ok_or(Refusal::Caller)?;
    let caller = Caller::from_certificate(cert).ok_or(Refusal::Caller)?;
    let network_key = CertificateChain::decode(cert)
        .map_err(|_| Refusal::Caller)?
        .network_key()
        .to_vec();
    if caller.peer_key.is_empty()
        || caller.peer_key.len() > MAX_PEER_KEY_BYTES
        || caller.alias.len() > MAX_ALIAS_BYTES
        || req.server_key.len() > MAX_PEER_KEY_BYTES
    {
        return Err(Refusal::Caller);
    }
    let now = s.timer.now();
    if !s.limiter.borrow_mut().try_take(now, &caller.peer_key) {
        return Err(Refusal::TooFast);
    }
    let limits = s.config.limits;
    if req.body.len() > limits.max_body_bytes {
        return Err(WhisperRefusal::TooLong.into());
    }
    let entities = s.entities(&req.body);
    let peer_key = caller.peer_key;

    let mut tx = s.store.write(now).await?;
    let probe = WhisperRecord {
        peer_message_id: req.id,
        peer_key: peer_key.clone(),
        ..Default::default()
    };
    let ix = &WhisperRecords::BY_PEER_MESSAGE_ID;
    if req.id != 0
        && !WhisperRecords::index_ids(&mut tx, ix, &peer_message_id_key(&probe))
            .await?
            .is_empty()
    {
        return Ok(Received::Duplicate);
    }
    let ix = &ChatUiConfigIgnores::BY_PEER_KEY;
    let ignore = ChatUiConfigIgnores::index_get(&mut tx, ix, &peer_key).await?;
    let ix = &WhisperRecords::UNREAD_BY_PEER_KEY;
    let unread = WhisperRecords::index_ids(&mut tx, ix, &peer_key)
        .await?
        .len();
    let ix = &WhisperThreads::BY_PEER_KEY;
    let thread = WhisperThreads::index_get(&mut tx, ix, &peer_key).await?;
    // The count reads every thread. Senders can't add threads past
    // `max_threads`, so how many there are is up to the profile.
    let threads = match thread {
        Some(_) => 0,
        None => WhisperThreads::get_all(&mut tx).await?.len(),
    };
    check_receive(
        now,
        &peer_key,
        &req.body,
        ignore.as_ref(),
        unread,
        thread.is_some(),
        threads,
        &limits,
    )?;

    let id = tx.generate_id().await?;
    let record = WhisperRecord {
        id,
        network_key,
        server_key: req.server_key,
        peer_key: peer_key.clone(),
        state: MessageState::Unread as i32,
        message: Some(Message {
            server_time: now.unix_millis(),
            peer_key: peer_key.clone(),
            nick: caller.alias.clone(),
            body: req.body,
            entities: Some(entities),
            ..Default::default()
        }),
        peer_message_id: req.id,
        ..Default::default()
    };
    WhisperRecords::insert(&mut tx, record).await?;
    let mut thread = match thread {
        Some(t) => t,
        None => WhisperThread {
            id: tx.generate_id().await?,
            peer_key,
            ..Default::default()
        },
    };
    thread_received(&mut thread, &caller.alias, id, now);
    WhisperThreads::upsert(&mut tx, thread).await?;
    tx.commit().await?;
    Ok(Received::Stored)
}

// --- delivering -------------------------------------------------------------

/// How a delivery attempt ended.
pub(crate) enum Outcome {
    Delivered,
    /// The recipient answered [`WHISPER_REFUSED`].
    Refused,
    /// Anything else that failed; the attempt is retried.
    Failed(String),
    /// The record was deleted or is no longer ENQUEUED.
    Stale,
}

/// Makes attempt `d`, started under lease change `generation`, and
/// stores the state it settles.
pub(crate) async fn deliver<K: Kv, H: Host, T: Timer>(
    s: Rc<Shared<K, H, T>>,
    generation: u64,
    d: Delivery,
) {
    let outcome = attempt(&s, &d).await;
    match &outcome {
        Outcome::Failed(e) => {
            tracing::debug!(whisper = d.id, attempt = d.attempt, error = %e, "delivering a whisper failed");
        }
        Outcome::Refused => tracing::debug!(whisper = d.id, "a whisper was refused"),
        Outcome::Delivered | Outcome::Stale => {}
    }
    if let Some(state) = s.finish(generation, d.id, &outcome) {
        s.write_state(d.id, state).await;
    }
}

async fn attempt<K: Kv, H: Host, T: Timer>(s: &Shared<K, H, T>, d: &Delivery) -> Outcome {
    let state = async {
        let mut tx = s.store.read().await?;
        let r = WhisperRecords::get(&mut tx, d.id).await?;
        Ok::<_, strims_dao::Error>(r.map(|r| r.state))
    }
    .await;
    match state {
        Ok(Some(st)) if st == MessageState::Enqueued as i32 => {}
        Ok(_) => return Outcome::Stale,
        Err(e) => {
            s.metrics.borrow_mut().store_errors += 1;
            return Outcome::Failed(e.to_string());
        }
    }
    let Some((net, client)) = s.port_client(&d.network_key) else {
        return Outcome::Failed("no whisper port on the network".into());
    };
    let deadline = s.timer.now() + s.config.attempt_deadline;
    let call = Box::pin(call(s, net, client, d));
    let timeout = Box::pin(s.timer.sleep_until(deadline));
    match select(call, timeout).await {
        Either::Left((outcome, _)) => outcome,
        Either::Right(_) => Outcome::Failed("the attempt timed out".into()),
    }
}

/// Resolves the recipient's whisper address and sends it the whisper.
async fn call<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    net: Net<H>,
    client: VpnClient,
    d: &Delivery,
) -> Outcome {
    let salt = WHISPER_SALT.to_vec();
    let addr = match s.host.resolve_address(net, d.peer_key.clone(), salt).await {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::Failed("the recipient's address isn't published".into()),
        Err(e) => {
            s.metrics.borrow_mut().host_errors += 1;
            return Outcome::Failed(e.to_string());
        }
    };
    let retry = RetryClient::new(client.to(addr), s.timer.clone(), s.config.call_policy);
    let req = WhisperSendMessageRequest {
        server_key: d.server_key.clone(),
        body: d.body.clone(),
        id: d.id,
    };
    match WhisperClient::new(retry).send_message(&req).await {
        Ok(_) => Outcome::Delivered,
        Err(CallError::Remote(e)) if e.message == WHISPER_REFUSED => Outcome::Refused,
        Err(e) => Outcome::Failed(e.to_string()),
    }
}
