// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The node task and the interface its drivers implement.
//!
//! The node task ([`NodeTask`]) owns the composed state machines (`State`)
//! and runs the event loop. A driver owns sockets, timers and tasks, and
//! the loop reaches them only through [`NodeIo`]: the time, one timer,
//! yielding, and spawning the driver's tasks. [`start`] builds a node from
//! what the driver set up ([`NodeSetup`]); the driver then runs
//! [`NodeTask::run`] on its executor.
//!
//! ```text
//! loop {
//!     wait (biased): transport input | command | peer rpc write | timer at State::poll_timeout
//!     drain a bounded batch of ready inputs and commands
//!     State::handle_timeouts(now)
//!     State::pump(now)     // events, vpn.flush, vnic.poll_egress, link writes
//!     start queued dials
//! }
//! ```
//!
//! Transport tasks report to the node task with [`Input`]s over one
//! bounded channel ([`Inputs`]), and link tasks trade recycled buffers with
//! it ([`LinkChannels`]).
//!
//! **Queries** (`Node::metrics`, `Node::peers`) wait in a mailbox that the
//! loop drains after every pass, before it waits again, so they are
//! answered in a poll the node task makes anyway. Every poll of a task
//! shifts how a single-threaded executor orders the tasks after it, and
//! under a backlogged upload cap a different order is a different
//! schedule, so a poll made only to answer a query would let sampling
//! change the run. The loop publishes whether its next pass is due within
//! `NodeConfig::query_delay`. When it isn't (an idle node), the caller
//! sends `Command::Wake`, and the node answers without running timers,
//! egress or dials.
//!
//! **Order.** The loop's `select!` is `biased`, and it asks the driver to
//! spawn in a fixed order: [`NodeIo::start`] once, before the first wait;
//! a link's tasks when the link is attached ([`NodeIo::spawn_link`]); the
//! queued dials after each pass ([`NodeIo::dial`]). See
//! [Determinism](crate#determinism) for the rest of what keeps runs
//! reproducible.

use std::fmt;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering, fence};

use strims_base::time::{Duration, Time};
use strims_vnic::{DialTarget, LinkConfig, LinkDescription, LinkInterface, PeerKey as LinkKey};
use tokio::sync::{mpsc, oneshot};

use crate::config::{BufferConfig, NodeConfig};
use crate::error::NodeError;
use crate::handle::{Node, NodeEvents, NodeInfo};
use crate::rpc::Outgoing;
use crate::rtc::{RtcConn, RtcFactory, RtcInput};
use crate::state::{DialId, LinkIo, PublisherId, ReaderId, State, WriteOp};

/// Inputs and commands handled per wakeup before timers and egress run.
const BATCH: usize = 64;

/// The smallest write a link is cut to when its socket bounds the kernel's
/// unsent bytes.
const MIN_WRITE: usize = 4 * 1024;

/// A closure run on the node task's state.
pub(crate) type Call = Box<dyn FnOnce(&mut State, Time) + Send>;

/// Calls into the node task. Replies go back over oneshots (or, for reads
/// and writes, the handle's own reply channel, so the hot path doesn't
/// allocate).
pub(crate) enum Command {
    /// Runs on the state with the current time.
    Call(Call),
    /// Answer the queued queries now, without a pass (see the module docs).
    Wake,
    Read {
        reader: ReaderId,
        buf: Vec<u8>,
        max: usize,
    },
    Write {
        publisher: PublisherId,
        op: WriteOp,
        buf: Vec<u8>,
    },
    /// Move the reader back to the offset; answered on its rewind channel.
    Rewind {
        reader: ReaderId,
        offset: u64,
    },
    /// Give up the reader's waiting rewind (its future was dropped).
    CancelRewind(ReaderId),
    CloseReader(ReaderId),
    ClosePublisher(PublisherId),
    Shutdown(oneshot::Sender<()>),
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Command")
    }
}

/// Everything the node task hears from the driver's tasks. `T` is the
/// driver's transport ([`NodeIo::Transport`]).
pub enum Input<T> {
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
    /// A transport is ready for vnic: a dial (`dial`) or an accepted
    /// connection finished its transport handshake (TCP mux init,
    /// WebSocket upgrade), or the data channel of WebRTC connection `rtc`
    /// opened. `leftover` was read past the handshake.
    ///
    /// A data channel becomes a link of `config` (`LinkConfig::webrtc`)
    /// while its connection is live and has none; otherwise the transport
    /// is dropped.
    Connected {
        dial: Option<DialId>,
        transport: T,
        config: LinkConfig,
        leftover: Vec<u8>,
        /// The socket's `TCP_NOTSENT_LOWAT`, when set: the link's writes
        /// are cut to it.
        notsent_lowat: Option<u32>,
        rtc: Option<RtcConn>,
    },
    DialFailed {
        dial: DialId,
        error: String,
    },
    /// An accepted connection was dropped before vnic.
    AcceptFailed {
        error: String,
    },
    /// `buf[..len]` arrived from `from` on WebRTC UDP socket `socket`
    /// ([`RtcEndpoint::handle_datagram`](crate::rtc::RtcEndpoint::handle_datagram)).
    Datagram {
        socket: usize,
        buf: Vec<u8>,
        len: usize,
        from: SocketAddr,
    },
    /// The WebRTC interface's STUN servers, (re)resolved.
    StunServers(Vec<SocketAddr>),
    /// A signal about a WebRTC connection outside the node task.
    Signal(RtcInput),
}

