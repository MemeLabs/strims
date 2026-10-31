// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use strims_base::rng::det_rng;

use super::*;
use crate::amf0::Amf0Error;
use crate::rtmp::chunk::{ABORT, USER_CONTROL, WINDOW_ACK_SIZE};
use crate::rtmp::test_util::{Answer, Client, Ev, Replies, Reply, body, feed, pump};
use crate::synth::publisher::{Publisher, PublisherConfig};

fn new_server() -> RtmpServer {
    RtmpServer::new(&RtmpConfig::default(), &mut det_rng(1))
}

fn s(v: &str) -> Value<'_> {
    Value::String(v)
}

fn n(v: f64) -> Value<'static> {
    Value::Number(v)
}

fn publish_event(name: &str) -> Ev {
    Ev::Publish {
        app: "live".to_owned(),
        name: name.to_owned(),
    }
}

/// A server publishing `key`, its replies read, and the client.
fn publishing() -> (RtmpServer, Replies, Client) {
    let mut server = new_server();
    let mut c = Client::new();
    c.publish_flow("live", "key");
    let events = feed(&mut server, &c.take(), Answer::Accept).unwrap();
    assert_eq!(events, [publish_event("key")]);
    assert_eq!(server.phase(), Phase::Publishing);
    let mut replies = Replies::new();
    replies.read(&mut server);
    (server, replies, c)
}

fn control(type_id: u8, body: &[u8]) -> Reply {
    Reply {
        type_id,
        stream_id: 0,
        body: body.to_vec(),
    }
}

fn status<'a>(level: &'a str, code: &'a str, description: &'a str) -> Vec<Value<'a>> {
    vec![
        s("onStatus"),
        n(0.0),
        Value::Null,
        Value::Object(vec![
            ("level", s(level)),
            ("code", s(code)),
            ("description", s(description)),
        ]),
    ]
}

#[test]
fn command_flow_replies_in_order() {
    let mut server = new_server();
    assert_eq!(server.phase(), Phase::Handshake);
    let mut c = Client::new();
    c.publish_flow("live", "key?secret=1");
    let events = feed(&mut server, &c.take(), Answer::Hold).unwrap();
    assert_eq!(events, [publish_event("key")]);
    assert_eq!(server.phase(), Phase::PublishPending);
    assert_eq!(server.app(), Some("live"));
    assert_eq!(server.tc_url(), Some("rtmp://example/live"));
    assert!(!server.digest_handshake());
    server.accept_publish();
    assert_eq!(server.phase(), Phase::Publishing);

    let replies = Replies::new().read(&mut server);
    assert_eq!(replies.len(), 11);
    let mut bandwidth = 2_500_000u32.to_be_bytes().to_vec();
    bandwidth.push(2);
    assert_eq!(
        replies[..4],
        [
            control(WINDOW_ACK_SIZE, &2_500_000u32.to_be_bytes()),
            control(SET_PEER_BANDWIDTH, &bandwidth),
            control(USER_CONTROL, &[0, 0, 0, 0, 0, 0]),
            control(SET_CHUNK_SIZE, &4096u32.to_be_bytes()),
        ]
    );
    // The publish's message stream begins right before its status.
    assert_eq!(replies[9], control(USER_CONTROL, &[0, 0, 0, 0, 0, 1]));
    let commands: Vec<Vec<Value<'_>>> = replies[4..9]
        .iter()
        .chain(&replies[10..])
        .map(Reply::values)
        .collect();
    assert_eq!(
        commands,
        [
            vec![
                s("_result"),
                n(1.0),
                Value::Object(vec![
                    ("fmsVer", s("FMS/3,0,1,123")),
                    ("capabilities", n(31.0)),
                ]),
                Value::Object(vec![
                    ("level", s("status")),
                    ("code", s("NetConnection.Connect.Success")),
                    ("description", s("Connection succeeded.")),
                    ("objectEncoding", n(0.0)),
                ]),
            ],
            vec![s("_result"), n(2.0), Value::Null],
            vec![s("_result"), n(3.0), Value::Null],
            vec![
                s("onFCPublish"),
                n(0.0),
                Value::Null,
                Value::Object(vec![
                    ("code", s("NetStream.Publish.Start")),
                    ("description", s("FCPublish")),
                ]),
            ],
            vec![s("_result"), n(4.0), Value::Null, n(1.0)],
            status("status", "NetStream.Publish.Start", "Publishing."),
        ]
    );
    // The status goes on the publish's message stream.
    assert_eq!(replies[10].stream_id, 1);
    assert!(
        replies[4..9]
            .iter()
            .chain(&replies[10..])
            .all(|r| r.type_id == COMMAND_AMF0)
    );
}

