// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;
use crate::fmp4::ntp_timestamp;
use crate::fmp4::test_dump::{MediaDump, TrafDump, dump_media};

const VIDEO: TrackId = TrackId(1);
const AUDIO: TrackId = TrackId(2);
/// A frame at 30 fps on 90 kHz.
const FRAME: i64 = 3000;

fn video_config() -> DecoderConfig {
    DecoderConfig {
        track: VIDEO,
        kind: TrackKind::Video,
        codec: "avc1.64001f".into(),
        timescale: 90_000,
        params: CodecParams::Avc {
            avcc: vec![1, 0x64, 0, 0x1f],
            width: 16,
            height: 16,
        },
    }
}

fn audio_config() -> DecoderConfig {
    DecoderConfig {
        track: AUDIO,
        kind: TrackKind::Audio,
        codec: "mp4a.40.2".into(),
        timescale: 48_000,
        params: CodecParams::Aac {
            asc: vec![0x11, 0x90],
            sample_rate: 48_000,
            channels: 2,
            frame_samples: 1024,
        },
    }
}

fn writer(span: Duration, configs: &[DecoderConfig]) -> FragmentWriter {
    let mut w = FragmentWriter::new(span);
    w.set_tracks(configs, Duration::ZERO).unwrap();
    w
}

/// The publish time of a unit at `dts` of `track`: a second past the
/// epoch per track id, plus the DTS in milliseconds.
fn published(track: TrackId, dts: i64) -> Time {
    Time::from_unix_millis(i64::from(track.0) * 1000 + dts / 90)
}

/// Pushes a unit of `track` with a payload of `size` bytes of its DTS;
/// the bytes written.
fn push(
    w: &mut FragmentWriter,
    out: &mut Vec<u8>,
    track: TrackId,
    dts: i64,
    pts: i64,
    size: usize,
) -> usize {
    let payload = vec![dts as u8; size];
    w.push(
        &AccessUnit {
            track,
            dts,
            pts,
            keyframe: track == AUDIO || dts == 0,
            payload: &payload,
            published: published(track, dts),
        },
        out,
    )
    .unwrap()
}

fn video(w: &mut FragmentWriter, out: &mut Vec<u8>, k: i64) -> usize {
    push(w, out, VIDEO, k * FRAME, k * FRAME, 10)
}

fn audio(w: &mut FragmentWriter, out: &mut Vec<u8>, n: i64) -> usize {
    push(w, out, AUDIO, n * 1024, n * 1024, 4)
}

fn dump(out: &[u8]) -> MediaDump {
    dump_media(out)
}

/// Each fragment's tracks and sample counts.
fn shape(d: &MediaDump) -> Vec<Vec<(u32, usize)>> {
    d.moofs
        .iter()
        .map(|m| m.trafs.iter().map(|t| (t.track, t.samples.len())).collect())
        .collect()
}

#[test]
fn a_zero_span_writes_each_unit_once_the_next_of_its_track_times_it() {
    let mut w = writer(Duration::ZERO, &[video_config(), audio_config()]);
    let mut out = Vec::new();
    assert_eq!(video(&mut w, &mut out, 0), 0);
    assert_eq!(audio(&mut w, &mut out, 0), 0);
    // A0 is timed, but the run's first fragment waits for V0.
    assert_eq!(audio(&mut w, &mut out, 1), 0);
    assert!(video(&mut w, &mut out, 1) > 0);
    assert!(audio(&mut w, &mut out, 2) > 0);
    assert!(video(&mut w, &mut out, 2) > 0);
    assert!(audio(&mut w, &mut out, 3) > 0);
    let d = dump(&out);
    assert_eq!(
        shape(&d),
        [
            vec![(1, 1), (2, 1)],
            vec![(2, 1)],
            vec![(1, 1)],
            vec![(2, 1)]
        ]
    );
    let sequences: Vec<u32> = d.moofs.iter().map(|m| m.sequence).collect();
    assert_eq!(sequences, [1, 2, 3, 4]);
    assert_eq!(w.next_sequence(), 5);
    // Each track's tfdt follows on from the fragment before.
    for track in [1, 2] {
        let trafs = d.trafs(track);
        for pair in trafs.windows(2) {
            assert_eq!(
                pair[0].tfdt + pair[0].duration(),
                pair[1].tfdt,
                "track {track}"
            );
        }
    }
    assert_eq!(d.traf(1).durations(), [3000, 3000]);
    assert_eq!(d.traf(2).durations(), [1024, 1024, 1024]);
    // V2 and A3 are held.
    assert_eq!(w.held_bytes(), 14);
}

