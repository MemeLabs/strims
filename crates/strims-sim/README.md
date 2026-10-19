# strims-sim

Deterministic whole-node simulations. Every host runs a `strims-node` node
on turmoil's simulated network and clock, and everything a run does
derives from one `u64` seed.

## Running

```sh
cargo nextest run -p strims-sim --release                   # every scenario, 8 seeds
cargo nextest run -p strims-sim                             # debug: 2 seeds
cargo nextest run -p strims-sim --release --run-ignored only
```

| variable | effect |
|---|---|
| `STRIMS_SIM_SEEDS=<n>` | run seeds `1..=n` per scenario |
| `STRIMS_SIM_SEED=<s>` | run only seed `s` |
| `STRIMS_SIM_TRACE=1` | print each run's trace |

A sweep runs its seeds at once, each on a thread of its own. With
`--success-output immediate` (or `cargo test` with `--nocapture`), each
run prints a summary.

## Replaying a failure

A failing seed prints its reason, the tail of the trace and a replay
command:

```text
replay: STRIMS_SIM_SEED=1 cargo test -p strims-sim --release --test scenarios slow_consumer -- --exact --nocapture --include-ignored
```

Debug and release builds produce the same trace for a seed.

## Adding a scenario

Scenarios live in `tests/scenarios.rs`:
- Build a `Scenario` from hosts, links, swarms, faults and invariants.
- Pass the test function's name to `sweep`, so the replay command names the
  test.
- A scenario that reproduces an open bug is marked
  `#[ignore = "bug: <description>"]`.
