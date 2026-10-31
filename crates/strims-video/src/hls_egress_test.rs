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
use crate::CutRule;
use crate::egress::{ChunkedReader, Rewind};
use crate::synth::script::{Script, ScriptConfig, expected_records};
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

fn records(config: &ScriptConfig) -> Vec<Vec<u8>> {
    expected_records(&Script::new(config), CutRule::default())
}

fn record_feed(max_record_bytes: usize) -> RecordFeed {
    RecordFeed::new(max_record_bytes, 4)
}

const SEGMENT: Fed = Fed {
    segments: 1,
    skipped: 0,
};
const SKIPPED: Fed = Fed {
    segments: 0,
    skipped: 1,
};
const NOTHING: Fed = Fed {
    segments: 0,
    skipped: 0,
};

/// Feeds `data` in pieces, and ends the record if `end`: what the pieces
/// did, summed.
fn feed_record(feed: &mut RecordFeed, s: &Served, data: &[u8], end: bool) -> Fed {
    let mut fed = Fed::default();
    let mut add = |f: Fed| {
        fed.segments += f.segments;
        fed.skipped += f.skipped;
    };
    for piece in data.chunks(10_000) {
        add(feed.feed(EgressPiece::Data(piece), &s.stream));
    }
    if end {
        add(feed.feed(EgressPiece::RecordEnd, &s.stream));
    }
    fed
}

/// The EXTINF line of a segment lasting `d`.
fn extinf_of(d: Duration) -> String {
    let ms = (d.as_nanos() + 500_000) / 1_000_000;
    format!("#EXTINF:{}.{:03},", ms / 1000, ms % 1000)
}

/// The EXTINF a segment of `record` is listed with.
fn extinf(record: &[u8]) -> String {
    let (init, media) = split_record(record).unwrap();
    extinf_of(media_duration(&parse_init(init).unwrap(), media).unwrap())
}

/// The playlist's EXTINF and EXT-X-DISCONTINUITY lines.
fn tags(playlist: &str) -> Vec<&str> {
    playlist
        .lines()
        .filter(|l| l.starts_with("#EXTINF") || *l == "#EXT-X-DISCONTINUITY")
        .collect()
}

/// The listed segments' durations, in milliseconds.
fn listed_millis(playlist: &str) -> Vec<u64> {
    playlist
        .lines()
        .filter_map(|l| l.strip_prefix("#EXTINF:"))
        .map(|l| {
            let (s, f) = l.trim_end_matches(',').split_once('.').unwrap();
            s.parse::<u64>().unwrap() * 1000 + f.parse::<u64>().unwrap()
        })
        .collect()
}

#[test]
fn whole_records_are_segments_lasting_their_media_time() {
    let s = served();
    let rs = records(&ScriptConfig::default());
    assert!(rs.len() >= 4);
    let mut feed = record_feed(16 << 20);
    for r in &rs {
        assert_eq!(feed_record(&mut feed, &s, r, true), SEGMENT);
    }
    let playlist = s.playlist();
    let listed: Vec<&str> = tags(&playlist);
    let expected: Vec<String> = rs.iter().map(|r| extinf(r)).collect();
    assert_eq!(listed, expected);
    assert!(playlist.contains(&format!("#EXT-X-MAP:URI=\"init-{FIRST}.mp4\"\n")));
    let (init, _) = split_record(&rs[0]).unwrap();
    assert_eq!(s.file(&format!("init-{FIRST}.mp4")), init);
    for (i, r) in rs.iter().enumerate() {
        let (_, media) = split_record(r).unwrap();
        assert_eq!(s.file(&format!("{}.m4s", FIRST + i as u64)), media, "{i}");
    }
}

#[test]
fn a_record_longer_than_the_target_is_split_at_its_fragments() {
    let s = served();
    // 10 s GOPs, in 100 ms fragments.
    let config = ScriptConfig {
        gop_frames: 300,
        duration_secs: 20,
        ..ScriptConfig::default()
    };
    let rs = records(&config);
    let mut feed = record_feed(16 << 20);
    let fed = feed_record(&mut feed, &s, &rs[0], true);
    assert_eq!(
        fed,
        Fed {
            segments: 3,
            skipped: 0
        }
    );
    let playlist = s.playlist();
    let millis = listed_millis(&playlist);
    assert_eq!(millis.len(), 3);
    assert!(millis.iter().all(|&m| (m + 500) / 1000 <= 4), "{playlist}");
    assert_eq!(
        tags(&playlist)
            .iter()
            .filter(|t| t.contains("DISCONTINUITY"))
            .count(),
        0
    );
    // The pieces are the record's media in order, each a run of whole
    // fragments.
    let (init, media) = split_record(&rs[0]).unwrap();
    let pieces: Vec<Vec<u8>> = (0..3)
        .map(|i| s.file(&format!("{}.m4s", FIRST + i)))
        .collect();
    assert_eq!(pieces.concat(), media);
    let info = parse_init(init).unwrap();
    let total: Duration = pieces
        .iter()
        .map(|p| media_duration(&info, p).unwrap())
        .fold(Duration::ZERO, |a, b| a + b);
    assert_eq!(total, media_duration(&info, media).unwrap());
    for p in &pieces {
        assert_eq!(&p[4..8], b"prft");
    }
}

