// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Low-latency playlists: parts, their window, the preload hint, dates
//! and rendition reports, and RFC 8216bis's rules over a muxed stream.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;
use crate::units::test_support::{audio, unit, video};

const VIDEO: u32 = 1;
const AUDIO: u32 = 2;
/// 90 kHz ticks of a frame at 30 frames a second.
const FRAME: i64 = 3000;
/// 2026-10-29T12:00:00Z.
const NOON: i64 = 1_793_275_200_000;

fn ms(n: i64) -> Duration {
    Duration::from_millis(n)
}

/// A playlist of 4 s targets and 1 s parts, numbering from 100.
fn playlist() -> Playlist {
    Playlist::starting_at(
        PlaylistConfig {
            part_target: ms(1000),
            ..PlaylistConfig::default()
        },
        100,
    )
}

/// Part `index` of a segment of init `init` published at `published`.
fn part(index: u32, millis: i64, independent: bool, media: &[u8], published: i64) -> HlsPart {
    HlsPart {
        media: Rc::from(media),
        duration: ms(millis),
        independent,
        index,
        init: Rc::from(&b"init"[..]),
        discontinuity: false,
        published: Time::from_unix_millis(published),
        vtt: None,
    }
}

fn segment(media: &[u8], millis: i64, published: i64, vtt: Option<&str>) -> HlsSegment {
    HlsSegment {
        init: Rc::from(&b"init"[..]),
        media: media.to_vec(),
        duration: ms(millis),
        keyframe: true,
        discontinuity: false,
        vtt: vtt.map(|v| v.as_bytes().to_vec()),
        codecs: Rc::from("avc1.64001f,mp4a.40.2"),
        resolution: Some((1280, 720)),
        captions: false,
        published: Time::from_unix_millis(published),
    }
}

const LL_HEAD: &str = "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:4\n\
    #EXT-X-SERVER-CONTROL:CAN-BLOCK-RELOAD=YES,PART-HOLD-BACK=3.000\n\
    #EXT-X-PART-INF:PART-TARGET=1.000\n";

#[test]
fn a_segments_parts_are_listed_while_it_is_open_and_after() {
    let mut p = playlist();
    p.push_part(&part(0, 1000, true, b"aa", NOON));
    let want = [
        LL_HEAD,
        "#EXT-X-MEDIA-SEQUENCE:100\n#EXT-X-DISCONTINUITY-SEQUENCE:0\n",
        "#EXT-X-MAP:URI=\"init-100.mp4\"\n",
        "#EXT-X-PROGRAM-DATE-TIME:2026-10-29T12:00:00.000Z\n",
        "#EXT-X-PART:DURATION=1.000,URI=\"part-100.m4s\",INDEPENDENT=YES\n",
        "#EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"part-101.m4s\"\n",
    ]
    .concat();
    assert_eq!(p.render(), want);
    p.push_part(&part(1, 999, false, b"bbb", NOON));
    p.push(&segment(b"aabbb", 1999, NOON, None));
    p.push_part(&part(0, 500, true, b"c", NOON + 2000));
    let want = [
        LL_HEAD,
        "#EXT-X-MEDIA-SEQUENCE:100\n#EXT-X-DISCONTINUITY-SEQUENCE:0\n",
        "#EXT-X-MAP:URI=\"init-100.mp4\"\n",
        "#EXT-X-PROGRAM-DATE-TIME:2026-10-29T12:00:00.000Z\n",
        "#EXT-X-PART:DURATION=1.000,URI=\"part-100.m4s\",INDEPENDENT=YES\n",
        "#EXT-X-PART:DURATION=0.999,URI=\"part-101.m4s\"\n",
        "#EXTINF:1.999,\n100.m4s\n",
        "#EXT-X-PROGRAM-DATE-TIME:2026-10-29T12:00:02.000Z\n",
        "#EXT-X-PART:DURATION=0.500,URI=\"part-102.m4s\",INDEPENDENT=YES\n",
        "#EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"part-103.m4s\"\n",
    ]
    .concat();
    assert_eq!(p.render(), want);
    // The kept segment's parts are ranges of its media.
    assert_eq!(&*p.part(100).unwrap(), b"aa");
    assert_eq!(&*p.part(101).unwrap(), b"bbb");
    assert_eq!(&*p.part(102).unwrap(), b"c");
    assert_eq!(p.part(103), None);
    assert_eq!(p.part(99), None);
    assert_eq!(&*p.segment(100).unwrap(), b"aabbb");
    assert_eq!(p.retained_bytes(), 4 + 5 + 1);
    assert_eq!(p.next_part(), 103);
    // The subtitles' playlist lists the same parts as WebVTT files, empty
    // for a stream without a text track.
    let subtitles = p.render_subtitles();
    assert!(
        subtitles.contains("#EXT-X-PART-INF:PART-TARGET=1.000\n"),
        "{subtitles}"
    );
    assert!(
        subtitles
            .contains("#EXT-X-PART:DURATION=0.999,URI=\"part-101.vtt\"\n#EXTINF:1.999,\n100.vtt\n"),
        "{subtitles}"
    );
    assert!(subtitles.ends_with("#EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"part-103.vtt\"\n"));
    assert!(!subtitles.contains("MAP"));
    assert_eq!(&*p.part_vtt(101).unwrap(), EMPTY_VTT.as_bytes());
    assert_eq!(p.part_vtt(103), None);
}

