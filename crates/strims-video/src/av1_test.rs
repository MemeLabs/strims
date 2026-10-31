// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::*;
use crate::test_util::{Av1SeqSpec, av1c, obu};

/// av1C records SVT-AV1 wrote through ffmpeg's FLV muxer: 1280x720 8-bit
/// and 1366x768 10-bit, both 4:2:0 Main profile at level 3.1.
const SVT_720P: &str = "81050c000a0b0000002d4cffb3c02f8004";
const SVT_768P_10BIT: &str = "81054c000a0b0000002d4d55bfc02f8404";

#[test]
fn svt_records_name_their_codec_and_size() {
    for (hex, codec, size, depth) in [
        (SVT_720P, "av01.0.05M.08", (1280, 720), 8),
        (SVT_768P_10BIT, "av01.0.05M.10", (1366, 768), 10),
    ] {
        let c = parse_av1c(&hex::decode(hex).unwrap()).unwrap();
        assert_eq!(c.codec(), codec);
        assert_eq!((c.width, c.height), size, "{codec}");
        assert_eq!(c.bit_depth, depth);
        assert!(!c.monochrome);
    }
}

#[test]
fn codec_strings_name_profile_level_tier_and_depth() {
    let mut record = av1c(&Av1SeqSpec::main(64, 64).build());
    record[1] = (2 << 5) | 13;
    record[2] = 0x80 | 0x40 | 0x20;
    let c = parse_av1c(&record).unwrap();
    assert_eq!(c.codec(), "av01.2.13H.12");
    record[2] = 0x10;
    let c = parse_av1c(&record).unwrap();
    assert_eq!(c.codec(), "av01.2.13M.08");
    assert!(c.monochrome);
}

#[test]
fn sequence_headers_are_read_past_their_optional_fields() {
    let specs = [
        Av1SeqSpec {
            reduced_still_picture: true,
            ..Av1SeqSpec::main(4096, 2304)
        },
        Av1SeqSpec {
            timing: Some((true, None)),
            ..Av1SeqSpec::main(4096, 2304)
        },
        Av1SeqSpec {
            timing: Some((false, Some(23))),
            initial_display_delay: true,
            operating_points: vec![(4, true, false), (12, false, true), (19, true, true)],
            ..Av1SeqSpec::main(4096, 2304)
        },
        Av1SeqSpec {
            width_bits: 13,
            height_bits: 12,
            ..Av1SeqSpec::main(4096, 2304)
        },
    ];
    for spec in specs {
        assert_eq!(
            parse_sequence_header(&spec.build()),
            Ok((4096, 2304)),
            "{spec:?}"
        );
    }
}

#[test]
fn frame_sizes_past_16_bits_are_refused() {
    let spec = Av1SeqSpec {
        width_bits: 16,
        ..Av1SeqSpec::main(65_536, 64)
    };
    assert_eq!(
        parse_sequence_header(&spec.build()),
        Err(Av1Error::SequenceHeaderField("max_frame_width_minus_1"))
    );
}

#[test]
fn av1c_header_errors() {
    let good = av1c(&Av1SeqSpec::main(640, 360).build());
    assert!(parse_av1c(&good).is_ok());

    let mut marker = good.clone();
    marker[0] = 0x01;
    assert_eq!(parse_av1c(&marker), Err(Av1Error::Marker));
    let mut version = good.clone();
    version[0] = 0x82;
    assert_eq!(parse_av1c(&version), Err(Av1Error::Version(2)));

    for n in 0..good.len() {
        assert!(parse_av1c(&good[..n]).is_err(), "{n} bytes");
    }
    assert_eq!(parse_av1c(&good[..4]), Err(Av1Error::NoSequenceHeader));
}

#[test]
fn the_sequence_header_may_follow_other_config_obus() {
    let mut record = good_header();
    record.extend(obu(5, &[4, 1, 2]));
    record.extend(obu(1, &Av1SeqSpec::main(320, 180).build()));
    assert_eq!(
        parse_av1c(&record).map(|c| (c.width, c.height)),
        Ok((320, 180))
    );

    let mut metadata_only = good_header();
    metadata_only.extend(obu(5, &[4, 1, 2]));
    assert_eq!(parse_av1c(&metadata_only), Err(Av1Error::NoSequenceHeader));
}

fn good_header() -> Vec<u8> {
    vec![0x81, 0x08, 0x0c, 0x00]
}

