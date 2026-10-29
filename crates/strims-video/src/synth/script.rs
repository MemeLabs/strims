// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A deterministic script of FLV tags, and the records it produces.
//!
//! A [`Script`] is what an encoder publishing over RTMP sends: an
//! `onMetaData` tag, the AVC sequence header (a real High profile SPS for
//! the configured size, codec `avc1.64001f`), the AAC sequence header
//! (LC, stereo), then video frames and AAC frames in time order, video
//! first at equal times. Bodies are legacy FLV tag bodies.
//!
//! - Video frame `k` is at `k * 1000 / fps` ms. Each GOP starts with a
//!   keyframe. A frame is one NAL unit, 4-byte length prefixed, of a
//!   deterministic byte pattern. Sizes follow the bitrate, keyframes four
//!   times the others.
//! - With B-frames, the decode order within a GOP is `I P B P B … P`: each
//!   P is shown after the B that follows it, so its composition time is
//!   two frames, the keyframe's and a closing P's one frame, a B's zero.
//! - AAC frame `n` is at `n * 1024 * 1000 / sample_rate` ms.
//! - A resize at GOP `g` sends a new sequence header, for half the size,
//!   before that GOP's keyframe.
//!
//! [`expected_records`] runs a script through the real source and
//! segmenter into a [`PackedSink`] and decodes the packed stream back into
//! records.

// Test support: a panic is a test failure.
#![allow(clippy::expect_used, clippy::panic)]

use strims_base::time::Time;
use strims_chunkstream::Frag;
use strims_chunkstream::packed::{DEFAULT_SIZE, PackedDecoder};

use crate::flv::{FlvSource, SourceEvent};
use crate::fmp4::parse::{Boxes, split_record};
use crate::media::Muxer;
use crate::record::PackedSink;
use crate::segment::{CutRule, Segmenter};
use crate::test_util::{BitWriter, SpsSpec, avcc as avcc_record};

/// The kind of an FLV tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    Video,
    Audio,
    /// Script data: the `onMetaData` message.
    Metadata,
}

/// An FLV tag: its kind, its 32-bit millisecond timestamp and its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub kind: TagKind,
    pub timestamp: u32,
    pub body: Vec<u8>,
}

/// An `onMetaData` value.
#[derive(Debug, Clone, PartialEq)]
pub enum MetaValue {
    Number(f64),
    Bool(bool),
    String(String),
}

/// What a script holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptConfig {
    pub fps: u32,
    /// Frames per GOP, the first a keyframe.
    pub gop_frames: u32,
    /// Video payload bits per second.
    pub video_bitrate: u32,
    /// The picture size; odd sizes round up to even.
    pub width: u16,
    pub height: u16,
    pub audio: bool,
    /// An AAC sampling frequency; rates outside its table are signalled
    /// explicitly.
    pub sample_rate: u32,
    /// Audio payload bits per second.
    pub audio_bitrate: u32,
    pub duration_secs: u32,
    pub b_frames: bool,
    /// The first tag's timestamp; later ones wrap past `u32::MAX`.
    pub start_ms: u32,
    /// The GOP before whose keyframe a new sequence header, for half the
    /// picture size, is sent.
    pub resize_at_gop: Option<u32>,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        ScriptConfig {
            fps: 30,
            gop_frames: 60,
            video_bitrate: 1_000_000,
            width: 1280,
            height: 720,
            audio: true,
            sample_rate: 48_000,
            audio_bitrate: 128_000,
            duration_secs: 10,
            b_frames: false,
            start_ms: 0,
            resize_at_gop: None,
        }
    }
}

/// The tags of a stream, in sending order.
#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    pub tags: Vec<Tag>,
    /// The `onMetaData` fields, also encoded in the first tag's body.
    pub metadata: Vec<(String, MetaValue)>,
}

