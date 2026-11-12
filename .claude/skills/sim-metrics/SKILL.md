---
name: sim-metrics
description: "Adding or changing what the strims sims record or the sim lab computes: a row field or per-second series, a METRICS score, a new sim table or log or a column on one, an event kind (crates/strims-wan/vocab.toml), a verdict-derived table or run metric, a simlab analysis module with its simdb.py report command, or a saved console view. Use it when planning such a change and before editing the sims' tables or rows (strims-sim's tables.rs or measures.rs, the ppspp sim's tests/sim.rs), hack/simlab's store_host, catalog, suite METRICS, verdict or views, or hack/console/views, even for a docs-only change there. For querying results use sim-lab; for console links and selections, sim-console."
---

# Adding a metric to the sims and the lab

Querying results, and validating a new metric on runs: sim-lab. Links to
a view or reading the user's selection: sim-console.

## The pipeline

Both sims (strims-sim, and the ppspp sim in
`crates/strims-ppspp/tests/sim.rs`) write each run's row (one JSON
line), its tables (CSV beside a schema file each) and its events. Every
name they record is declared in `crates/strims-wan/fields.toml` (its
schema is `hack/simlab/spec.py`): `python3 hack/simlab/vocab.py write`
generates each sim's typed names from it, and each sim writes through
its module (`crates/strims-sim/src/fields.rs`,
`crates/strims-ppspp/tests/sim/fields.rs`).
`hack/simlab/store_host.py` folds a job's tables into the cache, and the
farm's hosts run that same file. The catalog (`hack/simlab/catalog.py`)
indexes the cache into the DuckLake: views over the tables, the
verdict's derived tables, the scoring macros, and the doc tables, which
it reads from fields.toml (`catalog.FIELDS`), the event vocabulary and
the registries below. The work is simlab's: scoring, the verdict,
the analysis modules. `simdb.py`, `ui.py`, the console server and the
saved views only parse, format or select.

## First, plan it with the user

If the metric, its tables or its schema aren't in a plan the user
approved, plan in plan mode first and get it approved. Schemas outlive
experiments: a sampled log of a decision's rivals was built on a branch
and then reverted at the user's call. The plan settles:

```
Plan: <the metric>
- Definition: what it measures, over whom, in what unit; what "none" is.
- Lines: which lines get it (main, decl, p4tf), and where the mechanism lives.
- Sims: both, with one name and one meaning, or each sim's meaning written down.
- Knob: one the measure tier already sets (suite MEASURE_LOGS), or a new one added there.
- Tables: each one's grain and keys, its size at star-31-5 seed 2004, sample table or not.
- Truth: computed in the sim, or joined in the lab's view from what the sim logs.
- Neutrality: the cells beyond the harness's set that exercise the change.
- Analysis and views: the module and report command, the macros, the saved views.
```

## Which change, which reference

Read the reference for the change, and the main commit that shows it:

| Change | Reference | Main's example |
|---|---|---|
| A new table or log, or a column on one | `references/sim-tables.md` | 6e2d479a + 3ed18cb8 (stalls, parents); 6262342c + 051596ac (the request logs); e18b801c (truth in the view) |
| A row field, a series or bins column, a METRICS score | `references/row-fields.md` | 8f6f4977 (a row field in both sims); e6533e0f (METRICS) |
| A verdict run metric, a derived table, a scoring macro | `references/derived.md` | 47f3ec81 (one phase rule); a28bcfdd (phases from events) |
| An event kind | `references/event-kinds.md` | cc950b02 (the vocabulary); f23ab376 (one name in both sims) |
| An analysis module and its report command | `references/reports.md` | 56cee325 (forecast-report) |
| A saved console view | `references/console-views.md` | b0705011, c23b84dd, 8959d6c8 |

Every change ends with `references/checks.md`: the trace-neutral check,
the Rust and Python checks, and what each golden diff means.

## Rules

- **Both sims, one meaning.** Give a value one name in both sims when it
  means the same thing; otherwise document each sim's meaning where the
  catalog reads it. A pair of names for one reading makes every
  consumer coalesce them (`meter_budget_seeds` and `budget_capacity`,
  8f6f4977), and a sim left out is a gap no test sees.
- **Trace-neutral.** A log draws no randomness of its own (the sims have
  one rng), no decision reads it, and no iteration order moves. Prove it
  against the parent commit (`references/checks.md`): the sims are
  deterministic, so any moved trace has a cause.
- **One library, one definition.** The work goes in hack/simlab; saved
  views are SQL over the lake. A metric shown in both a report and the
  console has one definition, a lake SQL macro in
  `hack/simlab/sql/scoring.sql` that both call, as `structural` and
  `effects_of` (through `effects`) are called by `simlab.scoring` and
  the console's `hack/console/views/scorecard.sql` (ac121f30 removed the
  console's own port of the scoring, which had drifted).
- **The vocabulary is the authority.** What a field, a table or a
  column holds is its `crates/strims-wan/fields.toml` entry's
  description, with each sim's notes: the catalog's descriptions come
  from it (`catalog.field_docs`), from the event vocabulary and from the
  registries, so an edit there moves the cli golden. The sims' doc
  tables point at it. No dev logs; rationale goes in rustdoc or module
  docs; results go in the commit message and in notebooks under target/,
  never in tracked files.
