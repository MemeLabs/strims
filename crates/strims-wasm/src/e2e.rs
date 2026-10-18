// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A bare node for the browser link tests, driven over the frontend port.
//!
//! The e2e page (`src/web/e2e.ts`) starts its workers with `options.e2e`
//! set, and relays each worker's frontend port to a WebSocket of the
//! native test harness, one binary WebSocket message per port message.
//! [`start`] runs the control task on that port: the harness starts a
//! node, has it dial, link and send, and hears about its peers and
//! messages.
//!
//! # Protocol
//!
//! Each message is one JSON object, UTF-8 encoded, with a `type`. Byte
//! strings are lowercase hex; host ids are the id's 32 bytes in hex.
//! Numbers are integers of at most 2^53.
//!
//! Commands, from the harness:
//!
//! | `type` | fields | what the node does |
//! |---|---|---|
//! | `start` | `profileKey: {type, private, public}`, `rngSeed`, `networkKey`, `bindPort`, `echoPort`, `replyPort`, and optionally `maxBufferedAmount` | Starts a node of [`node::browser_config`] with `ws://` dials allowed and no ICE servers (host candidates only), and `WebRtcConfig::max_buffered_amount` when given. It joins the network `networkKey` with `NetworkOptions::auto_bind_port` `bindPort`, handles vpn messages on `echoPort` and `replyPort`, and reports `started`. Only the first `start` runs. |
//! | `dial` | `uri` | Dials `uri` and reports `dialed`. |
//! | `connectPeer` | `hostId` | Starts a peer exchange with the host on the network (`Node::connect_peer`) and reports `connecting`. |
//! | `send` | `hostId`, `bytes` | Sends `bytes` from `replyPort` to the host's `echoPort`. Nothing is reported unless it fails. |
//! | `stats` | | Reports `stats`. |
//!
//! Events, from the node:
//!
//! | `type` | fields | when |
//! |---|---|---|
//! | `started` | `hostId` | The node runs. |
//! | `dialed` | `uri`, `hostId` | A dial linked to `hostId`. |
//! | `connecting` | `hostId`, `started` | A `connectPeer` returned; `started` is false when the host is this node, already a peer, or being exchanged with. |
//! | `peerAdded` | `hostId`, `kind` (`tcp`, `ws`, `webrtc` or `other`), `dialed` | A link finished its handshake; `dialed` is set when this node dialed or offered it. |
//! | `peerRemoved` | `hostId` | A peer went away. |
//! | `exchangeClosed` | `hostId`, `outcome` | A peer exchange ended; `outcome` is the `ExchangeOutcome`'s debug form. |
//! | `message` | `from`, `bytes` | A vpn message arrived on `replyPort`. |
//! | `stats` | `echoed`, `echoErrors`, `eventsDropped`, `linkQueueFull`, `deliveriesDropped`, `links: [{kind, mtu, readBytes, writeBytes}]` | Answers `stats`. |
//! | `error` | `command`, `message` | A command didn't parse or failed, or an echo failed (`command` `echo`). |
//!
//! Every vpn message on `echoPort` goes back to its sender's port,
//! unchanged and from `echoPort`, in the order the messages arrived; it
//! isn't reported. Commands run in the order they arrive, and each runs
//! to completion before the next, except `dial` and `connectPeer`, which
//! report when they finish.

use std::cell::Cell;
use std::fmt::Write as _;
use std::rc::Rc;

use js_sys::{Array, ArrayBuffer, JSON, Object, Reflect, Uint8Array};
use strims_node_core::{HostId, LinkKind, NetworkId, NetworkOptions, Node, NodeEvent, NodeEvents};
use strims_proto::types::Key;
use tokio::sync::mpsc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, MessagePort};

use crate::{js, node, runtime};

/// Whether `options` asks for the e2e node: `options.e2e` is `true`.
pub fn requested(options: &JsValue) -> bool {
    property(options, "e2e").as_bool() == Some(true)
}

