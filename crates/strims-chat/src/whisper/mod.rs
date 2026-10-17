// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Whispers: direct messages between profiles.
//!
//! - **Receiving.** [`check_receive`] decides whether a profile takes a
//!   whisper. Every refusal answers the sender with [`WHISPER_REFUSED`], so
//!   the sender can't tell an ignore from a full inbox.
//! - **Sending.** A sent whisper is stored ENQUEUED. [`Deliveries`] is the
//!   sender's queue of those records: it schedules each attempt and
//!   retries, and says when a record becomes DELIVERED or FAILED.
//! - **Threads.** [`thread_received`] and [`thread_sent`] update a peer's
//!   thread for a whisper either way.
//!
//! # Running
//!
//! [`WhisperControl`] runs whispers for a profile on a node:
//!
//! - **Ports.** On each network the node has joined with a trusted
//!   membership, it opens a whisper port: a vpn port that serves
//!   `chat.v1.Whisper` and carries the profile's deliveries on that
//!   network. A port follows the profile's certificate on the network and
//!   closes when the node leaves it.
//! - **Lease.** Among the processes sharing the profile's store, the
//!   holder of the lease [`lease_key`] publishes the address of each of
//!   its ports under (profile key,
//!   [`WHISPER_SALT`](crate::WHISPER_SALT)), signed by the profile key,
//!   and delivers the profile's whispers. The others check again after
//!   every [`lease::recheck_delay`](strims_dao::lease::recheck_delay).
//! - **Receiving.** A `SendMessage` comes from the profile of the caller's
//!   membership certificate, on the network of the certificate's root.
//!   Calls over one sender's rate ([`WhisperConfig::rate`]) answer
//!   [`ChatError::TooFast`](crate::ChatError::TooFast). A whisper whose
//!   (`id`, sender) is stored already succeeds without a write. Otherwise
//!   [`check_receive`] decides, in the transaction that stores the whisper
//!   UNREAD and marks the sender's thread unread.
//! - **Sending.** [`WhisperControl::enqueue`] stores a whisper ENQUEUED and
//!   updates the recipient's thread. The lease holder queues every
//!   ENQUEUED record in [`Deliveries`]. An attempt resolves the
//!   recipient's address on the record's network and calls it with the
//!   record's id as the whisper's id, within
//!   [`WhisperConfig::attempt_deadline`]. Success makes the record
//!   DELIVERED, a [`WHISPER_REFUSED`] answer makes it FAILED, and any
//!   other failure retries on the [`Deliveries`] schedule. A record's
//!   attempts count from when the process queued it, and its age from its
//!   message's server time. A state is written only while the stored
//!   record is still ENQUEUED.
//!
//! A whisper's id is the same on every attempt, and a receiver stores one
//! whisper per (id, sender), so an attempt that landed but timed out
//! stores nothing when it is retried.

mod control;
mod service;

use std::collections::{BTreeMap, BTreeSet};

use strims_base::time::{Duration, Time};
use strims_proto::chat::v1::{MessageState, UiConfigIgnore, WhisperRecord, WhisperThread};

pub use control::{EnqueueError, WhisperConfig, WhisperControl, WhisperMetrics};

/// The key of the lease whose holder publishes the profile's whisper
/// addresses and delivers its whispers.
pub fn lease_key() -> String {
    strims_dao::lease::key(&["whisper"])
}

/// The error string a refused whisper answers with.
pub const WHISPER_REFUSED: &str = "unable to receive whisper";

/// Unread whispers from one peer, counting the one being received.
pub const MAX_UNREAD: usize = 15;

/// Whisper threads: a whisper from a peer without a thread is refused when
/// this many exist.
pub const MAX_WHISPER_THREADS: usize = 512;

/// Whisper body bytes.
pub const MAX_WHISPER_BODY_BYTES: usize = 2048;

/// Records a sender's [`Deliveries`] holds.
pub const MAX_PENDING_DELIVERIES: usize = 256;

/// Delivery attempts outstanding at once.
pub const MAX_IN_FLIGHT: usize = 8;

/// The wait before each retry: the first retry follows the first failed
/// attempt by 10 s, and the last delay repeats.
pub const RETRY_DELAYS: [Duration; 5] = [
    Duration::from_secs(10),
    Duration::from_secs(30),
    Duration::MINUTE,
    Duration::from_secs(120),
    Duration::from_secs(300),
];

/// Bounds of whisper receipt and delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WhisperLimits {
    /// Unread whispers from one peer, counting the one being received.
    pub max_unread: usize,
    /// Threads; a whisper from a peer without one is refused at this many.
    pub max_threads: usize,
    /// Body bytes of a received whisper.
    pub max_body_bytes: usize,
    /// Records awaiting delivery; [`Deliveries::enqueue`] refuses more.
    pub max_pending: usize,
    /// Delivery attempts outstanding at once.
    pub max_in_flight: usize,
    /// Attempts of one record; a failure of the last makes it FAILED.
    pub max_attempts: u32,
    /// How long after its enqueue a record is retried; a failure after
    /// this makes it FAILED, and no retry is scheduled later than this.
    pub max_age: Duration,
}

