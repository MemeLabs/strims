// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! WebRTC in the worker: the [`SignalPort`] task between the node's
//! [`ProxyRtc`](strims_node_core::ProxyRtc) and the window's peer
//! connections, and the data channels the window hands over
//! ([`DcSocket`]).
//!
//! # The signal port
//!
//! Messages are plain objects with a `type`; `conn` is a connection id, a
//! JS-safe integer.
//!
//! | from | message | here |
//! |---|---|---|
//! | node | `Create` | posts `{type: "create", conn, iceServers, gatherTimeoutMs}` |
//! | node | `Describe` | posts `{type: "describe", conn, sdpType: "offer" \| "answer"}` |
//! | node | `SetRemote` | posts `{type: "remote", conn, description}`, the description's JSON |
//! | node | `Close` | posts `{type: "close", conn}` and forgets the connection |
//! | window | `{type: "channel", conn, channel}` | sets the transferred channel up at once and keeps it |
//! | window | `{type: "local", conn, description}` | `Signal(LocalDescription)`, or `Signal(Closed)` when the JSON doesn't parse |
//! | window | `{type: "connected", conn, maxMessageSize}` | `Signal(Connected)` |
//! | window | `{type: "closed", conn, reason}` | `Signal(Closed(reason))`, and forgets the connection |
//! | window | `{type: "visibility", hidden, forceHUP}` | when visible, hangs up lost WebSockets ([`crate::ws`]) |
//! | panic | | posts `{type: "fatal", message}` |
//!
//! Window messages about connections the node didn't create, or has
//! closed, are dropped; a channel for one is closed.
//!
//! **Attaching a channel.** Once the channel is open and `connected` has
//! arrived, in either order, the task sends `Input::Connected` with
//! `rtc: Some(conn)` and a link MTU of the smaller of `maxMessageSize` and
//! `DATA_CHANNEL_MTU`, or `DATA_CHANNEL_MTU` when the size is 0 or unknown.
//! A channel that closes before then is `Signal(Closed("data channel
//! closed"))`.
//!
//! # Data channels
//!
//! A transferred channel gets `binaryType = "arraybuffer"`, listeners that
//! fill its `Inbox`, and a `bufferedAmountLowThreshold` of half
//! `WebRtcConfig::max_buffered_amount`, all in the callback that received
//! it, before any of its events can be dispatched. As a link it has the
//! shared reader ([`crate::io`]) with no read timeout: ICE and the
//! connection state end a dead channel. Its writer sends while the
//! channel's `bufferedAmount` is below `max_buffered_amount`, and otherwise
//! waits for `bufferedamountlow`. Dropping the last handle of a
//! [`DcSocket`] closes the channel.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt;
use std::future::poll_fn;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use js_sys::{Array, ArrayBuffer, Object, Reflect, Uint8Array};
use strims_node_core::rtc::proxy::MAX_CONN_ID;
use strims_node_core::{Input, Inputs, LinkKey, RtcCommand, RtcConn, RtcInput, RtcSignal};
use strims_vnic::LinkConfig;
use strims_vnic::webrtc::{DATA_CHANNEL_MTU, SessionDescription};
use tokio::sync::mpsc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{
    Event, MessageEvent, MessagePort, RtcDataChannel, RtcDataChannelState, RtcDataChannelType,
};

use crate::inbox::Inbox;
use crate::io::WasmTransport;
use crate::{js, ws};

/// What the port's and the channels' callbacks hand the task, in the order
/// they ran.
enum PortEvent {
    /// A transferred channel, already set up.
    Channel {
        conn: u64,
        socket: DcSocket,
    },
    /// A kept channel's `open` event.
    Open(u64),
    /// A kept channel's `close` event.
    ChannelClosed(u64),
    Local {
        conn: u64,
        description: String,
    },
    Connected {
        conn: u64,
        max_message_size: f64,
    },
    Closed {
        conn: u64,
        reason: String,
    },
}

/// A connection the node created and hasn't closed. At most
/// `EndpointConfig::max_connections` exist, as the proxy makes no more.
#[derive(Default)]
struct Slot {
    /// The channel, until it becomes a link.
    channel: Option<DcSocket>,
    /// `connected` arrived, with this `maxMessageSize`.
    connected: Option<f64>,
    /// The channel went to the node.
    attached: bool,
}

/// The task that owns the window's signal port (see the
/// [module docs](self)).
pub struct SignalPort {
    port: MessagePort,
    commands: mpsc::UnboundedReceiver<RtcCommand>,
    /// From the callbacks. Unbounded: the window sends a few messages per
    /// connection, and a channel two events, which the task takes as they
    /// come.
    events: mpsc::UnboundedReceiver<PortEvent>,
    on_message: Closure<dyn FnMut(MessageEvent)>,
    slots: BTreeMap<u64, Slot>,
}

