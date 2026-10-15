// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The node task: one tokio task that owns the composed state machines
//! ([`State`]) and runs the event loop.
//!
//! ```text
//! loop {
//!     wait (biased): transport input | command | timer at State::poll_timeout
//!     drain a bounded batch of ready inputs and commands
//!     State::handle_timeouts(now)
//!     State::pump(now)     // events, vpn.flush, vnic.poll_egress, link writes
//!     start queued dials
//! }
//! ```
//!
//! Transport inputs include WebRTC datagrams from the UDP socket tasks
//! (`udp.rs`); the str0m endpoint itself is part of [`State`].
//!
//! **Queries** (`Node::metrics`, `Node::peers`) wait in a mailbox that the
//! loop drains after every pass, before it waits again, so they are
//! answered in a poll the node task makes anyway. Every poll of a task
//! shifts how a current-thread runtime orders the tasks after it, and under
//! a backlogged upload cap a different order is a different schedule, so a
//! poll made only to answer a query would let sampling change the run. The
//! loop publishes whether its next pass is due within
//! `NodeConfig::query_delay`. When it isn't (an idle node), the caller
//! sends `Command::Wake`, and the node answers without running timers,
//! egress or dials.
//!
//! The loop's `select!` is `biased` and it spawns tasks in a fixed order;
//! see [Determinism](crate#determinism) for the rest of what keeps runs
//! reproducible.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering, fence};

use futures_util::StreamExt;
use strims_base::time::{Duration, Time};
use strims_vnic::tcp::format_tcp_uri;
use strims_vnic::ws::{WsListenAddress, format_ws_uri};
use strims_vnic::{LinkConfig, LinkDescription, LinkInterface, PeerKey as LinkKey};
use tokio::sync::{mpsc, oneshot};
use tokio::task::AbortHandle;

use crate::clock::Clock;
use crate::config::NodeConfig;
use crate::error::NodeError;
use crate::event::NodeEvent;
use crate::handle::NodeInfo;
use crate::link::{self, Input, Transport};
use crate::net::Net;
use crate::state::{DialId, LinkIo, PublisherId, ReaderId, RtcSetup, State, UdpIo, WriteOp};
use crate::transport::{self, IoConfig};
use crate::udp;

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
    CloseReader(ReaderId),
    ClosePublisher(PublisherId),
    Shutdown(oneshot::Sender<()>),
}

pub(crate) struct Driver<N: Net, C: Clock> {
    state: State,
    net: N,
    clock: C,
    io: IoConfig,
    input_tx: mpsc::Sender<Input<N>>,
    input_rx: mpsc::Receiver<Input<N>>,
    cmd_rx: mpsc::Receiver<Command>,
    /// Read-only calls, answered after passes (see the module docs).
    query_rx: mpsc::Receiver<Call>,
    /// Whether the next pass is due within `query_delay`.
    answers_soon: Arc<AtomicBool>,
    query_delay: Duration,
    tcp_listener: Option<N::TcpListener>,
    ws_listener: Option<N::TcpListener>,
    /// The WebRTC sockets' tasks, spawned when the loop starts.
    udp_tasks: Vec<UdpTasks<N>>,
    /// STUN servers to resolve, and how often.
    stun: Option<(Vec<strims_webrtc::stun::StunServer>, std::time::Duration)>,
    /// The accept loops, UDP readers and the STUN resolver, aborted at
    /// shutdown. (UDP writers end when the state drops their queues, after
    /// sending the last datagrams.)
    tasks: Vec<AbortHandle>,
    shutdown: Option<oneshot::Sender<()>>,
}

/// One UDP socket's task ends, until the loop spawns them.
struct UdpTasks<N: Net> {
    socket: usize,
    sock: Arc<N::UdpSocket>,
    free_rx: mpsc::Receiver<Vec<u8>>,
    send_rx: mpsc::Receiver<(SocketAddr, Vec<u8>)>,
}

