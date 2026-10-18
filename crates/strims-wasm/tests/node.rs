// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A node in a dedicated worker: starting, querying and stopping it, its
//! dials, and the signal port and data channels the window's side drives.
//!
//! Workers have no `RTCPeerConnection`, so the window is scripted: a test
//! answers the signal port from the other end of a `MessageChannel`, and a
//! plain JS object stands in for a transferred data channel.

#![cfg(target_arch = "wasm32")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::future::Future;
use std::task::Poll;

use js_sys::{Array, Function, Object, Reflect, Uint8Array};
use slotmap::KeyData;
use strims_base::time::Duration;
use strims_crypto::key_from_seed;
use strims_node_core::{
    Input, Interfaces, LinkChannels, LinkKey, NodeConfig, NodeError, NodeIo, ProxyRtc, RtcConn,
    RtcEndpoint, RtcEvent, RtcInput, RtcSignal, WebRtcConfig,
};
use strims_rpc::Timer;
use strims_vnic::webrtc::{
    DEFAULT_ICE_SERVERS, RemoteDescriptionAction, SdpType, SessionDescription,
};
use strims_wasm::clock::{self, WasmClock};
use strims_wasm::io::{WasmIo, WasmTransport};
use strims_wasm::rtc::SignalPort;
use strims_wasm::{node, runtime, ws};
use tokio::sync::{mpsc, oneshot};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;
use web_sys::{MessageChannel, MessageEvent, MessagePort};

wasm_bindgen_test_configure!(run_in_dedicated_worker);

fn config(seed: u8) -> NodeConfig {
    node::browser_config(key_from_seed(&[seed; 32]), u64::from(seed))
}

/// A signal port whose other end nobody reads.
fn idle_port() -> MessagePort {
    MessageChannel::new().unwrap().port1()
}

fn peer_key() -> String {
    "00".repeat(32)
}

async fn sleep(d: Duration) {
    WasmClock.sleep_until(clock::now() + d).await;
}

/// `f`, or a panic naming `what` after `d`.
async fn within<F: Future>(d: Duration, what: &str, f: F) -> F::Output {
    let deadline = WasmClock.sleep_until(clock::now() + d);
    tokio::select! {
        biased;
        v = f => v,
        () = deadline => panic!("timed out waiting for {what}"),
    }
}

fn get(v: &JsValue, key: &str) -> JsValue {
    Reflect::get(v, &JsValue::from_str(key)).unwrap()
}

fn set(v: &JsValue, key: &str, value: &JsValue) {
    Reflect::set(v, &JsValue::from_str(key), value).unwrap();
}

fn object(fields: &[(&str, JsValue)]) -> JsValue {
    let o = Object::new();
    for (k, v) in fields {
        set(&o, k, v);
    }
    o.into()
}

fn variant(i: &Input<WasmTransport>) -> &'static str {
    match i {
        Input::Read { .. } => "Read",
        Input::WriteDone { .. } => "WriteDone",
        Input::LinkDown { .. } => "LinkDown",
        Input::Connected { .. } => "Connected",
        Input::DialFailed { .. } => "DialFailed",
        Input::AcceptFailed { .. } => "AcceptFailed",
        Input::Datagram { .. } => "Datagram",
        Input::StunServers(_) => "StunServers",
        Input::Signal(_) => "Signal",
    }
}

async fn next_input(rx: &mut mpsc::Receiver<Input<WasmTransport>>) -> Input<WasmTransport> {
    within(Duration::from_secs(5), "an input", rx.recv())
        .await
        .expect("the inputs channel ended")
}

async fn next_signal(rx: &mut mpsc::Receiver<Input<WasmTransport>>) -> RtcInput {
    match next_input(rx).await {
        Input::Signal(s) => s,
        other => panic!("expected a signal, got {}", variant(&other)),
    }
}

/// Nothing more arrives for a while.
async fn quiet(rx: &mut mpsc::Receiver<Input<WasmTransport>>) {
    sleep(Duration::from_millis(50)).await;
    if let Ok(i) = rx.try_recv() {
        panic!("unexpected {}", variant(&i));
    }
}

