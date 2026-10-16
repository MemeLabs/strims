// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The sans-IO RPC endpoint: one side of one transport, as a client, a
//! server or both.
//!
//! Feed received bytes to [`Endpoint::handle_input`], send what
//! [`Endpoint::poll_transmit`] returns, and handle
//! [`Endpoint::poll_event`]s: requests to serve, cancels of requests in
//! service, and responses to the endpoint's own calls, read with
//! [`Endpoint::poll_response`]. There are no timers; callers own deadlines
//! and cancel with [`Endpoint::cancel`].
//!
//! [`Endpoint::poll_transmit`] returns each Call as an owned buffer rather
//! than filling the caller's: a Call must leave whole, as one message on
//! message transports, and it is encoded once, when it is queued.
//!
//! # Remotes
//!
//! A stream transport has one remote, [`RemoteId::STREAM`]. A message
//! transport that reaches many (a vpn port) names each with a [`RemoteId`]
//! and passes decoded Calls to [`Endpoint::handle_call`]: requests are keyed
//! by (remote, caller id), a response is accepted only from the remote its
//! call went to, and [`Endpoint::poll_transmit_to`] says where each Call
//! goes. [`Endpoint::is_remote_idle`] tells the owner when a remote holds
//! nothing, so it can forget the remote's address.

use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use prost::Message;
use strims_proto::rpc::call::Kind;
use strims_proto::rpc::{Call, Close};
use strims_proto::service::{CallError, Error as RpcError};
use strims_proto::types::Certificate;

use crate::frame::{FrameDecoder, FrameError};
use crate::wire::{
    Argument, CALLBACK, CANCEL, Framing, call_len, encode_call_with_headers, encode_headers,
};

/// Endpoint limits. Every queue that the peer can grow is bounded by one
/// of them.
#[derive(Clone, Debug)]
pub struct Config {
    /// How Calls are delimited on the transport.
    pub framing: Framing,
    /// Whether the endpoint serves requests. A client-only endpoint drops
    /// them.
    pub serve: bool,
    /// The longest Call accepted, without its prefix. A longer one is
    /// fatal: the stream can't be resynchronised. The frame decoder holds
    /// at most this plus [`MAX_VARINT_LEN`](crate::wire::MAX_VARINT_LEN)
    /// bytes.
    pub max_message_bytes: usize,
    /// The longest Call sent, without its prefix. A longer request fails
    /// with [`CallError::TooLarge`]; a longer reply is answered with an
    /// error instead. Peers close the transport on Calls over their own
    /// receive limit, so keep this at or under it.
    pub max_send_bytes: usize,
    /// Requests in service at once. More are answered with an error.
    pub max_inbound_calls: usize,
    /// Requests in service at once from one remote. More are answered with
    /// an error, so one remote can't hold every slot.
    pub max_inbound_calls_per_remote: usize,
    /// The endpoint's own open calls. More fail with
    /// [`CallError::TooManyCalls`].
    pub max_outbound_calls: usize,
    /// Response bytes queued for one call until [`Endpoint::poll_response`]
    /// takes them, counting [`RESPONSE_OVERHEAD`] per response. More fail
    /// the call with [`CallError::Overflow`] and cancel it, so a slow
    /// reader loses only its own call. Never below `max_message_bytes` plus
    /// the overhead, so a lone response always fits. All calls together
    /// hold at most `max_outbound_calls` times this.
    pub max_queued_response_bytes: usize,
    /// Queued transmit bytes at which [`Endpoint::is_congested`] holds. The
    /// driver then stops reading input and stops pulling server streams,
    /// so the transmit queue is bounded by this plus one reply per request
    /// in service, one request per open call, and the answers to the input
    /// handled in one [`Endpoint::handle_input`].
    pub transmit_high_water: usize,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            framing: Framing::Delimited,
            serve: true,
            max_message_bytes: 512 * 1024,
            max_send_bytes: 512 * 1024,
            max_inbound_calls: 128,
            max_inbound_calls_per_remote: usize::MAX,
            max_outbound_calls: 128,
            max_queued_response_bytes: 1024 * 1024,
            transmit_high_water: 256 * 1024,
        }
    }
}

