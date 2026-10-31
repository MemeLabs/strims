// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Builders of synthetic codec configs for the unit tests and `synth`.

// Test support: a panic is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

/// Writes bits most significant first.
#[derive(Default)]
pub(crate) struct BitWriter {
    bytes: Vec<u8>,
    cur: u8,
    len: u32,
}

impl BitWriter {
    pub(crate) fn new() -> Self {
        BitWriter::default()
    }

    pub(crate) fn bit(&mut self, b: bool) -> &mut Self {
        self.cur = (self.cur << 1) | u8::from(b);
        self.len += 1;
        if self.len == 8 {
            self.bytes.push(self.cur);
            self.cur = 0;
            self.len = 0;
        }
        self
    }

    /// The low `n` bits of `v`.
    pub(crate) fn bits(&mut self, n: u32, v: u64) -> &mut Self {
        for i in (0..n).rev() {
            self.bit((v >> i) & 1 == 1);
        }
        self
    }

    pub(crate) fn ue(&mut self, v: u32) -> &mut Self {
        let x = u64::from(v) + 1;
        let len = 64 - x.leading_zeros();
        self.bits(len - 1, 0).bits(len, x)
    }

    pub(crate) fn se(&mut self, v: i32) -> &mut Self {
        let v = i64::from(v);
        let k = if v > 0 { 2 * v - 1 } else { -2 * v };
        self.ue(u32::try_from(k).unwrap())
    }

    /// The bytes written, zero-padded to a byte boundary.
    pub(crate) fn finish(&mut self) -> Vec<u8> {
        while self.len != 0 {
            self.bit(false);
        }
        std::mem::take(&mut self.bytes)
    }

    /// The bytes written as an RBSP: a stop bit, then zero padding.
    pub(crate) fn rbsp(&mut self) -> Vec<u8> {
        self.bit(true);
        self.finish()
    }
}

/// A NAL unit of `header` and `rbsp`, with emulation prevention bytes.
pub(crate) fn nal(header: u8, rbsp: &[u8]) -> Vec<u8> {
    let mut out = vec![header];
    let mut zeros = 0;
    for &b in rbsp {
        if zeros >= 2 && b <= 3 {
            out.push(3);
            zeros = 0;
        }
        out.push(b);
        zeros = if b == 0 { zeros + 1 } else { 0 };
    }
    out
}

/// The fields of a synthetic SPS.
#[derive(Clone)]
pub(crate) struct SpsSpec {
    pub(crate) profile: u8,
    pub(crate) constraints: u8,
    pub(crate) level: u8,
    /// Written for high profiles only.
    pub(crate) chroma_format_idc: u32,
    pub(crate) separate_colour_plane: bool,
    /// Per list: None absent, Some(deltas) with the delta_scale values.
    pub(crate) scaling_lists: Option<Vec<Option<Vec<i32>>>>,
    pub(crate) poc_type: u32,
    pub(crate) width_mbs: u32,
    pub(crate) height_map_units: u32,
    pub(crate) frame_mbs_only: bool,
    /// Left, right, top, bottom.
    pub(crate) crop: Option<[u32; 4]>,
}

impl SpsSpec {
    pub(crate) fn baseline(width_mbs: u32, height_map_units: u32) -> Self {
        SpsSpec {
            profile: 66,
            constraints: 0xc0,
            level: 31,
            chroma_format_idc: 1,
            separate_colour_plane: false,
            scaling_lists: None,
            poc_type: 2,
            width_mbs,
            height_map_units,
            frame_mbs_only: true,
            crop: None,
        }
    }

    pub(crate) fn high(width_mbs: u32, height_map_units: u32) -> Self {
        SpsSpec {
            profile: 100,
            constraints: 0,
            level: 40,
            poc_type: 0,
            ..SpsSpec::baseline(width_mbs, height_map_units)
        }
    }

    /// The SPS NAL unit.
    pub(crate) fn build(&self) -> Vec<u8> {
        let high = [100, 110, 122, 244, 44, 83, 86, 118, 128, 138, 139, 134, 135];
        let mut w = BitWriter::new();
        w.bits(8, self.profile.into())
            .bits(8, self.constraints.into())
            .bits(8, self.level.into())
            .ue(0); // seq_parameter_set_id
        if high.contains(&self.profile) {
            w.ue(self.chroma_format_idc);
            if self.chroma_format_idc == 3 {
                w.bit(self.separate_colour_plane);
            }
            w.ue(0).ue(0).bit(false); // bit depths, qpprime_y_zero_transform_bypass
            match &self.scaling_lists {
                None => {
                    w.bit(false);
                }
                Some(lists) => {
                    w.bit(true);
                    for list in lists {
                        match list {
                            None => {
                                w.bit(false);
                            }
                            Some(deltas) => {
                                w.bit(true);
                                for &d in deltas {
                                    w.se(d);
                                }
                            }
                        }
                    }
                }
            }
        }
        w.ue(0); // log2_max_frame_num_minus4
        w.ue(self.poc_type);
        match self.poc_type {
            0 => {
                w.ue(2);
            }
            1 => {
                w.bit(false).se(-1).se(3).ue(2).se(5).se(-7);
            }
            _ => {}
        }
        w.ue(4).bit(false); // max_num_ref_frames, gaps_in_frame_num_value_allowed
        w.ue(self.width_mbs - 1).ue(self.height_map_units - 1);
        w.bit(self.frame_mbs_only);
        if !self.frame_mbs_only {
            w.bit(false);
        }
        w.bit(true); // direct_8x8_inference
        match self.crop {
            None => {
                w.bit(false);
            }
            Some([l, r, t, b]) => {
                w.bit(true).ue(l).ue(r).ue(t).ue(b);
            }
        }
        w.bit(false); // vui_parameters_present
        nal(0x67, &w.rbsp())
    }
}

