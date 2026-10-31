// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::cell::RefCell;
use std::rc::Rc;
use std::task::Waker;

use futures_util::FutureExt;

use super::*;
use crate::codec::RenditionPlan;
use crate::flv::{AUDIO_TRACK, VIDEO_TRACK};
use crate::media::{CodecParams, TrackId};
use crate::synth::script::{aac_lc_asc, avcc};

const T0: i64 = 1_793_275_200_000;

/// A fake process's pipes: what it was given, and what it says.
#[derive(Default)]
struct Pipes {
    args: Vec<String>,
    stdin: Vec<u8>,
    /// The most bytes each write takes; 0 takes none.
    write_limit: Option<usize>,
    stdin_closed: bool,
    stdout: Vec<u8>,
    /// Standard output ends once read.
    stdout_ends: bool,
    stderr: String,
    fail_spawn: bool,
}

#[derive(Clone, Default)]
struct FakeProcesses(Rc<RefCell<Pipes>>);

struct FakeChild(Rc<RefCell<Pipes>>);

impl Processes for FakeProcesses {
    type Error = String;
    type Child = FakeChild;

    fn spawn(&self, args: &[String]) -> Result<FakeChild, String> {
        let mut p = self.0.borrow_mut();
        if p.fail_spawn {
            return Err("no such file".into());
        }
        p.args = args.to_vec();
        Ok(FakeChild(self.0.clone()))
    }
}

impl ChildProcess for FakeChild {
    type Error = String;