/// The window's end of a signal port.
struct FakeWindow {
    port: MessagePort,
    rx: mpsc::UnboundedReceiver<JsValue>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl FakeWindow {
    fn new(port: MessagePort) -> FakeWindow {
        let (tx, rx) = mpsc::unbounded_channel();
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            let _ = tx.send(e.data());
        });
        port.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        FakeWindow {
            port,
            rx,
            _on_message: on_message,
        }
    }

    /// The next message from the worker, which must be of `kind`.
    async fn next(&mut self, kind: &str) -> JsValue {
        let msg = within(Duration::from_secs(5), kind, self.rx.recv())
            .await
            .unwrap();
        assert_eq!(
            get(&msg, "type").as_string().as_deref(),
            Some(kind),
            "{msg:?}"
        );
        msg
    }

    fn post(&self, fields: &[(&str, JsValue)]) {
        self.port.post_message(&object(fields)).unwrap();
    }

    fn assert_quiet(&mut self) {
        if let Ok(m) = self.rx.try_recv() {
            panic!("unexpected message {m:?}");
        }
    }
}

fn n(conn: RtcConn) -> JsValue {
    JsValue::from_f64(conn.0 as f64)
}

fn description(sdp_type: SdpType) -> SessionDescription {
    SessionDescription {
        sdp_type,
        sdp: format!("v=0\r\ns={}\r\n", sdp_type.as_str()),
    }
}

/// A proxy and the signal task carrying its commands, over a channel whose
/// other end is the returned window.
struct Signalling {
    rtc: ProxyRtc,
    window: FakeWindow,
    /// The worker's end, for calling its message handler directly.
    port: MessagePort,
    inputs: mpsc::Receiver<Input<WasmTransport>>,
    done: oneshot::Receiver<()>,
}

fn signalling() -> Signalling {
    let channel = MessageChannel::new().unwrap();
    let window = FakeWindow::new(channel.port2());
    let (commands, commands_rx) = mpsc::unbounded_channel();
    let (inputs_tx, inputs) = mpsc::channel(16);
    let config = WebRtcConfig {
        enabled: true,
        ..WebRtcConfig::default()
    };
    let rtc = ProxyRtc::new(&config, 0, commands);
    let port = channel.port1();
    let signal = SignalPort::new(port.clone(), commands_rx, config.max_buffered_amount);
    let (done_tx, done) = oneshot::channel();
    drop(runtime::spawn(async move {
        signal.run(inputs_tx).await;
        let _ = done_tx.send(());
    }));
    Signalling {
        rtc,
        window,
        port,
        inputs,
        done,
    }
}

#[wasm_bindgen_test]
async fn a_browser_node_starts_reports_and_shuts_down() {
    let (node, _events) = node::spawn(config(1), idle_port()).unwrap();
    assert_eq!(
        node.interfaces(),
        Interfaces {
            tcp: false,
            wss: true
        }
    );
    let info = node.info();
    assert_eq!(
        (info.tcp_uri.as_deref(), info.ws_uri.as_deref()),
        (None, None)
    );
    assert!(info.tcp_addr.is_none() && info.ws_addr.is_none());
    assert!(info.webrtc_addrs.is_empty());

    let metrics = node.metrics().await.unwrap();
    assert!(metrics.links.is_empty());
    assert_eq!(metrics.driver.tcp_dials + metrics.driver.ws_dials, 0);
    assert!(node.peers().await.unwrap().is_empty());

    node.shutdown().await.unwrap();
    assert_eq!(node.metrics().await.unwrap_err(), NodeError::Stopped);
    assert_eq!(node.peers().await.unwrap_err(), NodeError::Stopped);
}

/// A browser can't listen.
#[wasm_bindgen_test]
fn listeners_are_refused() {
    let mut c = config(2);
    c.ws_listen = Some("127.0.0.1:0".to_owned());
    assert!(matches!(
        node::spawn(c, idle_port()),
        Err(NodeError::Unsupported(_))
    ));
}

