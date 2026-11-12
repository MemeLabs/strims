#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sims' tables: a store of runs' data, and views over them in DuckDB.

  hack/sim-db/simdb.py describe [TOPIC]
  hack/sim-db/simdb.py lake QUERY [--param NAME=VALUE ...] [--json]
  hack/sim-db/simdb.py view [NAME] [--param NAME=VALUE ...] [--json] [--max-rows N]
  hack/sim-db/simdb.py index [--workers N] [--quiet]
  hack/sim-db/simdb.py fetch --exp E [--cells C,...] [--arms A,...] [--tables reads,forecast]
  hack/sim-db/simdb.py sql QUERY [selection] [--reads] [--json]
  hack/sim-db/simdb.py pooled [selection] [--by none|phase|bin|viewer] [--bin MS] [--json]
  hack/sim-db/simdb.py aligned EVENT [selection] [--field F] [--bin MS] [--per run|node] [--node N] [--peer N]
                                     [--detail GLOB] [--pool] [--before S] [--after S] [--nth N] [--json]
  hack/sim-db/simdb.py events [selection] [--json]
  hack/sim-db/simdb.py spec [selection] [--by phase|none] [--json]
  hack/sim-db/simdb.py osc [selection] [--json]
  hack/sim-db/simdb.py rows [selection] [--fields A,B,...]
  hack/sim-db/simdb.py forecast-report [--exp E] [--cell C] [--arm A] [--seed N] [--name NAME=GLOB]
                                       [--phases rest|all|P,...] [--collapsed yes|no] [--section S ...]
                                       [--sample N] [--decode-sample N] [--iters N] [--json]
  hack/sim-db/simdb.py coverage-report [--exp E] [--cell C] [--arm A] [--seed N] [--name NAME=GLOB]
                                       [--phases all|rest|P,...] [--collapsed yes|no] [--section S ...] [--json]
  hack/sim-db/simdb.py duckdb
  hack/sim-db/simdb.py fold DB JOB_DIR... [--src SRC]
  hack/sim-db/simdb.py export DB DEST
  hack/sim-db/simdb.py pull --run RUN [--reads [--cell C] [--arm A] [--seed N]] [--dry-run]
  hack/sim-db/simdb.py gc [--days N]

