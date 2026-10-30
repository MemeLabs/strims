// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;

/// The header every WebCodecs Opus encoder tested gives for stereo at
/// 48 kHz: version 1, pre-skip 312, gain 0, family 0.
const STEREO: [u8; 19] = [
    b'O', b'p', b'u', b's', b'H', b'e', b'a', b'd', 1, 2, 0x38, 0x01, 0x80, 0xbb, 0, 0, 0, 0, 0,
];

/// A family-1 5.1 header: 4 streams, 2 coupled, Vorbis order.
fn surround() -> Vec<u8> {
    let mut h = STEREO.to_vec();
    h[9] = 6;
    h[18] = 1;
    h.extend_from_slice(&[4, 2, 0, 4, 1, 2, 3, 5]);
    h
}

#[test]
fn encoder_stereo_header() {
    assert_eq!(
        parse_opus_head(&STEREO).unwrap(),
        OpusHead {
            version: 1,
            channels: 2,
            pre_skip: 312,
            input_sample_rate: 48_000,
            output_gain: 0,
            mapping_family: 0,
            streams: 1,
            coupled: 1,
            mapping: &[],
        }
    );
    let mut mono = STEREO;
    mono[9] = 1;
    let h = parse_opus_head(&mono).unwrap();
    assert_eq!((h.channels, h.streams, h.coupled), (1, 1, 0));
}

#[test]
fn little_endian_fields() {
    let mut h = STEREO;
    h[10..12].copy_from_slice(&[0x02, 0x01]);
    h[12..16].copy_from_slice(&44_100u32.to_le_bytes());
    h[16..18].copy_from_slice(&(-256i16).to_le_bytes());
    let p = parse_opus_head(&h).unwrap();
    assert_eq!(
        (p.pre_skip, p.input_sample_rate, p.output_gain),
        (0x0102, 44_100, -256)
    );
}

#[test]
fn a_later_minor_version_may_append_fields() {
    let mut h = STEREO.to_vec();
    h[8] = 0x0f;
    h.extend_from_slice(&[0xaa, 0xbb]);
    assert_eq!(parse_opus_head(&h).unwrap().version, 0x0f);
    let s = surround();
    let mut longer = s.clone();
    longer.push(0xcc);
    assert_eq!(
        parse_opus_head(&longer).unwrap().mapping,
        parse_opus_head(&s).unwrap().mapping
    );
}

#[test]
fn a_mapping_table_follows_families_1_and_255() {
    let h = surround();
    let p = parse_opus_head(&h).unwrap();
    assert_eq!(
        (
            p.channels,
            p.mapping_family,
            p.streams,
            p.coupled,
            p.mapping
        ),
        (6, 1, 4, 2, &[0, 4, 1, 2, 3, 5][..])
    );
    let mut any = h.clone();
    any[18] = 255;
    // A silent channel.
    any[26] = 255;
    assert_eq!(parse_opus_head(&any).unwrap().mapping[5], 255);
}

#[test]
fn malformed_headers_are_refused() {
    let with = |f: &dyn Fn(&mut Vec<u8>)| {
        let mut h = STEREO.to_vec();
        f(&mut h);
        parse_opus_head(&h).map(|_| ())
    };
    type Edit = Box<dyn Fn(&mut Vec<u8>)>;
    let cases: Vec<(Edit, OpusError)> = vec![
        (Box::new(|h| h.truncate(18)), OpusError::Truncated),
        (Box::new(|h| h[0] = b'o'), OpusError::Magic),
        (Box::new(|h| h[8] = 0x10), OpusError::Version(0x10)),
        (Box::new(|h| h[9] = 0), OpusError::NoChannels),
        (
            Box::new(|h| h[9] = 3),
            OpusError::Channels {
                family: 0,
                channels: 3,
            },
        ),
        (Box::new(|h| h[18] = 2), OpusError::Family(2)),
        (Box::new(|h| h[18] = 3), OpusError::Family(3)),
        // Family 1 without its table.
        (Box::new(|h| h[18] = 1), OpusError::Truncated),
        (
            Box::new(|h| h.resize(MAX_CONFIG_LEN + 1, 0)),
            OpusError::TooLong,
        ),
    ];
    for (i, (f, e)) in cases.into_iter().enumerate() {
        assert_eq!(with(&*f), Err(e), "case {i}");
    }
    let table = |f: &dyn Fn(&mut Vec<u8>)| {
        let mut h = surround();
        f(&mut h);
        parse_opus_head(&h).map(|_| ())
    };
    assert_eq!(table(&|h| h.truncate(26)), Err(OpusError::Truncated));
    assert_eq!(table(&|h| h[19] = 0), Err(OpusError::NoStreams));
    assert_eq!(
        table(&|h| h[20] = 5),
        Err(OpusError::Coupled {
            streams: 4,
            coupled: 5
        })
    );
    assert_eq!(
        table(&|h| h[22] = 6),
        Err(OpusError::Mapping {
            channel: 1,
            index: 6
        })
    );
    assert_eq!(
        table(&|h| h[9] = 9),
        Err(OpusError::Channels {
            family: 1,
            channels: 9
        })
    );
    // 255 streams of which 255 coupled decode to 510 channels.
    let mut wide = STEREO.to_vec();
    wide[18] = 255;
    wide[9] = 1;
    wide.extend_from_slice(&[255, 255, 0]);
    assert_eq!(
        parse_opus_head(&wide),
        Err(OpusError::Coupled {
            streams: 255,
            coupled: 255
        })
    );
}

#[test]
fn dops_carries_the_header_big_endian_at_version_0() {
    let mut out = vec![0xee];
    write_dops(&STEREO, &mut out).unwrap();
    assert_eq!(
        out,
        [0xee, 0, 2, 0x01, 0x38, 0, 0, 0xbb, 0x80, 0, 0, 0],
        "version, channels, pre-skip, input rate, gain, family"
    );
    let mut out = Vec::new();
    write_dops(&surround(), &mut out).unwrap();
    assert_eq!(
        out,
        [
            0, 6, 0x01, 0x38, 0, 0, 0xbb, 0x80, 0, 0, 1, 4, 2, 0, 4, 1, 2, 3, 5
        ]
    );
    let mut out = vec![1];
    assert_eq!(write_dops(b"OpusTags", &mut out), Err(OpusError::Truncated));
    assert_eq!(out, [1]);
}

#[test]
fn dops_declares_48_khz_input_whatever_the_encoder_took() {
    let mut h = STEREO;
    h[12..16].copy_from_slice(&44_100u32.to_le_bytes());
    assert_eq!(parse_opus_head(&h).unwrap().input_sample_rate, 44_100);
    let mut out = Vec::new();
    write_dops(&h, &mut out).unwrap();
    assert_eq!(out[4..8], 48_000u32.to_be_bytes());
}
