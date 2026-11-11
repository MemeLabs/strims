---
name: sim-lab
description: The strims sims' results and the lab CLI that queries them. Use for any question about the sims' results (experiments, farm or suite runs, arms, cells, seeds, collapses, lag, stalls, playback, overhead, scorecards, "did X help", comparing commits or branches), and for planning or running sim experiments. Use it before querying sim data or writing any analysis code over it.
---

# The sim lab

Every sim result this machine knows of, local suite runs and farm runs of
both sims, is in one read-only DuckDB catalog (a DuckLake), with suite.py's
scoring as SQL macros. The CLI below answers questions over it. Run each
tool as `python3 SCRIPT` from any checkout or worktree: the scripts re-exec
under hack/'s uv venv, and every worktree reads the main checkout's
target/sim-db.

## Start here

    python3 hack/sim-db/simdb.py describe            # the topics, and every table, macro, saved view and event kind
    python3 hack/sim-db/simdb.py describe glossary   # the terms: cells, arms, the verdict, collapses, phases, ...
    python3 hack/sim-db/simdb.py describe events     # the event kinds (crates/strims-wan/vocab.toml) and their counts
    python3 hack/sim-db/simdb.py describe NAME       # a table's columns and notes, a macro, a view's params and SQL, a kind

## Which command

| Question | Command |
|---|---|
| Anything across experiments, and the scoring macros (effects, collapse_test, arm_score, structural, dynamics, fisher_p) | `simdb.py lake 'SQL' [--param name=value ...] [--json]` |
| What the console's pages show (scorecard, collapse_grid, playback, run_timeline, ...) | `simdb.py view` lists them; `simdb.py view NAME --param exp=... --param arm=...` runs one |
| Raw tables of a selection of runs, pulling their reads from S3 on demand | `simdb.py sql 'SQL' --src/--cell/--arm/--seed ... [--reads]` |
| Deep dives into a few runs | `simdb.py pooled`, `aligned EVENT`, `events`, `osc`, `spec`, `forecast`, `rows` (each takes a selection) |
| Comparing arms | `python3 hack/sim-suite/suite.py score RUN:ARM RUN:ARM` (farm runs, the base first); `suite.py race --flags A,B,...` races flags in local runs |
| Data the catalog lacks that S3 still holds (7 days) | `simdb.py fetch --exp EXP [--tables reads]`, which prints what S3 charged |
| After new local runs | `simdb.py index` |
| Local runs | `suite.py run [--base REF] [--tier T] [--seeds N]` |
| New farm runs | `suite.py farm ... --no-run` writes the matrices (target/sim-suite/farm/RUN-NN.toml); the main checkout's `python3 hack/sim-farm/farm.py estimate MATRICES...` quotes them; nothing runs without the user's explicit approval |
| The console UI and its Ask panel | `python3 hack/console/server.py`, then http://127.0.0.1:8086/ |

`simdb.py` is `hack/sim-db/simdb.py`. Each command's `--help` says more:
`simdb.py --help` holds the cache's tables and selections, and
`simdb.py sql --help` their joins.

- Put a query with `$name`s in single quotes: in double quotes the shell
  expands `$exp` to nothing.
- A `lake --param` value binds as JSON where it parses (`7`, `true`,
  `[1,2]`), else as text. Quote a string of digits as `'"007"'`. `view`
  params are text, as the console's pickers send them.
- Narrow the per-second and per-read tables to an experiment's jobs
  first: `JOIN experiment_jobs USING (k, src) WHERE exp_id = ...`.
  `describe tables` gives each table's grain.

## Rules

- A farm run costs money. Quote it first with `farm.py estimate`, and run
  nothing on the farm without the user's explicit approval.
- Report dynamics, not a bare p99. Keep startup tails apart from
  steady-state spikes (`dynamics(exp)`), and give stalls at the 1250 and
  4250 ms playout delays. Put absolute values beside ratios.
- Judge collapse counts by their Fisher p (`collapse_test`), never by
  which seeds collapsed. Leave structural cells (`structural(exp)`) out of
  totals, and say how many collapses they hold.
- Decide at 32 seeds; 8 rarely can. Confirm a winner on fresh seeds
  (3033 and up), not only on the seeds it was tuned on.
- The sims are deterministic: a binary, a cell and a seed always give the
  same run. A result is never "flaky", and a difference has a cause.
- If the CLI can't answer something, add a thin wrapper: the work goes in
  hack/simlab, and a command in simdb.py or catalog.py only parses and
  prints. Don't write an ad-hoc script over the cache's parquet files,
  and don't curl the console server.
