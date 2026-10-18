// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The browser driver: [`WasmIo`] runs a core node on the worker's
//! executor, with WebSocket links and data channels from the window.
//!
//! - **Time** is [`crate::clock`]'s.
//! - **Dials.** `wss://` URIs, and `ws://` ones when the config allows
//!   them. Each dial is a task, bounded by `NodeConfig::dial_timeout`,
//!   that reports `Input::Connected` with a [`WasmTransport::Ws`] or
//!   `Input::DialFailed`. A browser has no TCP: the node refuses `tcp://`
//!   before a dial starts.
//! - **WebRTC.** The window's peer connections sit behind a
//!   [`ProxyRtc`] in the node task. Its commands go to the
//!   [`SignalPort`] task, which talks to the window and attaches each
//!   open data channel as a [`WasmTransport::Channel`].
//! - **Links.** Each link has a reader task and a writer task. The reader
//!   copies its socket's [`Inbox`] into the node's recycled read buffers;
//!   the writer sends the node's write buffers and returns them, and
//!   closes the socket when the node drops the link.
//!
//! The node task has the driver spawn, in order: the signal task at start,
//! a link's reader then its writer when the link is attached, and the
//! queued dials after each pass. Tasks end on their own: dials at their
//! timeout, a link's writer with its link (its reader is aborted then),
//! and the signal task once the node and its endpoint are gone, after it
//! has passed on their last commands. So [`NodeIo::stop`] has nothing to
//! do.

use std::cell::Cell;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

use strims_base::time::{Duration, Time};
use strims_node_core::{
    DialId, IdleOk, Input, Inputs, Interfaces, LinkChannels, LinkKey, LinkTasks, NodeConfig,
    NodeError, NodeIo, NodeSetup, ProxyRtc, RtcCommand, TaskGuard,
};
use strims_vnic::ws::WsOptions;
use strims_vnic::{DialTarget, LinkConfig};
use tokio::sync::mpsc;
use web_sys::MessagePort;

use crate::clock::{self, Sleep};
use crate::inbox::Inbox;
use crate::rtc::{DcSocket, SignalPort};
use crate::runtime;
use crate::ws::{self, WsSocket};

/// The link interfaces a browser has: `wss://` dials, and no TCP.
pub const BROWSER_INTERFACES: Interfaces = Interfaces {
    tcp: false,
    wss: true,
};

/// Connection ids of successive nodes in one worker start this far apart,
/// so a node's ids never meet those of the node before it while the
/// window still hosts both.
const ID_SPACING: u64 = 1 << 32;

thread_local! {
    /// The next node's connection id base, in units of `ID_SPACING`.
    static NEXT_ID_BASE: Cell<u64> = const { Cell::new(0) };
}

fn next_id_base() -> u64 {
    // 2^21 bases of 2^32 ids fill the JS-safe range; after that they wrap.
    NEXT_ID_BASE.with(|n| {
        let k = n.get();
        n.set((k + 1) % (1 << 21));
        k * ID_SPACING
    })
}

/// A browser link's transport.
pub enum WasmTransport {
    Ws(WsSocket),
    /// A data channel the window transferred, open.
    Channel(DcSocket),
}

impl fmt::Debug for WasmTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WasmTransport::Ws(s) => f.debug_tuple("Ws").field(s).finish(),
            WasmTransport::Channel(c) => f.debug_tuple("Channel").field(c).finish(),
        }
    }
}

/// The browser driver (see the [module docs](self)).
pub struct WasmIo {
    allow_insecure_ws: bool,
    dial_timeout: Duration,
    ws: WsOptions,
    /// `WebRtcConfig::max_buffered_amount`, for the channel writers.
    max_buffered: usize,
    /// Until [`NodeIo::start`] spawns it.
    signal: Option<SignalPort>,
    /// Holds the signal task's command queue open while the node runs,
    /// with or without a WebRTC endpoint.
    _commands: mpsc::UnboundedSender<RtcCommand>,
}

impl fmt::Debug for WasmIo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmIo")
            .field("allow_insecure_ws", &self.allow_insecure_ws)
            .field("dial_timeout", &self.dial_timeout)
            .finish_non_exhaustive()
    }
}

