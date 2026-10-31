// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The player workers' feeds: a unit-format stream read from the swarm
//! ([`VideoControl::open_stream`]) and posted to a player worker over a
//! port of the stream's own, with no hop through the window.
//!
//! # The feeds port
//!
//! The window starts the worker with a feeds port ([`crate::entry::start`]),
//! which [`serve`] takes once a profile runs: messages the window sends
//! before then wait in the port. Each message from the window is an object
//! with a `type`:
//!
//! | `type` | fields | what the node does |
//! |---|---|---|
//! | `open` | `id`, `swarmUri`, `networkKeys` (an array of `Uint8Array`), `mimeType`, `port` (a transferred `MessagePort`), and optionally `replaces` | Opens a stream of the swarm in the format `mimeType` names ([`StreamFormat::from_mime`]) and feeds it on `port`. Once the stream opened or failed to, it closes feed `replaces`: a player's new feed holds the swarm before its old one lets go of it. |
//! | `close` | `id` | Closes the feed: its stream closes, and its port gets nothing more. |
//!
//! Ids are the window's: a number of at most 2^53, distinct among the
//! feeds open. A feed ends when its stream doesn't open, fails or ends
//! ([`VideoError::StreamClosed`] once its swarm is gone with none in its
//! place), or when its port closes; the node then posts
//! `{type: "closed", id, message}` on the feeds port, with the reason. A
//! feed the window closes, or one a new feed replaces, posts nothing more
//! on either port. At most [`MAX_FEEDS`] are open; an `open` past that, of
//! an id in use or that doesn't parse answers `closed` at once.
//!
//! # A feed's port
//!
//! The node posts batches, each one `ArrayBuffer`, transferred:
//!
//! ```text
//! kind      u8   BATCH_DATA or BATCH_END
//! flags     u8   BATCH_DATA only; bit 0 (BATCH_ACK): the node asks for an ack
//! messages  BATCH_DATA only, until the end of the buffer:
//!   flags   u8   bit 0 (DATA_RECORD_END) the message completes a record,
//!                bit 1 (DATA_DISCONTINUITY) the bytes before it were dropped
//!   len     u32  little-endian
//!   bytes   len bytes of records
//! ```
//!
//! Each message is an Egress `Data` message, with the same contract as the
//! RPC's ([`strims_video::frontend`]): the bytes since the last record end
//! or discontinuity are one whole record, and after a discontinuity the
//! next message has it and begins a whole record. A batch holds what the
//! stream had ready when its next read would wait, at most
//! [`MAX_BATCH_BYTES`] of records; a record past that spans batches. An
//! end batch comes last: the stream ended.
//!
//! The worker posts a number on the port: how many batches it has taken
//! so far, counted from the first. It acks each batch that asks, once that
//! batch's outputs are taken. The node keeps at most [`WINDOW_BYTES`] and
//! [`WINDOW_BATCHES`] posted and unacked, and asks for an ack every
//! quarter of either and on the batch that fills the window. With the
//! window full it reads no more: the swarm reader falls behind, within the
//! swarm's live window, and past it the stream skips ahead, with a
//! discontinuity. So a player that falls behind holds up its own stream,
//! within a bounded queue, and no one else's.
//!
//! The port's `close` event, where the browser fires one, ends the feed.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::future::{Future, poll_fn};
use std::pin::pin;
use std::rc::Rc;
use std::task::{Poll, Waker};

use futures_util::future::{Either, select};
use js_sys::{Array, Object, Reflect, Uint8Array};
use strims_directory::Host;
use strims_kv::Kv;
use strims_node_core::app::NodeHost;
use strims_rpc::Timer;
use strims_video::egress::{EgressPiece, EgressStream, StreamFormat};
use strims_video::{NoVideoHost, VideoControl, VideoError, VideoHost};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Event, MessageEvent, MessagePort};

use crate::clock::WasmClock;
use crate::js;
use crate::runtime::{self, TaskHandle};

/// A profile's video control in the worker.
pub type WasmVideo<K> = VideoControl<K, NodeHost, NoVideoHost, WasmClock>;

