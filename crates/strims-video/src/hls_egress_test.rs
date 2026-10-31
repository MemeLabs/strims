// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::VecDeque;

use futures_util::FutureExt;
use strims_chunkstream::chunked::{ChunkedRecordDecoder, Item};
use strims_transfer::{ReadFailure, SwarmRead};

use super::*;
use crate::egress::{ChunkedReader, Rewind};
use crate::fmp4::{media_duration, parse_init};
use crate::units::test_support::{audio, unit, video};
use crate::units::{Record, UnitWriter, record::parse_record};

const T: Time = Time::from_unix_millis(1_700_000_000_000);
/// The first segment's and init's number.
const FIRST: u64 = 1_700_000_000_000;

/// A stream registered on its own routes.
struct Served {
    routes: HlsRoutes,
    _profile: HlsProfile,
    stream: HlsStream,
}

fn served() -> Served {
    served_with(PlaylistConfig::default())
}

/// A stream whose playlist lists every segment of a minute.
fn served_long() -> Served {
    served_with(PlaylistConfig {
        listed: Duration::from_secs(60),
        retained: Duration::from_secs(60),
        ..PlaylistConfig::default()
    })
}

fn served_with(config: PlaylistConfig) -> Served {
    let routes = HlsRoutes::new();
    let profile = routes.profile(&[1; 32]).unwrap();
    let stream = profile.open_stream(T, [2; 32], config).unwrap();
    Served {
        routes,
        _profile: profile,
        stream,
    }
}

impl Served {
    fn playlist(&self) -> String {
        let r = self.routes.get(T, self.stream.path());
        assert_eq!(r.status, 200);
        String::from_utf8(r.body.to_vec()).unwrap()
    }

    /// The bytes of `file`, beside the playlist.
    fn file(&self, file: &str) -> Vec<u8> {
        let path = self.stream.path().replace("index.m3u8", file);
        let r = self.routes.get(T, &path);
        assert_eq!(r.status, 200, "{file}");
        r.body.to_vec()
    }
}

const SKIPPED: Fed = Fed {
    segments: 0,
    skipped: 1,
};
const NOTHING: Fed = Fed {
    segments: 0,
    skipped: 0,
};

const CHUNK: usize = 4096;
/// 90 kHz ticks of a frame at 30 frames a second.
const FRAME: i64 = 3000;

/// A unit-format stream in 4 KiB frames: `pictures` video frames at 30 a
/// second, keyframes at `keyframes` (picture numbers) of 12 KiB and
/// pictures of 1.5 KiB, with AAC frames between, its session still live.
/// Its bytes, and its records with their offsets.
fn units_stream(pictures: i64, keyframes: &[i64]) -> (Vec<u8>, Vec<(u64, Vec<u8>)>) {
    let mut w = UnitWriter::new(CHUNK).unwrap();
    w.set_config(&video(1)).unwrap();
    w.set_config(&audio(2)).unwrap();
    let big = vec![0x5a; 12 * 1024];
    let mut n = 0i64;
    for k in 0..pictures {
        let key = keyframes.contains(&k);
        let mut u = unit(1, k * FRAME, key, 1500);
        if key {
            u.payload = &big;
        }
        w.push(&u).unwrap();
        while n * 1024 * 90_000 / 48_000 <= k * FRAME {
            w.push(&unit(2, n * 1024, true, 300)).unwrap();
            n += 1;
        }
    }
    let stream = w.take_output();
    let mut d = ChunkedRecordDecoder::new(CHUNK, units::MAX_RECORD_LEN).unwrap();
    let mut records = Vec::new();
    let mut at = 0;
    loop {
        let (used, item) = d.decode(&stream[at..]);
        at += used;
        match item {
            Some(Item::Record { offset, payload }) => records.push((offset, payload.to_vec())),
            Some(Item::Lost) => panic!("lost records"),
            None => return (stream, records),
        }
    }
}

