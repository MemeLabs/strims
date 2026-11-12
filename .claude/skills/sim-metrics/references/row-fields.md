# A row field, a series or bins column, a METRICS score

Main's examples: 8f6f4977 (a reading new to both sims' rows, series and
bins); e6533e0f (METRICS over rows that have the fields and rows that
don't).

## The sims

1. **strims-sim.** Declare the field first, in
   `crates/strims-wan/fields.toml` (its `rows` entry: the sims that
   write it, its type, unit, none and description; a family's `{param}`
   parts with their values), then run
   `python3 hack/simlab/vocab.py write`. The description is the field's
   doc in the catalog (`catalog.field_docs`): what it counts or
   measures, over whom and when, in words of its own, and a sim's note
   where that sim's reading says more or differs. `Report::row` in
   `crates/strims-sim/src/measures.rs` writes the row through the
   generated names (`crates/strims-sim/src/fields.rs`):
   `j.int(RowInt::Streams, n)`, `j.num`, `j.nums`, ..., a family's name
   by its constructor (`RowNum::delivery_who`), each key taking its
   declared type.
   - A per-second series field is a `seconds` column: declared under
     fields.toml's `columns.seconds` and written in
     `Report::series_data` (its doc table, and its `cols::Seconds`
     column).
   - A bins counter goes in `NODE_BIN_COUNTERS` and `bin_cols`.
2. **The ppspp sim.** Declared as step 1 says, the field is written by
   `job_row` in `crates/strims-ppspp/tests/sim.rs` through the generated
   names (`crates/strims-ppspp/tests/sim/fields.rs`):
   `row.int(RowInt::Streams, n)`, `row.num`, .... Its series and bins
   are `Sim::write_tables`' (`series_data`, `bin_cols`).
3. **One name in both sims** where the meaning is the same, and each
   sim's meaning documented where they differ. 8f6f4977 gave one reading
   two names (`meter_budget_seeds` in strims-sim, `budget_capacity` in
   the ppspp sim), so every metric over it has to read both. The same
   goes for its aggregation and shape: a list of deciles in one sim and
   a median in the other are two readings, whatever their names. Where
   hostile hosts split a value, add the `_honest` variant beside it, as
   `lag_p99_ms_honest` is.
4. **A row field named like a table column** is a name of its own
   in fields.toml (the rows' and each table's names are apart): give it
   a description of its own, and a name of its own where the meanings
   differ. Check the field's doc in `simdb.py describe rows`.
5. **Readings come from counters the runners keep**, or from new ones
   that only count. Nothing new draws randomness or feeds a decision: a
   run's trace and every table stay the parent's, and its row differs by
   the new keys alone (`checks.md`).

## Scoring, in a simlab commit

6. A `[[scored]]` entry in `crates/strims-wan/metrics.toml`, in the
   scorecards' order (`semantic.scored` makes `suite.METRICS` of them):
   its name, its `sql` over `r.` (the row), `v.` (the verdict) and `j.`
   (the job), or a `share` or `per_useful`; better; `score` (kind: ratio
   or diff, tie band); its unit; and a description where it isn't one
   field's plain reading (else it takes the field's), a note where how
   it scores says more: metric_catalog's description
   (`catalog.suite_metrics`). A field one sim's rows lack reads NULL,
   which is no value. `semantic.problems` (`python3 hack/simlab/vocab.py
   check`) holds every field it reads to fields.toml.
7. The tie band from `python3 hack/sim-suite/suite.py calibrate`, whose
   output gives each metric's spread between seeds; the commit gives the
   spread the band came from.
8. A metric's `guard` and `shown` (in its `score`; metrics.toml's
   `guardrails` gives the guards' order; `suite.GUARDRAILS` and
   `suite.SHOWN`) change only with the user's decision: they change
   `arm_score`'s verdicts.
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
