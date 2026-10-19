// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;
use std::rc::Rc;

use proptest::prelude::*;
use strims_proto::debug::v1::{PProfRequest, PProfResponse, WatchMetricsResponse};
use strims_proto::types::Certificate;

use super::*;
use crate::wire::{Encoded, put_uvarint};

const PPROF: &str = "strims.debug.v1.Debug.PProf";
const WATCH: &str = "strims.debug.v1.Debug.WatchMetrics";

/// Moves everything `from` has to send into `to`, cutting byte streams
/// every `chunk` bytes.
fn pump(from: &mut Endpoint, to: &mut Endpoint, chunk: usize) {
    let mut stream = Vec::new();
    while let Some(frame) = from.poll_transmit() {
        match to.config().framing {
            Framing::Delimited => stream.extend_from_slice(&frame),
            Framing::Message => to.handle_input(&frame).unwrap(),
        }
    }
    for c in stream.chunks(chunk.max(1)) {
        to.handle_input(c).unwrap();
    }
}

fn events(e: &mut Endpoint) -> Vec<Event> {
    std::iter::from_fn(|| e.poll_event()).collect()
}

fn responses(e: &mut Endpoint, call: CallId) -> Vec<Response> {
    std::iter::from_fn(|| e.poll_response(call)).collect()
}

/// The single request among `e`'s events.
fn request(e: &mut Endpoint) -> (InboundId, u64, String, Vec<u8>) {
    match events(e).as_slice() {
        [
            Event::Request {
                call,
                id,
                method,
                argument,
                ..
            },
        ] => (*call, *id, method.clone(), argument.clone()),
        other => panic!("expected one request, got {other:?}"),
    }
}

/// Decodes the Calls `e` has to send.
fn sent(e: &mut Endpoint) -> Vec<Call> {
    let framing = e.config().framing;
    std::iter::from_fn(|| e.poll_transmit())
        .map(|f| {
            let body = match framing {
                Framing::Delimited => {
                    let (len, n) = crate::wire::get_uvarint(&f).unwrap().unwrap();
                    assert_eq!(f.len(), n + len as usize);
                    f[n..].to_vec()
                }
                Framing::Message => f,
            };
            Call::decode(body.as_slice()).unwrap()
        })
        .collect()
}

fn delimited(call: &Call) -> Vec<u8> {
    let body = call.encode_to_vec();
    let mut out = Vec::new();
    put_uvarint(&mut out, body.len() as u64);
    out.extend_from_slice(&body);
    out
}

fn callback(parent_id: u64, kind: Kind, argument: Vec<u8>) -> Call {
    Call {
        id: 1000 + parent_id,
        parent_id,
        method: CALLBACK.into(),
        kind: kind as i32,
        argument,
        headers: Default::default(),
    }
}

fn pprof(name: &str) -> PProfRequest {
    PProfRequest {
        name: name.into(),
        ..Default::default()
    }
}

fn item(n: u8) -> WatchMetricsResponse {
    WatchMetricsResponse { data: vec![n; 4] }
}

fn item_response(n: u8) -> Response {
    Response::Item(item(n).encode_to_vec())
}

#[test]
fn unary_calls_round_trip_across_any_cut() {
    for chunk in [1, 2, 3, 7, 1000] {
        let mut client = Endpoint::new(Config::default());
        let mut server = Endpoint::new(Config::default());

        let call = client.start_call(PPROF, false, &pprof("heap")).unwrap();
        pump(&mut client, &mut server, chunk);
        let (req, id, method, arg) = request(&mut server);
        assert_eq!((id, method.as_str()), (call.get(), PPROF));
        assert_eq!(PProfRequest::decode(arg.as_slice()).unwrap(), pprof("heap"));

        let res = PProfResponse {
            name: "heap".into(),
            data: vec![1, 2, 3],
        };
        assert!(server.reply_value(req, &res));
        assert!(!server.is_serving(req));
        pump(&mut server, &mut client, chunk);

        assert_eq!(events(&mut client), [Event::Response { call }]);
        assert_eq!(
            responses(&mut client, call),
            [Response::Item(res.encode_to_vec())]
        );
        assert_eq!(client.poll_response(call), None);
    }
}

#[test]
fn streams_deliver_items_then_end() {
    let mut client = Endpoint::new(Config::default());
    let mut server = Endpoint::new(Config::default());
    let call = client.start_call(WATCH, true, &pprof("")).unwrap();
    pump(&mut client, &mut server, 5);
    let (req, ..) = request(&mut server);

    for n in 0..3 {
        assert!(server.reply_item(req, &item(n)));
    }
    assert!(server.reply_close(req));
    pump(&mut server, &mut client, 5);

    // one event for the queue going non-empty
    assert_eq!(events(&mut client), [Event::Response { call }]);
    assert_eq!(
        responses(&mut client, call),
        [
            item_response(0),
            item_response(1),
            item_response(2),
            Response::End
        ]
    );
    assert_eq!(client.queued_response_bytes(), 0);
}

