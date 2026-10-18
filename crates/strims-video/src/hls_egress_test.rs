// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use super::*;
use crate::CutRule;
use crate::synth::script::{Script, ScriptConfig, expected_records};

const T: Time = Time::from_unix_millis(1_700_000_000_000);

/// A stream registered on its own routes.
struct Served {
    routes: HlsRoutes,
    _profile: HlsProfile,
    stream: HlsStream,
}

fn served() -> Served {
    let routes = HlsRoutes::new();
    let profile = routes.profile(&[1; 32]).unwrap();
    let stream = profile
        .open_stream(T, [2; 32], PlaylistConfig::default())
        .unwrap();
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

/// Feeds `data` in pieces, and ends the record if `end`: what the last
/// piece did.
fn feed_record(feed: &mut RecordFeed, s: &Served, data: &[u8], end: bool) -> Fed {
    let mut fed = Fed::Nothing;
    for piece in data.chunks(10_000) {
        fed = feed.feed(EgressPiece::Data(piece), &s.stream);
    }
    if end {
        fed = feed.feed(EgressPiece::RecordEnd, &s.stream);
    }
    fed
}

/// The EXTINF a segment of `record` is listed with.
fn extinf(record: &[u8]) -> String {
    let (init, media) = split_record(record).unwrap();
    let d = media_duration(&parse_init(init).unwrap(), media).unwrap();
    let ms = (d.as_nanos() + 500_000) / 1_000_000;
    format!("#EXTINF:{}.{:03},", ms / 1000, ms % 1000)
}

/// The playlist's EXTINF and EXT-X-DISCONTINUITY lines.
fn tags(playlist: &str) -> Vec<&str> {
    playlist
        .lines()
        .filter(|l| l.starts_with("#EXTINF") || *l == "#EXT-X-DISCONTINUITY")
        .collect()
}

#[test]
fn whole_records_are_segments_lasting_their_media_time() {
    let s = served();
    let rs = records(&ScriptConfig::default());
    assert!(rs.len() >= 4);
    let mut feed = RecordFeed::new(16 << 20);
    for r in &rs {
        assert_eq!(feed_record(&mut feed, &s, r, true), Fed::Segment);
    }
    let playlist = s.playlist();
    let listed: Vec<&str> = tags(&playlist);
    let expected: Vec<String> = rs.iter().map(|r| extinf(r)).collect();
    assert_eq!(listed, expected);
    let first = 1_700_000_000_000u64;
    assert!(playlist.contains(&format!("#EXT-X-MAP:URI=\"init-{first}.mp4\"\n")));
    let (init, _) = split_record(&rs[0]).unwrap();
    assert_eq!(s.file(&format!("init-{first}.mp4")), init);
    for (i, r) in rs.iter().enumerate() {
        let (_, media) = split_record(r).unwrap();
        assert_eq!(s.file(&format!("{}.m4s", first + i as u64)), media, "{i}");
    }
}

#[test]
fn a_discontinuity_marks_the_next_segment_and_drops_the_record_in_progress() {
    let s = served();
    let rs = records(&ScriptConfig::default());
    let mut feed = RecordFeed::new(16 << 20);
    feed_record(&mut feed, &s, &rs[0], true);
    feed_record(&mut feed, &s, &rs[1][..50_000], false);
    assert_eq!(
        feed.feed(EgressPiece::Discontinuity, &s.stream),
        Fed::Nothing
    );
    assert_eq!(feed_record(&mut feed, &s, &rs[2], true), Fed::Segment);
    assert_eq!(
        tags(&s.playlist()),
        [
            extinf(&rs[0]).as_str(),
            "#EXT-X-DISCONTINUITY",
            &extinf(&rs[2])
        ]
    );
    let (_, media) = split_record(&rs[2]).unwrap();
    assert_eq!(s.file("1700000000001.m4s"), media);
}

#[test]
fn a_record_over_the_bound_is_dropped_then_a_discontinuity() {
    let s = served();
    let rs = records(&ScriptConfig::default());
    let max = rs.iter().map(Vec::len).max().unwrap();
    let mut feed = RecordFeed::new(max);
    feed_record(&mut feed, &s, &rs[0], true);
    let mut large = rs[1].clone();
    large.resize(max + 50_000, 0);
    let fed: Vec<Fed> = large
        .chunks(10_000)
        .map(|piece| feed.feed(EgressPiece::Data(piece), &s.stream))
        .collect();
    assert_eq!(fed.iter().filter(|f| **f == Fed::Skipped).count(), 1);
    assert_eq!(feed.feed(EgressPiece::RecordEnd, &s.stream), Fed::Nothing);
    assert_eq!(feed_record(&mut feed, &s, &rs[2], true), Fed::Segment);
    assert_eq!(
        tags(&s.playlist()),
        [
            extinf(&rs[0]).as_str(),
            "#EXT-X-DISCONTINUITY",
            &extinf(&rs[2])
        ]
    );
    assert!(feed.record.capacity() <= max);
}

#[test]
fn a_record_over_the_bound_is_skipped_once() {
    let s = served();
    let mut feed = RecordFeed::new(1000);
    let fed: Vec<Fed> = [400usize, 400, 400, 400]
        .iter()
        .map(|&n| feed.feed(EgressPiece::Data(&vec![0; n]), &s.stream))
        .collect();
    assert_eq!(
        fed,
        [Fed::Nothing, Fed::Nothing, Fed::Skipped, Fed::Nothing]
    );
    assert_eq!(feed.feed(EgressPiece::RecordEnd, &s.stream), Fed::Nothing);
    assert!(feed.record.is_empty());
}

#[test]
fn records_that_are_not_fmp4_are_skipped() {
    let s = served();
    let rs = records(&ScriptConfig::default());
    let mut feed = RecordFeed::new(16 << 20);
    // A WebM record: an EBML header, then clusters.
    let mut webm = vec![0x1a, 0x45, 0xdf, 0xa3];
    webm.resize(70_000, 0x42);
    assert_eq!(feed_record(&mut feed, &s, &webm, true), Fed::Skipped);
    assert_eq!(feed_record(&mut feed, &s, &[0x00], true), Fed::Skipped);
    assert_eq!(feed_record(&mut feed, &s, &[], true), Fed::Skipped);
    assert_eq!(tags(&s.playlist()), Vec::<&str>::new());

    // Before any segment, a skip is no discontinuity; after one, it is.
    feed_record(&mut feed, &s, &rs[0], true);
    assert_eq!(feed_record(&mut feed, &s, &webm, true), Fed::Skipped);
    // An init alone has no media time.
    let (init, _) = split_record(&rs[1]).unwrap();
    let mut init_only = (init.len() as u16).to_be_bytes().to_vec();
    init_only.extend_from_slice(init);
    assert_eq!(feed_record(&mut feed, &s, &init_only, true), Fed::Skipped);
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
    let mut feed = RecordFeed::new(16 << 20);
    for r in &rs {
        assert_eq!(feed_record(&mut feed, &s, r, true), Fed::Segment);
    }
    assert_eq!(feed.init.init, inits[inits.len() - 1]);
    let playlist = s.playlist();
    assert_eq!(playlist.matches("#EXT-X-MAP").count(), 2, "{playlist}");
    let second = 1_700_000_000_001u64;
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
