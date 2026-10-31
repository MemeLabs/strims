// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! CEA-608 caption byte pairs of channel 1 (CC1), as an encoder sends
//! them in field 1.

/// `b` with its odd parity bit, bit 7.
pub fn parity(b: u8) -> u8 {
    let b = b & 0x7f;
    if b.count_ones().is_multiple_of(2) {
        b | 0x80
    } else {
        b
    }
}

/// The byte pair `(a, b)`, each with its parity bit.
pub fn pair(a: u8, b: u8) -> [u8; 2] {
    [parity(a), parity(b)]
}

/// The pairs of a pop-on caption of `text` on row 15: resume caption
/// loading, erase non-displayed memory and the row's preamble address,
/// each control sent twice; the text, a space padding an odd length; then
/// end of caption, twice, which shows it.
pub fn pop_on(text: &str) -> Vec<[u8; 2]> {
    let mut out = Vec::new();
    for control in [(0x14, 0x20), (0x14, 0x2e), (0x14, 0x70)] {
        out.push(pair(control.0, control.1));
        out.push(pair(control.0, control.1));
    }
    let mut bytes = text.as_bytes().to_vec();
    if bytes.len() % 2 == 1 {
        bytes.push(b' ');
    }
    for &[a, b] in bytes.as_chunks::<2>().0 {
        out.push(pair(a, b));
    }
    out.push(pair(0x14, 0x2f));
    out.push(pair(0x14, 0x2f));
    out
}

/// The caption tuple of a field 1 pair.
pub fn field1(pair: [u8; 2]) -> crate::captions::CcTuple {
    [0xfc, pair[0], pair[1]]
}
