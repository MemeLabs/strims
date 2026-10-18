// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's frontend RPC connection, over the `MessagePort` the window
//! hands the worker.
//!
//! # The port
//!
//! Each message from the window is an `ArrayBuffer` (or a `Uint8Array`)
//! of whole varint-delimited Calls, which [`PortStream`] yields as one
//! item. Each item the connection writes is one prefixed Call, and
//! [`PortSink`] posts it as one `ArrayBuffer`, transferred. A message that
//! isn't binary fails the stream.
//!
//! A port can't be paused, so the stream keeps what arrives as JS buffers,
//! outside wasm memory, until the connection reads it: at most
//! [`MAX_QUEUED_BYTES`]. A message past that fails the stream and drops
//! what was queued.
//!
//! The stream ends at the port's `close` event, after the messages before
//! it, where the browser fires one. Dropping the stream closes the port.
//!
//! # Serving
//!
//! [`serve`] runs a frontend connection on a port with only
//! `strims.auth.v1.AuthFrontend` bound; a sign-in or sign-up binds the
//! session's frontends on the same connection. The connection has no
//! session key, so a persisted sign-in gets the profile key itself: the UI
//! keeps it, in its own storage, and signs in with it again.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use futures_sink::Sink;
use js_sys::{Array, ArrayBuffer, Uint8Array};
use strims_kv::Kv;
use strims_proto::auth::v1::register_auth_frontend_service;
use strims_rpc::{Config as RpcConfig, ConnectionError, Router, Timer, connect};
use strims_session::{AuthService, Manager, SessionFactory};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Event, MessageEvent, MessagePort};

use crate::js;

/// The most bytes of messages the stream holds unread: four of the
/// frontend's largest Calls (`strims_rpc::Config::frontend`, 10 MiB).
pub const MAX_QUEUED_BYTES: usize = 40 * 1024 * 1024;

/// Why a frontend port failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PortError {
    /// More than [`MAX_QUEUED_BYTES`] arrived unread.
    #[error("frontend read queue overflow")]
    Overflow,
    /// The window sent something other than an `ArrayBuffer` or a
    /// `Uint8Array`.
    #[error("a frontend message that isn't binary")]
    NotBinary,
    /// A message didn't deserialize (`messageerror`).
    #[error("a frontend message that didn't deserialize")]
    Undecodable,
    /// `postMessage` threw.
    #[error("posting a frontend message: {0}")]
    Post(String),
    /// The sink was closed.
    #[error("the frontend port is closed")]
    Closed,
}

/// Runs a frontend connection on `port` for `manager`'s profiles until the
/// port closes or the connection fails. See the [module docs](self).
pub async fn serve<K, F, T>(
    manager: Rc<Manager<K, F, T>>,
    port: MessagePort,
) -> Result<(), ConnectionError<PortError>>
where
    K: Kv + Clone,
    F: SessionFactory<K>,
    T: Timer,
{
    let (reader, writer) = transport(port);
    let mut router = Router::new();
    let auth = AuthService::new(manager, None, router.clone());
    register_auth_frontend_service(&mut router, Rc::new(auth));
    let (_, conn) = connect(RpcConfig::frontend(), reader, writer, Some(router));
    conn.await
}

/// The two halves of a frontend connection on `port`, which starts
/// delivering its messages.
pub fn transport(port: MessagePort) -> (PortStream, PortSink) {
    let sink = PortSink {
        port: port.clone(),
        closed: false,
    };
    (PortStream::new(port), sink)
}

/// What the port's callbacks queue for the stream.
#[derive(Default)]
struct Queue {
    /// At most [`MAX_QUEUED_BYTES`] in all.
    messages: VecDeque<Uint8Array>,
    bytes: usize,
    /// Why the stream failed, until it reports it.
    error: Option<PortError>,
    /// The port closed or the stream failed: nothing more is queued.
    ended: bool,
    waker: Option<Waker>,
}

impl Queue {
    fn push(&mut self, data: Uint8Array) {
        if self.ended {
            return;
        }
        let len = data.length() as usize;
        if self.bytes.saturating_add(len) > MAX_QUEUED_BYTES {
            self.fail(PortError::Overflow);
            return;
        }
        self.messages.push_back(data);
        self.bytes += len;
    }

    fn fail(&mut self, error: PortError) {
        if self.ended {
            return;
        }
        self.messages.clear();
        self.bytes = 0;
        self.error = Some(error);
        self.ended = true;
    }

    /// Runs `f` on the queue, then wakes the stream outside the borrow.
    fn update(queue: &RefCell<Queue>, f: impl FnOnce(&mut Queue)) {
        let waker = {
            let mut q = queue.borrow_mut();
            f(&mut q);
            q.waker.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
    }
}

/// A port's messages, one item each; see the [module docs](self).
pub struct PortStream {
    port: MessagePort,
    queue: Rc<RefCell<Queue>>,
    /// The port's handlers, attached while the stream lives.
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_message_error: Closure<dyn FnMut(MessageEvent)>,
    on_close: Closure<dyn FnMut(Event)>,
}

impl fmt::Debug for PortStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let q = self.queue.borrow();
        f.debug_struct("PortStream")
            .field("queued", &q.messages.len())
            .field("bytes", &q.bytes)
            .field("ended", &q.ended)
            .finish_non_exhaustive()
    }
}

