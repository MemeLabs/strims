// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The PSI sessions that find the networks two peers share, and the
//! engines that run them.
//!
//! A run's session is `strims-mpc`'s PSZ, in the role the run picked. A
//! [`PsiEngine`] starts sessions, and the control drives each one through
//! [`PsiSession`] as broker bytes arrive from the peer and leave for it.
//! A session costs tens of milliseconds of CPU, and one call into it can
//! compute for a large part of that, so an engine decides where that
//! computing happens:
//!
//! - [`InlinePsi`], the default, computes inside the session's calls, on
//!   the control's thread. It is deterministic: the same seeds and inputs
//!   give the same bytes and outcome, and it never notifies its
//!   [`PsiReady`], so a node's runs replay exactly.
//! - [`RemotePsi`] runs sessions on a [`PsiServer`] elsewhere, such as a
//!   thread or a worker, over a [`PsiTransport`] the driver supplies, and
//!   multiplexes them by id. Its sessions answer asynchronously: what the
//!   server makes of one message, a step, arrives through
//!   [`RemotePsi::receive`], and the step's last message makes its bytes
//!   and its outcome visible together and notifies the session's
//!   [`PsiReady`]. The control then wakes the peer's broker task, which
//!   sends the bytes and takes the outcome as it takes an inline one.
//!
//! # Contract
//!
//! - A session reads the peer's bytes in order, split anywhere, and its
//!   bytes for the peer form one ordered stream.
//! - [`PsiSession::poll_outcome`] returns the outcome once. Bytes the
//!   session made before it stay available from
//!   [`PsiSession::poll_transmit`], and the peer needs them.
//! - The outcome is available no later than the session's last bytes:
//!   once `poll_transmit` has returned them, `poll_outcome` has it. A run
//!   moves on from its session before the peer, reading those bytes, can
//!   answer.
//! - [`PsiReady::notify`] may run inside a call into an engine or one of
//!   its sessions, so it only schedules work.
//! - Dropping a session ends it. A remote session that has no outcome yet
//!   tells the server to drop its side.
//!
//! # Privacy
//!
//! A remote engine's transport carries the sessions' keys and seeds, the
//! PSI's private inputs: it must stay inside the device.
//!
//! # Bounds
//!
//! The server and the client both decode every message under the codec's
//! bounds, and drop messages that are malformed or break one, and those
//! for sessions they don't hold:
//!
//! - a message holds at most [`MAX_PEER_SET_SIZE`] keys of at most 64
//!   bytes each, and at most 64 KiB of stream bytes; longer runs of
//!   stream bytes go as several messages;
//! - a failure's message holds at most 256 bytes;
//! - a server runs at most [`MAX_SERVER_SESSIONS`] sessions at once;
//! - a remote session holds at most [`MAX_PENDING_TRANSMIT`] bytes that
//!   the caller hasn't polled, and a shared outcome with no more keys than
//!   the session started with.
//!
//! A remote session that breaks a bound fails.
//!
//! [`MAX_PEER_SET_SIZE`]: strims_mpc::MAX_PEER_SET_SIZE

mod codec;
#[cfg(test)]
mod tests;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::rc::Rc;

use strims_mpc::{Error as MpcError, PszReceiver, PszSender};

/// The most sessions a [`PsiServer`] runs at once. A node's control runs
/// at most [`Config::max_sessions`](crate::Config::max_sessions), 4 by
/// default; a start beyond this bound fails.
pub const MAX_SERVER_SESSIONS: usize = 16;

/// The most bytes a [`RemotePsi`] session holds for the caller to poll.
/// A session's bytes between two of its peer's messages, even at
/// [`MAX_PEER_SET_SIZE`] keys, take a few MiB; a session that would hold
/// more fails.
///
/// [`MAX_PEER_SET_SIZE`]: strims_mpc::MAX_PEER_SET_SIZE
pub const MAX_PENDING_TRANSMIT: usize = 32 << 20;

