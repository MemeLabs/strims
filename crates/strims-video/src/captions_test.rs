// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use proptest::prelude::*;

use super::*;

/// `nals` as a unit of 4-byte lengths.
fn unit(nals: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for n in nals {
        out.extend_from_slice(&(n.len() as u32).to_be_bytes());
        out.extend_from_slice(n);
    }
    out
}

/// The NAL units of a unit of 4-byte lengths.
fn nals(unit: &[u8]) -> Vec<Vec<u8>> {
    Nals::new(unit, 4).map(|n| n.unwrap().1.to_vec()).collect()
}

const AUD: &[u8] = &[0x09, 0xf0];
const SPS: &[u8] = &[0x67, 0x64, 0x00, 0x1f];
const PPS: &[u8] = &[0x68, 0xee, 0x3c, 0x80];
const IDR: &[u8] = &[0x65, 0x88, 0x84, 0x00];
const X264_SEI: &[u8] = &[0x06, 0x05, 0x02, 0xaa, 0xbb, 0x80];

fn f1(a: u8, b: u8) -> CcTuple {
    [0xfc, a, b]
}

/// An A/53 SEI as an encoder writes it, after a message of user data
/// whose bytes need emulation prevention, with every entry of the frame:
/// one field 1 pair, a field 2 pair not valid, 608 nulls in field 1, and
/// 708 data.
fn encoder_sei() -> Vec<u8> {
    let rbsp = [
        &[0x05, 20][..],
        &[0x11; 16],
        &[0x00, 0x00, 0x01, 0x02],
        &[0x04, 8 + 2 + 4 * 3 + 1],
        &A53_HEADER,
        &[0xc4, 0xff],
        &[0xfc, 0x94, 0x2c],
        &[0xf9, 0x80, 0x80],
        &[0xfc, 0x80, 0x80],
        &[0xfe, 0x00, 0x00],
        &[0xff],
        &[0x80],
    ]
    .concat();
    let mut nal = vec![0x06];
    let mut zeros = 0;
    for b in rbsp {
        if zeros >= 2 && b <= 3 {
            nal.push(3);
            zeros = 0;
        }
        nal.push(b);
        zeros = if b == 0 { zeros + 1 } else { 0 };
    }
    nal
}

#[test]
fn the_valid_tuples_that_carry_data_are_read_from_an_avc_units_sei() {
    let sei = encoder_sei();
    assert!(sei.windows(3).any(|w| w == [0, 0, 3]), "escaped");
    let u = unit(&[AUD, X264_SEI, &sei, IDR]);
    let mut got = Vec::new();
    NalFormat::avc(4).cc_tuples(&u, &mut got);
    assert_eq!(got, [f1(0x94, 0x2c), [0xfe, 0x00, 0x00]]);
    assert!(NalFormat::avc(4).has_captions(&u));
    assert!(!NalFormat::avc(4).has_captions(&unit(&[AUD, X264_SEI, IDR])));
}

#[test]
fn hevc_prefix_sei_carries_them_too() {
    let avc = encoder_sei();
    // A prefix SEI's two-byte header, then the same messages.
    let mut sei = vec![0x4e, 0x01];
    sei.extend_from_slice(&avc[1..]);
    // A suffix SEI (type 40) of the same messages is not read.
    let mut suffix = vec![0x50, 0x01];
    suffix.extend_from_slice(&avc[1..]);
    let idr = [0x26, 0x01, 0xaf];
    let u = unit(&[&sei, &suffix, &idr]);
    let hevc = NalFormat {
        hevc: true,
        length_size: 4,
    };
    let mut got = Vec::new();
    hevc.cc_tuples(&u, &mut got);
    assert_eq!(got, [f1(0x94, 0x2c), [0xfe, 0x00, 0x00]]);
    assert_eq!(
        hevc.with_cc_sei(&u, &got, &mut Vec::new()),
        Err(CaptionError::Malformed)
    );
}

#[test]
fn captions_go_in_an_sei_before_the_first_slice_and_read_back() {
    let tuples: Vec<CcTuple> = (1..41u8).map(|i| f1(0x80 | i, 0x00)).collect();
    let u = unit(&[AUD, SPS, PPS, X264_SEI, IDR, IDR]);
    let mut out = Vec::new();
    NalFormat::avc(4)
        .with_cc_sei(&u, &tuples, &mut out)
        .unwrap();
    let n = nals(&out);
    assert_eq!(n.len(), 7);
    assert_eq!((&n[..4], &n[5..]), (&nals(&u)[..4], &nals(&u)[4..]));
    assert_eq!(n[4][0], 0x06);
    let mut got = Vec::new();
    NalFormat::avc(4).cc_tuples(&out, &mut got);
    assert_eq!(got, tuples, "in two messages, of 31 and 9");

    let mut same = Vec::new();
    NalFormat::avc(4).with_cc_sei(&u, &[], &mut same).unwrap();
    assert_eq!(same, u);
}