/// Whether unit-format record `r` is a video unit of a picture in
/// `pictures`.
fn picture_in(r: &[u8], pictures: std::ops::Range<i64>) -> bool {
    match parse_record(r).unwrap() {
        Record::Unit { header, .. } => {
            header.track.0 == 1 && pictures.contains(&(header.dts / FRAME))
        }
        _ => false,
    }
}

/// Feeds whole records as pieces: the summed result.
fn feed_units(feed: &mut UnitFeed, s: &Served, records: &[&[u8]]) -> Fed {
    let mut fed = Fed::default();
    for r in records {
        for piece in r.chunks(5000) {
            let f = feed.feed(EgressPiece::Data(piece), &s.stream);
            fed.segments += f.segments;
            fed.skipped += f.skipped;
        }
        let f = feed.feed(EgressPiece::RecordEnd, &s.stream);
        fed.segments += f.segments;
        fed.skipped += f.skipped;
    }
    fed
}

fn unit_feed() -> UnitFeed {
    UnitFeed::new(HlsMuxerConfig::default())
}

/// The listed segments: EXTINF milliseconds, and whether a discontinuity
/// precedes each.
fn listed(playlist: &str) -> Vec<(u64, bool)> {
    let mut out = Vec::new();
    let mut discontinuity = false;
    for l in playlist.lines() {
        if l == "#EXT-X-DISCONTINUITY" {
            discontinuity = true;
        } else if let Some(x) = l.strip_prefix("#EXTINF:") {
            let (s, f) = x.trim_end_matches(',').split_once('.').unwrap();
            let ms = s.parse::<u64>().unwrap() * 1000 + f.parse::<u64>().unwrap();
            out.push((ms, std::mem::take(&mut discontinuity)));
        }
    }
    out
}

#[test]
fn unit_records_are_muxed_into_segments_cut_at_keyframes() {
    let s = served();
    let keyframes: Vec<i64> = (0..6).map(|g| g * 30).collect();
    let (_, recs) = units_stream(180, &keyframes);
    let mut feed = unit_feed();
    let all: Vec<&[u8]> = recs.iter().map(|(_, r)| r.as_slice()).collect();
    let fed = feed_units(&mut feed, &s, &all);
    // The last GOP is still open.
    assert_eq!(
        fed,
        Fed {
            segments: 5,
            skipped: 0
        }
    );
    let playlist = s.playlist();
    assert_eq!(listed(&playlist), vec![(1000, false); 5]);
    assert!(!playlist.contains("INDEPENDENT"));
    let init = s.file(&format!("init-{FIRST}.mp4"));
    let info = parse_init(&init).unwrap();
    assert_eq!(info.tracks().len(), 2);
    for i in 0..5 {
        let media = s.file(&format!("{}.m4s", FIRST + i));
        assert_eq!(
            media_duration(&info, &media).unwrap(),
            Duration::from_secs(1)
        );
    }
}

#[test]
fn scene_cut_gops_up_to_10_s_are_split_within_the_target() {
    let s = served_long();
    let keyframes = [0, 21, 96, 396, 420, 495, 630];
    let (_, recs) = units_stream(700, &keyframes);
    let mut feed = unit_feed();
    let all: Vec<&[u8]> = recs.iter().map(|(_, r)| r.as_slice()).collect();
    feed_units(&mut feed, &s, &all);
    let millis: Vec<u64> = listed(&s.playlist()).iter().map(|(m, _)| *m).collect();
    assert!(millis.iter().all(|&m| (m + 500) / 1000 <= 4), "{millis:?}");
    // 700, 2500, 4467, 4467 and 1067 ms: the 10 s GOP is in three.
    let shown: Vec<u64> = millis.iter().take(5).copied().collect();
    assert_eq!(shown, [700, 2500, 4467, 4467, 1067]);
    assert_eq!(feed.muxer.metrics().mid_gop_segments, 3);
}