/// A session's role. The side with more keys is the sender, and on a tie
/// the side with the greater host id; only the receiver learns the shared
/// keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PsiRole {
    Sender,
    Receiver,
}

/// How a session ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PsiOutcome {
    /// The sender finished.
    Sent,
    /// The receiver finished with the keys the sender also holds, in
    /// cuckoo-table order.
    Shared(Vec<Vec<u8>>),
    /// The receiver's cuckoo table failed: the run completed, learning
    /// nothing, and runs again with fresh seeds.
    CuckooFailed,
    /// The session failed, and why.
    Failed(String),
}

/// One PSI run. See the [module docs](self) for the contract.
pub trait PsiSession {
    /// Takes the peer's bytes. A failure shows as the outcome.
    fn handle_input(&mut self, data: &[u8]);

    /// Copies the next bytes for the peer into `buf` and returns how many,
    /// or `None` if there are none now.
    fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize>;

    /// The outcome, once the session has one. It returns `Some` once.
    fn poll_outcome(&mut self) -> Option<PsiOutcome>;
}

/// Starts PSI sessions.
pub trait PsiEngine {
    /// Starts a session in `role` over this side's `keys`. `seed` seeds
    /// the protocol's rng and must be secret and fresh per run. The
    /// session fails if the peer announces more than `max_peer_keys` keys
    /// (capped at [`MAX_PEER_SET_SIZE`]). It notifies `ready` when bytes or
    /// its outcome arrive outside a call into it.
    ///
    /// [`MAX_PEER_SET_SIZE`]: strims_mpc::MAX_PEER_SET_SIZE
    fn start(
        &self,
        role: PsiRole,
        keys: Vec<Vec<u8>>,
        seed: [u8; 16],
        max_peer_keys: usize,
        ready: PsiReady,
    ) -> Box<dyn PsiSession>;
}

/// The handle a session notifies when it has new bytes or its outcome.
/// Clones share the notification.
///
/// A notification may come from inside a call into the engine or one of
/// its sessions, so the function must not call into them, or into their
/// owner, before it returns: it only schedules work.
#[derive(Clone)]
pub struct PsiReady(Rc<dyn Fn()>);

impl PsiReady {
    pub fn new(notify: impl Fn() + 'static) -> Self {
        PsiReady(Rc::new(notify))
    }

    pub fn notify(&self) {
        (self.0)();
    }
}

impl fmt::Debug for PsiReady {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PsiReady")
    }
}

// --- inline -----------------------------------------------------------------

/// The engine that computes inside the sessions' calls. See the [module
/// docs](self).
#[derive(Clone, Copy, Debug, Default)]
pub struct InlinePsi;

impl PsiEngine for InlinePsi {
    fn start(
        &self,
        role: PsiRole,
        keys: Vec<Vec<u8>>,
        seed: [u8; 16],
        max_peer_keys: usize,
        _ready: PsiReady,
    ) -> Box<dyn PsiSession> {
        Box::new(InlineSession::new(role, keys, seed, max_peer_keys))
    }
}

enum Psz {
    Sender(PszSender),
    Receiver(PszReceiver),
}

/// A PSZ session, computing inside its calls.
struct InlineSession {
    psz: Psz,
    /// The outcome has been returned.
    done: bool,
}

impl InlineSession {
    fn new(role: PsiRole, keys: Vec<Vec<u8>>, seed: [u8; 16], max_peer_keys: usize) -> Self {
        let psz = match role {
            PsiRole::Sender => Psz::Sender(PszSender::with_peer_limit(keys, seed, max_peer_keys)),
            PsiRole::Receiver => {
                Psz::Receiver(PszReceiver::with_peer_limit(keys, seed, max_peer_keys))
            }
        };
        InlineSession { psz, done: false }
    }
}

