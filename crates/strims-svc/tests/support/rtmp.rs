// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A publisher's side of a service's RTMP ingress: a synthetic stream sent
//! over a real connection.

use std::rc::Rc;
use std::time::Duration as StdDuration;

use strims_base::rng::det_rng;
use strims_video::amf0::Value;
use strims_video::synth::publisher::{Publisher, PublisherConfig};
use strims_video::synth::script::{MetaValue, Script, Tag, TagKind};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Instant;

use super::video::STEP;

/// A publisher's RTMP connection.
pub struct Publish {
    stream: TcpStream,
    publisher: Publisher,
    input: Vec<u8>,
    output: Vec<u8>,
    /// The server closed the connection.
    closed: bool,
}

impl Publish {
    /// Connects to `addr` and publishes `live/<key>`; `None` if the
    /// connection fails or the publish isn't accepted within `limit`.
    pub async fn open(addr: &str, key: &str, limit: StdDuration) -> Option<Publish> {
        let stream = TcpStream::connect(addr).await.ok()?;
        let config = PublisherConfig::new("live", key);
        let mut p = Publish {
            stream,
            publisher: Publisher::new(config, &mut det_rng(0x70)),
            input: vec![0; 64 * 1024],
            output: vec![0; 64 * 1024],
            closed: false,
        };
        let end = Instant::now() + limit;
        while !p.publisher.publish_accepted() && !p.closed && Instant::now() < end {
            if let Some(refused) = p.publisher.refused() {
                eprintln!("the publish was refused: {refused:?}");
                return None;
            }
            p.exchange(Instant::now() + StdDuration::from_millis(10))
                .await;
        }
        p.publisher.publish_accepted().then_some(p)
    }

    /// Writes what the publisher queued.
    async fn flush(&mut self) {
        loop {
            let n = self.publisher.poll_transmit(&mut self.output);
            if n == 0 || self.closed {
                return;
            }
            if self.stream.write_all(&self.output[..n]).await.is_err() {
                self.closed = true;
            }
        }
    }

    /// Takes and answers the server's bytes until `deadline`, or until
    /// the server closes the connection.
    async fn exchange(&mut self, deadline: Instant) {
        while !self.closed {
            self.flush().await;
            let read = tokio::select! {
                biased;
                r = self.stream.read(&mut self.input) => Some(r),
                () = tokio::time::sleep_until(deadline) => None,
            };
            match read {
                None => return,
                Some(Ok(0)) | Some(Err(_)) => self.closed = true,
                Some(Ok(n)) => self.publisher.handle_input(&self.input[..n]).unwrap(),
            }
        }
    }

    fn send(&mut self, tag: &Tag, metadata: &[(String, MetaValue)]) {
        match tag.kind {
            TagKind::Video => self.publisher.send_video(tag.timestamp, &tag.body).unwrap(),
            TagKind::Audio => self.publisher.send_audio(tag.timestamp, &tag.body).unwrap(),
            TagKind::Metadata => {
                let fields: Vec<(&str, Value<'_>)> = metadata
                    .iter()
                    .map(|(k, v)| {
                        let v = match v {
                            MetaValue::Number(n) => Value::Number(*n),
                            MetaValue::Bool(b) => Value::Boolean(*b),
                            MetaValue::String(s) => Value::String(s),
                        };
                        (k.as_str(), v)
                    })
                    .collect();
                self.publisher.send_metadata(&fields).unwrap();
            }
        }
    }

    /// Sends the script's tags, each at its time after now, then takes
    /// the server's bytes until it closes the connection.
    pub async fn stream(mut self, script: Rc<Script>) {
        let start = Instant::now();
        for tag in &script.tags {
            let due = start + StdDuration::from_millis(u64::from(tag.timestamp));
            self.exchange(due).await;
            assert!(!self.closed, "the server closed the publishing connection");
            self.send(tag, &script.metadata);
        }
        self.flush().await;
        self.exchange(Instant::now() + STEP).await;
    }
}