#[test]
fn a_two_byte_length_size_takes_them_and_a_one_byte_one_refuses_a_long_sei() {
    let two = |n: &[u8]| [&(n.len() as u16).to_be_bytes()[..], n].concat();
    let u = [two(AUD), two(IDR)].concat();
    let tuples = [f1(0x94, 0x20); 31];
    let mut out = Vec::new();
    NalFormat::avc(2)
        .with_cc_sei(&u, &tuples, &mut out)
        .unwrap();
    let mut got = Vec::new();
    NalFormat::avc(2).cc_tuples(&out, &mut got);
    assert_eq!(got, tuples);

    let one = [&[IDR.len() as u8][..], IDR].concat();
    let mut out = Vec::new();
    assert_eq!(
        NalFormat::avc(1).with_cc_sei(&one, &[f1(0x94, 0x20); 93], &mut out),
        Err(CaptionError::TooLong)
    );
    assert!(out.is_empty());
}

#[test]
fn malformed_units_give_what_came_before_and_take_no_captions() {
    let sei = encoder_sei();
    let mut u = unit(&[&sei, IDR]);
    u.extend_from_slice(&[0, 0, 0, 9, 1]);
    let mut got = Vec::new();
    NalFormat::avc(4).cc_tuples(&u, &mut got);
    assert_eq!(got.len(), 2, "the tuples before the bad length");
    let mut out = Vec::new();
    assert_eq!(
        NalFormat::avc(4).with_cc_sei(&u, &got, &mut out),
        Err(CaptionError::Malformed)
    );
    assert_eq!(
        NalFormat::avc(4).with_cc_sei(&unit(&[AUD, SPS]), &got, &mut out),
        Err(CaptionError::Malformed),
        "no slice"
    );
    assert!(out.is_empty());
}

#[test]
fn a_unit_gives_at_most_its_bound_of_tuples() {
    let tuples = vec![f1(0x94, 0x20); 31];
    let mut u = Vec::new();
    for _ in 0..3 {
        NalFormat::avc(4)
            .with_cc_sei(&unit(&[IDR]), &tuples, &mut u)
            .unwrap();
    }
    let mut got = Vec::new();
    NalFormat::avc(4).cc_tuples(&u, &mut got);
    assert_eq!(got.len(), MAX_UNIT_TUPLES);
}

/// A rendition at half the stream's rate takes two frames' tuples a
/// frame, and one at its rate one; a frame takes those since its
/// rendition's last, in presentation order.
#[test]
fn renditions_take_the_streams_tuples_up_to_each_frame_in_presentation_order() {
    let mut c = CaptionCarry::new(2);
    let frame = 1500; // 60 fps at 90 kHz
    // Decode order of I P B: PTS 0, 2, 1 frames.
    for (i, pts) in [0, 2, 1, 4, 3, 5].into_iter().enumerate() {
        c.source(pts * frame, 90_000, &[f1(0x80 | pts as u8, i as u8)]);
    }
    let byte = |t: &[CcTuple]| t.iter().map(|t| t[1] & 0x7f).collect::<Vec<_>>();
    assert_eq!(byte(&c.take(0, 0, 90_000)), [0]);
    assert_eq!(byte(&c.take(1, 0, 90_000)), [0]);
    assert_eq!(byte(&c.take(0, frame, 90_000)), [1]);
    assert_eq!(byte(&c.take(1, 2 * frame, 90_000)), [1, 2], "half rate");
    assert_eq!(byte(&c.take(0, 2 * frame, 90_000)), [2]);
    assert_eq!(
        byte(&c.take(0, 2 * frame, 90_000)),
        [] as [u8; 0],
        "a repeat"
    );
    // In milliseconds, the timescale of another track.
    assert_eq!(byte(&c.take(0, 4 * 1000 / 60 + 1, 1000)), [3, 4]);
    assert_eq!(c.frames.len(), 3, "what rendition 1 hasn't taken");
    c.restart();
    assert_eq!(c.frames.len(), 0);
    c.source(9 * frame, 90_000, &[f1(0x89, 0)]);
    assert_eq!(byte(&c.take(0, 10 * frame, 90_000)), [9], "after a restart");
    assert!(
        c.take(5, 10 * frame, 90_000).is_empty(),
        "no such rendition"
    );
}

#[test]
fn a_carry_holds_its_bound_of_frames() {
    let mut c = CaptionCarry::new(1);
    for i in 0..(MAX_CARRIED_FRAMES as i64 + 10) {
        c.source(i, 1000, &[f1(0x94, 0x20)]);
    }
    assert_eq!(c.frames.len(), MAX_CARRIED_FRAMES);
    assert_eq!(c.take(0, i64::MAX, 1000).len(), MAX_CARRIED_FRAMES);
    assert_eq!(c.frames.len(), 0);
}

proptest! {
    #[test]
    fn hostile_units_never_panic(unit in proptest::collection::vec(any::<u8>(), 0..512), size in 0u8..6) {
        for f in [NalFormat::avc(size), NalFormat { hevc: true, length_size: size }] {
            let mut t = Vec::new();
            f.cc_tuples(&unit, &mut t);
            prop_assert!(t.len() <= MAX_UNIT_TUPLES);
            let mut out = Vec::new();
            let _ = f.with_cc_sei(&unit, &[f1(0x94, 0x20)], &mut out);
        }
    }
}