impl fmt::Debug for SignalPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignalPort")
            .field("connections", &self.slots.len())
            .finish_non_exhaustive()
    }
}

impl SignalPort {
    /// Takes over `port`, whose messages are handled from now on, and
    /// makes a panic post `fatal` on it. `commands` are the node's
    /// [`ProxyRtc`](strims_node_core::ProxyRtc)'s; `max_buffered_amount`
    /// is `WebRtcConfig::max_buffered_amount`.
    pub fn new(
        port: MessagePort,
        commands: mpsc::UnboundedReceiver<RtcCommand>,
        max_buffered_amount: usize,
    ) -> SignalPort {
        let (events_tx, events) = mpsc::unbounded_channel();
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            on_message(&events_tx, &e.data(), max_buffered_amount);
        });
        // Setting `onmessage` starts the port, which delivers what it
        // buffered.
        port.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        let fatal = port.clone();
        crate::log::set_fatal_hook(move |message| {
            let msg = object(&[("type", "fatal".into()), ("message", message.into())]);
            let _ = fatal.post_message(&msg);
        });
        SignalPort {
            port,
            commands,
            events,
            on_message,
            slots: BTreeMap::new(),
        }
    }

    /// Runs until the node's commands end, sending what the window reports
    /// to `inputs`.
    pub async fn run(mut self, inputs: Inputs<WasmTransport>) {
        loop {
            tokio::select! {
                biased;
                c = self.commands.recv() => match c {
                    Some(c) => self.command(c),
                    None => break,
                },
                Some(e) = self.events.recv() => self.event(e, &inputs).await,
            }
        }
    }

    fn command(&mut self, c: RtcCommand) {
        let msg = match c {
            RtcCommand::Create {
                conn,
                ice_servers,
                gather_timeout,
            } => {
                self.slots.insert(conn.0, Slot::default());
                let urls: Array = ice_servers.iter().map(|s| JsValue::from_str(s)).collect();
                object(&[
                    ("type", "create".into()),
                    ("conn", id(conn)),
                    ("iceServers", urls.into()),
                    (
                        "gatherTimeoutMs",
                        (gather_timeout.as_millis().max(0) as f64).into(),
                    ),
                ])
            }
            RtcCommand::Describe { conn, sdp_type } => object(&[
                ("type", "describe".into()),
                ("conn", id(conn)),
                ("sdpType", sdp_type.as_str().into()),
            ]),
            RtcCommand::SetRemote { conn, desc } => object(&[
                ("type", "remote".into()),
                ("conn", id(conn)),
                ("description", desc.to_json().into()),
            ]),
            RtcCommand::Close { conn } => {
                self.slots.remove(&conn.0);
                object(&[("type", "close".into()), ("conn", id(conn))])
            }
        };
        if let Err(e) = self.port.post_message(&msg) {
            tracing::debug!(error = %js::describe(&e), "signal port");
        }
    }

    async fn event(&mut self, e: PortEvent, inputs: &Inputs<WasmTransport>) {
        match e {
            PortEvent::Channel { conn, socket } => {
                // Unknown, or a second channel: dropping it closes it.
                let Some(slot) = self.slots.get_mut(&conn) else {
                    return;
                };
                if slot.attached || slot.channel.is_some() {
                    return;
                }
                if socket.closed() {
                    signal(
                        inputs,
                        conn,
                        RtcSignal::Closed("data channel closed".to_owned()),
                    )
                    .await;
                    return;
                }
                // It may have opened before its `open` listener was set.
                slot.channel = Some(socket);
                self.attach(conn, inputs).await;
            }
            PortEvent::Open(conn) => self.attach(conn, inputs).await,
            PortEvent::ChannelClosed(conn) => {
                // A link's reader reports its own channel closing.
                let Some(slot) = self.slots.get_mut(&conn) else {
                    return;
                };
                if slot.channel.as_ref().is_some_and(DcSocket::closed) {
                    slot.channel = None;
                    signal(
                        inputs,
                        conn,
                        RtcSignal::Closed("data channel closed".to_owned()),
                    )
                    .await;
                }
            }
            PortEvent::Local { conn, description } => {
                if !self.slots.contains_key(&conn) {
                    return;
                }
                let s = match SessionDescription::from_json(&description) {
                    Ok(d) => RtcSignal::LocalDescription(d),
                    Err(e) => RtcSignal::Closed(format!("invalid local description: {e}")),
                };
                signal(inputs, conn, s).await;
            }
            PortEvent::Connected {
                conn,
                max_message_size,
            } => {
                let Some(slot) = self.slots.get_mut(&conn) else {
                    return;
                };
                if slot.connected.is_some() {
                    return;
                }
                slot.connected = Some(max_message_size);
                signal(inputs, conn, RtcSignal::Connected).await;
                self.attach(conn, inputs).await;
            }
            PortEvent::Closed { conn, reason } => {
                if self.slots.remove(&conn).is_some() {
                    signal(inputs, conn, RtcSignal::Closed(reason)).await;
                }
            }
        }
    }

    /// Hands the connection's channel to the node once it is open and the
    /// connection is up.
    async fn attach(&mut self, conn: u64, inputs: &Inputs<WasmTransport>) {
        let Some(slot) = self.slots.get_mut(&conn) else {
            return;
        };
        let (Some(mms), false) = (slot.connected, slot.attached) else {
            return;
        };
        let Some(socket) = slot.channel.take_if(|c| c.open()) else {
            return;
        };
        slot.attached = true;
        let msg = Input::Connected {
            dial: None,
            transport: WasmTransport::Channel(socket),
            config: LinkConfig::webrtc().with_mtu(channel_mtu(mms)),
            leftover: Vec::new(),
            notsent_lowat: None,
            rtc: Some(RtcConn(conn)),
        };
        let _ = inputs.send(msg).await;
    }
}

