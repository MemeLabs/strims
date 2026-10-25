// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The native driver: [`NativeIo`] runs a core node on tokio, over a
//! [`Net`]'s sockets and a [`Clock`].
//!
//! [`spawn`] binds the listeners and the WebRTC sockets, builds the node
//! with `strims_node_core::start` and spawns its task on the current
//! runtime. The node task then has the driver spawn, in order: the TCP and
//! WebSocket accept loops, each UDP socket's reader and writer, and the
//! STUN resolver at start; a link's reader then its writer when the link is
//! attached; the queued dials after each pass.

use std::future::Future;
use std::pin::Pin;

use futures_util::StreamExt;
use strims_base::time::Time;
use strims_node_core::{
    DialId, IdleOk, Inputs, Interfaces, LinkChannels, LinkKey, LinkTasks, Node, NodeConfig,
    NodeError, NodeEvents, NodeIo, NodeSetup, RtcFactory, SpawnNode, TaskGuard,
};
use strims_vnic::DialTarget;
use strims_vnic::tcp::format_tcp_uri;
use strims_vnic::ws::{WsListenAddress, format_ws_uri};
use strims_webrtc::stun::StunServer;
use tokio::task::AbortHandle;
use tokio::time::Sleep;

use crate::clock::{Clock, SimClock, SystemClock};
use crate::link::{self, Transport};
use crate::net::Net;
use crate::rtc::{NativeRtc, UdpTasks, bind_webrtc};
use crate::transport::{self, IoConfig};
use crate::udp;

/// The link interfaces native nodes have: TCP, and `wss://` dials fail in
/// the transport (no TLS client yet).
const NATIVE_INTERFACES: Interfaces = Interfaces {
    tcp: true,
    wss: false,
};

/// Builds the node, binds its listeners and spawns the node task on the
/// current runtime (a current-thread runtime for determinism; turmoil
/// hosts are one).
pub(crate) async fn spawn<N: Net, C: Clock>(
    config: NodeConfig,
    net: N,
    clock: C,
) -> Result<(Node, NodeEvents), NodeError> {
    let (setup, io) = NativeIo::bind(&config, net, clock).await?;
    let (node, events, task) = strims_node_core::start(&config, setup, io)?;
    tokio::spawn(task.run());
    Ok((node, events))
}

impl<N: Net> SpawnNode<N> for SystemClock {
    fn spawn_node(
        self,
        config: NodeConfig,
        net: N,
    ) -> impl Future<Output = Result<(Node, NodeEvents), NodeError>> + Send {
        spawn(config, net, self)
    }
}

impl<N: Net> SpawnNode<N> for SimClock {
    fn spawn_node(
        self,
        config: NodeConfig,
        net: N,
    ) -> impl Future<Output = Result<(Node, NodeEvents), NodeError>> + Send {
        spawn(config, net, self)
    }
}

/// The native driver's sockets and tasks.
pub(crate) struct NativeIo<N: Net, C: Clock> {
    net: N,
    clock: C,
    io: IoConfig,
    tcp_listener: Option<N::TcpListener>,
    ws_listener: Option<N::TcpListener>,
    /// The WebRTC sockets' tasks, spawned when the loop starts.
    udp_tasks: Vec<UdpTasks<N>>,
    /// STUN servers to resolve, and how often.
    stun: Option<(Vec<StunServer>, std::time::Duration)>,
    /// The accept loops, UDP readers and the STUN resolver, aborted at
    /// shutdown. (UDP writers end when the state drops their queues, after
    /// sending the last datagrams.)
    tasks: Vec<AbortHandle>,
}

impl<N: Net, C: Clock> NativeIo<N, C> {
    /// Binds the listeners and the WebRTC sockets.
    async fn bind(
        config: &NodeConfig,
        net: N,
        clock: C,
    ) -> Result<(NodeSetup, NativeIo<N, C>), NodeError> {
        let profile_key = config.profile_public_key()?;
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

        let (rtc, udp_tasks) = if config.webrtc.enabled {
            let (setup, tasks) = bind_webrtc(&net, config).await?;
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
        let rtc = rtc.map(|setup| -> RtcFactory {
            Box::new(move |now, rng| Box::new(NativeRtc::new(setup, now, rng)))
        });
        let io = IoConfig {
            tcp: config.tcp,
            tcp_socket: config.tcp_socket.clone(),
            ws: config.ws,
            dial_timeout: config.dial_timeout,
            profile_key,
        };
        Ok((
            NodeSetup {
                interfaces: NATIVE_INTERFACES,
                tcp_addr,
                ws_addr,
                tcp_uri,
                ws_uri,
                webrtc_addrs,
                rtc,
            },
            NativeIo {
                net,
                clock,
                io,
                tcp_listener,
                ws_listener,
                udp_tasks,
                stun,
                tasks: Vec::new(),
            },
        ))
    }
}

fn abort_guard(h: AbortHandle) -> TaskGuard {
    TaskGuard::new(move || h.abort())
}

impl<N: Net, C: Clock> NodeIo for NativeIo<N, C> {
    type Transport = Transport<N>;
    type Sleep = Sleep;