#[test]
fn stream_errors_arrive_before_or_after_items() {
    let mut client = Endpoint::new(Config::default());
    let mut server = Endpoint::new(Config::default());
    let before = client.start_call(WATCH, true, &pprof("a")).unwrap();
    let after = client.start_call(WATCH, true, &pprof("b")).unwrap();
    pump(&mut client, &mut server, 64);
    let reqs: Vec<InboundId> = events(&mut server)
        .into_iter()
        .map(|e| match e {
            Event::Request { call, .. } => call,
            e => panic!("{e:?}"),
        })
        .collect();

    assert!(server.reply_error(reqs[0], &RpcError::new(4, "denied")));
    assert!(server.reply_item(reqs[1], &item(9)));
    assert!(server.reply_error(reqs[1], &RpcError::new(-2, "broke")));
    pump(&mut server, &mut client, 64);

    assert_eq!(
        responses(&mut client, before),
        [Response::Failed(CallError::Remote(RpcError::new(
            4, "denied"
        )))]
    );
    assert_eq!(
        responses(&mut client, after),
        [
            item_response(9),
            Response::Failed(CallError::Remote(RpcError::new(-2, "broke")))
        ]
    );
}

#[test]
fn a_cancel_names_the_call_and_silences_the_server() {
    let mut client = Endpoint::new(Config::default());
    let mut server = Endpoint::new(Config::default());
    let call = client.start_call(WATCH, true, &pprof("")).unwrap();
    pump(&mut client, &mut server, 64);
    let (req, ..) = request(&mut server);

    client.cancel(call);
    let cancels = sent(&mut client);
    assert_eq!(cancels.len(), 1);
    assert_eq!(cancels[0].method, CANCEL);
    assert_eq!(cancels[0].kind, Kind::CallKindCancel as i32);
    assert_eq!(cancels[0].parent_id, call.get());
    assert!(cancels[0].argument.is_empty());

    // replay the cancel into the server
    server.handle_input(&delimited(&cancels[0])).unwrap();
    assert_eq!(events(&mut server), [Event::Cancelled { call: req }]);
    assert!(!server.reply_item(req, &item(1)));
    assert!(!server.reply_close(req));
    assert_eq!(server.poll_transmit(), None);

    // responses already in flight are dropped
    client
        .handle_input(&delimited(&callback(
            call.get(),
            Kind::CallKindDefault,
            item(1).encode_to_vec(),
        )))
        .unwrap();
    assert_eq!(events(&mut client), []);
    assert_eq!(client.metrics().dropped, 1);
}

#[test]
fn cancels_are_recognised_by_method_whatever_their_kind() {
    let mut server = Endpoint::new(Config::default());
    let req = Call {
        id: 3,
        method: PPROF.into(),
        ..Default::default()
    };
    server.handle_input(&delimited(&req)).unwrap();
    let (handle, ..) = request(&mut server);
    let cancel = Call {
        id: 4,
        parent_id: 3,
        method: CANCEL.into(),
        kind: Kind::CallKindDefault as i32,
        ..Default::default()
    };
    server.handle_input(&delimited(&cancel)).unwrap();
    assert_eq!(events(&mut server), [Event::Cancelled { call: handle }]);
}

#[test]
fn a_cancelled_request_id_can_be_reused_without_crossing_replies() {
    let mut server = Endpoint::new(Config::default());
    let req = Call {
        id: 3,
        method: PPROF.into(),
        ..Default::default()
    };
    let cancel = Call {
        id: 4,
        parent_id: 3,
        method: CANCEL.into(),
        ..Default::default()
    };
    server.handle_input(&delimited(&req)).unwrap();
    server.handle_input(&delimited(&cancel)).unwrap();
    server.handle_input(&delimited(&req)).unwrap();
    let evs = events(&mut server);
    let (Event::Request { call: old, .. }, Event::Request { call: new, .. }) = (&evs[0], &evs[2])
    else {
        panic!("{evs:?}");
    };
    assert_ne!(old, new);
    assert!(!server.reply_value(*old, &PProfResponse::default()));
    assert!(server.reply_value(*new, &PProfResponse::default()));
    assert_eq!(sent(&mut server).len(), 1);
}

#[test]
fn a_client_only_endpoint_drops_requests() {
    let mut client = Endpoint::new(Config {
        serve: false,
        ..Config::default()
    });
    let req = Call {
        id: 1,
        method: PPROF.into(),
        ..Default::default()
    };
    client.handle_input(&delimited(&req)).unwrap();
    assert_eq!(events(&mut client), []);
    assert_eq!(client.poll_transmit(), None);
    assert_eq!(client.metrics().dropped, 1);
}