/// An avcC record holding `sps` and one PPS, with 4-byte NAL lengths.
pub(crate) fn avcc(sps: &[u8]) -> Vec<u8> {
    let pps = [0x68, 0xce, 0x3c, 0x80];
    let mut out = vec![1, sps[1], sps[2], sps[3], 0xff, 0xe1];
    out.extend_from_slice(&u16::try_from(sps.len()).unwrap().to_be_bytes());
    out.extend_from_slice(sps);
    out.push(1);
    out.extend_from_slice(&u16::try_from(pps.len()).unwrap().to_be_bytes());
    out.extend_from_slice(&pps);
    out
}

/// The avcC of a 1280x720 High profile SPS: codec `avc1.640028`.
pub(crate) fn avcc_720p() -> Vec<u8> {
    avcc(&SpsSpec::high(80, 45).build())
}

/// The fields of a synthetic H.265 SPS ([`HevcSpsSpec::build`]): the ones
/// [`hevc::parse_sps`](crate::hevc::parse_sps) reads, up to the bit depths.
#[derive(Clone, Debug)]
pub(crate) struct HevcSpsSpec {
    /// 1 to 7.
    pub(crate) sub_layers: u32,
    /// Each sub-layer's profile and level present flags, for the first
    /// `sub_layers - 1`.
    pub(crate) sub_layer_present: Vec<(bool, bool)>,
    pub(crate) chroma_format_idc: u32,
    pub(crate) separate_colour_plane: bool,
    pub(crate) coded_width: u32,
    pub(crate) coded_height: u32,
    /// Conformance window offsets: left, right, top, bottom.
    pub(crate) window: Option<[u32; 4]>,
    pub(crate) bit_depth_luma: u32,
    pub(crate) bit_depth_chroma: u32,
}

impl HevcSpsSpec {
    /// A 4:2:0 8-bit SPS of one sub-layer.
    pub(crate) fn main(coded_width: u32, coded_height: u32) -> Self {
        HevcSpsSpec {
            sub_layers: 1,
            sub_layer_present: Vec::new(),
            chroma_format_idc: 1,
            separate_colour_plane: false,
            coded_width,
            coded_height,
            window: None,
            bit_depth_luma: 8,
            bit_depth_chroma: 8,
        }
    }

    /// The SPS NAL unit, its two header bytes included.
    pub(crate) fn build(&self) -> Vec<u8> {
        let mut w = BitWriter::new();
        w.bits(4, 0)
            .bits(3, u64::from(self.sub_layers - 1))
            .bit(true);
        // general profile space 0, Main tier, profile 1, compatibility
        // flags 1 and 2, constraint flags (progressive, frame only), level
        // 3.1
        w.bits(8, 1)
            .bits(32, 0x6000_0000)
            .bits(48, 0x9000_0000_0000)
            .bits(8, 93);
        let minus1 = self.sub_layers - 1;
        let present = |i: usize| self.sub_layer_present.get(i).copied().unwrap_or_default();
        for i in 0..minus1 as usize {
            let (profile, level) = present(i);
            w.bit(profile).bit(level);
        }
        if minus1 > 0 {
            for _ in minus1..8 {
                w.bits(2, 0);
            }
        }
        for i in 0..minus1 as usize {
            let (profile, level) = present(i);
            if profile {
                w.bits(44, 0).bits(44, 0);
            }
            if level {
                w.bits(8, 90);
            }
        }
        w.ue(0).ue(self.chroma_format_idc);
        if self.chroma_format_idc == 3 {
            w.bit(self.separate_colour_plane);
        }
        w.ue(self.coded_width).ue(self.coded_height);
        w.bit(self.window.is_some());
        if let Some(window) = self.window {
            for v in window {
                w.ue(v);
            }
        }
        w.ue(self.bit_depth_luma - 8).ue(self.bit_depth_chroma - 8);
        let mut out = nal(0x42, &w.rbsp());
        out.insert(1, 0x01);
        out
    }
}

