# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The cache's selections and the queries over them, as results, and the
text hack/sim-db/simdb.py prints of each (its module docs say what they
are).

A Selection picks runs as simdb.py's options do (--src, --cell, --arm,
--seed, --since, --until, --bin, --pull-mb). `views` makes the views over
it in DuckDB (the canonical source of each key, the rows a farm run
reused, the suite's cells and arms, the derived tables, macros.sql's),
which a Views holds with a connection of its own; each query returns an
engine.Result (its values the CLI's JSON values by `records()`), and its
`*_text` renderer the text simdb.py prints of those records.

  views(sel, derived=True, fetch=True, say=None) -> Views
  sql(sel, query, reads=False, text=False) -> [engine.Result]
  pooled(sel, by="none") -> engine.Result
  aligned(sel, event, field="lag_p99", per="run", node=None, detail=None,
          pool=False, before=10, after=60, nth=1) -> engine.Result
  events(sel) -> engine.Result
  osc(sel) -> engine.Result
  forecast(sel) -> engine.Result
  selected_rows(sel, fields=None) -> [dict]
  pull_run(run, reads=False, cell=None, arm=None, seed=None, dry_run=False)
          -> Pulled
  pooled_text(rows, by), aligned_text(rows, ...), events_text(rows),
  osc_text(rows), forecast_text(rows), pull_text(pulled)

and the selection's parts, which other modules use too:

  sources(patterns) -> [src]          (simlab.store's)
  reused(srcs, stored) -> (sql | None, [src])
  source_ranks(srcs) -> {src: rank}
  suite_index(run), suite_cells(names), arm_label(knob, sim, arm, env)
  rows(pairs, series=False) -> {(src, k): row}
  pull_sources(srcs) -> [src]

A query that the selection can't answer raises Refused (store's: a
SystemExit, its message why); a failing statement store_host.DuckError,
DuckDB's message. `say` takes the lines a selection says on the way (a
pull of what it reused, of its samples), stderr's by default.
"""

import contextlib
import dataclasses
import json
from pathlib import Path

from . import engine, store, store_host, verdict
from .store import Refused, have_src, manifest_of, sources  # noqa: F401
from .store_host import SAMPLE_TABLES, TABLES, ident, q, struct
from .verdict import DERIVED, derived_dir, files, judge, load_series, plist, wanted

# The most a query pulls of the sample tables (SAMPLE_TABLES) by default
# (`--pull-mb`).
PULL_MB = 1000
# macros.sql: the views' macros.
MACROS = Path(__file__).with_name("sql") / "macros.sql"


@dataclasses.dataclass
class Selection:
    """The runs a query reads: `src` (cache paths, globs, default every
    one), `cell` and `arm` (globs) and `seed`; the reads' times `since`
    and `until` (s from the stream's start), the bins `bin` (ms) and the
    most of the sample tables a query pulls from S3, `pull_mb`."""
    src: list = None
    cell: list = None
    arm: list = None
    seed: list = None
    since: float = None
    until: float = None
    bin: int = 1000
    pull_mb: float = PULL_MB

    @classmethod
    def of(cls, a):
        """The selection simdb.py's options `a` (argparse's) give."""
        return cls(**{f.name: getattr(a, f.name, f.default) for f in dataclasses.fields(cls)})


@dataclasses.dataclass
class Views:
    """The SQL that makes a selection's views, the tables it has, and an
    in-memory DuckDB instance holding them, made at its first query and
    closed by `close` (or the end of a `with`)."""
    sql: str
    have: set = dataclasses.field(default_factory=set)
    conn: object = dataclasses.field(default=None, repr=False)

    def connection(self):
        if self.conn is None:
            engine.need_module()
            conn = engine.store(":memory:", None, None, store.spill())
            try:
                store.shown(conn, self.sql)
            except BaseException:
                conn.close()
                raise
            self.conn = conn
        return self.conn

    def results(self, sql, text=False):
        """The Results of the statements of `sql` the CLI prints, over the
        views (store.shown)."""
        return store.shown(self.connection(), sql, text=text)

    def run(self, sql):
        """The last Result `sql` shows, or None."""
        got = self.results(sql)
        return got[-1] if got else None

    def records(self, sql):
        """The rows of the last statement `sql` shows, as the CLI's JSON
        gave them ([] for none)."""
        got = self.run(sql)
        return got.records() if got else []

    def close(self):
        if self.conn is not None:
            self.conn.close()
            self.conn = None

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()


def stderr(message):
    store.stderr(message)


def records(sql):
    """The rows of `sql`'s last shown statement, run in an instance of its
    own, as the CLI's JSON gave them."""
    return store.execute(sql)


# --- rows ---------------------------------------------------------------------


def rows(pairs, series=False):
    """The rows of `pairs` of (src, k) in the cache, {(src, k): row}: each
    the sim's row (less its series) with `_verdict` and `_metrics`
    (analyze.py) from its tables' series, `_wall_s` and `_rss_mb`, and
    with `series` the series as `_series`."""
    pairs = sorted(set(pairs))
    runs = files(sorted({s for s, _ in pairs}), "runs")
    if not pairs or not runs:
        return {}
    with store.installed():
        got = load_series(pairs)
    out = {}
    for r in records(f"SELECT src, k, row_json, wall_s, rss_mb FROM read_parquet({plist(runs)}, union_by_name=true) "
                     f"SEMI JOIN {wanted(pairs)} USING (src, k);"):
        row = json.loads(r["row_json"])
        row["_wall_s"], row["_rss_mb"] = r["wall_s"], r["rss_mb"]
        _, s = got.get((r["src"], r["k"]), ({}, None))
        row["_verdict"], row["_metrics"] = judge(s)
        if series:
            row["_series"] = s
        out[(r["src"], r["k"])] = row
    return out