    fn poll_write(&mut self, _: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, String>> {
        let mut p = self.0.borrow_mut();
        let n = p.write_limit.map_or(buf.len(), |l| l.min(buf.len()));
        if n == 0 {
            return Poll::Pending;
        }
        p.stdin.extend_from_slice(&buf[..n]);
        Poll::Ready(Ok(n))
    }

    fn close_input(&mut self) {
        self.0.borrow_mut().stdin_closed = true;
    }

    fn poll_read(&mut self, _: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize, String>> {
        let mut p = self.0.borrow_mut();
        if p.stdout.is_empty() {
            return if p.stdout_ends {
                Poll::Ready(Ok(0))
            } else {
                Poll::Pending
            };
        }
        let n = buf.len().min(p.stdout.len());
        buf[..n].copy_from_slice(&p.stdout[..n]);
        p.stdout.drain(..n);
        Poll::Ready(Ok(n))
    }

    fn diagnostics(&self) -> String {
        self.0.borrow().stderr.clone()
    }
}

fn plan(n: usize) -> TranscodePlan {
    TranscodePlan {
        renditions: (0..n)
            .map(|i| RenditionPlan {
                short_side: [720, 480, 360][i % 3],
                max_frame_rate: [60, 30, 30][i % 3],
                bitrate: [3_000_000, 1_200_000, 700_000][i % 3],
            })
            .collect(),
    }
}

fn video(width: u16, height: u16) -> DecoderConfig {
    DecoderConfig {
        track: VIDEO_TRACK,
        kind: TrackKind::Video,
        codec: "avc1.64001f".into(),
        timescale: 90_000,
        params: CodecParams::Avc {
            avcc: avcc(width, height),
            width,
            height,
        },
    }
}

fn audio() -> DecoderConfig {
    DecoderConfig {
        track: AUDIO_TRACK,
        kind: TrackKind::Audio,
        codec: "mp4a.40.2".into(),
        timescale: 48_000,
        params: CodecParams::Aac {
            asc: aac_lc_asc(48_000),
            sample_rate: 48_000,
            channels: 2,
            frame_samples: 1024,
        },
    }
}

fn unit(track: TrackId, dts: i64, keyframe: bool, payload: &[u8]) -> AccessUnit<'_> {
    let ms = if track == VIDEO_TRACK {
        dts / 90
    } else {
        dts / 48
    };
    AccessUnit {
        track,
        dts,
        pts: dts,
        keyframe,
        payload,
        published: Time::from_unix_millis(T0 + ms),
    }
}

fn poll<T: Transcode>(t: &mut T) -> Poll<Option<Result<CodecEvent, T::Error>>> {
    t.poll_next(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn transcode_args_select_scale_and_encode_each_rendition() {
    let args = transcode_args(&FfmpegSettings::default(), &plan(2)).join(" ");
    assert_eq!(
        args,
        "-hide_banner -loglevel error -nostats -nostdin -protocol_whitelist pipe -copyts \
         -probesize 32768 -analyzeduration 0 -f flv -i pipe:0 \
         -filter_complex [0:v]split=2[s0][s1];\
         [s0]select='key+isnan(prev_selected_t)+gte(t-prev_selected_t\\,0.0147)',\
         scale='if(gt(iw\\,ih)\\,-2\\,720)':'if(gt(iw\\,ih)\\,720\\,-2)'[o0];\
         [s1]select='key+isnan(prev_selected_t)+gte(t-prev_selected_t\\,0.0313)',\
         scale='if(gt(iw\\,ih)\\,-2\\,480)':'if(gt(iw\\,ih)\\,480\\,-2)'[o1] \
         -map [o0] -map [o1] -map 0:a? -c:a copy -fps_mode passthrough \
         -c:v libx264 -preset veryfast -tune zerolatency -x264-params scenecut=0:keyint=infinite \
         -a53cc 0 -enc_time_base:v 1:1000 -force_key_frames source \
         -b:v:0 3000000 -maxrate:v:0 3000000 -bufsize:v:0 6000000 \
         -b:v:1 1200000 -maxrate:v:1 1200000 -bufsize:v:1 2400000 \
         -max_interleave_delta 0 -flush_packets 1 -flvflags no_duration_filesize -f flv pipe:1"
    );
    let thumb = thumbnail_args(320).join(" ");
    assert!(thumb.ends_with(
        "-i pipe:0 -frames:v 1 -vf scale=320:-2:out_range=pc,format=yuvj420p -c:v mjpeg -q:v 5 \
         -f image2pipe pipe:1"
    ));
}

#[test]
fn plans_of_no_renditions_or_too_many_are_refused() {
    let ffmpeg = Ffmpeg::new(FakeProcesses::default(), FfmpegSettings::default());
    assert!(matches!(ffmpeg.transcode(&plan(0)), Err(FfmpegError::Plan)));
    assert!(matches!(
        ffmpeg.transcode(&plan(MAX_RENDITIONS + 1)),
        Err(FfmpegError::Plan)
    ));
    let failing = FakeProcesses::default();
    failing.0.borrow_mut().fail_spawn = true;
    let ffmpeg = Ffmpeg::new(failing, FfmpegSettings::default());
    assert!(matches!(
        ffmpeg.transcode(&plan(1)),
        Err(FfmpegError::Spawn(_))
    ));
}

/// What the fake ffmpeg got as input, read back through a source: the
/// configs, then each unit's track, tag time and payload.
fn input_units(stdin: &[u8]) -> (usize, Vec<(TrackId, i64, Vec<u8>)>) {
    let mut r = FlvTagReader::new(1 << 20);
    r.push(stdin);
    let mut s = FlvSource::new();
    let (mut configs, mut units) = (0, Vec::new());
    while let Some(tag) = r.next_tag().unwrap() {
        let e = match tag.kind {
            TAG_VIDEO => s.video(tag.timestamp, tag.body, Time::default()),
            TAG_AUDIO => s.audio(tag.timestamp, tag.body, Time::default()),
            _ => Ok(None),
        };
        match e.unwrap() {
            Some(SourceEvent::Config(_)) => configs += 1,
            Some(SourceEvent::Unit(u)) => {
                units.push((u.track, i64::from(tag.timestamp), u.payload.to_vec()));
            }
            None => {}
        }
    }
    (configs, units)
}

#[test]
fn the_stream_reaches_ffmpeg_as_flv_from_the_start_time() {
    let processes = FakeProcesses::default();
    let ffmpeg = Ffmpeg::new(processes.clone(), FfmpegSettings::default());
    let mut t = ffmpeg.transcode(&plan(2)).unwrap();
    let (v, a) = (video(1920, 1080), audio());
    t.send(Input::Config(&v)).unwrap();
    t.send(Input::Config(&a)).unwrap();
    // nothing is written before the first unit
    assert_eq!(t.queued_bytes(), 0);
    let first = 3_600 * 90_000;
    t.send(Input::Unit(&unit(VIDEO_TRACK, first, true, b"k")))
        .unwrap();
    t.send(Input::Unit(&unit(
        AUDIO_TRACK,
        3_600 * 48_000 + 960,
        true,
        b"a",
    )))
    .unwrap();
    t.send(Input::Unit(&unit(VIDEO_TRACK, first + 3000, false, b"p")))
        .unwrap();
    assert!(t.queued_bytes() > 0);
    assert!(poll(&mut t).is_pending());
    assert_eq!(t.queued_bytes(), 0);
    let p = processes.0.borrow();
    assert!(p.args.iter().any(|a| a == "pipe:1"));
    let (configs, units) = input_units(&p.stdin);
    assert_eq!(configs, 2);
    assert_eq!(
        units,
        [
            (VIDEO_TRACK, 1000, b"k".to_vec()),
            (AUDIO_TRACK, 1020, b"a".to_vec()),
            (VIDEO_TRACK, 1033, b"p".to_vec()),
        ]
    );
}

#[test]
fn input_past_the_bound_is_refused_until_ffmpeg_takes_it() {
    let processes = FakeProcesses::default();
    processes.0.borrow_mut().write_limit = Some(0);
    let settings = FfmpegSettings {
        max_queued: 1000,
        ..FfmpegSettings::default()
    };
    let mut t = Ffmpeg::new(processes.clone(), settings)
        .transcode(&plan(1))
        .unwrap();
    let v = video(640, 360);
    t.send(Input::Config(&v)).unwrap();
    let big = vec![0x65; 600];
    t.send(Input::Unit(&unit(VIDEO_TRACK, 0, true, &big)))
        .unwrap();
    t.send(Input::Unit(&unit(VIDEO_TRACK, 3000, false, &big)))
        .unwrap();
    assert_eq!(
        t.send(Input::Unit(&unit(VIDEO_TRACK, 6000, false, &big))),
        Err(Refused::Full)
    );
    assert!(poll(&mut t).is_pending());
    processes.0.borrow_mut().write_limit = Some(100);
    assert!(poll(&mut t).is_pending());
    assert_eq!(t.queued_bytes(), 0);
    t.send(Input::Unit(&unit(VIDEO_TRACK, 6000, false, &big)))
        .unwrap();
    let orphan = unit(AUDIO_TRACK, 0, true, b"a");
    assert_eq!(
        t.send(Input::Unit(&orphan)),
        Err(Refused::Media("a unit before its track's config"))
    );
}

#[test]
fn finishing_closes_ffmpegs_input_once_it_took_the_queue() {
    let processes = FakeProcesses::default();
    processes.0.borrow_mut().write_limit = Some(0);
    let mut t = Ffmpeg::new(processes.clone(), FfmpegSettings::default())
        .transcode(&plan(1))
        .unwrap();
    let v = video(640, 360);
    t.send(Input::Config(&v)).unwrap();
    t.send(Input::Unit(&unit(VIDEO_TRACK, 0, true, b"k")))
        .unwrap();
    t.finish();
    // input after the end is dropped
    t.send(Input::Unit(&unit(VIDEO_TRACK, 3000, false, b"p")))
        .unwrap();
    assert!(poll(&mut t).is_pending());
    assert!(!processes.0.borrow().stdin_closed);
    processes.0.borrow_mut().write_limit = None;
    assert!(poll(&mut t).is_pending());
    assert!(processes.0.borrow().stdin_closed);
    processes.0.borrow_mut().stdout_ends = true;
    assert!(matches!(poll(&mut t), Poll::Ready(None)));
    let (_, units) = input_units(&processes.0.borrow().stdin);
    assert_eq!(units.len(), 1);
}

/// ffmpeg's output for a transcode of two renditions: rendition 0 in
/// the legacy layout, rendition 1 as multitrack video of track 1, and the
/// audio, from tag times `start`.
fn ffmpeg_output(start: u32) -> Vec<u8> {
    let (v0, a) = (video(1280, 720), audio());
    let mut w = FlvWriter::new(start);
    let mut out = Vec::new();
    w.header(&[v0.clone(), a.clone()], None, &mut out);
    w.config(&v0, &mut out).unwrap();
    w.config(&a, &mut out).unwrap();
    let multitrack = |packet: u8, ts: u32, data: &[u8], out: &mut Vec<u8>| {
        let mut body = vec![0x80 | (1 << 4) | 6, packet];
        body.extend_from_slice(b"avc1");
        body.push(1);
        body.extend_from_slice(data);
        let len = body.len() as u32;
        let [_, s0, s1, s2] = len.to_be_bytes();
        let [t3, t0, t1, t2] = ts.to_be_bytes();
        out.extend_from_slice(&[9, s0, s1, s2, t0, t1, t2, t3, 0, 0, 0]);
        out.extend_from_slice(&body);
        out.extend_from_slice(&(len + 11).to_be_bytes());
    };
    multitrack(0, start, &avcc(854, 480), &mut out);
    w.unit(&v0, &unit(VIDEO_TRACK, 0, true, b"r0"), &mut out)
        .unwrap();
    multitrack(3, start, b"r1", &mut out);
    w.unit(&a, &unit(AUDIO_TRACK, 960, true, b"aa"), &mut out)
        .unwrap();
    out
}

#[test]
fn ffmpegs_output_comes_back_on_the_streams_timeline_for_each_rendition() {
    let processes = FakeProcesses::default();
    let mut t = Ffmpeg::new(processes.clone(), FfmpegSettings::default())
        .transcode(&plan(2))
        .unwrap();
    let (v, a) = (video(1920, 1080), audio());
    t.send(Input::Config(&v)).unwrap();
    t.send(Input::Config(&a)).unwrap();
    let first = 3_600 * 90_000;
    t.send(Input::Unit(&unit(VIDEO_TRACK, first, true, b"k")))
        .unwrap();
    t.send(Input::Unit(&unit(
        AUDIO_TRACK,
        3_600 * 48_000 + 960,
        true,
        b"a",
    )))
    .unwrap();
    processes.0.borrow_mut().stdout = ffmpeg_output(START_MS);
    let mut events = Vec::new();
    while let Poll::Ready(Some(e)) = poll(&mut t) {
        events.push(e.unwrap());
    }
    let configs: Vec<(usize, Option<(u16, u16)>)> = events
        .iter()
        .filter_map(|e| match &e.output {
            Output::Config(c) => Some((e.rendition, c.params.picture_size())),
            Output::Unit(_) => None,
        })
        .collect();
    assert_eq!(
        configs,
        [
            (0, Some((1280, 720))),
            (0, None),
            (1, None),
            (1, Some((854, 480))),
        ]
    );
    let units: Vec<(usize, TrackId, i64, Vec<u8>, i64)> = events
        .iter()
        .filter_map(|e| match &e.output {
            Output::Unit(u) => Some((
                e.rendition,
                u.track,
                u.dts,
                u.payload.clone(),
                u.published.unix_millis() - T0,
            )),
            Output::Config(_) => None,
        })
        .collect();
    let hour = 3_600_000;
    assert_eq!(
        units,
        [
            (0, VIDEO_TRACK, first, b"r0".to_vec(), hour),
            (1, VIDEO_TRACK, first, b"r1".to_vec(), hour),
            (0, AUDIO_TRACK, (hour + 20) * 48, b"aa".to_vec(), hour + 20),
            (1, AUDIO_TRACK, (hour + 20) * 48, b"aa".to_vec(), hour + 20),
        ]
    );
}

#[test]
fn malformed_output_ends_the_transcode() {
    let processes = FakeProcesses::default();
    let mut t = Ffmpeg::new(processes.clone(), FfmpegSettings::default())
        .transcode(&plan(1))
        .unwrap();
    processes.0.borrow_mut().stdout = b"GIF89a....".to_vec();
    assert!(matches!(
        poll(&mut t),
        Poll::Ready(Some(Err(FfmpegError::Output(_))))
    ));
    assert!(matches!(poll(&mut t), Poll::Ready(None)));

    // a track past the plan's renditions
    let processes = FakeProcesses::default();
    let mut t = Ffmpeg::new(processes.clone(), FfmpegSettings::default())
        .transcode(&plan(1))
        .unwrap();
    processes.0.borrow_mut().stdout = ffmpeg_output(START_MS);
    let mut error = None;
    while let Poll::Ready(Some(e)) = poll(&mut t) {
        if let Err(e) = e {
            error = Some(e);
        }
    }
    assert!(matches!(error, Some(FfmpegError::Output(e)) if e.contains("multitrack")));
}

#[test]
fn a_thumbnail_is_ffmpegs_output_for_the_keyframe() {
    let processes = FakeProcesses::default();
    let ffmpeg = Ffmpeg::new(processes.clone(), FfmpegSettings::default());
    let v = video(1280, 720);
    let key = unit(VIDEO_TRACK, 90_000, true, b"keyframe");
    processes.0.borrow_mut().stdout = b"\xff\xd8jpeg\xff\xd9".to_vec();
    processes.0.borrow_mut().stdout_ends = true;
    let jpeg = ffmpeg
        .thumbnail(&[audio(), v.clone()], &key, 320, 64)
        .now_or_never()
        .unwrap()
        .unwrap();
    assert_eq!(jpeg, b"\xff\xd8jpeg\xff\xd9");
    let p = processes.0.borrow();
    assert!(p.stdin_closed);
    assert!(p.args.iter().any(|a| a.starts_with("scale=320:-2")));
    let (configs, units) = input_units(&p.stdin);
    assert_eq!(
        (configs, units),
        (1, vec![(VIDEO_TRACK, 1000, b"keyframe".to_vec())])
    );
    drop(p);

    processes.0.borrow_mut().stdout = vec![0; 65];
    let big = ffmpeg
        .thumbnail(std::slice::from_ref(&v), &key, 320, 64)
        .now_or_never();
    assert!(matches!(big, Some(Err(FfmpegError::PictureTooLarge(64)))));
    processes.0.borrow_mut().stderr = "Invalid data found".into();
    let none = ffmpeg.thumbnail(&[v], &key, 320, 64).now_or_never();
    assert!(matches!(none, Some(Err(FfmpegError::NoPicture(e))) if e == "Invalid data found"));
    let no_config = ffmpeg.thumbnail(&[audio()], &key, 320, 64).now_or_never();
    assert!(matches!(no_config, Some(Err(FfmpegError::Input(_)))));
}
