// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A fake [`VideoHost`] whose listener takes connections the test makes,
//! each a pair of in-memory pipes, and an RTMP publisher driven over one.

#![allow(dead_code, clippy::unwrap_used, clippy::panic)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use futures_core::Stream;
use futures_sink::Sink;
use futures_util::{FutureExt, StreamExt};
use strims_base::rng::DetRng;
use strims_testworld::{Closed, Pipe, Reader, Writer};
use strims_video::amf0::Value;
use strims_video::synth::publisher::{Publisher, PublisherConfig};
use strims_video::synth::script::{MetaValue, Script, Tag, TagKind};
use strims_video::{Accepted, VideoHost};

pub mod codec;

#[derive(Default)]
struct ListenerState {
    addr: String,
    queue: VecDeque<Accepted<ConnReader, ConnWriter>>,
    waker: Option<Waker>,
    closed: bool,
}

#[derive(Default)]
pub struct FakeVideoState {
    /// The codec stage, if the host has one.
    pub codec: Option<codec::FakeCodec>,
    /// Every address `listen` was called with, in order.
    pub listens: Vec<String>,
    /// Listens to fail before the next succeeds.
    pub fail_listens: usize,
    listener: Option<Rc<RefCell<ListenerState>>>,
}

/// A host whose listener takes the connections [`FakeVideo::connect`]
/// makes. Clones share it.
#[derive(Clone, Default)]
pub struct FakeVideo(pub Rc<RefCell<FakeVideoState>>);

impl FakeVideo {
    /// The address of the open listener.
    pub fn listening(&self) -> Option<String> {
        let s = self.0.borrow();
        let l = s.listener.as_ref()?.borrow();
        (!l.closed).then(|| l.addr.clone())
    }

    pub fn listens(&self) -> Vec<String> {
        self.0.borrow().listens.clone()
    }

    /// Connects to the open listener from `peer`: the client's ends, or
    /// `None` without a listener.
    pub fn connect(&self, peer: &str) -> Option<ClientConn> {
        let l = self.0.borrow().listener.clone()?;
        let (up, down) = (Pipe::default(), Pipe::default());
        let waker = {
            let mut l = l.borrow_mut();
            if l.closed {
                return None;
            }
            l.queue.push_back(Accepted {
                reader: ConnReader(Reader(up.clone())),
                writer: ConnWriter(Writer(down.clone())),
                peer: peer.to_owned(),
            });
            l.waker.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
        Some(ClientConn {
            up: Writer(up),
            down: Reader(down),
        })
    }
}

/// The listener: dropping it closes it.
pub struct FakeListener(Rc<RefCell<ListenerState>>);

impl Stream for FakeListener {
    type Item = Result<Accepted<ConnReader, ConnWriter>, Closed>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut l = self.0.borrow_mut();
        if let Some(a) = l.queue.pop_front() {
            return Poll::Ready(Some(Ok(a)));
        }
        l.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Drop for FakeListener {
    fn drop(&mut self) {
        let queued = {
            let mut l = self.0.borrow_mut();
            l.closed = true;
            std::mem::take(&mut l.queue)
        };
        drop(queued);
    }
}

/// What the server reads; dropping it closes the client's sends.
pub struct ConnReader(Reader);

impl Stream for ConnReader {
    type Item = Result<Vec<u8>, Closed>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.0).poll_next(cx)
    }
}

impl Drop for ConnReader {
    fn drop(&mut self) {
        self.0.0.close();
    }
}

/// What the server sends; dropping it ends the client's reads.
pub struct ConnWriter(Writer);

impl Sink<Vec<u8>> for ConnWriter {
    type Error = Closed;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Pin::new(&mut self.0).poll_ready(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Closed> {
        Pin::new(&mut self.0).start_send(item)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Pin::new(&mut self.0).poll_close(cx)
    }
}

impl Drop for ConnWriter {
    fn drop(&mut self) {
        self.0.0.close();
    }
}

impl VideoHost for FakeVideo {
    type Error = Closed;
    type Reader = ConnReader;
    type Writer = ConnWriter;
    type Listener = FakeListener;

    fn rtmp_supported(&self) -> bool {
        true
    }

    fn listen(&self, addr: &str) -> impl Future<Output = Result<FakeListener, Closed>> {
        let mut s = self.0.borrow_mut();
        s.listens.push(addr.to_owned());
        let res = if s.fail_listens > 0 {
            s.fail_listens -= 1;
            Err(Closed)
        } else {
            let l = Rc::new(RefCell::new(ListenerState {
                addr: addr.to_owned(),
                ..ListenerState::default()
            }));
            s.listener = Some(l.clone());
            Ok(FakeListener(l))
        };
        std::future::ready(res)
    }