#[wasm_bindgen_test]
async fn ws_dials_need_allow_insecure_ws() {
    let (node, _events) = node::spawn(config(3), idle_port()).unwrap();
    let err = node
        .dial(&format!("ws://127.0.0.1:1/{}", peer_key()))
        .await
        .unwrap_err();
    assert!(matches!(err, NodeError::InvalidUri(_)), "{err:?}");
    assert_eq!(node.metrics().await.unwrap().driver.ws_dials, 0);
    node.shutdown().await.unwrap();

    // With the option, the dial reaches the network (and fails there).
    let mut c = config(4);
    c.allow_insecure_ws = true;
    let (node, _events) = node::spawn(c, idle_port()).unwrap();
    let err = node
        .dial(&format!("ws://127.0.0.1:1/{}", peer_key()))
        .await
        .unwrap_err();
    assert!(matches!(err, NodeError::Dial(_)), "{err:?}");
    node.shutdown().await.unwrap();
}

#[wasm_bindgen_test]
async fn tcp_dials_are_unsupported() {
    let (node, _events) = node::spawn(config(5), idle_port()).unwrap();
    let err = node
        .dial(&format!("tcp://127.0.0.1:1/{}", peer_key()))
        .await
        .unwrap_err();
    assert!(matches!(err, NodeError::Unsupported(_)), "{err:?}");
    assert_eq!(node.metrics().await.unwrap().driver.tcp_dials, 0);
    node.shutdown().await.unwrap();
}

/// Nothing listens on port 1: the dial fails well within its timeout, and
/// leaves no socket behind.
#[wasm_bindgen_test]
async fn a_dead_wss_dial_fails_within_the_timeout() {
    let mut c = config(6);
    let timeout = Duration::from_secs(5);
    c.dial_timeout = timeout;
    let (node, _events) = node::spawn(c, idle_port()).unwrap();
    let started = clock::now();
    let err = within(
        timeout + Duration::from_secs(1),
        "the dial",
        node.dial(&format!("wss://127.0.0.1:1/{}", peer_key())),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, NodeError::Dial(_)), "{err:?}");
    assert!(
        clock::now() - started <= timeout,
        "{:?}",
        clock::now() - started
    );
    let d = node.metrics().await.unwrap().driver;
    assert_eq!((d.ws_dials, d.ws_dial_errors), (1, 1));
    assert_eq!(ws::live_sockets(), 0);
    node.shutdown().await.unwrap();
}