#[test]
fn a_record_whose_fragment_outlasts_the_target_is_skipped_from_it() {
    let s = served();
    // One fragment a record: a 10 s GOP can't be cut.
    let config = ScriptConfig {
        gop_frames: 300,
        duration_secs: 20,
        ..ScriptConfig::default()
    };
    let whole = expected_records(
        &Script::new(&config),
        CutRule {
            fragment_duration: Duration::MAX,
            ..CutRule::default()
        },
    );
    let short = records(&ScriptConfig::default());
    let mut feed = record_feed(16 << 20);
    assert_eq!(feed_record(&mut feed, &s, &whole[0], true), SKIPPED);
    assert!(listed_millis(&s.playlist()).is_empty());

    // A record of fitting fragments, then the long one: the first are
    // served, the rest is skipped and the next segment discontinuous.
    let (init, media) = split_record(&short[0]).unwrap();
    let (_, long) = split_record(&whole[0]).unwrap();
    let mut joined = (init.len() as u16).to_be_bytes().to_vec();
    joined.extend_from_slice(init);
    joined.extend_from_slice(media);
    joined.extend_from_slice(long);
    assert_eq!(
        feed_record(&mut feed, &s, &joined, true),
        Fed {
            segments: 1,
            skipped: 1
        }
    );
    feed_record(&mut feed, &s, &short[1], true);
    assert_eq!(
        tags(&s.playlist()),
        [
            extinf(&short[0]).as_str(),
            "#EXT-X-DISCONTINUITY",
            &extinf(&short[1])
        ]
    );
}

#[test]
fn fragments_are_the_boxes_up_to_each_mdat() {
    let b = |kind: &[u8; 4], len: u32| {
        let mut v = len.to_be_bytes().to_vec();
        v.extend_from_slice(kind);
        v.resize(len as usize, 0);
        v
    };
    let media = [
        b(b"prft", 32),
        b(b"moof", 16),
        b(b"mdat", 12),
        b(b"moof", 8),
        b(b"mdat", 8),
        b(b"free", 9),
    ]
    .concat();
    assert_eq!(fragments(&media), Some(vec![0..60, 60..85]));
    // A 64-bit size, and a box to the end.
    let mut large = 1u32.to_be_bytes().to_vec();
    large.extend_from_slice(b"mdat");
    large.extend_from_slice(&20u64.to_be_bytes());
    large.extend_from_slice(&[0; 4]);
    let one = |r: std::ops::Range<usize>| Some(Vec::from([r]));
    assert_eq!(fragments(&large), one(0..20));
    let mut to_end = 0u32.to_be_bytes().to_vec();
    to_end.extend_from_slice(b"mdat");
    to_end.extend_from_slice(&[0; 9]);
    assert_eq!(fragments(&to_end), one(0..17));
    // No mdat, a short size, one past the end, a cut header.
    assert_eq!(fragments(&b(b"moof", 16)), None);
    assert_eq!(fragments(&[0, 0, 0, 7, b'm', b'd', b'a', b't']), None);
    assert_eq!(fragments(&[0, 0, 0, 9, b'm', b'd', b'a', b't']), None);
    assert_eq!(fragments(&[0, 0, 0]), None);
    assert_eq!(fragments(&[]), None);
}

#[test]
fn a_discontinuity_marks_the_next_segment_and_drops_the_record_in_progress() {
    let s = served();
    let rs = records(&ScriptConfig::default());
    let mut feed = record_feed(16 << 20);
    feed_record(&mut feed, &s, &rs[0], true);
    feed_record(&mut feed, &s, &rs[1][..50_000], false);
    assert_eq!(feed.feed(EgressPiece::Discontinuity, &s.stream), NOTHING);
    assert_eq!(feed_record(&mut feed, &s, &rs[2], true), SEGMENT);
    assert_eq!(
        tags(&s.playlist()),
        [
            extinf(&rs[0]).as_str(),
            "#EXT-X-DISCONTINUITY",
            &extinf(&rs[2])
        ]
    );
    let (_, media) = split_record(&rs[2]).unwrap();
    assert_eq!(s.file(&format!("{}.m4s", FIRST + 1)), media);
}

