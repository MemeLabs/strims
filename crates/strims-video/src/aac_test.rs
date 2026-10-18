// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

use super::*;
use crate::test_util::BitWriter;

/// An ASC of the given fields, GASpecificConfig after the object type.
fn asc(f: impl FnOnce(&mut BitWriter)) -> Vec<u8> {
    let mut w = BitWriter::new();
    f(&mut w);
    w.finish()
}

#[test]
fn lc_44100_stereo() {
    let a = parse_asc(&[0x12, 0x10]).unwrap();
    assert_eq!(
        a,
        Asc {
            object_type: 2,
            core_object_type: 2,
            core_sample_rate: 44_100,
            sample_rate: 44_100,
            channel_config: 2,
            channels: 2,
            frame_samples: 1024,
        }
    );
    assert_eq!(a.codec(), "mp4a.40.2");
}

#[test]
fn lc_48000_stereo() {
    let a = parse_asc(&[0x11, 0x90]).unwrap();
    assert_eq!(
        (a.sample_rate, a.channels, a.frame_samples),
        (48_000, 2, 1024)
    );
    assert_eq!(a.codec(), "mp4a.40.2");
}

#[test]
fn explicit_he_aac_runs_at_the_sbr_rate() {
    // AOT 5, core 24 kHz, stereo, SBR 48 kHz, core AOT 2, GASpecificConfig 0
    let a = parse_asc(&[0x2b, 0x11, 0x88, 0x00]).unwrap();
    assert_eq!(
        a,
        Asc {
            object_type: 5,
            core_object_type: 2,
            core_sample_rate: 24_000,
            sample_rate: 48_000,
            channel_config: 2,
            channels: 2,
            frame_samples: 2048,
        }
    );
    assert_eq!(a.codec(), "mp4a.40.5");
}

#[test]
fn explicit_he_aac_v2_outputs_stereo() {
    let data = asc(|w| {
        w.bits(5, 29)
            .bits(4, 6)
            .bits(4, 1)
            .bits(4, 3)
            .bits(5, 2)
            .bits(3, 0);
    });
    let a = parse_asc(&data).unwrap();
    assert_eq!((a.object_type, a.core_object_type), (29, 2));
    assert_eq!((a.channel_config, a.channels), (1, 2));
    assert_eq!((a.sample_rate, a.frame_samples), (48_000, 2048));
    assert_eq!(a.codec(), "mp4a.40.29");
}

#[test]
fn downsampled_sbr_keeps_the_core_frame() {
    let data = asc(|w| {
        w.bits(5, 5)
            .bits(4, 3)
            .bits(4, 2)
            .bits(4, 3)
            .bits(5, 2)
            .bits(3, 0);
    });
    let a = parse_asc(&data).unwrap();
    assert_eq!((a.sample_rate, a.frame_samples), (48_000, 1024));
}

#[test]
fn sbr_rate_must_be_the_core_rate_or_twice_it() {
    let data = asc(|w| {
        w.bits(5, 5)
            .bits(4, 8)
            .bits(4, 2)
            .bits(4, 3)
            .bits(5, 2)
            .bits(3, 0);
    });
    assert_eq!(
        parse_asc(&data),
        Err(AacError::SbrRate {
            core: 16_000,
            extension: 48_000
        })
    );
}

#[test]
fn escaped_frequency_is_read_from_24_bits() {
    let data = asc(|w| {
        w.bits(5, 2)
            .bits(4, 15)
            .bits(24, 44_100)
            .bits(4, 2)
            .bits(3, 0);
    });
    let a = parse_asc(&data).unwrap();
    assert_eq!(
        (a.sample_rate, a.channels, a.frame_samples),
        (44_100, 2, 1024)
    );
    let sbr = asc(|w| {
        w.bits(5, 5)
            .bits(4, 15)
            .bits(24, 22_050)
            .bits(4, 2)
            .bits(4, 15)
            .bits(24, 44_100)
            .bits(5, 2)
            .bits(3, 0);
    });
    let a = parse_asc(&sbr).unwrap();
    assert_eq!((a.core_sample_rate, a.sample_rate), (22_050, 44_100));
    let zero = asc(|w| {
        w.bits(5, 2).bits(4, 15).bits(24, 0).bits(4, 2).bits(3, 0);
    });
    assert_eq!(parse_asc(&zero), Err(AacError::ZeroRate));
}