impl Drop for SignalPort {
    fn drop(&mut self) {
        // A later node's task may have taken the port over.
        let ours = self
            .port
            .onmessage()
            .is_some_and(|f| Object::is(f.as_ref(), self.on_message.as_ref()));
        if ours {
            self.port.set_onmessage(None);
        }
    }
}

/// A link MTU for a connection whose SCTP transport takes messages of up
/// to `max_message_size` bytes: 0 (or not a positive number) means the
/// channel's default.
fn channel_mtu(max_message_size: f64) -> usize {
    if max_message_size >= 1.0 {
        // `as` saturates; the value is at most DATA_CHANNEL_MTU here.
        max_message_size.min(DATA_CHANNEL_MTU as f64) as usize
    } else {
        DATA_CHANNEL_MTU
    }
}

async fn signal(inputs: &Inputs<WasmTransport>, conn: u64, signal: RtcSignal) {
    let input = RtcInput {
        conn: RtcConn(conn),
        signal,
    };
    let _ = inputs.send(Input::Signal(input)).await;
}

/// Handles one message from the window, in its callback.
fn on_message(events: &mpsc::UnboundedSender<PortEvent>, data: &JsValue, max_buffered: usize) {
    let Some(kind) = property(data, "type").as_string() else {
        return;
    };
    if kind == "visibility" {
        if property(data, "hidden").as_bool() == Some(false) {
            ws::hang_up(property(data, "forceHUP").as_bool() == Some(true));
        }
        return;
    }
    let Some(conn) = property(data, "conn").as_f64().and_then(conn_id) else {
        return;
    };
    let text = |key| property(data, key).as_string().unwrap_or_default();
    let event = match kind.as_str() {
        "channel" => {
            let channel = property(data, "channel");
            if !channel.is_object() {
                return;
            }
            let socket =
                DcSocket::new(channel.unchecked_into(), conn, events.clone(), max_buffered);
            PortEvent::Channel { conn, socket }
        }
        "local" => PortEvent::Local {
            conn,
            description: text("description"),
        },
        "connected" => PortEvent::Connected {
            conn,
            max_message_size: property(data, "maxMessageSize").as_f64().unwrap_or(0.0),
        },
        "closed" => PortEvent::Closed {
            conn,
            reason: text("reason"),
        },
        _ => return,
    };
    let _ = events.send(event);
}

/// A connection id from a JS number.
fn conn_id(v: f64) -> Option<u64> {
    (v.fract() == 0.0 && (0.0..=MAX_CONN_ID as f64).contains(&v)).then_some(v as u64)
}

fn id(conn: RtcConn) -> JsValue {
    // Ids are at most MAX_CONN_ID, which an f64 holds exactly.
    JsValue::from_f64(conn.0 as f64)
}

fn property(v: &JsValue, key: &str) -> JsValue {
    if !v.is_object() {
        return JsValue::UNDEFINED;
    }
    Reflect::get(v, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
}

fn object(fields: &[(&str, JsValue)]) -> Object {
    let o = Object::new();
    for (k, v) in fields {
        let _ = Reflect::set(&o, &JsValue::from_str(k), v);
    }
    o
}

/// A wake-up for one waiting task, kept until it is taken.
#[derive(Debug, Default)]
struct Notify {
    fired: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl Notify {
    fn notify(&self) {
        self.fired.set(true);
        let waker = self.waker.borrow_mut().take();
        if let Some(w) = waker {
            w.wake();
        }
    }

    fn poll_wait(&self, cx: &mut Context<'_>) -> Poll<()> {
        if self.fired.replace(false) {
            return Poll::Ready(());
        }
        let old = self.waker.borrow_mut().replace(cx.waker().clone());
        drop(old);
        Poll::Pending
    }
}

/// A data channel the window transferred. Dropping the last handle closes
/// it.
pub struct DcSocket(Rc<Dc>);

struct Dc {
    channel: RtcDataChannel,
    inbox: Inbox,
    /// `bufferedamountlow`, or the channel ending.
    low: Rc<Notify>,
    _on_open: Closure<dyn FnMut()>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut(Event)>,
    _on_close: Closure<dyn FnMut()>,
    _on_low: Closure<dyn FnMut()>,
}

impl fmt::Debug for DcSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DcSocket")
            .field("ready_state", &self.0.channel.ready_state())
            .finish()
    }
}