#[test]
fn a_gap_in_the_units_marks_the_segment_at_the_next_keyframe_discontinuous() {
    let s = served();
    let keyframes: Vec<i64> = (0..5).map(|g| g * 30).collect();
    let (_, recs) = units_stream(150, &keyframes);
    let mut feed = unit_feed();
    // Video units 40 to 44 are lost.
    let mut fed = Fed::default();
    for (_, r) in &recs {
        if picture_in(r, 40..45) {
            let f = feed.feed(EgressPiece::Discontinuity, &s.stream);
            fed.segments += f.segments;
            continue;
        }
        let f = feed_units(&mut feed, &s, &[r]);
        fed.segments += f.segments;
    }
    assert_eq!(fed.segments, 4);
    let got = listed(&s.playlist());
    // The GOP holding the gap ends at it; the next starts at a keyframe.
    assert_eq!(got[0], (1000, false));
    assert_eq!(got[1], (333, false));
    assert_eq!(&got[2..], [(1000, true), (1000, false)]);
    let m = feed.muxer.metrics();
    assert!(m.dropped_awaiting_keyframe > 15, "{m:?}");
    assert_eq!(feed.reader.metrics().gaps, 1);
}

#[test]
fn a_low_latency_stream_lists_parts_as_they_come_and_a_gap_once() {
    let second = Duration::from_secs(1);
    let s = served_with(PlaylistConfig {
        part_target: second,
        ..PlaylistConfig::default()
    });
    let keyframes: Vec<i64> = (0..5).map(|g| g * 60).collect();
    let (_, recs) = units_stream(300, &keyframes);
    let mut feed = UnitFeed::new(HlsMuxerConfig {
        part_target: second,
        ..HlsMuxerConfig::default()
    });
    // Video units 130 to 134 are lost.
    for (_, r) in &recs {
        if picture_in(r, 130..135) {
            feed.feed(EgressPiece::Discontinuity, &s.stream);
            continue;
        }
        feed_units(&mut feed, &s, &[r]);
    }
    let playlist = s.playlist();
    assert_eq!(
        listed(&playlist),
        [(2000, false), (2000, false), (333, false), (2000, true)]
    );
    assert_eq!(playlist.matches("#EXT-X-DISCONTINUITY\n").count(), 1);
    // The last GOP's first part is listed after the segments, before the
    // next part's hint.
    let first_part = FIRST + 2 + 2 + 1 + 2;
    let tail = format!(
        "#EXT-X-PART:DURATION=1.000,URI=\"part-{first_part}.m4s\",INDEPENDENT=YES\n\
         #EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"part-{}.m4s\"\n",
        first_part + 1
    );
    assert!(playlist.ends_with(&tail), "{playlist}");
    // Each segment's parts are its media.
    let mut number = FIRST;
    for (seq, parts) in [(0, 2), (1, 2), (2, 1), (3, 2)] {
        let media: Vec<u8> = (0..parts)
            .flat_map(|i| s.file(&format!("part-{}.m4s", number + i)))
            .collect();
        assert!(media == s.file(&format!("{}.m4s", FIRST + seq)), "{seq}");
        number += parts;
    }
}

/// Whether unit-format record `r` is an audio unit of a frame in
/// `frames`.
fn audio_frame_in(r: &[u8], frames: std::ops::Range<i64>) -> bool {
    match parse_record(r).unwrap() {
        Record::Unit { header, .. } => header.track.0 == 2 && frames.contains(&(header.dts / 1024)),
        _ => false,
    }
}

#[test]
fn lost_audio_units_are_filled_with_silence_and_the_segment_goes_on() {
    let s = served();
    let keyframes: Vec<i64> = (0..5).map(|g| g * 30).collect();
    let (_, recs) = units_stream(150, &keyframes);
    let mut feed = unit_feed();
    // AAC frames 60 to 64 are lost, the video around them not.
    for (_, r) in &recs {
        if audio_frame_in(r, 60..65) {
            feed.feed(EgressPiece::Discontinuity, &s.stream);
            continue;
        }
        feed_units(&mut feed, &s, &[r]);
    }
    assert_eq!(listed(&s.playlist()), vec![(1000, false); 4]);
    let m = feed.muxer.metrics();
    assert_eq!((m.silence_fills, m.silence_units), (1, 5), "{m:?}");
    assert_eq!(m.dropped_awaiting_keyframe, 0, "{m:?}");
    assert_eq!(feed.reader.metrics().gaps, 1);
}