#[test]
fn parts_durations_add_up_to_their_segments_in_milliseconds() {
    let mut p = playlist();
    // Three parts of 333.4 ms each: listed as 333, 334 and 333.
    let third = Duration::from_micros(333_400);
    for i in 0..3 {
        let mut x = part(i, 0, i == 0, b"x", NOON);
        x.duration = third;
        p.push_part(&x);
    }
    let mut s = segment(b"xxx", 0, NOON, None);
    s.duration = third + third + third;
    p.push(&s);
    let text = p.render();
    let durations: Vec<&str> = text
        .lines()
        .filter_map(|l| l.strip_prefix("#EXT-X-PART:DURATION="))
        .map(|l| l.split(',').next().unwrap())
        .collect();
    assert_eq!(durations, ["0.333", "0.334", "0.333"]);
    assert!(text.contains("#EXTINF:1.000,\n100.m4s\n"), "{text}");
}

#[test]
fn parts_leave_the_listing_three_targets_from_its_end_and_stay_fetchable() {
    let mut p = playlist();
    // Ten 4 s segments of four 1 s parts.
    for g in 0..10i64 {
        for i in 0..4 {
            p.push_part(&part(i, 1000, i == 0, &[g as u8, i as u8], NOON + g * 4000));
        }
        let media: Vec<u8> = (0..4).flat_map(|i| [g as u8, i]).collect();
        p.push(&segment(&media, 4000, NOON + g * 4000, None));
    }
    let text = p.render();
    // 16 s listed: segments 106 to 109, the last three of them with
    // their parts.
    assert!(text.contains("#EXT-X-MEDIA-SEQUENCE:106\n"), "{text}");
    let parts: Vec<u64> = text
        .lines()
        .filter_map(|l| l.split("URI=\"part-").nth(1))
        .map(|l| l.split('.').next().unwrap().parse().unwrap())
        .collect();
    assert_eq!(parts, (128..141).collect::<Vec<u64>>());
    assert!(text.contains("#EXTINF:4.000,\n106.m4s\n#EXT-X-PROGRAM-DATE-TIME"));
    // A delisted part of a kept segment is still served.
    assert_eq!(&*p.part(100 + 4 * 7).unwrap(), &[7, 0]);
    assert_eq!(&*p.part(100 + 4 * 2 + 3).unwrap(), &[2, 3]);
}

#[test]
fn a_segment_its_muxer_left_open_is_finished_from_its_parts() {
    let mut p = playlist();
    p.push_part(&part(0, 1000, true, b"ab", NOON));
    p.push_part(&part(1, 700, false, b"c", NOON));
    // Another muxer's first part: the open segment is kept as its parts.
    p.push_part(&part(0, 1000, true, b"d", NOON + 1700));
    assert_eq!(&*p.segment(100).unwrap(), b"abc");
    assert!(p.render().contains("#EXTINF:1.700,\n100.m4s\n"));
    // Out of turn, a part is dropped.
    p.push_part(&part(2, 1000, false, b"e", NOON + 1700));
    assert_eq!(p.part(103), None);
    // The end finishes the open segment, and no part is hinted.
    p.end();
    assert_eq!(&*p.segment(101).unwrap(), b"d");
    let text = p.render();
    assert!(
        text.ends_with("#EXTINF:1.000,\n101.m4s\n#EXT-X-ENDLIST\n"),
        "{text}"
    );
    assert!(!text.contains("PRELOAD-HINT"));
    // A part after the end opens a segment after a discontinuity.
    p.push_part(&part(0, 1000, true, b"f", NOON + 9000));
    let text = p.render();
    assert!(!text.contains("ENDLIST"));
    assert!(text.contains("#EXT-X-DISCONTINUITY\n#EXT-X-PROGRAM-DATE-TIME"));
}