def pull_sources(srcs):
    """Pulls the farm runs of the sources `srcs` (farm/<run>/...) that the
    cache lacks, a run at a time, less their samples (store.pull_sources);
    returns those still missing (expired from S3, or never uploaded)."""
    return store.pull_sources(srcs)[1]


# --- derived tables -----------------------------------------------------------

# The derived tables a source has once judged (verdict.derive_source).
JUDGED = (*DERIVED, "verdicts")
# The processes `derive` judges more than one source in (verdict.derive's
# workers: None its default, 0 this process).
WORKERS = None


def judged(src):
    """Whether source `src` has its derived tables (verdict.derive_source)."""
    name = src.replace("/", "~") + ".parquet"
    return all((derived_dir(t) / name).exists() for t in JUDGED)


def derive(srcs):
    """Makes the derived tables of the sources `srcs` that lack them
    (verdict.derive): one in this process, more in a pool. Refused names
    those that failed."""
    lacking = [s for s in srcs if not judged(s)]
    if not lacking:
        return
    with store.installed():
        got = verdict.derive(lacking, workers=0 if len(lacking) == 1 else WORKERS)
    if got.failed:
        raise Refused("\n".join(f"verdicts: {s}: {e}" for s, e in sorted(got.failed.items())))


def derived_files(table, srcs):
    """SQL reading derived `table`'s files of the sources `srcs`, as `x`,
    else None. The `v=` directory isn't a hive partition, and a `v` column
    a file holds (DuckDB's, from a file written there with hive
    partitioning on) is left out."""
    d = derived_dir(table)
    f = [str(d / name) for name in (s.replace("/", "~") + ".parquet" for s in srcs) if (d / name).exists()]
    if not f:
        return None
    return (f"(SELECT COLUMNS(lambda c: c <> 'v') FROM read_parquet({plist(f)}, union_by_name=true, "
            "hive_partitioning=false)) x")


# --- selection and views ------------------------------------------------------


def suite_index(run):
    """Suite farm run `run`'s index: the main checkout's
    target/sim-suite/farm-runs/<run>.json, or while suite.py keeps it apart
    (written by `farm --no-run`, the run started by hand) pending/<run>.json
    there; {} if neither."""
    d = store_host.main_checkout() / "target" / "sim-suite" / "farm-runs"
    for p in (d / f"{run}.json", d / "pending" / f"{run}.json"):
        try:
            return json.loads(p.read_text())
        except (OSError, ValueError):
            continue
    return {}


def arm_label(knob, sim, arm, env):
    """A suite farm job's arm, as simdb and suite.py both name it: when its
    run's index names a knob (`knob`, by sim: a `--sets` run's), the value
    the job's `env` gives its sim's, its set; else `arm`, its matrix's arm
    (a `--refs` name). views names its runs' arms this way in SQL."""
    k = (knob or {}).get(sim)
    return arm if k is None else (env or {}).get(k, arm)


def suite_cells(names):
    """The suite cells that farm matrices named `names` run: a suite farm
    run's matrices are <run>-<i>, the i-th cell of its index (suite_index),
    and a run's arms are named by arm_label, from the knob its index names.
    By matrix: (<cell>@<run>, knob), a name of the matrix alone, so a cell
    that two suite runs share (a farm run of several tiers, an A/B and its
    confirmation) is apart whichever of them a selection holds. Names no
    index tells are left out."""
    idx, got = {}, {}
    for n in names:
        run, _, i = (n or "").rpartition("-")
        if not (len(run) == 6 and run[0] == "s" and i.isdigit()):
            continue
        if run not in idx:
            idx[run] = suite_index(run)
        cells = idx[run].get("cells") or []
        if int(i) < len(cells):
            c = cells[int(i)]
            got[n] = (f"{c['name']}@{run}", (idx[run].get("knob") or {}).get(c.get("sim")))
    return got


def reused(srcs, stored):
    """The rows the farm runs of sources `srcs` count by reference: the
    lines of their matrices' indexes (the main checkout's
    target/sim-farm/<matrix>/index.jsonl) whose row ran for another matrix
    (`from`: the farm reused a row of an earlier run, or of a matrix of its
    own that shares the job), as SQL of (k, src, matrix, arm), else None;
    and the sources holding those rows. A run's matrices are its state's
    list, and the cells its rows in `srcs` were `stored` under."""
    farm = store_host.main_checkout() / "target" / "sim-farm"
    names = set(stored)
    for run in {s.split("/")[1] for s in srcs if s.startswith("farm/")}:
        with contextlib.suppress(OSError):
            names |= set((farm / run / "matrices").read_text().split())
    paths = sorted(str(farm / n / "index.jsonl") for n in names if n and (farm / n / "index.jsonl").is_file())
    if not paths:
        return None, []
    matrix = r"regexp_extract(filename, '([^/]+)/index\.jsonl$', 1)"
    cols = struct({"key": "VARCHAR", "src": "VARCHAR", "arm": "VARCHAR", "from": "VARCHAR"})
    sql = (f"(SELECT key AS k, src, {matrix} AS matrix, arm FROM read_json({plist(paths)}, "
           f"format='newline_delimited', columns={cols}, filename=true, ignore_errors=true) "
           f"WHERE src IS NOT NULL AND \"from\" <> {matrix})")
    held = [r["src"] for r in records(f"SELECT DISTINCT src FROM {sql};")]
    return (sql if held else None), held


def source_ranks(srcs):
    """{src: rank} over the cache's sources `srcs`, the higher the rank the
    sooner the views take a key's tables from it: the source holding the
    most of the sims' tables (a farm run's reads come with `pull --reads`),
    then the later flush (its manifest's `at`), then a farm flush's later
    seq (farm/<run>/<instance>/<seq>, by number), then the name."""
    root = store_host.cache_root() / "cache"

    def key(s):
        last = s.rsplit("/", 1)[-1]
        seq = int(last) if s.startswith("farm/") and last.isdigit() else -1
        return sum((root / s / f"{t}.parquet").exists() for t in TABLES), manifest_of(s).get("at") or "", seq, s

    return {s: n for n, s in enumerate(sorted(srcs, key=key))}


