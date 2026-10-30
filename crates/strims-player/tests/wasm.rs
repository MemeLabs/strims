// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The module's JS API in a browser worker: the objects it answers have
//! the shapes `src/web/strims-player.d.ts` declares.
//!
//! `GECKODRIVER=<path> cargo test -p strims-player --target wasm32-unknown-unknown`

#![cfg(target_arch = "wasm32")]
// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use js_sys::{Array, Object, Reflect, Uint8Array};
use strims_base::time::Time;
use strims_chunkstream::chunked::{ChunkedRecordDecoder, Item};
use strims_player::wasm::JsRemuxer;
use strims_video::synth::script::{Script, ScriptConfig, TagKind};
use strims_video::units::MAX_RECORD_LEN;
use strims_video::units::record::{Record, parse_record};
use strims_video::{FlvSource, Muxer, SourceEvent, UnitWriter};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_dedicated_worker);

const FRAME: usize = 4096;

/// The unit-format records of a writer session of `config`'s script.
fn records(config: &ScriptConfig) -> Vec<Vec<u8>> {
    let mut source = FlvSource::new();
    let mut w = UnitWriter::new(FRAME).unwrap();
    for tag in &Script::new(config).tags {
        let at = Time::from_unix_millis(1_793_275_200_000 + i64::from(tag.timestamp));
        let event = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, at),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, at),
            TagKind::Metadata => continue,
        };
        match event.unwrap() {
            Some(SourceEvent::Config(c)) => w.set_config(&c).unwrap(),
            Some(SourceEvent::Unit(u)) => w.push(&u).unwrap(),
            None => {}
        }
    }
    w.finish().unwrap();
    let mut d = ChunkedRecordDecoder::new(FRAME, MAX_RECORD_LEN).unwrap();
    let (stream, mut at, mut out) = (w.output(), 0, Vec::new());
    loop {
        let (n, item) = d.decode(&stream[at..]);
        at += n;
        match item {
            Some(Item::Record { payload, .. }) => out.push(payload.to_vec()),
            Some(Item::Lost) => panic!("lost records"),
            None => return out,
        }
    }
}

fn config() -> ScriptConfig {
    ScriptConfig {
        duration_secs: 4,
        b_frames: true,
        ..ScriptConfig::default()
    }
}

fn get(o: &JsValue, key: &str) -> JsValue {
    Reflect::get(o, &JsValue::from_str(key)).unwrap()
}

fn keys(o: &JsValue) -> Vec<String> {
    let mut k: Vec<String> = Object::keys(o.unchecked_ref::<Object>())
        .iter()
        .map(|k| k.as_string().unwrap())
        .collect();
    k.sort();
    k
}

fn assert_keys(o: &JsValue, expected: &[&str]) {
    let mut e: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    e.sort();
    assert_eq!(keys(o), e);
}

fn number(o: &JsValue, key: &str) -> f64 {
    get(o, key)
        .as_f64()
        .unwrap_or_else(|| panic!("{key} is not a number"))
}

/// A `data` the caller owns: a Uint8Array over a buffer of exactly its
/// bytes.
fn assert_owned_bytes(o: &JsValue) -> usize {
    let data: Uint8Array = get(o, "data").dyn_into().unwrap();
    assert_eq!(data.byte_offset(), 0);
    assert_eq!(data.buffer().byte_length(), data.length());
    assert!(data.length() > 0);
    data.length() as usize
}

fn push_all(r: &mut JsRemuxer, records: &[Vec<u8>], skip_before: Option<usize>) -> Vec<JsValue> {
    let mut out = Vec::new();
    for (i, rec) in records.iter().enumerate() {
        let outputs: Array = r.push(rec, true, skip_before == Some(i));
        out.extend(outputs.iter());
    }
    out
}

