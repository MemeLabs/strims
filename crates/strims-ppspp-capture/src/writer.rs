// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Encoding records into a byte sink.

use std::io::{self, Write};

use strims_base::time::Time;

use crate::format::{Header, KIND_HEADER, MAGIC, Record, VERSION, put_svarint, uvarint_array};

/// Writes one capture file. Each record is encoded into a scratch buffer
/// reused for the next, so writing allocates only while the scratch grows
/// to the largest record. Wrap an unbuffered sink in a
/// [`std::io::BufWriter`].
#[derive(Debug)]
pub struct Writer<W: Write> {
    w: W,
    scratch: Vec<u8>,
    last: Time,
    written: u64,
}

impl<W: Write> Writer<W> {
    /// Writes the magic, the version and `header`.
    pub fn new(mut w: W, header: &Header<'_>) -> io::Result<Writer<W>> {
        w.write_all(&MAGIC)?;
        w.write_all(&VERSION.to_be_bytes())?;
        let mut scratch = Vec::new();
        scratch.push(KIND_HEADER);
        put_svarint(&mut scratch, 0);
        header.encode_body(&mut scratch);
        let mut wr = Writer {
            w,
            scratch: Vec::new(),
            last: header.origin,
            written: (MAGIC.len() + 2) as u64,
        };
        wr.emit(&scratch, &[])?;
        wr.scratch = scratch;
        Ok(wr)
    }

    /// Writes `record` at `time`.
    pub fn write(&mut self, time: Time, record: &Record<'_>) -> io::Result<()> {
        let mut scratch = std::mem::take(&mut self.scratch);
        scratch.clear();
        scratch.push(record.kind());
        put_svarint(&mut scratch, (time - self.last).as_nanos());
        let tail = record.encode_body(&mut scratch);
        let res = self.emit(&scratch, tail);
        self.scratch = scratch;
        if res.is_ok() {
            self.last = time;
        }
        res
    }

    fn emit(&mut self, head: &[u8], tail: &[u8]) -> io::Result<()> {
        let len = head.len() + tail.len();
        let (prefix, n) = uvarint_array(len as u64);
        self.w.write_all(prefix.get(..n).unwrap_or_default())?;
        self.w.write_all(head)?;
        self.w.write_all(tail)?;
        self.written = self.written.saturating_add((n + len) as u64);
        Ok(())
    }

    /// The bytes written so far, header included.
    pub fn written(&self) -> u64 {
        self.written
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }

    pub fn get_ref(&self) -> &W {
        &self.w
    }

    pub fn into_inner(self) -> W {
        self.w
    }
}