def views(sel, derived=True, fetch=True, say=None):
    """The views over selection `sel`'s sources (a Views). A farm source's
    runs name the suite cells and arms its matrices ran (suite_cells), and
    a farm run's cells hold the rows it reused (reused: their sources'
    tables join the views, pulled into the cache when it lacks them,
    unless not `fetch`); `matrix` is the matrix (or local cell) a run
    counts for. With `derived` the sources' derived tables are made where
    they lack them (derive) and join the views."""
    say = say or stderr
    srcs = sources(sel.src)
    if not srcs:
        raise Refused("no runs in the cache match --src")
    stored = f"read_parquet({plist(files(srcs, 'runs'))}, union_by_name=true)"
    moved, held = None, []
    if any(s.startswith("farm/") for s in srcs):
        moved, held = reused(srcs, [r["cell"] for r in records(
            f"SELECT DISTINCT cell FROM {stored} WHERE src LIKE 'farm/%';")])
    lack = [s for s in held if not have_src(s)]
    if lack and not fetch:
        say(f"the cache lacks {len(lack)} flushes whose rows the selection's runs reused: left out")
    elif lack:
        say(f"pulling the farm runs of {len(lack)} flushes whose rows the selection's runs reused")
        gone = pull_sources(lack)
        if gone:
            say(f"{len(gone)} of them are gone from S3 (a week's): their rows are left out")
    srcs = sorted({*srcs, *(s for s in held if have_src(s))})
    if derived:
        derive(srcs)
    read_runs = f"(SELECT *, cell AS matrix FROM {stored})"
    if moved:
        read_runs = (f"(SELECT * FROM {read_runs} UNION ALL BY NAME "
                     f"SELECT r.* REPLACE (m.matrix AS cell, m.arm AS arm), m.matrix "
                     f"FROM read_parquet({plist(files(srcs, 'runs'))}, union_by_name=true) r "
                     f"JOIN {moved} m USING (k, src))")
    cells = {}
    if any(s.startswith("farm/") for s in srcs):
        cells = suite_cells([r["matrix"] for r in records(f"SELECT DISTINCT matrix FROM {read_runs};")])
    if cells:
        pairs = ", ".join(f"({q(n)}, {q(c)}, {q(k) if k else 'NULL'})" for n, (c, k) in cells.items())
        # The arm as arm_label names it.
        read_runs = (
            f"(SELECT r.* REPLACE (coalesce(m.cell, r.cell) AS cell, CASE WHEN m.knob IS NULL THEN r.arm "
            f"ELSE coalesce(json_extract_string(r.env, '$.' || m.knob), r.arm) END AS arm) "
            f"FROM {read_runs} r LEFT JOIN (VALUES {pairs}) m(matrix, cell, knob) ON r.matrix = m.matrix)"
        )
    where = []
    if sel.cell:
        where.append("(" + " OR ".join(f"cell GLOB {q(c)} OR split_part(cell, '@', 1) GLOB {q(c)}"
                                       for c in sel.cell) + ")")
    if sel.arm:
        where.append("(" + " OR ".join(f"arm GLOB {q(x)}" for x in sel.arm) + ")")
    if sel.seed:
        where.append(f"seed IN ({', '.join(str(int(s)) for s in sel.seed)})")
    rank = "(VALUES " + ", ".join(f"({q(s)}, {n})" for s, n in source_ranks(srcs).items()) + ") h(src, rank)"
    sql = [
        f"CREATE VIEW runs_all AS SELECT * FROM {read_runs};",
        # A key's tables come from one source, whatever cells it counts in
        # (the copies are one job's): the top of source_ranks. So (k) is a
        # key's whole identity in the views, and USING (k) joins the rows
        # USING (k, src) does.
        f"CREATE VIEW canon AS SELECT k, src FROM runs_all LEFT JOIN {rank} USING (src) "
        "QUALIFY row_number() OVER (PARTITION BY k ORDER BY coalesce(rank, -1) DESC) = 1;",
        # A key in two cells (a job the farm reused) counts in each, with the
        # cell's own row (its copy in the key's source, else its top one).
        f"CREATE VIEW runs AS SELECT * FROM (SELECT r.* REPLACE (c.src AS src) FROM runs_all r JOIN canon c USING (k) "
        f"LEFT JOIN {rank} ON h.src = r.src QUALIFY row_number() OVER (PARTITION BY r.k, r.cell "
        "ORDER BY r.src = c.src DESC, coalesce(h.rank, -1) DESC) = 1)"
        + (" WHERE " + " AND ".join(where) if where else "") + ";",
    ]
    have = {"runs"}
    for t in ("scalars", *TABLES):
        f = files(srcs, t)
        if f:
            cols = "x.*"
            if t == "viewers":
                # strims-sim names a viewer's node its host: `node` in both
                # sims, as events, node_bins and forecast name it.
                got = {r["column_name"] for r in records(f"DESCRIBE SELECT * FROM read_parquet({plist(f)}, "
                                                         "union_by_name=true);")}
                if "host" in got:
                    cols = "x.* REPLACE (coalesce(x.node, x.host) AS node)" if "node" in got else "x.*, x.host AS node"
            sql.append(f"CREATE VIEW {t} AS SELECT {cols} FROM read_parquet({plist(f)}, union_by_name=true) x "
                       "SEMI JOIN runs USING (k, src);")
            have.add(t)
    if derived:
        for t in JUDGED:
            if f := derived_files(t, srcs):
                sql.append(f"CREATE VIEW {t} AS SELECT x.* FROM {f} SEMI JOIN runs USING (k, src);")
                have.add(t)
    sql += macros(have)
    return Views("\n".join(sql), have)