impl Config {
    /// The defaults for a link between nodes.
    pub fn peer() -> Config {
        Config::default()
    }

    /// The defaults for the UI's connection to its node, which carries
    /// larger messages (up to 10 MiB).
    pub fn frontend() -> Config {
        Config {
            max_message_bytes: 10 * 1024 * 1024,
            max_send_bytes: 10 * 1024 * 1024,
            max_queued_response_bytes: 10 * 1024 * 1024,
            ..Config::default()
        }
    }
}

/// What a queued response costs beyond its bytes, so that empty responses
/// are bounded too.
pub const RESPONSE_OVERHEAD: usize = 64;

/// The remote end of calls; see the module docs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RemoteId(pub u64);

impl RemoteId {
    /// The single remote of a stream transport.
    pub const STREAM: RemoteId = RemoteId(0);
}

/// One of the endpoint's own calls: the id it was sent with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallId(u64);

impl CallId {
    /// The call's id on the wire.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// A request in service. Ids are the endpoint's own and never reused, so
/// a late reply to a cancelled request can't reach a newer one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InboundId(u64);

/// Something for the endpoint's owner to act on.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A request to serve. Answer it with one of the `reply_*` methods.
    Request {
        /// The request's handle.
        call: InboundId,
        /// Where it came from.
        from: RemoteId,
        /// The caller's id for it.
        id: u64,
        /// The method's full name.
        method: String,
        /// The encoded request message.
        argument: Vec<u8>,
        /// The request's headers.
        headers: BTreeMap<String, Vec<u8>>,
        /// The caller's certificate, if the transport verified one.
        certificate: Option<Rc<Certificate>>,
    },
    /// The caller cancelled a request. Its replies are dropped from now on.
    Cancelled {
        /// The request's handle.
        call: InboundId,
    },
    /// [`Endpoint::poll_response`] has something for this call. Emitted
    /// when a response arrives for a call whose queue was empty.
    Response {
        /// The call.
        call: CallId,
    },
}

/// A response to one of the endpoint's calls.
#[derive(Clone, Debug, PartialEq)]
pub enum Response {
    /// An encoded response message: the result of a unary call, or one
    /// item of a stream.
    Item(Vec<u8>),
    /// The end of a stream.
    End,
    /// The call failed. Nothing follows.
    Failed(CallError),
}

/// Why the endpoint stopped. Every error is fatal: the endpoint closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The input couldn't be split into Calls.
    #[error(transparent)]
    Frame(#[from] FrameError),
    /// A message-framed Call is longer than the limit.
    #[error("message of {len} bytes exceeds the {max}-byte limit")]
    MessageTooLarge {
        /// The message's length.
        len: usize,
        /// The limit.
        max: usize,
    },
    /// The endpoint is closed.
    #[error("endpoint closed")]
    Closed,
}

/// Counters for diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Calls received, including malformed ones.
    pub calls_received: u64,
    /// Calls sent.
    pub calls_sent: u64,
    /// Requests handed to the owner.
    pub requests: u64,
    /// Requests answered with "too many concurrent calls".
    pub requests_rejected: u64,
    /// Received Calls that didn't decode.
    pub malformed: u64,
    /// Received Calls dropped: requests while not serving, requests with id
    /// 0 or a duplicate id, responses to calls that aren't open or from
    /// another remote, cancels of requests that aren't in service, and
    /// Calls handed to a closed endpoint.
    pub dropped: u64,
    /// Calls failed with [`CallError::Overflow`].
    pub overflows: u64,
    /// Replies over `max_send_bytes`, answered with an error instead.
    pub oversized_replies: u64,
}

