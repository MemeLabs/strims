# Conventions

Rules for every crate in `crates/`. `clippy.toml` and the workspace lints
enforce the mechanical parts.

## Documentation

- **The code is the authority on behaviour.** Don't keep parallel specs,
  design logs or status files in markdown; they drift.
- Crate- and module-level `//!` docs give purpose, contracts and invariants.
  Item docs give usage.
- Comments state non-obvious constraints and invariants, next to the code
  they constrain. A comment must not:
  - describe remote behaviour (what other nodes or implementations do);
  - narrate what the code does;
  - recount bugs, fixes, history or migrations;
  - use contrastive framing ("unlike…", "instead of…", "rather than…");
  - mention sessions, agents or reasoning steps.
- Name regression tests after the behaviour they protect.
- Results and measurements belong in test output, not in files.
- `crates/PLAN.md` is the only planning document. It holds forward-looking
  work and open questions, nothing about what's already done.

## Architecture: sans-IO core

Core crates (`strims-base`, `-binmap`, `-chunkstream`, `-proto`, `-crypto`,
`-qos`, `-kademlia`, `-mpc`, `-ppspp`, `-vnic`, `-vpn`, `-webrtc`) are
synchronous, single-threaded state machines: no async, no tokio, no threads,
no locks and no I/O. Drivers (`strims-node`, `strims-wasm`) own sockets,
timers and tasks. Both run `strims-node-core`, which composes the core
crates and the app layer over a driver interface; it uses only tokio's
`sync` and `macros` features, which build on wasm32.

Shape:

```rust
fn handle_input(&mut self, now: Time, from: Key, data: &[u8]) -> Result<(), Error>;
fn poll_transmit(&mut self, now: Time, to: Key, buf: &mut [u8]) -> Option<usize>; // caller-owned buffer
fn poll_timeout(&self) -> Option<Time>;
fn handle_timeout(&mut self, now: Time);
fn poll_event(&mut self) -> Option<Event>;
```

- Egress is pull-based: the driver asks for bytes when a link can write.
- The owner holds objects in `SlotMap`/`Vec` and passes keys. No
  `Rc<RefCell<_>>` graphs.
- Core crates don't call into each other's state machines; the driver
  composes them.
- Core crates use `strims-proto` without its `services` feature, which
  declares the generated RPC interfaces.

## Architecture: app layer

App-layer crates (`strims-rpc`, then the services on it) are async, so the
native and browser drivers can run them:

- Protocol state lives in sans-IO state machines under the core rules;
  the async code around them stays thin.
- The async code uses only `futures-core`, `futures-util` and
  `futures-sink`: no tokio, threads, timers or OS access. Drivers own the
  executor, and the browser's futures can't be `Send`, so app-layer futures
  aren't either. Spawn them as local tasks.
- Deadlines belong to callers, who drop a future to cancel it.
- Storage goes through `strims-kv`. Backends that touch the OS
  (`strims-kv-redb`) are native-only and belong to drivers.
- A connection may share one `Rc<RefCell<_>>` between its handles and the
  future that drives it. A borrow never spans an `.await`, a wake, or a
  call into user code (including drops of user values).
- Handlers run inside their connection's future: they must not block or
  panic.

## Determinism

- Time is `strims_base::time::{Time, Duration}` and is always passed in.
  `Duration` is signed, and all arithmetic saturates.
- Randomness comes from a seeded `strims_base::rng::DetRng` owned by the state
  machine. Derive child rngs with `fork`, and build third-party keys from bytes
  drawn from it.
- No std `HashMap`/`HashSet`. Use `BTreeMap`, `IndexMap` or `slotmap`.
- No global mutable state. Metrics are plain structs exposed by accessor.
- Same seed, same inputs, same outputs. Every simulation asserts it.
- Driver code that must touch the OS (the clock, sockets) allows the lint at
  the call site with a comment. Every `tokio::select!` is `biased;`.

## Hostile peers

Hostile peers are in the threat model. Any peer may send arbitrary bytes,
floods, or well-formed but malicious messages.

- Peer input never panics. Use `get()` and checked arithmetic, and return
  `Result`. `indexing_slicing`, `unwrap_used`, `expect_used` and `panic` warn;
  allow them locally only for internal invariants, with an
  `// invariant: ...` comment.
- Work and memory per message are bounded. Every collection that peer input
  can grow has an explicit bound, documented where it's declared.