impl Drop for Dc {
    fn drop(&mut self) {
        let c = &self.channel;
        c.set_onopen(None);
        c.set_onmessage(None);
        c.set_onerror(None);
        c.set_onclose(None);
        c.set_onbufferedamountlow(None);
        c.close();
        self.inbox.end("data channel closed");
    }
}

impl DcSocket {
    /// Sets up connection `conn`'s channel. Its `open` and `close` events
    /// also go to the signal task, while it keeps the channel.
    fn new(
        channel: RtcDataChannel,
        conn: u64,
        events: mpsc::UnboundedSender<PortEvent>,
        max_buffered: usize,
    ) -> DcSocket {
        channel.set_binary_type(RtcDataChannelType::Arraybuffer);
        channel
            .set_buffered_amount_low_threshold(u32::try_from(max_buffered / 2).unwrap_or(u32::MAX));
        let inbox = Inbox::new();
        let low = Rc::new(Notify::default());
        let on_open = {
            let (inbox, events) = (inbox.clone(), events.clone());
            Closure::<dyn FnMut()>::new(move || {
                inbox.set_open();
                let _ = events.send(PortEvent::Open(conn));
            })
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
        // An RTCErrorEvent; `close` follows.
        let on_error = {
            let (inbox, low) = (inbox.clone(), low.clone());
            Closure::<dyn FnMut(Event)>::new(move |e: Event| {
                let error = property(&e, "error");
                let reason = if error.is_undefined() {
                    "data channel error".to_owned()
                } else {
                    js::describe(&error)
                };
                inbox.end(&reason);
                low.notify();
            })
        };
        let on_close = {
            let (inbox, low) = (inbox.clone(), low.clone());
            Closure::<dyn FnMut()>::new(move || {
                inbox.end("data channel closed");
                low.notify();
                let _ = events.send(PortEvent::ChannelClosed(conn));
            })
        };
        let on_low = {
            let low = low.clone();
            Closure::<dyn FnMut()>::new(move || low.notify())
        };
        channel.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        channel.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        channel.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        channel.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        channel.set_onbufferedamountlow(Some(on_low.as_ref().unchecked_ref()));
        if channel.ready_state() == RtcDataChannelState::Open {
            inbox.set_open();
        }
        DcSocket(Rc::new(Dc {
            channel,
            inbox,
            low,
            _on_open: on_open,
            _on_message: on_message,
            _on_error: on_error,
            _on_close: on_close,
            _on_low: on_low,
        }))
    }

    pub(crate) fn inbox(&self) -> Inbox {
        self.0.inbox.clone()
    }

    fn open(&self) -> bool {
        self.0.channel.ready_state() == RtcDataChannelState::Open
    }

    fn closed(&self) -> bool {
        matches!(
            self.0.channel.ready_state(),
            RtcDataChannelState::Closing | RtcDataChannelState::Closed
        )
    }

    /// Sends `buf` as one message once the channel holds less than
    /// `max_buffered` bytes.
    async fn send(&self, buf: &[u8], max_buffered: usize) -> Result<(), String> {
        let c = &self.0.channel;
        loop {
            if c.ready_state() != RtcDataChannelState::Open {
                return Err(self
                    .0
                    .inbox
                    .ended()
                    .unwrap_or_else(|| "data channel closed".to_owned()));
            }
            if (c.buffered_amount() as usize) < max_buffered {
                break;
            }
            poll_fn(|cx| self.0.low.poll_wait(cx)).await;
        }
        c.send_with_u8_array(buf).map_err(|e| js::describe(&e))
    }
}

/// Writes the node's buffers to the channel, returning each, and closes
/// the channel when the node drops the link.
pub(crate) async fn writer(
    link: LinkKey,
    channel: DcSocket,
    mut writes: mpsc::Receiver<Vec<u8>>,
    inputs: Inputs<WasmTransport>,
    max_buffered: usize,
) {
    while let Some(buf) = writes.recv().await {
        if let Err(error) = channel.send(&buf, max_buffered).await {
            let _ = inputs.send(Input::LinkDown { link, error }).await;
            return;
        }
        if inputs.send(Input::WriteDone { link, buf }).await.is_err() {
            return;
        }
    }
    channel.0.channel.close();
}