def macros(have):
    """macros.sql's statements whose `-- needs:` tables the views have."""
    out, needs, cur = [], set(), []
    for line in MACROS.read_text().splitlines():
        if line.startswith("-- needs:"):
            needs = set(line.split(":", 1)[1].split())
        cur.append(line)
        if line.rstrip().endswith(";"):
            if needs <= have:
                out.append("\n".join(cur))
            cur, needs = [], set()
    return out


def need(have, *tables):
    """Refused unless the views have every one of `tables`."""
    missing = [t for t in tables if t not in have]
    if missing:
        raise Refused(f"the selection has no {', '.join(missing)}"
                      + (" (nor does S3: run with the reads)" if "reads" in missing else "")
                      + (" (nor does S3: run with SIM_FORECAST=1 / STRIMS_SIM_FORECAST=1)"
                         if "forecast" in missing else ""))


def with_samples(sel, v, tables, derived=True, say=None):
    """The views `v` of selection `sel` once the cache holds what S3 has of
    its jobs' sample `tables` (store.pull_samples, at most `sel.pull_mb`):
    new views when it pulled any."""
    if store.pull_samples(v.sql, tables, limit_mb=sel.pull_mb, say=say):
        v.close()
        return views(sel, derived, say=say)
    return v


def time_filter(sel, col="t_ms"):
    w = []
    if sel.since is not None:
        w.append(f"{col} >= {int(sel.since * 1000)}")
    if sel.until is not None:
        w.append(f"{col} < {int(sel.until * 1000)}")
    return w


# --- text ---------------------------------------------------------------------


def fmt(x, nd=0):
    if x is None:
        return "-"
    if isinstance(x, float):
        return f"{x:.{nd}f}"
    return str(x)


def short(arm):
    """An arm label for a table's header."""
    return arm if arm is None or len(arm) <= 24 else arm[:10] + "..." + arm[-11:]


# --- queries ------------------------------------------------------------------


def sql(sel, query, reads=False, text=False, say=None):
    """`query` (a script, its statements run in order) over selection
    `sel`'s views, with `reads` once the cache holds the selection's
    sample tables (with_samples): the Results of the statements the CLI
    prints, with `text` their values DuckDB's strings."""
    v = views(sel, say=say)
    if reads:
        v = with_samples(sel, v, SAMPLE_TABLES, say=say)
    with v:
        return v.results(query + ("" if query.rstrip().endswith(";") else ";"), text=text)


def sql_text(results):
    """`sql`'s text Results as the CLI's -table mode prints them."""
    return "".join(r.table() for r in results)


def pooled(sel, by="none", say=None):
    """Read lags pooled over the selection's runs, by cell and arm, and by
    `by` (none, phase, bin or viewer)."""
    v = with_samples(sel, views(sel, say=say), ("reads",), say=say)
    with v:
        need(v.have, "reads")
        group = {"none": "NULL", "bin": f"(t_ms // {sel.bin}) * {sel.bin}", "viewer": "viewer",
                 "phase": "coalesce(vs.phase, 'tail')"}[by]
        join = ("LEFT JOIN verdict_seconds vs ON vs.k = x.k AND vs.src = x.src AND vs.sec = x.t_ms // 1000"
                if by == "phase" else "")
        where = time_filter(sel, "x.t_ms")
        # Every column is the group's: its runs are those with a read in it,
        # and each run's p99 is of its reads there.
        same = " AND ".join(f"pool.{c} IS NOT DISTINCT FROM runp.{c}" for c in ("cell", "arm", "g"))
        return v.run(f"""
WITH x AS (SELECT r.cell, r.arm, {group} AS g, x.* FROM reads x JOIN runs r USING (k, src) {join}
           {"WHERE " + " AND ".join(where) if where else ""}),
per AS (SELECT cell, arm, g, k, src, quantile_cont(lag_ms, 0.99) AS p99 FROM x GROUP BY ALL),
runp AS (SELECT cell, arm, g, avg(p99) AS mean_run_p99, count(*) AS runs FROM per GROUP BY ALL),
pool AS (SELECT cell, arm, g, count(*) AS reads, quantile_cont(lag_ms, [0.5, 0.9, 0.99]) AS q, max(lag_ms) AS lag_max
         FROM x GROUP BY ALL)
SELECT pool.cell, pool.arm, pool.g, runs, reads, q[1] AS lag_p50, q[2] AS lag_p90, q[3] AS lag_p99, lag_max,
       mean_run_p99
FROM pool JOIN runp ON {same} ORDER BY pool.cell, pool.arm, pool.g;""")


def pooled_text(rows, by="none"):
    lines = [f"{'cell':22} {'arm':24} {by if by != 'none' else '':>10} {'runs':>5} {'reads':>9} "
             f"{'p50':>7} {'p90':>7} {'p99':>7} {'max':>7}  mean of runs' p99"]
    for r in rows:
        lines.append(f"{r['cell'] or '-':22} {short(r['arm']) or '-':24} {fmt(r['g']):>10} {r['runs']:5d} "
                     f"{r['reads']:9d} {fmt(r['lag_p50']):>7} {fmt(r['lag_p90']):>7} {fmt(r['lag_p99']):>7} "
                     f"{fmt(r['lag_max']):>7}  {fmt(r['mean_run_p99'])}")
    return "\n".join(lines)


READ_FIELDS = {"lag_p50": "quantile_cont(lag_ms, 0.5)", "lag_p99": "quantile_cont(lag_ms, 0.99)",
               "lag_max": "max(lag_ms)", "reads": "count(*)"}
VERDICT_FIELDS = {"healthy": "avg(healthy::INT)", "lag_q": "avg(lag_q)", "hit_share": "avg(hit_share)",
                  "delivery": "avg(delivery)"}
