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
no locks and no I/O. Drivers (`strims-node`, later `strims-wasm`) own
sockets, timers and tasks.

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
- `unsafe` is forbidden.

## Memory

- Chunk storage is preallocated when a swarm is created. Hot paths don't
  allocate per chunk or per message.
- Decoders return borrowed views into the input. Encoders write into
  caller-supplied buffers.

## Compatibility with the deployed network

Until the protocol version bump planned in `PLAN.md`, wire formats stay
compatible with the existing (Go) nodes. The golden-vector and live
cross-implementation tests (`golden/`, `tests/cross_go.rs`,
`tests/interop_go*`) guard this, and they skip when `go` isn't installed.
In code, express a compatibility requirement as the local constraint it
imposes, e.g. "port 20 must stay unhandled by the v2 protocol".

## Testing

1. Unit tests per crate.
2. In-crate deterministic simulations for protocol behaviour: an event-queue
   simulator, seeded latency and loss, and a virtual clock.
3. Whole-node deterministic simulation with turmoil in `strims-sim`. Failures
   print a one-line seed-replay command.

## Checks

From the repo root:

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --features strims-node/turmoil`
- `cargo test --workspace`
- `cargo check --workspace --exclude strims-node --exclude strims-sim --exclude strims-kv-redb --exclude strims-testworld --exclude strims-svc --target wasm32-unknown-unknown`

Every `.rs` file starts with:

```rust
// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
```
