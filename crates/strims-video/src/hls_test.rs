// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::collections::BTreeMap;

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::*;

const HEAD: &str = "#EXTM3U\n#EXT-X-VERSION:7\n";

fn ms(n: i64) -> Duration {
    Duration::from_millis(n)
}

fn at(ms: i64) -> Time {
    Time::from_unix_millis(ms)
}

/// A playlist config of target `target`, listing `listed` and keeping
/// `retained` milliseconds of segments within `max_retained_bytes`.
fn config(target: u64, listed: i64, retained: i64, max_retained_bytes: usize) -> PlaylistConfig {
    PlaylistConfig {
        target,
        listed: ms(listed),
        retained: ms(retained),
        max_retained_bytes,
        part_target: Duration::ZERO,
    }
}

fn header(target: u64, media_sequence: u64, discontinuity_sequence: u64) -> String {
    format!(
        "{HEAD}#EXT-X-TARGETDURATION:{target}\n#EXT-X-MEDIA-SEQUENCE:{media_sequence}\n\
         #EXT-X-DISCONTINUITY-SEQUENCE:{discontinuity_sequence}\n"
    )
}

fn entry(millis: &str, seq: u64) -> String {
    format!("#EXTINF:{millis},\n{seq}.m4s\n")
}

fn map(id: u64) -> String {
    format!("#EXT-X-MAP:URI=\"init-{id}.mp4\"\n")
}

const DISCONTINUITY: &str = "#EXT-X-DISCONTINUITY\n";

#[test]
fn default_config_targets_4_s_lists_16_s_and_keeps_40_s_within_64_mib() {
    assert_eq!(
        PlaylistConfig::default(),
        config(4, 16_000, 40_000, 64 << 20)
    );
}

#[test]
fn renders_an_empty_playlist() {
    let p = Playlist::new(PlaylistConfig::default());
    assert_eq!(p.render(), header(4, 0, 0));
    assert_eq!(p.init_id(), None);
    assert_eq!(p.segment(0), None);
}

#[test]
fn renders_segments_sharing_an_init() {
    let mut p = Playlist::new(PlaylistConfig::default());
    p.push_segment(b"init", b"m0", ms(2002));
    p.push_segment(b"init", b"m1", ms(1500));
    p.push_segment(b"init", b"m2", Duration::from_nanos(2_499_600_000));
    p.push_segment(b"init", b"m3", Duration::from_nanos(999_999_999));
    p.push_segment(b"init", b"m4", Duration::from_nanos(1_499_999));
    let want = [
        header(4, 0, 0),
        map(0),
        entry("2.002", 0),
        entry("1.500", 1),
        entry("2.500", 2),
        entry("1.000", 3),
        entry("0.001", 4),
    ]
    .concat();
    assert_eq!(p.render(), want);
    assert_eq!(p.target_duration(), 4);
    assert_eq!(p.segment(2).as_deref(), Some(&b"m2"[..]));
    assert_eq!(p.init(0).as_deref(), Some(&b"init"[..]));
}

#[test]
fn the_playlist_never_declares_independent_segments() {
    let mut p = Playlist::new(PlaylistConfig::default());
    assert!(!p.render().contains("INDEPENDENT"));
    p.push_segment(b"init", b"m0", ms(1000));
    assert!(!p.render().contains("INDEPENDENT"));
}

#[test]
fn sliding_window_advances_the_media_sequence() {
    let mut p = Playlist::new(config(1, 3000, 4000, 1 << 20));
    for i in 0..6u8 {
        p.push_segment(b"init", &[i], ms(1000));
    }
    let want = [
        header(1, 3, 0),
        map(0),
        entry("1.000", 3),
        entry("1.000", 4),
        entry("1.000", 5),
    ]
    .concat();
    assert_eq!(p.render(), want);
    assert_eq!(p.media_sequence(), 3);
    assert_eq!(p.next_sequence(), 6);
    assert_eq!((p.listed_segments(), p.retained_segments()), (3, 4));
    // Kept but no longer listed.
    assert_eq!(p.segment(2).as_deref(), Some(&[2u8][..]));
    assert_eq!(p.segment(1), None);
    assert_eq!(p.segment(6), None);
}

#[test]
fn the_listing_holds_three_targets_of_short_segments() {
    // Listing 1 s asks for less than three 2 s targets.
    let mut p = Playlist::new(config(2, 1000, 1000, 1 << 20));
    for i in 0..20u8 {
        p.push_segment(b"init", &[i], ms(500));
    }
    assert_eq!((p.listed_segments(), p.retained_segments()), (12, 12));
    assert_eq!(p.media_sequence(), 8);

    // The fewest newest segments that last three targets.
    let mut p = Playlist::new(config(2, 0, 0, 1 << 20));
    for _ in 0..4 {
        p.push_segment(b"init", b"m", ms(2000));
    }
    assert_eq!((p.listed_segments(), p.retained_segments()), (3, 3));
    p.push_segment(b"init", b"m", ms(1));
    assert_eq!((p.listed_segments(), p.retained_segments()), (4, 4));
}

#[test]
fn discontinuity_flags_the_next_segment_and_counts_once_it_slides_out() {
    let mut p = Playlist::new(config(1, 3000, 3000, 1 << 20));
    p.push_segment(b"init", b"m0", ms(1000));
    p.push_segment(b"init", b"m1", ms(1000));
    p.discontinuity();
    p.discontinuity();
    p.push_segment(b"init", b"m2", ms(1000));
    let want = [
        header(1, 0, 0),
        map(0),
        entry("1.000", 0),
        entry("1.000", 1),
        DISCONTINUITY.to_owned(),
        entry("1.000", 2),
    ]
    .concat();
    assert_eq!(p.render(), want);

    // The flagged segment is listed first; its tag stays with it.
    p.push_segment(b"init", b"m3", ms(1000));
    p.push_segment(b"init", b"m4", ms(1000));
    let want = [
        header(1, 2, 0),
        DISCONTINUITY.to_owned(),
        map(0),
        entry("1.000", 2),
        entry("1.000", 3),
        entry("1.000", 4),
    ]
    .concat();
    assert_eq!(p.render(), want);

    p.push_segment(b"init", b"m5", ms(1000));
    let want = [
        header(1, 3, 1),
        map(0),
        entry("1.000", 3),
        entry("1.000", 4),
        entry("1.000", 5),
    ]
    .concat();
    assert_eq!(p.render(), want);
    assert_eq!(p.discontinuity_sequence(), 1);
}