# viewer_seconds' fields over a bin: lag_ms the worst second's with a frame,
# frames (and any other) summed.
VIEWER_SECOND_FIELDS = {"lag_ms": "max(lag_ms) FILTER (WHERE lag_ms >= 0)::DOUBLE", "frames": "sum(frames)::DOUBLE"}
# The series' own columns, not fields.
ID_COLS = {"k", "src", "bin", "t_ms", "sec", "node", "viewer"}
# The forecast log's fields: (table, aggregate) over a bin's samples (a
# forecast's node is a viewer), as `forecast` judges them against the
# truth: each HAVE's first sample of its stream that didn't set the chunk's
# first sight, from an honest peer the sim saw hold the chunk as it arrived
# (`live`); the error is held - (seen + mean).
FORECAST_LIVE = "peer_honest AND live AND run = 0 AND NOT first_sight AND mean_ms >= 0 AND held_ms >= 0"
LOG_FIELDS = {
    "forecast_n": ("forecast", f"count(*) FILTER (WHERE {FORECAST_LIVE})::DOUBLE"),
    "forecast_err_p50": ("forecast", f"quantile_cont(held_ms - (seen_ms + mean_ms), 0.5) FILTER (WHERE {FORECAST_LIVE})"),
    "forecast_err_p90": ("forecast", f"quantile_cont(held_ms - (seen_ms + mean_ms), 0.9) FILTER (WHERE {FORECAST_LIVE})"),
    "forecast_abs_err_p50": ("forecast",
                             f"quantile_cont(abs(held_ms - (seen_ms + mean_ms)), 0.5) FILTER (WHERE {FORECAST_LIVE})"),
    "forecast_abs_err_p90": ("forecast",
                             f"quantile_cont(abs(held_ms - (seen_ms + mean_ms)), 0.9) FILTER (WHERE {FORECAST_LIVE})"),
}


def field_source(sel, v, field, per):
    """Where `field` comes from: (series, a table or a query; time column
    in ms; aggregate). Per node (`per` "node") each of the series' rows
    has its `node`: a viewer's reads and viewer_seconds by `viewers`."""
    f, per_node, have = field, per == "node", v.have

    def by_node(t):
        need(have, "viewers")
        return f"(SELECT x.*, w.node FROM {t} x JOIN viewers w USING (k, src, viewer))"

    if f in READ_FIELDS:
        need(have, "reads")
        return (by_node("reads") if per_node else "reads"), "t_ms", READ_FIELDS[f]
    if f in LOG_FIELDS:
        t, agg = LOG_FIELDS[f]
        need(have, t)
        return t, "t_ms", agg

    def columns(t):
        return {r["column_name"] for r in v.records(f"DESCRIBE {t};")} - ID_COLS if t in have else set()

    def check_bin(t):
        unit = 1000 if t in ("seconds", "viewer_seconds") else (
            v.records("SELECT max(bin_ms) AS b FROM runs;")[0]["b"] or 1)
        if sel.bin % unit:
            raise Refused(f"{f} comes in {unit} ms bins: --bin must be a multiple of {unit} "
                          "(or rerun with a smaller SIM_BIN_MS / STRIMS_SIM_BIN_MS)")

    nodes = [t for t in ("node_bins", "viewer_seconds") if f in columns(t)]
    if per_node:
        if nodes:
            t = nodes[0]
            check_bin(t)
            if t == "node_bins":
                return "node_bins", "t_ms", f"sum({ident(f)})::DOUBLE"
            return by_node("viewer_seconds"), "sec * 1000", VIEWER_SECOND_FIELDS.get(f, f"sum({ident(f)})::DOUBLE")
        wide = "a verdict" if f in VERDICT_FIELDS else next(
            (f"a {t}" for t in ("bins", "seconds") if f in columns(t)), None)
        if wide:
            raise Refused(f"{f} is run-wide ({wide} column): --per node aligns a node's own fields, its reads' "
                          f"({', '.join(READ_FIELDS)}), viewer_seconds' ({', '.join(VIEWER_SECOND_FIELDS)}) and "
                          "node_bins' columns"
                          + ("" if "node_bins" in have else
                             " (the selection has no node_bins: SIM_BIN_NODES=1 / STRIMS_SIM_BIN_NODES=1 write them)"))
    else:
        if f in VERDICT_FIELDS:
            if sel.bin % 1000:
                raise Refused(f"{f} is per second: --bin must be a multiple of 1000")
            return "verdict_seconds", "sec * 1000", VERDICT_FIELDS[f]
        for t, tcol in (("bins", "t_ms"), ("seconds", "sec * 1000")):
            if f in columns(t):
                check_bin(t)
                return t, tcol, f"sum({ident(f)})::DOUBLE"
        if nodes:
            raise Refused(f"{f} is a node's (a {nodes[0]} column): align it --per node")
    raise Refused(f"no field {f!r}: a read lag ({', '.join(READ_FIELDS)}), "
                  + ("a viewer_seconds column or a node_bins one" if per_node else
                     f"a verdict column ({', '.join(VERDICT_FIELDS)}), or a seconds or bins column"))