/// Runs the control task on `frontend` (see the [module docs](self)). The
/// node it starts signals WebRTC with the window over `signal`.
pub fn start(frontend: MessagePort, signal: MessagePort) {
    let (commands_tx, commands) = mpsc::unbounded_channel();
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
        // Anything but an ArrayBuffer is no bytes, which don't parse.
        let bytes = match e.data().dyn_into::<ArrayBuffer>() {
            Ok(b) => Uint8Array::new(&b).to_vec(),
            Err(_) => Vec::new(),
        };
        let _ = commands_tx.send(bytes);
    });
    // Setting `onmessage` starts the port, which delivers what it buffered.
    frontend.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    let control = Control {
        out: Out(frontend),
        signal: Some(signal),
        running: None,
        _on_message: on_message,
    };
    // Detached: the worker runs it for as long as it lives.
    drop(runtime::spawn(control.run(commands)));
}

/// The harness's end of the frontend port.
#[derive(Clone)]
struct Out(MessagePort);

impl Out {
    /// Posts `fields` as one JSON message.
    fn post(&self, fields: &[(&str, JsValue)]) {
        let o = Object::new();
        for (k, v) in fields {
            let _ = Reflect::set(&o, &JsValue::from_str(k), v);
        }
        let text = match JSON::stringify(&o) {
            Ok(t) => String::from(t),
            Err(e) => {
                tracing::warn!(error = %js::describe(&e), "e2e: unencodable event");
                return;
            }
        };
        let buffer = Uint8Array::from(text.as_bytes()).buffer();
        if let Err(e) = self
            .0
            .post_message_with_transferable(&buffer, &Array::of1(&buffer))
        {
            tracing::warn!(error = %js::describe(&e), "e2e: frontend port");
        }
    }

    fn error(&self, command: &str, message: &str) {
        self.post(&[
            ("type", "error".into()),
            ("command", command.into()),
            ("message", message.into()),
        ]);
    }
}

/// The started node and what its commands and echoes need.
#[derive(Clone)]
struct Running {
    node: Node,
    net: NetworkId,
    echo_port: u16,
    reply_port: u16,
    counts: Rc<Counts>,
}

#[derive(Default)]
struct Counts {
    echoed: Cell<u64>,
    echo_errors: Cell<u64>,
}

