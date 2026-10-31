// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! PSI sessions computed in a worker of their own: [`start_psi`] runs the
//! server there, and [`WorkerPsi`] is the node's engine on it.
//!
//! # Cost
//!
//! A PSI session costs tens of milliseconds of CPU, even in an optimized
//! build, and one step of it, the work one of the peer's messages causes,
//! can take a large part of that. Computed inline
//! ([`InlinePsi`](strims_network::psi::InlinePsi)), a step runs on the
//! node's worker, the one thread that runs the node, and its sockets,
//! data channels, timers and frontend calls wait for it. [`WorkerPsi`]
//! computes the sessions on a [`PsiServer`] in the PSI worker, so the
//! node's worker only copies their messages in and out, and swarm I/O
//! goes on while a step computes.
//!
//! # The port
//!
//! The window creates a `MessageChannel` and gives one port to each
//! worker: the node's worker gets it as [`start`](crate::entry::start)'s
//! `psi`, and the PSI worker as [`start_psi`]'s `port`. Each message is
//! one engine message, posted as one `ArrayBuffer`, transferred; a
//! `Uint8Array` is taken as its bytes. Both ends drop anything else, and
//! the server and the engine drop malformed messages.
//!
//! The server answers one message at a time, in the order they arrive, so
//! the sessions of every profile the node runs share the PSI worker.
//!
//! # Privacy
//!
//! The port carries the sessions' keys and seeds, the PSI's private
//! inputs, so both of its ends must stay inside the page's own workers:
//! the window creates the channel and gives its ports to nothing else.
//!
//! # Lifetime
//!
//! [`WorkerPsi`] takes over its port's `onmessage`, and the handler holds
//! the engine that the replies feed. The engine's handles and its
//! sessions hold the handler attached. Once the last of them is gone the
//! handler detaches and the port closes; the replies still in flight
//! belonged to sessions that are gone.
//!
//! The server holds its port for the PSI worker's lifetime.
//!
//! # Bounds
//!
//! A port's queue is unbounded, and a transport's send can't wait: a
//! dropped message would break its session's stream. The queues hold what
//! the sessions in flight exchange: the server runs at most
//! [`MAX_SERVER_SESSIONS`](strims_network::psi::MAX_SERVER_SESSIONS) at
//! once (a network control runs at most
//! [`max_sessions`](strims_network::Config::max_sessions)), every message
//! is within the codec's bounds, a session's input comes from its peer's
//! link at the rate the link delivers it, and the server drops a
//! session's input once the session has ended.
//!
//! # Determinism
//!
//! Replies arrive when the PSI worker has computed them, so a run on a
//! `WorkerPsi` doesn't replay.

use std::fmt;
use std::rc::Rc;

use js_sys::{Array, ArrayBuffer, Uint8Array};
use strims_network::psi::{
    PsiEngine, PsiOutcome, PsiReady, PsiRole, PsiServer, PsiSession, PsiTransport, RemotePsi,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, MessagePort};

use crate::js;
use crate::options::Options;

/// Starts the PSI worker: a [`PsiServer`] answering the engine messages
/// that arrive on `port`, for the rest of the worker's life. `options` are
/// the window's (see [`crate::options`]); only `logLevel` applies. See the
/// [module docs](self).
#[wasm_bindgen]
pub fn start_psi(options: JsValue, port: MessagePort) {
    let (parsed, ignored) = Options::parse(&options);
    crate::log::init(parsed.log_level);
    for line in &ignored {
        tracing::warn!("ignoring an option: {line}");
    }
    serve(&port);
    tracing::info!("strims-wasm psi worker started");
}

/// Answers the engine messages on `port` with a server of its own, until
/// the worker ends. Setting `onmessage` starts the port, which delivers
/// what it buffered.
fn serve(port: &MessagePort) {
    let mut server = PsiServer::new();
    let replies = port.clone();
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
        let Some(msg) = bytes(&e.data()) else {
            tracing::debug!("a psi engine message that isn't binary dropped");
            return;
        };
        for reply in server.handle(&msg) {
            post(&replies, &reply);
        }
    });
    port.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    // The handler and its server live as long as the worker.
    on_message.forget();
}

/// A message's bytes: an `ArrayBuffer`'s or a `Uint8Array`'s.
fn bytes(data: &JsValue) -> Option<Vec<u8>> {
    if let Some(buffer) = data.dyn_ref::<ArrayBuffer>() {
        return Some(Uint8Array::new(buffer).to_vec());
    }
    data.dyn_ref::<Uint8Array>().map(Uint8Array::to_vec)
}

/// Posts `msg` on `port` as one transferred `ArrayBuffer`.
fn post(port: &MessagePort, msg: &[u8]) {
    let buffer = Uint8Array::from(msg).buffer();
    if let Err(e) = port.post_message_with_transferable(&buffer, &Array::of1(&buffer)) {
        tracing::debug!(error = %js::describe(&e), "posting a psi engine message");
    }
}

