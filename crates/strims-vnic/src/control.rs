// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Link control: the bodies of frames on [`crate::ports::KEEPALIVE_PORT`].
//!
//! An empty body is a plain keepalive. Any other body is a list of records,
//! each a type byte and a big-endian `u32`:
//!
//! | type | record |
//! |---|---|
//! | 1 | probe: a nonce for the peer to echo |
//! | 2 | echo: the nonce of the newest probe read from the peer |
//! | 3 | filler: the rest of the body is a path train's padding |
//! | 4 | timed probe: a probe whose echo carries a gap |
//! | 5 | gap: after the echo of a timed probe, how long after the probe before it the peer read it, in microseconds |
//!
//! Readers stop at the first record of a type they don't know, or one cut
//! short, and at filler. [`strims_base::path`] has what the probes and
//! trains measure.

use crate::frame::FRAME_HEADER_LEN;
use crate::host::ports::KEEPALIVE_PORT;

/// A probe record's type.
pub const PROBE: u8 = 1;
/// An echo record's type.
pub const ECHO: u8 = 2;
/// A filler record's type: a body that starts with it is padding.
pub const FILLER: u8 = 3;
/// A timed probe record's type.
pub const TIMED_PROBE: u8 = 4;
/// A gap record's type.
pub const GAP: u8 = 5;
/// The bytes of one record.
pub const RECORD_LEN: usize = 5;
/// The largest control frame vnic writes, header included: an echo, its
/// gap and a probe.
pub const MAX_CONTROL_FRAME_LEN: usize = FRAME_HEADER_LEN + 3 * RECORD_LEN;

/// A control record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Record {
    Probe(u32),
    Echo(u32),
    TimedProbe(u32),
    /// Microseconds.
    Gap(u32),
}

/// The length of a control frame holding these records, header included.
pub fn frame_len(echo: bool, probe: bool) -> usize {
    FRAME_HEADER_LEN + RECORD_LEN * (usize::from(echo) + usize::from(probe))
}

/// Writes a control frame, header included, at the start of `buf`: the echo
/// first, then the probe. Returns its length, or `None` if `buf` is too
/// short.
pub fn encode(echo: Option<u32>, probe: Option<u32>, buf: &mut [u8]) -> Option<usize> {
    encode_records(&[echo.map(Record::Echo), probe.map(Record::Probe)], buf)
}

/// Writes a control frame holding `records` (the `Some` ones, in order),
/// header included, at the start of `buf`. Returns its length, or `None`
/// if `buf` is too short.
pub fn encode_records(records: &[Option<Record>], buf: &mut [u8]) -> Option<usize> {
    let len = FRAME_HEADER_LEN + RECORD_LEN * records.iter().flatten().count();
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
    for r in records.iter().flatten() {
        let (kind, nonce) = match *r {
            Record::Probe(n) => (PROBE, n),
            Record::Echo(n) => (ECHO, n),
            Record::TimedProbe(n) => (TIMED_PROBE, n),
            Record::Gap(us) => (GAP, us),
        };
        let (record, rest) = body.split_at_mut(RECORD_LEN);
        let (t, n) = record.split_at_mut(1);
        t.copy_from_slice(&[kind]);
        n.copy_from_slice(&nonce.to_be_bytes());
        body = rest;
    }
    Some(len)
}

/// Writes a control frame of `len` bytes, header included, that holds only
/// filler, at the start of `buf`. Returns its length, or `None` if `buf` is
/// too short or `len` can't hold the header and the filler's type.
pub fn encode_filler(len: usize, buf: &mut [u8]) -> Option<usize> {
    let body_len = u16::try_from(len.checked_sub(FRAME_HEADER_LEN + 1)? + 1).ok()?;
    let frame = buf.get_mut(..len)?;
    let (header, body) = frame.split_at_mut(FRAME_HEADER_LEN);
    header.copy_from_slice(
        &crate::frame::FrameHeader {
            port: KEEPALIVE_PORT,
            length: body_len,
        }
        .to_bytes(),
    );
    body.fill(0);
    if let Some(t) = body.first_mut() {
        *t = FILLER;
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
            TIMED_PROBE => Some(Record::TimedProbe(nonce)),
            GAP => Some(Record::Gap(nonce)),
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
        assert_eq!(n, frame_len(true, true));
        let (f, len) = Frame::decode(&buf).unwrap();
        assert_eq!((f.port, len), (KEEPALIVE_PORT, n));
        let got: Vec<Record> = records(f.body).collect();
        assert_eq!(got, vec![Record::Echo(7), Record::Probe(0x0102_0304)]);
    }

    #[test]
    fn an_echo_carries_its_gap_and_a_timed_probe() {
        let mut buf = [0u8; MAX_CONTROL_FRAME_LEN];
        let records = [
            Some(Record::Echo(7)),
            Some(Record::Gap(15_000)),
            Some(Record::TimedProbe(9)),
        ];
        let n = encode_records(&records, &mut buf).unwrap();
        assert_eq!(n, MAX_CONTROL_FRAME_LEN);
        let (f, _) = Frame::decode(&buf).unwrap();
        let got: Vec<Record> = super::records(f.body).collect();
        assert_eq!(got, records.into_iter().flatten().collect::<Vec<_>>());
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