#[test]
fn a_record_over_the_bound_is_dropped_then_a_discontinuity() {
    let s = served();
    let rs = records(&ScriptConfig::default());
    let max = rs.iter().map(Vec::len).max().unwrap();
    let mut feed = record_feed(max);
    feed_record(&mut feed, &s, &rs[0], true);
    let mut large = rs[1].clone();
    large.resize(max + 50_000, 0);
    let fed: Vec<Fed> = large
        .chunks(10_000)
        .map(|piece| feed.feed(EgressPiece::Data(piece), &s.stream))
        .collect();
    assert_eq!(fed.iter().filter(|f| **f == SKIPPED).count(), 1);
    assert_eq!(feed.feed(EgressPiece::RecordEnd, &s.stream), NOTHING);
    assert_eq!(feed_record(&mut feed, &s, &rs[2], true), SEGMENT);
    assert_eq!(
        tags(&s.playlist()),
        [
            extinf(&rs[0]).as_str(),
            "#EXT-X-DISCONTINUITY",
            &extinf(&rs[2])
        ]
    );
    assert!(feed.held.record.capacity() <= max);
}

#[test]
fn a_record_over_the_bound_is_skipped_once() {
    let s = served();
    let mut feed = record_feed(1000);
    let fed: Vec<Fed> = [400usize, 400, 400, 400]
        .iter()
        .map(|&n| feed.feed(EgressPiece::Data(&vec![0; n]), &s.stream))
        .collect();
    assert_eq!(fed, [NOTHING, NOTHING, SKIPPED, NOTHING]);
    assert_eq!(feed.feed(EgressPiece::RecordEnd, &s.stream), NOTHING);
    assert!(feed.held.record.is_empty());
}

#[test]
fn records_that_are_not_fmp4_are_skipped() {
    let s = served();
    let rs = records(&ScriptConfig::default());
    let mut feed = record_feed(16 << 20);
    // A record whose 16-byte init doesn't parse.
    let mut junk = vec![0x00, 0x10];
    junk.resize(70_000, 0x42);
    assert_eq!(feed_record(&mut feed, &s, &junk, true), SKIPPED);
    assert_eq!(feed_record(&mut feed, &s, &[0x00], true), SKIPPED);
    assert_eq!(feed_record(&mut feed, &s, &[], true), SKIPPED);
    assert_eq!(tags(&s.playlist()), Vec::<&str>::new());

    // Before any segment, a skip is no discontinuity; after one, it is.
    feed_record(&mut feed, &s, &rs[0], true);
    assert_eq!(feed_record(&mut feed, &s, &junk, true), SKIPPED);
    // An init alone has no media time.
    let (init, _) = split_record(&rs[1]).unwrap();
    let mut init_only = (init.len() as u16).to_be_bytes().to_vec();
    init_only.extend_from_slice(init);
    assert_eq!(feed_record(&mut feed, &s, &init_only, true), SKIPPED);
    feed_record(&mut feed, &s, &rs[1], true);
    assert_eq!(
        tags(&s.playlist()),
        [
            extinf(&rs[0]).as_str(),
            "#EXT-X-DISCONTINUITY",
            &extinf(&rs[1])
        ]
    );
}

#[test]
fn a_changed_init_is_parsed_and_mapped_again() {
    let s = served();
    let config = ScriptConfig {
        resize_at_gop: Some(2),
        ..ScriptConfig::default()
    };
    let rs = records(&config);
    let inits: Vec<&[u8]> = rs.iter().map(|r| split_record(r).unwrap().0).collect();
    let changed = (1..inits.len())
        .find(|&i| inits[i] != inits[i - 1])
        .unwrap();
    let mut feed = record_feed(16 << 20);
    for r in &rs {
        assert_eq!(feed_record(&mut feed, &s, r, true), SEGMENT);
    }
    assert_eq!(feed.init.init, inits[inits.len() - 1]);
    let playlist = s.playlist();
    assert_eq!(playlist.matches("#EXT-X-MAP").count(), 2, "{playlist}");
    let second = FIRST + 1;
    assert!(playlist.contains(&format!("#EXT-X-MAP:URI=\"init-{second}.mp4\"")));
    assert_eq!(s.file(&format!("init-{second}.mp4")), inits[changed]);
    let expected: Vec<String> = rs
        .iter()
        .enumerate()
        .flat_map(|(i, r)| {
            let mut t = Vec::new();
            if i == changed {
                t.push("#EXT-X-DISCONTINUITY".to_owned());
            }
            t.push(extinf(r));
            t
        })
        .collect();
    assert_eq!(tags(&playlist), expected);
}

/// The frame size of the unit-format streams.
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
