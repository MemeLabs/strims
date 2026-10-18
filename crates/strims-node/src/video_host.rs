// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The node's TCP listeners as the video control's [`VideoHost`]: the RTMP
//! ingress on real sockets or under turmoil.
//!
//! [`NetVideoHost`] binds through a [`Net`], so a node's ingress runs on
//! the sockets its links use:
//!
//! - a [`NetListener`] yields each connection its socket accepts, with
//!   the remote address as `host:port` (an IPv6 host in brackets). An
//!   accept that fails for that connection alone (aborted or reset before
//!   it was taken), or is interrupted, is skipped; any other failure is
//!   yielded, and ends the listener for the control. Dropping it closes
//!   the listening socket, and the connections it took stay open.
//! - a [`NetReader`] yields what the connection receives, at most
//!   [`READ_BYTES`] per piece, and ends at the peer's end of the stream.
//! - a [`NetWriter`] holds one piece at a time: [`Sink::poll_ready`] waits
//!   until the socket has taken all of the piece before. Flushing waits
//!   for that and flushes the socket; closing then shuts its sending side
//!   down.
//!
//! The reader and writer share the socket, which closes once both are
//! dropped.

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_util::stream::LocalBoxStream;
use futures_util::{Sink, Stream, StreamExt};
use strims_video::{Accepted, VideoHost};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, ReadHalf, WriteHalf};

use crate::Net;

/// The most bytes a [`NetReader`] piece holds.
pub const READ_BYTES: usize = 64 * 1024;

/// The TCP listeners of `N` as a [`VideoHost`]; see the
/// [module docs](self).
#[derive(Clone, Copy, Debug, Default)]
pub struct NetVideoHost<N: Net>(pub N);

impl<N: Net> VideoHost for NetVideoHost<N> {
    type Error = io::Error;
    type Reader = NetReader<N>;
    type Writer = NetWriter<N>;
    type Listener = NetListener<N>;

    fn rtmp_supported(&self) -> bool {
        true
    }

    fn listen(&self, addr: &str) -> impl Future<Output = io::Result<NetListener<N>>> {
        let (net, addr) = (self.0.clone(), addr.to_owned());
        async move {
            let listener = net.tcp_bind(addr).await?;
            let local_addr = net.tcp_listener_addr(&listener)?;
            Ok(NetListener::new(net, listener, local_addr))
        }
    }
}

/// A connection a [`NetListener`] accepted.
pub type NetAccepted<N> = Accepted<NetReader<N>, NetWriter<N>>;

/// The connections a listening socket accepts; dropping it closes the
/// socket.
pub struct NetListener<N: Net> {
    accepts: LocalBoxStream<'static, io::Result<NetAccepted<N>>>,
    local_addr: SocketAddr,
}

impl<N: Net> NetListener<N> {
    fn new(net: N, listener: N::TcpListener, local_addr: SocketAddr) -> Self {
        let accepts = futures_util::stream::unfold((net, listener), |(net, listener)| async move {
            let accepted = loop {
                match net.tcp_accept(&listener).await {
                    Ok((stream, peer)) => break Ok(split::<N>(stream, peer.to_string())),
                    Err(e) if retry_accept(&e) => {
                        tracing::debug!(error = %e, "an accept failed for its connection");
                    }
                    Err(e) => break Err(e),
                }
            };
            Some((accepted, (net, listener)))
        });
        NetListener {
            accepts: accepts.boxed_local(),
            local_addr,
        }
    }

    /// The address the socket listens on: the bound one, with the port
    /// picked for port 0.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl<N: Net> Stream for NetListener<N> {
    type Item = io::Result<NetAccepted<N>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.accepts.poll_next_unpin(cx)
    }
}

/// Whether an accept failed for its connection alone or was interrupted,
/// leaving the listening socket working.
fn retry_accept(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::Interrupted
    )
}