impl WasmIo {
    /// The driver for a node of `config`, talking to the window over
    /// `signal`, and the node's setup: the browser's interfaces, no
    /// listeners or URIs, and a [`ProxyRtc`] when WebRTC is enabled.
    ///
    /// The signal port's messages are handled from now on (see
    /// [`SignalPort::new`]). A browser can't listen, so a config with
    /// listen addresses is refused.
    pub fn new(config: &NodeConfig, signal: MessagePort) -> Result<(NodeSetup, WasmIo), NodeError> {
        if config.tcp_listen.is_some() || config.ws_listen.is_some() {
            return Err(NodeError::Unsupported("listeners in a browser"));
        }
        let (commands, commands_rx) = mpsc::unbounded_channel();
        let w = &config.webrtc;
        let rtc = w
            .enabled
            .then(|| ProxyRtc::factory(w, next_id_base(), commands.clone()));
        let max_buffered = w.max_buffered_amount.max(1);
        let setup = NodeSetup {
            interfaces: BROWSER_INTERFACES,
            tcp_addr: None,
            ws_addr: None,
            tcp_uri: None,
            ws_uri: None,
            webrtc_addrs: Vec::new(),
            rtc,
        };
        let io = WasmIo {
            allow_insecure_ws: config.allow_insecure_ws,
            dial_timeout: config.dial_timeout,
            ws: config.ws,
            max_buffered,
            signal: Some(SignalPort::new(signal, commands_rx, max_buffered)),
            _commands: commands,
        };
        Ok((setup, io))
    }
}

/// Runs `f` for at most `d`; a non-positive `d` means no limit. `None` on
/// timeout.
pub(crate) async fn with_timeout<F: Future>(d: Duration, f: F) -> Option<F::Output> {
    if d <= Duration::ZERO {
        return Some(f.await);
    }
    let sleep = Sleep::until(clock::now() + d);
    tokio::select! {
        biased;
        v = f => Some(v),
        () = sleep => None,
    }
}

/// A link's reader: fills the node's free read buffers from the socket's
/// inbox. A wait for bytes longer than `read_timeout` (none when it isn't
/// positive) fails the link, unless `idle_ok` is set then, which renews
/// the wait.
pub(crate) async fn reader(
    link: LinkKey,
    inbox: Inbox,
    mut free: mpsc::Receiver<Vec<u8>>,
    inputs: Inputs<WasmTransport>,
    read_timeout: Duration,
    idle_ok: Option<IdleOk>,
) {
    let error = loop {
        let Some(mut buf) = free.recv().await else {
            return;
        };
        let res = loop {
            match with_timeout(read_timeout, inbox.read(&mut buf)).await {
                None if idle_ok.as_ref().is_some_and(IdleOk::get) => {}
                res => break res,
            }
        };
        match res {
            None => break "read timeout".to_owned(),
            Some(Err(e)) => break e,
            Some(Ok(len)) => {
                if inputs.send(Input::Read { link, buf, len }).await.is_err() {
                    return;
                }
            }
        }
    };
    let _ = inputs.send(Input::LinkDown { link, error }).await;
}

/// Dials a WebSocket target and reports the outcome.
async fn dial_ws(
    dial: DialId,
    target: DialTarget,
    allow_insecure_ws: bool,
    timeout: Duration,
    inputs: Inputs<WasmTransport>,
) {
    let res = match target {
        DialTarget::Ws(t) if t.secure || allow_insecure_ws => {
            with_timeout(timeout, ws::dial(&t.url))
                .await
                .unwrap_or_else(|| Err("dial timeout".to_owned()))
        }
        DialTarget::Ws(_) => Err("ws:// dials are not allowed".to_owned()),
        DialTarget::Tcp(_) => Err("a browser has no tcp".to_owned()),
    };
    let msg = match res {
        Ok(socket) => Input::Connected {
            dial: Some(dial),
            transport: WasmTransport::Ws(socket),
            config: LinkConfig::ws(),
            leftover: Vec::new(),
            notsent_lowat: None,
            rtc: None,
        },
        Err(error) => Input::DialFailed { dial, error },
    };
    let _ = inputs.send(msg).await;
}

fn guard(task: runtime::TaskHandle) -> TaskGuard {
    TaskGuard::new(move || task.abort())
}

impl NodeIo for WasmIo {
    type Transport = WasmTransport;
    type Sleep = Sleep;

    fn now(&self) -> Time {
        clock::now()
    }

    fn new_sleep(&self) -> Sleep {
        clock::new_sleep()
    }

    fn reset_sleep(&self, sleep: Pin<&mut Sleep>, t: Time) {
        clock::reset_sleep(sleep, t);
    }

    fn yield_now(&self) -> impl Future<Output = ()> {
        clock::yield_now()
    }

    /// Spawns the signal task, which needs the node's inputs.
    fn start(&mut self, inputs: &Inputs<WasmTransport>) {
        if let Some(signal) = self.signal.take() {
            // Detached: it ends once the node's commands do.
            drop(runtime::spawn(signal.run(inputs.clone())));
        }
    }

    fn dial(&mut self, id: DialId, target: DialTarget, inputs: &Inputs<WasmTransport>) {
        // Detached: the dial timeout bounds it.
        drop(runtime::spawn(dial_ws(
            id,
            target,
            self.allow_insecure_ws,
            self.dial_timeout,
            inputs.clone(),
        )));
    }