#[test]
fn duplicate_and_zero_request_ids_are_dropped() {
    let mut server = Endpoint::new(Config::default());
    let req = |id| Call {
        id,
        method: PPROF.into(),
        ..Default::default()
    };
    server.handle_input(&delimited(&req(5))).unwrap();
    server.handle_input(&delimited(&req(5))).unwrap();
    server.handle_input(&delimited(&req(0))).unwrap();
    // a zero-length frame is a Call with every field zero
    server.handle_input(&[0]).unwrap();
    assert_eq!(events(&mut server).len(), 1);
    assert_eq!(server.metrics().dropped, 3);
}

#[test]
fn response_kinds_map_to_call_errors() {
    let cases = [
        (false, Kind::CallKindUndefined as i32, CallError::Undefined),
        (false, Kind::CallKindClose as i32, CallError::Closed),
        (true, Kind::CallKindCancel as i32, CallError::UnknownKind(2)),
        (true, 7, CallError::UnknownKind(7)),
    ];
    for (streaming, kind, want) in cases {
        let mut client = Endpoint::new(Config::default());
        let call = client.start_call(WATCH, streaming, &pprof("")).unwrap();
        let mut res = callback(call.get(), Kind::CallKindDefault, Vec::new());
        res.kind = kind;
        client.handle_input(&delimited(&res)).unwrap();
        assert_eq!(responses(&mut client, call), [Response::Failed(want)]);
    }

    // an error that doesn't decode
    let mut client = Endpoint::new(Config::default());
    let call = client.start_call(PPROF, false, &pprof("")).unwrap();
    client
        .handle_input(&delimited(&callback(
            call.get(),
            Kind::CallKindError,
            vec![0xff],
        )))
        .unwrap();
    assert!(matches!(
        responses(&mut client, call).as_slice(),
        [Response::Failed(CallError::Decode(_))]
    ));
}

#[test]
fn requests_over_the_inbound_limit_are_answered_with_an_error() {
    let mut client = Endpoint::new(Config::default());
    let mut server = Endpoint::new(Config {
        max_inbound_calls: 2,
        ..Config::default()
    });
    let calls: Vec<CallId> = (0..3)
        .map(|_| client.start_call(PPROF, false, &pprof("")).unwrap())
        .collect();
    pump(&mut client, &mut server, 64);
    assert_eq!(events(&mut server).len(), 2);
    assert_eq!(server.metrics().requests_rejected, 1);
    pump(&mut server, &mut client, 64);
    assert_eq!(
        responses(&mut client, calls[2]),
        [Response::Failed(CallError::Remote(RpcError::new(
            0,
            "too many concurrent calls"
        )))]
    );
}

#[test]
fn calls_over_the_outbound_limit_fail_locally() {
    let mut client = Endpoint::new(Config {
        max_outbound_calls: 1,
        ..Config::default()
    });
    let call = client.start_call(PPROF, false, &pprof("")).unwrap();
    assert_eq!(
        client.start_call(PPROF, false, &pprof("")),
        Err(CallError::TooManyCalls)
    );
    client.cancel(call);
    client.start_call(PPROF, false, &pprof("")).unwrap();
}

/// A request Call of exactly `len` bytes.
fn request_of_len(len: usize) -> Call {
    let mut c = Call {
        id: 1,
        method: PPROF.into(),
        ..Default::default()
    };
    let base = c.encoded_len();
    // the argument field adds a key, a length prefix and the bytes
    let mut arg = len - base - 2;
    while base + 1 + crate::wire::uvarint_len(arg as u64) + arg > len {
        arg -= 1;
    }
    c.argument = vec![0; arg];
    assert_eq!(c.encoded_len(), len);
    c
}

#[test]
fn a_call_of_the_maximum_length_passes_and_one_byte_more_closes() {
    let config = Config {
        max_message_bytes: 1000,
        ..Config::default()
    };
    let mut server = Endpoint::new(config.clone());
    server
        .handle_input(&delimited(&request_of_len(1000)))
        .unwrap();
    assert_eq!(events(&mut server).len(), 1);

    let mut client = Endpoint::new(config);
    let call = client.start_call(PPROF, false, &pprof("")).unwrap();
    let err = client.handle_input(&delimited(&request_of_len(1001)));
    assert_eq!(
        err,
        Err(Error::Frame(FrameError::TooLarge {
            len: 1001,
            max: 1000
        }))
    );
    assert!(client.is_closed());
    assert_eq!(
        responses(&mut client, call),
        [Response::Failed(CallError::Disconnected)]
    );
    assert_eq!(client.handle_input(&[0]), Err(Error::Closed));
}

#[test]
fn an_overlong_length_prefix_closes() {
    let mut server = Endpoint::new(Config::default());
    assert_eq!(
        server.handle_input(&[0x80; 11]),
        Err(Error::Frame(FrameError::MalformedLength))
    );
    assert!(server.is_closed());
}

