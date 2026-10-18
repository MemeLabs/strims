// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Dialing and accepting TCP and WebSocket transports, up to the point
//! where vnic takes over.
//!
//! - TCP dial: connect, apply the socket options, and hand the stream to
//!   vnic with `LinkConfig::tcp_dial`, which writes the `TCPMuxInit` and runs
//!   the AES link handshake.
//! - TCP accept: read the `TCPMuxInit` with a `TcpMuxAcceptor`, route it with
//!   a `TcpMux` holding only this node's profile key (any other key closes
//!   the connection), and hand the stream plus the bytes read past the init
//!   to vnic with `LinkConfig::tcp_accept`.
//! - WebSocket: a listener of its own upgrades at `/<hex profile key>` and
//!   answers 404 elsewhere; then `LinkConfig::ws`. Only `ws://`: dialing
//!   `wss://` fails.
//!
//! `NodeConfig::dial_timeout` bounds a dial's connect plus upgrade, and an
//! accepted connection's wait for its `TCPMuxInit` or upgrade. Every
//! dialed and accepted socket, WebSocket ones included, gets
//! [`Net::configure_tcp`] and the send-queue options.

use std::net::SocketAddr;

use strims_node_core::DialId;
use strims_vnic::ws::ws_path;
use strims_vnic::{DialTarget, LinkConfig, TcpMux, TcpMuxAcceptor, TcpOptions};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::{StatusCode, Uri};

use crate::config::TcpSocketConfig;
use crate::link::{ACCEPT_BACKOFF, Input, Transport, with_timeout, ws_config};
use crate::net::Net;

/// What the transport tasks need from the config.
#[derive(Clone, Debug)]
pub(crate) struct IoConfig {
    pub(crate) tcp: TcpOptions,
    pub(crate) tcp_socket: TcpSocketConfig,
    pub(crate) ws: strims_vnic::ws::WsOptions,
    pub(crate) dial_timeout: strims_base::time::Duration,
    /// This node's profile public key: the TCP mux route and WS path.
    pub(crate) profile_key: [u8; 32],
}

/// Connects a dial's transport and reports it to the node task.
pub(crate) async fn dial<N: Net>(
    net: N,
    dial: DialId,
    target: DialTarget,
    io: IoConfig,
    input: mpsc::Sender<Input<N>>,
) {
    let res = with_timeout(io.dial_timeout, connect(&net, target, &io))
        .await
        .unwrap_or_else(|| Err("dial timeout".to_owned()));
    let msg = match res {
        Ok((transport, config, notsent_lowat)) => Input::Connected {
            dial: Some(dial),
            transport,
            config,
            leftover: Vec::new(),
            notsent_lowat,
            rtc: None,
        },
        Err(error) => Input::DialFailed { dial, error },
    };
    let _ = input.send(msg).await;
}

async fn connect<N: Net>(
    net: &N,
    target: DialTarget,
    io: &IoConfig,
) -> Result<(Transport<N>, LinkConfig, Option<u32>), String> {
    match target {
        DialTarget::Tcp(uri) => {
            let s = net
                .tcp_connect(uri.host.clone())
                .await
                .map_err(|e| e.to_string())?;
            net.configure_tcp(&s, &io.tcp).map_err(|e| e.to_string())?;
            let lowat = send_queue(net, &s, io);
            Ok((
                Transport::Tcp(s),
                LinkConfig::tcp_dial(&uri.peer_key),
                lowat,
            ))
        }
        DialTarget::Ws(t) => {
            if t.secure {
                // TODO: wss:// via tokio-rustls behind a `tls` feature.
                return Err("wss:// is not supported yet (no TLS)".to_owned());
            }
            let addr = ws_authority(&t.url)?;
            let s = net.tcp_connect(addr).await.map_err(|e| e.to_string())?;
            // WebSocket links carry vnic's small writes too: TCP_NODELAY.
            net.configure_tcp(&s, &io.tcp).map_err(|e| e.to_string())?;
            let lowat = send_queue(net, &s, io);
            let (ws, _) =
                tokio_tungstenite::client_async_with_config(t.url.as_str(), s, Some(ws_config()))
                    .await
                    .map_err(|e| e.to_string())?;
            Ok((Transport::Ws(Box::new(ws)), LinkConfig::ws(), lowat))
        }
    }
}