/// The most feeds open at once.
pub const MAX_FEEDS: usize = 32;
/// The most bytes of records a batch holds.
pub const MAX_BATCH_BYTES: usize = 256 * 1024;
/// The most bytes of batches posted and not yet acked.
pub const WINDOW_BYTES: usize = 4 * 1024 * 1024;
/// The most batches posted and not yet acked.
pub const WINDOW_BATCHES: usize = 64;

/// A batch's kinds: records, and the stream's end.
pub const BATCH_DATA: u8 = 1;
pub const BATCH_END: u8 = 2;
/// A data batch's flag: the node asks for an ack.
pub const BATCH_ACK: u8 = 0x01;
/// A message's flags.
pub const DATA_RECORD_END: u8 = 0x01;
pub const DATA_DISCONTINUITY: u8 = 0x02;

/// A data batch's header: its kind and flags.
const BATCH_HEADER: usize = 2;
/// A message's header: its flags and length.
const MESSAGE_HEADER: usize = 5;

// --- batches ------------------------------------------------------------------

/// Builds a data batch from a stream's pieces; see the
/// [module docs](self#a-feeds-port).
#[derive(Debug)]
pub struct Batch {
    /// The header and the messages so far, the last one open while
    /// `open` is set. Bound: [`MAX_BATCH_BYTES`] of records, plus the
    /// headers.
    buf: Vec<u8>,
    /// Bytes of records in `buf`.
    records: usize,
    /// Where the open message's header starts in `buf`.
    open: Option<usize>,
    /// The next message has [`DATA_DISCONTINUITY`].
    discontinuity: bool,
}

impl Default for Batch {
    fn default() -> Self {
        Batch::new()
    }
}

impl Batch {
    pub fn new() -> Batch {
        let mut buf = Vec::with_capacity(BATCH_HEADER + MESSAGE_HEADER + MAX_BATCH_BYTES);
        buf.extend_from_slice(&[BATCH_DATA, 0]);
        Batch {
            buf,
            records: 0,
            open: None,
            discontinuity: false,
        }
    }

    /// The most bytes the next data piece may hold: 0 once the batch is
    /// full.
    pub fn room(&self) -> usize {
        MAX_BATCH_BYTES.saturating_sub(self.records)
    }

    /// Nothing to post: no message.
    pub fn is_empty(&self) -> bool {
        self.buf.len() == BATCH_HEADER
    }

    /// Takes the next piece of the stream. A data piece holds at most
    /// [`room`](Self::room) bytes, which bounds the batch.
    pub fn piece(&mut self, piece: EgressPiece<'_>) {
        match piece {
            EgressPiece::Data(d) => {
                self.message();
                self.buf.extend_from_slice(d);
                self.records += d.len();
            }
            EgressPiece::RecordEnd => {
                let at = self.message();
                if let Some(flags) = self.buf.get_mut(at) {
                    *flags |= DATA_RECORD_END;
                }
                self.close();
            }
            EgressPiece::Discontinuity => {
                // The record in progress is no record: its bytes in this
                // batch go, and the next message says the bytes before it
                // in earlier batches went too.
                if let Some(at) = self.open.take() {
                    self.records = self
                        .records
                        .saturating_sub(self.buf.len().saturating_sub(at + MESSAGE_HEADER));
                    self.buf.truncate(at);
                }
                self.discontinuity = true;
            }
        }
    }

    /// The batch's bytes, with [`BATCH_ACK`] if `ack`, for posting. The
    /// open message goes in it unended, and the record goes on in the next
    /// batch's first message. Call [`clear`](Self::clear) after.
    pub fn finish(&mut self, ack: bool) -> &[u8] {
        self.close();
        if let Some(flags) = self.buf.get_mut(1) {
            *flags = if ack { BATCH_ACK } else { 0 };
        }
        &self.buf
    }

    /// Empties the batch for the next one.
    pub fn clear(&mut self) {
        self.buf.truncate(BATCH_HEADER);
        self.records = 0;
        self.open = None;
    }