#[test]
fn parts_that_dont_make_up_their_segment_go() {
    let mut p = playlist();
    p.push_part(&part(0, 1000, true, b"ab", NOON));
    p.push(&segment(b"abc", 1500, NOON, None));
    let text = p.render();
    assert!(!text.contains("#EXT-X-PART:"), "{text}");
    assert_eq!(p.part(100), None);
    assert_eq!(p.retained_bytes(), 4 + 3);
}

#[test]
fn a_discontinuity_comes_before_its_segments_first_part() {
    let mut p = playlist();
    p.push_part(&part(0, 1000, true, b"a", NOON));
    p.push(&segment(b"a", 1000, NOON, None));
    let mut first = part(0, 1000, true, b"b", NOON + 5000);
    first.discontinuity = true;
    p.push_part(&first);
    p.push_part(&part(1, 1000, false, b"c", NOON + 5000));
    let mut s = segment(b"bc", 2000, NOON + 5000, None);
    s.discontinuity = true;
    p.push(&s);
    p.push_part(&part(0, 1000, true, b"d", NOON + 7000));
    let text = p.render();
    assert_eq!(text.matches("#EXT-X-DISCONTINUITY\n").count(), 1, "{text}");
    assert!(
        text.contains(
            "#EXT-X-DISCONTINUITY\n#EXT-X-PROGRAM-DATE-TIME:2026-10-29T12:00:05.000Z\n#EXT-X-PART"
        ),
        "{text}"
    );
}

#[test]
fn the_media_and_subtitles_playlists_report_each_other() {
    let mut p = playlist();
    p.push_part(&part(0, 1000, true, b"a", NOON));
    p.push(&segment(b"a", 1000, NOON, Some("WEBVTT\n")));
    p.push_part(&part(0, 1000, true, b"b", NOON + 1000));
    p.push_part(&part(1, 1000, false, b"c", NOON + 1000));
    // Both list the same parts: each reports the other's newest.
    let media = p.render();
    assert!(
        media
            .ends_with("#EXT-X-RENDITION-REPORT:URI=\"subtitles.m3u8\",LAST-MSN=101,LAST-PART=1\n"),
        "{media}"
    );
    let subtitles = p.render_subtitles();
    assert!(
        subtitles.ends_with(
            "#EXT-X-PART:DURATION=1.000,URI=\"part-102.vtt\"\n\
             #EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"part-103.vtt\"\n\
             #EXT-X-RENDITION-REPORT:URI=\"index.m3u8\",LAST-MSN=101,LAST-PART=1\n"
        ),
        "{subtitles}"
    );
    assert!(subtitles.contains("#EXT-X-PROGRAM-DATE-TIME:2026-10-29T12:00:00.000Z\n"));
}

#[test]
fn dates_are_utc_to_the_millisecond_and_never_go_back_a_segment() {
    let date = |ms: i64| {
        let mut s = String::new();
        push_date(&mut s, Time::from_unix_millis(ms));
        s
    };
    assert_eq!(date(0), "1970-01-01T00:00:00.000Z");
    assert_eq!(date(951_782_400_000), "2000-02-29T00:00:00.000Z");
    assert_eq!(date(951_868_799_999), "2000-02-29T23:59:59.999Z");
    assert_eq!(date(1_793_449_496_789), "2026-10-31T12:24:56.789Z");
    assert_eq!(date(4_107_542_400_000), "2100-03-01T00:00:00.000Z");
    // The latest time there is.
    assert_eq!(date(i64::MAX), "2262-04-11T23:47:16.854Z");
    // Every day of 1970 to 2401 follows the one before.
    let mut prev = civil_from_days(-1);
    for d in 0..days_from_1970_to_2401() {
        let next = civil_from_days(d);
        let follows = if prev.2 + 1 == next.2 {
            (prev.0, prev.1) == (next.0, next.1)
        } else if next.2 == 1 && prev.1 + 1 == next.1 {
            prev.0 == next.0
        } else {
            (next.1, next.2, prev.1, prev.2) == (1, 1, 12, 31) && prev.0 + 1 == next.0
        };
        assert!(follows, "{prev:?} {next:?}");
        prev = next;
    }

    let mut p = playlist();
    // A clock that steps back 2 s dates the next segment at the end of the
    // one before; one that steps back 0.4 s, or forward, keeps its date.
    for (i, published) in [NOON, NOON, NOON + 3600, NOON + 9000].iter().enumerate() {
        p.push_part(&part(0, 2000, true, &[i as u8], *published));
        p.push(&segment(&[i as u8], 2000, *published, None));
    }
    let mut q = playlist();
    q.push(&segment(b"a", 2000, NOON, None));
    q.push(&segment(b"b", 2000, NOON - 1000, None));
    q.push(&segment(b"c", 2000, -5, None));
    let dates = |p: &Playlist| -> Vec<String> {
        p.render()
            .lines()
            .filter_map(|l| l.strip_prefix("#EXT-X-PROGRAM-DATE-TIME:"))
            .map(str::to_owned)
            .collect()
    };
    assert_eq!(
        dates(&p),
        [
            "2026-10-29T12:00:00.000Z",
            "2026-10-29T12:00:02.000Z",
            "2026-10-29T12:00:03.600Z",
            "2026-10-29T12:00:09.000Z"
        ]
    );
    assert_eq!(
        dates(&q),
        ["2026-10-29T12:00:00.000Z", "2026-10-29T12:00:02.000Z"]
    );
}