fn split<N: Net>(stream: N::TcpStream, peer: String) -> NetAccepted<N> {
    let (reader, writer) = tokio::io::split(stream);
    Accepted {
        reader: NetReader {
            half: reader,
            buf: vec![0; READ_BYTES].into_boxed_slice(),
            ended: false,
        },
        writer: NetWriter {
            half: writer,
            piece: Vec::new(),
            written: 0,
        },
        peer,
    }
}

/// What a connection receives, in pieces of at most [`READ_BYTES`]; it
/// ends at the peer's end of the stream, or after an error.
pub struct NetReader<N: Net> {
    half: ReadHalf<N::TcpStream>,
    buf: Box<[u8]>,
    ended: bool,
}

impl<N: Net> Stream for NetReader<N> {
    type Item = io::Result<Vec<u8>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        while !this.ended {
            let mut buf = ReadBuf::new(&mut this.buf);
            match ready!(Pin::new(&mut this.half).poll_read(cx, &mut buf)) {
                Ok(()) if buf.filled().is_empty() => this.ended = true,
                Ok(()) => return Poll::Ready(Some(Ok(buf.filled().to_vec()))),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => {
                    this.ended = true;
                    return Poll::Ready(Some(Err(e)));
                }
            }
        }
        Poll::Ready(None)
    }
}

/// Sends a connection's bytes, a piece at a time; see the
/// [module docs](self).
pub struct NetWriter<N: Net> {
    half: WriteHalf<N::TcpStream>,
    /// The piece being written; the socket took `written` bytes of it.
    piece: Vec<u8>,
    written: usize,
}

