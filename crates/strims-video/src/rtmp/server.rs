// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The server side of an RTMP publish.
//!
//! The driver loop: pass received bytes to
//! [`handle_input`](RtmpServer::handle_input), which returns how many it
//! consumed; drain [`poll_event`](RtmpServer::poll_event) until `None`;
//! call `handle_input` again with the bytes it left; write out
//! [`poll_transmit`](RtmpServer::poll_transmit). A call stops early only
//! when an event is ready, so each call consumes input or yields an event.
//! Answer [`RtmpEvent::Publish`] with
//! [`accept_publish`](RtmpServer::accept_publish) or
//! [`refuse`](RtmpServer::refuse). Once [`phase`](RtmpServer::phase) is
//! [`Phase::Closed`], flush the output and close the connection.

use strims_base::rng::{self, DetRng};

use super::RtmpError;
use super::chunk::{
    ACKNOWLEDGEMENT, AGGREGATE, AUDIO, COMMAND_AMF0, ChunkReader, ChunkWriter, Control, DATA_AMF0,
    Limits, MAX_CHUNK_SIZE, Message, Outcome, PROTOCOL_CSID, SET_CHUNK_SIZE, SET_PEER_BANDWIDTH,
    VIDEO, WINDOW_ACK_SIZE,
};
use super::handshake::ServerHandshake;
use crate::amf0::{self, Decoder, Metadata, Value};

/// The chunk stream of connection-level command replies.
const COMMAND_CSID: u32 = 3;
/// The chunk stream of stream status messages.
const STATUS_CSID: u32 = 5;
/// The message stream id `createStream` returns.
const STREAM_ID: f64 = 1.0;
/// Set Peer Bandwidth's dynamic limit type.
const LIMIT_DYNAMIC: u8 = 2;
/// The `fmsVer` and `capabilities` a connect reply declares.
const FMS_VERSION: &str = "FMS/3,0,1,123";
const CAPABILITIES: f64 = 31.0;
/// An aggregate's FLV tag header: type, length (3), timestamp (3 + 1),
/// stream id (3). A 4-byte back pointer follows each tag's data.
const TAG_HEADER_LEN: usize = 11;
const TAG_TRAILER_LEN: usize = 4;

/// The bounds of a connection and what the server announces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtmpConfig {
    /// The largest audio, video or aggregate message.
    pub max_media_message: usize,
    /// The largest message of another type: commands, data and control.
    pub max_command_message: usize,
    /// The bytes the reassembly buffers of all chunk streams may hold
    /// together; at least `max_media_message`.
    pub max_partial_bytes: usize,
    /// The most chunk streams a peer may use.
    pub max_chunk_streams: usize,
    /// The most output queued for `poll_transmit`; a reply that would
    /// exceed it fails the connection.
    pub max_output: usize,
    /// The chunk size the server announces after `connect` and writes at.
    pub chunk_size: u32,
    /// The Window Acknowledgement Size the server announces.
    pub window_ack_size: u32,
    /// The Set Peer Bandwidth the server announces.
    pub peer_bandwidth: u32,
}

impl Default for RtmpConfig {
    fn default() -> Self {
        RtmpConfig {
            max_media_message: 4 << 20,
            max_command_message: 64 << 10,
            max_partial_bytes: 8 << 20,
            max_chunk_streams: 64,
            max_output: 64 << 10,
            chunk_size: 4096,
            window_ack_size: 2_500_000,
            peer_bandwidth: 2_500_000,
        }
    }
}

