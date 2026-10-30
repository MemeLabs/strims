// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A bare node for the browser link tests and the testbed's browser
//! nodes, driven over the frontend port.
//!
//! The e2e page (`src/web/e2e.ts`) starts its workers with `options.e2e`
//! set, and relays each worker's frontend port to a WebSocket of the
//! native harness, one binary WebSocket message per port message.
//! [`start`] runs the control task on that port: the harness starts a
//! node, has it dial, link and send, and hears about its peers and
//! messages. A testbed harness (strims-testbed's `testbed_browser`) also
//! has it run the testbed's swarm ([`strims_testbed::node`]).
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
//! | `start` | `profileKey: {type, private, public}`, `rngSeed`, `networkKey`, `bindPort`, `echoPort`, `replyPort`, and optionally `maxBufferedAmount` and `capture: {maxFileBytes, tap, label}` | Starts a node of [`node::browser_config`] with `ws://` dials allowed and no ICE servers (host candidates only), and `WebRtcConfig::max_buffered_amount` when given. With `capture`, the node captures its ppspp traffic (`NodeConfig::ppspp_capture`, labelled `label`) and sends the dump's bytes (see below). It joins the network `networkKey` with `NetworkOptions::auto_bind_port` `bindPort`, handles vpn messages on `echoPort` and `replyPort`, and reports `started`. Only the first `start` runs. |
//! | `dial` | `uri` | Dials `uri` and reports `dialed`. |
//! | `connectPeer` | `hostId` | Starts a peer exchange with the host on the network (`Node::connect_peer`) and reports `connecting`. |
//! | `send` | `hostId`, `bytes` | Sends `bytes` from `replyPort` to the host's `echoPort`. Nothing is reported unless it fails. |
//! | `stats` | | Reports `stats`. |
//! | `testbed` | `index`, `nodes`, `startMs`, `secs`, and optionally `rate`, `chunk` with `munro`, and `units` | Runs the testbed's swarm as node `index` of `nodes` ([`strims_testbed::node::run`]) with every peer added since `start`, and reports `sample` each second. With `units`, the stream is media in the unit format in a swarm of `units` chunks per signature ([`strims_testbed::media`]), which the node reads as a viewer, sending the page its Data messages (see below). Only the first `testbed` runs. |
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
//! | `sample` | `line`, `postMs` | The testbed's line of a second, a JSON object in a string, with `cpu_ms` `null`; and `Date.now()` as it's posted. |
//! | `error` | `command`, `message` | A command didn't parse or failed, an echo failed (`command` `echo`), or the testbed reported something (`command` `testbed`). |
//! | `captureOpen` | `file`, `node` | The capture started file `file` of the node named `node`. |
//!
//! A capture's bytes come as binary messages that aren't JSON: `0x01`,
//! then the file's index as a LEB128 varint, then the bytes that follow
//! in that file. The node sends them each time the capture flushes, which
//! is between records, about once a second.
//!
//! A media stream's Data messages come as binary messages for the page,
//! which plays them and doesn't relay them: `0x02`, then a flags byte
//! (bit 0 the message completes a record, bit 1 a discontinuity came
//! before it), then the message's bytes of records.
//!
//! Every vpn message on `echoPort` goes back to its sender's port,
//! unchanged and from `echoPort`, in the order the messages arrived; it
//! isn't reported. Commands run in the order they arrive, and each runs
//! to completion before the next, except `dial` and `connectPeer`, which
//! report when they finish, and `testbed`, which runs on.
//!
//! The testbed measures lag and its timers' lateness on `Date.now()`, read
//! at the moment: [`clock::now`] holds the time a turn of the event loop
//! started.

use std::cell::Cell;
use std::fmt::Write as _;
use std::io;
use std::rc::Rc;
use std::sync::Arc;

