# A verdict run metric, a derived table, a scoring macro

Main's examples: 47f3ec81 (a viewer-second's phase from one derived
table, `viewer_phases`, in place of three rules); a28bcfdd (the
verdict's phases from the runs' scenario events).

## Where it goes

1. **`hack/simlab/verdict.py`.**
   - A run metric: `verdict.run_metrics`, and its `run_metrics` entry
     in `crates/strims-wan/metrics.toml` (its description and unit), the
     catalog's doc of its column (`catalog.run_metric_docs`); a
     `_censored` companion is an entry of its own. test_catalog holds
     the entries to the scalars `verdict.run_metrics` writes
     (`semantic.run_metric_names`).
   - A derived table: `verdict.DERIVED` (its columns),
     `verdict.derive_source` (what makes it), `verdict.DERIVE_CODE`
     (the code its version hashes) and `verdict.VERDICT_COLS` for a
     verdicts column.
   - `VERDICT_VERSION` hashes the functions `verdict.verdict_version`
     lists by hand, apart from `verdict.DERIVE_CODE`: a new helper on
     the verdict's path joins one of the two lists, or a change to it
     derives nothing again (1e008248 fixed such a gap in the hashing).
   - simdb.py, suite.py and the tests import simlab.verdict itself, and
     the catalog's docs cite its names as `verdict.`: name a new one
     there where the docs use it.
2. **The cost.** A change moves `VERDICT_VERSION` or
   `verdict.derived_version`, and every cache source is derived again at
   the next index; collapse counts can move with it. Plan it with the
   user first, and before landing derive the sources the catalog holds
   into a scratch directory under target/, read-only, to see what moves
   (a28bcfdd derived 1,210 sources that way).
3. **The catalog.** A `catalog.TABLE_DOCS` entry for a new derived
   table, and a `catalog.VERDICT_DOCS` entry for every one of its
   columns; the catalog's verdict tables follow `verdict.DERIVED`
   (`catalog.VERDICT_TABLES`).
4. **A scoring macro** in `hack/simlab/sql/scoring.sql`:
   - a `-- needs:` line naming the tables or columns it reads that an
     older catalog may lack (`catalog.needed` makes it fail, saying so,
     where they're missing); a table macro's `-- else none:` or `-- else
     lacking:` line gives it a fallback there instead (`catalog.fallback`),
     so that a macro reading a table only some runs log (a log knob's) can
     read it through a gate (scoring.sql's `packets_or_none`) and score
     what the catalog holds;
   - a comment right above each `CREATE`, which `catalog.scoring_docs`
     makes its description;
   - `@{name}` for a constant of suite.py's (`catalog.scoring_consts`),
     never a copied number;
   - a `jobs` list as its input where a view or a report may call it
     (the `*_of(jobs)` form: `viewer_lag_of`, `effects_of`), so that
     neither copies it for jobs of its own;
   - vocabulary names from the vocabulary (as `@{...}` constants or the
     vocabulary's tables), never literals;
   - no list-valued default (`gaps := [100, 400]`, and a struct or map,
     however written): take scalars or a delimited string, as DuckLake
     commits such a macro and then can't read it back, every later
     ATTACH of the catalog failing until it is made again from the cache
     (`catalog.scoring_text` refuses one).
   `catalog.scoring_version` moves, and the catalog makes the scoring
   again at the next index. Before it, `simdb.py lake` and `view` from
   the changed checkout already run the change (`catalog.scoring_served`),
   so its before and after are the same query over one lake from the
   parent's checkout and from the change's.

## One definition

A phase rule, a score or a metric is defined once, and every report,
view and scorecard calls it: 47f3ec81 removed three phase rules that
disagreed (aimd's startup tail read x0.998 under one and x1.026 under
another), and ac121f30 replaced the console scorecard's port of the
Python scoring, whose ties had drifted, with calls to the macros. A
metric that both a report and the console show is a scoring.sql macro
both call (`structural` is both `simlab.scoring`'s and the collapse
views'). The phase of a second outside the verdict's seconds
(`query.TAIL`) has one rule too, which views and reports call; a view
that writes its own `coalesce(phase, ...)` gives a wrong meaning that
test_views' vocabulary check can't see.

The quantile has one method too: strims-base's log sketch (128 buckets
per doubling, read at the bucket's lower bound, integer nearest rank).
It is `simlab.sketch.quantile` in verdict.py and the reports, and
`sketch_quantile`/`sketch_quantiles` in scoring.sql and the views; the
sims' fields use `strims_base::stats::sketch`. A change to how a
reference quantity is taken goes in sim-suite's `golden.TAUGHT`, so the
scorecards set compares like with like.

A sum of floats that a scorecard prints uses scoring.sql's
`compensated_sum`, which adds as Python's sum() does. A `list_reduce`
fold strays in the last bits, so a mean that falls on a tie at the
printed places would print differently from the Python scoring's.

- These sums use `compensated_sum`: effects_of's means and sums of
  squares, arm_score_of's geometric means and fisher_p's sum.
- These sums of floats add plainly, as no golden compares them with a
  reference to the bit: dynamics' means (DuckDB's avg) and a metric's own
  SQL in metrics.toml (depth_mean's terms).
- The other macros' sums are of integers.

A new sum of floats that a scorecard prints takes `compensated_sum`.

## Checks

- Tests: test_verdict; `hack/sim-db/simdb_test.py`'s `PerViewerVerdict`
  and `Derived`; `hack/sim-suite/suite_test.py`'s `Dynamics` and
  `LagSince` fixtures; test_catalog.
- Goldens: the cli outputs that read verdicts move (pooled-phase, osc,
  rows, sql-verdicts among them). Run
  `python3 hack/simlab/tests/golden.py real capture` before the change
  and `golden.py real check` after it, over real experiments, and say in
  the commit what moved.
- The console's Ask answers: `python3 hack/simlab/ask_eval.py check`
  (free); re-pin `hack/simlab/ask_eval.json` where an answer moves for
  the change's reason.
- `hack/simlab/glossary.md`, where a term's meaning changes.