#[test]
fn a_writer_starting_over_resets_the_muxer_after_a_discontinuity() {
    let s = served();
    let keyframes: Vec<i64> = (0..3).map(|g| g * 30).collect();
    let (_, first) = units_stream(90, &keyframes);
    let (_, again) = units_stream(90, &keyframes);
    let mut feed = unit_feed();
    let a: Vec<&[u8]> = first.iter().map(|(_, r)| r.as_slice()).collect();
    let b: Vec<&[u8]> = again.iter().map(|(_, r)| r.as_slice()).collect();
    feed_units(&mut feed, &s, &a);
    feed.feed(EgressPiece::Discontinuity, &s.stream);
    feed_units(&mut feed, &s, &b);
    let got = listed(&s.playlist());
    // Each writer's last GOP ends where the stream does: the second
    // writer's first record ends the first's.
    assert_eq!(
        got,
        [
            (1000, false),
            (1000, false),
            (1000, false),
            (1000, true),
            (1000, false)
        ]
    );
}

/// An END ends the last GOP's segment and the playlist; a later
/// session's segment takes the end back off, after a discontinuity.
#[test]
fn an_end_ends_the_playlist_until_a_later_session() {
    let s = served();
    let keyframes: Vec<i64> = (0..3).map(|g| g * 30).collect();
    let (_, first) = units_stream(90, &keyframes);
    let (_, again) = units_stream(60, &keyframes);
    let mut end = Vec::new();
    crate::units::record::write_end(&mut end);
    let mut feed = unit_feed();
    let a: Vec<&[u8]> = first.iter().map(|(_, r)| r.as_slice()).collect();
    assert_eq!(feed_units(&mut feed, &s, &a).segments, 2);
    assert!(!s.playlist().contains("#EXT-X-ENDLIST"));
    assert_eq!(feed_units(&mut feed, &s, &[&end]).segments, 1);
    let playlist = s.playlist();
    assert!(playlist.ends_with("#EXT-X-ENDLIST\n"), "{playlist}");
    assert_eq!(listed(&playlist), vec![(1000, false); 3]);

    let b: Vec<&[u8]> = again.iter().map(|(_, r)| r.as_slice()).collect();
    feed.feed(EgressPiece::Discontinuity, &s.stream);
    feed_units(&mut feed, &s, &b);
    let playlist = s.playlist();
    assert!(!playlist.contains("#EXT-X-ENDLIST"), "{playlist}");
    assert_eq!(
        listed(&playlist),
        [(1000, false), (1000, false), (1000, false), (1000, true)]
    );
    assert_eq!(feed.reader.metrics().ends, 1);
}

#[test]
fn a_unit_record_over_the_bound_is_dropped() {
    let s = served();
    let mut feed = unit_feed();
    let big = vec![0u8; units::MAX_RECORD_LEN / 2 + 1];
    assert_eq!(feed.feed(EgressPiece::Data(&big), &s.stream), NOTHING);
    assert_eq!(feed.feed(EgressPiece::Data(&big), &s.stream), SKIPPED);
    assert_eq!(feed.feed(EgressPiece::Data(&big), &s.stream), NOTHING);
    assert_eq!(feed.feed(EgressPiece::RecordEnd, &s.stream), NOTHING);
    assert!(feed.held.record.is_empty());
    assert_eq!(feed.reader.metrics().lost, 1);
}

/// A swarm reader over a stream, from `from`, in reads of 3000 bytes,
/// that never has the chunks of `lost`.
struct Lossy {
    reads: VecDeque<(u64, Vec<u8>)>,
}

impl Lossy {
    fn new(stream: &[u8], from: usize, lost: std::ops::Range<usize>) -> Lossy {
        let mut reads = VecDeque::new();
        let mut at = from;
        while at < stream.len() {
            if lost.contains(&at) {
                at = lost.end;
                continue;
            }
            let end = stream.len().min(at + 3000).min(if at < lost.start {
                lost.start
            } else {
                usize::MAX
            });
            reads.push_back((at as u64, stream[at..end].to_vec()));
            at = end;
        }
        Lossy { reads }
    }
}

