// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Drivers and raw clients for the RTMP unit tests.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::chunk::{COMMAND_AMF0, ChunkReader, ChunkWriter, Control, DATA_AMF0, Outcome};
use super::handshake::PACKET_LEN;
use super::{RtmpConfig, RtmpError, RtmpEvent, RtmpServer};
use crate::amf0::{self, Metadata, Value};
use crate::synth::publisher::Publisher;

/// An [`RtmpEvent`] that owns its data.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Ev {
    Publish { app: String, name: String },
    Metadata(Metadata),
    Video(u32, Vec<u8>),
    Audio(u32, Vec<u8>),
    Unpublish,
}

impl From<RtmpEvent<'_>> for Ev {
    fn from(e: RtmpEvent<'_>) -> Self {
        match e {
            RtmpEvent::Publish { app, name } => Ev::Publish { app, name },
            RtmpEvent::Metadata(m) => Ev::Metadata(m),
            RtmpEvent::Video { timestamp, data } => Ev::Video(timestamp, data.to_vec()),
            RtmpEvent::Audio { timestamp, data } => Ev::Audio(timestamp, data.to_vec()),
            RtmpEvent::Unpublish => Ev::Unpublish,
        }
    }
}

/// What the driver answers a publish with.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Answer {
    Accept,
    Refuse(&'static str),
    Hold,
}

/// Feeds `input` to `server` as a driver does, answering publishes;
/// returns the events. Checks that each call consumes input or yields an
/// event.
pub(crate) fn feed(
    server: &mut RtmpServer,
    mut input: &[u8],
    answer: Answer,
) -> Result<Vec<Ev>, RtmpError> {
    let mut events = Vec::new();
    loop {
        let n = server.handle_input(input)?;
        assert!(n <= input.len());
        input = &input[n..];
        let mut yielded = false;
        while let Some(e) = server.poll_event() {
            yielded = true;
            let e = Ev::from(e);
            if matches!(e, Ev::Publish { .. }) {
                match answer {
                    Answer::Accept => server.accept_publish(),
                    Answer::Refuse(code) => server.refuse(code, "refused"),
                    Answer::Hold => {}
                }
            }
            events.push(e);
        }
        assert!(n > 0 || yielded || input.is_empty(), "handle_input stalled");
        if input.is_empty() && !yielded {
            return Ok(events);
        }
    }
}

/// A message the server sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reply {
    pub(crate) type_id: u8,
    pub(crate) stream_id: u32,
    pub(crate) body: Vec<u8>,
}

impl Reply {
    pub(crate) fn values(&self) -> Vec<Value<'_>> {
        amf0::decode_all(&self.body).unwrap()
    }
}

/// Reads the server's output after S0 S1 S2, following its chunk size.
pub(crate) struct Replies {
    reader: ChunkReader,
    skip: usize,
    pings: ChunkWriter,
}

impl Replies {
    pub(crate) fn new() -> Self {
        Replies {
            reader: ChunkReader::new(RtmpConfig::default().limits()),
            skip: 1 + 2 * PACKET_LEN,
            pings: ChunkWriter::new(usize::MAX, false),
        }
    }

    /// Drains the server's output into messages.
    pub(crate) fn read(&mut self, server: &mut RtmpServer) -> Vec<Reply> {
        let mut out = vec![0; 1 << 20];
        let n = server.poll_transmit(&mut out);
        let skipped = self.skip.min(n);
        self.skip -= skipped;
        let mut input = &out[skipped..n];
        let mut replies = Vec::new();
        loop {
            let (n, outcome) = self.reader.read(input).unwrap();
            input = &input[n..];
            match outcome {
                Outcome::None => return replies,
                Outcome::Ack(_) => {}
                Outcome::Message => {
                    let m = self.reader.message().unwrap();
                    let reply = Reply {
                        type_id: m.type_id,
                        stream_id: m.stream_id,
                        body: m.body.to_vec(),
                    };
                    if let Some(c) = Control::parse(reply.type_id, &reply.body).unwrap() {
                        c.apply(&mut self.reader, &mut self.pings).unwrap();
                    }
                    replies.push(reply);
                }
            }
        }
    }
}

/// A client that sends a plain handshake, then whatever a test writes.
pub(crate) struct Client {
    pub(crate) w: ChunkWriter,
}

impl Client {
    pub(crate) fn new() -> Self {
        let mut w = ChunkWriter::new(usize::MAX, false);
        w.push_raw(&[3]).unwrap();
        w.push_raw(&[0; 2 * PACKET_LEN]).unwrap();
        Client { w }
    }

    pub(crate) fn message(&mut self, csid: u32, ts: u32, type_id: u8, sid: u32, body: &[u8]) {
        self.w.write(csid, ts, type_id, sid, body).unwrap();
    }

    pub(crate) fn command(&mut self, stream_id: u32, values: &[Value<'_>]) {
        let body = body(values);
        self.message(3, 0, COMMAND_AMF0, stream_id, &body);
    }

    pub(crate) fn data(&mut self, values: &[Value<'_>]) {
        let body = body(values);
        self.message(5, 0, DATA_AMF0, 1, &body);
    }

    /// connect, releaseStream, FCPublish, createStream and publish.
    pub(crate) fn publish_flow(&mut self, app: &str, name: &str) {
        let s = Value::String;
        let n = Value::Number;
        self.command(
            0,
            &[
                s("connect"),
                n(1.0),
                Value::Object(vec![("app", s(app)), ("tcUrl", s("rtmp://example/live"))]),
            ],
        );
        self.command(0, &[s("releaseStream"), n(2.0), Value::Null, s(name)]);
        self.command(0, &[s("FCPublish"), n(3.0), Value::Null, s(name)]);
        self.command(0, &[s("createStream"), n(4.0), Value::Null]);
        self.command(1, &[s("publish"), n(5.0), Value::Null, s(name), s("live")]);
    }

    pub(crate) fn take(&mut self) -> Vec<u8> {
        let mut out = vec![0; self.w.queued()];
        self.w.poll_transmit(&mut out);
        out
    }
}

pub(crate) fn body(values: &[Value<'_>]) -> Vec<u8> {
    let mut b = Vec::new();
    for v in values {
        amf0::encode(v, &mut b).unwrap();
    }
    b
}

/// Moves bytes between `p` and `server` until both are idle; appends what
/// `p` sent to `sent` and returns the server's events.
pub(crate) fn pump(
    p: &mut Publisher,
    server: &mut RtmpServer,
    answer: Answer,
    sent: &mut Vec<u8>,
) -> Result<Vec<Ev>, RtmpError> {
    let mut events = Vec::new();
    let mut buf = vec![0; 1 << 16];
    loop {
        let n = p.poll_transmit(&mut buf);
        sent.extend_from_slice(&buf[..n]);
        if n > 0 {
            events.extend(feed(server, &buf[..n], answer)?);
        }
        let m = server.poll_transmit(&mut buf);
        if m > 0 {
            p.handle_input(&buf[..m])?;
        }
        if n == 0 && m == 0 {
            return Ok(events);
        }
    }
}