impl RtmpConfig {
    pub(crate) fn limits(&self) -> Limits {
        Limits {
            max_media: self.max_media_message,
            max_command: self.max_command_message,
            max_partial: self.max_partial_bytes,
            max_streams: self.max_chunk_streams,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Before C2.
    Handshake,
    /// Before `connect`, or connected without a publish.
    Connecting,
    /// A publish awaits [`RtmpServer::accept_publish`] or
    /// [`RtmpServer::refuse`].
    PublishPending,
    /// Audio, video and metadata flow.
    Publishing,
    /// Refused or failed: input is discarded.
    Closed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RtmpEvent<'a> {
    /// A publish: the connect's app and the stream name up to any `?`.
    Publish { app: String, name: String },
    /// The stream's `onMetaData`.
    Metadata(Metadata),
    /// An FLV video tag body.
    Video { timestamp: u32, data: &'a [u8] },
    /// An FLV audio tag body.
    Audio { timestamp: u32, data: &'a [u8] },
    /// `deleteStream`, `FCUnpublish` or `closeStream` ended the publish.
    Unpublish,
}

enum Pending {
    Publish {
        app: String,
        name: String,
    },
    Metadata(Metadata),
    Unpublish,
    /// A range of the delivered message's body.
    Media {
        video: bool,
        timestamp: u32,
        start: usize,
        end: usize,
    },
}

/// An aggregate message being split into its tags.
struct Aggregate {
    /// The next tag's offset in the message body.
    pos: usize,
    /// The message's timestamp, which the first tag's time maps to.
    timestamp: u32,
    first: Option<u32>,
}

pub struct RtmpServer {
    reader: ChunkReader,
    session: Session,
}

/// Everything but the reader, so a delivered message can be read while
/// the session changes.
struct Session {
    config: RtmpConfig,
    rng: DetRng,
    phase: Phase,
    handshake: ServerHandshake,
    writer: ChunkWriter,
    connected: bool,
    app: Option<String>,
    tc_url: Option<String>,
    /// The message stream of the publish.
    stream_id: u32,
    pending: Option<Pending>,
    aggregate: Option<Aggregate>,
    /// A failure outside `handle_input`, returned by its next call.
    error: Option<RtmpError>,
}

fn amf(context: &'static str) -> impl Fn(amf0::Amf0Error) -> RtmpError {
    move |error| RtmpError::Amf { context, error }
}

/// The next command value; undefined past the end.
fn next<'a>(d: &mut Decoder<'a>) -> Result<Value<'a>, RtmpError> {
    if d.is_empty() {
        return Ok(Value::Undefined);
    }
    d.decode().map_err(amf("command"))
}

fn object<'a>(props: &[(&'a str, &'a str)]) -> Value<'a> {
    Value::Object(props.iter().map(|&(k, v)| (k, Value::String(v))).collect())
}

impl RtmpServer {
    pub fn new(config: &RtmpConfig, rng: &mut DetRng) -> Self {
        RtmpServer {
            reader: ChunkReader::new(config.limits()),
            session: Session {
                config: config.clone(),
                rng: rng::fork(rng),
                phase: Phase::Handshake,
                handshake: ServerHandshake::new(),
                writer: ChunkWriter::new(config.max_output, false),
                connected: false,
                app: None,
                tc_url: None,
                stream_id: 0,
                pending: None,
                aggregate: None,
                error: None,
            },
        }
    }

    /// Reads `input` until it ends or a message yields an event; returns
    /// the bytes consumed. Returns 0 while an event awaits `poll_event`.
    /// An error fails the connection: the phase becomes
    /// [`Phase::Closed`].
    pub fn handle_input(&mut self, input: &[u8]) -> Result<usize, RtmpError> {
        if self.session.pending.is_some() {
            return Ok(0);
        }
        let result = self.run(input);
        if result.is_err() {
            self.session.close();
        }
        result
    }

    /// The event the last `handle_input` stopped at. Its data borrows the
    /// server until the next `handle_input`.
    pub fn poll_event(&mut self) -> Option<RtmpEvent<'_>> {
        Some(match self.session.pending.take()? {
            Pending::Publish { app, name } => RtmpEvent::Publish { app, name },
            Pending::Metadata(m) => RtmpEvent::Metadata(m),
            Pending::Unpublish => RtmpEvent::Unpublish,
            Pending::Media {
                video,
                timestamp,
                start,
                end,
            } => {
                let data = self.reader.message()?.body.get(start..end)?;
                if video {
                    RtmpEvent::Video { timestamp, data }
                } else {
                    RtmpEvent::Audio { timestamp, data }
                }
            }
        })
    }

    /// Moves queued output into `buf`; returns how many bytes.
    pub fn poll_transmit(&mut self, buf: &mut [u8]) -> usize {
        self.session.writer.poll_transmit(buf)
    }

    /// Starts the pending publish: `onStatus NetStream.Publish.Start`.
    pub fn accept_publish(&mut self) {
        let s = &mut self.session;
        if s.phase != Phase::PublishPending {
            return;
        }
        s.phase = Phase::Publishing;
        if let Err(e) = s.send_status("status", "NetStream.Publish.Start", "Publishing.") {
            s.close();
            s.error = Some(e);
        }
    }

    /// Refuses the publish, or ends the connection in another phase: an
    /// `onStatus` at level error with `code` (such as
    /// `NetStream.Publish.BadName` or `NetStream.Publish.Failed`), then
    /// [`Phase::Closed`].
    pub fn refuse(&mut self, code: &'static str, description: &str) {
        let s = &mut self.session;
        if matches!(
            s.phase,
            Phase::Connecting | Phase::PublishPending | Phase::Publishing
        ) && let Err(e) = s.send_status("error", code, description)
        {
            s.error = Some(e);
        }
        s.close();
    }

    pub fn phase(&self) -> Phase {
        self.session.phase
    }

    /// The connect's app.
    pub fn app(&self) -> Option<&str> {
        self.session.app.as_deref()
    }

    /// The connect's tcUrl.
    pub fn tc_url(&self) -> Option<&str> {
        self.session.tc_url.as_deref()
    }

    /// Whether the handshake took the digest scheme.
    pub fn digest_handshake(&self) -> bool {
        self.session.handshake.digest()
    }

    fn run(&mut self, input: &[u8]) -> Result<usize, RtmpError> {
        if let Some(e) = self.session.error.take() {
            return Err(e);
        }
        let mut pos = 0;
        loop {
            let rest = input.get(pos..).unwrap_or_default();
            match self.session.phase {
                Phase::Closed => return Ok(input.len()),
                Phase::Handshake => {
                    let s = &mut self.session;
                    let (n, reply) = s.handshake.read(rest, &mut s.rng)?;
                    pos += n;
                    if let Some(reply) = reply {
                        s.writer.push_raw(&reply)?;
                    }
                    if !s.handshake.is_done() {
                        return Ok(pos);
                    }
                    s.phase = Phase::Connecting;
                }
                _ if self.session.aggregate.is_some() => {
                    let body = self.reader.message().map_or(&[][..], |m| m.body);
                    if self.session.next_tag(body)? {
                        return Ok(pos);
                    }
                }
                _ => {
                    let (n, outcome) = self.reader.read(rest)?;
                    pos += n;
                    match outcome {
                        Outcome::None => return Ok(pos),
                        Outcome::Ack(seq) => self.session.writer.write(
                            PROTOCOL_CSID,
                            0,
                            ACKNOWLEDGEMENT,
                            0,
                            &seq.to_be_bytes(),
                        )?,
                        Outcome::Message => {
                            if self.process()? {
                                return Ok(pos);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Acts on the delivered message; returns whether it yields an event.
    fn process(&mut self) -> Result<bool, RtmpError> {
        let Some(msg) = self.reader.message() else {
            return Ok(false);
        };
        if let Some(control) = Control::parse(msg.type_id, msg.body)? {
            control.apply(&mut self.reader, &mut self.session.writer)?;
            return Ok(false);
        }
        self.session.message(msg)
    }
}

impl Session {
    fn close(&mut self) {
        self.phase = Phase::Closed;
        self.pending = None;
        self.aggregate = None;
    }

    fn message(&mut self, msg: Message<'_>) -> Result<bool, RtmpError> {
        match msg.type_id {
            AUDIO | VIDEO => Ok(self.media(msg.type_id == VIDEO, msg.timestamp, 0, msg.body.len())),
            AGGREGATE => {
                if self.phase == Phase::Publishing {
                    self.aggregate = Some(Aggregate {
                        pos: 0,
                        timestamp: msg.timestamp,
                        first: None,
                    });
                }
                Ok(false)
            }
            DATA_AMF0 => self.data(msg.body),
            COMMAND_AMF0 => self.command(msg.stream_id, msg.body),
            _ => Ok(false),
        }
    }

    fn media(&mut self, video: bool, timestamp: u32, start: usize, end: usize) -> bool {
        if self.phase != Phase::Publishing {
            return false;
        }
        self.pending = Some(Pending::Media {
            video,
            timestamp,
            start,
            end,
        });
        true
    }

    /// Splits the next tags off the aggregate in `body` until one yields
    /// an event.
    fn next_tag(&mut self, body: &[u8]) -> Result<bool, RtmpError> {
        loop {
            let Some(agg) = &mut self.aggregate else {
                return Ok(false);
            };
            let rest = body.get(agg.pos..).unwrap_or_default();
            if rest.is_empty() {
                self.aggregate = None;
                return Ok(false);
            }
            let Some(&[type_id, l0, l1, l2, t0, t1, t2, t3, ..]) = rest.get(..TAG_HEADER_LEN)
            else {
                return Err(RtmpError::Aggregate);
            };
            let len = usize::try_from(u32::from_be_bytes([0, l0, l1, l2])).unwrap_or(usize::MAX);
            let time = u32::from_be_bytes([t3, t0, t1, t2]);
            let start = agg.pos + TAG_HEADER_LEN;
            let end = start.saturating_add(len);
            if end.saturating_add(TAG_TRAILER_LEN) > body.len() {
                return Err(RtmpError::Aggregate);
            }
            agg.pos = end + TAG_TRAILER_LEN;
            let first = *agg.first.get_or_insert(time);
            let timestamp = agg.timestamp.wrapping_add(time.wrapping_sub(first));
            let event = match type_id {
                AUDIO | VIDEO => self.media(type_id == VIDEO, timestamp, start, end),
                DATA_AMF0 => {
                    if len > self.config.max_command_message {
                        return Err(RtmpError::MessageTooLarge {
                            type_id,
                            len: u32::try_from(len).unwrap_or(u32::MAX),
                            max: self.config.max_command_message,
                        });
                    }
                    self.data(body.get(start..end).unwrap_or_default())?
                }
                _ => false,
            };
            if event {
                return Ok(true);
            }
        }
    }

    fn data(&mut self, body: &[u8]) -> Result<bool, RtmpError> {
        if self.phase != Phase::Publishing {
            return Ok(false);
        }
        let mut d = Decoder::new(body);
        let mut handler = d.decode().map_err(amf("data message"))?;
        if handler.as_str() == Some("@setDataFrame") {
            handler = d.decode().map_err(amf("data message"))?;
        }
        if handler.as_str() != Some("onMetaData") {
            return Ok(false);
        }
        let value = d.decode().map_err(amf("data message"))?;
        let Some(metadata) = Metadata::from_value(&value) else {
            return Ok(false);
        };
        self.pending = Some(Pending::Metadata(metadata));
        Ok(true)
    }

    fn require_connected(&self, command: &'static str) -> Result<(), RtmpError> {
        if self.connected {
            Ok(())
        } else {
            Err(RtmpError::NotConnected(command))
        }
    }

    fn command(&mut self, stream_id: u32, body: &[u8]) -> Result<bool, RtmpError> {
        let mut d = Decoder::new(body);
        let name = d.decode().map_err(amf("command"))?;
        let name = name.as_str().ok_or(RtmpError::CommandName)?;
        match name {
            "connect" => {
                if self.connected {
                    return Err(RtmpError::Unexpected("second connect"));
                }
                let txid = next(&mut d)?.as_number().unwrap_or(0.0);
                let properties = next(&mut d)?;
                let app = properties
                    .get("app")
                    .and_then(Value::as_str)
                    .ok_or(RtmpError::ConnectApp)?;
                let tc_url = properties
                    .get("tcUrl")
                    .or_else(|| properties.get("tcurl"))
                    .and_then(Value::as_str);
                self.app = Some(app.to_owned());
                self.tc_url = tc_url.map(str::to_owned);
                self.connected = true;
                self.connect_reply(txid)?;
                Ok(false)
            }
            "releaseStream" | "FCPublish" => {
                let fc_publish = name == "FCPublish";
                self.require_connected(if fc_publish {
                    "FCPublish"
                } else {
                    "releaseStream"
                })?;
                let txid = next(&mut d)?.as_number().unwrap_or(0.0);
                self.send(
                    COMMAND_CSID,
                    0,
                    &[Value::String("_result"), Value::Number(txid), Value::Null],
                )?;
                if fc_publish {
                    self.send(
                        COMMAND_CSID,
                        0,
                        &[
                            Value::String("onFCPublish"),
                            Value::Number(0.0),
                            Value::Null,
                            object(&[
                                ("code", "NetStream.Publish.Start"),
                                ("description", "FCPublish"),
                            ]),
                        ],
                    )?;
                }
                Ok(false)
            }
            "createStream" => {
                self.require_connected("createStream")?;
                let txid = next(&mut d)?.as_number().unwrap_or(0.0);
                self.send(
                    COMMAND_CSID,
                    0,
                    &[
                        Value::String("_result"),
                        Value::Number(txid),
                        Value::Null,
                        Value::Number(STREAM_ID),
                    ],
                )?;
                Ok(false)
            }
            "publish" => {
                self.require_connected("publish")?;
                if self.phase != Phase::Connecting {
                    return Err(RtmpError::Unexpected("publish while publishing"));
                }
                next(&mut d)?;
                next(&mut d)?;
                let name = next(&mut d)?;
                let name = name.as_str().ok_or(RtmpError::PublishName)?;
                let name = name.split_once('?').map_or(name, |(n, _)| n);
                self.stream_id = stream_id;
                self.phase = Phase::PublishPending;
                self.pending = Some(Pending::Publish {
                    app: self.app.clone().unwrap_or_default(),
                    name: name.to_owned(),
                });
                Ok(true)
            }
            "deleteStream" | "FCUnpublish" | "closeStream" => {
                if !matches!(self.phase, Phase::PublishPending | Phase::Publishing) {
                    return Ok(false);
                }
                self.phase = Phase::Connecting;
                self.pending = Some(Pending::Unpublish);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Window Acknowledgement Size, Set Peer Bandwidth, Set Chunk Size,
    /// then `_result` NetConnection.Connect.Success.
    fn connect_reply(&mut self, txid: f64) -> Result<(), RtmpError> {
        let window = self.config.window_ack_size;
        let [a, b, c, d] = self.config.peer_bandwidth.to_be_bytes();
        let chunk_size = self.config.chunk_size.clamp(1, MAX_CHUNK_SIZE);
        self.writer
            .write(PROTOCOL_CSID, 0, WINDOW_ACK_SIZE, 0, &window.to_be_bytes())?;
        self.writer.write(
            PROTOCOL_CSID,
            0,
            SET_PEER_BANDWIDTH,
            0,
            &[a, b, c, d, LIMIT_DYNAMIC],
        )?;
        self.writer.write(
            PROTOCOL_CSID,
            0,
            SET_CHUNK_SIZE,
            0,
            &chunk_size.to_be_bytes(),
        )?;
        self.writer.set_chunk_size(chunk_size);
        self.send(
            COMMAND_CSID,
            0,
            &[
                Value::String("_result"),
                Value::Number(txid),
                Value::Object(vec![
                    ("fmsVer", Value::String(FMS_VERSION)),
                    ("capabilities", Value::Number(CAPABILITIES)),
                ]),
                Value::Object(vec![
                    ("level", Value::String("status")),
                    ("code", Value::String("NetConnection.Connect.Success")),
                    ("description", Value::String("Connection succeeded.")),
                    ("objectEncoding", Value::Number(0.0)),
                ]),
            ],
        )
    }

    fn send_status(&mut self, level: &str, code: &str, description: &str) -> Result<(), RtmpError> {
        self.send(
            STATUS_CSID,
            self.stream_id,
            &[
                Value::String("onStatus"),
                Value::Number(0.0),
                Value::Null,
                object(&[
                    ("level", level),
                    ("code", code),
                    ("description", description),
                ]),
            ],
        )
    }

    fn send(&mut self, csid: u32, stream_id: u32, values: &[Value<'_>]) -> Result<(), RtmpError> {
        let mut body = Vec::new();
        for v in values {
            amf0::encode(v, &mut body).map_err(amf("reply"))?;
        }
        self.writer.write(csid, 0, COMMAND_AMF0, stream_id, &body)
    }
}

#[cfg(test)]
#[path = "server_test.rs"]
mod tests;