#[test]
fn frame_length_flag_selects_960_samples() {
    let data = asc(|w| {
        w.bits(5, 2).bits(4, 3).bits(4, 2).bit(true).bits(2, 0);
    });
    assert_eq!(parse_asc(&data).unwrap().frame_samples, 960);
    let sbr = asc(|w| {
        w.bits(5, 5)
            .bits(4, 6)
            .bits(4, 2)
            .bits(4, 3)
            .bits(5, 2)
            .bit(true);
    });
    assert_eq!(parse_asc(&sbr).unwrap().frame_samples, 1920);
}

#[test]
fn escaped_object_type_reads_six_more_bits() {
    // 31 escapes to 32 + 10: USAC
    let data = asc(|w| {
        w.bits(5, 31).bits(6, 10).bits(4, 3).bits(4, 2).bits(3, 0);
    });
    assert_eq!(parse_asc(&data), Err(AacError::ObjectType(42)));
    // under SBR, the core object type escapes the same way
    let sbr = asc(|w| {
        w.bits(5, 5)
            .bits(4, 6)
            .bits(4, 2)
            .bits(4, 3)
            .bits(5, 31)
            .bits(6, 7);
    });
    assert_eq!(parse_asc(&sbr), Err(AacError::ObjectType(39)));
}

#[test]
fn other_object_types_are_refused() {
    for aot in [0u8, 6, 17, 23, 30] {
        let data = asc(|w| {
            w.bits(5, aot.into()).bits(4, 3).bits(4, 2).bits(3, 0);
        });
        assert_eq!(parse_asc(&data), Err(AacError::ObjectType(aot)), "{aot}");
    }
    for core in [1u8, 3, 4] {
        let data = asc(|w| {
            w.bits(5, core.into()).bits(4, 3).bits(4, 2).bits(3, 0);
        });
        assert_eq!(parse_asc(&data).unwrap().core_object_type, core);
    }
}

#[test]
fn channel_configurations() {
    let with = |config: u64| {
        parse_asc(&asc(|w| {
            w.bits(5, 2).bits(4, 3).bits(4, config).bits(3, 0);
        }))
    };
    assert_eq!(with(0).unwrap().channels, 0);
    assert_eq!(with(1).unwrap().channels, 1);
    assert_eq!(with(6).unwrap().channels, 6);
    assert_eq!(with(7).unwrap().channels, 8);
    assert_eq!(with(11).unwrap().channels, 7);
    assert_eq!(with(13).unwrap().channels, 24);
    for reserved in [8u8, 9, 10, 15] {
        assert_eq!(
            with(reserved.into()),
            Err(AacError::ChannelConfig(reserved))
        );
    }
}

#[test]
fn reserved_frequency_index_is_refused() {
    let data = asc(|w| {
        w.bits(5, 2).bits(4, 13).bits(4, 2).bits(3, 0);
    });
    assert_eq!(parse_asc(&data), Err(AacError::FrequencyIndex(13)));
}

#[test]
fn truncated_and_oversized_configs_are_refused() {
    assert_eq!(parse_asc(&[]), Err(AacError::Truncated));
    assert_eq!(parse_asc(&[0x12]), Err(AacError::Truncated));
    assert_eq!(parse_asc(&[0x2b, 0x11]), Err(AacError::Truncated));
    let mut long = vec![0x12, 0x10];
    long.resize(MAX_CONFIG_LEN, 0);
    assert!(parse_asc(&long).is_ok());
    long.push(0);
    assert_eq!(parse_asc(&long), Err(AacError::TooLong));
}

