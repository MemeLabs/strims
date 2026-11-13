# Checks

## Trace-neutral

A change that only records must change no run. The harness builds the
parent and the change, runs both sims' cells on the same seeds with
every table and log on, and compares each run's trace and reads hash,
every row key, every column of every table and `series_meta.json`:

    python3 hack/sim-suite/suite.py neutral --base PARENT [--head REF] [--cells SET|CELL,...] [--seeds ...] [--expect TABLE.COL ...] [--renamed OLD=NEW ...] [--json]

- `--base` defaults to the head's parents, so a merge is checked against
  each of them; name a base to compare against another commit. `--head`
  defaults to HEAD, built from a worktree of its own; `--head .` checks a
  snapshot of the working tree, untracked files included.
- `--cells` takes a named set or cells. The harness's module docs
  (`hack/simlab/neutral.py`) say what its sets hold; add cells that
  exercise what the change touches (cc950b02 ran 16 cells, so that every
  kind it renamed was written).
- `--expect TABLE.COL` names a column the change means to add or move.
  Anything else that differs fails the check, and a difference always
  has a cause: the sims are deterministic.
- `--renamed OLD=NEW` compares the base's row field (or series_meta
  key) OLD with the head's NEW: a rename (`row-fields.md`) moves
  nothing, and a value that moved under it is a move of NEW's. The
  report names a rename that renamed nothing.
- A vocabulary commit, one that changes how the sims name what they
  record and not what they record (fields.toml, the generated modules,
  their call sites), is neutral with no `--expect`, over the `vocab`
  set (`neutral.VOCAB`: the audit's cells and every writer they leave
  off).
- The commit message gives the cells and seeds compared, the tables and
  columns that moved, and that nothing else did. Save the comparison
  itself (`--json`) in the notebook under target/, naming what it
  compared against: the parent, or another binary with the same traces.

A new log also has a size: run star-31-5 at seed 2004 with its knob on
and read the job's RSS (`jobs.rss_mb`); the measure tier gives each
cell `MEASURE_MEM_GIB`. After the harness, `suite.py run --base PARENT
--tier measure` scores the change over the measure tier, and its
`traces:` line says how many runs kept the base's trace.

## Rust

For the crates the change touches, against the parent's results:

- `cargo fmt --all --check`
- `cargo clippy --locked -p strims-ppspp -p strims-node-core -p strims-sim --all-targets --features strims-node/turmoil -- -D warnings`
- `cargo nextest run --locked --release -p strims-ppspp -p strims-node-core -p strims-sim --features strims-node/turmoil --no-fail-fast`
- the rustdoc warnings, public and private (`cargo doc --no-deps` and
  `cargo doc --no-deps --document-private-items` for each crate), no
  more than the parent's
- a node-core change also runs `crates/CONVENTIONS.md`'s wasm32 clippy
  line: `cargo clippy -p strims-node-core -p strims-wasm --target wasm32-unknown-unknown --all-targets --features strims-wasm/e2e -- -D warnings`

`hack/check.sh` runs the workspace's whole set; the change's crates are
usually enough.

## Python

The tests each reference names, then
`python3 hack/simlab/tests/golden.py check --skip scorecards` while
iterating, and the full `golden.py check` before landing (the
scorecards set takes minutes).

- The scorecards set reads the shared catalog: while another process
  writes it (an index), the set can fail with "database is locked".
  That says nothing of the change; run `golden.py check --only
  scorecards` again once the writer is done.
- `pnpm run test:js` under load needs mocha's `--timeout 20000`, and a
  `--spec` of one file still runs the whole suite (the config's spec
  glob adds to it).
- A new tier, or a tier's new cells, gets its line in
  `hack/sim-suite/suite.py`'s Tiers docs, beside `suite.TIERS`.

## What a golden diff means

| The diff | Why it moved | What to do |
|---|---|---|
| `cli: catalog` | a doc the catalog reads (fields.toml, `catalog.FIELDS`; a module's docstring, `catalog.NORMALIZED`, `catalog.TABLE_DOCS`), a registry (`store_host.SAMPLE_TABLES` is in availability's doc) | re-capture, and say why |
| `cli: help`, `cli: sql-help` | simdb.py's docs, the vocabulary's kinds, the registries its help is made from | re-capture, and say why |
| `cli` outputs that read verdicts | `VERDICT_VERSION` or a derived table moved (`references/derived.md`) | re-capture after `golden.py real` |
| `engine` | a fixture lacks what the change added (test_store's FoldExport without a new table) | fix the fixture, never the assertion |
| `matrices` | `suite.TIERS` or `suite.MEASURE_LOGS` | re-capture, and say why |
| `scorecards` | an existing metric's SQL or a scoring macro changed | the user's decision |
| a failure the parent has too | it was there before | report it; don't absorb it |

`python3 hack/simlab/tests/golden.py capture --only cli` re-captures the
cli set, and the commit says which outputs moved and why.

## Scratch scripts

A scratch script that calls simlab's indexer guards its work with
`if __name__ == "__main__":`. The verdict's derive pool is a
forkserver; a script without the guard hung under it while holding the
catalog's index lock, which blocked the console's index.
