// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Local queues between the control's tasks.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::poll_fn;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use strims_proto::replication::v1::EventLog;

/// What to send a peer next.
#[derive(Clone, Debug, PartialEq)]
pub enum Push {
    /// New logs.
    Logs(Vec<EventLog>),
    /// Logs were dropped: send a delta from the peer's checkpoint.
    Resync,
}

#[derive(Default)]
struct PushState {
    logs: VecDeque<EventLog>,
    resync: bool,
    waker: Option<Waker>,
}

/// The logs waiting for one peer, at most `capacity` of them. A full
/// queue drops its logs and asks for a resync, so a slow peer costs
/// bounded memory.
#[derive(Clone)]
pub struct PushQueue {
    state: Rc<RefCell<PushState>>,
    capacity: usize,
}

impl PushQueue {
    /// An empty queue.
    pub fn new(capacity: usize) -> PushQueue {
        PushQueue {
            state: Rc::default(),
            capacity,
        }
    }

    /// Queues a log.
    pub fn push(&self, log: EventLog) {
        let waker = {
            let mut s = self.state.borrow_mut();
            if s.logs.len() >= self.capacity {
                s.logs.clear();
                s.resync = true;
            } else if !s.resync {
                s.logs.push_back(log);
            }
            s.waker.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
    }

    /// Drops the queued logs and asks for a resync.
    pub fn resync(&self) {
        let waker = {
            let mut s = self.state.borrow_mut();
            s.logs.clear();
            s.resync = true;
            s.waker.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
    }

    fn poll_next(&self, cx: &mut Context<'_>) -> Poll<Push> {
        let mut s = self.state.borrow_mut();
        if s.resync {
            s.resync = false;
            return Poll::Ready(Push::Resync);
        }
        if !s.logs.is_empty() {
            return Poll::Ready(Push::Logs(s.logs.drain(..).collect()));
        }
        s.waker = Some(cx.waker().clone());
        Poll::Pending
    }

    /// The next thing to send.
    pub async fn next(&self) -> Push {
        poll_fn(|cx| self.poll_next(cx)).await
    }
}

/// An unbounded local channel with one receiver. Its senders are the
/// control's own methods, called by the driver.
pub(crate) struct Mailbox<T> {
    state: Rc<RefCell<(VecDeque<T>, Option<Waker>)>>,
}

impl<T> Clone for Mailbox<T> {
    fn clone(&self) -> Self {
        Mailbox {
            state: self.state.clone(),
        }
    }
}

impl<T> Mailbox<T> {
    pub(crate) fn new() -> Mailbox<T> {
        Mailbox {
            state: Rc::new(RefCell::new((VecDeque::new(), None))),
        }
    }

    pub(crate) fn send(&self, t: T) {
        let waker = {
            let mut s = self.state.borrow_mut();
            s.0.push_back(t);
            s.1.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
    }

    pub(crate) fn poll_recv(&self, cx: &mut Context<'_>) -> Poll<T> {
        let mut s = self.state.borrow_mut();
        match s.0.pop_front() {
            Some(t) => Poll::Ready(t),
            None => {
                s.1 = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use futures_util::FutureExt;

    use super::*;

    fn log(id: u64) -> EventLog {
        EventLog {
            id,
            ..Default::default()
        }
    }

    #[test]
    fn a_full_queue_asks_for_a_resync() {
        let q = PushQueue::new(2);
        q.push(log(1));
        q.push(log(2));
        assert_eq!(
            q.next().now_or_never(),
            Some(Push::Logs(vec![log(1), log(2)]))
        );
        assert_eq!(q.next().now_or_never(), None);
        q.push(log(3));
        q.push(log(4));
        q.push(log(5));
        q.push(log(6));
        assert_eq!(q.next().now_or_never(), Some(Push::Resync));
        assert_eq!(q.next().now_or_never(), None);
    }

    #[test]
    fn mailboxes_deliver_in_order() {
        let m = Mailbox::new();
        m.send(1);
        m.send(2);
        let waker = futures_util::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert_eq!(m.poll_recv(&mut cx), Poll::Ready(1));
        assert_eq!(m.poll_recv(&mut cx), Poll::Ready(2));
        assert!(m.poll_recv(&mut cx).is_pending());
    }
}