`describe`, `lake`, `view`, `index` and `fetch` are the catalog's
(hack/sim-db/catalog.py, whose docs say more): every result the cache,
the farm's and the suite's indexes know, in one DuckLake, whose tables
answer a question in one SQL statement, suite.py's scoring among its
macros. Start with `describe`, which lists the topics (`describe
glossary` the terms, `describe TABLE` a table's columns). `lake` queries
the whole catalog, across experiments, its `--param`s binding a
statement's `$name`s, and `view` runs one of the console's saved views
by name. `sql` and the queries after it read a selection of the cache's
runs instead (`--src`, `--cell`, `--arm`, `--seed`), for the deep dives
into a few runs: they pull the sample tables (below) from S3 where the
cache lacks them (`sql --reads`), and the catalog holds only those the
cache has (`fetch --tables` brings an experiment's in). The selections
and queries are hack/simlab/query.py's and the cache's pulls
hack/simlab/store.py's, in the duckdb module: every command but
`duckdb`, `fold` and `export` (the store's, in the CLI, under any
python3) runs under hack/'s uv venv, which the script execs itself into.

A run of either sim given an output dir (`SIM_OUT`, `STRIMS_SIM_OUT`)
writes its tables there as CSV files (the large ones zstd-compressed,
`<table>.csv.zst`), each beside a `<table>.schema.json` of its columns'
names and types, and a `series_meta.json`. The sims' rustdoc documents
them (crates/strims-sim/src/tables.rs, and the ppspp sim's
`Sim::write_tables`), and the catalog reads those docs: `describe
tables` lists the tables with their grains, and `describe TABLE` gives a
table's grain, keys, joins and what each column holds, `describe events`
the event kinds and `describe phases` the parents table's reasons. The
sims' tables (store_host's TABLES), each with the settings that write it
where a knob does:

  {tables}

The sample tables, the bulky ones the farm uploads apart and a pull
fetches only when asked (SAMPLE_TABLES):

  {samples}

A run's `bin_ms` is its bins' width: `--bin` over a bins column is a
multiple of every selected run's. The CLI's cache root is `SIM_DB_ROOT`
when set.

`fold` adds jobs' tables to a local DuckDB file, one transaction a batch,
each job's rows under its key `k` and the batch's `src`, beside two of
its own: `runs` (one a job: its cell, arm, seed, env, the row as JSON,
the meta, and the commit its binary was built from, whether the working
tree's changes went in (dirty) and the binary's sha256 where its folder
knows them: suite.py's local runs; the farm's are in its indexes) and
`scalars` (the row's numbers, long form, for SQL). Columns come from
the schema files, so a table gains the columns a later sim writes
(`ADD COLUMN IF NOT EXISTS`); folding a job again replaces it.
`export` writes a store to zstd Parquet, a file a table, with a
`manifest.json`. suite.py folds its local runs this way and exports each
invocation's store into the cache, `<main checkout>/target/sim-db/cache/`
(suite/<worktree>/<invocation>/), which every worktree shares; the farm
puts its instances' flushes in S3, and `pull` copies a run's into the
cache (farm/<run>/...). Its sample tables (sim-samples/) come with
`--reads`: the whole run's, or with `--cell`, `--arm` and `--seed` only
the flushes holding those jobs (the manifests' keys; a job held twice, a
verify copy, from the fewest flushes), and the queries that read them
(pooled, aligned on a read lag, forecast, and sql with `--reads`) pull
their selection's that way when the cache lacks them, noting the size,
at most `--pull-mb`. `--dry-run` lists what a pull would fetch, asking
S3 nothing. What a pull costs S3 goes to the farm's S3 ledger (pull). Nothing in the cache is kept for
long: a run is reproduced when it's wanted again, and `gc` deletes
what's older than `--days` (7).

The queries read the cache's tables and, made from them and kept under
derived/<table>/v=<derived_version>/ (a hash of analyze.py's
VERDICT_VERSION and of the derived tables' columns and code, so a change
to either makes them again), the verdicts' tables: `verdict_seconds`
(each second's phase, health and what judged it), `verdicts` (a run's
verdict, the seconds a collapse must last past, `startup_s`
(analyze.startup), and analyze.run_metrics), `episodes` (healthy,
unhealthy and collapse spells), `viewer_phases` (each viewer's phases,
its own startup then its seconds' verdict phases, as [start, stop) spans:
a viewer-second's phase) and `derived_events` (none of them a kind the
sims' events have; the viewers present rising or falling are
present_join and present_leave, the sims' join and leave a node's), the
vocabulary's derived kinds, DERIVED_KINDS:
{derived}

A selection picks runs: `--src` (cache paths, globs, default every one),
`--cell`, `--arm` (globs) and `--seed`, and a run's cell and arm are the
same whatever else it selects. A suite farm run's runs are named by the
suite cell their matrix ran, <cell>@<run>, which `--cell` matches whole
or by the cell alone, and by the arm suite.py names them by (arm_label):
a `--sets` run's (its index has a knob) by its set, the value its env
gives the knob, else by its matrix's arm, a `--refs` name. A farm run's
cells hold every row it scored: also those the farm reused from other
runs or matrices, by its matrices' indexes
(target/sim-farm/<matrix>/index.jsonl), whose sources' tables join the
selection (pulled when the cache lacks them). A key's tables come from
one of its sources whatever cells it counts in (its copies are one
job's): the one holding the most of them, then the later flush (its
manifest's `at`), then a farm flush's later seq, by number
(source_ranks).

  sql      a query over the views (runs, scalars, the sims' tables, the
           derived ones, and macros.sql's), their keys and joins below
  pooled   read lags pooled over the selected runs, by cell and arm: the
           quantiles of every read, beside the mean of the runs' own p99s;
           `--by` splits them by phase (its viewer's, viewer_phases_of's:
           tail past the verdict's seconds),
           by `--bin` of read time, or by viewer, each group's columns its
           own (its runs those with a read in it, their p99s of those
           reads)
  aligned  bands across runs around an event (`--nth` occurrence, the
           first by default): per `--bin` from `--before` to `--after`
           seconds, the median and p10/p90 over runs of `--field`: a read
           lag (lag_p50, lag_p99, lag_max, reads), a verdict column (healthy,
           lag_q, hit_share, delivery), a `seconds` column, or a `bins` one
           (summed over the bin). The events are the derived ones and the
           sims' own (`events`: the vocabulary's scenario and observation
           kinds, and with the decision logs its decision kinds, as
           `describe events` lists them; `--node` and `--peer` pick one
           node's, or the other's), a kind the selection lacks refused
           with the kinds it has. `aligned depart` anchors a run on its
           first departure, which opens the departures' window
           (`SIM_DEPART`, `STRIMS_SIM_DEPART`), `--nth 0` on each one, and
           `--per node` each viewer that left on its own. With `--per node` each
           node's own series is aligned on its own `--nth` occurrence of
           a sim's per-node event (one whose `node` is a viewer, as
           `describe events` says), the bands over every run's nodes
           (n counts them); the per-node fields are a node's reads' lags,
           its `node_bins` columns (summed over the bin) and its
           `viewer_seconds` lag_ms (the bin's worst second) and frames
           (summed). The verdict, `seconds` and `bins` columns are
           run-wide, as are the derived events: `--per node` refuses
           them, as `--per run` refuses a field only a node has. The
           forecast log's fields (LOG_FIELDS: forecast_n,
           forecast_err_p50, forecast_err_p90, forecast_abs_err_p50,
           forecast_abs_err_p90) are a bin's samples, per run or per node
           (the forecasting node's), as `forecast` judges them against
           the truth; the decision logs' fields (spec_n, spec_answered,
           spec_not_yet, spec_expired, spec_cancelled, spec_err_p50,
           spec_err_p90, spec_early, spec_late, park_n, park_expired) are
           a bin's rows of honest nodes, per run or per node (the
           requester's or the holder's). `--nth 0` aligns on every
           occurrence, a window each (n counts windows); `--pool` takes
           the field over every window's rows of a cell and arm
           together, a bin, rather than a band over windows; `--detail`
           (a glob) and `--since` / `--until` pick the events by their
           detail and time
  events   the run metrics pooled: collapses, onset after a step down and
           recovery after a step up (with how many runs they censored),
           excess lag, flaps
  osc      oscillation: spells and their dwell times, flaps, the lag's
           autocorrelation period and amplitude
  rows     the selected runs' rows (JSON lines), with their verdicts
  forecast the forecast logs pooled, by sim, cell and arm: the arrival
           models' predictions, where the sim knows when an honest
           neighbour truly held the chunk as it arrived (`live`), by the
           truth error at the median and the 90th percentile (held -
           (seen + mean), held - (seen + horizon): positive, the model
           predicted the hold too early) and the calibration of the
           predicted quantiles (the share of true holds by the predicted
           p50, p90 and p99 holds: 0.5, 0.9 and 0.99 when calibrated; p99
           where the log has `tail_ms`); and the HAVE samples, by the pace
           asked of the HAVEs: the error of each neighbour's
           announcement-lag estimate against the HAVE (sample - mean) and
           its horizon's coverage, and where the sim knows the truth the
           error against that (held - (seen + mean)) and the HAVE's
           lateness over it (seen + sample - held: the HAVE's hold and its
           one-way delay). Both give the share never held (censored), and
           the share the peer held before our channel with it opened
           (backlog: its first HAVEs announce them, so they don't count as
           forecasts of arrival). Only each HAVE's first sample of a
           stream counts, and not those that set the chunk's first sight
           (they are zero). `--json` gives the HAVE samples' rows, then
           the predictions' (`kind` prediction)
  spec     the decision logs pooled (honest nodes' rows), by sim, cell,
           arm and `--by` phase (the verdict's, or none): speculative
           requests by predictor, their outcome shares, the answered
           ones' time to answer, their graces, and where the holder truly
           held the chunk as it arrived (`live`) the early/late error
           (err_ms; positive, the request reached the holder first) and
           its share early, the prediction's error (held - predicted_ms,
           the model's median hold: positive, it predicted the hold too
           early) and the share of true holds by the predicted p50, p90
           and p99 holds (p90 and p99 where the log has them); parked and
           refused requests, their outcome shares, the refusals by reason
           and those of chunks the holder had held (the startup's
           announced-then-dropped chunks among them), the parked ones'
           release and expiry shares, and how long a park waited against
           its grace (released ones' wait over grace, and their wait).
           `--json` gives {"spec": rows, "parks": rows}

`forecast-report` is the catalog's (hack/simlab/forecast.py and
regimes.py, whose docs say more): the arrival forecast study over the
decision logs (asks, arrivals, haves) of the jobs its own selection picks
(`--exp`, `--cell` and `--arm` globs over experiment_jobs' exp_id, cell
and arm_label, `--seed`; `--name NAME=GLOB` names the arm of the labels a
glob matches, so one arm's two sims report as one), the requester's
phases `--phases` keeps (rest, every one but startup, by default) and the
runs `--collapsed` keeps by their verdict (every one by default), by
`--section`: runs (each arm and cell's runs, collapsed or not, and how
much and how badly they ask), decompose (failures and prediction errors
by the chunk's structure: the peer's path, its supply, its position
against the peer's STATE, whether a rival declared it), explained (how
well the model, structure and both predict a request's failure), scores
(pinball loss, WIS, coverage, the PIT histogram, sharpness), aligned (on
events), baselines (persistence and cadence, and the model's skill
against them), staleness (the STATE's age, the time since the peer's
parent changed), selection (the picked peer against its rivals), bound
(main's knowledge delay and its requests' outcomes beside the HAVE
removal's), every one of those by default; and regimes (a hidden
semi-Markov model of each peer's availability per stream, fit on
`--sample` streams a run and described on `--decode-sample`, `--iters`
EM iterations; minutes).

`coverage-report` is the catalog's too (hack/simlab/coverage.py, whose
docs say why each definition is what it is; the definitions are
scoring.sql's macros coverage_runs_of, seed_children_of,
coverage_chunks_of and seed_cover_of, which `describe macros` lists):
seed coverage, how soon each chunk the seed publishes reaches a node but
the seed (arrivals), by which route, and the seed's children of its
stream (parents), over the jobs forecast-report's flags pick, each phase
kept (`--phases`: all by default, of the chunk's publication or the
stream-second; a row of phase NULL pools those past startup) and each
verdict apart, by `--section`: latency (the judged chunks' latency
quantiles, their shares held late past the run's chunk interval and
never held, censored ones counted apart), routes (each first copy's
route and source: the seed's push, an answer, other, never), invariants
(I1: stream-seconds with no seed child while a viewer watches, the seed's
children and how their subscriptions ended), late (each late chunk
classed by the seed's children at its publication: none, released,
pushed late, never held by them, other), every one by default. Runs
without arrivals are left out, a selection of none with them refused;
the warnings (those runs, runs without push parents, an inferred stream
count) go to stderr, and `--json` gives them as `warnings` beside the
sections.

Every query takes `--json`: its rows, integer and decimal values (HUGEINT,
which sum() of integers gives, too) as numbers. The DuckDB CLI is pinned
(DUCKDB_VERSION, its sha256s) and fetched once into ~/.cache/strims/duckdb/
by `duckdb`, or by whatever needs it first.
"""

import argparse
import importlib.util
import json
import re
import sys
import textwrap
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT / "hack") not in sys.path:
    sys.path.insert(0, str(ROOT / "hack"))

# The store (hack/simlab/store_host.py: the farm hosts', stdlib and the
# DuckDB CLI), whose fold and export are this script's commands; its names
# are this module's too, for the scripts that import them from it.
from simlab.store_host import (  # noqa: E402, F401
    DOLLAR_QUOTE, DUCKDB_GZ_SHA256, DUCKDB_SHA256, DUCKDB_URL, DUCKDB_VERSION, FIRST_WORD, INT_TYPES, QUERY_WORDS,
    RUN_COLS, SAMPLE_TABLES, SCALAR_COLS, SQL_SPECIAL, TABLES, TYPES, DuckError, FoldError, Pairs, cache_root,
    csv_text, described, duck, duckdb_bin, export, fold, git, ident, locked, main_checkout, number, plain, q,
    recover, scalars, schema, sha256, statements, struct, tables_in, typed_row,
)
# The runs' series, their verdicts and the derived tables made from them
# (hack/simlab/verdict.py), which the views read.
from simlab.verdict import (  # noqa: E402, F401
    DERIVE_CODE, DERIVED, DERIVED_KINDS, META_ONLY, VERDICT_COLS, derive_source, derived_dir, derived_version, files,
    judge, load_series, plist, series_of, wanted, write_table,
)
# The cache's selections and queries (hack/simlab/query.py), and its
# pulls from S3 (hack/simlab/store.py), which the commands print.
from simlab import query, store  # noqa: E402
from simlab.query import PULL_MB  # noqa: E402, F401
# The arrival forecast study's sections (hack/simlab/forecast.py).
from simlab.forecast import SECTIONS as FORECAST_SECTIONS  # noqa: E402
# The seed coverage report's sections (hack/simlab/coverage.py).
from simlab.coverage import SECTIONS as COVERAGE_SECTIONS  # noqa: E402
# The event vocabulary (hack/simlab/vocab.py, stdlib), whose names the
# docs list.
from simlab import vocab  # noqa: E402

# The sims' event kinds, those a knob gates after the others, by knob.
EVENT_KINDS = "; ".join(
    [", ".join(k for k in vocab.kinds() if vocab.kind(k)["class"] != "derived" and "env_gate" not in vocab.kind(k))]
    + [f"with {knob}=1 / STRIMS_{knob}=1 {', '.join(ks)}" for knob, ks in vocab.knobs().items()])


def listed(doc, fields):
    """`doc` with each line that is a `{name}` alone `fields`' text of the
    name, wrapped at 72 columns at that line's indent."""
    def fill(m):
        return textwrap.fill(fields[m.group(2)], 72, initial_indent=m.group(1), subsequent_indent=m.group(1),
                             break_on_hyphens=False, break_long_words=False)

    return re.sub(r"^( *)\{([a-z_]+)\}$", fill, doc, flags=re.M)


# The sims' tables, a knob's beside those it writes, and the sample tables.
TABLES_TEXT = ", ".join(t + (f" ({query.knobs_text(t)})" if t in query.TABLE_KNOBS else "") for t in TABLES)
SAMPLES_TEXT = ", ".join(SAMPLE_TABLES)
__doc__ = listed(__doc__, {"tables": TABLES_TEXT, "samples": SAMPLES_TEXT, "derived": ", ".join(DERIVED_KINDS)})

# The views' keys and joins: `sql --help`, and the end of `--help`.
SQL_DOC = """\
Every view (macros.sql's too) has a run's key `k` and its source `src`, and
a key's tables come from one source, so any two views join on them:
`JOIN runs USING (k, src)` gives a row its run's cell, arm and seed, once a
cell (a key the farm reused counts in each of its cells). USING (k) joins
the same rows but keeps both `src` columns, to name as runs.src. Beside
(k, src) the views join on their own columns: reads and viewer_seconds to
viewers USING (k, src, viewer), as stalls does; events, parents, node_bins,
forecast, arrivals, spec_requests, parks, asks and haves to viewers USING (k, src,
node) (`node` in both sims; strims-sim's viewers also name it `host`),
and asks and haves to arrivals on the peer's holds (arrivals.node =
asks.peer, seq, arrivals.incarnation = asks.peer_incarnation, as haves':
the views' truth); seconds
to verdict_seconds USING (k, src, sec) (macros.sql's `sec`); a viewer's
second (viewer_seconds' sec, a read's or a stall's t_ms // 1000) to
scoring.sql's viewer_phases_of((SELECT list({'k': k, 'src': src}) FROM
runs)) ON (k, src, viewer) with start <= sec < stop, its phase (the
verdict's, then tail past its seconds).
runs' frames and bytes are its row's: beside viewer_seconds' frames, name
them runs.frames and runs.bytes.

  simdb.py sql "SELECT r.cell, r.arm, w.up, quantile_cont(lag_ms, 0.99) AS p99
    FROM reads JOIN viewers w USING (k, src, viewer) JOIN runs r USING (k, src)
    GROUP BY ALL ORDER BY ALL"
"""

_analyze = None


def analyze():
    """hack/capture/analyze.py: the verdicts, defined once."""
    global _analyze
    if _analyze is None:
        spec = importlib.util.spec_from_file_location("analyze", ROOT / "hack" / "capture" / "analyze.py")
        _analyze = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(_analyze)
    return _analyze


def __getattr__(name):
    """simlab.query's and simlab.store's names (rows, views, arm_label,
    pull_sources, have_src, sample_plan, ...), for the scripts that load
    this one."""
    for m in (query, store):
        if hasattr(m, name):
            return getattr(m, name)
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


# --- the commands -------------------------------------------------------------


def emit(a, rows, text):
    if a.json:
        print(json.dumps(rows))
    else:
        print(text())


def selection(a):
    return query.Selection.of(a)


def cmd_sql(a):
    got = query.sql(selection(a), a.query, reads=getattr(a, "reads", False), text=not a.json)
    if a.json:
        print(json.dumps(got[-1].records() if got else []))
    else:
        print(query.sql_text(got), end="")


def cmd_pooled(a):
    rows = query.pooled(selection(a), a.by).records()
    emit(a, rows, lambda: query.pooled_text(rows, a.by))


def cmd_aligned(a):
    rows = query.aligned(selection(a), a.event, a.field, a.per, a.node, getattr(a, "peer", None), a.detail, a.pool,
                         a.before, a.after, a.nth).records()
    emit(a, rows, lambda: query.aligned_text(rows, a.event, a.field, a.per, a.nth, a.pool))


def cmd_events(a):
    rows = query.events(selection(a)).records()
    emit(a, rows, lambda: query.events_text(rows))


def cmd_osc(a):
    rows = query.osc(selection(a)).records()
    emit(a, rows, lambda: query.osc_text(rows))


def cmd_forecast(a):
    rows = query.forecast(selection(a)).records()
    emit(a, rows, lambda: query.forecast_text(rows))


def cmd_spec(a):
    rows = query.spec(selection(a), a.by).records()
    emit(a, rows, lambda: query.spec_text(rows["spec"], rows["parks"]))


def cmd_rows(a):
    for line in query.selected_rows(selection(a), a.fields.split(",") if a.fields else None):
        print(json.dumps(line))


def report_selection(a):
    """A report's selection (simlab.forecast.Selection) from its flags."""
    from simlab import forecast

    phases = a.phases if a.phases in ("all", forecast.REST) else a.phases.split(",")
    names = {}
    for x in a.name or ():
        name, glob = x.split("=", 1)
        names[glob] = name
    collapsed = {None: None, "yes": True, "no": False}[a.collapsed]
    return forecast.Selection(exp=a.exp, cell=a.cell, arm=a.arm, seed=a.seed, names=names or None, phases=phases,
                              collapsed=collapsed)


def cmd_forecast_report(a):
    from simlab import forecast, regimes

    sel = report_selection(a)
    sections = a.section or list(forecast.SECTIONS)
    say = lambda m: print(m, file=sys.stderr)  # noqa: E731
    sql = [s for s in sections if s != "regimes"]
    got = forecast.report(sel, sql) if sql else {}
    out = {s: r.records() for s, r in got.items()}
    text = forecast.report_text(got) if got and not a.json else ""
    if "regimes" in sections:
        tables, _ = regimes.regimes(sel, sample=a.sample, decode_sample=a.decode_sample, iters=a.iters, say=say)
        out["regimes"] = tables
        text += regimes.regimes_text(tables) if not a.json else ""
    if a.json:
        print(json.dumps(out))
    else:
        print(text, end="")


def cmd_coverage_report(a):
    from simlab import coverage

    warned = []
    got = coverage.report(report_selection(a), a.section or list(coverage.SECTIONS),
                          warn=warned.append if a.json else None)
    if a.json:
        print(json.dumps({**{s: r.records() for s, r in got.items()}, "warnings": warned}))
    else:
        print(coverage.report_text(got), end="")


def cmd_pull(a):
    print(query.pull_text(query.pull_run(a.run, a.reads, a.cell, a.arm, a.seed, a.dry_run)))


def cmd_gc(a):
    print(f"removed {store.gc(a.days)} sources older than {a.days} days")


def catalog():
    """hack/sim-db/catalog.py: the catalog's commands (simlab.catalog's)."""
    spec = importlib.util.spec_from_file_location("catalog", Path(__file__).with_name("catalog.py"))
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def report_flags(p, phases_help, phases):
    """A report's selection flags (report_selection's) on parser `p`."""
    p.add_argument("--exp", action="append", help="experiments, globs (experiment_jobs' exp_id)")
    p.add_argument("--cell", action="append", help="cells, globs")
    p.add_argument("--arm", action="append", help="arm labels, globs")
    p.add_argument("--seed", action="append", type=int)
    p.add_argument("--name", action="append", metavar="NAME=GLOB", help="the arm name of the labels GLOB matches")
    p.add_argument("--phases", default=phases, help=phases_help)
    p.add_argument("--collapsed", choices=("yes", "no"), help="only the runs whose verdict collapsed, or only "
                   "those that didn't")


def main():
    raw = argparse.RawDescriptionHelpFormatter
    ap = argparse.ArgumentParser(description=__doc__, epilog=SQL_DOC, formatter_class=raw)
    sub = ap.add_subparsers(dest="cmd", required=True)
    lake = catalog().add_commands(sub)
    qs = {}
    qs["sql"] = sub.add_parser("sql", help="a query over a selection of the cache's runs, pulling their reads with "
                               "--reads (lake: the whole catalog)", description=SQL_DOC, formatter_class=raw)
    qs["sql"].add_argument("query")
    qs["sql"].add_argument("--reads", action="store_true",
                           help=f"pull the selection's sample tables ({SAMPLES_TEXT}) first, where the cache lacks "
                                "them")
    qs["pooled"] = sub.add_parser("pooled")
    qs["pooled"].add_argument("--by", choices=("none", "phase", "bin", "viewer"), default="none")
    qs["aligned"] = sub.add_parser("aligned")
    qs["aligned"].add_argument("event", help=f"a derived event ({', '.join(DERIVED_KINDS)}) or a sim's (the "
                               f"vocabulary's: {EVENT_KINDS}; `describe events` says what each holds)")
    qs["aligned"].add_argument("--node", type=int, help="only events of this node (the sims' events)")
    qs["aligned"].add_argument("--peer", type=int, help="only events about this other node (the sims' events)")
    qs["aligned"].add_argument("--detail", help="only events whose detail matches this glob (the sims' events)")
    qs["aligned"].add_argument("--pool", action="store_true",
                               help="the field over every window's rows pooled a bin, not a band over windows")
    qs["aligned"].add_argument("--per", choices=("run", "node"), default="run",
                               help="align each run's series on its event (run), or each node's own series on "
                                    "its own event (node: a sim's per-node event; the fields its reads', "
                                    "viewer_seconds' and node_bins')")
    qs["aligned"].add_argument("--field", default="lag_p99")
    qs["aligned"].add_argument("--before", type=float, default=10)
    qs["aligned"].add_argument("--after", type=float, default=60)
    qs["aligned"].add_argument("--nth", type=int, default=1, help="the occurrence to align on; 0: every one")
    qs["events"] = sub.add_parser("events")
    qs["osc"] = sub.add_parser("osc")
    qs["forecast"] = sub.add_parser("forecast")
    qs["spec"] = sub.add_parser("spec")
    qs["spec"].add_argument("--by", choices=("phase", "none"), default="phase")
    qs["rows"] = sub.add_parser("rows")
    qs["rows"].add_argument("--fields", help="the row's fields to print, comma-separated (all by default)")
    for x in qs.values():
        x.add_argument("--src", action="append", help="cache sources, globs (suite/WT/INV, farm/RUN/...)")
        x.add_argument("--cell", action="append", help="cells, globs")
        x.add_argument("--arm", action="append", help="arms, globs")
        x.add_argument("--seed", action="append", type=int)
        x.add_argument("--bin", type=int, default=1000, help="ms")
        x.add_argument("--since", type=float, help="s from the stream's start")
        x.add_argument("--until", type=float, help="s from the stream's start")
        x.add_argument("--pull-mb", type=float, default=PULL_MB,
                       help="the most of the reads it pulls from S3 for the selection, MB")
        x.add_argument("--json", action="store_true")
    fr = sub.add_parser("forecast-report", help="the arrival forecast study over the decision logs (simlab's "
                        "forecast and regimes)")
    report_flags(fr, "the requester's phases kept: rest (all but startup), all, or phases, comma-separated", "rest")
    fr.add_argument("--section", action="append", choices=[*FORECAST_SECTIONS, "regimes"],
                    help="the sections (every one but regimes by default)")
    fr.add_argument("--sample", type=int, default=1, help="regimes: streams a run the fit reads")
    fr.add_argument("--decode-sample", type=int, default=2, help="regimes: streams a run it describes")
    fr.add_argument("--iters", type=int, default=12, help="regimes: EM iterations")
    fr.add_argument("--json", action="store_true")
    cr = sub.add_parser("coverage-report", help="seed coverage: how soon each chunk reaches a node but the seed, "
                        "by which route, and the seed's children (simlab's coverage)")
    report_flags(cr, "the phases kept, a chunk's publication's or a stream-second's (the run's): all, rest (all but "
                 "startup), or phases, comma-separated; each kept phase reports apart, and a row of phase NULL "
                 "pools the kept ones past startup where a group has two or more", "all")
    cr.add_argument("--section", action="append", choices=COVERAGE_SECTIONS, help="the sections (every one by "
                    "default)")
    cr.add_argument("--json", action="store_true")
    sub.add_parser("duckdb", help="fetch the pinned DuckDB CLI")
    f = sub.add_parser("fold", help="fold job dirs into a store (each a dir holding the tables and a job.json)")
    f.add_argument("db")
    f.add_argument("dirs", nargs="+")
    f.add_argument("--src", default="manual")
    e = sub.add_parser("export")
    e.add_argument("db")
    e.add_argument("dest")
    p = sub.add_parser("pull")
    p.add_argument("--run", required=True)
    p.add_argument("--reads", action="store_true",
                   help=f"the sample tables too ({SAMPLES_TEXT}; sim-samples/)")
    p.add_argument("--cell", action="append", help="only these cells' reads, globs")
    p.add_argument("--arm", action="append", help="only these arms' reads, globs")
    p.add_argument("--seed", action="append", type=int, help="only these seeds' reads")
    p.add_argument("--dry-run", action="store_true", help="list the reads' files a pull would fetch, asking S3 nothing")
    g = sub.add_parser("gc")
    g.add_argument("--days", type=float, default=7)
    a = ap.parse_args()
    if a.cmd == "duckdb":
        print(duckdb_bin())
    elif a.cmd == "fold":
        jobs = []
        for d in a.dirs:
            meta = json.loads((Path(d) / "job.json").read_text()) if (Path(d) / "job.json").exists() else {}
            jobs.append({"k": meta.get("k", Path(d).name), "dir": d, **meta})
        print(json.dumps(fold(a.db, jobs, a.src)))
    elif a.cmd == "export":
        print(json.dumps(export(a.db, a.dest)))
    elif a.cmd == "pull":
        cmd_pull(a)
    elif a.cmd == "gc":
        cmd_gc(a)
    elif a.cmd in lake:
        lake[a.cmd](a)
    elif a.cmd == "forecast-report":
        cmd_forecast_report(a)
    elif a.cmd == "coverage-report":
        cmd_coverage_report(a)
    else:
        {"sql": cmd_sql, "pooled": cmd_pooled, "aligned": cmd_aligned, "events": cmd_events, "osc": cmd_osc,
         "rows": cmd_rows, "forecast": cmd_forecast, "spec": cmd_spec}[a.cmd](a)


# The commands the plain python3 runs, with the DuckDB CLI: the store's
# (store_host's). The others run in the duckdb module, under hack/'s uv
# venv.
STORE_COMMANDS = ("duckdb", "fold", "export")

if __name__ == "__main__":
    if sys.argv[1:2] and not sys.argv[1].startswith("-") and sys.argv[1] not in STORE_COMMANDS:
        from simlab import bootstrap

        bootstrap.engine(__file__)
        store.install()
    # The store's errors are this script's: one it doesn't catch reads
    # `DuckError: ...`, as a class of the script's own would.
    DuckError.__module__ = FoldError.__module__ = "__main__"
    main()