/// One connection's signalling end to end: the proxy's commands reach the
/// window as messages, and the window's answers reach the node as
/// signals. Messages about other connections, or about this one once it
/// closed, are dropped.
#[wasm_bindgen_test]
async fn the_signal_port_carries_a_connections_signalling() {
    let mut s = signalling();
    let now = clock::now();
    let conn = s.rtc.create_connection(now).unwrap();
    let create = s.window.next("create").await;
    assert_eq!(get(&create, "conn"), n(conn));
    let ice: Vec<String> = Array::from(&get(&create, "iceServers"))
        .iter()
        .map(|v| v.as_string().unwrap())
        .collect();
    assert_eq!(ice, DEFAULT_ICE_SERVERS);
    assert_eq!(get(&create, "gatherTimeoutMs").as_f64(), Some(5000.0));

    assert!(s.rtc.poll_local_description(now, conn).is_pending());
    let describe = s.window.next("describe").await;
    assert_eq!(get(&describe, "conn"), n(conn));
    assert_eq!(
        get(&describe, "sdpType").as_string().as_deref(),
        Some("offer")
    );

    // As `JSON.stringify(pc.localDescription)` writes it.
    let offer = description(SdpType::Offer);
    let json = js_sys::JSON::stringify(&object(&[
        ("type", "offer".into()),
        ("sdp", offer.sdp.as_str().into()),
    ]))
    .unwrap();
    s.window.post(&[
        ("type", "local".into()),
        ("conn", n(conn)),
        ("description", json.into()),
    ]);
    let local = next_signal(&mut s.inputs).await;
    assert_eq!(
        local,
        RtcInput {
            conn,
            signal: RtcSignal::LocalDescription(offer.clone())
        }
    );
    s.rtc.handle_signal(now, local);
    assert_eq!(s.rtc.poll_event(), Some(RtcEvent::LocalDescription(conn)));
    assert_eq!(
        s.rtc.poll_local_description(now, conn),
        Poll::Ready(Ok(offer))
    );

    let answer = description(SdpType::Answer);
    assert_eq!(
        s.rtc.set_remote_description(now, conn, &answer),
        Ok(RemoteDescriptionAction::AwaitLink)
    );
    let remote = s.window.next("remote").await;
    assert_eq!(get(&remote, "conn"), n(conn));
    let parsed = js_sys::JSON::parse(&get(&remote, "description").as_string().unwrap()).unwrap();
    assert_eq!(get(&parsed, "type").as_string().as_deref(), Some("answer"));
    assert_eq!(get(&parsed, "sdp").as_string(), Some(answer.sdp));

    s.window.post(&[
        ("type", "connected".into()),
        ("conn", n(conn)),
        ("maxMessageSize", 65536.into()),
    ]);
    assert_eq!(
        next_signal(&mut s.inputs).await,
        RtcInput {
            conn,
            signal: RtcSignal::Connected
        }
    );
    // No channel arrived, so nothing is attached.
    quiet(&mut s.inputs).await;

    // Unknown connections, ids that aren't integers, unknown types and
    // visibility are dropped; then a description that isn't JSON closes
    // the connection.
    s.window.post(&[
        ("type", "local".into()),
        ("conn", JsValue::from_f64(conn.0 as f64 + 1.0)),
        ("description", "{}".into()),
    ]);
    s.window.post(&[
        ("type", "connected".into()),
        ("conn", JsValue::from_f64(conn.0 as f64 + 0.5)),
        ("maxMessageSize", 0.into()),
    ]);
    s.window
        .post(&[("type", "bogus".into()), ("conn", n(conn))]);
    s.window.post(&[
        ("type", "visibility".into()),
        ("hidden", true.into()),
        ("forceHUP", true.into()),
    ]);
    s.window.post(&[
        ("type", "local".into()),
        ("conn", n(conn)),
        ("description", "not json".into()),
    ]);
    let closed = next_signal(&mut s.inputs).await;
    assert_eq!(closed.conn, conn);
    assert!(
        matches!(&closed.signal, RtcSignal::Closed(r) if r.starts_with("invalid local description")),
        "{closed:?}"
    );
    s.rtc.handle_signal(now, closed);
    assert!(matches!(
        s.rtc.poll_event(),
        Some(RtcEvent::Closed { conn: c, .. }) if c == conn
    ));
    assert_eq!(get(&s.window.next("close").await, "conn"), n(conn));

    // Closed: the window's late messages about it are dropped.
    s.window.post(&[
        ("type", "closed".into()),
        ("conn", n(conn)),
        ("reason", "connection closed".into()),
    ]);
    quiet(&mut s.inputs).await;

    // The task ends with the proxy, having sent its last commands.
    let other = s.rtc.create_connection(now).unwrap();
    s.rtc.close_all(now);
    drop(s.rtc);
    assert_eq!(get(&s.window.next("create").await, "conn"), n(other));
    assert_eq!(get(&s.window.next("close").await, "conn"), n(other));
    within(Duration::from_secs(5), "the signal task to end", s.done)
        .await
        .unwrap();
    s.window.assert_quiet();
    drop(s.port);
}

/// A plain object with the parts of an `RTCDataChannel` the worker uses.
/// `send` keeps copies of what it was given in `sent`.
fn fake_channel() -> JsValue {
    Function::new_no_args(
        "return {
            readyState: 'connecting',
            bufferedAmount: 0,
            binaryType: 'blob',
            bufferedAmountLowThreshold: 0,
            sent: [],
            closed: false,
            send(d) { this.sent.push(d.slice()); },
            close() { this.closed = true; this.readyState = 'closed'; },
        };",
    )
    .call0(&JsValue::NULL)
    .unwrap()
}

