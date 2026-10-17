// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! In-memory pipes: unbounded, in-order byte message queues.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use futures_sink::Sink;

/// The error of every fake channel: it is closed.
#[derive(Debug)]
pub struct Closed;

impl std::fmt::Display for Closed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("closed")
    }
}

#[derive(Default)]
pub struct PipeState {
    queue: VecDeque<Vec<u8>>,
    reader: Option<Waker>,
    closed: bool,
}

/// One direction of a link. Its [`Reader`] ends after the queued messages
/// once it is closed; its [`Writer`] fails from then on.
#[derive(Clone, Default)]
pub struct Pipe(pub Rc<RefCell<PipeState>>);

impl Pipe {
    pub fn close(&self) {
        let w = {
            let mut s = self.0.borrow_mut();
            s.closed = true;
            s.reader.take()
        };
        if let Some(w) = w {
            w.wake();
        }
    }
}

pub struct Reader(pub Pipe);
pub struct Writer(pub Pipe);

impl Stream for Reader {
    type Item = Result<Vec<u8>, Closed>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut s = self.0.0.borrow_mut();
        if let Some(b) = s.queue.pop_front() {
            return Poll::Ready(Some(Ok(b)));
        }
        if s.closed {
            return Poll::Ready(None);
        }
        s.reader = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Sink<Vec<u8>> for Writer {
    type Error = Closed;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        if self.0.0.borrow().closed {
            return Poll::Ready(Err(Closed));
        }
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Closed> {
        let w = {
            let mut s = self.0.0.borrow_mut();
            s.queue.push_back(item);
            s.reader.take()
        };
        if let Some(w) = w {
            w.wake();
        }
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        self.0.close();
        Poll::Ready(Ok(()))
    }
}
