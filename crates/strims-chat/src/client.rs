// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Client streams ([`ClientStream`]): a room's asset bundles and events,
//! read from its server's swarms through the bounded readers of
//! [`reader`](crate::reader).

use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use futures_core::Stream;
use futures_util::future::LocalBoxFuture;
use futures_util::stream::{LocalBoxStream, unfold};
use futures_util::{FutureExt, StreamExt};
use prost::Message;
use strims_base::time::Duration;
use strims_directory::Host;
use strims_kv::Kv;
use strims_proto::chat::v1::{AssetBundle, ServerEvent};
use strims_rpc::Timer;
use strims_transfer::{ReadFailure, RecordReader, SwarmHost, SwarmRead};

use crate::control::{Runner, Shared, Swarm};
use crate::events::ClientEvent;
use crate::reader::{AssetReader, EventReader, ReaderMetrics};
use crate::{ASSET_FRAME, EVENT_FRAME};

/// The wait after a failed swarm read before reading again.
const READ_RETRY: Duration = Duration::SECOND;

/// The records a stream decodes in one poll before it yields to other
/// tasks.
const POLL_BUDGET: usize = 256;

type Reader<H> = RecordReader<<H as SwarmHost>::Reader>;
/// Opening a stream's readers on a runner's event and asset swarms.
type OpenReaders<H> = LocalBoxFuture<'static, Result<(Reader<H>, Reader<H>), String>>;

/// What a client stream's readers have yielded and dropped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClientMetrics {
    pub events: ReaderMetrics,
    pub assets: ReaderMetrics,
    /// Times the stream opened readers on its runner's swarms.
    pub opens: u64,
}

/// The decoders of a stream, kept across its readers.
struct Decoders {
    events: EventReader,
    assets: AssetReader,
    opens: u64,
}

/// What a record reader gave.
enum Read<T> {
    Item(T),
    /// A record the decoder dropped, or a discontinuity.
    Skipped,
    /// The swarm closed.
    Closed,
}

enum State<H: Host> {
    /// Waiting for the runner's next swarms.
    Waiting,
    Opening(OpenReaders<H>),
    Reading {
        events: LocalBoxStream<'static, Read<ServerEvent>>,
        assets: LocalBoxStream<'static, Read<AssetBundle>>,
    },
}

/// A client's stream of a chat room, from
/// [`ChatControl::open_client`](crate::ChatControl::open_client): its
/// asset bundles and events, as [`ClientEvent`]s.
///
/// The stream opens its own record reader on each of the swarms the room's
/// runner holds, starting at the swarms' window tails: the asset stream
/// from its last unified bundle on, the event stream with its recent
/// history. When the runner's mode changes, its swarms close and the
/// stream reopens its readers on the next mode's, after a discontinuity;
/// the asset reader then drops deltas until a unified bundle. A record
/// that fails the room's bounds ([`RoomLimits`](crate::reader::RoomLimits))
/// is dropped and counted ([`ClientStream::metrics`]).
///
/// It ends when the server's runner stops. Dropping it closes it.
pub struct ClientStream<K: Kv, H: Host, T: Timer> {
    shared: Rc<Shared<K, H, T>>,
    runner: Rc<Runner<H>>,
    slot: u64,
    /// The generation of the runner's swarms the readers are open on.
    generation: u64,
    state: State<H>,
    decoders: Rc<RefCell<Decoders>>,
}

impl<K: Kv, H: Host, T: Timer> ClientStream<K, H, T> {
    pub(crate) fn new(shared: Rc<Shared<K, H, T>>, runner: Rc<Runner<H>>, slot: u64) -> Self {
        let room = shared.config.room;
        ClientStream {
            shared,
            runner,
            slot,
            generation: 0,
            state: State::Waiting,
            decoders: Rc::new(RefCell::new(Decoders {
                events: EventReader::new(room),
                assets: AssetReader::new(room),
                opens: 0,
            })),
        }
    }

    /// The server's public key.
    pub fn server_key(&self) -> &[u8] {
        &self.runner.key
    }

    /// The key of the network the server is read on.
    pub fn network_key(&self) -> &[u8] {
        &self.runner.network_key
    }

    /// What the stream's readers have yielded and dropped.
    pub fn metrics(&self) -> ClientMetrics {
        let d = self.decoders.borrow();
        ClientMetrics {
            events: *d.events.metrics(),
            assets: *d.assets.metrics(),
            opens: d.opens,
        }
    }

    /// Registers the task to wake when the runner's swarms change.
    fn wait_for_swarms(&self, cx: &Context<'_>) {
        let prev = self
            .runner
            .streams
            .borrow_mut()
            .get_mut(&self.slot)
            .and_then(|w| w.replace(cx.waker().clone()));
        drop(prev);
    }
}

/// Opens a record reader on each swarm.
fn open<K: Kv, H: Host, T: Timer>(
    s: Rc<Shared<K, H, T>>,
    events: Swarm<H>,
    assets: Swarm<H>,
) -> OpenReaders<H> {
    async move {
        let err = |e: &dyn std::fmt::Display| e.to_string();
        let room = s.config.room;
        let e = s.host.open_reader(events).await.map_err(|e| err(&e))?;
        let e = RecordReader::new(e, EVENT_FRAME, room.max_event_record).map_err(|e| err(&e))?;
        let a = s.host.open_reader(assets).await.map_err(|e| err(&e))?;
        let a = RecordReader::new(a, ASSET_FRAME, room.max_asset_record).map_err(|e| err(&e))?;
        Ok((e, a))
    }
    .boxed_local()
}

