// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::*;
use crate::test_util::{SpsSpec, avcc};

fn size(spec: &SpsSpec) -> (u16, u16) {
    let sps = parse_sps(&spec.build()).unwrap();
    (sps.width, sps.height)
}

#[test]
fn baseline_720p() {
    let sps = parse_sps(&SpsSpec::baseline(80, 45).build()).unwrap();
    assert_eq!(
        sps,
        Sps {
            profile_idc: 66,
            level_idc: 31,
            chroma_format_idc: 1,
            frame_mbs_only: true,
            width: 1280,
            height: 720,
        }
    );
}

#[test]
fn high_profile_1080p_crops_the_bottom_rows() {
    let spec = SpsSpec {
        crop: Some([0, 0, 0, 4]),
        ..SpsSpec::high(120, 68)
    };
    assert_eq!(size(&spec), (1920, 1080));
}

#[test]
fn high_profile_skips_scaling_lists() {
    let flat: Vec<i32> = vec![1; 16];
    let mut wrapping: Vec<i32> = vec![-100, 127, -128];
    wrapping.extend(std::iter::repeat_n(0, 61));
    let spec = SpsSpec {
        scaling_lists: Some(vec![
            Some(flat.clone()),
            None,
            // delta_scale reaching 0 ends the list: the default matrix
            Some(vec![-8]),
            Some(flat),
            None,
            None,
            Some(wrapping),
            Some(vec![5, -13]),
        ]),
        crop: Some([0, 0, 0, 4]),
        ..SpsSpec::high(120, 68)
    };
    assert_eq!(size(&spec), (1920, 1080));
}

#[test]
fn high_444_reads_twelve_scaling_lists() {
    let mut lists = vec![None; 12];
    lists[11] = Some(vec![2; 64]);
    lists[3] = Some(vec![-8]);
    let spec = SpsSpec {
        profile: 244,
        chroma_format_idc: 3,
        scaling_lists: Some(lists),
        ..SpsSpec::high(40, 30)
    };
    assert_eq!(size(&spec), (640, 480));
}

#[test]
fn crop_units_follow_the_chroma_format() {
    let crop = Some([0, 1, 0, 1]);
    let with = |chroma_format_idc, frame_mbs_only| SpsSpec {
        chroma_format_idc,
        frame_mbs_only,
        crop,
        ..SpsSpec::high(10, 10)
    };
    // 4:2:0 crops in pairs of samples both ways
    assert_eq!(size(&with(1, true)), (158, 158));
    // 4:2:2 crops pairs across, rows down
    assert_eq!(size(&with(2, true)), (158, 159));
    // 4:4:4 and monochrome crop single samples
    assert_eq!(size(&with(3, true)), (159, 159));
    assert_eq!(size(&with(0, true)), (159, 159));
    // field-coded pictures double the vertical unit
    assert_eq!(size(&with(1, false)), (158, 316));
    assert_eq!(size(&with(3, false)), (159, 318));
}

#[test]
fn separate_colour_planes_crop_like_monochrome() {
    let spec = SpsSpec {
        chroma_format_idc: 3,
        separate_colour_plane: true,
        crop: Some([1, 0, 1, 0]),
        ..SpsSpec::high(10, 10)
    };
    assert_eq!(size(&spec), (159, 159));
}

#[test]
fn baseline_defaults_to_420_crop_units() {
    let spec = SpsSpec {
        crop: Some([1, 0, 0, 1]),
        ..SpsSpec::baseline(10, 10)
    };
    assert_eq!(size(&spec), (158, 158));
}

#[test]
fn picture_order_count_type_one_is_skipped() {
    let spec = SpsSpec {
        poc_type: 1,
        ..SpsSpec::baseline(80, 45)
    };
    assert_eq!(size(&spec), (1280, 720));
}

#[test]
fn emulation_prevention_bytes_are_dropped() {
    let spec = SpsSpec {
        profile: 0,
        constraints: 0,
        level: 1,
        ..SpsSpec::baseline(80, 45)
    };
    let nal = spec.build();
    assert_eq!(nal[..5], [0x67, 0, 0, 3, 1]);
    let sps = parse_sps(&nal).unwrap();
    assert_eq!((sps.level_idc, sps.width, sps.height), (1, 1280, 720));
}

#[test]
fn sps_out_of_range_fields_are_refused() {
    let crop_all = SpsSpec {
        crop: Some([5, 5, 0, 0]),
        ..SpsSpec::baseline(1, 1)
    };
    assert_eq!(
        parse_sps(&crop_all.build()),
        Err(AvcError::SpsField("picture width"))
    );
    let too_wide = SpsSpec::baseline(4096, 1);
    assert_eq!(
        parse_sps(&too_wide.build()),
        Err(AvcError::SpsField("picture width"))
    );
    let poc = SpsSpec {
        poc_type: 3,
        ..SpsSpec::baseline(1, 1)
    };
    assert_eq!(
        parse_sps(&poc.build()),
        Err(AvcError::SpsField("pic_order_cnt_type"))
    );
    let chroma = SpsSpec {
        chroma_format_idc: 4,
        ..SpsSpec::high(1, 1)
    };
    assert_eq!(
        parse_sps(&chroma.build()),
        Err(AvcError::SpsField("chroma_format_idc"))
    );
    let delta = SpsSpec {
        scaling_lists: Some(vec![Some(vec![128])]),
        ..SpsSpec::high(1, 1)
    };
    assert_eq!(
        parse_sps(&delta.build()),
        Err(AvcError::SpsField("delta_scale"))
    );
}