    /// The open message's header offset, opening one if none is.
    fn message(&mut self) -> usize {
        if let Some(at) = self.open {
            return at;
        }
        let at = self.buf.len();
        let flags = if std::mem::take(&mut self.discontinuity) {
            DATA_DISCONTINUITY
        } else {
            0
        };
        self.buf.extend_from_slice(&[flags, 0, 0, 0, 0]);
        self.open = Some(at);
        at
    }

    /// Writes the open message's length, and closes it.
    fn close(&mut self) {
        let Some(at) = self.open.take() else {
            return;
        };
        let len = self.buf.len().saturating_sub(at + MESSAGE_HEADER);
        // A message holds at most MAX_BATCH_BYTES, far below u32::MAX.
        let len = u32::try_from(len).unwrap_or(u32::MAX).to_le_bytes();
        if let Some(field) = self.buf.get_mut(at + 1..at + MESSAGE_HEADER) {
            field.copy_from_slice(&len);
        }
    }
}

// --- the window ---------------------------------------------------------------

/// The batches posted and not yet acked, within [`WINDOW_BYTES`] and
/// [`WINDOW_BATCHES`]; see the [module docs](self#a-feeds-port).
#[derive(Debug, Default)]
pub struct Credit {
    /// Each unacked batch's bytes, oldest first. Bound: [`WINDOW_BATCHES`].
    unacked: VecDeque<usize>,
    bytes: usize,
    /// Batches posted, and acked.
    posted: u64,
    acked: u64,
    /// Bytes and batches posted since the last batch that asked for an
    /// ack.
    unasked_bytes: usize,
    unasked_batches: usize,
}

impl Credit {
    /// Another batch may be posted.
    pub fn is_open(&self) -> bool {
        self.bytes < WINDOW_BYTES && self.unacked.len() < WINDOW_BATCHES
    }

    /// Counts a batch of `bytes` as posted: whether it asks for an ack.
    pub fn post(&mut self, bytes: usize) -> bool {
        self.unacked.push_back(bytes);
        self.bytes = self.bytes.saturating_add(bytes);
        self.posted += 1;
        self.unasked_bytes = self.unasked_bytes.saturating_add(bytes);
        self.unasked_batches += 1;
        let ask = !self.is_open()
            || self.unasked_bytes >= WINDOW_BYTES / 4
            || self.unasked_batches >= WINDOW_BATCHES / 4;
        if ask {
            self.unasked_bytes = 0;
            self.unasked_batches = 0;
        }
        ask
    }

    /// The worker has taken `taken` batches, counted from the first. A
    /// count past those posted counts as all of them; one below the acks
    /// so far changes nothing.
    pub fn ack(&mut self, taken: u64) {
        let taken = taken.min(self.posted);
        while self.acked < taken {
            self.acked += 1;
            if let Some(b) = self.unacked.pop_front() {
                self.bytes = self.bytes.saturating_sub(b);
            }
        }
    }

    /// Bytes and batches posted and not yet acked.
    pub fn unacked(&self) -> (usize, usize) {
        (self.bytes, self.unacked.len())
    }
}

// --- a feed's port -------------------------------------------------------------

/// What a feed's port callbacks share with its task.
#[derive(Default)]
struct PortState {
    credit: Credit,
    closed: bool,
    waker: Option<Waker>,
}

impl PortState {
    /// Runs `f` on the state, then wakes the task outside the borrow.
    fn update(state: &RefCell<PortState>, f: impl FnOnce(&mut PortState)) {
        let waker = {
            let mut s = state.borrow_mut();
            f(&mut s);
            s.waker.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
    }

    fn park(&mut self, waker: &Waker) {
        if !self.waker.as_ref().is_some_and(|w| w.will_wake(waker)) {
            self.waker = Some(waker.clone());
        }
    }
}

/// The node's end of a feed's port: posts batches, takes acks. Dropping it
/// closes the port.
pub struct FeedPort {
    port: MessagePort,
    state: Rc<RefCell<PortState>>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    on_close: Closure<dyn FnMut(Event)>,
}

impl fmt::Debug for FeedPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.state.borrow();
        f.debug_struct("FeedPort")
            .field("unacked", &s.credit.unacked())
            .field("closed", &s.closed)
            .finish_non_exhaustive()
    }
}