/// The days from 1970 to 2401.
fn days_from_1970_to_2401() -> i64 {
    (2401 - 1970) * 365 + 105
}

#[test]
fn part_files_are_routed_by_number() {
    let routes = HlsRoutes::new();
    let profile = routes.profile(&[1; 32]).unwrap();
    let now = Time::from_unix_millis(NOON);
    let config = PlaylistConfig {
        part_target: ms(1000),
        ..PlaylistConfig::default()
    };
    let stream = profile.open_stream(now, [2; 32], config).unwrap();
    let first = u64::try_from(NOON).unwrap();
    stream.push_part(&part(0, 1000, true, b"part", NOON));
    let path = |f: &str| stream.path().replace("index.m3u8", f);
    let r = routes.get(now, &path(&format!("part-{first}.m4s")));
    assert_eq!(
        (r.status, r.content_type, &*r.body),
        (200, "video/iso.segment", &b"part"[..])
    );
    assert_eq!(r.cache_control, "max-age=31536000, immutable");
    // The hinted part blocks, so answering at once is 503; one past it is
    // missing.
    assert_eq!(
        routes
            .get(now, &path(&format!("part-{}.m4s", first + 1)))
            .status,
        503
    );
    assert_eq!(
        routes
            .get(now, &path(&format!("part-{}.m4s", first + 2)))
            .status,
        404
    );
    for bad in ["part-.m4s", "part-x.m4s", "part--1.m4s"] {
        assert_eq!(routes.get(now, &path(bad)).status, 400, "{bad}");
    }
}

/// A listed segment, or the open one, as a playlist lists it.
#[derive(Debug, Default)]
struct Listed {
    seq: Option<u64>,
    millis: u64,
    date: Option<String>,
    /// Number, milliseconds and independence of each listed part.
    parts: Vec<(u64, u64, bool)>,
}

fn millis_of(s: &str) -> u64 {
    let (whole, frac) = s.split_once('.').unwrap();
    assert_eq!(frac.len(), 3, "{s}");
    whole.parse::<u64>().unwrap() * 1000 + frac.parse::<u64>().unwrap()
}

