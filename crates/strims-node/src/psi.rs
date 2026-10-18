// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! PSI sessions computed on a thread of their own: [`ThreadPsi`].
//!
//! # Cost
//!
//! A PSI session costs tens of milliseconds of CPU, and one step of it,
//! the work one of the peer's messages causes, can take most of that.
//! Computed inline ([`InlinePsi`](strims_network::psi::InlinePsi)), a
//! step runs on the thread that runs the node, and on a current-thread
//! runtime (strims-svc's) the node's links, timers and every other task
//! wait for it. [`ThreadPsi`] computes the sessions on a [`PsiServer`] on
//! a dedicated thread, so the node's thread only copies their messages in
//! and out, and swarm I/O goes on while a step computes.
//!
//! # Shape
//!
//! [`ThreadPsi::spawn`] starts the thread and returns the engine, which
//! runs its sessions as [`RemotePsi`] sessions whose transport sends on
//! the thread's request channel, and the [`ThreadPsiTask`] that hands the
//! thread's replies back. The caller runs the task as a local task on the
//! thread that uses the engine. The server answers one message at a time,
//! in the order they were sent, so the sessions of every engine handle
//! share the thread.
//!
//! # Lifetime
//!
//! The engine's handles and its sessions hold the request channel open.
//! Once the last of them is gone the channel closes, the thread ends, and
//! the task ends with it.
//!
//! # Bounds
//!
//! Both channels are unbounded: a transport's send can't wait, and a
//! dropped message would break its session's stream. They hold what the
//! sessions in flight exchange: the server runs at most
//! [`MAX_SERVER_SESSIONS`](strims_network::psi::MAX_SERVER_SESSIONS) at
//! once (a network control runs at most
//! [`max_sessions`](strims_network::Config::max_sessions)), every message
//! is within the codec's bounds, a session's input comes from its peer's
//! link at the rate the link delivers it, and the server drops a
//! session's input once the session has ended.
//!
//! # Determinism
//!
//! Replies arrive when the thread has computed them, in wall time, so a
//! run on a `ThreadPsi` doesn't replay. Simulations, under turmoil or
//! otherwise, compute PSI inline.

use std::fmt;
use std::rc::{Rc, Weak};
use std::sync::mpsc as std_mpsc;

use strims_network::psi::{
    PsiEngine, PsiOutcome, PsiReady, PsiRole, PsiServer, PsiSession, PsiTransport, RemotePsi,
};
use tokio::sync::mpsc as tokio_mpsc;

/// The name of a [`ThreadPsi`]'s thread.
pub const THREAD_NAME: &str = "strims-psi";

/// The thread's request channel. Every engine handle and session holds
/// it open; the transport holds it weakly.
type Requests = Rc<std_mpsc::Sender<Vec<u8>>>;

/// A PSI engine whose sessions compute on a thread of their own. See the
/// [module docs](self).
///
/// Clones share the engine and its thread.
#[derive(Clone)]
pub struct ThreadPsi {
    engine: RemotePsi,
    requests: Requests,
}

impl fmt::Debug for ThreadPsi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ThreadPsi").field(&self.engine).finish()
    }
}

impl ThreadPsi {
    /// Starts the engine's thread. Returns the engine and the task that
    /// feeds it the thread's replies, for the caller to run as a local
    /// task beside the engine's users: until it runs, sessions get no
    /// answers.
    pub fn spawn() -> std::io::Result<(ThreadPsi, ThreadPsiTask)> {
        let (requests, serve_requests) = std_mpsc::channel();
        let (serve_replies, replies) = tokio_mpsc::unbounded_channel();
        // A driver's thread: the PSI server's CPU stays off the node's.
        std::thread::Builder::new()
            .name(THREAD_NAME.to_owned())
            .spawn(move || serve(&serve_requests, &serve_replies))?;
        let requests = Rc::new(requests);
        let engine = RemotePsi::new(Transport(Rc::downgrade(&requests)));
        let task = ThreadPsiTask {
            replies,
            engine: engine.clone(),
        };
        Ok((ThreadPsi { engine, requests }, task))
    }
}

impl PsiEngine for ThreadPsi {
    fn start(
        &self,
        role: PsiRole,
        keys: Vec<Vec<u8>>,
        seed: [u8; 16],
        max_peer_keys: usize,
        ready: PsiReady,
    ) -> Box<dyn PsiSession> {
        Box::new(ThreadSession {
            session: self.engine.start(role, keys, seed, max_peer_keys, ready),
            _requests: self.requests.clone(),
        })
    }
}

/// A session on the thread. It holds the request channel open until its
/// remote session, dropped first, has sent its close.
struct ThreadSession {
    session: Box<dyn PsiSession>,
    _requests: Requests,
}