#[derive(Debug)]
struct Outbound {
    remote: RemoteId,
    streaming: bool,
    /// The request's position in the order of queued Calls.
    request_seq: u64,
    responses: VecDeque<Response>,
    /// The cost of `responses`; see [`Config::max_queued_response_bytes`].
    queued: usize,
    /// A terminal response is queued; nothing more is accepted.
    done: bool,
}

/// What a queued response counts against its call's limit.
fn response_cost(r: &Response) -> usize {
    RESPONSE_OVERHEAD
        + match r {
            Response::Item(b) => b.len(),
            _ => 0,
        }
}

/// What the endpoint holds for one remote.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RemoteRefs {
    /// Requests in service, open calls and queued Calls.
    refs: usize,
    /// Requests in service.
    inbound: usize,
}

/// One side of an RPC transport. See the module docs.
#[derive(Debug)]
pub struct Endpoint {
    config: Config,
    decoder: FrameDecoder,
    /// The id of the next Call sent. Every Call gets a fresh one.
    next_id: u64,
    next_inbound: u64,
    /// Requests in service, by remote and the caller's id. Bounded by
    /// `max_inbound_calls`.
    inbound: BTreeMap<(RemoteId, u64), InboundId>,
    /// The remote and caller's id of each request in service.
    inbound_ids: BTreeMap<InboundId, (RemoteId, u64)>,
    /// Open calls. Bounded by `max_outbound_calls`.
    outbound: BTreeMap<u64, Outbound>,
    /// The cost of the responses queued across `outbound`.
    queued_response_bytes: usize,
    /// Bounded by the requests in service, the open calls, and the input
    /// handled since the owner last drained it.
    events: VecDeque<Event>,
    /// Encoded Calls to send. See [`Config::transmit_high_water`].
    transmit: VecDeque<(RemoteId, Vec<u8>)>,
    transmit_bytes: usize,
    /// Calls queued to `transmit`, ever. `transmit` is FIFO, so a Call is
    /// still queued while fewer than its position have been transmitted.
    queued_calls: u64,
    /// Calls taken from `transmit`, ever.
    transmitted_calls: u64,
    /// Bytes passed to [`Endpoint::handle_input`].
    received_bytes: u64,
    /// One entry per remote that has a request in service, an open call or
    /// a queued Call, so bounded by those.
    remotes: BTreeMap<RemoteId, RemoteRefs>,
    /// The encoded headers of every Call sent; see [`Endpoint::set_headers`].
    headers: Vec<u8>,
    closed: bool,
    metrics: Metrics,
}

impl Endpoint {
    /// An endpoint with `config`'s limits.
    pub fn new(mut config: Config) -> Endpoint {
        config.max_queued_response_bytes = config
            .max_queued_response_bytes
            .max(config.max_message_bytes + RESPONSE_OVERHEAD);
        Endpoint {
            decoder: FrameDecoder::new(config.max_message_bytes),
            config,
            next_id: 1,
            next_inbound: 0,
            inbound: BTreeMap::new(),
            inbound_ids: BTreeMap::new(),
            outbound: BTreeMap::new(),
            queued_response_bytes: 0,
            events: VecDeque::new(),
            transmit: VecDeque::new(),
            transmit_bytes: 0,
            queued_calls: 0,
            transmitted_calls: 0,
            received_bytes: 0,
            remotes: BTreeMap::new(),
            headers: Vec::new(),
            closed: false,
            metrics: Metrics::default(),
        }
    }

    /// The endpoint's limits.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Counters for diagnostics.
    pub fn metrics(&self) -> Metrics {
        self.metrics
    }

    /// Whether the endpoint is closed.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Sets the headers of every Call sent from now on: requests, answers
    /// and cancels. They count towards [`Config::max_send_bytes`].
    pub fn set_headers(&mut self, headers: &BTreeMap<String, Vec<u8>>) {
        self.headers = encode_headers(headers);
    }

    /// Whether `remote` has no request in service, no open call and no
    /// queued Call.
    pub fn is_remote_idle(&self, remote: RemoteId) -> bool {
        !self.remotes.contains_key(&remote)
    }