/// Binds the WebRTC interface's UDP sockets, one per listen address.
async fn bind_webrtc<N: Net>(
    net: &N,
    config: &NodeConfig,
) -> Result<(RtcSetup, Vec<UdpTasks<N>>), NodeError> {
    let w = &config.webrtc;
    if w.listen.is_empty() {
        return Err(NodeError::Config(
            "webrtc is enabled without a listen address".to_owned(),
        ));
    }
    let mut endpoint = w.endpoint.clone();
    endpoint.host_ip = endpoint.host_ip.or(config.public_ip);
    let mut sockets = Vec::new();
    let mut tasks = Vec::new();
    for (socket, a) in w.listen.iter().enumerate() {
        let sock = net.udp_bind(a.clone()).await?;
        let addr = net.udp_local_addr(&sock)?;
        // str0m matches a datagram's destination against the host
        // candidates, which must be addresses peers can reach.
        if addr.ip().is_unspecified()
            && !endpoint
                .host_ip
                .is_some_and(|ip| ip.is_ipv4() == addr.is_ipv4())
        {
            return Err(NodeError::Config(format!(
                "webrtc listen address {addr} is unspecified: bind a concrete ip, or set \
                 webrtc.endpoint.host_ip or public_ip (same address family)"
            )));
        }
        let (send_tx, send_rx) = mpsc::channel(w.udp_send_queue.max(1));
        let n = w.udp_read_buffers.max(1);
        let (read_free_tx, free_rx) = mpsc::channel(n);
        for _ in 0..n {
            let _ = read_free_tx.try_send(vec![0; w.udp_read_buffer_size.max(1)]);
        }
        sockets.push(UdpIo {
            addr,
            send_tx,
            read_free_tx,
        });
        tasks.push(UdpTasks {
            socket,
            sock: Arc::new(sock),
            free_rx,
            send_rx,
        });
    }
    Ok((
        RtcSetup {
            config: endpoint,
            sockets,
            max_buffered: w.max_buffered_amount.max(1),
        },
        tasks,
    ))
}

/// What `Driver::new` hands to the API handle.
pub(crate) struct Started<N: Net, C: Clock> {
    pub(crate) driver: Driver<N, C>,
    pub(crate) info: Arc<NodeInfo>,
    pub(crate) commands: mpsc::Sender<Command>,
    pub(crate) queries: mpsc::Sender<Call>,
    pub(crate) answers_soon: Arc<AtomicBool>,
    pub(crate) events: mpsc::Receiver<NodeEvent>,
}

impl<N: Net, C: Clock> Driver<N, C> {
    /// Builds the state machines and binds the listeners.
    pub(crate) async fn start(
        config: NodeConfig,
        net: N,
        clock: C,
    ) -> Result<Started<N, C>, NodeError> {
        let profile_key: [u8; 32] = config
            .profile_key
            .public
            .as_slice()
            .try_into()
            .map_err(|_| NodeError::Config("the profile key must be ed25519".to_owned()))?;
        let tcp_listener = match &config.tcp_listen {
            Some(a) => Some(net.tcp_bind(a.clone()).await?),
            None => None,
        };
        let ws_listener = match &config.ws_listen {
            Some(a) => Some(net.tcp_bind(a.clone()).await?),
            None => None,
        };
        let tcp_addr = tcp_listener
            .as_ref()
            .map(|l| net.tcp_listener_addr(l))
            .transpose()?;
        let ws_addr = ws_listener
            .as_ref()
            .map(|l| net.tcp_listener_addr(l))
            .transpose()?;

        // A URI that can't be formatted (an unspecified listen address
        // without a public address) isn't advertised.
        let host_ip = config
            .public_ip
            .map(|ip| ip.to_string())
            .unwrap_or_default();
        let tcp_uri =
            tcp_addr.and_then(|a| format_tcp_uri(&a.to_string(), &host_ip, &profile_key).ok());
        let ws_uri = ws_addr.and_then(|a| {
            let public_hostname = if config.ws_public_hostname.is_empty() {
                host_ip.clone()
            } else {
                config.ws_public_hostname.clone()
            };
            let addr = WsListenAddress {
                address: a.to_string(),
                public_hostname,
                public_port: config.ws_public_port,
                // TODO: a config flag to advertise `wss://` when TLS
                // terminates at a reverse proxy.
                secure: false,
            };
            format_ws_uri(&addr, &profile_key).ok()
        });
        let describe = |iface: LinkInterface, uri: &Option<String>| {
            uri.as_ref().map(|u| LinkDescription {
                interface: iface.name().to_owned(),
                description: u.clone(),
            })
        };

        let (rtc, udp_tasks) = if config.webrtc.enabled {
            let (setup, tasks) = bind_webrtc(&net, &config).await?;
            (Some(setup), tasks)
        } else {
            (None, Vec::new())
        };
        let webrtc_addrs = rtc
            .as_ref()
            .map(|r| r.sockets.iter().map(|s| s.addr).collect())
            .unwrap_or_default();
        let stun = rtc.as_ref().and_then(|r| {
            let servers = r.config.stun_servers();
            let refresh = config.webrtc.stun_refresh.to_std().unwrap_or_default();
            (!servers.is_empty()).then_some((servers, refresh))
        });

        let (events_tx, events) = mpsc::channel(config.event_queue.max(1));
        let state = State::new(
            &config,
            describe(LinkInterface::Tcp, &tcp_uri),
            describe(LinkInterface::Ws, &ws_uri),
            rtc,
            clock.now(),
            events_tx,
        )?;
        let info = Arc::new(NodeInfo {
            host_id: state.vnic.id(),
            profile_key: config.profile_key.public.clone(),
            tcp_addr,
            ws_addr,
            tcp_uri,
            ws_uri,
            webrtc_addrs,
        });
        let (commands, cmd_rx) = mpsc::channel(config.command_queue.max(1));
        let (queries, query_rx) = mpsc::channel(config.command_queue.max(1));
        let answers_soon = Arc::new(AtomicBool::new(false));
        let (input_tx, input_rx) = mpsc::channel(config.input_queue.max(1));
        let io = IoConfig {
            tcp: config.tcp,
            tcp_socket: config.tcp_socket.clone(),
            ws: config.ws,
            buffers: config.buffers,
            dial_timeout: config.dial_timeout,
            profile_key,
        };
        Ok(Started {
            driver: Driver {
                state,
                net,
                clock,
                io,
                input_tx,
                input_rx,
                cmd_rx,
                query_rx,
                answers_soon: answers_soon.clone(),
                query_delay: config.query_delay,
                tcp_listener,
                ws_listener,
                udp_tasks,
                stun,
                tasks: Vec::new(),
                shutdown: None,
            },
            info,
            commands,
            queries,
            answers_soon,
            events,
        })
    }

