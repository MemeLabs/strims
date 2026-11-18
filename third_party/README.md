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

## sctp-proto

[sctp-proto](https://github.com/algesten/sctp-proto) 0.10.8 as published on
crates.io (upstream commit `da1ac299d949c17680bc8070f334010ab91d5b1c`),
under its MIT or Apache-2.0 licence (`sctp-proto/LICENSE-MIT`,
`sctp-proto/LICENSE-APACHE`). str0m's data channels run on it.

Every time the crate reads is its caller's but one: `Endpoint::connect`
gave a new client association `Instant::now()`, and its T1-init timer
started from it. strims-webrtc maps a node's clock to instants from one
anchor (`crates/strims-webrtc/src/clock.rs`), so under a simulated clock
that timer, and the wake-up it asks for, fell wherever the wall clock had
got to: a strims-sim run with WebRTC replayed differently from one process
to the next. The patch gives `connect` no time (`src/endpoint/mod.rs`): a
client association it makes starts its T1-init timer at the first time it
is given, by `handle_timeout`, `poll_transmit` or an inbound packet
(`Association::arm_t1_init` in `src/association/mod.rs`), and closing it
drops a timer it never started. str0m gives it that time as it inits SCTP
at the SDP negotiation: its connection's last input or timeout, in the
poll that follows. strims-webrtc brings the connection to the
negotiation's time first (`sync_rtc` in
`crates/strims-webrtc/src/endpoint.rs`), so the timer runs from the
answer however long the offer waited. `Association::new` takes the time
as an `Option`, and the tests' calls give it one.

Its tests (`cargo test` in a copy outside this workspace) pass.