#[test]
fn each_fragment_starts_with_the_prft_of_its_first_trafs_first_sample() {
    let mut w = writer(Duration::ZERO, &[video_config(), audio_config()]);
    let mut out = Vec::new();
    // A keyframe shown two frames after its decode time.
    push(&mut w, &mut out, VIDEO, 9000, 15_000, 10);
    audio(&mut w, &mut out, 0);
    audio(&mut w, &mut out, 1);
    push(&mut w, &mut out, VIDEO, 12_000, 21_000, 10);
    audio(&mut w, &mut out, 2);
    w.flush(None, &mut out);
    let d = dump(&out);
    let boxes: Vec<&str> = d.boxes.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(
        boxes,
        [
            "prft", "moof", "mdat", "prft", "moof", "mdat", "prft", "moof", "mdat"
        ]
    );
    let prfts: Vec<(u32, u64, u64)> = d
        .prfts
        .iter()
        .map(|p| {
            let p = p.unwrap();
            assert_eq!((p.version, p.flags), (1, 1));
            (p.track, p.media_time, p.ntp)
        })
        .collect();
    assert_eq!(
        prfts,
        [
            (1, 15_000, ntp_timestamp(published(VIDEO, 9000))),
            (2, 1024, ntp_timestamp(published(AUDIO, 1024))),
            (1, 21_000, ntp_timestamp(published(VIDEO, 12_000))),
        ]
    );
}

#[test]
fn a_negative_presentation_time_is_media_time_zero() {
    let mut w = writer(Duration::ZERO, &[video_config()]);
    let mut out = Vec::new();
    push(&mut w, &mut out, VIDEO, -6000, -3000, 1);
    w.flush(None, &mut out);
    let d = dump(&out);
    assert_eq!(d.prfts[0].unwrap().media_time, 0);
    assert_eq!(d.traf(1).tfdt, 0);
}

#[test]
fn a_run_without_video_starts_at_its_first_sample() {
    // An audio-only init: nothing waits for a video sample.
    let mut w = writer(Duration::ZERO, &[audio_config()]);
    let mut out = Vec::new();
    assert_eq!(audio(&mut w, &mut out, 0), 0);
    assert!(audio(&mut w, &mut out, 1) > 0);
    assert_eq!(shape(&dump(&out)), [vec![(2, 1)]]);

    // A run whose first sample is audio waits for that sample only.
    let mut w = writer(Duration::ZERO, &[video_config(), audio_config()]);
    let mut out = Vec::new();
    audio(&mut w, &mut out, 0);
    assert!(audio(&mut w, &mut out, 1) > 0);
    assert_eq!(shape(&dump(&out)), [vec![(2, 1)]]);
}