#[test]
fn changed_init_starts_a_discontinuity_with_a_new_map() {
    let mut p = Playlist::new(PlaylistConfig::default());
    p.push_segment(b"init a", b"m0", ms(1000));
    p.push_segment(b"init a", b"m1", ms(1000));
    p.push_segment(b"init b", b"m2", ms(1000));
    p.push_segment(b"init b", b"m3", ms(1000));
    // A return to an earlier init is another change.
    p.push_segment(b"init a", b"m4", ms(1000));
    let want = [
        header(4, 0, 0),
        map(0),
        entry("1.000", 0),
        entry("1.000", 1),
        DISCONTINUITY.to_owned(),
        map(1),
        entry("1.000", 2),
        entry("1.000", 3),
        DISCONTINUITY.to_owned(),
        map(2),
        entry("1.000", 4),
    ]
    .concat();
    assert_eq!(p.render(), want);
    assert_eq!(p.init(0).as_deref(), Some(&b"init a"[..]));
    assert_eq!(p.init(1).as_deref(), Some(&b"init b"[..]));
    assert_eq!(p.init(2).as_deref(), Some(&b"init a"[..]));
    assert_eq!(p.init(3), None);
    assert_eq!(p.init_id(), Some(2));
}

#[test]
fn init_change_and_flagged_discontinuity_share_one_tag() {
    let mut p = Playlist::new(config(1, 3000, 3000, 1 << 20));
    p.push_segment(b"a", b"m0", ms(1000));
    p.discontinuity();
    p.push_segment(b"b", b"m1", ms(1000));
    let want = [
        header(1, 0, 0),
        map(0),
        entry("1.000", 0),
        DISCONTINUITY.to_owned(),
        map(1),
        entry("1.000", 1),
    ]
    .concat();
    assert_eq!(p.render(), want);
    for _ in 0..3 {
        p.push_segment(b"b", b"m", ms(1000));
    }
    assert_eq!(p.discontinuity_sequence(), 1);
}

#[test]
fn identical_init_is_shared() {
    let mut p = Playlist::new(PlaylistConfig::default());
    let (init, same) = (b"init".to_vec(), b"init".to_vec());
    p.push_segment(&init, b"m0", ms(1000));
    let first = p.init(0).unwrap();
    p.push_segment(&same, b"m11", ms(1000));
    assert_eq!(p.init_id(), Some(0));
    assert!(Rc::ptr_eq(&first, &p.init(0).unwrap()));
    assert_eq!(p.retained_bytes(), 4 + 2 + 3);
    assert!(!p.render().contains(DISCONTINUITY));
}

#[test]
fn the_target_duration_is_the_configs_whatever_is_pushed() {
    let mut p = Playlist::new(PlaylistConfig::default());
    for d in [400, 4000, 4499, 4500, 9000, 100] {
        p.push_segment(b"i", b"m", ms(d));
        assert_eq!(p.target_duration(), 4);
        assert!(p.render().contains("#EXT-X-TARGETDURATION:4\n"));
    }
    let p = Playlist::new(config(0, 0, 0, 0));
    assert_eq!(p.target_duration(), 1);
    assert!(p.render().contains("#EXT-X-TARGETDURATION:1\n"));
}

#[test]
fn segments_fit_a_target_when_they_round_to_at_most_it() {
    assert!(fits_target(ms(4499), 4));
    assert!(fits_target(Duration::from_nanos(4_499_499_999), 4));
    // 4.4995 s is listed as 4.500, which rounds to 5.
    assert!(!fits_target(Duration::from_nanos(4_499_500_000), 4));
    assert!(!fits_target(ms(4500), 4));
    assert!(fits_target(ms(-5), 4));
    assert!(fits_target(ms(1499), 0));
    assert!(!fits_target(Duration::MAX, 1_000_000));
    assert!(fits_target(Duration::MAX, u64::MAX));
    assert_eq!(target_secs(ms(4000)), 4);
    assert_eq!(target_secs(ms(4001)), 5);
    assert_eq!(target_secs(ms(0)), 1);
    assert_eq!(target_secs(ms(-3000)), 1);
}

#[test]
fn out_of_range_durations_render() {
    let mut p = Playlist::new(PlaylistConfig::default());
    p.push_segment(b"i", b"m0", Duration::MAX);
    p.push_segment(b"i", b"m1", Duration::from_nanos(-5));
    p.push_segment(b"i", b"m2", Duration::MIN);
    let want = [
        header(4, 0, 0),
        map(0),
        entry("9223372036.855", 0),
        entry("0.000", 1),
        entry("0.000", 2),
    ]
    .concat();
    assert_eq!(p.render(), want);
}

