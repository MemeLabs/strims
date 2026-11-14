# Vendored crates

Crates the workspace builds from this directory instead of crates.io, through
`[patch.crates-io]` in the root `Cargo.toml`. They aren't workspace members:
the workspace's lints, formatting and tests don't run over them.

## turmoil

[turmoil](https://github.com/tokio-rs/turmoil) 0.7.2 as published on
crates.io (upstream commit `4709a564ed8908b2dec57bbe993757ec0d25fa6f`),
under its MIT licence (`turmoil/LICENSE`). strims-sim runs on it.

Unchanged from upstream.