use js_sys::{Array, ArrayBuffer, JSON, Object, Reflect, Uint8Array};
use strims_base::time::Time;
use strims_node_core::{
    HostId, NetworkId, NetworkOptions, Node, NodeEvent, NodeEvents, PeerInfo, PpsppCapture,
};
use strims_proto::types::Key;
use strims_testbed::node::{Runtime, Settings, Stream, kind_name};
use strims_testbed::shape::RATE;
use tokio::sync::mpsc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, MessagePort};

use crate::{clock, js, node, runtime};

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
        peers: None,
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

    /// Posts `bytes` as one binary message.
    fn post_bytes(&self, bytes: &[u8]) {
        let buffer = Uint8Array::from(bytes).buffer();
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
    /// The peers added since `start`, until `testbed` takes them.
    peers: Option<mpsc::UnboundedReceiver<PeerInfo>>,
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
                        set("kind", kind_name(l.kind).into());
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
            "testbed" => {
                let peers = self.peers.take().ok_or("the testbed already runs")?;
                let rate = match property(cmd, "rate") {
                    v if v.is_undefined() => RATE,
                    _ => int(cmd, "rate")?,
                };
                let size = |key: &str| usize::try_from(int(cmd, key)?).map_err(|e| e.to_string());
                let chunks = match property(cmd, "chunk") {
                    v if v.is_undefined() => None,
                    _ => Some((size("chunk")?, size("munro")?)),
                };
                let stream = match property(cmd, "units") {
                    v if v.is_undefined() => Stream::Frames,
                    _ => Stream::Units {
                        chunks_per_signature: size("units")?,
                    },
                };
                let settings = Settings {
                    index: size("index")?,
                    nodes: size("nodes")?,
                    start_ms: int(cmd, "startMs")?,
                    secs: int(cmd, "secs")?,
                    rate,
                    chunks,
                    stream,
                };
                let out = self.out.clone();
                // Detached: it ends 5 s after the stream does.
                drop(runtime::spawn(async move {
                    let rt = Testbed(out.clone());
                    if let Err(e) =
                        strims_testbed::node::run(&r.node, peers, settings, None, &rt).await
                    {
                        out.error("testbed", &e);
                    }
                }));
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
        let capture = match property(cmd, "capture") {
            c if c.is_undefined() || c.is_null() => None,
            c => Some((
                int(&c, "maxFileBytes")?,
                property(&c, "tap").as_bool().unwrap_or(true),
                property(&c, "label").as_string().unwrap_or_default(),
            )),
        };
        let signal = self.signal.take().ok_or("already started")?;

        let mut config = node::browser_config(profile_key, rng_seed);
        config.allow_insecure_ws = true;
        config.webrtc.endpoint.ice_servers = Some(Vec::new());
        if let Some(m) = max_buffered {
            config.webrtc.max_buffered_amount = m;
        }
        if let Some((max_file_bytes, tap, label)) = capture {
            let (chunks, rx) = mpsc::unbounded_channel();
            config.label = label;
            config.ppspp_capture = Some(PpsppCapture {
                open: Arc::new(move |node: &[u8], file| {
                    let _ = chunks.send(Chunk::Open {
                        node: node.to_vec(),
                        file,
                    });
                    Ok(Box::new(ChunkWriter {
                        chunks: chunks.clone(),
                        file,
                        buf: Vec::new(),
                    }) as Box<dyn io::Write + Send>)
                }),
                max_file_bytes,
                tap,
            });
            drop(runtime::spawn(post_capture(self.out.clone(), rx)));
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
        let (peers_tx, peers) = mpsc::unbounded_channel();
        self.peers = Some(peers);
        // It runs once this task waits: after `started` is posted.
        drop(runtime::spawn(watch(
            running.clone(),
            events,
            self.out.clone(),
            peers_tx,
        )));
        let id = running.node.host_id();
        tracing::info!(host_id = %id, "e2e node started");
        self.out
            .post(&[("type", "started".into()), ("hostId", host_id(id))]);
        self.running = Some(running);
        Ok(())
    }
}

/// What the capture's files send the harness.
enum Chunk {
    Open { node: Vec<u8>, file: u64 },
    Bytes { file: u64, bytes: Vec<u8> },
}

/// A capture file whose bytes go to the harness at each flush, so each
/// message ends on a record.
struct ChunkWriter {
    chunks: mpsc::UnboundedSender<Chunk>,
    file: u64,
    buf: Vec<u8>,
}

impl io::Write for ChunkWriter {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(b);
        Ok(b.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }
        let bytes = std::mem::take(&mut self.buf);
        self.chunks
            .send(Chunk::Bytes {
                file: self.file,
                bytes,
            })
            .map_err(|_| io::Error::other("the control task ended"))
    }
}

