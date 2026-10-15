// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Link I/O tasks: one reader and one writer per transport connection.
//!
//! They own the socket halves and nothing else. Buffers circulate between
//! them and the node task over bounded channels and are reused for the
//! link's lifetime:
//!
//! ```text
//!   reader task --Read{buf}--> node --read_free_tx--> reader task
//!   node --write_tx{buf}--> writer task --WriteDone{buf}--> node
//! ```
//!
//! A link holds at most `read_buffers` read buffers and `write_buffers`
//! write buffers, so its memory is fixed. A slow socket keeps the write
//! buffers in its writer task; vnic's link buffer then fills and QoS stops
//! scheduling the link's writers (backpressure to the owners).
//!
//! A read waiting longer than the read timeout (an idle timeout, renewed
//! before every read) or a write taking longer than the write timeout
//! fails the link. WebSocket links also ping every `ping_interval`, and any
//! message, pongs included, renews the read deadline.
//!
//! An idle TCP link is silent unless its peer sends vnic keepalives. While
//! the node task marks a TCP link [`IdleOk`] (an established peer that
//! hasn't sent a keepalive), a read timeout re-arms and the link stays up;
//! TCP keepalive finds a dead peer.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration as StdDuration;

use bytes::Bytes;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use strims_base::time::Duration;
use strims_vnic::{LinkConfig, PeerKey as LinkKey};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

use crate::net::Net;
use crate::state::DialId;

/// A connected transport, ready for vnic.
///
/// TCP and WebSocket links are byte streams with their own I/O tasks.
/// WebRTC links have none: str0m is sans-IO, so its peer connections live
/// in the node task next to vnic (`state/rtc.rs`), and a data channel
/// moves vnic link bytes in-process. Datagrams go through one reader and
/// one writer task per UDP socket (`udp.rs`).
pub(crate) enum Transport<N: Net> {
    Tcp(N::TcpStream),
    /// Boxed: tungstenite's state is large and a link is built once.
    Ws(Box<WebSocketStream<N::TcpStream>>),
}

/// Everything the node task hears from transport tasks.
pub(crate) enum Input<N: Net> {
    /// `buf[..len]` was read from the link.
    Read {
        link: LinkKey,
        buf: Vec<u8>,
        len: usize,
    },
    /// The writer task wrote `buf` and returns it.
    WriteDone {
        link: LinkKey,
        buf: Vec<u8>,
    },
    /// The transport failed or the peer closed it.
    LinkDown {
        link: LinkKey,
        error: String,
    },
    /// A dial or an accepted connection finished its transport handshake
    /// (TCP mux init, WebSocket upgrade). `leftover` was read past it.
    Connected {
        dial: Option<DialId>,
        transport: Transport<N>,
        config: LinkConfig,
        leftover: Vec<u8>,
        /// The socket's `TCP_NOTSENT_LOWAT`, when set: the link's writes
        /// are cut to it.
        notsent_lowat: Option<u32>,
    },
    DialFailed {
        dial: DialId,
        error: String,
    },
    /// An accepted connection was dropped before vnic.
    AcceptFailed {
        error: String,
    },
    /// `buf[..len]` arrived from `from` on WebRTC UDP socket `socket`.
    Datagram {
        socket: usize,
        buf: Vec<u8>,
        len: usize,
        from: SocketAddr,
    },
    /// The WebRTC interface's STUN servers, (re)resolved.
    StunServers(Vec<SocketAddr>),
}

/// Runs `f` with a timeout; a non-positive duration means none. `None` on
/// timeout.
pub(crate) async fn with_timeout<F: Future>(d: Duration, f: F) -> Option<F::Output> {
    match d.to_std() {
        Some(d) if !d.is_zero() => tokio::time::timeout(d, f).await.ok(),
        _ => Some(f.await),
    }
}

// TODO: remove once protocol v3 makes keepalives mandatory.
/// Whether a link may stay silent past its read timeout, shared by the node
/// task (which sets it) and the link's reader (which reads it when a read
/// times out). Set for an established peer that hasn't sent a vnic
/// keepalive yet. A peer that sends keepalives writes at least every
/// keepalive interval (10 s), so for it the read timeout applies again.
#[derive(Clone, Debug, Default)]
pub(crate) struct IdleOk(Arc<AtomicBool>);