#[test]
fn malformed_calls_are_skipped() {
    let mut server = Endpoint::new(Config::default());
    let mut data = vec![2, 0xff, 0xff];
    data.extend(delimited(&Call {
        id: 1,
        method: PPROF.into(),
        ..Default::default()
    }));
    server.handle_input(&data).unwrap();
    assert_eq!(events(&mut server).len(), 1);
    assert_eq!(server.metrics().malformed, 1);
}

#[test]
fn every_call_sent_gets_a_fresh_id_from_one() {
    let mut a = Endpoint::new(Config::default());
    let mut b = Endpoint::new(Config::default());
    let c1 = a.start_call(PPROF, false, &pprof("")).unwrap();
    let c2 = a.start_call(PPROF, false, &pprof("")).unwrap();
    assert_eq!((c1.get(), c2.get()), (1, 2));
    pump(&mut a, &mut b, 64);
    let reqs: Vec<InboundId> = events(&mut b)
        .into_iter()
        .map(|e| match e {
            Event::Request { call, .. } => call,
            e => panic!("{e:?}"),
        })
        .collect();
    // b's own call and its answers share b's counter
    b.start_call(PPROF, false, &pprof("")).unwrap();
    b.reply_value(reqs[1], &PProfResponse::default());
    b.reply_value(reqs[0], &PProfResponse::default());
    let calls = sent(&mut b);
    let ids: Vec<(u64, u64, &str)> = calls
        .iter()
        .map(|c| (c.id, c.parent_id, c.method.as_str()))
        .collect();
    assert_eq!(ids, [(1, 0, PPROF), (2, 2, CALLBACK), (3, 1, CALLBACK)]);
}

#[test]
fn encoded_requests_go_out_as_their_messages_do() {
    let mut e = Endpoint::new(Config::default());
    let req = pprof("heap");
    e.start_call(PPROF, false, &req).unwrap();
    e.start_call(PPROF, false, &Encoded(&req.encode_to_vec()))
        .unwrap();
    let calls = sent(&mut e);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].method, calls[0].method);
    assert_eq!(calls[1].argument, calls[0].argument);
}

#[test]
fn a_request_is_queued_until_it_is_transmitted() {
    let mut e = Endpoint::new(Config::default());
    let c1 = e.start_call(PPROF, false, &pprof("1")).unwrap();
    let c2 = e.start_call(PPROF, false, &pprof("2")).unwrap();
    assert!(e.is_request_queued(c1) && e.is_request_queued(c2));
    e.poll_transmit().unwrap();
    assert!(!e.is_request_queued(c1) && e.is_request_queued(c2));

    // a cancel queued between requests holds its own place
    e.cancel(c1);
    let c3 = e.start_call(PPROF, false, &pprof("3")).unwrap();
    let queued: Vec<(bool, bool)> = std::iter::from_fn(|| {
        e.poll_transmit()?;
        Some((e.is_request_queued(c2), e.is_request_queued(c3)))
    })
    .collect();
    assert_eq!(queued, [(false, true), (false, true), (false, false)]);

    let c4 = e.start_call(PPROF, false, &pprof("4")).unwrap();
    assert!(e.is_request_queued(c4));
    e.close();
    assert!(!e.is_request_queued(c4));
}

#[test]
fn part_of_a_call_counts_as_receiving() {
    let mut client = Endpoint::new(Config::default());
    let call = client.start_call(PPROF, false, &pprof("")).unwrap();
    client.poll_transmit().unwrap();
    let answer = delimited(&callback(call.get(), Kind::CallKindDefault, vec![7; 300]));
    let (head, tail) = answer.split_at(100);
    assert!(!client.is_receiving());
    client.handle_input(&head[..1]).unwrap();
    assert!(client.is_receiving());
    client.handle_input(&head[1..]).unwrap();
    assert!(client.is_receiving());
    assert_eq!(client.received_bytes(), 100);
    client.handle_input(tail).unwrap();
    assert!(!client.is_receiving());
    assert_eq!(client.received_bytes(), answer.len() as u64);
    assert_eq!(events(&mut client), [Event::Response { call }]);
}

#[test]
fn responses_to_unknown_calls_are_dropped() {
    let mut client = Endpoint::new(Config::default());
    client
        .handle_input(&delimited(&callback(42, Kind::CallKindDefault, vec![1])))
        .unwrap();
    assert_eq!(events(&mut client), []);
    assert_eq!(client.metrics().dropped, 1);
}