#[wasm_bindgen_test]
fn inits_and_media_have_their_declared_shapes() {
    let mut r = JsRemuxer::new();
    let mut out = push_all(&mut r, &records(&config()), None);
    out.extend(r.finish().iter());
    let init = &out[0];
    assert_eq!(get(init, "type").as_string().unwrap(), "init");
    assert_keys(init, &["type", "data", "mime", "tracks", "compatible"]);
    assert_owned_bytes(init);
    assert_eq!(
        get(init, "mime").as_string().unwrap(),
        r#"video/mp4;codecs="avc1.64001f,mp4a.40.2""#
    );
    assert_eq!(get(init, "compatible").as_bool(), Some(false));
    let tracks: Array = get(init, "tracks").dyn_into().unwrap();
    assert_eq!(tracks.length(), 2);
    let video = tracks.get(0);
    assert_keys(
        &video,
        &["id", "kind", "codec", "timescale", "width", "height"],
    );
    assert_eq!(get(&video, "kind").as_string().unwrap(), "video");
    assert_eq!(get(&video, "codec").as_string().unwrap(), "avc1.64001f");
    assert_eq!(
        (number(&video, "width"), number(&video, "height")),
        (1280.0, 720.0)
    );
    let audio = tracks.get(1);
    assert_keys(
        &audio,
        &["id", "kind", "codec", "timescale", "sampleRate", "channels"],
    );
    assert_eq!(number(&audio, "sampleRate"), 48_000.0);
    assert_eq!(number(&audio, "channels"), 2.0);

    let mut units = 0;
    let mut bytes = 0;
    for media in &out[1..] {
        assert_eq!(get(media, "type").as_string().unwrap(), "media");
        assert_keys(media, &["type", "data", "units"]);
        bytes += assert_owned_bytes(media);
        let list: Array = get(media, "units").dyn_into().unwrap();
        assert!(list.length() > 0);
        for u in list.iter() {
            assert_keys(&u, &["track", "kind", "keyframe", "pts", "published"]);
            assert!(get(&u, "keyframe").as_bool().is_some());
            // The first GOP's B-frames present at about the edit list's start.
            assert!(number(&u, "pts") > -0.1);
            assert!(number(&u, "published") > 1.7e12);
            units += 1;
        }
    }
    let m = r.metrics();
    assert_eq!(number(&m, "units"), f64::from(units));
    assert_eq!(number(&m, "fragments"), (out.len() - 1) as f64);
    let init_len = Uint8Array::from(get(init, "data")).length() as f64;
    assert_eq!(number(&m, "outputBytes"), bytes as f64 + init_len);
    assert!(number(&m, "memoryBytes") > 0.0);
    let reader = get(&m, "reader");
    assert_eq!(number(&reader, "units"), f64::from(units));
    assert_eq!(number(&reader, "malformed"), 0.0);
}

#[wasm_bindgen_test]
fn gaps_discontinuities_and_restarts_have_their_declared_shapes() {
    let first = records(&config());
    // A record lost mid-stream, then a second session from its start.
    let lost = (first.len() / 2..first.len())
        .find(|&i| matches!(parse_record(&first[i]), Ok(Record::Unit { .. })))
        .unwrap();
    let mut stream: Vec<Vec<u8>> = first.clone();
    stream.remove(lost);
    let resume = stream.len();
    stream.extend(records(&config()));
    let mut r = JsRemuxer::new();
    let mut out = Vec::new();
    for (i, rec) in stream.iter().enumerate() {
        let skip = i == lost || i == resume;
        out.extend(r.push(rec, true, skip).iter());
    }
    let kind = |o: &JsValue| get(o, "type").as_string().unwrap();
    let disc: Vec<&JsValue> = out.iter().filter(|o| kind(o) == "discontinuity").collect();
    assert_eq!(disc.len(), 2);
    assert_keys(disc[0], &["type"]);
    let gap = out.iter().find(|o| kind(o) == "gap").unwrap();
    assert_keys(gap, &["type", "track", "kind", "missing"]);
    assert_eq!(number(gap, "missing"), 1.0);
    let restart = out.iter().find(|o| kind(o) == "restart").unwrap();
    assert_keys(restart, &["type", "offset"]);
    assert!((number(restart, "offset") - 4.0).abs() < 0.05);
    assert_eq!(out.iter().filter(|o| kind(o) == "init").count(), 1);
}

#[wasm_bindgen_test]
fn hostile_bytes_answer_without_throwing() {
    let mut r = JsRemuxer::new();
    let mut x: u32 = 0x9e37_79b9;
    for i in 0..200 {
        let len = (x % 3000) as usize;
        let data: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x as u8
            })
            .collect();
        let out = r.push(&data, i % 3 == 0, i % 17 == 0);
        for o in out.iter() {
            let t = get(&o, "type").as_string().unwrap();
            assert!(["init", "media", "gap", "discontinuity", "restart"].contains(&t.as_str()));
        }
    }
    let _ = r.finish();
    assert!(number(&r.metrics(), "messages") == 200.0);
}