/// `body(values)` as an AMF3 message's: its format byte, then the values.
fn amf3(format: u8, values: &[Value<'_>]) -> Vec<u8> {
    let mut b = vec![format];
    b.extend(body(values));
    b
}

#[test]
fn amf3_commands_and_data_of_amf0_values_are_taken() {
    let mut server = new_server();
    let mut c = Client::new();
    let connect = [
        s("connect"),
        n(1.0),
        Value::Object(vec![
            ("app", s("live")),
            ("tcUrl", s("rtmp://example/live")),
        ]),
    ];
    c.message(3, 0, COMMAND_AMF3, 0, &amf3(0, &connect));
    c.message(
        3,
        0,
        COMMAND_AMF3,
        0,
        &amf3(0, &[s("createStream"), n(2.0), Value::Null]),
    );
    let publish = [s("publish"), n(3.0), Value::Null, s("key"), s("live")];
    c.message(5, 0, COMMAND_AMF3, 1, &amf3(0, &publish));
    let events = feed(&mut server, &c.take(), Answer::Accept).unwrap();
    assert_eq!(events, [publish_event("key")]);
    let replies = Replies::new().read(&mut server);
    let results = replies
        .iter()
        .filter(|r| r.type_id == COMMAND_AMF0)
        .map(|r| r.values()[0].clone())
        .collect::<Vec<_>>();
    assert_eq!(results, [s("_result"), s("_result"), s("onStatus")]);

    let meta = [
        s("@setDataFrame"),
        s("onMetaData"),
        Value::Object(vec![("width", n(640.0))]),
    ];
    c.message(5, 0, DATA_AMF3, 1, &amf3(0, &meta));
    let events = feed(&mut server, &c.take(), Answer::Accept).unwrap();
    assert_eq!(
        events,
        [Ev::Metadata(Metadata {
            width: Some(640.0),
            ..Metadata::default()
        })]
    );
}

#[test]
fn amf3_messages_of_amf3_values_are_ignored() {
    let (mut server, mut replies, mut c) = publishing();
    let name = [s("onMetaData"), Value::Object(vec![("width", n(640.0))])];
    // Another format byte, an AMF3 value (the 0x11 switch marker), no
    // body, and a name that isn't a string.
    c.message(5, 0, DATA_AMF3, 1, &amf3(3, &name));
    c.message(5, 0, DATA_AMF3, 1, &[0, 0x11, 0x06, 0x03, b'x']);
    c.message(5, 0, DATA_AMF3, 1, &[]);
    c.message(3, 0, COMMAND_AMF3, 0, &amf3(0, &[n(1.0)]));
    c.message(3, 0, COMMAND_AMF3, 0, &[0, 0x11, 0x0a]);
    let events = feed(&mut server, &c.take(), Answer::Accept).unwrap();
    assert!(events.is_empty());
    assert_eq!(server.phase(), Phase::Publishing);
    assert!(replies.read(&mut server).is_empty());
}

#[test]
fn refusals_send_an_error_status_and_close() {
    for code in ["NetStream.Publish.BadName", "NetStream.Publish.Failed"] {
        let mut server = new_server();
        let mut c = Client::new();
        c.publish_flow("live", "key");
        let events = feed(&mut server, &c.take(), Answer::Refuse(code)).unwrap();
        assert_eq!(events, [publish_event("key")]);
        assert_eq!(server.phase(), Phase::Closed);
        let replies = Replies::new().read(&mut server);
        assert_eq!(
            replies.last().unwrap().values(),
            status("error", code, "refused")
        );
        // Input after the refusal is discarded.
        c.message(6, 0, VIDEO, 1, &[1, 2, 3]);
        let rest = c.take();
        assert_eq!(server.handle_input(&rest), Ok(rest.len()));
        assert_eq!(server.poll_event(), None);
        assert_eq!(server.poll_transmit(&mut [0; 64]), 0);
    }
    // Before a publish, the refusal still answers.
    let mut server = new_server();
    let mut c = Client::new();
    c.command(
        0,
        &[s("connect"), n(1.0), Value::Object(vec![("app", s("x"))])],
    );
    feed(&mut server, &c.take(), Answer::Hold).unwrap();
    server.refuse("NetConnection.Connect.Rejected", "no");
    let replies = Replies::new().read(&mut server);
    assert_eq!(
        replies.last().unwrap().values(),
        status("error", "NetConnection.Connect.Rejected", "no")
    );
    // In the handshake there is nothing to answer on.
    let mut server = new_server();
    server.refuse("NetStream.Publish.Failed", "early");
    assert_eq!(server.phase(), Phase::Closed);
    assert_eq!(server.poll_transmit(&mut [0; 64]), 0);
}

#[test]
fn unpublish_commands_end_the_publish() {
    for command in ["deleteStream", "FCUnpublish", "closeStream"] {
        let (mut server, _, mut c) = publishing();
        c.command(0, &[s(command), n(6.0), Value::Null, s("key")]);
        let events = feed(&mut server, &c.take(), Answer::Accept).unwrap();
        assert_eq!(events, [Ev::Unpublish], "{command}");
        assert_eq!(server.phase(), Phase::Connecting);
        // A second end yields nothing; media no longer flows.
        c.command(0, &[s("deleteStream"), n(7.0), Value::Null, n(1.0)]);
        c.message(6, 0, VIDEO, 1, &[1]);
        assert_eq!(feed(&mut server, &c.take(), Answer::Accept).unwrap(), []);
        // The connection can publish again.
        c.command(
            1,
            &[s("publish"), n(8.0), Value::Null, s("again"), s("live")],
        );
        let events = feed(&mut server, &c.take(), Answer::Accept).unwrap();
        assert_eq!(events, [publish_event("again")]);
    }
    // An unanswered publish ends too.
    let mut server = new_server();
    let mut c = Client::new();
    c.publish_flow("live", "key");
    c.command(0, &[s("FCUnpublish"), n(6.0), Value::Null, s("key")]);
    let events = feed(&mut server, &c.take(), Answer::Hold).unwrap();
    assert_eq!(events, [publish_event("key"), Ev::Unpublish]);
    server.accept_publish();
    assert_eq!(server.phase(), Phase::Connecting);
}

#[test]
fn media_and_metadata_flow_only_while_publishing() {
    let mut server = new_server();
    let mut c = Client::new();
    c.publish_flow("live", "key");
    c.message(6, 0, VIDEO, 1, &[0x17, 0, 0, 0, 0, 9]);
    c.data(&[s("onMetaData"), Value::Object(vec![("width", n(1.0))])]);
    let events = feed(&mut server, &c.take(), Answer::Hold).unwrap();
    assert_eq!(events, [publish_event("key")]);
    server.accept_publish();

    c.message(6, 40, VIDEO, 1, &[0x17, 1, 0, 0, 0, 7, 7]);
    c.message(4, 41, AUDIO, 1, &[0xaf, 1, 5]);
    c.data(&[
        s("@setDataFrame"),
        s("onMetaData"),
        Value::EcmaArray(vec![
            ("width", n(1280.0)),
            ("height", n(720.0)),
            ("encoder", s("test")),
        ]),
    ]);
    c.data(&[s("onMetaData"), Value::Object(vec![("framerate", n(30.0))])]);
    // Other data messages and values that aren't objects are skipped.
    c.data(&[s("onTextData"), Value::Object(vec![])]);
    c.data(&[s("onMetaData"), n(1.0)]);
    c.data(&[s("@setDataFrame"), s("onCuePoint"), Value::Null]);
    // An empty media message yields an empty body.
    c.message(6, 42, VIDEO, 1, &[]);
    let events = feed(&mut server, &c.take(), Answer::Accept).unwrap();
    assert_eq!(
        events,
        [
            Ev::Video(40, vec![0x17, 1, 0, 0, 0, 7, 7]),
            Ev::Audio(41, vec![0xaf, 1, 5]),
            Ev::Metadata(Metadata {
                width: Some(1280.0),
                height: Some(720.0),
                encoder: Some("test".to_owned()),
                ..Metadata::default()
            }),
            Ev::Metadata(Metadata {
                framerate: Some(30.0),
                ..Metadata::default()
            }),
            Ev::Video(42, vec![]),
        ]
    );
}

fn tag(type_id: u8, ts: u32, data: &[u8]) -> Vec<u8> {
    let mut t = vec![type_id];
    t.extend(&(data.len() as u32).to_be_bytes()[1..]);
    t.extend(&ts.to_be_bytes()[1..]);
    t.push((ts >> 24) as u8);
    t.extend([0, 0, 0]);
    t.extend(data);
    t.extend((11 + data.len() as u32).to_be_bytes());
    t
}

#[test]
fn aggregates_split_into_tags() {
    let (mut server, _, mut c) = publishing();
    let meta = body(&[
        s("@setDataFrame"),
        s("onMetaData"),
        Value::Object(vec![("width", n(640.0))]),
    ]);
    let agg = [
        tag(VIDEO, 1000, &[1, 2, 3]),
        tag(AUDIO, 1010, &[4]),
        tag(DATA_AMF0, 1020, &meta),
        tag(0x30, 1030, &[9]),
        tag(VIDEO, 0x0100_0000 + 1000, &[5, 6]),
    ]
    .concat();
    c.message(6, 5000, AGGREGATE, 1, &agg);
    c.message(4, 6000, AUDIO, 1, &[8]);
    let events = feed(&mut server, &c.take(), Answer::Accept).unwrap();
    assert_eq!(
        events,
        [
            Ev::Video(5000, vec![1, 2, 3]),
            Ev::Audio(5010, vec![4]),
            Ev::Metadata(Metadata {
                width: Some(640.0),
                ..Metadata::default()
            }),
            Ev::Video(5000 + 0x0100_0000, vec![5, 6]),
            Ev::Audio(6000, vec![8]),
        ]
    );

    // A tag past the message's end.
    let mut bad = tag(VIDEO, 0, &[1, 2, 3]);
    bad.truncate(bad.len() - 1);
    c.message(6, 0, AGGREGATE, 1, &bad);
    let e = feed(&mut server, &c.take(), Answer::Accept).unwrap_err();
    assert_eq!(e, RtmpError::Aggregate);
    assert_eq!(e.to_string(), "aggregate message is malformed");
    assert_eq!(server.phase(), Phase::Closed);

    // A data tag over the command bound.
    let config = RtmpConfig {
        max_command_message: 100,
        ..RtmpConfig::default()
    };
    let mut server = RtmpServer::new(&config, &mut det_rng(1));
    let mut c = Client::new();
    c.publish_flow("live", "key");
    c.message(6, 0, AGGREGATE, 1, &tag(DATA_AMF0, 0, &[0; 101]));
    let e = feed(&mut server, &c.take(), Answer::Accept).unwrap_err();
    assert_eq!(
        e,
        RtmpError::MessageTooLarge {
            type_id: DATA_AMF0,
            len: 101,
            max: 100
        }
    );

    // Before the publish is accepted an aggregate is skipped whole.
    let mut server = new_server();
    let mut c = Client::new();
    c.publish_flow("live", "key");
    c.message(6, 0, AGGREGATE, 1, &bad);
    let events = feed(&mut server, &c.take(), Answer::Hold).unwrap();
    assert_eq!(events, [publish_event("key")]);
}

#[test]
fn acks_follow_the_client_window() {
    let mut server = new_server();
    let mut c = Client::new();
    c.publish_flow("live", "key");
    let setup = c.take();
    feed(&mut server, &setup, Answer::Accept).unwrap();
    let mut replies = Replies::new();
    replies.read(&mut server);
    c.message(2, 0, WINDOW_ACK_SIZE, 0, &1000u32.to_be_bytes());
    for ts in 0..10 {
        c.message(6, ts, VIDEO, 1, &[0; 500]);
    }
    let input = c.take();
    let events = feed(&mut server, &input, Answer::Accept).unwrap();
    assert_eq!(events.len(), 10);
    let acks: Vec<u32> = replies
        .read(&mut server)
        .iter()
        .filter(|r| r.type_id == ACKNOWLEDGEMENT)
        .map(|r| u32::from_be_bytes(r.body[..4].try_into().unwrap()))
        .collect();
    // Sequence numbers count every byte after the handshake, and each
    // comes at the first chunk end a window past the last.
    let total = (setup.len() - (1 + 2 * 1536) + input.len()) as u32;
    let chunk = 12 + 128;
    assert!(acks.len() >= 4);
    assert!((1000..1000 + chunk).contains(&acks[0]));
    for w in acks.windows(2) {
        assert!((1000..1000 + chunk).contains(&(w[1] - w[0])), "{acks:?}");
    }
    assert!(total - acks.last().unwrap() < 1000);
}

#[test]
fn pings_are_answered() {
    let (mut server, mut replies, mut c) = publishing();
    c.message(2, 0, USER_CONTROL, 0, &[0, 6, 0, 0, 0x30, 0x39]);
    feed(&mut server, &c.take(), Answer::Accept).unwrap();
    assert_eq!(
        replies.read(&mut server),
        [control(USER_CONTROL, &[0, 7, 0, 0, 0x30, 0x39])]
    );
}

/// Feeds a plain handshake and `values` as a command; returns the result.
fn command_after(
    setup: impl FnOnce(&mut Client),
    values: &[Value<'_>],
) -> Result<Vec<Ev>, RtmpError> {
    let mut server = new_server();
    let mut c = Client::new();
    setup(&mut c);
    c.command(1, values);
    let r = feed(&mut server, &c.take(), Answer::Accept);
    if r.is_err() {
        assert_eq!(server.phase(), Phase::Closed);
        // A failed connection discards input.
        assert_eq!(server.handle_input(&[1, 2, 3]), Ok(3));
    }
    r
}

fn connected(c: &mut Client) {
    c.command(
        0,
        &[
            s("connect"),
            n(1.0),
            Value::Object(vec![("app", s("live"))]),
        ],
    );
}

#[test]
fn commands_out_of_order_fail_the_connection() {
    for name in ["releaseStream", "FCPublish", "createStream", "publish"] {
        let e = command_after(|_| {}, &[s(name), n(2.0), Value::Null, s("key")]).unwrap_err();
        assert_eq!(e, RtmpError::NotConnected(name));
        assert_eq!(e.to_string(), format!("{name} before connect"));
    }
    // Unknown commands are ignored, connected or not.
    assert_eq!(
        command_after(|_| {}, &[s("getStreamLength"), n(2.0)]),
        Ok(vec![])
    );
    assert_eq!(
        command_after(connected, &[s("_checkbw"), n(2.0)]),
        Ok(vec![])
    );
    assert_eq!(
        command_after(|_| {}, &[s("deleteStream"), n(2.0)]),
        Ok(vec![])
    );

    let e = command_after(connected, &[s("connect"), n(2.0), Value::Null]).unwrap_err();
    assert_eq!(e, RtmpError::Unexpected("second connect"));
    assert_eq!(e.to_string(), "unexpected second connect");
    let e = command_after(
        |_| {},
        &[s("connect"), n(1.0), Value::Object(vec![("tcUrl", s("x"))])],
    )
    .unwrap_err();
    assert_eq!(e, RtmpError::ConnectApp);
    assert_eq!(e.to_string(), "connect names no app");
    let e = command_after(connected, &[s("publish"), n(2.0), Value::Null, n(1.0)]).unwrap_err();
    assert_eq!(e, RtmpError::PublishName);
    assert_eq!(e.to_string(), "publish names no stream");
    let publish = [s("publish"), n(5.0), Value::Null, s("key"), s("live")];
    let e = command_after(|c| c.publish_flow("live", "key"), &publish).unwrap_err();
    assert_eq!(e, RtmpError::Unexpected("publish while publishing"));
    let e = command_after(|_| {}, &[n(1.0)]).unwrap_err();
    assert_eq!(e, RtmpError::CommandName);
    assert_eq!(e.to_string(), "command name is not a string");
}

#[test]
fn malformed_amf_fails_the_connection() {
    let mut server = new_server();
    let mut c = Client::new();
    c.message(3, 0, COMMAND_AMF0, 0, &[0x11, 0x02]);
    let e = feed(&mut server, &c.take(), Answer::Accept).unwrap_err();
    assert_eq!(
        e,
        RtmpError::Amf {
            context: "command",
            error: Amf0Error::Amf3
        }
    );
    assert_eq!(
        e.to_string(),
        "invalid AMF0 in a command: AMF3 values are not supported"
    );
    let (mut server, _, mut c) = publishing();
    let mut b = body(&[s("@setDataFrame"), s("onMetaData")]);
    b.extend([0x03, 0, 1, b'k', 0x07, 0, 0]);
    c.message(5, 0, DATA_AMF0, 1, &b);
    let e = feed(&mut server, &c.take(), Answer::Accept).unwrap_err();
    assert_eq!(
        e,
        RtmpError::Amf {
            context: "data message",
            error: Amf0Error::Reference
        }
    );
}

#[test]
fn bounds_fail_the_connection() {
    // A media message over its bound.
    let (mut server, _, mut c) = publishing();
    c.message(6, 0, VIDEO, 1, &vec![0; (4 << 20) + 1]);
    let input = c.take();
    let e = feed(&mut server, &input, Answer::Accept).unwrap_err();
    assert!(matches!(
        e,
        RtmpError::MessageTooLarge { type_id: VIDEO, .. }
    ));
    // A media message at the bound passes.
    let (mut server, _, mut c) = publishing();
    let frame = vec![7; 4 << 20];
    c.message(6, 0, VIDEO, 1, &frame);
    let events = feed(&mut server, &c.take(), Answer::Accept).unwrap();
    assert_eq!(events, [Ev::Video(0, frame)]);
    // Chunk streams.
    let (mut server, _, mut c) = publishing();
    for csid in 10..80 {
        c.message(csid, 0, AUDIO, 1, &[1]);
    }
    let e = feed(&mut server, &c.take(), Answer::Accept).unwrap_err();
    assert_eq!(e, RtmpError::TooManyChunkStreams(64));
}

#[test]
fn output_past_its_bound_fails_the_connection() {
    // The connect reply doesn't fit beside S0 S1 S2.
    let config = RtmpConfig {
        max_output: 1 + 2 * 1536 + 40,
        ..RtmpConfig::default()
    };
    let mut server = RtmpServer::new(&config, &mut det_rng(1));
    let mut c = Client::new();
    connected(&mut c);
    let e = feed(&mut server, &c.take(), Answer::Accept).unwrap_err();
    assert_eq!(e, RtmpError::OutputFull(config.max_output));
    assert_eq!(server.phase(), Phase::Closed);

    // The publish status doesn't fit: the next call returns the error.
    let mut c = Client::new();
    c.publish_flow("live", "key");
    let input = c.take();
    let mut probe = new_server();
    feed(&mut probe, &input, Answer::Hold).unwrap();
    let config = RtmpConfig {
        max_output: probe.session.writer.queued(),
        ..RtmpConfig::default()
    };
    let mut server = RtmpServer::new(&config, &mut det_rng(1));
    feed(&mut server, &input, Answer::Hold).unwrap();
    server.accept_publish();
    assert_eq!(server.phase(), Phase::Closed);
    assert_eq!(
        server.handle_input(&[]),
        Err(RtmpError::OutputFull(config.max_output))
    );
    assert_eq!(server.handle_input(&[1]), Ok(1));
}

#[test]
fn handle_input_waits_for_the_event_to_be_polled() {
    let mut server = new_server();
    let mut c = Client::new();
    c.publish_flow("live", "key");
    c.message(6, 0, VIDEO, 1, &[1]);
    let input = c.take();
    let n = server.handle_input(&input).unwrap();
    assert!(n < input.len());
    assert_eq!(server.handle_input(&input[n..]), Ok(0));
    assert_eq!(
        server.poll_event().map(Ev::from),
        Some(publish_event("key"))
    );
    assert_eq!(server.poll_event(), None);
}

/// The bytes a publisher sends over a whole session: digest handshake,
/// commands, a window, metadata, media, an aggregate, a ping and the end.
fn recorded_session() -> Vec<u8> {
    let mut p = Publisher::new(PublisherConfig::new("live", "key"), &mut det_rng(5));
    let mut server = RtmpServer::new(&RtmpConfig::default(), &mut det_rng(6));
    let mut sent = Vec::new();
    pump(&mut p, &mut server, Answer::Accept, &mut sent).unwrap();
    assert!(p.publish_accepted());
    p.send_message(2, 0, WINDOW_ACK_SIZE, 0, &300u32.to_be_bytes())
        .unwrap();
    p.send_metadata(&[("width", n(320.0)), ("encoder", s("synth"))])
        .unwrap();
    for i in 0..6u32 {
        let body: Vec<u8> = (0..150 + i * 40).map(|b| b as u8).collect();
        p.send_video(i * 33, &body).unwrap();
        p.send_audio(i * 21, &body[..20]).unwrap();
    }
    p.send_message(
        6,
        300,
        AGGREGATE,
        1,
        &[tag(VIDEO, 0, &[1]), tag(AUDIO, 5, &[2])].concat(),
    )
    .unwrap();
    p.send_message(2, 0, USER_CONTROL, 0, &[0, 6, 0, 0, 0, 1])
        .unwrap();
    p.unpublish().unwrap();
    let events = pump(&mut p, &mut server, Answer::Accept, &mut sent).unwrap();
    assert_eq!(events.len(), 1 + 12 + 2 + 1);
    sent
}

#[test]
fn input_split_at_every_byte_gives_the_same_events() {
    let input = recorded_session();
    let run = |cuts: &[usize]| {
        let mut server = RtmpServer::new(&RtmpConfig::default(), &mut det_rng(6));
        let mut events = Vec::new();
        let mut out = Vec::new();
        let mut buf = [0; 4096];
        let mut start = 0;
        for end in cuts.iter().copied().chain([input.len()]) {
            events.extend(feed(&mut server, &input[start..end], Answer::Accept).unwrap());
            loop {
                let n = server.poll_transmit(&mut buf);
                if n == 0 {
                    break;
                }
                out.extend_from_slice(&buf[..n]);
            }
            start = end;
        }
        (events, out)
    };
    let whole = run(&[]);
    assert_eq!(whole.0.last(), Some(&Ev::Unpublish));
    for cut in 0..=input.len() {
        assert!(run(&[cut]) == whole, "cut at {cut}");
    }
    let every: Vec<usize> = (1..input.len()).collect();
    assert!(run(&every) == whole);
}

#[derive(Debug, Clone)]
struct RawMessage {
    csid: u32,
    timestamp: u32,
    type_id: u8,
    body: Vec<u8>,
}

fn raw_message() -> impl Strategy<Value = RawMessage> {
    let type_id = prop_oneof![
        prop::sample::select(vec![
            SET_CHUNK_SIZE,
            ABORT,
            ACKNOWLEDGEMENT,
            USER_CONTROL,
            WINDOW_ACK_SIZE,
            SET_PEER_BANDWIDTH,
            AUDIO,
            VIDEO,
            DATA_AMF0,
            COMMAND_AMF0,
            AGGREGATE,
        ]),
        any::<u8>(),
    ];
    let body = prop_oneof![
        proptest::collection::vec(any::<u8>(), 0..64),
        proptest::collection::vec(any::<u8>(), 0..600),
        (0u32..200).prop_map(|v| v.to_be_bytes().to_vec()),
        Just(body(&[
            s("onMetaData"),
            Value::Object(vec![("width", n(1.0))])
        ])),
        Just([tag(VIDEO, 3, &[1, 2]), tag(DATA_AMF0, 4, &[2, 0])].concat()),
    ];
    (
        prop_oneof![2u32..8, 2u32..400, Just(65599)],
        prop_oneof![0u32..100, any::<u32>()],
        type_id,
        body,
    )
        .prop_map(|(csid, timestamp, type_id, body)| RawMessage {
            csid,
            timestamp,
            type_id,
            body,
        })
}

fn proptest_config() -> Config {
    Config {
        cases: 256,
        rng_seed: RngSeed::Fixed(0x0e7f_5eed),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    // Each call consumes input or yields an event (checked by `feed`), and
    // the reader's steps stay linear in the bytes fed.
    #[test]
    fn arbitrary_input_never_panics(
        prefix in 0usize..3,
        messages in proptest::collection::vec(raw_message(), 0..24),
        chunk_size in prop_oneof![Just(128u32), 1u32..300],
        flips in proptest::collection::vec((any::<usize>(), any::<u8>()), 0..4),
        tail in proptest::collection::vec(any::<u8>(), 0..256),
        cuts in proptest::collection::vec(any::<usize>(), 0..6),
    ) {
        let mut c = Client::new();
        match prefix {
            0 => {}
            1 => connected(&mut c),
            _ => c.publish_flow("live", "key"),
        }
        let mut input = c.take();
        let start = if prefix == 0 { 0 } else { input.len() };
        let mut w = crate::rtmp::chunk::ChunkWriter::new(usize::MAX, true);
        if chunk_size != 128 {
            w.write(2, 0, SET_CHUNK_SIZE, 0, &chunk_size.to_be_bytes()).unwrap();
            w.set_chunk_size(chunk_size);
        }
        for m in &messages {
            w.write(m.csid, m.timestamp, m.type_id, 1, &m.body).unwrap();
        }
        let mut chunks = vec![0; w.queued()];
        w.poll_transmit(&mut chunks);
        input.extend(chunks);
        input.extend(&tail);
        let len = input.len();
        for (at, x) in flips {
            if len > start {
                input[start + at % (len - start)] ^= x;
            }
        }
        let mut cuts: Vec<usize> = cuts.into_iter().map(|c| c % (len + 1)).collect();
        cuts.sort_unstable();
        let mut server = new_server();
        let mut from = 0;
        let mut calls = 0;
        for end in cuts.into_iter().chain([len]) {
            calls += 1;
            if feed(&mut server, &input[from..end], Answer::Accept).is_err() {
                break;
            }
            server.poll_transmit(&mut vec![0; 1 << 20]);
            from = end;
        }
        prop_assert!(server.reader.steps <= 66 * (len as u64 + 2 * calls + 1));
    }
}