/// Where the driver's tasks send [`Input`]s. Bounded
/// (`NodeConfig::input_queue`): a task waits when the node is behind.
pub type Inputs<T> = mpsc::Sender<Input<T>>;

/// A link's channels to the node task, for its I/O tasks.
///
/// ```text
///   reader --Input::Read{buf}--> node --read_free--> reader
///   node --writes{buf}--> writer --Input::WriteDone{buf}--> node
/// ```
///
/// A link holds at most `BufferConfig::read_buffers` read buffers and
/// `BufferConfig::write_buffers` write buffers, so its memory is fixed.
#[derive(Debug)]
pub struct LinkChannels<T> {
    /// Empty read buffers. The reader fills one and sends it as
    /// [`Input::Read`].
    pub read_free: mpsc::Receiver<Vec<u8>>,
    /// Filled write buffers, each at most one link MTU. The writer writes
    /// each and returns it with [`Input::WriteDone`]; when the channel
    /// ends (the link went), it ends the transport.
    pub writes: mpsc::Receiver<Vec<u8>>,
    pub inputs: Inputs<T>,
    /// The node captures its ppspp traffic: the driver returns a
    /// [`LinkProbe`] if it can read the transport.
    pub probe: bool,
}

/// What [`NodeIo::spawn_link`] started.
#[derive(Debug)]
pub struct LinkTasks {
    /// Aborts the reader. The node drops it first when the link goes; the
    /// writer ends when its channel does.
    pub reader: TaskGuard,
    /// Set when the reader honours [`IdleOk`].
    pub idle_ok: Option<IdleOk>,
    /// Reads the transport for the capture, when [`LinkChannels::probe`]
    /// asked and the driver can.
    pub probe: Option<Box<dyn LinkProbe>>,
}

/// Reads the state under a link for a ppspp capture: the socket's or the
/// channel's, or in a simulation, its model's. The node samples it at
/// passes it already makes; it must not block.
pub trait LinkProbe: Send + fmt::Debug {
    /// The state now, if the transport can tell.
    fn sample(&mut self, now: Time) -> Option<strims_ppspp_capture::Sample>;
}

/// Aborts a task when dropped (or at [`TaskGuard::abort`]).
pub struct TaskGuard(Option<Box<dyn FnOnce() + Send>>);

impl TaskGuard {
    /// A guard that runs `abort` once, when it is dropped or aborted.
    pub fn new(abort: impl FnOnce() + Send + 'static) -> TaskGuard {
        TaskGuard(Some(Box::new(abort)))
    }

    /// Aborts the task now.
    pub fn abort(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}

impl From<futures_util::future::AbortHandle> for TaskGuard {
    fn from(h: futures_util::future::AbortHandle) -> TaskGuard {
        TaskGuard::new(move || h.abort())
    }
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        self.abort();
    }
}

impl fmt::Debug for TaskGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TaskGuard")
            .field(&self.0.as_ref().map(|_| "armed"))
            .finish()
    }
}

// TODO: remove once protocol v3 makes keepalives mandatory.
/// Whether a link may stay silent past its read timeout, shared by the node
/// task (which sets it) and the link's reader (which reads it when a read
/// times out). Set for an established peer that hasn't sent a vnic
/// keepalive yet. A peer that sends keepalives writes at least every
/// keepalive interval (10 s), so for it the read timeout applies again.
#[derive(Clone, Debug, Default)]
pub struct IdleOk(Arc<AtomicBool>);

impl IdleOk {
    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn set(&self, v: bool) {
        self.0.store(v, Ordering::Relaxed);
    }
}

/// The link interfaces a driver has. Natively `{ tcp: true, wss: false }`;
/// in a browser `{ tcp: false, wss: true }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interfaces {
    /// TCP links: dialing `tcp://`, and a TCP candidate in peer exchanges.
    /// Without it a `tcp://` dial is `NodeError::Unsupported`.
    pub tcp: bool,
    /// Dialing `wss://`.
    pub wss: bool,
}

/// What a driver set up before the node starts.
pub struct NodeSetup {
    pub interfaces: Interfaces,
    /// The listeners, as bound.
    pub tcp_addr: Option<SocketAddr>,
    pub ws_addr: Option<SocketAddr>,
    /// The URIs advertised to peer exchange and in [`NodeInfo`].
    pub tcp_uri: Option<String>,
    pub ws_uri: Option<String>,
    /// The WebRTC interface's UDP sockets, as bound.
    pub webrtc_addrs: Vec<SocketAddr>,
    /// Builds the WebRTC endpoint when the interface is enabled.
    pub rtc: Option<RtcFactory>,
}

impl fmt::Debug for NodeSetup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeSetup")
            .field("interfaces", &self.interfaces)
            .field("tcp_addr", &self.tcp_addr)
            .field("ws_addr", &self.ws_addr)
            .field("tcp_uri", &self.tcp_uri)
            .field("ws_uri", &self.ws_uri)
            .field("webrtc_addrs", &self.webrtc_addrs)
            .field("rtc", &self.rtc.is_some())
            .finish()
    }
}