/// Applies the send-queue options. They are best effort: a socket that
/// refuses them still carries the link, with the kernel's default queue.
fn send_queue<N: Net>(net: &N, stream: &N::TcpStream, io: &IoConfig) -> Option<u32> {
    net.configure_tcp_send_queue(stream, &io.tcp_socket)
        .unwrap_or_else(|e| {
            tracing::debug!(error = %e, "tcp send queue options");
            None
        })
}

/// `host:port` of a `ws://` URL (port 80 when absent).
fn ws_authority(url: &str) -> Result<String, String> {
    let uri: Uri = url.parse().map_err(|e| format!("invalid url {url}: {e}"))?;
    let a = uri
        .authority()
        .ok_or_else(|| format!("no host in {url}"))?
        .as_str();
    let hostport = a.rsplit_once('@').map_or(a, |(_, h)| h);
    Ok(match uri.port_u16() {
        Some(_) => hostport.to_owned(),
        None => format!("{hostport}:80"),
    })
}

/// Accepts TCP connections until aborted.
pub(crate) async fn tcp_accept_loop<N: Net>(
    net: N,
    listener: N::TcpListener,
    io: IoConfig,
    input: mpsc::Sender<Input<N>>,
) {
    loop {
        match net.tcp_accept(&listener).await {
            Ok((stream, addr)) => {
                tokio::spawn(tcp_accept(
                    net.clone(),
                    stream,
                    addr,
                    io.clone(),
                    input.clone(),
                ));
            }
            Err(e) => {
                tracing::debug!(error = %e, "tcp accept");
                tokio::time::sleep(ACCEPT_BACKOFF).await;
            }
        }
    }
}

/// Reads an accepted connection's `TCPMuxInit` and hands the connection to
/// the node task, or drops it.
async fn tcp_accept<N: Net>(
    net: N,
    mut stream: N::TcpStream,
    addr: SocketAddr,
    io: IoConfig,
    input: mpsc::Sender<Input<N>>,
) {
    if let Err(e) = net.configure_tcp(&stream, &io.tcp) {
        tracing::debug!(error = %e, %addr, "tcp socket options");
    }
    let notsent_lowat = send_queue(&net, &stream, &io);
    let mut mux = TcpMux::new();
    mux.handle(io.profile_key, ());
    let res = with_timeout(io.dial_timeout, read_mux_init(&mut stream))
        .await
        .unwrap_or_else(|| Err("tcp mux init timeout".to_owned()))
        .and_then(|(key, leftover)| {
            mux.route(&key).map_err(|e| e.to_string())?;
            Ok(leftover)
        });
    let msg = match res {
        Ok(leftover) => Input::Connected {
            dial: None,
            transport: Transport::Tcp(stream),
            config: LinkConfig::tcp_accept(),
            leftover,
            notsent_lowat,
            rtc: None,
        },
        Err(error) => Input::AcceptFailed { error },
    };
    let _ = input.send(msg).await;
}

/// Reads the dialer's `TCPMuxInit`. Returns the profile key it names and
/// the bytes read past it (the start of the AES handshake).
async fn read_mux_init<S: AsyncRead + Unpin>(s: &mut S) -> Result<([u8; 32], Vec<u8>), String> {
    let mut acceptor = TcpMuxAcceptor::new();
    let mut buf = vec![0u8; 1024];
    loop {
        let n = s.read(&mut buf).await.map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("connection closed".to_owned());
        }
        let mut data = buf.get(..n).unwrap_or_default();
        while !data.is_empty() {
            let (c, key) = acceptor.handle_input(data).map_err(|e| e.to_string())?;
            data = data.get(c..).unwrap_or_default();
            if let Some(key) = key {
                return Ok((key, data.to_vec()));
            }
            if c == 0 {
                break;
            }
        }
    }
}