impl PsiSession for InlineSession {
    fn handle_input(&mut self, data: &[u8]) {
        // an error is also the session's outcome
        let _ = match &mut self.psz {
            Psz::Sender(s) => s.handle_input(data),
            Psz::Receiver(r) => r.handle_input(data),
        };
    }

    fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize> {
        match &mut self.psz {
            Psz::Sender(s) => s.poll_transmit(buf),
            Psz::Receiver(r) => r.poll_transmit(buf),
        }
    }

    fn poll_outcome(&mut self) -> Option<PsiOutcome> {
        if self.done {
            return None;
        }
        let outcome = match &mut self.psz {
            Psz::Sender(s) => match s.result()? {
                Ok(()) => PsiOutcome::Sent,
                Err(e) => PsiOutcome::Failed(e.to_string()),
            },
            Psz::Receiver(r) => match r.take_result()? {
                Ok(keys) => PsiOutcome::Shared(keys),
                Err(MpcError::CuckooHashMapFull) => PsiOutcome::CuckooFailed,
                Err(e) => PsiOutcome::Failed(e.to_string()),
            },
        };
        self.done = true;
        Some(outcome)
    }
}

// --- remote -----------------------------------------------------------------

/// Carries a [`RemotePsi`]'s messages to its [`PsiServer`].
pub trait PsiTransport {
    /// Sends one message, whole and after the ones before it. It may
    /// deliver at once, even to [`RemotePsi::receive`]: the engine holds
    /// no borrow while it runs.
    fn send(&self, msg: Vec<u8>);
}

/// The engine that runs sessions on a [`PsiServer`] over a transport.
/// See the [module docs](self).
///
/// Clones share the engine and its sessions.
#[derive(Clone)]
pub struct RemotePsi(Rc<Remote>);

struct Remote {
    transport: Box<dyn PsiTransport>,
    /// The sessions, by id. Bound: the sessions the caller holds, each
    /// removed when dropped.
    sessions: RefCell<BTreeMap<u64, Slot>>,
    next_id: Cell<u64>,
}

/// A remote session's side of the engine.
struct Slot {
    ready: PsiReady,
    /// The keys the session started with: a shared outcome holds no more.
    keys: usize,
    /// Bytes for the peer, in order, and how far into the first chunk the
    /// caller has polled.
    pending: VecDeque<Vec<u8>>,
    offset: usize,
    /// The bytes of a step whose last message hasn't arrived.
    held: Vec<Vec<u8>>,
    /// Bytes in `pending` and `held`. Bound: `MAX_PENDING_TRANSMIT`.
    pending_len: usize,
    outcome: Option<PsiOutcome>,
    /// The outcome arrived, so the server holds nothing more for it.
    finished: bool,
}

impl Slot {
    /// Ends the session's step: its held bytes become visible.
    fn release(&mut self) {
        self.pending.extend(self.held.drain(..));
    }

    fn finish(&mut self, outcome: PsiOutcome) {
        self.release();
        self.outcome = Some(outcome);
        self.finished = true;
    }

    /// Takes bytes for the peer, held until the step's last message, or
    /// fails the session over its bound. Returns whether it took them.
    fn push(&mut self, data: &[u8], more: bool) -> bool {
        if self.pending_len.saturating_add(data.len()) > MAX_PENDING_TRANSMIT {
            self.pending.clear();
            self.held.clear();
            self.offset = 0;
            self.pending_len = 0;
            self.finish(PsiOutcome::Failed("psi engine output overflow".into()));
            return false;
        }
        self.pending_len += data.len();
        self.held.push(data.to_vec());
        if !more {
            self.release();
        }
        true
    }

    fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize> {
        let mut n = 0;
        while let Some(chunk) = self.pending.front() {
            let src = chunk.get(self.offset..).unwrap_or_default();
            let dst = buf.get_mut(n..).unwrap_or_default();
            let k = src.len().min(dst.len());
            if k == 0 && !src.is_empty() {
                break;
            }
            let (to, _) = dst.split_at_mut(k);
            to.copy_from_slice(src.get(..k).unwrap_or_default());
            n += k;
            self.offset += k;
            self.pending_len -= k;
            if self.offset == chunk.len() {
                self.pending.pop_front();
                self.offset = 0;
            }
        }
        (n > 0).then_some(n)
    }
}