/// A driver: time, a timer, and the tasks that own its sockets.
///
/// Every method returns at once; the work runs in the driver's tasks,
/// which report back with [`Input`]s.
pub trait NodeIo: 'static {
    /// A connected transport, from [`Input::Connected`].
    type Transport;
    /// The node's one timer.
    type Sleep: Future<Output = ()>;

    /// The current time, at `Time::PRECISION`.
    fn now(&self) -> Time;

    /// A timer, not yet armed for any deadline the node cares about.
    fn new_sleep(&self) -> Self::Sleep;

    /// Re-arms the timer to fire at or after `t`.
    fn reset_sleep(&self, sleep: Pin<&mut Self::Sleep>, t: Time);

    /// Lets the driver's other tasks run, then resumes.
    fn yield_now(&self) -> impl Future<Output = ()>;

    /// Spawns the driver's own tasks (listeners, sockets, resolvers). Runs
    /// once, before the loop first waits.
    fn start(&mut self, inputs: &Inputs<Self::Transport>);

    /// Spawns a dial of `target`, which reports [`Input::Connected`] with
    /// `dial: Some(dial)` or [`Input::DialFailed`].
    fn dial(&mut self, dial: DialId, target: DialTarget, inputs: &Inputs<Self::Transport>);

    /// Spawns the link's reader, then its writer.
    fn spawn_link(
        &mut self,
        link: LinkKey,
        transport: Self::Transport,
        channels: LinkChannels<Self::Transport>,
    ) -> LinkTasks;

    /// Aborts the tasks [`NodeIo::start`] spawned. Runs once, before the
    /// node closes its links.
    fn stop(&mut self);
}

/// The node task: run it with [`NodeTask::run`] on the driver's executor.
pub struct NodeTask<I: NodeIo> {
    state: State,
    io: I,
    buffers: BufferConfig,
    input_tx: Inputs<I::Transport>,
    input_rx: mpsc::Receiver<Input<I::Transport>>,
    cmd_rx: mpsc::Receiver<Command>,
    /// Calls from peer RPC writers and messages from vpn port writers.
    /// Unbounded, but each writer's credit bounds what it has queued (see
    /// [`crate::rpc`]), and each port closes once.
    rpc_rx: mpsc::UnboundedReceiver<Outgoing>,
    /// Read-only calls, answered after passes (see the module docs).
    query_rx: mpsc::Receiver<Call>,
    /// Whether the next pass is due within `query_delay`.
    answers_soon: Arc<AtomicBool>,
    query_delay: Duration,
    shutdown: Option<oneshot::Sender<()>>,
}

impl<I: NodeIo> fmt::Debug for NodeTask<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeTask")
            .field("host_id", &self.state.vnic.id())
            .finish_non_exhaustive()
    }
}

/// Builds a node's state machines on a driver. Returns the API handle, its
/// event stream, and the node task for the driver to run.
pub fn start<I: NodeIo>(
    config: &NodeConfig,
    setup: NodeSetup,
    io: I,
) -> Result<(Node, NodeEvents, NodeTask<I>), NodeError> {
    config.profile_public_key()?;
    let describe = |iface: LinkInterface, uri: &Option<String>| {
        uri.as_ref().map(|u| LinkDescription {
            interface: iface.name().to_owned(),
            description: u.clone(),
        })
    };
    let (events_tx, events) = mpsc::channel(config.event_queue.max(1));
    let (rpc_tx, rpc_rx) = mpsc::unbounded_channel();
    let state = State::new(
        config,
        describe(LinkInterface::Tcp, &setup.tcp_uri),
        describe(LinkInterface::Ws, &setup.ws_uri),
        setup.interfaces,
        setup.rtc,
        io.now(),
        events_tx,
        rpc_tx,
    )?;
    let info = Arc::new(NodeInfo {
        host_id: state.vnic.id(),
        profile_key: config.profile_key.public.clone(),
        tcp_addr: setup.tcp_addr,
        ws_addr: setup.ws_addr,
        tcp_uri: setup.tcp_uri,
        ws_uri: setup.ws_uri,
        webrtc_addrs: setup.webrtc_addrs,
    });
    let (commands, cmd_rx) = mpsc::channel(config.command_queue.max(1));
    let (queries, query_rx) = mpsc::channel(config.command_queue.max(1));
    let answers_soon = Arc::new(AtomicBool::new(false));
    let (input_tx, input_rx) = mpsc::channel(config.input_queue.max(1));
    let node = Node::new(
        commands,
        queries,
        answers_soon.clone(),
        info,
        setup.interfaces,
    );
    let task = NodeTask {
        state,
        io,
        buffers: config.buffers,
        input_tx,
        input_rx,
        cmd_rx,
        rpc_rx,
        query_rx,
        answers_soon,
        query_delay: config.query_delay,
        shutdown: None,
    };
    Ok((node, NodeEvents::new(events), task))
}

