// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! WebSocket links: the dial, the link's writer, and hanging up sockets
//! the page lost while it was hidden.
//!
//! - **Dial.** `new WebSocket(url)` with `binaryType = "arraybuffer"`,
//!   then the first of open, error or close. The caller bounds it with the
//!   dial timeout. Messages that arrive before the link's reader starts
//!   wait in the socket's `Inbox`.
//! - **Reads.** Every binary message goes to the inbox; a text message
//!   fails the link. The reader is the shared one in [`crate::io`]. A
//!   browser can't send pings and doesn't surface the peer's, so a silent
//!   link times out only while the node says its peer sends keepalives
//!   (`IdleOk`).
//! - **Writes.** One message per write buffer. While more than
//!   [`MAX_BUFFERED`] bytes wait in the socket, the writer polls every
//!   5 ms, for at most the write timeout. When the node drops the link,
//!   the writer closes the socket with code 1000.
//! - **Hang-ups.** Locking the screen or backgrounding a page on iOS kills
//!   its sockets without reliably firing their events or updating
//!   `readyState`. When the page is visible again (`hang_up`), sockets
//!   past `OPEN` are closed and their links failed, or every socket when
//!   the window says the platform needs it.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use js_sys::{ArrayBuffer, Uint8Array};
use strims_base::time::{Duration, Time};
use strims_node_core::{Input, Inputs, LinkKey};
use tokio::sync::mpsc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{BinaryType, CloseEvent, MessageEvent, WebSocket};

use crate::clock::{self, Sleep};
use crate::inbox::Inbox;
use crate::io::WasmTransport;
use crate::js;

/// The most bytes a link leaves in its socket's `bufferedAmount` before it
/// waits.
pub const MAX_BUFFERED: u32 = 256 * 1024;

/// How often a writer over [`MAX_BUFFERED`] checks the socket again.
const WRITE_POLL: Duration = Duration::from_millis(5);

thread_local! {
    /// Every live socket, for [`hang_up`]. An entry lives as long as its
    /// socket's [`WsSocket`].
    static SOCKETS: RefCell<Registry> = RefCell::default();
}

#[derive(Default)]
struct Registry {
    next: u64,
    sockets: BTreeMap<u64, (WebSocket, Inbox)>,
}

/// A WebSocket link's socket. Dropping the last handle closes it.
pub struct WsSocket(Rc<Ws>);

struct Ws {
    socket: WebSocket,
    inbox: Inbox,
    /// The socket's key in `SOCKETS`.
    id: u64,
    _on_open: Closure<dyn FnMut()>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut()>,
    _on_close: Closure<dyn FnMut(CloseEvent)>,
}

impl fmt::Debug for WsSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WsSocket")
            .field("url", &self.0.socket.url())
            .field("ready_state", &self.0.socket.ready_state())
            .finish()
    }
}

impl Drop for Ws {
    fn drop(&mut self) {
        let _ = SOCKETS.try_with(|r| r.borrow_mut().sockets.remove(&self.id));
        let s = &self.socket;
        s.set_onopen(None);
        s.set_onmessage(None);
        s.set_onerror(None);
        s.set_onclose(None);
        // A no-op once it is closed.
        let _ = s.close_with_code(1000);
        self.inbox.end("connection closed");
    }
}

/// Opens a WebSocket to `url` and waits until it is open. Fails with the
/// browser's error, or with how the socket ended before it opened.
pub(crate) async fn dial(url: &str) -> Result<WsSocket, String> {
    let socket = WebSocket::new(url).map_err(|e| js::describe(&e))?;
    let ws = WsSocket::new(socket);
    ws.0.inbox.opened().await?;
    Ok(ws)
}

