// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! WebRTC on a net that models data channels ([`Net::models_channels`]:
//! the sim's window model). The node's endpoint is a [`ProxyRtc`], as a
//! browser worker's is, and [`run`] plays the window: each connection is a
//! stream of the net, carried as a modelled channel ([`Net::open_channel`],
//! of `WebRtcConfig::max_buffered_amount`), and attached as the
//! connection's link (`Input::Connected` with `LinkConfig::webrtc`).
//!
//! - **Descriptions.** An offer is `strims-model <address> <token>`: the
//!   node's channel listener, TCP at its first WebRTC listen address and
//!   the public address, and the offering connection's id. An answer is
//!   `strims-model answer`.
//! - **Linking.** The answering side dials the offer's address and sends
//!   the token. The offering side's listener takes the token to its
//!   connection, and once the answer is applied too (a real connection's
//!   checks need both descriptions) sends a byte back, reports the
//!   connection up (`Signal(Connected)`) and attaches the stream. The
//!   answering side does the same once that byte arrives.
//! - **Failures.** A dial that fails, or a stream the net can't model,
//!   closes the connection (`Signal(Closed)`). A token for no live offer
//!   drops its stream, and so does `Close` for an offer's.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::net::SocketAddr;

use strims_node_core::{
    Input, Inputs, NodeConfig, NodeError, ProxyRtc, RtcCommand, RtcConn, RtcFactory, RtcInput,
    RtcSignal,
};
use strims_vnic::LinkConfig;
use strims_vnic::webrtc::{SdpType, SessionDescription};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use crate::link::Transport;
use crate::net::Net;

const PREFIX: &str = "strims-model";

/// The driver's listener and the proxy's commands, until [`run`] takes
/// them.
pub(crate) struct ModelRtc<N: Net> {
    listener: N::TcpListener,
    /// Where peers reach `listener`.
    pub(crate) advertised: SocketAddr,
    commands: mpsc::UnboundedReceiver<RtcCommand>,
    /// Each channel's most bytes buffered.
    cap: usize,
}

/// Binds the channel listener: the node's endpoint and its driver.
pub(crate) async fn bind<N: Net>(
    net: &N,
    config: &NodeConfig,
) -> Result<(RtcFactory, ModelRtc<N>), NodeError> {
    let w = &config.webrtc;
    let Some(listen) = w.listen.first() else {
        return Err(NodeError::Config(
            "webrtc is enabled without a listen address".to_owned(),
        ));
    };
    let listener = net.tcp_bind(listen.clone()).await?;
    let mut advertised = net.tcp_listener_addr(&listener)?;
    if advertised.ip().is_unspecified() {
        let Some(ip) = w.endpoint.host_ip.or(config.public_ip) else {
            return Err(NodeError::Config(format!(
                "webrtc listen address {advertised} is unspecified: bind a concrete ip, or set \
                 webrtc.endpoint.host_ip or public_ip"
            )));
        };
        advertised.set_ip(ip);
    }
    let (tx, commands) = mpsc::unbounded_channel();
    let model = ModelRtc {
        listener,
        advertised,
        commands,
        cap: w.max_buffered_amount,
    };
    Ok((ProxyRtc::factory(w, 0, tx), model))
}

/// The offer's address and token, if `desc` is one of this module's.
fn parse_offer(desc: &SessionDescription) -> Option<(String, u64)> {
    let mut w = desc.sdp.split_whitespace();
    (w.next()? == PREFIX).then_some(())?;
    let addr = w.next()?.to_owned();
    let token = w.next()?.parse().ok()?;
    Some((addr, token))
}

async fn signal<T>(inputs: &Inputs<T>, conn: RtcConn, signal: RtcSignal) {
    let _ = inputs.send(Input::Signal(RtcInput { conn, signal })).await;
}

/// Reports `conn` up and hands `stream` to the node as its link.
async fn attach<N: Net>(stream: N::TcpStream, conn: RtcConn, inputs: &Inputs<Transport<N>>) {
    signal(inputs, conn, RtcSignal::Connected).await;
    let _ = inputs
        .send(Input::Connected {
            dial: None,
            transport: Transport::Tcp(stream),
            config: LinkConfig::webrtc(),
            leftover: Vec::new(),
            notsent_lowat: None,
            rtc: Some(conn),
        })
        .await;
}