impl PortStream {
    /// Takes over `port`'s messages. Setting `onmessage` starts the port,
    /// which delivers what it buffered.
    fn new(port: MessagePort) -> PortStream {
        let queue = Rc::new(RefCell::new(Queue::default()));
        let q = queue.clone();
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            let data = e.data();
            Queue::update(&q, |q| {
                if let Some(buffer) = data.dyn_ref::<ArrayBuffer>() {
                    q.push(Uint8Array::new(buffer));
                } else if let Ok(view) = data.dyn_into::<Uint8Array>() {
                    q.push(view);
                } else {
                    q.fail(PortError::NotBinary);
                }
            });
        });
        let q = queue.clone();
        let on_message_error = Closure::<dyn FnMut(MessageEvent)>::new(move |_: MessageEvent| {
            Queue::update(&q, |q| q.fail(PortError::Undecodable));
        });
        let q = queue.clone();
        let on_close = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
            Queue::update(&q, |q| q.ended = true);
        });
        port.set_onmessageerror(Some(on_message_error.as_ref().unchecked_ref()));
        // Browsers without the event never end the stream this way.
        let _ = port.add_event_listener_with_callback("close", on_close.as_ref().unchecked_ref());
        port.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        PortStream {
            port,
            queue,
            _on_message: on_message,
            _on_message_error: on_message_error,
            on_close,
        }
    }
}

impl Stream for PortStream {
    type Item = Result<Vec<u8>, PortError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut q = self.queue.borrow_mut();
        if let Some(m) = q.messages.pop_front() {
            q.bytes = q.bytes.saturating_sub(m.length() as usize);
            return Poll::Ready(Some(Ok(m.to_vec())));
        }
        if let Some(e) = q.error.take() {
            return Poll::Ready(Some(Err(e)));
        }
        if q.ended {
            return Poll::Ready(None);
        }
        let old = match q.waker.as_ref().is_some_and(|w| w.will_wake(cx.waker())) {
            true => None,
            false => q.waker.replace(cx.waker().clone()),
        };
        drop(q);
        drop(old);
        Poll::Pending
    }
}

impl Drop for PortStream {
    fn drop(&mut self) {
        // No callback may run once its closure is gone.
        self.port.set_onmessage(None);
        self.port.set_onmessageerror(None);
        let _ = self
            .port
            .remove_event_listener_with_callback("close", self.on_close.as_ref().unchecked_ref());
        self.port.close();
    }
}

/// Posts each Call to a port as one transferred `ArrayBuffer`; see the
/// [module docs](self).
#[derive(Debug)]
pub struct PortSink {
    port: MessagePort,
    closed: bool,
}

impl Sink<Vec<u8>> for PortSink {
    type Error = PortError;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), PortError>> {
        Poll::Ready(match self.closed {
            true => Err(PortError::Closed),
            false => Ok(()),
        })
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), PortError> {
        if self.closed {
            return Err(PortError::Closed);
        }
        let buffer = Uint8Array::from(item.as_slice()).buffer();
        self.port
            .post_message_with_transferable(&buffer, &Array::of1(&buffer))
            .map_err(|e| PortError::Post(js::describe(&e)))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), PortError>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), PortError>> {
        self.closed = true;
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use futures_util::{SinkExt, StreamExt};
    use wasm_bindgen_test::wasm_bindgen_test;
    use web_sys::MessageChannel;

    use super::*;

    #[wasm_bindgen_test]
    async fn messages_cross_the_port_whole() {
        let channel = MessageChannel::new().unwrap();
        let (mut a, _a) = transport(channel.port1());
        let (_b, mut b) = transport(channel.port2());
        b.send(vec![1, 2, 3]).await.unwrap();
        b.send(vec![4]).await.unwrap();
        assert_eq!(a.next().await, Some(Ok(vec![1, 2, 3])));
        assert_eq!(a.next().await, Some(Ok(vec![4])));
    }

    #[wasm_bindgen_test]
    async fn views_are_taken_and_other_messages_fail_the_stream() {
        let channel = MessageChannel::new().unwrap();
        let (mut a, _a) = transport(channel.port1());
        let other = channel.port2();
        other
            .post_message(&Uint8Array::from(&[5u8, 6][..]))
            .unwrap();
        other.post_message(&"text".into()).unwrap();
        other.post_message(&Uint8Array::from(&[7u8][..])).unwrap();
        assert_eq!(a.next().await, Some(Ok(vec![5, 6])));
        assert_eq!(a.next().await, Some(Err(PortError::NotBinary)));
        assert_eq!(a.next().await, None);
    }

    #[wasm_bindgen_test]
    fn a_full_queue_fails_the_stream() {
        let mut q = Queue::default();
        q.push(Uint8Array::new_with_length(MAX_QUEUED_BYTES as u32 - 1));
        q.push(Uint8Array::new_with_length(1));
        assert!(!q.ended, "exactly full");
        q.push(Uint8Array::new_with_length(1));
        assert_eq!((q.ended, q.bytes), (true, 0));
        assert_eq!(q.error, Some(PortError::Overflow));
    }
}