/// Checks a low-latency media playlist against RFC 8216bis's rules for
/// parts and the playlist's own: the part target and hold-back, parts
/// within the target that add up to their segment, the tags that apply to
/// a segment before its first part, consecutive part numbers, the
/// window of listed parts, the hint, dates in order, and every listed
/// part served with its segment's bytes.
fn check(p: &Playlist) {
    let text = p.render();
    let mut target = None;
    let mut hold_back = None;
    let mut hint = None;
    let mut ended = false;
    let mut segments: Vec<Listed> = Vec::new();
    let mut current = Listed::default();
    let mut lines = text.lines();
    while let Some(l) = lines.next() {
        if let Some(v) = l.strip_prefix("#EXT-X-PART-INF:PART-TARGET=") {
            target = Some(millis_of(v));
        } else if let Some(v) =
            l.strip_prefix("#EXT-X-SERVER-CONTROL:CAN-BLOCK-RELOAD=YES,PART-HOLD-BACK=")
        {
            hold_back = Some(millis_of(v));
        } else if l == "#EXT-X-DISCONTINUITY" || l.starts_with("#EXT-X-MAP:") {
            assert!(current.parts.is_empty(), "{l} after a part:\n{text}");
        } else if let Some(v) = l.strip_prefix("#EXT-X-PROGRAM-DATE-TIME:") {
            assert!(current.parts.is_empty(), "{l} after a part:\n{text}");
            current.date = Some(v.to_owned());
        } else if let Some(v) = l.strip_prefix("#EXT-X-PART:DURATION=") {
            let (d, rest) = v.split_once(',').unwrap();
            let n = rest
                .strip_prefix("URI=\"part-")
                .unwrap()
                .split_once(".m4s\"")
                .unwrap();
            let independent = match n.1 {
                "" => false,
                ",INDEPENDENT=YES" => true,
                other => panic!("{other}"),
            };
            current
                .parts
                .push((n.0.parse().unwrap(), millis_of(d), independent));
        } else if let Some(v) = l.strip_prefix("#EXTINF:") {
            current.millis = millis_of(v.strip_suffix(',').unwrap());
            let uri = lines.next().unwrap();
            current.seq = Some(uri.strip_suffix(".m4s").unwrap().parse().unwrap());
            segments.push(std::mem::take(&mut current));
        } else if let Some(v) = l.strip_prefix("#EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"part-") {
            hint = Some(v.strip_suffix(".m4s\"").unwrap().parse::<u64>().unwrap());
        } else if l == "#EXT-X-ENDLIST" {
            ended = true;
        }
    }
    let target = target.unwrap();
    assert_eq!(hold_back, Some(3 * target));
    let open = current;
    let all: Vec<&Listed> = segments.iter().chain(Some(&open)).collect();
    let numbers: Vec<u64> = all
        .iter()
        .flat_map(|s| s.parts.iter().map(|p| p.0))
        .collect();
    assert!(numbers.windows(2).all(|w| w[0] + 1 == w[1]), "{numbers:?}");
    if ended {
        assert_eq!(hint, None);
        assert!(open.parts.is_empty());
    } else {
        assert_eq!(hint, Some(p.next_part()));
        if let Some(last) = numbers.last() {
            assert_eq!(*last + 1, p.next_part());
        }
    }
    // Segments list their parts from the first that ends within three
    // targets of the end.
    let window = p.target_duration() * 3000;
    let open_millis: u64 = open.parts.iter().map(|p| p.1).sum();
    let mut from_end = open_millis;
    for s in segments.iter().rev() {
        let within = from_end < window;
        assert_eq!(!s.parts.is_empty(), within, "{s:?}\n{text}");
        from_end += s.millis;
    }
    for s in &all {
        for (n, millis, _) in &s.parts {
            assert!(*millis <= target, "part {n}: {millis}\n{text}");
            assert!(p.part(*n).is_some(), "part {n} isn't served");
        }
        if let Some(seq) = s.seq.filter(|_| !s.parts.is_empty()) {
            let sum: u64 = s.parts.iter().map(|p| p.1).sum();
            assert_eq!(sum, s.millis, "segment {seq}\n{text}");
            let bytes: Vec<u8> = s
                .parts
                .iter()
                .flat_map(|(n, ..)| p.part(*n).unwrap().to_vec())
                .collect();
            assert!(bytes == *p.segment(seq).unwrap(), "segment {seq}'s parts");
        }
    }
    let dates: Vec<&String> = all.iter().filter_map(|s| s.date.as_ref()).collect();
    assert!(dates.windows(2).all(|w| w[0] <= w[1]), "{dates:?}");
}

/// `n` frames at 30 a second from `from`, keyframes every `gop`, with
/// AAC over their time, in decode-time order.
fn av(from: i64, n: i64, gop: i64) -> Vec<AccessUnit<'static>> {
    let mut out = Vec::new();
    let mut a = (from * FRAME * 48_000 / 90_000 + 1023) / 1024;
    for f in from..from + n {
        out.push(unit(
            VIDEO,
            f * FRAME,
            f % gop == 0,
            if f % gop == 0 { 3000 } else { 1200 },
        ));
        while a * 1024 * 90_000 <= (f + 1) * FRAME * 48_000 {
            out.push(unit(AUDIO, a * 1024, true, 300));
            a += 1;
        }
    }
    out
}

