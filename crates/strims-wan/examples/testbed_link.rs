// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `run.sh`'s parser of a node's access link spec
//! ([`strims_wan::testbed::cli`]):
//!
//! ```text
//! testbed_link --link <spec> --up-mbit <m> --burst <bytes> --limit <bytes>
//!              [--peak-mbit <m>] [--base-mbit <m>]
//! ```
//!
//! prints what `hack/testbed/links.sh` builds the link from, as shell
//! assignments, or fails with what is wrong. `run.sh` builds it from its
//! own tree (or takes `LINK_BIN`), so a run whose nodes are another
//! commit's binaries still parses its specs as its `links.sh` reads them.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match strims_wan::testbed::cli(&args) {
        Ok(sh) => print!("{sh}"),
        Err(e) => {
            eprintln!("testbed_link: {e}");
            std::process::exit(2);
        }
    }
}