def aligned(sel, event, field="lag_p99", per="run", node=None, detail=None, pool=False, before=10, after=60, nth=1,
            say=None):
    """Bands across the selection's runs (`per` "run") or their nodes
    (`per` "node") around their `nth` `event` (0: every one, a window
    each): per bin of `sel.bin` ms from `before` to `after` seconds, the
    median and p10/p90 of `field` over the windows, or with `pool` the
    field over every window's rows of a cell and arm (its value thrice)."""
    say = say or stderr
    v = views(sel, say=say)
    if field in READ_FIELDS:
        v = with_samples(sel, v, ("reads",), say=say)
    elif field in LOG_FIELDS:
        v = with_samples(sel, v, (LOG_FIELDS[field][0],), say=say)
    with v:
        have = v.have
        need(have, "derived_events")
        per_node = per == "node"
        if per_node:
            need(have, "events")
        series, tcol, agg = field_source(sel, v, field, per)
        lo, hi = int(-before * 1000), int(after * 1000)
        on = "" if node is None else f" AND node = {int(node)}"
        keys = "k, src, node" if per_node else "k, src"
        kind = f"kind = {q(event)}"
        sims = (f"(SELECT count(*) FROM events WHERE {kind}{on}) AS sims, "
                f"(SELECT count(*) FROM events WHERE {kind}{on} AND node >= 0) AS nodes"
                if "events" in have else "0 AS sims, 0 AS nodes")
        # A derived event has no node.
        derived = f"(SELECT count(*) FROM derived_events WHERE {kind}{'' if node is None else ' AND false'})"
        got = v.records(f"SELECT {derived} AS derived, {sims}, "
                        "(SELECT list(DISTINCT kind ORDER BY kind) FROM ev) AS kinds;")[0]
        if got["derived"] and got["sims"]:
            raise Refused(f"{event} is both a derived event and a sim's: analyze.events must name its own apart "
                          "(DERIVED_KINDS)")
        if not got["derived"] and not got["sims"]:
            raise Refused(f"the selection has no {event} event" + (f" of node {node}" if node is not None else "")
                          + f"; its kinds: {', '.join(got['kinds'] or [])}")
        if per_node:
            if not got["nodes"]:
                raise Refused(f"{event} is run-wide (a derived event, or a sim's with no node): align it --per run")
            on += " AND node >= 0"
        if detail is not None:
            on += f" AND detail GLOB {q(detail)}"
        on += "".join(f" AND {w}" for w in time_filter(sel))
        # The nth occurrence, or every one (`nth` 0), each its own window.
        pick = "" if int(nth) == 0 else f" WHERE nth = {int(nth)}"
        query = f"""
WITH e AS (SELECT {keys}, t_ms AS e_ms, nth AS eid FROM
              (SELECT *, row_number() OVER (PARTITION BY {keys} ORDER BY t_ms) AS nth
               FROM ev WHERE kind = {q(event)}{on}){pick}),
x AS (SELECT x.*, e.eid, floor(({tcol} - e.e_ms) / {sel.bin}) * {sel.bin} AS rel
      FROM {series} x JOIN e USING ({keys})
      WHERE {tcol} - e.e_ms >= {lo} AND {tcol} - e.e_ms < {hi}),"""
        if pool:
            # Every window's rows of a cell and arm in one pool a bin.
            query += f"""
pool AS (SELECT r.cell, r.arm, x.rel, count(*) AS n, {agg} AS v FROM x JOIN runs r USING (k, src) GROUP BY ALL)
SELECT cell, arm, rel::BIGINT AS rel_ms, n, v AS p10, v AS p50, v AS p90 FROM pool WHERE v IS NOT NULL
ORDER BY cell, rel_ms, arm;"""
        else:
            query += f"""
per AS (SELECT * FROM (SELECT {keys}, eid, rel, {agg} AS v FROM x GROUP BY ALL) WHERE v IS NOT NULL),
band AS (SELECT r.cell, r.arm, rel::BIGINT AS rel_ms, count(*) AS n, quantile_cont(v, [0.1, 0.5, 0.9]) AS q
         FROM per JOIN runs r USING (k, src) GROUP BY ALL)
SELECT cell, arm, rel_ms, n, q[1] AS p10, q[2] AS p50, q[3] AS p90 FROM band ORDER BY cell, rel_ms, arm;"""
        out = v.run(query)
    if per_node and not out.rows:
        say(f"no node with a {event} event has a {field} series around it")
    return out


def aligned_text(rows, event, field="lag_p99", per="run", nth=1, pool=False):
    arms = sorted({r["arm"] for r in rows})
    lines = []
    per_node = per == "node"
    which = "every one" if int(nth) == 0 else f"#{nth}"
    around = f"each node's own {event} ({which})" if per_node else f"{event} ({which})"
    over = "nodes of runs" if per_node else "runs"
    if int(nth) == 0:
        over = f"windows ({over}' every event)"
    if pool:
        over = "the windows' pooled rows (the pool's value thrice)"
    for cell in sorted({r["cell"] for r in rows}):
        lines.append(f"\n{cell}: {field} around {around}, median [p10, p90] over {over} (n)")
        lines.append(f"{'rel_s':>8} " + " ".join(f"{short(x) or '-':>34}" for x in arms))
        by = {(r["rel_ms"], r["arm"]): r for r in rows if r["cell"] == cell}
        for rel in sorted({k for k, _ in by}):
            cells = []
            for x in arms:
                r = by.get((rel, x))
                cells.append(f"{'-':>34}" if r is None else
                             f"{fmt(r['p50'], 2):>10} [{fmt(r['p10'], 2)}, {fmt(r['p90'], 2)}] ({r['n']})".rjust(34))
            lines.append(f"{rel / 1000:8.2f} " + " ".join(cells))
    return "\n".join(lines)


def events(sel, say=None):
    """The run metrics pooled by cell and arm: collapses, onset after a
    step down and recovery after a step up (with the runs they censored),
    excess lag, flaps."""
    with views(sel, say=say) as v:
        need(v.have, "verdicts")
        return v.run("""
SELECT r.cell, r.arm, count(*) AS runs, sum(collapsed::INT)::BIGINT AS collapsed, sum(collapses)::BIGINT AS collapses,
       median(onset_after_drop_s) AS onset_after_drop_s, sum(onset_after_drop_censored::INT)::BIGINT AS onset_censored,
       count(onset_after_drop_s) AS drops,
       median(recover_after_rise_s) AS recover_after_rise_s, sum(recover_after_rise_censored::INT)::BIGINT AS recover_censored,
       count(recover_after_rise_s) AS rises,
       median(excess_lag_area_ms_s) AS excess_area_p50, quantile_cont(excess_lag_area_ms_s, 0.9) AS excess_area_p90,
       median(excess_lag_peak_ms) AS excess_peak_p50, median(flaps_per_min) AS flaps_per_min
FROM verdicts JOIN runs r USING (k, src) GROUP BY ALL ORDER BY ALL;""")


