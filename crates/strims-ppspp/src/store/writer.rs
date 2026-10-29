// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Assigns consecutive bins to the chunks a local writer publishes.

use strims_binmap::Bin;

/// The next base bin of a local stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Writer {
    bin: Bin,
}

impl Writer {
    pub fn new() -> Writer {
        Writer::default()
    }

    /// The bin for the next chunk.
    pub fn next_bin(&mut self) -> Bin {
        let b = self.bin;
        self.bin += 2;
        b
    }

    /// The bin the next chunk takes.
    pub fn peek(&self) -> Bin {
        self.bin
    }

    /// Restarts at bin 0. Resetting the swarm is the caller's job.
    pub fn reset(&mut self) {
        self.bin = Bin(0);
    }
}