#[test]
fn retention_by_duration_frees_unreferenced_inits() {
    let mut p = Playlist::new(config(1, 3000, 3000, 1 << 20));
    p.push_segment(b"aa", b"m0", ms(1000));
    p.push_segment(b"bbb", b"m1", ms(1000));
    p.push_segment(b"cccc", b"m2", ms(1000));
    p.push_segment(b"cccc", b"m3", ms(1000));
    assert_eq!(p.retained_segments(), 3);
    assert_eq!(p.segment(0), None);
    assert_eq!(p.init(0), None);
    assert_eq!(p.init(1).as_deref(), Some(&b"bbb"[..]));
    assert_eq!(p.retained_bytes(), 3 + 4 + 2 * 3);

    // A segment lasting the whole kept time keeps no other.
    p.push_segment(b"cccc", b"m4", ms(3000));
    assert_eq!(p.retained_segments(), 1);
    assert_eq!(p.init(1), None);
    assert_eq!(p.init(2).as_deref(), Some(&b"cccc"[..]));
    assert_eq!(p.retained_bytes(), 4 + 2);
}

#[test]
fn retention_is_bounded_by_count_whatever_the_durations() {
    let mut p = Playlist::new(PlaylistConfig::default());
    for i in 0..(MAX_SEGMENTS + 10) {
        p.push_segment(b"i", &[i as u8], Duration::ZERO);
    }
    assert_eq!(p.retained_segments(), MAX_SEGMENTS);
    assert_eq!(p.listed_segments(), MAX_SEGMENTS);
    assert_eq!(p.media_sequence(), 10);
}

#[test]
fn retention_by_bytes_drops_the_oldest_and_frees_inits() {
    let mut p = Playlist::new(config(1, 6000, 8000, 100));
    let a = [b'a'; 10];
    let b = [b'b'; 10];
    let media = [0u8; 30];
    for _ in 0..3 {
        p.push_segment(&a, &media, ms(1000));
    }
    assert_eq!(p.retained_bytes(), 100);
    assert_eq!(p.retained_segments(), 3);

    p.push_segment(&a, &media, ms(1000));
    assert_eq!(p.retained_bytes(), 100);
    assert_eq!(p.segment(0), None);
    assert!(p.segment(1).is_some());

    // 140 bytes: segments 1 and 2 go.
    p.push_segment(&b, &media, ms(1000));
    assert_eq!(p.retained_bytes(), 80);
    assert_eq!((p.media_sequence(), p.retained_segments()), (3, 2));
    assert!(p.init(0).is_some());

    // Segment 3 goes, and init 0 with it.
    p.push_segment(&b, &media, ms(1000));
    assert_eq!(p.retained_bytes(), 70);
    assert_eq!(p.init(0), None);
    assert_eq!(p.init(1).as_deref(), Some(&b[..]));
}

#[test]
fn oversized_segment_is_dropped_and_the_next_starts_a_new_init() {
    let mut p = Playlist::new(config(1, 6000, 8000, 50));
    p.push_segment(b"init", b"m0", ms(1000));
    p.push_segment(b"init", &[0; 100], ms(1000));
    assert_eq!((p.retained_segments(), p.retained_bytes()), (0, 0));
    assert_eq!((p.init(0), p.segment(0), p.segment(1)), (None, None, None));
    assert_eq!(p.render(), header(1, 2, 0));

    p.push_segment(b"init", b"m2", ms(1000));
    let want = [
        header(1, 2, 0),
        DISCONTINUITY.to_owned(),
        map(1),
        entry("1.000", 2),
    ]
    .concat();
    assert_eq!(p.render(), want);
}

#[test]
fn a_config_keeping_no_bytes_keeps_nothing() {
    let mut p = Playlist::new(config(1, 0, 0, 0));
    for i in 0..3u8 {
        p.push_segment(&[i], &[i], ms(1000));
    }
    assert_eq!((p.retained_segments(), p.retained_bytes()), (0, 0));
    assert_eq!(p.render(), header(1, 3, 2));
}

#[test]
fn numbers_start_where_asked() {
    let mut p = Playlist::starting_at(PlaylistConfig::default(), u64::MAX - 1);
    p.push_segment(b"a", b"m0", ms(1000));
    p.push_segment(b"b", b"m1", ms(1000));
    p.push_segment(b"c", b"m2", ms(1000));
    assert_eq!(p.segment(u64::MAX - 1).as_deref(), Some(&b"m0"[..]));
    assert_eq!(p.init(u64::MAX - 1).as_deref(), Some(&b"a"[..]));
    assert_eq!(p.media_sequence(), u64::MAX - 1);
    let _ = p.render();
}

/// A rendered playlist, checked against RFC 8216's structure.
#[derive(Debug)]
struct Parsed {
    target: u64,
    media_sequence: u64,
    discontinuity_sequence: u64,
    entries: Vec<Entry>,
}

#[derive(Debug)]
struct Entry {
    seq: u64,
    init: u64,
    millis: u64,
    discontinuity: bool,
    /// The segment's discontinuity sequence number.
    number: u64,
}

fn tag<'a>(line: Option<&'a str>, name: &str) -> &'a str {
    line.unwrap().strip_prefix(name).unwrap()
}