/// A PSI engine whose sessions compute in the PSI worker, over a port to
/// it. See the [module docs](self).
///
/// Clones share the engine and its port.
#[derive(Clone)]
pub struct WorkerPsi {
    engine: RemotePsi,
    handler: Rc<Handler>,
}

impl fmt::Debug for WorkerPsi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WorkerPsi").field(&self.engine).finish()
    }
}

impl WorkerPsi {
    /// An engine on `port`, whose other end the PSI worker serves
    /// ([`start_psi`]). It takes over the port's `onmessage`, which starts
    /// the port. One engine runs on a port: the server tells sessions
    /// apart by the ids their engine gives them.
    pub fn new(port: MessagePort) -> WorkerPsi {
        let engine = RemotePsi::new(Transport(port.clone()));
        let replies = engine.clone();
        let on_message =
            Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                match bytes(&e.data()) {
                    Some(msg) => replies.receive(&msg),
                    None => tracing::debug!("a psi engine message that isn't binary dropped"),
                }
            });
        port.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        WorkerPsi {
            engine,
            handler: Rc::new(Handler {
                port,
                _on_message: on_message,
            }),
        }
    }
}

impl PsiEngine for WorkerPsi {
    fn start(
        &self,
        role: PsiRole,
        keys: Vec<Vec<u8>>,
        seed: [u8; 16],
        max_peer_keys: usize,
        ready: PsiReady,
    ) -> Box<dyn PsiSession> {
        Box::new(WorkerSession {
            session: self.engine.start(role, keys, seed, max_peer_keys, ready),
            _handler: self.handler.clone(),
        })
    }
}