/// Fires `channel.on<event>(arg)`, as the browser would.
fn fire(channel: &JsValue, event: &str, arg: &JsValue) {
    let handler: Function = get(channel, &format!("on{event}")).dyn_into().unwrap();
    handler.call1(channel, arg).unwrap();
}

fn bytes(data: &[u8]) -> JsValue {
    Uint8Array::from(data).buffer().into()
}

/// A transferred channel is set up as it arrives, and once it is open and
/// its connection is up it goes to the node as a link of the connection,
/// with the connection's message size as its MTU. As a link it reads into
/// the node's buffers, writes while the channel has room, and closes when
/// the node drops it.
#[wasm_bindgen_test]
async fn a_transferred_channel_becomes_a_link() {
    let mut s = signalling();
    let now = clock::now();
    let conn = s.rtc.create_connection(now).unwrap();
    s.window.next("create").await;

    // The window's `channel` message, as the port delivers it. A channel
    // can't be posted from a worker, so the handler is called directly.
    let channel = fake_channel();
    let handler = s.port.onmessage().unwrap();
    let message = |conn: JsValue, channel: &JsValue| {
        object(&[(
            "data",
            object(&[
                ("type", "channel".into()),
                ("conn", conn),
                ("channel", channel.clone()),
            ]),
        )])
    };
    handler
        .call1(&JsValue::NULL, &message(n(conn), &channel))
        .unwrap();
    assert_eq!(
        get(&channel, "binaryType").as_string().as_deref(),
        Some("arraybuffer")
    );
    assert_eq!(
        get(&channel, "bufferedAmountLowThreshold").as_f64(),
        Some(131072.0)
    );

    // A channel for a connection nobody made is closed.
    let stray = fake_channel();
    handler
        .call1(
            &JsValue::NULL,
            &message(JsValue::from_f64(conn.0 as f64 + 7.0), &stray),
        )
        .unwrap();
    sleep(Duration::from_millis(20)).await;
    assert_eq!(get(&stray, "closed"), JsValue::TRUE);

    s.window.post(&[
        ("type", "connected".into()),
        ("conn", n(conn)),
        ("maxMessageSize", 16384.into()),
    ]);
    assert_eq!(
        next_signal(&mut s.inputs).await.signal,
        RtcSignal::Connected
    );
    quiet(&mut s.inputs).await;
    set(&channel, "readyState", &"open".into());
    fire(&channel, "open", &JsValue::UNDEFINED);
    let transport = match next_input(&mut s.inputs).await {
        Input::Connected {
            dial: None,
            transport,
            config,
            leftover,
            notsent_lowat: None,
            rtc: Some(c),
        } => {
            assert_eq!(c, conn);
            assert_eq!(config.mtu, 16384);
            assert!(leftover.is_empty());
            transport
        }
        other => panic!("expected the channel, got {}", variant(&other)),
    };
    assert!(matches!(transport, WasmTransport::Channel(_)));
    assert_eq!(get(&channel, "closed"), JsValue::FALSE);

    // The link's tasks, on a driver of its own.
    let (_, mut io) = WasmIo::new(&config(7), idle_port()).unwrap();
    let link = LinkKey::from(KeyData::from_ffi(1));
    let (free_tx, read_free) = mpsc::channel(2);
    let (writes_tx, writes) = mpsc::channel(2);
    let (inputs_tx, mut inputs) = mpsc::channel(8);
    let _tasks = io.spawn_link(
        link,
        transport,
        LinkChannels {
            read_free,
            writes,
            inputs: inputs_tx,
        },
    );

    // A message larger than a read buffer is split; queued messages are
    // read together.
    fire(
        &channel,
        "message",
        &object(&[("data", bytes(&[1, 2, 3, 4, 5, 6]))]),
    );
    fire(&channel, "message", &object(&[("data", bytes(&[7]))]));
    free_tx.try_send(vec![0; 4]).unwrap();
    let mut read = Vec::new();
    while read.len() < 7 {
        match next_input(&mut inputs).await {
            Input::Read { link: l, buf, len } => {
                assert_eq!(l, link);
                read.extend_from_slice(&buf[..len]);
                free_tx.try_send(buf).unwrap();
            }
            other => panic!("expected a read, got {}", variant(&other)),
        }
    }
    assert_eq!(read, [1, 2, 3, 4, 5, 6, 7]);

    // Writes go out while the channel has room, and wait for
    // `bufferedamountlow` once it is full.
    writes_tx.try_send(vec![9, 9]).unwrap();
    match next_input(&mut inputs).await {
        Input::WriteDone { buf, .. } => assert_eq!(buf, [9, 9]),
        other => panic!("expected a write, got {}", variant(&other)),
    }
    set(&channel, "bufferedAmount", &(256 * 1024).into());
    writes_tx.try_send(vec![8]).unwrap();
    quiet(&mut inputs).await;
    set(&channel, "bufferedAmount", &0.into());
    fire(&channel, "bufferedamountlow", &JsValue::UNDEFINED);
    match next_input(&mut inputs).await {
        Input::WriteDone { buf, .. } => assert_eq!(buf, [8]),
        other => panic!("expected a write, got {}", variant(&other)),
    }
    let sent: Vec<Vec<u8>> = Array::from(&get(&channel, "sent"))
        .iter()
        .map(|v| Uint8Array::new(&v).to_vec())
        .collect();
    assert_eq!(sent, [vec![9, 9], vec![8]]);

    // The node dropping the link closes the channel.
    drop(writes_tx);
    sleep(Duration::from_millis(20)).await;
    assert_eq!(get(&channel, "closed"), JsValue::TRUE);
}