    type Codec = codec::FakeCodec;

    fn codec(&self) -> Option<Self::Codec> {
        self.0.borrow().codec.clone()
    }
}

/// The client's ends of a connection.
pub struct ClientConn {
    pub up: Writer,
    pub down: Reader,
}

impl ClientConn {
    /// Whether the server closed the connection: its sends ended. Takes
    /// what it sent meanwhile.
    pub fn closed(&mut self) -> bool {
        loop {
            match self.down.next().now_or_never() {
                Some(Some(_)) => {}
                Some(None) => return true,
                None => return false,
            }
        }
    }
}

/// An RTMP publisher on a connection.
pub struct TestPublisher {
    pub publisher: Publisher,
    pub conn: ClientConn,
    /// The server ended the connection.
    pub closed: bool,
}

impl TestPublisher {
    pub fn new(conn: ClientConn, app: &str, key: &str, rng: &mut DetRng) -> TestPublisher {
        TestPublisher {
            publisher: Publisher::new(PublisherConfig::new(app, key), rng),
            conn,
            closed: false,
        }
    }

    /// Connects to `video`'s listener from `peer` and publishes `key` to
    /// `app`, without waiting.
    pub fn connect(video: &FakeVideo, peer: &str, app: &str, key: &str, rng: &mut DetRng) -> Self {
        let conn = video.connect(peer).expect("a listener");
        TestPublisher::new(conn, app, key, rng)
    }

    /// Sends what the publisher queued. A pipe takes every send at once;
    /// a closed one drops it.
    pub async fn flush(&mut self) {
        let mut buf = vec![0; 64 * 1024];
        loop {
            let n = self.publisher.poll_transmit(&mut buf);
            if n == 0 {
                return;
            }
            let _ = Pin::new(&mut self.conn.up).start_send(buf[..n].to_vec());
        }
    }

    /// Takes what the server sent, answering it.
    pub fn pump(&mut self) {
        loop {
            match self.conn.down.next().now_or_never() {
                Some(Some(Ok(b))) => self.publisher.handle_input(&b).unwrap(),
                Some(Some(Err(_))) | Some(None) => {
                    self.closed = true;
                    return;
                }
                None => return,
            }
        }
    }

    /// Exchanges messages with the server until `cond` holds, within 10
    /// seconds.
    pub async fn until(&mut self, what: &str, cond: impl Fn(&TestPublisher) -> bool) {
        for _ in 0..1000 {
            self.flush().await;
            self.pump();
            if cond(self) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("timed out waiting for {what}");
    }

    /// Publishes until the server accepts or refuses.
    pub async fn answered(&mut self) {
        self.until("the publish's answer", |p| {
            p.publisher.publish_accepted() || p.publisher.refused().is_some() || p.closed
        })
        .await;
    }

    /// The server's refusal, once it closed the connection.
    pub async fn refusal(&mut self) -> (String, String) {
        self.until("the server to close", |p| p.closed).await;
        let (code, description) = self.publisher.refused().expect("a refusal");
        (code.to_owned(), description.to_owned())
    }

    /// Sends `tag`.
    pub fn send(&mut self, tag: &Tag, script: &Script) {
        match tag.kind {
            TagKind::Video => self.publisher.send_video(tag.timestamp, &tag.body).unwrap(),
            TagKind::Audio => self.publisher.send_audio(tag.timestamp, &tag.body).unwrap(),
            TagKind::Metadata => {
                let fields: Vec<(&str, Value<'_>)> = script
                    .metadata
                    .iter()
                    .map(|(k, v)| {
                        let v = match v {
                            MetaValue::Number(n) => Value::Number(*n),
                            MetaValue::Bool(b) => Value::Boolean(*b),
                            MetaValue::String(s) => Value::String(s),
                        };
                        (k.as_str(), v)
                    })
                    .collect();
                self.publisher.send_metadata(&fields).unwrap();
            }
        }
    }

    /// Sends `tags` of `script`, a few at a time, letting the server take
    /// each batch.
    pub async fn send_tags(&mut self, tags: &[Tag], script: &Script) {
        for batch in tags.chunks(8) {
            for tag in batch {
                self.send(tag, script);
            }
            self.flush().await;
            self.pump();
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }

    /// Ends the publish.
    pub async fn unpublish(&mut self) {
        self.publisher.unpublish().unwrap();
        self.flush().await;
    }
}
