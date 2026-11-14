# A row field, a series or bins column, a METRICS score

Main's examples: 8f6f4977 (a reading new to the rows, series and
bins); e6533e0f (METRICS over rows that have the fields and rows that
don't).

## The sim

1. **strims-sim.** Declare the field first, in
   `crates/strims-wan/fields.toml` (its `rows` entry: the sims that
   write it, its type, unit, none and description; a family's `{param}`
   parts with their values), then run
   `python3 hack/simlab/vocab.py write`. The description is the field's
   doc in the catalog (`catalog.field_docs`): what it counts or
   measures, over whom and when, in words of its own, and the sim's
   note where its reading says more. `Report::row` in
   `crates/strims-sim/src/measures.rs` writes the row through the
   generated names (`crates/strims-sim/src/fields.rs`):
   `j.int(RowInt::Streams, n)`, `j.num`, `j.nums`, ..., a family's name
   by its constructor (`RowNum::delivery_who`), each key taking its
   declared type.
   - A field only a research line writes is still declared on main
     first, with `sims = []` and `lines = ["decl"]` (the lines that write
     it). The line's branch adds its sim to `sims` when it starts writing
     the field. A field a lab program adds to a sim's row (suite.py's
     `_wall_s`) names that program in `writers` instead.
   - A per-second series field is a `seconds` column: declared under
     fields.toml's `columns.seconds` and written in
     `Report::series_data` (its doc table, and its `cols::Seconds`
     column).
   - A bins counter goes in `NODE_BIN_COUNTERS` and `bin_cols`.
2. **One name for one reading**, and a new name for a new meaning.
   8f6f4977 gave one reading two names, so every metric over it had to
   read both. The same goes for its aggregation and shape: a list of
   deciles and a median are two readings, whatever their names. A value
   recorded for honest and dishonest peers is written as `_honest` (in
   every run) and `_dishonest` (in runs with dishonest peers),
   the population last, as `lag_p99_ms_honest` is, and the unsuffixed
   all-peer value beside them only where the parts can't make it: the
   entry declares its `aggregate` (a count or sum is its parts alone; a
   quantile, ratio, mean or distinct count keeps its whole), and
   test_semantic's Populations holds the rule. Goals and guardrails read
   the `_honest` value. A quantile (a percentile, a median) is the one
   method: strims_base::stats::sketch's `percentile` over the values
   sorted, in the unit the field is written in (`ValueHistogram` where
   the sim keeps a histogram), and the lab's `simlab.sketch.quantile`
   and scoring.sql's `sketch_quantile`; an integer percentile, never a
   float q.
3. **A row field named like a table column** is a name of its own
   in fields.toml (the rows' and each table's names are apart): give it
   a description of its own, and a name of its own where the meanings
   differ. Check the field's doc in `simdb.py describe rows`.
4. **Readings come from counters the runners keep**, or from new ones
   that only count. Nothing new draws randomness or feeds a decision: a
   run's trace and every table stay the parent's, and its row differs by
   the new keys alone (`checks.md`).

## Scoring, in a simlab commit

5. A `[[scored]]` entry in `crates/strims-wan/metrics.toml`, in the
   scorecards' order (`semantic.scored` makes `suite.METRICS` of them):
   its name, its `sql` over `r.` (the row), `v.` (the verdict) and `j.`
   (the job), or a `share` or `per_useful`; better; `score` (kind: ratio
   or diff, tie band); its unit; and a description where it isn't one
   field's plain reading (else it takes the field's), a note where how
   it scores says more: metric_catalog's description
   (`catalog.suite_metrics`). A field a row lacks reads NULL, which is
   no value. `semantic.problems` (`python3 hack/simlab/vocab.py
   check`) holds every field it reads to fields.toml.
6. The tie band from `python3 hack/sim-suite/suite.py calibrate`, whose
   output gives each metric's spread between seeds; the commit gives the
   spread the band came from.
7. A metric's `guard` and `shown` (in its `score`; metrics.toml's
   `guardrails` gives the guards' order; `suite.GUARDRAILS` and
   `suite.SHOWN`) change only with the user's decision: they change
   `arm_score`'s verdicts.
8. `catalog.scoring_version` hashes METRICS and scoring.sql, so the
   catalog's scoring is made again at the next index.

## Renaming or retiring a name

9. **A name is never reused.** A field, a meta key or a table's column
    whose meaning changes, or that the sim stops writing while older
    runs hold it, stays in fields.toml, retired: `sims = []`, `retired`
    the sims whose older runs hold it, `replaced_by` the name that took
    its place (a table by sim where each sim's is another; left out
    where none did), and its description what it held. `spec.check`
    holds a replacement to a name its sims write now, and
    `semantic.problems` a metric to current names.
10. **A rename** (`rename = true`, rows only) is a replacement that
    means the same. The catalog's rows and scalars views, `query.rows`
    and farm.py's reports read an older run's value under the current
    name for the sims it's renamed for (`vocab.canonical_row`), raw.rows
    keeping what was written, and a matrix may name either
    (`vocab.resolve`). One commit renames the field in the sim, with
    METRICS reading the new name; it is neutral with `--renamed
    OLD=NEW` (`checks.md`), and the scorecards golden shows it moves no
    score. A replacement whose meaning differs is no rename: `--expect`
    names the fields that moved, and the commit says by how much.

## Checks

- `golden.py check`. The scorecards set compares the lake's macros with
  the Python scoring at hack/sim-suite/golden.py's `REFERENCE`, over the
  metrics both have (`ref_metrics`): a new metric is left out of it, and
  a change to how an existing metric reads a row (the user's decision)
  fails it until `golden.RESTATED` gives the reference that reading.
- The cli set moves (metric_defs, metric_catalog, `scoring_version`):
  re-capture it and say why.
- The commit says that existing experiments score the same with the new
  metric as without it, as e6533e0f did.