impl FeedPort {
    /// Takes over `port`, which starts delivering the worker's acks.
    pub fn new(port: MessagePort) -> FeedPort {
        let state = Rc::new(RefCell::new(PortState::default()));
        let s = state.clone();
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            // Anything but a count is no ack.
            if let Some(n) = e.data().as_f64().filter(|n| *n >= 0.0) {
                // `as` saturates.
                PortState::update(&s, |s| s.credit.ack(n as u64));
            }
        });
        let s = state.clone();
        let on_close = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
            PortState::update(&s, |s| s.closed = true);
        });
        // Browsers without the event never end the feed this way.
        let _ = port.add_event_listener_with_callback("close", on_close.as_ref().unchecked_ref());
        port.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        FeedPort {
            port,
            state,
            _on_message: on_message,
            on_close,
        }
    }

    /// Resolves once another batch may be posted; `false` once the port
    /// closed.
    pub async fn ready(&self) -> bool {
        poll_fn(|cx| {
            let mut s = self.state.borrow_mut();
            if s.closed {
                return Poll::Ready(false);
            }
            if s.credit.is_open() {
                return Poll::Ready(true);
            }
            s.park(cx.waker());
            Poll::Pending
        })
        .await
    }

    /// Resolves once the port closed.
    pub async fn closed(&self) {
        poll_fn(|cx| {
            let mut s = self.state.borrow_mut();
            if s.closed {
                return Poll::Ready(());
            }
            s.park(cx.waker());
            Poll::Pending
        })
        .await
    }

    /// Posts `batch` and empties it. A port that can't take it counts as
    /// closed.
    pub fn post(&self, batch: &mut Batch) {
        let ask = {
            let len = batch.finish(false).len();
            self.state.borrow_mut().credit.post(len)
        };
        self.send(batch.finish(ask));
        batch.clear();
    }

    /// Posts the end batch.
    pub fn end(&self) {
        self.send(&[BATCH_END]);
    }

    /// Bytes and batches posted and not yet acked.
    pub fn unacked(&self) -> (usize, usize) {
        self.state.borrow().credit.unacked()
    }

    fn send(&self, bytes: &[u8]) {
        let buffer = Uint8Array::from(bytes).buffer();
        if let Err(e) = self
            .port
            .post_message_with_transferable(&buffer, &Array::of1(&buffer))
        {
            tracing::debug!(error = %js::describe(&e), "posting a feed batch");
            PortState::update(&self.state, |s| s.closed = true);
        }
    }
}

impl Drop for FeedPort {
    fn drop(&mut self) {
        // No callback may run once its closure is gone.
        self.port.set_onmessage(None);
        let _ = self
            .port
            .remove_event_listener_with_callback("close", self.on_close.as_ref().unchecked_ref());
        self.port.close();
    }
}

// --- feeding -------------------------------------------------------------------

/// A stream's pieces, as [`EgressStream::next`] yields them.
pub trait Pieces {
    /// The next piece; a data piece holds at most `max` bytes (at least 1).
    fn next(&mut self, max: usize) -> impl Future<Output = Result<EgressPiece<'_>, VideoError>>;
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Pieces for EgressStream<K, H, V, T> {
    fn next(&mut self, max: usize) -> impl Future<Output = Result<EgressPiece<'_>, VideoError>> {
        EgressStream::next(self, max)
    }
}

/// Why [`pump`] stopped.
#[derive(Debug, Clone, PartialEq)]
pub enum Stopped {
    /// The stream failed or ended.
    Stream(VideoError),
    /// The port closed.
    Port,
}

impl fmt::Display for Stopped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stopped::Stream(e) => e.fmt(f),
            Stopped::Port => f.write_str("the player's port closed"),
        }
    }
}