fn parse(text: &str) -> Parsed {
    assert!(text.starts_with(HEAD), "{text}");
    assert!(!text.contains("#EXT-X-INDEPENDENT-SEGMENTS"), "{text}");
    let mut lines = text.lines().skip(2);
    let target = tag(lines.next(), "#EXT-X-TARGETDURATION:").parse().unwrap();
    let media_sequence = tag(lines.next(), "#EXT-X-MEDIA-SEQUENCE:").parse().unwrap();
    let discontinuity_sequence = tag(lines.next(), "#EXT-X-DISCONTINUITY-SEQUENCE:")
        .parse()
        .unwrap();
    let mut entries = Vec::new();
    let (mut init, mut discontinuity, mut number) = (None, false, discontinuity_sequence);
    while let Some(line) = lines.next() {
        if line == "#EXT-X-DISCONTINUITY" {
            assert!(!discontinuity, "{text}");
            discontinuity = true;
            number += 1;
        } else if let Some(uri) = line.strip_prefix("#EXT-X-MAP:URI=\"init-") {
            let id = uri.strip_suffix(".mp4\"").unwrap().parse().unwrap();
            assert_ne!(init, Some(id), "{text}");
            init = Some(id);
        } else {
            let extinf = tag(Some(line), "#EXTINF:").strip_suffix(',').unwrap();
            let (secs, frac) = extinf.split_once('.').unwrap();
            assert_eq!(frac.len(), 3);
            let millis = secs.parse::<u64>().unwrap() * 1000 + frac.parse::<u64>().unwrap();
            let seq = lines
                .next()
                .unwrap()
                .strip_suffix(".m4s")
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(seq, media_sequence + entries.len() as u64, "{text}");
            assert!(millis.saturating_add(500) / 1000 <= target, "{text}");
            entries.push(Entry {
                seq,
                init: init.unwrap(),
                millis,
                discontinuity: std::mem::take(&mut discontinuity),
                number,
            });
        }
    }
    assert!(!discontinuity, "a trailing discontinuity: {text}");
    Parsed {
        target,
        media_sequence,
        discontinuity_sequence,
        entries,
    }
}

#[derive(Debug, Clone)]
enum Op {
    Push {
        init: u8,
        media_len: usize,
        millis: i64,
    },
    Discontinuity,
}

/// Pushes of segments that fit a 4 s target, and discontinuities.
fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (0u8..3, 0usize..64, -100i64..4_500).prop_map(|(init, media_len, millis)| Op::Push {
            init,
            media_len,
            millis,
        }),
        1 => Just(Op::Discontinuity),
    ]
}

/// What the test pushed as a segment.
struct Pushed {
    init_id: u64,
    init: Vec<u8>,
    media: Vec<u8>,
    millis: u64,
    discontinuity: bool,
}

/// The fewest of the newest of `durations` that add up to `want`, or all
/// of them.
fn newest_lasting(durations: &[u64], want: u64) -> usize {
    let mut sum = 0;
    let mut n = 0;
    for d in durations.iter().rev() {
        if sum >= want {
            break;
        }
        sum += d;
        n += 1;
    }
    n
}

fn proptest_config() -> Config {
    Config {
        cases: 512,
        rng_seed: RngSeed::Fixed(0x415_5eed),
        failure_persistence: None,
        ..Config::default()
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|b| format!("{b:02x}")).collect()
}

const PROFILE: [u8; 32] = [0xab; 32];
const TRANSFER: [u8; 32] = [0x0f; 32];

/// A stream path's directory: `/hls/<profile>/<transfer>/`.
fn dir(profile: &[u8], transfer: &[u8]) -> String {
    format!("/hls/{}/{}/", hex(profile), hex(transfer))
}

/// Routes holding [`PROFILE`] with a [`TRANSFER`] stream of two segments.
fn served() -> (HlsRoutes, HlsProfile, HlsStream) {
    let routes = HlsRoutes::new();
    let profile = routes.profile(&PROFILE).unwrap();
    let stream = profile
        .open_stream(Time::EPOCH, TRANSFER, PlaylistConfig::default())
        .unwrap();
    stream.push_segment(b"init", b"media 0", ms(2000));
    stream.push_segment(b"init", b"media 1", ms(2000));
    (routes, profile, stream)
}

proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn playlists_stay_bounded_and_consistent(
        target in 4u64..6,
        listed in 0i64..20_000,
        retained in 0i64..30_000,
        max_retained_bytes in 0usize..300,
        ops in proptest::collection::vec(op(), 0..60),
    ) {
        let config = config(target, listed, retained, max_retained_bytes);
        let mut p = Playlist::new(config);
        let mut pushed: Vec<Pushed> = Vec::new();
        let (mut next_init, mut pending) = (0u64, false);
        let (mut media_sequence, mut discontinuity_sequence) = (0, 0);
        let listed_millis = (listed as u64).max(3000 * target);
        let retained_millis = (retained as u64).max(listed_millis);
        let mut numbers = BTreeMap::new();
        for op in ops {
            match op {
                Op::Discontinuity => {
                    p.discontinuity();
                    pending = true;
                }
                Op::Push { init, media_len, millis } => {
                    let init = vec![init; usize::from(init) + 1];
                    let media = vec![pushed.len() as u8; media_len];
                    let kept = p.retained_segments() > 0;
                    let shared = kept && pushed.last().is_some_and(|l| l.init == init);
                    let init_id = if shared {
                        pushed.last().unwrap().init_id
                    } else {
                        next_init += 1;
                        next_init - 1
                    };
                    let discontinuity = pending || (!shared && !pushed.is_empty());
                    pending = false;
                    p.push_segment(&init, &media, ms(millis));
                    pushed.push(Pushed {
                        init_id,
                        init,
                        media,
                        millis: millis.max(0) as u64,
                        discontinuity,
                    });
                }
            }

            prop_assert!(p.retained_segments() <= MAX_SEGMENTS);
            prop_assert!(p.retained_bytes() <= max_retained_bytes);
            let n = p.retained_segments();
            let kept = &pushed[pushed.len() - n..];
            // No kept segment is kept beyond the kept media time.
            let millis: Vec<u64> = kept.iter().map(|s| s.millis).collect();
            prop_assert!(n <= newest_lasting(&millis, retained_millis));
            let mut inits = BTreeMap::new();
            for (i, s) in kept.iter().enumerate() {
                let seq = (pushed.len() - n + i) as u64;
                prop_assert_eq!(p.segment(seq).as_deref().map(<[u8]>::to_vec), Some(s.media.clone()));
                prop_assert_eq!(p.init(s.init_id).as_deref().map(<[u8]>::to_vec), Some(s.init.clone()));
                inits.insert(s.init_id, s.init.len());
            }
            let bytes = kept.iter().map(|s| s.media.len()).sum::<usize>()
                + inits.values().sum::<usize>();
            prop_assert_eq!(p.retained_bytes(), bytes);
            let first_kept = (pushed.len() - n) as u64;
            if first_kept > 0 {
                prop_assert_eq!(p.segment(first_kept - 1), None);
            }
            prop_assert_eq!(p.segment(pushed.len() as u64), None);
            let oldest_init = kept.first().map_or(next_init, |s| s.init_id);
            for id in 0..oldest_init {
                prop_assert_eq!(p.init(id), None);
            }

            let parsed = parse(&p.render());
            prop_assert_eq!(parsed.target, target);
            prop_assert_eq!(parsed.entries.len(), newest_lasting(&millis, listed_millis));
            prop_assert!(parsed.media_sequence >= media_sequence);
            prop_assert!(parsed.discontinuity_sequence >= discontinuity_sequence);
            (media_sequence, discontinuity_sequence) =
                (parsed.media_sequence, parsed.discontinuity_sequence);
            prop_assert_eq!(
                parsed.media_sequence + parsed.entries.len() as u64,
                pushed.len() as u64
            );
            for e in &parsed.entries {
                let s = &pushed[e.seq as usize];
                prop_assert_eq!(e.init, s.init_id);
                prop_assert_eq!(e.millis, s.millis);
                prop_assert_eq!(e.discontinuity, s.discontinuity);
                // A segment's discontinuity number never changes.
                prop_assert_eq!(*numbers.entry(e.seq).or_insert(e.number), e.number);
            }
        }
    }

    #[test]
    fn arbitrary_paths_never_panic(path in ".{0,300}") {
        let (routes, _profile, _stream) = served();
        let r = routes.get(Time::EPOCH, &path);
        prop_assert!([200, 400, 404].contains(&r.status));
        if path.len() > MAX_PATH_LEN {
            prop_assert_eq!(r.status, 400);
        }
    }

    #[test]
    fn route_shaped_paths_never_panic(
        prefix in prop_oneof![Just("/hls/"), Just("/hls"), Just("/"), Just("")],
        profile in prop_oneof![Just(hex(&PROFILE)), Just(hex(&PROFILE).to_uppercase()), "[0-9a-fA-Fg]{0,70}"],
        transfer in prop_oneof![Just(hex(&TRANSFER)), "[0-9a-fA-Fg]{0,70}"],
        file in prop_oneof![
            Just("index.m3u8".to_owned()),
            "init-[0-9a-z+-]{0,22}\\.mp4",
            "[0-9+-]{0,22}\\.m4s",
            "[ -~]{0,12}",
        ],
        query in "(\\?[ -~]{0,20})?",
    ) {
        let (routes, _profile, _stream) = served();
        let path = format!("{prefix}{profile}/{transfer}/{file}{query}");
        let r = routes.get(Time::EPOCH, &path);
        prop_assert!([200, 400, 404].contains(&r.status));
        if r.status == 200 {
            prop_assert!(
                ["application/vnd.apple.mpegurl", "video/mp4", "video/iso.segment"]
                    .contains(&r.content_type)
            );
        }
    }
}

#[test]
fn playlist_path_is_lowercase_hex() {
    let want = format!("/hls/{}/{}/index.m3u8", "ab".repeat(32), "0f".repeat(32));
    assert_eq!(playlist_path(&PROFILE, &TRANSFER), want);
    let (_routes, _profile, stream) = served();
    assert_eq!(stream.path(), want);
    assert_eq!(stream.transfer_id(), &TRANSFER);
}

#[test]
fn serves_the_playlist_its_inits_and_segments() {
    let (routes, profile, stream) = served();
    assert_eq!(profile.key(), &PROFILE);
    let base = dir(&PROFILE, &TRANSFER);

    let r = routes.get(at(1), stream.path());
    assert_eq!(
        (r.status, r.content_type, r.cache_control),
        (200, "application/vnd.apple.mpegurl", "no-cache")
    );
    let want = [
        header(4, 0, 0),
        map(0),
        entry("2.000", 0),
        entry("2.000", 1),
    ]
    .concat();
    assert_eq!(std::str::from_utf8(&r.body).unwrap(), want);

    let r = routes.get(at(2), &format!("{base}init-0.mp4"));
    assert_eq!(
        (r.status, r.content_type, r.cache_control),
        (200, "video/mp4", "max-age=31536000, immutable")
    );
    assert_eq!(&*r.body, b"init");

    let r = routes.get(at(3), &format!("{base}1.m4s"));
    assert_eq!(
        (r.status, r.content_type, r.cache_control),
        (200, "video/iso.segment", "max-age=31536000, immutable")
    );
    assert_eq!(&*r.body, b"media 1");
    assert_eq!(stream.retained_bytes(), 4 + 2 * 7);
}

#[test]
fn routes_take_either_hex_case_and_ignore_the_query() {
    let (routes, _profile, stream) = served();
    let upper = format!(
        "/hls/{}/{}/0.m4s",
        hex(&PROFILE).to_uppercase(),
        hex(&TRANSFER).to_uppercase()
    );
    assert_eq!(routes.get(Time::EPOCH, &upper).status, 200);
    let query = format!("{}?_HLS_msn=3", stream.path());
    assert_eq!(routes.get(Time::EPOCH, &query).status, 200);

    // The length bound covers the query.
    let mut long = format!("{}?", stream.path());
    long.push_str(&"x".repeat(MAX_PATH_LEN - long.len()));
    assert_eq!(routes.get(Time::EPOCH, &long).status, 200);
    long.push('x');
    assert_eq!(routes.get(Time::EPOCH, &long).status, 400);
}

