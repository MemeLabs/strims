// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A sans-IO RTMP publisher client, the counterpart of
//! [`RtmpServer`](crate::rtmp::RtmpServer) in tests.
//!
//! [`Publisher::new`] queues C0 C1. As the server's replies arrive
//! through [`handle_input`](Publisher::handle_input), it sends C2, its
//! chunk size, `connect`; after the connect `_result`, `releaseStream`,
//! `FCPublish` and `createStream`; after the createStream `_result`,
//! `publish`. The `onStatus` that answers the publish sets
//! [`publish_accepted`](Publisher::publish_accepted) or
//! [`refused`](Publisher::refused). The driver then sends metadata, video
//! and audio messages and drains [`poll_transmit`](Publisher::poll_transmit).
//!
//! Messages after the first on a chunk stream take the shortest chunk
//! format their header allows, so a run of tags exercises formats 0-3.
//! The publisher acknowledges the server's window and answers pings.

use strims_base::rng::{self, DetRng};

use crate::amf0::{self, Value};
use crate::rtmp::chunk::{
    ACKNOWLEDGEMENT, AUDIO, COMMAND_AMF0, ChunkReader, ChunkWriter, Control, DATA_AMF0,
    DEFAULT_CHUNK_SIZE as INITIAL_CHUNK_SIZE, Outcome, PROTOCOL_CSID, SET_CHUNK_SIZE, VIDEO,
};
use crate::rtmp::handshake::ClientHandshake;
use crate::rtmp::{RtmpConfig, RtmpError};

/// The chunk size [`PublisherConfig::new`] sets.
pub const DEFAULT_CHUNK_SIZE: u32 = 4096;
/// The most output the publisher queues; a send past it fails.
pub const MAX_OUTPUT: usize = 64 << 20;

const COMMAND_CSID: u32 = 3;
const AUDIO_CSID: u32 = 4;
const DATA_CSID: u32 = 5;
const VIDEO_CSID: u32 = 6;

const CONNECT_TXID: f64 = 1.0;
const RELEASE_STREAM_TXID: f64 = 2.0;
const FC_PUBLISH_TXID: f64 = 3.0;
const CREATE_STREAM_TXID: f64 = 4.0;
const PUBLISH_TXID: f64 = 5.0;
const FC_UNPUBLISH_TXID: f64 = 6.0;
const DELETE_STREAM_TXID: f64 = 7.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublisherConfig {
    /// The connect's app.
    pub app: String,
    /// The connect's tcUrl.
    pub tc_url: String,
    /// The publish's stream name, with any query.
    pub name: String,
    /// Whether C1 carries a Flash Player digest.
    pub digest_handshake: bool,
    /// The chunk size announced after the handshake and written at.
    pub chunk_size: u32,
}

impl PublisherConfig {
    /// A digest-handshake publish of `name` to app `app`, chunked at
    /// [`DEFAULT_CHUNK_SIZE`].
    pub fn new(app: &str, name: &str) -> Self {
        PublisherConfig {
            app: app.to_owned(),
            tc_url: format!("rtmp://localhost/{app}"),
            name: name.to_owned(),
            digest_handshake: true,
            chunk_size: DEFAULT_CHUNK_SIZE,
        }
    }
}

/// The fields of a server command the publisher reads.
struct Command {
    name: String,
    txid: f64,
    /// The first argument after the command object, if a number.
    number: Option<f64>,
    level: String,
    code: String,
    description: String,
}

pub struct Publisher {
    config: PublisherConfig,
    rng: DetRng,
    handshake: ClientHandshake,
    reader: ChunkReader,
    writer: ChunkWriter,
    stream_id: u32,
    connected: bool,
    accepted: bool,
    refused: Option<(String, String)>,
}

impl Publisher {
    /// A publisher with C0 C1 queued.
    pub fn new(config: PublisherConfig, rng: &mut DetRng) -> Self {
        let mut rng = rng::fork(rng);
        let (handshake, hello) = ClientHandshake::new(config.digest_handshake, &mut rng);
        let mut writer = ChunkWriter::new(MAX_OUTPUT, true);
        // The queue is empty and far larger than C0 C1.
        let _ = writer.push_raw(&hello);
        Publisher {
            config,
            rng,
            handshake,
            reader: ChunkReader::new(RtmpConfig::default().limits()),
            writer,
            stream_id: 0,
            connected: false,
            accepted: false,
            refused: None,
        }
    }