#[test]
fn a_slow_stream_reader_overflows_its_call_only() {
    // room for two items of 6 bytes, at 70 each
    let mut client = Endpoint::new(Config {
        max_message_bytes: 100,
        max_queued_response_bytes: 0,
        ..Config::default()
    });
    assert_eq!(client.config().max_queued_response_bytes, 164);
    let slow = client.start_call(WATCH, true, &pprof("")).unwrap();
    let other = client.start_call(PPROF, false, &pprof("")).unwrap();
    sent(&mut client);
    for n in 0..4 {
        let res = callback(slow.get(), Kind::CallKindDefault, item(n).encode_to_vec());
        client.handle_input(&delimited(&res)).unwrap();
    }
    // the cancel goes out at once
    let cancels = sent(&mut client);
    assert_eq!(cancels.len(), 1);
    assert_eq!(
        (cancels[0].method.as_str(), cancels[0].parent_id),
        (CANCEL, slow.get())
    );
    assert_eq!(
        responses(&mut client, slow),
        [
            item_response(0),
            item_response(1),
            Response::Failed(CallError::Overflow)
        ]
    );
    assert_eq!(client.metrics().overflows, 1);

    let res = callback(
        other.get(),
        Kind::CallKindDefault,
        PProfResponse::default().encode_to_vec(),
    );
    client.handle_input(&delimited(&res)).unwrap();
    assert_eq!(responses(&mut client, other), [Response::Item(vec![])]);
    assert_eq!(client.queued_response_bytes(), 0);
}

#[test]
fn a_lone_response_always_fits() {
    let mut client = Endpoint::new(Config {
        max_message_bytes: 100,
        max_queued_response_bytes: 0,
        ..Config::default()
    });
    let stream = client.start_call(WATCH, true, &pprof("")).unwrap();
    let unary = client.start_call(PPROF, false, &pprof("")).unwrap();
    sent(&mut client);
    for _ in 0..2 {
        let res = callback(stream.get(), Kind::CallKindDefault, vec![0; 60]);
        client.handle_input(&delimited(&res)).unwrap();
    }
    // as large as fits in a Call of max_message_bytes
    let res = callback(unary.get(), Kind::CallKindDefault, vec![0; 80]);
    assert!(res.encoded_len() <= 100);
    client.handle_input(&delimited(&res)).unwrap();
    // the stream overflowed on its second item; the unary call didn't,
    // though both together are past the limit
    assert_eq!(sent(&mut client).len(), 1);
    assert_eq!(
        client.queued_response_bytes(),
        (60 + RESPONSE_OVERHEAD) + RESPONSE_OVERHEAD + (80 + RESPONSE_OVERHEAD)
    );
    assert_eq!(responses(&mut client, unary), [Response::Item(vec![0; 80])]);
    client.cancel(stream);
    assert_eq!(client.queued_response_bytes(), 0);
}

#[test]
fn oversized_replies_become_errors() {
    let mut client = Endpoint::new(Config::default());
    let mut server = Endpoint::new(Config {
        max_send_bytes: 100,
        ..Config::default()
    });
    let unary = client.start_call(PPROF, false, &pprof("")).unwrap();
    let stream = client.start_call(WATCH, true, &pprof("")).unwrap();
    pump(&mut client, &mut server, 64);
    let reqs: Vec<InboundId> = events(&mut server)
        .into_iter()
        .map(|e| match e {
            Event::Request { call, .. } => call,
            e => panic!("{e:?}"),
        })
        .collect();
    let big = PProfResponse {
        name: String::new(),
        data: vec![0; 200],
    };
    assert!(!server.reply_value(reqs[0], &big));
    assert!(server.reply_item(reqs[1], &item(1)));
    let big_item = WatchMetricsResponse { data: vec![0; 200] };
    assert!(!server.reply_item(reqs[1], &big_item));
    assert!(!server.is_serving(reqs[1]));
    assert_eq!(server.metrics().oversized_replies, 2);
    pump(&mut server, &mut client, 64);

    let too_large = |len| {
        Response::Failed(CallError::Remote(RpcError::new(
            0,
            format!("response too large: {len} bytes"),
        )))
    };
    assert_eq!(responses(&mut client, unary), [too_large(220)]);
    assert_eq!(
        responses(&mut client, stream),
        [item_response(1), too_large(220)]
    );
}

#[test]
fn oversized_requests_fail_locally() {
    let mut client = Endpoint::new(Config {
        max_send_bytes: 100,
        ..Config::default()
    });
    let err = client.start_call(PPROF, false, &pprof(&"x".repeat(200)));
    assert_eq!(err, Err(CallError::TooLarge { len: 237, max: 100 }));
    assert_eq!(client.poll_transmit(), None);
}

#[test]
fn message_framing_carries_one_unprefixed_call_per_message() {
    let config = Config {
        framing: Framing::Message,
        ..Config::default()
    };
    let mut client = Endpoint::new(config.clone());
    let mut server = Endpoint::new(config);
    let call = client.start_call(PPROF, false, &pprof("a")).unwrap();
    let frame = client.poll_transmit().unwrap();
    assert_eq!(
        Call::decode(frame.as_slice()).unwrap().method,
        PPROF,
        "no prefix"
    );
    server.handle_input(&frame).unwrap();
    let (req, ..) = request(&mut server);
    server.reply_value(req, &PProfResponse::default());
    pump(&mut server, &mut client, 1);
    assert_eq!(responses(&mut client, call), [Response::Item(vec![])]);
}