impl Default for WhisperLimits {
    fn default() -> Self {
        WhisperLimits {
            max_unread: MAX_UNREAD,
            max_threads: MAX_WHISPER_THREADS,
            max_body_bytes: MAX_WHISPER_BODY_BYTES,
            max_pending: MAX_PENDING_DELIVERIES,
            max_in_flight: MAX_IN_FLIGHT,
            max_attempts: 6,
            max_age: Duration::from_secs(600),
        }
    }
}

/// Why [`check_receive`] refused a whisper. Each displays as
/// [`WHISPER_REFUSED`], the string the sender gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WhisperRefusal {
    /// The sender is ignored.
    #[error("unable to receive whisper")]
    Ignored,
    /// The body is over [`WhisperLimits::max_body_bytes`].
    #[error("unable to receive whisper")]
    TooLong,
    /// The whisper would make more than [`WhisperLimits::max_unread`]
    /// unread whispers from the sender.
    #[error("unable to receive whisper")]
    TooManyUnread,
    /// The sender has no thread, and [`WhisperLimits::max_threads`] exist.
    #[error("unable to receive whisper")]
    TooManyThreads,
}

/// Whether a profile takes a whisper from `sender_peer_key`, before it
/// stores anything.
///
/// - `ignore` is the profile's ignore record for the sender, if any. It
///   applies while its `deadline`, in Unix seconds, is 0 (forever) or
///   later than `now`.
/// - `unread_count` is the stored UNREAD whispers from the sender; the
///   new one counts on top.
/// - `thread_exists` says whether the sender has a thread, and
///   `thread_count` how many threads exist.
///
/// Check for a duplicate first: a whisper whose `(peer_message_id,
/// peer_key)` is already stored succeeds without a write, so a delivery
/// retried after it landed isn't refused for the unread it added.
#[allow(clippy::too_many_arguments)]
pub fn check_receive(
    now: Time,
    sender_peer_key: &[u8],
    body: &str,
    ignore: Option<&UiConfigIgnore>,
    unread_count: usize,
    thread_exists: bool,
    thread_count: usize,
    limits: &WhisperLimits,
) -> Result<(), WhisperRefusal> {
    if let Some(ig) = ignore
        && ig.peer_key == sender_peer_key
        && (ig.deadline == 0 || ig.deadline > now.unix())
    {
        return Err(WhisperRefusal::Ignored);
    }
    if body.len() > limits.max_body_bytes {
        return Err(WhisperRefusal::TooLong);
    }
    if unread_count.saturating_add(1) > limits.max_unread {
        return Err(WhisperRefusal::TooManyUnread);
    }
    if !thread_exists && thread_count >= limits.max_threads {
        return Err(WhisperRefusal::TooManyThreads);
    }
    Ok(())
}

/// Updates the sender's thread for a whisper received from it: `alias` is
/// the sender's certificate subject and `record_id` the stored record's id.
pub fn thread_received(thread: &mut WhisperThread, alias: &str, record_id: u64, now: Time) {
    set_last(thread, alias, record_id, now);
    thread.has_unread = true;
}

/// Updates the recipient's thread for a whisper sent to it: `alias` is the
/// recipient's certificate subject and `record_id` the stored record's id.
pub fn thread_sent(thread: &mut WhisperThread, alias: &str, record_id: u64, now: Time) {
    set_last(thread, alias, record_id, now);
}

/// `last_message_time` is in Unix milliseconds.
fn set_last(thread: &mut WhisperThread, alias: &str, record_id: u64, now: Time) {
    alias.clone_into(&mut thread.alias);
    thread.last_message_time = now.unix_millis();
    thread.last_message_id = record_id;
}

/// One delivery attempt: send `body` to `peer_key` on `network_key` as
/// whisper `id`, then report the result with [`Deliveries::finished`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delivery {
    pub id: u64,
    pub network_key: Vec<u8>,
    pub server_key: Vec<u8>,
    pub peer_key: Vec<u8>,
    pub body: String,
    /// The attempt's number, from 1.
    pub attempt: u32,
}

#[derive(Clone, Debug)]
struct Pending {
    network_key: Vec<u8>,
    server_key: Vec<u8>,
    peer_key: Vec<u8>,
    body: String,
    enqueued: Time,
    attempts: u32,
    /// When the next attempt is due; `None` while one is in flight.
    due: Option<Time>,
    /// Cancelled while in flight: forgotten when the attempt finishes.
    cancelled: bool,
}

/// A sender's ENQUEUED whispers and their delivery schedule.
///
/// Records are attempted in order of their due time, at most
/// [`WhisperLimits::max_in_flight`] at once. A failed attempt is retried
/// after the next of [`RETRY_DELAYS`], but no later than
/// [`WhisperLimits::max_age`] after the enqueue. A record is DELIVERED
/// when an attempt succeeds, and FAILED when its last attempt
/// ([`WhisperLimits::max_attempts`]) fails or an attempt fails after
/// `max_age`. The caller stores that state only while the stored record is
/// still ENQUEUED.
#[derive(Clone, Debug)]
pub struct Deliveries {
    limits: WhisperLimits,
    /// Records by id. Bound: `limits.max_pending`.
    pending: BTreeMap<u64, Pending>,
    /// The records waiting for an attempt, by due time. Bound: as
    /// `pending`.
    queue: BTreeSet<(Time, u64)>,
    /// Attempts outstanding, those of cancelled records included.
    in_flight: usize,
}