#[test]
fn malformed_paths_are_bad_requests() {
    let (routes, _profile, _stream) = served();
    let (p, t) = (hex(&PROFILE), hex(&TRANSFER));
    let bad_hex = format!("{}g", &p[..63]);
    let paths = [
        format!("/hls/{p}/{t}/{}.m4s", "0".repeat(200)),
        format!("/hls/{bad_hex}/{t}/index.m3u8"),
        format!("/hls/{p}/{bad_hex}/index.m3u8"),
        format!("/hls/{}/{t}/index.m3u8", &p[..62]),
        format!("/hls/{}/{t}/index.m3u8", &p[..63]),
        format!("/hls/{p}ab/{t}/index.m3u8"),
        format!("/hls/{p}/{}/index.m3u8", &t[..62]),
        format!("/hls/{p}/{t}0f/index.m3u8"),
        format!("/hls/{p}/{t}/x/index.m3u8"),
        format!("/hls/{p}/{t}/index.m3u8/"),
        format!("/hls/{p}/index.m3u8"),
        format!("/hls/{p}"),
        "/hls/".to_owned(),
        format!("/hls//{t}/index.m3u8"),
        format!("/hls/{p}//index.m3u8"),
        format!("/hls/{p}/{t}/"),
        format!("/hls/{p}/{t}/abc.m4s"),
        format!("/hls/{p}/{t}/-1.m4s"),
        format!("/hls/{p}/{t}/+1.m4s"),
        format!("/hls/{p}/{t}/.m4s"),
        format!("/hls/{p}/{t}/1.5.m4s"),
        format!("/hls/{p}/{t}/ 1.m4s"),
        format!("/hls/{p}/{t}/18446744073709551616.m4s"),
        format!("/hls/{p}/{t}/init-.mp4"),
        format!("/hls/{p}/{t}/init-x.mp4"),
        format!("/hls/{p}/{t}/init--1.mp4"),
        format!("/hls/{p}/{t}/init-18446744073709551616.mp4"),
    ];
    for path in paths {
        let r = routes.get(Time::EPOCH, &path);
        assert_eq!(
            (r.status, r.content_type, r.cache_control),
            (400, "text/plain; charset=utf-8", "no-store"),
            "{path}"
        );
        assert_eq!(&*r.body, b"bad request\n");
    }
}

#[test]
fn missing_resources_are_not_found() {
    let routes = HlsRoutes::new();
    let profile = routes.profile(&PROFILE).unwrap();
    let stream = profile
        .open_stream(Time::EPOCH, TRANSFER, config(1, 3000, 3000, 1 << 20))
        .unwrap();
    stream.push_segment(b"a", b"m0", ms(1000));
    // It lasts the kept time: the segment before it expires.
    stream.push_segment(b"b", b"m1", ms(3000));
    let (p, t) = (hex(&PROFILE), hex(&TRANSFER));
    let base = dir(&PROFILE, &TRANSFER);
    let paths = [
        String::new(),
        "/".to_owned(),
        "/favicon.ico".to_owned(),
        "/hls".to_owned(),
        format!("/HLS/{p}/{t}/index.m3u8"),
        format!("hls/{p}/{t}/index.m3u8"),
        format!("{base}init.mp4"),
        format!("{base}index.m3u"),
        format!("{base}0.mp4"),
        format!("{base}foo"),
        format!("{}index.m3u8", dir(&[0xac; 32], &TRANSFER)),
        format!("{}index.m3u8", dir(&PROFILE, &[0x10; 32])),
        format!("{}0.m4s", dir(&TRANSFER, &PROFILE)),
        // An expired segment, its init, and a segment yet to come.
        format!("{base}0.m4s"),
        format!("{base}init-0.mp4"),
        format!("{base}2.m4s"),
        format!("{base}init-2.mp4"),
    ];
    for path in paths {
        let r = routes.get(Time::EPOCH, &path);
        assert_eq!(
            (r.status, r.content_type, r.cache_control),
            (404, "text/plain; charset=utf-8", "no-store"),
            "{path}"
        );
        assert_eq!(&*r.body, b"not found\n");
    }
    assert_eq!(routes.get(Time::EPOCH, &format!("{base}1.m4s")).status, 200);
    assert_eq!(
        routes.get(Time::EPOCH, &format!("{base}init-1.mp4")).status,
        200
    );
}

#[test]
fn successful_gets_record_the_last_access() {
    let routes = HlsRoutes::new();
    let profile = routes.profile(&PROFILE).unwrap();
    let stream = profile
        .open_stream(at(10), TRANSFER, PlaylistConfig::default())
        .unwrap();
    let other = profile
        .open_stream(at(10), [0x10; 32], PlaylistConfig::default())
        .unwrap();
    let base = dir(&PROFILE, &TRANSFER);
    assert_eq!(stream.last_access(), at(10));

    assert_eq!(routes.get(at(20), stream.path()).status, 200);
    assert_eq!(stream.last_access(), at(20));

    // Before any segment, nothing but the playlist is found.
    assert_eq!(routes.get(at(30), &format!("{base}10.m4s")).status, 404);
    assert_eq!(routes.get(at(30), &format!("{base}abc.m4s")).status, 400);
    assert_eq!(stream.last_access(), at(20));

    stream.push_segment(b"init", b"m", ms(1000));
    assert_eq!(
        routes.get(at(40), &format!("{base}init-10.mp4")).status,
        200
    );
    assert_eq!(stream.last_access(), at(40));
    assert_eq!(routes.get(at(50), &format!("{base}10.m4s")).status, 200);
    assert_eq!(stream.last_access(), at(50));

    // A late request doesn't move it back.
    assert_eq!(routes.get(at(45), stream.path()).status, 200);
    assert_eq!(stream.last_access(), at(50));

    assert_eq!(routes.get(at(60), other.path()).status, 200);
    assert_eq!(
        (stream.last_access(), other.last_access()),
        (at(50), at(60))
    );
}

