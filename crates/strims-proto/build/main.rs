// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Build script for `strims-proto`.
//!
//! 1. Parses `schema/**/*.proto` and the crate's own `proto/**/*.proto` (the
//!    `strims.rpc` envelope) with protox (pure Rust; no `protoc`).
//! 2. Generates prost message types, one file per proto package.
//! 3. Generates `StableEncode`/`Signable` impls (the signable encoding, see
//!    `src/stable.rs`) from the same descriptors.
//! 4. Generates the module tree: package `strims.vnic.v1` becomes
//!    `strims_proto::vnic::v1`.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::{env, fs};

mod stable;
mod tree;

type BoxError = Box<dyn Error>;

fn main() -> Result<(), BoxError> {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("CARGO_MANIFEST_DIR unset")?);
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR unset")?);
    let schema = manifest_dir.join("../../schema");
    let local = manifest_dir.join("proto");

    println!("cargo:rerun-if-changed={}", schema.display());
    println!("cargo:rerun-if-changed={}", local.display());
    println!("cargo:rerun-if-changed=build");

    let mut protos = Vec::new();
    collect_protos(&schema, &schema, &mut protos)?;
    collect_protos(&local, &local, &mut protos)?;
    protos.sort();

    let fds = protox::Compiler::new([&schema, &local])?
        .include_source_info(true)
        // prost-build's message graph needs the imported well-known types.
        .include_imports(true)
        .open_files(&protos)?
        .file_descriptor_set();

    let mut config = prost_build::Config::new();
    configure(&mut config);
    config.out_dir(&out_dir);
    config.compile_fds(fds.clone())?;

    // Our generators cover only the `strims.*` schema; prost maps the
    // imported well-known types to `prost_types` and primitives.
    let mut schema_fds = fds;
    schema_fds
        .file
        .retain(|f| f.package().starts_with("strims."));
    stable::generate(&schema_fds, &out_dir)?;
    tree::generate(&schema_fds, &out_dir)?;
    Ok(())
}

/// prost-build settings for every generated type.
fn configure(config: &mut prost_build::Config) {
    // TODO: RPC service codegen plugs in here with
    // `config.service_generator(...)`.
    // Deterministic iteration and encoding order; std HashMap is banned in
    // core crates (clippy.toml).
    config.btree_map(["."]);
    // `prost::Name` impls (full names and type URLs) for Any and RPC.
    // Stored and transmitted `Any` values carry this domain, so it must not
    // change.
    config.enable_type_names();
    config.type_name_domain(["."], "type.googleapis.com");
}

/// Collects `.proto` paths relative to `root`, using `/` separators.
fn collect_protos(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), BoxError> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_protos(root, &path, out)?;
        } else if path.extension().is_some_and(|e| e == "proto") {
            let rel = path.strip_prefix(root)?;
            let parts: Vec<_> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            out.push(parts.join("/"));
        }
    }
    Ok(())
}