#[test]
fn message_framing_rejects_oversized_messages() {
    let mut server = Endpoint::new(Config {
        framing: Framing::Message,
        max_message_bytes: 10,
        ..Config::default()
    });
    assert_eq!(
        server.handle_input(&[0; 11]),
        Err(Error::MessageTooLarge { len: 11, max: 10 })
    );
}

// --- remotes ------------------------------------------------------------------

fn message_config() -> Config {
    Config {
        framing: Framing::Message,
        ..Config::default()
    }
}

fn request_call(id: u64, method: &str, arg: &impl Message) -> Call {
    Call {
        id,
        method: method.into(),
        argument: arg.encode_to_vec(),
        ..Default::default()
    }
}

/// Decodes the Calls `e` has to send, with where they go.
fn sent_to(e: &mut Endpoint) -> Vec<(RemoteId, Call)> {
    std::iter::from_fn(|| e.poll_transmit_to())
        .map(|(r, f)| (r, Call::decode(f.as_slice()).unwrap()))
        .collect()
}

const A: RemoteId = RemoteId(1);
const B: RemoteId = RemoteId(2);

#[test]
fn requests_are_keyed_by_remote_and_caller_id() {
    let mut server = Endpoint::new(message_config());
    let cert = Rc::new(Certificate {
        body: vec![1; 32],
        ..Default::default()
    });
    server.handle_call(A, request_call(7, PPROF, &pprof("a")), Some(cert.clone()));
    server.handle_call(B, request_call(7, PPROF, &pprof("b")), None);
    // a duplicate from the same remote is dropped
    server.handle_call(A, request_call(7, PPROF, &pprof("a")), None);
    assert_eq!(server.metrics().dropped, 1);

    let reqs: Vec<_> = events(&mut server)
        .into_iter()
        .map(|e| match e {
            Event::Request {
                call,
                from,
                id,
                certificate,
                ..
            } => (call, from, id, certificate),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(reqs.len(), 2);
    assert_eq!(
        (reqs[0].1, reqs[0].2, reqs[0].3.clone()),
        (A, 7, Some(cert))
    );
    assert_eq!((reqs[1].1, reqs[1].2, reqs[1].3.clone()), (B, 7, None));

    // a cancel reaches only its own remote's request
    server.handle_call(
        B,
        Call {
            id: 8,
            parent_id: 7,
            method: CANCEL.into(),
            ..Default::default()
        },
        None,
    );
    assert_eq!(events(&mut server), [Event::Cancelled { call: reqs[1].0 }]);
    assert!(server.reply_value(reqs[0].0, &PProfResponse::default()));
    let out = sent_to(&mut server);
    assert_eq!(out.len(), 1);
    assert_eq!((out[0].0, out[0].1.parent_id), (A, 7));
}

#[test]
fn responses_from_another_remote_are_dropped() {
    let mut client = Endpoint::new(message_config());
    let call = client.start_call_to(A, PPROF, false, &pprof("")).unwrap();
    let (to, req) = sent_to(&mut client).remove(0);
    assert_eq!(to, A);
    let value = PProfResponse::default().encode_to_vec();
    client.handle_call(
        B,
        callback(req.id, Kind::CallKindDefault, value.clone()),
        None,
    );
    assert_eq!(client.metrics().dropped, 1);
    assert_eq!(client.poll_response(call), None);
    client.handle_call(
        A,
        callback(req.id, Kind::CallKindDefault, value.clone()),
        None,
    );
    assert_eq!(responses(&mut client, call), [Response::Item(value)]);
}

#[test]
fn one_remote_cannot_hold_every_request_slot() {
    let mut server = Endpoint::new(Config {
        max_inbound_calls: 4,
        max_inbound_calls_per_remote: 2,
        ..message_config()
    });
    for id in 1..=3 {
        server.handle_call(A, request_call(id, PPROF, &pprof("")), None);
    }
    server.handle_call(B, request_call(1, PPROF, &pprof("")), None);
    assert_eq!(server.metrics().requests, 3);
    assert_eq!(server.metrics().requests_rejected, 1);
    let rejected: Vec<_> = sent_to(&mut server)
        .into_iter()
        .map(|(r, c)| (r, c.parent_id, c.kind))
        .collect();
    assert_eq!(rejected, [(A, 3, Kind::CallKindError as i32)]);
}

#[test]
fn a_remote_is_idle_once_its_last_reply_is_sent() {
    let mut server = Endpoint::new(message_config());
    assert!(server.is_remote_idle(A));
    server.handle_call(A, request_call(1, PPROF, &pprof("")), None);
    assert!(!server.is_remote_idle(A));
    let (req, ..) = request(&mut server);
    server.reply_value(req, &PProfResponse::default());
    assert!(!server.is_remote_idle(A), "the reply is still queued");
    assert_eq!(sent_to(&mut server).len(), 1);
    assert!(server.is_remote_idle(A));

    let mut client = Endpoint::new(message_config());
    let call = client.start_call_to(B, PPROF, false, &pprof("")).unwrap();
    let (_, req) = sent_to(&mut client).remove(0);
    assert!(!client.is_remote_idle(B), "the call is open");
    client.handle_call(B, callback(req.id, Kind::CallKindDefault, vec![]), None);
    assert!(!client.is_remote_idle(B), "the response is unread");
    responses(&mut client, call);
    assert!(client.is_remote_idle(B));
}

#[test]
fn headers_go_on_every_call_and_count_towards_the_send_limit() {
    let headers = BTreeMap::from([("certificate".to_owned(), vec![5; 100])]);
    let mut client = Endpoint::new(message_config());
    let mut server = Endpoint::new(message_config());
    client.set_headers(&headers);
    server.set_headers(&headers);

    let call = client.start_call_to(A, WATCH, true, &pprof("")).unwrap();
    for (_, c) in sent_to(&mut client) {
        server.handle_call(A, c, None);
    }
    let (req, ..) = request(&mut server);
    server.reply_item(req, &item(1));
    let out = sent_to(&mut server);
    client.cancel(call);
    let cancel = sent_to(&mut client);
    assert_eq!(cancel[0].1.method, CANCEL);
    for (_, c) in out.iter().chain(&cancel) {
        assert_eq!(c.headers, headers);
    }

    let len = client.send_len(0, PPROF, Kind::CallKindDefault, pprof("").encoded_len());
    let mut tight = Endpoint::new(Config {
        max_send_bytes: len - 1,
        ..message_config()
    });
    tight.set_headers(&headers);
    assert_eq!(
        tight.start_call(PPROF, false, &pprof("")),
        Err(CallError::TooLarge { len, max: len - 1 })
    );
}

/// The recorded `rpc_*` vectors in strims-proto's `golden/vectors.hex`.
fn golden(name: &str) -> Vec<u8> {
    let vectors = include_str!("../../../strims-proto/golden/vectors.hex");
    let line = vectors
        .lines()
        .find(|l| l.split(' ').next() == Some(name))
        .unwrap_or_else(|| panic!("no vector {name}"));
    hex::decode(line.split(' ').nth(1).unwrap()).unwrap()
}

#[test]
fn recorded_calls_are_understood() {
    let config = Config {
        framing: Framing::Message,
        ..Config::default()
    };

    // a request and its cancel (sent with the default kind)
    let mut server = Endpoint::new(config.clone());
    server.handle_input(&golden("rpc_call_request")).unwrap();
    let (req, id, method, arg) = request(&mut server);
    assert_eq!((id, method.as_str()), (1, PPROF));
    assert_eq!(PProfRequest::decode(arg.as_slice()).unwrap().name, "heap");
    server
        .handle_input(&golden("rpc_call_cancel_default_kind"))
        .unwrap();
    assert_eq!(events(&mut server), [Event::Cancelled { call: req }]);

    // answers to call 1
    let answers = [
        ("rpc_call_callback", None),
        (
            "rpc_call_error",
            Some(CallError::Remote(RpcError::new(5, "boom"))),
        ),
        ("rpc_call_undefined", Some(CallError::Undefined)),
        ("rpc_call_close", Some(CallError::Closed)),
    ];
    for (name, want) in answers {
        let mut client = Endpoint::new(config.clone());
        let call = client.start_call(PPROF, false, &pprof("heap")).unwrap();
        client.handle_input(&golden(name)).unwrap();
        let got = responses(&mut client, call);
        match want {
            None => {
                let [Response::Item(b)] = got.as_slice() else {
                    panic!("{name}: {got:?}");
                };
                assert_eq!(PProfResponse::decode(b.as_slice()).unwrap().data.len(), 40);
            }
            Some(e) => assert_eq!(got, [Response::Failed(e)], "{name}"),
        }
    }

    // our request encodes as recorded
    let mut client = Endpoint::new(config);
    let req = PProfRequest {
        name: "heap".into(),
        debug: true,
        gc: false,
    };
    client.start_call(PPROF, false, &req).unwrap();
    assert_eq!(client.poll_transmit().unwrap(), golden("rpc_call_request"));
}

/// Runs a fixed exchange and returns every byte both sides sent.
fn script() -> Vec<u8> {
    let mut log = Vec::new();
    let mut client = Endpoint::new(Config::default());
    let mut server = Endpoint::new(Config::default());
    let calls: Vec<CallId> = (0..5)
        .map(|i| {
            client
                .start_call(
                    if i % 2 == 0 { PPROF } else { WATCH },
                    i % 2 == 1,
                    &pprof("s"),
                )
                .unwrap()
        })
        .collect();
    while let Some(f) = client.poll_transmit() {
        log.extend_from_slice(&f);
        server.handle_input(&f).unwrap();
    }
    for e in events(&mut server) {
        if let Event::Request { call, method, .. } = e {
            if method == WATCH {
                server.reply_item(call, &item(1));
                server.reply_close(call);
            } else {
                server.reply_value(call, &PProfResponse::default());
            }
        }
    }
    client.cancel(calls[1]);
    for e in [&mut server, &mut client] {
        while let Some(f) = e.poll_transmit() {
            log.extend_from_slice(&f);
        }
    }
    log
}

#[test]
fn the_same_exchange_sends_the_same_bytes() {
    assert_eq!(script(), script());
}

#[test]
fn closing_fails_calls_and_cancels_requests() {
    let mut a = Endpoint::new(Config::default());
    let mut b = Endpoint::new(Config::default());
    let call = a.start_call(WATCH, true, &pprof("")).unwrap();
    b.start_call(PPROF, false, &pprof("")).unwrap();
    pump(&mut b, &mut a, 64);
    let (req, ..) = request(&mut a);

    a.close();
    assert!(a.is_closed());
    assert_eq!(a.poll_transmit(), None);
    assert_eq!(
        events(&mut a),
        [Event::Response { call }, Event::Cancelled { call: req }]
    );
    assert_eq!(
        responses(&mut a, call),
        [Response::Failed(CallError::Disconnected)]
    );
    assert!(!a.reply_value(req, &PProfResponse::default()));
    assert_eq!(
        a.start_call(PPROF, false, &pprof("")),
        Err(CallError::Disconnected)
    );
}

#[test]
fn congestion_starts_at_the_high_water_mark() {
    let mut client = Endpoint::new(Config {
        transmit_high_water: 100,
        ..Config::default()
    });
    client
        .start_call(PPROF, false, &pprof(&"x".repeat(40)))
        .unwrap();
    assert!(!client.is_congested());
    client
        .start_call(PPROF, false, &pprof(&"x".repeat(40)))
        .unwrap();
    assert!(client.is_congested());
    client.poll_transmit();
    assert!(!client.is_congested());
    assert_eq!(client.transmit_buffered(), 76);
}

/// The per-remote counts, recomputed from the call tables.
fn recount(e: &Endpoint) -> BTreeMap<RemoteId, RemoteRefs> {
    let mut m: BTreeMap<RemoteId, RemoteRefs> = BTreeMap::new();
    for &(r, _) in e.inbound.keys() {
        let c = m.entry(r).or_default();
        c.refs += 1;
        c.inbound += 1;
    }
    for out in e.outbound.values() {
        m.entry(out.remote).or_default().refs += 1;
    }
    for (r, _) in &e.transmit {
        m.entry(*r).or_default().refs += 1;
    }
    m
}

proptest! {
    #[test]
    fn endpoint_survives_arbitrary_input(
        data in proptest::collection::vec(any::<u8>(), 0..2048),
        cuts in proptest::collection::vec(1usize..64, 1..16),
        message in any::<bool>(),
    ) {
        let config = Config {
            framing: if message { Framing::Message } else { Framing::Delimited },
            max_message_bytes: 512,
            max_inbound_calls: 4,
            max_queued_response_bytes: 0,
            ..Config::default()
        };
        let mut e = Endpoint::new(config);
        let calls: Vec<CallId> = (0..3)
            .map(|i| e.start_call_to(RemoteId(i), WATCH, i != 0, &pprof("")).unwrap())
            .collect();
        let mut rest = data.as_slice();
        for cut in cuts.iter().cycle() {
            if rest.is_empty() || e.is_closed() {
                break;
            }
            let (head, tail) = rest.split_at((*cut).min(rest.len()));
            rest = tail;
            match Call::decode(head) {
                // decoded Calls arrive from one of a few remotes
                Ok(call) if message => {
                    e.handle_call(RemoteId(*cut as u64 % 3), call, None);
                }
                _ => {
                    let _ = e.handle_input(head);
                }
            }
            prop_assert!(e.decoder.buffered() <= 512 + crate::wire::MAX_VARINT_LEN);
            prop_assert!(e.inbound.len() <= 4);
            prop_assert_eq!(&e.remotes, &recount(&e));
            for out in e.outbound.values() {
                prop_assert!(
                    out.queued <= e.config.max_queued_response_bytes + RESPONSE_OVERHEAD
                );
            }
            while let Some(ev) = e.poll_event() {
                if let Event::Request { call, .. } = ev {
                    e.reply_value(call, &PProfResponse::default());
                }
            }
            prop_assert_eq!(&e.remotes, &recount(&e));
            while e.poll_transmit().is_some() {}
        }
        for call in calls {
            while e.poll_response(call).is_some() {}
        }
        prop_assert_eq!(e.queued_response_bytes(), 0);
        prop_assert_eq!(&e.remotes, &recount(&e));
    }
}
