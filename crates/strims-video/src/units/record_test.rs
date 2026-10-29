// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::*;
use crate::synth::script::{aac_lc_asc, avcc};

fn proptest_config() -> Config {
    Config {
        cases: 1024,
        rng_seed: RngSeed::Fixed(0x0051_7e5e),
        failure_persistence: None,
        ..Config::default()
    }
}

fn video() -> DecoderConfig {
    ConfigEntry {
        track: TrackId(1),
        version: 0,
        next_seq: 0,
        sample_entry: SAMPLE_ENTRY_AVC,
        timescale: 90_000,
        config: &avcc(640, 360),
    }
    .decoder_config()
    .unwrap()
    .unwrap()
}

fn audio() -> DecoderConfig {
    ConfigEntry {
        track: TrackId(2),
        version: 0,
        next_seq: 0,
        sample_entry: SAMPLE_ENTRY_AAC,
        timescale: 48_000,
        config: &aac_lc_asc(48_000),
    }
    .decoder_config()
    .unwrap()
    .unwrap()
}

fn header() -> UnitHeader {
    UnitHeader {
        track: TrackId(1),
        keyframe: true,
        seq: 300,
        config: 2,
        dts: 90_000,
        pts: 93_000,
        published: Time::from_unix_millis(1_793_275_200_250),
    }
}

fn unit_record(gop_back: u64, h: &UnitHeader, payload: &[u8]) -> Vec<u8> {
    let mut r = Vec::new();
    write_unit_head(gop_back, h, &mut r);
    r.extend_from_slice(payload);
    r
}

#[test]
fn units_read_back_as_written() {
    let h = header();
    let r = unit_record(1234, &h, b"payload");
    assert_eq!(
        parse_record(&r).unwrap(),
        Record::Unit {
            gop_back: 1234,
            header: h,
            payload: b"payload",
        }
    );
    let b = UnitHeader {
        keyframe: false,
        pts: h.dts - 3000,
        ..h
    };
    let r = unit_record(0, &b, b"");
    assert_eq!(
        parse_record(&r).unwrap(),
        Record::Unit {
            gop_back: 0,
            header: b,
            payload: b"",
        }
    );
}

#[test]
fn configs_read_back_as_written_and_derive_their_configs() {
    let (v, a) = (video(), audio());
    assert_eq!(v.codec, "avc1.64001f");
    assert_eq!(a.codec, "mp4a.40.2");
    let mut r = Vec::new();
    write_configs(
        0,
        [ConfigEntry::of(&v, 3, 100), ConfigEntry::of(&a, 0, 250)],
        &mut r,
    );
    let Record::Configs { gop_back, entries } = parse_record(&r).unwrap() else {
        panic!("not configs");
    };
    assert_eq!(gop_back, 0);
    let entries: Vec<ConfigEntry<'_>> = entries.collect();
    assert_eq!(
        entries,
        [ConfigEntry::of(&v, 3, 100), ConfigEntry::of(&a, 0, 250)]
    );
    assert_eq!(entries[0].decoder_config().unwrap().unwrap(), v);
    assert_eq!(entries[1].decoder_config().unwrap().unwrap(), a);
    assert!(entries[0].describes(&v) && !entries[0].describes(&a));
}

#[test]
fn readers_skip_header_fields_and_entry_bytes_they_dont_know() {
    // A header with a field after the known ones.
    let h = header();
    let mut known = Vec::new();
    write_unit_head(7, &h, &mut known);
    let header_len = known[2] as usize;
    let mut r = vec![KIND_UNIT, 7, header_len as u8 + 2];
    r.extend_from_slice(&known[3..]);
    r.extend_from_slice(&[0xaa, 0xbb]);
    r.extend_from_slice(b"data");
    assert_eq!(
        parse_record(&r).unwrap(),
        Record::Unit {
            gop_back: 7,
            header: h,
            payload: b"data",
        }
    );
    // Unknown flag bits.
    let mut f = unit_record(7, &h, b"data");
    f[4] |= 0x80;
    assert!(matches!(parse_record(&f).unwrap(), Record::Unit { header, .. } if header == h));

    // An entry with bytes after its config, and one of an unknown sample
    // entry.
    let v = video();
    let mut c = Vec::new();
    write_configs(0, [ConfigEntry::of(&v, 0, 0)], &mut c);
    let len = c[3] as usize;
    let mut r = vec![KIND_CONFIGS, 0, 0, len as u8 + 1];
    r.extend_from_slice(&c[4..]);
    r.push(0xee);
    let opus = ConfigEntry {
        track: TrackId(3),
        version: 0,
        next_seq: 0,
        sample_entry: *b"Opus",
        timescale: 48_000,
        config: b"OpusHead",
    };
    let mut o = Vec::new();
    write_configs(0, [opus], &mut o);
    r.extend_from_slice(&o[3..]);
    let Record::Configs { entries, .. } = parse_record(&r).unwrap() else {
        panic!("not configs");
    };
    let entries: Vec<ConfigEntry<'_>> = entries.collect();
    assert_eq!(entries, [ConfigEntry::of(&v, 0, 0), opus]);
    assert!(entries[0].is_known() && !entries[1].is_known());
    assert!(entries[1].decoder_config().is_none());
}

