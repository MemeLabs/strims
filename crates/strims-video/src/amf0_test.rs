// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::*;

fn bytes(v: &Value<'_>) -> Vec<u8> {
    let mut out = Vec::new();
    encode(v, &mut out).unwrap();
    out
}

fn decode_one(b: &[u8]) -> Result<Value<'_>, Amf0Error> {
    let mut d = Decoder::new(b);
    let v = d.decode()?;
    assert!(d.is_empty(), "trailing bytes after {v:?}");
    Ok(v)
}

fn sample() -> Value<'static> {
    Value::Object(vec![
        ("number", Value::Number(-1.5)),
        ("nan-free", Value::Number(f64::MAX)),
        ("yes", Value::Boolean(true)),
        ("no", Value::Boolean(false)),
        ("text", Value::String("héllo")),
        ("empty", Value::String("")),
        ("null", Value::Null),
        ("undefined", Value::Undefined),
        (
            "ecma",
            Value::EcmaArray(vec![("a", Value::Number(1.0)), ("b", Value::String("x"))]),
        ),
        (
            "strict",
            Value::StrictArray(vec![Value::Null, Value::Number(2.0), Value::Object(vec![])]),
        ),
        (
            "date",
            Value::Date {
                millis: 1_700_000_000_000.0,
                timezone: 0,
            },
        ),
        (
            "nested",
            Value::Object(vec![("deeper", Value::Object(vec![]))]),
        ),
    ])
}

#[test]
fn values_round_trip() {
    let v = sample();
    let b = bytes(&v);
    assert_eq!(decode_one(&b).unwrap(), v);
    // Several values in one message, as a command carries them.
    let mut message = bytes(&Value::String("connect"));
    message.extend(bytes(&Value::Number(1.0)));
    message.extend(&b);
    assert_eq!(
        decode_all(&message).unwrap(),
        vec![Value::String("connect"), Value::Number(1.0), v]
    );
}