#[test]
fn sps_must_be_an_sps_nal_unit() {
    let mut nal = SpsSpec::baseline(80, 45).build();
    nal[0] = 0x68;
    assert_eq!(parse_sps(&nal), Err(AvcError::NotSps));
    assert_eq!(parse_sps(&[]), Err(AvcError::SpsTruncated));
    let nal = SpsSpec::baseline(80, 45).build();
    assert_eq!(parse_sps(&nal[..6]), Err(AvcError::SpsTruncated));
}

#[test]
fn avcc_codec_string_comes_from_the_record_header() {
    let mut record = avcc(&SpsSpec::high(80, 45).build());
    record[3] = 0x1f;
    let config = parse_avcc(&record).unwrap();
    assert_eq!(config.codec(), "avc1.64001f");
    assert_eq!(config.nal_length_size, 4);
    assert_eq!((config.sps.width, config.sps.height), (1280, 720));
    assert_eq!(config.sps.level_idc, 40);
}

#[test]
fn avcc_nal_length_sizes() {
    let mut record = avcc(&SpsSpec::baseline(80, 45).build());
    for (byte, size) in [(0xfc, 1), (0xfd, 2), (0xff, 4)] {
        record[4] = byte;
        assert_eq!(parse_avcc(&record).unwrap().nal_length_size, size);
    }
    record[4] = 0xfe;
    assert_eq!(parse_avcc(&record), Err(AvcError::NalLengthSize(3)));
}

#[test]
fn avcc_header_errors() {
    let record = avcc(&SpsSpec::baseline(80, 45).build());
    let mut v0 = record.clone();
    v0[0] = 0;
    assert_eq!(parse_avcc(&v0), Err(AvcError::Version(0)));
    let mut no_sps = record.clone();
    no_sps[5] = 0xe0;
    assert_eq!(parse_avcc(&no_sps), Err(AvcError::NoSps));
    for len in 0..record.len() {
        assert_eq!(
            parse_avcc(&record[..len]),
            Err(AvcError::Truncated),
            "{len}"
        );
    }
    // a declared PPS past the end
    let mut pps = record.clone();
    let at = record.len() - 7;
    pps[at] = 2;
    assert_eq!(parse_avcc(&pps), Err(AvcError::Truncated));
}

#[test]
fn avcc_reads_every_parameter_set_and_ignores_trailing_bytes() {
    let sps = SpsSpec::baseline(80, 45).build();
    let mut record = vec![1, 0x42, 0xc0, 0x1f, 0xff, 0xe2];
    for s in [&sps, &sps] {
        record.extend_from_slice(&u16::try_from(s.len()).unwrap().to_be_bytes());
        record.extend_from_slice(s);
    }
    record.extend_from_slice(&[2, 0, 1, 0x68, 0, 1, 0x68]);
    record.extend_from_slice(&[0xfd, 0xf8, 0xf8, 0]);
    let config = parse_avcc(&record).unwrap();
    assert_eq!(config.codec(), "avc1.42c01f");
    record.truncate(record.len() - 6);
    assert_eq!(parse_avcc(&record), Err(AvcError::Truncated));
}

#[test]
fn avcc_over_the_bound_is_refused() {
    let mut record = avcc(&SpsSpec::baseline(80, 45).build());
    record.resize(MAX_CONFIG_LEN, 0);
    assert!(parse_avcc(&record).is_ok());
    record.push(0);
    assert_eq!(parse_avcc(&record), Err(AvcError::TooLong));
}

fn config() -> Config {
    Config {
        cases: 1024,
        rng_seed: RngSeed::Fixed(0xa7c0_5eed),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn arbitrary_avcc_never_panics(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = parse_avcc(&data);
    }

    #[test]
    fn arbitrary_sps_never_panics(
        profile in prop::sample::select(vec![66u8, 77, 100, 110, 122, 244]),
        data in proptest::collection::vec(any::<u8>(), 2..256),
    ) {
        let mut nal = vec![0x67, profile];
        nal.extend_from_slice(&data);
        if let Ok(sps) = parse_sps(&nal) {
            prop_assert!(sps.width > 0 && sps.height > 0);
        }
        let _ = parse_avcc(&avcc(&nal));
    }

    #[test]
    fn built_sps_round_trips(
        width_mbs in 1u32..=255,
        height_map_units in 1u32..=255,
        chroma_format_idc in 0u32..=3,
        frame_mbs_only: bool,
        high: bool,
    ) {
        let spec = SpsSpec {
            chroma_format_idc,
            frame_mbs_only,
            ..if high {
                SpsSpec::high(width_mbs, height_map_units)
            } else {
                SpsSpec::baseline(width_mbs, height_map_units)
            }
        };
        let fields = if frame_mbs_only { 1 } else { 2 };
        prop_assert_eq!(
            size(&spec),
            (
                u16::try_from(width_mbs * 16).unwrap(),
                u16::try_from(fields * height_map_units * 16).unwrap()
            )
        );
    }
}