struct Control {
    out: Out,
    /// Until `start` hands it to the node.
    signal: Option<MessagePort>,
    running: Option<Running>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl Control {
    async fn run(mut self, mut commands: mpsc::UnboundedReceiver<Vec<u8>>) {
        while let Some(bytes) = commands.recv().await {
            let cmd = match parse(&bytes) {
                Ok(c) => c,
                Err(e) => {
                    self.out.error("", &e);
                    continue;
                }
            };
            let kind = property(&cmd, "type").as_string().unwrap_or_default();
            if let Err(e) = self.command(&kind, &cmd).await {
                self.out.error(&kind, &e);
            }
        }
    }

    async fn command(&mut self, kind: &str, cmd: &JsValue) -> Result<(), String> {
        if kind == "start" {
            return self.start(cmd).await;
        }
        let r = self.running.clone().ok_or("the node isn't started")?;
        match kind {
            "dial" => {
                let uri = string(cmd, "uri")?;
                let out = self.out.clone();
                // Detached: the dial timeout bounds it.
                drop(runtime::spawn(async move {
                    match r.node.dial(&uri).await {
                        Ok(host) => out.post(&[
                            ("type", "dialed".into()),
                            ("uri", uri.as_str().into()),
                            ("hostId", host_id(host)),
                        ]),
                        Err(e) => out.error("dial", &e.to_string()),
                    }
                }));
            }
            "connectPeer" => {
                let host = host(cmd, "hostId")?;
                let out = self.out.clone();
                drop(runtime::spawn(async move {
                    match r.node.connect_peer(r.net, host).await {
                        Ok(started) => out.post(&[
                            ("type", "connecting".into()),
                            ("hostId", host_id(host)),
                            ("started", started.into()),
                        ]),
                        Err(e) => out.error("connectPeer", &e.to_string()),
                    }
                }));
            }
            "send" => {
                let host = host(cmd, "hostId")?;
                let body = bytes(cmd, "bytes")?;
                r.node
                    .send(r.net, host, r.echo_port, r.reply_port, body)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            "stats" => {
                let m = r.node.metrics().await.map_err(|e| e.to_string())?;
                let links: Array = m
                    .links
                    .iter()
                    .map(|l| {
                        let o = Object::new();
                        let set = |k: &str, v: JsValue| {
                            let _ = Reflect::set(&o, &JsValue::from_str(k), &v);
                        };
                        set("kind", link_kind(l.kind).into());
                        set("mtu", number(l.mtu as u64));
                        set("readBytes", number(l.metrics.read_bytes));
                        set("writeBytes", number(l.metrics.write_bytes));
                        JsValue::from(o)
                    })
                    .collect();
                self.out.post(&[
                    ("type", "stats".into()),
                    ("echoed", number(r.counts.echoed.get())),
                    ("echoErrors", number(r.counts.echo_errors.get())),
                    ("eventsDropped", number(m.driver.events_dropped)),
                    ("linkQueueFull", number(m.vpn.link_queue_full)),
                    ("deliveriesDropped", number(m.vpn.deliveries_dropped)),
                    ("links", links.into()),
                ]);
            }
            _ => return Err("unknown command".to_owned()),
        }
        Ok(())
    }

    async fn start(&mut self, cmd: &JsValue) -> Result<(), String> {
        let key = property(cmd, "profileKey");
        let profile_key = Key {
            r#type: i32::try_from(int(&key, "type")?).map_err(|e| e.to_string())?,
            private: bytes(&key, "private")?,
            public: bytes(&key, "public")?,
        };
        let rng_seed = int(cmd, "rngSeed")?;
        let network_key = bytes(cmd, "networkKey")?;
        let bind_port = port(cmd, "bindPort")?;
        let echo_port = port(cmd, "echoPort")?;
        let reply_port = port(cmd, "replyPort")?;
        let max_buffered = match property(cmd, "maxBufferedAmount") {
            v if v.is_undefined() => None,
            _ => Some(usize::try_from(int(cmd, "maxBufferedAmount")?).map_err(|e| e.to_string())?),
        };
        let signal = self.signal.take().ok_or("already started")?;

        let mut config = node::browser_config(profile_key, rng_seed);
        config.allow_insecure_ws = true;
        config.webrtc.endpoint.ice_servers = Some(Vec::new());
        if let Some(m) = max_buffered {
            config.webrtc.max_buffered_amount = m;
        }
        let (node, events) = node::spawn(config, signal).map_err(|e| e.to_string())?;
        let options = NetworkOptions {
            auto_bind_port: Some(bind_port),
        };
        let net = node
            .add_network(&network_key, options)
            .await
            .map_err(|e| e.to_string())?;
        for p in [echo_port, reply_port] {
            node.set_vpn_handler(net, p)
                .await
                .map_err(|e| e.to_string())?;
        }
        let running = Running {
            node,
            net,
            echo_port,
            reply_port,
            counts: Rc::default(),
        };
        // It runs once this task waits: after `started` is posted.
        drop(runtime::spawn(watch(
            running.clone(),
            events,
            self.out.clone(),
        )));
        let id = running.node.host_id();
        tracing::info!(host_id = %id, "e2e node started");
        self.out
            .post(&[("type", "started".into()), ("hostId", host_id(id))]);
        self.running = Some(running);
        Ok(())
    }
}

/// Reports the node's events and echoes the messages on the echo port,
/// until the node stops.
async fn watch(r: Running, mut events: NodeEvents, out: Out) {
    while let Some(e) = events.recv().await {
        match e {
            NodeEvent::PeerAdded(p) => out.post(&[
                ("type", "peerAdded".into()),
                ("hostId", host_id(p.host_id)),
                ("kind", link_kind(p.kind).into()),
                ("dialed", p.dialed.into()),
            ]),
            NodeEvent::PeerRemoved { host_id: h } => {
                out.post(&[("type", "peerRemoved".into()), ("hostId", host_id(h))])
            }
            NodeEvent::ExchangeClosed {
                host_id: h,
                outcome,
                ..
            } => out.post(&[
                ("type", "exchangeClosed".into()),
                ("hostId", host_id(h)),
                ("outcome", format!("{outcome:?}").into()),
            ]),
            NodeEvent::Message(d) if d.dst_port == r.echo_port => {
                // Awaited before the next event, so echoes keep their order.
                match r
                    .node
                    .send(r.net, d.src, d.src_port, r.echo_port, d.body)
                    .await
                {
                    Ok(()) => r.counts.echoed.set(r.counts.echoed.get() + 1),
                    Err(e) => {
                        r.counts.echo_errors.set(r.counts.echo_errors.get() + 1);
                        out.error("echo", &e.to_string());
                    }
                }
            }
            NodeEvent::Message(d) if d.dst_port == r.reply_port => out.post(&[
                ("type", "message".into()),
                ("from", host_id(d.src)),
                ("bytes", hex(&d.body).into()),
            ]),
            _ => {}
        }
    }
}

fn link_kind(kind: LinkKind) -> &'static str {
    match kind {
        LinkKind::Tcp => "tcp",
        LinkKind::Ws => "ws",
        LinkKind::WebRtc => "webrtc",
        LinkKind::Other => "other",
    }
}

fn host_id(id: HostId) -> JsValue {
    hex(&id.to_bytes()).into()
}

fn number(n: u64) -> JsValue {
    // Counters stay far below 2^53.
    JsValue::from_f64(n as f64)
}

fn parse(bytes: &[u8]) -> Result<JsValue, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let v = JSON::parse(text).map_err(|e| js::describe(&e))?;
    if v.is_object() {
        Ok(v)
    } else {
        Err("not a JSON object".to_owned())
    }
}