#[test]
fn records_of_unknown_kinds_read_as_other_with_their_gop() {
    assert_eq!(
        parse_record(&[3, 0x81, 0x01, 0xff]).unwrap(),
        Record::Other {
            kind: 3,
            gop_back: 129
        }
    );
    assert_eq!(gop_back(&[3, 0x81, 0x01]), Some(129));
    assert_eq!(gop_back(&[0, 1]), None);
    assert_eq!(gop_back(&[2, 0x81]), None);
    assert_eq!(gop_back(&[]), None);
}

#[test]
fn malformed_records_are_refused() {
    let v = video();
    let configs = |entries: &[ConfigEntry<'_>]| {
        let mut r = Vec::new();
        write_configs(0, entries.iter().copied(), &mut r);
        parse_record(&r).map(|_| ())
    };
    assert_eq!(parse_record(&[]), Err(RecordError::Truncated));
    assert_eq!(parse_record(&[0, 0, 0]), Err(RecordError::KindZero));
    assert_eq!(parse_record(&[KIND_UNIT, 0]), Err(RecordError::Truncated));
    // A header longer than the record.
    assert_eq!(
        parse_record(&[KIND_UNIT, 0, 9, 1]),
        Err(RecordError::Truncated)
    );
    let r = unit_record(0, &header(), b"x");
    // Every cut short of the payload.
    for n in 0..r.len() - 1 {
        assert!(parse_record(&r[..n]).is_err(), "{n}");
    }
    assert_eq!(
        parse_record(&unit_record(
            0,
            &UnitHeader {
                track: TrackId(0),
                ..header()
            },
            b""
        )),
        Err(RecordError::TrackZero)
    );
    // A track id over u32.
    assert_eq!(
        parse_record(&[KIND_UNIT, 0, 5, 0x80, 0x80, 0x80, 0x80, 0x10]),
        Err(RecordError::Range("track"))
    );
    assert_eq!(configs(&[]), Err(RecordError::NoTracks));
    let e = ConfigEntry::of(&v, 0, 0);
    assert_eq!(configs(&[e, e]), Err(RecordError::DuplicateTrack(1)));
    let many: Vec<ConfigEntry<'_>> = (1..=MAX_TRACKS as u32 + 1)
        .map(|t| ConfigEntry {
            track: TrackId(t),
            ..e
        })
        .collect();
    assert_eq!(configs(&many[..MAX_TRACKS]), Ok(()));
    assert_eq!(configs(&many), Err(RecordError::TooManyTracks));
    assert_eq!(
        configs(&[ConfigEntry { timescale: 0, ..e }]),
        Err(RecordError::ZeroTimescale(1))
    );
    let long = vec![0; MAX_CONFIG_LEN + 1];
    assert_eq!(
        configs(&[ConfigEntry { config: &long, ..e }]),
        Err(RecordError::ConfigTooLong)
    );
    let big = vec![0; MAX_RECORD_LEN];
    assert_eq!(
        parse_record(&unit_record(0, &header(), &big)),
        Err(RecordError::TooLong)
    );
    // Configs that don't parse, and an AAC timescale other than its rate.
    let bad = ConfigEntry {
        config: b"\x02",
        ..e
    };
    assert!(matches!(
        bad.decoder_config(),
        Some(Err(RecordError::Avc { track: 1, .. }))
    ));
    let a = audio();
    let off = ConfigEntry {
        timescale: 44_100,
        ..ConfigEntry::of(&a, 0, 0)
    };
    assert_eq!(off.decoder_config(), Some(Err(RecordError::Timescale(2))));
}

#[test]
fn zigzag_round_trips_extremes() {
    for v in [0, 1, -1, 2, -2, i64::MAX, i64::MIN] {
        assert_eq!(unzigzag(zigzag(v)), v);
    }
    assert_eq!([zigzag(0), zigzag(-1), zigzag(1), zigzag(-2)], [0, 1, 2, 3]);
}

fn headers() -> impl Strategy<Value = UnitHeader> {
    (
        1..=u32::MAX,
        any::<bool>(),
        any::<u64>(),
        any::<u32>(),
        any::<i64>(),
        -1_000_000i64..1_000_000,
        any::<i64>(),
    )
        .prop_map(|(track, keyframe, seq, config, dts, cts, ms)| UnitHeader {
            track: TrackId(track),
            keyframe,
            seq,
            config,
            dts,
            pts: dts.saturating_add(cts),
            published: Time::from_unix_millis(ms / 1_000_000),
        })
}

proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn unit_headers_round_trip(
        h in headers(),
        gop in any::<u64>(),
        payload in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        // A saturated PTS is the one value a CTS can't carry back.
        prop_assume!(h.pts.checked_sub(h.dts).is_some());
        let r = unit_record(gop, &h, &payload);
        prop_assert_eq!(
            parse_record(&r).unwrap(),
            Record::Unit { gop_back: gop, header: h, payload: &payload }
        );
        prop_assert_eq!(gop_back(&r[..r.len().min(GOP_HEAD_LEN)]), Some(gop));
    }

    #[test]
    fn arbitrary_records_never_panic(
        kind in prop::sample::select(vec![KIND_CONFIGS, KIND_UNIT, 0, 7]),
        tail in proptest::collection::vec(any::<u8>(), 0..256),
    ) {
        let mut r = vec![kind];
        r.extend(tail);
        if let Ok(Record::Configs { entries, .. }) = parse_record(&r) {
            for e in entries {
                let _ = e.decoder_config();
            }
        }
        let _ = gop_back(&r);
    }
}