#[test]
fn malformed_obus_are_refused() {
    // forbidden bit
    let mut forbidden = good_header();
    forbidden.extend([0x8a, 0x00]);
    assert_eq!(parse_av1c(&forbidden), Err(Av1Error::Obu));
    // a size past the record's end
    let mut long = good_header();
    long.extend([0x0a, 0x20, 0, 0]);
    assert_eq!(parse_av1c(&long), Err(Av1Error::Obu));
    // a leb128 of nine bytes
    let mut leb = good_header();
    leb.push(0x0a);
    leb.extend([0x80; 8]);
    leb.push(0);
    assert_eq!(parse_av1c(&leb), Err(Av1Error::Obu));
    // a leb128 past 2^32
    let mut big = good_header();
    big.extend([0x0a, 0xff, 0xff, 0xff, 0xff, 0x7f]);
    assert_eq!(parse_av1c(&big), Err(Av1Error::Obu));
    // an extension header, then nothing
    let mut ext = good_header();
    ext.push(0x0e);
    assert_eq!(parse_av1c(&ext), Err(Av1Error::Obu));
}

#[test]
fn an_unsized_sequence_header_lasts_to_the_end() {
    let mut record = good_header();
    record.push(0x08);
    record.extend(Av1SeqSpec::main(800, 600).build());
    assert_eq!(
        parse_av1c(&record).map(|c| (c.width, c.height)),
        Ok((800, 600))
    );
}

#[test]
fn av1c_over_the_bound_is_refused() {
    let mut record = av1c(&Av1SeqSpec::main(640, 360).build());
    record.resize(MAX_CONFIG_LEN, 0);
    assert!(parse_av1c(&record).is_ok());
    record.push(0);
    assert_eq!(parse_av1c(&record), Err(Av1Error::TooLong));
}

#[test]
fn temporal_delimiters_are_stripped_from_a_units_start() {
    let frame = obu(6, &[1, 2, 3]);
    let td = obu(2, &[]);
    let with = [td.clone(), td.clone(), frame.clone()].concat();
    assert_eq!(strip_temporal_delimiters(&with), &frame[..]);
    // with an extension header
    let ext = [vec![0x16, 0x00, 0x00], frame.clone()].concat();
    assert_eq!(strip_temporal_delimiters(&ext), &frame[..]);
    assert_eq!(strip_temporal_delimiters(&frame), &frame[..]);
    assert_eq!(strip_temporal_delimiters(&td), &[] as &[u8]);
    // an unsized delimiter would take the rest: it is left
    let unsized_td = [vec![0x10], frame.clone()].concat();
    assert_eq!(strip_temporal_delimiters(&unsized_td), &unsized_td[..]);
    // only delimiters at the start
    let later = [frame.clone(), td].concat();
    assert_eq!(strip_temporal_delimiters(&later), &later[..]);
    let garbage = [0xff, 0x00];
    assert_eq!(strip_temporal_delimiters(&garbage), &garbage[..]);
}

fn config() -> Config {
    Config {
        cases: 1024,
        rng_seed: RngSeed::Fixed(0x0a71_5eed),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn arbitrary_av1c_never_panics(data in proptest::collection::vec(any::<u8>(), 0..256)) {
        let _ = parse_av1c(&data);
    }

    #[test]
    fn arbitrary_sequence_headers_never_panic(
        data in proptest::collection::vec(any::<u8>(), 0..128),
    ) {
        let _ = parse_sequence_header(&data);
        let mut record = good_header();
        record.push(0x08);
        record.extend_from_slice(&data);
        if let Ok(c) = parse_av1c(&record) {
            prop_assert!(c.width > 0 && c.height > 0);
        }
    }

    #[test]
    fn stripping_yields_a_suffix(data in proptest::collection::vec(any::<u8>(), 0..64)) {
        let rest = strip_temporal_delimiters(&data);
        prop_assert!(data.ends_with(rest));
    }

    #[test]
    fn built_sequence_headers_round_trip(
        width in 1u32..=65_535,
        height in 1u32..=65_535,
        reduced: bool,
        timing in proptest::option::of((any::<bool>(), proptest::option::of(1u32..=32))),
        delay: bool,
        points in proptest::collection::vec((0u32..32, any::<bool>(), any::<bool>()), 1..=32),
    ) {
        let bits = |v: u32| 32 - (v - 1).leading_zeros().min(31);
        let spec = Av1SeqSpec {
            reduced_still_picture: reduced,
            timing,
            initial_display_delay: delay,
            operating_points: points,
            width_bits: bits(width).max(1),
            height_bits: bits(height).max(1),
            ..Av1SeqSpec::main(width, height)
        };
        prop_assert_eq!(
            parse_sequence_header(&spec.build()),
            Ok((u16::try_from(width).unwrap(), u16::try_from(height).unwrap()))
        );
    }
}