#[test]
fn encodings_match_the_format() {
    assert_eq!(
        bytes(&Value::Number(1.0)),
        [0x00, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(bytes(&Value::Boolean(true)), [0x01, 0x01]);
    assert_eq!(bytes(&Value::String("ab")), [0x02, 0, 2, b'a', b'b']);
    assert_eq!(bytes(&Value::Null), [0x05]);
    assert_eq!(bytes(&Value::Undefined), [0x06]);
    assert_eq!(
        bytes(&Value::Object(vec![("k", Value::Null)])),
        [0x03, 0, 1, b'k', 0x05, 0, 0, 0x09]
    );
    assert_eq!(
        bytes(&Value::EcmaArray(vec![("k", Value::Null)])),
        [0x08, 0, 0, 0, 1, 0, 1, b'k', 0x05, 0, 0, 0x09]
    );
    assert_eq!(
        bytes(&Value::StrictArray(vec![Value::Null])),
        [0x0a, 0, 0, 0, 1, 0x05]
    );
    assert_eq!(
        bytes(&Value::Date {
            millis: 0.0,
            timezone: -1
        }),
        [0x0b, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff]
    );
}

#[test]
fn long_strings_decode_and_encode_past_u16() {
    let text = "x".repeat(70_000);
    let b = bytes(&Value::String(&text));
    assert_eq!(b[0], LONG_STRING);
    assert_eq!(&b[1..5], 70_000u32.to_be_bytes());
    // 70 000 bytes exceeds the long string bound.
    assert_eq!(decode_one(&b), Err(Amf0Error::LongString(70_000)));

    let short = "y".repeat(10);
    let mut long = vec![LONG_STRING];
    long.extend(10u32.to_be_bytes());
    long.extend(short.as_bytes());
    assert_eq!(decode_one(&long).unwrap(), Value::String(&short));

    let limit = "z".repeat(MAX_LONG_STRING);
    let mut at_limit = vec![LONG_STRING];
    at_limit.extend((MAX_LONG_STRING as u32).to_be_bytes());
    at_limit.extend(limit.as_bytes());
    assert_eq!(decode_one(&at_limit).unwrap(), Value::String(&limit));
    at_limit[1..5].copy_from_slice(&(MAX_LONG_STRING as u32 + 1).to_be_bytes());
    at_limit.push(b'z');
    assert_eq!(
        decode_one(&at_limit),
        Err(Amf0Error::LongString(MAX_LONG_STRING as u32 + 1))
    );
}

#[test]
fn refused_markers_error() {
    let cases = [
        (vec![AVMPLUS, 0x01], Amf0Error::Amf3),
        (vec![REFERENCE, 0, 0], Amf0Error::Reference),
        (vec![XML_DOCUMENT, 0, 0, 0, 0], Amf0Error::Xml),
        (
            vec![TYPED_OBJECT, 0, 1, b'T', 0, 0, 9],
            Amf0Error::TypedObject,
        ),
        (vec![0x04], Amf0Error::Marker(0x04)),
        (vec![0x0d], Amf0Error::Marker(0x0d)),
        (vec![0x0e], Amf0Error::Marker(0x0e)),
        (vec![OBJECT_END], Amf0Error::Marker(OBJECT_END)),
        (vec![0x12], Amf0Error::Marker(0x12)),
        (vec![0xff], Amf0Error::Marker(0xff)),
    ];
    for (b, e) in cases {
        assert_eq!(decode_one(&b), Err(e), "{b:02x?}");
    }
    // Inside a container, too.
    let mut b = vec![OBJECT, 0, 1, b'k', AVMPLUS];
    b.extend([0, 0, OBJECT_END]);
    assert_eq!(decode_one(&b), Err(Amf0Error::Amf3));
    assert_eq!(Amf0Error::Amf3.to_string(), "AMF3 values are not supported");
}

#[test]
fn objects_end_only_at_the_end_marker() {
    // An empty key must be followed by the end marker.
    assert_eq!(decode_one(&[OBJECT, 0, 0, NULL]), Err(Amf0Error::EmptyKey));
    // An object without its end is truncated.
    assert_eq!(
        decode_one(&[OBJECT, 0, 1, b'k', NULL]),
        Err(Amf0Error::Truncated)
    );
    // An ECMA array's count is a hint: here it claims far more.
    let b = [
        ECMA_ARRAY, 0xff, 0xff, 0xff, 0xff, 0, 1, b'k', NULL, 0, 0, OBJECT_END,
    ];
    assert_eq!(
        decode_one(&b).unwrap(),
        Value::EcmaArray(vec![("k", Value::Null)])
    );
}

#[test]
fn every_truncation_errors() {
    let b = bytes(&sample());
    for n in 0..b.len() {
        assert_eq!(decode_one(&b[..n]), Err(Amf0Error::Truncated), "at {n}");
    }
}

#[test]
fn strings_must_be_utf8() {
    assert_eq!(decode_one(&[STRING, 0, 1, 0xff]), Err(Amf0Error::Utf8));
    assert_eq!(
        decode_one(&[LONG_STRING, 0, 0, 0, 1, 0xc3]),
        Err(Amf0Error::Utf8)
    );
    assert_eq!(
        decode_one(&[OBJECT, 0, 1, 0x80, NULL, 0, 0, OBJECT_END]),
        Err(Amf0Error::Utf8)
    );
}

fn nested(
    depth: usize,
    leaf: Value<'static>,
    wrap: fn(Value<'static>) -> Value<'static>,
) -> Value<'static> {
    (0..depth).fold(leaf, |v, _| wrap(v))
}

#[test]
fn containers_nest_at_most_max_depth() {
    let wraps: [fn(Value<'static>) -> Value<'static>; 3] = [
        |v| Value::Object(vec![("k", v)]),
        |v| Value::EcmaArray(vec![("k", v)]),
        |v| Value::StrictArray(vec![v]),
    ];
    for wrap in wraps {
        let ok = nested(MAX_DEPTH, Value::Null, wrap);
        assert_eq!(decode_one(&bytes(&ok)).unwrap(), ok);
        let deep = nested(MAX_DEPTH + 1, Value::Null, wrap);
        assert_eq!(decode_one(&bytes(&deep)), Err(Amf0Error::Depth));
    }
    // An empty container at the limit counts too.
    let deep = nested(MAX_DEPTH, Value::Object(vec![]), |v| {
        Value::StrictArray(vec![v])
    });
    assert_eq!(decode_one(&bytes(&deep)), Err(Amf0Error::Depth));
}

#[test]
fn a_message_holds_at_most_max_items() {
    // The array itself and its elements are values.
    let ok = Value::StrictArray(vec![Value::Null; MAX_ITEMS - 1]);
    assert_eq!(decode_one(&bytes(&ok)).unwrap(), ok);
    let over = Value::StrictArray(vec![Value::Null; MAX_ITEMS]);
    assert_eq!(decode_one(&bytes(&over)), Err(Amf0Error::Items));
    // The bound spans the values of one decoder.
    let mut message = vec![NULL; MAX_ITEMS];
    assert_eq!(decode_all(&message).unwrap().len(), MAX_ITEMS);
    message.push(NULL);
    assert_eq!(decode_all(&message), Err(Amf0Error::Items));
    // A strict array's count never sizes the work: a huge count stops at
    // the bound or the input's end.
    let b = [STRICT_ARRAY, 0xff, 0xff, 0xff, 0xff, NULL];
    assert_eq!(decode_one(&b), Err(Amf0Error::Truncated));
    let mut b = vec![STRICT_ARRAY, 0xff, 0xff, 0xff, 0xff];
    b.extend([NULL; MAX_ITEMS]);
    assert_eq!(decode_one(&b), Err(Amf0Error::Items));
}

#[test]
fn encoding_refuses_keys_past_u16() {
    let key = "k".repeat(70_000);
    let mut out = Vec::new();
    assert_eq!(
        encode(&Value::Object(vec![(&key, Value::Null)]), &mut out),
        Err(Amf0Error::KeyTooLong(70_000))
    );
}

#[test]
fn metadata_reads_its_fields() {
    let v = Value::EcmaArray(vec![
        ("duration", Value::Number(0.0)),
        ("width", Value::Number(1280.0)),
        ("height", Value::Number(720.0)),
        ("framerate", Value::Number(29.97)),
        ("videodatarate", Value::Number(2500.0)),
        ("videocodecid", Value::Number(7.0)),
        ("audiodatarate", Value::Number(160.0)),
        ("audiocodecid", Value::Number(10.0)),
        ("audiosamplerate", Value::Number(48000.0)),
        ("encoder", Value::String("obs-output module")),
    ]);
    assert_eq!(
        Metadata::from_value(&v),
        Some(Metadata {
            width: Some(1280.0),
            height: Some(720.0),
            framerate: Some(29.97),
            videodatarate: Some(2500.0),
            audiodatarate: Some(160.0),
            videocodecid: Some(7.0),
            audiocodecid: Some(10.0),
            audiosamplerate: Some(48000.0),
            encoder: Some("obs-output module".to_owned()),
        })
    );
    // Objects too; fields of another type are left out.
    let v = Value::Object(vec![
        ("width", Value::String("1280")),
        ("framerate", Value::Number(30.0)),
        ("encoder", Value::Number(1.0)),
    ]);
    assert_eq!(
        Metadata::from_value(&v),
        Some(Metadata {
            framerate: Some(30.0),
            ..Metadata::default()
        })
    );
    assert_eq!(Metadata::from_value(&Value::Null), None);
    assert_eq!(Metadata::from_value(&Value::StrictArray(vec![])), None);
}

fn proptest_config() -> Config {
    Config {
        cases: 1024,
        rng_seed: RngSeed::Fixed(0xa3f0_5eed),
        failure_persistence: None,
        ..Config::default()
    }
}

/// Bytes that start like each container and continue arbitrarily.
fn message() -> impl Strategy<Value = Vec<u8>> {
    let prefixes = vec![
        vec![],
        vec![OBJECT],
        vec![ECMA_ARRAY, 0, 0, 0, 1],
        vec![STRICT_ARRAY, 0, 0, 0, 3],
        vec![OBJECT, 0, 1, b'k', OBJECT],
        bytes(&sample()),
    ];
    (
        prop::sample::select(prefixes),
        proptest::collection::vec(any::<u8>(), 0..256),
    )
        .prop_map(|(mut p, tail)| {
            p.extend(tail);
            p
        })
}

/// The values `v` decodes as, itself included.
fn items(v: &Value<'_>) -> usize {
    1 + match v {
        Value::Object(p) | Value::EcmaArray(p) => p.iter().map(|(_, v)| items(v)).sum(),
        Value::StrictArray(a) => a.iter().map(items).sum(),
        _ => 0,
    }
}

fn arb_value() -> impl Strategy<Value = Value<'static>> {
    let leaf = prop_oneof![
        any::<f64>()
            .prop_filter("NaN never equals itself", |f| !f.is_nan())
            .prop_map(Value::Number),
        any::<bool>().prop_map(Value::Boolean),
        prop::sample::select(vec!["", "a", "onMetaData", "ünïcödé"]).prop_map(Value::String),
        Just(Value::Null),
        Just(Value::Undefined),
        (any::<i32>(), any::<i16>()).prop_map(|(m, timezone)| Value::Date {
            millis: f64::from(m),
            timezone
        }),
    ];
    leaf.prop_recursive(MAX_DEPTH as u32, 64, 6, |inner| {
        let key = prop::sample::select(vec!["a", "width", "x y"]);
        prop_oneof![
            proptest::collection::vec((key.clone(), inner.clone()), 0..6).prop_map(Value::Object),
            proptest::collection::vec((key, inner.clone()), 0..6).prop_map(Value::EcmaArray),
            proptest::collection::vec(inner, 0..6).prop_map(Value::StrictArray),
        ]
    })
    .prop_filter("at most MAX_ITEMS values", |v| items(v) <= MAX_ITEMS)
}

proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn arbitrary_bytes_never_panic(b in message()) {
        let mut d = Decoder::new(&b);
        while !d.is_empty() {
            let before = d.remaining().len();
            if d.decode().is_err() {
                break;
            }
            prop_assert!(d.remaining().len() < before);
        }
    }

    #[test]
    fn arbitrary_values_round_trip(v in arb_value()) {
        let b = bytes(&v);
        prop_assert_eq!(decode_one(&b).unwrap(), v);
    }
}
