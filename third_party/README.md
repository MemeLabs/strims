# Vendored crates

Crates the workspace builds from this directory instead of crates.io, through
`[patch.crates-io]` in the root `Cargo.toml`. They aren't workspace members:
the workspace's lints, formatting and tests don't run over them.

## turmoil

[turmoil](https://github.com/tokio-rs/turmoil) 0.7.2 as published on
crates.io (upstream commit `4709a564ed8908b2dec57bbe993757ec0d25fa6f`),
under its MIT licence (`turmoil/LICENSE`). strims-sim runs on it.

turmoil registers a link between every pair of hosts, and stepped every one
of them for every host each step. The patch makes a step's cost follow the
messages in flight instead (`src/top.rs`), delivering in the same order:

- The topology keeps the positions of the links with messages in flight,
  and per host those of its links with messages ready for it, in link
  order. A tick visits the first, a delivery to a host the second.
- Links share the topology's time rather than each keeping a copy.
- `Sim::busy_links` iterates the links with messages in flight, in the
  order `Sim::links` yields links, `LinkIter::index` gives a link's
  position in that order, and a `LinkIter` knows its length.
- `src/net/tcp/stream.rs` calls `AtomicUsize::try_update` where it called
  `fetch_update`, its deprecated name.

Its tests (`cargo test` in a copy outside this workspace, with the
crate's `.cargo/config.toml`, which sets `--cfg tokio_unstable`) pass, and
`src/sim.rs` adds `busy_links_are_the_links_with_messages_in_flight`.