    fn now(&self) -> Time {
        self.clock.now()
    }

    fn new_sleep(&self) -> Sleep {
        tokio::time::sleep_until(tokio::time::Instant::now())
    }

    fn reset_sleep(&self, sleep: Pin<&mut Sleep>, t: Time) {
        sleep.reset(self.clock.deadline(t));
    }

    fn yield_now(&self) -> impl Future<Output = ()> {
        tokio::task::yield_now()
    }

    /// The accept loops, then each UDP socket's reader and writer, then
    /// the STUN resolver.
    fn start(&mut self, inputs: &Inputs<Transport<N>>) {
        if let Some(l) = self.tcp_listener.take() {
            let h = tokio::spawn(transport::tcp_accept_loop(
                self.net.clone(),
                l,
                self.io.clone(),
                inputs.clone(),
            ));
            self.tasks.push(h.abort_handle());
        }
        if let Some(l) = self.ws_listener.take() {
            let h = tokio::spawn(transport::ws_accept_loop(
                self.net.clone(),
                l,
                self.io.clone(),
                inputs.clone(),
            ));
            self.tasks.push(h.abort_handle());
        }
        for t in std::mem::take(&mut self.udp_tasks) {
            let h = tokio::spawn(udp::reader(
                self.net.clone(),
                t.socket,
                t.sock.clone(),
                t.free_rx,
                inputs.clone(),
            ));
            self.tasks.push(h.abort_handle());
            tokio::spawn(udp::writer(self.net.clone(), t.sock, t.send_rx));
        }
        if let Some((servers, refresh)) = self.stun.take() {
            let h = tokio::spawn(udp::stun_resolver(
                self.net.clone(),
                servers,
                refresh,
                inputs.clone(),
            ));
            self.tasks.push(h.abort_handle());
        }
    }

    fn dial(&mut self, dial: DialId, target: DialTarget, inputs: &Inputs<Transport<N>>) {
        tokio::spawn(transport::dial(
            self.net.clone(),
            dial,
            target,
            self.io.clone(),
            inputs.clone(),
        ));
    }

    fn spawn_link(
        &mut self,
        link: LinkKey,
        transport: Transport<N>,
        channels: LinkChannels<Transport<N>>,
    ) -> LinkTasks {
        let LinkChannels {
            read_free,
            writes,
            inputs,
            probe,
        } = channels;
        let mut idle_ok = None;
        let mut link_probe = None;
        let reader = match transport {
            Transport::Tcp(stream) => {
                if probe {
                    link_probe = self.net.link_probe(&stream, false);
                }
                let (r, w) = tokio::io::split(stream);
                let flag = IdleOk::default();
                idle_ok = Some(flag.clone());
                let reader = tokio::spawn(link::stream_reader(
                    link,
                    r,
                    read_free,
                    inputs.clone(),
                    self.io.tcp.read_timeout,
                    flag,
                ));
                tokio::spawn(link::stream_writer(
                    link,
                    w,
                    writes,
                    inputs,
                    self.io.tcp.write_timeout,
                ));
                reader
            }
            Transport::Ws(ws) => {
                if probe {
                    link_probe = self.net.link_probe(ws.get_ref(), true);
                }
                let (sink, stream) = (*ws).split();
                let reader = tokio::spawn(link::ws_reader(
                    link,
                    stream,
                    read_free,
                    inputs.clone(),
                    self.io.ws.read_timeout,
                ));
                tokio::spawn(link::ws_writer(
                    link,
                    sink,
                    writes,
                    inputs,
                    self.io.ws.write_timeout,
                    self.io.ws.ping_interval,
                ));
                reader
            }
        };
        LinkTasks {
            reader: abort_guard(reader.abort_handle()),
            idle_ok,
            probe: link_probe,
        }
    }

    fn stop(&mut self) {
        for t in self.tasks.drain(..) {
            t.abort();
        }
    }
}