impl Rewind for Lossy {
    async fn rewind(&mut self, _: u64) -> Option<u64> {
        None
    }
}

impl SwarmRead for Lossy {
    type Error = String;

    async fn read_at(&mut self, out: &mut [u8]) -> Result<(u64, usize), ReadFailure<String>> {
        let Some((offset, data)) = self.reads.pop_front() else {
            return Err(ReadFailure::Closed);
        };
        let n = data.len().min(out.len());
        out[..n].copy_from_slice(&data[..n]);
        if n < data.len() {
            self.reads
                .push_front((offset + n as u64, data[n..].to_vec()));
        }
        Ok((offset, n))
    }
}

/// Reads `reader`'s stream through the chunked layout into a unit feed.
fn read_chunked(reader: Lossy, s: &Served) -> UnitFeed {
    let mut r = ChunkedReader::new(reader, CHUNK, 64 * 1024).unwrap();
    let mut feed = unit_feed();
    loop {
        match r
            .next(usize::MAX)
            .now_or_never()
            .expect("the reader never waits")
        {
            Ok(piece) => {
                feed.feed(piece, &s.stream);
            }
            Err(VideoError::StreamClosed) => return feed,
            Err(e) => panic!("{e}"),
        }
    }
}

#[test]
fn a_lost_chunk_costs_the_units_to_the_next_keyframe() {
    let keyframes: Vec<i64> = (0..5).map(|g| g * 30).collect();
    let (stream, recs) = units_stream(150, &keyframes);
    let whole = served();
    read_chunked(Lossy::new(&stream, 0, 0..0), &whole);
    assert_eq!(listed(&whole.playlist()), vec![(1000, false); 4]);

    // Lose a chunk in the middle of the second GOP.
    let mid = recs.iter().find(|(_, r)| picture_in(r, 45..46)).unwrap().0 as usize;
    let lost = mid / CHUNK * CHUNK;
    let s = served();
    let feed = read_chunked(Lossy::new(&stream, 0, lost..lost + CHUNK), &s);
    let got = listed(&s.playlist());
    assert_eq!(got.len(), 4);
    assert_eq!(got[0], (1000, false));
    // The second ends before the lost units; the third starts at the next
    // keyframe, after a discontinuity.
    assert!((400..=500).contains(&got[1].0), "{got:?}");
    assert_eq!(&got[2..], [(1000, true), (1000, false)]);
    assert!(feed.reader.metrics().gaps >= 1);
}

#[test]
fn a_join_mid_gop_starts_at_the_next_keyframe_when_it_can_not_rewind() {
    let keyframes: Vec<i64> = (0..5).map(|g| g * 30).collect();
    let (stream, _) = units_stream(150, &keyframes);
    let s = served();
    let join = stream.len() / 3 / CHUNK * CHUNK;
    let feed = read_chunked(Lossy::new(&stream, join, 0..0), &s);
    let got = listed(&s.playlist());
    assert!(!got.is_empty());
    assert!(got.iter().all(|&(ms, d)| ms == 1000 && !d), "{got:?}");
    // The join dropped the units before the next GOP's sync record.
    assert_eq!(feed.reader.metrics().unknown_tracks, 0);
}

/// A piece a [`Scripted`] stream yields.
enum Step {
    Record(Vec<u8>),
    Fail(VideoError),
}

/// A stream of scripted steps; it ends when they run out. The record a
/// step holds is yielded as one data piece, then its end.
#[derive(Default)]
struct ScriptedStream {
    steps: VecDeque<Step>,
    /// The record whose data was yielded, and whose end comes next.
    record: Option<Vec<u8>>,
}

/// A piece source of scripted streams, whose opens fail or succeed in
/// turn; once they run out, it keeps the playlist's text and drops its
/// stream, which ends the serving.
struct Scripted {
    opens: VecDeque<Result<ScriptedStream, VideoError>>,
    waits: Vec<Duration>,
    routes: HlsRoutes,
    served: Option<Rc<HlsStream>>,
    playlist: String,
}

