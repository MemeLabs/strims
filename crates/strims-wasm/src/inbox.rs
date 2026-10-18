// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A link's read queue: what its socket's JS callbacks deliver, until the
//! link's reader task copies it into the node's read buffers.
//!
//! JS sockets can't be paused: a WebSocket or data channel dispatches
//! every message as it arrives, whatever the reader is doing. The inbox
//! keeps the messages as JS buffers, outside wasm memory (which never
//! shrinks), up to [`INBOX_CAP`] bytes. A message past that fails the link
//! with "read queue overflow" and drops what was queued.
//!
//! The inbox also records the socket's state: whether it opened, and how
//! it ended. The reader reports the end after every byte queued before it.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::{Future, poll_fn};
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use js_sys::Uint8Array;

/// The most bytes a link's inbox holds.
pub const INBOX_CAP: usize = 4 * 1024 * 1024;

/// One socket's queue and state; clones share it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Inbox(Rc<RefCell<State>>);

#[derive(Debug, Default)]
struct State {
    /// At most `INBOX_CAP` bytes.
    queue: VecDeque<Uint8Array>,
    /// Bytes in `queue` not yet read.
    bytes: usize,
    /// Bytes of the front message already read.
    offset: usize,
    open: bool,
    end: Option<String>,
    /// The one task waiting on the inbox: the dial before the link, the
    /// reader after.
    waker: Option<Waker>,
}

impl Inbox {
    pub(crate) fn new() -> Inbox {
        Inbox::default()
    }

    /// Queues a message. Ignored once the socket has ended.
    pub(crate) fn push(&self, data: Uint8Array) {
        let len = data.length() as usize;
        let waker = {
            let mut s = self.0.borrow_mut();
            if s.end.is_some() || len == 0 {
                return;
            }
            if s.bytes.saturating_add(len) > INBOX_CAP {
                s.queue.clear();
                s.bytes = 0;
                s.offset = 0;
                s.end = Some("read queue overflow".to_owned());
            } else {
                s.queue.push_back(data);
                s.bytes += len;
            }
            s.waker.take()
        };
        wake(waker);
    }

    /// The socket opened.
    pub(crate) fn set_open(&self) {
        let waker = {
            let mut s = self.0.borrow_mut();
            s.open = true;
            s.waker.take()
        };
        wake(waker);
    }

    /// The socket ended, and why. Only the first end counts.
    pub(crate) fn end(&self, reason: &str) {
        let waker = {
            let mut s = self.0.borrow_mut();
            if s.end.is_some() {
                return;
            }
            s.end = Some(reason.to_owned());
            s.waker.take()
        };
        wake(waker);
    }

    /// Why the socket ended, if it has.
    pub(crate) fn ended(&self) -> Option<String> {
        self.0.borrow().end.clone()
    }

    /// Resolves once the socket opens, or with the reason it ended first.
    pub(crate) fn opened(&self) -> impl Future<Output = Result<(), String>> + '_ {
        poll_fn(|cx| {
            let mut s = self.0.borrow_mut();
            if let Some(e) = &s.end {
                return Poll::Ready(Err(e.clone()));
            }
            if s.open {
                return Poll::Ready(Ok(()));
            }
            register(&mut s, cx);
            Poll::Pending
        })
    }

    /// Copies queued bytes into `buf`, as many as fit, across messages.
    /// Resolves with the count once there are any, or with the reason the
    /// socket ended once nothing queued before the end is left.
    pub(crate) fn read<'a>(
        &'a self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = Result<usize, String>> + 'a {
        poll_fn(move |cx| self.poll_read(cx, buf))
    }

    fn poll_read(&self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize, String>> {
        let mut s = self.0.borrow_mut();
        let mut n = 0;
        while n < buf.len() {
            let Some(front) = s.queue.front() else {
                break;
            };
            let len = front.length() as usize;
            let take = (len - s.offset).min(buf.len() - n);
            let Some(dst) = buf.get_mut(n..n + take) else {
                break;
            };
            if s.offset == 0 && take == len {
                front.copy_to(dst);
            } else {
                // Lengths are below `INBOX_CAP`, so they fit a u32.
                let start = s.offset as u32;
                front.subarray(start, start + take as u32).copy_to(dst);
            }
            n += take;
            s.bytes -= take;
            s.offset += take;
            if s.offset == len {
                s.queue.pop_front();
                s.offset = 0;
            }
        }
        if n > 0 {
            return Poll::Ready(Ok(n));
        }
        if let Some(e) = &s.end {
            return Poll::Ready(Err(e.clone()));
        }
        register(&mut s, cx);
        Poll::Pending
    }
}