#[test]
fn numbers_start_at_the_open_time() {
    let opened = 1_700_000_000_123;
    let routes = HlsRoutes::new();
    let profile = routes.profile(&PROFILE).unwrap();
    let stream = profile
        .open_stream(at(opened), TRANSFER, PlaylistConfig::default())
        .unwrap();
    for _ in 0..3 {
        stream.push_segment(b"init", b"m", ms(1000));
    }
    let base = dir(&PROFILE, &TRANSFER);
    let playlist = routes.get(at(opened), stream.path()).body;
    let playlist = std::str::from_utf8(&playlist).unwrap();
    assert!(playlist.contains("#EXT-X-MEDIA-SEQUENCE:1700000000123\n"));
    assert!(playlist.contains("init-1700000000123.mp4"));
    assert!(playlist.ends_with("1700000000125.m4s\n"));
    let r = routes.get(at(opened), &format!("{base}1700000000124.m4s"));
    assert_eq!(r.status, 200);
    drop(stream);

    // Reopened at the same path, the stream starts past the old numbers.
    let stream = profile
        .open_stream(at(opened + 5000), TRANSFER, PlaylistConfig::default())
        .unwrap();
    stream.push_segment(b"init", b"m", ms(1000));
    let parsed = parse(std::str::from_utf8(&routes.get(at(opened), stream.path()).body).unwrap());
    assert_eq!(parsed.media_sequence, 1_700_000_005_123);
    assert_eq!(parsed.entries[0].init, 1_700_000_005_123);
}

#[test]
fn dropping_a_stream_unregisters_it() {
    let (routes, profile, stream) = served();
    let path = stream.path().to_owned();
    assert_eq!(routes.get(Time::EPOCH, &path).status, 200);
    drop(stream);
    assert_eq!(routes.get(Time::EPOCH, &path).status, 404);
    assert_eq!(profile.streams(), 0);
}

#[test]
fn dropping_a_profile_unregisters_its_streams() {
    let (routes, profile, stream) = served();
    drop(profile);
    assert_eq!(routes.profiles(), 0);
    assert_eq!(routes.get(Time::EPOCH, stream.path()).status, 404);
    stream.push_segment(b"init", b"m", ms(1000));
    stream.discontinuity();
    drop(stream);
}

#[test]
fn reopening_a_stream_replaces_it_until_the_new_handle_drops() {
    let (routes, profile, first) = served();
    let second = profile
        .open_stream(Time::EPOCH, TRANSFER, PlaylistConfig::default())
        .unwrap();
    second.push_segment(b"init", b"second", ms(1000));
    let segment = format!("{}0.m4s", dir(&PROFILE, &TRANSFER));
    assert_eq!(&*routes.get(Time::EPOCH, &segment).body, b"second");
    drop(first);
    assert_eq!(profile.streams(), 1);
    assert_eq!(&*routes.get(Time::EPOCH, &segment).body, b"second");
    drop(second);
    assert_eq!(routes.get(Time::EPOCH, &segment).status, 404);
}

#[test]
fn reregistering_a_profile_replaces_it_until_the_new_handle_drops() {
    let (routes, first, first_stream) = served();
    let second = routes.profile(&PROFILE).unwrap();
    assert_eq!(routes.get(Time::EPOCH, first_stream.path()).status, 404);
    let stream = second
        .open_stream(Time::EPOCH, TRANSFER, PlaylistConfig::default())
        .unwrap();
    drop(first);
    drop(first_stream);
    assert_eq!(routes.profiles(), 1);
    assert_eq!(routes.get(Time::EPOCH, stream.path()).status, 200);
    drop(second);
    assert_eq!(routes.get(Time::EPOCH, stream.path()).status, 404);
}

#[test]
fn profile_keys_are_32_bytes() {
    let routes = HlsRoutes::new();
    assert_eq!(routes.profile(&[1; 31]).unwrap_err(), RouteError::KeyLength);
    assert_eq!(routes.profile(&[1; 33]).unwrap_err(), RouteError::KeyLength);
    assert_eq!(routes.profile(&[]).unwrap_err(), RouteError::KeyLength);
    assert_eq!(routes.profiles(), 0);
}

#[test]
fn profiles_are_capped() {
    let routes = HlsRoutes::new();
    let mut profiles: Vec<_> = (0..MAX_PROFILES)
        .map(|i| routes.profile(&[i as u8; 32]).unwrap())
        .collect();
    assert_eq!(
        routes.profile(&[0xff; 32]).unwrap_err(),
        RouteError::TooManyProfiles
    );
    // Replacing a registered key doesn't add one.
    profiles.push(routes.profile(&[0; 32]).unwrap());
    assert_eq!(routes.profiles(), MAX_PROFILES);
    drop(profiles.remove(1));
    let _last = routes.profile(&[0xff; 32]).unwrap();
}

#[test]
fn streams_are_capped() {
    let routes = HlsRoutes::new();
    let profile = routes.profile(&PROFILE).unwrap();
    let open = |id: u8| profile.open_stream(Time::EPOCH, [id; 32], PlaylistConfig::default());
    let mut streams: Vec<_> = (0..MAX_STREAMS).map(|i| open(i as u8).unwrap()).collect();
    assert_eq!(open(0xff).unwrap_err(), RouteError::TooManyStreams);
    streams.push(open(0).unwrap());
    assert_eq!(profile.streams(), MAX_STREAMS);
    drop(streams.remove(1));
    let _last = open(0xff).unwrap();
}

#[test]
fn handles_outlive_the_routes() {
    let (routes, profile, stream) = served();
    drop(routes);
    let other = profile
        .open_stream(Time::EPOCH, [1; 32], PlaylistConfig::default())
        .unwrap();
    drop(profile);
    stream.push_segment(b"init", b"m", ms(1000));
    drop(stream);
    drop(other);
}