#[test]
fn esds_for_lc_stereo() {
    let mut out = Vec::new();
    write_esds(&[0x12, 0x10], &mut out).unwrap();
    #[rustfmt::skip]
    let expected = [
        // FullBox version 0, flags 0
        0x00, 0x00, 0x00, 0x00,
        // ES_Descriptor, 25 bytes: 3 + 19 + 3
        0x03, 0x19,
        //   ES_ID 0, flags 0
        0x00, 0x00, 0x00,
        //   DecoderConfigDescriptor, 17 bytes: 13 + 4
        0x04, 0x11,
        //     objectTypeIndication MPEG-4 audio; streamType 5 << 2 | reserved 1
        0x40, 0x15,
        //     bufferSizeDB (3 bytes), maxBitrate, avgBitrate: 0
        0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        //     DecoderSpecificInfo, 2 bytes: the ASC
        0x05, 0x02, 0x12, 0x10,
        //   SLConfigDescriptor, 1 byte: predefined 2
        0x06, 0x01, 0x02,
    ];
    assert_eq!(out, expected);
}

#[test]
fn esds_sizes_over_127_take_more_bytes() {
    let asc: Vec<u8> = (0..200u8).collect();
    let mut out = vec![0xaa];
    write_esds(&asc, &mut out).unwrap();
    let out = &out[1..];
    assert_eq!(out.len(), 4 + 3 + 225);
    // 225 = 1 * 128 + 0x61
    assert_eq!(out[4..7], [0x03, 0x81, 0x61]);
    // 216 = 13 + 3 + 200 = 1 * 128 + 0x58
    assert_eq!(out[10..13], [0x04, 0x81, 0x58]);
    // 200 = 1 * 128 + 0x48
    assert_eq!(out[26..29], [0x05, 0x81, 0x48]);
    assert_eq!(out[29..229], asc[..]);
    assert_eq!(out[229..], [0x06, 0x01, 0x02]);
}

/// A descriptor's tag, content and the bytes after it, checking its size
/// takes the fewest bytes.
fn descriptor(b: &[u8]) -> (u8, &[u8], &[u8]) {
    let tag = b[0];
    let mut size = 0usize;
    let mut i = 1;
    loop {
        let group = b[i];
        assert!(i > 1 || group != 0x80, "size has a leading zero group");
        size = (size << 7) | usize::from(group & 0x7f);
        i += 1;
        if group & 0x80 == 0 {
            break;
        }
    }
    assert!(i <= 5);
    (tag, &b[i..i + size], &b[i + size..])
}

#[test]
fn esds_descriptor_sizes_nest() {
    for len in [0, 1, 127, 128, 16_383, 16_384, MAX_CONFIG_LEN] {
        let asc = vec![0x5a; len];
        let mut out = Vec::new();
        write_esds(&asc, &mut out).unwrap();
        assert_eq!(out[..4], [0; 4]);
        let (tag, es, rest) = descriptor(&out[4..]);
        assert_eq!((tag, rest.len()), (ES_DESCRIPTOR, 0));
        let (tag, dcd, rest) = descriptor(&es[3..]);
        assert_eq!(tag, DECODER_CONFIG_DESCRIPTOR);
        assert_eq!(rest, [SL_CONFIG_DESCRIPTOR, 1, SL_MP4]);
        let (tag, dsi, rest) = descriptor(&dcd[13..]);
        assert_eq!((tag, dsi, rest.len()), (DECODER_SPECIFIC_INFO, &asc[..], 0));
    }
    let mut out = Vec::new();
    assert_eq!(
        write_esds(&vec![0; MAX_CONFIG_LEN + 1], &mut out),
        Err(AacError::TooLong)
    );
    assert!(out.is_empty());
}

fn config() -> Config {
    Config {
        cases: 1024,
        rng_seed: RngSeed::Fixed(0xaac0_5eed),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn arbitrary_asc_never_panics(data in proptest::collection::vec(any::<u8>(), 0..64)) {
        if let Ok(a) = parse_asc(&data) {
            prop_assert!(a.sample_rate > 0);
            prop_assert!([960, 1024, 1920, 2048].contains(&a.frame_samples));
            prop_assert!(a.codec().starts_with("mp4a.40."));
        }
    }
}