/// The records of `reader`, decoded by `decode`. A discontinuity is passed
/// to `discontinuity`, and a failed read is retried after
/// [`READ_RETRY`]. A reader is dropped with the stream, never between a
/// read and its decoding.
fn records<R: SwarmRead, T: Timer, M: 'static>(
    reader: RecordReader<R>,
    timer: T,
    decoders: Rc<RefCell<Decoders>>,
    decode: fn(&mut Decoders, &[u8]) -> Option<M>,
    discontinuity: fn(&mut Decoders),
) -> LocalBoxStream<'static, Read<M>> {
    unfold(Some(reader), move |reader| {
        let timer = timer.clone();
        let decoders = decoders.clone();
        async move {
            let mut reader = reader?;
            loop {
                let res = reader
                    .next()
                    .await
                    .map(|record| decode(&mut decoders.borrow_mut(), record));
                match res {
                    Ok(m) => {
                        let read = m.map_or(Read::Skipped, Read::Item);
                        return Some((read, Some(reader)));
                    }
                    Err(ReadFailure::Discontinuity) => {
                        discontinuity(&mut decoders.borrow_mut());
                        return Some((Read::Skipped, Some(reader)));
                    }
                    Err(ReadFailure::Closed) => return Some((Read::Closed, None)),
                    Err(ReadFailure::Failed(e)) => {
                        tracing::debug!(error = %e, "reading a chat swarm failed");
                        timer.sleep_until(timer.now() + READ_RETRY).await;
                    }
                }
            }
        }
    })
    .boxed_local()
}

/// The stream's batch so far, or `Pending` without one.
fn flush(batch: &mut Vec<ServerEvent>) -> Poll<Option<ClientEvent>> {
    if batch.is_empty() {
        Poll::Pending
    } else {
        Poll::Ready(Some(ClientEvent::Events(std::mem::take(batch))))
    }
}

impl<K: Kv, H: Host, T: Timer> Stream for ClientStream<K, H, T> {
    type Item = ClientEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<ClientEvent>> {
        let this = self.get_mut();
        let max_events = this.shared.config.max_batch_events.max(1);
        let max_bytes = this.shared.config.max_batch_bytes;
        let mut batch: Vec<ServerEvent> = Vec::new();
        let mut bytes = 0;
        let mut budget = POLL_BUDGET;
        loop {
            match &mut this.state {
                State::Waiting => {
                    if this.runner.stopped.get() {
                        return match flush(&mut batch) {
                            Poll::Pending => Poll::Ready(None),
                            ready => ready,
                        };
                    }
                    match this.runner.swarms.get() {
                        Some((events, assets))
                            if this.runner.generation.get() != this.generation =>
                        {
                            this.generation = this.runner.generation.get();
                            this.state = State::Opening(open(this.shared.clone(), events, assets));
                        }
                        _ => {
                            this.wait_for_swarms(cx);
                            return flush(&mut batch);
                        }
                    }
                }
                State::Opening(f) => match f.as_mut().poll(cx) {
                    Poll::Pending => return flush(&mut batch),
                    Poll::Ready(Ok((events, assets))) => {
                        {
                            let mut d = this.decoders.borrow_mut();
                            d.events.discontinuity();
                            d.assets.discontinuity();
                            d.opens += 1;
                        }
                        let timer = this.shared.timer.clone();
                        let d = this.decoders.clone();
                        this.state = State::Reading {
                            events: records(
                                events,
                                timer.clone(),
                                d.clone(),
                                |d, r| d.events.handle_record(r),
                                |d| d.events.discontinuity(),
                            ),
                            assets: records(
                                assets,
                                timer,
                                d,
                                |d, r| d.assets.handle_record(r),
                                |d| d.assets.discontinuity(),
                            ),
                        };
                    }
                    Poll::Ready(Err(e)) => {
                        this.shared.metrics.borrow_mut().host_errors += 1;
                        tracing::debug!(error = %e, "opening chat readers failed");
                        this.state = State::Waiting;
                    }
                },
                State::Reading { events, assets } => {
                    if budget == 0 {
                        cx.waker().wake_by_ref();
                        return flush(&mut batch);
                    }
                    budget -= 1;
                    if batch.is_empty() {
                        match assets.poll_next_unpin(cx) {
                            Poll::Ready(Some(Read::Item(b))) => {
                                return Poll::Ready(Some(ClientEvent::Assets(b)));
                            }
                            Poll::Ready(Some(Read::Skipped)) => continue,
                            Poll::Ready(Some(Read::Closed) | None) => {
                                this.state = State::Waiting;
                                continue;
                            }
                            Poll::Pending => {}
                        }
                    }
                    match events.poll_next_unpin(cx) {
                        Poll::Ready(Some(Read::Item(e))) => {
                            bytes += e.encoded_len();
                            batch.push(e);
                            if batch.len() >= max_events || bytes >= max_bytes {
                                return flush(&mut batch);
                            }
                        }
                        Poll::Ready(Some(Read::Skipped)) => {}
                        Poll::Ready(Some(Read::Closed) | None) => this.state = State::Waiting,
                        Poll::Pending => return flush(&mut batch),
                    }
                }
            }
        }
    }
}

impl<K: Kv, H: Host, T: Timer> Drop for ClientStream<K, H, T> {
    fn drop(&mut self) {
        let removed = self.runner.streams.borrow_mut().remove(&self.slot);
        drop(removed);
        if self.runner.streams.borrow().is_empty() && !self.runner.stopped.get() {
            let until = self.shared.timer.now() + self.shared.config.linger;
            self.runner.linger_until.set(Some(until));
            self.runner.poke();
            self.shared.wake_run();
        }
    }
}