#[test]
fn a_span_groups_samples_until_a_track_lasts_it() {
    let span = Duration::from_millis(100);
    let mut w = writer(span, &[video_config(), audio_config()]);
    let mut out = Vec::new();
    let mut n = 0;
    for k in 0..30 {
        video(&mut w, &mut out, k);
        while n * 1024 * 90_000 / 48_000 <= k * FRAME {
            audio(&mut w, &mut out, n);
            n += 1;
        }
    }
    w.flush(None, &mut out);
    let d = dump(&out);
    assert!(d.moofs.len() >= 9, "{}", d.moofs.len());
    let last = d.moofs.len() - 1;
    for (i, m) in d.moofs.iter().enumerate() {
        let lasts = |t: &TrafDump| {
            let ts = if t.track == 1 { 90_000 } else { 48_000 };
            Duration::from_nanos(t.duration() as i64 * 1_000_000_000 / ts)
        };
        let longest = m.trafs.iter().map(lasts).max().unwrap();
        if i < last {
            assert!(longest >= span, "fragment {i}: {longest}");
            // The fragment was written as soon as it reached the span.
            assert!(
                longest < span + Duration::from_millis(34),
                "fragment {i}: {longest}"
            );
        }
    }
    // Every sample is written once, in order.
    assert_eq!(d.traf(1).samples.len(), 30);
    assert_eq!(d.traf(2).samples.len(), n as usize);
    assert_eq!(w.held_bytes(), 0);
}

#[test]
fn a_max_span_writes_only_at_flush() {
    let mut w = writer(Duration::MAX, &[video_config(), audio_config()]);
    let mut out = Vec::new();
    for k in 0..10 {
        assert_eq!(video(&mut w, &mut out, k), 0);
        assert_eq!(audio(&mut w, &mut out, k), 0);
    }
    assert_eq!(w.held_bytes(), 140);
    let len = w.flush_len();
    assert_eq!(w.flush(Some((VIDEO, 10 * FRAME)), &mut out), len);
    assert_eq!(out.len(), len);
    assert_eq!(shape(&dump(&out)), [vec![(1, 10), (2, 10)]]);
    assert_eq!((w.flush_len(), w.held_bytes()), (0, 0));
    assert_eq!(w.flush(None, &mut out), 0);
    assert_eq!(out.len(), len);
}

#[test]
fn flush_times_each_tracks_last_sample() {
    let mut w = writer(Duration::MAX, &[video_config(), audio_config()]);
    let mut out = Vec::new();
    push(&mut w, &mut out, VIDEO, 0, 0, 1);
    push(&mut w, &mut out, VIDEO, 3600, 3600, 1);
    audio(&mut w, &mut out, 0);
    // The named track lasts until the DTS given; audio a frame.
    w.flush(Some((VIDEO, 9000)), &mut out);
    let d = dump(&out);
    assert_eq!(d.traf(1).durations(), [3600, 5400]);
    assert_eq!(d.traf(2).durations(), [1024]);
    // Without a DTS, video repeats its previous duration, which the
    // flush's DTS set.
    let mut out = Vec::new();
    push(&mut w, &mut out, VIDEO, 9000, 9000, 1);
    w.flush(None, &mut out);
    assert_eq!(dump(&out).traf(1).durations(), [5400]);
    // Without any, it lasts 1/30 s.
    let mut w = writer(Duration::MAX, &[video_config()]);
    let mut out = Vec::new();
    push(&mut w, &mut out, VIDEO, 0, 0, 1);
    w.flush(None, &mut out);
    assert_eq!(dump(&out).traf(1).durations(), [3000]);
}

#[test]
fn a_decreasing_dts_lasts_zero() {
    let mut w = writer(Duration::MAX, &[video_config()]);
    let mut out = Vec::new();
    push(&mut w, &mut out, VIDEO, 6000, 6000, 1);
    push(&mut w, &mut out, VIDEO, 3000, 3000, 1);
    w.flush(Some((VIDEO, 9000)), &mut out);
    assert_eq!(dump(&out).traf(1).durations(), [0, 6000]);
}

#[test]
fn units_of_undeclared_tracks_are_refused() {
    let mut w = writer(Duration::ZERO, &[video_config()]);
    let payload = [1];
    let unit = AccessUnit {
        track: AUDIO,
        dts: 0,
        pts: 0,
        keyframe: true,
        payload: &payload,
        published: Time::EPOCH,
    };
    let mut out = Vec::new();
    assert_eq!(w.push(&unit, &mut out), Err(FragmentError::UnknownTrack(2)));
    assert_eq!(w.held_bytes(), 0);
    assert!(out.is_empty());
}

