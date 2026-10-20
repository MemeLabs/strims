// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! ppspp captures (`strims_ppspp_capture`) into files.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::config::PpsppCapture;

/// The files a capture keeps: opening one deletes the one this many
/// before it.
pub const CAPTURE_FILES: u64 = 8;

/// Where a node's ppspp capture goes and how much of it stays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureConfig {
    /// Files are `<host id>-<n>.ppcap` in it, `n` counting from 0.
    pub dir: PathBuf,
    /// The bytes the kept files hold at most, split evenly between
    /// [`CAPTURE_FILES`] files. A file passes its share by what one drain
    /// of the runner writes.
    pub max_bytes: u64,
    /// Capture the runner's decisions, not only its frames.
    pub tap: bool,
}

impl CaptureConfig {
    /// The node option that writes this capture.
    pub fn node_capture(&self) -> PpsppCapture {
        let dir = self.dir.clone();
        PpsppCapture {
            open: Arc::new(move |node, index| open(&dir, node, index)),
            max_file_bytes: (self.max_bytes / CAPTURE_FILES).max(1),
            tap: self.tap,
        }
    }
}

/// Opens file `index` of `node`'s capture and deletes the one it retires.
fn open(dir: &Path, node: &[u8], index: u64) -> io::Result<Box<dyn Write + Send>> {
    let name = hex::encode(node);
    std::fs::create_dir_all(dir)?;
    if let Some(old) = index.checked_sub(CAPTURE_FILES) {
        match std::fs::remove_file(dir.join(format!("{name}-{old}.ppcap"))) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    }
    let f = File::create(dir.join(format!("{name}-{index}.ppcap")))?;
    Ok(Box::new(BufWriter::with_capacity(64 * 1024, f)))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn a_capture_keeps_its_last_files() {
        let dir = std::env::temp_dir().join(format!("strims-capture-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let c = CaptureConfig {
            dir: dir.clone(),
            max_bytes: 8 * 1024,
            tap: true,
        }
        .node_capture();
        assert_eq!(c.max_file_bytes, 1024);
        for i in 0..CAPTURE_FILES + 3 {
            let mut f = (c.open)(&[0xab, 0xcd], i).unwrap();
            f.write_all(&[i as u8; 10]).unwrap();
        }
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        let mut want: Vec<String> = (3..CAPTURE_FILES + 3)
            .map(|i| format!("abcd-{i}.ppcap"))
            .collect();
        want.sort();
        assert_eq!(names, want);
        assert_eq!(std::fs::read(dir.join("abcd-10.ppcap")).unwrap(), [10; 10]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