impl PsiSession for ThreadSession {
    fn handle_input(&mut self, data: &[u8]) {
        self.session.handle_input(data);
    }

    fn poll_transmit(&mut self, buf: &mut [u8]) -> Option<usize> {
        self.session.poll_transmit(buf)
    }

    fn poll_outcome(&mut self) -> Option<PsiOutcome> {
        self.session.poll_outcome()
    }
}

/// Hands a [`ThreadPsi`]'s replies to its sessions. Run
/// [`ThreadPsiTask::run`] as a local task on the engine's thread.
pub struct ThreadPsiTask {
    replies: tokio_mpsc::UnboundedReceiver<Vec<u8>>,
    engine: RemotePsi,
}

impl fmt::Debug for ThreadPsiTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreadPsiTask")
            .field("engine", &self.engine)
            .finish_non_exhaustive()
    }
}

impl ThreadPsiTask {
    /// Feeds the replies to the sessions until the thread ends.
    pub async fn run(mut self) {
        while let Some(msg) = self.replies.recv().await {
            self.engine.receive(&msg);
        }
    }
}

/// The engine's transport: the thread's request channel, while an engine
/// handle or a session holds it open.
struct Transport(Weak<std_mpsc::Sender<Vec<u8>>>);

impl PsiTransport for Transport {
    /// A send fails only once the thread has ended; its sessions then get
    /// no answer, and the network control's stall timeout ends their runs.
    fn send(&self, msg: Vec<u8>) {
        if let Some(requests) = self.0.upgrade() {
            let _ = requests.send(msg);
        }
    }
}