    /// The event loop. Ends on `Shutdown` or when every handle is dropped.
    pub(crate) async fn run(mut self) {
        self.start_listeners();
        let sleep = tokio::time::sleep_until(tokio::time::Instant::now());
        tokio::pin!(sleep);
        // The deadline the timer is armed for. A wakeup that moves no
        // deadline (a query) leaves the timer untouched.
        let mut armed: Option<Time> = None;
        let mut again = false;
        let mut last = self.clock.now();
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
                tokio::task::yield_now().await;
            } else {
                if let Some(t) = deadline
                    && armed != Some(t)
                {
                    sleep.as_mut().reset(self.clock.deadline(t));
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
                    () = &mut sleep, if deadline.is_some() => {}
                }
            }
            for _ in 0..BATCH {
                match self.input_rx.try_recv() {
                    Ok(input) => self.handle_input(input),
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
            let now = self.clock.now();
            self.state.handle_timeouts(now);
            again = self.state.pump(now);
            self.start_dials();
            self.state.metrics.loop_iterations += 1;
            last = now;
        }
        self.stop();
    }

    fn stop(&mut self) {
        for t in self.tasks.drain(..) {
            t.abort();
        }
        let now = self.clock.now();
        // Closing drops every link's I/O: writers shut their sockets down.
        self.state.close(now);
        if let Some(done) = self.shutdown.take() {
            let _ = done.send(());
        }
    }

    fn start_listeners(&mut self) {
        if let Some(l) = self.tcp_listener.take() {
            let h = tokio::spawn(transport::tcp_accept_loop(
                self.net.clone(),
                l,
                self.io.clone(),
                self.input_tx.clone(),
            ));
            self.tasks.push(h.abort_handle());
        }
        if let Some(l) = self.ws_listener.take() {
            let h = tokio::spawn(transport::ws_accept_loop(
                self.net.clone(),
                l,
                self.io.clone(),
                self.input_tx.clone(),
            ));
            self.tasks.push(h.abort_handle());
        }
        for t in std::mem::take(&mut self.udp_tasks) {
            let h = tokio::spawn(udp::reader(
                self.net.clone(),
                t.socket,
                t.sock.clone(),
                t.free_rx,
                self.input_tx.clone(),
            ));
            self.tasks.push(h.abort_handle());
            tokio::spawn(udp::writer(self.net.clone(), t.sock, t.send_rx));
        }
        if let Some((servers, refresh)) = self.stun.take() {
            let h = tokio::spawn(udp::stun_resolver(
                self.net.clone(),
                servers,
                refresh,
                self.input_tx.clone(),
            ));
            self.tasks.push(h.abort_handle());
        }
    }