/// The answering side: dials the offer, sends its token, and attaches the
/// stream once the offering side has the answer.
async fn answer<N: Net>(
    net: N,
    addr: String,
    token: u64,
    conn: RtcConn,
    cap: usize,
    inputs: Inputs<Transport<N>>,
) {
    let linked = async {
        let mut s = net.tcp_connect(addr).await?;
        if !net.open_channel(&s, cap) {
            return Err(std::io::Error::other("the net models no data channels"));
        }
        s.write_all(&token.to_be_bytes()).await?;
        s.read_exact(&mut [0u8; 1]).await?;
        Ok(s)
    };
    match linked.await {
        Ok(s) => attach::<N>(s, conn, &inputs).await,
        Err(e) => {
            signal(
                &inputs,
                conn,
                RtcSignal::Closed(format!("model channel: {e}")),
            )
            .await
        }
    }
}

/// One of our offers: whether its answer is applied, and its stream once
/// the answering side's has arrived.
struct Offer<S> {
    conn: RtcConn,
    answered: bool,
    stream: Option<S>,
}

/// Answers the proxy's commands and links its connections, until the
/// proxy or the listener goes away.
pub(crate) async fn run<N: Net>(net: N, model: ModelRtc<N>, inputs: Inputs<Transport<N>>) {
    let ModelRtc {
        listener,
        advertised,
        mut commands,
        cap,
    } = model;
    // Our offers by token. Bound: the endpoint's live connections.
    let mut offers: BTreeMap<u64, Offer<N::TcpStream>> = BTreeMap::new();
    let (accepted_tx, mut accepted) = mpsc::unbounded_channel::<(u64, N::TcpStream)>();
    let accepts = {
        let net = net.clone();
        async move {
            while let Ok((mut s, _)) = net.tcp_accept(&listener).await {
                if !net.open_channel(&s, cap) {
                    continue;
                }
                let tx = accepted_tx.clone();
                tokio::spawn(async move {
                    let mut token = [0u8; 8];
                    if s.read_exact(&mut token).await.is_ok() {
                        let _ = tx.send((u64::from_be_bytes(token), s));
                    }
                });
            }
        }
    };
    tokio::pin!(accepts);
    loop {
        let ready = tokio::select! {
            biased;
            () = &mut accepts => return,
            cmd = commands.recv() => {
                let Some(cmd) = cmd else {
                    return;
                };
                match cmd {
                    RtcCommand::Create { .. } => None,
                    RtcCommand::Describe { conn, sdp_type } => {
                        let sdp = if sdp_type == SdpType::Offer {
                            offers.insert(conn.0, Offer { conn, answered: false, stream: None });
                            format!("{PREFIX} {advertised} {}", conn.0)
                        } else {
                            format!("{PREFIX} answer")
                        };
                        let desc = SessionDescription { sdp_type, sdp };
                        signal(&inputs, conn, RtcSignal::LocalDescription(desc)).await;
                        None
                    }
                    RtcCommand::SetRemote { conn, desc } if desc.sdp_type == SdpType::Offer => {
                        match parse_offer(&desc) {
                            Some((addr, token)) => {
                                let (net, inputs) = (net.clone(), inputs.clone());
                                tokio::spawn(answer(net, addr, token, conn, cap, inputs));
                            }
                            None => {
                                let e = RtcSignal::Closed("not a model offer".to_owned());
                                signal(&inputs, conn, e).await;
                            }
                        }
                        None
                    }
                    RtcCommand::SetRemote { conn, .. } => {
                        offers.get_mut(&conn.0).map(|o| {
                            o.answered = true;
                            conn.0
                        })
                    }
                    RtcCommand::Close { conn } => {
                        offers.remove(&conn.0);
                        None
                    }
                }
            }
            got = accepted.recv() => got.and_then(|(token, s)| {
                offers.get_mut(&token).map(|o| {
                    o.stream = Some(s);
                    token
                })
            }),
        };
        let Some(Entry::Occupied(o)) = ready.map(|token| offers.entry(token)) else {
            continue;
        };
        if !(o.get().answered && o.get().stream.is_some()) {
            continue;
        }
        let Offer { conn, stream, .. } = o.remove();
        let Some(mut s) = stream else {
            continue;
        };
        match s.write_all(&[1]).await {
            Ok(()) => attach::<N>(s, conn, &inputs).await,
            Err(e) => {
                signal(
                    &inputs,
                    conn,
                    RtcSignal::Closed(format!("model channel: {e}")),
                )
                .await
            }
        }
    }
}