- One peer's behaviour must not stall service to others.
- Protocols tolerate Byzantine peers and assume a majority of cooperative
  ones. A decision that ranks or trusts peers rests on what this node
  measures or on an aggregate of other nodes' observations, never on a
  peer's claim about itself.
- `unsafe` is forbidden.

## Memory

- Chunk storage is preallocated when a swarm is created. Hot paths don't
  allocate per chunk or per message.
- Decoders return borrowed views into the input. Encoders write into
  caller-supplied buffers.

## Formats

No deployed network constrains the wire or store formats. Golden vectors
(`golden/` in a crate, replayed by its `tests/golden*.rs` through
`strims-golden`) pin them, so a format can't change by accident. A
deliberate change updates the vectors in the same commit:
`STRIMS_GOLDEN=update cargo test -p <crate> --test <test>` rewrites them
from the code (under `cargo test`, so the tests of a file update it in one
process), and a plain run must pass afterwards. In code, express a
format requirement as the local constraint it imposes, e.g. "port 20 must
stay unhandled by the v2 protocol".

Messages between peers evolve add-only:

- A change adds fields, oneof members and enum values under new tags. A
  removed field's tag and name are `reserved`, and no tag is reused.
- Receivers ignore fields they don't know. A oneof member they don't know
  reads as unset, and an enum value they don't know keeps its number; to
  the receiver both are input it can't use.
- A field whose meaning a receiver can't safely ignore needs a version or
  capability both sides agreed on first.

A signed object travels as an envelope (`strims_crypto::signed`): an
encoded body in `bytes body` and its signature in `bytes signature`. The
signature covers those bytes, and whoever stores or forwards the object
keeps them, so a body's unknown fields survive any relay. Nothing
re-encodes a body it verified or forwards; what must survive a relay
belongs in the body. Every ed25519 signature goes through
`strims_crypto::sign` with the `Domain` of what it signs, and a new kind of
signed object adds a domain. Only `strims-crypto` depends on
`ed25519-dalek`.

Stored records (`strims-dao`) and the UI's frontend API are same-version
and keep no unknown fields. A store opens only at its `STORE_VERSION`, and
replication refuses a peer at another, so a change to a stored record's
encoding or meaning bumps it. The UI is built with its node.

A change under `schema/` regenerates the UI's `src/apis` and infra's
`pkg/apis` with `hack/protogen.sh` in the same commit.

## Testing

1. Unit tests per crate.
2. In-crate deterministic simulations for protocol behaviour: an event-queue
   simulator, seeded latency and loss, and a virtual clock.
3. Whole-node deterministic simulation with turmoil in `strims-sim`. Failures
   print a one-line seed-replay command.

Both kinds of simulation can run on real-world paths from `strims-wan`:
city-pair latency, access link rates, jitter and loss.

## Checks

`hack/check.sh` runs the checks below side by side and logs each to
`target/check/`; `hack/check.sh --full` adds the browser suites and the
release simulations. From the repo root:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --features strims-node/turmoil -- -D warnings`
- `cargo nextest run --workspace` and `cargo test --workspace --doc`:
  [cargo-nextest](https://nexte.st) runs every test in a process of its
  own, as many at once as there are cores, and skips doctests.
  `.config/nextest.toml` sets each heavy test's share of the cores.
- `cargo check --workspace --exclude strims-node --exclude strims-sim --exclude strims-kv-redb --exclude strims-testworld --exclude strims-svc --target wasm32-unknown-unknown`
- `cargo clippy -p strims-node-core -p strims-wasm --target wasm32-unknown-unknown --all-targets --features strims-wasm/e2e -- -D warnings`
- `GECKODRIVER=<path to geckodriver> cargo test -p strims-wasm --target wasm32-unknown-unknown`
  runs strims-wasm's tests in a headless Firefox worker. It needs
  `wasm-bindgen-cli` at the crate's `wasm-bindgen` pin, whose
  `wasm-bindgen-test-runner` `.cargo/config.toml` names as the runner.

The browser suites launch headless Firefox against the webpack build and
are ignored by default; they also need `node_modules`:
`cargo test -p strims-wasm --test e2e_links --test e2e_app --test ui_smoke -- --ignored`.
They stay on `cargo test`, which builds a binary's bundle once for all of
its tests.

The release simulations, the ignored ones included:
`cargo nextest run --release -p strims-sim -p strims-ppspp --run-ignored all`.

Every `.rs` file starts with:

```rust
// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
```