impl PieceSource for Scripted {
    type Stream = ScriptedStream;

    async fn open(&mut self) -> Result<ScriptedStream, VideoError> {
        self.opens.pop_front().unwrap_or_else(|| {
            if let Some(s) = self.served.take() {
                let body = self.routes.get(T, s.path()).body;
                self.playlist = String::from_utf8(body.to_vec()).unwrap();
            }
            Err(VideoError::StreamClosed)
        })
    }

    async fn next<'a>(&mut self, s: &'a mut ScriptedStream) -> Result<EgressPiece<'a>, VideoError> {
        if s.record.take().is_some() {
            return Ok(EgressPiece::RecordEnd);
        }
        match s.steps.pop_front() {
            Some(Step::Record(r)) => Ok(EgressPiece::Data(s.record.insert(r))),
            Some(Step::Fail(e)) => Err(e),
            None => Err(VideoError::StreamClosed),
        }
    }

    async fn wait(&mut self, d: Duration) {
        self.waits.push(d);
    }
}

/// A scripted stream of `records`, with a failed read after the first
/// `fail_after` when given.
fn scripted(records: &[(u64, Vec<u8>)], fail_after: Option<usize>) -> ScriptedStream {
    let mut steps: VecDeque<Step> = records
        .iter()
        .map(|(_, r)| Step::Record(r.clone()))
        .collect();
    if let Some(n) = fail_after {
        steps.insert(n, Step::Fail(VideoError::StreamRead("store".into())));
    }
    ScriptedStream {
        steps,
        record: None,
    }
}

#[test]
fn a_stream_reads_on_after_failures_and_opens_again_once_it_ends() {
    let routes = HlsRoutes::new();
    let profile = routes.profile(&[1; 32]).unwrap();
    let served = Rc::new(
        profile
            .open_stream(T, [2; 32], PlaylistConfig::default())
            .unwrap(),
    );
    let keyframes: Vec<i64> = (0..3).map(|g| g * 30).collect();
    let (_, recs) = units_stream(90, &keyframes);
    let half = recs.len() / 2;
    let mut source = Scripted {
        opens: VecDeque::from([
            Err(VideoError::TooManyStreams),
            // A later session: its configs start the muxer over.
            Ok(scripted(&recs, None)),
        ]),
        waits: Vec::new(),
        routes: routes.clone(),
        served: Some(served.clone()),
        playlist: String::new(),
    };
    let weak = Rc::downgrade(&served);
    drop(served);
    let mut feed = unit_feed();
    let mut events = Vec::new();
    let mut segments = 0;
    let backoff = Backoff::new(Duration::from_secs(1), Duration::from_secs(3));
    let first = scripted(&recs, Some(half));
    serve(&mut source, Some(first), &mut feed, &weak, backoff, |e| {
        events.push(match e {
            ServeEvent::Fed(fed) => {
                segments += fed.segments;
                return;
            }
            ServeEvent::Reopened => "reopened".to_owned(),
            ServeEvent::Failed(e) => e.to_string(),
        })
    })
    .now_or_never()
    .expect("the script never waits");
    let secs: Vec<i64> = source
        .waits
        .iter()
        .map(|d| d.as_nanos() / 1_000_000_000)
        .collect();
    // The failed read, the end, the refused open, the second end and the
    // end of the script.
    assert_eq!(secs, [1, 1, 2, 1, 2]);
    assert_eq!(
        events,
        [
            "reading the stream failed: store",
            "stream closed",
            "too many streams",
            "reopened",
            "stream closed",
            "stream closed",
        ]
    );
    assert!(weak.upgrade().is_none());
    // Each run's two whole GOPs, and the first run's last, which the later
    // session's configs end.
    assert_eq!(segments, 5);
    let got = listed(&source.playlist);
    assert_eq!(
        got,
        [
            (1000, false),
            (1000, false),
            (1000, false),
            (1000, true),
            (1000, false)
        ]
    );
}