impl WsSocket {
    fn new(socket: WebSocket) -> WsSocket {
        socket.set_binary_type(BinaryType::Arraybuffer);
        let inbox = Inbox::new();
        let on_open = {
            let inbox = inbox.clone();
            Closure::<dyn FnMut()>::new(move || inbox.set_open())
        };
        let on_message = {
            let inbox = inbox.clone();
            Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                match e.data().dyn_into::<ArrayBuffer>() {
                    Ok(b) => inbox.push(Uint8Array::new(&b)),
                    Err(_) => inbox.end("unexpected text message"),
                }
            })
        };
        // Browsers say nothing about why a socket failed; its close
        // follows.
        let on_error = {
            let inbox = inbox.clone();
            Closure::<dyn FnMut()>::new(move || inbox.end("websocket error"))
        };
        let on_close = {
            let inbox = inbox.clone();
            Closure::<dyn FnMut(CloseEvent)>::new(move |e: CloseEvent| {
                inbox.end(&match e.code() {
                    1000 => "connection closed".to_owned(),
                    code => format!("connection closed (code {code})"),
                });
            })
        };
        socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        let id = SOCKETS.with(|r| {
            let mut r = r.borrow_mut();
            let id = r.next;
            r.next += 1;
            r.sockets.insert(id, (socket.clone(), inbox.clone()));
            id
        });
        WsSocket(Rc::new(Ws {
            socket,
            inbox,
            id,
            _on_open: on_open,
            _on_message: on_message,
            _on_error: on_error,
            _on_close: on_close,
        }))
    }

    pub(crate) fn inbox(&self) -> Inbox {
        self.0.inbox.clone()
    }

    /// Sends `buf` as one binary message once the socket holds at most
    /// [`MAX_BUFFERED`] bytes, waiting at most `write_timeout` (none when
    /// it isn't positive).
    async fn send(&self, buf: &[u8], write_timeout: Duration) -> Result<(), String> {
        let s = &self.0.socket;
        if s.buffered_amount() > MAX_BUFFERED {
            let deadline = (write_timeout > Duration::ZERO).then(|| clock::now() + write_timeout);
            let mut sleep = Sleep::until(Time::NIL);
            while s.buffered_amount() > MAX_BUFFERED {
                if s.ready_state() > WebSocket::OPEN {
                    break;
                }
                if deadline.is_some_and(|d| clock::now() >= d) {
                    return Err("write timeout".to_owned());
                }
                sleep.reset(clock::now() + WRITE_POLL);
                (&mut sleep).await;
            }
        }
        // A closing socket would take the message and drop it.
        if s.ready_state() != WebSocket::OPEN {
            return Err(self
                .0
                .inbox
                .ended()
                .unwrap_or_else(|| "connection closed".to_owned()));
        }
        s.send_with_u8_array(buf).map_err(|e| js::describe(&e))
    }
}

/// Writes the node's buffers to the socket, returning each, and closes it
/// with code 1000 when the node drops the link.
pub(crate) async fn writer(
    link: LinkKey,
    ws: WsSocket,
    mut writes: mpsc::Receiver<Vec<u8>>,
    inputs: Inputs<WasmTransport>,
    write_timeout: Duration,
) {
    while let Some(buf) = writes.recv().await {
        if let Err(error) = ws.send(&buf, write_timeout).await {
            let _ = inputs.send(Input::LinkDown { link, error }).await;
            return;
        }
        if inputs.send(Input::WriteDone { link, buf }).await.is_err() {
            return;
        }
    }
    let _ = ws.0.socket.close_with_code(1000);
}

/// The page is visible again: closes the sockets past `OPEN`, or every
/// socket when `force` is set, and fails their links.
pub(crate) fn hang_up(force: bool) {
    let lost: Vec<(WebSocket, Inbox)> = SOCKETS.with(|r| {
        r.borrow()
            .sockets
            .values()
            .filter(|(s, _)| force || s.ready_state() > WebSocket::OPEN)
            .cloned()
            .collect()
    });
    if !lost.is_empty() {
        tracing::debug!(sockets = lost.len(), force, "hanging up websockets");
    }
    for (socket, inbox) in lost {
        let _ = socket.close_with_code(1000);
        inbox.end("connection closed");
    }
}

/// The live sockets, for tests.
#[doc(hidden)]
pub fn live_sockets() -> usize {
    SOCKETS.with(|r| r.borrow().sockets.len())
}