/// FLV AVC frame types.
const FRAME_KEY: u8 = 1;
const FRAME_INTER: u8 = 2;
/// The legacy AVC codec id, AVCPacketType values.
const CODEC_AVC: u8 = 7;
const AVC_SEQUENCE_HEADER: u8 = 0;
const AVC_NALU: u8 = 1;
/// Sound format AAC, 44 kHz, 16-bit, stereo: the header byte of every AAC
/// tag.
const AAC_TAG_HEADER: u8 = 0xaf;
const AAC_SEQUENCE_HEADER: u8 = 0;
const AAC_RAW: u8 = 1;
/// NAL unit headers: IDR slice, reference slice, non-reference slice.
const NAL_IDR: u8 = 0x65;
const NAL_REFERENCE: u8 = 0x41;
const NAL_NON_REFERENCE: u8 = 0x01;
/// Samples per AAC frame.
const AAC_FRAME_SAMPLES: u64 = 1024;
/// AAC samplingFrequencyIndex rates.
const AAC_RATES: [u32; 13] = [
    96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025, 8_000,
    7_350,
];

/// The avcC record of a High profile, level 3.1 SPS for a `width` by
/// `height` picture (rounded up to even), with one PPS: codec
/// `avc1.64001f`.
pub fn avcc(width: u16, height: u16) -> Vec<u8> {
    let (w, h) = (
        u32::from(width).div_ceil(2) * 2,
        u32::from(height).div_ceil(2) * 2,
    );
    let (w_mbs, h_mbs) = (w.div_ceil(16).max(1), h.div_ceil(16).max(1));
    // 4:2:0 progressive crops in units of 2 pixels.
    let crop = [0, (w_mbs * 16 - w) / 2, 0, (h_mbs * 16 - h) / 2];
    let sps = SpsSpec {
        level: 31,
        crop: (crop != [0; 4]).then_some(crop),
        ..SpsSpec::high(w_mbs, h_mbs)
    };
    avcc_record(&sps.build())
}

/// The AudioSpecificConfig of AAC LC, stereo, at `sample_rate`.
pub fn aac_lc_asc(sample_rate: u32) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.bits(5, 2);
    match AAC_RATES.iter().position(|&r| r == sample_rate) {
        Some(i) => w.bits(4, i as u64),
        None => w.bits(4, 15).bits(24, u64::from(sample_rate)),
    };
    // channelConfiguration 2; frameLengthFlag, dependsOnCoreCoder,
    // extensionFlag 0.
    w.bits(4, 2).bits(3, 0);
    w.finish()
}

impl Script {
    pub fn new(config: &ScriptConfig) -> Script {
        assert!(config.fps > 0 && config.gop_frames > 0, "fps and GOP > 0");
        assert!(config.sample_rate > 0, "sample rate > 0");
        let metadata = metadata(config);
        let at = |ms: u64| config.start_ms.wrapping_add(ms as u32);
        let mut tags = vec![
            Tag {
                kind: TagKind::Metadata,
                timestamp: at(0),
                body: encode_metadata(&metadata),
            },
            video_config_tag(at(0), config.width, config.height),
        ];
        if config.audio {
            let mut body = vec![AAC_TAG_HEADER, AAC_SEQUENCE_HEADER];
            body.extend_from_slice(&aac_lc_asc(config.sample_rate));
            tags.push(Tag {
                kind: TagKind::Audio,
                timestamp: at(0),
                body,
            });
        }

        let fps = u64::from(config.fps);
        let gop = u64::from(config.gop_frames);
        let frames = u64::from(config.duration_secs) * fps;
        let frame_ms = |k: u64| k * 1000 / fps;
        let average = u64::from(config.video_bitrate) / 8 / fps;
        let inter_size = (average * gop / (gop + 3)).max(16);
        let key_size = 4 * inter_size;

        let rate = u64::from(config.sample_rate);
        let audio_frames = if config.audio {
            u64::from(config.duration_secs) * rate / AAC_FRAME_SAMPLES
        } else {
            0
        };
        let audio_ms = |n: u64| n * AAC_FRAME_SAMPLES * 1000 / rate;
        let audio_size = (u64::from(config.audio_bitrate) / 8 * AAC_FRAME_SAMPLES / rate).max(8);

        let (mut k, mut n) = (0, 0);
        while k < frames || n < audio_frames {
            let video_next = k < frames && (n >= audio_frames || frame_ms(k) <= audio_ms(n));
            if video_next {
                let ms = frame_ms(k);
                let j = k % gop;
                if j == 0
                    && config
                        .resize_at_gop
                        .is_some_and(|g| u64::from(g) == k / gop)
                {
                    tags.push(video_config_tag(
                        at(ms),
                        config.width / 2,
                        config.height / 2,
                    ));
                }
                let (frame_type, nal, size, shift) = match j {
                    0 => (FRAME_KEY, NAL_IDR, key_size, 1),
                    _ if !config.b_frames => (FRAME_INTER, NAL_REFERENCE, inter_size, 0),
                    j if j % 2 == 1 && j + 1 < gop => (FRAME_INTER, NAL_REFERENCE, inter_size, 2),
                    j if j % 2 == 1 => (FRAME_INTER, NAL_REFERENCE, inter_size, 1),
                    _ => (FRAME_INTER, NAL_NON_REFERENCE, inter_size, 0),
                };
                let cts = if config.b_frames {
                    frame_ms(k + shift) - ms
                } else {
                    0
                };
                let [_, c0, c1, c2] = (cts as u32).to_be_bytes();
                let mut body = vec![(frame_type << 4) | CODEC_AVC, AVC_NALU, c0, c1, c2];
                nal_unit(&mut body, nal, size as usize, k as u32);
                tags.push(Tag {
                    kind: TagKind::Video,
                    timestamp: at(ms),
                    body,
                });
                k += 1;
            } else {
                let mut body = vec![AAC_TAG_HEADER, AAC_RAW];
                pattern(&mut body, audio_size as usize, 0x8000_0000 | n as u32);
                tags.push(Tag {
                    kind: TagKind::Audio,
                    timestamp: at(audio_ms(n)),
                    body,
                });
                n += 1;
            }
        }
        Script { tags, metadata }
    }
}