/// An hvcC record of a Main profile stream at level 3.1, holding `sps` in
/// its SPS array between a VPS and a PPS, with 4-byte NAL lengths: codec
/// `hvc1.1.6.L93.90`.
pub(crate) fn hvcc(sps: &[u8]) -> Vec<u8> {
    let mut out = vec![
        1, 0x01, 0x60, 0, 0, 0, 0x90, 0, 0, 0, 0, 0, 93, 0xf0, 0, 0xfc, 0xfd, 0xf8, 0xf8, 0, 0,
        0x0f, 3,
    ];
    for (kind, nal) in [
        (32u8, &[0x40, 0x01, 0x0c][..]),
        (33, sps),
        (34, &[0x44, 0x01, 0xc0][..]),
    ] {
        out.push(0x80 | kind);
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&u16::try_from(nal.len()).unwrap().to_be_bytes());
        out.extend_from_slice(nal);
    }
    out
}

/// The hvcC of a 1280x720 Main profile SPS.
pub(crate) fn hvcc_720p() -> Vec<u8> {
    hvcc(&HevcSpsSpec::main(1280, 720).build())
}

/// The fields of a synthetic AV1 sequence header OBU's payload
/// ([`Av1SeqSpec::build`]): the ones
/// [`av1::parse_sequence_header`](crate::av1::parse_sequence_header)
/// reads.
#[derive(Clone, Debug)]
pub(crate) struct Av1SeqSpec {
    pub(crate) reduced_still_picture: bool,
    /// Timing info: equal picture intervals, and the decoder model's
    /// buffer delay length when it has one.
    pub(crate) timing: Option<(bool, Option<u32>)>,
    pub(crate) initial_display_delay: bool,
    /// Each operating point's level, whether the decoder model is present
    /// for it, and whether its initial display delay is.
    pub(crate) operating_points: Vec<(u32, bool, bool)>,
    pub(crate) width_bits: u32,
    pub(crate) height_bits: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Av1SeqSpec {
    /// A sequence header of one operating point at level 4.0.
    pub(crate) fn main(width: u32, height: u32) -> Self {
        Av1SeqSpec {
            reduced_still_picture: false,
            timing: None,
            initial_display_delay: false,
            operating_points: vec![(8, false, false)],
            width_bits: 16,
            height_bits: 16,
            width,
            height,
        }
    }

    /// The OBU's payload.
    pub(crate) fn build(&self) -> Vec<u8> {
        let mut w = BitWriter::new();
        w.bits(3, 0).bit(false).bit(self.reduced_still_picture);
        if self.reduced_still_picture {
            w.bits(5, 8);
        } else {
            w.bit(self.timing.is_some());
            let mut delay_bits = None;
            if let Some((equal, model)) = self.timing {
                w.bits(32, 1001).bits(32, 60_000).bit(equal);
                if equal {
                    w.ue(1);
                }
                w.bit(model.is_some());
                if let Some(bits) = model {
                    w.bits(5, u64::from(bits - 1)).bits(32, 1001).bits(10, 0);
                    delay_bits = Some(bits);
                }
            }
            w.bit(self.initial_display_delay);
            w.bits(5, (self.operating_points.len() - 1) as u64);
            for &(level, model, delay) in &self.operating_points {
                w.bits(12, 0).bits(5, u64::from(level));
                if level > 7 {
                    w.bit(false);
                }
                if let Some(bits) = delay_bits {
                    w.bit(model);
                    if model {
                        w.bits(bits, 0).bits(bits, 0).bit(false);
                    }
                }
                if self.initial_display_delay {
                    w.bit(delay);
                    if delay {
                        w.bits(4, 9);
                    }
                }
            }
        }
        w.bits(4, u64::from(self.width_bits - 1))
            .bits(4, u64::from(self.height_bits - 1))
            .bits(self.width_bits, u64::from(self.width - 1))
            .bits(self.height_bits, u64::from(self.height - 1));
        // frame_id_numbers_present_flag and on: not read
        w.bits(8, 0).finish()
    }
}

/// An OBU of `kind` with its size field, holding `payload`.
pub(crate) fn obu(kind: u8, payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() < 128);
    let mut out = vec![(kind << 3) | 0x02, payload.len() as u8];
    out.extend_from_slice(payload);
    out
}

/// An av1C record of a Main profile 8-bit 4:2:0 stream at level 4.0,
/// holding `sequence_header` as its config OBU: codec `av01.0.08M.08`.
pub(crate) fn av1c(sequence_header: &[u8]) -> Vec<u8> {
    let mut out = vec![0x81, 0x08, 0x0c, 0x00];
    out.extend(obu(1, sequence_header));
    out
}

/// The av1C of a 1280x720 sequence.
pub(crate) fn av1c_720p() -> Vec<u8> {
    av1c(&Av1SeqSpec::main(1280, 720).build())
}