fn register(s: &mut State, cx: &mut Context<'_>) {
    if !s.waker.as_ref().is_some_and(|w| w.will_wake(cx.waker())) {
        let old = s.waker.replace(cx.waker().clone());
        drop(old);
    }
}

fn wake(waker: Option<Waker>) {
    if let Some(w) = waker {
        w.wake();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use futures_util::FutureExt;
    use wasm_bindgen_test::wasm_bindgen_test;

    use super::*;

    fn bytes(data: &[u8]) -> Uint8Array {
        Uint8Array::from(data)
    }

    fn read_now(inbox: &Inbox, n: usize) -> Option<Result<Vec<u8>, String>> {
        let mut buf = vec![0; n];
        let r = inbox.read(&mut buf).now_or_never()?;
        Some(r.map(|len| buf[..len].to_vec()))
    }

    /// Reads fill the buffer across messages and split a message that
    /// doesn't fit.
    #[wasm_bindgen_test]
    fn reads_join_and_split_messages() {
        let inbox = Inbox::new();
        assert_eq!(read_now(&inbox, 4), None);
        inbox.push(bytes(&[1, 2, 3]));
        inbox.push(bytes(&[]));
        inbox.push(bytes(&[4, 5, 6]));
        assert_eq!(read_now(&inbox, 4), Some(Ok(vec![1, 2, 3, 4])));
        assert_eq!(read_now(&inbox, 1), Some(Ok(vec![5])));
        assert_eq!(read_now(&inbox, 4), Some(Ok(vec![6])));
        assert_eq!(read_now(&inbox, 4), None);
    }

    /// The end is reported after the bytes before it; later messages and
    /// ends are ignored.
    #[wasm_bindgen_test]
    fn the_end_follows_the_queued_bytes() {
        let inbox = Inbox::new();
        inbox.push(bytes(&[1]));
        inbox.end("connection closed");
        inbox.push(bytes(&[2]));
        inbox.end("later");
        assert_eq!(read_now(&inbox, 4), Some(Ok(vec![1])));
        let end = Some(Err("connection closed".to_owned()));
        assert_eq!(read_now(&inbox, 4), end);
        assert_eq!(read_now(&inbox, 4), end);
        assert_eq!(
            inbox.opened().now_or_never(),
            Some(Err("connection closed".to_owned()))
        );
    }

    /// A message past the cap fails the link at once, dropping the queue.
    #[wasm_bindgen_test]
    fn a_full_inbox_fails_the_link() {
        let inbox = Inbox::new();
        inbox.push(bytes(&vec![7; INBOX_CAP - 1]));
        inbox.push(bytes(&[8]));
        assert_eq!(inbox.ended(), None, "exactly full");
        inbox.push(bytes(&[9]));
        assert_eq!(
            read_now(&inbox, 16),
            Some(Err("read queue overflow".to_owned()))
        );
    }

    #[wasm_bindgen_test]
    fn opening_resolves_the_dial() {
        let inbox = Inbox::new();
        assert_eq!(inbox.opened().now_or_never(), None);
        inbox.set_open();
        assert_eq!(inbox.opened().now_or_never(), Some(Ok(())));
    }
}