fn video_config_tag(timestamp: u32, width: u16, height: u16) -> Tag {
    let mut body = vec![(FRAME_KEY << 4) | CODEC_AVC, AVC_SEQUENCE_HEADER, 0, 0, 0];
    body.extend_from_slice(&avcc(width, height));
    Tag {
        kind: TagKind::Video,
        timestamp,
        body,
    }
}

/// Appends a 4-byte length prefixed NAL unit of `size` bytes in all.
fn nal_unit(out: &mut Vec<u8>, header: u8, size: usize, seed: u32) {
    let len = size.saturating_sub(4).max(1);
    out.extend_from_slice(&(len as u32).to_be_bytes());
    out.push(header);
    pattern(out, len - 1, seed);
}

/// Appends `len` bytes of a pattern of `seed`.
fn pattern(out: &mut Vec<u8>, len: usize, seed: u32) {
    let base = seed.wrapping_mul(0x9e37_79b9);
    out.extend(
        (0..len as u32).map(|i| (base.wrapping_add(i.wrapping_mul(0x85eb_ca6b)) >> 24) as u8),
    );
}

fn metadata(config: &ScriptConfig) -> Vec<(String, MetaValue)> {
    let num = |k: &str, v: f64| (k.to_owned(), MetaValue::Number(v));
    let mut fields = vec![
        num("duration", 0.0),
        num("width", f64::from(config.width)),
        num("height", f64::from(config.height)),
        num("videodatarate", f64::from(config.video_bitrate) / 1000.0),
        num("framerate", f64::from(config.fps)),
        num("videocodecid", 7.0),
    ];
    if config.audio {
        fields.extend([
            num("audiodatarate", f64::from(config.audio_bitrate) / 1000.0),
            num("audiosamplerate", f64::from(config.sample_rate)),
            num("audiosamplesize", 16.0),
            ("stereo".to_owned(), MetaValue::Bool(true)),
            num("audiocodecid", 10.0),
        ]);
    }
    fields.push((
        "encoder".to_owned(),
        MetaValue::String("strims synth".to_owned()),
    ));
    fields
}

