// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The state a connection shares with its client handles and handlers.
//!
//! One `Rc<RefCell<Shared>>` per connection. A borrow never spans an
//! `.await`, a wake, or a call into user code (handlers, router closures,
//! drops of user values): wakers are taken out under the borrow and woken
//! after it ends.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::future::poll_fn;
use std::rc::Rc;
use std::task::{Poll, Waker};

use prost::Message;
use strims_proto::service::Error;

use crate::endpoint::{CallId, Endpoint, InboundId};

pub(crate) type SharedRef = Rc<RefCell<Shared>>;

#[derive(Debug)]
pub(crate) struct Shared {
    pub(crate) endpoint: Endpoint,
    /// Calls waiting for a response. One entry per pending call future or
    /// stream, so bounded by the endpoint's open calls.
    pub(crate) call_wakers: BTreeMap<CallId, Waker>,
    /// The connection's task, woken when there's something to send.
    pub(crate) driver: Option<Waker>,
    /// Server streams waiting for the transmit queue to drain. One entry
    /// per request in service at most.
    pub(crate) congested: Vec<Waker>,
}

impl Shared {
    pub(crate) fn new(endpoint: Endpoint) -> SharedRef {
        Rc::new(RefCell::new(Shared {
            endpoint,
            call_wakers: BTreeMap::new(),
            driver: None,
            congested: Vec::new(),
        }))
    }
}

/// Runs `f` on the shared state, then wakes the connection's task so it
/// sends what `f` queued.
pub(crate) fn with_endpoint<T>(shared: &SharedRef, f: impl FnOnce(&mut Endpoint) -> T) -> T {
    let (t, driver) = {
        let mut s = shared.borrow_mut();
        let t = f(&mut s.endpoint);
        (t, s.driver.clone())
    };
    if let Some(w) = driver {
        w.wake();
    }
    t
}

/// A handler's channel back to its caller.
pub(crate) struct Responder {
    shared: SharedRef,
    call: InboundId,
}

impl Responder {
    pub(crate) fn new(shared: SharedRef, call: InboundId) -> Responder {
        Responder { shared, call }
    }

    pub(crate) fn reply_value(&self, value: &impl Message) {
        with_endpoint(&self.shared, |e| e.reply_value(self.call, value));
    }

    pub(crate) fn reply_error(&self, error: &Error) {
        with_endpoint(&self.shared, |e| e.reply_error(self.call, error));
    }

    /// Sends a stream item; false once the call is over.
    pub(crate) fn reply_item(&self, item: &impl Message) -> bool {
        with_endpoint(&self.shared, |e| e.reply_item(self.call, item))
    }

    pub(crate) fn reply_close(&self) {
        with_endpoint(&self.shared, |e| e.reply_close(self.call));
    }

    /// Waits until the endpoint can take more to send.
    pub(crate) async fn ready(&self) {
        poll_fn(|cx| {
            let mut s = self.shared.borrow_mut();
            if s.endpoint.is_congested() {
                s.congested.push(cx.waker().clone());
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
        .await;
    }
}

/// Returns `Pending` once, waking its task, so other work runs first.
pub(crate) async fn yield_now() {
    let mut yielded = false;
    poll_fn(|cx| {
        if yielded {
            Poll::Ready(())
        } else {
            yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
}