/// Feeds `stream` on `port` until either stops, in batches within the
/// port's window; see the [module docs](self#a-feeds-port). The batch in
/// progress is posted before it returns.
pub async fn pump<S: Pieces>(stream: &mut S, port: &FeedPort) -> Stopped {
    let mut batch = Batch::new();
    loop {
        if !port.ready().await {
            return Stopped::Port;
        }
        let mut next = pin!(stream.next(batch.room().max(1)));
        let read = match poll_fn(|cx| Poll::Ready(next.as_mut().poll(cx))).await {
            Poll::Ready(r) => r,
            Poll::Pending => {
                // What the stream had ready goes before the read waits.
                if !batch.is_empty() {
                    port.post(&mut batch);
                }
                match select(next, pin!(port.closed())).await {
                    Either::Left((r, _)) => r,
                    Either::Right(((), _)) => return Stopped::Port,
                }
            }
        };
        match read {
            Ok(piece) => batch.piece(piece),
            Err(e) => {
                if !batch.is_empty() {
                    port.post(&mut batch);
                }
                return Stopped::Stream(e);
            }
        }
        if batch.room() == 0 {
            port.post(&mut batch);
        }
    }
}

// --- the feeds port -------------------------------------------------------------

/// A feed the window opened.
struct Entry {
    /// Which open made it: an id the window reuses names another feed.
    serial: u64,
    task: TaskHandle,
}

/// What the feeds port's handler shares with the feeds.
struct Feeds {
    port: MessagePort,
    /// Bound: [`MAX_FEEDS`].
    open: RefCell<BTreeMap<u64, Entry>>,
    serial: Cell<u64>,
}

impl Feeds {
    /// Posts `{type: "closed", id, message}` to the window.
    fn closed(&self, id: u64, message: &str) {
        let o = Object::new();
        let _ = Reflect::set(&o, &"type".into(), &"closed".into());
        // Ids are at most 2^53.
        let _ = Reflect::set(&o, &"id".into(), &JsValue::from_f64(id as f64));
        let _ = Reflect::set(&o, &"message".into(), &message.into());
        if let Err(e) = self.port.post_message(&o) {
            tracing::warn!(error = %js::describe(&e), "posting on the feeds port");
        }
    }

    /// Closes feed `id`, if it's open.
    fn close(&self, id: u64) {
        let entry = self.open.borrow_mut().remove(&id);
        if let Some(e) = entry {
            e.task.abort();
        }
    }

    /// Feed `id` of `serial` ended: it's forgotten, and the window told
    /// why.
    fn ended(&self, id: u64, serial: u64, message: &str) {
        let mine = {
            let mut open = self.open.borrow_mut();
            let mine = open.get(&id).is_some_and(|e| e.serial == serial);
            if mine {
                open.remove(&id);
            }
            mine
        };
        if mine {
            self.closed(id, message);
        }
    }
}

/// An `open` message's fields.
struct Open {
    id: u64,
    swarm_uri: String,
    network_keys: Vec<Vec<u8>>,
    mime_type: String,
    port: MessagePort,
    replaces: Option<u64>,
}

/// Serves the window's feeds on the feeds port; see the
/// [module docs](self). Dropping it stops taking messages and closes every
/// feed.
pub struct FeedServer {
    feeds: Rc<Feeds>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl fmt::Debug for FeedServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FeedServer")
            .field("open", &self.feeds.open.borrow().len())
            .finish_non_exhaustive()
    }
}

impl Drop for FeedServer {
    fn drop(&mut self) {
        self.feeds.port.set_onmessage(None);
        let open = std::mem::take(&mut *self.feeds.open.borrow_mut());
        for e in open.into_values() {
            e.task.abort();
        }
    }
}

/// Serves the feeds the window opens on `port` from `video`'s streams.
/// Setting the handler starts the port, which delivers what it buffered.
pub fn serve<K: Kv>(video: WasmVideo<K>, port: MessagePort) -> FeedServer {
    let feeds = Rc::new(Feeds {
        port: port.clone(),
        open: RefCell::new(BTreeMap::new()),
        serial: Cell::new(0),
    });
    let f = Rc::downgrade(&feeds);
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
        if let Some(feeds) = f.upgrade() {
            handle(&feeds, &video, &e.data());
        }
    });
    port.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    FeedServer {
        feeds,
        _on_message: on_message,
    }
}