def events_text(rows):
    lines = [f"{'cell':22} {'arm':24} {'runs':>4} {'coll':>4} {'onset_s (cens/n)':>17} {'recover_s (cens/n)':>19} "
             f"{'excess p50 p90':>15} {'peak':>6} {'flaps':>6}"]
    for r in rows:
        lines.append(
            f"{r['cell'] or '-':22} {short(r['arm']) or '-':24} {r['runs']:4d} {r['collapsed'] or 0:4d} "
            f"{fmt(r['onset_after_drop_s'], 1):>7} ({r['onset_censored'] or 0}/{r['drops']})".ljust(73)
            + f" {fmt(r['recover_after_rise_s'], 1):>7} ({r['recover_censored'] or 0}/{r['rises']})".ljust(20)
            + f" {fmt(r['excess_area_p50']):>7} {fmt(r['excess_area_p90']):>7} {fmt(r['excess_peak_p50']):>6}"
            f" {fmt(r['flaps_per_min'], 2):>6}")
    return "\n".join(lines)


def osc(sel, say=None):
    """Oscillation by cell and arm: spells and their dwell times, flaps,
    the lag's autocorrelation period and amplitude."""
    with views(sel, say=say) as v:
        need(v.have, "verdicts", "episodes")
        return v.run("""
WITH sp AS (SELECT r.cell, r.arm, e.kind, e.len FROM episodes e JOIN runs r USING (k, src)),
d AS (SELECT cell, arm,
             quantile_cont(len, 0.5) FILTER (WHERE kind = 'healthy') AS healthy_p50,
             quantile_cont(len, 0.9) FILTER (WHERE kind = 'healthy') AS healthy_p90,
             quantile_cont(len, 0.5) FILTER (WHERE kind = 'unhealthy') AS unhealthy_p50,
             quantile_cont(len, 0.9) FILTER (WHERE kind = 'unhealthy') AS unhealthy_p90,
             count(*) FILTER (WHERE kind = 'unhealthy') AS unhealthy_spells
      FROM sp GROUP BY ALL),
m AS (SELECT r.cell, r.arm, count(*) AS runs, median(flaps_per_min) AS flaps_per_min,
             max(collapses)::BIGINT AS max_collapses, count(osc_period_s) AS periodic,
             median(osc_period_s) AS period_s, median(osc_acf) AS acf, median(osc_amp_ms) AS amp_ms,
             quantile_cont(osc_amp_ms, 0.9) AS amp_p90_ms
      FROM verdicts JOIN runs r USING (k, src) GROUP BY ALL)
SELECT * FROM m JOIN d USING (cell, arm) ORDER BY ALL;""")


def osc_text(rows):
    lines = [f"{'cell':22} {'arm':24} {'runs':>4} {'flaps/min':>9} {'unhealthy spells':>16} "
             f"{'dwell s: healthy p50 p90':>24} {'unhealthy p50 p90':>18} {'periodic':>8} {'period_s':>8} "
             f"{'acf':>5} {'amp_ms p50 p90':>15}"]
    for r in rows:
        lines.append(
            f"{r['cell'] or '-':22} {short(r['arm']) or '-':24} {r['runs']:4d} {fmt(r['flaps_per_min'], 2):>9} "
            f"{r['unhealthy_spells']:>16} {fmt(r['healthy_p50']):>15} {fmt(r['healthy_p90']):>8} "
            f"{fmt(r['unhealthy_p50']):>9} {fmt(r['unhealthy_p90']):>8} {r['periodic']:>8} "
            f"{fmt(r['period_s'], 1):>8} {fmt(r['acf'], 2):>5} {fmt(r['amp_ms']):>7} {fmt(r['amp_p90_ms']):>7}")
    return "\n".join(lines)


def forecast(sel, say=None):
    """The forecast logs pooled by sim, cell, arm and the pace asked of
    the HAVEs (simdb.py's docs)."""
    v = with_samples(sel, views(sel, derived=False, say=say), ("forecast",), derived=False, say=say)
    with v:
        need(v.have, "forecast")
        w = " AND ".join(["run = 0", "NOT first_sight", "mean_ms >= 0", *time_filter(sel)])
        return v.run(f"""
WITH f AS (SELECT r.sim, r.cell, r.arm, CASE WHEN lazy THEN 'lazy' ELSE 'prompt' END AS pace,
                  sample_ms - mean_ms AS err, sample_ms <= horizon_ms AS covered, held_ms,
                  peer_honest,
                  peer_honest AND held_ms >= 0 AND held_ms < channel_open_ms AS backlog,
                  CASE WHEN peer_honest AND live THEN held_ms - (seen_ms + mean_ms) END AS truth_err,
                  CASE WHEN peer_honest AND live THEN seen_ms + sample_ms - held_ms END AS late
           FROM forecast x JOIN runs r USING (k, src) WHERE {w})
SELECT sim, cell, arm, pace, count(*) AS n,
       quantile_cont(err, [0.1, 0.5, 0.9, 0.99]) AS err_q, avg(covered::INT) AS coverage,
       count(truth_err) AS held, quantile_cont(truth_err, [0.1, 0.5, 0.9, 0.99]) AS truth_q,
       quantile_cont(late, [0.1, 0.5, 0.9, 0.99]) AS late_q,
       avg((held_ms IS NULL OR held_ms < 0)::INT) FILTER (WHERE peer_honest) AS censored,
       avg(backlog::INT) FILTER (WHERE peer_honest) AS backlog
FROM f GROUP BY ALL ORDER BY ALL;""")


def q4(x):
    return "-" if not x else " ".join(fmt(y) for y in x)