impl IdleOk {
    pub(crate) fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub(crate) fn set(&self, v: bool) {
        self.0.store(v, Ordering::Relaxed);
    }
}

/// Reads a byte stream (TCP) into recycled buffers. A read that times out
/// while `idle_ok` is set is retried (`read` is cancel safe).
pub(crate) async fn stream_reader<N: Net, R: AsyncRead + Unpin>(
    link: LinkKey,
    mut r: R,
    mut free: mpsc::Receiver<Vec<u8>>,
    input: mpsc::Sender<Input<N>>,
    read_timeout: Duration,
    idle_ok: IdleOk,
) {
    let error = loop {
        let Some(mut buf) = free.recv().await else {
            return;
        };
        let res = loop {
            match with_timeout(read_timeout, r.read(&mut buf)).await {
                None if idle_ok.get() => {}
                res => break res,
            }
        };
        match res {
            None => break "read timeout".to_owned(),
            Some(Err(e)) => break e.to_string(),
            Some(Ok(0)) => break "connection closed".to_owned(),
            Some(Ok(len)) => {
                if input.send(Input::Read { link, buf, len }).await.is_err() {
                    return;
                }
            }
        }
    };
    let _ = input.send(Input::LinkDown { link, error }).await;
}

/// Writes a byte stream (TCP). Ends with a shutdown (FIN) when the node
/// drops the link.
pub(crate) async fn stream_writer<N: Net, W: AsyncWrite + Unpin>(
    link: LinkKey,
    mut w: W,
    mut bufs: mpsc::Receiver<Vec<u8>>,
    input: mpsc::Sender<Input<N>>,
    write_timeout: Duration,
) {
    while let Some(buf) = bufs.recv().await {
        let error = match with_timeout(write_timeout, w.write_all(&buf)).await {
            Some(Ok(())) => None,
            Some(Err(e)) => Some(e.to_string()),
            None => Some("write timeout".to_owned()),
        };
        if let Some(error) = error {
            let _ = input.send(Input::LinkDown { link, error }).await;
            return;
        }
        if input.send(Input::WriteDone { link, buf }).await.is_err() {
            return;
        }
    }
    let _ = with_timeout(write_timeout, w.shutdown()).await;
}

/// Reads WebSocket binary messages and concatenates their payloads: the
/// link is a byte stream, and message boundaries need not match frames. A
/// text message fails the link. Any message, pongs included, renews the
/// read deadline.
pub(crate) async fn ws_reader<N: Net, S: AsyncRead + AsyncWrite + Unpin>(
    link: LinkKey,
    mut stream: SplitStream<WebSocketStream<S>>,
    mut free: mpsc::Receiver<Vec<u8>>,
    input: mpsc::Sender<Input<N>>,
    read_timeout: Duration,
) {
    let error = loop {
        let msg = match with_timeout(read_timeout, stream.next()).await {
            None => break "read timeout".to_owned(),
            Some(None) => break "connection closed".to_owned(),
            Some(Some(Err(e))) => break e.to_string(),
            Some(Some(Ok(m))) => m,
        };
        let data = match msg {
            Message::Binary(b) => b,
            Message::Text(_) => break "unexpected text message".to_owned(),
            Message::Close(_) => break "connection closed".to_owned(),
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
        };
        let mut rest: &[u8] = &data;
        while !rest.is_empty() {
            let Some(mut buf) = free.recv().await else {
                return;
            };
            let len = buf.len().min(rest.len());
            let (head, tail) = rest.split_at(len);
            if let Some(dst) = buf.get_mut(..len) {
                dst.copy_from_slice(head);
            }
            rest = tail;
            if input.send(Input::Read { link, buf, len }).await.is_err() {
                return;
            }
        }
    };
    let _ = input.send(Input::LinkDown { link, error }).await;
}