impl<N: Net> NetWriter<N> {
    /// Writes the rest of the piece; ready once the socket took it all.
    fn poll_piece(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        while let Some(rest) = self.piece.get(self.written..).filter(|r| !r.is_empty()) {
            match ready!(Pin::new(&mut self.half).poll_write(cx, rest)) {
                Ok(0) => return Poll::Ready(Err(io::ErrorKind::WriteZero.into())),
                Ok(n) => self.written += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
        self.piece.clear();
        self.written = 0;
        Poll::Ready(Ok(()))
    }
}

impl<N: Net> Sink<Vec<u8>> for NetWriter<N> {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.get_mut().poll_piece(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> io::Result<()> {
        let this = self.get_mut();
        if this.piece.is_empty() {
            this.piece = item;
        } else {
            this.piece.extend_from_slice(&item);
        }
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_piece(cx))?;
        Pin::new(&mut this.half).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_piece(cx))?;
        Pin::new(&mut this.half).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

    use std::future::poll_fn;

    use futures_util::{FutureExt, SinkExt};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    use super::*;
    use crate::TokioNet;

    type Conn = NetAccepted<TokioNet>;

    /// A listener on a free loopback port.
    async fn listen() -> NetListener<TokioNet> {
        NetVideoHost(TokioNet).listen("127.0.0.1:0").await.unwrap()
    }

    /// A client connected to `l`, and the connection `l` accepted.
    async fn connect(l: &mut NetListener<TokioNet>) -> (TcpStream, Conn) {
        let client = TcpStream::connect(l.local_addr()).await.unwrap();
        let conn = l.next().await.unwrap().unwrap();
        (client, conn)
    }

    /// Byte `i` of a test stream.
    fn pattern(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    /// Reads until the client's stream ends.
    async fn read_to_end(client: &mut TcpStream) -> Vec<u8> {
        let mut got = Vec::new();
        client.read_to_end(&mut got).await.unwrap();
        got
    }

    /// Reads exactly `n` bytes from the reader, checking each piece's
    /// bound.
    async fn read_exactly(r: &mut NetReader<TokioNet>, n: usize) -> Vec<u8> {
        let mut got = Vec::new();
        while got.len() < n {
            let piece = r.next().await.unwrap().unwrap();
            assert!(!piece.is_empty() && piece.len() <= READ_BYTES);
            got.extend_from_slice(&piece);
        }
        got
    }

    #[tokio::test]
    async fn connections_carry_bytes_both_ways_and_half_close() {
        let host = NetVideoHost(TokioNet);
        assert!(host.rtmp_supported());
        let mut l = listen().await;
        let (mut client, mut conn) = connect(&mut l).await;
        assert_eq!(conn.peer, client.local_addr().unwrap().to_string());

        let sent = pattern(200 * 1024);
        client.write_all(&sent).await.unwrap();
        assert_eq!(read_exactly(&mut conn.reader, sent.len()).await, sent);

        conn.writer.send(b"hello".to_vec()).await.unwrap();
        let mut hello = [0; 5];
        client.read_exact(&mut hello).await.unwrap();
        assert_eq!(&hello, b"hello");

        // the client's end of its stream ends the reader
        client.shutdown().await.unwrap();
        assert!(conn.reader.next().await.is_none());
        assert!(conn.reader.next().await.is_none());

        // closing the writer ends the client's reads after what it sent
        conn.writer.feed(b"bye".to_vec()).await.unwrap();
        conn.writer.close().await.unwrap();
        assert_eq!(read_to_end(&mut client).await, b"bye");
    }

    #[tokio::test]
    async fn a_listen_on_a_bound_address_fails() {
        let l = listen().await;
        let again = NetVideoHost(TokioNet)
            .listen(&l.local_addr().to_string())
            .await;
        assert!(again.is_err());
    }

    /// A peer that doesn't read leaves the writer unready once the
    /// connection holds what it can, and it stays so; once the peer reads,
    /// every byte arrives, in order.
    #[tokio::test]
    async fn a_writer_waits_while_its_peer_does_not_read() {
        let mut l = listen().await;
        let (mut client, conn) = connect(&mut l).await;
        let mut writer = conn.writer;
        let piece = pattern(READ_BYTES);
        let mut sent = 0;
        let ready = |w: &mut NetWriter<TokioNet>| {
            poll_fn(|cx| Pin::new(&mut *w).poll_ready(cx)).now_or_never()
        };
        // the kernel moves bytes to the peer's buffers for a while after
        // the socket's own fill up
        loop {
            assert!(sent < 512 << 20, "the connection took {sent} bytes unread");
            match ready(&mut writer) {
                Some(r) => {
                    r.unwrap();
                    Pin::new(&mut writer).start_send(piece.clone()).unwrap();
                    sent += piece.len();
                }
                None => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    if ready(&mut writer).is_none() {
                        break;
                    }
                }
            }
        }
        for _ in 0..3 {
            tokio::task::yield_now().await;
            assert!(
                ready(&mut writer).is_none(),
                "ready while the peer reads nothing"
            );
        }

        let read = tokio::spawn(async move { read_to_end(&mut client).await });
        writer.close().await.unwrap();
        let got = read.await.unwrap();
        assert_eq!(got.len(), sent);
        assert!(got.chunks(piece.len()).all(|c| c == piece));
    }

    #[tokio::test]
    async fn dropping_the_reader_and_writer_closes_the_connection() {
        let mut l = listen().await;
        let (mut client, conn) = connect(&mut l).await;
        let Accepted {
            reader, mut writer, ..
        } = conn;
        drop(reader);
        writer.send(b"still open".to_vec()).await.unwrap();
        let mut got = [0; 10];
        client.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"still open");
        drop(writer);
        assert!(read_to_end(&mut client).await.is_empty());
    }

    #[tokio::test]
    async fn dropping_the_listener_leaves_its_connections_open() {
        let mut l = listen().await;
        let addr = l.local_addr();
        let (mut client, mut conn) = connect(&mut l).await;
        drop(l);
        assert!(TcpStream::connect(addr).await.is_err());

        client.write_all(b"ping").await.unwrap();
        assert_eq!(read_exactly(&mut conn.reader, 4).await, b"ping");
        conn.writer.send(b"pong".to_vec()).await.unwrap();
        let mut got = [0; 4];
        client.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"pong");
    }
}
