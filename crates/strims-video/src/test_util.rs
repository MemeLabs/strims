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
