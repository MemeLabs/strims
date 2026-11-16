// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Link control: the bodies of frames on [`crate::ports::KEEPALIVE_PORT`].
//!
//! An empty body is a plain keepalive. Any other body is a list of records,
//! each a type byte and a big-endian `u32` nonce:
//!
//! | type | record |
//! |---|---|
//! | 1 | probe: a nonce for the peer to echo |
//! | 2 | echo: the nonce of the newest probe read from the peer |
//!
//! Readers stop at the first record of a type they don't know, or one cut
//! short. [`strims_base::path`] has what the probes measure.

use crate::frame::FRAME_HEADER_LEN;
use crate::host::ports::KEEPALIVE_PORT;

/// A probe record's type.
pub const PROBE: u8 = 1;
/// An echo record's type.
pub const ECHO: u8 = 2;
/// The bytes of one record.
pub const RECORD_LEN: usize = 5;
/// The largest control frame vnic writes, header included: an echo and a
/// probe.
pub const MAX_CONTROL_FRAME_LEN: usize = FRAME_HEADER_LEN + 2 * RECORD_LEN;

/// A control record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Record {
    Probe(u32),
    Echo(u32),
}

/// The length of a control frame holding these records, header included.
pub fn frame_len(echo: bool, probe: bool) -> usize {
    FRAME_HEADER_LEN + RECORD_LEN * (usize::from(echo) + usize::from(probe))
}

/// Writes a control frame, header included, at the start of `buf`: the echo
/// first, then the probe. Returns its length, or `None` if `buf` is too
/// short.
pub fn encode(echo: Option<u32>, probe: Option<u32>, buf: &mut [u8]) -> Option<usize> {
    let len = frame_len(echo.is_some(), probe.is_some());
    let frame = buf.get_mut(..len)?;
    let (header, mut body) = frame.split_at_mut(FRAME_HEADER_LEN);
    let body_len = u16::try_from(len - FRAME_HEADER_LEN).ok()?;
    header.copy_from_slice(
        &crate::frame::FrameHeader {
            port: KEEPALIVE_PORT,
            length: body_len,
        }
        .to_bytes(),
    );
    let records = [echo.map(|n| (ECHO, n)), probe.map(|n| (PROBE, n))];
    for (kind, nonce) in records.into_iter().flatten() {
        let (record, rest) = body.split_at_mut(RECORD_LEN);
        let (t, n) = record.split_at_mut(1);
        t.copy_from_slice(&[kind]);
        n.copy_from_slice(&nonce.to_be_bytes());
        body = rest;
    }
    Some(len)
}

/// The records of a control frame's body, up to the first unknown or cut
/// short one.
pub fn records(body: &[u8]) -> impl Iterator<Item = Record> + '_ {
    body.chunks(RECORD_LEN).map_while(|r| {
        let (&kind, nonce) = r.split_first()?;
        let nonce = u32::from_be_bytes(nonce.try_into().ok()?);
        match kind {
            PROBE => Some(Record::Probe(nonce)),
            ECHO => Some(Record::Echo(nonce)),
            _ => None,
        }
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::frame::Frame;

    #[test]
    fn a_frame_holds_its_echo_then_its_probe() {
        let mut buf = [0u8; MAX_CONTROL_FRAME_LEN];
        let n = encode(Some(7), Some(0x0102_0304), &mut buf).unwrap();
        assert_eq!(n, MAX_CONTROL_FRAME_LEN);
        let (f, len) = Frame::decode(&buf).unwrap();
        assert_eq!((f.port, len), (KEEPALIVE_PORT, n));
        let got: Vec<Record> = records(f.body).collect();
        assert_eq!(got, vec![Record::Echo(7), Record::Probe(0x0102_0304)]);
    }

    #[test]
    fn a_frame_without_records_is_a_keepalive() {
        let mut buf = [0u8; MAX_CONTROL_FRAME_LEN];
        assert_eq!(encode(None, None, &mut buf), Some(FRAME_HEADER_LEN));
        assert_eq!(buf[..FRAME_HEADER_LEN], [0xff, 0xff, 0, 0]);
        assert_eq!(encode(Some(1), None, &mut buf[..8]), None);
    }

    #[test]
    fn readers_stop_at_unknown_and_short_records() {
        let body = [1, 0, 0, 0, 9, 3, 0, 0, 0, 1, 2, 0, 0, 0, 2];
        assert_eq!(records(&body).collect::<Vec<_>>(), vec![Record::Probe(9)]);
        let short = [2, 0, 0, 0, 5, 1, 0, 0];
        assert_eq!(records(&short).collect::<Vec<_>>(), vec![Record::Echo(5)]);
        assert_eq!(records(&[]).count(), 0);
    }
}