/// Writes each buffer as one binary message (at most one link MTU: several
/// frames, or part of one) and pings every `ping_interval`. Ends with a
/// close frame when the node drops the link.
pub(crate) async fn ws_writer<N: Net, S: AsyncRead + AsyncWrite + Unpin>(
    link: LinkKey,
    mut sink: SplitSink<WebSocketStream<S>, Message>,
    mut bufs: mpsc::Receiver<Vec<u8>>,
    input: mpsc::Sender<Input<N>>,
    write_timeout: Duration,
    ping_interval: Duration,
) {
    let mut ping = ping_interval.to_std().filter(|d| !d.is_zero()).map(|d| {
        let mut i = tokio::time::interval_at(tokio::time::Instant::now() + d, d);
        i.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        i
    });
    loop {
        let next = tokio::select! {
            biased;
            buf = bufs.recv() => buf.map(|b| (Message::Binary(Bytes::copy_from_slice(&b)), Some(b))),
            () = tick(&mut ping) => Some((Message::Ping(Bytes::new()), None)),
        };
        let Some((msg, buf)) = next else {
            let _ = with_timeout(write_timeout, sink.close()).await;
            return;
        };
        let error = match with_timeout(write_timeout, sink.send(msg)).await {
            Some(Ok(())) => None,
            Some(Err(e)) => Some(e.to_string()),
            None => Some("write timeout".to_owned()),
        };
        if let Some(error) = error {
            let _ = input.send(Input::LinkDown { link, error }).await;
            return;
        }
        if let Some(buf) = buf
            && input.send(Input::WriteDone { link, buf }).await.is_err()
        {
            return;
        }
    }
}

async fn tick(i: &mut Option<tokio::time::Interval>) {
    match i {
        Some(i) => {
            i.tick().await;
        }
        None => std::future::pending().await,
    }
}

/// The tungstenite limits for vnic links: messages and frames of at most
/// one WebSocket MTU, and no write batching (the writer flushes every
/// message).
pub(crate) fn ws_config() -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
    tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default()
        .read_buffer_size(strims_vnic::ws::WS_MTU)
        .write_buffer_size(0)
        .max_message_size(Some(strims_vnic::ws::WS_MTU))
        .max_frame_size(Some(strims_vnic::ws::WS_MTU))
}