    fn retain(&mut self, remote: RemoteId) {
        self.remotes.entry(remote).or_default().refs += 1;
    }

    fn release(&mut self, remote: RemoteId) {
        if let Some(r) = self.remotes.get_mut(&remote) {
            r.refs = r.refs.saturating_sub(1);
            if r.refs == 0 {
                self.remotes.remove(&remote);
            }
        }
    }

    fn add_inbound(&mut self, remote: RemoteId, id: u64, handle: InboundId) {
        self.inbound.insert((remote, id), handle);
        self.inbound_ids.insert(handle, (remote, id));
        let r = self.remotes.entry(remote).or_default();
        r.refs += 1;
        r.inbound += 1;
    }

    fn remove_inbound(&mut self, remote: RemoteId, id: u64) -> Option<InboundId> {
        let handle = self.inbound.remove(&(remote, id))?;
        self.inbound_ids.remove(&handle);
        if let Some(r) = self.remotes.get_mut(&remote) {
            r.inbound = r.inbound.saturating_sub(1);
        }
        self.release(remote);
        Some(handle)
    }

    fn remove_outbound(&mut self, id: u64) -> Option<Outbound> {
        let out = self.outbound.remove(&id)?;
        self.release(out.remote);
        Some(out)
    }

    // --- input ------------------------------------------------------------