#[test]
fn a_muxed_stream_keeps_the_rules_at_every_step() {
    let mut p = playlist();
    let mut m = HlsMuxer::new(HlsMuxerConfig {
        part_target: ms(1000),
        ..HlsMuxerConfig::default()
    });
    m.set_config(&video(VIDEO)).unwrap();
    m.set_config(&audio(AUDIO)).unwrap();
    let take = |m: &mut HlsMuxer, p: &mut Playlist| {
        while let Some(x) = m.take_part() {
            p.push_part(&x);
        }
        while let Some(s) = m.take_segment() {
            p.push(&s);
        }
    };
    // 2 s GOPs, then a gap, then 10 s GOPs split within the target, then
    // a 6 s video stall.
    let mut units = av(0, 600, 60);
    units.extend(av(650, 600, 300));
    units.extend(
        av(1250, 600, 60)
            .into_iter()
            .filter(|u| u.track.0 != VIDEO || !(1400 * FRAME..1580 * FRAME).contains(&u.dts)),
    );
    let mut gapped = false;
    for (i, u) in units.iter().enumerate() {
        if !gapped && u.dts >= 650 * FRAME && u.track.0 == VIDEO {
            m.gap();
            gapped = true;
        }
        m.push(u).unwrap();
        take(&mut m, &mut p);
        if i % 29 == 0 {
            check(&p);
        }
    }
    m.finish().unwrap();
    take(&mut m, &mut p);
    check(&p);
    p.end();
    check(&p);
    assert_eq!(m.metrics().dropped_parts, 0);
}

/// A waker that counts its wakes.
struct Wakes(std::sync::atomic::AtomicUsize);

impl std::task::Wake for Wakes {
    fn wake(self: std::sync::Arc<Self>) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Polls a blocked request once: its answer, or `None` while it waits.
fn poll(block: &mut HlsBlock, wakes: &std::sync::Arc<Wakes>) -> Option<HlsResponse> {
    let waker = Waker::from(wakes.clone());
    let mut cx = Context::from_waker(&waker);
    match Pin::new(block).poll(&mut cx) {
        Poll::Ready(r) => Some(r),
        Poll::Pending => None,
    }
}

fn woken(wakes: &std::sync::Arc<Wakes>) -> usize {
    wakes.0.load(std::sync::atomic::Ordering::SeqCst)
}

/// Routes holding a low-latency stream opened at noon: its segments and
/// parts number from noon's milliseconds.
struct Ll {
    routes: HlsRoutes,
    _profile: HlsProfile,
    stream: Option<HlsStream>,
    first: u64,
}

impl Ll {
    fn new(part_target: Duration) -> Ll {
        let routes = HlsRoutes::new();
        let profile = routes.profile(&[1; 32]).unwrap();
        let config = PlaylistConfig {
            part_target,
            ..PlaylistConfig::default()
        };
        let stream = profile.open_stream(at(NOON), [2; 32], config).unwrap();
        Ll {
            routes,
            _profile: profile,
            stream: Some(stream),
            first: u64::try_from(NOON).unwrap(),
        }
    }

    fn stream(&self) -> &HlsStream {
        self.stream.as_ref().unwrap()
    }

    fn path(&self, file: &str) -> String {
        format!(
            "{}{file}",
            self.stream().path().strip_suffix("index.m3u8").unwrap()
        )
    }

    fn request(&self, file: &str) -> HlsAnswer {
        self.routes.request(at(NOON), &self.path(file))
    }

    fn blocked(&self, file: &str) -> HlsBlock {
        match self.request(file) {
            HlsAnswer::Blocked(b) => b,
            HlsAnswer::Ready(r) => panic!("{file}: answered {}", r.status),
        }
    }

    fn status(&self, file: &str) -> u16 {
        match self.request(file) {
            HlsAnswer::Ready(r) => r.status,
            HlsAnswer::Blocked(_) => panic!("{file} blocks"),
        }
    }