/// The port's handler, which feeds the replies to the engine's sessions.
/// Every engine handle and session holds it attached.
struct Handler {
    port: MessagePort,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl Drop for Handler {
    fn drop(&mut self) {
        // No callback may run once its closure is gone.
        self.port.set_onmessage(None);
        self.port.close();
    }
}

/// A session in the PSI worker. It holds the handler attached until its
/// remote session, dropped first, has sent its close.
struct WorkerSession {
    session: Box<dyn PsiSession>,
    _handler: Rc<Handler>,
}

impl PsiSession for WorkerSession {
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

/// The engine's transport: the port, without its handler, which holds the
/// engine and with it this transport.
struct Transport(MessagePort);

impl PsiTransport for Transport {
    /// A message to a PSI worker that has died is lost: its sessions get
    /// no answer, and the network control's
    /// [`stall_timeout`](strims_network::Config::stall_timeout) ends their
    /// runs.
    fn send(&self, msg: Vec<u8>) {
        post(&self.0, &msg);
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

    use std::cell::Cell;
    use std::future::Future;

    use rand_core::Rng;
    use strims_base::rng::det_rng;
    use strims_base::time::Duration;
    use strims_network::psi::InlinePsi;
    use strims_node_core::app::AppPlatform;
    use strims_rpc::Timer;
    use tokio::sync::Notify;
    use wasm_bindgen_test::wasm_bindgen_test;
    use web_sys::MessageChannel;

    use super::*;
    use crate::clock::{self, WasmClock};
    use crate::factory::WasmPlatform;

    const WAIT: Duration = Duration::from_secs(30);

    /// `f`, or a panic naming `what` after [`WAIT`].
    async fn within<F: Future>(what: &str, f: F) -> F::Output {
        let deadline = WasmClock.sleep_until(clock::now() + WAIT);
        tokio::select! {
            biased;
            v = f => v,
            () = deadline => panic!("timed out waiting for {what}"),
        }
    }

    /// The two ends of a new channel: the engine's and the server's.
    fn channel() -> (MessagePort, MessagePort) {
        let channel = MessageChannel::new().unwrap();
        (channel.port1(), channel.port2())
    }

    /// An engine on a channel whose other end a server answers.
    fn served() -> (WorkerPsi, MessagePort) {
        let (client, server) = channel();
        serve(&server);
        (WorkerPsi::new(client.clone()), client)
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

    /// A ready handle, the notification it gives, and how many it gave.
    fn ready() -> (PsiReady, Rc<Notify>, Rc<Cell<usize>>) {
        let notify = Rc::new(Notify::new());
        let count = Rc::new(Cell::new(0));
        let (n, c) = (notify.clone(), count.clone());
        let ready = PsiReady::new(move || {
            c.set(c.get() + 1);
            n.notify_one();
        });
        (ready, notify, count)
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
        notified: Rc<Cell<usize>>,
    }

    impl Pair {
        fn start(
            sender: &dyn PsiEngine,
            receiver: &dyn PsiEngine,
            keys: (Vec<Vec<u8>>, Vec<Vec<u8>>),
        ) -> Pair {
            let (ready, notify, notified) = ready();
            Pair {
                s: sender.start(PsiRole::Sender, keys.0, [1; 16], 4096, ready.clone()),
                r: receiver.start(PsiRole::Receiver, keys.1, [2; 16], 4096, ready),
                notify,
                notified,
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
                    within("the psi server", self.notify.notified()).await;
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

    /// The inline run of `keys`, checked to find `shared` keys.
    async fn inline(keys: (Vec<Vec<u8>>, Vec<Vec<u8>>), shared: usize) -> Run {
        let run = run(&InlinePsi, &InlinePsi, keys).await;
        assert_eq!(run.sender, PsiOutcome::Sent);
        match &run.receiver {
            PsiOutcome::Shared(k) => assert_eq!(k.len(), shared),
            o => panic!("{o:?}"),
        }
        run
    }

    /// Sessions over the port, in either role or both, give the bytes and
    /// the intersection that inline sessions give with the same seeds.
    #[wasm_bindgen_test]
    async fn port_sessions_match_inline_ones() {
        let (engine, _) = served();
        // the last pair's steps are longer than one engine message
        for (i, (s, r, shared)) in [(1, 1, 1), (12, 12, 7), (2000, 1100, 600)]
            .into_iter()
            .enumerate()
        {
            let keys = || sets(i as u64, s, r, shared);
            let inline = inline(keys(), shared).await;
            assert_eq!(run(&engine, &engine, keys()).await, inline, "case {i}");
            assert_eq!(run(&engine, &InlinePsi, keys()).await, inline, "case {i}");
            assert_eq!(run(&InlinePsi, &engine, keys()).await, inline, "case {i}");
        }
    }

    /// What isn't an engine message, on the PSI worker's port or back, is
    /// dropped, and the sessions after it run.
    #[wasm_bindgen_test]
    async fn malformed_port_messages_are_dropped() {
        let (client, server) = channel();
        // to the server, buffered until it starts
        client.post_message(&"start".into()).unwrap();
        client.post_message(&JsValue::NULL).unwrap();
        client.post_message(&ArrayBuffer::new(0)).unwrap();
        client
            .post_message(&Uint8Array::from(&[0xffu8; 300][..]))
            .unwrap();
        // to the engine
        server.post_message(&7.into()).unwrap();
        server
            .post_message(&Uint8Array::from(&[0xffu8; 300][..]).buffer())
            .unwrap();
        let engine = WorkerPsi::new(client);
        start_psi(JsValue::UNDEFINED, server);
        let keys = || sets(4, 20, 15, 5);
        let inline = inline(keys(), 5).await;
        assert_eq!(run(&engine, &engine, keys()).await, inline);
    }

    /// A session that the peer's garbage fails leaves the server serving
    /// the sessions after it.
    #[wasm_bindgen_test]
    async fn a_failed_session_leaves_the_server_serving() {
        let (engine, _) = served();
        let (ready, notify, _) = ready();
        let mut s = engine.start(PsiRole::Sender, sets(1, 3, 3, 1).0, [1; 16], 8, ready);
        s.handle_input(&[0xff; 4096]);
        let outcome = within("the failed session", async {
            loop {
                if let Some(o) = s.poll_outcome() {
                    return o;
                }
                notify.notified().await;
            }
        })
        .await;
        assert!(matches!(outcome, PsiOutcome::Failed(_)), "{outcome:?}");

        let keys = || sets(2, 5, 3, 2);
        let inline = inline(keys(), 2).await;
        assert_eq!(run(&engine, &engine, keys()).await, inline);
    }

    /// Sessions keep their answers after the engine's last handle is gone,
    /// and the port's handler detaches with the last session.
    #[wasm_bindgen_test]
    async fn the_handler_detaches_with_the_engines_last_session() {
        let (engine, port) = served();
        let keys = || sets(3, 40, 30, 10);
        let inline = inline(keys(), 10).await;
        let mut pair = Pair::start(&engine, &engine.clone(), keys());
        drop(engine);
        assert_eq!(pair.finish().await, inline);
        assert!(port.onmessage().is_some(), "detached under live sessions");
        drop(pair);
        assert!(
            port.onmessage().is_none(),
            "the handler outlived the engine"
        );
    }

    /// A platform given a port runs its sessions in the PSI worker, and
    /// one without computes them inline.
    #[wasm_bindgen_test]
    async fn the_platform_computes_psi_where_its_port_says() {
        let signal = || channel().0;
        let keys = || sets(5, 9, 6, 3);
        let inline = inline(keys(), 3).await;

        let (client, server) = channel();
        serve(&server);
        let platform = WasmPlatform::new(&Options::default(), signal(), Some(client), None);
        let mut pair = Pair::start(&*platform.psi(), &*platform.psi(), keys());
        assert_eq!(pair.finish().await, inline);
        assert!(pair.notified.get() > 0, "the sessions never answered later");

        let platform = WasmPlatform::new(&Options::default(), signal(), None, None);
        let mut pair = Pair::start(&*platform.psi(), &*platform.psi(), keys());
        assert_eq!(pair.finish().await, inline);
        assert_eq!(
            pair.notified.get(),
            0,
            "inline sessions answer in their calls"
        );
    }
}