/// The thread's loop: the server's answers to each request, in order,
/// until the requests end or nobody takes the answers. The server drops
/// malformed messages.
fn serve(requests: &std_mpsc::Receiver<Vec<u8>>, replies: &tokio_mpsc::UnboundedSender<Vec<u8>>) {
    let mut server = PsiServer::new();
    while let Ok(msg) = requests.recv() {
        for reply in server.handle(&msg) {
            if replies.send(reply).is_err() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )]

    use std::time::Duration as StdDuration;

    use rand_core::Rng;
    use strims_base::rng::det_rng;
    use strims_network::psi::InlinePsi;
    use tokio::sync::Notify;
    use tokio::task::LocalSet;

    use super::*;

    const WAIT: StdDuration = StdDuration::from_secs(30);

    fn run_local<F: Future>(f: F) -> F::Output {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(LocalSet::new().run_until(f))
    }

    /// A sender with `s` keys and a receiver with `r`, `shared` of them in
    /// both sets.
    fn sets(seed: u64, s: usize, r: usize, shared: usize) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
        let mut rng = det_rng(seed);
        let mut key = || {
            let mut k = vec![0u8; 32];
            rng.fill_bytes(&mut k);
            k
        };
        let common: Vec<_> = (0..shared).map(|_| key()).collect();
        let mut a = common.clone();
        a.extend((shared..s).map(|_| key()));
        let mut b: Vec<_> = (shared..r).map(|_| key()).collect();
        b.extend(common);
        (a, b)
    }

    /// A ready handle and the notification it gives.
    fn ready() -> (PsiReady, Rc<Notify>) {
        let notify = Rc::new(Notify::new());
        let n = notify.clone();
        (PsiReady::new(move || n.notify_one()), notify)
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Run {
        sender: PsiOutcome,
        receiver: PsiOutcome,
        sender_bytes: Vec<u8>,
        receiver_bytes: Vec<u8>,
    }

    /// A sender and a receiver, each on its engine.
    struct Pair {
        s: Box<dyn PsiSession>,
        r: Box<dyn PsiSession>,
        notify: Rc<Notify>,
    }

    impl Pair {
        fn start(
            sender: &dyn PsiEngine,
            receiver: &dyn PsiEngine,
            keys: (Vec<Vec<u8>>, Vec<Vec<u8>>),
        ) -> Pair {
            let (ready, notify) = ready();
            Pair {
                s: sender.start(PsiRole::Sender, keys.0, [1; 16], 4096, ready.clone()),
                r: receiver.start(PsiRole::Receiver, keys.1, [2; 16], 4096, ready),
                notify,
            }
        }

        /// Moves each side's bytes to the other, waiting for the engines'
        /// notifications when none move, until both sides have outcomes.
        async fn finish(&mut self) -> Run {
            let (mut sender_bytes, mut receiver_bytes) = (Vec::new(), Vec::new());
            let (mut sender, mut receiver) = (None, None);
            loop {
                let to_r = drain(self.s.as_mut(), &mut sender_bytes);
                if !to_r.is_empty() {
                    self.r.handle_input(&to_r);
                }
                let to_s = drain(self.r.as_mut(), &mut receiver_bytes);
                if !to_s.is_empty() {
                    self.s.handle_input(&to_s);
                }
                sender = sender.or_else(|| self.s.poll_outcome());
                receiver = receiver.or_else(|| self.r.poll_outcome());
                if let (Some(sender), Some(receiver)) = (&sender, &receiver) {
                    // the outcomes' last bytes
                    drain(self.s.as_mut(), &mut sender_bytes);
                    drain(self.r.as_mut(), &mut receiver_bytes);
                    return Run {
                        sender: sender.clone(),
                        receiver: receiver.clone(),
                        sender_bytes,
                        receiver_bytes,
                    };
                }
                if to_r.is_empty() && to_s.is_empty() {
                    tokio::time::timeout(WAIT, self.notify.notified())
                        .await
                        .expect("the psi thread never answered");
                }
            }
        }
    }

    /// Takes every byte `session` has for its peer, and logs them.
    fn drain(session: &mut dyn PsiSession, log: &mut Vec<u8>) -> Vec<u8> {
        let mut out = Vec::new();
        let mut buf = vec![0u8; 1400];
        while let Some(n) = session.poll_transmit(&mut buf) {
            out.extend_from_slice(&buf[..n]);
        }
        log.extend_from_slice(&out);
        out
    }

    async fn run(
        sender: &dyn PsiEngine,
        receiver: &dyn PsiEngine,
        keys: (Vec<Vec<u8>>, Vec<Vec<u8>>),
    ) -> Run {
        Pair::start(sender, receiver, keys).finish().await
    }

    /// Sessions on the thread, in either role or both, give the bytes and
    /// the intersection that inline sessions give with the same seeds.
    #[test]
    fn thread_sessions_match_inline_ones() {
        run_local(async {
            let (engine, task) = ThreadPsi::spawn().unwrap();
            tokio::task::spawn_local(task.run());
            // the last pair's steps are longer than one engine message
            for (i, (s, r, shared)) in [(1, 1, 1), (12, 12, 7), (2000, 1100, 600)]
                .into_iter()
                .enumerate()
            {
                let keys = || sets(i as u64, s, r, shared);
                let inline = run(&InlinePsi, &InlinePsi, keys()).await;
                assert_eq!(inline.sender, PsiOutcome::Sent, "case {i}");
                match &inline.receiver {
                    PsiOutcome::Shared(k) => assert_eq!(k.len(), shared, "case {i}"),
                    o => panic!("case {i}: {o:?}"),
                }
                assert_eq!(run(&engine, &engine, keys()).await, inline, "case {i}");
                assert_eq!(run(&engine, &InlinePsi, keys()).await, inline, "case {i}");
                assert_eq!(run(&InlinePsi, &engine, keys()).await, inline, "case {i}");
            }
        });
    }

    /// A session that the peer's garbage fails leaves the thread serving
    /// the sessions after it.
    #[test]
    fn a_failed_session_leaves_the_thread_serving() {
        run_local(async {
            let (engine, task) = ThreadPsi::spawn().unwrap();
            tokio::task::spawn_local(task.run());
            let (ready, notify) = ready();
            let mut s = engine.start(PsiRole::Sender, sets(1, 3, 3, 1).0, [1; 16], 8, ready);
            s.handle_input(&[0xff; 4096]);
            let outcome = tokio::time::timeout(WAIT, async {
                loop {
                    if let Some(o) = s.poll_outcome() {
                        return o;
                    }
                    notify.notified().await;
                }
            })
            .await
            .expect("the failed session never ended");
            assert!(matches!(outcome, PsiOutcome::Failed(_)), "{outcome:?}");

            let inline = run(&InlinePsi, &InlinePsi, sets(2, 5, 3, 2)).await;
            assert_eq!(run(&engine, &engine, sets(2, 5, 3, 2)).await, inline);
        });
    }

    /// Sessions keep their answers after the engine's last handle is gone,
    /// and the thread and its task end with the last session.
    #[test]
    fn the_thread_ends_with_the_engines_last_session() {
        run_local(async {
            let (engine, task) = ThreadPsi::spawn().unwrap();
            let task = tokio::task::spawn_local(task.run());
            let keys = || sets(3, 40, 30, 10);
            let inline = run(&InlinePsi, &InlinePsi, keys()).await;
            let mut pair = Pair::start(&engine, &engine.clone(), keys());
            drop(engine);
            assert_eq!(pair.finish().await, inline);
            assert!(!task.is_finished(), "the task ended under live sessions");
            drop(pair);
            tokio::time::timeout(WAIT, task)
                .await
                .expect("the task outlived the engine")
                .unwrap();
        });
    }
}