    /// Handles received bytes: any part of the stream under
    /// [`Framing::Delimited`], exactly one Call under [`Framing::Message`].
    /// Calls that don't decode are skipped. An error closes the endpoint.
    pub fn handle_input(&mut self, data: &[u8]) -> Result<(), Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        self.received_bytes = self.received_bytes.saturating_add(data.len() as u64);
        let res = match self.config.framing {
            Framing::Delimited => {
                let mut decoder = std::mem::replace(&mut self.decoder, FrameDecoder::new(0));
                let res = decoder.decode(data, &mut |frame| self.handle_frame(frame));
                self.decoder = decoder;
                res.map_err(Error::from)
            }
            Framing::Message if data.len() > self.config.max_message_bytes => {
                Err(Error::MessageTooLarge {
                    len: data.len(),
                    max: self.config.max_message_bytes,
                })
            }
            Framing::Message => {
                self.handle_frame(data);
                Ok(())
            }
        };
        if res.is_err() {
            self.close();
        }
        res
    }

    fn handle_frame(&mut self, frame: &[u8]) {
        self.metrics.calls_received += 1;
        let Ok(call) = Call::decode(frame) else {
            self.metrics.malformed += 1;
            return;
        };
        self.dispatch(RemoteId::STREAM, call, None);
    }

    /// Handles a decoded Call from `from`, with the caller's certificate if
    /// the transport verified one. Never closes the endpoint; a closed
    /// endpoint drops the Call.
    pub fn handle_call(
        &mut self,
        from: RemoteId,
        call: Call,
        certificate: Option<Rc<Certificate>>,
    ) {
        self.metrics.calls_received += 1;
        if self.closed {
            self.metrics.dropped += 1;
            return;
        }
        self.dispatch(from, call, certificate);
    }

    fn dispatch(&mut self, from: RemoteId, call: Call, certificate: Option<Rc<Certificate>>) {
        if call.method == CANCEL {
            self.handle_cancel(from, call.parent_id);
        } else if call.parent_id != 0 {
            self.handle_response(from, call);
        } else {
            self.handle_request(from, call, certificate);
        }
    }

    fn handle_cancel(&mut self, from: RemoteId, id: u64) {
        match self.remove_inbound(from, id) {
            Some(call) => self.events.push_back(Event::Cancelled { call }),
            None => self.metrics.dropped += 1,
        }
    }

    fn handle_request(&mut self, from: RemoteId, call: Call, certificate: Option<Rc<Certificate>>) {
        if !self.config.serve || call.id == 0 || self.inbound.contains_key(&(from, call.id)) {
            self.metrics.dropped += 1;
            return;
        }
        let remote_inbound = self.remotes.get(&from).map_or(0, |r| r.inbound);
        if self.inbound.len() >= self.config.max_inbound_calls
            || remote_inbound >= self.config.max_inbound_calls_per_remote
        {
            self.metrics.requests_rejected += 1;
            let e = RpcError::new(0, "too many concurrent calls");
            self.queue_callback(from, call.id, Kind::CallKindError, &e);
            return;
        }
        let handle = InboundId(self.next_inbound);
        self.next_inbound += 1;
        self.add_inbound(from, call.id, handle);
        self.metrics.requests += 1;
        self.events.push_back(Event::Request {
            call: handle,
            from,
            id: call.id,
            method: call.method,
            argument: call.argument,
            headers: call.headers,
            certificate,
        });
    }

    fn handle_response(&mut self, from: RemoteId, call: Call) {
        let Some(out) = self.outbound.get_mut(&call.parent_id) else {
            self.metrics.dropped += 1;
            return;
        };
        if out.done || out.remote != from {
            self.metrics.dropped += 1;
            return;
        }
        let response = match Kind::try_from(call.kind) {
            Ok(Kind::CallKindDefault) => Response::Item(call.argument),
            Ok(Kind::CallKindError) => match RpcError::decode(call.argument.as_slice()) {
                Ok(e) => Response::Failed(CallError::Remote(e)),
                Err(e) => Response::Failed(CallError::Decode(e)),
            },
            Ok(Kind::CallKindClose) if out.streaming => Response::End,
            Ok(Kind::CallKindClose) => Response::Failed(CallError::Closed),
            Ok(Kind::CallKindUndefined) => Response::Failed(CallError::Undefined),
            Ok(Kind::CallKindCancel) | Err(_) => {
                Response::Failed(CallError::UnknownKind(call.kind))
            }
        };
        let id = call.parent_id;
        let is_item = matches!(response, Response::Item(_));
        let terminal = !is_item || !out.streaming;
        let cost = response_cost(&response);
        // terminal responses other than items may take the call one entry
        // past its limit
        let overflow = is_item && out.queued + cost > self.config.max_queued_response_bytes;
        if out.responses.is_empty() {
            self.events.push_back(Event::Response { call: CallId(id) });
        }
        let response = if overflow {
            self.metrics.overflows += 1;
            Response::Failed(CallError::Overflow)
        } else {
            response
        };
        let cost = response_cost(&response);
        out.queued += cost;
        self.queued_response_bytes += cost;
        out.responses.push_back(response);
        out.done = terminal || overflow;
        let remote = out.remote;
        if overflow && !terminal {
            self.queue_cancel(remote, id);
        }
    }

    // --- output -----------------------------------------------------------

    /// The next Call to send: one whole frame, prefixed under
    /// [`Framing::Delimited`].
    pub fn poll_transmit(&mut self) -> Option<Vec<u8>> {
        self.poll_transmit_to().map(|(_, b)| b)
    }

    /// [`Endpoint::poll_transmit`] with the remote the Call goes to.
    pub fn poll_transmit_to(&mut self) -> Option<(RemoteId, Vec<u8>)> {
        let (remote, b) = self.transmit.pop_front()?;
        self.transmit_bytes -= b.len();
        self.transmitted_calls += 1;
        self.release(remote);
        Some((remote, b))
    }

    /// Bytes waiting in [`Endpoint::poll_transmit`].
    pub fn transmit_buffered(&self) -> usize {
        self.transmit_bytes
    }

    /// Whether the transmit queue is at its high water mark; see
    /// [`Config::transmit_high_water`].
    pub fn is_congested(&self) -> bool {
        self.transmit_bytes >= self.config.transmit_high_water
    }

    /// The next event.
    pub fn poll_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        id
    }

    fn queue(&mut self, remote: RemoteId, frame: Vec<u8>) {
        self.metrics.calls_sent += 1;
        self.queued_calls += 1;
        self.transmit_bytes += frame.len();
        self.transmit.push_back((remote, frame));
        self.retain(remote);
    }

    /// The length of a Call with these fields and the endpoint's headers.
    fn send_len(&self, parent_id: u64, method: &str, kind: Kind, arg_len: usize) -> usize {
        call_len(self.next_id, parent_id, method, kind, arg_len) + self.headers.len()
    }

    fn encode(
        &self,
        id: u64,
        parent_id: u64,
        method: &str,
        kind: Kind,
        arg: &impl Argument,
    ) -> Vec<u8> {
        let mut frame = Vec::new();
        encode_call_with_headers(
            &mut frame,
            self.config.framing,
            id,
            parent_id,
            method,
            kind,
            arg,
            &self.headers,
        );
        frame
    }

    fn queue_callback(&mut self, remote: RemoteId, parent_id: u64, kind: Kind, arg: &impl Message) {
        let id = self.next_id();
        let frame = self.encode(id, parent_id, CALLBACK, kind, arg);
        self.queue(remote, frame);
    }

    fn queue_cancel(&mut self, remote: RemoteId, parent_id: u64) {
        let id = self.next_id();
        let frame = self.encode(id, parent_id, CANCEL, Kind::CallKindCancel, &Close {});
        self.queue(remote, frame);
    }

    // --- calls ------------------------------------------------------------

    /// Starts a call of `method`, a server stream if `streaming`, and
    /// queues its request: a message, or bytes already encoded
    /// ([`Encoded`](crate::wire::Encoded)).
    pub fn start_call(
        &mut self,
        method: &str,
        streaming: bool,
        req: &impl Argument,
    ) -> Result<CallId, CallError> {
        self.start_call_to(RemoteId::STREAM, method, streaming, req)
    }

    /// [`Endpoint::start_call`] to `to`.
    pub fn start_call_to(
        &mut self,
        to: RemoteId,
        method: &str,
        streaming: bool,
        req: &impl Argument,
    ) -> Result<CallId, CallError> {
        if self.closed {
            return Err(CallError::Disconnected);
        }
        if self.outbound.len() >= self.config.max_outbound_calls {
            return Err(CallError::TooManyCalls);
        }
        let len = self.send_len(0, method, Kind::CallKindDefault, req.argument_len());
        if len > self.config.max_send_bytes {
            return Err(CallError::TooLarge {
                len,
                max: self.config.max_send_bytes,
            });
        }
        let id = self.next_id();
        let frame = self.encode(id, 0, method, Kind::CallKindDefault, req);
        let request_seq = self.queued_calls;
        self.queue(to, frame);
        self.retain(to);
        self.outbound.insert(
            id,
            Outbound {
                remote: to,
                streaming,
                request_seq,
                responses: VecDeque::new(),
                queued: 0,
                done: false,
            },
        );
        Ok(CallId(id))
    }

    /// Takes the call's next response. A call ends with its last response
    /// (a unary call's [`Response::Item`], [`Response::End`] or
    /// [`Response::Failed`]); after that, and for unknown calls, this
    /// returns `None`.
    pub fn poll_response(&mut self, call: CallId) -> Option<Response> {
        let out = self.outbound.get_mut(&call.0)?;
        let r = out.responses.pop_front()?;
        let cost = response_cost(&r);
        out.queued -= cost;
        self.queued_response_bytes -= cost;
        if out.done && out.responses.is_empty() {
            self.remove_outbound(call.0);
        }
        Some(r)
    }

    /// Ends a call early: drops its queued responses and, unless its last
    /// response already arrived, tells the server.
    pub fn cancel(&mut self, call: CallId) {
        let Some(out) = self.remove_outbound(call.0) else {
            return;
        };
        self.queued_response_bytes -= out.queued;
        if !out.done && !self.closed {
            self.queue_cancel(out.remote, call.0);
        }
    }

    /// Whether the call's request is still waiting in
    /// [`Endpoint::poll_transmit`]. False once the call has ended or the
    /// endpoint has closed.
    pub fn is_request_queued(&self, call: CallId) -> bool {
        !self.closed
            && self
                .outbound
                .get(&call.0)
                .is_some_and(|out| self.transmitted_calls <= out.request_seq)
    }

    /// Bytes received so far: the total passed to
    /// [`Endpoint::handle_input`].
    pub fn received_bytes(&self) -> u64 {
        self.received_bytes
    }

    /// Whether part of a Call has arrived and the rest hasn't. Only a
    /// [`Framing::Delimited`] stream splits Calls across inputs.
    pub fn is_receiving(&self) -> bool {
        self.decoder.buffered() > 0
    }

    // --- replies ----------------------------------------------------------

    fn reply(&mut self, call: InboundId, kind: Kind, arg: &impl Message, last: bool) -> bool {
        let Some(&(remote, parent_id)) = self.inbound_ids.get(&call) else {
            return false;
        };
        let len = self.send_len(parent_id, CALLBACK, kind, arg.encoded_len());
        let fits = len <= self.config.max_send_bytes;
        if fits {
            self.queue_callback(remote, parent_id, kind, arg);
        } else {
            self.metrics.oversized_replies += 1;
            let e = RpcError::new(0, format!("response too large: {len} bytes"));
            self.queue_callback(remote, parent_id, Kind::CallKindError, &e);
        }
        if last || !fits {
            self.remove_inbound(remote, parent_id);
        }
        fits
    }

    /// Answers a unary request. Returns false if the request is no longer
    /// in service (cancelled, answered or closed), or if the value was too
    /// large and the caller got an error instead.
    pub fn reply_value(&mut self, call: InboundId, value: &impl Message) -> bool {
        self.reply(call, Kind::CallKindDefault, value, true)
    }

    /// Sends one item of a server stream. Returns false if the request is
    /// no longer in service, or if the item was too large, which ends the
    /// stream with an error.
    pub fn reply_item(&mut self, call: InboundId, item: &impl Message) -> bool {
        self.reply(call, Kind::CallKindDefault, item, false)
    }

    /// Ends a server stream.
    pub fn reply_close(&mut self, call: InboundId) -> bool {
        self.reply(call, Kind::CallKindClose, &Close {}, true)
    }

    /// Answers a request with an error, ending it.
    pub fn reply_error(&mut self, call: InboundId, error: &RpcError) -> bool {
        self.reply(call, Kind::CallKindError, error, true)
    }

    /// Whether the request is still in service.
    pub fn is_serving(&self, call: InboundId) -> bool {
        self.inbound_ids.contains_key(&call)
    }

    // --- lifecycle --------------------------------------------------------

    /// Closes the endpoint: open calls fail with
    /// [`CallError::Disconnected`], requests in service are cancelled, and
    /// nothing more is sent.
    pub fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        self.transmit.clear();
        self.transmit_bytes = 0;
        for (id, out) in &mut self.outbound {
            if !out.done {
                if out.responses.is_empty() {
                    self.events.push_back(Event::Response { call: CallId(*id) });
                }
                let r = Response::Failed(CallError::Disconnected);
                out.queued += response_cost(&r);
                self.queued_response_bytes += response_cost(&r);
                out.responses.push_back(r);
                out.done = true;
            }
        }
        for call in std::mem::take(&mut self.inbound).into_values() {
            self.events.push_back(Event::Cancelled { call });
        }
        self.inbound_ids.clear();
        // only the open calls remain, until their responses are taken
        self.remotes.clear();
        for out in self.outbound.values() {
            self.remotes.entry(out.remote).or_default().refs += 1;
        }
    }
}

impl Endpoint {
    /// The cost of the responses waiting in [`Endpoint::poll_response`],
    /// counted as for [`Config::max_queued_response_bytes`].
    pub fn queued_response_bytes(&self) -> usize {
        self.queued_response_bytes
    }
}

#[cfg(test)]
mod tests;