#[test]
fn set_tracks_drops_held_samples_and_keeps_durations_of_tracks_that_stay() {
    let mut w = writer(Duration::MAX, &[video_config(), audio_config()]);
    let mut out = Vec::new();
    push(&mut w, &mut out, VIDEO, 0, 0, 1);
    push(&mut w, &mut out, VIDEO, 3600, 3600, 1);
    w.flush(Some((VIDEO, 7200)), &mut out);
    push(&mut w, &mut out, VIDEO, 7200, 7200, 1);
    // Refused configs change nothing.
    let zero = DecoderConfig {
        timescale: 0,
        ..audio_config()
    };
    assert_eq!(
        w.set_tracks(&[video_config(), zero], Duration::ZERO),
        Err(InitError::ZeroTimescale(2))
    );
    assert_eq!(w.held_bytes(), 1);
    w.set_tracks(&[video_config()], Duration::ZERO).unwrap();
    assert_eq!(w.held_bytes(), 0);
    let mut out = Vec::new();
    push(&mut w, &mut out, VIDEO, 7200, 7200, 1);
    w.flush(None, &mut out);
    assert_eq!(dump(&out).traf(1).durations(), [3600]);
    // Another timescale forgets the duration.
    w.set_tracks(
        &[DecoderConfig {
            timescale: 1000,
            ..video_config()
        }],
        Duration::ZERO,
    )
    .unwrap();
    let mut out = Vec::new();
    push(&mut w, &mut out, VIDEO, 7200, 7200, 1);
    w.flush(None, &mut out);
    assert_eq!(dump(&out).traf(1).durations(), [33]);
}

#[test]
fn flush_len_is_what_flush_writes() {
    let mut w = writer(Duration::from_millis(50), &[video_config(), audio_config()]);
    let mut out = Vec::new();
    assert_eq!(w.flush_len(), 0);
    for k in 0..12 {
        video(&mut w, &mut out, k);
        audio(&mut w, &mut out, k);
        if k % 5 == 4 {
            let (len, before) = (w.flush_len(), out.len());
            assert!(len > 0);
            assert_eq!(w.flush(None, &mut out), len);
            assert_eq!(out.len() - before, len);
        }
    }
    let (len, before) = (w.flush_len(), out.len());
    assert_eq!(w.flush(None, &mut out), len);
    assert_eq!(out.len() - before, len);
    let d = dump(&out);
    assert_eq!(d.traf(1).samples.len(), 12);
    assert_eq!(d.traf(2).samples.len(), 12);
}

