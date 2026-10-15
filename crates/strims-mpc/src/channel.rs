// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The byte channel the protocols run over: buffered reads and writes with
//! an explicit flush.
//!
//! The protocols are straight-line `async fn`s over [`Channel`].
//! [`crate::Session`] owns the top-level future and polls it with a no-op
//! waker whenever input arrives; a read that can't be satisfied yet flushes
//! pending output and returns `Pending` once, and the next `handle_input`
//! resumes it. Nothing here touches a runtime, a timer or a thread. The
//! async is an implementation detail that never escapes the crate.
//!
//! The channel and the session share two in-memory mailboxes. They are
//! `std::sync::mpsc` queues only because those are the std types that make
//! the boxed future `Send` without a lock; both ends live on the caller's
//! thread and nothing ever blocks on them.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

use crate::Error;
use crate::block::Block;
use crate::common::decode_point;
use curve25519_dalek::ristretto::RistrettoPoint;

/// Returns `Pending` once, then `Ready`.
#[derive(Default)]
struct YieldNow(bool);

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            Poll::Pending
        }
    }
}

/// The protocol side of a [`crate::Session`]'s byte stream.
pub(crate) struct Channel {
    /// Chunks from `Session::handle_input`.
    inbound: Receiver<Vec<u8>>,
    /// Received bytes not consumed yet: `rbuf[rpos..]`. Never longer than the
    /// largest single read plus one input chunk, because the protocol drains
    /// it before yielding.
    rbuf: Vec<u8>,
    rpos: usize,
    /// Flushed output for `Session::poll_transmit`.
    outbound: Sender<Vec<u8>>,
    /// Written bytes not flushed yet.
    wbuf: Vec<u8>,
}

impl Channel {
    pub(crate) fn new(inbound: Receiver<Vec<u8>>, outbound: Sender<Vec<u8>>) -> Channel {
        Channel {
            inbound,
            rbuf: Vec::new(),
            rpos: 0,
            outbound,
            wbuf: Vec::new(),
        }
    }

    /// Buffers `b` for sending, like a buffered `Write`.
    pub(crate) fn write(&mut self, b: &[u8]) {
        self.wbuf.extend_from_slice(b);
    }

    /// Hands buffered output to the session. The channel also flushes before
    /// waiting for input, so a forgotten flush can't deadlock a protocol and
    /// flush boundaries never affect the wire.
    pub(crate) fn flush(&mut self) {
        if !self.wbuf.is_empty() {
            // The session holds the receiver for as long as it polls us.
            let _ = self.outbound.send(core::mem::take(&mut self.wbuf));
        }
    }

    /// Fills `dst` from the input stream, waiting for more input as needed.
    pub(crate) async fn read_exact(&mut self, dst: &mut [u8]) -> Result<(), Error> {
        loop {
            let end = self.rpos.saturating_add(dst.len());
            if let Some(src) = self.rbuf.get(self.rpos..end) {
                dst.copy_from_slice(src);
                self.rpos = end;
                if self.rpos == self.rbuf.len() {
                    self.rbuf.clear();
                    self.rpos = 0;
                }
                return Ok(());
            }
            match self.inbound.try_recv() {
                Ok(chunk) => {
                    // TODO: each chunk is copied into the session's queue and
                    // again here, and `drain` shifts `rbuf` per chunk; a
                    // `VecDeque<u8>` or a chunk list would avoid both.
                    self.rbuf.drain(..self.rpos);
                    self.rpos = 0;
                    self.rbuf.extend_from_slice(&chunk);
                }
                Err(TryRecvError::Empty) => {
                    self.flush();
                    YieldNow::default().await;
                }
                Err(TryRecvError::Disconnected) => return Err(Error::Closed),
            }
        }
    }

    /// Reads a [`Block`].
    pub(crate) async fn read_block(&mut self) -> Result<Block, Error> {
        let mut b = [0u8; 16];
        self.read_exact(&mut b).await?;
        Ok(b)
    }

    /// Reads a canonically encoded Ristretto point.
    pub(crate) async fn read_point(&mut self) -> Result<RistrettoPoint, Error> {
        let mut b = [0u8; 32];
        self.read_exact(&mut b).await?;
        decode_point(&b)
    }

    /// Reads a big-endian `u32`.
    pub(crate) async fn read_int(&mut self) -> Result<u32, Error> {
        let mut b = [0u8; 4];
        self.read_exact(&mut b).await?;
        Ok(u32::from_be_bytes(b))
    }

    /// Writes a big-endian `u32`.
    pub(crate) fn write_int(&mut self, n: u32) {
        self.write(&n.to_be_bytes());
    }

    /// Whether received bytes are waiting that the protocol never read.
    pub(crate) fn has_unread_input(&mut self) -> bool {
        while let Ok(chunk) = self.inbound.try_recv() {
            self.rbuf.extend_from_slice(&chunk);
        }
        self.rpos < self.rbuf.len()
    }
}
