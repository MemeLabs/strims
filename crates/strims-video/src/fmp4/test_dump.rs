// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A full reading of media segments for the tests: top-level boxes, and
//! each fragment's tracks, `trun` entries and sample bytes.

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use crate::fmp4::parse::{Boxes, FourCc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SampleDump {
    pub(crate) duration: u32,
    pub(crate) size: u32,
    pub(crate) flags: Option<u32>,
    pub(crate) composition_offset: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrafDump {
    pub(crate) track: u32,
    pub(crate) tfhd_flags: u32,
    pub(crate) tfdt: u64,
    pub(crate) trun_flags: u32,
    pub(crate) samples: Vec<SampleDump>,
    /// The bytes the data offset and sizes point at.
    pub(crate) data: Vec<u8>,
}

impl TrafDump {
    pub(crate) fn durations(&self) -> Vec<u32> {
        self.samples.iter().map(|s| s.duration).collect()
    }

    pub(crate) fn duration(&self) -> u64 {
        self.samples.iter().map(|s| u64::from(s.duration)).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoofDump {
    pub(crate) sequence: u32,
    pub(crate) trafs: Vec<TrafDump>,
}

/// A `prft`'s fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PrftDump {
    pub(crate) version: u8,
    pub(crate) flags: u32,
    pub(crate) track: u32,
    pub(crate) ntp: u64,
    pub(crate) media_time: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediaDump {
    /// Top-level boxes: type and size.
    pub(crate) boxes: Vec<(String, usize)>,
    pub(crate) moofs: Vec<MoofDump>,
    /// Each `moof`'s `prft`, the box right before it, if any.
    pub(crate) prfts: Vec<Option<PrftDump>>,
}

impl MediaDump {
    /// A track's samples over every fragment, as one `traf`: the first
    /// `traf`'s header fields, every `trun` entry and every sample's
    /// bytes.
    pub(crate) fn traf(&self, track: u32) -> TrafDump {
        let mut trafs = self
            .moofs
            .iter()
            .flat_map(|m| &m.trafs)
            .filter(|t| t.track == track);
        let mut merged = trafs
            .next()
            .unwrap_or_else(|| panic!("no traf of track {track}"))
            .clone();
        for t in trafs {
            merged.samples.extend_from_slice(&t.samples);
            merged.data.extend_from_slice(&t.data);
        }
        merged
    }

    /// Each fragment's `traf`s of `track`.
    pub(crate) fn trafs(&self, track: u32) -> Vec<&TrafDump> {
        self.moofs
            .iter()
            .flat_map(|m| &m.trafs)
            .filter(|t| t.track == track)
            .collect()
    }
}

fn be32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(b[at..at + 4].try_into().unwrap())
}

fn fourcc(f: FourCc) -> String {
    String::from_utf8_lossy(&f.0).into_owned()
}

/// Reads a media segment of 32-bit sized boxes; panics on anything
/// malformed.
pub(crate) fn dump_media(media: &[u8]) -> MediaDump {
    let mut boxes = Vec::new();
    let mut moofs = Vec::new();
    let mut prfts = Vec::new();
    let mut prft = None;
    let mut at = 0;
    for b in Boxes::new(media) {
        let (t, content) = b.unwrap();
        let size = content.len() + 8;
        boxes.push((fourcc(t), size));
        match &t.0 {
            b"moof" => {
                moofs.push(dump_moof(media, at, content));
                prfts.push(prft.take());
            }
            b"prft" => {
                assert_eq!(size, 32, "prft size");
                prft = Some(PrftDump {
                    version: content[0],
                    flags: be32(content, 0) & 0xff_ffff,
                    track: be32(content, 4),
                    ntp: u64::from_be_bytes(content[8..16].try_into().unwrap()),
                    media_time: u64::from_be_bytes(content[16..24].try_into().unwrap()),
                });
            }
            _ => prft = None,
        }
        at += size;
    }
    MediaDump {
        boxes,
        moofs,
        prfts,
    }
}

fn dump_moof(media: &[u8], moof_at: usize, moof: &[u8]) -> MoofDump {
    let mut sequence = None;
    let mut trafs = Vec::new();
    for b in Boxes::new(moof) {
        let (t, content) = b.unwrap();
        match &t.0 {
            b"mfhd" => sequence = Some(be32(content, 4)),
            b"traf" => trafs.push(dump_traf(media, moof_at, content)),
            other => panic!("unexpected {} in moof", other.escape_ascii()),
        }
    }
    MoofDump {
        sequence: sequence.unwrap(),
        trafs,
    }
}

fn dump_traf(media: &[u8], moof_at: usize, traf: &[u8]) -> TrafDump {
    let mut dump = TrafDump {
        track: 0,
        tfhd_flags: 0,
        tfdt: 0,
        trun_flags: 0,
        samples: Vec::new(),
        data: Vec::new(),
    };
    for b in Boxes::new(traf) {
        let (t, c) = b.unwrap();
        match &t.0 {
            b"tfhd" => {
                dump.tfhd_flags = be32(c, 0) & 0xff_ffff;
                dump.track = be32(c, 4);
            }
            b"tfdt" => {
                assert_eq!(c[0], 1, "tfdt version");
                dump.tfdt = u64::from_be_bytes(c[4..12].try_into().unwrap());
            }
            b"trun" => {
                assert_eq!(c[0], 1, "trun version");
                let flags = be32(c, 0) & 0xff_ffff;
                dump.trun_flags = flags;
                let count = be32(c, 4) as usize;
                let mut at = 8;
                let mut offset = None;
                if flags & 0x01 != 0 {
                    offset = Some(be32(c, at) as i32);
                    at += 4;
                }
                assert_eq!(flags & 0x04, 0, "first-sample flags");
                for _ in 0..count {
                    let mut field = |present: bool| {
                        present.then(|| {
                            let v = be32(c, at);
                            at += 4;
                            v
                        })
                    };
                    let duration = field(flags & 0x100 != 0).unwrap_or(0);
                    let size = field(flags & 0x200 != 0).unwrap_or(0);
                    let sample_flags = field(flags & 0x400 != 0);
                    let cto = field(flags & 0x800 != 0).map(|v| v as i32);
                    dump.samples.push(SampleDump {
                        duration,
                        size,
                        flags: sample_flags,
                        composition_offset: cto,
                    });
                }
                assert_eq!(at, c.len(), "trun length");
                let start = moof_at + offset.unwrap() as usize;
                let len: usize = dump.samples.iter().map(|s| s.size as usize).sum();
                dump.data = media[start..start + len].to_vec();
            }
            other => panic!("unexpected {} in traf", other.escape_ascii()),
        }
    }
    dump
}