/// A muxer's segment of `media` lasting `millis`, with `vtt` and
/// `captions`.
fn muxed(media: &[u8], millis: i64, vtt: Option<&str>, captions: bool) -> HlsSegment {
    HlsSegment {
        init: Rc::from(&b"init"[..]),
        media: media.to_vec(),
        duration: ms(millis),
        keyframe: true,
        discontinuity: false,
        vtt: vtt.map(|v| v.as_bytes().to_vec()),
        codecs: Rc::from("avc1.64001f,mp4a.40.2"),
        resolution: Some((1280, 720)),
        captions,
        published: Time::from_unix_millis(1_793_275_200_000),
    }
}

#[test]
fn the_multivariant_playlist_declares_the_variant_its_segments_describe() {
    let mut p = Playlist::new(PlaylistConfig::default());
    assert_eq!(
        p.render_multivariant(),
        "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-STREAM-INF:BANDWIDTH=1000000\nindex.m3u8\n"
    );
    // 250 KB over 2 s and 100 KB over 1 s: the first's 1 Mbit/s is the
    // peak.
    p.push(&muxed(&[0; 250_000], 2000, None, false));
    p.push(&muxed(&[0; 100_000], 1000, None, false));
    assert_eq!(
        p.render_multivariant(),
        "#EXTM3U\n#EXT-X-VERSION:7\n\
         #EXT-X-STREAM-INF:BANDWIDTH=1000000,CODECS=\"avc1.64001f,mp4a.40.2\",RESOLUTION=1280x720\n\
         index.m3u8\n"
    );
    // Subtitles and captions, once seen, stay declared.
    p.push(&muxed(b"m", 1000, Some("WEBVTT\n"), true));
    p.push(&muxed(b"m", 1000, Some("WEBVTT\n"), false));
    let want = "#EXTM3U\n#EXT-X-VERSION:7\n\
         #EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"subs\",NAME=\"Subtitles\",DEFAULT=NO,AUTOSELECT=YES,FORCED=NO,URI=\"subtitles.m3u8\"\n\
         #EXT-X-MEDIA:TYPE=CLOSED-CAPTIONS,GROUP-ID=\"cc\",NAME=\"Captions\",DEFAULT=NO,AUTOSELECT=YES,INSTREAM-ID=\"CC1\"\n\
         #EXT-X-STREAM-INF:BANDWIDTH=1000000,CODECS=\"avc1.64001f,mp4a.40.2\",RESOLUTION=1280x720,SUBTITLES=\"subs\",CLOSED-CAPTIONS=\"cc\"\n\
         index.m3u8\n";
    assert_eq!(p.render_multivariant(), want);
}

#[test]
fn the_subtitles_playlist_lists_each_segments_webvtt() {
    let mut p = Playlist::new(config(4, 0, 0, 1 << 20));
    p.push_segment(b"init", b"before", ms(2000));
    p.push(&muxed(b"m", 2000, Some("WEBVTT\ncue"), false));
    p.discontinuity();
    p.push(&muxed(b"m", 1500, Some("WEBVTT\n"), false));
    p.end();
    let want = [
        header(4, 0, 0),
        "#EXTINF:2.000,\n0.vtt\n".to_owned(),
        // The muxer's segments are dated; the third's publish time comes
        // before the second's end, so it starts at that end.
        "#EXT-X-PROGRAM-DATE-TIME:2026-10-29T12:00:00.000Z\n".to_owned(),
        "#EXTINF:2.000,\n1.vtt\n".to_owned(),
        "#EXT-X-DISCONTINUITY\n".to_owned(),
        "#EXT-X-PROGRAM-DATE-TIME:2026-10-29T12:00:02.000Z\n".to_owned(),
        "#EXTINF:1.500,\n2.vtt\n".to_owned(),
        "#EXT-X-ENDLIST\n".to_owned(),
    ]
    .concat();
    assert_eq!(p.render_subtitles(), want);
    assert_eq!(&*p.vtt(1).unwrap(), b"WEBVTT\ncue");
    // A segment from before the text track has none.
    assert_eq!(
        &*p.vtt(0).unwrap(),
        b"WEBVTT\nX-TIMESTAMP-MAP=MPEGTS:0,LOCAL:00:00:00.000\n\n"
    );
    assert_eq!(p.vtt(3), None);
    assert_eq!(p.retained_bytes(), 4 + 6 + 1 + 10 + 1 + 7);
}

#[test]
fn serves_the_multivariant_and_subtitles_playlists_and_webvtt() {
    let (routes, _profile, stream) = served();
    stream.push(&muxed(b"m", 2000, Some("WEBVTT\nX"), false));
    let base = dir(&PROFILE, &TRANSFER);
    assert_eq!(
        multivariant_path(&PROFILE, &TRANSFER),
        format!("{base}multivariant.m3u8")
    );
    for (file, kind) in [
        ("multivariant.m3u8", "application/vnd.apple.mpegurl"),
        ("subtitles.m3u8", "application/vnd.apple.mpegurl"),
    ] {
        let r = routes.get(at(1), &format!("{base}{file}"));
        assert_eq!(
            (r.status, r.content_type, r.cache_control),
            (200, kind, "no-cache")
        );
    }
    let r = routes.get(at(1), &format!("{base}2.vtt"));
    assert_eq!(
        (r.status, r.content_type, r.cache_control),
        (200, "text/vtt", "max-age=31536000, immutable")
    );
    assert_eq!(&*r.body, b"WEBVTT\nX");
    assert_eq!(routes.get(at(1), &format!("{base}9.vtt")).status, 404);
    assert_eq!(routes.get(at(1), &format!("{base}x.vtt")).status, 400);
    assert_eq!(routes.get(at(1), &format!("{base}.vtt")).status, 400);
    assert_eq!(stream.listed_segments(), 3);
}
