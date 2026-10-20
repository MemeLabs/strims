// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Decoding a dump, which is untrusted input: every record costs at most
//! [`MAX_RECORD`] bytes of buffer and a pass over them.

use std::io::{self, Read};

use strims_base::time::{Duration, Time};

use crate::format::{
    Clock, Dec, Header, KIND_HEADER, MAGIC, MAX_RECORD, RawHeader, Record, VERSION,
};

/// Why a dump can't be read further, or why one record was skipped.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a capture")]
    BadMagic,
    #[error("unsupported version {0}")]
    UnsupportedVersion(u16),
    #[error("bad header: {0}")]
    BadHeader(&'static str),
    #[error("record at {offset} is truncated")]
    Truncated { offset: u64 },
    #[error("record at {offset} has length {len}, over the limit")]
    Oversized { offset: u64, len: u64 },
    /// One record is unreadable; the reader goes on with the next.
    #[error("record at {offset} (kind {kind}) is malformed: {what}")]
    Malformed {
        offset: u64,
        kind: u8,
        what: &'static str,
    },
    #[error("read: {0}")]
    Io(#[from] io::Error),
}

impl Error {
    /// Whether the reader can go on after it.
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Error::Malformed { .. })
    }
}

/// A file's header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedHeader {
    pub version: u16,
    pub clock: Clock,
    pub origin: Time,
    pub file: u64,
    pub node: Vec<u8>,
    /// The label, with invalid UTF-8 replaced.
    pub label: String,
}

impl OwnedHeader {
    /// The header as the writer takes it.
    pub fn as_header(&self) -> Header<'_> {
        Header {
            clock: self.clock,
            origin: self.origin,
            file: self.file,
            node: &self.node,
            label: &self.label,
        }
    }
}

/// A record, where it starts and its time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry<'a> {
    /// The byte offset of the record's length.
    pub offset: u64,
    pub time: Time,
    pub record: Record<'a>,
}

/// Reads one capture file record by record.
#[derive(Debug)]
pub struct Reader<R: Read> {
    r: R,
    header: OwnedHeader,
    buf: Vec<u8>,
    time: Time,
    offset: u64,
    done: bool,
}

impl<R: Read> Reader<R> {
    /// Reads the magic, the version and the header. Wrap an unbuffered
    /// source in a [`std::io::BufReader`]: lengths are read a byte at a
    /// time.
    pub fn new(mut r: R) -> Result<Reader<R>, Error> {
        let mut magic = [0u8; 10];
        read_exact(&mut r, &mut magic).map_err(|e| match e {
            Some(e) => Error::Io(e),
            None => Error::BadMagic,
        })?;
        let [m @ .., v0, v1] = magic;
        if m != MAGIC {
            return Err(Error::BadMagic);
        }
        let version = u16::from_be_bytes([v0, v1]);
        if version != VERSION {
            return Err(Error::UnsupportedVersion(version));
        }
        let mut rd = Reader {
            r,
            header: OwnedHeader {
                version,
                clock: Clock::Other(0),
                origin: Time::EPOCH,
                file: 0,
                node: Vec::new(),
                label: String::new(),
            },
            buf: Vec::new(),
            time: Time::EPOCH,
            offset: magic.len() as u64,
            done: false,
        };
        if !rd.frame()? {
            return Err(Error::BadHeader("missing"));
        }
        if rd.buf.first() != Some(&KIND_HEADER) {
            return Err(Error::BadHeader("first record isn't a header"));
        }
        let mut d = Dec::new(&rd.buf);
        let parsed = d
            .u8()
            .and_then(|_| d.svarint())
            .and_then(|_| RawHeader::decode(d.rest_slice()));
        let h = parsed.map_err(Error::BadHeader)?;
        rd.header = OwnedHeader {
            version,
            clock: h.clock,
            origin: h.origin,
            file: h.file,
            node: h.node.to_vec(),
            label: String::from_utf8_lossy(h.label).into_owned(),
        };
        rd.time = h.origin;
        Ok(rd)
    }

    pub fn header(&self) -> &OwnedHeader {
        &self.header
    }

    /// The next record. `None` at the end of the dump, or after an error
    /// that leaves the rest unframed (anything but [`Error::Malformed`]).
    pub fn next_record(&mut self) -> Option<Result<Entry<'_>, Error>> {
        if self.done {
            return None;
        }
        let offset = self.offset;
        match self.frame() {
            Ok(true) => {}
            Ok(false) => {
                self.done = true;
                return None;
            }
            Err(e) => {
                self.done = true;
                return Some(Err(e));
            }
        }
        let mut d = Dec::new(&self.buf);
        let kind = self.buf.first().copied().unwrap_or(KIND_HEADER);
        let dt = match d.u8().and_then(|_| d.svarint()) {
            Ok(dt) => dt,
            Err(what) => return Some(Err(Error::Malformed { offset, kind, what })),
        };
        self.time += Duration::from_nanos(dt);
        let time = self.time;
        Some(match Record::decode(kind, d.rest_slice()) {
            Ok(record) => Ok(Entry {
                offset,
                time,
                record,
            }),
            Err(what) => Err(Error::Malformed { offset, kind, what }),
        })
    }

    /// Reads the next record's bytes after its length into `buf`; `false`
    /// at a clean end.
    fn frame(&mut self) -> Result<bool, Error> {
        let offset = self.offset;
        let (len, n) = match read_uvarint(&mut self.r) {
            Ok(Some(v)) => v,
            Ok(None) => return Ok(false),
            Err(LenError::Io(e)) => return Err(Error::Io(e)),
            Err(LenError::Eof) => return Err(Error::Truncated { offset }),
            Err(LenError::Overlong) => {
                return Err(Error::Oversized {
                    offset,
                    len: u64::MAX,
                });
            }
        };
        if len > MAX_RECORD as u64 {
            return Err(Error::Oversized { offset, len });
        }
        self.buf.clear();
        self.buf.resize(len as usize, 0);
        read_exact(&mut self.r, &mut self.buf).map_err(|e| match e {
            Some(e) => Error::Io(e),
            None => Error::Truncated { offset },
        })?;
        self.offset = offset.saturating_add(n as u64).saturating_add(len);
        Ok(true)
    }
}

/// Fills `buf`; `Err(None)` at the end of the input.
fn read_exact(r: &mut impl Read, buf: &mut [u8]) -> Result<(), Option<io::Error>> {
    r.read_exact(buf).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            None
        } else {
            Some(e)
        }
    })
}

enum LenError {
    Io(io::Error),
    /// The input ends inside the varint.
    Eof,
    /// More than 10 bytes, or past `u64`.
    Overlong,
}

/// A uvarint and its length; `Ok(None)` at a clean end before its first
/// byte.
fn read_uvarint(r: &mut impl Read) -> Result<Option<(u64, usize)>, LenError> {
    let mut x: u64 = 0;
    for i in 0..10 {
        let mut b = [0u8; 1];
        match read_exact(r, &mut b) {
            Ok(()) => {}
            Err(None) if i == 0 => return Ok(None),
            Err(None) => return Err(LenError::Eof),
            Err(Some(e)) => return Err(LenError::Io(e)),
        }
        let [b] = b;
        if b < 0x80 {
            if i == 9 && b > 1 {
                return Err(LenError::Overlong);
            }
            return Ok(Some((x | u64::from(b) << (7 * i), i + 1)));
        }
        x |= u64::from(b & 0x7f) << (7 * i);
    }
    Err(LenError::Overlong)
}
