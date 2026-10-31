// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::*;
use crate::test_util::{HevcSpsSpec, hvcc};

/// hvcC records x265 wrote through ffmpeg's FLV muxer, without the SEI
/// array of its options: 1280x720 Main, 1366x768 Main (a conformance
/// window), 1920x1080 Main 10, and 642x362 4:2:2 10-bit (Range
/// Extensions).
const X265_720P: &str = "0101600000009000000000005df000fcfdf8f800000f03200001001840010c01ffff01600000030090000003000003005d959409210001002b42010101600000030090000003000003005da00280802d16595952930bc05a020000030002000003003c1022000100064401c073c189";
const X265_768P: &str = "01016000000090000000000078f000fcfdf8f800000f03200001001840010c01ffff016000000300900000030000030078959409210001002c420101016000000300900000030000030078a002b080301cde595952930bc05a020000030002000003003c1022000100064401c073c189";
const X265_1080P_MAIN10: &str = "01022000000090000000000078f000fcfdfafa00000f03200001001840010c01ffff022000000300900000030000030078959409210001002d420101022000000300900000030000030078a003c0801107cad965654a4c2f016808000003000800000300f04022000100064401c073c189";
const X265_422_10: &str = "0104080000009d08000000003ff000fcfefafa00000f03200001001740010c01ffff0408000003009d0800000300003f959409210001002c4201010408000003009d0800000300003fb005220171c44ed965654a4c2f016808000003000800000300f04022000100064401c073c189";

fn x265(hex: &str) -> HevcConfig {
    parse_hvcc(&hex::decode(hex).unwrap()).unwrap()
}

#[test]
fn x265_records_name_their_codec_and_size() {
    let cases = [
        (X265_720P, "hvc1.1.6.L93.90", (1280, 720), 1, 8),
        (X265_768P, "hvc1.1.6.L120.90", (1366, 768), 1, 8),
        (X265_1080P_MAIN10, "hvc1.2.4.L120.90", (1920, 1080), 1, 10),
        (X265_422_10, "hvc1.4.10.L63.9D.08", (642, 362), 2, 10),
    ];
    for (hex, codec, (width, height), chroma, depth) in cases {
        let c = x265(hex);
        assert_eq!(c.codec(), codec);
        assert_eq!((c.sps.width, c.sps.height), (width, height), "{codec}");
        assert_eq!(c.sps.chroma_format_idc, chroma, "{codec}");
        assert_eq!(
            (c.sps.bit_depth_luma, c.sps.bit_depth_chroma),
            (depth, depth)
        );
        assert_eq!(c.nal_length_size, 4);
    }
}

#[test]
fn codec_strings_follow_iso_14496_15_annex_e() {
    let mut c = x265(X265_720P);
    c.profile_space = 2;
    c.high_tier = true;
    c.constraints = [0, 0, 0, 0, 0, 0];
    assert_eq!(c.codec(), "hvc1.B1.6.H93");
    c.constraints = [0xb0, 0, 0x01, 0, 0, 0];
    c.compatibility = 0x8000_0001;
    assert_eq!(c.codec(), "hvc1.B1.80000001.H93.B0.00.01");
}

#[test]
fn conformance_windows_count_in_chroma_samples() {
    let window = Some([1, 2, 3, 4]);
    let cases = [
        (1, false, (100 - 2 * 3, 100 - 2 * 7)),
        (2, false, (100 - 2 * 3, 100 - 7)),
        (3, false, (100 - 3, 100 - 7)),
        (3, true, (100 - 3, 100 - 7)),
        (0, false, (100 - 3, 100 - 7)),
    ];
    for (chroma_format_idc, separate_colour_plane, size) in cases {
        let spec = HevcSpsSpec {
            chroma_format_idc,
            separate_colour_plane,
            window,
            ..HevcSpsSpec::main(100, 100)
        };
        let sps = parse_sps(&spec.build()).unwrap();
        assert_eq!((sps.width, sps.height), size, "chroma {chroma_format_idc}");
    }
}

#[test]
fn sub_layer_profiles_and_levels_are_read_past() {
    let spec = HevcSpsSpec {
        sub_layers: 4,
        sub_layer_present: vec![(true, true), (false, true), (true, false)],
        bit_depth_luma: 10,
        bit_depth_chroma: 12,
        ..HevcSpsSpec::main(3840, 2160)
    };
    let sps = parse_sps(&spec.build()).unwrap();
    assert_eq!((sps.width, sps.height), (3840, 2160));
    assert_eq!((sps.bit_depth_luma, sps.bit_depth_chroma), (10, 12));
}

#[test]
fn emulation_prevention_bytes_are_dropped() {
    // 1024 is coded as 00000000001 0000000001: with the bits before it, two
    // zero bytes the writer escapes.
    let spec = HevcSpsSpec::main(1024, 576);
    let nal = spec.build();
    assert!(nal.windows(3).any(|w| w == [0, 0, 3]));
    let sps = parse_sps(&nal).unwrap();
    assert_eq!((sps.width, sps.height), (1024, 576));
}

