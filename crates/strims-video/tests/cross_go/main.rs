// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Checks against Go's implementations, through the Go programs under
//! `crates/strims-video/golden` (`go run -tags golden`). Skipped when `go`
//! isn't on PATH.

#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::process::Command;

mod fmp4;
mod rtmp;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn go_available() -> bool {
    match Command::new("go").arg("version").output() {
        Ok(o) if o.status.success() => true,
        _ => {
            eprintln!("skipping: `go` is not on PATH");
            false
        }
    }
}