fn property(v: &JsValue, key: &str) -> JsValue {
    if !v.is_object() {
        return JsValue::UNDEFINED;
    }
    Reflect::get(v, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
}

fn string(v: &JsValue, key: &str) -> Result<String, String> {
    property(v, key)
        .as_string()
        .ok_or_else(|| format!("{key}: not a string"))
}

/// A non-negative integer of at most 2^53.
fn int(v: &JsValue, key: &str) -> Result<u64, String> {
    match property(v, key).as_f64() {
        Some(n) if n.fract() == 0.0 && (0.0..=9_007_199_254_740_992.0).contains(&n) => Ok(n as u64),
        _ => Err(format!("{key}: not an integer")),
    }
}

fn port(v: &JsValue, key: &str) -> Result<u16, String> {
    u16::try_from(int(v, key)?).map_err(|_| format!("{key}: not a port"))
}

fn bytes(v: &JsValue, key: &str) -> Result<Vec<u8>, String> {
    unhex(&string(v, key)?).ok_or_else(|| format!("{key}: not hex"))
}

fn host(v: &JsValue, key: &str) -> Result<HostId, String> {
    let b: [u8; 32] = bytes(v, key)?
        .try_into()
        .map_err(|_| format!("{key}: not 32 bytes"))?;
    Ok(HostId::from_bytes(&b))
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        let _ = write!(s, "{x:02x}");
    }
    s
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    let (pairs, rest) = s.as_bytes().as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    pairs
        .iter()
        .map(|&[hi, lo]| {
            let hi = char::from(hi).to_digit(16)?;
            let lo = char::from(lo).to_digit(16)?;
            u8::try_from(hi << 4 | lo).ok()
        })
        .collect()
}