#[test]
fn sps_out_of_range_fields_are_refused() {
    let cases = [
        (
            HevcSpsSpec {
                sub_layers: 8,
                ..HevcSpsSpec::main(64, 64)
            },
            "sps_max_sub_layers_minus1",
        ),
        (
            HevcSpsSpec {
                chroma_format_idc: 4,
                ..HevcSpsSpec::main(64, 64)
            },
            "chroma_format_idc",
        ),
        (
            HevcSpsSpec {
                bit_depth_luma: 17,
                ..HevcSpsSpec::main(64, 64)
            },
            "bit depth",
        ),
        (
            HevcSpsSpec {
                window: Some([16, 16, 0, 0]),
                ..HevcSpsSpec::main(64, 64)
            },
            "picture width",
        ),
        (HevcSpsSpec::main(64, 65_536), "picture height"),
        (HevcSpsSpec::main(0, 64), "picture width"),
    ];
    for (spec, field) in cases {
        let nal = if spec.sub_layers > 7 {
            // The writer's 3-bit field can't hold 8.
            let mut nal = HevcSpsSpec::main(64, 64).build();
            nal[2] |= 0x0e;
            nal
        } else {
            spec.build()
        };
        assert_eq!(parse_sps(&nal), Err(HevcError::SpsField(field)), "{field}");
    }
}

#[test]
fn sps_must_be_an_sps_nal_unit() {
    let mut nal = HevcSpsSpec::main(64, 64).build();
    nal[0] = 0x44;
    assert_eq!(parse_sps(&nal), Err(HevcError::NotSps));
    assert_eq!(parse_sps(&[0x42]), Err(HevcError::SpsTruncated));
    assert_eq!(parse_sps(&[0x42, 0x01, 0x01]), Err(HevcError::SpsTruncated));
}

#[test]
fn hvcc_header_errors() {
    let good = hvcc(&HevcSpsSpec::main(640, 360).build());
    assert!(parse_hvcc(&good).is_ok());

    let mut version = good.clone();
    version[0] = 0;
    assert_eq!(parse_hvcc(&version), Err(HevcError::Version(0)));

    let mut length = good.clone();
    length[21] = (length[21] & !0x03) | 0x02;
    assert_eq!(parse_hvcc(&length), Err(HevcError::NalLengthSize(3)));

    for n in 0..good.len() {
        assert!(parse_hvcc(&good[..n]).is_err(), "{n} bytes");
    }

    let mut no_sps = good[..22].to_vec();
    no_sps.push(0);
    assert_eq!(parse_hvcc(&no_sps), Err(HevcError::NoSps));

    // an SPS array whose NAL unit is a PPS
    let mut not_sps = good.clone();
    let at = 23 + 3 + 2 + 3 + 3 + 2;
    assert_eq!(not_sps[at] >> 1, 33);
    not_sps[at] = 34 << 1;
    assert_eq!(parse_hvcc(&not_sps), Err(HevcError::NotSps));
}

#[test]
fn hvcc_reads_through_every_array_and_ignores_trailing_bytes() {
    let mut record = hvcc(&HevcSpsSpec::main(640, 360).build());
    record.extend_from_slice(&[0xde, 0xad]);
    let c = parse_hvcc(&record).unwrap();
    assert_eq!((c.sps.width, c.sps.height), (640, 360));
    assert_eq!(c.codec(), "hvc1.1.6.L93.90");
}

#[test]
fn hvcc_over_the_bound_is_refused() {
    let mut record = hvcc(&HevcSpsSpec::main(640, 360).build());
    record.resize(MAX_CONFIG_LEN, 0);
    assert!(parse_hvcc(&record).is_ok());
    record.push(0);
    assert_eq!(parse_hvcc(&record), Err(HevcError::TooLong));
}

fn config() -> Config {
    Config {
        cases: 1024,
        rng_seed: RngSeed::Fixed(0x0e7c_5eed),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn arbitrary_hvcc_never_panics(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = parse_hvcc(&data);
    }

    #[test]
    fn arbitrary_sps_never_panics(data in proptest::collection::vec(any::<u8>(), 0..256)) {
        let mut nal = vec![0x42, 0x01];
        nal.extend_from_slice(&data);
        if let Ok(sps) = parse_sps(&nal) {
            prop_assert!(sps.width > 0 && sps.height > 0);
        }
        let _ = parse_hvcc(&hvcc(&nal));
    }

    #[test]
    fn mutated_records_never_panic(at in 0usize..128, byte: u8) {
        let mut record = hvcc(&HevcSpsSpec::main(1920, 1080).build());
        let at = at % record.len();
        record[at] = byte;
        if let Ok(c) = parse_hvcc(&record) {
            prop_assert!(c.sps.width > 0 && c.sps.height > 0);
        }
    }

    #[test]
    fn built_sps_round_trips(
        coded_width in 1u32..=8192,
        coded_height in 1u32..=8192,
        chroma_format_idc in 0u32..=3,
        crop in proptest::array::uniform4(0u32..=3),
        sub_layers in 1u32..=7,
        present in proptest::collection::vec((any::<bool>(), any::<bool>()), 6),
    ) {
        let (sw, sh) = match chroma_format_idc {
            1 => (2, 2),
            2 => (2, 1),
            _ => (1, 1),
        };
        let width = coded_width.saturating_sub(sw * (crop[0] + crop[1]));
        let height = coded_height.saturating_sub(sh * (crop[2] + crop[3]));
        prop_assume!(width > 0 && height > 0);
        let spec = HevcSpsSpec {
            sub_layers,
            sub_layer_present: present,
            chroma_format_idc,
            window: Some(crop),
            ..HevcSpsSpec::main(coded_width, coded_height)
        };
        let sps = parse_sps(&spec.build()).unwrap();
        prop_assert_eq!((u32::from(sps.width), u32::from(sps.height)), (width, height));
        prop_assert_eq!(u32::from(sps.chroma_format_idc), chroma_format_idc);
    }
}