/// Handles one message from the window.
fn handle<K: Kv>(feeds: &Rc<Feeds>, video: &WasmVideo<K>, msg: &JsValue) {
    let kind = property(msg, "type").as_string().unwrap_or_default();
    match kind.as_str() {
        "open" => {
            let id = match id_field(msg, "id") {
                Some(id) => id,
                None => {
                    tracing::warn!("a feed open without an id");
                    return;
                }
            };
            match parse_open(id, msg) {
                Ok(open) => start(feeds, video, open),
                Err(e) => feeds.closed(id, &e),
            }
        }
        "close" => match id_field(msg, "id") {
            Some(id) => feeds.close(id),
            None => tracing::warn!("a feed close without an id"),
        },
        other => tracing::warn!(kind = other, "an unknown feeds message"),
    }
}

/// Starts feed `open`, unless the window's feeds are refused.
fn start<K: Kv>(feeds: &Rc<Feeds>, video: &WasmVideo<K>, open: Open) {
    let refusal = {
        let o = feeds.open.borrow();
        if o.contains_key(&open.id) {
            Some("the feed id is in use")
        } else if o.len() >= MAX_FEEDS {
            Some("too many feeds")
        } else {
            None
        }
    };
    if let Some(r) = refusal {
        open.port.close();
        feeds.closed(open.id, r);
        return;
    }
    let serial = feeds.serial.get() + 1;
    feeds.serial.set(serial);
    let id = open.id;
    let task = runtime::spawn(feed(feeds.clone(), video.clone(), serial, open));
    feeds.open.borrow_mut().insert(id, Entry { serial, task });
}

/// Runs one feed to its end.
async fn feed<K: Kv>(feeds: Rc<Feeds>, video: WasmVideo<K>, serial: u64, open: Open) {
    let port = FeedPort::new(open.port);
    let opened = video
        .open_stream(
            &open.swarm_uri,
            &open.network_keys,
            StreamFormat::from_mime(&open.mime_type),
        )
        .await;
    if let Some(old) = open.replaces.filter(|r| *r != open.id) {
        feeds.close(old);
    }
    let stopped = match opened {
        Ok(mut stream) => pump(&mut stream, &port).await,
        Err(e) => Stopped::Stream(e),
    };
    port.end();
    feeds.ended(open.id, serial, &stopped.to_string());
}

fn parse_open(id: u64, msg: &JsValue) -> Result<Open, String> {
    let string = |key: &str| {
        property(msg, key)
            .as_string()
            .ok_or_else(|| format!("{key}: not a string"))
    };
    let port = property(msg, "port")
        .dyn_into::<MessagePort>()
        .map_err(|_| "port: not a MessagePort".to_owned())?;
    let keys = property(msg, "networkKeys")
        .dyn_into::<Array>()
        .map_err(|_| "networkKeys: not an array".to_owned())?;
    let mut network_keys = Vec::with_capacity(keys.length() as usize);
    for k in keys.iter() {
        let k = k
            .dyn_into::<Uint8Array>()
            .map_err(|_| "networkKeys: not byte arrays".to_owned())?;
        network_keys.push(k.to_vec());
    }
    let replaces = match property(msg, "replaces") {
        v if v.is_undefined() || v.is_null() => None,
        _ => Some(id_of(&property(msg, "replaces")).ok_or("replaces: not an id")?),
    };
    Ok(Open {
        id,
        swarm_uri: string("swarmUri")?,
        network_keys,
        mime_type: string("mimeType")?,
        port,
        replaces,
    })
}

fn property(v: &JsValue, key: &str) -> JsValue {
    if !v.is_object() {
        return JsValue::UNDEFINED;
    }
    Reflect::get(v, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
}

fn id_field(v: &JsValue, key: &str) -> Option<u64> {
    id_of(&property(v, key))
}

/// A non-negative integer of at most 2^53.
fn id_of(v: &JsValue) -> Option<u64> {
    match v.as_f64() {
        Some(n) if n.fract() == 0.0 && (0.0..=9_007_199_254_740_992.0).contains(&n) => {
            Some(n as u64)
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "feed_test.rs"]
mod tests;