impl fmt::Debug for RemotePsi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RemotePsi")
            .field(
                "sessions",
                &self.0.sessions.try_borrow().map(|s| s.len()).ok(),
            )
            .finish()
    }
}

impl RemotePsi {
    /// An engine whose messages go out on `transport`.
    pub fn new(transport: impl PsiTransport + 'static) -> Self {
        RemotePsi(Rc::new(Remote {
            transport: Box::new(transport),
            sessions: RefCell::new(BTreeMap::new()),
            next_id: Cell::new(0),
        }))
    }

    /// Takes a message from the server: a session's bytes or its outcome.
    /// The last message of a step makes the step visible and notifies the
    /// session's [`PsiReady`]. Malformed messages, and those for sessions
    /// that are gone or finished, are dropped.
    pub fn receive(&self, msg: &[u8]) {
        let Some(reply) = codec::decode_reply(msg) else {
            tracing::debug!(len = msg.len(), "malformed psi engine message dropped");
            return;
        };
        let (id, ready, close) = {
            let mut sessions = self.0.sessions.borrow_mut();
            let id = match &reply {
                codec::Reply::Transmit { id, .. } | codec::Reply::Outcome { id, .. } => *id,
            };
            let Some(slot) = sessions.get_mut(&id).filter(|s| !s.finished) else {
                return;
            };
            let close = match reply {
                codec::Reply::Transmit { more, data, .. } => {
                    let overflow = !slot.push(data, more);
                    if more && !overflow {
                        // the step goes on
                        return;
                    }
                    overflow
                }
                codec::Reply::Outcome { outcome, .. } => {
                    let outcome = match outcome {
                        PsiOutcome::Shared(keys) if keys.len() > slot.keys => PsiOutcome::Failed(
                            "psi engine shared more keys than the session holds".into(),
                        ),
                        o => o,
                    };
                    slot.finish(outcome);
                    false
                }
            };
            (id, slot.ready.clone(), close)
        };
        if close {
            self.0.transport.send(codec::encode_close(id));
        }
        ready.notify();
    }
}

impl PsiEngine for RemotePsi {
    fn start(
        &self,
        role: PsiRole,
        keys: Vec<Vec<u8>>,
        seed: [u8; 16],
        max_peer_keys: usize,
        ready: PsiReady,
    ) -> Box<dyn PsiSession> {
        let remote = &self.0;
        let id = remote.next_id.get();
        remote.next_id.set(id.wrapping_add(1));
        let msg = codec::encode_start(id, role, &seed, max_peer_keys, &keys);
        let mut slot = Slot {
            ready: ready.clone(),
            keys: keys.len(),
            pending: VecDeque::new(),
            offset: 0,
            held: Vec::new(),
            pending_len: 0,
            outcome: None,
            finished: false,
        };
        if msg.is_none() {
            slot.finish(PsiOutcome::Failed(
                "psi keys exceed the engine's bounds".into(),
            ));
        }
        // the slot is in place before the server can answer
        remote.sessions.borrow_mut().insert(id, slot);
        match msg {
            Some(msg) => remote.transport.send(msg),
            None => ready.notify(),
        }
        Box::new(RemoteSession {
            remote: remote.clone(),
            id,
        })
    }
}

struct RemoteSession {
    remote: Rc<Remote>,
    id: u64,
}

impl PsiSession for RemoteSession {
    fn handle_input(&mut self, data: &[u8]) {
        let live = self
            .remote
            .sessions
            .borrow()
            .get(&self.id)
            .is_some_and(|s| !s.finished);
        if !live {
            return;
        }
        for chunk in data.chunks(codec::MAX_DATA) {
            self.remote
                .transport
                .send(codec::encode_input(self.id, chunk));
        }
    }

    fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize> {
        self.remote
            .sessions
            .borrow_mut()
            .get_mut(&self.id)?
            .poll_transmit(buf)
    }

    fn poll_outcome(&mut self) -> Option<PsiOutcome> {
        self.remote
            .sessions
            .borrow_mut()
            .get_mut(&self.id)?
            .outcome
            .take()
    }
}

impl Drop for RemoteSession {
    fn drop(&mut self) {
        let slot = self.remote.sessions.borrow_mut().remove(&self.id);
        if slot.is_some_and(|s| !s.finished) {
            self.remote.transport.send(codec::encode_close(self.id));
        }
    }
}

// --- server -----------------------------------------------------------------

/// The engine's side of a [`RemotePsi`]: runs its sessions inline, as
/// [`InlinePsi`] does, from the messages it sends. Sans-IO: the driver
/// hands it each message and sends back what it returns, in order.
///
/// It runs at most [`MAX_SERVER_SESSIONS`] sessions, and forgets each
/// once it has sent the outcome.
pub struct PsiServer {
    /// Bound: `MAX_SERVER_SESSIONS`.
    sessions: BTreeMap<u64, InlineSession>,
    buf: Vec<u8>,
}

impl Default for PsiServer {
    fn default() -> Self {
        PsiServer::new()
    }
}

impl fmt::Debug for PsiServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PsiServer")
            .field("sessions", &self.sessions.len())
            .finish()
    }
}

impl PsiServer {
    pub fn new() -> Self {
        PsiServer {
            sessions: BTreeMap::new(),
            buf: vec![0; codec::MAX_DATA],
        }
    }

    /// Takes a message from a [`RemotePsi`] and returns the messages for
    /// it, in order. Malformed messages, and those for sessions it doesn't
    /// hold, are dropped; a start beyond [`MAX_SERVER_SESSIONS`] fails at
    /// once.
    pub fn handle(&mut self, msg: &[u8]) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let Some(request) = codec::decode_request(msg) else {
            tracing::debug!(len = msg.len(), "malformed psi engine message dropped");
            return out;
        };
        match request {
            codec::Request::Start {
                id,
                role,
                seed,
                max_peer_keys,
                keys,
            } => {
                if self.sessions.contains_key(&id) {
                    return out;
                }
                if self.sessions.len() >= MAX_SERVER_SESSIONS {
                    let busy = PsiOutcome::Failed("psi server busy".into());
                    out.push(codec::encode_outcome(id, &busy));
                    return out;
                }
                let session = InlineSession::new(role, keys, seed, max_peer_keys);
                self.sessions.insert(id, session);
                self.step(id, &mut out);
            }
            codec::Request::Input { id, data } => {
                if let Some(s) = self.sessions.get_mut(&id) {
                    s.handle_input(data);
                    self.step(id, &mut out);
                }
            }
            codec::Request::Close { id } => {
                self.sessions.remove(&id);
            }
        }
        out
    }

    /// Sends a session's step: its bytes, then its outcome if it has one,
    /// which ends the session. Each transmit but the step's last says more
    /// follows.
    fn step(&mut self, id: u64, out: &mut Vec<Vec<u8>>) {
        let Some(s) = self.sessions.get_mut(&id) else {
            return;
        };
        let mut data = Vec::new();
        while let Some(n) = s.poll_transmit(&mut self.buf) {
            data.push(self.buf.get(..n).unwrap_or_default().to_vec());
        }
        let outcome = s.poll_outcome();
        let last = data.len();
        for (i, d) in data.iter().enumerate() {
            let more = outcome.is_some() || i + 1 < last;
            out.push(codec::encode_transmit(id, more, d));
        }
        if let Some(outcome) = outcome {
            out.push(codec::encode_outcome(id, &outcome));
            self.sessions.remove(&id);
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.sessions.len()
    }
}