    fn spawn_link(
        &mut self,
        link: LinkKey,
        transport: WasmTransport,
        channels: LinkChannels<WasmTransport>,
    ) -> LinkTasks {
        let LinkChannels {
            read_free,
            writes,
            inputs,
        } = channels;
        match transport {
            WasmTransport::Ws(socket) => {
                let flag = IdleOk::default();
                let reader = runtime::spawn(reader(
                    link,
                    socket.inbox(),
                    read_free,
                    inputs.clone(),
                    self.ws.read_timeout,
                    Some(flag.clone()),
                ));
                drop(runtime::spawn(ws::writer(
                    link,
                    socket,
                    writes,
                    inputs,
                    self.ws.write_timeout,
                )));
                LinkTasks {
                    reader: guard(reader),
                    idle_ok: Some(flag),
                }
            }
            // ICE and the connection state end a dead channel: no read
            // timeout.
            WasmTransport::Channel(channel) => {
                let reader = runtime::spawn(reader(
                    link,
                    channel.inbox(),
                    read_free,
                    inputs.clone(),
                    Duration::ZERO,
                    None,
                ));
                drop(runtime::spawn(crate::rtc::writer(
                    link,
                    channel,
                    writes,
                    inputs,
                    self.max_buffered,
                )));
                LinkTasks {
                    reader: guard(reader),
                    idle_ok: None,
                }
            }
        }
    }

    fn stop(&mut self) {}
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

    use js_sys::Uint8Array;
    use slotmap::KeyData;
    use strims_rpc::Timer;
    use wasm_bindgen_test::wasm_bindgen_test;

    use super::*;
    use crate::clock::WasmClock;

    async fn sleep(d: Duration) {
        WasmClock.sleep_until(clock::now() + d).await;
    }

    /// A silent link outlives its read timeout while `IdleOk` is set,
    /// still delivers bytes, and fails on the next timeout once cleared.
    #[wasm_bindgen_test]
    async fn idle_ok_outwaits_the_read_timeout() {
        let link = LinkKey::from(KeyData::from_ffi(1));
        let inbox = Inbox::new();
        let (free_tx, free) = mpsc::channel(1);
        free_tx.try_send(vec![0; 16]).unwrap();
        let (inputs_tx, mut inputs) = mpsc::channel(4);
        let idle_ok = IdleOk::default();
        idle_ok.set(true);
        let timeout = Duration::from_millis(30);
        let _task = runtime::spawn(reader(
            link,
            inbox.clone(),
            free,
            inputs_tx,
            timeout,
            Some(idle_ok.clone()),
        ));

        sleep(timeout * 4).await;
        assert!(inputs.try_recv().is_err(), "silence while idle is fine");
        inbox.push(Uint8Array::from(&b"late"[..]));
        match inputs.recv().await.unwrap() {
            Input::Read { buf, len, .. } => {
                assert_eq!(&buf[..len], b"late");
                free_tx.try_send(buf).unwrap();
            }
            _ => panic!("expected a read"),
        }

        idle_ok.set(false);
        let started = clock::now();
        match inputs.recv().await.unwrap() {
            Input::LinkDown { error, .. } => assert_eq!(error, "read timeout"),
            _ => panic!("expected the link to fail"),
        }
        assert!(clock::now() - started >= timeout);
    }

    /// Without a read timeout a link waits for bytes indefinitely; it
    /// ends when its socket does, after the bytes before the end.
    #[wasm_bindgen_test]
    async fn a_reader_reports_the_end_after_the_bytes() {
        let link = LinkKey::from(KeyData::from_ffi(2));
        let inbox = Inbox::new();
        let (free_tx, free) = mpsc::channel(1);
        free_tx.try_send(vec![0; 16]).unwrap();
        let (inputs_tx, mut inputs) = mpsc::channel(4);
        let _task = runtime::spawn(reader(
            link,
            inbox.clone(),
            free,
            inputs_tx,
            Duration::ZERO,
            None,
        ));
        sleep(Duration::from_millis(20)).await;
        assert!(inputs.try_recv().is_err());
        inbox.push(Uint8Array::from(&[1u8, 2][..]));
        inbox.end("data channel closed");
        match inputs.recv().await.unwrap() {
            Input::Read { buf, len, .. } => {
                assert_eq!(&buf[..len], [1, 2]);
                free_tx.try_send(buf).unwrap();
            }
            _ => panic!("expected a read"),
        }
        match inputs.recv().await.unwrap() {
            Input::LinkDown { error, .. } => assert_eq!(error, "data channel closed"),
            _ => panic!("expected the link to fail"),
        }
    }
}
