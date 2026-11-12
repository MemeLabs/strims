# A row field, a series or bins column, a METRICS score

Main's examples: 8f6f4977 (a reading new to both sims' rows, series and
bins); e6533e0f (METRICS over rows that have the fields and rows that
don't).

## The sims

1. **strims-sim.** Declare the field first, in
   `crates/strims-wan/fields.toml` (its `rows` entry: the sims that
   write it, its type, unit, none and description; a family's `{param}`
   parts with their values), then run
   `python3 hack/simlab/vocab.py write`. `Report::row` in
   `crates/strims-sim/src/measures.rs` writes the row through the
   generated names (`crates/strims-sim/src/fields.rs`):
   `j.int(RowInt::Streams, n)`, `j.num`, `j.nums`, ..., a family's name
   by its constructor (`RowNum::delivery_who`), each key taking its
   declared type. Document the field with a sentence of measures.rs's
   module docs that names it in backticks: `catalog.row_docs` takes the
   first sentence of the sims'
   docs (`catalog.DOC_FILES`) and of suite.py's module docs that names
   a field, an upper-case part of a name (`stalls_per_min_D`) standing
   for any digits.
   - A per-second series field is a `seconds` column: declared under
     fields.toml's `columns.seconds`, written in `Report::series_data`
     (its doc table, and its `cols::Seconds` column), and in the
     `seconds` row of tables.rs's doc table.
   - A bins counter goes in `NODE_BIN_COUNTERS` and `bin_cols`.
2. **The ppspp sim.** Declared as step 1 says, the field is written by
   `job_row` in `crates/strims-ppspp/tests/sim.rs` through the generated
   names (`crates/strims-ppspp/tests/sim/fields.rs`):
   `row.int(RowInt::Streams, n)`, `row.num`, ..., with comment lines
   right above the line
   that writes the field: `catalog.row_docs` reads up to 12 lines above
   it when no doc sentence names it. Its series and bins are
   `Sim::write_tables`' (`series_data`, `bin_cols`).
3. **One name in both sims** where the meaning is the same, and each
   sim's meaning documented where they differ. 8f6f4977 gave one reading
   two names (`meter_budget_seeds` in strims-sim, `budget_capacity` in
   the ppspp sim), so every metric over it has to read both. The same
   goes for its aggregation and shape: a list of deciles in one sim and
   a median in the other are two readings, whatever their names. Where
   hostile hosts split a value, add the `_honest` variant beside it, as
   `lag_p99_ms_honest` is.
4. **A row field named like a table column** takes that column's
   sentence: `catalog.row_docs` takes the first doc sentence naming a
   field before any comment, and reads tables.rs's docs first. Check
   the field's doc in `simdb.py describe rows`, and give it a name of
   its own where the meanings differ.
5. **Readings come from counters the runners keep**, or from new ones
   that only count. Nothing new draws randomness or feeds a decision: a
   run's trace and every table stay the parent's, and its row differs by
   the new keys alone (`checks.md`).

## Scoring, in a simlab commit

6. A `suite.METRICS` tuple in `hack/simlab/suite.py`: (name, its SQL
   over `r.` (the row), `v.` (the verdict) and `j.` (the job), better,
   ratio or diff, tie band), with a comment right above it, which
   becomes metric_catalog's description (`catalog.suite_metrics`). One
   comment per tuple: a comment over a group of tuples documents only
   the first. A field one sim's rows lack reads NULL, which is no value.
7. The tie band from `python3 hack/sim-suite/suite.py calibrate`, whose
   output gives each metric's spread between seeds; the commit gives the
   spread the band came from.
8. `suite.GUARDRAILS` and `suite.SHOWN` change only with the user's
   decision: they change `arm_score`'s verdicts.
9. `catalog.scoring_version` hashes METRICS and scoring.sql, so the
   catalog's scoring is made again at the next index.

## Checks

- `golden.py check`. The scorecards set compares the lake's macros with
  the Python scoring at hack/sim-suite/golden.py's `REFERENCE`, over the
  metrics both have (`ref_metrics`): a new metric is left out of it, and
  a change to an existing metric's SQL fails it on purpose (the user's
  decision).
- The cli set moves (metric_defs, metric_catalog, `scoring_version`):
  re-capture it and say why.
- The commit says that existing experiments score the same with the new
  metric as without it, as e6533e0f did.