/// A short pause after a failed accept, so a listener error doesn't spin.
pub(crate) const ACCEPT_BACKOFF: StdDuration = StdDuration::from_millis(50);

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use slotmap::KeyData;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;
    use crate::net::TokioNet;

    fn link() -> LinkKey {
        LinkKey::from(KeyData::from_ffi(1))
    }

    /// The most bytes between the writer task's input and a reader that
    /// drains 4 KiB per millisecond (about 4 MB/s), when the node feeds the
    /// writer two buffers of `write` bytes as fast as it returns them, on
    /// a socket with this send-queue bound. Also returns the receiver's
    /// buffer size.
    #[cfg(target_os = "linux")]
    async fn slow_reader_in_flight(notsent_lowat: Option<u32>, write: usize) -> (usize, usize) {
        use std::sync::atomic::AtomicUsize;

        use strims_vnic::TcpOptions;

        use crate::config::TcpSocketConfig;

        const TOTAL: usize = 1 << 20;
        let listener =
            socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None).unwrap();
        listener.set_recv_buffer_size(32 * 1024).unwrap();
        let any: SocketAddr = "127.0.0.1:0".parse().unwrap();
        listener.bind(&any.into()).unwrap();
        listener.listen(1).unwrap();
        listener.set_nonblocking(true).unwrap();
        let listener = tokio::net::TcpListener::from_std(listener.into()).unwrap();
        let addr = listener.local_addr().unwrap();
        let net = TokioNet;
        let (sender, accepted) = tokio::join!(net.tcp_connect(addr.to_string()), listener.accept());
        let sender = sender.unwrap();
        let (mut receiver, _) = accepted.unwrap();
        net.configure_tcp(&sender, &TcpOptions::default()).unwrap();
        let config = TcpSocketConfig {
            notsent_lowat,
            congestion: None,
        };
        assert_eq!(
            net.configure_tcp_send_queue(&sender, &config).unwrap(),
            notsent_lowat
        );
        let rcvbuf = socket2::SockRef::from(&receiver)
            .recv_buffer_size()
            .unwrap();

        let read = Arc::new(AtomicUsize::new(0));
        let reader = tokio::spawn({
            let read = read.clone();
            async move {
                let mut buf = vec![0u8; 4096];
                while read.load(Ordering::Relaxed) < TOTAL {
                    let n = receiver.read(&mut buf).await.unwrap();
                    assert!(n > 0, "closed early");
                    read.fetch_add(n, Ordering::Relaxed);
                    tokio::time::sleep(StdDuration::from_millis(1)).await;
                }
            }
        });

        let (_, w) = tokio::io::split(sender);
        let (write_tx, write_rx) = mpsc::channel(2);
        let (input_tx, mut input) = mpsc::channel::<Input<TokioNet>>(4);
        tokio::spawn(stream_writer(
            link(),
            w,
            write_rx,
            input_tx,
            Duration::from_secs(60),
        ));
        let mut handed = 0;
        let mut max = 0;
        for _ in 0..2 {
            write_tx.send(vec![7u8; write]).await.unwrap();
            handed += write;
        }
        while handed < TOTAL {
            let Some(Input::WriteDone { buf, .. }) = input.recv().await else {
                panic!("writer failed");
            };
            max = max.max(handed - read.load(Ordering::Relaxed));
            write_tx.send(buf).await.unwrap();
            handed += write;
        }
        drop(write_tx);
        reader.await.unwrap();
        (max, rcvbuf)
    }

    /// With the default send-queue bound, a slow reader leaves only a few
    /// writes' worth of bytes between the writer and the receiver's
    /// buffer, so data waits in QoS. Without it, the kernel queues up to
    /// its send buffer (megabytes) in FIFO order.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn a_slow_reader_leaves_few_bytes_in_the_kernel() {
        use crate::config::DEFAULT_NOTSENT_LOWAT;

        let lowat = DEFAULT_NOTSENT_LOWAT as usize;
        let (bounded, rcvbuf) = slow_reader_in_flight(Some(DEFAULT_NOTSENT_LOWAT), lowat).await;
        let (unbounded, _) = slow_reader_in_flight(None, strims_vnic::tcp::TCP_MTU).await;
        // The receiver's buffer, the mark, a write past it, the writer's
        // two buffers, and a write's worth of slack.
        let limit = rcvbuf + 5 * lowat;
        eprintln!(
            "in flight to a 4 MB/s reader: {bounded} B with TCP_NOTSENT_LOWAT {lowat} \
             (limit {limit}, receive buffer {rcvbuf}), {unbounded} B without"
        );
        assert!(bounded <= limit, "{bounded} > {limit}");
        // A system-wide mark (net.ipv4.tcp_notsent_lowat) bounds the second
        // run too.
        let system = std::fs::read_to_string("/proc/sys/net/ipv4/tcp_notsent_lowat")
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);
        if system >= 1 << 20 {
            assert!(unbounded > 2 * bounded, "{unbounded} vs {bounded}");
        }
    }

    /// A silent TCP link outlives its read timeout while `IdleOk` is set,
    /// still delivers bytes, and fails on the next timeout once cleared.
    #[tokio::test(start_paused = true)]
    async fn idle_ok_outwaits_the_read_timeout() {
        let (local, mut remote) = tokio::io::duplex(1024);
        let (free_tx, free_rx) = mpsc::channel(2);
        free_tx.try_send(vec![0; 64]).unwrap();
        let (input_tx, mut input) = mpsc::channel::<Input<TokioNet>>(4);
        let idle_ok = IdleOk::default();
        idle_ok.set(true);
        tokio::spawn(stream_reader(
            link(),
            local,
            free_rx,
            input_tx,
            Duration::from_secs(25),
            idle_ok.clone(),
        ));

        // A minute of silence: nothing, not even a timeout.
        tokio::time::sleep(StdDuration::from_secs(60)).await;
        assert!(input.try_recv().is_err());
        remote.write_all(b"late").await.unwrap();
        match input.recv().await.unwrap() {
            Input::Read { buf, len, .. } => {
                assert_eq!(buf.get(..len), Some(&b"late"[..]));
                free_tx.try_send(buf).unwrap();
            }
            _ => panic!("expected a read"),
        }

        // Without the flag the next silence fails the link after 25 s.
        idle_ok.set(false);
        let started = tokio::time::Instant::now();
        match input.recv().await.unwrap() {
            Input::LinkDown { error, .. } => assert_eq!(error, "read timeout"),
            _ => panic!("expected the link to fail"),
        }
        assert_eq!(started.elapsed(), StdDuration::from_secs(25));
    }
}