    fn start_dials(&mut self) {
        while let Some((id, target)) = self.state.next_dial() {
            tokio::spawn(transport::dial(
                self.net.clone(),
                id,
                target,
                self.io.clone(),
                self.input_tx.clone(),
            ));
        }
    }

    /// Publishes whether the next pass is due within `query_delay`, then
    /// answers the queued queries. In that order: a caller queues first and
    /// reads the flag second, so a query this drain misses sees the new
    /// flag and wakes the node if it must.
    fn answer_queries(&mut self, again: bool, deadline: Option<Time>) {
        let now = self.clock.now();
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
        let now = self.clock.now();
        match c {
            // `answer_queries` answers them at the top of the loop.
            Command::Wake => self.state.metrics.query_wakes += 1,
            Command::Call(f) => f(&mut self.state, now),
            Command::Read { reader, buf, max } => self.state.read(reader, buf, max, now),
            Command::Write { publisher, op, buf } => self.state.write(publisher, op, buf, now),
            Command::CloseReader(r) => self.state.close_reader(r),
            Command::ClosePublisher(p) => self.state.close_publisher(p),
            Command::Shutdown(done) => {
                self.shutdown = Some(done);
                return true;
            }
        }
        false
    }

    fn handle_input(&mut self, input: Input<N>) {
        let now = self.clock.now();
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
            } => self.attach(dial, transport, config, notsent_lowat, &mut leftover, now),
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
        }
    }

    /// Adds a connected transport to vnic and starts its I/O tasks.
    ///
    /// Bytes in the link's write buffers and in vnic's link buffer are past
    /// QoS. A socket with a bound on its unsent bytes therefore gets writes
    /// of that size, and vnic keeps at most one MTU plus one write for it.
    fn attach(
        &mut self,
        dial: Option<DialId>,
        transport: Transport<N>,
        mut config: LinkConfig,
        notsent_lowat: Option<u32>,
        leftover: &mut [u8],
        now: Time,
    ) {
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
                return;
            }
        };
        let io = self.spawn_link_io(link, transport, max_write);
        self.state.insert_link(link, kind, io, dial);
        if !leftover.is_empty() {
            self.state.handle_read(link, now, leftover);
        }
    }

    /// Spawns the link's reader, then its writer, with their buffers.
    fn spawn_link_io(&self, link: LinkKey, transport: Transport<N>, max_write: usize) -> LinkIo {
        let b = self.io.buffers;
        let (nr, nw) = (b.read_buffers.max(1), b.write_buffers.max(1));
        let (write_tx, write_rx) = mpsc::channel(nw);
        let (read_free_tx, read_free_rx) = mpsc::channel(nr);
        for _ in 0..nr {
            let _ = read_free_tx.try_send(vec![0; b.read_buffer_size.max(1)]);
        }
        let free_write = (0..nw).map(|_| Vec::with_capacity(max_write)).collect();
        let input = self.input_tx.clone();
        let mut idle_ok = None;
        let reader = match transport {
            Transport::Tcp(stream) => {
                let (r, w) = tokio::io::split(stream);
                let flag = link::IdleOk::default();
                idle_ok = Some(flag.clone());
                let reader = tokio::spawn(link::stream_reader(
                    link,
                    r,
                    read_free_rx,
                    input.clone(),
                    self.io.tcp.read_timeout,
                    flag,
                ));
                tokio::spawn(link::stream_writer(
                    link,
                    w,
                    write_rx,
                    input,
                    self.io.tcp.write_timeout,
                ));
                reader
            }
            Transport::Ws(ws) => {
                let (sink, stream) = (*ws).split();
                let reader = tokio::spawn(link::ws_reader(
                    link,
                    stream,
                    read_free_rx,
                    input.clone(),
                    self.io.ws.read_timeout,
                ));
                tokio::spawn(link::ws_writer(
                    link,
                    sink,
                    write_rx,
                    input,
                    self.io.ws.write_timeout,
                    self.io.ws.ping_interval,
                ));
                reader
            }
        };
        LinkIo {
            write_tx,
            free_write,
            max_write,
            read_free_tx,
            reader: Some(reader.abort_handle()),
            idle_ok,
        }
    }
}