impl<I: NodeIo> NodeTask<I> {
    /// The event loop. Ends on `Node::shutdown` or when every handle is
    /// dropped.
    pub async fn run(mut self) {
        self.io.start(&self.input_tx);
        let mut sleep = pin!(self.io.new_sleep());
        // The deadline the timer is armed for. A wakeup that moves no
        // deadline (a query) leaves the timer untouched.
        let mut armed: Option<Time> = None;
        let mut again = false;
        let mut last = self.io.now();
        loop {
            // A deadline at or before the last pass would spin; the state
            // machines handled everything due at `last`.
            let deadline = self
                .state
                .poll_timeout()
                .map(|t| t.max(last + Duration::MILLISECOND));
            self.answer_queries(again, deadline);
            if again {
                // The pump stopped with work left: let the link tasks run,
                // then continue without waiting.
                self.io.yield_now().await;
            } else {
                if let Some(t) = deadline
                    && armed != Some(t)
                {
                    self.io.reset_sleep(sleep.as_mut(), t);
                    armed = Some(t);
                }
                tokio::select! {
                    biased;
                    Some(input) = self.input_rx.recv() => self.handle_input(input),
                    cmd = self.cmd_rx.recv() => match cmd {
                        // No pass, and `last` stays, so the wait resumes
                        // with the same deadline. A timer that came due
                        // meanwhile fires on the next select.
                        Some(c @ Command::Wake) => {
                            self.handle_command(c);
                            continue;
                        }
                        Some(c) => {
                            if self.handle_command(c) {
                                break;
                            }
                        }
                        None => break,
                    },
                    Some(o) = self.rpc_rx.recv() => self.state.outgoing(o),
                    () = &mut sleep, if deadline.is_some() => {}
                }
            }
            for _ in 0..BATCH {
                match self.input_rx.try_recv() {
                    Ok(input) => self.handle_input(input),
                    Err(_) => break,
                }
            }
            for _ in 0..BATCH {
                match self.rpc_rx.try_recv() {
                    Ok(o) => self.state.outgoing(o),
                    Err(_) => break,
                }
            }
            let mut stop = false;
            for _ in 0..BATCH {
                match self.cmd_rx.try_recv() {
                    Ok(c) => {
                        if self.handle_command(c) {
                            stop = true;
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            if stop {
                break;
            }
            let now = self.io.now();
            self.state.handle_timeouts(now);
            again = self.state.pump(now);
            self.start_dials();
            self.state.metrics.loop_iterations += 1;
            last = now;
        }
        self.stop();
    }

    fn stop(&mut self) {
        self.io.stop();
        let now = self.io.now();
        // Closing drops every link's I/O: writers shut their transports
        // down.
        self.state.close(now);
        if let Some(done) = self.shutdown.take() {
            let _ = done.send(());
        }
    }

    fn start_dials(&mut self) {
        while let Some((id, target)) = self.state.next_dial() {
            self.io.dial(id, target, &self.input_tx);
        }
    }

    /// Publishes whether the next pass is due within `query_delay`, then
    /// answers the queued queries. In that order: a caller queues first and
    /// reads the flag second, so a query this drain misses sees the new
    /// flag and wakes the node if it must.
    fn answer_queries(&mut self, again: bool, deadline: Option<Time>) {
        let now = self.io.now();
        let soon = again || deadline.is_some_and(|t| t <= now + self.query_delay);
        self.answers_soon.store(soon, Ordering::SeqCst);
        fence(Ordering::SeqCst);
        for _ in 0..BATCH {
            let Ok(f) = self.query_rx.try_recv() else {
                break;
            };
            self.state.metrics.queries += 1;
            f(&mut self.state, now);
        }
    }

    /// Returns true to stop the loop.
    fn handle_command(&mut self, c: Command) -> bool {
        if !matches!(c, Command::Wake) {
            self.state.metrics.commands += 1;
        }
        let now = self.io.now();
        match c {
            // `answer_queries` answers them at the top of the loop.
            Command::Wake => self.state.metrics.query_wakes += 1,
            Command::Call(f) => f(&mut self.state, now),
            Command::Read { reader, buf, max } => self.state.read(reader, buf, max, now),
            Command::Write { publisher, op, buf } => self.state.write(publisher, op, buf, now),
            Command::Rewind { reader, offset } => self.state.rewind(reader, offset, now),
            Command::CancelRewind(r) => self.state.cancel_rewind(r, now),
            Command::CloseReader(r) => self.state.close_reader(r, now),
            Command::ClosePublisher(p) => self.state.close_publisher(p),
            Command::Shutdown(done) => {
                self.shutdown = Some(done);
                return true;
            }
        }
        false
    }

    fn handle_input(&mut self, input: Input<I::Transport>) {
        let now = self.io.now();
        match input {
            Input::Read { link, mut buf, len } => {
                if let Some(data) = buf.get_mut(..len) {
                    self.state.handle_read(link, now, data);
                }
                self.state.return_read_buf(link, buf);
            }
            Input::WriteDone { link, buf } => self.state.return_write_buf(link, buf),
            Input::LinkDown { link, error } => self.state.link_down(link, error, now),
            Input::Connected {
                dial,
                transport,
                config,
                mut leftover,
                notsent_lowat,
                rtc,
            } => self.attach(
                dial,
                rtc,
                transport,
                config,
                notsent_lowat,
                &mut leftover,
                now,
            ),
            Input::DialFailed { dial, error } => self.state.dial_failed(dial, error, now),
            Input::AcceptFailed { error } => {
                tracing::debug!(%error, "accept");
                self.state.metrics.accept_errors += 1;
            }
            Input::Datagram {
                socket,
                buf,
                len,
                from,
            } => {
                if let Some(data) = buf.get(..len) {
                    self.state.handle_datagram(socket, from, data, now);
                }
                self.state.return_udp_buf(socket, buf);
            }
            Input::StunServers(addrs) => self.state.set_stun_servers(addrs),
            Input::Signal(s) => self.state.handle_rtc_signal(s, now),
        }
    }

    /// Adds a connected transport to vnic and starts its I/O tasks.
    ///
    /// Bytes in the link's write buffers and in vnic's link buffer are past
    /// QoS. A socket with a bound on its unsent bytes therefore gets writes
    /// of that size, and vnic keeps at most one MTU plus one write for it.
    #[allow(clippy::too_many_arguments)]
    fn attach(
        &mut self,
        dial: Option<DialId>,
        rtc: Option<RtcConn>,
        transport: I::Transport,
        mut config: LinkConfig,
        notsent_lowat: Option<u32>,
        leftover: &mut [u8],
        now: Time,
    ) {
        if let Some(conn) = rtc
            && !self.state.rtc_accepts_stream(conn)
        {
            tracing::debug!(?conn, "a data channel without a connection to carry");
            return;
        }
        let kind = config.kind;
        let max_write = notsent_lowat.map_or(config.mtu, |l| {
            usize::try_from(l)
                .unwrap_or(usize::MAX)
                .clamp(MIN_WRITE.min(config.mtu), config.mtu)
        });
        if max_write < config.mtu {
            config.send_buffer = config.mtu.saturating_add(max_write);
        }
        let link = match self.state.vnic.add_link(now, config) {
            Ok(l) => l,
            Err(e) => {
                if let Some(d) = dial {
                    self.state.dial_failed(d, e.to_string(), now);
                }
                if let Some(conn) = rtc {
                    self.state.rtc_link_failed(conn, e.to_string(), now);
                }
                return;
            }
        };
        let io = self.spawn_link_io(link, transport, max_write);
        self.state.insert_link(link, kind, io, dial, rtc);
        if !leftover.is_empty() {
            self.state.handle_read(link, now, leftover);
        }
    }

    /// Sets up the link's buffers and has the driver spawn its reader, then
    /// its writer.
    fn spawn_link_io(
        &mut self,
        link: LinkKey,
        transport: I::Transport,
        max_write: usize,
    ) -> LinkIo {
        let b = self.buffers;
        let (nr, nw) = (b.read_buffers.max(1), b.write_buffers.max(1));
        let (write_tx, write_rx) = mpsc::channel(nw);
        let (read_free_tx, read_free_rx) = mpsc::channel(nr);
        for _ in 0..nr {
            let _ = read_free_tx.try_send(vec![0; b.read_buffer_size.max(1)]);
        }
        let free_write = (0..nw).map(|_| Vec::with_capacity(max_write)).collect();
        let tasks = self.io.spawn_link(
            link,
            transport,
            LinkChannels {
                read_free: read_free_rx,
                writes: write_rx,
                inputs: self.input_tx.clone(),
                probe: self.state.capturing(),
            },
        );
        LinkIo {
            write_tx,
            free_write,
            write_buffers: nw,
            max_write,
            read_free_tx,
            reader: tasks.reader,
            idle_ok: tasks.idle_ok,
            probe: tasks.probe,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )]

    use std::collections::VecDeque;
    use std::task::Poll;

    use slotmap::KeyData;
    use strims_crypto::key_from_seed;
    use strims_vnic::webrtc::{RemoteDescriptionAction, SdpType, SessionDescription};
    use strims_vnic::{CandidateError, LinkCandidate, LinkKind};
    use strims_vpn::ExchangeId;

    use super::*;
    use crate::error::ReadError;
    use crate::event::{DriverMetrics, NodeEvent, PeerInfo};
    use crate::exchange::CandidateState;
    use crate::rtc::{RtcCloseReason, RtcEndpoint, RtcEvent};

    fn epoch() -> Time {
        Time::from_unix_millis(1_700_000_000_000)
    }

    /// A driver whose links are channels the test pumps by hand.
    #[derive(Default)]
    struct FakeIo {
        links: Vec<(LinkKey, LinkChannels<()>)>,
    }

    impl NodeIo for FakeIo {
        type Transport = ();
        type Sleep = std::future::Pending<()>;

        fn now(&self) -> Time {
            epoch()
        }

        fn new_sleep(&self) -> Self::Sleep {
            std::future::pending()
        }

        fn reset_sleep(&self, _: Pin<&mut Self::Sleep>, _: Time) {}

        fn yield_now(&self) -> impl Future<Output = ()> {
            std::future::ready(())
        }

        fn start(&mut self, _: &Inputs<()>) {}

        fn dial(&mut self, _: DialId, _: DialTarget, _: &Inputs<()>) {}

        fn spawn_link(&mut self, link: LinkKey, (): (), channels: LinkChannels<()>) -> LinkTasks {
            self.links.push((link, channels));
            LinkTasks {
                reader: TaskGuard::new(|| {}),
                idle_ok: None,
                probe: None,
            }
        }

        fn stop(&mut self) {}
    }

    /// An endpoint whose connections live outside the node task: it
    /// answers descriptions by their type and records closes.
    #[derive(Debug)]
    struct ProxyLike {
        next: u64,
        events: VecDeque<RtcEvent>,
        closed: mpsc::UnboundedSender<RtcConn>,
    }

    impl RtcEndpoint for ProxyLike {
        fn create_connection(&mut self, _: Time) -> Result<RtcConn, String> {
            self.next += 1;
            Ok(RtcConn(self.next))
        }

        fn poll_local_description(
            &mut self,
            _: Time,
            _: RtcConn,
        ) -> Poll<Result<SessionDescription, String>> {
            Poll::Pending
        }

        fn set_remote_description(
            &mut self,
            _: Time,
            _: RtcConn,
            desc: &SessionDescription,
        ) -> Result<RemoteDescriptionAction, String> {
            Ok(match desc.sdp_type {
                SdpType::Answer => RemoteDescriptionAction::AwaitLink,
                _ => RemoteDescriptionAction::AwaitInBackground,
            })
        }

        fn close(&mut self, _: Time, conn: RtcConn) {
            let _ = self.closed.send(conn);
            self.events.push_back(RtcEvent::Closed {
                conn,
                reason: RtcCloseReason::Local,
            });
        }

        fn close_all(&mut self, _: Time) {}

        fn poll_event(&mut self) -> Option<RtcEvent> {
            self.events.pop_front()
        }

        fn poll_timeout(&self) -> Option<Time> {
            None
        }

        fn handle_timeout(&mut self, _: Time) {}

        fn flush(&mut self, _: &mut DriverMetrics) -> bool {
            false
        }
    }

    struct Side {
        _node: Node,
        events: NodeEvents,
        task: NodeTask<FakeIo>,
        closed: mpsc::UnboundedReceiver<RtcConn>,
    }

    /// A node without TCP, as a browser runs one.
    fn browser(i: u8) -> Side {
        let config = NodeConfig::new(key_from_seed(&[i; 32]), u64::from(i));
        let (closed_tx, closed) = mpsc::unbounded_channel();
        let setup = NodeSetup {
            interfaces: Interfaces {
                tcp: false,
                wss: true,
            },
            tcp_addr: None,
            ws_addr: None,
            tcp_uri: None,
            ws_uri: None,
            webrtc_addrs: Vec::new(),
            rtc: Some(Box::new(move |_, _| {
                Box::new(ProxyLike {
                    next: 0,
                    events: VecDeque::new(),
                    closed: closed_tx,
                })
            })),
        };
        let (node, events, task) = start(&config, setup, FakeIo::default()).unwrap();
        Side {
            _node: node,
            events,
            task,
            closed,
        }
    }

    fn sdp(sdp_type: SdpType) -> SessionDescription {
        SessionDescription {
            sdp_type,
            sdp: "v=0\r\n".to_owned(),
        }
    }

    /// Exchange `x`'s WebRTC connection, with a remote description of
    /// type `remote` applied.
    fn exchange(s: &mut Side, x: ExchangeId, remote: SdpType) -> RtcConn {
        let pool = s.task.state.test_pool(x, epoch());
        let ifaces: Vec<_> = pool.iter().map(|c| c.interface().to_owned()).collect();
        assert_eq!(ifaces, ["ws", "webrtc"], "no tcp candidate without tcp");
        let conn = pool[1].conn.unwrap();
        s.task.state.test_describe(x, sdp(remote), epoch());
        conn
    }

    fn candidate_state(s: &mut Side, x: ExchangeId) -> CandidateState {
        s.task.state.test_pool(x, epoch())[1].state.clone()
    }

    /// The driver's data channel for `conn` opened.
    fn connected(s: &mut Side, conn: RtcConn) {
        s.task.handle_input(Input::Connected {
            dial: None,
            transport: (),
            config: LinkConfig::webrtc(),
            leftover: Vec::new(),
            notsent_lowat: None,
            rtc: Some(conn),
        });
    }

    /// Moves `from`'s link bytes into `to`'s link. Returns whether any
    /// moved.
    fn carry(from: &mut Side, to: &mut Side) -> bool {
        from.task.state.pump(epoch());
        let mut moved = false;
        loop {
            let (from_link, ch) = from.task.io.links.last_mut().unwrap();
            let Ok(buf) = ch.writes.try_recv() else {
                return moved;
            };
            let from_link = *from_link;
            let (to_link, to_ch) = to.task.io.links.last_mut().unwrap();
            let to_link = *to_link;
            let mut read = to_ch.read_free.try_recv().unwrap();
            read[..buf.len()].copy_from_slice(&buf);
            let len = buf.len();
            to.task.handle_input(Input::Read {
                link: to_link,
                buf: read,
                len,
            });
            from.task.handle_input(Input::WriteDone {
                link: from_link,
                buf,
            });
            moved = true;
        }
    }

    fn peer_added(s: &mut Side) -> Option<PeerInfo> {
        while let Some(e) = s.events.try_recv() {
            if let NodeEvent::PeerAdded(info) = e {
                return Some(info);
            }
        }
        None
    }

    /// A data channel the driver owns becomes a WebRTC stream link of its
    /// connection: the offerer's link is the dialed one, and its candidate
    /// is linked once the peer is up.
    #[test]
    fn a_driver_data_channel_links_like_an_in_process_one() {
        let x = ExchangeId::from(KeyData::from_ffi(1));
        let mut a = browser(1);
        let mut b = browser(2);
        let ca = exchange(&mut a, x, SdpType::Answer);
        let cb = exchange(&mut b, x, SdpType::Offer);
        assert_eq!(candidate_state(&mut a, x), CandidateState::AwaitLink);
        assert_eq!(candidate_state(&mut b, x), CandidateState::Background);

        connected(&mut a, ca);
        connected(&mut b, cb);
        assert_eq!(a.task.state.metrics.webrtc_opened, 1);
        assert_eq!(a.task.state.metrics.accepted, 0);
        for _ in 0..16 {
            let ab = carry(&mut a, &mut b);
            let ba = carry(&mut b, &mut a);
            if !ab && !ba {
                break;
            }
        }
        let pa = peer_added(&mut a).expect("a's peer");
        let pb = peer_added(&mut b).expect("b's peer");
        assert_eq!((pa.kind, pa.dialed), (LinkKind::WebRtc, true));
        assert_eq!((pb.kind, pb.dialed), (LinkKind::WebRtc, false));
        assert_eq!(candidate_state(&mut a, x), CandidateState::Linked);
    }

    /// The driver's channel failing closes its connection and fails the
    /// candidate waiting for it; a channel for a connection that is gone
    /// becomes no link.
    #[test]
    fn a_failed_driver_data_channel_fails_its_candidate() {
        let x = ExchangeId::from(KeyData::from_ffi(1));
        let mut a = browser(1);
        let conn = exchange(&mut a, x, SdpType::Answer);
        connected(&mut a, conn);
        let link = a.task.io.links[0].0;
        assert_eq!(a.task.state.test_link_count(), 1);
        a.task.handle_input(Input::LinkDown {
            link,
            error: "channel closed".to_owned(),
        });
        assert_eq!(a.closed.try_recv(), Ok(conn));
        assert_eq!(
            candidate_state(&mut a, x),
            CandidateState::Failed(CandidateError::Failed("channel closed".to_owned()))
        );
        assert_eq!(a.task.state.test_link_count(), 0);
        assert_eq!(a.task.state.metrics.webrtc_closed, 1);

        connected(&mut a, conn);
        assert_eq!(a.task.io.links.len(), 1, "no tasks for a closed connection");
        assert_eq!(a.task.state.test_link_count(), 0);
    }

    /// Without TCP, `tcp://` dials are refused before any task starts.
    #[test]
    fn tcp_dials_need_the_tcp_interface() {
        let mut a = browser(1);
        let key = "00".repeat(32);
        assert!(matches!(
            a.task
                .state
                .start_dial(&format!("tcp://127.0.0.1:1/{key}"), None),
            Err(NodeError::Unsupported(_))
        ));
        assert!(
            a.task
                .state
                .start_dial("wss://a.example:443/x", None)
                .is_ok()
        );
    }

    /// A remote seed's runner, wired by hand to a node's: the node runs the
    /// swarm with it as a peer.
    struct Remote {
        runner: strims_ppspp::Runner,
        swarm: strims_ppspp::SwarmKey,
        peer: strims_ppspp::PeerKey,
        writer: strims_ppspp::Writer,
        node_peer: strims_ppspp::PeerKey,
        node_swarm: strims_ppspp::SwarmKey,
        now: Time,
        written: u64,
    }

    fn stream_byte(o: u64) -> u8 {
        (o.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 56) as u8
    }

    impl Remote {
        fn new(s: &mut State) -> Remote {
            let now = epoch();
            let key = key_from_seed(&[9; 32]);
            let options = strims_ppspp::SwarmOptions {
                live_window: 1024,
                ..Default::default()
            };
            let (writer, swarm) = strims_ppspp::Writer::new(
                strims_ppspp::WriterOptions {
                    swarm_options: options.clone(),
                    key: key.clone(),
                },
                now,
            )
            .unwrap();
            let mut runner = strims_ppspp::Runner::new(strims_base::rng::det_rng(7));
            let ss = runner.add_swarm(swarm);
            let peer = runner.add_peer(b"node", now);
            runner.run_swarm(peer, ss, 2, 1, now).unwrap();
            let node_swarm = s
                .add_swarm(strims_ppspp::SwarmId::new(&key.public), options)
                .unwrap();
            let node_peer = s.runner.add_peer(b"seed", now);
            s.runner
                .run_swarm(node_peer, node_swarm, 1, 2, now)
                .unwrap();
            Remote {
                runner,
                swarm: ss,
                peer,
                writer,
                node_peer,
                node_swarm,
                now,
                written: 0,
            }
        }

        /// Writes `munros` 64 KiB munros.
        fn write(&mut self, munros: u64) {
            for _ in 0..munros {
                let data: Vec<u8> = (self.written..self.written + 65536)
                    .map(stream_byte)
                    .collect();
                self.runner
                    .write(self.swarm, &mut self.writer, self.now, &data)
                    .unwrap();
                self.written += 65536;
            }
        }

        /// Moves frames both ways until both are quiet, then ticks `ms`.
        fn tick(&mut self, s: &mut State, ms: i64) {
            let mut buf = vec![0u8; 16 * 1024];
            loop {
                let mut moved = false;
                while self.runner.wants_transmit(self.peer) {
                    let n = self.runner.poll_transmit(self.peer, self.now, &mut buf);
                    if n == 0 {
                        break;
                    }
                    moved = true;
                    s.runner
                        .handle_frame(self.node_peer, self.now, &buf[..n])
                        .unwrap();
                }
                while s.runner.wants_transmit(self.node_peer) {
                    let n = s.runner.poll_transmit(self.node_peer, self.now, &mut buf);
                    if n == 0 {
                        break;
                    }
                    moved = true;
                    self.runner
                        .handle_frame(self.peer, self.now, &buf[..n])
                        .unwrap();
                }
                s.test_process_events(self.now);
                while self.runner.poll_event().is_some() {}
                if !moved {
                    break;
                }
            }
            self.now += Duration::from_millis(ms);
            self.runner.handle_timeout(self.now);
            s.runner.handle_timeout(self.now);
            s.test_process_events(self.now);
        }
    }

    struct TestReader {
        id: ReaderId,
        reads: mpsc::Receiver<crate::state::ReadReply>,
        rewinds: mpsc::Receiver<crate::state::RewindReply>,
    }

    fn open(s: &mut State, swarm: strims_ppspp::SwarmKey) -> TestReader {
        let (tx, reads) = mpsc::channel(1);
        let (rtx, rewinds) = mpsc::channel(1);
        TestReader {
            id: s.open_reader(swarm, tx, rtx).unwrap(),
            reads,
            rewinds,
        }
    }

    /// Reads what is there: the offset of the first byte and the bytes. The
    /// last read waits in the node for data; a wake answers it, and the
    /// next call takes that answer first.
    fn read_all(s: &mut State, r: &mut TestReader, now: Time) -> (Option<u64>, Vec<u8>) {
        let (mut first, mut out) = (None, vec![]);
        loop {
            let reply = match r.reads.try_recv() {
                Ok(reply) => reply,
                Err(_) => {
                    s.read(r.id, vec![0; 64 * 1024], 64 * 1024, now);
                    match r.reads.try_recv() {
                        Ok(reply) => reply,
                        Err(_) => return (first, out),
                    }
                }
            };
            let (off, n) = reply.result.unwrap();
            assert!(first.is_none() || off == first.unwrap() + out.len() as u64);
            first.get_or_insert(off);
            out.extend_from_slice(&reply.buf[..n]);
        }
    }

    /// A reader moves back two munros behind where it joined, waiting for
    /// the chunks from the seed, and reads on from there; the other reader
    /// stays. A refusal and a cancel answer at once; a removed swarm
    /// closes a waiting rewind.
    #[test]
    fn a_reader_rewinds_alone_and_waits_for_the_chunks() {
        let mut a = browser(1);
        let s = &mut a.task.state;
        let mut seed = Remote::new(s);
        let swarm = seed.node_swarm;
        let mut r1 = open(s, swarm);
        let mut r2 = open(s, swarm);
        seed.write(6);
        for _ in 0..4 {
            seed.tick(s, 50);
        }
        let (start, got) = read_all(s, &mut r1, seed.now);
        let start = start.unwrap();
        let pos = start + got.len() as u64;
        assert!(start >= 4 * 65536, "{start}");
        let (start2, got2) = read_all(s, &mut r2, seed.now);
        let pos2 = start2.unwrap() + got2.len() as u64;

        s.rewind(r1.id, pos + 1, seed.now);
        assert!(matches!(
            r1.rewinds.try_recv().unwrap().result,
            Err(ReadError::Refused(strims_ppspp::RewindError::Ahead))
        ));
        let k = start - 2 * 65536 + 300;
        s.rewind(r1.id, k, seed.now);
        assert!(r1.rewinds.try_recv().is_err(), "waits for the chunks");
        for _ in 0..40 {
            seed.tick(s, 50);
            if let Ok(r) = r1.rewinds.try_recv() {
                assert_eq!(r.result.unwrap(), k - 300);
                break;
            }
        }
        seed.write(1);
        for _ in 0..4 {
            seed.tick(s, 50);
        }
        let (first, got) = read_all(s, &mut r1, seed.now);
        assert_eq!(first, Some(k - 300));
        assert_eq!(k - 300 + got.len() as u64, seed.written);
        assert!(
            got.iter()
                .enumerate()
                .all(|(i, &b)| b == stream_byte(k - 300 + i as u64))
        );
        let (first2, _) = read_all(s, &mut r2, seed.now);
        assert_eq!(first2, Some(pos2), "the other reader stays");

        // a cancelled rewind is answered, and its backlog goes
        let k = k - 300 - 65536;
        s.rewind(r1.id, k, seed.now);
        assert!(r1.rewinds.try_recv().is_err());
        assert!(s.runner.swarm(swarm).unwrap().store().backlog().is_some());
        s.cancel_rewind(r1.id, seed.now);
        assert!(r1.rewinds.try_recv().unwrap().result.is_err());
        assert!(s.runner.swarm(swarm).unwrap().store().backlog().is_none());

        // a removed swarm closes a waiting one
        s.rewind(r1.id, k, seed.now);
        assert!(r1.rewinds.try_recv().is_err());
        s.remove_swarm(swarm).unwrap();
        assert!(matches!(
            r1.rewinds.try_recv().unwrap().result,
            Err(ReadError::Closed)
        ));
    }
}