- **Main registers first.** Python registration of a table lands on main
  in its own commit, and each line gets it by merging main: tooling kept
  on a branch was lost when the branch merged main (9af57af6). test_catalog's
  `Registries` fails on a branch whose sims document a table that
  `store_host.TABLES` lacks.
- **Every recorded name is declared on main first.** Metrics that
  measure something main has land on main first, and branches get them
  by merging main. Every name a sim records, a line's included (today
  spec_requests, parks, and the STATE columns of the HAVE-removal line's
  asks), is declared in `crates/strims-wan/fields.toml` on main before
  any sim writes it: a line's declarations are its branch's bottom
  commit, landed on main before the rest. An entry only lines write has
  `sims = []` and names them in `lines`, so main's generated modules
  leave it out, and a line adds its sim to the entry when it writes the
  name. A dropped line's entries leave in the commit that adds its
  `crates/TRIED.md` entry.
- **The farm runs store_host.py and worker.py.** The farm ships both to
  its hosts from the main checkout, so a change landed during a run can
  hand its hosts different code. Before landing a change to either,
  check that no farm run is live:
  `pgrep -a python3 | grep -E 'farm.py|suite.py'` and
  `python3 hack/sim-farm/farm.py status`.
- **Path-scoped commits.** The main checkout holds other sessions'
  edits, under `src/devtools` among them: name exact files. Never
  checkout, restore, reset, clean or stash in the main checkout.
- **Worktrees.** Fast-forward a worktree to local main first, and give
  each tree its own `CARGO_TARGET_DIR`: Cargo judges freshness by
  mtime, so a shared target dir reuses another tree's stale units.
- **Index from main's code.** `simdb.py index` writes the catalog every
  session reads, its docs, keys, views and macros made by the indexing
  checkout's code. Index only from a checkout whose catalog code is
  main's; a branch's report or check reads its own definitions without
  indexing (`simlab.scoring` loads this checkout's scoring as TEMP).

## Order of work

1. The Rust lands on main first where main has the mechanism: both sims
   in one commit, trace-neutral against main's head. A module the lines
   share stays world-agnostic (its callers pass in their world's
   reading), so the lines' merges of main resolve to one file. A
   mechanism that only a line has gets its log on that line.
2. The Python registration, on main, committed `-- hack/`, with no farm
   run live.
3. Each line merges main, and is checked trace-neutral against its own
   head.
4. The analysis module, on main.
5. The views, on main.
6. Local measure-tier runs (`suite.py run --tier measure`),
   `simdb.py index` from main's checkout, and validation through
   sim-lab.

Commit messages follow the house style: the subject names the
components; the body says what changed and carries the evidence below,
the tests run and the golden sets, with each re-capture's reason.

## Baseline first, and the land gate

Before changing anything, run the checks of the change's crates and
`python3 hack/simlab/tests/golden.py check` on the parent, and note what
already fails there, so a failure the parent has isn't blamed on the
change, nor absorbed into it.

To land, the full golden check passes (scorecards included), and the
commit message carries the evidence of each check the change called for:

- the neutrality comparison (cells, seeds, what moved, what it compared
  against), saved in the notebook too (`references/checks.md`);
- every golden re-capture and its reason;
- for a new table or column, after its first runs, no warning of the
  index's (`catalog.warn_undocumented`) naming its columns;
- no farm run live, for a store_host or worker change;
- the wasm32 clippy line, for a node-core change;
- the calibrate spread behind a new tie band;
- `golden.py real` before and after, for a change that moves verdicts.

## The registries

Read these rather than any copy of them, this skill included:

- `crates/strims-wan/fields.toml`, every name the sims record, whose
  schema is `hack/simlab/spec.py`
- `crates/strims-wan/metrics.toml`, every scored metric, run metric, lake
  metric and dimension (`hack/simlab/semantic.py` reads it; test_semantic
  holds it to `suite.METRICS` and the naming rules to fields.toml)
- `store_host.TABLES`, `store_host.SAMPLE_TABLES`; `query.TABLE_KNOBS`
- `catalog.FIELDS`, `catalog.NORMALIZED`, `catalog.STANDARD_KEYS`,
  `catalog.STANDARD_JOINS`, `catalog.TABLE_DOCS`, `catalog.VERDICT_DOCS`,
  `catalog.UNITS`
- `suite.METRICS`, `suite.GUARDRAILS`, `suite.SHOWN`,
  `suite.MEASURE_LOGS`, `suite.TIERS` (`hack/simlab/suite.py`)
- `verdict.DERIVED`, `verdict.DERIVE_CODE`
- `crates/strims-wan/vocab.toml`, whose header gives its schema
- `views.KINDS`, `views.KEYS`
- `golden.SETS`

`simdb.py describe TABLE` shows what the catalog made of them for a
table: its grain, keys, joins and each column's doc.

## Keeping this skill current

`hack/simlab/tests/test_skills.py`, in golden.py's engine set, checks
that every path, Python and Rust name, command and commit this skill
and its references name exists. A commit that renames a registry
or changes a check procedure edits this skill too. Name things; don't
copy lists of tables, columns or kinds here.