impl Drop for ChunkWriter {
    fn drop(&mut self) {
        let _ = io::Write::flush(self);
    }
}

/// Posts the capture's files to the harness as they come.
async fn post_capture(out: Out, mut chunks: mpsc::UnboundedReceiver<Chunk>) {
    let mut msg = Vec::new();
    while let Some(c) = chunks.recv().await {
        match c {
            Chunk::Open { node, file } => out.post(&[
                ("type", "captureOpen".into()),
                ("file", number(file)),
                ("node", hex(&node).into()),
            ]),
            Chunk::Bytes { file, bytes } => {
                msg.clear();
                msg.push(CAPTURE_BYTES);
                let mut v = file;
                while v >= 0x80 {
                    msg.push(v as u8 | 0x80);
                    v >>= 7;
                }
                msg.push(v as u8);
                msg.extend_from_slice(&bytes);
                out.post_bytes(&msg);
            }
        }
    }
}

/// The first byte of a binary message of capture bytes; a JSON message's
/// is `{`.
pub const CAPTURE_BYTES: u8 = 0x01;
/// The first byte of a binary message of a media stream's Data message,
/// and its flags.
pub const MEDIA_BYTES: u8 = 0x02;
const MEDIA_SEGMENT_END: u8 = 0x01;
const MEDIA_DISCONTINUITY: u8 = 0x02;

/// Reports the node's events, hands added peers to `peers` for the
/// testbed, and echoes the messages on the echo port, until the node
/// stops.
async fn watch(
    r: Running,
    mut events: NodeEvents,
    out: Out,
    peers: mpsc::UnboundedSender<PeerInfo>,
) {
    while let Some(e) = events.recv().await {
        match e {
            NodeEvent::PeerAdded(p) => {
                out.post(&[
                    ("type", "peerAdded".into()),
                    ("hostId", host_id(p.host_id)),
                    ("kind", kind_name(p.kind).into()),
                    ("dialed", p.dialed.into()),
                ]);
                let _ = peers.send(p);
            }
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

/// The testbed's side of the worker: its lines go out as `sample` events,
/// and a media stream's Data messages to the page.
#[derive(Clone)]
struct Testbed(Out);

impl Runtime for Testbed {
    fn now_ms(&self) -> u64 {
        // `as` saturates, and NaN is 0.
        js_sys::Date::now() as u64
    }

    async fn sleep_until_ms(&self, at_ms: u64) {
        clock::Sleep::until(Time::from_unix_millis(at_ms as i64)).await;
    }

    fn cpu_ms(&self) -> Option<u64> {
        None
    }

    fn emit(&self, line: String) {
        self.0.post(&[
            ("type", "sample".into()),
            ("line", line.into()),
            ("postMs", js_sys::Date::now().into()),
        ]);
    }

    fn error(&self, message: String) {
        self.0.error("testbed", &message);
    }

    fn media(&self, data: &[u8], segment_end: bool, discontinuity: bool) {
        let mut msg = Vec::with_capacity(data.len() + 2);
        msg.push(MEDIA_BYTES);
        msg.push(
            if segment_end { MEDIA_SEGMENT_END } else { 0 }
                | if discontinuity {
                    MEDIA_DISCONTINUITY
                } else {
                    0
                },
        );
        msg.extend_from_slice(data);
        self.0.post_bytes(&msg);
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