def forecast_text(rows):
    lines = [f"{'sim':10} {'cell':20} {'arm':24} {'pace':6} {'n':>8} {'err ms p10 p50 p90 p99':>23} "
             f"{'cover':>5} {'held':>7} {'truth err p10 p50 p90 p99':>26} {'HAVE late p10 p50 p90 p99':>26} "
             f"{'cens':>5} {'backlog':>7}"]
    for r in rows:
        lines.append(
            f"{r['sim'] or '-':10} {r['cell'] or '-':20} {short(r['arm']) or '-':24} {r['pace']:6} {r['n']:8d} "
            f"{q4(r['err_q']):>23} {fmt(r['coverage'], 2):>5} {r['held']:7d} {q4(r['truth_q']):>26} "
            f"{q4(r['late_q']):>26} {fmt(r['censored'], 3):>5} {fmt(r['backlog'], 3):>7}")
    return "\n".join(lines)


def selected_rows(sel, fields=None, say=None):
    """The selection's runs' rows, each a dict of its cell, arm, seed, key
    and source, then the row's `fields` (all by default), its `_verdict`'s
    onset, recovered, absorbing, unhealthy and lag_p50, and `_collapsed`;
    ordered by cell, arm and seed, a key in each of its cells from its own
    copy."""
    with views(sel, derived=False, say=say) as v:
        keys = v.records("SELECT src, k, cell, arm, seed FROM runs ORDER BY cell, arm, seed;")
    got = rows([(r["src"], r["k"]) for r in keys])
    out = []
    for r in keys:
        row = got.get((r["src"], r["k"]))
        if row is None:
            continue
        # A key counts in each of its cells: each prints its own copy.
        row = dict(row)
        v_ = row.pop("_verdict") or {}
        line = {"cell": r["cell"], "arm": r["arm"], "seed": r["seed"], "k": r["k"], "src": r["src"]}
        line.update({f: row.get(f) for f in fields} if fields else row)
        line["_verdict"] = {x: v_.get(x) for x in ("onset", "recovered", "absorbing", "unhealthy", "lag_p50")}
        line["_collapsed"] = verdict.collapsed(v_) if v_ else None
        out.append(line)
    return out


# --- pulls --------------------------------------------------------------------


@dataclasses.dataclass
class Pulled:
    """What pull_run did, or with `dry_run` would do: a whole run's pull
    (its directory `dest` in the cache and its `sources` there), the
    selection's samples pulled (`fetched`), or the samples' `files` a pull
    would GET ((url, bytes, jobs)) for `jobs` jobs; `lost` jobs S3 holds
    none for."""
    run: str
    dry_run: bool = False
    dest: Path = None
    sources: int = 0
    fetched: bool = False
    files: list = dataclasses.field(default_factory=list)
    jobs: int = 0
    lost: int = 0
    tables: tuple = SAMPLE_TABLES


def pull_run(run, reads=False, cell=None, arm=None, seed=None, dry_run=False, say=None):
    """simdb.py pull: farm run `run`'s flushes copied into the cache, with
    `reads` their samples (the whole run's, or the jobs `cell`, `arm` and
    `seed` select), or with `dry_run` the samples' files a pull would GET,
    asking S3 nothing."""
    picked = cell or arm or seed
    if (picked or dry_run) and not reads:
        raise Refused("--cell, --arm, --seed and --dry-run are of a pull's reads: give --reads")
    out = Pulled(run, dry_run)
    if not dry_run and not picked:
        out.dest = store.pull(run, reads).dest
        out.sources = len(sources([f"farm/{run}"]))
        return out
    srcs = sources([f"farm/{run}"])
    if not srcs:
        if dry_run:
            raise Refused(f"the cache lacks farm/{run}: pull --run {run} copies the tables that name its jobs")
        store.pull(run)
    if picked:
        sel = Selection(src=[f"farm/{run}"], cell=cell, arm=arm, seed=seed)
        v = views(sel, derived=False, fetch=not dry_run, say=say)
        if not dry_run:
            out.fetched = store.pull_samples(v.sql, SAMPLE_TABLES, say=say)
            return out
        plan, out.lost = store.sample_plan(v.sql, SAMPLE_TABLES)
    else:
        # The whole run's, as `pull --reads` syncs them: each flush's.
        root, bad = store_host.cache_root() / "cache", store.unlanded(srcs)
        plan = {}
        for s in srcs:
            m = manifest_of(s)
            listed, jobs = m.get("tables") or {}, len(m.get("keys") or [])
            lack = [t for t in SAMPLE_TABLES if t in listed and not (root / s / f"{t}.parquet").exists()]
            if lack and s in bad:
                out.lost += jobs
            elif lack:
                plan[s] = (lack, sum(listed[t].get("bytes", 0) for t in lack), jobs)
    bucket, _, _ = store.farm_bucket()
    out.files = [(f"s3://{bucket}/sim-samples/{s.split('/', 1)[1]}/{t}.parquet", manifest_of(s)["tables"][t]["bytes"],
                  n) for s, (tables, _, n) in plan.items() for t in tables]
    out.jobs = sum(n for *_, n in plan.values())
    return out


def pull_text(p):
    if not p.dry_run and p.dest is not None:
        return f"{p.run}: {p.sources} sources in {p.dest}"
    if not p.dry_run:
        return f"{p.run}: the selection's {' and '.join(p.tables)} in {store_host.cache_root() / 'cache' / 'farm'}"
    gone = f"; S3 holds none for {p.lost} jobs (expired, or never landed)" if p.lost else ""
    if not p.files:
        return f"{p.run}: nothing to pull: the cache holds what S3 has of the jobs' samples{gone}"
    lines = [f"{p.run}: would GET {len(p.files)} files, {sum(b for _, b, _ in p.files) / 1e6:.1f} MB, for "
             f"{p.jobs} jobs' {' and '.join(p.tables)}{gone}"]
    lines += [f"  {url}  {b / 1e6:.1f} MB  {n} jobs" for url, b, n in p.files]
    return "\n".join(lines)