/// A text message fails a channel link, after the bytes before it.
#[wasm_bindgen_test]
async fn a_text_message_fails_a_channel_link() {
    let mut s = signalling();
    let now = clock::now();
    let conn = s.rtc.create_connection(now).unwrap();
    s.window.next("create").await;
    let channel = fake_channel();
    // Already open when it arrives.
    set(&channel, "readyState", &"open".into());
    s.port
        .onmessage()
        .unwrap()
        .call1(
            &JsValue::NULL,
            &object(&[(
                "data",
                object(&[
                    ("type", "channel".into()),
                    ("conn", n(conn)),
                    ("channel", channel.clone()),
                ]),
            )]),
        )
        .unwrap();
    s.window.post(&[
        ("type", "connected".into()),
        ("conn", n(conn)),
        ("maxMessageSize", 0.into()),
    ]);
    next_signal(&mut s.inputs).await;
    let transport = match next_input(&mut s.inputs).await {
        Input::Connected {
            transport, config, ..
        } => {
            assert_eq!(config.mtu, 64 * 1024, "0 is the default size");
            transport
        }
        other => panic!("expected the channel, got {}", variant(&other)),
    };

    let (_, mut io) = WasmIo::new(&config(8), idle_port()).unwrap();
    let link = LinkKey::from(KeyData::from_ffi(2));
    let (free_tx, read_free) = mpsc::channel(1);
    let (_writes_tx, writes) = mpsc::channel(1);
    let (inputs_tx, mut inputs) = mpsc::channel(8);
    let _tasks = io.spawn_link(
        link,
        transport,
        LinkChannels {
            read_free,
            writes,
            inputs: inputs_tx,
        },
    );
    fire(&channel, "message", &object(&[("data", bytes(&[5]))]));
    fire(&channel, "message", &object(&[("data", "hello".into())]));
    free_tx.try_send(vec![0; 16]).unwrap();
    match next_input(&mut inputs).await {
        Input::Read { buf, len, .. } => {
            assert_eq!(&buf[..len], [5]);
            free_tx.try_send(buf).unwrap();
        }
        other => panic!("expected a read, got {}", variant(&other)),
    }
    match next_input(&mut inputs).await {
        Input::LinkDown { link: l, error } => {
            assert_eq!(l, link);
            assert_eq!(error, "unexpected text message");
        }
        other => panic!("expected the link to fail, got {}", variant(&other)),
    }
}
