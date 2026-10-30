// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The module's API: the `Remuxer` class the player worker calls, as
//! `src/web/strims-player.d.ts` declares it.
//!
//! Outputs are plain objects, which a worker can post on (Firefox's
//! MediaSource lives in the window). Every `data` is a new `Uint8Array`
//! over an `ArrayBuffer` of exactly its bytes, copied out of the module's
//! memory, so the caller owns it and can transfer it. Times are numbers:
//! presentation times in seconds, publish times in Unix milliseconds,
//! offsets in seconds. No input throws; a call throws only when the
//! module traps (its memory can't grow), after which it is unusable.

use js_sys::{Array, Object, Reflect, Uint8Array};
use strims_video::{CodecParams, DecoderConfig, TrackKind, units::UnitReaderMetrics};
use wasm_bindgen::prelude::*;

use crate::mime::mse_codec;
use crate::remux::{Output, Remuxer, RemuxerMetrics, UnitInfo};

/// Bytes of a wasm memory page.
const PAGE_BYTES: f64 = 65_536.0;

/// A [`Remuxer`] for JS: `new Remuxer()`, then `push` per Egress `Data`
/// message and `finish` at the stream's end, each answering the outputs
/// completed, in order.
#[wasm_bindgen(js_name = Remuxer)]
pub struct JsRemuxer {
    inner: Remuxer,
}

impl Default for JsRemuxer {
    fn default() -> Self {
        JsRemuxer::new()
    }
}

#[wasm_bindgen(js_class = Remuxer)]
impl JsRemuxer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsRemuxer {
        JsRemuxer {
            inner: Remuxer::new(),
        }
    }

    /// One `Data` message: its bytes, `segmentEnd` and `discontinuity`.
    /// The bytes are copied during the call.
    pub fn push(&mut self, data: &[u8], segment_end: bool, discontinuity: bool) -> Array {
        self.inner.message(data, segment_end, discontinuity);
        self.drain()
    }

    /// The stream ended: the outputs of the units held.
    pub fn finish(&mut self) -> Array {
        self.inner.finish();
        self.drain()
    }

    pub fn metrics(&self) -> Object {
        metrics(&self.inner.metrics())
    }

    fn drain(&mut self) -> Array {
        let outputs = Array::new();
        while let Some(output) = self.inner.poll_output() {
            outputs.push(&output_object(&output));
        }
        outputs
    }
}

fn set(o: &Object, key: &str, value: &JsValue) {
    // Defining a property of a plain object fails only for a frozen one.
    let _ = Reflect::set(o, &JsValue::from_str(key), value);
}

fn set_str(o: &Object, key: &str, value: &str) {
    set(o, key, &JsValue::from_str(value));
}

fn set_num(o: &Object, key: &str, value: f64) {
    set(o, key, &JsValue::from_f64(value));
}

fn set_bool(o: &Object, key: &str, value: bool) {
    set(o, key, &JsValue::from_bool(value));
}

fn kind_name(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Video => "video",
        TrackKind::Audio => "audio",
    }
}

fn output_object(output: &Output<'_>) -> Object {
    let o = Object::new();
    match output {
        Output::Init {
            data,
            mime,
            tracks,
            compatible,
        } => {
            set_str(&o, "type", "init");
            set(&o, "data", &Uint8Array::from(*data));
            set_str(&o, "mime", mime);
            let list = Array::new();
            for track in *tracks {
                list.push(&track_object(track));
            }
            set(&o, "tracks", &list);
            set_bool(&o, "compatible", *compatible);
        }
        Output::Media { data, units } => {
            set_str(&o, "type", "media");
            set(&o, "data", &Uint8Array::from(*data));
            let list = Array::new();
            for unit in *units {
                list.push(&unit_object(unit));
            }
            set(&o, "units", &list);
        }
        Output::Gap {
            track,
            kind,
            missing,
        } => {
            set_str(&o, "type", "gap");
            set_num(&o, "track", f64::from(track.0));
            set_str(&o, "kind", kind_name(*kind));
            set_num(&o, "missing", *missing as f64);
        }
        Output::Discontinuity => set_str(&o, "type", "discontinuity"),
        Output::Restart { offset } => {
            set_str(&o, "type", "restart");
            set_num(&o, "offset", offset.as_secs_f64());
        }
    }
    o
}

fn track_object(config: &DecoderConfig) -> Object {
    let o = Object::new();
    set_num(&o, "id", f64::from(config.track.0));
    set_str(&o, "kind", kind_name(config.kind));
    set_str(&o, "codec", mse_codec(&config.codec));
    set_num(&o, "timescale", f64::from(config.timescale));
    match &config.params {
        CodecParams::Avc { width, height, .. } => {
            set_num(&o, "width", f64::from(*width));
            set_num(&o, "height", f64::from(*height));
        }
        CodecParams::Aac {
            sample_rate,
            channels,
            ..
        } => {
            set_num(&o, "sampleRate", f64::from(*sample_rate));
            set_num(&o, "channels", f64::from(*channels));
        }
        _ => {}
    }
    o
}

fn unit_object(unit: &UnitInfo) -> Object {
    let o = Object::new();
    set_num(&o, "track", f64::from(unit.track.0));
    set_str(&o, "kind", kind_name(unit.kind));
    set_bool(&o, "keyframe", unit.keyframe);
    set_num(&o, "pts", unit.pts.as_secs_f64());
    set_num(&o, "published", unit.published.unix_millis() as f64);
    o
}

fn metrics(m: &RemuxerMetrics) -> Object {
    let o = Object::new();
    for (key, value) in [
        ("messages", m.messages),
        ("bytes", m.bytes),
        ("records", m.records),
        ("oversizeRecords", m.oversize_records),
        ("partialRecords", m.partial_records),
        ("discontinuities", m.discontinuities),
        ("units", m.units),
        ("droppedBeforeInit", m.dropped_before_init),
        ("droppedPendingConfig", m.dropped_pending_config),
        ("droppedBackwards", m.dropped_backwards),
        ("refusedInits", m.refused_inits),
        ("inits", m.inits),
        ("fragments", m.fragments),
        ("outputBytes", m.output_bytes),
        ("restarts", m.restarts),
        ("heldFlushes", m.held_flushes),
        ("heldBytes", m.held_bytes),
    ] {
        set_num(&o, key, value as f64);
    }
    let memory = core::arch::wasm32::memory_size::<0>() as f64 * PAGE_BYTES;
    set_num(&o, "memoryBytes", memory);
    set(&o, "reader", &reader_metrics(&m.reader));
    o
}

fn reader_metrics(m: &UnitReaderMetrics) -> Object {
    let o = Object::new();
    for (key, value) in [
        ("records", m.records),
        ("units", m.units),
        ("configs", m.configs),
        ("gaps", m.gaps),
        ("missing", m.missing),
        ("awaitingKeyframe", m.awaiting_keyframe),
        ("configMismatches", m.config_mismatches),
        ("unknownTracks", m.unknown_tracks),
        ("stale", m.stale),
        ("outOfOrder", m.out_of_order),
        ("malformed", m.malformed),
        ("unknownKinds", m.unknown_kinds),
        ("unknownEntries", m.unknown_entries),
        ("lost", m.lost),
    ] {
        set_num(&o, key, value as f64);
    }
    o
}