    /// A reload of the media playlist asking for segment `seq` (counted
    /// from the first), or part `part` of it.
    fn reload(&self, seq: u64, part: Option<u64>) -> String {
        let msn = self.first + seq;
        match part {
            Some(p) => format!("index.m3u8?_HLS_msn={msn}&_HLS_part={p}"),
            None => format!("index.m3u8?_HLS_msn={msn}"),
        }
    }
}

fn at(ms: i64) -> Time {
    Time::from_unix_millis(ms)
}

#[test]
fn a_reload_blocks_until_the_playlist_lists_what_it_asks_for() {
    let ll = Ll::new(ms(1000));
    let wakes = std::sync::Arc::new(Wakes(Default::default()));
    ll.stream().push_part(&part(0, 1000, true, b"a", NOON));
    assert_eq!(ll.status(&ll.reload(0, Some(0))), 200);

    // The next part.
    let mut next = ll.blocked(&ll.reload(0, Some(1)));
    assert_eq!(next.timeout(), ms(12_000));
    assert!(poll(&mut next, &wakes).is_none());
    assert_eq!(ll.routes.blocked(&[1; 32], &[2; 32]), 1);
    ll.stream().push_part(&part(1, 1000, false, b"b", NOON));
    assert_eq!(woken(&wakes), 1);
    let r = poll(&mut next, &wakes).unwrap();
    assert_eq!(r.status, 200);
    let text = String::from_utf8(r.body.to_vec()).unwrap();
    assert!(
        text.contains(&format!("part-{}.m4s\"\n#EXT-X-PRELOAD", ll.first + 1)),
        "{text}"
    );
    drop(next);
    assert_eq!(ll.routes.blocked(&[1; 32], &[2; 32]), 0);

    // The whole segment, and a part past its last: the next one's first.
    let mut whole = ll.blocked(&ll.reload(0, None));
    let mut past = ll.blocked(&ll.reload(0, Some(2)));
    assert!(poll(&mut whole, &wakes).is_none());
    assert!(poll(&mut past, &wakes).is_none());
    ll.stream().push(&segment(b"ab", 2000, NOON, None));
    assert_eq!(poll(&mut whole, &wakes).unwrap().status, 200);
    assert!(poll(&mut past, &wakes).is_none());
    ll.stream()
        .push_part(&part(0, 1000, true, b"c", NOON + 2000));
    assert_eq!(poll(&mut past, &wakes).unwrap().status, 200);
    // Asked again, both answer at once.
    assert_eq!(ll.status(&ll.reload(0, Some(2))), 200);
    assert_eq!(ll.status(&ll.reload(1, Some(0))), 200);
    // The next segment, and a part of the one after.
    drop(ll.blocked(&ll.reload(1, None)));
    drop(ll.blocked(&ll.reload(2, Some(2))));
}

#[test]
fn the_hinted_part_blocks_until_it_comes() {
    let ll = Ll::new(ms(1000));
    let wakes = std::sync::Arc::new(Wakes(Default::default()));
    let first = ll.first;
    let mut hinted = ll.blocked(&format!("part-{first}.m4s"));
    assert_eq!(ll.status(&format!("part-{}.m4s", first + 1)), 404);
    assert!(poll(&mut hinted, &wakes).is_none());
    ll.stream().push_part(&part(0, 1000, true, b"part", NOON));
    let r = poll(&mut hinted, &wakes).unwrap();
    assert_eq!((r.status, &*r.body), (200, &b"part"[..]));
    // Once the stream ends, no part is hinted.
    ll.stream().push(&segment(b"part", 1000, NOON, None));
    ll.stream().end();
    assert_eq!(ll.status(&format!("part-{}.m4s", first + 1)), 404);
}

#[test]
fn reloads_too_far_ahead_or_malformed_are_refused() {
    let ll = Ll::new(ms(1000));
    ll.stream().push_part(&part(0, 1000, true, b"a", NOON));
    let msn = ll.first;
    for (query, why) in [
        (format!("_HLS_msn={}", msn + 3), "three segments ahead"),
        (
            format!("_HLS_msn={msn}&_HLS_part=4"),
            "past the advance part limit",
        ),
        (
            format!("_HLS_msn={}&_HLS_part=3", msn + 1),
            "past it in the next",
        ),
        ("_HLS_part=1".to_owned(), "a part without a segment"),
        ("_HLS_msn=x".to_owned(), "not a number"),
        ("_HLS_msn=".to_owned(), "empty"),
        (format!("_HLS_msn={msn}&_HLS_msn={msn}"), "twice"),
    ] {
        assert_eq!(ll.status(&format!("index.m3u8?{query}")), 400, "{why}");
    }
    // Within the limit, they block; other parameters are left alone.
    drop(ll.blocked(&format!("index.m3u8?_HLS_msn={msn}&_HLS_part=3")));
    drop(ll.blocked(&format!("index.m3u8?_HLS_msn={}", msn + 2)));
    drop(ll.blocked(&format!("index.m3u8?x=1&_HLS_msn={msn}&_HLS_skip=YES")));
    assert_eq!(ll.status("index.m3u8?x=1&_HLS_skip=YES"), 200);
    // Queries of other files are ignored.
    assert_eq!(ll.status("multivariant.m3u8?_HLS_part=1"), 200);
    // Shorter parts take a longer limit: three seconds' worth.
    let short = Ll::new(ms(500));
    short.stream().push_part(&part(0, 500, true, b"a", NOON));
    drop(short.blocked(&format!("index.m3u8?_HLS_msn={msn}&_HLS_part=6")));
    assert_eq!(
        short.status(&format!("index.m3u8?_HLS_msn={msn}&_HLS_part=7")),
        400
    );
}

#[test]
fn an_ended_playlist_and_one_without_parts_answer_reloads_at_once() {
    let ll = Ll::new(ms(1000));
    ll.stream().push_part(&part(0, 1000, true, b"a", NOON));
    ll.stream().end();
    assert_eq!(ll.status(&ll.reload(5, Some(1))), 200);
    let plain = Ll::new(Duration::ZERO);
    plain.stream().push_segment(b"init", b"a", ms(1000));
    let text = String::from_utf8(
        plain
            .routes
            .get(at(NOON), plain.stream().path())
            .body
            .to_vec(),
    )
    .unwrap();
    assert!(!text.contains("SERVER-CONTROL"), "{text}");
    assert_eq!(plain.status(&plain.reload(1, None)), 200);
}

#[test]
fn the_subtitles_playlist_and_its_hinted_part_block_as_the_media_playlists_do() {
    let ll = Ll::new(ms(1000));
    let wakes = std::sync::Arc::new(Wakes(Default::default()));
    ll.stream().push_part(&part(0, 1000, true, b"a", NOON));
    let (msn, first) = (ll.first, ll.first);
    let mut reload = ll.blocked(&format!("subtitles.m3u8?_HLS_msn={msn}&_HLS_part=1"));
    let mut hinted = ll.blocked(&format!("part-{}.vtt", first + 1));
    assert!(poll(&mut reload, &wakes).is_none());
    assert!(poll(&mut hinted, &wakes).is_none());
    let mut next = part(1, 1000, false, b"b", NOON);
    next.vtt = Some(Rc::from(&b"WEBVTT\n\ncue"[..]));
    ll.stream().push_part(&next);
    let r = poll(&mut reload, &wakes).unwrap();
    let text = String::from_utf8(r.body.to_vec()).unwrap();
    assert!(
        text.contains(&format!("URI=\"part-{}.vtt\"\n", first + 1)),
        "{text}"
    );
    assert!(!text.contains(".m4s"), "{text}");
    let r = poll(&mut hinted, &wakes).unwrap();
    assert_eq!(
        (r.status, r.content_type, &*r.body),
        (200, "text/vtt", &b"WEBVTT\n\ncue"[..])
    );
    // A part of the stream before any text track is an empty WebVTT file.
    let r = ll
        .routes
        .get(at(NOON), &ll.path(&format!("part-{first}.vtt")));
    assert_eq!(&*r.body, EMPTY_VTT.as_bytes());
    assert_eq!(ll.status(&format!("part-{}.vtt", first + 3)), 404);
    assert_eq!(ll.status("part-x.vtt"), 400);
}

#[test]
fn blocked_requests_answer_404_once_their_stream_goes() {
    let mut ll = Ll::new(ms(1000));
    let wakes = std::sync::Arc::new(Wakes(Default::default()));
    let mut reload = ll.blocked(&ll.reload(0, Some(0)));
    assert!(poll(&mut reload, &wakes).is_none());
    drop(ll.stream.take());
    assert_eq!(woken(&wakes), 1);
    assert_eq!(poll(&mut reload, &wakes).unwrap().status, 404);
}

#[test]
fn blocked_requests_are_bounded_per_stream() {
    let ll = Ll::new(ms(1000));
    let mut blocked: Vec<HlsBlock> = (0..MAX_BLOCKED_REQUESTS)
        .map(|_| ll.blocked(&ll.reload(0, Some(0))))
        .collect();
    assert_eq!(ll.status(&ll.reload(0, Some(0))), 503);
    assert_eq!(ll.status(&format!("part-{}.m4s", ll.first)), 503);
    // Requests that answer at once still do.
    assert_eq!(ll.status("index.m3u8"), 200);
    blocked.pop();
    drop(ll.blocked(&ll.reload(0, Some(0))));
    drop(blocked);
    assert_eq!(ll.routes.blocked(&[1; 32], &[2; 32]), 0);
}