    /// Reads the server's bytes, queueing the replies they call for.
    pub fn handle_input(&mut self, input: &[u8]) -> Result<(), RtmpError> {
        let mut pos = 0;
        if !self.handshake.is_done() {
            let (n, c2) = self.handshake.read(input, &mut self.rng)?;
            pos = n;
            let Some(c2) = c2 else {
                return Ok(());
            };
            self.writer.push_raw(&c2)?;
            self.start()?;
        }
        loop {
            let (n, outcome) = self.reader.read(input.get(pos..).unwrap_or_default())?;
            pos += n;
            match outcome {
                Outcome::None => return Ok(()),
                Outcome::Ack(seq) => {
                    self.writer
                        .write(PROTOCOL_CSID, 0, ACKNOWLEDGEMENT, 0, &seq.to_be_bytes())?;
                }
                Outcome::Message => self.process()?,
            }
        }
    }

    /// Moves queued output into `buf`; returns how many bytes.
    pub fn poll_transmit(&mut self, buf: &mut [u8]) -> usize {
        self.writer.poll_transmit(buf)
    }

    /// Bytes queued for [`poll_transmit`](Self::poll_transmit).
    pub fn queued(&self) -> usize {
        self.writer.queued()
    }

    /// Sends `@setDataFrame onMetaData` with `fields` as an ECMA array.
    pub fn send_metadata(&mut self, fields: &[(&str, Value<'_>)]) -> Result<(), RtmpError> {
        let values = [
            Value::String("@setDataFrame"),
            Value::String("onMetaData"),
            Value::EcmaArray(fields.to_vec()),
        ];
        let body = encode(&values)?;
        self.writer
            .write(DATA_CSID, 0, DATA_AMF0, self.stream_id, &body)
    }

    /// Sends `onTextData` of `text` as FLV muxers write a subtitle: an
    /// ECMA array of its `type`, `Text`, and its `text`.
    pub fn send_text(&mut self, timestamp: u32, text: &str) -> Result<(), RtmpError> {
        let values = [
            Value::String("onTextData"),
            Value::EcmaArray(vec![
                ("type", Value::String("Text")),
                ("text", Value::String(text)),
            ]),
        ];
        let body = encode(&values)?;
        self.writer
            .write(DATA_CSID, timestamp, DATA_AMF0, self.stream_id, &body)
    }

    /// Sends an FLV video tag body.
    pub fn send_video(&mut self, timestamp: u32, body: &[u8]) -> Result<(), RtmpError> {
        self.writer
            .write(VIDEO_CSID, timestamp, VIDEO, self.stream_id, body)
    }

    /// Sends an FLV audio tag body.
    pub fn send_audio(&mut self, timestamp: u32, body: &[u8]) -> Result<(), RtmpError> {
        self.writer
            .write(AUDIO_CSID, timestamp, AUDIO, self.stream_id, body)
    }

    /// Sends any message.
    pub fn send_message(
        &mut self,
        csid: u32,
        timestamp: u32,
        type_id: u8,
        stream_id: u32,
        body: &[u8],
    ) -> Result<(), RtmpError> {
        self.writer.write(csid, timestamp, type_id, stream_id, body)
    }

    /// Ends the publish: `FCUnpublish` and `deleteStream`.
    pub fn unpublish(&mut self) -> Result<(), RtmpError> {
        let name = self.config.name.clone();
        self.command(
            0,
            &[
                Value::String("FCUnpublish"),
                Value::Number(FC_UNPUBLISH_TXID),
                Value::Null,
                Value::String(&name),
            ],
        )?;
        self.command(
            0,
            &[
                Value::String("deleteStream"),
                Value::Number(DELETE_STREAM_TXID),
                Value::Null,
                Value::Number(f64::from(self.stream_id)),
            ],
        )
    }

    pub fn handshake_done(&self) -> bool {
        self.handshake.is_done()
    }

    /// Whether the server's S1 and S2 carried valid digests; `None`
    /// before S2 or after a plain C1.
    pub fn server_digest(&self) -> Option<bool> {
        self.handshake.server_digest()
    }

    /// Whether the server answered `connect` with `_result`.
    pub fn connected(&self) -> bool {
        self.connected
    }

    /// Whether the server answered `publish` with NetStream.Publish.Start.
    pub fn publish_accepted(&self) -> bool {
        self.accepted
    }

    /// The code and description of an error status or `_error`.
    pub fn refused(&self) -> Option<(&str, &str)> {
        self.refused
            .as_ref()
            .map(|(code, description)| (code.as_str(), description.as_str()))
    }

    /// The message stream `createStream` returned; 0 before.
    pub fn stream_id(&self) -> u32 {
        self.stream_id
    }

    /// The chunk size, then `connect`.
    fn start(&mut self) -> Result<(), RtmpError> {
        let size = self.config.chunk_size;
        if size != INITIAL_CHUNK_SIZE {
            self.writer
                .write(PROTOCOL_CSID, 0, SET_CHUNK_SIZE, 0, &size.to_be_bytes())?;
            self.writer.set_chunk_size(size);
        }
        let (app, tc_url) = (self.config.app.clone(), self.config.tc_url.clone());
        self.command(
            0,
            &[
                Value::String("connect"),
                Value::Number(CONNECT_TXID),
                Value::Object(vec![
                    ("app", Value::String(&app)),
                    ("type", Value::String("nonprivate")),
                    ("flashVer", Value::String("FMLE/3.0 (compatible; FMSc/1.0)")),
                    ("tcUrl", Value::String(&tc_url)),
                ]),
            ],
        )
    }

    fn process(&mut self) -> Result<(), RtmpError> {
        let Some(msg) = self.reader.message() else {
            return Ok(());
        };
        if let Some(control) = Control::parse(msg.type_id, msg.body)? {
            return control.apply(&mut self.reader, &mut self.writer);
        }
        if msg.type_id != COMMAND_AMF0 {
            return Ok(());
        }
        let values = amf0::decode_all(msg.body).map_err(|error| RtmpError::Amf {
            context: "command",
            error,
        })?;
        let info = values.get(3);
        let field = |key| {
            info.and_then(|v| v.get(key))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let c = Command {
            name: values
                .first()
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            txid: values.get(1).and_then(Value::as_number).unwrap_or(0.0),
            number: info.and_then(Value::as_number),
            level: field("level"),
            code: field("code"),
            description: field("description"),
        };
        self.reply(c)
    }

    fn reply(&mut self, c: Command) -> Result<(), RtmpError> {
        let name = self.config.name.clone();
        match c.name.as_str() {
            "_error" => self.refused = Some((c.code, c.description)),
            "onStatus" if c.level == "error" => self.refused = Some((c.code, c.description)),
            "onStatus" if c.code == "NetStream.Publish.Start" => self.accepted = true,
            "_result" if c.txid == CONNECT_TXID && !self.connected => {
                if c.level == "error" {
                    self.refused = Some((c.code, c.description));
                    return Ok(());
                }
                self.connected = true;
                self.command(
                    0,
                    &[
                        Value::String("releaseStream"),
                        Value::Number(RELEASE_STREAM_TXID),
                        Value::Null,
                        Value::String(&name),
                    ],
                )?;
                self.command(
                    0,
                    &[
                        Value::String("FCPublish"),
                        Value::Number(FC_PUBLISH_TXID),
                        Value::Null,
                        Value::String(&name),
                    ],
                )?;
                self.command(
                    0,
                    &[
                        Value::String("createStream"),
                        Value::Number(CREATE_STREAM_TXID),
                        Value::Null,
                    ],
                )?;
            }
            "_result" if c.txid == CREATE_STREAM_TXID && self.stream_id == 0 => {
                // A stream id is a small positive number; `as` saturates
                // anything else.
                self.stream_id = c.number.map_or(1, |n| n as u32);
                self.command(
                    self.stream_id,
                    &[
                        Value::String("publish"),
                        Value::Number(PUBLISH_TXID),
                        Value::Null,
                        Value::String(&name),
                        Value::String("live"),
                    ],
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn command(&mut self, stream_id: u32, values: &[Value<'_>]) -> Result<(), RtmpError> {
        let body = encode(values)?;
        self.writer
            .write(COMMAND_CSID, 0, COMMAND_AMF0, stream_id, &body)
    }
}

fn encode(values: &[Value<'_>]) -> Result<Vec<u8>, RtmpError> {
    let mut body = Vec::new();
    for v in values {
        amf0::encode(v, &mut body).map_err(|error| RtmpError::Amf {
            context: "publisher message",
            error,
        })?;
    }
    Ok(body)
}

#[cfg(test)]
#[path = "publisher_test.rs"]
mod tests;
