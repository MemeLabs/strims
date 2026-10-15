// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The sans-IO driver around a protocol future.

use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use std::sync::mpsc::{self, Receiver, Sender};

use crate::Error;
use crate::channel::Channel;

type ProtocolFuture<T> = Pin<Box<dyn Future<Output = Result<T, Error>> + Send>>;

/// One run of a two-party protocol as a sans-IO state machine.
///
/// The protocol sees a reliable, ordered byte stream. Feed every byte the peer sends to [`Session::handle_input`], in
/// order and split anywhere, and send whatever [`Session::poll_transmit`]
/// returns. All computation happens inside those calls; there are no timers.
pub struct Session<T> {
    future: Option<ProtocolFuture<T>>,
    input: Sender<Vec<u8>>,
    output: Receiver<Vec<u8>>,
    /// Output not handed out yet: `out[out_pos..]`.
    out: Vec<u8>,
    out_pos: usize,
    result: Option<Result<T, Error>>,
    finished: bool,
}

impl<T: Send + 'static> Session<T> {
    /// Starts `protocol` on a fresh channel and runs it until it first needs
    /// input.
    pub(crate) fn start<F, Fut>(protocol: F) -> Session<T>
    where
        F: FnOnce(Channel) -> Fut,
        Fut: Future<Output = Result<T, Error>> + Send + 'static,
    {
        let (input, inbound) = mpsc::channel();
        let (outbound, output) = mpsc::channel();
        let mut s = Session {
            future: Some(Box::pin(protocol(Channel::new(inbound, outbound)))),
            input,
            output,
            out: Vec::new(),
            out_pos: 0,
            result: None,
            finished: false,
        };
        s.drive();
        s
    }
}

impl<T> Session<T> {
    fn drive(&mut self) {
        let Some(future) = self.future.as_mut() else {
            return;
        };
        let poll = future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()));
        while let Ok(chunk) = self.output.try_recv() {
            if self.out_pos == self.out.len() {
                self.out.clear();
                self.out_pos = 0;
            }
            self.out.extend_from_slice(&chunk);
        }
        if let Poll::Ready(result) = poll {
            self.future = None;
            if result.is_err() {
                // Partial output from a failed run is useless to the peer.
                self.out.clear();
                self.out_pos = 0;
            }
            self.result = Some(result);
            self.finished = true;
        }
    }

    /// Feeds bytes received from the peer and runs the protocol as far as
    /// they allow.
    ///
    /// Returns the protocol's error if it fails during this call, and
    /// [`Error::UnexpectedInput`] for input after the protocol finished.
    pub fn handle_input(&mut self, data: &[u8]) -> Result<(), Error> {
        if self.future.is_none() {
            return if data.is_empty() {
                Ok(())
            } else {
                Err(Error::UnexpectedInput)
            };
        }
        if !data.is_empty() {
            // The channel holding the receiver lives inside `future`.
            let _ = self.input.send(data.to_vec());
        }
        self.drive();
        match &self.result {
            Some(Err(e)) => Err(e.clone()),
            _ => Ok(()),
        }
    }

    /// Copies the next bytes to send into `buf` and returns how many, or
    /// `None` if there is nothing to send (or `buf` is empty).
    pub fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize> {
        let pending = self.out.get(self.out_pos..).unwrap_or_default();
        let n = pending.len().min(buf.len());
        if n == 0 {
            return None;
        }
        let (dst, _) = buf.split_at_mut(n);
        dst.copy_from_slice(pending.get(..n).unwrap_or_default());
        self.out_pos += n;
        if self.out_pos == self.out.len() {
            self.out.clear();
            self.out_pos = 0;
        }
        Some(n)
    }

    /// Bytes waiting for [`Session::poll_transmit`].
    pub fn pending_transmit(&self) -> usize {
        self.out.len() - self.out_pos
    }

    /// Whether the protocol has finished and all of its output has been
    /// handed out.
    pub fn is_done(&self) -> bool {
        self.finished && self.pending_transmit() == 0
    }

    /// The outcome, once the protocol has finished (and until taken).
    pub fn result(&self) -> Option<Result<&T, &Error>> {
        self.result.as_ref().map(Result::as_ref)
    }

    /// Takes the outcome out of a finished session.
    pub fn take_result(&mut self) -> Option<Result<T, Error>> {
        self.result.take()
    }
}

impl<T> fmt::Debug for Session<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("finished", &self.finished)
            .field("pending_transmit", &self.pending_transmit())
            .finish_non_exhaustive()
    }
}

/// The sans-IO surface shared by [`Session`] and the PSZ wrappers, so
/// drivers and test harnesses can treat them alike.
pub trait Endpoint {
    /// See [`Session::handle_input`].
    fn handle_input(&mut self, data: &[u8]) -> Result<(), Error>;
    /// See [`Session::poll_transmit`].
    fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize>;
    /// See [`Session::is_done`].
    fn is_done(&self) -> bool;
}

impl<T> Endpoint for Session<T> {
    fn handle_input(&mut self, data: &[u8]) -> Result<(), Error> {
        Session::handle_input(self, data)
    }

    fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize> {
        Session::poll_transmit(self, buf)
    }

    fn is_done(&self) -> bool {
        Session::is_done(self)
    }
}

/// Ends a protocol run: flushes its output on success and rejects input the
/// protocol never read, which means the peers disagree about the transcript.
pub(crate) fn finish<T>(conn: &mut Channel, result: Result<T, Error>) -> Result<T, Error> {
    let value = result?;
    if conn.has_unread_input() {
        return Err(Error::UnexpectedInput);
    }
    conn.flush();
    Ok(value)
}
