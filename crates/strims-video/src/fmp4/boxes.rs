// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Writing ISO BMFF boxes into a caller's buffer. A box starts with a
//! placeholder size, which [`end`] patches once its contents are written.

/// Bytes of a box header: a 32-bit size and the type.
pub(crate) const HEADER_LEN: usize = 8;
/// Bytes of a full box header: the box header, version and flags.
pub(crate) const FULL_HEADER_LEN: usize = 12;

/// Starts a box of type `fourcc`. Returns its offset in `out`, for [`end`].
pub(crate) fn start(out: &mut Vec<u8>, fourcc: &[u8; 4]) -> usize {
    let at = out.len();
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(fourcc);
    at
}

/// Starts a full box: [`start`], then the version and 24 bits of flags.
pub(crate) fn start_full(out: &mut Vec<u8>, fourcc: &[u8; 4], version: u8, flags: u32) -> usize {
    let at = start(out, fourcc);
    u32(out, (u32::from(version) << 24) | (flags & 0x00ff_ffff));
    at
}

/// Ends the box started at `at`: its size is the bytes written since. The
/// size field holds 32 bits; callers keep boxes below 4 GiB.
pub(crate) fn end(out: &mut [u8], at: usize) {
    let size = u32::try_from(out.len().saturating_sub(at)).unwrap_or(u32::MAX);
    if let Some(field) = out.get_mut(at..at.saturating_add(4)) {
        field.copy_from_slice(&size.to_be_bytes());
    }
}

pub(crate) fn u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_be_bytes());
}

pub(crate) fn u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}

pub(crate) fn i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_be_bytes());
}

pub(crate) fn u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_be_bytes());
}

pub(crate) fn i64(out: &mut Vec<u8>, v: i64) {
    out.extend_from_slice(&v.to_be_bytes());
}

pub(crate) fn zeros(out: &mut Vec<u8>, n: usize) {
    out.resize(out.len().saturating_add(n), 0);
}

/// The unity transformation matrix of `mvhd` and `tkhd`.
pub(crate) fn unity_matrix(out: &mut Vec<u8>) {
    for v in [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000] {
        u32(out, v);
    }
}
