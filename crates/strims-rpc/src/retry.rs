// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Calls with deadlines and retries.
//!
//! A [`RetryClient`] makes each unary call through a [`Client`] in
//! attempts timed on the driver's [`Timer`]. An attempt that gets no
//! answer within [`RetryPolicy::attempt_timeout`] is cancelled, and the
//! request goes out again after [`RetryPolicy::delay`], up to
//! [`RetryPolicy::retries`] times; each later wait is
//! [`RetryPolicy::backoff`] times the one before. Only an attempt's
//! deadline is retried: an answer, an error answer included, or a local
//! failure ends the call. When the attempts or [`RetryPolicy::deadline`]
//! run out, the call fails with [`CallError::Timeout`]. Dropping the
//! call's future cancels the attempt in flight.
//!
//! # Progress
//!
//! An attempt's deadline moves forward while the attempt is progressing:
//!
//! - its request hasn't left the connection, because the transport isn't
//!   taking more; or
//! - part of a Call has arrived, the rest hasn't, and more of it arrived
//!   since the deadline was set.
//!
//! A stream transport delivers Calls in order, so a Call partway in is the
//! answer or one ahead of it, and a repeated request would only queue
//! behind both. A message transport (a vpn port) delivers every Call
//! whole, so its attempts end on time. [`RetryPolicy::deadline`] bounds a
//! call however long it keeps progressing.
//!
//! Server streams have no deadline: their calls start as [`Client`]'s do.

use std::fmt;
use std::future::{Future, poll_fn};
use std::pin::pin;
use std::task::Poll;

use prost::Message;
use strims_base::time::{Duration, Time};
use strims_proto::service::{CallError, Caller, StreamingMethod, UnaryMethod};

use crate::client::{Call, Client, ClientStream};
use crate::wire::Encoded;

/// The driver's clock, which [`RetryClient`] times its calls on.
pub trait Timer: Clone + 'static {
    /// A wait for a deadline.
    type Sleep: Future<Output = ()>;

    /// The current time.
    fn now(&self) -> Time;

    /// Resolves once [`Timer::now`] has reached `deadline`.
    fn sleep_until(&self, deadline: Time) -> Self::Sleep;
}

/// When a [`RetryClient`] gives up on an attempt, and on a call; see the
/// [module docs](self).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    /// How long an attempt waits for its answer without progress.
    pub attempt_timeout: Duration,
    /// Attempts after the first.
    pub retries: u32,
    /// The wait before the first retry.
    pub delay: Duration,
    /// What each later wait is multiplied by.
    pub backoff: u32,
    /// The longest a call takes, from its first attempt to its last.
    pub deadline: Duration,
}

impl RetryPolicy {
    /// The defaults for calls over a peer's channel pair. The deadline
    /// leaves room for a maximum-size answer (512 KiB) arriving at about
    /// 140 kbit/s.
    pub fn peer() -> RetryPolicy {
        RetryPolicy {
            attempt_timeout: Duration::SECOND,
            retries: 3,
            delay: Duration::from_millis(100),
            backoff: 2,
            deadline: Duration::from_secs(30),
        }
    }

    /// The defaults for calls over a vpn port, whose messages cross relays
    /// and can be lost.
    pub fn vpn() -> RetryPolicy {
        RetryPolicy {
            attempt_timeout: Duration::from_secs(10),
            retries: 3,
            delay: Duration::SECOND,
            backoff: 2,
            deadline: Duration::from_secs(60),
        }
    }
}

/// Makes calls through a [`Client`] with deadlines and retries; see the
/// [module docs](self). Clones share the client.
#[derive(Clone)]
pub struct RetryClient<T> {
    client: Client,
    timer: T,
    policy: RetryPolicy,
}

impl<T> fmt::Debug for RetryClient<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RetryClient")
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

impl<T: Timer> RetryClient<T> {
    /// Calls through `client`, timed on `timer`.
    pub fn new(client: Client, timer: T, policy: RetryPolicy) -> RetryClient<T> {
        RetryClient {
            client,
            timer,
            policy,
        }
    }

    /// The client that the calls go through.
    pub fn client(&self) -> &Client {
        &self.client
    }
}

impl<T: Timer> Caller for RetryClient<T> {
    type Stream<R: Message + Default + 'static> = ClientStream<R>;

    fn call_unary<M: UnaryMethod>(
        &self,
        req: &M::Request,
    ) -> impl Future<Output = Result<M::Response, CallError>> + use<T, M> {
        let client = self.client.clone();
        let timer = self.timer.clone();
        let policy = self.policy;
        let req = req.encode_to_vec();
        async move { call_unary(&client, &timer, policy, M::NAME, &req).await }
    }

    fn call_streaming<M: StreamingMethod>(
        &self,
        req: &M::Request,
    ) -> impl Future<Output = Result<Self::Stream<M::Response>, CallError>> + use<T, M> {
        self.client.call_streaming::<M>(req)
    }
}

/// Calls `method` with the encoded `req` in attempts until one is
/// answered or `policy` runs out.
async fn call_unary<R: Message + Default>(
    client: &Client,
    timer: &impl Timer,
    policy: RetryPolicy,
    method: &str,
    req: &[u8],
) -> Result<R, CallError> {
    let deadline = timer.now() + policy.deadline;
    let mut delay = policy.delay;
    let mut retries = policy.retries;
    loop {
        let mut call = client.start(method, false, &Encoded(req))?;
        if let Some(res) = attempt(&mut call, timer, policy.attempt_timeout, deadline).await {
            return res;
        }
        // cancels the attempt
        drop(call);
        let now = timer.now();
        if retries == 0 || !now.before(deadline) {
            return Err(CallError::Timeout);
        }
        retries -= 1;
        timer.sleep_until((now + delay).min(deadline)).await;
        if !timer.now().before(deadline) {
            return Err(CallError::Timeout);
        }
        delay = delay * i64::from(policy.backoff);
    }
}

/// The result of `call`, or `None` once it goes `timeout` without
/// progress or reaches `deadline`.
async fn attempt<R: Message + Default>(
    call: &mut Call,
    timer: &impl Timer,
    timeout: Duration,
    deadline: Time,
) -> Option<Result<R, CallError>> {
    let mut mark = call.progress();
    loop {
        let mut sleep = pin!(timer.sleep_until((timer.now() + timeout).min(deadline)));
        let res = poll_fn(|cx| match call.poll_unary(cx) {
            Poll::Ready(res) => Poll::Ready(Some(res)),
            Poll::Pending => sleep.as_mut().poll(cx).map(|()| None),
        })
        .await;
        if res.is_some() {
            return res;
        }
        let progress = call.progress();
        let arriving = progress.receiving && progress.received > mark.received;
        if !(progress.queued || arriving) || !timer.now().before(deadline) {
            return None;
        }
        mark = progress;
    }
}