/// Accepts WebSocket upgrades at `/<hex profile key>` until aborted.
pub(crate) async fn ws_accept_loop<N: Net>(
    net: N,
    listener: N::TcpListener,
    io: IoConfig,
    input: mpsc::Sender<Input<N>>,
) {
    loop {
        match net.tcp_accept(&listener).await {
            Ok((stream, _)) => {
                // WebSocket links carry vnic's small writes too: TCP_NODELAY.
                if let Err(e) = net.configure_tcp(&stream, &io.tcp) {
                    tracing::debug!(error = %e, "ws socket options");
                }
                let lowat = send_queue(&net, &stream, &io);
                tokio::spawn(ws_accept(stream, io.clone(), lowat, input.clone()));
            }
            Err(e) => {
                tracing::debug!(error = %e, "ws accept");
                tokio::time::sleep(ACCEPT_BACKOFF).await;
            }
        }
    }
}

async fn ws_accept<N: Net>(
    stream: N::TcpStream,
    io: IoConfig,
    notsent_lowat: Option<u32>,
    input: mpsc::Sender<Input<N>>,
) {
    let path = ws_path(&io.profile_key);
    // tungstenite's `Callback` fixes the error type.
    #[allow(clippy::result_large_err)]
    let check = move |req: &Request, resp: Response| -> Result<Response, ErrorResponse> {
        if req.uri().path() == path {
            Ok(resp)
        } else {
            let mut r = ErrorResponse::new(Some("not found".to_owned()));
            *r.status_mut() = StatusCode::NOT_FOUND;
            Err(r)
        }
    };
    let res = with_timeout(
        io.dial_timeout,
        tokio_tungstenite::accept_hdr_async_with_config(stream, check, Some(ws_config())),
    )
    .await;
    let msg = match res {
        Some(Ok(ws)) => Input::Connected {
            dial: None,
            transport: Transport::Ws(Box::new(ws)),
            config: LinkConfig::ws(),
            leftover: Vec::new(),
            notsent_lowat,
            rtc: None,
        },
        Some(Err(e)) => Input::AcceptFailed {
            error: e.to_string(),
        },
        None => Input::AcceptFailed {
            error: "websocket upgrade timeout".to_owned(),
        },
    };
    let _ = input.send(msg).await;
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

    use super::*;

    #[test]
    fn ws_authorities() {
        assert_eq!(ws_authority("ws://a.b:81/x").unwrap(), "a.b:81");
        assert_eq!(ws_authority("ws://a.b/x").unwrap(), "a.b:80");
        assert_eq!(ws_authority("ws://u:p@[::1]:9/x").unwrap(), "[::1]:9");
        assert!(ws_authority("/x").is_err());
    }

    /// Dialed and accepted WebSocket sockets get the TCP options too,
    /// `TCP_NODELAY` above all, so vnic's small writes aren't held back by
    /// Nagle's algorithm.
    #[tokio::test]
    async fn ws_sockets_have_nodelay() {
        use crate::net::TokioNet;
        let key = [9u8; 32];
        let io = IoConfig {
            tcp: TcpOptions::default(),
            tcp_socket: TcpSocketConfig::default(),
            ws: strims_vnic::ws::WsOptions::default(),
            dial_timeout: strims_base::time::Duration::from_secs(5),
            profile_key: key,
        };
        let net = TokioNet;
        let listener = net.tcp_bind("127.0.0.1:0".to_owned()).await.unwrap();
        let addr = net.tcp_listener_addr(&listener).unwrap();
        let (tx, mut rx) = mpsc::channel::<Input<TokioNet>>(4);
        tokio::spawn(ws_accept_loop(net, listener, io.clone(), tx));

        let uri = format!("ws://{addr}{}", ws_path(&key));
        let target = DialTarget::parse(&uri, true).unwrap();
        let Ok((Transport::Ws(dialed), _, _)) = connect(&net, target, &io).await else {
            panic!("dial failed");
        };
        assert!(dialed.get_ref().nodelay().unwrap(), "dialed ws socket");
        let Some(Input::Connected {
            transport: Transport::Ws(accepted),
            ..
        }) = rx.recv().await
        else {
            panic!("accept failed");
        };
        assert!(accepted.get_ref().nodelay().unwrap(), "accepted ws socket");
    }

    /// The send-queue options reach every TCP socket a link runs on: dialed
    /// and accepted, TCP and WebSocket. An unavailable congestion
    /// controller leaves the socket usable.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn link_sockets_get_the_send_queue_options() {
        use tokio::io::AsyncWriteExt;

        use crate::net::TokioNet;

        fn controller(s: socket2::SockRef<'_>) -> Vec<u8> {
            let mut cc = s.tcp_congestion().unwrap();
            cc.truncate(cc.iter().position(|&b| b == 0).unwrap_or(cc.len()));
            cc
        }
        fn options(s: &tokio::net::TcpStream) -> (u32, Vec<u8>) {
            let r = socket2::SockRef::from(s);
            (r.tcp_notsent_lowat().unwrap(), controller(r))
        }
        fn cc(name: &str) -> Vec<u8> {
            name.as_bytes().to_vec()
        }

        let key = [9u8; 32];
        let net = TokioNet;
        for (congestion, expect) in [("reno", "reno"), ("no-such-cc", "")] {
            let io = IoConfig {
                tcp: TcpOptions::default(),
                tcp_socket: TcpSocketConfig {
                    notsent_lowat: Some(20_000),
                    congestion: Some(congestion.to_owned()),
                },
                ws: strims_vnic::ws::WsOptions::default(),
                dial_timeout: strims_base::time::Duration::from_secs(5),
                profile_key: key,
            };
            let tcp = net.tcp_bind("127.0.0.1:0".to_owned()).await.unwrap();
            let tcp_addr = net.tcp_listener_addr(&tcp).unwrap();
            let ws = net.tcp_bind("127.0.0.1:0".to_owned()).await.unwrap();
            let ws_addr = net.tcp_listener_addr(&ws).unwrap();
            let (tx, mut rx) = mpsc::channel::<Input<TokioNet>>(4);
            let tcp_task = tokio::spawn(tcp_accept_loop(net, tcp, io.clone(), tx.clone()));
            let ws_task = tokio::spawn(ws_accept_loop(net, ws, io.clone(), tx));

            let uri = format!("tcp://{tcp_addr}/{}", hex::encode(key));
            let target = DialTarget::parse(&uri, false).unwrap();
            let Ok((Transport::Tcp(mut dialed), _, lowat)) = connect(&net, target, &io).await
            else {
                panic!("tcp dial failed");
            };
            assert_eq!(lowat, Some(20_000));
            let default_cc = {
                let probe =
                    socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)
                        .unwrap();
                controller(socket2::SockRef::from(&probe))
            };
            let want = if expect.is_empty() {
                default_cc
            } else {
                cc(expect)
            };
            assert_eq!(options(&dialed), (20_000, want.clone()), "dialed tcp");
            let mut init = Vec::new();
            strims_vnic::tcp::encode_tcp_mux_init(&key, &mut init).unwrap();
            dialed.write_all(&init).await.unwrap();
            let Some(Input::Connected {
                transport: Transport::Tcp(accepted),
                notsent_lowat,
                ..
            }) = rx.recv().await
            else {
                panic!("tcp accept failed");
            };
            assert_eq!(notsent_lowat, Some(20_000));
            assert_eq!(options(&accepted), (20_000, want.clone()), "accepted tcp");

            let uri = format!("ws://{ws_addr}{}", ws_path(&key));
            let target = DialTarget::parse(&uri, true).unwrap();
            let Ok((Transport::Ws(dialed), _, lowat)) = connect(&net, target, &io).await else {
                panic!("ws dial failed");
            };
            assert_eq!(lowat, Some(20_000));
            assert_eq!(
                options(dialed.get_ref()),
                (20_000, want.clone()),
                "dialed ws"
            );
            let Some(Input::Connected {
                transport: Transport::Ws(accepted),
                notsent_lowat,
                ..
            }) = rx.recv().await
            else {
                panic!("ws accept failed");
            };
            assert_eq!(notsent_lowat, Some(20_000));
            assert_eq!(options(accepted.get_ref()), (20_000, want), "accepted ws");
            tcp_task.abort();
            ws_task.abort();
        }
    }

    #[tokio::test]
    async fn mux_init_leaves_the_rest() {
        let key = [7u8; 32];
        let mut wire = Vec::new();
        strims_vnic::tcp::encode_tcp_mux_init(&key, &mut wire).unwrap();
        wire.extend_from_slice(b"aes init");
        let mut r: &[u8] = &wire;
        let (k, rest) = read_mux_init(&mut r).await.unwrap();
        assert_eq!(k, key);
        assert_eq!(rest, b"aes init");
        let mut short: &[u8] = &wire[..3];
        assert!(read_mux_init(&mut short).await.is_err());
    }
}