impl Deliveries {
    pub fn new(limits: WhisperLimits) -> Self {
        Deliveries {
            limits,
            pending: BTreeMap::new(),
            queue: BTreeSet::new(),
            in_flight: 0,
        }
    }

    /// Queues an ENQUEUED record for its first attempt at `now`. Returns
    /// whether the record is queued: false when it isn't ENQUEUED, has no
    /// message, or the queue is full. A record already queued keeps its
    /// schedule.
    pub fn enqueue(&mut self, now: Time, record: &WhisperRecord) -> bool {
        if record.state != MessageState::Enqueued as i32 {
            return false;
        }
        let Some(message) = &record.message else {
            return false;
        };
        if let Some(p) = self.pending.get_mut(&record.id) {
            p.cancelled = false;
            return true;
        }
        if self.pending.len() >= self.limits.max_pending {
            return false;
        }
        self.pending.insert(
            record.id,
            Pending {
                network_key: record.network_key.clone(),
                server_key: record.server_key.clone(),
                peer_key: record.peer_key.clone(),
                body: message.body.clone(),
                enqueued: now,
                attempts: 0,
                due: Some(now),
                cancelled: false,
            },
        );
        self.queue.insert((now, record.id));
        true
    }

    /// The next attempt due by `now`, if fewer than
    /// [`WhisperLimits::max_in_flight`] are outstanding.
    pub fn poll_send(&mut self, now: Time) -> Option<Delivery> {
        if self.in_flight >= self.limits.max_in_flight {
            return None;
        }
        let &(due, id) = self.queue.first()?;
        if due > now {
            return None;
        }
        self.queue.remove(&(due, id));
        let p = self.pending.get_mut(&id)?;
        p.due = None;
        p.attempts = p.attempts.saturating_add(1);
        self.in_flight += 1;
        Some(Delivery {
            id,
            network_key: p.network_key.clone(),
            server_key: p.server_key.clone(),
            peer_key: p.peer_key.clone(),
            body: p.body.clone(),
            attempt: p.attempts,
        })
    }

    /// Reports the attempt at record `id` that [`poll_send`](Self::poll_send)
    /// returned. Returns the state to store: DELIVERED, FAILED, or `None`
    /// when a retry is scheduled or the record was cancelled or isn't in
    /// flight.
    pub fn finished(&mut self, now: Time, id: u64, ok: bool) -> Option<MessageState> {
        let p = self.pending.get_mut(&id)?;
        if p.due.is_some() {
            return None;
        }
        self.in_flight = self.in_flight.saturating_sub(1);
        if p.cancelled {
            self.pending.remove(&id);
            return None;
        }
        if ok {
            self.pending.remove(&id);
            return Some(MessageState::Delivered);
        }
        let deadline = p.enqueued + self.limits.max_age;
        if p.attempts >= self.limits.max_attempts || now >= deadline {
            self.pending.remove(&id);
            return Some(MessageState::Failed);
        }
        let i = usize::try_from(p.attempts.saturating_sub(1)).unwrap_or(usize::MAX);
        let delay = RETRY_DELAYS
            .get(i)
            .or(RETRY_DELAYS.last())
            .copied()
            .unwrap_or(Duration::ZERO);
        let due = (now + delay).min(deadline);
        p.due = Some(due);
        self.queue.insert((due, id));
        None
    }

    /// Reports that the recipient refused the attempt at record `id`, so
    /// it isn't retried. Returns FAILED, or `None` when the record was
    /// cancelled or isn't in flight.
    pub fn refused(&mut self, id: u64) -> Option<MessageState> {
        if self.pending.get(&id)?.due.is_some() {
            return None;
        }
        self.in_flight = self.in_flight.saturating_sub(1);
        let p = self.pending.remove(&id)?;
        (!p.cancelled).then_some(MessageState::Failed)
    }

    /// Forgets record `id`: it was deleted or is no longer ENQUEUED. An
    /// attempt in flight keeps its slot until it finishes.
    pub fn cancel(&mut self, id: u64) {
        let Some(p) = self.pending.get_mut(&id) else {
            return;
        };
        match p.due {
            Some(due) => {
                self.queue.remove(&(due, id));
                self.pending.remove(&id);
            }
            None => p.cancelled = true,
        }
    }

    /// When the next attempt is due; `None` when none waits or
    /// [`WhisperLimits::max_in_flight`] attempts are outstanding, since
    /// only [`finished`](Self::finished) frees a slot then.
    pub fn poll_timeout(&self) -> Option<Time> {
        if self.in_flight >= self.limits.max_in_flight {
            return None;
        }
        self.queue.first().map(|&(due, _)| due)
    }

    /// Records queued, in flight or waiting.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Attempts outstanding.
    pub fn in_flight(&self) -> usize {
        self.in_flight
    }
}

#[cfg(test)]
mod tests;