/// The AMF0 script data of an FLV `onMetaData` tag: the string, then an
/// ECMA array of the fields. RTMP sends it after an `@setDataFrame`
/// string.
pub fn encode_metadata(fields: &[(String, MetaValue)]) -> Vec<u8> {
    let mut out = Vec::new();
    amf_string(&mut out, "onMetaData");
    out.push(0x08);
    out.extend_from_slice(&(fields.len() as u32).to_be_bytes());
    for (key, value) in fields {
        out.extend_from_slice(&(key.len() as u16).to_be_bytes());
        out.extend_from_slice(key.as_bytes());
        match value {
            MetaValue::Number(v) => {
                out.push(0x00);
                out.extend_from_slice(&v.to_be_bytes());
            }
            MetaValue::Bool(v) => out.extend_from_slice(&[0x01, u8::from(*v)]),
            MetaValue::String(v) => amf_string(&mut out, v),
        }
    }
    // The empty key and object end marker.
    out.extend_from_slice(&[0x00, 0x00, 0x09]);
    out
}

fn amf_string(out: &mut Vec<u8>, s: &str) {
    out.push(0x02);
    out.extend_from_slice(&(s.len() as u16).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// The time a script's units are received at: the epoch plus the tag time.
fn received(tag: &Tag) -> Time {
    Time::from_unix_millis(i64::from(tag.timestamp))
}

/// The records `script` produces: its tags through [`FlvSource`] and a
/// [`Segmenter`] into a [`PackedSink`], finished at the end, and the
/// packed stream decoded from offset 0 into whole records.
pub fn expected_records(script: &Script, rule: CutRule) -> Vec<Vec<u8>> {
    let mut source = FlvSource::new();
    let mut segmenter = Segmenter::new(rule, PackedSink::new());
    let mut stream = Vec::new();
    for tag in &script.tags {
        let event = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, received(tag)),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, received(tag)),
            TagKind::Metadata => continue,
        };
        let result = match event.expect("script tags are valid") {
            Some(SourceEvent::Config(c)) => segmenter.set_config(&c),
            Some(SourceEvent::Unit(u)) => segmenter.push(&u),
            None => Ok(()),
        };
        result.expect("the segmenter takes the script");
        let out = segmenter.sink_mut().take_output();
        stream.extend_from_slice(&out);
        segmenter.sink_mut().restore(out);
    }
    segmenter.finish().expect("the segmenter finishes");
    stream.extend_from_slice(segmenter.sink().output());
    decode_records(&stream)
}

/// The whole records of a packed stream read from offset 0.
pub fn decode_records(stream: &[u8]) -> Vec<Vec<u8>> {
    let mut decoder = PackedDecoder::new(DEFAULT_SIZE).expect("DEFAULT_SIZE is valid");
    let mut records = Vec::new();
    let mut current = Vec::new();
    let mut input = stream;
    // An end can follow the input's last byte, so decoding goes on until
    // no fragment is left.
    loop {
        let (used, frag) = decoder.decode(input);
        input = input.get(used..).unwrap_or_default();
        match frag {
            Some(Frag::Data(d)) => current.extend_from_slice(d),
            Some(Frag::End) => records.push(std::mem::take(&mut current)),
            Some(Frag::Corrupt) => panic!("the packed stream is corrupt"),
            None => return records,
        }
    }
}

/// `record` with the NTP timestamp of each of its media's `prft` boxes
/// zeroed: the record as any writer of the same units writes it,
/// whenever it received them. Bytes that aren't an fMP4 record come back
/// as they are.
pub fn without_publish_times(record: &[u8]) -> Vec<u8> {
    let Ok((init, media)) = split_record(record) else {
        return record.to_vec();
    };
    let mut out = record.to_vec();
    out.truncate(2 + init.len());
    out.extend_from_slice(&media_without_publish_times(media));
    out
}

/// A media segment with the NTP timestamp of each of its `prft` boxes
/// zeroed (see [`without_publish_times`]).
pub fn media_without_publish_times(media: &[u8]) -> Vec<u8> {
    let mut out = media.to_vec();
    for b in Boxes::new(media) {
        let Ok((fourcc, content)) = b else {
            break;
        };
        // Version and flags and reference_track_ID, then ntp_timestamp.
        let at = content.as_ptr().addr() - media.as_ptr().addr();
        if fourcc.0 == *b"prft"
            && let Some(ntp) = out.get_mut(at + 8..at + 16)
        {
            ntp.fill(0);
        }
    }
    out
}

#[cfg(test)]
#[path = "script_test.rs"]
mod tests;