#[test]
fn samples_decoding_before_the_edit_keep_their_gop_in_one_fragment() {
    // An edit of two frames: 6000 units of video, 3200 of audio.
    let delay = Duration::from_nanos(66_666_666);
    let mut w = FragmentWriter::new(Duration::ZERO);
    w.set_tracks(&[video_config(), audio_config()], delay)
        .unwrap();
    let mut out = Vec::new();
    let mut n = 0;
    for k in 0..6 {
        push(&mut w, &mut out, VIDEO, k * FRAME, k * FRAME, 10);
        while n * 1024 * 90_000 / 48_000 <= k * FRAME {
            audio(&mut w, &mut out, n);
            n += 1;
        }
    }
    // The first GOP decodes from before the edit: nothing until the next
    // keyframe.
    assert!(out.is_empty());
    let key = |w: &mut FragmentWriter, out: &mut Vec<u8>, k: i64| {
        let payload = [1];
        w.push(
            &AccessUnit {
                track: VIDEO,
                dts: k * FRAME,
                pts: k * FRAME,
                keyframe: true,
                payload: &payload,
                published: Time::EPOCH,
            },
            out,
        )
        .unwrap()
    };
    assert!(key(&mut w, &mut out, 6) > 0);
    let d = dump(&out);
    assert_eq!(shape(&d), [vec![(1, 6), (2, n as usize - 1)]]);
    // Past the edit, a fragment per unit again.
    let before = out.len();
    assert!(video(&mut w, &mut out, 7) > 0);
    assert_eq!(shape(&dump(&out[before..])), [vec![(1, 1)]]);

    // Decode times past the edit don't wait for a keyframe.
    let mut w = FragmentWriter::new(Duration::ZERO);
    w.set_tracks(&[video_config(), audio_config()], delay)
        .unwrap();
    let mut out = Vec::new();
    video(&mut w, &mut out, 100);
    assert!(video(&mut w, &mut out, 101) > 0);
    // Nor do tracks without an edit.
    let mut w = FragmentWriter::new(Duration::ZERO);
    w.set_tracks(&[video_config()], Duration::ZERO).unwrap();
    let mut out = Vec::new();
    video(&mut w, &mut out, 0);
    assert!(video(&mut w, &mut out, 1) > 0);
}

#[test]
fn held_units_are_written_with_the_next_push() {
    let mut w = writer(Duration::ZERO, &[video_config(), audio_config()]);
    let mut out = Vec::new();
    video(&mut w, &mut out, 0);
    audio(&mut w, &mut out, 0);
    assert!(video(&mut w, &mut out, 1) > 0);
    out.clear();
    for n in 1..4 {
        let payload = [0u8; 4];
        w.hold(&AccessUnit {
            track: AUDIO,
            dts: n * 1024,
            pts: n * 1024,
            keyframe: true,
            payload: &payload,
            published: published(AUDIO, n),
        })
        .unwrap();
    }
    // Each hold timed the one before, but nothing is written until a push.
    assert!(out.is_empty());
    assert!(audio(&mut w, &mut out, 4) > 0);
    let d = dump(&out);
    assert_eq!(shape(&d), [vec![(2, 4)]]);
    assert_eq!(d.traf(2).tfdt, 0);
    assert_eq!(d.traf(2).durations(), [1024; 4]);
    let unit = AccessUnit {
        track: TrackId(9),
        dts: 0,
        pts: 0,
        keyframe: true,
        payload: &[],
        published: Time::EPOCH,
    };
    assert_eq!(w.hold(&unit), Err(FragmentError::UnknownTrack(9)));
}

#[test]
fn an_ended_track_starts_its_next_traf_at_its_own_decode_time() {
    let mut w = writer(Duration::ZERO, &[video_config(), audio_config()]);
    let mut out = Vec::new();
    for k in 0..3 {
        video(&mut w, &mut out, k);
    }
    audio(&mut w, &mut out, 0);
    audio(&mut w, &mut out, 1);
    out.clear();
    // Audio ends a frame after its last unit, which is written though
    // nothing times it; video's untimed last stays held.
    assert!(w.end_track(AUDIO, 2 * 1024, &mut out).unwrap() > 0);
    let d = dump(&out);
    assert_eq!(shape(&d), [vec![(2, 1)]]);
    assert_eq!((d.traf(2).tfdt, d.traf(2).durations()), (1024, vec![1024]));
    assert_eq!(w.end_track(AUDIO, 0, &mut out).unwrap(), 0);
    // The next audio unit, ten frames on, starts a traf at its DTS.
    out.clear();
    audio(&mut w, &mut out, 12);
    audio(&mut w, &mut out, 13);
    let d = dump(&out);
    assert_eq!(
        (d.traf(2).tfdt, d.traf(2).durations()),
        (12 * 1024, vec![1024])
    );
    assert_eq!(
        w.end_track(TrackId(9), 0, &mut out),
        Err(FragmentError::UnknownTrack(9))
    );
}
